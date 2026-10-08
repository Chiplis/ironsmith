//! DrawCards effect implementation.

use crate::decision::DecisionMaker;
use crate::decisions::context::BooleanContext;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
#[cfg(test)]
use crate::events::CardRevealedEvent;
use crate::events::processing::{
    ReplacementEventContext, TraitEventResult, process_trait_event_with_execution_context,
};
use crate::events::{CardsDrawnEvent, Event};
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
    let replaced_player =
        crate::events::downcast_event::<crate::events::DrawEvent>(context.event.inner())
            .ok_or_else(|| {
                ExecutionError::InternalError("draw replacement lost its draw event".into())
            })?
            .player;
    let mut outcome = crate::effects::replacement::execute_replacement_payload(
        game,
        ctx,
        effects,
        replacement_source,
        replacement_controller,
        context,
        None,
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    let drawn_count = outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<CardsDrawnEvent>())
        .filter(|event| event.player == replaced_player)
        .try_fold(0i64, |total, event| {
            let amount = i64::try_from(event.amount()).map_err(|_| {
                ExecutionError::InternalError(
                    "draw replacement outcome exceeds the supported count range".into(),
                )
            })?;
            total.checked_add(amount).ok_or_else(|| {
                ExecutionError::InternalError(
                    "draw replacement outcome exceeds the supported count range".into(),
                )
            })
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
    pub source_snapshot: ObjectSnapshot,
    pub occurrence: crate::events::other::FirstDrawRevealOccurrence,
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

fn revealed_draw_snapshot(game: &GameState, card: ObjectId) -> Option<ObjectSnapshot> {
    let object = game.object(card)?;
    let mut snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(object, game);
    if !game.is_hidden_card_placeholder(card) {
        snapshot.revealed_cast_definition = Some(std::sync::Arc::new(object.to_card_definition()));
    }
    Some(snapshot)
}

pub(crate) fn collect_automatic_draw_reveal_candidates(
    game: &GameState,
    player_id: PlayerId,
    drawn: &[ObjectId],
    draws_before: u32,
) -> Result<Vec<AutomaticDrawRevealCandidate>, ExecutionError> {
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    let mut candidates = Vec::new();
    let draws_after = draws_before.saturating_add(drawn.len() as u32);

    for &source_id in &game.battlefield {
        let Some(source_obj) = game.object(source_id) else {
            continue;
        };
        let Some(characteristics) = view.calculated_characteristics_arc(source_id) else {
            continue;
        };
        if characteristics.controller != player_id {
            continue;
        }
        for (ability_index, ability) in characteristics.abilities.iter().enumerate() {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if !ability.functions_in(&source_obj.zone) {
                continue;
            }
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

            let object = game
                .object(card_id)
                .filter(|object| object.zone == Zone::Hand && object.owner == player_id)
                .ok_or(ExecutionError::InvalidTarget)?;
            let owner = crate::linked_exile::LinkedExileOwner::capture(
                source_id,
                spec.linked_reveal_pair,
                characteristics.abilities.origin(ability_index),
            );
            if spec.linked_reveal_pair.is_some() && owner.is_none() {
                return Err(ExecutionError::IncompleteEvidence(
                    "first-draw reveal lacks its exact rules acquisition; native recovery or replay required".into(),
                ));
            }
            let snapshot = revealed_draw_snapshot(game, card_id);
            let source_snapshot = ObjectSnapshot::from_object_with_known_characteristics(
                source_obj,
                game,
                Some(&characteristics),
            );
            let occurrence = crate::events::other::FirstDrawRevealOccurrence {
                owner,
                drawn_card: card_id,
                drawn_stable_id: object.stable_id,
                player: player_id,
                card_number: draw_number,
            };
            candidates.push(AutomaticDrawRevealCandidate {
                source_id,
                source_name: source_obj.name.to_string(),
                player_id,
                card_id,
                zone: Zone::Hand,
                optional: spec.optional,
                snapshot,
                source_snapshot,
                occurrence,
            });
        }
    }

    Ok(candidates)
}

pub(crate) fn emit_automatic_draw_reveal_event(
    game: &mut GameState,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
    candidate: &AutomaticDrawRevealCandidate,
    provenance: ProvNodeId,
) -> TriggerEvent {
    for viewer_idx in 0..game.players.len() {
        let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
        super::public_reveal_view(
            game,
            decision_maker,
            viewer,
            candidate.player_id,
            candidate.source_id,
            candidate.zone,
            &[candidate.card_id],
            "Reveal drawn card",
        );
    }

    let provenance = game
        .provenance_graph_mut()
        .alloc_child_event(provenance, crate::events::EventKind::CardRevealed);
    super::public_reveal_observation(
        candidate.player_id,
        candidate.card_id,
        candidate.zone,
        candidate.source_id,
        candidate.snapshot.clone(),
        None,
        Some(candidate.occurrence.clone()),
        provenance,
    )
    .with_source_snapshot(candidate.source_snapshot.clone())
    .with_lookback_source_snapshots(vec![candidate.source_snapshot.clone()])
}

/// One immutable draw observation for effect and turn-based draw adapters.
/// The physical draw owner supplies its occurrence provenance and step facts;
/// delayed identity disclosure retains this receipt instead of rebuilding it.
pub(crate) fn draw_observation(
    game: &GameState,
    player: PlayerId,
    cards: Vec<ObjectId>,
    is_first: bool,
    step_context: (bool, u32),
    provenance: ProvNodeId,
) -> TriggerEvent {
    draw_observation_with_miracle(
        game,
        player,
        cards,
        is_first,
        step_context,
        provenance,
        None,
    )
}

fn draw_observation_with_miracle(
    game: &GameState,
    player: PlayerId,
    cards: Vec<ObjectId>,
    is_first: bool,
    step_context: (bool, u32),
    provenance: ProvNodeId,
    miracle: Option<crate::events::other::MiracleDrawDecision>,
) -> TriggerEvent {
    let snapshots = cards
        .iter()
        .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
        .collect();
    TriggerEvent::new_with_provenance(
        CardsDrawnEvent::new_with_step_context(
            player,
            cards,
            is_first,
            step_context.0,
            step_context.1,
        )
        .with_snapshots(snapshots)
        .with_miracle_decision(miracle),
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
        occurrence: candidate.occurrence.clone(),
        source_snapshot: candidate.source_snapshot.clone(),
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
        snapshot: revealed_draw_snapshot(game, pending.card),
        source_snapshot: pending.source_snapshot.clone(),
        occurrence: pending.occurrence.clone(),
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
) -> Result<Vec<TriggerEvent>, ExecutionError> {
    game.refresh_continuous_state()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let mut reveal_events = Vec::new();

    for candidate in collect_automatic_draw_reveal_candidates(game, player_id, drawn, draws_before)?
    {
        // Hidden-information matches: the drawn card is known to its owner
        // only, so the reveal (and the "whenever you reveal ... this way"
        // trigger reading its characteristics) must wait for the owner to
        // open it publicly on every peer (see `hidden_hand_choices`).
        if game.hidden_identity_is_private(candidate.card_id) {
            let pending = pending_hidden_automatic_draw_reveal(&candidate);
            match hidden_mode {
                HiddenDrawRevealMode::Defer => {
                    game.defer_hidden_automatic_draw_reveal(pending, provenance);
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
                        return Ok(reveal_events);
                    };
                    if revealed.contains(&candidate.card_id) {
                        if game.is_hidden_card_placeholder(candidate.card_id) {
                            return Err(ExecutionError::IncompleteEvidence(
                                "first-draw revealed identity was not authenticated on this peer"
                                    .into(),
                            ));
                        }
                        game.refresh_continuous_state()
                            .map_err(ExecutionError::ContinuousDiscovery)?;
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
                return Ok(reveal_events);
            }
            if !reveal {
                continue;
            }
        }

        // A preceding independent reveal may have opened this hidden card.
        // Refresh only its revealed characteristics; never recapture ownership.
        let candidate = automatic_draw_reveal_candidate_for_pending(
            game,
            &pending_hidden_automatic_draw_reveal(&candidate),
        );
        reveal_events.push(emit_automatic_draw_reveal_event(
            game,
            decision_maker,
            &candidate,
            provenance,
        ));
        if decision_maker.awaiting_choice() {
            return Ok(reveal_events);
        }
    }

    Ok(reveal_events)
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
    fn as_cost_executable(&self) -> Option<&dyn crate::effects::CostExecutableEffect> {
        Some(self)
    }
    fn directly_mentions_player_filter(&self, needle: &crate::target::PlayerFilter) -> bool {
        self.player.mentions_player_filter(needle)
    }
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Drawn)
    }
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
        crate::effects::composition::execute_transaction_from_body(
            game,
            ctx,
            || EffectOutcome::count(0),
            |game, ctx| execute_draw_instruction(self, game, ctx),
        )
    }
}

impl crate::effects::CostExecutableEffect for DrawCardsEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        let player = resolve_player_filter(game, &self.player, &ctx)
            .map_err(|error| crate::effects::CostValidationError::Other(error.to_string()))?;
        let count = resolve_value(game, &self.count, &ctx)
            .map_err(|error| crate::effects::CostValidationError::Other(error.to_string()))?
            .max(0);
        if count == 0 {
            return Ok(());
        }
        if !game.can_draw(player) {
            return Err(crate::effects::CostValidationError::NotEnoughCards);
        }
        let has_drawn = game
            .turn_store
            .turn_history
            .has_drawn_cards_this_turn(player)
            .map_err(crate::effects::CostValidationError::ExecutionFailed)?;
        if game.can_draw_extra_cards(player) || (count == 1 && !has_drawn) {
            Ok(())
        } else {
            Err(crate::effects::CostValidationError::NotEnoughCards)
        }
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
    miracle: &mut Option<crate::events::other::MiracleDrawDecision>,
    automatic_reveals: &mut Vec<TriggerEvent>,
) -> Vec<TriggerEvent> {
    if drawn.is_empty() {
        return Vec::new();
    }
    // Every physical observation has its own identity. Reusing the proposal's
    // provenance lets a later added observation replace this staged draw.
    let draw_provenance = game
        .provenance_graph_mut()
        .alloc_child_event(ctx.provenance, crate::events::EventKind::CardsDrawn);
    let event = draw_observation_with_miracle(
        game,
        player,
        std::mem::take(drawn),
        is_first,
        step_context,
        draw_provenance,
        miracle.take(),
    );
    let draw = event
        .downcast::<CardsDrawnEvent>()
        .expect("draw notification is typed");
    game.record_cards_drawn_in_current_draw_step(player, draw.amount());
    // Physical order is already established even when trigger matching is held.
    // Added programs can draw again before the enclosing outcome is published.
    game.stage_turn_history_event(&event);
    game.note_hidden_draw_for_reveal_window(&event);
    let reveals = super::miracle_reveal_event(game, draw, draw_provenance);
    let mut events = vec![event];
    events.extend(reveals);
    events.append(automatic_reveals);
    events
}

/// Commit an expanded draw's original result before its appended programs.
fn commit_expanded_draw_original(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requested_player: PlayerId,
    result: TraitEventResult,
    prior_events: &[TriggerEvent],
) -> Result<EffectOutcome, ExecutionError> {
    commit_draw_original_with_reveal_mode(
        game,
        ctx,
        requested_player,
        result,
        prior_events,
        HiddenDrawRevealMode::Inline,
    )
}

fn commit_draw_original_with_reveal_mode(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requested_player: PlayerId,
    result: TraitEventResult,
    prior_events: &[TriggerEvent],
    _hidden_mode: HiddenDrawRevealMode,
) -> Result<EffectOutcome, ExecutionError> {
    match result {
        TraitEventResult::Prevented => Ok(EffectOutcome::prevented()),
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => execute_scoped_draw_replacement_effects(
            game, ctx, &effects, source, controller, &context,
        ),
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let draw = crate::events::downcast_event::<crate::events::DrawEvent>(event.inner())
                .ok_or_else(|| {
                    ExecutionError::InternalError(
                        "draw replacement returned an incompatible event".into(),
                    )
                })?;
            let player = draw.player;
            if !game
                .player(player)
                .is_some_and(|player| player.is_in_game())
            {
                return Err(ExecutionError::PlayerNotFound(player));
            }
            let count = usize::try_from(draw.count).map_err(|_| {
                ExecutionError::InternalError("resolved draw count exceeds supported range".into())
            })?;
            if !game.can_draw(player) {
                return Ok(EffectOutcome::count(0));
            }
            let is_first = !game
                .turn_store
                .turn_history
                .has_drawn_cards_this_turn(player)?
                && !prior_events.iter().any(|event| {
                    event
                        .downcast::<CardsDrawnEvent>()
                        .is_some_and(|draw| draw.player == player && !draw.cards.is_empty())
                });
            let step = game.draw_step_context_for_player(player);
            let completed = super::draw_cards_with_miracle_window(
                game,
                player,
                count,
                is_first,
                &mut *ctx.decision_maker,
                ctx.provenance,
            )?;
            let mut drawn = completed.cards;
            let mut miracle = completed.miracle;
            let mut automatic_reveals = completed.automatic_reveals;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let count = if player == requested_player {
                i64::try_from(drawn.len()).map_err(|_| {
                    ExecutionError::InternalError(
                        "draw outcome exceeds supported count range".into(),
                    )
                })?
            } else {
                0
            };
            let ids = drawn.clone();
            let events = finish_direct_draw_segment(
                game,
                ctx,
                player,
                &mut drawn,
                is_first,
                step,
                &mut miracle,
                &mut automatic_reveals,
            );
            Ok(EffectOutcome::count(count)
                .with_result_objects(ids)
                .with_events(events))
        }
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            if ctx.decision_maker.awaiting_choice() {
                Ok(EffectOutcome::count(0))
            } else {
                Err(ExecutionError::InternalError(
                    "draw replacement suspended without a captured decision".into(),
                ))
            }
        }
        TraitEventResult::Expanded { .. } => Err(ExecutionError::InternalError(
            "draw expansion did not flatten".into(),
        )),
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
    game.update_replacement_effects()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let is_first = !game
        .turn_store
        .turn_history
        .has_drawn_cards_this_turn(player)?;
    let (in_step, step_draws) = game.draw_step_context_for_player(player);
    let event = Event::draw_in_instruction(player, 1, is_first, true, in_step && step_draws == 0)
        .with_provenance(ctx.provenance);
    let result = process_trait_event_with_execution_context(game, event, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    let (original, programs) = result.into_expansion();
    let original = commit_draw_original_with_reveal_mode(
        game,
        ctx,
        player,
        original,
        &[],
        HiddenDrawRevealMode::Defer,
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    // Added instructions must observe the completed original draw's history.
    for event in &original.events {
        game.stage_turn_history_event(event);
    }
    let completed = crate::effects::replacement::execute_deferred_replacement_programs(
        game, ctx, original, programs,
    )?;
    Ok(completed)
}

/// The reached draw instruction owns this quantity and recipient even when
/// an enclosing replacement must finish its original action before drawing.
#[derive(Clone, Copy)]
pub(crate) struct PreparedDrawInstruction {
    player: PlayerId,
    pub(crate) requested_count: u32,
}
pub(crate) fn prepare_draw_instruction(
    effect: &DrawCardsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<PreparedDrawInstruction, ExecutionError> {
    Ok(PreparedDrawInstruction {
        player: resolve_player_filter(game, &effect.player, ctx)?,
        requested_count: resolve_value(game, &effect.count, ctx)?.max(0) as u32,
    })
}
fn execute_draw_instruction(
    effect: &DrawCardsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    let prepared = prepare_draw_instruction(effect, game, ctx)?;
    execute_prepared_draw_instruction(prepared, game, ctx)
}
pub(crate) fn execute_prepared_draw_instruction(
    prepared: PreparedDrawInstruction,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    let PreparedDrawInstruction {
        player: player_id,
        requested_count,
    } = prepared;
    if requested_count == 0 {
        return Ok(EffectOutcome::count(0));
    }

    // Check for "can't draw extra cards" restriction (e.g., Narset)
    let count = if !game.can_draw_extra_cards(player_id) {
        let has_drawn = game
            .turn_store
            .turn_history
            .has_drawn_cards_this_turn(player_id)?;
        // Player can only draw their first card of the turn
        if has_drawn {
            // Already drew this turn, can't draw any more
            return Ok(EffectOutcome::prevented());
        }
        // First draw - can only draw 1, not more
        requested_count.min(1)
    } else {
        requested_count
    };

    let mut total_drawn: i64 = 0;
    let mut replacement_count = 0;
    let mut events = Vec::new();
    let mut replacement_facts = Vec::new();
    let mut original_draw_facts = Vec::new();
    let mut direct_drawn = Vec::new();
    let mut direct_draw_is_first = false;
    let mut direct_miracle = None;
    let mut direct_automatic_reveals = Vec::new();
    let mut direct_draw_step_context = (false, 0);

    for index in 0..count {
        if !game.can_draw(player_id) {
            continue;
        }

        let is_first = !game
            .turn_store
            .turn_history
            .has_drawn_cards_this_turn(player_id)?
            && direct_drawn.is_empty()
            && !events.iter().any(|event: &TriggerEvent| {
                event
                    .downcast::<CardsDrawnEvent>()
                    .is_some_and(|draw| draw.player == player_id && !draw.cards.is_empty())
            });
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
            game,
            draw_event.with_provenance(ctx.provenance),
            ctx,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let (processed, programs) = processed.into_expansion();
        if !programs.is_empty() || matches!(&processed, TraitEventResult::Replaced { .. }) {
            // Earlier cards were already drawn. Their event-time subjects
            // cannot observe changes made by this later draw's replacement.
            // Keep the physical receipts while recording their matched proof.
            let segment = finish_direct_draw_segment(
                game,
                ctx,
                player_id,
                &mut direct_drawn,
                direct_draw_is_first,
                direct_draw_step_context,
                &mut direct_miracle,
                &mut direct_automatic_reveals,
            );
            original_draw_facts.extend(
                segment
                    .iter()
                    .flat_map(|event| crate::effects::outcome_recording::event_facts(game, event)),
            );
            events.extend(segment);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            crate::effects::capture_triggers_before_added_program(
                game,
                ctx,
                None,
                events.iter_mut(),
            )?;
        }
        if !programs.is_empty() {
            let original = commit_expanded_draw_original(game, ctx, player_id, processed, &events)?;
            original_draw_facts.extend(
                original
                    .events
                    .iter()
                    .flat_map(|event| crate::effects::outcome_recording::event_facts(game, event)),
            );
            let completed = crate::effects::replacement::execute_deferred_replacement_programs(
                game, ctx, original, programs,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            total_drawn = total_drawn
                .checked_add(completed.count_or_zero())
                .ok_or_else(|| {
                    ExecutionError::InternalError(
                        "draw outcome exceeds supported count range".into(),
                    )
                })?;
            events.extend(completed.events);
            replacement_facts.extend(completed.execution_facts);
            continue;
        }
        match processed {
            TraitEventResult::Expanded { .. } => {
                return Err(ExecutionError::InternalError(
                    "draw expansion did not flatten".into(),
                ));
            }
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
                original_draw_facts.extend(
                    replacement_outcome
                        .instruction_result()
                        .events
                        .iter()
                        .flat_map(|event| {
                            crate::effects::outcome_recording::event_facts(game, event)
                        }),
                );
                events.extend(replacement_outcome.events);
                replacement_facts.extend(replacement_outcome.execution_facts);
                continue;
            }
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                // The real pending-input path returned before flattening.
                // A completed malformed answer must not publish a partial draw.
                return Err(ExecutionError::InternalError(
                    "draw replacement suspended without a captured decision".into(),
                ));
            }
            TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
                let final_draw = Some(
                    crate::events::downcast_event::<crate::events::DrawEvent>(e.inner())
                        .ok_or_else(|| {
                            ExecutionError::InternalError(
                                "draw replacement returned an incompatible event".into(),
                            )
                        })?,
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
                    // Earlier direct cards precede this recipient change even
                    // when their enclosing instruction has not been published.
                    let segment = finish_direct_draw_segment(
                        game,
                        ctx,
                        player_id,
                        &mut direct_drawn,
                        direct_draw_is_first,
                        direct_draw_step_context,
                        &mut direct_miracle,
                        &mut direct_automatic_reveals,
                    );
                    original_draw_facts.extend(segment.iter().flat_map(|event| {
                        crate::effects::outcome_recording::event_facts(game, event)
                    }));
                    events.extend(segment);
                    let redirected_is_first = !game
                        .turn_store
                        .turn_history
                        .has_drawn_cards_this_turn(redirected_player)?
                        && !events.iter().any(|event: &TriggerEvent| {
                            event.downcast::<CardsDrawnEvent>().is_some_and(|draw| {
                                draw.player == redirected_player && !draw.cards.is_empty()
                            })
                        });
                    let (redirected_in_draw_step, redirected_previous) =
                        game.draw_step_context_for_player(redirected_player);
                    let completed = super::draw_cards_with_miracle_window(
                        game,
                        redirected_player,
                        final_count as usize,
                        redirected_is_first,
                        &mut *ctx.decision_maker,
                        ctx.provenance,
                    )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    let drawn = completed.cards;
                    if drawn.is_empty() {
                        continue;
                    }
                    let redirected_provenance = game.alloc_child_event_provenance(
                        ctx.provenance,
                        crate::events::EventKind::CardsDrawn,
                    );
                    let event = draw_observation_with_miracle(
                        game,
                        redirected_player,
                        drawn,
                        redirected_is_first,
                        (redirected_in_draw_step, redirected_previous),
                        redirected_provenance,
                        completed.miracle,
                    );
                    let drawn_count = event
                        .downcast::<CardsDrawnEvent>()
                        .map(CardsDrawnEvent::amount)
                        .unwrap_or(0);
                    game.record_cards_drawn_in_current_draw_step(redirected_player, drawn_count);
                    game.stage_turn_history_event(&event);
                    game.note_hidden_draw_for_reveal_window(&event);
                    original_draw_facts
                        .extend(crate::effects::outcome_recording::event_facts(game, &event));
                    let reveal = super::miracle_reveal_event(
                        game,
                        event.downcast::<CardsDrawnEvent>().expect("typed draw"),
                        ctx.provenance,
                    );
                    events.push(event);
                    events.extend(reveal);
                    events.extend(completed.automatic_reveals);
                    continue;
                }

                let completed = super::draw_cards_with_miracle_window(
                    game,
                    player_id,
                    final_count as usize,
                    is_first,
                    &mut *ctx.decision_maker,
                    ctx.provenance,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                let drawn = completed.cards;

                // Only emit event if cards were actually drawn
                if drawn.is_empty() {
                    continue;
                }
                let drawn_len = drawn.len() as i64;
                if direct_drawn.is_empty() {
                    direct_draw_is_first = is_first;
                    direct_miracle = completed.miracle;
                    direct_draw_step_context = (
                        is_during_players_draw_step,
                        cards_previously_drawn_this_draw_step,
                    );
                }
                direct_automatic_reveals.extend(completed.automatic_reveals);
                total_drawn += drawn_len;
                direct_drawn.extend(drawn);
            }
        }
    }

    let segment = finish_direct_draw_segment(
        game,
        ctx,
        player_id,
        &mut direct_drawn,
        direct_draw_is_first,
        direct_draw_step_context,
        &mut direct_miracle,
        &mut direct_automatic_reveals,
    );
    original_draw_facts.extend(
        segment
            .iter()
            .flat_map(|event| crate::effects::outcome_recording::event_facts(game, event)),
    );
    events.extend(segment);
    original_draw_facts.retain(|fact| {
        matches!(
            fact,
            crate::effect::ExecutionFact::ActionObjects {
                action: crate::effect::PriorEffectAction::Drawn,
                ..
            }
        )
    });
    if original_draw_facts.is_empty() {
        original_draw_facts.push(crate::effect::ExecutionFact::ActionObjects {
            action: crate::effect::PriorEffectAction::Drawn,
            player: Some(player_id),
            objects: Vec::new(),
        });
    }
    // Added programs remain observable but do not supply the original draw's
    // subjects, even when they happen to draw more cards themselves.
    replacement_facts.retain(|fact| {
        !matches!(
            fact,
            crate::effect::ExecutionFact::ActionObjects {
                action: crate::effect::PriorEffectAction::Drawn,
                ..
            }
        )
    });
    let primary = EffectOutcome::count(0).with_execution_facts(original_draw_facts.clone());
    let snapshots = crate::effects::outcome_recording::action_objects(
        &primary,
        crate::effect::PriorEffectAction::Drawn,
        None,
    )
    .unwrap_or_default();
    let original_count = crate::effects::outcome_recording::action_objects(
        &primary,
        crate::effect::PriorEffectAction::Drawn,
        Some(&[player_id]),
    )
    .unwrap_or_default()
    .len() as i64;
    let ids = snapshots
        .iter()
        .map(|snapshot| snapshot.object_id)
        .collect::<Vec<_>>();
    let original_events = events
        .iter()
        .filter(|event| {
            event
                .downcast::<CardsDrawnEvent>()
                .is_some_and(|draw| draw.cards.iter().all(|id| ids.contains(id)))
        })
        .cloned()
        .collect::<Vec<_>>();
    let original = EffectOutcome::count(original_count)
        .with_result_objects(ids)
        .with_affected_object_memory(snapshots.clone())
        .with_execution_fact(crate::effect::ExecutionFact::ResultObjectMemory(snapshots))
        .with_execution_facts(original_draw_facts.clone())
        .with_events(original_events);
    replacement_facts.extend(original_draw_facts);
    let mut observed = EffectOutcome::count(total_drawn + replacement_count)
        .with_events(events)
        .with_execution_facts(EffectOutcome::merge_execution_facts(replacement_facts));
    observed.instruction_result = Some(Box::new(original));
    Ok(observed)
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
        ).expect("finite replacement fixture evaluates successfully");
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

#[cfg(test)]
mod removed_draw_operation_tests {
    use super::*;
    fn check_removed_draw(turn_draw: bool) {
        struct PreferSubtractor(ObjectId);
        impl DecisionMaker for PreferSubtractor {
            fn decide_options(&mut self, _game: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                let option = ctx.options.iter().find(|option| option.legal && option.object_id == Some(self.0))
                    .or_else(|| ctx.options.iter().find(|option| option.legal)).unwrap();
                vec![option.index]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Draw replacement source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let subtractor = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let adder = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for _ in 0..3 { game.create_object_from_definition(&definition, alice, Zone::Library); }
        let effect = |source, modification| crate::replacement::ReplacementEffect::with_matcher(
            source, alice, crate::events::cards::matchers::WouldDrawCardMatcher::any_player(),
            crate::replacement::ReplacementAction::Modify(modification));
        let removed = game.effect_store.replacement_effects.add_one_shot_effect(effect(subtractor, crate::replacement::EventModification::Subtract(1)));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(effect(adder, crate::replacement::EventModification::Add(1)));
        let mut chooser = PreferSubtractor(subtractor);
        for positive in [false, true] {
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new(subtractor, alice, &mut chooser);
            let outcome = if turn_draw { execute_turn_draw_proposal(&mut game, &mut ctx, alice) }
                else { crate::effects::execute_effect(&mut game, &Effect::new(DrawCardsEffect::you(1)), &mut ctx) }.unwrap();
            let count = if positive { 2 } else { 0 };
            assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(count), "a removed draw cannot be revived by a later increase");
            assert_eq!(game.player(alice).unwrap().hand.len(), count as usize);
            assert_eq!(game.player(alice).unwrap().library.len(), 3 - count as usize);
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), count as u32);
            assert_eq!(outcome.events_of_type::<CardsDrawnEvent>().map(CardsDrawnEvent::amount).sum::<u32>(), count as u32);
            assert!(game.effect_store.replacement_effects.get_effect(removed).is_none());
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), !positive);
            if !positive { assert!(game.take_pending_trigger_events().is_empty()); }
        }
    }
    #[test]
    fn effect_draw_reduced_to_zero_preserves_later_one_shot_until_positive_draw() { check_removed_draw(false); }
    #[test]
    fn turn_draw_reduced_to_zero_preserves_later_one_shot_until_positive_draw() { check_removed_draw(true); }
    #[test]
    fn positive_draw_from_empty_library_still_applies_its_replacement() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Empty library replacement source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
            source, alice, crate::events::cards::matchers::WouldDrawCardWhileLibraryEmptyMatcher::you(),
            crate::replacement::ReplacementAction::Instead(vec![Effect::gain_life(3)])));
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = DrawCardsEffect::you(1).execute(&mut game, &mut ctx).unwrap();
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().life, 23);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(), 1);
        assert_eq!(outcome.events_of_type::<CardsDrawnEvent>().count(), 0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
}

#[cfg(test)]
mod removed_draw_selected_api_tests {
    use super::*;
    #[test]
    fn selected_zero_draw_preserves_one_shot_until_positive_proposal() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Selected draw source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::any_player(),
                crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Add(1))));
        for count in [0, 1] {
            let result = crate::events::processing::process_event_with_chosen_replacement_trait(
                &mut game, Event::draw(alice, count, true), shield).unwrap();
            let event = result.resolved_event().expect("selected draw API retains a resolved proposal");
            let draw = crate::events::downcast_event::<crate::events::DrawEvent>(event.inner()).unwrap();
            assert_eq!(draw.count, if count == 0 { 0 } else { 2 });
            assert_eq!(draw.player, alice);
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), count == 0);
            assert!(game.player(alice).unwrap().hand.is_empty(), "proposal APIs do not commit a draw");
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[cfg(test)]
mod checked_first_draw_producer_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::{CostExecutableEffect, CostValidationError, SequenceEffect, execute_effect};
    use crate::ids::CardId;
    use crate::types::CardType;

    fn fixture() -> (GameState, ObjectId, PlayerId, PlayerId) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::new(), "Draw receipt witness")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, a, Zone::Battlefield);
        for player in [a, b] {
            for _ in 0..8 { game.create_object_from_card(&card, player, Zone::Library); }
        }
        (game, source, a, b)
    }

    fn draw(game: &mut GameState, source: ObjectId, player: PlayerId, amount: i32) -> EffectOutcome {
        execute_effect(game, &Effect::draw(amount),
            &mut ExecutionContext::new_default(source, player)).unwrap()
    }

    #[test]
    fn real_draws_keep_first_nonfirst_receipts_across_batch_publication_and_native_recovery() {
        let (mut game, source, a, b) = fixture();
        let first = draw(&mut game, source, a, 2);
        let event = first.events_of_type::<CardsDrawnEvent>().next().unwrap();
        assert!(event.is_first_this_turn);
        assert_eq!(event.cards.len(), 2);
        assert!(game.turn_store.turn_history.has_drawn_cards_this_turn(a).unwrap());
        assert!(!game.turn_store.turn_history.has_drawn_cards_this_turn(b).unwrap());
        let checkpoint = game.clone();
        for event in first.events { game.queue_trigger_event(event.provenance(), event); }
        assert_eq!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().len(), 1);
        let second = draw(&mut game, source, a, 1);
        assert!(!second.events_of_type::<CardsDrawnEvent>().next().unwrap().is_first_this_turn);
        game.restore_execution_checkpoint(checkpoint, false);
        let replayed = draw(&mut game, source, a, 1);
        assert!(!replayed.events_of_type::<CardsDrawnEvent>().next().unwrap().is_first_this_turn);
        assert_eq!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().len(), 2);
        game.next_turn();
        let new_turn = draw(&mut game, source, a, 1);
        assert!(new_turn.events_of_type::<CardsDrawnEvent>().next().unwrap().is_first_this_turn);
    }

    #[test]
    fn redirected_draws_bind_first_status_to_actual_drawer_and_unpublished_local_segments() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        for prior in [false, true] {
            let (mut game, source, a, b) = fixture();
            if prior { draw(&mut game, source, b, 1); }
            let replacement_card = CardBuilder::new(CardId::new(), "Draw redirect source")
                .card_types(vec![CardType::Artifact]).build();
            let replacement_source = game.create_object_from_card(&replacement_card, b, Zone::Battlefield);
            game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
                replacement_source, b, crate::events::cards::matchers::WouldDrawCardMatcher::opponent(),
                ReplacementAction::RedirectDrawToController,
            ));
            let result = draw(&mut game, source, a, 2);
            let draws: Vec<_> = result.events_of_type::<CardsDrawnEvent>().collect();
            assert_eq!(draws.len(), 2);
            assert!(draws.iter().all(|draw| draw.player == b && draw.cards.len() == 1));
            assert_eq!(draws.iter().map(|draw| draw.is_first_this_turn).collect::<Vec<_>>(),
                vec![!prior, false]);
            assert!(game.player(a).unwrap().hand.is_empty());
            assert_eq!(game.player(b).unwrap().hand.len(), if prior { 3 } else { 2 });
            assert!(!game.turn_store.turn_history.has_drawn_cards_this_turn(a).unwrap());
            assert!(game.turn_store.turn_history.has_drawn_cards_this_turn(b).unwrap());
        }
    }

    #[test]
    fn held_expanded_originals_keep_first_status_for_actual_drawers() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        for simultaneous in [false, true] {
            for redirect in [false, true] {
                for prior in [false, true] {
                    let (mut game, source, a, b) = fixture();
                    let drawer = if redirect { b } else { a };
                    if prior { draw(&mut game, source, drawer, 1); }
                    let mut addition = ReplacementEffect::with_matcher(
                        source, a,
                        crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                        ReplacementAction::Additionally(vec![Effect::gain_life(1)]),
                    );
                    addition.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
                    game.effect_store.replacement_effects.add_resolution_effect(addition);
                    if redirect {
                        let redirect_source = game.create_object_from_card(
                            &CardBuilder::new(CardId::new(), "Held draw redirect")
                                .card_types(vec![CardType::Artifact]).build(), b, Zone::Battlefield);
                        game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
                            redirect_source, b, crate::events::cards::matchers::WouldDrawCardMatcher::opponent(),
                            ReplacementAction::RedirectDrawToController,
                        ));
                    }
                    if simultaneous { assert!(game.open_simultaneous_action()); }
                    else { game.effect_store.trigger_matching_holds = 1; }
                    let result = draw(&mut game, source, a, 2);
                    let draws: Vec<_> = result.events_of_type::<CardsDrawnEvent>().collect();
                    assert_eq!(draws.len(), 2);
                    assert!(draws.iter().all(|draw| draw.player == drawer && draw.cards.len() == 1));
                    assert_eq!(draws.iter().map(|draw| draw.is_first_this_turn).collect::<Vec<_>>(),
                        vec![!prior, false]);
                    assert_eq!(game.player(a).unwrap().life, 22, "both expanded originals execute their addition");
                    assert_eq!(game.player(drawer).unwrap().hand.len(), if prior { 3 } else { 2 });
                    assert_eq!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().len(),
                        if prior { 3 } else { 2 });
                }
            }
        }
    }

    #[test]
    fn held_added_draws_observe_original_occurrences_before_aggregate_publication() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        for simultaneous in [false, true] {
            let (mut game, source, a, _) = fixture();
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source, a, crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Additionally(vec![Effect::draw(1)]),
            ));
            if simultaneous { assert!(game.open_simultaneous_action()); }
            else { game.effect_store.trigger_matching_holds = 1; }
            let result = draw(&mut game, source, a, 2);
            let draws: Vec<_> = result.events_of_type::<CardsDrawnEvent>().collect();
            assert_eq!(draws.len(), 3);
            assert_eq!(draws.iter().map(|draw| draw.is_first_this_turn).collect::<Vec<_>>(),
                vec![true, false, false]);
            assert_eq!(result.instruction_result().count_or_zero(), 2);
            assert_eq!(game.player(a).unwrap().hand.len(), 3);
            let ordered_cards: Vec<_> = game.turn_store.turn_history.ordered_draw_occurrences().unwrap()
                .iter().flat_map(|record| record.event.downcast::<CardsDrawnEvent>().unwrap().cards.iter().copied())
                .collect();
            let observed_cards: Vec<_> = draws.iter().flat_map(|draw| draw.cards.iter().copied()).collect();
            assert_eq!(ordered_cards, observed_cards, "native chronology must retain original, addition, then later original");
            for event in result.events { game.queue_trigger_event(event.provenance(), event); }
            assert_eq!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().len(), 3,
                "aggregate publication enriches each occurrence once");
        }
    }

    #[test]
    fn held_mixed_direct_redirected_and_expanded_segments_retain_physical_order() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        #[derive(Debug, Clone)]
        struct LaterCard;
        impl crate::events::traits::ReplacementMatcher for LaterCard {
            fn matches_prepared_event(&self, event: &dyn crate::events::traits::GameEventType,
                _: &crate::events::context::PreparedEventContext) -> bool {
                crate::events::downcast_event::<crate::events::DrawEvent>(event)
                    .is_some_and(|draw| !draw.first_of_instruction)
            }
            fn display(&self) -> String { "Later card of this draw instruction".into() }
        }
        for simultaneous in [false, true] {
            for redirected_then_expanded in [false, true] {
                let (mut game, source, a, b) = fixture();
                let redirect_source = game.create_object_from_card(
                    &CardBuilder::new(CardId::new(), "Mixed draw redirect")
                        .card_types(vec![CardType::Artifact]).build(), b, Zone::Battlefield);
                let redirect = if redirected_then_expanded {
                    ReplacementEffect::with_matcher(redirect_source, b,
                        crate::events::cards::matchers::WouldDrawCardMatcher::opponent(),
                        ReplacementAction::RedirectDrawToController)
                } else {
                    ReplacementEffect::with_matcher(redirect_source, b, LaterCard,
                        ReplacementAction::RedirectDrawToController)
                };
                game.effect_store.replacement_effects.add_resolution_effect(redirect);
                if redirected_then_expanded {
                    let mut addition = ReplacementEffect::with_matcher(source, a, LaterCard,
                        ReplacementAction::Additionally(vec![Effect::gain_life(1)]));
                    addition.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
                    game.effect_store.replacement_effects.add_resolution_effect(addition);
                }
                if simultaneous { assert!(game.open_simultaneous_action()); }
                else { game.effect_store.trigger_matching_holds = 1; }
                let result = draw(&mut game, source, a, 2);
                let draws: Vec<_> = result.events_of_type::<CardsDrawnEvent>().collect();
                assert_eq!(draws.len(), 2);
                assert_eq!(draws.iter().map(|draw| draw.player).collect::<Vec<_>>(),
                    if redirected_then_expanded { vec![b, b] } else { vec![a, b] });
                assert_eq!(draws.iter().map(|draw| draw.is_first_this_turn).collect::<Vec<_>>(),
                    vec![true, !redirected_then_expanded]);
                let observed: Vec<_> = draws.iter().flat_map(|draw| draw.cards.iter().copied()).collect();
                let retained: Vec<_> = game.turn_store.turn_history.ordered_draw_occurrences().unwrap()
                    .iter().flat_map(|record| record.event.downcast::<CardsDrawnEvent>().unwrap().cards.iter().copied())
                    .collect();
                assert_eq!(retained, observed);
                assert_eq!(game.player(a).unwrap().life, if redirected_then_expanded { 21 } else { 20 });
                for event in result.events { game.queue_trigger_event(event.provenance(), event); }
                assert_eq!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().len(), 2);
            }
        }
    }

    #[test]
    fn held_added_program_error_or_pending_restores_staged_original_draw() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        struct Pause { pending: bool }
        impl DecisionMaker for Pause {
            fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
                self.pending = true;
                false
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        for simultaneous in [false, true] {
            for pending in [false, true] {
                let (mut game, source, a, _) = fixture();
                let payload = if pending { vec![Effect::may(vec![Effect::gain_life(2)])] }
                    else { vec![Effect::gain_life(2), Effect::lose_life(crate::effect::Value::X)] };
                let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                    source, a, crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                    ReplacementAction::Additionally(payload),
                ));
                if simultaneous { assert!(game.open_simultaneous_action()); }
                else { game.effect_store.trigger_matching_holds = 1; }
                let library = game.player(a).unwrap().library.to_vec();
                let mut pause = Pause { pending: false };
                let result = execute_effect(&mut game, &Effect::draw(2),
                    &mut ExecutionContext::new(source, a, &mut pause));
                if pending {
                    assert!(pause.pending);
                    assert_eq!(result.unwrap().events_of_type::<CardsDrawnEvent>().count(), 0);
                } else {
                    assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
                }
                assert_eq!(game.player(a).unwrap().library.as_slice(), library.as_slice());
                assert!(game.player(a).unwrap().hand.is_empty());
                assert_eq!(game.player(a).unwrap().life, 20);
                assert!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().is_empty());
                assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            }
        }
    }

    #[test]
    fn unknown_history_stops_real_draws_and_cost_queries_without_publishing_partial_work() {
        let (mut game, source, a, _) = fixture();
        game.turn_store.turn_history.draw_occurrences = None;
        let library = game.player(a).unwrap().library.to_vec();
        let records = game.turn_store.turn_history.event_records.len();
        assert!(matches!(crate::effects::CostExecutableEffect::can_execute_as_cost(&DrawCardsEffect::you(1), &game, source, a),
            Err(CostValidationError::ExecutionFailed(ExecutionError::IncompleteEvidence(_)))));
        let sequence = Effect::new(SequenceEffect::new(vec![Effect::gain_life(5), Effect::draw(1)]));
        let result = execute_effect(&mut game, &sequence, &mut ExecutionContext::new_default(source, a));
        assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))));
        assert_eq!(game.player(a).unwrap().life, 20);
        assert_eq!(game.player(a).unwrap().library.as_slice(), library.as_slice());
        assert!(game.player(a).unwrap().hand.is_empty());
        assert_eq!(game.turn_store.turn_history.event_records.len(), records);
        assert!(game.turn_store.turn_history.draw_occurrences.is_none());
        crate::effects::CostExecutableEffect::can_execute_as_cost(&DrawCardsEffect::you(0), &game, source, a).unwrap();
        let zero = draw(&mut game, source, a, 0);
        assert_eq!(zero.events_of_type::<CardsDrawnEvent>().count(), 0);
        assert!(game.turn_store.turn_history.draw_occurrences.is_none(), "a no-op cannot invent history");
        game.next_turn();
        let actual = draw(&mut game, source, a, 1);
        assert!(actual.events_of_type::<CardsDrawnEvent>().next().unwrap().is_first_this_turn);
    }

    #[test]
    fn pending_physical_draw_does_not_consume_first_draw_before_retry() {
        struct Pause { pending: bool }
        impl DecisionMaker for Pause {
            fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
                self.pending = true;
                false
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        struct Decline;
        impl DecisionMaker for Decline {
            fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { false }
        }
        let (mut game, source, a, _) = fixture();
        let commander = CardBuilder::new(CardId::new(), "Pending commander draw")
            .card_types(vec![CardType::Creature]).build();
        let commander = game.create_object_from_card(&commander, a, Zone::Library);
        game.set_as_commander(commander, a);
        let library = game.player(a).unwrap().library.to_vec();
        let mut pause = Pause { pending: false };
        let outcome = execute_effect(&mut game, &Effect::draw(2),
            &mut ExecutionContext::new(source, a, &mut pause)).unwrap();
        assert!(pause.pending);
        assert_eq!(outcome.events_of_type::<CardsDrawnEvent>().count(), 0);
        assert_eq!(game.player(a).unwrap().library.as_slice(), library.as_slice());
        assert!(!game.turn_store.turn_history.has_drawn_cards_this_turn(a).unwrap());
        let complete = execute_effect(&mut game, &Effect::draw(2),
            &mut ExecutionContext::new(source, a, &mut Decline)).unwrap();
        let event = complete.events_of_type::<CardsDrawnEvent>().next().unwrap();
        assert!(event.is_first_this_turn);
        assert_eq!(event.cards.len(), 2);
        assert_eq!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().len(), 1);
    }
}
