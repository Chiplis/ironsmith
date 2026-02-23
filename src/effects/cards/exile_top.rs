//! Exile top cards of library effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that exiles cards from the top of a player's library.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileTopOfLibraryEffect {
    /// How many cards to exile.
    pub count: Value,
    /// Which player's library to exile from.
    pub player: PlayerFilter,
}

impl ExileTopOfLibraryEffect {
    /// Create a new exile-top effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }
}

impl EffectExecutor for ExileTopOfLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        let top_cards = game
            .player(player_id)
            .map(|p| {
                let lib_len = p.library.len();
                let exile_count = count.min(lib_len);
                p.library[lib_len.saturating_sub(exile_count)..].to_vec()
            })
            .unwrap_or_default();

        let mut moved = 0i32;
        for card_id in top_cards {
            if game.move_object(card_id, Zone::Exile).is_some() {
                moved += 1;
            }
        }

        Ok(EffectOutcome::count(moved))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::EffectResult;
    use crate::events::DamageEvent;
    use crate::game_event::DamageTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::triggers::TriggerEvent;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn add_cards_to_library(game: &mut GameState, player: PlayerId, count: usize) {
        for i in 1..=count {
            let card = CardBuilder::new(CardId::from_raw(i as u32), &format!("Card {i}"))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&card, player, Zone::Library);
        }
    }

    #[test]
    fn exiles_top_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileTopOfLibraryEffect::new(3, PlayerFilter::You);

        let result = effect.execute(&mut game, &mut ctx).expect("execute");
        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).expect("alice").library.len(), 2);
        assert_eq!(game.exile.len(), 3);
    }

    #[test]
    fn exiles_from_damaged_player_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        add_cards_to_library(&mut game, bob, 2);

        let source = game.new_object_id();
        let damage_event =
            TriggerEvent::new(DamageEvent::new(source, DamageTarget::Player(bob), 2, true));
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_triggering_event(damage_event);

        let effect = ExileTopOfLibraryEffect::new(1, PlayerFilter::DamagedPlayer);
        let result = effect.execute(&mut game, &mut ctx).expect("execute");
        assert_eq!(result.result, EffectResult::Count(1));
        assert_eq!(game.player(bob).expect("bob").library.len(), 1);
        assert_eq!(game.exile.len(), 1);
    }
}
