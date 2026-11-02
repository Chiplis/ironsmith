//! Immutable completion receipt for an explicitly labelled custom operation.
use std::any::Any;
use crate::{GameState, ObjectId, PlayerId};
use crate::events::{EventKind, GameEventType};
use crate::events::cause::EventCause;
use crate::snapshot::ObjectSnapshot;
#[derive(Debug, Clone)]
pub struct NamedActionEvent {
    pub label: String,
    pub player: PlayerId,
    /// Exact performer incarnation; a later zone-change object is different.
    pub source: ObjectId,
    pub snapshot: ObjectSnapshot,
    pub objects: Vec<ObjectSnapshot>,
    pub amount: u32,
    /// Existing turn-local combat ordinal at actual completion, or None
    /// outside combat. Bare announcements still lack completion authority.
    pub combat_phase: Option<u32>,
    pub cause: EventCause,
    pub mana_spent: Option<crate::player::ManaPool>,
    /// Player-only completions have attribution but no object performer role.
    pub player_only_performer: bool,
    pub complete_source_lookback: bool,
    pub participant_source_lookback: Option<Vec<ObjectSnapshot>>,
}
impl NamedActionEvent {
    pub fn with_combat_phase(mut self, combat_phase: Option<u32>) -> Self { self.combat_phase = combat_phase; self }
    pub fn with_player_only_performer(mut self) -> Self { self.player_only_performer = true; self }
    pub fn with_mana_spent(mut self,payment:crate::player::ManaPool)->Self {self.mana_spent=Some(payment);self}
    pub fn new(label: impl Into<String>, player: PlayerId, source: ObjectId,
        snapshot: ObjectSnapshot, objects: Vec<ObjectSnapshot>, amount: u32, cause: EventCause) -> Self {
        Self { label: label.into().trim().to_ascii_lowercase(), player, source, snapshot, objects, amount, combat_phase: None, cause, mana_spent:None, player_only_performer: false, complete_source_lookback: false, participant_source_lookback: None }
    }
    pub fn with_participant_source_lookback(mut self, sources: Vec<ObjectSnapshot>) -> Self { self.participant_source_lookback = Some(sources); self }
    pub fn with_complete_source_lookback(mut self) -> Self { self.complete_source_lookback = true; self }
}
impl GameEventType for NamedActionEvent {
    fn event_kind(&self) -> EventKind { EventKind::KeywordAction }
    fn is_replacement_proposal(&self) -> bool { false }
    fn affected_player(&self, _: &GameState) -> PlayerId { self.player }
    fn player(&self) -> Option<PlayerId> { Some(self.player) }
    fn controller(&self) -> Option<PlayerId> { Some(self.snapshot.controller) }
    fn source_object(&self) -> Option<ObjectId> { (!self.player_only_performer).then_some(self.source) }
    fn cause(&self) -> Option<&EventCause> { Some(&self.cause) }
    // The operation's objects bind "that card"; the performer has its own
    // immutable snapshot and explicit __named_action_source reference.
    fn object_id(&self) -> Option<ObjectId> { self.objects.first().map(|s| s.object_id).or_else(|| (!self.player_only_performer).then_some(self.source)) }
    fn snapshot(&self) -> Option<&ObjectSnapshot> { self.objects.first().or_else(|| (!self.player_only_performer).then_some(&self.snapshot)) }
    fn snapshots(&self) -> Vec<&ObjectSnapshot> { if self.objects.is_empty() { if self.player_only_performer { Vec::new() } else { vec![&self.snapshot] } } else { self.objects.iter().collect() } }
    fn display(&self) -> String { format!("Performed the {} action ({})", self.label, self.amount) }
    fn as_any(&self) -> &dyn Any { self }
}
