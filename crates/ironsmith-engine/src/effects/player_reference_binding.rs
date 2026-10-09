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

/// A discourse player reference a resolving instruction names: a target, the
/// iterated or damaged player, a tagged player, or an object's controller.
fn names_one_player(player: &PlayerFilter) -> bool {
    binds_at_resolution(player)
        || matches!(
            player,
            PlayerFilter::DamagedPlayer
                | PlayerFilter::TaggedPlayer(_)
                | PlayerFilter::ControllerOf(_)
                | PlayerFilter::AliasedControllerOf(_)
        )
}

/// Lock a player reference a resolving instruction names to its player.
pub(crate) fn bind_player_reference(
    player: &PlayerFilter,
    game: &GameState,
    ctx: &ExecutionContext,
) -> PlayerFilter {
    if names_one_player(player)
        && let Ok(id) = crate::effects::helpers::resolve_player_filter(game, player, ctx)
    {
        return PlayerFilter::Specific(id);
    }
    player.clone()
}

/// Lock every resolution reference in a filter a registered effect keeps
/// after its resolution context is gone: player references (as above, plus
/// the damaged/tagged player and an object's controller) and objects named by
/// this resolution's tags (CR 611.2c). A tag with no recorded object is left
/// as written.
pub(crate) fn bind_filter_resolution_references(
    filter: &ObjectFilter,
    game: &GameState,
    ctx: &ExecutionContext,
) -> ObjectFilter {
    let mut bound = filter.clone();
    for player in [&mut bound.controller, &mut bound.owner].into_iter().flatten() {
        *player = bind_player_reference(player, game, ctx);
    }
    bound.any_of = bound
        .any_of
        .iter()
        .map(|branch| bind_filter_resolution_references(branch, game, ctx))
        .collect();
    let all_identity = !bound.tagged_constraints.is_empty()
        && bound.tagged_constraints.iter().all(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        });
    if all_identity {
        let mut ids = Vec::new();
        for constraint in &bound.tagged_constraints {
            if let Some(snapshots) = ctx.get_tagged_all(&constraint.tag) {
                for snapshot in snapshots {
                    if !ids.contains(&snapshot.object_id) {
                        ids.push(snapshot.object_id);
                    }
                }
            }
        }
        match ids.as_slice() {
            [] => {}
            [id] => return ObjectFilter::specific(*id),
            _ => {
                return ObjectFilter {
                    any_of: ids.into_iter().map(ObjectFilter::specific).collect(),
                    ..Default::default()
                };
            }
        }
    }
    bound
}
