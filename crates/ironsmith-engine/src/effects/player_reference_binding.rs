//! Locking discourse player references inside an object filter.
//!
//! "Protection from that player" (Eon Frolicker, Noble Heritage) names the
//! player the instruction is talking about as it resolves: the targeted
//! opponent, or the opponent a "for each opponent who does" loop is visiting.
//! A shield, restriction or granted ability created by that instruction keeps
//! naming that player after the resolution context (its targets and loop
//! binding) is gone (CR 608.2h, 611.2c), so the reference is replaced by the
//! concrete player before the effect is stored.

use crate::effects::ExecutionContext;
use crate::game_state::GameState;
use crate::target::{ObjectFilter, PlayerFilter};

fn binds_at_resolution(player: &PlayerFilter) -> bool {
    matches!(
        player,
        PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_) | PlayerFilter::IteratedPlayer
    )
}

/// Replace a targeted, aliased-target or iterated controller/owner reference
/// with the concrete player it names in this resolution. References that
/// can't be resolved here are left untouched.
pub(crate) fn bind_filter_player_references(
    filter: &ObjectFilter,
    game: &GameState,
    ctx: &ExecutionContext,
) -> ObjectFilter {
    let mut bound = filter.clone();
    for player in [&mut bound.controller, &mut bound.owner].into_iter().flatten() {
        if binds_at_resolution(player)
            && let Ok(id) = crate::effects::helpers::resolve_player_filter(game, player, ctx)
        {
            *player = PlayerFilter::Specific(id);
        }
    }
    bound.any_of = bound
        .any_of
        .iter()
        .map(|branch| bind_filter_player_references(branch, game, ctx))
        .collect();
    bound
}
