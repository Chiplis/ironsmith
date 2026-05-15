//! Keyword action event implementation.

use std::any::Any;
use std::collections::HashMap;

use super::players_finished_voting::PlayerVote;
use crate::events::context::EventContext;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, downcast_event};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ObjectFilter;

pub use ironsmith_core::KeywordActionKind;

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
    /// Optional tagged object snapshots attached to the action event.
    pub object_tags: HashMap<TagKey, Vec<ObjectSnapshot>>,
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
            object_tags: HashMap::new(),
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

    pub fn with_object_tags(mut self, tags: HashMap<TagKey, Vec<ObjectSnapshot>>) -> Self {
        self.object_tags.extend(tags);
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

/// Matches when an object matching a filter would perform a keyword action.
#[derive(Debug, Clone, PartialEq)]
pub struct WouldKeywordActionMatcher {
    pub action: KeywordActionKind,
    pub source_filter: ObjectFilter,
}

impl WouldKeywordActionMatcher {
    pub fn new(action: KeywordActionKind, source_filter: ObjectFilter) -> Self {
        Self {
            action,
            source_filter,
        }
    }
}

impl ReplacementMatcher for WouldKeywordActionMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::KeywordAction {
            return false;
        }
        let Some(keyword_event) = downcast_event::<KeywordActionEvent>(event) else {
            return false;
        };
        if keyword_event.action != self.action {
            return false;
        }

        if let Some(snapshot) = keyword_event.snapshot.as_ref() {
            return self
                .source_filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }

        ctx.game.object(keyword_event.source).is_some_and(|object| {
            self.source_filter
                .matches(object, &ctx.filter_ctx, ctx.game)
        })
    }

    fn display(&self) -> String {
        format!(
            "When {} would {}",
            self.source_filter.description(),
            self.action.infinitive()
        )
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
