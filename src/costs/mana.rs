//! Mana payment cost implementation.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;
use crate::mana::ManaCost;

/// A mana payment cost (e.g., {2}{W}{W}).
///
/// This wraps the existing ManaCost type and provides CostPayer implementation.
/// Mana payment typically happens through the mana payment phase in the game loop,
/// so the `pay` method here defers to the mana pool's `try_pay` method.
#[derive(Debug, Clone, PartialEq)]
pub struct ManaPaymentCost {
    /// The mana cost to pay.
    pub cost: ManaCost,
}

impl ManaPaymentCost {
    /// Create a new mana payment cost.
    pub fn new(cost: ManaCost) -> Self {
        Self { cost }
    }

    /// Get the wrapped mana cost.
    pub fn mana_cost(&self) -> &ManaCost {
        &self.cost
    }
}

impl CostPayer for ManaPaymentCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let player = game
            .player(ctx.payer)
            .ok_or(CostPaymentError::PlayerNotFound)?;

        let x_value = ctx.x_value.unwrap_or(0);

        let allow_any_color = game.can_spend_mana_as_any_color(ctx.payer, Some(ctx.source));

        if !player
            .mana_pool
            .can_pay_with_any_color(&self.cost, x_value, allow_any_color)
        {
            return Err(CostPaymentError::InsufficientMana);
        }

        Ok(())
    }

    fn can_potentially_pay(
        &self,
        game: &GameState,
        ctx: &CostContext,
    ) -> Result<(), CostPaymentError> {
        let x_value = ctx.x_value.unwrap_or(0);

        // Use the existing compute_potential_mana function
        let potential = crate::decision::compute_potential_mana(game, ctx.payer);

        let allow_any_color = game.can_spend_mana_as_any_color(ctx.payer, Some(ctx.source));

        if !potential.can_pay_with_any_color(&self.cost, x_value, allow_any_color) {
            return Err(CostPaymentError::InsufficientMana);
        }

        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        let x_value = ctx.x_value.unwrap_or(0);

        // Try to pay from the player's mana pool
        let allow_any_color = game.can_spend_mana_as_any_color(ctx.payer, Some(ctx.source));
        if let Some(player) = game.player_mut(ctx.payer) {
            if !player
                .mana_pool
                .try_pay_with_any_color(&self.cost, x_value, allow_any_color)
            {
                return Err(CostPaymentError::InsufficientMana);
            }
        } else {
            return Err(CostPaymentError::PlayerNotFound);
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        // Format the mana cost as a string
        format_mana_cost(&self.cost)
    }

    fn is_mana_cost(&self) -> bool {
        true
    }

    fn mana_cost(&self) -> Option<&crate::mana::ManaCost> {
        Some(&self.cost)
    }

    fn needs_player_choice(&self) -> bool {
        // Mana payment requires player to select which mana sources to tap
        true
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::ManaPayment {
            cost: self.cost.clone(),
        }
    }
}

/// Format a ManaCost for display.
fn format_mana_cost(cost: &ManaCost) -> String {
    use crate::mana::ManaSymbol;

    let mut parts = Vec::new();

    for pip in cost.pips() {
        if pip.len() == 1 {
            // Single option pip
            match pip[0] {
                ManaSymbol::White => parts.push("{W}".to_string()),
                ManaSymbol::Blue => parts.push("{U}".to_string()),
                ManaSymbol::Black => parts.push("{B}".to_string()),
                ManaSymbol::Red => parts.push("{R}".to_string()),
                ManaSymbol::Green => parts.push("{G}".to_string()),
                ManaSymbol::Colorless => parts.push("{C}".to_string()),
                ManaSymbol::Generic(n) => parts.push(format!("{{{}}}", n)),
                ManaSymbol::Snow => parts.push("{S}".to_string()),
                ManaSymbol::Life(n) => parts.push(format!("{{{}/P}}", n)),
                ManaSymbol::X => parts.push("{X}".to_string()),
            }
        } else {
            // Hybrid/alternative pip - format as {W/U}, {2/W}, etc.
            let alts: Vec<String> = pip
                .iter()
                .map(|s| match s {
                    ManaSymbol::White => "W".to_string(),
                    ManaSymbol::Blue => "U".to_string(),
                    ManaSymbol::Black => "B".to_string(),
                    ManaSymbol::Red => "R".to_string(),
                    ManaSymbol::Green => "G".to_string(),
                    ManaSymbol::Colorless => "C".to_string(),
                    ManaSymbol::Generic(n) => format!("{}", n),
                    ManaSymbol::Snow => "S".to_string(),
                    ManaSymbol::Life(n) => format!("P{}", n),
                    ManaSymbol::X => "X".to_string(),
                })
                .collect();
            parts.push(format!("{{{}}}", alts.join("/")));
        }
    }

    if parts.is_empty() {
        "{0}".to_string()
    } else {
        parts.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::ManaSymbol;

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_mana_cost_display() {
        let cost = ManaPaymentCost::new(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]));
        assert_eq!(cost.display(), "{2}{W}{W}");
    }

    #[test]
    fn test_mana_cost_display_empty() {
        let cost = ManaPaymentCost::new(ManaCost::new());
        assert_eq!(cost.display(), "{0}");
    }

    #[test]
    fn test_mana_cost_display_hybrid() {
        let cost = ManaPaymentCost::new(ManaCost::from_pips(vec![vec![
            ManaSymbol::White,
            ManaSymbol::Blue,
        ]]));
        assert_eq!(cost.display(), "{W/U}");
    }

    #[test]
    fn test_mana_cost_is_mana_cost() {
        let cost = ManaPaymentCost::new(ManaCost::from_symbols(vec![ManaSymbol::White]));
        assert!(cost.is_mana_cost());
        assert!(!cost.requires_tap());
    }

    #[test]
    fn test_mana_cost_can_pay_with_mana_in_pool() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Add mana to pool (using ManaSymbol, not Color)
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::White, 2);
        }

        let cost = ManaPaymentCost::new(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_mana_cost_cannot_pay_insufficient_mana() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Add only 1 white mana to pool
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::White, 1);
        }

        let cost = ManaPaymentCost::new(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientMana)
        );
    }

    #[test]
    fn test_mana_cost_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Add mana to pool
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::White, 2);
        }

        let cost = ManaPaymentCost::new(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        // Mana should be consumed
        assert!(
            !game
                .player(alice)
                .unwrap()
                .mana_pool
                .can_pay(&ManaCost::from_symbols(vec![ManaSymbol::White]), 0)
        );
    }

    #[test]
    fn test_mana_cost_clone_box() {
        let cost = ManaPaymentCost::new(ManaCost::from_symbols(vec![ManaSymbol::White]));
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("ManaPaymentCost"));
    }
}
