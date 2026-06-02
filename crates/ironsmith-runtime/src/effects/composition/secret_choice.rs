use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter_to_list;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::{ChooseSpec, PlayerFilter};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecretChoiceResult {
    pub choices: Vec<(PlayerId, String)>,
}

impl SecretChoiceResult {
    pub fn choices_match(&self) -> bool {
        let Some((_, first)) = self.choices.first() else {
            return false;
        };
        self.choices.len() > 1
            && self
                .choices
                .iter()
                .all(|(_, choice)| choice.eq_ignore_ascii_case(first))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecretChoiceEffect {
    pub options: Vec<String>,
    pub participants: Vec<PlayerFilter>,
    pub participant_target: Option<ChooseSpec>,
}

impl SecretChoiceEffect {
    pub fn new(options: Vec<String>, participants: Vec<PlayerFilter>) -> Self {
        let participant_target = participants.iter().find_map(|participant| {
            if let PlayerFilter::Target(inner) = participant {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            } else {
                None
            }
        });
        Self {
            options,
            participants,
            participant_target,
        }
    }

    fn participating_players(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);
        let mut players = Vec::new();
        for participant in &self.participants {
            for player in resolve_player_filter_to_list(game, participant, &filter_ctx, ctx)? {
                if game.player(player).is_some_and(|player| player.is_in_game())
                    && !players.contains(&player)
                {
                    players.push(player);
                }
            }
        }
        Ok(players)
    }
}

impl EffectExecutor for SecretChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }
        let players = self.participating_players(game, ctx)?;
        let display_options = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, option)| SelectableOption::new(idx, option.clone()))
            .collect::<Vec<_>>();
        let mut choices = Vec::new();
        for player in players {
            let choice_ctx = SelectOptionsContext::new(
                player,
                Some(ctx.source),
                "Secretly choose one",
                display_options.clone(),
                1,
                1,
            );
            let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if let Some(chosen) = selected.into_iter().next().filter(|idx| *idx < self.options.len())
            {
                choices.push((player, self.options[chosen].clone()));
            }
        }
        let count = choices.len();
        ctx.secret_choice_results
            .insert(ctx.source, SecretChoiceResult { choices });
        Ok(EffectOutcome::count(count as i32))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.participant_target.as_ref()
    }
}
