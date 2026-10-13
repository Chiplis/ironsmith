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
    walk_filter_references(
        &mut bound,
        PlayerReferenceFields::ControllerOwner,
        &mut |player| {
            if binds_at_resolution(player)
                && let Ok(id) = crate::effects::helpers::resolve_player_filter(game, player, ctx)
            {
                *player = PlayerFilter::Specific(id);
            }
            Ok::<_, std::convert::Infallible>(())
        },
        &mut |_| Ok(()),
    )
    .unwrap();
    bound
}

/// Deliberately bounded field sets: different stored effects freeze different
/// facets. Selecting a wider set is a semantic change, not a traversal cleanup.
#[derive(Clone, Copy)]
pub(crate) enum PlayerReferenceFields {
    ControllerOwner,
    SpellFacets,
    AllAuthored,
}

pub(crate) fn walk_filter_references<E>(
    filter: &mut ObjectFilter,
    fields: PlayerReferenceFields,
    bind: &mut impl FnMut(&mut PlayerFilter) -> Result<(), E>,
    finish: &mut impl FnMut(&mut ObjectFilter) -> Result<(), E>,
) -> Result<(), E> {
    for player in [&mut filter.controller, &mut filter.owner]
        .into_iter()
        .flatten()
    {
        bind(player)?;
    }
    if !matches!(fields, PlayerReferenceFields::ControllerOwner) {
        for player in [
            &mut filter.cast_by,
            &mut filter.targets_player,
            &mut filter.targets_only_player,
        ]
        .into_iter()
        .flatten()
        {
            bind(player)?;
        }
    }
    if matches!(fields, PlayerReferenceFields::AllAuthored) {
        if let Some(player) = &mut filter.entered_battlefield_controller {
            bind(player)?;
        }
        if let Some(constraint) = &mut filter.counters_put_on_this_turn {
            bind(&mut constraint.source_controller)?;
        }
        if let Some(player) = &mut filter.attached_to_player {
            bind(player)?;
        }
        if let Some(filter) = &mut filter.attached_to_object {
            walk_filter_references(filter, fields, bind, finish)?;
        }
    }
    for branch in &mut filter.any_of {
        walk_filter_references(branch, fields, bind, finish)?;
    }
    finish(filter)
}

pub(crate) fn walk_player_reference_wrappers(
    filter: &mut PlayerFilter,
    bind: &mut impl FnMut(&mut PlayerFilter),
) {
    bind(filter);
    match filter {
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
        | PlayerFilter::HasMoreLifeThanYou { base }
        | PlayerFilter::LostLifeThisTurn { base }
        | PlayerFilter::OpponentOf(base)
        | PlayerFilter::PlayerToLeftOf(base)
        | PlayerFilter::MaxSpeed { base, .. } => walk_player_reference_wrappers(base, bind),
        PlayerFilter::Excluding { base, excluded } => {
            walk_player_reference_wrappers(base, bind);
            walk_player_reference_wrappers(excluded, bind);
        }
        _ => {}
    }
}
