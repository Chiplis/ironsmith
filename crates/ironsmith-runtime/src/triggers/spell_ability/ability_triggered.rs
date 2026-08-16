//! "Whenever another ability triggers" trigger.

use crate::events::EventKind;
use crate::events::spells::AbilityTriggeredEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityTriggeredTrigger {
    pub another: bool,
    pub source_filter: Option<ObjectFilter>,
    pub caused_by_source_entering: bool,
}

impl AbilityTriggeredTrigger {
    pub fn new(another: bool) -> Self {
        Self::new_qualified(another, None, false)
    }

    pub fn new_qualified(
        another: bool,
        source_filter: Option<ObjectFilter>,
        caused_by_source_entering: bool,
    ) -> Self {
        Self {
            another,
            source_filter,
            caused_by_source_entering,
        }
    }
}

impl TriggerMatcher for AbilityTriggeredTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::AbilityTriggered {
            return false;
        }
        let Some(event) = event.downcast::<AbilityTriggeredEvent>() else {
            return false;
        };

        let same_source = ctx
            .game
            .object(ctx.source_id)
            .is_some_and(|source| source.stable_id == event.source_stable_id)
            || crate::ids::StableId::from(ctx.source_id) == event.source_stable_id;
        if self.another && same_source && ctx.trigger_identity == Some(event.trigger_identity) {
            return false;
        }
        if let Some(filter) = &self.source_filter {
            let source_matches = ctx
                .game
                .object(event.source)
                .is_some_and(|source| filter.matches(source, &ctx.filter_ctx, ctx.game))
                || event.source_snapshot.as_ref().is_some_and(|snapshot| {
                    filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
                });
            if !source_matches {
                return false;
            }
        }
        if self.caused_by_source_entering
            && !event.zone_change_cause.as_ref().is_some_and(|cause| {
                cause.to == crate::zone::Zone::Battlefield
                    && cause.destination_objects.contains(&event.source)
            })
        {
            return false;
        }
        true
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::AbilityTriggered])
    }

    fn display(&self) -> String {
        if self.caused_by_source_entering {
            let source = self
                .source_filter
                .as_ref()
                .map(|filter| {
                    let mut semantic = filter.clone();
                    semantic.controller = None;
                    semantic.zone = None;
                    let description = semantic.description();
                    description
                        .strip_prefix("a ")
                        .or_else(|| description.strip_prefix("an "))
                        .unwrap_or(&description)
                        .to_string()
                })
                .unwrap_or_else(|| "permanent".to_string());
            let controller = self
                .source_filter
                .as_ref()
                .and_then(|filter| filter.controller.as_ref())
                .map(|controller| controller.description())
                .unwrap_or_else(|| "a player".to_string());
            return format!(
                "Whenever a {source} entering under {controller}'s control causes a triggered ability of that {source} to trigger"
            );
        }
        if self.another {
            "Whenever another ability triggers".to_string()
        } else {
            "Whenever an ability triggers".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::events::{AbilityTriggerZoneChangeCause, RawEvent};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId, StableId};
    use crate::provenance::ProvNodeId;
    use crate::triggers::TriggerIdentity;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn another_ability_excludes_only_the_same_trigger_identity() {
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = ObjectId::from_raw(41);
        let controller = PlayerId::from_index(0);
        let event = RawEvent::new(
            AbilityTriggeredEvent::new(
                source,
                StableId::from(source),
                controller,
                TriggerIdentity(7),
            ),
            ProvNodeId::default(),
        );
        let same = TriggerContext::for_source(source, controller, &game)
            .with_trigger_identity(TriggerIdentity(7));
        let different = TriggerContext::for_source(source, controller, &game)
            .with_trigger_identity(TriggerIdentity(8));
        let same_identity_different_source =
            TriggerContext::for_source(ObjectId::from_raw(42), controller, &game)
                .with_trigger_identity(TriggerIdentity(7));
        let matcher = AbilityTriggeredTrigger::new(true);

        assert!(!matcher.matches(&event, &same));
        assert!(matcher.matches(&event, &different));
        assert!(matcher.matches(&event, &same_identity_different_source));
    }

    #[test]
    fn source_entry_qualification_rejects_the_same_ability_from_a_non_entry_cause() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let watcher = CardDefinitionBuilder::new(CardId::from_raw(91), "Watcher")
            .card_types(vec![CardType::Creature])
            .build();
        let entrant = CardDefinitionBuilder::new(CardId::from_raw(92), "Entrant")
            .card_types(vec![CardType::Creature])
            .build();
        let watcher_id = game.create_object_from_definition(&watcher, alice, Zone::Battlefield);
        let entrant_id = game.create_object_from_definition(&entrant, bob, Zone::Battlefield);
        let entrant_stable = game.object(entrant_id).expect("entrant").stable_id;
        let matcher = AbilityTriggeredTrigger::new_qualified(
            false,
            Some(ObjectFilter::creature().opponent_controls()),
            true,
        );
        let ctx = TriggerContext::for_source(watcher_id, alice, &game);

        let etb = RawEvent::new(
            AbilityTriggeredEvent::new(entrant_id, entrant_stable, bob, TriggerIdentity(7))
                .with_cause(
                    EventKind::ZoneChange,
                    Some(entrant_id),
                    Some(AbilityTriggerZoneChangeCause {
                        from: Zone::Hand,
                        to: Zone::Battlefield,
                        destination_objects: vec![entrant_id],
                    }),
                ),
            ProvNodeId::default(),
        );
        let non_etb = RawEvent::new(
            AbilityTriggeredEvent::new(entrant_id, entrant_stable, bob, TriggerIdentity(7))
                .with_cause(EventKind::Damage, Some(entrant_id), None),
            ProvNodeId::default(),
        );

        assert!(matcher.matches(&etb, &ctx));
        assert!(!matcher.matches(&non_etb, &ctx));
    }
}
