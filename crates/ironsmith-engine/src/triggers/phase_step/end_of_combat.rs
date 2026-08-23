//! "At end of combat" trigger.

use crate::events::EventKind;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires at end of combat.
///
/// Used by cards that clean up combat effects or exile attacking creatures.
#[derive(Debug, Clone, PartialEq)]
pub struct EndOfCombatTrigger;

impl TriggerMatcher for EndOfCombatTrigger {
    fn matches(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        event.kind() == EventKind::EndOfCombat
    }

    fn display(&self) -> String {
        "At end of combat".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::{BeginningOfCombatEvent, EndOfCombatEvent};
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_matches_end_of_combat() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = EndOfCombatTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            EndOfCombatEvent::new(),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_other_events() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = EndOfCombatTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            BeginningOfCombatEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = EndOfCombatTrigger;
        assert!(trigger.display().contains("end of combat"));
    }
}
