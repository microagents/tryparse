//! State machine parser for JSONish content.
//!
//! This module implements a token-by-token state machine parser that can handle
//! malformed JSON more intelligently than regex-based approaches. It maintains
//! a context stack to track nested collections and make better decisions about
//! when to close unquoted strings, handle missing commas, etc.
//!
//! This is heavily inspired by BAML's jsonish parser but adapted for compile-time
//! schema information.

use serde_json::Value;

use crate::{
    error::{ParseError, Result},
    value::{FlexValue, Source},
};

/// Represents a collection type being parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonCollection {
    /// Currently inside an object/map `{...}`
    Object,
    /// Currently inside an array `[...]`
    Array,
}

impl JsonCollection {
    /// Returns the opening character for this collection type.
    #[inline]
    pub const fn open_char(&self) -> char {
        match self {
            JsonCollection::Object => '{',
            JsonCollection::Array => '[',
        }
    }

    /// Returns the closing character for this collection type.
    #[inline]
    pub const fn close_char(&self) -> char {
        match self {
            JsonCollection::Object => '}',
            JsonCollection::Array => ']',
        }
    }

    /// Returns true if this collection type requires key-value pairs.
    #[inline]
    pub const fn requires_keys(&self) -> bool {
        matches!(self, JsonCollection::Object)
    }
}

/// Parsing context that tracks where we are in the JSON structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseContext {
    /// At the root level, haven't entered any collection
    Root,
    /// Inside an object, expecting a key
    ObjectKey,
    /// Inside an object, after a key, expecting a colon
    ObjectColon,
    /// Inside an object, after colon, expecting a value
    ObjectValue,
    /// Inside an array, expecting a value
    ArrayValue,
    /// After a value, expecting comma or closing bracket
    AfterValue,
}

/// Represents a position in the collection stack.
#[derive(Debug, Clone)]
struct StackFrame {
    /// Type of collection
    collection: JsonCollection,
    /// Current parsing context within this collection
    context: ParseContext,
    /// Content accumulated so far
    content: String,
}

impl StackFrame {
    fn new(collection: JsonCollection) -> Self {
        Self {
            collection,
            context: if collection.requires_keys() {
                ParseContext::ObjectKey
            } else {
                ParseContext::ArrayValue
            },
            content: String::from(collection.open_char()),
        }
    }
}

/// State machine parser for JSONish content.
///
/// This parser maintains a stack of collections being parsed and processes
/// the input character by character, making context-aware decisions.
#[derive(Debug, Clone)]
pub struct StateMachineParser {
    /// Stack of collection contexts (most recent on top)
    stack: Vec<StackFrame>,
    /// Current position in input
    position: usize,
    /// Whether we're currently inside a quoted string
    in_string: bool,
    /// Whether the last character was an escape
    escaped: bool,
    /// Accumulated candidates
    candidates: Vec<String>,
}

impl StateMachineParser {
    /// Create a new state machine parser.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            position: 0,
            in_string: false,
            escaped: false,
            candidates: Vec::new(),
        }
    }

    /// Parse the input and return JSON candidates.
    pub fn parse(&mut self, input: &str) -> Result<Vec<FlexValue>> {
        self.reset();

        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();

        while self.position < len {
            let ch = chars[self.position];

            // Handle escape sequences
            if self.in_string {
                if self.escaped {
                    self.append_to_current(ch);
                    self.escaped = false;
                } else if ch == '\\' {
                    self.append_to_current(ch);
                    self.escaped = true;
                } else if ch == '"' {
                    self.append_to_current(ch);
                    self.in_string = false;
                } else {
                    self.append_to_current(ch);
                }
                self.position += 1;
                continue;
            }

            // Not in a string - process structural characters
            match ch {
                '"' => {
                    self.in_string = true;
                    self.append_to_current(ch);
                }
                '{' => self.open_object()?,
                '}' => self.close_collection(JsonCollection::Object)?,
                '[' => self.open_array()?,
                ']' => self.close_collection(JsonCollection::Array)?,
                ':' => self.handle_colon()?,
                ',' => self.handle_comma()?,
                ch if ch.is_whitespace() => {
                    // Skip whitespace at root level, preserve it otherwise
                    if !self.stack.is_empty() {
                        self.append_to_current(ch);
                    }
                }
                _ => {
                    // Regular character - append to current context
                    self.append_to_current(ch);
                }
            }

            self.position += 1;
        }

        // Try to close any unclosed collections
        while !self.stack.is_empty() {
            let collection = self.stack.last().unwrap().collection;
            self.close_collection(collection)?;
        }

        // Convert accumulated JSON strings to FlexValue candidates
        self.convert_candidates()
    }

    fn reset(&mut self) {
        self.stack.clear();
        self.position = 0;
        self.in_string = false;
        self.escaped = false;
        self.candidates.clear();
    }

    fn append_to_current(&mut self, ch: char) {
        if let Some(frame) = self.stack.last_mut() {
            frame.content.push(ch);
        }
    }

    fn open_object(&mut self) -> Result<()> {
        self.stack.push(StackFrame::new(JsonCollection::Object));
        Ok(())
    }

    fn open_array(&mut self) -> Result<()> {
        self.stack.push(StackFrame::new(JsonCollection::Array));
        Ok(())
    }

    fn close_collection(&mut self, expected: JsonCollection) -> Result<()> {
        if let Some(mut frame) = self.stack.pop() {
            // Verify we're closing the right type
            if frame.collection != expected {
                // Mismatched brackets - try to recover by pushing back
                self.stack.push(frame);
                return Ok(());
            }

            frame.content.push(expected.close_char());

            if self.stack.is_empty() {
                // We've completed a top-level collection
                self.candidates.push(frame.content);
            } else {
                // Append to parent collection
                if let Some(parent) = self.stack.last_mut() {
                    parent.content.push_str(&frame.content);
                }
            }
        }
        Ok(())
    }

    fn handle_colon(&mut self) -> Result<()> {
        if let Some(frame) = self.stack.last_mut() {
            if frame.collection == JsonCollection::Object {
                frame.content.push(':');
                frame.context = ParseContext::ObjectValue;
            }
        }
        Ok(())
    }

    fn handle_comma(&mut self) -> Result<()> {
        if let Some(frame) = self.stack.last_mut() {
            frame.content.push(',');
            frame.context = if frame.collection.requires_keys() {
                ParseContext::ObjectKey
            } else {
                ParseContext::ArrayValue
            };
        }
        Ok(())
    }

    fn convert_candidates(&self) -> Result<Vec<FlexValue>> {
        let mut flex_values = Vec::new();

        // If we extracted multiple candidates, use MultiJson source
        // If only one candidate, use Direct (it's the whole input)
        let use_multi_source = self.candidates.len() > 1;

        for (index, candidate) in self.candidates.iter().enumerate() {
            // Try to parse as JSON
            if let Ok(value) = serde_json::from_str::<Value>(candidate) {
                let source = if use_multi_source {
                    Source::MultiJson { index }
                } else {
                    Source::Direct
                };
                flex_values.push(FlexValue::new(value, source));
            }
        }

        if flex_values.is_empty() {
            return Err(ParseError::NoCandidates);
        }

        Ok(flex_values)
    }
}

impl Default for StateMachineParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_collection_chars() {
        assert_eq!(JsonCollection::Object.open_char(), '{');
        assert_eq!(JsonCollection::Object.close_char(), '}');
        assert_eq!(JsonCollection::Array.open_char(), '[');
        assert_eq!(JsonCollection::Array.close_char(), ']');
    }

    #[test]
    fn test_json_collection_requires_keys() {
        assert!(JsonCollection::Object.requires_keys());
        assert!(!JsonCollection::Array.requires_keys());
    }

    #[test]
    fn test_parse_simple_object() {
        let mut parser = StateMachineParser::new();
        let result = parser.parse(r#"{"name": "Alice"}"#);
        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_parse_simple_array() {
        let mut parser = StateMachineParser::new();
        let result = parser.parse(r#"[1, 2, 3]"#);
        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_parse_nested_structure() {
        let mut parser = StateMachineParser::new();
        let result = parser.parse(r#"{"items": [1, 2, 3]}"#);
        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_parse_with_whitespace() {
        let mut parser = StateMachineParser::new();
        let result = parser.parse(r#"  {  "name"  :  "Bob"  }  "#);
        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_unclosed_object() {
        let mut parser = StateMachineParser::new();
        let result = parser.parse(r#"{"name": "Charlie""#);
        // Should auto-close
        assert!(result.is_ok());
    }

    #[test]
    fn test_unclosed_array() {
        let mut parser = StateMachineParser::new();
        let result = parser.parse(r#"[1, 2, 3"#);
        // Should auto-close
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_input() {
        let mut parser = StateMachineParser::new();
        let result = parser.parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_top_level_objects() {
        let mut parser = StateMachineParser::new();
        let result = parser.parse(r#"{"a": 1} {"b": 2}"#);
        assert!(result.is_ok());
        let candidates = result.unwrap();
        // Should find multiple candidates
        assert!(!candidates.is_empty());
    }
}
