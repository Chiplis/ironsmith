//! Add scaled mana effect implementation.
//!
//! Adds a fixed mana pattern repeated by a resolved numeric value.
//! Example: "Add {B} for each creature card in your graveyard."

use super::choice_helpers::{credit_mana_symbols_from_context, mana_added_value_outputs};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub type AddScaledManaEffect = ironsmith_core::AddScaledManaEffect;

impl EffectExecutor for AddScaledManaEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::Repeated {
            symbols: &self.mana,
            amount: &self.amount,
            player: &self.player,
        })
    }

    fn directly_produces_mana(&self) -> bool {
        !self.mana.is_empty()
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
        let (player_id, symbols) = self
            .mana_production()
            .expect("mana production descriptor")
            .resolve_exact(game, ctx)?;
        let mana = credit_mana_symbols_from_context(game, player_id, symbols, ctx)?;
        Ok(mana_added_value_outputs(ctx, player_id, mana))
    }

    fn producible_mana_symbols(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        Some(self.mana.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::ids::PlayerId;
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn put_card_in_graveyard(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        card_types: Vec<CardType>,
    ) {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .build();
        let id = game.new_object_id();
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
    }

    #[test]
    fn test_add_scaled_mana_fixed_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            AddScaledManaEffect::new(vec![ManaSymbol::Black], Value::Fixed(3), PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).expect("execute");

        assert_eq!(
            result.value,
            crate::effect::OutcomeValue::ManaAdded(vec![
                ManaSymbol::Black,
                ManaSymbol::Black,
                ManaSymbol::Black
            ])
        );
        assert_eq!(game.player(alice).expect("alice").mana_pool.black, 3);
    }

    #[test]
    fn test_add_scaled_mana_counts_graveyard_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        put_card_in_graveyard(&mut game, alice, "Dead Bear", vec![CardType::Creature]);
        put_card_in_graveyard(&mut game, alice, "Dead Elf", vec![CardType::Creature]);
        put_card_in_graveyard(&mut game, alice, "Dead Ritual", vec![CardType::Sorcery]);

        let effect = AddScaledManaEffect::new(
            vec![ManaSymbol::Black],
            Value::Count(
                ObjectFilter::creature()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            ),
            PlayerFilter::You,
        );
        let result = effect.execute(&mut game, &mut ctx).expect("execute");

        assert_eq!(
            result.value,
            crate::effect::OutcomeValue::ManaAdded(vec![ManaSymbol::Black, ManaSymbol::Black])
        );
        assert_eq!(game.player(alice).expect("alice").mana_pool.black, 2);
    }

    #[test]
    fn test_add_scaled_mana_uses_devotion_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // GG contributes 2 devotion to green.
        let green_card = CardBuilder::new(CardId::new(), "GG Creature")
            .card_types(vec![CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![
                vec![ManaSymbol::Green],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let green_id = game.new_object_id();
        let green_obj = Object::from_card(green_id, &green_card, alice, Zone::Battlefield);
        game.add_object(green_obj);

        // Hybrid G/U contributes 1 devotion to green.
        let hybrid_card = CardBuilder::new(CardId::new(), "Hybrid Creature")
            .card_types(vec![CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                ManaSymbol::Green,
                ManaSymbol::Blue,
            ]]))
            .build();
        let hybrid_id = game.new_object_id();
        let hybrid_obj = Object::from_card(hybrid_id, &hybrid_card, alice, Zone::Battlefield);
        game.add_object(hybrid_obj);

        let effect = AddScaledManaEffect::new(
            vec![ManaSymbol::Green],
            Value::Devotion {
                player: PlayerFilter::You,
                color: crate::color::Color::Green,
            },
            PlayerFilter::You,
        );
        let result = effect.execute(&mut game, &mut ctx).expect("execute");

        assert_eq!(
            result.value,
            crate::effect::OutcomeValue::ManaAdded(vec![
                ManaSymbol::Green,
                ManaSymbol::Green,
                ManaSymbol::Green
            ])
        );
        assert_eq!(game.player(alice).expect("alice").mana_pool.green, 3);
    }
}
