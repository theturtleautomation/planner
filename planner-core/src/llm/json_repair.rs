//! # JSON Repair Helper
//!
//! Attempts to recover valid JSON from malformed LLM responses.
//! LLMs sometimes wrap JSON in markdown fences, add preamble text,
//! or append trailing commentary. This module tries common fixes.

/// Attempt to repair malformed JSON from an LLM response.
///
/// Strategy:
/// 1. Strip markdown code fences
/// 2. Try to parse the cleaned string as-is
/// 3. Find the first `{` and last `}` and try that substring
/// 4. Try stripping common LLM preambles before the JSON object
///
/// Returns `Some(json_string)` if any strategy succeeds, `None` if all fail.
pub fn try_repair_json(raw: &str) -> Option<String> {
    // 1. Strip markdown code fences
    let s = strip_code_fences(raw);

    // 2. Try to parse as-is (handles already-valid JSON)
    if serde_json::from_str::<serde_json::Value>(&s).is_ok() {
        return Some(s);
    }

    // 3. Find first { and last } and try that substring
    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
        if start < end {
            let candidate = &s[start..=end];
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
    }

    // 4. Try stripping common LLM preambles
    for prefix in ["Here is", "Sure,", "```json", "The JSON", "Here's"] {
        if let Some(pos) = s.find(prefix) {
            let after = &s[pos..];
            if let (Some(start), Some(end)) = (after.find('{'), after.rfind('}')) {
                if start < end {
                    let candidate = &after[start..=end];
                    if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                        return Some(candidate.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Strip markdown code fences from a string.
///
/// Handles ` ```json `, ` ``` `, and bare content.
pub(crate) fn strip_code_fences(s: &str) -> String {
    let s = s.trim();
    if s.starts_with("```") {
        // Drop the opening fence line (e.g. "```json")
        let after_first = s.splitn(2, '\n').nth(1).unwrap_or(s);
        // Drop trailing fence
        let trimmed = after_first.trim_end_matches("```").trim();
        trimmed.to_string()
    } else {
        s.to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_valid_json_passthrough() {
        let raw = r#"{"key": "value", "num": 42}"#;
        let result = try_repair_json(raw);
        assert!(result.is_some());
        let s = result.unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn repair_strips_code_fences() {
        let raw = "```json\n{\"key\": \"value\"}\n```";
        let result = try_repair_json(raw);
        assert!(result.is_some(), "Should parse JSON with code fences");
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn repair_strips_bare_code_fences() {
        let raw = "```\n{\"key\": \"value\"}\n```";
        let result = try_repair_json(raw);
        assert!(result.is_some());
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn repair_finds_json_in_preamble_text() {
        let raw = "Here is the JSON you requested:\n{\"score\": 0.9, \"passed\": true}";
        let result = try_repair_json(raw);
        assert!(result.is_some(), "Should extract JSON from preamble");
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["passed"], true);
    }

    #[test]
    fn repair_finds_json_boundaries() {
        let raw = "Some leading text {\"amendments\": []} trailing text here";
        let result = try_repair_json(raw);
        assert!(result.is_some(), "Should find JSON by boundaries");
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(v["amendments"].is_array());
    }

    #[test]
    fn repair_returns_none_on_truly_broken_json() {
        let raw = "This is not JSON at all, no braces here";
        let result = try_repair_json(raw);
        assert!(result.is_none(), "Should return None for content with no JSON");
    }

    #[test]
    fn strip_code_fences_no_fences() {
        let s = "{\"key\": \"value\"}";
        assert_eq!(strip_code_fences(s), "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_code_fences_json_fence() {
        let s = "```json\n{\"key\": \"value\"}\n```";
        let result = strip_code_fences(s);
        assert!(result.contains("\"key\""));
        assert!(!result.contains("```"));
    }

    #[test]
    fn repair_with_trailing_text_after_json() {
        let raw = "{\"score\": 0.8, \"passed\": true}\n\nAdditional commentary here.";
        let result = try_repair_json(raw);
        // The brace-boundary finder should extract the valid JSON
        assert!(result.is_some());
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["passed"], true);
    }
}
