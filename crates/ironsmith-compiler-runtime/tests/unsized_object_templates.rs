//! Frozen no-size grants: live self quantities, ordered ability clearing and native layer timestamps.
//! Authored only; campaign compilation and execution remain deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{AttachToEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::object::CounterType;
use ironsmith::resolution::ResolutionProgram;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{
    CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/unsized_object_templates.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    if let Some(n) = row["loyalty"].as_str() {
        text.push_str(&format!("Loyalty: {n}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn witness(game: &mut GameState, types: Vec<CardType>, colors: ColorSet, token: bool) -> ObjectId {
    let mut card = CardBuilder::new(CardId::new(), "Unlisted witness")
        .card_types(types)
        .subtypes(vec![Subtype::Elf])
        .color_indicator(colors)
        .power_toughness(PowerToughness::fixed(2, 2));
    if token {
        card = card.token();
    }
    game.create_object_from_card(&card.build(), A, Zone::Battlefield)
}
fn source(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.take_pending_trigger_events();
    source
}
fn program(
    game: &mut GameState,
    source: ObjectId,
    program: ResolutionProgram,
    targets: Vec<ObjectId>,
) {
    game.refresh_continuous_state().unwrap();
    let requirements =
        extract_target_requirements_from_program_with_modes(game, &program, A, Some(source), None);
    assert_eq!(
        requirements.len(),
        targets.len(),
        "one authored target must not multiply across its characteristic changes"
    );
    let assignments = requirements
        .iter()
        .zip(&targets)
        .enumerate()
        .map(|(i, (requirement, id))| {
            assert!(requirement.legal_targets.contains(&Target::Object(*id)));
            TargetAssignment {
                spec: requirement.spec.clone(),
                range: i..i + 1,
            }
        })
        .collect();
    game.push_to_stack(
        StackEntry::ability(source, A, program)
            .with_targets(targets.into_iter().map(Target::Object).collect())
            .with_target_assignments(assignments),
    );
    resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    game.refresh_continuous_state().unwrap();
}
fn activated(definition: &CardDefinition, index: usize) -> ResolutionProgram {
    definition
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a.effects.clone()),
            _ => None,
        })
        .nth(index)
        .unwrap()
}
fn event(game: &mut GameState, event: TriggerEvent) {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    while !game.stack_is_empty() {
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    game.refresh_continuous_state().unwrap();
}
fn legendary(game: &GameState, id: ObjectId) -> bool {
    game.current_supertypes(id)
        .unwrap()
        .contains(&Supertype::Legendary)
}


fn settle(game: &mut GameState) {
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    while !game.stack_is_empty() { resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap(); }
    game.refresh_continuous_state().unwrap();
}
fn base(game: &mut GameState, host: ObjectId, size: i32) {
    ironsmith::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::SpecificObject(host),
        ironsmith::continuous::Modification::SetPowerToughness { power: size.into(), toughness: size.into(), sublayer: ironsmith::continuous::PtSublayer::Setting },
        ironsmith::effect::Until::EndOfTurn,
    ).execute(game, &mut EffectContext::new_default(host, A)).unwrap();
    game.refresh_continuous_state().unwrap();
}
#[test]
fn five_complete_no_size_bodies_round_trip_without_loss() {
    let cards: Vec<_> = fixtures().into_iter().filter(|row| row["proposed_complete"] == true).collect();
    assert_eq!(cards.len(), 5);
    for card in cards { for definition in definitions(card["name"].as_str().unwrap()) { assert_eq!(definition.card.name, card["name"]); } }
}
#[test]
fn chimeric_mass_live_counters_and_ordinary_grant_timestamp_are_layer_seven_b() {
    for definition in definitions("Chimeric Mass") {
        let mut game = game(); let host = source(&mut game, &definition);
        game.add_counters(host, CounterType::Charge, 3).unwrap();
        base(&mut game, host, 9);
        program(&mut game, host, activated(&definition, 0), vec![]);
        assert_eq!(game.current_power(host), Some(3), "new grant must override older7b setting");
        game.add_counters(host, CounterType::Charge, 2).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(host), Some(5), "self-stat ability remains dynamic");
        base(&mut game, host, 7);
        assert_eq!(game.current_power(host), Some(7), "later7b setting overrides granted ability");
        assert!(game.object_has_card_type(host, CardType::Artifact));
        assert!(game.object_has_card_type(host, CardType::Creature));
        ironsmith::turn::execute_cleanup_step(&mut game); game.refresh_continuous_state().unwrap();
        assert!(!game.object_has_card_type(host, CardType::Creature));
        assert_eq!(game.object(host).unwrap().counters[&CounterType::Charge], 5);
    }
}
#[test]
fn druid_class_grants_land_its_own_live_controller_scope_not_the_class_scope() {
    for definition in definitions("Druid Class") {
        let mut game = game(); let host = source(&mut game, &definition);
        let land = witness(&mut game, vec![CardType::Land], ColorSet::COLORLESS, false);
        let other = CardBuilder::new(CardId::new(), "Opponent land").card_types(vec![CardType::Land]).build();
        for _ in 0..2 { game.create_object_from_card(&other, B, Zone::Battlefield); }
        game.take_pending_trigger_events();
        let outcome = ironsmith::effects::SetClassLevelEffect::new(3).execute(&mut game, &mut EffectContext::new_default(host, A)).unwrap();
        for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        settle(&mut game);
        assert_eq!(game.current_power(land), Some(1));
        assert!(game.object_has_card_type(land, CardType::Land));
        assert!(game.object_has_ability(land, &StaticAbility::haste()));
        ironsmith::effects::GainControlEffect::permanent(ChooseSpec::SpecificObject(land)).execute(&mut game, &mut EffectContext::new_default(host, B)).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(land), Some(3));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(land), Some(3), "resolved grant survives departed Class");
    }
}
#[test]
fn svogthos_uses_live_graveyard_and_receiver_controller_and_expires() {
    for definition in definitions("Svogthos, the Restless Tomb") {
        let mut game = game(); let host = source(&mut game, &definition);
        let creature = CardBuilder::new(CardId::new(), "Graveyard body").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
        game.create_object_from_card(&creature, A, Zone::Graveyard);
        for _ in 0..2 { game.create_object_from_card(&creature, B, Zone::Graveyard); }
        program(&mut game, host, activated(&definition, 1), vec![]);
        assert_eq!(game.current_power(host), Some(1));
        assert_eq!(game.current_colors(host), Some(ColorSet::BLACK.union(ColorSet::GREEN)));
        assert!(game.object_has_card_type(host, CardType::Land));
        ironsmith::effects::GainControlEffect::permanent(ChooseSpec::SpecificObject(host)).execute(&mut game, &mut EffectContext::new_default(host, B)).unwrap();
        game.refresh_continuous_state().unwrap(); assert_eq!(game.current_power(host), Some(2));
        game.create_object_from_card(&creature, B, Zone::Graveyard);
        game.refresh_continuous_state().unwrap(); assert_eq!(game.current_power(host), Some(3));
        ironsmith::turn::execute_cleanup_step(&mut game); game.refresh_continuous_state().unwrap();
        assert!(!game.object_has_card_type(host, CardType::Creature));
        assert!(game.object_has_card_type(host, CardType::Land));
    }
}
#[test]
fn warden_subtype_only_conversion_preserves_existing_size_and_retains_both_grants() {
    for definition in definitions("Warden of the First Tree") {
        let mut game = game(); let host = source(&mut game, &definition);
        program(&mut game, host, activated(&definition, 1), vec![]);
        assert!(!game.object_has_ability(host, &StaticAbility::trample()), "Warrior gate is real");
        program(&mut game, host, activated(&definition, 0), vec![]);
        base(&mut game, host, 9);
        program(&mut game, host, activated(&definition, 1), vec![]);
        assert_eq!(game.current_power(host), Some(9));
        assert!(game.current_has_subtype(host, Subtype::Spirit));
        assert!(game.object_has_ability(host, &StaticAbility::trample()));
        assert!(game.object_has_ability(host, &StaticAbility::lifelink()));
        program(&mut game, host, activated(&definition, 2), vec![]);
        assert_eq!(game.current_power(host), Some(14));
        assert_eq!(game.object(host).unwrap().counters[&CounterType::PlusOnePlusOne], 5);
    }
}
#[test]
fn frodo_granted_combat_trigger_keeps_its_recipient_and_ring_history() {
    for definition in definitions("Frodo, Sauron's Bane") {
        for times in [0, 4] { for combat in [false, true] {
            let mut game = game(); let host = source(&mut game, &definition);
            program(&mut game, host, activated(&definition, 0), vec![]);
            program(&mut game, host, activated(&definition, 1), vec![]);
            assert_eq!(game.current_power(host), Some(2));
            assert!(game.current_has_subtype(host, Subtype::Rogue));
            assert!(!game.current_has_subtype(host, Subtype::Scout));
            assert!(game.object_has_ability(host, &StaticAbility::lifelink()));
            for _ in 0..times { game.increment_ring_temptations(A); }
            let outcome = ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(B)).with_combat(combat).execute(&mut game, &mut EffectContext::new_default(host, A)).unwrap();
            for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
            settle(&mut game);
            assert_eq!(game.player(B).unwrap().has_lost, combat && times >= 4);
            assert_eq!(game.ring_temptations(A), times + u32::from(combat && times < 4));
        } }
    }
}

#[test]
fn noncreature_template_clears_old_abilities_before_its_new_mana_grant() {
    let text = "Type: Sorcery\nTarget creature becomes a Treasure artifact with \"{T}, Sacrifice this artifact: Add one mana of any color\" and loses all other card types and abilities.";
    let (artifact, direct) = compile_to_artifact("No-size object transformation", text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct, materialize_artifact(&restored).unwrap()] {
        let mut game = game();
        let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Old abilities witness")
            .card_types(vec![CardType::Creature, CardType::Enchantment])
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(2, 3))
            .with_ability(ironsmith::ability::Ability::static_ability(StaticAbility::flying()))
            .build();
        let recipient = game.create_object_from_definition(&creature, A, Zone::Battlefield);
        let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
        program(&mut game, spell, definition.spell_effect.clone().unwrap(), vec![recipient]);
        assert!(game.object_has_card_type(recipient, CardType::Artifact));
        assert!(!game.object_has_card_type(recipient, CardType::Creature));
        assert!(!game.object_has_card_type(recipient, CardType::Enchantment));
        assert!(game.current_has_subtype(recipient, Subtype::Treasure));
        assert!(!game.current_has_subtype(recipient, Subtype::Elf));
        assert!(!game.object_has_ability(recipient, &StaticAbility::flying()));
        let abilities = game.current_abilities(recipient).unwrap();
        assert_eq!(abilities.len(), 1);
        let AbilityKind::Activated(ability) = &abilities[0].kind else { panic!("new mana grant was lost") };
        assert!(ability.is_mana_ability());
        assert!(!ability.effects.is_empty());
    }
}
