//! Match a typed completed gain using the affected permanent's after-state.
use crate::events::{CharacteristicGainedEvent, EventKind};
use crate::filter::ObjectFilterExt as _;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};
pub type CharacteristicGainedTrigger = ironsmith_core::trigger_model::CharacteristicGainTrigger;
impl TriggerMatcher for CharacteristicGainedTrigger {
    fn canonical_model(&self) -> Option<ironsmith_core::trigger_model::Trigger> {
        Some(ironsmith_core::trigger_model::Trigger::characteristic_gained(self.clone()))
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> { Some(vec![EventKind::CharacteristicGained]) }
    fn uses_snapshot(&self) -> bool { true }
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event.downcast::<CharacteristicGainedEvent>().is_some_and(|event|
            event.characteristic == self.characteristic
            && (!self.filter.source || event.permanent == ctx.source_id)
            && self.filter.matches_snapshot(&event.snapshot, &ctx.filter_ctx, ctx.game))
    }
    fn display(&self) -> String {
        use ironsmith_core::trigger_model::GainedCharacteristic as C;
        let (article, name) = match self.characteristic {
            C::CardType(value) => ("a ", value.name().to_owned()),
            C::Subtype(value) => ("a ", value.display_name()),
            C::Supertype(value) => ("", value.name().to_owned()),
        };
        format!("Whenever {} becomes {article}{name}", self.filter.description())
    }
}
