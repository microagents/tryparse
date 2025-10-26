//! Cleaning strategies that normalize and remove garbage from extracted JSON.

use crate::{error::Result, parser::Candidate};

/// Trait for strategies that clean/normalize extracted JSON candidates.
///
/// Cleaners run between extraction and fixing to remove garbage characters,
/// normalize whitespace, and perform other preprocessing.
pub trait Cleaner: Send + Sync + std::fmt::Debug {
    /// Returns the name of this cleaner for debugging.
    fn name(&self) -> &'static str;

    /// Cleans the candidate, returning a new candidate if changes were made.
    ///
    /// Returns None if no cleaning was needed.
    fn clean(&self, candidate: &Candidate) -> Result<Option<Candidate>>;
}

/// Garbage cleaner that removes common invalid patterns.
///
/// Removes:
/// - Multiple consecutive commas: `,,,,` → `,`
/// - Trailing commas before closing braces/brackets (if not caught by fixer)
/// - Extra whitespace in structural positions
/// - Invalid characters between JSON elements
#[derive(Debug, Clone, Default)]
pub struct GarbageCleaner;

impl GarbageCleaner {
    /// Creates a new garbage cleaner.
    pub fn new() -> Self {
        Self
    }

    /// Removes multiple consecutive commas.
    fn clean_multiple_commas(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut last_was_comma = false;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                result.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => {
                    escape_next = true;
                    result.push(ch);
                }
                '"' => {
                    in_string = !in_string;
                    last_was_comma = false;
                    result.push(ch);
                }
                ',' if !in_string => {
                    if !last_was_comma {
                        result.push(ch);
                        last_was_comma = true;
                    }
                    // Skip additional commas
                }
                _ => {
                    if !ch.is_whitespace() {
                        last_was_comma = false;
                    }
                    result.push(ch);
                }
            }
        }

        result
    }

    /// Removes unnecessary backslashes before quotes.
    ///
    /// LLMs sometimes output JSON with literal backslashes:
    /// `{\"name\": \"Alice\"}` → `{"name": "Alice"}`
    ///
    /// This is different from double-escaped JSON (which wraps in quotes).
    /// We replace `\"` with `"` unless the backslash itself is escaped.
    ///
    /// IMPORTANT: We only remove backslashes that appear OUTSIDE of JSON string values.
    /// Inside string values, `\"` is valid JSON and should be preserved.
    pub fn fix_unnecessary_backslashes(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        let mut in_string = false;
        let mut string_escape_next = false;

        while i < chars.len() {
            let ch = chars[i];

            // Track if we're inside a JSON string value
            if ch == '"' && !string_escape_next {
                in_string = !in_string;
                result.push(ch);
                i += 1;
                string_escape_next = false;
                continue;
            }

            if in_string {
                // Inside a string value - preserve everything as-is, including `\"`
                if string_escape_next {
                    string_escape_next = false;
                } else if ch == '\\' {
                    string_escape_next = true;
                }
                result.push(ch);
                i += 1;
                continue;
            }

            // Outside a string value - check for unnecessary backslashes
            if ch == '\\' && i + 1 < chars.len() {
                // Check if this backslash is itself escaped
                let num_preceding_backslashes =
                    (0..i).rev().take_while(|&j| chars[j] == '\\').count();

                // If odd number of preceding backslashes, this one is escaped
                let is_escaped = num_preceding_backslashes % 2 == 1;

                if !is_escaped && chars[i + 1] == '"' {
                    // Unnecessary backslash before quote OUTSIDE string - skip it, keep quote
                    result.push('"');
                    i += 2;
                    continue;
                }
            }

            result.push(ch);
            i += 1;
        }

        result
    }

    /// Detects and extracts content from excessively nested structures.
    ///
    /// If the input has more than `max_depth` levels of nesting (e.g., `{{{{...}}}}`),
    /// this extracts the innermost valid JSON object or array.
    ///
    /// This prevents stack overflow errors from deeply nested structures.
    pub fn extract_from_deep_nesting(&self, input: &str, max_depth: usize) -> Option<String> {
        let trimmed = input.trim();

        // Quick check: does it even look nested?
        if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            return None;
        }

        // Count the depth of opening braces/brackets
        let mut max_brace_depth: usize = 0;
        let mut max_bracket_depth: usize = 0;
        let mut current_brace_depth: usize = 0;
        let mut current_bracket_depth: usize = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in trimmed.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => {
                    current_brace_depth += 1;
                    max_brace_depth = max_brace_depth.max(current_brace_depth);
                }
                '}' if !in_string => {
                    current_brace_depth = current_brace_depth.saturating_sub(1);
                }
                '[' if !in_string => {
                    current_bracket_depth += 1;
                    max_bracket_depth = max_bracket_depth.max(current_bracket_depth);
                }
                ']' if !in_string => {
                    current_bracket_depth = current_bracket_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        // Check if either depth exceeds the limit
        if max_brace_depth <= max_depth && max_bracket_depth <= max_depth {
            return None; // Not too deep, no extraction needed
        }

        // Extract the innermost actual JSON object/array
        // Strategy: Find the first position where we have actual content (not just delimiters)
        let chars: Vec<char> = trimmed.chars().collect();

        // Skip all opening delimiters to find start of actual content
        let mut start = 0;
        for (i, &ch) in chars.iter().enumerate() {
            match ch {
                '{' | '[' => {
                    // Keep looking
                    start = i + 1;
                }
                c if c.is_whitespace() => {
                    // Skip whitespace
                }
                _ => {
                    // Found actual content, backtrack to last delimiter
                    if i > 0 {
                        start = i - 1;
                        // Find the actual opening delimiter before any whitespace
                        while start > 0 && chars[start].is_whitespace() {
                            start -= 1;
                        }
                    }
                    break;
                }
            }
        }

        // Skip all closing delimiters from the end to find end of actual content
        let mut end = chars.len();
        for (i, &ch) in chars.iter().enumerate().rev() {
            match ch {
                '}' | ']' => {
                    // Keep looking
                    end = i;
                }
                c if c.is_whitespace() => {
                    // Skip whitespace
                }
                _ => {
                    // Found actual content, advance to next delimiter
                    if i + 1 < chars.len() {
                        end = i + 1;
                        // Find the actual closing delimiter after any whitespace
                        while end < chars.len() && chars[end].is_whitespace() {
                            end += 1;
                        }
                        if end < chars.len() {
                            end += 1; // Include the closing delimiter
                        }
                    }
                    break;
                }
            }
        }

        if start < end && end <= chars.len() {
            let extracted: String = chars[start..end].iter().collect();
            let trimmed_extracted = extracted.trim();

            // Validate it looks like JSON
            if (trimmed_extracted.starts_with('{') && trimmed_extracted.ends_with('}'))
                || (trimmed_extracted.starts_with('[') && trimmed_extracted.ends_with(']'))
            {
                return Some(extracted);
            }
        }

        None
    }

    /// Removes invisible characters that can break JSON parsing.
    ///
    /// Removes:
    /// - Zero-width space (U+200B)
    /// - Zero-width non-joiner (U+200C)
    /// - Zero-width joiner (U+200D)
    /// - Byte Order Mark (U+FEFF)
    /// - Other zero-width and control characters
    pub fn remove_invisible_chars(&self, input: &str) -> String {
        input.replace(
            [
                '\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}', '\u{200E}', '\u{200F}', '\u{202A}',
                '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}',
            ],
            "",
        ) // Right-to-left override
    }

    /// Handles double-escaped JSON (JSON serialized as a string).
    ///
    /// Converts: `"{\"name\": \"Alice\"}"` → `{"name": "Alice"}`
    /// Converts: `"[{\"id\": 1}]"` → `[{"id": 1}]`
    fn fix_double_escaped(&self, input: &str) -> String {
        let trimmed = input.trim();

        // Check if this looks like double-escaped JSON
        // Pattern: starts with " followed by { or [, ends with } or ] followed by "
        if trimmed.len() < 4 {
            return input.to_string();
        }

        let starts_with_quote_brace = trimmed.starts_with("\"{") || trimmed.starts_with("\"[");
        let ends_with_brace_quote = trimmed.ends_with("}\"") || trimmed.ends_with("]\"");

        if !starts_with_quote_brace || !ends_with_brace_quote {
            return input.to_string();
        }

        // Try to parse as a JSON string first
        match serde_json::from_str::<String>(trimmed) {
            Ok(unescaped) => {
                // Verify the unescaped version is valid JSON
                if serde_json::from_str::<serde_json::Value>(&unescaped).is_ok() {
                    unescaped
                } else {
                    input.to_string()
                }
            }
            Err(_) => input.to_string(),
        }
    }

    /// Normalizes excessive whitespace.
    fn normalize_whitespace(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut in_string = false;
        let mut escape_next = false;
        let mut last_was_space = false;

        for ch in input.chars() {
            if escape_next {
                result.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => {
                    escape_next = true;
                    result.push(ch);
                }
                '"' => {
                    in_string = !in_string;
                    last_was_space = false;
                    result.push(ch);
                }
                _ if in_string => {
                    last_was_space = false;
                    result.push(ch);
                }
                _ if ch.is_whitespace() && !in_string => {
                    if !last_was_space {
                        result.push(' '); // Normalize all whitespace to space
                        last_was_space = true;
                    }
                }
                _ => {
                    last_was_space = false;
                    result.push(ch);
                }
            }
        }

        result.trim().to_string()
    }

    /// Normalizes field names to snake_case for fuzzy matching.
    ///
    /// DISABLED: Field normalization is now handled by the struct deserializer's
    /// fuzzy field matching (FieldMatcher). This preserves HashMap keys correctly
    /// while still allowing flexible struct field matching.
    ///
    /// Only normalizes if the input is valid JSON. If not valid JSON,
    /// returns the input unchanged (fixes will handle it later).
    pub fn normalize_field_names(&self, input: &str) -> String {
        // DISABLED: Return input unchanged to preserve HashMap keys
        // Field matching is handled by struct deserializer's FieldMatcher
        input.to_string()

        // Original implementation (disabled):
        // Try to parse as JSON first
        // let value: serde_json::Value = match serde_json::from_str(input) {
        //     Ok(v) => v,
        //     Err(_) => return input.to_string(), // Not valid JSON, return as-is
        // };
        //
        // // Recursively normalize all field names
        // let (normalized_value, modified) = Self::normalize_value(value);
        //
        // // Only re-serialize if field names were actually changed
        // if modified {
        //     match serde_json::to_string(&normalized_value) {
        //         Ok(result) => result,
        //         Err(_) => input.to_string(),
        //     }
        // } else {
        //     input.to_string()
        // }
    }
}

impl Cleaner for GarbageCleaner {
    fn name(&self) -> &'static str {
        "garbage"
    }

    fn clean(&self, candidate: &Candidate) -> Result<Option<Candidate>> {
        // OPTIMIZATION: Apply cleaning steps sequentially and track if any changes were made
        // This avoids unnecessary string allocations if no cleaning is needed
        let mut content = &candidate.content;
        let mut owned_content: Option<String> = None;

        // Step -2: Remove invisible characters (very first!)
        let invisible_cleaned = self.remove_invisible_chars(content);
        if invisible_cleaned != *content {
            owned_content = Some(invisible_cleaned);
            content = owned_content.as_ref().unwrap();
        }

        // Step -1: Fix unnecessary backslashes
        let backslash_cleaned = self.fix_unnecessary_backslashes(content);
        if backslash_cleaned != *content {
            owned_content = Some(backslash_cleaned);
            content = owned_content.as_ref().unwrap();
        }

        // Step 0: Handle double-escaped JSON (must be early!)
        let double_escape_cleaned = self.fix_double_escaped(content);
        if double_escape_cleaned != *content {
            owned_content = Some(double_escape_cleaned);
            content = owned_content.as_ref().unwrap();
        }

        // Step 1: Clean multiple commas
        let comma_cleaned = self.clean_multiple_commas(content);
        if comma_cleaned != *content {
            owned_content = Some(comma_cleaned);
            content = owned_content.as_ref().unwrap();
        }

        // Step 2: Normalize whitespace
        let ws_cleaned = self.normalize_whitespace(content);
        if ws_cleaned != *content {
            owned_content = Some(ws_cleaned);
            content = owned_content.as_ref().unwrap();
        }

        // Step 3: Normalize field names (only if valid JSON)
        let field_cleaned = self.normalize_field_names(content);
        if field_cleaned != *content {
            owned_content = Some(field_cleaned);
        }

        // OPTIMIZATION: Only clone the candidate if we made changes
        if let Some(cleaned_content) = owned_content {
            let mut cleaned = candidate.clone();
            cleaned.content = cleaned_content;
            Ok(Some(cleaned))
        } else {
            // No changes made, return None to indicate original should be used
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::CandidateSource;

    #[test]
    fn test_clean_multiple_commas() {
        let cleaner = GarbageCleaner::new();
        let input = r#"{"name": "Alice",,,,, "age": 30}"#;
        let expected = r#"{"name": "Alice", "age": 30}"#;

        let result = cleaner.clean_multiple_commas(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_clean_multiple_commas_in_string() {
        let cleaner = GarbageCleaner::new();
        let input = r#"{"text": "Hello,,,,world"}"#;

        let result = cleaner.clean_multiple_commas(input);
        // Commas inside strings should be preserved
        assert_eq!(result, input);
    }

    #[test]
    fn test_normalize_whitespace() {
        let cleaner = GarbageCleaner::new();
        let input = "  {  \"name\"  :   \"Alice\"  ,  \"age\"  :  30  }  ";
        let expected = r#"{ "name" : "Alice" , "age" : 30 }"#;

        let result = cleaner.normalize_whitespace(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_cleaner_trait() {
        let cleaner = GarbageCleaner::new();
        let candidate = Candidate {
            content: r#"{"name": "Alice",,,, "age": 30}"#.to_string(),
            source: CandidateSource::Direct,
        };

        let result = cleaner.clean(&candidate).unwrap();
        assert!(result.is_some());

        let cleaned = result.unwrap();
        assert_eq!(cleaned.content, r#"{"name": "Alice", "age": 30}"#);
    }

    #[test]
    fn test_no_cleaning_needed() {
        let cleaner = GarbageCleaner::new();
        let candidate = Candidate {
            content: r#"{"name": "Alice", "age": 30}"#.to_string(),
            source: CandidateSource::Direct,
        };

        let result = cleaner.clean(&candidate).unwrap();
        assert!(result.is_none()); // No changes needed
    }

    #[test]
    fn test_fix_unnecessary_backslashes() {
        let cleaner = GarbageCleaner::new();
        let input = r#"{\"name\": \"Alice\", \"age\": 30}"#;

        println!("Input: {}", input);
        println!("Input contains backslash: {}", input.contains('\\'));

        let result = cleaner.fix_unnecessary_backslashes(input);
        println!("Result: {}", result);

        assert_eq!(result, r#"{"name": "Alice", "age": 30}"#);

        // Verify result is valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&result).is_ok());
    }

    #[test]
    fn test_extract_from_deep_nesting() {
        let cleaner = GarbageCleaner::new();

        // Test with 10 levels of nesting (exceeds max_depth of 3)
        let input = format!(
            "{}{{\"name\": \"Alice\", \"age\": 30}}{}",
            "{".repeat(10),
            "}".repeat(10)
        );

        println!("Input (first 50 chars): {}", &input[..50.min(input.len())]);
        let result = cleaner.extract_from_deep_nesting(&input, 3);
        println!("Result: {:?}", result);

        assert!(result.is_some());
        let extracted = result.unwrap();
        println!("Extracted: {}", extracted);

        // Should extract the inner content
        assert!(extracted.contains("\"name\""));
        assert!(extracted.contains("Alice"));

        // Verify it's valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&extracted).is_ok());
    }

    #[test]
    fn test_extract_from_deep_nesting_not_too_deep() {
        let cleaner = GarbageCleaner::new();

        // Test with 2 levels (does not exceed max_depth of 3)
        let input = r#"{{"name": "Alice"}}"#;

        let result = cleaner.extract_from_deep_nesting(input, 3);

        // Should return None (not too deep)
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_from_very_deep_nesting() {
        let cleaner = GarbageCleaner::new();

        // Test with 100 levels (like the brutal_reality test)
        let input = format!(
            "{}{{\"name\": \"Alice\", \"age\": 30}}{}",
            "{".repeat(100),
            "}".repeat(100)
        );

        let result = cleaner.extract_from_deep_nesting(&input, 50);

        assert!(result.is_some());
        let extracted = result.unwrap();

        // Should extract valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 30);
    }
}
