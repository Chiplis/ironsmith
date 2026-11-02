//! "Whenever [player] plays [land filter]" trigger.

use crate::events::EventKind;
use crate::events::other::LandPlayedEvent;

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
            PlayerFilter::Opponent => ctx.game.are_opponents(ctx.controller, e.player),
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
        // "play a land from anywhere other than your hand" (Shadow of the
        // Goblin): the excluded origin is the zone the land was played from.
        if filter
            .excluded_cast_origin_zone
            .take()
            .is_some_and(|zone| zone == e.from_zone)
        {
            return false;
        }
        if !filter.custom_statuses.is_empty() || !filter.excluded_custom_statuses.is_empty() {
            let origin=e.origin_snapshot.as_ref().filter(|origin|origin.zone==e.from_zone
                && e.snapshot.as_ref().is_some_and(|completed|completed.stable_id==origin.stable_id));
            let Some(origin)=origin else {
                ctx.game.record_token_resource_failure(&crate::effects::ExecutionError::IncompleteEvidence(
                    "land-play status predicate lacks its exact source-zone receipt".into()));
                return false;
            };
            if !filter.custom_statuses.iter().all(|name|origin.custom_statuses.contains(name))
                || filter.excluded_custom_statuses.iter().any(|name|origin.custom_statuses.contains(name)) { return false; }
            filter.custom_statuses.clear();
            filter.excluded_custom_statuses.clear();
        }
        match e.required_completed_snapshot() {
            Ok(snapshot) => super::permanent_lifecycle::matches_completed(&filter, snapshot, ctx),
            Err(error) => {
                ctx.game.record_token_resource_failure(&error);
                false
            }
        }
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
        let excluded_origin = filter.excluded_cast_origin_zone.take();
        let mut object_text = filter.description();
        if !object_text.starts_with("a ")
            && !object_text.starts_with("an ")
            && !object_text.starts_with("another ")
        {
            let article = match object_text.chars().next() {
                Some(ch) if matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u') => "an",
                _ => "a",
            };
            object_text = format!("{article} {object_text}");
        }
        if let Some(origin) = origin {
            object_text.push_str(&format!(" from {}", origin.name().to_lowercase()));
        }
        if let Some(excluded) = excluded_origin {
            let zone = match excluded {
                crate::zone::Zone::Hand => "hand".to_string(),
                other => other.name().to_lowercase(),
            };
            let possessive = if matches!(self.player, PlayerFilter::You) { "your" } else { "their" };
            object_text.push_str(&format!(" from anywhere other than {possessive} {zone}"));
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
                LandPlayedEvent::with_current_snapshot(land, player, origin, Zone::Battlefield, &game)
                    .expect("completed land-play receipt"),
                crate::provenance::ProvNodeId::default(),
            );
            assert_eq!(trigger.matches(&event, &ctx), expected);
        }
    }

    #[test]
    fn played_card_status_uses_exact_origin_without_marking_the_entered_permanent() {
        use crate::{card::CardBuilder,game_state::GameState,ids::{CardId,ObjectId,PlayerId},types::CardType,zone::Zone};
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let a=PlayerId(0);
        let card=CardBuilder::new(CardId::new(),"Origin land").card_types(vec![CardType::Land]).build();
        let origin=game.create_object_from_card(&card,a,Zone::Exile);
        game.set_custom_status_in_zone(origin,"archived",Zone::Exile,true);
        let snapshot=crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(origin).unwrap(),&game).unwrap();
        let land=game.move_object(origin,Zone::Battlefield,crate::events::EventCause::effect()).unwrap();
        assert!(!game.has_custom_status(land,"archived"));
        let mut filter=ObjectFilter::land();filter.custom_statuses.push("archived".into());
        let trigger=PlayerPlaysLandTrigger::new(PlayerFilter::You,filter);
        let ctx=TriggerContext::for_source(ObjectId::from_raw(99),a,&game);
        let base=LandPlayedEvent::with_current_snapshot(land,a,Zone::Exile,Zone::Battlefield,&game).unwrap();
        let event=|e|TriggerEvent::new_with_provenance(e,crate::provenance::ProvNodeId::default());
        assert!(!trigger.matches(&event(base.clone()),&ctx),"no receipt cannot borrow world LKI");
        assert!(trigger.matches(&event(base.clone().with_origin_snapshot(snapshot.clone())),&ctx));
        let mut unrelated=snapshot.clone();unrelated.stable_id=crate::ids::StableId::from(ObjectId::from_raw(81234));
        assert!(!trigger.matches(&event(base.clone().with_origin_snapshot(unrelated)),&ctx));
        let mut unmarked=snapshot;unmarked.custom_statuses.clear();
        assert!(!trigger.matches(&event(base.with_origin_snapshot(unmarked)),&ctx));
    }

    #[test]
    fn test_display() {
        let trigger = PlayerPlaysLandTrigger::new(PlayerFilter::Opponent, ObjectFilter::land());
        assert_eq!(trigger.display(), "Whenever an opponent plays a land");
    }
}
