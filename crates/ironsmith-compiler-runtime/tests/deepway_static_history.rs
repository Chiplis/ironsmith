//! Restored full-body source scenarios; every compilation/execution check is UNRUN.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::continuous::Modification;
use ironsmith::decision::{AttackerDeclaration, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn definitions() -> [CardDefinition; 3] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/deepway_static_history.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Deepway Navigator").unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Deepway Navigator", &text, false));
    let (artifact, materialized) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap(); assert_eq!(artifact, decoded);
    // compile_to_artifact returns an artifact-materialized definition. Keep
    // its route and the JSON roundtrip separate from true direct lowering.
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition("Deepway Navigator", &text, false));
    let direct = direct.unwrap();
    assert!(!direct_loss.is_lossy(), "direct route: {}", direct_loss.reasons_text());
    [direct, materialized, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    for p in [A, B, C] { for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green] {
        game.player_mut(p).unwrap().mana_pool.add(color, 10);
    }}
    game
}
fn object(game: &mut GameState, owner: PlayerId, name: &str, subtype: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, format!("Mana cost: {{1}}\nType: Creature — {subtype}\nPower/Toughness: 2/2"), false).unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}
fn cast(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(id).unwrap().stable_id;
    game.turn.priority_player = Some(A);
    let action = LegalAction::CastSpell { spell_id: id, from_zone: Zone::Hand, casting_method: CastingMethod::Normal };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut dm = SelectFirstDecisionMaker; let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    for _ in 0..20 {
        // Direct resolution queues entry events but does not put their
        // triggers on the stack. Drain again after the spell resolves.
        put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
        if game.stack_is_empty() { break; }
        resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    assert!(game.stack_is_empty()); game.find_object_by_stable_id(stable).unwrap()
}
fn declare(game: &mut GameState, attackers: &[ObjectId]) {
    game.turn.phase = ironsmith::Phase::Combat; game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let declarations = attackers.iter().map(|id| AttackerDeclaration { creature: *id, target: AttackTarget::Player(B) }).collect::<Vec<_>>();
    let mut combat = CombatState::default(); let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, &declarations).unwrap();
    game.combat = Some(combat); put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
}
#[test]
fn complete_body_keeps_flash_entry_and_anthem_through_both_routes() {
    for definition in definitions() {
        let text = ironsmith_text::compiled_text_lines(&definition).join("\n");
        assert!(text.contains("Flash") || text.contains("flash"), "{text}");
        assert!(text.contains("untap") && text.contains("this turn") && text.contains("+1/+0"), "{text}");
        assert_eq!(definition.abilities.len(), 3);
    }
}
#[test]
fn flash_entry_untaps_only_other_merfolk_you_control() {
    for definition in definitions() {
        let mut game = game();
        let own = object(&mut game, A, "Own Merfolk", "Merfolk");
        let foreign = object(&mut game, B, "Foreign Merfolk", "Merfolk");
        let bear = object(&mut game, A, "Own Bear", "Bear");
        for id in [own, foreign, bear] { game.tap(id); }
        game.turn.active_player = B; game.turn.phase = ironsmith::Phase::Combat;
        let source = cast(&mut game, &definition);
        assert!(!game.is_tapped(own)); assert!(game.is_tapped(foreign)); assert!(game.is_tapped(bear));
        assert_eq!(game.current_power(source), Some(2));
    }
}
#[test]
fn distinct_historical_merfolk_survive_departure_type_control_and_phase_changes() {
    for definition in definitions() {
        let mut game = game(); let source = cast(&mut game, &definition);
        let fish: Vec<_> = (0..3).map(|i| object(&mut game, A, &format!("Merfolk {i}"), "Merfolk")).collect();
        let bear = object(&mut game, A, "Bear", "Bear"); let foreign = object(&mut game, B, "Foreign fish", "Merfolk");
        let witness = object(&mut game, A, "Nonattacking Merfolk", "Merfolk");
        for id in fish.iter().copied().chain([source, bear]) { game.remove_summoning_sickness(id); }
        declare(&mut game, &[fish[0], fish[1], bear]);
        assert_eq!(game.current_power(source), Some(2));
        game.combat = None; for id in [fish[0], fish[1]] { game.untap(id); }
        declare(&mut game, &[fish[0], fish[1]]);
        assert_eq!(game.current_power(source), Some(2), "repeat attacks do not count the same objects twice");
        game.combat = None; declare(&mut game, &[fish[2]]);
        assert_eq!(game.current_power(source), Some(3)); assert_eq!(game.current_power(bear), Some(2)); assert_eq!(game.current_power(foreign), Some(2));
        game.move_object_by_effect(fish[0], Zone::Graveyard).unwrap(); game.set_current_controller(fish[1], B).unwrap();
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(fish[2]), Modification::SetCardTypes(vec![CardType::Artifact]), Until::EndOfTurn)
            .execute(&mut game, &mut EffectContext::new_default(fish[2], A)).unwrap();
        assert_eq!(game.current_power(source), Some(3));
        assert_eq!(game.current_power(witness), Some(3));
        game.phase_out(source); assert_eq!(game.current_power(source), None);
        assert_eq!(game.current_power(witness), Some(2), "phased source grants no anthem");
        game.phase_in(source); assert_eq!(game.current_power(source), Some(3));
        assert_eq!(game.current_power(witness), Some(3));
        game.set_current_controller(source, B).unwrap(); assert_eq!(game.current_power(source), Some(2));
        game.set_current_controller(source, A).unwrap(); assert_eq!(game.current_power(source), Some(3));
        game.next_turn(); assert_eq!(game.current_power(source), Some(2));
    }
}
#[test]
fn a_blinked_attacker_is_a_new_creature_even_with_the_same_stable_card() {
    for definition in definitions() {
        let mut game = game(); let source = cast(&mut game, &definition);
        let first = object(&mut game, A, "First fish", "Merfolk"); let second = object(&mut game, A, "Second fish", "Merfolk");
        for id in [first, second] { game.remove_summoning_sickness(id); }
        declare(&mut game, &[first, second]); assert_eq!(game.current_power(source), Some(2)); game.combat = None;
        let stable = game.object(first).unwrap().stable_id;
        let exiled = game.move_object_by_effect(first, Zone::Exile).unwrap(); let returned = game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
        assert_ne!(returned, first); assert_eq!(game.object(returned).unwrap().stable_id, stable);
        game.remove_summoning_sickness(returned); declare(&mut game, &[returned]); assert_eq!(game.current_power(source), Some(3));
    }
}

#[test]
fn late_source_reads_attack_time_types_and_phasing_keeps_one_incarnation() {
    for definition in definitions() {
        let mut game = game();
        let first = object(&mut game, A, "First historical fish", "Merfolk");
        let second = object(&mut game, A, "Second historical fish", "Merfolk");
        let bear = object(&mut game, A, "Future fish", "Bear");
        for id in [first, second, bear] { game.remove_summoning_sickness(id); }
        declare(&mut game, &[first, second, bear]);
        game.combat = None;
        game.phase_out(first); game.phase_in(first); game.untap(first);
        declare(&mut game, &[first]);
        game.combat = None;
        ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(bear),
            Modification::AddSubtypes(vec![ironsmith::Subtype::Merfolk]),
            Until::EndOfTurn,
        ).execute(&mut game, &mut EffectContext::new_default(bear, A)).unwrap();

        let source = cast(&mut game, &definition);
        assert_eq!(game.current_power(source), Some(2),
            "entering later sees only two historical Merfolk, despite a repeat attack and a later subtype change");
        assert!(!game.is_tapped(bear), "the entry trigger reads the bear's current Merfolk subtype");
        declare(&mut game, &[bear]);
        assert_eq!(game.current_power(source), Some(3),
            "the same bear first qualifies when it actually attacks as a Merfolk");
    }
}
