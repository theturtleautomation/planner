//! # Pyramid Summaries — Hierarchical Context Compression
//!
//! For large projects with many CXDB turns, Pyramid Summaries provide
//! a tiered compression scheme that the DCC (Dynamic Context Compiler)
//! uses to route Consequence Cards and build Context Packs efficiently.
//!
//! ## Architecture
//!
//! ```text
//! Root (1 page summary of entire project)
//!   ├── Branch A (1 paragraph — turns 1-15)
//!   │     ├── Leaf 1 (1-2 sentences — turn 1)
//!   │     ├── Leaf 2 (1-2 sentences — turn 2)
//!   │     └── ...
//!   ├── Branch B (1 paragraph — turns 16-30)
//!   └── ...
//! ```
//!
//! The DCC traverses top-down: Root → relevant Branch → relevant Leaves.

use uuid::Uuid;

use super::steps::StepResult;
use crate::llm::providers::LlmRouter;
use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use planner_schemas::*;

// ---------------------------------------------------------------------------
// Pyramid Builder
// ---------------------------------------------------------------------------

/// Builds a Pyramid Summary tree from a set of turn summaries.
pub struct PyramidBuilder {
    config: PyramidConfig,
}

impl PyramidBuilder {
    pub fn new(config: PyramidConfig) -> Self {
        PyramidBuilder { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(PyramidConfig::default())
    }

    /// Build leaf-tier summaries from raw turn texts.
    /// Each turn gets a 1-2 sentence summary.
    pub fn build_leaves(
        &self,
        project_id: Uuid,
        turns: &[(Uuid, &str)], // (turn_id, turn_text)
    ) -> Vec<PyramidSummaryV1> {
        turns
            .iter()
            .map(|(turn_id, text)| {
                // For leaves, we create a condensed summary
                let summary = Self::truncate_to_summary(text, PyramidTier::Leaf.target_tokens());
                let topics = Self::extract_topics(text);

                PyramidSummaryV1 {
                    project_id,
                    tier: PyramidTier::Leaf,
                    node_id: Uuid::new_v4(),
                    parent_id: None, // Set when branches are built
                    children: vec![],
                    summary,
                    token_count: PyramidTier::Leaf.target_tokens(),
                    covered_turn_ids: vec![*turn_id],
                    topics,
                    refreshed_at: chrono::Utc::now().to_rfc3339(),
                    stale: false,
                }
            })
            .collect()
    }

    /// Group leaves into branches. Each branch covers up to `leaves_per_branch` leaves.
    pub fn build_branches(
        &self,
        project_id: Uuid,
        leaves: &mut [PyramidSummaryV1],
    ) -> Vec<PyramidSummaryV1> {
        let chunk_size = self.config.leaves_per_branch;
        let mut branches = Vec::new();

        for chunk in leaves.chunks_mut(chunk_size) {
            let branch_id = Uuid::new_v4();

            // Aggregate summaries
            let combined: String = chunk
                .iter()
                .map(|leaf| leaf.summary.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            let summary = Self::truncate_to_summary(&combined, PyramidTier::Branch.target_tokens());
            let topics: Vec<String> = chunk
                .iter()
                .flat_map(|leaf| leaf.topics.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let covered_turns: Vec<Uuid> = chunk
                .iter()
                .flat_map(|leaf| leaf.covered_turn_ids.clone())
                .collect();

            let children: Vec<Uuid> = chunk.iter().map(|leaf| leaf.node_id).collect();

            // Set parent on leaves
            for leaf in chunk.iter_mut() {
                leaf.parent_id = Some(branch_id);
            }

            branches.push(PyramidSummaryV1 {
                project_id,
                tier: PyramidTier::Branch,
                node_id: branch_id,
                parent_id: None, // Set when root is built
                children,
                summary,
                token_count: PyramidTier::Branch.target_tokens(),
                covered_turn_ids: covered_turns,
                topics,
                refreshed_at: chrono::Utc::now().to_rfc3339(),
                stale: false,
            });
        }

        branches
    }

    /// Build the root from branches.
    pub fn build_root(
        &self,
        project_id: Uuid,
        branches: &mut [PyramidSummaryV1],
    ) -> PyramidSummaryV1 {
        let root_id = Uuid::new_v4();

        let combined: String = branches
            .iter()
            .map(|b| b.summary.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let summary = Self::truncate_to_summary(&combined, PyramidTier::Root.target_tokens());
        let topics: Vec<String> = branches
            .iter()
            .flat_map(|b| b.topics.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let covered_turns: Vec<Uuid> = branches
            .iter()
            .flat_map(|b| b.covered_turn_ids.clone())
            .collect();

        let children: Vec<Uuid> = branches.iter().map(|b| b.node_id).collect();

        // Set parent on branches
        for branch in branches.iter_mut() {
            branch.parent_id = Some(root_id);
        }

        PyramidSummaryV1 {
            project_id,
            tier: PyramidTier::Root,
            node_id: root_id,
            parent_id: None,
            children,
            summary,
            token_count: PyramidTier::Root.target_tokens(),
            covered_turn_ids: covered_turns,
            topics,
            refreshed_at: chrono::Utc::now().to_rfc3339(),
            stale: false,
        }
    }

    /// Build the complete pyramid from raw turns.
    /// Returns (root, branches, leaves) — all nodes with parent_ids set.
    pub fn build_pyramid(&self, project_id: Uuid, turns: &[(Uuid, &str)]) -> PyramidTree {
        let mut leaves = self.build_leaves(project_id, turns);

        if leaves.len() < self.config.min_turns_for_pyramid {
            // Not enough turns for a full pyramid — return flat
            return PyramidTree {
                root: None,
                branches: vec![],
                leaves,
            };
        }

        let mut branches = self.build_branches(project_id, &mut leaves);
        let root = self.build_root(project_id, &mut branches);

        PyramidTree {
            root: Some(root),
            branches,
            leaves,
        }
    }

    // -- Helpers --

    /// Truncate text to approximately `target_tokens` worth (rough estimate: 4 chars per token).
    fn truncate_to_summary(text: &str, target_tokens: u32) -> String {
        let max_chars = (target_tokens as usize) * 4;
        if text.len() <= max_chars {
            text.to_string()
        } else {
            // Find a sentence boundary near the limit
            let truncated = &text[..max_chars];
            match truncated.rfind(". ") {
                Some(pos) => format!("{}.", &truncated[..pos]),
                None => format!("{}...", truncated),
            }
        }
    }

    /// Extract topic keywords from text.
    fn extract_topics(text: &str) -> Vec<String> {
        // Simple keyword extraction: words that appear to be nouns/concepts
        // (capitalized words, technical terms, etc.)
        let mut topics = Vec::new();
        let lower = text.to_lowercase();

        // Match against known domain keywords
        let domain_keywords = [
            "auth",
            "authentication",
            "payment",
            "stripe",
            "api",
            "database",
            "frontend",
            "backend",
            "user",
            "admin",
            "security",
            "performance",
            "webhook",
            "notification",
            "email",
            "file",
            "upload",
            "search",
            "cache",
            "session",
            "token",
            "jwt",
            "oauth",
            "role",
            "permission",
        ];

        for keyword in &domain_keywords {
            if lower.contains(keyword) {
                topics.push(keyword.to_string());
            }
        }

        topics.dedup();
        topics
    }
}

// ---------------------------------------------------------------------------
// PyramidTree — the complete output
// ---------------------------------------------------------------------------

/// A complete Pyramid Summary tree.
#[derive(Debug)]
pub struct PyramidTree {
    /// Root node (None if not enough turns for a pyramid).
    pub root: Option<PyramidSummaryV1>,
    /// Branch-tier nodes.
    pub branches: Vec<PyramidSummaryV1>,
    /// Leaf-tier nodes.
    pub leaves: Vec<PyramidSummaryV1>,
}

impl PyramidTree {
    /// Total number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        let root_count = if self.root.is_some() { 1 } else { 0 };
        root_count + self.branches.len() + self.leaves.len()
    }

    /// Find branches relevant to a topic.
    pub fn branches_for_topic(&self, topic: &str) -> Vec<&PyramidSummaryV1> {
        self.branches
            .iter()
            .filter(|b| b.topics.iter().any(|t| t == topic))
            .collect()
    }

    /// Find leaves for a specific branch.
    pub fn leaves_for_branch(&self, branch_id: Uuid) -> Vec<&PyramidSummaryV1> {
        self.leaves
            .iter()
            .filter(|l| l.parent_id == Some(branch_id))
            .collect()
    }

    /// Get all node summaries as a flat list (for debugging).
    pub fn all_nodes(&self) -> Vec<&PyramidSummaryV1> {
        let mut nodes: Vec<&PyramidSummaryV1> = Vec::new();
        if let Some(ref root) = self.root {
            nodes.push(root);
        }
        nodes.extend(self.branches.iter());
        nodes.extend(self.leaves.iter());
        nodes
    }
}

// ---------------------------------------------------------------------------
// LLM-enhanced summarization (for production use)
// ---------------------------------------------------------------------------

const LEAF_SUMMARY_PROMPT: &str = r#"Summarize this CXDB turn in 1-2 sentences. Focus on: what decision was made, what artifact was produced, or what state changed. Be concise and factual.

Turn content:
"#;

const BRANCH_SUMMARY_PROMPT: &str = r#"Summarize these grouped turn summaries into a single paragraph. Capture the key themes, decisions, and progress represented by this group. Maximum 200 tokens.

Turn summaries:
"#;

const ROOT_SUMMARY_PROMPT: &str = r#"Summarize this entire project's progress in one page or less. Capture: the project's goal, key decisions made, current state, and major open items. Maximum 800 tokens.

Branch summaries:
"#;

/// LLM-enhanced leaf summarization for production use.
pub async fn summarize_turn_with_llm(router: &LlmRouter, turn_text: &str) -> StepResult<String> {
    let request = CompletionRequest {
        system: None,
        messages: vec![Message {
            role: Role::User,
            content: format!("{}{}", LEAF_SUMMARY_PROMPT, turn_text),
        }],
        max_tokens: 100,
        temperature: 0.1,
        model: DefaultModels::TELEMETRY_PRESENTER.to_string(), // Haiku — fast + cheap
    };

    let response = router.complete(request).await?;
    Ok(response.content)
}

/// LLM-enhanced branch summarization.
pub async fn summarize_branch_with_llm(
    router: &LlmRouter,
    leaf_summaries: &[String],
) -> StepResult<String> {
    let combined = leaf_summaries.join("\n- ");
    let request = CompletionRequest {
        system: None,
        messages: vec![Message {
            role: Role::User,
            content: format!("{}- {}", BRANCH_SUMMARY_PROMPT, combined),
        }],
        max_tokens: 300,
        temperature: 0.1,
        model: DefaultModels::TELEMETRY_PRESENTER.to_string(),
    };

    let response = router.complete(request).await?;
    Ok(response.content)
}

/// LLM-enhanced root summarization.
pub async fn summarize_root_with_llm(
    router: &LlmRouter,
    branch_summaries: &[String],
) -> StepResult<String> {
    let combined = branch_summaries.join("\n\n");
    let request = CompletionRequest {
        system: None,
        messages: vec![Message {
            role: Role::User,
            content: format!("{}{}", ROOT_SUMMARY_PROMPT, combined),
        }],
        max_tokens: 1000,
        temperature: 0.1,
        model: DefaultModels::TELEMETRY_PRESENTER.to_string(),
    };

    let response = router.complete(request).await?;
    Ok(response.content)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_turns(count: usize) -> Vec<(Uuid, String)> {
        (0..count)
            .map(|i| {
                (
                    Uuid::new_v4(),
                    format!(
                        "Turn {} processed authentication token validation for the user session. \
                 The payment flow was updated to include stripe webhook verification.",
                        i,
                    ),
                )
            })
            .collect()
    }

    #[test]
    fn build_leaves_from_turns() {
        let builder = PyramidBuilder::with_defaults();
        let turns = make_turns(5);
        let turn_refs: Vec<(Uuid, &str)> = turns
            .iter()
            .map(|(id, text)| (*id, text.as_str()))
            .collect();

        let leaves = builder.build_leaves(Uuid::new_v4(), &turn_refs);
        assert_eq!(leaves.len(), 5);
        assert!(leaves.iter().all(|l| l.tier == PyramidTier::Leaf));
        assert!(leaves.iter().all(|l| !l.topics.is_empty()));
    }

    #[test]
    fn build_branches_groups_leaves() {
        let builder = PyramidBuilder::new(PyramidConfig {
            leaves_per_branch: 3,
            min_turns_for_pyramid: 5,
            ..Default::default()
        });

        let project_id = Uuid::new_v4();
        let turns = make_turns(9);
        let turn_refs: Vec<(Uuid, &str)> = turns
            .iter()
            .map(|(id, text)| (*id, text.as_str()))
            .collect();

        let mut leaves = builder.build_leaves(project_id, &turn_refs);
        let branches = builder.build_branches(project_id, &mut leaves);

        assert_eq!(branches.len(), 3); // 9 leaves / 3 per branch
        assert!(branches.iter().all(|b| b.tier == PyramidTier::Branch));
        assert!(branches.iter().all(|b| b.children.len() == 3));

        // Leaves should have parent_ids set
        assert!(leaves.iter().all(|l| l.parent_id.is_some()));
    }

    #[test]
    fn build_full_pyramid() {
        let builder = PyramidBuilder::new(PyramidConfig {
            leaves_per_branch: 10,
            min_turns_for_pyramid: 20,
            ..Default::default()
        });

        let project_id = Uuid::new_v4();
        let turns = make_turns(50);
        let turn_refs: Vec<(Uuid, &str)> = turns
            .iter()
            .map(|(id, text)| (*id, text.as_str()))
            .collect();

        let tree = builder.build_pyramid(project_id, &turn_refs);

        assert!(tree.root.is_some());
        assert_eq!(tree.branches.len(), 5); // 50 / 10
        assert_eq!(tree.leaves.len(), 50);
        assert_eq!(tree.node_count(), 56); // 1 root + 5 branches + 50 leaves

        // Root should reference all branches
        let root = tree.root.as_ref().unwrap();
        assert_eq!(root.children.len(), 5);
        assert_eq!(root.tier, PyramidTier::Root);
    }

    #[test]
    fn below_threshold_no_pyramid() {
        let builder = PyramidBuilder::new(PyramidConfig {
            min_turns_for_pyramid: 100,
            ..Default::default()
        });

        let turns = make_turns(10);
        let turn_refs: Vec<(Uuid, &str)> = turns
            .iter()
            .map(|(id, text)| (*id, text.as_str()))
            .collect();

        let tree = builder.build_pyramid(Uuid::new_v4(), &turn_refs);

        assert!(tree.root.is_none());
        assert!(tree.branches.is_empty());
        assert_eq!(tree.leaves.len(), 10);
    }

    #[test]
    fn topic_routing() {
        let builder = PyramidBuilder::new(PyramidConfig {
            leaves_per_branch: 5,
            min_turns_for_pyramid: 10,
            ..Default::default()
        });

        let project_id = Uuid::new_v4();
        let turns = make_turns(20);
        let turn_refs: Vec<(Uuid, &str)> = turns
            .iter()
            .map(|(id, text)| (*id, text.as_str()))
            .collect();

        let tree = builder.build_pyramid(project_id, &turn_refs);

        // All our test turns mention "authentication" and "payment"
        let auth_branches = tree.branches_for_topic("auth");
        assert!(!auth_branches.is_empty());

        let payment_branches = tree.branches_for_topic("payment");
        assert!(!payment_branches.is_empty());
    }

    #[test]
    fn leaf_branch_navigation() {
        let builder = PyramidBuilder::new(PyramidConfig {
            leaves_per_branch: 5,
            min_turns_for_pyramid: 10,
            ..Default::default()
        });

        let turns = make_turns(15);
        let turn_refs: Vec<(Uuid, &str)> = turns
            .iter()
            .map(|(id, text)| (*id, text.as_str()))
            .collect();

        let tree = builder.build_pyramid(Uuid::new_v4(), &turn_refs);
        assert_eq!(tree.branches.len(), 3);

        let first_branch_id = tree.branches[0].node_id;
        let branch_leaves = tree.leaves_for_branch(first_branch_id);
        assert_eq!(branch_leaves.len(), 5);
    }

    #[test]
    fn truncation_finds_sentence_boundary() {
        let text = "First sentence here. Second sentence here. Third sentence that is much longer and would exceed the limit.";
        let truncated = PyramidBuilder::truncate_to_summary(text, 10); // ~40 chars
        assert!(truncated.ends_with('.'));
        assert!(!truncated.contains("Third"));
    }

    #[test]
    fn topic_extraction() {
        let text = "The authentication module handles JWT token validation for user sessions. \
                     Payment processing via Stripe requires webhook security.";
        let topics = PyramidBuilder::extract_topics(text);
        assert!(
            topics.contains(&"auth".to_string()) || topics.contains(&"authentication".to_string())
        );
        assert!(topics.contains(&"token".to_string()));
        assert!(topics.contains(&"stripe".to_string()));
    }

    #[test]
    fn tier_target_tokens() {
        assert_eq!(PyramidTier::Leaf.target_tokens(), 50);
        assert_eq!(PyramidTier::Branch.target_tokens(), 200);
        assert_eq!(PyramidTier::Root.target_tokens(), 800);
    }
}
