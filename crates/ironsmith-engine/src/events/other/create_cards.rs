//! Typed prospective generation of real cards. No existing object changes zones.
use crate::events::{EventKind, GameEventType, ReplacementMatcher};
use crate::filter::player_filter_matches_game;
use crate::target::PlayerFilter;
use crate::zone::Zone;
use crate::{GameState, ObjectId, PlayerId};
use std::any::Any;

#[derive(Debug, Clone)]
pub struct CreateCardsEvent {
    pub operation_label: Option<String>,
    /// True for a retained destination descriptor after selection replacements.
    /// Such descriptors cannot acquire the selection replacements a second time.
    pub selection_finalized: bool,
    pub card_name: String,
    /// Exact applying source; its supplied definition is captured before choices.
    pub replacement_source: Option<ObjectId>,
    pub amount: u32,
    pub player: PlayerId,
    pub source: ObjectId,
    pub zone: Zone,
}
impl GameEventType for CreateCardsEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::KeywordAction
    }
    fn is_replacement_proposal(&self) -> bool {
        !self.selection_finalized
    }
    fn affected_player(&self, _: &GameState) -> PlayerId {
        self.player
    }
    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }
    fn controller(&self) -> Option<PlayerId> {
        Some(self.player)
    }
    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }
    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }
    fn display(&self) -> String {
        format!(
            "Create {} real cards named {} in {}",
            self.amount, self.card_name, self.zone
        )
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct WouldCreateCardsMatcher {
    pub operation_label: Option<String>,
    pub player: Option<PlayerFilter>,
}
impl ReplacementMatcher for WouldCreateCardsMatcher {
    fn matches_prepared_event(
        &self,
        event: &dyn GameEventType,
        ctx: &crate::events::context::PreparedEventContext,
    ) -> bool {
        let Some(creation) = event.as_any().downcast_ref::<CreateCardsEvent>() else {
            return false;
        };
        !creation.selection_finalized
            && self.operation_label.as_ref().is_none_or(|label| {
                creation
                    .operation_label
                    .as_ref()
                    .is_some_and(|actual| actual.eq_ignore_ascii_case(label))
            })
            && self.player.as_ref().is_none_or(|filter| {
                player_filter_matches_game(filter, creation.player, ctx.game, &ctx.filter_ctx)
            })
    }
    fn display(&self) -> String {
        "Would generate real cards".into()
    }
}
