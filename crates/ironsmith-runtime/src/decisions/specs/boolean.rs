//! Boolean (yes/no) decision specifications.
//!
//! These specs are for decisions where the player chooses yes or no,
//! such as "may" effects, paying ward costs, casting with miracle, etc.

use crate::decision::FallbackStrategy;
use crate::decisions::context::{BooleanContext, DecisionContext};
use crate::decisions::spec::{DecisionPrimitive, DecisionSpec};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::targeting::WardCost;

// ============================================================================
// MaySpec - Optional "may" actions
// ============================================================================

/// Specification for a "may" choice - player can optionally perform an action.
#[derive(Debug, Clone)]
pub struct MaySpec {
    /// Description of what the player may do.
    pub description: String,
    /// The source of the effect (for display).
    pub source: ObjectId,
}

impl MaySpec {
    /// Create a new MaySpec.
    pub fn new(source: ObjectId, description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            source,
        }
    }
}

impl DecisionSpec for MaySpec {
    type Response = bool;

    fn description(&self) -> String {
        format!("You may {}", self.description)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Boolean
    }

    fn default_response(&self, strategy: FallbackStrategy) -> bool {
        matches!(strategy, FallbackStrategy::Accept)
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        DecisionContext::Boolean(BooleanContext::new(
            player,
            Some(self.source),
            self.description.clone(),
        ))
    }
}

// ============================================================================
// WardSpec - Ward cost payment
// ============================================================================

/// Specification for paying a ward cost.
#[derive(Debug, Clone)]
pub struct WardSpec {
    /// The spell or ability that would be countered.
    pub source: ObjectId,
    /// The permanent with ward being targeted.
    pub target: ObjectId,
    /// The ward cost.
    pub cost: WardCost,
    /// Description of the ward cost.
    pub description: String,
}

impl WardSpec {
    /// Create a new WardSpec.
    pub fn new(
        source: ObjectId,
        target: ObjectId,
        cost: WardCost,
        description: impl Into<String>,
    ) -> Self {
        Self {
            source,
            target,
            cost,
            description: description.into(),
        }
    }
}

impl DecisionSpec for WardSpec {
    type Response = bool;

    fn description(&self) -> String {
        format!("Pay ward cost: {}", self.description)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Boolean
    }

    fn default_response(&self, strategy: FallbackStrategy) -> bool {
        // Default is to decline ward payment (spell gets countered)
        matches!(strategy, FallbackStrategy::Accept)
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        DecisionContext::Boolean(BooleanContext::new(
            player,
            Some(self.source),
            format!("Pay ward cost: {}", self.description),
        ))
    }
}

// ============================================================================
// MiracleSpec - Miracle trigger
// ============================================================================

/// Specification for a miracle trigger - player drew a miracle card as first draw.
#[derive(Debug, Clone)]
pub struct MiracleSpec {
    /// The card with miracle.
    pub card: ObjectId,
    /// The miracle cost.
    pub miracle_cost: ManaCost,
}

impl MiracleSpec {
    /// Create a new MiracleSpec.
    pub fn new(card: ObjectId, miracle_cost: ManaCost) -> Self {
        Self { card, miracle_cost }
    }
}

impl DecisionSpec for MiracleSpec {
    type Response = bool;

    fn description(&self) -> String {
        "Cast with Miracle cost?".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Boolean
    }

    fn default_response(&self, strategy: FallbackStrategy) -> bool {
        // Default is to decline miracle (keep in hand)
        matches!(strategy, FallbackStrategy::Accept)
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        DecisionContext::Boolean(BooleanContext::new(
            player,
            Some(self.card),
            "Cast with Miracle cost?",
        ))
    }
}

// ============================================================================
// MadnessSpec - Madness trigger
// ============================================================================

/// Specification for a madness trigger - discarded card with madness.
#[derive(Debug, Clone)]
pub struct MadnessSpec {
    /// The card with madness (in exile).
    pub card: ObjectId,
    /// The madness cost.
    pub madness_cost: ManaCost,
}

impl MadnessSpec {
    /// Create a new MadnessSpec.
    pub fn new(card: ObjectId, madness_cost: ManaCost) -> Self {
        Self { card, madness_cost }
    }
}

impl DecisionSpec for MadnessSpec {
    type Response = bool;

    fn description(&self) -> String {
        "Cast with Madness cost?".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Boolean
    }

    fn default_response(&self, strategy: FallbackStrategy) -> bool {
        // Default is to decline madness (card goes to graveyard)
        matches!(strategy, FallbackStrategy::Accept)
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        DecisionContext::Boolean(BooleanContext::new(
            player,
            Some(self.card),
            "Cast with Madness cost?",
        ))
    }
}

// ============================================================================
// AssignDamageAsUnblockedSpec - Thorn Elemental-style damage assignment
// ============================================================================

/// Specification for choosing whether to assign combat damage as though unblocked.
/// Used by creatures with the MayAssignDamageAsUnblocked ability (like Thorn Elemental).
#[derive(Debug, Clone)]
pub struct AssignDamageAsUnblockedSpec {
    /// The attacking creature.
    pub attacker: ObjectId,
    /// Name of the attacker (for display).
    pub attacker_name: String,
}

impl AssignDamageAsUnblockedSpec {
    /// Create a new AssignDamageAsUnblockedSpec.
    pub fn new(attacker: ObjectId, attacker_name: impl Into<String>) -> Self {
        Self {
            attacker,
            attacker_name: attacker_name.into(),
        }
    }
}

impl DecisionSpec for AssignDamageAsUnblockedSpec {
    type Response = bool;

    fn description(&self) -> String {
        format!(
            "Assign {}'s damage to defending player instead of blockers?",
            self.attacker_name
        )
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Boolean
    }

    fn default_response(&self, strategy: FallbackStrategy) -> bool {
        // Default is to assign damage normally to blockers
        matches!(strategy, FallbackStrategy::Accept)
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        DecisionContext::Boolean(
            BooleanContext::new(
                player,
                Some(self.attacker),
                format!(
                    "Assign {}'s damage to defending player instead of blockers?",
                    self.attacker_name
                ),
            )
            .with_source_name(&self.attacker_name),
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana::ManaSymbol;

    #[test]
    fn test_may_spec() {
        let source = ObjectId::from_raw(1);
        let spec = MaySpec::new(source, "draw a card");

        assert_eq!(spec.description(), "You may draw a card");
        assert!(matches!(spec.primitive(), DecisionPrimitive::Boolean));
        assert!(!spec.default_response(FallbackStrategy::Decline));
        assert!(spec.default_response(FallbackStrategy::Accept));
    }

    #[test]
    fn test_miracle_spec() {
        let card = ObjectId::from_raw(1);
        let cost = ManaCost::from_symbols(vec![ManaSymbol::White]);
        let spec = MiracleSpec::new(card, cost);

        assert_eq!(spec.description(), "Cast with Miracle cost?");
        assert!(!spec.default_response(FallbackStrategy::Decline));
    }

    #[test]
    fn test_assign_damage_as_unblocked_spec() {
        let attacker = ObjectId::from_raw(1);
        let spec = AssignDamageAsUnblockedSpec::new(attacker, "Thorn Elemental");

        assert!(
            spec.description()
                .contains("Thorn Elemental's damage to defending player")
        );
        assert!(!spec.default_response(FallbackStrategy::Decline));
    }
}
