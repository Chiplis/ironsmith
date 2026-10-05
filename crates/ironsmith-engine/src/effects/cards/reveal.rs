//! The public reveal action, independent of how its cards were selected.

use crate::decisions::context::ViewCardsContext;
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

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
        // A view context has one subject and zone; partition mixed selections
        // without changing their original order or publishing other cards.
        let mut groups = Vec::new();
        for snapshot in &self.objects {
            let key = (snapshot.owner, snapshot.zone);
            if !groups.contains(&key) {
                groups.push(key);
            }
        }
        for (owner, zone) in groups {
            let ids = self
                .objects
                .iter()
                .filter(|snapshot| snapshot.owner == owner && snapshot.zone == zone)
                .map(|snapshot| snapshot.object_id)
                .collect::<Vec<_>>();
            for index in 0..game.players.len() {
                let viewer = PlayerId::from_index(index as u8);
                let view = ViewCardsContext::new(
                    viewer,
                    owner,
                    Some(ctx.source),
                    zone,
                    self.description.clone(),
                )
                .with_public(true);
                ctx.decision_maker.view_cards(game, viewer, &ids, &view);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
            }
        }
        let ids = self
            .objects
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        game.mark_hidden_cards_publicly_revealed(&ids);
        let public = ctx
            .tagged_objects
            .entry(TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
            .or_default();
        for snapshot in &self.objects {
            if !public
                .iter()
                .any(|existing| existing.object_id == snapshot.object_id)
            {
                public.push(snapshot.clone());
            }
        }
        let events = self
            .objects
            .iter()
            .map(|snapshot| {
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CardRevealedEvent::new(
                        self.actor.unwrap_or(snapshot.owner),
                        snapshot.object_id,
                        snapshot.zone,
                        Some(ctx.source),
                        Some(snapshot.clone()),
                    )
                    .with_reveal_context_amount(self.context_amount),
                    ctx.provenance,
                )
            })
            .collect::<Vec<_>>();
        Ok(EffectOutcome::count(self.objects.len() as i32)
            .with_events(events)
            .with_chosen_object_memory(self.objects.clone())
            .with_affected_object_memory(self.objects.clone()))
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
