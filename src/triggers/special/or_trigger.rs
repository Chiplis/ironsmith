//! Or trigger combinator - matches if any of the inner triggers match.

use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{Trigger, TriggerEvent};

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
}

impl TriggerMatcher for OrTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.triggers.iter().any(|t| t.matches(event, ctx))
    }

    fn display(&self) -> String {
        if self.triggers.is_empty() {
            return "never".to_string();
        }
        if self.triggers.len() == 1 {
            return self.triggers[0].display();
        }
        // Combine displays with "or"
        let displays: Vec<String> = self.triggers.iter().map(|t| t.display()).collect();
        displays.join(" or ")
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
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
    use crate::events::zones::ZoneChangeEvent;
    use crate::game_event::DamageTarget;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::triggers::ThisDealsCombatDamageToPlayerTrigger;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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
        let etb_event = TriggerEvent::new(ZoneChangeEvent::new(
            source_id,
            Zone::Hand,
            Zone::Battlefield,
            None,
        ));
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
        let damage_event = TriggerEvent::new(DamageEvent::new(
            source_id,
            DamageTarget::Player(bob),
            3,
            true, // is_combat
        ));
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
        let damage_event = TriggerEvent::new(DamageEvent::new(
            source_id,
            DamageTarget::Player(bob),
            3,
            false, // not combat
        ));
        assert!(!trigger.matches(&damage_event, &ctx));

        // ETB of different object shouldn't match
        let etb_event = TriggerEvent::new(ZoneChangeEvent::new(
            other_id,
            Zone::Hand,
            Zone::Battlefield,
            None,
        ));
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
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            source_id,
            Zone::Hand,
            Zone::Battlefield,
            None,
        ));
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
