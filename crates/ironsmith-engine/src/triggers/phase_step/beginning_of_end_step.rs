//! "At the beginning of [player]'s end step" trigger.

use crate::events::EventKind;
use crate::filter::PlayerFilterExt as _;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_possessive};
pub use ironsmith_core::trigger_model::EndStepSurface;

/// Trigger that fires at the beginning of a player's end step.
///
/// Used by cards like Conjurer's Closet, Obzedat, and many others.
#[derive(Clone, PartialEq)]
pub struct BeginningOfEndStepTrigger {
    /// Which player's end step triggers this ability.
    pub player: PlayerFilter,
    /// Oracle wording for Any-player end-step triggers.
    pub surface: EndStepSurface,
}

impl std::fmt::Debug for BeginningOfEndStepTrigger {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = formatter.debug_struct("BeginningOfEndStepTrigger");
        debug.field("player", &self.player);
        if self.surface != EndStepSurface::Each {
            debug.field("surface", &self.surface);
        }
        debug.finish()
    }
}

impl BeginningOfEndStepTrigger {
    /// Create a new end step trigger for the specified player.
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            surface: EndStepSurface::Each,
        }
    }

    /// Create an end step trigger for your end step.
    pub fn your_end_step() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Create an end step trigger for each end step.
    pub fn each_end_step() -> Self {
        Self::new(PlayerFilter::Any)
    }

    /// Create the legacy definite surface "the end step" while retaining the
    /// same Any-player runtime semantics as `each_end_step`.
    pub fn the_end_step() -> Self {
        Self {
            player: PlayerFilter::Any,
            surface: EndStepSurface::Definite,
        }
    }

    /// Create an end-step trigger qualified by the monarch designation at the
    /// time the event occurs. This is deliberately not an intervening-if.
    pub fn monarch_end_step() -> Self {
        Self {
            player: PlayerFilter::Any,
            surface: EndStepSurface::Monarch,
        }
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
        if self.surface == EndStepSurface::Monarch && ctx.game.monarch != Some(player) {
            return false;
        }
        player_filter_matches(&self.player, player, ctx)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "At the beginning of your end step".to_string(),
            PlayerFilter::Any if self.surface == EndStepSurface::Definite => {
                "At the beginning of the end step".to_string()
            }
            PlayerFilter::Any if self.surface == EndStepSurface::Monarch => {
                "At the beginning of the monarch's end step".to_string()
            }
            PlayerFilter::Any => "At the beginning of each player's end step".to_string(),
            PlayerFilter::Opponent => "At the beginning of each opponent's end step".to_string(),
            PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
                "At the beginning of enchanted player's end step".to_string()
            }
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
        PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
            let Some(source) = ctx.game.object(ctx.source_id) else {
                return false;
            };
            matches!(
                source.attached_to,
                Some(crate::object::AttachmentTarget::Player(attached_player))
                    if attached_player == player
            )
        }
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

    #[test]
    fn definite_surface_keeps_any_end_step_matching_semantics() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let trigger = BeginningOfEndStepTrigger::the_end_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        for active_player in [alice, bob] {
            let event = TriggerEvent::new_with_provenance(
                BeginningOfEndStepEvent::new(active_player),
                crate::provenance::ProvNodeId::default(),
            );
            assert!(trigger.matches(&event, &ctx));
        }
        assert_eq!(trigger.display(), "At the beginning of the end step");
    }

    #[test]
    fn monarch_surface_matches_only_the_current_monarchs_end_step() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.monarch = Some(bob);
        let trigger = BeginningOfEndStepTrigger::monarch_end_step();
        let source_id = ObjectId::from_raw(1);

        for (active_player, expected) in [(alice, false), (bob, true)] {
            let event = TriggerEvent::new_with_provenance(
                BeginningOfEndStepEvent::new(active_player),
                crate::provenance::ProvNodeId::default(),
            );
            let ctx = TriggerContext::for_source(source_id, alice, &game);
            assert_eq!(trigger.matches(&event, &ctx), expected);
        }
        assert_eq!(
            trigger.display(),
            "At the beginning of the monarch's end step"
        );
    }

    #[test]
    fn enchanted_player_end_step_matches_only_the_attached_player() {
        use crate::card::CardBuilder;
        use crate::object::AttachmentTarget;
        use crate::types::CardType;
        use crate::zone::Zone;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let aura = CardBuilder::new(crate::ids::CardId::from_raw(910_001), "Player Aura")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source_id = game.create_object_from_card(&aura, alice, Zone::Battlefield);
        game.object_mut(source_id).expect("aura exists").attached_to =
            Some(AttachmentTarget::Player(bob));

        let trigger = BeginningOfEndStepTrigger::new(PlayerFilter::TaggedPlayer(
            crate::tag::TagKey::from("enchanted"),
        ));
        assert_eq!(
            trigger.display(),
            "At the beginning of enchanted player's end step"
        );

        for (active_player, expected) in [(alice, false), (bob, true)] {
            let event = TriggerEvent::new_with_provenance(
                BeginningOfEndStepEvent::new(active_player),
                crate::provenance::ProvNodeId::default(),
            );
            let ctx = TriggerContext::for_source(source_id, alice, &game);
            assert_eq!(trigger.matches(&event, &ctx), expected);
        }
    }
}
