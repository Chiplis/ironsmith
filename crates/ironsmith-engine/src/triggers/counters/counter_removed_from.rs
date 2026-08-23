//! "Whenever a counter is removed from [filter]" trigger.

use crate::events::EventKind;
use crate::events::other::MarkersChangedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CounterRemovedFromTrigger {
    pub filter: ObjectFilter,
    pub counter_type: Option<crate::object::CounterType>,
    pub last: bool,
    pub one_or_more: bool,
    pub caused_by_source: bool,
}

impl CounterRemovedFromTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter_type: None,
            last: false,
            one_or_more: false,
            caused_by_source: false,
        }
    }

    pub fn one_or_more(mut self) -> Self {
        self.one_or_more = true;
        self
    }

    pub fn counter_type(mut self, counter_type: crate::object::CounterType) -> Self {
        self.counter_type = Some(counter_type);
        self
    }

    pub fn last(mut self) -> Self {
        self.last = true;
        self
    }

    pub fn caused_by_source(mut self) -> Self {
        self.caused_by_source = true;
        self
    }
}

impl TriggerMatcher for CounterRemovedFromTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::MarkersChanged {
            return false;
        }
        let Some(e) = event.downcast::<MarkersChangedEvent>() else {
            return false;
        };
        if !e.is_removed() {
            return false;
        }
        if self
            .counter_type
            .is_some_and(|counter_type| e.marker.as_counter() != Some(counter_type))
            || (self.last && e.count_after != Some(0))
        {
            return false;
        }
        if self.caused_by_source && e.source != Some(ctx.source_id) {
            return false;
        }

        let Some(object_id) = e.object() else {
            return false;
        };

        if let Some(obj) = ctx.game.object(object_id) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::MarkersChanged])
    }

    fn source_must_match_event_object(&self, event_kind: EventKind) -> bool {
        event_kind == EventKind::MarkersChanged && self.filter.source
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        if self.one_or_more {
            return 1;
        }
        event
            .downcast::<MarkersChangedEvent>()
            .map_or(1, |event| event.amount.max(1))
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        if !self.matches(event, ctx) {
            return None;
        }
        let event = event.downcast::<MarkersChangedEvent>()?;
        i32::try_from(event.amount).ok()
    }

    fn display(&self) -> String {
        if self.last {
            let counter = self.counter_type.map_or_else(
                || "counter".to_string(),
                |counter_type| format!("{} counter", counter_type.description()),
            );
            return format!(
                "When the last {counter} is removed from {}",
                self.filter.description()
            );
        }
        let counter_noun = self.counter_type.map_or_else(
            || "counter".to_string(),
            |counter_type| format!("{} counter", counter_type.description()),
        );
        let counter_phrase = if self.one_or_more {
            format!("one or more {counter_noun}s are")
        } else {
            format!("a {counter_noun} is")
        };
        let this_way = if self.caused_by_source {
            " this way"
        } else {
            ""
        };
        format!(
            "Whenever {counter_phrase} removed from {}{this_way}",
            self.filter.description()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::definitions::grizzly_bears;
    use crate::ids::PlayerId;
    use crate::object::CounterType;
    use crate::zone::Zone;

    #[test]
    fn test_display() {
        let trigger = CounterRemovedFromTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("counter is removed"));
    }

    #[test]
    fn grouped_named_counter_display_keeps_the_counter_type() {
        let trigger = CounterRemovedFromTrigger::new(ObjectFilter::source_with_surface(
            crate::target::SourceReferenceSurface::ShortName("Chandra".to_string()),
        ))
        .counter_type(CounterType::Loyalty)
        .one_or_more();
        assert_eq!(
            trigger.display(),
            "Whenever one or more loyalty counters are removed from Chandra"
        );
    }

    #[test]
    fn last_named_counter_uses_event_time_remainder_and_exact_type() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let trigger = CounterRemovedFromTrigger::new(ObjectFilter::source())
            .counter_type(CounterType::Charge)
            .last();
        let ctx = TriggerContext::for_source(source, alice, &game);

        let still_has_one = TriggerEvent::new(
            MarkersChangedEvent::removed(CounterType::Charge, source, 1, None, None)
                .with_count_after(1),
            crate::provenance::ProvNodeId::default(),
        );
        let last = TriggerEvent::new(
            MarkersChangedEvent::removed(CounterType::Charge, source, 1, None, None)
                .with_count_after(0),
            crate::provenance::ProvNodeId::default(),
        );
        let wrong_type = TriggerEvent::new(
            MarkersChangedEvent::removed(CounterType::PlusOnePlusOne, source, 1, None, None)
                .with_count_after(0),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&still_has_one, &ctx));
        assert!(trigger.matches(&last, &ctx));
        assert!(!trigger.matches(&wrong_type, &ctx));
        assert_eq!(
            trigger.display(),
            "When the last charge counter is removed from this"
        );
    }

    #[test]
    fn grouped_this_way_trigger_matches_only_source_caused_removals_and_exports_amount() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let trigger = CounterRemovedFromTrigger::new(ObjectFilter::source_with_surface(
            crate::target::SourceReferenceSurface::ThisPermanentType("this creature".to_string()),
        ))
        .one_or_more()
        .caused_by_source();
        let ctx = TriggerContext::for_source(source, alice, &game);

        let matching = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::removed(
                CounterType::PlusOnePlusOne,
                source,
                3,
                Some(source),
                Some(alice),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&matching, &ctx));
        assert_eq!(trigger.trigger_count(&matching), 1);
        assert_eq!(trigger.event_value_amount(&matching, &ctx), Some(3));
        assert_eq!(
            trigger.display(),
            "Whenever one or more counters are removed from this creature this way"
        );
        let singular = CounterRemovedFromTrigger::new(ObjectFilter::source()).caused_by_source();
        assert_eq!(
            singular.trigger_count(&matching),
            3,
            "the ungrouped form triggers once for each removed counter"
        );

        let unrelated_cause = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::removed(
                CounterType::PlusOnePlusOne,
                source,
                3,
                Some(other),
                Some(alice),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&unrelated_cause, &ctx));
        assert_eq!(trigger.event_value_amount(&unrelated_cause, &ctx), None);

        let wrong_object = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::removed(
                CounterType::PlusOnePlusOne,
                other,
                3,
                Some(source),
                Some(alice),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&wrong_object, &ctx));
    }
}
