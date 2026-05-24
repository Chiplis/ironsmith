//! "Whenever [filter] deals damage" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::target::{PlayerFilter, PlayerFilterExt};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DealsDamageTrigger {
    pub filter: ObjectFilter,
    pub damaged_player: Option<PlayerFilter>,
    pub combat_only: bool,
    pub noncombat_only: bool,
}

impl DealsDamageTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            damaged_player: None,
            combat_only: false,
            noncombat_only: false,
        }
    }

    pub fn combat_only(filter: ObjectFilter) -> Self {
        Self {
            filter,
            damaged_player: None,
            combat_only: true,
            noncombat_only: false,
        }
    }

    pub fn noncombat_to_player(filter: ObjectFilter, damaged_player: PlayerFilter) -> Self {
        Self {
            filter,
            damaged_player: Some(damaged_player),
            combat_only: false,
            noncombat_only: true,
        }
    }
}

impl TriggerMatcher for DealsDamageTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if self.combat_only && !e.is_combat {
            return false;
        }
        if self.noncombat_only && e.is_combat {
            return false;
        }
        if let Some(player_filter) = &self.damaged_player {
            let DamageTarget::Player(player) = e.target else {
                return false;
            };
            if !player_filter.matches_player(player, &ctx.filter_ctx) {
                return false;
            }
        }
        if let Some(obj) = ctx.game.object(e.source) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        let source_description = if self.filter == ObjectFilter::default() {
            "a source".to_string()
        } else {
            self.filter.description()
        };
        if self.combat_only {
            format!("Whenever {} deals combat damage", source_description)
        } else if self.noncombat_only {
            if self.damaged_player.is_some() {
                format!(
                    "Whenever {} deals noncombat damage to a player",
                    source_description
                )
            } else {
                format!("Whenever {} deals noncombat damage", source_description)
            }
        } else if let Some(player) = &self.damaged_player {
            format!(
                "Whenever {} deals damage to {}",
                source_description,
                player.description()
            )
        } else {
            format!("Whenever {} deals damage", source_description)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = DealsDamageTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("deals damage"));
    }
}
