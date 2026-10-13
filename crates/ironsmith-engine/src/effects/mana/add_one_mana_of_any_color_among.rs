//! Add one mana of a color found among objects matching a filter.

use super::add_mana_of_colors_among::{colors_among_filter};
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;

pub type AddOneManaOfAnyColorAmongEffect = ironsmith_core::AddOneManaOfAnyColorAmongEffect;

impl EffectExecutor for AddOneManaOfAnyColorAmongEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::ColorsAmong {
            filter: &self.filter,
            choose_one: true,
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
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let symbols = colors_among_filter(game, &self.filter, source, controller);
        (!symbols.is_empty()).then_some(symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::color::{Color, ColorSet};
    use crate::ids::CardId;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::{CardType, Supertype};
    use crate::zone::Zone;

    fn add_permanent(
        game: &mut GameState,
        card_id: u32,
        name: &str,
        controller: PlayerId,
        color: Color,
        legendary: bool,
    ) {
        let mut builder = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::from(color));
        if legendary {
            builder = builder.supertypes(vec![Supertype::Legendary]);
        }
        let card = builder.build();
        game.create_object_from_card(&card, controller, Zone::Battlefield);
    }

    #[test]
    fn chooses_only_from_colors_of_matching_permanents() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        add_permanent(&mut game, 91_001, "Red Legend", alice, Color::Red, true);
        add_permanent(
            &mut game,
            91_002,
            "Blue Commoner",
            alice,
            Color::Blue,
            false,
        );
        add_permanent(&mut game, 91_003, "White Rival", bob, Color::White, true);

        let filter = ObjectFilter::permanent()
            .with_supertype(Supertype::Legendary)
            .controlled_by(PlayerFilter::You);
        let effect = AddOneManaOfAnyColorAmongEffect::you(filter);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        let pool = &game.player(alice).unwrap().mana_pool;
        assert_eq!(pool.red, 1);
        assert_eq!(pool.white + pool.blue + pool.black + pool.green, 0);
    }
}
