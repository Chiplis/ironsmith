//! DrawCards effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::CardsDrawnEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

/// Effect that causes a player to draw cards.
///
/// Handles replacement effects, "can't draw extra cards" restrictions,
/// and tracks cards drawn this turn for triggered abilities.
///
/// # Fields
///
/// * `count` - Number of cards to draw
/// * `player` - Which player draws (defaults to controller)
///
/// # Example
///
/// ```ignore
/// // Draw 2 cards (you draw)
/// let effect = DrawCardsEffect::you(2);
///
/// // Opponent draws 3 cards
/// let effect = DrawCardsEffect::new(3, PlayerFilter::Opponent);
///
/// // Specific player draws 2 cards
/// let effect = DrawCardsEffect::new(2, PlayerFilter::Specific(player_id));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DrawCardsEffect {
    /// Number of cards to draw.
    pub count: Value,
    /// Which player draws.
    pub player: PlayerFilter,
}

impl DrawCardsEffect {
    /// Create a new DrawCards effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create a "draw N cards" effect for the controller.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for DrawCardsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::event_processor::{EventOutcome, process_draw};

        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        // Check if this is the first draw this turn
        let current_draws = game
            .cards_drawn_this_turn
            .get(&player_id)
            .copied()
            .unwrap_or(0);
        let is_first = current_draws == 0;

        // Check for "can't draw extra cards" restriction (e.g., Narset)
        let count = if !game.can_draw_extra_cards(player_id) {
            // Player can only draw their first card of the turn
            if current_draws >= 1 {
                // Already drew this turn, can't draw any more
                return Ok(EffectOutcome::from_result(EffectResult::Prevented));
            }
            // First draw - can only draw 1, not more
            count.min(1)
        } else {
            count
        };

        // Process through replacement effects with decision maker
        match process_draw(game, player_id, count, is_first, &mut ctx.decision_maker) {
            EventOutcome::Prevented => Ok(EffectOutcome::from_result(EffectResult::Prevented)),
            EventOutcome::Proceed(final_count) => {
                // Actually draw the cards using GameState::draw_cards
                // which properly updates object zones
                let drawn = game.draw_cards(player_id, final_count as usize);

                // Track cards drawn this turn
                let cards_before = *game.cards_drawn_this_turn.entry(player_id).or_insert(0);
                *game.cards_drawn_this_turn.entry(player_id).or_insert(0) += drawn.len() as u32;

                let is_first = cards_before == 0;
                let count = drawn.len() as i32;

                // Only emit event if cards were actually drawn
                if drawn.is_empty() {
                    return Ok(EffectOutcome::count(0));
                }

                // Create a single CardsDrawnEvent with all drawn cards
                let event = TriggerEvent::new(CardsDrawnEvent::new(player_id, drawn, is_first));

                Ok(EffectOutcome::count(count).with_event(event))
            }
            EventOutcome::Replaced => {
                // Replacement effects already executed by process_draw
                Ok(EffectOutcome::from_result(EffectResult::Replaced))
            }
            EventOutcome::NotApplicable => {
                // Player can't draw (no library, etc.)
                Ok(EffectOutcome::from_result(EffectResult::Prevented))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn add_cards_to_library(game: &mut GameState, owner: PlayerId, count: usize) {
        for i in 1..=count {
            let card = CardBuilder::new(CardId::new(), &format!("Library Card {}", i))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&card, owner, Zone::Library);
        }
    }

    #[test]
    fn test_draw_cards_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        assert_eq!(game.player(alice).unwrap().library.len(), 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
    }

    #[test]
    fn test_draw_cards_tracks_drawn_this_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // First draw
        let effect = DrawCardsEffect::you(2);
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            game.cards_drawn_this_turn.get(&alice).copied().unwrap_or(0),
            2
        );

        // Second draw
        let effect = DrawCardsEffect::you(1);
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            game.cards_drawn_this_turn.get(&alice).copied().unwrap_or(0),
            3
        );
    }

    #[test]
    fn test_draw_cards_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // No cards in library
        assert_eq!(game.player(alice).unwrap().library.len(), 0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Can't draw from empty library
        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
    }

    #[test]
    fn test_draw_cards_partial_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 2);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only draw what's available
        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 0);
    }

    #[test]
    fn test_draw_cards_for_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        add_cards_to_library(&mut game, bob, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Alice makes Bob draw
        let effect = DrawCardsEffect::new(2, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(bob).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
    }

    #[test]
    fn test_draw_cards_variable_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 10);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(3);

        let effect = DrawCardsEffect::new(Value::X, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).unwrap().hand.len(), 3);
    }

    #[test]
    fn test_draw_cards_clone_box() {
        let effect = DrawCardsEffect::you(2);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DrawCardsEffect"));
    }

    #[test]
    fn test_draw_cards_returns_events() {
        use crate::events::EventKind;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have 1 CardsDrawnEvent containing all 3 cards
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::CardsDrawn);

        let event = result.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert_eq!(event.cards.len(), 3);
        assert!(event.is_first_this_turn);
    }

    #[test]
    fn test_draw_cards_first_draw_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // First draw of turn
        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let event = result.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert!(event.is_first_this_turn);
        assert_eq!(event.cards.len(), 2);

        // Second draw of turn
        let effect2 = DrawCardsEffect::you(1);
        let result2 = effect2.execute(&mut game, &mut ctx).unwrap();

        let event2 = result2.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert!(!event2.is_first_this_turn); // Not first draw anymore
    }
}
