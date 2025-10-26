//! Parsing strategies for extracting JSON from messy LLM responses.

mod direct_json;
mod markdown;
mod multiple_objects;

#[cfg(feature = "yaml")]
mod yaml;

mod extractor;
mod heuristic;
mod json_fixer;
mod raw_primitive;
mod state_machine_strategy;

pub use direct_json::DirectJsonStrategy;
pub use extractor::{DirectExtractor, Extractor, HeuristicExtractor, MarkdownExtractor};
pub use heuristic::HeuristicStrategy;
pub use json_fixer::JsonFixerStrategy;
pub use markdown::MarkdownStrategy;
pub use multiple_objects::MultipleObjectsStrategy;
pub use raw_primitive::RawPrimitiveStrategy;
pub use state_machine_strategy::StateMachineStrategy;
#[cfg(feature = "yaml")]
pub use yaml::YamlStrategy;

use crate::{error::Result, value::FlexValue};

/// Trait for parsing strategies that extract JSON from text.
///
/// Each strategy represents a different way to find and parse JSON
/// content from potentially messy LLM responses.
pub trait ParsingStrategy: Send + Sync + std::fmt::Debug {
    /// Returns the name of this strategy for debugging.
    fn name(&self) -> &'static str;

    /// Attempts to parse the input using this strategy.
    ///
    /// Returns a vector of candidate values, or an error if parsing
    /// completely failed. An empty vector indicates the strategy is
    /// not applicable to this input.
    fn parse(&self, input: &str) -> Result<Vec<FlexValue>>;

    /// Returns the priority of this strategy.
    ///
    /// Lower values are tried first. This allows fast strategies
    /// like direct JSON parsing to run before expensive ones like
    /// JSON repair.
    fn priority(&self) -> u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_json_priority() {
        let strategy = DirectJsonStrategy;
        assert_eq!(strategy.priority(), 1);
    }

    #[test]
    fn test_strategy_name() {
        let strategy = DirectJsonStrategy;
        assert_eq!(strategy.name(), "direct_json");
    }
}
