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
use crate::events::{Event, EventContext, ReplacementMatcher as _};
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
    find_matching_cards_in_hand, find_matching_sacrificable_permanents,
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

pub(crate) fn replacement_effect_choice_description(
    game: &GameState,
    effect: &ReplacementEffect,
) -> String {
    match &effect.replacement {
        ReplacementAction::Additionally(_) => {
            format!(
                "Do not apply {}",
                replacement_option_description(game, effect.source)
            )
        }
        ReplacementAction::DeclineOptional(_) => {
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

fn mark_applied_replacement_choice(
    state: &mut TraitEventProcessingState,
    effect: &ReplacementEffect,
) {
    state.mark_applied_effect(effect);
    if let Some(decline) = effect.optional_decline_effect() {
        state.applied_effect_keys.insert(decline.application_key());
    }
    if let ReplacementAction::DeclineOptional(declined_key) = &effect.replacement {
        state.applied_effect_keys.insert(declined_key.clone());
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
    reserved_objects: &std::collections::HashSet<ObjectId>,
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
            .filter(|candidate| !reserved_objects.contains(&candidate.id))
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
                            copy_duration: spec.copy_duration.clone(),
                            linked_exile_objects: vec![copy_candidate, counter_candidate],
                            additional_counters: vec![(linked_pair.counter_type, counter_count)],
                            name_override: spec.name_override.clone(),
                            added_colors: spec.added_colors,
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
                    copy_duration: spec.copy_duration.clone(),
                    linked_exile_objects: Vec::new(),
                    additional_counters: Vec::new(),
                    name_override: spec.name_override.clone(),
                    added_colors: spec.added_colors,
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
        None,
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

fn damage_event_has_been_removed(event: &Event) -> bool {
    crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
        .is_some_and(|damage| damage.amount == 0)
}

/// Process an event directly using trait-based matchers.
fn process_event_direct(
    game: &mut GameState,
    event: Event,
    state: &mut TraitEventProcessingState,
    additional_effects: &[ReplacementEffect],
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> TraitEventResult {
    // CR 614.7: once a replacement effect reduces a damage event to zero,
    // that damage event no longer exists. Do not offer it to later replacement
    // effects. Returning the zero-valued carrier lets the caller retain any
    // separately modeled partial-redirection remainder without dealing damage.
    if damage_event_has_been_removed(&event) {
        return TraitEventResult::Proceed(event);
    }

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
        mark_applied_replacement_choice(state, &chosen_effect);
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
                sacrifice_count,
                destinations,
            } => TraitEventResult::NeedsInteraction {
                decision_ctx,
                redirect_zone,
                effect_id,
                object_id,
                event: Box::new(event),
                filter,
                sacrifice_count,
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
    mark_applied_replacement_choice(state, &chosen_effect);
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
            sacrifice_count,
            destinations,
        } => TraitEventResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            event: Box::new(event),
            filter,
            sacrifice_count,
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
    sacrifice_count: Option<u32>,
    redirect_zone: Zone,
    life_cost: Option<u32>,
    destinations: Option<&[Zone]>,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
) -> InteractiveReplacementResult {
    if let (Some(filter), Some(count)) = (filter, sacrifice_count) {
        return handle_sacrifice_or_redirect(
            game,
            response,
            object_id,
            controller,
            filter,
            count,
            redirect_zone,
            provenance,
            decision_maker,
        );
    }

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

fn handle_sacrifice_or_redirect(
    game: &mut GameState,
    response: &InteractiveReplacementResponse,
    object_id: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    filter: &crate::target::ObjectFilter,
    count: u32,
    redirect_zone: Zone,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
) -> InteractiveReplacementResult {
    let InteractiveReplacementResponse::Objects(objects) = response else {
        return InteractiveReplacementResult::redirected(redirect_zone);
    };
    if objects.len() != count as usize {
        return InteractiveReplacementResult::redirected(redirect_zone);
    }
    let distinct = objects
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    if distinct.len() != objects.len() {
        return InteractiveReplacementResult::redirected(redirect_zone);
    }
    let candidates = find_matching_sacrificable_permanents(game, controller, object_id, filter);
    if !objects.iter().all(|object| candidates.contains(object)) {
        return InteractiveReplacementResult::redirected(redirect_zone);
    }

    let mut ctx = crate::effects::ExecutionContext::new(object_id, controller, decision_maker);
    ctx.provenance = provenance;
    for permanent in objects {
        let effect = crate::effect::Effect::new(crate::effects::SacrificeTargetEffect::new(
            crate::target::ChooseSpec::SpecificObject(*permanent),
        ));
        let Ok(outcome) = crate::effects::execute_effect(game, &effect, &mut ctx) else {
            return InteractiveReplacementResult::redirected(redirect_zone);
        };
        if !matches!(outcome.value, crate::effect::OutcomeValue::Count(value) if value >= 1) {
            return InteractiveReplacementResult::redirected(redirect_zone);
        }
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
    }
    InteractiveReplacementResult::enters_battlefield()
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
        Zone::Graveyard | Zone::Battlefield | Zone::Stack | Zone::Command | Zone::Ante => true,
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
                            aggregate_constraint: requirement.aggregate_constraint.clone(),
                            min_targets: requirement.min_targets,
                            max_targets: requirement.max_targets,
                            distinct_player_group: requirement.distinct_player_group,
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
        /// Exact sacrifice count for InteractiveSacrificeOrRedirect.
        sacrifice_count: Option<u32>,
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
    let prospective_etb_game =
        crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
            .and_then(|etb| etb.prospective_game_state(game));

    // Check registered replacement effects in the game
    for effect in game.effect_store.replacement_effects.effects() {
        // Skip if already applied (Rule 614.5)
        if state.was_applied_effect(effect) {
            continue;
        }

        // Check if effect matches using trait-based matcher
        if let Some(priority) = trait_effect_matches_event(
            game,
            effect,
            event,
            event_source_snapshot,
            prospective_etb_game.as_ref(),
        ) {
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
        if let Some(priority) = trait_effect_matches_event(
            game,
            effect,
            event,
            event_source_snapshot,
            prospective_etb_game.as_ref(),
        ) {
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
    prospective_etb_game: Option<&GameState>,
) -> Option<ReplacementPriority> {
    use crate::events::ReplacementPriority as TraitPriority;

    // All effects should have trait-based matchers
    let matcher = effect.matcher.as_ref()?;

    let ctx = EventContext::for_replacement_effect(effect.controller, effect.source, game)
        .with_prospective_etb_game(prospective_etb_game)
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
        /// Exact sacrifice count for InteractiveSacrificeOrRedirect.
        sacrifice_count: Option<u32>,
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

    let destroy_snapshot = lki_snapshot.clone().or_else(|| {
        game.object(permanent).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        })
    });

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
    let event = game.ensure_event_provenance(Event::destroy(permanent, source));

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
                    if final_zone == Zone::Graveyard
                        && let Some(snapshot) = destroy_snapshot.clone()
                    {
                        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
                            crate::events::DestroyEvent::new(permanent, source)
                                .with_successful_result(snapshot, final_zone),
                            event.provenance(),
                        );
                        game.queue_trigger_event(trigger_event.provenance(), trigger_event);
                    }
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

fn merged_card_only_change_destinations(
    game: &GameState,
    event: &crate::events::ZoneChangeEvent,
    additional_effects: &[ReplacementEffect],
) -> std::collections::HashSet<Zone> {
    game.effect_store
        .replacement_effects
        .effects()
        .iter()
        .chain(additional_effects.iter())
        .filter_map(|effect| {
            let matcher = effect.matcher.as_ref()?;
            let ctx = crate::events::context::EventContext::for_replacement_effect(
                effect.controller,
                effect.source,
                game,
            );
            if !matcher.matches_merged_card_component_only(event, &ctx) {
                return None;
            }
            match &effect.replacement {
                crate::replacement::ReplacementAction::ChangeDestination(destination) => {
                    Some(*destination)
                }
                _ => None,
            }
        })
        .collect()
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

    let zone_event =
        ZoneChangeEvent::with_cause(object, from, requested_to, cause.clone(), snapshot.clone());
    let merged_card_only_destinations =
        merged_card_only_change_destinations(game, &zone_event, additional_effects);
    let event = Event::zone_change(object, from, requested_to, cause.clone(), snapshot.clone());
    let mut additional_effects = additional_effects.to_vec();
    assign_ephemeral_effect_ids(&mut additional_effects, (u64::MAX / 2).saturating_add(1024));
    let result =
        process_with_dm_and_additional_effects(game, event.clone(), dm, &additional_effects);

    match result {
        TraitEventResult::Prevented => EventOutcome::Prevented,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            let final_zone = if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner())
            {
                zone_change.to
            } else if downcast_event::<crate::events::EnterBattlefieldEvent>(e.inner()).is_some() {
                // Entry modifiers can promote an evolving Stack/other-zone
                // change into the ETB event carrier (for example, a
                // destination replacement followed by "under your control").
                // The carrier itself proves that the rewritten destination is
                // the battlefield; falling back to the originally requested
                // graveyard here would discard the earlier destination change.
                Zone::Battlefield
            } else {
                requested_to
            };
            if merged_card_only_destinations.contains(&final_zone) {
                game.prepare_merged_token_card_component_destinations(object, to, final_zone);
            } else {
                game.prepare_merged_component_destinations(object, final_zone, dm);
            }
            EventOutcome::Proceed(final_zone)
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
                let replacement_object_snapshot = if let Some(new_id) =
                    game.move_object(object, destination, cause.clone())
                {
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
                    game.object(new_id).map(|object| {
                        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                            object, game,
                        )
                    })
                } else {
                    None
                };
                if !effects.is_empty() {
                    let mut ctx = crate::effects::ExecutionContext::new(
                        replacement_source,
                        replacement_controller,
                        dm,
                    );
                    if let Some(snapshot) = replacement_object_snapshot {
                        ctx.tag_object(crate::tag::ZONE_REPLACEMENT_OBJECT_TAG, snapshot);
                    }
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

/// Result of applying one impending game-loss event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLossOutcome {
    /// The loss was not replaced and the player left the game.
    Lost,
    /// A replacement effect replaced the loss with its effect sequence.
    Replaced,
    /// A rule restriction or replacement effect prevented the loss.
    Prevented,
}

fn loss_replacement_source_destination(
    game: &GameState,
    player: PlayerId,
    source: crate::ids::ObjectId,
    effects: &[crate::effect::Effect],
) -> Option<crate::zone::Zone> {
    for effect in effects {
        if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>()
            && matches!(exile.spec.base(), crate::target::ChooseSpec::Source)
        {
            return Some(crate::zone::Zone::Exile);
        }
        if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            && matches!(
                move_to_zone.target.base(),
                crate::target::ChooseSpec::Source
            )
        {
            return Some(move_to_zone.zone);
        }
        if let Some(shuffle) =
            effect.downcast_ref::<crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect>()
            && shuffle.include_owned_permanents
            && game
                .object(source)
                .is_some_and(|object| object.owner == player)
        {
            return Some(crate::zone::Zone::Library);
        }
    }
    None
}

fn choose_mutually_exclusive_source_destination(
    game: &GameState,
    source: crate::ids::ObjectId,
    replacement_destination: crate::zone::Zone,
    sba_destination: crate::zone::Zone,
    dm: &mut (impl DecisionMaker + ?Sized),
) -> crate::zone::Zone {
    let chooser = game
        .current_controller(source)
        .or_else(|| game.object(source).map(|object| object.owner));
    let Some(chooser) = chooser else {
        return replacement_destination;
    };
    let source_name = game
        .object(source)
        .map(|object| object.name.to_string())
        .unwrap_or_else(|| "the object".to_string());
    let options = vec![
        crate::decisions::DisplayOption::new(
            0,
            format!("Move {source_name} to {replacement_destination:?} (replacement effect)"),
        ),
        crate::decisions::DisplayOption::new(
            1,
            format!("Move {source_name} to {sba_destination:?} (state-based action)"),
        ),
    ];
    let selected = crate::decisions::make_decision(
        game,
        dm,
        chooser,
        Some(source),
        crate::decisions::ChoiceSpec::single(source, options),
    );
    if selected.first().copied() == Some(1) {
        sba_destination
    } else {
        replacement_destination
    }
}

/// Process and apply a game loss through the ordinary replacement framework.
///
/// Both spell/ability effects and state-based actions use this entry point so
/// an impending loss is never committed before applicable CR 614 effects are
/// chosen. The replacement source is snapshotted before its effects execute;
/// that preserves source LKI when the replacement moves its own source as its
/// first instruction (for example, Exquisite Archangel).
pub fn process_player_loss(
    game: &mut GameState,
    player: PlayerId,
    dm: &mut dyn DecisionMaker,
) -> PlayerLossOutcome {
    process_player_loss_with_simultaneous_zone_changes(
        game,
        player,
        dm,
        &std::collections::HashMap::new(),
    )
}

/// Process a loss that is simultaneous with other SBA zone changes.
///
/// CR 400.6 lets an object's controller (or owner when it has no controller)
/// choose between mutually exclusive destinations. The chosen destination is
/// carried through the replacement sequence so the object moves exactly once.
pub(crate) fn process_player_loss_with_simultaneous_zone_changes(
    game: &mut GameState,
    player: PlayerId,
    dm: &mut dyn DecisionMaker,
    simultaneous_zone_changes: &std::collections::HashMap<crate::ids::ObjectId, crate::zone::Zone>,
) -> PlayerLossOutcome {
    if !game.can_lose_game(player)
        || game
            .player(player)
            .is_none_or(|player| !player.is_in_game())
    {
        return PlayerLossOutcome::Prevented;
    }

    game.update_replacement_effects();
    let event = Event::player_loses_game(player);
    match process_with_dm(game, event, dm) {
        TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {
            if game.mark_player_lost(player) {
                PlayerLossOutcome::Lost
            } else {
                PlayerLossOutcome::Prevented
            }
        }
        TraitEventResult::Prevented => PlayerLossOutcome::Prevented,
        TraitEventResult::Replaced {
            effects,
            effect_id,
            source,
            controller,
            ..
        } => {
            let source_snapshot = game.object(source).map(|object| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    object, game,
                )
            });
            game.effect_store
                .replacement_effects
                .mark_effect_used(effect_id);
            let mut ctx = crate::effects::ExecutionContext::new(source, controller, dm);
            ctx.source_snapshot = source_snapshot;
            if let Some(sba_destination) = simultaneous_zone_changes.get(&source).copied()
                && let Some(replacement_destination) =
                    loss_replacement_source_destination(game, player, source, &effects)
                && replacement_destination != sba_destination
            {
                let chosen_destination = choose_mutually_exclusive_source_destination(
                    game,
                    source,
                    replacement_destination,
                    sba_destination,
                    &mut *ctx.decision_maker,
                );
                if chosen_destination != replacement_destination {
                    ctx.replacement
                        .simultaneous_zone_destinations
                        .insert(source, chosen_destination);
                }
            }
            for effect in effects {
                if let Ok(outcome) = crate::effects::execute_effect(game, &effect, &mut ctx) {
                    for trigger_event in outcome.events {
                        game.queue_trigger_event(trigger_event.provenance(), trigger_event);
                    }
                }
            }
            PlayerLossOutcome::Replaced
        }
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            // Synchronous SBA/effect callers cannot commit a loss while a
            // replacement choice is outstanding.
            PlayerLossOutcome::Prevented
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
    process_with_dm_and_additional_effects_and_snapshot(game, event, dm, &[], None)
}

fn process_with_dm_and_additional_effects(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
) -> TraitEventResult {
    process_with_dm_and_additional_effects_and_snapshot(game, event, dm, additional_effects, None)
}

fn process_with_dm_and_additional_effects_and_snapshot(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> TraitEventResult {
    process_with_dm_and_additional_effects_and_applied(
        game,
        event,
        dm,
        additional_effects,
        &std::collections::HashSet::new(),
        &std::collections::HashSet::new(),
        event_source_snapshot,
    )
}

fn process_with_dm_and_additional_effects_and_applied(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
    applied_effects: &std::collections::HashSet<ReplacementEffectId>,
    applied_effect_keys: &std::collections::HashSet<ReplacementEffectKey>,
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
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
            event_source_snapshot,
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
                            find_effect_for_choice(game, additional_effects, id).map(|e| {
                                ReplacementOption::new(
                                    idx,
                                    e.source,
                                    replacement_effect_choice_description(game, &e),
                                )
                                .with_related_objects(replacement_effect_related_objects(&e))
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

                let Some(chosen_effect) =
                    find_effect_for_choice(game, additional_effects, effect_id)
                else {
                    // Effect disappeared (e.g., source left battlefield). Continue with event.
                    state.mark_applied(effect_id);
                    current_event = *boxed_event;
                    continue;
                };

                mark_applied_replacement_choice(&mut state, &chosen_effect);

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
                        sacrifice_count,
                        destinations,
                    } => {
                        return TraitEventResult::NeedsInteraction {
                            decision_ctx,
                            redirect_zone,
                            effect_id,
                            object_id,
                            event: Box::new(current_event),
                            filter,
                            sacrifice_count,
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
    /// Duration of a temporary as-enters copy effect, if any.
    pub copy_duration: Option<crate::effect::Until>,
    pub copy_name_override: Option<String>,
    /// Colors added by an ETB copy choice.
    pub added_colors: crate::color::ColorSet,
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
    /// As-entry choices collected before the destination object exists.
    pub(crate) prepared_choices: Option<crate::game_state::PreparedEtbChoices>,
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

/// One damage event in a set that would happen simultaneously.
#[derive(Debug, Clone)]
pub struct SimultaneousDamageEvent {
    pub source: crate::ids::ObjectId,
    pub target: DamageTarget,
    pub amount: u32,
    pub is_combat: bool,
    pub unpreventable: bool,
    pub cause: crate::events::cause::EventCause,
    pub source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
}

#[derive(Debug, Clone, Default)]
struct PreventionBatchAllocation {
    allocated_shields: std::collections::HashSet<crate::prevention::PreventionShieldId>,
    limits: std::collections::HashMap<crate::prevention::PreventionShieldId, u32>,
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
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(
        game,
        source,
        target,
        amount,
        is_combat,
        unpreventable,
        cause,
        source_snapshot,
        &mut dm,
    )
}

#[derive(Debug, Clone)]
struct PreventionShieldReplacementMatcher {
    shield_id: crate::prevention::PreventionShieldId,
    source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
}

impl crate::events::ReplacementMatcher for PreventionShieldReplacementMatcher {
    fn matches_event(&self, event: &dyn crate::events::GameEventType, ctx: &EventContext) -> bool {
        let Some(damage) = crate::events::downcast_event::<crate::events::DamageEvent>(event)
        else {
            return false;
        };
        let Some(shield) = ctx
            .game
            .effect_store
            .prevention_effects
            .shields()
            .iter()
            .find(|shield| shield.id == self.shield_id)
        else {
            return false;
        };
        if !shield.has_prevention_remaining() {
            return false;
        }

        // CR 801.13b keys range to whichever side the prevention effect
        // specifies: source, recipient, or both when neither is specified.
        let range_exempt = ctx.game.source_is_exempt_from_range(Some(shield.source));
        let source_in_range = range_exempt
            || ctx.game.object(damage.source).map_or_else(
                || {
                    self.source_snapshot.as_ref().is_some_and(|snapshot| {
                        ctx.game
                            .player_is_within_range(shield.controller, snapshot.controller)
                    })
                },
                |_| {
                    ctx.game.object_is_within_range(
                        shield.controller,
                        damage.source,
                        Some(shield.source),
                    )
                },
            );
        let recipient_in_range = range_exempt
            || match damage.target {
                DamageTarget::Player(player) => {
                    ctx.game.player_is_within_range(shield.controller, player)
                }
                DamageTarget::Object(object) => {
                    ctx.game
                        .object_is_within_range(shield.controller, object, Some(shield.source))
                }
            };
        let source_is_specified = shield.damage_filter.from_source.is_some()
            || shield.damage_filter.from_colors.is_some()
            || shield.damage_filter.from_card_types.is_some()
            || shield.damage_filter.from_specific_source.is_some()
            || shield.damage_filter.excluded_specific_source.is_some();
        let recipient_is_specified = shield.protected != crate::prevention::PreventionTarget::All;
        if (source_is_specified && !source_in_range)
            || (recipient_is_specified && !recipient_in_range)
            || (!source_is_specified
                && !recipient_is_specified
                && !(source_in_range && recipient_in_range))
        {
            return false;
        }

        let protects_target = match (damage.target, &shield.protected) {
            (DamageTarget::Player(player), crate::prevention::PreventionTarget::Player(p)) => {
                player == *p
            }
            (DamageTarget::Player(_), crate::prevention::PreventionTarget::Players) => true,
            (DamageTarget::Player(player), crate::prevention::PreventionTarget::You)
            | (
                DamageTarget::Player(player),
                crate::prevention::PreventionTarget::YouAndPermanentsYouControl,
            )
            | (
                DamageTarget::Player(player),
                crate::prevention::PreventionTarget::YouAndPermanentsMatching(_),
            ) => player == shield.controller,
            (DamageTarget::Object(object), crate::prevention::PreventionTarget::Permanent(p)) => {
                object == *p
            }
            (
                DamageTarget::Object(object),
                crate::prevention::PreventionTarget::YouAndPermanentsYouControl,
            ) => ctx
                .game
                .object(object)
                .is_some_and(|object| ctx.game.controller_of(object) == shield.controller),
            (
                DamageTarget::Object(object),
                crate::prevention::PreventionTarget::PermanentsMatching(filter),
            )
            | (
                DamageTarget::Object(object),
                crate::prevention::PreventionTarget::YouAndPermanentsMatching(filter),
            ) => {
                let filter_ctx = ctx
                    .game
                    .filter_context_for(shield.controller, Some(shield.source));
                ctx.game
                    .object(object)
                    .is_some_and(|object| filter.matches(object, &filter_ctx, ctx.game))
            }
            (_, crate::prevention::PreventionTarget::All) => true,
            _ => false,
        };
        if !protects_target {
            return false;
        }

        let filter_ctx = ctx
            .game
            .filter_context_for(shield.controller, Some(shield.source));
        if let Some(source_filter) = &shield.damage_filter.from_source {
            let current_matches = ctx
                .game
                .object(damage.source)
                .is_some_and(|source| source_filter.matches(source, &filter_ctx, ctx.game));
            let lki_matches = self
                .source_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.object_id == damage.source)
                .is_some_and(|snapshot| {
                    source_filter.matches_snapshot(snapshot, &filter_ctx, ctx.game)
                });
            if !current_matches && !lki_matches {
                return false;
            }
        }

        let (source_colors, source_card_types) =
            if let Some(characteristics) = ctx.game.calculated_characteristics(damage.source) {
                (characteristics.colors, characteristics.card_types.to_vec())
            } else if let Some(snapshot) = self
                .source_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.object_id == damage.source)
            {
                (snapshot.colors, snapshot.card_types.clone())
            } else {
                (crate::color::ColorSet::COLORLESS, Vec::new())
            };
        shield.damage_filter.matches(
            damage.is_combat,
            damage.source,
            &source_colors,
            &source_card_types,
        )
    }

    fn priority(&self) -> crate::events::ReplacementPriority {
        crate::events::ReplacementPriority::Other
    }

    fn display(&self) -> String {
        format!("Prevention shield {}", self.shield_id.0)
    }
}

fn prevention_shield_replacement_effects(
    game: &GameState,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    batch_allocation: Option<&PreventionBatchAllocation>,
) -> Vec<ReplacementEffect> {
    game.effect_store
        .prevention_effects
        .shields()
        .iter()
        .filter_map(|shield| {
            let max_amount = if batch_allocation
                .is_some_and(|allocation| allocation.allocated_shields.contains(&shield.id))
            {
                let amount = batch_allocation
                    .and_then(|allocation| allocation.limits.get(&shield.id))
                    .copied()
                    .unwrap_or(0);
                if amount == 0 {
                    return None;
                }
                Some(amount)
            } else {
                None
            };
            Some(ReplacementEffect::with_matcher(
                shield.source,
                shield.controller,
                PreventionShieldReplacementMatcher {
                    shield_id: shield.id,
                    source_snapshot: source_snapshot.cloned(),
                },
                ReplacementAction::PreventWithShield {
                    shield_id: shield.id,
                    max_amount,
                },
            ))
        })
        .collect()
}

fn affected_player_for_damage(
    game: &GameState,
    target: DamageTarget,
    fallback: PlayerId,
) -> PlayerId {
    match target {
        DamageTarget::Player(player) => player,
        DamageTarget::Object(object) => game.current_controller(object).unwrap_or(fallback),
    }
}

fn apnap_position(game: &GameState, player: PlayerId) -> usize {
    let order = game.team_apnap_player_order();
    order
        .iter()
        .position(|candidate| *candidate == player)
        .unwrap_or(order.len() + player.index())
}

fn collect_simultaneous_prevention_allocations(
    game: &GameState,
    events: &[SimultaneousDamageEvent],
    dm: &mut dyn DecisionMaker,
) -> Vec<PreventionBatchAllocation> {
    let mut allocations = vec![PreventionBatchAllocation::default(); events.len()];
    if !game.can_prevent_damage() {
        return allocations;
    }

    // Snapshot before any shield is mutated. Every allocation decision therefore
    // sees the complete simultaneous source/amount set required by CR 615.7.
    let shields = game.effect_store.prevention_effects.shields().to_vec();
    for shield in shields {
        let Some(capacity) = shield.amount_remaining.filter(|amount| *amount > 0) else {
            continue;
        };
        let matcher = PreventionShieldReplacementMatcher {
            shield_id: shield.id,
            source_snapshot: None,
        };
        let mut eligible = events
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                if item.amount == 0 || item.unpreventable {
                    return None;
                }
                let damage = crate::events::DamageEvent::with_cause(
                    item.source,
                    item.target,
                    item.amount,
                    item.is_combat,
                    item.cause.clone(),
                );
                let matcher = PreventionShieldReplacementMatcher {
                    source_snapshot: item.source_snapshot.clone(),
                    ..matcher.clone()
                };
                let ctx =
                    EventContext::for_replacement_effect(shield.controller, shield.source, game)
                        .with_event_source_snapshot(item.source_snapshot.as_ref());
                matcher.matches_event(&damage, &ctx).then_some(index)
            })
            .collect::<Vec<_>>();

        let distinct_sources = eligible
            .iter()
            .map(|index| events[*index].source)
            .collect::<std::collections::HashSet<_>>();
        let total_damage = eligible
            .iter()
            .map(|index| events[*index].amount)
            .sum::<u32>();
        if distinct_sources.len() < 2 || total_damage <= capacity {
            continue;
        }

        eligible.sort_by_key(|index| {
            let player = affected_player_for_damage(game, events[*index].target, shield.controller);
            (apnap_position(game, player), *index)
        });

        for index in &eligible {
            allocations[*index].allocated_shields.insert(shield.id);
        }

        let mut remaining = capacity.min(total_damage);
        for (position, index) in eligible.iter().copied().enumerate() {
            let later_damage = eligible[position + 1..]
                .iter()
                .map(|later| events[*later].amount)
                .sum::<u32>();
            let minimum = remaining.saturating_sub(later_damage);
            let maximum = events[index].amount.min(remaining);
            let chosen = if minimum == maximum {
                minimum
            } else {
                let player =
                    affected_player_for_damage(game, events[index].target, shield.controller);
                let source_name = game
                    .current_name(events[index].source)
                    .unwrap_or_else(|| format!("source {}", events[index].source.0));
                let spec = crate::decisions::NumberSpec::range(
                    shield.source,
                    minimum,
                    maximum,
                    format!(
                        "Choose how much of prevention shield {} applies to damage from {source_name}",
                        shield.id.0
                    ),
                );
                crate::decisions::make_decision_with_fallback(
                    game,
                    dm,
                    player,
                    Some(shield.source),
                    spec,
                    crate::decision::FallbackStrategy::Maximum,
                )
                .clamp(minimum, maximum)
            };
            if chosen > 0 {
                allocations[index].limits.insert(shield.id, chosen);
            }
            remaining = remaining.saturating_sub(chosen);
        }
    }

    allocations
}

/// Process damage events that would happen simultaneously as one CR 615.7 batch.
///
/// Limited prevention shields are allocated before any event consumes them. The
/// returned results remain aligned with the input events.
pub fn process_simultaneous_damage_assignments_with_event_with_dm(
    game: &mut GameState,
    events: &[SimultaneousDamageEvent],
    dm: &mut dyn DecisionMaker,
) -> Vec<ProcessedDamageResult> {
    game.update_cant_effects();
    game.update_replacement_effects();
    let pending_event_start = game.effect_store.pending_trigger_events.len();
    let allocations = collect_simultaneous_prevention_allocations(game, events, dm);
    let mut results = Vec::with_capacity(events.len());
    for (index, item) in events.iter().enumerate() {
        results.push(
            process_damage_assignments_with_event_with_source_snapshot_opts_with_dm_and_allocation(
                game,
                item.source,
                item.target,
                item.amount,
                item.is_combat,
                item.unpreventable,
                item.cause.clone(),
                item.source_snapshot.as_ref(),
                dm,
                Some(&allocations[index]),
            ),
        );
    }
    coalesce_simultaneous_shield_prevention_events(game, pending_event_start);
    results
}

fn coalesce_simultaneous_shield_prevention_events(game: &mut GameState, start_index: usize) {
    let removed = game.remove_pending_trigger_events_matching_from(start_index, |event| {
        event
            .downcast::<crate::events::DamagePreventedEvent>()
            .is_some_and(|prevented| prevented.prevention_shield.is_some())
    });
    let mut grouped: Vec<(
        crate::provenance::ProvNodeId,
        crate::events::DamagePreventedEvent,
    )> = Vec::new();
    for trigger_event in removed {
        let provenance = trigger_event.provenance();
        let Some(prevented) = trigger_event
            .downcast::<crate::events::DamagePreventedEvent>()
            .cloned()
        else {
            continue;
        };
        if grouped
            .iter_mut()
            .any(|(_, existing)| existing.merge_simultaneous(prevented.clone()))
        {
            continue;
        }
        grouped.push((provenance, prevented));
    }
    for (provenance, prevented) in grouped {
        game.queue_trigger_event(
            provenance,
            crate::triggers::TriggerEvent::new_with_provenance(
                prevented,
                crate::provenance::ProvNodeId::default(),
            ),
        );
    }
}

/// Deterministic convenience wrapper for a simultaneous damage batch.
pub fn process_simultaneous_damage_assignments_with_event(
    game: &mut GameState,
    events: &[SimultaneousDamageEvent],
) -> Vec<ProcessedDamageResult> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    process_simultaneous_damage_assignments_with_event_with_dm(game, events, &mut dm)
}

#[allow(clippy::too_many_arguments)]
pub fn process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    unpreventable: bool,
    cause: crate::events::cause::EventCause,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    dm: &mut dyn DecisionMaker,
) -> ProcessedDamageResult {
    process_damage_assignments_with_event_with_source_snapshot_opts_with_dm_and_allocation(
        game,
        source,
        target,
        amount,
        is_combat,
        unpreventable,
        cause,
        source_snapshot,
        dm,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn process_damage_assignments_with_event_with_source_snapshot_opts_with_dm_and_allocation(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    unpreventable: bool,
    cause: crate::events::cause::EventCause,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    dm: &mut dyn DecisionMaker,
    batch_allocation: Option<&PreventionBatchAllocation>,
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
    // CR 615.12 still applies prevention effects to unpreventable damage. They
    // prevent zero, retain their shield capacity, and perform additional parts.
    let mut prevention_effects =
        prevention_shield_replacement_effects(game, source_snapshot, batch_allocation);
    assign_ephemeral_effect_ids(&mut prevention_effects, u64::MAX / 4);
    let result = process_with_dm_and_additional_effects_and_snapshot(
        game,
        event,
        dm,
        &prevention_effects,
        source_snapshot,
    );
    execute_pending_prevention_follow_ups(game, dm);

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
            let mut exec_ctx = crate::effects::ExecutionContext::new(
                replacement_source,
                replacement_controller,
                dm,
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
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            debug_assert!(
                false,
                "damage replacement choice remained pending after decision-driven processing"
            );
            return ProcessedDamageResult {
                assignments: Vec::new(),
                replacement_prevented: true,
            };
        }
    };

    let mut assignments = Vec::new();
    let final_damage = replaced.amount;
    let source_controller = game
        .object(replaced.source)
        .map(|source| game.controller_of(source))
        .or_else(|| source_snapshot.map(|source| source.controller));
    let target_is_in_source_range = source_controller.is_none_or(|controller| {
        game.source_snapshot_is_exempt_from_range(Some(replaced.source), source_snapshot)
            || match replaced.target {
                DamageTarget::Player(player) => game.player_is_within_range(controller, player),
                DamageTarget::Object(object) => {
                    game.object_is_within_range(controller, object, Some(replaced.source))
                }
            }
    });
    if final_damage > 0 && target_is_in_source_range {
        assignments.push(ProcessedDamageAssignment {
            target: replaced.target,
            amount: final_damage,
        });
    }

    if let Some((remainder_target, remainder_amount)) = replaced.remainder
        && remainder_amount > 0
    {
        let remainder = process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(
            game,
            replaced.source,
            remainder_target,
            remainder_amount,
            replaced.is_combat,
            unpreventable,
            replaced.cause.clone(),
            source_snapshot,
            dm,
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

fn execute_pending_prevention_follow_ups(game: &mut GameState, dm: &mut dyn DecisionMaker) {
    let pending = game
        .effect_store
        .prevention_effects
        .take_pending_follow_ups();
    for pending in pending {
        let follow_up = pending.follow_up;
        let prevented_event =
            crate::events::RawEvent::new(pending.damage.clone(), pending.provenance);
        let mut exec_ctx =
            crate::effects::ExecutionContext::new(follow_up.source, follow_up.controller, &mut *dm)
                .with_triggering_event(prevented_event)
                .with_cause(crate::events::cause::EventCause::from_effect(
                    follow_up.source,
                    follow_up.controller,
                ))
                .with_provenance(pending.provenance);
        if follow_up.targets.is_empty() {
            match pending.damage.target {
                DamageTarget::Player(player_id) => exec_ctx
                    .targets
                    .push(crate::effects::ResolvedTarget::Player(player_id)),
                DamageTarget::Object(object_id) => exec_ctx
                    .targets
                    .push(crate::effects::ResolvedTarget::Object(object_id)),
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

    if game
        .turn_store
        .turn_history
        .player_counter_is_locked_this_turn(target, counter_type)
    {
        return 0;
    }

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
    process_etb_with_event_and_dm_with_initial_counters_and_reservations(
        game,
        object,
        from,
        dm,
        initial_enters_with_counters,
        None,
        &std::collections::HashSet::new(),
    )
}

pub(crate) fn process_etb_with_event_and_dm_with_initial_counters_and_controller(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    dm: &mut dyn DecisionMaker,
    initial_enters_with_counters: Vec<(CounterType, u32)>,
    entering_controller: Option<PlayerId>,
) -> EtbEventResult {
    process_etb_with_event_and_dm_with_initial_counters_and_reservations(
        game,
        object,
        from,
        dm,
        initial_enters_with_counters,
        entering_controller,
        &std::collections::HashSet::new(),
    )
}

/// Prepare one member of a simultaneous ETB event without committing its zone
/// change. Reserved objects are already changing zones in this event (or were
/// selected by another entry replacement) and cannot be selected again.
pub(crate) fn process_etb_batch_proposal_with_initial_counters(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    dm: &mut dyn DecisionMaker,
    initial_enters_with_counters: Vec<(CounterType, u32)>,
    entering_controller: Option<PlayerId>,
    reserved_objects: &std::collections::HashSet<ObjectId>,
) -> EtbEventResult {
    process_etb_with_event_and_dm_with_initial_counters_and_reservations(
        game,
        object,
        from,
        dm,
        initial_enters_with_counters,
        entering_controller,
        reserved_objects,
    )
}

fn process_etb_with_event_and_dm_with_initial_counters_and_reservations(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    dm: &mut dyn DecisionMaker,
    initial_enters_with_counters: Vec<(CounterType, u32)>,
    entering_controller: Option<PlayerId>,
    batch_reserved_objects: &std::collections::HashSet<ObjectId>,
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
    let mut reserved_objects = batch_reserved_objects.clone();

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
        if obj.card_types.contains(&CardType::Battle)
            && let Some(defense) = obj.base_defense
            && defense > 0
        {
            // Battles intrinsically enter with defense counters equal to their
            // printed defense. Keep these in the ETB proposal so ordinary
            // replacement effects can modify the event.
            enters_with_counters.push((CounterType::Defense, defense));
        }
        let controller = entering_controller.unwrap_or_else(|| game.controller_of(obj));
        let view = crate::derived_view::DerivedGameView::new(game);
        let mut current_static_abilities = view
            .static_abilities_rc(object)
            .map(|abilities| abilities.as_ref().clone())
            .unwrap_or_default();
        // The object has not entered yet, so the derived view correctly omits
        // battlefield-only static abilities.  Its own as-enters replacement
        // abilities still need to inspect the proposed zone change, however.
        // Supplement the derived set with the object's printed abilities and
        // deduplicate by stable static-ability instance identity.
        for static_ability in obj
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability),
                _ => None,
            })
        {
            if !current_static_abilities
                .iter()
                .any(|existing| existing.instance_id() == static_ability.instance_id())
            {
                current_static_abilities.push(static_ability.clone());
            }
        }
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
                    &reserved_objects,
                    &mut copy_choice_effects,
                );
            }
        }
    }

    if let Some(sparse_candidates) = game.sparse_enter_as_copy_source_abilities() {
        for (source, static_ability) in sparse_candidates.iter() {
            if *source == object {
                continue;
            }
            let Some(spec) = static_ability.enter_as_copy_as_enters() else {
                continue;
            };
            if spec.affected_filter.is_none() {
                continue;
            }
            let Some(source_obj) = game.object(*source) else {
                continue;
            };
            push_enter_as_copy_effects_for_spec(
                game,
                object,
                *source,
                game.controller_of(source_obj),
                spec,
                &reserved_objects,
                &mut copy_choice_effects,
            );
        }
    } else {
        // Ability-copying, text-changing, or relevant ability add/remove
        // effects can make the printed candidate set incomplete. Preserve the
        // fully layered path for those uncommon states.
        let view = crate::derived_view::DerivedGameView::new(game);
        view.prewarm_characteristics(&game.battlefield);
        for &source in &game.battlefield {
            if source == object {
                continue;
            }
            let Some(source_obj) = game.object(source) else {
                continue;
            };
            let controller = game.controller_of(source_obj);
            let static_abilities = view.static_abilities_rc(source).unwrap_or_else(|| {
                std::rc::Rc::new(
                    source_obj
                        .abilities
                        .iter()
                        .filter_map(|ability| match &ability.kind {
                            AbilityKind::Static(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect(),
                )
            });
            for static_ability in static_abilities.iter() {
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
                    &reserved_objects,
                    &mut copy_choice_effects,
                );
            }
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
            copy_duration: None,
            copy_name_override: None,
            added_colors: crate::color::ColorSet::new(),
            added_card_types: Vec::new(),
            removed_supertypes: Vec::new(),
            added_subtypes: Vec::new(),
            added_abilities: Vec::new(),
            set_base_power_toughness: None,
            controller_override: entering_controller,
            prepared_choices: None,
        },
        etb_event_provenance,
    );
    let mut state = TraitEventProcessingState::default();
    let mut paid_labels = Vec::new();

    loop {
        if let Some(etb) = downcast_event::<EnterBattlefieldEvent>(current_event.inner()) {
            reserved_objects.extend(etb.linked_exile_with_entering.iter().copied());
        }
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
                    // Copy-as-enters effects are applied before other ETB
                    // modifications. Recompute a battle's intrinsic defense
                    // proposal from the copied values so effects such as
                    // Doubling Season can still modify those counters.
                    if !copy_choice_consumed && let Some(copy_source) = etb.enters_as_copy_of {
                        let mut copied_etb = etb.clone();
                        copied_etb
                            .enters_with_counters
                            .retain(|(counter, _)| *counter != CounterType::Defense);
                        if let Some(defense) = game
                            .object(copy_source)
                            .filter(|source| source.card_types.contains(&CardType::Battle))
                            .and_then(|source| source.base_defense)
                            .filter(|defense| *defense > 0)
                        {
                            copied_etb
                                .enters_with_counters
                                .push((CounterType::Defense, defense));
                        }
                        current_event = Event::new_with_provenance(copied_etb, e.provenance());
                        continue;
                    }
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
                    let event_result = EtbEventResult {
                        enters_tapped: etb.enters_tapped,
                        enters_with_counters: etb.enters_with_counters.clone(),
                        linked_exile_with_entering: etb.linked_exile_with_entering.clone(),
                        prevented: false,
                        new_destination: None,
                        enters_as_copy_of: etb.enters_as_copy_of,
                        copy_duration: etb.copy_duration.clone(),
                        copy_name_override: etb.copy_name_override.clone(),
                        added_colors: etb.added_colors,
                        added_card_types: etb.added_card_types.clone(),
                        removed_supertypes: etb.removed_supertypes.clone(),
                        added_subtypes: etb.added_subtypes.clone(),
                        added_abilities: etb.added_abilities.clone(),
                        set_base_power_toughness: etb.set_base_power_toughness,
                        controller_override: etb.controller_override,
                        prepared_choices: etb.prepared_choices.clone(),
                        paid_labels: paid_labels.clone(),
                        interactive_replacement: None,
                    };
                    if etb.prepared_choices.is_none() {
                        let Some(prepared) = game.prepare_etb_entry_with_controller_and_dm(
                            object,
                            event_result,
                            etb.controller_override,
                            dm,
                        ) else {
                            return EtbEventResult {
                                prevented: true,
                                ..Default::default()
                            };
                        };
                        let mut prepared_event = etb.clone();
                        prepared_event.prepared_choices = Some(prepared.choices);
                        current_event = Event::new_with_provenance(prepared_event, e.provenance());
                        continue;
                    }
                    return event_result;
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

                mark_applied_replacement_choice(&mut state, &chosen_effect);
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
                        sacrifice_count,
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
                            crate::decisions::context::DecisionContext::SelectObjects(mut ctx) => {
                                ctx.candidates
                                    .retain(|candidate| !reserved_objects.contains(&candidate.id));
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
                            sacrifice_count,
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
                sacrifice_count,
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
                    crate::decisions::context::DecisionContext::SelectObjects(mut ctx) => {
                        ctx.candidates
                            .retain(|candidate| !reserved_objects.contains(&candidate.id));
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
                    sacrifice_count,
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

    mark_applied_replacement_choice(&mut state, &effect);

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
            sacrifice_count,
            destinations,
        } => TraitEventResult::NeedsInteraction {
            decision_ctx,
            redirect_zone,
            effect_id,
            object_id,
            event: Box::new(event),
            filter,
            sacrifice_count,
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
    use crate::target::{ChooseSpec, ObjectFilter};
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

    fn create_creature_in_zone(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        zone: Zone,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, controller, zone)
    }

    fn external_enter_as_copy_ability() -> StaticAbility {
        StaticAbility::with_enter_as_copy_as_enters(
            crate::static_abilities::EnterAsCopyAsEntersSpec {
                filter: crate::target::ObjectFilter::source(),
                affected_filter: Some(crate::target::ObjectFilter::creature()),
                may: false,
                enters_tapped_if_chosen: false,
                copy_duration: None,
                linked_exile_pair: None,
                copy_source_self: true,
                copy_source_enchanted: false,
                name_override: None,
                added_colors: crate::color::ColorSet::new(),
                added_card_types: Vec::new(),
                removed_supertypes: Vec::new(),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
                set_base_power_toughness: None,
                set_base_power_toughness_from_self: false,
            },
            "Creatures enter as a copy of this creature.".to_string(),
        )
    }

    fn create_noncreature_in_zone(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        zone: Zone,
        card_type: CardType,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .build();
        game.create_object_from_card(&card, controller, zone)
    }

    #[test]
    fn later_etb_replacement_sees_characteristics_added_by_an_earlier_replacement() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_noncreature_in_zone(
            &mut game,
            "Characteristic Source",
            alice,
            Zone::Battlefield,
            CardType::Enchantment,
        );
        let entering = create_noncreature_in_zone(
            &mut game,
            "Entering Relic",
            alice,
            Zone::Hand,
            CardType::Artifact,
        );
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any(),
                ReplacementAction::EnterWithCharacteristics {
                    added_card_types: vec![CardType::Creature],
                    added_subtypes: Vec::new(),
                    set_base_power_toughness: Some((2, 2)),
                },
            ));
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::enters_tapped(
                source,
                alice,
                ObjectFilter::creature(),
            ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm);

        assert!(
            result.enters_tapped,
            "the creature-only replacement must be re-evaluated against the evolving ETB event"
        );
        assert!(result.added_card_types.contains(&CardType::Creature));
    }

    #[test]
    fn prospective_etb_matching_does_not_invent_an_unapplied_characteristic() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_noncreature_in_zone(
            &mut game,
            "Negative Characteristic Source",
            alice,
            Zone::Battlefield,
            CardType::Enchantment,
        );
        let entering = create_noncreature_in_zone(
            &mut game,
            "Entering Noncreature",
            alice,
            Zone::Hand,
            CardType::Artifact,
        );
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::enters_tapped(
                source,
                alice,
                ObjectFilter::creature(),
            ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm);

        assert!(!result.enters_tapped);
        assert!(!result.added_card_types.contains(&CardType::Creature));
    }

    #[test]
    fn copy_priority_changes_later_etb_replacement_applicability() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let copy_source = create_creature(&mut game, "Prospective Copy Creature", alice);
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::enters_tapped(
                copy_source,
                alice,
                ObjectFilter::creature(),
            ));
        let entering = create_noncreature_in_zone(
            &mut game,
            "Entering Copy Shell",
            alice,
            Zone::Hand,
            CardType::Artifact,
        );
        game.object_mut(entering)
            .expect("entering object should exist")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                StaticAbility::with_enter_as_copy_as_enters(
                    crate::static_abilities::EnterAsCopyAsEntersSpec {
                        filter: ObjectFilter::creature(),
                        affected_filter: None,
                        may: false,
                        enters_tapped_if_chosen: false,
                        copy_duration: None,
                        linked_exile_pair: None,
                        copy_source_self: false,
                        copy_source_enchanted: false,
                        name_override: None,
                        added_colors: crate::color::ColorSet::new(),
                        added_card_types: Vec::new(),
                        removed_supertypes: Vec::new(),
                        added_subtypes: Vec::new(),
                        added_abilities: Vec::new(),
                        set_base_power_toughness: None,
                        set_base_power_toughness_from_self: false,
                    },
                    "This permanent enters as a copy of a creature.".to_string(),
                ),
            ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm);

        assert_eq!(result.enters_as_copy_of, Some(copy_source));
        assert!(
            result.enters_tapped,
            "the ordinary replacement must match after the higher-priority copy replacement"
        );
    }

    #[test]
    fn entrants_own_battlefield_static_effect_is_used_for_etb_matching() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_noncreature_in_zone(
            &mut game,
            "Creature Entry Watcher",
            alice,
            Zone::Battlefield,
            CardType::Enchantment,
        );
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::enters_tapped(
                source,
                alice,
                ObjectFilter::creature(),
            ));
        let entering = create_noncreature_in_zone(
            &mut game,
            "Self-Animating Relic",
            alice,
            Zone::Hand,
            CardType::Artifact,
        );
        game.object_mut(entering)
            .expect("entering object should exist")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                StaticAbility::add_card_types(ObjectFilter::source(), vec![CardType::Creature]),
            ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm);

        assert!(
            result.enters_tapped,
            "the entrant's own static effect must animate it in the prospective battlefield view"
        );
    }

    #[test]
    fn existing_continuous_effect_is_used_for_prospective_etb_matching() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_noncreature_in_zone(
            &mut game,
            "Existing Animation Source",
            alice,
            Zone::Battlefield,
            CardType::Enchantment,
        );
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                source,
                alice,
                crate::continuous::EffectTarget::Filter(
                    ObjectFilter::artifact().in_zone(Zone::Battlefield),
                ),
                crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
            ));
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::enters_tapped(
                source,
                alice,
                ObjectFilter::creature(),
            ));
        let entering = create_noncreature_in_zone(
            &mut game,
            "Continuously Animated Relic",
            alice,
            Zone::Hand,
            CardType::Artifact,
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm);

        assert!(
            result.enters_tapped,
            "continuous effects already present must apply to the provisional battlefield object"
        );
    }

    #[test]
    fn control_change_priority_changes_later_etb_replacement_applicability() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_noncreature_in_zone(
            &mut game,
            "Bob's Entry Source",
            bob,
            Zone::Battlefield,
            CardType::Enchantment,
        );
        game.effect_store.replacement_effects.add_effect(
            ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any(),
                ReplacementAction::EnterUnderControl(bob),
            )
            .with_priority_override(crate::events::ReplacementPriority::ControlChanging),
        );
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::enters_tapped(
                source,
                bob,
                ObjectFilter::creature().you_control(),
            ));
        let entering = create_creature_in_zone(
            &mut game,
            "Changing-Control Entrant",
            alice,
            Zone::Hand,
            2,
            2,
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm);

        assert_eq!(result.controller_override, Some(bob));
        assert!(
            result.enters_tapped,
            "the later controller-relative filter must see the higher-priority control change"
        );
    }

    #[test]
    fn prepared_as_enters_choice_changes_later_replacement_applicability() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_noncreature_in_zone(
            &mut game,
            "Chosen Characteristic Watcher",
            alice,
            Zone::Battlefield,
            CardType::Enchantment,
        );
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::enters_tapped(
                source,
                alice,
                ObjectFilter::creature(),
            ));
        let entering = create_noncreature_in_zone(
            &mut game,
            "Choice-Animated Relic",
            alice,
            Zone::Hand,
            CardType::Artifact,
        );
        game.object_mut(entering)
            .expect("entering object should exist")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                StaticAbility::choose_power_toughness_options_as_enters_or_turns_face_up(
                    vec![
                        crate::static_abilities::PowerToughnessChoiceOption::with_abilities(
                            2,
                            2,
                            vec![StaticAbility::add_card_types(
                                ObjectFilter::source(),
                                vec![CardType::Creature],
                            )],
                        ),
                    ],
                    "As this enters, choose its characteristics.".to_string(),
                ),
            ));

        let result = game
            .move_object_with_etb_processing(entering, Zone::Battlefield)
            .expect("the prepared entry should commit");

        assert!(
            result.enters_tapped,
            "the later matcher must see abilities granted by the pre-entry choice"
        );
        assert!(
            game.current_card_types(result.new_id)
                .is_some_and(|types| types.contains(&CardType::Creature))
        );
    }

    #[test]
    fn absent_external_enter_as_copy_sources_skip_layered_battlefield_scan() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let creatures = (0..96)
            .map(|index| {
                create_creature(
                    &mut game,
                    &format!("Unrelated Layered Creature {index}"),
                    alice,
                )
            })
            .collect::<Vec<_>>();
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                creatures[0],
                alice,
                crate::continuous::EffectTarget::AllCreatures,
                crate::continuous::Modification::AddAbility(StaticAbility::flying()),
            ));
        game.refresh_continuous_state();
        let entering = create_creature_in_zone(&mut game, "Entering Bear", alice, Zone::Hand, 2, 2);
        game.refresh_continuous_state();
        // ETB processing must inspect the entering object's own characteristics
        // once for replacement abilities. Warm that required lookup so the
        // counter window below isolates external battlefield-source discovery.
        game.prewarm_calculated_characteristics(&[entering]);
        let before = game.work_counters();
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let result = process_etb_with_event_and_dm_with_initial_counters(
            &mut game,
            entering,
            Zone::Hand,
            &mut dm,
            Vec::new(),
        );

        assert_eq!(result.enters_as_copy_of, None);
        let after = game.work_counters();
        assert_eq!(
            after.characteristics_full_recomputes, before.characteristics_full_recomputes,
            "irrelevant ability grants must not force characteristics for every permanent"
        );
        assert_eq!(
            after.dependency_sorts, before.dependency_sorts,
            "irrelevant ability grants must not enter dependency sorting"
        );

        let first = game
            .sparse_enter_as_copy_source_abilities()
            .expect("irrelevant grants should permit the sparse path");
        let second = game
            .sparse_enter_as_copy_source_abilities()
            .expect("the sparse result should stay cacheable");
        assert!(first.is_empty());
        assert!(std::sync::Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn continuously_granted_external_enter_as_copy_uses_layered_fallback() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature_in_zone(
            &mut game,
            "Granted Copy Source",
            alice,
            Zone::Battlefield,
            6,
            6,
        );
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                source,
                alice,
                crate::continuous::EffectTarget::Specific(source),
                crate::continuous::Modification::AddAbility(external_enter_as_copy_ability()),
            ));
        let entering = create_creature_in_zone(&mut game, "Entering Bear", alice, Zone::Hand, 2, 2);

        let result = game
            .move_object_with_etb_processing(entering, Zone::Battlefield)
            .expect("the creature should enter");
        let entered = game
            .object(result.new_id)
            .expect("entered object should exist");

        assert_eq!(entered.name, "Granted Copy Source");
        assert_eq!(entered.base_power, Some(crate::card::PtValue::Fixed(6)));
        assert_eq!(entered.base_toughness, Some(crate::card::PtValue::Fixed(6)));
    }

    #[test]
    fn continuously_removed_external_enter_as_copy_uses_layered_fallback() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature_in_zone(
            &mut game,
            "Removed Copy Source",
            alice,
            Zone::Battlefield,
            6,
            6,
        );
        let copy_ability = external_enter_as_copy_ability();
        game.object_mut(source)
            .expect("copy source should exist")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                copy_ability.clone(),
            ));
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                source,
                alice,
                crate::continuous::EffectTarget::Specific(source),
                crate::continuous::Modification::RemoveAbility(copy_ability),
            ));
        let entering = create_creature_in_zone(&mut game, "Entering Bear", alice, Zone::Hand, 2, 2);

        let result = game
            .move_object_with_etb_processing(entering, Zone::Battlefield)
            .expect("the creature should enter");
        let entered = game
            .object(result.new_id)
            .expect("entered object should exist");

        assert_eq!(entered.name, "Entering Bear");
        assert_eq!(entered.base_power, Some(crate::card::PtValue::Fixed(2)));
        assert_eq!(entered.base_toughness, Some(crate::card::PtValue::Fixed(2)));
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
    fn damage_reduced_to_zero_is_not_offered_to_later_replacements() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let damage_source = create_creature(&mut game, "Damage Source", alice);
        let zero_source = create_creature(&mut game, "Zero Replacement", alice);
        let revive_source = create_creature(&mut game, "Later Replacement", alice);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                zero_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Subtract(3)),
            )
            .with_priority_override(crate::events::traits::ReplacementPriority::SelfReplacement),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                revive_source,
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
                3,
                false,
                EventCause::effect(),
            ),
        );
        let event = match result {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => event,
            other => panic!("zero damage should terminate as a removed event, got {other:?}"),
        };
        let damage = crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
            .expect("the removed damage carrier should retain its event type");
        assert_eq!(
            damage.amount, 0,
            "the later +100 replacement must not revive a zeroed damage event"
        );

        let processed = process_damage_assignments_with_event(
            &mut game,
            damage_source,
            DamageTarget::Player(bob),
            3,
            false,
            EventCause::effect(),
        );
        assert!(processed.assignments.is_empty());
    }

    #[test]
    fn distinct_identical_static_abilities_each_replace_the_same_event_once() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Double Modifier", alice);

        for _ in 0..2 {
            game.object_mut(source)
                .expect("replacement source should exist")
                .abilities_mut()
                .push(crate::ability::Ability::static_ability(
                    StaticAbility::modify_damage_amount_replacement(
                        crate::target::ObjectFilter::default().you_control(),
                        Some(crate::target::PlayerFilter::Opponent),
                        None,
                        1,
                        "If a source you control would deal damage to an opponent, it deals that much damage plus 1 instead."
                            .to_string(),
                    ),
                ));
        }
        game.update_replacement_effects();

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = process_with_dm(
            &mut game,
            Event::damage(
                source,
                DamageTarget::Player(bob),
                2,
                false,
                EventCause::effect(),
            ),
            &mut dm,
        );
        let event = match result {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => event,
            other => panic!("both replacements should finish processing, got {other:?}"),
        };
        let damage = crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
            .expect("processed event should remain damage");
        assert_eq!(
            damage.amount, 4,
            "each distinct +1 static ability must apply exactly once"
        );
    }

    #[test]
    fn tied_damage_replacements_use_affected_players_choice_without_restoring_original() {
        struct ChooseLastReplacement {
            expected_player: PlayerId,
        }

        impl crate::decision::DecisionMaker for ChooseLastReplacement {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, self.expected_player);
                ctx.options
                    .iter()
                    .rev()
                    .find(|option| option.legal)
                    .map(|option| vec![option.index])
                    .unwrap_or_default()
            }
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let damage_source = create_creature(&mut game, "Damage Source", alice);
        let add_source = create_creature(&mut game, "Add Replacement", alice);
        let double_source = create_creature(&mut game, "Double Replacement", alice);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                add_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Add(1)),
            ),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                double_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );

        let mut dm = ChooseLastReplacement {
            expected_player: bob,
        };
        let processed = process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(
            &mut game,
            damage_source,
            DamageTarget::Player(bob),
            3,
            false,
            false,
            EventCause::effect(),
            None,
            &mut dm,
        );
        assert_eq!(
            processed.assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Player(bob),
                amount: 7,
            }],
            "choosing double before +1 should produce seven damage, never restore the original three"
        );
    }

    #[test]
    fn prevention_shield_and_damage_replacement_share_affected_players_ordering() {
        struct ChooseShieldReplacement {
            expected_player: PlayerId,
            shield_source: ObjectId,
        }

        impl crate::decision::DecisionMaker for ChooseShieldReplacement {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, self.expected_player);
                ctx.options
                    .iter()
                    .find(|option| option.legal && option.object_id == Some(self.shield_source))
                    .map(|option| vec![option.index])
                    .unwrap_or_default()
            }
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let damage_source = create_creature(&mut game, "Damage Source", alice);
        let replacement_source = create_creature(&mut game, "Double Replacement", alice);
        let shield_source = create_creature(&mut game, "Prevention Shield", bob);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                replacement_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );
        game.effect_store
            .prevention_effects
            .add_shield(PreventionShield::prevent_next_n(
                shield_source,
                bob,
                PreventionTarget::Player(bob),
                1,
            ));

        let mut dm = ChooseShieldReplacement {
            expected_player: bob,
            shield_source,
        };
        let processed = process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(
            &mut game,
            damage_source,
            DamageTarget::Player(bob),
            3,
            false,
            false,
            EventCause::effect(),
            None,
            &mut dm,
        );
        assert_eq!(
            processed.assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Player(bob),
                amount: 4,
            }],
            "choosing prevention before doubling must produce (3 - 1) * 2 damage"
        );
    }

    #[test]
    fn limited_prevention_shields_are_consumed_in_affected_players_chosen_order() {
        struct ChooseLastReplacement(PlayerId);

        impl crate::decision::DecisionMaker for ChooseLastReplacement {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, self.0);
                ctx.options
                    .iter()
                    .rev()
                    .find(|option| option.legal)
                    .map(|option| vec![option.index])
                    .unwrap_or_default()
            }
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let damage_source = create_creature(&mut game, "Damage Source", alice);
        let first_source = create_creature(&mut game, "One Point Shield", bob);
        let second_source = create_creature(&mut game, "Three Point Shield", bob);
        let first_id =
            game.effect_store
                .prevention_effects
                .add_shield(PreventionShield::prevent_next_n(
                    first_source,
                    bob,
                    PreventionTarget::Player(bob),
                    1,
                ));
        let second_id =
            game.effect_store
                .prevention_effects
                .add_shield(PreventionShield::prevent_next_n(
                    second_source,
                    bob,
                    PreventionTarget::Player(bob),
                    3,
                ));

        let mut dm = ChooseLastReplacement(bob);
        let processed = process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(
            &mut game,
            damage_source,
            DamageTarget::Player(bob),
            3,
            false,
            false,
            EventCause::effect(),
            None,
            &mut dm,
        );
        assert!(processed.assignments.is_empty());
        assert_eq!(
            game.effect_store.prevention_effects.shields().len(),
            1,
            "the chosen three-point shield should exhaust before the earlier shield is touched"
        );
        assert_eq!(
            game.effect_store.prevention_effects.shields()[0].id,
            first_id
        );
        assert_eq!(
            game.effect_store.prevention_effects.shields()[0].amount_remaining,
            Some(1)
        );
        assert_ne!(first_id, second_id);
    }

    #[test]
    fn limited_shield_is_allocated_across_simultaneous_damage_sources_before_commit() {
        struct AllocateToLaterSource {
            expected_player: PlayerId,
            decisions: usize,
        }

        impl crate::decision::DecisionMaker for AllocateToLaterSource {
            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                assert_eq!(ctx.player, self.expected_player);
                self.decisions += 1;
                ctx.min
            }
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let earlier_source = create_creature(&mut game, "Earlier Source", alice);
        let later_source = create_creature(&mut game, "Later Source", alice);
        let shield_source = create_creature(&mut game, "Limited Shield", bob);
        game.effect_store
            .prevention_effects
            .add_shield(PreventionShield::prevent_next_n(
                shield_source,
                bob,
                PreventionTarget::Player(bob),
                2,
            ));

        let events = vec![
            SimultaneousDamageEvent {
                source: earlier_source,
                target: DamageTarget::Player(bob),
                amount: 3,
                is_combat: true,
                unpreventable: false,
                cause: EventCause::combat_damage(earlier_source),
                source_snapshot: None,
            },
            SimultaneousDamageEvent {
                source: later_source,
                target: DamageTarget::Player(bob),
                amount: 3,
                is_combat: true,
                unpreventable: false,
                cause: EventCause::combat_damage(later_source),
                source_snapshot: None,
            },
        ];
        let mut dm = AllocateToLaterSource {
            expected_player: bob,
            decisions: 0,
        };
        let processed =
            process_simultaneous_damage_assignments_with_event_with_dm(&mut game, &events, &mut dm);

        assert_eq!(
            dm.decisions, 1,
            "the final constrained allocation is automatic"
        );
        assert_eq!(
            processed[0].assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Player(bob),
                amount: 3,
            }],
            "the affected player allocated no shield to the earlier source"
        );
        assert_eq!(
            processed[1].assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Player(bob),
                amount: 1,
            }],
            "the full two-point shield allocation must apply to the later source"
        );
        assert!(
            game.effect_store.prevention_effects.shields().is_empty(),
            "the allocated shield capacity should be committed exactly once"
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
    fn applying_a_prevention_effect_emits_one_prevented_damage_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let protected = create_creature(&mut game, "Protected Bear", alice);
        let damage_source = create_creature(&mut game, "Shock Bear", bob);
        let prevention_source = create_creature(&mut game, "Shield Bear", alice);
        game.object_mut(prevention_source)
            .expect("prevention source")
            .abilities_mut()
            .push(crate::ability::Ability::triggered(
                crate::triggers::Trigger::damage_prevented(),
                vec![],
            ));
        game.effect_store
            .prevention_effects
            .add_shield(PreventionShield::prevent_next_n(
                prevention_source,
                alice,
                PreventionTarget::Permanent(protected),
                2,
            ));

        let processed = process_damage_assignments_with_event(
            &mut game,
            damage_source,
            DamageTarget::Object(protected),
            3,
            false,
            EventCause::effect(),
        );

        assert_eq!(
            processed.assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Object(protected),
                amount: 1,
            }]
        );
        let events = game.take_pending_trigger_events();
        assert_eq!(events.len(), 1, "one prevention application event");
        let event = events[0]
            .downcast::<crate::events::DamagePreventedEvent>()
            .expect("typed prevented-damage event");
        assert_eq!(event.damage_source, damage_source);
        assert_eq!(event.target, DamageTarget::Object(protected));
        assert_eq!(event.amount, 2);
        assert_eq!(event.prevention_source, prevention_source);
        assert_eq!(
            crate::triggers::check_triggers(&game, &events[0]).len(),
            1,
            "the event must be consumable by downstream triggers"
        );
    }

    #[test]
    fn one_shield_applied_across_simultaneous_damage_emits_one_prevention_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let damage_source_one = create_creature(&mut game, "First Attacker", bob);
        let damage_source_two = create_creature(&mut game, "Second Attacker", bob);
        let prevention_source = create_creature(&mut game, "Shared Shield", alice);
        game.effect_store
            .prevention_effects
            .add_shield(PreventionShield::prevent_next_n(
                prevention_source,
                alice,
                PreventionTarget::Player(alice),
                4,
            ));

        let results = process_simultaneous_damage_assignments_with_event(
            &mut game,
            &[
                SimultaneousDamageEvent {
                    source: damage_source_one,
                    target: DamageTarget::Player(alice),
                    amount: 3,
                    is_combat: true,
                    unpreventable: false,
                    cause: EventCause::effect(),
                    source_snapshot: None,
                },
                SimultaneousDamageEvent {
                    source: damage_source_two,
                    target: DamageTarget::Player(alice),
                    amount: 3,
                    is_combat: true,
                    unpreventable: false,
                    cause: EventCause::effect(),
                    source_snapshot: None,
                },
            ],
        );

        assert_eq!(results[0].assignments, Vec::new());
        assert_eq!(
            results[1].assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Player(alice),
                amount: 2,
            }]
        );
        let prevented = game
            .take_pending_trigger_events()
            .into_iter()
            .filter_map(|event| {
                event
                    .downcast::<crate::events::DamagePreventedEvent>()
                    .cloned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            prevented.len(),
            1,
            "CR 615.13 triggers once when one prevention effect is applied across simultaneous damage events"
        );
        assert_eq!(prevented[0].amount, 4);
        assert_eq!(prevented[0].applications.len(), 2);
        assert_eq!(
            prevented[0].applications[0].damage_source,
            damage_source_one
        );
        assert_eq!(prevented[0].applications[0].amount, 3);
        assert_eq!(
            prevented[0].applications[1].damage_source,
            damage_source_two
        );
        assert_eq!(prevented[0].applications[1].amount, 1);
    }

    #[test]
    fn static_partial_prevention_emits_the_amount_actually_prevented() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let prevention_source = create_creature(&mut game, "Defending Cleric", alice);
        game.object_mut(prevention_source)
            .expect("prevention source")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                StaticAbility::prevent_damage_to_you_from_source_filter(
                    1,
                    ObjectFilter::creature(),
                    "If a creature would deal damage to you, prevent 1 of that damage.",
                ),
            ));
        let damage_source = create_creature(&mut game, "Attacking Bear", bob);

        let processed = process_damage_assignments_with_event(
            &mut game,
            damage_source,
            DamageTarget::Player(alice),
            3,
            true,
            EventCause::effect(),
        );

        assert_eq!(
            processed.assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Player(alice),
                amount: 2,
            }]
        );
        let events = game.take_pending_trigger_events();
        let prevented = events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::DamagePreventedEvent>())
            .collect::<Vec<_>>();
        assert_eq!(prevented.len(), 1);
        assert_eq!(prevented[0].amount, 1);
        assert_eq!(prevented[0].prevention_source, prevention_source);
    }

    #[test]
    fn unpreventable_damage_keeps_shield_and_executes_follow_up_once() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let protected = create_creature(&mut game, "Protected Bear", alice);
        let source = create_creature(&mut game, "Unstoppable Bear", bob);
        let shield = PreventionShield::prevent_next_n(
            source,
            alice,
            PreventionTarget::Permanent(protected),
            3,
        )
        .with_follow_up_effects(vec![
            Effect::new(crate::effects::PutCountersEffect::new(
                CounterType::PlusOnePlusOne,
                Value::Fixed(1),
                ChooseSpec::AnyTarget,
            )),
            Effect::new(crate::effects::PutCountersEffect::new(
                CounterType::Ice,
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::AnyTarget,
            )),
        ]);
        game.effect_store.prevention_effects.add_shield(shield);

        for expected_counters in 1..=2 {
            let processed = process_damage_assignments_with_event_with_source_snapshot_opts(
                &mut game,
                source,
                DamageTarget::Object(protected),
                3,
                false,
                true,
                EventCause::effect(),
                None,
            );

            assert_eq!(
                processed.assignments,
                vec![ProcessedDamageAssignment {
                    target: DamageTarget::Object(protected),
                    amount: 3,
                }],
                "CR 615.12 must not prevent unpreventable damage"
            );
            assert_eq!(
                game.counter_count(protected, CounterType::PlusOnePlusOne),
                expected_counters,
                "the shield's unconditional follow-up should happen once per damage event"
            );
            assert_eq!(
                game.counter_count(protected, CounterType::Ice),
                0,
                "the amount prevented is zero for an unpreventable damage event"
            );
            assert_eq!(
                game.effect_store.prevention_effects.shields()[0].amount_remaining,
                Some(3),
                "unpreventable damage must not consume the shield"
            );
            assert!(
                game.take_pending_trigger_events().iter().all(|event| event
                    .downcast::<crate::events::DamagePreventedEvent>()
                    .is_none()),
                "a prevention effect that prevents zero damage must not emit the CR 615.13 event"
            );
        }
    }

    #[test]
    fn filter_based_prevention_shield_only_protects_matching_permanents() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let white_card = CardBuilder::new(CardId::new(), "White Bear")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::White]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let black_card = CardBuilder::new(CardId::new(), "Black Bear")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let white = game.create_object_from_card(&white_card, alice, Zone::Battlefield);
        let black = game.create_object_from_card(&black_card, alice, Zone::Battlefield);
        let source = create_creature(&mut game, "Damage Source", bob);
        let shield = PreventionShield::prevent_all(
            source,
            alice,
            PreventionTarget::PermanentsMatching(
                crate::target::ObjectFilter::creature().with_colors(crate::color::ColorSet::WHITE),
            ),
        );
        game.effect_store.prevention_effects.add_shield(shield);

        let excluded = process_damage_assignments_with_event(
            &mut game,
            source,
            DamageTarget::Object(black),
            3,
            false,
            EventCause::effect(),
        );
        assert_eq!(
            excluded.assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Object(black),
                amount: 3,
            }],
            "the white-permanent shield must not prevent damage to a black permanent"
        );

        let included = process_damage_assignments_with_event(
            &mut game,
            source,
            DamageTarget::Object(white),
            3,
            false,
            EventCause::effect(),
        );
        assert!(
            included.assignments.is_empty(),
            "the shield should prevent damage to the matching white permanent"
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
        let prevented = game
            .take_pending_trigger_events()
            .into_iter()
            .filter_map(|event| {
                event
                    .downcast::<crate::events::DamagePreventedEvent>()
                    .cloned()
            })
            .collect::<Vec<_>>();
        assert_eq!(prevented.len(), 1);
        assert_eq!(prevented[0].amount, 3);
        assert_eq!(prevented[0].prevention_source, protected);

        let unpreventable = process_damage_assignments_with_event_with_source_snapshot_opts(
            &mut game,
            source,
            DamageTarget::Object(protected),
            3,
            false,
            true,
            EventCause::effect(),
            None,
        );
        assert_eq!(
            unpreventable.assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Object(protected),
                amount: 3,
            }]
        );
        assert_eq!(
            game.counter_count(protected, CounterType::PlusOnePlusOne),
            3,
            "the additional part still runs with zero damage prevented"
        );
        assert!(
            game.take_pending_trigger_events().iter().all(|event| event
                .downcast::<crate::events::DamagePreventedEvent>()
                .is_none()),
            "zero prevented damage must not emit CR 615.13"
        );
    }

    #[test]
    fn combined_to_and_from_self_prevention_stops_both_damage_directions() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let protected = create_creature(&mut game, "Lightbound Creature", alice);
        game.object_mut(protected)
            .expect("protected creature should exist")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                StaticAbility::prevent_all_damage_dealt_to_and_by_this_permanent(),
            ));
        let other = create_creature(&mut game, "Other Damage Source", bob);

        let dealt_to = process_damage_assignments_with_event(
            &mut game,
            other,
            DamageTarget::Object(protected),
            3,
            false,
            EventCause::effect(),
        );
        assert!(
            dealt_to.assignments.is_empty() && dealt_to.replacement_prevented,
            "preventable damage dealt to the protected permanent should be stopped: {dealt_to:?}"
        );

        let dealt_by = process_damage_assignments_with_event(
            &mut game,
            protected,
            DamageTarget::Player(bob),
            3,
            false,
            EventCause::effect(),
        );
        assert!(
            dealt_by.assignments.is_empty() && dealt_by.replacement_prevented,
            "preventable damage dealt by the protected permanent should be stopped: {dealt_by:?}"
        );

        let unrelated = process_damage_assignments_with_event(
            &mut game,
            other,
            DamageTarget::Player(alice),
            3,
            false,
            EventCause::effect(),
        );
        assert_eq!(
            unrelated.assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Player(alice),
                amount: 3,
            }],
            "unrelated damage must proceed"
        );

        let unpreventable = process_damage_assignments_with_event_with_source_snapshot_opts(
            &mut game,
            protected,
            DamageTarget::Player(bob),
            4,
            false,
            true,
            EventCause::effect(),
            None,
        );
        assert_eq!(
            unpreventable.assignments,
            vec![ProcessedDamageAssignment {
                target: DamageTarget::Player(bob),
                amount: 4,
            }],
            "unpreventable damage dealt by the permanent must still proceed"
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

    #[test]
    fn prevention_source_properties_use_calculated_characteristics_and_lki() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let damage_source = create_creature(&mut game, "Colorless Sparkmage", alice);
        let color_source = create_creature(&mut game, "Color Setter", alice);
        let shield_source = create_creature(&mut game, "Red Ward", bob);

        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                color_source,
                alice,
                crate::continuous::EffectTarget::Specific(damage_source),
                crate::continuous::Modification::SetColors(crate::color::ColorSet::RED),
            ));
        game.refresh_continuous_state();
        assert_eq!(
            game.calculated_characteristics(damage_source)
                .expect("damage source should have calculated characteristics")
                .colors,
            crate::color::ColorSet::RED
        );

        game.effect_store.prevention_effects.add_shield(
            PreventionShield::prevent_all(shield_source, bob, PreventionTarget::Player(bob))
                .with_filter(crate::prevention::DamageFilter::from_color(
                    crate::color::Color::Red,
                )),
        );

        let current = process_damage_assignments_with_event(
            &mut game,
            damage_source,
            DamageTarget::Player(bob),
            2,
            false,
            EventCause::effect(),
        );
        assert!(
            current.assignments.is_empty(),
            "a source made red by a continuous effect must match red-source prevention"
        );

        let source_snapshot =
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(damage_source)
                    .expect("source should still exist"),
                &game,
            );
        game.move_object(damage_source, Zone::Graveyard, EventCause::effect())
            .expect("damage source should move");
        let departed = process_damage_assignments_with_event_with_source_snapshot(
            &mut game,
            damage_source,
            DamageTarget::Player(bob),
            2,
            false,
            EventCause::effect(),
            Some(&source_snapshot),
        );
        assert!(
            departed.assignments.is_empty(),
            "red calculated characteristics captured in source LKI must keep matching prevention"
        );
    }

    #[test]
    fn intrinsic_battle_defense_counters_are_modified_by_etb_replacements() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let doubler = create_creature(&mut game, "Defense Doubler", alice);
        game.object_mut(doubler)
            .expect("counter doubler")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                StaticAbility::double_counters_replacement(
                    crate::target::ObjectFilter::permanent(),
                    Some(CounterType::Defense),
                    "If counters would be put on a permanent, put twice that many instead."
                        .to_string(),
                ),
            ));

        let siege_card = CardBuilder::new(CardId::new(), "Doubled Siege")
            .card_types(vec![CardType::Battle])
            .subtypes(vec![crate::types::Subtype::Siege])
            .defense(4)
            .build();
        let siege = game.create_object_from_card(&siege_card, alice, Zone::Hand);
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        let entered = game
            .move_object_with_etb_processing_with_dm(siege, Zone::Battlefield, &mut decision_maker)
            .expect("the Siege should enter");

        assert_eq!(
            game.counter_count(entered.new_id, CounterType::Defense),
            8,
            "the intrinsic defense-counter replacement must use the ordinary ETB event"
        );
    }
}
