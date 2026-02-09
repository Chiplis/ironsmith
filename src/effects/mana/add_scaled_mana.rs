//! Add scaled mana effect implementation.
//!
//! Adds a fixed mana pattern repeated by a resolved numeric value.
//! Example: "Add {B} for each creature card in your graveyard."

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::target::PlayerFilter;

/// Effect that adds a mana pattern repeated by `amount`.
#[derive(Debug, Clone, PartialEq)]
pub struct AddScaledManaEffect {
    /// The base mana pattern to repeat (e.g., [`ManaSymbol::Black`]).
    pub mana: Vec<ManaSymbol>,
    /// Number of repetitions to add.
    pub amount: Value,
    /// Which player receives the mana.
    pub player: PlayerFilter,
}

impl AddScaledManaEffect {
    /// Create a new scaled add-mana effect.
    pub fn new(mana: Vec<ManaSymbol>, amount: Value, player: PlayerFilter) -> Self {
        Self {
            mana,
            amount,
            player,
        }
    }
}

impl EffectExecutor for AddScaledManaEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let repeats = resolve_value(game, &self.amount, ctx)?.max(0) as usize;

        let mut added = Vec::new();
        if let Some(player) = game.player_mut(player_id) {
            for _ in 0..repeats {
                for symbol in &self.mana {
                    player.mana_pool.add(*symbol, 1);
                    added.push(*symbol);
                }
            }
        }

        Ok(EffectOutcome::from_result(EffectResult::ManaAdded(added)))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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
            result.result,
            EffectResult::ManaAdded(vec![
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
            result.result,
            EffectResult::ManaAdded(vec![ManaSymbol::Black, ManaSymbol::Black])
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
            result.result,
            EffectResult::ManaAdded(vec![
                ManaSymbol::Green,
                ManaSymbol::Green,
                ManaSymbol::Green
            ])
        );
        assert_eq!(game.player(alice).expect("alice").mana_pool.green, 3);
    }
}
