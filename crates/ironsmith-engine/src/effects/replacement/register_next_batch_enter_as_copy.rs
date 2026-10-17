use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutionCategory, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::zones::matchers::WouldEnterBattlefieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterNextBatchEnterAsCopyEffect = ironsmith_core::RegisterNextBatchEnterAsCopyEffect;

impl EffectExecutor for RegisterNextBatchEnterAsCopyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // The copied object is fixed as the instruction resolves; the later
        // entry uses its copiable values (CR 707.2).
        let copy_source = match resolve_objects_for_effect(game, ctx, &self.copy_source) {
            Ok(ids) => ids.into_iter().next(),
            Err(ExecutionError::InvalidTarget) => None,
            Err(error) => return Err(error),
        };
        let Some(copy_source) = copy_source else {
            return Ok(EffectOutcome::target_invalid());
        };
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterAsCopy {
                source: copy_source,
                enters_tapped: false,
                copy_duration: None,
                linked_exile_objects: Vec::new(),
                additional_counters: Vec::new(),
                name_override: None,
                added_colors: crate::color::ColorSet::new(),
                added_card_types: Vec::new(),
                removes_other_card_types: false,
                added_supertypes: Vec::new(),
                removed_supertypes: Vec::new(),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
                set_base_power_toughness: None,
                copy_followups: Vec::new(),
            },
        );
        game.effect_store
            .replacement_effects
            .add_batch_one_shot_effect(replacement);
        Ok(EffectOutcome::from_status(OutcomeStatus::Succeeded))
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}
