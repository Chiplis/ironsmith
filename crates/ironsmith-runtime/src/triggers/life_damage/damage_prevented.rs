//! "Whenever damage that would be dealt to [player] is prevented" triggers.

use crate::events::{DamagePreventedEvent, DamageTarget, EventKind};
use crate::filter::PlayerFilterExt;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DamagePreventedTrigger {
    pub player: PlayerFilter,
}

impl DamagePreventedTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for DamagePreventedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::DamagePrevented {
            return false;
        }
        let Some(e) = event.downcast::<DamagePreventedEvent>() else {
            return false;
        };
        match e.target {
            DamageTarget::Player(player_id) => {
                self.player.matches_player(player_id, &ctx.filter_ctx)
            }
            DamageTarget::Object(_) => false,
        }
    }

    fn display(&self) -> String {
        format!(
            "Whenever damage that would be dealt to {} is prevented",
            self.player.description()
        )
    }

    fn event_value_amount(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> Option<i32> {
        let e = event.downcast::<DamagePreventedEvent>()?;
        Some(e.amount as i32)
    }
}
