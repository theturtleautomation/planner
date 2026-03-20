use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use planner_schemas::artifacts::blueprint::*;

use crate::blueprint::{Blueprint, BlueprintStore};
use crate::component_naming::{generate_directory_name, DirectoryNamingInput};

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
    CodeGraphContext,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedEdge {
    pub id: String,
    pub edge: Edge,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProposalNodeRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_origin_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technology_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedEdgeProposal {
    pub edge_type: EdgeType,
    pub source: ProposalNodeRef,
    pub target: ProposalNodeRef,
    pub reason: String,
    pub confidence: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_artifact: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImportedEdgeProposalEnvelope {
    proposals: Vec<ImportedEdgeProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeImportResult {
    pub inserted: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
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
    edge_proposals: RwLock<Vec<ProposedEdge>>,
    proposals_path: Option<PathBuf>,
    edge_proposals_path: Option<PathBuf>,
}

impl ProposalStore {
    pub fn new() -> Self {
        Self {
            proposals: RwLock::new(Vec::new()),
            edge_proposals: RwLock::new(Vec::new()),
            proposals_path: None,
            edge_proposals_path: None,
        }
    }

    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        let blueprint_dir = data_dir.join("blueprint");
        std::fs::create_dir_all(&blueprint_dir)?;
        let proposals_path = blueprint_dir.join("proposals.msgpack");
        let edge_proposals_path = blueprint_dir.join("edge_proposals.msgpack");

        let proposals = if proposals_path.exists() {
            let bytes = std::fs::read(&proposals_path)?;
            rmp_serde::from_slice::<Vec<ProposedNode>>(&bytes).map_err(|err| {
                std::io::Error::other(format!("failed to decode proposals: {}", err))
            })?
        } else {
            Vec::new()
        };

        let edge_proposals = if edge_proposals_path.exists() {
            let bytes = std::fs::read(&edge_proposals_path)?;
            rmp_serde::from_slice::<Vec<ProposedEdge>>(&bytes).map_err(|err| {
                std::io::Error::other(format!("failed to decode edge proposals: {}", err))
            })?
        } else {
            Vec::new()
        };

        Ok(Self {
            proposals: RwLock::new(proposals),
            edge_proposals: RwLock::new(edge_proposals),
            proposals_path: Some(proposals_path),
            edge_proposals_path: Some(edge_proposals_path),
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

    pub fn list_edge_proposals(&self, status: Option<ProposalStatus>) -> Vec<ProposedEdge> {
        let mut proposals: Vec<ProposedEdge> = self
            .edge_proposals
            .read()
            .iter()
            .filter(|proposal| status.map(|value| proposal.status == value).unwrap_or(true))
            .cloned()
            .collect();
        proposals.sort_by(|left, right| right.proposed_at.cmp(&left.proposed_at));
        proposals
    }

    pub fn get_edge_proposal(&self, proposal_id: &str) -> Option<ProposedEdge> {
        self.edge_proposals
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

    pub fn insert_many_edges(
        &self,
        proposals: Vec<ProposedEdge>,
    ) -> std::io::Result<(usize, usize)> {
        let mut guard = self.edge_proposals.write();
        let mut fingerprints: HashSet<String> =
            guard.iter().map(edge_proposal_fingerprint).collect();
        let mut inserted = 0usize;
        let mut skipped = 0usize;

        for proposal in proposals {
            let fingerprint = edge_proposal_fingerprint(&proposal);
            if !fingerprints.insert(fingerprint) {
                skipped += 1;
                continue;
            }

            guard.push(proposal);
            inserted += 1;
        }

        self.persist_edges_locked(&guard)?;
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

    pub fn mark_edge_accepted(&self, proposal_id: &str) -> std::io::Result<Option<ProposedEdge>> {
        self.update_edge_status(proposal_id, ProposalStatus::Accepted, None)
    }

    pub fn mark_edge_merged(&self, proposal_id: &str) -> std::io::Result<Option<ProposedEdge>> {
        self.update_edge_status(proposal_id, ProposalStatus::Merged, None)
    }

    pub fn mark_edge_rejected(
        &self,
        proposal_id: &str,
        reason: Option<String>,
    ) -> std::io::Result<Option<ProposedEdge>> {
        self.update_edge_status(proposal_id, ProposalStatus::Rejected, reason)
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

    fn update_edge_status(
        &self,
        proposal_id: &str,
        status: ProposalStatus,
        review_note: Option<String>,
    ) -> std::io::Result<Option<ProposedEdge>> {
        let mut guard = self.edge_proposals.write();
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
        self.persist_edges_locked(&guard)?;
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

    fn persist_edges_locked(&self, proposals: &[ProposedEdge]) -> std::io::Result<()> {
        let Some(path) = &self.edge_proposals_path else {
            return Ok(());
        };

        let bytes = rmp_serde::to_vec_named(proposals).map_err(|err| {
            std::io::Error::other(format!("failed to encode edge proposals: {}", err))
        })?;

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
                output.errors.push(format!("{}: {}", relative, err));
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
                scope: NodeScope::default(),
                created_at: now_iso(),
                updated_at: now_iso(),
            });

            output.proposals.push(ProposedNode {
                id: Uuid::new_v4().to_string(),
                node,
                source: DiscoverySource::CargoToml,
                reason: format!(
                    "Dependency '{}' discovered in {}",
                    dependency.name, relative
                ),
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
    let existing_origins = existing_component_origin_keys(blueprints);
    let mut seen_origins = HashSet::new();
    let project_name_hint = project_root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty());

    for path in directory_candidates(project_root) {
        let relative = relative_display(project_root, &path);
        let component_type = infer_component_type(&relative);
        let naming_ts = now_iso();
        let generated = generate_directory_name(DirectoryNamingInput {
            relative_path: &relative,
            project_name: project_name_hint,
            component_type: component_type.clone(),
            timestamp: &naming_ts,
        });
        let origin_key = generated.naming.origin_key.to_ascii_lowercase();
        if !seen_origins.insert(origin_key.clone()) || existing_origins.contains(&origin_key) {
            output.skipped_count += 1;
            continue;
        }

        let node = BlueprintNode::Component(Component {
            id: NodeId::with_prefix("COMP", &relative.replace(['/', '\\'], "-")),
            name: generated.name.clone(),
            component_type,
            naming: Some(generated.naming),
            description: format!("Discovered from directory structure at {}", relative),
            provides: Vec::new(),
            consumes: Vec::new(),
            status: ComponentStatus::Planned,
            tags: vec!["discovery".into(), "directory".into()],
            documentation: None,
            scope: NodeScope::default(),
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
    let identity = match &proposal.node {
        BlueprintNode::Component(component) => component
            .naming
            .as_ref()
            .map(|naming| format!("origin:{}", naming.origin_key.to_ascii_lowercase()))
            .unwrap_or_else(|| format!("name:{}", proposal.node.name().to_lowercase())),
        _ => format!("name:{}", proposal.node.name().to_lowercase()),
    };

    format!(
        "{}:{}:{}:{}",
        proposal.node.type_name(),
        identity,
        proposal.source_artifact.as_deref().unwrap_or(""),
        match proposal.source {
            DiscoverySource::CargoToml => "cargo_toml",
            DiscoverySource::DirectoryScan => "directory_scan",
            DiscoverySource::PipelineRun => "pipeline_run",
            DiscoverySource::Manual => "manual",
            DiscoverySource::CodeGraphContext => "code_graph_context",
        }
    )
}

fn edge_proposal_fingerprint(proposal: &ProposedEdge) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        proposal.edge.edge_type,
        proposal.edge.source,
        proposal.edge.target,
        proposal.source_artifact.as_deref().unwrap_or(""),
        match proposal.source {
            DiscoverySource::CargoToml => "cargo_toml",
            DiscoverySource::DirectoryScan => "directory_scan",
            DiscoverySource::PipelineRun => "pipeline_run",
            DiscoverySource::Manual => "manual",
            DiscoverySource::CodeGraphContext => "code_graph_context",
        }
    )
}

#[derive(Debug, Clone)]
struct ResolvedNodeRef {
    id: NodeId,
    type_name: &'static str,
}

pub fn import_edge_proposals(
    proposals: &ProposalStore,
    blueprints: &BlueprintStore,
    imports: Vec<ImportedEdgeProposal>,
) -> std::io::Result<EdgeImportResult> {
    let snapshot = blueprints.snapshot();
    let mut resolved = Vec::new();
    let mut errors = Vec::new();

    for (index, proposal) in imports.into_iter().enumerate() {
        match resolve_imported_edge_proposal(&snapshot, proposal) {
            Ok(proposal) => resolved.push(proposal),
            Err(err) => errors.push(format!("proposal {}: {}", index + 1, err)),
        }
    }

    let (inserted, skipped) = proposals.insert_many_edges(resolved)?;
    Ok(EdgeImportResult {
        inserted,
        skipped,
        errors,
    })
}

pub fn collect_code_graph_edge_proposals(
    project_root: &Path,
    blueprints: &BlueprintStore,
) -> Result<Vec<ImportedEdgeProposal>, String> {
    if configured_cgc_binary().is_none() {
        return collect_code_graph_edge_proposals_from_command(project_root);
    }

    let indexed_files = ensure_cgc_index_and_collect_files(project_root)?;
    let snapshot = blueprints.snapshot();
    let packages = collect_workspace_packages(project_root, &indexed_files)?;
    if packages.is_empty() {
        return Err(
            "CodeGraphContext did not find any indexed workspace packages under the scan root"
                .into(),
        );
    }

    let path_components = existing_path_backed_components(&snapshot);
    let local_component_refs: HashMap<String, String> = packages
        .iter()
        .filter_map(|package| {
            path_components
                .get(&package.origin_key.to_ascii_lowercase())
                .map(|_| (package.origin_key.clone(), package.relative_dir.clone()))
        })
        .collect();

    if local_component_refs.is_empty() {
        let package_list = packages
            .iter()
            .map(|package| package.relative_dir.clone())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "CodeGraphContext indexed this repo, but the blueprint has no filesystem-backed components to map those packages. Run the directory_structure scanner and accept components for: {}",
            package_list
        ));
    }

    let project_id = infer_project_id_for_code_graph(&snapshot, path_components.values().collect());
    let existing_technologies = existing_technology_name_map(&snapshot);
    let mut seen = HashSet::new();
    let mut proposals = Vec::new();

    for package in &packages {
        if !local_component_refs.contains_key(&package.origin_key) {
            continue;
        }

        if let Some(project_id) = project_id.as_deref() {
            let key = format!("contains:{}:{}", project_id, package.origin_key);
            if seen.insert(key) {
                proposals.push(ImportedEdgeProposal {
                    edge_type: EdgeType::Contains,
                    source: ProposalNodeRef {
                        project_id: Some(project_id.to_string()),
                        ..ProposalNodeRef::default()
                    },
                    target: ProposalNodeRef {
                        component_origin_key: Some(package.origin_key.clone()),
                        ..ProposalNodeRef::default()
                    },
                    reason: format!(
                        "Indexed package '{}' belongs to this project",
                        package.relative_dir
                    ),
                    confidence: 0.95,
                    metadata: Some("cgc:indexed-package".into()),
                    source_artifact: Some(package.relative_dir.clone()),
                });
            }
        }

        for dependency in &package.local_dependencies {
            let Some(target_package) = packages
                .iter()
                .find(|candidate| &candidate.name == dependency)
            else {
                continue;
            };
            if !local_component_refs.contains_key(&target_package.origin_key) {
                continue;
            }

            let key = format!(
                "depends_on:{}:{}",
                package.origin_key, target_package.origin_key
            );
            if seen.insert(key) {
                proposals.push(ImportedEdgeProposal {
                    edge_type: EdgeType::DependsOn,
                    source: ProposalNodeRef {
                        component_origin_key: Some(package.origin_key.clone()),
                        ..ProposalNodeRef::default()
                    },
                    target: ProposalNodeRef {
                        component_origin_key: Some(target_package.origin_key.clone()),
                        ..ProposalNodeRef::default()
                    },
                    reason: format!(
                        "Indexed package '{}' declares a dependency on '{}'",
                        package.relative_dir, target_package.relative_dir
                    ),
                    confidence: 0.92,
                    metadata: Some(format!(
                        "manifest:{}->{}",
                        package.name, target_package.name
                    )),
                    source_artifact: Some(package.manifest_path.clone()),
                });
            }
        }

        for dependency in &package.external_dependencies {
            let Some(technology_name) = existing_technologies.get(&dependency.to_ascii_lowercase())
            else {
                continue;
            };

            let key = format!(
                "uses:{}:{}",
                package.origin_key,
                technology_name.to_ascii_lowercase()
            );
            if seen.insert(key) {
                proposals.push(ImportedEdgeProposal {
                    edge_type: EdgeType::Uses,
                    source: ProposalNodeRef {
                        component_origin_key: Some(package.origin_key.clone()),
                        ..ProposalNodeRef::default()
                    },
                    target: ProposalNodeRef {
                        technology_name: Some(technology_name.clone()),
                        ..ProposalNodeRef::default()
                    },
                    reason: format!(
                        "Indexed package '{}' declares a dependency on '{}'",
                        package.relative_dir, technology_name
                    ),
                    confidence: 0.86,
                    metadata: Some(format!("manifest:{}->{}", package.name, technology_name)),
                    source_artifact: Some(package.manifest_path.clone()),
                });
            }
        }
    }

    if proposals.is_empty() {
        return Err(
            "CodeGraphContext indexed the repo, but no resolvable blueprint relationships were found. This usually means the current blueprint has no accepted path-based components or matching technology nodes yet.".into(),
        );
    }

    Ok(proposals)
}

fn parse_imported_edge_proposals_payload(raw: &str) -> Result<Vec<ImportedEdgeProposal>, String> {
    let payload = normalize_imported_edge_payload(raw);
    if payload.is_empty() {
        return Err("code graph command returned empty output".into());
    }

    if let Ok(items) = serde_json::from_str::<Vec<ImportedEdgeProposal>>(&payload) {
        return Ok(items);
    }

    if let Ok(envelope) = serde_json::from_str::<ImportedEdgeProposalEnvelope>(&payload) {
        return Ok(envelope.proposals);
    }

    Err(
        "unable to parse code graph output; expected JSON array or object with 'proposals' array"
            .into(),
    )
}

fn normalize_imported_edge_payload(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    let mut lines = trimmed.lines();
    let Some(first_line) = lines.next() else {
        return String::new();
    };

    if !first_line.trim_start().starts_with("```") {
        return trimmed.to_string();
    }

    let mut body = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            break;
        }
        body.push(line);
    }

    body.join("\n").trim().to_string()
}

#[derive(Debug, Clone)]
struct WorkspacePackage {
    name: String,
    relative_dir: String,
    manifest_path: String,
    origin_key: String,
    local_dependencies: Vec<String>,
    external_dependencies: Vec<String>,
}

#[derive(Debug)]
struct PathBackedComponentRef {
    project_id: Option<String>,
}

pub fn code_graph_context_available() -> bool {
    configured_cgc_binary().is_some() || legacy_scan_command_configured().is_some()
}

fn configured_cgc_binary() -> Option<String> {
    let configured = std::env::var("PLANNER_CGC_COMMAND")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(value) = configured {
        return Some(value);
    }

    let default_path = "/opt/planner/bin/cgc";
    if Path::new(default_path).is_file() {
        return Some(default_path.to_string());
    }

    None
}

fn legacy_scan_command_configured() -> Option<String> {
    std::env::var("PLANNER_CGC_SCAN_COMMAND")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn cgc_timeout_secs(var_name: &str, default_secs: u64, min_secs: u64) -> u64 {
    std::env::var(var_name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.max(min_secs))
        .unwrap_or(default_secs)
}

fn run_cgc_command(
    project_root: &Path,
    args: &[String],
    timeout_secs: u64,
) -> Result<String, String> {
    let Some(binary) = configured_cgc_binary() else {
        return Err("CodeGraphContext binary is not configured. Set PLANNER_CGC_COMMAND or install /opt/planner/bin/cgc".into());
    };

    let output = Command::new("timeout")
        .arg("--signal=TERM")
        .arg(format!("{}s", timeout_secs))
        .arg(binary)
        .args(args)
        .current_dir(project_root)
        .env("COLUMNS", "1000")
        .env("NO_COLOR", "1")
        .output()
        .map_err(|err| format!("failed to run CodeGraphContext: {}", err))?;

    if output.status.code() == Some(124) {
        return Err(format!(
            "CodeGraphContext command timed out after {} seconds",
            timeout_secs
        ));
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit status {}", output.status)
        };
        return Err(format!("CodeGraphContext command failed: {}", detail));
    }

    String::from_utf8(output.stdout)
        .map_err(|_| "CodeGraphContext output is not valid UTF-8".to_string())
}

fn collect_code_graph_edge_proposals_from_command(
    project_root: &Path,
) -> Result<Vec<ImportedEdgeProposal>, String> {
    let command = legacy_scan_command_configured().ok_or_else(|| {
        "CodeGraphContext is not available and PLANNER_CGC_SCAN_COMMAND is not configured"
            .to_string()
    })?;
    let timeout_secs = cgc_timeout_secs("PLANNER_CGC_SCAN_TIMEOUT_SECS", 180, 10);
    let wrapped_command = format!("timeout --signal=TERM {}s {}", timeout_secs, command);

    let output = Command::new("/bin/bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-lc")
        .arg(wrapped_command)
        .current_dir(project_root)
        .env("PLANNER_PROJECT_ROOT", project_root.as_os_str())
        .output()
        .map_err(|err| format!("failed to run legacy code graph command: {}", err))?;

    if output.status.code() == Some(124) {
        return Err(format!(
            "legacy code graph command timed out after {} seconds",
            timeout_secs
        ));
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit status {}", output.status)
        };
        return Err(format!("legacy code graph command failed: {}", detail));
    }

    let raw = String::from_utf8(output.stdout)
        .map_err(|_| "legacy code graph command output is not valid UTF-8".to_string())?;
    parse_imported_edge_proposals_payload(&raw)
}

fn cypher_string_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn query_cgc_file_paths(project_root: &Path) -> Result<Vec<String>, String> {
    let query = format!(
        "MATCH (f:File) WHERE f.path STARTS WITH \"{}/\" RETURN f.path AS path LIMIT 50000",
        cypher_string_literal(&project_root.display().to_string())
    );
    let stdout = run_cgc_command(
        project_root,
        &["query".into(), query],
        cgc_timeout_secs("PLANNER_CGC_SCAN_TIMEOUT_SECS", 60, 10),
    )?;
    let json_payload = strip_non_json_prefix(&stdout);
    let json: Value = serde_json::from_str(&json_payload).map_err(|err| {
        format!(
            "failed to decode CodeGraphContext file query output: {}",
            err
        )
    })?;
    let rows = json
        .as_array()
        .ok_or_else(|| "CodeGraphContext file query did not return a JSON array".to_string())?;

    let mut paths = Vec::new();
    for row in rows {
        if let Some(path) = row.get("path").and_then(|value| value.as_str()) {
            paths.push(path.to_string());
        }
    }

    Ok(paths)
}

fn strip_non_json_prefix(raw: &str) -> String {
    let trimmed = raw.trim();
    let object_start = trimmed.find('{');
    let array_start = trimmed.find('[');
    let start = match (object_start, array_start) {
        (Some(left), Some(right)) => left.min(right),
        (Some(left), None) => left,
        (None, Some(right)) => right,
        (None, None) => 0,
    };
    trimmed[start..].trim().to_string()
}

fn ensure_cgc_index_and_collect_files(project_root: &Path) -> Result<Vec<String>, String> {
    let existing = query_cgc_file_paths(project_root)?;
    if !existing.is_empty() {
        return Ok(existing);
    }

    let timeout_secs = cgc_timeout_secs("PLANNER_CGC_INDEX_TIMEOUT_SECS", 300, 30);
    let _ = run_cgc_command(
        project_root,
        &["index".into(), project_root.display().to_string()],
        timeout_secs,
    )?;

    let indexed = query_cgc_file_paths(project_root)?;
    if indexed.is_empty() {
        return Err(
            "CodeGraphContext index completed but no indexed files were found under the scan root"
                .into(),
        );
    }

    Ok(indexed)
}

fn existing_path_backed_components(
    blueprint: &Blueprint,
) -> HashMap<String, PathBackedComponentRef> {
    blueprint
        .nodes
        .values()
        .filter_map(|node| match node {
            BlueprintNode::Component(component) => component.naming.as_ref().and_then(|naming| {
                if !naming.origin_key.to_ascii_lowercase().starts_with("path:") {
                    return None;
                }
                Some((
                    naming.origin_key.to_ascii_lowercase(),
                    PathBackedComponentRef {
                        project_id: component
                            .scope
                            .project
                            .as_ref()
                            .map(|scope| scope.project_id.clone()),
                    },
                ))
            }),
            _ => None,
        })
        .collect()
}

fn infer_project_id_for_code_graph(
    blueprint: &Blueprint,
    path_components: Vec<&PathBackedComponentRef>,
) -> Option<String> {
    let project_ids: HashSet<String> = path_components
        .into_iter()
        .filter_map(|component| component.project_id.clone())
        .collect();
    if project_ids.len() == 1 {
        return project_ids.into_iter().next();
    }

    let project_nodes: Vec<String> = blueprint
        .nodes
        .values()
        .filter_map(|node| match node {
            BlueprintNode::Project(project) => project
                .scope
                .project
                .as_ref()
                .map(|scope| scope.project_id.clone()),
            _ => None,
        })
        .collect();
    if project_nodes.len() == 1 {
        return project_nodes.into_iter().next();
    }

    None
}

fn existing_technology_name_map(blueprint: &Blueprint) -> HashMap<String, String> {
    blueprint
        .nodes
        .values()
        .filter_map(|node| match node {
            BlueprintNode::Technology(technology) => Some((
                technology.name.to_ascii_lowercase(),
                technology.name.clone(),
            )),
            _ => None,
        })
        .collect()
}

fn collect_workspace_packages(
    project_root: &Path,
    indexed_files: &[String],
) -> Result<Vec<WorkspacePackage>, String> {
    let indexed_files: Vec<String> = indexed_files
        .iter()
        .map(|path| {
            Path::new(path)
                .strip_prefix(project_root)
                .map(|relative| relative.display().to_string())
                .unwrap_or_else(|_| path.clone())
        })
        .collect();

    let cargo_packages = collect_cargo_workspace_packages(project_root, &indexed_files)?;
    let npm_packages = collect_npm_workspace_packages(project_root, &indexed_files)?;

    let mut packages = cargo_packages;
    for package in npm_packages {
        if packages
            .iter()
            .any(|existing| existing.origin_key == package.origin_key)
        {
            continue;
        }
        packages.push(package);
    }
    packages.sort_by(|left, right| left.relative_dir.cmp(&right.relative_dir));
    Ok(packages)
}

fn collect_cargo_workspace_packages(
    project_root: &Path,
    indexed_files: &[String],
) -> Result<Vec<WorkspacePackage>, String> {
    let manifests = collect_paths(project_root, "Cargo.toml");
    let mut package_entries = Vec::new();

    for manifest in manifests {
        let relative_manifest = relative_display(project_root, &manifest);
        let Some(relative_dir) = Path::new(&relative_manifest)
            .parent()
            .map(|path| path.display().to_string())
        else {
            continue;
        };
        if relative_dir.is_empty() {
            continue;
        }

        let contents = std::fs::read_to_string(&manifest)
            .map_err(|err| format!("{}: {}", relative_manifest, err))?;
        let Some(name) = parse_cargo_package_name(&contents) else {
            continue;
        };
        if !indexed_files.iter().any(|path| {
            path == &relative_manifest || path.starts_with(&(relative_dir.clone() + "/"))
        }) {
            continue;
        }

        let generated = generate_directory_name(DirectoryNamingInput {
            relative_path: &relative_dir,
            project_name: project_root.file_name().and_then(|value| value.to_str()),
            component_type: infer_component_type(&relative_dir),
            timestamp: &now_iso(),
        });
        let dependencies = parse_cargo_dependencies(&contents)
            .into_iter()
            .map(|dep| dep.name)
            .collect::<Vec<_>>();

        package_entries.push((
            name,
            relative_dir,
            relative_manifest,
            generated.naming.origin_key,
            dependencies,
        ));
    }

    let local_names: HashSet<String> = package_entries
        .iter()
        .map(|(name, _, _, _, _)| name.clone())
        .collect();

    Ok(package_entries
        .into_iter()
        .map(
            |(name, relative_dir, manifest_path, origin_key, dependencies)| {
                let (local_dependencies, external_dependencies): (Vec<_>, Vec<_>) = dependencies
                    .into_iter()
                    .partition(|dependency| local_names.contains(dependency));
                WorkspacePackage {
                    name,
                    relative_dir,
                    manifest_path,
                    origin_key,
                    local_dependencies,
                    external_dependencies,
                }
            },
        )
        .collect())
}

fn collect_npm_workspace_packages(
    project_root: &Path,
    indexed_files: &[String],
) -> Result<Vec<WorkspacePackage>, String> {
    let manifests = collect_paths(project_root, "package.json");
    let mut package_entries = Vec::new();

    for manifest in manifests {
        let relative_manifest = relative_display(project_root, &manifest);
        let Some(relative_dir) = Path::new(&relative_manifest)
            .parent()
            .map(|path| path.display().to_string())
        else {
            continue;
        };
        if relative_dir.is_empty() {
            continue;
        }
        if !indexed_files.iter().any(|path| {
            path == &relative_manifest || path.starts_with(&(relative_dir.clone() + "/"))
        }) {
            continue;
        }

        let contents = std::fs::read_to_string(&manifest)
            .map_err(|err| format!("{}: {}", relative_manifest, err))?;
        let json: Value = serde_json::from_str(&contents)
            .map_err(|err| format!("{}: {}", relative_manifest, err))?;
        let Some(name) = json.get("name").and_then(|value| value.as_str()) else {
            continue;
        };

        let mut dependencies = Vec::new();
        for field in ["dependencies", "devDependencies", "peerDependencies"] {
            if let Some(object) = json.get(field).and_then(|value| value.as_object()) {
                dependencies.extend(object.keys().cloned());
            }
        }

        let generated = generate_directory_name(DirectoryNamingInput {
            relative_path: &relative_dir,
            project_name: project_root.file_name().and_then(|value| value.to_str()),
            component_type: infer_component_type(&relative_dir),
            timestamp: &now_iso(),
        });
        package_entries.push((
            name.to_string(),
            relative_dir,
            relative_manifest,
            generated.naming.origin_key,
            dependencies,
        ));
    }

    let local_names: HashSet<String> = package_entries
        .iter()
        .map(|(name, _, _, _, _)| name.clone())
        .collect();

    Ok(package_entries
        .into_iter()
        .map(
            |(name, relative_dir, manifest_path, origin_key, dependencies)| {
                let (local_dependencies, external_dependencies): (Vec<_>, Vec<_>) = dependencies
                    .into_iter()
                    .partition(|dependency| local_names.contains(dependency));
                WorkspacePackage {
                    name,
                    relative_dir,
                    manifest_path,
                    origin_key,
                    local_dependencies,
                    external_dependencies,
                }
            },
        )
        .collect())
}

fn parse_cargo_package_name(contents: &str) -> Option<String> {
    let mut section = String::new();
    for raw_line in contents.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(['[', ']']).to_string();
            continue;
        }

        if section != "package" {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "name" {
            return extract_quoted(value.trim());
        }
    }

    None
}

fn resolve_imported_edge_proposal(
    blueprint: &Blueprint,
    proposal: ImportedEdgeProposal,
) -> Result<ProposedEdge, String> {
    if !(0.0..=1.0).contains(&proposal.confidence) {
        return Err(format!(
            "confidence must be between 0.0 and 1.0, got {}",
            proposal.confidence
        ));
    }

    let source =
        resolve_node_ref(blueprint, &proposal.source).map_err(|err| format!("source {}", err))?;
    let target =
        resolve_node_ref(blueprint, &proposal.target).map_err(|err| format!("target {}", err))?;

    if source.id == target.id {
        return Err("source and target resolve to the same node".into());
    }

    validate_import_edge_types(proposal.edge_type, &source, &target)?;

    Ok(ProposedEdge {
        id: Uuid::new_v4().to_string(),
        edge: Edge {
            source: source.id,
            target: target.id,
            edge_type: proposal.edge_type,
            metadata: proposal.metadata,
        },
        source: DiscoverySource::CodeGraphContext,
        reason: proposal.reason,
        status: ProposalStatus::Pending,
        proposed_at: now_iso(),
        reviewed_at: None,
        confidence: proposal.confidence,
        source_artifact: proposal.source_artifact,
        review_note: None,
    })
}

fn validate_import_edge_types(
    edge_type: EdgeType,
    source: &ResolvedNodeRef,
    target: &ResolvedNodeRef,
) -> Result<(), String> {
    match edge_type {
        EdgeType::Contains => {
            if source.type_name != "project" {
                return Err("contains edges must start at a project node".into());
            }
        }
        EdgeType::DecidedBy => {
            let valid_source = matches!(source.type_name, "technology" | "component" | "pattern");
            if !valid_source || target.type_name != "decision" {
                return Err(
                    "decided_by edges must connect technology/component/pattern -> decision".into(),
                );
            }
        }
        EdgeType::Supersedes => {
            if source.type_name != "decision" || target.type_name != "decision" {
                return Err("supersedes edges must connect decision -> decision".into());
            }
        }
        EdgeType::DependsOn => {
            if source.type_name != "component" || target.type_name != "component" {
                return Err("depends_on edges must connect component -> component".into());
            }
        }
        EdgeType::Uses => {
            if source.type_name != "component" || target.type_name != "technology" {
                return Err("uses edges must connect component -> technology".into());
            }
        }
        EdgeType::Constrains => {
            let valid_target = matches!(target.type_name, "decision" | "component" | "technology");
            if source.type_name != "constraint" || !valid_target {
                return Err(
                    "constrains edges must connect constraint -> decision/component/technology"
                        .into(),
                );
            }
        }
        EdgeType::Implements => {
            if source.type_name != "component" || target.type_name != "pattern" {
                return Err("implements edges must connect component -> pattern".into());
            }
        }
        EdgeType::Satisfies => {
            let valid_source = matches!(source.type_name, "component" | "decision" | "pattern");
            if !valid_source || target.type_name != "quality_requirement" {
                return Err(
                    "satisfies edges must connect component/decision/pattern -> quality_requirement"
                        .into(),
                );
            }
        }
        EdgeType::Affects => {
            let valid_target = matches!(target.type_name, "component" | "technology");
            if source.type_name != "decision" || !valid_target {
                return Err("affects edges must connect decision -> component/technology".into());
            }
        }
    }

    Ok(())
}

fn resolve_node_ref(
    blueprint: &Blueprint,
    reference: &ProposalNodeRef,
) -> Result<ResolvedNodeRef, String> {
    if let Some(node_id) = reference.node_id.as_deref() {
        let node = blueprint
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("node_id '{}' not found", node_id))?;
        return Ok(ResolvedNodeRef {
            id: node.id().clone(),
            type_name: node.type_name(),
        });
    }

    if let Some(origin_key) = reference.component_origin_key.as_deref() {
        let node = blueprint
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::Component(component)
                    if component.naming.as_ref().is_some_and(|naming| {
                        naming.origin_key.eq_ignore_ascii_case(origin_key)
                    }) =>
                {
                    Some(node)
                }
                _ => None,
            })
            .ok_or_else(|| format!("component_origin_key '{}' not found", origin_key))?;
        return Ok(ResolvedNodeRef {
            id: node.id().clone(),
            type_name: node.type_name(),
        });
    }

    if let Some(technology_name) = reference.technology_name.as_deref() {
        let node = blueprint
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::Technology(technology)
                    if technology.name.eq_ignore_ascii_case(technology_name) =>
                {
                    Some(node)
                }
                _ => None,
            })
            .ok_or_else(|| format!("technology_name '{}' not found", technology_name))?;
        return Ok(ResolvedNodeRef {
            id: node.id().clone(),
            type_name: node.type_name(),
        });
    }

    if let Some(project_id) = reference.project_id.as_deref() {
        let node =
            blueprint
                .nodes
                .values()
                .find_map(|node| match node {
                    BlueprintNode::Project(project)
                        if project.scope.project.as_ref().is_some_and(|scope| {
                            scope.project_id.eq_ignore_ascii_case(project_id)
                        }) =>
                    {
                        Some(node)
                    }
                    _ => None,
                })
                .ok_or_else(|| format!("project_id '{}' not found", project_id))?;
        return Ok(ResolvedNodeRef {
            id: node.id().clone(),
            type_name: node.type_name(),
        });
    }

    Err(
        "reference must include node_id, component_origin_key, technology_name, or project_id"
            .into(),
    )
}

fn existing_node_names(blueprints: &BlueprintStore, node_type: &str) -> HashSet<String> {
    blueprints
        .list_by_type(node_type)
        .into_iter()
        .map(|node| node.name.to_lowercase())
        .collect()
}

fn existing_component_origin_keys(blueprints: &BlueprintStore) -> HashSet<String> {
    blueprints
        .snapshot()
        .nodes
        .values()
        .filter_map(|node| match node {
            BlueprintNode::Component(component) => component
                .naming
                .as_ref()
                .map(|naming| naming.origin_key.to_ascii_lowercase()),
            _ => None,
        })
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
        "api"
            | "app"
            | "cli"
            | "core"
            | "db"
            | "lib"
            | "pipeline"
            | "services"
            | "store"
            | "ui"
            | "web"
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
    } else if normalized.contains("schemas")
        || normalized.contains("schema")
        || normalized.contains("core")
        || normalized.contains("lib")
    {
        ComponentType::Library
    } else if normalized.contains("api")
        || normalized.contains("service")
        || normalized.ends_with("server")
    {
        ComponentType::Service
    } else if normalized.contains("store") || normalized.contains("db") {
        ComponentType::Store
    } else if normalized.contains("web") || normalized.contains("ui") || normalized.contains("cli")
    {
        ComponentType::Interface
    } else {
        ComponentType::Module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(prefix: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("planner-discovery-{}-{}", prefix, Uuid::new_v4()));
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
        let origin_keys: HashSet<String> = output
            .proposals
            .iter()
            .filter_map(|proposal| match &proposal.node {
                BlueprintNode::Component(component) => {
                    component.naming.as_ref().map(|n| n.origin_key.clone())
                }
                _ => None,
            })
            .collect();

        assert!(
            names
                .iter()
                .all(|name| name != "Api" && name != "Core" && name != "Web"),
            "Directory scan should avoid weak placeholder names: {:?}",
            names
        );
        assert!(origin_keys.contains("path:api"));
        assert!(origin_keys.contains("path:core"));
        assert!(origin_keys.contains("path:src/web"));

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
                scope: NodeScope::default(),
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
        store
            .mark_rejected(&proposal.id, Some("not needed".into()))
            .unwrap();

        let reopened = ProposalStore::open(&dir).unwrap();
        let rejected = reopened.list(Some(ProposalStatus::Rejected));
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].id, proposal.id);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn proposal_store_dedupes_component_proposals_by_origin_key() {
        let store = ProposalStore::new();
        let ts = now_iso();
        let first = ProposedNode {
            id: Uuid::new_v4().to_string(),
            node: BlueprintNode::Component(Component {
                id: NodeId::with_prefix("COMP", "auth"),
                name: "Authentication Service".into(),
                component_type: ComponentType::Service,
                naming: Some(ComponentNaming {
                    origin_key: "path:src/auth".into(),
                    source: ComponentNameSource::Generated,
                    strategy: ComponentNamingStrategy::DirectoryScan,
                    generated_name: "Authentication Service".into(),
                    naming_version: 1,
                    last_generated_at: ts.clone(),
                }),
                description: "First proposal".into(),
                provides: vec![],
                consumes: vec![],
                status: ComponentStatus::Planned,
                tags: vec!["discovery".into()],
                documentation: None,
                scope: NodeScope::default(),
                created_at: ts.clone(),
                updated_at: ts.clone(),
            }),
            source: DiscoverySource::DirectoryScan,
            reason: "Detected from src/auth".into(),
            status: ProposalStatus::Pending,
            proposed_at: ts.clone(),
            reviewed_at: None,
            confidence: 0.8,
            source_artifact: Some("src/auth".into()),
            review_note: None,
        };
        let mut second = first.clone();
        second.id = Uuid::new_v4().to_string();
        if let BlueprintNode::Component(component) = &mut second.node {
            component.name = "Identity Service".into();
            if let Some(naming) = component.naming.as_mut() {
                naming.generated_name = "Identity Service".into();
            }
        }

        let (inserted, skipped) = store.insert_many(vec![first, second]).unwrap();
        assert_eq!(inserted, 1);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn import_edge_proposals_resolves_component_origin_keys_and_technology_names() {
        let store = ProposalStore::new();
        let blueprints = BlueprintStore::new();
        let ts = now_iso();

        blueprints.upsert_node(BlueprintNode::Project(Project {
            id: NodeId::from_raw("proj-task-widget"),
            name: "task-widget".into(),
            description: "Task widget blueprint root".into(),
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
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));

        blueprints.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-review-controls"),
            name: "Review Controls UI".into(),
            component_type: ComponentType::Module,
            naming: Some(ComponentNaming {
                origin_key: "path:src/review".into(),
                source: ComponentNameSource::Generated,
                strategy: ComponentNamingStrategy::DirectoryScan,
                generated_name: "Review Controls UI".into(),
                naming_version: 1,
                last_generated_at: ts.clone(),
            }),
            description: "Review controls".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Planned,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));

        blueprints.upsert_node(BlueprintNode::Technology(Technology {
            id: NodeId::from_raw("tech-dnd-kit"),
            name: "@dnd-kit/core".into(),
            version: None,
            category: TechnologyCategory::Library,
            ring: AdoptionRing::Adopt,
            rationale: "Used for drag and drop".into(),
            license: None,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));

        let result = import_edge_proposals(
            &store,
            &blueprints,
            vec![
                ImportedEdgeProposal {
                    edge_type: EdgeType::Contains,
                    source: ProposalNodeRef {
                        project_id: Some("task-widget".into()),
                        ..ProposalNodeRef::default()
                    },
                    target: ProposalNodeRef {
                        component_origin_key: Some("path:src/review".into()),
                        ..ProposalNodeRef::default()
                    },
                    reason: "Project owns the review controls component".into(),
                    confidence: 0.9,
                    metadata: Some("cgc import".into()),
                    source_artifact: Some("src/review".into()),
                },
                ImportedEdgeProposal {
                    edge_type: EdgeType::Uses,
                    source: ProposalNodeRef {
                        component_origin_key: Some("path:src/review".into()),
                        ..ProposalNodeRef::default()
                    },
                    target: ProposalNodeRef {
                        technology_name: Some("@dnd-kit/core".into()),
                        ..ProposalNodeRef::default()
                    },
                    reason: "Review controls import dnd-kit".into(),
                    confidence: 0.88,
                    metadata: None,
                    source_artifact: Some("src/review/SortableList.tsx".into()),
                },
            ],
        )
        .unwrap();

        assert_eq!(result.inserted, 2);
        assert_eq!(result.skipped, 0);
        assert!(result.errors.is_empty());

        let proposals = store.list_edge_proposals(Some(ProposalStatus::Pending));
        assert_eq!(proposals.len(), 2);
        assert!(proposals
            .iter()
            .any(|proposal| proposal.edge.edge_type == EdgeType::Contains));
        assert!(proposals
            .iter()
            .any(|proposal| proposal.edge.edge_type == EdgeType::Uses));
    }

    #[test]
    fn import_edge_proposals_accepts_semantic_edge_types() {
        let store = ProposalStore::new();
        let blueprints = BlueprintStore::new();
        let ts = now_iso();

        blueprints.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-auth"),
            name: "Authentication Service".into(),
            component_type: ComponentType::Module,
            naming: Some(ComponentNaming {
                origin_key: "spec:proj:root:auth".into(),
                source: ComponentNameSource::Generated,
                strategy: ComponentNamingStrategy::SpecGroup,
                generated_name: "Authentication Service".into(),
                naming_version: 1,
                last_generated_at: ts.clone(),
            }),
            description: "Authentication".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Planned,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));
        blueprints.upsert_node(BlueprintNode::Technology(Technology {
            id: NodeId::from_raw("tech-auth0"),
            name: "Auth0".into(),
            version: None,
            category: TechnologyCategory::Platform,
            ring: AdoptionRing::Adopt,
            rationale: "Identity provider".into(),
            license: None,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));
        blueprints.upsert_node(BlueprintNode::Constraint(Constraint {
            id: NodeId::from_raw("con-auth0"),
            title: "Use Auth0".into(),
            constraint_type: ConstraintType::Technical,
            description: "Authentication must use Auth0".into(),
            source: "test".into(),
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));
        blueprints.upsert_node(BlueprintNode::QualityRequirement(QualityRequirement {
            id: NodeId::from_raw("qr-login"),
            attribute: QualityAttribute::Reliability,
            label: None,
            scenario: "Login flow completes in under 2s".into(),
            priority: QualityPriority::Critical,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));

        let result = import_edge_proposals(
            &store,
            &blueprints,
            vec![
                ImportedEdgeProposal {
                    edge_type: EdgeType::Constrains,
                    source: ProposalNodeRef {
                        node_id: Some("con-auth0".into()),
                        ..ProposalNodeRef::default()
                    },
                    target: ProposalNodeRef {
                        technology_name: Some("Auth0".into()),
                        ..ProposalNodeRef::default()
                    },
                    reason: "Auth constraint limits Auth0 usage".into(),
                    confidence: 0.9,
                    metadata: None,
                    source_artifact: None,
                },
                ImportedEdgeProposal {
                    edge_type: EdgeType::Satisfies,
                    source: ProposalNodeRef {
                        component_origin_key: Some("spec:proj:root:auth".into()),
                        ..ProposalNodeRef::default()
                    },
                    target: ProposalNodeRef {
                        node_id: Some("qr-login".into()),
                        ..ProposalNodeRef::default()
                    },
                    reason: "Authentication service satisfies login quality goal".into(),
                    confidence: 0.87,
                    metadata: None,
                    source_artifact: None,
                },
            ],
        )
        .unwrap();

        assert_eq!(result.inserted, 2);
        assert!(result.errors.is_empty());

        let proposals = store.list_edge_proposals(Some(ProposalStatus::Pending));
        assert!(proposals
            .iter()
            .any(|proposal| proposal.edge.edge_type == EdgeType::Constrains));
        assert!(proposals
            .iter()
            .any(|proposal| proposal.edge.edge_type == EdgeType::Satisfies));
    }

    #[test]
    fn import_edge_proposals_rejects_invalid_semantic_edge_pairs() {
        let ts = now_iso();
        let mut blueprint = Blueprint::default();
        blueprint.upsert_node(BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-auth"),
            name: "Authentication Service".into(),
            component_type: ComponentType::Module,
            naming: Some(ComponentNaming {
                origin_key: "spec:proj:root:auth".into(),
                source: ComponentNameSource::Generated,
                strategy: ComponentNamingStrategy::SpecGroup,
                generated_name: "Authentication Service".into(),
                naming_version: 1,
                last_generated_at: ts.clone(),
            }),
            description: "Authentication".into(),
            provides: vec![],
            consumes: vec![],
            status: ComponentStatus::Planned,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));
        blueprint.upsert_node(BlueprintNode::Technology(Technology {
            id: NodeId::from_raw("tech-auth0"),
            name: "Auth0".into(),
            version: None,
            category: TechnologyCategory::Platform,
            ring: AdoptionRing::Adopt,
            rationale: "Identity provider".into(),
            license: None,
            tags: vec![],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }));

        let err = resolve_imported_edge_proposal(
            &blueprint,
            ImportedEdgeProposal {
                edge_type: EdgeType::Constrains,
                source: ProposalNodeRef {
                    node_id: Some("comp-auth".into()),
                    ..ProposalNodeRef::default()
                },
                target: ProposalNodeRef {
                    technology_name: Some("Auth0".into()),
                    ..ProposalNodeRef::default()
                },
                reason: "invalid".into(),
                confidence: 0.5,
                metadata: None,
                source_artifact: None,
            },
        )
        .unwrap_err();

        assert!(err.contains("constrains edges must connect constraint"));
    }

    #[test]
    fn proposal_store_persists_edge_proposals() {
        let dir = temp_dir("edge-store");
        let store = ProposalStore::open(&dir).unwrap();
        let proposal = ProposedEdge {
            id: Uuid::new_v4().to_string(),
            edge: Edge {
                source: NodeId::from_raw("proj-task-widget"),
                target: NodeId::from_raw("comp-review-controls"),
                edge_type: EdgeType::Contains,
                metadata: Some("cgc import".into()),
            },
            source: DiscoverySource::CodeGraphContext,
            reason: "Project contains review controls".into(),
            status: ProposalStatus::Pending,
            proposed_at: now_iso(),
            reviewed_at: None,
            confidence: 0.92,
            source_artifact: Some("src/review".into()),
            review_note: None,
        };

        store.insert_many_edges(vec![proposal.clone()]).unwrap();
        store
            .mark_edge_rejected(&proposal.id, Some("not needed".into()))
            .unwrap();

        let reopened = ProposalStore::open(&dir).unwrap();
        let rejected = reopened.list_edge_proposals(Some(ProposalStatus::Rejected));
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].id, proposal.id);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_imported_edge_proposals_accepts_array_payload() {
        let payload = r#"
        [
          {
            "edge_type": "depends_on",
            "source": { "component_origin_key": "path:src/review" },
            "target": { "component_origin_key": "path:src/list" },
            "reason": "Review imports list",
            "confidence": 0.9
          }
        ]
        "#;

        let parsed = parse_imported_edge_proposals_payload(payload).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].edge_type, EdgeType::DependsOn);
    }

    #[test]
    fn parse_imported_edge_proposals_accepts_envelope_payload() {
        let payload = r#"
        {
          "proposals": [
            {
              "edge_type": "uses",
              "source": { "component_origin_key": "path:src/review" },
              "target": { "technology_name": "@dnd-kit/core" },
              "reason": "Review uses dnd-kit",
              "confidence": 0.88
            }
          ]
        }
        "#;

        let parsed = parse_imported_edge_proposals_payload(payload).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].edge_type, EdgeType::Uses);
    }

    #[test]
    fn parse_imported_edge_proposals_accepts_fenced_json_payload() {
        let payload = r#"
```json
{
  "proposals": [
    {
      "edge_type": "contains",
      "source": { "project_id": "proj-1" },
      "target": { "component_origin_key": "path:src/app" },
      "reason": "repo contains app",
      "confidence": 0.8
    }
  ]
}
```
        "#;

        let parsed = parse_imported_edge_proposals_payload(payload).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].edge_type, EdgeType::Contains);
    }

    #[test]
    fn collect_workspace_packages_extracts_local_dependencies_and_origin_keys() {
        let dir = temp_dir("workspace-packages");
        std::fs::create_dir_all(dir.join("planner-core/src")).unwrap();
        std::fs::create_dir_all(dir.join("planner-server/src")).unwrap();
        std::fs::create_dir_all(dir.join("planner-web/src")).unwrap();
        std::fs::write(
            dir.join("planner-core/Cargo.toml"),
            r#"
                [package]
                name = "planner-core"

                [dependencies]
                planner-schemas = { path = "../planner-schemas" }
                serde = "1"
            "#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("planner-schemas/src")).unwrap();
        std::fs::write(
            dir.join("planner-schemas/Cargo.toml"),
            r#"
                [package]
                name = "planner-schemas"

                [dependencies]
                serde = "1"
            "#,
        )
        .unwrap();
        std::fs::write(
            dir.join("planner-server/Cargo.toml"),
            r#"
                [package]
                name = "planner-server"

                [dependencies]
                planner-core = { path = "../planner-core" }
                planner-schemas = { path = "../planner-schemas" }
                axum = "0.8"
            "#,
        )
        .unwrap();
        std::fs::write(
            dir.join("planner-web/package.json"),
            r#"
                {
                  "name": "planner-web",
                  "dependencies": {
                    "react": "^19.0.0"
                  }
                }
            "#,
        )
        .unwrap();

        let indexed = vec![
            "planner-core/src/lib.rs".to_string(),
            "planner-server/src/main.rs".to_string(),
            "planner-schemas/src/lib.rs".to_string(),
            "planner-web/src/main.tsx".to_string(),
        ];

        let packages = collect_workspace_packages(&dir, &indexed).unwrap();
        assert!(packages
            .iter()
            .any(|pkg| pkg.name == "planner-core" && pkg.origin_key.starts_with("path:")));
        assert!(packages
            .iter()
            .any(|pkg| pkg.name == "planner-server" && pkg.origin_key.starts_with("path:")));
        assert!(packages
            .iter()
            .any(|pkg| pkg.name == "planner-web" && pkg.origin_key.starts_with("path:")));

        let server = packages
            .iter()
            .find(|pkg| pkg.name == "planner-server")
            .unwrap();
        assert!(server
            .local_dependencies
            .contains(&"planner-core".to_string()));
        assert!(server
            .local_dependencies
            .contains(&"planner-schemas".to_string()));
        assert!(server.external_dependencies.contains(&"axum".to_string()));

        let web = packages
            .iter()
            .find(|pkg| pkg.name == "planner-web")
            .unwrap();
        assert!(web.external_dependencies.contains(&"react".to_string()));

        let _ = std::fs::remove_dir_all(dir);
    }
}
