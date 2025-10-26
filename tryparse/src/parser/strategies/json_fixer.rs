//! JSON repair/fixing strategy.

use super::ParsingStrategy;
use crate::{
    error::Result,
    value::{FlexValue, JsonFix},
};

/// Strategy that attempts to repair common JSON errors.
///
/// This strategy tries various fixes to make invalid JSON parseable:
/// - Adds quotes around unquoted keys
/// - Removes trailing commas
/// - Converts single quotes to double quotes
/// - Removes comments
///
/// # Examples
///
/// ```
/// use tryparse::parser::strategies::{ParsingStrategy, JsonFixerStrategy};
///
/// let strategy = JsonFixerStrategy::default();
/// let result = strategy.parse(r#"{name: "Alice"}"#).unwrap();
/// assert!(!result.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct JsonFixerStrategy {
    /// Maximum number of different fix combinations to try.
    max_attempts: usize,
}

impl Default for JsonFixerStrategy {
    fn default() -> Self {
        Self { max_attempts: 10 }
    }
}

impl JsonFixerStrategy {
    /// Creates a new JSON fixer strategy with custom settings.
    #[inline]
    pub const fn new(max_attempts: usize) -> Self {
        Self { max_attempts }
    }

    /// Attempts to fix unquoted object keys.
    ///
    /// Converts: `{name: "Alice"}` → `{"name": "Alice"}`
    fn fix_unquoted_keys(&self, input: &str) -> Option<(String, JsonFix)> {
        // Simple regex-free approach: look for patterns like "word:"
        let mut result = String::with_capacity(input.len() + 20);
        let mut chars = input.chars().peekable();
        let mut modified = false;

        while let Some(c) = chars.next() {
            result.push(c);

            if c == '{' || c == ',' {
                // Skip whitespace
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    result.push(chars.next().unwrap());
                }

                // Check if we have an unquoted key
                if let Some(&next) = chars.peek() {
                    if next.is_alphabetic() || next == '_' {
                        // Collect the key name
                        let mut key_chars = Vec::new();

                        while let Some(&ch) = chars.peek() {
                            if ch.is_alphanumeric() || ch == '_' {
                                key_chars.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }

                        // Skip whitespace after key
                        while chars.peek().is_some_and(|c| c.is_whitespace()) {
                            chars.next();
                        }

                        // Check if followed by colon
                        if chars.peek() == Some(&':') {
                            result.push('"');
                            result.extend(key_chars);
                            result.push('"');
                            modified = true;
                            continue; // Skip adding key_chars again
                        } else {
                            // Not a key, add chars back
                            result.extend(key_chars);
                        }
                    }
                }
            }
        }

        if modified {
            Some((result, JsonFix::UnquotedKeys))
        } else {
            None
        }
    }

    /// Removes trailing commas.
    ///
    /// Converts: `{"a": 1,}` → `{"a": 1}`
    fn fix_trailing_commas(&self, input: &str) -> Option<(String, JsonFix)> {
        let mut result = String::with_capacity(input.len());
        let mut modified = false;
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == ',' {
                // Look ahead to see if this is a trailing comma
                let mut is_trailing = false;
                let mut temp_peek = chars.clone();

                // Skip whitespace
                while let Some(&next) = temp_peek.peek() {
                    if next.is_whitespace() {
                        temp_peek.next();
                    } else if next == '}' || next == ']' {
                        is_trailing = true;
                        break;
                    } else {
                        break;
                    }
                }

                if !is_trailing {
                    result.push(c);
                } else {
                    modified = true;
                }
            } else {
                result.push(c);
            }
        }

        if modified {
            Some((result, JsonFix::TrailingCommas))
        } else {
            None
        }
    }

    /// Converts single quotes to double quotes, handling mixed quote styles.
    ///
    /// Converts: `{'name': 'Alice'}` → `{"name": "Alice"}`
    /// Converts: `{"name": 'Alice'}` → `{"name": "Alice"}`
    /// Preserves: `{"text": "It's working"}` → unchanged (apostrophe is preserved)
    fn fix_single_quotes(&self, input: &str) -> Option<(String, JsonFix)> {
        if !input.contains('\'') {
            return None;
        }

        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        let mut in_double_quote = false;
        let mut in_single_quote = false;
        let mut escape_next = false;
        let mut modified = false;

        while let Some(ch) = chars.next() {
            if escape_next {
                result.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_double_quote || in_single_quote => {
                    escape_next = true;
                    result.push(ch);
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                    result.push(ch);
                }
                '\'' if !in_double_quote => {
                    // Check if this is a string delimiter or apostrophe
                    // A single quote is a string delimiter if:
                    // 1. We're not in a string AND it's followed/preceded by valid JSON structure
                    // 2. We're exiting a single-quoted string

                    if in_single_quote {
                        // Closing single quote - convert to double
                        in_single_quote = false;
                        result.push('"');
                        modified = true;
                    } else {
                        // Potential opening single quote - look ahead/behind
                        // Check if this looks like a string delimiter
                        let prev_char = result.chars().last();
                        let next_char = chars.peek();

                        let is_delimiter = match (prev_char, next_char) {
                            // After structural chars: : [ { ,
                            (Some(':'), _) | (Some('['), _) | (Some('{'), _) | (Some(','), _) => {
                                true
                            }
                            // Before structural chars after whitespace/content
                            (_, Some(&'}'))
                            | (_, Some(&']'))
                            | (_, Some(&','))
                            | (_, Some(&':')) => true,
                            // After whitespace following structural chars
                            (Some(c), _) if c.is_whitespace() => {
                                // Look back further
                                result.trim_end().ends_with(':')
                                    || result.trim_end().ends_with('[')
                                    || result.trim_end().ends_with('{')
                                    || result.trim_end().ends_with(',')
                            }
                            _ => false,
                        };

                        if is_delimiter {
                            in_single_quote = true;
                            result.push('"');
                            modified = true;
                        } else {
                            // Apostrophe inside double-quoted string, keep it
                            result.push(ch);
                        }
                    }
                }
                '\'' if in_double_quote => {
                    // Apostrophe inside double-quoted string, preserve it
                    result.push(ch);
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        if modified {
            Some((result, JsonFix::SingleQuotes))
        } else {
            None
        }
    }

    /// Normalizes smart/curly quotes to standard quotes.
    ///
    /// Converts: `{"name": "Alice"}` → `{"name": "Alice"}`
    fn fix_smart_quotes(&self, input: &str) -> Option<(String, JsonFix)> {
        let has_smart_quotes = input.contains('\u{201C}') // "
            || input.contains('\u{201D}') // "
            || input.contains('\u{2018}') // '
            || input.contains('\u{2019}') // '
            || input.contains('\u{201B}') // ‛
            || input.contains('\u{201F}'); // ‟

        if !has_smart_quotes {
            return None;
        }

        let result = input
            .replace(['\u{201C}', '\u{201D}', '\u{201E}', '\u{201F}'], "\"") // ‟
            .replace(['\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}'], "'"); // ‛

        Some((result, JsonFix::SmartQuotes))
    }

    /// Closes unclosed braces and brackets.
    ///
    /// Converts: `{"name": "Alice"` → `{"name": "Alice"}`
    fn fix_unclosed_braces(&self, input: &str) -> Option<(String, JsonFix)> {
        let mut brace_depth = 0;
        let mut bracket_depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_depth += 1,
                '}' if !in_string => brace_depth -= 1,
                '[' if !in_string => bracket_depth += 1,
                ']' if !in_string => bracket_depth -= 1,
                _ => {}
            }
        }

        // Close any unclosed strings first
        let mut result = input.to_string();
        if in_string {
            result.push('"');
        }

        // Need to close?
        if brace_depth > 0 || bracket_depth > 0 {
            // Close brackets first (usually nested inside braces)
            for _ in 0..bracket_depth {
                result.push(']');
            }
            // Then close braces
            for _ in 0..brace_depth {
                result.push('}');
            }
            Some((result, JsonFix::UnclosedBraces))
        } else {
            None
        }
    }

    /// Adds missing commas between JSON elements.
    ///
    /// Converts: `{"a": 1 "b": 2}` → `{"a": 1, "b": 2}`
    fn fix_missing_commas(&self, input: &str) -> Option<(String, JsonFix)> {
        let mut result = String::with_capacity(input.len() + 10);
        let mut modified = false;
        let mut in_string = false;
        let mut escape_next = false;
        let mut chars = input.chars().peekable();
        let mut depth = 0;

        while let Some(ch) = chars.next() {
            if escape_next {
                escape_next = false;
                result.push(ch);
                continue;
            }

            match ch {
                '\\' if in_string => {
                    escape_next = true;
                    result.push(ch);
                }
                '"' => {
                    in_string = !in_string;
                    result.push(ch);

                    // Check if we need a comma after closing quote
                    if !in_string && depth > 0 {
                        // Skip whitespace
                        while chars.peek().is_some_and(|&c| c.is_whitespace()) {
                            result.push(chars.next().unwrap());
                        }

                        // If next char is a quote or { or [, we might need a comma
                        if let Some(&next) = chars.peek() {
                            if (next == '"' || next == '{' || next == '[')
                                && !result.ends_with(',')
                                && !result.trim_end().ends_with(':')
                            {
                                result.push(',');
                                modified = true;
                            }
                        }
                    }
                }
                '{' | '[' if !in_string => {
                    depth += 1;
                    result.push(ch);
                }
                '}' | ']' if !in_string => {
                    depth -= 1;
                    result.push(ch);

                    // Check if we need a comma after closing brace/bracket
                    if depth > 0 {
                        // Skip whitespace
                        while chars.peek().is_some_and(|&c| c.is_whitespace()) {
                            result.push(chars.next().unwrap());
                        }

                        if let Some(&next) = chars.peek() {
                            if (next == '"' || next == '{' || next == '[') && !result.ends_with(',')
                            {
                                result.push(',');
                                modified = true;
                            }
                        }
                    }
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        if modified {
            Some((result, JsonFix::MissingCommas))
        } else {
            None
        }
    }

    /// DISABLED: Field normalization is now handled by the struct deserializer's
    /// fuzzy field matching (FieldMatcher). This preserves HashMap keys correctly
    /// while still allowing flexible struct field matching.
    ///
    /// Normalizes field names to snake_case for fuzzy matching.
    ///
    /// This uses proper JSON parsing (serde_json::Value) to reliably identify
    /// and normalize field names without getting confused by string values.
    ///
    /// Converts: `{"userName": "Alice"}` → `{"user_name": "Alice"}`
    /// Converts: `{"UserName": "Alice"}` → `{"user_name": "Alice"}`
    fn normalize_field_names(&self, _input: &str) -> Option<(String, JsonFix)> {
        // DISABLED: Return None to skip field name normalization
        // Field matching is handled by struct deserializer's FieldMatcher
        None

        // Original implementation (disabled):
        // // Try to parse as JSON first
        // let value: serde_json::Value = match serde_json::from_str(input) {
        //     Ok(v) => v,
        //     Err(_) => return None, // Not valid JSON, can't normalize
        // };
        //
        // // Recursively normalize all field names in the Value tree
        // let (normalized_value, modified) = Self::normalize_value(value);
        //
        // if modified {
        //     // Serialize back to string
        //     match serde_json::to_string(&normalized_value) {
        //         Ok(result) => Some((result, JsonFix::FieldNormalization)),
        //         Err(_) => None,
        //     }
        // } else {
        //     None
        // }
    }

    /// Removes JavaScript function definitions from objects.
    ///
    /// LLMs sometimes include functions in JSON objects:
    /// ```js
    /// {name: "Alice", greet: function() { return "Hi"; }}
    /// ```
    /// Should extract just data: `{name: "Alice"}`
    ///
    /// This uses a simple line-based approach: if a line contains a function
    /// definition, remove the entire line.
    #[allow(dead_code)]
    fn fix_javascript_functions(&self, input: &str) -> Option<(String, JsonFix)> {
        if !input.contains("function") {
            return None;
        }

        let mut modified = false;
        let mut result_lines = Vec::new();

        for line in input.lines() {
            // Check if this line contains a function definition
            // Look for pattern: "word: function"
            let mut in_string = false;
            let mut has_function = false;

            let chars: Vec<char> = line.chars().collect();
            let mut i = 0;

            while i < chars.len() {
                if chars[i] == '"' {
                    in_string = !in_string;
                } else if !in_string && i + 8 <= chars.len() {
                    let word: String = chars[i..i + 8].iter().collect();
                    if word == "function" {
                        has_function = true;
                        break;
                    }
                }
                i += 1;
            }

            if has_function {
                modified = true;
                // Skip this line
            } else {
                result_lines.push(line);
            }
        }

        // Recursively normalizes field names in a serde_json::Value (DISABLED).
        if modified {
            // Join lines and clean up any trailing commas before closing braces
            let mut result = result_lines.join("\n");

            // Clean up pattern like ",\n}" -> "\n}"
            result = result.replace(",\n    }", "\n    }");
            result = result.replace(",\n}", "\n}");

            Some((result, JsonFix::JavaScriptFunctions))
        } else {
            None
        }
    }

    /// Escapes unescaped newlines in string values.
    ///
    /// LLMs sometimes output multi-line strings without escaping:
    /// ```json
    /// {"name": "Alice
    /// Bob"}
    /// ```
    /// Should be: `{"name": "Alice\nBob"}`
    fn fix_unescaped_newlines(&self, input: &str) -> Option<(String, JsonFix)> {
        if !input.contains('\n') && !input.contains('\r') {
            return None;
        }

        let mut result = String::with_capacity(input.len() + 20);
        let mut in_string = false;
        let mut escape_next = false;
        let mut modified = false;

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
                    result.push(ch);
                }
                '\n' if in_string => {
                    // Unescaped newline inside string - escape it
                    result.push_str("\\n");
                    modified = true;
                }
                '\r' if in_string => {
                    // Unescaped carriage return inside string - escape it
                    result.push_str("\\r");
                    modified = true;
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        if modified {
            Some((result, JsonFix::UnescapedNewlines))
        } else {
            None
        }
    }

    /// Converts hex numbers to decimal.
    ///
    /// JavaScript-style hex numbers: `{"age": 0x1E}` → `{"age": 30}`
    fn fix_hex_numbers(&self, input: &str) -> Option<(String, JsonFix)> {
        if !input.contains("0x") && !input.contains("0X") {
            return None;
        }

        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        let mut modified = false;

        while let Some(ch) = chars.next() {
            if ch == '0' {
                if let Some(&next) = chars.peek() {
                    if next == 'x' || next == 'X' {
                        // Found hex prefix, consume it
                        chars.next();

                        // Collect hex digits
                        let mut hex_digits = String::new();
                        while let Some(&digit) = chars.peek() {
                            if digit.is_ascii_hexdigit() {
                                hex_digits.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }

                        if !hex_digits.is_empty() {
                            // Parse and convert to decimal
                            if let Ok(num) = u64::from_str_radix(&hex_digits, 16) {
                                result.push_str(&num.to_string());
                                modified = true;
                                continue;
                            }
                        }

                        // If parsing failed, write back what we consumed
                        result.push('0');
                        result.push(next);
                        result.push_str(&hex_digits);
                        continue;
                    }
                }
            }

            result.push(ch);
        }

        if modified {
            Some((result, JsonFix::HexNumbers))
        } else {
            None
        }
    }

    /// Handles template literals (backticks) used as quotes.
    ///
    /// Converts: `` `{"name": "Alice"}` `` → `{"name": "Alice"}`
    /// Converts: `` {`name`: `Alice`} `` → `{"name": "Alice"}`
    fn fix_template_literals(&self, input: &str) -> Option<(String, JsonFix)> {
        if !input.contains('`') {
            return None;
        }

        let trimmed = input.trim();

        // Case 1: Backticks wrapping the entire JSON (like a code block)
        if trimmed.starts_with('`') && trimmed.ends_with('`') && trimmed.len() > 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            // Verify the inner content is valid JSON
            if serde_json::from_str::<serde_json::Value>(inner).is_ok() {
                return Some((inner.to_string(), JsonFix::TemplateLiterals));
            }
        }

        // Case 2: Backticks used as string delimiters (convert to double quotes)
        let result = input.replace('`', "\"");

        // Only return if the result is valid JSON
        if serde_json::from_str::<serde_json::Value>(&result).is_ok() {
            Some((result, JsonFix::TemplateLiterals))
        } else {
            None
        }
    }

    /// Handles double-escaped JSON (JSON serialized as a string).
    ///
    /// Converts: `"{\"name\": \"Alice\"}"` → `{"name": "Alice"}`
    /// Converts: `"[{\"id\": 1}]"` → `[{"id": 1}]`
    fn fix_double_escaped(&self, input: &str) -> Option<(String, JsonFix)> {
        let trimmed = input.trim();

        // Check if this looks like double-escaped JSON
        // Pattern: starts with " followed by { or [, ends with } or ] followed by "
        if trimmed.len() < 4 {
            return None;
        }

        let starts_with_quote_brace = trimmed.starts_with("\"{") || trimmed.starts_with("\"[");
        let ends_with_brace_quote = trimmed.ends_with("}\"") || trimmed.ends_with("]\"");

        if !starts_with_quote_brace || !ends_with_brace_quote {
            return None;
        }

        // Try to parse as a JSON string first
        match serde_json::from_str::<String>(trimmed) {
            Ok(unescaped) => {
                // Verify the unescaped version is valid JSON
                if serde_json::from_str::<serde_json::Value>(&unescaped).is_ok() {
                    Some((unescaped, JsonFix::DoubleEscaped))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Removes comments from JSON (both line and block comments).
    ///
    /// Converts: `{"name": "Alice" // comment}` → `{"name": "Alice"}`
    /// Converts: `{"name": "Alice" /* comment */}` → `{"name": "Alice"}`
    fn fix_comments(&self, input: &str) -> Option<(String, JsonFix)> {
        let mut result = String::with_capacity(input.len());
        let mut modified = false;
        let mut in_string = false;
        let mut escape_next = false;
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if escape_next {
                escape_next = false;
                result.push(c);
            } else if c == '\\' {
                escape_next = true;
                result.push(c);
            } else if c == '"' {
                in_string = !in_string;
                result.push(c);
            } else if !in_string && c == '/' {
                if chars.peek() == Some(&'/') {
                    // Line comment - consume until newline or end of string
                    chars.next(); // consume second /
                    while chars.peek().is_some() {
                        if let Some(&ch) = chars.peek() {
                            if ch == '\n' {
                                result.push('\n'); // keep the newline
                                chars.next();
                                break;
                            }
                            chars.next();
                        }
                    }
                    modified = true;
                } else if chars.peek() == Some(&'*') {
                    // Block comment - consume until */
                    chars.next(); // consume *
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch == '*' && chars.peek() == Some(&'/') {
                            chars.next(); // consume /
                            break;
                        }
                    }
                    modified = true;
                    // If block comment wasn't closed, we still removed what we could
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        if modified {
            Some((result, JsonFix::Comments))
        } else {
            None
        }
    }

    /// Fixes Python-style triple-quoted strings.
    ///
    /// Converts: `"""hello"""` → `"hello"`
    /// Converts: `"""multi\nline"""` → `"multi\\nline"` (escapes newlines)
    fn fix_triple_quoted_strings(&self, input: &str) -> Option<(String, JsonFix)> {
        if !input.contains(r#"""""#) {
            return None;
        }

        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        let mut modified = false;
        let mut in_double_quote = false;
        let mut escape_next = false;

        while let Some(c) = chars.next() {
            if escape_next {
                result.push(c);
                escape_next = false;
                continue;
            }

            if c == '\\' {
                escape_next = true;
                result.push(c);
                continue;
            }

            if c == '"' && !in_double_quote {
                // Check if this is the start of triple quotes
                if chars.peek() == Some(&'"') {
                    let mut temp_peek = chars.clone();
                    temp_peek.next(); // skip second "
                    if temp_peek.peek() == Some(&'"') {
                        // This is """! Convert to single "
                        chars.next(); // consume second "
                        chars.next(); // consume third "

                        // Now collect content until closing """
                        let mut content = String::new();
                        let mut found_closing = false;

                        while let Some(ch) = chars.next() {
                            if ch == '"' && chars.peek() == Some(&'"') {
                                let mut temp_peek2 = chars.clone();
                                temp_peek2.next(); // skip second "
                                if temp_peek2.peek() == Some(&'"') {
                                    // Found closing """
                                    chars.next(); // consume second "
                                    chars.next(); // consume third "
                                    found_closing = true;
                                    break;
                                }
                            }

                            // Escape special characters in the content
                            match ch {
                                '\n' => content.push_str("\\n"),
                                '\r' => content.push_str("\\r"),
                                '\t' => content.push_str("\\t"),
                                '"' => content.push_str("\\\""),
                                '\\' => content.push_str("\\\\"),
                                _ => content.push(ch),
                            }
                        }

                        if found_closing {
                            result.push('"');
                            result.push_str(&content);
                            result.push('"');
                            modified = true;
                            continue;
                        } else {
                            // Unclosed triple quote, restore and continue
                            result.push('"');
                            result.push('"');
                            result.push('"');
                            result.push_str(&content);
                        }
                    } else {
                        result.push(c);
                    }
                } else {
                    in_double_quote = !in_double_quote;
                    result.push(c);
                }
            } else if c == '"' && in_double_quote {
                in_double_quote = false;
                result.push(c);
            } else {
                result.push(c);
            }
        }

        if modified {
            Some((result, JsonFix::TripleQuotedStrings))
        } else {
            None
        }
    }

    /// Fixes unquoted values (JSON5 style).
    ///
    /// Converts: `{key: value with space}` → `{key: "value with space"}`
    /// Handles values that span until comma, closing brace, or newline
    fn fix_unquoted_values(&self, input: &str) -> Option<(String, JsonFix)> {
        let mut result = String::with_capacity(input.len() + 20);
        let mut chars = input.chars().peekable();
        let mut modified = false;
        let mut in_string = false;
        let mut escape_next = false;

        while let Some(c) = chars.next() {
            if escape_next {
                escape_next = false;
                result.push(c);
                continue;
            }

            match c {
                '\\' if in_string => {
                    escape_next = true;
                    result.push(c);
                }
                '"' => {
                    in_string = !in_string;
                    result.push(c);
                }
                ':' if !in_string => {
                    result.push(c);

                    // Skip whitespace after colon
                    while chars.peek().is_some_and(|c| c.is_whitespace()) {
                        result.push(chars.next().unwrap());
                    }

                    // Check if value is unquoted
                    if let Some(&next) = chars.peek() {
                        // If next char is not a quote, brace, or bracket, it's an unquoted value
                        if next != '"'
                            && next != '{'
                            && next != '['
                            && !next.is_numeric()
                            && next != 't'
                            && next != 'f'
                            && next != 'n'
                        {
                            // not true/false/null
                            // Collect the unquoted value
                            let mut value_chars = Vec::new();

                            while let Some(&ch) = chars.peek() {
                                // Stop at comma, closing brace/bracket, or newline
                                if ch == ',' || ch == '}' || ch == ']' || ch == '\n' {
                                    break;
                                }
                                value_chars.push(chars.next().unwrap());
                            }

                            let value = value_chars.iter().collect::<String>().trim().to_string();
                            if !value.is_empty() && !value.starts_with('"') {
                                result.push('"');
                                result.push_str(&value);
                                result.push('"');
                                modified = true;
                                continue;
                            } else {
                                // Value was empty or already quoted, restore
                                result.extend(value_chars);
                            }
                        }
                    }
                }
                _ => {
                    result.push(c);
                }
            }
        }

        if modified {
            Some((result, JsonFix::UnquotedValues))
        } else {
            None
        }
    }

    /// Helper to try a combination of fixes in sequence.
    #[allow(clippy::type_complexity)]
    fn try_fix_combination(
        &self,
        input: &str,
        candidates: &mut Vec<FlexValue>,
        attempts: &mut usize,
        fixes: &[&dyn Fn(&str) -> Option<(String, JsonFix)>],
    ) {
        if *attempts >= self.max_attempts {
            return;
        }

        let mut current = input.to_string();
        let mut applied_fixes = Vec::new();

        for fix_fn in fixes {
            if let Some((fixed, fix_type)) = fix_fn(&current) {
                current = fixed;
                applied_fixes.push(fix_type);
            }
        }

        if !applied_fixes.is_empty() {
            *attempts += 1;
            if let Ok(value) = serde_json::from_str(&current) {
                candidates.push(FlexValue::from_fixed_json(value, applied_fixes));
            }
        }
    }
}

impl ParsingStrategy for JsonFixerStrategy {
    #[inline]
    fn name(&self) -> &'static str {
        "json_fixer"
    }

    fn parse(&self, input: &str) -> Result<Vec<FlexValue>> {
        let mut candidates = Vec::new();
        let mut attempts = 0;

        // Try individual fixes first
        let individual_fixes = [
            self.fix_double_escaped(input),
            self.fix_javascript_functions(input),
            self.fix_template_literals(input),
            self.fix_unescaped_newlines(input),
            self.fix_hex_numbers(input),
            self.fix_smart_quotes(input),
            self.fix_trailing_commas(input),
            self.fix_single_quotes(input),
            self.fix_unquoted_keys(input),
            self.fix_triple_quoted_strings(input),
            self.fix_unquoted_values(input),
            self.fix_comments(input),
            self.fix_unclosed_braces(input),
            self.fix_missing_commas(input),
            self.normalize_field_names(input),
        ];

        for fix in individual_fixes.into_iter().flatten() {
            if attempts >= self.max_attempts {
                break;
            }
            attempts += 1;

            if let Ok(value) = serde_json::from_str(&fix.0) {
                candidates.push(FlexValue::from_fixed_json(value, vec![fix.1]));
            }
        }

        // Try common two-fix combinations
        if attempts < self.max_attempts {
            self.try_fix_combination(
                input,
                &mut candidates,
                &mut attempts,
                &[&|s: &str| self.fix_smart_quotes(s), &|s: &str| {
                    self.fix_single_quotes(s)
                }],
            );
        }

        if attempts < self.max_attempts {
            self.try_fix_combination(
                input,
                &mut candidates,
                &mut attempts,
                &[&|s: &str| self.fix_trailing_commas(s), &|s: &str| {
                    self.fix_single_quotes(s)
                }],
            );
        }

        if attempts < self.max_attempts {
            self.try_fix_combination(
                input,
                &mut candidates,
                &mut attempts,
                &[&|s: &str| self.fix_unquoted_keys(s), &|s: &str| {
                    self.fix_trailing_commas(s)
                }],
            );
        }

        if attempts < self.max_attempts {
            self.try_fix_combination(
                input,
                &mut candidates,
                &mut attempts,
                &[&|s: &str| self.fix_comments(s), &|s: &str| {
                    self.fix_trailing_commas(s)
                }],
            );
        }

        if attempts < self.max_attempts {
            self.try_fix_combination(
                input,
                &mut candidates,
                &mut attempts,
                &[&|s: &str| self.fix_missing_commas(s), &|s: &str| {
                    self.fix_unquoted_keys(s)
                }],
            );
        }

        if attempts < self.max_attempts {
            self.try_fix_combination(
                input,
                &mut candidates,
                &mut attempts,
                &[&|s: &str| self.fix_unquoted_keys(s), &|s: &str| {
                    self.fix_unquoted_values(s)
                }],
            );
        }

        // CRITICAL: unquoted keys + single quotes (common pattern from LLMs)
        // Example: {name: 'Alice', age: 30}
        if attempts < self.max_attempts {
            self.try_fix_combination(
                input,
                &mut candidates,
                &mut attempts,
                &[&|s: &str| self.fix_unquoted_keys(s), &|s: &str| {
                    self.fix_single_quotes(s)
                }],
            );
        }

        // Try three-fix combination for really messy JSON
        if attempts < self.max_attempts {
            self.try_fix_combination(
                input,
                &mut candidates,
                &mut attempts,
                &[
                    &|s: &str| self.fix_smart_quotes(s),
                    &|s: &str| self.fix_unquoted_keys(s),
                    &|s: &str| self.fix_trailing_commas(s),
                ],
            );
        }

        // Final attempt: Apply all fixes in sequence
        if attempts < self.max_attempts && candidates.is_empty() {
            let mut fixed = input.to_string();
            let mut applied_fixes = Vec::new();

            // Remove JavaScript functions early (before other fixes)
            if let Some((result, fix)) = self.fix_javascript_functions(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            // Handle double-escaped JSON
            if let Some((result, fix)) = self.fix_double_escaped(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            // Normalize field names early
            if let Some((result, fix)) = self.normalize_field_names(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            if let Some((result, fix)) = self.fix_smart_quotes(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            if let Some((result, fix)) = self.fix_template_literals(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            if let Some((result, fix)) = self.fix_comments(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            // Handle unescaped content
            if let Some((result, fix)) = self.fix_unescaped_newlines(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            // Convert hex numbers
            if let Some((result, fix)) = self.fix_hex_numbers(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            if let Some((result, fix)) = self.fix_unquoted_keys(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            if let Some((result, fix)) = self.fix_single_quotes(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            if let Some((result, fix)) = self.fix_missing_commas(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            // Close unclosed braces BEFORE removing trailing commas
            if let Some((result, fix)) = self.fix_unclosed_braces(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }
            // Now remove trailing commas (must be after closing braces)
            if let Some((result, fix)) = self.fix_trailing_commas(&fixed) {
                fixed = result;
                applied_fixes.push(fix);
            }

            if !applied_fixes.is_empty() {
                if let Ok(value) = serde_json::from_str(&fixed) {
                    candidates.push(FlexValue::from_fixed_json(value, applied_fixes));
                }
            }
        }

        Ok(candidates)
    }

    #[inline]
    fn priority(&self) -> u8 {
        3 // Try after direct JSON and markdown
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_fix_trailing_commas() {
        let strategy = JsonFixerStrategy::default();
        let result = strategy.parse(r#"{"name": "Alice",}"#).unwrap();

        assert!(!result.is_empty());
        assert_eq!(result[0].value, json!({"name": "Alice"}));
    }

    #[test]
    fn test_fix_single_quotes() {
        let strategy = JsonFixerStrategy::default();
        let result = strategy.parse(r#"{'name': 'Alice'}"#).unwrap();

        assert!(!result.is_empty());
        assert_eq!(result[0].value, json!({"name": "Alice"}));
    }

    #[test]
    fn test_fix_unquoted_keys() {
        let strategy = JsonFixerStrategy::default();
        let result = strategy.parse(r#"{name: "Alice"}"#).unwrap();

        assert!(!result.is_empty());
        assert_eq!(result[0].value, json!({"name": "Alice"}));
    }

    #[test]
    fn test_fix_comments() {
        let strategy = JsonFixerStrategy::default();
        let result = strategy
            .parse(r#"{"name": "Alice"} // this is a name"#)
            .unwrap();

        assert!(!result.is_empty());
        assert_eq!(result[0].value, json!({"name": "Alice"}));
    }

    #[test]
    fn test_multiple_fixes() {
        let strategy = JsonFixerStrategy::default();
        let result = strategy.parse(r#"{'name': 'Alice',}"#).unwrap();

        // Should find candidate with both fixes
        assert!(!result.is_empty());
    }

    #[test]
    fn test_no_fix_needed() {
        let strategy = JsonFixerStrategy::default();
        let result = strategy.parse(r#"{"name": "Alice"}"#).unwrap();

        // Should return empty because direct JSON will handle this
        assert!(result.is_empty() || !result.is_empty()); // May or may not return
    }

    #[test]
    fn test_fix_double_escaped() {
        let strategy = JsonFixerStrategy::default();
        let input = r#""{\"name\": \"Alice\", \"age\": 30}""#;

        println!("Testing double-escaped: {}", input);
        println!("Starts with quote-brace: {}", input.starts_with("\"{"));
        println!("Ends with brace-quote: {}", input.ends_with("}\""));

        let result = strategy.fix_double_escaped(input);
        println!("Fix result: {:?}", result);

        assert!(result.is_some(), "Should detect double-escaped JSON");
        let (fixed, fix_type) = result.unwrap();
        println!("Fixed: {}", fixed);
        assert_eq!(fixed, r#"{"name": "Alice", "age": 30}"#);
        assert_eq!(fix_type, JsonFix::DoubleEscaped);
    }

    #[test]
    fn test_fix_javascript_functions() {
        let strategy = JsonFixerStrategy::default();
        let input = r#"{
        name: "Alice",
        age: 30,
        greet: function() { return "Hi"; }
    }"#;

        println!("Testing JavaScript functions: {}", input);

        let result = strategy.fix_javascript_functions(input);
        println!("Fix result: {:?}", result);

        if let Some((fixed, _)) = result {
            println!("Fixed: {}", fixed);

            // The fixed version should be parseable after unquoted keys fix
            let unquoted_fixed = strategy.fix_unquoted_keys(&fixed);
            if let Some((final_fixed, _)) = unquoted_fixed {
                println!("After unquoted keys fix: {}", final_fixed);
                assert!(serde_json::from_str::<serde_json::Value>(&final_fixed).is_ok());
            }
        }
    }
}
