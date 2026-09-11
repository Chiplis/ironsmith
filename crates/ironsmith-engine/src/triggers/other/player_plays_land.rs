//! "Whenever [player] plays [land filter]" trigger.

use crate::events::EventKind;
use crate::events::other::LandPlayedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerPlaysLandTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
}

impl PlayerPlaysLandTrigger {
    pub fn new(player: PlayerFilter, mut filter: ObjectFilter) -> Self {
        // ObjectFilter::land describes a battlefield permanent by default.
        // That is the land's state after being played, not an origin restriction.
        if filter.zone == Some(crate::zone::Zone::Battlefield) {
            filter.zone = None;
        }
        Self { player, filter }
    }
}

impl TriggerMatcher for PlayerPlaysLandTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::LandPlayed {
            return false;
        }
        let Some(e) = event.downcast::<LandPlayedEvent>() else {
            return false;
        };

        let player_matches = match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Active => ctx.game.is_active_player(e.player),
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        };
        if !player_matches {
            return false;
        }

        // The event stores the origin; the land itself is already on the battlefield.
        let mut filter = self.filter.clone();
        if filter.zone.take().is_some_and(|zone| zone != e.from_zone) {
            return false;
        }
        ctx.game
            .object(e.land)
            .is_some_and(|obj| filter.matches(obj, &ctx.filter_ctx, ctx.game))
    }

    fn display(&self) -> String {
        let player_text = match &self.player {
            PlayerFilter::You => "you play",
            PlayerFilter::Opponent => "an opponent plays",
            PlayerFilter::Any => "a player plays",
            PlayerFilter::Active => "the active player plays",
            _ => "someone plays",
        };
        let mut filter = self.filter.clone();
        let origin = filter.zone.take();
        let mut object_text = filter.description();
        if !object_text.starts_with("a ") && !object_text.starts_with("an ") {
            let article = match object_text.chars().next() {
                Some(ch) if matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u') => "an",
                _ => "a",
            };
            object_text = format!("{article} {object_text}");
        }
        if let Some(origin) = origin {
            object_text.push_str(&format!(" from {}", origin.name().to_lowercase()));
        }
        format!("Whenever {player_text} {object_text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn land_play_origin_is_checked_before_matching_the_battlefield_object() {
        use crate::{
            card::CardBuilder,
            game_state::GameState,
            ids::{CardId, ObjectId, PlayerId},
            types::CardType,
            zone::Zone,
        };
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::from_raw(1), "Land Probe")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let trigger = PlayerPlaysLandTrigger::new(
            PlayerFilter::You,
            ObjectFilter::land().in_zone(Zone::Exile),
        );
        assert_eq!(trigger.display(), "Whenever you play a land from exile");
        let ctx = TriggerContext::for_source(ObjectId::from_raw(99), alice, &game);
        for (player, origin, expected) in [
            (alice, Zone::Exile, true),
            (alice, Zone::Hand, false),
            (bob, Zone::Exile, false),
        ] {
            let event = TriggerEvent::new_with_provenance(
                LandPlayedEvent::new(land, player, origin),
                crate::provenance::ProvNodeId::default(),
            );
            assert_eq!(trigger.matches(&event, &ctx), expected);
        }
    }

    #[test]
    fn test_display() {
        let trigger = PlayerPlaysLandTrigger::new(PlayerFilter::Opponent, ObjectFilter::land());
        assert_eq!(trigger.display(), "Whenever an opponent plays a land");
    }
}
