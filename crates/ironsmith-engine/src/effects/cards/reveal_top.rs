//! Reveal top card effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryConsultMode, LibraryConsultStopRule, execute_library_consult,
};
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::RevealTopEffect;

/// Effect that reveals the top card of a player's library and tags it.
///
/// This is a composable primitive for effects like Goblin Guide.
impl EffectExecutor for RevealTopEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        let result = execute_library_consult(
            game,
            ctx,
            player_id,
            LibraryConsultMode::Reveal,
            LibraryConsultStopRule::MatchCount(1),
            None,
            None,
            |_, _| true,
        )?;

        let count = result.exposed_object_ids.len() as i32;
        if let Some(tag) = &self.tag
            && !result.exposed_snapshots.is_empty()
        {
            ctx.tag_objects_unique(tag.clone(), result.exposed_snapshots.clone());
        }
        if !result.exposed_snapshots.is_empty() {
            ctx.tag_objects(
                crate::effects::PUBLIC_REVEALED_TAG,
                result.exposed_snapshots.clone(),
            );
        }
        Ok(result.attach_to_outcome(EffectOutcome::count(count)))
    }

    fn is_read_only_simultaneous_player_action(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::tag::TagKey;
    use crate::test_prelude::*;
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

    #[cfg(ironsmith_runtime_parser_tests)]
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
