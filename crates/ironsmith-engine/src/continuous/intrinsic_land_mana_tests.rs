//! Rule-origin contracts. Authored under a source-only verification gate.
use super::*;
use crate::card::CardBuilder;
use crate::cards::definitions::{basic_forest, basic_island, basic_mountain, basic_plains, basic_swamp};
use crate::game_state::GameState;
use crate::ids::{CardId, PlayerId};

fn game() -> GameState {
    GameState::new(vec!["A".into(), "B".into()], 20)
}

#[test]
fn native_basics_have_no_copiable_mana_text_and_one_current_rule_occurrence() {
    for (definition, subtype) in [
        (basic_plains(), Subtype::Plains), (basic_island(), Subtype::Island),
        (basic_swamp(), Subtype::Swamp), (basic_mountain(), Subtype::Mountain),
        (basic_forest(), Subtype::Forest),
    ] {
        assert!(definition.abilities.is_empty());
        let mut game = game();
        let id = game.create_object_from_definition(&definition, PlayerId::from_index(0), Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let chars = game.calculated_characteristics(id).unwrap();
        assert_eq!(chars.abilities.as_slice(), &[Ability::basic_land_mana(subtype).unwrap()]);
        assert_eq!(chars.abilities.origin(0), Some(&AbilityOrigin::IntrinsicBasicLandMana(subtype)));
        assert!(crate::snapshot::CopiableValues::from_object(game.object(id).unwrap()).abilities.is_empty());
        assert_eq!(game.current_ability(id, 0), Some(chars.abilities[0].clone()));
    }
}

#[test]
fn equal_printed_and_rule_mana_keep_distinct_origins_and_dispatch_indices() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let printed = Ability::basic_land_mana(Subtype::Forest).unwrap();
    let mut definition = basic_forest();
    definition.abilities.push(printed.clone());
    let id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let chars = game.calculated_characteristics(id).unwrap();
    assert_eq!(chars.abilities.as_slice(), &[printed.clone(), printed.clone()]);
    assert_eq!(chars.abilities.origin(0), Some(&AbilityOrigin::Printed(0)));
    assert_eq!(chars.abilities.origin(1), Some(&AbilityOrigin::IntrinsicBasicLandMana(Subtype::Forest)));
    let view = crate::derived_view::DerivedGameView::new(&game);
    assert_eq!(view.abilities_rc(id).unwrap().as_slice(), chars.abilities.as_slice());
    let sparse = unmodified_ability_occurrences(game.object(id).unwrap(), game.turn.turn_number);
    assert_eq!(sparse.as_slice(), chars.abilities.as_slice());
    for index in 0..2 { assert_eq!(game.current_ability(id, index), Some(printed.clone())); }
    let mut resupplied = chars.clone();
    add_intrinsic_basic_land_mana_abilities(&mut resupplied);
    assert_eq!(resupplied.abilities.len(), 2, "the same rule origin is idempotent");
}

#[test]
fn copied_forest_text_changes_to_island_without_copying_old_rule_mana_or_losing_grants() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let donor = game.create_object_from_definition(&basic_forest(), alice, Zone::Battlefield);
    let recipient = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Copy recipient")
        .card_types(vec![CardType::Artifact]).build(), alice, Zone::Battlefield);
    let values = crate::snapshot::CopiableValues::from_object(game.object(donor).unwrap());
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(donor, alice,
        EffectTarget::Specific(recipient), Modification::CopyOf {
            target_id: donor, copiable_values: Box::new(values), preserve_source_abilities: false,
            name_override: None, name_override_surface: None, add_supertypes: vec![],
        }));
    let green = Ability::basic_land_mana(Subtype::Forest).unwrap();
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(donor, alice,
        EffectTarget::Specific(recipient), Modification::AddAbilityGeneric(green.clone())));
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(donor, alice,
        vec![recipient], Modification::RewriteText(
            ironsmith_core::TextChange::basic_land_type(Subtype::Forest, Subtype::Island).unwrap())));
    game.refresh_continuous_state().unwrap();
    let chars = game.calculated_characteristics(recipient).unwrap();
    assert_eq!(chars.subtypes.as_slice(), &[Subtype::Island]);
    assert_eq!(chars.abilities.len(), 2);
    assert_eq!(chars.abilities.origin(0), Some(&AbilityOrigin::IntrinsicBasicLandMana(Subtype::Island)));
    assert_eq!(chars.abilities[0], Ability::basic_land_mana(Subtype::Island).unwrap());
    assert_eq!(chars.abilities[1], green);
    assert!(matches!(chars.abilities.origin(1), Some(AbilityOrigin::Effect { .. })));
    let copied = copiable_values_with_effects(recipient, game.objects_map(),
        game.effect_store.continuous_effects.effects(), &game.battlefield, game.commander_objects(), &game).unwrap();
    assert_eq!(copied.subtypes, vec![Subtype::Forest]);
    assert!(copied.abilities.is_empty(), "layer-three types and layer-six grants are not copiable text");
}

#[test]
fn text_box_exchange_does_not_transfer_basic_land_rule_mana() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let donor = game.create_object_from_definition(&basic_forest(), alice, Zone::Battlefield);
    let recipient = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Text recipient")
        .card_types(vec![CardType::Land]).subtypes(vec![Subtype::Island]).build(), alice, Zone::Battlefield);
    let printed = game.object(donor).unwrap().abilities_vec();
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(donor, alice,
        EffectTarget::Specific(recipient), Modification::SetTextBox(TextBoxOverlay::new(String::new(), printed))));
    game.refresh_continuous_state().unwrap();
    let chars = game.calculated_characteristics(recipient).unwrap();
    assert_eq!(chars.abilities.as_slice(), &[Ability::basic_land_mana(Subtype::Island).unwrap()]);
    assert_eq!(chars.abilities.origin(0), Some(&AbilityOrigin::IntrinsicBasicLandMana(Subtype::Island)));
}
