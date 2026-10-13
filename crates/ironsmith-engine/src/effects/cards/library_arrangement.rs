//! Library ordering is a position change, not a zone change.

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Capture exact retained identities still present in one library, in its
/// current top-to-bottom order. Stable card identities never rebind this set.
pub(crate) fn capture_retained_library_set(
    game: &GameState,
    snapshots: &[crate::snapshot::ObjectSnapshot],
) -> Option<(PlayerId, Vec<ObjectId>)> {
    let cards = snapshots
        .iter()
        .filter_map(|snapshot| game.object(snapshot.object_id))
        .filter(|object| object.zone == crate::zone::Zone::Library)
        .collect::<Vec<_>>();
    let owner = cards.first()?.owner;
    if cards.iter().any(|object| object.owner != owner) {
        return None;
    }
    let ordered = game
        .player(owner)?
        .library
        .iter()
        .rev()
        .copied()
        .filter(|id| cards.iter().any(|object| object.id == *id))
        .collect::<Vec<_>>();
    (!ordered.is_empty()).then_some((owner, ordered))
}

/// Keep each available identity once, then append omissions in original order.
/// This normalizes permutations without changing the zone's orientation.
pub(crate) fn normalize_order_response<T: Copy + PartialEq>(
    response: Vec<T>,
    original: &[T],
) -> Vec<T> {
    let mut remaining = original.to_vec();
    let mut ordered = Vec::with_capacity(original.len());
    for id in response {
        if let Some(index) = remaining.iter().position(|candidate| *candidate == id) {
            ordered.push(id);
            remaining.remove(index);
        }
    }
    ordered.extend(remaining);
    ordered
}

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
    normalize_order_response(answer, cards)
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

#[cfg(test)]
mod retained_set_tests {
    use super::*;
    use crate::{card::CardBuilder, ids::CardId, snapshot::ObjectSnapshot, types::CardType, zone::Zone};
    #[test]
    fn capture_uses_exact_current_ids_one_owner_and_current_order() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let card = CardBuilder::new(CardId::new(), "Retained card").card_types(vec![CardType::Sorcery]).build();
        let a = game.create_object_from_card(&card, PlayerId(0), Zone::Library);
        let b = game.create_object_from_card(&card, PlayerId(0), Zone::Library);
        let other = game.create_object_from_card(&card, PlayerId(1), Zone::Library);
        let snap = |id| ObjectSnapshot::from_object(game.object(id).unwrap(), &game);
        let retained = vec![snap(a), snap(b), snap(a)];
        assert_eq!(capture_retained_library_set(&game, &retained), Some((PlayerId(0), vec![b, a])));
        assert!(capture_retained_library_set(&game, &[snap(a), snap(other)]).is_none());
        let departed = game.move_object_by_effect(a, Zone::Hand).unwrap();
        let returned = game.move_object_by_effect(departed, Zone::Library).unwrap();
        assert_ne!(returned, a);
        assert_eq!(capture_retained_library_set(&game, &retained), Some((PlayerId(0), vec![b])));
    }
    #[test]
    fn permutation_discards_foreign_and_duplicate_answers_and_appends_omissions() {
        assert_eq!(normalize_order_response(vec![3, 3, 99, 1], &[1, 2, 3]), vec![3, 1, 2]);
        assert_eq!(normalize_order_response(Vec::<u32>::new(), &[1, 2, 3]), vec![1, 2, 3]);
        assert!(normalize_order_response(vec![1], &[]).is_empty());
    }
}

/// Choose the bottom partition of already captured cards. This neither opens
/// hidden information nor moves cards nor emits a scry/other keyword event.
/// Callers own disclosure, ordering of each partition, and the commit boundary.
pub(crate) fn choose_library_bottom_partition(
    game: &GameState,
    ctx: &mut crate::effects::ExecutionContext,
    chooser: PlayerId,
    cards: &[ObjectId],
) -> Option<Vec<ObjectId>> {
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    if cards.is_empty() {
        return Some(Vec::new());
    }
    let selected: Vec<ObjectId> = crate::decisions::make_decision(
        game,
        &mut ctx.decision_maker,
        chooser,
        Some(ctx.source),
        crate::decisions::ScrySpec::new(ctx.source, cards.to_vec()),
    );
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    Some(
        selected
            .into_iter()
            .filter(|card| cards.contains(card))
            .collect(),
    )
}
