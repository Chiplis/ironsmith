use super::*;
use compiler::static_abilities::StaticAbility as ModelStatic;
use ironsmith::effects::EffectContext as ExecutionContext;
use ironsmith::effects::{EffectExecutor, SacrificeTargetEffect};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::{Ability, CardId, CardType, CounterType, GameState, PlayerId, Zone};

fn static_cases() -> Vec<ModelStatic> {
    vec![
        ModelStatic::dredge(3),
        ModelStatic::grants(compiler::grant::GrantSpec::new(
            compiler::grant::Grantable::play_from(),
            ObjectFilter::source(),
            Zone::Graveyard,
        )),
        ModelStatic::exile_to_countered_exile_instead_of_graveyard(
            PlayerFilter::Opponent,
            CounterType::Void,
        ),
        ModelStatic::exile_to_exile_instead_of_graveyard(ObjectFilter::source(), PlayerFilter::Any),
        ModelStatic::exile_would_die_instead(ObjectFilter::source()),
        ModelStatic::shuffle_into_library_from_graveyard(),
    ]
}

#[test]
fn functional_zones_survive_all_conversion_paths_and_explicit_overrides() {
    for model in static_cases() {
        for zones in [
            vec![],
            vec![Zone::Battlefield],
            vec![Zone::Hand],
            vec![Zone::Exile, Zone::Graveyard],
        ] {
            let compiled =
                compiler::ability::Ability::static_ability(model.clone()).in_zones(zones.clone());
            let runtime = runtime_ability_from_core_model(compiled.clone()).unwrap();
            assert_eq!(
                runtime.functional_zones, zones,
                "compiler bridge: {:?}",
                model.id
            );
            let definition = compiler::CardDefinitionBuilder::new(CardId::new(), "Zone fixture")
                .with_ability(compiled)
                .build();
            let wire = wire_definition_from_serializable(&definition).unwrap();
            let loaded =
                ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire)
                    .unwrap();
            assert_eq!(
                loaded.abilities[0].functional_zones, zones,
                "artifact loader: {:?}",
                model.id
            );

            let nested = convert_nested_ability(
                runtime_static_ability_model(model.clone()).unwrap(),
                zones.clone(),
            );
            assert_eq!(
                nested.functional_zones, zones,
                "nested ability: {:?}",
                model.id
            );
            let built = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Native fixture",
            )
            .with_ability(runtime.clone())
            .with_abilities(vec![runtime])
            .build();
            assert!(built.abilities.iter().all(|a| a.functional_zones == zones));
        }
    }
}

// The model interpreter converts embedded abilities independently of artifact loading.
fn convert_nested_ability(
    model: ironsmith::static_abilities::CompiledStaticAbility,
    zones: Vec<Zone>,
) -> Ability {
    let runtime = ironsmith::static_abilities::StaticAbility::from_model(model.clone());
    let ability = Ability::static_ability(runtime).in_zones(zones);
    let model_ability = ability
        .try_map(|_| Ok::<_, ()>(model.clone()), Ok, Ok, Ok, Ok)
        .unwrap();
    ironsmith::static_abilities::StaticAbilityModelInterpreter::ability_from_model(&model_ability)
}

#[test]
fn functional_zone_defaults_are_resolved_before_serialization() {
    for model in static_cases() {
        let compiled = compiler::ability::Ability::static_ability(model.clone());
        let native = Ability::static_ability(runtime_static_ability(model.clone()).unwrap());
        assert_eq!(compiled.functional_zones, native.functional_zones);
        match model.id.unwrap() {
            ironsmith::static_abilities::StaticAbilityId::Dredge | ironsmith::static_abilities::StaticAbilityId::Grants => assert_eq!(compiled.functional_zones, vec![Zone::Graveyard]),
            ironsmith::static_abilities::StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard => assert_eq!(compiled.functional_zones, vec![Zone::Battlefield]),
            _ => assert!(compiled.functional_zones.contains(&Zone::Library)),
        }
    }
    // Native leaves must use the same defaults without going through a compiled model.
    assert_eq!(
        Ability::static_ability(ironsmith::static_abilities::StaticAbility::dredge(3))
            .functional_zones,
        vec![Zone::Graveyard]
    );
}

fn artifact_card(name: &str, text: &str) -> ironsmith::CardDefinition {
    let (artifact, loaded) = compile_builder_to_artifact(
        compiler::CardDefinitionBuilder::new(CardId::new(), name),
        text,
        false,
    )
    .unwrap();
    for (wire, runtime) in artifact
        .payload
        .definition
        .abilities
        .iter()
        .zip(&loaded.abilities)
    {
        assert_eq!(wire.functional_zones, runtime.functional_zones, "{name}");
    }
    loaded
}

#[test]
fn functional_zones_compile_legitimate_nonbattlefield_abilities() {
    let dredge = artifact_card(
        "Dredge fixture",
        "Type: Creature\nPower/Toughness: 1/1\nDredge 3",
    );
    assert_eq!(dredge.abilities[0].functional_zones, vec![Zone::Graveyard]);
    let self_replacement = artifact_card(
        "Self replacement",
        "Type: Creature\nPower/Toughness: 1/1\nIf Self replacement would be put into a graveyard from anywhere, exile it instead.",
    );
    assert!(
        self_replacement.abilities[0]
            .functional_zones
            .contains(&Zone::Library)
    );
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&self_replacement, alice, Zone::Library);
    let mut ctx = ExecutionContext::new_default(source, alice);
    ironsmith::effects::MillEffect::new(1, PlayerFilter::You)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(game.player(alice).unwrap().graveyard.is_empty());
    assert_eq!(game.exile.len(), 1);
    assert_eq!(game.object(game.exile[0]).unwrap().name, "Self replacement");
}

#[derive(Default)]
struct ReplacementChoices(Vec<usize>);
impl ironsmith::decision::DecisionMaker for ReplacementChoices {
    fn decide_options(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.0.push(ctx.options.len());
        vec![
            ctx.options
                .iter()
                .find(|option| option.legal)
                .unwrap()
                .index,
        ]
    }
}

#[test]
fn functional_zones_voidwalker_library_and_departure_do_not_replace_sacrifices() {
    let voidwalker = artifact_card(
        "Dauthi Voidwalker",
        "Type: Creature — Dauthi Rogue\nPower/Toughness: 3/2\nShadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.",
    );
    let land =
        ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Fetchland fixture")
            .card_types(vec![CardType::Land])
            .build();
    let alice = PlayerId::from_index(0);
    for battlefield_count in 0..=2 {
        let mut game = GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dave".into()],
            40,
        );
        let mut battlefield_sources = Vec::new();
        for index in 1..=3 {
            let zone = if index <= battlefield_count {
                Zone::Battlefield
            } else {
                Zone::Library
            };
            let id =
                game.create_object_from_definition(&voidwalker, PlayerId::from_index(index), zone);
            if zone == Zone::Battlefield {
                battlefield_sources.push(id);
            }
        }
        let mut choices = ReplacementChoices::default();
        for after_departure in [false, true] {
            if after_departure {
                for source in &battlefield_sources {
                    game.move_object_by_effect(*source, Zone::Graveyard)
                        .unwrap();
                }
            }
            let fetch = game.create_object_from_definition(&land, alice, Zone::Battlefield);
            let mut ctx = ExecutionContext::new(fetch, alice, &mut choices);
            SacrificeTargetEffect::new(ChooseSpec::SpecificObject(fetch))
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert!(!game.battlefield.contains(&fetch));
            let exiled = !after_departure && battlefield_count > 0;
            let moved = game
                .object_ids_in_deterministic_order()
                .into_iter()
                .filter_map(|id| game.object(id))
                .find(|object| {
                    object.name == "Fetchland fixture"
                        && object.zone == if exiled { Zone::Exile } else { Zone::Graveyard }
                })
                .unwrap();
            assert_eq!(
                moved.counters.get(&CounterType::Void).copied().unwrap_or(0),
                if exiled { 1 } else { 0 }
            );
        }
        assert_eq!(
            choices.0,
            if battlefield_count == 2 {
                vec![2]
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn functional_zones_level_row_combining_preserves_distinct_restrictions() {
    let mut builder = compiler::CardDefinitionBuilder::new(CardId::new(), "Level zone fixture");
    for (level, zone) in [(1, Zone::Hand), (2, Zone::Battlefield), (3, Zone::Hand)] {
        builder = builder.with_ability(
            compiler::ability::Ability::static_ability(ModelStatic::level(
                compiler::ability::LevelAbility::new(level, Some(level)).with_pt(2, 2),
            ))
            .in_zones(vec![zone]),
        );
    }
    let compiled = builder.build();
    let wire = wire_definition_from_serializable(&compiled).unwrap();
    let from_artifact =
        ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).unwrap();
    let from_bridge = into_runtime_definition(compiled).unwrap();
    for loaded in [from_artifact, from_bridge] {
        assert_eq!(loaded.abilities.len(), 2);
        for ability in loaded.abilities {
            let ironsmith::AbilityKind::Static(static_ability) = ability.kind else {
                panic!("expected level static")
            };
            let levels = static_ability.level_abilities().unwrap();
            let expected_levels = if ability.functional_zones == [Zone::Hand] {
                vec![1, 3]
            } else {
                assert_eq!(ability.functional_zones, vec![Zone::Battlefield]);
                vec![2]
            };
            assert_eq!(
                levels
                    .iter()
                    .map(|level| level.min_level)
                    .collect::<Vec<_>>(),
                expected_levels
            );
        }
    }
}

#[test]
fn functional_zones_legacy_artifacts_require_recompilation() {
    let (mut artifact, _) = compile_builder_to_artifact(
        compiler::CardDefinitionBuilder::new(CardId::new(), "Legacy dredge"),
        "Type: Creature\nPower/Toughness: 1/1\nDredge 3",
        false,
    )
    .unwrap();
    artifact.format_version = 3;
    artifact.refresh_checksum();
    let mut registry = ironsmith::cards::CardRegistry::new();
    assert!(matches!(
        registry.register_compiled_artifact(&artifact),
        Err(ironsmith_runtime_catalog::ArtifactRegistrationError::Invalid(_))
    ));
}
