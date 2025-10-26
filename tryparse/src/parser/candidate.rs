//! Intermediate candidate representation for multi-stage parsing.

use crate::value::Source;

/// A candidate substring extracted from the input.
///
/// This represents a potential JSON value before it has been successfully
/// parsed. Candidates can be extracted from prose, code blocks, etc., and
/// then go through fixing and parsing stages.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// The extracted content (potential JSON)
    pub content: String,

    /// How this candidate was extracted
    pub source: CandidateSource,
}

/// Describes how a candidate was extracted from the input.
#[derive(Debug, Clone, PartialEq)]
pub enum CandidateSource {
    /// Direct input (no extraction needed)
    Direct,

    /// Extracted using heuristic brace matching
    Heuristic { pattern: String },

    /// Extracted from markdown code block
    Markdown { language: Option<String> },

    /// Extracted using regex or other pattern
    Pattern { pattern: String },
}

impl Candidate {
    /// Creates a new candidate from direct input
    pub fn direct(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            source: CandidateSource::Direct,
        }
    }

    /// Creates a new candidate from heuristic extraction
    pub fn heuristic(content: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            source: CandidateSource::Heuristic {
                pattern: pattern.into(),
            },
        }
    }

    /// Creates a new candidate from markdown extraction
    pub fn markdown(content: impl Into<String>, language: Option<String>) -> Self {
        Self {
            content: content.into(),
            source: CandidateSource::Markdown { language },
        }
    }

    /// Converts this candidate source to a FlexValue source
    pub fn to_source(&self) -> Source {
        match &self.source {
            CandidateSource::Direct => Source::Direct,
            CandidateSource::Heuristic { pattern } => Source::Heuristic {
                pattern: pattern.clone(),
            },
            CandidateSource::Markdown { language } => Source::Markdown {
                lang: language.clone(),
            },
            CandidateSource::Pattern { pattern } => Source::Heuristic {
                pattern: pattern.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_candidate() {
        let candidate = Candidate::direct(r#"{"name": "Alice"}"#);
        assert_eq!(candidate.content, r#"{"name": "Alice"}"#);
        assert_eq!(candidate.source, CandidateSource::Direct);
    }

    #[test]
    fn test_heuristic_candidate() {
        let candidate = Candidate::heuristic(r#"{"name": "Alice"}"#, "object");
        assert_eq!(
            candidate.source,
            CandidateSource::Heuristic {
                pattern: "object".to_string()
            }
        );
    }

    #[test]
    fn test_to_source() {
        let candidate = Candidate::heuristic("test", "pattern");
        let source = candidate.to_source();
        assert!(matches!(source, Source::Heuristic { .. }));
    }
}
