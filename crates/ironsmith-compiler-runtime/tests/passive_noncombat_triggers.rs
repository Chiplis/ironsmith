use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{
    DealDamageEffect, EffectContext, EffectExecutor, ExecuteWithSourceEffect, ForEachObject,
};
use ironsmith::game_loop::{put_triggers_on_stack, resolve_stack_entry};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ObjectFilter, PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/passive_noncombat_triggers.json.fixture"
    ))
    .unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    [direct, registry.get(name).unwrap().clone()]
}

fn assert_typed_trigger(definition: &CardDefinition, grouped: bool) {
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            matches!(
                triggered.trigger.compiled_model()?.kind,
                TriggerKind::DealsNoncombatDamageToPlayer { .. }
            )
            .then_some(triggered)
        })
        .expect("passive noncombat-damage trigger remains typed");
    let TriggerKind::DealsNoncombatDamageToPlayer {
        source,
        player,
        damaged_player_one_or_more,
        during_turn,
        ..
    } = &triggered.trigger.compiled_model().unwrap().kind
    else {
        unreachable!()
    };
    let mut expected_source = ObjectFilter::default();
    expected_source.set_union_one_or_more(true);
    assert_eq!(source, &expected_source);
    assert_eq!(player, &PlayerFilter::Opponent);
    assert_eq!(*damaged_player_one_or_more, grouped);
    assert!(during_turn.is_none());
    let display = triggered.trigger.display();
    assert!(
        display.contains(if grouped {
            "are dealt noncombat damage"
        } else {
            "is dealt noncombat damage"
        }),
        "{display}"
    );
}

fn settle(game: &mut GameState) -> usize {
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack(game, &mut queue).unwrap();
    let count = game.stack.len();
    while !game.stack_is_empty() {
        resolve_stack_entry(game).unwrap();
    }
    count
}

fn deal(
    game: &mut GameState,
    source: ObjectId,
    recipient: ChooseSpec,
    combat: bool,
    amount: i32,
) -> usize {
    let mut dm = SelectFirstDecisionMaker;
    let mut context = EffectContext::new(source, A, &mut dm);
    let outcome = DealDamageEffect::new(amount, recipient)
        .with_combat(combat)
        .execute(game, &mut context)
        .unwrap();
    for event in outcome.events {
        game.queue_trigger_event(Default::default(), event);
    }
    settle(game)
}

#[test]
fn passive_noncombat_triggers_all_four_full_cards_materialize_and_round_trip() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 4);
    for fixture in fixtures {
        let name = fixture["name"].as_str().unwrap();
        for definition in definitions(name, fixture["text"].as_str().unwrap()) {
            assert_typed_trigger(&definition, name == "Master of Barbs");
        }
    }
}

#[test]
fn passive_noncombat_triggers_execute_and_distinguish_grouped_opponents() {
    for fixture in fixtures() {
        let name = fixture["name"].as_str().unwrap();
        let grouped = name == "Master of Barbs";
        for definition in definitions(name, fixture["text"].as_str().unwrap()) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let observer = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let source_def =
                compile_to_runtime_definition("Damage source", "Type: Artifact", false).unwrap();
            let source = game.create_object_from_definition(&source_def, B, Zone::Battlefield);
            let base_power = game.calculated_power(observer).unwrap();
            assert_eq!(
                deal(&mut game, source, ChooseSpec::SpecificPlayer(A), false, 1),
                0
            );
            assert_eq!(
                deal(&mut game, source, ChooseSpec::SpecificPlayer(B), true, 1),
                0
            );
            assert_eq!(
                deal(&mut game, source, ChooseSpec::SpecificPlayer(B), false, 0),
                0
            );
            let expected = if grouped { 1 } else { 2 };
            assert_eq!(
                deal(
                    &mut game,
                    source,
                    ChooseSpec::EachPlayer(PlayerFilter::Opponent),
                    false,
                    1
                ),
                expected,
                "{name}"
            );
            let amount = if name == "Chandra's Spitfire" { 3 } else { 1 };
            assert_eq!(
                game.calculated_power(observer),
                Some(base_power + amount * expected as i32),
                "{name}"
            );
            // A later event is a new trigger, regardless of the same source.
            assert_eq!(
                deal(&mut game, source, ChooseSpec::SpecificPlayer(B), false, 1),
                1
            );
        }
    }
}

#[test]
fn passive_noncombat_triggers_group_simultaneous_sources_by_damaged_recipient() {
    for (clause, expected) in [
        ("an opponent is dealt noncombat damage", 2),
        ("one or more opponents are dealt noncombat damage", 1),
    ] {
        for definition in definitions(
            "Observer",
            &format!("Type: Enchantment\nWhenever {clause}, you gain 1 life."),
        ) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let observer = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let creature = compile_to_runtime_definition(
                "Source",
                "Type: Creature\nPower/Toughness: 1/1",
                false,
            )
            .unwrap();
            let sources = (0..2)
                .map(|_| game.create_object_from_definition(&creature, A, Zone::Battlefield))
                .collect::<Vec<_>>();
            // Isolate the simultaneous-damage producer from text recognition
            // of a separate multi-source damage spell. This uses production
            // object/source/player scopes and real damage, never fake events.
            let damage = ForEachObject::new(
                ObjectFilter::creature().you_control(),
                vec![Effect::new(ExecuteWithSourceEffect::new(
                    ChooseSpec::Iterated,
                    Effect::for_players(
                        PlayerFilter::Opponent,
                        vec![Effect::deal_damage(
                            1,
                            ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                        )],
                    ),
                ))],
            );
            let mut dm = SelectFirstDecisionMaker;
            let mut context = EffectContext::new(observer, A, &mut dm);
            let outcome = damage.execute(&mut game, &mut context).unwrap();
            let damage_events = outcome
                .events
                .iter()
                .filter_map(|event| event.downcast::<ironsmith::events::DamageEvent>())
                .collect::<Vec<_>>();
            assert_eq!(damage_events.len(), 4);
            for source in sources {
                assert_eq!(
                    damage_events
                        .iter()
                        .filter(|event| event.source == source)
                        .count(),
                    2
                );
            }
            for event in outcome.events {
                game.queue_trigger_event(Default::default(), event);
            }
            assert_eq!(game.player(B).unwrap().life, 18);
            assert_eq!(game.player(PlayerId(2)).unwrap().life, 18);
            assert_eq!(settle(&mut game), expected, "{clause}");
            assert_eq!(game.player(A).unwrap().life, 20 + expected as i32);
        }
    }
}

#[test]
fn authored_each_creature_damage_preserves_each_source_and_one_simultaneous_action() {
    for (text, creatures_deal_damage) in [
        (
            "Type: Sorcery\nEach creature you control deals 1 damage to each opponent.",
            true,
        ),
        (
            "Type: Sorcery\nFor each creature you control, this spell deals 1 damage to each opponent.",
            false,
        ),
    ] {
        for definition in definitions("Authored multi-source damage", text) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let creature = compile_to_runtime_definition(
                "Source",
                "Type: Creature\nPower/Toughness: 1/1",
                false,
            )
            .unwrap();
            let sources = (0..2)
                .map(|_| game.create_object_from_definition(&creature, A, Zone::Battlefield))
                .collect::<Vec<_>>();
            game.create_object_from_definition(&creature, B, Zone::Battlefield);
            let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
            let mut dm = SelectFirstDecisionMaker;
            let mut context = EffectContext::new(spell, A, &mut dm);
            let mut events = Vec::new();
            for effect in definition
                .spell_effect
                .as_ref()
                .unwrap()
                .flattened_default_effects()
            {
                events.extend(
                    ironsmith::effects::execute_effect(&mut game, effect, &mut context)
                        .unwrap()
                        .events,
                );
            }
            let damage = events
                .iter()
                .filter_map(|event| event.downcast::<ironsmith::events::DamageEvent>())
                .collect::<Vec<_>>();
            assert_eq!(damage.len(), 4);
            if creatures_deal_damage {
                let batches = events
                    .iter()
                    .filter(|event| event.downcast::<ironsmith::events::DamageEvent>().is_some())
                    .map(|event| event.simultaneous_batch())
                    .collect::<Vec<_>>();
                assert!(batches[0].is_some());
                assert!(
                    batches.iter().all(|batch| *batch == batches[0]),
                    "all sources and recipients belong to one authored damage action"
                );
                for source in sources {
                    assert_eq!(
                        damage.iter().filter(|event| event.source == source).count(),
                        2,
                        "each creature is an authored damage source: {text}"
                    );
                }
            } else {
                assert!(
                    damage.iter().all(|event| event.source == spell),
                    "counting creatures must not change an explicit spell source: {text}"
                );
            }
            assert_eq!(game.player(B).unwrap().life, 18);
            assert_eq!(game.player(PlayerId(2)).unwrap().life, 18);
        }
    }
}
