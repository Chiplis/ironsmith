//! Crew cost effect implementation.
//!
//! This effect is intended to be used as a COST component for the Crew keyword:
//! "Tap any number of untapped creatures you control with total power N or more".
//!
//! When paid, we also record which creatures crewed the source this turn so
//! later effects/triggers can reference "each creature that crewed it this turn".

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::{CostValidationError, EffectExecutor};
use crate::events::PermanentTappedEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct CrewCostEffect {
    pub required_power: u32,
}

impl CrewCostEffect {
    pub fn new(required_power: u32) -> Self {
        Self { required_power }
    }

    fn crew_candidates(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|&id| {
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

impl EffectExecutor for CrewCostEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller = ctx.controller;
        let mut candidates = Self::crew_candidates(game, controller);
        if candidates.is_empty() && self.required_power > 0 {
            return Err(ExecutionError::Impossible(
                "No untapped creatures available to crew".to_string(),
            ));
        }

        let min = if self.required_power == 0 { 0 } else { 1 };
        let max = Some(candidates.len());
        let chosen = {
            // Prefer higher-power candidates in fallback selection.
            candidates.sort_by_key(|id| -Self::object_power(game, *id));
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
                "Not enough total power to crew".to_string(),
            ));
        }

        let mut events = Vec::new();
        for id in &chosen {
            if game.object(*id).is_some() && !game.is_tapped(*id) {
                game.tap(*id);
                events.push(TriggerEvent::new(PermanentTappedEvent::new(*id)));
            }
        }

        // Record crew contributors for "crewed it this turn" references.
        let entry = game.crewed_this_turn.entry(ctx.source).or_default();
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

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        _source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if self.required_power == 0 {
            return Ok(());
        }
        let candidates = Self::crew_candidates(game, controller);
        let total: i32 = candidates.iter().map(|id| Self::object_power(game, *id)).sum();
        if total >= self.required_power as i32 {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "Not enough total power to crew".to_string(),
            ))
        }
    }

    fn cost_description(&self) -> Option<String> {
        Some(format!(
            "Tap any number of untapped creatures you control with total power {} or more",
            self.required_power
        ))
    }
}
