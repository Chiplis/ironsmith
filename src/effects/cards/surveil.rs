//! Surveil effect implementation.

use crate::decisions::{SurveilSpec, make_decision};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Effect that lets a player surveil N cards.
///
/// Per Rule 701.42, look at the top N cards, then put any number into your
/// graveyard and the rest on top of your library in any order.
///
/// # Fields
///
/// * `count` - Number of cards to surveil
/// * `player` - The player who surveils
///
/// # Example
///
/// ```ignore
/// // Surveil 2
/// let effect = SurveilEffect::new(2, PlayerFilter::You);
///
/// // Surveil 1
/// let effect = SurveilEffect::you(1);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SurveilEffect {
    /// Number of cards to surveil.
    pub count: Value,
    /// The player who surveils.
    pub player: PlayerFilter,
}

impl SurveilEffect {
    /// Create a new surveil effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// The controller surveils N.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for SurveilEffect {
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
                let surveil_count = count.min(lib_len);
                p.library[lib_len.saturating_sub(surveil_count)..].to_vec()
            })
            .unwrap_or_default();

        if top_cards.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let surveil_count = top_cards.len();

        // Ask player which cards to put in graveyard using the new spec-based system
        let spec = SurveilSpec::new(ctx.source, top_cards.clone());
        let cards_to_graveyard: Vec<ObjectId> = make_decision(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            spec,
        )
        .into_iter()
        .filter(|c| top_cards.contains(c))
        .collect();

        // Remove the surveilled cards from library temporarily
        if let Some(p) = game.player_mut(player_id) {
            let lib_len = p.library.len();
            p.library.truncate(lib_len.saturating_sub(surveil_count));
        }

        // Put cards going to graveyard
        for &card_id in &cards_to_graveyard {
            game.move_object(card_id, Zone::Graveyard);
        }

        // Put the rest back on top
        let cards_to_top: Vec<ObjectId> = top_cards
            .iter()
            .filter(|c| !cards_to_graveyard.contains(c))
            .copied()
            .collect();

        if let Some(p) = game.player_mut(player_id) {
            // Add top cards back to end (top of library)
            p.library.extend(cards_to_top);
        }

        Ok(
            EffectOutcome::count(surveil_count as i32).with_event(TriggerEvent::new(
                KeywordActionEvent::new(
                    KeywordActionKind::Surveil,
                    player_id,
                    ctx.source,
                    surveil_count as u32,
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
    fn test_surveil() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_cards_to_library(&mut game, alice, 5);

        let lib_size_before = game.player(alice).unwrap().library.len();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SurveilEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        // Without decision maker, all cards stay on top, so library unchanged
        assert_eq!(game.player(alice).unwrap().library.len(), lib_size_before);
    }

    #[test]
    fn test_surveil_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SurveilEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_surveil_more_than_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_cards_to_library(&mut game, alice, 2);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SurveilEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only surveilled 2 cards (all in library)
        assert_eq!(result.result, EffectResult::Count(2));
    }

    #[test]
    fn test_surveil_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_cards_to_library(&mut game, alice, 5);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SurveilEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_surveil_variable_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_cards_to_library(&mut game, alice, 5);

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(3);
        let effect = SurveilEffect::new(Value::X, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
    }

    #[test]
    fn test_surveil_clone_box() {
        let effect = SurveilEffect::you(2);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("SurveilEffect"));
    }
}
