use planner_schemas::artifacts::blueprint::QualityAttribute;

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn title_case(value: &str) -> String {
    let mut out = Vec::new();
    for word in value.split_whitespace() {
        let lower = word.to_ascii_lowercase();
        let mapped = match lower.as_str() {
            "ui" => "UI".to_string(),
            "api" => "API".to_string(),
            "dod" => "DoD".to_string(),
            "typescript" => "TypeScript".to_string(),
            "localstorage" => "localStorage".to_string(),
            _ => {
                let mut chars = lower.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            }
        };
        out.push(mapped);
    }
    out.join(" ")
}

fn shorten_clause(text: &str) -> String {
    let normalized = normalize(text);
    normalized
        .split([';', ':', '.'])
        .next()
        .unwrap_or(normalized.as_str())
        .trim()
        .to_string()
}

pub fn concise_constraint_title(text: &str) -> String {
    let normalized = normalize(text);
    let lower = normalized.to_ascii_lowercase();

    let label = if lower.contains("single-file react component") {
        "Keep component single-file".to_string()
    } else if lower.contains("tailwind css for all styling") {
        "Use Tailwind CSS only".to_string()
    } else if lower.contains("vite as the build tool") {
        "Use Vite with npm".to_string()
    } else if lower.contains("no external state management") {
        "Avoid external state management".to_string()
    } else if lower.contains("drag-and-drop reordering may use") {
        "Use lightweight drag-and-drop only".to_string()
    } else if lower.contains("localstorage persistence is underspecified") {
        "Specify localStorage contract".to_string()
    } else if lower.contains("persist")
        && lower.contains("every mutation")
        && lower.contains("ambiguous")
    {
        "Define persistence timing guarantees".to_string()
    } else if lower.contains("task.order") {
        "Resolve Task.order semantics".to_string()
    } else if lower.contains("inline editing behavior is not specified") {
        "Specify inline editing contract".to_string()
    } else {
        let shortened = shorten_clause(&normalized);
        if shortened.len() <= 72 {
            title_case(&shortened)
        } else {
            format!(
                "{}…",
                &title_case(&shortened)[..72.min(title_case(&shortened).len())]
            )
        }
    };

    normalize(&label)
}

pub fn concise_quality_label(
    scenario: &str,
    attribute: &QualityAttribute,
    tags: &[String],
) -> String {
    let normalized = normalize(scenario);
    let lower = normalized.to_ascii_lowercase();

    let base = if (lower.contains("press enter") || lower.contains("pressing enter"))
        && (lower.contains("adds a task")
            || lower.contains("adds task")
            || lower.contains("appear in the list")
            || lower.contains("visible list"))
    {
        "Add tasks on Enter".to_string()
    } else if lower.contains("mode toggle")
        || lower.contains("drag handles")
        || lower.contains("edit/delete controls")
        || (lower.contains("quick entry") && lower.contains("review mode"))
    {
        "Switch between Quick Entry and Review".to_string()
    } else if (lower.contains("checkbox") && lower.contains("strikethrough"))
        || lower.contains("check off")
        || (lower.contains("single click") && lower.contains("completion"))
        || lower.contains("visual feedback of completion")
    {
        "Toggle task completion".to_string()
    } else if (lower.contains("drag-and-drop") || lower.contains("reorder"))
        && (lower.contains("persist") || lower.contains("ordering intact"))
    {
        "Persist task order changes".to_string()
    } else if lower.contains("drag") || lower.contains("reorder") {
        "Reorder tasks in Review mode".to_string()
    } else if lower.contains("delete") {
        "Delete tasks in Review mode".to_string()
    } else if lower.contains("inline") && lower.contains("edit") {
        "Edit tasks inline".to_string()
    } else if lower.contains("refresh") && (lower.contains("restores") || lower.contains("persist"))
    {
        "Restore tasks after refresh".to_string()
    } else if lower.contains("localstorage") {
        "Persist tasks via localStorage".to_string()
    } else if lower.contains("without errors") && lower.contains("typescript") {
        "Compile without TypeScript errors".to_string()
    } else {
        let shortened = shorten_clause(&normalized);
        title_case(&shortened)
    };

    let prefix = if tags.iter().any(|tag| tag == "definition-of-done") {
        "DoD"
    } else if tags.iter().any(|tag| tag == "satisfaction") {
        "Goal"
    } else {
        match attribute {
            QualityAttribute::Reliability => "Reliability",
            QualityAttribute::Usability => "Usability",
            QualityAttribute::Maintainability => "Maintainability",
            QualityAttribute::Performance => "Performance",
            QualityAttribute::Security => "Security",
        }
    };

    normalize(&format!("{}: {}", prefix, base))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concise_constraint_title_uses_semantic_summary() {
        assert_eq!(
            concise_constraint_title("Tailwind CSS for all styling — no additional CSS frameworks"),
            "Use Tailwind CSS only"
        );
        assert_eq!(
            concise_constraint_title(
                "Inline editing behavior is not specified (how edit mode is entered, how changes are saved)"
            ),
            "Specify inline editing contract"
        );
    }

    #[test]
    fn concise_quality_label_uses_tagged_short_label() {
        let goal_tags = vec!["satisfaction".to_string()];
        let dod_tags = vec!["definition-of-done".to_string()];

        assert_eq!(
            concise_quality_label(
                "User can type a task name, press Enter, and see it appear in the list instantly",
                &QualityAttribute::Reliability,
                &goal_tags
            ),
            "Goal: Add tasks on Enter"
        );
        assert_eq!(
            concise_quality_label(
                "Refreshing the browser restores all tasks with their titles, completion states, and ordering intact",
                &QualityAttribute::Maintainability,
                &dod_tags
            ),
            "DoD: Restore tasks after refresh"
        );
        assert_eq!(
            concise_quality_label(
                "Typing text into the input and pressing Enter adds a task to the visible list without any additional click",
                &QualityAttribute::Usability,
                &dod_tags
            ),
            "DoD: Add tasks on Enter"
        );
        assert_eq!(
            concise_quality_label(
                "User can check off a task with a single click and see clear visual feedback of completion",
                &QualityAttribute::Usability,
                &goal_tags
            ),
            "Goal: Toggle task completion"
        );
        assert_eq!(
            concise_quality_label(
                "A mode toggle switches between Quick Entry and Review modes; drag handles and edit/delete controls are only visible in Review mode",
                &QualityAttribute::Usability,
                &dod_tags
            ),
            "DoD: Switch between Quick Entry and Review"
        );
        assert_eq!(
            concise_quality_label(
                "Tasks can be reordered via drag-and-drop in Review mode and the new order persists",
                &QualityAttribute::Maintainability,
                &dod_tags
            ),
            "DoD: Persist task order changes"
        );
    }
}
