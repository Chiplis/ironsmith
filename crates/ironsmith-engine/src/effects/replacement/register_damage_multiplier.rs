//! Duration-bound damage replacement created by a resolving spell/ability.
use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbilityKind;
pub type RegisterDamageMultiplierEffect = ironsmith_core::RegisterDamageMultiplierEffect;

/// A resolution-registered replacement outlives the resolution that names
/// "that player" or "that creature" (CR 611.2c, 614.1a): those references are
/// fixed to the objects and players they denote now, so the replacement keeps
/// watching them for its whole duration.
fn freeze_player_reference(
    game: &GameState,
    ctx: &ExecutionContext,
    filter: &crate::target::PlayerFilter,
) -> Result<crate::target::PlayerFilter, ExecutionError> {
    use crate::target::PlayerFilter;
    Ok(match filter {
        // "that player" after "Whenever ... deals combat damage to a player"
        // is the damaged player when no iteration names one.
        PlayerFilter::IteratedPlayer => PlayerFilter::Specific(
            crate::effects::helpers::resolve_player_filter(game, filter, ctx).or_else(|_| {
                crate::effects::helpers::resolve_player_filter(
                    game,
                    &PlayerFilter::DamagedPlayer,
                    ctx,
                )
            })?,
        ),
        PlayerFilter::Target(_)
        | PlayerFilter::AliasedTarget(_)
        | PlayerFilter::TaggedPlayer(_)
        | PlayerFilter::ChosenPlayer
        | PlayerFilter::DamagedPlayer => PlayerFilter::Specific(
            crate::effects::helpers::resolve_player_filter(game, filter, ctx)?,
        ),
        other => other.clone(),
    })
}

fn freeze_object_reference(
    game: &GameState,
    ctx: &ExecutionContext,
    filter: &crate::target::ObjectFilter,
) -> Result<crate::target::ObjectFilter, ExecutionError> {
    // "that creature": one tagged object becomes that exact object.
    if let [constraint] = filter.tagged_constraints.as_slice()
        && let Some(snapshots) = ctx.get_tagged_all(constraint.tag.as_str())
        && let [snapshot] = snapshots.as_slice()
    {
        return Ok(crate::target::ObjectFilter::specific(snapshot.object_id));
    }
    let mut frozen = filter.clone();
    if let Some(controller) = &filter.controller {
        frozen.controller = Some(freeze_player_reference(game, ctx, controller)?);
    }
    Ok(frozen)
}

impl EffectExecutor for RegisterDamageMultiplierEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_filter = freeze_object_reference(game, ctx, &self.source_filter)?;
        let target_player_filter = self
            .target_player_filter
            .as_ref()
            .map(|filter| freeze_player_reference(game, ctx, filter))
            .transpose()?;
        let target_object_filter = self
            .target_object_filter
            .as_ref()
            .map(|filter| freeze_object_reference(game, ctx, filter))
            .transpose()?;
        if let Some(modifier) = self.amount_override {
            // A set or halved amount, through the shared amount-modifier
            // replacement (CR 616.1 ordering with any other modifiers).
            let replacement = crate::static_abilities::EventAmountReplacement::new(
                ironsmith_core::AmountEventSpec::Damage {
                    source_filter: Some(source_filter),
                    player: target_player_filter,
                    object: target_object_filter,
                    combat_only: self.combat_only,
                    minimum: self.minimum,
                },
                modifier,
                false,
                "Resolved damage amount replacement",
            )
            .generate_replacement_effect(ctx.source, ctx.controller)
            .expect("an amount replacement always creates a replacement");
            return ApplyReplacementEffect {
                effect: replacement,
                mode: self.mode,
            }
            .execute_child(game, ctx);
        }
        let mut ability = crate::static_abilities::DoubleDamageAmountReplacement::new(
            source_filter,
            target_player_filter,
            target_object_filter,
            self.factor,
            self.combat_only,
            "Resolved damage multiplier",
        );
        if self.noncombat_only {
            ability = ability.noncombat_only();
        }
        let replacement = ability
            .generate_replacement_effect(ctx.source, ctx.controller)
            .expect("damage multiplier always creates a replacement");
        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute_child(game, ctx)
    }
    fn primary_execution_category(&self) -> crate::effects::EffectExecutionCategory {
        crate::effects::EffectExecutionCategory::ReplacementRegistration
    }
}
