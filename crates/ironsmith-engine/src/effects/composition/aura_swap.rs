//! Aura swap keyword action.

use crate::decisions::context::SelectionRevealPolicy;
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::{AttachmentTarget, AuraAttachmentFilterRuntimeExt};
use crate::types::Subtype;
use crate::zone::Zone;

pub type AuraSwapEffect = ironsmith_core::AuraSwapEffect;

impl EffectExecutor for AuraSwapEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = execute_aura_swap(game, ctx);
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        result
    }
}

fn execute_aura_swap(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
        let Some(source) = game.object(ctx.source) else {
            return Ok(EffectOutcome::resolved());
        };
        if source.zone != Zone::Battlefield
            || game.is_phased_out(ctx.source)
            || source.owner != ctx.controller
        {
            return Ok(EffectOutcome::resolved());
        }
        let Some(attached_to) = source.attached_to else {
            return Ok(EffectOutcome::resolved());
        };
        if !game.attachment_target_exists(attached_to) {
            return Ok(EffectOutcome::resolved());
        }

        let mut candidates = aura_swap_candidates(game, ctx.controller, attached_to);
        // In peer matches only the owner knows which of its hidden hand cards
        // are Auras: keep this peer's placeholders choosable and always ask,
        // so every peer replays the owner's answer (see
        // `game_state::hidden_hand_choices`). The chosen card is opened on
        // every peer before the replay and then checked like any other.
        let aura_filter = crate::filter::ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(crate::target::PlayerFilter::Specific(ctx.controller))
            .with_subtype(Subtype::Aura);
        let hand: Vec<ObjectId> = game
            .player(ctx.controller)
            .map(|player| player.hand.to_vec())
            .unwrap_or_default();
        let hidden_hand_choice =
            game.hand_choice_depends_on_hidden_identity(&aura_filter, hand.iter().copied());
        if hidden_hand_choice {
            let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
            for id in game.hidden_hand_placeholder_candidates(
                &aura_filter,
                &filter_ctx,
                hand.iter().copied(),
            ) {
                if !candidates.contains(&id) {
                    candidates.push(id);
                }
            }
        }
        if candidates.is_empty() && !hidden_hand_choice {
            return Ok(EffectOutcome::resolved());
        }

        let mut spec = ChooseObjectsSpec::new(
            ctx.source,
            "Choose an Aura card in your hand",
            candidates.clone(),
            0,
            Some(1),
        );
        if hidden_hand_choice {
            spec = spec
                .require_explicit_choice()
                .with_selection_reveal_policy(SelectionRevealPolicy::Public);
        }
        let chosen = make_decision(
            game,
            ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
        );
        if ctx.decision_maker.awaiting_choice() || chosen.is_empty() {
            return Ok(EffectOutcome::resolved());
        }
        let hand_aura = chosen[0];
        if !candidates.contains(&hand_aura) {
            return Ok(EffectOutcome::resolved());
        }
        if hidden_hand_choice {
            let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
            game.record_hidden_identity_obligations(
                &[hand_aura],
                &aura_filter,
                &filter_ctx,
                "exchange with an Aura card in hand",
            );
            game.mark_hidden_cards_publicly_revealed(&[hand_aura]);
            // The opened card must still be one that can be exchanged.
            if !aura_swap_candidates(game, ctx.controller, attached_to).contains(&hand_aura) {
                return Ok(EffectOutcome::resolved());
            }
        }

        // CR 701.12a / 702.65b: stage the exchange and publish it only if
        // both movements and the required attachment can be completed. This
        // also keeps replacement/as-enters decisions from exposing a partial
        // exchange when the decision maker needs to suspend for input.
        let mut exchange = game.clone();
        // Freeze both proposals before either program changes the exchange.
        let lookback = exchange.trigger_source_lookback_snapshots();
        let source_snapshot = exchange.object(ctx.source).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, &exchange)
        });
        let aura_snapshot = exchange.object(hand_aura).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, &exchange)
        });
        let scope = |object, from, to| crate::events::processing::ReplacementEventContext::with_scope(&exchange, crate::events::Event::zone_change(object, from, to, ctx.cause.clone(), None)
                .with_provenance(ctx.provenance), &ctx.replacement);
        // These are distinct events: inherit the parent scope independently.
        let return_scope = scope(ctx.source, Zone::Battlefield, Zone::Hand);
        let entry_scope = scope(hand_aura, Zone::Hand, Zone::Battlefield);
        let additional = ctx.replacement.additional_replacement_effects.clone();
        use crate::events::processing::{EventOutcome, PreparedEventOutcome};
        // Preparation prefixes and commit additions belong to each movement.
        // A failed original exchange is discarded with its speculative clone:
        // neither half of the exchange has happened (701.12a / 702.65b).
        let PreparedEventOutcome { original: return_original, mut programs } =
            crate::events::processing::prepare_zone_change_scoped(
                &mut exchange, ctx.source, Zone::Battlefield, Zone::Hand,
                ctx.cause.clone(), ctx.decision_maker, &additional,
                source_snapshot, Some(&return_scope), Vec::new(), Some(&lookback),
            )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let EventOutcome::Proceed(return_proposal) = return_original
            else { return Ok(EffectOutcome::prevented()); };
        let PreparedEventOutcome { original: entry_original, programs: mut entry_programs } =
            crate::events::processing::prepare_zone_change_scoped(
                &mut exchange, hand_aura, Zone::Hand, Zone::Battlefield,
                ctx.cause.clone(), ctx.decision_maker, &additional,
                aura_snapshot, Some(&entry_scope), Vec::new(), Some(&lookback),
            )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let EventOutcome::Proceed(mut entry_proposal) = entry_original
            else { return Ok(EffectOutcome::prevented()); };
        let mut returned = crate::events::processing::commit_prepared_zone_change(
            &mut exchange, ctx.source, return_proposal, ctx.decision_maker,
        )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let EventOutcome::Proceed(returned_source) = returned.original
            else { return Ok(EffectOutcome::prevented()); };
        programs.append(&mut returned.programs);
        let mut entered = if let Some(prepared) = entry_proposal.entry.take() {
            // Preserve legality established while the outgoing Aura existed.
            let committed = exchange.commit_prepared_exchange_etb_with_dm(
                hand_aura, prepared, ctx.controller, ctx.cause.clone(), ctx.decision_maker,
            )?;
            if committed.pending || ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let original = match committed.original {
                EventOutcome::Proceed(entry) => EventOutcome::Proceed(entry.new_id),
                EventOutcome::Prevented => EventOutcome::Prevented,
                EventOutcome::Replaced => EventOutcome::Replaced,
                EventOutcome::NotApplicable => EventOutcome::NotApplicable,
            };
            PreparedEventOutcome { original, programs: committed.programs }
        } else {
            crate::events::processing::commit_prepared_zone_change(
                &mut exchange, hand_aura, entry_proposal, ctx.decision_maker,
            )?
        };
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let EventOutcome::Proceed(new_aura) = entered.original
            else { return Ok(EffectOutcome::prevented()); };
        // A commit returning the unchanged original is no completed entry.
        if new_aura == hand_aura && exchange.object(hand_aura).is_some_and(|card| card.zone == Zone::Hand) {
            return Ok(EffectOutcome::prevented());
        }
        entry_programs.append(&mut entered.programs);
        // Exchange legality was checked with the old Aura still present
        // (CR 701.12e). Its departure may remove a type/quality needed by the
        // new Aura. That doesn't undo the exchange; the subsequent SBA will
        // deal with an Aura that is no longer legally enchanting its object.
        // A destination replacement modifies this otherwise legal exchange
        // (CR 614.6). A redirected card does not become attached.
        if exchange
            .object(new_aura)
            .is_some_and(|aura| aura.zone == Zone::Battlefield)
            && exchange.current_has_subtype(new_aura, Subtype::Aura)
        {
            if !exchange.attach_object_to_target(new_aura, attached_to) {
                return Ok(EffectOutcome::impossible());
            }
        }

        let returned = crate::effects::zones::promote_committed_zone_change_receipt(
            &mut exchange, ctx.source, PreparedEventOutcome { original: EventOutcome::Proceed(returned_source), programs },
        )?;
        let entered = crate::effects::zones::promote_committed_zone_change_receipt(
            &mut exchange, hand_aura, PreparedEventOutcome { original: EventOutcome::Proceed(new_aura), programs: entry_programs },
        )?;
        let mut ids = Vec::new();
        for receipt in [&returned, &entered] {
            if let EventOutcome::Proceed(change) = &receipt.original { ids.extend(change.new_object_ids.iter().copied()); }
        }
        let original = EffectOutcome::with_objects(ids);
        let outcome = crate::effects::zones::finish_zone_change_receipts(
            &mut exchange, ctx, original, vec![(ctx.source, returned), (hand_aura, entered)],
        )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        *game = exchange;
        Ok(outcome)
}

fn aura_swap_candidates(
    game: &GameState,
    player: PlayerId,
    attached_to: AttachmentTarget,
) -> Vec<ObjectId> {
    let Some(player_state) = game.player(player) else {
        return Vec::new();
    };
    player_state
        .hand
        .iter()
        .copied()
        .filter(|id| aura_card_can_attach_to_target(game, *id, player, attached_to))
        .collect()
}

fn aura_card_can_attach_to_target(
    game: &GameState,
    aura_id: ObjectId,
    controller: PlayerId,
    target: AttachmentTarget,
) -> bool {
    let Some(aura) = game.object(aura_id) else {
        return false;
    };
    if aura.zone != Zone::Hand
        || !aura.subtypes.contains(&Subtype::Aura)
        || game.card_cannot_enter_battlefield(aura_id)
        || !game.attachment_target_is_within_range(controller, target, Some(aura_id))
    {
        return false;
    }
    match target {
        AttachmentTarget::Object(target_id) => {
            if game.is_phased_out(target_id)
                || crate::targeting::has_protection_from_source(game, target_id, aura_id)
            {
                return false;
            }
        }
        AttachmentTarget::Player(player) => {
            if crate::effects::permanents::player_has_protection_from_object(game, player, aura) {
                return false;
            }
        }
    }
    let Some(filter) = aura.aura_attach_filter_owned() else {
        return false;
    };
    let filter_ctx = game.filter_context_for(controller, Some(aura_id));
    filter.matches_target(target, &filter_ctx, game)
}

#[cfg(test)]
mod replacement_aura_exchange_owner_contract_tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, StableId};
    use crate::object::{AuraAttachmentFilter, CounterType};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::types::CardType;
    struct Answers { outgoing: ObjectId, incoming: ObjectId, target: ObjectId, stable: StableId, arrival_zone: Zone, pause: bool, pending: bool, calls: usize, binding: bool }
    impl DecisionMaker for Answers {
        fn decide_objects(&mut self, _: &GameState, context: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            context.candidates.iter().filter(|candidate| candidate.legal && candidate.id == self.incoming).map(|candidate| candidate.id).collect()
        }
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1; assert!(game.object(self.outgoing).is_none()); assert!(game.object(self.incoming).is_none());
            let attached = &game.object(self.target).unwrap().attachments; assert_eq!(attached.len(), 1);
            assert_eq!(game.object(attached[0]).unwrap().attached_to, Some(AttachmentTarget::Object(self.target)));
            if self.binding { let arrival = game.objects_in_deterministic_order().into_iter().find(|object| object.stable_id == self.stable).unwrap(); assert_eq!(arrival.zone, self.arrival_zone); assert_eq!(game.counter_count(arrival.id, CounterType::PlusOnePlusOne), 1); }
            self.pending = self.pause; !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn definition(kind: CardType, aura: bool) -> crate::cards::CardDefinition {
        let mut builder = crate::CardDefinitionBuilder::new(CardId::new(), "Exchange owner fixture").card_types(vec![kind]);
        if aura { builder = builder.subtypes(vec![Subtype::Aura]).enchants(AuraAttachmentFilter::Object(ObjectFilter::creature())).with_ability(crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::enchant(AuraAttachmentFilter::Object(ObjectFilter::creature())))); }
        builder.build()
    }
    fn check(incoming_match: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let target = game.create_object_from_definition(&definition(CardType::Creature, false), alice, Zone::Battlefield);
        let outgoing = game.create_object_from_definition(&definition(CardType::Enchantment, true), alice, Zone::Battlefield);
        let incoming = game.create_object_from_definition(&definition(CardType::Enchantment, true), alice, Zone::Hand);
        let replacement_source = game.create_object_from_definition(&definition(CardType::Artifact, false), bob, Zone::Battlefield);
        let sentinel = game.create_object_from_definition(&definition(CardType::Artifact, false), alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(outgoing, AttachmentTarget::Object(target)));
        let matched = if incoming_match { incoming } else { outgoing }; let stable = game.object(matched).unwrap().stable_id;
        let (from, to) = if incoming_match { (Zone::Hand, Zone::Battlefield) } else { (Zone::Battlefield, Zone::Hand) };
        let actions = match mode { 1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it"))), Effect::may(vec![Effect::gain_life(0)])],
            _ => vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])] };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(replacement_source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(matched), Some(from), Some(to)), ReplacementAction::Additionally(actions)));
        let sentinel_snapshot = ObjectSnapshot::from_object(game.object(sentinel).unwrap(), &game);
        game.take_pending_trigger_events(); let ids = game.next_object_id_counter(); let objects = game.objects_in_deterministic_order().len();
        let mut dm = Answers { outgoing, incoming, target, stable, arrival_zone: to, pause: mode == 2, pending: false, calls: 0, binding: mode == 3 };
        let effect = Effect::aura_swap(); let mut ctx = ExecutionContext::new(outgoing, alice, &mut dm); ctx.set_tagged_objects("it", vec![sentinel_snapshot.clone()]);
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(result.unwrap().events.is_empty()); }
        else {
            let outcome = result.unwrap(); let arrivals = outcome.value.objects().unwrap(); assert_eq!(arrivals.len(), 2);
            assert_eq!(game.object(arrivals[0]).unwrap().zone, Zone::Hand); assert_eq!(game.object(arrivals[1]).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.object(arrivals[1]).unwrap().attached_to, Some(AttachmentTarget::Object(target))); assert_eq!(game.object(target).unwrap().attachments, vec![arrivals[1]]);
            assert_eq!(game.player(alice).unwrap().life, 20); assert_eq!(game.player(bob).unwrap().life, if mode == 3 { 20 } else { 27 });
            if mode == 3 { let arrival = game.objects_in_deterministic_order().into_iter().find(|object| object.stable_id == stable).unwrap(); assert_eq!(arrival.zone, to); assert_eq!(game.counter_count(arrival.id, CounterType::PlusOnePlusOne), 1); }
            else { assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).map(|event| (event.player, event.amount)).collect::<Vec<_>>(), vec![(bob, 3), (bob, 4)]); }
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source, outgoing); assert_eq!(ctx.controller, alice); assert!(ctx.targets.is_empty()); assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, sentinel_snapshot.object_id); assert_eq!(game.counter_count(sentinel, CounterType::PlusOnePlusOne), 0);
        if mode == 1 || mode == 2 {
            assert_eq!(game.object(outgoing).unwrap().zone, Zone::Battlefield); assert_eq!(game.object(incoming).unwrap().zone, Zone::Hand); assert_eq!(game.object(outgoing).unwrap().attached_to, Some(AttachmentTarget::Object(target))); assert_eq!(game.object(target).unwrap().attachments, vec![outgoing]);
            assert_eq!(game.player(bob).unwrap().life, 20); assert_eq!(game.next_object_id_counter(), ids); assert_eq!(game.objects_in_deterministic_order().len(), objects); assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx); if mode == 0 || mode == 3 { assert_eq!(dm.calls, 1); }
        if mode == 2 { assert_eq!(dm.calls, 1); dm.pause = false; dm.pending = false; let mut ctx = ExecutionContext::new(outgoing, alice, &mut dm);
            let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap(); assert_eq!(outcome.value.objects().unwrap().len(), 2); assert_eq!(game.player(bob).unwrap().life, 27); assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.calls, 2);
        }
    }
    #[test] fn departure_additions_follow_complete_attached_exchange() { check(false, 0); }
    #[test] fn departure_error_restores_complete_exchange() { check(false, 1); }
    #[test] fn departure_pending_replays_once() { check(false, 2); }
    #[test] fn departure_addition_binds_hand_arrival() { check(false, 3); }
    #[test] fn entry_additions_follow_complete_attached_exchange() { check(true, 0); }
    #[test] fn entry_error_restores_complete_exchange() { check(true, 1); }
    #[test] fn entry_pending_replays_once() { check(true, 2); }
    #[test] fn entry_addition_binds_battlefield_arrival() { check(true, 3); }
}
