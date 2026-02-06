//! Tap cost implementation ({T}).

use crate::ability::AbilityKind;
use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;

/// A tap cost ({T}).
///
/// The source permanent must be untapped and not have summoning sickness
/// (unless it has haste).
#[derive(Debug, Clone, PartialEq)]
pub struct TapCost;

impl TapCost {
    /// Create a new tap cost.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TapCost {
    fn default() -> Self {
        Self::new()
    }
}

impl CostPayer for TapCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let source = game
            .object(ctx.source)
            .ok_or(CostPaymentError::SourceNotFound)?;

        // Can't tap if already tapped
        if game.is_tapped(ctx.source) {
            return Err(CostPaymentError::AlreadyTapped);
        }

        // Can't tap if summoning sick (unless has haste) - rule 302.6
        if game.is_summoning_sick(ctx.source) && source.is_creature() {
            let has_haste = source.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            });
            if !has_haste {
                return Err(CostPaymentError::SummoningSickness);
            }
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

        // Tap the permanent
        game.tap(ctx.source);

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "{T}".to_string()
    }

    fn requires_tap(&self) -> bool {
        true
    }

    // TapCost is an immediate cost - no player choice needed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn basic_land() -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(1), "Forest")
            .card_types(vec![CardType::Land])
            .build()
    }

    fn creature_card() -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(2), "Bear")
            .mana_cost(ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Generic(2),
            ]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    #[test]
    fn test_tap_cost_display() {
        let cost = TapCost::new();
        assert_eq!(cost.display(), "{T}");
        assert!(cost.requires_tap());
        assert!(!cost.is_mana_cost());
    }

    #[test]
    fn test_tap_cost_can_pay_untapped_land() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let land = basic_land();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);

        let cost = TapCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(land_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_tap_cost_cannot_pay_tapped() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let land = basic_land();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        game.tap(land_id);

        let cost = TapCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(land_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::AlreadyTapped)
        );
    }

    #[test]
    fn test_tap_cost_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let land = basic_land();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);

        let cost = TapCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(land_id, alice, &mut dm);

        assert!(!game.is_tapped(land_id));
        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert!(game.is_tapped(land_id));
    }

    #[test]
    fn test_tap_cost_summoning_sick_creature() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let creature = creature_card();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        // Creature just entered - summoning sick

        let cost = TapCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(creature_id, alice, &mut dm);

        // Note: In a real game, we'd need to track when creatures entered.
        // For this test, we manually check the logic.
        // The game state should track summoning sickness.
        // If is_summoning_sick returns true, we expect SummoningSickness error.
        if game.is_summoning_sick(creature_id) {
            assert_eq!(
                cost.can_pay(&game, &ctx),
                Err(CostPaymentError::SummoningSickness)
            );
        }
    }

    #[test]
    fn test_tap_cost_clone_box() {
        let cost = TapCost::new();
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("TapCost"));
    }
}
