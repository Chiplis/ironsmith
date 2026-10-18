//! Add mana of any one color effect implementation.


use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub use ironsmith_core::AddManaOfAnyOneColorEffect;


/// Effect that adds mana of any ONE color to a player's mana pool.
///
/// Unlike `AddManaOfAnyColorEffect`, all mana must be the same color
/// (e.g., for "add three mana of any one color", the player must choose
/// all red, all blue, etc.).
///
/// # Fields
///
/// * `amount` - Number of mana to add
/// * `player` - Which player receives the mana
///
/// # Example
///
/// ```ignore
/// // Add 3 mana of any one color (must all be same color)
/// let effect = AddManaOfAnyOneColorEffect::you(3);
/// ```
impl EffectExecutor for AddManaOfAnyOneColorEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::ChooseColors {
            amount: &self.amount,
            available: &crate::color::Color::ALL,
            same_color: true,
            distinct: false,
            player: &self.player,
        })
    }

    fn directly_produces_mana(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        use super::production_resolution::{execute_production, ManaOutputProjection, ManaChoiceMenu};
        execute_production(self.mana_production().expect("mana production descriptor"),
            game, ctx, ManaOutputProjection::AuthoredCount, ManaChoiceMenu::AnyColor)
    }

    fn producible_mana_symbols(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        Some(vec![
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Value;
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_add_mana_of_any_one_color_default() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // new_default uses SelectFirstDecisionMaker, so unrestricted color
        // choices take the first color in WUBRG order.
        let effect = AddManaOfAnyOneColorEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        assert_eq!(game.player(alice).unwrap().mana_pool.white, 3);
    }

    #[test]
    fn test_add_mana_of_any_one_color_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyOneColorEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_add_mana_of_any_one_color_single() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyOneColorEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.white, 1);
    }

    #[test]
    fn test_add_mana_of_any_one_color_variable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(5);

        let effect = AddManaOfAnyOneColorEffect::you(Value::X);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));
        assert_eq!(game.player(alice).unwrap().mana_pool.white, 5);
    }

    #[test]
    fn test_add_mana_of_any_one_color_to_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyOneColorEffect::new(2, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.white, 2);
    }

    #[test]
    fn test_add_mana_of_any_one_color_clone_box() {
        let effect = AddManaOfAnyOneColorEffect::you(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("AddManaOfAnyOneColorEffect"));
    }
}
