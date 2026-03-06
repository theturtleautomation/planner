//! # Living System Blueprint — Data Model
//!
//! Typed dependency graph where every node is an editable parameter that
//! influences system design.  Editing any node triggers impact preview,
//! user confirmation, and AI reconvergence of affected downstream nodes.
//!
//! ## Node Types
//!
//! | Type               | Shape (UI) | ID Prefix |
//! |--------------------|------------|-----------|
//! | Decision           | rounded rect | DEC-      |
//! | Technology         | hexagon      | TECH-     |
//! | Component          | sharp rect   | COMP-     |
//! | Constraint         | diamond      | CON-      |
//! | Pattern            | ellipse      | PAT-      |
//! | Quality Requirement| shield       | QR-       |
//!
//! ## Edge Types
//!
//! | Edge          | Source → Target                              |
//! |---------------|----------------------------------------------|
//! | decided_by    | Tech/Comp/Pattern → Decision                 |
//! | supersedes    | Decision → Decision                          |
//! | depends_on    | Component → Component                        |
//! | uses          | Component → Technology                       |
//! | constrains    | Constraint → Decision/Comp/Tech              |
//! | implements    | Component → Pattern                          |
//! | satisfies     | Decision/Pattern → QualityRequirement        |
//! | affects       | Decision → Component/Technology              |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Node ID — human-readable slug with UUID suffix
// ---------------------------------------------------------------------------

/// A globally unique, human-readable node identifier.
///
/// Format: `{slug}-{uuid8}` where slug is a kebab-case summary and uuid8 is
/// the first 8 hex chars of a UUID v4.  Examples:
/// - `use-messagepack-a1b2c3d4`
/// - `rust-core-engine-e5f6a7b8`
///
/// NodeIds are serialized as plain strings for JSON/MessagePack compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new NodeId from a human-readable slug.
    /// Appends the first 8 hex chars of a fresh UUID for uniqueness.
    pub fn new(slug: &str) -> Self {
        let uuid_prefix = &Uuid::new_v4().to_string()[..8];
        let clean = slug
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
            .trim_matches('-')
            .to_string();
        NodeId(format!("{}-{}", clean, uuid_prefix))
    }

    /// Create a NodeId with a specific prefix (e.g. "DEC", "TECH") and slug.
    pub fn with_prefix(prefix: &str, slug: &str) -> Self {
        let uuid_prefix = &Uuid::new_v4().to_string()[..8];
        let clean = slug
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
            .trim_matches('-')
            .to_string();
        NodeId(format!(
            "{}-{}-{}",
            prefix.to_lowercase(),
            clean,
            uuid_prefix
        ))
    }

    /// Parse a NodeId from an existing string (no generation).
    pub fn from_raw(raw: impl Into<String>) -> Self {
        NodeId(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Edge types
// ---------------------------------------------------------------------------

/// The semantic type of a relationship between two Blueprint nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Technology/Component/Pattern → Decision: "exists because of"
    DecidedBy,
    /// Decision → Decision: "replaces"
    Supersedes,
    /// Component → Component: "needs at runtime/build"
    DependsOn,
    /// Component → Technology: "is built with"
    Uses,
    /// Constraint → Decision/Component/Technology: "limits"
    Constrains,
    /// Component → Pattern: "follows"
    Implements,
    /// Decision/Pattern → QualityRequirement: "achieves"
    Satisfies,
    /// Decision → Component/Technology: "changing impacts"
    Affects,
}

impl EdgeType {
    /// All edge type variants for iteration.
    pub const ALL: &'static [EdgeType] = &[
        EdgeType::DecidedBy,
        EdgeType::Supersedes,
        EdgeType::DependsOn,
        EdgeType::Uses,
        EdgeType::Constrains,
        EdgeType::Implements,
        EdgeType::Satisfies,
        EdgeType::Affects,
    ];
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeType::DecidedBy => write!(f, "decided_by"),
            EdgeType::Supersedes => write!(f, "supersedes"),
            EdgeType::DependsOn => write!(f, "depends_on"),
            EdgeType::Uses => write!(f, "uses"),
            EdgeType::Constrains => write!(f, "constrains"),
            EdgeType::Implements => write!(f, "implements"),
            EdgeType::Satisfies => write!(f, "satisfies"),
            EdgeType::Affects => write!(f, "affects"),
        }
    }
}

// ---------------------------------------------------------------------------
// Edge
// ---------------------------------------------------------------------------

/// A typed, directional edge between two Blueprint nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub source: NodeId,
    pub target: NodeId,
    pub edge_type: EdgeType,
    /// Optional context for the edge (e.g. "technology choice" for decided_by).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

// ---------------------------------------------------------------------------
// Shared enums used across node types
// ---------------------------------------------------------------------------

/// Decision status lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Proposed,
    Accepted,
    Superseded,
    Deprecated,
}

/// Technology adoption ring (ThoughtWorks Radar pattern).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdoptionRing {
    Adopt,
    Trial,
    Assess,
    Hold,
}

/// Technology category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TechnologyCategory {
    Language,
    Framework,
    Library,
    Runtime,
    Tool,
    Platform,
    Protocol,
}

/// Component type within the system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    Module,
    Service,
    Library,
    Store,
    Interface,
    Pipeline,
}

/// Component lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Planned,
    InProgress,
    Shipped,
    Deprecated,
}

/// Constraint type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    Technical,
    Organizational,
    Philosophical,
    Regulatory,
}

/// Quality attribute category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityAttribute {
    Performance,
    Reliability,
    Security,
    Usability,
    Maintainability,
}

/// Quality requirement priority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// Primary scope class for a knowledge record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeClass {
    Global,
    Project,
    ProjectContextual,
    Unscoped,
}

impl Default for ScopeClass {
    fn default() -> Self {
        Self::Unscoped
    }
}

/// Visibility label used in project views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeVisibility {
    Shared,
    ProjectLocal,
    Unscoped,
}

/// Primary software project scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectScope {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
}

/// Optional narrower working context inside a project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecondaryScopeRefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
}

/// Rules for how shared knowledge appears in project views.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharedScope {
    /// Projects that explicitly link this shared record.
    #[serde(default)]
    pub linked_project_ids: Vec<String>,
    /// Shared records are inherited into linked project views by default.
    #[serde(default = "default_true")]
    pub inherit_to_linked_projects: bool,
}

/// Scope payload attached to every node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeScope {
    #[serde(default)]
    pub scope_class: ScopeClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectScope>,
    #[serde(default)]
    pub secondary: SecondaryScopeRefs,
    #[serde(default)]
    pub is_shared: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<SharedScope>,
}

fn default_true() -> bool {
    true
}

impl Default for NodeScope {
    fn default() -> Self {
        Self::unscoped()
    }
}

impl NodeScope {
    pub fn unscoped() -> Self {
        Self {
            scope_class: ScopeClass::Unscoped,
            project: None,
            secondary: SecondaryScopeRefs::default(),
            is_shared: false,
            shared: None,
        }
    }

    pub fn visibility(&self) -> ScopeVisibility {
        if matches!(self.scope_class, ScopeClass::Unscoped) {
            ScopeVisibility::Unscoped
        } else if self.is_shared {
            ScopeVisibility::Shared
        } else {
            ScopeVisibility::ProjectLocal
        }
    }
}

// ---------------------------------------------------------------------------
// Node Types — one struct per Blueprint node kind
// ---------------------------------------------------------------------------

/// An option considered during a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    pub name: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub chosen: bool,
}

/// A consequence of a decision — positive or negative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consequence {
    pub description: String,
    /// true = positive, false = negative
    pub positive: bool,
}

/// An assumption embedded in a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assumption {
    pub description: String,
    /// high / medium / low
    pub confidence: String,
}

/// Architectural decision with rationale (MADR variant).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: NodeId,
    pub title: String,
    pub status: DecisionStatus,
    pub context: String,
    pub options: Vec<DecisionOption>,
    pub consequences: Vec<Consequence>,
    pub assumptions: Vec<Assumption>,
    /// Previous decision this replaces, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<NodeId>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Freeform markdown documentation attached to this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(default)]
    pub scope: NodeScope,
    pub created_at: String,
    pub updated_at: String,
}

/// A specific technology, framework, library, or tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Technology {
    pub id: NodeId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub category: TechnologyCategory,
    pub ring: AdoptionRing,
    pub rationale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Freeform markdown documentation attached to this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(default)]
    pub scope: NodeScope,
    pub created_at: String,
    pub updated_at: String,
}

/// A logical building block of the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: NodeId,
    pub name: String,
    pub component_type: ComponentType,
    pub description: String,
    /// APIs/interfaces this component exposes.
    #[serde(default)]
    pub provides: Vec<String>,
    /// APIs/interfaces this component consumes.
    #[serde(default)]
    pub consumes: Vec<String>,
    pub status: ComponentStatus,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Freeform markdown documentation attached to this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(default)]
    pub scope: NodeScope,
    pub created_at: String,
    pub updated_at: String,
}

/// An external force that narrows the solution space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: NodeId,
    pub title: String,
    pub constraint_type: ConstraintType,
    pub description: String,
    /// Who/what imposed this constraint.
    pub source: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Freeform markdown documentation attached to this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(default)]
    pub scope: NodeScope,
    pub created_at: String,
    pub updated_at: String,
}

/// An architectural pattern or design principle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: NodeId,
    pub name: String,
    pub description: String,
    pub rationale: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Freeform markdown documentation attached to this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(default)]
    pub scope: NodeScope,
    pub created_at: String,
    pub updated_at: String,
}

/// A measurable quality attribute the system must satisfy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRequirement {
    pub id: NodeId,
    pub attribute: QualityAttribute,
    /// Specific, testable scenario.
    pub scenario: String,
    pub priority: QualityPriority,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Freeform markdown documentation attached to this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(default)]
    pub scope: NodeScope,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// BlueprintNode — enum wrapper for all node types
// ---------------------------------------------------------------------------

/// A Blueprint node — one of six architectural element types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "node_type", rename_all = "snake_case")]
pub enum BlueprintNode {
    Decision(Decision),
    Technology(Technology),
    Component(Component),
    Constraint(Constraint),
    Pattern(Pattern),
    QualityRequirement(QualityRequirement),
}

impl BlueprintNode {
    /// Return the NodeId of this node.
    pub fn id(&self) -> &NodeId {
        match self {
            BlueprintNode::Decision(n) => &n.id,
            BlueprintNode::Technology(n) => &n.id,
            BlueprintNode::Component(n) => &n.id,
            BlueprintNode::Constraint(n) => &n.id,
            BlueprintNode::Pattern(n) => &n.id,
            BlueprintNode::QualityRequirement(n) => &n.id,
        }
    }

    /// Return the display name of this node.
    pub fn name(&self) -> &str {
        match self {
            BlueprintNode::Decision(n) => &n.title,
            BlueprintNode::Technology(n) => &n.name,
            BlueprintNode::Component(n) => &n.name,
            BlueprintNode::Constraint(n) => &n.title,
            BlueprintNode::Pattern(n) => &n.name,
            BlueprintNode::QualityRequirement(n) => &n.scenario,
        }
    }

    /// Return the type name as a static string (for filtering/display).
    pub fn type_name(&self) -> &'static str {
        match self {
            BlueprintNode::Decision(_) => "decision",
            BlueprintNode::Technology(_) => "technology",
            BlueprintNode::Component(_) => "component",
            BlueprintNode::Constraint(_) => "constraint",
            BlueprintNode::Pattern(_) => "pattern",
            BlueprintNode::QualityRequirement(_) => "quality_requirement",
        }
    }

    /// Return updated_at timestamp.
    pub fn updated_at(&self) -> &str {
        match self {
            BlueprintNode::Decision(n) => &n.updated_at,
            BlueprintNode::Technology(n) => &n.updated_at,
            BlueprintNode::Component(n) => &n.updated_at,
            BlueprintNode::Constraint(n) => &n.updated_at,
            BlueprintNode::Pattern(n) => &n.updated_at,
            BlueprintNode::QualityRequirement(n) => &n.updated_at,
        }
    }

    /// Return tags for this node.
    pub fn tags(&self) -> &[String] {
        match self {
            BlueprintNode::Decision(n) => &n.tags,
            BlueprintNode::Technology(n) => &n.tags,
            BlueprintNode::Component(n) => &n.tags,
            BlueprintNode::Constraint(n) => &n.tags,
            BlueprintNode::Pattern(n) => &n.tags,
            BlueprintNode::QualityRequirement(n) => &n.tags,
        }
    }

    /// Return the optional markdown documentation attached to this node.
    pub fn documentation(&self) -> Option<&str> {
        match self {
            BlueprintNode::Decision(n) => n.documentation.as_deref(),
            BlueprintNode::Technology(n) => n.documentation.as_deref(),
            BlueprintNode::Component(n) => n.documentation.as_deref(),
            BlueprintNode::Constraint(n) => n.documentation.as_deref(),
            BlueprintNode::Pattern(n) => n.documentation.as_deref(),
            BlueprintNode::QualityRequirement(n) => n.documentation.as_deref(),
        }
    }

    /// Return the scope metadata attached to this node.
    pub fn scope(&self) -> &NodeScope {
        match self {
            BlueprintNode::Decision(n) => &n.scope,
            BlueprintNode::Technology(n) => &n.scope,
            BlueprintNode::Component(n) => &n.scope,
            BlueprintNode::Constraint(n) => &n.scope,
            BlueprintNode::Pattern(n) => &n.scope,
            BlueprintNode::QualityRequirement(n) => &n.scope,
        }
    }

    /// Return a normalized status string for display.
    ///
    /// Each node type has a different lifecycle field; this maps them all
    /// to a single human-readable string for table/filter UIs.
    pub fn status(&self) -> String {
        match self {
            BlueprintNode::Decision(n) => match n.status {
                DecisionStatus::Proposed => "Proposed".into(),
                DecisionStatus::Accepted => "Accepted".into(),
                DecisionStatus::Superseded => "Superseded".into(),
                DecisionStatus::Deprecated => "Deprecated".into(),
            },
            BlueprintNode::Technology(n) => match n.ring {
                AdoptionRing::Adopt => "Adopt".into(),
                AdoptionRing::Trial => "Trial".into(),
                AdoptionRing::Assess => "Assess".into(),
                AdoptionRing::Hold => "Hold".into(),
            },
            BlueprintNode::Component(n) => match n.status {
                ComponentStatus::Planned => "Planned".into(),
                ComponentStatus::InProgress => "In Progress".into(),
                ComponentStatus::Shipped => "Shipped".into(),
                ComponentStatus::Deprecated => "Deprecated".into(),
            },
            BlueprintNode::Constraint(n) => match n.constraint_type {
                ConstraintType::Technical => "Technical".into(),
                ConstraintType::Organizational => "Organizational".into(),
                ConstraintType::Philosophical => "Philosophical".into(),
                ConstraintType::Regulatory => "Regulatory".into(),
            },
            BlueprintNode::Pattern(_) => "Active".into(),
            BlueprintNode::QualityRequirement(n) => match n.priority {
                QualityPriority::Critical => "Critical".into(),
                QualityPriority::High => "High".into(),
                QualityPriority::Medium => "Medium".into(),
                QualityPriority::Low => "Low".into(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Impact Analysis types
// ---------------------------------------------------------------------------

/// What happens to a node during reconvergence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactAction {
    /// AI work required — re-evaluate the node.
    Reconverge,
    /// Metadata change only — no AI work.
    Update,
    /// Node is broken by the change — must be addressed.
    Invalidate,
    /// New node needs to be created.
    Add,
    /// Node should be removed.
    Remove,
}

/// Severity of the reconvergence impact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactSeverity {
    /// Metadata only — status change, description update.
    Shallow,
    /// Local reconverge — technology version bump, compatibility check.
    Medium,
    /// Full cascade — decision reversal, artifact rebuilds.
    Deep,
}

/// A single affected node in an impact report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEntry {
    pub node_id: NodeId,
    pub node_name: String,
    pub node_type: String,
    pub action: ImpactAction,
    pub severity: ImpactSeverity,
    pub explanation: String,
}

/// The full impact analysis result from proposing a change to a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub source_node_id: NodeId,
    pub source_node_name: String,
    pub change_description: String,
    pub entries: Vec<ImpactEntry>,
    /// Counts by action type for the summary line.
    pub summary: HashMap<String, usize>,
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Blueprint — the top-level graph structure
// ---------------------------------------------------------------------------

/// Node summary for list endpoints (avoids cloning full node data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: NodeId,
    pub name: String,
    pub node_type: String,
    /// Normalized status string derived from each node type's lifecycle field.
    pub status: String,
    pub scope_class: ScopeClass,
    pub scope_visibility: ScopeVisibility,
    #[serde(default)]
    pub is_shared: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(default)]
    pub secondary_scope: SecondaryScopeRefs,
    #[serde(default)]
    pub linked_project_ids: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub has_documentation: bool,
    pub updated_at: String,
}

impl From<&BlueprintNode> for NodeSummary {
    fn from(node: &BlueprintNode) -> Self {
        let scope = node.scope();
        let (project_id, project_name) = scope
            .project
            .as_ref()
            .map(|project| {
                (
                    Some(project.project_id.clone()),
                    project.project_name.clone(),
                )
            })
            .unwrap_or((None, None));
        let linked_project_ids = scope
            .shared
            .as_ref()
            .map(|shared| shared.linked_project_ids.clone())
            .unwrap_or_default();

        NodeSummary {
            id: node.id().clone(),
            name: node.name().to_string(),
            node_type: node.type_name().to_string(),
            status: node.status(),
            scope_class: scope.scope_class.clone(),
            scope_visibility: scope.visibility(),
            is_shared: scope.is_shared,
            project_id,
            project_name,
            secondary_scope: scope.secondary.clone(),
            linked_project_ids,
            tags: node.tags().to_vec(),
            has_documentation: node.documentation().is_some(),
            updated_at: node.updated_at().to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Event Sourcing
// ---------------------------------------------------------------------------

/// A single immutable event recording a mutation to the Blueprint graph.
///
/// Events are append-only and serve as the source of truth for the
/// blueprint's change history. The current state can always be
/// reconstructed by replaying events from the initial empty state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum BlueprintEvent {
    /// A node was created (first upsert for this ID).
    NodeCreated {
        node: BlueprintNode,
        timestamp: String,
    },
    /// A node was updated (upsert for an existing ID).
    NodeUpdated {
        node_id: String,
        before: BlueprintNode,
        after: BlueprintNode,
        timestamp: String,
    },
    /// A node and its incident edges were deleted.
    NodeDeleted {
        node_id: String,
        node: BlueprintNode,
        /// Edges that were removed along with the node.
        removed_edges: Vec<Edge>,
        timestamp: String,
    },
    /// An edge was created.
    EdgeCreated { edge: Edge, timestamp: String },
    /// One or more edges were deleted.
    EdgesDeleted { edges: Vec<Edge>, timestamp: String },
}

impl BlueprintEvent {
    /// The ISO-8601 timestamp of the event.
    pub fn timestamp(&self) -> &str {
        match self {
            Self::NodeCreated { timestamp, .. }
            | Self::NodeUpdated { timestamp, .. }
            | Self::NodeDeleted { timestamp, .. }
            | Self::EdgeCreated { timestamp, .. }
            | Self::EdgesDeleted { timestamp, .. } => timestamp,
        }
    }

    /// Short human-readable description of the event.
    pub fn summary(&self) -> String {
        match self {
            Self::NodeCreated { node, .. } => {
                format!("Created {} '{}'", node.type_name(), node.name())
            }
            Self::NodeUpdated { node_id, after, .. } => {
                format!("Updated {} '{}'", after.type_name(), node_id)
            }
            Self::NodeDeleted {
                node_id,
                node,
                removed_edges,
                ..
            } => {
                let edge_note = if removed_edges.is_empty() {
                    String::new()
                } else {
                    format!(" ({} edges removed)", removed_edges.len())
                };
                format!("Deleted {} '{}'{}", node.type_name(), node_id, edge_note)
            }
            Self::EdgeCreated { edge, .. } => {
                format!(
                    "Edge {} -[{}]-> {}",
                    edge.source, edge.edge_type, edge.target
                )
            }
            Self::EdgesDeleted { edges, .. } => {
                if edges.len() == 1 {
                    let e = &edges[0];
                    format!(
                        "Removed edge {} -[{}]-> {}",
                        e.source, e.edge_type, e.target
                    )
                } else {
                    format!("Removed {} edges", edges.len())
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_generation() {
        let id = NodeId::new("Use MessagePack");
        assert!(id.0.starts_with("use-messagepack-"));
        assert!(id.0.len() > "use-messagepack-".len());
    }

    #[test]
    fn node_id_with_prefix() {
        let id = NodeId::with_prefix("DEC", "native CLI LLM");
        assert!(id.0.starts_with("dec-native-cli-llm-"));
    }

    #[test]
    fn node_id_from_raw() {
        let id = NodeId::from_raw("dec-use-messagepack-a1b2c3d4");
        assert_eq!(id.as_str(), "dec-use-messagepack-a1b2c3d4");
    }

    #[test]
    fn edge_type_display() {
        assert_eq!(EdgeType::DecidedBy.to_string(), "decided_by");
        assert_eq!(EdgeType::DependsOn.to_string(), "depends_on");
        assert_eq!(EdgeType::Satisfies.to_string(), "satisfies");
    }

    #[test]
    fn edge_type_all_variants() {
        assert_eq!(EdgeType::ALL.len(), 8);
    }

    #[test]
    fn decision_serde_roundtrip() {
        let decision = Decision {
            id: NodeId::from_raw("dec-use-msgpack-a1b2c3d4"),
            title: "Use MessagePack for disk serialization".into(),
            status: DecisionStatus::Accepted,
            context: "CXDB needs a fast, compact disk format".into(),
            options: vec![
                DecisionOption {
                    name: "MessagePack".into(),
                    pros: vec!["Fast binary".into(), "Compact".into()],
                    cons: vec!["Not human-readable".into()],
                    chosen: true,
                },
                DecisionOption {
                    name: "SQLite".into(),
                    pros: vec!["ACID".into(), "Query capability".into()],
                    cons: vec!["Heavier runtime".into()],
                    chosen: false,
                },
            ],
            consequences: vec![
                Consequence {
                    description: "Minimal deserialization overhead".into(),
                    positive: true,
                },
                Consequence {
                    description: "Cannot query without full load".into(),
                    positive: false,
                },
            ],
            assumptions: vec![Assumption {
                description: "Data volumes stay small".into(),
                confidence: "medium".into(),
            }],
            supersedes: None,
            tags: vec!["storage".into(), "core".into()],
            documentation: Some("# Decision Notes".into()),
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        };

        let node = BlueprintNode::Decision(decision);

        // JSON roundtrip
        let json = serde_json::to_string(&node).unwrap();
        let decoded: BlueprintNode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id().as_str(), "dec-use-msgpack-a1b2c3d4");
        assert_eq!(decoded.type_name(), "decision");

        // MessagePack roundtrip (must use named to preserve serde tag keys)
        let bytes = rmp_serde::to_vec_named(&node).unwrap();
        let decoded_mp: BlueprintNode = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded_mp.name(), "Use MessagePack for disk serialization");
    }

    #[test]
    fn technology_serde_roundtrip() {
        let tech = Technology {
            id: NodeId::from_raw("tech-rust-e5f6a7b8"),
            name: "Rust".into(),
            version: Some("1.82".into()),
            category: TechnologyCategory::Language,
            ring: AdoptionRing::Adopt,
            rationale: "Memory safety without GC".into(),
            license: Some("MIT/Apache-2.0".into()),
            tags: vec!["core".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        };

        let node = BlueprintNode::Technology(tech);
        let json = serde_json::to_string(&node).unwrap();
        let decoded: BlueprintNode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.type_name(), "technology");
        assert_eq!(decoded.name(), "Rust");
    }

    #[test]
    fn component_serde_roundtrip() {
        let comp = Component {
            id: NodeId::from_raw("comp-cxdb-c1d2e3f4"),
            name: "CXDB".into(),
            component_type: ComponentType::Store,
            description: "Conversation Experience Database".into(),
            provides: vec!["TurnStore API".into()],
            consumes: vec!["MessagePack serialization".into()],
            status: ComponentStatus::Shipped,
            tags: vec!["storage".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        };

        let node = BlueprintNode::Component(comp);
        let bytes = rmp_serde::to_vec_named(&node).unwrap();
        let decoded: BlueprintNode = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.name(), "CXDB");
        assert_eq!(decoded.type_name(), "component");
    }

    #[test]
    fn constraint_serde_roundtrip() {
        let con = Constraint {
            id: NodeId::from_raw("con-no-api-keys-d4e5f6a7"),
            title: "No HTTP API keys for LLM access".into(),
            constraint_type: ConstraintType::Philosophical,
            description: "LLM clients must use native CLIs".into(),
            source: "user directive".into(),
            tags: vec!["llm".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        };

        let node = BlueprintNode::Constraint(con);
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("constraint"));
        let decoded: BlueprintNode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.type_name(), "constraint");
    }

    #[test]
    fn quality_requirement_serde_roundtrip() {
        let qr = QualityRequirement {
            id: NodeId::from_raw("qr-crash-safe-f7a8b9c0"),
            attribute: QualityAttribute::Reliability,
            scenario: "Session recovery on restart completes within 2s for 1000 sessions".into(),
            priority: QualityPriority::Critical,
            tags: vec!["persistence".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        };

        let node = BlueprintNode::QualityRequirement(qr);
        let json = serde_json::to_string(&node).unwrap();
        let decoded: BlueprintNode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.type_name(), "quality_requirement");
    }

    #[test]
    fn pattern_serde_roundtrip() {
        let pat = Pattern {
            id: NodeId::from_raw("pat-event-sourcing-b2c3d4e5"),
            name: "Event Sourcing".into(),
            description: "Store events, reconstruct state on demand".into(),
            rationale: "Full audit trail, time-travel debugging".into(),
            tags: vec!["persistence".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        };

        let node = BlueprintNode::Pattern(pat);
        let bytes = rmp_serde::to_vec_named(&node).unwrap();
        let decoded: BlueprintNode = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.name(), "Event Sourcing");
    }

    #[test]
    fn edge_serde_roundtrip() {
        let edge = Edge {
            source: NodeId::from_raw("comp-cxdb-c1d2e3f4"),
            target: NodeId::from_raw("tech-rust-e5f6a7b8"),
            edge_type: EdgeType::Uses,
            metadata: Some("primary language".into()),
        };

        let json = serde_json::to_string(&edge).unwrap();
        let decoded: Edge = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.edge_type, EdgeType::Uses);
        assert_eq!(decoded.metadata.as_deref(), Some("primary language"));
    }

    #[test]
    fn impact_report_structure() {
        let report = ImpactReport {
            source_node_id: NodeId::from_raw("dec-use-msgpack-a1b2c3d4"),
            source_node_name: "Use MessagePack".into(),
            change_description: "Switch to SQLite".into(),
            entries: vec![
                ImpactEntry {
                    node_id: NodeId::from_raw("comp-cxdb-c1d2e3f4"),
                    node_name: "CXDB".into(),
                    node_type: "component".into(),
                    action: ImpactAction::Reconverge,
                    severity: ImpactSeverity::Deep,
                    explanation: "Storage implementation must be rewritten".into(),
                },
                ImpactEntry {
                    node_id: NodeId::from_raw("tech-rmp-serde-a2b3c4d5"),
                    node_name: "rmp-serde".into(),
                    node_type: "technology".into(),
                    action: ImpactAction::Remove,
                    severity: ImpactSeverity::Medium,
                    explanation: "No longer needed".into(),
                },
            ],
            summary: {
                let mut m = HashMap::new();
                m.insert("reconverge".into(), 1);
                m.insert("remove".into(), 1);
                m
            },
            timestamp: "2026-03-05T12:00:00Z".into(),
        };

        let json = serde_json::to_string(&report).unwrap();
        let decoded: ImpactReport = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.entries.len(), 2);
        assert_eq!(decoded.entries[0].action, ImpactAction::Reconverge);
        assert_eq!(decoded.entries[0].severity, ImpactSeverity::Deep);
    }

    #[test]
    fn node_summary_from_blueprint_node() {
        let node = BlueprintNode::Technology(Technology {
            id: NodeId::from_raw("tech-tokio-a1a1a1a1"),
            name: "Tokio".into(),
            version: Some("1.38".into()),
            category: TechnologyCategory::Runtime,
            ring: AdoptionRing::Adopt,
            rationale: "Async runtime for Rust".into(),
            license: Some("MIT".into()),
            tags: vec!["async".into(), "core".into()],
            documentation: Some("Runtime selection notes".into()),
            scope: NodeScope {
                scope_class: ScopeClass::ProjectContextual,
                project: Some(ProjectScope {
                    project_id: "proj-planner".into(),
                    project_name: Some("Planner".into()),
                }),
                secondary: SecondaryScopeRefs {
                    feature: Some("pipeline".into()),
                    widget: None,
                    artifact: None,
                    component: Some("runtime".into()),
                },
                is_shared: true,
                shared: Some(SharedScope {
                    linked_project_ids: vec!["proj-planner".into(), "proj-shared".into()],
                    inherit_to_linked_projects: true,
                }),
            },
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-02T00:00:00Z".into(),
        });

        let summary = NodeSummary::from(&node);
        assert_eq!(summary.name, "Tokio");
        assert_eq!(summary.node_type, "technology");
        assert_eq!(summary.status, "Adopt");
        assert_eq!(summary.tags, vec!["async", "core"]);
        assert!(summary.has_documentation);
        assert_eq!(summary.scope_class, ScopeClass::ProjectContextual);
        assert_eq!(summary.scope_visibility, ScopeVisibility::Shared);
        assert_eq!(summary.project_id.as_deref(), Some("proj-planner"));
        assert_eq!(summary.secondary_scope.feature.as_deref(), Some("pipeline"));
        assert_eq!(
            summary.linked_project_ids,
            vec!["proj-planner", "proj-shared"]
        );
    }

    #[test]
    fn node_with_documentation_roundtrip() {
        let node = BlueprintNode::Pattern(Pattern {
            id: NodeId::from_raw("pat-docs-a1b2c3d4"),
            name: "Documented Pattern".into(),
            description: "A pattern with docs".into(),
            rationale: "Tests markdown persistence".into(),
            tags: vec!["docs".into()],
            documentation: Some("## Notes\n\n- one".into()),
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        });

        let bytes = rmp_serde::to_vec_named(&node).unwrap();
        let decoded: BlueprintNode = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.documentation(), Some("## Notes\n\n- one"));
    }

    #[test]
    fn node_without_documentation_defaults_to_none() {
        let raw = serde_json::json!({
            "node_type": "technology",
            "id": "tech-no-docs",
            "name": "No Docs",
            "category": "tool",
            "ring": "adopt",
            "rationale": "Back compat",
            "tags": [],
            "created_at": "2026-03-01T00:00:00Z",
            "updated_at": "2026-03-01T00:00:00Z"
        });

        let decoded: BlueprintNode = serde_json::from_value(raw).unwrap();
        assert_eq!(decoded.documentation(), None);
        assert!(matches!(decoded.scope().scope_class, ScopeClass::Unscoped));
        assert_eq!(decoded.scope().visibility(), ScopeVisibility::Unscoped);
    }

    #[test]
    fn node_scope_class_roundtrip() {
        let node = BlueprintNode::Decision(Decision {
            id: NodeId::from_raw("dec-scope-roundtrip"),
            title: "Scoped Decision".into(),
            status: DecisionStatus::Accepted,
            context: "Validate scope serialization".into(),
            options: vec![],
            consequences: vec![],
            assumptions: vec![],
            supersedes: None,
            tags: vec![],
            documentation: None,
            scope: NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "proj-demo".into(),
                    project_name: Some("Demo".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
            },
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        });

        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["scope"]["scope_class"], "project");
        assert_eq!(json["scope"]["project"]["project_id"], "proj-demo");
    }
}
