use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::color::ColorSet;
use crate::effects::ExecutionContext;
use crate::ids::{CardId, PlayerId};
use crate::mana::{ManaCost, ManaSymbol};
use ironsmith_core::effect::{
    BattlefieldEntryColorChange as ColorChange, BattlefieldEntryTypeChanges,
};

#[test]
fn entry_colors_preserve_instruction_order_and_expire_on_blink() {
    for (changes, expected) in [
        (
            vec![ColorChange::Add(ColorSet::BLACK)],
            ColorSet::WHITE.union(ColorSet::BLACK),
        ),
        (vec![ColorChange::Set(ColorSet::BLACK)], ColorSet::BLACK),
        (
            vec![
                ColorChange::Add(ColorSet::RED),
                ColorChange::Set(ColorSet::BLUE),
            ],
            ColorSet::BLUE,
        ),
        (
            vec![
                ColorChange::Set(ColorSet::BLUE),
                ColorChange::Add(ColorSet::RED),
            ],
            ColorSet::BLUE.union(ColorSet::RED),
        ),
    ] {
        let player = PlayerId::from_index(0);
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let definition = CardBuilder::new(CardId::new(), "Entry color subject")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::White]))
            .power_toughness(PowerToughness::fixed(2, 3))
            .build();
        let old = game.create_object_from_card(&definition, player, Zone::Graveyard);
        let stable = game.object(old).unwrap().stable_id;
        let mut movement =
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(old), Zone::Battlefield, false);
        movement.entry_type_changes = Some(BattlefieldEntryTypeChanges {
            color_changes: changes,
            ..Default::default()
        });
        let serializable = movement.clone().try_map_entry_modifications(|_| -> Result<(), ()> { unreachable!() }).unwrap();
        let wire = serde_json::to_value(&serializable).unwrap();
        let restored: ironsmith_core::MoveToZoneEffect = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(restored, serializable);
        let mut legacy = wire;
        legacy["entry_type_changes"]
            .as_object_mut()
            .unwrap()
            .remove("color_changes");
        let legacy: ironsmith_core::MoveToZoneEffect = serde_json::from_value(legacy).unwrap();
        assert!(legacy.entry_type_changes.unwrap().color_changes.is_empty());
        restored.try_map_entry_modifications(|_| -> Result<crate::continuous::Modification, ()> { unreachable!() }).unwrap()
            .execute(&mut game, &mut ExecutionContext::new_default(old, player))
            .unwrap();
        let arrived = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.current_colors(arrived), Some(expected));
        let exiled = game.move_object_by_effect(arrived, Zone::Exile).unwrap();
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(exiled), Zone::Battlefield, false)
            .execute(
                &mut game,
                &mut ExecutionContext::new_default(exiled, player),
            )
            .unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(returned, arrived);
        assert_eq!(game.current_colors(returned), Some(ColorSet::WHITE));
    }
}
