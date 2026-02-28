//! Sacrifice cost implementations.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::events::permanents::SacrificeEvent;
use crate::filter::{FilterContext, ObjectFilter};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// A sacrifice self cost.
///
/// The source permanent sacrifices itself as part of the cost.
#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeSelfCost;

impl SacrificeSelfCost {
    /// Create a new sacrifice self cost.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SacrificeSelfCost {
    fn default() -> Self {
        Self::new()
    }
}

impl CostPayer for SacrificeSelfCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let source = game
            .object(ctx.source)
            .ok_or(CostPaymentError::SourceNotFound)?;

        // Can only sacrifice if on the battlefield
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

        let snapshot = game
            .object(ctx.source)
            .map(|obj| ObjectSnapshot::from_object(obj, game));
        let sacrificing_player = snapshot
            .as_ref()
            .map(|snap| snap.controller)
            .or(Some(ctx.payer));

        // Move to graveyard
        game.move_object(ctx.source, Zone::Graveyard);
        game.queue_trigger_event(TriggerEvent::new(
            SacrificeEvent::new(ctx.source, Some(ctx.source))
                .with_snapshot(snapshot, sacrificing_player),
        ));

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "Sacrifice ~".to_string()
    }

    fn is_sacrifice_self(&self) -> bool {
        true
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::InlineWithTriggers
    }
}

/// A sacrifice another permanent cost.
///
/// The player must sacrifice a permanent matching the filter.
/// The actual choice of which permanent to sacrifice is handled
/// during cost payment phase by the game loop.
#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeCost {
    /// Filter for which permanents can be sacrificed.
    pub filter: ObjectFilter,
}

impl SacrificeCost {
    /// Create a new sacrifice cost with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Create a cost to sacrifice any creature.
    pub fn creature() -> Self {
        Self::new(ObjectFilter::creature().you_control())
    }

    /// Create a cost to sacrifice another creature.
    pub fn another_creature() -> Self {
        Self::new(ObjectFilter::creature().you_control().other())
    }

    /// Create a cost to sacrifice any artifact.
    pub fn artifact() -> Self {
        Self::new(ObjectFilter::artifact().you_control())
    }

    /// Create a cost to sacrifice any permanent.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent().you_control())
    }

    /// Get valid sacrifice targets for this cost.
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

impl CostPayer for SacrificeCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        // Check if there's at least one valid permanent to sacrifice
        let valid_targets = self.get_valid_targets(game, ctx.payer, ctx.source);

        if valid_targets.is_empty() {
            return Err(CostPaymentError::NoValidSacrificeTarget);
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

        // The actual choice and sacrifice happens in the game loop's cost payment phase.
        // This just signals that a choice is needed.
        let filter_description = self.display();
        Ok(CostPaymentResult::NeedsChoice(filter_description))
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
                .map(|t| match t {
                    crate::types::CardType::Creature => "creature",
                    crate::types::CardType::Artifact => "artifact",
                    crate::types::CardType::Enchantment => "enchantment",
                    crate::types::CardType::Land => "land",
                    crate::types::CardType::Planeswalker => "planeswalker",
                    crate::types::CardType::Instant => "instant",
                    crate::types::CardType::Sorcery => "sorcery",
                    crate::types::CardType::Battle => "battle",
                    crate::types::CardType::Kindred => "kindred",
                })
                .collect();
            parts.push(Box::leak(types.join(" or ").into_boxed_str()));
        } else {
            parts.push("permanent");
        }

        format!("Sacrifice a {}", parts.join(" "))
    }

    fn is_sacrifice(&self) -> bool {
        true
    }

    fn sacrifice_filter(&self) -> Option<&ObjectFilter> {
        Some(&self.filter)
    }

    fn needs_player_choice(&self) -> bool {
        // Player needs to choose which permanent to sacrifice
        true
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::SacrificeTarget {
            filter: self.filter.clone(),
        }
    }
}
