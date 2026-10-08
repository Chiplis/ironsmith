//! Library ordering is a position change, not a zone change.

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Both ordered partitions are top-to-bottom. Cards that have left this
/// library are excluded; all other cards retain their relative order.
pub(crate) fn arrange_library_cards(
    game: &mut GameState,
    owner: PlayerId,
    top: &[ObjectId],
    bottom: &[ObjectId],
    reason: &str,
) {
    let Some(player) = game.player(owner) else {
        return;
    };
    let mut selected = std::collections::HashSet::new();
    let top = top
        .iter()
        .copied()
        .filter(|id| player.library.contains(id) && selected.insert(*id))
        .collect::<Vec<_>>();
    let bottom = bottom
        .iter()
        .copied()
        .filter(|id| player.library.contains(id) && selected.insert(*id))
        .collect::<Vec<_>>();
    let mut order = bottom.into_iter().rev().collect::<Vec<_>>();
    order.extend(
        player
            .library
            .iter()
            .copied()
            .filter(|id| !selected.contains(id)),
    );
    order.extend(top.into_iter().rev());
    game.set_player_library_order_with_audit(owner, order, reason);
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum LibraryCardPosition {
    Top,
    Bottom,
    NthFromTop(usize),
}

/// Position a single exact identity. Batch adapters decide insertion order;
/// this owner never shuffles or generates a zone-change observation.
pub(crate) fn position_library_card(
    game: &mut GameState,
    owner: PlayerId,
    card: ObjectId,
    position: LibraryCardPosition,
    reason: &str,
) {
    match position {
        LibraryCardPosition::Top => arrange_library_cards(game, owner, &[card], &[], reason),
        LibraryCardPosition::Bottom => arrange_library_cards(game, owner, &[], &[card], reason),
        LibraryCardPosition::NthFromTop(position) => {
            game.move_library_card_to_nth_from_top(owner, card, position, reason);
        }
    }
}

/// Choose a permutation in top-to-bottom order. Invalid or partial answers
/// retain the remaining identities in their original order.
pub(crate) fn order_library_cards_top_to_bottom(
    game: &GameState,
    ctx: &mut crate::effects::ExecutionContext,
    chooser: PlayerId,
    description: &str,
    cards: &[ObjectId],
) -> Vec<ObjectId> {
    if ctx.decision_maker.awaiting_choice() || cards.len() <= 1 {
        return cards.to_vec();
    }
    let items = cards
        .iter()
        .map(|id| {
            (
                *id,
                game.object(*id)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| "Unknown".into()),
            )
        })
        .collect();
    let prompt =
        crate::decisions::context::OrderContext::new(chooser, Some(ctx.source), description, items);
    let answer = ctx.decision_maker.decide_order(game, &prompt);
    let mut remaining = cards.to_vec();
    let mut ordered = Vec::with_capacity(cards.len());
    for id in answer {
        if let Some(index) = remaining.iter().position(|candidate| *candidate == id) {
            ordered.push(id);
            remaining.remove(index);
        }
    }
    ordered.extend(remaining);
    ordered
}

pub(crate) fn execute_library_instruction_atomically<'a>(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext<'a>,
    body: impl FnOnce(
        &mut GameState,
        &mut crate::effects::ExecutionContext<'a>,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError>,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    crate::effects::composition::execute_compound(game, ctx, body)
}
