//! Mill effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that mills cards from a player's library to their graveyard.
///
/// # Fields
///
/// * `count` - How many cards to mill (can be fixed or variable)
/// * `player` - Which player mills
///
/// # Example
///
/// ```ignore
/// // Mill 3 cards
/// let effect = MillEffect::new(3, PlayerFilter::You);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MillEffect {
    /// How many cards to mill.
    pub count: Value,
    /// Which player mills.
    pub player: PlayerFilter,
}

impl MillEffect {
    /// Create a new mill effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you mill cards.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for MillEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        // Get the cards to mill (from top of library)
        let milled: Vec<ObjectId> = game
            .player(player_id)
            .map(|p| {
                let lib_len = p.library.len();
                let mill_count = count.min(lib_len);
                p.library[lib_len.saturating_sub(mill_count)..].to_vec()
            })
            .unwrap_or_default();

        let milled_count = milled.len();

        // Remove from library
        if let Some(p) = game.player_mut(player_id) {
            p.library
                .truncate(p.library.len().saturating_sub(milled_count));
        }

        // Move each card to graveyard
        for card_id in milled {
            if let Some(obj) = game.object_mut(card_id) {
                obj.zone = Zone::Graveyard;
            }
            if let Some(p) = game.player_mut(player_id) {
                p.graveyard.push(card_id);
            }
        }

        Ok(EffectOutcome::count(milled_count as i32))
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
    fn test_mill_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MillEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).unwrap().library.len(), 2);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 3);
    }

    #[test]
    fn test_mill_more_than_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 3);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MillEffect::you(10);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Can only mill as many cards as in library
        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).unwrap().library.len(), 0);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 3);
    }

    #[test]
    fn test_mill_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MillEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().library.len(), 5);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 0);
    }

    #[test]
    fn test_mill_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MillEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_mill_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        add_cards_to_library(&mut game, bob, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MillEffect::new(2, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(bob).unwrap().library.len(), 3);
        assert_eq!(game.player(bob).unwrap().graveyard.len(), 2);
    }

    #[test]
    fn test_mill_x_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 10);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(4);

        let effect = MillEffect::new(Value::X, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(4));
        assert_eq!(game.player(alice).unwrap().library.len(), 6);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 4);
    }

    #[test]
    fn test_mill_clone_box() {
        let effect = MillEffect::you(3);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MillEffect"));
    }
}
