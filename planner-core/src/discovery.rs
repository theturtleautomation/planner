use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use planner_schemas::artifacts::blueprint::*;

use crate::blueprint::BlueprintStore;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    CargoToml,
    DirectoryScan,
    PipelineRun,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Accepted,
    Rejected,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedNode {
    pub id: String,
    pub node: BlueprintNode,
    pub source: DiscoverySource,
    pub reason: String,
    pub status: ProposalStatus,
    pub proposed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    pub confidence: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScanOutput {
    pub proposals: Vec<ProposedNode>,
    pub skipped_count: usize,
    pub errors: Vec<String>,
}

impl Default for ScanOutput {
    fn default() -> Self {
        Self {
            proposals: Vec::new(),
            skipped_count: 0,
            errors: Vec::new(),
        }
    }
}

pub struct ProposalStore {
    proposals: RwLock<Vec<ProposedNode>>,
    proposals_path: Option<PathBuf>,
}

impl ProposalStore {
    pub fn new() -> Self {
        Self {
            proposals: RwLock::new(Vec::new()),
            proposals_path: None,
        }
    }

    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        let blueprint_dir = data_dir.join("blueprint");
        std::fs::create_dir_all(&blueprint_dir)?;
        let proposals_path = blueprint_dir.join("proposals.msgpack");

        let proposals = if proposals_path.exists() {
            let bytes = std::fs::read(&proposals_path)?;
            rmp_serde::from_slice::<Vec<ProposedNode>>(&bytes)
                .map_err(|err| std::io::Error::other(format!("failed to decode proposals: {}", err)))?
        } else {
            Vec::new()
        };

        Ok(Self {
            proposals: RwLock::new(proposals),
            proposals_path: Some(proposals_path),
        })
    }

    pub fn list(&self, status: Option<ProposalStatus>) -> Vec<ProposedNode> {
        let mut proposals: Vec<ProposedNode> = self
            .proposals
            .read()
            .iter()
            .filter(|proposal| status.map(|value| proposal.status == value).unwrap_or(true))
            .cloned()
            .collect();
        proposals.sort_by(|left, right| right.proposed_at.cmp(&left.proposed_at));
        proposals
    }

    pub fn get(&self, proposal_id: &str) -> Option<ProposedNode> {
        self.proposals
            .read()
            .iter()
            .find(|proposal| proposal.id == proposal_id)
            .cloned()
    }

    pub fn insert_many(&self, proposals: Vec<ProposedNode>) -> std::io::Result<(usize, usize)> {
        let mut guard = self.proposals.write();
        let mut fingerprints: HashSet<String> = guard.iter().map(proposal_fingerprint).collect();
        let mut inserted = 0usize;
        let mut skipped = 0usize;

        for proposal in proposals {
            let fingerprint = proposal_fingerprint(&proposal);
            if !fingerprints.insert(fingerprint) {
                skipped += 1;
                continue;
            }

            guard.push(proposal);
            inserted += 1;
        }

        self.persist_locked(&guard)?;
        Ok((inserted, skipped))
    }

    pub fn mark_accepted(&self, proposal_id: &str) -> std::io::Result<Option<ProposedNode>> {
        self.update_status(proposal_id, ProposalStatus::Accepted, None)
    }

    pub fn mark_merged(&self, proposal_id: &str) -> std::io::Result<Option<ProposedNode>> {
        self.update_status(proposal_id, ProposalStatus::Merged, None)
    }

    pub fn mark_rejected(
        &self,
        proposal_id: &str,
        reason: Option<String>,
    ) -> std::io::Result<Option<ProposedNode>> {
        self.update_status(proposal_id, ProposalStatus::Rejected, reason)
    }

    fn update_status(
        &self,
        proposal_id: &str,
        status: ProposalStatus,
        review_note: Option<String>,
    ) -> std::io::Result<Option<ProposedNode>> {
        let mut guard = self.proposals.write();
        let Some(proposal) = guard.iter_mut().find(|proposal| proposal.id == proposal_id) else {
            return Ok(None);
        };

        if proposal.status == ProposalStatus::Merged && status != ProposalStatus::Merged {
            return Ok(Some(proposal.clone()));
        }

        proposal.status = status;
        proposal.reviewed_at = Some(now_iso());
        if review_note.is_some() {
            proposal.review_note = review_note;
        }

        let updated = proposal.clone();
        self.persist_locked(&guard)?;
        Ok(Some(updated))
    }

    fn persist_locked(&self, proposals: &[ProposedNode]) -> std::io::Result<()> {
        let Some(path) = &self.proposals_path else {
            return Ok(());
        };

        let bytes = rmp_serde::to_vec_named(proposals)
            .map_err(|err| std::io::Error::other(format!("failed to encode proposals: {}", err)))?;

        let tmp_path = path.with_extension("msgpack.tmp");
        {
            let mut file = std::fs::File::create(&tmp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }
}

pub fn scan_cargo_toml(project_root: &Path, blueprints: &BlueprintStore) -> ScanOutput {
    let mut output = ScanOutput::default();
    let existing_names = existing_node_names(blueprints, "technology");
    let mut seen_names = HashSet::new();
    let manifests = collect_paths(project_root, "Cargo.toml");

    for manifest in manifests {
        let relative = relative_display(project_root, &manifest);
        let contents = match std::fs::read_to_string(&manifest) {
            Ok(contents) => contents,
            Err(err) => {
                output
                    .errors
                    .push(format!("{}: {}", relative, err));
                continue;
            }
        };

        for dependency in parse_cargo_dependencies(&contents) {
            let key = dependency.name.to_lowercase();
            if !seen_names.insert(key.clone()) || existing_names.contains(&key) {
                output.skipped_count += 1;
                continue;
            }

            let node = BlueprintNode::Technology(Technology {
                id: NodeId::with_prefix("TECH", &dependency.name),
                name: dependency.name.clone(),
                version: dependency.version.clone(),
                category: classify_technology_category(&dependency.name),
                ring: AdoptionRing::Adopt,
                rationale: format!("Detected in {}", relative),
                license: None,
                tags: vec!["discovery".into(), "cargo".into()],
                documentation: None,
                created_at: now_iso(),
                updated_at: now_iso(),
            });

            output.proposals.push(ProposedNode {
                id: Uuid::new_v4().to_string(),
                node,
                source: DiscoverySource::CargoToml,
                reason: format!("Dependency '{}' discovered in {}", dependency.name, relative),
                status: ProposalStatus::Pending,
                proposed_at: now_iso(),
                reviewed_at: None,
                confidence: 0.9,
                source_artifact: Some(relative.clone()),
                review_note: None,
            });
        }
    }

    output
}

pub fn scan_directory_structure(project_root: &Path, blueprints: &BlueprintStore) -> ScanOutput {
    let mut output = ScanOutput::default();
    let existing_names = existing_node_names(blueprints, "component");
    let mut seen = HashSet::new();

    for path in directory_candidates(project_root) {
        let relative = relative_display(project_root, &path);
        let key = relative.to_lowercase();
        let component_name = humanize_name(path.file_name().and_then(|name| name.to_str()).unwrap_or("component"));
        if !seen.insert(key) || existing_names.contains(&component_name.to_lowercase()) {
            output.skipped_count += 1;
            continue;
        }

        let node = BlueprintNode::Component(Component {
            id: NodeId::with_prefix("COMP", &relative.replace(['/', '\\'], "-")),
            name: component_name.clone(),
            component_type: infer_component_type(&relative),
            description: format!("Discovered from directory structure at {}", relative),
            provides: Vec::new(),
            consumes: Vec::new(),
            status: ComponentStatus::Planned,
            tags: vec!["discovery".into(), "directory".into()],
            documentation: None,
            created_at: now_iso(),
            updated_at: now_iso(),
        });

        output.proposals.push(ProposedNode {
            id: Uuid::new_v4().to_string(),
            node,
            source: DiscoverySource::DirectoryScan,
            reason: format!("Potential component inferred from {}", relative),
            status: ProposalStatus::Pending,
            proposed_at: now_iso(),
            reviewed_at: None,
            confidence: 0.6,
            source_artifact: Some(relative),
            review_note: None,
        });
    }

    output
}

fn proposal_fingerprint(proposal: &ProposedNode) -> String {
    format!(
        "{}:{}:{}:{}",
        proposal.node.type_name(),
        proposal.node.name().to_lowercase(),
        proposal.source_artifact.as_deref().unwrap_or(""),
        match proposal.source {
            DiscoverySource::CargoToml => "cargo_toml",
            DiscoverySource::DirectoryScan => "directory_scan",
            DiscoverySource::PipelineRun => "pipeline_run",
            DiscoverySource::Manual => "manual",
        }
    )
}

fn existing_node_names(blueprints: &BlueprintStore, node_type: &str) -> HashSet<String> {
    blueprints
        .list_by_type(node_type)
        .into_iter()
        .map(|node| node.name.to_lowercase())
        .collect()
}

fn relative_display(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn collect_paths(project_root: &Path, filename: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut stack = vec![project_root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if entry_path.is_dir() {
                if should_skip_dir(&name) {
                    continue;
                }
                stack.push(entry_path);
                continue;
            }

            if name == filename {
                paths.push(entry_path);
            }
        }
    }

    paths
}

fn directory_candidates(project_root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut roots = vec![project_root.to_path_buf()];
    for extra in ["src", "crates"] {
        let path = project_root.join(extra);
        if path.is_dir() {
            roots.push(path);
        }
    }

    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !path.is_dir() || should_skip_dir(&name) {
                continue;
            }

            if root == project_root || is_conventional_component_dir(&name) {
                candidates.push(path);
            }
        }
    }

    candidates
}

fn should_skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target" | "node_modules" | "dist" | "coverage" | "data" | "__pycache__"
        )
}

fn is_conventional_component_dir(name: &str) -> bool {
    let normalized = name.to_lowercase();
    matches!(
        normalized.as_str(),
        "api" | "app" | "cli" | "core" | "db" | "lib" | "pipeline" | "services" | "store" | "ui" | "web"
    )
}

#[derive(Debug)]
struct CargoDependency {
    name: String,
    version: Option<String>,
}

fn parse_cargo_dependencies(contents: &str) -> Vec<CargoDependency> {
    let mut dependencies = Vec::new();
    let mut section = String::new();
    let mut pending_inline: Option<(String, String, i32)> = None;

    for raw_line in contents.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if let Some((name, buffer, depth)) = pending_inline.as_mut() {
            buffer.push(' ');
            buffer.push_str(line);
            *depth += brace_delta(line);
            if *depth <= 0 {
                dependencies.push(CargoDependency {
                    name: name.clone(),
                    version: extract_version(buffer),
                });
                pending_inline = None;
            }
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(['[', ']']).to_string();
            continue;
        }

        if !section.ends_with("dependencies") {
            continue;
        }

        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim().trim_matches('"').to_string();
        let value = value.trim();

        if value.starts_with('{') && brace_delta(value) > 0 {
            pending_inline = Some((name, value.to_string(), brace_delta(value)));
            continue;
        }

        dependencies.push(CargoDependency {
            name,
            version: if value.starts_with('{') {
                extract_version(value)
            } else {
                extract_quoted(value)
            },
        });
    }

    dependencies
}

fn brace_delta(value: &str) -> i32 {
    value.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn extract_version(value: &str) -> Option<String> {
    let version_idx = value.find("version")?;
    let rest = &value[version_idx + "version".len()..];
    let rest = rest.split_once('=')?.1.trim();
    extract_quoted(rest)
}

fn extract_quoted(value: &str) -> Option<String> {
    let start = value.find('"')?;
    let end = value[start + 1..].find('"')?;
    Some(value[start + 1..start + 1 + end].to_string())
}

fn classify_technology_category(name: &str) -> TechnologyCategory {
    let normalized = name.to_lowercase();
    if matches!(normalized.as_str(), "tokio" | "async-std") {
        TechnologyCategory::Runtime
    } else if normalized.contains("axum")
        || normalized.contains("actix")
        || normalized.contains("rocket")
        || normalized.contains("react")
        || normalized.contains("next")
    {
        TechnologyCategory::Framework
    } else if normalized.contains("serde")
        || normalized.contains("uuid")
        || normalized.contains("chrono")
        || normalized.contains("tower")
        || normalized.contains("tracing")
    {
        TechnologyCategory::Library
    } else if normalized.contains("postgres")
        || normalized.contains("mysql")
        || normalized.contains("sqlite")
    {
        TechnologyCategory::Platform
    } else {
        TechnologyCategory::Tool
    }
}

fn infer_component_type(path: &str) -> ComponentType {
    let normalized = path.to_lowercase();
    if normalized.contains("pipeline") {
        ComponentType::Pipeline
    } else if normalized.contains("api") || normalized.contains("service") || normalized.ends_with("server") {
        ComponentType::Service
    } else if normalized.contains("store") || normalized.contains("db") {
        ComponentType::Store
    } else if normalized.contains("web") || normalized.contains("ui") || normalized.contains("cli") {
        ComponentType::Interface
    } else if normalized.contains("core") || normalized.contains("lib") {
        ComponentType::Library
    } else {
        ComponentType::Module
    }
}

fn humanize_name(value: &str) -> String {
    value
        .split(['-', '_', '/'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("planner-discovery-{}-{}", prefix, Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn scan_cargo_toml_finds_dependencies() {
        let dir = temp_dir("cargo");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
                [package]
                name = "example"

                [dependencies]
                serde = "1.0"
                tokio = { version = "1.42", features = ["rt-multi-thread"] }
            "#,
        )
        .unwrap();

        let output = scan_cargo_toml(&dir, &BlueprintStore::new());
        let names: HashSet<String> = output
            .proposals
            .iter()
            .map(|proposal| proposal.node.name().to_string())
            .collect();

        assert!(names.contains("serde"));
        assert!(names.contains("tokio"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn scan_directory_finds_components() {
        let dir = temp_dir("dirs");
        std::fs::create_dir_all(dir.join("api")).unwrap();
        std::fs::create_dir_all(dir.join("core")).unwrap();
        std::fs::create_dir_all(dir.join("src/web")).unwrap();

        let output = scan_directory_structure(&dir, &BlueprintStore::new());
        let names: HashSet<String> = output
            .proposals
            .iter()
            .map(|proposal| proposal.node.name().to_string())
            .collect();

        assert!(names.contains("Api"));
        assert!(names.contains("Core"));
        assert!(names.contains("Web"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn proposal_store_persists_and_filters() {
        let dir = temp_dir("store");
        let store = ProposalStore::open(&dir).unwrap();
        let proposal = ProposedNode {
            id: Uuid::new_v4().to_string(),
            node: BlueprintNode::Technology(Technology {
                id: NodeId::with_prefix("TECH", "serde"),
                name: "serde".into(),
                version: Some("1.0".into()),
                category: TechnologyCategory::Library,
                ring: AdoptionRing::Adopt,
                rationale: "Detected in Cargo.toml".into(),
                license: None,
                tags: vec!["discovery".into()],
                documentation: None,
                created_at: now_iso(),
                updated_at: now_iso(),
            }),
            source: DiscoverySource::CargoToml,
            reason: "Dependency discovered".into(),
            status: ProposalStatus::Pending,
            proposed_at: now_iso(),
            reviewed_at: None,
            confidence: 0.9,
            source_artifact: Some("Cargo.toml".into()),
            review_note: None,
        };

        store.insert_many(vec![proposal.clone()]).unwrap();
        store.mark_rejected(&proposal.id, Some("not needed".into())).unwrap();

        let reopened = ProposalStore::open(&dir).unwrap();
        let rejected = reopened.list(Some(ProposalStatus::Rejected));
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].id, proposal.id);

        let _ = std::fs::remove_dir_all(dir);
    }
}
