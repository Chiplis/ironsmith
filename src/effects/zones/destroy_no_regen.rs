//! Destroy effect implementation that ignores regeneration.
//!
//! Used for oracle text like:
//! - "Destroy target creature. It can't be regenerated."
//! - "Destroy all creatures. They can't be regenerated."

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    ObjectApplyResultPolicy, apply_single_target_object_from_context, apply_to_selected_objects,
};
use crate::event_processor::{EventOutcome, process_destroy};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};

/// Effect that destroys permanents while ignoring regeneration shields.
///
/// This matches "can't be regenerated" tails on destroy effects: regeneration shields
/// don't replace the destruction event.
#[derive(Debug, Clone, PartialEq)]
pub struct DestroyNoRegenerationEffect {
    /// What to destroy - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl DestroyNoRegenerationEffect {
    /// Create a destroy-no-regeneration effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted destroy-no-regeneration effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted destroy-no-regeneration effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted destroy-no-regeneration effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    fn destroy_object_no_regen(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
    ) -> Result<Option<EffectResult>, ExecutionError> {
        // Regeneration shields are one-shot replacement effects; "can't be regenerated"
        // means they can't replace this destruction.
        //
        // We clear both:
        // - trait-based one-shot replacement effects (current regeneration implementation)
        // - legacy shield counters (older implementation)
        game.replacement_effects
            .remove_one_shot_effects_from_source(object_id);
        game.clear_regeneration_shields(object_id);

        let result = process_destroy(game, object_id, Some(ctx.source), &mut ctx.decision_maker);
        match result {
            EventOutcome::Proceed(_) => Ok(None),
            EventOutcome::Prevented => Ok(Some(EffectResult::Protected)),
            EventOutcome::Replaced => Ok(Some(EffectResult::Replaced)),
            EventOutcome::NotApplicable => Ok(Some(EffectResult::TargetInvalid)),
        }
    }
}

impl EffectExecutor for DestroyNoRegenerationEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.spec.is_target() && self.spec.is_single() {
            return apply_single_target_object_from_context(game, ctx, |game, ctx, object_id| {
                Self::destroy_object_no_regen(game, ctx, object_id)
            });
        }

        let apply_result = match apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            ObjectApplyResultPolicy::CountApplied,
            |game, ctx, object_id| {
                game.replacement_effects
                    .remove_one_shot_effects_from_source(object_id);
                game.clear_regeneration_shields(object_id);
                let result =
                    process_destroy(game, object_id, Some(ctx.source), &mut ctx.decision_maker);
                Ok(matches!(result, EventOutcome::Proceed(_)))
            },
        ) {
            Ok(result) => result,
            Err(_) => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        Ok(EffectOutcome::from_result(EffectResult::Count(
            apply_result.applied_count as i32,
        )))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.spec)
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        Some(self.spec.count())
    }

    fn target_description(&self) -> &'static str {
        "permanent to destroy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::RegenerateEffect;
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::ids::ObjectId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn destroy_no_regeneration_ignores_regeneration_shields() {
        let mut game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Shielded Bear")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
            .build();
        let creature_id: ObjectId =
            game.create_object_from_card(&creature_card, bob, Zone::Battlefield);

        // Apply regeneration via the proper effect (creates replacement effect).
        let mut regen_ctx = ExecutionContext::new_default(creature_id, bob);
        RegenerateEffect::source(crate::effect::Until::EndOfTurn)
            .execute(&mut game, &mut regen_ctx)
            .unwrap();
        assert!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id)
                > 0
        );

        let effect = DestroyNoRegenerationEffect::target(ChooseSpec::creature());
        let mut ctx =
            ExecutionContext::new_default(game.new_object_id(), alice).with_targets(vec![
                ResolvedTarget::Object(creature_id),
            ]);

        let out = effect.execute(&mut game, &mut ctx).expect("execute");
        assert!(
            out.result.is_success(),
            "expected destroy to succeed, got {:?}",
            out.result
        );
        assert!(
            game.object(creature_id).is_none(),
            "expected creature to be destroyed"
        );
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            0
        );
    }
}
