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
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
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

fn crew_candidates(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            let Some(obj) = game.object(id) else {
                return false;
            };
            game.current_is_creature(id)
                && game.controller_of(obj) == controller
                && !game.is_tapped(id)
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
            .map(|obj| obj.abilities.clone())
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
        marker == "this creature crews vehicles using its toughness rather than its power."
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
    source: ObjectId,
    controller: PlayerId,
) -> Result<TriggerEvent, ExecutionError> {
    let planeswalker = loyalty_planeswalker_for_crew(game, controller).ok_or_else(|| {
        ExecutionError::Impossible(
            "No planeswalker with a loyalty counter available to pay crew cost".to_string(),
        )
    })?;
    game.remove_counters(
        planeswalker,
        CounterType::Loyalty,
        1,
        Some(source),
        Some(controller),
    )
    .map(|(_, event)| event)
    .ok_or_else(|| {
        ExecutionError::Impossible(
            "No loyalty counter could be removed to pay crew cost".to_string(),
        )
    })
}

fn keyword_crew_event(
    game: &GameState,
    crewer: ObjectId,
    vehicle: ObjectId,
    controller: PlayerId,
    provenance: crate::provenance::ProvNodeId,
) -> TriggerEvent {
    let crewer_snapshot = game
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
        KeywordActionEvent::new(KeywordActionKind::Crew, controller, crewer, 1)
            .with_snapshot(crewer_snapshot)
            .with_object_tags(object_tags),
        provenance,
    )
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
        let controller = ctx.controller;
        let mut candidates = crew_candidates(game, controller);
        if candidates.is_empty() && self.required_power > 0 {
            if can_pay_loyalty_crew_alternative(game, ctx.source, controller) {
                let event = pay_loyalty_crew_alternative(game, ctx.source, controller)?;
                return Ok(EffectOutcome::resolved().with_events(vec![event]));
            }
            return Err(ExecutionError::Impossible(
                "No untapped creatures available to crew".to_string(),
            ));
        }

        let min = if self.required_power == 0 { 0 } else { 1 };
        let max = Some(candidates.len());
        let chosen = {
            // Prefer higher-power candidates in fallback selection.
            candidates.sort_by_key(|id| -crew_value(game, *id));
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose creatures to crew",
                candidates.clone(),
                min,
                max,
            );
            make_decision(game, ctx.decision_maker, controller, Some(ctx.source), spec)
        };

        let mut chosen = chosen;
        chosen.sort();
        chosen.dedup();

        // If the decision maker picked a set that doesn't meet the requirement,
        // greedily add remaining candidates until it does (or we exhaust options).
        let required = self.required_power as i32;
        let mut total_power: i32 = chosen.iter().map(|id| crew_value(game, *id)).sum();
        if total_power < required {
            let mut remaining: Vec<ObjectId> = candidates
                .iter()
                .copied()
                .filter(|id| !chosen.contains(id))
                .collect();
            remaining.sort_by_key(|id| -crew_value(game, *id));
            for id in remaining {
                if total_power >= required {
                    break;
                }
                chosen.push(id);
                total_power += crew_value(game, id);
            }
        }

        if total_power < required {
            if can_pay_loyalty_crew_alternative(game, ctx.source, controller) {
                let event = pay_loyalty_crew_alternative(game, ctx.source, controller)?;
                return Ok(EffectOutcome::resolved().with_events(vec![event]));
            }
            return Err(ExecutionError::Impossible(
                "Not enough total power to crew".to_string(),
            ));
        }

        let mut events = Vec::new();
        for id in &chosen {
            if game.object(*id).is_some() && !game.is_tapped(*id) {
                game.tap(*id);
                events.push(TriggerEvent::new_with_provenance(
                    PermanentTappedEvent::new(*id),
                    ctx.provenance,
                ));
                events.push(keyword_crew_event(
                    game,
                    *id,
                    ctx.source,
                    controller,
                    ctx.provenance,
                ));
            }
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

        Ok(EffectOutcome::resolved().with_events(events))
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
        _source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if self.required_power == 0 {
            return Ok(());
        }
        let candidates = crew_candidates(game, controller);
        let total: i32 = candidates.iter().map(|id| crew_value(game, *id)).sum();
        if total >= self.required_power as i32
            || can_pay_loyalty_crew_alternative(game, _source, controller)
        {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "Not enough total power to crew".to_string(),
            ))
        }
    }
}
