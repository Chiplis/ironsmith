//! "Whenever [player] [keyword action]" trigger.

use crate::events::EventKind;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::filter::ObjectFilterExt as _;
use crate::tag::{EXPLOITED_TAG, TagKey};
use crate::target::ObjectFilter;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::types::CardType;

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

fn ensure_singular_noun_phrase_article(description: String) -> String {
    if description.starts_with("a ")
        || description.starts_with("an ")
        || description.starts_with("the ")
        || description.starts_with("this ")
        || description.starts_with("that ")
        || description.starts_with("each ")
        || description.starts_with("another ")
    {
        description
    } else {
        format!("a {description}")
    }
}

fn explore_revealed_card_phrase(filter: &ObjectFilter) -> String {
    let land_only = filter.card_types == [CardType::Land]
        && filter.excluded_card_types.is_empty()
        && filter.zone.is_none();
    let nonland_only = filter.card_types.is_empty()
        && filter.excluded_card_types == [CardType::Land]
        && filter.zone.is_none();
    if land_only {
        "a land card".to_string()
    } else if nonland_only {
        "a nonland card".to_string()
    } else {
        format!(
            "{} card",
            ensure_singular_noun_phrase_article(filter.description())
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeywordActionTrigger {
    pub action: KeywordActionKind,
    pub player: PlayerFilter,
    pub source_must_match: bool,
    pub source_filter: Option<ObjectFilter>,
    pub tagged_object_filter: Option<(TagKey, ObjectFilter)>,
}

impl KeywordActionTrigger {
    pub fn new(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self {
            action,
            player,
            source_must_match: false,
            source_filter: None,
            tagged_object_filter: None,
        }
    }

    pub fn from_source(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self {
            action,
            player,
            source_must_match: true,
            source_filter: None,
            tagged_object_filter: None,
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
            tagged_object_filter: None,
        }
    }

    pub fn matching_source_and_tagged_object(
        action: KeywordActionKind,
        player: PlayerFilter,
        source_filter: ObjectFilter,
        object_tag: TagKey,
        object_filter: ObjectFilter,
    ) -> Self {
        Self {
            action,
            player,
            source_must_match: false,
            source_filter: Some(source_filter),
            tagged_object_filter: Some((object_tag, object_filter)),
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

        if let Some((tag, object_filter)) = &self.tagged_object_filter {
            let matches = e.object_tags.get(tag).is_some_and(|snapshots| {
                snapshots.iter().any(|snapshot| {
                    object_filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
                })
            });
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
        if self.source_must_match && self.action == KeywordActionKind::Plot {
            return match &self.player {
                PlayerFilter::You => "When this card becomes plotted".to_string(),
                PlayerFilter::Opponent => {
                    "When this card becomes plotted by an opponent".to_string()
                }
                PlayerFilter::Any => "When this card becomes plotted".to_string(),
                _ => "When this card becomes plotted".to_string(),
            };
        }
        if self.source_must_match && self.action == KeywordActionKind::Exploit {
            return "Whenever this creature exploits a creature".to_string();
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
        if self.action == KeywordActionKind::ChaosEnsues && self.player == PlayerFilter::Any {
            return "Whenever chaos ensues".to_string();
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
        if self.action == KeywordActionKind::Crew
            && let Some(source_filter) = &self.source_filter
        {
            let object = self
                .tagged_object_filter
                .as_ref()
                .map(|(_, object_filter)| {
                    ensure_singular_noun_phrase_article(object_filter.description())
                })
                .unwrap_or_else(|| "a Vehicle".to_string());
            return format!("Whenever {} crews {object}", source_filter.description());
        }
        if self.action == KeywordActionKind::Explore
            && let Some(source_filter) = &self.source_filter
        {
            if let Some((tag, object_filter)) = &self.tagged_object_filter
                && tag.as_str() == crate::effects::PUBLIC_REVEALED_TAG
            {
                return format!(
                    "Whenever {} {} {}",
                    source_filter.description(),
                    self.action.third_person(),
                    explore_revealed_card_phrase(object_filter)
                );
            }
            return format!(
                "Whenever {} {}",
                source_filter.description(),
                self.action.third_person()
            );
        }
        if self.action == KeywordActionKind::Fight
            && let Some(source_filter) = &self.source_filter
        {
            return format!(
                "Whenever {} {}",
                source_filter.description(),
                self.action.third_person()
            );
        }
        if self.action == KeywordActionKind::Exploit
            && let Some(source_filter) = &self.source_filter
        {
            let subject = ensure_singular_noun_phrase_article(source_filter.description());
            let object = self
                .tagged_object_filter
                .as_ref()
                .filter(|(tag, _)| tag.as_str() == EXPLOITED_TAG)
                .map(|(_, object_filter)| {
                    ensure_singular_noun_phrase_article(object_filter.description())
                })
                .unwrap_or_else(|| "a creature".to_string());
            return format!("Whenever {subject} exploits {object}");
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
    fn keyword_action_plot_from_source_display_phrase() {
        let trigger = KeywordActionTrigger::from_source(KeywordActionKind::Plot, PlayerFilter::You);
        assert_eq!(trigger.display(), "When this card becomes plotted");
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
    fn keyword_action_explore_matching_revealed_card_filter_distinguishes_empty_library() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let explorer_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(10), "Explorer")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let explorer_id =
            game.create_object_from_card(&explorer_card, alice, crate::zone::Zone::Battlefield);
        let land_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(11), "Forest")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land_card, alice, crate::zone::Zone::Library);
        let land_snapshot = ObjectSnapshot::from_object(game.object(land_id).expect("land"), &game);

        let trigger = KeywordActionTrigger::matching_source_and_tagged_object(
            KeywordActionKind::Explore,
            PlayerFilter::Any,
            ObjectFilter::creature().you_control(),
            TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
            ObjectFilter::default().with_type(crate::types::CardType::Land),
        );
        let ctx = TriggerContext::for_source(explorer_id, alice, &game);
        let land_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Explore, alice, explorer_id, 1)
                .with_snapshot(Some(ObjectSnapshot::from_object(
                    game.object(explorer_id).expect("explorer"),
                    &game,
                )))
                .with_object_tags(std::collections::HashMap::from([(
                    TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
                    vec![land_snapshot],
                )])),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&land_event, &ctx));

        let empty_library_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Explore, alice, explorer_id, 1)
                .with_snapshot(Some(ObjectSnapshot::from_object(
                    game.object(explorer_id).expect("explorer"),
                    &game,
                ))),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            !trigger.matches(&empty_library_event, &ctx),
            "exploring with no revealed card should not satisfy land-card explore triggers"
        );
    }

    #[test]
    fn keyword_action_exploit_from_source_matches_and_displays_mechanic_phrase() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let trigger =
            KeywordActionTrigger::from_source(KeywordActionKind::Exploit, PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Exploit, alice, source_id, 1),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(
            trigger.display(),
            "Whenever this creature exploits a creature"
        );
    }

    #[test]
    fn keyword_action_exploit_matching_creature_filter_displays_mechanic_phrase() {
        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::Exploit,
            PlayerFilter::Any,
            ObjectFilter::creature().you_control(),
        );

        assert_eq!(
            trigger.display(),
            "Whenever a creature you control exploits a creature"
        );
    }

    #[test]
    fn keyword_action_exploit_matching_tagged_object_filters_exploited_object() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Exploiter")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let source_id =
            game.create_object_from_card(&source_card, alice, crate::zone::Zone::Battlefield);

        let exploited_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(2), "Victim")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let exploited_id =
            game.create_object_from_card(&exploited_card, alice, crate::zone::Zone::Battlefield);
        let mut exploited_snapshot =
            ObjectSnapshot::from_object(game.object(exploited_id).expect("exploited"), &game);

        let trigger = KeywordActionTrigger::matching_source_and_tagged_object(
            KeywordActionKind::Exploit,
            PlayerFilter::Any,
            ObjectFilter::creature().you_control(),
            TagKey::from(EXPLOITED_TAG),
            ObjectFilter::creature().nontoken(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let object_tags = std::collections::HashMap::from([(
            TagKey::from(EXPLOITED_TAG),
            vec![exploited_snapshot.clone()],
        )]);
        let nontoken_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Exploit, alice, source_id, 1)
                .with_object_tags(object_tags),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&nontoken_event, &ctx));

        exploited_snapshot.is_token = true;
        let object_tags = std::collections::HashMap::from([(
            TagKey::from(EXPLOITED_TAG),
            vec![exploited_snapshot],
        )]);
        let token_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Exploit, alice, source_id, 1)
                .with_object_tags(object_tags),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&token_event, &ctx));
    }

    #[test]
    fn keyword_action_exploit_matching_tagged_object_display_phrase() {
        let trigger = KeywordActionTrigger::matching_source_and_tagged_object(
            KeywordActionKind::Exploit,
            PlayerFilter::Any,
            ObjectFilter::creature().you_control(),
            TagKey::from(EXPLOITED_TAG),
            ObjectFilter::creature().nontoken(),
        );

        assert_eq!(
            trigger.display(),
            "Whenever a creature you control exploits a nontoken creature"
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
