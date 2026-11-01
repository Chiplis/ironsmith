use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_core::custom_subtypes::{CustomSubtype, CustomSubtypeDefinition};
use ironsmith_core::{Subtype, SubtypeFamily};
const A: PlayerId = PlayerId(0);
fn vocabulary() -> Vec<CustomSubtypeDefinition> {
    vec![
        CustomSubtypeDefinition {
            singular: "Moonkin".into(),
            plural: "Moonkin".into(),
            family: SubtypeFamily::Creature,
        },
        CustomSubtypeDefinition {
            singular: "Skychild".into(),
            plural: "Skychildren".into(),
            family: SubtypeFamily::Creature,
        },
        CustomSubtypeDefinition {
            singular: "Cloudbank".into(),
            plural: "Cloudbanks".into(),
            family: SubtypeFamily::Land,
        },
    ]
}
fn ty(name: &str, family: SubtypeFamily) -> Subtype {
    Subtype::Custom(CustomSubtype::new(name, family).unwrap())
}
fn compile(text: &str, vocab: &[CustomSubtypeDefinition]) -> [CardDefinition; 2] {
    let (artifact, direct) = ironsmith_compiler::with_custom_subtypes(vocab, || {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact("Vocabulary test", text, false)
        })
    })
    .unwrap()
    .unwrap_or_else(|e| panic!("{text}\n{e:?}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn game() -> GameState {
    GameState::new(vec!["A".into(), "B".into()], 20)
}
#[test]
fn singular_irregular_plural_and_invariant_plural_share_identity_and_filters() {
    let moon = ty("moonkin", SubtypeFamily::Creature);
    let sky = ty("skychild", SubtypeFamily::Creature);
    for index in 0..2 {
        let host = compile(
            "Type: Creature — Skychild\nPower/Toughness: 2/2",
            &vocabulary(),
        )[index]
            .clone();
        let plural_host = compile(
            "Type: Creature — Skychildren\nPower/Toughness: 2/2",
            &vocabulary(),
        )[index]
            .clone();
        assert_eq!(host.card.subtypes, plural_host.card.subtypes);
        assert_eq!(host.card.subtypes, vec![sky]);
        let lord = compile(
            "Type: Creature — Moonkin\nPower/Toughness: 1/1\nSkychildren you control get +1/+1.",
            &vocabulary(),
        )[index]
            .clone();
        let mut game = game();
        let id = game.create_object_from_definition(&host, A, Zone::Battlefield);
        game.create_object_from_definition(&lord, A, Zone::Battlefield);
        assert_eq!(game.calculated_power(id), Some(3));
        assert!(
            game.subtypes_in_family(SubtypeFamily::Creature)
                .contains(&moon)
        );
        assert!(
            game.subtypes_in_family(SubtypeFamily::Creature)
                .contains(&sky)
        );
        assert!(
            !game
                .subtypes_in_family(SubtypeFamily::Land)
                .contains(&ty("cloudbank", SubtypeFamily::Land))
        );
        for text in [
            "Type: Instant\nDestroy target Skychild.",
            "Type: Instant\nDestroy target Moonkin.",
            "Type: Enchantment\nWhenever a Skychild enters, you gain 1 life.",
            "Type: Sorcery\nCreate two 1/1 white Skychildren creature tokens.",
        ] {
            compile(text, &vocabulary());
        }
    }
}
#[test]
fn changeling_and_every_type_are_game_local_across_zones_and_card_departure() {
    let moon = ty("moonkin", SubtypeFamily::Creature);
    let land = ty("cloudbank", SubtypeFamily::Land);
    for index in 0..2 {
        let changeling = compile(
            "Type: Creature — Shapeshifter\nPower/Toughness: 1/1\nChangeling",
            &[],
        )[index]
            .clone();
        let reference = compile(
            "Type: Sorcery\nCreate a 1/1 white Moonkin creature token.",
            &vocabulary(),
        )[index]
            .clone();
        let cloudbank = compile("Type: Land — Cloudbank", &vocabulary())[index].clone();
        let every_land = compile(
            "Type: Enchantment\nEach land is every land type in addition to its other types.",
            &[],
        )[index]
            .clone();
        let forest = compile("Type: Basic Land — Forest", &[])[index].clone();
        let mut custom = game();
        let mut official = game();
        let reference_id = custom.create_object_from_definition(&reference, A, Zone::Library);
        custom.create_object_from_definition(&cloudbank, A, Zone::Library);
        for zone in [
            Zone::Battlefield,
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Stack,
        ] {
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
        custom.move_object_by_effect(reference_id, Zone::Exile);
        assert!(
            custom
                .subtypes_in_family(SubtypeFamily::Creature)
                .contains(&moon)
        );
        assert!(
            custom
                .clone()
                .subtypes_in_family(SubtypeFamily::Creature)
                .contains(&moon)
        );
        assert!(
            !game()
                .subtypes_in_family(SubtypeFamily::Creature)
                .contains(&moon)
        );
        assert!(
            !official
                .subtypes_in_family(SubtypeFamily::Land)
                .contains(&Subtype::Satrapy)
        );
    }
}
#[test]
fn compilation_scope_is_explicit_restored_and_rejects_collisions() {
    let unknown = "Type: Creature — Skychild\nPower/Toughness: 1/1";
    assert!(ironsmith_compiler_runtime::compile_to_artifact("Unknown", unknown, false).is_err());
    compile(unknown, &vocabulary());
    assert!(ironsmith_compiler_runtime::compile_to_artifact("Unknown", unknown, false).is_err());
    for word in [
        "Elf",
        "Elves",
        "Creature",
        "target",
        "Mouse",
        "Mice",
        "Red",
        "Legendary",
        "One",
        "Graveyard",
        "Tapped",
    ] {
        let bad = [CustomSubtypeDefinition {
            singular: "Novelkin".into(),
            plural: word.into(),
            family: SubtypeFamily::Creature,
        }];
        assert!(
            ironsmith_compiler::with_custom_subtypes(&bad, || ()).is_err(),
            "{word}"
        );
    }
    let mut conflict = vocabulary();
    conflict[1].plural = "Moonkin".into();
    assert!(ironsmith_compiler::with_custom_subtypes(&conflict, || ()).is_err());
    let failed = std::panic::catch_unwind(|| {
        ironsmith_compiler::with_custom_subtypes(&vocabulary(), || panic!("scope test"))
    });
    assert!(failed.is_err());
    assert!(ironsmith_compiler_runtime::compile_to_artifact("Unknown", unknown, false).is_err());
}

#[test]
fn choices_use_only_the_game_vocabulary_and_respect_exclusions() {
    use ironsmith::effects::{ChooseCreatureTypeEffect, EffectExecutor, EffectContext as ExecutionContext};
    use ironsmith::target::PlayerFilter;
    struct Choose {
        expected: bool,
    }
    impl ironsmith::decision::DecisionMaker for Choose {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            let custom = ctx
                .options
                .iter()
                .find(|option| option.description == "Skychild");
            assert_eq!(custom.is_some(), self.expected);
            vec![custom.unwrap_or(&ctx.options[0]).index]
        }
    }
    let reference = compile(
        "Type: Sorcery\nCreate a 1/1 white Skychild creature token.",
        &vocabulary(),
    )[0]
    .clone();
    for included in [false, true] {
        let mut game = game();
        if included {
            game.create_object_from_definition(&reference, A, Zone::Library);
        }
        let source = ironsmith::ObjectId::new();
        let mut dm = Choose { expected: included };
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        ChooseCreatureTypeEffect::new(PlayerFilter::You, vec![])
            .execute(&mut game, &mut ctx)
            .unwrap();
        if included {
            assert_eq!(
                game.chosen_subtype(source),
                Some(ty("skychild", SubtypeFamily::Creature))
            );
        }
        let mut dm = Choose { expected: false };
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        ChooseCreatureTypeEffect::new(
            PlayerFilter::You,
            vec![ty("skychild", SubtypeFamily::Creature)],
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
    }
}

#[test]
fn everything_counters_include_custom_types_only_in_their_own_game() {
    use ironsmith::object::CounterType;
    let everything = CounterType::Named("everything".into());
    let omo = compile(
        "Type: Enchantment\nEach land with an everything counter on it is every land type in addition to its other types.\nEach nonland creature with an everything counter on it is every creature type.",
        &[],
    );
    let custom_creature = compile(
        "Type: Creature — Skychild\nPower/Toughness: 1/1",
        &vocabulary(),
    );
    let custom_land = compile("Type: Land — Cloudbank", &vocabulary());
    let bear = compile("Type: Creature — Bear\nPower/Toughness: 2/2", &[]);
    let forest = compile("Type: Basic Land — Forest", &[]);
    for index in 0..2 {
        for included in [false, true] {
            let mut game = game();
            if included {
                game.create_object_from_definition(&custom_creature[index], A, Zone::Library);
                game.create_object_from_definition(&custom_land[index], A, Zone::Library);
            }
            let creature = game.create_object_from_definition(&bear[index], A, Zone::Battlefield);
            let land = game.create_object_from_definition(&forest[index], A, Zone::Battlefield);
            game.create_object_from_definition(&omo[index], A, Zone::Battlefield);
            assert!(!game.current_has_subtype(creature, Subtype::Elf));
            assert!(!game.current_has_subtype(land, Subtype::Island));
            game.add_counters(creature, everything, 1);
            game.add_counters(land, everything, 1);
            assert!(game.current_has_subtype(creature, Subtype::Elf));
            assert!(game.current_has_subtype(land, Subtype::Island));
            assert_eq!(
                game.current_has_subtype(creature, ty("skychild", SubtypeFamily::Creature)),
                included
            );
            assert_eq!(
                game.current_has_subtype(land, ty("cloudbank", SubtypeFamily::Land)),
                included
            );
            assert!(!game.current_has_subtype(creature, ty("cloudbank", SubtypeFamily::Land)));
            game.remove_counters(creature, everything, 1, None, None);
            assert!(!game.current_has_subtype(creature, ty("skychild", SubtypeFamily::Creature)));
        }
    }
}

#[test]
fn vocabulary_dependencies_are_per_card_even_within_one_compilation_scope() {
    ironsmith_compiler::with_custom_subtypes(&vocabulary(), || {
        let custom = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Custom",
            "Type: Creature — Skychild\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        let official = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Official",
            "Type: Creature — Bear\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        assert_eq!(
            custom.card.custom_subtypes,
            vec![ty("skychild", SubtypeFamily::Creature)]
        );
        assert!(official.card.custom_subtypes.is_empty());
        let mut game = game();
        game.create_object_from_definition(&official, A, Zone::Library);
        assert!(
            !game
                .subtypes_in_family(SubtypeFamily::Creature)
                .contains(&ty("skychild", SubtypeFamily::Creature))
        );
        assert!(
            ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Wrong family",
                "Type: Land — Skychild",
                false
            )
            .is_err()
        );
        assert!(
            ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Undeclared plural",
                "Type: Creature — Skychilds\nPower/Toughness: 1/1",
                false
            )
            .is_err()
        );
    })
    .unwrap();
}

#[test]
fn all_supported_subtype_families_round_trip_without_new_inherent_rules() {
    for (family, card_type) in [
        (SubtypeFamily::Creature, "Creature"),
        (SubtypeFamily::Land, "Land"),
        (SubtypeFamily::Artifact, "Artifact"),
        (SubtypeFamily::Enchantment, "Enchantment"),
        (SubtypeFamily::Spell, "Instant"),
        (SubtypeFamily::Planeswalker, "Planeswalker"),
        (SubtypeFamily::Battle, "Battle"),
    ] {
        let defs = [CustomSubtypeDefinition {
            singular: "Noveltype".into(),
            plural: "Noveltypes".into(),
            family,
        }];
        let mut text = format!("Type: {card_type} — Noveltype");
        if family == SubtypeFamily::Creature {
            text.push_str("\nPower/Toughness: 1/1");
        }
        for definition in compile(&text, &defs) {
            let subtype = ty("noveltype", family);
            assert_eq!(definition.card.subtypes, vec![subtype]);
            assert!(subtype.belongs_to_family(family));
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Library);
            assert!(game.subtypes_in_family(family).contains(&subtype));
            assert!(
                !game
                    .subtypes_in_family(if family == SubtypeFamily::Creature {
                        SubtypeFamily::Land
                    } else {
                        SubtypeFamily::Creature
                    })
                    .contains(&subtype)
            );
        }
    }
}

#[test]
fn mentioning_a_card_name_does_not_activate_the_same_named_subtype() {
    let official = compile(
        "Type: Sorcery\nSearch your library for a card named Skychild, reveal it, put it into your hand, then shuffle.",
        &vocabulary(),
    );
    for definition in official {
        assert!(
            definition.card.custom_subtypes.is_empty(),
            "{:?}",
            definition.card.custom_subtypes
        );
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Library);
        assert!(
            !game
                .subtypes_in_family(SubtypeFamily::Creature)
                .contains(&ty("skychild", SubtypeFamily::Creature))
        );
    }
}

#[test]
fn creating_custom_subtype_tokens_executes_with_the_canonical_singular_name() {
    use ironsmith::effects::{EffectContext, execute_effect};
    for definition in compile(
        "Type: Sorcery\nCreate two 1/1 white Skychildren creature tokens.",
        &vocabulary(),
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut ctx = EffectContext::new_default(source, A);
        for effect in definition.spell_effect.as_ref().unwrap().iter() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let tokens: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .collect();
        assert_eq!(tokens.len(), 2);
        for token in tokens {
            assert_eq!(token.name.as_ref(), "Skychild Token");
            assert!(
                token
                    .subtypes
                    .contains(&ty("skychild", SubtypeFamily::Creature))
            );
        }
    }
}
