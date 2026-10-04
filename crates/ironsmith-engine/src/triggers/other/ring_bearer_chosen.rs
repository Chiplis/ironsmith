use crate::events::{EventKind, KeywordActionEvent, KeywordActionKind};
use crate::filter::PlayerFilterExt as _;
use crate::target::PlayerFilter;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct RingBearerChosenTrigger {
    pub player: PlayerFilter,
}
impl TriggerMatcher for RingBearerChosenTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event.downcast::<KeywordActionEvent>().is_some_and(|event| {
            event.action == KeywordActionKind::RingTemptsYou
                && self.player.matches_player(event.player, &ctx.filter_ctx)
                && event
                    .object_tags
                    .get(ironsmith_core::tag::RING_BEARER_CHOSEN_TAG)
                    .is_some_and(|objects| objects.len() == 1)
        })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::KeywordAction])
    }
    fn display(&self) -> String {
        format!(
            "Whenever {} {} a creature as {} Ring-bearer",
            self.player.description(),
            if self.player == PlayerFilter::You {
                "choose"
            } else {
                "chooses"
            },
            if self.player == PlayerFilter::You {
                "your"
            } else {
                "their"
            }
        )
    }
}
