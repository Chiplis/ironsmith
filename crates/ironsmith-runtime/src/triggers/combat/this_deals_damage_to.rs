//! "Whenever this permanent deals damage to [filter]" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};

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
        if !self
            .target_filter
            .matches(target_obj, &ctx.filter_ctx, ctx.game)
        {
            return false;
        }
        if self.combat_only && self.target_filter.union_is_one_or_more() {
            let already_matched =
                ctx.game
                    .combat_damage_object_batch_hits()
                    .iter()
                    .any(|(source, prior_target)| {
                        *source == damage.source
                            && ctx.game.object(*prior_target).is_some_and(|object| {
                                self.target_filter
                                    .matches(object, &ctx.filter_ctx, ctx.game)
                            })
                    });
            if already_matched {
                return false;
            }
        }
        true
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Damage])
    }

    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        if !self.target_filter.union_is_one_or_more() {
            return None;
        }
        let damage = event.downcast::<DamageEvent>()?;
        Some(SimultaneousTriggerKey::DamageSource(damage.source))
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
    let one_or_more = filter.union_is_one_or_more();
    let mut surface_filter = filter.clone();
    surface_filter.set_union_one_or_more(false);
    let description = surface_filter.description();
    if one_or_more {
        return format!("one or more {}", pluralize_damage_recipient(&description));
    }
    match description.as_str() {
        "artifact" | "enchantment" => format!("an {description}"),
        "creature" | "land" | "permanent" | "planeswalker" | "battle" => format!("a {description}"),
        description => description.to_string(),
    }
}

fn pluralize_damage_recipient(description: &str) -> String {
    let description = description
        .strip_prefix("a ")
        .or_else(|| description.strip_prefix("an "))
        .unwrap_or(description);
    if description.ends_with('s') {
        return description.to_string();
    }
    if let Some(stem) = description.strip_suffix('y')
        && !stem
            .chars()
            .last()
            .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        return format!("{stem}ies");
    }
    format!("{description}s")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cause::EventCause;
    use crate::ids::ObjectId;
    use crate::provenance::ProvNodeId;

    #[test]
    fn grouped_object_recipient_preserves_surface_and_source_batch_key() {
        let source = ObjectId::from_raw(1);
        let target = ObjectId::from_raw(2);
        let mut creatures = ObjectFilter::creature();
        creatures.set_union_one_or_more(true);

        let ordinary = ThisDealsDamageToTrigger::new(creatures.clone());
        assert_eq!(
            ordinary.display(),
            "Whenever this permanent deals damage to one or more creatures"
        );

        creatures.blocking = true;
        let combat = ThisDealsDamageToTrigger::combat_only(creatures);
        assert_eq!(
            combat.display(),
            "Whenever this permanent deals combat damage to one or more blocking creatures"
        );
        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source,
                DamageTarget::Object(target),
                2,
                true,
                EventCause::combat_damage(source),
            ),
            ProvNodeId::default(),
        );
        assert_eq!(
            combat.simultaneous_trigger_key(&event),
            Some(SimultaneousTriggerKey::DamageSource(source))
        );
    }
}
