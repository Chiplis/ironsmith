//! Exile from hand as cost effect.
//!
//! This effect exiles cards from the controller's hand as part of paying a cost,
//! such as Force of Will's alternative cost. Unlike regular exile effects,
//! this operates on the controller's hand and may require player choice.

use crate::color::ColorSet;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::zone::Zone;

/// Effect that exiles cards from the controller's hand as a cost.
///
/// This is used for alternative casting costs like Force of Will's
/// "exile a blue card from your hand" cost.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileFromHandAsCostEffect {
    /// Number of cards to exile
    pub count: u32,
    /// Optional color filter (e.g., "exile a blue card")
    pub color_filter: Option<ColorSet>,
}

impl ExileFromHandAsCostEffect {
    /// Create a new exile from hand as cost effect.
    pub fn new(count: u32, color_filter: Option<ColorSet>) -> Self {
        Self {
            count,
            color_filter,
        }
    }

    /// Get cards in hand that match the color filter.
    fn get_matching_cards(&self, game: &GameState, ctx: &ExecutionContext) -> Vec<ObjectId> {
        let controller = ctx.controller;
        let source_id = ctx.source;

        let Some(player) = game.player(controller) else {
            return Vec::new();
        };

        player
            .hand
            .iter()
            .filter(|&&card_id| {
                // Can't exile the source card itself (the spell being cast)
                if card_id == source_id {
                    return false;
                }

                // Apply color filter if present
                if let Some(filter) = &self.color_filter {
                    if let Some(card) = game.object(card_id) {
                        let card_colors = card.colors();
                        !card_colors.intersection(*filter).is_empty()
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .copied()
            .collect()
    }
}

impl EffectExecutor for ExileFromHandAsCostEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Auto-select: get matching cards and take the required number
        let matching = self.get_matching_cards(game, ctx);

        if (matching.len() as u32) < self.count {
            return Err(ExecutionError::Impossible(
                "Not enough matching cards in hand to exile".to_string(),
            ));
        }

        // Auto-select the first N matching cards
        let cards_to_exile: Vec<ObjectId> =
            matching.into_iter().take(self.count as usize).collect();

        // Exile each chosen card
        let count_exiled = cards_to_exile.len() as i32;
        for card_id in cards_to_exile {
            game.move_object(card_id, Zone::Exile);
        }

        Ok(EffectOutcome::from_result(EffectResult::Count(
            count_exiled,
        )))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn exile_from_hand_cost_info(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        Some((self.count, self.color_filter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    #[test]
    fn test_new() {
        let effect = ExileFromHandAsCostEffect::new(1, None);
        assert_eq!(effect.count, 1);
        assert!(effect.color_filter.is_none());

        let effect_blue = ExileFromHandAsCostEffect::new(1, Some(ColorSet::from(Color::Blue)));
        assert_eq!(effect_blue.count, 1);
        assert!(effect_blue.color_filter.is_some());
    }
}
