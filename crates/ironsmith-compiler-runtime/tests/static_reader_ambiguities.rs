//! Full frozen bodies; authored source scenarios, execution deferred.
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/static_reader_ambiguities.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn simple(name: &str, types: &str) -> CardDefinition {
    let size = if types.contains("Creature") { "\nPower/Toughness: 2/2" } else { "" };
    compile_to_runtime_definition(name, format!("Type: {types}{size}"), false).unwrap()
}
fn enter(game: &mut GameState, definition: &CardDefinition, owner: PlayerId) -> ObjectId {
    let hand = game.create_object_from_definition(definition, owner, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(
        hand, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    receipt.original.into_result().unwrap().new_id
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) {
    execute_effect(game, &effect, &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
}
fn can_block(game: &GameState, attacker: ObjectId, blocker: ObjectId) -> bool {
    ironsmith::rules::combat::can_block(game.object(attacker).unwrap(), game.object(blocker).unwrap(), game)
}
fn activate(game: &mut GameState, source: ObjectId) {
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, .. } | LegalAction::ActivateManaAbility { source: id, .. }
        if *id == source)).expect("source has a real legal activation");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..32 {
        if state.pending_activation.is_none() && state.pending_mana_ability.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    assert!(state.pending_activation.is_none() && state.pending_mana_ability.is_none());
    while !game.stack_is_empty() { resolve_stack_entry_with(game, &mut dm).unwrap(); }
}

#[test]
fn bedlam_is_a_live_rule_that_survives_recipient_ability_loss() {
    for definition in definitions("Bedlam") {
        let mut game = game();
        let attacker = game.create_object_from_definition(&simple("Attacker", "Creature — Bear"), A, Zone::Battlefield);
        let blocker = game.create_object_from_definition(&simple("Blocker", "Creature — Bear"), B, Zone::Battlefield);
        assert!(can_block(&game, attacker, blocker));
        let source = enter(&mut game, &definition, A);
        assert!(!can_block(&game, attacker, blocker));
        apply(&mut game, source, Effect::new(ApplyContinuousEffect::new(
            EffectTarget::Specific(blocker), Modification::RemoveAllAbilities, Until::Forever)));
        assert!(!can_block(&game, attacker, blocker), "global rule is not a granted creature ability");
        let late = game.create_object_from_definition(&simple("Late blocker", "Creature — Bear"), B, Zone::Battlefield);
        assert!(!can_block(&game, attacker, late));
        game.phase_out(source);
        assert!(can_block(&game, attacker, blocker));
        game.phase_in(source);
        assert!(!can_block(&game, attacker, blocker));
        apply(&mut game, source, Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Graveyard, false));
        assert!(can_block(&game, attacker, blocker));
    }
}

#[test]
fn orb_applies_to_both_players_and_all_permanent_types_only_while_present() {
    for definition in definitions("Orb of Dreams") {
        let mut game = game();
        let source = enter(&mut game, &definition, A);
        // Orb's global ability does not apply before Orb exists on the battlefield.
        assert!(!game.is_tapped(source));
        for owner in [A, B] { for types in ["Land", "Artifact", "Enchantment", "Creature — Bear"] {
            let entrant = enter(&mut game, &simple("Entrant", types), owner);
            assert!(game.is_tapped(entrant), "{owner:?}: {types}");
        } }
        game.phase_out(source);
        let absent = enter(&mut game, &simple("Absent source entrant", "Artifact"), B);
        assert!(!game.is_tapped(absent));
        game.phase_in(source);
        let restored = enter(&mut game, &simple("Restored source entrant", "Land"), A);
        assert!(game.is_tapped(restored));
        apply(&mut game, source, Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Graveyard, false));
        let departed = enter(&mut game, &simple("Departed source entrant", "Creature — Bear"), B);
        assert!(!game.is_tapped(departed));
    }
}

#[test]
fn tapped_counter_artifacts_enter_with_both_parts_and_pay_the_exact_counter() {
    for (name, counter) in [("Noble's Purse", CounterType::Named("coin".into())), ("Sphere of the Suns", CounterType::Charge)] {
        for definition in definitions(name) {
            let mut game = game();
            let source = enter(&mut game, &definition, A);
            assert!(game.is_tapped(source));
            assert_eq!(game.counter_count(source, counter), 3);
            assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action,
                LegalAction::ActivateAbility { source: id, .. } | LegalAction::ActivateManaAbility { source: id, .. } if *id == source)));
            for remaining in (0..3).rev() {
                game.untap(source);
                let mana_before = game.player(A).unwrap().mana_pool.total();
                activate(&mut game, source);
                assert!(game.is_tapped(source));
                assert_eq!(game.counter_count(source, counter), remaining);
                if name == "Sphere of the Suns" {
                    assert_eq!(game.player(A).unwrap().mana_pool.total(), mana_before + 1);
                } else {
                    let treasures = game.battlefield.iter().copied().filter(|id|
                        game.object(*id).is_some_and(|object| object.kind == ObjectKind::Token)
                        && game.current_controller(*id) == Some(A)
                        && game.calculated_subtypes(*id).contains(&Subtype::Treasure)).collect::<Vec<_>>();
                    assert_eq!(treasures.len(), 1);
                    activate(&mut game, treasures[0]);
                    assert!(!game.battlefield.contains(&treasures[0]));
                    assert_eq!(game.player(A).unwrap().mana_pool.total(), mana_before + 1);
                }
            }
            game.untap(source);
            let unrelated = game.create_object_from_definition(&simple("Unrelated counters", "Artifact"), A, Zone::Battlefield);
            game.add_counters(unrelated, counter, 3);
            assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action,
                LegalAction::ActivateAbility { source: id, .. } | LegalAction::ActivateManaAbility { source: id, .. } if *id == source)));
        }
    }
}
