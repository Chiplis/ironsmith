//! Source attack trigger with an event-time controller-board qualification.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksWhileYouControlTrigger {
    pub filter: ObjectFilter,
}

fn with_indefinite_article(text: String) -> String {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
        || lower.starts_with("any ")
    {
        return trimmed.to_string();
    }
    let article = if trimmed
        .chars()
        .next()
        .is_some_and(|first| matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

impl ThisAttacksWhileYouControlTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for ThisAttacksWhileYouControlTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(event) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        event.attacker == ctx.source_id
            && ctx.game.battlefield.iter().any(|object_id| {
                ctx.game.object(*object_id).is_some_and(|object| {
                    object.zone == Zone::Battlefield
                        && ctx.game.controller_of(object) == ctx.controller
                        && self.filter.matches(object, &ctx.filter_ctx, ctx.game)
                })
            })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn source_must_match_event_object(&self, event_kind: EventKind) -> bool {
        event_kind == EventKind::CreatureAttacked
    }

    fn display(&self) -> String {
        let mut filter = self.filter.clone();
        if filter.controller == Some(PlayerFilter::You) {
            filter.controller = None;
        }
        format!(
            "Whenever this creature attacks while you control {}",
            with_indefinite_article(filter.description())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::combat::AttackEventTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::provenance::ProvNodeId;
    use crate::types::CardType;

    #[test]
    fn event_time_control_qualification_is_checked_when_attack_occurs() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let wargling = CardBuilder::new(CardId::from_raw(60_140), "Wargling")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let ferocious = CardBuilder::new(CardId::from_raw(60_141), "Ferocious Friend")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let source = game.create_object_from_card(&wargling, alice, Zone::Battlefield);
        let friend = game.create_object_from_card(&ferocious, alice, Zone::Battlefield);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(source, AttackEventTarget::Player(bob)),
            ProvNodeId::default(),
        );
        let trigger = ThisAttacksWhileYouControlTrigger::new(
            ObjectFilter::creature()
                .with_power(ironsmith_core::FilterComparison::GreaterThanOrEqual(4)),
        );

        assert!(trigger.matches(&event, &TriggerContext::for_source(source, alice, &game)));
        game.move_object_by_effect(friend, Zone::Graveyard);
        assert!(!trigger.matches(&event, &TriggerContext::for_source(source, alice, &game)));
    }
}
