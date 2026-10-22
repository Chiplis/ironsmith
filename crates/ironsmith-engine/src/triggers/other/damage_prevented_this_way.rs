//! "Whenever damage [from a <quality> source] is prevented this way" — the
//! delayed triggered ability a prevention spell creates (CR 603.7). The
//! registration links it to the shield created just before it, so only
//! that shield's prevention events (CR 615.5, 615.13) reach this matcher's
//! delayed owner; the matcher itself qualifies the prevented damage's
//! source.

use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DamagePreventedThisWayTrigger {
    pub source_filter: Option<ObjectFilter>,
}

impl TriggerMatcher for DamagePreventedThisWayTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(prevented) = event.downcast::<crate::events::DamagePreventedEvent>() else {
            return false;
        };
        let Some(filter) = &self.source_filter else {
            return true;
        };
        prevented.applications.iter().any(|application| {
            ctx.game
                .object(application.damage_source)
                .is_some_and(|source| filter.matches(source, &ctx.filter_ctx, ctx.game))
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::DamagePrevented])
    }

    fn display(&self) -> String {
        match &self.source_filter {
            Some(filter) => {
                // A zone-unrestricted quality names a damage source of any
                // kind ("a black or red source"), not only a permanent.
                let source = if filter.zone.is_none() {
                    filter.description_with_default_noun("source")
                } else {
                    filter.description()
                };
                let article = if source.starts_with("a ")
                    || source.starts_with("an ")
                    || source.starts_with("the ")
                {
                    ""
                } else if source.starts_with(['a', 'e', 'i', 'o', 'u']) {
                    "an "
                } else {
                    "a "
                };
                format!("Whenever damage from {article}{source} is prevented this way")
            }
            None => "Whenever damage is prevented this way".to_string(),
        }
    }

    fn event_value_amount(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> Option<i32> {
        event
            .downcast::<crate::events::DamagePreventedEvent>()
            .map(|prevented| prevented.amount as i32)
    }
}

/// "Whenever damage that would be dealt to you is prevented" (Selfless
/// Squire): any prevention effect's application to damage that would have
/// been dealt to a matching player (CR 615.13). The event amount is the
/// prevented damage.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageToPlayerPreventedTrigger {
    pub player: crate::target::PlayerFilter,
}

impl DamageToPlayerPreventedTrigger {
    fn prevented_to_matching_player(
        &self,
        prevented: &crate::events::DamagePreventedEvent,
        ctx: &TriggerContext,
    ) -> u32 {
        use crate::filter::PlayerFilterExt as _;
        prevented
            .applications
            .iter()
            .filter(|application| match application.target {
                crate::events::DamageTarget::Player(player) => {
                    self.player.matches_player(player, &ctx.filter_ctx)
                }
                crate::events::DamageTarget::Object(_) => false,
            })
            .map(|application| application.amount)
            .sum()
    }
}

impl TriggerMatcher for DamageToPlayerPreventedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event
            .downcast::<crate::events::DamagePreventedEvent>()
            .is_some_and(|prevented| self.prevented_to_matching_player(prevented, ctx) > 0)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::DamagePrevented])
    }

    fn display(&self) -> String {
        if self.player == crate::target::PlayerFilter::You {
            "Whenever damage that would be dealt to you is prevented".to_string()
        } else {
            "Whenever damage that would be dealt to a player is prevented".to_string()
        }
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        event
            .downcast::<crate::events::DamagePreventedEvent>()
            .map(|prevented| self.prevented_to_matching_player(prevented, ctx) as i32)
    }
}

/// "When damage is prevented this way" printed after a permanent's own static
/// prevention (Phyrexian Vindicator): prevention events whose prevention
/// source is this permanent's static replacement (not a resolution shield).
#[derive(Debug, Clone, PartialEq)]
pub struct DamagePreventedBySourceTrigger {
    pub source_filter: Option<ObjectFilter>,
}

impl TriggerMatcher for DamagePreventedBySourceTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(prevented) = event.downcast::<crate::events::DamagePreventedEvent>() else {
            return false;
        };
        if prevented.prevention_source != ctx.source_id || prevented.prevention_shield.is_some() {
            return false;
        }
        let Some(filter) = &self.source_filter else {
            return true;
        };
        prevented.applications.iter().any(|application| {
            ctx.game
                .object(application.damage_source)
                .is_some_and(|source| filter.matches(source, &ctx.filter_ctx, ctx.game))
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::DamagePrevented])
    }

    fn display(&self) -> String {
        "When damage is prevented this way".to_string()
    }

    fn event_value_amount(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> Option<i32> {
        event
            .downcast::<crate::events::DamagePreventedEvent>()
            .map(|prevented| prevented.amount as i32)
    }
}
