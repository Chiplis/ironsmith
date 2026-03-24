//! "Whenever an ability of [filter] is activated" trigger.

use crate::events::EventKind;
use crate::events::spells::AbilityActivatedEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityActivatedTrigger {
    pub activator: PlayerFilter,
    pub filter: ObjectFilter,
    pub non_mana_only: bool,
}

impl AbilityActivatedTrigger {
    pub fn new(activator: PlayerFilter, filter: ObjectFilter, non_mana_only: bool) -> Self {
        Self {
            activator,
            filter,
            non_mana_only,
        }
    }
}

impl TriggerMatcher for AbilityActivatedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::AbilityActivated {
            return false;
        }
        let Some(e) = event.downcast::<AbilityActivatedEvent>() else {
            return false;
        };
        if self.non_mana_only && e.is_mana_ability {
            return false;
        }
        if !self.activator.matches_player(e.activator, &ctx.filter_ctx) {
            return false;
        }

        if let Some(obj) = ctx.game.object(e.source) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else if let Some(snapshot) = e.snapshot.as_ref() {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        let subject = self.activator.description();
        let ability = if self.non_mana_only {
            "a non-mana ability"
        } else {
            "an ability"
        };
        if self.filter == ObjectFilter::default() {
            format!("Whenever {subject} activates {ability}")
        } else {
            format!(
                "Whenever {subject} activates {ability} of {}",
                self.filter.description()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger =
            AbilityActivatedTrigger::new(PlayerFilter::Any, ObjectFilter::default(), false);
        assert!(trigger.display().contains("activates"));
    }
}
