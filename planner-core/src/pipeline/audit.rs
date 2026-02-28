//! # Anti-Lock-In Audit Module
//!
//! Analyzes a project's NLSpec for vendor lock-in risks and generates
//! an audit report with recommendations for maintaining portability.
//!
//! ## Risk Categories
//! 1. **Single-vendor dependency**: project relies on a single provider for a critical function
//! 2. **Proprietary API surface**: large API surface without abstraction layer
//! 3. **Data format lock-in**: vendor-specific data formats without export paths
//! 4. **Cost escalation risk**: usage patterns that could lead to pricing surprises
//! 5. **Migration complexity**: how hard would it be to switch providers
//!
//! ## Output
//! Produces an AuditReport with findings and an overall risk score.

use uuid::Uuid;
use serde::{Deserialize, Serialize};

use planner_schemas::*;

// ---------------------------------------------------------------------------
// Audit Types
// ---------------------------------------------------------------------------

/// An anti-lock-in audit report for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInAuditReport {
    /// Which project was audited.
    pub project_id: Uuid,

    /// Overall risk level.
    pub overall_risk: RiskLevel,

    /// Overall score (0.0 = no risk, 1.0 = maximum lock-in).
    pub risk_score: f64,

    /// Individual findings.
    pub findings: Vec<LockInFinding>,

    /// Recommendations for reducing lock-in.
    pub recommendations: Vec<String>,

    /// Dependencies analyzed.
    pub dependencies_audited: usize,
}

/// A specific lock-in finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInFinding {
    /// Finding ID.
    pub id: String,

    /// Which dependency this relates to.
    pub dependency_name: String,

    /// Risk category.
    pub category: LockInCategory,

    /// Severity.
    pub severity: RiskLevel,

    /// Description of the risk.
    pub description: String,

    /// Suggested mitigation.
    pub mitigation: String,
}

/// Lock-in risk categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockInCategory {
    SingleVendor,
    ProprietaryApi,
    DataFormat,
    CostEscalation,
    MigrationComplexity,
}

/// Risk severity levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

// ---------------------------------------------------------------------------
// Known vendor analysis patterns
// ---------------------------------------------------------------------------

/// Known vendor patterns and their lock-in characteristics.
const VENDOR_ANALYSIS: &[VendorPattern] = &[
    VendorPattern {
        name_pattern: "stripe",
        category: "payments",
        has_standard_alternative: true,
        migration_complexity: "medium",
        data_export: true,
        cost_model: "per-transaction",
    },
    VendorPattern {
        name_pattern: "auth0",
        category: "auth",
        has_standard_alternative: true,
        migration_complexity: "high",
        data_export: true,
        cost_model: "per-user",
    },
    VendorPattern {
        name_pattern: "sendgrid",
        category: "email",
        has_standard_alternative: true,
        migration_complexity: "low",
        data_export: false,
        cost_model: "per-email",
    },
    VendorPattern {
        name_pattern: "supabase",
        category: "database",
        has_standard_alternative: true,
        migration_complexity: "medium",
        data_export: true,
        cost_model: "per-row",
    },
    VendorPattern {
        name_pattern: "twilio",
        category: "messaging",
        has_standard_alternative: true,
        migration_complexity: "low",
        data_export: false,
        cost_model: "per-message",
    },
    VendorPattern {
        name_pattern: "firebase",
        category: "database",
        has_standard_alternative: true,
        migration_complexity: "high",
        data_export: true,
        cost_model: "per-read",
    },
    VendorPattern {
        name_pattern: "aws",
        category: "cloud",
        has_standard_alternative: true,
        migration_complexity: "high",
        data_export: true,
        cost_model: "per-resource",
    },
];

struct VendorPattern {
    name_pattern: &'static str,
    category: &'static str,
    #[allow(dead_code)] // Part of lock-in audit report data model — used in future audit report generation
    has_standard_alternative: bool,
    migration_complexity: &'static str,
    data_export: bool,
    cost_model: &'static str,
}

// ---------------------------------------------------------------------------
// Audit Engine
// ---------------------------------------------------------------------------

/// Run an anti-lock-in audit on an NLSpec.
pub fn audit_lock_in(spec: &NLSpecV1) -> LockInAuditReport {
    let mut findings = Vec::new();
    let mut idx = 0u32;

    // Analyze each external dependency
    for dep in &spec.external_dependencies {
        let dep_lower = dep.name.to_lowercase();

        // Find matching vendor pattern
        let vendor = VENDOR_ANALYSIS.iter()
            .find(|v| dep_lower.contains(v.name_pattern));

        if let Some(v) = vendor {
            // Check migration complexity
            if v.migration_complexity == "high" {
                idx += 1;
                findings.push(LockInFinding {
                    id: format!("LOCKIN-{}", idx),
                    dependency_name: dep.name.clone(),
                    category: LockInCategory::MigrationComplexity,
                    severity: RiskLevel::High,
                    description: format!(
                        "{} has high migration complexity in the {} category. \
                         Switching providers would require significant refactoring.",
                        dep.name, v.category,
                    ),
                    mitigation: format!(
                        "Add an abstraction layer (adapter/port pattern) between your code and {}. \
                         Define interfaces in terms of your domain, not the vendor's API.",
                        dep.name,
                    ),
                });
            }

            // Check data export
            if !v.data_export {
                idx += 1;
                findings.push(LockInFinding {
                    id: format!("LOCKIN-{}", idx),
                    dependency_name: dep.name.clone(),
                    category: LockInCategory::DataFormat,
                    severity: RiskLevel::Medium,
                    description: format!(
                        "{} does not provide straightforward data export. \
                         Historical data may be difficult to migrate.",
                        dep.name,
                    ),
                    mitigation: format!(
                        "Maintain a shadow copy of critical data in your own database. \
                         Log all {} interactions for replay if needed.",
                        dep.name,
                    ),
                });
            }

            // Check cost model risks
            if v.cost_model == "per-read" || v.cost_model == "per-resource" {
                idx += 1;
                findings.push(LockInFinding {
                    id: format!("LOCKIN-{}", idx),
                    dependency_name: dep.name.clone(),
                    category: LockInCategory::CostEscalation,
                    severity: RiskLevel::Medium,
                    description: format!(
                        "{} uses a {} pricing model which can escalate unpredictably with usage growth.",
                        dep.name, v.cost_model,
                    ),
                    mitigation: "Set up billing alerts and usage monitoring. Consider caching \
                        strategies to reduce API call volume.".into(),
                });
            }
        }

        // Check if this is a High-priority dep with no DTU (no abstraction)
        if dep.dtu_priority == DtuPriority::High {
            // Check if architectural constraints mention abstraction
            let has_abstraction = spec.architectural_constraints.iter()
                .any(|c| {
                    let lower = c.to_lowercase();
                    (lower.contains("adapter") || lower.contains("port") || lower.contains("abstraction"))
                    && lower.contains(&dep_lower)
                });

            if !has_abstraction {
                idx += 1;
                findings.push(LockInFinding {
                    id: format!("LOCKIN-{}", idx),
                    dependency_name: dep.name.clone(),
                    category: LockInCategory::ProprietaryApi,
                    severity: RiskLevel::Medium,
                    description: format!(
                        "{} is a high-priority dependency with no abstraction layer specified \
                         in architectural constraints.",
                        dep.name,
                    ),
                    mitigation: format!(
                        "Add an architectural constraint requiring a port/adapter pattern for {}. \
                         This enables swapping providers without changing business logic.",
                        dep.name,
                    ),
                });
            }
        }
    }

    // Check for single-vendor categories
    let mut categories: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for dep in &spec.external_dependencies {
        let dep_lower = dep.name.to_lowercase();
        if let Some(v) = VENDOR_ANALYSIS.iter().find(|v| dep_lower.contains(v.name_pattern)) {
            categories.entry(v.category.to_string()).or_default().push(dep.name.clone());
        }
    }

    for (category, deps) in &categories {
        if deps.len() == 1 && spec.external_dependencies.iter()
            .any(|d| d.name == deps[0] && d.dtu_priority == DtuPriority::High) {
            idx += 1;
            findings.push(LockInFinding {
                id: format!("LOCKIN-{}", idx),
                dependency_name: deps[0].clone(),
                category: LockInCategory::SingleVendor,
                severity: RiskLevel::Low,
                description: format!(
                    "{} is the only provider in the '{}' category with high priority. \
                     If this vendor has an outage or changes pricing, there's no fallback.",
                    deps[0], category,
                ),
                mitigation: format!(
                    "Document a contingency plan for {} failure. Consider whether a secondary \
                     provider should be evaluated for the '{}' category.",
                    deps[0], category,
                ),
            });
        }
    }

    // Generate recommendations
    let recommendations = generate_recommendations(&findings);

    // Calculate risk score
    let risk_score = calculate_risk_score(&findings);
    let overall_risk = if risk_score > 0.7 {
        RiskLevel::Critical
    } else if risk_score > 0.5 {
        RiskLevel::High
    } else if risk_score > 0.25 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };

    LockInAuditReport {
        project_id: spec.project_id,
        overall_risk,
        risk_score,
        findings,
        recommendations,
        dependencies_audited: spec.external_dependencies.len(),
    }
}

fn calculate_risk_score(findings: &[LockInFinding]) -> f64 {
    if findings.is_empty() {
        return 0.0;
    }

    let total_weight: f64 = findings.iter().map(|f| match f.severity {
        RiskLevel::Critical => 1.0,
        RiskLevel::High => 0.75,
        RiskLevel::Medium => 0.5,
        RiskLevel::Low => 0.25,
    }).sum();

    // Normalize to 0.0-1.0 range (cap at 4 findings worth of max severity)
    (total_weight / 4.0).min(1.0)
}

fn generate_recommendations(findings: &[LockInFinding]) -> Vec<String> {
    let mut recs = Vec::new();

    let has_no_abstraction = findings.iter().any(|f| f.category == LockInCategory::ProprietaryApi);
    let has_migration_risk = findings.iter().any(|f| f.category == LockInCategory::MigrationComplexity);
    let has_data_risk = findings.iter().any(|f| f.category == LockInCategory::DataFormat);

    if has_no_abstraction {
        recs.push(
            "Implement a port/adapter architecture for external dependencies. \
             Define interfaces in your domain language, not vendor-specific terms.".into()
        );
    }

    if has_migration_risk {
        recs.push(
            "Create a migration playbook documenting how to switch each high-complexity \
             provider. Include estimated effort and data migration steps.".into()
        );
    }

    if has_data_risk {
        recs.push(
            "Implement shadow logging for services without data export. \
             Store copies of critical state transitions in your own database.".into()
        );
    }

    if findings.is_empty() {
        recs.push("No significant lock-in risks identified. Continue monitoring as dependencies evolve.".into());
    }

    recs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spec_with_deps(deps: Vec<ExternalDependency>) -> NLSpecV1 {
        NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: Some("Test".into()),
            sacred_anchors: None,
            requirements: vec![],
            architectural_constraints: vec![],
            phase1_contracts: None,
            external_dependencies: deps,
            definition_of_done: vec![],
            satisfaction_criteria: vec![],
            open_questions: vec![],
            out_of_scope: vec![],
            amendment_log: vec![],
        }
    }

    #[test]
    fn audit_no_deps_low_risk() {
        let spec = make_spec_with_deps(vec![]);
        let report = audit_lock_in(&spec);
        assert_eq!(report.overall_risk, RiskLevel::Low);
        assert_eq!(report.risk_score, 0.0);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn audit_stripe_generates_findings() {
        let spec = make_spec_with_deps(vec![
            ExternalDependency {
                name: "Stripe".into(),
                usage_description: "Payment processing".into(),
                dtu_priority: DtuPriority::High,
            },
        ]);

        let report = audit_lock_in(&spec);
        assert!(!report.findings.is_empty());
        assert_eq!(report.dependencies_audited, 1);

        // Should flag: proprietary API (no abstraction) + single vendor
        assert!(report.findings.iter().any(|f| f.category == LockInCategory::ProprietaryApi));
    }

    #[test]
    fn audit_auth0_flags_migration_complexity() {
        let spec = make_spec_with_deps(vec![
            ExternalDependency {
                name: "Auth0".into(),
                usage_description: "Authentication".into(),
                dtu_priority: DtuPriority::High,
            },
        ]);

        let report = audit_lock_in(&spec);
        assert!(report.findings.iter().any(|f| f.category == LockInCategory::MigrationComplexity));
    }

    #[test]
    fn audit_sendgrid_flags_no_data_export() {
        let spec = make_spec_with_deps(vec![
            ExternalDependency {
                name: "SendGrid".into(),
                usage_description: "Transactional email".into(),
                dtu_priority: DtuPriority::High,
            },
        ]);

        let report = audit_lock_in(&spec);
        assert!(report.findings.iter().any(|f| f.category == LockInCategory::DataFormat));
    }

    #[test]
    fn audit_with_abstraction_constraint_reduces_findings() {
        let mut spec = make_spec_with_deps(vec![
            ExternalDependency {
                name: "Stripe".into(),
                usage_description: "Payments".into(),
                dtu_priority: DtuPriority::High,
            },
        ]);
        spec.architectural_constraints = vec![
            "Use adapter pattern for Stripe integration".into(),
        ];

        let report = audit_lock_in(&spec);
        // Should NOT flag proprietary API because abstraction is specified
        assert!(!report.findings.iter().any(|f|
            f.category == LockInCategory::ProprietaryApi && f.dependency_name == "Stripe"
        ));
    }

    #[test]
    fn audit_multiple_deps_higher_risk() {
        let spec = make_spec_with_deps(vec![
            ExternalDependency { name: "Auth0".into(), usage_description: "Auth".into(), dtu_priority: DtuPriority::High },
            ExternalDependency { name: "Stripe".into(), usage_description: "Payments".into(), dtu_priority: DtuPriority::High },
            ExternalDependency { name: "SendGrid".into(), usage_description: "Email".into(), dtu_priority: DtuPriority::High },
        ]);

        let report = audit_lock_in(&spec);
        assert!(report.findings.len() >= 3);
        assert!(report.risk_score > 0.0);
    }

    #[test]
    fn audit_low_priority_dep_fewer_findings() {
        let spec = make_spec_with_deps(vec![
            ExternalDependency {
                name: "Redis".into(),
                usage_description: "Caching".into(),
                dtu_priority: DtuPriority::Low,
            },
        ]);

        let report = audit_lock_in(&spec);
        // Redis is not in VENDOR_ANALYSIS, and Low priority = no proprietary API finding
        assert!(report.findings.is_empty());
    }

    #[test]
    fn recommendations_generated_for_findings() {
        let spec = make_spec_with_deps(vec![
            ExternalDependency { name: "Auth0".into(), usage_description: "Auth".into(), dtu_priority: DtuPriority::High },
        ]);

        let report = audit_lock_in(&spec);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn risk_score_bounded() {
        let spec = make_spec_with_deps(vec![
            ExternalDependency { name: "Auth0".into(), usage_description: "Auth".into(), dtu_priority: DtuPriority::High },
            ExternalDependency { name: "Firebase".into(), usage_description: "DB".into(), dtu_priority: DtuPriority::High },
            ExternalDependency { name: "AWS Lambda".into(), usage_description: "Compute".into(), dtu_priority: DtuPriority::High },
        ]);

        let report = audit_lock_in(&spec);
        assert!(report.risk_score >= 0.0);
        assert!(report.risk_score <= 1.0);
    }
}
