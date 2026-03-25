use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_player_filter, resolve_player_filter_to_list};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::{FilterContext, PlayerFilter};

#[derive(Debug, Clone, PartialEq)]
pub struct ChoosePlayerEffect {
    pub chooser: PlayerFilter,
    pub filter: PlayerFilter,
    pub tag: crate::tag::TagKey,
    pub random: bool,
    pub excluded_tags: Vec<crate::tag::TagKey>,
    pub remember_as_chosen_player: bool,
}

impl ChoosePlayerEffect {
    pub fn new(
        chooser: PlayerFilter,
        filter: PlayerFilter,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            chooser,
            filter,
            tag: tag.into(),
            random: false,
            excluded_tags: Vec::new(),
            remember_as_chosen_player: false,
        }
    }

    pub fn at_random(mut self) -> Self {
        self.random = true;
        self
    }

    pub fn excluding_tags(mut self, excluded_tags: Vec<crate::tag::TagKey>) -> Self {
        self.excluded_tags = excluded_tags;
        self
    }

    pub fn remember_as_chosen_player(mut self) -> Self {
        self.remember_as_chosen_player = true;
        self
    }

    fn candidate_players(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        let mut filter_ctx: FilterContext = ctx.filter_context(game);
        for excluded_tag in &self.excluded_tags {
            if let Some(players) = ctx.get_tagged_players(excluded_tag.as_str()) {
                let filtered = filter_ctx
                    .tagged_players
                    .remove(excluded_tag)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|player| !players.contains(player))
                    .collect::<Vec<_>>();
                if !filtered.is_empty() {
                    filter_ctx
                        .tagged_players
                        .insert(excluded_tag.clone(), filtered);
                }
            }
        }
        let mut players = resolve_player_filter_to_list(game, &self.filter, &filter_ctx, ctx)?;
        for excluded_tag in &self.excluded_tags {
            if let Some(excluded_players) = ctx.get_tagged_players(excluded_tag.as_str()) {
                players.retain(|player| !excluded_players.contains(player));
            }
        }
        players.sort_by_key(|player| player.0);
        players.dedup();
        Ok(players)
    }
}

impl EffectExecutor for ChoosePlayerEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let candidates = self.candidate_players(game, ctx)?;
        let Some(chosen) = (if self.random {
            let mut shuffled = candidates.clone();
            game.shuffle_slice(&mut shuffled);
            shuffled.first().copied()
        } else {
            let options = candidates
                .iter()
                .filter_map(|player_id| {
                    game.player(*player_id)
                        .map(|player| (player.name.clone(), *player_id))
                })
                .collect::<Vec<_>>();
            (!options.is_empty())
                .then(|| {
                    crate::decisions::ask_choose_one(
                        game,
                        &mut ctx.decision_maker,
                        chooser,
                        ctx.source,
                        &options,
                    )
                })
                .flatten()
        }) else {
            return Ok(EffectOutcome::resolved());
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        ctx.set_tagged_players(self.tag.clone(), vec![chosen]);
        if self.remember_as_chosen_player {
            game.set_chosen_player(ctx.source, chosen);
            ctx.chosen_player = Some(chosen);
        }
        if self.tag.as_str() != "__it__" {
            // Mirror the most recent chosen player onto the conventional follow-up
            // tag so clauses like "that player ..." resolve against the new choice.
            ctx.set_tagged_players(crate::tag::TagKey::from("__it__"), vec![chosen]);
        }
        Ok(EffectOutcome::count(1))
    }

    fn cost_description(&self) -> Option<String> {
        match self.filter {
            PlayerFilter::Opponent => Some("Choose an opponent".to_string()),
            PlayerFilter::Any => Some("Choose a player".to_string()),
            _ => None,
        }
    }
}

impl CostExecutableEffect for ChoosePlayerEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        let candidates = self.candidate_players(game, &ctx).map_err(|err| {
            CostValidationError::Other(format!("unable to choose player: {err:?}"))
        })?;
        if candidates.is_empty() {
            Err(CostValidationError::Other(
                "no legal player choices available".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}
