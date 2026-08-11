//! "Whenever [target] is dealt damage" trigger.

use crate::events::DamageTarget;
use crate::events::{DamageEvent, EventKind};
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct IsDealtDamageTrigger {
    pub target: ChooseSpec,
    pub combat_only: bool,
    pub noncombat_only: bool,
    pub excess_only: bool,
}

impl IsDealtDamageTrigger {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            combat_only: false,
            noncombat_only: false,
            excess_only: false,
        }
    }

    pub fn combat_only(target: ChooseSpec) -> Self {
        Self {
            target,
            combat_only: true,
            noncombat_only: false,
            excess_only: false,
        }
    }

    pub fn excess_noncombat(target: ChooseSpec) -> Self {
        Self {
            target,
            combat_only: false,
            noncombat_only: true,
            excess_only: true,
        }
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
            DamageTarget::Object(object_id) => target_matches_object(&self.target, object_id, ctx),
            DamageTarget::Player(player_id) => target_matches_player(&self.target, player_id, ctx),
        }
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Damage])
    }

    fn display(&self) -> String {
        let damage_text = if self.excess_only && self.noncombat_only {
            "excess noncombat damage"
        } else if self.combat_only {
            "combat damage"
        } else if self.noncombat_only {
            "noncombat damage"
        } else {
            "damage"
        };
        fn base_spec(spec: &ChooseSpec) -> &ChooseSpec {
            match spec {
                ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => base_spec(inner),
                other => other,
            }
        }

        match base_spec(&self.target) {
            ChooseSpec::Source => {
                format!("Whenever this creature is dealt {damage_text}")
            }
            ChooseSpec::SpecificObject(_) => {
                format!("Whenever that permanent is dealt {damage_text}")
            }
            ChooseSpec::Object(filter) => {
                format!("Whenever {} is dealt {damage_text}", filter.description())
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
        if !self.excess_only || !self.matches(event, ctx) {
            return None;
        }
        event
            .downcast::<DamageEvent>()
            .map(|damage| damage.excess_damage as i32)
    }
}

fn target_matches_object(
    spec: &ChooseSpec,
    object_id: crate::ids::ObjectId,
    ctx: &TriggerContext,
) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            target_matches_object(inner, object_id, ctx)
        }
        ChooseSpec::Source => object_id == ctx.source_id,
        ChooseSpec::SpecificObject(id) => object_id == *id,
        ChooseSpec::Object(filter) => ctx
            .game
            .object(object_id)
            .is_some_and(|obj| filter.matches(obj, &ctx.filter_ctx, ctx.game)),
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
        ChooseSpec::Player(filter) => filter.matches_player(player_id, &ctx.filter_ctx),
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
