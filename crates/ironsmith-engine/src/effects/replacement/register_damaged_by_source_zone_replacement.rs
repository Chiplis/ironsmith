use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::{EffectExecutionCategory, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith_core::DamagedBySource;

pub type RegisterDamagedBySourceZoneReplacementEffect =
    ironsmith_core::RegisterDamagedBySourceZoneReplacementEffect;

impl EffectExecutor for RegisterDamagedBySourceZoneReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut matcher = crate::events::zones::matchers::WouldDieDamagedBySourceThisTurnMatcher::new(
            self.filter.clone(),
            DamagedBySource::ThisCreature,
        );
        let victims = permanents_damaged_by_this_resolution(game, ctx);
        if !victims.is_empty() {
            matcher = matcher.with_victims(victims);
        }
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            matcher,
            ReplacementAction::ChangeDestination(self.replacement_zone),
        );

        match self.mode {
            crate::effects::ReplacementApplyMode::OneShot => {
                game.effect_store
                    .replacement_effects
                    .add_one_shot_effect(replacement);
            }
            crate::effects::ReplacementApplyMode::UntilEndOfTurn => {
                game.effect_store
                    .replacement_effects
                    .add_until_end_of_turn_effect(replacement);
            }
            crate::effects::ReplacementApplyMode::UntilYourNextTurn => {
                game.effect_store
                    .replacement_effects
                    .add_until_next_turn_effect(replacement, ctx.controller, game.turn.turn_number);
            }
            crate::effects::ReplacementApplyMode::Resolution => {
                game.effect_store
                    .replacement_effects
                    .add_resolution_effect(replacement);
            }
        }

        Ok(EffectOutcome::from_status(OutcomeStatus::Succeeded))
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}

/// "If a creature dealt damage this way would die this turn" (Chandra,
/// Awakened Inferno; Annihilating Fire) names the permanents this resolution
/// dealt damage to, not everything the source damaged earlier in the turn.
/// Damage events an instruction of this resolution produced carry provenance
/// descending from the resolution's node. An empty result (no traceable
/// damage) keeps the broader "damaged by this source this turn" reading.
fn permanents_damaged_by_this_resolution(
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<crate::ids::StableId> {
    let mut victims = Vec::new();
    if ctx.provenance == crate::provenance::ProvNodeId::default() {
        return victims;
    }
    let source_stable_id = game.object(ctx.source).map(|obj| obj.stable_id);
    let graph = game.provenance_graph();
    for record in game.turn_store.turn_history.projected_records() {
        let Some(damage) = record.event.downcast::<crate::events::DamageEvent>() else {
            continue;
        };
        let crate::events::DamageTarget::Object(target) = damage.target else {
            continue;
        };
        if damage.amount == 0 || !graph.is_descendant_of(record.event.provenance(), ctx.provenance)
        {
            continue;
        }
        let source_matches = damage.source == ctx.source
            || source_stable_id.is_some_and(|stable_id| {
                record
                    .source_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.stable_id == stable_id)
            });
        if !source_matches {
            continue;
        }
        let stable_id = damage
            .target_snapshot
            .as_ref()
            .map(|snapshot| snapshot.stable_id)
            .or_else(|| game.object(target).map(|obj| obj.stable_id));
        if let Some(stable_id) = stable_id
            && !victims.contains(&stable_id)
        {
            victims.push(stable_id);
        }
    }
    victims
}
