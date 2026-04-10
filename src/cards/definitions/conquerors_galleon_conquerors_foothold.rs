//! Conqueror's Galleon // Conqueror's Foothold card definition.

use super::CardDefinitionBuilder;
use crate::card::LinkedFaceLayout;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

const GALLEON_ID: u32 = 234_001;
const FOOTHOLD_ID: u32 = 234_002;

/// Conqueror's Galleon // Conqueror's Foothold
pub fn conquerors_galleon() -> CardDefinition {
    CardDefinitionBuilder::new(
        CardId::from_raw(GALLEON_ID),
        "Conqueror's Galleon // Conqueror's Foothold",
    )
    .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
    .card_types(vec![CardType::Artifact])
    .subtypes(vec![Subtype::Vehicle])
    .power_toughness(PowerToughness::fixed(2, 10))
    .other_face(CardId::from_raw(FOOTHOLD_ID))
    .other_face_name("Conqueror's Foothold")
    .linked_face_layout(LinkedFaceLayout::TransformLike)
    .parse_text(
        "When this Vehicle attacks, exile it at end of combat, then return it to the battlefield transformed under your control.\n\
         Crew 4 (Tap any number of creatures you control with total power 4 or more: This Vehicle becomes an artifact creature until end of turn.)",
    )
    .expect("Conqueror's Galleon text should be supported")
}

/// Conqueror's Foothold
pub fn conquerors_foothold() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(FOOTHOLD_ID), "Conqueror's Foothold")
        .card_types(vec![CardType::Land])
        .other_face(CardId::from_raw(GALLEON_ID))
        .other_face_name("Conqueror's Galleon")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .parse_text(
            "{T}: Add {C}.\n\
             {2}, {T}: Draw a card, then discard a card.\n\
             {4}, {T}: Draw a card.\n\
             {6}, {T}: Return target card from your graveyard to your hand.",
        )
        .expect("Conqueror's Foothold text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::events::combat::CreatureAttackedEvent;
    use crate::events::phase::EndOfCombatEvent;
    use crate::ids::CardId;
    use crate::game_loop::{put_triggers_on_stack, resolve_stack_entry};
    use crate::ids::PlayerId;
    use crate::triggers::{AttackEventTarget, TriggerEvent, TriggerQueue, check_triggers};
    use crate::zone::Zone;

    fn setup_game() -> crate::game_state::GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn conquerors_galleon_and_foothold_are_linked_transform_faces() {
        crate::cards::clear_runtime_custom_cards();
        let registry = crate::cards::CardRegistry::with_builtin_cards();
        let galleon = registry
            .get("Conqueror's Galleon // Conqueror's Foothold")
            .expect("galleon should be in builtin registry");
        let foothold = registry
            .get("Conqueror's Foothold")
            .expect("foothold should be in builtin registry");

        assert_eq!(
            galleon.card.other_face_name.as_deref(),
            Some("Conqueror's Foothold")
        );
        assert_eq!(
            foothold.card.other_face_name.as_deref(),
            Some("Conqueror's Galleon")
        );
        assert_eq!(
            galleon.card.other_face,
            Some(CardId::from_raw(FOOTHOLD_ID))
        );
        assert_eq!(
            foothold.card.other_face,
            Some(CardId::from_raw(GALLEON_ID))
        );
        assert_eq!(galleon.card.linked_face_layout, LinkedFaceLayout::TransformLike);
        assert_eq!(foothold.card.linked_face_layout, LinkedFaceLayout::TransformLike);
        assert_eq!(galleon.abilities.len(), 2);
        assert_eq!(foothold.abilities.len(), 4);
    }

    #[test]
    fn conquerors_galleon_transforms_into_foothold_after_combat() {
        crate::cards::clear_runtime_custom_cards();
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let galleon = conquerors_galleon();
        let galleon_id = game.create_object_from_definition(&galleon, alice, Zone::Battlefield);

        assert!(
            game.object(galleon_id)
                .unwrap()
                .abilities
                .iter()
                .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("attacks"))),
            "galleon should have an attack trigger"
        );

        let attack_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(galleon_id, AttackEventTarget::Player(PlayerId::from_index(1))),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in check_triggers(&game, &attack_event) {
            trigger_queue.add(trigger);
        }
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("should queue attack trigger");
        while !game.stack_is_empty() {
            resolve_stack_entry(&mut game).expect("resolve attack trigger");
        }

        assert!(
            game.battlefield.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Conqueror's Galleon // Conqueror's Foothold")
            }),
            "Galleon should stay on the battlefield until end of combat"
        );
        assert_eq!(game.delayed_triggers.len(), 1);

        let end_of_combat_event = TriggerEvent::new_with_provenance(
            EndOfCombatEvent::new(),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_delayed_triggers(&mut game, &end_of_combat_event) {
            trigger_queue.add(trigger);
        }
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("should queue delayed end-of-combat trigger");
        while !game.stack_is_empty() {
            resolve_stack_entry(&mut game).expect("resolve delayed end-of-combat trigger");
        }

        let foothold_id = game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Conqueror's Foothold")
            })
            .expect("foothold should return to the battlefield");
        let foothold = game.object(foothold_id).expect("foothold should exist");
        assert!(game.is_face_down(foothold_id));
        assert_eq!(foothold.card_types, vec![CardType::Land]);
        assert_eq!(foothold.abilities.len(), 4);
    }
}
