//! "Whenever [player] [keyword action]" trigger.

use crate::events::EventKind;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::target::ObjectFilter;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

fn is_plain_other_card_filter(filter: &ObjectFilter) -> bool {
    filter.other
        && !filter.source
        && filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.name.is_none()
        && filter.excluded_name.is_none()
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeywordActionTrigger {
    pub action: KeywordActionKind,
    pub player: PlayerFilter,
    pub source_must_match: bool,
    pub source_filter: Option<ObjectFilter>,
}

impl KeywordActionTrigger {
    pub fn new(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self {
            action,
            player,
            source_must_match: false,
            source_filter: None,
        }
    }

    pub fn from_source(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self {
            action,
            player,
            source_must_match: true,
            source_filter: None,
        }
    }

    pub fn matching_object(
        action: KeywordActionKind,
        player: PlayerFilter,
        source_filter: ObjectFilter,
    ) -> Self {
        Self {
            action,
            player,
            source_must_match: false,
            source_filter: Some(source_filter),
        }
    }
}

impl TriggerMatcher for KeywordActionTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::KeywordAction {
            return false;
        }
        let Some(e) = event.downcast::<KeywordActionEvent>() else {
            return false;
        };
        if e.action != self.action {
            return false;
        }

        if self.source_must_match {
            // Zone changes create a new ObjectId (rule 400.7), so match on the
            // source's stable identity when possible.
            let ctx_stable_source = ctx
                .game
                .object(ctx.source_id)
                .map(|obj| obj.stable_id.object_id())
                .unwrap_or(ctx.source_id);
            if e.source != ctx.source_id && e.source != ctx_stable_source {
                return false;
            }
        }

        if let Some(source_filter) = &self.source_filter {
            let matches = if let Some(snapshot) = e.snapshot.as_ref() {
                source_filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
            } else if let Some(source_object) = ctx.game.object(e.source) {
                source_filter.matches(source_object, &ctx.filter_ctx, ctx.game)
            } else {
                false
            };
            if !matches {
                return false;
            }
        }

        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        }
    }

    fn display(&self) -> String {
        if self.source_must_match && self.action == KeywordActionKind::Cycle {
            return match &self.player {
                PlayerFilter::You => "Whenever you cycle this card".to_string(),
                PlayerFilter::Opponent => "Whenever an opponent cycles this card".to_string(),
                PlayerFilter::Any => "Whenever a player cycles this card".to_string(),
                _ => "Whenever a player cycles this card".to_string(),
            };
        }
        if self.action == KeywordActionKind::Cycle
            && let Some(source_filter) = &self.source_filter
            && is_plain_other_card_filter(source_filter)
        {
            return match &self.player {
                PlayerFilter::You => "Whenever you cycle another card".to_string(),
                PlayerFilter::Opponent => "Whenever an opponent cycles another card".to_string(),
                PlayerFilter::Any => "Whenever a player cycles another card".to_string(),
                _ => "Whenever a player cycles another card".to_string(),
            };
        }
        if self.action == KeywordActionKind::Vote && self.player == PlayerFilter::Any {
            return "Whenever players finish voting".to_string();
        }
        if self.action == KeywordActionKind::NameSticker {
            return match &self.player {
                PlayerFilter::You => "Whenever you put a name sticker on a creature".to_string(),
                PlayerFilter::Opponent => {
                    "Whenever an opponent puts a name sticker on a creature".to_string()
                }
                _ => "Whenever a player puts a name sticker on a creature".to_string(),
            };
        }
        if self.action == KeywordActionKind::RingTemptsYou {
            return match &self.player {
                PlayerFilter::You => "Whenever the Ring tempts you".to_string(),
                PlayerFilter::Opponent => "Whenever the Ring tempts an opponent".to_string(),
                PlayerFilter::Any => "Whenever the Ring tempts a player".to_string(),
                _ => "Whenever the Ring tempts a player".to_string(),
            };
        }
        if self.action == KeywordActionKind::Exert
            && let Some(source_filter) = &self.source_filter
        {
            return match &self.player {
                PlayerFilter::You => {
                    format!("Whenever you exert {}", source_filter.description())
                }
                PlayerFilter::Opponent => format!(
                    "Whenever an opponent exerts {}",
                    source_filter.description()
                ),
                PlayerFilter::Any => {
                    format!("Whenever a player exerts {}", source_filter.description())
                }
                _ => format!("Whenever a player exerts {}", source_filter.description()),
            };
        }
        if self.action == KeywordActionKind::Explore
            && let Some(source_filter) = &self.source_filter
        {
            return format!(
                "Whenever {} {}",
                source_filter.description(),
                self.action.third_person()
            );
        }

        match &self.player {
            PlayerFilter::You => format!("Whenever you {}", self.action.infinitive()),
            PlayerFilter::Opponent => {
                format!("Whenever an opponent {}", self.action.third_person())
            }
            PlayerFilter::Any => format!("Whenever a player {}", self.action.third_person()),
            _ => format!("Whenever a player {}", self.action.third_person()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;

    #[test]
    fn keyword_action_trigger_matches_you() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = KeywordActionTrigger::new(KeywordActionKind::Earthbend, PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let you_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Earthbend, alice, source_id, 2),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&you_event, &ctx));

        let opp_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Earthbend, bob, source_id, 2),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&opp_event, &ctx));
    }

    #[test]
    fn keyword_action_trigger_matches_source_stable_id() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let hand_id = game.create_object_from_card(
            &crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Cycler")
                .card_types(vec![crate::types::CardType::Creature])
                .build(),
            alice,
            crate::zone::Zone::Hand,
        );
        let source_id = game
            .move_object_by_effect(hand_id, crate::zone::Zone::Graveyard)
            .expect("move to graveyard should create new id");

        // Simulate an event emitted using the old/stable ID.
        let stable = game
            .object(source_id)
            .map(|obj| obj.stable_id.object_id())
            .unwrap_or(source_id);
        assert_ne!(
            stable, source_id,
            "expected stable id to differ after zone change"
        );
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Cycle, alice, stable, 1),
            crate::provenance::ProvNodeId::default(),
        );

        let trigger =
            KeywordActionTrigger::from_source(KeywordActionKind::Cycle, PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn keyword_action_trigger_matches_another_cycled_card_and_excludes_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Source")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let source_id =
            game.create_object_from_card(&source_card, alice, crate::zone::Zone::Battlefield);

        let other_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(2), "Cycler")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let other_id =
            game.create_object_from_card(&other_card, alice, crate::zone::Zone::Graveyard);

        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::Cycle,
            PlayerFilter::You,
            ObjectFilter::default().other(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let other_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Cycle, alice, other_id, 1),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&other_event, &ctx));

        let source_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Cycle, alice, source_id, 1),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&source_event, &ctx));
    }

    #[test]
    fn keyword_action_cycle_another_card_display_phrase() {
        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::Cycle,
            PlayerFilter::You,
            ObjectFilter::default().other(),
        );
        assert_eq!(trigger.display(), "Whenever you cycle another card");
    }

    #[test]
    fn keyword_action_trigger_mismatched_action() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let trigger = KeywordActionTrigger::new(KeywordActionKind::Investigate, PlayerFilter::Any);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Scry, alice, source_id, 1),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn keyword_action_vote_display_uses_finished_voting_phrase() {
        let trigger = KeywordActionTrigger::new(KeywordActionKind::Vote, PlayerFilter::Any);
        assert_eq!(trigger.display(), "Whenever players finish voting");
    }

    #[test]
    fn keyword_action_name_sticker_display_phrase() {
        let trigger = KeywordActionTrigger::new(KeywordActionKind::NameSticker, PlayerFilter::You);
        assert_eq!(
            trigger.display(),
            "Whenever you put a name sticker on a creature"
        );
    }

    #[test]
    fn keyword_action_ring_tempts_display_phrase() {
        let trigger =
            KeywordActionTrigger::new(KeywordActionKind::RingTemptsYou, PlayerFilter::You);
        assert_eq!(trigger.display(), "Whenever the Ring tempts you");
    }

    #[test]
    fn keyword_action_explore_display_phrase_uses_subject_filter() {
        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::Explore,
            PlayerFilter::Any,
            ObjectFilter::creature().you_control(),
        );
        assert_eq!(
            trigger.display(),
            "Whenever a creature you control explores"
        );
    }

    #[test]
    fn keyword_action_matching_object_rejects_land_exert_for_creature_trigger() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Runner")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let creature_id =
            game.create_object_from_card(&creature_card, alice, crate::zone::Zone::Battlefield);

        let land_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(2), "Arena")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let land_id =
            game.create_object_from_card(&land_card, alice, crate::zone::Zone::Battlefield);

        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::Exert,
            PlayerFilter::You,
            ObjectFilter::creature(),
        );
        let ctx = TriggerContext::for_source(creature_id, alice, &game);

        let creature_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Exert, alice, creature_id, 1),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            trigger.matches(&creature_event, &ctx),
            "creature exert should satisfy the creature-only exert trigger"
        );

        let land_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Exert, alice, land_id, 1),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            !trigger.matches(&land_event, &ctx),
            "land exert should not satisfy a trigger that asks for exerting a creature"
        );
    }

    #[test]
    fn keyword_action_matching_object_uses_event_snapshot_for_explore_lki() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(3), "Explorer")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let battlefield_id =
            game.create_object_from_card(&creature_card, alice, crate::zone::Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(battlefield_id).expect("creature"), &game);
        let source_id = game
            .move_object_by_effect(battlefield_id, crate::zone::Zone::Graveyard)
            .expect("moving to graveyard should create a new id");

        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::Explore,
            PlayerFilter::Any,
            ObjectFilter::creature().you_control(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Explore, alice, snapshot.object_id, 1)
                .with_snapshot(Some(snapshot)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(
            trigger.matches(&event, &ctx),
            "explore triggers should use the event snapshot when the exploring permanent has left"
        );
    }
}
