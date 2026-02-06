//! Sacrifice cost implementations.

use crate::cost::{CostPaymentError, PermanentFilter};
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;
use crate::object::ObjectKind;
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

        // Move to graveyard
        game.move_object(ctx.source, Zone::Graveyard);

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
    pub filter: PermanentFilter,
}

impl SacrificeCost {
    /// Create a new sacrifice cost with the given filter.
    pub fn new(filter: PermanentFilter) -> Self {
        Self { filter }
    }

    /// Create a cost to sacrifice any creature.
    pub fn creature() -> Self {
        Self::new(PermanentFilter::creature())
    }

    /// Create a cost to sacrifice another creature.
    pub fn another_creature() -> Self {
        Self::new(PermanentFilter::creature().other())
    }

    /// Create a cost to sacrifice any artifact.
    pub fn artifact() -> Self {
        Self::new(PermanentFilter::artifact())
    }

    /// Create a cost to sacrifice any permanent.
    pub fn any() -> Self {
        Self::new(PermanentFilter::any())
    }

    /// Get valid sacrifice targets for this cost.
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

    fn sacrifice_filter(&self) -> Option<&crate::cost::PermanentFilter> {
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

    // === SacrificeSelfCost tests ===

    #[test]
    fn test_sacrifice_self_display() {
        let cost = SacrificeSelfCost::new();
        assert_eq!(cost.display(), "Sacrifice ~");
    }

    #[test]
    fn test_sacrifice_self_can_pay_on_battlefield() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let creature = creature_card("Bear", 1);
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

        let cost = SacrificeSelfCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(creature_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_sacrifice_self_cannot_pay_not_on_battlefield() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let creature = creature_card("Bear", 1);
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);

        let cost = SacrificeSelfCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(creature_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::SourceNotOnBattlefield)
        );
    }

    #[test]
    fn test_sacrifice_self_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let creature = creature_card("Bear", 1);
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

        let cost = SacrificeSelfCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(creature_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        // Creature should be in graveyard now (or moved)
        // Note: move_object returns the new ID, creature_id may no longer exist
        assert!(
            game.object(creature_id).is_none()
                || game.object(creature_id).map(|o| o.zone) != Some(Zone::Battlefield)
        );
    }

    // === SacrificeCost tests ===

    #[test]
    fn test_sacrifice_creature_display() {
        let cost = SacrificeCost::creature();
        assert_eq!(cost.display(), "Sacrifice a creature");
    }

    #[test]
    fn test_sacrifice_another_creature_display() {
        let cost = SacrificeCost::another_creature();
        assert_eq!(cost.display(), "Sacrifice a another creature");
    }

    #[test]
    fn test_sacrifice_artifact_display() {
        let cost = SacrificeCost::artifact();
        assert_eq!(cost.display(), "Sacrifice a artifact");
    }

    #[test]
    fn test_sacrifice_cost_can_pay_with_valid_target() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source = creature_card("Source", 1);
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        let target = creature_card("Target", 2);
        let _target_id = game.create_object_from_card(&target, alice, Zone::Battlefield);

        let cost = SacrificeCost::another_creature();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_sacrifice_cost_cannot_pay_without_valid_target() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source = creature_card("Source", 1);
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        // No other creatures - only the source
        let cost = SacrificeCost::another_creature();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::NoValidSacrificeTarget)
        );
    }

    #[test]
    fn test_sacrifice_cost_get_valid_targets() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = creature_card("Source", 1);
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        let alice_creature = creature_card("Alice's Creature", 2);
        let alice_creature_id =
            game.create_object_from_card(&alice_creature, alice, Zone::Battlefield);

        let bob_creature = creature_card("Bob's Creature", 3);
        let _bob_creature_id = game.create_object_from_card(&bob_creature, bob, Zone::Battlefield);

        let cost = SacrificeCost::another_creature();
        let targets = cost.get_valid_targets(&game, alice, source_id);

        // Should only include Alice's other creature, not the source or Bob's creature
        assert_eq!(targets.len(), 1);
        assert!(targets.contains(&alice_creature_id));
    }

    #[test]
    fn test_sacrifice_cost_pay_returns_needs_choice() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source = creature_card("Source", 1);
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        let target = creature_card("Target", 2);
        let _target_id = game.create_object_from_card(&target, alice, Zone::Battlefield);

        let cost = SacrificeCost::another_creature();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert!(matches!(result, Ok(CostPaymentResult::NeedsChoice(_))));
    }

    #[test]
    fn test_sacrifice_cost_clone_box() {
        let cost = SacrificeCost::creature();
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("SacrificeCost"));
    }
}
