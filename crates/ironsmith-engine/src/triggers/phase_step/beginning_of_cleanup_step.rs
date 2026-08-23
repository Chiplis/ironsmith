//! "At the beginning of [player]'s cleanup step" trigger.

use crate::events::EventKind;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires as a player's cleanup step begins.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfCleanupStepTrigger {
    pub player: PlayerFilter,
    /// Preserve the one-shot "the next cleanup step" event surface.
    pub next: bool,
}

impl BeginningOfCleanupStepTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            next: false,
        }
    }

    pub fn next(player: PlayerFilter) -> Self {
        Self { player, next: true }
    }
}

impl TriggerMatcher for BeginningOfCleanupStepTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BeginningOfCleanupStep {
            return false;
        }
        let Some(player) = event.player() else {
            return false;
        };
        crate::triggers::player_filter_matches_with_context(
            &self.player,
            player,
            ctx.controller,
            ctx.game,
            None,
        )
    }

    fn display(&self) -> String {
        if self.next {
            return match &self.player {
                PlayerFilter::You => "At the beginning of your next cleanup step".to_string(),
                PlayerFilter::Target(_) | PlayerFilter::IteratedPlayer => {
                    "At the beginning of that player's next cleanup step".to_string()
                }
                _ => "At the beginning of the next cleanup step".to_string(),
            };
        }
        match &self.player {
            PlayerFilter::You => "At the beginning of your cleanup step".to_string(),
            PlayerFilter::Any => "At the beginning of each player's cleanup step".to_string(),
            PlayerFilter::Opponent => {
                "At the beginning of each opponent's cleanup step".to_string()
            }
            PlayerFilter::Target(_) | PlayerFilter::IteratedPlayer => {
                "At the beginning of that player's cleanup step".to_string()
            }
            _ => "At the beginning of the cleanup step".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::BeginningOfCleanupStepEvent;
    use crate::ids::PlayerId;

    #[test]
    fn cleanup_trigger_matches_the_configured_players_step() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let ctx = TriggerContext::for_source(source, alice, &game);
        let trigger = BeginningOfCleanupStepTrigger::new(PlayerFilter::You);

        assert!(trigger.matches(
            &TriggerEvent::new(
                BeginningOfCleanupStepEvent::new(alice),
                crate::provenance::ProvNodeId::default(),
            ),
            &ctx
        ));
        assert!(!trigger.matches(
            &TriggerEvent::new(
                BeginningOfCleanupStepEvent::new(bob),
                crate::provenance::ProvNodeId::default(),
            ),
            &ctx
        ));
    }

    #[test]
    fn next_cleanup_trigger_keeps_a_one_event_surface_and_matches_cleanup() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let ctx = TriggerContext::for_source(source, alice, &game);
        let trigger = BeginningOfCleanupStepTrigger::next(PlayerFilter::Any);

        assert_eq!(
            trigger.display(),
            "At the beginning of the next cleanup step"
        );
        assert!(trigger.matches(
            &TriggerEvent::new(
                BeginningOfCleanupStepEvent::new(alice),
                crate::provenance::ProvNodeId::default(),
            ),
            &ctx
        ));
    }
}
