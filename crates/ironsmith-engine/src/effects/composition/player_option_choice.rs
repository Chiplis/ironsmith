//! Per-player named choices ("Each opponent chooses fame or fortune",
//! "For each player, choose friend or foe"). See
//! [`ironsmith_core::ChoosePlayerOptionEffect`].

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter_to_list;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub use ironsmith_core::{ChoosePlayerOptionEffect, PlayerOptionChooser, player_option_choice_tag};

use super::player_order::apnap_position;

impl EffectExecutor for ChoosePlayerOptionEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::compound::execute_transaction(
            game,
            ctx,
            || EffectOutcome::count(0),
            |game, ctx| {
                let filter_ctx = ctx.filter_context(game);
                let mut participants =
                    resolve_player_filter_to_list(game, &self.participants, &filter_ctx, ctx)?;
                participants.retain(|player| {
                    game.player(*player)
                        .is_some_and(|player| player.is_in_game())
                });
                participants.sort_by_key(|player| apnap_position(game, *player));
                participants.dedup();

                let options = self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, name)| (i, name.clone()))
                    .collect::<Vec<_>>();
                let Some(choices) = super::scoped_choice::collect_choices_before_commit(
                    game,
                    ctx,
                    participants,
                    |game, ctx, participant| {
                        let chooser = match self.chooser {
                            PlayerOptionChooser::Participant => participant,
                            PlayerOptionChooser::Controller => ctx.controller,
                        };
                        let description = match self.chooser {
                            PlayerOptionChooser::Participant => "Choose one".to_string(),
                            PlayerOptionChooser::Controller => {
                                let name = game
                                    .player(participant)
                                    .map(|player| player.name.to_string())
                                    .unwrap_or_else(|| "that player".to_string());
                                format!("Choose one for {name}")
                            }
                        };
                        let index = super::scoped_choice::select_typed_option(
                            game,
                            ctx,
                            chooser,
                            &description,
                            &options,
                            super::scoped_choice::SingleChoicePolicy::FirstOrDefault,
                        )?;
                        Ok((participant, index))
                    },
                )?
                else {
                    return Ok(EffectOutcome::count(0));
                };
                let mut chosen_by_option: Vec<Vec<PlayerId>> = vec![Vec::new(); self.options.len()];
                for (participant, index) in choices {
                    if let Some(players) = index.and_then(|i| chosen_by_option.get_mut(i)) {
                        players.push(participant);
                    }
                }

                let mut total = 0usize;
                // Every option's set is recorded, empty ones included, so a
                // later "each foe" with no foes iterates nobody.
                for (option, players) in self.options.iter().zip(chosen_by_option) {
                    total += players.len();
                    ctx.set_tagged_players(player_option_choice_tag(option), players);
                }
                Ok(EffectOutcome::count(total as i32))
            },
        )
    }
}

pub use ironsmith_core::ControlVotesThisTurnEffect;

impl EffectExecutor for ControlVotesThisTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller = ctx.controller;
        super::execute_world_checkpoint_transaction(game, |game| {
            game.add_vote_control_this_turn(controller);
            Ok(EffectOutcome::resolved())
        })
    }
}
