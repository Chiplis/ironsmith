//! Read-only observation of an already selected private card set.
use crate::effect::EffectOutcome;
use crate::effects::ExecutionContext;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

/// Selection and hidden-identity proofs stay with their owner. This operation
/// provides one private view and captures its exact pre-movement identities;
/// it creates neither public reveal state nor a reveal event.
pub(crate) fn look_at_cards(
    game: &GameState,
    ctx: &mut ExecutionContext,
    viewer: PlayerId,
    subject: PlayerId,
    zone: Zone,
    cards: &[ObjectId],
    description: impl Into<String>,
) -> EffectOutcome {
    let view = crate::decisions::context::ViewCardsContext::new(
        viewer,
        subject,
        Some(ctx.source),
        zone,
        description.into(),
    );
    ctx.decision_maker.view_cards(game, viewer, cards, &view);
    let snapshots = cards
        .iter()
        .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
        .collect::<Vec<_>>();
    EffectOutcome::count(cards.len() as i64)
        .with_execution_fact(crate::effect::ExecutionFact::ChosenObjects(cards.to_vec()))
        .with_chosen_object_memory(snapshots.clone())
        .with_affected_object_memory(snapshots)
}
