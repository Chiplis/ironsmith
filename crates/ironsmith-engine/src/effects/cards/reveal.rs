//! The public reveal action, independent of how its cards were selected.

use crate::decisions::context::ViewCardsContext;
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Publish one exact group. Selection and hidden-card opening happen before
/// this boundary; the caller owns whether presentation can suspend its action.
pub(crate) fn public_reveal_view(
    game: &GameState,
    decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
    viewer: PlayerId,
    subject: PlayerId,
    source: crate::ids::ObjectId,
    zone: crate::zone::Zone,
    cards: &[crate::ids::ObjectId],
    description: &str,
) {
    let view =
        ViewCardsContext::new(viewer, subject, Some(source), zone, description).with_public(true);
    decision_maker.view_cards(game, viewer, cards, &view);
}

/// One observation owner for ordinary reveals and draw-time rule reveals.
/// Preserve the supplied pre-action snapshot and the caller's exact provenance.
pub(crate) fn public_reveal_observation(
    actor: PlayerId,
    card: crate::ids::ObjectId,
    zone: crate::zone::Zone,
    source: crate::ids::ObjectId,
    snapshot: Option<ObjectSnapshot>,
    context_amount: Option<i32>,
    provenance: crate::provenance::ProvNodeId,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::CardRevealedEvent::new(actor, card, zone, Some(source), snapshot)
            .with_reveal_context_amount(context_amount),
        provenance,
    )
}

#[derive(Debug, Clone)]
struct RevealObjects {
    objects: Vec<ObjectSnapshot>,
    actor: Option<PlayerId>,
    description: String,
    context_amount: Option<i32>,
}

impl EffectExecutor for RevealObjects {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Revealed)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        crate::effects::composition::execute_transaction(game, ctx, || EffectOutcome::count(0), |game, ctx| {
        let mut objects = self.objects.clone();
        // Authentication precedes every public view and identity-dependent observation.
        for index in 0..game.players.len() {
            let owner = PlayerId::from_index(index as u8);
            let private = objects.iter().filter_map(|snapshot| {
                game.object(snapshot.object_id)
                    .filter(|object| object.owner == owner && game.hidden_identity_is_private(object.id))
                    .map(|object| object.id)
            }).collect::<Vec<_>>();
            if private.is_empty() { continue; }
            let Some(opened) = game.reveal_private_hidden_cards_publicly(
                &mut *ctx.decision_maker, owner, ctx.source, &private, &self.description, false,
            ) else {
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                return Err(ExecutionError::IncompleteEvidence("mandatory reveal has no completed opening answer".into()));
            };
            if private.iter().any(|id| !opened.contains(id) || game.is_hidden_card_placeholder(*id)) {
                return Err(ExecutionError::IncompleteEvidence(
                    "a mandatory public reveal lacks an authenticated selected identity".into()));
            }
        }
        for snapshot in &mut objects {
            if game.object(snapshot.object_id).is_some_and(|object| object.stable_id == snapshot.stable_id) {
                *snapshot = ObjectSnapshot::try_from_object_id(game, snapshot.object_id)?
                    .ok_or_else(|| ExecutionError::IncompleteEvidence("revealed object disappeared during capture".into()))?;
            }
        }
        // A view context has one subject and zone; partition mixed selections
        // without changing their original order or publishing other cards.
        let mut groups = Vec::new();
        for snapshot in &objects {
            let key = (snapshot.owner, snapshot.zone);
            if !groups.contains(&key) {
                groups.push(key);
            }
        }
        for (owner, zone) in groups {
            let ids = objects
                .iter()
                .filter(|snapshot| snapshot.owner == owner && snapshot.zone == zone)
                .map(|snapshot| snapshot.object_id)
                .collect::<Vec<_>>();
            for index in 0..game.players.len() {
                let viewer = PlayerId::from_index(index as u8);
                public_reveal_view(
                    game,
                    ctx.decision_maker,
                    viewer,
                    owner,
                    ctx.source,
                    zone,
                    &ids,
                    &self.description,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
            }
        }
        let ids = objects
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        game.mark_hidden_cards_publicly_revealed(&ids);
        let public = ctx
            .tagged_objects
            .entry(TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
            .or_default();
        for snapshot in &objects {
            if let Some(existing) = public.iter_mut().find(|existing| existing.object_id == snapshot.object_id) {
                *existing = snapshot.clone();
            } else {
                public.push(snapshot.clone());
            }
        }
        let events = objects
            .iter()
            .map(|snapshot| {
                let provenance = game.alloc_child_event_provenance(ctx.provenance, crate::events::EventKind::CardRevealed);
                public_reveal_observation(
                    self.actor.unwrap_or(snapshot.owner),
                    snapshot.object_id,
                    snapshot.zone,
                    ctx.source,
                    Some(snapshot.clone()),
                    self.context_amount,
                    provenance,
                )
            })
            .collect::<Vec<_>>();
        Ok(EffectOutcome::count(objects.len() as i32)
            .with_events(events)
            .with_chosen_object_memory(objects.clone())
            .with_affected_object_memory(objects.clone()))
        })
    }
}

/// Execute disclosure through the same child boundary as other authored actions.
/// The caller retains its selection/hidden-proof protocol and completion event.
pub(crate) fn reveal_objects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    objects: Vec<ObjectSnapshot>,
    actor: Option<PlayerId>,
    description: impl Into<String>,
    context_amount: Option<i32>,
) -> Result<EffectOutcome, ExecutionError> {
    RevealObjects {
        objects,
        actor,
        description: description.into(),
        context_amount,
    }
    .execute_child(game, ctx)
}
