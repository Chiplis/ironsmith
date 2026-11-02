//! MSEM C752.1–3 reuse ordinary die tables; reroll-only rows are legal.
use ironsmith::{GameState, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: ironsmith::PlayerId = ironsmith::PlayerId::from_index(0);

fn definitions(text: &str) -> [ironsmith::cards::CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition("Result reader", text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let direct = direct.unwrap();
    let (artifact, _) = ironsmith_compiler_runtime::compile_to_artifact("Result reader", text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

#[test]
fn mandatory_reroll_only_row_repeats_without_a_choice_and_stops_on_another_row() {
    for definition in definitions("Type: Sorcery\nRoll a d6.\n1—3 | Roll again.\n4—6 | You gain 4 life.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        game.force_next_die_roll(2);
        game.force_next_die_roll(5);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(A).unwrap().life, 24);
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 2);
        assert_eq!(game.take_forced_die_roll(), None);
    }
}

#[test]
fn a_non_reroll_row_never_repeats_the_process() {
    for definition in definitions("Type: Sorcery\nRoll a d6.\n1—3 | Roll again.\n4—6 | You gain 4 life.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        game.force_next_die_roll(6);
        game.force_next_die_roll(1);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(A).unwrap().life, 24);
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 1);
        assert_eq!(game.take_forced_die_roll(), Some(1));
    }
}

#[test]
fn optional_reroll_only_rows_keep_the_optional_continuation() {
    for definition in definitions("Type: Sorcery\nRoll a d6.\n1—3 | You may roll again.\n4—6 | You gain 4 life.") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("RepeatProcessEffect"), "{debug}");
        assert!(debug.contains("RepeatProcessPromptEffect"), "{debug}");
    }
}

#[test]
fn mandatory_row_effect_repeats_but_an_unrelated_prefix_runs_once() {
    for definition in definitions("Type: Sorcery\nYou gain 2 life.\nRoll a d6.\n1—3 | You gain 1 life. Roll again.\n4—6 | You gain 4 life.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        game.force_next_die_roll(1); game.force_next_die_roll(6);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(A).unwrap().life, 27);
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 2);
    }
}
