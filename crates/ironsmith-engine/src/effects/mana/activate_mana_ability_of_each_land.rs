//! "<player> activates a mana ability of each land they control".
//!
//! Drain Power, Pygmy Hippo: for each land the player controls, that player
//! chooses one of its mana abilities whose costs they can pay and activates it
//! (CR 605.3a: an instruction may tell a player to activate a mana ability).
//! A land with no activatable mana ability (already tapped, for example) is
//! skipped.

use crate::effect::EffectOutcome;
use crate::effects::composition::scoped_choice::{SingleChoicePolicy, select_typed_option};
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::types::CardType;

pub type ActivateManaAbilityOfEachLandEffect = ironsmith_core::ActivateManaAbilityOfEachLandEffect;

/// The lands `player` controls, in battlefield order.
fn controlled_lands(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| !game.is_phased_out(id))
        .filter(|&id| {
            game.object(id)
                .is_some_and(|object| game.controller_of(object) == player)
                && game.current_has_card_type(id, CardType::Land)
        })
        .collect()
}

/// Indices of the land's mana abilities `player` can activate right now.
fn activatable_mana_abilities(game: &GameState, player: PlayerId, land: ObjectId) -> Vec<usize> {
    let ability_count = game
        .current_abilities(land)
        .map_or(0, |abilities| abilities.len());
    (0..ability_count)
        .filter(|&index| {
            crate::special_actions::can_activate_mana_ability_check(game, player, land, index)
                .is_ok()
        })
        .collect()
}

impl EffectExecutor for ActivateManaAbilityOfEachLandEffect {
    fn directly_mentions_player_filter(&self, needle: &crate::target::PlayerFilter) -> bool {
        self.player.mentions_player_filter(needle)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if game.player(player).is_none() {
            return Err(ExecutionError::InvalidTarget);
        }
        let mut activated = 0i32;
        for land in controlled_lands(game, player) {
            // An earlier activation may have moved or changed this land.
            if !game.battlefield.contains(&land) {
                continue;
            }
            let abilities = activatable_mana_abilities(game, player, land);
            let ability_index = match abilities.as_slice() {
                [] => continue,
                [only] => *only,
                _ => {
                    let land_name = game
                        .object(land)
                        .map(|object| object.name.to_string())
                        .unwrap_or_else(|| "a land".to_string());
                    let options = abilities
                        .iter()
                        .enumerate()
                        .map(|(position, index)| {
                            (*index, format!("{land_name}: mana ability {}", position + 1))
                        })
                        .collect::<Vec<_>>();
                    let chosen = select_typed_option(
                        game,
                        ctx,
                        player,
                        &format!("Choose a mana ability of {land_name} to activate"),
                        &options,
                        SingleChoicePolicy::First,
                    )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(activated));
                    }
                    let Some(index) = chosen else {
                        continue;
                    };
                    index
                }
            };
            if crate::special_actions::perform_activate_mana_ability(
                game,
                player,
                land,
                ability_index,
                &mut *ctx.decision_maker,
            )
            .is_ok()
            {
                activated += 1;
            }
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(activated));
            }
        }
        Ok(EffectOutcome::count(activated))
    }
}
