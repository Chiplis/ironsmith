//! Return to hand effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, OutcomeStatus};
use crate::effects::helpers::{
    ObjectApplyResultPolicy, apply_single_target_object_from_spec, apply_to_selected_objects,
};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::event_processor::EventOutcome;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

use super::apply_zone_change_with_additional_effects;

/// Effect that returns permanents to their owners' hands.
///
/// This is commonly called "bouncing" in MTG terminology.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Return target creature to its owner's hand (targeted - can fizzle)
/// let effect = ReturnToHandEffect::target(ChooseSpec::creature());
///
/// // Return all creatures to their owners' hands (non-targeted - cannot fizzle)
/// let effect = ReturnToHandEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnToHandEffect {
    /// What to return - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl ReturnToHandEffect {
    /// Create a return to hand effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted return to hand effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted return to hand effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted return to hand effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create a return to hand effect targeting any creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create a return to hand effect targeting any permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Create an effect that returns all creatures.
    pub fn creatures() -> Self {
        Self::all(ObjectFilter::creature())
    }

    /// Create an effect that returns all nonland permanents.
    pub fn nonland_permanents() -> Self {
        Self::all(ObjectFilter::nonland_permanent())
    }

    /// Helper to return a single object to hand (shared logic).
    fn return_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
    ) -> Result<Option<OutcomeStatus>, ExecutionError> {
        if let Some(obj) = game.object(object_id) {
            let from_zone = obj.zone;
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            // Process through replacement effects with decision maker.
            let result = apply_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                Zone::Hand,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match result {
                EventOutcome::Prevented => {
                    return Ok(Some(crate::effect::OutcomeStatus::Prevented));
                }
                EventOutcome::Proceed(_) => {
                    return Ok(None); // Successfully returned
                }
                EventOutcome::Replaced => {
                    // Replacement effects already executed
                    return Ok(Some(crate::effect::OutcomeStatus::Replaced));
                }
                EventOutcome::NotApplicable => {
                    return Ok(Some(crate::effect::OutcomeStatus::TargetInvalid));
                }
            }
        }
        // Object doesn't exist - target is invalid
        Ok(Some(crate::effect::OutcomeStatus::TargetInvalid))
    }
}

impl EffectExecutor for ReturnToHandEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        if self.spec.is_target() && self.spec.is_single() {
            return apply_single_target_object_from_spec(
                game,
                ctx,
                &self.spec,
                |game, ctx, object_id| Self::return_object(game, ctx, object_id),
            );
        }

        // For all/multi-target effects, count successful moves to hand.
        let apply_result = match apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            ObjectApplyResultPolicy::CountApplied,
            |game, ctx, object_id| {
                let Some(from_zone) = game.object(object_id).map(|obj| obj.zone) else {
                    return Ok(false);
                };
                let additional_effects = ctx.additional_replacement_effects_snapshot();
                match apply_zone_change_with_additional_effects(
                    game,
                    object_id,
                    from_zone,
                    Zone::Hand,
                    ctx.cause.clone(),
                    &mut ctx.decision_maker,
                    &additional_effects,
                ) {
                    EventOutcome::Proceed(result) => Ok(result.new_object_id.is_some()),
                    EventOutcome::Prevented
                    | EventOutcome::Replaced
                    | EventOutcome::NotApplicable => Ok(false),
                }
            },
        ) {
            Ok(result) => result,
            Err(_) => return Ok(EffectOutcome::target_invalid()),
        };

        Ok(apply_result.outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to return"
    }

    fn cost_description(&self) -> Option<String> {
        match self.spec.base() {
            ChooseSpec::Source => Some("Return ~ to its owner's hand".to_string()),
            ChooseSpec::Object(filter) => Some(format!(
                "Return a {} you control to its owner's hand",
                filter.description()
            )),
            _ => None,
        }
    }
}

impl CostExecutableEffect for ReturnToHandEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        match self.spec.base() {
            ChooseSpec::Source => {
                if game
                    .object(source)
                    .is_some_and(|obj| obj.zone == Zone::Battlefield)
                {
                    Ok(())
                } else {
                    Err(crate::effects::CostValidationError::Other(
                        "source must be on the battlefield".to_string(),
                    ))
                }
            }
            ChooseSpec::Object(filter) => {
                let filter_ctx = crate::filter::FilterContext::new(controller).with_source(source);
                let available = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| {
                        game.object(*id)
                            .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
                    })
                    .count();
                if available == 0 {
                    Err(crate::effects::CostValidationError::Other(
                        "no valid return target".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            _ => Err(crate::effects::CostValidationError::Other(
                "unsupported return-to-hand cost".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;

    struct SelectIdsDecisionMaker {
        chosen: Vec<ObjectId>,
    }

    impl DecisionMaker for SelectIdsDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.chosen
                .iter()
                .copied()
                .filter(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *id)
                })
                .collect()
        }
    }

    fn add_land(game: &mut GameState, card_id: u32, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn growth_chamber_style_bounce_can_choose_the_source_land() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let growth_chamber = add_land(&mut game, 561, "Simic Growth Chamber", alice);
        let forest = add_land(&mut game, 562, "Forest", alice);
        let mut dm = SelectIdsDecisionMaker {
            chosen: vec![growth_chamber],
        };
        let mut ctx =
            ExecutionContext::new_default(growth_chamber, alice).with_decision_maker(&mut dm);
        let effect = ReturnToHandEffect::with_spec(
            ChooseSpec::Object(ObjectFilter::land().you_control())
                .with_count(ChoiceCount::exactly(1)),
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("bounce effect should resolve");
        let bounced_card_in_hand = game.players[0].hand.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Simic Growth Chamber")
        });

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert!(bounced_card_in_hand);
        assert!(!game.battlefield.contains(&growth_chamber));
        assert!(game.battlefield.contains(&forest));
    }
}
