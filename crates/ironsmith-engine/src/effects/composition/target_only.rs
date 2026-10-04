//! TargetOnly effect implementation.
//!
//! This effect resolves a target and does nothing else. It exists for cards
//! whose rules text only establishes a target (e.g., "Target permanent.").

use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_players_from_spec};
use crate::effects::{EffectExecutor, TargetReusePolicy};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::TargetOnlyEffect;

impl EffectExecutor for TargetOnlyEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Count bounds constrain announcement (CR 601.2c), not how many of
        // those targets must survive resolution (CR 608.2b). Synthetic object
        // declarations retain every surviving member for later instructions.
        let member_spec = if !self.explicit_declaration && self.target.is_target() {
            ChooseSpec::target(self.target.base().clone())
        } else {
            self.target.clone()
        };
        if let Ok(objects) = resolve_objects_for_effect(game, ctx, &member_spec)
            && !objects.is_empty()
        {
            return Ok(EffectOutcome::count(objects.len() as i32)
                .with_chosen_objects_from_game(game, objects));
        }

        if let Ok(players) = resolve_players_from_spec(game, &self.target, ctx)
            && !players.is_empty()
        {
            return Ok(EffectOutcome::count(players.len() as i32));
        }

        if self.target.count().min == 0 {
            return Ok(EffectOutcome::count(0));
        }
        if self.explicit_declaration {
            Err(ExecutionError::InvalidTarget)
        } else {
            Ok(EffectOutcome::target_invalid())
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_chooser(&self) -> Option<&crate::target::PlayerFilter> {
        self.chooser.as_ref()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        Some(self.target.count())
    }

    fn target_reuse_policy(&self) -> TargetReusePolicy {
        if self.explicit_declaration {
            TargetReusePolicy::AlwaysDeclareNew
        } else {
            TargetReusePolicy::SyntheticPrelude
        }
    }

    fn target_description(&self) -> &'static str {
        "target"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_target_declaration_allows_no_target_but_required_does_not() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let target = ChooseSpec::target(ChooseSpec::creature());
        let required = TargetOnlyEffect::explicit(target.clone());
        assert!(matches!(
            required.execute(&mut game, &mut ctx),
            Err(ExecutionError::InvalidTarget)
        ));
        let optional =
            TargetOnlyEffect::new(target.with_count(crate::effect::ChoiceCount::up_to(1)));
        assert_eq!(
            optional.execute(&mut game, &mut ctx).unwrap().as_count(),
            Some(0)
        );
    }
}

#[cfg(test)]
mod partial_target_declaration_tests {
    use super::*;
    #[test]
    fn synthetic_count_bounds_do_not_erase_a_surviving_announced_target() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let definition =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Survivor")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 4))
                .build();
        let survivor =
            game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(survivor)];
        let target = ChooseSpec::target(ChooseSpec::creature())
            .with_count(crate::effect::ChoiceCount::exactly(2));
        let declaration = TargetOnlyEffect::new(target);
        assert_eq!(declaration.get_target_count().unwrap().min, 2);
        let outcome = declaration.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(outcome.as_count(), Some(1));
        assert_eq!(outcome.chosen_objects().unwrap()[0], survivor);
        ctx.targets.clear();
        assert!(declaration.execute(&mut game, &mut ctx).is_ok());
    }
}
