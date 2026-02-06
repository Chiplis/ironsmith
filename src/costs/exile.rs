//! Exile cost implementations.

use crate::color::ColorSet;
use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::types::CardType;
use crate::zone::Zone;

/// An exile self cost.
///
/// The source permanent exiles itself as part of the cost.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileSelfCost;

impl ExileSelfCost {
    /// Create a new exile self cost.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExileSelfCost {
    fn default() -> Self {
        Self::new()
    }
}

impl CostPayer for ExileSelfCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let source = game
            .object(ctx.source)
            .ok_or(CostPaymentError::SourceNotFound)?;

        // Can only exile self if on the battlefield
        if source.zone != Zone::Battlefield {
            return Err(CostPaymentError::SourceNotOnBattlefield);
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

        // Move to exile
        game.move_object(ctx.source, Zone::Exile);

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "Exile ~".to_string()
    }
}

/// An exile cards from graveyard cost.
///
/// The player must exile cards from their graveyard.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileFromGraveyardCost {
    /// The number of cards to exile.
    pub count: u32,
    /// Optional card type restriction.
    pub card_type: Option<CardType>,
}

impl ExileFromGraveyardCost {
    /// Create a new exile from graveyard cost.
    pub fn new(count: u32, card_type: Option<CardType>) -> Self {
        Self { count, card_type }
    }

    /// Create a cost to exile any cards from graveyard.
    pub fn any(count: u32) -> Self {
        Self::new(count, None)
    }

    /// Get the number of valid cards in graveyard for this cost.
    pub fn count_valid_cards(&self, game: &GameState, player: crate::ids::PlayerId) -> usize {
        let Some(player_obj) = game.player(player) else {
            return 0;
        };

        player_obj
            .graveyard
            .iter()
            .filter(|&&card_id| {
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

impl CostPayer for ExileFromGraveyardCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let valid_count = self.count_valid_cards(game, ctx.payer);

        if valid_count < self.count as usize {
            return Err(CostPaymentError::InsufficientCardsInGraveyard);
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

        // The actual choice happens in the game loop
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
            format!("Exile a {} from your graveyard", type_str)
        } else {
            format!("Exile {} {}s from your graveyard", self.count, type_str)
        }
    }
}

/// An exile cards from hand cost (e.g., Force of Will's "exile a blue card").
///
/// The player must exile cards from their hand matching the color filter.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileFromHandCost {
    /// The number of cards to exile.
    pub count: u32,
    /// Optional color filter (card must have at least one of these colors).
    pub color_filter: Option<ColorSet>,
}

impl ExileFromHandCost {
    /// Create a new exile from hand cost.
    pub fn new(count: u32, color_filter: Option<ColorSet>) -> Self {
        Self {
            count,
            color_filter,
        }
    }

    /// Create a cost to exile any cards from hand.
    pub fn any(count: u32) -> Self {
        Self::new(count, None)
    }

    /// Create a cost to exile a card of a specific color.
    pub fn colored(count: u32, colors: ColorSet) -> Self {
        Self::new(count, Some(colors))
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
                // Don't count the card being cast
                if card_id == source {
                    return false;
                }
                // Check color filter
                if let Some(required_colors) = self.color_filter {
                    if let Some(obj) = game.object(card_id) {
                        let card_colors = obj.colors();
                        !card_colors.intersection(required_colors).is_empty()
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

impl CostPayer for ExileFromHandCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let valid_count = self.count_valid_cards(game, ctx.payer, ctx.source);

        if valid_count < self.count as usize {
            return Err(CostPaymentError::InsufficientCardsToExile);
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

        // If we have pre-chosen cards, use them directly
        if !ctx.pre_chosen_cards.is_empty() {
            if ctx.pre_chosen_cards.len() < self.count as usize {
                return Err(CostPaymentError::InsufficientCardsToExile);
            }

            // Take the required number of cards
            let cards_to_exile: Vec<ObjectId> =
                ctx.pre_chosen_cards.drain(..self.count as usize).collect();

            // Exile the cards
            for card_id in cards_to_exile {
                game.move_object(card_id, Zone::Exile);
            }

            return Ok(CostPaymentResult::Paid);
        }

        // Otherwise, the actual choice happens in the game loop
        Ok(CostPaymentResult::NeedsChoice(self.display()))
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        use crate::color::Color;

        let color_str = if let Some(colors) = self.color_filter {
            let color_names: Vec<&str> = [
                (colors.contains(Color::White), "white"),
                (colors.contains(Color::Blue), "blue"),
                (colors.contains(Color::Black), "black"),
                (colors.contains(Color::Red), "red"),
                (colors.contains(Color::Green), "green"),
            ]
            .iter()
            .filter_map(|(has, name)| if *has { Some(*name) } else { None })
            .collect();

            if color_names.is_empty() {
                "".to_string()
            } else {
                format!("{} ", color_names.join(" or "))
            }
        } else {
            "".to_string()
        };

        if self.count == 1 {
            format!("Exile a {}card from your hand", color_str)
        } else {
            format!("Exile {} {}cards from your hand", self.count, color_str)
        }
    }

    fn is_exile_from_hand(&self) -> bool {
        true
    }

    fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        Some((self.count, self.color_filter))
    }

    fn needs_player_choice(&self) -> bool {
        // Player needs to choose which cards to exile (unless pre-chosen)
        true
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::ExileFromHand {
            count: self.count,
            color_filter: self.color_filter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn simple_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .build()
    }

    fn blue_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build()
    }

    // === ExileSelfCost tests ===

    #[test]
    fn test_exile_self_display() {
        let cost = ExileSelfCost::new();
        assert_eq!(cost.display(), "Exile ~");
    }

    #[test]
    fn test_exile_self_can_pay_on_battlefield() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        let cost = ExileSelfCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(card_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_exile_self_cannot_pay_in_hand() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Hand);

        let cost = ExileSelfCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(card_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::SourceNotOnBattlefield)
        );
    }

    #[test]
    fn test_exile_self_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        let cost = ExileSelfCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
    }

    // === ExileFromGraveyardCost tests ===

    #[test]
    fn test_exile_from_graveyard_display() {
        assert_eq!(
            ExileFromGraveyardCost::any(1).display(),
            "Exile a card from your graveyard"
        );
        assert_eq!(
            ExileFromGraveyardCost::any(3).display(),
            "Exile 3 cards from your graveyard"
        );
    }

    #[test]
    fn test_exile_from_graveyard_can_pay() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Add cards to graveyard
        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Graveyard);
        let card2 = simple_card("Card 2", 2);
        let _id2 = game.create_object_from_card(&card2, alice, Zone::Graveyard);

        let cost = ExileFromGraveyardCost::any(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_exile_from_graveyard_cannot_pay_insufficient() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Add only one card to graveyard
        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Graveyard);

        let cost = ExileFromGraveyardCost::any(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientCardsInGraveyard)
        );
    }

    // === ExileFromHandCost tests ===

    #[test]
    fn test_exile_from_hand_display() {
        assert_eq!(
            ExileFromHandCost::any(1).display(),
            "Exile a card from your hand"
        );
        assert_eq!(
            ExileFromHandCost::colored(1, ColorSet::from_color(Color::Blue)).display(),
            "Exile a blue card from your hand"
        );
    }

    #[test]
    fn test_exile_from_hand_can_pay() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Hand);

        let cost = ExileFromHandCost::any(1);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_exile_from_hand_color_filter() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Add a blue card
        let card = blue_card("Blue Card", 1);
        let _id = game.create_object_from_card(&card, alice, Zone::Hand);

        let cost = ExileFromHandCost::colored(1, ColorSet::from_color(Color::Blue));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_exile_from_hand_excludes_source() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        // Create the source card in hand
        let source_card = simple_card("Source", 1);
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Hand);

        let cost = ExileFromHandCost::any(1);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source_id, alice, &mut dm);

        // Should fail because the only card is the source
        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientCardsToExile)
        );
    }

    #[test]
    fn test_exile_cost_clone_box() {
        let cost = ExileSelfCost::new();
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("ExileSelfCost"));
    }
}
