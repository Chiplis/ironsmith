//! Shuffle hand and graveyard into library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::events::ShuffleLibraryEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Effect that moves all cards from a player's hand and graveyard to their library, then shuffles.
#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleHandAndGraveyardIntoLibraryEffect {
    /// Which player's hand, graveyard, and library to use.
    pub player: PlayerFilter,
}

impl ShuffleHandAndGraveyardIntoLibraryEffect {
    /// Create a new effect for the provided player filter.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for ShuffleHandAndGraveyardIntoLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        let (hand_cards, graveyard_cards) = game
            .player(player_id)
            .map(|player| (player.hand.clone(), player.graveyard.clone()))
            .unwrap_or_default();

        for card_id in hand_cards.into_iter().chain(graveyard_cards) {
            let _ = game.move_object_with_commander_options(
                card_id,
                Zone::Library,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            );
        }

        game.shuffle_player_library(player_id);

        Ok(
            EffectOutcome::resolved().with_event(TriggerEvent::new_with_provenance(
                ShuffleLibraryEvent::new(player_id, ctx.cause.clone()),
                ctx.provenance,
            )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_card_in_zone(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn shuffle_hand_and_graveyard_into_library_moves_all_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_card_in_zone(&mut game, alice, Zone::Hand, "Hand Card");
        create_card_in_zone(&mut game, alice, Zone::Graveyard, "Grave Card");
        create_card_in_zone(&mut game, alice, Zone::Library, "Library Card");

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ShuffleHandAndGraveyardIntoLibraryEffect::new(PlayerFilter::You);
        effect
            .execute(&mut game, &mut ctx)
            .expect("shuffle-hand-and-graveyard effect should resolve");

        let player = game.player(alice).expect("player should exist");
        assert!(player.hand.is_empty());
        assert!(player.graveyard.is_empty());
        assert_eq!(player.library.len(), 3);
    }

    #[test]
    fn shuffle_hand_and_graveyard_into_library_emits_shuffle_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_card_in_zone(&mut game, alice, Zone::Hand, "Hand Card");
        create_card_in_zone(&mut game, alice, Zone::Library, "Library A");
        create_card_in_zone(&mut game, alice, Zone::Library, "Library B");

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let before = game.irreversible_random_count();
        let effect = ShuffleHandAndGraveyardIntoLibraryEffect::new(PlayerFilter::You);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("shuffle-hand-and-graveyard effect should resolve");

        assert_eq!(game.irreversible_random_count(), before + 1);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.downcast::<ShuffleLibraryEvent>().is_some())
        );
    }
}
