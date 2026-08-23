//! "Whenever [player] [keyword action]" trigger.

use crate::events::EventKind;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::Phase;
use crate::tag::{EXPLOITED_TAG, TagKey};
use crate::target::ObjectFilter;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::types::CardType;

const CREW_ACTIVATION_TAG: &str = "__crew_activation";

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
    pub during_your_turn: bool,
    pub during_your_main_phase: bool,
}

impl KeywordActionTrigger {
    pub fn new(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self {
            action,
            player,
            source_must_match: false,
            source_filter: None,
            tagged_object_filter: None,
            during_your_turn: false,
            during_your_main_phase: false,
        }
    }

    pub fn from_source(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self {
            action,
            player,
            source_must_match: true,
            source_filter: None,
            tagged_object_filter: None,
            during_your_turn: false,
            during_your_main_phase: false,
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
            during_your_turn: false,
            during_your_main_phase: false,
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
            during_your_turn: false,
            during_your_main_phase: false,
        }
    }

    pub fn during_your_turn(mut self) -> Self {
        self.during_your_turn = true;
        self
    }

    pub fn during_your_main_phase(mut self) -> Self {
        self.during_your_main_phase = true;
        self
    }

    fn with_timing_suffix(&self, display: String) -> String {
        if self.during_your_turn {
            format!("{display} during your turn")
        } else if self.during_your_main_phase {
            format!("{display} during your main phase")
        } else {
            display
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
        if !self.action.matches_performed_action(e.action) {
            return false;
        }
        if self.during_your_turn && !ctx.game.is_active_player(ctx.controller) {
            return false;
        }
        if self.during_your_main_phase
            && (!ctx.game.is_active_player(ctx.controller)
                || !matches!(ctx.game.turn.phase, Phase::FirstMain | Phase::NextMain))
        {
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
            if self.action == KeywordActionKind::Crew
                && object_filter.source
                && !e
                    .object_tags
                    .contains_key(&TagKey::from(CREW_ACTIVATION_TAG))
            {
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
        if self.source_must_match && self.action == KeywordActionKind::CumulativeUpkeepNotPaid {
            return match &self.player {
                PlayerFilter::You => {
                    "When you don't pay this permanent's cumulative upkeep".to_string()
                }
                PlayerFilter::Opponent => {
                    "When an opponent doesn't pay this permanent's cumulative upkeep".to_string()
                }
                PlayerFilter::Any => {
                    "When a player doesn't pay this permanent's cumulative upkeep".to_string()
                }
                _ => "When a player doesn't pay this permanent's cumulative upkeep".to_string(),
            };
        }
        if self.source_must_match && self.action == KeywordActionKind::Cycle {
            return match &self.player {
                PlayerFilter::You => "When you cycle this card".to_string(),
                PlayerFilter::Opponent => "When an opponent cycles this card".to_string(),
                PlayerFilter::Any => "When a player cycles this card".to_string(),
                _ => "When a player cycles this card".to_string(),
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
        if self.source_must_match && self.action == KeywordActionKind::UnlockDoor {
            return match &self.player {
                PlayerFilter::You => "When you unlock this door".to_string(),
                PlayerFilter::Opponent => "When an opponent unlocks this door".to_string(),
                PlayerFilter::Any => "When a player unlocks this door".to_string(),
                _ => "When a player unlocks this door".to_string(),
            };
        }
        if self.source_must_match && self.action == KeywordActionKind::Exploit {
            return "Whenever this creature exploits a creature".to_string();
        }
        if self.source_must_match && self.action == KeywordActionKind::Enlist {
            return "Whenever this creature enlists a creature".to_string();
        }
        if self.source_must_match && self.action == KeywordActionKind::Train {
            return "Whenever this creature trains".to_string();
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
        if self.action == KeywordActionKind::UnlockDoor
            && self
                .source_filter
                .as_ref()
                .is_some_and(|filter| filter.subtypes == [crate::types::Subtype::Room])
        {
            return match &self.player {
                PlayerFilter::You => "Whenever you fully unlock a Room".to_string(),
                PlayerFilter::Opponent => "Whenever an opponent fully unlocks a Room".to_string(),
                _ => "Whenever a player fully unlocks a Room".to_string(),
            };
        }
        if matches!(
            self.action,
            KeywordActionKind::Sticker
                | KeywordActionKind::NameSticker
                | KeywordActionKind::ArtSticker
                | KeywordActionKind::AbilitySticker
                | KeywordActionKind::PowerToughnessSticker
        ) {
            let sticker = match self.action {
                KeywordActionKind::Sticker => "a sticker",
                KeywordActionKind::NameSticker => "a name sticker",
                KeywordActionKind::ArtSticker => "an art sticker",
                KeywordActionKind::AbilitySticker => "an ability sticker",
                KeywordActionKind::PowerToughnessSticker => "a power and toughness sticker",
                _ => unreachable!("guarded by sticker-action match"),
            };
            let object = self
                .source_filter
                .as_ref()
                .map(|filter| ensure_singular_noun_phrase_article(filter.description()))
                .unwrap_or_else(|| "a creature".to_string());
            return match &self.player {
                PlayerFilter::You => format!("Whenever you put {sticker} on {object}"),
                PlayerFilter::Opponent => {
                    format!("Whenever an opponent puts {sticker} on {object}")
                }
                _ => format!("Whenever a player puts {sticker} on {object}"),
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
            let source = ensure_singular_noun_phrase_article(source_filter.description());
            return match &self.player {
                PlayerFilter::You => format!("Whenever you exert {source}"),
                PlayerFilter::Opponent => format!("Whenever an opponent exerts {source}"),
                PlayerFilter::Any => format!("Whenever a player exerts {source}"),
                _ => format!("Whenever a player exerts {source}"),
            };
        }
        if self.action == KeywordActionKind::Crew
            && let Some(source_filter) = &self.source_filter
        {
            if self
                .tagged_object_filter
                .as_ref()
                .is_some_and(|(_, object_filter)| object_filter.source)
            {
                let object = self
                    .tagged_object_filter
                    .as_ref()
                    .map(|(_, object_filter)| object_filter.description())
                    .unwrap_or_else(|| "this Vehicle".to_string());
                return self.with_timing_suffix(format!("Whenever {object} becomes crewed"));
            }
            let object = self
                .tagged_object_filter
                .as_ref()
                .map(|(_, object_filter)| {
                    ensure_singular_noun_phrase_article(object_filter.description())
                })
                .unwrap_or_else(|| "a Vehicle".to_string());
            return self.with_timing_suffix(format!(
                "Whenever {} crews {object}",
                source_filter.description()
            ));
        }
        if self.action == KeywordActionKind::Saddle
            && let Some(source_filter) = &self.source_filter
        {
            let object = self
                .tagged_object_filter
                .as_ref()
                .map(|(_, object_filter)| {
                    ensure_singular_noun_phrase_article(object_filter.description())
                })
                .unwrap_or_else(|| "a Mount".to_string());
            return self.with_timing_suffix(format!(
                "Whenever {} saddles {object}",
                source_filter.description()
            ));
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

        // Oracle names the cycled object: "Whenever a player cycles a card".
        let object_suffix = if self.action == KeywordActionKind::Cycle {
            " a card"
        } else {
            ""
        };
        let display = match &self.player {
            PlayerFilter::You => {
                format!("Whenever you {}{object_suffix}", self.action.infinitive())
            }
            PlayerFilter::Opponent => {
                format!(
                    "Whenever an opponent {}{object_suffix}",
                    self.action.third_person()
                )
            }
            PlayerFilter::Any => format!(
                "Whenever a player {}{object_suffix}",
                self.action.third_person()
            ),
            _ => format!(
                "Whenever a player {}{object_suffix}",
                self.action.third_person()
            ),
        };
        self.with_timing_suffix(display)
    }

    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        matches!(
            self.action,
            KeywordActionKind::Planeswalk | KeywordActionKind::CumulativeUpkeepNotPaid
        ) && event
            .downcast::<KeywordActionEvent>()
            .is_some_and(|event| event.action == self.action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;

    #[test]
    fn manifest_dread_observer_does_not_match_ordinary_manifest() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let trigger =
            KeywordActionTrigger::new(KeywordActionKind::ManifestDread, PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let manifest = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Manifest, alice, source_id, 1),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&manifest, &ctx));

        let manifest_dread = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::ManifestDread, alice, source_id, 1),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&manifest_dread, &ctx));
        assert_eq!(trigger.display(), "Whenever you manifest dread");
    }

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
    fn keyword_action_fully_unlock_room_display_preserves_room_fact() {
        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::UnlockDoor,
            PlayerFilter::You,
            ObjectFilter::default().with_subtype(crate::types::Subtype::Room),
        );
        assert_eq!(trigger.display(), "Whenever you fully unlock a Room");
    }

    #[test]
    fn source_door_unlock_is_a_one_shot_when_surface() {
        let trigger =
            KeywordActionTrigger::from_source(KeywordActionKind::UnlockDoor, PlayerFilter::You);
        assert_eq!(trigger.display(), "When you unlock this door");
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
    fn keyword_action_sticker_display_preserves_action_and_recipient() {
        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::Sticker,
            PlayerFilter::You,
            ObjectFilter::source().with_type(crate::types::CardType::Enchantment),
        );
        assert_eq!(
            trigger.display(),
            "Whenever you put a sticker on this enchantment"
        );

        let typed = KeywordActionTrigger::matching_object(
            KeywordActionKind::AbilitySticker,
            PlayerFilter::Opponent,
            ObjectFilter::creature(),
        );
        assert_eq!(
            typed.display(),
            "Whenever an opponent puts an ability sticker on a creature"
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
    fn keyword_action_saddle_matching_tagged_mount_uses_event_snapshot() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let saddler_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Pilot")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let saddler_id =
            game.create_object_from_card(&saddler_card, alice, crate::zone::Zone::Battlefield);
        let mount_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(2), "Mount")
            .card_types(vec![crate::types::CardType::Creature])
            .subtypes(vec![crate::types::Subtype::Mount])
            .build();
        let mount_id =
            game.create_object_from_card(&mount_card, alice, crate::zone::Zone::Battlefield);
        let mount_snapshot =
            ObjectSnapshot::from_object(game.object(mount_id).expect("mount"), &game);
        let vehicle_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(3), "Vehicle")
                .card_types(vec![crate::types::CardType::Artifact])
                .subtypes(vec![crate::types::Subtype::Vehicle])
                .build();
        let vehicle_id =
            game.create_object_from_card(&vehicle_card, alice, crate::zone::Zone::Battlefield);
        let vehicle_snapshot =
            ObjectSnapshot::from_object(game.object(vehicle_id).expect("vehicle"), &game);

        let trigger = KeywordActionTrigger::matching_source_and_tagged_object(
            KeywordActionKind::Saddle,
            PlayerFilter::Any,
            ObjectFilter::source_with_surface(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this creature".to_string(),
                ),
            ),
            TagKey::from("__it__"),
            ObjectFilter::default().with_subtype(crate::types::Subtype::Mount),
        );
        let ctx = TriggerContext::for_source(saddler_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Saddle, alice, saddler_id, 1)
                .with_snapshot(Some(ObjectSnapshot::from_object(
                    game.object(saddler_id).expect("saddler"),
                    &game,
                )))
                .with_object_tags(std::collections::HashMap::from([(
                    TagKey::from("__it__"),
                    vec![mount_snapshot],
                )])),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));

        let wrong_object_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Saddle, alice, saddler_id, 1)
                .with_snapshot(Some(ObjectSnapshot::from_object(
                    game.object(saddler_id).expect("saddler"),
                    &game,
                )))
                .with_object_tags(std::collections::HashMap::from([(
                    TagKey::from("__it__"),
                    vec![vehicle_snapshot],
                )])),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&wrong_object_event, &ctx));
    }

    #[test]
    fn keyword_action_during_your_main_phase_gates_keyword_events() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let trigger = KeywordActionTrigger::new(KeywordActionKind::Saddle, PlayerFilter::Any)
            .during_your_main_phase();
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Saddle, alice, source_id, 1),
            crate::provenance::ProvNodeId::default(),
        );

        game.turn.active_player = alice;
        game.turn.phase = Phase::FirstMain;
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&event, &ctx));

        game.turn.phase = Phase::Combat;
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(!trigger.matches(&event, &ctx));

        game.turn.active_player = bob;
        game.turn.phase = Phase::NextMain;
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn keyword_action_during_your_turn_gates_keyword_events_and_renders_timing() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let trigger = KeywordActionTrigger::new(KeywordActionKind::CommitCrime, PlayerFilter::You)
            .during_your_turn();
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::CommitCrime, alice, source_id, 1),
            crate::provenance::ProvNodeId::default(),
        );

        game.turn.active_player = alice;
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&event, &ctx));
        assert_eq!(
            trigger.display(),
            "Whenever you commit a crime during your turn"
        );

        game.turn.active_player = bob;
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(!trigger.matches(&event, &ctx));
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
        assert_eq!(trigger.display(), "Whenever you exert a creature");
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

    #[test]
    fn generic_sticker_trigger_matches_each_sticker_subtype() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(4), "Sticker Source")
                .card_types(vec![crate::types::CardType::Enchantment])
                .build();
        let source_id =
            game.create_object_from_card(&source_card, alice, crate::zone::Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object(game.object(source_id).expect("source"), &game);
        let trigger = KeywordActionTrigger::matching_object(
            KeywordActionKind::Sticker,
            PlayerFilter::You,
            ObjectFilter::source(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        for action in [
            KeywordActionKind::Sticker,
            KeywordActionKind::NameSticker,
            KeywordActionKind::ArtSticker,
            KeywordActionKind::AbilitySticker,
            KeywordActionKind::PowerToughnessSticker,
        ] {
            let event = TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(action, alice, source_id, 1)
                    .with_snapshot(Some(snapshot.clone())),
                crate::provenance::ProvNodeId::default(),
            );
            assert!(
                trigger.matches(&event, &ctx),
                "generic sticker trigger should match {action:?}"
            );
        }
    }
}
