use crate::events::EventKind;
use crate::events::other::MarkersChangedEvent;
use crate::filter::PlayerFilterExt;
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{CountMode, TriggerEvent};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerGetsCountersTrigger {
    pub player: PlayerFilter,
    pub counter_type: Option<CounterType>,
    pub count_mode: CountMode,
}

impl PlayerGetsCountersTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            counter_type: None,
            count_mode: CountMode::Each,
        }
    }

    pub fn counter_type(mut self, counter_type: CounterType) -> Self {
        self.counter_type = Some(counter_type);
        self
    }

    pub fn count(mut self, count_mode: CountMode) -> Self {
        self.count_mode = count_mode;
        self
    }
}

impl TriggerMatcher for PlayerGetsCountersTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::MarkersChanged {
            return false;
        }
        let Some(e) = event.downcast::<MarkersChangedEvent>() else {
            return false;
        };
        if !e.is_added() {
            return false;
        }
        let Some(counter_type) = e.marker.as_counter() else {
            return false;
        };
        let Some(player_id) = e.player() else {
            return false;
        };
        if !self.player.matches_player(player_id, &ctx.filter_ctx) {
            return false;
        }
        if let Some(required) = self.counter_type
            && required != counter_type
        {
            return false;
        }
        true
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        match self.count_mode {
            CountMode::OneOrMore => 1,
            CountMode::Each => event
                .downcast::<MarkersChangedEvent>()
                .map_or(1, |e| e.amount.max(1)),
        }
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::MarkersChanged])
    }

    fn display(&self) -> String {
        let player = self.player.description();
        let verb = if player.eq_ignore_ascii_case("you") {
            "get"
        } else {
            "gets"
        };
        if self.counter_type == Some(CounterType::Energy) {
            let energy = match self.count_mode {
                CountMode::OneOrMore => "one or more {E}",
                CountMode::Each => "{E}",
            };
            return format!("Whenever {player} {verb} {energy}");
        }
        let counter_phrase = match self.counter_type {
            Some(counter_type) => match self.count_mode {
                CountMode::OneOrMore => {
                    format!("one or more {} counters", counter_type.description())
                }
                CountMode::Each => format!("a {} counter", counter_type.description()),
            },
            None => match self.count_mode {
                CountMode::OneOrMore => "one or more counters".to_string(),
                CountMode::Each => "a counter".to_string(),
            },
        };
        format!("Whenever {player} {verb} {counter_phrase}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::definitions::grizzly_bears;
    use crate::events::other::{MarkerChangeType, MarkersChangedEvent};
    use crate::ids::PlayerId;
    use crate::provenance::ProvNodeId;
    use crate::triggers::matcher_trait::TriggerContext;
    use crate::zone::Zone;

    #[test]
    fn matches_only_added_counters_for_matching_player_and_type() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let mut game = game;
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let trigger = PlayerGetsCountersTrigger::new(PlayerFilter::You)
            .counter_type(CounterType::Energy)
            .count(CountMode::OneOrMore);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let added_for_you = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::added(CounterType::Energy, alice, 2, None, None),
            ProvNodeId::default(),
        );
        assert!(trigger.matches(&added_for_you, &ctx));

        let added_for_opponent = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::added(CounterType::Energy, bob, 2, None, None),
            ProvNodeId::default(),
        );
        assert!(!trigger.matches(&added_for_opponent, &ctx));

        let removed_for_you = TriggerEvent::new_with_provenance(
            MarkersChangedEvent {
                change_type: MarkerChangeType::Removed,
                marker: CounterType::Energy.into(),
                location: alice.into(),
                amount: 1,
                source: None,
                source_controller: None,
            },
            ProvNodeId::default(),
        );
        assert!(!trigger.matches(&removed_for_you, &ctx));
    }

    #[test]
    fn each_mode_uses_added_amount() {
        let trigger = PlayerGetsCountersTrigger::new(PlayerFilter::You).count(CountMode::Each);
        let event = TriggerEvent::new_with_provenance(
            MarkersChangedEvent::added(CounterType::Energy, PlayerId::from_index(0), 3, None, None),
            ProvNodeId::default(),
        );
        assert_eq!(trigger.trigger_count(&event), 3);
    }

    #[test]
    fn display_uses_you_get_not_you_gets() {
        let trigger = PlayerGetsCountersTrigger::new(PlayerFilter::You)
            .counter_type(CounterType::Energy)
            .count(CountMode::OneOrMore);
        assert_eq!(trigger.display(), "Whenever you get one or more {E}");
    }
}
