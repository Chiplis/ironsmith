//! Add mana of the chosen color effect implementation.

use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub type AddManaOfChosenColorEffect = ironsmith_core::AddManaOfChosenColorEffect;

impl EffectExecutor for AddManaOfChosenColorEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::ChosenColor {
            amount: &self.amount,
            fixed_option: self.fixed_option,
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
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let chosen = game.current_characteristics(source).and_then(|chars| {
            chars.abilities.iter().enumerate().find_map(|(slot, ability)| {
                let crate::ability::AbilityKind::Activated(ability) = &ability.kind else { return None; };
                ability.effects.all_effects().into_iter().any(|effect| effect.downcast_ref::<AddManaOfChosenColorEffect>() == Some(self))
                    .then(|| game.chosen_color_for_ability(source, chars.abilities.origin(slot))).flatten()
            })
        });
        let mut symbols = chosen
            .map(ManaSymbol::from_color)
            .into_iter()
            .collect::<Vec<_>>();
        if let Some(fixed) = self.fixed_option {
            let fixed_symbol = ManaSymbol::from_color(fixed);
            if !symbols.contains(&fixed_symbol) {
                symbols.push(fixed_symbol);
            }
        }
        Some(symbols)
    }
}
