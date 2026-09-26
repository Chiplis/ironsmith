//! "Whenever [filter] blocks" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBlockedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BlocksTrigger {
    pub filter: ObjectFilter,
    pub one_or_more: bool,
}

impl BlocksTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: false,
        }
    }

    pub fn one_or_more(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: true,
        }
    }

    fn is_first_matching_blocker_this_combat(
        &self,
        blocker: crate::ids::ObjectId,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(combat) = ctx.game.combat.as_ref() else {
            return true;
        };
        let Some(first_matching) = combat
            .blockers
            .values()
            .flat_map(|blockers| blockers.iter().copied())
            .filter(|blocker_id| {
                ctx.game
                    .object(*blocker_id)
                    .is_some_and(|obj| self.filter.matches(obj, &ctx.filter_ctx, ctx.game))
            })
            .min_by_key(|id| id.0)
        else {
            return true;
        };
        first_matching == blocker
    }
}

/// CR 509.3a: "Whenever [a creature] blocks" triggers only once each combat
/// for that creature, even if it blocks several attackers. One
/// `CreatureBlockedEvent` is emitted per (blocker, attacker) pair, so only the
/// pair with the lowest-id attacker this blocker is blocking counts. (The
/// per-attacker "blocks a creature" form, 509.3b, doesn't use this.)
pub(crate) fn is_first_blocked_attacker_for_blocker(
    game: &crate::game_state::GameState,
    blocker: crate::ids::ObjectId,
    attacker: crate::ids::ObjectId,
) -> bool {
    let Some(combat) = game.combat.as_ref() else {
        return true;
    };
    combat
        .blockers
        .iter()
        .filter(|(_, blockers)| blockers.contains(&blocker))
        .map(|(attacker_id, _)| *attacker_id)
        .min_by_key(|id| id.0)
        .is_none_or(|first| first == attacker)
}

impl TriggerMatcher for BlocksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBlockedEvent>() else {
            return false;
        };
        if let Some(obj) = ctx.game.object(e.blocker) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
                && is_first_blocked_attacker_for_blocker(ctx.game, e.blocker, e.attacker)
                && (!self.one_or_more || self.is_first_matching_blocker_this_combat(e.blocker, ctx))
        } else {
            false
        }
    }

    fn display(&self) -> String {
        if self.one_or_more {
            let mut subject = self.filter.description();
            if let Some(stripped) = subject.strip_prefix("a ") {
                subject = stripped.to_string();
            } else if let Some(stripped) = subject.strip_prefix("an ") {
                subject = stripped.to_string();
            }
            if subject == "creature" {
                subject = "creatures".to_string();
            } else if let Some(rest) = subject.strip_prefix("creature ") {
                subject = format!("creatures {rest}");
            }
            return format!("Whenever one or more {subject} block");
        }
        format!("Whenever {} blocks", self.filter.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = BlocksTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("blocks"));
    }
}
