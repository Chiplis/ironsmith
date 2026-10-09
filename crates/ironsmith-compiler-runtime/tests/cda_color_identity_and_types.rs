use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
use ironsmith::{ColorSet, GameState, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

fn definitions(text: &str) -> [ironsmith::cards::CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition("CDA Probe", text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let direct = direct.unwrap();
    let (artifact, _) = compile_to_artifact("CDA Probe", text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}

const ZONES: [Zone; 9] = [
    Zone::Battlefield,
    Zone::Hand,
    Zone::Library,
    Zone::Stack,
    Zone::Graveyard,
    Zone::Exile,
    Zone::Command,
    Zone::Ante,
    Zone::OutsideGame,
];

#[test]
fn color_identity_exception_preserves_all_zone_color_and_other_identity_sources() {
    let all: ColorSet = ironsmith::color::Color::ALL.into_iter().collect();
    for (cost, identity) in [("{3}", ColorSet::COLORLESS), ("{2}{R}", ColorSet::RED)] {
        let text = format!(
            "Mana cost: {cost}\nType: Creature — Shapeshifter\nPower/Toughness: 2/2\nCDA Probe is all colors. This ability doesn't affect its color identity."
        );
        for definition in definitions(&text) {
            assert_eq!(definition.card.color_identity(), identity);
            let rendered =
                ironsmith_text::compiled_text::compiled_text_lines(&definition).join("\n");
            assert!(
                rendered.contains("This ability doesn't affect its color identity"),
                "{rendered}"
            );
            for zone in ZONES {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let id =
                    game.create_object_from_definition(&definition, PlayerId::from_index(0), zone);
                assert_eq!(game.current_colors(id), Some(all), "{zone:?}");
                assert_eq!(game.object(id).unwrap().color_identity(), identity);
            }
            // Reparse the generated prose without borrowing the original ability model.
            let reparsed = definitions(&format!(
                "Mana cost: {cost}\nType: Creature — Shapeshifter\nPower/Toughness: 2/2\n{rendered}"
            ));
            assert_eq!(reparsed[0].card.color_identity(), identity);
        }
    }
    let ordinary = definitions(
        "Mana cost: {3}\nType: Creature — Shapeshifter\nPower/Toughness: 2/2\nCDA Probe is all colors.",
    );
    assert_eq!(ordinary[0].card.color_identity(), all);
}

#[test]
fn every_creature_type_functions_in_all_zones_and_obeys_layer_four_order() {
    for definition in definitions(
        "Mana cost: {3}{U}\nType: Creature — Illusion\nPower/Toughness: 3/3\nCDA Probe is every creature type (even if this card isn't on the battlefield).",
    ) {
        for zone in ZONES {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let id = game.create_object_from_definition(&definition, PlayerId::from_index(0), zone);
            let subtypes = game.current_subtypes(id).unwrap();
            for subtype in Subtype::all_creature_types() {
                assert!(
                    subtypes.contains(subtype),
                    "missing {subtype:?} in {zone:?}"
                );
            }
            assert!(!subtypes.contains(&Subtype::Forest));
        }
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let owner = PlayerId::from_index(0);
        let painter = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                painter,
                owner,
                EffectTarget::AllPermanents,
                Modification::SetSubtypes(vec![Subtype::Zombie]),
            ));
        let id = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.current_subtypes(id).unwrap(),
            vec![Subtype::Zombie],
            "CDA precedes an older ordinary setter"
        );
    }
}

#[test]
fn native_construction_preserves_exception_and_subtype_family_scope() {
    use ironsmith::ability::Ability;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::static_abilities::{SetColorsForFilter, StaticAbility};
    use ironsmith::{CardId, ObjectFilter};
    let mut colors = SetColorsForFilter::new(ObjectFilter::source(), ColorSet::GREEN);
    colors.exclude_from_color_identity = true;
    let color_ability = StaticAbility::new(colors);
    assert_eq!(
        color_ability.characteristic_defining_colors(),
        Some(ColorSet::GREEN)
    );
    let definition = CardDefinitionBuilder::new(CardId::new(), "Native CDA")
        .card_types(vec![ironsmith::CardType::Creature])
        .with_ability(Ability::static_ability(color_ability))
        .with_ability(Ability::static_ability(
            StaticAbility::add_all_subtypes_of_family(
                ObjectFilter::source(),
                ironsmith::types::SubtypeFamily::Creature,
            ),
        ))
        .build();
    assert_eq!(definition.card.color_identity(), ColorSet::COLORLESS);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let id = game.create_object_from_definition(&definition, PlayerId::from_index(0), Zone::Hand);
    assert_eq!(game.current_colors(id), Some(ColorSet::GREEN));
    assert!(
        game.current_subtypes(id)
            .unwrap()
            .contains(&Subtype::Wizard)
    );
}

#[test]
fn identity_exception_remains_a_cda_for_color_layer_ordering() {
    for definition in definitions(
        "Mana cost: {2}{R}\nType: Creature — Shapeshifter\nPower/Toughness: 2/2\nCDA Probe is all colors. This ability doesn't affect its color identity.",
    ) {
        let owner = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let painter = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                painter,
                owner,
                EffectTarget::AllPermanents,
                Modification::SetColors(ColorSet::BLUE),
            ));
        let id = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_colors(id), Some(ColorSet::BLUE));
        assert_eq!(game.object(id).unwrap().color_identity(), ColorSet::RED);
    }
}

#[test]
fn identity_exception_does_not_hide_other_rules_text_symbols_or_color_indicators() {
    for definition in definitions(
        "Mana cost: {3}\nColor indicator: Blue\nType: Creature — Shapeshifter\nPower/Toughness: 2/2\nCDA Probe is all colors. This ability doesn't affect its color identity.\n{G}: CDA Probe gets +1/+1 until end of turn.",
    ) {
        assert_eq!(
            definition.card.color_identity(),
            ColorSet::BLUE.union(ColorSet::GREEN)
        );
    }
}

#[test]
fn exception_cannot_silently_attach_to_a_group_or_discard_a_tail() {
    for line in [
        "Creatures are all colors. This ability doesn't affect its color identity.",
        "CDA Probe is all colors. This ability doesn't affect its color identity until end of turn.",
    ] {
        let text = format!("Type: Creature — Shapeshifter\nPower/Toughness: 2/2\n{line}");
        assert!(
            compile_to_runtime_definition("CDA Probe", &text, false).is_err(),
            "{line}"
        );
    }
}

#[test]
fn ordinary_granted_all_types_do_not_become_all_zone_cdas() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
    use ironsmith::{CardId, CardType, ObjectFilter};
    for zone in [Zone::Hand, Zone::Battlefield] {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Grant host")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .build();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let id = game.create_object_from_definition(&definition, PlayerId::from_index(0), zone);
        game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
            id,
            StaticAbilityId::AddAllSubtypesOfFamily,
            Some(StaticAbility::add_all_subtypes_of_family(
                ObjectFilter::source(),
                ironsmith::types::SubtypeFamily::Creature,
            )),
        );
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.current_subtypes(id)
                .unwrap()
                .contains(&Subtype::Wizard),
            zone == Zone::Battlefield
        );
    }
}

#[test]
fn artifacts_from_before_the_cda_changes_require_recompilation() {
    let (mut artifact, _) = compile_to_artifact(
        "CDA Probe",
        "Type: Creature — Illusion\nPower/Toughness: 3/3\nCDA Probe is every creature type.",
        false,
    )
    .unwrap();
    artifact.engine_schema_hash =
        "a4bb7964a2b6b477e4d128c655ca747453c74a9d5d1cf119ac7daf399fb840c0".into();
    artifact.refresh_checksum();
    assert!(artifact.validate().is_err());
    assert!(
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).is_err()
    );
}
