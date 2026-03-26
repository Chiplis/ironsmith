//! "At the beginning of [player]'s end step" trigger.

use crate::events::EventKind;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_possessive};

/// Trigger that fires at the beginning of a player's end step.
///
/// Used by cards like Conjurer's Closet, Obzedat, and many others.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfEndStepTrigger {
    /// Which player's end step triggers this ability.
    pub player: PlayerFilter,
}

impl BeginningOfEndStepTrigger {
    /// Create a new end step trigger for the specified player.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Create an end step trigger for your end step.
    pub fn your_end_step() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Create an end step trigger for each end step.
    pub fn each_end_step() -> Self {
        Self::new(PlayerFilter::Any)
    }
}

impl TriggerMatcher for BeginningOfEndStepTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BeginningOfEndStep {
            return false;
        }
        let Some(player) = event.player() else {
            return false;
        };
        player_filter_matches(&self.player, player, ctx)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "At the beginning of your end step".to_string(),
            PlayerFilter::Any => "At the beginning of each player's end step".to_string(),
            PlayerFilter::Opponent => "At the beginning of each opponent's end step".to_string(),
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag.as_str() == "enchanted" =>
            {
                "At the beginning of the end step of enchanted permanent's controller".to_string()
            }
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag.as_str() == "equipped" =>
            {
                "At the beginning of the end step of equipped creature's controller".to_string()
            }
            PlayerFilter::Target(_) | PlayerFilter::IteratedPlayer => {
                "At the beginning of that player's end step".to_string()
            }
            _ => format!(
                "At the beginning of {} end step",
                describe_player_filter_possessive(&self.player)
            ),
        }
    }
}

fn player_filter_matches(filter: &PlayerFilter, player: PlayerId, ctx: &TriggerContext) -> bool {
    match filter {
        PlayerFilter::You => player == ctx.controller,
        PlayerFilter::Opponent => player != ctx.controller,
        PlayerFilter::Any => true,
        PlayerFilter::Specific(id) => player == *id,
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            if matches!(tag.as_str(), "enchanted" | "equipped") =>
        {
            let Some(source) = ctx.game.object(ctx.source_id) else {
                return false;
            };
            let Some(attached_to) = source.attached_to else {
                return false;
            };
            match attached_to {
                crate::object::AttachmentTarget::Object(id) => ctx
                    .game
                    .object(id)
                    .is_some_and(|obj| obj.controller == player),
                crate::object::AttachmentTarget::Player(id) => id == player,
            }
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::BeginningOfEndStepEvent;
    use crate::game_state::GameState;
    use crate::ids::ObjectId;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_matches_own_end_step() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfEndStepTrigger::your_end_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_end_step() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfEndStepTrigger::your_end_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_each_end_step() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfEndStepTrigger::each_end_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event1 = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let event2 = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event1, &ctx));
        assert!(trigger.matches(&event2, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = BeginningOfEndStepTrigger::your_end_step();
        assert!(trigger.display().contains("end step"));
    }
}
