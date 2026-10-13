//! Ordering policies preserve the caller's authored participation protocol.
use crate::{game_state::GameState, ids::PlayerId};
#[derive(Clone, Copy)]
pub(crate) enum PlayerOrderBasis {
    Seating,
    TurnWithSeatingFallback,
}
pub(crate) fn rotate_to_start(players: &mut [PlayerId], start: PlayerId) {
    if let Some(index) = players.iter().position(|player| *player == start) {
        players.rotate_left(index);
    }
}
pub(crate) fn active_players_from(
    game: &GameState,
    start: PlayerId,
    basis: PlayerOrderBasis,
) -> Vec<PlayerId> {
    let seating = || {
        game.players
            .iter()
            .filter(|p| p.is_in_game())
            .map(|p| p.id)
            .collect::<Vec<_>>()
    };
    let mut players = match basis {
        PlayerOrderBasis::Seating => seating(),
        PlayerOrderBasis::TurnWithSeatingFallback => {
            let order = game
                .turn_store
                .turn_order
                .iter()
                .copied()
                .filter(|id| game.player(*id).is_some_and(|p| p.is_in_game()))
                .collect::<Vec<_>>();
            if order.contains(&start) {
                order
            } else {
                seating()
            }
        }
    };
    rotate_to_start(&mut players, start);
    players
}
/// Ranking preserves selected-player order for equal ranks. If the active
/// player is absent, the historical numeric-ID fallback is intentional.
pub(crate) fn apnap_position(game: &GameState, player: PlayerId) -> usize {
    let order = &game.turn_store.turn_order;
    let Some(active) = order.iter().position(|id| *id == game.turn.active_player) else {
        return player.0 as usize;
    };
    order
        .iter()
        .position(|id| *id == player)
        .map_or(usize::MAX, |i| (i + order.len() - active) % order.len())
}
/// Keep turn-order multiplicity, then append unknown selected players once.
/// No in-game filtering here: the selection owner already defines eligibility.
pub(crate) fn selected_players_from(
    game: &GameState,
    selected: Vec<PlayerId>,
    start: PlayerId,
) -> Vec<PlayerId> {
    let mut order = game.turn_store.turn_order.clone();
    rotate_to_start(&mut order, start);
    let mut ordered = order
        .into_iter()
        .filter(|id| selected.contains(id))
        .collect::<Vec<_>>();
    for id in selected {
        if !ordered.contains(&id) {
            ordered.push(id);
        }
    }
    ordered
}
