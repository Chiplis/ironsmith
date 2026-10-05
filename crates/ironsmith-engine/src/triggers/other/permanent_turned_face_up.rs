//! "Whenever [filter] is turned face up" trigger.

use crate::events::EventKind;
use crate::events::other::TurnedFaceUpEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentTurnedFaceUpTrigger {
    pub filter: ObjectFilter,
    pub player: Option<crate::target::PlayerFilter>,
}

impl PermanentTurnedFaceUpTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            player: None,
        }
    }
}

impl TriggerMatcher for PermanentTurnedFaceUpTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::TurnedFaceUp {
            return false;
        }
        let Some(e) = event.downcast::<TurnedFaceUpEvent>() else {
            return false;
        };
        if self.player.as_ref().is_some_and(|player| {
            !crate::filter::player_filter_matches_game(player, e.player, ctx.game, &ctx.filter_ctx)
        }) {
            return false;
        }
        e.snapshot
            .as_ref()
            .map(|snapshot| {
                super::permanent_lifecycle::matches_completed(&self.filter, snapshot, ctx)
            })
            .unwrap_or_else(|| {
                self.player.is_none()
                    && ctx.game.object(e.permanent).is_some_and(|object| {
                        self.filter.matches(object, &ctx.filter_ctx, ctx.game)
                    })
            })
    }

    fn uses_snapshot(&self) -> bool {
        true
    }

    fn display(&self) -> String {
        if let Some(player) = &self.player {
            format!(
                "Whenever {} turn{} {} face up",
                player.description(),
                if *player == crate::target::PlayerFilter::You {
                    ""
                } else {
                    "s"
                },
                self.filter.description()
            )
        } else {
            format!("Whenever {} is turned face up", self.filter.description())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = PermanentTurnedFaceUpTrigger::new(ObjectFilter::permanent().you_control());
        assert_eq!(
            trigger.display(),
            "Whenever a permanent you control is turned face up"
        );
    }
}
