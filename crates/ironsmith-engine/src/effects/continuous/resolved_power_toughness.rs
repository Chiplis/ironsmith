//! Frozen values for resolution-created power/toughness effects only.
//! Counter placement, static abilities, copy and transform keep their own owners.
use crate::continuous::{EffectTarget, Modification, PtSublayer};
use crate::effect::{Until, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{ApplyContinuousEffect, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub(crate) struct ResolvedPowerToughness {
    power: i32,
    toughness: i32,
}

impl ResolvedPowerToughness {
    pub(crate) fn resolve(
        game: &GameState,
        ctx: &ExecutionContext,
        power: &Value,
        toughness: &Value,
    ) -> Result<Self, ExecutionError> {
        Ok(Self {
            power: resolve_value(game, power, ctx)?,
            toughness: resolve_value(game, toughness, ctx)?,
        })
    }

    /// "Gets +X/+Y": per CR 107.1b a calculated amount that would be
    /// negative is zero (a literal or explicitly negated amount keeps its sign).
    pub(crate) fn resolve_modifier(
        game: &GameState,
        ctx: &ExecutionContext,
        power: &Value,
        toughness: &Value,
    ) -> Result<Self, ExecutionError> {
        fn amount(
            game: &GameState,
            ctx: &ExecutionContext,
            value: &Value,
        ) -> Result<i32, ExecutionError> {
            Ok(match value {
                Value::SurfaceHinted { value, .. } => amount(game, ctx, value)?,
                Value::Fixed(n) => *n,
                Value::Add(_, _) | Value::XTimes(_) => resolve_value(game, value, ctx)?,
                Value::Scaled(inner, factor) => {
                    resolve_value(game, inner, ctx)?.max(0).saturating_mul(*factor)
                }
                _ => resolve_value(game, value, ctx)?.max(0),
            })
        }
        Ok(Self {
            power: amount(game, ctx, power)?,
            toughness: amount(game, ctx, toughness)?,
        })
    }

    pub(crate) fn scaled(
        game: &GameState,
        ctx: &ExecutionContext,
        power: i32,
        toughness: i32,
        count: &Value,
    ) -> Result<Self, ExecutionError> {
        let count = resolve_value(game, count, ctx)?;
        Ok(Self {
            power: power * count,
            toughness: toughness * count,
        })
    }

    pub(crate) fn additive(self, target: EffectTarget, duration: Until) -> ApplyContinuousEffect {
        ApplyContinuousEffect::new(
            target,
            Modification::ModifyPowerToughness {
                power: self.power,
                toughness: self.toughness,
            },
            duration,
        )
    }

    pub(crate) fn setting(self, target: EffectTarget, duration: Until) -> ApplyContinuousEffect {
        ApplyContinuousEffect::new(
            target,
            Modification::SetPowerToughness {
                power: Value::Fixed(self.power),
                toughness: Value::Fixed(self.toughness),
                sublayer: PtSublayer::Setting,
            },
            duration,
        )
    }
}
