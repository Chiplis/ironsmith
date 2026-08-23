//! "Whenever a counter is put on [filter]" trigger.

use crate::events::EventKind;
use crate::events::other::{CounterPlacedEvent, MarkersChangedEvent};
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
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
    /// "on a permanent or player" — counters placed on players match too.
    pub include_players: bool,
    /// Match the event that crosses a particular resulting counter number.
    /// For example, adding two counters to a permanent with three counters
    /// crosses both the fourth and fifth counter ordinals.
    pub counter_number: Option<u32>,
}

impl CounterPutOnTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter_type: None,
            source_controller: None,
            count_mode: CountMode::Each,
            include_players: false,
            counter_number: None,
        }
    }

    pub fn include_players(mut self) -> Self {
        self.include_players = true;
        self
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

    pub fn counter_number(mut self, counter_number: u32) -> Self {
        self.counter_number = Some(counter_number);
        self
    }
}

impl TriggerMatcher for CounterPutOnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let (permanent, counter_type, source_controller, amount, count_after) = match event.kind() {
            EventKind::CounterPlaced => {
                let Some(e) = event.downcast::<CounterPlacedEvent>() else {
                    return false;
                };
                (e.permanent, e.counter_type, None, e.amount, None)
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
                if let Some(permanent) = e.object() {
                    (
                        permanent,
                        counter_type,
                        e.source_controller,
                        e.amount,
                        e.count_after,
                    )
                } else if self.include_players && e.player().is_some() {
                    // Player recipient of an "on a permanent or player"
                    // trigger: the object filter does not apply.
                    if let Some(required_source_controller) = &self.source_controller {
                        let Some(source_controller) = e.source_controller else {
                            return false;
                        };
                        if !required_source_controller
                            .matches_player(source_controller, &ctx.filter_ctx)
                        {
                            return false;
                        }
                    }
                    if let Some(required_counter_type) = self.counter_type
                        && counter_type != required_counter_type
                    {
                        return false;
                    }
                    return true;
                } else {
                    return false;
                }
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
        let Some(obj) = ctx.game.object(permanent) else {
            return false;
        };
        if !self.filter.matches(obj, &ctx.filter_ctx, ctx.game) {
            return false;
        }
        if let Some(counter_number) = self.counter_number {
            let count_after =
                count_after.unwrap_or_else(|| ctx.game.counter_count(permanent, counter_type));
            let count_before = count_after.saturating_sub(amount);
            return count_before < counter_number && counter_number <= count_after;
        }
        true
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CounterPlaced, EventKind::MarkersChanged])
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        if self.counter_number.is_some() {
            return 1;
        }
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
                other => other.description().into_owned(),
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

        fn recipient_with_article(description: String) -> String {
            let lower = description.to_ascii_lowercase();
            let has_determiner = [
                "a ",
                "an ",
                "the ",
                "this ",
                "that ",
                "each ",
                "another ",
                "enchanted ",
                "equipped ",
                "target ",
                "one ",
            ]
            .iter()
            .any(|prefix| lower.starts_with(prefix));
            if has_determiner {
                return description;
            }
            let article = if matches!(lower.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u')) {
                "an"
            } else {
                "a"
            };
            format!("{article} {description}")
        }

        let player_suffix = if self.include_players {
            " or player"
        } else {
            ""
        };
        if let Some(counter_number) = self.counter_number {
            let ordinal =
                ironsmith_core::ordinal_word(counter_number).unwrap_or_else(|| "nth".to_string());
            return format!(
                "Whenever the {ordinal} {counters} is put on {}{player_suffix}",
                recipient_with_article(self.filter.description())
            );
        }
        if let Some(source_controller) = &self.source_controller {
            let (subject, verb) = if source_controller == &PlayerFilter::You {
                ("you".to_string(), "put")
            } else {
                (source_controller.description(), "puts")
            };
            return format!(
                "Whenever {subject} {verb} {counter_phrase} on {}{player_suffix}",
                recipient_with_article(self.filter.description())
            );
        }

        match self.count_mode {
            CountMode::OneOrMore => format!(
                "Whenever one or more {}s are put on {}{player_suffix}",
                counters,
                recipient_with_article(self.filter.description())
            ),
            CountMode::Each => format!(
                "Whenever a {} is put on {}{player_suffix}",
                counters,
                recipient_with_article(self.filter.description())
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

        let your_event = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::added(
                CounterType::MinusOneMinusOne,
                creature_id,
                2,
                Some(creature_id),
                Some(alice),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            trigger.matches(&your_event, &ctx),
            "expected trigger to match your -1/-1 counter placement"
        );

        let opponent_event = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::added(
                CounterType::MinusOneMinusOne,
                creature_id,
                2,
                Some(creature_id),
                Some(bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            !trigger.matches(&opponent_event, &ctx),
            "expected trigger to reject opponent counter placement"
        );
    }

    #[test]
    fn test_trigger_count_uses_markers_changed_amount_for_each_mode() {
        let trigger = CounterPutOnTrigger::new(ObjectFilter::creature()).count(CountMode::Each);
        let event = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::added(
                CounterType::MinusOneMinusOne,
                crate::ids::ObjectId::from_raw(1),
                4,
                None,
                None,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert_eq!(trigger.trigger_count(&event), 4);
    }

    #[test]
    fn numbered_counter_trigger_matches_a_batched_crossing_once() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let permanent =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let trigger = CounterPutOnTrigger::new(ObjectFilter::creature())
            .counter_type(CounterType::Charge)
            .counter_number(4);
        let ctx = TriggerContext::for_source(permanent, alice, &game);

        let crossing = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::added(CounterType::Charge, permanent, 2, None, None)
                .with_count_after(5),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&crossing, &ctx));
        assert_eq!(trigger.trigger_count(&crossing), 1);

        let after_threshold = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::added(CounterType::Charge, permanent, 1, None, None)
                .with_count_after(6),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&after_threshold, &ctx));
        assert_eq!(
            trigger.display(),
            "Whenever the fourth charge counter is put on a creature"
        );
    }
}
