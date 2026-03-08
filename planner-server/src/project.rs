//! # Project Store — Canonical Product Project Persistence
//!
//! Phase 00 introduces a persisted `Project` entity that owns sessions and
//! provides canonical project identity (UUID + slug + legacy aliases).

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const MIGRATION_OWNER_USER_ID: &str = "system|migration";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub owner_user_id: String,
    #[serde(default)]
    pub team_label: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(default)]
    pub legacy_scope_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migration_source: Option<String>,
}

impl Project {
    fn new(
        id: Uuid,
        owner_user_id: &str,
        name: &str,
        description: Option<String>,
        team_label: Option<String>,
        legacy_scope_keys: Vec<String>,
        migration_source: Option<String>,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        let normalized_name =
            normalize_name(name).unwrap_or_else(|| format!("Project {}", &id.to_string()[..8]));
        Self {
            id,
            slug: slugify(&normalized_name),
            name: normalized_name,
            description: normalize_optional_text(description),
            owner_user_id: owner_user_id.to_string(),
            team_label: normalize_optional_text(team_label),
            created_at: now.clone(),
            updated_at: now,
            archived_at: None,
            legacy_scope_keys: normalize_legacy_scope_keys(legacy_scope_keys),
            migration_source: normalize_optional_text(migration_source),
        }
    }
}

fn normalize_name(value: &str) -> Option<String> {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(120).collect())
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        let trimmed = collapsed.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_legacy_scope_keys(keys: Vec<String>) -> Vec<String> {
    let mut unique = HashSet::new();
    let mut out = Vec::new();
    for key in keys {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            continue;
        }
        let canonical = trimmed.to_lowercase();
        if unique.insert(canonical) {
            out.push(trimmed.to_string());
        }
    }
    out
}

pub fn slugify(input: &str) -> String {
    let slug = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        format!("project-{}", &Uuid::new_v4().to_string()[..8])
    } else {
        slug
    }
}

pub fn derive_project_name(seed: &str) -> String {
    if let Some(name) = normalize_name(seed) {
        return name.chars().take(60).collect();
    }
    "Untitled Project".into()
}

fn ensure_unique_slug(
    projects: &HashMap<Uuid, Project>,
    preferred: &str,
    current_id: Uuid,
) -> String {
    let base = if preferred.trim().is_empty() {
        format!("project-{}", &current_id.to_string()[..8])
    } else {
        slugify(preferred)
    };

    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while projects
        .values()
        .any(|project| project.id != current_id && project.slug.eq_ignore_ascii_case(&candidate))
    {
        candidate = format!("{}-{}", base, suffix);
        suffix += 1;
    }
    candidate
}

/// Thread-safe, memory-first, disk-backed store for canonical projects.
pub struct ProjectStore {
    projects: RwLock<HashMap<Uuid, Project>>,
    dirty: RwLock<HashSet<Uuid>>,
    projects_dir: Option<PathBuf>,
}

impl ProjectStore {
    pub fn new() -> Self {
        Self {
            projects: RwLock::new(HashMap::new()),
            dirty: RwLock::new(HashSet::new()),
            projects_dir: None,
        }
    }

    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        let projects_dir = data_dir.join("projects");
        std::fs::create_dir_all(&projects_dir)?;

        let probe = projects_dir.join(".write_probe");
        std::fs::write(&probe, b"ok")?;
        std::fs::remove_file(&probe)?;

        let mut projects = HashMap::new();
        let mut load_errors = 0u32;

        for entry in std::fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if !file_name.ends_with(".msgpack") || file_name.ends_with(".tmp") {
                continue;
            }

            match std::fs::read(&path) {
                Ok(bytes) => match rmp_serde::from_slice::<Project>(&bytes) {
                    Ok(project) => {
                        projects.insert(project.id, project);
                    }
                    Err(err) => {
                        tracing::error!("Failed to decode project {}: {}", file_name, err);
                        load_errors += 1;
                    }
                },
                Err(err) => {
                    tracing::error!("Failed to read project file {}: {}", file_name, err);
                    load_errors += 1;
                }
            }
        }

        if load_errors > 0 {
            tracing::warn!(
                "Project store: loaded {} projects, {} files had errors",
                projects.len(),
                load_errors
            );
        } else if !projects.is_empty() {
            tracing::info!(
                "Project store: loaded {} projects from disk",
                projects.len()
            );
        }

        Ok(Self {
            projects: RwLock::new(projects),
            dirty: RwLock::new(HashSet::new()),
            projects_dir: Some(projects_dir),
        })
    }

    fn mark_dirty(&self, id: Uuid) {
        if self.projects_dir.is_some() {
            self.dirty.write().insert(id);
        }
    }

    pub fn count(&self) -> usize {
        self.projects.read().len()
    }

    pub fn dirty_count(&self) -> usize {
        self.dirty.read().len()
    }

    pub fn is_persistent(&self) -> bool {
        self.projects_dir.is_some()
    }

    pub fn list_for_user(&self, user_id: &str) -> Vec<Project> {
        self.projects
            .read()
            .values()
            .filter(|project| {
                project.owner_user_id == user_id || project.owner_user_id == MIGRATION_OWNER_USER_ID
            })
            .cloned()
            .collect()
    }

    pub fn get(&self, id: Uuid) -> Option<Project> {
        self.projects.read().get(&id).cloned()
    }

    pub fn resolve_ref(&self, project_ref: &str) -> Option<Project> {
        let trimmed = project_ref.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Ok(id) = Uuid::parse_str(trimmed) {
            if let Some(project) = self.get(id) {
                return Some(project);
            }
        }

        let lowered = trimmed.to_lowercase();
        let projects = self.projects.read();
        if let Some(project) = projects
            .values()
            .find(|project| project.slug.eq_ignore_ascii_case(&lowered))
        {
            return Some(project.clone());
        }

        projects
            .values()
            .find(|project| {
                project
                    .legacy_scope_keys
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(trimmed))
            })
            .cloned()
    }

    fn save_locked(
        projects: &mut HashMap<Uuid, Project>,
        mut project: Project,
        mark_dirty: impl Fn(Uuid),
    ) -> Project {
        let now = Utc::now().to_rfc3339();
        if project.created_at.trim().is_empty() {
            project.created_at = now.clone();
        }
        project.updated_at = now;
        project.name = normalize_name(&project.name)
            .unwrap_or_else(|| format!("Project {}", &project.id.to_string()[..8]));
        project.description = normalize_optional_text(project.description.take());
        project.team_label = normalize_optional_text(project.team_label.take());
        project.migration_source = normalize_optional_text(project.migration_source.take());
        project.slug = ensure_unique_slug(projects, &project.slug, project.id);
        project.legacy_scope_keys = normalize_legacy_scope_keys(project.legacy_scope_keys);
        projects.insert(project.id, project.clone());
        mark_dirty(project.id);
        project
    }

    pub fn save(&self, project: Project) -> Project {
        let mut projects = self.projects.write();
        Self::save_locked(&mut projects, project, |id| self.mark_dirty(id))
    }

    pub fn create(
        &self,
        owner_user_id: &str,
        name: &str,
        description: Option<String>,
        team_label: Option<String>,
        legacy_scope_keys: Vec<String>,
        migration_source: Option<String>,
    ) -> Project {
        let id = Uuid::new_v4();
        self.create_with_id(
            owner_user_id,
            id,
            name,
            description,
            team_label,
            legacy_scope_keys,
            migration_source,
        )
    }

    pub fn create_with_id(
        &self,
        owner_user_id: &str,
        id: Uuid,
        name: &str,
        description: Option<String>,
        team_label: Option<String>,
        legacy_scope_keys: Vec<String>,
        migration_source: Option<String>,
    ) -> Project {
        if let Some(existing) = self.get(id) {
            return existing;
        }
        let project = Project::new(
            id,
            owner_user_id,
            name,
            description,
            team_label,
            legacy_scope_keys,
            migration_source,
        );
        self.save(project)
    }

    pub fn add_legacy_alias(&self, project_id: Uuid, alias: &str) -> Option<Project> {
        let trimmed = alias.trim();
        if trimmed.is_empty() {
            return self.get(project_id);
        }
        self.update(project_id, |project| {
            if !project
                .legacy_scope_keys
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(trimmed))
            {
                project.legacy_scope_keys.push(trimmed.to_string());
            }
        })
    }

    pub fn update<F>(&self, id: Uuid, f: F) -> Option<Project>
    where
        F: FnOnce(&mut Project),
    {
        let mut projects = self.projects.write();
        let mut updated = projects.get(&id)?.clone();
        f(&mut updated);
        Some(Self::save_locked(&mut projects, updated, |pid| {
            self.mark_dirty(pid)
        }))
    }

    pub fn flush_dirty(&self) -> (usize, usize) {
        let projects_dir = match &self.projects_dir {
            Some(dir) => dir,
            None => return (0, 0),
        };

        let dirty_ids: Vec<Uuid> = self.dirty.read().iter().copied().collect();
        if dirty_ids.is_empty() {
            return (0, 0);
        }

        let mut flushed = 0usize;
        let mut errors = 0usize;

        let snapshots: Vec<(Uuid, Vec<u8>)> = {
            let projects = self.projects.read();
            let mut out = Vec::with_capacity(dirty_ids.len());
            for id in &dirty_ids {
                match projects.get(id) {
                    Some(project) => match rmp_serde::to_vec(project) {
                        Ok(bytes) => out.push((*id, bytes)),
                        Err(err) => {
                            tracing::error!("Failed to encode project {}: {}", id, err);
                            errors += 1;
                        }
                    },
                    None => {
                        self.dirty.write().remove(id);
                    }
                }
            }
            out
        };

        for (id, bytes) in snapshots {
            let final_path = projects_dir.join(format!("{}.msgpack", id));
            let tmp_path = projects_dir.join(format!("{}.msgpack.tmp", id));
            let write_result = (|| -> std::io::Result<()> {
                let mut file = std::fs::File::create(&tmp_path)?;
                std::io::Write::write_all(&mut file, &bytes)?;
                file.sync_all()?;
                Ok(())
            })();

            if let Err(err) = write_result {
                tracing::error!("Failed to write project {}: {}", id, err);
                errors += 1;
                continue;
            }
            if let Err(err) = std::fs::rename(&tmp_path, &final_path) {
                tracing::error!("Failed to rename project {}: {}", id, err);
                errors += 1;
                continue;
            }

            self.dirty.write().remove(&id);
            flushed += 1;
        }

        if flushed > 0 || errors > 0 {
            tracing::debug!("Project flush: {} written, {} errors", flushed, errors);
        }

        (flushed, errors)
    }
}

pub struct Phase0BackfillReport {
    pub projects_created: usize,
    pub sessions_assigned: usize,
    pub blueprint_nodes_normalized: usize,
}

fn project_name_from_alias(alias: &str) -> String {
    let raw = alias.trim().trim_start_matches("proj-");
    let mut out = String::new();
    for token in raw.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        if token.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        let mut chars = token.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() {
        derive_project_name(alias)
    } else {
        out
    }
}

fn normalize_node_scope_to_canonical(
    node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
    projects: &ProjectStore,
) -> bool {
    fn scope_mut(
        node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
    ) -> &mut planner_schemas::artifacts::blueprint::NodeScope {
        match node {
            planner_schemas::artifacts::blueprint::BlueprintNode::Decision(inner) => {
                &mut inner.scope
            }
            planner_schemas::artifacts::blueprint::BlueprintNode::Technology(inner) => {
                &mut inner.scope
            }
            planner_schemas::artifacts::blueprint::BlueprintNode::Component(inner) => {
                &mut inner.scope
            }
            planner_schemas::artifacts::blueprint::BlueprintNode::Constraint(inner) => {
                &mut inner.scope
            }
            planner_schemas::artifacts::blueprint::BlueprintNode::Pattern(inner) => {
                &mut inner.scope
            }
            planner_schemas::artifacts::blueprint::BlueprintNode::QualityRequirement(inner) => {
                &mut inner.scope
            }
        }
    }

    let mut changed = false;
    let scope = scope_mut(node);

    if let Some(project_scope) = scope.project.as_mut() {
        let source = project_scope.project_id.trim().to_string();
        if !source.is_empty() {
            if let Some(project) = projects.resolve_ref(&source) {
                let canonical = project.id.to_string();
                if source != canonical {
                    project_scope.project_id = canonical;
                    let _ = projects.add_legacy_alias(project.id, &source);
                    changed = true;
                }
                if project_scope
                    .project_name
                    .as_deref()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
                {
                    project_scope.project_name = Some(project.name);
                    changed = true;
                }
            }
        }
    }

    if let Some(shared) = scope.shared.as_mut() {
        let mut next = Vec::new();
        let mut seen = HashSet::new();
        for linked in &shared.linked_project_ids {
            let resolved = projects
                .resolve_ref(linked)
                .map(|project| {
                    let canonical = project.id.to_string();
                    if !linked.eq_ignore_ascii_case(&canonical) {
                        let _ = projects.add_legacy_alias(project.id, linked);
                    }
                    canonical
                })
                .unwrap_or_else(|| linked.trim().to_string());

            if resolved.is_empty() {
                continue;
            }
            if seen.insert(resolved.to_lowercase()) {
                next.push(resolved);
            }
        }
        if next != shared.linked_project_ids {
            shared.linked_project_ids = next;
            changed = true;
        }
    }

    changed
}

pub fn phase0_backfill(
    projects: &ProjectStore,
    sessions: &crate::session::SessionStore,
    blueprints: &planner_core::blueprint::BlueprintStore,
) -> Phase0BackfillReport {
    let mut projects_created = 0usize;
    let mut sessions_assigned = 0usize;
    let mut blueprint_nodes_normalized = 0usize;

    // Seed projects from Blueprint project references (aliases/UUIDs).
    for summary in blueprints.list_summaries() {
        if let Some(scope_key) = summary.project_id.as_deref() {
            let key = scope_key.trim();
            if !key.is_empty() && projects.resolve_ref(key).is_none() {
                let preferred_name = summary
                    .project_name
                    .as_deref()
                    .map(derive_project_name)
                    .unwrap_or_else(|| project_name_from_alias(key));
                if let Ok(existing_uuid) = Uuid::parse_str(key) {
                    let _ = projects.create_with_id(
                        MIGRATION_OWNER_USER_ID,
                        existing_uuid,
                        &preferred_name,
                        None,
                        None,
                        Vec::new(),
                        Some("knowledge_alias".into()),
                    );
                } else {
                    let _ = projects.create(
                        MIGRATION_OWNER_USER_ID,
                        &preferred_name,
                        None,
                        None,
                        vec![key.to_string()],
                        Some("knowledge_alias".into()),
                    );
                }
                projects_created += 1;
            }
        }

        for linked in &summary.linked_project_ids {
            let key = linked.trim();
            if key.is_empty() || projects.resolve_ref(key).is_some() {
                continue;
            }
            let preferred_name = project_name_from_alias(key);
            if let Ok(existing_uuid) = Uuid::parse_str(key) {
                let _ = projects.create_with_id(
                    MIGRATION_OWNER_USER_ID,
                    existing_uuid,
                    &preferred_name,
                    None,
                    None,
                    Vec::new(),
                    Some("knowledge_alias".into()),
                );
            } else {
                let _ = projects.create(
                    MIGRATION_OWNER_USER_ID,
                    &preferred_name,
                    None,
                    None,
                    vec![key.to_string()],
                    Some("knowledge_alias".into()),
                );
            }
            projects_created += 1;
        }
    }

    // Backfill session project ownership.
    for session_id in sessions.list_ids() {
        let Some(session) = sessions.get(session_id) else {
            continue;
        };

        let current_project = session
            .project_id
            .and_then(|pid| projects.get(pid))
            .or_else(|| {
                session.project_id.map(|pid| {
                    projects.create_with_id(
                        &session.user_id,
                        pid,
                        &session.display_title(),
                        session.project_description.clone(),
                        None,
                        Vec::new(),
                        Some("session_seed".into()),
                    )
                })
            })
            .or_else(|| {
                session.cxdb_project_id.map(|pid| {
                    projects.create_with_id(
                        &session.user_id,
                        pid,
                        &session.display_title(),
                        session.project_description.clone(),
                        None,
                        Vec::new(),
                        Some("cxdb_seed".into()),
                    )
                })
            })
            .or_else(|| {
                session
                    .project_description
                    .as_deref()
                    .and_then(|description| {
                        let trimmed = description.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(projects.create(
                                &session.user_id,
                                &derive_project_name(trimmed),
                                Some(trimmed.to_string()),
                                None,
                                Vec::new(),
                                Some("session_seed".into()),
                            ))
                        }
                    })
            });

        let Some(project) = current_project else {
            continue;
        };

        let needs_update = session.project_id != Some(project.id)
            || session.project_slug.as_deref() != Some(project.slug.as_str())
            || session.project_name.as_deref() != Some(project.name.as_str());

        if needs_update {
            sessions.update(session_id, |draft| {
                draft.project_id = Some(project.id);
                draft.project_slug = Some(project.slug.clone());
                draft.project_name = Some(project.name.clone());
                if draft.cxdb_project_id.is_none() {
                    draft.cxdb_project_id = Some(project.id);
                }
            });
            sessions_assigned += 1;
        }
    }

    // Normalize blueprint node scope IDs to canonical project UUIDs.
    for summary in blueprints.list_summaries() {
        let node_id = summary.id.as_str().to_string();
        let Some(mut node) = blueprints.get_node(&node_id) else {
            continue;
        };
        if normalize_node_scope_to_canonical(&mut node, projects) {
            blueprints.upsert_node(node);
            blueprint_nodes_normalized += 1;
        }
    }

    Phase0BackfillReport {
        projects_created,
        sessions_assigned,
        blueprint_nodes_normalized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_resolve_project_refs() {
        let store = ProjectStore::new();
        let created = store.create(
            "dev|local",
            "Task Tracker",
            Some("Planning workspace".into()),
            None,
            vec!["proj-task-tracker".into()],
            None,
        );

        let by_id = store.resolve_ref(&created.id.to_string()).unwrap();
        assert_eq!(by_id.id, created.id);

        let by_slug = store.resolve_ref(&created.slug).unwrap();
        assert_eq!(by_slug.id, created.id);

        let by_alias = store.resolve_ref("proj-task-tracker").unwrap();
        assert_eq!(by_alias.id, created.id);
    }

    #[test]
    fn creates_unique_slugs() {
        let store = ProjectStore::new();
        let p1 = store.create("dev|local", "Alpha", None, None, Vec::new(), None);
        let p2 = store.create("dev|local", "Alpha", None, None, Vec::new(), None);
        assert_ne!(p1.id, p2.id);
        assert_eq!(p1.slug, "alpha");
        assert_eq!(p2.slug, "alpha-2");
    }

    #[test]
    fn disk_persistence_round_trip() {
        let data_dir =
            std::env::temp_dir().join(format!("planner_project_test_{}", Uuid::new_v4()));
        let store = ProjectStore::open(&data_dir).unwrap();
        let created = store.create(
            "dev|local",
            "Ops Console",
            Some("Control plane".into()),
            None,
            vec!["proj-ops-console".into()],
            Some("session_seed".into()),
        );
        let (flushed, errors) = store.flush_dirty();
        assert_eq!(flushed, 1);
        assert_eq!(errors, 0);

        let reopened = ProjectStore::open(&data_dir).unwrap();
        let loaded = reopened.get(created.id).unwrap();
        assert_eq!(loaded.name, "Ops Console");
        assert_eq!(loaded.owner_user_id, "dev|local");
        assert!(loaded
            .legacy_scope_keys
            .iter()
            .any(|entry| entry == "proj-ops-console"));

        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
