//! # RBAC — Role-Based Access Control
//!
//! Defines roles, permissions, and team membership for multi-tenant access.
//!
//! ## Phases
//! - **Phase 1** (this file): Type definitions and `has_permission` helpers.
//! - **Phase 2**: Database-backed role assignment, JWT claim injection,
//!   and enforcement middleware.
//!
//! ## Role hierarchy
//! ```text
//! Admin    → full access
//! Operator → create/run/read, no delete or settings write
//! Viewer   → read-only
//! Service  → CI/CD accounts: run pipeline, read/export turns
//! ```

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

/// Top-level role assigned to a user or service account within a team.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Role {
    /// Full administrative access — can do everything.
    Admin,
    /// Operators can create and run pipelines but cannot delete sessions or
    /// change global settings.
    Operator,
    /// Read-only access to sessions, turns, and settings.
    Viewer,
    /// Service accounts used by CI/CD integrations.  May run pipelines and
    /// export turns but cannot create/delete sessions or access settings.
    Service,
}

// ---------------------------------------------------------------------------
// Permission
// ---------------------------------------------------------------------------

/// Fine-grained permission checked at the handler level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Create a new planning session.
    SessionCreate,
    /// Read an existing session (metadata + messages).
    SessionRead,
    /// Permanently delete a session.
    SessionDelete,
    /// Submit a description to kick off the pipeline.
    PipelineRun,
    /// Cancel a running pipeline.
    PipelineCancel,
    /// List / retrieve CXDB turns for a session.
    TurnsRead,
    /// Export CXDB turns (bulk download).
    TurnsExport,
    /// Read global or project settings.
    SettingsRead,
    /// Modify global or project settings.
    SettingsWrite,
}

// ---------------------------------------------------------------------------
// Role → Permissions mapping
// ---------------------------------------------------------------------------

impl Role {
    /// Return all permissions granted by this role.
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Role::Admin => vec![
                Permission::SessionCreate,
                Permission::SessionRead,
                Permission::SessionDelete,
                Permission::PipelineRun,
                Permission::PipelineCancel,
                Permission::TurnsRead,
                Permission::TurnsExport,
                Permission::SettingsRead,
                Permission::SettingsWrite,
            ],
            Role::Operator => vec![
                Permission::SessionCreate,
                Permission::SessionRead,
                Permission::PipelineRun,
                Permission::TurnsRead,
                Permission::TurnsExport,
                Permission::SettingsRead,
            ],
            Role::Viewer => vec![
                Permission::SessionRead,
                Permission::TurnsRead,
                Permission::SettingsRead,
            ],
            Role::Service => vec![
                Permission::PipelineRun,
                Permission::TurnsRead,
                Permission::TurnsExport,
            ],
        }
    }

    /// Returns `true` if this role grants `perm`.
    pub fn has_permission(&self, perm: &Permission) -> bool {
        self.permissions().contains(perm)
    }
}

// ---------------------------------------------------------------------------
// Team types
// ---------------------------------------------------------------------------

/// A single user's membership record within a team.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    /// Auth0 user sub (or other identity provider subject).
    pub user_id: String,
    /// Role assigned to this member.
    pub role: Role,
    /// Which team this membership belongs to.
    pub team_id: String,
}

/// A logical team grouping multiple users.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    /// Unique team identifier (UUID string).
    pub team_id: String,
    /// Human-readable team name.
    pub name: String,
    /// All members belonging to this team.
    pub members: Vec<TeamMember>,
}

impl Team {
    /// Look up a member's role by user ID.  Returns `None` if the user is
    /// not a member of the team.
    pub fn role_for(&self, user_id: &str) -> Option<&Role> {
        self.members
            .iter()
            .find(|m| m.user_id == user_id)
            .map(|m| &m.role)
    }

    /// Returns `true` if `user_id` has `perm` in this team.
    pub fn has_permission(&self, user_id: &str, perm: &Permission) -> bool {
        self.role_for(user_id)
            .map(|r| r.has_permission(perm))
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_has_all_permissions() {
        let all = vec![
            Permission::SessionCreate,
            Permission::SessionRead,
            Permission::SessionDelete,
            Permission::PipelineRun,
            Permission::PipelineCancel,
            Permission::TurnsRead,
            Permission::TurnsExport,
            Permission::SettingsRead,
            Permission::SettingsWrite,
        ];
        let role = Role::Admin;
        for perm in &all {
            assert!(
                role.has_permission(perm),
                "Admin should have {:?}",
                perm
            );
        }
    }

    #[test]
    fn viewer_cannot_create_session() {
        assert!(!Role::Viewer.has_permission(&Permission::SessionCreate));
    }

    #[test]
    fn viewer_cannot_delete_session() {
        assert!(!Role::Viewer.has_permission(&Permission::SessionDelete));
    }

    #[test]
    fn viewer_cannot_run_pipeline() {
        assert!(!Role::Viewer.has_permission(&Permission::PipelineRun));
    }

    #[test]
    fn viewer_can_read_session() {
        assert!(Role::Viewer.has_permission(&Permission::SessionRead));
    }

    #[test]
    fn viewer_can_read_turns() {
        assert!(Role::Viewer.has_permission(&Permission::TurnsRead));
    }

    #[test]
    fn operator_can_create_and_run() {
        assert!(Role::Operator.has_permission(&Permission::SessionCreate));
        assert!(Role::Operator.has_permission(&Permission::PipelineRun));
    }

    #[test]
    fn operator_cannot_delete_session() {
        assert!(!Role::Operator.has_permission(&Permission::SessionDelete));
    }

    #[test]
    fn operator_cannot_write_settings() {
        assert!(!Role::Operator.has_permission(&Permission::SettingsWrite));
    }

    #[test]
    fn service_can_run_pipeline_and_export() {
        assert!(Role::Service.has_permission(&Permission::PipelineRun));
        assert!(Role::Service.has_permission(&Permission::TurnsExport));
    }

    #[test]
    fn service_cannot_create_session() {
        assert!(!Role::Service.has_permission(&Permission::SessionCreate));
    }

    #[test]
    fn service_cannot_read_settings() {
        assert!(!Role::Service.has_permission(&Permission::SettingsRead));
    }

    #[test]
    fn team_role_for_returns_correct_role() {
        let team = Team {
            team_id: "team-1".into(),
            name: "Alpha Team".into(),
            members: vec![
                TeamMember {
                    user_id: "alice".into(),
                    role: Role::Admin,
                    team_id: "team-1".into(),
                },
                TeamMember {
                    user_id: "bob".into(),
                    role: Role::Viewer,
                    team_id: "team-1".into(),
                },
            ],
        };

        assert_eq!(team.role_for("alice"), Some(&Role::Admin));
        assert_eq!(team.role_for("bob"), Some(&Role::Viewer));
        assert_eq!(team.role_for("charlie"), None);
    }

    #[test]
    fn team_has_permission_delegates_to_role() {
        let team = Team {
            team_id: "team-2".into(),
            name: "Beta Team".into(),
            members: vec![TeamMember {
                user_id: "diana".into(),
                role: Role::Operator,
                team_id: "team-2".into(),
            }],
        };

        assert!(team.has_permission("diana", &Permission::PipelineRun));
        assert!(!team.has_permission("diana", &Permission::SettingsWrite));
        assert!(!team.has_permission("unknown_user", &Permission::SessionRead));
    }

    #[test]
    fn all_roles_have_nonempty_permissions() {
        for role in [Role::Admin, Role::Operator, Role::Viewer, Role::Service] {
            assert!(!role.permissions().is_empty(), "{:?} has no permissions", role);
        }
    }
}
