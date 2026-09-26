//! Enter attacking effect implementation.

use crate::combat_state::{AttackTarget, AttackerInfo};
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

/// CR 506.2 / 802.2 / 508.4: what a creature put onto the battlefield
/// attacking under `controller`'s control may attack — every defending
/// player of this combat (not only players something already attacks), the
/// planeswalkers they control and the battles they protect.
///
/// Empty when there is no combat or `controller` isn't an attacking player
/// (CR 506.3b: the creature enters but is never attacking).
pub(crate) fn enters_attacking_targets(game: &GameState, controller: PlayerId) -> Vec<AttackTarget> {
    if game.combat.is_none() {
        return Vec::new();
    }
    let active = game.turn.active_player;
    let is_attacking_player = controller == active
        || (game.shared_team_turns_enabled() && game.are_teammates(controller, active));
    if !is_attacking_player {
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
        let creature_id = resolve_single_object_for_effect(game, ctx, &self.target)?;

        // CR 508.4: the controller chooses what it attacks; whether the
        // source of the effect is itself attacking doesn't matter.
        if let Some(target) = choose_enters_attacking_target(game, ctx, creature_id)
            && let Some(ref mut combat) = game.combat
        {
            combat.attackers.push(AttackerInfo {
                creature: creature_id,
                target,
            });
        }

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}
