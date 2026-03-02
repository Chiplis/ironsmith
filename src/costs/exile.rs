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

        // If cards were pre-selected by the game loop, consume them directly.
        if !ctx.pre_chosen_cards.is_empty() {
            if ctx.pre_chosen_cards.len() < self.count as usize {
                return Err(CostPaymentError::InsufficientCardsInGraveyard);
            }

            let cards_to_exile: Vec<ObjectId> =
                ctx.pre_chosen_cards.drain(..self.count as usize).collect();

            let graveyard = game
                .player(ctx.payer)
                .ok_or(CostPaymentError::PlayerNotFound)?
                .graveyard
                .clone();
            for card_id in &cards_to_exile {
                if !graveyard.contains(card_id) {
                    return Err(CostPaymentError::InsufficientCardsInGraveyard);
                }
                if let Some(ct) = self.card_type
                    && !game.object(*card_id).is_some_and(|obj| obj.has_card_type(ct))
                {
                    return Err(CostPaymentError::InsufficientCardsInGraveyard);
                }
            }

            for card_id in cards_to_exile {
                game.move_object(card_id, Zone::Exile);
            }
            return Ok(CostPaymentResult::Paid);
        }

        // The actual choice happens in the game loop
        Ok(CostPaymentResult::NeedsChoice(self.display()))
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        let type_str = self
            .card_type
            .map_or("card".to_string(), |ct| ct.card_phrase().to_string());

        if self.count == 1 {
            format!("Exile a {} from your graveyard", type_str)
        } else {
            format!("Exile {} {}s from your graveyard", self.count, type_str)
        }
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::ExileFromGraveyard {
            count: self.count,
            card_type: self.card_type,
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
