//! Departure notification. Leaving the game has no destination-zone object.
use std::any::Any;
use crate::events::{EventKind, GameEventType};
use crate::events::cause::EventCause;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

#[derive(Debug, Clone)]
pub struct ObjectLeavesGameEvent {
    pub object: ObjectId,
    pub snapshot: ObjectSnapshot,
    pub cause: EventCause,
}
impl ObjectLeavesGameEvent {
    pub fn new(object: ObjectId, snapshot: ObjectSnapshot, cause: EventCause) -> Self {
        Self { object, snapshot, cause }
    }
}
impl GameEventType for ObjectLeavesGameEvent {
    fn event_kind(&self) -> EventKind { EventKind::ObjectLeavesGame }
    fn is_replacement_proposal(&self) -> bool { false }
    fn affected_player(&self, _game: &GameState) -> PlayerId { self.snapshot.controller }
    fn object_id(&self) -> Option<ObjectId> { Some(self.object) }
    fn source_object(&self) -> Option<ObjectId> { self.cause.source }
    fn snapshot(&self) -> Option<&ObjectSnapshot> { Some(&self.snapshot) }
    fn snapshots(&self) -> Vec<&ObjectSnapshot> { vec![&self.snapshot] }
    fn display(&self) -> String { "Object leaves the game".into() }
    fn as_any(&self) -> &dyn Any { self }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Clone)]
    struct AnyEvent;
    impl crate::events::ReplacementMatcher for AnyEvent {
        fn matches_prepared_event(&self, _event: &dyn GameEventType, _ctx: &crate::events::context::PreparedEventContext) -> bool { true }
        fn display(&self) -> String { "any proposed event".into() }
    }
    #[test]
    fn notification_cannot_consume_an_eligible_replacement() {
        let mut game = GameState::new(vec!["Alice".into(),"Bob".into(),"Charlie".into()],20);
        let alice = PlayerId::from_index(0);
        let card = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Notification probe")
            .card_types(vec![crate::types::CardType::Creature]).build();
        let object = game.create_object_from_definition(&card,alice,crate::zone::Zone::Battlefield);
        let snapshot=ObjectSnapshot::from_object(game.object(object).unwrap(),&game);
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
            object,alice,AnyEvent,crate::replacement::ReplacementAction::Prevent));
        let event=crate::events::Event::new_with_provenance(ObjectLeavesGameEvent::new(object,snapshot,EventCause::from_game_rule()), Default::default());
        let outcome=crate::events::processing::process_trait_event(&mut game,event).expect("finite replacement fixture evaluates successfully");
        assert!(matches!(outcome,crate::events::processing::TraitEventResult::Proceed(event) if event.kind()==EventKind::ObjectLeavesGame));
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
    }
}
