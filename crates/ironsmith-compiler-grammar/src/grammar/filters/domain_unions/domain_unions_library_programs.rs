use super::*;

/// Propagate a trailing non-battlefield card domain across independently
/// nouned card arms. In `Assassin card or card with freerunning from your
/// graveyard`, both arms repeat the card noun and the final domain scopes the
/// whole union. A permanent/controller arm remains excluded from this lift.
pub(super) fn propagate_trailing_shared_card_zone_scope(
    branches: &mut [ObjectFilter],
    repeated_card_noun_surface: bool,
) {
    let Some((last, preceding)) = branches.split_last_mut() else {
        return;
    };
    let Some(zone) = last.zone else {
        return;
    };
    if zone == Zone::Battlefield
        || !last.has_explicit_card_noun()
        || !preceding.iter().all(|branch| {
            matches!(branch.zone, None | Some(Zone::Battlefield))
                && branch.controller.is_none()
                && (branch.has_explicit_card_noun() || repeated_card_noun_surface)
        })
    {
        return;
    }
    for branch in preceding {
        branch.zone = Some(zone);
    }
}
