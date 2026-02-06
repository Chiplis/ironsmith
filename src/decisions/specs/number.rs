//! Numeric decision specifications.
//!
//! These specs are for decisions where the player chooses a number within a range,
//! such as X value for spells or "choose a number" effects.

use crate::decision::FallbackStrategy;
use crate::decisions::context::{DecisionContext, NumberContext};
use crate::decisions::spec::{DecisionPrimitive, DecisionSpec};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

// ============================================================================
// XValueSpec - X value for spells
// ============================================================================

/// Specification for choosing X value in a spell cost.
#[derive(Debug, Clone)]
pub struct XValueSpec {
    /// The spell being cast.
    pub source: ObjectId,
    /// Maximum X value the player can afford.
    pub max_x: u32,
}

impl XValueSpec {
    /// Create a new XValueSpec.
    pub fn new(source: ObjectId, max_x: u32) -> Self {
        Self { source, max_x }
    }
}

impl DecisionSpec for XValueSpec {
    type Response = u32;

    fn description(&self) -> String {
        format!("Choose value for X (0-{})", self.max_x)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectNumber {
            min: 0,
            max: self.max_x,
        }
    }

    fn default_response(&self, strategy: FallbackStrategy) -> u32 {
        match strategy {
            FallbackStrategy::Minimum => 0,
            FallbackStrategy::Maximum => self.max_x,
            _ => 0, // Default to minimum for X
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        DecisionContext::Number(NumberContext::new(
            player,
            Some(self.source),
            0,
            self.max_x,
            format!("Choose value for X (0-{})", self.max_x),
        ))
    }
}

// ============================================================================
// NumberSpec - General number selection
// ============================================================================

/// Specification for choosing a number within a range.
/// Used for "choose a number" effects and "up to X" effects.
#[derive(Debug, Clone)]
pub struct NumberSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Minimum value (inclusive).
    pub min: u32,
    /// Maximum value (inclusive).
    pub max: u32,
    /// Description of what the number represents.
    pub description: String,
}

impl NumberSpec {
    /// Create a new NumberSpec.
    pub fn new(source: ObjectId, min: u32, max: u32, description: impl Into<String>) -> Self {
        Self {
            source,
            min,
            max,
            description: description.into(),
        }
    }

    /// Create a spec for "up to X" effects (0 to max).
    pub fn up_to(source: ObjectId, max: u32, description: impl Into<String>) -> Self {
        Self::new(source, 0, max, description)
    }

    /// Create a spec for choosing exactly within a range.
    pub fn range(source: ObjectId, min: u32, max: u32, description: impl Into<String>) -> Self {
        Self::new(source, min, max, description)
    }
}

impl DecisionSpec for NumberSpec {
    type Response = u32;

    fn description(&self) -> String {
        if self.min == 0 {
            format!("{} (up to {})", self.description, self.max)
        } else {
            format!("{} ({}-{})", self.description, self.min, self.max)
        }
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectNumber {
            min: self.min,
            max: self.max,
        }
    }

    fn default_response(&self, strategy: FallbackStrategy) -> u32 {
        match strategy {
            FallbackStrategy::Minimum => self.min,
            FallbackStrategy::Maximum => self.max,
            FallbackStrategy::FirstOption => self.min,
            _ => self.max, // Default to maximum for "up to" effects
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        DecisionContext::Number(NumberContext::new(
            player,
            Some(self.source),
            self.min,
            self.max,
            self.description.clone(),
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x_value_spec() {
        let source = ObjectId::from_raw(1);
        let spec = XValueSpec::new(source, 5);

        assert_eq!(spec.description(), "Choose value for X (0-5)");
        assert!(matches!(
            spec.primitive(),
            DecisionPrimitive::SelectNumber { min: 0, max: 5 }
        ));
        assert_eq!(spec.default_response(FallbackStrategy::Minimum), 0);
        assert_eq!(spec.default_response(FallbackStrategy::Maximum), 5);
    }

    #[test]
    fn test_number_spec_up_to() {
        let source = ObjectId::from_raw(1);
        let spec = NumberSpec::up_to(source, 3, "creatures to target");

        assert!(spec.description().contains("up to 3"));
        assert!(matches!(
            spec.primitive(),
            DecisionPrimitive::SelectNumber { min: 0, max: 3 }
        ));
    }

    #[test]
    fn test_number_spec_range() {
        let source = ObjectId::from_raw(1);
        let spec = NumberSpec::range(source, 1, 6, "die roll");

        assert!(spec.description().contains("1-6"));
        assert_eq!(spec.default_response(FallbackStrategy::Minimum), 1);
    }
}
