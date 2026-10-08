//! Per-player named choices ("Each opponent chooses fame or fortune",
//! "For each player, choose friend or foe"). See
//! [`ironsmith_core::ChoosePlayerOptionEffect`].

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter_to_list;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub use ironsmith_core::{ChoosePlayerOptionEffect, PlayerOptionChooser, player_option_choice_tag};

/// Position of `player` in turn order starting with the active player
/// (CR 101.4: choices are made in APNAP order).
fn apnap_position(game: &GameState, player: PlayerId) -> usize {
    let order = &game.turn_store.turn_order;
    let Some(active) = order
        .iter()
        .position(|candidate| *candidate == game.turn.active_player)
    else {
        return player.0 as usize;
    };
    order
        .iter()
        .position(|candidate| *candidate == player)
        .map_or(usize::MAX, |index| {
            (index + order.len() - active) % order.len()
        })
}

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

                let display_options = self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| SelectableOption::new(index, option.clone()))
                    .collect::<Vec<_>>();
                let mut chosen_by_option: Vec<Vec<PlayerId>> =
                    vec![Vec::new(); self.options.len()];
                for participant in participants {
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
                    let choice_ctx = SelectOptionsContext::new(
                        chooser,
                        Some(ctx.source),
                        description,
                        display_options.clone(),
                        1,
                        1,
                    );
                    let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    // A missing answer keeps the first option, as a forced
                    // single choice would.
                    let index = selected
                        .into_iter()
                        .next()
                        .filter(|index| *index < self.options.len())
                        .unwrap_or(0);
                    if let Some(players) = chosen_by_option.get_mut(index) {
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
