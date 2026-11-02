//! Source-owned ordinary programs at the actual public reveal boundary.
use crate::ability::AbilityKind;
use crate::decision::DecisionMaker;
use crate::decisions::context::BooleanContext;
use crate::effects::{EffectContext, ExecutionError};
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::RevealActionOffer;
use crate::triggers::TriggerEvent;
use crate::GameState;

#[derive(Debug, Clone)]
pub(crate) struct RevealedCardActionOffer {
    pub event: TriggerEvent,
    pub snapshot: ObjectSnapshot,
    pub offer: RevealActionOffer,
}

fn allowance_remains(game: &GameState, pending: &RevealedCardActionOffer) -> bool {
    let Some(maximum) = pending.offer.max_per_player_turn else { return true; };
    let Some(reveal) = pending.event.downcast::<crate::events::CardRevealedEvent>() else { return false; };
    let count = game.turn_store.turn_history.projected_records().filter(|record|
        record.event.downcast::<crate::events::NamedActionEvent>().is_some_and(|action|
            action.player == reveal.player && action.label.eq_ignore_ascii_case(&pending.offer.label)))
        .count();
    count < maximum as usize
}

fn exact_revealed_source_remains(game: &GameState, pending: &RevealedCardActionOffer) -> bool {
    game.object(pending.snapshot.object_id).is_some_and(|object|
        object.stable_id == pending.snapshot.stable_id && object.zone == pending.snapshot.zone)
}

pub(crate) fn reveal_action_offer_is_available(game: &GameState, pending: &RevealedCardActionOffer) -> bool {
    exact_revealed_source_remains(game, pending) && allowance_remains(game, pending)
}

pub(crate) fn reveal_action_offers_for_event(game: &GameState, event: &TriggerEvent)
    -> Result<Vec<RevealedCardActionOffer>, ExecutionError>
{
    if game.turn_store.turn_history.projected_records().any(|record|
        record.event.occurrence_key() == event.occurrence_key()) { return Ok(Vec::new()); }
    let Some(reveal) = event.downcast::<crate::events::CardRevealedEvent>() else { return Ok(Vec::new()); };
    let Some(snapshot) = reveal.snapshot.as_ref() else {
        return Err(ExecutionError::IncompleteEvidence("public reveal lacks exact source snapshot".into()));
    };
    if snapshot.object_id != reveal.card || snapshot.zone != reveal.zone {
        return Err(ExecutionError::IncompleteEvidence("public reveal source snapshot contradicts its occurrence".into()));
    }
    let mut offers = Vec::new();
    for ability in snapshot.abilities.iter() {
        let AbilityKind::Static(ability) = &ability.kind else { continue; };
        let Some(offer) = ability.reveal_action_offer() else { continue; };
        if !offer.source_zones.contains(&reveal.zone) { continue; }
        let pending = RevealedCardActionOffer { event: event.clone(), snapshot: snapshot.clone(), offer };
        if exact_revealed_source_remains(game, &pending) && allowance_remains(game, &pending) {
            offers.push(pending);
        }
    }
    Ok(offers)
}

pub(crate) fn reveal_action_offer_boolean_context(pending: &RevealedCardActionOffer) -> BooleanContext {
    let reveal = pending.event.downcast::<crate::events::CardRevealedEvent>().expect("typed reveal offer");
    BooleanContext::new(reveal.player, Some(pending.snapshot.object_id), pending.offer.display.clone())
        .with_source_name(pending.snapshot.name.clone())
}

/// Only an accepted program with a real original destination receipt completes.
/// This does not finish the reveal boundary: callers own all offers for that reveal.
pub(crate) fn complete_reveal_action_offer(game: &mut GameState, pending: &RevealedCardActionOffer,
    accepted: bool, decisions: &mut dyn DecisionMaker) -> Result<bool, ExecutionError>
{
    if !accepted || !exact_revealed_source_remains(game, pending) || !allowance_remains(game, pending) {
        return Ok(false);
    }
    let reveal = pending.event.downcast::<crate::events::CardRevealedEvent>().expect("typed reveal offer");
    let mut ctx = EffectContext::new_default(pending.snapshot.object_id, reveal.player)
        .with_decision_maker(decisions).with_provenance(pending.event.provenance());
    ctx.source_snapshot = Some(pending.snapshot.clone());
    ctx.set_tagged_objects("__reveal_action_source", vec![pending.snapshot.clone()]);
    crate::effects::composition::execute_transaction(game, &mut ctx, || false, |game, ctx| {
        let program = crate::effect::Effect::new(crate::effects::SequenceEffect::new(pending.offer.effects.clone()));
        let mut outputs = crate::effects::execute_effect_with_outputs(game, &program, ctx)?;
        if ctx.decision_maker.awaiting_choice() { return Ok(false); }
        // The authored program's primary projection excludes replacement
        // child observations, including an independent move of this SAME
        // physical card into the requested zone. Preserve aggregate events
        // below, but never treat those child receipts as the accepted action.
        let arrivals = outputs.outcome.instruction_result().execution_facts.iter().filter_map(|fact| match fact {
            crate::effect::ExecutionFact::OriginalZoneMoveCards(objects) => Some(objects), _ => None,
        }).flatten().filter(|snapshot|
            snapshot.stable_id == pending.snapshot.stable_id && snapshot.zone == pending.offer.arrival_zone)
            .cloned().collect::<Vec<_>>();
        let arrival = arrivals.iter().find(|snapshot| game.object(snapshot.object_id).is_some_and(|object|
            object.stable_id == snapshot.stable_id && object.zone == snapshot.zone)).cloned();
        for event in outputs.outcome.events.drain(..) {
            game.stage_turn_history_event(&event);
            game.queue_trigger_event(ctx.provenance, event);
        }
        let Some(arrival) = arrival else { return Ok(false); };
        ctx.source = arrival.object_id;
        ctx.source_snapshot = Some(arrival.clone());
        ctx.set_tagged_objects("__reveal_action_arrival", vec![arrival.clone()]);
        let mut lookback = game.try_trigger_source_lookback_snapshots()?;
        if !lookback.iter().any(|source| source.object_id == arrival.object_id) { lookback.push(arrival.clone()); }
        let provenance = game.alloc_child_event_provenance(ctx.provenance, crate::events::EventKind::KeywordAction);
        let completion = TriggerEvent::new_with_provenance(crate::events::NamedActionEvent::new(
            pending.offer.label.clone(), reveal.player, arrival.object_id, arrival.clone(), vec![arrival],
            1, ctx.cause.clone()).with_complete_source_lookback(), provenance)
            .with_lookback_source_snapshots(lookback);
        game.stage_turn_history_event(&completion);
        game.queue_trigger_event(ctx.provenance, completion);
        Ok(true)
    })
}

pub(crate) fn finish_public_reveal_offer_boundary(game: &mut GameState, event: &TriggerEvent) {
    game.stage_turn_history_event(event);
}

pub(crate) fn process_public_reveal_action_offers(game: &mut GameState, event: &TriggerEvent,
    decisions: &mut dyn DecisionMaker) -> Result<(), ExecutionError>
{
    let offers = reveal_action_offers_for_event(game, event)?;
    let Some(reveal) = event.downcast::<crate::events::CardRevealedEvent>() else { return Ok(()); };
    let mut ctx = EffectContext::new_default(reveal.card, reveal.player).with_decision_maker(decisions);
    crate::effects::composition::execute_transaction(game, &mut ctx, || (), |game, ctx| {
        for pending in offers {
            if !exact_revealed_source_remains(game, &pending) || !allowance_remains(game, &pending) { continue; }
            let accepted = ctx.decision_maker.decide_boolean(game, &reveal_action_offer_boolean_context(&pending));
            if ctx.decision_maker.awaiting_choice() { return Ok(()); }
            complete_reveal_action_offer(game, &pending, accepted, ctx.decision_maker)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(()); }
        }
        finish_public_reveal_offer_boundary(game, event);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CardId, PlayerId, Zone};
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::effects::{EffectExecutor, ExileEffect};
    use crate::static_abilities::{CompiledStaticAbility, StaticAbility, StaticAbilityModelInterpreter};
    use crate::target::ChooseSpec;
    const A: PlayerId = PlayerId(0);
    struct Choices { answers: std::collections::VecDeque<bool>, offers: usize }
    impl DecisionMaker for Choices {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            self.offers += 1;
            self.answers.pop_front().expect("unexpected reveal offer")
        }
    }
    fn card() -> crate::CardDefinition {
        let model = CompiledStaticAbility::reveal_action_offer("shared disclosed action",
            vec![Zone::Hand, Zone::Library], Some(1), Zone::Exile,
            vec![crate::Effect::new(ExileEffect::with_spec(ChooseSpec::Source))], "Exile this revealed card?");
        CardDefinitionBuilder::new(CardId::new(), "Offered card")
            .card_types(vec![crate::CardType::Sorcery])
            .with_ability(crate::ability::Ability::static_ability(
                StaticAbility::new(StaticAbilityModelInterpreter::new(model))))
            .build()
    }
    fn event(game: &mut GameState, card: crate::ObjectId) -> TriggerEvent {
        let snapshot = ObjectSnapshot::from_object(game.object(card).unwrap(), game);
        let provenance = game.provenance_graph_mut().alloc_root_event(crate::events::EventKind::CardRevealed);
        super::super::public_reveal_observation(A, card, snapshot.zone, card,
            Some(snapshot), None, None, provenance)
    }
    fn completions(game: &GameState) -> Vec<crate::events::NamedActionEvent> {
        game.turn_store.turn_history.projected_records().filter_map(|record|
            record.event.downcast::<crate::events::NamedActionEvent>().cloned()).collect()
    }
    #[test]
    fn decline_then_accept_uses_one_shared_player_allowance_and_exact_arrival() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let first = game.create_object_from_definition(&card(), A, Zone::Hand);
        let second = game.create_object_from_definition(&card(), A, Zone::Library);
        let third = game.create_object_from_definition(&card(), A, Zone::Hand);
        let stable = game.object(second).unwrap().stable_id;
        let mut choices = Choices { answers: [false, true].into(), offers: 0 };
        let first_event = event(&mut game, first);
        process_public_reveal_action_offers(&mut game, &first_event, &mut choices).unwrap();
        // Bubbling the same observation is not another reveal action.
        process_public_reveal_action_offers(&mut game, &first_event, &mut choices).unwrap();
        let second_event = event(&mut game, second);
        process_public_reveal_action_offers(&mut game, &second_event, &mut choices).unwrap();
        let third_event = event(&mut game, third);
        process_public_reveal_action_offers(&mut game, &third_event, &mut choices).unwrap();
        assert_eq!(choices.offers, 2);
        assert_eq!(game.object(first).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(third).unwrap().zone, Zone::Hand);
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(arrival, second);
        assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
        let completed = completions(&game);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].source, arrival);
        assert_eq!(completed[0].snapshot.object_id, arrival);
        assert_eq!(completed[0].objects[0].object_id, arrival);
        assert_eq!(completed[0].objects[0].zone, Zone::Exile);
        let original = second_event.downcast::<crate::events::CardRevealedEvent>().unwrap();
        assert_eq!(original.card, second);
        assert_eq!(original.snapshot.as_ref().unwrap().zone, Zone::Library);
        // Current-object resolution cannot put the departed reveal back into a pile.
        let ctx = EffectContext::new_default(first, A);
        let mut ctx = ctx;
        ctx.set_tagged_objects("revealed pool", vec![original.snapshot.clone().unwrap()]);
        // A plain local tag intentionally follows an authored zone change.
        // An exact source collection must explicitly retain its incarnation.
        assert_eq!(crate::effects::helpers::resolve_objects_from_spec(&game,
            &ChooseSpec::Tagged("revealed pool".into()), &ctx).unwrap(), vec![arrival]);
        ctx.exact_object_reference_tags.insert("revealed pool".into());
        assert!(crate::effects::helpers::resolve_objects_from_spec(&game,
            &ChooseSpec::Tagged("revealed pool".into()), &ctx).unwrap().is_empty());
    }
    #[test]
    fn prevented_or_redirected_program_does_not_consume_allowance() {
        for prevent in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let first = game.create_object_from_definition(&card(), A, Zone::Hand);
            let second = game.create_object_from_definition(&card(), A, Zone::Hand);
            let mut replacement = crate::replacement::ZoneReplacementSpec::new(
                crate::target::ObjectFilter::specific(first), Zone::Graveyard)
                .from_zone(Zone::Hand).to_zone(Zone::Exile).build(second, A);
            if prevent { replacement.replacement = crate::replacement::ReplacementAction::Prevent; }
            game.effect_store.replacement_effects.add_one_shot_effect(replacement);
            let mut choices = Choices { answers: [true, true].into(), offers: 0 };
            let original = event(&mut game, first);
            process_public_reveal_action_offers(&mut game, &original, &mut choices).unwrap();
            assert!(completions(&game).is_empty());
            let next = event(&mut game, second);
            process_public_reveal_action_offers(&mut game, &next, &mut choices).unwrap();
            assert_eq!(choices.offers, 2);
            assert_eq!(completions(&game).len(), 1);
        }
    }
    #[test]
    fn replacement_child_exiling_same_revealed_card_is_not_the_original_reveal_action() {
        for origin in [Zone::Hand, Zone::Library] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let first = game.create_object_from_definition(&card(), A, origin);
            let second = game.create_object_from_definition(&card(), A, Zone::Hand);
            let stable = game.object(first).unwrap().stable_id;
            let second_stable = game.object(second).unwrap().stable_id;
            let mut replacement = crate::replacement::ZoneReplacementSpec::new(
                crate::target::ObjectFilter::specific(first), Zone::Graveyard)
                .from_zone(origin).to_zone(Zone::Exile).build(second, A);
            // Its one-shot owner is consumed before its Instead program, so
            // this independent ordinary exile really moves the exact same
            // original object to the exact requested destination.
            replacement.replacement = crate::replacement::ReplacementAction::Instead(vec![
                crate::Effect::gain_life(2),
                crate::Effect::new(ExileEffect::with_spec(ChooseSpec::SpecificObject(first))),
                crate::Effect::gain_life(3),
            ]);
            game.effect_store.replacement_effects.add_one_shot_effect(replacement);
            let mut choices = Choices { answers: [true, true].into(), offers: 0 };
            let original = event(&mut game, first);
            process_public_reveal_action_offers(&mut game, &original, &mut choices).unwrap();
            let child_arrival = game.find_object_by_stable_id(stable).unwrap();
            assert_ne!(child_arrival, first);
            assert_eq!(game.object(child_arrival).unwrap().zone, Zone::Exile,
                "the adversarial child must actually move this same card");
            assert_eq!(game.player(A).unwrap().life, 25,
                "replacement prefix and suffix still execute exactly once");
            assert!(completions(&game).is_empty(),
                "a child original-zone receipt cannot complete the replaced parent action");
            assert!(!game.effect_store.grant_registry.card_can_play_from_zone(
                &game, child_arrival, Zone::Exile, A),
                "a replacement child's exile must not invent an action permission");
            assert!(game.retained_action_observations()
                .chain(game.effect_store.pending_trigger_events.iter()).any(|observation|
                observation.downcast::<crate::events::zones::ZoneChangeEvent>().is_some_and(|move_event|
                    move_event.from == origin && move_event.to == Zone::Exile
                        && move_event.objects.contains(&first))),
                "the real child movement observation remains published; origin={origin:?}, original={first:?}, arrival={child_arrival:?}, observations={:?}",
                game.retained_action_observations().chain(game.effect_store.pending_trigger_events.iter())
                    .map(|event|(event.kind(),event.provenance(),event.downcast::<crate::events::ZoneChangeEvent>().map(|change|(change.from,change.to,change.objects.clone(),change.result_objects.clone())))).collect::<Vec<_>>());
            // The failed original spent no once-per-player allowance. A new
            // ordinary reveal of a different card can still complete it.
            let legitimate = event(&mut game, second);
            process_public_reveal_action_offers(&mut game, &legitimate, &mut choices).unwrap();
            assert_eq!(choices.offers, 2);
            let completed = completions(&game);
            assert_eq!(completed.len(), 1);
            assert_eq!(completed[0].snapshot.stable_id, second_stable);
            assert_ne!(completed[0].snapshot.stable_id, stable);
            assert_eq!(completed[0].player, A);
        }
    }
    #[test]
    fn private_look_does_not_offer_but_real_reveal_enters_same_gateway() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&card(), A, Zone::Battlefield);
        let revealed = game.create_object_from_definition(&card(), A, Zone::Library);
        let stable = game.object(revealed).unwrap().stable_id;
        let mut choices = Choices { answers: [true].into(), offers: 0 };
        {
            let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut choices);
            crate::effects::LookAtTopCardsEffect::new(crate::target::PlayerFilter::You, 1, "private")
                .execute(&mut game, &mut ctx).unwrap();
        }
        assert_eq!(choices.offers, 0);
        {
            let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut choices);
            crate::effects::RevealTopEffect::new(crate::target::PlayerFilter::You, None)
                .execute(&mut game, &mut ctx).unwrap();
        }
        assert_eq!(choices.offers, 1);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Exile);
        assert_eq!(completions(&game).len(), 1);
    }
}
