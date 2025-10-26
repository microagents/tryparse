//! Constraint validation system for LLM-parsed values.
//!
//! This module provides runtime validation similar to BAML's `@assert` and `@check` annotations.
//!
//! ## Design
//!
//! Constraints are validated during deserialization and can either:
//! - **Assert** (`@assert`): Fail deserialization if the constraint fails
//! - **Check** (`@check`): Track the result but don't fail deserialization
//!
//! ## Example
//!
//! ```
//! use tryparse::constraints::{Constraint, ConstraintLevel, ConstraintResult};
//!
//! // Define a constraint that age must be positive
//! let age_positive = Constraint::new(
//!     ConstraintLevel::Assert,
//!     "age_positive",
//!     "age must be greater than 0"
//! );
//!
//! // In your deserializer, validate the constraint
//! let age: i64 = 25;
//! let result = age_positive.validate(age > 0);
//! assert!(result.passed());
//! ```

use std::fmt;

/// Level of constraint enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConstraintLevel {
    /// Must pass or deserialization fails.
    ///
    /// Equivalent to BAML's `@assert`.
    Assert,

    /// Result is tracked but doesn't fail deserialization.
    ///
    /// Equivalent to BAML's `@check`.
    Check,
}

/// A constraint that validates a condition.
///
/// Constraints can be used to validate parsed values at runtime,
/// ensuring data quality beyond type checking.
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Enforcement level (assert vs check).
    pub level: ConstraintLevel,

    /// Unique identifier for this constraint.
    pub name: String,

    /// Human-readable description of what this constraint checks.
    pub description: String,
}

impl Constraint {
    /// Creates a new constraint.
    pub fn new(
        level: ConstraintLevel,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            level,
            name: name.into(),
            description: description.into(),
        }
    }

    /// Creates an assert-level constraint.
    ///
    /// This constraint will fail deserialization if it doesn't pass.
    pub fn assert(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(ConstraintLevel::Assert, name, description)
    }

    /// Creates a check-level constraint.
    ///
    /// This constraint is tracked but doesn't fail deserialization.
    pub fn check(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(ConstraintLevel::Check, name, description)
    }

    /// Validates a boolean condition.
    ///
    /// Returns a `ConstraintResult` indicating success or failure.
    pub fn validate(&self, passed: bool) -> ConstraintResult {
        ConstraintResult {
            constraint: self.clone(),
            passed,
        }
    }

    /// Returns true if this is an assert-level constraint.
    pub const fn is_assert(&self) -> bool {
        matches!(self.level, ConstraintLevel::Assert)
    }

    /// Returns true if this is a check-level constraint.
    pub const fn is_check(&self) -> bool {
        matches!(self.level, ConstraintLevel::Check)
    }
}

/// Result of a constraint validation.
#[derive(Debug, Clone)]
pub struct ConstraintResult {
    /// The constraint that was validated.
    pub constraint: Constraint,

    /// Whether the constraint passed.
    pub passed: bool,
}

impl ConstraintResult {
    /// Returns true if the constraint passed.
    pub const fn passed(&self) -> bool {
        self.passed
    }

    /// Returns true if the constraint failed.
    pub const fn failed(&self) -> bool {
        !self.passed
    }

    /// Returns true if this is a failing assert.
    pub const fn is_failing_assert(&self) -> bool {
        self.constraint.is_assert() && !self.passed
    }
}

impl fmt::Display for ConstraintResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        write!(
            f,
            "[{:?}] {}: {} ({})",
            self.constraint.level, status, self.constraint.name, self.constraint.description
        )
    }
}

/// Collection of constraint validation results.
#[derive(Debug, Clone, Default)]
pub struct ConstraintResults {
    results: Vec<ConstraintResult>,
}

impl ConstraintResults {
    /// Creates a new empty constraint results collection.
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Adds a constraint result.
    pub fn add(&mut self, result: ConstraintResult) {
        self.results.push(result);
    }

    /// Returns all constraint results.
    pub fn all(&self) -> &[ConstraintResult] {
        &self.results
    }

    /// Returns true if all asserts passed.
    #[inline]
    pub fn all_asserts_passed(&self) -> bool {
        self.results
            .iter()
            .filter(|r| r.constraint.is_assert())
            .all(|r| r.passed)
    }

    /// Returns all failing asserts.
    pub fn failing_asserts(&self) -> Vec<&ConstraintResult> {
        self.results
            .iter()
            .filter(|r| r.is_failing_assert())
            .collect()
    }

    /// Returns all results for check-level constraints.
    pub fn checks(&self) -> Vec<&ConstraintResult> {
        self.results
            .iter()
            .filter(|r| r.constraint.is_check())
            .collect()
    }

    /// Returns true if there are any constraints.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Returns the number of constraints.
    #[inline]
    pub fn len(&self) -> usize {
        self.results.len()
    }
}

impl fmt::Display for ConstraintResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "No constraints");
        }

        writeln!(f, "Constraint Results ({} total):", self.len())?;
        for result in &self.results {
            writeln!(f, "  {}", result)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_creation() {
        let constraint = Constraint::assert("age_positive", "age must be > 0");
        assert_eq!(constraint.level, ConstraintLevel::Assert);
        assert_eq!(constraint.name, "age_positive");
        assert!(constraint.is_assert());
        assert!(!constraint.is_check());
    }

    #[test]
    fn test_constraint_validation_pass() {
        let constraint = Constraint::assert("test", "test constraint");
        let result = constraint.validate(true);
        assert!(result.passed());
        assert!(!result.failed());
    }

    #[test]
    fn test_constraint_validation_fail() {
        let constraint = Constraint::assert("test", "test constraint");
        let result = constraint.validate(false);
        assert!(!result.passed());
        assert!(result.failed());
        assert!(result.is_failing_assert());
    }

    #[test]
    fn test_constraint_results() {
        let mut results = ConstraintResults::new();
        assert!(results.is_empty());

        let c1 = Constraint::assert("c1", "first constraint");
        let c2 = Constraint::check("c2", "second constraint");

        results.add(c1.validate(true));
        results.add(c2.validate(false));

        assert_eq!(results.len(), 2);
        assert!(results.all_asserts_passed());
        assert_eq!(results.checks().len(), 1);
    }

    #[test]
    fn test_failing_asserts() {
        let mut results = ConstraintResults::new();

        let c1 = Constraint::assert("c1", "should pass");
        let c2 = Constraint::assert("c2", "should fail");
        let c3 = Constraint::check("c3", "check that fails");

        results.add(c1.validate(true));
        results.add(c2.validate(false));
        results.add(c3.validate(false));

        assert!(!results.all_asserts_passed());
        assert_eq!(results.failing_asserts().len(), 1);
        assert_eq!(results.failing_asserts()[0].constraint.name, "c2");
    }

    #[test]
    fn test_constraint_result_display() {
        let constraint = Constraint::assert("age_check", "age must be positive");
        let result = constraint.validate(false);
        let display = format!("{}", result);
        assert!(display.contains("FAIL"));
        assert!(display.contains("age_check"));
        assert!(display.contains("age must be positive"));
    }

    #[test]
    fn test_constraint_results_display() {
        let mut results = ConstraintResults::new();
        results.add(Constraint::assert("c1", "test1").validate(true));
        results.add(Constraint::check("c2", "test2").validate(false));

        let display = format!("{}", results);
        assert!(display.contains("2 total"));
    }
}
