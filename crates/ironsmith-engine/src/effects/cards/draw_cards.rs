//! DrawCards effect implementation.

use crate::decision::DecisionMaker;
use crate::decisions::context::{BooleanContext, ViewCardsContext};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionContextCheckpoint, ExecutionError};
use crate::events::processing::{
    TraitEventResult, ReplacementEventContext, process_trait_event_with_execution_context,
};
use crate::events::{CardRevealedEvent, CardsDrawnEvent, Event};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
pub use ironsmith_core::DrawCardsEffect;

/// Execute a draw replacement with its captured event and complete history.
pub(crate) fn execute_scoped_draw_replacement_effects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    replacement_source: ObjectId,
    replacement_controller: PlayerId,
    context: &ReplacementEventContext,
) -> Result<EffectOutcome, ExecutionError> {
    let replaced_player = crate::events::downcast_event::<crate::events::DrawEvent>(
        context.event.inner(),
    ).ok_or_else(|| ExecutionError::InternalError(
        "draw replacement lost its draw event".into(),
    ))?.player;
    let mut outcome = crate::effects::replacement::execute_replacement_payload(
        game, ctx, effects, replacement_source, replacement_controller, context, None,
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    let drawn_count = outcome.events.iter()
        .filter_map(|event| event.downcast::<CardsDrawnEvent>())
        .filter(|event| event.player == replaced_player)
        .try_fold(0i32, |total, event| {
            let amount = i32::try_from(event.amount()).map_err(|_| ExecutionError::InternalError(
                "draw replacement outcome exceeds the supported count range".into(),
            ))?;
            total.checked_add(amount).ok_or_else(|| ExecutionError::InternalError(
                "draw replacement outcome exceeds the supported count range".into(),
            ))
        })?;
    outcome.value = crate::effect::OutcomeValue::Count(drawn_count);
    Ok(outcome)
}

#[derive(Debug, Clone)]
pub(crate) struct AutomaticDrawRevealCandidate {
    pub source_id: ObjectId,
    pub source_name: String,
    pub player_id: PlayerId,
    pub card_id: ObjectId,
    pub zone: Zone,
    pub optional: bool,
    pub snapshot: Option<ObjectSnapshot>,
}

pub(crate) fn automatic_draw_reveal_boolean_context(
    candidate: &AutomaticDrawRevealCandidate,
) -> BooleanContext {
    BooleanContext::new(
        candidate.player_id,
        Some(candidate.source_id),
        "reveal the first card you draw",
    )
    .with_source_name(candidate.source_name.clone())
}

pub(crate) fn collect_automatic_draw_reveal_candidates(
    game: &GameState,
    player_id: PlayerId,
    drawn: &[ObjectId],
    draws_before: u32,
) -> Vec<AutomaticDrawRevealCandidate> {
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    let mut candidates = Vec::new();
    let draws_after = draws_before + drawn.len() as u32;

    for &source_id in &game.battlefield {
        let Some(source_obj) = game.object(source_id) else {
            continue;
        };
        if game.controller_of(source_obj) != player_id {
            continue;
        }
        let Some(static_abilities) = view.static_abilities_rc(source_id) else {
            continue;
        };
        for static_ability in static_abilities.iter() {
            let Some(spec) = static_ability.reveal_drawn_card_spec() else {
                continue;
            };
            if spec.your_turns_only && !game.is_active_player(player_id) {
                continue;
            }
            let draw_number = spec.card_number;
            if draw_number == 0 || draws_before >= draw_number || draw_number > draws_after {
                continue;
            }

            let drawn_index = (draw_number - draws_before - 1) as usize;
            let Some(&card_id) = drawn.get(drawn_index) else {
                continue;
            };

            let snapshot = game
                .object(card_id)
                .map(|obj| ObjectSnapshot::from_object(obj, game));
            candidates.push(AutomaticDrawRevealCandidate {
                source_id,
                source_name: source_obj.name.to_string(),
                player_id,
                card_id,
                zone: Zone::Hand,
                optional: spec.optional,
                snapshot,
            });
        }
    }

    candidates
}

pub(crate) fn emit_automatic_draw_reveal_event(
    game: &mut GameState,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
    candidate: &AutomaticDrawRevealCandidate,
    provenance: ProvNodeId,
) -> TriggerEvent {
    for viewer_idx in 0..game.players.len() {
        let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
        let view_ctx = ViewCardsContext::new(
            viewer,
            candidate.player_id,
            Some(candidate.source_id),
            candidate.zone,
            "Reveal drawn card",
        )
        .with_public(true);
        decision_maker.view_cards(game, viewer, &[candidate.card_id], &view_ctx);
    }

    let provenance = game.provenance_graph_mut()
        .alloc_child_event(provenance, crate::events::EventKind::CardRevealed);
    TriggerEvent::new_with_provenance(
        CardRevealedEvent::new(
            candidate.player_id,
            candidate.card_id,
            candidate.zone,
            Some(candidate.source_id),
            candidate.snapshot.clone(),
        ),
        provenance,
    )
}

/// How a "reveal the first card you draw" reveal of a private hidden card
/// (hidden-information matches) is made public on every peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HiddenDrawRevealMode {
    /// Ask the owner right away; the caller stops while the answer is awaited
    /// and is re-run with it (effect resolution).
    Inline,
    /// Defer to the draw reveal windows answered before triggers are put on
    /// the stack (turn-based draws, which cannot pause mid-step).
    Defer,
}

/// The pending reveal record for a candidate whose card is private.
pub(crate) fn pending_hidden_automatic_draw_reveal(
    candidate: &AutomaticDrawRevealCandidate,
) -> crate::game_state::PendingAutomaticDrawReveal {
    crate::game_state::PendingAutomaticDrawReveal {
        player: candidate.player_id,
        card: candidate.card_id,
        source: candidate.source_id,
        optional: candidate.optional,
    }
}

/// Prompt text of the owner-answered reveal of a private drawn card.
pub(crate) fn hidden_automatic_draw_reveal_description(optional: bool) -> &'static str {
    if optional {
        "You may reveal the first card you drew this turn"
    } else {
        "Reveal the first card you drew this turn"
    }
}

/// Rebuild a reveal candidate for `pending` from the current (now publicly
/// opened) card, so its snapshot carries the revealed characteristics.
pub(crate) fn automatic_draw_reveal_candidate_for_pending(
    game: &GameState,
    pending: &crate::game_state::PendingAutomaticDrawReveal,
) -> AutomaticDrawRevealCandidate {
    AutomaticDrawRevealCandidate {
        source_id: pending.source,
        source_name: game
            .object(pending.source)
            .map(|source| source.name.to_string())
            .unwrap_or_default(),
        player_id: pending.player,
        card_id: pending.card,
        zone: Zone::Hand,
        optional: pending.optional,
        snapshot: game
            .object(pending.card)
            .map(|obj| ObjectSnapshot::from_object(obj, game)),
    }
}

pub(crate) fn automatic_reveal_events_for_draw(
    game: &mut GameState,
    player_id: PlayerId,
    drawn: &[ObjectId],
    draws_before: u32,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
    provenance: ProvNodeId,
    hidden_mode: HiddenDrawRevealMode,
) -> Vec<TriggerEvent> {
    let mut reveal_events = Vec::new();

    for candidate in collect_automatic_draw_reveal_candidates(game, player_id, drawn, draws_before)
    {
        // Hidden-information matches: the drawn card is known to its owner
        // only, so the reveal (and the "whenever you reveal ... this way"
        // trigger reading its characteristics) must wait for the owner to
        // open it publicly on every peer (see `hidden_hand_choices`).
        if game.hidden_identity_is_private(candidate.card_id) {
            let pending = pending_hidden_automatic_draw_reveal(&candidate);
            match hidden_mode {
                HiddenDrawRevealMode::Defer => {
                    game.defer_hidden_automatic_draw_reveal(pending);
                }
                HiddenDrawRevealMode::Inline => {
                    let Some(revealed) = game.reveal_private_hidden_cards_publicly(
                        decision_maker,
                        candidate.player_id,
                        candidate.source_id,
                        &[candidate.card_id],
                        hidden_automatic_draw_reveal_description(candidate.optional),
                        candidate.optional,
                    ) else {
                        return reveal_events;
                    };
                    if revealed.contains(&candidate.card_id) {
                        let candidate = automatic_draw_reveal_candidate_for_pending(game, &pending);
                        reveal_events.push(emit_automatic_draw_reveal_event(
                            game,
                            decision_maker,
                            &candidate,
                            provenance,
                        ));
                    }
                }
            }
            continue;
        }
        if candidate.optional {
            let reveal = decision_maker
                .decide_boolean(game, &automatic_draw_reveal_boolean_context(&candidate));
            if decision_maker.awaiting_choice() {
                return reveal_events;
            }
            if !reveal {
                continue;
            }
        }

        reveal_events.push(emit_automatic_draw_reveal_event(
            game,
            decision_maker,
            &candidate,
            provenance,
        ));
    }

    reveal_events
}

/// Effect that causes a player to draw cards.
///
/// Handles replacement effects, "can't draw extra cards" restrictions,
/// and tracks cards drawn this turn for triggered abilities.
///
/// # Fields
///
/// * `count` - Number of cards to draw
/// * `player` - Which player draws (defaults to controller)
///
/// # Example
///
/// ```ignore
/// // Draw 2 cards (you draw)
/// let effect = DrawCardsEffect::you(2);
///
/// // Opponent draws 3 cards
/// let effect = DrawCardsEffect::new(3, PlayerFilter::Opponent);
///
/// // Specific player draws 2 cards
/// let effect = DrawCardsEffect::new(2, PlayerFilter::Specific(player_id));
/// ```
impl EffectExecutor for DrawCardsEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Drawing makes no choices; defer to commit so the whole
        // each-player action lands as one batch.
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let game_checkpoint = game.clone();
        let context_checkpoint = ExecutionContextCheckpoint::capture(ctx);
        let result = execute_draw_instruction(self, game, ctx);
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = game_checkpoint;
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
    }
}

/// Own all draws, replacement programs and reveal decisions in one checkpoint.
/// Publish a completed segment before another program can inspect or change
/// the game. Segments without an intervening replacement program stay batched.
#[allow(clippy::too_many_arguments)]
fn finish_direct_draw_segment(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    drawn: &mut Vec<ObjectId>,
    is_first: bool,
    step_context: (bool, u32),
    draws_before: u32,
    hidden_mode: HiddenDrawRevealMode,
) -> Vec<TriggerEvent> {
    if drawn.is_empty() { return Vec::new(); }
    // Every physical observation has its own identity. Reusing the proposal's
    // provenance lets a later added observation replace this staged draw.
    let draw_provenance = game.provenance_graph_mut()
        .alloc_child_event(ctx.provenance, crate::events::EventKind::CardsDrawn);
    let event = TriggerEvent::new_with_provenance(
        CardsDrawnEvent::new_with_step_context(
            player, std::mem::take(drawn), is_first, step_context.0, step_context.1,
        ), draw_provenance,
    );
    let draw = event.downcast::<CardsDrawnEvent>().expect("draw notification is typed");
    game.record_cards_drawn_in_current_draw_step(player, draw.amount());
    game.note_hidden_draw_for_reveal_window(&event);
    let reveals = automatic_reveal_events_for_draw(
        game, player, &draw.cards, draws_before, &mut *ctx.decision_maker,
        draw_provenance, hidden_mode,
    );
    let mut events = vec![event]; events.extend(reveals); events
}

/// Commit an expanded draw's original result before its appended programs.
fn commit_expanded_draw_original(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requested_player: PlayerId,
    result: TraitEventResult,
) -> Result<EffectOutcome, ExecutionError> {
    commit_draw_original_with_reveal_mode(game, ctx, requested_player, result, HiddenDrawRevealMode::Inline)
}

fn commit_draw_original_with_reveal_mode(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requested_player: PlayerId,
    result: TraitEventResult,
    hidden_mode: HiddenDrawRevealMode,
) -> Result<EffectOutcome, ExecutionError> {
    match result {
        TraitEventResult::Prevented => Ok(EffectOutcome::prevented()),
        TraitEventResult::Replaced { effects, source, controller, context, .. } => {
            execute_scoped_draw_replacement_effects(game, ctx, &effects, source, controller, &context)
        }
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let draw = crate::events::downcast_event::<crate::events::DrawEvent>(event.inner())
                .ok_or_else(|| ExecutionError::InternalError("draw replacement returned an incompatible event".into()))?;
            let player = draw.player;
            if !game.player(player).is_some_and(|player| player.is_in_game()) {
                return Err(ExecutionError::PlayerNotFound(player));
            }
            let count = usize::try_from(draw.count).map_err(|_| ExecutionError::InternalError(
                "resolved draw count exceeds supported range".into(),
            ))?;
            if !game.can_draw(player) { return Ok(EffectOutcome::count(0)); }
            let before = game.turn_store.turn_history.cards_drawn_by_player(player);
            let step = game.draw_step_context_for_player(player);
            let mut drawn = game.draw_cards_with_dm(player, count, &mut *ctx.decision_maker);
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let count = if player == requested_player {
                i32::try_from(drawn.len()).map_err(|_| ExecutionError::InternalError("draw outcome exceeds supported count range".into()))?
            } else { 0 };
            let ids = drawn.clone();
            let events = finish_direct_draw_segment(
                game, ctx, player, &mut drawn,
                draw.is_first_this_turn,
                step, before, hidden_mode,
            );
            Ok(EffectOutcome::count(count).with_result_objects(ids).with_events(events))
        }
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            if ctx.decision_maker.awaiting_choice() { Ok(EffectOutcome::count(0)) }
            else { Err(ExecutionError::InternalError("draw replacement suspended without a captured decision".into())) }
        }
        TraitEventResult::Expanded { .. } => Err(ExecutionError::InternalError("draw expansion did not flatten".into())),
    }
}

/// One turn-based draw proposal, including its original commit and all added
/// programs. The runner owns the private game and replays from before matching.
/// Thus every reveal and replacement decision remains in the same transaction.
pub(crate) fn execute_turn_draw_proposal(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
) -> Result<EffectOutcome, ExecutionError> {
    game.update_replacement_effects().map_err(ExecutionError::ContinuousDiscovery)?;
    let before = game.turn_store.turn_history.cards_drawn_by_player(player);
    let (in_step, step_draws) = game.draw_step_context_for_player(player);
    let event = Event::draw_in_instruction(player, 1, before == 0, true, in_step && step_draws == 0)
        .with_provenance(ctx.provenance);
    let result = process_trait_event_with_execution_context(game, event, ctx)?;
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    let (original, programs) = result.into_expansion();
    let original = commit_draw_original_with_reveal_mode(
        game, ctx, player, original, HiddenDrawRevealMode::Defer,
    )?;
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    // Added instructions must observe the completed original draw's history.
    for event in &original.events { game.stage_turn_history_event(event); }
    let completed = crate::effects::replacement::execute_deferred_replacement_programs(
        game, ctx, original, programs,
    )?;
    Ok(completed)
}

fn execute_draw_instruction(
    effect: &DrawCardsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    let player_id = resolve_player_filter(game, &effect.player, ctx)?;
    let requested_count = resolve_value(game, &effect.count, ctx)?.max(0) as u32;

    // Check for "can't draw extra cards" restriction (e.g., Narset)
    let count = if !game.can_draw_extra_cards(player_id) {
        let current_draws = game
            .turn_store
            .turn_history
            .cards_drawn_by_player(player_id);
        // Player can only draw their first card of the turn
        if current_draws >= 1 {
            // Already drew this turn, can't draw any more
            return Ok(EffectOutcome::prevented());
        }
        // First draw - can only draw 1, not more
        requested_count.min(1)
    } else {
        requested_count
    };

    let mut total_drawn: i32 = 0;
    let mut replacement_count = 0;
    let mut events = Vec::new();
    let mut replacement_facts = Vec::new();
    let mut direct_drawn = Vec::new();
    let mut direct_draw_is_first = false;
    let mut direct_draw_step_context = (false, 0);
    let mut direct_draws_before = 0;

    for index in 0..count {
        if !game.can_draw(player_id) {
            continue;
        }

        let current_draws = game
            .turn_store
            .turn_history
            .cards_drawn_by_player(player_id)
            .saturating_add(direct_drawn.len() as u32);
        let is_first = current_draws == 0;
        let (is_during_players_draw_step, cards_previously_drawn_this_draw_step) =
            game.draw_step_context_for_player(player_id);
        let draw_event = Event::draw_in_instruction(
            player_id,
            1,
            is_first,
            index == 0,
            is_during_players_draw_step && cards_previously_drawn_this_draw_step == 0,
        );
        let processed = process_trait_event_with_execution_context(
            game, draw_event.with_provenance(ctx.provenance), ctx,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let (processed, programs) = processed.into_expansion();
        if !programs.is_empty() {
            events.extend(finish_direct_draw_segment(
                game, ctx, player_id, &mut direct_drawn, direct_draw_is_first,
                direct_draw_step_context, direct_draws_before, HiddenDrawRevealMode::Inline,
            ));
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let original = commit_expanded_draw_original(game, ctx, player_id, processed)?;
            let completed = crate::effects::replacement::execute_deferred_replacement_programs(
                game, ctx, original, programs,
            )?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            total_drawn = total_drawn.checked_add(completed.count_or_zero())
                .ok_or_else(|| ExecutionError::InternalError("draw outcome exceeds supported count range".into()))?;
            events.extend(completed.events);
            replacement_facts.extend(completed.execution_facts);
            continue;
        }
        match processed {
            TraitEventResult::Expanded { .. } => return Err(ExecutionError::InternalError("draw expansion did not flatten".into())),
            TraitEventResult::Prevented => continue,
            TraitEventResult::Replaced {
                effects,
                source,
                controller,
                context,
                ..
            } => {
                let replacement_outcome = execute_scoped_draw_replacement_effects(
                    game, ctx, &effects, source, controller, &context,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                replacement_count += replacement_outcome.count_or_zero();
                events.extend(replacement_outcome.events);
                replacement_facts.extend(replacement_outcome.execution_facts);
                continue;
            }
            TraitEventResult::NeedsChoice { .. }
            | TraitEventResult::NeedsInteraction { .. } => {
                // The real pending-input path returned before flattening.
                // A completed malformed answer must not publish a partial draw.
                return Err(ExecutionError::InternalError(
                    "draw replacement suspended without a captured decision".into(),
                ));
            }
            TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
                let final_draw = Some(
                    crate::events::downcast_event::<crate::events::DrawEvent>(e.inner())
                        .ok_or_else(|| ExecutionError::InternalError(
                            "draw replacement returned an incompatible event".into(),
                        ))?,
                );
                let final_count = final_draw.unwrap().count;
                // A redirect replacement ("instead that player skips that
                // draw and you draw a card", Notion Thief) changes who
                // draws; the card goes to that player's hand.
                if let Some(redirected_player) = final_draw
                    .map(|draw| draw.player)
                    .filter(|drawer| *drawer != player_id)
                {
                    if !game.can_draw(redirected_player) {
                        continue;
                    }
                    let redirected_is_first = game
                        .turn_store
                        .turn_history
                        .cards_drawn_by_player(redirected_player)
                        == 0;
                    let (redirected_in_draw_step, redirected_previous) =
                        game.draw_step_context_for_player(redirected_player);
                    let drawn = game.draw_cards_with_dm(
                        redirected_player,
                        final_count as usize,
                        &mut *ctx.decision_maker,
                    );
                    if drawn.is_empty() {
                        continue;
                    }
                    let event = TriggerEvent::new_with_provenance(
                        CardsDrawnEvent::new_with_step_context(
                            redirected_player,
                            drawn,
                            redirected_is_first,
                            redirected_in_draw_step,
                            redirected_previous,
                        ),
                        ctx.provenance,
                    );
                    let drawn_count = event
                        .downcast::<CardsDrawnEvent>()
                        .map(CardsDrawnEvent::amount)
                        .unwrap_or(0);
                    game.record_cards_drawn_in_current_draw_step(
                        redirected_player,
                        drawn_count,
                    );
                    game.note_hidden_draw_for_reveal_window(&event);
                    events.push(event);
                    continue;
                }

                let drawn = game.draw_cards_with_dm(
                    player_id,
                    final_count as usize,
                    &mut *ctx.decision_maker,
                );

                // Only emit event if cards were actually drawn
                if drawn.is_empty() {
                    continue;
                }
                let drawn_len = drawn.len() as i32;
                if direct_drawn.is_empty() {
                    direct_draw_is_first = is_first;
                    direct_draw_step_context = (
                        is_during_players_draw_step,
                        cards_previously_drawn_this_draw_step,
                    );
                    direct_draws_before = current_draws;
                }
                total_drawn += drawn_len;
                direct_drawn.extend(drawn);
            }
        }
    }

    events.extend(finish_direct_draw_segment(
        game, ctx, player_id, &mut direct_drawn, direct_draw_is_first,
        direct_draw_step_context, direct_draws_before, HiddenDrawRevealMode::Inline,
    ));

    Ok(EffectOutcome::count(total_drawn + replacement_count).with_events(events)
        .with_execution_facts(EffectOutcome::merge_execution_facts(replacement_facts)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_cards_to_library(game: &mut GameState, owner: PlayerId, count: usize) {
        for i in 1..=count {
            let card = CardBuilder::new(CardId::new(), format!("Library Card {}", i))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&card, owner, Zone::Library);
        }
    }

    fn add_static_source(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        ability: crate::static_abilities::StaticAbility,
    ) -> ObjectId {
        let source_card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, owner, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(ability));
        source
    }

    fn empty_library_draw_win_ability() -> crate::static_abilities::StaticAbility {
        crate::static_abilities::StaticAbility::conditional_draw_replacement(
            crate::effect::Condition::ValueComparison {
                left: crate::effect::Value::CardsInLibrary(crate::target::PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: crate::effect::Value::Fixed(0),
            },
            vec![crate::effect::Effect::win_the_game()],
            false,
            "If you would draw a card while your library has no cards in it, you win the game instead.",
        )
    }

    fn add_graveyard_dredger(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        amount: u32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&card, owner, Zone::Graveyard);
        game.object_mut(source)
            .expect("dredger exists")
            .abilities_mut()
            .push(
                crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::dredge(amount),
                )
                .in_zones(vec![Zone::Graveyard]),
            );
        source
    }

    struct ChooseReplacementNamed(&'static str);

    impl crate::decision::DecisionMaker for ChooseReplacementNamed {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx
                .description
                .eq_ignore_ascii_case("Choose which replacement effect to apply")
                && let Some(option) = ctx
                    .options
                    .iter()
                    .find(|option| option.description.contains(self.0))
            {
                return vec![option.index];
            }

            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    #[test]
    fn test_draw_cards_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        assert_eq!(game.player(alice).unwrap().library.len(), 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
    }

    #[test]
    fn test_draw_cards_tracks_drawn_this_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // First draw
        let effect = DrawCardsEffect::you(2);
        crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .unwrap();

        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 2);

        // Second draw
        let effect = DrawCardsEffect::you(1);
        crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .unwrap();

        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 3);
    }

    #[test]
    fn test_draw_cards_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // No cards in library
        assert_eq!(game.player(alice).unwrap().library.len(), 0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Can't draw from empty library
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert!(
            game.player(alice)
                .unwrap()
                .attempted_draw_from_empty_library
        );
    }

    #[test]
    fn test_draw_cards_partial_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 2);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only draw what's available
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 0);
        assert!(
            game.player(alice)
                .unwrap()
                .attempted_draw_from_empty_library
        );
    }

    #[test]
    fn empty_library_draw_win_replaces_each_impossible_draw_before_the_loss_flag() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        add_cards_to_library(&mut game, alice, 1);
        let laboratory_maniac = add_static_source(
            &mut game,
            alice,
            "Laboratory Maniac",
            empty_library_draw_win_ability(),
        );
        let mut ctx = ExecutionContext::new_default(laboratory_maniac, alice);

        let result = DrawCardsEffect::you(2)
            .execute(&mut game, &mut ctx)
            .expect("the second draw should be replaced with a win");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        let alice_state = game.player(alice).expect("alice exists");
        assert_eq!(
            alice_state.hand.len(),
            1,
            "the first draw should still happen"
        );
        assert!(alice_state.library.is_empty());
        assert!(
            !alice_state.attempted_draw_from_empty_library,
            "the replaced second draw must not set the CR 704.5b loss flag"
        );
        assert!(
            !game.player(bob).expect("bob exists").is_in_game(),
            "the replacement's win effect should resolve"
        );
    }

    #[test]
    fn empty_library_draw_win_only_matches_its_controllers_draw() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let laboratory_maniac = add_static_source(
            &mut game,
            alice,
            "Laboratory Maniac",
            empty_library_draw_win_ability(),
        );
        let mut ctx = ExecutionContext::new_default(laboratory_maniac, alice);

        DrawCardsEffect::new(1, crate::target::PlayerFilter::Specific(bob))
            .execute(&mut game, &mut ctx)
            .expect("Bob's ordinary empty-library draw should resolve");

        assert!(
            game.player(bob)
                .expect("bob exists")
                .attempted_draw_from_empty_library,
            "Alice's replacement must not apply to an opponent"
        );
        assert!(game.player(alice).expect("alice exists").is_in_game());
        assert!(game.player(bob).expect("bob exists").is_in_game());
    }

    #[test]
    fn failed_empty_library_draw_win_is_still_replaced_when_winning_is_forbidden() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let laboratory_maniac = add_static_source(
            &mut game,
            alice,
            "Laboratory Maniac",
            empty_library_draw_win_ability(),
        );
        add_static_source(
            &mut game,
            bob,
            "Your Opponents Can't Win",
            crate::static_abilities::StaticAbility::opponents_cant_win_game(),
        );
        game.update_cant_effects();
        let mut ctx = ExecutionContext::new_default(laboratory_maniac, alice);

        let result = DrawCardsEffect::you(1)
            .execute(&mut game, &mut ctx)
            .expect("the replacement should resolve even when its win is prevented");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(game.player(alice).expect("alice exists").is_in_game());
        assert!(game.player(bob).expect("bob exists").is_in_game());
        assert!(
            !game
                .player(alice)
                .expect("alice exists")
                .attempted_draw_from_empty_library,
            "a failed replacement action does not restore the replaced draw"
        );
    }

    #[test]
    fn test_draw_cards_respects_double_draw_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        let source = game.new_object_id();
        game.effect_store.replacement_effects.add_resolution_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Modify(
                    crate::replacement::EventModification::Multiply(2),
                ),
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
    }

    #[test]
    fn test_draw_cards_executes_instead_draw_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 3);
        let source = add_static_source(
            &mut game,
            alice,
            "Draw Replacer",
            crate::static_abilities::StaticAbility::draw_replacement_exile_top_face_down(),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 1);
        assert_eq!(game.exile.len(), 2);
    }

    #[test]
    fn draw_cards_effect_declines_one_dredge_then_uses_another() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_cards_to_library(&mut game, alice, 4);
        let first = add_graveyard_dredger(&mut game, alice, "First Dredger", 2);
        let second = add_graveyard_dredger(&mut game, alice, "Second Dredger", 3);
        let mut dm = ChooseReplacementNamed("Do not apply First Dredger");
        let mut ctx = ExecutionContext::new(second, alice, &mut dm);

        let result = DrawCardsEffect::you(1)
            .execute(&mut game, &mut ctx)
            .expect("dredge replacement should execute through DrawCardsEffect");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        let player = game.player(alice).expect("alice");
        assert_eq!(player.library.len(), 1);
        assert_eq!(player.graveyard.len(), 4);
        assert!(player.graveyard.contains(&first));
        assert_eq!(
            game.current_name(player.hand[0]).as_deref(),
            Some("Second Dredger")
        );
    }

    #[test]
    fn draw_replacement_choice_contains_all_dredges_declines_and_other_replacements() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_cards_to_library(&mut game, alice, 5);
        let first = add_graveyard_dredger(&mut game, alice, "First Dredger", 2);
        let second = add_graveyard_dredger(&mut game, alice, "Second Dredger", 3);
        let other = add_static_source(
            &mut game,
            alice,
            "Thought Reflection",
            crate::static_abilities::StaticAbility::draw_replacement_double(),
        );
        game.update_replacement_effects().unwrap();

        let result = crate::events::processing::process_trait_event(
            &mut game,
            crate::events::Event::draw(alice, 1, true),
        );
        let crate::events::processing::TraitEventResult::NeedsChoice {
            applicable_effects, ..
        } = result
        else {
            panic!("all equal-priority draw replacements should share one CR 616 choice");
        };
        let effects: Vec<_> = applicable_effects
            .iter()
            .filter_map(|id| game.effect_store.replacement_effects.get_effect(*id))
            .collect();
        assert_eq!(effects.len(), 5);
        for dredger in [first, second] {
            assert_eq!(
                effects
                    .iter()
                    .filter(|effect| effect.source == dredger)
                    .count(),
                2,
                "each dredger should contribute its replacement and explicit decline"
            );
            assert!(effects.iter().any(|effect| {
                effect.source == dredger
                    && matches!(
                        &effect.replacement,
                        crate::replacement::ReplacementAction::DeclineOptional(_)
                    )
            }));
        }
        assert_eq!(
            effects
                .iter()
                .filter(|effect| effect.source == other)
                .count(),
            1
        );
    }

    #[test]
    fn optional_conditional_draw_replacement_can_be_applied_or_declined() {
        fn always() -> crate::effect::Condition {
            crate::effect::Condition::ValueComparison {
                left: crate::effect::Value::Fixed(1),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: crate::effect::Value::Fixed(1),
            }
        }

        for (choice, expected_hand, expected_counters) in [
            ("Replacement Source", 0, 1),
            ("Do not apply Replacement Source", 1, 0),
        ] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            add_cards_to_library(&mut game, alice, 1);
            let source = add_static_source(
                &mut game,
                alice,
                "Replacement Source",
                crate::static_abilities::StaticAbility::conditional_draw_replacement(
                    always(),
                    vec![crate::effect::Effect::put_counters_on_source(
                        crate::object::CounterType::Study,
                        1,
                    )],
                    true,
                    "Pursuit-style replacement",
                ),
            );
            let mut dm = ChooseReplacementNamed(choice);
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);

            DrawCardsEffect::you(1)
                .execute(&mut game, &mut ctx)
                .expect("optional draw replacement should resolve");

            assert_eq!(game.player(alice).unwrap().hand.len(), expected_hand);
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&crate::object::CounterType::Study)
                    .copied()
                    .unwrap_or(0),
                expected_counters,
            );
        }
    }

    #[test]
    fn test_draw_replacement_double_executes_nested_draws() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        let source = add_static_source(
            &mut game,
            alice,
            "Thought Reflection",
            crate::static_abilities::StaticAbility::draw_replacement_double(),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
    }

    #[test]
    fn test_draw_replacement_double_allows_other_replacements_on_nested_draws() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        let asmodeus = add_static_source(
            &mut game,
            alice,
            "Asmodeus the Archfiend",
            crate::static_abilities::StaticAbility::draw_replacement_exile_top_face_down(),
        );
        add_static_source(
            &mut game,
            alice,
            "Thought Reflection",
            crate::static_abilities::StaticAbility::draw_replacement_double(),
        );
        let mut dm = ChooseReplacementNamed("Thought Reflection");
        let mut ctx = ExecutionContext::new(asmodeus, alice, &mut dm);

        let effect = DrawCardsEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
        assert_eq!(game.exile.len(), 2);
    }

    #[test]
    fn test_draw_replacement_instead_can_preempt_double_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        let asmodeus = add_static_source(
            &mut game,
            alice,
            "Asmodeus the Archfiend",
            crate::static_abilities::StaticAbility::draw_replacement_exile_top_face_down(),
        );
        add_static_source(
            &mut game,
            alice,
            "Thought Reflection",
            crate::static_abilities::StaticAbility::draw_replacement_double(),
        );
        let mut dm = ChooseReplacementNamed("Asmodeus the Archfiend");
        let mut ctx = ExecutionContext::new(asmodeus, alice, &mut dm);

        let effect = DrawCardsEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 4);
        assert_eq!(game.exile.len(), 1);
    }

    #[test]
    fn test_draw_cards_executes_static_instead_replacement_for_each_requested_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 3);
        let source = add_static_source(
            &mut game,
            alice,
            "Asmodeus the Archfiend",
            crate::static_abilities::StaticAbility::draw_replacement_exile_top_face_down(),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 1);
        assert_eq!(game.exile.len(), 2);
    }

    #[test]
    fn test_draw_cards_for_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        add_cards_to_library(&mut game, bob, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Alice makes Bob draw
        let effect = DrawCardsEffect::new(2, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(bob).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
    }

    #[test]
    fn test_draw_cards_variable_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 10);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(3);

        let effect = DrawCardsEffect::new(Value::X, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        assert_eq!(game.player(alice).unwrap().hand.len(), 3);
    }

    #[test]
    fn test_draw_cards_clone_box() {
        let effect = DrawCardsEffect::you(2);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DrawCardsEffect"));
    }

    #[test]
    fn test_draw_cards_returns_events() {
        use crate::events::EventKind;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have 1 CardsDrawnEvent containing all 3 cards
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::CardsDrawn);

        let event = result.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert_eq!(event.cards.len(), 3);
        assert!(event.is_first_this_turn);
    }

    #[test]
    fn test_draw_cards_first_draw_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // First draw of turn
        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let event = result.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert!(event.is_first_this_turn);
        assert_eq!(event.cards.len(), 2);
        game.stage_turn_history_event(&result.events[0]);

        // Second draw of turn
        let effect2 = DrawCardsEffect::you(1);
        let result2 = effect2.execute(&mut game, &mut ctx).unwrap();

        let event2 = result2.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert!(!event2.is_first_this_turn); // Not first draw anymore
    }
}
