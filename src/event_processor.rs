//! Event processor for replacement and prevention effects.
//!
//! This module handles the processing of game events through replacement effects
//! per MTG Rules 614-616. When an event is about to happen, it's passed through
//! this processor which:
//! 1. Finds applicable replacement effects
//! 2. Sorts them by Rule 616.1 priority
//! 3. Applies one effect at a time (each effect can only apply once per Rule 614.5)
//! 4. Loops until no more replacement effects apply
//!
//! This enables proper handling of complex interactions like:
//! - "If you would gain life, you gain that much life plus 1 instead"
//! - "If a creature you control would die, exile it instead"
//! - "Damage can't be prevented"

use crate::DecisionMaker;
use crate::events::{Event, EventContext, EventKind};
use crate::game_event::DamageTarget;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::object::CounterType;
use crate::replacement::{
    EventModification, ReplacementAction, ReplacementEffect, ReplacementEffectId,
};
use crate::types::CardType;
use crate::zone::Zone;

/// Priority order for replacement effects per Rule 616.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplacementPriority {
    /// 616.1a: Self-replacement effects (affect only their source)
    SelfReplacement = 0,
    /// 616.1b: Control-changing effects
    ControlChanging = 1,
    /// 616.1c: Copy effects
    CopyEffect = 2,
    /// 616.1d: Effects that cause permanents to enter as back face (MDFCs)
    BackFace = 3,
    /// 616.1e: All other replacement effects (affected player/controller chooses)
    Other = 4,
}

/// Process an event through the replacement effect system.
///
/// This is the main entry point for event processing. It finds and applies
/// applicable replacement effects using trait-based matchers.
pub fn process_trait_event(game: &GameState, event: Event) -> TraitEventResult {
    let mut state = TraitEventProcessingState::default();
    process_event_direct(game, event, &mut state, &[])
}

/// Process an event through the replacement effect system with additional self-replacement effects.
///
/// This variant allows passing in self-replacement effects from an object's abilities,
/// which is needed for effects like shock lands' "pay 2 life or enter tapped" that apply
/// to the object while it's still in the source zone (before it enters the battlefield).
pub fn process_trait_event_with_self_effects(
    game: &GameState,
    event: Event,
    self_effects: &[ReplacementEffect],
) -> TraitEventResult {
    let mut state = TraitEventProcessingState::default();
    process_event_direct(game, event, &mut state, self_effects)
}

/// State for tracking trait-based event processing.
#[derive(Debug, Clone, Default)]
pub struct TraitEventProcessingState {
    /// Replacement effects that have already been applied to this event.
    pub applied_effects: std::collections::HashSet<ReplacementEffectId>,
    /// Iteration count to detect infinite loops.
    pub iteration_count: u32,
}

impl TraitEventProcessingState {
    /// Maximum iterations before we assume infinite loop.
    pub const MAX_ITERATIONS: u32 = 100;

    /// Check if we've exceeded the maximum iteration count.
    pub fn exceeded_max_iterations(&self) -> bool {
        self.iteration_count >= Self::MAX_ITERATIONS
    }

    /// Mark an effect as applied.
    pub fn mark_applied(&mut self, id: ReplacementEffectId) {
        self.applied_effects.insert(id);
    }

    /// Check if an effect was already applied.
    pub fn was_applied(&self, id: ReplacementEffectId) -> bool {
        self.applied_effects.contains(&id)
    }

    /// Increment iteration count.
    pub fn increment(&mut self) {
        self.iteration_count += 1;
    }
}

/// Process an event directly using trait-based matchers.
fn process_event_direct(
    game: &GameState,
    event: Event,
    state: &mut TraitEventProcessingState,
    self_effects: &[ReplacementEffect],
) -> TraitEventResult {
    // Safety check for infinite loops
    if state.exceeded_max_iterations() {
        return TraitEventResult::Proceed(event);
    }
    state.increment();

    // Find all applicable replacement effects using trait-based matchers
    let applicable = find_applicable_trait_replacements(game, &event, state, self_effects);

    if applicable.is_empty() {
        return TraitEventResult::Proceed(event);
    }

    // Sort by Rule 616.1 priority
    let mut sorted = applicable;
    sorted.sort_by_key(|(_, priority)| *priority);

    let highest_priority = sorted[0].1;

    // Filter to effects at highest priority
    let at_highest: Vec<_> = sorted
        .into_iter()
        .filter(|(_, p)| *p == highest_priority)
        .map(|(effect, _)| effect)
        .collect();

    // Per Rule 616.1e: If multiple effects at "Other" priority, player chooses
    if at_highest.len() > 1 && highest_priority == ReplacementPriority::Other {
        let affected_player = event.0.affected_player(game);
        let effect_ids: Vec<_> = at_highest.iter().map(|e| e.id).collect();

        return TraitEventResult::NeedsChoice {
            player: affected_player,
            applicable_effects: effect_ids,
            event: Box::new(event),
        };
    }

    // Apply the chosen effect
    let chosen_effect = at_highest[0].clone();
    let effect_id = chosen_effect.id;

    // Extract life_cost before apply_trait_replacement consumes the effect
    let life_cost = if let ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } =
        &chosen_effect.replacement
    {
        Some(*life_cost)
    } else {
        None
    };

    let result = apply_trait_replacement(game, event.clone(), &chosen_effect);
    state.mark_applied(effect_id);

    match result {
        TraitApplyResult::Modified(modified_event) => {
            process_event_direct(game, modified_event, state, self_effects)
        }
        TraitApplyResult::Prevented => TraitEventResult::Prevented,
        TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced { effects, effect_id },
        TraitApplyResult::Unchanged(event) => TraitEventResult::Proceed(event),
        TraitApplyResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            filter,
        } => TraitEventResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            event: Box::new(event),
            filter,
            life_cost,
        },
    }
}

// =============================================================================
// Interactive Replacement Effect Handling
// =============================================================================

/// Result of continuing an interactive replacement effect after player decision.
#[derive(Debug, Clone)]
pub struct InteractiveReplacementResult {
    /// Whether the permanent enters the battlefield (true) or is redirected (false).
    pub enters: bool,
    /// If entering, whether it enters tapped (for shock lands).
    pub enters_tapped: bool,
    /// If not entering, the zone it goes to instead.
    pub redirect_zone: Option<Zone>,
}

impl InteractiveReplacementResult {
    /// Create a result indicating the permanent enters the battlefield.
    pub fn enters_battlefield() -> Self {
        Self {
            enters: true,
            enters_tapped: false,
            redirect_zone: None,
        }
    }

    /// Create a result indicating the permanent enters tapped.
    pub fn enters_tapped() -> Self {
        Self {
            enters: true,
            enters_tapped: true,
            redirect_zone: None,
        }
    }

    /// Create a result indicating the permanent is redirected to another zone.
    pub fn redirected(zone: Zone) -> Self {
        Self {
            enters: false,
            enters_tapped: false,
            redirect_zone: Some(zone),
        }
    }
}

/// Continue an interactive replacement effect after the player has made a decision.
///
/// This is called after the player responds to a `NeedsInteraction` result.
///
/// # Arguments
/// * `game` - The game state (may be modified for discards/life payment)
/// * `response` - The player's response to the decision
/// * `object_id` - The object being affected (the permanent entering)
/// * `controller` - The controller of the permanent
/// * `filter` - The filter for discard (Some for InteractiveDiscardOrRedirect)
/// * `redirect_zone` - Where to redirect if the player declines
/// * `life_cost` - The life cost (Some for InteractivePayLifeOrEnterTapped)
/// * `decision_maker` - Optional decision maker for follow-up decisions (e.g., Library of Leng)
///
/// # Returns
/// An `InteractiveReplacementResult` indicating whether the permanent enters,
/// enters tapped, or is redirected.
#[derive(Debug, Clone, PartialEq, Eq)]
enum InteractiveReplacementResponse {
    Accept,
    Decline,
    Objects(Vec<crate::ids::ObjectId>),
}

fn continue_interactive_replacement(
    game: &mut GameState,
    response: &InteractiveReplacementResponse,
    object_id: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    filter: Option<&crate::target::ObjectFilter>,
    redirect_zone: Zone,
    life_cost: Option<u32>,
    decision_maker: &mut impl DecisionMaker,
) -> InteractiveReplacementResult {
    // Handle discard-or-redirect (Mox Diamond pattern)
    if let Some(filter) = filter {
        return handle_discard_or_redirect(
            game,
            response,
            object_id,
            controller,
            filter,
            redirect_zone,
            decision_maker,
        );
    }

    // Handle pay-life-or-enter-tapped (shock land pattern)
    if let Some(cost) = life_cost {
        return handle_pay_life_or_enter_tapped(game, response, controller, cost);
    }

    // Fallback: redirect
    InteractiveReplacementResult::redirected(redirect_zone)
}

/// Handle a discard-or-redirect interactive replacement.
fn handle_discard_or_redirect(
    game: &mut GameState,
    response: &InteractiveReplacementResponse,
    _object_id: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    filter: &crate::target::ObjectFilter,
    redirect_zone: Zone,
    decision_maker: &mut impl DecisionMaker,
) -> InteractiveReplacementResult {
    match response {
        InteractiveReplacementResponse::Objects(cards) => {
            // Handle new context-based discard response (vector of cards)
            // For interactive replacement, we expect exactly 1 card
            if let Some(&card_id) = cards.first() {
                let matching_cards = find_matching_cards_in_hand(game, controller, filter);
                if matching_cards.contains(&card_id) {
                    let result = execute_discard(
                        game,
                        card_id,
                        controller,
                        crate::events::cause::EventCause::default(),
                        true,
                        decision_maker,
                    );
                    if result.type_verifiable {
                        InteractiveReplacementResult::enters_battlefield()
                    } else {
                        InteractiveReplacementResult::redirected(redirect_zone)
                    }
                } else {
                    InteractiveReplacementResult::redirected(redirect_zone)
                }
            } else {
                // No card selected, redirect
                InteractiveReplacementResult::redirected(redirect_zone)
            }
        }
        InteractiveReplacementResponse::Decline | InteractiveReplacementResponse::Accept => {
            // Player chose not to discard, redirect
            InteractiveReplacementResult::redirected(redirect_zone)
        }
    }
}

/// Handle a pay-life-or-enter-tapped interactive replacement.
fn handle_pay_life_or_enter_tapped(
    game: &mut GameState,
    response: &InteractiveReplacementResponse,
    controller: crate::ids::PlayerId,
    life_cost: u32,
) -> InteractiveReplacementResult {
    match response {
        InteractiveReplacementResponse::Accept => {
            // Player chose to pay life
            // Verify they can still pay
            let can_pay = game
                .player(controller)
                .map(|p| p.life >= life_cost as i32)
                .unwrap_or(false);

            if can_pay {
                // Deduct life
                if let Some(player) = game.player_mut(controller) {
                    player.life -= life_cost as i32;
                }
                // Permanent enters untapped
                InteractiveReplacementResult::enters_battlefield()
            } else {
                // Can't pay anymore (life changed since decision was made)
                // Permanent enters tapped
                InteractiveReplacementResult::enters_tapped()
            }
        }
        InteractiveReplacementResponse::Decline | InteractiveReplacementResponse::Objects(_) => {
            // Player chose not to pay life - permanent enters tapped
            InteractiveReplacementResult::enters_tapped()
        }
    }
}

// =============================================================================
// Unified Discard Processing
// =============================================================================

/// Result of executing a discard with potential replacement effects.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardResult {
    /// The ID of the card after it moved zones (may be different from original).
    pub new_id: Option<crate::ids::ObjectId>,
    /// The zone the card ended up in.
    pub final_zone: Zone,
    /// Whether the card's type can be verified in its final zone.
    /// - Graveyard: true (public zone, card is revealed)
    /// - Library: false (hidden zone, type undefined per rule 701.8c)
    /// - Exile: depends on whether card is face-up
    pub type_verifiable: bool,
    /// Whether the discard was prevented entirely.
    pub prevented: bool,
}

impl DiscardResult {
    /// Returns true if the card went to the graveyard (the default discard destination).
    pub fn went_to_graveyard(&self) -> bool {
        self.final_zone == Zone::Graveyard
    }

    /// Create a result indicating the card was discarded to graveyard (default).
    pub fn to_graveyard(new_id: Option<crate::ids::ObjectId>) -> Self {
        Self {
            new_id,
            final_zone: Zone::Graveyard,
            type_verifiable: true,
            prevented: false,
        }
    }

    /// Create a result indicating the card went to library (Library of Leng).
    pub fn to_library(new_id: Option<crate::ids::ObjectId>) -> Self {
        Self {
            new_id,
            final_zone: Zone::Library,
            type_verifiable: false, // Hidden zone
            prevented: false,
        }
    }

    /// Create a result indicating the discard was prevented.
    pub fn prevented() -> Self {
        Self {
            new_id: None,
            final_zone: Zone::Hand, // Card stayed in hand
            type_verifiable: true,
            prevented: true,
        }
    }
}

/// Check if a zone allows card type verification after a discard.
///
/// Per MTG rule 701.8c: "If a card is discarded, but an effect causes it to be
/// put into a hidden zone instead of into its owner's graveyard without being
/// revealed, all values of that card's characteristics are considered to be
/// undefined."
pub fn zone_allows_type_verification(zone: Zone) -> bool {
    match zone {
        // Public zones - cards are visible, characteristics can be verified
        Zone::Graveyard | Zone::Battlefield | Zone::Stack | Zone::Command => true,
        // Hidden zones - characteristics become undefined per rule 701.8c
        Zone::Library | Zone::Hand => false,
        // Exile is special - face-up cards can be verified, face-down cannot
        // For simplicity, we treat exile as verifiable since face-down exile
        // typically happens through specific effects, not discard replacement
        Zone::Exile => true,
    }
}

/// Execute a discard using the generic trait-based replacement effect system.
///
/// This is the unified entry point for all discard operations. It:
/// 1. Creates a DiscardEvent with the appropriate cause
/// 2. Processes it through the trait-based replacement effect system
/// 3. Handles interactive replacements (like Library of Leng) via the decision maker
/// 4. Moves the card to the final destination
///
/// The `EventCause` determines which replacement effects apply:
/// - `EventCause::from_effect(...)` - Library of Leng applies
/// - `EventCause::from_game_rule()` - Library of Leng applies (cleanup discard)
/// - `EventCause::from_cost(...)` - Library of Leng does NOT apply
///
/// # Arguments
/// * `game` - The game state
/// * `card_id` - The card being discarded
/// * `player` - The player discarding
/// * `cause` - What caused this discard (effect, cost, game rule)
/// * `_requires_type_verification` - Unused, type_verifiable is always computed from zone
/// * `decision_maker` - Optional decision maker for player choices
///
/// # Returns
/// A `DiscardResult` with information about where the card went.
pub fn execute_discard(
    game: &mut GameState,
    card_id: crate::ids::ObjectId,
    player: crate::ids::PlayerId,
    cause: crate::events::cause::EventCause,
    _requires_type_verification: bool,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
) -> DiscardResult {
    use crate::events::cards::DiscardEvent;
    use crate::events::traits::downcast_event;

    game.update_replacement_effects();

    // Create a discard event with the cause
    let discard_event = DiscardEvent::with_cause(card_id, player, cause);
    let event = Event::new(discard_event);

    // Process through the trait-based replacement effect system
    let result = process_with_dm(game, event, decision_maker);

    match result {
        TraitEventResult::Proceed(final_event) | TraitEventResult::Modified(final_event) => {
            // Extract the final destination from the (possibly modified) event
            if let Some(discard) = downcast_event::<DiscardEvent>(final_event.inner()) {
                let mut destination = discard.destination;

                // Check for Madness: if card has Madness and destination is Graveyard,
                // replace destination with Exile (Madness replacement effect)
                let has_madness = game
                    .object(card_id)
                    .map(|obj| obj.alternative_casts.iter().any(|alt| alt.is_madness()))
                    .unwrap_or(false);

                if has_madness && destination == Zone::Graveyard {
                    destination = Zone::Exile;
                }

                let new_id = if destination == Zone::Library {
                    move_to_top_of_library(game, card_id, player)
                } else {
                    game.move_object(card_id, destination)
                };

                // Mark as madness_exiled if card went to exile via Madness
                if has_madness
                    && destination == Zone::Exile
                    && let Some(id) = new_id
                {
                    game.set_madness_exiled(id);
                }

                DiscardResult {
                    new_id,
                    final_zone: destination,
                    type_verifiable: zone_allows_type_verification(destination),
                    prevented: false,
                }
            } else {
                debug_assert!(
                    false,
                    "discard replacement processing returned a non-DiscardEvent"
                );
                DiscardResult::prevented()
            }
        }

        TraitEventResult::Prevented => DiscardResult::prevented(),

        TraitEventResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            event: _original_event,
            ..
        } => {
            // Interactive replacement effect (like Library of Leng)
            // Use the decision maker to resolve the choice
            match decision_ctx {
                crate::decisions::context::DecisionContext::SelectOptions(ctx) => {
                    // Get the player's choice
                    let selected = decision_maker.decide_options(game, &ctx);

                    // Map the selection back to a zone
                    // The options are indexed by position in the destinations list
                    let chosen_zone = if let Some(&idx) = selected.first() {
                        // Extract destinations from the original event context
                        // For Library of Leng: [Graveyard, Library]
                        match idx {
                            0 => Zone::Graveyard,
                            1 => Zone::Library,
                            _ => redirect_zone,
                        }
                    } else {
                        redirect_zone
                    };

                    let new_id = if chosen_zone == Zone::Library {
                        move_to_top_of_library(game, card_id, player)
                    } else {
                        game.move_object(card_id, chosen_zone)
                    };

                    DiscardResult {
                        new_id,
                        final_zone: chosen_zone,
                        type_verifiable: zone_allows_type_verification(chosen_zone),
                        prevented: false,
                    }
                }
                _ => {
                    // Unexpected context type, use default
                    let new_id = game.move_object(card_id, redirect_zone);
                    DiscardResult {
                        new_id,
                        final_zone: redirect_zone,
                        type_verifiable: zone_allows_type_verification(redirect_zone),
                        prevented: false,
                    }
                }
            }
        }

        TraitEventResult::Replaced { .. } => {
            // Discard replaced with other effects - treat as prevented
            DiscardResult::prevented()
        }

        TraitEventResult::NeedsChoice { .. } => DiscardResult::prevented(),
    }
}

/// Move a card to the top of the owner's library.
fn move_to_top_of_library(
    game: &mut GameState,
    card_id: crate::ids::ObjectId,
    owner: crate::ids::PlayerId,
) -> Option<crate::ids::ObjectId> {
    // Get the new ID from the zone change
    let new_id = game.move_object(card_id, Zone::Library)?;

    // The card should now be at the end of the library array (which represents the top)
    // move_object already handles this correctly for Zone::Library

    // Ensure the card is at the top (end of the Vec for library)
    if let Some(player) = game.player_mut(owner) {
        // Remove from current position if not already at top
        if let Some(pos) = player.library.iter().position(|&id| id == new_id)
            && pos != player.library.len() - 1
        {
            player.library.remove(pos);
            player.library.push(new_id);
        }
    }

    Some(new_id)
}

/// Result of applying a single replacement effect to a trait-based event.
enum TraitApplyResult {
    /// Event was modified, continue processing
    Modified(Event),
    /// Event was prevented
    Prevented,
    /// Event was replaced with other effects
    Replaced(Vec<crate::effect::Effect>),
    /// Effect didn't change anything
    Unchanged(Event),
    /// Effect requires player interaction before proceeding.
    ///
    /// The caller must:
    /// 1. Present the decision to the player
    /// 2. Call `continue_interactive_replacement()` with the response
    /// 3. Use the result to determine if the event proceeds
    NeedsInteraction {
        /// The decision context that needs to be resolved by the player.
        decision_ctx: crate::decisions::context::DecisionContext,
        /// The zone to redirect to if the player declines or can't pay.
        redirect_zone: Zone,
        /// The ID of the replacement effect, for tracking.
        effect_id: ReplacementEffectId,
        /// The object being affected (for tracking).
        object_id: crate::ids::ObjectId,
        /// The filter for discarding (for InteractiveDiscardOrRedirect).
        filter: Option<crate::target::ObjectFilter>,
    },
}

/// Find all replacement effects that apply to a trait-based event.
fn find_applicable_trait_replacements(
    game: &GameState,
    event: &Event,
    state: &TraitEventProcessingState,
    self_effects: &[ReplacementEffect],
) -> Vec<(ReplacementEffect, ReplacementPriority)> {
    let mut applicable = Vec::new();

    // Check registered replacement effects in the game
    for effect in game.replacement_effects.effects() {
        // Skip if already applied (Rule 614.5)
        if state.was_applied(effect.id) {
            continue;
        }

        // Check if effect matches using trait-based matcher
        if let Some(priority) = trait_effect_matches_event(game, effect, event) {
            applicable.push((effect.clone(), priority));
        }
    }

    // Check self-replacement effects (from the object's own abilities)
    for effect in self_effects {
        // Skip if already applied (Rule 614.5)
        if state.was_applied(effect.id) {
            continue;
        }

        // Check if effect matches
        if let Some(priority) = trait_effect_matches_event(game, effect, event) {
            applicable.push((effect.clone(), priority));
        }
    }

    applicable
}

/// Check if a replacement effect matches an event using trait-based matching.
fn trait_effect_matches_event(
    game: &GameState,
    effect: &ReplacementEffect,
    event: &Event,
) -> Option<ReplacementPriority> {
    use crate::events::ReplacementPriority as TraitPriority;

    // All effects should have trait-based matchers
    let matcher = effect.matcher.as_ref()?;

    let ctx = EventContext::for_replacement_effect(effect.controller, effect.source, game);
    if !matcher.matches_event(event.inner(), &ctx) {
        return None;
    }

    let priority = if effect.self_replacement {
        ReplacementPriority::SelfReplacement
    } else {
        match matcher.priority() {
            TraitPriority::SelfReplacement => ReplacementPriority::SelfReplacement,
            TraitPriority::ControlChanging => ReplacementPriority::ControlChanging,
            TraitPriority::CopyEffect => ReplacementPriority::CopyEffect,
            TraitPriority::BackFace => ReplacementPriority::BackFace,
            TraitPriority::Other => ReplacementPriority::Other,
        }
    };

    Some(priority)
}

/// Apply a replacement effect to a trait-based event.
fn apply_trait_replacement(
    game: &GameState,
    event: Event,
    effect: &ReplacementEffect,
) -> TraitApplyResult {
    match &effect.replacement {
        ReplacementAction::Prevent => TraitApplyResult::Prevented,

        ReplacementAction::Skip => TraitApplyResult::Prevented,

        ReplacementAction::Instead(effects) => TraitApplyResult::Replaced(effects.clone()),

        ReplacementAction::Modify(modification) => {
            // Apply modification based on event type
            let modified = apply_trait_modification(&event, modification);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::Double => {
            let modified = apply_trait_double(&event);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::ChangeDestination(new_zone) => {
            let modified = apply_trait_change_destination(&event, *new_zone);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::EnterTapped => {
            let modified = apply_trait_enter_tapped(&event);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::EnterWithCounters {
            counter_type,
            count,
        } => {
            let resolved_count = resolve_value_for_etb(count, game, effect.source);
            let modified = apply_trait_enter_with_counters(&event, *counter_type, resolved_count);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::Redirect { target, which } => {
            let modified =
                apply_trait_redirect(&event, target, which, effect.controller, effect.source);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::Additionally(_effects) => {
            // For "additionally" effects, proceed with original (additional effects handled separately)
            TraitApplyResult::Modified(event)
        }

        ReplacementAction::EnterAsCopy(_source_id) => {
            let modified = apply_trait_enter_as_copy(&event, *_source_id);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::InteractiveDiscardOrRedirect {
            filter,
            redirect_zone,
        } => {
            // Check if controller has any cards in hand matching the filter
            let controller = effect.controller;
            let matching_cards = find_matching_cards_in_hand(game, controller, filter);

            if matching_cards.is_empty() {
                // No matching cards - automatically redirect
                // Modify the event to change destination
                let modified = apply_trait_change_destination(&event, *redirect_zone);
                match modified {
                    Some(e) => TraitApplyResult::Modified(e),
                    None => TraitApplyResult::Unchanged(event),
                }
            } else {
                // Matching cards exist - need player decision
                // Build candidates for the select objects context
                let candidates: Vec<crate::decisions::context::SelectableObject> = matching_cards
                    .iter()
                    .map(|&id| {
                        let name = game
                            .object(id)
                            .map(|o| o.name.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        crate::decisions::context::SelectableObject::new(id, name)
                    })
                    .collect();
                let source_name = game
                    .object(effect.source)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "permanent".to_string());
                let decision_ctx = crate::decisions::context::DecisionContext::SelectObjects(
                    crate::decisions::context::SelectObjectsContext::new(
                        controller,
                        Some(effect.source),
                        format!(
                            "Discard a card to put {} onto the battlefield, or it goes to {:?}",
                            source_name, redirect_zone
                        ),
                        candidates,
                        1,       // min
                        Some(1), // max
                    ),
                );
                TraitApplyResult::NeedsInteraction {
                    decision_ctx,
                    redirect_zone: *redirect_zone,
                    effect_id: effect.id,
                    object_id: effect.source,
                    filter: Some(filter.clone()),
                }
            }
        }

        ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } => {
            // Check if controller can pay the life cost
            let controller = effect.controller;
            let can_pay = game
                .player(controller)
                .map(|p| p.life >= *life_cost as i32)
                .unwrap_or(false);

            if !can_pay {
                // Can't pay - automatically enter tapped
                let modified = apply_trait_enter_tapped(&event);
                match modified {
                    Some(e) => TraitApplyResult::Modified(e),
                    None => TraitApplyResult::Unchanged(event),
                }
            } else {
                // Can pay - need player decision
                let source_name = game.object(effect.source).map(|o| o.name.clone());
                let mut bool_ctx = crate::decisions::context::BooleanContext::new(
                    controller,
                    Some(effect.source),
                    format!("Pay {} life? (If you don't, this enters tapped)", life_cost),
                );
                if let Some(name) = source_name {
                    bool_ctx = bool_ctx.with_source_name(name);
                }
                let decision_ctx = crate::decisions::context::DecisionContext::Boolean(bool_ctx);
                TraitApplyResult::NeedsInteraction {
                    decision_ctx,
                    redirect_zone: Zone::Battlefield, // Not a zone redirect, but we need a placeholder
                    effect_id: effect.id,
                    object_id: effect.source,
                    filter: None,
                }
            }
        }

        ReplacementAction::InteractiveChooseDestination {
            destinations,
            description,
        } => {
            // Interactive choice of destination (e.g., Library of Leng)
            // Build options from the destination list
            let options: Vec<crate::decisions::context::SelectableOption> = destinations
                .iter()
                .enumerate()
                .map(|(idx, zone)| {
                    let zone_name = match zone {
                        Zone::Library => "Top of library",
                        Zone::Graveyard => "Graveyard",
                        Zone::Hand => "Hand",
                        Zone::Exile => "Exile",
                        Zone::Battlefield => "Battlefield",
                        Zone::Stack => "Stack",
                        Zone::Command => "Command zone",
                    };
                    crate::decisions::context::SelectableOption::new(idx, zone_name.to_string())
                })
                .collect();

            let controller = effect.controller;
            let decision_ctx = crate::decisions::context::DecisionContext::SelectOptions(
                crate::decisions::context::SelectOptionsContext::new(
                    controller,
                    Some(effect.source),
                    description.clone(),
                    options,
                    1, // min
                    1, // max
                ),
            );

            // Default to the first destination if player doesn't choose
            let default_zone = destinations.first().copied().unwrap_or(Zone::Graveyard);

            TraitApplyResult::NeedsInteraction {
                decision_ctx,
                redirect_zone: default_zone,
                effect_id: effect.id,
                object_id: effect.source,
                filter: None,
            }
        }
    }
}

/// Find cards in hand matching the filter.
fn find_matching_cards_in_hand(
    game: &GameState,
    controller: crate::ids::PlayerId,
    filter: &crate::target::ObjectFilter,
) -> Vec<crate::ids::ObjectId> {
    use crate::target::FilterContext;

    let filter_ctx = FilterContext::new(controller);
    game.player(controller)
        .map(|p| {
            p.hand
                .iter()
                .filter(|&&card_id| {
                    game.object(card_id)
                        .map(|obj| filter.matches(obj, &filter_ctx, game))
                        .unwrap_or(false)
                })
                .copied()
                .collect()
        })
        .unwrap_or_default()
}

/// Apply an EventModification to a trait-based event.
fn apply_trait_modification(event: &Event, modification: &EventModification) -> Option<Event> {
    use crate::events::{DamageEvent, DrawEvent, LifeGainEvent, PutCountersEvent, downcast_event};

    match event.kind() {
        EventKind::Damage => {
            let damage = downcast_event::<DamageEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    damage.with_amount(damage.amount.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    damage.with_amount((damage.amount as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => damage.reduced(*delta),
                EventModification::SetTo(value) => damage.with_amount(*value),
                EventModification::ReduceToZero => damage.prevented(),
            };
            Some(Event::new(modified))
        }
        EventKind::LifeGain => {
            let life_gain = downcast_event::<LifeGainEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    life_gain.with_amount(life_gain.amount.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    life_gain.with_amount((life_gain.amount as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => {
                    life_gain.with_amount(life_gain.amount.saturating_sub(*delta))
                }
                EventModification::SetTo(value) => life_gain.with_amount(*value),
                EventModification::ReduceToZero => life_gain.with_amount(0),
            };
            Some(Event::new(modified))
        }
        EventKind::PutCounters => {
            let put_counters = downcast_event::<PutCountersEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    put_counters.with_count(put_counters.count.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    put_counters.with_count((put_counters.count as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => {
                    put_counters.with_count(put_counters.count.saturating_sub(*delta))
                }
                EventModification::SetTo(value) => put_counters.with_count(*value),
                EventModification::ReduceToZero => put_counters.with_count(0),
            };
            Some(Event::new(modified))
        }
        EventKind::Draw => {
            let draw = downcast_event::<DrawEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    draw.with_count(draw.count.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    draw.with_count((draw.count as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => {
                    draw.with_count(draw.count.saturating_sub(*delta))
                }
                EventModification::SetTo(value) => draw.with_count(*value),
                EventModification::ReduceToZero => draw.with_count(0),
            };
            Some(Event::new(modified))
        }
        _ => None,
    }
}

/// Apply doubling to a trait-based event.
fn apply_trait_double(event: &Event) -> Option<Event> {
    use crate::events::{DamageEvent, DrawEvent, LifeGainEvent, PutCountersEvent, downcast_event};

    match event.kind() {
        EventKind::Damage => {
            let damage = downcast_event::<DamageEvent>(event.inner())?;
            Some(Event::new(damage.doubled()))
        }
        EventKind::LifeGain => {
            let life_gain = downcast_event::<LifeGainEvent>(event.inner())?;
            Some(Event::new(life_gain.doubled()))
        }
        EventKind::PutCounters => {
            let put_counters = downcast_event::<PutCountersEvent>(event.inner())?;
            Some(Event::new(put_counters.doubled()))
        }
        EventKind::Draw => {
            let draw = downcast_event::<DrawEvent>(event.inner())?;
            Some(Event::new(draw.doubled()))
        }
        _ => None,
    }
}

/// Apply destination change to a zone change or ETB event.
fn apply_trait_change_destination(event: &Event, new_zone: Zone) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            Some(Event::new(zone_change.with_destination(new_zone)))
        }
        EventKind::EnterBattlefield => {
            // Convert EnterBattlefield event to a ZoneChange event with the new destination
            // This happens when a replacement effect redirects the destination (e.g., Mox Diamond
            // going to graveyard instead of battlefield when no lands are discarded)
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(Event::zone_change(etb.object, etb.from, new_zone, None))
        }
        _ => None,
    }
}

/// Apply "enters tapped" to an ETB event.
fn apply_trait_enter_tapped(event: &Event) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(Event::new(etb.with_tapped()))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                // Convert to EnterBattlefieldEvent with tapped
                Some(Event::enter_battlefield(
                    *zone_change.objects.first()?,
                    zone_change.from,
                    true,
                    vec![],
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Apply "enters with counters" to an ETB event.
fn apply_trait_enter_with_counters(
    event: &Event,
    counter_type: CounterType,
    count: u32,
) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(Event::new(etb.with_counters(counter_type, count)))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                Some(Event::enter_battlefield(
                    *zone_change.objects.first()?,
                    zone_change.from,
                    false,
                    vec![(counter_type, count)],
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Apply "enters as a copy" to an ETB event.
fn apply_trait_enter_as_copy(event: &Event, source_id: crate::ids::ObjectId) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(Event::new(etb.with_copy_of(source_id)))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                let mut etb =
                    EnterBattlefieldEvent::new(*zone_change.objects.first()?, zone_change.from);
                etb = etb.with_copy_of(source_id);
                Some(Event::new(etb))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Apply redirect to a trait-based event.
fn apply_trait_redirect(
    event: &Event,
    redirect_target: &crate::replacement::RedirectTarget,
    which: &crate::replacement::RedirectWhich,
    effect_controller: PlayerId,
    _effect_source: crate::ids::ObjectId,
) -> Option<Event> {
    use crate::game_state::Target;
    use crate::replacement::{RedirectTarget, RedirectWhich};

    let redirectable = event.0.redirectable_targets();
    if redirectable.is_empty() {
        return None;
    }

    let selected = match which {
        RedirectWhich::First => redirectable.first(),
        RedirectWhich::Index(idx) => redirectable.get(*idx),
        RedirectWhich::ByDescription(desc) => redirectable.iter().find(|t| t.description == *desc),
    }?;

    let new_target = match redirect_target {
        RedirectTarget::ToController => Target::Player(effect_controller),
        RedirectTarget::ToPlayer(player_id) => Target::Player(*player_id),
        RedirectTarget::ToObject(object_id) => Target::Object(*object_id),
        RedirectTarget::ToSource => Target::Object(event.0.source_object()?),
    };

    if !selected.valid_redirect_types.is_valid(&new_target) {
        return None;
    }

    let new_event_box = event
        .0
        .with_target_replaced(&selected.target, &new_target)?;
    Some(Event(new_event_box))
}

/// Result of processing an event through replacement effects.
///
/// Indicates how the event should proceed after checking replacement effects.
#[derive(Debug, Clone)]
pub enum TraitEventResult {
    /// Event should proceed (possibly modified).
    Proceed(Event),
    /// Event should proceed with modifications.
    Modified(Event),
    /// Event was prevented entirely.
    Prevented,
    /// Event was replaced with other effects.
    Replaced {
        effects: Vec<crate::effect::Effect>,
        /// The ID of the replacement effect that was applied.
        /// Used to consume one-shot effects after application.
        effect_id: crate::replacement::ReplacementEffectId,
    },
    /// Multiple replacement effects apply - player must choose.
    NeedsChoice {
        player: PlayerId,
        applicable_effects: Vec<crate::replacement::ReplacementEffectId>,
        event: Box<Event>,
    },
    /// An interactive replacement effect needs player input.
    ///
    /// Used by effects like Mox Diamond (discard or redirect) and shock lands
    /// (pay life or enter tapped). The caller must:
    /// 1. Get the player's decision using the provided `decision_ctx`
    /// 2. Call `continue_interactive_replacement()` with the response
    /// 3. Use the result to determine the final event outcome
    NeedsInteraction {
        /// The decision context that needs to be resolved by the player.
        decision_ctx: crate::decisions::context::DecisionContext,
        /// The zone to redirect to if the player declines or can't pay.
        redirect_zone: Zone,
        /// The ID of the replacement effect, for tracking.
        effect_id: crate::replacement::ReplacementEffectId,
        /// The object being affected.
        object_id: crate::ids::ObjectId,
        /// The original event being processed.
        event: Box<Event>,
        /// The filter for discarding (for InteractiveDiscardOrRedirect).
        filter: Option<crate::target::ObjectFilter>,
        /// The life cost (for InteractivePayLifeOrEnterTapped).
        life_cost: Option<u32>,
    },
}

impl TraitEventResult {
    /// Check if the event was prevented.
    pub fn is_prevented(&self) -> bool {
        matches!(self, TraitEventResult::Prevented)
    }

    /// Get the final event if it proceeded (possibly modified).
    pub fn into_event(self) -> Option<Event> {
        match self {
            TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => Some(e),
            _ => None,
        }
    }
}

fn resolve_value_for_etb(
    count: &crate::effect::Value,
    game: &GameState,
    source: crate::ids::ObjectId,
) -> u32 {
    let controller = game
        .object(source)
        .map(|o| o.controller)
        .unwrap_or(crate::ids::PlayerId::from_index(0));

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::executor::ExecutionContext::new(source, controller, &mut dm);

    if let Some(source_obj) = game.object(source) {
        ctx.optional_costs_paid = source_obj.optional_costs_paid.clone();
    }

    crate::effects::helpers::resolve_value(game, count, &ctx)
        .unwrap_or(0)
        .max(0) as u32
}

// =============================================================================
// Unified Event Outcome Type
// =============================================================================

/// Unified result of processing any event through replacement effects.
///
/// This generic type provides a consistent interface for all event processing,
/// replacing the separate `DestroyResult`, `ZoneChangeResult`, and `DrawResult`
/// types. The type parameter `T` represents the "success" value type for the
/// specific event:
/// - For destroy events: `Zone` (the final destination)
/// - For zone change events: `Zone` (the final destination)
/// - For draw events: `u32` (the number of cards drawn)
///
/// # Variants
///
/// - `Proceed(T)` - Event proceeds with the given result value
/// - `Prevented` - Event was prevented entirely (e.g., indestructible)
/// - `Replaced` - Event was replaced with other effects (already executed)
/// - `NotApplicable` - Object didn't exist or wasn't applicable
#[derive(Debug, Clone, PartialEq)]
pub enum EventOutcome<T> {
    /// Event proceeds with the given result value.
    Proceed(T),
    /// Event was prevented entirely.
    Prevented,
    /// Event was replaced - replacement effects already executed.
    Replaced,
    /// Object didn't exist or wasn't applicable.
    NotApplicable,
}

impl<T> EventOutcome<T> {
    /// Check if the event was prevented.
    pub fn is_prevented(&self) -> bool {
        matches!(self, EventOutcome::Prevented)
    }

    /// Check if the event was replaced.
    pub fn is_replaced(&self) -> bool {
        matches!(self, EventOutcome::Replaced)
    }

    /// Check if the event proceeded.
    pub fn is_proceed(&self) -> bool {
        matches!(self, EventOutcome::Proceed(_))
    }

    /// Get the result value if the event proceeded.
    pub fn into_result(self) -> Option<T> {
        match self {
            EventOutcome::Proceed(t) => Some(t),
            _ => None,
        }
    }

    /// Map the result value.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> EventOutcome<U> {
        match self {
            EventOutcome::Proceed(t) => EventOutcome::Proceed(f(t)),
            EventOutcome::Prevented => EventOutcome::Prevented,
            EventOutcome::Replaced => EventOutcome::Replaced,
            EventOutcome::NotApplicable => EventOutcome::NotApplicable,
        }
    }
}

/// Type alias for destroy event outcomes.
pub type DestroyOutcome = EventOutcome<Zone>;

/// Type alias for zone change event outcomes.
pub type ZoneChangeOutcome = EventOutcome<Zone>;

/// Type alias for draw event outcomes.
pub type DrawOutcome = EventOutcome<u32>;

// =============================================================================
// Event processing result types and functions
// =============================================================================

/// Result of attempting to destroy a permanent.
#[derive(Debug, Clone, PartialEq)]
pub enum DestroyResult {
    /// The permanent was destroyed and is now in the specified zone.
    /// Normally this is the graveyard, but replacement effects can change the destination.
    Destroyed { final_zone: Zone },

    /// The destruction was prevented (indestructible, "can't be destroyed" effect).
    Prevented,

    /// The destruction was replaced (regeneration shield used).
    Replaced,

    /// The permanent didn't exist or wasn't on the battlefield.
    NotApplicable,
}

impl DestroyResult {
    /// Returns true if the permanent actually died (went to graveyard).
    pub fn died(&self) -> bool {
        matches!(
            self,
            DestroyResult::Destroyed {
                final_zone: Zone::Graveyard
            }
        )
    }

    /// Returns true if the destruction was successful (permanent left the battlefield).
    pub fn was_destroyed(&self) -> bool {
        matches!(self, DestroyResult::Destroyed { .. })
    }
}

/// Process a destroy event through the event system.
///
/// Handles all the special cases for destruction:
/// - Indestructible permanents (prevents destruction)
/// - "Can't be destroyed" effects (prevents destruction)
/// - Regeneration shields (replaces destruction with tap + remove damage)
/// - Other replacement effects that modify zone changes
///
/// Returns a `DestroyResult` indicating what happened to the permanent.
pub fn process_destroy_full(
    game: &mut GameState,
    permanent: crate::ids::ObjectId,
    source: Option<crate::ids::ObjectId>,
) -> DestroyResult {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    match process_destroy(game, permanent, source, &mut dm) {
        EventOutcome::Proceed(final_zone) => DestroyResult::Destroyed { final_zone },
        EventOutcome::Prevented => DestroyResult::Prevented,
        EventOutcome::Replaced => DestroyResult::Replaced,
        EventOutcome::NotApplicable => DestroyResult::NotApplicable,
    }
}

// =============================================================================
// New process functions with DecisionMaker support
// =============================================================================

/// Process a destroy event with optional DecisionMaker for resolving choices.
///
/// This is the new API that uses `EventOutcome` and can resolve `NeedsChoice`
/// synchronously via the decision maker, rather than storing in `pending_replacement_choice`.
///
/// When multiple replacement effects at the same priority apply, the decision maker
/// is used to resolve the choice immediately. If no decision maker is provided and
/// a choice is needed, the first effect is applied automatically.
pub fn process_destroy(
    game: &mut GameState,
    permanent: crate::ids::ObjectId,
    source: Option<crate::ids::ObjectId>,
    dm: &mut impl DecisionMaker,
) -> DestroyOutcome {
    use crate::executor::{ExecutionContext, execute_effect};

    game.update_replacement_effects();

    // Check if the object exists and is on the battlefield
    let Some(obj) = game.object(permanent) else {
        return EventOutcome::NotApplicable;
    };

    if obj.zone != Zone::Battlefield {
        return EventOutcome::NotApplicable;
    }

    // Check for indestructible (this is a static ability that prevents destruction)
    if obj.has_indestructible() {
        return EventOutcome::Prevented;
    }

    // Check "can't be destroyed" effects
    if !game.can_be_destroyed(permanent) {
        return EventOutcome::Prevented;
    }

    // Get the controller before we lose the reference
    let controller = obj.controller;

    // Create the destroy event using the trait-based system
    let event = Event::destroy(permanent, source);

    // Process through replacement effects with NeedsChoice handling
    let result = process_with_dm(game, event.clone(), dm);

    match result {
        TraitEventResult::Prevented => EventOutcome::Prevented,

        TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {
            // Destruction proceeds - now process the zone change
            let zone_result = process_zone_change(
                game,
                permanent,
                Zone::Battlefield,
                Zone::Graveyard,
                dm, // Reuse the decision maker for zone change choices
            );

            match zone_result {
                EventOutcome::Prevented => EventOutcome::Prevented,
                EventOutcome::Proceed(final_zone) => {
                    game.move_object(permanent, final_zone);
                    EventOutcome::Proceed(final_zone)
                }
                EventOutcome::Replaced => EventOutcome::Replaced,
                EventOutcome::NotApplicable => EventOutcome::NotApplicable,
            }
        }

        TraitEventResult::Replaced { effects, effect_id } => {
            // Destruction was replaced with other effects.
            // Execute the replacement effects with a minimal context.
            // The effects typically use ChooseSpec::SpecificObject, so they're self-contained.

            // Consume one-shot effects (like regeneration shields)
            game.replacement_effects.mark_effect_used(effect_id);

            let effect_source = source.unwrap_or(permanent);
            let mut ctx = ExecutionContext::new(effect_source, controller, dm);

            for effect in effects {
                // Ignore errors from effect execution - the replacement still happened
                let _ = execute_effect(game, &effect, &mut ctx);
            }

            EventOutcome::Replaced
        }

        TraitEventResult::NeedsChoice { .. } => {
            debug_assert!(
                false,
                "process_with_dm returned NeedsChoice for destroy event"
            );
            EventOutcome::Prevented
        }

        TraitEventResult::NeedsInteraction { .. } => {
            debug_assert!(
                false,
                "interactive replacement unexpectedly matched destroy event"
            );
            EventOutcome::Prevented
        }
    }
}

/// Process a zone change event with optional DecisionMaker for resolving choices.
///
/// This is the new API that uses `EventOutcome` and can resolve `NeedsChoice`
/// synchronously via the decision maker.
pub fn process_zone_change(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
    dm: &mut (impl DecisionMaker + ?Sized),
) -> ZoneChangeOutcome {
    use crate::events::{ZoneChangeEvent, downcast_event};

    game.update_replacement_effects();

    // Finality counter rule text: "If a creature with a finality counter on it would die, exile it instead."
    // Apply this as a baseline destination rewrite for battlefield->graveyard moves.
    let mut requested_to = to;
    if from == Zone::Battlefield
        && to == Zone::Graveyard
        && game.object_has_card_type(object, CardType::Creature)
        && game
            .object(object)
            .and_then(|obj| obj.counters.get(&CounterType::Finality).copied())
            .unwrap_or(0)
            > 0
    {
        requested_to = Zone::Exile;
    }

    let snapshot = game
        .object(object)
        .map(|o| crate::snapshot::ObjectSnapshot::from_object(o, game));

    let event = Event::zone_change(object, from, requested_to, snapshot);
    let result = process_with_dm(game, event.clone(), dm); // dm is already &mut Option

    match result {
        TraitEventResult::Prevented => EventOutcome::Prevented,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner()) {
                EventOutcome::Proceed(zone_change.to)
            } else {
                EventOutcome::Proceed(requested_to)
            }
        }
        TraitEventResult::Replaced { .. } => EventOutcome::Replaced,
        TraitEventResult::NeedsChoice { .. } => {
            debug_assert!(
                false,
                "process_with_dm returned NeedsChoice for zone change event"
            );
            EventOutcome::Prevented
        }
        // Interactive replacements don't apply to zone change events directly
        // (they're handled at the ETB level)
        TraitEventResult::NeedsInteraction { .. } => {
            debug_assert!(
                false,
                "interactive replacement unexpectedly matched zone change event"
            );
            EventOutcome::Prevented
        }
    }
}

/// Process a draw event with optional DecisionMaker for resolving choices.
///
/// This is the new API that uses `EventOutcome` and can resolve `NeedsChoice`
/// synchronously via the decision maker.
pub fn process_draw(
    game: &mut GameState,
    player: PlayerId,
    count: u32,
    is_first_this_turn: bool,
    dm: &mut impl DecisionMaker,
) -> DrawOutcome {
    use crate::events::{DrawEvent, downcast_event};

    game.update_replacement_effects();

    // Check if player can draw cards
    if !game.can_draw(player) {
        return EventOutcome::Prevented;
    }

    let event = Event::draw(player, count, is_first_this_turn);
    let result = process_with_dm(game, event.clone(), dm); // dm is already &mut Option

    match result {
        TraitEventResult::Prevented => EventOutcome::Prevented,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(draw) = downcast_event::<DrawEvent>(e.inner()) {
                EventOutcome::Proceed(draw.count)
            } else {
                EventOutcome::Proceed(count)
            }
        }
        TraitEventResult::Replaced { .. } => EventOutcome::Replaced,
        TraitEventResult::NeedsChoice { .. } => {
            debug_assert!(false, "process_with_dm returned NeedsChoice for draw event");
            EventOutcome::Prevented
        }
        // Interactive replacements don't apply to draw events
        TraitEventResult::NeedsInteraction { .. } => {
            debug_assert!(
                false,
                "interactive replacement unexpectedly matched draw event"
            );
            EventOutcome::Prevented
        }
    }
}

/// Process an event through replacement effects, using a DecisionMaker to resolve choices.
///
/// When `NeedsChoice` is returned (multiple effects at same priority), this function
/// uses the decision maker to ask the player which replacement effect to apply.
/// If no decision maker is provided, the first applicable effect is chosen automatically.
///
/// Takes a mutable reference to the Option so the decision maker can be reused by the caller
/// for subsequent processing (e.g., zone change after destruction).
fn process_with_dm(
    game: &GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
) -> TraitEventResult {
    use crate::decisions::{
        make_decision,
        specs::{ReplacementOption, ReplacementSpec},
    };

    let mut current_event = event;
    let mut state = TraitEventProcessingState::default();

    loop {
        let result = process_event_direct(game, current_event.clone(), &mut state, &[]);

        match result {
            TraitEventResult::NeedsChoice {
                player,
                applicable_effects,
                event: boxed_event,
            } => {
                // Determine which effect to apply
                let chosen_index = {
                    // Build options for the decision
                    let options: Vec<ReplacementOption> = applicable_effects
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, &id)| {
                            game.replacement_effects.get_effect(id).map(|e| {
                                ReplacementOption::new(
                                    idx,
                                    e.source,
                                    e.matcher
                                        .as_ref()
                                        .map(|m| m.display())
                                        .unwrap_or_else(|| "Unknown effect".to_string()),
                                )
                            })
                        })
                        .collect();

                    let spec = ReplacementSpec::new(options);
                    make_decision(game, dm, player, None, spec)
                };

                // Apply the chosen effect immediately, then continue processing
                let chosen_id = applicable_effects
                    .get(chosen_index)
                    .copied()
                    .or_else(|| applicable_effects.first().copied());

                let Some(effect_id) = chosen_id else {
                    return TraitEventResult::Proceed(*boxed_event);
                };

                let Some(chosen_effect) = game.replacement_effects.get_effect(effect_id).cloned()
                else {
                    // Effect disappeared (e.g., source left battlefield). Continue with event.
                    state.mark_applied(effect_id);
                    current_event = *boxed_event;
                    continue;
                };

                state.mark_applied(effect_id);

                match apply_trait_replacement(game, *boxed_event, &chosen_effect) {
                    TraitApplyResult::Modified(modified_event) => {
                        current_event = modified_event;
                    }
                    TraitApplyResult::Prevented => return TraitEventResult::Prevented,
                    TraitApplyResult::Replaced(effects) => {
                        return TraitEventResult::Replaced { effects, effect_id };
                    }
                    TraitApplyResult::Unchanged(unchanged_event) => {
                        current_event = unchanged_event;
                    }
                    TraitApplyResult::NeedsInteraction {
                        decision_ctx,
                        redirect_zone,
                        effect_id,
                        object_id,
                        filter,
                    } => {
                        return TraitEventResult::NeedsInteraction {
                            decision_ctx,
                            redirect_zone,
                            effect_id,
                            object_id,
                            event: Box::new(current_event),
                            filter,
                            life_cost: match &chosen_effect.replacement {
                                ReplacementAction::InteractivePayLifeOrEnterTapped {
                                    life_cost,
                                } => Some(*life_cost),
                                _ => None,
                            },
                        };
                    }
                }
            }
            other => return other,
        }
    }
}

fn find_effect_for_choice(
    game: &GameState,
    self_effects: &[ReplacementEffect],
    id: ReplacementEffectId,
) -> Option<ReplacementEffect> {
    game.replacement_effects
        .get_effect(id)
        .cloned()
        .or_else(|| self_effects.iter().find(|e| e.id == id).cloned())
}

fn assign_ephemeral_effect_ids(effects: &mut [ReplacementEffect]) {
    // Keep ephemeral IDs far away from manager-issued IDs.
    const EPHEMERAL_ID_BASE: u64 = u64::MAX - 1_000_000;
    for (idx, effect) in effects.iter_mut().enumerate() {
        effect.id = ReplacementEffectId(EPHEMERAL_ID_BASE.saturating_add(idx as u64));
    }
}

/// Result of processing an ETB (Enter the Battlefield) event.
#[derive(Debug, Clone, Default)]
pub struct EtbEventResult {
    /// Whether the permanent enters tapped
    pub enters_tapped: bool,
    /// Counters the permanent enters with (counter_type, count)
    pub enters_with_counters: Vec<(CounterType, u32)>,
    /// Whether the ETB was prevented (e.g., creature entering from graveyard replaced with exile)
    pub prevented: bool,
    /// If zone was changed, the new destination
    pub new_destination: Option<Zone>,
    /// If set, the object enters as a copy of this source object.
    pub enters_as_copy_of: Option<crate::ids::ObjectId>,
    /// An interactive replacement that requires player input.
    ///
    /// If present, the caller must:
    /// 1. Present the decision to the player
    /// 2. Call `continue_interactive_replacement()` with the response
    /// 3. Use the result to determine if the permanent enters
    pub interactive_replacement: Option<InteractiveEtbReplacement>,
}

/// Information about an interactive ETB replacement effect.
#[derive(Debug, Clone)]
pub struct InteractiveEtbReplacement {
    /// The decision context that needs to be resolved by the player.
    pub decision_ctx: crate::decisions::context::DecisionContext,
    /// The zone to redirect to if the player declines or can't pay.
    pub redirect_zone: Zone,
    /// The ID of the replacement effect.
    pub effect_id: ReplacementEffectId,
    /// The filter for discarding (for InteractiveDiscardOrRedirect).
    pub filter: Option<crate::target::ObjectFilter>,
    /// The life cost (for InteractivePayLifeOrEnterTapped).
    pub life_cost: Option<u32>,
}

// =============================================================================
// Event-based convenience functions (trait-based API)
// =============================================================================

/// Process a damage event using the Event type.
pub fn process_damage_with_event(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
) -> (u32, bool) {
    process_damage_with_event_with_source_snapshot(game, source, target, amount, is_combat, None)
}

/// Process a damage event using the Event type, with optional source LKI.
///
/// When `source_snapshot` is provided and the source object is no longer present
/// in game state, source-dependent checks (like prevention based on source color/type)
/// use the snapshot as last known information.
pub fn process_damage_with_event_with_source_snapshot(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> (u32, bool) {
    use crate::events::{DamageEvent, downcast_event};

    // Check if damage can be prevented
    let can_prevent = game.can_prevent_damage();

    // Create the event using the new Event type
    let event = if can_prevent {
        Event::damage(source, target, amount, is_combat)
    } else {
        Event::unpreventable_damage(source, target, amount, is_combat)
    };

    // Process through the trait-based system
    let result = process_trait_event(game, event);

    let after_replacement = match result {
        TraitEventResult::Prevented => return (0, true),
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            // Extract the final damage amount from the event
            if let Some(damage) = downcast_event::<DamageEvent>(e.inner()) {
                damage.amount
            } else {
                debug_assert!(
                    false,
                    "damage replacement processing returned a non-DamageEvent"
                );
                0
            }
        }
        _ => amount,
    };

    if after_replacement == 0 {
        return (0, false);
    }

    // Apply prevention shields
    let (source_colors, source_card_types) = if let Some(obj) = game.object(source) {
        (obj.colors(), obj.card_types.clone())
    } else if let Some(snapshot) = source_snapshot {
        (snapshot.colors, snapshot.card_types.clone())
    } else {
        (crate::color::ColorSet::COLORLESS, Vec::new())
    };

    let final_damage = match target {
        DamageTarget::Player(player_id) => game.prevention_effects.apply_prevention_to_player(
            player_id,
            after_replacement,
            is_combat,
            source,
            &source_colors,
            &source_card_types,
            can_prevent,
        ),
        DamageTarget::Object(object_id) => {
            let controller = game
                .object(object_id)
                .map(|o| o.controller)
                .unwrap_or(game.turn.active_player);
            game.prevention_effects.apply_prevention_to_permanent(
                object_id,
                controller,
                after_replacement,
                is_combat,
                source,
                &source_colors,
                &source_card_types,
                can_prevent,
            )
        }
    };

    (final_damage, false)
}

/// Process a life gain event using the new Event type.
///
/// This is the Event-based version of `process_life_gain_event`.
pub fn process_life_gain_with_event(game: &GameState, player: PlayerId, amount: u32) -> u32 {
    use crate::events::{LifeGainEvent, downcast_event};

    if !game.can_gain_life(player) {
        return 0;
    }

    let event = Event::life_gain(player, amount);
    let result = process_trait_event(game, event);

    match result {
        TraitEventResult::Prevented => 0,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(life_gain) = downcast_event::<LifeGainEvent>(e.inner()) {
                life_gain.amount
            } else {
                amount
            }
        }
        _ => amount,
    }
}

/// Process a dies event using the new Event type.
///
/// This processes a creature dying through the replacement effect system,
/// handling effects like "exile instead of dying".
///
/// Returns the zone the creature should go to (Graveyard by default, or
/// another zone if a replacement effect changed it), or None if prevented.
pub fn process_dies_with_event(
    game: &GameState,
    creature: crate::ids::ObjectId,
    snapshot: crate::snapshot::ObjectSnapshot,
) -> Option<Zone> {
    use crate::events::{ZoneChangeEvent, downcast_event};

    let event = Event::zone_change(creature, Zone::Battlefield, Zone::Graveyard, Some(snapshot));
    let result = process_trait_event(game, event);

    match result {
        TraitEventResult::Prevented => None,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner()) {
                // Replacement effect changed the destination
                Some(zone_change.to)
            } else {
                debug_assert!(
                    false,
                    "dies replacement processing returned a non-zone-change event"
                );
                None
            }
        }
        _ => Some(Zone::Graveyard),
    }
}

/// Process a zone change event using the new Event type.
///
/// Returns the final destination zone, or None if the change was prevented.
pub fn process_zone_change_with_event(
    game: &GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
) -> Option<Zone> {
    use crate::events::{ZoneChangeEvent, downcast_event};

    let event = Event::zone_change(object, from, to, None);
    let result = process_trait_event(game, event);

    match result {
        TraitEventResult::Prevented => None,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner()) {
                Some(zone_change.to)
            } else {
                Some(to)
            }
        }
        _ => Some(to),
    }
}

/// Process a put counters event using the new Event type.
///
/// Returns the final number of counters to place.
pub fn process_put_counters_with_event(
    game: &GameState,
    target: crate::ids::ObjectId,
    counter_type: CounterType,
    count: u32,
) -> u32 {
    use crate::events::{PutCountersEvent, downcast_event};

    if !game.can_have_counters_placed(target) {
        return 0;
    }

    let event = Event::put_counters(target, counter_type, count);
    let result = process_trait_event(game, event);

    match result {
        TraitEventResult::Prevented => 0,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(put_counters) = downcast_event::<PutCountersEvent>(e.inner()) {
                put_counters.count
            } else {
                count
            }
        }
        _ => count,
    }
}

/// Process an ETB event using the new Event type.
///
/// This is the Event-based version of `process_etb_event`.
pub fn process_etb_with_event(
    game: &GameState,
    object: crate::ids::ObjectId,
    from: Zone,
) -> EtbEventResult {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut game_clone = game.clone();
    process_etb_with_event_and_dm(&mut game_clone, object, from, &mut dm)
}

/// Process an ETB event and fully resolve all replacement choices/interactions.
pub fn process_etb_with_event_and_dm(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    dm: &mut impl DecisionMaker,
) -> EtbEventResult {
    use crate::ability::AbilityKind;
    use crate::decisions::{
        make_decision,
        specs::{ReplacementOption, ReplacementSpec},
    };
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    game.update_replacement_effects();

    // Check the object's own abilities for self-ETB effects
    // Per Rule 616.1a, self-replacement effects apply first
    let mut enters_tapped = false;
    let enters_with_counters: Vec<(CounterType, u32)> = Vec::new();

    // Gather self-replacement effects from the object's abilities
    // These are effects like shock lands' "pay 2 life or enter tapped"
    let mut self_replacement_effects: Vec<ReplacementEffect> = Vec::new();

    if let Some(obj) = game.object(object) {
        let controller = obj.controller;
        for ability in &obj.abilities {
            if let AbilityKind::Static(s) = &ability.kind {
                if s.enters_tapped() {
                    enters_tapped = true;
                }
                // Check for unified replacement effects
                if let Some(effect) = s.generate_replacement_effect(object, controller) {
                    self_replacement_effects.push(effect);
                }
            }
        }
    }
    assign_ephemeral_effect_ids(&mut self_replacement_effects);

    let mut current_event = Event::new(EnterBattlefieldEvent {
        object,
        from,
        enters_tapped,
        enters_with_counters,
        enters_as_copy_of: None,
    });
    let mut state = TraitEventProcessingState::default();

    loop {
        let result = process_event_direct(
            game,
            current_event.clone(),
            &mut state,
            &self_replacement_effects,
        );

        match result {
            TraitEventResult::Prevented => {
                return EtbEventResult {
                    prevented: true,
                    ..Default::default()
                };
            }
            TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
                if let Some(etb) = downcast_event::<EnterBattlefieldEvent>(e.inner()) {
                    return EtbEventResult {
                        enters_tapped: etb.enters_tapped,
                        enters_with_counters: etb.enters_with_counters.clone(),
                        prevented: false,
                        new_destination: None,
                        enters_as_copy_of: etb.enters_as_copy_of,
                        interactive_replacement: None,
                    };
                }
                if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner()) {
                    return EtbEventResult {
                        prevented: zone_change.to != Zone::Battlefield,
                        new_destination: if zone_change.to != Zone::Battlefield {
                            Some(zone_change.to)
                        } else {
                            None
                        },
                        interactive_replacement: None,
                        ..Default::default()
                    };
                }
                return EtbEventResult::default();
            }
            TraitEventResult::Replaced { effects, effect_id } => {
                use crate::executor::{ExecutionContext, execute_effect};
                if let Some(controller) = game.object(object).map(|o| o.controller) {
                    game.replacement_effects.mark_effect_used(effect_id);
                    let mut ctx = ExecutionContext::new(object, controller, dm);
                    for effect in effects {
                        let _ = execute_effect(game, &effect, &mut ctx);
                    }
                }
                return EtbEventResult {
                    prevented: true,
                    ..Default::default()
                };
            }
            TraitEventResult::NeedsChoice {
                player,
                applicable_effects,
                event,
            } => {
                let options: Vec<ReplacementOption> = applicable_effects
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, &id)| {
                        find_effect_for_choice(game, &self_replacement_effects, id).map(|e| {
                            ReplacementOption::new(
                                idx,
                                e.source,
                                e.matcher
                                    .as_ref()
                                    .map(|m| m.display())
                                    .unwrap_or_else(|| "Unknown effect".to_string()),
                            )
                        })
                    })
                    .collect();
                let chosen_index =
                    make_decision(game, dm, player, None, ReplacementSpec::new(options));
                let chosen_id = applicable_effects
                    .get(chosen_index)
                    .copied()
                    .or_else(|| applicable_effects.first().copied());
                let Some(chosen_id) = chosen_id else {
                    return EtbEventResult::default();
                };
                let Some(chosen_effect) =
                    find_effect_for_choice(game, &self_replacement_effects, chosen_id)
                else {
                    state.mark_applied(chosen_id);
                    current_event = *event;
                    continue;
                };

                state.mark_applied(chosen_id);
                match apply_trait_replacement(game, *event, &chosen_effect) {
                    TraitApplyResult::Modified(modified_event) => current_event = modified_event,
                    TraitApplyResult::Prevented => {
                        return EtbEventResult {
                            prevented: true,
                            ..Default::default()
                        };
                    }
                    TraitApplyResult::Replaced(effects) => {
                        use crate::executor::{ExecutionContext, execute_effect};
                        if let Some(controller) = game.object(object).map(|o| o.controller) {
                            game.replacement_effects.mark_effect_used(chosen_id);
                            let mut ctx = ExecutionContext::new(object, controller, dm);
                            for effect in effects {
                                let _ = execute_effect(game, &effect, &mut ctx);
                            }
                        }
                        return EtbEventResult {
                            prevented: true,
                            ..Default::default()
                        };
                    }
                    TraitApplyResult::Unchanged(unchanged_event) => current_event = unchanged_event,
                    TraitApplyResult::NeedsInteraction {
                        decision_ctx,
                        redirect_zone,
                        effect_id,
                        object_id,
                        filter,
                    } => {
                        let life_cost = match &chosen_effect.replacement {
                            ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } => {
                                Some(*life_cost)
                            }
                            _ => None,
                        };
                        let controller = game
                            .object(object_id)
                            .map(|o| o.controller)
                            .unwrap_or(PlayerId::from_index(0));
                        let response = match decision_ctx {
                            crate::decisions::context::DecisionContext::Boolean(ctx) => {
                                if dm.decide_boolean(game, &ctx) {
                                    InteractiveReplacementResponse::Accept
                                } else {
                                    InteractiveReplacementResponse::Decline
                                }
                            }
                            crate::decisions::context::DecisionContext::SelectObjects(ctx) => {
                                InteractiveReplacementResponse::Objects(
                                    dm.decide_objects(game, &ctx),
                                )
                            }
                            _ => InteractiveReplacementResponse::Decline,
                        };
                        state.mark_applied(effect_id);
                        let interactive_result = continue_interactive_replacement(
                            game,
                            &response,
                            object_id,
                            controller,
                            filter.as_ref(),
                            redirect_zone,
                            life_cost,
                            dm,
                        );
                        if !interactive_result.enters {
                            return EtbEventResult {
                                prevented: true,
                                new_destination: interactive_result.redirect_zone,
                                ..Default::default()
                            };
                        }
                        if interactive_result.enters_tapped
                            && let Some(tapped_event) = apply_trait_enter_tapped(&current_event)
                        {
                            current_event = tapped_event;
                        }
                    }
                }
            }
            TraitEventResult::NeedsInteraction {
                decision_ctx,
                redirect_zone,
                effect_id,
                object_id,
                event,
                filter,
                life_cost,
            } => {
                let controller = game
                    .object(object_id)
                    .map(|o| o.controller)
                    .unwrap_or(PlayerId::from_index(0));
                let response = match decision_ctx {
                    crate::decisions::context::DecisionContext::Boolean(ctx) => {
                        if dm.decide_boolean(game, &ctx) {
                            InteractiveReplacementResponse::Accept
                        } else {
                            InteractiveReplacementResponse::Decline
                        }
                    }
                    crate::decisions::context::DecisionContext::SelectObjects(ctx) => {
                        InteractiveReplacementResponse::Objects(dm.decide_objects(game, &ctx))
                    }
                    _ => InteractiveReplacementResponse::Decline,
                };
                state.mark_applied(effect_id);
                let interactive_result = continue_interactive_replacement(
                    game,
                    &response,
                    object_id,
                    controller,
                    filter.as_ref(),
                    redirect_zone,
                    life_cost,
                    dm,
                );
                if !interactive_result.enters {
                    return EtbEventResult {
                        prevented: true,
                        new_destination: interactive_result.redirect_zone,
                        ..Default::default()
                    };
                }

                current_event = *event;
                if interactive_result.enters_tapped
                    && let Some(tapped_event) = apply_trait_enter_tapped(&current_event)
                {
                    current_event = tapped_event;
                }
            }
        }
    }
}

/// Result of processing a zone change event with full replacement effect handling.
///
/// Unlike `process_zone_change_with_event` which returns `Option<Zone>`, this
/// returns the full result including replacement effects and pending choices.
#[derive(Debug, Clone)]
pub enum ZoneChangeResult {
    /// Zone change proceeds to the specified zone.
    Proceed(Zone),
    /// Zone change was prevented.
    Prevented,
    /// Zone change was replaced with other effects.
    Replaced(Vec<crate::effect::Effect>),
    /// Multiple replacement effects apply, player must choose.
    NeedsChoice {
        player: PlayerId,
        applicable_effects: Vec<ReplacementEffectId>,
        event: Box<Event>,
        default_zone: Zone,
    },
}

/// Process a zone change event with full replacement effect handling.
///
/// This is the comprehensive version that returns all possible outcomes,
/// including `Replaced` and `NeedsChoice` cases that the simpler
/// `process_zone_change_with_event` doesn't support.
pub fn process_zone_change_full(
    game: &GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
) -> ZoneChangeResult {
    use crate::events::{ZoneChangeEvent, downcast_event};

    let snapshot = game
        .object(object)
        .map(|o| crate::snapshot::ObjectSnapshot::from_object(o, game));

    let event = Event::zone_change(object, from, to, snapshot);
    let result = process_trait_event(game, event);

    match result {
        TraitEventResult::Prevented => ZoneChangeResult::Prevented,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner()) {
                ZoneChangeResult::Proceed(zone_change.to)
            } else {
                ZoneChangeResult::Proceed(to)
            }
        }
        TraitEventResult::Replaced { effects, .. } => ZoneChangeResult::Replaced(effects),
        TraitEventResult::NeedsChoice {
            player,
            applicable_effects,
            event,
        } => ZoneChangeResult::NeedsChoice {
            player,
            applicable_effects,
            event,
            default_zone: to,
        },
        // Interactive replacements don't apply to zone change events directly
        TraitEventResult::NeedsInteraction { .. } => ZoneChangeResult::Proceed(to),
    }
}

/// Result of processing a draw event with full replacement effect handling.
#[derive(Debug, Clone)]
pub enum DrawResult {
    /// Player should draw the specified number of cards.
    Proceed(u32),
    /// Drawing was prevented.
    Prevented,
    /// Drawing was replaced with other effects.
    Replaced(Vec<crate::effect::Effect>),
    /// Multiple replacement effects apply, player must choose.
    NeedsChoice {
        player: PlayerId,
        applicable_effects: Vec<ReplacementEffectId>,
        event: Box<Event>,
        default_count: u32,
    },
}

/// Process a draw event with full replacement effect handling.
///
/// This is the comprehensive version that returns all possible outcomes,
/// using the new Event type.
pub fn process_draw_full(
    game: &GameState,
    player: PlayerId,
    count: u32,
    is_first_this_turn: bool,
) -> DrawResult {
    use crate::events::{DrawEvent, downcast_event};

    // Check if player can draw cards
    if !game.can_draw(player) {
        return DrawResult::Prevented;
    }

    let event = Event::draw(player, count, is_first_this_turn);
    let result = process_trait_event(game, event);

    match result {
        TraitEventResult::Prevented => DrawResult::Prevented,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(draw) = downcast_event::<DrawEvent>(e.inner()) {
                DrawResult::Proceed(draw.count)
            } else {
                DrawResult::Proceed(count)
            }
        }
        TraitEventResult::Replaced { effects, .. } => DrawResult::Replaced(effects),
        TraitEventResult::NeedsChoice {
            player,
            applicable_effects,
            event,
        } => DrawResult::NeedsChoice {
            player,
            applicable_effects,
            event,
            default_count: count,
        },
        // Interactive replacements don't apply to draw events
        TraitEventResult::NeedsInteraction { .. } => DrawResult::Proceed(count),
    }
}

/// Process an event with a chosen replacement effect, using the new Event type.
///
/// When a player chooses which replacement effect to apply (per Rule 616.1e),
/// this function applies that effect and continues processing.
pub fn process_event_with_chosen_replacement_trait(
    game: &GameState,
    event: Event,
    chosen_effect_id: ReplacementEffectId,
) -> TraitEventResult {
    // Get the chosen effect
    let Some(effect) = game.replacement_effects.get_effect(chosen_effect_id) else {
        // Effect no longer exists - just process normally
        return process_trait_event(game, event);
    };

    // Apply the chosen replacement effect
    let apply_result = apply_trait_replacement(game, event.clone(), effect);

    // Create state with the chosen effect marked as applied
    let mut state = TraitEventProcessingState::default();
    state.mark_applied(chosen_effect_id);

    match apply_result {
        TraitApplyResult::Modified(modified) => {
            // Continue processing with the modified event
            process_event_direct(game, modified, &mut state, &[])
        }
        TraitApplyResult::Prevented => TraitEventResult::Prevented,
        TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced {
            effects,
            effect_id: chosen_effect_id,
        },
        TraitApplyResult::Unchanged(unchanged) => {
            // Effect didn't change anything - continue with original event
            process_event_direct(game, unchanged, &mut state, &[])
        }
        TraitApplyResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            filter,
        } => TraitEventResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            event: Box::new(event),
            filter,
            life_cost: match &effect.replacement {
                ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } => {
                    Some(*life_cost)
                }
                _ => None,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::AutoPassDecisionMaker;
    use crate::ids::CardId;
    use crate::ids::ObjectId;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn test_replacement_priority_ordering() {
        assert!(ReplacementPriority::SelfReplacement < ReplacementPriority::ControlChanging);
        assert!(ReplacementPriority::ControlChanging < ReplacementPriority::CopyEffect);
        assert!(ReplacementPriority::CopyEffect < ReplacementPriority::BackFace);
        assert!(ReplacementPriority::BackFace < ReplacementPriority::Other);
    }

    // === Tests for trait-based matcher integration ===

    #[test]
    fn test_trait_based_matcher_integration() {
        use crate::events::DamageToPlayerMatcher;
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        use crate::target::PlayerFilter;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Create a replacement effect using a trait-based matcher
        let matcher = DamageToPlayerMatcher::new(PlayerFilter::You);
        let effect = ReplacementEffect::with_matcher(
            source,
            alice,
            matcher,
            ReplacementAction::Modify(EventModification::Subtract(1)),
        );

        game.replacement_effects.add_effect(effect);

        // Process a damage event to Alice - should be reduced by 1
        let (final_damage, _prevented) =
            process_damage_with_event(&mut game, source, DamageTarget::Player(alice), 5, false);

        assert_eq!(final_damage, 4, "Damage should be reduced by 1");
    }

    #[test]
    fn test_trait_based_matcher_does_not_match_wrong_target() {
        use crate::events::DamageToPlayerMatcher;
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        use crate::target::PlayerFilter;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(1);

        // Create a replacement effect that only matches damage to "you" (Alice)
        let matcher = DamageToPlayerMatcher::new(PlayerFilter::You);
        let effect = ReplacementEffect::with_matcher(
            source,
            alice,
            matcher,
            ReplacementAction::Modify(EventModification::Subtract(1)),
        );

        game.replacement_effects.add_effect(effect);

        // Process a damage event to Bob - should NOT be affected
        let (final_damage, _prevented) =
            process_damage_with_event(&mut game, source, DamageTarget::Player(bob), 5, false);

        assert_eq!(final_damage, 5, "Damage to Bob should not be affected");
    }

    #[test]
    fn test_trait_based_combat_damage_matcher() {
        use crate::events::CombatDamageMatcher;
        use crate::replacement::{ReplacementAction, ReplacementEffect};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Create a replacement effect that doubles combat damage
        let effect = ReplacementEffect::with_matcher(
            source,
            alice,
            CombatDamageMatcher,
            ReplacementAction::Double,
        );

        game.replacement_effects.add_effect(effect);

        // Process combat damage - should be doubled
        let (combat_damage, _) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            true, // is_combat
        );
        assert_eq!(combat_damage, 6, "Combat damage should be doubled");

        // Process noncombat damage - should NOT be doubled
        let (noncombat_damage, _) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            false, // not combat
        );
        assert_eq!(
            noncombat_damage, 3,
            "Noncombat damage should not be doubled"
        );
    }

    #[test]
    fn test_damage_prevention_uses_source_snapshot_when_source_missing() {
        use crate::color::{Color, ColorSet};
        use crate::prevention::PreventionShield;
        use crate::snapshot::ObjectSnapshot;
        use crate::types::CardType;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(42);

        let shield =
            PreventionShield::circle_of_protection(ObjectId::from_raw(1), alice, Color::Red);
        game.prevention_effects.add_shield(shield);

        let source_snapshot = ObjectSnapshot::for_testing(source, alice, "Former Red Source")
            .with_colors(ColorSet::RED)
            .with_card_types(vec![CardType::Creature]);

        let (final_damage, _prevented) = process_damage_with_event_with_source_snapshot(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            false,
            Some(&source_snapshot),
        );

        assert_eq!(
            final_damage, 0,
            "Prevention should apply using source snapshot colors when source is gone"
        );
    }

    #[test]
    fn test_damage_prevention_without_snapshot_treats_missing_source_as_colorless() {
        use crate::color::Color;
        use crate::prevention::PreventionShield;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(42);

        let shield =
            PreventionShield::circle_of_protection(ObjectId::from_raw(1), alice, Color::Red);
        game.prevention_effects.add_shield(shield);

        let (final_damage, _prevented) = process_damage_with_event_with_source_snapshot(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            false,
            None,
        );

        assert_eq!(
            final_damage, 3,
            "Without snapshot, missing source defaults to colorless and should not match red-only prevention"
        );
    }

    #[test]
    fn test_finality_counter_replaces_dies_with_exile() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(1), "Finality Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::power_toughness::PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(creature_id) {
            obj.add_counters(CounterType::Finality, 1);
        }

        let mut dm = AutoPassDecisionMaker;
        let outcome = process_zone_change(
            &mut game,
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            &mut dm,
        );

        assert_eq!(outcome, EventOutcome::Proceed(Zone::Exile));
    }

    #[test]
    fn test_finality_counter_does_not_redirect_noncreatures() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(2), "Finality Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(artifact_id) {
            obj.add_counters(CounterType::Finality, 1);
        }

        let mut dm = AutoPassDecisionMaker;
        let outcome = process_zone_change(
            &mut game,
            artifact_id,
            Zone::Battlefield,
            Zone::Graveyard,
            &mut dm,
        );

        assert_eq!(outcome, EventOutcome::Proceed(Zone::Graveyard));
    }
}
