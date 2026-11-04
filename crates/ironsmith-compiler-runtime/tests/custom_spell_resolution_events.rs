//! Native completion boundary for delayed rules such as Orchestrate.
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, StackEntry, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) = ironsmith_compiler_runtime::compile_builder_to_artifact(
        ironsmith_compiler::CardDefinitionBuilder::new(
            ironsmith::CardId::new(),
            "Resolution witness",
        ),
        text,
        false,
    )
    .unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn setup_game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    let model = ironsmith_core::CompilerTrigger::spell_resolves(PlayerFilter::You);
    let restored = serde_json::from_str(&serde_json::to_string(&model).unwrap()).unwrap();
    assert_eq!(model, restored);
    let observer = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Resolution observer",
    )
    .card_types(vec![ironsmith::CardType::Enchantment])
    .with_ability(ironsmith::ability::Ability::triggered(
        ironsmith::triggers::Trigger::from_model(restored).unwrap(),
        vec![Effect::gain_life(5)],
    ))
    .build();
    for player in [A, B] {
        game.create_object_from_definition(&observer, player, Zone::Battlefield);
    }
    game
}
fn settle(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    for _ in 0..32 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut dm)
            .unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("unsettled stack");
}
fn spell(game: &mut GameState, definition: &CardDefinition, controller: PlayerId) -> ObjectId {
    let source = game.create_object_from_definition(definition, controller, Zone::Stack);
    game.push_to_stack(StackEntry::new(source, controller));
    source
}
#[test]
fn resolution_notifies_once_for_normal_permanent_and_self_moved_spells() {
    for text in [
        "Type: Sorcery\nYou gain 1 life.",
        "Type: Creature — Bear\nPower/Toughness: 2/2",
        "Type: Sorcery\nExile this spell.",
    ] {
        for definition in definitions(text) {
            let mut game = setup_game();
            spell(&mut game, &definition, A);
            settle(&mut game);
            assert_eq!(
                game.player(A).unwrap().life,
                if text.contains("gain 1") { 26 } else { 25 }
            );
            assert_eq!(game.player(B).unwrap().life, 20);
        }
    }
}
#[test]
fn countered_illegal_target_and_ability_resolution_do_not_notify() {
    for definition in definitions("Type: Instant\nDestroy target creature.") {
        let mut game = setup_game();
        let target_definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Target",
            "Type: Creature\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        let target = game.create_object_from_definition(&target_definition, B, Zone::Battlefield);
        spell(&mut game, &definition, A);
        game.stack.last_mut().unwrap().targets = vec![Target::Object(target)];
        game.move_object_by_effect(target, Zone::Graveyard).unwrap();
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().life,
            20,
            "all-illegal target exit is not resolution"
        );
        let source = spell(&mut game, &definition, A);
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::CounterEffect::new(
                ChooseSpec::SpecificObject(source),
            )),
            &mut EffectContext::new_default(source, B),
        )
        .unwrap();
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 20);
        game.push_to_stack(StackEntry::ability(
            game.battlefield[0],
            A,
            vec![Effect::gain_life(1)],
        ));
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().life,
            21,
            "ability instructions do not generate spell completion"
        );
    }
}
#[test]
fn real_spell_copies_each_resolve_and_current_spell_controller_is_the_actor() {
    for definition in definitions("Type: Sorcery\nYou gain 1 life.") {
        let mut game = setup_game();
        let source = spell(&mut game, &definition, A);
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::CopySpellEffect::single(
                ChooseSpec::SpecificObject(source),
            )),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        assert_eq!(game.stack.len(), 2);
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().life,
            32,
            "two copies of instructions and two completion events"
        );
        let mut game = setup_game();
        spell(&mut game, &definition, B);
        game.stack.last_mut().unwrap().controller = A;
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(
            game.player(B).unwrap().life,
            26,
            "current spell controller, not stale entry controller, performs resolution"
        );
    }
}
