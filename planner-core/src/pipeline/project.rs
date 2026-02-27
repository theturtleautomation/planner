//! # Multi-Project Support
//!
//! Phase 5 adds multi-project support: a single Planner instance can manage
//! multiple projects with isolated state and cross-project queries.
//!
//! ## Features
//! - ProjectRegistry: register, list, and look up projects
//! - Per-project isolation: each project has its own set of runs
//! - Cross-project queries: find patterns across projects (e.g. shared dependencies)
//! - Project metadata: name, created_at, tags, status

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Project Registry
// ---------------------------------------------------------------------------

/// Metadata for a registered project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Unique project ID.
    pub project_id: Uuid,

    /// Human-readable project name.
    pub name: String,

    /// Short slug (used in file paths, URLs).
    pub slug: String,

    /// When the project was registered.
    pub created_at: DateTime<Utc>,

    /// Project status.
    pub status: ProjectStatus,

    /// Optional tags for categorization.
    pub tags: Vec<String>,

    /// Count of runs associated with this project.
    pub run_count: usize,
}

/// Project lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    /// Actively being worked on.
    Active,
    /// Development paused.
    Paused,
    /// Project completed.
    Completed,
    /// Project archived (read-only).
    Archived,
}

/// Registry managing all projects in the Planner instance.
pub struct ProjectRegistry {
    projects: HashMap<Uuid, ProjectInfo>,
    slug_index: HashMap<String, Uuid>,
}

impl ProjectRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        ProjectRegistry {
            projects: HashMap::new(),
            slug_index: HashMap::new(),
        }
    }

    /// Register a new project. Returns error if slug already exists.
    pub fn register(
        &mut self,
        name: String,
        slug: String,
        tags: Vec<String>,
    ) -> Result<ProjectInfo, ProjectError> {
        if self.slug_index.contains_key(&slug) {
            return Err(ProjectError::SlugAlreadyExists(slug));
        }

        let project = ProjectInfo {
            project_id: Uuid::new_v4(),
            name,
            slug: slug.clone(),
            created_at: Utc::now(),
            status: ProjectStatus::Active,
            tags,
            run_count: 0,
        };

        let id = project.project_id;
        self.slug_index.insert(slug, id);
        self.projects.insert(id, project.clone());

        Ok(project)
    }

    /// Look up a project by ID.
    pub fn get(&self, project_id: Uuid) -> Option<&ProjectInfo> {
        self.projects.get(&project_id)
    }

    /// Look up a project by slug.
    pub fn get_by_slug(&self, slug: &str) -> Option<&ProjectInfo> {
        self.slug_index.get(slug)
            .and_then(|id| self.projects.get(id))
    }

    /// List all projects, optionally filtered by status.
    pub fn list(&self, status_filter: Option<ProjectStatus>) -> Vec<&ProjectInfo> {
        self.projects.values()
            .filter(|p| {
                status_filter.as_ref().map(|s| &p.status == s).unwrap_or(true)
            })
            .collect()
    }

    /// Update a project's status.
    pub fn update_status(
        &mut self,
        project_id: Uuid,
        status: ProjectStatus,
    ) -> Result<(), ProjectError> {
        let project = self.projects.get_mut(&project_id)
            .ok_or(ProjectError::NotFound(project_id))?;
        project.status = status;
        Ok(())
    }

    /// Increment the run count for a project.
    pub fn increment_run_count(&mut self, project_id: Uuid) -> Result<(), ProjectError> {
        let project = self.projects.get_mut(&project_id)
            .ok_or(ProjectError::NotFound(project_id))?;
        project.run_count += 1;
        Ok(())
    }

    /// Find projects by tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<&ProjectInfo> {
        self.projects.values()
            .filter(|p| p.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Get total number of registered projects.
    pub fn count(&self) -> usize {
        self.projects.len()
    }

    /// Cross-project dependency analysis: find projects that share
    /// common dependency patterns (e.g. multiple projects using Stripe).
    pub fn find_shared_tags(&self) -> HashMap<String, Vec<Uuid>> {
        let mut tag_map: HashMap<String, Vec<Uuid>> = HashMap::new();
        for project in self.projects.values() {
            for tag in &project.tags {
                tag_map.entry(tag.clone()).or_default().push(project.project_id);
            }
        }
        // Only return tags shared by multiple projects
        tag_map.retain(|_, ids| ids.len() > 1);
        tag_map
    }
}

/// Project registry errors.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Project not found: {0}")]
    NotFound(Uuid),

    #[error("Project slug already exists: {0}")]
    SlugAlreadyExists(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_get_project() {
        let mut registry = ProjectRegistry::new();
        let project = registry.register(
            "My App".into(),
            "my-app".into(),
            vec!["web".into(), "stripe".into()],
        ).unwrap();

        assert_eq!(project.name, "My App");
        assert_eq!(project.slug, "my-app");
        assert_eq!(project.status, ProjectStatus::Active);

        let fetched = registry.get(project.project_id).unwrap();
        assert_eq!(fetched.name, "My App");
    }

    #[test]
    fn get_by_slug() {
        let mut registry = ProjectRegistry::new();
        registry.register("App A".into(), "app-a".into(), vec![]).unwrap();

        assert!(registry.get_by_slug("app-a").is_some());
        assert!(registry.get_by_slug("app-b").is_none());
    }

    #[test]
    fn duplicate_slug_rejected() {
        let mut registry = ProjectRegistry::new();
        registry.register("App A".into(), "my-slug".into(), vec![]).unwrap();

        let result = registry.register("App B".into(), "my-slug".into(), vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn list_with_status_filter() {
        let mut registry = ProjectRegistry::new();
        let _p1 = registry.register("Active 1".into(), "a1".into(), vec![]).unwrap();
        let p2 = registry.register("Active 2".into(), "a2".into(), vec![]).unwrap();
        registry.register("Archived".into(), "archived".into(), vec![]).unwrap();

        registry.update_status(p2.project_id, ProjectStatus::Archived).unwrap();

        let active = registry.list(Some(ProjectStatus::Active));
        assert_eq!(active.len(), 2); // p1 + "Archived" (initially active)

        // Actually p2 was archived, so let me count: p1=Active, p2=Archived, p3=Active
        // p1 Active, p2 Archived, Archived=Active (just named "Archived")
        // Let me fix the test logic:
        let all = registry.list(None);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn increment_run_count() {
        let mut registry = ProjectRegistry::new();
        let project = registry.register("App".into(), "app".into(), vec![]).unwrap();
        assert_eq!(project.run_count, 0);

        registry.increment_run_count(project.project_id).unwrap();
        registry.increment_run_count(project.project_id).unwrap();

        let fetched = registry.get(project.project_id).unwrap();
        assert_eq!(fetched.run_count, 2);
    }

    #[test]
    fn find_by_tag() {
        let mut registry = ProjectRegistry::new();
        registry.register("App A".into(), "a".into(), vec!["web".into(), "stripe".into()]).unwrap();
        registry.register("App B".into(), "b".into(), vec!["mobile".into(), "stripe".into()]).unwrap();
        registry.register("App C".into(), "c".into(), vec!["mobile".into()]).unwrap();

        let stripe_projects = registry.find_by_tag("stripe");
        assert_eq!(stripe_projects.len(), 2);

        let mobile_projects = registry.find_by_tag("mobile");
        assert_eq!(mobile_projects.len(), 2);

        let auth0_projects = registry.find_by_tag("auth0");
        assert_eq!(auth0_projects.len(), 0);
    }

    #[test]
    fn find_shared_tags() {
        let mut registry = ProjectRegistry::new();
        registry.register("A".into(), "a".into(), vec!["web".into(), "stripe".into()]).unwrap();
        registry.register("B".into(), "b".into(), vec!["stripe".into(), "auth0".into()]).unwrap();
        registry.register("C".into(), "c".into(), vec!["auth0".into()]).unwrap();

        let shared = registry.find_shared_tags();
        assert!(shared.contains_key("stripe")); // shared by A and B
        assert!(shared.contains_key("auth0"));  // shared by B and C
        assert!(!shared.contains_key("web"));   // only A
    }

    #[test]
    fn empty_registry() {
        let registry = ProjectRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.list(None).is_empty());
    }
}
