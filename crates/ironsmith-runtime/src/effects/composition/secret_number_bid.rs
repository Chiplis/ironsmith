//! Secret numeric bid effects.

use crate::decision::DecisionMaker as _;
use crate::decisions::context::NumberContext;
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::{ChooseSpec, PlayerFilter};

pub type SecretNumberBidLifeLossEffect = ironsmith_core::SecretNumberBidLifeLossEffect;

fn active_players_in_turn_order(game: &GameState, start: PlayerId) -> Vec<PlayerId> {
    let mut players: Vec<PlayerId> = game
        .turn_store
        .turn_order
        .iter()
        .copied()
        .filter(|&player_id| {
            game.player(player_id)
                .is_some_and(|player| player.is_in_game())
        })
        .collect();

    if !players.contains(&start) {
        players = game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect();
    }

    if let Some(start_pos) = players.iter().position(|&player_id| player_id == start) {
        players.rotate_left(start_pos);
    }
    players
}

impl crate::effects::EffectExecutor for SecretNumberBidLifeLossEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let minimum = self.minimum.max(1);
        let mut choices = Vec::new();
        for player_id in active_players_in_turn_order(game, ctx.controller) {
            let decision = NumberContext::new(
                player_id,
                Some(ctx.source),
                minimum,
                u32::MAX,
                format!("Choose at least {minimum} hidden item count"),
            );
            let chosen = ctx.decision_maker.decide_number(game, &decision).max(minimum);
            choices.push((player_id, chosen));
            if ctx.decision_maker.awaiting_choice() {
                break;
            }
        }

        let mut outcomes = Vec::new();
        for (player_id, chosen) in &choices {
            outcomes.push(execute_effect(
                game,
                &Effect::new(crate::effects::LoseLifeEffect::new(
                    *chosen,
                    ChooseSpec::SpecificPlayer(*player_id),
                )),
                ctx,
            )?);
        }

        if let Some(fewest) = choices.iter().map(|(_, chosen)| *chosen).min() {
            for (player_id, _) in choices.iter().filter(|(_, chosen)| *chosen == fewest) {
                outcomes.push(execute_effect(
                    game,
                    &Effect::new(crate::effects::LoseLifeEffect::new(
                        Value::HalfLifeTotalRoundedUp(PlayerFilter::Specific(*player_id)),
                        ChooseSpec::SpecificPlayer(*player_id),
                    )),
                    ctx,
                )?);
            }
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::effects::EffectExecutor;

    struct ScriptedCounts {
        choices: std::collections::VecDeque<(PlayerId, u32)>,
    }

    impl ScriptedCounts {
        fn new(choices: Vec<(PlayerId, u32)>) -> Self {
            Self {
                choices: choices.into(),
            }
        }
    }

    impl DecisionMaker for ScriptedCounts {
        fn decide_number(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            let (expected_player, choice) = self
                .choices
                .pop_front()
                .expect("expected another secret-number prompt");
            assert_eq!(ctx.player, expected_player);
            choice
        }
    }

    #[test]
    fn secret_number_bid_life_loss_applies_item_loss_then_half_to_lowest() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut decisions = ScriptedCounts::new(vec![(alice, 3), (bob, 5)]);
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_decision_maker(&mut decisions);

        SecretNumberBidLifeLossEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("secret-number effect should resolve");

        assert_eq!(game.player(alice).expect("Alice should exist").life, 8);
        assert_eq!(game.player(bob).expect("Bob should exist").life, 15);
    }
}
