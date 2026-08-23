//! "Whenever [spell] is countered" trigger.

use crate::events::EventKind;
use crate::events::SpellCounteredEvent;
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCounteredTrigger {
    pub filter: Option<ObjectFilter>,
    pub controller: PlayerFilter,
}

impl SpellCounteredTrigger {
    pub fn new(filter: Option<ObjectFilter>, controller: PlayerFilter) -> Self {
        Self { filter, controller }
    }
}

impl TriggerMatcher for SpellCounteredTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SpellCountered {
            return false;
        }
        let Some(countered) = event.downcast::<SpellCounteredEvent>() else {
            return false;
        };
        if !self
            .controller
            .matches_player(countered.controller, &ctx.filter_ctx)
        {
            return false;
        }
        let Some(filter) = &self.filter else {
            return true;
        };
        if let Some(snapshot) = countered.snapshot.as_ref().or_else(|| event.snapshot()) {
            return filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }
        ctx.game
            .object(countered.spell)
            .is_some_and(|object| filter.matches(object, &ctx.filter_ctx, ctx.game))
    }

    fn display(&self) -> String {
        match &self.controller {
            PlayerFilter::You => "Whenever a spell you've cast is countered".to_string(),
            PlayerFilter::Opponent => "Whenever a spell an opponent cast is countered".to_string(),
            player => format!(
                "Whenever a spell {} cast is countered",
                describe_player_filter_subject(player)
            ),
        }
    }

    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        event.kind() == EventKind::SpellCountered
    }
}
