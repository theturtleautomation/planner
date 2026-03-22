use std::collections::{HashMap, HashSet};

use planner_schemas::{
    Dimension, PromptItemKind, RequirementsBeliefState, SocraticCategoryNode,
    SocraticCategoryPathEntry, SocraticCategorySnapshot, SocraticCategoryStatus,
    SocraticWorkspaceGroup, SocraticWorkspaceItemPreview, SocraticWorkspaceSnapshot,
};

use super::prompt_batch_planner::{self, PromptCandidate};

const MAX_CATEGORY_CANDIDATES: u32 = 32;
const ROOT_CONTRADICTIONS: &str = "root-contradictions";
const ROOT_VERIFICATION: &str = "root-verification";
const ROOT_DISCOVERY: &str = "root-discovery";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CategoryGroup {
    Contradictions,
    Verification,
    Discovery,
}

#[derive(Debug, Clone)]
enum BuilderNodeKind {
    Root(CategoryGroup),
    Dimension {
        group: CategoryGroup,
        dimension: Dimension,
    },
    ContradictionPeer {
        primary: Dimension,
        secondary: Dimension,
    },
    Leaf(PromptCandidate),
}

#[derive(Debug, Clone)]
struct CategoryNodeSeed {
    category_id: String,
    title: String,
    mapped_dimensions: Vec<Dimension>,
    kind: BuilderNodeKind,
}

#[derive(Debug, Clone)]
struct CategoryNodeBuilder {
    category_id: String,
    parent_category_id: Option<String>,
    title: String,
    depth: u32,
    mapped_dimensions: Vec<Dimension>,
    child_ids: Vec<String>,
    candidate_count: u32,
    kind: BuilderNodeKind,
}

pub fn build_category_snapshot(
    state: &RequirementsBeliefState,
    active_category_ids: &[String],
    build_ready: bool,
    previous_snapshot: Option<&SocraticCategorySnapshot>,
) -> SocraticCategorySnapshot {
    let candidates =
        prompt_batch_planner::select_prompt_candidates(state, None, MAX_CATEGORY_CANDIDATES);
    let mut builders: HashMap<String, CategoryNodeBuilder> = HashMap::new();
    let mut root_ids = Vec::new();

    for group in [
        CategoryGroup::Contradictions,
        CategoryGroup::Verification,
        CategoryGroup::Discovery,
    ] {
        let group_candidates: Vec<PromptCandidate> = candidates
            .iter()
            .filter(|candidate| category_group(candidate) == group)
            .cloned()
            .collect();
        if group_candidates.is_empty() {
            continue;
        }

        let root_id = group_id(group).to_string();
        root_ids.push(root_id.clone());

        for candidate in group_candidates {
            let path = category_path_for_candidate(&candidate);
            insert_candidate_path(&mut builders, &path);
        }
    }

    let sanitized_path = sanitize_active_path(active_category_ids, &root_ids, &builders);
    let visible_parent_id = sanitized_path.last().map(String::as_str);
    let current_visible_ids = visible_ids_from_builders(visible_parent_id, &root_ids, &builders);
    let previous_visible_nodes = previous_snapshot
        .map(|snapshot| relevant_visible_nodes(snapshot, visible_parent_id))
        .unwrap_or_default();
    let previous_visible_ids = previous_visible_nodes
        .iter()
        .map(|node| node.category_id.clone())
        .collect::<HashSet<_>>();
    let newly_available_category_ids = current_visible_ids
        .iter()
        .filter(|category_id| !previous_visible_ids.contains(*category_id))
        .cloned()
        .collect::<Vec<_>>();
    let active_category_path = sanitized_path
        .iter()
        .filter_map(|category_id| builders.get(category_id))
        .map(|node| SocraticCategoryPathEntry {
            category_id: node.category_id.clone(),
            title: node.title.clone(),
        })
        .collect::<Vec<_>>();

    let completed_placeholders =
        completed_placeholder_nodes(previous_visible_nodes, &builders, &current_visible_ids);
    if visible_parent_id.is_none() {
        for node in &completed_placeholders {
            if node.parent_category_id.is_none()
                && !root_ids.iter().any(|root_id| root_id == &node.category_id)
            {
                root_ids.push(node.category_id.clone());
            }
        }
    }

    let nodes = flatten_nodes(
        state,
        &root_ids,
        &sanitized_path,
        &builders,
        &current_visible_ids,
        &completed_placeholders,
    );
    let revision = snapshot_revision(state, &candidates, &active_category_path, build_ready);

    SocraticCategorySnapshot {
        revision,
        root_category_ids: root_ids,
        nodes,
        active_category_path,
        newly_available_category_ids,
        build_ready,
        build_readiness_message: build_readiness_message(state, build_ready),
    }
}

pub fn active_leaf_category_id(snapshot: &SocraticCategorySnapshot) -> Option<&str> {
    let active_id = snapshot.active_category_path.last()?.category_id.as_str();
    snapshot
        .nodes
        .iter()
        .find(|node| node.category_id == active_id && node.has_prompt_ready)
        .map(|node| node.category_id.as_str())
}

pub fn visible_category_ids(snapshot: &SocraticCategorySnapshot) -> Vec<String> {
    let Some(active_id) = snapshot
        .active_category_path
        .last()
        .map(|entry| entry.category_id.as_str())
    else {
        return snapshot.root_category_ids.clone();
    };

    snapshot
        .nodes
        .iter()
        .filter(|node| node.parent_category_id.as_deref() == Some(active_id))
        .map(|node| node.category_id.clone())
        .collect()
}

pub fn resolve_category_path(
    snapshot: &SocraticCategorySnapshot,
    category_id: &str,
) -> Option<Vec<String>> {
    if !visible_category_ids(snapshot)
        .iter()
        .any(|visible_id| visible_id == category_id)
    {
        return None;
    }

    let mut path = Vec::new();
    let mut current_id = Some(category_id.to_string());

    while let Some(node_id) = current_id {
        let node = snapshot
            .nodes
            .iter()
            .find(|node| node.category_id == node_id)?;
        path.push(node.category_id.clone());
        current_id = node.parent_category_id.clone();
    }

    path.reverse();
    Some(path)
}

pub fn filter_candidates_for_active_category(
    state: &RequirementsBeliefState,
    category_id: &str,
    max_visible_items: u32,
) -> Vec<PromptCandidate> {
    prompt_batch_planner::select_prompt_candidates(state, None, MAX_CATEGORY_CANDIDATES)
        .into_iter()
        .filter(|candidate| category_id_for_candidate(candidate) == category_id)
        .take(max_visible_items.max(1) as usize)
        .collect()
}

pub fn build_workspace_snapshot(
    state: &RequirementsBeliefState,
    snapshot: &SocraticCategorySnapshot,
    focused_category_id: Option<&str>,
    branch_notice: Option<String>,
    max_visible_items: u32,
) -> SocraticWorkspaceSnapshot {
    let groups = snapshot
        .nodes
        .iter()
        .filter(|node| node.has_prompt_ready)
        .map(|node| {
            let preview_candidates =
                filter_candidates_for_active_category(state, &node.category_id, max_visible_items);
            let question_count = preview_candidates.len() as u32;
            let preview_items = preview_candidates
                .into_iter()
                .enumerate()
                .map(|(index, candidate)| SocraticWorkspaceItemPreview {
                    item_id: format!("{}::preview::{}", node.category_id, index),
                    kind: candidate.kind,
                    text: candidate.rationale,
                })
                .collect::<Vec<_>>();

            SocraticWorkspaceGroup {
                category_id: node.category_id.clone(),
                title: node.title.clone(),
                summary: node.summary.clone(),
                status: node.status.clone(),
                question_count,
                preview_items,
                is_focused: focused_category_id == Some(node.category_id.as_str()),
                is_new: snapshot
                    .newly_available_category_ids
                    .iter()
                    .any(|category_id| category_id == &node.category_id),
            }
        })
        .collect();

    SocraticWorkspaceSnapshot {
        category_snapshot: snapshot.clone(),
        groups,
        focused_category_id: focused_category_id.map(ToOwned::to_owned),
        branch_notice,
    }
}

fn insert_candidate_path(
    builders: &mut HashMap<String, CategoryNodeBuilder>,
    path: &[CategoryNodeSeed],
) {
    let mut parent_id: Option<String> = None;

    for (depth, seed) in path.iter().enumerate() {
        let depth = depth as u32;
        let category_id = seed.category_id.clone();
        let entry = builders
            .entry(category_id.clone())
            .or_insert_with(|| CategoryNodeBuilder {
                category_id: category_id.clone(),
                parent_category_id: parent_id.clone(),
                title: seed.title.clone(),
                depth,
                mapped_dimensions: seed.mapped_dimensions.clone(),
                child_ids: Vec::new(),
                candidate_count: 0,
                kind: seed.kind.clone(),
            });

        if entry.parent_category_id.is_none() {
            entry.parent_category_id = parent_id.clone();
        }
        entry.depth = depth;
        merge_dimensions(&mut entry.mapped_dimensions, &seed.mapped_dimensions);
        entry.candidate_count = entry.candidate_count.saturating_add(1);

        if let Some(parent_id_ref) = parent_id.as_ref() {
            if let Some(parent) = builders.get_mut(parent_id_ref) {
                if !parent
                    .child_ids
                    .iter()
                    .any(|child_id| child_id == &category_id)
                {
                    parent.child_ids.push(category_id.clone());
                }
            }
        }

        parent_id = Some(category_id);
    }
}

fn sanitize_active_path(
    active_category_ids: &[String],
    root_ids: &[String],
    builders: &HashMap<String, CategoryNodeBuilder>,
) -> Vec<String> {
    if active_category_ids.is_empty() {
        return Vec::new();
    }

    let mut sanitized = Vec::new();
    let mut expected_parent: Option<String> = None;

    for category_id in active_category_ids {
        let Some(node) = builders.get(category_id) else {
            break;
        };

        if expected_parent.is_none() && !root_ids.iter().any(|root_id| root_id == category_id) {
            break;
        }

        if node.parent_category_id != expected_parent {
            break;
        }

        sanitized.push(category_id.clone());
        expected_parent = Some(category_id.clone());
    }

    sanitized
}

fn flatten_nodes(
    state: &RequirementsBeliefState,
    root_ids: &[String],
    active_path: &[String],
    builders: &HashMap<String, CategoryNodeBuilder>,
    current_visible_ids: &[String],
    completed_placeholders: &[SocraticCategoryNode],
) -> Vec<SocraticCategoryNode> {
    let mut ordered_ids = Vec::new();
    for root_id in root_ids {
        if builders.contains_key(root_id) {
            collect_descendants(root_id, builders, &mut ordered_ids);
        }
    }

    let mut nodes = ordered_ids
        .into_iter()
        .filter_map(|category_id| builders.get(&category_id))
        .map(|builder| {
            let has_children = !builder.child_ids.is_empty();
            let has_prompt_ready = matches!(builder.kind, BuilderNodeKind::Leaf(_));
            SocraticCategoryNode {
                category_id: builder.category_id.clone(),
                parent_category_id: builder.parent_category_id.clone(),
                title: builder.title.clone(),
                summary: node_summary(state, builder),
                status: node_status(builder, active_path, current_visible_ids, builders),
                depth: builder.depth,
                mapped_dimensions: builder.mapped_dimensions.clone(),
                has_children,
                has_prompt_ready,
                item_count_hint: if has_children {
                    builder.child_ids.len() as u32
                } else {
                    1
                },
            }
        })
        .collect::<Vec<_>>();
    nodes.extend(completed_placeholders.iter().cloned());
    nodes
}

fn node_status(
    builder: &CategoryNodeBuilder,
    active_path: &[String],
    current_visible_ids: &[String],
    builders: &HashMap<String, CategoryNodeBuilder>,
) -> SocraticCategoryStatus {
    if active_path.last() == Some(&builder.category_id) {
        return SocraticCategoryStatus::Active;
    }

    let visible = current_visible_ids
        .iter()
        .any(|category_id| category_id == &builder.category_id);
    if !visible {
        return SocraticCategoryStatus::Pending;
    }

    if builder.parent_category_id.is_none()
        && active_path.is_empty()
        && is_blocked_root(builder, builders)
    {
        return SocraticCategoryStatus::Blocked;
    }

    SocraticCategoryStatus::Ready
}

fn is_blocked_root(
    builder: &CategoryNodeBuilder,
    builders: &HashMap<String, CategoryNodeBuilder>,
) -> bool {
    let Some(node_group) = builder_group(builder) else {
        return false;
    };
    let highest_group = builders
        .values()
        .filter(|node| node.parent_category_id.is_none())
        .filter_map(builder_group)
        .min_by_key(|group| group_rank(*group));

    matches!(highest_group, Some(highest) if group_rank(node_group) > group_rank(highest))
}

fn builder_group(builder: &CategoryNodeBuilder) -> Option<CategoryGroup> {
    match &builder.kind {
        BuilderNodeKind::Root(group) => Some(*group),
        BuilderNodeKind::Dimension { group, .. } => Some(*group),
        BuilderNodeKind::ContradictionPeer { .. } => Some(CategoryGroup::Contradictions),
        BuilderNodeKind::Leaf(candidate) => Some(category_group(candidate)),
    }
}

fn group_rank(group: CategoryGroup) -> u8 {
    match group {
        CategoryGroup::Contradictions => 0,
        CategoryGroup::Verification => 1,
        CategoryGroup::Discovery => 2,
    }
}

fn visible_ids_from_builders(
    visible_parent_id: Option<&str>,
    root_ids: &[String],
    builders: &HashMap<String, CategoryNodeBuilder>,
) -> Vec<String> {
    match visible_parent_id {
        Some(parent_id) => builders
            .get(parent_id)
            .map(|node| node.child_ids.clone())
            .unwrap_or_default(),
        None => root_ids
            .iter()
            .filter(|root_id| builders.contains_key(*root_id))
            .cloned()
            .collect(),
    }
}

fn relevant_visible_nodes<'a>(
    snapshot: &'a SocraticCategorySnapshot,
    visible_parent_id: Option<&str>,
) -> Vec<&'a SocraticCategoryNode> {
    match visible_parent_id {
        Some(parent_id) => snapshot
            .nodes
            .iter()
            .filter(|node| node.parent_category_id.as_deref() == Some(parent_id))
            .collect(),
        None => snapshot
            .root_category_ids
            .iter()
            .filter_map(|category_id| {
                snapshot
                    .nodes
                    .iter()
                    .find(|node| &node.category_id == category_id)
            })
            .collect(),
    }
}

fn completed_placeholder_nodes(
    previous_visible_nodes: Vec<&SocraticCategoryNode>,
    builders: &HashMap<String, CategoryNodeBuilder>,
    current_visible_ids: &[String],
) -> Vec<SocraticCategoryNode> {
    previous_visible_nodes
        .into_iter()
        .filter(|node| !builders.contains_key(&node.category_id))
        .filter(|node| {
            !current_visible_ids
                .iter()
                .any(|current_id| current_id == &node.category_id)
        })
        .map(|node| SocraticCategoryNode {
            category_id: node.category_id.clone(),
            parent_category_id: node.parent_category_id.clone(),
            title: node.title.clone(),
            summary: format!("Complete: {}", node.summary),
            status: SocraticCategoryStatus::Complete,
            depth: node.depth,
            mapped_dimensions: node.mapped_dimensions.clone(),
            has_children: false,
            has_prompt_ready: false,
            item_count_hint: 0,
        })
        .collect()
}

fn build_readiness_message(state: &RequirementsBeliefState, build_ready: bool) -> String {
    if build_ready {
        return String::from("Build is ready. Further category exploration is optional.");
    }

    let unresolved_conflicts = state
        .contradictions
        .iter()
        .filter(|contradiction| !contradiction.resolved)
        .count();
    if unresolved_conflicts > 0 {
        return format!(
            "Build is blocked by {unresolved_conflicts} unresolved conflict{}.",
            if unresolved_conflicts == 1 { "" } else { "s" }
        );
    }

    if !state.uncertain.is_empty() {
        let uncertain_count = state.uncertain.len();
        return format!(
            "Build is blocked until {uncertain_count} uncertain area{} are verified.",
            if uncertain_count == 1 { "" } else { "s" }
        );
    }

    let required_remaining = state
        .required_dimensions
        .iter()
        .filter(|dimension| !state.filled.contains_key(*dimension))
        .filter(|dimension| !state.out_of_scope.contains(*dimension))
        .count();
    if required_remaining > 0 {
        return format!(
            "Build is blocked until {required_remaining} required area{} are covered.",
            if required_remaining == 1 { "" } else { "s" }
        );
    }

    if !state.missing.is_empty() {
        let missing_count = state.missing.len();
        return format!(
            "Build is blocked until {missing_count} remaining area{} are explored.",
            if missing_count == 1 { "" } else { "s" }
        );
    }

    String::from("Build is still blocked until the remaining intake work is resolved.")
}

fn collect_descendants(
    category_id: &str,
    builders: &HashMap<String, CategoryNodeBuilder>,
    ordered_ids: &mut Vec<String>,
) {
    ordered_ids.push(category_id.to_string());
    if let Some(builder) = builders.get(category_id) {
        for child_id in &builder.child_ids {
            collect_descendants(child_id, builders, ordered_ids);
        }
    }
}

fn category_path_for_candidate(candidate: &PromptCandidate) -> Vec<CategoryNodeSeed> {
    let group = category_group(candidate);
    let root_id = group_id(group).to_string();
    let mut path = vec![CategoryNodeSeed {
        category_id: root_id.clone(),
        title: group_title(group),
        mapped_dimensions: mapped_dimensions(candidate),
        kind: BuilderNodeKind::Root(group),
    }];

    match candidate.kind {
        PromptItemKind::Contradiction => {
            if let Some(contradiction) = candidate.contradiction.as_ref() {
                let primary_id = format!(
                    "{}::dimension::{}",
                    root_id,
                    dimension_id_fragment(&contradiction.dimension_a)
                );
                path.push(CategoryNodeSeed {
                    category_id: primary_id.clone(),
                    title: contradiction.dimension_a.label(),
                    mapped_dimensions: vec![
                        contradiction.dimension_a.clone(),
                        contradiction.dimension_b.clone(),
                    ],
                    kind: BuilderNodeKind::Dimension {
                        group,
                        dimension: contradiction.dimension_a.clone(),
                    },
                });

                let secondary_id = format!(
                    "{}::dimension::{}",
                    primary_id,
                    dimension_id_fragment(&contradiction.dimension_b)
                );
                path.push(CategoryNodeSeed {
                    category_id: secondary_id,
                    title: contradiction.dimension_b.label(),
                    mapped_dimensions: vec![
                        contradiction.dimension_a.clone(),
                        contradiction.dimension_b.clone(),
                    ],
                    kind: BuilderNodeKind::ContradictionPeer {
                        primary: contradiction.dimension_a.clone(),
                        secondary: contradiction.dimension_b.clone(),
                    },
                });
            }
        }
        PromptItemKind::Verification | PromptItemKind::Discovery | PromptItemKind::DraftSection => {
            if let Some(dimension) = candidate.target_dimension.as_ref() {
                let dimension_id = format!(
                    "{}::dimension::{}",
                    root_id,
                    dimension_id_fragment(dimension)
                );
                path.push(CategoryNodeSeed {
                    category_id: dimension_id,
                    title: dimension.label(),
                    mapped_dimensions: vec![dimension.clone()],
                    kind: BuilderNodeKind::Dimension {
                        group,
                        dimension: dimension.clone(),
                    },
                });
            }
        }
    }

    path.push(CategoryNodeSeed {
        category_id: category_id_for_candidate(candidate),
        title: candidate_title(candidate),
        mapped_dimensions: mapped_dimensions(candidate),
        kind: BuilderNodeKind::Leaf(candidate.clone()),
    });

    path
}

fn snapshot_revision(
    state: &RequirementsBeliefState,
    candidates: &[PromptCandidate],
    active_category_path: &[SocraticCategoryPathEntry],
    build_ready: bool,
) -> String {
    let mut identity = format!("turn:{}|build_ready:{}|", state.turn_count, build_ready);
    for entry in active_category_path {
        identity.push_str(entry.category_id.as_str());
        identity.push('|');
    }
    for candidate in candidates {
        identity.push_str(prompt_batch_planner::candidate_identity_key(candidate).as_str());
        identity.push('|');
    }
    let digest = blake3::hash(identity.as_bytes()).to_hex().to_string();
    format!("category-{}-{}", state.turn_count, &digest[..10])
}

fn category_group(candidate: &PromptCandidate) -> CategoryGroup {
    match candidate.kind {
        PromptItemKind::Contradiction => CategoryGroup::Contradictions,
        PromptItemKind::Verification => CategoryGroup::Verification,
        PromptItemKind::Discovery | PromptItemKind::DraftSection => CategoryGroup::Discovery,
    }
}

fn group_id(group: CategoryGroup) -> &'static str {
    match group {
        CategoryGroup::Contradictions => ROOT_CONTRADICTIONS,
        CategoryGroup::Verification => ROOT_VERIFICATION,
        CategoryGroup::Discovery => ROOT_DISCOVERY,
    }
}

fn group_title(group: CategoryGroup) -> String {
    match group {
        CategoryGroup::Contradictions => String::from("Resolve conflicts"),
        CategoryGroup::Verification => String::from("Verify assumptions"),
        CategoryGroup::Discovery => String::from("Explore missing areas"),
    }
}

fn group_summary(group: CategoryGroup, count: u32) -> String {
    match group {
        CategoryGroup::Contradictions => format!(
            "{count} conflict{} need resolution.",
            if count == 1 { "" } else { "s" }
        ),
        CategoryGroup::Verification => format!(
            "{count} uncertain area{} need confirmation.",
            if count == 1 { "" } else { "s" }
        ),
        CategoryGroup::Discovery => format!(
            "{count} area{} still need discovery.",
            if count == 1 { "" } else { "s" }
        ),
    }
}

fn category_id_for_candidate(candidate: &PromptCandidate) -> String {
    format!(
        "category-{}",
        prompt_batch_planner::deterministic_item_id(candidate)
    )
}

fn candidate_title(candidate: &PromptCandidate) -> String {
    match candidate.kind {
        PromptItemKind::Contradiction => {
            if let Some(contradiction) = candidate.contradiction.as_ref() {
                format!(
                    "{} vs {}",
                    contradiction.dimension_a.label(),
                    contradiction.dimension_b.label()
                )
            } else {
                String::from("Resolve contradiction")
            }
        }
        PromptItemKind::Verification => candidate
            .target_dimension
            .as_ref()
            .map(|dimension| format!("Verify {}", dimension.label()))
            .unwrap_or_else(|| String::from("Verify requirement")),
        PromptItemKind::Discovery | PromptItemKind::DraftSection => candidate
            .target_dimension
            .as_ref()
            .map(|dimension| format!("Explore {}", dimension.label()))
            .unwrap_or_else(|| String::from("Explore requirement")),
    }
}

fn candidate_summary(state: &RequirementsBeliefState, candidate: &PromptCandidate) -> String {
    match candidate.kind {
        PromptItemKind::Contradiction => candidate
            .contradiction
            .as_ref()
            .map(|contradiction| contradiction.explanation.clone())
            .unwrap_or_else(|| candidate.rationale.clone()),
        PromptItemKind::Verification => candidate
            .target_dimension
            .as_ref()
            .and_then(|dimension| state.uncertain.get(dimension))
            .map(|(slot, confidence)| {
                format!(
                    "Current assumption: \"{}\" ({:.0}% confidence).",
                    slot.value,
                    confidence * 100.0
                )
            })
            .unwrap_or_else(|| candidate.rationale.clone()),
        PromptItemKind::Discovery | PromptItemKind::DraftSection => candidate.rationale.clone(),
    }
}

fn node_summary(state: &RequirementsBeliefState, builder: &CategoryNodeBuilder) -> String {
    match &builder.kind {
        BuilderNodeKind::Root(group) => group_summary(*group, builder.candidate_count),
        BuilderNodeKind::Dimension { group, dimension } => format!(
            "{} {} branch{} under {}.",
            builder.candidate_count,
            group_branch_noun(*group),
            if builder.candidate_count == 1 {
                ""
            } else {
                "es"
            },
            dimension.label()
        ),
        BuilderNodeKind::ContradictionPeer { primary, secondary } => format!(
            "{} conflict branch{} between {} and {}.",
            builder.candidate_count,
            if builder.candidate_count == 1 {
                ""
            } else {
                "es"
            },
            primary.label(),
            secondary.label()
        ),
        BuilderNodeKind::Leaf(candidate) => candidate_summary(state, candidate),
    }
}

fn group_branch_noun(group: CategoryGroup) -> &'static str {
    match group {
        CategoryGroup::Contradictions => "conflict",
        CategoryGroup::Verification => "verification",
        CategoryGroup::Discovery => "discovery",
    }
}

fn mapped_dimensions(candidate: &PromptCandidate) -> Vec<Dimension> {
    if let Some(contradiction) = candidate.contradiction.as_ref() {
        return vec![
            contradiction.dimension_a.clone(),
            contradiction.dimension_b.clone(),
        ];
    }

    candidate
        .target_dimension
        .as_ref()
        .cloned()
        .into_iter()
        .collect()
}

fn merge_dimensions(existing: &mut Vec<Dimension>, additional: &[Dimension]) {
    let mut seen: HashSet<String> = existing.iter().map(dimension_identity).collect();
    for dimension in additional {
        let identity = dimension_identity(dimension);
        if seen.insert(identity) {
            existing.push(dimension.clone());
        }
    }
}

fn dimension_identity(dimension: &Dimension) -> String {
    serde_json::to_string(dimension).unwrap_or_else(|_| dimension.label())
}

fn dimension_id_fragment(dimension: &Dimension) -> String {
    slugify(dimension_identity(dimension).trim_matches('"'))
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use planner_schemas::{ComplexityTier, Contradiction, DomainClassification, ProjectType};

    use super::*;

    fn make_state() -> RequirementsBeliefState {
        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: Vec::new(),
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };
        RequirementsBeliefState::from_classification(&classification)
    }

    #[test]
    fn snapshot_builds_recursive_category_tree() {
        let mut state = make_state();
        state.missing = vec![Dimension::Goal];
        state.uncertain.clear();
        state.contradictions = vec![Contradiction {
            dimension_a: Dimension::Goal,
            value_a: "Internal planning hub".into(),
            dimension_b: Dimension::Platform,
            value_b: "Mobile-only native app".into(),
            explanation: "The requested planning hub needs desktop collaboration support.".into(),
            resolved: false,
        }];

        let snapshot = build_category_snapshot(&state, &[], false, None);

        assert!(snapshot
            .root_category_ids
            .iter()
            .any(|id| id == ROOT_CONTRADICTIONS));

        let contradiction_dimension = snapshot
            .nodes
            .iter()
            .find(|node| node.parent_category_id.as_deref() == Some(ROOT_CONTRADICTIONS))
            .expect("root contradiction node should have a dimension child");
        let contradiction_peer = snapshot
            .nodes
            .iter()
            .find(|node| {
                node.parent_category_id.as_deref()
                    == Some(contradiction_dimension.category_id.as_str())
            })
            .expect("contradiction dimension should have a peer category child");
        let contradiction_leaf = snapshot
            .nodes
            .iter()
            .find(|node| {
                node.parent_category_id.as_deref() == Some(contradiction_peer.category_id.as_str())
            })
            .expect("contradiction peer should have a prompt-ready leaf child");

        assert!(contradiction_leaf.has_prompt_ready);
        assert_eq!(contradiction_leaf.depth, 3);
    }

    #[test]
    fn snapshot_truncates_stale_deep_path_to_valid_prefix() {
        let mut state = make_state();
        state.missing = vec![Dimension::Goal];
        state.uncertain.clear();

        let snapshot = build_category_snapshot(
            &state,
            &[
                ROOT_DISCOVERY.into(),
                "root-discovery::dimension::goal".into(),
                "missing-leaf".into(),
            ],
            false,
            None,
        );

        assert_eq!(snapshot.active_category_path.len(), 2);
        assert_eq!(snapshot.active_category_path[0].category_id, ROOT_DISCOVERY);
        assert_eq!(
            snapshot.active_category_path[1].category_id,
            "root-discovery::dimension::goal"
        );
    }

    #[test]
    fn visible_category_ids_follow_active_path_depth() {
        let mut state = make_state();
        state.missing = vec![Dimension::Goal];
        state.uncertain.clear();

        let root_snapshot = build_category_snapshot(&state, &[], false, None);
        assert_eq!(
            visible_category_ids(&root_snapshot),
            root_snapshot.root_category_ids
        );

        let branch_snapshot = build_category_snapshot(
            &state,
            &[
                ROOT_DISCOVERY.into(),
                "root-discovery::dimension::goal".into(),
            ],
            false,
            None,
        );
        let visible = visible_category_ids(&branch_snapshot);
        assert_eq!(visible.len(), 1);
        assert!(visible[0].starts_with("category-"));
    }

    #[test]
    fn resolve_category_path_returns_full_recursive_path() {
        let mut state = make_state();
        state.missing = vec![Dimension::Goal];
        state.uncertain.clear();

        let snapshot = build_category_snapshot(&state, &[ROOT_DISCOVERY.into()], false, None);
        let visible = visible_category_ids(&snapshot);
        let path =
            resolve_category_path(&snapshot, &visible[0]).expect("visible child should resolve");

        assert_eq!(path, vec![ROOT_DISCOVERY.to_string(), visible[0].clone()]);
    }

    #[test]
    fn snapshot_marks_newly_visible_categories_and_build_guidance() {
        let mut previous_state = make_state();
        previous_state.missing = vec![Dimension::Goal];
        previous_state.uncertain.clear();

        let previous_snapshot =
            build_category_snapshot(&previous_state, &[ROOT_DISCOVERY.into()], false, None);

        let mut next_state = make_state();
        next_state.missing = vec![Dimension::Goal, Dimension::Timeline];
        next_state.uncertain.clear();

        let next_snapshot = build_category_snapshot(
            &next_state,
            &[ROOT_DISCOVERY.into()],
            false,
            Some(&previous_snapshot),
        );

        assert!(next_snapshot
            .newly_available_category_ids
            .iter()
            .any(|category_id| category_id.contains("timeline")));
        assert!(
            next_snapshot
                .build_readiness_message
                .contains("required area")
                || next_snapshot
                    .build_readiness_message
                    .contains("remaining area")
        );
    }

    #[test]
    fn snapshot_marks_lower_priority_roots_blocked_and_missing_previous_visible_complete() {
        let previous_snapshot = SocraticCategorySnapshot {
            revision: "category-prev".into(),
            root_category_ids: vec![
                "root-contradictions".into(),
                "root-verification".into(),
                "root-discovery".into(),
            ],
            nodes: vec![
                SocraticCategoryNode {
                    category_id: "root-contradictions".into(),
                    parent_category_id: None,
                    title: "Resolve conflicts".into(),
                    summary: "1 conflict needs resolution.".into(),
                    status: SocraticCategoryStatus::Ready,
                    depth: 0,
                    mapped_dimensions: Vec::new(),
                    has_children: true,
                    has_prompt_ready: false,
                    item_count_hint: 1,
                },
                SocraticCategoryNode {
                    category_id: "root-verification".into(),
                    parent_category_id: None,
                    title: "Verify assumptions".into(),
                    summary: "1 uncertain area needs confirmation.".into(),
                    status: SocraticCategoryStatus::Ready,
                    depth: 0,
                    mapped_dimensions: Vec::new(),
                    has_children: true,
                    has_prompt_ready: false,
                    item_count_hint: 1,
                },
                SocraticCategoryNode {
                    category_id: "root-discovery".into(),
                    parent_category_id: None,
                    title: "Explore missing areas".into(),
                    summary: "1 area still needs discovery.".into(),
                    status: SocraticCategoryStatus::Ready,
                    depth: 0,
                    mapped_dimensions: Vec::new(),
                    has_children: true,
                    has_prompt_ready: false,
                    item_count_hint: 1,
                },
            ],
            active_category_path: Vec::new(),
            newly_available_category_ids: Vec::new(),
            build_ready: false,
            build_readiness_message: String::new(),
        };

        let mut contradiction_state = make_state();
        contradiction_state.missing.clear();
        contradiction_state.uncertain.insert(
            Dimension::Security,
            (
                planner_schemas::SlotValue {
                    value: "Basic auth".into(),
                    source_turn: 1,
                    source_quote: None,
                },
                0.4,
            ),
        );
        contradiction_state.contradictions = vec![Contradiction {
            dimension_a: Dimension::Goal,
            value_a: "Internal planning hub".into(),
            dimension_b: Dimension::Platform,
            value_b: "Mobile-only native app".into(),
            explanation: "The requested planning hub needs desktop collaboration support.".into(),
            resolved: false,
        }];

        let snapshot =
            build_category_snapshot(&contradiction_state, &[], false, Some(&previous_snapshot));

        let contradiction_root = snapshot
            .nodes
            .iter()
            .find(|node| node.category_id == ROOT_CONTRADICTIONS)
            .expect("contradiction root should exist");
        let verification_root = snapshot
            .nodes
            .iter()
            .find(|node| node.category_id == ROOT_VERIFICATION)
            .expect("verification root should exist");
        let discovery_root = snapshot
            .nodes
            .iter()
            .find(|node| node.category_id == ROOT_DISCOVERY)
            .expect("discovery root should be preserved as complete");

        assert_eq!(contradiction_root.status, SocraticCategoryStatus::Ready);
        assert_eq!(verification_root.status, SocraticCategoryStatus::Blocked);
        assert_eq!(discovery_root.status, SocraticCategoryStatus::Complete);
    }
}
