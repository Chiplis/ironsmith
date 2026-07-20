//! Register a one-shot spell-cost reduction for the next matching spell this turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter_to_list, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub type GrantNextSpellCostReductionEffect = ironsmith_core::GrantNextSpellCostReductionEffect;

impl EffectExecutor for GrantNextSpellCostReductionEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let players =
            resolve_player_filter_to_list(game, &self.player, &ctx.filter_context(game), ctx)?;
        if self.applies_to_all_matching_this_turn {
            let amount = self
                .generic_reduction
                .as_ref()
                .map(|value| resolve_value(game, value, ctx))
                .transpose()?
                .unwrap_or(0)
                .max(0);
            for player in players {
                let mut filter = self.filter.clone();
                lock_target_player_filters_for_player(&mut filter, player);
                game.add_temporary_matching_spell_cost_reduction_until(
                    player,
                    ctx.source,
                    ctx.controller,
                    filter,
                    crate::effect::Value::Fixed(amount),
                    self.duration.clone(),
                );
            }
        } else {
            for player in players {
                let mut filter = self.filter.clone();
                lock_target_player_filters_for_player(&mut filter, player);
                game.add_temporary_spell_cost_reduction_until(
                    player,
                    ctx.source,
                    ctx.controller,
                    filter,
                    self.reduction.clone(),
                    1,
                    self.duration.clone(),
                );
            }
        }
        Ok(EffectOutcome::resolved())
    }
}

fn lock_target_player_filters_for_player(
    filter: &mut crate::target::ObjectFilter,
    player: PlayerId,
) {
    if let Some(controller) = &mut filter.controller {
        lock_target_player_filter(controller, player);
    }
    if let Some(owner) = &mut filter.owner {
        lock_target_player_filter(owner, player);
    }
    if let Some(cast_by) = &mut filter.cast_by {
        lock_target_player_filter(cast_by, player);
    }
    if let Some(targets_player) = &mut filter.targets_player {
        lock_target_player_filter(targets_player, player);
    }
    if let Some(targets_only_player) = &mut filter.targets_only_player {
        lock_target_player_filter(targets_only_player, player);
    }
    if let Some(entered_controller) = &mut filter.entered_battlefield_controller {
        lock_target_player_filter(entered_controller, player);
    }
    if let Some(attached_to_player) = &mut filter.attached_to_player {
        lock_target_player_filter(attached_to_player, player);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        lock_target_player_filters_for_player(attached_to, player);
    }
    for nested in &mut filter.any_of {
        lock_target_player_filters_for_player(nested, player);
    }
}

fn lock_target_player_filter(filter: &mut crate::target::PlayerFilter, player: PlayerId) {
    match filter {
        crate::target::PlayerFilter::Target(_) | crate::target::PlayerFilter::AliasedTarget(_) => {
            *filter = crate::target::PlayerFilter::Specific(player);
        }
        crate::target::PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
        | crate::target::PlayerFilter::HasMoreLifeThanYou { base }
        | crate::target::PlayerFilter::MaxSpeed { base, .. } => {
            lock_target_player_filter(base, player);
        }
        crate::target::PlayerFilter::Excluding { base, excluded } => {
            lock_target_player_filter(base, player);
            lock_target_player_filter(excluded, player);
        }
        _ => {}
    }
}
