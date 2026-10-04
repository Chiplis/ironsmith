//! UNVALIDATED base-power primitive and exact source/incarnation scenarios.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::{Effect, Until, Value};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::resolve_stack_entry_with;
use ironsmith::game_state::StackEntry;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::target::ChooseSpec;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
const A: PlayerId = PlayerId::from_index(0);
fn definitions() -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(
            "Base power source",
            "Mana cost: {2}\nType: Creature — Human\nPower/Toughness: 2/3\n{0}: You gain life equal to this creature's base power.",
            false,
        )
    });
    let (artifact, direct) = result.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) {
    execute_effect(game, &effect, &mut EffectContext::new_default(source, A)).unwrap();
}
#[test]
fn source_base_power_value_keeps_setting_ignores_modifiers_and_uses_departure_lki() {
    for definition in definitions() {
        for departed in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let ability = definition
                .abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
                    _ => None,
                })
                .unwrap();
            assert!(format!("{:?}", ability.effects).contains("BasePowerOf"));
            game.push_to_stack(StackEntry::ability(
                source,
                A,
                ability
                    .effects
                    .flattened_default_effects()
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>(),
            ));
            apply(
                &mut game,
                source,
                Effect::set_base_power_toughness(
                    7,
                    9,
                    ChooseSpec::SpecificObject(source),
                    Until::EndOfTurn,
                ),
            );
            apply(
                &mut game,
                source,
                Effect::pump(10, 3, ChooseSpec::SpecificObject(source), Until::EndOfTurn),
            );
            apply(
                &mut game,
                source,
                Effect::put_counters(
                    ironsmith::object::CounterType::PlusOnePlusOne,
                    2,
                    ChooseSpec::SpecificObject(source),
                ),
            );
            assert_eq!(game.calculated_power(source), Some(19));
            if departed {
                let grave = game
                    .move_object_by_game_rule(source, Zone::Graveyard)
                    .unwrap();
                let returned = game
                    .move_object_by_game_rule(grave, Zone::Battlefield)
                    .unwrap();
                apply(
                    &mut game,
                    returned,
                    Effect::set_base_power_toughness(
                        99,
                        99,
                        ChooseSpec::SpecificObject(returned),
                        Until::EndOfTurn,
                    ),
                );
                assert_eq!(
                    game.stack[0].source_snapshot.as_ref().unwrap().base_power,
                    Some(7)
                );
            }
            resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.player(A).unwrap().life, 27);
        }
    }
}
#[test]
fn tagged_base_power_uses_live_setting_then_exact_departure_not_the_card_that_returned() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let watched = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut entry = StackEntry::ability(
            source,
            A,
            vec![Effect::gain_life(Value::BasePowerOf(Box::new(
                ChooseSpec::Tagged("watched".into()),
            )))],
        );
        entry.tagged_objects.insert(
            "watched".into(),
            vec![ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(watched).unwrap(),
                &game,
            )],
        );
        game.push_to_stack(entry);
        apply(
            &mut game,
            watched,
            Effect::set_base_power_toughness(
                8,
                3,
                ChooseSpec::SpecificObject(watched),
                Until::EndOfTurn,
            ),
        );
        let grave = game
            .move_object_by_game_rule(watched, Zone::Graveyard)
            .unwrap();
        let returned = game
            .move_object_by_game_rule(grave, Zone::Battlefield)
            .unwrap();
        apply(
            &mut game,
            returned,
            Effect::set_base_power_toughness(
                40,
                40,
                ChooseSpec::SpecificObject(returned),
                Until::EndOfTurn,
            ),
        );
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().life, 28);
    }
}
