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

mod application;

use crate::DecisionMaker;
use crate::ability::ActivatedAbilityRuntimeExt as _;
use crate::decisions::replacement_option_description;
use crate::events::DamageTarget;
use crate::events::{Event, EventContext};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::{GameState, UiBattlefieldTransitionKind};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::replacement::{
    ReplacementAction, ReplacementEffect, ReplacementEffectId, ReplacementEffectKey,
};
use crate::types::CardType;
use crate::zone::Zone;
use application::{
    apply_trait_enter_tapped, apply_trait_enter_with_counters, apply_trait_replacement,
    find_matching_cards_in_hand,
};

fn apply_tribute_response(
    game: &GameState,
    event: Event,
    response: &InteractiveReplacementResponse,
    source: ObjectId,
    controller: PlayerId,
    counter_type: CounterType,
    count: u32,
    paid_label: &str,
    paid_labels: &mut Vec<String>,
    dm: &mut dyn DecisionMaker,
) -> Event {
    let response = resolve_tribute_response(game, response, source, controller, count, dm);
    if !matches!(response, InteractiveReplacementResponse::Accept) {
        return event;
    }
    if !paid_labels
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(paid_label))
    {
        paid_labels.push(paid_label.to_string());
    }
    apply_trait_enter_with_counters(&event, counter_type, count, &[], &[]).unwrap_or(event)
}

fn apply_enter_counter_choice_response(
    game: &GameState,
    event: Event,
    response: &InteractiveReplacementResponse,
    source: ObjectId,
    counter_types: &[CounterType],
    count: &crate::effect::Value,
) -> Event {
    let Some(counter_type) = response
        .selected_option_index()
        .and_then(|index| counter_types.get(index))
        .copied()
        .or_else(|| counter_types.first().copied())
    else {
        return event;
    };
    let resolved_count = application::resolve_value_for_etb_for_choice(count, game, source);
    apply_trait_enter_with_counters(&event, counter_type, resolved_count, &[], &[]).unwrap_or(event)
}

impl InteractiveReplacementResponse {
    fn selected_option_index(&self) -> Option<usize> {
        match self {
            InteractiveReplacementResponse::Options(selected) => selected.first().copied(),
            _ => None,
        }
    }
}

pub(super) fn tribute_opponents(game: &GameState, controller: PlayerId) -> Vec<PlayerId> {
    let mut opponents = game
        .players
        .iter()
        .filter(|player| player.is_in_game() && player.id != controller)
        .map(|player| player.id)
        .collect::<Vec<_>>();
    opponents.sort_by_key(|player| player.0);
    opponents
}

fn tribute_source_name(game: &GameState, source: ObjectId) -> String {
    game.object(source)
        .map(|object| object.name.to_string())
        .unwrap_or_else(|| "this creature".to_string())
}

pub(super) fn counter_choice_context(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    counter_types: &[CounterType],
) -> crate::decisions::context::DecisionContext {
    let source_name = tribute_source_name(game, source);
    let options = counter_types
        .iter()
        .enumerate()
        .map(|(index, counter_type)| {
            crate::decisions::context::SelectableOption::new(
                index,
                format!("{} counter", counter_type.description()),
            )
        })
        .collect();
    crate::decisions::context::DecisionContext::SelectOptions(
        crate::decisions::context::SelectOptionsContext::new(
            controller,
            Some(source),
            format!("Choose a counter type for {source_name}"),
            options,
            1,
            1,
        ),
    )
}

pub(super) fn tribute_boolean_context(
    game: &GameState,
    source: ObjectId,
    opponent: PlayerId,
    count: u32,
) -> crate::decisions::context::DecisionContext {
    let source_name = tribute_source_name(game, source);
    let bool_ctx = crate::decisions::context::BooleanContext::new(
        opponent,
        Some(source),
        format!("Put {count} +1/+1 counters on {source_name}? (Tribute {count})"),
    )
    .with_source_name(source_name);
    crate::decisions::context::DecisionContext::Boolean(bool_ctx)
}

pub(super) fn tribute_opponent_choice_context(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    opponents: &[PlayerId],
) -> crate::decisions::context::DecisionContext {
    let source_name = tribute_source_name(game, source);
    let options = opponents
        .iter()
        .enumerate()
        .map(|(index, opponent)| {
            let name = game
                .player(*opponent)
                .map(|player| player.name.to_string())
                .unwrap_or_else(|| format!("Player {}", opponent.index() + 1));
            crate::decisions::context::SelectableOption::new(index, name)
        })
        .collect();
    crate::decisions::context::DecisionContext::SelectOptions(
        crate::decisions::context::SelectOptionsContext::new(
            controller,
            Some(source),
            format!("Choose an opponent for {source_name}'s tribute"),
            options,
            1,
            1,
        ),
    )
}

fn resolve_tribute_response(
    game: &GameState,
    response: &InteractiveReplacementResponse,
    source: ObjectId,
    controller: PlayerId,
    count: u32,
    dm: &mut dyn DecisionMaker,
) -> InteractiveReplacementResponse {
    let InteractiveReplacementResponse::Options(selected) = response else {
        return response.clone();
    };
    let opponents = tribute_opponents(game, controller);
    let Some(opponent) = selected
        .first()
        .and_then(|index| opponents.get(*index))
        .copied()
        .or_else(|| opponents.first().copied())
    else {
        return InteractiveReplacementResponse::Decline;
    };
    let crate::decisions::context::DecisionContext::Boolean(ctx) =
        tribute_boolean_context(game, source, opponent, count)
    else {
        return InteractiveReplacementResponse::Decline;
    };
    if dm.decide_boolean(game, &ctx) {
        InteractiveReplacementResponse::Accept
    } else {
        InteractiveReplacementResponse::Decline
    }
}

fn replacement_effect_choice_description(game: &GameState, effect: &ReplacementEffect) -> String {
    match &effect.replacement {
        ReplacementAction::Additionally(_) => {
            format!(
                "Do not apply {}",
                replacement_option_description(game, effect.source)
            )
        }
        ReplacementAction::EnterAsCopy { source, .. } => {
            let source_name = game
                .current_name(*source)
                .unwrap_or_else(|| "Unknown object".to_string());
            format!("Enter as a copy of {source_name}")
        }
        _ => replacement_option_description(game, effect.source),
    }
}

fn replacement_effect_related_objects(effect: &ReplacementEffect) -> Vec<crate::ids::ObjectId> {
    match &effect.replacement {
        ReplacementAction::EnterAsCopy {
            source,
            linked_exile_objects,
            ..
        } => std::iter::once(*source)
            .chain(linked_exile_objects.iter().copied())
            .collect(),
        _ => Vec::new(),
    }
}

fn push_enter_as_copy_effects_for_spec(
    game: &GameState,
    entering_object: ObjectId,
    source: ObjectId,
    controller: PlayerId,
    spec: &crate::static_abilities::EnterAsCopyAsEntersSpec,
    copy_choice_effects: &mut Vec<ReplacementEffect>,
) {
    let filter_ctx = game.filter_context_for(controller, Some(source));
    if let Some(affected_filter) = &spec.affected_filter {
        let Some(entering) = game.object(entering_object) else {
            return;
        };
        let mut prospective = entering.clone();
        prospective.zone = Zone::Battlefield;
        if !affected_filter.matches(&prospective, &filter_ctx, game) {
            return;
        }
    } else if source != entering_object {
        return;
    }

    let mut candidates = if spec.copy_source_self {
        vec![source]
    } else if spec.copy_source_enchanted {
        game.object(source)
            .and_then(|obj| obj.attached_to.and_then(|target| target.object_id()))
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        game.objects_in_deterministic_order()
            .into_iter()
            .filter(|candidate| candidate.id != entering_object)
            .filter(|candidate| spec.filter.matches(candidate, &filter_ctx, game))
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>()
    };
    candidates.sort_by_key(|id| id.0);
    candidates.dedup();
    if candidates.is_empty() {
        return;
    }

    let set_base_power_toughness = spec.set_base_power_toughness.or_else(|| {
        spec.set_base_power_toughness_from_self
            .then(|| {
                game.object(entering_object)
                    .and_then(|obj| Some((obj.power()?, obj.toughness()?)))
            })
            .flatten()
    });

    if let Some(linked_pair) = spec.linked_exile_pair {
        if candidates.len() < 2 {
            return;
        }
        if spec.may {
            copy_choice_effects.push(
                ReplacementEffect::with_matcher(
                    entering_object,
                    controller,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::Additionally(Vec::new()),
                )
                .with_priority_override(crate::events::ReplacementPriority::CopyEffect),
            );
        }
        for &copy_candidate in &candidates {
            for &counter_candidate in &candidates {
                if copy_candidate == counter_candidate {
                    continue;
                }
                let counter_count = game
                    .object(counter_candidate)
                    .and_then(|object| object.power())
                    .unwrap_or(0)
                    .max(0) as u32;
                copy_choice_effects.push(
                    ReplacementEffect::with_matcher(
                        entering_object,
                        controller,
                        crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                        ReplacementAction::EnterAsCopy {
                            source: copy_candidate,
                            enters_tapped: spec.enters_tapped_if_chosen,
                            linked_exile_objects: vec![copy_candidate, counter_candidate],
                            additional_counters: vec![(linked_pair.counter_type, counter_count)],
                            name_override: spec.name_override.clone(),
                            added_card_types: spec.added_card_types.clone(),
                            removed_supertypes: spec.removed_supertypes.clone(),
                            added_subtypes: spec.added_subtypes.clone(),
                            added_abilities: spec.added_abilities.clone(),
                            set_base_power_toughness,
                        },
                    )
                    .with_priority_override(crate::events::ReplacementPriority::CopyEffect),
                );
            }
        }
        return;
    }

    if spec.may {
        copy_choice_effects.push(
            ReplacementEffect::with_matcher(
                entering_object,
                controller,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::Additionally(Vec::new()),
            )
            .with_priority_override(crate::events::ReplacementPriority::CopyEffect),
        );
    }

    for candidate in candidates {
        copy_choice_effects.push(
            ReplacementEffect::with_matcher(
                entering_object,
                controller,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::EnterAsCopy {
                    source: candidate,
                    enters_tapped: spec.enters_tapped_if_chosen,
                    linked_exile_objects: Vec::new(),
                    additional_counters: Vec::new(),
                    name_override: spec.name_override.clone(),
                    added_card_types: spec.added_card_types.clone(),
                    removed_supertypes: spec.removed_supertypes.clone(),
                    added_subtypes: spec.added_subtypes.clone(),
                    added_abilities: spec.added_abilities.clone(),
                    set_base_power_toughness,
                },
            )
            .with_priority_override(crate::events::ReplacementPriority::CopyEffect),
        );
    }
}

/// Priority order for replacement effects per Rule 616.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplacementPriority {
    /// 616.1a: True self-replacement effects per CR 614.15
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
pub fn process_trait_event(game: &mut GameState, event: Event) -> TraitEventResult {
    let event = game.ensure_event_provenance(event);
    let mut state = TraitEventProcessingState::default();
    process_event_direct(game, event, &mut state, &[], None)
}

/// Process an event through the replacement effect system with additional effects.
///
/// This variant allows passing in additional replacement effects for the event,
/// which is needed for object-local ETB replacement effects that apply before the
/// object fully enters the battlefield.
pub fn process_trait_event_with_additional_effects(
    game: &mut GameState,
    event: Event,
    additional_effects: &[ReplacementEffect],
) -> TraitEventResult {
    let event = game.ensure_event_provenance(event);
    let mut state = TraitEventProcessingState::default();
    let mut additional_effects = additional_effects.to_vec();
    assign_ephemeral_effect_ids(&mut additional_effects, u64::MAX / 2);
    process_event_direct(game, event, &mut state, &additional_effects, None)
}

/// Process an event while treating selected replacement effects as already applied.
///
/// This is for nested events created by replacement effects. CR 614.5 prevents a
/// replacement effect from applying again to the event it replaced or any event
/// created by that replacement path, while unrelated replacement effects must
/// still be considered normally.
pub fn process_trait_event_with_dm_and_applied_effects(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    applied_effects: &std::collections::HashSet<ReplacementEffectId>,
    applied_effect_keys: &std::collections::HashSet<ReplacementEffectKey>,
) -> TraitEventResult {
    process_with_dm_and_additional_effects_and_applied(
        game,
        event,
        dm,
        &[],
        applied_effects,
        applied_effect_keys,
    )
}

/// State for tracking trait-based event processing.
#[derive(Debug, Clone, Default)]
pub struct TraitEventProcessingState {
    /// Replacement effects that have already been applied to this event.
    pub applied_effects: std::collections::HashSet<ReplacementEffectId>,
    /// Stable replacement identities already applied to this event.
    pub applied_effect_keys: std::collections::HashSet<ReplacementEffectKey>,
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

    /// Mark an effect as applied by both transient ID and stable key.
    pub fn mark_applied_effect(&mut self, effect: &ReplacementEffect) {
        self.mark_applied(effect.id);
        self.applied_effect_keys.insert(effect.application_key());
    }

    /// Check if an effect was already applied.
    pub fn was_applied(&self, id: ReplacementEffectId) -> bool {
        self.applied_effects.contains(&id)
    }

    /// Check if an effect was already applied, including regenerated static effects.
    pub fn was_applied_effect(&self, effect: &ReplacementEffect) -> bool {
        self.was_applied(effect.id) || self.applied_effect_keys.contains(&effect.application_key())
    }

    /// Increment iteration count.
    pub fn increment(&mut self) {
        self.iteration_count += 1;
    }
}

/// Process an event directly using trait-based matchers.
fn process_event_direct(
    game: &mut GameState,
    event: Event,
    state: &mut TraitEventProcessingState,
    additional_effects: &[ReplacementEffect],
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> TraitEventResult {
    // Safety check for infinite loops
    if state.exceeded_max_iterations() {
        return TraitEventResult::Proceed(event);
    }
    state.increment();

    // Find all applicable replacement effects using trait-based matchers
    let applicable = find_applicable_trait_replacements(
        game,
        &event,
        state,
        additional_effects,
        event_source_snapshot,
    );

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

    // Multiple equivalent one-shot replacement effects from the same source are
    // redundant for the current event. Regeneration can create this shape when
    // a creature has more than one shield: choosing either shield has the same
    // event outcome, and exactly one shield should be consumed.
    if tied_replacements_are_duplicate_regeneration_shields(game, &at_highest) {
        let chosen_effect = at_highest[0].clone();
        let effect_id = chosen_effect.id;
        let result = apply_trait_replacement(game, event.clone(), &chosen_effect);
        state.mark_applied_effect(&chosen_effect);
        consume_one_shot_if_applied(game, effect_id, &result);
        return match result {
            TraitApplyResult::Modified(modified_event) => process_event_direct(
                game,
                modified_event,
                state,
                additional_effects,
                event_source_snapshot,
            ),
            TraitApplyResult::Prevented => TraitEventResult::Prevented,
            TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced {
                effects,
                effect_id,
                replacement: chosen_effect.replacement.clone(),
                source: chosen_effect.source,
                controller: chosen_effect.controller,
            },
            TraitApplyResult::Unchanged(event) => TraitEventResult::Proceed(event),
            TraitApplyResult::NeedsInteraction {
                decision_ctx,
                redirect_zone,
                effect_id,
                object_id,
                filter,
                destinations,
            } => TraitEventResult::NeedsInteraction {
                decision_ctx,
                redirect_zone,
                effect_id,
                object_id,
                event: Box::new(event),
                filter,
                life_cost: match &chosen_effect.replacement {
                    ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } => {
                        Some(*life_cost)
                    }
                    _ => None,
                },
                destinations,
            },
        };
    }

    // When multiple replacement effects are tied at the highest priority,
    // the affected player/controller chooses which one to apply next.
    if at_highest.len() > 1 {
        let affected_player = event.inner().affected_player(game);
        let effect_ids: Vec<_> = at_highest.iter().map(|e| e.id).collect();

        return TraitEventResult::NeedsChoice {
            player: affected_player,
            applicable_effects: effect_ids,
            event: Box::new(event),
            applied_effects: state.applied_effects.clone(),
            applied_effect_keys: state.applied_effect_keys.clone(),
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
    state.mark_applied_effect(&chosen_effect);
    consume_one_shot_if_applied(game, effect_id, &result);

    match result {
        TraitApplyResult::Modified(modified_event) => process_event_direct(
            game,
            modified_event,
            state,
            additional_effects,
            event_source_snapshot,
        ),
        TraitApplyResult::Prevented => TraitEventResult::Prevented,
        TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced {
            effects,
            effect_id,
            replacement: chosen_effect.replacement.clone(),
            source: chosen_effect.source,
            controller: chosen_effect.controller,
        },
        TraitApplyResult::Unchanged(event) => TraitEventResult::Proceed(event),
        TraitApplyResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            filter,
            destinations,
        } => TraitEventResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            event: Box::new(event),
            filter,
            life_cost,
            destinations,
        },
    }
}

fn tied_replacements_are_duplicate_regeneration_shields(
    game: &GameState,
    effects: &[ReplacementEffect],
) -> bool {
    let Some(first) = effects.first() else {
        return false;
    };
    // NOTE: `ReplacementAction::Instead` payloads can never compare equal via
    // `==` (runtime `Effect` deliberately implements `PartialEq` as
    // always-false), so identical shields are recognized by their debug
    // representation. Shields with extra follow-up effects (Debt of Loyalty)
    // render differently and are intentionally NOT deduplicated.
    let same_instead_payload =
        |effect: &ReplacementEffect| match (&effect.replacement, &first.replacement) {
            (ReplacementAction::Instead(a), ReplacementAction::Instead(b)) => {
                a.len() == b.len() && format!("{a:?}") == format!("{b:?}")
            }
            _ => false,
        };
    effects.len() > 1
        && effects.iter().all(|effect| {
            game.effect_store.replacement_effects.is_one_shot(effect.id)
                && effect.source == first.source
                && effect.controller == first.controller
                && effect.priority_override == first.priority_override
                && same_instead_payload(effect)
                && effect
                    .matcher
                    .as_ref()
                    .is_some_and(|matcher| matcher.display() == "Regeneration shield")
        })
}

fn consume_one_shot_if_applied(
    game: &mut GameState,
    effect_id: ReplacementEffectId,
    result: &TraitApplyResult,
) {
    if !matches!(result, TraitApplyResult::Unchanged(_)) {
        game.effect_store
            .replacement_effects
            .mark_effect_used(effect_id);
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
    Options(Vec<usize>),
}

fn continue_interactive_replacement(
    game: &mut GameState,
    response: &InteractiveReplacementResponse,
    object_id: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    filter: Option<&crate::target::ObjectFilter>,
    redirect_zone: Zone,
    life_cost: Option<u32>,
    destinations: Option<&[Zone]>,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
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
            provenance,
            decision_maker,
        );
    }

    // Handle pay-life-or-enter-tapped (shock land pattern)
    if let Some(cost) = life_cost {
        return handle_pay_life_or_enter_tapped(game, response, controller, cost);
    }

    if let Some(destinations) = destinations {
        let selected_zone = match response {
            InteractiveReplacementResponse::Options(selected) => selected
                .first()
                .and_then(|idx| destinations.get(*idx))
                .copied()
                .unwrap_or(redirect_zone),
            _ => redirect_zone,
        };
        return InteractiveReplacementResult::redirected(selected_zone);
    }

    // Fallback: redirect
    InteractiveReplacementResult::redirected(redirect_zone)
}

/// Handle a discard-or-redirect interactive replacement.
fn handle_discard_or_redirect(
    game: &mut GameState,
    response: &InteractiveReplacementResponse,
    object_id: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    filter: &crate::target::ObjectFilter,
    redirect_zone: Zone,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
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
                        crate::events::cause::EventCause::from_effect(object_id, controller),
                        true,
                        provenance,
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
        InteractiveReplacementResponse::Decline
        | InteractiveReplacementResponse::Accept
        | InteractiveReplacementResponse::Options(_) => {
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
            let can_pay = game.can_pay_life(controller, life_cost);

            if can_pay {
                // Deduct life
                game.pay_life(controller, life_cost);
                // Permanent enters untapped
                InteractiveReplacementResult::enters_battlefield()
            } else {
                // Can't pay anymore (life changed since decision was made)
                // Permanent enters tapped
                InteractiveReplacementResult::enters_tapped()
            }
        }
        InteractiveReplacementResponse::Decline
        | InteractiveReplacementResponse::Objects(_)
        | InteractiveReplacementResponse::Options(_) => {
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
        Zone::Library | Zone::Hand | Zone::OutsideGame => false,
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
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
) -> DiscardResult {
    use crate::events::cards::DiscardEvent;
    use crate::events::traits::downcast_event;

    game.update_replacement_effects();

    // Create a discard event with the cause
    let discard_event = DiscardEvent::with_cause(card_id, player, cause.clone());
    let event = Event::new_with_provenance(discard_event, provenance);

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
                    move_to_top_of_library(
                        game,
                        card_id,
                        player,
                        discard.cause.clone(),
                        decision_maker,
                    )
                } else {
                    game.move_object(card_id, destination, discard.cause.clone())
                };

                // Mark as madness_exiled if card went to exile via Madness
                if has_madness
                    && destination == Zone::Exile
                    && let Some(id) = new_id
                {
                    game.set_madness_exiled(id);
                    if let Some(result) =
                        resolve_madness_discard(game, id, player, provenance, decision_maker)
                    {
                        return result;
                    }
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
            destinations,
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
                        destinations
                            .as_deref()
                            .and_then(|zones| zones.get(idx))
                            .copied()
                            .unwrap_or(redirect_zone)
                    } else {
                        redirect_zone
                    };

                    let new_id = if chosen_zone == Zone::Library {
                        move_to_top_of_library(game, card_id, player, cause.clone(), decision_maker)
                    } else {
                        game.move_object(card_id, chosen_zone, cause.clone())
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
                    let new_id = game.move_object(card_id, redirect_zone, cause);
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

fn resolve_madness_discard(
    game: &mut GameState,
    exiled_id: crate::ids::ObjectId,
    player: crate::ids::PlayerId,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
) -> Option<DiscardResult> {
    let madness_cost = game.object(exiled_id).and_then(|obj| {
        obj.alternative_casts
            .iter()
            .find_map(|method| match method {
                crate::alternative_cast::AlternativeCastingMethod::Madness { cost } => {
                    Some(cost.clone())
                }
                _ => None,
            })
    })?;

    let cast_with_madness = crate::decisions::make_decision(
        game,
        decision_maker,
        player,
        Some(exiled_id),
        crate::decisions::specs::MadnessSpec::new(exiled_id, madness_cost.clone()),
    );

    if !cast_with_madness {
        game.clear_madness_exiled(exiled_id);
        let new_id = game.move_object(
            exiled_id,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_game_rule(),
        );
        return Some(DiscardResult {
            new_id,
            final_zone: Zone::Graveyard,
            type_verifiable: true,
            prevented: false,
        });
    }

    if !pay_madness_mana_cost(game, player, exiled_id, &madness_cost, decision_maker) {
        game.clear_madness_exiled(exiled_id);
        let new_id = game.move_object(
            exiled_id,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_game_rule(),
        );
        return Some(DiscardResult {
            new_id,
            final_zone: Zone::Graveyard,
            type_verifiable: true,
            prevented: false,
        });
    }

    let stack_id = game.move_object(
        exiled_id,
        Zone::Stack,
        crate::events::cause::EventCause::from_effect(exiled_id, player),
    )?;
    game.clear_madness_exiled(stack_id);

    let mut entry =
        crate::game_state::StackEntry::new(stack_id, player).with_provenance(provenance);
    if let Some(program) = game
        .object(stack_id)
        .and_then(|obj| obj.spell_effect_owned())
    {
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
            game,
            &program,
            player,
            Some(stack_id),
            None,
        );
        if !requirements.is_empty() {
            let context = game
                .object(stack_id)
                .map(|obj| obj.name.to_string())
                .unwrap_or_else(|| "madness spell".to_string());
            let target_ctx = crate::decisions::context::TargetsContext::new(
                player,
                stack_id,
                context,
                requirements
                    .iter()
                    .map(
                        |requirement| crate::decisions::context::TargetRequirementContext {
                            description: requirement.description.clone(),
                            legal_targets: requirement.legal_targets.clone(),
                            legal_target_sets: requirement.legal_target_sets.clone(),
                            min_targets: requirement.min_targets,
                            max_targets: requirement.max_targets,
                        },
                    )
                    .collect(),
            );
            let proposed = decision_maker.decide_targets(game, &target_ctx);
            let targets = crate::targeting::normalize_targets_for_requirements(
                &target_ctx.requirements,
                proposed,
            )
            .unwrap_or_default();
            let target_assignments =
                crate::targeting::assigned_target_ranges(&target_ctx.requirements, &targets)
                    .map(|ranges| {
                        requirements
                            .iter()
                            .zip(ranges)
                            .map(|(requirement, range)| crate::game_state::TargetAssignment {
                                spec: requirement.spec.clone(),
                                range,
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
            entry = entry
                .with_targets(targets)
                .with_target_assignments(target_assignments);
        }
    }

    game.push_to_stack(entry);
    let _ = crate::game_loop::resolve_stack_entry_with(game, decision_maker);
    Some(DiscardResult {
        new_id: None,
        final_zone: Zone::Graveyard,
        type_verifiable: true,
        prevented: false,
    })
}

/// Move a card to the top of the owner's library.
fn move_to_top_of_library(
    game: &mut GameState,
    card_id: crate::ids::ObjectId,
    owner: crate::ids::PlayerId,
    cause: crate::events::cause::EventCause,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
) -> Option<crate::ids::ObjectId> {
    // Get the new ID from the zone change
    let (new_id, final_zone) =
        game.move_object_with_commander_options(card_id, Zone::Library, cause, decision_maker)?;
    if final_zone != Zone::Library {
        return Some(new_id);
    }

    // The card should now be at the end of the library array (which represents the top)
    // move_object already handles this correctly for Zone::Library

    game.move_library_card_to_top(owner, new_id, "replacement moved card to top of library");

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
        /// Destination options for InteractiveChooseDestination.
        destinations: Option<Vec<Zone>>,
    },
}

/// Find all replacement effects that apply to a trait-based event.
fn find_applicable_trait_replacements(
    game: &GameState,
    event: &Event,
    state: &TraitEventProcessingState,
    additional_effects: &[ReplacementEffect],
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Vec<(ReplacementEffect, ReplacementPriority)> {
    let mut applicable = Vec::new();

    // Check registered replacement effects in the game
    for effect in game.effect_store.replacement_effects.effects() {
        // Skip if already applied (Rule 614.5)
        if state.was_applied_effect(effect) {
            continue;
        }

        // Check if effect matches using trait-based matcher
        if let Some(priority) =
            trait_effect_matches_event(game, effect, event, event_source_snapshot)
        {
            applicable.push((effect.clone(), priority));
        }
    }

    // Check additional ephemeral effects for this event.
    for effect in additional_effects {
        // Skip if already applied (Rule 614.5)
        if state.was_applied_effect(effect) {
            continue;
        }

        // Check if effect matches
        if let Some(priority) =
            trait_effect_matches_event(game, effect, event, event_source_snapshot)
        {
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
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Option<ReplacementPriority> {
    use crate::events::ReplacementPriority as TraitPriority;

    // All effects should have trait-based matchers
    let matcher = effect.matcher.as_ref()?;

    let ctx = EventContext::for_replacement_effect(effect.controller, effect.source, game)
        .with_event_source_snapshot(event_source_snapshot);
    if !matcher.matches_event(event.inner(), &ctx) {
        return None;
    }

    let trait_priority = effect
        .priority_override
        .unwrap_or_else(|| matcher.priority());
    let priority = match trait_priority {
        TraitPriority::SelfReplacement => ReplacementPriority::SelfReplacement,
        TraitPriority::ControlChanging => ReplacementPriority::ControlChanging,
        TraitPriority::CopyEffect => ReplacementPriority::CopyEffect,
        TraitPriority::BackFace => ReplacementPriority::BackFace,
        TraitPriority::Other => ReplacementPriority::Other,
    };

    Some(priority)
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
        replacement: ReplacementAction,
        source: crate::ids::ObjectId,
        controller: PlayerId,
    },
    /// Multiple replacement effects apply - player must choose.
    NeedsChoice {
        player: PlayerId,
        applicable_effects: Vec<crate::replacement::ReplacementEffectId>,
        event: Box<Event>,
        applied_effects: std::collections::HashSet<crate::replacement::ReplacementEffectId>,
        applied_effect_keys: std::collections::HashSet<crate::replacement::ReplacementEffectKey>,
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
        /// Destination options for InteractiveChooseDestination.
        destinations: Option<Vec<Zone>>,
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
    dm: &mut dyn DecisionMaker,
) -> DestroyOutcome {
    process_destroy_inner(game, permanent, source, dm, None)
}

pub(crate) fn process_destroy_with_snapshot(
    game: &mut GameState,
    permanent: crate::ids::ObjectId,
    source: Option<crate::ids::ObjectId>,
    dm: &mut dyn DecisionMaker,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> DestroyOutcome {
    process_destroy_inner(game, permanent, source, dm, snapshot)
}

fn process_destroy_inner(
    game: &mut GameState,
    permanent: crate::ids::ObjectId,
    source: Option<crate::ids::ObjectId>,
    dm: &mut dyn DecisionMaker,
    lki_snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> DestroyOutcome {
    use crate::effects::{ExecutionContext, execute_effect};

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
    let controller = game.controller_of(obj);
    let cause = if let Some(source_id) = source {
        let source_controller = game
            .object(source_id)
            .map(|source_obj| game.controller_of(source_obj))
            .unwrap_or(controller);
        crate::events::cause::EventCause::from_effect(source_id, source_controller)
    } else {
        crate::events::cause::EventCause::from_sba()
    };

    // Create the destroy event using the trait-based system
    let event = Event::destroy(permanent, source);

    // Process through replacement effects with NeedsChoice handling
    let result = process_with_dm(game, event.clone(), dm);

    match result {
        TraitEventResult::Prevented => EventOutcome::Prevented,

        TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {
            // Destruction proceeds - now process the zone change
            let zone_result = process_zone_change_with_snapshot(
                game,
                permanent,
                Zone::Battlefield,
                Zone::Graveyard,
                cause.clone(),
                dm, // Reuse the decision maker for zone change choices
                lki_snapshot.clone(),
            );

            match zone_result {
                EventOutcome::Prevented => EventOutcome::Prevented,
                EventOutcome::Proceed(final_zone) => {
                    if final_zone == Zone::Graveyard
                        && let Some(stable_id) = game.object(permanent).map(|obj| obj.stable_id)
                    {
                        game.record_ui_battlefield_transition(
                            UiBattlefieldTransitionKind::Destroyed,
                            stable_id,
                        );
                    }
                    let moved_id = game.move_object_with_snapshot(
                        permanent,
                        final_zone,
                        cause.clone(),
                        lki_snapshot,
                    );
                    if final_zone == Zone::Graveyard {
                        let mut moved_ids = game.take_zone_change_results(permanent);
                        if moved_ids.is_empty()
                            && let Some(id) = moved_id
                        {
                            moved_ids.push(id);
                        }
                        let mut applied = crate::effects::zones::AppliedZoneChange {
                            final_zone,
                            new_object_id: moved_ids.first().copied(),
                            new_object_ids: moved_ids,
                        };
                        crate::effects::zones::maybe_prompt_for_split_result_order(
                            game,
                            dm,
                            final_zone,
                            &cause,
                            &mut applied,
                        );
                        if !applied.new_object_ids.is_empty() {
                            game.record_zone_change_results(permanent, applied.new_object_ids);
                        }
                    }
                    EventOutcome::Proceed(final_zone)
                }
                EventOutcome::Replaced => EventOutcome::Replaced,
                EventOutcome::NotApplicable => EventOutcome::NotApplicable,
            }
        }

        TraitEventResult::Replaced {
            effects,
            effect_id,
            source: replacement_source,
            controller: replacement_controller,
            ..
        } => {
            // Destruction was replaced with other effects.
            // Execute the replacement effects with a minimal context.
            // The effects typically use ChooseSpec::SpecificObject, so they're self-contained.

            // Consume one-shot effects (like regeneration shields)
            game.effect_store
                .replacement_effects
                .mark_effect_used(effect_id);

            let effect_source = source.unwrap_or(replacement_source);
            let mut ctx = ExecutionContext::new(effect_source, replacement_controller, dm);

            for effect in effects {
                // Ignore errors from effect execution - the replacement still happened
                let _ = execute_effect(game, &effect, &mut ctx);
            }

            EventOutcome::Replaced
        }

        TraitEventResult::NeedsChoice {
            applicable_effects,
            event: boxed_event,
            ..
        } => {
            // The decision maker deferred the tie-break. Returning Prevented
            // here while damage stays marked would wedge the SBA loop, so
            // apply the first tied effect (rule-equivalent for the common
            // duplicate-shield case) instead of refusing the destruction.
            let chosen = applicable_effects.first().copied().and_then(|effect_id| {
                game.effect_store
                    .replacement_effects
                    .get_effect(effect_id)
                    .cloned()
                    .map(|effect| (effect_id, effect))
            });
            let Some((effect_id, chosen_effect)) = chosen else {
                return EventOutcome::Prevented;
            };
            let apply_result = apply_trait_replacement(game, *boxed_event, &chosen_effect);
            consume_one_shot_if_applied(game, effect_id, &apply_result);
            match apply_result {
                TraitApplyResult::Replaced(effects) => {
                    let effect_source = source.unwrap_or(chosen_effect.source);
                    let mut ctx =
                        ExecutionContext::new(effect_source, chosen_effect.controller, dm);
                    for effect in effects {
                        let _ = execute_effect(game, &effect, &mut ctx);
                    }
                    EventOutcome::Replaced
                }
                TraitApplyResult::Prevented => EventOutcome::Prevented,
                // Modified/Unchanged destroy events proceed as plain destruction.
                TraitApplyResult::Modified(_) | TraitApplyResult::Unchanged(_) => {
                    let moved = game.move_object_with_snapshot(
                        permanent,
                        Zone::Graveyard,
                        cause.clone(),
                        None,
                    );
                    if moved.is_some() {
                        EventOutcome::Proceed(Zone::Graveyard)
                    } else {
                        EventOutcome::Prevented
                    }
                }
                TraitApplyResult::NeedsInteraction { .. } => EventOutcome::Prevented,
            }
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
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
) -> ZoneChangeOutcome {
    process_zone_change_with_additional_effects(game, object, from, to, cause, dm, &[])
}

pub(crate) fn process_zone_change_with_snapshot(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> ZoneChangeOutcome {
    process_zone_change_inner(game, object, from, to, cause, dm, &[], snapshot)
}

pub fn process_zone_change_with_additional_effects(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
) -> ZoneChangeOutcome {
    process_zone_change_inner(game, object, from, to, cause, dm, additional_effects, None)
}

fn process_zone_change_inner(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    lki_snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> ZoneChangeOutcome {
    use crate::events::{ZoneChangeEvent, downcast_event};

    game.update_replacement_effects();

    // Finality counter rule text: "If a creature with a finality counter on it would die, exile it instead."
    // Apply this as a baseline destination rewrite for battlefield->graveyard moves.
    let mut requested_to = game.resolve_commander_move_destination(object, to, dm);
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

    let snapshot = lki_snapshot.or_else(|| {
        game.object(object).map(|o| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(o, game)
        })
    });

    let event = Event::zone_change(object, from, requested_to, cause.clone(), snapshot.clone());
    let mut additional_effects = additional_effects.to_vec();
    assign_ephemeral_effect_ids(&mut additional_effects, (u64::MAX / 2).saturating_add(1024));
    let result =
        process_with_dm_and_additional_effects(game, event.clone(), dm, &additional_effects);

    match result {
        TraitEventResult::Prevented => EventOutcome::Prevented,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner()) {
                EventOutcome::Proceed(zone_change.to)
            } else {
                EventOutcome::Proceed(requested_to)
            }
        }
        TraitEventResult::Replaced {
            effects,
            effect_id,
            replacement,
            source: replacement_source,
            controller: replacement_controller,
        } => {
            game.effect_store
                .replacement_effects
                .mark_effect_used(effect_id);
            if matches!(
                replacement,
                crate::replacement::ReplacementAction::MoveToZoneWithCounters { .. }
                    | crate::replacement::ReplacementAction::ExileWithSourceLink
                    | crate::replacement::ReplacementAction::ExileWithSourceLinkThen(_)
                    | crate::replacement::ReplacementAction::ExileWithSourceLinkCountersThen { .. }
            ) {
                let destination = match &replacement {
                    crate::replacement::ReplacementAction::MoveToZoneWithCounters {
                        zone, ..
                    } => *zone,
                    _ => Zone::Exile,
                };
                let counters = match &replacement {
                    crate::replacement::ReplacementAction::MoveToZoneWithCounters {
                        counters,
                        ..
                    } => counters.as_slice(),
                    crate::replacement::ReplacementAction::ExileWithSourceLinkCountersThen {
                        counters,
                        ..
                    } => counters.as_slice(),
                    _ => &[],
                };
                let should_link_to_source = matches!(
                    replacement,
                    crate::replacement::ReplacementAction::ExileWithSourceLink
                        | crate::replacement::ReplacementAction::ExileWithSourceLinkThen(_)
                        | crate::replacement::ReplacementAction::ExileWithSourceLinkCountersThen { .. }
                );
                if let Some(new_id) = game.move_object(object, destination, cause.clone()) {
                    for (counter_type, count) in counters {
                        if let Some(event) = game.add_counters_with_source(
                            new_id,
                            *counter_type,
                            *count,
                            Some(replacement_source),
                            Some(replacement_controller),
                        ) {
                            game.queue_trigger_event(event.provenance(), event);
                        }
                    }
                    if should_link_to_source {
                        game.add_exiled_with_source_link(replacement_source, new_id);
                    }
                    game.record_zone_change_results(object, vec![new_id]);
                }
                if !effects.is_empty() {
                    let mut ctx = crate::effects::ExecutionContext::new(
                        replacement_source,
                        replacement_controller,
                        dm,
                    );
                    for effect in effects {
                        if let Ok(outcome) = crate::effects::execute_effect(game, &effect, &mut ctx)
                        {
                            for trigger_event in outcome.events {
                                game.queue_trigger_event(trigger_event.provenance(), trigger_event);
                            }
                        }
                    }
                }
                return EventOutcome::Replaced;
            }
            let mut ctx = crate::effects::ExecutionContext::new(
                replacement_source,
                replacement_controller,
                dm,
            );
            for effect in effects {
                let _ = crate::effects::execute_effect(game, &effect, &mut ctx);
            }
            EventOutcome::Replaced
        }
        TraitEventResult::NeedsChoice { .. } => {
            debug_assert!(
                false,
                "process_with_dm returned NeedsChoice for zone change event"
            );
            EventOutcome::Prevented
        }
        TraitEventResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            destinations: Some(destinations),
            ..
        } => {
            let chosen_zone = match decision_ctx {
                crate::decisions::context::DecisionContext::SelectOptions(ctx) => {
                    let selected = dm.decide_options(game, &ctx);
                    selected
                        .first()
                        .and_then(|idx| destinations.get(*idx))
                        .copied()
                        .unwrap_or(redirect_zone)
                }
                _ => redirect_zone,
            };
            EventOutcome::Proceed(chosen_zone)
        }
        TraitEventResult::NeedsInteraction { .. } => {
            debug_assert!(
                false,
                "non-destination interactive replacement unexpectedly matched zone change event"
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
    dm: &mut dyn DecisionMaker,
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
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
) -> TraitEventResult {
    process_with_dm_and_additional_effects(game, event, dm, &[])
}

fn process_with_dm_and_additional_effects(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
) -> TraitEventResult {
    process_with_dm_and_additional_effects_and_applied(
        game,
        event,
        dm,
        additional_effects,
        &std::collections::HashSet::new(),
        &std::collections::HashSet::new(),
    )
}

fn process_with_dm_and_additional_effects_and_applied(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
    applied_effects: &std::collections::HashSet<ReplacementEffectId>,
    applied_effect_keys: &std::collections::HashSet<ReplacementEffectKey>,
) -> TraitEventResult {
    use crate::decisions::{
        make_decision,
        specs::{ReplacementOption, ReplacementSpec},
    };

    let mut current_event = game.ensure_event_provenance(event);
    let mut state = TraitEventProcessingState::default();
    state
        .applied_effects
        .extend(applied_effects.iter().copied());
    state
        .applied_effect_keys
        .extend(applied_effect_keys.iter().cloned());

    loop {
        let result = process_event_direct(
            game,
            current_event.clone(),
            &mut state,
            additional_effects,
            None,
        );

        match result {
            TraitEventResult::NeedsChoice {
                player,
                applicable_effects,
                event: boxed_event,
                ..
            } => {
                // Determine which effect to apply
                let chosen_index = {
                    // Build options for the decision
                    let options: Vec<ReplacementOption> = applicable_effects
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, &id)| {
                            game.effect_store
                                .replacement_effects
                                .get_effect(id)
                                .map(|e| {
                                    ReplacementOption::new(
                                        idx,
                                        e.source,
                                        replacement_effect_choice_description(game, e),
                                    )
                                    .with_related_objects(replacement_effect_related_objects(e))
                                })
                        })
                        .collect();

                    let spec = ReplacementSpec::new(options);
                    make_decision(game, dm, player, None, spec)
                };
                if dm.awaiting_choice() {
                    return TraitEventResult::NeedsChoice {
                        player,
                        applicable_effects,
                        event: boxed_event,
                        applied_effects: state.applied_effects.clone(),
                        applied_effect_keys: state.applied_effect_keys.clone(),
                    };
                }

                // Apply the chosen effect immediately, then continue processing
                let chosen_id = applicable_effects
                    .get(chosen_index)
                    .copied()
                    .or_else(|| applicable_effects.first().copied());

                let Some(effect_id) = chosen_id else {
                    return TraitEventResult::Proceed(*boxed_event);
                };

                let Some(chosen_effect) = game
                    .effect_store
                    .replacement_effects
                    .get_effect(effect_id)
                    .cloned()
                else {
                    // Effect disappeared (e.g., source left battlefield). Continue with event.
                    state.mark_applied(effect_id);
                    current_event = *boxed_event;
                    continue;
                };

                state.mark_applied_effect(&chosen_effect);

                let apply_result = apply_trait_replacement(game, *boxed_event, &chosen_effect);
                consume_one_shot_if_applied(game, effect_id, &apply_result);
                match apply_result {
                    TraitApplyResult::Modified(modified_event) => {
                        current_event = modified_event;
                    }
                    TraitApplyResult::Prevented => return TraitEventResult::Prevented,
                    TraitApplyResult::Replaced(effects) => {
                        return TraitEventResult::Replaced {
                            effects,
                            effect_id,
                            replacement: chosen_effect.replacement.clone(),
                            source: chosen_effect.source,
                            controller: chosen_effect.controller,
                        };
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
                        destinations,
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
                            destinations,
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
    additional_effects: &[ReplacementEffect],
    id: ReplacementEffectId,
) -> Option<ReplacementEffect> {
    game.effect_store
        .replacement_effects
        .get_effect(id)
        .cloned()
        .or_else(|| additional_effects.iter().find(|e| e.id == id).cloned())
}

fn assign_ephemeral_effect_ids(effects: &mut [ReplacementEffect], id_base: u64) {
    for (idx, effect) in effects.iter_mut().enumerate() {
        effect.id = ReplacementEffectId(id_base.saturating_add(idx as u64));
    }
}

fn copied_object_etb_replacement_effects(
    game: &GameState,
    object: crate::ids::ObjectId,
    event: &Event,
    id_base: u64,
) -> Vec<ReplacementEffect> {
    let Some(etb) =
        crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
    else {
        return Vec::new();
    };
    let Some(copy_source_id) = etb.enters_as_copy_of else {
        return Vec::new();
    };
    let Some(controller) = game.object(object).map(|obj| game.controller_of(obj)) else {
        return Vec::new();
    };

    let mut copied_abilities = game
        .object(copy_source_id)
        .map(|source| source.abilities_vec())
        .unwrap_or_default();
    copied_abilities.extend(etb.added_abilities.clone());

    let mut effects = Vec::new();
    for ability in copied_abilities {
        if let crate::ability::AbilityKind::Static(static_ability) = ability.kind
            && let Some(effect) = static_ability.generate_replacement_effect(object, controller)
        {
            effects.push(effect);
        }
    }
    assign_ephemeral_effect_ids(&mut effects, id_base);
    effects
}

/// Result of processing an ETB (Enter the Battlefield) event.
#[derive(Debug, Clone, Default)]
pub struct EtbEventResult {
    /// Whether the permanent enters tapped
    pub enters_tapped: bool,
    /// Counters the permanent enters with (counter_type, count)
    pub enters_with_counters: Vec<(CounterType, u32)>,
    /// Objects exiled and linked to the entering permanent by an as-enters choice.
    pub linked_exile_with_entering: Vec<crate::ids::ObjectId>,
    /// Whether the ETB was prevented (e.g., creature entering from graveyard replaced with exile)
    pub prevented: bool,
    /// If zone was changed, the new destination
    pub new_destination: Option<Zone>,
    /// If set, the object enters as a copy of this source object.
    pub enters_as_copy_of: Option<crate::ids::ObjectId>,
    pub copy_name_override: Option<String>,
    /// Additional card types granted by an ETB copy choice.
    pub added_card_types: Vec<crate::types::CardType>,
    /// Supertypes removed by an ETB copy choice.
    pub removed_supertypes: Vec<crate::types::Supertype>,
    /// Additional subtypes granted by an ETB copy choice.
    pub added_subtypes: Vec<crate::types::Subtype>,
    /// Additional abilities granted by an ETB copy choice.
    pub added_abilities: Vec<crate::ability::Ability>,
    /// Base power/toughness set as the object enters.
    pub set_base_power_toughness: Option<(i32, i32)>,
    /// If set, the controller to use as the object enters.
    pub controller_override: Option<crate::ids::PlayerId>,
    /// Keyword payment labels set by as-enters replacements.
    pub paid_labels: Vec<String>,
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
    cause: crate::events::cause::EventCause,
) -> (u32, bool) {
    let processed = process_damage_assignments_with_event_with_source_snapshot(
        game, source, target, amount, is_combat, cause, None,
    );
    let original_target_damage = processed
        .assignments
        .iter()
        .filter(|assignment| assignment.target == target)
        .map(|assignment| assignment.amount)
        .sum();
    (original_target_damage, processed.replacement_prevented)
}

/// A final damage assignment after replacement and prevention effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessedDamageAssignment {
    pub target: DamageTarget,
    pub amount: u32,
}

/// Final result of processing damage through replacement and prevention effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedDamageResult {
    pub assignments: Vec<ProcessedDamageAssignment>,
    pub replacement_prevented: bool,
}

/// Process damage and return all final assignments after replacement/prevention.
pub fn process_damage_assignments_with_event(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    cause: crate::events::cause::EventCause,
) -> ProcessedDamageResult {
    process_damage_assignments_with_event_with_source_snapshot(
        game, source, target, amount, is_combat, cause, None,
    )
}

/// Process a damage event using the Event type, with optional source LKI.
///
/// When `source_snapshot` is provided and the source object is no longer present
/// in game state, source-dependent checks (like prevention based on source color/type)
/// use the snapshot as last known information.
pub fn process_damage_assignments_with_event_with_source_snapshot(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    cause: crate::events::cause::EventCause,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> ProcessedDamageResult {
    process_damage_assignments_with_event_with_source_snapshot_opts(
        game,
        source,
        target,
        amount,
        is_combat,
        false,
        cause,
        source_snapshot,
    )
}

pub fn process_damage_assignments_with_event_with_source_snapshot_opts(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    unpreventable: bool,
    cause: crate::events::cause::EventCause,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> ProcessedDamageResult {
    use crate::events::{DamageEvent, downcast_event};

    game.update_cant_effects();
    game.update_replacement_effects();

    // Check if damage can be prevented
    let can_prevent = !unpreventable && game.can_prevent_damage();

    // Create the event using the new Event type
    let event = if can_prevent {
        Event::damage(source, target, amount, is_combat, cause.clone())
    } else {
        Event::unpreventable_damage(source, target, amount, is_combat, cause.clone())
    };

    // Process through the trait-based system, retaining event provenance for
    // replacement-generated effect execution.
    let event = game.ensure_event_provenance(event);
    let event_provenance = event.provenance();
    let mut state = TraitEventProcessingState::default();
    let result = process_event_direct(game, event, &mut state, &[], source_snapshot);

    let replaced = match result {
        TraitEventResult::Prevented => {
            return ProcessedDamageResult {
                assignments: Vec::new(),
                replacement_prevented: true,
            };
        }
        TraitEventResult::Replaced {
            effects,
            source: replacement_source,
            controller: replacement_controller,
            ..
        } => {
            let triggering_event = crate::events::RawEvent::new(
                crate::events::DamageEvent::with_cause(
                    source,
                    target,
                    amount,
                    is_combat,
                    cause.clone(),
                ),
                event_provenance,
            );
            let mut dm = crate::decision::AutoPassDecisionMaker;
            let mut exec_ctx = crate::effects::ExecutionContext::new(
                replacement_source,
                replacement_controller,
                &mut dm,
            )
            .with_triggering_event(triggering_event)
            .with_cause(crate::events::cause::EventCause::from_effect(
                replacement_source,
                replacement_controller,
            ))
            .with_provenance(event_provenance);
            match target {
                DamageTarget::Player(player_id) => exec_ctx
                    .targets
                    .push(crate::effects::ResolvedTarget::Player(player_id)),
                DamageTarget::Object(object_id) => exec_ctx
                    .targets
                    .push(crate::effects::ResolvedTarget::Object(object_id)),
            }
            for effect in effects {
                if let Ok(outcome) = crate::effects::execute_effect(game, &effect, &mut exec_ctx) {
                    for trigger_event in outcome.events {
                        game.queue_trigger_event(trigger_event.provenance(), trigger_event);
                    }
                }
            }

            return ProcessedDamageResult {
                assignments: Vec::new(),
                replacement_prevented: true,
            };
        }
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(damage) = downcast_event::<DamageEvent>(e.inner()) {
                damage.clone()
            } else {
                debug_assert!(
                    false,
                    "damage replacement processing returned a non-DamageEvent"
                );
                DamageEvent::with_cause(source, target, amount, is_combat, cause.clone())
            }
        }
        _ => DamageEvent::with_cause(source, target, amount, is_combat, cause.clone()),
    };

    let mut assignments = Vec::new();
    let final_damage = apply_prevention_for_damage_assignment(
        game,
        replaced.target,
        replaced.amount,
        replaced.is_combat,
        replaced.source,
        source_snapshot,
        can_prevent,
        &replaced.cause,
        event_provenance,
    );
    if final_damage > 0 {
        assignments.push(ProcessedDamageAssignment {
            target: replaced.target,
            amount: final_damage,
        });
    }

    if let Some((remainder_target, remainder_amount)) = replaced.remainder
        && remainder_amount > 0
    {
        let remainder = process_damage_assignments_with_event_with_source_snapshot(
            game,
            replaced.source,
            remainder_target,
            remainder_amount,
            replaced.is_combat,
            replaced.cause.clone(),
            source_snapshot,
        );
        assignments.extend(remainder.assignments);
    }

    ProcessedDamageResult {
        assignments,
        replacement_prevented: false,
    }
}

/// Backwards-compatible wrapper that reports only damage assigned to the original target.
pub fn process_damage_with_event_with_source_snapshot(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    cause: crate::events::cause::EventCause,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> (u32, bool) {
    let processed = process_damage_assignments_with_event_with_source_snapshot(
        game,
        source,
        target,
        amount,
        is_combat,
        cause,
        source_snapshot,
    );
    let original_target_damage = processed
        .assignments
        .iter()
        .filter(|assignment| assignment.target == target)
        .map(|assignment| assignment.amount)
        .sum();
    (original_target_damage, processed.replacement_prevented)
}

fn apply_prevention_for_damage_assignment(
    game: &mut GameState,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    can_prevent: bool,
    cause: &crate::events::cause::EventCause,
    provenance: crate::provenance::ProvNodeId,
) -> u32 {
    use crate::events::DamageEvent;

    if amount == 0 {
        return 0;
    }

    let (source_colors, source_card_types) = if let Some(obj) = game.object(source) {
        (obj.colors(), obj.card_types.to_vec())
    } else if let Some(snapshot) = source_snapshot {
        (snapshot.colors, snapshot.card_types.clone())
    } else {
        (crate::color::ColorSet::COLORLESS, Vec::new())
    };

    let source_filter_matches: std::collections::HashMap<_, _> = game
        .effect_store
        .prevention_effects
        .shields()
        .iter()
        .filter_map(|shield| {
            let source_filter = shield.damage_filter.from_source.as_ref()?;
            let filter_ctx = game.filter_context_for(shield.controller, Some(shield.source));
            let matches_current = game
                .object(source)
                .is_some_and(|source_obj| source_filter.matches(source_obj, &filter_ctx, game));
            let matches_lki = source_snapshot
                .filter(|snapshot| snapshot.object_id == source)
                .is_some_and(|snapshot| {
                    source_filter.matches_snapshot(snapshot, &filter_ctx, game)
                });
            Some((shield.id, matches_current || matches_lki))
        })
        .collect();

    let result = match target {
        DamageTarget::Player(player_id) => game
            .effect_store
            .prevention_effects
            .apply_prevention_to_player_with_follow_ups(
                player_id,
                amount,
                is_combat,
                source,
                &source_colors,
                &source_card_types,
                can_prevent,
                &source_filter_matches,
            ),
        DamageTarget::Object(object_id) => {
            let controller = game
                .object(object_id)
                .map(|o| game.controller_of(o))
                .unwrap_or(game.turn.active_player);
            game.effect_store
                .prevention_effects
                .apply_prevention_to_permanent_with_follow_ups(
                    object_id,
                    controller,
                    amount,
                    is_combat,
                    source,
                    &source_colors,
                    &source_card_types,
                    can_prevent,
                    &source_filter_matches,
                )
        }
    };

    if can_prevent && !result.follow_ups.is_empty() {
        for follow_up in result.follow_ups {
            let prevented_event = crate::events::RawEvent::new(
                DamageEvent::with_cause(
                    source,
                    target,
                    follow_up.prevented,
                    is_combat,
                    cause.clone(),
                ),
                provenance,
            );
            let mut dm = crate::decision::AutoPassDecisionMaker;
            let mut exec_ctx = crate::effects::ExecutionContext::new(
                follow_up.source,
                follow_up.controller,
                &mut dm,
            )
            .with_triggering_event(prevented_event.clone())
            .with_cause(crate::events::cause::EventCause::from_effect(
                follow_up.source,
                follow_up.controller,
            ))
            .with_provenance(provenance);
            if follow_up.targets.is_empty() {
                match target {
                    DamageTarget::Player(player_id) => {
                        exec_ctx
                            .targets
                            .push(crate::effects::ResolvedTarget::Player(player_id));
                    }
                    DamageTarget::Object(object_id) => {
                        exec_ctx
                            .targets
                            .push(crate::effects::ResolvedTarget::Object(object_id));
                    }
                }
            } else {
                exec_ctx.targets = follow_up.targets;
                exec_ctx.target_assignments = follow_up.target_assignments;
            }
            for effect in follow_up.effects {
                if let Ok(outcome) = crate::effects::execute_effect(game, &effect, &mut exec_ctx) {
                    for trigger_event in outcome.events {
                        game.queue_trigger_event(trigger_event.provenance(), trigger_event);
                    }
                }
            }
        }
    }

    result.remaining
}

/// Process a life gain event using the new Event type.
///
/// This is the Event-based version of `process_life_gain_event`.
pub fn process_life_gain_with_event(game: &mut GameState, player: PlayerId, amount: u32) -> u32 {
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
    game: &mut GameState,
    creature: crate::ids::ObjectId,
    snapshot: crate::snapshot::ObjectSnapshot,
) -> Option<Zone> {
    use crate::events::{ZoneChangeEvent, downcast_event};

    let event = Event::zone_change(
        creature,
        Zone::Battlefield,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_sba(),
        Some(snapshot),
    );
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
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
) -> Option<Zone> {
    use crate::events::{ZoneChangeEvent, downcast_event};

    let snapshot = game.object(object).map(|o| {
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(o, game)
    });
    let event = Event::zone_change(object, from, to, cause, snapshot);
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
    game: &mut GameState,
    target: crate::ids::ObjectId,
    counter_type: CounterType,
    count: u32,
    cause: crate::events::cause::EventCause,
) -> u32 {
    use crate::events::{PutCountersEvent, downcast_event};

    if !game.can_have_counters_placed(target) {
        return 0;
    }

    let event = Event::put_counters(target, counter_type, count, cause);
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

/// Process a player counter event through replacement effects.
///
/// Returns the final number of counters to give that player.
pub fn process_player_counters_with_event(
    game: &mut GameState,
    target: PlayerId,
    counter_type: CounterType,
    count: u32,
    cause: crate::events::cause::EventCause,
) -> u32 {
    use crate::events::{PutCountersEvent, downcast_event};

    let event = Event::put_player_counters(target, counter_type, count, cause);
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

/// Process a token creation event through replacement effects.
///
/// Returns the final number of tokens to create.
pub fn process_token_creation_with_event(
    game: &mut GameState,
    controller: PlayerId,
    count: u32,
    cause: crate::events::cause::EventCause,
    dm: &mut (impl DecisionMaker + ?Sized),
) -> u32 {
    process_token_creation_for_token_with_event(game, controller, count, None, cause, dm).count
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenCreationReplacementResult {
    pub count: u32,
    pub additional_tokens: Vec<(ironsmith_core::AdditionalTokenKind, u32)>,
}

/// Process a token creation event with known token characteristics.
///
/// Returns the final original-token count and any separately defined tokens added by replacements.
pub fn process_token_creation_for_token_with_event(
    game: &mut GameState,
    controller: PlayerId,
    count: u32,
    token: Option<crate::object::Object>,
    cause: crate::events::cause::EventCause,
    dm: &mut (impl DecisionMaker + ?Sized),
) -> TokenCreationReplacementResult {
    use crate::events::{CreateTokensEvent, downcast_event};

    if count == 0 {
        return TokenCreationReplacementResult::default();
    }

    let event = if let Some(token) = token {
        Event::create_tokens_with_token(controller, count, token, cause)
    } else {
        Event::create_tokens(controller, count, cause)
    };
    let result = process_with_dm(game, event, dm);

    let count = match result {
        TraitEventResult::Prevented => 0,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(create_tokens) = downcast_event::<CreateTokensEvent>(e.inner()) {
                return TokenCreationReplacementResult {
                    count: create_tokens.count,
                    additional_tokens: create_tokens.additional_tokens.clone(),
                };
            } else {
                count
            }
        }
        _ => count,
    };
    TokenCreationReplacementResult {
        count,
        additional_tokens: Vec::new(),
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
    dm: &mut dyn DecisionMaker,
) -> EtbEventResult {
    process_etb_with_event_and_dm_with_initial_counters(game, object, from, dm, Vec::new())
}

/// Process an ETB event and fully resolve all replacement choices/interactions,
/// including counters that are part of the original enter event.
pub fn process_etb_with_event_and_dm_with_initial_counters(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    dm: &mut dyn DecisionMaker,
    initial_enters_with_counters: Vec<(CounterType, u32)>,
) -> EtbEventResult {
    use crate::ability::AbilityKind;
    use crate::decisions::{
        make_decision,
        specs::{ReplacementOption, ReplacementSpec},
    };
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    game.update_replacement_effects();

    // Check the object's own abilities for ETB replacement effects.
    let enters_tapped = false;
    let mut enters_with_counters: Vec<(CounterType, u32)> = initial_enters_with_counters;

    // Gather ETB replacement effects from the object's abilities.
    let mut object_etb_effects: Vec<ReplacementEffect> = Vec::new();
    let mut copy_choice_effects: Vec<ReplacementEffect> = Vec::new();

    if let Some(obj) = game.object(object) {
        if let Some(loyalty) = obj.base_loyalty
            && loyalty > 0
        {
            // Planeswalkers intrinsically enter with loyalty counters equal to
            // their printed loyalty. Model this as ETB counters so replacement
            // effects can modify it (e.g., Doubling Season).
            let loyalty = loyalty_after_compleated_life_payment(obj, loyalty);
            enters_with_counters.push((CounterType::Loyalty, loyalty));
        }
        let controller = game.controller_of(obj);
        let view = crate::derived_view::DerivedGameView::new(game);
        let current_static_abilities = view
            .static_abilities_rc(object)
            .map(|abilities| abilities.as_ref().clone())
            .unwrap_or_else(|| {
                obj.abilities
                    .iter()
                    .filter_map(|ability| match &ability.kind {
                        AbilityKind::Static(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect()
            });
        for s in &current_static_abilities {
            // Check for unified replacement effects
            if let Some(effect) = s.generate_replacement_effect(object, controller) {
                object_etb_effects.push(effect);
            }
            if let Some(spec) = s.enter_as_copy_as_enters() {
                push_enter_as_copy_effects_for_spec(
                    game,
                    object,
                    object,
                    controller,
                    spec,
                    &mut copy_choice_effects,
                );
            }
        }
    }

    let view = crate::derived_view::DerivedGameView::new(game);
    let battlefield_sources = game
        .objects_in_deterministic_order()
        .into_iter()
        .filter(|candidate| candidate.id != object && candidate.zone == Zone::Battlefield)
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>();
    for source in battlefield_sources {
        let Some(source_obj) = game.object(source) else {
            continue;
        };
        let controller = game.controller_of(source_obj);
        let static_abilities = view
            .static_abilities_rc(source)
            .map(|abilities| abilities.as_ref().clone())
            .unwrap_or_else(|| {
                source_obj
                    .abilities
                    .iter()
                    .filter_map(|ability| match &ability.kind {
                        AbilityKind::Static(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect()
            });
        for static_ability in &static_abilities {
            let Some(spec) = static_ability.enter_as_copy_as_enters() else {
                continue;
            };
            if spec.affected_filter.is_none() {
                continue;
            }
            push_enter_as_copy_effects_for_spec(
                game,
                object,
                source,
                controller,
                spec,
                &mut copy_choice_effects,
            );
        }
    }
    // Keep ephemeral IDs far away from manager-issued IDs.
    const OBJECT_ETB_ID_BASE: u64 = u64::MAX - 1_000_000;
    const COPIED_OBJECT_ETB_ID_BASE: u64 = u64::MAX - 750_000;
    const COPY_CHOICE_ID_BASE: u64 = u64::MAX - 500_000;
    assign_ephemeral_effect_ids(&mut object_etb_effects, OBJECT_ETB_ID_BASE);
    assign_ephemeral_effect_ids(&mut copy_choice_effects, COPY_CHOICE_ID_BASE);

    let etb_event_provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::EnterBattlefield);
    let mut current_event = Event::new_with_provenance(
        EnterBattlefieldEvent {
            object,
            from,
            enters_tapped,
            enters_with_counters,
            linked_exile_with_entering: Vec::new(),
            enters_as_copy_of: None,
            copy_name_override: None,
            added_card_types: Vec::new(),
            removed_supertypes: Vec::new(),
            added_subtypes: Vec::new(),
            added_abilities: Vec::new(),
            set_base_power_toughness: None,
            controller_override: None,
        },
        etb_event_provenance,
    );
    let mut state = TraitEventProcessingState::default();
    let mut paid_labels = Vec::new();

    loop {
        let copy_choice_consumed = copy_choice_effects
            .iter()
            .any(|effect| state.was_applied(effect.id));
        let original_object_effects_still_apply =
            downcast_event::<EnterBattlefieldEvent>(current_event.inner())
                .map(|etb| etb.enters_as_copy_of.is_none())
                .unwrap_or(false);
        let copied_object_etb_effects = copied_object_etb_replacement_effects(
            game,
            object,
            &current_event,
            COPIED_OBJECT_ETB_ID_BASE,
        );
        let current_additional_effects: Vec<ReplacementEffect> = copy_choice_effects
            .iter()
            .filter(|_| !copy_choice_consumed)
            .chain(
                object_etb_effects
                    .iter()
                    .filter(|_| original_object_effects_still_apply),
            )
            .chain(
                copied_object_etb_effects
                    .iter()
                    .filter(|effect| !state.was_applied(effect.id)),
            )
            .cloned()
            .collect();
        let result = process_event_direct(
            game,
            current_event.clone(),
            &mut state,
            &current_additional_effects,
            None,
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
                    let copied_effects = copied_object_etb_replacement_effects(
                        game,
                        object,
                        &e,
                        COPIED_OBJECT_ETB_ID_BASE,
                    );
                    if !find_applicable_trait_replacements(game, &e, &state, &copied_effects, None)
                        .is_empty()
                    {
                        current_event = e;
                        continue;
                    }
                    return EtbEventResult {
                        enters_tapped: etb.enters_tapped,
                        enters_with_counters: etb.enters_with_counters.clone(),
                        linked_exile_with_entering: etb.linked_exile_with_entering.clone(),
                        prevented: false,
                        new_destination: None,
                        enters_as_copy_of: etb.enters_as_copy_of,
                        copy_name_override: etb.copy_name_override.clone(),
                        added_card_types: etb.added_card_types.clone(),
                        removed_supertypes: etb.removed_supertypes.clone(),
                        added_subtypes: etb.added_subtypes.clone(),
                        added_abilities: etb.added_abilities.clone(),
                        set_base_power_toughness: etb.set_base_power_toughness,
                        controller_override: etb.controller_override,
                        paid_labels,
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
            TraitEventResult::Replaced {
                effects,
                effect_id,
                source: replacement_source,
                controller: replacement_controller,
                ..
            } => {
                use crate::effects::{ExecutionContext, execute_effect};
                if game.object(object).is_some() {
                    game.effect_store
                        .replacement_effects
                        .mark_effect_used(effect_id);
                    let mut ctx =
                        ExecutionContext::new(replacement_source, replacement_controller, dm);
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
                ..
            } => {
                let options: Vec<ReplacementOption> = applicable_effects
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, &id)| {
                        find_effect_for_choice(game, &current_additional_effects, id).map(|e| {
                            ReplacementOption::new(
                                idx,
                                e.source,
                                replacement_effect_choice_description(game, &e),
                            )
                            .with_related_objects(replacement_effect_related_objects(&e))
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
                    find_effect_for_choice(game, &current_additional_effects, chosen_id)
                else {
                    state.mark_applied(chosen_id);
                    current_event = *event;
                    continue;
                };

                state.mark_applied_effect(&chosen_effect);
                if matches!(
                    chosen_effect.priority_override,
                    Some(crate::events::ReplacementPriority::CopyEffect)
                ) {
                    for effect in &copy_choice_effects {
                        state.mark_applied_effect(effect);
                    }
                }
                let apply_result = apply_trait_replacement(game, *event, &chosen_effect);
                consume_one_shot_if_applied(game, chosen_id, &apply_result);
                match apply_result {
                    TraitApplyResult::Modified(modified_event) => current_event = modified_event,
                    TraitApplyResult::Prevented => {
                        return EtbEventResult {
                            prevented: true,
                            ..Default::default()
                        };
                    }
                    TraitApplyResult::Replaced(effects) => {
                        use crate::effects::{ExecutionContext, execute_effect};
                        if game.object(object).is_some() {
                            game.effect_store
                                .replacement_effects
                                .mark_effect_used(chosen_id);
                            let mut ctx = ExecutionContext::new(
                                chosen_effect.source,
                                chosen_effect.controller,
                                dm,
                            );
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
                        destinations,
                    } => {
                        let life_cost = match &chosen_effect.replacement {
                            ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } => {
                                Some(*life_cost)
                            }
                            _ => None,
                        };
                        let controller = game
                            .object(object_id)
                            .map(|o| game.controller_of(o))
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
                            crate::decisions::context::DecisionContext::SelectOptions(ctx) => {
                                InteractiveReplacementResponse::Options(
                                    dm.decide_options(game, &ctx),
                                )
                            }
                            _ => InteractiveReplacementResponse::Decline,
                        };
                        state.mark_applied(effect_id);
                        if let ReplacementAction::Tribute {
                            counter_type,
                            count,
                            paid_label,
                        } = &chosen_effect.replacement
                        {
                            current_event = apply_tribute_response(
                                game,
                                current_event,
                                &response,
                                chosen_effect.source,
                                chosen_effect.controller,
                                *counter_type,
                                *count,
                                paid_label,
                                &mut paid_labels,
                                dm,
                            );
                            continue;
                        }
                        if let ReplacementAction::EnterWithCounterChoice {
                            counter_types,
                            count,
                        } = &chosen_effect.replacement
                        {
                            current_event = apply_enter_counter_choice_response(
                                game,
                                current_event,
                                &response,
                                chosen_effect.source,
                                counter_types,
                                count,
                            );
                            continue;
                        }
                        let interactive_result = continue_interactive_replacement(
                            game,
                            &response,
                            object_id,
                            controller,
                            filter.as_ref(),
                            redirect_zone,
                            life_cost,
                            destinations.as_deref(),
                            current_event.provenance(),
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
                destinations,
            } => {
                let controller = game
                    .object(object_id)
                    .map(|o| game.controller_of(o))
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
                    crate::decisions::context::DecisionContext::SelectOptions(ctx) => {
                        InteractiveReplacementResponse::Options(dm.decide_options(game, &ctx))
                    }
                    _ => InteractiveReplacementResponse::Decline,
                };
                state.mark_applied(effect_id);
                if let Some(ReplacementAction::Tribute {
                    counter_type,
                    count,
                    paid_label,
                }) = find_effect_for_choice(game, &current_additional_effects, effect_id)
                    .map(|effect| effect.replacement)
                {
                    current_event = apply_tribute_response(
                        game,
                        *event,
                        &response,
                        object_id,
                        controller,
                        counter_type,
                        count,
                        &paid_label,
                        &mut paid_labels,
                        dm,
                    );
                    continue;
                }
                if let Some(ReplacementAction::EnterWithCounterChoice {
                    counter_types,
                    count,
                }) = find_effect_for_choice(game, &current_additional_effects, effect_id)
                    .map(|effect| effect.replacement)
                {
                    current_event = apply_enter_counter_choice_response(
                        game,
                        *event,
                        &response,
                        object_id,
                        &counter_types,
                        &count,
                    );
                    continue;
                }
                let interactive_result = continue_interactive_replacement(
                    game,
                    &response,
                    object_id,
                    controller,
                    filter.as_ref(),
                    redirect_zone,
                    life_cost,
                    destinations.as_deref(),
                    event.provenance(),
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

fn loyalty_after_compleated_life_payment(obj: &crate::object::Object, loyalty: u32) -> u32 {
    let life_paid_count = obj
        .optional_costs_paid
        .times_paid_label("CompleatedLifePaid");
    if life_paid_count == 0 || !object_has_compleated_marker(obj) {
        return loyalty;
    }
    loyalty.saturating_sub(life_paid_count.saturating_mul(2))
}

fn object_has_compleated_marker(obj: &crate::object::Object) -> bool {
    obj.abilities.iter().any(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        static_ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
            && static_ability.display().eq_ignore_ascii_case("compleated")
    })
}

fn pay_madness_mana_cost(
    game: &mut GameState,
    player: crate::ids::PlayerId,
    source: crate::ids::ObjectId,
    cost: &crate::mana::ManaCost,
    decision_maker: &mut dyn DecisionMaker,
) -> bool {
    const MAX_MANA_ACTIVATIONS: usize = 32;

    for _ in 0..MAX_MANA_ACTIVATIONS {
        if game.try_pay_mana_cost_with_reason(
            player,
            Some(source),
            cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ) {
            return true;
        }

        let view = crate::derived_view::DerivedGameView::new(game);
        let next_mana_ability = game
            .objects_in_deterministic_order()
            .into_iter()
            .filter(|object| {
                object.zone == Zone::Battlefield && game.controller_of(object) == player
            })
            .find_map(|object| {
                let abilities = game.current_abilities(object.id)?;
                abilities
                    .into_iter()
                    .enumerate()
                    .find_map(|(ability_index, ability)| {
                        let crate::ability::AbilityKind::Activated(activated) = &ability.kind
                        else {
                            return None;
                        };
                        if !activated.is_runtime_mana_ability(game, object.id, player) {
                            return None;
                        }
                        if crate::special_actions::can_activate_mana_ability_check_with_view(
                            game,
                            player,
                            object.id,
                            ability_index,
                            &ability,
                            &view,
                            None,
                        )
                        .is_ok()
                        {
                            Some((object.id, ability_index))
                        } else {
                            None
                        }
                    })
            });

        let Some((permanent_id, ability_index)) = next_mana_ability else {
            return false;
        };
        if crate::special_actions::perform_activate_mana_ability(
            game,
            player,
            permanent_id,
            ability_index,
            decision_maker,
        )
        .is_err()
        {
            return false;
        }
        if decision_maker.awaiting_choice() {
            return false;
        }
    }

    game.try_pay_mana_cost_with_reason(
        player,
        Some(source),
        cost,
        0,
        crate::costs::PaymentReason::CastSpell,
    )
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
        applied_effects: std::collections::HashSet<ReplacementEffectId>,
        applied_effect_keys: std::collections::HashSet<ReplacementEffectKey>,
        default_zone: Zone,
    },
}

/// Process a zone change event with full replacement effect handling.
///
/// This is the comprehensive version that returns all possible outcomes,
/// including `Replaced` and `NeedsChoice` cases that the simpler
/// `process_zone_change_with_event` doesn't support.
pub fn process_zone_change_full(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
) -> ZoneChangeResult {
    use crate::events::{ZoneChangeEvent, downcast_event};

    game.update_replacement_effects();

    let snapshot = game.object(object).map(|o| {
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(o, game)
    });

    let event = Event::zone_change(object, from, to, cause, snapshot);
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
            applied_effects,
            applied_effect_keys,
        } => ZoneChangeResult::NeedsChoice {
            player,
            applicable_effects,
            event,
            applied_effects,
            applied_effect_keys,
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
        applied_effects: std::collections::HashSet<ReplacementEffectId>,
        applied_effect_keys: std::collections::HashSet<ReplacementEffectKey>,
        default_count: u32,
    },
}

/// Process a draw event with full replacement effect handling.
///
/// This is the comprehensive version that returns all possible outcomes,
/// using the new Event type.
pub fn process_draw_full(
    game: &mut GameState,
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
            applied_effects,
            applied_effect_keys,
        } => DrawResult::NeedsChoice {
            player,
            applicable_effects,
            event,
            applied_effects,
            applied_effect_keys,
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
    game: &mut GameState,
    event: Event,
    chosen_effect_id: ReplacementEffectId,
) -> TraitEventResult {
    process_event_with_chosen_replacement_trait_and_applied_effects(
        game,
        event,
        chosen_effect_id,
        &std::collections::HashSet::new(),
        &std::collections::HashSet::new(),
    )
}

/// Process an event with a chosen replacement effect and prior applied state.
///
/// This is used when a replacement-choice prompt was deferred after one or more
/// replacement effects had already modified the same event. CR 614.5 still
/// prevents those prior effects from applying again after the player chooses.
pub fn process_event_with_chosen_replacement_trait_and_applied_effects(
    game: &mut GameState,
    event: Event,
    chosen_effect_id: ReplacementEffectId,
    applied_effects: &std::collections::HashSet<ReplacementEffectId>,
    applied_effect_keys: &std::collections::HashSet<ReplacementEffectKey>,
) -> TraitEventResult {
    let event = game.ensure_event_provenance(event);
    let mut state = TraitEventProcessingState::default();
    state
        .applied_effects
        .extend(applied_effects.iter().copied());
    state
        .applied_effect_keys
        .extend(applied_effect_keys.iter().cloned());

    // Get the chosen effect
    let Some(effect) = game
        .effect_store
        .replacement_effects
        .get_effect(chosen_effect_id)
        .cloned()
    else {
        // Effect no longer exists - continue while preserving prior applications.
        return process_event_direct(game, event, &mut state, &[], None);
    };

    // Apply the chosen replacement effect
    let apply_result = apply_trait_replacement(game, event.clone(), &effect);
    consume_one_shot_if_applied(game, chosen_effect_id, &apply_result);

    state.mark_applied_effect(&effect);

    match apply_result {
        TraitApplyResult::Modified(modified) => {
            // Continue processing with the modified event
            process_event_direct(game, modified, &mut state, &[], None)
        }
        TraitApplyResult::Prevented => TraitEventResult::Prevented,
        TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced {
            effects,
            effect_id: chosen_effect_id,
            replacement: effect.replacement.clone(),
            source: effect.source,
            controller: effect.controller,
        },
        TraitApplyResult::Unchanged(unchanged) => {
            // Effect didn't change anything - continue with original event
            process_event_direct(game, unchanged, &mut state, &[], None)
        }
        TraitApplyResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            filter,
            destinations,
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
            destinations,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{Effect, EventValueSpec, Value};
    use crate::events::cause::EventCause;
    use crate::ids::{CardId, ObjectId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::prevention::{PreventionShield, PreventionTarget};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::static_abilities::{Anthem, StaticAbility};
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn deferred_replacement_choice_preserves_prior_applications_for_614_5() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let damage_source = create_creature(&mut game, "Sparkmage", alice);
        let first_replacement_source = create_creature(&mut game, "First Replacement", alice);
        let choice_a_source = create_creature(&mut game, "Choice A Replacement", alice);
        let choice_b_source = create_creature(&mut game, "Choice B Replacement", alice);

        let first_effect_id = game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                first_replacement_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Add(1)),
            )
            .with_priority_override(crate::events::traits::ReplacementPriority::SelfReplacement),
        );
        let choice_a_effect_id = game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                choice_a_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Add(10)),
            ),
        );
        let choice_b_effect_id = game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                choice_b_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Add(100)),
            ),
        );

        let result = process_trait_event(
            &mut game,
            Event::damage(
                damage_source,
                DamageTarget::Player(bob),
                1,
                false,
                EventCause::effect(),
            ),
        );

        let TraitEventResult::NeedsChoice {
            applicable_effects,
            event,
            applied_effects,
            applied_effect_keys,
            ..
        } = result
        else {
            panic!("expected the two equal-priority replacements to require a choice");
        };
        assert!(
            applicable_effects.contains(&choice_a_effect_id)
                && applicable_effects.contains(&choice_b_effect_id),
            "expected both equal-priority replacements in the deferred choice"
        );
        assert!(
            applied_effects.contains(&first_effect_id),
            "the earlier replacement effect must be carried into the deferred choice"
        );
        let pending_damage =
            crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
                .expect("pending choice should carry the modified damage event");
        assert_eq!(
            pending_damage.amount, 2,
            "the first replacement should have already modified the event"
        );

        let resumed = process_event_with_chosen_replacement_trait_and_applied_effects(
            &mut game,
            (*event).clone(),
            choice_a_effect_id,
            &applied_effects,
            &applied_effect_keys,
        );
        let final_event = match resumed {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => event,
            other => panic!("expected resumed replacement processing to proceed, got {other:?}"),
        };
        let final_damage =
            crate::events::downcast_event::<crate::events::DamageEvent>(final_event.inner())
                .expect("resumed choice should still be a damage event");
        assert_eq!(
            final_damage.amount, 112,
            "CR 614.5 should prevent the first replacement from applying again after the choice"
        );
    }

    #[test]
    fn zone_change_lki_snapshot_uses_calculated_characteristics() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Anthem Bear", alice);
        game.object_mut(creature)
            .expect("creature exists")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(StaticAbility::new(
                Anthem::for_source(2, 0),
            )));

        let external_source = create_creature(&mut game, "Replacement Source", alice);
        for destination in [Zone::Exile, Zone::Hand] {
            game.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    external_source,
                    alice,
                    crate::events::zones::matchers::WouldGoToGraveyardMatcher::new(
                        crate::target::ObjectFilter::default()
                            .controlled_by(crate::target::PlayerFilter::Specific(alice)),
                    ),
                    ReplacementAction::ChangeDestination(destination),
                ),
            );
        }

        let result = process_zone_change_full(
            &mut game,
            creature,
            Zone::Battlefield,
            Zone::Graveyard,
            EventCause::effect(),
        );

        let ZoneChangeResult::NeedsChoice { event, .. } = result else {
            panic!("expected multiple replacements to expose the zone-change event");
        };
        let snapshot = event
            .0
            .snapshot()
            .expect("zone-change event should carry object LKI");
        assert_eq!(
            snapshot.power,
            Some(4),
            "LKI should include continuous effects that modified the creature before it left"
        );
    }

    #[test]
    fn exile_with_source_link_counters_replacement_adds_counters_to_exiled_object() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = create_creature(&mut game, "Ice Necromancer", alice);
        let creature = create_creature(&mut game, "Doomed Bear", alice);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldGoToGraveyardMatcher::new(
                    crate::target::ObjectFilter::specific(creature),
                ),
                ReplacementAction::ExileWithSourceLinkCountersThen {
                    counters: vec![(CounterType::Ice, 1)],
                    effects: Vec::new(),
                },
            ),
        );

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let outcome = process_zone_change(
            &mut game,
            creature,
            Zone::Battlefield,
            Zone::Graveyard,
            EventCause::effect(),
            &mut dm,
        );

        assert!(
            outcome.is_replaced(),
            "expected replacement, got {outcome:?}"
        );
        let [exiled] = game.exile.as_slice() else {
            panic!("expected one exiled object, got {:?}", game.exile);
        };
        assert_eq!(game.counter_count(*exiled, CounterType::Ice), 1);
        assert_eq!(game.get_exiled_with_source_links(source), &[*exiled]);
    }

    #[test]
    fn prevention_follow_up_executes_with_prevented_amount_on_damaged_target() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let protected = create_creature(&mut game, "Protected Bear", alice);
        let source = create_creature(&mut game, "Shock Bear", bob);

        let shield = PreventionShield::prevent_next_n(
            source,
            alice,
            PreventionTarget::Permanent(protected),
            3,
        )
        .with_follow_up_effects(vec![Effect::new(crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            Value::EventValue(EventValueSpec::Amount),
            ChooseSpec::AnyTarget,
        ))]);
        game.effect_store.prevention_effects.add_shield(shield);

        let processed = process_damage_assignments_with_event(
            &mut game,
            source,
            DamageTarget::Object(protected),
            3,
            false,
            EventCause::effect(),
        );

        assert!(
            processed.assignments.is_empty(),
            "damage should be fully prevented: {processed:?}"
        );
        assert_eq!(
            game.counter_count(protected, CounterType::PlusOnePlusOne),
            3,
            "follow-up should use the prevented amount on the damaged creature"
        );
    }

    #[test]
    fn static_damage_prevention_replacements_are_refreshed_before_damage() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let protected = create_creature(&mut game, "Stormwild Stand-In", alice);
        game.object_mut(protected)
            .expect("creature exists")
            .abilities_mut().push(crate::ability::Ability::static_ability(
                StaticAbility::prevent_constrained_damage_to_self_put_counters_instead(
                    CounterType::PlusOnePlusOne,
                    "If noncombat damage would be dealt to this creature, prevent that damage. Put a +1/+1 counter on it for each 1 damage prevented this way.",
                    None,
                    Some(false),
                ),
            ));
        let source = create_creature(&mut game, "Shock Bear", bob);

        let processed = process_damage_assignments_with_event(
            &mut game,
            source,
            DamageTarget::Object(protected),
            3,
            false,
            EventCause::effect(),
        );

        assert!(
            processed.assignments.is_empty(),
            "static prevention replacement should fully replace the damage: {processed:?}"
        );
        assert_eq!(
            game.counter_count(protected, CounterType::PlusOnePlusOne),
            3,
            "replacement follow-up should put counters equal to the prevented damage"
        );
    }

    #[test]
    fn damage_replacement_source_filter_uses_lki_for_departed_source() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let damage_source = create_creature(&mut game, "Departed Sparkmage", alice);
        let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(damage_source).expect("source exists"),
            &game,
        );
        let replacement_source = create_creature(&mut game, "Damage Doubler", bob);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::double_damage(
                replacement_source,
                bob,
                crate::target::ObjectFilter::creature(),
            ),
        );

        game.move_object(damage_source, Zone::Graveyard, EventCause::effect())
            .expect("source moved");

        let processed = process_damage_assignments_with_event_with_source_snapshot(
            &mut game,
            damage_source,
            DamageTarget::Player(bob),
            3,
            false,
            EventCause::effect(),
            Some(&source_snapshot),
        );

        let total_damage: u32 = processed
            .assignments
            .iter()
            .map(|assignment| assignment.amount)
            .sum();
        assert_eq!(
            total_damage, 6,
            "source-filtered damage replacements should match departed sources using LKI"
        );
    }
}
