//! "Whenever [filter] becomes blocked" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBecameBlockedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesBlockedTrigger {
    pub filter: ObjectFilter,
    pub one_or_more: bool,
}

impl BecomesBlockedTrigger {
    pub fn one_or_more(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: true,
        }
    }
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: false,
        }
    }
}

impl TriggerMatcher for BecomesBlockedTrigger {
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureBecameBlocked])
    }

    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBecameBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBecameBlockedEvent>() else {
            return false;
        };
        if let Some(snapshot) = e
            .attacker_snapshot
            .as_ref()
            .filter(|snapshot| snapshot.object_id == e.attacker)
        {
            return self
                .filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }
        if let Some(obj) = ctx.game.object(e.attacker) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn simultaneous_trigger_key(
        &self,
        event: &TriggerEvent,
    ) -> Option<crate::triggers::matcher_trait::SimultaneousTriggerKey> {
        (self.one_or_more && event.kind() == EventKind::CreatureBecameBlocked)
            .then_some(crate::triggers::matcher_trait::SimultaneousTriggerKey::BecomesBlockedBatch)
    }
    fn display(&self) -> String {
        if self.one_or_more {
            format!(
                "Whenever one or more {} become blocked",
                super::pluralize_one_or_more_attack_subject(&self.filter.description())
            )
        } else {
            format!("Whenever {} becomes blocked", self.filter.description())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = BecomesBlockedTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("becomes blocked"));
    }
}

#[cfg(test)]
mod retained_participant_tests {
    use super::*;
    use crate::ids::PlayerId;
    #[test]
    fn blocked_subject_uses_its_exact_declaration_snapshot_after_control_changes() {
        let a = PlayerId(0);
        let b = PlayerId(1);
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Attacker")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let attacker = game.create_object_from_card(&card, a, crate::zone::Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(attacker).unwrap(),
            &game,
        );
        let event = TriggerEvent::new(
            CreatureBecameBlockedEvent::with_target_and_blockers(
                attacker,
                vec![],
                Some(crate::triggers::AttackEventTarget::Player(b)),
                Some(snapshot),
                vec![],
            ),
            Default::default(),
        );
        game.set_current_controller(attacker, b);
        let matcher = BecomesBlockedTrigger::one_or_more(ObjectFilter::creature().you_control());
        assert!(matcher.matches(&event, &TriggerContext::for_source(attacker, a, &game)));
        assert!(matcher.simultaneous_trigger_key(&event).is_some());
        assert_eq!(
            matcher.subscribed_kinds(),
            Some(vec![EventKind::CreatureBecameBlocked])
        );
    }
}
