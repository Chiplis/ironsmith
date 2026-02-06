//! Shuffle library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that shuffles a player's library.
///
/// # Fields
///
/// * `player` - Which player's library to shuffle
///
/// # Example
///
/// ```ignore
/// // Shuffle your library
/// let effect = ShuffleLibraryEffect::you();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleLibraryEffect {
    /// Which player's library to shuffle.
    pub player: PlayerFilter,
}

impl ShuffleLibraryEffect {
    /// Create a new shuffle library effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Create an effect to shuffle your library.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for ShuffleLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        if let Some(p) = game.player_mut(player_id) {
            p.shuffle_library();
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::EffectResult;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn add_cards_to_library(game: &mut GameState, player: PlayerId, count: usize) {
        for i in 1..=count {
            let card = CardBuilder::new(CardId::from_raw(i as u32), &format!("Card {}", i))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&card, player, Zone::Library);
        }
    }

    #[test]
    fn test_shuffle_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add cards to library
        add_cards_to_library(&mut game, alice, 10);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ShuffleLibraryEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Library still has same number of cards
        assert_eq!(game.player(alice).unwrap().library.len(), 10);
    }

    #[test]
    fn test_shuffle_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ShuffleLibraryEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().library.len(), 0);
    }

    #[test]
    fn test_shuffle_opponent_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        add_cards_to_library(&mut game, bob, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ShuffleLibraryEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(bob).unwrap().library.len(), 5);
    }

    #[test]
    fn test_shuffle_clone_box() {
        let effect = ShuffleLibraryEffect::you();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ShuffleLibraryEffect"));
    }
}
