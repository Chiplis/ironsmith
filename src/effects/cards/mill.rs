//! Mill effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::zones::apply_zone_change;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::event_processor::EventOutcome;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that mills cards from a player's library to their graveyard.
///
/// # Fields
///
/// * `count` - How many cards to mill (can be fixed or variable)
/// * `player` - Which player mills
///
/// # Example
///
/// ```ignore
/// // Mill 3 cards
/// let effect = MillEffect::new(3, PlayerFilter::You);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MillEffect {
    /// How many cards to mill.
    pub count: Value,
    /// Which player mills.
    pub player: PlayerFilter,
}

impl MillEffect {
    /// Create a new mill effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you mill cards.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for MillEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn cost_description(&self) -> Option<String> {
        if self.player == PlayerFilter::You
            && let Value::Fixed(count) = self.count
        {
            return Some(if count == 1 {
                "Mill a card".to_string()
            } else {
                format!("Mill {} cards", count)
            });
        }
        None
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        // Snapshot the top cards first so replacement/prevention on one card does not
        // change which original cards are being milled.
        let cards_to_mill: Vec<ObjectId> = game
            .player(player_id)
            .map(|p| {
                p.library
                    .iter()
                    .rev()
                    .take(count)
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut milled = Vec::new();
        let mut any_prevented = false;

        for card_id in cards_to_mill {
            let Some(from_zone) = game.object(card_id).map(|obj| obj.zone) else {
                continue;
            };

            match apply_zone_change(
                game,
                card_id,
                from_zone,
                Zone::Graveyard,
                &mut *ctx.decision_maker,
            ) {
                EventOutcome::Proceed(change) => {
                    if change.final_zone == Zone::Graveyard
                        && let Some(new_id) = change.new_object_id
                    {
                        milled.push(new_id);
                    }
                }
                EventOutcome::Prevented => {
                    any_prevented = true;
                }
                EventOutcome::Replaced | EventOutcome::NotApplicable => {}
            }
        }

        if !milled.is_empty() {
            return Ok(EffectOutcome::from_result(EffectResult::Objects(milled)));
        }
        if any_prevented {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }
        Ok(EffectOutcome::count(0))
    }
}

impl CostExecutableEffect for MillEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        let player_id = match self.player {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(id) => id,
            _ => controller,
        };
        let count = match &self.count {
            Value::Fixed(count) => (*count).max(0) as usize,
            Value::X => {
                return Err(crate::effects::CostValidationError::Other(
                    "dynamic X mill costs are not supported".to_string(),
                ));
            }
            _ => {
                let ctx = crate::executor::ExecutionContext::new_default(source, controller);
                crate::effects::helpers::resolve_value(game, &self.count, &ctx)
                    .map_err(|err| crate::effects::CostValidationError::Other(format!("{err:?}")))?
                    .max(0) as usize
            }
        };
        let available = game.player(player_id).map_or(0, |p| p.library.len());
        if available >= count {
            Ok(())
        } else {
            Err(crate::effects::CostValidationError::Other(
                "not enough cards in library to pay mill cost".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::Effect;
    use crate::executor::execute_effect;
    use crate::ids::{CardId, PlayerId};
    use crate::tag::TagKey;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_cards_to_library(game: &mut GameState, owner: PlayerId, count: usize) -> Vec<ObjectId> {
        (0..count)
            .map(|idx| {
                let card = CardBuilder::new(
                    CardId::from_raw(20_000 + idx as u32),
                    &format!("Library Card {idx}"),
                )
                .card_types(vec![CardType::Instant])
                .build();
                game.create_object_from_card(&card, owner, Zone::Library)
            })
            .collect()
    }

    #[test]
    fn mill_moves_cards_through_zone_change_and_returns_graveyard_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let original_ids = add_cards_to_library(&mut game, alice, 3);
        let original_top_two = vec![original_ids[2], original_ids[1]];
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MillEffect::you(2);
        let outcome = effect.execute(&mut game, &mut ctx).expect("execute mill");
        let EffectResult::Objects(milled_ids) = outcome.result else {
            panic!("expected mill to return moved graveyard objects");
        };

        assert_eq!(milled_ids.len(), 2);
        assert_eq!(game.player(alice).expect("alice").library.len(), 1);
        assert_eq!(game.player(alice).expect("alice").graveyard, milled_ids);
        for original_id in original_top_two {
            assert!(
                game.object(original_id).is_none(),
                "original milled object should not remain after zone change"
            );
        }
    }

    #[test]
    fn tagged_mill_tags_post_move_graveyard_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_cards_to_library(&mut game, alice, 1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = Effect::mill(1).tag("milled");
        let outcome = execute_effect(&mut game, &effect, &mut ctx).expect("execute tagged mill");
        let EffectResult::Objects(milled_ids) = outcome.result else {
            panic!("expected tagged mill to return milled object ids");
        };

        let tagged = ctx
            .tagged_objects
            .get(&TagKey::from("milled"))
            .expect("milled tag should exist");
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].object_id, milled_ids[0]);
        assert_eq!(tagged[0].zone, Zone::Graveyard);
    }
}
