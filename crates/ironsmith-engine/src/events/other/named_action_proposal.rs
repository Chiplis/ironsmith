//! Prospective named operation; distinct from its immutable completion receipt.
use std::any::Any;
use crate::{GameState, ObjectId, PlayerId};
use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher};
use crate::filter::{ObjectFilterExt as _, player_filter_matches_game};
use crate::snapshot::ObjectSnapshot;
use crate::target::{ObjectFilter, PlayerFilter};

#[derive(Debug, Clone)]
pub struct NamedActionProposalEvent {
    pub label: String,
    pub player: PlayerId,
    pub source: ObjectId,
    pub snapshot: ObjectSnapshot,
    pub objects: Vec<ObjectSnapshot>,
    /// The source-owned program is immutable across replacement applications.
    pub original_program: Vec<crate::Effect>,
    pub first_then_other_mode: Option<u32>,
    pub cause: EventCause,
}
impl GameEventType for NamedActionProposalEvent {
    fn event_kind(&self) -> EventKind { EventKind::KeywordAction }
    fn is_replacement_proposal(&self) -> bool { true }
    fn affected_player(&self, _: &GameState) -> PlayerId { self.player }
    fn player(&self) -> Option<PlayerId> { Some(self.player) }
    fn controller(&self) -> Option<PlayerId> { Some(self.snapshot.controller) }
    fn source_object(&self) -> Option<ObjectId> { Some(self.source) }
    fn object_id(&self) -> Option<ObjectId> { Some(self.source) }
    fn snapshot(&self) -> Option<&ObjectSnapshot> { Some(&self.snapshot) }
    fn cause(&self) -> Option<&EventCause> { Some(&self.cause) }
    fn display(&self) -> String { format!("Would perform the {} action", self.label) }
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WouldNamedActionMatcher {
    pub label: String,
    pub source_filter: ObjectFilter,
    pub performer_filter: Option<PlayerFilter>,
    pub object_requirement: Option<ironsmith_core::filter_model::NamedActionObjectRequirement>,
}
impl ReplacementMatcher for WouldNamedActionMatcher {
    fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &crate::events::context::PreparedEventContext) -> bool {
        let Some(proposal) = event.as_any().downcast_ref::<NamedActionProposalEvent>() else { return false; };
        proposal.label.eq_ignore_ascii_case(&self.label)
            && self.performer_filter.as_ref().is_none_or(|player| player_filter_matches_game(player, proposal.player, ctx.game, &ctx.filter_ctx))
            && self.source_filter.matches_snapshot(&proposal.snapshot, &ctx.filter_ctx, ctx.game)
            && self.object_requirement.as_ref().is_none_or(|requirement| proposal.objects.iter()
                .filter(|object|requirement.filter.matches_snapshot(object,&ctx.filter_ctx,ctx.game)).count()>=requirement.minimum as usize)
    }
    fn display(&self) -> String { format!("Would perform the {} action", self.label) }
}
