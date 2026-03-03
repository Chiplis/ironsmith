//! "Whenever a counter is put on [filter]" trigger.

use crate::events::EventKind;
use crate::events::other::{CounterPlacedEvent, MarkersChangedEvent};
use crate::object::CounterType;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::CountMode;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CounterPutOnTrigger {
    pub filter: ObjectFilter,
    pub counter_type: Option<CounterType>,
    pub source_controller: Option<PlayerFilter>,
    pub count_mode: CountMode,
}

impl CounterPutOnTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter_type: None,
            source_controller: None,
            count_mode: CountMode::Each,
        }
    }

    pub fn counter_type(mut self, counter_type: CounterType) -> Self {
        self.counter_type = Some(counter_type);
        self
    }

    pub fn source_controller(mut self, source_controller: PlayerFilter) -> Self {
        self.source_controller = Some(source_controller);
        self
    }

    pub fn count(mut self, count_mode: CountMode) -> Self {
        self.count_mode = count_mode;
        self
    }
}

impl TriggerMatcher for CounterPutOnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let (permanent, counter_type, source_controller) = match event.kind() {
            EventKind::CounterPlaced => {
                let Some(e) = event.downcast::<CounterPlacedEvent>() else {
                    return false;
                };
                (e.permanent, e.counter_type, None)
            }
            EventKind::MarkersChanged => {
                let Some(e) = event.downcast::<MarkersChangedEvent>() else {
                    return false;
                };
                if !e.is_added() {
                    return false;
                }
                let Some(counter_type) = e.marker.as_counter() else {
                    return false;
                };
                let Some(permanent) = e.object() else {
                    return false;
                };
                (permanent, counter_type, e.source_controller)
            }
            _ => return false,
        };

        if let Some(required_source_controller) = &self.source_controller {
            let Some(source_controller) = source_controller else {
                return false;
            };
            if !required_source_controller.matches_player(source_controller, &ctx.filter_ctx) {
                return false;
            }
        }

        if let Some(required_counter_type) = self.counter_type
            && counter_type != required_counter_type
        {
            return false;
        }
        if let Some(obj) = ctx.game.object(permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        match self.count_mode {
            CountMode::OneOrMore => 1,
            CountMode::Each => {
                if let Some(e) = event.downcast::<CounterPlacedEvent>() {
                    e.amount.max(1)
                } else if let Some(e) = event.downcast::<MarkersChangedEvent>() {
                    e.amount.max(1)
                } else {
                    1
                }
            }
        }
    }

    fn display(&self) -> String {
        fn counter_text(counter_type: CounterType) -> String {
            match counter_type {
                CounterType::PlusOnePlusOne => "+1/+1".to_string(),
                CounterType::MinusOneMinusOne => "-1/-1".to_string(),
                CounterType::Named(name) => name.to_string(),
                other => format!("{other:?}").to_ascii_lowercase(),
            }
        }

        let counters = match self.counter_type {
            Some(counter_type) => format!("{} counter", counter_text(counter_type)),
            None => "counter".to_string(),
        };

        let counter_phrase = match self.count_mode {
            CountMode::OneOrMore => format!("one or more {}s", counters),
            CountMode::Each => format!("a {}", counters),
        };

        if let Some(source_controller) = &self.source_controller {
            let (subject, verb) = if source_controller == &PlayerFilter::You {
                ("you".to_string(), "put")
            } else {
                (source_controller.description(), "puts")
            };
            return format!(
                "Whenever {subject} {verb} {counter_phrase} on {}",
                self.filter.description()
            );
        }

        match self.count_mode {
            CountMode::OneOrMore => format!(
                "Whenever one or more {}s are put on {}",
                counters,
                self.filter.description()
            ),
            CountMode::Each => format!(
                "Whenever a {} is put on {}",
                counters,
                self.filter.description()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::definitions::grizzly_bears;
    use crate::events::other::MarkersChangedEvent;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    #[test]
    fn test_display() {
        let trigger = CounterPutOnTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("counter is put on"));
    }

    #[test]
    fn test_matches_markers_changed_for_you_put() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);

        let trigger = CounterPutOnTrigger::new(ObjectFilter::creature())
            .counter_type(CounterType::MinusOneMinusOne)
            .source_controller(PlayerFilter::You)
            .count(CountMode::OneOrMore);
        let ctx = TriggerContext::for_source(creature_id, alice, &game);

        let your_event = TriggerEvent::new(MarkersChangedEvent::added(
            CounterType::MinusOneMinusOne,
            creature_id,
            2,
            Some(creature_id),
            Some(alice),
        ));
        assert!(
            trigger.matches(&your_event, &ctx),
            "expected trigger to match your -1/-1 counter placement"
        );

        let opponent_event = TriggerEvent::new(MarkersChangedEvent::added(
            CounterType::MinusOneMinusOne,
            creature_id,
            2,
            Some(creature_id),
            Some(bob),
        ));
        assert!(
            !trigger.matches(&opponent_event, &ctx),
            "expected trigger to reject opponent counter placement"
        );
    }

    #[test]
    fn test_trigger_count_uses_markers_changed_amount_for_each_mode() {
        let trigger = CounterPutOnTrigger::new(ObjectFilter::creature()).count(CountMode::Each);
        let event = TriggerEvent::new(MarkersChangedEvent::added(
            CounterType::MinusOneMinusOne,
            crate::ids::ObjectId::from_raw(1),
            4,
            None,
            None,
        ));
        assert_eq!(trigger.trigger_count(&event), 4);
    }
}
