use ratatui::widgets::TableState;

use planner_core::blueprint::BlueprintStore;
use planner_schemas::artifacts::blueprint::{BlueprintNode, Edge, NodeSummary};

const FILTER_ORDER: [&str; 6] = [
    "decision",
    "technology",
    "component",
    "constraint",
    "pattern",
    "quality_requirement",
];

pub struct BlueprintTableState {
    pub nodes: Vec<NodeSummary>,
    pub edges: Vec<Edge>,
    pub selected: usize,
    pub filter: String,
    pub type_filter: Option<String>,
    pub table_state: TableState,
    pub detail_expanded: bool,
    pub detail_node: Option<BlueprintNode>,
    pub search_mode: bool,
}

impl Default for BlueprintTableState {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            selected: 0,
            filter: String::new(),
            type_filter: None,
            table_state: TableState::default(),
            detail_expanded: false,
            detail_node: None,
            search_mode: false,
        }
    }
}

impl BlueprintTableState {
    pub fn filtered_nodes(&self) -> Vec<&NodeSummary> {
        let query = self.filter.trim().to_lowercase();

        self.nodes
            .iter()
            .filter(|node| {
                self.type_filter
                    .as_deref()
                    .map(|filter| node.node_type == filter)
                    .unwrap_or(true)
            })
            .filter(|node| {
                query.is_empty()
                    || node.name.to_lowercase().contains(&query)
                    || node.id.as_str().to_lowercase().contains(&query)
                    || node
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query))
            })
            .collect()
    }

    pub fn selected_node_id(&self) -> Option<String> {
        self.filtered_nodes()
            .get(self.selected)
            .map(|node| node.id.as_str().to_string())
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.refresh_selection();
    }

    pub fn move_down(&mut self) {
        let len = self.filtered_nodes().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected + 1 < len {
            self.selected += 1;
        }
        self.refresh_selection();
    }

    pub fn jump_top(&mut self) {
        self.selected = 0;
        self.refresh_selection();
    }

    pub fn jump_bottom(&mut self) {
        let len = self.filtered_nodes().len();
        self.selected = len.saturating_sub(1);
        self.refresh_selection();
    }

    pub fn cycle_type_filter(&mut self) {
        self.type_filter = match self.type_filter.as_deref() {
            None => Some(FILTER_ORDER[0].to_string()),
            Some(current) => FILTER_ORDER
                .iter()
                .position(|candidate| *candidate == current)
                .and_then(|index| FILTER_ORDER.get(index + 1))
                .map(|candidate| candidate.to_string()),
        };
        self.selected = 0;
        self.refresh_selection();
    }

    pub fn clear_search(&mut self) {
        self.filter.clear();
        self.search_mode = false;
        self.selected = 0;
        self.refresh_selection();
    }

    pub fn push_filter_char(&mut self, ch: char) {
        self.filter.push(ch);
        self.selected = 0;
        self.refresh_selection();
    }

    pub fn pop_filter_char(&mut self) {
        self.filter.pop();
        self.selected = 0;
        self.refresh_selection();
    }

    pub fn load_blueprint(&mut self, store: &BlueprintStore) {
        let snapshot = store.snapshot();
        self.nodes = snapshot.list_summaries();
        self.edges = snapshot.edges;
        self.selected = 0;
        self.refresh_selection();
        self.load_selected_detail(store);
    }

    pub fn load_selected_detail(&mut self, store: &BlueprintStore) {
        self.detail_node = self
            .selected_node_id()
            .and_then(|node_id| store.get_node(&node_id));
    }

    pub fn refresh_selection(&mut self) {
        let len = self.filtered_nodes().len();
        if len == 0 {
            self.selected = 0;
            self.table_state.select(None);
        } else {
            if self.selected >= len {
                self.selected = len - 1;
            }
            self.table_state.select(Some(self.selected));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use planner_schemas::artifacts::blueprint::{
        AdoptionRing, BlueprintNode, Component, ComponentStatus, ComponentType, Decision,
        DecisionStatus, NodeId, NodeScope, NodeSummary, Technology, TechnologyCategory,
    };

    fn test_scope() -> NodeScope {
        NodeScope::default()
    }

    fn sample_decision_node() -> BlueprintNode {
        BlueprintNode::Decision(Decision {
            id: NodeId::from_raw("dec-alpha"),
            title: "Alpha Decision".into(),
            status: DecisionStatus::Accepted,
            context: "Use alpha".into(),
            options: Vec::new(),
            consequences: Vec::new(),
            assumptions: Vec::new(),
            supersedes: None,
            tags: vec!["core".into()],
            documentation: None,
            scope: test_scope(),
            created_at: "2026-03-06T00:00:00Z".into(),
            updated_at: "2026-03-06T00:00:00Z".into(),
        })
    }

    fn sample_technology_node() -> BlueprintNode {
        BlueprintNode::Technology(Technology {
            id: NodeId::from_raw("tech-beta"),
            name: "Beta Tech".into(),
            version: Some("1.0".into()),
            category: TechnologyCategory::Library,
            ring: AdoptionRing::Adopt,
            rationale: "Needed".into(),
            license: None,
            tags: vec!["infra".into()],
            documentation: Some("# docs".into()),
            scope: test_scope(),
            created_at: "2026-03-06T00:00:00Z".into(),
            updated_at: "2026-03-06T00:00:00Z".into(),
        })
    }

    fn sample_component_node() -> BlueprintNode {
        BlueprintNode::Component(Component {
            id: NodeId::from_raw("comp-gamma"),
            name: "Gamma".into(),
            component_type: ComponentType::Module,
            description: "Gamma component".into(),
            provides: Vec::new(),
            consumes: Vec::new(),
            status: ComponentStatus::Planned,
            tags: vec!["feature".into()],
            documentation: None,
            scope: test_scope(),
            created_at: "2026-03-06T00:00:00Z".into(),
            updated_at: "2026-03-06T00:00:00Z".into(),
        })
    }

    fn sample_nodes() -> Vec<NodeSummary> {
        vec![
            NodeSummary::from(&sample_decision_node()),
            NodeSummary::from(&sample_technology_node()),
        ]
    }

    #[test]
    fn filtered_nodes_returns_all_when_no_filter() {
        let state = BlueprintTableState {
            nodes: sample_nodes(),
            ..BlueprintTableState::default()
        };

        assert_eq!(state.filtered_nodes().len(), 2);
    }

    #[test]
    fn filtered_nodes_filters_by_search_string() {
        let state = BlueprintTableState {
            nodes: sample_nodes(),
            filter: "beta".into(),
            ..BlueprintTableState::default()
        };

        assert_eq!(state.filtered_nodes().len(), 1);
        assert_eq!(state.filtered_nodes()[0].name, "Beta Tech");
    }

    #[test]
    fn filtered_nodes_filters_by_type() {
        let state = BlueprintTableState {
            nodes: sample_nodes(),
            type_filter: Some("decision".into()),
            ..BlueprintTableState::default()
        };

        assert_eq!(state.filtered_nodes().len(), 1);
        assert_eq!(state.filtered_nodes()[0].node_type, "decision");
    }

    #[test]
    fn navigation_wraps_at_boundaries() {
        let mut state = BlueprintTableState {
            nodes: sample_nodes(),
            ..BlueprintTableState::default()
        };

        state.move_up();
        assert_eq!(state.selected, 0);

        state.move_down();
        state.move_down();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn type_filter_cycles_through_all_types() {
        let mut state = BlueprintTableState::default();

        for expected in FILTER_ORDER {
            state.cycle_type_filter();
            assert_eq!(state.type_filter.as_deref(), Some(expected));
        }

        state.cycle_type_filter();
        assert!(state.type_filter.is_none());
    }

    #[test]
    fn load_blueprint_populates_state() {
        let store = BlueprintStore::new();
        store.upsert_node(sample_decision_node());
        store.upsert_node(sample_technology_node());
        store.upsert_node(sample_component_node());

        let mut state = BlueprintTableState::default();
        state.load_blueprint(&store);

        assert_eq!(state.nodes.len(), 3);
        assert!(state.detail_node.is_some());
    }
}
