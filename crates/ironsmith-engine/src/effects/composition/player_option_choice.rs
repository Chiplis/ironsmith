//! Per-player named choices ("Each opponent chooses fame or fortune",
//! "For each player, choose friend or foe"). See
//! [`ironsmith_core::ChoosePlayerOptionEffect`].

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter_to_list;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub use ironsmith_core::{ChoosePlayerOptionEffect, PlayerOptionChooser, player_option_choice_tag};
use ironsmith_core::{
    PlayerNumberChoice, player_guess_tag, player_number_extreme_tag, player_option_participants_tag,
};

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
                ctx.set_tagged_players(player_option_participants_tag(), participants.clone());

                if self.choose_one_participant {
                    let options = participants
                        .iter()
                        .filter_map(|player_id| {
                            game.player(*player_id)
                                .map(|player| (player.name.to_string(), *player_id))
                        })
                        .collect::<Vec<_>>();
                    let chosen = if options.is_empty() {
                        None
                    } else {
                        crate::decisions::ask_choose_one(
                            game,
                            &mut ctx.decision_maker,
                            ctx.controller,
                            ctx.source,
                            &options,
                        )
                    };
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    participants = chosen.into_iter().collect();
                    ctx.set_tagged_players(player_option_participants_tag(), participants.clone());
                }

                if let Some(range) = self.number {
                    return choose_player_numbers(game, ctx, participants, range, self);
                }

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

/// Each participant chooses a number in `range`. Every number is collected
/// before any is recorded (a secret choice reveals them together); then the
/// players who chose the highest and the lowest number are recorded, and the
/// numbers are kept for [`crate::effect::Value::PlayersChosenNumberExtreme`].
fn choose_player_numbers(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: Vec<PlayerId>,
    range: PlayerNumberChoice,
    choice: &ChoosePlayerOptionEffect,
) -> Result<EffectOutcome, ExecutionError> {
    if range.max.is_some_and(|max| range.min > max) {
        return Err(ExecutionError::Impossible(
            "per-player number choice has an empty range".into(),
        ));
    }
    // "a number between 1 and 5 that hasn't been chosen": the numbers still
    // available, as a menu.
    let available = if choice.exclude_source_history {
        let Some(max) = range.max else {
            return Err(ExecutionError::Impossible(
                "an unbounded number choice can't exclude earlier choices".into(),
            ));
        };
        let chosen = game.numbers_chosen_by_source(ctx.source).to_vec();
        Some(
            (range.min..=max)
                .filter(|number| !chosen.contains(number))
                .map(|number| (number, number.to_string()))
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let description = if let Some(surface) = &choice.hidden_items_surface {
        format!("Hide at least one {}", surface.item_noun)
    } else {
        match (choice.guess, choice.secret) {
            (true, _) => "Guess the chosen number",
            (false, true) => "Secretly choose a number",
            (false, false) => "Choose a number",
        }
        .to_string()
    };
    let numbers = if available.as_ref().is_some_and(Vec::is_empty) {
        // Every number has been chosen: no one can choose ("If you do" is
        // false).
        Vec::new()
    } else {
        let Some(numbers) = super::scoped_choice::collect_choices_before_commit(
            game,
            ctx,
            participants,
            |game, ctx, participant| {
                if let Some(available) = &available {
                    let number = super::scoped_choice::select_typed_option(
                        game,
                        ctx,
                        participant,
                        description.as_str(),
                        available,
                        super::scoped_choice::SingleChoicePolicy::FirstOrDefault,
                    )?;
                    return Ok(number.map(|number| (participant, number)));
                }
                let legal_max = if choice.draw_count_choice {
                    if !game.can_draw(participant) {
                        0
                    } else if game.can_draw_extra_cards(participant) {
                        range.max.unwrap_or(u32::MAX)
                    } else if game
                        .turn_store
                        .turn_history
                        .has_drawn_cards_this_turn(participant)?
                    {
                        0
                    } else {
                        range.max.unwrap_or(u32::MAX).min(1)
                    }
                } else {
                    range.max.unwrap_or(u32::MAX)
                };
                if legal_max < range.min {
                    return Err(ExecutionError::Impossible(
                        "draw count choice has no legal value".into(),
                    ));
                }
                let mut prompt = crate::decisions::context::NumberContext::new(
                    participant,
                    Some(ctx.source),
                    range.min,
                    legal_max,
                    description.as_str(),
                );
                prompt.authored_max = range.max;
                let number = ctx.decision_maker.decide_number(game, &prompt);
                let number = number.max(range.min);
                let number = number.min(legal_max);
                Ok(Some((participant, number)))
            },
        )?
        else {
            return Ok(EffectOutcome::count(0));
        };
        numbers.into_iter().flatten().collect::<Vec<_>>()
    };
    if choice.exclude_source_history && game.object(ctx.source).is_some() {
        for (_, number) in &numbers {
            game.record_number_chosen_by_source(ctx.source, *number);
        }
    }
    let highest = numbers.iter().map(|(_, number)| *number).max();
    if choice.guess {
        if !numbers.is_empty() {
            // "spell or ability that caused one or more players to guess".
            super::emit_keyword_action::record_pile_or_guess_instruction(game, ctx);
        }
        let chosen = ctx
            .secret_choice_results
            .get(&ctx.source)
            .map(|result| {
                result
                    .number_choices
                    .iter()
                    .map(|(_, number)| *number)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let (right, wrong): (Vec<_>, Vec<_>) = numbers
            .iter()
            .partition(|(_, guess)| chosen.contains(guess));
        ctx.set_tagged_players(
            player_guess_tag(true),
            right.into_iter().map(|(player, _)| player).collect(),
        );
        ctx.set_tagged_players(
            player_guess_tag(false),
            wrong.into_iter().map(|(player, _)| player).collect(),
        );
        ctx.secret_choice_results.entry(ctx.source).or_default().guesses = numbers;
        return Ok(EffectOutcome::count(highest.map_or(0, i64::from)));
    }
    let lowest = numbers.iter().map(|(_, number)| *number).min();
    let with = |target: Option<u32>| {
        numbers
            .iter()
            .filter(|(_, number)| Some(*number) == target)
            .map(|(player, _)| *player)
            .collect::<Vec<_>>()
    };
    ctx.set_tagged_players(player_number_extreme_tag(true), with(highest));
    ctx.set_tagged_players(player_number_extreme_tag(false), with(lowest));
    let entry = ctx.secret_choice_results.entry(ctx.source).or_default();
    entry.number_choices = numbers;
    Ok(EffectOutcome::count(highest.map_or(0, i64::from)))
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
