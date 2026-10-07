//! Full frozen Lavabrink body, direct/artifact routes, authored native scenarios.
//! Execution is deferred with the rest of the campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, AttachToEffect, DealDamageEffect, EffectContext, EffectExecutor};
use ironsmith::object::AttachmentTarget;
use ironsmith::target::ChooseSpec;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/chosen_parity_protection.json.fixture")).unwrap();
    let row = &rows[0];
    let name = row["name"].as_str().unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, artifact);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions { assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition)); }
    definitions
}
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }
fn simple(types: &str, mana: u32, text: &str) -> CardDefinition {
    let stats = if types.contains("Creature") { "\nPower/Toughness: 2/3" } else { "" };
    compile_to_runtime_definition("Witness", format!("Mana cost: {{{mana}}}\nType: {types}{stats}\n{text}"), false).unwrap()
}
struct ParityChoice(&'static str);
impl DecisionMaker for ParityChoice {
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(option) = context.options.iter().find(|option| option.legal && option.description.eq_ignore_ascii_case(self.0)) {
            vec![option.index]
        } else { SelectFirstDecisionMaker.decide_options(game, context) }
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, choice: &'static str) -> ObjectId {
    let hand = game.create_object_from_definition(definition, A, Zone::Hand);
    enter_id(game, hand, choice)
}
fn enter_id(game: &mut GameState, hand: ObjectId, choice: &'static str) -> ObjectId {
    let receipt = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut ParityChoice(choice)).unwrap();
    assert!(!receipt.pending && receipt.programs.is_empty());
    let id = receipt.original.into_result().unwrap().new_id;
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.chosen_named_option(id), Some(choice));
    id
}
#[test]
fn both_entry_choices_cover_damage_enchanting_equipping_blocking_and_targeting() {
    for definition in definitions() { for choice in ["odd", "even"] { for mana in 0..=3 {
        let mut game = game();
        let protected = enter(&mut game, &definition, choice);
        let matching = (mana % 2 == 1) == (choice == "odd");
        let source = game.create_object_from_definition(&simple("Creature — Bear", mana, ""), B, Zone::Battlefield);
        // Giving the source the opposite choice cannot change the protected
        // permanent's choice. Zero is included and must match even.
        game.set_chosen_named_option(source, if choice == "odd" { "even" } else { "odd" }.into());
        assert_eq!(ironsmith::targeting::has_protection_from_source(&game, protected, source), matching);
        assert_eq!(ironsmith::rules::combat::can_block(game.object(protected).unwrap(), game.object(source).unwrap(), &game), !matching);
        let spell = game.create_object_from_definition(&simple("Instant", mana, ""), B, Zone::Stack);
        assert_eq!(ironsmith::targeting::compute_legal_targets(&game, &ChooseSpec::target_creature(), B, Some(spell))
            .contains(&Target::Object(protected)), !matching);
        DealDamageEffect::new(2, ChooseSpec::SpecificObject(protected))
            .execute(&mut game, &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker)).unwrap();
        assert_eq!(game.damage_on(protected), if matching { 0 } else { 2 });
        for types in ["Artifact — Equipment", "Enchantment — Aura"] {
            let text = if types.contains("Aura") { "Enchant creature" } else { "" };
            let attachment = game.create_object_from_definition(&simple(types, mana, text), A, Zone::Battlefield);
            AttachToEffect::new(ChooseSpec::SpecificObject(protected))
                .execute(&mut game, &mut EffectContext::new(attachment, A, &mut SelectFirstDecisionMaker)).unwrap();
            assert_eq!(game.object(attachment).unwrap().attached_to == Some(AttachmentTarget::Object(protected)), !matching);
        }
        game.set_current_controller(protected, B).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.chosen_named_option(protected), Some(choice));
        assert_eq!(ironsmith::targeting::has_protection_from_source(&game, protected, source), matching);
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(protected),
            ironsmith::continuous::Modification::RemoveAllAbilities, Until::EndOfTurn)
            .execute(&mut game, &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker)).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::targeting::has_protection_from_source(&game, protected, source));
    } } }
}
#[test]
fn protection_reads_exact_damage_lki_and_a_new_entry_gets_a_new_choice() {
    for definition in definitions() {
        let mut game = game();
        let protected = enter(&mut game, &definition, "odd");
        let source = game.create_object_from_definition(&simple("Creature — Bear", 1, ""), B, Zone::Battlefield);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        DealDamageEffect::new(2, ChooseSpec::SpecificObject(protected)).execute(&mut game,
            &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker).with_source_snapshot(snapshot)).unwrap();
        assert_eq!(game.damage_on(protected), 0);
        let hand = game.move_object_by_effect(protected, Zone::Hand).unwrap();
        let returned = enter_id(&mut game, hand, "even");
        assert_ne!(returned, protected);
        let odd = game.create_object_from_definition(&simple("Creature — Bear", 1, ""), B, Zone::Battlefield);
        let zero = game.create_object_from_definition(&simple("Creature — Bear", 0, ""), B, Zone::Battlefield);
        assert!(!ironsmith::targeting::has_protection_from_source(&game, returned, odd));
        assert!(ironsmith::targeting::has_protection_from_source(&game, returned, zero));
    }
}
#[test]
fn no_choice_does_not_borrow_the_damage_sources_choice_and_grants_bind_their_own_choice() {
    for definition in definitions() {
        let mut game = game();
        let without_choice = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&simple("Creature — Bear", 1, ""), B, Zone::Battlefield);
        game.set_chosen_named_option(source, "odd".into());
        assert!(!ironsmith::targeting::has_protection_from_source(&game, without_choice, source));
        let recipient = enter(&mut game, &simple("Creature — Elf", 2, "As this creature enters, choose odd or even."), "even");
        let grant = enter(&mut game, &simple("Enchantment", 2,
            "As this enchantment enters, choose odd or even.\nCreatures you control have protection from each mana value of the chosen quality."), "odd");
        assert!(ironsmith::targeting::has_protection_from_source(&game, recipient, source));
        let even = game.create_object_from_definition(&simple("Creature — Bear", 2, ""), B, Zone::Battlefield);
        assert!(!ironsmith::targeting::has_protection_from_source(&game, recipient, even));
        game.phase_out(grant);
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::targeting::has_protection_from_source(&game, recipient, source));
    }
}

#[test]
fn missing_source_mana_value_is_incomplete_and_checked_damage_rolls_back() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{ExecutionError, SequenceEffect, execute_effect};
    for definition in definitions() {
        let mut game = game();
        let protected = enter(&mut game, &definition, "odd");
        let absent = ObjectId::new();
        let other = game.create_object_from_definition(&simple("Creature — Bear", 2, ""), B, Zone::Battlefield);
        let other_snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(other).unwrap(), &game);
        let effect = Effect::new(SequenceEffect::new(vec![Effect::gain_life(3),
            Effect::new(DealDamageEffect::new(2, ChooseSpec::SpecificObject(protected)))]));
        for wrong_snapshot in [false, true] {
            game.take_pending_trigger_events();
            let life = game.player(B).unwrap().life;
            let mut dm = SelectFirstDecisionMaker;
            let mut context = EffectContext::new(absent, B, &mut dm);
            if wrong_snapshot { context.source_snapshot = Some(other_snapshot.clone()); }
            assert!(matches!(execute_effect(&mut game, &effect, &mut context), Err(ExecutionError::IncompleteEvidence(_))));
            assert_eq!(game.damage_on(protected), 0);
            assert_eq!(game.player(B).unwrap().life, life);
            assert!(game.take_pending_trigger_events().is_empty());
        }
        // An exact even source is known not to match the chosen odd quality.
        game.move_object_by_effect(other, Zone::Graveyard).unwrap();
        let mut dm = SelectFirstDecisionMaker;
        let mut context = EffectContext::new(other, B, &mut dm).with_source_snapshot(other_snapshot);
        execute_effect(&mut game, &Effect::new(DealDamageEffect::new(2, ChooseSpec::SpecificObject(protected))), &mut context).unwrap();
        assert_eq!(game.damage_on(protected), 2);
    }
}

#[test]
fn stack_x_uses_announced_mana_value_for_parity() {
    for definition in definitions() {
        let mut game = game();
        let protected = enter(&mut game, &definition, "odd");
        let x_spell = compile_to_runtime_definition("X source", "Mana cost: {X}{R}\nType: Instant", false).unwrap();
        let spell = game.create_object_from_definition(&x_spell, B, Zone::Stack);
        for x in 0..=2 {
            game.object_mut(spell).unwrap().x_value = Some(x);
            assert_eq!(ironsmith::targeting::has_protection_from_source(&game, protected, spell), x % 2 == 0);
        }
    }
}

#[test]
fn an_unchosen_grantor_cannot_borrow_its_recipients_even_choice() {
    let grant_text = "Type: Enchantment\nAs this enchantment enters, choose odd or even.\nCreatures you control have protection from each mana value of the chosen quality.";
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition("Unchosen granter", grant_text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = parse_loss::capture(|| compile_to_artifact("Unchosen granter", grant_text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
        let mut game = game();
        let recipient = enter(&mut game, &simple("Creature — Elf", 2, "As this creature enters, choose odd or even."), "even");
        // Bypass entry only to establish the known no-choice negative state.
        let grant = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(game.chosen_named_option(grant), None);
        game.refresh_continuous_state().unwrap();
        for mana in [1, 2] {
            let source = game.create_object_from_definition(&simple("Creature — Bear", mana, ""), B, Zone::Battlefield);
            assert!(!ironsmith::targeting::has_protection_from_source(&game, recipient, source));
            assert!(ironsmith::rules::combat::can_block(game.object(recipient).unwrap(), game.object(source).unwrap(), &game));
        }
    }
}
