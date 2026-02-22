//! Keyword action event implementation.

use std::any::Any;
use std::collections::HashMap;

use super::players_finished_voting::PlayerVote;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Keyword actions that can be observed by triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordActionKind {
    Convoke,
    Earthbend,
    Improvise,
    Investigate,
    NameSticker,
    Connive,
    Proliferate,
    Scry,
    Surveil,
    Vote,
}

impl KeywordActionKind {
    /// Parse the inflected trigger verb form.
    pub fn from_trigger_word(word: &str) -> Option<Self> {
        match word {
            "convoke" | "convokes" | "convoked" => Some(Self::Convoke),
            "earthbend" | "earthbends" => Some(Self::Earthbend),
            "improvise" | "improvises" | "improvised" => Some(Self::Improvise),
            "investigate" | "investigates" => Some(Self::Investigate),
            "sticker" | "stickers" | "stickered" => Some(Self::NameSticker),
            "connive" | "connives" | "connived" => Some(Self::Connive),
            "proliferate" | "proliferates" => Some(Self::Proliferate),
            "scry" | "scries" => Some(Self::Scry),
            "surveil" | "surveils" => Some(Self::Surveil),
            "vote" | "votes" | "voting" => Some(Self::Vote),
            _ => None,
        }
    }

    pub fn infinitive(self) -> &'static str {
        match self {
            Self::Convoke => "convoke",
            Self::Earthbend => "earthbend",
            Self::Improvise => "improvise",
            Self::Investigate => "investigate",
            Self::NameSticker => "put a name sticker",
            Self::Connive => "connive",
            Self::Proliferate => "proliferate",
            Self::Scry => "scry",
            Self::Surveil => "surveil",
            Self::Vote => "vote",
        }
    }

    pub fn third_person(self) -> &'static str {
        match self {
            Self::Convoke => "convokes",
            Self::Earthbend => "earthbends",
            Self::Improvise => "improvises",
            Self::Investigate => "investigates",
            Self::NameSticker => "puts a name sticker",
            Self::Connive => "connives",
            Self::Proliferate => "proliferates",
            Self::Scry => "scries",
            Self::Surveil => "surveils",
            Self::Vote => "votes",
        }
    }
}

/// Event emitted when a player performs a keyword action.
#[derive(Debug, Clone)]
pub struct KeywordActionEvent {
    /// Which keyword action was performed.
    pub action: KeywordActionKind,
    /// Player who performed the action.
    pub player: PlayerId,
    /// Source object that instructed/performed it.
    pub source: ObjectId,
    /// Optional action magnitude (for "scry N", "earthbend N", etc.).
    pub amount: u32,
    /// Optional vote records for "vote" keyword actions.
    pub votes: Option<Vec<PlayerVote>>,
    /// Optional tagged players attached to the action event.
    pub player_tags: HashMap<TagKey, Vec<PlayerId>>,
}

impl KeywordActionEvent {
    pub fn new(action: KeywordActionKind, player: PlayerId, source: ObjectId, amount: u32) -> Self {
        Self {
            action,
            player,
            source,
            amount,
            votes: None,
            player_tags: HashMap::new(),
        }
    }

    pub fn with_votes(mut self, votes: Vec<PlayerVote>) -> Self {
        self.votes = Some(votes);
        self
    }

    pub fn with_player_tags(mut self, tags: HashMap<TagKey, Vec<PlayerId>>) -> Self {
        self.player_tags.extend(tags);
        self
    }
}

impl GameEventType for KeywordActionEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::KeywordAction
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }

    fn display(&self) -> String {
        format!(
            "Player performed keyword action '{}' ({})",
            self.action.infinitive(),
            self.amount
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_action_parse_words() {
        assert_eq!(
            KeywordActionKind::from_trigger_word("earthbends"),
            Some(KeywordActionKind::Earthbend)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("surveil"),
            Some(KeywordActionKind::Surveil)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("voting"),
            Some(KeywordActionKind::Vote)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("sticker"),
            Some(KeywordActionKind::NameSticker)
        );
        assert_eq!(KeywordActionKind::from_trigger_word("unknown"), None);
    }

    #[test]
    fn keyword_action_event_kind() {
        let e = KeywordActionEvent::new(
            KeywordActionKind::Investigate,
            PlayerId::from_index(0),
            ObjectId::from_raw(1),
            1,
        );
        assert_eq!(e.event_kind(), EventKind::KeywordAction);
        assert_eq!(
            e.display(),
            "Player performed keyword action 'investigate' (1)"
        );
    }
}
