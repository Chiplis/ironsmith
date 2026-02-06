//! Untap cost implementation ({Q}).

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;

/// An untap cost ({Q}).
///
/// The source permanent must be tapped to pay this cost.
/// This is used by cards like Knacksaw Clique.
#[derive(Debug, Clone, PartialEq)]
pub struct UntapCost;

impl UntapCost {
    /// Create a new untap cost.
    pub fn new() -> Self {
        Self
    }
}

impl Default for UntapCost {
    fn default() -> Self {
        Self::new()
    }
}

impl CostPayer for UntapCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let _source = game
            .object(ctx.source)
            .ok_or(CostPaymentError::SourceNotFound)?;

        // Can't untap if already untapped
        if !game.is_tapped(ctx.source) {
            return Err(CostPaymentError::AlreadyUntapped);
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

        // Untap the permanent
        game.untap(ctx.source);

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "{Q}".to_string()
    }

    fn requires_untap(&self) -> bool {
        true
    }

    // UntapCost is an immediate cost - no player choice needed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
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

    #[test]
    fn test_untap_cost_display() {
        let cost = UntapCost::new();
        assert_eq!(cost.display(), "{Q}");
        assert!(!cost.requires_tap());
        assert!(!cost.is_mana_cost());
    }

    #[test]
    fn test_untap_cost_can_pay_tapped() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let land = basic_land();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        game.tap(land_id); // Must be tapped to untap

        let cost = UntapCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(land_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_untap_cost_cannot_pay_untapped() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let land = basic_land();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        // Land is already untapped

        let cost = UntapCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(land_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::AlreadyUntapped)
        );
    }

    #[test]
    fn test_untap_cost_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let land = basic_land();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        game.tap(land_id);

        let cost = UntapCost::new();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(land_id, alice, &mut dm);

        assert!(game.is_tapped(land_id));
        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert!(!game.is_tapped(land_id));
    }

    #[test]
    fn test_untap_cost_clone_box() {
        let cost = UntapCost::new();
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("UntapCost"));
    }
}
