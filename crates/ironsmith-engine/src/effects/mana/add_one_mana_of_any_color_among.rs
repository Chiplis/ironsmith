//! Add one mana of a color found among objects matching a filter.

use super::add_mana_of_colors_among::colors_among_filter;
use super::choice_helpers::{
    choose_mana_colors, credit_mana_symbols_from_context, mana_added_count_outcome,
};
use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;

pub type AddOneManaOfAnyColorAmongEffect = ironsmith_core::AddOneManaOfAnyColorAmongEffect;

impl EffectExecutor for AddOneManaOfAnyColorAmongEffect {
    fn directly_produces_mana(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let symbols = colors_among_filter(game, &self.filter, ctx.source, player_id);
        let colors = symbols
            .iter()
            .filter_map(|symbol| color_for_symbol(*symbol))
            .collect::<Vec<_>>();
        let Some(default_color) = colors.first().copied() else {
            return Ok(EffectOutcome::count(0));
        };

        let chosen = choose_mana_colors(
            game,
            ctx,
            player_id,
            1,
            false,
            false,
            Some(&colors),
            default_color,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(color) = chosen.into_iter().next() else {
            return Ok(EffectOutcome::count(0));
        };
        let symbols = credit_mana_symbols_from_context(
            game,
            player_id,
            vec![ManaSymbol::from_color(color)],
            ctx,
        );
        Ok(mana_added_count_outcome(ctx, player_id, symbols, 1))
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

fn color_for_symbol(symbol: ManaSymbol) -> Option<Color> {
    match symbol {
        ManaSymbol::White => Some(Color::White),
        ManaSymbol::Blue => Some(Color::Blue),
        ManaSymbol::Black => Some(Color::Black),
        ManaSymbol::Red => Some(Color::Red),
        ManaSymbol::Green => Some(Color::Green),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::color::ColorSet;
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
