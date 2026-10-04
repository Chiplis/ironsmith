//! "Whenever [target] is dealt damage" trigger.

use crate::events::DamageTarget;
use crate::events::{DamageEvent, EventKind};
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct IsDealtDamageTrigger {
    pub target: ChooseSpec,
    pub combat_only: bool,
    pub noncombat_only: bool,
    pub excess_only: bool,
    pub minimum: Option<u32>,
    pub single_source: bool,
}

impl IsDealtDamageTrigger {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            combat_only: false,
            noncombat_only: false,
            excess_only: false,
            minimum: None,
            single_source: false,
        }
    }

    pub fn combat_only(target: ChooseSpec) -> Self {
        Self {
            target,
            combat_only: true,
            noncombat_only: false,
            excess_only: false,
            minimum: None,
            single_source: false,
        }
    }

    /// "is dealt excess [combat] damage": keeps the excess requirement for
    /// wordings that don't also say "noncombat".
    pub fn excess(target: ChooseSpec, combat_only: bool) -> Self {
        Self {
            target,
            combat_only,
            noncombat_only: false,
            excess_only: true,
            minimum: None,
            single_source: false,
        }
    }

    pub fn excess_noncombat(target: ChooseSpec) -> Self {
        Self {
            target,
            combat_only: false,
            noncombat_only: true,
            excess_only: true,
            minimum: None,
            single_source: false,
        }
    }
}

impl IsDealtDamageTrigger {
    fn is_one_or_more(&self) -> bool {
        matches!(base_spec(&self.target), ChooseSpec::Object(filter) if filter.union_is_one_or_more())
    }
}

impl TriggerMatcher for IsDealtDamageTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if e.amount == 0 {
            return false;
        }
        if self.minimum.is_some_and(|minimum| {
            e.received_amount(
                if self.combat_only {
                    Some(true)
                } else if self.noncombat_only {
                    Some(false)
                } else {
                    None
                },
                self.single_source,
            ) < u128::from(minimum)
        }) {
            return false;
        }
        if self.combat_only && !e.is_combat {
            return false;
        }
        if self.noncombat_only && e.is_combat {
            return false;
        }
        if self.excess_only && e.excess_damage == 0 {
            return false;
        }

        match e.target {
            DamageTarget::Object(object_id) => {
                target_matches_object(&self.target, object_id, e.target_snapshot.as_ref(), ctx)
            }
            DamageTarget::Player(player_id) => target_matches_player(&self.target, player_id, ctx),
        }
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Damage])
    }

    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        let damage = event.downcast::<DamageEvent>()?;
        if self.single_source {
            return Some(SimultaneousTriggerKey::DamageSourceTarget(
                damage.source,
                damage.target,
            ));
        }
        if self.is_one_or_more() {
            return Some(SimultaneousTriggerKey::DamageBatch);
        }
        // CR 603.2c / 120.4b: damage dealt simultaneously by several sources
        // (combat damage from multiple blockers) is one event for each
        // recipient, so "whenever this creature is dealt damage" triggers once
        // per recipient, with "that much" being the total.
        Some(SimultaneousTriggerKey::DamageTarget(damage.target))
    }

    fn display(&self) -> String {
        let damage_text = if self.excess_only && self.noncombat_only {
            "excess noncombat damage"
        } else if self.excess_only && self.combat_only {
            "excess combat damage"
        } else if self.excess_only {
            "excess damage"
        } else if self.combat_only {
            "combat damage"
        } else if self.noncombat_only {
            "noncombat damage"
        } else {
            "damage"
        };
        let damage_text = format!(
            "{}{}{}",
            self.minimum
                .map(|minimum| format!("{minimum} or more "))
                .unwrap_or_default(),
            damage_text,
            if self.single_source {
                " by a single source"
            } else {
                ""
            }
        );
        match base_spec(&self.target) {
            ChooseSpec::Source => {
                format!("Whenever this creature is dealt {damage_text}")
            }
            ChooseSpec::SpecificObject(_) => {
                format!("Whenever that permanent is dealt {damage_text}")
            }
            ChooseSpec::Object(filter) => {
                let subject = if filter.union_is_one_or_more() {
                    let mut singular = filter.clone();
                    singular.set_union_one_or_more(false);
                    format!(
                        "one or more {}",
                        pluralize_damage_subject(&singular.description())
                    )
                } else {
                    filter.description()
                };
                let verb = if filter.union_is_one_or_more() {
                    "are"
                } else {
                    "is"
                };
                format!("Whenever {subject} {verb} dealt {damage_text}")
            }
            ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => {
                format!("Whenever a target is dealt {damage_text}")
            }
            ChooseSpec::SourceController => format!("Whenever you are dealt {damage_text}"),
            ChooseSpec::SourceOwner => format!("Whenever you are dealt {damage_text}"),
            ChooseSpec::SpecificPlayer(_) => {
                format!("Whenever that player is dealt {damage_text}")
            }
            ChooseSpec::Player(filter) => {
                format!("Whenever {} is dealt {damage_text}", filter.description())
            }
            _ => format!("Whenever a target is dealt {damage_text}"),
        }
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        if self.excess_only {
            if !self.matches(event, ctx) {
                return None;
            }
            return event
                .downcast::<DamageEvent>()
                .map(|damage| damage.excess_damage as i32);
        }
        // Per-recipient groups sum "that much damage" across the sources
        // merged into one trigger (see `simultaneous_trigger_key`).
        if self.is_one_or_more() || !self.matches(event, ctx) {
            return None;
        }
        event
            .downcast::<DamageEvent>()
            .map(|damage| damage.amount as i32)
    }
}

fn base_spec(spec: &ChooseSpec) -> &ChooseSpec {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => base_spec(inner),
        other => other,
    }
}

fn pluralize_damage_subject(description: &str) -> String {
    let description = description
        .strip_prefix("a ")
        .or_else(|| description.strip_prefix("an "))
        .unwrap_or(description);
    if description.ends_with('s') {
        description.to_string()
    } else if let Some(stem) = description.strip_suffix('y') {
        format!("{stem}ies")
    } else {
        format!("{description}s")
    }
}

fn target_matches_object(
    spec: &ChooseSpec,
    object_id: crate::ids::ObjectId,
    snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    ctx: &TriggerContext,
) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            target_matches_object(inner, object_id, snapshot, ctx)
        }
        ChooseSpec::Source => object_id == ctx.source_id,
        ChooseSpec::SpecificObject(id) => object_id == *id,
        ChooseSpec::Object(filter) => snapshot
            .filter(|snapshot| snapshot.object_id == object_id)
            .map(|snapshot| filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game))
            .unwrap_or_else(|| {
                ctx.game
                    .object(object_id)
                    .is_some_and(|obj| filter.matches(obj, &ctx.filter_ctx, ctx.game))
            }),
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => true,
        _ => false,
    }
}

fn target_matches_player(
    spec: &ChooseSpec,
    player_id: crate::ids::PlayerId,
    ctx: &TriggerContext,
) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            target_matches_player(inner, player_id, ctx)
        }
        ChooseSpec::SourceController => player_id == ctx.controller,
        ChooseSpec::SourceOwner => ctx
            .game
            .object(ctx.source_id)
            .is_some_and(|obj| obj.owner == player_id),
        ChooseSpec::SpecificPlayer(id) => player_id == *id,
        ChooseSpec::Player(filter) => {
            crate::filter::player_filter_matches_game(filter, player_id, ctx.game, &ctx.filter_ctx)
        }
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObjectFilter;
    use crate::events::cause::EventCause;
    use crate::ids::{ObjectId, PlayerId};
    use crate::provenance::ProvNodeId;
    use crate::target::FilterContext;

    #[test]
    fn test_display() {
        let trigger = IsDealtDamageTrigger::new(ChooseSpec::creature());
        assert!(trigger.display().contains("dealt damage"));

        let combat_trigger = IsDealtDamageTrigger::combat_only(ChooseSpec::creature());
        assert!(combat_trigger.display().contains("combat damage"));
    }

    #[test]
    fn excess_noncombat_trigger_matches_and_exports_only_the_excess() {
        let game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(11);
        let damaged = ObjectId::from_raw(12);
        let ctx = TriggerContext::new(source, alice, FilterContext::new(alice), &game);
        let trigger = IsDealtDamageTrigger::excess_noncombat(ChooseSpec::SpecificObject(damaged));
        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                ObjectId::from_raw(13),
                DamageTarget::Object(damaged),
                5,
                false,
                EventCause::effect(),
            )
            .with_excess_damage(3),
            ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.event_value_amount(&event, &ctx), Some(3));
        assert_eq!(
            trigger.display(),
            "Whenever that permanent is dealt excess noncombat damage"
        );

        let no_excess = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                ObjectId::from_raw(13),
                DamageTarget::Object(damaged),
                2,
                false,
                EventCause::effect(),
            ),
            ProvNodeId::default(),
        );
        let combat = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                ObjectId::from_raw(13),
                DamageTarget::Object(damaged),
                5,
                true,
                EventCause::effect(),
            )
            .with_excess_damage(3),
            ProvNodeId::default(),
        );
        assert!(!trigger.matches(&no_excess, &ctx));
        assert!(!trigger.matches(&combat, &ctx));
    }

    #[test]
    fn one_or_more_excess_recipients_share_one_simultaneous_damage_group() {
        let mut filter = ObjectFilter::creature();
        filter.set_union_one_or_more(true);
        let trigger = IsDealtDamageTrigger::excess_noncombat(ChooseSpec::Object(filter));
        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                ObjectId::from_raw(13),
                DamageTarget::Object(ObjectId::from_raw(12)),
                5,
                false,
                EventCause::effect(),
            )
            .with_excess_damage(3),
            ProvNodeId::default(),
        );

        assert_eq!(
            trigger.simultaneous_trigger_key(&event),
            Some(SimultaneousTriggerKey::DamageBatch)
        );
        assert_eq!(
            trigger.display(),
            "Whenever one or more creatures are dealt excess noncombat damage"
        );
    }
}
