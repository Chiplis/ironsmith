//! A blind opening is committed before the ordinary cast/land announcement.
//! Physical reversal and the already public learned identity are separate.
use super::*;
use crate::alternative_cast::{CastingMethod, GrantSelection};
use crate::decision::LegalAction;

fn option_label(game: &GameState, action: &LegalAction) -> String {
    // Reuse the ordinary renderer only after the exact face is publicly open.
    crate::decision::format_action_short(game, action, None)
}

pub(super) fn begin_open_exile_play(
    game: &mut GameState, queue: &mut TriggerQueue, state: &mut PriorityLoopState,
    card_id: ObjectId, incarnation: Option<u64>, permission: &GrantSelection, dm: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let player = game.turn.priority_player.ok_or_else(|| GameLoopError::InvalidState("No priority player".into()))?;
    if state.has_pending_action() || !crate::alternative_cast::blind_play::requires_opening(game, card_id, player) {
        return Err(GameLoopError::InvalidState("Card is not awaiting a blind exile announcement".into()));
    }
    crate::alternative_cast::blind_play::validate_incarnation(game, card_id, incarnation)?;
    crate::alternative_cast::blind_play::resolve(game, card_id, player, permission)?;
    if game.is_hidden_card_placeholder(card_id) {
        return Err(crate::effects::ExecutionError::IncompleteEvidence("blind announcement requires its verified public card opening".into()).into());
    }
    let before_opening = Box::new((game.clone(), queue.clone()));
    if !game.set_face_up(card_id).map_err(crate::effects::ExecutionError::ContinuousDiscovery)? {
        return Err(crate::effects::ExecutionError::IncompleteEvidence("blind announcement could not open its exact exile incarnation".into()).into());
    }
    // The face is public before any proposed face/price/land determination.
    // Retain the learned identity in the after-opening rollback checkpoint.
    let viewers = game.players.iter().map(|player| player.id).collect::<Vec<_>>();
    for viewer in viewers { game.grant_face_down_exile_view(card_id, viewer); }
    state.pending_exile_play = Some(PendingExilePlay { card_id, incarnation, player, permission: permission.clone(), actions: Vec::new() });
    state.opened_exile_play = state.pending_exile_play.clone();
    state.exile_play_before_opening = Some(before_opening);
    state.save_checkpoint(game);
    resume_open_exile_play(game, queue, state, dm)
}

pub(super) fn resume_open_exile_play(
    game: &mut GameState, queue: &mut TriggerQueue, state: &mut PriorityLoopState,
    dm: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let pending = state.pending_exile_play.as_ref().ok_or_else(|| GameLoopError::InvalidState("No opened exile play to resume".into()))?;
    if game.turn.priority_player != Some(pending.player) {
        return Err(GameLoopError::InvalidState("Opened exile play belongs to another player".into()));
    }
    crate::alternative_cast::blind_play::validate_incarnation(game, pending.card_id, pending.incarnation)?;
    let actions = crate::decision::opened_exile_play_actions(game, pending.player, pending.card_id, &pending.permission)?;
    if actions.is_empty() {
        // CR 733: an uncompletable attempted play reverses physical rules
        // state and queues. The identity already opened publicly cannot become
        // unknown again; retain knowledge only for that exact exile incarnation.
        let card_id = pending.card_id;
        let before = state.exile_play_before_opening.take().ok_or_else(|| crate::effects::ExecutionError::IncompleteEvidence(
            "opened exile play lost its physical rollback receipt".into()))?;
        let (before_game, before_queue) = *before;
        game.restore_execution_checkpoint(before_game, false); *queue = before_queue;
        let viewers = game.players.iter().map(|player| player.id).collect::<Vec<_>>();
        for viewer in viewers { game.grant_face_down_exile_view(card_id, viewer); }
        state.pending_exile_play = None; state.clear_checkpoint();
        return advance_priority_with_dm(game, queue, dm);
    }
    let mut context = crate::decisions::context::SelectOptionsContext::new(pending.player, Some(pending.card_id),
        "Choose how to play the opened card", actions.iter().enumerate().map(|(index, action)|
            crate::decisions::context::SelectableOption::new(index, option_label(game, action))).collect(), 1, 1);
    context.exile_play_choice = true;
    state.pending_exile_play.as_mut().unwrap().actions = actions;
    state.opened_exile_play = state.pending_exile_play.clone();
    Ok(GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::SelectOptions(context)))
}

pub(super) fn apply_exile_play_choice(
    game: &mut GameState, queue: &mut TriggerQueue, state: &mut PriorityLoopState,
    choice: usize, dm: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let pending = state.pending_exile_play.clone().ok_or_else(|| GameLoopError::InvalidState("No opened exile play choice".into()))?;
    if game.turn.priority_player != Some(pending.player) { return Err(GameLoopError::InvalidState("Opened exile play belongs to another player".into())); }
    crate::alternative_cast::blind_play::validate_incarnation(game, pending.card_id, pending.incarnation)?;
    let selected = pending.actions.get(choice).cloned().ok_or_else(|| ResponseError::IllegalChoice("Choose one offered opened-card play".into()))?;
    let current = crate::decision::opened_exile_play_actions(game, pending.player, pending.card_id, &pending.permission)?;
    if !current.contains(&selected) { return Err(ResponseError::IllegalChoice("The selected opened-card play is no longer legal".into()).into()); }
    let before = (game.clone(), queue.clone(), state.clone());
    state.pending_exile_play = None;
    state.opened_exile_play = Some(pending);
    let is_land = matches!(selected, LegalAction::PlayLand { .. } | LegalAction::PlayLandBackFace { .. });
    // The exact selection has just been authoritatively revalidated. Continue
    // through the same announcement/cost/entry owner as ordinary priority play.
    let result = super::priority_apply::apply_admitted_priority_action(game, queue, state, &selected, dm);
    if result.is_err() || dm.awaiting_choice() {
        game.restore_execution_checkpoint(before.0, result.is_ok() && dm.awaiting_choice());
        *queue = before.1; *state = before.2;
        if result.is_ok() && dm.awaiting_choice() { return Ok(GameProgress::Continue); }
    } else if is_land {
        state.clear_checkpoint();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::{SelectFirstDecisionMaker, compute_legal_actions};
    use crate::grant_registry::{GrantSource, PlayFromConstraints};
    use crate::mana::ManaCost;
    use crate::effect::Value;
    use crate::{CardId, CardType};
    const A: PlayerId = PlayerId::from_index(0);
    const B: PlayerId = PlayerId::from_index(1);
    fn fixture(kind: CardType, cost: u32) -> (GameState, ObjectId, ObjectId) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = B; game.turn.priority_player = Some(B); game.turn.phase = crate::game_state::Phase::FirstMain; game.turn.step = None;
        let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Public permission source").card_types(vec![CardType::Enchantment]).build(), A, Zone::Battlefield);
        let card = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Previously private face")
            .card_types(vec![kind]).mana_cost(ManaCost::new().add_generic(cost)).build(), A, Zone::Exile);
        game.set_face_down(card); game.grant_face_down_exile_view(card, A);
        // A preceding qualified grant would make the ordinary face-filtered
        // index differ. It must not affect the opaque opening's index.
        game.effect_store.grant_registry.grant_to_filter(crate::target::ObjectFilter::creature().in_zone(Zone::Exile), Zone::Exile, B,
            crate::grant::Grantable::PlayFrom, GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
        game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, B,
            PlayFromConstraints { cast_mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::AnyColor, ..Default::default() },
            GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
        (game, source, card)
    }
    fn card_actions(game: &GameState, card: ObjectId) -> Vec<LegalAction> {
        compute_legal_actions(game, B).unwrap().into_iter().filter(|action| crate::decision::legal_action_source(action) == Some(card)).collect()
    }
    fn open(game: &mut GameState, queue: &mut TriggerQueue, state: &mut PriorityLoopState, card: ObjectId) -> GameProgress {
        let actions = card_actions(game, card);
        let action = actions.iter().find(|action| matches!(action, LegalAction::OpenExiledCardForPlay { .. })).expect("opaque opening authority");
        apply_priority_response_with_dm(game, queue, state, &PriorityResponse::PriorityAction(action.clone()), &mut SelectFirstDecisionMaker).unwrap()
    }
    #[test]
    fn unopened_actions_do_not_depend_on_type_price_mana_or_land_allowance() {
        let mut expected = None;
        for kind in [CardType::Land, CardType::Sorcery, CardType::Creature] { for cost in [0, 100] { for mana in [0, 200] {
            let (mut game, _, card) = fixture(kind, cost); game.player_mut(B).unwrap().mana_pool.red = mana; game.player_mut(B).unwrap().lands_played_this_turn = 1;
            assert!(!game.can_player_look_at_face_down_exiled_card(card, B)); let actions = card_actions(&game, card);
            assert_eq!(actions.len(), 2); assert!(matches!(&actions[0], LegalAction::OpenExiledCardForPlay { permission, .. } if permission.index == 0));
            assert_eq!(crate::decision::format_action_short(&game, &actions[0], None), "Play exiled card");
            if let Some(expected) = &expected { assert_eq!(&actions, expected); } else { expected = Some(actions); }
        } } }
    }
    #[test]
    fn failed_authority_does_not_open_or_grant_private_inspection() {
        let (mut game, _, card) = fixture(CardType::Sorcery, 1); let mut action = card_actions(&game, card).remove(0);
        if let LegalAction::OpenExiledCardForPlay { permission, .. } = &mut action { permission.index += 1; }
        assert!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut PriorityLoopState::new(2),
            &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).is_err());
        assert!(game.is_face_down(card)); assert!(!game.can_player_look_at_face_down_exiled_card(card, B)); assert!(game.stack.is_empty());
    }
    #[test]
    fn unavailable_spells_and_lands_leave_the_public_opening_without_consuming_a_play() {
        for kind in [CardType::Land, CardType::Sorcery] {
            let (mut game, _, card) = fixture(kind, 100); game.player_mut(B).unwrap().lands_played_this_turn = 1;
            let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new();
            let before = game.player(B).unwrap().mana_pool.clone();
            let events_before = game.clone().take_pending_trigger_events().len();
            open(&mut game, &mut queue, &mut state, card);
            assert!(game.is_face_down(card), "illegal attempt restores its physical face-down state"); assert_eq!(game.object(card).unwrap().zone, Zone::Exile);
            assert!(game.can_player_look_at_face_down_exiled_card(card, A)); assert!(game.can_player_look_at_face_down_exiled_card(card, B));
            assert!(game.current_characteristics(card).unwrap().card_types.is_empty());
            assert!(queue.entries.is_empty()); assert_eq!(game.clone().take_pending_trigger_events().len(), events_before);
            assert!(!state.has_opened_exile_play_receipt());
            assert!(!state.has_pending_action()); assert_eq!(game.player(B).unwrap().mana_pool, before); assert_eq!(game.player(B).unwrap().lands_played_this_turn, 1); assert!(game.stack.is_empty());
        }
    }
    #[test]
    fn opened_land_failure_preserves_disclosure_exact_authority_and_native_recovery() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        let (mut game, source, card) = fixture(CardType::Land, 0); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
        let progress = open(&mut game, &mut queue, &mut state, card);
        assert!(matches!(progress, GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::SelectOptions(context)) if context.exile_play_choice));
        let unrelated = crate::decisions::context::SelectOptionsContext::new(B, Some(card), "Nested entry choice", vec![], 1, 1);
        assert!(!unrelated.exile_play_choice);
        let committed = state.opened_exile_play.clone().unwrap(); assert_eq!(committed.card_id, card); assert_eq!(committed.player, B);
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, B,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(crate::target::ObjectFilter::specific(card), Some(Zone::Exile), Some(Zone::Battlefield)),
            ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::lose_life(Value::X)])));
        let saved = (game.clone(), state.clone()); let next_id = game.next_object_id_counter();
        let choice = state.pending_exile_play.as_ref().unwrap().actions.iter().position(|action| matches!(action, LegalAction::PlayLand { .. })).unwrap();
        assert!(apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::ExilePlayChoice(choice), &mut SelectFirstDecisionMaker).is_err());
        assert!(!game.is_face_down(card)); assert_eq!(game.object(card).unwrap().zone, Zone::Exile); assert_eq!(game.player(B).unwrap().lands_played_this_turn, 0);
        assert_eq!(game.player(B).unwrap().life, 20); assert_eq!(game.next_object_id_counter(), next_id); assert!(state.pending_exile_play.is_some());
        assert_eq!(state.opened_exile_play.as_ref().unwrap().permission, committed.permission);
        game = saved.0; state = saved.1; game.effect_store.replacement_effects.remove_effect(replacement);
        apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::ExilePlayChoice(choice), &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(B).unwrap().lands_played_this_turn, 1); assert!(!state.has_pending_action()); assert!(!game.exile.contains(&card));
    }
    #[test]
    fn a_spell_choice_retains_the_selected_origin_and_cannot_cancel_the_opening() {
        let (mut game, _, card) = fixture(CardType::Sorcery, 1); game.player_mut(B).unwrap().mana_pool.red = 1;
        let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new(); open(&mut game, &mut queue, &mut state, card);
        let selected = state.pending_exile_play.as_ref().unwrap().permission.clone(); let after_open = game.clone();
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::ExilePlayChoice(0), &mut SelectFirstDecisionMaker).unwrap();
        for _ in 0..32 { if !state.has_pending_action() { break; }
            let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("ordinary play has a resumable decision"); };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
        }
        assert!(!state.has_pending_action()); assert_eq!(game.player(B).unwrap().mana_pool.total(), 0); assert_eq!(game.stack.len(), 1);
        let receipt = game.object(game.stack[0].object_id).unwrap().cast_play_permission.as_deref().unwrap();
        assert_eq!(receipt.identity, selected.identity); assert_eq!(receipt.origin, card); assert_eq!(receipt.player, B);
        // A recovered before-choice checkpoint already knows the opened face.
        let mut recovered = after_open; let mut state = PriorityLoopState::new(2);
        state.pending_exile_play = Some(PendingExilePlay { card_id: card, incarnation: None, player: B, permission: selected, actions: Vec::new() });
        state.opened_exile_play = state.pending_exile_play.clone(); state.save_checkpoint(&recovered);
        assert!(apply_priority_response_with_dm(&mut recovered, &mut TriggerQueue::new(), &mut state,
            &PriorityResponse::ManaPaymentPlan(crate::mana_payment::ManaPaymentResponse::Cancel), &mut SelectFirstDecisionMaker).is_err());
        assert!(!recovered.is_face_down(card)); assert!(state.opened_exile_play.is_some());
    }
    #[test]
    fn prepared_opened_land_uses_selected_grant_and_observes_original_before_additions() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        use crate::special_actions::{LandPlayObservationKind, LandPlayObservationTiming};
        let (mut game, source, card) = fixture(CardType::Land, 0);
        game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, B,
            PlayFromConstraints { lands_enter_tapped: true, ..Default::default() },
            GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
        let permission = crate::alternative_cast::blind_play::selections(&game, card, B).unwrap().pop().unwrap();
        let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new();
        apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(LegalAction::OpenExiledCardForPlay {
                card_id: card, incarnation: None, permission: permission.clone(),
            }), &mut SelectFirstDecisionMaker).unwrap();
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, B,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(crate::target::ObjectFilter::specific(card), Some(Zone::Exile), Some(Zone::Battlefield)),
            ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::destroy(
                crate::target::ChooseSpec::Object(crate::target::ObjectFilter::specific(source)))])));
        let mut observations = Vec::new();
        crate::special_actions::execute_land_play_with_observer(&mut game, B, card, false, Some(&permission),
            LandPlayObservationTiming::BeforeHistory, &mut SelectFirstDecisionMaker,
            |game, _, arrival, kind, event| {
                assert!(game.object(source).is_some(), "completion has not removed the source");
                assert_eq!(event.snapshot().unwrap().object_id, arrival);
                assert!(game.is_tapped(arrival), "the second selected grant supplies tapped entry");
                observations.push((matches!(kind, LandPlayObservationKind::Entry), game.player(B).unwrap().life,
                    game.player(B).unwrap().lands_played_this_turn));
                Ok(())
            }).unwrap();
        assert_eq!(observations, vec![(true, 20, 0), (false, 20, 0)]);
        assert_eq!(game.player(B).unwrap().life, 23); assert_eq!(game.player(B).unwrap().lands_played_this_turn, 1);
        assert!(game.object(source).is_none()); assert!(game.object(card).is_none());
    }

}

#[cfg(test)]
mod independent_price_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::{SelectFirstDecisionMaker, compute_legal_actions};
    use crate::grant_registry::{GrantSource, PlayFromConstraints};
    use crate::{CardId, CardType};
    #[test]
    fn an_independent_free_price_is_hidden_before_opening_and_keeps_the_selected_origin_afterward() {
        let player = PlayerId::from_index(0); let owner = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20); game.turn.active_player = player; game.turn.priority_player = Some(player); game.turn.phase = crate::game_state::Phase::FirstMain; game.turn.step = None;
        let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Origin").card_types(vec![CardType::Enchantment]).build(), player, Zone::Battlefield);
        let price = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Independent price").card_types(vec![CardType::Enchantment]).build(), player, Zone::Battlefield);
        let card = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Expensive secret").card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::new().add_generic(100)).build(), owner, Zone::Exile); game.set_face_down(card);
        game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, player,
            PlayFromConstraints { cast_mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::AnyColor, ..Default::default() },
            GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
        game.effect_store.grant_registry.grant_to_card(card, Zone::Exile, player,
            crate::grant::Grantable::AlternativePrice { costs: vec![], origin: None },
            GrantSource::Effect { source_id: price, expires_end_of_turn: u32::MAX });
        let actions = compute_legal_actions(&game, player).unwrap().into_iter().filter(|action| crate::decision::legal_action_source(action) == Some(card)).collect::<Vec<_>>();
        assert_eq!(actions.len(), 2);
        let opening = actions.iter().find(|action| matches!(action, LegalAction::OpenExiledCardForPlay { .. })).expect("opaque opening, no free-price face leak");
        let LegalAction::OpenExiledCardForPlay { permission, .. } = opening else { panic!("only opening authority before disclosure"); };
        let selected = permission.identity.clone(); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
        apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(opening.clone()), &mut SelectFirstDecisionMaker).unwrap();
        let pending = state.pending_exile_play.as_ref().unwrap(); assert_eq!(pending.actions.len(), 1);
        assert!(matches!(&pending.actions[0], LegalAction::CastSpell { casting_method: CastingMethod::AlternativePrice { origin_permission: Some(origin), .. }, .. }
            if origin.identity == selected));
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::ExilePlayChoice(0), &mut SelectFirstDecisionMaker).unwrap();
        for _ in 0..32 { if !state.has_pending_action() { break; }
            let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("ordinary price announcement"); };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
        }
        assert!(!state.has_pending_action()); assert_eq!(game.stack.len(), 1); assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
        let receipt = game.object(game.stack[0].object_id).unwrap().cast_play_permission.as_deref().unwrap(); assert_eq!(receipt.identity, selected);
    }
}
