//! "Whenever a card is put into your graveyard" trigger.

use crate::events::EventKind;
use crate::events::zones::ZoneChangeEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CardPutIntoYourGraveyardTrigger;

impl TriggerMatcher for CardPutIntoYourGraveyardTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::ZoneChange {
            return false;
        }
        let Some(zc) = event.downcast::<ZoneChangeEvent>() else {
            return false;
        };
        if zc.to != crate::zone::Zone::Graveyard {
            return false;
        }

        zc.destination_objects().iter().any(|&id| {
            ctx.game
                .object(id)
                .is_some_and(|object| object.owner == ctx.controller)
        })
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        let Some(zc) = event.downcast::<ZoneChangeEvent>() else {
            return 1;
        };
        zc.destination_objects().len() as u32
    }

    fn display(&self) -> String {
        "Whenever a card is put into your graveyard from anywhere".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_display() {
        let trigger = CardPutIntoYourGraveyardTrigger;
        assert!(trigger.display().contains("graveyard"));
    }

    #[test]
    fn test_meld_split_counts_two_cards_put_into_your_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(777);

        let make_card = |name: &str, id: u32| {
            CardBuilder::new(CardId::from_raw(id), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build()
        };
        let first = game.new_object_id();
        game.add_object(Object::from_card(
            first,
            &make_card("Graf Rats", first.0 as u32),
            alice,
            Zone::Graveyard,
        ));
        let second = game.new_object_id();
        game.add_object(Object::from_card(
            second,
            &make_card("Midnight Scavengers", second.0 as u32),
            alice,
            Zone::Graveyard,
        ));

        let event = TriggerEvent::new_with_provenance(
            ZoneChangeEvent::with_results(
                ObjectId::from_raw(999),
                vec![first, second],
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(crate::snapshot::ObjectSnapshot::for_testing(
                    ObjectId::from_raw(999),
                    alice,
                    "Chittering Host",
                )),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let trigger = CardPutIntoYourGraveyardTrigger;

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.trigger_count(&event), 2);
    }
}
