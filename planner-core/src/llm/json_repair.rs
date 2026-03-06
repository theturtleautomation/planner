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

    // 5. Attempt to close truncated JSON by balancing braces/brackets.
    //    LLM responses can be truncated mid-stream when hitting output
    //    token limits, leaving valid JSON with missing closing delimiters.
    if let Some(start) = s.find('{') {
        if let Some(repaired) = try_close_truncated_json(&s[start..]) {
            return Some(repaired);
        }
    }

    None
}

/// Attempt to close truncated JSON by analyzing bracket/brace nesting.
///
/// When an LLM response is truncated mid-stream (e.g. hitting output token
/// limits), the JSON may be structurally valid up to the truncation point
/// but missing closing delimiters. This function:
///
/// 1. Finds the last position where the JSON is "clean" (not mid-string
///    or mid-key), truncating any partial string/value at the end.
/// 2. Appends the necessary closing `]` and `}` to balance the nesting.
///
/// This is intentionally conservative — it only handles the common case
/// of truncation that leaves a valid JSON prefix needing closing.
fn try_close_truncated_json(s: &str) -> Option<String> {
    // Track nesting stack: '{' or '['
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escape_next = false;
    // Whether the next string close is a value (true) or a key (false).
    // In objects, strings alternate key → value. In arrays, all strings
    // are values. We use the nesting stack to determine context.
    let mut expecting_value = false;
    // Position just after the last complete value (not key).
    let mut last_value_end: usize = 0;

    for (i, ch) in s.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if in_string {
            match ch {
                '\\' => escape_next = true,
                '"' => {
                    in_string = false;
                    if expecting_value || stack.last() == Some(&'[') {
                        // This string is a value (after ':' or inside array)
                        last_value_end = i + 1;
                    }
                    // If it's a key (inside object, not after ':'), don't
                    // update last_value_end — we'd lose the previous pair.
                }
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
            }
            ':' => {
                // After a key's colon, the next value is a value.
                expecting_value = true;
            }
            '{' => {
                stack.push('{');
                expecting_value = false; // next string in object is a key
            }
            '[' => {
                stack.push('[');
                // In arrays, all elements are values — handled by the
                // `stack.last() == Some(&'[')` check above.
            }
            '}' => {
                if stack.last() == Some(&'{') {
                    stack.pop();
                    last_value_end = i + 1;
                    expecting_value = false;
                }
            }
            ']' => {
                if stack.last() == Some(&'[') {
                    stack.pop();
                    last_value_end = i + 1;
                }
            }
            ',' => {
                // Comma marks the boundary between complete elements.
                last_value_end = i;
                expecting_value = false; // next string in object is a key
            }
            _ => {
                // Whitespace, bare values (number, true, false, null).
                // For bare values after ':', mark end when we hit the
                // next structural character. For simplicity, we rely on
                // comma/close handling above to capture these.
            }
        }
    }

    // If everything is balanced and we're not mid-string, nothing to do
    if stack.is_empty() && !in_string {
        return None;
    }

    if last_value_end == 0 {
        return None;
    }

    // Truncate to the last complete value boundary.
    let mut result = s[..last_value_end].trim_end().to_string();

    // Remove trailing comma
    if result.ends_with(',') {
        result.pop();
    }

    // Re-scan the truncated result to determine which delimiters need closing.
    // This is more robust than tracking the stack through truncation.
    let mut close_stack: Vec<char> = Vec::new();
    let mut close_in_string = false;
    let mut close_escape = false;
    for ch in result.chars() {
        if close_escape {
            close_escape = false;
            continue;
        }
        if close_in_string {
            match ch {
                '\\' => close_escape = true,
                '"' => close_in_string = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => close_in_string = true,
            '{' => close_stack.push('{'),
            '[' => close_stack.push('['),
            '}' => {
                close_stack.pop();
            }
            ']' => {
                close_stack.pop();
            }
            _ => {}
        }
    }

    // Close in reverse order (innermost first)
    for &opener in close_stack.iter().rev() {
        match opener {
            '{' => result.push('}'),
            '[' => result.push(']'),
            _ => {}
        }
    }

    // Validate the repaired JSON
    if serde_json::from_str::<serde_json::Value>(&result).is_ok() {
        Some(result)
    } else {
        None
    }
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
        assert!(
            result.is_none(),
            "Should return None for content with no JSON"
        );
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

    // --- Truncated JSON repair tests ---

    #[test]
    fn repair_truncated_json_missing_closing_brace() {
        // Simulates LLM output truncated mid-response: object left unclosed
        let raw = r#"{"findings": [{"severity": "blocking", "description": "Issue"}], "summary": "Partial""#;
        let result = try_repair_json(raw);
        assert!(result.is_some(), "Should close truncated JSON");
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["summary"], "Partial");
    }

    #[test]
    fn repair_truncated_json_missing_closing_bracket_and_brace() {
        // Truncated inside an array: array and root object both unclosed
        let raw = r#"{"findings": [{"severity": "advisory", "description": "A"}, {"severity": "blocking", "description": "B"}"#;
        let result = try_repair_json(raw);
        assert!(result.is_some(), "Should close truncated array + object");
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["findings"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn repair_truncated_json_mid_string_value() {
        // Truncated in the middle of a string value
        let raw = r#"{"findings": [], "summary": "This is a long summ"#;
        let result = try_repair_json(raw);
        assert!(result.is_some(), "Should handle mid-string truncation");
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        // The partial string is dropped; findings array should survive
        assert!(v["findings"].is_array());
    }

    #[test]
    fn repair_truncated_json_trailing_comma() {
        // Truncated right after a comma (common when a new field was about to start)
        let raw = r#"{"findings": [{"severity": "blocking", "description": "X"}],"#;
        let result = try_repair_json(raw);
        assert!(result.is_some(), "Should handle trailing comma truncation");
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["findings"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn repair_truncated_json_realistic_ar_response() {
        // Realistic: AR GPT response truncated mid-finding (mirrors the actual bug)
        let raw = r#"{
  "findings": [
    {
      "severity": "blocking",
      "affected_section": "Amendment Log",
      "affected_requirements": ["FR-1", "FR-3"],
      "description": "The Amendment Log mentions concrete deliverable constra"#;
        let result = try_repair_json(raw);
        assert!(
            result.is_some(),
            "Should repair realistic truncated AR response"
        );
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        // The partial finding gets dropped since it's mid-string,
        // but the outer structure should be valid
        assert!(v["findings"].is_array());
    }

    #[test]
    fn try_close_truncated_returns_none_for_balanced() {
        // Already balanced — should return None (let other strategies handle)
        let s = r#"{"key": "value"}"#;
        assert!(try_close_truncated_json(s).is_none());
    }
}
