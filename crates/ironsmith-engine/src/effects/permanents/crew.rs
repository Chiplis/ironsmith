//! Crew cost effect implementation.
//!
//! This effect is intended to be used as a COST component for the Crew keyword:
//! "Tap any number of untapped creatures you control with total power N or more".
//!
//! When paid, we also record which creatures crewed the source this turn so
//! later effects/triggers can reference "each creature that crewed it this turn".

use std::collections::HashMap;

use crate::ability::AbilityKind;
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind, PermanentTappedEvent};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbilityId;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
use crate::types::CardType;
pub type CrewCostEffect = ironsmith_core::CrewCostEffect;

const CREWED_VEHICLE_TAG: &str = "__it__";
const CREW_ACTIVATION_TAG: &str = "__crew_activation";
const CREWERS_TAG: &str = "crewed_it_this_turn";
const FIRST_CREWED_THIS_TURN_TAG: &str = "__first_crewed_this_turn";
// Cost-time tags carried on the crew ability's stack entry until it resolves.
const PENDING_CREW_ACTIVATION_TAG: &str = "__crew_activation_pending";
const PENDING_CREWERS_TAG: &str = "__crew_activation_pending_crewers";

/// CR 702.122a: crew taps any number of *other* untapped creatures.
fn crew_candidates(game: &GameState, source: ObjectId, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            if id == source {
                return false;
            }
            let Some(obj) = game.object(id) else {
                return false;
            };
            game.current_is_creature(id)
                && game.controller_of(obj) == controller
                && !game.is_tapped(id)
                // CR 702.26b: a phased-out permanent is treated as though it
                // doesn't exist.
                && !game.is_phased_out(id)
        })
        .collect()
}

fn object_power(game: &GameState, object_id: ObjectId) -> i32 {
    game.calculated_characteristics(object_id)
        .and_then(|calc| calc.power)
        .or_else(|| game.object(object_id).and_then(|obj| obj.power()))
        .unwrap_or(0)
}

fn object_toughness(game: &GameState, object_id: ObjectId) -> i32 {
    game.calculated_characteristics(object_id)
        .and_then(|calc| calc.toughness)
        .or_else(|| game.object(object_id).and_then(|obj| obj.toughness()))
        .unwrap_or(0)
}

fn keyword_marker_texts(game: &GameState, object_id: ObjectId) -> Vec<String> {
    let abilities = game.current_abilities(object_id).unwrap_or_else(|| {
        game.object(object_id)
            .map(|obj| obj.abilities_vec())
            .unwrap_or_default()
    });
    abilities
        .into_iter()
        .filter_map(|ability| match ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::KeywordMarker =>
            {
                Some(static_ability.display().to_ascii_lowercase())
            }
            _ => None,
        })
        .collect()
}

fn crew_power_bonus_from_marker(marker: &str) -> Option<i32> {
    let prefixes = [
        "this creature crews vehicles as though its power were ",
        "this creature saddles mounts and crews vehicles as though its power were ",
        "this token saddles mounts and crews vehicles as though its power were ",
    ];
    prefixes.iter().find_map(|prefix| {
        marker
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(" greater."))
            .and_then(|amount| amount.parse::<i32>().ok())
    })
}

fn crew_value(game: &GameState, object_id: ObjectId) -> i32 {
    let markers = keyword_marker_texts(game, object_id);
    let use_toughness = markers.iter().any(|marker| {
        let marker = marker.trim_end_matches('.');
        marker == "this creature crews vehicles using its toughness rather than its power"
            || marker
                == "this creature saddles mounts and crews vehicles using its toughness rather than its power"
    });
    let base = if use_toughness {
        object_toughness(game, object_id)
    } else {
        object_power(game, object_id)
    };
    base + markers
        .iter()
        .filter_map(|marker| crew_power_bonus_from_marker(marker))
        .sum::<i32>()
}

fn source_has_loyalty_crew_alternative(game: &GameState, source: ObjectId) -> bool {
    keyword_marker_texts(game, source).iter().any(|marker| {
        marker.starts_with(
            "you may remove a loyalty counter from a planeswalker you control rather than pay ",
        ) && marker.ends_with("'s crew cost.")
    })
}

fn loyalty_planeswalker_for_crew(game: &GameState, controller: PlayerId) -> Option<ObjectId> {
    game.battlefield.iter().copied().find(|id| {
        let Some(obj) = game.object(*id) else {
            return false;
        };
        if game.controller_of(obj) != controller || obj.loyalty().unwrap_or(0) == 0 {
            return false;
        }
        game.calculated_characteristics(*id)
            .map(|calc| calc.card_types.contains(&CardType::Planeswalker))
            .unwrap_or_else(|| obj.has_card_type(CardType::Planeswalker))
    })
}

fn can_pay_loyalty_crew_alternative(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> bool {
    source_has_loyalty_crew_alternative(game, source)
        && loyalty_planeswalker_for_crew(game, controller).is_some()
}

fn pay_loyalty_crew_alternative(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let planeswalker = loyalty_planeswalker_for_crew(game, ctx.controller).ok_or_else(|| {
        ExecutionError::Impossible(
            "No planeswalker with a loyalty counter available to pay crew cost".to_string(),
        )
    })?;
    let event = crate::events::Event::remove_counters(planeswalker, CounterType::Loyalty, 1)
        .with_provenance(ctx.provenance);
    let payment =
        crate::effects::counters::execute_counter_removal_cost_with_outputs(game, ctx, event)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(payment);
    }
    if payment.outcome.requested_amount() != Some(1)
        || payment.outcome.instruction_result().status != crate::effect::OutcomeStatus::Succeeded
    {
        return Err(ExecutionError::Impossible(
            "No loyalty counter could be removed to pay crew cost".to_string(),
        ));
    }
    Ok(payment)
}

#[cfg(test)]
mod loyalty_counter_payment_receipt_tests {
    use super::*;

    // UNRUN: replacement changes the physical action, not acceptance of the
    // nominal loyalty cost that was available when this payment began.
    #[test]
    fn loyalty_crew_accepts_prevented_or_reduced_counter_payment() {
        for prevented in [false, true] {
            let payer = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let vehicle = crate::card::CardBuilder::new(crate::CardId::new(), "Crew source")
                .card_types(vec![CardType::Artifact]).build();
            let source = game.create_object_from_card(&vehicle, payer, crate::Zone::Battlefield);
            let card = crate::card::CardBuilder::new(crate::CardId::new(), "Loyalty payer")
                .card_types(vec![CardType::Planeswalker]).loyalty(3).build();
            let walker = game.create_object_from_card(&card, payer, crate::Zone::Battlefield);
            game.object_mut(walker).unwrap().counters.insert(CounterType::Loyalty, 3);
            let action = if prevented { crate::replacement::ReplacementAction::Prevent }
                else { crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Subtract(1)) };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(source, payer,
                    crate::events::counters::matchers::WouldRemoveCountersMatcher::new(
                        crate::target::ObjectFilter::specific(walker), Some(CounterType::Loyalty)), action));
            let mut ctx = ExecutionContext::new_default(source, payer);
            let payment = pay_loyalty_crew_alternative(&mut game, &mut ctx).unwrap();
            assert_eq!(payment.outcome.requested_amount(), Some(1));
            assert_eq!(payment.outcome.instruction_result().count_or_zero(), 1,
                "the legacy direct caller retains its nominal payment result");
            assert_eq!(payment.shared.len(), 1);
            assert_eq!(payment.shared[0].outputs.outcome.instruction_result().count_or_zero(), 0,
                "the physical child still records no loyalty removal");
            assert_eq!(payment.outcome.instruction_result().status, crate::effect::OutcomeStatus::Succeeded);
            assert_eq!(game.counter_count(walker, CounterType::Loyalty), 3);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
    }
}

/// CR 702.122b: the "crews a Vehicle" event for one creature tapped to pay a
/// crew cost. The Vehicle's own "becomes crewed" event waits for resolution.
fn keyword_crew_event(
    game: &GameState,
    crewer: ObjectId,
    vehicle: ObjectId,
    controller: PlayerId,
    crew_count: usize,
    provenance: crate::provenance::ProvNodeId,
) -> TriggerEvent {
    let event_snapshot = game
        .object(crewer)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
    let mut object_tags = HashMap::new();
    if let Some(vehicle_snapshot) = game
        .object(vehicle)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
    {
        object_tags.insert(TagKey::from(CREWED_VEHICLE_TAG), vec![vehicle_snapshot]);
    }
    TriggerEvent::new_with_provenance(
        KeywordActionEvent::new(
            KeywordActionKind::Crew,
            controller,
            crewer,
            crew_count as u32,
        )
        .with_snapshot(event_snapshot)
        .with_object_tags(object_tags),
        provenance,
    )
}

/// Records, on the crew ability's cost tags, the data for the deferred
/// "becomes crewed" event (CR 702.122d).
fn stash_pending_crew_activation(
    game: &GameState,
    ctx: &mut ExecutionContext,
    crewers: &[ObjectId],
) {
    let Some(vehicle_snapshot) = game
        .object(ctx.source)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
    else {
        return;
    };
    let crewer_snapshots = crewers
        .iter()
        .filter_map(|id| {
            game.object(*id)
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        })
        .collect::<Vec<_>>();
    ctx.set_tagged_objects(PENDING_CREW_ACTIVATION_TAG, vec![vehicle_snapshot.clone()]);
    ctx.set_tagged_objects(PENDING_CREWERS_TAG, crewer_snapshots);
}

/// CR 702.122d: builds the Vehicle's "becomes crewed" keyword-action event
/// when a crew ability resolves, and records the resolution for "becomes
/// crewed for the first time each turn". Returns `None` for any other stack
/// entry.
fn build_crew_ability_resolved_event(
    game: &mut GameState,
    entry: &crate::game_state::StackEntry,
) -> Option<TriggerEvent> {
    if !entry.is_ability {
        return None;
    }
    let pending_vehicle = entry
        .tagged_objects
        .get(&TagKey::from(PENDING_CREW_ACTIVATION_TAG))?
        .first()?
        .clone();
    let vehicle = pending_vehicle.object_id;
    let vehicle_snapshot = game
        .object(vehicle)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .unwrap_or(pending_vehicle);
    let crewer_snapshots = entry
        .tagged_objects
        .get(&TagKey::from(PENDING_CREWERS_TAG))
        .cloned()
        .unwrap_or_default();
    // CR 702.122d: the first crew ability of this Vehicle to *resolve* this
    // turn is its first "becomes crewed"; a countered activation or a
    // loyalty-paid crew doesn't distort this.
    let is_first_crewed_this_turn = game
        .turn_store
        .turn_history
        .crew_abilities_resolved_this_turn
        .insert(vehicle);
    let mut object_tags = HashMap::new();
    object_tags.insert(
        TagKey::from(CREWED_VEHICLE_TAG),
        vec![vehicle_snapshot.clone()],
    );
    object_tags.insert(
        TagKey::from(CREW_ACTIVATION_TAG),
        vec![vehicle_snapshot.clone()],
    );
    if is_first_crewed_this_turn {
        object_tags.insert(
            TagKey::from(FIRST_CREWED_THIS_TURN_TAG),
            vec![vehicle_snapshot.clone()],
        );
    }
    let crew_count = crewer_snapshots.len();
    if !crewer_snapshots.is_empty() {
        object_tags.insert(TagKey::from(CREWERS_TAG), crewer_snapshots);
    }
    Some(TriggerEvent::new_with_provenance(
        KeywordActionEvent::new(
            KeywordActionKind::Crew,
            entry.controller,
            vehicle,
            crew_count as u32,
        )
        .with_snapshot(Some(vehicle_snapshot))
        .with_object_tags(object_tags),
        entry.provenance,
    ))
}

/// Publication remains at crew-ability resolution, independently of payment.
/// Undo first-resolution bookkeeping if the completion cannot be published.
pub(crate) fn complete_crew_ability_resolution(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    entry: &crate::game_state::StackEntry,
) -> Result<EffectOutcome, ExecutionError> {
    crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
        let Some(event) = build_crew_ability_resolved_event(game, entry) else {
            return Ok(EffectOutcome::resolved());
        };
        crate::effects::composition::publish_keyword_action_completion(game, ctx, event)
    })
}

fn contributor_value(effect: &CrewCostEffect, game: &GameState, id: ObjectId) -> i32 {
    if effect.teamwork {
        object_power(game, id)
    } else {
        crew_value(game, id)
    }
}

impl EffectExecutor for CrewCostEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let controller = ctx.controller;
                let mut candidates = crew_candidates(game, ctx.source, controller);
                if candidates.is_empty() && self.required_power > 0 {
                    if !self.teamwork
                        && can_pay_loyalty_crew_alternative(game, ctx.source, controller)
                    {
                        let payment = pay_loyalty_crew_alternative(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        stash_pending_crew_activation(game, ctx, &[]);
                        return Ok(CompletedEffectOutputs::with_primary_result(
                            EffectOutcome::resolved(),
                            [payment],
                        ));
                    }
                    return Err(ExecutionError::Impossible(
                        "No untapped creatures available to crew".to_string(),
                    ));
                }

                let min = if self.required_power == 0 { 0 } else { 1 };
                let max = Some(candidates.len());
                let chosen = {
                    // Prefer higher-power candidates in fallback selection.
                    candidates.sort_by_key(|id| -contributor_value(self, game, *id));
                    let spec = ChooseObjectsSpec::new(
                        ctx.source,
                        if self.teamwork {
                            "Choose creatures for teamwork"
                        } else {
                            "Choose creatures to crew"
                        },
                        candidates.clone(),
                        min,
                        max,
                    );
                    make_decision(game, ctx.decision_maker, controller, Some(ctx.source), spec)
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let mut chosen = chosen;
                chosen.sort();
                chosen.dedup();

                // If the decision maker picked a set that doesn't meet the requirement,
                // greedily add remaining candidates until it does (or we exhaust options).
                let required = self.required_power as i32;
                let mut total_power: i32 = chosen
                    .iter()
                    .map(|id| contributor_value(self, game, *id))
                    .sum();
                if total_power < required {
                    let mut remaining: Vec<ObjectId> = candidates
                        .iter()
                        .copied()
                        .filter(|id| !chosen.contains(id))
                        .collect();
                    remaining.sort_by_key(|id| -contributor_value(self, game, *id));
                    for id in remaining {
                        if total_power >= required {
                            break;
                        }
                        chosen.push(id);
                        total_power += contributor_value(self, game, id);
                    }
                }

                if total_power < required {
                    if !self.teamwork
                        && can_pay_loyalty_crew_alternative(game, ctx.source, controller)
                    {
                        let payment = pay_loyalty_crew_alternative(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        stash_pending_crew_activation(game, ctx, &[]);
                        return Ok(CompletedEffectOutputs::with_primary_result(
                            EffectOutcome::resolved(),
                            [payment],
                        ));
                    }
                    return Err(ExecutionError::Impossible(
                        "Not enough total power to crew".to_string(),
                    ));
                }

                let taps = super::tap::tap_cost_objects_with_outputs(game, ctx, &chosen)?;
                let batch = taps
                    .outcome
                    .events
                    .iter()
                    .find_map(TriggerEvent::simultaneous_batch);
                let mut children = vec![taps];
                let mut completions = Vec::new();
                let crew_count = chosen.len();

                if self.teamwork {
                    return Ok(CompletedEffectOutputs::with_primary_result(
                        EffectOutcome::resolved(),
                        children,
                    ));
                }
                // CR 702.122d: "becomes crewed" means "a crew ability of this Vehicle
                // resolves", so the Vehicle-level event is deferred to resolution.
                // Stash what it needs on the activation's cost tags; the stack entry
                // carries them to `complete_crew_ability_resolution`.
                stash_pending_crew_activation(game, ctx, &chosen);
                for id in chosen.iter() {
                    let mut event = keyword_crew_event(
                        game,
                        *id,
                        ctx.source,
                        controller,
                        crew_count,
                        ctx.provenance,
                    );
                    if let Some(batch) = batch {
                        event = event.with_simultaneous_batch(batch);
                    }
                    completions.push(event);
                }

                // Record crew contributors for "crewed it this turn" references.
                let entry = game
                    .turn_store
                    .turn_history
                    .crewed_this_turn
                    .entry(ctx.source)
                    .or_default();
                for id in chosen {
                    if !entry.contains(&id) {
                        entry.push(id);
                    }
                }

                for event in completions {
                    children.push(
                        crate::effects::composition::publish_keyword_action_completion_receipt(
                            game, ctx, event,
                        )?,
                    );
                }
                Ok(CompletedEffectOutputs::with_primary_result(
                    EffectOutcome::resolved(),
                    children,
                ))
            },
        )
    }

    fn cost_description(&self) -> Option<String> {
        Some(format!(
            "Tap any number of untapped creatures you control with total power {} or more",
            self.required_power
        ))
    }
}

impl CostExecutableEffect for CrewCostEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if self.required_power == 0 {
            return Ok(());
        }
        let candidates = crew_candidates(game, source, controller);
        let total: i32 = candidates
            .iter()
            .map(|id| contributor_value(self, game, *id).max(0))
            .sum();
        if total >= self.required_power as i32
            || (!self.teamwork && can_pay_loyalty_crew_alternative(game, source, controller))
        {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "Not enough total power to crew".to_string(),
            ))
        }
    }
}
