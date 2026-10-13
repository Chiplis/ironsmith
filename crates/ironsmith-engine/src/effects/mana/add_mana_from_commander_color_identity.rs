//! Add mana from commander color identity effect implementation.

use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub use ironsmith_core::AddManaFromCommanderColorIdentityEffect;

/// Effect that adds mana of any color in the player's commander's color identity.
///
/// Used by cards like Arcane Signet and Command Tower. If the commander's color
/// identity is colorless (or there is no commander), adds colorless mana instead.
///
/// # Fields
///
/// * `amount` - Number of mana to add
/// * `player` - Which player receives the mana
///
/// # Example
///
/// ```ignore
/// // Arcane Signet: Tap to add one mana of any color in your commander's identity
/// let effect = AddManaFromCommanderColorIdentityEffect::you(1);
/// ```
impl EffectExecutor for AddManaFromCommanderColorIdentityEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::CommanderIdentity {
            amount: &self.amount,
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
            game, ctx, ManaOutputProjection::AuthoredCount, ManaChoiceMenu::AvailableColors)
    }

    fn producible_mana_symbols(
        &self,
        game: &GameState,
        _source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let identity = game.get_commander_color_identity(controller);
        if identity.is_empty() {
            return Some(vec![ManaSymbol::Colorless]);
        }

        let mut symbols = Vec::new();
        if identity.contains(Color::White) {
            symbols.push(ManaSymbol::White);
        }
        if identity.contains(Color::Blue) {
            symbols.push(ManaSymbol::Blue);
        }
        if identity.contains(Color::Black) {
            symbols.push(ManaSymbol::Black);
        }
        if identity.contains(Color::Red) {
            symbols.push(ManaSymbol::Red);
        }
        if identity.contains(Color::Green) {
            symbols.push(ManaSymbol::Green);
        }
        Some(symbols)
    }
}
