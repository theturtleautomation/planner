//! # planner.agents_manifest.v1
//!
//! AGENTS.md hierarchy generated from NLSpec chunks. The root NLSpec
//! becomes the root AGENTS.md; domain chunks become `docs/` files
//! loaded by the factory agent on demand.
//!
//! Follows the AGENTS.md spec: under 500 lines root file,
//! Jurisdiction / Accumulation / Precedence / Inheritance model.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// AgentsManifestV1
// ---------------------------------------------------------------------------

/// The generated AGENTS.md hierarchy for Kilroy's factory agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsManifestV1 {
    /// Which project this manifest belongs to.
    pub project_id: Uuid,

    /// Which NLSpec version this was generated from.
    pub nlspec_version: String,

    /// The root AGENTS.md content (under 500 lines).
    pub root_agents_md: String,

    /// Domain-specific doc files (e.g. "docs/auth.md", "docs/api.md").
    pub domain_docs: Vec<DomainDoc>,

    /// SKILL.md references (e.g. for common patterns).
    pub skill_refs: Vec<SkillRef>,
}

impl ArtifactPayload for AgentsManifestV1 {
    const TYPE_ID: &'static str = "planner.agents_manifest.v1";
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// A domain-specific documentation file loaded by factory agents on demand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainDoc {
    /// Relative path (e.g. "docs/auth.md").
    pub path: String,

    /// The markdown content.
    pub content: String,

    /// Which NLSpec chunk this was generated from.
    pub source_chunk: String,
}

/// A reference to a SKILL.md for common patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRef {
    /// Skill name (e.g. "stripe-checkout", "jwt-auth").
    pub name: String,

    /// Brief description of what the skill provides.
    pub description: String,

    /// Relative path to the skill file.
    pub path: String,
}
