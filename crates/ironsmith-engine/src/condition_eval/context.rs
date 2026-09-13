use super::*;

/// The three callers expose different facts. Keep their binding policies here
/// instead of maintaining a separate interpreter for each caller.
pub(super) struct ConditionContext<'a, 'event> {
    pub controller: PlayerId,
    pub source: ObjectId,
    mode: ContextMode<'a, 'event>,
}

enum ContextMode<'a, 'event> {
    CastTime,
    External(&'a ExternalEvaluationContext<'event>),
    Resolution(&'a ExecutionContext<'event>),
}

impl<'a, 'event> ConditionContext<'a, 'event> {
    pub fn cast_time(controller: PlayerId, source: ObjectId) -> Self {
        Self {
            controller,
            source,
            mode: ContextMode::CastTime,
        }
    }

    pub fn external_context(ctx: &'a ExternalEvaluationContext<'event>) -> Self {
        Self {
            controller: ctx.controller,
            source: ctx.source,
            mode: ContextMode::External(ctx),
        }
    }

    pub fn resolution(ctx: &'a ExecutionContext<'event>) -> Self {
        Self {
            controller: ctx.controller,
            source: ctx.source,
            mode: ContextMode::Resolution(ctx),
        }
    }

    pub fn is_cast_time(&self) -> bool {
        matches!(self.mode, ContextMode::CastTime)
    }

    pub fn execution(&self) -> Option<&'a ExecutionContext<'event>> {
        match self.mode {
            ContextMode::Resolution(ctx) => Some(ctx),
            _ => None,
        }
    }

    pub fn external(&self) -> Option<&'a ExternalEvaluationContext<'event>> {
        match self.mode {
            ContextMode::External(ctx) => Some(ctx),
            _ => None,
        }
    }

    pub fn shared(&self) -> SharedConditionContext<'_> {
        match self.mode {
            ContextMode::CastTime => SharedConditionContext {
                controller: self.controller,
                source: self.source,
                filter_source: Some(self.source),
                triggering_event: None,
                trigger_identity: None,
                ability_index: None,
            },
            ContextMode::External(ctx) => SharedConditionContext {
                controller: ctx.controller,
                source: ctx.source,
                filter_source: ctx.filter_source,
                triggering_event: ctx.triggering_event,
                trigger_identity: ctx.trigger_identity,
                ability_index: ctx.ability_index,
            },
            ContextMode::Resolution(ctx) => SharedConditionContext {
                controller: ctx.controller,
                source: ctx.source,
                filter_source: Some(ctx.source),
                triggering_event: ctx.triggering_event.as_ref(),
                trigger_identity: ctx.trigger_identity,
                ability_index: ctx.ability_index,
            },
        }
    }

    /// Missing bindings are false at cast time and during external gating, but
    /// resolution preserves the execution helper's errors.
    pub fn resolve_player(
        &self,
        game: &GameState,
        player: &PlayerFilter,
    ) -> Result<Option<PlayerId>, ExecutionError> {
        match self.mode {
            ContextMode::CastTime => Ok(resolve_condition_player_simple(
                game,
                self.controller,
                player,
            )),
            ContextMode::External(ctx) => Ok(resolve_condition_player_external(game, ctx, player)),
            ContextMode::Resolution(ctx) => {
                crate::effects::helpers::resolve_player_filter(game, player, ctx).map(Some)
            }
        }
    }

    pub fn matching_players(
        &self,
        game: &GameState,
        player: &PlayerFilter,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        match player {
            PlayerFilter::Opponent | PlayerFilter::NotYou | PlayerFilter::Any => Ok(
                matching_condition_players_simple(game, self.controller, player),
            ),
            _ => Ok(self.resolve_player(game, player)?.into_iter().collect()),
        }
    }

    pub fn filter_context(&self, game: &GameState) -> FilterContext {
        match self.mode {
            ContextMode::CastTime => condition_filter_context(
                game,
                self.controller,
                self.source,
                &PlayerFilter::You,
                None,
            ),
            ContextMode::External(ctx) => {
                game.filter_context_for(ctx.controller, ctx.filter_source)
            }
            ContextMode::Resolution(ctx) => ctx.filter_context(game),
        }
    }

    pub fn spell_history_filter_context(&self, game: &GameState) -> FilterContext {
        match self.mode {
            ContextMode::CastTime => game.filter_context_for(self.controller, Some(self.source)),
            _ => self.filter_context(game),
        }
    }

    /// Resolution retains the execution controller as "you" and binds the
    /// selected player as the iteration. Earlier phases rebase "you" onto
    /// the selected player; external checks additionally expose event facts.
    pub fn player_filter_context(
        &self,
        game: &GameState,
        player: &PlayerFilter,
        candidate: PlayerId,
    ) -> FilterContext {
        match self.mode {
            ContextMode::Resolution(ctx) => {
                let mut filter_ctx = ctx.filter_context(game);
                filter_ctx.iterated_player = Some(candidate);
                filter_ctx
            }
            _ => condition_filter_context(
                game,
                candidate,
                self.source,
                player,
                self.shared().triggering_event,
            ),
        }
    }

    /// Named ownership checks historically use no trigger snapshot outside
    /// resolution, unlike control/count predicates.
    pub fn owned_card_filter_context(
        &self,
        game: &GameState,
        player: &PlayerFilter,
        candidate: PlayerId,
    ) -> FilterContext {
        if self.execution().is_some() {
            self.player_filter_context(game, player, candidate)
        } else {
            condition_filter_context(game, candidate, self.source, player, None)
        }
    }
}
