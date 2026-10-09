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
mod damage_original_payloads;
mod prevention_draw_boundary;
mod simultaneous_redirect_budget;

#[cfg(test)]
pub(crate) use application::hand_replacement_choice_candidates;

use crate::DecisionMaker;
use crate::decisions::replacement_option_description;
use crate::events::DamageTarget;
use crate::events::{Event, EventContext, ReplacementMatcher as _};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::game_state::{GameState, UiBattlefieldTransitionKind};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::replacement::{
    ReplacementAction, ReplacementEffect, ReplacementEffectId, ReplacementEffectKey,
};
use crate::types::CardType;
use crate::zone::Zone;
use application::{
    apply_trait_change_destination, apply_trait_enter_tapped, apply_trait_enter_with_counters,
    apply_trait_replacement, find_matching_cards_in_hand, find_matching_sacrificable_permanents,
};

fn entry_controller_candidates(
    game: &GameState,
    controller: PlayerId,
    players: &crate::target::PlayerFilter,
) -> Vec<PlayerId> {
    let ctx = game.filter_context_for(controller, None);
    game.players
        .iter()
        .filter(|player| player.is_in_game() && players.matches_player(player.id, &ctx))
        .map(|player| player.id)
        .collect()
}

fn entry_controller_choice_context(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    players: &crate::target::PlayerFilter,
) -> crate::decisions::context::DecisionContext {
    let options = entry_controller_candidates(game, controller, players)
        .into_iter()
        .map(|player| {
            crate::decisions::context::SelectableOption::new(
                player.index(),
                game.player(player).unwrap().name.to_string(),
            )
        })
        .collect();
    crate::decisions::context::DecisionContext::SelectOptions(
        crate::decisions::context::SelectOptionsContext::new(
            controller,
            Some(source),
            "Choose the entering permanent's controller",
            options,
            1,
            1,
        ),
    )
}

fn apply_entry_controller_choice(
    game: &GameState,
    event: &Event,
    response: &InteractiveReplacementResponse,
    controller: PlayerId,
    players: &crate::target::PlayerFilter,
) -> Option<Event> {
    let InteractiveReplacementResponse::Options(selected) = response else {
        return None;
    };
    let [selected] = selected.as_slice() else {
        return None;
    };
    let selected = *selected;
    let player = entry_controller_candidates(game, controller, players)
        .into_iter()
        .find(|player| player.index() == selected)?;
    application::apply_trait_enter_under_control(event, player)
}

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
        .filter(|player| player.is_in_game() && game.are_opponents(controller, player.id))
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
        ReplacementAction::DiscardWithMadness => {
            "Exile this discarded card with Madness".to_string()
        }
        ReplacementAction::Additionally(_) => {
            format!(
                "Apply {}",
                replacement_option_description(game, effect.source)
            )
        }
        ReplacementAction::DeclineOptional(_) => {
            format!(
                "Do not apply {}",
                replacement_option_description(game, effect.source)
            )
        }
        ReplacementAction::TokenCreationTemplates {
            templates,
            choice_parent: Some(_),
            ..
        } => {
            let names: Vec<_> = templates
                .iter()
                .filter_map(|template| {
                    template
                        .downcast_ref::<crate::effects::CreateTokenEffect>()
                        .map(|create| create.token.card.name.clone())
                })
                .collect();
            format!(
                "Apply {}: create {} tokens",
                replacement_option_description(game, effect.source),
                names.join(" and ")
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
    origin: &crate::continuous::AbilityOrigin,
    ability: &crate::static_abilities::StaticAbility,
) -> Result<(), crate::effects::ExecutionError> {
    if source != entering_object && game.is_phased_out(source) {
        return Ok(());
    }
    let instance = ability.instance_id();
    let mut model = ability.compiled_model();
    if model.is_some_and(|model| {
        matches!(
            &model.payload,
            ironsmith_core::StaticAbilityPayload::Conditional { .. }
        )
    }) {
        let prospective;
        let evaluation_game = if source == entering_object {
            let from = game
                .object(source)
                .map_or(Zone::Stack, |object| object.zone);
            prospective = crate::events::EnterBattlefieldEvent::new(source, from)
                .with_controller_override(controller)
                .try_prospective_game_state(game)
                .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
            prospective.as_ref().unwrap_or(game)
        } else {
            game
        };
        while let Some(ironsmith_core::StaticAbility {
            payload:
                ironsmith_core::StaticAbilityPayload::Conditional {
                    ability: inner,
                    condition,
                },
            ..
        }) = model
        {
            let context = crate::condition_eval::ExternalEvaluationContext {
                controller,
                source,
                defending_player: None,
                attacking_player: None,
                filter_source: Some(source),
                iterated_player: None,
                triggering_event: None,
                trigger_identity: None,
                ability_index: None,
                options: Default::default(),
            };
            if !crate::condition_eval::evaluate_condition_external(
                evaluation_game,
                condition,
                &context,
            ) {
                return Ok(());
            }
            model = Some(inner);
        }
    }
    let start = copy_choice_effects.len();
    let mut spec = spec.clone();
    if spec.keep_other_source_abilities {
        // Copy exceptions retain copiable abilities, never abilities granted in
        // layer 6. Exclude only the occurrence currently applying (CR 707.9).
        let effects = game.all_continuous_effects();
        if let Some(values) = crate::continuous::copiable_values_with_effects(
            source,
            game.objects_map(),
            &effects,
            &game.battlefield,
            game.commander_objects(),
            game,
        ) {
            spec.added_abilities.extend(values.abilities.iter().cloned().filter(|ability|
                !matches!(&ability.kind, crate::ability::AbilityKind::Static(ability) if ability.instance_id() == instance)));
        }
    }
    build_enter_as_copy_effects_for_spec(
        game,
        entering_object,
        source,
        controller,
        &spec,
        reserved_objects,
        copy_choice_effects,
    );
    let face = matches!(origin, crate::continuous::AbilityOrigin::Printed(_))
        .then(|| game.object(source).and_then(|object| object.card))
        .flatten();
    for effect in &mut copy_choice_effects[start..] {
        // Candidate selection and declining are alternatives of this occurrence.
        // External abilities retain their actual host and recheck applicability
        // against the evolving entry rather than another ability's initial state.
        if source != entering_object {
            let mut filter = spec
                .affected_filter
                .clone()
                .expect("external copy has an affected filter");
            filter.specific = Some(entering_object);
            effect.source = source;
            effect.matcher = Some(Box::new(
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(filter),
            ));
        }
        effect.static_ability_instance = Some(instance);
        *effect = effect.clone().with_ability_origin(origin.clone(), face, 1);
    }
    Ok(())
}

fn build_enter_as_copy_effects_for_spec(
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

    let copy_condition_matches = |candidate, filter: &Option<crate::target::ObjectFilter>| {
        filter.as_ref().is_none_or(|filter| {
            let Some(mut object) = game.object(candidate).cloned() else {
                return false;
            };
            let effects = game.all_continuous_effects();
            if let Some(values) = crate::continuous::copiable_values_with_effects(
                candidate,
                game.objects_map(),
                &effects,
                &game.battlefield,
                game.commander_objects(),
                game,
            ) {
                object.copy_copiable_values_from_values(&values);
            }
            for card_type in &spec.added_card_types {
                if !object.card_types.contains(card_type) {
                    object.card_types.push(*card_type);
                }
            }
            let ctx = game.filter_context_for(controller, Some(entering_object));
            filter.matches_non_recursive(&object, &ctx, game)
        })
    };
    let added_abilities_for_source = |candidate| {
        let matches = copy_condition_matches(candidate, &spec.added_abilities_source_filter);
        if matches {
            spec.added_abilities.clone()
        } else {
            Vec::new()
        }
    };

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
                            removes_other_card_types: spec.removes_other_card_types,
                            added_supertypes: spec.added_supertypes.clone(),
                            removed_supertypes: spec.removed_supertypes.clone(),
                            added_subtypes: spec.added_subtypes.clone(),
                            added_abilities: added_abilities_for_source(copy_candidate),
                            set_base_power_toughness,
                            copy_followups: spec.copy_followups.clone(),
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
                    additional_counters: {
                        let mut counters = if copy_condition_matches(
                            candidate,
                            &spec.additional_counters_source_filter,
                        ) {
                            let mut counters = spec.additional_counters.clone();
                            let x = game
                                .object(entering_object)
                                .and_then(|object| object.own_entry_x_value())
                                .unwrap_or(0);
                            counters.extend(
                                spec.additional_x_counters
                                    .iter()
                                    .map(|counter| (*counter, x)),
                            );
                            counters
                        } else {
                            Vec::new()
                        };
                        for conditional in &spec.conditional_additional_counters {
                            if copy_condition_matches(
                                candidate,
                                &Some(conditional.source_filter.clone()),
                            ) {
                                counters.push((conditional.counter_type, conditional.count));
                            }
                        }
                        counters
                    },
                    name_override: spec.name_override.clone(),
                    added_colors: spec.added_colors,
                    added_card_types: spec.added_card_types.clone(),
                    removes_other_card_types: spec.removes_other_card_types,
                    added_supertypes: spec.added_supertypes.clone(),
                    removed_supertypes: spec.removed_supertypes.clone(),
                    added_subtypes: spec.added_subtypes.clone(),
                    added_abilities: added_abilities_for_source(candidate),
                    set_base_power_toughness,
                    copy_followups: spec.copy_followups.clone(),
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
pub fn process_trait_event(
    game: &mut GameState,
    event: Event,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    let operation_checkpoint = game.clone();
    let operation_result = (|| -> Result<TraitEventResult, crate::effects::ExecutionError> {
        let event = game.ensure_event_provenance(event);
        let mut state = TraitEventProcessingState::default();
        process_event_direct(game, event, &mut state, &[], None)
    })();
    if operation_result.is_err() {
        game.restore_execution_checkpoint(operation_checkpoint, false);
    }
    operation_result
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
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    let operation_checkpoint = game.clone();
    let operation_result = (|| -> Result<TraitEventResult, crate::effects::ExecutionError> {
        let event = game.ensure_event_provenance(event);
        let mut state = TraitEventProcessingState::default();
        let mut additional_effects = additional_effects.to_vec();
        assign_ephemeral_effect_ids(&mut additional_effects, u64::MAX / 2);
        process_event_direct(game, event, &mut state, &additional_effects, None)
    })();
    if operation_result.is_err() {
        game.restore_execution_checkpoint(operation_checkpoint, false);
    }
    operation_result
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
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
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

/// One branch split from damage, with applications inherited at the split.
#[derive(Debug, Clone)]
pub struct PendingDamageRemainder {
    pub event: crate::events::DamageEvent,
    pub applied_effects: std::collections::HashSet<ReplacementEffectId>,
    pub applied_effect_keys: std::collections::HashSet<ReplacementEffectKey>,
}

/// State for tracking trait-based event processing.
#[derive(Debug, Clone, Default)]
pub struct TraitEventProcessingState {
    /// Replacement effects that have already been applied to this event.
    pub applied_effects: std::collections::HashSet<ReplacementEffectId>,
    /// Stable replacement identities already applied to this event.
    pub applied_effect_keys: std::collections::HashSet<ReplacementEffectKey>,
    /// Number of processing passes (diagnostic only; not a semantic limit).
    pub iteration_count: u32,
    /// The entry driver refreshes prospective abilities between replacements.
    pub yield_after_etb_replacement: bool,
    /// Zone preparation hands entry to the complete entry proposal before any
    /// entry replacement sees an incomplete zone-only carrier.
    pub defer_battlefield_entry: bool,
    /// Original zone metadata remains available while the carrier is an entry.
    pub zone_change_context: Option<crate::events::ZoneChangeEvent>,
    /// Split branches survive prevention/replacement of the primary branch.
    pub damage_remainders: Vec<PendingDamageRemainder>,
    /// Added programs remain proposals until the owning event commits. In
    /// particular, selecting an addition does not execute it during matching.
    pub additional_programs: Vec<PreparedReplacementProgram>,
}

impl TraitEventProcessingState {
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
        self.iteration_count = self.iteration_count.saturating_add(1);
    }
}

fn quantitative_event_has_been_removed(event: &Event) -> bool {
    crate::events::downcast_event::<crate::events::CreateTokensEvent>(event.inner())
        .is_some_and(|creation| creation.total_count() == 0)
        || crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
        .is_some_and(|damage| damage.amount == 0)
        || crate::events::downcast_event::<crate::events::PutCountersEvent>(event.inner())
            .is_some_and(|placement| placement.count == 0)
        || crate::events::downcast_event::<crate::events::LifeGainEvent>(event.inner())
            .is_some_and(|gain| gain.amount == 0)
        || crate::events::downcast_event::<crate::events::LifeLossEvent>(event.inner())
            .is_some_and(|loss| loss.amount == 0)
        // A draw count of zero is absent; a positive attempt from an empty
        // library remains replaceable under CR 614.11.
        || crate::events::downcast_event::<crate::events::DrawEvent>(event.inner())
            .is_some_and(|draw| draw.count == 0)
        || crate::events::CounterRemovalEvent::from_event(event.inner())
            .is_some_and(|removal| removal.count() == 0)
}

/// Process an event directly using trait-based matchers.
fn retain_additional_programs(
    result: TraitEventResult,
    state: &mut TraitEventProcessingState,
) -> TraitEventResult {
    if state.additional_programs.is_empty() {
        result
    } else {
        TraitEventResult::Expanded {
            original: Box::new(result),
            programs: std::mem::take(&mut state.additional_programs),
        }
    }
}

fn process_event_direct(
    game: &mut GameState,
    mut event: Event,
    state: &mut TraitEventProcessingState,
    additional_effects: &[ReplacementEffect],
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    let checkpoint = game.clone();
    let state_checkpoint = state.clone();
    let result = process_event_direct_inner(
        game,
        event,
        state,
        additional_effects,
        event_source_snapshot,
    );
    if result.is_err() {
        game.restore_execution_checkpoint(checkpoint, false);
        *state = state_checkpoint;
    }
    result.map(|result| retain_additional_programs(result, state))
}

fn process_event_direct_inner(
    game: &mut GameState,
    mut event: Event,
    state: &mut TraitEventProcessingState,
    additional_effects: &[ReplacementEffect],
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    if !event.inner().is_replacement_proposal() {
        return Ok(TraitEventResult::Proceed(event));
    }
    // A zone proposal may acquire an entry carrier after its first replacement.
    // Keep the original cause/LKI/tags available to subsequent zone matchers.
    // Owning movement continuations may already have supplied richer context.
    if state.zone_change_context.is_none() {
        state.zone_change_context =
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner()).cloned();
    }
    // Each pass marks a distinct replacement identity before continuing.
    // The finite applicable-effect set and application history provide progress;
    // a fixed iteration cap must not authorize an incompletely replaced event.
    loop {
        // CR 119.10/614.7/616.1f: zero damage, counters, life change or draw has no operation.
        // Once a replacement removes that operation, later replacements have
        // nothing to modify and must not consume their one-shot lifetime.
        // Retain the typed carrier and independently collected programs/remainders;
        // an entry with zero counters still has its separate entry operation.
        if quantitative_event_has_been_removed(&event) {
            return Ok(TraitEventResult::Proceed(event));
        }

        if state.defer_battlefield_entry
            && crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
                .is_some_and(|change| change.to == Zone::Battlefield)
        {
            return Ok(TraitEventResult::Proceed(event));
        }
        state.increment();

        // Find all applicable replacement effects using trait-based matchers
        let applicable = find_applicable_trait_replacements(
            game,
            &event,
            state,
            additional_effects,
            event_source_snapshot,
        )?;

        if applicable.is_empty() {
            return Ok(TraitEventResult::Proceed(event));
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
            let result = apply_trait_replacement_retaining_damage_branches(
                game,
                event.clone(),
                &chosen_effect,
                state,
            )?;
            mark_applied_replacement_choice(state, &chosen_effect);
            consume_one_shot_if_applied(game, effect_id, &result);
            return Ok(
                match result {
                    TraitApplyResult::Modified(modified_event)
                        if state.yield_after_etb_replacement
                            && crate::events::downcast_event::<
                                crate::events::EnterBattlefieldEvent,
                            >(modified_event.inner())
                            .is_some() =>
                    {
                        TraitEventResult::Modified(modified_event)
                    }
                    TraitApplyResult::Modified(modified_event) => {
                        event = modified_event;
                        continue;
                    }
                    TraitApplyResult::Prevented => TraitEventResult::Prevented,
                    TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced {
                        context: Box::new(ReplacementEventContext::new(
                            &game,
                            event.clone(),
                            &state,
                        )),
                        effects,
                        effect_id,
                        replacement: chosen_effect.replacement.clone(),
                        source: chosen_effect.source,
                        controller: chosen_effect.controller,
                    },
                    TraitApplyResult::Unchanged(unchanged_event) => {
                        event = unchanged_event;
                        continue;
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
                        life_cost: match &chosen_effect.replacement {
                            ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } => {
                                Some(*life_cost)
                            }
                            _ => None,
                        },
                        destinations,
                        applied_effects: state.applied_effects.clone(),
                        applied_effect_keys: state.applied_effect_keys.clone(),
                        zone_change_context: state.zone_change_context.clone(),
                    },
                },
            );
        }

        // When multiple replacement effects are tied at the highest priority,
        // the affected player/controller chooses which one to apply next.
        if at_highest.len() > 1 || at_highest[0].replacement.needs_mana_color_choice() {
            let affected_player = event.inner().affected_player(game);
            let effect_ids: Vec<_> = at_highest.iter().map(|e| e.id).collect();

            return Ok(TraitEventResult::NeedsChoice {
                player: affected_player,
                applicable_effects: effect_ids,
                event: Box::new(event),
                applied_effects: state.applied_effects.clone(),
                applied_effect_keys: state.applied_effect_keys.clone(),
                zone_change_context: state.zone_change_context.clone(),
            });
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

        let result = apply_trait_replacement_retaining_damage_branches(
            game,
            event.clone(),
            &chosen_effect,
            state,
        )?;
        mark_applied_replacement_choice(state, &chosen_effect);
        consume_one_shot_if_applied(game, effect_id, &result);

        return Ok(match result {
            TraitApplyResult::Modified(modified_event)
                if state.yield_after_etb_replacement
                    && crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(
                        modified_event.inner(),
                    )
                    .is_some() =>
            {
                TraitEventResult::Modified(modified_event)
            }
            TraitApplyResult::Modified(modified_event) => {
                event = modified_event;
                continue;
            }
            TraitApplyResult::Prevented => TraitEventResult::Prevented,
            TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced {
                context: Box::new(ReplacementEventContext::new(&game, event.clone(), &state)),
                effects,
                effect_id,
                replacement: chosen_effect.replacement.clone(),
                source: chosen_effect.source,
                controller: chosen_effect.controller,
            },
            TraitApplyResult::Unchanged(unchanged_event) => {
                event = unchanged_event;
                continue;
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
                life_cost,
                destinations,
                applied_effects: state.applied_effects.clone(),
                applied_effect_keys: state.applied_effect_keys.clone(),
                zone_change_context: state.zone_change_context.clone(),
            },
        });
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
    if matches!(
        result,
        TraitApplyResult::Unchanged(_) | TraitApplyResult::NeedsInteraction { .. }
    ) {
        return;
    }
    // "The next N damage ... is dealt to ... instead": a redirection of
    // fewer than N damage leaves the rest of the shield for later damage.
    if let TraitApplyResult::Modified(event) = result
        && let Some(damage) =
            crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
        && game
            .effect_store
            .replacement_effects
            .consume_redirect_damage_amount(effect_id, damage.amount)
    {
        return;
    }
    game.effect_store
        .replacement_effects
        .mark_effect_used(effect_id);
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
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<InteractiveReplacementResult, crate::effects::ExecutionError> {
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

    // Handle reveal-or-enter-tapped (shadow land / snarl pattern). Its
    // fallback keeps the permanent on the battlefield, which no
    // discard-or-redirect gate does.
    if let Some(filter) = filter
        && redirect_zone == Zone::Battlefield
    {
        return Ok(handle_reveal_card_or_enter_tapped(
            game, response, object_id, controller, filter, provenance,
        ));
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
            replacement_scope,
            source_snapshot,
        );
    }

    // Handle pay-life-or-enter-tapped (shock land pattern)
    if let Some(cost) = life_cost {
        return handle_pay_life_or_enter_tapped(
            game,
            response,
            object_id,
            controller,
            cost,
            provenance,
            decision_maker,
            replacement_scope,
            source_snapshot,
        );
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
        return Ok(InteractiveReplacementResult::redirected(selected_zone));
    }

    // Fallback: redirect
    Ok(InteractiveReplacementResult::redirected(redirect_zone))
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
) -> Result<InteractiveReplacementResult, crate::effects::ExecutionError> {
    let InteractiveReplacementResponse::Objects(objects) = response else {
        return Ok(InteractiveReplacementResult::redirected(redirect_zone));
    };
    if objects.len() != count as usize {
        return Ok(InteractiveReplacementResult::redirected(redirect_zone));
    }
    let distinct = objects
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    if distinct.len() != objects.len() {
        return Ok(InteractiveReplacementResult::redirected(redirect_zone));
    }
    let candidates = find_matching_sacrificable_permanents(game, controller, object_id, filter);
    if !objects.iter().all(|object| candidates.contains(object)) {
        return Ok(InteractiveReplacementResult::redirected(redirect_zone));
    }

    let checkpoint = game.clone();
    let mut ctx = crate::effects::ExecutionContext::new(object_id, controller, decision_maker);
    let result = (|| {
        ctx.provenance = provenance;
        for permanent in objects {
            let effect = crate::effect::Effect::new(crate::effects::SacrificeTargetEffect::new(
                crate::target::ChooseSpec::SpecificObject(*permanent),
            ));
            let outcome = crate::effects::execute_effect(game, &effect, &mut ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(InteractiveReplacementResult::redirected(redirect_zone));
            }
            if !matches!(outcome.value, crate::effect::OutcomeValue::Count(value) if value >= 1) {
                return Ok(InteractiveReplacementResult::redirected(redirect_zone));
            }
            for event in outcome.events {
                game.queue_trigger_event(event.provenance(), event);
            }
        }
        Ok(InteractiveReplacementResult::enters_battlefield())
    })();
    let pending = ctx.decision_maker.awaiting_choice();
    if pending || result.as_ref().map_or(true, |outcome| !outcome.enters) {
        game.restore_execution_checkpoint(checkpoint, pending && result.is_ok());
    }
    result
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
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<InteractiveReplacementResult, crate::effects::ExecutionError> {
    Ok(match response {
        InteractiveReplacementResponse::Objects(cards) => {
            // Handle new context-based discard response (vector of cards)
            // For interactive replacement, we expect exactly 1 card
            if let Some(&card_id) = cards.first() {
                // A hidden hand card was opened on every peer before this
                // answer replayed; the claim is checked like any other.
                game.record_hidden_identity_obligations(
                    &[card_id],
                    filter,
                    &crate::target::FilterContext::new(controller),
                    "discard a card matching the filter",
                );
                let matching_cards = find_matching_cards_in_hand(game, controller, filter);
                if matching_cards.contains(&card_id) {
                    let cause =
                        crate::events::cause::EventCause::from_effect(object_id, controller);
                    let mut ctx = crate::effects::ExecutionContext::new(
                        object_id,
                        controller,
                        decision_maker,
                    )
                    .with_cause(cause)
                    .with_provenance(provenance);
                    ctx.source_snapshot = source_snapshot.cloned();
                    ctx.replacement = replacement_scope.clone();
                    let Some(prepared) = crate::effects::cards::prepare_selected_discard_batch(
                        game,
                        &mut ctx,
                        controller,
                        vec![card_id],
                        None,
                        true,
                    )?
                    else {
                        return Ok(InteractiveReplacementResult::redirected(redirect_zone));
                    };
                    let committed = crate::effects::cards::commit_selected_discard_batch(
                        game, &mut ctx, prepared,
                    )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(InteractiveReplacementResult::redirected(redirect_zone));
                    }
                    let Some(originals) =
                        committed.prepare_completion_with_outputs(game, &mut ctx)?
                    else {
                        return Ok(InteractiveReplacementResult::redirected(redirect_zone));
                    };
                    let type_verifiable = originals
                        .result_for(card_id)
                        .ok_or_else(|| {
                            crate::effects::ExecutionError::InternalError(
                                "entry discard payment lost its exact result".into(),
                            )
                        })?
                        .type_verifiable;
                    let mut outcome = originals
                        .complete_added_programs_with_outputs(game, &mut ctx)?
                        .into_outcome();
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(InteractiveReplacementResult::redirected(redirect_zone));
                    }
                    crate::effects::retain_unmatched_outcome_events(game, &mut outcome.events);
                    for event in outcome.events {
                        game.queue_trigger_event(event.provenance(), event);
                    }
                    if type_verifiable {
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
    })
}

/// Handle a reveal-card-or-enter-tapped interactive replacement.
fn handle_reveal_card_or_enter_tapped(
    game: &mut GameState,
    response: &InteractiveReplacementResponse,
    object_id: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    filter: &crate::target::ObjectFilter,
    provenance: crate::provenance::ProvNodeId,
) -> InteractiveReplacementResult {
    let InteractiveReplacementResponse::Objects(cards) = response else {
        return InteractiveReplacementResult::enters_tapped();
    };
    let Some(&card_id) = cards.first() else {
        return InteractiveReplacementResult::enters_tapped();
    };
    // A hidden hand card was opened on every peer before this answer
    // replayed; the claim is checked like any other.
    game.record_hidden_identity_obligations(
        &[card_id],
        filter,
        &crate::target::FilterContext::new(controller),
        "reveal a card matching the filter",
    );
    let matching_cards = find_matching_cards_in_hand(game, controller, filter);
    if !matching_cards.contains(&card_id) {
        return InteractiveReplacementResult::enters_tapped();
    }
    game.mark_hidden_cards_publicly_revealed(&[card_id]);
    let snapshot = game
        .object(card_id)
        .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game));
    let revealed = crate::effects::cards::public_reveal_observation(
        controller,
        card_id,
        Zone::Hand,
        object_id,
        snapshot,
        None,
        None,
        provenance,
    );
    game.queue_trigger_event(provenance, revealed);
    InteractiveReplacementResult::enters_battlefield()
}

/// Handle a pay-life-or-enter-tapped interactive replacement.
fn handle_pay_life_or_enter_tapped(
    game: &mut GameState,
    response: &InteractiveReplacementResponse,
    object_id: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    life_cost: u32,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<InteractiveReplacementResult, crate::effects::ExecutionError> {
    match response {
        InteractiveReplacementResponse::Accept => {
            // Player chose to pay life
            // Verify they can still pay
            let mut ctx =
                crate::effects::ExecutionContext::new(object_id, controller, decision_maker)
                    .with_provenance(provenance);
            ctx.replacement = replacement_scope.clone();
            ctx.source_snapshot = source_snapshot.cloned();
            if game
                .pay_life_with_context(controller, life_cost, &mut ctx)?
                .is_some()
            {
                // Permanent enters untapped
                Ok(InteractiveReplacementResult::enters_battlefield())
            } else {
                // Can't pay anymore (life changed since decision was made)
                // Permanent enters tapped
                Ok(InteractiveReplacementResult::enters_tapped())
            }
        }
        InteractiveReplacementResponse::Decline
        | InteractiveReplacementResponse::Objects(_)
        | InteractiveReplacementResponse::Options(_) => {
            // Player chose not to pay life - permanent enters tapped
            Ok(InteractiveReplacementResult::enters_tapped())
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

fn discard_result_from_arrival(arrival: Option<&crate::snapshot::ObjectSnapshot>) -> DiscardResult {
    arrival.map_or_else(DiscardResult::prevented, |snapshot| DiscardResult {
        new_id: Some(snapshot.object_id),
        final_zone: snapshot.zone,
        type_verifiable: zone_allows_type_verification(snapshot.zone),
        prevented: false,
    })
}

/// Completed original discard and programs that its owning operation must
/// execute after recording the original batch observations. This is not a
/// scalar compatibility result: the owner must retain each part explicitly.
#[derive(Debug)]
pub(crate) struct DiscardExecutionReceipt {
    pub result: DiscardResult,
    /// The actual discard identity/player/cause, before the physical move.
    /// None when the original discard was prevented or replaced by a program.
    pub resolved_event: Option<crate::events::DiscardEvent>,
    pub discarded_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    pub programs: Vec<PreparedReplacementProgram>,
    /// Observations from an Instead payload, separately from the original
    /// discard count. Publication belongs to the enclosing operation.
    pub payload_outcome: Option<crate::effects::CompletedEffectOutputs>,
}

/// Exact arrival evidence accompanies the completed discard receipt.
/// It is captured by the physical owner before its added/follow-up programs.
#[derive(Debug)]
pub(crate) struct CommittedDiscardOriginal {
    pub(crate) receipt: DiscardExecutionReceipt,
    pub(crate) arrival_snapshot: Option<crate::snapshot::ObjectSnapshot>,
}
impl CommittedDiscardOriginal {
    fn retain_original_payload(
        mut self,
        payload: crate::effects::CompletedEffectOutputs,
    ) -> Result<Self, crate::effects::ExecutionError> {
        if self.receipt.payload_outcome.replace(payload).is_some() {
            return Err(crate::effects::ExecutionError::InternalError(
                "discard selected program was committed twice".into(),
            ));
        }
        Ok(self)
    }
    fn finish_typed_original(
        mut self,
        arrival: Option<crate::snapshot::ObjectSnapshot>,
        payload: crate::effects::CompletedEffectOutputs,
    ) -> Result<Self, crate::effects::ExecutionError> {
        self.receipt.result = discard_result_from_arrival(arrival.as_ref());
        if self.receipt.result.prevented {
            self.receipt.resolved_event = None;
        } else if let Some(event) = &mut self.receipt.resolved_event {
            event.destination = self.receipt.result.final_zone;
        }
        self.arrival_snapshot = arrival;
        self.retain_original_payload(payload)
    }
    fn without_arrival(receipt: DiscardExecutionReceipt) -> Self {
        Self {
            receipt,
            arrival_snapshot: None,
        }
    }
}

impl DiscardExecutionReceipt {
    fn pending() -> Self {
        Self {
            result: DiscardResult::prevented(),
            resolved_event: None,
            discarded_snapshot: None,
            programs: Vec::new(),
            payload_outcome: None,
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
/// Some completed discard result, or None while a replacement choice is pending.
/// This root owner executes all programs and publishes actual observations;
/// enclosing instruction owners compose the shared selected batch preparation/commit.
pub fn execute_discard(
    game: &mut GameState,
    card_id: crate::ids::ObjectId,
    player: crate::ids::PlayerId,
    cause: crate::events::cause::EventCause,
    _requires_type_verification: bool,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Option<DiscardResult>, crate::effects::ExecutionError> {
    if decision_maker.awaiting_choice() {
        return Ok(None);
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let opened_batch = game.open_simultaneous_action();
    let result = (|| {
        let source = cause.source.unwrap_or(card_id);
        let controller = cause.source_controller.unwrap_or(player);
        let source_snapshot = game
            .object(source)
            .map(|object| {
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                    object, game,
                )
            })
            .transpose()?;
        let mut ctx =
            crate::effects::ExecutionContext::new(source, controller, &mut *decision_maker)
                .with_cause(cause)
                .with_provenance(provenance);
        ctx.source_snapshot = source_snapshot;
        let Some(prepared) = crate::effects::cards::prepare_selected_discard_batch(
            game,
            &mut ctx,
            player,
            vec![card_id],
            None,
            true,
        )?
        else {
            return Ok(None);
        };
        let committed =
            crate::effects::cards::commit_selected_discard_batch(game, &mut ctx, prepared)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let Some(originals) = committed.prepare_completion_with_outputs(game, &mut ctx)? else {
            return Ok(None);
        };
        let discard_result = originals.result_for(card_id).cloned().ok_or_else(|| {
            crate::effects::ExecutionError::InternalError(
                "root discard lost its exact result".into(),
            )
        })?;
        let mut outcome = originals
            .complete_added_programs_with_outputs(game, &mut ctx)?
            .into_outcome();
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        crate::effects::retain_unmatched_outcome_events(game, &mut outcome.events);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        Ok(Some(discard_result))
    })();
    game.close_simultaneous_action(opened_batch);
    if result.is_err() || decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            checkpoint,
            result.is_ok() && decision_maker.awaiting_choice(),
        );
    }
    result
}

/// Selected discard inputs belong to preparation; this record commits once.
/// The original snapshot and source evidence survive sibling card departures.
#[must_use = "commit the selected discard and retain its completion receipt"]
pub(crate) struct PreparedDiscard {
    result: TraitEventResult,
    card_id: ObjectId,
    player: PlayerId,
    provenance: crate::provenance::ProvNodeId,
    resolved_event: Option<crate::events::DiscardEvent>,
    discarded_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    programs: Vec<PreparedReplacementProgram>,
    scope: crate::effects::ReplacementExecutionContext,
    source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    replacement_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    typed_move: Option<PreparedTypedReplacementMove>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_discard_with_scope(
    game: &mut GameState,
    card_id: ObjectId,
    player: PlayerId,
    cause: crate::events::cause::EventCause,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<Option<PreparedDiscard>, crate::effects::ExecutionError> {
    prepare_discard_with_retention_scope(
        game,
        card_id,
        player,
        cause,
        provenance,
        decision_maker,
        replacement_scope,
        source_snapshot,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_discard_with_retention_scope(
    game: &mut GameState,
    card_id: ObjectId,
    player: PlayerId,
    cause: crate::events::cause::EventCause,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    retain_original: bool,
) -> Result<Option<PreparedDiscard>, crate::effects::ExecutionError> {
    if decision_maker.awaiting_choice() {
        return Ok(None);
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let result = prepare_discard_scoped_inner(
        game,
        card_id,
        player,
        cause,
        provenance,
        decision_maker,
        replacement_scope,
        source_snapshot,
        retain_original,
    );
    if result.is_err() || decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            checkpoint,
            result.is_ok() && decision_maker.awaiting_choice(),
        );
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn prepare_discard_scoped_inner(
    game: &mut GameState,
    card_id: ObjectId,
    player: PlayerId,
    cause: crate::events::cause::EventCause,
    provenance: crate::provenance::ProvNodeId,
    decision_maker: &mut dyn DecisionMaker,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    retain_original: bool,
) -> Result<Option<PreparedDiscard>, crate::effects::ExecutionError> {
    if decision_maker.awaiting_choice() {
        return Ok(None);
    }
    use crate::events::cards::DiscardEvent;
    use crate::events::traits::downcast_event;

    game.update_replacement_effects()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;

    // Create a discard event with the cause
    let original_snapshot = game
        .object(card_id)
        .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game));
    let discard_event = DiscardEvent::with_cause(card_id, player, cause.clone());
    let event = Event::new_with_provenance(discard_event, provenance);

    // Madness participates in the same CR 616 choice as discard and zone
    // replacements. Only applying this particular replacement can authorize
    // its linked trigger; merely ending up in exile is insufficient.
    let mut additional_effects = replacement_scope.additional_replacement_effects.clone();
    if game.object(card_id).is_some_and(|card| {
        card.alternative_casts
            .iter()
            .any(|alternative| alternative.is_madness())
    }) {
        additional_effects.push(ReplacementEffect::with_matcher(
            card_id,
            player,
            crate::events::cards::matchers::WouldDiscardMatcher::any_player()
                .with_card_filter(crate::target::ObjectFilter::source())
                .with_destination(Zone::Graveyard),
            ReplacementAction::DiscardWithMadness,
        ));
    }
    assign_ephemeral_effect_ids(&mut additional_effects, (u64::MAX / 2).saturating_add(2048));
    let result = process_with_dm_and_additional_effects_and_applied(
        game,
        event,
        decision_maker,
        &additional_effects,
        &replacement_scope.suppressed_replacement_effects,
        &replacement_scope.suppressed_replacement_effect_keys,
        source_snapshot,
    )?;
    // CR 616.1f: an interactive destination choice (Library of Leng) applies
    // that replacement, then the remaining applicable ones (madness, Rest in
    // Peace) still get their chance at the rewritten event.
    let result = continue_after_destination_choices_with_snapshot(
        game,
        result,
        decision_maker,
        &additional_effects,
        source_snapshot,
    )?;
    if decision_maker.awaiting_choice() {
        // The caller replays its checkpoint after answering the replacement
        // prompt. Never commit a provisional destination in the meantime.
        return Ok(None);
    }

    let (result, programs) = result.into_expansion();
    let resolved_event = match &result {
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            downcast_event::<DiscardEvent>(event.inner()).cloned()
        }
        TraitEventResult::Replaced {
            context,
            replacement,
            ..
        } if replacement_moves_object(replacement) => {
            downcast_event::<DiscardEvent>(context.event.inner()).cloned()
        }
        _ => None,
    };
    // Commit the resolved card and player, not the identities in the authored
    // instruction. Keep their snapshot while the original object still exists.
    let card_id = resolved_event.as_ref().map_or(card_id, |event| event.card);
    let player = resolved_event.as_ref().map_or(player, |event| event.player);
    let cause = resolved_event
        .as_ref()
        .map_or(cause, |event| event.cause.clone());
    let discarded_snapshot = resolved_event
        .as_ref()
        .and_then(|event| game.object(event.card))
        .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
        .or(original_snapshot);
    let replacement_source_snapshot = if let TraitEventResult::Replaced {
        source, controller, ..
    } = &result
    {
        let mut parent =
            crate::effects::ExecutionContext::new(*source, *controller, decision_maker);
        parent.source_snapshot = source_snapshot.cloned();
        crate::effects::replacement::capture_replacement_source_snapshot(game, &parent, *source)
    } else {
        None
    };
    // Consume a selected one-shot before the next sibling's matching pass.
    // Typed moves also resolve their generated zone chain in application order.
    let typed_move = if let TraitEventResult::Replaced {
        effects,
        effect_id,
        replacement,
        source,
        controller,
        context,
    } = &result
    {
        game.effect_store
            .replacement_effects
            .mark_effect_used(*effect_id);
        if replacement_moves_object(replacement) {
            let lookback = game.trigger_source_lookback_snapshots();
            prepare_typed_replacement_move(
                game,
                card_id,
                replacement,
                effects,
                *source,
                *controller,
                cause.clone(),
                decision_maker,
                context,
                &additional_effects,
                &lookback,
                Some(replacement_scope),
                retain_original,
            )?
        } else {
            None
        }
    } else {
        None
    };
    if decision_maker.awaiting_choice() {
        return Ok(None);
    }
    Ok(Some(PreparedDiscard {
        typed_move,
        result,
        card_id,
        player,
        provenance,
        resolved_event,
        discarded_snapshot,
        programs,
        scope: replacement_scope.clone(),
        source_snapshot: source_snapshot.cloned(),
        replacement_source_snapshot,
    }))
}

/// An exact discard record plus any still-retained selected replacement subtree.
/// The record's nonmoving result is already known; its payload is transferred
/// only after that authored subtree finishes. Moving replacements stay atomic.
pub(crate) struct DiscardOriginalCommit {
    pub original: CommittedDiscardOriginal,
    pub program: Option<RetainedDiscardOriginal>,
}

/// A selected authored discard original retains its native typed recipe until
/// its complete result and exact arrival are known. Internal generated actions
/// finish before the next physical sibling; enclosing discard additions remain
/// on the original record for the final batch completion.
pub(crate) struct RetainedDiscardOriginal {
    inner: RetainedDiscardKind,
}
enum RetainedDiscardKind {
    Program(crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>),
    Moved {
        arrival: Option<crate::snapshot::ObjectSnapshot>,
        program: crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    },
    Typed(RetainedTypedMoveOriginal),
}
impl RetainedDiscardOriginal {
    pub(crate) fn outcome(&self) -> &crate::effects::CompletedEffectOutputs {
        match &self.inner {
            RetainedDiscardKind::Program(program) => &program.outcome,
            RetainedDiscardKind::Moved { program, .. } => &program.outcome,
            RetainedDiscardKind::Typed(typed) => &typed.prefix,
        }
    }
    pub(crate) fn parts(
        &mut self,
    ) -> (
        &mut crate::effects::CompletedEffectOutputs,
        Option<&mut (dyn crate::effects::SimultaneousEffectCompletion + 'static)>,
    ) {
        match &mut self.inner {
            RetainedDiscardKind::Program(program) => {
                (&mut program.outcome, program.completion.as_deref_mut())
            }
            RetainedDiscardKind::Moved { program, .. } => {
                (&mut program.outcome, program.completion.as_deref_mut())
            }
            RetainedDiscardKind::Typed(typed) => (&mut typed.prefix, Some(&mut typed.completion)),
        }
    }
    pub(crate) fn prepare(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<bool, crate::effects::ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        let (outputs, completion) = self.parts();
        if let Some(completion) = completion {
            crate::effects::composition::prepare_standalone_original_completion(
                game,
                ctx,
                &mut outputs.outcome,
                completion,
            )?;
        }
        Ok(!ctx.decision_maker.awaiting_choice())
    }
    pub(crate) fn complete(
        self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: CommittedDiscardOriginal,
    ) -> Result<Option<CommittedDiscardOriginal>, crate::effects::ExecutionError> {
        let (arrival, payload, typed) = match self.inner {
            RetainedDiscardKind::Program(program) => (
                None,
                crate::effects::composition::complete_committed_original_with_outputs(
                    game, ctx, program,
                )?,
                false,
            ),
            RetainedDiscardKind::Moved { arrival, program } => (
                arrival,
                crate::effects::composition::complete_committed_original_with_outputs(
                    game, ctx, program,
                )?,
                true,
            ),
            RetainedDiscardKind::Typed(typed) => {
                let Some(handoff) =
                    Box::new(typed.completion).complete_original(game, ctx, typed.prefix)?
                else {
                    return Ok(None);
                };
                let arrival = handoff.added.original.arrival.clone();
                (
                    arrival,
                    crate::effects::composition::complete_committed_original_with_outputs(
                        game,
                        ctx,
                        handoff.into_receipt(),
                    )?,
                    true,
                )
            }
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let original = if typed {
            original.finish_typed_original(arrival, payload)?
        } else {
            original.retain_original_payload(payload)?
        };
        Ok(Some(original))
    }
}

pub(crate) fn commit_prepared_discard(
    game: &mut GameState,
    prepared: PreparedDiscard,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<CommittedDiscardOriginal, crate::effects::ExecutionError> {
    let committed = commit_discard_original(game, prepared, decision_maker, false)?;
    if committed.program.is_some() {
        return Err(crate::effects::ExecutionError::InternalError(
            "atomic discard retained an original program".into(),
        ));
    }
    Ok(committed.original)
}

/// Commit through the same physical owner, retaining a native selected draw.
pub(crate) fn commit_prepared_discard_original(
    game: &mut GameState,
    prepared: PreparedDiscard,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<DiscardOriginalCommit, crate::effects::ExecutionError> {
    commit_discard_original(game, prepared, decision_maker, true)
}

fn commit_discard_original(
    game: &mut GameState,
    prepared: PreparedDiscard,
    decision_maker: &mut dyn DecisionMaker,
    retain_original: bool,
) -> Result<DiscardOriginalCommit, crate::effects::ExecutionError> {
    let mut retained_program = None;
    let original = (|| {
        if decision_maker.awaiting_choice() {
            return Ok(CommittedDiscardOriginal::without_arrival(
                DiscardExecutionReceipt::pending(),
            ));
        }
        use crate::events::cards::DiscardEvent;
        use crate::events::traits::downcast_event;
        let PreparedDiscard {
            result,
            card_id,
            player,
            provenance,
            mut resolved_event,
            discarded_snapshot,
            programs,
            scope,
            source_snapshot,
            replacement_source_snapshot,
            typed_move,
        } = prepared;
        let replacement_scope = &scope;
        let source_snapshot = source_snapshot.as_ref();
        let mut arrival_snapshot = None;
        let mut payload_outcome = None;
        let discard_result = match result {
            TraitEventResult::Proceed(final_event) | TraitEventResult::Modified(final_event) => {
                // Extract the final destination from the (possibly modified) event
                if let Some(discard) = downcast_event::<DiscardEvent>(final_event.inner()) {
                    // A sibling replacement can consume this selected incarnation.
                    // Retain additions without reporting an unperformed discard.
                    let original_is_current = discarded_snapshot.as_ref().is_some_and(|snapshot| {
                        game.object(card_id).is_some_and(|object| {
                            object.zone == snapshot.zone && object.stable_id == snapshot.stable_id
                        })
                    });
                    if !original_is_current {
                        return Ok(CommittedDiscardOriginal::without_arrival(
                            DiscardExecutionReceipt {
                                result: DiscardResult::prevented(),
                                resolved_event: None,
                                discarded_snapshot,
                                programs,
                                payload_outcome: None,
                            },
                        ));
                    }
                    let destination = discard.destination;

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
                    if discard.madness_applied
                        && destination == Zone::Exile
                        && let Some(id) = new_id
                    {
                        game.set_madness_exiled(id);
                        queue_madness_trigger(game, id, player, &discard.cause, provenance);
                    }

                    DiscardResult {
                        new_id,
                        final_zone: destination,
                        type_verifiable: zone_allows_type_verification(destination),
                        prevented: false,
                    }
                } else {
                    return Err(crate::effects::ExecutionError::InternalError(
                        "discard replacement processing returned a non-DiscardEvent".into(),
                    ));
                }
            }

            TraitEventResult::Prevented => DiscardResult::prevented(),

            TraitEventResult::NeedsInteraction { .. } => {
                return Err(crate::effects::ExecutionError::InternalError(
                    "discard destination interaction did not resolve".into(),
                ));
            }

            TraitEventResult::Replaced {
                effects,
                effect_id: _,
                replacement,
                source: replacement_source,
                controller: replacement_controller,
                context,
            } => {
                // "If a card would be put into a graveyard from anywhere, exile it
                // instead" (Rest in Peace, Leyline of the Void, Dauthi Voidwalker):
                // the card is still discarded, it just ends up in exile (CR 614.6,
                // 701.9a).
                if !replacement_moves_object(&replacement) {
                    // Discard replaced with other effects - treat as prevented
                    let mut ctx = crate::effects::ExecutionContext::new(
                        replacement_source,
                        replacement_controller,
                        decision_maker,
                    );
                    ctx.replacement = replacement_scope.clone();
                    ctx.source_snapshot = source_snapshot.cloned();
                    let mut original = crate::effect::EffectOutcome::replaced();
                    original.set_value(crate::effect::OutcomeValue::Count(0));
                    let program = crate::effects::replacement::PreparedReplacementOriginal {
                        program: PreparedReplacementProgram {
                            context,
                            source: replacement_source,
                            controller: replacement_controller,
                            source_snapshot: replacement_source_snapshot,
                            effects,
                        },
                        scope: replacement_scope.clone(),
                        original,
                        bindings: crate::effects::replacement::ReplacementProgramBindings {
                            targets: None,
                            object_tags: Vec::new(),
                        },
                    };
                    let outcome = if retain_original {
                        let committed = program.commit_original_with_outputs(game, &mut ctx)?;
                        if committed.completion.is_some() {
                            retained_program = Some(RetainedDiscardOriginal {
                                inner: RetainedDiscardKind::Program(committed),
                            });
                            None
                        } else {
                            Some(committed.outcome)
                        }
                    } else {
                        Some(program.commit_with_outputs(game, &mut ctx)?)
                    };
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CommittedDiscardOriginal::without_arrival(
                            DiscardExecutionReceipt::pending(),
                        ));
                    }
                    return Ok(CommittedDiscardOriginal::without_arrival(
                        DiscardExecutionReceipt {
                            result: DiscardResult::prevented(),
                            resolved_event: None,
                            discarded_snapshot,
                            programs,
                            payload_outcome: outcome,
                        },
                    ));
                }

                if let Some(prepared) = typed_move {
                    let committed = commit_prepared_typed_replacement_move(
                        game,
                        prepared,
                        decision_maker,
                        retain_original,
                    )?;
                    if retain_original {
                        let inner = match committed {
                            TypedMoveOriginalCommit::ReadyOriginal(ready) => {
                                let mut continuations = ZoneDrawContinuations::default();
                                let Some(completed) =
                                    ready.prepare_tail(game, decision_maker, &mut continuations)?
                                else {
                                    return Ok(CommittedDiscardOriginal::without_arrival(
                                        DiscardExecutionReceipt::pending(),
                                    ));
                                };
                                continuations.require_committed_originals(
                                    "typed discard retained a selected action",
                                )?;
                                let mut program = crate::effects::composition::compose_original_commits_with_outputs(continuations.0);
                                program.outcome.retain_published_references(continuations.1);
                                if program.completion.is_none() {
                                    return CommittedDiscardOriginal::without_arrival(
                                        DiscardExecutionReceipt {
                                            result: DiscardResult::prevented(),
                                            resolved_event,
                                            discarded_snapshot,
                                            programs,
                                            payload_outcome: None,
                                        },
                                    )
                                    .finish_typed_original(completed.arrival, program.outcome);
                                }
                                RetainedDiscardKind::Moved {
                                    arrival: completed.arrival,
                                    program,
                                }
                            }
                            TypedMoveOriginalCommit::RetainedOriginal(retained) => {
                                RetainedDiscardKind::Typed(retained)
                            }
                            TypedMoveOriginalCommit::Suspended(_) => {
                                return Ok(CommittedDiscardOriginal::without_arrival(
                                    DiscardExecutionReceipt::pending(),
                                ));
                            }
                            TypedMoveOriginalCommit::Complete { .. } => {
                                return Err(crate::effects::ExecutionError::InternalError(
                                    "retained typed discard returned an atomic original".into(),
                                ));
                            }
                        };
                        retained_program = Some(RetainedDiscardOriginal { inner });
                        return Ok(CommittedDiscardOriginal::without_arrival(
                            DiscardExecutionReceipt {
                                result: DiscardResult::prevented(),
                                resolved_event,
                                discarded_snapshot,
                                programs,
                                payload_outcome: None,
                            },
                        ));
                    }
                    if let Some(completed) = committed.transfer_to(game, decision_maker, None)? {
                        arrival_snapshot = completed.arrival;
                        payload_outcome = Some(completed.outputs);
                    }
                }
                if decision_maker.awaiting_choice() {
                    return Ok(CommittedDiscardOriginal::without_arrival(
                        DiscardExecutionReceipt::pending(),
                    ));
                }
                discard_result_from_arrival(arrival_snapshot.as_ref())
            }

            TraitEventResult::NeedsChoice { .. } => {
                return Err(crate::effects::ExecutionError::InternalError(
                    "discard suspended without a captured decision".into(),
                ));
            }
            TraitEventResult::Expanded { .. } => {
                return Err(crate::effects::ExecutionError::InternalError(
                    "discard commit received an unflattened result".into(),
                ));
            }
        };
        if discard_result.prevented {
            resolved_event = None;
        } else if let Some(event) = &mut resolved_event {
            event.destination = discard_result.final_zone;
        }
        if arrival_snapshot.is_none() {
            arrival_snapshot = discard_result.new_id.and_then(|id| {
                game.object(id)
                    .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
            });
        }
        Ok(CommittedDiscardOriginal {
            receipt: DiscardExecutionReceipt {
                result: discard_result,
                resolved_event,
                discarded_snapshot,
                programs,
                payload_outcome,
            },
            arrival_snapshot,
        })
    })()?;
    Ok(DiscardOriginalCommit {
        original,
        program: retained_program,
    })
}

/// Madness's triggered ability (CR 702.35a): "When this card is exiled this
/// way, its owner may cast it by paying [cost] rather than paying its mana
/// cost. If that player doesn't, they put this card into their graveyard."
/// It's a real trigger that goes on the stack; the spell is cast while it
/// resolves (`MayCastForMadnessCostEffect`).
fn queue_madness_trigger(
    game: &mut GameState,
    exiled_id: crate::ids::ObjectId,
    player: crate::ids::PlayerId,
    cause: &crate::events::cause::EventCause,
    provenance: crate::provenance::ProvNodeId,
) {
    let Some(card) = game.object(exiled_id) else {
        return;
    };
    let owner = card.owner;
    let source_stable_id = card.stable_id;
    let source_name = card.name.to_string();
    let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(card, game);
    let ability = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::custom(
            "madness",
            "When this card is exiled this way".to_string(),
        ),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![
            crate::effect::Effect::new(crate::effects::MayCastForMadnessCostEffect::new()),
        ]),
        choices: vec![],
        intervening_if: None,
        presentation_label: None,
    };
    let trigger_identity = crate::triggers::compute_trigger_identity(&ability);
    let mut discarded =
        crate::events::cards::DiscardEvent::with_cause(exiled_id, player, cause.clone())
            .with_destination(Zone::Exile);
    discarded.madness_applied = true;
    let triggering_event =
        crate::triggers::TriggerEvent::new_with_provenance(discarded, provenance);
    game.defer_trigger_entries([crate::triggers::TriggeredAbilityEntry {
        linked_exile_owner: None,
        source_number_owner: None,
        source: exiled_id,
        controller: owner,
        x_value: None,
        event_value_amount: None,
        ability,
        triggering_event,
        source_stable_id,
        source_name,
        source_snapshot: Some(source_snapshot),
        tagged_objects: std::collections::HashMap::new(),
        source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
        trigger_identity,
    }]);
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
) -> Result<Vec<(ReplacementEffect, ReplacementPriority)>, crate::effects::ExecutionError> {
    let query = game
        .continuous_query_snapshot()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    let game = &query;
    let mut applicable = Vec::new();
    let prospective_etb_game =
        crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
            .map(|etb| etb.try_prospective_game_state(game))
            .transpose()
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
            .flatten();

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
            state.zone_change_context.as_ref(),
        )? {
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
            state.zone_change_context.as_ref(),
        )? {
            applicable.push((effect.clone(), priority));
        }
    }

    Ok(applicable)
}

/// Check if a replacement effect matches an event using trait-based matching.
fn trait_effect_matches_event(
    game: &GameState,
    effect: &ReplacementEffect,
    event: &Event,
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    prospective_etb_game: Option<&GameState>,
    zone_change_context: Option<&crate::events::ZoneChangeEvent>,
) -> Result<Option<ReplacementPriority>, crate::effects::ExecutionError> {
    use crate::events::ReplacementPriority as TraitPriority;

    if !game
        .effect_store
        .replacement_effects
        .available_in_damage_occurrence(effect.id)
    {
        return Ok(None);
    }

    if let ReplacementAction::EnterWithCounters {
        count,
        otherwise_count,
        ..
    } = &effect.replacement
        && (application::etb_value_uses_revealed_choice(count)
            || otherwise_count
                .as_ref()
                .is_some_and(application::etb_value_uses_revealed_choice))
        && let Some(etb) =
            crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
        && effect.source == etb.object
        && etb.prepared_choices.is_none()
    {
        return Ok(None);
    }
    // Entry programs may add counters while preparing the object. Apply
    // counter-placement modifiers once, after all entry counter proposals exist.
    if matches!(
        effect.replacement,
        ReplacementAction::DoubleCounters { .. } | ReplacementAction::AddCountersToPlacement { .. }
    ) && let Some(etb) =
        crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
        && etb.prepared_choices.is_none()
    {
        return Ok(None);
    }
    // All effects should have trait-based matchers
    let Some(matcher) = effect.matcher.as_ref() else {
        return Ok(None);
    };

    let ctx = EventContext::for_replacement_effect(effect.controller, effect.source, game)
        .with_prospective_etb_game(prospective_etb_game)
        .with_event_source_snapshot(event_source_snapshot);
    let matched = if let Some(entry) =
        crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
    {
        matcher
            .matches_entry_event(entry, &ctx)
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
    } else {
        matcher
            .matches_event(event.inner(), &ctx)
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
    };
    if !matched {
        // Discard includes a move out of the hand. Zone replacements must
        // compete with discard replacements before that move, and must see
        // its evolving destination after each applied replacement (CR 616.1).
        let zone_change = if let Some(discard) =
            crate::events::downcast_event::<crate::events::DiscardEvent>(event.inner())
        {
            let snapshot = game.object(discard.card).map(|card|
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(card, game)
            ).transpose()?;
            crate::events::ZoneChangeEvent::with_cause(
                discard.card,
                Zone::Hand,
                discard.destination,
                discard.cause.clone(),
                snapshot,
            )
        } else {
            let Some(context) = zone_change_context else {
                return Ok(None);
            };
            let mut retained = context.clone();
            if let Some(entry) =
                crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
            {
                retained.objects = vec![entry.object];
                retained.from = entry.from;
                retained.to = Zone::Battlefield;
            } else if let Some(change) =
                crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
            {
                retained.objects = change.objects.clone();
                retained.from = change.from;
                retained.to = change.to;
            } else {
                return Ok(None);
            }
            retained
        };
        if !matcher
            .matches_event(&zone_change, &ctx)
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
        {
            return Ok(None);
        }
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

    Ok(Some(priority))
}

/// The event immediately before an "instead" action and its full application
/// history. Consumers must not reconstruct this from the original instruction.
#[derive(Debug, Clone)]
pub struct ReplacementEventContext {
    pub event: Event,
    /// The event participant captured before original commitment can remove or change its object.
    pub affected_player: PlayerId,
    pub zone_change_context: Option<crate::events::ZoneChangeEvent>,
    pub applied_effects: std::collections::HashSet<ReplacementEffectId>,
    pub applied_effect_keys: std::collections::HashSet<ReplacementEffectKey>,
}

fn snapshot_replacement_damage_target(game: &GameState, event: Event) -> Event {
    let Some(damage) = crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
    else {
        return event;
    };
    let matching_snapshot = match damage.target {
        DamageTarget::Object(target) => damage
            .target_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.object_id == target),
        DamageTarget::Player(_) => false,
    };
    if matching_snapshot {
        return event;
    }
    let mut damage = damage.clone();
    // Defensively discard incompatible metadata from externally built events,
    // as well as redirects made by the ordinary event adapter.
    damage.target_snapshot = match damage.target {
        DamageTarget::Object(target) => game.object(target).and_then(|object| {
            crate::snapshot::ObjectSnapshot::capture_for_execution(object, game)
        }),
        DamageTarget::Player(_) => None,
    };
    event.rewrap(damage)
}

impl ReplacementEventContext {
    fn new(game: &GameState, event: Event, state: &TraitEventProcessingState) -> Self {
        let event = snapshot_replacement_damage_target(game, event);
        let affected_player = event.inner().affected_player(game);
        Self {
            event,
            affected_player,
            zone_change_context: state.zone_change_context.clone(),
            applied_effects: state.applied_effects.clone(),
            applied_effect_keys: state.applied_effect_keys.clone(),
        }
    }

    pub(crate) fn with_scope(
        game: &GameState,
        event: Event,
        scope: &crate::effects::ReplacementExecutionContext,
    ) -> Self {
        let event = snapshot_replacement_damage_target(game, event);
        let affected_player = event.inner().affected_player(game);
        Self {
            event,
            affected_player,
            zone_change_context: None,
            applied_effects: scope.suppressed_replacement_effects.clone(),
            applied_effect_keys: scope.suppressed_replacement_effect_keys.clone(),
        }
    }

    /// Preserve prior applications even if a one-shot was removed or static
    /// effects were regenerated before the replacement payload executes.
    pub fn apply_to(&self, ctx: &mut crate::effects::ExecutionContext<'_>) {
        ctx.triggering_event = Some(self.event.clone().into_raw());
        ctx.replacement.original_zone_event = self
            .zone_change_context
            .clone()
            .or_else(|| {
                crate::events::downcast_event::<crate::events::ZoneChangeEvent>(self.event.inner())
                    .cloned()
            })
            .map(Box::new);
        ctx.provenance = self.event.provenance();
        ctx.replacement
            .suppressed_replacement_effects
            .extend(self.applied_effects.iter().copied());
        ctx.replacement
            .suppressed_replacement_effect_keys
            .extend(self.applied_effect_keys.iter().cloned());
    }
}

/// A program captured from one replacement application. Its owning field
/// distinguishes substituted originals from added instructions; captured history
/// belongs to this exact event branch, never to an independent later event.
#[derive(Debug, Clone)]
pub struct PreparedReplacementProgram {
    pub context: Box<ReplacementEventContext>,
    pub source: ObjectId,
    pub controller: PlayerId,
    pub source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    pub effects: Vec<crate::effect::Effect>,
}

/// Result of processing an event through replacement effects.
///
/// Indicates how the event should proceed after checking replacement effects.
#[derive(Debug, Clone)]
#[must_use = "retain the resolved event, replacement actions and pending continuation"]
pub enum TraitEventResult {
    /// Keep both the original resolved proposal and added programs. The
    /// original may itself be prevented, replaced, or awaiting a choice.
    /// Consumers must retain this wrapper through continuation and execute
    /// it under the complete owning operation's checkpoint.
    Expanded {
        original: Box<TraitEventResult>,
        programs: Vec<PreparedReplacementProgram>,
    },
    /// Event should proceed (possibly modified).
    Proceed(Event),
    /// Event should proceed with modifications.
    Modified(Event),
    /// Event was prevented entirely.
    Prevented,
    /// Event was replaced with other effects.
    Replaced {
        context: Box<ReplacementEventContext>,
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
        /// Original zone cause, LKI and tags survive entry carrier conversion.
        zone_change_context: Option<crate::events::ZoneChangeEvent>,
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
        /// Effects already applied to `event` (including this one), so a
        /// caller that continues processing after the choice keeps CR 614.5.
        applied_effects: std::collections::HashSet<crate::replacement::ReplacementEffectId>,
        applied_effect_keys: std::collections::HashSet<crate::replacement::ReplacementEffectKey>,
        /// Original zone cause, LKI and tags survive entry carrier conversion.
        zone_change_context: Option<crate::events::ZoneChangeEvent>,
    },
}

impl TraitEventResult {
    /// Whether this original still needs a replacement decision. Added
    /// programs do not make a resolved original pending; they run later.
    pub(crate) fn requires_replacement_input(&self) -> bool {
        let mut original = self;
        while let Self::Expanded {
            original: nested, ..
        } = original
        {
            original = nested;
        }
        matches!(
            original,
            Self::NeedsChoice { .. } | Self::NeedsInteraction { .. }
        )
    }

    /// Extract the original result without discarding added programs. Outer
    /// wrappers represent earlier additions; later continuation wrappers
    /// append to those programs in replacement-application order.
    pub fn into_expansion(self) -> (Self, Vec<PreparedReplacementProgram>) {
        let mut result = self;
        let mut programs = Vec::new();
        while let Self::Expanded {
            original,
            programs: added,
        } = result
        {
            programs.extend(added);
            result = *original;
        }
        (result, programs)
    }

    /// Whether the original event is prevented. Added actions may still
    /// need execution; this predicate never consumes their receipt.
    pub fn is_prevented(&self) -> bool {
        match self {
            Self::Expanded { original, .. } => original.is_prevented(),
            Self::Prevented => true,
            _ => false,
        }
    }

    /// Borrow the resolved original event for inspection without dropping
    /// added actions or granting permission to commit it independently.
    pub fn resolved_event(&self) -> Option<&Event> {
        match self {
            Self::Expanded { original, .. } => original.resolved_event(),
            Self::Proceed(event) | Self::Modified(event) => Some(event),
            _ => None,
        }
    }

    /// Extract only an action-free resolved event. Any result requiring
    /// actions or continuation is returned intact to the caller.
    pub fn into_event(self) -> Result<Event, Self> {
        match self {
            Self::Proceed(event) | Self::Modified(event) => Ok(event),
            other => Err(other),
        }
    }
}

// =============================================================================
// Unified Event Outcome Type
// =============================================================================

/// Unified result of processing any event through replacement effects.
///
/// This generic type provides a consistent interface for all event processing,
/// carrying the original operation's success value. This value alone does
/// not carry deferred programs; prepared operations must retain their full receipt. The type parameter `T` represents the "success" value type for the
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

/// A prepared original operation plus programs added by its replacement
/// sequence. Even an original that was prevented/replaced/not applicable can
/// retain added programs. The commit owner must consume both parts explicitly.
#[derive(Debug, Clone)]
#[must_use = "commit the original and retain every deferred replacement program"]
pub struct PreparedEventOutcome<T> {
    pub original: EventOutcome<T>,
    pub programs: Vec<PreparedReplacementProgram>,
}

impl<T> PreparedEventOutcome<T> {
    pub(crate) fn pure(original: EventOutcome<T>) -> Self {
        Self {
            original,
            programs: Vec::new(),
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
    permanent: ObjectId,
    source: Option<ObjectId>,
) -> Result<DestroyResult, crate::effects::ExecutionError> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let original = process_destroy(game, permanent, source, &mut dm)?.ok_or_else(|| {
        crate::effects::ExecutionError::InternalError(
            "synchronous destruction unexpectedly suspended".into(),
        )
    })?;
    Ok(match original {
        EventOutcome::Proceed(final_zone) => DestroyResult::Destroyed { final_zone },
        EventOutcome::Prevented => DestroyResult::Prevented,
        EventOutcome::Replaced => DestroyResult::Replaced,
        EventOutcome::NotApplicable => DestroyResult::NotApplicable,
    })
}

/// One completed original destruction and its still-unexecuted additions.
/// Batch owners retain this until their original graveyard order and event
/// grouping are complete. A pending operation never returns a receipt.
#[derive(Debug)]
#[must_use]
pub(crate) struct DestroyExecutionReceipt {
    pub result: DestroyOutcome,
    pub permanent: ObjectId,
    pub snapshot: Option<crate::snapshot::ObjectSnapshot>,
    zone_receipts: Vec<(
        ObjectId,
        PreparedEventOutcome<crate::effects::zones::AppliedZoneChange>,
    )>,
    programs: Vec<PreparedReplacementProgram>,
    payload_outcome: Option<crate::effects::CompletedEffectOutputs>,
}

impl DestroyExecutionReceipt {
    pub(crate) fn has_deferred_programs(&self) -> bool {
        !self.programs.is_empty()
            || self
                .zone_receipts
                .iter()
                .any(|(_, receipt)| !receipt.programs.is_empty())
    }

    fn terminal(
        permanent: ObjectId,
        result: DestroyOutcome,
        snapshot: Option<crate::snapshot::ObjectSnapshot>,
    ) -> Self {
        Self {
            result,
            permanent,
            snapshot,
            zone_receipts: Vec::new(),
            programs: Vec::new(),
            payload_outcome: None,
        }
    }
}

pub fn process_destroy(
    game: &mut GameState,
    permanent: ObjectId,
    source: Option<ObjectId>,
    dm: &mut dyn DecisionMaker,
) -> Result<Option<DestroyOutcome>, crate::effects::ExecutionError> {
    process_destroy_owned(game, permanent, source, dm, None)
}

pub(crate) fn process_destroy_with_snapshot(
    game: &mut GameState,
    permanent: ObjectId,
    source: Option<ObjectId>,
    dm: &mut dyn DecisionMaker,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<Option<DestroyOutcome>, crate::effects::ExecutionError> {
    process_destroy_owned(game, permanent, source, dm, snapshot)
}

fn process_destroy_owned(
    game: &mut GameState,
    permanent: ObjectId,
    source: Option<ObjectId>,
    dm: &mut dyn DecisionMaker,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<Option<DestroyOutcome>, crate::effects::ExecutionError> {
    if dm.awaiting_choice() {
        return Ok(None);
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let controller = source
        .and_then(|id| game.object(id))
        .or_else(|| game.object(permanent))
        .map(|object| game.controller_of(object))
        .unwrap_or(game.turn.active_player);
    let mut ctx =
        crate::effects::ExecutionContext::new(source.unwrap_or(permanent), controller, dm);
    ctx.cause = source
        .map(|id| crate::events::cause::EventCause::from_effect(id, controller))
        .unwrap_or_else(crate::events::cause::EventCause::from_sba);
    let result = (|| {
        let Some(receipt) = process_destroy_scoped(game, permanent, source, &mut ctx, snapshot)?
        else {
            return Ok(None);
        };
        let original = receipt.result.clone();
        let outcome = finish_destroy_receipts(
            game,
            &mut ctx,
            crate::effect::EffectOutcome::resolved(),
            vec![receipt],
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        Ok(Some(original))
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            checkpoint,
            result.is_ok() && ctx.decision_maker.awaiting_choice(),
        );
    }
    result
}

pub(crate) fn process_destroy_scoped(
    game: &mut GameState,
    permanent: ObjectId,
    source: Option<ObjectId>,
    ctx: &mut crate::effects::ExecutionContext,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<Option<DestroyExecutionReceipt>, crate::effects::ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = process_destroy_scoped_inner(game, permanent, source, ctx, snapshot);
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            checkpoint,
            result.is_ok() && ctx.decision_maker.awaiting_choice(),
        );
        context_checkpoint.restore(ctx);
    }
    result
}

fn process_destroy_scoped_inner(
    game: &mut GameState,
    permanent: ObjectId,
    source: Option<ObjectId>,
    ctx: &mut crate::effects::ExecutionContext,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<Option<DestroyExecutionReceipt>, crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    game.update_replacement_effects()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    let observer_lookback = game.try_trigger_source_lookback_snapshots()?;
    let snapshot = match snapshot {
        Some(snapshot) => Some(snapshot),
        None => game
            .object(permanent)
            .map(|object| {
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                    object, game,
                )
            })
            .transpose()?,
    };
    if !game
        .object(permanent)
        .is_some_and(|object| object.zone == Zone::Battlefield)
    {
        return Ok(Some(DestroyExecutionReceipt::terminal(
            permanent,
            EventOutcome::NotApplicable,
            snapshot,
        )));
    }
    game.refresh_continuous_state()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    if game.current_has_static_ability_id(
        permanent,
        crate::static_abilities::StaticAbilityId::Indestructible,
    ) || !game.can_be_destroyed(permanent)
    {
        return Ok(Some(DestroyExecutionReceipt::terminal(
            permanent,
            EventOutcome::Prevented,
            snapshot,
        )));
    }
    let mut destroy_event =
        crate::events::DestroyEvent::new(permanent, source).with_cause(ctx.cause.clone());
    destroy_event.snapshot = snapshot.clone();
    let event =
        game.ensure_event_provenance(Event::new_with_provenance(destroy_event, ctx.provenance));
    let mut additional = ctx.additional_replacement_effects_snapshot();
    additional.extend(shield_counter_destroy_replacements(game, permanent, source));
    assign_ephemeral_effect_ids(&mut additional, (u64::MAX / 2).saturating_add(3072));
    let mut state = TraitEventProcessingState::default();
    let result = process_with_dm_and_additional_effects_and_applied_state(
        game,
        event.clone(),
        ctx.decision_maker,
        &additional,
        &ctx.replacement.suppressed_replacement_effects,
        &ctx.replacement.suppressed_replacement_effect_keys,
        ctx.source_snapshot.as_ref(),
        &mut state,
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let (original, programs) = result.into_expansion();
    let mut receipt =
        DestroyExecutionReceipt::terminal(permanent, EventOutcome::NotApplicable, snapshot);
    receipt.programs = programs;
    match original {
        TraitEventResult::Prevented => receipt.result = EventOutcome::Prevented,
        TraitEventResult::Proceed(final_event) | TraitEventResult::Modified(final_event) => {
            let destroyed =
                crate::events::downcast_event::<crate::events::DestroyEvent>(final_event.inner())
                    .ok_or_else(|| {
                    ExecutionError::InternalError(
                        "destruction replacement returned a different event kind".into(),
                    )
                })?;
            receipt.permanent = destroyed.permanent;
            receipt.snapshot = destroyed
                .snapshot
                .clone()
                .filter(|snapshot| snapshot.object_id == destroyed.permanent)
                .or_else(|| {
                    receipt
                        .snapshot
                        .take()
                        .filter(|snapshot| snapshot.object_id == destroyed.permanent)
                });
            if receipt.snapshot.is_none() {
                receipt.snapshot = game.object(destroyed.permanent).map(|object|
                    crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)
                ).transpose()?;
            }
            if !game
                .object(destroyed.permanent)
                .is_some_and(|object| object.zone == Zone::Battlefield)
            {
                receipt.result = EventOutcome::NotApplicable;
            } else if game.current_has_static_ability_id(
                destroyed.permanent,
                crate::static_abilities::StaticAbilityId::Indestructible,
            ) || !game.can_be_destroyed(destroyed.permanent)
            {
                receipt.result = EventOutcome::Prevented;
            } else {
                let scope = ReplacementEventContext::new(&game, final_event.clone(), &state);
                let parent_replacement = ctx.replacement.clone();
                let parent_source_snapshot = ctx.source_snapshot.clone();
                let parent_targets = ctx.targets.clone();
                let source_id = ctx.source;
                let controller = ctx.controller;
                let cause = if destroyed.source == source {
                    ctx.cause.clone()
                } else if let Some(source) = destroyed.source {
                    crate::events::cause::EventCause::from_effect(
                        source,
                        game.object(source)
                            .map(|object| game.controller_of(object))
                            .unwrap_or(controller),
                    )
                } else {
                    crate::events::cause::EventCause::from_sba()
                };
                let mut zone_ctx = crate::effects::ExecutionContext::new(
                    source_id,
                    controller,
                    &mut *ctx.decision_maker,
                );
                zone_ctx.replacement = parent_replacement;
                zone_ctx.source_snapshot = parent_source_snapshot;
                zone_ctx.targets = parent_targets;
                scope.apply_to(&mut zone_ctx);
                zone_ctx.cause = cause.clone();
                let zone_additional = zone_ctx.additional_replacement_effects_snapshot();
                let zone_receipt = crate::effects::zones::apply_zone_change_with_context_and_additional_effects_and_snapshot(
                    game, destroyed.permanent, Zone::Battlefield, Zone::Graveyard, cause, &mut zone_ctx, &zone_additional,
                    receipt.snapshot.clone(),
                )?;
                if zone_ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                receipt.result = match &zone_receipt.original {
                    EventOutcome::Proceed(applied) => {
                        if applied.final_zone != Zone::Battlefield
                            && !applied.new_object_ids.is_empty()
                        {
                            if let Some(snapshot) = receipt.snapshot.clone() {
                                if applied.final_zone == Zone::Graveyard {
                                    game.record_ui_battlefield_transition(
                                        UiBattlefieldTransitionKind::Destroyed,
                                        snapshot.stable_id,
                                    );
                                }
                                let mut trigger =
                                    crate::triggers::TriggerEvent::new_with_provenance(
                                        crate::events::DestroyEvent::new(
                                            destroyed.permanent,
                                            destroyed.source,
                                        )
                                        .with_cause(zone_ctx.cause.clone())
                                        .with_successful_result(snapshot, applied.final_zone)
                                        .with_complete_source_lookback(),
                                        final_event.provenance(),
                                    )
                                    .with_lookback_source_snapshots(observer_lookback.clone());
                                if destroyed.source == Some(zone_ctx.source)
                                    && game.object(zone_ctx.source).is_none()
                                    && let Some(snapshot) = zone_ctx.source_snapshot.clone()
                                {
                                    trigger = trigger.with_source_snapshot(snapshot);
                                }
                                game.queue_trigger_event(trigger.provenance(), trigger);
                            }
                        }
                        EventOutcome::Proceed(applied.final_zone)
                    }
                    EventOutcome::Prevented => EventOutcome::Prevented,
                    EventOutcome::Replaced => EventOutcome::Replaced,
                    EventOutcome::NotApplicable => EventOutcome::NotApplicable,
                };
                receipt
                    .zone_receipts
                    .push((destroyed.permanent, zone_receipt));
            }
        }
        TraitEventResult::Replaced {
            effects,
            effect_id,
            source,
            controller,
            context,
            ..
        } => {
            game.effect_store
                .replacement_effects
                .mark_effect_used(effect_id);
            let object_tags = receipt
                .snapshot
                .clone()
                .map(|snapshot| {
                    vec![
                        ("it".into(), vec![snapshot.clone()]),
                        ("__it__".into(), vec![snapshot]),
                    ]
                })
                .unwrap_or_default();
            let outcome = crate::effects::replacement::execute_replacement_payload_with_outputs(
                game,
                ctx,
                &effects,
                source,
                controller,
                &context,
                Some(vec![crate::effects::ResolvedTarget::Object(
                    receipt.permanent,
                )]),
                None,
                object_tags,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            receipt.payload_outcome = Some(outcome);
            receipt.result = EventOutcome::Replaced;
        }
        TraitEventResult::NeedsChoice { .. } => {
            return Err(ExecutionError::InternalError(
                "destruction replacement choice did not resolve".into(),
            ));
        }
        TraitEventResult::NeedsInteraction { .. } => {
            return Err(ExecutionError::InternalError(
                "unsupported interactive destruction replacement".into(),
            ));
        }
        TraitEventResult::Expanded { .. } => {
            return Err(ExecutionError::InternalError(
                "nested destruction expansion was not flattened".into(),
            ));
        }
    }
    Ok(Some(receipt))
}

pub(crate) fn finish_destroy_receipts(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    original: crate::effect::EffectOutcome,
    receipts: Vec<DestroyExecutionReceipt>,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    finish_destroy_receipts_with_outputs(game, ctx, original, receipts)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn finish_destroy_receipts_with_outputs(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    original: crate::effect::EffectOutcome,
    receipts: Vec<DestroyExecutionReceipt>,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    let frozen = freeze_destroy_receipts(game, receipts);
    finish_destroy_receipts_frozen_with_outputs(game, ctx, original, frozen)
}

pub(crate) struct FrozenDestroyReceipts(
    Result<
        Vec<(
            DestroyExecutionReceipt,
            (Vec<ObjectId>, Vec<crate::snapshot::ObjectSnapshot>),
            crate::effects::zones::FrozenZoneChangeReceipts,
        )>,
        crate::effects::ExecutionError,
    >,
);

pub(crate) fn freeze_destroy_receipts(
    game: &mut GameState,
    receipts: Vec<DestroyExecutionReceipt>,
) -> FrozenDestroyReceipts {
    let captured = (|| {
        // Freeze each arriving identity before any added program can move it again.
        let bindings = receipts.iter().map(|receipt| {
            let mut ids = receipt.zone_receipts.iter().flat_map(|(_, zone)| match &zone.original {
                EventOutcome::Proceed(applied) => applied.new_object_ids.clone(), _ => Vec::new(),
            }).collect::<Vec<_>>();
            if ids.is_empty() {
                if let Some(applied) = crate::effects::zones::take_recorded_zone_change(game, receipt.permanent) {
                    ids = applied.new_object_ids; game.record_zone_change_results(receipt.permanent, ids.clone());
                }
            }
            if ids.is_empty() { ids.push(receipt.permanent); }
            let mut snapshots = ids.iter().filter_map(|id| game.object(*id).map(|object|
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game))).collect::<Result<Vec<_>, crate::effects::ExecutionError>>()?;
            if snapshots.is_empty() { snapshots.extend(receipt.snapshot.clone()); }
            Ok((ids, snapshots))
        }).collect::<Result<Vec<_>, crate::effects::ExecutionError>>()?;
        Ok(receipts
            .into_iter()
            .zip(bindings)
            .map(|(mut receipt, bindings)| {
                let zones = crate::effects::zones::freeze_zone_change_receipts(
                    game,
                    std::mem::take(&mut receipt.zone_receipts),
                );
                (receipt, bindings, zones)
            })
            .collect())
    })();
    if let Err(error) = &captured {
        game.record_token_resource_failure(error);
    }
    FrozenDestroyReceipts(captured)
}

pub(crate) fn finish_destroy_receipts_frozen(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    original: crate::effect::EffectOutcome,
    frozen: FrozenDestroyReceipts,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    finish_destroy_receipts_frozen_with_outputs(game, ctx, original, frozen)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn finish_destroy_receipts_frozen_with_outputs<
    O: crate::effects::OriginalEffectOutput,
>(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    original: O,
    frozen: FrozenDestroyReceipts,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    let mut original = original.into_outputs();
    let frozen = frozen.0?;
    let died = frozen
        .iter()
        .filter(|(receipt, _, _)| {
            matches!(receipt.result, EventOutcome::Proceed(Zone::Graveyard))
                && receipt.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot
                        .card_types
                        .contains(&crate::types::CardType::Creature)
                })
        })
        .map(|(receipt, _, _)| receipt.permanent)
        .collect::<Vec<_>>();
    if !died.is_empty() {
        let aggregate = original
            .outcome
            .clone()
            .with_execution_fact(crate::effect::ExecutionFact::ObjectsDied(died));
        original = original.project_aggregate(aggregate);
    }
    let mut outcomes = Vec::new();
    for (receipt, (ids, snapshots), zones) in frozen {
        let base = receipt.payload_outcome.unwrap_or_else(|| {
            crate::effects::CompletedEffectOutputs::aggregate_only(
                crate::effect::EffectOutcome::resolved(),
            )
        });
        let outcome = crate::effects::zones::finish_zone_change_receipts_frozen_with_outputs(
            game, ctx, base, zones,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                crate::effect::EffectOutcome::count(0),
            ));
        }
        let outcome =
            crate::effects::replacement::complete_replacement_programs_with_original_outputs(
                game,
                ctx,
                outcome,
                |game, ctx, original| {
                    crate::effects::replacement::complete_deferred_replacement_programs_with_bindings(
                game,
                ctx,
                original,
                receipt.programs,
                |_, _, _| {
                    Ok(crate::effects::replacement::ReplacementProgramBindings {
                        targets: Some(
                            ids.iter()
                                .copied()
                                .map(crate::effects::ResolvedTarget::Object)
                                .collect(),
                        ),
                        object_tags: vec![
                            ("it".into(), snapshots.clone()),
                            ("__it__".into(), snapshots.clone()),
                        ],
                    })
                },
            )
                },
            )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                crate::effect::EffectOutcome::count(0),
            ));
        }
        outcomes.push(outcome);
    }
    Ok(original.append_replacement_outputs(outcomes))
}

fn shield_counter_count(game: &GameState, permanent: ObjectId) -> u32 {
    game.object(permanent)
        .filter(|object| object.zone == Zone::Battlefield)
        .and_then(|object| object.counters.get(&CounterType::Shield).copied())
        .unwrap_or(0)
}

/// The built-in destroy replacement created by shield counters (CR 122.1c).
fn shield_counter_destroy_replacements(
    game: &GameState,
    permanent: ObjectId,
    source: Option<ObjectId>,
) -> Vec<ReplacementEffect> {
    if source.is_none() || shield_counter_count(game, permanent) == 0 {
        return Vec::new();
    }
    let Some(controller) = game
        .object(permanent)
        .map(|object| game.controller_of(object))
    else {
        return Vec::new();
    };
    let mut effects = vec![ReplacementEffect::with_matcher(
        permanent,
        controller,
        crate::events::permanents::matchers::ThisWouldBeDestroyedMatcher,
        ReplacementAction::Instead(vec![crate::effect::Effect::remove_counters(
            CounterType::Shield,
            1,
            crate::target::ChooseSpec::SpecificObject(permanent),
        )]),
    )];
    assign_ephemeral_effect_ids(&mut effects, u64::MAX / 8);
    effects
}

/// The built-in damage prevention created by shield counters (CR 122.1c):
/// "If damage would be dealt to this permanent, prevent that damage and
/// remove a shield counter from it." The additional part still happens for
/// unpreventable damage (CR 615.12).
fn shield_counter_damage_replacements(
    game: &GameState,
    target: DamageTarget,
) -> Vec<ReplacementEffect> {
    let DamageTarget::Object(permanent) = target else {
        return Vec::new();
    };
    if shield_counter_count(game, permanent) == 0 {
        return Vec::new();
    }
    let Some(controller) = game
        .object(permanent)
        .map(|object| game.controller_of(object))
    else {
        return Vec::new();
    };
    vec![ReplacementEffect::with_matcher(
        permanent,
        controller,
        crate::events::damage::matchers::DamageToSelfMatcher,
        ReplacementAction::PreventDamageThen(vec![crate::effect::Effect::remove_counters(
            CounterType::Shield,
            1,
            crate::target::ChooseSpec::SpecificObject(permanent),
        )]),
    )]
}

/// The built-in replacement created by finality counters (CR 122.1h): "If
/// this permanent would be put into a graveyard from the battlefield, exile
/// it instead." It applies to any permanent, and as a real replacement effect
/// it is ordered against other "instead" effects by the controller (CR 616.1).
fn finality_counter_replacements(game: &GameState, permanent: ObjectId) -> Vec<ReplacementEffect> {
    let Some(object) = game.object(permanent) else {
        return Vec::new();
    };
    if object
        .counters
        .get(&CounterType::Finality)
        .copied()
        .unwrap_or(0)
        == 0
    {
        return Vec::new();
    }
    let controller = game.controller_of(object);
    vec![ReplacementEffect::with_matcher(
        permanent,
        controller,
        crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            crate::target::ObjectFilter::specific(permanent),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
        ),
        ReplacementAction::ChangeDestination(Zone::Exile),
    )]
}

/// The built-in untap replacement created by stun counters (CR 122.1d): "If
/// a permanent with a stun counter on it would become untapped, instead
/// remove a stun counter from it."
fn stun_counter_untap_replacements(
    game: &GameState,
    permanent: ObjectId,
) -> Vec<ReplacementEffect> {
    let stunned = game
        .object(permanent)
        .and_then(|object| object.counters.get(&CounterType::Stun).copied())
        .unwrap_or(0)
        > 0;
    let Some(controller) = game
        .object(permanent)
        .map(|object| game.controller_of(object))
    else {
        return Vec::new();
    };
    if !stunned {
        return Vec::new();
    }
    let mut effects = vec![ReplacementEffect::with_matcher(
        permanent,
        controller,
        crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
            crate::target::ObjectFilter::specific(permanent),
        ),
        ReplacementAction::Instead(vec![crate::effect::Effect::remove_counters(
            CounterType::Stun,
            1,
            crate::target::ChooseSpec::SpecificObject(permanent),
        )]),
    )];
    assign_ephemeral_effect_ids(&mut effects, u64::MAX / 8 + 1);
    effects
}

/// Untap a permanent using the complete replacement result. Notifications are
/// returned to the caller, which owns publishing the enclosing operation.
pub fn process_untap(
    game: &mut GameState,
    permanent: ObjectId,
    dm: &mut dyn DecisionMaker,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    let controller = game
        .current_controller(permanent)
        .unwrap_or(game.turn.active_player);
    let mut ctx = crate::effects::ExecutionContext::new(permanent, controller, dm);
    process_untap_with_execution_context(game, permanent, &mut ctx)
}

/// Scalar callers project the same untap owner used by compound effects.
pub(crate) fn process_untap_with_execution_context(
    game: &mut GameState,
    permanent: ObjectId,
    ctx: &mut crate::effects::ExecutionContext,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    process_untap_with_execution_context_and_outputs(game, permanent, ctx)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn process_untap_with_execution_context_and_outputs(
    game: &mut GameState,
    permanent: ObjectId,
    ctx: &mut crate::effects::ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    use crate::effect::EffectOutcome;
    use crate::effects::CompletedEffectOutputs;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    game.clear_pending_decision_controllers();
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let prepared = prepare_untap_with_execution_context(game, permanent, ctx)?;
            let original = commit_prepared_untap_with_outputs(game, ctx, prepared)?;
            crate::effects::composition::complete_standalone_original_with_outputs(
                game, ctx, original,
            )
        },
    )
}

/// Preparation captures replacement choices but performs no untap or payload.
#[derive(Debug)]
pub(crate) struct PreparedUntap {
    original: PreparedUntapOriginal,
    programs: Vec<PreparedReplacementProgram>,
}

/// One selected untap declaration owns either native operands or the actual
/// replacement programme. The discarded event result is not a second owner.
#[derive(Debug)]
enum PreparedUntapOriginal {
    Native {
        event: TraitEventResult,
        before_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    },
    Replacement(crate::effects::replacement::PreparedReplacementOriginal),
}

pub(crate) fn prepare_untap_with_execution_context(
    game: &mut GameState,
    permanent: ObjectId,
    ctx: &mut crate::effects::ExecutionContext,
) -> Result<Option<PreparedUntap>, crate::effects::ExecutionError> {
    use crate::effect::EffectOutcome;
    if ctx.decision_maker.awaiting_choice() || !game.is_tapped(permanent) {
        return Ok(None);
    }
    game.update_replacement_effects()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    let mut additional = ctx.additional_replacement_effects_snapshot();
    additional.extend(stun_counter_untap_replacements(game, permanent));
    let event = Event::untap(permanent).with_provenance(ctx.provenance);
    let processed = process_with_dm_and_additional_effects_and_applied(
        game,
        event,
        &mut *ctx.decision_maker,
        &additional,
        &ctx.replacement.suppressed_replacement_effects,
        &ctx.replacement.suppressed_replacement_effect_keys,
        ctx.source_snapshot.as_ref(),
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let (original, mut programs) = processed.into_expansion();
    for program in &mut programs {
        if program.source_snapshot.is_none() {
            program.source_snapshot =
                crate::effects::replacement::capture_replacement_source_snapshot(
                    game,
                    ctx,
                    program.source,
                );
        }
    }
    let original = match original {
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => {
            let targets = untap_program_targets(&context)?;
            let mut result = EffectOutcome::replaced();
            result.set_value(crate::effect::OutcomeValue::Count(0));
            PreparedUntapOriginal::Replacement(
                crate::effects::replacement::PreparedReplacementOriginal {
                    program: PreparedReplacementProgram {
                        context,
                        source,
                        controller,
                        source_snapshot:
                            crate::effects::replacement::capture_replacement_source_snapshot(
                                game, ctx, source,
                            ),
                        effects,
                    },
                    scope: ctx.replacement.clone(),
                    original: result,
                    bindings: crate::effects::replacement::ReplacementProgramBindings {
                        targets,
                        object_tags: Vec::new(),
                    },
                },
            )
        }
        event => {
            let before_snapshot = match &event {
                TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                    let untap =
                        crate::events::downcast_event::<crate::events::UntapEvent>(event.inner())
                            .ok_or_else(|| {
                            crate::effects::ExecutionError::InternalError(
                                "untap preparation returned an incompatible event".into(),
                            )
                        })?;
                    game.object(untap.permanent).map(|object|
                        crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)
                    ).transpose()?
                }
                _ => None,
            };
            PreparedUntapOriginal::Native {
                event,
                before_snapshot,
            }
        }
    };
    Ok(Some(PreparedUntap { original, programs }))
}

fn untap_program_targets(
    context: &ReplacementEventContext,
) -> Result<Option<Vec<crate::effects::ResolvedTarget>>, crate::effects::ExecutionError> {
    let untap = crate::events::downcast_event::<crate::events::UntapEvent>(context.event.inner())
        .ok_or_else(|| {
        crate::effects::ExecutionError::InternalError(
            "untap program lost its captured event".into(),
        )
    })?;
    Ok(Some(vec![crate::effects::ResolvedTarget::Object(
        untap.permanent,
    )]))
}

pub(crate) fn commit_prepared_untap_with_outputs(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    prepared: Option<PreparedUntap>,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    crate::effects::ExecutionError,
> {
    use crate::effects::{CompletedEffectOutputs, SimultaneousEffectCommit};
    let Some(prepared) = prepared else {
        return Ok(SimultaneousEffectCommit::finished(
            CompletedEffectOutputs::aggregate_only(crate::effect::EffectOutcome::count(0)),
        ));
    };
    let original = match prepared.original {
        PreparedUntapOriginal::Replacement(original) => {
            original.commit_original_with_outputs(game, ctx)?
        }
        PreparedUntapOriginal::Native {
            event,
            before_snapshot,
        } => SimultaneousEffectCommit::finished(commit_resolved_untap_event_with_outputs(
            game,
            ctx,
            event,
            before_snapshot,
        )?),
    };
    Ok(
        crate::effects::replacement::defer_replacement_programs_with_outputs(
            original,
            prepared.programs,
            |context| {
                Ok(crate::effects::replacement::ReplacementProgramBindings {
                    targets: untap_program_targets(context)?,
                    object_tags: Vec::new(),
                })
            },
        ),
    )
}

fn commit_resolved_untap_event_with_outputs(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    processed: TraitEventResult,
    before: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    use crate::effect::{EffectOutcome, OutcomeValue};
    use crate::effects::{CompletedEffectOutputs, ExecutionError};
    match processed {
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let untap = crate::events::downcast_event::<crate::events::UntapEvent>(event.inner())
                .ok_or_else(|| {
                ExecutionError::InternalError(
                    "untap replacement returned an incompatible event".into(),
                )
            })?;
            if !game.is_tapped(untap.permanent) {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }

            game.untap(untap.permanent);
            let mut notification = crate::events::PermanentUntappedEvent::capture(
                game,
                untap.permanent,
                Some(ctx.controller),
            );
            notification.before_snapshot = before;
            Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(1).with_event(
                    crate::triggers::TriggerEvent::new_with_provenance(
                        notification,
                        event.provenance(),
                    ),
                ),
            ))
        }
        TraitEventResult::Expanded { .. } | TraitEventResult::Replaced { .. } => Err(
            ExecutionError::InternalError("untap commit received an unsealed replacement".into()),
        ),
        TraitEventResult::Prevented => {
            let mut outcome = EffectOutcome::prevented();
            outcome.value = OutcomeValue::Count(0);
            Ok(CompletedEffectOutputs::aggregate_only(outcome))
        }
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            Err(ExecutionError::InternalError(
                "untap replacement suspended without a captured decision".into(),
            ))
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
) -> Result<PreparedEventOutcome<PreparedZoneChange>, crate::effects::ExecutionError> {
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
) -> Result<PreparedEventOutcome<PreparedZoneChange>, crate::effects::ExecutionError> {
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
) -> Result<PreparedEventOutcome<PreparedZoneChange>, crate::effects::ExecutionError> {
    process_zone_change_inner(game, object, from, to, cause, dm, additional_effects, None)
}

fn merged_card_only_change_destinations(
    game: &GameState,
    event: &crate::events::ZoneChangeEvent,
    additional_effects: &[ReplacementEffect],
) -> Result<std::collections::HashSet<Zone>, crate::effects::ExecutionError> {
    let mut destinations = std::collections::HashSet::new();
    for effect in game
        .effect_store
        .replacement_effects
        .effects()
        .iter()
        .chain(additional_effects.iter())
    {
        let Some(matcher) = effect.matcher.as_ref() else {
            continue;
        };
        let ctx = crate::events::context::EventContext::for_replacement_effect(
            effect.controller,
            effect.source,
            game,
        );
        if !matcher
            .matches_merged_card_component_only(event, &ctx)
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
        {
            continue;
        }
        if let crate::replacement::ReplacementAction::ChangeDestination(destination) =
            &effect.replacement
        {
            destinations.insert(*destination);
        }
    }
    Ok(destinations)
}

/// Resolve interactive "you may put it into [zone] instead" destination
/// choices (Library of Leng, optional zone replacements) by applying the
/// chosen destination and continuing replacement processing, with the chosen
/// effect and every earlier one marked applied (CR 614.5, 616.1f).
fn continue_after_destination_choices(
    game: &mut GameState,
    result: TraitEventResult,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    continue_after_destination_choices_with_snapshot(game, result, dm, additional_effects, None)
}

fn continue_after_destination_choices_with_snapshot(
    game: &mut GameState,
    mut result: TraitEventResult,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    let mut programs = Vec::new();
    loop {
        // Added programs are part of the proposal, not executable preparation
        // work. Keep them in application order while exposing the pending
        // destination interaction to its existing continuation driver.
        let (original, added) = result.into_expansion();
        programs.extend(added);
        result = original;
        let TraitEventResult::NeedsInteraction {
            decision_ctx: crate::decisions::context::DecisionContext::SelectOptions(ctx),
            redirect_zone,
            destinations: Some(destinations),
            effect_id,
            event,
            applied_effects,
            applied_effect_keys,
            ..
        } = &result
        else {
            return Ok(if programs.is_empty() {
                result
            } else {
                TraitEventResult::Expanded {
                    original: Box::new(result),
                    programs,
                }
            });
        };
        if dm.awaiting_choice() {
            return Ok(if programs.is_empty() {
                result
            } else {
                TraitEventResult::Expanded {
                    original: Box::new(result),
                    programs,
                }
            });
        }
        let chosen_zone = dm
            .decide_options(game, ctx)
            .first()
            .and_then(|idx| destinations.get(*idx))
            .copied()
            .unwrap_or(*redirect_zone);
        if dm.awaiting_choice() {
            return Ok(if programs.is_empty() {
                result
            } else {
                TraitEventResult::Expanded {
                    original: Box::new(result),
                    programs,
                }
            });
        }
        let rewritten =
            apply_trait_change_destination(event, chosen_zone).unwrap_or_else(|| (**event).clone());
        // A destination interaction is applied only after an answer exists.
        game.effect_store
            .replacement_effects
            .mark_effect_used(*effect_id);
        let applied_effects = applied_effects.clone();
        let applied_effect_keys = applied_effect_keys.clone();
        result = process_with_dm_and_additional_effects_and_applied(
            game,
            rewritten,
            dm,
            additional_effects,
            &applied_effects,
            &applied_effect_keys,
            source_snapshot,
        )?;
    }
}

fn process_zone_change_inner(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<PreparedEventOutcome<PreparedZoneChange>, crate::effects::ExecutionError> {
    prepare_zone_change_with_context_and_additional_effects(
        game,
        object,
        from,
        to,
        cause,
        dm,
        additional_effects,
        snapshot,
    )
}

/// Retained replacement subtree receipts owned by a compound zone instruction.
/// This is transaction-local state, never a runtime label or a GameState queue.
#[derive(Default)]
pub(crate) struct ZoneDrawContinuations(
    pub Vec<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>,
    /// Already-published entry programmes are retained beside deferred draws,
    /// never counted as draws or replayed by a continuation.
    pub Vec<crate::effects::PublishedEffectOutputs>,
    // Selected semantic actions are not committed prefix receipts.
    Vec<PreparedTypedReplacementMove>,
);
impl ZoneDrawContinuations {
    pub(crate) fn checkpoint(&self) -> (usize, usize, usize) {
        (self.0.len(), self.1.len(), self.2.len())
    }
    pub(crate) fn restore_checkpoint(&mut self, checkpoint: (usize, usize, usize)) {
        self.0.truncate(checkpoint.0);
        self.1.truncate(checkpoint.1);
        self.2.truncate(checkpoint.2);
    }

    pub(crate) fn has_pending_originals(&self) -> bool {
        !self.2.is_empty()
    }

    /// Transfer, freeze and completion may consume only committed originals.
    /// Selected actions retain their independent carrier until exact-object
    /// commitment; a draw-prefix packet is never evidence that they ran.
    pub(crate) fn require_committed_originals(
        &self,
        message: &'static str,
    ) -> Result<(), crate::effects::ExecutionError> {
        if self.has_pending_originals() {
            return Err(crate::effects::ExecutionError::InternalError(
                message.into(),
            ));
        }
        Ok(())
    }

    /// Consume the exact selected action once, during the owner's original phase.
    /// Its resulting prefix receipts and draw tail remain in this same carrier.
    pub(crate) fn commit_pending_replacement(
        &mut self,
        game: &mut GameState,
        object: ObjectId,
        dm: &mut dyn DecisionMaker,
    ) -> Result<(), crate::effects::ExecutionError> {
        if let Some(index) = self.2.iter().position(|prepared| prepared.object == object) {
            let prepared = self.2.remove(index);
            commit_prepared_typed_replacement_move(game, prepared, dm, true)?.transfer_to(
                game,
                dm,
                Some(self),
            )?;
        }
        Ok(())
    }
}
impl std::fmt::Debug for ZoneDrawContinuations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ZoneDrawContinuations")
            .field(&self.0.len())
            .field(&self.1.len())
            .field(&self.2.len())
            .finish()
    }
}

/// A zone proposal whose entry programs and choices have completed. A commit
/// consumes this record; it must not reconstruct an entry from `final_zone`.
#[derive(Debug, Clone)]
pub struct PreparedZoneChange {
    pub(crate) final_zone: Zone,
    pub(crate) entry: Option<crate::game_state::PreparedEtbEntry>,
    pub(crate) context: ReplacementEventContext,
    pub(crate) pre_event_lookback: Vec<crate::snapshot::ObjectSnapshot>,
}

impl PreparedZoneChange {
    pub fn final_zone(&self) -> Zone {
        self.final_zone
    }

    fn original_zone_context(
        &self,
    ) -> Result<&crate::events::ZoneChangeEvent, crate::effects::ExecutionError> {
        self.context
            .zone_change_context
            .as_ref()
            .or_else(|| {
                crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                    self.context.event.inner(),
                )
            })
            .ok_or_else(|| {
                crate::effects::ExecutionError::InternalError(
                    "prepared zone commit has no original zone context".into(),
                )
            })
    }

    /// A sibling replacement can consume this exact identity before its turn.
    /// Such an original is no longer applicable; never follow its successor.
    pub(crate) fn original_is_current(
        &self,
        game: &GameState,
        object: ObjectId,
    ) -> Result<bool, crate::effects::ExecutionError> {
        let original = self.original_zone_context()?;
        if original.objects.as_slice() != [object] {
            return Err(crate::effects::ExecutionError::InternalError(
                "prepared zone commit no longer matches its original object and zone".into(),
            ));
        }
        Ok(game
            .object(object)
            .is_some_and(|current| current.zone == original.from))
    }
}

/// An unresolved battlefield proposal. Its fields are kept together until
/// the entry owner supplies all authored options and batch reservations.
#[derive(Debug, Clone)]
pub(crate) struct PreparedBattlefieldZoneChange {
    context: ReplacementEventContext,
    additional_effects: Vec<ReplacementEffect>,
    pre_event_lookback: Vec<crate::snapshot::ObjectSnapshot>,
    programs: Vec<PreparedReplacementProgram>,
}

impl PreparedBattlefieldZoneChange {
    pub(crate) fn prepend_programs(&mut self, mut programs: Vec<PreparedReplacementProgram>) {
        programs.append(&mut self.programs);
        self.programs = programs;
    }
    pub(crate) fn take_programs(&mut self) -> Vec<PreparedReplacementProgram> {
        std::mem::take(&mut self.programs)
    }

    pub(crate) fn into_entry_scope(
        self,
    ) -> (
        ReplacementEventContext,
        Vec<ReplacementEffect>,
        Vec<crate::snapshot::ObjectSnapshot>,
        Vec<PreparedReplacementProgram>,
    ) {
        (
            self.context,
            self.additional_effects,
            self.pre_event_lookback,
            self.programs,
        )
    }
}

/// A proposal is either ready for movement or requires the entry phase. A
/// battlefield handoff cannot accidentally reach the raw zone commit API.
#[derive(Debug, Clone)]
pub(crate) enum PreparedZoneProposal {
    Ready(PreparedZoneChange),
    Battlefield(PreparedBattlefieldZoneChange),
}

/// Commit the exact prepared proposal. No replacement is run a second time
/// and no caller-supplied destination/controller can overwrite its result.
/// Exact committed movement with the entry producer's already-published facts.
/// Programs remain deferred; carrying references never executes the producer again.
pub(crate) struct CommittedZoneChange<T> {
    pub receipt: PreparedEventOutcome<T>,
    pub published_outputs: Vec<crate::effects::PublishedEffectOutputs>,
}

impl<T> CommittedZoneChange<T> {
    pub(crate) fn from_receipt(receipt: PreparedEventOutcome<T>) -> Self {
        Self {
            receipt,
            published_outputs: Vec::new(),
        }
    }
}

/// Receipt-only compatibility shares the same physical committer.
#[allow(dead_code)]
pub(crate) fn commit_prepared_zone_change(
    game: &mut GameState,
    object: ObjectId,
    prepared: PreparedZoneChange,
    dm: &mut dyn DecisionMaker,
) -> Result<PreparedEventOutcome<ObjectId>, crate::effects::ExecutionError> {
    commit_prepared_zone_change_with_outputs(game, object, prepared, dm)
        .map(|committed| committed.receipt)
}

pub(crate) fn commit_prepared_zone_change_with_outputs(
    game: &mut GameState,
    object: ObjectId,
    prepared: PreparedZoneChange,
    dm: &mut dyn DecisionMaker,
) -> Result<CommittedZoneChange<ObjectId>, crate::effects::ExecutionError> {
    if dm.awaiting_choice() {
        return Ok(CommittedZoneChange::from_receipt(
            PreparedEventOutcome::pure(EventOutcome::Prevented),
        ));
    }
    let original = prepared.original_zone_context()?;
    if !prepared.original_is_current(game, object)? {
        return Err(crate::effects::ExecutionError::InternalError(
            "prepared zone commit no longer matches its original object and zone".into(),
        ));
    }
    let original_from = original.from;
    let cause = original.cause.clone();
    let snapshot = original.snapshot.clone();
    let checkpoint = game.clone();
    let mut published_outputs = Vec::new();
    let result = (|| -> Result<PreparedEventOutcome<ObjectId>, crate::effects::ExecutionError> {
        if let Some(entry) = prepared.entry {
            let committed = game.commit_prepared_etb_with_cause_and_options_and_dm(
                object, entry, None, cause, true, dm,
            )?;
            if committed.pending || dm.awaiting_choice() {
                return Ok(PreparedEventOutcome::pure(EventOutcome::Prevented));
            }
            published_outputs.extend(committed.published_outputs);
            let original = match committed.original {
                EventOutcome::Proceed(entered) => {
                    // An impossible Aura entry can restore the original card;
                    // it has no destination identity and is not a movement.
                    if entered.new_id == object
                        && game
                            .object(object)
                            .is_some_and(|card| card.zone == original_from)
                    {
                        EventOutcome::Prevented
                    } else {
                        EventOutcome::Proceed(entered.new_id)
                    }
                }
                EventOutcome::Prevented => EventOutcome::Prevented,
                EventOutcome::Replaced => EventOutcome::Replaced,
                EventOutcome::NotApplicable => EventOutcome::NotApplicable,
            };
            Ok(PreparedEventOutcome {
                original,
                programs: committed.programs,
            })
        } else {
            let moved = game.move_object_with_snapshot_and_pre_event_lookback(
                object,
                prepared.final_zone,
                cause,
                snapshot,
                &prepared.pre_event_lookback,
            );
            Ok(PreparedEventOutcome::pure(match moved {
                Some(id) => EventOutcome::Proceed(id),
                None => EventOutcome::NotApplicable,
            }))
        }
    })();
    if result.is_err() || dm.awaiting_choice() {
        game.restore_execution_checkpoint(checkpoint, result.is_ok() && dm.awaiting_choice());
    }
    if dm.awaiting_choice() {
        return result.map(|_| {
            CommittedZoneChange::from_receipt(PreparedEventOutcome::pure(EventOutcome::Prevented))
        });
    }
    result.map(|receipt| CommittedZoneChange {
        receipt,
        published_outputs,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_zone_change_with_context_and_additional_effects(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<PreparedEventOutcome<PreparedZoneChange>, crate::effects::ExecutionError> {
    prepare_zone_change_scoped(
        game,
        object,
        from,
        to,
        cause,
        dm,
        additional_effects,
        snapshot,
        None,
        Vec::new(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_zone_change_scoped(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    inherited: Option<&ReplacementEventContext>,
    initial_counters: Vec<(CounterType, u32)>,
    inherited_lookback: Option<&[crate::snapshot::ObjectSnapshot]>,
) -> Result<PreparedEventOutcome<PreparedZoneChange>, crate::effects::ExecutionError> {
    prepare_zone_change_scoped_with_draws(
        game,
        object,
        from,
        to,
        cause,
        dm,
        additional_effects,
        snapshot,
        inherited,
        initial_counters,
        inherited_lookback,
        None,
    )
}

/// A compound original retains replacement-created draws until its remaining
/// unreplaced parts finish (CR 121.7). Non-draw prefixes run in their own scope.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_zone_change_scoped_with_draws(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    inherited: Option<&ReplacementEventContext>,
    initial_counters: Vec<(CounterType, u32)>,
    inherited_lookback: Option<&[crate::snapshot::ObjectSnapshot]>,
    mut draws: Option<&mut ZoneDrawContinuations>,
) -> Result<PreparedEventOutcome<PreparedZoneChange>, crate::effects::ExecutionError> {
    if dm.awaiting_choice() {
        return Ok(PreparedEventOutcome::pure(EventOutcome::Prevented));
    }
    // This checkpoint precedes commander choices, registration refresh and
    // every one-shot consumption in both the zone and entry phases.
    let checkpoint = game.clone();
    let draw_checkpoint = draws.as_ref().map_or((0, 0, 0), |draws| draws.checkpoint());
    let outcome = (|| {
        let PreparedEventOutcome {
            original,
            mut programs,
        } = prepare_zone_change_proposal_scoped_with_draws(
            game,
            object,
            from,
            to,
            cause,
            dm,
            additional_effects,
            snapshot,
            inherited,
            inherited_lookback,
            draws.as_deref_mut(),
        )?;
        let mut completed = match original {
            EventOutcome::Proceed(PreparedZoneProposal::Ready(prepared)) => {
                PreparedEventOutcome::pure(EventOutcome::Proceed(prepared))
            }
            EventOutcome::Proceed(PreparedZoneProposal::Battlefield(proposal)) => {
                complete_zone_entry_proposal(
                    game,
                    object,
                    proposal,
                    dm,
                    initial_counters,
                    draws.as_deref_mut(),
                )?
            }
            EventOutcome::Prevented => PreparedEventOutcome::pure(EventOutcome::Prevented),
            EventOutcome::Replaced => PreparedEventOutcome::pure(EventOutcome::Replaced),
            EventOutcome::NotApplicable => PreparedEventOutcome::pure(EventOutcome::NotApplicable),
        };
        programs.append(&mut completed.programs);
        completed.programs = programs;
        Ok(completed)
    })();
    if outcome.is_err() || dm.awaiting_choice() {
        game.restore_execution_checkpoint(checkpoint, outcome.is_ok() && dm.awaiting_choice());
        if let Some(draws) = draws {
            draws.restore_checkpoint(draw_checkpoint);
        }
    }
    if dm.awaiting_choice() {
        return outcome.map(|_| PreparedEventOutcome::pure(EventOutcome::Prevented));
    }
    outcome
}

/// Resolve zone replacements up to the battlefield-entry boundary. Entry
/// callers retain this receipt while supplying tapped/controller/counter/face
/// options; they must not restart processing from a scalar destination.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_zone_change_proposal_scoped(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    inherited: Option<&ReplacementEventContext>,
    inherited_lookback: Option<&[crate::snapshot::ObjectSnapshot]>,
) -> Result<PreparedEventOutcome<PreparedZoneProposal>, crate::effects::ExecutionError> {
    prepare_zone_change_proposal_scoped_with_draws(
        game,
        object,
        from,
        to,
        cause,
        dm,
        additional_effects,
        snapshot,
        inherited,
        inherited_lookback,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_zone_change_proposal_scoped_with_draws(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    inherited: Option<&ReplacementEventContext>,
    inherited_lookback: Option<&[crate::snapshot::ObjectSnapshot]>,
    mut draws: Option<&mut ZoneDrawContinuations>,
) -> Result<PreparedEventOutcome<PreparedZoneProposal>, crate::effects::ExecutionError> {
    if dm.awaiting_choice() {
        return Ok(PreparedEventOutcome::pure(EventOutcome::Prevented));
    }
    let checkpoint = game.clone();
    let draw_checkpoint = draws.as_ref().map_or((0, 0, 0), |draws| draws.checkpoint());
    let result = prepare_zone_change_with_context_inner(
        game,
        object,
        from,
        to,
        cause,
        dm,
        additional_effects,
        snapshot,
        inherited,
        inherited_lookback,
        draws.as_deref_mut(),
    );
    if result.is_err() || dm.awaiting_choice() {
        game.restore_execution_checkpoint(checkpoint, result.is_ok() && dm.awaiting_choice());
        if let Some(draws) = draws {
            draws.restore_checkpoint(draw_checkpoint);
        }
    }
    if dm.awaiting_choice() {
        return result.map(|_| PreparedEventOutcome::pure(EventOutcome::Prevented));
    }
    result
}

fn complete_zone_entry_proposal(
    game: &mut GameState,
    object: ObjectId,
    proposal: PreparedBattlefieldZoneChange,
    dm: &mut dyn DecisionMaker,
    initial_counters: Vec<(CounterType, u32)>,
    mut draws: Option<&mut ZoneDrawContinuations>,
) -> Result<PreparedEventOutcome<PreparedZoneChange>, crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    let (context, additional_effects, pre_event_lookback, mut programs) =
        proposal.into_entry_scope();
    let mut entry_result = process_etb_from_zone_change_context_with_options_and_draws(
        game,
        context,
        dm,
        initial_counters,
        &additional_effects,
        draws.as_deref_mut(),
    )?;
    if dm.awaiting_choice() {
        return Ok(PreparedEventOutcome::pure(EventOutcome::Prevented));
    }
    if let Some(retained) = draws.as_deref_mut() {
        crate::effects::PublishedEffectOutputs::append_distinct(
            &mut retained.1,
            entry_result.completed_program_outputs.iter().cloned(),
        );
    }
    programs.append(&mut entry_result.additional_programs);
    if entry_result.replaced {
        return Ok(PreparedEventOutcome {
            original: EventOutcome::Replaced,
            programs,
        });
    }
    if entry_result.prevented && entry_result.new_destination.is_none() {
        return Ok(PreparedEventOutcome {
            original: EventOutcome::Prevented,
            programs,
        });
    }
    let context = entry_result
        .replacement_context
        .as_deref()
        .cloned()
        .ok_or_else(|| {
            ExecutionError::InternalError("completed zone entry has no replacement receipt".into())
        })?;
    let final_zone = entry_result.new_destination.unwrap_or(Zone::Battlefield);
    let Some(mut entry) =
        game.prepare_etb_entry_with_controller_and_dm(object, entry_result, None, dm)?
    else {
        return Ok(PreparedEventOutcome {
            original: EventOutcome::Prevented,
            programs,
        });
    };
    if let Some(retained) = draws {
        crate::effects::PublishedEffectOutputs::append_distinct(
            &mut retained.1,
            entry.choices.as_enters_outputs.iter().cloned(),
        );
        crate::effects::PublishedEffectOutputs::append_distinct(
            &mut retained.1,
            entry.result.completed_program_outputs.iter().cloned(),
        );
    }
    entry.zone_entry_lookback = Some(pre_event_lookback.clone());
    Ok(PreparedEventOutcome {
        original: EventOutcome::Proceed(PreparedZoneChange {
            final_zone,
            entry: Some(entry),
            context,
            pre_event_lookback,
        }),
        programs,
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_zone_change_with_context_inner(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    inherited: Option<&ReplacementEventContext>,
    inherited_lookback: Option<&[crate::snapshot::ObjectSnapshot]>,
    mut draws: Option<&mut ZoneDrawContinuations>,
) -> Result<PreparedEventOutcome<PreparedZoneProposal>, crate::effects::ExecutionError> {
    use crate::effects::{ExecutionContext, ExecutionError};
    use crate::events::{ZoneChangeEvent, downcast_event};
    if !game
        .object(object)
        .is_some_and(|object| object.zone == from)
    {
        return Ok(PreparedEventOutcome::pure(EventOutcome::NotApplicable));
    }
    // Ordinary same-zone instructions do not create a movement or an entry.
    // Exile and command renew identity even without changing zones (400.8/400.10).
    if from == to && !matches!(to, Zone::Exile | Zone::Command) {
        return Ok(PreparedEventOutcome::pure(EventOutcome::NotApplicable));
    }
    // Freeze lookback before an entry/replacement program can change sources.
    let pre_event_lookback = match inherited_lookback {
        Some(snapshots) => snapshots.to_vec(),
        None => game.try_trigger_source_lookback_snapshots()?,
    };
    game.update_replacement_effects()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    let requested_to = game.resolve_commander_move_destination(object, to, dm);
    if dm.awaiting_choice() {
        return Ok(PreparedEventOutcome::pure(EventOutcome::Prevented));
    }
    let snapshot = match snapshot {
        Some(snapshot) => Some(snapshot),
        None => game
            .object(object)
            .map(|object| {
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                    object, game,
                )
            })
            .transpose()?,
    };
    let zone = ZoneChangeEvent::with_cause(object, from, requested_to, cause.clone(), snapshot);
    // Match merged-component policies against the original proposal, before
    // one-shots disappear or replacement destinations change its matchers.
    let merged_destinations =
        merged_card_only_change_destinations(game, &zone, additional_effects)?;
    let mut effects = additional_effects.to_vec();
    if from == Zone::Battlefield && requested_to == Zone::Graveyard {
        effects.extend(finality_counter_replacements(game, object));
    }
    assign_ephemeral_effect_ids(&mut effects, (u64::MAX / 2).saturating_add(1024));
    let mut state = TraitEventProcessingState {
        defer_battlefield_entry: true,
        zone_change_context: Some(zone.clone()),
        ..Default::default()
    };
    let applied_ids = inherited
        .map(|context| context.applied_effects.clone())
        .unwrap_or_default();
    let applied_keys = inherited
        .map(|context| context.applied_effect_keys.clone())
        .unwrap_or_default();
    let provenance = inherited
        .map(|context| context.event.provenance())
        .unwrap_or_default();
    let mut programs = Vec::new();
    let original = (|| -> Result<EventOutcome<PreparedZoneProposal>, ExecutionError> {
        let mut result = process_with_dm_and_additional_effects_and_applied_state(
            game,
            Event::new_with_provenance(zone, provenance),
            dm,
            &effects,
            &applied_ids,
            &applied_keys,
            None,
            &mut state,
        )?;
        // Keep the original zone context and the same state while destination
        // choices continue; rebuilding a default state would lose the handoff.
        loop {
            let (original, added) = result.into_expansion();
            programs.extend(added);
            result = original;
            let TraitEventResult::NeedsInteraction {
                decision_ctx: crate::decisions::context::DecisionContext::SelectOptions(ctx),
                redirect_zone,
                destinations: Some(destinations),
                effect_id,
                event,
                applied_effects,
                applied_effect_keys,
                ..
            } = &result
            else {
                break;
            };
            let chosen = dm.decide_options(game, ctx);
            if dm.awaiting_choice() {
                return Ok(EventOutcome::Prevented);
            }
            let unique: std::collections::HashSet<_> = chosen.iter().copied().collect();
            if chosen.len() < ctx.min
                || chosen.len() > ctx.max
                || chosen.len() > 1
                || unique.len() != chosen.len()
                || chosen.iter().any(|index| {
                    destinations.get(*index).is_none()
                        || !ctx
                            .options
                            .iter()
                            .any(|option| option.index == *index && option.legal)
                })
            {
                return Err(ExecutionError::InternalError(
                    "invalid response to zone replacement destination choice".into(),
                ));
            }
            // An empty selection is only a decline when the captured context
            // explicitly allows it; invalid selections never become a decline.
            let destination = chosen
                .first()
                .map(|index| destinations[*index])
                .unwrap_or(*redirect_zone);
            let event = apply_trait_change_destination(event, destination).ok_or_else(|| {
                ExecutionError::InternalError(
                    "destination replacement cannot modify the retained zone carrier".into(),
                )
            })?;
            game.effect_store
                .replacement_effects
                .mark_effect_used(*effect_id);
            let applied_effects = applied_effects.clone();
            let applied_effect_keys = applied_effect_keys.clone();
            result = process_with_dm_and_additional_effects_and_applied_state(
                game,
                event,
                dm,
                &effects,
                &applied_effects,
                &applied_effect_keys,
                None,
                &mut state,
            )?;
        }
        if dm.awaiting_choice() {
            return Ok(EventOutcome::Prevented);
        }
        let scoped_additional_effects = effects.clone();
        match result {
            TraitEventResult::Prevented => Ok(EventOutcome::Prevented),
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                let change = downcast_event::<ZoneChangeEvent>(event.inner())
                    .cloned()
                    .ok_or_else(|| {
                        ExecutionError::InternalError(
                            "zone preparation lost its unresolved zone carrier".into(),
                        )
                    })?;
                // A departure redirected to its current ordinary zone has already
                // consumed the applicable replacement, but produces no entry/move.
                if change.to == from && !matches!(from, Zone::Exile | Zone::Command) {
                    return Ok(EventOutcome::NotApplicable);
                }
                let mut context = ReplacementEventContext::new(&game, event, &state);
                if change.to == Zone::Battlefield {
                    return Ok(EventOutcome::Proceed(PreparedZoneProposal::Battlefield(
                        PreparedBattlefieldZoneChange {
                            context,
                            additional_effects: scoped_additional_effects,
                            pre_event_lookback,
                            programs: std::mem::take(&mut programs),
                        },
                    )));
                }
                let mut final_zone = change.to;
                if final_zone != requested_to && matches!(final_zone, Zone::Hand | Zone::Library) {
                    final_zone = game.resolve_commander_move_destination(object, final_zone, dm);
                }
                if dm.awaiting_choice() {
                    return Ok(EventOutcome::Prevented);
                }
                // Split-component preparation remains part of this owner. Its
                // decisions are covered by the checkpoint above.
                if final_zone != change.to {
                    let mut resolved_change = change.clone();
                    resolved_change.to = final_zone;
                    context.event =
                        Event::new_with_provenance(resolved_change, context.event.provenance());
                }
                if merged_destinations.contains(&final_zone) {
                    game.prepare_merged_token_card_component_destinations(object, to, final_zone);
                } else {
                    game.prepare_merged_component_destinations(object, final_zone, dm);
                }
                Ok(EventOutcome::Proceed(PreparedZoneProposal::Ready(
                    PreparedZoneChange {
                        final_zone,
                        entry: None,
                        context,
                        pre_event_lookback,
                    },
                )))
            }
            TraitEventResult::Replaced {
                effects,
                replacement,
                source,
                controller,
                context,
                ..
            } => {
                if replacement_moves_object(&replacement) {
                    if let Some(retained) = draws.as_deref_mut() {
                        if let Some(prepared) = prepare_typed_replacement_move(
                            game,
                            object,
                            &replacement,
                            &effects,
                            source,
                            controller,
                            cause,
                            dm,
                            &context,
                            &scoped_additional_effects,
                            &pre_event_lookback,
                            None,
                            true,
                        )? {
                            retained.2.push(prepared);
                        }
                        return Ok(EventOutcome::Replaced);
                    }
                    execute_typed_replacement_move(
                        game,
                        object,
                        &replacement,
                        &effects,
                        source,
                        controller,
                        cause,
                        dm,
                        &context,
                        &scoped_additional_effects,
                        &pre_event_lookback,
                        None,
                        draws.as_deref_mut(),
                    )?;
                    return Ok(EventOutcome::Replaced);
                }
                let mut ctx = ExecutionContext::new(source, controller, dm);
                ctx.replacement.additional_replacement_effects = scoped_additional_effects;
                if let Some(draws) = draws.as_deref_mut() {
                    let snapshot = context
                        .zone_change_context
                        .as_ref()
                        .and_then(|zone| zone.snapshot.clone())
                        .into_iter()
                        .collect::<Vec<_>>();
                    if let Some(prepared) = crate::effects::replacement::prepare_draw_continuation_with_bindings_and_outputs(
                        game, &mut ctx, &effects, source, controller, &context, None,
                        crate::effects::replacement::ReplacementProgramBindings {
                            targets: None,
                            object_tags: vec![("it".into(), snapshot.clone()), ("__it__".into(), snapshot)],
                        },
                    )? {
                        draws.0.push(prepared);
                        return Ok(EventOutcome::Replaced);
                    }
                }
                let mut outcome = crate::effects::replacement::execute_replacement_payload(
                    game, &mut ctx, &effects, source, controller, &context, None,
                )?;
                crate::effects::retain_unmatched_outcome_events(game, &mut outcome.events);
                for event in outcome.events {
                    game.queue_trigger_event(event.provenance(), event);
                }
                Ok(EventOutcome::Replaced)
            }
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                Err(ExecutionError::InternalError(
                    "zone replacement suspended without a supported captured choice".into(),
                ))
            }
            TraitEventResult::Expanded { .. } => Err(ExecutionError::InternalError(
                "zone preparation received an unflattened result".into(),
            )),
        }
    })();
    original.map(|original| PreparedEventOutcome { original, programs })
}

/// Execute a compound replacement under its owning zone checkpoint. Every
/// nested zone/counter event sees the applications that created it; errors or
/// pending input escape to the owner before any partial result is committed.
#[allow(clippy::too_many_arguments)]
fn execute_typed_replacement_move(
    game: &mut GameState,
    object: ObjectId,
    replacement: &ReplacementAction,
    follow_ups: &[crate::effect::Effect],
    source: ObjectId,
    controller: PlayerId,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    context: &ReplacementEventContext,
    additional_effects: &[ReplacementEffect],
    pre_event_lookback: &[crate::snapshot::ObjectSnapshot],
    replacement_scope: Option<&crate::effects::ReplacementExecutionContext>,
    draws: Option<&mut ZoneDrawContinuations>,
) -> Result<(), crate::effects::ExecutionError> {
    let prepared = prepare_typed_replacement_move(
        game,
        object,
        replacement,
        follow_ups,
        source,
        controller,
        cause,
        dm,
        context,
        additional_effects,
        pre_event_lookback,
        replacement_scope,
        draws.is_some(),
    )?;
    let Some(prepared) = prepared else {
        return Ok(());
    };
    commit_prepared_typed_replacement_move(game, prepared, dm, draws.is_some())?
        .transfer_to(game, dm, draws)
        .map(|_| ())
}

/// Capture the nested zone proposal separately from its physical commitment.
/// Legacy nested preparation can still commit replacement prefixes; these are
/// explicit retained receipts, not fabricated completed outcomes for this move.
struct PreparedTypedReplacementMove {
    object: ObjectId,
    source: ObjectId,
    controller: PlayerId,
    counters: Vec<(CounterType, u32)>,
    link: bool,
    proposal: PreparedEventOutcome<PreparedZoneChange>,
    nested_draws: ZoneDrawContinuations,
    source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    context: ReplacementEventContext,
    additional_effects: Vec<ReplacementEffect>,
    replacement_scope: Option<crate::effects::ReplacementExecutionContext>,
    followups: Vec<crate::effect::Effect>,
}

#[allow(clippy::too_many_arguments)]
fn prepare_typed_replacement_move(
    game: &mut GameState,
    object: ObjectId,
    replacement: &ReplacementAction,
    follow_ups: &[crate::effect::Effect],
    source: ObjectId,
    controller: PlayerId,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn DecisionMaker,
    context: &ReplacementEventContext,
    additional_effects: &[ReplacementEffect],
    pre_event_lookback: &[crate::snapshot::ObjectSnapshot],
    replacement_scope: Option<&crate::effects::ReplacementExecutionContext>,
    retain_draws: bool,
) -> Result<Option<PreparedTypedReplacementMove>, crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    let (destination, counters, link) = match replacement {
        ReplacementAction::MoveToZoneWithCounters { zone, counters } => {
            (*zone, counters.as_slice(), false)
        }
        ReplacementAction::ExileWithSourceLink | ReplacementAction::ExileWithSourceLinkThen(_) => {
            (Zone::Exile, &[][..], true)
        }
        ReplacementAction::ExileWithSourceLinkCountersThen { counters, .. } => {
            (Zone::Exile, counters.as_slice(), true)
        }
        _ => {
            return Err(ExecutionError::InternalError(
                "compound zone executor received a non-moving replacement".into(),
            ));
        }
    };
    let Some(from) = game.object(object).map(|object| object.zone) else {
        return Ok(None);
    };
    let snapshot = context
        .zone_change_context
        .as_ref()
        .and_then(|zone| zone.snapshot.clone())
        .or_else(|| {
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(context.event.inner())
                .and_then(|zone| zone.snapshot.clone())
        });
    let source_snapshot = game
        .object(source)
        .map(|object| {
            crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .transpose()?
        .or_else(|| game.source_last_known_snapshot(source).cloned());
    let mut nested_draws = ZoneDrawContinuations::default();
    let proposal = prepare_zone_change_scoped_with_draws(
        game,
        object,
        from,
        destination,
        cause,
        dm,
        additional_effects,
        snapshot,
        Some(context),
        counters.to_vec(),
        Some(pre_event_lookback),
        retain_draws.then_some(&mut nested_draws),
    )?;
    if dm.awaiting_choice() {
        return Ok(None);
    }
    Ok(Some(PreparedTypedReplacementMove {
        object,
        source,
        controller,
        counters: counters.to_vec(),
        link,
        proposal,
        nested_draws,
        source_snapshot,
        context: context.clone(),
        additional_effects: additional_effects.to_vec(),
        replacement_scope: replacement_scope.cloned(),
        followups: follow_ups.to_vec(),
    }))
}

/// One native retained original, before erasing its typed completion boundary.
/// Prefix projection retains the real child scopes and published references;
/// it neither executes another action nor fabricates completed original data.
struct RetainedTypedMoveOriginal {
    prefix: crate::effects::CompletedEffectOutputs,
    completion: TypedMoveDrawCompletion,
}

impl RetainedTypedMoveOriginal {
    fn new(completion: TypedMoveDrawCompletion) -> Self {
        let mut prefix = crate::effects::CompletedEffectOutputs::from_children(
            completion
                .nested
                .0
                .iter()
                .map(|receipt| receipt.outcome.clone_projection()),
            crate::effect::EffectOutcome::aggregate,
        );
        prefix.retain_published_references(completion.nested.1.iter().cloned());
        prefix.projections_complete = false;
        Self { prefix, completion }
    }

    fn into_continuations(self) -> ZoneDrawContinuations {
        let mut continuations = ZoneDrawContinuations::default();
        continuations
            .0
            .push(crate::effects::SimultaneousEffectCommit {
                outcome: self.prefix,
                completion: Some(Box::new(self.completion)),
            });
        continuations
    }
}

/// The native original is assembled, while generated additions have not begun.
/// Keep its exact arrival and captured frame available to a composing owner.
struct ReadyTypedMoveOriginal {
    original: TypedMoveOriginalReceipt,
    nested: ZoneDrawContinuations,
    programs: Vec<PreparedReplacementProgram>,
    followups: Vec<crate::effect::Effect>,
    scope: crate::effects::ExecutionContextCheckpoint,
}

impl ReadyTypedMoveOriginal {
    fn prepare_tail(
        mut self,
        game: &mut GameState,
        dm: &mut dyn DecisionMaker,
        recipient: &mut ZoneDrawContinuations,
    ) -> Result<Option<TypedMoveOriginalOutputs>, crate::effects::ExecutionError> {
        // These are actual finished prefixes and published handles, not another
        // physical execution. Preserve their native order ahead of this tail.
        recipient.0.append(&mut self.nested.0);
        recipient.1.append(&mut self.nested.1);
        recipient.2.append(&mut self.nested.2);
        let mut ctx = self.scope.reborrow(dm);
        let outputs = self.original.complete_tail(
            game,
            &mut ctx,
            self.programs,
            &self.followups,
            Some(recipient),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            Ok(None)
        } else {
            Ok(Some(outputs))
        }
    }
}

/// The native committer owns both its readiness and actual continuation carrier.
/// A retained original is distinct from a pending decision, even after physical
/// movement. Only a completed authored original supplies final arrival outputs.
enum TypedMoveOriginalCommit {
    Suspended(ZoneDrawContinuations),
    RetainedOriginal(RetainedTypedMoveOriginal),
    ReadyOriginal(ReadyTypedMoveOriginal),
    Complete {
        original: TypedMoveOriginalOutputs,
        continuations: ZoneDrawContinuations,
    },
}

impl TypedMoveOriginalCommit {
    fn finished(
        original: TypedMoveOriginalOutputs,
        continuations: ZoneDrawContinuations,
        pending: bool,
    ) -> Self {
        if pending {
            Self::Suspended(continuations)
        } else {
            Self::Complete {
                original,
                continuations,
            }
        }
    }

    /// Transfer each native carrier once without changing receipt order or
    /// reconstructing an original from a prefix. Atomic callers cannot consume
    /// a still-retained original; their producer must finish it first.
    fn transfer_to(
        self,
        game: &mut GameState,
        dm: &mut dyn DecisionMaker,
        recipient: Option<&mut ZoneDrawContinuations>,
    ) -> Result<Option<TypedMoveOriginalOutputs>, crate::effects::ExecutionError> {
        let (original, mut continuations, retained) = match self {
            Self::ReadyOriginal(original) => {
                let recipient = recipient.ok_or_else(|| {
                    crate::effects::ExecutionError::InternalError(
                        "atomic typed movement cannot consume an unprepared added tail".into(),
                    )
                })?;
                return original.prepare_tail(game, dm, recipient);
            }
            Self::Suspended(continuations) => (None, continuations, false),
            Self::RetainedOriginal(original) => (None, original.into_continuations(), true),
            Self::Complete {
                original,
                continuations,
            } => (Some(original), continuations, false),
        };
        if let Some(recipient) = recipient {
            recipient.0.append(&mut continuations.0);
            recipient.1.append(&mut continuations.1);
            recipient.2.append(&mut continuations.2);
        } else if retained
            || !continuations.0.is_empty()
            || !continuations.1.is_empty()
            || !continuations.2.is_empty()
        {
            return Err(crate::effects::ExecutionError::InternalError(
                "atomic typed movement cannot discard retained original receipts".into(),
            ));
        }
        Ok(original)
    }
}

fn commit_prepared_typed_replacement_move(
    game: &mut GameState,
    prepared: PreparedTypedReplacementMove,
    dm: &mut dyn DecisionMaker,
    retain_draws: bool,
) -> Result<TypedMoveOriginalCommit, crate::effects::ExecutionError> {
    use crate::effects::ExecutionContext;
    let PreparedTypedReplacementMove {
        object,
        source,
        controller,
        counters,
        link,
        proposal,
        mut nested_draws,
        source_snapshot,
        context,
        additional_effects,
        replacement_scope,
        followups,
    } = prepared;
    let PreparedEventOutcome {
        original: proposal,
        mut programs,
    } = proposal;
    // Nested selected actions also belong to commitment, not preparation.
    nested_draws.commit_pending_replacement(game, object, dm)?;
    if dm.awaiting_choice() {
        return Ok(TypedMoveOriginalCommit::Suspended(
            ZoneDrawContinuations::default(),
        ));
    }
    nested_draws
        .require_committed_originals("typed replacement retained an uncommitted nested original")?;
    let mut published_outputs = Vec::new();
    let (new_id, counters_in_entry) = match proposal {
        EventOutcome::Proceed(prepared) if !prepared.original_is_current(game, object)? => {
            (None, false)
        }
        EventOutcome::Proceed(prepared) => {
            let counters_in_entry =
                prepared.final_zone == Zone::Battlefield && prepared.entry.is_some();
            let committed = commit_prepared_zone_change_with_outputs(game, object, prepared, dm)?;
            published_outputs.extend(committed.published_outputs);
            let mut committed = committed.receipt;
            programs.append(&mut committed.programs);
            let id = match committed.original {
                EventOutcome::Proceed(id) => Some(id),
                EventOutcome::Replaced => {
                    let ids = game.take_zone_change_results(object);
                    let id = ids.first().copied();
                    if !ids.is_empty() {
                        game.record_zone_change_results(object, ids);
                    }
                    id
                }
                EventOutcome::Prevented | EventOutcome::NotApplicable => None,
            };
            (id, counters_in_entry)
        }
        EventOutcome::Replaced => (retained_zone_arrival(game, object), false),
        EventOutcome::Prevented | EventOutcome::NotApplicable => (None, false),
    };
    if dm.awaiting_choice() {
        return Ok(TypedMoveOriginalCommit::Suspended(
            ZoneDrawContinuations::default(),
        ));
    }
    let arrival = new_id.and_then(|id| {
        game.object(id)
            .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
    });
    let mut ctx = ExecutionContext::new(source, controller, dm);
    if let Some(scope) = replacement_scope {
        ctx.replacement = scope;
    }
    ctx.source_snapshot = source_snapshot;
    ctx.replacement.additional_replacement_effects = additional_effects;
    ctx.iteration.iterated_player = Some(context.affected_player);
    context.apply_to(&mut ctx);
    if retain_draws {
        nested_draws.1.append(&mut published_outputs);
        if nested_draws.0.iter().any(|draw| draw.completion.is_some()) {
            let original = RetainedTypedMoveOriginal::new(TypedMoveDrawCompletion {
                nested: nested_draws,
                object,
                new_id,
                arrival,
                counters_in_entry,
                counters: counters.to_vec(),
                link,
                programs,
                followups,
                scope: crate::effects::ExecutionContextCheckpoint::capture(&ctx),
            });
            return Ok(TypedMoveOriginalCommit::RetainedOriginal(original));
        }
        let Some(original) = assemble_typed_move_original(
            game,
            &mut ctx,
            object,
            new_id,
            arrival,
            counters_in_entry,
            &counters,
            link,
        )?
        else {
            return Ok(TypedMoveOriginalCommit::Suspended(nested_draws));
        };
        return Ok(TypedMoveOriginalCommit::ReadyOriginal(
            ReadyTypedMoveOriginal {
                original,
                nested: nested_draws,
                programs,
                followups,
                scope: crate::effects::ExecutionContextCheckpoint::capture(&ctx),
            },
        ));
    }
    let original = finish_typed_replacement_move(
        game,
        &mut ctx,
        object,
        new_id,
        arrival,
        counters_in_entry,
        &counters,
        link,
        programs,
        &followups,
        published_outputs,
        None,
    )?;
    Ok(TypedMoveOriginalCommit::finished(
        original,
        ZoneDrawContinuations::default(),
        ctx.decision_maker.awaiting_choice(),
    ))
}

fn retained_zone_arrival(game: &mut GameState, object: ObjectId) -> Option<ObjectId> {
    let ids = game.take_zone_change_results(object);
    let arrival = ids.first().copied();
    if !ids.is_empty() {
        game.record_zone_change_results(object, ids);
    }
    arrival
}

/// One selected typed move instruction. Nested selected replacement subtrees
/// finish in authored order before its counters/link; the enclosing move's
/// added programs and followups are retained separately below.
struct TypedMoveDrawCompletion {
    nested: ZoneDrawContinuations,
    object: ObjectId,
    new_id: Option<ObjectId>,
    arrival: Option<crate::snapshot::ObjectSnapshot>,
    counters_in_entry: bool,
    counters: Vec<(CounterType, u32)>,
    link: bool,
    programs: Vec<PreparedReplacementProgram>,
    followups: Vec<crate::effect::Effect>,
    scope: crate::effects::ExecutionContextCheckpoint,
}
/// The native original boundary retains its strong assembled receipt alongside
/// the actual packet and still-unexecuted additions. Generic phase consumers
/// project this handoff once; native compounds can read the exact arrival from
/// the owned receipt without consulting a later world or aggregate object IDs.
struct TypedMoveOriginalHandoff {
    outputs: crate::effects::CompletedEffectOutputs,
    added: TypedMoveAddedCompletion,
}

impl TypedMoveOriginalHandoff {
    fn into_receipt(
        self,
    ) -> crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs> {
        crate::effects::SimultaneousEffectCommit {
            outcome: self.outputs,
            completion: Some(Box::new(self.added)),
        }
    }
}

impl TypedMoveDrawCompletion {
    fn complete_original(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<Option<TypedMoveOriginalHandoff>, crate::effects::ExecutionError> {
        self.nested
            .require_committed_originals("typed movement has an unseparated original")?;
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.scope.restore_ref(ctx);
        let result = (|| {
            for receipt in &mut self.nested.0 {
                crate::effects::composition::inherit_original_observations(
                    &mut receipt.outcome.outcome,
                    &original.outcome.events,
                );
                receipt.outcome.synchronize_observations();
            }
            let Some(nested) =
                crate::effects::composition::complete_retained_originals_with_outputs(
                    game,
                    ctx,
                    self.nested.0,
                    |_, _, _, _| Ok(()),
                )?
            else {
                return Ok(None);
            };
            let arrival = self
                .new_id
                .or_else(|| retained_zone_arrival(game, self.object));
            let Some(assembled) = assemble_typed_move_original(
                game,
                ctx,
                self.object,
                arrival,
                self.arrival,
                self.counters_in_entry,
                &self.counters,
                self.link,
            )?
            else {
                return Ok(None);
            };
            // This is an ordered authored original, not a simultaneous cohort
            // of its internal actions. Nested instructions and their internal
            // replacements finish before counters/link, as in native execution.
            // Only this move's enclosing programs/followups remain unexecuted.
            let mut outputs = crate::effects::CompletedEffectOutputs::from_children(
                nested
                    .iter()
                    .map(crate::effects::CompletedEffectOutputs::clone_projection)
                    .chain(std::iter::once(assembled.outputs.clone_projection())),
                crate::effect::EffectOutcome::aggregate,
            );
            outputs.retain_owned_child(original);
            outputs.retain_published_references(self.nested.1.iter().cloned());
            outputs.projections_complete = false;
            Ok(Some(TypedMoveOriginalHandoff {
                outputs,
                added: TypedMoveAddedCompletion {
                    nested,
                    published: self.nested.1,
                    original: assembled,
                    programs: self.programs,
                    followups: self.followups,
                    scope: crate::effects::ExecutionContextCheckpoint::capture(ctx),
                },
            }))
        })();
        parent.restore(ctx);
        result
    }
}

impl crate::effects::SimultaneousEffectCompletion for TypedMoveDrawCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        use crate::effects::OriginalPhaseStatus;
        // Selected but uncommitted actions are independent of subtree receipts.
        // Never discard that carrier or infer readiness from its first receipt.
        if self.nested.has_pending_originals() {
            OriginalPhaseStatus::Combined
        } else {
            OriginalPhaseStatus::Retained
        }
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        self.complete_original_phase_from_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        Ok(match self.complete_original(game, ctx, original)? {
            Some(handoff) => handoff.into_receipt(),
            None => crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(
                    crate::effect::EffectOutcome::count(0),
                ),
            ),
        })
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut crate::effects::ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: crate::effects::CompletedEffectOutputs::aggregate_only(original),
            completion: Some(self),
        })
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: original,
            completion: Some(self),
        })
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), crate::effects::ExecutionError> {
        self.nested.require_committed_originals(
            "typed movement completion received an uncommitted replacement original",
        )?;
        for draw in &mut self.nested.0 {
            if let Some(completion) = &mut draw.completion {
                completion.freeze(game)?;
            }
        }
        Ok(())
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut crate::effect::EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.scope.restore_ref(ctx);
        let result = (|| {
            for draw in &mut self.nested.0 {
                crate::effects::composition::inherit_original_observations(
                    &mut draw.outcome.outcome,
                    &original.events,
                );
                if let Some(completion) = &mut draw.completion {
                    crate::effects::composition::observe_original_completion(
                        game,
                        ctx,
                        completion.as_mut(),
                        &mut draw.outcome.outcome,
                    )?;
                }
                draw.outcome.synchronize_observations();
            }
            *original = crate::effect::EffectOutcome::aggregate(
                self.nested
                    .0
                    .iter()
                    .map(|draw| draw.outcome.outcome.clone()),
            );
            Ok(())
        })();
        parent.restore(ctx);
        result
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        prefix: crate::effect::EffectOutcome,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        self.complete_with_outputs(game, ctx, prefix)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        prefix: crate::effect::EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
        self.complete_from_original_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(prefix),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        prefix: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
        self.nested.require_committed_originals(
            "typed movement completion received an uncommitted replacement original",
        )?;
        let receipt = self.complete_original_phase_from_outputs(game, ctx, prefix)?;
        crate::effects::composition::complete_committed_original_with_outputs(game, ctx, receipt)
    }
}

/// The ordered original is complete. Actual nested packets are retained as
/// metadata while the typed move's added tail consumes its frozen receipt.
struct TypedMoveAddedCompletion {
    nested: Vec<crate::effects::CompletedEffectOutputs>,
    published: Vec<crate::effects::PublishedEffectOutputs>,
    original: TypedMoveOriginalReceipt,
    programs: Vec<PreparedReplacementProgram>,
    followups: Vec<crate::effect::Effect>,
    scope: crate::effects::ExecutionContextCheckpoint,
}

impl crate::effects::SimultaneousEffectCompletion for TypedMoveAddedCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        self.prepare_draw_boundary_from_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: original,
            completion: Some(self),
        })
    }

    fn freeze(&mut self, _game: &mut GameState) -> Result<(), crate::effects::ExecutionError> {
        Ok(())
    }

    fn observe_original(
        &mut self,
        _game: &mut GameState,
        _ctx: &mut crate::effects::ExecutionContext,
        original: &mut crate::effect::EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        for outputs in &mut self.nested {
            crate::effects::composition::inherit_original_observations(
                &mut outputs.outcome,
                &original.events,
            );
            outputs.synchronize_observations();
        }
        crate::effects::composition::inherit_original_observations(
            &mut self.original.outputs.outcome,
            &original.events,
        );
        self.original.outputs.synchronize_observations();
        Ok(())
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
        self.complete_from_original_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.scope.restore_ref(ctx);
        let result = (|| {
            let mut children = self.nested;
            children.push(
                self.original
                    .complete_tail(game, ctx, self.programs, &self.followups, None)?
                    .outputs,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    crate::effect::EffectOutcome::count(0),
                ));
            }
            let mut outputs = crate::effects::CompletedEffectOutputs::from_children(
                children,
                crate::effect::EffectOutcome::aggregate,
            );
            outputs.retain_owned_child(original);
            outputs.retain_published_references(self.published);
            outputs.projections_complete = false;
            Ok(outputs)
        })();
        parent.restore(ctx);
        result
    }
}

/// One original's immutable arrival and its actual retained program outputs.
struct TypedMoveOriginalOutputs {
    arrival: Option<crate::snapshot::ObjectSnapshot>,
    outputs: crate::effects::CompletedEffectOutputs,
}

/// Original assembly owns its exact arrival, authored counter/link work,
/// acquired result bindings and actual child packets before any added tail.
struct TypedMoveOriginalReceipt {
    object: ObjectId,
    target: ObjectId,
    target_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    arrival: Option<crate::snapshot::ObjectSnapshot>,
    outputs: crate::effects::CompletedEffectOutputs,
}

#[allow(clippy::too_many_arguments)]
fn assemble_typed_move_original(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    object: ObjectId,
    new_id: Option<ObjectId>,
    arrival: Option<crate::snapshot::ObjectSnapshot>,
    counters_in_entry: bool,
    counters: &[(CounterType, u32)],
    link: bool,
) -> Result<Option<TypedMoveOriginalReceipt>, crate::effects::ExecutionError> {
    let source = ctx.source;
    let mut original_children = Vec::new();
    if let Some(new_id) = new_id {
        if !counters_in_entry {
            for (counter_type, count) in counters {
                let event = Event::put_counters(new_id, *counter_type, *count, ctx.cause.clone())
                    .with_provenance(ctx.provenance);
                let outcome =
                    crate::effects::counters::execute_object_counter_placement_with_outputs(
                        game, ctx, event,
                    )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                let mut unpublished = outcome.outcome.events.clone();
                crate::effects::retain_unmatched_outcome_events(game, &mut unpublished);
                for event in unpublished {
                    game.queue_trigger_event(event.provenance(), event);
                }
                original_children.push(outcome);
            }
        }
        if link
            && game
                .object(new_id)
                .is_some_and(|object| object.zone == Zone::Exile)
        {
            game.add_exiled_with_source_link(source, new_id);
        }
        game.record_zone_change_results(object, vec![new_id]);
        if let Some(object) = game.object(new_id) {
            let snapshot =
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                    object, game,
                )?;
            ctx.tag_object(crate::tag::ZONE_REPLACEMENT_OBJECT_TAG, snapshot);
        }
        ctx.effect_outcomes.insert(
            crate::effect::EffectId::REPLACED_EVENT,
            crate::effect::EffectOutcome::with_objects(vec![new_id]),
        );
    } else {
        ctx.effect_outcomes.insert(
            crate::effect::EffectId::REPLACED_EVENT,
            crate::effect::EffectOutcome::count(0),
        );
    }
    // The generated movement's added actions run after its move, authored
    // counters/link and original receipts, before the outer replacement's
    // subsequent follow-up instructions. Preserve its primary object summary.
    let target = new_id.unwrap_or(object);
    let target_snapshot = game
        .object(target)
        .map(|object| {
            crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .transpose()?;
    let arrival = new_id
        .and_then(|id| {
            game.object(id)
                .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
        })
        .or(arrival);
    let original_outcome = if let Some(id) = new_id {
        crate::effect::EffectOutcome::with_objects(vec![id])
    } else {
        crate::effect::EffectOutcome::count(0)
    };
    let original_outputs = crate::effects::CompletedEffectOutputs::with_primary_result(
        original_outcome.with_affected_object_memory(arrival.iter().cloned().collect()),
        original_children,
    );
    Ok(Some(TypedMoveOriginalReceipt {
        object,
        target,
        target_snapshot,
        arrival,
        outputs: original_outputs,
    }))
}

impl TypedMoveOriginalReceipt {
    fn complete_tail(
        self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        programs: Vec<PreparedReplacementProgram>,
        follow_ups: &[crate::effect::Effect],
        draws: Option<&mut ZoneDrawContinuations>,
    ) -> Result<TypedMoveOriginalOutputs, crate::effects::ExecutionError> {
        use crate::effects::ExecutionError;
        let Self {
            object,
            target,
            target_snapshot,
            arrival,
            outputs: original_outputs,
        } = self;
        if let Some(draws) = draws {
            let mut bound = Vec::new();
            for program in programs {
                let change = program.context.zone_change_context.as_ref().or_else(|| {
                    crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                        program.context.event.inner(),
                    )
                });
                let matches = change.is_some_and(|change| change.objects.as_slice() == [object])
                    || crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(
                        program.context.event.inner(),
                    )
                    .is_some_and(|entry| entry.object == object);
                if !matches {
                    return Err(ExecutionError::InternalError(
                        "compound addition lost its exact zone receipt".into(),
                    ));
                }
                let snapshots = target_snapshot.clone().into_iter().collect::<Vec<_>>();
                bound.push((
                    program,
                    crate::effects::replacement::ReplacementProgramBindings {
                        targets: Some(vec![crate::effects::ResolvedTarget::Object(target)]),
                        object_tags: vec![
                            ("it".into(), snapshots.clone()),
                            ("__it__".into(), snapshots),
                        ],
                    },
                ));
            }
            let prepared = crate::effects::replacement::prepare_zone_draw_tail_with_outputs(
                game,
                ctx,
                original_outputs,
                bound,
                follow_ups,
            )?;
            draws.0.push(prepared);
            return Ok(TypedMoveOriginalOutputs {
                arrival,
                outputs: crate::effects::CompletedEffectOutputs::aggregate_only(
                    crate::effect::EffectOutcome::count(0),
                ),
            });
        }
        let Some(tail) = complete_typed_move_tail_programs(
            game,
            ctx,
            object,
            target,
            target_snapshot,
            original_outputs,
            programs,
            follow_ups,
        )?
        else {
            return Ok(TypedMoveOriginalOutputs {
                arrival: None,
                outputs: crate::effects::CompletedEffectOutputs::aggregate_only(
                    crate::effect::EffectOutcome::count(0),
                ),
            });
        };
        let outputs = tail.into_outputs();
        Ok(TypedMoveOriginalOutputs { arrival, outputs })
    }
}

/// Actual added-program receipts stay separate from the already-owned original.
/// Full typed movement composes this projection; another semantic owner can
/// consume the additions without concatenating the original history again.
struct TypedMoveTailPrograms {
    original: crate::effects::CompletedEffectOutputs,
    programs: Vec<crate::effects::CompletedEffectOutputs>,
    followups: Vec<crate::effects::CompletedEffectOutputs>,
}

impl TypedMoveTailPrograms {
    fn into_outputs(self) -> crate::effects::CompletedEffectOutputs {
        let mut outputs = self.original.append_replacement_outputs(self.programs);
        for followup in self.followups {
            outputs = outputs.append_batch_completion_outputs(followup);
        }
        outputs
    }
}

/// Execute the native added-program schedule once, with its captured original
/// inputs. Publication stays after generated programs and at each follow-up,
/// while callers own their aggregate projection of the actual program packets.
#[allow(clippy::too_many_arguments)]
fn complete_typed_move_tail_programs(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    object: ObjectId,
    target: ObjectId,
    target_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    original_outputs: crate::effects::CompletedEffectOutputs,
    programs: Vec<PreparedReplacementProgram>,
    follow_ups: &[crate::effect::Effect],
) -> Result<Option<TypedMoveTailPrograms>, crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let completed =
        crate::effects::replacement::complete_deferred_replacement_programs_with_bindings(
            game,
            ctx,
            original_outputs.outcome.clone(),
            programs,
            |_, captured, _| {
                let captured_objects = if let Some(change) =
                    crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                        captured.event.inner(),
                    ) {
                    change.objects.clone()
                } else if let Some(entry) = crate::events::downcast_event::<
                    crate::events::EnterBattlefieldEvent,
                >(captured.event.inner())
                {
                    vec![entry.object]
                } else {
                    return Err(ExecutionError::InternalError(
                        "compound movement addition lost its zone/entry event".into(),
                    ));
                };
                if !captured_objects.contains(&object) {
                    return Err(ExecutionError::InternalError(
                        "compound movement addition does not match its original object".into(),
                    ));
                }
                let snapshots = target_snapshot.clone().into_iter().collect::<Vec<_>>();
                Ok(crate::effects::replacement::ReplacementProgramBindings {
                    targets: Some(vec![crate::effects::ResolvedTarget::Object(target)]),
                    object_tags: vec![
                        ("it".into(), snapshots.clone()),
                        ("__it__".into(), snapshots),
                    ],
                })
            },
        )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let (original, programs) = completed.into_outputs();
    let original = original_outputs.project_aggregate(original);
    // This view serves only the native publication boundary. Actual packets
    // remain owned below; no original or added instruction is executed again.
    let published = crate::effect::EffectOutcome::aggregate_replacement_outcomes(
        original.outcome.clone(),
        programs.iter().map(|outputs| outputs.outcome.clone()),
    );
    let mut unpublished = published.events;
    crate::effects::retain_unmatched_outcome_events(game, &mut unpublished);
    for event in unpublished {
        game.queue_trigger_event(event.provenance(), event);
    }
    let mut followups = Vec::new();
    // Keep the moved-object tag/outcome in this replacement's own scope.
    // The generic child-payload helper deliberately starts a fresh local scope.
    for effect in follow_ups {
        let outcome = crate::effects::execute_effect_with_outputs(game, effect, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let mut unpublished = outcome.outcome.events.clone();
        crate::effects::retain_unmatched_outcome_events(game, &mut unpublished);
        for event in unpublished {
            game.queue_trigger_event(event.provenance(), event);
        }
        followups.push(outcome);
    }
    Ok(Some(TypedMoveTailPrograms {
        original,
        programs,
        followups,
    }))
}

#[allow(clippy::too_many_arguments)]
fn finish_typed_replacement_move(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    object: ObjectId,
    new_id: Option<ObjectId>,
    arrival: Option<crate::snapshot::ObjectSnapshot>,
    counters_in_entry: bool,
    counters: &[(CounterType, u32)],
    link: bool,
    programs: Vec<PreparedReplacementProgram>,
    follow_ups: &[crate::effect::Effect],
    published_outputs: Vec<crate::effects::PublishedEffectOutputs>,
    draws: Option<&mut ZoneDrawContinuations>,
) -> Result<TypedMoveOriginalOutputs, crate::effects::ExecutionError> {
    let Some(mut original) = assemble_typed_move_original(
        game,
        ctx,
        object,
        new_id,
        arrival,
        counters_in_entry,
        counters,
        link,
    )?
    else {
        return Ok(TypedMoveOriginalOutputs {
            arrival: None,
            outputs: crate::effects::CompletedEffectOutputs::aggregate_only(
                crate::effect::EffectOutcome::count(0),
            ),
        });
    };
    original
        .outputs
        .retain_published_references(published_outputs);
    original.complete_tail(game, ctx, programs, follow_ups, draws)
}

/// Whether a `Replaced` replacement action is a "put it into [zone] instead"
/// move (with optional counters, source link and follow-up effects), rather
/// than an arbitrary replacement effect sequence.
fn replacement_moves_object(replacement: &crate::replacement::ReplacementAction) -> bool {
    matches!(
        replacement,
        crate::replacement::ReplacementAction::MoveToZoneWithCounters { .. }
            | crate::replacement::ReplacementAction::ExileWithSourceLink
            | crate::replacement::ReplacementAction::ExileWithSourceLinkThen(_)
            | crate::replacement::ReplacementAction::ExileWithSourceLinkCountersThen { .. }
    )
}

/// A resolved draw proposal, or the execution receipt for its replacement.
/// Proceed retains the complete event: callers must not reconstruct its player,
/// flags, amount or provenance from the original request. No draw is committed
/// by this adapter; a Replaced payload has executed and its observations are
/// returned for the owning caller to publish.
#[derive(Debug, Clone)]
#[must_use = "the draw commit owner must retain deferred replacement programs"]
pub enum ResolvedDrawOutcome {
    /// The original proposal and instructions added during its replacement
    /// sequence. The instructions run after the original draw is committed,
    /// or after its prevention/Instead outcome, under the draw owner's checkpoint.
    Expanded {
        original: Box<ResolvedDrawOutcome>,
        programs: Vec<PreparedReplacementProgram>,
    },
    Proceed(Event),
    Prevented,
    Replaced {
        context: Box<ReplacementEventContext>,
        source: ObjectId,
        controller: PlayerId,
        payload: crate::effect::EffectOutcome,
    },
    Pending,
}

impl ResolvedDrawOutcome {
    /// Retain every deferred program while extracting the original proposal.
    /// This adapter never commits an original draw, so it cannot execute its
    /// added instructions during matching/preparation.
    pub fn into_expansion(self) -> (Self, Vec<PreparedReplacementProgram>) {
        let mut original = self;
        let mut programs = Vec::new();
        while let Self::Expanded {
            original: next,
            programs: added,
        } = original
        {
            programs.extend(added);
            original = *next;
        }
        (original, programs)
    }
}

/// Resolve one draw proposal and execute any Instead program atomically.
/// This is a root adapter; effect execution with a parent scope uses the scoped
/// event processor and replacement-payload executor directly.
pub fn process_draw(
    game: &mut GameState,
    player: PlayerId,
    count: u32,
    is_first_this_turn: bool,
    dm: &mut dyn DecisionMaker,
) -> Result<ResolvedDrawOutcome, crate::effects::ExecutionError> {
    use crate::effects::{ExecutionContext, ExecutionError};
    if !dm.awaiting_choice() {
        game.clear_pending_decision_controllers();
    }
    let checkpoint = game.clone();
    let mut programs = Vec::new();
    let result = (|| {
        if game.player(player).is_none() {
            return Err(ExecutionError::PlayerNotFound(player));
        }
        game.update_replacement_effects()
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
        if !game.can_draw(player) {
            return Ok(ResolvedDrawOutcome::Prevented);
        }
        let event = Event::draw(player, count, is_first_this_turn);
        let (resolved, added) = process_with_dm(game, event, dm)?.into_expansion();
        programs.extend(added);
        if dm.awaiting_choice() {
            return Ok(ResolvedDrawOutcome::Pending);
        }
        match resolved {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                if crate::events::downcast_event::<crate::events::DrawEvent>(event.inner())
                    .is_none()
                {
                    return Err(ExecutionError::InternalError(
                        "draw replacement returned an incompatible event".into(),
                    ));
                }
                Ok(ResolvedDrawOutcome::Proceed(event))
            }
            TraitEventResult::Prevented => Ok(ResolvedDrawOutcome::Prevented),
            TraitEventResult::Replaced {
                context,
                effects,
                source,
                controller,
                ..
            } => {
                let mut parent = ExecutionContext::new(source, controller, dm);
                let payload = crate::effects::replacement::execute_replacement_payload(
                    game,
                    &mut parent,
                    &effects,
                    source,
                    controller,
                    &context,
                    None,
                )?;
                if parent.decision_maker.awaiting_choice() {
                    return Ok(ResolvedDrawOutcome::Pending);
                }
                Ok(ResolvedDrawOutcome::Replaced {
                    context,
                    source,
                    controller,
                    payload,
                })
            }
            TraitEventResult::Expanded { .. } => Err(ExecutionError::InternalError(
                "draw adapter received an unflattened result".into(),
            )),
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                Err(ExecutionError::InternalError(
                    "draw replacement returned an unresolved choice without pending input".into(),
                ))
            }
        }
    })();
    let pending = matches!(&result, Ok(ResolvedDrawOutcome::Pending));
    if result.is_err() || pending {
        game.restore_execution_checkpoint(checkpoint, pending);
    }
    result.map(|original| {
        // A pending proposal is rolled back and replayed, including matching
        // and one-shot consumption. Returning its programs would expose an
        // uncommitted prefix and could execute it twice on resume.
        if programs.is_empty() || matches!(&original, ResolvedDrawOutcome::Pending) {
            original
        } else {
            ResolvedDrawOutcome::Expanded {
                original: Box::new(original),
                programs,
            }
        }
    })
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

/// A loss proposal retained until the owner finishes the original operation.
#[derive(Debug)]
#[must_use]
pub(crate) struct PlayerLossReceipt {
    pub player: PlayerId,
    pub original: PlayerLossOutcome,
    programs: Vec<PreparedReplacementProgram>,
    pending_original: Option<crate::effects::replacement::PreparedReplacementOriginal>,
    payload_outcome: Option<crate::effects::CompletedEffectOutputs>,
}

impl PlayerLossReceipt {
    pub(crate) fn has_deferred_programs(&self) -> bool {
        !self.programs.is_empty()
    }
}

/// Public loss operation: pending returns no committed verdict, and errors
/// restore the state from before replacement selection.
pub fn process_player_loss(
    game: &mut GameState,
    player: PlayerId,
    dm: &mut dyn DecisionMaker,
) -> Result<Option<PlayerLossOutcome>, crate::effects::ExecutionError> {
    process_player_loss_with_simultaneous_zone_changes(
        game,
        player,
        dm,
        &std::collections::HashMap::new(),
    )
}

pub(crate) fn process_player_loss_with_simultaneous_zone_changes(
    game: &mut GameState,
    player: PlayerId,
    dm: &mut dyn DecisionMaker,
    simultaneous_zone_changes: &std::collections::HashMap<ObjectId, Zone>,
) -> Result<Option<PlayerLossOutcome>, crate::effects::ExecutionError> {
    let controller = game.turn.active_player;
    let mut ctx = crate::effects::ExecutionContext::new(ObjectId(0), controller, dm)
        .with_cause(crate::events::cause::EventCause::from_sba());
    let result =
        process_player_loss_with_context(game, player, &mut ctx, simultaneous_zone_changes)?;
    let Some((verdict, mut outcome)) = result else {
        return Ok(None);
    };
    crate::effects::retain_unmatched_outcome_events(game, &mut outcome.events);
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
    Ok(Some(verdict))
}

pub(crate) fn process_player_loss_with_context(
    game: &mut GameState,
    player: PlayerId,
    ctx: &mut crate::effects::ExecutionContext,
    simultaneous_zone_changes: &std::collections::HashMap<ObjectId, Zone>,
) -> Result<Option<(PlayerLossOutcome, crate::effect::EffectOutcome)>, crate::effects::ExecutionError>
{
    process_player_loss_with_context_and_outputs(game, player, ctx, simultaneous_zone_changes)
        .map(|receipt| receipt.map(|(verdict, outputs)| (verdict, outputs.into_outcome())))
}

pub(crate) fn process_player_loss_with_context_and_outputs(
    game: &mut GameState,
    player: PlayerId,
    ctx: &mut crate::effects::ExecutionContext,
    simultaneous_zone_changes: &std::collections::HashMap<ObjectId, Zone>,
) -> Result<
    Option<(PlayerLossOutcome, crate::effects::CompletedEffectOutputs)>,
    crate::effects::ExecutionError,
> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        let Some(receipt) =
            prepare_player_loss_scoped(game, player, ctx, simultaneous_zone_changes)?
        else {
            return Ok(None);
        };
        let Some((verdict, committed)) =
            commit_prepared_player_loss_original_with_outputs(game, ctx, receipt)?
        else {
            return Ok(None);
        };
        let outcome = crate::effects::composition::complete_standalone_original_with_outputs(
            game, ctx, committed,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        Ok(Some((verdict, outcome)))
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        *game = checkpoint;
        context_checkpoint.restore(ctx);
    }
    result
}

/// Evaluate before the simultaneous SBA batch, retaining additions until
/// every original action and loss/departure sweep has finished.
pub(crate) fn process_player_loss_replacements_before_commit(
    game: &mut GameState,
    player: PlayerId,
    dm: &mut dyn DecisionMaker,
    simultaneous_zone_changes: &std::collections::HashMap<ObjectId, Zone>,
) -> Result<Option<PlayerLossReceipt>, crate::effects::ExecutionError> {
    let controller = game.turn.active_player;
    let mut ctx = crate::effects::ExecutionContext::new(ObjectId(0), controller, dm)
        .with_cause(crate::events::cause::EventCause::from_sba());
    prepare_player_loss_scoped(game, player, &mut ctx, simultaneous_zone_changes)
}

pub(crate) fn prepare_player_loss_scoped(
    game: &mut GameState,
    player: PlayerId,
    ctx: &mut crate::effects::ExecutionContext,
    simultaneous_zone_changes: &std::collections::HashMap<ObjectId, Zone>,
) -> Result<Option<PlayerLossReceipt>, crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        let mut receipt = PlayerLossReceipt {
            player,
            original: PlayerLossOutcome::Prevented,
            programs: Vec::new(),
            pending_original: None,
            payload_outcome: None,
        };
        if !game.can_lose_game(player)
            || game
                .player(player)
                .is_none_or(|player| !player.is_in_game())
        {
            return Ok(Some(receipt));
        }
        let event = game.ensure_event_provenance(Event::new_with_provenance(
            crate::events::PlayerLosesGameEvent::new(player),
            ctx.provenance,
        ));
        let result = process_trait_event_with_execution_context(game, event, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let (original, programs) = result.into_expansion();
        receipt.programs = programs;
        match original {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                let loss = crate::events::downcast_event::<crate::events::PlayerLosesGameEvent>(
                    event.inner(),
                )
                .ok_or_else(|| {
                    ExecutionError::InternalError(
                        "loss replacement returned an incompatible event".into(),
                    )
                })?;
                receipt.player = loss.player;
                receipt.original = PlayerLossOutcome::Lost;
            }
            TraitEventResult::Prevented => {}
            TraitEventResult::Replaced {
                effects,
                effect_id,
                source,
                controller,
                context,
                ..
            } => {
                let source_snapshot = game.object(source).map(|object| crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)).transpose()?;
                let inherited = ctx.replacement.clone();
                if let Some(sba_destination) = simultaneous_zone_changes.get(&source).copied()
                    && let Some(replacement_destination) =
                        loss_replacement_source_destination(game, player, source, &effects)
                    && replacement_destination != sba_destination
                {
                    let chosen = choose_mutually_exclusive_source_destination(
                        game,
                        source,
                        replacement_destination,
                        sba_destination,
                        ctx.decision_maker,
                    );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                    if chosen != replacement_destination {
                        ctx.replacement
                            .simultaneous_zone_destinations
                            .insert(source, chosen);
                    }
                }
                game.effect_store
                    .replacement_effects
                    .mark_effect_used(effect_id);
                receipt.pending_original =
                    Some(crate::effects::replacement::PreparedReplacementOriginal {
                        program: PreparedReplacementProgram {
                            context,
                            source,
                            controller,
                            source_snapshot,
                            effects,
                        },
                        scope: ctx.replacement.clone(),
                        original: crate::effect::EffectOutcome::resolved(),
                        bindings: crate::effects::replacement::ReplacementProgramBindings {
                            targets: None,
                            object_tags: Vec::new(),
                        },
                    });
                ctx.replacement = inherited;
                receipt.original = PlayerLossOutcome::Replaced;
            }
            TraitEventResult::NeedsChoice { .. } => {
                return Err(ExecutionError::InternalError(
                    "loss replacement choice did not resolve".into(),
                ));
            }
            TraitEventResult::NeedsInteraction { .. } => {
                return Err(ExecutionError::InternalError(
                    "unsupported interactive loss replacement".into(),
                ));
            }
            TraitEventResult::Expanded { .. } => {
                return Err(ExecutionError::InternalError(
                    "nested loss expansion was not flattened".into(),
                ));
            }
        }
        Ok(Some(receipt))
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        *game = checkpoint;
        context_checkpoint.restore(ctx);
    }
    result
}

/// Commit selected replacement originals before the native loss/departure sweep.
/// All sibling loss proposals have been selected before this owner runs.
pub(crate) fn commit_player_loss_replacement_originals(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    receipts: &mut [PlayerLossReceipt],
) -> Result<(), crate::effects::ExecutionError> {
    for receipt in receipts {
        if let Some(original) = receipt.pending_original.take() {
            receipt.payload_outcome = Some(original.commit_with_outputs(game, ctx)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
    }
    Ok(())
}

/// The original loss owner is shared by ordinary and prepared effect adapters.
/// Appended programmes remain with completion, after the surrounding originals.
pub(crate) fn commit_prepared_player_loss_original_with_outputs(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    mut receipt: PlayerLossReceipt,
) -> Result<
    Option<(
        PlayerLossOutcome,
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    )>,
    crate::effects::ExecutionError,
> {
    commit_player_loss_replacement_originals(game, ctx, std::slice::from_mut(&mut receipt))?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    commit_player_loss_receipt(game, &mut receipt)?;
    let verdict = receipt.original;
    let original = if verdict == PlayerLossOutcome::Lost {
        crate::effect::EffectOutcome::resolved()
    } else {
        crate::effect::EffectOutcome::prevented()
    };
    let primary = original.value.clone();
    let base = receipt.payload_outcome.take().unwrap_or_else(|| {
        crate::effects::CompletedEffectOutputs::aggregate_only(
            crate::effect::EffectOutcome::resolved(),
        )
    });
    let outputs = crate::effects::CompletedEffectOutputs::from_children([base], |children| {
        let mut outcome =
            crate::effect::EffectOutcome::aggregate(std::iter::once(original).chain(children));
        outcome.value = primary;
        outcome
    });
    let completion = if receipt.programs.is_empty() {
        None
    } else {
        Some(Box::new(PlayerLossCompletion { receipt })
            as Box<dyn crate::effects::SimultaneousEffectCompletion>)
    };
    Ok(Some((
        verdict,
        crate::effects::SimultaneousEffectCommit {
            outcome: outputs,
            completion,
        },
    )))
}

struct PlayerLossCompletion {
    receipt: PlayerLossReceipt,
}
impl crate::effects::SimultaneousEffectCompletion for PlayerLossCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        if self.receipt.pending_original.is_some() {
            crate::effects::OriginalPhaseStatus::Combined
        } else {
            crate::effects::OriginalPhaseStatus::Complete
        }
    }

    fn freeze(&mut self, _game: &mut GameState) -> Result<(), crate::effects::ExecutionError> {
        Ok(())
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
        self.complete_from_original_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
        finish_player_loss_receipts_with_outputs(game, ctx, original, vec![self.receipt])
    }
}

pub(crate) fn commit_player_loss_receipt(
    game: &mut GameState,
    receipt: &mut PlayerLossReceipt,
) -> Result<(), crate::effects::ExecutionError> {
    commit_player_loss_receipts(game, std::slice::from_mut(receipt))
}
pub(crate) fn commit_player_loss_receipts(
    game: &mut GameState,
    receipts: &mut [PlayerLossReceipt],
) -> Result<(), crate::effects::ExecutionError> {
    let players = receipts
        .iter()
        .filter(|receipt| receipt.original == PlayerLossOutcome::Lost)
        .map(|receipt| receipt.player)
        .collect::<Vec<_>>();
    let lost = game.mark_players_lost_simultaneously(&players)?;
    for receipt in receipts {
        if receipt.original == PlayerLossOutcome::Lost && !lost.contains(&receipt.player) {
            receipt.original = PlayerLossOutcome::Prevented;
        }
    }
    Ok(())
}

pub(crate) fn finish_player_loss_receipts(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    original: crate::effect::EffectOutcome,
    receipts: Vec<PlayerLossReceipt>,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    finish_player_loss_receipts_with_outputs(game, ctx, original, receipts)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn finish_player_loss_receipts_with_outputs<O: crate::effects::OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    original: O,
    receipts: Vec<PlayerLossReceipt>,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    let original = original.into_outputs();
    let primary = original.outcome.value.clone();
    let mut outcomes = Vec::new();
    for receipt in receipts {
        if receipt.pending_original.is_some() {
            return Err(crate::effects::ExecutionError::InternalError(
                "loss completion received an uncommitted replacement original".into(),
            ));
        }
        let base = receipt.payload_outcome.unwrap_or_else(|| {
            crate::effects::CompletedEffectOutputs::aggregate_only(
                crate::effect::EffectOutcome::resolved(),
            )
        });
        let outputs =
            crate::effects::replacement::complete_replacement_programs_with_original_outputs(
                game,
                ctx,
                base,
                |game, ctx, original| {
                    crate::effects::replacement::complete_deferred_replacement_programs(
                        game,
                        ctx,
                        original,
                        receipt.programs,
                    )
                },
            )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                crate::effect::EffectOutcome::count(0),
            ));
        }
        outcomes.push(outputs);
    }
    Ok(crate::effects::CompletedEffectOutputs::from_children(
        std::iter::once(original).chain(outcomes),
        |children| {
            let mut outcome = crate::effect::EffectOutcome::aggregate(children);
            outcome.value = primary;
            outcome
        },
    ))
}

/// Process an event through replacement effects, using a DecisionMaker to resolve choices.
///
/// When `NeedsChoice` is returned (multiple effects at same priority), this function
/// uses the decision maker to ask the player which replacement effect to apply.
/// If no decision maker is provided, the first applicable effect is chosen automatically.
///
/// Takes a mutable reference to the Option so the decision maker can be reused by the caller
/// for subsequent processing (e.g., zone change after destruction).
/// Resolve a proposal using the enclosing execution's event-local effects
/// and complete suppression history, including stable keys after refresh.
pub(crate) fn process_trait_event_with_execution_context(
    game: &mut GameState,
    event: Event,
    ctx: &mut crate::effects::ExecutionContext,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    let operation_checkpoint = game.clone();
    let operation_result = (|| -> Result<TraitEventResult, crate::effects::ExecutionError> {
        let checkpoint = game.clone();
        game.try_update_static_ability_effects(Default::default())
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
        game.update_replacement_effects()
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
        if let Some(mana) =
            crate::events::downcast_event::<crate::events::ManaAddedEvent>(event.inner())
        {
            if let Some(witness) = ctx
                .decision_maker
                .take_mana_replacement_witness(game, mana)
                .map_err(crate::effects::ExecutionError::InternalError)?
            {
                // Event-local/self replacements carry a separate application scope.
                // They must be represented before this replay path can own them.
                if !ctx.additional_replacement_effects_snapshot().is_empty()
                    || !ctx.replacement.suppressed_replacement_effects.is_empty()
                    || !ctx
                        .replacement
                        .suppressed_replacement_effect_keys
                        .is_empty()
                {
                    return Err(crate::effects::ExecutionError::InternalError(
                        "mana witness cannot bypass event-local replacement scope".into(),
                    ));
                }
                let resolved = crate::mana_payment::replay_mana_replacements(game, mana, &witness)?;
                let event = game.ensure_event_provenance(event);
                return Ok(TraitEventResult::Modified(event.rewrap(resolved)));
            }
        }
        let mut additional = ctx.additional_replacement_effects_snapshot();
        assign_ephemeral_effect_ids(&mut additional, u64::MAX / 2);
        let result = process_with_dm_and_additional_effects_and_applied(
            game,
            event,
            &mut *ctx.decision_maker,
            &additional,
            &ctx.replacement.suppressed_replacement_effects,
            &ctx.replacement.suppressed_replacement_effect_keys,
            ctx.source_snapshot.as_ref(),
        )?;
        let mut original = &result;
        while let TraitEventResult::Expanded {
            original: retained, ..
        } = original
        {
            original = retained;
        }
        // A synchronous required choice cannot remain unresolved after a completed
        // response. Preserve actual pending continuations, but reject malformed
        // answers before any owning consumer can mistake them for success.
        if matches!(original, TraitEventResult::NeedsChoice { .. })
            && !ctx.decision_maker.awaiting_choice()
        {
            *game = checkpoint;
            return Err(crate::effects::ExecutionError::InternalError(
                "replacement choice must name exactly one offered effect".into(),
            ));
        }
        Ok(result)
    })();
    if operation_result.is_err() {
        game.restore_execution_checkpoint(operation_checkpoint, false);
    }
    operation_result
}

fn process_with_dm(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    process_with_dm_and_additional_effects_and_snapshot(game, event, dm, &[], None)
}

fn process_with_dm_and_additional_effects(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    process_with_dm_and_additional_effects_and_snapshot(game, event, dm, additional_effects, None)
}

fn process_with_dm_and_additional_effects_and_snapshot(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
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
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    process_with_dm_and_additional_effects_and_applied_state(
        game,
        event,
        dm,
        additional_effects,
        applied_effects,
        applied_effect_keys,
        event_source_snapshot,
        &mut TraitEventProcessingState::default(),
    )
}

#[allow(clippy::too_many_arguments)]
fn process_with_dm_and_additional_effects_and_applied_state(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
    applied_effects: &std::collections::HashSet<ReplacementEffectId>,
    applied_effect_keys: &std::collections::HashSet<ReplacementEffectKey>,
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    state: &mut TraitEventProcessingState,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    let checkpoint = game.clone();
    let state_checkpoint = state.clone();
    let result = process_with_dm_and_additional_effects_and_applied_state_inner(
        game,
        event,
        dm,
        additional_effects,
        applied_effects,
        applied_effect_keys,
        event_source_snapshot,
        state,
    );
    if result.is_err() {
        game.restore_execution_checkpoint(checkpoint, false);
        *state = state_checkpoint;
    }
    result.map(|result| retain_additional_programs(result, state))
}

fn choose_mana_rewrite_color(
    game: &GameState,
    dm: &mut (impl DecisionMaker + ?Sized),
    effect: &ReplacementEffect,
    event: &Event,
) -> Result<Option<crate::mana::ManaSymbol>, crate::effects::ExecutionError> {
    let ReplacementAction::RewriteMana { output, .. } = &effect.replacement else {
        return Err(crate::effects::ExecutionError::InternalError(
            "non-mana replacement requested a color".into(),
        ));
    };
    let mana = crate::events::downcast_event::<crate::events::ManaAddedEvent>(event.inner())
        .ok_or_else(|| {
            crate::effects::ExecutionError::InternalError(
                "mana replacement received a non-production event".into(),
            )
        })?;
    let chooser = if matches!(
        output,
        ironsmith_core::ManaRewriteOutput::ByBasicLandType(_)
    ) {
        mana.controller
    } else {
        effect.controller
    };
    let available = crate::events::mana::mana_rewrite_output_choices(*output, mana, game);
    if let [symbol] = available.as_slice() {
        return Ok(Some(*symbol));
    }
    if available.is_empty() {
        return Err(crate::effects::ExecutionError::InternalError(
            "mana replacement has no output color".into(),
        ));
    }
    let choice = crate::mana_payment::ManaProductionChoice {
        purpose: crate::mana_payment::ManaChoicePurpose::ReplacementColor,
        source: effect.source,
        player: chooser,
        available: available.clone(),
        count: 1,
        same_type: true,
        distinct: false,
    };
    if let Some(output) = dm
        .planned_mana_output(game, &choice)
        .map_err(crate::effects::ExecutionError::InternalError)?
    {
        if !choice.accepts(&output) {
            return Err(crate::effects::ExecutionError::InternalError(
                "invalid prepared mana replacement color".into(),
            ));
        }
        return Ok(Some(output[0]));
    }
    let colors = crate::color::Color::ALL
        .into_iter()
        .filter(|color| available.contains(&crate::mana::ManaSymbol::from_color(*color)))
        .collect();
    let mut context = crate::decisions::context::ColorsContext::restricted(
        chooser,
        Some(effect.source),
        1,
        true,
        false,
        colors,
    );
    context.description = "Choose the replacement mana color".into();
    let colors = dm.decide_colors(game, &context);
    if dm.awaiting_choice() {
        return Ok(None);
    }
    let [color] = colors.as_slice() else {
        return Err(crate::effects::ExecutionError::InternalError(
            "mana replacement requires one color".into(),
        ));
    };
    let symbol = crate::mana::ManaSymbol::from_color(*color);
    if !available.contains(&symbol) {
        return Err(crate::effects::ExecutionError::InternalError(
            "unavailable mana replacement color".into(),
        ));
    }
    Ok(Some(symbol))
}

#[allow(clippy::too_many_arguments)]
fn process_with_dm_and_additional_effects_and_applied_state_inner(
    game: &mut GameState,
    event: Event,
    dm: &mut (impl DecisionMaker + ?Sized),
    additional_effects: &[ReplacementEffect],
    applied_effects: &std::collections::HashSet<ReplacementEffectId>,
    applied_effect_keys: &std::collections::HashSet<ReplacementEffectKey>,
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    state: &mut TraitEventProcessingState,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    use crate::decisions::{
        make_decision,
        specs::{ReplacementOption, ReplacementSpec},
    };

    let mut current_event = game.ensure_event_provenance(event);
    state
        .applied_effects
        .extend(applied_effects.iter().copied());
    state
        .applied_effect_keys
        .extend(applied_effect_keys.iter().cloned());

    loop {
        let result = process_event_direct_inner(
            game,
            current_event.clone(),
            state,
            additional_effects,
            event_source_snapshot,
        )?;

        match result {
            TraitEventResult::NeedsChoice {
                player,
                applicable_effects,
                event: boxed_event,
                ..
            } => {
                // Determine which effect to apply
                let chosen_index = if applicable_effects.len() == 1
                    && find_effect_for_choice(game, additional_effects, applicable_effects[0])
                        .is_some_and(|effect| effect.replacement.needs_mana_color_choice())
                {
                    vec![0]
                } else {
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
                    return Ok(TraitEventResult::NeedsChoice {
                        player,
                        applicable_effects,
                        event: boxed_event,
                        applied_effects: state.applied_effects.clone(),
                        applied_effect_keys: state.applied_effect_keys.clone(),
                        zone_change_context: state.zone_change_context.clone(),
                    });
                }

                // A required replacement decision must name exactly one
                // currently offered effect. Invalid input retains the proposal
                // and history; the operation owner reports the unresolved choice.
                let effect_id = match chosen_index.as_slice() {
                    [index] => applicable_effects.get(*index).copied(),
                    _ => None,
                };
                let Some(effect_id) = effect_id else {
                    return Ok(TraitEventResult::NeedsChoice {
                        player,
                        applicable_effects,
                        event: boxed_event,
                        applied_effects: state.applied_effects.clone(),
                        applied_effect_keys: state.applied_effect_keys.clone(),
                        zone_change_context: state.zone_change_context.clone(),
                    });
                };

                let Some(mut chosen_effect) =
                    find_effect_for_choice(game, additional_effects, effect_id)
                else {
                    // Effect disappeared (e.g., source left battlefield). Continue with event.
                    state.mark_applied(effect_id);
                    current_event = *boxed_event;
                    continue;
                };

                let original_effect = chosen_effect.clone();
                if chosen_effect.replacement.needs_mana_color_choice() {
                    let Some(color) =
                        choose_mana_rewrite_color(game, dm, &chosen_effect, &boxed_event)?
                    else {
                        return Ok(TraitEventResult::NeedsChoice {
                            player,
                            applicable_effects,
                            event: boxed_event,
                            applied_effects: state.applied_effects.clone(),
                            applied_effect_keys: state.applied_effect_keys.clone(),
                            zone_change_context: state.zone_change_context.clone(),
                        });
                    };
                    if let ReplacementAction::RewriteMana { output, .. } =
                        &mut chosen_effect.replacement
                    {
                        *output = ironsmith_core::ManaRewriteOutput::Symbol(color);
                    }
                }
                // Choosing an output is not a distinct replacement occurrence.
                // Keep the original identity even for structural ephemeral keys.
                mark_applied_replacement_choice(state, &original_effect);

                let apply_result = apply_trait_replacement_retaining_damage_branches(
                    game,
                    (*boxed_event).clone(),
                    &chosen_effect,
                    state,
                )?;
                consume_one_shot_if_applied(game, effect_id, &apply_result);
                match apply_result {
                    TraitApplyResult::Modified(modified_event) => {
                        current_event = modified_event;
                    }
                    TraitApplyResult::Prevented => return Ok(TraitEventResult::Prevented),
                    TraitApplyResult::Replaced(effects) => {
                        return Ok(TraitEventResult::Replaced {
                            context: Box::new(ReplacementEventContext::new(
                                &game,
                                *boxed_event,
                                &state,
                            )),
                            effects,
                            effect_id,
                            replacement: chosen_effect.replacement.clone(),
                            source: chosen_effect.source,
                            controller: chosen_effect.controller,
                        });
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
                        return Ok(TraitEventResult::NeedsInteraction {
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
                            applied_effects: state.applied_effects.clone(),
                            applied_effect_keys: state.applied_effect_keys.clone(),
                            zone_change_context: state.zone_change_context.clone(),
                        });
                    }
                }
            }
            other => return Ok(other),
        }
    }
}

fn apply_trait_replacement_retaining_damage_branches(
    game: &mut GameState,
    event: Event,
    effect: &ReplacementEffect,
    state: &mut TraitEventProcessingState,
) -> Result<TraitApplyResult, crate::effects::ExecutionError> {
    // Selected-effect and captured-choice APIs reach this boundary before the
    // rescan loop. An absent operation cannot apply or consume a replacement.
    if quantitative_event_has_been_removed(&event) {
        return Ok(TraitApplyResult::Unchanged(event));
    }
    if state.zone_change_context.is_none() {
        state.zone_change_context =
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner()).cloned();
    }
    if let ReplacementAction::Additionally(effects) = &effect.replacement
        && !effects.is_empty()
    {
        let mut captured_history = state.clone();
        mark_applied_replacement_choice(&mut captured_history, effect);
        state.additional_programs.push(PreparedReplacementProgram {
            context: Box::new(ReplacementEventContext::new(
                &game,
                event.clone(),
                &captured_history,
            )),
            source: effect.source,
            controller: effect.controller,
            source_snapshot: game
                .object(effect.source)
                .filter(|_| !game.is_phased_out(effect.source))
                .map(|source| crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(source, game)).transpose()?
                .or_else(|| {
                    game.source_last_known_snapshot(effect.source)
                        .cloned()
                }),
            effects: effects.clone(),
        });
    }
    let before = crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
        .and_then(|damage| damage.remainder);
    let was_entry = event.kind() == crate::events::EventKind::EnterBattlefield;
    let result = apply_trait_replacement(game, event, effect)?;
    // Converting an entry back to a zone carrier changes its destination,
    // not the original operation's cause or pre-event snapshot context.
    let result = match result {
        TraitApplyResult::Modified(modified) if was_entry => {
            if let (Some(original), Some(changed)) = (
                state.zone_change_context.as_ref(),
                crate::events::downcast_event::<crate::events::ZoneChangeEvent>(modified.inner()),
            ) {
                let mut retained = original.clone();
                retained.objects = changed.objects.clone();
                retained.from = changed.from;
                retained.to = changed.to;
                TraitApplyResult::Modified(modified.rewrap(retained))
            } else {
                TraitApplyResult::Modified(modified)
            }
        }
        other => other,
    };
    if let TraitApplyResult::Modified(modified) = &result
        && let Some(damage) =
            crate::events::downcast_event::<crate::events::DamageEvent>(modified.inner())
        && let Some((target, amount)) = damage.remainder
        && amount > 0
        && before != damage.remainder
    {
        let mut history = TraitEventProcessingState {
            applied_effects: state.applied_effects.clone(),
            applied_effect_keys: state.applied_effect_keys.clone(),
            ..Default::default()
        };
        mark_applied_replacement_choice(&mut history, effect);
        let mut remainder = damage.with_target(target).with_amount(amount);
        remainder.remainder = None;
        state.damage_remainders.push(PendingDamageRemainder {
            event: remainder,
            applied_effects: history.applied_effects,
            applied_effect_keys: history.applied_effect_keys,
        });
    }
    Ok(result)
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

/// Entry programs may change the entrant's text or characteristics. Re-read
/// its abilities in the pending battlefield state, retaining a stable local
/// effect ID for each static ability across replacement-loop iterations.
fn prepared_object_etb_replacement_effects(
    game: &GameState,
    event: &Event,
    ids: &mut std::collections::HashMap<
        (crate::replacement::ReplacementAbilityOrigin, usize),
        ReplacementEffectId,
    >,
    state: &TraitEventProcessingState,
    reserved_objects: &std::collections::HashSet<ObjectId>,
) -> Result<Option<Vec<ReplacementEffect>>, crate::effects::ExecutionError> {
    let Some(etb) =
        crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
    else {
        return Ok(None);
    };
    // Entry replacements can be granted by another permanent. Inspect the
    // prospective characteristics, not only the entering card's printed text.
    let Some(prospective) = etb
        .try_prospective_game_state(game)
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
    else {
        return Ok(None);
    };
    let chars = prospective
        .calculated_characteristics(etb.object)
        .ok_or(crate::effects::ExecutionError::ObjectNotFound(etb.object))?;
    // CR 614.12 also includes already-existing ability grants. They need
    // the prospective battlefield view even when no as-enters program ran.
    let controller = prospective
        .current_controller(etb.object)
        .ok_or(crate::effects::ExecutionError::ObjectNotFound(etb.object))?;
    let mut effects = Vec::new();
    for (index, ability) in chars.abilities.iter().enumerate() {
        let crate::ability::AbilityKind::Static(ability) = &ability.kind else {
            continue;
        };
        let origin = chars
            .abilities
            .origin(index)
            .expect("calculated ability and origin remain paired")
            .clone();
        let face = matches!(&origin, crate::continuous::AbilityOrigin::Printed(_))
            .then(|| {
                prospective
                    .object(etb.object)
                    .and_then(|object| object.card)
            })
            .flatten();
        let parent = crate::replacement::ReplacementAbilityOrigin {
            ability: origin.clone(),
            printed_face: face,
            branch: 0,
        };
        if let Some(model) = ability.compiled_model()
            && let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
                program,
                turns_face_up_only: false,
                transforms_into: None,
                ..
            } = &model.payload
        {
            let mut effect = ReplacementEffect::with_matcher(
                etb.object,
                controller,
                crate::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(program.clone()),
            );
            let next_id = ReplacementEffectId(u64::MAX - 250_000 + ids.len() as u64);
            effect.id = *ids.entry((parent.clone(), usize::MAX)).or_insert(next_id);
            effect.static_ability_instance = Some(ability.instance_id());
            effects.push(effect.with_ability_origin(origin.clone(), face, usize::MAX));
        }
        if let Some(mut effect) = ability.generate_replacement_effect(etb.object, controller)
            && effect
                .matcher
                .as_ref()
                .is_some_and(|matcher| matcher.applies_from_entering_source())
        {
            let next_id = ReplacementEffectId(u64::MAX - 250_000 + ids.len() as u64);
            effect.id = *ids.entry((parent.clone(), 0)).or_insert(next_id);
            effects.push(effect.with_ability_origin(origin.clone(), face, 0));
        }
        if let Some(spec) = ability.enter_as_copy_as_enters()
            && spec.affected_filter.is_none()
        {
            // All candidates (including declining) are alternative outcomes
            // of one replacement, not independently applicable replacements.
            let consumed = ids.iter().any(|((existing, slot), id)| {
                existing == &parent && *slot > 0 && *slot != usize::MAX && state.was_applied(*id)
            });
            if consumed {
                continue;
            }
            let mut copies = Vec::new();
            push_enter_as_copy_effects_for_spec(
                &prospective,
                etb.object,
                etb.object,
                controller,
                spec,
                reserved_objects,
                &mut copies,
                &origin,
                ability,
            )?;
            for (index, mut effect) in copies.into_iter().enumerate() {
                let next_id = ReplacementEffectId(u64::MAX - 250_000 + ids.len() as u64);
                effect.id = *ids.entry((parent.clone(), index + 1)).or_insert(next_id);
                effect.static_ability_instance = Some(ability.instance_id());
                // Every candidate, including declining, belongs to this one
                // copy replacement; independently originating abilities differ.
                effects.push(effect.with_ability_origin(origin.clone(), face, 1));
            }
        }
    }
    Ok(Some(effects))
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
            && effect
                .matcher
                .as_ref()
                .is_some_and(|matcher| matcher.applies_from_entering_source())
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
    /// Instructions added during preparation; the original commit owner must
    /// execute these after committing its full original entry operation.
    pub additional_programs: Vec<PreparedReplacementProgram>,
    /// Actual completed entry programmes, independent of the final verdict.
    pub(crate) completed_program_outputs: Vec<crate::effects::PublishedEffectOutputs>,
    /// Whether the permanent enters tapped
    pub enters_tapped: bool,
    /// Counters the permanent enters with (counter_type, count)
    pub enters_with_counters: Vec<(CounterType, u32)>,
    /// Objects exiled and linked to the entering permanent by an as-enters choice.
    pub linked_exile_with_entering: Vec<crate::ids::ObjectId>,
    /// The original entry will not occur (prevention, redirection or replacement).
    pub prevented: bool,
    /// An instead-payload replaced the entry, rather than simply preventing it.
    pub replaced: bool,
    /// Completed entry metadata and all applications retained through commit.
    pub replacement_context: Option<Box<ReplacementEventContext>>,
    /// If zone was changed, the new destination
    pub new_destination: Option<Zone>,
    /// If set, the object enters as a copy of this source object.
    pub enters_as_copy_of: Option<crate::ids::ObjectId>,
    /// Consequences of that copy choice ("When you do, exile that card").
    pub copy_followups: Vec<ironsmith_core::EnterAsCopyFollowup>,
    /// Duration of a temporary as-enters copy effect, if any.
    pub copy_duration: Option<crate::effect::Until>,
    pub copy_name_override: Option<String>,
    /// Colors added by an ETB copy choice.
    pub added_colors: crate::color::ColorSet,
    /// Additional card types granted by an ETB copy choice.
    pub added_card_types: Vec<crate::types::CardType>,
    /// The copy's card types are exactly `added_card_types`.
    pub removes_other_card_types: bool,
    /// Supertypes removed by an ETB copy choice.
    pub added_supertypes: Vec<crate::types::Supertype>,
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

/// Test-only observation of damage assigned to the original target.
/// Production callers must retain the complete result and own its commit.
#[cfg(test)]
pub fn process_damage_summary_for_test(
    game: &mut GameState,
    source: ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    cause: crate::events::cause::EventCause,
) -> (u32, bool) {
    let processed =
        process_damage_assignments_with_event(game, source, target, amount, is_combat, cause)
            .expect("test damage proposal must process successfully");
    assert!(
        processed.programs.is_empty(),
        "a damage owner must finish retained replacement programs"
    );
    let amount = processed
        .assignments
        .iter()
        .filter(|assignment| assignment.target == target)
        .map(|assignment| assignment.amount)
        .sum();
    (amount, processed.replacement_prevented)
}

/// A final damage assignment after replacement and prevention effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessedDamageAssignment {
    pub target: DamageTarget,
    pub amount: u32,
}

/// Final result of processing damage through replacement and prevention effects.
#[derive(Debug, Clone)]
#[must_use = "commit assignments and finish retained damage replacement programs"]
pub struct ProcessedDamageResult {
    pub assignments: Vec<ProcessedDamageAssignment>,
    pub replacement_prevented: bool,
    /// Frozen Instead intents. Selection/sealing executes no payload action;
    /// the damage original owner must commit and retain these in this order.
    pub original_payloads: Vec<PreparedReplacementProgram>,
    /// Completed compatibility payloads only, never an uncommitted prefix.
    pub payload_outcome: Option<crate::effect::EffectOutcome>,
    /// Added replacement instructions belong to the eventual damage owner,
    /// after the complete original damage batch and its consequences.
    pub programs: Vec<PreparedReplacementProgram>,
}

/// Failure associated with the source of a damage event or prevention payload.
#[derive(Debug, Clone)]
pub struct DamageProcessingError {
    pub source: ObjectId,
    pub error: crate::effects::ExecutionError,
}

impl From<DamageProcessingError> for crate::effects::ExecutionError {
    fn from(failure: DamageProcessingError) -> Self {
        failure.error
    }
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
) -> Result<ProcessedDamageResult, crate::effects::ExecutionError> {
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
) -> Result<ProcessedDamageResult, crate::effects::ExecutionError> {
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
) -> Result<ProcessedDamageResult, crate::effects::ExecutionError> {
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
    fn matches_prepared_event(
        &self,
        event: &dyn crate::events::GameEventType,
        ctx: &crate::events::context::PreparedEventContext,
    ) -> bool {
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
        if !shield.has_prevention_remaining()
            || !crate::prevention::shield_duration_is_active(shield, ctx.game)
        {
            return false;
        }

        // CR 608.2h/609.7b: the live incarnation is authoritative, even when
        // it no longer has a quality retained by the ability's older snapshot.
        // Phasing is absence for this query, as in DamageFromSourceMatcher.
        let live_source = ctx
            .game
            .object(damage.source)
            .filter(|_| !ctx.game.is_phased_out(damage.source));
        let source_lki = if live_source.is_none() {
            ctx.game
                .turn_store
                .turn_history
                .source_last_known_snapshot(damage.source)
                .filter(|snapshot| snapshot.object_id == damage.source)
                .or_else(|| {
                    self.source_snapshot
                        .as_ref()
                        .filter(|snapshot| snapshot.object_id == damage.source)
                })
        } else {
            None
        };

        // CR 801.13b keys range to whichever side the prevention effect
        // specifies: source, recipient, or both when neither is specified.
        let range_exempt = ctx.game.source_is_exempt_from_range(Some(shield.source));
        let source_in_range = range_exempt
            || live_source.map_or_else(
                || {
                    source_lki.is_some_and(|snapshot| {
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
            let matches = if let Some(source) = live_source {
                source_filter.matches(source, &filter_ctx, ctx.game)
            } else {
                source_lki.is_some_and(|snapshot| {
                    source_filter.matches_snapshot(snapshot, &filter_ctx, ctx.game)
                })
            };
            if !matches {
                return false;
            }
        }

        let (source_colors, source_card_types) = if live_source.is_some() {
            // PreparedEventContext already checked continuous discovery.
            // A missing current view must never revive a stale snapshot.
            let Some(characteristics) = ctx.game.calculated_characteristics(damage.source) else {
                return false;
            };
            (characteristics.colors, characteristics.card_types.to_vec())
        } else if let Some(snapshot) = source_lki {
            (snapshot.colors, snapshot.card_types.clone())
        } else {
            if shield.damage_filter.from_colors.is_some()
                || shield.damage_filter.from_card_types.is_some()
            {
                return false;
            }
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
) -> Result<Vec<PreventionBatchAllocation>, DamageProcessingError> {
    let mut allocations = vec![PreventionBatchAllocation::default(); events.len()];
    if !game.can_prevent_damage() {
        return Ok(allocations);
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
        let mut eligible = Vec::new();
        for (index, item) in events.iter().enumerate() {
            if item.amount == 0
                || item.unpreventable
                || !game.can_prevent_damage_from(
                    item.source,
                    item.is_combat,
                    item.source_snapshot.as_ref(),
                )
            {
                continue;
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
            let ctx = EventContext::for_replacement_effect(shield.controller, shield.source, game)
                .with_event_source_snapshot(item.source_snapshot.as_ref());
            if matcher
                .matches_event(&damage, &ctx)
                .map_err(|error| DamageProcessingError {
                    source: item.source,
                    error: crate::effects::ExecutionError::ContinuousDiscovery(error),
                })?
            {
                eligible.push(index);
            }
        }

        let distinct_sources = eligible
            .iter()
            .map(|index| events[*index].source)
            .collect::<std::collections::HashSet<_>>();
        let total_damage = eligible
            .iter()
            .map(|index| u128::from(events[*index].amount))
            .sum::<u128>();
        if distinct_sources.len() < 2 || total_damage <= u128::from(capacity) {
            continue;
        }

        eligible.sort_by_key(|index| {
            let player = affected_player_for_damage(game, events[*index].target, shield.controller);
            (apnap_position(game, player), *index)
        });

        for index in &eligible {
            allocations[*index].allocated_shields.insert(shield.id);
        }

        let mut remaining = u128::from(capacity).min(total_damage) as u32;
        for (position, index) in eligible.iter().copied().enumerate() {
            let later_damage = eligible[position + 1..]
                .iter()
                .map(|later| u128::from(events[*later].amount))
                .sum::<u128>();
            let minimum = u128::from(remaining).saturating_sub(later_damage) as u32;
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

    Ok(allocations)
}

/// Process damage events that would happen simultaneously as one CR 615.7 batch.
///
/// Limited prevention shields are allocated before any event consumes them. The
/// returned results remain aligned with the input events when complete. If the
/// decision maker is awaiting a choice, callers must discard the partial result
/// and replay the batch after receiving an answer.
pub fn process_simultaneous_damage_assignments_with_event_with_dm(
    game: &mut GameState,
    events: &[SimultaneousDamageEvent],
    dm: &mut dyn DecisionMaker,
) -> Result<Vec<ProcessedDamageResult>, DamageProcessingError> {
    process_simultaneous_damage_assignments_with_event_with_scope(
        game,
        events,
        dm,
        &crate::effects::ReplacementExecutionContext::default(),
    )
}

pub(crate) fn process_simultaneous_damage_assignments_with_event_with_scope(
    game: &mut GameState,
    events: &[SimultaneousDamageEvent],
    dm: &mut dyn DecisionMaker,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
) -> Result<Vec<ProcessedDamageResult>, DamageProcessingError> {
    let scopes = vec![replacement_scope; events.len()];
    process_simultaneous_damage_assignments_with_scopes(game, events, dm, &scopes)
}

/// Allocate prevention once across the complete batch, while each assignment
/// retains its captured replacement scope. Results keep the same input order.
#[must_use = "retain assignment results and their captured prevention follow-ups"]
pub(crate) struct PreparedDamageProcessingBatch {
    results: Vec<ProcessedDamageResult>,
    follow_ups: Vec<CapturedPreventionFollowUps>,
}
impl PreparedDamageProcessingBatch {
    fn empty() -> Self {
        Self {
            results: Vec::new(),
            follow_ups: Vec::new(),
        }
    }
    /// Every result has a matching slot, including an explicitly empty batch.
    pub(crate) fn into_parts(
        self,
    ) -> (Vec<ProcessedDamageResult>, Vec<CapturedPreventionFollowUps>) {
        (self.results, self.follow_ups)
    }
}

pub(crate) fn process_simultaneous_damage_assignments_with_scopes(
    game: &mut GameState,
    events: &[SimultaneousDamageEvent],
    dm: &mut dyn DecisionMaker,
    replacement_scopes: &[&crate::effects::ReplacementExecutionContext],
) -> Result<Vec<ProcessedDamageResult>, DamageProcessingError> {
    process_simultaneous_damage_assignments_with_completion(
        game,
        events,
        dm,
        replacement_scopes,
        |game, dm, prepared| {
            let (mut results, follow_ups) = prepared.into_parts();
            let (_, payload_follow_ups) =
                capture_deferred_prevention_follow_ups(game, dm, |game, dm| {
                    for (index, processed) in results.iter_mut().enumerate() {
                        damage_original_payloads::complete_legacy(
                            game,
                            dm,
                            replacement_scopes[index],
                            processed,
                        )
                        .map_err(|error| DamageProcessingError {
                            source: events[index].source,
                            error,
                        })?;
                        if dm.awaiting_choice() {
                            break;
                        }
                    }
                    Ok::<_, DamageProcessingError>(())
                })?;
            let pending = follow_ups
                .into_iter()
                .flat_map(|batch| batch.pending)
                .chain(payload_follow_ups.pending)
                .collect();
            let follow_ups = CapturedPreventionFollowUps { pending };
            if game
                .effect_store
                .prevention_effects
                .follow_ups_are_deferred()
            {
                follow_ups.requeue(game);
            } else {
                follow_ups.complete(game, dm)?;
            }
            Ok(results)
        },
        Vec::new,
    )
}

/// The same assignment/prevention owner can retain follow-ups for a prepared
/// cohort without completing or requeuing them at this preparation boundary.
pub(crate) fn prepare_simultaneous_damage_assignments_with_scopes(
    game: &mut GameState,
    events: &[SimultaneousDamageEvent],
    dm: &mut dyn DecisionMaker,
    replacement_scopes: &[&crate::effects::ReplacementExecutionContext],
) -> Result<PreparedDamageProcessingBatch, DamageProcessingError> {
    process_simultaneous_damage_assignments_with_completion(
        game,
        events,
        dm,
        replacement_scopes,
        |_, _, prepared| Ok(prepared),
        PreparedDamageProcessingBatch::empty,
    )
}

fn process_simultaneous_damage_assignments_with_completion<R>(
    game: &mut GameState,
    events: &[SimultaneousDamageEvent],
    dm: &mut dyn DecisionMaker,
    replacement_scopes: &[&crate::effects::ReplacementExecutionContext],
    complete: impl FnOnce(
        &mut GameState,
        &mut dyn DecisionMaker,
        PreparedDamageProcessingBatch,
    ) -> Result<R, DamageProcessingError>,
    empty: impl FnOnce() -> R,
) -> Result<R, DamageProcessingError> {
    if replacement_scopes.len() != events.len() {
        return Err(DamageProcessingError {
            source: events
                .first()
                .map(|event| event.source)
                .unwrap_or(ObjectId::from_raw(0)),
            error: crate::effects::ExecutionError::InternalError(
                "simultaneous damage lost an assignment replacement scope".into(),
            ),
        });
    }
    if events.is_empty() || dm.awaiting_choice() {
        return Ok(empty());
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let result = (|| {
        game.update_cant_effects();
        game.update_replacement_effects()
            .map_err(|error| DamageProcessingError {
                source: events[0].source,
                error: crate::effects::ExecutionError::ContinuousDiscovery(error),
            })?;
        let pending_event_start = game.effect_store.pending_trigger_events.len();
        // The prevention events of this batch are coalesced below.
        game.effect_store.trigger_matching_holds += 1;
        let schedule_redirect_budget = simultaneous_redirect_budget::needed(game);
        let allocations = if schedule_redirect_budget {
            Vec::new()
        } else {
            collect_simultaneous_prevention_allocations(game, events, dm)?
        };
        if dm.awaiting_choice() {
            game.effect_store.trigger_matching_holds -= 1;
            return Ok(PreparedDamageProcessingBatch::empty());
        }
        // CR 615.5: the batch's additional prevention effects happen after the
        // whole simultaneous damage event, so they are collected here.
        let follow_up_start = game
            .effect_store
            .prevention_effects
            .begin_follow_up_deferral();
        game.effect_store.replacement_effects.begin_damage_occurrence();
        let (results, follow_up_owners) = if schedule_redirect_budget {
            simultaneous_redirect_budget::process(game, events, dm, replacement_scopes)?
        } else {
            let mut results = Vec::with_capacity(events.len());
            let mut follow_up_owners = Vec::new();
            for (index, item) in events.iter().enumerate() {
                let before = game
                    .effect_store
                    .prevention_effects
                    .pending_follow_up_count();
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
                    replacement_scopes[index],
                ).map_err(|error| DamageProcessingError { source: item.source, error })?,
            );
                let after = game
                    .effect_store
                    .prevention_effects
                    .pending_follow_up_count();
                let count = after
                    .checked_sub(before)
                    .ok_or_else(|| DamageProcessingError {
                        source: item.source,
                        error: crate::effects::ExecutionError::InternalError(
                            "damage assignment consumed another assignment's prevention follow-ups"
                                .into(),
                        ),
                    })?;
                follow_up_owners.extend(std::iter::repeat_n(index, count));
                if dm.awaiting_choice() {
                    break;
                }
            }
            (results, follow_up_owners)
        };
        // All matching siblings have now seen the same registrations. Later
        // damage from a prevention follow-up starts a separate occurrence.
        game.effect_store
            .replacement_effects
            .finish_damage_occurrence();
        game.effect_store.trigger_matching_holds -= 1;
        coalesce_simultaneous_shield_prevention_events(game, pending_event_start).map_err(
            |error| DamageProcessingError {
                source: events[0].source,
                error,
            },
        )?;
        let pending = game
            .effect_store
            .prevention_effects
            .end_follow_up_deferral(follow_up_start);
        if pending.len() != follow_up_owners.len() {
            return Err(DamageProcessingError {
                source: events[0].source,
                error: crate::effects::ExecutionError::InternalError(
                    "simultaneous prevention follow-ups lost assignment ownership".into(),
                ),
            });
        }
        let mut indexed = follow_up_owners
            .into_iter()
            .zip(pending)
            .collect::<Vec<_>>();
        dedupe_shield_counter_follow_ups(&mut indexed);
        let mut follow_ups = (0..results.len())
            .map(|_| CapturedPreventionFollowUps {
                pending: Vec::new(),
            })
            .collect::<Vec<_>>();
        for (index, pending) in indexed {
            follow_ups[index].pending.push(pending);
        }
        Ok(PreparedDamageProcessingBatch {
            results,
            follow_ups,
        })
    })()
    .and_then(|prepared| complete(game, dm, prepared));
    if dm.awaiting_choice() && result.is_ok() {
        game.restore_execution_checkpoint(checkpoint, true);
        return Ok(empty());
    }
    if result.is_err() {
        game.restore_execution_checkpoint(checkpoint, false);
    }
    result
}
/// CR 122.1c / 510.2: simultaneous damage to a permanent is one damage event,
/// so its shield counter prevents all of it and only one shield counter is
/// removed, however many sources dealt damage.
fn dedupe_shield_counter_follow_ups(
    follow_ups: &mut Vec<(usize, crate::prevention::PendingPreventionFollowUp)>,
) {
    let shield_target = |pending: &crate::prevention::PendingPreventionFollowUp| {
        let [effect] = pending.follow_up.effects.as_slice() else {
            return None;
        };
        let remove = effect.downcast_ref::<crate::effects::RemoveCountersEffect>()?;
        match (remove.counter_type, &remove.target) {
            (CounterType::Shield, crate::target::ChooseSpec::SpecificObject(id))
                if *id == pending.follow_up.source =>
            {
                Some(*id)
            }
            _ => None,
        }
    };
    let mut seen: Vec<ObjectId> = Vec::new();
    follow_ups.retain(|(_, pending)| match shield_target(pending) {
        Some(id) if seen.contains(&id) => false,
        Some(id) => {
            seen.push(id);
            true
        }
        None => true,
    });
}

fn coalesce_simultaneous_shield_prevention_events(
    game: &mut GameState,
    start_index: usize,
) -> Result<(), crate::effects::ExecutionError> {
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
        let mut merged = false;
        for (_, existing) in &mut grouped {
            if existing.merge_simultaneous(prevented.clone())? {
                merged = true;
                break;
            }
        }
        if merged {
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
    Ok(())
}

/// Deterministic convenience wrapper for a simultaneous damage batch.
pub fn process_simultaneous_damage_assignments_with_event(
    game: &mut GameState,
    events: &[SimultaneousDamageEvent],
) -> Result<Vec<ProcessedDamageResult>, DamageProcessingError> {
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
) -> Result<ProcessedDamageResult, crate::effects::ExecutionError> {
    process_damage_assignments_with_event_with_source_snapshot_opts_with_scope(
        game,
        source,
        target,
        amount,
        is_combat,
        unpreventable,
        cause,
        source_snapshot,
        dm,
        &crate::effects::ReplacementExecutionContext::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_damage_assignments_with_event_with_source_snapshot_opts_with_scope(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    unpreventable: bool,
    cause: crate::events::cause::EventCause,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    dm: &mut dyn DecisionMaker,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
) -> Result<ProcessedDamageResult, crate::effects::ExecutionError> {
    if dm.awaiting_choice() {
        return Ok(ProcessedDamageResult {
            assignments: Vec::new(),
            replacement_prevented: true,
            original_payloads: Vec::new(),
            payload_outcome: None,
            programs: Vec::new(),
        });
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    game.effect_store
        .replacement_effects
        .begin_damage_occurrence();
    let result =
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
            replacement_scope,
        )
        .and_then(|mut processed| {
            damage_original_payloads::complete_legacy(game, dm, replacement_scope, &mut processed)?;
            Ok(processed)
        });
    if dm.awaiting_choice() && result.is_ok() {
        game.restore_execution_checkpoint(checkpoint, true);
        return Ok(ProcessedDamageResult {
            assignments: Vec::new(),
            replacement_prevented: true,
            original_payloads: Vec::new(),
            payload_outcome: None,
            programs: Vec::new(),
        });
    }
    if result.is_err() {
        game.restore_execution_checkpoint(checkpoint, false);
    } else {
        game.effect_store
            .replacement_effects
            .finish_damage_occurrence();
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn prepare_damage_proposal(
    game: &mut GameState,
    source: ObjectId,
    target: DamageTarget,
    amount: u32,
    is_combat: bool,
    unpreventable: bool,
    cause: crate::events::cause::EventCause,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Event {
    // Check if damage can be prevented
    let can_prevent =
        !unpreventable && game.can_prevent_damage_from(source, is_combat, source_snapshot);

    // Create the event using the new Event type
    let event = if can_prevent {
        Event::damage(source, target, amount, is_combat, cause.clone())
    } else {
        Event::unpreventable_damage(source, target, amount, is_combat, cause.clone())
    };

    // Keep supplied damage-source LKI on the event envelope as well as the
    // matcher context, so prevention follow-ups retain the same source identity.
    let event = if let Some(snapshot) = source_snapshot {
        crate::events::Event::from_raw(event.into_raw().with_source_snapshot(snapshot.clone()))
    } else {
        event
    };
    // Process through the trait-based system, retaining event provenance for
    // replacement-generated effect execution.
    game.ensure_event_provenance(event)
}

fn damage_additional_replacements(
    game: &GameState,
    target: DamageTarget,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    batch_allocation: Option<&PreventionBatchAllocation>,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
) -> Vec<ReplacementEffect> {
    // CR 615.12 still applies prevention effects to unpreventable damage. They
    // prevent zero, retain their shield capacity, and perform additional parts.
    let mut prevention_effects =
        prevention_shield_replacement_effects(game, source_snapshot, batch_allocation);
    // CR 122.1c shield counters are prevention effects ordered with the
    // others by the affected player (CR 616.1).
    prevention_effects.extend(shield_counter_damage_replacements(game, target));
    assign_ephemeral_effect_ids(&mut prevention_effects, u64::MAX / 4);
    let mut additional = replacement_scope.additional_replacement_effects.clone();
    assign_ephemeral_effect_ids(&mut additional, u64::MAX / 2);
    prevention_effects.extend(additional);
    prevention_effects
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
    replacement_scope: &crate::effects::ReplacementExecutionContext,
) -> Result<ProcessedDamageResult, crate::effects::ExecutionError> {
    game.update_cant_effects();
    game.update_replacement_effects()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;

    let event = prepare_damage_proposal(
        game,
        source,
        target,
        amount,
        is_combat,
        unpreventable,
        cause.clone(),
        source_snapshot,
    );
    let event_provenance = event.provenance();
    let prevention_effects = damage_additional_replacements(
        game,
        target,
        source_snapshot,
        batch_allocation,
        replacement_scope,
    );
    game.effect_store
        .prevention_effects
        .begin_follow_up_replacement_scope(replacement_scope);
    let mut processing_state = TraitEventProcessingState::default();
    let result = process_with_dm_and_additional_effects_and_applied_state(
        game,
        event,
        dm,
        &prevention_effects,
        &replacement_scope.suppressed_replacement_effects,
        &replacement_scope.suppressed_replacement_effect_keys,
        source_snapshot,
        &mut processing_state,
    );
    game.effect_store
        .prevention_effects
        .end_follow_up_replacement_scope();
    let result = result?;
    if dm.awaiting_choice() {
        return Ok(ProcessedDamageResult {
            assignments: Vec::new(),
            replacement_prevented: true,
            original_payloads: Vec::new(),
            payload_outcome: None,
            programs: Vec::new(),
        });
    }
    execute_pending_prevention_follow_ups(game, dm)?;
    if dm.awaiting_choice() {
        return Ok(ProcessedDamageResult {
            assignments: Vec::new(),
            replacement_prevented: true,
            original_payloads: Vec::new(),
            payload_outcome: None,
            programs: Vec::new(),
        });
    }

    finish_processed_damage_result(
        game,
        source,
        cause,
        source_snapshot,
        event_provenance,
        dm,
        replacement_scope,
        result,
        processing_state,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_processed_damage_result(
    game: &mut GameState,
    source: ObjectId,
    _cause: crate::events::cause::EventCause,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    _event_provenance: crate::provenance::ProvNodeId,
    dm: &mut dyn DecisionMaker,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    result: TraitEventResult,
    processing_state: TraitEventProcessingState,
) -> Result<ProcessedDamageResult, crate::effects::ExecutionError> {
    use crate::events::{DamageEvent, downcast_event};

    let (result, mut programs) = result.into_expansion();
    let mut assignments = Vec::new();
    let mut payload_outcomes = Vec::new();
    let mut original_payloads = Vec::new();
    let mut replacement_prevented = false;
    let replaced = match result {
        TraitEventResult::Expanded { .. } => {
            return Err(crate::effects::ExecutionError::InternalError(
                "damage result retained an unflattened expansion".into(),
            ));
        }
        TraitEventResult::Prevented => {
            replacement_prevented = true;
            None
        }
        TraitEventResult::Replaced {
            effects,
            context,
            source: replacement_source,
            controller: replacement_controller,
            ..
        } => {
            downcast_event::<DamageEvent>(context.event.inner()).ok_or_else(|| {
                crate::effects::ExecutionError::InternalError(
                    "damage replacement lost its damage event".into(),
                )
            })?;
            let replacement_snapshot = game.object(replacement_source)
                .filter(|_| !game.is_phased_out(replacement_source))
                .map(|object| crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game))
                .transpose()?
                .or_else(|| game.source_last_known_snapshot(replacement_source).cloned())
                .or_else(|| (replacement_source == source).then(|| source_snapshot.cloned()).flatten());
            original_payloads.push(PreparedReplacementProgram {
                context,
                source: replacement_source,
                controller: replacement_controller,
                source_snapshot: replacement_snapshot,
                effects,
            });
            replacement_prevented = true;
            None
        }
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(damage) = downcast_event::<DamageEvent>(e.inner()) {
                Some(damage.clone())
            } else {
                return Err(crate::effects::ExecutionError::InternalError(
                    "damage replacement processing returned a non-DamageEvent".into(),
                ));
            }
        }
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            if !dm.awaiting_choice() {
                return Err(crate::effects::ExecutionError::InternalError(
                    "damage replacement suspended without a captured decision".into(),
                ));
            }
            return Ok(ProcessedDamageResult {
                assignments: Vec::new(),
                replacement_prevented: true,
                original_payloads: Vec::new(),
                payload_outcome: None,
                programs: Vec::new(),
            });
        }
    };

    if let Some(replaced) = replaced {
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
    }

    for pending in processing_state.damage_remainders {
        let mut branch_scope = replacement_scope.clone();
        branch_scope.suppressed_replacement_effects.extend(
            pending
                .applied_effects
                .into_iter()
                .filter(|id| id.0 < u64::MAX / 4),
        );
        branch_scope
            .suppressed_replacement_effect_keys
            .extend(pending.applied_effect_keys);
        // A split remainder belongs to the same damage occurrence as its
        // primary branch, including when that primary branch was replaced.
        let remainder =
            process_damage_assignments_with_event_with_source_snapshot_opts_with_dm_and_allocation(
                game,
                pending.event.source,
                pending.event.target,
                pending.event.amount,
                pending.event.is_combat,
                pending.event.is_unpreventable,
                pending.event.cause,
                source_snapshot,
                dm,
                None,
                &branch_scope,
            )?;
        if dm.awaiting_choice() {
            return Ok(ProcessedDamageResult {
                assignments: Vec::new(),
                replacement_prevented: true,
                original_payloads: Vec::new(),
                payload_outcome: None,
                programs: Vec::new(),
            });
        }
        replacement_prevented |= remainder.replacement_prevented;
        assignments.extend(remainder.assignments);
        payload_outcomes.extend(remainder.payload_outcome);
        original_payloads.extend(remainder.original_payloads);
        programs.extend(remainder.programs);
    }

    Ok(ProcessedDamageResult {
        assignments,
        replacement_prevented,
        original_payloads,
        programs,
        payload_outcome: (!payload_outcomes.is_empty())
            .then(|| crate::effect::EffectOutcome::aggregate(payload_outcomes)),
    })
}

fn execute_pending_prevention_follow_ups(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    if game
        .effect_store
        .prevention_effects
        .follow_ups_are_deferred()
    {
        return Ok(());
    }
    let pending = game
        .effect_store
        .prevention_effects
        .take_pending_follow_ups();
    CapturedPreventionFollowUps { pending }
        .complete(game, dm)
        .map(|_| ())
        .map_err(|failure| failure.error)
}

/// Follow-ups owned by one completed prevention-deferral scope. Capture does
/// not execute them; a staged action retains this batch until originals and
/// observations finish. Consuming completion preserves their captured order.
#[derive(Debug)]
#[must_use = "retain prevention follow-ups until the owning damage action completes"]
pub(crate) struct CapturedPreventionFollowUps {
    pending: Vec<crate::prevention::PendingPreventionFollowUp>,
}

impl CapturedPreventionFollowUps {
    /// Declare the authored owners before transferring this queue suffix.
    /// This changes routing metadata only, never follow-up order or execution.
    pub(crate) fn with_participants(
        mut self,
        scopes: Vec<crate::effects::EffectOutcomeScope>,
    ) -> Self {
        for pending in &mut self.pending {
            pending.participant_scopes = scopes.clone();
        }
        self
    }
    /// Transfer ownership back to the caller's still-open full-action scope.
    /// Ordinary damage does this immediately after preparation, restoring the
    /// original queue order before originals and added programs execute.
    pub(crate) fn requeue(self, game: &mut GameState) {
        debug_assert!(
            game.effect_store
                .prevention_effects
                .follow_ups_are_deferred()
        );
        for pending in self.pending {
            game.effect_store
                .prevention_effects
                .requeue_follow_up(pending);
        }
    }

    pub(crate) fn complete(
        self,
        game: &mut GameState,
        dm: &mut dyn DecisionMaker,
    ) -> Result<Vec<crate::effect::EffectOutcome>, DamageProcessingError> {
        self.complete_with_outputs(game, dm).map(|outcomes| {
            outcomes
                .into_iter()
                .map(crate::effects::CompletedEffectOutputs::into_outcome)
                .collect()
        })
    }

    pub(crate) fn complete_with_outputs(
        self,
        game: &mut GameState,
        dm: &mut dyn DecisionMaker,
    ) -> Result<Vec<crate::effects::CompletedEffectOutputs>, DamageProcessingError> {
        if dm.awaiting_choice() {
            return Ok(Vec::new());
        }
        execute_prevention_follow_ups_with_outputs(game, dm, self.pending)
    }

    fn complete_outputs(
        self,
        game: &mut GameState,
        dm: &mut dyn DecisionMaker,
        mut original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, DamageProcessingError> {
        let outcomes = if dm.awaiting_choice() {
            Vec::new()
        } else {
            execute_prevention_follow_ups_with_owned_outputs(game, dm, self.pending)?
        };
        let aggregate = crate::effect::EffectOutcome::aggregate_replacement_outcomes(
            original.outcome.clone(),
            outcomes.iter().map(|(_, outputs)| outputs.outcome.clone()),
        );
        original
            .shared
            .extend(outcomes.into_iter().map(|(scopes, outputs)| {
                crate::effects::SharedEffectOutcome {
                    ownership: if scopes.is_empty() {
                        crate::effects::SharedOutcomeOwnership::Batch
                    } else {
                        crate::effects::SharedOutcomeOwnership::Participants(scopes)
                    },
                    outputs: outputs.into(),
                }
            }));
        Ok(original.project_aggregate(aggregate))
    }
}

/// Close this action's deferral scope and return its follow-ups without running
/// any of them. Nested scopes retain only their own suffix of the pending queue.
/// Failed or suspended actions discard their suffix; their caller owns replay
/// and rollback, just as for the ordinary full-action deferral gateway.
pub(crate) fn capture_deferred_prevention_follow_ups<R, E: From<DamageProcessingError>>(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
    apply_damage: impl FnOnce(&mut GameState, &mut dyn DecisionMaker) -> Result<R, E>,
) -> Result<(R, CapturedPreventionFollowUps), E> {
    let start = game
        .effect_store
        .prevention_effects
        .begin_follow_up_deferral();
    let result = apply_damage(game, dm);
    let pending = game
        .effect_store
        .prevention_effects
        .end_follow_up_deferral(start);
    let pending = if result.is_ok() && !dm.awaiting_choice() {
        pending
    } else {
        Vec::new()
    };
    result.map(|result| (result, CapturedPreventionFollowUps { pending }))
}

/// Commit one damage application before executing its additional prevention
/// effects (CR 615.5). Nested damage owns only the follow-ups it produces.
pub(crate) fn with_deferred_prevention_follow_ups<R, E: From<DamageProcessingError>>(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
    apply_damage: impl FnOnce(&mut GameState, &mut dyn DecisionMaker) -> Result<R, E>,
) -> Result<R, E> {
    let (result, follow_ups) = capture_deferred_prevention_follow_ups(game, dm, apply_damage)?;
    follow_ups.complete(game, dm).map_err(E::from)?;
    Ok(result)
}

pub(crate) fn with_deferred_prevention_follow_up_outputs<E: From<DamageProcessingError>>(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
    apply_damage: impl FnOnce(
        &mut GameState,
        &mut dyn DecisionMaker,
    ) -> Result<crate::effects::CompletedEffectOutputs, E>,
) -> Result<crate::effects::CompletedEffectOutputs, E> {
    let (outcome, follow_ups) = capture_deferred_prevention_follow_ups(game, dm, apply_damage)?;
    follow_ups
        .complete_outputs(game, dm, outcome)
        .map_err(E::from)
}

fn execute_prevention_follow_ups_with_outputs(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
    pending: Vec<crate::prevention::PendingPreventionFollowUp>,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, DamageProcessingError> {
    execute_prevention_follow_ups_with_owned_outputs(game, dm, pending)
        .map(|outputs| outputs.into_iter().map(|(_, outputs)| outputs).collect())
}

fn execute_prevention_follow_ups_with_owned_outputs(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
    pending: Vec<crate::prevention::PendingPreventionFollowUp>,
) -> Result<
    Vec<(
        Vec<crate::effects::EffectOutcomeScope>,
        crate::effects::CompletedEffectOutputs,
    )>,
    DamageProcessingError,
> {
    let mut outcomes = Vec::new();
    for pending in pending {
        if dm.awaiting_choice() {
            break;
        }
        let follow_up = &pending.follow_up;
        let mut exec_ctx = prevention_draw_boundary::context(game, &mut *dm, &pending)?;
        for effect in &follow_up.effects {
            let outcome = crate::effects::execute_effect_with_outputs(game, effect, &mut exec_ctx)
                .map_err(|error| DamageProcessingError {
                    source: follow_up.source,
                    error,
                })?;
            if exec_ctx.decision_maker.awaiting_choice() {
                return Ok(outcomes);
            }
            for trigger_event in &outcome.outcome.events {
                game.queue_trigger_event(trigger_event.provenance(), trigger_event.clone());
            }
            outcomes.push((pending.participant_scopes.clone(), outcome));
        }
    }
    Ok(outcomes)
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
) -> Result<Option<Zone>, crate::effects::ExecutionError> {
    use crate::events::{ZoneChangeEvent, downcast_event};

    let event = Event::zone_change(
        creature,
        Zone::Battlefield,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_sba(),
        Some(snapshot),
    );
    let result = process_trait_event(game, event)?;

    Ok(match result {
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
    })
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
) -> Result<Option<Zone>, crate::effects::ExecutionError> {
    use crate::events::{ZoneChangeEvent, downcast_event};

    let snapshot = game
        .object(object)
        .map(|o| {
            crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                o, game,
            )
        })
        .transpose()?;
    let event = Event::zone_change(object, from, to, cause, snapshot);
    let result = process_trait_event(game, event)?;

    Ok(match result {
        TraitEventResult::Prevented => None,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner()) {
                Some(zone_change.to)
            } else {
                Some(to)
            }
        }
        _ => Some(to),
    })
}

/// Process a put counters event using the new Event type.
///
/// Returns the final number of counters to place. Callers without a decision
/// maker resolve ties among several counter replacements (CR 616.1) by
/// applying them in a deterministic order; every applicable modifier still
/// applies (Hardened Scales + Doubling Season both modify the event).
pub fn process_put_counters_with_event(
    game: &mut GameState,
    target: crate::ids::ObjectId,
    counter_type: CounterType,
    count: u32,
    cause: crate::events::cause::EventCause,
) -> Result<u32, crate::effects::ExecutionError> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    process_put_counters_with_event_with_dm(game, target, counter_type, count, cause, &mut dm)
}

/// Process a put counters event, asking the affected object's controller to
/// order tied counter replacements (CR 616.1, 616.1e).
///
/// Returns the final number of counters to place (0 while a choice is pending).
pub fn process_put_counters_with_event_with_dm(
    game: &mut GameState,
    target: crate::ids::ObjectId,
    counter_type: CounterType,
    count: u32,
    cause: crate::events::cause::EventCause,
    dm: &mut (impl DecisionMaker + ?Sized),
) -> Result<u32, crate::effects::ExecutionError> {
    let query = game
        .continuous_query_snapshot()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    if !query.can_have_counter_type_placed(target, counter_type) {
        return Ok(0);
    }

    let event = Event::put_counters(target, counter_type, count, cause);
    Ok(put_counters_result_count(
        process_with_dm(game, event, dm)?,
        count,
    ))
}

fn put_counters_result_count(result: TraitEventResult, count: u32) -> u32 {
    use crate::events::{PutCountersEvent, downcast_event};

    match result {
        TraitEventResult::Prevented => 0,
        TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
            if let Some(put_counters) = downcast_event::<PutCountersEvent>(e.inner()) {
                put_counters.count
            } else {
                count
            }
        }
        // A replacement-order choice is still pending; nothing is placed yet.
        TraitEventResult::NeedsChoice { .. } => 0,
        _ => count,
    }
}

/// The complete result of replacing a token-creation proposal.
/// A finished payload/prevention has no original token creation to commit.
pub enum TokenCreationReplacementResult {
    Proceed {
        event: crate::events::CreateTokensEvent,
        provenance: crate::provenance::ProvNodeId,
        programs: Vec<PreparedReplacementProgram>,
    },
    Finished(crate::effect::EffectOutcome),
}

/// Resolve, commit, and complete a token-creation operation atomically.
///
/// The callback commits only the final token event and returns the actual
/// created-object receipt, including its observations and execution facts.
/// Appended replacement programs run after that callback completes. Callers
/// must propagate pending choices from nested entry processing; this owner
/// restores selection, creation, and deferred consequences together on pending
/// input or error. Preparations outside this operation still need their outer
/// instruction's checkpoint.
pub fn execute_token_creation_with_event<'a, F>(
    game: &mut GameState,
    controller: PlayerId,
    count: u32,
    token: Option<crate::object::Object>,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext<'a>,
    commit_original: F,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError>
where
    F: FnOnce(
        &mut GameState,
        &mut crate::effects::ExecutionContext<'a>,
        crate::events::CreateTokensEvent,
        crate::provenance::ProvNodeId,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError>,
{
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effect::EffectOutcome::with_objects(Vec::new()));
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        let prepared = prepare_token_creation(game, controller, count, token, cause, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effect::EffectOutcome::with_objects(Vec::new()));
        }
        match prepared {
            TokenCreationReplacementResult::Finished(outcome) => Ok(outcome),
            TokenCreationReplacementResult::Proceed {
                event,
                provenance,
                programs,
            } => {
                let original = commit_original(game, ctx, event, provenance)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effect::EffectOutcome::with_objects(Vec::new()));
                }
                crate::effects::replacement::execute_deferred_replacement_programs(
                    game, ctx, original, programs,
                )
            }
        }
    })();
    let pending = ctx.decision_maker.awaiting_choice();
    if pending || result.is_err() {
        game.restore_execution_checkpoint(checkpoint, pending && result.is_ok());
        context_checkpoint.restore(ctx);
    }
    if pending && result.is_ok() {
        return Ok(crate::effect::EffectOutcome::with_objects(Vec::new()));
    }
    result
}

/// Resolve a token proposal without a particular token definition.
/// Returns the complete event or replacement outcome instead of a scalar count.
pub fn process_token_creation_with_event(
    game: &mut GameState,
    controller: PlayerId,
    count: u32,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
) -> Result<TokenCreationReplacementResult, crate::effects::ExecutionError> {
    process_token_creation_for_token_with_event(game, controller, count, None, cause, ctx)
}

/// Resolve token creation using the enclosing instruction's context/history.
pub fn process_token_creation_for_token_with_event(
    game: &mut GameState,
    controller: PlayerId,
    count: u32,
    token: Option<crate::object::Object>,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
) -> Result<TokenCreationReplacementResult, crate::effects::ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(TokenCreationReplacementResult::Finished(
            crate::effect::EffectOutcome::with_objects(Vec::new()),
        ));
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = prepare_token_creation(game, controller, count, token, cause, ctx);
    let pending = ctx.decision_maker.awaiting_choice();
    if pending || result.is_err() {
        game.restore_execution_checkpoint(checkpoint, pending && result.is_ok());
        context_checkpoint.restore(ctx);
    }
    if pending && result.is_ok() {
        return Ok(TokenCreationReplacementResult::Finished(
            crate::effect::EffectOutcome::with_objects(Vec::new()),
        ));
    }
    result
}

/// Prepared original plus appended programs, retained by a simultaneous owner.
pub(crate) enum PreparedTokenCreation {
    Proceed {
        event: crate::events::CreateTokensEvent,
        provenance: crate::provenance::ProvNodeId,
        programs: Vec<PreparedReplacementProgram>,
    },
    Finished {
        outputs: crate::effects::CompletedEffectOutputs,
        programs: Vec<PreparedReplacementProgram>,
    },
}

fn prepare_token_creation(
    game: &mut GameState,
    controller: PlayerId,
    count: u32,
    token: Option<crate::object::Object>,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
) -> Result<TokenCreationReplacementResult, crate::effects::ExecutionError> {
    match prepare_token_creation_deferred(game, controller, count, token, cause, ctx)? {
        PreparedTokenCreation::Proceed {
            event,
            provenance,
            programs,
        } => Ok(TokenCreationReplacementResult::Proceed {
            event,
            provenance,
            programs,
        }),
        PreparedTokenCreation::Finished { outputs, programs } => {
            crate::effects::replacement::execute_deferred_replacement_programs(
                game,
                ctx,
                outputs.into_outcome(),
                programs,
            )
            .map(TokenCreationReplacementResult::Finished)
        }
    }
}

pub(crate) fn prepare_token_creation_deferred(
    game: &mut GameState,
    controller: PlayerId,
    count: u32,
    token: Option<crate::object::Object>,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
) -> Result<PreparedTokenCreation, crate::effects::ExecutionError> {
    use crate::effect::{EffectOutcome, OutcomeStatus, OutcomeValue};
    use crate::effects::ExecutionError;
    use crate::events::{CreateTokensEvent, downcast_event};
    if count == 0 || ctx.decision_maker.awaiting_choice() {
        return Ok(PreparedTokenCreation::Finished {
            outputs: crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::with_objects(Vec::new()),
            ),
            programs: Vec::new(),
        });
    }
    let proposal = match token {
        Some(token) => CreateTokensEvent::with_token_cause(controller, count, token, cause),
        None => CreateTokensEvent::with_cause(controller, count, cause),
    };
    let event = Event::new_with_provenance(proposal, ctx.provenance);
    let result = process_trait_event_with_execution_context(game, event, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(PreparedTokenCreation::Finished {
            outputs: crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::with_objects(Vec::new()),
            ),
            programs: Vec::new(),
        });
    }
    let (result, programs) = result.into_expansion();
    match result {
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let token_event =
                downcast_event::<CreateTokensEvent>(event.inner()).ok_or_else(|| {
                    ExecutionError::InternalError(
                        "token replacement returned an incompatible event".into(),
                    )
                })?;
            Ok(PreparedTokenCreation::Proceed {
                event: token_event.clone(),
                provenance: event.provenance(),
                programs,
            })
        }
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => {
            let mut original = EffectOutcome::replaced();
            original.set_value(OutcomeValue::Count(0));
            let outputs =
                crate::effects::replacement::execute_replacement_original_payload_with_outputs(
                    game,
                    ctx,
                    &effects,
                    source,
                    controller,
                    &context,
                    crate::effects::replacement::ReplacementProgramBindings {
                        targets: None,
                        object_tags: Vec::new(),
                    },
                    None,
                    original,
                )?;
            Ok(PreparedTokenCreation::Finished { outputs, programs })
        }
        TraitEventResult::Prevented => {
            let mut outcome = EffectOutcome::prevented();
            outcome.value = OutcomeValue::Count(0);
            Ok(PreparedTokenCreation::Finished {
                outputs: crate::effects::CompletedEffectOutputs::aggregate_only(outcome),
                programs,
            })
        }
        TraitEventResult::Expanded { .. } => Err(ExecutionError::InternalError(
            "token replacement expansion did not flatten to an original result".into(),
        )),
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            Err(ExecutionError::InternalError(
                "token replacement suspended without a captured decision".into(),
            ))
        }
    }
}

/// Process an ETB event using the new Event type.
///
/// This is the Event-based version of `process_etb_event`.
pub fn process_etb_with_event(
    game: &GameState,
    object: crate::ids::ObjectId,
    from: Zone,
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
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
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
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
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    process_etb_with_event_and_dm_with_initial_counters_and_reservations(
        game,
        object,
        from,
        dm,
        initial_enters_with_counters,
        false,
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
    initial_enters_tapped: bool,
    cause: crate::events::cause::EventCause,
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    let scope = original_entry_zone_context(game, object, from, cause)?;
    process_etb_batch_proposal_with_scope(
        game,
        object,
        from,
        dm,
        initial_enters_with_counters,
        initial_enters_tapped,
        entering_controller,
        &std::collections::HashSet::new(),
        scope,
        &[],
        &[],
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
    initial_enters_tapped: bool,
    entering_controller: Option<PlayerId>,
    reserved_objects: &std::collections::HashSet<ObjectId>,
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    process_etb_with_event_and_dm_with_initial_counters_and_reservations(
        game,
        object,
        from,
        dm,
        initial_enters_with_counters,
        initial_enters_tapped,
        entering_controller,
        reserved_objects,
    )
}

/// Prepare an entry with its owning effect's scope. Distinct batch members
/// independently inherit the parent lineage, cause and temporary effects.
#[allow(clippy::too_many_arguments)]
pub(crate) fn process_etb_batch_proposal_with_scope(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    dm: &mut dyn DecisionMaker,
    counters: Vec<(CounterType, u32)>,
    tapped: bool,
    controller: Option<PlayerId>,
    reserved_objects: &std::collections::HashSet<ObjectId>,
    scope: ReplacementEventContext,
    additional_effects: &[ReplacementEffect],
    original_source_faces: &[crate::object::Object],
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    process_etb_batch_proposal_with_scope_and_draws(
        game,
        object,
        from,
        dm,
        counters,
        tapped,
        controller,
        reserved_objects,
        scope,
        additional_effects,
        original_source_faces,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_etb_batch_proposal_with_scope_and_draws(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    dm: &mut dyn DecisionMaker,
    counters: Vec<(CounterType, u32)>,
    tapped: bool,
    controller: Option<PlayerId>,
    reserved_objects: &std::collections::HashSet<ObjectId>,
    scope: ReplacementEventContext,
    additional_effects: &[ReplacementEffect],
    original_source_faces: &[crate::object::Object],
    draws: Option<&mut ZoneDrawContinuations>,
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    if dm.awaiting_choice() {
        return Ok(EtbEventResult {
            prevented: true,
            ..Default::default()
        });
    }
    let checkpoint = game.clone();
    let mut additional = additional_effects.to_vec();
    assign_ephemeral_effect_ids(&mut additional, (u64::MAX / 2).saturating_add(1024));
    let result = prepare_etb_replacements_inner(
        game,
        object,
        from,
        dm,
        counters,
        tapped,
        controller,
        reserved_objects,
        Some(scope),
        &additional,
        original_source_faces,
        draws,
    );
    if result.is_err() || dm.awaiting_choice() {
        *game = checkpoint;
    }
    if dm.awaiting_choice() {
        return result.map(|_| EtbEventResult {
            prevented: true,
            ..Default::default()
        });
    }
    result
}

// Every entry is also a zone-change proposal. Direct entry callers need the
// same original cause and snapshot as callers that arrive through zone processing.
fn original_entry_zone_context(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    cause: crate::events::cause::EventCause,
) -> Result<ReplacementEventContext, crate::effects::ExecutionError> {
    // Capture precedes provenance allocation and all entry mutations.
    let snapshot = game
        .object(object)
        .map(|card| {
            crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                card, game,
            )
        })
        .transpose()?;
    let provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::ZoneChange);
    let zone = crate::events::ZoneChangeEvent::with_cause(
        object,
        from,
        Zone::Battlefield,
        cause,
        snapshot,
    );
    let event = Event::new_with_provenance(zone.clone(), provenance);
    let mut context =
        ReplacementEventContext::new(game, event, &TraitEventProcessingState::default());
    context.zone_change_context = Some(zone);
    Ok(context)
}

fn process_etb_with_event_and_dm_with_initial_counters_and_reservations(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    dm: &mut dyn DecisionMaker,
    initial_enters_with_counters: Vec<(CounterType, u32)>,
    initial_enters_tapped: bool,
    entering_controller: Option<PlayerId>,
    batch_reserved_objects: &std::collections::HashSet<ObjectId>,
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    // Entry interactions can pay costs or create effects before a later
    // replacement asks another question. Replay starts from the entire entry
    // proposal, so suspended preparation must retain none of those mutations.
    let checkpoint = game.clone();
    let scope = original_entry_zone_context(
        game,
        object,
        from,
        crate::events::cause::EventCause::effect(),
    )?;
    let result = prepare_etb_replacements_inner(
        game,
        object,
        from,
        dm,
        initial_enters_with_counters,
        initial_enters_tapped,
        entering_controller,
        batch_reserved_objects,
        Some(scope),
        &[],
        &[],
        None,
    );
    if result.is_err() {
        *game = checkpoint;
        return result;
    }
    if dm.awaiting_choice() {
        *game = checkpoint;
        return Ok(EtbEventResult {
            prevented: true,
            ..Default::default()
        });
    }
    result
}

/// Continue a zone event at its entry boundary, preserving applications from
/// before the destination changed. The owning zone operation checkpoints its
/// earlier replacements; this boundary owns entry preparation itself.
pub(crate) fn process_etb_from_zone_change_context(
    game: &mut GameState,
    context: ReplacementEventContext,
    dm: &mut dyn DecisionMaker,
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    process_etb_from_zone_change_context_with_options(game, context, dm, Vec::new(), &[])
}

fn process_etb_from_zone_change_context_with_options(
    game: &mut GameState,
    context: ReplacementEventContext,
    dm: &mut dyn DecisionMaker,
    initial_counters: Vec<(CounterType, u32)>,
    additional_effects: &[ReplacementEffect],
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    process_etb_from_zone_change_context_with_options_and_draws(
        game,
        context,
        dm,
        initial_counters,
        additional_effects,
        None,
    )
}

fn process_etb_from_zone_change_context_with_options_and_draws(
    game: &mut GameState,
    context: ReplacementEventContext,
    dm: &mut dyn DecisionMaker,
    initial_counters: Vec<(CounterType, u32)>,
    additional_effects: &[ReplacementEffect],
    draws: Option<&mut ZoneDrawContinuations>,
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    if dm.awaiting_choice() {
        return Ok(EtbEventResult {
            prevented: true,
            ..Default::default()
        });
    }
    let change =
        crate::events::downcast_event::<crate::events::ZoneChangeEvent>(context.event.inner())
            .filter(|change| change.to == Zone::Battlefield && change.objects.len() == 1)
            .cloned()
            .ok_or_else(|| {
                crate::effects::ExecutionError::InternalError(
                    "entry continuation requires one unresolved battlefield zone change".into(),
                )
            })?;
    let checkpoint = game.clone();
    let result = prepare_etb_replacements_inner(
        game,
        change.objects[0],
        change.from,
        dm,
        initial_counters,
        false,
        None,
        &std::collections::HashSet::new(),
        Some(context),
        additional_effects,
        &[],
        draws,
    );
    if result.is_err() || dm.awaiting_choice() {
        *game = checkpoint;
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn execute_entry_replacement_payload(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
    effects: &[crate::effect::Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    additional: &[ReplacementEffect],
    reserved: &std::collections::HashSet<ObjectId>,
    draws: Option<&mut ZoneDrawContinuations>,
) -> Result<(), crate::effects::ExecutionError> {
    let mut ctx = crate::effects::ExecutionContext::new(source, controller, dm);
    ctx.replacement.additional_replacement_effects = additional.to_vec();
    ctx.replacement.entry_reserved_objects = reserved.clone();
    let snapshots = context
        .zone_change_context
        .as_ref()
        .and_then(|zone| zone.snapshot.clone())
        .into_iter()
        .collect::<Vec<_>>();
    let bindings = crate::effects::replacement::ReplacementProgramBindings {
        targets: None,
        object_tags: vec![
            ("it".into(), snapshots.clone()),
            ("__it__".into(), snapshots),
        ],
    };
    if let Some(draws) = draws {
        if let Some(prepared) =
            crate::effects::replacement::prepare_draw_continuation_with_bindings_and_outputs(
                game,
                &mut ctx,
                effects,
                source,
                controller,
                context,
                None,
                bindings.clone(),
            )?
        {
            draws.0.push(prepared);
            return Ok(());
        }
    }
    let mut outcome = crate::effects::replacement::execute_replacement_payload_with_snapshot(
        game,
        &mut ctx,
        effects,
        source,
        controller,
        context,
        bindings.targets,
        None,
        bindings.object_tags,
    )?;
    crate::effects::retain_unmatched_outcome_events(game, &mut outcome.events);
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
    Ok(())
}

fn restore_source_presentations_for_replaced_entry(
    game: &mut GameState,
    originals: &[crate::object::Object],
) {
    // Once entry is replaced, payload instructions operate on actual source
    // cards. Restore presentation before any movement or notification.
    for original in originals {
        if let Some(card) = game.object_mut(original.id)
            && card.zone == original.zone
            && card.stable_id == original.stable_id
        {
            card.restore_entry_presentation_from(original);
        }
    }
}

fn prepare_etb_replacements_inner(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    dm: &mut dyn DecisionMaker,
    initial_enters_with_counters: Vec<(CounterType, u32)>,
    initial_enters_tapped: bool,
    entering_controller: Option<PlayerId>,
    batch_reserved_objects: &std::collections::HashSet<ObjectId>,
    zone_context: Option<ReplacementEventContext>,
    zone_additional_effects: &[ReplacementEffect],
    original_source_faces: &[crate::object::Object],
    mut draws: Option<&mut ZoneDrawContinuations>,
) -> Result<EtbEventResult, crate::effects::ExecutionError> {
    use crate::ability::AbilityKind;
    use crate::decisions::{
        make_decision,
        specs::{ReplacementOption, ReplacementSpec},
    };
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};
    let mut deferred_programs = Vec::new();
    let mut completed_program_outputs = Vec::new();
    let outcome = (|| -> Result<EtbEventResult, crate::effects::ExecutionError> {
        game.update_replacement_effects()
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;

        // Carry the authored initial controller through replacement processing,
        // rather than reconstructing it from the original instruction at commit.
        let entering_controller = Some(match entering_controller {
            Some(controller) => controller,
            None => {
                let entrant = game
                    .object(object)
                    .ok_or(crate::effects::ExecutionError::ObjectNotFound(object))?;
                game.controller_of(entrant)
            }
        });

        // One-shot instructions establish the original event before replacements
        // (such as entering untapped) modify it.
        let enters_tapped = initial_enters_tapped;
        let mut enters_with_counters: Vec<(CounterType, u32)> = initial_enters_with_counters;

        // Gather ETB replacement effects from the object's abilities.
        let mut object_etb_effects: Vec<ReplacementEffect> = Vec::new();
        let mut copy_choice_effects: Vec<ReplacementEffect> = Vec::new();
        let mut reserved_objects = batch_reserved_objects.clone();
        if let Some(obj) = game.object(object) {
            let controller = entering_controller.unwrap_or_else(|| game.controller_of(obj));
            let view = crate::derived_view::DerivedGameView::new(game);
            let current = view
                .calculated_characteristics_arc(object)
                .ok_or(crate::effects::ExecutionError::ObjectNotFound(object))?;
            for (index, ability) in current.abilities.iter().enumerate() {
                let AbilityKind::Static(s) = &ability.kind else {
                    continue;
                };
                let origin = current
                    .abilities
                    .origin(index)
                    .expect("calculated ability and origin remain paired");
                let face = matches!(origin, crate::continuous::AbilityOrigin::Printed(_))
                    .then_some(obj.card)
                    .flatten();
                // Check for unified replacement effects
                if let Some(effect) = s.generate_replacement_effect(object, controller)
                    && effect
                        .matcher
                        .as_ref()
                        .is_some_and(|matcher| matcher.applies_from_entering_source())
                {
                    object_etb_effects.push(effect.with_ability_origin(origin.clone(), face, 0));
                }
                if let Some(spec) = s.enter_as_copy_as_enters()
                    && spec.affected_filter.is_none()
                {
                    push_enter_as_copy_effects_for_spec(
                        game,
                        object,
                        object,
                        controller,
                        spec,
                        &reserved_objects,
                        &mut copy_choice_effects,
                        origin,
                        s,
                    )?;
                }
            }
        }

        let own_copy_choice_count = copy_choice_effects.len();
        if let Some(sparse_candidates) = game.sparse_enter_as_copy_source_abilities() {
            for (source, origin, static_ability) in sparse_candidates.iter() {
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
                    origin,
                    static_ability,
                )?;
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
                let chars = view
                    .calculated_characteristics_arc(source)
                    .ok_or(crate::effects::ExecutionError::ObjectNotFound(source))?;
                for (index, ability) in chars.abilities.iter().enumerate() {
                    let AbilityKind::Static(static_ability) = &ability.kind else {
                        continue;
                    };
                    let origin = chars
                        .abilities
                        .origin(index)
                        .expect("calculated ability and origin remain paired");
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
                        origin,
                        static_ability,
                    )?;
                }
            }
        }
        // Keep ephemeral IDs far away from manager-issued IDs.
        const OBJECT_ETB_ID_BASE: u64 = u64::MAX - 1_000_000;
        const COPIED_OBJECT_ETB_ID_BASE: u64 = u64::MAX - 750_000;
        const COPY_CHOICE_ID_BASE: u64 = u64::MAX - 500_000;
        assign_ephemeral_effect_ids(&mut object_etb_effects, OBJECT_ETB_ID_BASE);
        assign_ephemeral_effect_ids(&mut copy_choice_effects, COPY_CHOICE_ID_BASE);

        let etb_event_provenance = match zone_context.as_ref() {
            Some(context) => context.event.provenance(),
            None => game
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::EnterBattlefield),
        };
        let mut current_event = Event::new_with_provenance(
            EnterBattlefieldEvent {
                object,
                completed_snapshot: None,
                emerge_sacrifice: None,
                from,
                enters_tapped,
                enters_with_counters,
                linked_exile_with_entering: Vec::new(),
                enters_as_copy_of: None,
                copy_followups: Vec::new(),
                copy_duration: None,
                copy_name_override: None,
                added_colors: crate::color::ColorSet::new(),
                added_card_types: Vec::new(),
                removes_other_card_types: false,
                added_supertypes: Vec::new(),
                removed_supertypes: Vec::new(),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
                set_base_power_toughness: None,
                controller_override: entering_controller,
                prepared_choices: None,
                pending_program: None,
                program_choices: Default::default(),
            },
            etb_event_provenance,
        );
        let mut state = TraitEventProcessingState {
            yield_after_etb_replacement: true,
            ..Default::default()
        };
        if let Some(context) = zone_context {
            state.applied_effects = context.applied_effects;
            state.applied_effect_keys = context.applied_effect_keys;
            state.zone_change_context = context.zone_change_context.or_else(|| {
                crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                    context.event.inner(),
                )
                .cloned()
            });
        }
        let mut paid_labels = Vec::new();
        let mut prepared_ability_ids = std::collections::HashMap::new();

        loop {
            if let Some(etb) = downcast_event::<EnterBattlefieldEvent>(current_event.inner()) {
                reserved_objects.extend(etb.linked_exile_with_entering.iter().copied());
                if etb.copy_duration.is_some()
                    && etb.enters_as_copy_of.is_some()
                    && etb.program_choices.entry_copy_registration.is_none()
                {
                    let mut reserved = etb.clone();
                    reserved.program_choices.entry_copy_registration =
                        Some(game.effect_store.continuous_effects.reserve_entry_effect());
                    current_event =
                        Event::new_with_provenance(reserved, current_event.provenance());
                }
            }
            if let Some(etb) = downcast_event::<EnterBattlefieldEvent>(current_event.inner())
                && let Some((program, controller)) = etb.pending_program.clone()
            {
                let mut resumed = etb.clone();
                resumed.pending_program = None;
                let Some(mut choices) = game.execute_entry_programs_with_reservations(
                    object,
                    controller,
                    vec![program],
                    Some(&resumed),
                    &reserved_objects,
                    dm,
                )?
                else {
                    return Ok(EtbEventResult {
                        prevented: true,
                        ..Default::default()
                    });
                };
                crate::effects::PublishedEffectOutputs::append_distinct(
                    &mut completed_program_outputs,
                    choices.as_enters_outputs.iter().cloned(),
                );
                crate::effects::PublishedEffectOutputs::append_distinct(
                    &mut resumed.program_choices.as_enters_outputs,
                    choices.as_enters_outputs.drain(..),
                );
                for (kind, count) in choices.as_enters_counters.drain(..) {
                    resumed = resumed.with_counters(kind, count);
                }
                resumed.program_choices.transfer_as_enters_source_links |=
                    choices.transfer_as_enters_source_links;
                resumed
                    .program_choices
                    .as_enters_continuous_effects
                    .extend(choices.as_enters_continuous_effects);
                for (tag, snapshots) in choices.as_enters_tagged_objects {
                    let retained = resumed
                        .program_choices
                        .as_enters_tagged_objects
                        .entry(tag)
                        .or_default();
                    for snapshot in snapshots {
                        if !retained
                            .iter()
                            .any(|existing| existing.stable_id == snapshot.stable_id)
                        {
                            retained.push(snapshot);
                        }
                    }
                }
                game.update_replacement_effects()
                    .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
                current_event = Event::new_with_provenance(resumed, current_event.provenance());
                continue;
            }
            let prepared_object_effects = prepared_object_etb_replacement_effects(
                game,
                &current_event,
                &mut prepared_ability_ids,
                &state,
                &reserved_objects,
            )?;
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
            // Compleated modifies the entire proposed loyalty placement at the
            // ordinary CR 616 priority, so the affected player can order it with
            // counter doublers. Re-evaluate the proposed characteristics after
            // copy/as-enters choices rather than retaining the original abilities.
            let compleated_life_payments = game.object(object).map_or(0, |obj| {
                obj.optional_costs_paid
                    .times_paid_label("CompleatedLifePaid")
            });
            let compleated_preview = if compleated_life_payments > 0 {
                downcast_event::<EnterBattlefieldEvent>(current_event.inner())
                    .map(|etb| etb.try_prospective_game_state(game))
                    .transpose()
                    .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
                    .flatten()
            } else {
                None
            };
            let compleated_effect = compleated_preview.and_then(|prospective| {
                let proposed = prospective.object(object)?;
                let has_compleated = prospective.current_abilities(object).is_some_and(|abilities| {
                    abilities.iter().any(|ability| matches!(
                        &ability.kind,
                        crate::ability::AbilityKind::Static(marker)
                            if marker.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
                                && marker.display().eq_ignore_ascii_case("compleated")
                    ))
                });
                if !has_compleated {
                    return None;
                }
                let mut effect = ReplacementEffect::with_matcher(
                    object, prospective.controller_of(proposed),
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    crate::replacement::ReplacementAction::AddCountersToPlacement {
                        counter_type: Some(CounterType::Loyalty),
                        additional: -i64::from(compleated_life_payments) * 2,
                    },
                );
                effect.id = ReplacementEffectId(u64::MAX - 1_250_000);
                Some(effect)
            });
            let current_additional_effects: Vec<ReplacementEffect> = copy_choice_effects
                .iter()
                .enumerate()
                .filter(|(index, effect)| {
                    !state.was_applied_effect(effect)
                        && (*index >= own_copy_choice_count || prepared_object_effects.is_none())
                })
                .map(|(_, effect)| effect)
                .chain(object_etb_effects.iter().filter(|_| {
                    original_object_effects_still_apply && prepared_object_effects.is_none()
                }))
                .chain(copied_object_etb_effects.iter().filter(|effect| {
                    prepared_object_effects.is_none() && !state.was_applied(effect.id)
                }))
                .chain(prepared_object_effects.iter().flatten())
                .chain(compleated_effect.iter())
                .chain(zone_additional_effects.iter())
                .cloned()
                .collect();
            let result = process_event_direct(
                game,
                current_event.clone(),
                &mut state,
                &current_additional_effects,
                None,
            )?;

            let (result, programs) = result.into_expansion();
            deferred_programs.extend(programs);
            match result {
                TraitEventResult::Expanded { .. } => {
                    return Err(crate::effects::ExecutionError::InternalError(
                        "entry expansion did not flatten to an original result".into(),
                    ));
                }
                TraitEventResult::Prevented => {
                    return Ok(EtbEventResult {
                        prevented: true,
                        ..Default::default()
                    });
                }
                TraitEventResult::Modified(e) => {
                    current_event = e;
                    continue;
                }
                TraitEventResult::Proceed(e) => {
                    if let Some(etb) = downcast_event::<EnterBattlefieldEvent>(e.inner()) {
                        let event_result = EtbEventResult {
                            additional_programs: Vec::new(),
                            completed_program_outputs: Vec::new(),
                            replaced: false,
                            replacement_context: Some(Box::new(ReplacementEventContext::new(
                                &game,
                                e.clone(),
                                &state,
                            ))),
                            enters_tapped: etb.enters_tapped,
                            enters_with_counters: etb.enters_with_counters.clone(),
                            linked_exile_with_entering: etb.linked_exile_with_entering.clone(),
                            prevented: false,
                            new_destination: None,
                            enters_as_copy_of: etb.enters_as_copy_of,
                            copy_followups: etb.copy_followups.clone(),
                            copy_duration: etb.copy_duration.clone(),
                            copy_name_override: etb.copy_name_override.clone(),
                            added_colors: etb.added_colors,
                            added_card_types: etb.added_card_types.clone(),
                            removes_other_card_types: etb.removes_other_card_types,
                            added_supertypes: etb.added_supertypes.clone(),
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
                            let prospective = etb
                                .try_prospective_game_state(game)
                                .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
                                .ok_or(crate::effects::ExecutionError::ObjectNotFound(object))?;
                            let abilities = prospective
                                .calculated_characteristics(object)
                                .ok_or(crate::effects::ExecutionError::ObjectNotFound(object))?
                                .abilities
                                .iter()
                                .cloned()
                                .collect();
                            let Some(prepared) = game.prepare_etb_entry_after_programs(
                                object,
                                event_result,
                                etb.controller_override,
                                dm,
                                Some((etb.program_choices.clone(), abilities)),
                            )?
                            else {
                                return Ok(EtbEventResult {
                                    prevented: true,
                                    ..Default::default()
                                });
                            };
                            let mut prepared_event = etb.clone();
                            let mut choices = prepared.choices;
                            crate::effects::PublishedEffectOutputs::append_distinct(
                                &mut completed_program_outputs,
                                choices.as_enters_outputs.iter().cloned(),
                            );
                            prepared_event
                                .enters_with_counters
                                .append(&mut choices.as_enters_counters);
                            let mut combined: Vec<(CounterType, u32)> = Vec::new();
                            for (counter, count) in prepared_event.enters_with_counters.drain(..) {
                                if let Some((_, total)) =
                                    combined.iter_mut().find(|(kind, _)| *kind == counter)
                                {
                                    *total = total.saturating_add(count);
                                } else {
                                    combined.push((counter, count));
                                }
                            }
                            prepared_event.enters_with_counters = combined;
                            let entry_program_ran = choices.transfer_as_enters_source_links;
                            prepared_event.prepared_choices = Some(choices);
                            // Entry programs can remove or exchange the abilities
                            // of existing permanents. Refresh the registered set
                            // before reconsidering the changed event; stable
                            // application keys retain the once-per-event history.
                            if entry_program_ran {
                                game.update_replacement_effects()
                                    .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
                            }
                            current_event =
                                Event::new_with_provenance(prepared_event, e.provenance());
                            continue;
                        }
                        // Entry counters are counter placement (CR 122.6), so
                        // prohibitions apply to them as well. Use the completed
                        // prospective entry: copy/control/choice modifications and
                        // the entrant's own static abilities must be accounted for
                        // before the batch commits any permanent.
                        let mut event_result = event_result;
                        if !event_result.enters_with_counters.is_empty() {
                            let prospective = etb
                                .try_prospective_game_state(game)
                                .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
                                .ok_or(crate::effects::ExecutionError::ObjectNotFound(object))?;
                            event_result
                                .enters_with_counters
                                .retain(|(counter_type, _)| {
                                    prospective.can_have_counter_type_placed(object, *counter_type)
                                });
                        }
                        return Ok(event_result);
                    }
                    if let Some(zone_change) = downcast_event::<ZoneChangeEvent>(e.inner()) {
                        return Ok(EtbEventResult {
                            replacement_context: Some(Box::new(ReplacementEventContext::new(
                                &game,
                                e.clone(),
                                &state,
                            ))),
                            prevented: zone_change.to != Zone::Battlefield,
                            new_destination: if zone_change.to != Zone::Battlefield {
                                Some(zone_change.to)
                            } else {
                                None
                            },
                            interactive_replacement: None,
                            ..Default::default()
                        });
                    }
                    return Ok(EtbEventResult::default());
                }
                TraitEventResult::Replaced {
                    effects,
                    effect_id,
                    source: replacement_source,
                    controller: replacement_controller,
                    context: replacement_context,
                    ..
                } => {
                    restore_source_presentations_for_replaced_entry(game, original_source_faces);
                    if game.object(object).is_some() {
                        game.effect_store
                            .replacement_effects
                            .mark_effect_used(effect_id);
                        execute_entry_replacement_payload(
                            game,
                            dm,
                            &effects,
                            replacement_source,
                            replacement_controller,
                            &replacement_context,
                            &current_additional_effects,
                            &reserved_objects,
                            draws.as_deref_mut(),
                        )?;
                    }
                    return Ok(EtbEventResult {
                        prevented: true,
                        replaced: true,
                        replacement_context: Some(replacement_context),
                        ..Default::default()
                    });
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
                    if dm.awaiting_choice() {
                        return Ok(EtbEventResult {
                            prevented: true,
                            ..Default::default()
                        });
                    }
                    let chosen_id = match chosen_index.as_slice() {
                        [index] => applicable_effects.get(*index).copied(),
                        _ => None,
                    }
                    .ok_or_else(|| {
                        crate::effects::ExecutionError::InternalError(
                            "entry replacement choice must name exactly one offered effect".into(),
                        )
                    })?;
                    let Some(chosen_effect) =
                        find_effect_for_choice(game, &current_additional_effects, chosen_id)
                    else {
                        state.mark_applied(chosen_id);
                        current_event = *event;
                        continue;
                    };

                    mark_applied_replacement_choice(&mut state, &chosen_effect);
                    let replacement_context =
                        ReplacementEventContext::new(&game, (*event).clone(), &state);
                    let apply_result = apply_trait_replacement_retaining_damage_branches(
                        game,
                        *event,
                        &chosen_effect,
                        &mut state,
                    )?;
                    consume_one_shot_if_applied(game, chosen_id, &apply_result);
                    match apply_result {
                        TraitApplyResult::Modified(modified_event) => {
                            current_event = modified_event
                        }
                        TraitApplyResult::Prevented => {
                            return Ok(EtbEventResult {
                                prevented: true,
                                ..Default::default()
                            });
                        }
                        TraitApplyResult::Replaced(effects) => {
                            restore_source_presentations_for_replaced_entry(
                                game,
                                original_source_faces,
                            );
                            if game.object(object).is_some() {
                                game.effect_store
                                    .replacement_effects
                                    .mark_effect_used(chosen_id);
                                execute_entry_replacement_payload(
                                    game,
                                    dm,
                                    &effects,
                                    chosen_effect.source,
                                    chosen_effect.controller,
                                    &replacement_context,
                                    &current_additional_effects,
                                    &reserved_objects,
                                    draws.as_deref_mut(),
                                )?;
                            }
                            return Ok(EtbEventResult {
                                prevented: true,
                                replaced: true,
                                replacement_context: Some(Box::new(replacement_context)),
                                ..Default::default()
                            });
                        }
                        TraitApplyResult::Unchanged(unchanged_event) => {
                            current_event = unchanged_event
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
                            let life_cost = match &chosen_effect.replacement {
                                ReplacementAction::InteractivePayLifeOrEnterTapped {
                                    life_cost,
                                } => Some(*life_cost),
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
                                crate::decisions::context::DecisionContext::SelectObjects(
                                    mut ctx,
                                ) => {
                                    ctx.candidates.retain(|candidate| {
                                        !reserved_objects.contains(&candidate.id)
                                    });
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
                            if dm.awaiting_choice() {
                                return Ok(EtbEventResult {
                                    prevented: true,
                                    ..Default::default()
                                });
                            }
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
                                if dm.awaiting_choice() {
                                    return Ok(EtbEventResult {
                                        prevented: true,
                                        ..Default::default()
                                    });
                                }
                                game.effect_store
                                    .replacement_effects
                                    .mark_effect_used(effect_id);
                                continue;
                            }
                            if let ReplacementAction::EnterUnderChosenControl { players } =
                                &chosen_effect.replacement
                            {
                                let Some(modified) = apply_entry_controller_choice(
                                    game,
                                    &current_event,
                                    &response,
                                    chosen_effect.controller,
                                    players,
                                ) else {
                                    return Ok(EtbEventResult {
                                        prevented: true,
                                        ..Default::default()
                                    });
                                };
                                current_event = modified;
                                if dm.awaiting_choice() {
                                    return Ok(EtbEventResult {
                                        prevented: true,
                                        ..Default::default()
                                    });
                                }
                                game.effect_store
                                    .replacement_effects
                                    .mark_effect_used(effect_id);
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
                                if dm.awaiting_choice() {
                                    return Ok(EtbEventResult {
                                        prevented: true,
                                        ..Default::default()
                                    });
                                }
                                game.effect_store
                                    .replacement_effects
                                    .mark_effect_used(effect_id);
                                continue;
                            }
                            let mut interaction_scope =
                                crate::effects::ReplacementExecutionContext::default();
                            interaction_scope.additional_replacement_effects =
                                current_additional_effects.clone();
                            interaction_scope.suppressed_replacement_effects =
                                state.applied_effects.clone();
                            interaction_scope.suppressed_replacement_effect_keys =
                                state.applied_effect_keys.clone();
                            let interaction_snapshot = game.object(object_id).map(|object| crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)).transpose()?;
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
                                &interaction_scope,
                                interaction_snapshot.as_ref(),
                            )?;
                            if dm.awaiting_choice() {
                                return Ok(EtbEventResult {
                                    prevented: true,
                                    ..Default::default()
                                });
                            }
                            game.effect_store
                                .replacement_effects
                                .mark_effect_used(effect_id);
                            if !interactive_result.enters {
                                return Ok(EtbEventResult {
                                    prevented: true,
                                    new_destination: interactive_result.redirect_zone,
                                    ..Default::default()
                                });
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
                    ..
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
                    if dm.awaiting_choice() {
                        return Ok(EtbEventResult {
                            prevented: true,
                            ..Default::default()
                        });
                    }
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
                        if dm.awaiting_choice() {
                            return Ok(EtbEventResult {
                                prevented: true,
                                ..Default::default()
                            });
                        }
                        game.effect_store
                            .replacement_effects
                            .mark_effect_used(effect_id);
                        continue;
                    }
                    if let Some(effect) =
                        find_effect_for_choice(game, &current_additional_effects, effect_id)
                        && let ReplacementAction::EnterUnderChosenControl { players } =
                            &effect.replacement
                    {
                        let Some(modified) = apply_entry_controller_choice(
                            game,
                            &event,
                            &response,
                            effect.controller,
                            players,
                        ) else {
                            return Ok(EtbEventResult {
                                prevented: true,
                                ..Default::default()
                            });
                        };
                        current_event = modified;
                        if dm.awaiting_choice() {
                            return Ok(EtbEventResult {
                                prevented: true,
                                ..Default::default()
                            });
                        }
                        game.effect_store
                            .replacement_effects
                            .mark_effect_used(effect_id);
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
                        if dm.awaiting_choice() {
                            return Ok(EtbEventResult {
                                prevented: true,
                                ..Default::default()
                            });
                        }
                        game.effect_store
                            .replacement_effects
                            .mark_effect_used(effect_id);
                        continue;
                    }
                    let mut interaction_scope =
                        crate::effects::ReplacementExecutionContext::default();
                    interaction_scope.additional_replacement_effects =
                        current_additional_effects.clone();
                    interaction_scope.suppressed_replacement_effects =
                        state.applied_effects.clone();
                    interaction_scope.suppressed_replacement_effect_keys =
                        state.applied_effect_keys.clone();
                    let interaction_snapshot = game.object(object_id).map(|object| crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)).transpose()?;
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
                        &interaction_scope,
                        interaction_snapshot.as_ref(),
                    )?;
                    if dm.awaiting_choice() {
                        return Ok(EtbEventResult {
                            prevented: true,
                            ..Default::default()
                        });
                    }
                    game.effect_store
                        .replacement_effects
                        .mark_effect_used(effect_id);
                    if !interactive_result.enters {
                        return Ok(EtbEventResult {
                            prevented: true,
                            new_destination: interactive_result.redirect_zone,
                            ..Default::default()
                        });
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
    })();
    outcome.map(|mut result| {
        // Suspended preparation is discarded by its owner. Replay must not
        // expose or execute a prefix from this attempt.
        if !dm.awaiting_choice() {
            crate::effects::PublishedEffectOutputs::append_distinct(
                &mut completed_program_outputs,
                result.completed_program_outputs.drain(..),
            );
            result.completed_program_outputs = completed_program_outputs;
            deferred_programs.append(&mut result.additional_programs);
            result.additional_programs = deferred_programs;
        } else {
            result.additional_programs.clear();
            result.completed_program_outputs.clear();
        }
        result
    })
}

/// Complete zone-change replacement result, including the evolving event,
/// replacement source/context/history, added actions and pending interaction.
/// The public full API prepares a proposal; its caller owns commit/execution.
pub type ZoneChangeResult = TraitEventResult;

/// Evaluate a zone-change proposal without collapsing it to a destination.
/// Consumers must retain every action and continuation in the returned result.
pub fn process_zone_change_full(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
) -> Result<ZoneChangeResult, crate::effects::ExecutionError> {
    game.update_replacement_effects()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    let snapshot = game
        .object(object)
        .map(|object| {
            crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .transpose()?;
    process_trait_event(game, Event::zone_change(object, from, to, cause, snapshot))
}

/// Complete draw replacement result. A modified event retains both its
/// recipient and count, and an unresolved interaction is never permission to draw.
pub type DrawResult = TraitEventResult;

/// Evaluate a draw proposal without discarding event fields or added actions.
/// This API does not itself draw cards or execute replacement programs.
pub fn process_draw_full(
    game: &mut GameState,
    player: PlayerId,
    count: u32,
    is_first_this_turn: bool,
) -> Result<DrawResult, crate::effects::ExecutionError> {
    if !game.can_draw(player) {
        return Ok(TraitEventResult::Prevented);
    }
    process_trait_event(game, Event::draw(player, count, is_first_this_turn))
}

/// Continue a captured replacement choice without losing earlier additions.
/// `additional_effects` must be the exact assigned ephemeral snapshots used to
/// produce the prompt. A phase driver may supply its saved state to retain
/// entry deferral, zone metadata and split-damage branches. Root callers use
/// None and an empty ephemeral slice. The pending result owns added programs;
/// those replace any duplicate program list in the supplied state.
#[allow(clippy::too_many_arguments)]
pub fn continue_replacement_choice_with_scope(
    game: &mut GameState,
    pending: TraitEventResult,
    chosen_effect_id: ReplacementEffectId,
    prior_state: Option<TraitEventProcessingState>,
    additional_effects: &[ReplacementEffect],
    event_source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    let operation_checkpoint = game.clone();
    let operation_result = (|| -> Result<TraitEventResult, crate::effects::ExecutionError> {
        let (original, programs) = pending.into_expansion();
        let TraitEventResult::NeedsChoice {
            applicable_effects,
            event,
            applied_effects,
            applied_effect_keys,
            zone_change_context,
            ..
        } = original
        else {
            return Err(crate::effects::ExecutionError::InternalError(
                "replacement continuation requires a captured choice".into(),
            ));
        };
        if !applicable_effects.contains(&chosen_effect_id) {
            return Err(crate::effects::ExecutionError::InternalError(
                "replacement continuation selected an effect outside its captured choices".into(),
            ));
        }
        let mut state = prior_state.unwrap_or_default();
        state.zone_change_context = zone_change_context.or(state.zone_change_context);
        state.applied_effects.extend(applied_effects);
        state.applied_effect_keys.extend(applied_effect_keys);
        state.additional_programs = programs;
        let Some(effect) = find_effect_for_choice(game, additional_effects, chosen_effect_id)
        else {
            state.mark_applied(chosen_effect_id);
            return process_event_direct(
                game,
                *event,
                &mut state,
                additional_effects,
                event_source_snapshot,
            );
        };
        let apply_result = apply_trait_replacement_retaining_damage_branches(
            game,
            (*event).clone(),
            &effect,
            &mut state,
        )?;
        mark_applied_replacement_choice(&mut state, &effect);
        consume_one_shot_if_applied(game, chosen_effect_id, &apply_result);
        let resolved = match apply_result {
            TraitApplyResult::Modified(event) | TraitApplyResult::Unchanged(event) => {
                process_event_direct_inner(
                    game,
                    event,
                    &mut state,
                    additional_effects,
                    event_source_snapshot,
                )?
            }
            TraitApplyResult::Prevented => TraitEventResult::Prevented,
            TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced {
                context: Box::new(ReplacementEventContext::new(
                    &game,
                    (*event).clone(),
                    &state,
                )),
                effects,
                effect_id: chosen_effect_id,
                replacement: effect.replacement.clone(),
                source: effect.source,
                controller: effect.controller,
            },
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
                event,
                filter,
                sacrifice_count,
                destinations,
                life_cost: match &effect.replacement {
                    ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } => {
                        Some(*life_cost)
                    }
                    _ => None,
                },
                applied_effects: state.applied_effects.clone(),
                applied_effect_keys: state.applied_effect_keys.clone(),
                zone_change_context: state.zone_change_context.clone(),
            },
        };
        Ok(retain_additional_programs(resolved, &mut state))
    })();
    if operation_result.is_err() {
        game.restore_execution_checkpoint(operation_checkpoint, false);
    }
    operation_result
}

/// Process an event with a chosen replacement effect, using the new Event type.
///
/// When a player chooses which replacement effect to apply (per Rule 616.1e),
/// this function applies that effect and continues processing.
pub fn process_event_with_chosen_replacement_trait(
    game: &mut GameState,
    event: Event,
    chosen_effect_id: ReplacementEffectId,
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
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
) -> Result<TraitEventResult, crate::effects::ExecutionError> {
    let operation_checkpoint = game.clone();
    let operation_result = (|| -> Result<TraitEventResult, crate::effects::ExecutionError> {
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
        let apply_result = apply_trait_replacement_retaining_damage_branches(
            game,
            event.clone(),
            &effect,
            &mut state,
        )?;
        consume_one_shot_if_applied(game, chosen_effect_id, &apply_result);

        mark_applied_replacement_choice(&mut state, &effect);

        Ok(match apply_result {
            TraitApplyResult::Modified(modified) => {
                // Continue processing with the modified event
                process_event_direct(game, modified, &mut state, &[], None)?
            }
            TraitApplyResult::Prevented => TraitEventResult::Prevented,
            TraitApplyResult::Replaced(effects) => TraitEventResult::Replaced {
                context: Box::new(ReplacementEventContext::new(&game, event.clone(), &state)),
                effects,
                effect_id: chosen_effect_id,
                replacement: effect.replacement.clone(),
                source: effect.source,
                controller: effect.controller,
            },
            TraitApplyResult::Unchanged(unchanged) => {
                // Effect didn't change anything - continue with original event
                process_event_direct(game, unchanged, &mut state, &[], None)?
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
                applied_effects: state.applied_effects.clone(),
                applied_effect_keys: state.applied_effect_keys.clone(),
                zone_change_context: state.zone_change_context.clone(),
            },
        })
    })();
    if operation_result.is_err() {
        game.restore_execution_checkpoint(operation_checkpoint, false);
    }
    operation_result
}

#[cfg(test)]
mod life_modification_tests;

#[cfg(test)]
mod entry_failure_tests;

#[cfg(test)]
mod next_damage_occurrence_tests;

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
                removes_other_card_types: false,
                added_supertypes: Vec::new(),
                removed_supertypes: Vec::new(),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
                set_base_power_toughness: None,
                additional_counters: Vec::new(),
                additional_x_counters: Vec::new(),
                keep_other_source_abilities: false,
                additional_counters_source_filter: None,
                added_abilities_source_filter: None,
                set_base_power_toughness_from_self: false,
                copy_followups: Vec::new(),
                conditional_additional_counters: Vec::new(),
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

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");

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

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");

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
                        removes_other_card_types: false,
                        added_supertypes: Vec::new(),
                        removed_supertypes: Vec::new(),
                        added_subtypes: Vec::new(),
                        added_abilities: Vec::new(),
                        set_base_power_toughness: None,
                        additional_counters: Vec::new(),
                        additional_x_counters: Vec::new(),
                        keep_other_source_abilities: false,
                        additional_counters_source_filter: None,
                        added_abilities_source_filter: None,
                        set_base_power_toughness_from_self: false,
                        copy_followups: Vec::new(),
                        conditional_additional_counters: Vec::new(),
                    },
                    "This permanent enters as a copy of a creature.".to_string(),
                ),
            ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");

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

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");

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

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");

        assert!(
            result.enters_tapped,
            "continuous effects already present must apply to the provisional battlefield object"
        );
    }

    #[test]
    fn compiled_entry_controller_model_materializes_typed_replacement() {
        let model = crate::static_abilities::CompiledStaticAbility::enters_under_chosen_control(
            crate::target::PlayerFilter::Opponent,
        );
        let ability = crate::static_abilities::StaticAbility::from_model(model.clone());
        assert_eq!(ability.compiled_model(), Some(&model));
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entering =
            create_creature_in_zone(&mut game, "Model entry fixture", alice, Zone::Hand, 4, 4);
        let effect = ability
            .generate_replacement_effect(entering, alice)
            .expect("typed model must generate replacement");
        assert!(matches!(
            effect.replacement,
            ReplacementAction::EnterUnderChosenControl {
                players: crate::target::PlayerFilter::Opponent
            }
        ));
        assert_eq!(
            effect.priority_override,
            Some(crate::events::ReplacementPriority::ControlChanging)
        );
        game.effect_store.replacement_effects.add_effect(effect);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");
        assert_eq!(result.controller_override, Some(PlayerId::from_index(1)));
        assert!(!result.prevented);
    }

    #[test]
    fn chosen_entry_controller_applies_before_controller_relative_replacements() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let entering = create_creature_in_zone(
            &mut game,
            "Chosen controller entrant",
            alice,
            Zone::Hand,
            4,
            4,
        );
        game.effect_store.replacement_effects.add_effect(
            ReplacementEffect::with_matcher(
                entering,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::EnterUnderChosenControl {
                    players: crate::target::PlayerFilter::Opponent,
                },
            )
            .with_priority_override(crate::events::ReplacementPriority::ControlChanging),
        );
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::enters_tapped(
                entering,
                bob,
                ObjectFilter::creature().you_control(),
            ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");
        assert!(!result.prevented);
        assert_eq!(result.controller_override, Some(bob));
        assert!(
            result.enters_tapped,
            "later replacements must use selected entry controller"
        );
    }

    #[test]
    fn chosen_entry_controller_preserves_multiplayer_choice_and_rejects_ineligible_player() {
        struct Choose {
            selected: usize,
            calls: usize,
        }
        impl crate::DecisionMaker for Choose {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, PlayerId::from_index(0));
                assert_eq!(
                    ctx.options.iter().map(|o| o.index).collect::<Vec<_>>(),
                    vec![1, 2]
                );
                self.calls += 1;
                if self.selected == 4 {
                    vec![1, 2]
                } else {
                    vec![self.selected]
                }
            }
        }
        for selected in [1, 2, 0, 3, 4] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Cara".into(),
                    "Departed".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            game.player_mut(PlayerId::from_index(3))
                .unwrap()
                .has_left_game = true;
            let entering = create_creature_in_zone(
                &mut game,
                "Chosen controller entrant",
                alice,
                Zone::Hand,
                4,
                4,
            );
            game.effect_store.replacement_effects.add_effect(
                ReplacementEffect::with_matcher(
                    entering,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::EnterUnderChosenControl {
                        players: crate::target::PlayerFilter::Opponent,
                    },
                )
                .with_priority_override(crate::events::ReplacementPriority::ControlChanging),
            );
            let mut dm = Choose { selected, calls: 0 };
            let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
                .expect("replacement operation must execute successfully in this scenario");
            assert_eq!(dm.calls, 1);
            if selected == 1 || selected == 2 {
                assert!(!result.prevented);
                assert_eq!(
                    result.controller_override,
                    Some(PlayerId::from_index(selected as u8))
                );
            } else {
                assert!(
                    result.prevented,
                    "invalid choice must not select an arbitrary opponent"
                );
                assert_eq!(result.controller_override, None);
            }
        }
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

        let result = process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");

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
            .expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions()
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
    fn copy_replacement_identity_distinct_temporary_occurrences_survive_sparse_discovery() {
        for cloned_payload in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let source = create_creature(&mut game, "Repeated Copy Source", alice);
            let first = external_enter_as_copy_ability();
            let second = if cloned_payload {
                first.clone()
            } else {
                external_enter_as_copy_ability()
            };
            for ability in [first, second] {
                game.object_mut(source)
                    .unwrap()
                    .temporary_static_ability_grants
                    .push(crate::object::TemporaryStaticAbilityGrant {
                        ability: ability.id(),
                        ability_payload: Some(ability),
                        expires_end_of_turn: Some(4),
                    });
            }
            let store = &game.object(source).unwrap().temporary_static_ability_grants;
            assert_ne!(store.origin(0), store.origin(1));
            assert_eq!(
                game.sparse_enter_as_copy_source_abilities().unwrap().len(),
                2,
                "independent registrations survive payload equality, cloned={cloned_payload}"
            );
        }
    }

    #[test]
    fn copy_replacement_identity_declining_one_external_ability_preserves_the_other() {
        struct ChooseDeclineThenCopy {
            players: Vec<PlayerId>,
            offered: Vec<usize>,
        }
        impl crate::decision::DecisionMaker for ChooseDeclineThenCopy {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                self.players.push(ctx.player);
                self.offered.push(ctx.options.len());
                let copy = self.offered.len() > 1;
                let option = ctx
                    .options
                    .iter()
                    .find(|option| {
                        option.legal && option.description.starts_with("Enter as a copy of") == copy
                    })
                    .expect("remaining copy ability offers its own alternative");
                vec![option.index]
            }
        }
        for layered in [false, true] {
            for same_host in [false, true] {
                let mut game = crate::tests::test_helpers::setup_two_player_game();
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let first = create_creature_in_zone(
                    &mut game,
                    "First Copy Source",
                    bob,
                    Zone::Battlefield,
                    4,
                    4,
                );
                let second = if same_host {
                    first
                } else {
                    create_creature_in_zone(
                        &mut game,
                        "Second Copy Source",
                        bob,
                        Zone::Battlefield,
                        6,
                        6,
                    )
                };
                let sources = [first, second];
                for source in sources {
                    let mut ability = external_enter_as_copy_ability();
                    let mut spec = ability.enter_as_copy_as_enters().unwrap().clone();
                    spec.may = true;
                    ability = StaticAbility::with_enter_as_copy_as_enters(
                        spec,
                        "Optional external copy".into(),
                    );
                    if layered {
                        game.effect_store.continuous_effects.add_effect(
                            crate::continuous::ContinuousEffect::new(
                                source,
                                bob,
                                crate::continuous::EffectTarget::Specific(source),
                                crate::continuous::Modification::AddAbility(ability),
                            ),
                        );
                    } else {
                        game.object_mut(source)
                            .unwrap()
                            .temporary_static_ability_grants
                            .push(crate::object::TemporaryStaticAbilityGrant {
                                ability: ability.id(),
                                ability_payload: Some(ability),
                                expires_end_of_turn: Some(4),
                            });
                    }
                }
                let entering =
                    create_creature_in_zone(&mut game, "Entering Bear", alice, Zone::Hand, 2, 2);
                let mut dm = ChooseDeclineThenCopy {
                    players: Vec::new(),
                    offered: Vec::new(),
                };
                let result =
                    process_etb_with_event_and_dm(&mut game, entering, Zone::Hand, &mut dm)
                        .expect("independent optional copy effects execute");
                assert_eq!(
                    dm.offered,
                    vec![4, 2],
                    "declining one occurrence leaves only the other alternatives, layered={layered}"
                );
                assert_eq!(
                    dm.players,
                    vec![alice, alice],
                    "the affected controller chooses both abilities"
                );
                assert_eq!(result.enters_as_copy_of, Some(sources[1]));
            }
        }
    }

    #[test]
    fn retained_temporary_registration_copy_entry_ignores_expired_external_grant() {
        for expires_end_of_turn in [4, 2] {
            for warm in [false, true] {
                let mut game = crate::tests::test_helpers::setup_two_player_game();
                let alice = PlayerId::from_index(0);
                let source = create_creature_in_zone(
                    &mut game,
                    "Temporary Copy Source",
                    alice,
                    Zone::Battlefield,
                    6,
                    6,
                );
                let ability = external_enter_as_copy_ability();
                game.object_mut(source)
                    .unwrap()
                    .temporary_static_ability_grants
                    .push(crate::object::TemporaryStaticAbilityGrant {
                        ability: ability.id(),
                        ability_payload: Some(ability),
                        expires_end_of_turn: Some(expires_end_of_turn),
                    });
                if warm {
                    assert_eq!(
                        game.sparse_enter_as_copy_source_abilities().unwrap().len(),
                        1
                    );
                }
                game.next_turn();
                game.next_turn();
                assert_eq!(game.turn.turn_number, 3);
                assert_eq!(
                    game.object(source)
                        .unwrap()
                        .temporary_static_ability_grants
                        .len(),
                    1
                );
                let entering =
                    create_creature_in_zone(&mut game, "Entering Bear", alice, Zone::Hand, 2, 2);
                let result = game
                    .move_object_with_etb_processing(entering, Zone::Battlefield)
                    .expect("temporary copy entry executes")
                    .assert_completed_without_additions()
                    .expect("entry completes");
                let entered = game.object(result.new_id).unwrap();
                let live = expires_end_of_turn >= game.turn.turn_number;
                assert_eq!(
                    entered.name,
                    if live {
                        "Temporary Copy Source"
                    } else {
                        "Entering Bear"
                    },
                    "actual copy entry respects grant lifetime, warm={warm}"
                );
                assert_eq!(
                    entered.base_power,
                    Some(crate::card::PtValue::Fixed(if live { 6 } else { 2 }))
                );
                assert_eq!(
                    game.sparse_enter_as_copy_source_abilities().unwrap().len(),
                    usize::from(live)
                );
            }
        }
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
        )
        .expect("replacement operation must execute successfully in this scenario");

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
            .expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions()
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
            .expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions()
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
        )
        .expect("finite replacement fixture evaluates successfully");

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
        )
        .expect("finite replacement fixture evaluates successfully");
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
        )
        .expect("finite replacement fixture evaluates successfully");
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
        )
        .expect("damage test proposal must process successfully");
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
        game.update_replacement_effects().unwrap();

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
        )
        .expect("finite replacement fixture evaluates successfully");
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
        )
        .unwrap();
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
        )
        .unwrap();
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
        )
        .unwrap();
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
            process_simultaneous_damage_assignments_with_event_with_dm(&mut game, &events, &mut dm)
                .unwrap();

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
        )
        .unwrap();

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

        let outcome = outcome
            .expect("zone preparation must succeed")
            .assert_without_additions();
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
    fn replaced_event_context_retains_prior_applications_after_one_shot_removal() {
        for choose_order in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let source = create_creature(&mut game, "Replacement source", alice);
            let mut doubler = ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                ReplacementAction::Double,
            );
            if !choose_order {
                doubler = doubler
                    .with_priority_override(crate::events::ReplacementPriority::SelfReplacement);
            }
            let first = game
                .effect_store
                .replacement_effects
                .add_resolution_effect(doubler);
            let second = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::life::matchers::WouldGainLifeMatcher::you(),
                    ReplacementAction::Instead(Vec::new()),
                ),
            );
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let result = process_with_dm(&mut game, Event::life_gain(alice, 3), &mut dm)
                .expect("finite replacement fixture evaluates successfully");
            let TraitEventResult::Replaced { context, .. } = result else {
                panic!("expected replacement actions");
            };
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(second)
                    .is_none()
            );
            assert_eq!(context.applied_effects.len(), 2);
            assert!(context.applied_effects.contains(&first));
            assert!(context.applied_effects.contains(&second));
            assert_eq!(context.applied_effect_keys.len(), 2);
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
            context.apply_to(&mut ctx);
            assert_eq!(
                ctx.replacement.suppressed_replacement_effect_keys,
                context.applied_effect_keys
            );
            assert_eq!(
                ctx.triggering_event
                    .as_ref()
                    .unwrap()
                    .downcast::<crate::events::LifeGainEvent>()
                    .unwrap()
                    .amount,
                6
            );
        }
    }

    #[test]
    fn damage_instead_payload_targets_the_redirected_recipient() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Replacement source", alice);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Redirect {
                    target: crate::replacement::RedirectTarget::ToPlayer(bob),
                    which: crate::replacement::RedirectWhich::First,
                },
            )
            .with_priority_override(crate::events::ReplacementPriority::SelfReplacement),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Instead(vec![Effect::gain_life_target(Value::EventValue(
                    EventValueSpec::Amount,
                ))]),
            ),
        );
        let outcome = process_damage_assignments_with_event(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            false,
            crate::events::cause::EventCause::from_effect(source, alice),
        )
        .expect("damage test proposal must process successfully");
        assert!(
            outcome.programs.is_empty(),
            "fixture has no added replacement instructions"
        );
        assert!(outcome.assignments.is_empty());
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 23);
    }

    #[test]
    fn damage_instead_payload_reads_the_modified_event_amount() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Replacement source", alice);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::new(
                    crate::target::PlayerFilter::You,
                ),
                ReplacementAction::Double,
            )
            .with_priority_override(crate::events::ReplacementPriority::SelfReplacement),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::new(
                    crate::target::PlayerFilter::You,
                ),
                ReplacementAction::Instead(vec![Effect::gain_life(Value::EventValue(
                    EventValueSpec::Amount,
                ))]),
            ),
        );
        let outcome = process_damage_assignments_with_event(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            false,
            crate::events::cause::EventCause::from_effect(source, alice),
        )
        .expect("damage test proposal must process successfully");
        assert!(
            outcome.programs.is_empty(),
            "fixture has no added replacement instructions"
        );
        assert!(outcome.assignments.is_empty());
        assert_eq!(
            game.player(alice).unwrap().life,
            26,
            "replacement payload must see doubled damage"
        );
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
        )
        .expect("damage test proposal must process successfully");

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
        )
        .expect("damage test proposal must process successfully");

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
        )
        .expect("damage test proposal must process successfully");

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
        )
        .expect("damage test proposal must process successfully");

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
            )
            .expect("damage test proposal must process successfully");

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
        )
        .expect("damage test proposal must process successfully");
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
        )
        .expect("damage test proposal must process successfully");
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
        )
        .expect("damage test proposal must process successfully");

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
        )
        .expect("damage test proposal must process successfully");
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
        )
        .expect("damage test proposal must process successfully");
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
        )
        .expect("damage test proposal must process successfully");
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
        )
        .expect("damage test proposal must process successfully");
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
        )
        .expect("damage test proposal must process successfully");
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
        )
        .expect("damage test proposal must process successfully");

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
        )
        .expect("damage test proposal must process successfully");
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
        )
        .expect("damage test proposal must process successfully");
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
            .expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions()
            .expect("the Siege should enter");

        assert_eq!(
            game.counter_count(entered.new_id, CounterType::Defense),
            8,
            "the intrinsic defense-counter replacement must use the ordinary ETB event"
        );
    }

    #[test]
    fn long_finite_replacement_chain_applies_every_distinct_effect() {
        #[derive(Clone, Debug)]
        struct ExactLifeGain(u32);
        impl crate::events::ReplacementMatcher for ExactLifeGain {
            fn matches_prepared_event(
                &self,
                event: &dyn crate::events::GameEventType,
                _: &crate::events::context::PreparedEventContext,
            ) -> bool {
                crate::events::downcast_event::<crate::events::LifeGainEvent>(event)
                    .is_some_and(|gain| gain.amount == self.0)
            }
            fn display(&self) -> String {
                format!("Gain exactly {} life", self.0)
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        for amount in 1..=130 {
            game.effect_store.replacement_effects.add_resolution_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    source,
                    alice,
                    ExactLifeGain(amount),
                    crate::replacement::ReplacementAction::Modify(
                        crate::replacement::EventModification::Add(1),
                    ),
                ),
            );
        }
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::EffectExecutor::execute(
            &crate::effects::GainLifeEffect::you(1),
            &mut game,
            &mut ctx,
        )
        .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 151);
        assert_eq!(outcome.count_or_zero(), 131);
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(
            outcome.events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .unwrap()
                .amount,
            131
        );
    }

    #[test]
    fn destination_continuation_retains_additions_through_pending_and_rescan() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Expanded destination")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Hand);
        let destination_id = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                object,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(object),
                    Some(Zone::Hand),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::InteractiveChooseDestination {
                    destinations: vec![Zone::Exile],
                    description: "Choose destination".into(),
                },
            ),
        );
        let later_id = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                object,
                bob,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(object),
                    Some(Zone::Hand),
                    Some(Zone::Exile),
                ),
                ReplacementAction::Additionally(vec![Effect::new(
                    crate::effects::GainLifeEffect::you(7),
                )]),
            ),
        );
        let event = Event::zone_change(
            object,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            None,
        );
        let earlier = PreparedReplacementProgram {
            context: Box::new(ReplacementEventContext::new(
                &game,
                event.clone(),
                &TraitEventProcessingState::default(),
            )),
            source: object,
            controller: alice,
            source_snapshot: None,
            effects: vec![Effect::new(crate::effects::GainLifeEffect::you(3))],
        };
        let pending = TraitEventResult::Expanded {
            original: Box::new(
                process_trait_event(&mut game, event)
                    .expect("finite replacement fixture evaluates successfully"),
            ),
            programs: vec![earlier],
        };
        struct Pause {
            asked: usize,
            pending: bool,
        }
        impl DecisionMaker for Pause {
            fn decide_options(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                self.asked += 1;
                self.pending = true;
                Vec::new()
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let mut dm = Pause {
            asked: 0,
            pending: false,
        };
        let pending = continue_after_destination_choices(&mut game, pending, &mut dm, &[])
            .expect("finite replacement fixture evaluates successfully");
        assert_eq!(
            dm.asked, 1,
            "the wrapped destination prompt must be reached"
        );
        let (original, programs) = pending.clone().into_expansion();
        assert!(matches!(
            original,
            TraitEventResult::NeedsInteraction { .. }
        ));
        assert_eq!(programs.len(), 1);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(destination_id)
                .is_some()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(later_id)
                .is_some()
        );
        let pending = continue_after_destination_choices(&mut game, pending, &mut dm, &[])
            .expect("finite replacement fixture evaluates successfully");
        assert_eq!(
            dm.asked, 1,
            "an outstanding prompt must not be replaced by another request"
        );
        let mut answer = crate::decision::SelectFirstDecisionMaker;
        let result = continue_after_destination_choices(&mut game, pending, &mut answer, &[])
            .expect("finite replacement fixture evaluates successfully");
        let (original, programs) = result.into_expansion();
        let event = match original {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => event,
            other => panic!("destination should finish: {other:?}"),
        };
        assert_eq!(
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
                .unwrap()
                .to,
            Zone::Exile
        );
        assert_eq!(
            programs.len(),
            2,
            "retain earlier addition and the newly applicable addition once"
        );
        assert_eq!(programs[0].controller, alice);
        assert_eq!(programs[1].controller, bob);
        assert_eq!(
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                programs[0].context.event.inner()
            )
            .unwrap()
            .to,
            Zone::Graveyard
        );
        assert_eq!(
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                programs[1].context.event.inner()
            )
            .unwrap()
            .to,
            Zone::Exile
        );
        assert!(
            programs[1]
                .context
                .applied_effects
                .contains(&destination_id)
        );
        assert!(programs[1].context.applied_effects.contains(&later_id));
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(destination_id)
                .is_none()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(later_id)
                .is_none()
        );
        assert_eq!(
            game.object(object).unwrap().zone,
            Zone::Hand,
            "continuation only prepares movement"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(
            game.player(bob).unwrap().life,
            20,
            "programs wait for the owning commit"
        );
    }

    #[test]
    fn interactive_one_shot_survives_until_destination_choice_completes() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Pending destination")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Hand);
        let effect = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                object,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(object),
                    Some(Zone::Hand),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::InteractiveChooseDestination {
                    destinations: vec![Zone::Exile, Zone::Graveyard],
                    description: "Choose destination".into(),
                },
            ),
        );
        let event = Event::zone_change(
            object,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            None,
        );
        let pending = process_trait_event(&mut game, event)
            .expect("finite replacement fixture evaluates successfully");
        assert!(matches!(pending, TraitEventResult::NeedsInteraction { .. }));
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(effect)
                .is_some(),
            "an unanswered interaction must retain its one-shot replacement"
        );
        struct Pending;
        impl crate::decision::DecisionMaker for Pending {
            fn decide_options(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                Vec::new()
            }
            fn awaiting_choice(&self) -> bool {
                true
            }
        }
        let pending = continue_after_destination_choices(&mut game, pending, &mut Pending, &[])
            .expect("finite replacement fixture evaluates successfully");
        assert!(matches!(pending, TraitEventResult::NeedsInteraction { .. }));
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(effect)
                .is_some()
        );
        assert_eq!(game.object(object).unwrap().zone, Zone::Hand);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = continue_after_destination_choices(&mut game, pending, &mut dm, &[])
            .expect("finite replacement fixture evaluates successfully");
        let event = match result {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => event,
            _ => panic!("destination choice must finish replacement processing"),
        };
        assert_eq!(
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
                .unwrap()
                .to,
            Zone::Exile
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(effect)
                .is_none()
        );
    }

    #[test]
    fn one_shot_entry_payment_waits_then_consumes_after_answer() {
        struct Answer {
            pause: bool,
            pending: bool,
        }
        impl crate::decision::DecisionMaker for Answer {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Entry payment")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Hand);
        let effect = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                object,
                alice,
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any(),
                ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost: 2 },
            ),
        );
        let mut dm = Answer {
            pause: true,
            pending: false,
        };
        let pending = process_etb_with_event_and_dm(&mut game, object, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");
        assert!(
            pending.prevented,
            "no entry may be committed before the answer"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(effect)
                .is_some()
        );
        dm.pause = false;
        dm.pending = false;
        let completed = process_etb_with_event_and_dm(&mut game, object, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");
        assert!(!completed.prevented);
        assert!(!completed.enters_tapped);
        assert_eq!(game.player(alice).unwrap().life, 18);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(effect)
                .is_none()
        );
    }

    #[test]
    fn one_shot_entry_counter_choice_keeps_action_until_applied() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counter choice")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Hand);
        let effect = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                object,
                alice,
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any(),
                ReplacementAction::EnterWithCounterChoice {
                    counter_types: vec![CounterType::PlusOnePlusOne, CounterType::Charge],
                    count: crate::effect::Value::Fixed(2),
                },
            ),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let completed = process_etb_with_event_and_dm(&mut game, object, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");
        assert!(!completed.prevented);
        assert_eq!(
            completed.enters_with_counters,
            vec![(CounterType::PlusOnePlusOne, 2)]
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(effect)
                .is_none()
        );
    }

    #[test]
    fn later_pending_entry_interaction_restores_earlier_payment_and_one_shots() {
        struct Answers {
            calls: usize,
            pause_second: bool,
            pending: bool,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_options(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                vec![0]
            }
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.calls += 1;
                self.pending = self.pause_second && self.calls == 2;
                !self.pending
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Two entry payments")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Hand);
        let mut shields = Vec::new();
        for _ in 0..2 {
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    object,
                    alice,
                    crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any(),
                    ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost: 2 },
                ),
            ));
        }
        let mut dm = Answers {
            calls: 0,
            pause_second: true,
            pending: false,
        };
        let pending = process_etb_with_event_and_dm(&mut game, object, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");
        assert!(pending.prevented);
        assert_eq!(dm.calls, 2);
        assert_eq!(game.player(alice).unwrap().life, 20);
        for effect in &shields {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(*effect)
                    .is_some()
            );
        }
        dm.calls = 0;
        dm.pause_second = false;
        dm.pending = false;
        let completed = process_etb_with_event_and_dm(&mut game, object, Zone::Hand, &mut dm)
            .expect("replacement operation must execute successfully in this scenario");
        assert!(!completed.prevented);
        assert!(!completed.enters_tapped);
        assert_eq!(game.player(alice).unwrap().life, 16);
        for effect in &shields {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(*effect)
                    .is_none()
            );
        }
    }

    #[test]
    fn pending_zone_destination_chain_restores_earlier_one_shot() {
        struct Answers {
            calls: usize,
            pause: bool,
            pending: bool,
        }
        impl DecisionMaker for Answers {
            fn decide_options(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                self.calls += 1;
                self.pending = self.pause && self.calls == 2;
                if self.pending { Vec::new() } else { vec![0] }
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let object =
            create_creature_in_zone(&mut game, "Pending zone chain", alice, Zone::Hand, 2, 2);
        let mut shields = Vec::new();
        for (from_destination, destination) in
            [(Zone::Graveyard, Zone::Exile), (Zone::Exile, Zone::Library)]
        {
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    object,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(object),
                        Some(Zone::Hand),
                        Some(from_destination),
                    ),
                    ReplacementAction::InteractiveChooseDestination {
                        destinations: vec![destination, from_destination],
                        description: "Choose destination".into(),
                    },
                ),
            ));
        }
        let mut dm = Answers {
            calls: 0,
            pause: true,
            pending: false,
        };
        let result = process_zone_change(
            &mut game,
            object,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            &mut dm,
        );
        assert!(dm.pending);
        assert_eq!(dm.calls, 2);
        assert!(
            !matches!(
                result
                    .expect("pending zone preparation must not fail")
                    .assert_without_additions(),
                EventOutcome::Proceed(_)
            ),
            "a pending event cannot permit a move"
        );
        assert_eq!(game.object(object).unwrap().zone, Zone::Hand);
        for shield in &shields {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(*shield)
                    .is_some()
            );
        }
        assert!(game.take_pending_trigger_events().is_empty());
        dm.calls = 0;
        dm.pause = false;
        dm.pending = false;
        let result = process_zone_change(
            &mut game,
            object,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            &mut dm,
        );
        let plan = result
            .expect("completed zone preparation must succeed")
            .assert_without_additions();
        assert!(
            matches!(plan, EventOutcome::Proceed(prepared) if prepared.final_zone() == Zone::Library)
        );
        assert_eq!(dm.calls, 2);
        for shield in &shields {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(*shield)
                    .is_none()
            );
        }
    }
}

#[cfg(test)]
mod unchanged_chain_tests {
    use super::*;

    fn scenario() -> (GameState, ObjectId, PlayerId, ReplacementEffectId) {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_card(
            &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Replacement source").build(),
            alice,
            Zone::Battlefield,
        );
        let unchanged = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::PreventHalfDamage { round_up: false },
            )
            .with_priority_override(crate::events::ReplacementPriority::SelfReplacement),
        );
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Double,
            ));
        (game, source, bob, unchanged)
    }

    #[test]
    fn unchanged_single_replacement_does_not_terminate_the_chain() {
        let (mut game, source, bob, unchanged) = scenario();
        let event = Event::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Player(bob),
                1,
                false,
                crate::events::cause::EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let result = process_trait_event(&mut game, event)
            .expect("finite replacement fixture evaluates successfully");
        let TraitEventResult::Proceed(event) = result else {
            panic!("expected a fully processed damage proposal, got {result:?}");
        };
        assert_eq!(
            crate::events::downcast_event::<crate::events::DamageEvent>(event.inner())
                .unwrap()
                .amount,
            2
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(unchanged)
                .is_some(),
            "a prevention action that prevented zero must remain available"
        );
    }

    #[test]
    fn unchanged_prevention_preserves_later_damage_replacements_and_its_own_lifetime() {
        use crate::effects::EffectExecutor;
        let (mut game, source, bob, unchanged) = scenario();
        let mut ctx =
            crate::effects::ExecutionContext::new_default(source, PlayerId::from_index(0));
        let first = crate::effects::DealDamageEffect::new(
            1,
            crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Specific(bob)),
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(game.player(bob).unwrap().life, 18);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(unchanged)
                .is_some()
        );
        let first_queued = game.take_pending_trigger_events();
        assert_eq!(
            first
                .events
                .iter()
                .chain(first_queued.iter())
                .filter(|event| event.kind() == crate::events::EventKind::DamagePrevented)
                .count(),
            0
        );
        // The no-op's per-event history must not suppress it on a later event.
        let second = crate::effects::DealDamageEffect::new(
            3,
            crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Specific(bob)),
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(game.player(bob).unwrap().life, 16);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(unchanged)
                .is_none()
        );
        let second_queued = game
            .turn_store
            .turn_history
            .projected_records()
            .map(|record| record.event.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            second
                .events
                .iter()
                .chain(second_queued.iter())
                .filter(|event| event.kind() == crate::events::EventKind::DamagePrevented)
                .count(),
            1
        );
        let prevented: Vec<_> = second
            .events
            .iter()
            .chain(second_queued.iter())
            .filter_map(|event| event.downcast::<crate::events::DamagePreventedEvent>())
            .collect();
        assert_eq!(prevented[0].amount, 1);
        assert_eq!(
            prevented[0].target,
            crate::events::DamageTarget::Player(bob)
        );
        assert_eq!(prevented[0].damage_source, source);
        assert_eq!(prevented[0].prevention_source, source);
    }
}

#[cfg(test)]
mod replacement_full_api_contract_tests {
    use super::*;
    use crate::effect::Effect;
    use crate::events::cause::EventCause;
    use crate::ids::CardId;
    fn setup() -> (GameState, PlayerId, PlayerId, ObjectId, ObjectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = crate::card::CardBuilder::new(CardId::new(), "Full API fixture")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let target = game.create_object_from_card(&card, alice, Zone::Hand);
        game.take_pending_trigger_events();
        (game, alice, bob, source, target)
    }
    #[test]
    fn full_draw_retains_redirected_recipient_and_count() {
        let (mut game, alice, bob, source, _) = setup();
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::cards::matchers::WouldDrawCardMatcher::new(
                    crate::target::PlayerFilter::Specific(alice),
                ),
                ReplacementAction::RedirectDrawToController,
            ));
        let result = process_draw_full(&mut game, alice, 2, true)
            .expect("finite replacement fixture evaluates successfully");
        let draw = crate::events::downcast_event::<crate::events::DrawEvent>(
            result.resolved_event().unwrap().inner(),
        )
        .unwrap();
        assert_eq!(draw.player, bob);
        assert_eq!(draw.count, 2);
        assert!(
            game.player(alice).unwrap().hand.len() == 1
                && game.player(bob).unwrap().hand.is_empty()
        );
        assert!(
            game.take_pending_trigger_events().is_empty(),
            "a proposal API must not publish a completed draw"
        );
    }
    #[test]
    fn full_draw_retains_additions_when_event_extraction_is_refused() {
        let (mut game, alice, bob, source, _) = setup();
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::cards::matchers::WouldDrawCardMatcher::new(
                    crate::target::PlayerFilter::Specific(alice),
                ),
                ReplacementAction::Additionally(vec![Effect::gain_life(3)]),
            ));
        let result = process_draw_full(&mut game, alice, 2, true)
            .expect("finite replacement fixture evaluates successfully");
        let retained = result
            .into_event()
            .expect_err("extracting only an event would discard additions");
        let (original, programs) = retained.into_expansion();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].source, source);
        assert_eq!(programs[0].controller, bob);
        assert_eq!(programs[0].context.affected_player, alice);
        assert_eq!(programs[0].effects.len(), 1);
        let draw = crate::events::downcast_event::<crate::events::DrawEvent>(
            original.resolved_event().unwrap().inner(),
        )
        .unwrap();
        assert_eq!(draw.player, alice);
        assert_eq!(draw.count, 2);
        assert_eq!(
            game.player(bob).unwrap().life,
            20,
            "the consumer owns execution after the original draw"
        );
    }
    #[test]
    fn full_zone_retains_instead_source_and_event_context() {
        let (mut game, alice, bob, source, target) = setup();
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(target),
                    Some(Zone::Hand),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::Instead(vec![Effect::gain_life(3)]),
            ));
        let result = process_zone_change_full(
            &mut game,
            target,
            Zone::Hand,
            Zone::Graveyard,
            EventCause::from_effect(source, bob),
        )
        .unwrap();
        let TraitEventResult::Replaced {
            context,
            source: actual_source,
            controller,
            effects,
            ..
        } = result
        else {
            panic!("the full zone API must return the replacement receipt");
        };
        assert_eq!(actual_source, source);
        assert_eq!(controller, bob);
        assert_eq!(effects.len(), 1);
        assert_eq!(context.affected_player, alice);
        let zone =
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(context.event.inner())
                .unwrap();
        assert_eq!(zone.objects, vec![target]);
        assert_eq!(zone.from, Zone::Hand);
        assert_eq!(zone.to, Zone::Graveyard);
        assert_eq!(game.object(target).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
    #[test]
    fn full_zone_retains_interaction_without_default_commit_permission() {
        let (mut game, _, bob, source, target) = setup();
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(target),
                    Some(Zone::Hand),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::InteractiveChooseDestination {
                    destinations: vec![Zone::Graveyard, Zone::Exile],
                    description: "Choose destination".into(),
                },
            ));
        let result = process_zone_change_full(
            &mut game,
            target,
            Zone::Hand,
            Zone::Graveyard,
            EventCause::from_game_rule(),
        )
        .unwrap();
        let retained = result
            .into_event()
            .expect_err("unanswered destination is not a resolved event");
        let TraitEventResult::NeedsInteraction {
            event,
            destinations,
            applied_effects,
            ..
        } = retained
        else {
            panic!("the destination interaction must reach the full API caller");
        };
        assert_eq!(destinations, Some(vec![Zone::Graveyard, Zone::Exile]));
        assert_eq!(applied_effects.len(), 1);
        assert_eq!(
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
                .unwrap()
                .objects,
            vec![target]
        );
        assert_eq!(game.object(target).unwrap().zone, Zone::Hand);
        assert!(game.take_pending_trigger_events().is_empty());
    }
}

#[cfg(test)]
mod replacement_damage_recipient_snapshot_contract_tests {
    use super::*;
    fn check(mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let first = game.create_object_from_card(
            &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Original recipient").build(),
            alice,
            Zone::Battlefield,
        );
        let second = game.create_object_from_card(
            &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Redirected recipient")
                .build(),
            alice,
            Zone::Battlefield,
        );
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(first).unwrap(), &game);
        let damage = crate::events::DamageEvent::with_cause(
            first,
            DamageTarget::Object(first),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        )
        .with_target_snapshot(snapshot);
        game.object_mut(first).unwrap().name = "Changed after snapshot".into();
        let target = match mode {
            0 => crate::game_state::Target::Object(second),
            1 => crate::game_state::Target::Player(alice),
            _ => crate::game_state::Target::Object(first),
        };
        let redirected = crate::events::GameEventType::with_target_replaced(
            &damage,
            &crate::game_state::Target::Object(first),
            &target,
        )
        .unwrap();
        let context = ReplacementEventContext::with_scope(
            &game,
            Event::from_boxed_with_provenance(redirected, crate::provenance::ProvNodeId::default()),
            &crate::effects::ReplacementExecutionContext::default(),
        );
        let captured =
            crate::events::downcast_event::<crate::events::DamageEvent>(context.event.inner())
                .unwrap();
        match mode {
            0 => {
                let snapshot = captured.target_snapshot.as_ref().unwrap();
                assert_eq!(captured.target, DamageTarget::Object(second));
                assert_eq!(snapshot.object_id, second);
                assert_eq!(snapshot.name, "Redirected recipient");
            }
            1 => {
                assert_eq!(captured.target, DamageTarget::Player(alice));
                assert!(
                    captured.target_snapshot.is_none(),
                    "redirected player cannot inherit prior permanent snapshot"
                );
            }
            _ => {
                let snapshot = captured.target_snapshot.as_ref().unwrap();
                assert_eq!(snapshot.object_id, first);
                assert_eq!(
                    snapshot.name, "Original recipient",
                    "same-recipient capture preserves earlier LKI"
                );
            }
        }
        assert_eq!(damage.target, DamageTarget::Object(first));
        assert_eq!(
            damage.target_snapshot.as_ref().unwrap().name,
            "Original recipient"
        );
    }
    #[test]
    fn redirected_object_captures_its_own_snapshot() {
        check(0);
    }
    #[test]
    fn redirected_player_has_no_permanent_snapshot() {
        check(1);
    }
    #[test]
    fn unchanged_recipient_preserves_captured_lki() {
        check(2);
    }
}

#[cfg(test)]
impl<T> PreparedEventOutcome<T> {
    pub(crate) fn assert_without_additions(self) -> EventOutcome<T> {
        assert!(
            self.programs.is_empty(),
            "fixture must finish retained replacement programs"
        );
        self.original
    }
}

#[cfg(test)]
mod replacement_invalid_draw_answer_contract_tests {
    use super::*;
    use crate::effects::EffectExecutor;
    use crate::replacement::ReplacementEffectId;
    struct Answer {
        indices: Vec<usize>,
        pending: bool,
        calls: usize,
    }
    impl DecisionMaker for Answer {
        fn decide_options(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.calls += 1;
            self.indices.clone()
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    fn fixture(prefix: bool) -> (GameState, PlayerId, ObjectId, Vec<ReplacementEffectId>) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(
            &crate::card::CardBuilder::new(
                crate::ids::CardId::new(),
                "Required replacement choice source",
            )
            .card_types(vec![crate::types::CardType::Enchantment])
            .build(),
            alice,
            Zone::Battlefield,
        );
        for name in ["First", "Second", "Third"] {
            game.create_object_from_card(
                &crate::card::CardBuilder::new(crate::ids::CardId::new(), name)
                    .card_types(vec![crate::types::CardType::Instant])
                    .build(),
                alice,
                Zone::Library,
            );
        }
        let mut shields = Vec::new();
        if prefix {
            let mut effect = ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Additionally(vec![crate::effect::Effect::gain_life(2)]),
            );
            effect.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
            shields.push(
                game.effect_store
                    .replacement_effects
                    .add_one_shot_effect(effect),
            );
        }
        for action in [ReplacementAction::Double, ReplacementAction::Prevent] {
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                    action,
                ),
            ));
        }
        game.take_pending_trigger_events();
        (game, alice, source, shields)
    }
    fn invalid(indices: Vec<usize>, scoped: bool, prefix: bool) {
        let (mut game, alice, source, shields) = fixture(prefix);
        let library = game.player(alice).unwrap().library.as_slice().to_vec();
        let ids = game.next_object_id_counter();
        let mut dm = Answer {
            indices,
            pending: false,
            calls: 0,
        };
        if scoped {
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
            let result =
                crate::effects::cards::DrawCardsEffect::you(1).execute(&mut game, &mut ctx);
            assert!(
                matches!(
                    result,
                    Err(crate::effects::ExecutionError::InternalError(_))
                ),
                "a malformed required replacement answer must fail before committing the draw"
            );
        } else {
            let result = process_draw(&mut game, alice, 1, true, &mut dm);
            assert!(
                matches!(
                    result,
                    Err(crate::effects::ExecutionError::InternalError(_))
                ),
                "root draw must not turn malformed input into the first replacement"
            );
        }
        assert_eq!(dm.calls, 1);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            library.as_slice()
        );
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(game.next_object_id_counter(), ids);
        assert!(game.take_pending_trigger_events().is_empty());
        for shield in shields {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some(),
                "failed choice must restore preceding applications and one-shot consumption"
            );
        }
    }
    #[test]
    fn root_empty_answer_is_invalid() {
        invalid(vec![], false, false);
    }
    #[test]
    fn root_out_of_range_answer_is_invalid() {
        invalid(vec![usize::MAX], false, false);
    }
    #[test]
    fn root_multiple_answers_are_invalid() {
        invalid(vec![1, 0], false, false);
    }
    #[test]
    fn root_duplicate_answers_are_invalid() {
        invalid(vec![0, 0], false, false);
    }
    #[test]
    fn scoped_empty_answer_is_invalid() {
        invalid(vec![], true, false);
    }
    #[test]
    fn scoped_out_of_range_answer_is_invalid() {
        invalid(vec![usize::MAX], true, false);
    }
    #[test]
    fn scoped_multiple_answers_are_invalid() {
        invalid(vec![1, 0], true, false);
    }
    #[test]
    fn scoped_duplicate_answers_are_invalid() {
        invalid(vec![0, 0], true, false);
    }
    #[test]
    fn invalid_answer_restores_an_already_applied_prefix() {
        invalid(vec![usize::MAX], false, true);
    }
    #[test]
    fn valid_answer_selects_its_offered_effect() {
        let (mut game, alice, _, shields) = fixture(false);
        let mut dm = Answer {
            indices: vec![1],
            pending: false,
            calls: 0,
        };
        let result = process_draw(&mut game, alice, 1, true, &mut dm).unwrap();
        assert!(matches!(result, ResolvedDrawOutcome::Prevented));
        assert_eq!(dm.calls, 1);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shields[0])
                .is_some()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shields[1])
                .is_none()
        );
    }
    #[test]
    fn pending_answer_remains_a_continuation_and_restores_prefix() {
        let (mut game, alice, _, shields) = fixture(true);
        let mut dm = Answer {
            indices: vec![],
            pending: true,
            calls: 0,
        };
        let result = process_draw(&mut game, alice, 1, true, &mut dm).unwrap();
        assert!(matches!(result, ResolvedDrawOutcome::Pending));
        assert_eq!(game.player(alice).unwrap().life, 20);
        for shield in shields {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
        }
        assert!(game.take_pending_trigger_events().is_empty());
    }
}

#[cfg(test)]
mod replacement_scoped_choice_owner_contract_tests {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
    use crate::ids::CardId;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::types::CardType;
    struct Answers {
        indices: Vec<usize>,
        pause: bool,
        pending: bool,
        calls: usize,
    }
    impl DecisionMaker for Answers {
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(ctx.min, 1);
            assert_eq!(ctx.max, 1);
            assert_eq!(ctx.options.len(), 2);
            self.calls += 1;
            self.pending = self.pause;
            self.indices.clone()
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    fn check(kind: u8, indices: Vec<usize>, pause: bool, prefix: bool) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let actor = game.create_object_from_definition(
            &crate::CardDefinitionBuilder::new(CardId::new(), "Keyword actor")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(3, 3))
                .build(),
            alice,
            Zone::Battlefield,
        );
        let shield_source = game.create_object_from_definition(
            &crate::CardDefinitionBuilder::new(CardId::new(), "Keyword replacement")
                .card_types(vec![CardType::Artifact])
                .build(),
            bob,
            Zone::Battlefield,
        );
        let library = game.create_object_from_definition(
            &crate::CardDefinitionBuilder::new(CardId::new(), "Uncommitted card")
                .card_types(vec![CardType::Instant])
                .build(),
            alice,
            Zone::Library,
        );
        game.set_damage_marked(actor, 2);
        game.object_mut(actor)
            .unwrap()
            .counters
            .insert(CounterType::Charge, 1);
        let action = match kind {
            0 => crate::events::KeywordActionKind::Learn,
            1 => crate::events::KeywordActionKind::Heal,
            2 => crate::events::KeywordActionKind::Connive,
            _ => crate::events::KeywordActionKind::Proliferate,
        };
        let matcher =
            crate::events::other::WouldKeywordActionMatcher::new(action, ObjectFilter::default());
        let mut shields = Vec::new();
        if prefix {
            let mut effect = ReplacementEffect::with_matcher(
                shield_source,
                bob,
                matcher.clone(),
                ReplacementAction::Additionally(vec![Effect::gain_life(2)]),
            );
            effect.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
            shields.push(
                game.effect_store
                    .replacement_effects
                    .add_one_shot_effect(effect),
            );
        }
        for action in [
            ReplacementAction::Prevent,
            ReplacementAction::Instead(vec![Effect::gain_life(3)]),
        ] {
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(shield_source, bob, matcher.clone(), action),
            ));
        }
        let ids = game.next_object_id_counter();
        let object_count = game.objects_in_deterministic_order().len();
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(actor).unwrap(), &game);
        game.take_pending_trigger_events();
        let mut dm = Answers {
            indices,
            pause,
            pending: false,
            calls: 0,
        };
        let mut ctx = ExecutionContext::new(actor, alice, &mut dm);
        ctx.set_tagged_objects("sentinel", vec![snapshot]);
        let result = match kind {
            0 => crate::effects::LearnEffect::new().execute(&mut game, &mut ctx),
            1 => crate::effects::HealDamageEffect::exact(ChooseSpec::SpecificObject(actor), 1)
                .execute(&mut game, &mut ctx),
            2 => crate::effects::ConniveEffect::new(ChooseSpec::SpecificObject(actor))
                .execute(&mut game, &mut ctx),
            _ => crate::effects::ProliferateEffect::new(1).execute(&mut game, &mut ctx),
        };
        if pause {
            assert!(ctx.decision_maker.awaiting_choice());
            assert!(result.unwrap().events.is_empty());
        } else {
            assert!(
                matches!(result, Err(ExecutionError::InternalError(_))),
                "completed malformed replacement input must fail in every scoped owner"
            );
        }
        assert_eq!(ctx.source, actor);
        assert_eq!(ctx.controller, alice);
        assert_eq!(ctx.get_tagged_all("sentinel").unwrap()[0].object_id, actor);
        drop(ctx);
        assert_eq!(dm.calls, 1);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().library.as_slice(), &[library]);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(game.damage_on(actor), 2);
        assert_eq!(game.counter_count(actor, CounterType::Charge), 1);
        assert_eq!(game.next_object_id_counter(), ids);
        assert_eq!(game.objects_in_deterministic_order().len(), object_count);
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(shields.iter().all(|id| {
            game.effect_store
                .replacement_effects
                .get_effect(*id)
                .is_some()
        }));
    }
    macro_rules! cases {
        ($kind:expr,$empty:ident,$range:ident,$multiple:ident,$duplicate:ident,$prefix:ident,$pending:ident) => {
            #[test]
            fn $empty() {
                check($kind, vec![], false, false);
            }
            #[test]
            fn $range() {
                check($kind, vec![usize::MAX], false, false);
            }
            #[test]
            fn $multiple() {
                check($kind, vec![0, 1], false, false);
            }
            #[test]
            fn $duplicate() {
                check($kind, vec![0, 0], false, false);
            }
            #[test]
            fn $prefix() {
                check($kind, vec![usize::MAX], false, true);
            }
            #[test]
            fn $pending() {
                check($kind, vec![0], true, true);
            }
        };
    }
    cases!(
        0,
        learn_empty,
        learn_range,
        learn_multiple,
        learn_duplicate,
        learn_invalid_after_prefix,
        learn_real_pending
    );
    cases!(
        1,
        heal_empty,
        heal_range,
        heal_multiple,
        heal_duplicate,
        heal_invalid_after_prefix,
        heal_real_pending
    );
    cases!(
        2,
        connive_empty,
        connive_range,
        connive_multiple,
        connive_duplicate,
        connive_invalid_after_prefix,
        connive_real_pending
    );
    cases!(
        3,
        proliferate_empty,
        proliferate_range,
        proliferate_multiple,
        proliferate_duplicate,
        proliferate_invalid_after_prefix,
        proliferate_real_pending
    );
}

#[cfg(test)]
mod replacement_additional_choice_label_contract_tests {
    use super::*;
    fn fixture(optional: bool) -> (GameState, ReplacementEffect) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(
            &crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Additional source")
                .card_types(vec![crate::types::CardType::Enchantment])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let effect = ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::cards::matchers::WouldDrawCardMatcher::you(),
            ReplacementAction::Additionally(vec![crate::effect::Effect::gain_life(3)]),
        );
        (game, if optional { effect.optional() } else { effect })
    }
    #[test]
    fn additional_apply_option_describes_application() {
        let (game, effect) = fixture(false);
        assert_eq!(
            replacement_effect_choice_description(&game, &effect),
            "Apply Additional source"
        );
    }
    #[test]
    fn optional_additional_apply_and_decline_are_distinguishable() {
        let (game, effect) = fixture(true);
        let decline = effect.optional_decline_effect().unwrap();
        let apply_text = replacement_effect_choice_description(&game, &effect);
        let decline_text = replacement_effect_choice_description(&game, &decline);
        assert_ne!(
            apply_text, decline_text,
            "applying additional actions must not look like declining them"
        );
        assert_eq!(decline_text, "Do not apply Additional source");
    }
}

#[cfg(test)]
mod native_prevention_savepoint_tests {
    use super::*;
    #[test]
    fn cloned_prevention_follow_up_executes_saved_event_source_targets_and_scope_once() {
        use crate::effect::{Effect, EventValueSpec, Value};
        use crate::prevention::{PreventionEffectManager, PreventionShield, PreventionTarget};
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut original = crate::tests::test_helpers::setup_two_player_game();
        original.turn.turn_number = 4;
        let definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Prevention source",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .build();
        let source =
            original.create_object_from_definition(&definition, alice, crate::Zone::Battlefield);
        let source_snapshot =
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                original.object(source).unwrap(),
                &original,
            );
        let suppressed = original
            .effect_store
            .replacement_effects
            .add_resolution_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                ReplacementAction::Double,
            ));
        let key = original
            .effect_store
            .replacement_effects
            .get_effect(suppressed)
            .unwrap()
            .application_key();
        let mut scope = crate::effects::ReplacementExecutionContext::default();
        scope.suppressed_replacement_effect_keys.insert(key);
        scope
            .additional_replacement_effects
            .push(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::any_player(),
                ReplacementAction::Modify(crate::replacement::EventModification::Add(1)),
            ));
        let mut manager = PreventionEffectManager::new();
        manager.set_turn(4);
        let shield = manager.add_shield(
            PreventionShield::prevent_next_n(source, alice, PreventionTarget::You, 5)
                .with_follow_up_effects(vec![
                    Effect::gain_life(Value::EventValue(EventValueSpec::Amount)),
                    Effect::gain_life_target(Value::SourcePower),
                ])
                .with_follow_up_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
                .with_follow_up_target_assignments(vec![crate::game_state::TargetAssignment {
                    spec: crate::target::ChooseSpec::target_player(),
                    range: 0..1,
                }]),
        );
        manager.begin_follow_up_replacement_scope(&scope);
        let follow_up = manager
            .apply_chosen_shield(shield, 2, true, None)
            .follow_ups
            .remove(0);
        manager.queue_follow_up_with_source_snapshot(
            follow_up,
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Player(alice),
                2,
                false,
                crate::events::cause::EventCause::from_effect(source, bob),
            ),
            crate::provenance::ProvNodeId::default(),
            Some(source_snapshot),
        );
        manager.end_follow_up_replacement_scope();
        let mut guest = crate::tests::test_helpers::setup_two_player_game();
        guest.turn.turn_number = 4;
        guest.effect_store.replacement_effects = original.effect_store.replacement_effects.clone();
        guest.effect_store.prevention_effects = manager.clone();
        assert!(
            guest.object(source).is_none(),
            "saved source snapshot must supply departed source characteristics"
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        execute_pending_prevention_follow_ups(&mut guest, &mut dm).unwrap();
        assert_eq!(
            guest.player(alice).unwrap().life,
            23,
            "saved amount and scope prevent reapplying the suppressed doubler"
        );
        assert_eq!(
            guest.player(bob).unwrap().life,
            24,
            "captured source power and explicit target survive"
        );
        assert_eq!(
            guest
                .effect_store
                .prevention_effects
                .prevented_by_shield(shield),
            2
        );
        assert_eq!(
            guest
                .effect_store
                .prevention_effects
                .get_shield_mut(shield)
                .unwrap()
                .amount_remaining,
            Some(3)
        );
        assert!(
            !guest
                .effect_store
                .prevention_effects
                .has_pending_follow_ups()
        );
        let observed: Vec<_> = guest
            .take_pending_trigger_events()
            .into_iter()
            .filter_map(|event| {
                event
                    .downcast::<crate::events::LifeGainEvent>()
                    .map(|gain| (gain.player, gain.amount))
            })
            .collect();
        assert_eq!(observed, vec![(alice, 3), (bob, 4)]);
        execute_pending_prevention_follow_ups(&mut guest, &mut dm).unwrap();
        assert_eq!(guest.player(alice).unwrap().life, 23);
        assert_eq!(guest.player(bob).unwrap().life, 24);
        assert!(guest.take_pending_trigger_events().is_empty());
    }
}

#[cfg(test)]
mod sacrifice_gate_resource_failure_tests {
    use super::*;
    #[test]
    fn later_sacrifice_exhaustion_restores_earlier_payment_and_surfaces_error() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(
            crate::ids::CardId::new(),
            "Sacrifice gate resource fixture",
        )
        .card_types(vec![CardType::Artifact])
        .build();
        let source = game.create_object_from_card(&card, player, Zone::Stack);
        let first = game.create_object_from_card(&card, player, Zone::Battlefield);
        let second = game.create_object_from_card(&card, player, Zone::Battlefield);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                player,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(second),
                    Some(Zone::Battlefield),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::Additionally(vec![crate::effect::Effect::new(
                    crate::effects::CreateTokenEffect::you(
                        crate::cards::tokens::treasure_token_definition(),
                        2,
                    ),
                )]),
            ),
        );
        game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits {
            max_created_tokens: 1,
            ..Default::default()
        });
        game.take_pending_trigger_events();
        let next = game.next_object_id_counter();
        let result = handle_sacrifice_or_redirect(
            &mut game,
            &InteractiveReplacementResponse::Objects(vec![first, second]),
            source,
            player,
            &crate::target::ObjectFilter::default(),
            2,
            Zone::Graveyard,
            crate::provenance::ProvNodeId::default(),
            &mut crate::decision::SelectFirstDecisionMaker,
        );
        assert!(matches!(
            result,
            Err(crate::effects::ExecutionError::ResourceLimitExceeded { .. })
        ));
        assert!(game.battlefield.contains(&first));
        assert!(game.battlefield.contains(&second));
        assert_eq!(game.object(source).unwrap().zone, Zone::Stack);
        assert_eq!(game.next_object_id_counter(), next);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }
}
