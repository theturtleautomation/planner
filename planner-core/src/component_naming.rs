use planner_schemas::artifacts::blueprint::{
    Component, ComponentNameSource, ComponentNaming, ComponentNamingStrategy, ComponentType,
};

pub const COMPONENT_NAMING_VERSION: u16 = 1;

const BOILERPLATE_PATH_SEGMENTS: &[&str] = &["src", "crates", "packages", "app", "lib", "general"];
const GENERIC_SUBJECT_TOKENS: &[&str] = &[
    "api", "core", "web", "ui", "lib", "store", "module", "component", "service", "project",
    "workspace", "output", "dist", "build", "target", "factory", "internal", "shared", "common",
];

#[derive(Debug, Clone)]
pub struct GeneratedComponentName {
    pub name: String,
    pub naming: ComponentNaming,
}

pub struct SpecGroupNamingInput<'a> {
    pub project_id: &'a str,
    pub project_name: Option<&'a str>,
    pub chunk_tag: &'a str,
    pub group_token: &'a str,
    pub statements: &'a [String],
    pub component_type: ComponentType,
    pub timestamp: &'a str,
}

pub struct DirectoryNamingInput<'a> {
    pub relative_path: &'a str,
    pub project_name: Option<&'a str>,
    pub component_type: ComponentType,
    pub timestamp: &'a str,
}

pub struct FactoryNamingInput<'a> {
    pub output_path: &'a str,
    pub project_name: Option<&'a str>,
    pub timestamp: &'a str,
}

pub fn generated_naming(
    origin_key: String,
    strategy: ComponentNamingStrategy,
    generated_name: String,
    timestamp: &str,
) -> ComponentNaming {
    ComponentNaming {
        origin_key,
        source: ComponentNameSource::Generated,
        strategy,
        generated_name,
        naming_version: COMPONENT_NAMING_VERSION,
        last_generated_at: timestamp.to_string(),
    }
}

pub fn manual_naming(node_id: &str, name: &str, timestamp: &str) -> ComponentNaming {
    let trimmed_name = normalize_whitespace(name);
    ComponentNaming {
        origin_key: format!("manual:{}", node_id),
        source: ComponentNameSource::Manual,
        strategy: ComponentNamingStrategy::ManualCreate,
        generated_name: if trimmed_name.is_empty() {
            "Component".into()
        } else {
            trimmed_name
        },
        naming_version: COMPONENT_NAMING_VERSION,
        last_generated_at: timestamp.to_string(),
    }
}

pub fn generate_spec_name(input: SpecGroupNamingInput<'_>) -> GeneratedComponentName {
    let mut tokens = tokenize_identifier(input.group_token);
    tokens.extend(tokenize_identifier(input.chunk_tag));
    for statement in input.statements {
        tokens.extend(tokenize_text(statement));
    }

    let subject = best_subject(tokens, input.project_name, Some(input.group_token));
    let role = infer_role(input.component_type, &subject);
    let name = finalize_name(subject, role, input.project_name, &tokens_from_string(input.group_token));

    let origin_key = format!(
        "spec:{}:{}:{}",
        normalize_origin_segment(input.project_id),
        normalize_origin_segment(input.chunk_tag),
        normalize_origin_segment(input.group_token)
    );

    let naming = generated_naming(
        origin_key,
        ComponentNamingStrategy::SpecGroup,
        name.clone(),
        input.timestamp,
    );

    GeneratedComponentName { name, naming }
}

pub fn generate_directory_name(input: DirectoryNamingInput<'_>) -> GeneratedComponentName {
    let normalized_path = normalize_path(input.relative_path);
    let path_segments: Vec<String> = normalized_path
        .split('/')
        .filter(|segment| !segment.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let mut meaningful_segments: Vec<String> = path_segments
        .iter()
        .filter(|segment| {
            let lower = segment.to_ascii_lowercase();
            !BOILERPLATE_PATH_SEGMENTS
                .iter()
                .any(|boilerplate| *boilerplate == lower)
        })
        .cloned()
        .collect();

    if meaningful_segments.is_empty() {
        meaningful_segments = path_segments.clone();
    }

    let leaf = meaningful_segments.last().cloned().unwrap_or_else(|| "component".into());
    let parent = if meaningful_segments.len() >= 2 {
        meaningful_segments.get(meaningful_segments.len() - 2).cloned()
    } else {
        None
    };

    let mut tokens: Vec<String> = meaningful_segments
        .iter()
        .flat_map(|segment| tokenize_identifier(segment))
        .collect();

    if tokens.is_empty() {
        tokens.push("component".into());
    }

    let mut subject = best_subject(tokens.clone(), input.project_name, Some(&leaf));

    if is_generic_subject(&subject) {
        let leaf_tokens = tokenize_identifier(&leaf);
        let parent_tokens = parent
            .as_ref()
            .map(|value| tokenize_identifier(value))
            .unwrap_or_default();

        if let Some(parent_subject) = first_non_generic_subject(&parent_tokens) {
            subject = parent_subject;
        } else if leaf_tokens.iter().any(|token| token == "web" || token == "ui") {
            subject = "Web App".into();
        } else if leaf_tokens.iter().any(|token| token == "api") {
            subject = "Public".into();
        }
    }

    let role = infer_role(input.component_type, &subject);
    let name = finalize_name(subject, role, input.project_name, &tokens);

    let naming = generated_naming(
        format!("path:{}", normalize_origin_segment(&normalized_path)),
        ComponentNamingStrategy::DirectoryScan,
        name.clone(),
        input.timestamp,
    );

    GeneratedComponentName { name, naming }
}

pub fn generate_factory_name(input: FactoryNamingInput<'_>) -> GeneratedComponentName {
    let normalized_path = normalize_path(input.output_path);
    let basename = normalized_path
        .rsplit('/')
        .next()
        .unwrap_or("workspace")
        .trim();

    let mut basename_tokens = tokenize_identifier(basename);
    basename_tokens.retain(|token| {
        !matches!(
            token.as_str(),
            "output" | "dist" | "build" | "target" | "factory" | "workspace"
        )
    });

    let subject = if let Some(subject) = first_non_generic_subject(&basename_tokens) {
        subject
    } else if let Some(project) = cleaned_project_subject(input.project_name) {
        project
    } else {
        "Generated".into()
    };

    let mut name = format!("{} Workspace", subject);
    if is_weak_component_name(&name) {
        name = "Generated Workspace".into();
    }

    let naming = generated_naming(
        format!("factory:{}", normalize_origin_segment(&normalized_path)),
        ComponentNamingStrategy::FactoryOutput,
        name.clone(),
        input.timestamp,
    );

    GeneratedComponentName { name, naming }
}

pub fn is_weak_component_name(name: &str) -> bool {
    let trimmed = normalize_whitespace(name);
    if trimmed.is_empty() {
        return true;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower == "factory output" || lower == "unknown module" || lower.ends_with(" module") {
        return true;
    }

    if lower.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }

    let tokens: Vec<&str> = lower.split_whitespace().collect();
    if tokens.len() == 2 && tokens[1] == "module" && tokens[0].chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }

    tokens.len() == 1
        && matches!(
            tokens[0],
            "api" | "core" | "web" | "lib" | "store" | "module" | "unknown"
        )
}

pub fn merge_generated_component(existing: &Component, generated: &GeneratedComponentName) -> Component {
    let mut next = existing.clone();
    let old_name = normalize_whitespace(&existing.name);

    let mut naming = existing
        .naming
        .clone()
        .unwrap_or_else(|| generated.naming.clone());

    naming.origin_key = if naming.origin_key.trim().is_empty() {
        generated.naming.origin_key.clone()
    } else {
        naming.origin_key
    };
    naming.strategy = generated.naming.strategy.clone();
    naming.generated_name = generated.name.clone();
    naming.naming_version = generated.naming.naming_version;
    naming.last_generated_at = generated.naming.last_generated_at.clone();

    let should_update_name = match existing.naming.as_ref() {
        Some(meta) if meta.source == ComponentNameSource::Manual => false,
        Some(meta) if meta.source == ComponentNameSource::Generated => {
            old_name == normalize_whitespace(&meta.generated_name) || is_weak_component_name(&old_name)
        }
        Some(_) => false,
        None => old_name == normalize_whitespace(&generated.name) || is_weak_component_name(&old_name),
    };

    if should_update_name {
        next.name = generated.name.clone();
        naming.source = ComponentNameSource::Generated;
    } else if existing
        .naming
        .as_ref()
        .is_some_and(|meta| meta.source == ComponentNameSource::Manual)
    {
        naming.source = ComponentNameSource::Manual;
    } else if existing.naming.is_none() {
        // Legacy component with no provenance and a non-weak custom name.
        naming.source = ComponentNameSource::Manual;
    }

    next.naming = Some(naming);
    next.updated_at = generated.naming.last_generated_at.clone();
    next
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_string()
}

fn normalize_origin_segment(value: &str) -> String {
    let normalized = normalize_path(value).to_ascii_lowercase();
    let mut out = String::with_capacity(normalized.len());
    let mut prev_sep = false;
    for ch in normalized.chars() {
        if ch.is_ascii_alphanumeric() || ch == '/' {
            out.push(ch);
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn tokenize_identifier(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .collect()
}

fn tokenize_text(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|segment| segment.len() >= 3)
        .map(|segment| segment.to_ascii_lowercase())
        .collect()
}

fn tokens_from_string(value: &str) -> Vec<String> {
    tokenize_identifier(value)
}

fn cleaned_project_subject(project_name: Option<&str>) -> Option<String> {
    let candidate = project_name.map(normalize_whitespace)?;
    if candidate.is_empty() {
        return None;
    }

    let words: Vec<&str> = candidate.split_whitespace().take(3).collect();
    if words.is_empty() {
        None
    } else {
        Some(words.join(" "))
    }
}

fn best_subject(tokens: Vec<String>, project_name: Option<&str>, seed: Option<&str>) -> String {
    if let Some(seed_value) = seed {
        let seed_subject = normalize_subject(seed_value);
        if !seed_subject.is_empty() && !is_generic_subject(&seed_subject) {
            return seed_subject;
        }
    }

    if let Some(subject) = first_non_generic_subject(&tokens) {
        return subject;
    }

    if let Some(project) = cleaned_project_subject(project_name) {
        return project;
    }

    "Project".into()
}

fn first_non_generic_subject(tokens: &[String]) -> Option<String> {
    let aliases: Vec<String> = tokens
        .iter()
        .filter_map(|token| alias_subject(token))
        .collect();
    if let Some(subject) = aliases.first() {
        return Some(subject.clone());
    }

    let mut words = Vec::new();
    for token in tokens {
        if token.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        if GENERIC_SUBJECT_TOKENS.iter().any(|generic| *generic == token) {
            continue;
        }
        words.push(format_token(token));
        if words.len() >= 3 {
            break;
        }
    }

    if words.is_empty() {
        None
    } else {
        Some(words.join(" "))
    }
}

fn alias_subject(token: &str) -> Option<String> {
    let subject = match token {
        "auth" | "authentication" => "Authentication",
        "billing" | "payment" | "payments" => "Billing",
        "notify" | "notification" | "notifications" => "Notifications",
        "search" => "Search",
        "admin" => "Administration",
        "config" | "settings" => "Configuration",
        "sync" => "Sync",
        "report" | "reporting" => "Reporting",
        "analytics" => "Analytics",
        _ => return None,
    };
    Some(subject.into())
}

fn normalize_subject(value: &str) -> String {
    let tokens = tokenize_identifier(value);
    if tokens.is_empty() {
        return String::new();
    }

    let words: Vec<String> = tokens
        .into_iter()
        .map(|token| alias_subject(&token).unwrap_or_else(|| format_token(&token)))
        .take(3)
        .collect();

    normalize_whitespace(&words.join(" "))
}

fn infer_role(component_type: ComponentType, subject: &str) -> Option<&'static str> {
    let lower = subject.to_ascii_lowercase();

    match component_type {
        ComponentType::Store => Some("Data Store"),
        ComponentType::Pipeline => Some("Pipeline"),
        ComponentType::Service => Some("Service"),
        ComponentType::Library => Some("Library"),
        ComponentType::Interface => {
            if lower.contains("api") {
                Some("API")
            } else {
                Some("UI")
            }
        }
        ComponentType::Module => {
            if lower.contains("api") || lower.contains("gateway") {
                Some("API")
            } else if lower.contains("ui") || lower.contains("dashboard") || lower.contains("web") {
                Some("UI")
            } else if lower.contains("store") || lower.contains("cache") || lower.contains("data") {
                Some("Data Store")
            } else if lower.contains("pipeline") {
                Some("Pipeline")
            } else if lower.contains("sync") || lower.contains("worker") {
                Some("Worker")
            } else if lower.contains("library") {
                Some("Library")
            } else {
                Some("Service")
            }
        }
    }
}

fn finalize_name(
    subject: String,
    role: Option<&'static str>,
    project_name: Option<&str>,
    fallback_tokens: &[String],
) -> String {
    let mut base_subject = normalize_whitespace(&subject);
    if base_subject.is_empty() || is_generic_subject(&base_subject) {
        if let Some(project) = cleaned_project_subject(project_name) {
            base_subject = project;
        } else if let Some(subject_from_tokens) = first_non_generic_subject(fallback_tokens) {
            base_subject = subject_from_tokens;
        } else {
            base_subject = "Project".into();
        }
    }

    let name = if let Some(role_label) = role {
        if has_role_word(&base_subject, role_label) {
            base_subject
        } else {
            format!("{} {}", base_subject, role_label)
        }
    } else {
        base_subject
    };

    let normalized = normalize_whitespace(&name);
    if is_weak_component_name(&normalized) {
        let fallback = if let Some(project) = cleaned_project_subject(project_name) {
            format!("{} Workspace", project)
        } else {
            "Project Workspace".into()
        };
        normalize_whitespace(&fallback)
    } else {
        normalized
    }
}

fn has_role_word(subject: &str, role: &str) -> bool {
    let subject_lower = subject.to_ascii_lowercase();
    let role_lower = role.to_ascii_lowercase();

    if subject_lower.contains(&role_lower) {
        return true;
    }

    (role == "API" && subject_lower.contains(" api"))
        || (role == "UI" && subject_lower.contains(" ui"))
        || (role == "Data Store" && subject_lower.contains("store"))
}

fn format_token(token: &str) -> String {
    match token {
        "api" => "API".into(),
        "ui" => "UI".into(),
        "jwt" => "JWT".into(),
        "oauth" => "OAuth".into(),
        "http" => "HTTP".into(),
        "cli" => "CLI".into(),
        "llm" => "LLM".into(),
        value => {
            let mut chars = value.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        }
    }
}

fn is_generic_subject(subject: &str) -> bool {
    let normalized = normalize_whitespace(subject).to_ascii_lowercase();
    if normalized.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }

    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    tokens.is_empty()
        || tokens
            .iter()
            .all(|token| GENERIC_SUBJECT_TOKENS.iter().any(|candidate| token == candidate))
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weak_name_blacklist_rejects_legacy_fallbacks() {
        assert!(is_weak_component_name("Unknown Module"));
        assert!(is_weak_component_name("001 Module"));
        assert!(is_weak_component_name("Api"));
        assert!(is_weak_component_name("Factory Output"));
        assert!(!is_weak_component_name("Authentication Service"));
    }

    #[test]
    fn spec_names_do_not_use_numeric_groups_or_module_suffix() {
        let statements = vec![
            "The system must support JWT authentication".to_string(),
            "Users can sign in with OAuth".to_string(),
        ];
        let generated = generate_spec_name(SpecGroupNamingInput {
            project_id: "proj-1",
            project_name: Some("Task Tracker"),
            chunk_tag: "root",
            group_token: "001",
            statements: &statements,
            component_type: ComponentType::Module,
            timestamp: "2026-03-07T00:00:00Z",
        });

        assert!(!generated.name.ends_with("Module"));
        assert!(!generated.name.starts_with("001"));
        assert!(generated.name.contains("Authentication"));
        assert_eq!(generated.naming.origin_key, "spec:proj_1:root:001");
    }

    #[test]
    fn directory_scan_names_promote_generic_api_label() {
        let generated = generate_directory_name(DirectoryNamingInput {
            relative_path: "api",
            project_name: None,
            component_type: ComponentType::Service,
            timestamp: "2026-03-07T00:00:00Z",
        });

        assert_ne!(generated.name, "Api");
        assert!(generated.name.contains("API") || generated.name.contains("Service"));
        assert_eq!(generated.naming.origin_key, "path:api");
    }

    #[test]
    fn merge_generated_component_preserves_manual_name() {
        let existing = Component {
            id: planner_schemas::artifacts::blueprint::NodeId::from_raw("comp-auth"),
            name: "Identity".into(),
            component_type: ComponentType::Service,
            naming: Some(ComponentNaming {
                origin_key: "spec:proj:root:auth".into(),
                source: ComponentNameSource::Manual,
                strategy: ComponentNamingStrategy::SpecGroup,
                generated_name: "Authentication Service".into(),
                naming_version: 1,
                last_generated_at: "2026-03-01T00:00:00Z".into(),
            }),
            description: "".into(),
            provides: Vec::new(),
            consumes: Vec::new(),
            status: planner_schemas::artifacts::blueprint::ComponentStatus::Planned,
            tags: Vec::new(),
            documentation: None,
            scope: planner_schemas::artifacts::blueprint::NodeScope::default(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-01T00:00:00Z".into(),
        };

        let generated = GeneratedComponentName {
            name: "Authentication Service".into(),
            naming: generated_naming(
                "spec:proj:root:auth".into(),
                ComponentNamingStrategy::SpecGroup,
                "Authentication Service".into(),
                "2026-03-07T00:00:00Z",
            ),
        };

        let merged = merge_generated_component(&existing, &generated);
        assert_eq!(merged.name, "Identity");
        assert_eq!(
            merged.naming.map(|n| n.source),
            Some(ComponentNameSource::Manual)
        );
    }
}
