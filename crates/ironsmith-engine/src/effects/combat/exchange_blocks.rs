//! Two blocking creatures trade the attackers they block (Sorrow's Path).

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;

pub type ExchangeBlocksEffect = ironsmith_core::ExchangeBlocksEffect;

fn attackers_blocked_by(game: &GameState, blocker: ObjectId) -> Vec<ObjectId> {
    game.combat
        .as_ref()
        .map(|combat| {
            combat
                .blockers
                .iter()
                .filter(|(_, blockers)| blockers.contains(&blocker))
                .map(|(attacker, _)| *attacker)
                .collect()
        })
        .unwrap_or_default()
}

/// Could `blocker` block every one of `attackers` (evasion, restrictions,
/// protection), judged one attacker at a time?
fn could_block_all(game: &GameState, blocker: ObjectId, attackers: &[ObjectId]) -> bool {
    let Some(blocker) = game.object(blocker) else {
        return false;
    };
    attackers.iter().all(|attacker| {
        game.object(*attacker)
            .is_some_and(|attacker| crate::rules::combat::can_block(attacker, blocker, game))
    })
}

fn swap_in(list: &mut [ObjectId], first: ObjectId, second: ObjectId) {
    for id in list.iter_mut() {
        if *id == first {
            *id = second;
        } else if *id == second {
            *id = first;
        }
    }
}

impl EffectExecutor for ExchangeBlocksEffect {
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
            let creatures = resolve_objects_for_effect(game, ctx, &self.target)?;
            let [first, second] = creatures.as_slice() else {
                return Ok(EffectOutcome::count(0));
            };
            let (first, second) = (*first, *second);
            if first == second {
                return Ok(EffectOutcome::count(0));
            }
            let first_blocks = attackers_blocked_by(game, first);
            let second_blocks = attackers_blocked_by(game, second);
            // Both must still be blocking, and each must be able to block
            // everything the other blocks; otherwise nothing happens.
            if first_blocks.is_empty()
                || second_blocks.is_empty()
                || !could_block_all(game, first, &second_blocks)
                || !could_block_all(game, second, &first_blocks)
            {
                return Ok(EffectOutcome::count(0));
            }
            let Some(combat) = game.combat.as_mut() else {
                return Ok(EffectOutcome::count(0));
            };
            for blockers in combat.blockers.values_mut() {
                swap_in(blockers, first, second);
                // A creature both were blocking keeps one entry for each.
                let mut seen = std::collections::HashSet::new();
                blockers.retain(|id| seen.insert(*id));
            }
            for order in combat.damage_assignment_order.values_mut() {
                swap_in(order, first, second);
                let mut seen = std::collections::HashSet::new();
                order.retain(|id| seen.insert(*id));
            }
            game.mark_continuous_state_dirty();
            Ok(EffectOutcome::count(2))
        })
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "blocking creature"
    }
}
