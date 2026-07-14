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
    pub source_surface: ironsmith_core::trigger_model::DamageSourceSurface,
}

impl DealsDamageTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            damaged_player: None,
            combat_only: false,
            noncombat_only: false,
            source_surface: ironsmith_core::trigger_model::DamageSourceSurface::Filter,
        }
    }

    pub fn combat_only(filter: ObjectFilter) -> Self {
        Self {
            filter,
            damaged_player: None,
            combat_only: true,
            noncombat_only: false,
            source_surface: ironsmith_core::trigger_model::DamageSourceSurface::Filter,
        }
    }

    pub fn noncombat_to_player(
        filter: ObjectFilter,
        damaged_player: PlayerFilter,
        source_surface: ironsmith_core::trigger_model::DamageSourceSurface,
    ) -> Self {
        Self {
            filter,
            damaged_player: Some(damaged_player),
            combat_only: false,
            noncombat_only: true,
            source_surface,
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

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Damage])
    }

    fn display(&self) -> String {
        let source_description =
            if self.source_surface == ironsmith_core::trigger_model::DamageSourceSurface::Source {
                generic_source_description(&self.filter)
            } else if self.filter == ObjectFilter::default() {
                "a source".to_string()
            } else {
                self.filter.description()
            };
        if self.combat_only {
            format!("Whenever {} deals combat damage", source_description)
        } else if self.noncombat_only {
            if let Some(player) = &self.damaged_player {
                format!(
                    "Whenever {} deals noncombat damage to {}",
                    source_description,
                    player.description()
                )
            } else {
                format!("Whenever {} deals noncombat damage", source_description)
            }
        } else if let Some(player) = &self.damaged_player {
            if self.filter == ObjectFilter::default() {
                let player_description = player.description();
                if player_description == "you" {
                    return "Whenever you are dealt damage".to_string();
                }
                return format!("Whenever {} is dealt damage", player_description);
            }
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

pub(super) fn generic_source_description(filter: &ObjectFilter) -> String {
    let uses_default_permanent_noun = filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && !filter.token
        && !filter.nontoken
        && filter.stack_kind.is_none();
    if uses_default_permanent_noun {
        let description = filter.description();
        if description.contains("permanent") {
            let source = description.replacen("permanent", "source", 1);
            if source == "source" {
                return "a source".to_string();
            }
            return source;
        }
    }
    let mut remaining = filter.clone();
    let controller = remaining.controller.take();
    if remaining != ObjectFilter::default() {
        return filter.description();
    }
    match controller {
        Some(PlayerFilter::You) => "a source you control".to_string(),
        Some(PlayerFilter::Opponent) => "a source an opponent controls".to_string(),
        Some(PlayerFilter::NotYou) => "a source you don't control".to_string(),
        None | Some(PlayerFilter::Any) => "a source".to_string(),
        _ => filter.description(),
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

    #[test]
    fn generic_source_surface_and_opponent_recipient_are_preserved() {
        let mut filter = ObjectFilter::default();
        filter.controller = Some(PlayerFilter::You);
        let trigger = DealsDamageTrigger::noncombat_to_player(
            filter,
            PlayerFilter::Opponent,
            ironsmith_core::trigger_model::DamageSourceSurface::Source,
        );
        assert_eq!(
            trigger.display(),
            "Whenever a source you control deals noncombat damage to an opponent"
        );
    }
}
