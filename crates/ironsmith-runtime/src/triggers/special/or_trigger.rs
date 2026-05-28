//! Or trigger combinator - matches if any of the inner triggers match.

use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{
    ThisAttacksTrigger, TransformsTrigger, Trigger, TriggerEvent, ZoneChangeTrigger, ZonePattern,
};
use crate::zone::Zone;

/// A trigger that matches if any of the inner triggers match.
///
/// This is useful for cards like Tivit, Seller of Secrets which trigger
/// "whenever ~ enters the battlefield or deals combat damage to a player".
///
/// # Example
///
/// ```ignore
/// let trigger = Trigger::or(vec![
///     Trigger::this_enters_battlefield(),
///     Trigger::this_deals_combat_damage_to_player(),
/// ]);
/// ```
#[derive(Debug, Clone)]
pub struct OrTrigger {
    /// The inner triggers - matches if any of these match.
    pub triggers: Vec<Trigger>,
}

impl OrTrigger {
    /// Create a new OrTrigger with the given triggers.
    pub fn new(triggers: Vec<Trigger>) -> Self {
        Self { triggers }
    }

    /// Create an OrTrigger from exactly two triggers.
    pub fn two(a: Trigger, b: Trigger) -> Self {
        Self::new(vec![a, b])
    }

    fn self_enters_or_attacks_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let (zone_change, attacks_first) = if let (Some(zone_change), Some(_)) = (
            first.downcast_ref::<ZoneChangeTrigger>(),
            second.downcast_ref::<ThisAttacksTrigger>(),
        ) {
            (zone_change, false)
        } else if let (Some(_), Some(zone_change)) = (
            first.downcast_ref::<ThisAttacksTrigger>(),
            second.downcast_ref::<ZoneChangeTrigger>(),
        ) {
            (zone_change, true)
        } else {
            return None;
        };

        if !zone_change.this_object
            || zone_change.from != ZonePattern::Any
            || zone_change.to != ZonePattern::Specific(Zone::Battlefield)
            || zone_change.player != crate::triggers::PlayerRelation::Any
            || zone_change.cause_filter.is_some()
        {
            return None;
        }

        let subject = zone_change.this_subject_text("creature");
        let action = if attacks_first {
            "attacks or enters"
        } else {
            "enters or attacks"
        };
        Some(format!("Whenever {subject} {action}"))
    }

    fn self_enters_or_transforms_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let zone_change = if let (Some(zone_change), Some(_)) = (
            first.downcast_ref::<ZoneChangeTrigger>(),
            second.downcast_ref::<TransformsTrigger>(),
        ) {
            zone_change
        } else if let (Some(_), Some(zone_change)) = (
            first.downcast_ref::<TransformsTrigger>(),
            second.downcast_ref::<ZoneChangeTrigger>(),
        ) {
            zone_change
        } else {
            return None;
        };

        if !zone_change.this_object
            || zone_change.from != ZonePattern::Any
            || zone_change.to != ZonePattern::Specific(Zone::Battlefield)
            || zone_change.player != crate::triggers::PlayerRelation::Any
            || zone_change.cause_filter.is_some()
        {
            return None;
        }

        let subject = zone_change.this_subject_text("creature");
        Some(format!(
            "Whenever {subject} enters or transforms into {subject}"
        ))
    }
}

impl TriggerMatcher for OrTrigger {
    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }

    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.triggers.iter().any(|t| t.matches(event, ctx))
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        self.triggers
            .iter()
            .map(|trigger| trigger.trigger_count(event))
            .max()
            .unwrap_or(0)
    }

    fn trigger_count_with_context(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        self.triggers
            .iter()
            .filter(|trigger| trigger.matches(event, ctx))
            .map(|trigger| trigger.trigger_count_with_context(event, ctx))
            .max()
            .unwrap_or(0)
    }

    fn display(&self) -> String {
        if self.triggers.is_empty() {
            return "never".to_string();
        }
        if self.triggers.len() == 1 {
            return self.triggers[0].display();
        }
        if let Some(display) = self.self_enters_or_attacks_display() {
            return display;
        }
        if let Some(display) = self.self_enters_or_transforms_display() {
            return display;
        }
        // Combine displays with "or", stripping leading "When"/"Whenever" from
        // subsequent triggers to avoid "When X or When Y" → "When X or Y".
        let displays: Vec<String> = self.triggers.iter().map(|t| t.display()).collect();
        let mut parts = vec![displays[0].clone()];
        for d in &displays[1..] {
            let stripped = d
                .strip_prefix("When ")
                .or_else(|| d.strip_prefix("Whenever "))
                .unwrap_or(d);
            parts.push(stripped.to_string());
        }
        parts.join(" or ")
    }

    fn uses_snapshot(&self) -> bool {
        // Use snapshot if any inner trigger uses snapshot
        self.triggers.iter().any(|t| t.uses_snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::DamageEvent;
    use crate::events::DamageTarget;
    use crate::events::zones::ZoneChangeEvent;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::triggers::ThisDealsCombatDamageToPlayerTrigger;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn etb_event(source_id: ObjectId) -> ZoneChangeEvent {
        ZoneChangeEvent::with_cause(
            source_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        )
    }

    #[test]
    fn test_or_trigger_matches_first() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = OrTrigger::two(
            Trigger::this_enters_battlefield(),
            Trigger::new(ThisDealsCombatDamageToPlayerTrigger),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // ETB event should match
        let etb_event = TriggerEvent::new_with_provenance(
            etb_event(source_id),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&etb_event, &ctx));
    }

    #[test]
    fn test_or_trigger_matches_second() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = OrTrigger::two(
            Trigger::this_enters_battlefield(),
            Trigger::new(ThisDealsCombatDamageToPlayerTrigger),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Combat damage event should match
        let damage_event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                3,
                true, // is_combat
                crate::events::cause::EventCause::combat_damage(source_id),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&damage_event, &ctx));
    }

    #[test]
    fn test_or_trigger_no_match() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let other_id = ObjectId::from_raw(2);

        let trigger = OrTrigger::two(
            Trigger::this_enters_battlefield(),
            Trigger::new(ThisDealsCombatDamageToPlayerTrigger),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Non-combat damage from source shouldn't match
        let damage_event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                3,
                false, // not combat
                crate::events::cause::EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&damage_event, &ctx));

        // ETB of different object shouldn't match
        let etb_event = TriggerEvent::new_with_provenance(
            etb_event(other_id),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&etb_event, &ctx));
    }

    #[test]
    fn test_or_trigger_display() {
        let trigger = OrTrigger::two(
            Trigger::this_enters_battlefield(),
            Trigger::new(ThisDealsCombatDamageToPlayerTrigger),
        );

        let display = trigger.display();
        assert!(display.contains("enters the battlefield"));
        assert!(display.contains("or"));
        assert!(display.contains("deals combat damage"));
    }

    #[test]
    fn test_or_trigger_empty() {
        let trigger = OrTrigger::new(vec![]);
        assert_eq!(trigger.display(), "never");

        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            etb_event(source_id),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_or_trigger_single() {
        let trigger = OrTrigger::new(vec![Trigger::this_enters_battlefield()]);
        assert_eq!(
            trigger.display(),
            "When this permanent enters the battlefield"
        );
    }
}
