//! "Creatures it was blocking that had become blocked by only this creature
//! this combat become unblocked." (Ydwen Efreet)

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;

pub type UnblockSoleBlockedAttackersEffect = ironsmith_core::UnblockSoleBlockedAttackersEffect;

/// Attackers whose only blocker this combat was `blocker`, which no longer
/// blocks them. Declarations this combat come from turn history, so a blocker
/// already removed from combat (CR 506.4) still identifies what it blocked.
fn attackers_blocked_only_by(game: &GameState, blocker: ObjectId) -> Vec<ObjectId> {
    let Some(combat) = game.combat.as_ref() else {
        return Vec::new();
    };
    let combat_phase = game.turn_store.combat_phases_started_this_turn;
    combat
        .attackers
        .iter()
        .map(|info| info.creature)
        .filter(|attacker| crate::combat_state::is_blocked(combat, *attacker))
        .filter(|attacker| {
            let current = combat.blockers.get(attacker).cloned().unwrap_or_default();
            if current.contains(&blocker) {
                return false;
            }
            let mut ever = game
                .turn_store
                .turn_history
                .blockers_of_in_combat(*attacker, combat_phase);
            ever.extend(current);
            ever.sort_unstable();
            ever.dedup();
            ever == [blocker]
        })
        .collect()
}

impl EffectExecutor for UnblockSoleBlockedAttackersEffect {
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
            let mut unblocked = Vec::new();
            for blocker in blockers {
                unblocked.extend(attackers_blocked_only_by(game, blocker));
            }
            unblocked.sort_unstable();
            unblocked.dedup();
            if unblocked.is_empty() {
                return Ok(EffectOutcome::count(0));
            }
            let Some(combat) = game.combat.as_mut() else {
                return Ok(EffectOutcome::count(0));
            };
            for attacker in &unblocked {
                combat.blocked_attackers.remove(attacker);
                combat.blockers.insert(*attacker, Vec::new());
                combat.damage_assignment_order.remove(attacker);
            }
            game.mark_continuous_state_dirty();
            Ok(EffectOutcome::count(unblocked.len() as i32))
        })
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        self.blocker.is_target().then_some(&self.blocker)
    }

    fn target_description(&self) -> &'static str {
        "blocking creature"
    }
}
