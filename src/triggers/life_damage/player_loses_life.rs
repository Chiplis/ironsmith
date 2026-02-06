//! "Whenever [player] loses life" trigger.

use crate::events::EventKind;
use crate::events::life::LifeLossEvent;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLosesLifeTrigger {
    pub player: PlayerFilter,
}

impl PlayerLosesLifeTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for PlayerLosesLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::LifeLoss {
            return false;
        }
        let Some(e) = event.downcast::<LifeLossEvent>() else {
            return false;
        };
        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        }
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "Whenever you lose life".to_string(),
            PlayerFilter::Opponent => "Whenever an opponent loses life".to_string(),
            PlayerFilter::Any => "Whenever a player loses life".to_string(),
            _ => format!("Whenever {:?} loses life", self.player),
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = PlayerLosesLifeTrigger::new(PlayerFilter::Any);
        assert!(trigger.display().contains("loses life"));
    }
}
