//! Return to hand cost implementations.

use crate::cost::{CostPaymentError, PermanentFilter};
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;
use crate::object::ObjectKind;
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
    pub filter: PermanentFilter,
}

impl ReturnToHandCost {
    /// Create a new return to hand cost with the given filter.
    pub fn new(filter: PermanentFilter) -> Self {
        Self { filter }
    }

    /// Create a cost to return any creature.
    pub fn creature() -> Self {
        Self::new(PermanentFilter::creature())
    }

    /// Create a cost to return another creature.
    pub fn another_creature() -> Self {
        Self::new(PermanentFilter::creature().other())
    }

    /// Create a cost to return any land.
    pub fn land() -> Self {
        Self::new(PermanentFilter::land())
    }

    /// Get valid return targets for this cost.
    pub fn get_valid_targets(
        &self,
        game: &GameState,
        player: crate::ids::PlayerId,
        source: crate::ids::ObjectId,
    ) -> Vec<crate::ids::ObjectId> {
        game.battlefield
            .iter()
            .filter(|&&id| {
                if let Some(obj) = game.object(id) {
                    // Must be controlled by player
                    if obj.controller != player {
                        return false;
                    }
                    // Check "other" requirement
                    if self.filter.other && id == source {
                        return false;
                    }
                    // Check card type filter
                    if !self.filter.card_types.is_empty()
                        && !self.filter.card_types.iter().any(|t| obj.has_card_type(*t))
                    {
                        return false;
                    }
                    // Check subtype filter
                    if !self.filter.subtypes.is_empty()
                        && !self
                            .filter
                            .subtypes
                            .iter()
                            .any(|s| obj.subtypes.contains(s))
                    {
                        return false;
                    }
                    // Check token/nontoken
                    if self.filter.token && obj.kind != ObjectKind::Token {
                        return false;
                    }
                    if self.filter.nontoken && obj.kind == ObjectKind::Token {
                        return false;
                    }
                    true
                } else {
                    false
                }
            })
            .copied()
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

        format!(
            "Return a {} you control to its owner's hand",
            parts.join(" ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::CardType;

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn creature_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Generic(2),
            ]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    // === ReturnSelfToHandCost tests ===

    #[test]
    fn test_return_self_display() {
        let cost = ReturnSelfToHandCost::new();
        assert_eq!(cost.display(), "Return ~ to its owner's hand");
    }

    #[test]
    fn test_return_self_can_pay_on_battlefield() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let creature = creature_card("Bear", 1);
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

        let cost = ReturnSelfToHandCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(creature_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_return_self_cannot_pay_in_graveyard() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let creature = creature_card("Bear", 1);
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Graveyard);

        let cost = ReturnSelfToHandCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(creature_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::SourceNotOnBattlefield)
        );
    }

    #[test]
    fn test_return_self_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let creature = creature_card("Bear", 1);
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

        let cost = ReturnSelfToHandCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(creature_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
    }

    // === ReturnToHandCost tests ===

    #[test]
    fn test_return_creature_display() {
        let cost = ReturnToHandCost::creature();
        assert_eq!(
            cost.display(),
            "Return a creature you control to its owner's hand"
        );
    }

    #[test]
    fn test_return_another_creature_display() {
        let cost = ReturnToHandCost::another_creature();
        assert_eq!(
            cost.display(),
            "Return a another creature you control to its owner's hand"
        );
    }

    #[test]
    fn test_return_cost_can_pay_with_valid_target() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source = creature_card("Source", 1);
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        let target = creature_card("Target", 2);
        let _target_id = game.create_object_from_card(&target, alice, Zone::Battlefield);

        let cost = ReturnToHandCost::another_creature();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_return_cost_cannot_pay_without_valid_target() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source = creature_card("Source", 1);
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        // No other creatures
        let cost = ReturnToHandCost::another_creature();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::NoValidReturnTarget)
        );
    }

    #[test]
    fn test_return_cost_pay_returns_needs_choice() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source = creature_card("Source", 1);
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        let target = creature_card("Target", 2);
        let _target_id = game.create_object_from_card(&target, alice, Zone::Battlefield);

        let cost = ReturnToHandCost::another_creature();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert!(matches!(result, Ok(CostPaymentResult::NeedsChoice(_))));
    }

    #[test]
    fn test_return_cost_clone_box() {
        let cost = ReturnSelfToHandCost::new();
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("ReturnSelfToHandCost"));
    }
}
