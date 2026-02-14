//! Discard cost implementations.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;
use crate::types::CardType;
use crate::zone::Zone;

/// A discard cards cost.
///
/// The player must discard a number of cards, optionally of a specific type.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardCost {
    /// The number of cards to discard.
    pub count: u32,
    /// Optional card type restriction.
    pub card_type: Option<CardType>,
}

impl DiscardCost {
    /// Create a new discard cost.
    pub fn new(count: u32, card_type: Option<CardType>) -> Self {
        Self { count, card_type }
    }

    /// Create a cost to discard any cards.
    pub fn any(count: u32) -> Self {
        Self::new(count, None)
    }

    /// Create a cost to discard one card.
    pub fn one() -> Self {
        Self::new(1, None)
    }

    /// Get the number of valid cards in hand for this cost.
    pub fn count_valid_cards(
        &self,
        game: &GameState,
        player: crate::ids::PlayerId,
        source: crate::ids::ObjectId,
    ) -> usize {
        let Some(player_obj) = game.player(player) else {
            return 0;
        };

        player_obj
            .hand
            .iter()
            .filter(|&&card_id| {
                // Don't count the spell being cast
                if card_id == source {
                    return false;
                }
                // Check card type filter
                if let Some(ct) = self.card_type {
                    if let Some(obj) = game.object(card_id) {
                        obj.has_card_type(ct)
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .count()
    }
}

impl CostPayer for DiscardCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let valid_count = self.count_valid_cards(game, ctx.payer, ctx.source);

        if valid_count < self.count as usize {
            return Err(CostPaymentError::InsufficientCardsInHand);
        }

        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        // Verify we can still pay
        self.can_pay(game, ctx)?;

        // The actual discard choice happens in the game loop
        Ok(CostPaymentResult::NeedsChoice(self.display()))
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        let type_str = self.card_type.map_or("card".to_string(), |ct| {
            match ct {
                CardType::Creature => "creature card",
                CardType::Artifact => "artifact card",
                CardType::Enchantment => "enchantment card",
                CardType::Land => "land card",
                CardType::Planeswalker => "planeswalker card",
                CardType::Instant => "instant card",
                CardType::Sorcery => "sorcery card",
                CardType::Battle => "battle card",
                CardType::Kindred => "kindred card",
            }
            .to_string()
        });

        if self.count == 1 {
            format!("Discard a {}", type_str)
        } else {
            format!("Discard {} {}s", self.count, type_str)
        }
    }

    fn is_discard(&self) -> bool {
        true
    }

    fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        Some((self.count, self.card_type))
    }

    fn needs_player_choice(&self) -> bool {
        // Player needs to choose which cards to discard
        true
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::DiscardCards {
            count: self.count,
            card_type: self.card_type,
        }
    }
}

/// A discard your hand cost.
///
/// The player must discard their entire hand.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardHandCost;

impl DiscardHandCost {
    /// Create a new discard hand cost.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiscardHandCost {
    fn default() -> Self {
        Self::new()
    }
}

impl CostPayer for DiscardHandCost {
    fn can_pay(&self, _game: &GameState, _ctx: &CostContext) -> Result<(), CostPaymentError> {
        // Can always discard hand (even if empty)
        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        use crate::event_processor::execute_discard;
        use crate::events::cause::EventCause;

        // Discard all cards in hand using the event system
        // This is a COST discard, so Library of Leng does NOT apply
        if let Some(player) = game.player(ctx.payer) {
            let hand_cards: Vec<_> = player.hand.clone();
            let cause = EventCause::from_cost(ctx.source, ctx.payer);
            for card_id in hand_cards {
                execute_discard(
                    game,
                    card_id,
                    ctx.payer,
                    cause.clone(),
                    false,
                    ctx.decision_maker,
                );
            }
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "Discard your hand".to_string()
    }
}

/// A discard-this-card cost.
///
/// Used by hand-zone activated abilities like Cycling/Bloodrush where the
/// source card itself must be discarded as part of paying the cost.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardSourceCost;

impl DiscardSourceCost {
    /// Create a new discard source cost.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiscardSourceCost {
    fn default() -> Self {
        Self::new()
    }
}

impl CostPayer for DiscardSourceCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let source = game
            .object(ctx.source)
            .ok_or(CostPaymentError::SourceNotFound)?;
        let player = game
            .player(ctx.payer)
            .ok_or(CostPaymentError::PlayerNotFound)?;

        if source.zone != Zone::Hand || !player.hand.contains(&ctx.source) {
            return Err(CostPaymentError::InsufficientCardsInHand);
        }

        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        use crate::event_processor::execute_discard;
        use crate::events::cause::EventCause;

        self.can_pay(game, ctx)?;

        let cause = EventCause::from_cost(ctx.source, ctx.payer);
        let result = execute_discard(
            game,
            ctx.source,
            ctx.payer,
            cause,
            false,
            ctx.decision_maker,
        );
        if result.prevented {
            return Err(CostPaymentError::Other(
                "Discard cost was prevented".to_string(),
            ));
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "Discard this card".to_string()
    }

    fn is_discard(&self) -> bool {
        true
    }

    fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        Some((1, None))
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::Immediate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::zone::Zone;

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn simple_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .build()
    }

    // === DiscardCost tests ===

    #[test]
    fn test_discard_cost_display() {
        assert_eq!(DiscardCost::one().display(), "Discard a card");
        assert_eq!(DiscardCost::any(2).display(), "Discard 2 cards");
        assert_eq!(
            DiscardCost::new(1, Some(CardType::Creature)).display(),
            "Discard a creature card"
        );
    }

    #[test]
    fn test_discard_cost_can_pay_with_cards() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Add cards to hand
        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Hand);
        let card2 = simple_card("Card 2", 2);
        let _id2 = game.create_object_from_card(&card2, alice, Zone::Hand);

        let cost = DiscardCost::any(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_discard_cost_cannot_pay_insufficient_cards() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Add only one card to hand
        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Hand);

        let cost = DiscardCost::any(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientCardsInHand)
        );
    }

    #[test]
    fn test_discard_cost_excludes_source() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        // Create the source card in hand
        let source_card = simple_card("Source Card", 1);
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Hand);

        // Add one other card to hand
        let card2 = simple_card("Card 2", 2);
        let _id2 = game.create_object_from_card(&card2, alice, Zone::Hand);

        let cost = DiscardCost::any(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source_id, alice, &mut dm);

        // Should fail because we only have 1 card (excluding source)
        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientCardsInHand)
        );
    }

    #[test]
    fn test_discard_cost_pay_returns_needs_choice() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Hand);

        let cost = DiscardCost::one();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert!(matches!(result, Ok(CostPaymentResult::NeedsChoice(_))));
    }

    // === DiscardHandCost tests ===

    #[test]
    fn test_discard_hand_display() {
        let cost = DiscardHandCost::new();
        assert_eq!(cost.display(), "Discard your hand");
    }

    #[test]
    fn test_discard_hand_can_always_pay() {
        let game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        let cost = DiscardHandCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        // Can pay even with empty hand
        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_discard_hand_pay_empties_hand() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Add cards to hand
        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Hand);
        let card2 = simple_card("Card 2", 2);
        let _id2 = game.create_object_from_card(&card2, alice, Zone::Hand);

        assert_eq!(game.player(alice).unwrap().hand.len(), 2);

        let cost = DiscardHandCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        // Hand should be empty now
        assert!(game.player(alice).unwrap().hand.is_empty());
    }

    #[test]
    fn test_discard_cost_clone_box() {
        let cost = DiscardCost::one();
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("DiscardCost"));
    }

    // === DiscardSourceCost tests ===

    #[test]
    fn test_discard_source_cost_display() {
        let cost = DiscardSourceCost::new();
        assert_eq!(cost.display(), "Discard this card");
    }

    #[test]
    fn test_discard_source_cost_can_pay_when_source_in_hand() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source_card = simple_card("Cycling Source", 77);
        let source = game.create_object_from_card(&source_card, alice, Zone::Hand);

        let cost = DiscardSourceCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_discard_source_cost_pay_moves_source_to_graveyard() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source_card = simple_card("Bloodrush Source", 88);
        let source = game.create_object_from_card(&source_card, alice, Zone::Hand);

        let cost = DiscardSourceCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        let player = game.player(alice).expect("player exists");
        assert!(
            !player.hand.contains(&source),
            "source card should no longer be in hand"
        );
        assert!(
            !player.graveyard.is_empty(),
            "source card should be in graveyard (possibly with a new id)"
        );
    }

    #[test]
    fn test_discard_source_cost_clone_box() {
        let cost = DiscardSourceCost::new();
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("DiscardSourceCost"));
    }
}
