//! # Blueprint Emitter — Pipeline → Blueprint Graph Integration
//!
//! Extracts architectural knowledge from pipeline artifacts and emits
//! Blueprint nodes and edges into the BlueprintStore. Called at key
//! pipeline stages to progressively build the Living System Blueprint
//! as the pipeline produces artifacts.
//!
//! ## Emission Points
//!
//! | Pipeline Step     | Emitted Node Types                           |
//! |-------------------|----------------------------------------------|
//! | Compile (NLSpec)  | Decisions, Technologies, Components,         |
//! |                   | Constraints, QualityRequirements             |
//! | AR Review         | (updates existing nodes with review findings) |
//! | Factory Worker    | Components (generated modules)               |
//! | Git Projection    | (no new nodes — metadata update only)        |

use chrono::Utc;

use planner_schemas::{artifacts::blueprint::*, ArReportV1, FactoryOutputV1, IntakeV1, NLSpecV1};

use crate::blueprint::BlueprintStore;
use crate::component_naming::{
    generate_factory_name, generate_spec_name, merge_generated_component, FactoryNamingInput,
    SpecGroupNamingInput,
};

/// Timestamp helper — ISO 8601 UTC.
fn now() -> String {
    Utc::now().to_rfc3339()
}

fn project_scope_from_intake(intake: &IntakeV1) -> NodeScope {
    NodeScope {
        scope_class: ScopeClass::Project,
        project: Some(ProjectScope {
            project_id: intake.project_id.to_string(),
            project_name: Some(intake.project_name.clone()),
        }),
        secondary: SecondaryScopeRefs::default(),
        is_shared: false,
        shared: None,
        lifecycle: NodeLifecycle::Active,
        override_scope: None,
    }
}

fn scope_for_spec(spec: &NLSpecV1) -> NodeScope {
    let mut scope = NodeScope {
        scope_class: ScopeClass::Project,
        project: Some(ProjectScope {
            project_id: spec.project_id.to_string(),
            project_name: None,
        }),
        secondary: SecondaryScopeRefs::default(),
        is_shared: false,
        shared: None,
        lifecycle: NodeLifecycle::Active,
        override_scope: None,
    };

    if let planner_schemas::ChunkType::Domain { name } = &spec.chunk {
        scope.scope_class = ScopeClass::ProjectContextual;
        scope.secondary.feature = Some(name.clone());
    }

    scope
}

// ---------------------------------------------------------------------------
// Stage 1: Intake → Constraints + initial Decisions
// ---------------------------------------------------------------------------

/// Emit Blueprint nodes from IntakeV1.
///
/// Extracts:
/// - A "project scope" Decision node
/// - Constraint nodes from the intake's key constraints (if present)
pub fn emit_from_intake(store: &BlueprintStore, intake: &IntakeV1) {
    let ts = now();
    let scope = project_scope_from_intake(intake);

    // Decision: project scope
    let scope_decision = Decision {
        id: NodeId::with_prefix("DEC", &intake.project_name),
        title: format!("Project scope: {}", intake.project_name),
        status: DecisionStatus::Accepted,
        context: intake.intent_summary.clone(),
        options: vec![DecisionOption {
            name: intake.project_name.clone(),
            pros: vec!["User-specified project direction".into()],
            cons: vec![],
            chosen: true,
        }],
        consequences: vec![],
        assumptions: vec![],
        supersedes: None,
        tags: vec!["intake".into(), "scope".into()],
        documentation: None,
        scope,
        created_at: ts.clone(),
        updated_at: ts.clone(),
    };
    store.upsert_node(BlueprintNode::Decision(scope_decision));

    tracing::info!(
        "Blueprint: emitted intake nodes for '{}'",
        intake.project_name
    );
}

// ---------------------------------------------------------------------------
// Stage 2: Compile (NLSpec) → Decisions, Technologies, Components, Constraints, QRs
// ---------------------------------------------------------------------------

/// Emit Blueprint nodes from a compiled NLSpec.
///
/// This is the richest extraction point — the NLSpec contains:
/// - `architectural_constraints` → Constraint nodes
/// - `external_dependencies` → Technology nodes + "use dependency" Decision nodes
/// - `requirements` → Component nodes (grouped by functional area)
/// - `satisfaction_criteria` → QualityRequirement nodes
/// - `definition_of_done` → QualityRequirement nodes (mechanically checkable)
pub fn emit_from_spec(store: &BlueprintStore, spec: &NLSpecV1) {
    let ts = now();
    let chunk_tag = match &spec.chunk {
        planner_schemas::ChunkType::Root => "root".to_string(),
        planner_schemas::ChunkType::Domain { name } => name.clone(),
    };
    let node_scope = scope_for_spec(spec);

    // --- Architectural Constraints → Constraint nodes ---
    for (i, constraint_text) in spec.architectural_constraints.iter().enumerate() {
        let slug = format!("constraint-{}-{}", chunk_tag, i);
        let node = Constraint {
            id: NodeId::with_prefix("CON", &slug),
            title: truncate(constraint_text, 80),
            constraint_type: ConstraintType::Technical,
            description: constraint_text.clone(),
            source: "NLSpec compilation".into(),
            tags: vec!["spec".into(), chunk_tag.clone()],
            documentation: None,
            scope: node_scope.clone(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        };
        store.upsert_node(BlueprintNode::Constraint(node));
    }

    // --- External Dependencies → Technology nodes + Decision nodes ---
    for dep in &spec.external_dependencies {
        let tech_id = NodeId::with_prefix("TECH", &dep.name);
        let dec_id = NodeId::with_prefix("DEC", &format!("use-{}", dep.name));

        let category = classify_dependency_category(&dep.name);
        let ring = match dep.dtu_priority {
            planner_schemas::DtuPriority::High => AdoptionRing::Adopt,
            planner_schemas::DtuPriority::Medium => AdoptionRing::Trial,
            planner_schemas::DtuPriority::Low => AdoptionRing::Assess,
            planner_schemas::DtuPriority::None => AdoptionRing::Hold,
        };

        let tech = Technology {
            id: tech_id.clone(),
            name: dep.name.clone(),
            version: None,
            category,
            ring,
            rationale: dep.usage_description.clone(),
            license: None,
            tags: vec!["dependency".into(), chunk_tag.clone()],
            documentation: None,
            scope: node_scope.clone(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        };
        store.upsert_node(BlueprintNode::Technology(tech));

        let decision = Decision {
            id: dec_id.clone(),
            title: format!("Use {} as external dependency", dep.name),
            status: DecisionStatus::Accepted,
            context: dep.usage_description.clone(),
            options: vec![DecisionOption {
                name: dep.name.clone(),
                pros: vec![dep.usage_description.clone()],
                cons: vec![],
                chosen: true,
            }],
            consequences: vec![],
            assumptions: vec![],
            supersedes: None,
            tags: vec!["dependency".into(), chunk_tag.clone()],
            documentation: None,
            scope: node_scope.clone(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        };
        store.upsert_node(BlueprintNode::Decision(decision));

        // Edge: Technology → Decision (decided_by)
        store.add_edge(Edge {
            source: tech_id,
            target: dec_id,
            edge_type: EdgeType::DecidedBy,
            metadata: Some("external dependency selection".into()),
        });
    }

    // --- Requirements → Component nodes (grouped by FR prefix) ---
    // Group requirements by their ID prefix (e.g., FR-AUTH-*, FR-API-*, etc.)
    // to create logical Component nodes.
    let mut component_groups: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for req in &spec.requirements {
        let group = extract_requirement_group(&req.id);
        component_groups
            .entry(group)
            .or_default()
            .push(req.statement.clone());
    }

    let project_id = spec.project_id.to_string();
    for (group_name, statements) in &component_groups {
        let generated = generate_spec_name(SpecGroupNamingInput {
            project_id: &project_id,
            project_name: None,
            chunk_tag: &chunk_tag,
            group_token: group_name,
            statements,
            component_type: ComponentType::Module,
            timestamp: &ts,
        });

        let description = format!(
            "{} functional requirements: {}",
            statements.len(),
            statements
                .iter()
                .take(3)
                .map(|s| truncate(s, 60))
                .collect::<Vec<_>>()
                .join("; ")
        );
        let provides: Vec<String> = statements.iter().take(5).map(|s| truncate(s, 80)).collect();

        let component = if let Some(existing) = store.find_component_by_origin_key(&generated.naming.origin_key) {
            let mut merged = merge_generated_component(&existing, &generated);
            merged.component_type = ComponentType::Module;
            merged.description = description;
            merged.provides = provides;
            merged.consumes = Vec::new();
            merged.tags = merge_component_tags(&existing.tags, &["spec", &chunk_tag]);
            merged.documentation = existing.documentation.clone();
            merged.scope = node_scope.clone();
            merged.status = existing.status.clone();
            merged
        } else {
            Component {
                id: NodeId::with_prefix("COMP", group_name),
                name: generated.name.clone(),
                component_type: ComponentType::Module,
                naming: Some(generated.naming.clone()),
                description,
                provides,
                consumes: vec![],
                status: ComponentStatus::Planned,
                tags: vec!["spec".into(), chunk_tag.clone()],
                documentation: None,
                scope: node_scope.clone(),
                created_at: ts.clone(),
                updated_at: ts.clone(),
            }
        };
        store.upsert_node(BlueprintNode::Component(component));
    }

    // --- Satisfaction Criteria → QualityRequirement nodes ---
    for criterion in &spec.satisfaction_criteria {
        let priority = match criterion.tier_hint {
            planner_schemas::ScenarioTierHint::Critical => QualityPriority::Critical,
            planner_schemas::ScenarioTierHint::High => QualityPriority::High,
            planner_schemas::ScenarioTierHint::Medium => QualityPriority::Medium,
        };

        let qr = QualityRequirement {
            id: NodeId::with_prefix("QR", &criterion.id),
            attribute: QualityAttribute::Reliability,
            scenario: criterion.description.clone(),
            priority,
            tags: vec!["satisfaction".into(), chunk_tag.clone()],
            documentation: None,
            scope: node_scope.clone(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        };
        store.upsert_node(BlueprintNode::QualityRequirement(qr));
    }

    // --- Definition of Done → QualityRequirement nodes (mechanically checkable) ---
    for (i, dod) in spec.definition_of_done.iter().enumerate() {
        if dod.mechanically_checkable {
            let qr = QualityRequirement {
                id: NodeId::with_prefix("QR", &format!("dod-{}", i)),
                attribute: QualityAttribute::Maintainability,
                scenario: dod.criterion.clone(),
                priority: QualityPriority::Medium,
                tags: vec!["definition-of-done".into(), chunk_tag.clone()],
                documentation: None,
                scope: node_scope.clone(),
                created_at: ts.clone(),
                updated_at: ts.clone(),
            };
            store.upsert_node(BlueprintNode::QualityRequirement(qr));
        }
    }

    let (node_count, edge_count) = store.counts();
    tracing::info!(
        "Blueprint: emitted spec nodes for chunk '{}' → {} total nodes, {} edges",
        chunk_tag,
        node_count,
        edge_count
    );
}

// ---------------------------------------------------------------------------
// Stage 3: Adversarial Review → update existing nodes with findings
// ---------------------------------------------------------------------------

/// Emit Blueprint updates from AR findings.
///
/// Blocking findings are emitted as Constraint nodes (the AR review
/// identified a constraint the system must satisfy). Advisory findings
/// are logged but not emitted — they're informational.
pub fn emit_from_ar(store: &BlueprintStore, reports: &[ArReportV1]) {
    let ts = now();
    let mut blocking_count = 0u32;

    for report in reports {
        for finding in &report.findings {
            if finding.severity == planner_schemas::ArSeverity::Blocking {
                blocking_count += 1;
                let node = Constraint {
                    id: NodeId::with_prefix("CON", &format!("ar-{}", finding.id)),
                    title: truncate(&finding.description, 80),
                    constraint_type: ConstraintType::Technical,
                    description: format!(
                        "[AR Finding {}] {}\n\nAffected: {}\nResolution: {}",
                        finding.id,
                        finding.description,
                        finding.affected_section,
                        finding.suggested_resolution.as_deref().unwrap_or("none"),
                    ),
                    source: format!("Adversarial Review ({})", report.chunk_name),
                    tags: vec!["ar-review".into(), "blocking".into()],
                    documentation: None,
                    scope: NodeScope::default(),
                    created_at: ts.clone(),
                    updated_at: ts.clone(),
                };
                store.upsert_node(BlueprintNode::Constraint(node));
            }
        }
    }

    if blocking_count > 0 {
        tracing::info!(
            "Blueprint: emitted {} AR blocking constraints",
            blocking_count
        );
    }
}

// ---------------------------------------------------------------------------
// Stage 4: Factory Output → Component nodes for generated artifacts
// ---------------------------------------------------------------------------

/// Emit Blueprint nodes from factory output.
///
/// Each generated file becomes evidence that a Component is now "in progress"
/// or "shipped". We emit a Pattern node for the overall factory approach.
pub fn emit_from_factory(store: &BlueprintStore, output: &FactoryOutputV1) {
    let ts = now();

    // Pattern: factory execution approach
    let status_str = format!("{:?}", output.build_status);
    let pattern = Pattern {
        id: NodeId::with_prefix("PAT", "factory-execution"),
        name: "Dark Factory Code Generation".into(),
        description: format!(
            "Codex-powered code generation. Status: {}. Output: {}",
            status_str, output.output_path,
        ),
        rationale: "Automated code generation with sandbox isolation and validation loop".into(),
        tags: vec!["factory".into(), "codegen".into()],
        documentation: None,
        scope: NodeScope::default(),
        created_at: ts.clone(),
        updated_at: ts.clone(),
    };
    store.upsert_node(BlueprintNode::Pattern(pattern));

    // Component: generated output workspace (project-specific instead of generic "Factory Output")
    let generated_name = generate_factory_name(FactoryNamingInput {
        output_path: &output.output_path,
        project_name: None,
        timestamp: &ts,
    });
    let output_component = if let Some(existing) =
        store.find_component_by_origin_key(&generated_name.naming.origin_key)
    {
        let mut merged = merge_generated_component(&existing, &generated_name);
        merged.component_type = ComponentType::Module;
        merged.description = format!(
            "Generated code output at {}. Build status: {:?}.",
            output.output_path, output.build_status,
        );
        merged.provides = vec!["Generated source code".into()];
        merged.consumes = vec!["NLSpec".into(), "GraphDot".into(), "AGENTS.md".into()];
        merged.status = match output.build_status {
            planner_schemas::BuildStatus::Success => ComponentStatus::Shipped,
            planner_schemas::BuildStatus::PartialSuccess => ComponentStatus::InProgress,
            _ => ComponentStatus::Planned,
        };
        merged.tags = merge_component_tags(&existing.tags, &["factory"]);
        merged.scope = NodeScope::default();
        merged
    } else {
        Component {
            id: NodeId::with_prefix("COMP", "factory-output"),
            name: generated_name.name.clone(),
            component_type: ComponentType::Module,
            naming: Some(generated_name.naming.clone()),
            description: format!(
                "Generated code output at {}. Build status: {:?}.",
                output.output_path, output.build_status,
            ),
            provides: vec!["Generated source code".into()],
            consumes: vec!["NLSpec".into(), "GraphDot".into(), "AGENTS.md".into()],
            status: match output.build_status {
                planner_schemas::BuildStatus::Success => ComponentStatus::Shipped,
                planner_schemas::BuildStatus::PartialSuccess => ComponentStatus::InProgress,
                _ => ComponentStatus::Planned,
            },
            tags: vec!["factory".into()],
            documentation: None,
            scope: NodeScope::default(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }
    };
    store.upsert_node(BlueprintNode::Component(output_component));

    let (node_count, _) = store.counts();
    tracing::info!(
        "Blueprint: emitted factory nodes → {} total nodes",
        node_count
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Truncate a string to `max_len` characters, appending "…" if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.min(s.len())])
    }
}

/// Classify a dependency name into a TechnologyCategory.
fn classify_dependency_category(name: &str) -> TechnologyCategory {
    let lower = name.to_lowercase();
    if lower.contains("react")
        || lower.contains("vue")
        || lower.contains("angular")
        || lower.contains("next")
        || lower.contains("express")
        || lower.contains("django")
        || lower.contains("flask")
        || lower.contains("rails")
        || lower.contains("axum")
        || lower.contains("actix")
    {
        TechnologyCategory::Framework
    } else if lower.contains("stripe")
        || lower.contains("auth0")
        || lower.contains("sendgrid")
        || lower.contains("twilio")
        || lower.contains("supabase")
        || lower.contains("aws")
        || lower.contains("gcp")
        || lower.contains("azure")
        || lower.contains("vercel")
        || lower.contains("heroku")
    {
        TechnologyCategory::Platform
    } else if lower.contains("rust")
        || lower.contains("python")
        || lower.contains("typescript")
        || lower.contains("javascript")
        || lower.contains("go")
        || lower.contains("java")
    {
        TechnologyCategory::Language
    } else if lower.contains("tokio")
        || lower.contains("node")
        || lower.contains("deno")
        || lower.contains("bun")
    {
        TechnologyCategory::Runtime
    } else if lower.contains("http")
        || lower.contains("grpc")
        || lower.contains("graphql")
        || lower.contains("websocket")
        || lower.contains("rest")
    {
        TechnologyCategory::Protocol
    } else {
        TechnologyCategory::Library
    }
}

/// Extract a group name from a requirement ID like "FR-AUTH-001" → "auth".
fn extract_requirement_group(req_id: &str) -> String {
    let parts: Vec<&str> = req_id.split('-').collect();
    if parts.len() >= 2 {
        // Skip "FR" prefix, take the domain part
        parts[1].to_lowercase()
    } else {
        "general".into()
    }
}

fn merge_component_tags(existing: &[String], required: &[&str]) -> Vec<String> {
    let mut merged = existing.to_vec();
    for tag in required {
        if !merged.iter().any(|existing_tag| existing_tag == tag) {
            merged.push((*tag).to_string());
        }
    }
    merged
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let result = truncate("this is a very long string that should be truncated", 20);
        assert!(result.len() <= 24); // 20 chars + "…"
        assert!(result.ends_with('…'));
    }

    #[test]
    fn classify_frameworks() {
        assert_eq!(
            classify_dependency_category("React"),
            TechnologyCategory::Framework
        );
        assert_eq!(
            classify_dependency_category("Axum"),
            TechnologyCategory::Framework
        );
    }

    #[test]
    fn classify_platforms() {
        assert_eq!(
            classify_dependency_category("Stripe"),
            TechnologyCategory::Platform
        );
        assert_eq!(
            classify_dependency_category("Auth0"),
            TechnologyCategory::Platform
        );
        assert_eq!(
            classify_dependency_category("AWS S3"),
            TechnologyCategory::Platform
        );
    }

    #[test]
    fn classify_default_is_library() {
        assert_eq!(
            classify_dependency_category("serde"),
            TechnologyCategory::Library
        );
        assert_eq!(
            classify_dependency_category("rmp-serde"),
            TechnologyCategory::Library
        );
    }

    #[test]
    fn extract_group_from_req_id() {
        assert_eq!(extract_requirement_group("FR-AUTH-001"), "auth");
        assert_eq!(extract_requirement_group("FR-API-003"), "api");
        assert_eq!(extract_requirement_group("FR-001"), "001");
        assert_eq!(extract_requirement_group("FR"), "general");
    }

    #[test]
    fn merge_component_tags_adds_required_without_duplicates() {
        let merged = merge_component_tags(
            &["spec".into(), "core".into()],
            &["spec", "root", "core"],
        );
        assert_eq!(merged, vec!["spec", "core", "root"]);
    }

    #[test]
    fn emit_from_intake_creates_scope_decision() {
        let store = BlueprintStore::new();
        let intake = IntakeV1 {
            project_id: uuid::Uuid::new_v4(),
            project_name: "Test Project".into(),
            feature_slug: "test-project".into(),
            intent_summary: "Build a test app".into(),
            output_domain: planner_schemas::OutputDomain::MicroTool {
                variant: planner_schemas::MicroToolVariant::ReactWidget,
            },
            environment: planner_schemas::EnvironmentInfo {
                language: "TypeScript".into(),
                framework: "React".into(),
                package_manager: Some("npm".into()),
                existing_dependencies: vec![],
                build_tool: Some("vite".into()),
            },
            sacred_anchors: vec![],
            satisfaction_criteria_seeds: vec![],
            out_of_scope: vec![],
            conversation_log: vec![],
        };

        emit_from_intake(&store, &intake);

        let (node_count, _) = store.counts();
        assert!(node_count >= 1, "Should have at least 1 node from intake");

        // Verify the scope decision exists
        let summaries = store.list_by_type("decision");
        assert!(
            !summaries.is_empty(),
            "Should have at least 1 decision node"
        );
        assert!(
            summaries.iter().any(|s| s.name.contains("Test Project")),
            "Should have a decision about the project scope"
        );
    }

    #[test]
    fn emit_from_spec_creates_nodes() {
        use planner_schemas::*;

        let store = BlueprintStore::new();
        let spec = NLSpecV1 {
            project_id: uuid::Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 100,
            created_from: "test".into(),
            intent_summary: Some("Build a web API".into()),
            sacred_anchors: None,
            requirements: vec![
                Requirement {
                    id: "FR-AUTH-001".into(),
                    statement: "The system must authenticate users via JWT".into(),
                    priority: Priority::Must,
                    traces_to: vec![],
                },
                Requirement {
                    id: "FR-AUTH-002".into(),
                    statement: "The system must support OAuth2 login".into(),
                    priority: Priority::Must,
                    traces_to: vec![],
                },
                Requirement {
                    id: "FR-API-001".into(),
                    statement: "The API must serve JSON responses".into(),
                    priority: Priority::Must,
                    traces_to: vec![],
                },
            ],
            architectural_constraints: vec![
                "Must use Rust for the backend".into(),
                "Must deploy to a single binary".into(),
            ],
            phase1_contracts: None,
            external_dependencies: vec![ExternalDependency {
                name: "Auth0".into(),
                dtu_priority: DtuPriority::High,
                usage_description: "OAuth2 identity provider".into(),
            }],
            definition_of_done: vec![DoDItem {
                criterion: "All endpoints return valid JSON".into(),
                mechanically_checkable: true,
            }],
            satisfaction_criteria: vec![SatisfactionCriterion {
                id: "SC-001".into(),
                description: "Login flow completes in under 2s".into(),
                tier_hint: ScenarioTierHint::Critical,
            }],
            open_questions: vec![],
            out_of_scope: vec!["Mobile app".into()],
            amendment_log: vec![],
        };

        emit_from_spec(&store, &spec);

        let (node_count, edge_count) = store.counts();
        assert!(
            node_count >= 5,
            "Expected at least 5 nodes, got {}",
            node_count
        );
        assert!(
            edge_count >= 1,
            "Expected at least 1 edge, got {}",
            edge_count
        );

        // Verify we got constraints
        let constraints = store.list_by_type("constraint");
        assert_eq!(
            constraints.len(),
            2,
            "Should have 2 architectural constraints"
        );

        // Verify we got technology nodes
        let techs = store.list_by_type("technology");
        assert!(
            !techs.is_empty(),
            "Should have technology nodes for dependencies"
        );
        assert!(
            techs.iter().any(|t| t.name == "Auth0"),
            "Should have Auth0 tech node"
        );

        // Verify we got component groups (auth + api = 2)
        let components = store.list_by_type("component");
        assert!(
            components.len() >= 2,
            "Should have at least 2 component groups (auth + api)"
        );
        for component in &components {
            assert!(
                !component.name.ends_with(" Module"),
                "Component name should not end with 'Module': {}",
                component.name
            );
            assert_ne!(component.name, "Api");
            assert_ne!(component.name, "Core");
            assert_ne!(component.name, "Web");
            assert_ne!(component.name, "Factory Output");
        }

        // Verify we got quality requirements
        let qrs = store.list_by_type("quality_requirement");
        assert!(
            qrs.len() >= 1,
            "Should have at least 1 QR from satisfaction criteria"
        );
    }

    #[test]
    fn emit_from_spec_rerun_uses_origin_key_and_preserves_manual_component_names() {
        use planner_schemas::*;

        let store = BlueprintStore::new();
        let spec = NLSpecV1 {
            project_id: uuid::Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 20,
            created_from: "test".into(),
            intent_summary: Some("Build a secure API".into()),
            sacred_anchors: None,
            requirements: vec![
                Requirement {
                    id: "FR-AUTH-001".into(),
                    statement: "Users can sign in with OAuth".into(),
                    priority: Priority::Must,
                    traces_to: vec![],
                },
                Requirement {
                    id: "FR-API-001".into(),
                    statement: "Expose authenticated REST endpoints".into(),
                    priority: Priority::Must,
                    traces_to: vec![],
                },
            ],
            architectural_constraints: vec![],
            phase1_contracts: None,
            external_dependencies: vec![],
            definition_of_done: vec![],
            satisfaction_criteria: vec![],
            open_questions: vec![],
            out_of_scope: vec![],
            amendment_log: vec![],
        };

        emit_from_spec(&store, &spec);
        let initial_components = store
            .snapshot()
            .nodes
            .values()
            .filter_map(|node| match node {
                BlueprintNode::Component(component) => Some(component.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(initial_components.len(), 2);

        let auth_component = initial_components
            .iter()
            .find(|component| {
                component
                    .naming
                    .as_ref()
                    .is_some_and(|naming| naming.origin_key.ends_with(":auth"))
            })
            .expect("auth component should exist");

        let auth_id = auth_component.id.as_str().to_string();
        store.update_node(&auth_id, |node| {
            if let BlueprintNode::Component(component) = node {
                component.name = "Identity Service".into();
                if let Some(naming) = component.naming.as_mut() {
                    naming.source = ComponentNameSource::Manual;
                }
            }
        });

        emit_from_spec(&store, &spec);
        let components_after = store
            .snapshot()
            .nodes
            .values()
            .filter_map(|node| match node {
                BlueprintNode::Component(component) => Some(component.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(components_after.len(), 2);

        let updated_auth = components_after
            .into_iter()
            .find(|component| component.id.as_str() == auth_id)
            .expect("manual component should still exist after rerun");
        assert_eq!(updated_auth.name, "Identity Service");
        assert_eq!(
            updated_auth.naming.map(|naming| naming.source),
            Some(ComponentNameSource::Manual)
        );
    }
}
