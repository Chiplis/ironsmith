//! Passive permanent sacrifice and successful-destruction triggers.

use crate::events::EventKind;
use crate::events::permanents::{DestroyEvent, SacrificeEvent};
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::zone::Zone;

fn passive_subject(filter: &ObjectFilter) -> String {
    let text = filter.description();
    if text.starts_with("a ") || text.starts_with("an ") {
        text
    } else {
        let article = if matches!(
            text.chars().next().map(|ch| ch.to_ascii_lowercase()),
            Some('a' | 'e' | 'i' | 'o' | 'u')
        ) {
            "an"
        } else {
            "a"
        };
        format!("{article} {text}")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentSacrificedTrigger {
    pub filter: ObjectFilter,
}

impl TriggerMatcher for PermanentSacrificedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(sacrifice) = event.downcast::<SacrificeEvent>() else {
            return false;
        };
        sacrifice.snapshot.as_ref().is_some_and(|snapshot| {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Sacrifice])
    }

    fn uses_snapshot(&self) -> bool {
        true
    }

    fn display(&self) -> String {
        format!("Whenever {} is sacrificed", passive_subject(&self.filter))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentDestroyedTrigger {
    pub filter: ObjectFilter,
}

impl TriggerMatcher for PermanentDestroyedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(destroy) = event.downcast::<DestroyEvent>() else {
            return false;
        };
        // Replacing the graveyard destination does not undo destruction.
        // A prospective/prevented event has no successful nonbattlefield result.
        if destroy.final_zone.is_none_or(|zone| zone == Zone::Battlefield) {
            return false;
        }
        destroy.snapshot.as_ref().is_some_and(|snapshot| {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Destroy])
    }

    fn uses_snapshot(&self) -> bool {
        true
    }

    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        event.kind() == EventKind::Destroy
    }

    fn display(&self) -> String {
        format!("Whenever {} is destroyed", passive_subject(&self.filter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    #[test]
    fn successful_destruction_accepts_replaced_destination_but_not_prospective_or_prevented() {
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let a = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Destroyed artifact").card_types(vec![crate::types::CardType::Artifact]).build();
        let id = game.create_object_from_card(&card, a, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(id).unwrap(), &game);
        let trigger = PermanentDestroyedTrigger { filter: ObjectFilter::default().in_zone(Zone::Battlefield).with_type(crate::types::CardType::Artifact) };
        for (zone, expected) in [(None,false),(Some(Zone::Battlefield),false),(Some(Zone::Graveyard),true),(Some(Zone::Exile),true)] {
            let mut destroyed = DestroyEvent::new(id, None); destroyed.snapshot = Some(snapshot.clone()); destroyed.final_zone = zone;
            let event = TriggerEvent::new_with_provenance(destroyed, Default::default());
            assert_eq!(trigger.matches(&event, &TriggerContext::for_source(id,a,&game)),expected);
        }
    }
}
