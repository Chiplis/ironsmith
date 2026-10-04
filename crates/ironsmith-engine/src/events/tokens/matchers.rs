//! Token creation replacement effect matchers.

use crate::events::cause::{CauseFilter, CauseFilterRuntimeExt as _};
use crate::events::context::EventContext;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, downcast_event};
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt as _;
use crate::target::{ObjectFilter, PlayerFilter};

use super::CreateTokensEvent;

/// Matches token creation under a player matching the configured filter.
#[derive(Debug, Clone)]
pub struct WouldCreateTokensUnderControlMatcher {
    pub controller_filter: PlayerFilter,
    pub cause_filter: CauseFilter,
    pub token_filter: Option<ObjectFilter>,
    pub condition: Option<crate::ConditionExpr>,
}

impl WouldCreateTokensUnderControlMatcher {
    pub fn new(controller_filter: PlayerFilter) -> Self {
        Self {
            controller_filter,
            cause_filter: CauseFilter::effect_like(),
            token_filter: None,
            condition: None,
        }
    }

    pub fn with_cause_filter(mut self, cause_filter: CauseFilter) -> Self {
        self.cause_filter = cause_filter;
        self
    }

    pub fn with_condition(mut self, condition: Option<crate::ConditionExpr>) -> Self {
        self.condition = condition; self
    }

    pub fn with_token_filter(mut self, token_filter: ObjectFilter) -> Self {
        self.token_filter = Some(token_filter);
        self
    }
}

impl ReplacementMatcher for WouldCreateTokensUnderControlMatcher {

    fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &crate::events::context::PreparedEventContext) -> bool {
        if event.event_kind() != EventKind::CreateTokens {
            return false;
        }

        let Some(create_tokens) = downcast_event::<CreateTokensEvent>(event) else {
            return false;
        };

        if create_tokens.total_count() == 0
            || !self
                .controller_filter
                .matches_player(create_tokens.controller, &ctx.filter_ctx)
            || !self.cause_filter.matches(
                &create_tokens.cause,
                ctx.game,
                create_tokens.affected_player(ctx.game),
            )
        {
            return false;
        }

        if let Some(condition) = &self.condition {
            let Some(source) = ctx.source else { return false; };
            let context = crate::condition_eval::ExternalEvaluationContext {
                controller: ctx.controller, source, defending_player: None, attacking_player: None,
                filter_source: Some(source), iterated_player: None, triggering_event: None,
                trigger_identity: None, ability_index: None, options: Default::default(),
            };
            if !crate::condition_eval::evaluate_condition_external(ctx.game, condition, &context) { return false; }
        }

        if let Some(token_filter) = &self.token_filter {
            // Tokens an earlier replacement added are part of the event too
            // (CR 616.1), so any matching group makes this apply.
            return create_tokens.group_keys().into_iter().any(|key| {
                create_tokens.group_count(key) > 0 && create_tokens.group_object(key)
                    .is_some_and(|token| token_filter.matches(&token, &ctx.filter_ctx, ctx.game))
            });
        }

        true
    }

    fn token_group_filter(&self) -> Option<&ObjectFilter> {
        self.token_filter.as_ref()
    }

    fn display(&self) -> String {
        "When an effect would create tokens under a matching player's control".to_string()
    }
}
