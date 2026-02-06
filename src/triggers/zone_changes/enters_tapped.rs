//! "When [filter] enters the battlefield tapped" trigger.

use crate::events::EventKind;
use crate::events::zones::EnterBattlefieldEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching object enters the battlefield tapped.
///
/// This is used by cards like Amulet of Vigor ("Whenever a permanent enters
/// the battlefield tapped and under your control, untap it.").
#[derive(Debug, Clone, PartialEq)]
pub struct EntersBattlefieldTappedTrigger {
    /// Filter for objects that trigger this ability.
    pub filter: ObjectFilter,
}

impl EntersBattlefieldTappedTrigger {
    /// Create a new ETB-tapped trigger with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Create an ETB-tapped trigger for any permanent you control.
    pub fn permanent_you_control() -> Self {
        Self::new(ObjectFilter::permanent().you_control())
    }
}

impl TriggerMatcher for EntersBattlefieldTappedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        // Fast-path: check event kind
        if event.kind() != EventKind::EnterBattlefield {
            return false;
        }

        // Downcast to access enters_tapped field
        let Some(enter_event) = event.downcast::<EnterBattlefieldEvent>() else {
            return false;
        };

        // Must enter tapped
        if !enter_event.enters_tapped {
            return false;
        }

        // Must match the filter
        let Some(object_id) = event.object_id() else {
            return false;
        };

        if let Some(obj) = ctx.game.object(object_id) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} enters the battlefield tapped",
            self.filter.description()
        )
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::EnterBattlefieldEvent;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_land(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Land])
            .build();

        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn test_matches_permanent_entering_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(100);
        let land_id = create_land(&mut game, "Tapped Land", alice);

        let trigger = EntersBattlefieldTappedTrigger::permanent_you_control();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Create an event where the permanent enters tapped
        let event = TriggerEvent::new(EnterBattlefieldEvent::tapped(land_id, Zone::Hand));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_permanent_entering_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(100);
        let land_id = create_land(&mut game, "Untapped Land", alice);

        let trigger = EntersBattlefieldTappedTrigger::permanent_you_control();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Create an event where the permanent enters untapped (default)
        let event = TriggerEvent::new(EnterBattlefieldEvent::new(land_id, Zone::Hand));

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_permanent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let land_id = create_land(&mut game, "Bob's Land", bob);

        let trigger = EntersBattlefieldTappedTrigger::permanent_you_control();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Bob's permanent enters tapped, but Alice's trigger only cares about her permanents
        let event = TriggerEvent::new(EnterBattlefieldEvent::tapped(land_id, Zone::Hand));

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_creature_entering_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(100);
        let creature_id = create_creature(&mut game, "Tapped Creature", alice);

        let trigger = EntersBattlefieldTappedTrigger::permanent_you_control();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(EnterBattlefieldEvent::tapped(creature_id, Zone::Hand));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_non_etb_events() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = EntersBattlefieldTappedTrigger::permanent_you_control();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(alice));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = EntersBattlefieldTappedTrigger::permanent_you_control();
        let display = trigger.display();
        assert!(display.contains("enters the battlefield tapped"));
    }

    #[test]
    fn test_uses_snapshot() {
        let trigger = EntersBattlefieldTappedTrigger::permanent_you_control();
        assert!(!trigger.uses_snapshot());
    }
}
