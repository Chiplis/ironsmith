use crate::alternative_cast::CastingMethod;
use crate::decisions::context::BooleanContext;
use crate::effect::{ChoiceCount, Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::filter::{Comparison, ObjectFilter};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct SavinesReclamationEffect {
    pub target: ChooseSpec,
    mode: SavinesReclamationMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SavinesReclamationMode {
    Full,
    CopyIfCastFromGraveyard,
}

impl SavinesReclamationEffect {
    pub fn new() -> Self {
        Self {
            target: Self::target_spec(),
            mode: SavinesReclamationMode::Full,
        }
    }

    pub fn copy_if_cast_from_graveyard() -> Self {
        Self {
            target: Self::target_spec(),
            mode: SavinesReclamationMode::CopyIfCastFromGraveyard,
        }
    }

    fn target_filter() -> ObjectFilter {
        ObjectFilter::permanent_card()
            .in_zone(Zone::Graveyard)
            .owned_by(crate::target::PlayerFilter::You)
            .with_mana_value(Comparison::LessThanOrEqual(3))
    }

    fn target_spec() -> ChooseSpec {
        ChooseSpec::target(ChooseSpec::Object(Self::target_filter()))
    }

    fn was_cast_from_graveyard(game: &GameState, ctx: &ExecutionContext) -> bool {
        match &ctx.casting_method {
            CastingMethod::GrantedFlashback | CastingMethod::GrantedEscape { .. } => true,
            CastingMethod::PlayFrom { zone, .. } => *zone == Zone::Graveyard,
            CastingMethod::Alternative(idx) => game
                .object(ctx.source)
                .and_then(|obj| obj.alternative_casts.get(*idx))
                .is_some_and(|method| method.cast_from_zone() == Zone::Graveyard),
            CastingMethod::Normal
            | CastingMethod::FaceDown
            | CastingMethod::SplitOtherHalf
            | CastingMethod::Fuse => false,
        }
    }
}

impl EffectExecutor for SavinesReclamationEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let return_effect =
            Effect::return_from_graveyard_to_battlefield(self.target.clone(), false);
        let first = match self.mode {
            SavinesReclamationMode::Full => execute_effect(game, &return_effect, ctx)?,
            SavinesReclamationMode::CopyIfCastFromGraveyard => EffectOutcome::resolved(),
        };

        if !Self::was_cast_from_graveyard(game, ctx) {
            return Ok(first);
        }

        let candidates = game
            .player(ctx.controller)
            .map(|player| {
                player
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|id| {
                        game.object(*id).is_some_and(|obj| {
                            Self::target_filter().matches(
                                obj,
                                &game.filter_context_for(ctx.controller, Some(ctx.source)),
                                game,
                            )
                        })
                    })
                    .count()
            })
            .unwrap_or(0);
        if candidates == 0 {
            return Ok(first);
        }

        let copy_ctx = BooleanContext::new(
            ctx.controller,
            Some(ctx.source),
            "Copy Savine's Reclamation?",
        );
        let should_copy = ctx.decision_maker.decide_boolean(game, &copy_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if !should_copy {
            return Ok(first);
        }

        let copy_choice_spec =
            ChooseSpec::Object(Self::target_filter()).with_count(ChoiceCount::exactly(1));
        let chosen = resolve_objects_for_effect(game, ctx, &copy_choice_spec)?;
        let Some(chosen_id) = chosen.first().copied() else {
            return Ok(first);
        };

        let second = ctx.with_temp_targets(vec![ResolvedTarget::Object(chosen_id)], |ctx| {
            execute_effect(game, &return_effect, ctx)
        })?;

        Ok(EffectOutcome::aggregate(vec![first, second]))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        match self.mode {
            SavinesReclamationMode::Full => Some(&self.target),
            SavinesReclamationMode::CopyIfCastFromGraveyard => None,
        }
    }
}
