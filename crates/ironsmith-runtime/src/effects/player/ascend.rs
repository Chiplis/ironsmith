//! Ascend effect implementation for instants and sorceries.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub use ironsmith_core::AscendEffect;

impl EffectExecutor for AscendEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller = ctx.controller;
        let permanent_count = game
            .battlefield
            .iter()
            .copied()
            .filter(|&object_id| game.controller_of_id(object_id) == Some(controller))
            .count();

        if permanent_count >= 10 {
            game.grant_citys_blessing(controller);
        }
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn add_permanent(game: &mut GameState, controller: PlayerId, name: &str) {
        let definition = crate::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    }

    #[test]
    fn spell_ascend_checks_permanents_only_when_it_resolves() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for index in 0..9 {
            add_permanent(&mut game, alice, &format!("Permanent {index}"));
        }

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        AscendEffect::new().execute(&mut game, &mut ctx).unwrap();
        assert!(!game.has_citys_blessing(alice));

        add_permanent(&mut game, alice, "Tenth Permanent");
        AscendEffect::new().execute(&mut game, &mut ctx).unwrap();
        assert!(game.has_citys_blessing(alice));
    }

    #[test]
    fn permanent_ascend_is_checked_during_continuous_state_refresh() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for index in 0..9 {
            add_permanent(&mut game, alice, &format!("Permanent {index}"));
        }
        let ascend_permanent = crate::CardDefinitionBuilder::new(CardId::new(), "Ascender")
            .card_types(vec![CardType::Creature])
            .with_ability(Ability::static_ability(StaticAbility::ascend()))
            .build();
        game.create_object_from_definition(&ascend_permanent, alice, Zone::Battlefield);

        game.refresh_continuous_state();
        assert!(game.has_citys_blessing(alice));
        assert!(
            game.stack.is_empty(),
            "permanent ascend does not use the stack"
        );
    }
}
