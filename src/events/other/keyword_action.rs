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
    Sticker,
    Amass,
    ArtSticker,
    AbilitySticker,
    Bolster,
    Clash,
    CommitCrime,
    Cycle,
    Convoke,
    Discover,
    CompleteDungeon,
    Evolve,
    Earthbend,
    Explore,
    Exert,
    Expend,
    Fateseal,
    Improvise,
    Investigate,
    Manifest,
    NameSticker,
    Populate,
    RingTemptsYou,
    Renown,
    Connive,
    Proliferate,
    Support,
    Scry,
    Surveil,
    Train,
    UnlockDoor,
    Vote,
}

impl KeywordActionKind {
    /// Parse the inflected trigger verb form.
    pub fn from_trigger_word(word: &str) -> Option<Self> {
        match word {
            "sticker" | "stickers" | "stickered" => Some(Self::Sticker),
            "amass" | "amasses" | "amassed" | "amassing" => Some(Self::Amass),
            "bolster" | "bolsters" | "bolstered" | "bolstering" => Some(Self::Bolster),
            "clash" | "clashes" | "clashed" | "clashing" => Some(Self::Clash),
            "crime" | "crimes" => Some(Self::CommitCrime),
            "cycle" | "cycles" | "cycled" | "cycling" => Some(Self::Cycle),
            "convoke" | "convokes" | "convoked" => Some(Self::Convoke),
            "discover" | "discovers" | "discovered" | "discovering" => Some(Self::Discover),
            "complete" | "completes" | "completed" | "completing" => Some(Self::CompleteDungeon),
            "evolve" | "evolves" | "evolved" | "evolving" => Some(Self::Evolve),
            "earthbend" | "earthbends" => Some(Self::Earthbend),
            "explore" | "explores" | "explored" | "exploring" => Some(Self::Explore),
            "exert" | "exerts" | "exerted" | "exerting" => Some(Self::Exert),
            "expend" | "expends" | "expended" => Some(Self::Expend),
            "fateseal" | "fateseals" | "fatesealed" | "fatesealing" => Some(Self::Fateseal),
            "improvise" | "improvises" | "improvised" => Some(Self::Improvise),
            "investigate" | "investigates" => Some(Self::Investigate),
            "manifest" | "manifests" | "manifested" | "manifesting" => Some(Self::Manifest),
            "populate" | "populates" | "populated" | "populating" => Some(Self::Populate),
            "renown" | "renowned" => Some(Self::Renown),
            "connive" | "connives" | "connived" => Some(Self::Connive),
            "proliferate" | "proliferates" => Some(Self::Proliferate),
            "support" | "supports" | "supported" | "supporting" => Some(Self::Support),
            "scry" | "scries" => Some(Self::Scry),
            "surveil" | "surveils" => Some(Self::Surveil),
            "train" | "trains" | "trained" | "training" => Some(Self::Train),
            "unlock" | "unlocks" | "unlocked" | "unlocking" => Some(Self::UnlockDoor),
            "vote" | "votes" | "voting" => Some(Self::Vote),
            _ => None,
        }
    }

    pub fn infinitive(self) -> &'static str {
        match self {
            Self::Sticker => "put a sticker",
            Self::Amass => "amass",
            Self::ArtSticker => "put an art sticker",
            Self::AbilitySticker => "put an ability sticker",
            Self::Bolster => "bolster",
            Self::Clash => "clash",
            Self::CommitCrime => "commit a crime",
            Self::Cycle => "cycle",
            Self::Convoke => "convoke",
            Self::Discover => "discover",
            Self::CompleteDungeon => "complete a dungeon",
            Self::Evolve => "evolve",
            Self::Earthbend => "earthbend",
            Self::Explore => "explore",
            Self::Exert => "exert",
            Self::Expend => "expend",
            Self::Fateseal => "fateseal",
            Self::Improvise => "improvise",
            Self::Investigate => "investigate",
            Self::Manifest => "manifest",
            Self::Populate => "populate",
            Self::NameSticker => "put a name sticker",
            Self::RingTemptsYou => "have the Ring tempt you",
            Self::Renown => "become renowned",
            Self::Connive => "connive",
            Self::Proliferate => "proliferate",
            Self::Support => "support",
            Self::Scry => "scry",
            Self::Surveil => "surveil",
            Self::Train => "train",
            Self::UnlockDoor => "unlock this door",
            Self::Vote => "vote",
        }
    }

    pub fn third_person(self) -> &'static str {
        match self {
            Self::Sticker => "puts a sticker",
            Self::Amass => "amasses",
            Self::ArtSticker => "puts an art sticker",
            Self::AbilitySticker => "puts an ability sticker",
            Self::Bolster => "bolsters",
            Self::Clash => "clashes",
            Self::CommitCrime => "commits a crime",
            Self::Cycle => "cycles",
            Self::Convoke => "convokes",
            Self::Discover => "discovers",
            Self::CompleteDungeon => "completes a dungeon",
            Self::Evolve => "evolves",
            Self::Earthbend => "earthbends",
            Self::Explore => "explores",
            Self::Exert => "exerts",
            Self::Expend => "expends",
            Self::Fateseal => "fateseals",
            Self::Improvise => "improvises",
            Self::Investigate => "investigates",
            Self::Manifest => "manifests",
            Self::Populate => "populates",
            Self::NameSticker => "puts a name sticker",
            Self::RingTemptsYou => "has the Ring tempt them",
            Self::Renown => "becomes renowned",
            Self::Connive => "connives",
            Self::Proliferate => "proliferates",
            Self::Support => "supports",
            Self::Scry => "scries",
            Self::Surveil => "surveils",
            Self::Train => "trains",
            Self::UnlockDoor => "unlocks this door",
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
    /// Snapshot of the object performing the action, when relevant.
    pub snapshot: Option<ObjectSnapshot>,
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
            snapshot: None,
            player_tags: HashMap::new(),
        }
    }

    pub fn with_votes(mut self, votes: Vec<PlayerVote>) -> Self {
        self.votes = Some(votes);
        self
    }

    pub fn with_snapshot(mut self, snapshot: Option<ObjectSnapshot>) -> Self {
        self.snapshot = snapshot;
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
        self.snapshot.as_ref()
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
            KeywordActionKind::from_trigger_word("amassing"),
            Some(KeywordActionKind::Amass)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("bolstered"),
            Some(KeywordActionKind::Bolster)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("clashed"),
            Some(KeywordActionKind::Clash)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("earthbends"),
            Some(KeywordActionKind::Earthbend)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("explores"),
            Some(KeywordActionKind::Explore)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("fatesealed"),
            Some(KeywordActionKind::Fateseal)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("manifested"),
            Some(KeywordActionKind::Manifest)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("populate"),
            Some(KeywordActionKind::Populate)
        );
        assert_eq!(
            KeywordActionKind::from_trigger_word("supported"),
            Some(KeywordActionKind::Support)
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
            Some(KeywordActionKind::Sticker)
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
