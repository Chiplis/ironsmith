//! Mill effect implementation.

use crate::effects::zones::apply_zone_change_with_context_and_additional_effects;
use crate::effect::{EffectOutcome, OutcomeObjectMemory, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::zones::apply_zone_change_with_additional_effects;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that mills cards from a player's library to their graveyard.
///
/// # Fields
///
/// * `count` - How many cards to mill (can be fixed or variable)
/// * `player` - Which player mills
///
/// # Example
///
/// ```ignore
/// // Mill 3 cards
/// let effect = MillEffect::new(3, PlayerFilter::You);
/// ```
pub type MillEffect = ironsmith_core::MillEffect;

impl EffectExecutor for MillEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn cost_description(&self) -> Option<String> {
        if self.player == PlayerFilter::You
            && let Value::Fixed(count) = self.count
        {
            return Some(if count == 1 {
                "Mill a card".to_string()
            } else {
                format!("Mill {} cards", count)
            });
        }
        None
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        // Snapshot the top cards first so replacement/prevention on one card does not
        // change which original cards are being milled.
        let cards_to_mill: Vec<ObjectId> = game
            .player(player_id)
            .map(|p| {
                p.library
                    .iter()
                    .rev()
                    .filter(|id| !ctx.replacement.entry_reserved_objects.contains(id))
                    .take(count)
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut milled = Vec::new();
        let mut milled_memory = Vec::new();
        let mut any_prevented = false;
        let mut receipts = Vec::new();

        // CR 701.17a / 603.2c: the cards are milled at the same time, as one
        // event ("whenever one or more cards are put into your graveyard").
        let opened_batch = game.open_simultaneous_action();
        for card_id in cards_to_mill {
            let Some(from_zone) = game.object(card_id).map(|obj| obj.zone) else {
                continue;
            };
            let pre_memory = OutcomeObjectMemory::from_object_id(game, card_id);
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            let receipt = apply_zone_change_with_context_and_additional_effects(
    game,
    card_id,
    from_zone,
    Zone::Graveyard,
    ctx.cause.clone(),
    ctx,
    &additional_effects
)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            match &receipt.original {
                EventOutcome::Proceed(change) => {
                    if change.final_zone.is_public()
                        && let Some(new_id) = change.new_object_id
                    {
                        milled.push(new_id);
                        if let Some(memory) = pre_memory {
                            milled_memory.push(memory);
                        }
                    }
                }
                EventOutcome::Prevented => {
                    any_prevented = true;
                }
                EventOutcome::Replaced | EventOutcome::NotApplicable => {}
            }
            receipts.push((card_id, receipt));
        }
        game.close_simultaneous_action(opened_batch);

        let original_outcome = if !milled.is_empty() {
            EffectOutcome::with_objects(milled.clone())
                .with_affected_objects(milled)
                .with_affected_object_memory(milled_memory)
        } else if any_prevented {
            EffectOutcome::prevented()
        } else {
            EffectOutcome::count(0)
        };
        crate::effects::zones::finish_zone_change_receipts(game, ctx, original_outcome, receipts)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if ctx.decision_maker.awaiting_choice() { return result.map(|_| EffectOutcome::count(0)); }
        result
    }
}

impl CostExecutableEffect for MillEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        let player_id = match self.player {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(id) => id,
            _ => controller,
        };
        let count = match &self.count {
            Value::Fixed(count) => (*count).max(0) as usize,
            Value::X => {
                return Err(crate::effects::CostValidationError::Other(
                    "dynamic X mill costs are not supported".to_string(),
                ));
            }
            _ => {
                let ctx = crate::effects::ExecutionContext::new_default(source, controller);
                crate::effects::helpers::resolve_value(game, &self.count, &ctx)
                    .map_err(|err| crate::effects::CostValidationError::Other(format!("{err:?}")))?
                    .max(0) as usize
            }
        };
        let available = game.player(player_id).map_or(0, |p| p.library.len());
        if available >= count {
            Ok(())
        } else {
            Err(crate::effects::CostValidationError::Other(
                "not enough cards in library to pay mill cost".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::Effect;
    use crate::effects::execute_effect;
    use crate::ids::{CardId, PlayerId};
    use crate::tag::TagKey;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_cards_to_library(game: &mut GameState, owner: PlayerId, count: usize) -> Vec<ObjectId> {
        (0..count)
            .map(|idx| {
                let card = CardBuilder::new(
                    CardId::from_raw(20_000 + idx as u32),
                    format!("Library Card {idx}"),
                )
                .card_types(vec![CardType::Instant])
                .build();
                game.create_object_from_card(&card, owner, Zone::Library)
            })
            .collect()
    }

    #[test]
    fn mill_moves_cards_through_zone_change_and_returns_graveyard_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let original_ids = add_cards_to_library(&mut game, alice, 3);
        let original_top_two = vec![original_ids[2], original_ids[1]];
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MillEffect::you(2);
        let outcome = effect.execute(&mut game, &mut ctx).expect("execute mill");
        let crate::effect::OutcomeValue::Objects(milled_ids) = outcome.value else {
            panic!("expected mill to return moved graveyard objects");
        };

        assert_eq!(milled_ids.len(), 2);
        assert_eq!(game.player(alice).expect("alice").library.len(), 1);
        assert_eq!(game.player(alice).expect("alice").graveyard, milled_ids);
        for original_id in original_top_two {
            assert!(
                game.object(original_id).is_none(),
                "original milled object should not remain after zone change"
            );
        }
    }

    #[test]
    fn tagged_mill_tags_post_move_graveyard_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_cards_to_library(&mut game, alice, 1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = Effect::mill(1).tag("milled");
        let outcome = execute_effect(&mut game, &effect, &mut ctx).expect("execute tagged mill");
        let crate::effect::OutcomeValue::Objects(milled_ids) = outcome.value else {
            panic!("expected tagged mill to return milled object ids");
        };

        let tagged = ctx
            .tagged_objects
            .get(&TagKey::from("milled"))
            .expect("milled tag should exist");
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].object_id, milled_ids[0]);
        assert_eq!(tagged[0].zone, Zone::Graveyard);
    }

    #[test]
    fn mill_mills_as_many_cards_as_possible() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_cards_to_library(&mut game, alice, 1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = MillEffect::you(3)
            .execute(&mut game, &mut ctx)
            .expect("mill resolves");
        let crate::effect::OutcomeValue::Objects(milled_ids) = outcome.value else {
            panic!("expected mill to return moved object ids");
        };

        assert_eq!(
            milled_ids.len(),
            1,
            "mill should move the only available card"
        );
        assert!(
            game.player(alice)
                .expect("alice should exist")
                .library
                .is_empty(),
            "library should be emptied when milling more cards than are available"
        );
        assert_eq!(
            game.player(alice).expect("alice should exist").graveyard,
            milled_ids,
            "the available top card should end up in the graveyard"
        );
    }

    #[test]
    fn mill_tracks_replaced_public_zone_cards_as_milled() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_cards_to_library(&mut game, alice, 1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        execute_effect(
            &mut game,
            &Effect::new(crate::effects::ExileInsteadOfGraveyardEffect::you()),
            &mut ctx,
        )
        .expect("replacement effect should resolve");

        let outcome = execute_effect(&mut game, &Effect::mill(1).tag("milled"), &mut ctx)
            .expect("execute tagged mill");
        let crate::effect::OutcomeValue::Objects(milled_ids) = outcome.value else {
            panic!("expected tagged mill to return milled object ids");
        };

        assert_eq!(
            milled_ids.len(),
            1,
            "replacement mill should still report the moved card"
        );
        let milled_id = milled_ids[0];
        let milled = game
            .object(milled_id)
            .expect("milled card should still exist");
        assert_eq!(
            milled.zone,
            Zone::Exile,
            "replacement should move the milled card to exile"
        );

        let tagged = ctx
            .tagged_objects
            .get(&TagKey::from("milled"))
            .expect("milled tag should exist");
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].object_id, milled_id);
        assert_eq!(
            tagged[0].zone,
            Zone::Exile,
            "milled tags should follow the card into a replaced public zone"
        );
    }
}


#[cfg(test)]
mod additional_contract_tests {
    use super::*;
    use crate::effect::Effect;
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    struct ObserveOriginalMill { alice: PlayerId, pause: bool, pending: bool, questions: usize }
    impl crate::decision::DecisionMaker for ObserveOriginalMill {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.questions += 1;
            assert_eq!(game.player(self.alice).unwrap().graveyard.len(), 2, "the entire original mill precedes its added instructions");
            assert!(game.player(self.alice).unwrap().library.is_empty());
            self.pending = self.pause;
            !self.pause
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn check_additional_mill(mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let card = crate::card::CardBuilder::new(CardId::new(), "Mill addition fixture")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2,2)).build();
        game.create_object_from_card(&card, alice, Zone::Library);
        let top = game.create_object_from_card(&card, alice, Zone::Library);
        let source = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let effects = if mode == 3 { vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne, 1, crate::target::ChooseSpec::tagged("it")))] }
        else if mode == 1 { vec![Effect::gain_life(3), Effect::lose_life(Value::X)] }
        else { vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])] };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                crate::target::ObjectFilter::specific(top), Some(Zone::Library), Some(Zone::Graveyard)),
            ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events();
        let library = game.player(alice).unwrap().library.clone();
        let before_id = game.next_object_id_counter();
        let parent_tag = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let mut dm = ObserveOriginalMill { alice, pause: mode == 2, pending: false, questions: 0 };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects("it", vec![parent_tag.clone()]);
        let result = MillEffect::you(2).execute(&mut game, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else {
            let outcome = result.unwrap();
            if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(outcome.events.is_empty()); }
            else {
                let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else { panic!("original mill summary"); };
                assert_eq!(ids.len(), 2);
                assert_eq!(outcome.affected_object_memory().unwrap().iter().filter(|memory| memory.zone == Zone::Library).count(), 2, "original pre-mill memory survives alongside any added action memory");
                assert!(game.player(alice).unwrap().library.is_empty());
                if mode == 3 {
                    let arrived = ids[0];
                    assert_eq!(game.object(arrived).unwrap().counters.get(&crate::object::CounterType::PlusOnePlusOne), Some(&1));
                    assert!(!game.object(source).unwrap().counters.contains_key(&crate::object::CounterType::PlusOnePlusOne));
                    assert!(outcome.execution_facts.iter().filter_map(|fact| match fact { crate::effect::ExecutionFact::AffectedObjectMemory(memory) => Some(memory.as_slice()), _ => None }).flatten().any(|memory| memory.object_id == arrived && memory.zone == Zone::Graveyard), "the added counter action must retain its own post-move memory");
                    assert!(!outcome.affected_object_memory().unwrap_or(&[]).iter().any(|memory| memory.object_id == arrived && memory.zone == Zone::Graveyard), "auxiliary post-move counter memory is not original movement memory");
                } else {
                    assert_eq!(game.player(bob).unwrap().life, 27);
                    let gains = outcome.events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>()).collect::<Vec<_>>();
                    assert_eq!(gains.iter().map(|e| e.amount).collect::<Vec<_>>(), vec![3,4]);
                    assert!(gains.iter().all(|e| e.player == bob));
                }
            }
        }
        assert_eq!(ctx.source, source); assert_eq!(ctx.controller, alice);
        assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, parent_tag.object_id);
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().life, 20);
        if mode == 1 || mode == 2 {
            assert_eq!(game.player(alice).unwrap().library, library);
            assert!(game.player(alice).unwrap().graveyard.is_empty());
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(game.next_object_id_counter(), before_id);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            assert!(game.take_pending_trigger_events().is_empty());
        } else { assert!(game.effect_store.replacement_effects.get_effect(shield).is_none()); }
        if mode == 2 {
            assert_eq!(dm.questions, 1);
            let mut replay = ObserveOriginalMill { alice, pause: false, pending: false, questions: 0 };
            let mut ctx = ExecutionContext::new(source, alice, &mut replay);
            let outcome = MillEffect::you(2).execute(&mut game, &mut ctx).unwrap();
            assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx);
            assert_eq!(replay.questions, 1); assert_eq!(game.player(bob).unwrap().life, 27);
            let crate::effect::OutcomeValue::Objects(ids) = outcome.value else { panic!("replayed original mill summary"); };
            assert_eq!(ids.len(), 2);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>()).map(|e| e.amount).collect::<Vec<_>>(), vec![3,4]);
        }
    }
    #[test] fn additional_mill_runs_after_original_batch_and_keeps_primary_objects() { check_additional_mill(0); }
    #[test] fn additional_mill_error_restores_entire_batch_and_program_prefix() { check_additional_mill(1); }
    #[test] fn additional_mill_pending_restores_then_replays_once() { check_additional_mill(2); }
    #[test] fn additional_mill_binds_arriving_card_and_keeps_counter_facts() { check_additional_mill(3); }
}
