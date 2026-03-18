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
//!   ├── events.msgpack              # Append-only event log
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

use crate::component_naming::{
    derive_spec_group_key, generate_directory_name, generate_factory_name, generate_spec_name,
    is_weak_component_name, DirectoryNamingInput, FactoryNamingInput, SpecGroupNamingInput,
};
use crate::knowledge_naming::{concise_constraint_title, concise_quality_label};

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
            self.edges
                .retain(|e| e.source.0 != node_id && e.target.0 != node_id);
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
        if self.edges.iter().any(|existing| {
            existing.source == edge.source
                && existing.target == edge.target
                && existing.edge_type == edge.edge_type
        }) {
            return;
        }
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
        self.forward_adj
            .get(node_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get reverse neighbors (incoming edges to this node).
    pub fn reverse_neighbors(&self, node_id: &str) -> &[(EdgeType, String)] {
        self.reverse_adj
            .get(node_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
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
    fn classify_impact(
        &self,
        source_id: &str,
        affected_id: &str,
    ) -> (ImpactAction, ImpactSeverity) {
        // Find the direct edge types between source and affected.
        let direct_edges: Vec<EdgeType> = self
            .edges
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
                // Project contains nodes → membership changed, shallow refresh only.
                (EdgeType::Contains, _) => (ImpactAction::Update, ImpactSeverity::Shallow),
                // Decision affects a component → reconverge (deep)
                (EdgeType::Affects, "component") => {
                    (ImpactAction::Reconverge, ImpactSeverity::Deep)
                }
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
        let source_name = self
            .nodes
            .get(source_id)
            .map(|n| n.name().to_string())
            .unwrap_or_else(|| source_id.to_string());
        let affected_name = self
            .nodes
            .get(affected_id)
            .map(|n| n.name().to_string())
            .unwrap_or_else(|| affected_id.to_string());

        // Find direct edge.
        let direct_edge = self
            .edges
            .iter()
            .find(|e| e.source.0 == source_id && e.target.0 == affected_id);

        match direct_edge {
            Some(edge) => {
                format!(
                    "{} {} {} via {} relationship",
                    source_name, edge.edge_type, affected_name, edge.edge_type,
                )
            }
            None => {
                format!(
                    "{} is transitively affected by changes to {}",
                    affected_name, source_name,
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
    events: RwLock<Vec<BlueprintEvent>>,
    dirty: RwLock<bool>,
    blueprint_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectPurgeReport {
    pub local_nodes_deleted: usize,
    pub shared_nodes_unlinked: usize,
    pub event_entries_pruned: usize,
    pub history_snapshots_pruned: usize,
}

fn node_scope_mut(node: &mut BlueprintNode) -> &mut NodeScope {
    match node {
        BlueprintNode::Project(inner) => &mut inner.scope,
        BlueprintNode::Decision(inner) => &mut inner.scope,
        BlueprintNode::Technology(inner) => &mut inner.scope,
        BlueprintNode::Component(inner) => &mut inner.scope,
        BlueprintNode::Constraint(inner) => &mut inner.scope,
        BlueprintNode::Pattern(inner) => &mut inner.scope,
        BlueprintNode::QualityRequirement(inner) => &mut inner.scope,
    }
}

fn project_root_node_id(project_id: &str) -> NodeId {
    let slug = project_id
        .to_ascii_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-")
        .trim_matches('-')
        .to_string();
    NodeId::from_raw(format!("proj-{}", slug))
}

fn backfill_project_root_nodes(blueprint: &mut Blueprint) -> usize {
    let existing_ids: Vec<String> = blueprint.nodes.keys().cloned().collect();
    let mut project_roots: HashMap<String, String> = HashMap::new();
    let mut pending_roots: Vec<(String, Option<String>)> = Vec::new();
    let mut inserted = 0usize;

    for node_id in &existing_ids {
        let Some(node) = blueprint.nodes.get(node_id) else {
            continue;
        };
        let scope = node.scope();
        let Some(project) = scope.project.as_ref() else {
            continue;
        };
        if matches!(node, BlueprintNode::Project(_)) {
            project_roots.insert(project.project_id.clone(), node.id().0.clone());
            continue;
        }
        if matches!(scope.scope_class, ScopeClass::Unscoped) {
            continue;
        }

        if !project_roots.contains_key(&project.project_id) {
            let id = project_root_node_id(&project.project_id);
            project_roots.insert(project.project_id.clone(), id.0.clone());
            pending_roots.push((project.project_id.clone(), project.project_name.clone()));
        }
    }

    for (project_id, project_name) in pending_roots {
        let id = project_root_node_id(&project_id);
        if blueprint.nodes.contains_key(id.as_str()) {
            continue;
        }

        let now = chrono::Utc::now().to_rfc3339();
        blueprint.upsert_node(BlueprintNode::Project(Project {
            id: id.clone(),
            name: project_name.clone().unwrap_or_else(|| project_id.clone()),
            description: format!("Blueprint root for project {}", project_id),
            tags: vec!["project-root".into(), "backfill".into()],
            documentation: None,
            scope: NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: project_id.clone(),
                    project_name: project_name.clone(),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
            created_at: now.clone(),
            updated_at: now,
        }));
        inserted += 1;
    }

    for node_id in existing_ids {
        let Some(node) = blueprint.nodes.get(&node_id) else {
            continue;
        };
        if matches!(node, BlueprintNode::Project(_)) {
            continue;
        }
        let Some(project) = node.scope().project.as_ref() else {
            continue;
        };
        let Some(root_id) = project_roots.get(&project.project_id) else {
            continue;
        };

        blueprint.add_edge(Edge {
            source: NodeId::from_raw(root_id.clone()),
            target: node.id().clone(),
            edge_type: EdgeType::Contains,
            metadata: Some("backfilled from scope".into()),
        });
    }

    inserted
}

fn unique_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn merge_legacy_component_into(target: &mut Component, duplicate: &Component) {
    if target.description.trim().len() < duplicate.description.trim().len() {
        target.description = duplicate.description.clone();
    }
    target.provides = unique_strings(
        target
            .provides
            .iter()
            .cloned()
            .chain(duplicate.provides.iter().cloned()),
    );
    target.consumes = unique_strings(
        target
            .consumes
            .iter()
            .cloned()
            .chain(duplicate.consumes.iter().cloned()),
    );
    target.tags = unique_strings(
        target
            .tags
            .iter()
            .cloned()
            .chain(duplicate.tags.iter().cloned()),
    );
    if target.updated_at < duplicate.updated_at {
        target.updated_at = duplicate.updated_at.clone();
    }
}

fn migrate_legacy_spec_component_naming(blueprint: &mut Blueprint) -> usize {
    let component_ids: Vec<String> = blueprint
        .nodes
        .iter()
        .filter_map(|(id, node)| match node {
            BlueprintNode::Component(_) => Some(id.clone()),
            _ => None,
        })
        .collect();

    let mut migrated = 0usize;
    let mut canonical_by_origin: HashMap<String, String> = HashMap::new();
    let mut redirects: HashMap<String, String> = HashMap::new();
    let mut removals = Vec::new();

    for component_id in component_ids {
        let Some(BlueprintNode::Component(component)) = blueprint.nodes.get(&component_id).cloned()
        else {
            continue;
        };
        let Some(naming) = component.naming.as_ref() else {
            continue;
        };
        if naming.strategy != ComponentNamingStrategy::SpecGroup {
            continue;
        }
        if !component.tags.iter().any(|tag| tag == "spec") {
            continue;
        }

        let mut parts = naming.origin_key.split(':');
        let Some(prefix) = parts.next() else {
            continue;
        };
        let Some(project_segment) = parts.next() else {
            continue;
        };
        let Some(chunk_tag) = parts.next() else {
            continue;
        };
        let Some(group_token) = parts.next() else {
            continue;
        };
        if prefix != "spec" {
            continue;
        }

        let statements = if component.provides.is_empty() {
            vec![component.description.clone()]
        } else {
            component.provides.clone()
        };
        let semantic_group = derive_spec_group_key(group_token, &statements);
        let generated = generate_spec_name(SpecGroupNamingInput {
            project_id: project_segment,
            project_name: component
                .scope
                .project
                .as_ref()
                .and_then(|project| project.project_name.as_deref()),
            chunk_tag,
            group_token: &semantic_group,
            statements: &statements,
            component_type: component.component_type.clone(),
            timestamp: &component.updated_at,
        });

        let should_refresh = group_token != semantic_group
            || naming.origin_key != generated.naming.origin_key
            || is_weak_component_name(&component.name)
            || component.name == naming.generated_name;
        if !should_refresh {
            canonical_by_origin
                .entry(naming.origin_key.to_ascii_lowercase())
                .or_insert_with(|| component_id.clone());
            continue;
        }

        if let Some(canonical_id) = canonical_by_origin
            .get(&generated.naming.origin_key.to_ascii_lowercase())
            .cloned()
        {
            if canonical_id != component_id {
                if let Some(BlueprintNode::Component(target)) =
                    blueprint.nodes.get_mut(&canonical_id)
                {
                    let mut refreshed = target.clone();
                    if is_weak_component_name(&refreshed.name)
                        || refreshed.name
                            == refreshed
                                .naming
                                .as_ref()
                                .map(|n| n.generated_name.as_str())
                                .unwrap_or("")
                    {
                        refreshed.name = generated.name.clone();
                    }
                    refreshed.naming = Some(generated.naming.clone());
                    merge_legacy_component_into(&mut refreshed, &component);
                    blueprint
                        .nodes
                        .insert(canonical_id.clone(), BlueprintNode::Component(refreshed));
                }
                redirects.insert(component_id.clone(), canonical_id);
                removals.push(component_id);
                migrated += 1;
                continue;
            }
        }

        let mut refreshed = component.clone();
        refreshed.name = generated.name.clone();
        refreshed.naming = Some(generated.naming.clone());
        blueprint
            .nodes
            .insert(component_id.clone(), BlueprintNode::Component(refreshed));
        canonical_by_origin.insert(
            generated.naming.origin_key.to_ascii_lowercase(),
            component_id,
        );
        migrated += 1;
    }

    if !redirects.is_empty() {
        for edge in &mut blueprint.edges {
            if let Some(target) = redirects.get(edge.source.as_str()) {
                edge.source = NodeId::from_raw(target.clone());
            }
            if let Some(target) = redirects.get(edge.target.as_str()) {
                edge.target = NodeId::from_raw(target.clone());
            }
        }
    }

    for node_id in removals {
        let _ = blueprint.nodes.remove(&node_id);
    }

    if migrated > 0 {
        let mut seen = HashSet::new();
        blueprint.edges.retain(|edge| {
            edge.source != edge.target
                && seen.insert((
                    edge.source.0.clone(),
                    edge.target.0.clone(),
                    edge.edge_type,
                    edge.metadata.clone(),
                ))
        });
        blueprint.rebuild_indexes();
    }

    migrated
}

fn migrate_legacy_factory_component_naming(blueprint: &mut Blueprint) -> usize {
    let component_ids: Vec<String> = blueprint
        .nodes
        .iter()
        .filter_map(|(id, node)| match node {
            BlueprintNode::Component(_) => Some(id.clone()),
            _ => None,
        })
        .collect();

    let mut migrated = 0usize;

    for component_id in component_ids {
        let Some(BlueprintNode::Component(component)) = blueprint.nodes.get(&component_id).cloned()
        else {
            continue;
        };
        let Some(naming) = component.naming.as_ref() else {
            continue;
        };
        if naming.strategy != ComponentNamingStrategy::FactoryOutput {
            continue;
        }
        if naming.source != ComponentNameSource::Generated {
            continue;
        }

        let Some(output_path) = naming.origin_key.strip_prefix("factory:") else {
            continue;
        };

        let generated = generate_factory_name(FactoryNamingInput {
            output_path,
            project_name: component
                .scope
                .project
                .as_ref()
                .and_then(|project| project.project_name.as_deref()),
            timestamp: &component.updated_at,
        });

        let old_name = component.name.trim().to_string();
        let old_generated = naming.generated_name.trim().to_string();
        if old_name == generated.name && old_generated == generated.name {
            continue;
        }
        if !is_weak_component_name(&old_name) && !is_weak_component_name(&old_generated) {
            continue;
        }

        let Some(BlueprintNode::Component(existing)) = blueprint.nodes.get_mut(&component_id)
        else {
            continue;
        };
        let next = crate::component_naming::merge_generated_component(existing, &generated);
        *existing = next;
        migrated += 1;
    }

    migrated
}

fn migrate_generated_directory_component_naming(blueprint: &mut Blueprint) -> usize {
    let component_ids: Vec<String> = blueprint
        .nodes
        .iter()
        .filter_map(|(id, node)| match node {
            BlueprintNode::Component(_) => Some(id.clone()),
            _ => None,
        })
        .collect();

    let mut migrated = 0usize;

    for component_id in component_ids {
        let Some(BlueprintNode::Component(component)) = blueprint.nodes.get(&component_id).cloned()
        else {
            continue;
        };
        let Some(naming) = component.naming.as_ref() else {
            continue;
        };
        if naming.strategy != ComponentNamingStrategy::DirectoryScan {
            continue;
        }
        if naming.source != ComponentNameSource::Generated {
            continue;
        }
        let Some(relative_path) = naming.origin_key.strip_prefix("path:") else {
            continue;
        };
        let normalized_path = relative_path.to_ascii_lowercase();
        let inferred_component_type = if normalized_path.contains("pipeline") {
            ComponentType::Pipeline
        } else if normalized_path.contains("schemas")
            || normalized_path.contains("schema")
            || normalized_path.contains("core")
            || normalized_path.contains("lib")
        {
            ComponentType::Library
        } else if normalized_path.contains("api")
            || normalized_path.contains("service")
            || normalized_path.ends_with("server")
        {
            ComponentType::Service
        } else if normalized_path.contains("store") || normalized_path.contains("db") {
            ComponentType::Store
        } else if normalized_path.contains("web")
            || normalized_path.contains("ui")
            || normalized_path.contains("cli")
            || normalized_path.contains("tui")
        {
            ComponentType::Interface
        } else {
            ComponentType::Module
        };

        let generated = generate_directory_name(DirectoryNamingInput {
            relative_path,
            project_name: component
                .scope
                .project
                .as_ref()
                .and_then(|project| project.project_name.as_deref()),
            component_type: inferred_component_type.clone(),
            timestamp: &component.updated_at,
        });

        if component.name.trim() == generated.name && naming.generated_name.trim() == generated.name
        {
            continue;
        }

        let Some(BlueprintNode::Component(existing)) = blueprint.nodes.get_mut(&component_id)
        else {
            continue;
        };
        let next = crate::component_naming::merge_generated_component(existing, &generated);
        *existing = Component {
            component_type: inferred_component_type,
            ..next
        };
        migrated += 1;
    }

    migrated
}

fn factory_snapshot_suffix(timestamp: &str) -> String {
    timestamp
        .replace('T', " ")
        .replace("+00:00", "")
        .chars()
        .take(16)
        .collect::<String>()
}

fn desired_factory_pattern_name(
    project_name: Option<&str>,
    updated_at: &str,
    archived: bool,
) -> String {
    let base = match project_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(project) => format!("{} Factory Code Generation", project),
        None => "Factory Code Generation".into(),
    };

    if archived {
        format!("{} Snapshot {}", base, factory_snapshot_suffix(updated_at))
    } else {
        base
    }
}

fn node_tags_mut(node: &mut BlueprintNode) -> &mut Vec<String> {
    match node {
        BlueprintNode::Project(n) => &mut n.tags,
        BlueprintNode::Decision(n) => &mut n.tags,
        BlueprintNode::Technology(n) => &mut n.tags,
        BlueprintNode::Component(n) => &mut n.tags,
        BlueprintNode::Constraint(n) => &mut n.tags,
        BlueprintNode::Pattern(n) => &mut n.tags,
        BlueprintNode::QualityRequirement(n) => &mut n.tags,
    }
}

fn migrate_legacy_scope_tags(blueprint: &mut Blueprint) -> usize {
    const ARCHIVED_TAG: &str = "archived";
    const OVERRIDE_PREFIX: &str = "overrides:";

    let node_ids: Vec<String> = blueprint.nodes.keys().cloned().collect();
    let mut migrated = 0usize;

    for node_id in node_ids {
        let Some(node) = blueprint.nodes.get_mut(&node_id) else {
            continue;
        };

        let tags = node_tags_mut(node);
        let mut seen = HashSet::new();
        let mut migrated_archived = false;
        let mut migrated_override_source: Option<String> = None;
        let mut normalized_tags = Vec::with_capacity(tags.len());

        for raw_tag in tags.iter() {
            let trimmed = raw_tag.trim();
            if trimmed.is_empty() {
                continue;
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower == ARCHIVED_TAG {
                migrated_archived = true;
                continue;
            }
            if lower.starts_with(OVERRIDE_PREFIX) {
                if migrated_override_source.is_none() {
                    let source = trimmed[OVERRIDE_PREFIX.len()..].trim();
                    if !source.is_empty() {
                        migrated_override_source = Some(source.to_string());
                    }
                }
                continue;
            }
            if seen.insert(lower) {
                normalized_tags.push(trimmed.to_string());
            }
        }

        let tags_changed = *tags != normalized_tags;
        if tags_changed {
            *tags = normalized_tags;
        }

        let scope = node_scope_mut(node);
        let mut scope_changed = false;
        if migrated_archived && scope.lifecycle != NodeLifecycle::Archived {
            scope.lifecycle = NodeLifecycle::Archived;
            scope_changed = true;
        }
        if scope.override_scope.is_none() {
            if let Some(source) = migrated_override_source {
                scope.override_scope = Some(OverrideScope {
                    shared_source_id: source,
                    override_reason: Some("migrated from legacy override tag".into()),
                    effective_from: None,
                });
                scope_changed = true;
            }
        }

        if tags_changed || scope_changed {
            migrated += 1;
        }
    }

    migrated
}

fn migrate_generated_factory_display_names(blueprint: &mut Blueprint) -> usize {
    let node_ids: Vec<String> = blueprint.nodes.keys().cloned().collect();
    let mut migrated = 0usize;

    for node_id in node_ids {
        let Some(node) = blueprint.nodes.get(&node_id).cloned() else {
            continue;
        };

        match node {
            BlueprintNode::Component(component) => {
                let Some(naming) = component.naming.as_ref() else {
                    continue;
                };
                if naming.strategy != ComponentNamingStrategy::FactoryOutput
                    || naming.source != ComponentNameSource::Generated
                {
                    continue;
                }
                let Some(output_path) = naming.origin_key.strip_prefix("factory:") else {
                    continue;
                };

                let mut generated = generate_factory_name(FactoryNamingInput {
                    output_path,
                    project_name: component
                        .scope
                        .project
                        .as_ref()
                        .and_then(|project| project.project_name.as_deref()),
                    timestamp: &component.updated_at,
                });

                if component.scope.lifecycle == NodeLifecycle::Archived {
                    generated.name = format!(
                        "{} Snapshot {}",
                        generated.name,
                        factory_snapshot_suffix(&component.updated_at)
                    );
                    generated.naming.generated_name = generated.name.clone();
                }

                if component.name.trim() == generated.name
                    && naming.generated_name.trim() == generated.name
                {
                    continue;
                }

                let Some(BlueprintNode::Component(existing)) = blueprint.nodes.get_mut(&node_id)
                else {
                    continue;
                };
                let next = crate::component_naming::merge_generated_component(existing, &generated);
                *existing = next;
                migrated += 1;
            }
            BlueprintNode::Pattern(pattern) => {
                if !pattern.tags.iter().any(|tag| tag == "factory") {
                    continue;
                }
                let desired_name = desired_factory_pattern_name(
                    pattern
                        .scope
                        .project
                        .as_ref()
                        .and_then(|project| project.project_name.as_deref()),
                    &pattern.updated_at,
                    pattern.scope.lifecycle == NodeLifecycle::Archived,
                );

                if pattern.name.trim() == desired_name {
                    continue;
                }

                let Some(BlueprintNode::Pattern(existing)) = blueprint.nodes.get_mut(&node_id)
                else {
                    continue;
                };
                existing.name = desired_name;
                existing.updated_at = existing.updated_at.clone();
                migrated += 1;
            }
            _ => {}
        }
    }

    migrated
}

fn migrate_constraint_titles(blueprint: &mut Blueprint) -> usize {
    let node_ids: Vec<String> = blueprint
        .nodes
        .iter()
        .filter_map(|(id, node)| match node {
            BlueprintNode::Constraint(_) => Some(id.clone()),
            _ => None,
        })
        .collect();

    let mut migrated = 0usize;

    for node_id in node_ids {
        let Some(BlueprintNode::Constraint(constraint)) = blueprint.nodes.get(&node_id).cloned()
        else {
            continue;
        };

        let desired = concise_constraint_title(&constraint.description);
        let title = constraint.title.trim();
        let should_migrate = title != desired
            && (title.len() > 48 || title.contains('…') || title == constraint.description.trim());
        if !should_migrate {
            continue;
        }

        let Some(BlueprintNode::Constraint(existing)) = blueprint.nodes.get_mut(&node_id) else {
            continue;
        };
        existing.title = desired;
        migrated += 1;
    }

    migrated
}

fn migrate_quality_requirement_labels(blueprint: &mut Blueprint) -> usize {
    let node_ids: Vec<String> = blueprint
        .nodes
        .iter()
        .filter_map(|(id, node)| match node {
            BlueprintNode::QualityRequirement(_) => Some(id.clone()),
            _ => None,
        })
        .collect();

    let mut migrated = 0usize;

    for node_id in node_ids {
        let Some(BlueprintNode::QualityRequirement(qr)) = blueprint.nodes.get(&node_id).cloned()
        else {
            continue;
        };

        let desired = concise_quality_label(&qr.scenario, &qr.attribute, &qr.tags);
        if qr.label.as_deref() == Some(desired.as_str()) {
            continue;
        }

        let Some(BlueprintNode::QualityRequirement(existing)) = blueprint.nodes.get_mut(&node_id)
        else {
            continue;
        };
        existing.label = Some(desired);
        migrated += 1;
    }

    migrated
}

fn backfill_legacy_factory_project_scope(blueprint: &mut Blueprint) -> usize {
    let project_roots: Vec<(String, Option<String>)> = blueprint
        .nodes
        .values()
        .filter_map(|node| match node {
            BlueprintNode::Project(project) => {
                Some((project.id.0.clone(), Some(project.name.clone())))
            }
            _ => None,
        })
        .collect();

    if project_roots.len() != 1 {
        return 0;
    }

    let (project_root_id, project_name) = project_roots[0].clone();
    let project_id = project_root_id
        .strip_prefix("proj-")
        .unwrap_or(project_root_id.as_str())
        .to_string();

    let project_scope = NodeScope {
        scope_class: ScopeClass::Project,
        project: Some(ProjectScope {
            project_id: project_id.clone(),
            project_name: project_name.clone(),
        }),
        secondary: SecondaryScopeRefs::default(),
        is_shared: false,
        shared: None,
        lifecycle: NodeLifecycle::Active,
        override_scope: None,
            scope_review: None,
    };

    let node_ids: Vec<String> = blueprint.nodes.keys().cloned().collect();
    let mut migrated = 0usize;

    for node_id in node_ids {
        let Some(node) = blueprint.nodes.get(&node_id).cloned() else {
            continue;
        };

        let should_migrate = match &node {
            BlueprintNode::Component(component) => {
                component.scope.project.is_none()
                    && matches!(component.scope.scope_class, ScopeClass::Unscoped)
                    && component.naming.as_ref().is_some_and(|naming| {
                        naming.strategy == ComponentNamingStrategy::FactoryOutput
                            && naming.source == ComponentNameSource::Generated
                    })
            }
            BlueprintNode::Pattern(pattern) => {
                pattern.scope.project.is_none()
                    && matches!(pattern.scope.scope_class, ScopeClass::Unscoped)
                    && pattern.tags.iter().any(|tag| tag == "factory")
            }
            _ => false,
        };

        if !should_migrate {
            continue;
        }

        let Some(existing) = blueprint.nodes.get_mut(&node_id) else {
            continue;
        };

        match existing {
            BlueprintNode::Component(component) => {
                component.scope = project_scope.clone();
            }
            BlueprintNode::Pattern(pattern) => {
                pattern.scope = project_scope.clone();
            }
            _ => continue,
        }

        blueprint.add_edge(Edge {
            source: NodeId::from_raw(project_root_id.clone()),
            target: NodeId::from_raw(node_id.clone()),
            edge_type: EdgeType::Contains,
            metadata: Some("backfilled legacy factory scope".into()),
        });
        migrated += 1;
    }

    migrated
}

fn canonicalize_project_scope_names(blueprint: &mut Blueprint) -> usize {
    let project_names: HashMap<String, String> = blueprint
        .nodes
        .values()
        .filter_map(|node| match node {
            BlueprintNode::Project(project) => project
                .scope
                .project
                .as_ref()
                .map(|scope| (scope.project_id.clone(), project.name.clone())),
            _ => None,
        })
        .collect();

    let node_ids: Vec<String> = blueprint.nodes.keys().cloned().collect();
    let mut migrated = 0usize;

    for node_id in node_ids {
        let Some(node) = blueprint.nodes.get_mut(&node_id) else {
            continue;
        };
        let scope = node_scope_mut(node);
        let Some(project) = scope.project.as_mut() else {
            continue;
        };
        let Some(canonical_name) = project_names.get(&project.project_id) else {
            continue;
        };
        if project.project_name.as_deref() == Some(canonical_name.as_str()) {
            continue;
        }
        project.project_name = Some(canonical_name.clone());
        migrated += 1;
    }

    migrated
}

fn backfill_project_scope_from_contains_edges(blueprint: &mut Blueprint) -> usize {
    let node_ids: Vec<String> = blueprint.nodes.keys().cloned().collect();
    let mut migrated = 0usize;

    for node_id in node_ids {
        let Some(node) = blueprint.nodes.get(&node_id) else {
            continue;
        };
        if matches!(node, BlueprintNode::Project(_)) {
            continue;
        }
        let scope = node.scope();
        if !matches!(scope.scope_class, ScopeClass::Unscoped) || scope.project.is_some() {
            continue;
        }

        let mut project_sources: Vec<String> = blueprint
            .edges
            .iter()
            .filter(|edge| {
                edge.target == NodeId::from_raw(node_id.clone())
                    && edge.edge_type == EdgeType::Contains
            })
            .filter_map(|edge| match blueprint.nodes.get(edge.source.as_str()) {
                Some(BlueprintNode::Project(_)) => Some(edge.source.0.clone()),
                _ => None,
            })
            .collect();
        project_sources.sort();
        project_sources.dedup();

        if project_sources.len() != 1 {
            continue;
        }

        let Some((project_id, project_name)) =
            blueprint
                .nodes
                .get(&project_sources[0])
                .and_then(|node| match node {
                    BlueprintNode::Project(project_root) => Some((
                        project_root
                            .scope
                            .project
                            .as_ref()
                            .map(|scope| scope.project_id.clone())
                            .unwrap_or_else(|| {
                                project_sources[0].trim_start_matches("proj-").to_string()
                            }),
                        project_root.name.clone(),
                    )),
                    _ => None,
                })
        else {
            continue;
        };

        let Some(existing) = blueprint.nodes.get_mut(&node_id) else {
            continue;
        };
        let scope = node_scope_mut(existing);
        scope.scope_class = ScopeClass::Project;
        scope.project = Some(ProjectScope {
            project_id,
            project_name: Some(project_name),
        });
        scope.is_shared = false;
        scope.shared = None;
        migrated += 1;
    }

    migrated
}

fn backfill_single_project_review_scope(blueprint: &mut Blueprint) -> usize {
    let project_roots: Vec<(String, String, String)> = blueprint
        .nodes
        .values()
        .filter_map(|node| match node {
            BlueprintNode::Project(project) => project.scope.project.as_ref().map(|scope| {
                (
                    project.id.0.clone(),
                    scope.project_id.clone(),
                    project.name.clone(),
                )
            }),
            _ => None,
        })
        .collect();

    if project_roots.len() != 1 {
        return 0;
    }

    let (project_root_id, project_id, project_name) = project_roots[0].clone();
    let node_ids: Vec<String> = blueprint.nodes.keys().cloned().collect();
    let mut migrated = 0usize;

    for node_id in node_ids {
        let Some(node) = blueprint.nodes.get(&node_id) else {
            continue;
        };
        let should_migrate = match node {
            BlueprintNode::Constraint(constraint) => {
                constraint.scope.project.is_none()
                    && matches!(constraint.scope.scope_class, ScopeClass::Unscoped)
                    && constraint.tags.iter().any(|tag| tag == "ar-review")
            }
            _ => false,
        };

        if !should_migrate {
            continue;
        }

        let Some(existing) = blueprint.nodes.get_mut(&node_id) else {
            continue;
        };
        let scope = node_scope_mut(existing);
        scope.scope_class = ScopeClass::Project;
        scope.project = Some(ProjectScope {
            project_id: project_id.clone(),
            project_name: Some(project_name.clone()),
        });
        scope.is_shared = false;
        scope.shared = None;

        blueprint.add_edge(Edge {
            source: NodeId::from_raw(project_root_id.clone()),
            target: NodeId::from_raw(node_id.clone()),
            edge_type: EdgeType::Contains,
            metadata: Some("backfilled legacy review scope".into()),
        });
        migrated += 1;
    }

    migrated
}

fn archive_stale_factory_history(blueprint: &mut Blueprint) -> usize {
    let mut component_groups: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut pattern_groups: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for (node_id, node) in &blueprint.nodes {
        match node {
            BlueprintNode::Component(component) => {
                if component
                    .naming
                    .as_ref()
                    .is_some_and(|naming| naming.strategy == ComponentNamingStrategy::FactoryOutput)
                {
                    if let Some(project) = component.scope.project.as_ref() {
                        component_groups
                            .entry(project.project_id.clone())
                            .or_default()
                            .push((node_id.clone(), component.updated_at.clone()));
                    }
                }
            }
            BlueprintNode::Pattern(pattern) => {
                if pattern.tags.iter().any(|tag| tag == "factory") {
                    if let Some(project) = pattern.scope.project.as_ref() {
                        pattern_groups
                            .entry(project.project_id.clone())
                            .or_default()
                            .push((node_id.clone(), pattern.updated_at.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    let mut migrated = 0usize;

    for group in component_groups.values_mut() {
        group.sort_by(|left, right| right.1.cmp(&left.1));
        for (index, (node_id, _)) in group.iter().enumerate() {
            let Some(node) = blueprint.nodes.get_mut(node_id) else {
                continue;
            };
            let scope = node_scope_mut(node);
            let next_lifecycle = if index == 0 {
                NodeLifecycle::Active
            } else {
                NodeLifecycle::Archived
            };
            if scope.lifecycle != next_lifecycle {
                scope.lifecycle = next_lifecycle;
                migrated += 1;
            }
        }
    }

    for group in pattern_groups.values_mut() {
        group.sort_by(|left, right| right.1.cmp(&left.1));
        for (index, (node_id, _)) in group.iter().enumerate() {
            let Some(node) = blueprint.nodes.get_mut(node_id) else {
                continue;
            };
            let scope = node_scope_mut(node);
            let next_lifecycle = if index == 0 {
                NodeLifecycle::Active
            } else {
                NodeLifecycle::Archived
            };
            if scope.lifecycle != next_lifecycle {
                scope.lifecycle = next_lifecycle;
                migrated += 1;
            }
        }
    }

    migrated
}

impl BlueprintStore {
    /// Create a purely in-memory store with no disk backing (for tests).
    pub fn new() -> Self {
        BlueprintStore {
            blueprint: RwLock::new(Blueprint::new()),
            events: RwLock::new(Vec::new()),
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

        let migrated_legacy_scope_tags = migrate_legacy_scope_tags(&mut blueprint);
        let migrated_project_roots = backfill_project_root_nodes(&mut blueprint);
        let migrated_contained_scope = backfill_project_scope_from_contains_edges(&mut blueprint);
        let migrated_factory_scope = backfill_legacy_factory_project_scope(&mut blueprint);
        let migrated_review_scope = backfill_single_project_review_scope(&mut blueprint);
        let migrated_scope_names = canonicalize_project_scope_names(&mut blueprint);
        let migrated_legacy_components = migrate_legacy_spec_component_naming(&mut blueprint);
        let migrated_directory_components =
            migrate_generated_directory_component_naming(&mut blueprint);
        let migrated_factory_components = migrate_legacy_factory_component_naming(&mut blueprint);
        let archived_factory_history = archive_stale_factory_history(&mut blueprint);
        let migrated_factory_display_names =
            migrate_generated_factory_display_names(&mut blueprint);
        let migrated_constraint_titles = migrate_constraint_titles(&mut blueprint);
        let migrated_quality_labels = migrate_quality_requirement_labels(&mut blueprint);
        blueprint.rebuild_indexes();

        let count = blueprint.nodes.len();
        let edge_count = blueprint.edges.len();
        if load_errors > 0 {
            tracing::warn!(
                "Blueprint store: loaded {} nodes, {} edges, {} errors",
                count,
                edge_count,
                load_errors,
            );
        } else if count > 0 {
            tracing::info!(
                "Blueprint store: loaded {} nodes, {} edges from disk",
                count,
                edge_count,
            );
        }

        let store = BlueprintStore {
            blueprint: RwLock::new(blueprint),
            events: RwLock::new(Self::load_events(&blueprint_dir)),
            dirty: RwLock::new(false),
            blueprint_dir: Some(blueprint_dir),
        };

        if migrated_project_roots > 0
            || migrated_legacy_scope_tags > 0
            || migrated_contained_scope > 0
            || migrated_factory_scope > 0
            || migrated_review_scope > 0
            || migrated_scope_names > 0
            || migrated_legacy_components > 0
            || migrated_directory_components > 0
            || migrated_factory_components > 0
            || archived_factory_history > 0
            || migrated_factory_display_names > 0
            || migrated_constraint_titles > 0
            || migrated_quality_labels > 0
        {
            *store.dirty.write() = true;
            store.flush()?;
            if migrated_legacy_scope_tags > 0 {
                tracing::info!(
                    "Blueprint store: normalized legacy archived/override tags on {} node(s)",
                    migrated_legacy_scope_tags,
                );
            }
            if migrated_project_roots > 0 {
                tracing::info!(
                    "Blueprint store: backfilled {} project root node(s) from scoped records",
                    migrated_project_roots,
                );
            }
            if migrated_contained_scope > 0 {
                tracing::info!(
                    "Blueprint store: backfilled project scope from contains edges for {} node(s)",
                    migrated_contained_scope,
                );
            }
            if migrated_legacy_components > 0 {
                tracing::info!(
                    "Blueprint store: refreshed {} legacy spec component record(s) to semantic naming",
                    migrated_legacy_components,
                );
            }
            if migrated_directory_components > 0 {
                tracing::info!(
                    "Blueprint store: refreshed {} generated directory component record(s) to improved naming",
                    migrated_directory_components,
                );
            }
            if migrated_factory_scope > 0 {
                tracing::info!(
                    "Blueprint store: backfilled project scope for {} legacy factory node(s)",
                    migrated_factory_scope,
                );
            }
            if migrated_review_scope > 0 {
                tracing::info!(
                    "Blueprint store: backfilled project scope for {} adversarial review node(s)",
                    migrated_review_scope,
                );
            }
            if migrated_scope_names > 0 {
                tracing::info!(
                    "Blueprint store: normalized project scope names for {} node(s)",
                    migrated_scope_names,
                );
            }
            if migrated_factory_components > 0 {
                tracing::info!(
                    "Blueprint store: refreshed {} legacy factory component record(s) to readable naming",
                    migrated_factory_components,
                );
            }
            if archived_factory_history > 0 {
                tracing::info!(
                    "Blueprint store: archived {} stale factory history node(s)",
                    archived_factory_history,
                );
            }
            if migrated_factory_display_names > 0 {
                tracing::info!(
                    "Blueprint store: refreshed {} factory history display name(s) for uniqueness",
                    migrated_factory_display_names,
                );
            }
            if migrated_constraint_titles > 0 {
                tracing::info!(
                    "Blueprint store: refreshed {} constraint title(s) to concise labels",
                    migrated_constraint_titles,
                );
            }
            if migrated_quality_labels > 0 {
                tracing::info!(
                    "Blueprint store: refreshed {} quality requirement label(s)",
                    migrated_quality_labels,
                );
            }
        }

        Ok(store)
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

    /// Find a component node by stable origin key.
    pub fn find_component_by_origin_key(&self, origin_key: &str) -> Option<Component> {
        let trimmed = origin_key.trim();
        if trimmed.is_empty() {
            return None;
        }

        self.blueprint
            .read()
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::Component(component)
                    if component
                        .naming
                        .as_ref()
                        .is_some_and(|naming| naming.origin_key.eq_ignore_ascii_case(trimmed)) =>
                {
                    Some(component.clone())
                }
                _ => None,
            })
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
        self.blueprint
            .read()
            .impact_analysis(node_id, change_description)
    }

    /// Return a clone of the full event log.
    pub fn events(&self) -> Vec<BlueprintEvent> {
        self.events.read().clone()
    }

    /// Return events filtered to a specific node.
    pub fn events_for_node(&self, node_id: &str) -> Vec<BlueprintEvent> {
        self.events
            .read()
            .iter()
            .filter(|e| match e {
                BlueprintEvent::NodeCreated { node, .. } => node.id().0 == node_id,
                BlueprintEvent::NodeUpdated { node_id: nid, .. } => nid == node_id,
                BlueprintEvent::NodeDeleted { node_id: nid, .. } => nid == node_id,
                BlueprintEvent::EdgeCreated { edge, .. } => {
                    edge.source.0 == node_id || edge.target.0 == node_id
                }
                BlueprintEvent::EdgesDeleted { edges, .. } => edges
                    .iter()
                    .any(|e| e.source.0 == node_id || e.target.0 == node_id),
                BlueprintEvent::ExportRecorded {
                    node_id: export_node_id,
                    ..
                } => export_node_id.as_deref() == Some(node_id),
            })
            .cloned()
            .collect()
    }

    /// Total number of events in the log.
    pub fn event_count(&self) -> usize {
        self.events.read().len()
    }

    /// Find node IDs relevant to a set of search terms.
    ///
    /// Matching is lexical and biased toward node name, tags, and ID, with
    /// lower-weight matches against freeform descriptions/documentation.
    pub fn find_relevant_node_ids(&self, terms: &[String], limit: usize) -> Vec<String> {
        if limit == 0 {
            return Vec::new();
        }

        let cleaned_terms: Vec<String> = terms
            .iter()
            .map(|term| term.trim().to_lowercase())
            .filter(|term| term.len() >= 3)
            .collect();
        if cleaned_terms.is_empty() {
            return Vec::new();
        }

        let blueprint = self.blueprint.read();
        let mut scored: Vec<(String, usize, String)> = blueprint
            .nodes
            .values()
            .filter_map(|node| {
                let name = node.name().to_lowercase();
                let id = node.id().as_str().to_lowercase();
                let tags: Vec<String> = node.tags().iter().map(|tag| tag.to_lowercase()).collect();
                let corpus = node_search_corpus(node);

                let mut score = 0usize;
                for term in &cleaned_terms {
                    if name == *term || id == *term {
                        score += 12;
                        continue;
                    }
                    if name.contains(term) {
                        score += 6;
                    }
                    if id.contains(term) {
                        score += 5;
                    }
                    if tags.iter().any(|tag| tag == term) {
                        score += 4;
                    } else if tags.iter().any(|tag| tag.contains(term)) {
                        score += 3;
                    }
                    if corpus.contains(term) {
                        score += 2;
                    }
                }

                (score > 0).then(|| (node.id().as_str().to_string(), score, name))
            })
            .collect();

        scored.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.0.cmp(&right.0))
        });

        scored
            .into_iter()
            .take(limit)
            .map(|(id, _, _)| id)
            .collect()
    }

    /// Render a bounded Markdown context block for a neighborhood of nodes.
    ///
    /// `root_ids` define the directly relevant nodes and `depth` controls how
    /// many graph hops of surrounding context to include.
    pub fn render_context_markdown(&self, root_ids: &[String], depth: usize) -> Option<String> {
        let blueprint = self.blueprint.read();

        let mut ordered_roots = Vec::new();
        for node_id in root_ids {
            if blueprint.nodes.contains_key(node_id)
                && !ordered_roots
                    .iter()
                    .any(|existing: &String| existing == node_id)
            {
                ordered_roots.push(node_id.clone());
            }
        }
        if ordered_roots.is_empty() {
            return None;
        }

        let mut ordered_ids = collect_context_node_ids(&blueprint, &ordered_roots, depth);
        const MAX_CONTEXT_NODES: usize = 12;
        if ordered_ids.len() > MAX_CONTEXT_NODES {
            ordered_ids.truncate(MAX_CONTEXT_NODES);
        }

        let included: HashSet<&str> = ordered_ids.iter().map(String::as_str).collect();
        let root_set: HashSet<&str> = ordered_roots.iter().map(String::as_str).collect();

        let mut markdown = String::from(
            "# Existing Blueprint Context\n\n\
             The following Blueprint nodes may already constrain or inform this work.\n",
        );

        for node_id in &ordered_ids {
            let Some(node) = blueprint.nodes.get(node_id) else {
                continue;
            };

            markdown.push_str(&format!(
                "\n## {} [{}] `{}`\n",
                node.name(),
                node.type_name(),
                node_id,
            ));
            markdown.push_str(&format!("- status: {}\n", node.status()));
            if root_set.contains(node_id.as_str()) {
                markdown.push_str("- relevance: direct lexical match for the current work\n");
            }

            let summary = node_context_summary(node);
            if !summary.is_empty() {
                markdown.push_str(&format!(
                    "- summary: {}\n",
                    truncate_context_text(&summary, 180),
                ));
            }
            if !node.tags().is_empty() {
                markdown.push_str(&format!("- tags: {}\n", node.tags().join(", ")));
            }
            if let Some(doc) = node.documentation() {
                markdown.push_str(&format!("- docs: {}\n", truncate_context_text(doc, 180),));
            }

            let relations = context_relation_lines(&blueprint, node_id, &included);
            for relation in relations {
                markdown.push_str(&format!("- relation: {}\n", relation));
            }
        }

        Some(markdown)
    }

    // -----------------------------------------------------------------------
    // Write operations (mark dirty + emit events)
    // -----------------------------------------------------------------------

    fn now_iso() -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    fn append_event(&self, event: BlueprintEvent) {
        tracing::debug!("Blueprint event: {}", event.summary());
        self.events.write().push(event);
    }

    /// Insert or replace a node.
    pub fn upsert_node(&self, node: BlueprintNode) {
        let ts = Self::now_iso();
        let node_id = node.id().0.clone();
        let before = self.blueprint.read().get_node(&node_id).cloned();
        self.blueprint.write().upsert_node(node.clone());
        self.mark_dirty();

        if let Some(before) = before {
            self.append_event(BlueprintEvent::NodeUpdated {
                node_id,
                before,
                after: node,
                timestamp: ts,
            });
        } else {
            self.append_event(BlueprintEvent::NodeCreated {
                node,
                timestamp: ts,
            });
        }
    }

    /// Remove a node and all incident edges.
    pub fn remove_node(&self, node_id: &str) -> Option<BlueprintNode> {
        let ts = Self::now_iso();
        // Capture incident edges before removal.
        let removed_edges: Vec<Edge> = self
            .blueprint
            .read()
            .edges
            .iter()
            .filter(|e| e.source.0 == node_id || e.target.0 == node_id)
            .cloned()
            .collect();

        let removed = self.blueprint.write().remove_node(node_id);
        if let Some(ref node) = removed {
            self.mark_dirty();
            self.append_event(BlueprintEvent::NodeDeleted {
                node_id: node_id.to_string(),
                node: node.clone(),
                removed_edges,
                timestamp: ts,
            });
        }
        removed
    }

    /// Apply a mutation to a node. Returns the updated node.
    pub fn update_node<F>(&self, node_id: &str, f: F) -> Option<BlueprintNode>
    where
        F: FnOnce(&mut BlueprintNode),
    {
        let ts = Self::now_iso();
        let mut bp = self.blueprint.write();
        if let Some(node) = bp.nodes.get_mut(node_id) {
            let before = node.clone();
            f(node);
            let after = node.clone();
            drop(bp);
            self.mark_dirty();
            self.append_event(BlueprintEvent::NodeUpdated {
                node_id: node_id.to_string(),
                before,
                after: after.clone(),
                timestamp: ts,
            });
            Some(after)
        } else {
            None
        }
    }

    /// Add an edge.
    pub fn add_edge(&self, edge: Edge) {
        let ts = Self::now_iso();
        self.blueprint.write().add_edge(edge.clone());
        self.mark_dirty();
        self.append_event(BlueprintEvent::EdgeCreated {
            edge,
            timestamp: ts,
        });
    }

    /// Remove edges matching a predicate.
    pub fn remove_edges_where<F: Fn(&Edge) -> bool>(&self, predicate: F) -> usize {
        let ts = Self::now_iso();
        // Capture edges that will be removed.
        let edges_to_remove: Vec<Edge> = self
            .blueprint
            .read()
            .edges
            .iter()
            .filter(|e| predicate(e))
            .cloned()
            .collect();

        let removed = self.blueprint.write().remove_edges_where(predicate);
        if removed > 0 {
            self.mark_dirty();
            self.append_event(BlueprintEvent::EdgesDeleted {
                edges: edges_to_remove,
                timestamp: ts,
            });
        }
        removed
    }

    /// Record a durable export event for project activity history.
    pub fn record_export_event(
        &self,
        export_id: String,
        kind: BlueprintExportKind,
        actor: Option<String>,
        node_id: Option<String>,
        node_count: usize,
        edge_count: usize,
        project_id: Option<String>,
        project_name: Option<String>,
        scope_snapshot: Option<serde_json::Value>,
    ) {
        let ts = Self::now_iso();
        self.mark_dirty();
        self.append_event(BlueprintEvent::ExportRecorded {
            export_id,
            kind,
            actor,
            node_id,
            node_count,
            edge_count,
            project_id,
            project_name,
            scope_snapshot,
            timestamp: ts,
        });
    }

    pub fn purge_project(&self, project_id: &str) -> ProjectPurgeReport {
        let canonical_project_id = project_id.trim();
        if canonical_project_id.is_empty() {
            return ProjectPurgeReport::default();
        }

        let mut local_node_ids = Vec::new();
        let mut shared_node_ids = Vec::new();

        {
            let mut blueprint = self.blueprint.write();
            for (node_id, node) in &blueprint.nodes {
                let scope = node.scope();
                if scope.is_project_local_to(canonical_project_id) {
                    local_node_ids.push(node_id.clone());
                } else if scope.is_shared_linked_to(canonical_project_id) {
                    shared_node_ids.push(node_id.clone());
                }
            }

            for node_id in &local_node_ids {
                let _ = blueprint.remove_node(node_id);
            }

            for node_id in &shared_node_ids {
                if let Some(node) = blueprint.get_node_mut(node_id) {
                    let scope = node_scope_mut(node);
                    if let Some(shared) = scope.shared.as_mut() {
                        shared
                            .linked_project_ids
                            .retain(|linked| !linked.eq_ignore_ascii_case(canonical_project_id));
                    }
                }
            }

            blueprint.rebuild_indexes();
        }

        let removed_node_id_set: HashSet<String> = local_node_ids.iter().cloned().collect();
        let event_entries_pruned =
            self.prune_project_events(canonical_project_id, &removed_node_id_set);
        let history_snapshots_pruned = self.prune_project_history(canonical_project_id);

        if !local_node_ids.is_empty() || !shared_node_ids.is_empty() || event_entries_pruned > 0 {
            self.mark_dirty();
        }

        ProjectPurgeReport {
            local_nodes_deleted: local_node_ids.len(),
            shared_nodes_unlinked: shared_node_ids.len(),
            event_entries_pruned,
            history_snapshots_pruned,
        }
    }

    fn prune_project_events(&self, project_id: &str, removed_node_ids: &HashSet<String>) -> usize {
        let mut events = self.events.write();
        let before = events.len();
        events.retain(|event| match event {
            BlueprintEvent::NodeCreated { node, .. } => {
                let scope = node.scope();
                !scope.is_project_local_to(project_id) && !scope.is_shared_linked_to(project_id)
            }
            BlueprintEvent::NodeUpdated { before, after, .. } => {
                let before_scope = before.scope();
                let after_scope = after.scope();
                !before_scope.is_project_local_to(project_id)
                    && !before_scope.is_shared_linked_to(project_id)
                    && !after_scope.is_project_local_to(project_id)
                    && !after_scope.is_shared_linked_to(project_id)
            }
            BlueprintEvent::NodeDeleted { node, .. } => {
                let scope = node.scope();
                !scope.is_project_local_to(project_id) && !scope.is_shared_linked_to(project_id)
            }
            BlueprintEvent::EdgeCreated { edge, .. } => {
                !removed_node_ids.contains(edge.source.as_str())
                    && !removed_node_ids.contains(edge.target.as_str())
            }
            BlueprintEvent::EdgesDeleted { edges, .. } => edges.iter().all(|edge| {
                !removed_node_ids.contains(edge.source.as_str())
                    && !removed_node_ids.contains(edge.target.as_str())
            }),
            BlueprintEvent::ExportRecorded {
                project_id: event_project,
                scope_snapshot,
                ..
            } => {
                let scoped_project_match = event_project
                    .as_deref()
                    .map(|value| value.eq_ignore_ascii_case(project_id))
                    .unwrap_or(false);
                let snapshot_mentions_project = scope_snapshot
                    .as_ref()
                    .map(|value| {
                        value
                            .to_string()
                            .to_lowercase()
                            .contains(&project_id.to_lowercase())
                    })
                    .unwrap_or(false);
                !(scoped_project_match || snapshot_mentions_project)
            }
        });
        before - events.len()
    }

    fn prune_project_history(&self, project_id: &str) -> usize {
        let Some(blueprint_dir) = &self.blueprint_dir else {
            return 0;
        };

        let history_dir = blueprint_dir.join("history");
        let mut removed = 0usize;

        let Ok(entries) = std::fs::read_dir(&history_dir) else {
            return 0;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("msgpack") {
                continue;
            }

            let should_remove = match std::fs::read(&path) {
                Ok(bytes) => match rmp_serde::from_slice::<Blueprint>(&bytes) {
                    Ok(snapshot) => snapshot.nodes.values().any(|node| {
                        let scope = node.scope();
                        scope.is_project_local_to(project_id)
                            || scope.is_shared_linked_to(project_id)
                    }),
                    Err(_) => true,
                },
                Err(_) => true,
            };

            if should_remove && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
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

        // Flush event log before clearing dirty so delete compaction failures
        // are surfaced and retried instead of being silently lost.
        self.flush_events(blueprint_dir)?;

        // Clear dirty flag only after all writes succeed.
        *self.dirty.write() = false;

        tracing::debug!(
            "Blueprint flush: {} nodes, {} edges written",
            snapshot.nodes.len(),
            snapshot.edges.len(),
        );

        Ok(true)
    }

    /// Load events from disk.
    fn load_events(blueprint_dir: &Path) -> Vec<BlueprintEvent> {
        let events_path = blueprint_dir.join("events.msgpack");
        match std::fs::read(&events_path) {
            Ok(bytes) => match rmp_serde::from_slice::<Vec<BlueprintEvent>>(&bytes) {
                Ok(events) => {
                    tracing::info!("Blueprint: loaded {} events from disk", events.len());
                    events
                }
                Err(e) => {
                    tracing::warn!("Blueprint: failed to decode events: {}", e);
                    Vec::new()
                }
            },
            Err(_) => Vec::new(), // No events file yet — normal for fresh stores.
        }
    }

    /// Flush the event log to disk (atomic write).
    fn flush_events(&self, blueprint_dir: &Path) -> std::io::Result<()> {
        let events = self.events.read().clone();
        if events.is_empty() {
            let final_path = blueprint_dir.join("events.msgpack");
            if final_path.exists() {
                std::fs::remove_file(&final_path)?;
            }
            return Ok(());
        }

        let bytes = rmp_serde::to_vec_named(&events)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let final_path = blueprint_dir.join("events.msgpack");
        let tmp_path = blueprint_dir.join("events.msgpack.tmp");

        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&tmp_path, &final_path)?;

        tracing::debug!("Blueprint: flushed {} events to disk", events.len());
        Ok(())
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

    /// List history snapshot files (timestamps).
    pub fn list_history(&self) -> Vec<(String, String)> {
        let blueprint_dir = match &self.blueprint_dir {
            Some(d) => d,
            None => return Vec::new(),
        };

        let history_dir = blueprint_dir.join("history");
        let mut entries: Vec<(String, String)> = Vec::new();

        if let Ok(read_dir) = std::fs::read_dir(&history_dir) {
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".msgpack") {
                    let timestamp = name.trim_end_matches(".msgpack").to_string();
                    entries.push((timestamp, name));
                }
            }
        }

        entries.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
        entries
    }

    /// Check if disk backing is enabled.
    pub fn is_persistent(&self) -> bool {
        self.blueprint_dir.is_some()
    }
}

fn node_context_summary(node: &BlueprintNode) -> String {
    match node {
        BlueprintNode::Project(project) => project.description.clone(),
        BlueprintNode::Decision(decision) => decision.context.clone(),
        BlueprintNode::Technology(technology) => match &technology.version {
            Some(version) => format!("{} (version {})", technology.rationale, version),
            None => technology.rationale.clone(),
        },
        BlueprintNode::Component(component) => component.description.clone(),
        BlueprintNode::Constraint(constraint) => constraint.description.clone(),
        BlueprintNode::Pattern(pattern) => {
            if pattern.rationale.trim().is_empty() {
                pattern.description.clone()
            } else {
                format!("{} — {}", pattern.description, pattern.rationale)
            }
        }
        BlueprintNode::QualityRequirement(requirement) => requirement.scenario.clone(),
    }
}

fn node_search_corpus(node: &BlueprintNode) -> String {
    let mut fields = vec![
        node.name().to_lowercase(),
        node.id().as_str().to_lowercase(),
        node.type_name().to_lowercase(),
        node.status().to_lowercase(),
        node.tags().join(" ").to_lowercase(),
        node_context_summary(node).to_lowercase(),
    ];

    match node {
        BlueprintNode::Project(project) => {
            fields.push(project.description.to_lowercase());
        }
        BlueprintNode::Decision(decision) => {
            fields.extend(
                decision
                    .options
                    .iter()
                    .map(|option| option.name.to_lowercase()),
            );
            fields.extend(
                decision
                    .consequences
                    .iter()
                    .map(|consequence| consequence.description.to_lowercase()),
            );
        }
        BlueprintNode::Technology(technology) => {
            if let Some(license) = &technology.license {
                fields.push(license.to_lowercase());
            }
        }
        BlueprintNode::Component(component) => {
            fields.push(component.provides.join(" ").to_lowercase());
            fields.push(component.consumes.join(" ").to_lowercase());
        }
        BlueprintNode::Constraint(constraint) => {
            fields.push(constraint.source.to_lowercase());
        }
        BlueprintNode::Pattern(_) | BlueprintNode::QualityRequirement(_) => {}
    }

    if let Some(doc) = node.documentation() {
        fields.push(doc.to_lowercase());
    }

    fields.join(" ")
}

fn truncate_context_text(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        return compact;
    }

    let truncated: String = compact.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", truncated.trim_end())
}

fn collect_context_node_ids(
    blueprint: &Blueprint,
    root_ids: &[String],
    depth: usize,
) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    for root in root_ids {
        if visited.insert(root.clone()) {
            ordered.push(root.clone());
            queue.push_back((root.clone(), 0usize));
        }
    }

    while let Some((current, current_depth)) = queue.pop_front() {
        if current_depth >= depth {
            continue;
        }

        let mut neighbors: Vec<String> = blueprint
            .forward_neighbors(&current)
            .iter()
            .map(|(_, neighbor)| neighbor.clone())
            .chain(
                blueprint
                    .reverse_neighbors(&current)
                    .iter()
                    .map(|(_, neighbor)| neighbor.clone()),
            )
            .collect();
        neighbors.sort();
        neighbors.dedup();

        for neighbor in neighbors {
            if visited.insert(neighbor.clone()) {
                ordered.push(neighbor.clone());
                queue.push_back((neighbor, current_depth + 1));
            }
        }
    }

    ordered
}

fn context_relation_lines(
    blueprint: &Blueprint,
    node_id: &str,
    included: &HashSet<&str>,
) -> Vec<String> {
    let mut relations = Vec::new();

    for (edge_type, target_id) in blueprint.forward_neighbors(node_id) {
        if included.contains(target_id.as_str()) {
            let target_name = blueprint
                .nodes
                .get(target_id)
                .map(|node| node.name().to_string())
                .unwrap_or_else(|| target_id.clone());
            relations.push(format!(
                "{} -> {} [`{}`]",
                edge_type, target_name, target_id,
            ));
        }
    }

    for (edge_type, source_id) in blueprint.reverse_neighbors(node_id) {
        if included.contains(source_id.as_str()) {
            let source_name = blueprint
                .nodes
                .get(source_id)
                .map(|node| node.name().to_string())
                .unwrap_or_else(|| source_id.clone());
            relations.push(format!(
                "{} <- {} [`{}`]",
                edge_type, source_name, source_id,
            ));
        }
    }

    relations.sort();
    relations.dedup();
    relations.truncate(4);
    relations
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
            documentation: None,
            scope: NodeScope::default(),
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
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        bp.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-cxdb"),
            name: "CXDB".into(),
            component_type: ComponentType::Store,
            naming: None,
            description: "Conversation Experience Database".into(),
            provides: vec!["TurnStore".into()],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        bp.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-event-store"),
            name: "EventStore".into(),
            component_type: ComponentType::Store,
            naming: None,
            description: "Event persistence".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
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
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        }));

        bp.upsert_node(BlueprintNode::QualityRequirement(QualityRequirement {
            id: NodeId::from_raw("qr-crash-safe"),
            attribute: QualityAttribute::Reliability,
            label: None,
            scenario: "Crash-safe persistence".into(),
            priority: QualityPriority::Critical,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
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

    fn make_test_store() -> BlueprintStore {
        let blueprint = make_test_blueprint();
        let store = BlueprintStore::new();

        for node in blueprint.nodes.into_values() {
            store.upsert_node(node);
        }
        for edge in blueprint.edges {
            store.add_edge(edge);
        }

        store
    }

    fn make_scoped_decision(id: &str, title: &str, scope: NodeScope) -> BlueprintNode {
        BlueprintNode::Decision(Decision {
            id: NodeId::from_raw(id),
            title: title.into(),
            status: DecisionStatus::Proposed,
            context: "Scoped decision".into(),
            options: vec![],
            consequences: vec![],
            assumptions: vec![],
            supersedes: None,
            tags: vec![],
            documentation: None,
            scope,
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        })
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
            documentation: None,
            scope: NodeScope::default(),
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
    fn find_relevant_node_ids_matches_names_tags_and_docs() {
        let store = make_test_store();
        store.update_node("comp-cxdb", |node| {
            if let BlueprintNode::Component(component) = node {
                component.tags = vec!["storage".into(), "persistence".into()];
                component.documentation = Some("Backs the persistence layer".into());
            }
        });

        let ids = store.find_relevant_node_ids(
            &["messagepack".into(), "storage".into(), "persistence".into()],
            4,
        );

        assert!(ids.contains(&"dec-use-msgpack".to_string()));
        assert!(ids.contains(&"comp-cxdb".to_string()));
    }

    #[test]
    fn render_context_markdown_includes_neighbors_and_relations() {
        let store = make_test_store();
        let markdown = store
            .render_context_markdown(&["dec-use-msgpack".into()], 1)
            .expect("context markdown should exist");

        assert!(markdown.contains("# Existing Blueprint Context"));
        assert!(markdown
            .contains("Use MessagePack for disk serialization [decision] `dec-use-msgpack`"));
        assert!(markdown.contains("rmp-serde [technology] `tech-rmp-serde`"));
        assert!(markdown.contains("relation: affects -> CXDB [`comp-cxdb`]"));
        assert!(markdown.contains(
            "relation: constrains <- File-system storage over SQLite [`con-fs-storage`]"
        ));
    }

    #[test]
    fn topological_sort_with_cycle() {
        let mut bp = Blueprint::new();

        bp.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("a"),
            name: "A".into(),
            component_type: ComponentType::Module,
            naming: None,
            description: "".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "".into(),
            updated_at: "".into(),
        }));
        bp.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("b"),
            name: "B".into(),
            component_type: ComponentType::Module,
            naming: None,
            description: "".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "".into(),
            updated_at: "".into(),
        }));

        // a -> b -> a (cycle)
        bp.add_edge(Edge {
            source: NodeId::from_raw("a"),
            target: NodeId::from_raw("b"),
            edge_type: EdgeType::DependsOn,
            metadata: None,
        });
        bp.add_edge(Edge {
            source: NodeId::from_raw("b"),
            target: NodeId::from_raw("a"),
            edge_type: EdgeType::DependsOn,
            metadata: None,
        });

        assert!(bp.topological_sort().is_none());
    }

    #[test]
    fn impact_analysis_basic() {
        let bp = make_test_blueprint();

        let report = bp.impact_analysis("dec-use-msgpack", "Switch to SQLite");
        assert!(report.is_some());
        let report = report.unwrap();

        assert_eq!(
            report.source_node_name,
            "Use MessagePack for disk serialization"
        );
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
            options: vec![],
            consequences: vec![],
            assumptions: vec![],
            supersedes: None,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "".into(),
            updated_at: "".into(),
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
                options: vec![],
                consequences: vec![],
                assumptions: vec![],
                supersedes: None,
                tags: vec!["test".into()],
                documentation: None,
                scope: NodeScope::default(),
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
                license: None,
                tags: vec![],
                documentation: None,
                scope: NodeScope::default(),
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
            id: NodeId::from_raw("a"),
            name: "A".into(),
            component_type: ComponentType::Module,
            naming: None,
            description: "".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "".into(),
            updated_at: "".into(),
        }));
        store.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("b"),
            name: "B".into(),
            component_type: ComponentType::Module,
            naming: None,
            description: "".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "".into(),
            updated_at: "".into(),
        }));

        store.add_edge(Edge {
            source: NodeId::from_raw("a"),
            target: NodeId::from_raw("b"),
            edge_type: EdgeType::DependsOn,
            metadata: None,
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
            options: vec![],
            consequences: vec![],
            assumptions: vec![],
            supersedes: None,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "".into(),
            updated_at: "".into(),
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
    fn store_find_component_by_origin_key() {
        let store = BlueprintStore::new();
        store.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-auth"),
            name: "Authentication Service".into(),
            component_type: ComponentType::Service,
            naming: Some(ComponentNaming {
                origin_key: "spec:proj:root:auth".into(),
                source: ComponentNameSource::Generated,
                strategy: ComponentNamingStrategy::SpecGroup,
                generated_name: "Authentication Service".into(),
                naming_version: 1,
                last_generated_at: "2026-03-07T00:00:00Z".into(),
            }),
            description: "Handles authentication".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Planned,
            tags: vec!["spec".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-07T00:00:00Z".into(),
            updated_at: "2026-03-07T00:00:00Z".into(),
        }));

        let found = store
            .find_component_by_origin_key("spec:proj:root:auth")
            .expect("component should be found");
        assert_eq!(found.id.as_str(), "comp-auth");
        assert_eq!(found.name, "Authentication Service");
    }

    #[test]
    fn migrate_legacy_spec_component_naming_promotes_semantic_name_and_origin() {
        let mut blueprint = Blueprint::new();
        blueprint.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-root-input"),
            name: "Root The System Service".into(),
            component_type: ComponentType::Module,
            naming: Some(ComponentNaming {
                origin_key: "spec:proj_1:root:001".into(),
                source: ComponentNameSource::Generated,
                strategy: ComponentNamingStrategy::SpecGroup,
                generated_name: "Root The System Service".into(),
                naming_version: 1,
                last_generated_at: "2026-03-08T00:00:00Z".into(),
            }),
            description: "1 functional requirements: The system must provide a text input".into(),
            provides: vec!["The system must provide a text input that adds a new task".into()],
            consumes: Vec::new(),
            status: ComponentStatus::Planned,
            tags: vec!["spec".into(), "root".into()],
            documentation: None,
            scope: NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "proj-1".into(),
                    project_name: Some("Task Widget".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
            created_at: "2026-03-08T00:00:00Z".into(),
            updated_at: "2026-03-08T00:00:00Z".into(),
        }));

        let migrated = migrate_legacy_spec_component_naming(&mut blueprint);
        assert_eq!(migrated, 1);

        let BlueprintNode::Component(component) = blueprint
            .get_node("comp-root-input")
            .cloned()
            .expect("component should still exist")
        else {
            panic!("expected component node");
        };

        assert_ne!(component.name, "Root The System Service");
        assert!(component.name.contains("Input") || component.name.contains("Entry"));
        assert_eq!(
            component
                .naming
                .as_ref()
                .map(|naming| naming.origin_key.as_str()),
            Some("spec:proj_1:root:input")
        );
    }

    #[test]
    fn migrate_legacy_factory_component_naming_replaces_uuid_workspace_name() {
        let mut blueprint = Blueprint::new();
        blueprint.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-factory-output-1"),
            name: "F6873403 1e41 46fb Workspace".into(),
            component_type: ComponentType::Module,
            naming: Some(ComponentNaming {
                origin_key: "factory:opt/planner/data/worktrees/f6873403_1e41_46fb_8414_b61b90df9003".into(),
                source: ComponentNameSource::Generated,
                strategy: ComponentNamingStrategy::FactoryOutput,
                generated_name: "F6873403 1e41 46fb Workspace".into(),
                naming_version: 1,
                last_generated_at: "2026-03-08T16:21:46Z".into(),
            }),
            description: "Generated code output at /opt/planner/data/worktrees/f6873403-1e41-46fb-8414-b61b90df9003. Build status: Success.".into(),
            provides: vec!["Generated source code".into()],
            consumes: vec!["NLSpec".into()],
            status: ComponentStatus::Shipped,
            tags: vec!["factory".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-08T16:21:46Z".into(),
            updated_at: "2026-03-08T16:21:46Z".into(),
        }));

        let migrated = migrate_legacy_factory_component_naming(&mut blueprint);
        assert_eq!(migrated, 1);

        let BlueprintNode::Component(component) = blueprint
            .get_node("comp-factory-output-1")
            .cloned()
            .expect("factory component should still exist")
        else {
            panic!("expected component node");
        };

        assert_eq!(component.name, "Generated Workspace");
        assert_eq!(
            component
                .naming
                .as_ref()
                .map(|naming| naming.generated_name.as_str()),
            Some("Generated Workspace")
        );
    }

    #[test]
    fn backfill_legacy_factory_project_scope_promotes_generated_workspace_to_project_name() {
        let mut blueprint = Blueprint::new();
        blueprint.upsert_node(BlueprintNode::Project(Project {
            id: NodeId::from_raw("proj-task-widget"),
            name: "Task Widget".into(),
            description: "Blueprint root for project task-widget".into(),
            tags: vec!["project-root".into()],
            documentation: None,
            scope: NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "task-widget".into(),
                    project_name: Some("Task Widget".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
            created_at: "2026-03-08T16:07:38Z".into(),
            updated_at: "2026-03-08T16:07:38Z".into(),
        }));
        blueprint.upsert_node(BlueprintNode::Pattern(Pattern {
            id: NodeId::from_raw("pat-factory-execution-1"),
            name: "Dark Factory Code Generation".into(),
            description: "Codex-powered code generation".into(),
            rationale: "Automated code generation".into(),
            tags: vec!["factory".into(), "codegen".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-08T16:21:46Z".into(),
            updated_at: "2026-03-08T16:21:46Z".into(),
        }));
        blueprint.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-factory-output-1"),
            name: "Generated Workspace".into(),
            component_type: ComponentType::Module,
            naming: Some(ComponentNaming {
                origin_key: "factory:opt/planner/data/worktrees/f6873403_1e41_46fb_8414_b61b90df9003".into(),
                source: ComponentNameSource::Generated,
                strategy: ComponentNamingStrategy::FactoryOutput,
                generated_name: "Generated Workspace".into(),
                naming_version: 1,
                last_generated_at: "2026-03-08T16:21:46Z".into(),
            }),
            description: "Generated code output at /opt/planner/data/worktrees/f6873403-1e41-46fb-8414-b61b90df9003. Build status: Success.".into(),
            provides: vec!["Generated source code".into()],
            consumes: vec!["NLSpec".into()],
            status: ComponentStatus::Shipped,
            tags: vec!["factory".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-08T16:21:46Z".into(),
            updated_at: "2026-03-08T16:21:46Z".into(),
        }));

        let backfilled = backfill_legacy_factory_project_scope(&mut blueprint);
        let renamed = migrate_legacy_factory_component_naming(&mut blueprint);

        assert_eq!(backfilled, 2);
        assert_eq!(renamed, 1);

        let BlueprintNode::Component(component) = blueprint
            .get_node("comp-factory-output-1")
            .cloned()
            .expect("factory component should still exist")
        else {
            panic!("expected component node");
        };
        assert_eq!(component.name, "Task Widget Generated Workspace");
        assert_eq!(
            component
                .scope
                .project
                .as_ref()
                .and_then(|project| project.project_name.as_deref()),
            Some("Task Widget")
        );
        assert!(blueprint.edges.iter().any(|edge| {
            edge.source == NodeId::from_raw("proj-task-widget")
                && edge.target == NodeId::from_raw("comp-factory-output-1")
                && edge.edge_type == EdgeType::Contains
        }));
        assert!(blueprint.edges.iter().any(|edge| {
            edge.source == NodeId::from_raw("proj-task-widget")
                && edge.target == NodeId::from_raw("pat-factory-execution-1")
                && edge.edge_type == EdgeType::Contains
        }));
    }

    #[test]
    fn backfill_project_scope_from_contains_edges_scopes_discovered_component() {
        let mut blueprint = Blueprint::new();
        blueprint.upsert_node(BlueprintNode::Project(Project {
            id: NodeId::from_raw("proj-task-widget"),
            name: "Task widget".into(),
            description: "Blueprint root for project task-widget".into(),
            tags: vec!["project-root".into()],
            documentation: None,
            scope: NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "8be86283-8ffd-4507-ac7c-2d35709eddbc".into(),
                    project_name: Some("Task widget".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
            created_at: "2026-03-08T16:07:38Z".into(),
            updated_at: "2026-03-08T16:07:38Z".into(),
        }));
        blueprint.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-planner-web-1"),
            name: "Planner Web UI".into(),
            component_type: ComponentType::Interface,
            naming: None,
            description: "Discovered package".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Shipped,
            tags: vec!["discovery".into(), "directory".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-15T17:23:30Z".into(),
            updated_at: "2026-03-15T17:23:30Z".into(),
        }));
        blueprint.add_edge(Edge {
            source: NodeId::from_raw("proj-task-widget"),
            target: NodeId::from_raw("comp-planner-web-1"),
            edge_type: EdgeType::Contains,
            metadata: Some("cgc:indexed-package".into()),
        });

        let migrated = backfill_project_scope_from_contains_edges(&mut blueprint);
        assert_eq!(migrated, 1);

        let BlueprintNode::Component(component) = blueprint
            .get_node("comp-planner-web-1")
            .cloned()
            .expect("component should still exist")
        else {
            panic!("expected component node");
        };
        assert_eq!(component.scope.scope_class, ScopeClass::Project);
        assert_eq!(
            component
                .scope
                .project
                .as_ref()
                .map(|scope| scope.project_id.as_str()),
            Some("8be86283-8ffd-4507-ac7c-2d35709eddbc")
        );
        assert_eq!(
            component
                .scope
                .project
                .as_ref()
                .and_then(|scope| scope.project_name.as_deref()),
            Some("Task widget")
        );
    }

    #[test]
    fn backfill_single_project_review_scope_attaches_ar_constraint() {
        let mut blueprint = Blueprint::new();
        blueprint.upsert_node(BlueprintNode::Project(Project {
            id: NodeId::from_raw("proj-task-widget"),
            name: "Task widget".into(),
            description: "Blueprint root for project task-widget".into(),
            tags: vec!["project-root".into()],
            documentation: None,
            scope: NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "8be86283-8ffd-4507-ac7c-2d35709eddbc".into(),
                    project_name: Some("Task widget".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
            created_at: "2026-03-08T16:07:38Z".into(),
            updated_at: "2026-03-08T16:07:38Z".into(),
        }));
        blueprint.upsert_node(BlueprintNode::Constraint(Constraint {
            id: NodeId::from_raw("con-ar-1"),
            title: "Persistence behavior is underspecified".into(),
            constraint_type: ConstraintType::Technical,
            description: "Legacy adversarial review constraint".into(),
            source: "Adversarial Review".into(),
            tags: vec!["ar-review".into(), "blocking".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-08T16:13:35Z".into(),
            updated_at: "2026-03-08T16:13:35Z".into(),
        }));

        let migrated = backfill_single_project_review_scope(&mut blueprint);
        assert_eq!(migrated, 1);

        let BlueprintNode::Constraint(constraint) = blueprint
            .get_node("con-ar-1")
            .cloned()
            .expect("constraint should still exist")
        else {
            panic!("expected constraint node");
        };
        assert_eq!(constraint.scope.scope_class, ScopeClass::Project);
        assert!(blueprint.edges.iter().any(|edge| {
            edge.source == NodeId::from_raw("proj-task-widget")
                && edge.target == NodeId::from_raw("con-ar-1")
                && edge.edge_type == EdgeType::Contains
        }));
    }

    #[test]
    fn archive_stale_factory_history_keeps_only_latest_factory_nodes_active() {
        let mut blueprint = Blueprint::new();
        let project_scope = NodeScope {
            scope_class: ScopeClass::Project,
            project: Some(ProjectScope {
                project_id: "task-widget".into(),
                project_name: Some("Task widget".into()),
            }),
            secondary: SecondaryScopeRefs::default(),
            is_shared: false,
            shared: None,
            lifecycle: NodeLifecycle::Active,
            override_scope: None,
            scope_review: None,
        };

        for (id, updated_at) in [
            ("comp-factory-output-old", "2026-03-08T16:21:46Z"),
            ("comp-factory-output-new", "2026-03-08T17:09:07Z"),
        ] {
            blueprint.upsert_node(BlueprintNode::Component(Component {
                id: NodeId::from_raw(id),
                name: "Task widget Workspace".into(),
                component_type: ComponentType::Module,
                naming: Some(ComponentNaming {
                    origin_key: format!("factory:{}", id),
                    source: ComponentNameSource::Generated,
                    strategy: ComponentNamingStrategy::FactoryOutput,
                    generated_name: "Task widget Workspace".into(),
                    naming_version: 1,
                    last_generated_at: updated_at.into(),
                }),
                description: "Generated output".into(),
                provides: vec![],
                consumes: vec![],
                status: ComponentStatus::Shipped,
                tags: vec!["factory".into()],
                documentation: None,
                scope: project_scope.clone(),
                created_at: updated_at.into(),
                updated_at: updated_at.into(),
            }));
        }

        for (id, updated_at) in [
            ("pat-factory-old", "2026-03-08T16:21:46Z"),
            ("pat-factory-new", "2026-03-08T17:09:07Z"),
        ] {
            blueprint.upsert_node(BlueprintNode::Pattern(Pattern {
                id: NodeId::from_raw(id),
                name: "Dark Factory Code Generation".into(),
                description: "Factory run".into(),
                rationale: "Automation".into(),
                tags: vec!["factory".into(), "codegen".into()],
                documentation: None,
                scope: project_scope.clone(),
                created_at: updated_at.into(),
                updated_at: updated_at.into(),
            }));
        }

        let migrated = archive_stale_factory_history(&mut blueprint);
        assert_eq!(migrated, 2);

        let old_component = blueprint
            .get_node("comp-factory-output-old")
            .expect("old component");
        let new_component = blueprint
            .get_node("comp-factory-output-new")
            .expect("new component");
        let old_pattern = blueprint.get_node("pat-factory-old").expect("old pattern");
        let new_pattern = blueprint.get_node("pat-factory-new").expect("new pattern");

        assert_eq!(old_component.scope().lifecycle, NodeLifecycle::Archived);
        assert_eq!(new_component.scope().lifecycle, NodeLifecycle::Active);
        assert_eq!(old_pattern.scope().lifecycle, NodeLifecycle::Archived);
        assert_eq!(new_pattern.scope().lifecycle, NodeLifecycle::Active);
    }

    #[test]
    fn migrate_generated_directory_component_naming_updates_role_and_acronyms() {
        let mut blueprint = Blueprint::new();
        for (id, origin_key, name, component_type) in [
            (
                "comp-planner-tui",
                "path:planner-tui",
                "Planner Tui",
                ComponentType::Interface,
            ),
            (
                "comp-planner-schemas",
                "path:planner-schemas",
                "Planner Schemas Service",
                ComponentType::Service,
            ),
        ] {
            blueprint.upsert_node(BlueprintNode::Component(Component {
                id: NodeId::from_raw(id),
                name: name.into(),
                component_type: component_type.clone(),
                naming: Some(ComponentNaming {
                    origin_key: origin_key.into(),
                    source: ComponentNameSource::Generated,
                    strategy: ComponentNamingStrategy::DirectoryScan,
                    generated_name: name.into(),
                    naming_version: 1,
                    last_generated_at: "2026-03-15T17:21:24Z".into(),
                }),
                description: "Discovered package".into(),
                provides: vec![],
                consumes: vec![],
                status: ComponentStatus::Shipped,
                tags: vec!["discovery".into(), "directory".into()],
                documentation: None,
                scope: NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: "task-widget".into(),
                        project_name: Some("Task widget".into()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: false,
                    shared: None,
                    lifecycle: NodeLifecycle::Active,
                    override_scope: None,
            scope_review: None,
                },
                created_at: "2026-03-15T17:21:24Z".into(),
                updated_at: "2026-03-15T17:21:24Z".into(),
            }));
        }

        let migrated = migrate_generated_directory_component_naming(&mut blueprint);
        assert_eq!(migrated, 2);

        let tui = blueprint
            .get_node("comp-planner-tui")
            .expect("tui component");
        let schemas = blueprint
            .get_node("comp-planner-schemas")
            .expect("schemas component");
        assert_eq!(tui.name(), "Planner TUI");
        assert_eq!(schemas.name(), "Planner Schemas Library");
    }

    #[test]
    fn migrate_generated_factory_display_names_disambiguates_archived_history() {
        let mut blueprint = Blueprint::new();
        for (id, updated_at, lifecycle) in [
            (
                "comp-factory-output-old",
                "2026-03-08T16:21:46Z",
                NodeLifecycle::Archived,
            ),
            (
                "comp-factory-output-new",
                "2026-03-08T17:09:07Z",
                NodeLifecycle::Active,
            ),
        ] {
            blueprint.upsert_node(BlueprintNode::Component(Component {
                id: NodeId::from_raw(id),
                name: "Task widget Workspace".into(),
                component_type: ComponentType::Module,
                naming: Some(ComponentNaming {
                    origin_key:
                        "factory:opt/planner/data/worktrees/f6873403_1e41_46fb_8414_b61b90df9003"
                            .into(),
                    source: ComponentNameSource::Generated,
                    strategy: ComponentNamingStrategy::FactoryOutput,
                    generated_name: "Task widget Workspace".into(),
                    naming_version: 1,
                    last_generated_at: updated_at.into(),
                }),
                description: "Generated output".into(),
                provides: vec![],
                consumes: vec![],
                status: ComponentStatus::Shipped,
                tags: vec!["factory".into()],
                documentation: None,
                scope: NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: "task-widget".into(),
                        project_name: Some("Task widget".into()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: false,
                    shared: None,
                    lifecycle,
                    override_scope: None,
            scope_review: None,
                },
                created_at: updated_at.into(),
                updated_at: updated_at.into(),
            }));
        }
        for (id, updated_at, lifecycle) in [
            (
                "pat-factory-old",
                "2026-03-08T16:21:46Z",
                NodeLifecycle::Archived,
            ),
            (
                "pat-factory-new",
                "2026-03-08T17:09:07Z",
                NodeLifecycle::Active,
            ),
        ] {
            blueprint.upsert_node(BlueprintNode::Pattern(Pattern {
                id: NodeId::from_raw(id),
                name: "Dark Factory Code Generation".into(),
                description: "Factory run".into(),
                rationale: "Automation".into(),
                tags: vec!["factory".into(), "codegen".into()],
                documentation: None,
                scope: NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: "task-widget".into(),
                        project_name: Some("Task widget".into()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: false,
                    shared: None,
                    lifecycle,
                    override_scope: None,
            scope_review: None,
                },
                created_at: updated_at.into(),
                updated_at: updated_at.into(),
            }));
        }

        let migrated = migrate_generated_factory_display_names(&mut blueprint);
        assert_eq!(migrated, 4);

        assert_eq!(
            blueprint
                .get_node("comp-factory-output-new")
                .expect("new factory")
                .name(),
            "Task widget Generated Workspace"
        );
        assert_eq!(
            blueprint
                .get_node("comp-factory-output-old")
                .expect("old factory")
                .name(),
            "Task widget Generated Workspace Snapshot 2026-03-08 16:21"
        );
        assert_eq!(
            blueprint
                .get_node("pat-factory-new")
                .expect("new pattern")
                .name(),
            "Task widget Factory Code Generation"
        );
        assert_eq!(
            blueprint
                .get_node("pat-factory-old")
                .expect("old pattern")
                .name(),
            "Task widget Factory Code Generation Snapshot 2026-03-08 16:21"
        );
    }

    #[test]
    fn migrate_constraint_titles_replaces_truncated_sentence_titles() {
        let mut blueprint = Blueprint::new();
        blueprint.upsert_node(BlueprintNode::Constraint(Constraint {
            id: NodeId::from_raw("con-1"),
            title:
                "Inline editing behavior is not specified (how edit mode is entered, how changes …"
                    .into(),
            constraint_type: ConstraintType::Technical,
            description:
                "Inline editing behavior is not specified (how edit mode is entered, how changes are saved)"
                    .into(),
            source: "AR".into(),
            tags: vec!["ar-review".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-08T16:13:35Z".into(),
            updated_at: "2026-03-08T16:13:35Z".into(),
        }));

        let migrated = migrate_constraint_titles(&mut blueprint);
        assert_eq!(migrated, 1);
        assert_eq!(
            blueprint.get_node("con-1").expect("constraint").name(),
            "Specify inline editing contract"
        );
    }

    #[test]
    fn migrate_quality_requirement_labels_backfills_concise_label() {
        let mut blueprint = Blueprint::new();
        blueprint.upsert_node(BlueprintNode::QualityRequirement(QualityRequirement {
            id: NodeId::from_raw("qr-1"),
            attribute: QualityAttribute::Reliability,
            label: None,
            scenario:
                "User can type a task name, press Enter, and see it appear in the list instantly"
                    .into(),
            priority: QualityPriority::Critical,
            tags: vec!["satisfaction".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: "2026-03-08T16:12:44Z".into(),
            updated_at: "2026-03-08T16:12:44Z".into(),
        }));

        let migrated = migrate_quality_requirement_labels(&mut blueprint);
        assert_eq!(migrated, 1);

        let BlueprintNode::QualityRequirement(qr) = blueprint.get_node("qr-1").expect("qr") else {
            panic!("expected qr");
        };
        assert_eq!(qr.label.as_deref(), Some("Goal: Add tasks on Enter"));
        assert_eq!(
            BlueprintNode::QualityRequirement(qr.clone()).name(),
            "Goal: Add tasks on Enter"
        );
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
                options: vec![],
                consequences: vec![],
                assumptions: vec![],
                supersedes: None,
                tags: vec![],
                documentation: None,
                scope: NodeScope::default(),
                created_at: "".into(),
                updated_at: "".into(),
            }));

            store.upsert_node(BlueprintNode::Decision(Decision {
                id: NodeId::from_raw("dec-stays"),
                title: "Stays".into(),
                status: DecisionStatus::Accepted,
                context: "".into(),
                options: vec![],
                consequences: vec![],
                assumptions: vec![],
                supersedes: None,
                tags: vec![],
                documentation: None,
                scope: NodeScope::default(),
                created_at: "".into(),
                updated_at: "".into(),
            }));

            store.flush().unwrap();

            // Verify both files exist.
            assert!(data_dir
                .join("blueprint/nodes/dec-to-remove.msgpack")
                .exists());
            assert!(data_dir.join("blueprint/nodes/dec-stays.msgpack").exists());

            // Remove one node and flush again.
            store.remove_node("dec-to-remove");
            store.flush().unwrap();

            // File should be cleaned up.
            assert!(!data_dir
                .join("blueprint/nodes/dec-to-remove.msgpack")
                .exists());
            assert!(data_dir.join("blueprint/nodes/dec-stays.msgpack").exists());
        }

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn purge_project_deletes_project_local_nodes() {
        let store = BlueprintStore::new();
        store.upsert_node(make_scoped_decision(
            "dec-proj-a",
            "Project A local",
            NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "proj-a".into(),
                    project_name: Some("Project A".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
        ));
        store.upsert_node(make_scoped_decision(
            "dec-proj-b",
            "Project B local",
            NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "proj-b".into(),
                    project_name: Some("Project B".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
        ));

        let report = store.purge_project("proj-a");
        assert_eq!(report.local_nodes_deleted, 1);
        assert!(store.get_node("dec-proj-a").is_none());
        assert!(store.get_node("dec-proj-b").is_some());
    }

    #[test]
    fn purge_project_unlinks_shared_nodes() {
        let store = BlueprintStore::new();
        store.upsert_node(make_scoped_decision(
            "dec-shared",
            "Shared record",
            NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "proj-b".into(),
                    project_name: Some("Project B".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: true,
                shared: Some(SharedScope {
                    linked_project_ids: vec!["proj-a".into(), "proj-b".into()],
                    inherit_to_linked_projects: true,
                }),
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
        ));

        let report = store.purge_project("proj-a");
        assert_eq!(report.shared_nodes_unlinked, 1);

        let node = store.get_node("dec-shared").unwrap();
        let linked = node
            .scope()
            .shared
            .as_ref()
            .map(|shared| shared.linked_project_ids.clone())
            .unwrap_or_default();
        assert_eq!(linked, vec!["proj-b".to_string()]);
    }

    #[test]
    fn purge_project_preserves_other_project_links() {
        let store = BlueprintStore::new();
        store.upsert_node(make_scoped_decision(
            "dec-shared-2",
            "Shared record",
            NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "proj-b".into(),
                    project_name: Some("Project B".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: true,
                shared: Some(SharedScope {
                    linked_project_ids: vec!["proj-a".into(), "proj-b".into(), "proj-c".into()],
                    inherit_to_linked_projects: true,
                }),
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
            scope_review: None,
            },
        ));

        store.purge_project("proj-a");
        let node = store.get_node("dec-shared-2").unwrap();
        let linked = node
            .scope()
            .shared
            .as_ref()
            .map(|shared| shared.linked_project_ids.clone())
            .unwrap_or_default();
        assert_eq!(linked, vec!["proj-b".to_string(), "proj-c".to_string()]);
    }

    #[test]
    fn purge_project_compacts_event_log_for_deleted_project() {
        let data_dir = temp_data_dir();
        {
            let store = BlueprintStore::open(&data_dir).unwrap();
            store.upsert_node(make_scoped_decision(
                "dec-purge-local",
                "Local record",
                NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: "proj-a".into(),
                        project_name: Some("Project A".into()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: false,
                    shared: None,
                    lifecycle: NodeLifecycle::Active,
                    override_scope: None,
            scope_review: None,
                },
            ));
            store.upsert_node(make_scoped_decision(
                "dec-purge-shared",
                "Shared record",
                NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: "proj-b".into(),
                        project_name: Some("Project B".into()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: true,
                    shared: Some(SharedScope {
                        linked_project_ids: vec!["proj-a".into(), "proj-b".into()],
                        inherit_to_linked_projects: true,
                    }),
                    lifecycle: NodeLifecycle::Active,
                    override_scope: None,
            scope_review: None,
                },
            ));
            store.save_snapshot().unwrap();
            store.flush().unwrap();

            let report = store.purge_project("proj-a");
            assert!(report.event_entries_pruned > 0);
            assert!(report.history_snapshots_pruned > 0);
            store.flush().unwrap();
        }

        let reopened = BlueprintStore::open(&data_dir).unwrap();
        assert!(reopened.get_node("dec-purge-local").is_none());
        let shared = reopened.get_node("dec-purge-shared").unwrap();
        let linked = shared
            .scope()
            .shared
            .as_ref()
            .map(|scope| scope.linked_project_ids.clone())
            .unwrap_or_default();
        assert_eq!(linked, vec!["proj-b".to_string()]);

        let events_path = data_dir.join("blueprint/events.msgpack");
        if events_path.exists() {
            let bytes = std::fs::read(&events_path).unwrap();
            let events: Vec<BlueprintEvent> = rmp_serde::from_slice(&bytes).unwrap();
            let serialized_events = serde_json::to_string(&events).unwrap().to_lowercase();
            assert!(!serialized_events.contains("proj-a"));
        }

        let history_dir = data_dir.join("blueprint/history");
        let remaining_history = std::fs::read_dir(&history_dir).unwrap().count();
        assert_eq!(remaining_history, 0);

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn flush_preserves_dirty_when_event_log_write_fails() {
        let data_dir = temp_data_dir();
        let store = BlueprintStore::open(&data_dir).unwrap();
        store.upsert_node(make_scoped_decision(
            "dec-flush-fail",
            "Flush fail",
            NodeScope::default(),
        ));

        let tmp_conflict = data_dir.join("blueprint/events.msgpack.tmp");
        std::fs::create_dir_all(&tmp_conflict).unwrap();

        let error = store.flush().unwrap_err();
        assert_ne!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(store.is_dirty());

        std::fs::remove_dir_all(&tmp_conflict).unwrap();
        let flushed = store.flush().unwrap();
        assert!(flushed);
        assert!(!store.is_dirty());

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn open_migrates_legacy_scope_tags_and_flushes_to_disk() {
        let data_dir = temp_data_dir();
        {
            let store = BlueprintStore::open(&data_dir).unwrap();
            store.upsert_node(BlueprintNode::Decision(Decision {
                id: NodeId::from_raw("dec-legacy-scope"),
                title: "Legacy scope".into(),
                status: DecisionStatus::Accepted,
                context: "legacy".into(),
                options: vec![],
                consequences: vec![],
                assumptions: vec![],
                supersedes: None,
                tags: vec![
                    "archived".into(),
                    "overrides:shared-guidance".into(),
                    "team:Platform".into(),
                ],
                documentation: None,
                scope: NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: "proj-alpha".into(),
                        project_name: Some("Alpha Project".into()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: false,
                    shared: None,
                    lifecycle: NodeLifecycle::Active,
                    override_scope: None,
            scope_review: None,
                },
                created_at: "2026-03-08T00:00:00Z".into(),
                updated_at: "2026-03-08T00:00:00Z".into(),
            }));
            store.flush().unwrap();
        }

        {
            let reopened = BlueprintStore::open(&data_dir).unwrap();
            let node = reopened.get_node("dec-legacy-scope").expect("node should reopen");
            assert_eq!(node.scope().lifecycle, NodeLifecycle::Archived);
            let override_scope = node
                .scope()
                .override_scope
                .as_ref()
                .expect("override_scope should be migrated");
            assert_eq!(override_scope.shared_source_id, "shared-guidance");
            assert_eq!(
                override_scope.override_reason.as_deref(),
                Some("migrated from legacy override tag")
            );
            assert_eq!(node.tags(), &["team:Platform".to_string()]);
            assert!(!reopened.is_dirty());
        }

        {
            let reopened_again = BlueprintStore::open(&data_dir).unwrap();
            let node = reopened_again
                .get_node("dec-legacy-scope")
                .expect("node should persist migrated state");
            assert_eq!(node.scope().lifecycle, NodeLifecycle::Archived);
            assert_eq!(
                node.scope()
                    .override_scope
                    .as_ref()
                    .map(|scope| scope.shared_source_id.as_str()),
                Some("shared-guidance")
            );
            assert_eq!(node.tags(), &["team:Platform".to_string()]);
        }

        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
