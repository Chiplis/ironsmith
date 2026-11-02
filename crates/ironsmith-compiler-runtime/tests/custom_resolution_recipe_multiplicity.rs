//! Declared recipe identity is opt-in; independent identical programs still stack.
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::{CardDefinition, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact("Recipe witness", text, false)
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let wire = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, wire);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&wire).unwrap(),
    ]
}
fn header(key: Option<&str>, body: &str) -> String {
    let once = key
        .map(|key| format!(" once for the resolution recipe named \"{key}\""))
        .unwrap_or_default();
    format!(
        "If this spell was cast from your hand, instead of putting it into your graveyard as it resolves, exile it{once} and {body}"
    )
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn announce(game: &mut GameState, spell: ObjectId) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
        .unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
            return;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    panic!("announcement stalled");
}
fn flush(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(
        game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(
        game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
}
#[test]
fn default_instances_and_distinct_declared_keys_keep_independent_identical_callbacks() {
    for (first, second, gain) in [
        (None, None, 5),
        (Some("shared recipe"), Some("shared recipe"), 3),
        (Some("first recipe"), Some("second recipe"), 5),
    ] {
        let text = format!(
            "Mana cost: {{U}}\nType: Instant\nYou gain 1 life.\n{}\n{}",
            header(first, "You gain 2 life."),
            header(second, "You gain 2 life.")
        );
        for definition in definitions(&text) {
            let mut game = game();
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Blue, 2);
            for occurrence in 1..=2 {
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                let stable = game.object(spell).unwrap().stable_id;
                announce(&mut game, spell);
                ironsmith::game_loop::resolve_stack_entry_with(
                    &mut game,
                    &mut SelectFirstDecisionMaker,
                )
                .unwrap();
                assert_eq!(game.player(A).unwrap().life, 20 + gain * occurrence);
                assert_eq!(
                    game.object(game.find_object_by_stable_id(stable).unwrap())
                        .unwrap()
                        .zone,
                    Zone::Exile
                );
            }
        }
    }
}
#[test]
fn header_receipt_scope_binds_explicit_exiled_card_in_a_delayed_child() {
    let text = format!(
        "Mana cost: {{U}}\nType: Instant\nYou gain 1 life.\n{}",
        header(
            Some("captured recipe"),
            "When you next cast a spell, you may cast the exiled card without paying its mana cost."
        )
    );
    for definition in definitions(&text) {
        let mut game = game();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Blue, 2);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        announce(&mut game, source);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let witness = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Next witness",
            "Mana cost: {U}\nType: Instant\nYou gain 1 life.",
            false,
        )
        .unwrap();
        let next = game.create_object_from_definition(&witness, A, Zone::Hand);
        announce(&mut game, next);
        flush(&mut game);
        assert_eq!(game.stack.len(), 2);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Stack
        );
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(game.player(A).unwrap().life, 22);
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Graveyard
        );
        assert!(game.effect_store.delayed_triggers.is_empty());
    }
}
