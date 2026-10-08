//! Full frozen-card runtime witnesses, authored but UNRUN. No source-only
//! assertion here establishes parser recovery, artifact admission, or a pass.

use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod, can_perform_check};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_builder_to_artifact, compile_builder_to_runtime_definition, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions() -> [CardDefinition; 2] {
    let frozen: serde_json::Value = serde_json::from_str(include_str!(
        "../../../reports/countered-spell-durable-permission-20261008/frozen-inputs.json"
    )).unwrap();
    let row = frozen["actual_card_metadata"].as_array().unwrap().iter()
        .find(|row| row["oracle_id"] == "c01411e0-77b2-4e65-a369-5dbe13745769")
        .expect("complete frozen Kheru metadata");
    let text = format!(
        "Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap(),
    );
    // This is the independent compiler-model -> runtime interpreter route.
    // compile_to_artifact's second result is already artifact-materialized
    // and must not be labelled or counted as a direct conversion witness.
    let builder = || ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), row["name"].as_str().unwrap())
        .first_printed_set_name(row["first_printed_set_name"].as_str().unwrap());
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_builder_to_runtime_definition(builder(), text.clone(), false)
    });
    let native = direct.expect("strict full Kheru direct conversion");
    assert!(!direct_loss.is_lossy(), "{}", direct_loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_builder_to_artifact(builder(), text, false)
    });
    let (artifact, _) = compiled.expect("strict full morph and triggered body");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let loaded = materialize_artifact(&restored).unwrap();
    assert_eq!(native.card.first_printed_set_name.as_deref(), row["first_printed_set_name"].as_str());
    assert_eq!(native.card.first_printed_set_name, loaded.card.first_printed_set_name);
    [native, loaded]
}

fn main_phase(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}

fn take_action(game: &mut GameState, player: PlayerId, action: LegalAction) {
    game.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm,
    ).expect("legal action should begin");
    for _ in 0..40 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending action without a decision: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .expect("action payment and target selection should complete");
    }
    assert!(!state.has_pending_action());
    put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
}

fn cast_actions(game: &GameState, player: PlayerId, id: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
        .collect()
}

fn face_down_kheru(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    main_phase(game, A);
    let hand = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
    let action = cast_actions(game, A, hand).into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { casting_method: CastingMethod::FaceDown, .. }
    )).expect("printed morph must supply the face-down casting route");
    take_action(game, A, action);
    let stack = game.find_object_by_stable_id(stable).unwrap();
    assert!(game.is_face_down(stack));
    assert_eq!(game.current_power(stack), Some(2));
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "face-down cast costs exactly three");
    resolve_stack_entry(game).unwrap();
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
    assert!(game.is_face_down(permanent));
    assert_eq!(game.current_power(permanent), Some(2));
    assert!(game.stack_is_empty(), "casting face down does not trigger turning face up");
    permanent
}

fn opposing_creature_spell(game: &mut GameState) -> ObjectId {
    let definition = compile_to_runtime_definition(
        "Opposing counter target", "Mana cost: {7}{G}\nType: Creature\nPower/Toughness: 4/4", false,
    ).unwrap();
    main_phase(game, B);
    let hand = game.create_object_from_definition(&definition, B, Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    game.player_mut(B).unwrap().mana_pool.add(ManaSymbol::Colorless, 7);
    game.player_mut(B).unwrap().mana_pool.add(ManaSymbol::Green, 1);
    let action = cast_actions(game, B, hand).into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { casting_method: CastingMethod::Normal, .. }
    )).unwrap();
    take_action(game, B, action);
    game.find_object_by_stable_id(stable).unwrap()
}

#[test]
fn frozen_kheru_casts_face_down_pays_full_morph_and_grants_exact_free_cast_after_countering() {
    for definition in definitions() {
        for source_leaves_before_resolution in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = face_down_kheru(&mut game, &definition);
            let target = opposing_creature_spell(&mut game);
            let target_stable = game.object(target).unwrap().stable_id;
            game.turn.priority_player = Some(A);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 4);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 2);
            let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action,
                LegalAction::TurnFaceUp { creature_id, method: TurnFaceUpMethod::TurnFaceUpAbility }
                    if *creature_id == source
            )).expect("full {4}{U}{U} pays the printed morph cost while a spell is on the stack");
            take_action(&mut game, A, action);
            assert!(!game.is_face_down(source));
            assert_eq!(game.current_power(source), Some(3));
            assert_eq!(game.current_toughness(source), Some(3));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.stack.len(), 2, "the face-up event must create the full counter trigger");
            let trigger = game.stack.last().unwrap();
            assert_eq!(trigger.controller, A);
            assert_eq!(trigger.targets, vec![Target::Object(target)]);
            if source_leaves_before_resolution {
                game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
            }
            resolve_stack_entry(&mut game).unwrap();
            assert!(game.stack_is_empty(), "resolving the trigger counters the opposing spell");
            assert!(game.object(target).is_none(), "the old stack incarnation must be gone");
            let exiled = game.find_object_by_stable_id(target_stable).unwrap();
            assert_ne!(exiled, target);
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
            assert_eq!(game.object(exiled).unwrap().owner, B);
            if !source_leaves_before_resolution {
                game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
            }
            assert!(cast_actions(&game, B, exiled).is_empty(), "the owner is not the permission recipient");
            assert!(cast_actions(&game, A, exiled).is_empty(), "a creature cast still needs ordinary timing");
            game.turn.turn_number += 2;
            main_phase(&mut game, A);
            let action = cast_actions(&game, A, exiled).into_iter().next()
                .expect("the zero-mana cast survives source departure and passing turns");
            take_action(&mut game, A, action);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            let cast = game.find_object_by_stable_id(target_stable).unwrap();
            assert_eq!(game.object(cast).unwrap().zone, Zone::Stack);
            assert_eq!(game.controller_of_id(cast), Some(A));
            resolve_stack_entry(&mut game).unwrap();
            let creature = game.find_object_by_stable_id(target_stable).unwrap();
            assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.controller_of_id(creature), Some(A));
            assert_eq!(game.object(creature).unwrap().owner, B);
        }
    }
}

#[test]
fn frozen_kheru_cannot_turn_face_up_with_five_mana_or_a_missing_blue_pip() {
    for definition in definitions() {
        for (colorless, blue) in [(3, 2), (5, 1)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = face_down_kheru(&mut game, &definition);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, colorless);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, blue);
            let before = game.player(A).unwrap().mana_pool.clone();
            let special = SpecialAction::TurnFaceUp {
                permanent_id: source, method: TurnFaceUpMethod::TurnFaceUpAbility,
            };
            assert!(can_perform_check(&special, &game, A).is_err());
            assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action,
                LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == source
            )));
            let result = apply_priority_response_with_dm(
                &mut game, &mut TriggerQueue::new(), &mut PriorityLoopState::new(2),
                &PriorityResponse::PriorityAction(LegalAction::TurnFaceUp {
                    creature_id: source, method: TurnFaceUpMethod::TurnFaceUpAbility,
                }),
                &mut SelectFirstDecisionMaker,
            );
            assert!(result.is_err(), "a forged unaffordable action must be rejected");
            assert!(game.is_face_down(source));
            assert_eq!(game.player(A).unwrap().mana_pool, before);
            assert!(game.stack_is_empty(), "no face-up event means no counter trigger or grant");
        }
    }
}
