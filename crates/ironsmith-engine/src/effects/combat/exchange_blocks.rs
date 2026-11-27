//! Two blocking creatures trade the attackers they block (Sorrow's Path).

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;

pub type ExchangeBlocksEffect = ironsmith_core::ExchangeBlocksEffect;

fn attackers_blocked_by(game: &GameState, blocker: ObjectId) -> Vec<ObjectId> {
    game.combat_lanes()
        .flat_map(|combat| combat.blockers.iter())
        .filter(|(_, blockers)| blockers.contains(&blocker))
        .map(|(attacker, _)| *attacker)
        .collect()
}

/// Resolve "could block" against the defending side of this attack before
/// checking evasion, protection and other blocking restrictions.
fn could_block(game: &GameState, attacker: ObjectId, blocker: ObjectId) -> bool {
    let (Some(combat), Some(blocker), Some(attacking_object)) = (
        game.combat_for_attacker(attacker),
        game.object(blocker),
        game.object(attacker),
    ) else {
        return false;
    };
    crate::combat_state::controller_can_block_in_combat(
        game,
        combat,
        attacker,
        game.controller_of(blocker),
    ) && crate::rules::combat::can_block(attacking_object, blocker, game)
}

/// General Jarkeld: two blocked attackers trade every creature blocking
/// exactly one of them, provided each could be blocked by all creatures the
/// other is blocked by. A creature blocking both keeps blocking both. No
/// blocker is declared again and both attackers stay blocked (CR 509.1h).
fn exchange_attackers_blockers(
    game: &mut GameState,
    first: ObjectId,
    second: ObjectId,
) -> Result<EffectOutcome, ExecutionError> {
    let Some(combat) = game.combat_for_attacker(first) else {
        return Ok(EffectOutcome::count(0));
    };
    let attacking = |id: ObjectId| combat.attackers.iter().any(|info| info.creature == id);
    if !attacking(first)
        || !attacking(second)
        || !combat.blocked_attackers.contains(&first)
        || !combat.blocked_attackers.contains(&second)
    {
        return Ok(EffectOutcome::count(0));
    }
    let first_blockers = combat.blockers.get(&first).cloned().unwrap_or_default();
    let second_blockers = combat.blockers.get(&second).cloned().unwrap_or_default();
    if !second_blockers
        .iter()
        .all(|blocker| could_block(game, first, *blocker))
        || !first_blockers
            .iter()
            .all(|blocker| could_block(game, second, *blocker))
    {
        return Ok(EffectOutcome::count(0));
    }
    let only_first = first_blockers
        .iter()
        .copied()
        .filter(|id| !second_blockers.contains(id))
        .collect::<Vec<_>>();
    let only_second = second_blockers
        .iter()
        .copied()
        .filter(|id| !first_blockers.contains(id))
        .collect::<Vec<_>>();
    if only_first.is_empty() && only_second.is_empty() {
        return Ok(EffectOutcome::count(0));
    }
    let new_first = first_blockers
        .iter()
        .copied()
        .filter(|id| second_blockers.contains(id))
        .chain(only_second.iter().copied())
        .collect::<Vec<_>>();
    let new_second = second_blockers
        .iter()
        .copied()
        .filter(|id| first_blockers.contains(id))
        .chain(only_first.iter().copied())
        .collect::<Vec<_>>();
    let moved = (only_first.len() + only_second.len()) as i32;
    let Some(combat) = game.combat_for_attacker_mut(first) else {
        return Ok(EffectOutcome::count(0));
    };
    for (attacker, blockers) in [(first, new_first), (second, new_second)] {
        if let Some(order) = combat.damage_assignment_order.get_mut(&attacker) {
            *order = blockers.clone();
        }
        combat.blockers.insert(attacker, blockers);
    }
    game.mark_continuous_state_dirty();
    for (attacker, blockers) in [(first, only_second), (second, only_first)] {
        for blocker in blockers {
            super::enter_attacking::publish_block_transition(
                game, blocker, attacker, false, true, true,
            );
        }
    }
    Ok(EffectOutcome::count(moved))
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
            if self.between_attackers {
                return exchange_attackers_blockers(game, first, second);
            }
            let first_blocks = attackers_blocked_by(game, first);
            let second_blocks = attackers_blocked_by(game, second);
            // Both must still be blocking, and each must be able to block
            // everything the other blocks; otherwise nothing happens.
            if !second_blocks
                .iter()
                .all(|attacker| could_block(game, *attacker, first))
                || !first_blocks
                    .iter()
                    .all(|attacker| could_block(game, *attacker, second))
            {
                return Ok(EffectOutcome::count(0));
            }
            let Some(combat) = game.combat_lanes_mut().find(|combat| {
                crate::combat_state::is_blocking(combat, first)
                    && crate::combat_state::is_blocking(combat, second)
            }) else {
                return Ok(EffectOutcome::count(0));
            };
            // Both leave combat before they block the other's attackers. A
            // blocking creature with no remaining attacker can still exchange.
            combat.retained_blockers.remove(&first);
            combat.retained_blockers.remove(&second);
            for blockers in combat.blockers.values_mut() {
                swap_in(blockers, first, second);
            }
            for order in combat.damage_assignment_order.values_mut() {
                swap_in(order, first, second);
            }
            game.mark_continuous_state_dirty();
            for (blocker, attackers) in [(first, second_blocks), (second, first_blocks)] {
                for (index, attacker) in attackers.into_iter().enumerate() {
                    super::enter_attacking::publish_block_transition(
                        game,
                        blocker,
                        attacker,
                        false,
                        index != 0,
                        true,
                    );
                }
            }
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
