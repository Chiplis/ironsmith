//! "Whenever this permanent deals damage to [filter]" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisDealsDamageToTrigger {
    pub target_filter: ObjectFilter,
    pub combat_only: bool,
}

impl ThisDealsDamageToTrigger {
    pub fn new(target_filter: ObjectFilter) -> Self {
        Self {
            target_filter,
            combat_only: false,
        }
    }

    pub fn combat_only(target_filter: ObjectFilter) -> Self {
        Self {
            target_filter,
            combat_only: true,
        }
    }
}

impl TriggerMatcher for ThisDealsDamageToTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(damage) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if damage.source != ctx.source_id {
            return false;
        }
        if self.combat_only && !damage.is_combat {
            return false;
        }
        let DamageTarget::Object(target_id) = damage.target else {
            return false;
        };
        let Some(target_obj) = ctx.game.object(target_id) else {
            return false;
        };
        self.target_filter
            .matches(target_obj, &ctx.filter_ctx, ctx.game)
    }

    fn display(&self) -> String {
        let target = damage_target_description(&self.target_filter);
        if self.combat_only {
            format!("Whenever this permanent deals combat damage to {}", target)
        } else {
            format!("Whenever this permanent deals damage to {}", target)
        }
    }
}

fn damage_target_description(filter: &ObjectFilter) -> String {
    let description = filter.description();
    match description.as_str() {
        "artifact" | "enchantment" => format!("an {description}"),
        "creature" | "land" | "permanent" | "planeswalker" | "battle" => format!("a {description}"),
        description => description.to_string(),
    }
}
