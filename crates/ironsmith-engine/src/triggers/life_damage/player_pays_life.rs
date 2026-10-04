use crate::events::{EventKind, LifePaidEvent};
use crate::filter::player_filter_matches_game;
use crate::target::PlayerFilter;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerPaysLifeTrigger {
    pub player: PlayerFilter,
}
impl PlayerPaysLifeTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}
impl TriggerMatcher for PlayerPaysLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event.downcast::<LifePaidEvent>().is_some_and(|paid| {
            player_filter_matches_game(&self.player, paid.player, ctx.game, &ctx.filter_ctx)
        })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::LifePaid])
    }
    fn event_value_amount(&self, event: &TriggerEvent, _: &TriggerContext) -> Option<i32> {
        i32::try_from(event.downcast::<LifePaidEvent>()?.amount).ok()
    }
    fn display(&self) -> String {
        format!(
            "Whenever {} {} life",
            self.player.description(),
            if self.player == PlayerFilter::You {
                "pay"
            } else {
                "pays"
            }
        )
    }
}
