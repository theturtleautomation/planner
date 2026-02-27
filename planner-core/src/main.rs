//! # planner-core
//!
//! The Dark Factory engine for Planner v2.
//!
//! This binary crate implements the Phase 0 pipeline:
//! Intake Gateway → Compiler → Factory Diplomat → Scenario Validator →
//! Telemetry Presenter → Live Preview → Git Projection.
//!
//! Usage:
//!   planner-core "Build me a task tracker widget"
//!   planner-core --front-office-only "Build me a countdown timer"
//!   planner-core --full "Build me a task tracker widget"
//!
//! Phase 0: Many types are built for future phases but not yet used in CLI paths.
#![allow(dead_code)]

mod llm;
mod pipeline;
mod storage;

use uuid::Uuid;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("planner_core=info".parse().unwrap()),
        )
        .init();

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();

    let mut front_office_only = false;
    let mut full_mode = false;
    let mut description_parts: Vec<String> = Vec::new();

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--front-office-only" | "--fo" => front_office_only = true,
            "--full" => full_mode = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => description_parts.push(arg.clone()),
        }
    }

    let user_description = description_parts.join(" ");
    if user_description.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    // Default: full pipeline (unless --front-office-only)
    if !front_office_only {
        full_mode = true;
    }

    // Initialize LLM router (checks CLI availability)
    let router = llm::providers::LlmRouter::from_env();
    let available = router.available_providers();
    if available.is_empty() {
        eprintln!("ERROR: No LLM CLI tools found.");
        eprintln!("Install at least one of:");
        eprintln!("  - claude  (Anthropic CLI — Max/Pro subscription)");
        eprintln!("  - gemini  (Google CLI — Gemini Pro subscription)");
        eprintln!("  - codex   (OpenAI CLI — ChatGPT Pro subscription)");
        std::process::exit(1);
    }
    tracing::info!("LLM providers available: {:?}", available);

    let project_id = Uuid::new_v4();
    tracing::info!("Project ID: {}", project_id);
    tracing::info!("User request: {}", user_description);

    if full_mode && !front_office_only {
        // Full pipeline: Front Office → Factory → Validator → Telemetry → Git
        match pipeline::run_phase0_full(&router, project_id, &user_description).await {
            Ok(output) => {
                println!("\n=== Phase 0 Pipeline Complete ===\n");
                println!("Project: {} ({})",
                    output.front_office.intake.project_name,
                    output.front_office.intake.feature_slug);
                println!("Intent:  {}", output.front_office.intake.intent_summary);
                println!();

                println!("Compilation:");
                println!("  NLSpecV1:           {} requirements, {} DoD items",
                    output.front_office.spec.requirements.len(),
                    output.front_office.spec.definition_of_done.len());
                println!("  GraphDotV1:         {} nodes, ${:.2} budget",
                    output.front_office.graph_dot.node_count,
                    output.front_office.graph_dot.run_budget_usd);
                println!("  ScenarioSetV1:      {} scenarios",
                    output.front_office.scenarios.scenarios.len());
                println!();

                println!("Factory:");
                println!("  Build Status:       {:?}", output.factory_output.build_status);
                println!("  Spend:              ${:.2} / ${:.2}",
                    output.budget.current_spend_usd, output.budget.hard_cap_usd);
                println!("  Nodes:              {} completed",
                    output.factory_output.node_results.iter().filter(|n| n.success).count());
                println!();

                println!("Validation:");
                println!("  Gates Passed:       {}", output.satisfaction.gates_passed);
                println!("  Satisfaction:       {}", output.satisfaction.user_message());
                println!();

                println!("Result:");
                println!("  {}", output.telemetry.headline);
                println!("  {}", output.telemetry.summary);
                println!();

                if !output.telemetry.consequence_cards.is_empty() {
                    println!("Consequence Cards ({}):", output.telemetry.consequence_cards.len());
                    for card in &output.telemetry.consequence_cards {
                        println!("  [{:?}] {}", card.trigger, card.problem);
                    }
                    println!();
                }

                println!("Git:");
                println!("  Commit:             {}",
                    &output.git_result.commit.commit_hash[..12.min(output.git_result.commit.commit_hash.len())]);
                println!("  Branch:             {}", output.git_result.commit.branch);
                println!("  Repo:               {}", output.git_result.repo_path);
            }
            Err(e) => {
                eprintln!("\nPipeline failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Front Office only
        match pipeline::run_phase0_front_office(&router, project_id, &user_description).await {
            Ok(output) => {
                println!("\n=== Phase 0 Front Office Complete ===\n");
                println!("Project: {} ({})", output.intake.project_name, output.intake.feature_slug);
                println!("Intent:  {}", output.intake.intent_summary);
                println!();
                println!("Artifacts produced:");
                println!("  IntakeV1:           {} sacred anchors, {} satisfaction seeds",
                    output.intake.sacred_anchors.len(),
                    output.intake.satisfaction_criteria_seeds.len());
                println!("  NLSpecV1:           {} requirements, {} DoD items",
                    output.spec.requirements.len(),
                    output.spec.definition_of_done.len());
                println!("  GraphDotV1:         {} nodes, ${:.2} budget",
                    output.graph_dot.node_count,
                    output.graph_dot.run_budget_usd);
                println!("  ScenarioSetV1:      {} scenarios",
                    output.scenarios.scenarios.len());
                println!("  AgentsManifestV1:   {} bytes",
                    output.agents_manifest.root_agents_md.len());
                println!();
                println!("Next: planner-core --full \"{}\" (to run Factory → Validator → Git)",
                    user_description);
            }
            Err(e) => {
                eprintln!("\nPipeline failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn print_usage() {
    eprintln!("Usage: planner-core [options] <description>");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --full               Run the complete pipeline (default)");
    eprintln!("  --front-office-only  Run only the Front Office (compilation only)");
    eprintln!("  --fo                 Alias for --front-office-only");
    eprintln!("  --help, -h           Show this help");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  planner-core \"Build me a task tracker widget\"");
    eprintln!("  planner-core --full \"Build me a countdown timer\"");
    eprintln!("  planner-core --fo \"Build me a pomodoro timer\"");
    eprintln!();
    eprintln!("planner-core v0.1.0 — Phase 0");
    eprintln!("Pipeline: Intake → Compile → Lint → Factory → Validate → Present → Git");
}
