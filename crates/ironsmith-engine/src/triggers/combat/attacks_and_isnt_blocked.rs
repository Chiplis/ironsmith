//! "Whenever [filter] attacks and isn't blocked" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedAndUnblockedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks and isn't blocked.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksAndIsntBlockedTrigger {
    pub filter: ObjectFilter,
    /// "one or more [filter] ... aren't blocked": one trigger per attacking
    /// player, carried by that player's first matching unblocked attacker.
    pub one_or_more: bool,
}

impl AttacksAndIsntBlockedTrigger {
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

    /// The grouped form reads its attacked-player constraint ("attack you")
    /// against the combat declaration, like the grouped attack trigger.
    fn grouped_matches(&self, attacker: crate::ids::ObjectId, ctx: &TriggerContext) -> bool {
        let (Some(attacker_obj), Some(combat)) = (ctx.game.object(attacker), ctx.game.combat.as_ref())
        else {
            return false;
        };
        let attacks = crate::triggers::combat::AttacksTrigger::one_or_more(self.filter.clone());
        let attacking_player = ctx.game.controller_of(attacker_obj);
        combat
            .attackers
            .iter()
            .find(|info| {
                !crate::combat_state::is_blocked(combat, info.creature)
                    && ctx
                        .game
                        .object(info.creature)
                        .is_some_and(|obj| ctx.game.controller_of(obj) == attacking_player)
                    && attacks.matches_attacker_info(info, ctx)
            })
            .is_some_and(|info| info.creature == attacker)
    }
}

impl TriggerMatcher for AttacksAndIsntBlockedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttackedAndUnblocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedAndUnblockedEvent>() else {
            return false;
        };
        let Some(obj) = ctx.game.object(e.attacker) else {
            return false;
        };
        if self.one_or_more {
            return self.grouped_matches(e.attacker, ctx);
        }
        self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
    }

    fn display(&self) -> String {
        if self.one_or_more {
            // Same subject and attacked-player surface as the grouped attack
            // trigger ("Whenever one or more creatures ... attack you").
            let attacks = crate::triggers::combat::AttacksTrigger::one_or_more(self.filter.clone());
            return format!("{} and aren't blocked", attacks.display());
        }
        format!(
            "Whenever {} attacks and isn't blocked",
            self.filter.description()
        )
    }
}
