//! Discard hand effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that causes a player to discard their entire hand.
///
/// # Fields
///
/// * `player` - The player who discards their hand
///
/// # Example
///
/// ```ignore
/// // Discard your hand
/// let effect = DiscardHandEffect::you();
///
/// // Target player discards their hand
/// let effect = DiscardHandEffect::new(PlayerFilter::Any);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardHandEffect {
    /// The player who discards their hand.
    pub player: PlayerFilter,
}

impl DiscardHandEffect {
    /// Create a new discard hand effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller discards their hand.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Target opponent discards their hand.
    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl EffectExecutor for DiscardHandEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::event_processor::execute_discard;
        use crate::events::cause::EventCause;

        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        let hand_cards: Vec<_> = game
            .player(player_id)
            .map(|p| p.hand.clone())
            .unwrap_or_default();

        let count = hand_cards.len();

        // Discard each card using the event system
        // This is an EFFECT discard, so Library of Leng CAN apply
        let cause = EventCause::from_effect(ctx.source, ctx.controller);
        for card_id in hand_cards {
            execute_discard(
                game,
                card_id,
                player_id,
                cause.clone(),
                false,
                &mut *ctx.decision_maker,
            );
        }

        Ok(EffectOutcome::count(count as i32))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::effect::EffectResult;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_spell_card(card_id: u32, name: &str) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Instant])
            .build()
    }

    fn add_card_to_hand(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_spell_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Hand);
        game.add_object(obj); // add_object automatically updates player.hand for Zone::Hand
        id
    }

    #[test]
    fn test_discard_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add 3 cards to hand
        add_card_to_hand(&mut game, "Card 1", alice);
        add_card_to_hand(&mut game, "Card 2", alice);
        add_card_to_hand(&mut game, "Card 3", alice);

        assert_eq!(game.player(alice).unwrap().hand.len(), 3);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardHandEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
    }

    #[test]
    fn test_discard_hand_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        assert!(game.player(alice).unwrap().hand.is_empty());

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardHandEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_discard_hand_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Add cards to both players' hands
        add_card_to_hand(&mut game, "Alice Card", alice);
        add_card_to_hand(&mut game, "Bob Card 1", bob);
        add_card_to_hand(&mut game, "Bob Card 2", bob);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardHandEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 1); // Alice's hand unchanged
        assert_eq!(game.player(bob).unwrap().hand.len(), 0);
    }

    #[test]
    fn test_discard_hand_clone_box() {
        let effect = DiscardHandEffect::you();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DiscardHandEffect"));
    }
}
