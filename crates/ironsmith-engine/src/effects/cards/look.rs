//! Read-only observation of an already selected private card set.
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
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
    let description = description.into();
    for entitled_viewer in game.private_information_viewers_for(viewer, zone) {
        let view = crate::decisions::context::ViewCardsContext::new(
            entitled_viewer,
            subject,
            Some(ctx.source),
            zone,
            description.clone(),
        );
        ctx.decision_maker
            .view_cards(game, entitled_viewer, cards, &view);
        if ctx.decision_maker.awaiting_choice() {
            return EffectOutcome::count(0);
        }
    }
    let snapshots = cards
        .iter()
        .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
        .collect::<Vec<_>>();
    EffectOutcome::count(cards.len() as i64)
        .with_execution_fact(crate::effect::ExecutionFact::ChosenObjects(cards.to_vec()))
        .with_chosen_object_memory(snapshots.clone())
        .with_affected_object_memory(snapshots)
}

/// A selected private observation. Public disclosure remains a separate action.
#[derive(Clone, Debug)]
struct LookAtCards {
    viewer: PlayerId,
    subject: PlayerId,
    zone: Zone,
    cards: Vec<ObjectId>,
    description: String,
}

impl EffectExecutor for LookAtCards {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(look_at_cards(
            game,
            ctx,
            self.viewer,
            self.subject,
            self.zone,
            &self.cards,
            self.description.clone(),
        ))
    }
}

/// Use the same private view owner while retaining its actual child output.
pub(crate) fn look_at_cards_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    viewer: PlayerId,
    subject: PlayerId,
    zone: Zone,
    cards: &[ObjectId],
    description: impl Into<String>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    LookAtCards {
        viewer,
        subject,
        zone,
        cards: cards.to_vec(),
        description: description.into(),
    }
    .execute_child_with_outputs(game, ctx)
}
