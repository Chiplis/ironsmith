//! Per-mana-unit retention composition effect.
//!
//! Child mana effects are executed with a temporary retention duration. The
//! mana crediting path records that duration on every produced unit, including
//! units created after mana-production replacement effects are applied.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, SequenceEffect};
use crate::game_state::GameState;

#[derive(Debug, Clone, PartialEq)]
pub struct ManaRetainedEffect {
    pub effects: Vec<Effect>,
    pub duration: ironsmith_core::ManaRetentionDuration,
}

impl ManaRetainedEffect {
    pub fn new(effects: Vec<Effect>, duration: ironsmith_core::ManaRetentionDuration) -> Self {
        Self { effects, duration }
    }

    pub fn until_end_of_combat(effects: Vec<Effect>) -> Self {
        Self::new(effects, ironsmith_core::ManaRetentionDuration::EndOfCombat)
    }
}

impl EffectExecutor for ManaRetainedEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let saved = ctx.mana.retention;
        ctx.mana.retention = Some(self.duration);
        let result = SequenceEffect::new(self.effects.clone()).execute(game, ctx);
        ctx.mana.retention = saved;
        result
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.effects])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.effects], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::AddManaEffect;
    use crate::game_state::{Phase, Step};
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::target::PlayerFilter;

    #[test]
    fn retains_only_wrapped_mana_until_the_end_of_combat() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // This ordinary red mana must not inherit Firebending's retention just
        // because it shares a color with the wrapped mana.
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 1);
        let effect = ManaRetainedEffect::until_end_of_combat(vec![Effect::new(
            AddManaEffect::new(vec![ManaSymbol::Red, ManaSymbol::Red], PlayerFilter::You),
        )]);

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect executes");
        assert_eq!(game.player(alice).expect("alice exists").mana_pool.red, 3);
        assert_eq!(ctx.mana.retention, None, "wrapper restores its context");

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.empty_mana_pools();
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.red,
            2,
            "ordinary mana empties while the two wrapped units survive"
        );

        game.turn.step = Some(Step::DeclareBlockers);
        game.empty_mana_pools();
        assert_eq!(game.player(alice).expect("alice exists").mana_pool.red, 2);

        game.turn.step = Some(Step::EndCombat);
        game.empty_mana_pools();
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.red,
            0,
            "Firebending mana expires as the combat phase ends"
        );
    }

    #[test]
    fn separate_combat_batches_expire_at_their_own_end_of_combat() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ManaRetainedEffect::until_end_of_combat(vec![Effect::new(
            AddManaEffect::new(vec![ManaSymbol::Red], PlayerFilter::You),
        )]);

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        effect.execute(&mut game, &mut ctx).expect("first combat");
        game.turn.step = Some(Step::EndCombat);
        game.empty_mana_pools();
        assert_eq!(game.player(alice).expect("alice exists").mana_pool.red, 0);

        game.turn.step = Some(Step::DeclareAttackers);
        effect.execute(&mut game, &mut ctx).expect("extra combat");
        game.empty_mana_pools();
        assert_eq!(game.player(alice).expect("alice exists").mana_pool.red, 1);
        game.turn.step = Some(Step::EndCombat);
        game.empty_mana_pools();
        assert_eq!(game.player(alice).expect("alice exists").mana_pool.red, 0);
    }
}
