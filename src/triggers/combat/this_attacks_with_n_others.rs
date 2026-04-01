//! "Whenever this creature and at least N other creatures attack" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when the source creature attacks with at least N other creatures.
///
/// This captures battalion-style wording:
/// "Whenever this creature and at least two other creatures attack, ..."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThisAttacksWithNOthersTrigger {
    /// Minimum number of *other* attacking creatures required.
    pub other_count: usize,
    /// Whether the trigger requires exactly that many other attackers.
    pub exact: bool,
}

impl ThisAttacksWithNOthersTrigger {
    pub const fn new(other_count: usize) -> Self {
        Self {
            other_count,
            exact: false,
        }
    }

    pub const fn exact(other_count: usize) -> Self {
        Self {
            other_count,
            exact: true,
        }
    }
}

impl TriggerMatcher for ThisAttacksWithNOthersTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        // The source itself must be one of the attackers, and total attackers
        // must satisfy the requested source-plus-others threshold.
        e.attacker == ctx.source_id
            && if self.exact {
                e.total_attackers == self.other_count.saturating_add(1)
            } else {
                e.total_attackers >= self.other_count.saturating_add(1)
            }
    }

    fn display(&self) -> String {
        let noun = if self.other_count == 1 {
            "creature"
        } else {
            "creatures"
        };
        if self.exact {
            format!(
                "Whenever this creature and exactly {} other {} attack",
                self.other_count, noun
            )
        } else {
            format!(
                "Whenever this creature and at least {} other {} attack",
                self.other_count, noun
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn matches_when_source_attacks_with_enough_others() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(11);
        let trigger = ThisAttacksWithNOthersTrigger::new(2);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_source_is_not_attacker() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(11);
        let other_id = ObjectId::from_raw(12);
        let trigger = ThisAttacksWithNOthersTrigger::new(2);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                other_id,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_not_enough_attackers() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(11);
        let trigger = ThisAttacksWithNOthersTrigger::new(2);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn exact_mode_requires_exact_other_count() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(11);
        let trigger = ThisAttacksWithNOthersTrigger::exact(1);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let exact_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&exact_event, &ctx));

        let too_many_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&too_many_event, &ctx));
    }
}
