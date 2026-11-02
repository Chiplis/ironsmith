//! Complete source linkage across descriptive exile/return and delayed face-up.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-linked-face-down.json")).unwrap();
    let card = fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap();
    assert_eq!(card["rewrite_status"], "frontend_materialized_unverified", "{name}");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact(name, card["executable_text"].as_str().unwrap(), false)));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn move_card(game: &mut GameState, source: ObjectId, card: ObjectId, zone: Zone) {
    execute_effect(game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(card), zone, false)), &mut EffectContext::new_default(source, A)).unwrap();
}
fn settle(game: &mut GameState) {
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap(); }
}
#[test]
fn actual_source_leave_delay_pins_returned_card_and_never_follows_later_incarnations() {
    for definition in definitions("An and En, Destined Souls") { for departed_before in [false, true] { for recipient_blinked in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Submerged recipient", "Mana cost: {G}\nType: Creature — Elf\nPower/Toughness: 4/5\nFlying", false).unwrap();
        let target = game.create_object_from_definition(&body, B, Zone::Battlefield);
        let stable = game.object(target).unwrap().stable_id;
        let activated = definition.abilities.iter().find_map(|a| match &a.kind { ironsmith::ability::AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
        let mut entry = StackEntry::ability(source, A, activated.effects.clone()).with_targets(vec![Target::Object(target)]);
        entry.source_snapshot = Some(ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game));
        game.push_to_stack(entry);
        if departed_before { move_card(&mut game, source, source, Zone::Graveyard); }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        let mut recipient = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.is_face_down(recipient));
        assert_eq!(game.controller_of_id(recipient), Some(B));
        assert!(!game.has_printed_cost_turn_face_up_permission(recipient));
        if recipient_blinked {
            move_card(&mut game, source, recipient, Zone::Exile);
            recipient = game.find_object_by_stable_id(stable).unwrap();
            move_card(&mut game, source, recipient, Zone::Battlefield);
            recipient = game.find_object_by_stable_id(stable).unwrap();
            game.set_face_down(recipient);
        }
        if !departed_before { move_card(&mut game, source, source, Zone::Graveyard); }
        settle(&mut game);
        assert_eq!(game.is_face_down(recipient), departed_before || recipient_blinked);
        if !departed_before && !recipient_blinked {
            assert_eq!(game.calculated_power(recipient), Some(4));
            assert!(!game.has_custom_status(recipient, "submerged"));
        }
    } } }
}

#[test]
fn actual_absence_state_delay_triggers_even_if_source_left_before_its_entry_ability_resolves() {
    for definition in definitions("Engulfing Incubus") { for departed_before in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("State-delay recipient", "Mana cost: {G}\nType: Creature\nPower/Toughness: 4/5", false).unwrap();
        let target = game.create_object_from_definition(&body, B, Zone::Battlefield);
        let stable = game.object(target).unwrap().stable_id;
        let source_hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let source_stable = game.object(source_hand).unwrap().stable_id;
        move_card(&mut game, source_hand, source_hand, Zone::Battlefield);
        let source = game.find_object_by_stable_id(source_stable).unwrap();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        if departed_before { move_card(&mut game, source, source, Zone::Graveyard); }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let recipient = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.is_face_down(recipient), "the state delay is a triggered ability, never an immediate face-up instruction");
        if !departed_before { move_card(&mut game, source, source, Zone::Graveyard); }
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "state true before registration still creates the one-shot delayed trigger");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(!game.is_face_down(recipient));
        assert_eq!(game.calculated_power(recipient), Some(4));
    } }
}

#[test]
fn source_phasing_is_not_absence_from_the_battlefield_for_the_actual_state_delay() {
    for definition in definitions("Engulfing Incubus") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Phase-state recipient", "Type: Creature\nPower/Toughness: 4/5", false).unwrap();
        let target = game.create_object_from_definition(&body, B, Zone::Battlefield);
        let stable = game.object(target).unwrap().stable_id;
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let source_stable = game.object(hand).unwrap().stable_id;
        move_card(&mut game, hand, hand, Zone::Battlefield);
        let source = game.find_object_by_stable_id(source_stable).unwrap();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        game.phase_out(source);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let recipient = game.find_object_by_stable_id(stable).unwrap();
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert!(game.stack.is_empty(), "phasing preserves the source's battlefield zone");
        assert!(game.is_face_down(recipient));
        game.phase_in(source);
        move_card(&mut game, source, source, Zone::Graveyard);
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(!game.is_face_down(recipient));
    }
}
