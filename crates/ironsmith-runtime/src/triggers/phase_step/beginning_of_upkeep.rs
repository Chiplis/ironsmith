//! "At the beginning of [player]'s upkeep" trigger.

use crate::events::EventKind;
use crate::filter::PlayerFilterExt as _;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_possessive};

/// Trigger that fires at the beginning of a player's upkeep step.
///
/// Used by cards like Black Market Connections, Bitterblossom, and many others.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfUpkeepTrigger {
    /// Which player's upkeep triggers this ability.
    pub player: PlayerFilter,
}

impl BeginningOfUpkeepTrigger {
    /// Create a new upkeep trigger for the specified player.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Create an upkeep trigger for your upkeep.
    pub fn your_upkeep() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Create an upkeep trigger for each player's upkeep.
    pub fn each_upkeep() -> Self {
        Self::new(PlayerFilter::Any)
    }
}

impl TriggerMatcher for BeginningOfUpkeepTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BeginningOfUpkeep {
            return false;
        }
        let Some(player) = event.player() else {
            return false;
        };
        player_filter_matches(&self.player, player, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::BeginningOfUpkeep])
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "At the beginning of your upkeep".to_string(),
            PlayerFilter::Any => "At the beginning of each player's upkeep".to_string(),
            PlayerFilter::Opponent => "At the beginning of each opponent's upkeep".to_string(),
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag.as_str() == "enchanted" =>
            {
                "At the beginning of the upkeep of enchanted permanent's controller".to_string()
            }
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag.as_str() == "equipped" =>
            {
                "At the beginning of the upkeep of equipped creature's controller".to_string()
            }
            _ => format!(
                "At the beginning of {} upkeep",
                describe_player_filter_possessive(&self.player)
            ),
        }
    }
}

/// Check if a PlayerFilter matches a specific player in the trigger context.
fn player_filter_matches(filter: &PlayerFilter, player: PlayerId, ctx: &TriggerContext) -> bool {
    match filter {
        PlayerFilter::You => player == ctx.controller,
        PlayerFilter::Opponent => player != ctx.controller,
        PlayerFilter::Any => true,
        PlayerFilter::Specific(id) => player == *id,
        PlayerFilter::Active => ctx.game.is_active_player(player),
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
                    .is_some_and(|obj| ctx.game.controller_of(obj) == player),
                crate::object::AttachmentTarget::Player(id) => id == player,
            }
        }
        _ => filter.matches_player(player, &ctx.filter_ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::BeginningOfDrawStepEvent;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::game_state::GameState;
    use crate::ids::ObjectId;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use std::collections::HashMap;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_matches_own_upkeep() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfUpkeepTrigger::your_upkeep();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_upkeep() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfUpkeepTrigger::your_upkeep();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_each_upkeep_matches_both() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfUpkeepTrigger::each_upkeep();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event1 = TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let event2 = TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event1, &ctx));
        assert!(trigger.matches(&event2, &ctx));
    }

    #[test]
    fn owner_of_delayed_tag_matches_only_that_objects_owner() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let triggering_id = ObjectId::from_raw(10);

        let mut triggering = ObjectSnapshot::for_testing(triggering_id, alice, "Returned Card");
        triggering.owner = bob;
        let tagged_objects = HashMap::from([(TagKey::from("triggering"), vec![triggering])]);
        let ctx = TriggerContext::for_delayed_source(source_id, alice, &game, &tagged_objects);
        let trigger = BeginningOfUpkeepTrigger::new(PlayerFilter::OwnerOf(
            crate::target::ObjectRef::Tagged(TagKey::from("triggering")),
        ));

        let alice_upkeep = TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let bob_upkeep = TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!trigger.matches(&alice_upkeep, &ctx));
        assert!(trigger.matches(&bob_upkeep, &ctx));
    }

    #[test]
    fn test_does_not_match_non_upkeep_events() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfUpkeepTrigger::your_upkeep();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            BeginningOfDrawStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display_your_upkeep() {
        let trigger = BeginningOfUpkeepTrigger::your_upkeep();
        assert!(trigger.display().contains("your upkeep"));
    }

    #[test]
    fn test_display_each_upkeep() {
        let trigger = BeginningOfUpkeepTrigger::each_upkeep();
        assert!(trigger.display().contains("each player's upkeep"));
    }
}
