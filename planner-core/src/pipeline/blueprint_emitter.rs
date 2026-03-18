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

use planner_schemas::{
    artifacts::blueprint::*, ArReportV1, FactoryOutputV1, IntakeV1, NLSpecV1, Requirement,
};

use crate::blueprint::BlueprintStore;
use crate::component_naming::{
    derive_spec_group_key, generate_factory_name, generate_spec_name, merge_generated_component,
    FactoryNamingInput, SpecGroupNamingInput,
};
use crate::knowledge_naming::{concise_constraint_title, concise_quality_label};

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
            scope_review: None,
    }
}

fn project_root_id(project_id: &str) -> NodeId {
    let slug = project_id
        .to_ascii_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-")
        .trim_matches('-')
        .to_string();
    NodeId::from_raw(format!("proj-{}", slug))
}

fn ensure_project_root_node(
    store: &BlueprintStore,
    project_id: &str,
    project_name: Option<&str>,
    description: &str,
    timestamp: &str,
) -> NodeId {
    let id = project_root_id(project_id);
    let scope = NodeScope {
        scope_class: ScopeClass::Project,
        project: Some(ProjectScope {
            project_id: project_id.to_string(),
            project_name: project_name.map(|value| value.to_string()),
        }),
        secondary: SecondaryScopeRefs::default(),
        is_shared: false,
        shared: None,
        lifecycle: NodeLifecycle::Active,
        override_scope: None,
            scope_review: None,
    };

    let node = Project {
        id: id.clone(),
        name: project_name.unwrap_or(project_id).to_string(),
        description: description.to_string(),
        tags: vec!["project-root".into()],
        documentation: None,
        scope,
        created_at: timestamp.to_string(),
        updated_at: timestamp.to_string(),
    };
    store.upsert_node(BlueprintNode::Project(node));
    id
}

fn attach_to_project_root(
    store: &BlueprintStore,
    project_root_id: &NodeId,
    child_id: &NodeId,
    metadata: Option<&str>,
) {
    if project_root_id == child_id {
        return;
    }

    store.add_edge(Edge {
        source: project_root_id.clone(),
        target: child_id.clone(),
        edge_type: EdgeType::Contains,
        metadata: metadata.map(|value| value.to_string()),
    });
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
            scope_review: None,
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
    let project_root_id = ensure_project_root_node(
        store,
        &intake.project_id.to_string(),
        Some(&intake.project_name),
        &intake.intent_summary,
        &ts,
    );

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
    let scope_decision_id = scope_decision.id.clone();
    store.upsert_node(BlueprintNode::Decision(scope_decision));
    attach_to_project_root(
        store,
        &project_root_id,
        &scope_decision_id,
        Some("project scope decision"),
    );

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
    let project_id = spec.project_id.to_string();
    let project_root_id = ensure_project_root_node(
        store,
        &project_id,
        node_scope
            .project
            .as_ref()
            .and_then(|project| project.project_name.as_deref()),
        &format!("Blueprint root for project {}", project_id),
        &ts,
    );
    let mut emitted_constraints: Vec<(NodeId, String)> = Vec::new();
    let mut emitted_technologies: Vec<(NodeId, String, String)> = Vec::new();
    let mut emitted_decisions: Vec<(NodeId, String, String)> = Vec::new();
    let mut emitted_components: Vec<SemanticComponentRef> = Vec::new();
    let mut emitted_quality_requirements: Vec<(NodeId, String)> = Vec::new();

    // --- Architectural Constraints → Constraint nodes ---
    for (i, constraint_text) in spec.architectural_constraints.iter().enumerate() {
        let slug = format!("constraint-{}-{}", chunk_tag, i);
        let node = Constraint {
            id: NodeId::with_prefix("CON", &slug),
            title: concise_constraint_title(constraint_text),
            constraint_type: ConstraintType::Technical,
            description: constraint_text.clone(),
            source: "NLSpec compilation".into(),
            tags: vec!["spec".into(), chunk_tag.clone()],
            documentation: None,
            scope: node_scope.clone(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        };
        let node_id = node.id.clone();
        store.upsert_node(BlueprintNode::Constraint(node));
        attach_to_project_root(
            store,
            &project_root_id,
            &node_id,
            Some("architectural constraint"),
        );
        emitted_constraints.push((node_id, constraint_text.clone()));
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
        let tech_node_id = tech.id.clone();
        store.upsert_node(BlueprintNode::Technology(tech));
        attach_to_project_root(
            store,
            &project_root_id,
            &tech_node_id,
            Some("external dependency"),
        );
        emitted_technologies.push((
            tech_node_id.clone(),
            dep.name.clone(),
            dep.usage_description.clone(),
        ));

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
        attach_to_project_root(
            store,
            &project_root_id,
            &dec_id,
            Some("dependency decision"),
        );
        emitted_decisions.push((
            dec_id.clone(),
            dep.name.clone(),
            dep.usage_description.clone(),
        ));

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
        let group = extract_requirement_group(req);
        component_groups
            .entry(group)
            .or_default()
            .push(req.statement.clone());
    }

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

        let component = if let Some(existing) =
            store.find_component_by_origin_key(&generated.naming.origin_key)
        {
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
        let component_id = component.id.clone();
        store.upsert_node(BlueprintNode::Component(component));
        attach_to_project_root(
            store,
            &project_root_id,
            &component_id,
            Some("requirement group"),
        );
        emitted_components.push(SemanticComponentRef {
            id: component_id,
            name: generated.name.clone(),
            group_token: group_name.clone(),
            statements: statements.clone(),
        });
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
            label: Some(concise_quality_label(
                &criterion.description,
                &QualityAttribute::Reliability,
                &["satisfaction".to_string(), chunk_tag.clone()],
            )),
            scenario: criterion.description.clone(),
            priority,
            tags: vec!["satisfaction".into(), chunk_tag.clone()],
            documentation: None,
            scope: node_scope.clone(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        };
        let qr_id = qr.id.clone();
        store.upsert_node(BlueprintNode::QualityRequirement(qr));
        attach_to_project_root(
            store,
            &project_root_id,
            &qr_id,
            Some("satisfaction criterion"),
        );
        emitted_quality_requirements.push((qr_id, criterion.description.clone()));
    }

    // --- Definition of Done → QualityRequirement nodes (mechanically checkable) ---
    for (i, dod) in spec.definition_of_done.iter().enumerate() {
        if dod.mechanically_checkable {
            let qr = QualityRequirement {
                id: NodeId::with_prefix("QR", &format!("dod-{}", i)),
                attribute: QualityAttribute::Maintainability,
                label: Some(concise_quality_label(
                    &dod.criterion,
                    &QualityAttribute::Maintainability,
                    &["definition-of-done".to_string(), chunk_tag.clone()],
                )),
                scenario: dod.criterion.clone(),
                priority: QualityPriority::Medium,
                tags: vec!["definition-of-done".into(), chunk_tag.clone()],
                documentation: None,
                scope: node_scope.clone(),
                created_at: ts.clone(),
                updated_at: ts.clone(),
            };
            let qr_id = qr.id.clone();
            store.upsert_node(BlueprintNode::QualityRequirement(qr));
            attach_to_project_root(store, &project_root_id, &qr_id, Some("definition of done"));
            emitted_quality_requirements.push((qr_id, dod.criterion.clone()));
        }
    }

    emit_semantic_spec_edges(
        store,
        &emitted_constraints,
        &emitted_technologies,
        &emitted_decisions,
        &emitted_components,
        &emitted_quality_requirements,
    );

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
        let project_id = report.project_id.to_string();
        let project_root_id = ensure_project_root_node(
            store,
            &project_id,
            None,
            &format!("Blueprint root for project {}", project_id),
            &ts,
        );
        for finding in &report.findings {
            if finding.severity == planner_schemas::ArSeverity::Blocking {
                blocking_count += 1;
                let node = Constraint {
                    id: NodeId::with_prefix("CON", &format!("ar-{}", finding.id)),
                    title: concise_constraint_title(&finding.description),
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
                    scope: NodeScope {
                        scope_class: ScopeClass::Project,
                        project: Some(ProjectScope {
                            project_id: project_id.clone(),
                            project_name: None,
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
                };
                let node_id = node.id.clone();
                store.upsert_node(BlueprintNode::Constraint(node));
                attach_to_project_root(
                    store,
                    &project_root_id,
                    &node_id,
                    Some("adversarial review finding"),
                );
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
pub fn emit_from_factory(
    store: &BlueprintStore,
    output: &FactoryOutputV1,
    project_id: Option<&str>,
    project_name: Option<&str>,
) {
    let ts = now();
    let project_root_id = project_id.map(|project_id| {
        ensure_project_root_node(
            store,
            project_id,
            project_name,
            &format!("Blueprint root for project {}", project_id),
            &ts,
        )
    });
    let factory_scope = if let Some(project_id) = project_id {
        NodeScope {
            scope_class: ScopeClass::Project,
            project: Some(ProjectScope {
                project_id: project_id.to_string(),
                project_name: project_name.map(|value| value.to_string()),
            }),
            secondary: SecondaryScopeRefs::default(),
            is_shared: false,
            shared: None,
            lifecycle: NodeLifecycle::Active,
            override_scope: None,
            scope_review: None,
        }
    } else {
        NodeScope::default()
    };

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
        scope: factory_scope.clone(),
        created_at: ts.clone(),
        updated_at: ts.clone(),
    };
    let pattern_id = pattern.id.clone();
    store.upsert_node(BlueprintNode::Pattern(pattern));
    if let Some(project_root_id) = &project_root_id {
        attach_to_project_root(
            store,
            project_root_id,
            &pattern_id,
            Some("factory execution pattern"),
        );
    }

    // Component: generated output workspace (project-specific instead of generic "Factory Output")
    let generated_name = generate_factory_name(FactoryNamingInput {
        output_path: &output.output_path,
        project_name,
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
        merged.scope = factory_scope.clone();
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
            scope: factory_scope.clone(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
        }
    };
    let output_component_id = output_component.id.clone();
    store.upsert_node(BlueprintNode::Component(output_component));
    if let Some(project_root_id) = &project_root_id {
        attach_to_project_root(
            store,
            project_root_id,
            &output_component_id,
            Some("factory output workspace"),
        );
    }

    let (node_count, _) = store.counts();
    tracing::info!(
        "Blueprint: emitted factory nodes → {} total nodes",
        node_count
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct SemanticComponentRef {
    id: NodeId,
    name: String,
    group_token: String,
    statements: Vec<String>,
}

fn emit_semantic_spec_edges(
    store: &BlueprintStore,
    constraints: &[(NodeId, String)],
    technologies: &[(NodeId, String, String)],
    decisions: &[(NodeId, String, String)],
    components: &[SemanticComponentRef],
    quality_requirements: &[(NodeId, String)],
) {
    for component in components {
        for (technology_id, technology_name, usage_description) in technologies {
            if component_mentions_technology(component, technology_name, usage_description) {
                store.add_edge(Edge {
                    source: component.id.clone(),
                    target: technology_id.clone(),
                    edge_type: EdgeType::Uses,
                    metadata: Some("derived from requirement/dependency text".into()),
                });
            }
        }
    }

    for (constraint_id, constraint_text) in constraints {
        for (technology_id, technology_name, _) in technologies {
            if text_mentions_phrase(constraint_text, technology_name) {
                store.add_edge(Edge {
                    source: constraint_id.clone(),
                    target: technology_id.clone(),
                    edge_type: EdgeType::Constrains,
                    metadata: Some("constraint names the technology directly".into()),
                });
            }
        }

        for component in components {
            if component_constraint_score(component, constraint_text) >= 2 {
                store.add_edge(Edge {
                    source: constraint_id.clone(),
                    target: component.id.clone(),
                    edge_type: EdgeType::Constrains,
                    metadata: Some("constraint overlaps component concern".into()),
                });
            }
        }
    }

    for (qr_id, qr_text) in quality_requirements {
        if let Some(component) = best_matching_component(components, qr_text) {
            store.add_edge(Edge {
                source: component.id.clone(),
                target: qr_id.clone(),
                edge_type: EdgeType::Satisfies,
                metadata: Some("quality requirement aligned to component concern".into()),
            });
            continue;
        }

        if let Some((decision_id, _, _)) = decisions
            .iter()
            .filter(|(_, decision_name, usage_description)| {
                semantic_overlap_score(qr_text, decision_name)
                    + semantic_overlap_score(qr_text, usage_description)
                    >= 2
            })
            .max_by_key(|(_, decision_name, usage_description)| {
                semantic_overlap_score(qr_text, decision_name)
                    + semantic_overlap_score(qr_text, usage_description)
            })
        {
            store.add_edge(Edge {
                source: decision_id.clone(),
                target: qr_id.clone(),
                edge_type: EdgeType::Satisfies,
                metadata: Some("quality requirement aligned to dependency decision".into()),
            });
        }
    }
}

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
/// Numeric root requirements are regrouped semantically from the statement.
fn extract_requirement_group(req: &Requirement) -> String {
    let parts: Vec<&str> = req.id.split('-').collect();
    if parts.len() >= 2 {
        // Skip "FR" prefix, take the domain part when it carries semantic meaning.
        let candidate = parts[1].trim().to_ascii_lowercase();
        let candidate_is_numeric =
            !candidate.is_empty() && candidate.chars().all(|ch| ch.is_ascii_digit() || ch == '_');
        if !candidate.is_empty()
            && !candidate_is_numeric
            && !matches!(candidate.as_str(), "root" | "general")
        {
            return candidate;
        }
    } else {
        return derive_spec_group_key("general", std::slice::from_ref(&req.statement));
    }

    derive_spec_group_key(
        parts.last().copied().unwrap_or("general"),
        std::slice::from_ref(&req.statement),
    )
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

fn component_mentions_technology(
    component: &SemanticComponentRef,
    technology_name: &str,
    usage_description: &str,
) -> bool {
    let technology_overlap = component_texts(component)
        .into_iter()
        .any(|text| text_mentions_phrase(&text, technology_name));
    if technology_overlap {
        return true;
    }

    component_texts(component)
        .into_iter()
        .map(|text| semantic_overlap_score(&text, usage_description))
        .max()
        .unwrap_or(0)
        >= 2
}

fn best_matching_component<'a>(
    components: &'a [SemanticComponentRef],
    text: &str,
) -> Option<&'a SemanticComponentRef> {
    components
        .iter()
        .filter_map(|component| {
            let score = component_quality_score(component, text);
            (score >= 2).then_some((component, score))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(component, _)| component)
}

fn component_constraint_score(component: &SemanticComponentRef, text: &str) -> usize {
    let base = component_quality_score(component, text);
    if text_mentions_phrase(text, &component.group_token) {
        base.max(2)
    } else {
        base
    }
}

fn component_quality_score(component: &SemanticComponentRef, text: &str) -> usize {
    let mut score = semantic_overlap_score(text, &component.name);
    score += semantic_overlap_score(text, &component.group_token);

    for keyword in keywords_for_group_token(&component.group_token) {
        if text_mentions_phrase(text, keyword) {
            score += 2;
        }
    }

    for statement in &component.statements {
        score = score.max(semantic_overlap_score(text, statement));
    }

    score
}

fn component_texts(component: &SemanticComponentRef) -> Vec<String> {
    let mut texts = vec![component.name.clone(), component.group_token.clone()];
    texts.extend(component.statements.iter().cloned());
    texts
}

fn semantic_overlap_score(left: &str, right: &str) -> usize {
    let left_tokens = normalized_tokens(left);
    let right_tokens = normalized_tokens(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0;
    }

    left_tokens
        .iter()
        .filter(|token| right_tokens.contains(*token))
        .count()
}

fn text_mentions_phrase(text: &str, phrase: &str) -> bool {
    let normalized_text = normalize_for_match(text);
    let normalized_phrase = normalize_for_match(phrase);
    !normalized_phrase.is_empty() && normalized_text.contains(&normalized_phrase)
}

fn normalized_tokens(text: &str) -> std::collections::BTreeSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|token| {
            let normalized = normalize_token(token);
            if normalized.len() < 3 || is_noise_token(&normalized) {
                None
            } else {
                Some(normalized)
            }
        })
        .collect()
}

fn normalize_for_match(text: &str) -> String {
    normalized_tokens(text)
        .into_iter()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_token(token: &str) -> String {
    let mut normalized = token
        .trim()
        .trim_matches(|ch: char| ch.is_ascii_digit())
        .to_ascii_lowercase();
    while normalized
        .chars()
        .last()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        normalized.pop();
    }
    if normalized.ends_with("ies") && normalized.len() > 4 {
        normalized.truncate(normalized.len() - 3);
        normalized.push('y');
    } else if normalized.ends_with('s') && normalized.len() > 4 {
        normalized.truncate(normalized.len() - 1);
    }
    normalized
}

fn is_noise_token(token: &str) -> bool {
    matches!(
        token,
        "the"
            | "and"
            | "for"
            | "with"
            | "from"
            | "into"
            | "must"
            | "should"
            | "will"
            | "that"
            | "this"
            | "can"
            | "all"
            | "use"
            | "user"
            | "users"
            | "system"
            | "via"
            | "only"
            | "mode"
    )
}

fn keywords_for_group_token(group_token: &str) -> &'static [&'static str] {
    match group_token {
        "auth" => &[
            "auth", "login", "signin", "oauth", "jwt", "session", "identity",
        ],
        "api" => &["api", "endpoint", "json", "rest", "http", "route"],
        "review" => &["review", "drag", "drop", "reorder", "edit", "delete"],
        "task" => &["task", "todo", "list", "tracker"],
        "storage" | "store" => &["storage", "persist", "database", "cache", "localstorage"],
        _ => &[],
    }
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
        assert_eq!(
            extract_requirement_group(&Requirement {
                id: "FR-AUTH-001".into(),
                statement: "Authentication must succeed".into(),
                priority: planner_schemas::Priority::Must,
                traces_to: Vec::new(),
            }),
            "auth"
        );
        assert_eq!(
            extract_requirement_group(&Requirement {
                id: "FR-API-003".into(),
                statement: "API returns JSON".into(),
                priority: planner_schemas::Priority::Must,
                traces_to: Vec::new(),
            }),
            "api"
        );
        assert_eq!(
            extract_requirement_group(&Requirement {
                id: "FR-001".into(),
                statement: "The system must provide a text input".into(),
                priority: planner_schemas::Priority::Must,
                traces_to: Vec::new(),
            }),
            "input"
        );
        assert_eq!(
            extract_requirement_group(&Requirement {
                id: "FR".into(),
                statement: "The system must persist task ordering".into(),
                priority: planner_schemas::Priority::Must,
                traces_to: Vec::new(),
            }),
            "persistence"
        );
    }

    #[test]
    fn merge_component_tags_adds_required_without_duplicates() {
        let merged =
            merge_component_tags(&["spec".into(), "core".into()], &["spec", "root", "core"]);
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
    fn emit_from_spec_derives_semantic_relationships() {
        use planner_schemas::*;

        let store = BlueprintStore::new();
        let spec = NLSpecV1 {
            project_id: uuid::Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 24,
            created_from: "test".into(),
            intent_summary: Some("Build review tooling with login".into()),
            sacred_anchors: None,
            requirements: vec![
                Requirement {
                    id: "FR-AUTH-001".into(),
                    statement: "Users can sign in with OAuth".into(),
                    priority: Priority::Must,
                    traces_to: vec![],
                },
                Requirement {
                    id: "FR-REVIEW-001".into(),
                    statement: "Users can drag and drop tasks in review mode".into(),
                    priority: Priority::Must,
                    traces_to: vec![],
                },
            ],
            architectural_constraints: vec![
                "Authentication must use Auth0".into(),
                "Review mode must support drag and drop".into(),
            ],
            phase1_contracts: None,
            external_dependencies: vec![
                ExternalDependency {
                    name: "Auth0".into(),
                    dtu_priority: DtuPriority::High,
                    usage_description: "OAuth2 identity provider".into(),
                },
                ExternalDependency {
                    name: "@dnd-kit/core".into(),
                    dtu_priority: DtuPriority::Medium,
                    usage_description: "Drag and drop interaction library".into(),
                },
            ],
            definition_of_done: vec![DoDItem {
                criterion: "Review tasks can be reordered with drag and drop".into(),
                mechanically_checkable: true,
            }],
            satisfaction_criteria: vec![SatisfactionCriterion {
                id: "SC-LOGIN".into(),
                description: "Login flow completes in under 2s".into(),
                tier_hint: ScenarioTierHint::Critical,
            }],
            open_questions: vec![],
            out_of_scope: vec![],
            amendment_log: vec![],
        };

        emit_from_spec(&store, &spec);

        let snapshot = store.snapshot();
        let auth_component = snapshot
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::Component(component)
                    if component
                        .naming
                        .as_ref()
                        .is_some_and(|naming| naming.origin_key.ends_with(":auth")) =>
                {
                    Some(component.id.clone())
                }
                _ => None,
            })
            .expect("auth component should exist");
        let review_component = snapshot
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::Component(component)
                    if component
                        .naming
                        .as_ref()
                        .is_some_and(|naming| naming.origin_key.ends_with(":review")) =>
                {
                    Some(component.id.clone())
                }
                _ => None,
            })
            .expect("review component should exist");
        let auth0_technology = snapshot
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::Technology(technology) if technology.name == "Auth0" => {
                    Some(technology.id.clone())
                }
                _ => None,
            })
            .expect("Auth0 technology should exist");
        let dnd_technology = snapshot
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::Technology(technology) if technology.name == "@dnd-kit/core" => {
                    Some(technology.id.clone())
                }
                _ => None,
            })
            .expect("dnd-kit technology should exist");
        let login_qr = snapshot
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::QualityRequirement(qr) if qr.scenario.contains("Login flow") => {
                    Some(qr.id.clone())
                }
                _ => None,
            })
            .expect("login quality requirement should exist");
        let reorder_qr = snapshot
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::QualityRequirement(qr) if qr.scenario.contains("drag and drop") => {
                    Some(qr.id.clone())
                }
                _ => None,
            })
            .expect("review quality requirement should exist");

        assert!(snapshot.edges.iter().any(|edge| {
            edge.source == auth_component
                && edge.target == auth0_technology
                && edge.edge_type == EdgeType::Uses
        }));
        assert!(snapshot.edges.iter().any(|edge| {
            edge.source == review_component
                && edge.target == dnd_technology
                && edge.edge_type == EdgeType::Uses
        }));
        assert!(snapshot.edges.iter().any(|edge| {
            edge.target == auth0_technology && edge.edge_type == EdgeType::Constrains
        }));
        assert!(snapshot.edges.iter().any(|edge| {
            edge.source == auth_component
                && edge.target == login_qr
                && edge.edge_type == EdgeType::Satisfies
        }));
        assert!(snapshot.edges.iter().any(|edge| {
            edge.source == review_component
                && edge.target == reorder_qr
                && edge.edge_type == EdgeType::Satisfies
        }));
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

    #[test]
    fn emit_from_factory_uses_project_identity_for_scope_and_name() {
        use planner_schemas::*;

        let store = BlueprintStore::new();
        let output = FactoryOutputV1 {
            kilroy_run_id: uuid::Uuid::new_v4(),
            nlspec_version: "1.0".into(),
            attempt: 1,
            build_status: BuildStatus::Success,
            spend_usd: 1.25,
            checkpoint_path: "/tmp/checkpoint.json".into(),
            dod_results: vec![],
            node_results: vec![],
            output_path: "/opt/planner/data/worktrees/f6873403-1e41-46fb-8414-b61b90df9003".into(),
        };

        emit_from_factory(&store, &output, Some("task-widget"), Some("Task Widget"));

        let snapshot = store.snapshot();
        let factory_component = snapshot
            .nodes
            .values()
            .find_map(|node| match node {
                BlueprintNode::Component(component)
                    if component.naming.as_ref().is_some_and(|naming| {
                        naming.strategy == ComponentNamingStrategy::FactoryOutput
                    }) =>
                {
                    Some(component.clone())
                }
                _ => None,
            })
            .expect("factory component should exist");

        assert_eq!(factory_component.name, "Task Widget Generated Workspace");
        assert_eq!(
            factory_component
                .scope
                .project
                .as_ref()
                .and_then(|project| project.project_name.as_deref()),
            Some("Task Widget")
        );
        assert!(snapshot.edges.iter().any(|edge| {
            edge.edge_type == EdgeType::Contains
                && edge.target == factory_component.id
                && edge.source.as_str() == "proj-task-widget"
        }));
    }
}
