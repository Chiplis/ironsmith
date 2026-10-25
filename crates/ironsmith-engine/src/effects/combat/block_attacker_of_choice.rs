//! "You may have it block an attacking creature of your choice." (False
//! Orders): a creature already on the battlefield starts blocking an attacker
//! chosen as the effect resolves.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::zone::Zone;

pub type BlockAttackerOfChoiceEffect = ironsmith_core::BlockAttackerOfChoiceEffect;

/// Attacking creatures matching `filter` that attack `blocker`'s controller
/// (or a planeswalker or battle that player defends) and that `blocker`
/// isn't already blocking.
fn candidate_attackers(
    game: &GameState,
    ctx: &ExecutionContext,
    blocker: ObjectId,
    filter: &crate::target::ObjectFilter,
) -> Vec<ObjectId> {
    let Some(combat) = game.combat.as_ref() else {
        return Vec::new();
    };
    let Some(controller) = game.object(blocker).map(|object| game.controller_of(object)) else {
        return Vec::new();
    };
    let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
    combat
        .attackers
        .iter()
        .filter(|info| {
            crate::combat_state::defending_player_for_attack_target(game, &info.target)
                == Some(controller)
        })
        .map(|info| info.creature)
        .filter(|attacker| {
            !combat
                .blockers
                .get(attacker)
                .is_some_and(|blockers| blockers.contains(&blocker))
        })
        .filter(|attacker| {
            game.object(*attacker)
                .is_some_and(|object| filter.matches(object, &filter_ctx, game))
        })
        .collect()
}

impl EffectExecutor for BlockAttackerOfChoiceEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        crate::effects::composition::execute_result_transaction(game, ctx, |game, ctx| {
            let blockers = resolve_objects_for_effect(game, ctx, &self.blocker)?;
            let Some(blocker) = blockers.first().copied() else {
                return Ok(EffectOutcome::count(0));
            };
            if !game
                .object(blocker)
                .is_some_and(|object| object.zone == Zone::Battlefield)
                || !game.current_is_creature(blocker)
            {
                return Ok(EffectOutcome::count(0));
            }
            let candidates = candidate_attackers(game, ctx, blocker, &self.attackers);
            if candidates.is_empty() {
                return Ok(EffectOutcome::count(0));
            }
            let spec = crate::decisions::specs::ChooseObjectsSpec::new(
                ctx.source,
                "Choose an attacking creature for it to block",
                candidates.clone(),
                usize::from(!self.optional),
                Some(1),
            )
            .require_explicit_choice();
            let chooser = ctx.controller;
            let source = ctx.source;
            let chosen: Vec<ObjectId> = crate::decisions::make_decision(
                game,
                &mut *ctx.decision_maker,
                chooser,
                Some(source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let Some(attacker) = chosen.into_iter().find(|id| candidates.contains(id)) else {
                return Ok(EffectOutcome::count(0));
            };
            // CR 509.3a-e: an effect making an existing creature block
            // produces its blocking transitions (unlike entering blocking).
            if !super::add_blocker_after_declaration(game, blocker, attacker, false) {
                return Ok(EffectOutcome::count(0));
            }
            Ok(EffectOutcome::count(1).with_affected_objects(vec![blocker]))
        })
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        self.blocker.is_target().then_some(&self.blocker)
    }

    fn target_description(&self) -> &'static str {
        "creature"
    }
}
