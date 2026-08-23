#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::types::CardType;

fn find_create_token(effect: &crate::effect::Effect) -> Option<CreateTokenEffect> {
    if let Some(create) = effect.downcast_ref::<CreateTokenEffect>() {
        return Some(create.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_create_token(child);
        }
    });
    found
}

fn count_effects<T: 'static>(effect: &crate::effect::Effect) -> usize {
    let mut count = usize::from(effect.downcast_ref::<T>().is_some());
    effect.visit_child_effects(&mut |child| count += count_effects::<T>(child));
    count
}

#[test]
fn thousand_moons_smithy_compiles_to_its_exact_front_face_oracle_text() {
    let definition = parse_oracle_card_definition("Thousand Moons Smithy");

    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "When Thousand Moons Smithy enters, create a white Gnome Soldier artifact creature token with \"This token's power and toughness are each equal to the number of artifacts and/or creatures you control.\"".to_string(),
            "At the beginning of your first main phase, you may tap five untapped artifacts and/or creatures you control. If you do, transform Thousand Moons Smithy.".to_string(),
        ],
    );
}

#[test]
fn barracks_of_the_thousand_compiles_to_its_exact_back_face_oracle_text() {
    let definition = parse_oracle_card_definition("Barracks of the Thousand");

    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "{T}: Add {W}.".to_string(),
            "Whenever you cast an artifact or creature spell using mana produced by Barracks of the Thousand, create a white Gnome Soldier artifact creature token with \"This token's power and toughness are each equal to the number of artifacts and/or creatures you control.\"".to_string(),
        ],
    );
}

#[test]
fn barracks_cast_trigger_keeps_typed_mana_source_provenance() {
    let definition = parse_oracle_card_definition("Barracks of the Thousand");
    let (triggered, spell_cast) = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>()
                .map(|spell_cast| (triggered, spell_cast)),
            _ => None,
        })
        .expect("Barracks should retain its spell-cast trigger");
    assert_eq!(
        triggered.intervening_if, None,
        "cast-payment provenance is an event matcher, not an intervening-if condition"
    );
    let source_filter = spell_cast
        .mana_source_filter
        .as_ref()
        .expect("Barracks should retain a typed mana-source restriction");

    assert!(source_filter.source, "{source_filter:#?}");
    assert_eq!(
        source_filter.source_surface,
        Some(SourceReferenceSurface::ShortName(
            "Barracks of the Thousand".to_string()
        )),
        "{source_filter:#?}"
    );
}

#[test]
fn both_smithy_faces_create_tokens_with_one_typed_union_count_cda() {
    for name in ["Thousand Moons Smithy", "Barracks of the Thousand"] {
        let definition = parse_oracle_card_definition(name);
        let triggered = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => triggered
                    .effects
                    .segments
                    .iter()
                    .flat_map(|segment| &segment.default_effects)
                    .find_map(find_create_token)
                    .map(|create| (triggered, create)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name} should create its Gnome token"));
        let (triggered, create) = triggered;

        assert_eq!(
            triggered
                .effects
                .segments
                .iter()
                .flat_map(|segment| &segment.default_effects)
                .map(count_effects::<crate::effects::SetBasePowerToughnessEffect>)
                .sum::<usize>(),
            0,
            "{name}: a quoted token CDA must not also lower as an outer base-P/T effect"
        );

        let characteristic_abilities = create
            .token
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT =>
                {
                    Some(static_ability)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            characteristic_abilities.len(),
            1,
            "{name}: {:#?}",
            create.token.abilities
        );

        let game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
        let effects = characteristic_abilities[0].generate_effects(
            crate::ids::ObjectId::from_raw(1),
            crate::ids::PlayerId::from_index(0),
            &game,
        );
        let crate::continuous::Modification::SetPowerToughness {
            power, toughness, ..
        } = &effects[0].modification
        else {
            panic!("{name}: expected the token CDA to set power and toughness");
        };
        assert_eq!(power, toughness, "{name}");
        let crate::effect::Value::Count(filter) = power.unhinted() else {
            panic!("{name}: expected the token CDA to count a typed permanent set: {power:#?}");
        };
        assert_eq!(
            filter.card_types,
            [CardType::Artifact, CardType::Creature],
            "{name}"
        );
        assert_eq!(filter.controller, Some(PlayerFilter::You), "{name}");
        assert_eq!(
            filter.union_connective(),
            ironsmith_core::ObjectFilterUnionConnective::AndOr,
            "{name}"
        );
    }
}
