//! Scry effect implementation.

use crate::decisions::{ScrySpec, make_decision};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

/// Effect that lets a player scry N cards.
///
/// Per Rule 701.18, look at the top N cards, then put any number on the bottom
/// of the library in any order and the rest on top in any order.
///
/// # Fields
///
/// * `count` - Number of cards to scry
/// * `player` - The player who scries
///
/// # Example
///
/// ```ignore
/// // Scry 2
/// let effect = ScryEffect::new(2, PlayerFilter::You);
///
/// // Scry 1
/// let effect = ScryEffect::you(1);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ScryEffect {
    /// Number of cards to scry.
    pub count: Value,
    /// The player who scries.
    pub player: PlayerFilter,
}

impl ScryEffect {
    /// Create a new scry effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// The controller scries N.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for ScryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        if count == 0 {
            return Ok(EffectOutcome::count(0));
        }

        // Get the top N cards (they're at the end of the library vec)
        let top_cards: Vec<ObjectId> = game
            .player(player_id)
            .map(|p| {
                let lib_len = p.library.len();
                let scry_count = count.min(lib_len);
                p.library[lib_len.saturating_sub(scry_count)..].to_vec()
            })
            .unwrap_or_default();

        if top_cards.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let scry_count = top_cards.len();

        // Ask player which cards to put on bottom using the new spec-based system
        let spec = ScrySpec::new(ctx.source, top_cards.clone());
        let cards_to_bottom: Vec<ObjectId> = make_decision(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            spec,
        )
        .into_iter()
        .filter(|c| top_cards.contains(c))
        .collect();

        // Remove the scried cards from library temporarily
        if let Some(p) = game.player_mut(player_id) {
            let lib_len = p.library.len();
            p.library.truncate(lib_len.saturating_sub(scry_count));
        }

        // Put cards going to bottom first (they go under the remaining library)
        // Then put the rest back on top
        let cards_to_top: Vec<ObjectId> = top_cards
            .iter()
            .filter(|c| !cards_to_bottom.contains(c))
            .copied()
            .collect();

        if let Some(p) = game.player_mut(player_id) {
            // Insert bottom cards at the beginning of library
            for &card_id in cards_to_bottom.iter().rev() {
                p.library.insert(0, card_id);
            }
            // Add top cards back to end (top of library)
            p.library.extend(cards_to_top);
        }

        Ok(
            EffectOutcome::count(scry_count as i32).with_event(TriggerEvent::new(
                KeywordActionEvent::new(
                    KeywordActionKind::Scry,
                    player_id,
                    ctx.source,
                    scry_count as u32,
                ),
            )),
        )
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn add_cards_to_library(game: &mut GameState, owner: PlayerId, count: usize) {
        for _ in 0..count {
            let id = game.new_object_id();
            if let Some(player) = game.player_mut(owner) {
                player.library.push(id);
            }
        }
    }

    #[test]
    fn test_scry() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_cards_to_library(&mut game, alice, 5);

        let lib_size_before = game.player(alice).unwrap().library.len();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ScryEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        // Library size should be unchanged (cards just reordered)
        assert_eq!(game.player(alice).unwrap().library.len(), lib_size_before);
    }

    #[test]
    fn test_scry_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ScryEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_scry_more_than_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_cards_to_library(&mut game, alice, 2);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ScryEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only scried 2 cards (all in library)
        assert_eq!(result.result, EffectResult::Count(2));
    }

    #[test]
    fn test_scry_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_cards_to_library(&mut game, alice, 5);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ScryEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_scry_variable_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_cards_to_library(&mut game, alice, 5);

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(3);
        let effect = ScryEffect::new(Value::X, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
    }

    #[test]
    fn test_scry_clone_box() {
        let effect = ScryEffect::you(2);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ScryEffect"));
    }
}
