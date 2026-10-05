use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::compile_to_runtime_definition;
use serde_json::Value;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/lossy_metadata.json.fixture"
    ))
    .unwrap()
}

fn source(fixture: &Value) -> String {
    let mut lines = Vec::new();
    for (key, label) in [
        ("mana_cost", "Mana cost"),
        ("type_line", "Type"),
        ("loyalty", "Loyalty"),
        ("defense", "Defense"),
    ] {
        if let Some(value) = fixture[key].as_str().filter(|value| !value.is_empty()) {
            lines.push(format!("{label}: {value}"));
        }
    }
    if let (Some(power), Some(toughness)) =
        (fixture["power"].as_str(), fixture["toughness"].as_str())
    {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    lines.push(fixture["oracle_text"].as_str().unwrap().to_string());
    lines.join("\n")
}

#[test]
fn named_source_metadata_inputs_compile_without_loss() {
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let name = fixture["name"].as_str().unwrap();
        if fixture["repair_group"] != "contextual_nontrigger_source" {
            continue;
        }
        let (result, loss) =
            parse_loss::capture(|| compile_to_runtime_definition(name, source(&fixture), false));
        match result {
            Ok(definition) => {
                if loss.is_lossy() {
                    failures.push(format!("{name}: {}", loss.reasons_text()));
                }
                assert_eq!(definition.card.name, name);
                assert!(!definition.card.card_types.is_empty(), "{name}: lost types");
                if let Some(cost) = fixture["mana_cost"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                {
                    assert_eq!(
                        definition.card.mana_cost.as_ref().unwrap().to_oracle(),
                        cost,
                        "{name}"
                    );
                }
                if let Some(power) = fixture["power"].as_str() {
                    let pt = definition
                        .card
                        .power_toughness
                        .expect("printed P/T must survive");
                    assert_eq!(pt.power.to_string(), power, "{name}");
                    assert_eq!(
                        pt.toughness.to_string(),
                        fixture["toughness"].as_str().unwrap(),
                        "{name}"
                    );
                }
                if let Some(loyalty) = fixture["loyalty"].as_str() {
                    assert_eq!(
                        definition.card.loyalty,
                        Some(loyalty.parse().unwrap()),
                        "{name}"
                    );
                }
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed metadata inputs:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn materialized(name: &str, text: &str) -> [ironsmith::cards::CardDefinition; 2] {
    let (result, loss) =
        parse_loss::capture(|| ironsmith_compiler_runtime::compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let bytes = artifact.to_json().unwrap();
    let decoded = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&bytes).unwrap();
    assert_eq!(decoded, artifact);
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded)
        .expect("typed artifact must materialize");
    [direct, restored]
}

fn named_anthem(fixture: &Value, renamed: bool) -> (String, String) {
    let name = fixture["name"].as_str().unwrap();
    let mut text = source(fixture);
    if !renamed {
        return (name.to_string(), text);
    }
    // The same semantic family must work under unrelated full and short names.
    text = text.replace(name, "Counter Custodian, the Patient");
    if let Some((short_name, _)) = name.split_once(',') {
        text = text.replace(short_name, "Counter Custodian");
    }
    ("Counter Custodian, the Patient".to_string(), text)
}

fn is_source_counter_value(value: &ironsmith_core::AnthemValue) -> bool {
    use ironsmith_core::{AnthemCountExpression, AnthemValue, ChooseSpec, Value};
    match value {
        AnthemValue::PerCount {
            multiplier: 1,
            count:
                AnthemCountExpression::CountersOnSource(_)
                | AnthemCountExpression::CountersOnSourceWithSurface { .. },
        } => true,
        AnthemValue::Dynamic(value) => matches!(
            value.unhinted(),
            Value::CountersOn(target, None) if matches!(target.unhinted(), ChooseSpec::Source)
        ),
        _ => false,
    }
}

#[test]
fn named_source_counter_anthems_round_trip_as_source_bound_values() {
    use ironsmith::ability::AbilityKind;
    use ironsmith_core::StaticAbilityPayload;
    for fixture in fixtures().into_iter().filter(|fixture| {
        matches!(
            fixture["name"].as_str(),
            Some("Kangee, Aerie Keeper" | "Kyler, Sigardian Emissary" | "Excalibur II")
        )
    }) {
        for renamed in [false, true] {
            let (name, text) = named_anthem(&fixture, renamed);
            for definition in materialized(&name, &text) {
                let anthem = definition
                    .abilities
                    .iter()
                    .find_map(|ability| {
                        let AbilityKind::Static(ability) = &ability.kind else {
                            return None;
                        };
                        let model = ability.compiled_model()?;
                        let StaticAbilityPayload::Anthem(anthem) = &model.payload else {
                            return None;
                        };
                        Some(anthem)
                    })
                    .expect("counter-counting line must lower to a typed anthem");
                assert!(
                    is_source_counter_value(&anthem.power),
                    "{name}: {:?}",
                    anthem.power
                );
                assert!(
                    is_source_counter_value(&anthem.toughness),
                    "{name}: {:?}",
                    anthem.toughness
                );
            }
        }
    }
}

#[test]
fn named_source_counter_anthems_count_the_correct_object_and_update_live() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::object::{AttachmentTarget, CounterType};
    use ironsmith::{CardId, CardType, GameState, PlayerId, Subtype, Zone};
    for fixture in fixtures().into_iter().filter(|fixture| {
        matches!(
            fixture["name"].as_str(),
            Some("Kangee, Aerie Keeper" | "Kyler, Sigardian Emissary" | "Excalibur II")
        )
    }) {
        let original = fixture["name"].as_str().unwrap();
        let equipment = original == "Excalibur II";
        let (subtype, counted, counts_all) = match original {
            "Kangee, Aerie Keeper" => (Subtype::Bird, CounterType::Named("feather".into()), false),
            "Kyler, Sigardian Emissary" => (Subtype::Human, CounterType::PlusOnePlusOne, true),
            _ => (Subtype::Soldier, CounterType::Charge, false),
        };
        for renamed in [false, true] {
            let (name, text) = named_anthem(&fixture, renamed);
            for definition in materialized(&name, &text) {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let second_source =
                    game.create_object_from_definition(&definition, bob, Zone::Battlefield);
                let target_card = CardBuilder::new(CardId::new(), "Affected creature")
                    .card_types(vec![CardType::Creature])
                    .subtypes(vec![subtype])
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .build();
                let target = game.create_object_from_card(&target_card, alice, Zone::Battlefield);
                let unrelated_card = CardBuilder::new(CardId::new(), "Unrelated creature")
                    .card_types(vec![CardType::Creature])
                    .subtypes(vec![Subtype::Goblin])
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .build();
                let unrelated =
                    game.create_object_from_card(&unrelated_card, alice, Zone::Battlefield);
                if equipment {
                    game.object_mut(source).unwrap().attached_to =
                        Some(AttachmentTarget::Object(target));
                    game.object_mut(target).unwrap().attachments.push(source);
                }
                assert_eq!(
                    game.calculated_power(target),
                    Some(2),
                    "{name}: before counters"
                );
                game.add_counters(target, CounterType::Charge, 7).unwrap();
                game.add_counters(second_source, counted, 5).unwrap();
                // Kangee affects every other Bird; Kyler only affects its
                // controller's Humans and Equipment only affects its host.
                let foreign_bonus = if original == "Kangee, Aerie Keeper" {
                    5
                } else {
                    0
                };
                assert_eq!(
                    game.calculated_power(target),
                    Some(2 + foreign_bonus),
                    "{name}: another object's counters"
                );
                game.add_counters(source, counted, 3).unwrap();
                assert_eq!(
                    game.calculated_power(target),
                    Some(5 + foreign_bonus),
                    "{name}: source counters"
                );
                assert_eq!(
                    game.calculated_toughness(target),
                    Some(5 + foreign_bonus),
                    "{name}"
                );
                assert_eq!(game.calculated_power(unrelated), Some(2), "{name}: scope");
                game.add_counters(source, CounterType::Luck, 2).unwrap();
                let all_counter_bonus = if counts_all { 2 } else { 0 };
                assert_eq!(
                    game.calculated_power(target),
                    Some(5 + foreign_bonus + all_counter_bonus),
                    "{name}: counter kinds"
                );
                game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                assert_eq!(
                    game.calculated_power(target),
                    Some(2 + foreign_bonus),
                    "{name}: source departure"
                );
                game.move_object_by_effect(second_source, Zone::Graveyard)
                    .unwrap();
                assert_eq!(
                    game.calculated_power(target),
                    Some(2),
                    "{name}: all sources gone"
                );
            }
        }
    }
}

fn is_no_defender_fixture(fixture: &Value) -> bool {
    matches!(
        fixture["name"].as_str(),
        Some(
            "Mobile Fort"
                | "Nivix Cyclops"
                | "Territorial Witchstalker"
                | "Walking Wall"
                | "Wall of Wonder"
        )
    )
}

#[test]
fn no_defender_permissions_compile_without_suffix_recovery() {
    for fixture in fixtures().into_iter().filter(is_no_defender_fixture) {
        let text = source(&fixture);
        for name in [fixture["name"].as_str().unwrap(), "Synthetic Guard"] {
            for definition in materialized(name, &text) {
                assert_eq!(
                    definition.card.mana_cost.as_ref().unwrap().to_oracle(),
                    fixture["mana_cost"].as_str().unwrap()
                );
                assert!(
                    definition
                        .card
                        .card_types
                        .contains(&ironsmith::CardType::Creature)
                );
                let pt = definition.card.power_toughness.unwrap();
                assert_eq!(pt.power.to_string(), fixture["power"].as_str().unwrap());
                assert_eq!(
                    pt.toughness.to_string(),
                    fixture["toughness"].as_str().unwrap()
                );
            }
        }
    }
}

fn perform_action_and_resolve(
    game: &mut ironsmith::GameState,
    action: ironsmith::decision::LegalAction,
) {
    perform_action_and_resolve_with(
        game,
        action,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    );
}

fn perform_action_and_resolve_with(
    game: &mut ironsmith::GameState,
    action: ironsmith::decision::LegalAction,
    dm: &mut impl ironsmith::decision::DecisionMaker,
) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
    };
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_activation.is_none() && state.pending_cast.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            break;
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_activation.is_none() && state.pending_cast.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    assert!(
        !game.stack_is_empty(),
        "an activation or cast must really reach the stack"
    );
    while !game.stack_is_empty() {
        resolve_stack_entry_with(game, dm).unwrap();
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    }
}

#[test]
fn no_defender_permissions_keep_pumps_defender_scope_and_expiration() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{
        generate_and_queue_step_triggers, put_triggers_on_stack, resolve_stack_entry,
    };
    use ironsmith::game_state::{Phase, Step};
    use ironsmith::mana::ManaSymbol;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::{GameState, PlayerId, Zone};
    for fixture in fixtures().into_iter().filter(is_no_defender_fixture) {
        let name = fixture["name"].as_str().unwrap();
        let original_power: i32 = fixture["power"].as_str().unwrap().parse().unwrap();
        let original_toughness: i32 = fixture["toughness"].as_str().unwrap().parse().unwrap();
        let (pump_power, pump_toughness) = match name {
            "Wall of Wonder" => (4, -4),
            "Territorial Witchstalker" => (1, 0),
            "Nivix Cyclops" => (3, 0),
            _ => (3, -1),
        };
        for definition in materialized(name, &source(&fixture)) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 20);
            let actor = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.remove_summoning_sickness(actor);
            let bystander_definition = compile_to_runtime_definition(
                "Other defender",
                "Type: Creature — Wall\nPower/Toughness: 0/8\nDefender",
                false,
            )
            .unwrap();
            let bystander =
                game.create_object_from_definition(&bystander_definition, alice, Zone::Battlefield);
            game.remove_summoning_sickness(bystander);
            assert!(!ironsmith::rules::combat::can_attack(
                game.object(actor).unwrap(),
                &game
            ));
            assert!(!game.current_has_static_ability_id(
                actor,
                StaticAbilityId::CanAttackAsThoughNoDefender
            ));
            match name {
                "Nivix Cyclops" => {
                    let spell = compile_to_runtime_definition(
                        "Harmless instant",
                        "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
                        false,
                    )
                    .unwrap();
                    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
                    perform_action_and_resolve(
                        &mut game,
                        LegalAction::CastSpell {
                            spell_id,
                            from_zone: Zone::Hand,
                            casting_method: CastingMethod::Normal,
                        },
                    );
                }
                "Territorial Witchstalker" => {
                    game.turn.phase = Phase::Combat;
                    game.turn.step = Some(Step::BeginCombat);
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    generate_and_queue_step_triggers(&mut game, &mut queue);
                    assert!(
                        queue.entries.is_empty(),
                        "intervening-if must require a powerful creature"
                    );
                    let support = compile_to_runtime_definition(
                        "Combat support",
                        "Type: Creature — Beast\nPower/Toughness: 4/4",
                        false,
                    )
                    .unwrap();
                    game.create_object_from_definition(&support, alice, Zone::Battlefield);
                    generate_and_queue_step_triggers(&mut game, &mut queue);
                    assert_eq!(queue.entries.len(), 1);
                    put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    assert_eq!(game.stack.len(), 1);
                    resolve_stack_entry(&mut game).unwrap();
                }
                _ => {
                    let activation = compute_legal_actions(&game, alice).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == actor)).expect("metadata-bearing creature must have a payable activation");
                    perform_action_and_resolve(&mut game, activation);
                }
            }
            assert_eq!(
                game.current_power(actor),
                Some(original_power + pump_power),
                "{name}: pump preserved"
            );
            assert_eq!(
                game.current_toughness(actor),
                Some(original_toughness + pump_toughness),
                "{name}: toughness preserved"
            );
            assert!(
                game.current_has_static_ability_id(actor, StaticAbilityId::Defender),
                "{name}: defender is retained"
            );
            assert!(
                game.current_has_static_ability_id(
                    actor,
                    StaticAbilityId::CanAttackAsThoughNoDefender
                ),
                "{name}: typed attack permission"
            );
            assert!(
                ironsmith::rules::combat::can_attack(game.object(actor).unwrap(), &game),
                "{name}"
            );
            assert!(
                !ironsmith::rules::combat::can_attack(game.object(bystander).unwrap(), &game),
                "{name}: only the source receives permission"
            );
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(
                game.current_power(actor),
                Some(original_power),
                "{name}: pump expires"
            );
            assert_eq!(
                game.current_toughness(actor),
                Some(original_toughness),
                "{name}"
            );
            assert!(game.current_has_static_ability_id(actor, StaticAbilityId::Defender));
            assert!(
                !game.current_has_static_ability_id(
                    actor,
                    StaticAbilityId::CanAttackAsThoughNoDefender
                ),
                "{name}: permission expires"
            );
            assert!(!ironsmith::rules::combat::can_attack(
                game.object(actor).unwrap(),
                &game
            ));
        }
    }
}

#[test]
fn authored_ability_word_triggers_keep_source_context_and_labels() {
    let mut failures = Vec::new();
    for fixture in fixtures()
        .into_iter()
        .filter(|fixture| fixture["repair_group"] == "authored_ability_word_trigger")
    {
        let name = fixture["name"].as_str().unwrap();
        let (result, loss) =
            parse_loss::capture(|| compile_to_runtime_definition(name, source(&fixture), false));
        match result {
            Ok(definition) => {
                if loss.is_lossy() {
                    failures.push(format!("{name}: {}", loss.reasons_text()));
                }
                assert_eq!(
                    definition.card.mana_cost.as_ref().unwrap().to_oracle(),
                    fixture["mana_cost"].as_str().unwrap()
                );
                assert!(!definition.card.card_types.is_empty());
                for line in fixture["oracle_text"].as_str().unwrap().lines() {
                    if let Some((label, body)) = line.split_once(" — ") {
                        if body.starts_with("When") || body.starts_with("At ") {
                            assert!(
                                definition
                                    .ability_labels
                                    .iter()
                                    .any(|compiled| compiled.starts_with(label)),
                                "{name}: lost {label}: {:?}",
                                definition.ability_labels
                            );
                        }
                    }
                }
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} labeled source failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn labeled_landfall_binds_its_counter_to_the_named_source_after_playing_a_land() {
    use ironsmith::decision::LegalAction;
    use ironsmith::game_state::Phase;
    use ironsmith::object::CounterType;
    use ironsmith::{GameState, PlayerId, Zone};
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture["name"] == "Nissa of Shadowed Boughs")
        .unwrap();
    for renamed in [false, true] {
        let mut fixture = fixture.clone();
        if renamed {
            fixture["name"] = Value::String("Synthetic Walker".into());
            fixture["oracle_text"] = Value::String(
                fixture["oracle_text"]
                    .as_str()
                    .unwrap()
                    .replace("Nissa", "Synthetic Walker"),
            );
        }
        let name = fixture["name"].as_str().unwrap();
        for definition in materialized(name, &source(&fixture)) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let other = compile_to_runtime_definition(
                "Unrelated Walker",
                "Type: Planeswalker — Jace\nLoyalty: 6",
                false,
            )
            .unwrap();
            let bystander = game.create_object_from_definition(&other, alice, Zone::Battlefield);
            let before = game
                .object(source)
                .unwrap()
                .counters
                .get(&CounterType::Loyalty)
                .copied()
                .unwrap_or(0);
            assert_eq!(before, 4, "printed loyalty survives");
            let land = compile_to_runtime_definition(
                "Ordinary Forest",
                "Type: Basic Land — Forest",
                false,
            )
            .unwrap();
            let land_id = game.create_object_from_definition(&land, alice, Zone::Hand);
            perform_action_and_resolve(&mut game, LegalAction::PlayLand { land_id });
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&CounterType::Loyalty),
                Some(&5),
                "{name}: landfall counter binds to its source"
            );
            assert_eq!(
                game.object(bystander)
                    .unwrap()
                    .counters
                    .get(&CounterType::Loyalty),
                Some(&6),
                "{name}: another planeswalker is untouched"
            );
        }
    }
}

#[test]
fn labeled_undergrowth_counts_creature_cards_when_the_source_is_cast() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::decision::LegalAction;
    use ironsmith::game_state::Phase;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::{CardId, CardType, GameState, PlayerId, Subtype, Zone};
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture["name"] == "Izoni, Thousand-Eyed")
        .unwrap();
    for definition in materialized(fixture["name"].as_str().unwrap(), &source(&fixture)) {
        for count in [0, 2] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Black, 3);
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Green, 3);
            for _ in 0..count {
                let dead = CardBuilder::new(CardId::new(), "Graveyard creature")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(1, 1))
                    .build();
                game.create_object_from_card(&dead, alice, Zone::Graveyard);
            }
            let noncreature = CardBuilder::new(CardId::new(), "Graveyard artifact")
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&noncreature, alice, Zone::Graveyard);
            let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
            perform_action_and_resolve(
                &mut game,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                },
            );
            let insects = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| object.has_subtype(Subtype::Insect))
                .collect::<Vec<_>>();
            assert_eq!(insects.len(), count, "undergrowth counts creatures only");
            for insect in insects {
                assert_eq!(game.current_power(insect.id), Some(1));
                assert_eq!(game.current_toughness(insect.id), Some(1));
            }
        }
    }
}

#[test]
fn modal_modes_inherit_named_source_context_without_rewriting_token_names() {
    for fixture in fixtures()
        .into_iter()
        .filter(|fixture| fixture["repair_group"] == "modal_source_context")
    {
        let name = fixture["name"].as_str().unwrap();
        for definition in materialized(name, &source(&fixture)) {
            assert_eq!(
                definition.card.mana_cost.as_ref().unwrap().to_oracle(),
                fixture["mana_cost"].as_str().unwrap()
            );
            assert_eq!(
                definition.card.power_toughness.unwrap().power.to_string(),
                fixture["power"].as_str().unwrap()
            );
            if name == "Koma, Cosmos Serpent" {
                assert!(
                    definition.canonical_text.contains("Koma's Coil"),
                    "created token identity must stay literal"
                );
            }
        }
    }
    for name in ["Mode Keeper", "Counter Custodian, the Patient"] {
        let text = format!(
            "Mana cost: {{3}}\nType: Creature — Human\nPower/Toughness: 2/3\nWhenever {name} attacks, choose one —\n• {name} gains flying until end of turn.\n• {name} gains haste until end of turn."
        );
        materialized(name, &text);
    }
}

struct ChooseIndestructible;
impl ironsmith::decision::DecisionMaker for ChooseIndestructible {
    fn decide_options(
        &mut self,
        game: &ironsmith::GameState,
        context: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        context
            .options
            .iter()
            .find(|option| {
                option.legal
                    && option
                        .description
                        .to_ascii_lowercase()
                        .contains("indestructible")
            })
            .map(|option| vec![option.index])
            .unwrap_or_else(|| {
                ironsmith::decision::SelectFirstDecisionMaker.decide_options(game, context)
            })
    }
}

#[test]
fn named_modal_grants_resolve_on_their_source_and_keep_cost_and_choice_constraints() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_state::Phase;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::{GameState, PlayerId, Zone};
    for fixture in fixtures().into_iter().filter(|fixture| {
        matches!(
            fixture["name"].as_str(),
            Some("Koma, Cosmos Serpent" | "The Vision")
        )
    }) {
        let name = fixture["name"].as_str().unwrap();
        for definition in materialized(name, &source(&fixture)) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            let actor = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let other = compile_to_runtime_definition(
                "Other Serpent",
                "Type: Creature — Serpent\nPower/Toughness: 2/2",
                false,
            )
            .unwrap();
            let bystander = game.create_object_from_definition(&other, alice, Zone::Battlefield);
            let mut chooser = ChooseIndestructible;
            if name == "Koma, Cosmos Serpent" {
                let action = compute_legal_actions(&game, alice).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == actor)).unwrap();
                perform_action_and_resolve_with(&mut game, action, &mut chooser);
                assert!(
                    game.player(alice).unwrap().graveyard.iter().any(|id| game
                        .object(*id)
                        .is_some_and(|object| object.name == "Other Serpent")),
                    "the other-Serpent sacrifice cost must be paid"
                );
                assert!(
                    game.battlefield.contains(&actor),
                    "Koma cannot sacrifice itself to this cost"
                );
            } else {
                let instant = compile_to_runtime_definition(
                    "Mode trigger",
                    "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
                    false,
                )
                .unwrap();
                for repeat in [false, true] {
                    let spell_id = game.create_object_from_definition(&instant, alice, Zone::Hand);
                    perform_action_and_resolve_with(
                        &mut game,
                        LegalAction::CastSpell {
                            spell_id,
                            from_zone: Zone::Hand,
                            casting_method: CastingMethod::Normal,
                        },
                        &mut chooser,
                    );
                    assert!(
                        game.current_has_static_ability_id(actor, StaticAbilityId::Indestructible)
                    );
                    assert_eq!(
                        game.current_has_static_ability_id(actor, StaticAbilityId::DoubleStrike),
                        repeat,
                        "the already-used indestructible mode cannot be selected again this turn"
                    );
                    assert!(
                        !game.current_has_static_ability_id(
                            bystander,
                            StaticAbilityId::Indestructible
                        )
                    );
                }
            }
            assert!(game.current_has_static_ability_id(actor, StaticAbilityId::Indestructible));
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert!(!game.current_has_static_ability_id(actor, StaticAbilityId::Indestructible));
            assert!(!game.current_has_static_ability_id(actor, StaticAbilityId::DoubleStrike));
        }
    }
}

struct ChooseTargetAndAccept(ironsmith::ObjectId);
impl ironsmith::decision::DecisionMaker for ChooseTargetAndAccept {
    fn decide_boolean(
        &mut self,
        _game: &ironsmith::GameState,
        _context: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
    fn decide_targets(
        &mut self,
        _game: &ironsmith::GameState,
        context: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::Target> {
        let target = ironsmith::Target::Object(self.0);
        assert!(
            context
                .requirements
                .iter()
                .any(|requirement| requirement.legal_targets.contains(&target))
        );
        vec![target]
    }
}

#[test]
fn typed_named_source_power_toughness_is_sampled_when_attack_trigger_resolves() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    use ironsmith::game_loop::{
        apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
    };
    use ironsmith::game_state::{Phase, Step};
    use ironsmith::object::CounterType;
    use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
    for fixture in fixtures()
        .into_iter()
        .filter(|fixture| fixture["repair_group"] == "typed_source_pt_copy")
    {
        let name = fixture["name"].as_str().unwrap();
        for definition in materialized(name, &source(&fixture)) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            let actor = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.remove_summoning_sickness(actor);
            let ally = CardBuilder::new(CardId::new(), "Copy recipient")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            let ally = game.create_object_from_card(&ally, alice, Zone::Battlefield);
            let opponent = CardBuilder::new(CardId::new(), "Opponent creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(5, 7))
                .build();
            let opponent = game.create_object_from_card(&opponent, bob, Zone::Battlefield);
            game.add_counters(ally, CounterType::PlusOnePlusOne, 1)
                .unwrap();
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut combat = CombatState::default();
            apply_attacker_declarations(
                &mut game,
                &mut combat,
                &mut queue,
                &[AttackerDeclaration {
                    creature: actor,
                    target: AttackTarget::Player(bob),
                }],
            )
            .unwrap();
            assert_eq!(queue.entries.len(), 1, "{name}: actual attack trigger");
            let mut chooser = ChooseTargetAndAccept(ally);
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut chooser).unwrap();
            assert_eq!(game.stack.len(), 1);
            // The printed 4/4 becomes 6/6 after triggering but before resolution.
            game.add_counters(actor, CounterType::PlusOnePlusOne, 2)
                .unwrap();
            resolve_stack_entry_with(&mut game, &mut chooser).unwrap();
            assert_eq!(game.current_power(actor), Some(6));
            assert_eq!(game.current_toughness(actor), Some(6));
            // Source characteristics are read at resolution; recipient counters
            // are applied after its new base P/T in layer 7c.
            assert_eq!(
                game.current_power(ally),
                Some(7),
                "{name}: source power plus recipient counter"
            );
            assert_eq!(
                game.current_toughness(ally),
                Some(7),
                "{name}: source toughness plus recipient counter"
            );
            assert_eq!(game.current_power(opponent), Some(5));
            assert_eq!(game.current_toughness(opponent), Some(7));
            game.add_counters(actor, CounterType::PlusOnePlusOne, 3)
                .unwrap();
            assert_eq!(
                game.current_power(ally),
                Some(7),
                "{name}: one resolution snapshot, not a live anthem"
            );
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(
                game.current_power(ally),
                Some(3),
                "{name}: base-setting duration expires"
            );
            assert_eq!(game.current_toughness(ally), Some(3));
        }
    }
}

#[test]
fn authored_pronoun_triggers_compile_with_printed_characteristics() {
    for fixture in fixtures()
        .into_iter()
        .filter(|fixture| fixture["repair_group"] == "authored_pronoun_trigger")
    {
        let name = fixture["name"].as_str().unwrap();
        for definition in materialized(name, &source(&fixture)) {
            assert_eq!(
                definition.card.mana_cost.as_ref().unwrap().to_oracle(),
                fixture["mana_cost"].as_str().unwrap()
            );
            let pt = definition.card.power_toughness.unwrap();
            assert_eq!(pt.power.to_string(), fixture["power"].as_str().unwrap());
            assert_eq!(
                pt.toughness.to_string(),
                fixture["toughness"].as_str().unwrap()
            );
        }
    }
}

#[test]
fn authored_connive_pronouns_draw_discard_and_put_the_counter_on_the_source() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::{AttackerDeclaration, LegalAction, SelectFirstDecisionMaker};
    use ironsmith::game_loop::{
        apply_attacker_declarations, execute_combat_damage_step, put_triggers_on_stack_with_dm,
        queue_combat_damage_triggers, resolve_stack_entry_with,
    };
    use ironsmith::game_state::{Phase, Step};
    use ironsmith::mana::ManaSymbol;
    use ironsmith::object::CounterType;
    use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
    for fixture in fixtures().into_iter().filter(|fixture| {
        matches!(
            fixture["name"].as_str(),
            Some("Madame Masque" | "Tiger Shark, Abyssal Hunter" | "Norman Osborn")
        )
    }) {
        let name = fixture["name"].as_str().unwrap();
        for definition in materialized(name, &source(&fixture)) {
            for discard_nonland in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                game.turn.active_player = alice;
                game.turn.priority_player = Some(alice);
                game.turn.phase = Phase::FirstMain;
                game.turn.step = None;
                for symbol in [ManaSymbol::Black, ManaSymbol::Blue] {
                    game.player_mut(alice).unwrap().mana_pool.add(symbol, 10);
                }
                let drawn = CardBuilder::new(CardId::new(), "Connive draw")
                    .card_types(vec![if discard_nonland {
                        CardType::Artifact
                    } else {
                        CardType::Land
                    }])
                    .build();
                game.create_object_from_card(&drawn, alice, Zone::Library);
                let other = CardBuilder::new(CardId::new(), "Untouched creature")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .build();
                let bystander = game.create_object_from_card(&other, alice, Zone::Battlefield);
                let actor;
                if name == "Norman Osborn" {
                    actor =
                        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                    game.remove_summoning_sickness(actor);
                    game.turn.phase = Phase::Combat;
                    game.turn.step = Some(Step::DeclareAttackers);
                    let mut combat = CombatState::default();
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    apply_attacker_declarations(
                        &mut game,
                        &mut combat,
                        &mut queue,
                        &[AttackerDeclaration {
                            creature: actor,
                            target: AttackTarget::Player(bob),
                        }],
                    )
                    .unwrap();
                    assert!(
                        queue.entries.is_empty(),
                        "Norman triggers on damage, not declaring an attack"
                    );
                    let events = execute_combat_damage_step(&mut game, &combat, false);
                    assert_eq!(game.player(bob).unwrap().life, 19);
                    queue_combat_damage_triggers(&mut game, &events, &mut queue);
                    assert_eq!(queue.entries.len(), 1);
                    let mut chooser = SelectFirstDecisionMaker;
                    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut chooser).unwrap();
                    assert_eq!(game.stack.len(), 1);
                    resolve_stack_entry_with(&mut game, &mut chooser).unwrap();
                } else {
                    let spell_id =
                        game.create_object_from_definition(&definition, alice, Zone::Hand);
                    perform_action_and_resolve(
                        &mut game,
                        LegalAction::CastSpell {
                            spell_id,
                            from_zone: Zone::Hand,
                            casting_method: CastingMethod::Normal,
                        },
                    );
                    actor = *game
                        .battlefield
                        .iter()
                        .find(|id| game.object(**id).is_some_and(|object| object.name == name))
                        .unwrap();
                }
                assert!(
                    game.player(alice).unwrap().hand.is_empty(),
                    "{name}: drawn card was discarded"
                );
                assert!(
                    game.player(alice).unwrap().library.is_empty(),
                    "{name}: one card was drawn"
                );
                assert_eq!(
                    game.player(alice).unwrap().graveyard.len(),
                    1,
                    "{name}: one discard"
                );
                assert_eq!(
                    game.object(actor)
                        .unwrap()
                        .counters
                        .get(&CounterType::PlusOnePlusOne)
                        .copied()
                        .unwrap_or(0),
                    u32::from(discard_nonland),
                    "{name}: connive only rewards a nonland discard"
                );
                assert_eq!(
                    game.current_power(bystander),
                    Some(2),
                    "{name}: pronoun must name the trigger source"
                );
            }
        }
    }
}

#[test]
fn labeled_personal_damage_pronoun_keeps_both_damage_recipients() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::decision::LegalAction;
    use ironsmith::game_state::Phase;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture["name"] == "Shocker, Unshakable")
        .unwrap();
    for definition in materialized(fixture["name"].as_str().unwrap(), &source(&fixture)) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 6);
        let target = CardBuilder::new(CardId::new(), "Damage recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 5))
            .build();
        let target = game.create_object_from_card(&target, bob, Zone::Battlefield);
        let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
        perform_action_and_resolve_with(
            &mut game,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            },
            &mut ChooseTargetAndAccept(target),
        );
        assert_eq!(game.damage_on(target), 2);
        assert_eq!(game.player(bob).unwrap().life, 18);
        assert_eq!(game.player(alice).unwrap().life, 20);
    }
}

#[test]
fn named_subtype_alias_does_not_replace_the_transformation_descriptor() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::decision::LegalAction;
    use ironsmith::game_state::Phase;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::{GameState, PlayerId, Subtype, Zone};
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture["name"] == "Lizard, Connors's Curse")
        .unwrap();
    for definition in materialized(fixture["name"].as_str().unwrap(), &source(&fixture)) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        for symbol in [ManaSymbol::Blue, ManaSymbol::Green] {
            game.player_mut(alice).unwrap().mana_pool.add(symbol, 10);
        }
        let other = compile_to_runtime_definition(
            "Transform recipient",
            "Type: Creature — Bird\nPower/Toughness: 2/2\nFlying",
            false,
        )
        .unwrap();
        let target = game.create_object_from_definition(&other, bob, Zone::Battlefield);
        let untouched = game.create_object_from_definition(&other, bob, Zone::Battlefield);
        let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
        perform_action_and_resolve_with(
            &mut game,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            },
            &mut ChooseTargetAndAccept(target),
        );
        assert_eq!(game.current_power(target), Some(4));
        assert_eq!(game.current_toughness(target), Some(4));
        assert!(
            game.calculated_characteristics(target)
                .unwrap()
                .subtypes
                .contains(&Subtype::Lizard)
        );
        assert!(!game.current_has_static_ability_id(target, StaticAbilityId::Flying));
        assert_eq!(game.current_power(untouched), Some(2));
        assert!(game.current_has_static_ability_id(untouched, StaticAbilityId::Flying));
    }
}

#[test]
fn named_loyalty_copy_is_source_scoped_and_tracks_other_planeswalkers() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::{GameState, ObjectId, PlayerId, Zone};
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture["name"] == "Nicol Bolas, Dragon-God")
        .unwrap();
    let loyalty_count = |game: &GameState, id: ObjectId| {
        game.current_abilities(id).unwrap().iter().filter(|ability| matches!(&ability.kind, AbilityKind::Activated(activated) if activated.is_loyalty_ability)).count()
    };
    for renamed in [false, true] {
        let mut fixture = fixture.clone();
        if renamed {
            fixture["name"] = Value::String("Synthetic Magistrate".into());
            fixture["oracle_text"] = Value::String(
                fixture["oracle_text"]
                    .as_str()
                    .unwrap()
                    .replace("Nicol Bolas", "Synthetic Magistrate"),
            );
        }
        for definition in materialized(fixture["name"].as_str().unwrap(), &source(&fixture)) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let actor = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let bystander = compile_to_runtime_definition(
                "Unrelated Bolas",
                "Type: Planeswalker — Bolas\nLoyalty: 3",
                false,
            )
            .unwrap();
            let bystander =
                game.create_object_from_definition(&bystander, alice, Zone::Battlefield);
            assert_eq!(loyalty_count(&game, actor), 3);
            let donor = compile_to_runtime_definition(
                "Ability donor",
                "Type: Planeswalker — Jace\nLoyalty: 3\n+1: You gain 7 life.",
                false,
            )
            .unwrap();
            let donor = game.create_object_from_definition(&donor, bob, Zone::Battlefield);
            assert_eq!(
                loyalty_count(&game, actor),
                4,
                "the named source borrows the opponent's loyalty ability"
            );
            assert_eq!(
                loyalty_count(&game, bystander),
                0,
                "Bolas is not a recovered recipient subtype"
            );
            game.move_object_by_effect(donor, Zone::Graveyard).unwrap();
            assert_eq!(loyalty_count(&game, actor), 3);
        }
    }
}
