//! Combat decision specifications.
//!
//! These specs are for combat-related decisions: declaring attackers,
//! declaring blockers, and ordering attackers/blockers for damage assignment.

use crate::decision::FallbackStrategy;
use crate::decisions::context::{
    AttackerOptionContext, AttackersContext, BlockerOptionContext, BlockersContext,
    DecisionContext, OrderContext,
};
use crate::decisions::spec::{
    AttackerDeclaration, AttackerOption, BlockerDeclaration, BlockerOption, DecisionPrimitive,
    DecisionSpec,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

// ============================================================================
// DeclareAttackersSpec - Declare which creatures attack
// ============================================================================

/// Specification for declaring attackers.
#[derive(Debug, Clone)]
pub struct DeclareAttackersSpec {
    /// Creatures that can attack with their valid targets.
    pub attacker_options: Vec<AttackerOption>,
}

impl DeclareAttackersSpec {
    /// Create a new DeclareAttackersSpec.
    pub fn new(attacker_options: Vec<AttackerOption>) -> Self {
        Self { attacker_options }
    }
}

impl DecisionSpec for DeclareAttackersSpec {
    type Response = Vec<AttackerDeclaration>;

    fn description(&self) -> String {
        "Declare attackers".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::DeclareAttackers
    }

    fn default_response(&self, strategy: FallbackStrategy) -> Vec<AttackerDeclaration> {
        match strategy {
            FallbackStrategy::Maximum => {
                // Attack with everything that can attack
                self.attacker_options
                    .iter()
                    .filter_map(|opt| {
                        opt.valid_targets.first().map(|target| AttackerDeclaration {
                            creature: opt.creature,
                            target: target.clone(),
                        })
                    })
                    .collect()
            }
            FallbackStrategy::Minimum => {
                // Only attack with creatures that must attack
                self.attacker_options
                    .iter()
                    .filter(|opt| opt.must_attack)
                    .filter_map(|opt| {
                        opt.valid_targets.first().map(|target| AttackerDeclaration {
                            creature: opt.creature,
                            target: target.clone(),
                        })
                    })
                    .collect()
            }
            _ => Vec::new(), // Default: don't attack
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let attacker_options: Vec<AttackerOptionContext> = self
            .attacker_options
            .iter()
            .map(|opt| {
                let creature_name = game
                    .object(opt.creature)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                AttackerOptionContext {
                    creature: opt.creature,
                    creature_name,
                    valid_targets: opt.valid_targets.clone(),
                    must_attack: opt.must_attack,
                }
            })
            .collect();

        DecisionContext::Attackers(AttackersContext::new(player, attacker_options))
    }
}

// ============================================================================
// DeclareBlockersSpec - Declare which creatures block
// ============================================================================

/// Specification for declaring blockers.
#[derive(Debug, Clone)]
pub struct DeclareBlockersSpec {
    /// Options for each attacker showing valid blockers.
    pub blocker_options: Vec<BlockerOption>,
}

impl DeclareBlockersSpec {
    /// Create a new DeclareBlockersSpec.
    pub fn new(blocker_options: Vec<BlockerOption>) -> Self {
        Self { blocker_options }
    }
}

impl DecisionSpec for DeclareBlockersSpec {
    type Response = Vec<BlockerDeclaration>;

    fn description(&self) -> String {
        "Declare blockers".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::DeclareBlockers
    }

    fn default_response(&self, strategy: FallbackStrategy) -> Vec<BlockerDeclaration> {
        match strategy {
            FallbackStrategy::Maximum => {
                // Block with everything that can block (first valid blocker per attacker)
                self.blocker_options
                    .iter()
                    .filter_map(|opt| {
                        opt.valid_blockers
                            .first()
                            .map(|&blocker| BlockerDeclaration {
                                blocker,
                                blocking: opt.attacker,
                            })
                    })
                    .collect()
            }
            _ => Vec::new(), // Default: don't block
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let blocker_options: Vec<BlockerOptionContext> = self
            .blocker_options
            .iter()
            .map(|opt| {
                let attacker_name = game
                    .object(opt.attacker)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                let valid_blockers: Vec<(ObjectId, String)> = opt
                    .valid_blockers
                    .iter()
                    .map(|&id| {
                        let name = game
                            .object(id)
                            .map(|o| o.name.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        (id, name)
                    })
                    .collect();
                BlockerOptionContext {
                    attacker: opt.attacker,
                    attacker_name,
                    valid_blockers,
                    min_blockers: opt.min_blockers,
                }
            })
            .collect();

        DecisionContext::Blockers(BlockersContext::new(player, blocker_options))
    }
}

// ============================================================================
// OrderBlockersSpec - Order blockers for damage assignment
// ============================================================================

/// Specification for ordering blockers.
/// When multiple creatures block one attacker, the attacking player orders
/// them to determine damage assignment order.
#[derive(Debug, Clone)]
pub struct OrderBlockersSpec {
    /// The attacking creature.
    pub attacker: ObjectId,
    /// The blockers to order.
    pub blockers: Vec<ObjectId>,
}

impl OrderBlockersSpec {
    /// Create a new OrderBlockersSpec.
    pub fn new(attacker: ObjectId, blockers: Vec<ObjectId>) -> Self {
        Self { attacker, blockers }
    }
}

impl DecisionSpec for OrderBlockersSpec {
    type Response = Vec<ObjectId>;

    fn description(&self) -> String {
        "Order blockers for damage assignment".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Order
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> Vec<ObjectId> {
        // Default: keep original order
        self.blockers.clone()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let items: Vec<(ObjectId, String)> = self
            .blockers
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (id, name)
            })
            .collect();

        DecisionContext::Order(OrderContext::new(
            player,
            Some(self.attacker),
            "Order blockers for damage assignment",
            items,
        ))
    }
}

// ============================================================================
// OrderAttackersSpec - Order attackers for damage assignment
// ============================================================================

/// Specification for ordering attackers.
/// When one creature blocks multiple attackers (banding, etc.), the defending
/// player orders them to determine damage assignment order.
#[derive(Debug, Clone)]
pub struct OrderAttackersSpec {
    /// The blocking creature.
    pub blocker: ObjectId,
    /// The attackers to order.
    pub attackers: Vec<ObjectId>,
}

impl OrderAttackersSpec {
    /// Create a new OrderAttackersSpec.
    pub fn new(blocker: ObjectId, attackers: Vec<ObjectId>) -> Self {
        Self { blocker, attackers }
    }
}

impl DecisionSpec for OrderAttackersSpec {
    type Response = Vec<ObjectId>;

    fn description(&self) -> String {
        "Order attackers for damage assignment".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Order
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> Vec<ObjectId> {
        // Default: keep original order
        self.attackers.clone()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let items: Vec<(ObjectId, String)> = self
            .attackers
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (id, name)
            })
            .collect();

        DecisionContext::Order(OrderContext::new(
            player,
            Some(self.blocker),
            "Order attackers for damage assignment",
            items,
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_state::AttackTarget;
    use crate::ids::PlayerId;

    #[test]
    fn test_declare_attackers_spec_empty() {
        let spec = DeclareAttackersSpec::new(vec![]);

        assert!(spec.default_response(FallbackStrategy::Decline).is_empty());
        assert!(spec.default_response(FallbackStrategy::Maximum).is_empty());
    }

    #[test]
    fn test_declare_attackers_spec_with_options() {
        let creature = ObjectId::from_raw(1);
        let target = AttackTarget::Player(PlayerId::from_index(1));
        let options = vec![AttackerOption {
            creature,
            valid_targets: vec![target],
            must_attack: false,
        }];
        let spec = DeclareAttackersSpec::new(options);

        // Default is to not attack
        assert!(spec.default_response(FallbackStrategy::Decline).is_empty());

        // Maximum attacks with everything
        let max = spec.default_response(FallbackStrategy::Maximum);
        assert_eq!(max.len(), 1);
        assert_eq!(max[0].creature, creature);
    }

    #[test]
    fn test_declare_attackers_must_attack() {
        let creature = ObjectId::from_raw(1);
        let target = AttackTarget::Player(PlayerId::from_index(1));
        let options = vec![AttackerOption {
            creature,
            valid_targets: vec![target],
            must_attack: true,
        }];
        let spec = DeclareAttackersSpec::new(options);

        // Minimum strategy attacks with must-attack creatures
        let min = spec.default_response(FallbackStrategy::Minimum);
        assert_eq!(min.len(), 1);
    }

    #[test]
    fn test_declare_blockers_spec() {
        let attacker = ObjectId::from_raw(1);
        let blocker = ObjectId::from_raw(2);
        let options = vec![BlockerOption {
            attacker,
            valid_blockers: vec![blocker],
            min_blockers: 0,
        }];
        let spec = DeclareBlockersSpec::new(options);

        // Default is to not block
        assert!(spec.default_response(FallbackStrategy::Decline).is_empty());

        // Maximum blocks with everything
        let max = spec.default_response(FallbackStrategy::Maximum);
        assert_eq!(max.len(), 1);
        assert_eq!(max[0].blocker, blocker);
        assert_eq!(max[0].blocking, attacker);
    }

    #[test]
    fn test_order_blockers_spec() {
        let attacker = ObjectId::from_raw(1);
        let blockers = vec![ObjectId::from_raw(2), ObjectId::from_raw(3)];
        let spec = OrderBlockersSpec::new(attacker, blockers.clone());

        assert_eq!(spec.description(), "Order blockers for damage assignment");
        assert_eq!(
            spec.default_response(FallbackStrategy::FirstOption),
            blockers
        );
    }

    #[test]
    fn test_order_attackers_spec() {
        let blocker = ObjectId::from_raw(1);
        let attackers = vec![ObjectId::from_raw(2), ObjectId::from_raw(3)];
        let spec = OrderAttackersSpec::new(blocker, attackers.clone());

        assert_eq!(spec.description(), "Order attackers for damage assignment");
        assert_eq!(
            spec.default_response(FallbackStrategy::FirstOption),
            attackers
        );
    }
}
