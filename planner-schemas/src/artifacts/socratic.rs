//! # Socratic Intake Types
//!
//! Type definitions for the 6-component Socratic elicitation engine:
//!
//! 1. **Domain Classifier** — `DomainClassification`, `ProjectType`, `ComplexityTier`
//! 2. **Belief State** — `RequirementsBeliefState`, `Dimension`, `SlotValue`, `Contradiction`
//! 3. **Question Planner** — `QuestionStrategy`, `QuestionOutput`
//! 4. **Speculative Draft** — `SpeculativeDraft`, `DraftReaction`
//! 5. **Convergence Decider** — `ConvergenceResult`, `StoppingReason`
//! 6. **Interviewer Constitution** — `InterviewerConstitution`, `ConstitutionRule`
//!
//! Plus the `SocraticEvent` enum for streaming updates to TUI/Web.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::turn::ArtifactPayload;

// ===========================================================================
// Component 1: Domain Classifier + Complexity Scorer
// ===========================================================================

/// Project type classification — determines question templates and depth.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    CliTool,
    WebApp,
    ApiBackend,
    DataPipeline,
    MobileApp,
    LibraryCrate,
    Hybrid,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::CliTool => write!(f, "CLI Tool"),
            ProjectType::WebApp => write!(f, "Web App"),
            ProjectType::ApiBackend => write!(f, "API/Backend"),
            ProjectType::DataPipeline => write!(f, "Data Pipeline"),
            ProjectType::MobileApp => write!(f, "Mobile App"),
            ProjectType::LibraryCrate => write!(f, "Library/Crate"),
            ProjectType::Hybrid => write!(f, "Hybrid"),
        }
    }
}

/// Complexity tier — determines interview depth.
///
/// Derived from Adaptive-RAG patterns:
/// - **Light**: CLI, script, prototype → shallow interview
/// - **Standard**: Web app, API, multi-user → standard interview
/// - **Deep**: Distributed, regulated, multi-tenant → thorough interview
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplexityTier {
    Light,
    Standard,
    Deep,
}

impl ComplexityTier {
    /// Confidence threshold — dimensions below this are considered too uncertain.
    pub fn confidence_threshold(&self) -> f32 {
        match self {
            ComplexityTier::Light => 0.5,
            ComplexityTier::Standard => 0.6,
            ComplexityTier::Deep => 0.7,
        }
    }
}

/// Output of the domain classifier — produced on first user message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainClassification {
    /// Detected project type.
    pub project_type: ProjectType,
    /// Estimated complexity tier.
    pub complexity: ComplexityTier,
    /// Signals that drove the classification (for transparency).
    pub detected_signals: Vec<String>,
    /// Required dimensions for this project type.
    pub required_dimensions: Vec<Dimension>,
}

impl ArtifactPayload for DomainClassification {
    const TYPE_ID: &'static str = "planner.domain_classification.v1";
}

// ===========================================================================
// Component 2: Belief State
// ===========================================================================

/// Requirement dimensions — the axes along which requirements are tracked.
///
/// Taxonomy derived from Volere, ISO 29148, and domain-specific patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    // -- Project Drivers --
    /// The "why" — goal, purpose, motivation.
    Goal,
    /// What does "done" look like? Measurable success criteria.
    SuccessCriteria,
    /// Who are the users/stakeholders?
    Stakeholders,
    /// Business context — why now, what problem exists.
    BusinessContext,

    // -- Functional --
    /// Core features and capabilities.
    CoreFeatures,
    /// User flows and interaction patterns.
    UserFlows,
    /// Data model and storage.
    DataModel,
    /// External integrations and APIs.
    Integrations,
    /// Authentication and authorization model.
    Auth,
    /// Error handling strategy.
    ErrorHandling,

    // -- Quality Attributes (NFRs) --
    /// Performance targets (latency, throughput, concurrent users).
    Performance,
    /// Availability and uptime requirements.
    Availability,
    /// Security requirements and threat model.
    Security,
    /// Scalability constraints and growth expectations.
    Scalability,
    /// Usability and accessibility requirements.
    Usability,

    // -- Constraints --
    /// Required or preferred technology stack.
    TechStack,
    /// Timeline and deadline constraints.
    Timeline,
    /// Budget constraints (compute, API costs, hosting).
    Budget,
    /// Regulatory and compliance requirements.
    Regulatory,
    /// Target platform(s).
    Platform,

    // -- Scope --
    /// Explicit in-scope items.
    InScope,
    /// Explicit out-of-scope items.
    OutOfScope,
    /// Future phases and deferred work.
    FuturePhases,

    // -- Domain-Specific (loaded per project type) --
    /// Custom dimension — name stored in the string.
    Custom(String),
}

impl Dimension {
    /// Human-readable label for display.
    pub fn label(&self) -> String {
        match self {
            Dimension::Goal => "Goal / Purpose".into(),
            Dimension::SuccessCriteria => "Success Criteria".into(),
            Dimension::Stakeholders => "Stakeholders".into(),
            Dimension::BusinessContext => "Business Context".into(),
            Dimension::CoreFeatures => "Core Features".into(),
            Dimension::UserFlows => "User Flows".into(),
            Dimension::DataModel => "Data Model".into(),
            Dimension::Integrations => "Integrations".into(),
            Dimension::Auth => "Auth".into(),
            Dimension::ErrorHandling => "Error Handling".into(),
            Dimension::Performance => "Performance".into(),
            Dimension::Availability => "Availability".into(),
            Dimension::Security => "Security".into(),
            Dimension::Scalability => "Scalability".into(),
            Dimension::Usability => "Usability".into(),
            Dimension::TechStack => "Tech Stack".into(),
            Dimension::Timeline => "Timeline".into(),
            Dimension::Budget => "Budget".into(),
            Dimension::Regulatory => "Regulatory".into(),
            Dimension::Platform => "Platform".into(),
            Dimension::InScope => "In Scope".into(),
            Dimension::OutOfScope => "Out of Scope".into(),
            Dimension::FuturePhases => "Future Phases".into(),
            Dimension::Custom(name) => name.clone(),
        }
    }

    /// Priority weight for question planning (higher = ask sooner).
    /// Core functional dimensions are weighted higher than NFRs.
    pub fn priority_weight(&self) -> f32 {
        match self {
            // Project drivers — highest priority (the "why")
            Dimension::Goal => 1.0,
            Dimension::SuccessCriteria => 0.95,
            Dimension::Stakeholders => 0.85,
            Dimension::BusinessContext => 0.9,

            // Functional — high priority
            Dimension::CoreFeatures => 0.95,
            Dimension::UserFlows => 0.8,
            Dimension::DataModel => 0.75,
            Dimension::Integrations => 0.7,
            Dimension::Auth => 0.7,
            Dimension::ErrorHandling => 0.6,

            // Quality attributes — medium priority
            Dimension::Performance => 0.5,
            Dimension::Availability => 0.4,
            Dimension::Security => 0.6,
            Dimension::Scalability => 0.4,
            Dimension::Usability => 0.5,

            // Constraints — medium
            Dimension::TechStack => 0.65,
            Dimension::Timeline => 0.55,
            Dimension::Budget => 0.3,
            Dimension::Regulatory => 0.5,
            Dimension::Platform => 0.6,

            // Scope — important for boundary-setting
            Dimension::InScope => 0.7,
            Dimension::OutOfScope => 0.65,
            Dimension::FuturePhases => 0.3,

            // Custom — default medium
            Dimension::Custom(_) => 0.5,
        }
    }

    /// Returns the standard required dimensions for a project type.
    pub fn required_for(project_type: &ProjectType) -> Vec<Dimension> {
        let mut dims = vec![
            // Universal — always required
            Dimension::Goal,
            Dimension::SuccessCriteria,
            Dimension::CoreFeatures,
            Dimension::ErrorHandling,
            Dimension::Security,
            Dimension::OutOfScope,
        ];

        match project_type {
            ProjectType::CliTool => {
                dims.extend_from_slice(&[
                    Dimension::Platform,
                    Dimension::Custom("Exit Codes".into()),
                    Dimension::Custom("Input Formats".into()),
                ]);
            }
            ProjectType::WebApp => {
                dims.extend_from_slice(&[
                    Dimension::Stakeholders,
                    Dimension::UserFlows,
                    Dimension::Auth,
                    Dimension::Performance,
                    Dimension::Usability,
                    Dimension::DataModel,
                    Dimension::Custom("Browser Support".into()),
                ]);
            }
            ProjectType::ApiBackend => {
                dims.extend_from_slice(&[
                    Dimension::Auth,
                    Dimension::Performance,
                    Dimension::DataModel,
                    Dimension::Integrations,
                    Dimension::Scalability,
                    Dimension::Custom("API Style".into()),
                ]);
            }
            ProjectType::DataPipeline => {
                dims.extend_from_slice(&[
                    Dimension::DataModel,
                    Dimension::Performance,
                    Dimension::Scalability,
                    Dimension::Custom("Data Sources".into()),
                    Dimension::Custom("Scheduling".into()),
                    Dimension::Custom("Idempotency".into()),
                ]);
            }
            ProjectType::MobileApp => {
                dims.extend_from_slice(&[
                    Dimension::Stakeholders,
                    Dimension::UserFlows,
                    Dimension::Auth,
                    Dimension::Platform,
                    Dimension::Usability,
                    Dimension::Custom("Offline Support".into()),
                ]);
            }
            ProjectType::LibraryCrate => {
                dims.extend_from_slice(&[
                    Dimension::Platform,
                    Dimension::Custom("Public API Surface".into()),
                    Dimension::Custom("Compatibility".into()),
                    Dimension::Custom("Documentation".into()),
                ]);
            }
            ProjectType::Hybrid => {
                dims.extend_from_slice(&[
                    Dimension::Stakeholders,
                    Dimension::UserFlows,
                    Dimension::Auth,
                    Dimension::DataModel,
                    Dimension::Integrations,
                    Dimension::Performance,
                    Dimension::Platform,
                ]);
            }
        }

        dims
    }
}

/// A typed value for a belief state slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotValue {
    /// The value itself — plain English description.
    pub value: String,
    /// The conversation turn where this was captured.
    pub source_turn: u32,
    /// The exact user quote that produced this value (for traceability).
    pub source_quote: Option<String>,
}

/// A detected contradiction between two filled dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    /// First conflicting dimension.
    pub dimension_a: Dimension,
    /// Value of the first dimension.
    pub value_a: String,
    /// Second conflicting dimension.
    pub dimension_b: Dimension,
    /// Value of the second dimension.
    pub value_b: String,
    /// Explanation of why these conflict.
    pub explanation: String,
    /// Whether this has been resolved.
    pub resolved: bool,
}

/// The central belief state — what the system knows, guesses, and is missing.
///
/// This is the single most important data structure. Updated after every
/// user turn by a Verifier pass (separate LLM call). Persisted to CXDB
/// (MessagePack on disk) after every update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementsBeliefState {
    /// Filled slots — confirmed by user, high confidence.
    pub filled: HashMap<Dimension, SlotValue>,

    /// Uncertain slots — system has a guess but confidence is low.
    /// Tuple: (guess value, confidence 0.0–1.0).
    pub uncertain: HashMap<Dimension, (SlotValue, f32)>,

    /// Missing — no information yet.
    pub missing: Vec<Dimension>,

    /// Explicitly out of scope.
    pub out_of_scope: Vec<Dimension>,

    /// Detected contradictions.
    pub contradictions: Vec<Contradiction>,

    /// Domain-specific required dimensions (loaded from classifier).
    pub required_dimensions: Vec<Dimension>,

    /// Current turn count.
    pub turn_count: u32,

    /// Domain classification that produced this state.
    pub classification: Option<DomainClassification>,
}

impl RequirementsBeliefState {
    /// Create a new empty belief state from a domain classification.
    pub fn from_classification(classification: &DomainClassification) -> Self {
        Self {
            filled: HashMap::new(),
            uncertain: HashMap::new(),
            missing: classification.required_dimensions.clone(),
            out_of_scope: Vec::new(),
            contradictions: Vec::new(),
            required_dimensions: classification.required_dimensions.clone(),
            turn_count: 0,
            classification: Some(classification.clone()),
        }
    }

    /// Fill a dimension with a confirmed value.
    pub fn fill(&mut self, dimension: Dimension, value: SlotValue) {
        // Remove from missing and uncertain
        self.missing.retain(|d| d != &dimension);
        self.uncertain.remove(&dimension);
        self.filled.insert(dimension, value);
    }

    /// Mark a dimension as uncertain with a guess and confidence.
    pub fn mark_uncertain(&mut self, dimension: Dimension, value: SlotValue, confidence: f32) {
        self.missing.retain(|d| d != &dimension);
        self.uncertain.insert(dimension, (value, confidence));
    }

    /// Mark a dimension as explicitly out of scope.
    pub fn mark_out_of_scope(&mut self, dimension: Dimension) {
        self.missing.retain(|d| d != &dimension);
        self.uncertain.remove(&dimension);
        self.filled.remove(&dimension);
        if !self.out_of_scope.contains(&dimension) {
            self.out_of_scope.push(dimension);
        }
    }

    /// Record a contradiction.
    pub fn add_contradiction(&mut self, contradiction: Contradiction) {
        self.contradictions.push(contradiction);
    }

    /// Percentage of required dimensions that are filled or out-of-scope.
    pub fn convergence_pct(&self) -> f32 {
        if self.required_dimensions.is_empty() {
            return 1.0;
        }
        let resolved = self
            .required_dimensions
            .iter()
            .filter(|d| self.filled.contains_key(d) || self.out_of_scope.contains(d))
            .count();
        resolved as f32 / self.required_dimensions.len() as f32
    }

    /// Count of dimensions in each category.
    pub fn counts(&self) -> BeliefStateCounts {
        BeliefStateCounts {
            filled: self.filled.len(),
            uncertain: self.uncertain.len(),
            missing: self.missing.len(),
            out_of_scope: self.out_of_scope.len(),
            contradictions: self.contradictions.iter().filter(|c| !c.resolved).count(),
        }
    }
}

impl ArtifactPayload for RequirementsBeliefState {
    const TYPE_ID: &'static str = "planner.belief_state.v1";
}

/// Summary counts for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefStateCounts {
    pub filled: usize,
    pub uncertain: usize,
    pub missing: usize,
    pub out_of_scope: usize,
    pub contradictions: usize,
}

// ===========================================================================
// Component 3: Question Planner
// ===========================================================================

/// The strategy for the next question — which dimension to target and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionStrategy {
    /// The dimension this question targets.
    pub target_dimension: Dimension,
    /// Why this dimension was chosen (for transparency).
    pub rationale: String,
    /// Priority score (priority_weight × information_gain).
    pub score: f32,
}

/// Output from the question planner — the actual question to ask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOutput {
    /// The question text to present to the user.
    pub question: String,
    /// The dimension being targeted.
    pub target_dimension: Dimension,
    /// Quick-select options (if applicable).
    pub quick_options: Vec<QuickOption>,
    /// Whether a "Not sure yet" option should be shown.
    pub allow_skip: bool,
}

/// A quick-select answer option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickOption {
    /// Short label for the button/chip.
    pub label: String,
    /// What this option means (for the belief state update).
    pub value: String,
}

// ===========================================================================
// Component 4: Speculative Draft
// ===========================================================================

/// A speculative draft generated from the current belief state.
///
/// Presented to the user for reaction-based elicitation. User reactions
/// to a draft are 2–5× more information-dense than answers to open questions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculativeDraft {
    /// Formatted draft sections.
    pub sections: Vec<DraftSection>,
    /// Assumptions (from uncertain slots) that need validation.
    pub assumptions: Vec<DraftAssumption>,
    /// Dimensions not yet discussed.
    pub not_discussed: Vec<Dimension>,
}

/// A section of the speculative draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSection {
    /// Section heading (e.g., "Goal", "Core Features").
    pub heading: String,
    /// Content of this section.
    pub content: String,
    /// Source dimensions.
    pub dimensions: Vec<Dimension>,
}

/// An assumption from the uncertain slots, presented for validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftAssumption {
    /// The dimension this assumption covers.
    pub dimension: Dimension,
    /// What the system is assuming.
    pub assumption: String,
    /// Confidence level (0.0–1.0).
    pub confidence: f32,
}

/// User reaction to a speculative draft (per-section or whole-draft).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftReaction {
    /// "Yes, that's correct."
    Confirm,
    /// "No, here's what's actually right."
    Correct { correction: String },
    /// "I hadn't thought of that." (Surprise — explore jointly.)
    Surprise { comment: String },
    /// "Completely wrong." (Fundamental misunderstanding.)
    Reject { reason: String },
}

// ===========================================================================
// Component 5: Convergence Decider
// ===========================================================================

/// Result of the convergence check — should we stop or continue?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceResult {
    /// Whether the interview should end.
    pub is_done: bool,
    /// Why (or why not) — for transparency.
    pub reason: StoppingReason,
    /// Overall convergence percentage.
    pub convergence_pct: f32,
}

/// The reason the convergence decider reached its conclusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoppingReason {
    /// All required dimensions filled or out-of-scope.
    CompletenessGate,
    /// All uncertain dimensions above confidence threshold.
    ConfidenceThreshold,
    /// Last N questions produced no new information.
    DiminishingReturns { stale_turns: u32 },
    /// User explicitly said "just build it."
    UserSignal,
    /// Still more to ask — interview continues.
    Continue {
        /// Top remaining priorities.
        next_priorities: Vec<Dimension>,
    },
}

// ===========================================================================
// Component 6: Interviewer Constitution
// ===========================================================================

/// A single constitution rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionRule {
    /// Rule number (for reference).
    pub id: u32,
    /// Category: behavioral, coverage, or process.
    pub category: RuleCategory,
    /// The rule text.
    pub text: String,
}

/// Rule category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleCategory {
    Behavioral,
    Coverage,
    Process,
}

/// The full interviewer constitution — configurable rules layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewerConstitution {
    /// The rules list.
    pub rules: Vec<ConstitutionRule>,
    /// Project-specific overrides or additions.
    pub project_overrides: Vec<ConstitutionRule>,
}

impl InterviewerConstitution {
    /// Load the default constitution.
    pub fn default_constitution() -> Self {
        Self {
            rules: vec![
                // BEHAVIORAL
                ConstitutionRule {
                    id: 1,
                    category: RuleCategory::Behavioral,
                    text: "Never assume a technology stack the user hasn't mentioned.".into(),
                },
                ConstitutionRule {
                    id: 2,
                    category: RuleCategory::Behavioral,
                    text: "Never ask more than one question per turn.".into(),
                },
                ConstitutionRule {
                    id: 3,
                    category: RuleCategory::Behavioral,
                    text: "Always probe for edge cases before moving to the next dimension.".into(),
                },
                ConstitutionRule {
                    id: 4,
                    category: RuleCategory::Behavioral,
                    text: "Never ask about implementation details until functional scope is established.".into(),
                },
                ConstitutionRule {
                    id: 5,
                    category: RuleCategory::Behavioral,
                    text: "If user expertise appears low, prefer concrete examples over abstract terminology.".into(),
                },
                ConstitutionRule {
                    id: 6,
                    category: RuleCategory::Behavioral,
                    text: "Never accept 'it should be fast' — always probe for quantitative targets.".into(),
                },
                // COVERAGE
                ConstitutionRule {
                    id: 7,
                    category: RuleCategory::Coverage,
                    text: "A session is incomplete if security, error handling, and success criteria haven't been addressed.".into(),
                },
                ConstitutionRule {
                    id: 8,
                    category: RuleCategory::Coverage,
                    text: "Never omit stakeholder identification in a multi-user system.".into(),
                },
                ConstitutionRule {
                    id: 9,
                    category: RuleCategory::Coverage,
                    text: "Explicitly confirm scope boundaries (what's NOT being built).".into(),
                },
                // PROCESS
                ConstitutionRule {
                    id: 10,
                    category: RuleCategory::Process,
                    text: "After 3 filled dimensions, offer a speculative draft for validation.".into(),
                },
                ConstitutionRule {
                    id: 11,
                    category: RuleCategory::Process,
                    text: "If user gives a one-word answer, probe deeper — don't accept surface responses.".into(),
                },
                ConstitutionRule {
                    id: 12,
                    category: RuleCategory::Process,
                    text: "If a contradiction is detected, surface it immediately rather than recording both.".into(),
                },
            ],
            project_overrides: Vec::new(),
        }
    }

    /// Get all active rules (base + overrides).
    pub fn all_rules(&self) -> Vec<&ConstitutionRule> {
        self.rules
            .iter()
            .chain(self.project_overrides.iter())
            .collect()
    }

    /// Format rules as a string for inclusion in LLM system prompts.
    pub fn as_prompt_text(&self) -> String {
        let mut text = String::from("INTERVIEWER CONSTITUTION:\n\n");
        for rule in self.all_rules() {
            let category = match rule.category {
                RuleCategory::Behavioral => "BEHAVIORAL",
                RuleCategory::Coverage => "COVERAGE",
                RuleCategory::Process => "PROCESS",
            };
            text.push_str(&format!("{}. [{}] {}\n", rule.id, category, rule.text));
        }
        text
    }
}

// ===========================================================================
// Socratic Event — Streaming Updates to TUI/Web
// ===========================================================================

/// Events emitted by the Socratic engine during an interview session.
///
/// Both TUI and WebSocket consumers use these to update the UI in real-time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SocraticEvent {
    /// Domain classification complete.
    #[serde(rename = "classified")]
    Classified {
        classification: DomainClassification,
    },

    /// Belief state updated after processing user input.
    #[serde(rename = "belief_state_update")]
    BeliefStateUpdate { state: RequirementsBeliefState },

    /// System asks a question.
    #[serde(rename = "question")]
    Question { output: QuestionOutput },

    /// Speculative draft generated — present for review.
    #[serde(rename = "speculative_draft")]
    SpeculativeDraftReady { draft: SpeculativeDraft },

    /// Contradiction detected.
    #[serde(rename = "contradiction")]
    ContradictionDetected { contradiction: Contradiction },

    /// Convergence reached — interview ending.
    #[serde(rename = "converged")]
    Converged { result: ConvergenceResult },

    /// System message (informational, not a question).
    #[serde(rename = "system_message")]
    SystemMessage { content: String },

    /// Error during Socratic processing.
    #[serde(rename = "socratic_error")]
    Error { message: String },
}

// ===========================================================================
// Socratic Session — the full interview state (for CXDB persistence)
// ===========================================================================

/// A complete Socratic interview session, persisted to CXDB.
///
/// This wraps the belief state with conversation history and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocraticSession {
    /// The current belief state.
    pub belief_state: RequirementsBeliefState,
    /// Full conversation log.
    pub conversation: Vec<SocraticTurn>,
    /// The interviewer constitution used for this session.
    pub constitution: InterviewerConstitution,
    /// Whether the interview has concluded.
    pub is_complete: bool,
    /// Final convergence result (set when interview ends).
    pub convergence_result: Option<ConvergenceResult>,
}

impl ArtifactPayload for SocraticSession {
    const TYPE_ID: &'static str = "planner.socratic_session.v1";
}

/// A single turn in the Socratic conversation (richer than IntakeV1's ConversationTurn).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocraticTurn {
    /// Turn number (1-indexed).
    pub turn_number: u32,
    /// Who spoke: user or system.
    pub role: SocraticRole,
    /// What was said.
    pub content: String,
    /// If this was a question, what dimension it targeted.
    pub target_dimension: Option<Dimension>,
    /// If this was a user response, what slots were updated.
    pub slots_updated: Vec<Dimension>,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Role in the Socratic conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SocraticRole {
    User,
    Interviewer,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_labels() {
        assert_eq!(Dimension::Goal.label(), "Goal / Purpose");
        assert_eq!(Dimension::Custom("Exit Codes".into()).label(), "Exit Codes");
    }

    #[test]
    fn complexity_thresholds() {
        assert!((ComplexityTier::Light.confidence_threshold() - 0.5).abs() < f32::EPSILON);
        assert!((ComplexityTier::Standard.confidence_threshold() - 0.6).abs() < f32::EPSILON);
        assert!((ComplexityTier::Deep.confidence_threshold() - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn belief_state_crud() {
        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec!["web".into(), "users".into()],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };

        let mut state = RequirementsBeliefState::from_classification(&classification);
        assert!(!state.missing.is_empty());
        assert!(state.filled.is_empty());

        // Fill a dimension
        state.fill(
            Dimension::Goal,
            SlotValue {
                value: "Task tracking for team".into(),
                source_turn: 1,
                source_quote: Some("I want to build a task tracker".into()),
            },
        );

        assert!(state.filled.contains_key(&Dimension::Goal));
        assert!(!state.missing.contains(&Dimension::Goal));

        // Mark uncertain
        state.mark_uncertain(
            Dimension::Performance,
            SlotValue {
                value: "Sub-200ms responses".into(),
                source_turn: 2,
                source_quote: None,
            },
            0.6,
        );

        assert!(state.uncertain.contains_key(&Dimension::Performance));

        // Mark out of scope
        state.mark_out_of_scope(Dimension::Scalability);
        assert!(state.out_of_scope.contains(&Dimension::Scalability));
        assert!(!state.missing.contains(&Dimension::Scalability));
    }

    #[test]
    fn convergence_pct() {
        let classification = DomainClassification {
            project_type: ProjectType::CliTool,
            complexity: ComplexityTier::Light,
            detected_signals: vec![],
            required_dimensions: vec![
                Dimension::Goal,
                Dimension::CoreFeatures,
                Dimension::OutOfScope,
            ],
        };

        let mut state = RequirementsBeliefState::from_classification(&classification);
        assert_eq!(state.convergence_pct(), 0.0);

        state.fill(
            Dimension::Goal,
            SlotValue {
                value: "Parse CSV files".into(),
                source_turn: 1,
                source_quote: None,
            },
        );
        // 1 of 3 = 33.3%
        assert!((state.convergence_pct() - 0.333).abs() < 0.01);

        state.mark_out_of_scope(Dimension::OutOfScope);
        // 2 of 3 = 66.7%
        assert!((state.convergence_pct() - 0.667).abs() < 0.01);
    }

    #[test]
    fn default_constitution() {
        let constitution = InterviewerConstitution::default_constitution();
        assert_eq!(constitution.rules.len(), 12);

        let prompt = constitution.as_prompt_text();
        assert!(prompt.contains("BEHAVIORAL"));
        assert!(prompt.contains("COVERAGE"));
        assert!(prompt.contains("PROCESS"));
    }

    #[test]
    fn required_dimensions_vary_by_type() {
        let cli_dims = Dimension::required_for(&ProjectType::CliTool);
        let web_dims = Dimension::required_for(&ProjectType::WebApp);

        // CLI should have "Exit Codes", web should not
        assert!(cli_dims.contains(&Dimension::Custom("Exit Codes".into())));
        assert!(!web_dims.contains(&Dimension::Custom("Exit Codes".into())));

        // Web should have Auth, CLI should not
        assert!(web_dims.contains(&Dimension::Auth));
        assert!(!cli_dims.contains(&Dimension::Auth));
    }

    #[test]
    fn socratic_event_serde() {
        let event = SocraticEvent::SystemMessage {
            content: "Let's talk about your project.".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"system_message\""));

        let deserialized: SocraticEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            SocraticEvent::SystemMessage { content } => {
                assert_eq!(content, "Let's talk about your project.");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn belief_state_counts() {
        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec![],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };

        let mut state = RequirementsBeliefState::from_classification(&classification);
        let initial_missing = state.missing.len();

        state.fill(
            Dimension::Goal,
            SlotValue {
                value: "test".into(),
                source_turn: 1,
                source_quote: None,
            },
        );
        state.mark_uncertain(
            Dimension::Performance,
            SlotValue {
                value: "fast".into(),
                source_turn: 2,
                source_quote: None,
            },
            0.5,
        );

        let counts = state.counts();
        assert_eq!(counts.filled, 1);
        assert_eq!(counts.uncertain, 1);
        // Goal moved from missing to filled, Performance moved from missing to uncertain.
        assert_eq!(counts.missing, initial_missing - 2);
    }

    #[test]
    fn draft_reaction_serde() {
        let reaction = DraftReaction::Correct {
            correction: "Actually we need 500 users, not 50".into(),
        };
        let json = serde_json::to_string(&reaction).unwrap();
        let back: DraftReaction = serde_json::from_str(&json).unwrap();
        match back {
            DraftReaction::Correct { correction } => {
                assert!(correction.contains("500 users"));
            }
            _ => panic!("wrong variant"),
        }
    }
}
