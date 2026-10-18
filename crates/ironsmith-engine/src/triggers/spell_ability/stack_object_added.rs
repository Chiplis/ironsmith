//! Any completed spell/ability insertion (casts, activations, triggers, copies).
use crate::events::EventKind;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};
#[derive(Debug, Clone, PartialEq)]
pub struct StackObjectAddedTrigger;
impl TriggerMatcher for StackObjectAddedTrigger {
    fn matches(&self, event: &TriggerEvent, _: &TriggerContext) -> bool {
        event.downcast::<crate::events::spells::StackObjectAddedEvent>().is_some()
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> { Some(vec![EventKind::StackObjectAdded]) }
    fn display(&self) -> String { "Whenever a spell or ability is put onto the stack".into() }
}
