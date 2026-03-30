//! Reveal tagged cards effect implementation.
//!
//! Reveals currently update player-facing visibility and carry that visibility
//! through tagged contexts when later stack objects still need it.

use crate::decisions::context::ViewCardsContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;

/// Effect that reveals the objects currently tagged under `tag`.
///
/// This is mainly used to support clauses like "reveal it" where "it" refers to
/// a card found/drawn earlier in the same effect chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealTaggedEffect {
    pub tag: TagKey,
}

impl RevealTaggedEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

impl EffectExecutor for RevealTaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let tagged = ctx
            .get_tagged_all(self.tag.clone())
            .cloned()
            .unwrap_or_default();
        let count = tagged.len();
        if let Some(first) = tagged.first() {
            let card_ids = tagged.iter().map(|obj| obj.object_id).collect::<Vec<_>>();
            for viewer_idx in 0..game.players.len() {
                let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
                let view_ctx = ViewCardsContext::new(
                    viewer,
                    first.owner,
                    Some(ctx.source),
                    first.zone,
                    "Reveal cards",
                )
                .with_public(true);
                ctx.decision_maker
                    .view_cards(game, viewer, &card_ids, &view_ctx);
            }
        }
        if !tagged.is_empty() {
            let entry = ctx
                .tagged_objects
                .entry(TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
                .or_default();
            for snapshot in tagged.iter().cloned() {
                if !entry
                    .iter()
                    .any(|existing| existing.object_id == snapshot.object_id)
                {
                    entry.push(snapshot);
                }
            }
        }
        let reveal_events = tagged
            .iter()
            .map(|snapshot| {
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CardRevealedEvent::new(
                        snapshot.owner,
                        snapshot.object_id,
                        snapshot.zone,
                        Some(ctx.source),
                        Some(snapshot.clone()),
                    ),
                    ctx.provenance,
                )
            })
            .collect::<Vec<_>>();

        Ok(EffectOutcome::count(count as i32).with_events(reveal_events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<crate::ids::ObjectId>)>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[crate::ids::ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls
                .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
        }
    }

    #[test]
    fn reveal_tagged_emits_public_view_for_tagged_cards() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(201), "Tagged Card")
            .card_types(vec![CardType::Instant])
            .build();
        let object_id = game.create_object_from_card(&card, bob, Zone::Library);

        let snapshot = {
            let obj = game.object(object_id).expect("tagged object");
            ObjectSnapshot::from_object(obj, &game)
        };
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects(TagKey::from("revealed"), vec![snapshot]);

        RevealTaggedEffect::new("revealed")
            .execute(&mut game, &mut ctx)
            .expect("reveal tagged");

        assert_eq!(dm.calls.len(), 2);
        assert!(dm.calls.iter().all(|(_, subject, zone, public, cards)| {
            *subject == bob && *zone == Zone::Library && *public && cards == &vec![object_id]
        }));
    }
}
