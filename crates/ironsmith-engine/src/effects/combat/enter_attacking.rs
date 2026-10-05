//! Enter attacking effect implementation.

use crate::combat_state::AttackTarget;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that causes a creature that was just put onto the battlefield to be
/// attacking (if combat is active). Its controller chooses what it attacks
/// (CR 508.4).
#[derive(Debug, Clone, PartialEq)]
pub struct EnterAttackingEffect {
    pub target: ChooseSpec,
}

impl EnterAttackingEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

/// CR 506.2: is a combat phase in progress? `game.combat` alone can't answer
/// this — it stays `Some` (emptied) after the first combat of the game, and
/// is still `None` during the first beginning of combat step.
fn combat_phase_in_progress(game: &GameState) -> bool {
    game.turn.phase == crate::game_state::Phase::Combat
}

/// Is `controller` an attacking player this combat (CR 506.2; with shared
/// team turns, every member of the active team)?
fn is_attacking_player(game: &GameState, controller: PlayerId) -> bool {
    let active = game.turn.active_player;
    controller == active
        || (game.shared_team_turns_enabled() && game.are_teammates(controller, active))
}

/// CR 506.3a / 506.3b / 506.3f / 508.4: may the permanent `object_id`, just
/// put onto the battlefield "attacking", actually become an attacking
/// creature? Only during a combat phase, only if it's a creature (and not a
/// battle), and only if its controller is an attacking player. Otherwise it
/// enters but is never attacking.
pub(crate) fn can_enter_attacking(game: &GameState, object_id: ObjectId) -> bool {
    if !combat_phase_in_progress(game) {
        return false;
    }
    let Some(object) = game.object(object_id) else {
        return false;
    };
    if object.zone != Zone::Battlefield
        || !game.current_is_creature(object_id)
        || game.current_has_card_type(object_id, crate::types::CardType::Battle)
    {
        return false;
    }
    is_attacking_player(game, game.controller_of(object))
}

/// CR 509.4 / 509.4a / 506.3a / 506.3e / 506.3f: make `blocker`, just put
/// onto the battlefield "blocking `attacker`", a blocking creature. It
/// becomes one only during combat, only if it's a creature (not a battle),
/// only if `attacker` is still attacking, and only if its controller is the
/// defending player for that attacker (attacked directly, through a
/// planeswalker they control, or through a battle they protect). Returns
/// whether it's now blocking. It never "blocked" (CR 509.4), so no blocks
/// trigger event is produced.
pub(crate) fn put_onto_battlefield_blocking(
    game: &mut GameState,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    if !combat_phase_in_progress(game) {
        return false;
    }
    let Some(object) = game.object(blocker) else {
        return false;
    };
    if object.zone != Zone::Battlefield
        || !game.current_is_creature(blocker)
        || game.current_has_card_type(blocker, crate::types::CardType::Battle)
    {
        return false;
    }
    let controller = game.controller_of(object);
    let Some(target) = game.combat.as_ref().and_then(|combat| {
        combat
            .attackers
            .iter()
            .find(|info| info.creature == attacker)
            .map(|info| info.target.clone())
    }) else {
        return false;
    };
    if crate::combat_state::defending_player_for_attack_target(game, &target) != Some(controller) {
        return false;
    }
    let Some(combat) = game.combat.as_mut() else {
        return false;
    };
    let blockers = combat.blockers.entry(attacker).or_default();
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
    if let Some(order) = combat.damage_assignment_order.get_mut(&attacker)
        && !order.contains(&blocker)
    {
        order.push(blocker);
    }
    combat.blocked_attackers.insert(attacker);
    game.mark_continuous_state_dirty();
    true
}

/// CR 506.2 / 802.2 / 508.4: what a creature put onto the battlefield
/// attacking under `controller`'s control may attack — every defending
/// player of this combat (not only players something already attacks), the
/// planeswalkers they control and the battles they protect.
///
/// Empty outside a combat phase or when `controller` isn't an attacking player
/// (CR 506.3b: the creature enters but is never attacking).
pub(crate) fn enters_attacking_targets(
    game: &GameState,
    controller: PlayerId,
) -> Vec<AttackTarget> {
    if !combat_phase_in_progress(game) || !is_attacking_player(game, controller) {
        return Vec::new();
    }

    let defending_players = game
        .players
        .iter()
        .filter(|player| {
            player.is_in_game()
                && game.are_opponents(controller, player.id)
                && game.player_is_within_range(controller, player.id)
                && game.attack_direction_allows_defender(controller, player.id)
        })
        .map(|player| player.id)
        .collect::<Vec<_>>();

    let all_effects = game.all_continuous_effects();
    let mut targets = Vec::new();
    for defender in defending_players {
        targets.push(AttackTarget::Player(defender));
        for &object_id in &game.battlefield {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.zone != Zone::Battlefield {
                continue;
            }
            if game.controller_of(object) == defender
                && game.object_has_card_type_with_effects(
                    object_id,
                    crate::types::CardType::Planeswalker,
                    &all_effects,
                )
            {
                targets.push(AttackTarget::Planeswalker(object_id));
            } else if game.object_has_card_type_with_effects(
                object_id,
                crate::types::CardType::Battle,
                &all_effects,
            ) && game.battle_protector(object_id) == Some(defender)
            {
                targets.push(AttackTarget::Battle(object_id));
            }
        }
    }
    targets
}

fn attack_target_description(game: &GameState, target: &AttackTarget) -> String {
    match target {
        AttackTarget::Player(player) => game
            .player(*player)
            .map(|player| player.name.to_string())
            .unwrap_or_else(|| format!("player {}", player.0)),
        AttackTarget::Planeswalker(object_id) => game
            .object(*object_id)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| format!("planeswalker #{}", object_id.0)),
        AttackTarget::Battle(object_id) => game
            .object(*object_id)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| format!("battle #{}", object_id.0)),
        AttackTarget::Nothing { .. } => "nothing".to_string(),
    }
}

/// CR 508.4: the entering creature's controller chooses which defending
/// player, planeswalker or battle it's attacking.
pub(crate) fn choose_enters_attacking_target(
    game: &GameState,
    ctx: &mut ExecutionContext<'_>,
    entering_id: ObjectId,
) -> Option<AttackTarget> {
    if !can_enter_attacking(game, entering_id) {
        return None;
    }
    let chooser = game
        .object(entering_id)
        .map(|object| game.controller_of(object))
        .unwrap_or(ctx.controller);
    let targets = enters_attacking_targets(game, chooser);
    if targets.len() <= 1 {
        return targets.first().cloned();
    }

    let options = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            crate::decisions::DisplayOption::new(index, attack_target_description(game, target))
        })
        .collect();
    let source = ctx.source;
    let selected = crate::decisions::make_decision(
        game,
        &mut *ctx.decision_maker,
        chooser,
        Some(source),
        crate::decisions::ChoiceSpec::single(source, options),
    );
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    let selected_index = selected.into_iter().next().unwrap_or(0);
    targets
        .get(selected_index)
        .cloned()
        .or_else(|| targets.first().cloned())
}

impl EffectExecutor for EnterAttackingEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let creature_id = resolve_single_object_for_effect(game, ctx, &self.target)?;

        // CR 508.4: the controller chooses what it attacks; whether the
        // source of the effect is itself attacking doesn't matter.
        let target = choose_enters_attacking_target(game, ctx, creature_id);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if let Some(target) = target {
            game.add_entering_attacker(creature_id, target);
        }

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}
