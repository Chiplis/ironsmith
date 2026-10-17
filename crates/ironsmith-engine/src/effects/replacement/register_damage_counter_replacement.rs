//! "For each 1 damage that would be dealt to you until your next upkeep, you
//! remove an echo counter from this enchantment instead." (Soul Echo)
//!
//! A resolving spell or ability registers a replacement of damage that would
//! be dealt to a player (CR 614.1a): that many counters of one kind are
//! removed from a fixed object instead, for the registered duration.
use crate::effect::{Effect, EffectOutcome, EventValueSpec, Value};
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::damage::matchers::DamageToPlayerMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, PlayerFilter};
pub type RegisterDamageCounterReplacementEffect =
    ironsmith_core::RegisterDamageCounterReplacementEffect;
pub use ironsmith_core::RegisteredReplacementExpiry;

impl EffectExecutor for RegisterDamageCounterReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // The replacement outlives this resolution: "you" and "this
        // enchantment" denote what they denote now (CR 611.2c).
        let player = match &self.player {
            PlayerFilter::Any | PlayerFilter::Opponent => self.player.clone(),
            filter => PlayerFilter::Specific(crate::effects::helpers::resolve_player_filter(
                game, filter, ctx,
            )?),
        };
        let object = crate::effects::helpers::resolve_objects_from_spec(game, &self.object, ctx)
            .ok()
            .and_then(|objects| objects.first().copied())
            .unwrap_or(ctx.source);
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            DamageToPlayerMatcher::new(player),
            ReplacementAction::Instead(vec![Effect::remove_counters(
                self.counter_type,
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::SpecificObject(object),
            )]),
        );
        ApplyReplacementEffect {
            effect: replacement,
            mode: self.expiry.apply_mode(),
        }
        .execute_child(game, ctx)
    }

    fn primary_execution_category(&self) -> crate::effects::EffectExecutionCategory {
        crate::effects::EffectExecutionCategory::ReplacementRegistration
    }
}
