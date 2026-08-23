//! Mana-restricted composition effect.
//!
//! Runs child effects with temporary mana usage restrictions so mana those
//! effects add to a mana pool is credited as restricted mana.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError, SequenceEffect};
use crate::game_state::GameState;

#[derive(Debug, Clone, PartialEq)]
pub struct ManaRestrictedEffect {
    pub effects: Vec<Effect>,
    pub restrictions: Vec<crate::ability::ManaUsageRestriction>,
}

impl ManaRestrictedEffect {
    pub fn new(
        effects: Vec<Effect>,
        restrictions: Vec<crate::ability::ManaUsageRestriction>,
    ) -> Self {
        Self {
            effects,
            restrictions,
        }
    }
}

impl EffectExecutor for ManaRestrictedEffect {
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
        let saved = ctx.mana.mana_usage_restrictions.clone();
        ctx.mana
            .mana_usage_restrictions
            .extend(self.restrictions.clone());
        let result = SequenceEffect::new(self.effects.clone()).execute(game, ctx);
        ctx.mana.mana_usage_restrictions = saved;
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
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::target::PlayerFilter;
    use crate::types::CardType;

    #[test]
    fn credits_child_mana_with_temporary_usage_restriction() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let restriction = crate::ability::ManaUsageRestriction::CastSpell {
            card_types: vec![CardType::Creature],
            subtype_requirement: None,
            restrict_to_matching_spell: true,
            grant_uncounterable: false,
            enters_with_counters: Vec::new(),
            granted_abilities: Vec::new(),
        };

        let effect = ManaRestrictedEffect::new(
            vec![Effect::new(AddManaEffect::new(
                vec![ManaSymbol::Green],
                PlayerFilter::You,
            ))],
            vec![restriction.clone()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect executes");

        let player = game.player(alice).expect("alice exists");
        assert_eq!(player.mana_pool.green, 1);
        assert_eq!(player.restricted_mana.len(), 1);
        assert_eq!(player.restricted_mana[0].symbol, ManaSymbol::Green);
        assert_eq!(player.restricted_mana[0].source, source);
        assert_eq!(player.restricted_mana[0].restrictions, vec![restriction]);
        assert!(ctx.mana.mana_usage_restrictions.is_empty());
    }
}
