//! Exile a chosen object, then grant permission to cast or play it from exile.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_single_object_for_effect};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::{GrantDuration, Grantable};
use crate::grant_registry::GrantSource;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExileThenGrantPlayEffect {
    pub target: ChooseSpec,
    pub player: PlayerFilter,
    pub duration: GrantDuration,
}

impl ExileThenGrantPlayEffect {
    pub fn new(target: ChooseSpec, player: PlayerFilter, duration: GrantDuration) -> Self {
        Self {
            target,
            player,
            duration,
        }
    }
}

impl EffectExecutor for ExileThenGrantPlayEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let expires = match self.duration {
            GrantDuration::UntilEndOfTurn => game.turn.turn_number,
            GrantDuration::Forever => u32::MAX,
        };
        let additional_effects = ctx.additional_replacement_effects_snapshot();

        let outcome = crate::effects::zones::apply_zone_change_with_additional_effects(
            game,
            target_id,
            game.object(target_id)
                .map(|obj| obj.zone)
                .ok_or(ExecutionError::ObjectNotFound(target_id))?,
            Zone::Exile,
            crate::events::cause::EventCause::from_effect(ctx.source, ctx.controller),
            ctx.decision_maker,
            &additional_effects,
        );

        let crate::event_processor::EventOutcome::Proceed(result) = outcome else {
            return Ok(EffectOutcome::count(0));
        };
        if result.final_zone != Zone::Exile {
            return Ok(EffectOutcome::count(0));
        }
        let Some(exiled_id) = result.new_object_id else {
            return Ok(EffectOutcome::count(0));
        };

        game.grant_registry.grant_to_card(
            exiled_id,
            Zone::Exile,
            player,
            Grantable::PlayFrom,
            GrantSource::Effect {
                source_id: ctx.source,
                expires_end_of_turn: expires,
            },
        );

        Ok(EffectOutcome::with_objects(vec![exiled_id]))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to exile and grant play permission to"
    }
}
