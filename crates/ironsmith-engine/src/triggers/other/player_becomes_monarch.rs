use crate::events::{EventKind, MonarchChangedEvent};
use crate::target::PlayerFilter;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerBecomesMonarchTrigger {
    pub player: PlayerFilter,
}
impl TriggerMatcher for PlayerBecomesMonarchTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event
            .downcast::<MonarchChangedEvent>()
            .is_some_and(|change| {
                change.previous != Some(change.monarch)
                    && crate::filter::player_filter_matches_game(
                        &self.player,
                        change.monarch,
                        ctx.game,
                        &ctx.filter_ctx,
                    )
            })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::MonarchChanged])
    }
    fn display(&self) -> String {
        format!(
            "Whenever {} {} the monarch",
            self.player.description(),
            if self.player == PlayerFilter::You {
                "become"
            } else {
                "becomes"
            }
        )
    }
}
