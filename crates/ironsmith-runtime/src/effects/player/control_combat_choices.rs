//! Combat-choice control effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Lets the source controller choose attackers and/or blockers this turn.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlCombatChoicesThisTurnEffect {
    pub attackers: bool,
    pub blockers: bool,
}

impl ControlCombatChoicesThisTurnEffect {
    pub fn new(attackers: bool, blockers: bool) -> Self {
        Self {
            attackers,
            blockers,
        }
    }
}

impl EffectExecutor for ControlCombatChoicesThisTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.add_combat_choice_control(ctx.controller, self.attackers, self.blockers);
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_state::AttackTarget;
    use crate::decision::{DecisionMaker, DecisionRouter};
    use crate::decisions::context::{
        AttackerOptionContext, AttackersContext, BlockerOptionContext, BlockersContext,
    };
    use crate::decisions::spec::{AttackerDeclaration, BlockerDeclaration};
    use crate::executor::ExecutionContext;
    use crate::ids::PlayerId;

    #[derive(Default)]
    struct AttackBlockDm {
        attackers: Vec<AttackerDeclaration>,
        blockers: Vec<BlockerDeclaration>,
    }

    impl DecisionMaker for AttackBlockDm {
        fn decide_attackers(
            &mut self,
            _game: &GameState,
            _ctx: &AttackersContext,
        ) -> Vec<AttackerDeclaration> {
            self.attackers.clone()
        }

        fn decide_blockers(
            &mut self,
            _game: &GameState,
            _ctx: &BlockersContext,
        ) -> Vec<BlockerDeclaration> {
            self.blockers.clone()
        }
    }

    #[test]
    fn control_combat_choices_effect_registers_until_end_of_turn() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ControlCombatChoicesThisTurnEffect::new(true, true);
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(game.combat_choice_controller_for_attackers(), Some(alice));
        assert_eq!(game.combat_choice_controller_for_blockers(), Some(alice));

        game.cleanup_combat_choice_control_end_of_turn();
        assert_eq!(game.combat_choice_controller_for_attackers(), None);
        assert_eq!(game.combat_choice_controller_for_blockers(), None);
    }

    #[test]
    fn decision_router_uses_combat_choice_controller_for_attackers_and_blockers() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = game.new_object_id();
        let blocker = game.new_object_id();

        game.add_combat_choice_control(alice, true, true);

        let expected_attackers = vec![AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(alice),
        }];
        let expected_blockers = vec![BlockerDeclaration {
            blocker,
            blocking: attacker,
        }];

        let alice_dm = AttackBlockDm {
            attackers: expected_attackers.clone(),
            blockers: expected_blockers.clone(),
        };
        let bob_dm = AttackBlockDm::default();

        let mut router = DecisionRouter::new(Box::new(alice_dm)).with_player(bob, Box::new(bob_dm));
        let attackers_ctx = AttackersContext::new(
            bob,
            vec![AttackerOptionContext {
                creature: attacker,
                creature_name: "Attacker".to_string(),
                valid_targets: Vec::new(),
                must_attack: false,
            }],
        );
        let blockers_ctx = BlockersContext::new(
            bob,
            vec![BlockerOptionContext {
                attacker,
                attacker_name: "Attacker".to_string(),
                valid_blockers: vec![(blocker, "Blocker".to_string())],
                min_blockers: 0,
            }],
        );

        let attackers = router.decide_attackers(&game, &attackers_ctx);
        let blockers = router.decide_blockers(&game, &blockers_ctx);

        assert_eq!(attackers.len(), 1);
        assert_eq!(attackers[0].creature, expected_attackers[0].creature);
        assert_eq!(attackers[0].target, expected_attackers[0].target);
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].blocker, expected_blockers[0].blocker);
        assert_eq!(blockers[0].blocking, expected_blockers[0].blocking);
    }
}
