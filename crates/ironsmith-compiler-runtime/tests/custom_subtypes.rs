use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_core::{Subtype, SubtypeFamily};
use ironsmith_core::custom_subtypes::{CustomSubtype, CustomSubtypeDefinition};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn vocabulary() -> Vec<CustomSubtypeDefinition> {
    vec![CustomSubtypeDefinition { singular: "Moonkin".into(), plural: "Moonkin".into(), family: SubtypeFamily::Creature },
         CustomSubtypeDefinition { singular: "Skychild".into(), plural: "Skychildren".into(), family: SubtypeFamily::Creature },
         CustomSubtypeDefinition { singular: "Cloudbank".into(), plural: "Cloudbanks".into(), family: SubtypeFamily::Land }]
}
fn ty(name: &str, family: SubtypeFamily) -> Subtype { Subtype::Custom(CustomSubtype::new(name, family).unwrap()) }
fn compile(text: &str, vocab: &[CustomSubtypeDefinition]) -> [CardDefinition; 2] {
    let (artifact, direct) = ironsmith_compiler::with_custom_subtypes(vocab, || {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact("Vocabulary test", text, false))
    }).unwrap().unwrap_or_else(|e| panic!("{text}\n{e:?}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState { GameState::new(vec!["A".into(), "B".into()], 20) }
#[test]
fn singular_irregular_plural_and_invariant_plural_share_identity_and_filters() {
    let moon = ty("moonkin", SubtypeFamily::Creature);
    let sky = ty("skychild", SubtypeFamily::Creature);
    for index in 0..2 {
        let host = compile("Type: Creature — Skychild\nPower/Toughness: 2/2", &vocabulary())[index].clone();
        let plural_host = compile("Type: Creature — Skychildren\nPower/Toughness: 2/2", &vocabulary())[index].clone();
        assert_eq!(host.card.subtypes, plural_host.card.subtypes);
        assert_eq!(host.card.subtypes, vec![sky]);
        let lord = compile("Type: Creature — Moonkin\nPower/Toughness: 1/1\nSkychildren you control get +1/+1.", &vocabulary())[index].clone();
        let mut game = game();
        let id = game.create_object_from_definition(&host, A, Zone::Battlefield);
        game.create_object_from_definition(&lord, A, Zone::Battlefield);
        assert_eq!(game.calculated_power(id), Some(3));
        assert!(game.subtypes_in_family(SubtypeFamily::Creature).contains(&moon));
        assert!(game.subtypes_in_family(SubtypeFamily::Creature).contains(&sky));
        assert!(!game.subtypes_in_family(SubtypeFamily::Land).contains(&ty("cloudbank", SubtypeFamily::Land)));
        for text in ["Type: Instant\nDestroy target Skychild.", "Type: Instant\nDestroy target Moonkin.",
            "Type: Enchantment\nWhenever a Skychild enters, you gain 1 life.",
            "Type: Sorcery\nCreate two 1/1 white Skychildren creature tokens."] {
            compile(text, &vocabulary());
        }
    }
}
#[test]
fn changeling_and_every_type_are_game_local_across_zones_and_card_departure() {
    let moon = ty("moonkin", SubtypeFamily::Creature);
    let land = ty("cloudbank", SubtypeFamily::Land);
    for index in 0..2 {
        let changeling = compile("Type: Creature — Shapeshifter\nPower/Toughness: 1/1\nChangeling", &[])[index].clone();
        let reference = compile("Type: Sorcery\nCreate a 1/1 white Moonkin creature token.", &vocabulary())[index].clone();
        let cloudbank = compile("Type: Land — Cloudbank", &vocabulary())[index].clone();
        let every_land = compile("Type: Enchantment\nEach land is every land type in addition to its other types.", &[])[index].clone();
        let forest = compile("Type: Basic Land — Forest", &[])[index].clone();
        let mut custom = game(); let mut official = game();
        let reference_id = custom.create_object_from_definition(&reference, A, Zone::Library);
        custom.create_object_from_definition(&cloudbank, A, Zone::Library);
        for zone in [Zone::Battlefield, Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile, Zone::Stack] {
            let c = custom.create_object_from_definition(&changeling, A, zone);
            let o = official.create_object_from_definition(&changeling, A, zone);
            assert!(custom.current_has_subtype(c, moon), "custom {zone:?}");
            assert!(!official.current_has_subtype(o, moon), "official {zone:?}");
            assert!(official.current_has_subtype(o, Subtype::Elf));
        }
        for (g, expected) in [(&mut custom, true), (&mut official, false)] {
            let f = g.create_object_from_definition(&forest, A, Zone::Battlefield);
            g.create_object_from_definition(&every_land, A, Zone::Battlefield);
            assert_eq!(g.current_has_subtype(f, land), expected);
            assert!(g.current_has_subtype(f, Subtype::Island));
        }
        custom.move_object(reference_id, Zone::Exile);
        assert!(custom.subtypes_in_family(SubtypeFamily::Creature).contains(&moon));
        assert!(custom.clone().subtypes_in_family(SubtypeFamily::Creature).contains(&moon));
        assert!(!game().subtypes_in_family(SubtypeFamily::Creature).contains(&moon));
        assert!(!official.subtypes_in_family(SubtypeFamily::Land).contains(&Subtype::Satrapy));
    }
}
#[test]
fn compilation_scope_is_explicit_restored_and_rejects_collisions() {
    let unknown = "Type: Creature — Skychild\nPower/Toughness: 1/1";
    assert!(ironsmith_compiler_runtime::compile_to_artifact("Unknown", unknown, false).is_err());
    compile(unknown, &vocabulary());
    assert!(ironsmith_compiler_runtime::compile_to_artifact("Unknown", unknown, false).is_err());
    for word in ["Elf", "Elves", "Creature", "target", "Mouse", "Mice"] {
        let bad = [CustomSubtypeDefinition { singular: "Novelkin".into(), plural: word.into(), family: SubtypeFamily::Creature }];
        assert!(ironsmith_compiler::with_custom_subtypes(&bad, || ()).is_err(), "{word}");
    }
    let mut conflict = vocabulary(); conflict[1].plural = "Moonkin".into();
    assert!(ironsmith_compiler::with_custom_subtypes(&conflict, || ()).is_err());
    let failed = std::panic::catch_unwind(|| ironsmith_compiler::with_custom_subtypes(&vocabulary(), || panic!("scope test")));
    assert!(failed.is_err());
    assert!(ironsmith_compiler_runtime::compile_to_artifact("Unknown", unknown, false).is_err());
}
