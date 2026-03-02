//! Return to hand cost implementations.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::filter::{FilterContext, ObjectFilter};
use crate::game_state::GameState;
use crate::zone::Zone;

/// A return self to hand cost.
///
/// The source permanent returns to its owner's hand as part of the cost.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnSelfToHandCost;

impl ReturnSelfToHandCost {
    /// Create a new return self to hand cost.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReturnSelfToHandCost {
    fn default() -> Self {
        Self::new()
    }
}

impl CostPayer for ReturnSelfToHandCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let source = game
            .object(ctx.source)
            .ok_or(CostPaymentError::SourceNotFound)?;

        // Can only return to hand if on the battlefield
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

        // Return to hand
        game.move_object(ctx.source, Zone::Hand);

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "Return ~ to its owner's hand".to_string()
    }
}

/// A return another permanent to hand cost.
///
/// The player must return a permanent matching the filter to its owner's hand.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnToHandCost {
    /// Filter for which permanents can be returned.
    pub filter: ObjectFilter,
}

impl ReturnToHandCost {
    /// Create a new return to hand cost with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Create a cost to return any creature.
    pub fn creature() -> Self {
        Self::new(ObjectFilter::creature().you_control())
    }

    /// Create a cost to return another creature.
    pub fn another_creature() -> Self {
        Self::new(ObjectFilter::creature().you_control().other())
    }

    /// Create a cost to return any land.
    pub fn land() -> Self {
        Self::new(ObjectFilter::land().you_control())
    }

    /// Get valid return targets for this cost.
    pub fn get_valid_targets(
        &self,
        game: &GameState,
        player: crate::ids::PlayerId,
        source: crate::ids::ObjectId,
    ) -> Vec<crate::ids::ObjectId> {
        let ctx = FilterContext {
            you: Some(player),
            source: Some(source),
            ..Default::default()
        };
        game.battlefield
            .iter()
            .copied()
            .filter(|&id| {
                game.object(id)
                    .is_some_and(|obj| self.filter.matches(obj, &ctx, game))
            })
            .collect()
    }
}

impl CostPayer for ReturnToHandCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        // Check if there's at least one valid permanent to return
        let valid_targets = self.get_valid_targets(game, ctx.payer, ctx.source);

        if valid_targets.is_empty() {
            return Err(CostPaymentError::NoValidReturnTarget);
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

        // If target is pre-selected, consume it directly.
        if let Some(target_id) = ctx.pre_chosen_cards.first().copied() {
            let valid_targets = self.get_valid_targets(game, ctx.payer, ctx.source);
            if !valid_targets.contains(&target_id) {
                return Err(CostPaymentError::NoValidReturnTarget);
            }
            ctx.pre_chosen_cards.remove(0);
            game.move_object(target_id, Zone::Hand);
            return Ok(CostPaymentResult::Paid);
        }

        // The actual choice happens in the game loop
        Ok(CostPaymentResult::NeedsChoice(self.display()))
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        let mut parts = Vec::new();

        if self.filter.other {
            parts.push("another");
        }

        if self.filter.nontoken {
            parts.push("nontoken");
        }

        if self.filter.token {
            parts.push("token");
        }

        if !self.filter.card_types.is_empty() {
            let types: Vec<&str> = self
                .filter
                .card_types
                .iter()
                .map(|card_type| card_type.name())
                .collect();
            parts.push(Box::leak(types.join(" or ").into_boxed_str()));
        } else {
            parts.push("permanent");
        }

        format!(
            "Return a {} you control to its owner's hand",
            parts.join(" ")
        )
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::ReturnToHandTarget {
            filter: self.filter.clone(),
        }
    }
}
