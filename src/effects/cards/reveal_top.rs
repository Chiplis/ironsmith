//! Reveal top card effect implementation.

use crate::decisions::context::ViewCardsContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that reveals the top card of a player's library and tags it.
///
/// This is a composable primitive for effects like Goblin Guide.
#[derive(Debug, Clone, PartialEq)]
pub struct RevealTopEffect {
    pub player: PlayerFilter,
    pub tag: Option<TagKey>,
}

impl RevealTopEffect {
    /// Create a new reveal-top effect, optionally tagging the revealed card.
    pub fn new(player: PlayerFilter, tag: Option<TagKey>) -> Self {
        Self { player, tag }
    }

    /// Create a tagged reveal-top effect.
    pub fn tagged(player: PlayerFilter, tag: impl Into<TagKey>) -> Self {
        Self::new(player, Some(tag.into()))
    }
}

impl EffectExecutor for RevealTopEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        let top_card_id = game
            .player(player_id)
            .and_then(|p| p.library.last().copied());

        let Some(card_id) = top_card_id else {
            return Ok(EffectOutcome::count(0));
        };

        let snapshot = game
            .object(card_id)
            .map(|obj| ObjectSnapshot::from_object(obj, game));
        if let Some(snapshot) = snapshot.clone()
            && let Some(tag) = &self.tag
        {
            ctx.set_tagged_objects(tag.clone(), vec![snapshot]);
        }

        for viewer_idx in 0..game.players.len() {
            let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
            let view_ctx = ViewCardsContext::new(
                viewer,
                player_id,
                Some(ctx.source),
                crate::zone::Zone::Library,
                "Reveal the top card of a library",
            )
            .with_public(true);
            ctx.decision_maker
                .view_cards(game, viewer, &[card_id], &view_ctx);
        }

        Ok(
            EffectOutcome::count(1).with_event(crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::CardRevealedEvent::new(
                    player_id,
                    card_id,
                    crate::zone::Zone::Library,
                    Some(ctx.source),
                    snapshot,
                ),
                ctx.provenance,
            )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::tag::TagKey;
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

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_card_to_library(game: &mut GameState, owner: PlayerId, name: &str, id: u32) {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, owner, Zone::Library);
    }

    #[test]
    fn reveal_top_emits_public_view_for_all_players() {
        let mut game = setup_game();
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        add_card_to_library(&mut game, bob, "Top Card", 101);

        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, bob, &mut dm);
        let effect = RevealTopEffect::new(PlayerFilter::You, None);
        effect.execute(&mut game, &mut ctx).expect("reveal top");

        assert_eq!(dm.calls.len(), 2);
        assert!(dm.calls.iter().all(|(_, subject, zone, public, cards)| {
            *subject == bob && *zone == Zone::Library && *public && cards.len() == 1
        }));
    }

    #[test]
    fn reveal_top_emits_card_revealed_event_that_triggers_reveal_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let watcher = CardDefinitionBuilder::new(CardId::from_raw(301), "Reveal Watcher")
            .card_types(vec![CardType::Enchantment])
            .parse_text("Whenever you reveal a creature card, draw a card.")
            .expect("watcher should parse");
        game.create_object_from_definition(&watcher, alice, Zone::Battlefield);

        let creature = CardBuilder::new(CardId::from_raw(302), "Creature Reveal")
            .card_types(vec![CardType::Creature])
            .build();
        let revealed_id = game.create_object_from_card(&creature, alice, Zone::Library);

        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = RevealTopEffect::new(PlayerFilter::You, None)
            .execute(&mut game, &mut ctx)
            .expect("reveal top");

        let reveal_event = outcome.events.first().expect("reveal event");
        let triggered = crate::triggers::check::check_triggers(&game, reveal_event);
        assert_eq!(triggered.len(), 1);
        let revealed = triggered[0]
            .tagged_objects
            .get(&TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
            .expect("triggered reveal should preserve public-revealed tag");
        assert_eq!(revealed.len(), 1);
        assert_eq!(revealed[0].object_id, revealed_id);
    }
}
