//! # Blueprint Store — Memory-First, Disk-Backed Blueprint Graph
//!
//! Manages the Living System Blueprint: a typed dependency graph of
//! architectural decisions, technologies, components, constraints,
//! patterns, and quality requirements.
//!
//! ## Storage Layout
//!
//! ```text
//! {data_dir}/blueprint/
//!   ├── nodes/
//!   │   ├── {node_id}.msgpack       # One file per node
//!   │   └── ...
//!   ├── edges.msgpack               # All edges in a single file
//!   └── history/
//!       ├── {timestamp}.msgpack     # Snapshot before edit
//!       └── ...
//! ```
//!
//! ## Architecture
//!
//! Same pattern as SessionStore: memory-first, dirty tracking, periodic
//! flush, atomic write-then-rename with fsync for durability.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use planner_schemas::artifacts::blueprint::*;

// ---------------------------------------------------------------------------
// Blueprint — in-memory graph with precomputed adjacency
// ---------------------------------------------------------------------------

/// The in-memory Blueprint graph with precomputed adjacency indexes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub nodes: HashMap<String, BlueprintNode>,
    pub edges: Vec<Edge>,
    /// Precomputed: source → Vec<(EdgeType, target)>
    #[serde(skip)]
    forward_adj: HashMap<String, Vec<(EdgeType, String)>>,
    /// Precomputed: target → Vec<(EdgeType, source)>
    #[serde(skip)]
    reverse_adj: HashMap<String, Vec<(EdgeType, String)>>,
}

impl Default for Blueprint {
    fn default() -> Self {
        Self::new()
    }
}

impl Blueprint {
    pub fn new() -> Self {
        Blueprint {
            nodes: HashMap::new(),
            edges: Vec::new(),
            forward_adj: HashMap::new(),
            reverse_adj: HashMap::new(),
        }
    }

    /// Rebuild adjacency indexes from the edge list.
    /// Must be called after loading from disk or modifying edges.
    pub fn rebuild_indexes(&mut self) {
        self.forward_adj.clear();
        self.reverse_adj.clear();
        for edge in &self.edges {
            self.forward_adj
                .entry(edge.source.0.clone())
                .or_default()
                .push((edge.edge_type, edge.target.0.clone()));
            self.reverse_adj
                .entry(edge.target.0.clone())
                .or_default()
                .push((edge.edge_type, edge.source.0.clone()));
        }
    }

    // -----------------------------------------------------------------------
    // Node CRUD
    // -----------------------------------------------------------------------

    /// Insert or replace a node.
    pub fn upsert_node(&mut self, node: BlueprintNode) {
        let id = node.id().0.clone();
        self.nodes.insert(id, node);
    }

    /// Remove a node and all its incident edges.
    pub fn remove_node(&mut self, node_id: &str) -> Option<BlueprintNode> {
        let removed = self.nodes.remove(node_id);
        if removed.is_some() {
            self.edges.retain(|e| e.source.0 != node_id && e.target.0 != node_id);
            self.rebuild_indexes();
        }
        removed
    }

    /// Get a node by ID.
    pub fn get_node(&self, node_id: &str) -> Option<&BlueprintNode> {
        self.nodes.get(node_id)
    }

    /// Get a mutable reference to a node by ID.
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut BlueprintNode> {
        self.nodes.get_mut(node_id)
    }

    /// List all node summaries.
    pub fn list_summaries(&self) -> Vec<NodeSummary> {
        self.nodes.values().map(NodeSummary::from).collect()
    }

    /// List node summaries filtered by type.
    pub fn list_summaries_by_type(&self, type_name: &str) -> Vec<NodeSummary> {
        self.nodes
            .values()
            .filter(|n| n.type_name() == type_name)
            .map(NodeSummary::from)
            .collect()
    }

    /// Count nodes by type.
    pub fn counts_by_type(&self) -> HashMap<&'static str, usize> {
        let mut counts = HashMap::new();
        for node in self.nodes.values() {
            *counts.entry(node.type_name()).or_insert(0) += 1;
        }
        counts
    }

    // -----------------------------------------------------------------------
    // Edge CRUD
    // -----------------------------------------------------------------------

    /// Add an edge. Rebuilds indexes.
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
        self.rebuild_indexes();
    }

    /// Remove edges matching a predicate. Rebuilds indexes.
    pub fn remove_edges_where<F: Fn(&Edge) -> bool>(&mut self, predicate: F) -> usize {
        let before = self.edges.len();
        self.edges.retain(|e| !predicate(e));
        let removed = before - self.edges.len();
        if removed > 0 {
            self.rebuild_indexes();
        }
        removed
    }

    // -----------------------------------------------------------------------
    // Graph traversal
    // -----------------------------------------------------------------------

    /// Get forward neighbors (outgoing edges from this node).
    pub fn forward_neighbors(&self, node_id: &str) -> &[(EdgeType, String)] {
        self.forward_adj.get(node_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get reverse neighbors (incoming edges to this node).
    pub fn reverse_neighbors(&self, node_id: &str) -> &[(EdgeType, String)] {
        self.reverse_adj.get(node_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// BFS traversal downstream from a node (following forward edges).
    /// Returns all reachable node IDs in BFS order.
    pub fn downstream_bfs(&self, start: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        visited.insert(start.to_string());
        queue.push_back(start.to_string());

        while let Some(current) = queue.pop_front() {
            if current != start {
                result.push(current.clone());
            }
            for (_, neighbor) in self.forward_neighbors(&current) {
                if visited.insert(neighbor.clone()) {
                    queue.push_back(neighbor.clone());
                }
            }
        }
        result
    }

    /// BFS traversal upstream from a node (following reverse edges).
    pub fn upstream_bfs(&self, start: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        visited.insert(start.to_string());
        queue.push_back(start.to_string());

        while let Some(current) = queue.pop_front() {
            if current != start {
                result.push(current.clone());
            }
            for (_, neighbor) in self.reverse_neighbors(&current) {
                if visited.insert(neighbor.clone()) {
                    queue.push_back(neighbor.clone());
                }
            }
        }
        result
    }

    /// Topological sort of all nodes (Kahn's algorithm).
    /// Returns None if the graph has a cycle.
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

        // Initialize in-degree for all nodes.
        for id in self.nodes.keys() {
            in_degree.entry(id.as_str()).or_insert(0);
        }
        for edge in &self.edges {
            *in_degree.entry(edge.target.0.as_str()).or_insert(0) += 1;
        }

        // Collect nodes with in-degree 0.
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id.to_string())
            .collect();

        let mut sorted = Vec::new();
        while let Some(node) = queue.pop_front() {
            sorted.push(node.clone());
            for (_, neighbor) in self.forward_neighbors(&node) {
                if let Some(deg) = in_degree.get_mut(neighbor.as_str()) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        if sorted.len() == self.nodes.len() {
            Some(sorted)
        } else {
            None // cycle detected
        }
    }

    // -----------------------------------------------------------------------
    // Impact analysis
    // -----------------------------------------------------------------------

    /// Compute the impact of changing a node.
    ///
    /// Traverses downstream from the changed node and classifies each
    /// affected node by action (reconverge/update/invalidate) and severity
    /// (shallow/medium/deep).
    pub fn impact_analysis(&self, node_id: &str, change_description: &str) -> Option<ImpactReport> {
        let source_node = self.nodes.get(node_id)?;
        let source_name = source_node.name().to_string();

        let affected_ids = self.downstream_bfs(node_id);
        let mut entries = Vec::new();

        for affected_id in &affected_ids {
            if let Some(affected_node) = self.nodes.get(affected_id.as_str()) {
                // Classify the impact based on the edge type connecting them
                // and the affected node's type.
                let (action, severity) = self.classify_impact(node_id, affected_id);

                entries.push(ImpactEntry {
                    node_id: affected_node.id().clone(),
                    node_name: affected_node.name().to_string(),
                    node_type: affected_node.type_name().to_string(),
                    action,
                    severity,
                    explanation: self.explain_impact(node_id, affected_id),
                });
            }
        }

        // Build summary counts.
        let mut summary = HashMap::new();
        for entry in &entries {
            let key = match entry.action {
                ImpactAction::Reconverge => "reconverge",
                ImpactAction::Update => "update",
                ImpactAction::Invalidate => "invalidate",
                ImpactAction::Add => "add",
                ImpactAction::Remove => "remove",
            };
            *summary.entry(key.to_string()).or_insert(0) += 1;
        }

        Some(ImpactReport {
            source_node_id: NodeId::from_raw(node_id),
            source_node_name: source_name,
            change_description: change_description.to_string(),
            entries,
            summary,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Classify the impact action and severity for an affected node.
    fn classify_impact(&self, source_id: &str, affected_id: &str) -> (ImpactAction, ImpactSeverity) {
        // Find the direct edge types between source and affected.
        let direct_edges: Vec<EdgeType> = self.edges
            .iter()
            .filter(|e| e.source.0 == source_id && e.target.0 == affected_id)
            .map(|e| e.edge_type)
            .collect();

        let affected_node = match self.nodes.get(affected_id) {
            Some(n) => n,
            None => return (ImpactAction::Update, ImpactSeverity::Shallow),
        };

        // Direct dependency: more severe impact.
        if !direct_edges.is_empty() {
            let edge_type = direct_edges[0];
            match (edge_type, affected_node.type_name()) {
                // Decision affects a component → reconverge (deep)
                (EdgeType::Affects, "component") => (ImpactAction::Reconverge, ImpactSeverity::Deep),
                // Decision affects a technology → potentially remove/replace
                (EdgeType::Affects, "technology") => (ImpactAction::Update, ImpactSeverity::Medium),
                // Constraint constrains → re-validate compliance
                (EdgeType::Constrains, _) => (ImpactAction::Update, ImpactSeverity::Medium),
                // Supersedes → the old decision is invalidated
                (EdgeType::Supersedes, _) => (ImpactAction::Invalidate, ImpactSeverity::Deep),
                // decided_by/uses/implements → update metadata
                (EdgeType::DecidedBy, _) | (EdgeType::Uses, _) | (EdgeType::Implements, _) => {
                    (ImpactAction::Update, ImpactSeverity::Medium)
                }
                // satisfies → re-check quality
                (EdgeType::Satisfies, _) => (ImpactAction::Update, ImpactSeverity::Shallow),
                // depends_on → reconverge
                (EdgeType::DependsOn, _) => (ImpactAction::Reconverge, ImpactSeverity::Medium),
                _ => (ImpactAction::Update, ImpactSeverity::Shallow),
            }
        } else {
            // Indirect impact (transitive) — generally update/shallow.
            match affected_node.type_name() {
                "component" => (ImpactAction::Update, ImpactSeverity::Medium),
                "decision" => (ImpactAction::Update, ImpactSeverity::Shallow),
                _ => (ImpactAction::Update, ImpactSeverity::Shallow),
            }
        }
    }

    /// Generate a human-readable explanation of why a node is affected.
    fn explain_impact(&self, source_id: &str, affected_id: &str) -> String {
        let source_name = self.nodes.get(source_id)
            .map(|n| n.name().to_string())
            .unwrap_or_else(|| source_id.to_string());
        let affected_name = self.nodes.get(affected_id)
            .map(|n| n.name().to_string())
            .unwrap_or_else(|| affected_id.to_string());

        // Find direct edge.
        let direct_edge = self.edges.iter()
            .find(|e| e.source.0 == source_id && e.target.0 == affected_id);

        match direct_edge {
            Some(edge) => {
                format!(
                    "{} {} {} via {} relationship",
                    source_name,
                    edge.edge_type,
                    affected_name,
                    edge.edge_type,
                )
            }
            None => {
                format!(
                    "{} is transitively affected by changes to {}",
                    affected_name,
                    source_name,
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// BlueprintStore — thread-safe, disk-backed
// ---------------------------------------------------------------------------

/// Thread-safe, memory-first, disk-backed Blueprint store.
///
/// Same persistence pattern as SessionStore: in-memory HashMap + dirty
/// tracking + periodic flush with atomic rename + fsync.
pub struct BlueprintStore {
    blueprint: RwLock<Blueprint>,
    dirty: RwLock<bool>,
    blueprint_dir: Option<PathBuf>,
}

impl BlueprintStore {
    /// Create a purely in-memory store with no disk backing (for tests).
    pub fn new() -> Self {
        BlueprintStore {
            blueprint: RwLock::new(Blueprint::new()),
            dirty: RwLock::new(false),
            blueprint_dir: None,
        }
    }

    /// Open a disk-backed store from `data_dir/blueprint/`.
    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        let blueprint_dir = data_dir.join("blueprint");
        let nodes_dir = blueprint_dir.join("nodes");
        let history_dir = blueprint_dir.join("history");
        std::fs::create_dir_all(&nodes_dir)?;
        std::fs::create_dir_all(&history_dir)?;

        let mut blueprint = Blueprint::new();
        let mut load_errors = 0u32;

        // Load nodes.
        for entry in std::fs::read_dir(&nodes_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if !name.ends_with(".msgpack") || name.ends_with(".tmp") {
                continue;
            }

            match std::fs::read(&path) {
                Ok(bytes) => match rmp_serde::from_slice::<BlueprintNode>(&bytes) {
                    Ok(node) => {
                        let id = node.id().0.clone();
                        blueprint.nodes.insert(id, node);
                    }
                    Err(e) => {
                        tracing::error!("Failed to decode blueprint node {}: {}", name, e);
                        load_errors += 1;
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read blueprint node file {}: {}", name, e);
                    load_errors += 1;
                }
            }
        }

        // Load edges.
        let edges_path = blueprint_dir.join("edges.msgpack");
        if edges_path.exists() {
            match std::fs::read(&edges_path) {
                Ok(bytes) => match rmp_serde::from_slice::<Vec<Edge>>(&bytes) {
                    Ok(edges) => {
                        blueprint.edges = edges;
                    }
                    Err(e) => {
                        tracing::error!("Failed to decode blueprint edges: {}", e);
                        load_errors += 1;
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read blueprint edges file: {}", e);
                    load_errors += 1;
                }
            }
        }

        // Rebuild adjacency indexes.
        blueprint.rebuild_indexes();

        let count = blueprint.nodes.len();
        let edge_count = blueprint.edges.len();
        if load_errors > 0 {
            tracing::warn!(
                "Blueprint store: loaded {} nodes, {} edges, {} errors",
                count, edge_count, load_errors,
            );
        } else if count > 0 {
            tracing::info!(
                "Blueprint store: loaded {} nodes, {} edges from disk",
                count, edge_count,
            );
        }

        Ok(BlueprintStore {
            blueprint: RwLock::new(blueprint),
            dirty: RwLock::new(false),
            blueprint_dir: Some(blueprint_dir),
        })
    }

    // -----------------------------------------------------------------------
    // Read operations (no dirty marking)
    // -----------------------------------------------------------------------

    /// Get a snapshot of the full blueprint (clone).
    pub fn snapshot(&self) -> Blueprint {
        self.blueprint.read().clone()
    }

    /// Get a single node by ID.
    pub fn get_node(&self, node_id: &str) -> Option<BlueprintNode> {
        self.blueprint.read().nodes.get(node_id).cloned()
    }

    /// List all node summaries.
    pub fn list_summaries(&self) -> Vec<NodeSummary> {
        self.blueprint.read().list_summaries()
    }

    /// List node summaries filtered by type.
    pub fn list_by_type(&self, type_name: &str) -> Vec<NodeSummary> {
        self.blueprint.read().list_summaries_by_type(type_name)
    }

    /// Node and edge counts.
    pub fn counts(&self) -> (usize, usize) {
        let bp = self.blueprint.read();
        (bp.nodes.len(), bp.edges.len())
    }

    /// Counts by node type.
    pub fn counts_by_type(&self) -> HashMap<&'static str, usize> {
        self.blueprint.read().counts_by_type()
    }

    /// Forward neighbors.
    pub fn forward_neighbors(&self, node_id: &str) -> Vec<(EdgeType, String)> {
        self.blueprint.read().forward_neighbors(node_id).to_vec()
    }

    /// Reverse neighbors.
    pub fn reverse_neighbors(&self, node_id: &str) -> Vec<(EdgeType, String)> {
        self.blueprint.read().reverse_neighbors(node_id).to_vec()
    }

    /// Topological sort.
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        self.blueprint.read().topological_sort()
    }

    /// Impact analysis for a proposed node change.
    pub fn impact_analysis(&self, node_id: &str, change_description: &str) -> Option<ImpactReport> {
        self.blueprint.read().impact_analysis(node_id, change_description)
    }

    // -----------------------------------------------------------------------
    // Write operations (mark dirty)
    // -----------------------------------------------------------------------

    /// Insert or replace a node.
    pub fn upsert_node(&self, node: BlueprintNode) {
        self.blueprint.write().upsert_node(node);
        self.mark_dirty();
    }

    /// Remove a node and all incident edges.
    pub fn remove_node(&self, node_id: &str) -> Option<BlueprintNode> {
        let removed = self.blueprint.write().remove_node(node_id);
        if removed.is_some() {
            self.mark_dirty();
        }
        removed
    }

    /// Apply a mutation to a node. Returns the updated node.
    pub fn update_node<F>(&self, node_id: &str, f: F) -> Option<BlueprintNode>
    where
        F: FnOnce(&mut BlueprintNode),
    {
        let mut bp = self.blueprint.write();
        if let Some(node) = bp.nodes.get_mut(node_id) {
            f(node);
            let result = node.clone();
            drop(bp);
            self.mark_dirty();
            Some(result)
        } else {
            None
        }
    }

    /// Add an edge.
    pub fn add_edge(&self, edge: Edge) {
        self.blueprint.write().add_edge(edge);
        self.mark_dirty();
    }

    /// Remove edges matching a predicate.
    pub fn remove_edges_where<F: Fn(&Edge) -> bool>(&self, predicate: F) -> usize {
        let removed = self.blueprint.write().remove_edges_where(predicate);
        if removed > 0 {
            self.mark_dirty();
        }
        removed
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    fn mark_dirty(&self) {
        if self.blueprint_dir.is_some() {
            *self.dirty.write() = true;
        }
    }

    /// Check if there are unflushed changes.
    pub fn is_dirty(&self) -> bool {
        *self.dirty.read()
    }

    /// Flush the blueprint to disk if dirty.
    ///
    /// Uses atomic write-then-rename with fsync for durability.
    /// Only clears dirty flag after all writes succeed.
    pub fn flush(&self) -> std::io::Result<bool> {
        if !self.is_dirty() {
            return Ok(false);
        }

        let blueprint_dir = match &self.blueprint_dir {
            Some(d) => d,
            None => return Ok(false),
        };

        let nodes_dir = blueprint_dir.join("nodes");

        // Snapshot the blueprint under read lock, then release.
        let snapshot = self.blueprint.read().clone();

        // Write each node as a separate file.
        for (id, node) in &snapshot.nodes {
            let bytes = rmp_serde::to_vec_named(node)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            let final_path = nodes_dir.join(format!("{}.msgpack", id));
            let tmp_path = nodes_dir.join(format!("{}.msgpack.tmp", id));

            // Write + fsync + rename for durability.
            {
                let mut file = std::fs::File::create(&tmp_path)?;
                file.write_all(&bytes)?;
                file.sync_all()?; // fsync before rename
            }
            std::fs::rename(&tmp_path, &final_path)?;
        }

        // Clean up any node files that no longer exist in the graph.
        if let Ok(entries) = std::fs::read_dir(&nodes_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(".msgpack") && !name.ends_with(".tmp") {
                    let id = name.strip_suffix(".msgpack").unwrap_or("");
                    if !snapshot.nodes.contains_key(id) {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }

        // Write edges.
        {
            let bytes = rmp_serde::to_vec_named(&snapshot.edges)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let final_path = blueprint_dir.join("edges.msgpack");
            let tmp_path = blueprint_dir.join("edges.msgpack.tmp");

            let mut file = std::fs::File::create(&tmp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(&tmp_path, &final_path)?;
        }

        // Clear dirty flag only after all writes succeed.
        *self.dirty.write() = false;

        tracing::debug!(
            "Blueprint flush: {} nodes, {} edges written",
            snapshot.nodes.len(),
            snapshot.edges.len(),
        );

        Ok(true)
    }

    /// Save a history snapshot (before an edit).
    pub fn save_snapshot(&self) -> std::io::Result<()> {
        let blueprint_dir = match &self.blueprint_dir {
            Some(d) => d,
            None => return Ok(()),
        };

        let history_dir = blueprint_dir.join("history");
        let snapshot = self.blueprint.read().clone();
        let bytes = rmp_serde::to_vec_named(&snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ").to_string();
        let path = history_dir.join(format!("{}.msgpack", timestamp));

        let mut file = std::fs::File::create(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;

        Ok(())
    }

    /// Check if disk backing is enabled.
    pub fn is_persistent(&self) -> bool {
        self.blueprint_dir.is_some()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_blueprint() -> Blueprint {
        let mut bp = Blueprint::new();

        // Nodes
        bp.upsert_node(BlueprintNode::Decision(Decision {
            id: NodeId::from_raw("dec-use-msgpack"),
            title: "Use MessagePack for disk serialization".into(),
            status: DecisionStatus::Accepted,
            context: "Need fast compact format".into(),
            options: vec![],
            consequences: vec![],
            assumptions: vec![],
            supersedes: None,
            tags: vec![],
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        bp.upsert_node(BlueprintNode::Technology(Technology {
            id: NodeId::from_raw("tech-rmp-serde"),
            name: "rmp-serde".into(),
            version: Some("1.3".into()),
            category: TechnologyCategory::Library,
            ring: AdoptionRing::Adopt,
            rationale: "MessagePack serde support".into(),
            license: Some("MIT".into()),
            tags: vec![],
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        bp.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-cxdb"),
            name: "CXDB".into(),
            component_type: ComponentType::Store,
            description: "Conversation Experience Database".into(),
            provides: vec!["TurnStore".into()],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec![],
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        bp.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-event-store"),
            name: "EventStore".into(),
            component_type: ComponentType::Store,
            description: "Event persistence".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec![],
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        bp.upsert_node(BlueprintNode::Constraint(Constraint {
            id: NodeId::from_raw("con-fs-storage"),
            title: "File-system storage over SQLite".into(),
            constraint_type: ConstraintType::Philosophical,
            description: "Use filesystem, not SQLite".into(),
            source: "user directive".into(),
            tags: vec![],
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        bp.upsert_node(BlueprintNode::QualityRequirement(QualityRequirement {
            id: NodeId::from_raw("qr-crash-safe"),
            attribute: QualityAttribute::Reliability,
            scenario: "Crash-safe persistence".into(),
            priority: QualityPriority::Critical,
            tags: vec![],
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        // Edges
        // dec-use-msgpack --affects--> comp-cxdb
        bp.add_edge(Edge {
            source: NodeId::from_raw("dec-use-msgpack"),
            target: NodeId::from_raw("comp-cxdb"),
            edge_type: EdgeType::Affects,
            metadata: None,
        });
        // dec-use-msgpack --affects--> comp-event-store
        bp.add_edge(Edge {
            source: NodeId::from_raw("dec-use-msgpack"),
            target: NodeId::from_raw("comp-event-store"),
            edge_type: EdgeType::Affects,
            metadata: None,
        });
        // dec-use-msgpack --affects--> tech-rmp-serde
        bp.add_edge(Edge {
            source: NodeId::from_raw("dec-use-msgpack"),
            target: NodeId::from_raw("tech-rmp-serde"),
            edge_type: EdgeType::Affects,
            metadata: None,
        });
        // comp-cxdb --uses--> tech-rmp-serde
        bp.add_edge(Edge {
            source: NodeId::from_raw("comp-cxdb"),
            target: NodeId::from_raw("tech-rmp-serde"),
            edge_type: EdgeType::Uses,
            metadata: None,
        });
        // con-fs-storage --constrains--> dec-use-msgpack
        bp.add_edge(Edge {
            source: NodeId::from_raw("con-fs-storage"),
            target: NodeId::from_raw("dec-use-msgpack"),
            edge_type: EdgeType::Constrains,
            metadata: None,
        });
        // comp-event-store --depends_on--> comp-cxdb
        bp.add_edge(Edge {
            source: NodeId::from_raw("comp-event-store"),
            target: NodeId::from_raw("comp-cxdb"),
            edge_type: EdgeType::DependsOn,
            metadata: None,
        });
        // dec-use-msgpack --satisfies--> qr-crash-safe
        bp.add_edge(Edge {
            source: NodeId::from_raw("dec-use-msgpack"),
            target: NodeId::from_raw("qr-crash-safe"),
            edge_type: EdgeType::Satisfies,
            metadata: None,
        });

        bp
    }

    #[test]
    fn blueprint_node_crud() {
        let mut bp = Blueprint::new();
        assert_eq!(bp.nodes.len(), 0);

        bp.upsert_node(BlueprintNode::Decision(Decision {
            id: NodeId::from_raw("dec-test"),
            title: "Test Decision".into(),
            status: DecisionStatus::Proposed,
            context: "Testing".into(),
            options: vec![],
            consequences: vec![],
            assumptions: vec![],
            supersedes: None,
            tags: vec![],
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        assert_eq!(bp.nodes.len(), 1);
        assert!(bp.get_node("dec-test").is_some());
        assert!(bp.get_node("nonexistent").is_none());

        // Remove
        let removed = bp.remove_node("dec-test");
        assert!(removed.is_some());
        assert_eq!(bp.nodes.len(), 0);
    }

    #[test]
    fn blueprint_edge_crud() {
        let mut bp = make_test_blueprint();
        let initial_edges = bp.edges.len();
        assert_eq!(initial_edges, 7);

        // Remove edges from a specific source
        let removed = bp.remove_edges_where(|e| e.source.0 == "con-fs-storage");
        assert_eq!(removed, 1);
        assert_eq!(bp.edges.len(), 6);
    }

    #[test]
    fn forward_and_reverse_neighbors() {
        let bp = make_test_blueprint();

        // dec-use-msgpack has 4 outgoing edges
        let fwd = bp.forward_neighbors("dec-use-msgpack");
        assert_eq!(fwd.len(), 4); // affects comp-cxdb, comp-event-store, tech-rmp-serde + satisfies qr

        // comp-cxdb has reverse edges from dec-use-msgpack and comp-event-store
        let rev = bp.reverse_neighbors("comp-cxdb");
        assert_eq!(rev.len(), 2); // dec-use-msgpack --affects--> and comp-event-store --depends_on-->
    }

    #[test]
    fn downstream_bfs() {
        let bp = make_test_blueprint();

        // From dec-use-msgpack: should reach comp-cxdb, comp-event-store, tech-rmp-serde, qr-crash-safe
        let downstream = bp.downstream_bfs("dec-use-msgpack");
        assert!(downstream.contains(&"comp-cxdb".to_string()));
        assert!(downstream.contains(&"comp-event-store".to_string()));
        assert!(downstream.contains(&"tech-rmp-serde".to_string()));
        assert!(downstream.contains(&"qr-crash-safe".to_string()));
        // Should NOT contain itself or the constraint
        assert!(!downstream.contains(&"dec-use-msgpack".to_string()));
    }

    #[test]
    fn upstream_bfs() {
        let bp = make_test_blueprint();

        // From tech-rmp-serde: should reach comp-cxdb, dec-use-msgpack, con-fs-storage
        let upstream = bp.upstream_bfs("tech-rmp-serde");
        assert!(upstream.contains(&"comp-cxdb".to_string()));
        assert!(upstream.contains(&"dec-use-msgpack".to_string()));
    }

    #[test]
    fn topological_sort_acyclic() {
        let bp = make_test_blueprint();
        let sorted = bp.topological_sort();
        assert!(sorted.is_some());
        let sorted = sorted.unwrap();
        assert_eq!(sorted.len(), bp.nodes.len());

        // con-fs-storage should come before dec-use-msgpack
        let con_pos = sorted.iter().position(|s| s == "con-fs-storage");
        let dec_pos = sorted.iter().position(|s| s == "dec-use-msgpack");
        assert!(con_pos.is_some());
        assert!(dec_pos.is_some());
        assert!(con_pos.unwrap() < dec_pos.unwrap());
    }

    #[test]
    fn topological_sort_with_cycle() {
        let mut bp = Blueprint::new();

        bp.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("a"), name: "A".into(),
            component_type: ComponentType::Module, description: "".into(),
            provides: vec![], consumes: vec![],
            status: ComponentStatus::Shipped, tags: vec![],
            created_at: "".into(), updated_at: "".into(),
        }));
        bp.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("b"), name: "B".into(),
            component_type: ComponentType::Module, description: "".into(),
            provides: vec![], consumes: vec![],
            status: ComponentStatus::Shipped, tags: vec![],
            created_at: "".into(), updated_at: "".into(),
        }));

        // a -> b -> a (cycle)
        bp.add_edge(Edge {
            source: NodeId::from_raw("a"), target: NodeId::from_raw("b"),
            edge_type: EdgeType::DependsOn, metadata: None,
        });
        bp.add_edge(Edge {
            source: NodeId::from_raw("b"), target: NodeId::from_raw("a"),
            edge_type: EdgeType::DependsOn, metadata: None,
        });

        assert!(bp.topological_sort().is_none());
    }

    #[test]
    fn impact_analysis_basic() {
        let bp = make_test_blueprint();

        let report = bp.impact_analysis("dec-use-msgpack", "Switch to SQLite");
        assert!(report.is_some());
        let report = report.unwrap();

        assert_eq!(report.source_node_name, "Use MessagePack for disk serialization");
        assert!(!report.entries.is_empty());

        // comp-cxdb should be marked as Reconverge/Deep (direct affects)
        let cxdb_entry = report.entries.iter().find(|e| e.node_id.0 == "comp-cxdb");
        assert!(cxdb_entry.is_some());
        assert_eq!(cxdb_entry.unwrap().action, ImpactAction::Reconverge);
        assert_eq!(cxdb_entry.unwrap().severity, ImpactSeverity::Deep);
    }

    #[test]
    fn impact_analysis_nonexistent_node() {
        let bp = make_test_blueprint();
        assert!(bp.impact_analysis("nonexistent", "anything").is_none());
    }

    #[test]
    fn list_summaries() {
        let bp = make_test_blueprint();
        let summaries = bp.list_summaries();
        assert_eq!(summaries.len(), 6);

        let summaries_by_type = bp.list_summaries_by_type("component");
        assert_eq!(summaries_by_type.len(), 2);
    }

    #[test]
    fn counts_by_type() {
        let bp = make_test_blueprint();
        let counts = bp.counts_by_type();
        assert_eq!(counts.get("decision"), Some(&1));
        assert_eq!(counts.get("technology"), Some(&1));
        assert_eq!(counts.get("component"), Some(&2));
        assert_eq!(counts.get("constraint"), Some(&1));
        assert_eq!(counts.get("quality_requirement"), Some(&1));
    }

    // -----------------------------------------------------------------------
    // BlueprintStore persistence tests
    // -----------------------------------------------------------------------

    fn temp_data_dir() -> PathBuf {
        std::env::temp_dir().join(format!("planner_blueprint_test_{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn store_in_memory() {
        let store = BlueprintStore::new();
        assert!(!store.is_persistent());
        assert!(!store.is_dirty());

        store.upsert_node(BlueprintNode::Decision(Decision {
            id: NodeId::from_raw("dec-test"),
            title: "Test".into(),
            status: DecisionStatus::Proposed,
            context: "".into(),
            options: vec![], consequences: vec![], assumptions: vec![],
            supersedes: None, tags: vec![],
            created_at: "".into(), updated_at: "".into(),
        }));

        // In-memory store doesn't track dirty.
        assert!(!store.is_dirty());
        let (nodes, edges) = store.counts();
        assert_eq!(nodes, 1);
        assert_eq!(edges, 0);
    }

    #[test]
    fn store_persist_and_reload() {
        let data_dir = temp_data_dir();

        // First lifetime — create and flush.
        {
            let store = BlueprintStore::open(&data_dir).unwrap();
            assert!(store.is_persistent());

            store.upsert_node(BlueprintNode::Decision(Decision {
                id: NodeId::from_raw("dec-test"),
                title: "Test Decision".into(),
                status: DecisionStatus::Accepted,
                context: "Testing persistence".into(),
                options: vec![], consequences: vec![], assumptions: vec![],
                supersedes: None, tags: vec!["test".into()],
                created_at: "2026-03-01T00:00:00Z".into(),
                updated_at: "2026-03-01T00:00:00Z".into(),
            }));

            store.upsert_node(BlueprintNode::Technology(Technology {
                id: NodeId::from_raw("tech-test"),
                name: "TestLib".into(),
                version: Some("1.0".into()),
                category: TechnologyCategory::Library,
                ring: AdoptionRing::Adopt,
                rationale: "Testing".into(),
                license: None, tags: vec![],
                created_at: "2026-03-01T00:00:00Z".into(),
                updated_at: "2026-03-01T00:00:00Z".into(),
            }));

            store.add_edge(Edge {
                source: NodeId::from_raw("dec-test"),
                target: NodeId::from_raw("tech-test"),
                edge_type: EdgeType::Affects,
                metadata: Some("selection".into()),
            });

            assert!(store.is_dirty());
            let flushed = store.flush().unwrap();
            assert!(flushed);
            assert!(!store.is_dirty());
        }

        // Second lifetime — reload and verify.
        {
            let store = BlueprintStore::open(&data_dir).unwrap();
            let (nodes, edges) = store.counts();
            assert_eq!(nodes, 2);
            assert_eq!(edges, 1);

            let dec = store.get_node("dec-test").unwrap();
            assert_eq!(dec.name(), "Test Decision");
            assert_eq!(dec.type_name(), "decision");

            let tech = store.get_node("tech-test").unwrap();
            assert_eq!(tech.name(), "TestLib");

            // Graph traversal works after reload.
            let downstream = store.snapshot().downstream_bfs("dec-test");
            assert!(downstream.contains(&"tech-test".to_string()));

            // Impact analysis works after reload.
            let report = store.impact_analysis("dec-test", "change");
            assert!(report.is_some());
        }

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn store_remove_node_cleans_edges() {
        let store = BlueprintStore::new();

        store.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("a"), name: "A".into(),
            component_type: ComponentType::Module, description: "".into(),
            provides: vec![], consumes: vec![],
            status: ComponentStatus::Shipped, tags: vec![],
            created_at: "".into(), updated_at: "".into(),
        }));
        store.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("b"), name: "B".into(),
            component_type: ComponentType::Module, description: "".into(),
            provides: vec![], consumes: vec![],
            status: ComponentStatus::Shipped, tags: vec![],
            created_at: "".into(), updated_at: "".into(),
        }));

        store.add_edge(Edge {
            source: NodeId::from_raw("a"), target: NodeId::from_raw("b"),
            edge_type: EdgeType::DependsOn, metadata: None,
        });

        assert_eq!(store.counts(), (2, 1));

        store.remove_node("a");
        let (nodes, edges) = store.counts();
        assert_eq!(nodes, 1);
        assert_eq!(edges, 0); // Edge cleaned up
    }

    #[test]
    fn store_update_node() {
        let store = BlueprintStore::new();

        store.upsert_node(BlueprintNode::Decision(Decision {
            id: NodeId::from_raw("dec-test"),
            title: "Original Title".into(),
            status: DecisionStatus::Proposed,
            context: "".into(),
            options: vec![], consequences: vec![], assumptions: vec![],
            supersedes: None, tags: vec![],
            created_at: "".into(), updated_at: "".into(),
        }));

        let updated = store.update_node("dec-test", |node| {
            if let BlueprintNode::Decision(d) = node {
                d.title = "Updated Title".into();
                d.status = DecisionStatus::Accepted;
            }
        });

        assert!(updated.is_some());
        let node = store.get_node("dec-test").unwrap();
        assert_eq!(node.name(), "Updated Title");
    }

    #[test]
    fn store_node_removal_deletes_disk_files() {
        let data_dir = temp_data_dir();

        {
            let store = BlueprintStore::open(&data_dir).unwrap();

            store.upsert_node(BlueprintNode::Decision(Decision {
                id: NodeId::from_raw("dec-to-remove"),
                title: "Will be removed".into(),
                status: DecisionStatus::Proposed,
                context: "".into(),
                options: vec![], consequences: vec![], assumptions: vec![],
                supersedes: None, tags: vec![],
                created_at: "".into(), updated_at: "".into(),
            }));

            store.upsert_node(BlueprintNode::Decision(Decision {
                id: NodeId::from_raw("dec-stays"),
                title: "Stays".into(),
                status: DecisionStatus::Accepted,
                context: "".into(),
                options: vec![], consequences: vec![], assumptions: vec![],
                supersedes: None, tags: vec![],
                created_at: "".into(), updated_at: "".into(),
            }));

            store.flush().unwrap();

            // Verify both files exist.
            assert!(data_dir.join("blueprint/nodes/dec-to-remove.msgpack").exists());
            assert!(data_dir.join("blueprint/nodes/dec-stays.msgpack").exists());

            // Remove one node and flush again.
            store.remove_node("dec-to-remove");
            store.flush().unwrap();

            // File should be cleaned up.
            assert!(!data_dir.join("blueprint/nodes/dec-to-remove.msgpack").exists());
            assert!(data_dir.join("blueprint/nodes/dec-stays.msgpack").exists());
        }

        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
