//! Complete frozen bodies and native scenarios; campaign execution is deferred.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::effects::{DealDamageEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/chosen_entry_static_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {p}/{t}\n")); }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions { assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition)); }
    definitions
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 100);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn simple(types: &str, cost: &str, text: &str) -> CardDefinition {
    let stats = if types.contains("Creature") { "\nPower/Toughness: 2/3" } else { "" };
    compile_to_runtime_definition("Witness", format!("Mana cost: {cost}\nType: {types}{stats}\n{text}"), false).unwrap()
}
struct ChooseDinosaur;
impl DecisionMaker for ChooseDinosaur {
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(option) = context.options.iter().find(|option| option.legal && option.description.eq_ignore_ascii_case("Dinosaur")) {
            vec![option.index]
        } else { SelectFirstDecisionMaker.decide_options(game, context) }
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, owner: PlayerId) -> ObjectId {
    let hand = game.create_object_from_definition(definition, owner, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut ChooseDinosaur).unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    let id = receipt.original.into_result().unwrap().new_id;
    game.refresh_continuous_state().unwrap();
    id
}
fn damage(game: &mut GameState, source: ObjectId, controller: PlayerId, target: ChooseSpec, combat: bool) {
    DealDamageEffect::new(2, target).with_combat(combat)
        .execute(game, &mut EffectContext::new(source, controller, &mut SelectFirstDecisionMaker)).unwrap();
}

#[test]
fn collective_inferno_convoke_and_choice_are_real_and_both_damage_kinds_double() {
    for definition in definitions("Collective Inferno") {
        let mut game = game();
        let creatures = (0..5).map(|_| game.create_object_from_definition(
            &simple("Creature — Dinosaur", "{R}", ""), A, Zone::Battlefield)).collect::<Vec<_>>();
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).expect("five red creatures can convoke the full cost");
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut dm = ChooseDinosaur;
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..40 {
            if state.pending_cast.is_none() { break; }
            let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut dm).unwrap();
        }
        assert!(state.pending_cast.is_none());
        assert!(creatures.iter().all(|id| game.is_tapped(*id)), "convoke taps all five without spending mana");
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        while !game.stack_is_empty() { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        let inferno = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Collective Inferno").unwrap();
        assert_eq!(game.chosen_creature_type(inferno), Some(Subtype::Dinosaur));
        damage(&mut game, creatures[0], A, ChooseSpec::SpecificPlayer(B), false);
        damage(&mut game, creatures[0], A, ChooseSpec::SpecificPlayer(B), true);
        assert_eq!(game.player(B).unwrap().life, 92);
        let wrong_type = game.create_object_from_definition(&simple("Creature — Elf", "{R}", ""), A, Zone::Battlefield);
        let wrong_controller = game.create_object_from_definition(&simple("Creature — Dinosaur", "{R}", ""), B, Zone::Battlefield);
        damage(&mut game, wrong_type, A, ChooseSpec::SpecificPlayer(B), false);
        damage(&mut game, wrong_controller, B, ChooseSpec::SpecificPlayer(B), false);
        assert_eq!(game.player(B).unwrap().life, 88);
        let victim = game.create_object_from_definition(&simple("Creature — Bear", "{2}", ""), B, Zone::Battlefield);
        damage(&mut game, creatures[0], A, ChooseSpec::SpecificObject(victim), false);
        assert_eq!(game.damage_on(victim), 4);
        let spell = game.create_object_from_definition(&simple("Kindred Instant — Dinosaur", "{R}", ""), A, Zone::Stack);
        damage(&mut game, spell, A, ChooseSpec::SpecificPlayer(B), false);
        assert_eq!(game.player(B).unwrap().life, 84, "sources are not restricted to permanent creatures");
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(creatures[0]).unwrap(), &game);
        game.move_object_by_effect(creatures[0], Zone::Graveyard).unwrap();
        DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(B)).execute(&mut game,
            &mut EffectContext::new(creatures[0], A, &mut SelectFirstDecisionMaker).with_source_snapshot(snapshot)).unwrap();
        assert_eq!(game.player(B).unwrap().life, 80, "departed damage sources use their last-known type/controller");
        game.set_current_controller(inferno, B).unwrap();
        game.refresh_continuous_state().unwrap();
        damage(&mut game, creatures[1], A, ChooseSpec::SpecificPlayer(B), false);
        damage(&mut game, wrong_controller, B, ChooseSpec::SpecificPlayer(B), false);
        assert_eq!(game.player(B).unwrap().life, 74);
        game.phase_out(inferno);
        game.refresh_continuous_state().unwrap();
        damage(&mut game, wrong_controller, B, ChooseSpec::SpecificPlayer(B), false);
        assert_eq!(game.player(B).unwrap().life, 72);
        game.phase_in(inferno);
        game.move_object_by_effect(inferno, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        damage(&mut game, wrong_controller, B, ChooseSpec::SpecificPlayer(B), false);
        assert_eq!(game.player(B).unwrap().life, 70);
    }
}

#[test]
fn displaced_dinosaurs_sets_only_new_historic_permanents_and_preserves_added_types() {
    for definition in definitions("Displaced Dinosaurs") {
        let mut game = game();
        let old_artifact = enter(&mut game, &simple("Artifact", "{1}", ""), A);
        let source = enter(&mut game, &definition, A);
        assert!(!game.calculated_card_types(old_artifact).contains(&CardType::Creature));
        let artifact = enter(&mut game, &simple("Artifact", "{1}", ""), A);
        let legendary = enter(&mut game, &simple("Legendary Enchantment", "{1}", ""), A);
        let saga = enter(&mut game, &simple("Enchantment — Saga", "{1}", "I, II — You gain 1 life."), A);
        let flyer = enter(&mut game, &simple("Artifact Creature — Bird", "{1}", "Flying"), A);
        for id in [artifact, legendary, saga, flyer] {
            assert_eq!((game.current_power(id), game.current_toughness(id)), (Some(7), Some(7)));
            assert!(game.current_has_subtype(id, Subtype::Dinosaur));
            assert!(game.calculated_card_types(id).contains(&CardType::Creature));
        }
        assert!(game.calculated_card_types(artifact).contains(&CardType::Artifact));
        assert!(game.calculated_card_types(legendary).contains(&CardType::Enchantment));
        assert!(game.current_has_subtype(saga, Subtype::Saga));
        assert!(game.current_has_subtype(flyer, Subtype::Bird));
        assert!(game.object_has_ability(flyer, &ironsmith::static_abilities::StaticAbility::flying()));
        let ordinary = enter(&mut game, &simple("Creature — Elf", "{1}", ""), A);
        let opponent = enter(&mut game, &simple("Artifact", "{1}", ""), B);
        assert_eq!(game.current_power(ordinary), Some(2));
        assert!(!game.calculated_card_types(opponent).contains(&CardType::Creature));
        game.phase_out(source);
        game.refresh_continuous_state().unwrap();
        let while_absent = enter(&mut game, &simple("Artifact", "{1}", ""), A);
        assert!(!game.calculated_card_types(while_absent).contains(&CardType::Creature));
        assert_eq!(game.current_power(artifact), Some(7), "entry modification persists after source phasing");
        game.phase_in(source);
        game.set_current_controller(source, B).unwrap();
        game.refresh_continuous_state().unwrap();
        let new_b = enter(&mut game, &simple("Artifact", "{1}", ""), B);
        let new_a = enter(&mut game, &simple("Artifact", "{1}", ""), A);
        assert_eq!(game.current_power(new_b), Some(7));
        assert!(!game.calculated_card_types(new_a).contains(&CardType::Creature));
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(artifact), Some(7));
        ironsmith::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(artifact))
            .execute(&mut game, &mut EffectContext::new(artifact, A, &mut SelectFirstDecisionMaker)).unwrap();
        let copy = *game.battlefield.iter().find(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token).unwrap();
        assert_eq!((game.current_power(copy), game.current_toughness(copy)), (Some(7), Some(7)));
        assert!(game.current_has_subtype(copy, Subtype::Dinosaur));
        assert!(game.calculated_card_types(copy).contains(&CardType::Artifact));
        assert!(game.calculated_card_types(copy).contains(&CardType::Creature));
        let returned = game.move_object_by_effect(artifact, Zone::Hand).unwrap();
        let receipt = game.move_object_with_etb_processing_with_dm(returned, Zone::Battlefield, &mut ChooseDinosaur).unwrap();
        let returned = receipt.original.into_result().unwrap().new_id;
        assert!(!game.calculated_card_types(returned).contains(&CardType::Creature), "a new incarnation loses the old entry modification");
    }
}

#[test]
fn filtered_damage_requires_exact_source_evidence_and_rolls_back_unknowns() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{ExecutionError, SequenceEffect, execute_effect};
    for definition in definitions("Collective Inferno") {
        for missing_kind in 0..2 {
            let mut game = game();
            let inferno = enter(&mut game, &definition, A);
            assert_eq!(game.chosen_creature_type(inferno), Some(Subtype::Dinosaur));
            let absent = ObjectId::new();
            let other = game.create_object_from_definition(&simple("Creature — Elf", "{1}", ""), A, Zone::Battlefield);
            let other_snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(other).unwrap(), &game);
            game.take_pending_trigger_events();
            let before_life = (game.player(A).unwrap().life, game.player(B).unwrap().life);
            let sequence = Effect::new(SequenceEffect::new(vec![
                Effect::gain_life(3),
                Effect::new(DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(B))),
            ]));
            let mut dm = SelectFirstDecisionMaker;
            let mut context = EffectContext::new(absent, A, &mut dm);
            if missing_kind == 1 { context.source_snapshot = Some(other_snapshot.clone()); }
            let result = execute_effect(&mut game, &sequence, &mut context);
            assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))),
                "missing/mismatched LKI cannot be a false replacement predicate: {result:?}");
            assert_eq!((game.player(A).unwrap().life, game.player(B).unwrap().life), before_life);
            assert!(game.take_pending_trigger_events().is_empty(), "failed operations cannot publish earlier life or damage events");
        }
        // Exact, known nonmatching evidence is a successful nonmatch. The
        // captured departed Elf must deal two, not error or become a Dinosaur.
        let mut game = game();
        enter(&mut game, &definition, A);
        let elf = game.create_object_from_definition(&simple("Creature — Elf", "{1}", ""), A, Zone::Battlefield);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(elf).unwrap(), &game);
        game.move_object_by_effect(elf, Zone::Graveyard).unwrap();
        let mut dm = SelectFirstDecisionMaker;
        let mut context = EffectContext::new(elf, A, &mut dm).with_source_snapshot(snapshot);
        execute_effect(&mut game, &Effect::new(DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(B))), &mut context).unwrap();
        assert_eq!(game.player(B).unwrap().life, 98);
    }
}
