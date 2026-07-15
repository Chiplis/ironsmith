//! Cast-source effect implementation.
//!
//! Casts the source card of the resolving effect/ability.

use crate::alternative_cast::CastingMethod;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;
pub use ironsmith_core::CastSourceEffect;

use super::runtime_helpers::with_spell_cast_event;

/// Effect that casts the source card immediately.
impl EffectExecutor for CastSourceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = ctx.source;
        let Some(source_obj) = game.object(source_id) else {
            return Ok(EffectOutcome::target_invalid());
        };

        if source_obj.is_land() {
            return Ok(EffectOutcome::target_invalid());
        }
        if self.require_exile && source_obj.zone != Zone::Exile {
            return Ok(EffectOutcome::target_invalid());
        }

        let from_zone = source_obj.zone;
        let mut suspend_alternative_index = if from_zone == Zone::Exile {
            source_obj
                .alternative_casts
                .iter()
                .position(|method| method.suspend_spec().is_some())
        } else {
            None
        };
        if self.cast_as_suspend && suspend_alternative_index.is_none() {
            if let Some(obj) = game.object_mut(source_id) {
                suspend_alternative_index = Some(obj.alternative_casts.len());
                obj.alternative_casts.push(
                    crate::alternative_cast::AlternativeCastingMethod::Suspend {
                        cost: crate::mana::ManaCost::new(),
                        time: 0,
                    },
                );
            }
        }
        let casting_method = CastingMethod::PlayFrom {
            source: source_id,
            zone: from_zone,
            use_alternative: suspend_alternative_index,
        };
        let result = crate::game_loop::cast_spell_from_resolving_effect(
            game,
            source_id,
            from_zone,
            ctx.controller,
            &casting_method,
            self.without_paying_mana_cost,
            None,
            ctx.provenance,
            &mut ctx.decision_maker,
        )
        .map_err(|error| ExecutionError::Impossible(error.to_string()))?;
        let Some(new_id) = result else {
            return if ctx.decision_maker.awaiting_choice() {
                Ok(EffectOutcome::count(0))
            } else {
                Ok(EffectOutcome::impossible())
            };
        };
        Ok(with_spell_cast_event(
            EffectOutcome::with_objects(vec![new_id]),
            game,
            new_id,
            ctx.controller,
            from_zone,
            ctx.provenance,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::{OutcomeStatus, OutcomeValue};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn cast_source_requires_exile_when_requested() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Suspend Probe")
                .card_types(vec![CardType::Sorcery])
                .build(),
            alice,
            Zone::Hand,
        );

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        let outcome = CastSourceEffect::new()
            .without_paying_mana_cost()
            .require_exile()
            .execute(&mut game, &mut ctx)
            .expect("cast source should execute");

        assert_eq!(outcome.status, OutcomeStatus::TargetInvalid);
        assert!(game.stack.is_empty());
    }

    #[test]
    fn cast_source_free_cast_sets_x_to_zero_and_emits_spell_cast_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "X Fireball")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X, ManaSymbol::Red]))
                .build(),
            alice,
            Zone::Exile,
        );

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        let outcome = CastSourceEffect::new()
            .without_paying_mana_cost()
            .require_exile()
            .execute(&mut game, &mut ctx)
            .expect("free cast from exile should resolve");

        let OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected the source card to move to the stack");
        };
        let cast_id = ids[0];

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == crate::events::EventKind::SpellCast),
            "cast-source should emit a SpellCast event"
        );

        let stack_entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == cast_id)
            .expect("cast object should be on the stack");
        assert_eq!(stack_entry.x_value, Some(0));

        let spell = game.object(cast_id).expect("stack spell should exist");
        assert_eq!(spell.zone, Zone::Stack);
        assert_eq!(spell.x_value, Some(0));
    }
}
