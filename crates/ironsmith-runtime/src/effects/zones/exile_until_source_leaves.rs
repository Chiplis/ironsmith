//! Exile-until effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::apply_zone_change_with_additional_effects;

/// Duration for "exile ... until ..." effects.
pub type ExileUntilDuration = ironsmith_core::ExileUntilDuration;

/// Exile objects with an associated duration.
pub type ExileUntilEffect = ironsmith_core::ExileUntilEffect;

impl EffectExecutor for ExileUntilEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.duration == ExileUntilDuration::SourceLeavesBattlefield
            && !game
                .object(ctx.source)
                .is_some_and(|source| source.zone == Zone::Battlefield)
        {
            return Ok(EffectOutcome::count(0));
        }

        let objects = resolve_objects_for_effect(game, ctx, &self.spec)?;
        let mut exiled_count = 0_i32;
        for object_id in objects {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let from_zone = obj.zone;
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            let result = apply_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                Zone::Exile,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            if let EventOutcome::Proceed(result) = result
                && result.final_zone == Zone::Exile
            {
                for &new_id in &result.new_object_ids {
                    if self.face_down {
                        game.set_face_down(new_id);
                    }
                    game.add_exiled_with_source_link(ctx.source, new_id);
                    exiled_count += 1;
                }
            }
        }

        if exiled_count > 0 && self.duration == ExileUntilDuration::SourceLeavesBattlefield {
            game.mark_return_exiled_when_source_leaves(ctx.source);
        }
        Ok(EffectOutcome::count(exiled_count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target to exile"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::zones::matchers::WouldBeExiledMatcher;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_exile_until_respects_destination_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_creature_on_battlefield(&mut game, "Elite Vanguard", alice);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldBeExiledMatcher::new(ObjectFilter::permanent()),
                ReplacementAction::ChangeDestination(Zone::Hand),
            ),
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(game.exile.is_empty());
        assert_eq!(game.get_exiled_with_source_links(source).len(), 0);
        assert_eq!(game.players[0].hand.len(), 1);
        assert!(game.battlefield.is_empty());
    }

    #[test]
    fn source_leaves_duration_noops_if_source_already_left_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature_on_battlefield(&mut game, "Banisher Priest", alice);
        let creature_id = create_creature_on_battlefield(&mut game, "Elite Vanguard", alice);
        game.move_object_by_effect(source, Zone::Graveyard);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(game.exile.is_empty());
        assert!(game.battlefield.contains(&creature_id));
    }

    #[test]
    fn source_leaves_duration_returns_exiled_card_without_stack_trigger() {
        let mut game = setup_game();
        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        let alice = PlayerId::from_index(0);
        let source = create_creature_on_battlefield(&mut game, "Banisher Priest", alice);
        let creature_id = create_creature_on_battlefield(&mut game, "Elite Vanguard", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.get_exiled_with_source_links(source).len(), 1);
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Elite Vanguard")
        }));

        game.move_object_by_effect(source, Zone::Graveyard);
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);

        assert!(trigger_queue.entries.is_empty());
        assert!(game.exile.is_empty());
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Elite Vanguard")
        }));
    }
}
