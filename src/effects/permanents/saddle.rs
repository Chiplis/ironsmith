//! Saddle cost/effect implementation.
//!
//! Comprehensive Rules reference (as of Jan 16, 2026):
//! - 702.171a: "Saddle N" means "Tap any number of other untapped creatures you control
//!   with total power N or greater: This permanent becomes saddled until end of turn.
//!   Activate only as a sorcery."
//! - 702.171b: A creature "saddles" a permanent as it's tapped to pay that cost.
//!
//! We model this similarly to Crew:
//! - The tap/selection is an effect-backed COST component (`SaddleCostEffect`).
//! - The "becomes saddled until end of turn" is a resolution effect
//!   (`BecomeSaddledUntilEotEffect`) that marks game state.
//! - We record which creatures saddled the source this turn for filters like
//!   "that saddled it this turn".

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::events::PermanentTappedEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct SaddleCostEffect {
    pub required_power: u32,
}

impl SaddleCostEffect {
    pub fn new(required_power: u32) -> Self {
        Self { required_power }
    }

    fn saddle_candidates(
        game: &GameState,
        controller: PlayerId,
        source: ObjectId,
    ) -> Vec<ObjectId> {
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
                obj.is_creature() && obj.controller == controller && !game.is_tapped(id)
            })
            .collect()
    }

    fn object_power(game: &GameState, object_id: ObjectId) -> i32 {
        game.calculated_characteristics(object_id)
            .and_then(|calc| calc.power)
            .or_else(|| game.object(object_id).and_then(|obj| obj.power()))
            .unwrap_or(0)
    }
}

impl EffectExecutor for SaddleCostEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller = ctx.controller;
        let source = ctx.source;
        let mut candidates = Self::saddle_candidates(game, controller, source);
        if candidates.is_empty() && self.required_power > 0 {
            return Err(ExecutionError::Impossible(
                "No untapped creatures available to saddle".to_string(),
            ));
        }

        let min = if self.required_power == 0 { 0 } else { 1 };
        let max = Some(candidates.len());
        let chosen = {
            // Prefer higher-power candidates in fallback selection.
            candidates.sort_by_key(|id| -Self::object_power(game, *id));
            let spec = ChooseObjectsSpec::new(
                source,
                "Choose other creatures to saddle",
                candidates.clone(),
                min,
                max,
            );
            make_decision(game, ctx.decision_maker, controller, Some(source), spec)
        };

        let mut chosen = chosen;
        chosen.sort();
        chosen.dedup();

        // If the decision maker picked a set that doesn't meet the requirement,
        // greedily add remaining candidates until it does (or we exhaust options).
        let required = self.required_power as i32;
        let mut total_power: i32 = chosen.iter().map(|id| Self::object_power(game, *id)).sum();
        if total_power < required {
            let mut remaining: Vec<ObjectId> = candidates
                .iter()
                .copied()
                .filter(|id| !chosen.contains(id))
                .collect();
            remaining.sort_by_key(|id| -Self::object_power(game, *id));
            for id in remaining {
                if total_power >= required {
                    break;
                }
                chosen.push(id);
                total_power += Self::object_power(game, id);
            }
        }

        if total_power < required {
            return Err(ExecutionError::Impossible(
                "Not enough total power to saddle".to_string(),
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
            }
        }

        // Record saddle contributors for "saddled it this turn" references.
        let entry = game.saddled_this_turn.entry(source).or_default();
        for id in chosen {
            if !entry.contains(&id) {
                entry.push(id);
            }
        }

        Ok(EffectOutcome {
            result: EffectResult::Resolved,
            events,
        })
    }

    fn cost_description(&self) -> Option<String> {
        Some(format!(
            "Tap any number of other untapped creatures you control with total power {} or more",
            self.required_power
        ))
    }
}

impl CostExecutableEffect for SaddleCostEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if self.required_power == 0 {
            return Ok(());
        }
        let candidates = Self::saddle_candidates(game, controller, source);
        let total: i32 = candidates
            .iter()
            .map(|id| Self::object_power(game, *id))
            .sum();
        if total >= self.required_power as i32 {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "Not enough total power to saddle".to_string(),
            ))
        }
    }
}

/// Effect that marks the source permanent as saddled until end of turn.
#[derive(Debug, Clone, PartialEq)]
pub struct BecomeSaddledUntilEotEffect;

impl BecomeSaddledUntilEotEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for BecomeSaddledUntilEotEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.set_saddled_until_end_of_turn(ctx.source);
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }
}
