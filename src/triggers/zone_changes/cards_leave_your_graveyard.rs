//! "Whenever one or more cards leave your graveyard" trigger.

use crate::events::EventKind;
use crate::events::zones::ZoneChangeEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::types::CardType;
use crate::zone::Zone;

/// Trigger that fires when one or more cards leave your graveyard.
///
/// Used by cards like Willow Geist, Desecrated Tomb, and Thran Vigil.
#[derive(Debug, Clone, PartialEq)]
pub struct CardsLeaveYourGraveyardTrigger {
    /// Filter for cards that cause this trigger to fire.
    pub filter: ObjectFilter,
    /// If true, this triggers once per zone-change batch event ("one or more").
    /// If false, this triggers once per object in the event ("a card").
    pub one_or_more: bool,
    /// If true, only trigger during the controller's turn ("during your turn").
    pub during_your_turn: bool,
}

impl CardsLeaveYourGraveyardTrigger {
    pub fn new(filter: ObjectFilter, one_or_more: bool, during_your_turn: bool) -> Self {
        Self {
            filter,
            one_or_more,
            during_your_turn,
        }
    }

    fn matching_count(&self, zc: &ZoneChangeEvent, ctx: &TriggerContext) -> u32 {
        if zc.from != Zone::Graveyard || zc.to == Zone::Graveyard {
            return 0;
        }

        if self.during_your_turn && ctx.game.turn.active_player != ctx.controller {
            return 0;
        }

        // move_object emits single-object zone changes with a pre-move snapshot.
        if zc.objects.len() == 1
            && let Some(snapshot) = zc.snapshot.as_ref()
        {
            if snapshot.owner != ctx.controller {
                return 0;
            }

            return if self
                .filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
            {
                1
            } else {
                0
            };
        }

        let mut count = 0u32;
        for &object_id in &zc.objects {
            let Some(object) = ctx.game.object(object_id) else {
                continue;
            };
            if object.owner != ctx.controller {
                continue;
            }
            if self.filter.matches(object, &ctx.filter_ctx, ctx.game) {
                count = count.saturating_add(1);
            }
        }
        count
    }

    fn describe_subject(&self, plural: bool) -> String {
        let noun = if plural { "cards" } else { "card" };

        let types = if !self.filter.all_card_types.is_empty() {
            Some(
                self.filter
                    .all_card_types
                    .iter()
                    .map(|t| format!("{t:?}").to_lowercase())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        } else if !self.filter.card_types.is_empty() {
            // Preserve "artifact and/or creature" wording where it is used in Oracle text.
            let has_artifact = self.filter.card_types.contains(&CardType::Artifact);
            let has_creature = self.filter.card_types.contains(&CardType::Creature);
            if self.filter.card_types.len() == 2 && has_artifact && has_creature {
                Some("artifact and/or creature".to_string())
            } else {
                Some(
                    self.filter
                        .card_types
                        .iter()
                        .map(|t| format!("{t:?}").to_lowercase())
                        .collect::<Vec<_>>()
                        .join(" or "),
                )
            }
        } else {
            None
        };

        if let Some(types) = types
            && !types.is_empty()
        {
            format!("{types} {noun}")
        } else {
            noun.to_string()
        }
    }
}

impl TriggerMatcher for CardsLeaveYourGraveyardTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::ZoneChange {
            return false;
        }
        let Some(zc) = event.downcast::<ZoneChangeEvent>() else {
            return false;
        };

        self.matching_count(zc, ctx) > 0
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        if self.one_or_more {
            return 1;
        }
        event
            .downcast::<ZoneChangeEvent>()
            .map(|zc| zc.count() as u32)
            .unwrap_or(1)
    }

    fn display(&self) -> String {
        let subject = self.describe_subject(self.one_or_more);
        let mut text = if self.one_or_more {
            format!("Whenever one or more {subject} leave your graveyard")
        } else {
            format!("Whenever a {subject} leaves your graveyard")
        };
        if self.during_your_turn {
            text.push_str(" during your turn");
        }
        text
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }

    fn uses_snapshot(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn matches_when_card_leaves_your_graveyard() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = CardsLeaveYourGraveyardTrigger::new(ObjectFilter::default(), true, false);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let mut snapshot = ObjectSnapshot::for_testing(ObjectId::from_raw(10), alice, "Card");
        snapshot.zone = Zone::Graveyard;

        let event = TriggerEvent::new(ZoneChangeEvent::new(
            ObjectId::from_raw(20),
            Zone::Graveyard,
            Zone::Exile,
            Some(snapshot),
        ));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_card_leaves_opponents_graveyard() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = CardsLeaveYourGraveyardTrigger::new(ObjectFilter::default(), true, false);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let mut snapshot = ObjectSnapshot::for_testing(ObjectId::from_raw(10), bob, "Card");
        snapshot.zone = Zone::Graveyard;
        snapshot.owner = bob;

        let event = TriggerEvent::new(ZoneChangeEvent::new(
            ObjectId::from_raw(20),
            Zone::Graveyard,
            Zone::Exile,
            Some(snapshot),
        ));

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_outside_your_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        // It's Bob's turn.
        game.turn.active_player = bob;

        let trigger = CardsLeaveYourGraveyardTrigger::new(ObjectFilter::default(), true, true);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let mut snapshot = ObjectSnapshot::for_testing(ObjectId::from_raw(10), alice, "Card");
        snapshot.zone = Zone::Graveyard;

        let event = TriggerEvent::new(ZoneChangeEvent::new(
            ObjectId::from_raw(20),
            Zone::Graveyard,
            Zone::Exile,
            Some(snapshot),
        ));

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn display_formats_one_or_more() {
        let mut filter = ObjectFilter::default();
        filter.card_types.push(CardType::Creature);
        let trigger = CardsLeaveYourGraveyardTrigger::new(filter, true, true);
        assert_eq!(
            trigger.display(),
            "Whenever one or more creature cards leave your graveyard during your turn"
        );
    }
}
