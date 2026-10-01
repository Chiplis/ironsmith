//! Strict canonical spells whose later effects read targets, costs, or earlier outcomes.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    sacrifice: usize,
    accept_optional: bool,
    prefer_alternate: bool,
    x: u32,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        let selected = if ctx.is_x_value {
            self.x.clamp(ctx.min, ctx.max)
        } else {
            ctx.min
        };
        self.trace.push(json!({"choice":"number","description":ctx.description,"min":ctx.min,"max":ctx.max,"selected":selected,"is_x":ctx.is_x_value}));
        selected
    }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        let selected = self.sacrifice > 0 || self.accept_optional;
        self.trace
            .push(json!({"choice":"boolean","description":ctx.description,"selected":selected}));
        selected
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut result = Vec::new();
        for requirement in &ctx.requirements {
            let chosen: Vec<_> = self
                .targets
                .iter()
                .copied()
                .filter(|t| requirement.legal_targets.contains(t))
                .take(requirement.max_targets.unwrap_or(self.targets.len()))
                .collect();
            if chosen.len() < requirement.min_targets {
                return SelectFirstDecisionMaker.decide_targets(game, ctx);
            }
            result.extend(chosen);
        }
        self.trace.push(json!({"choice":"targets","requirements":format!("{:?}",ctx.requirements),"selected":format!("{result:?}")}));
        result
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let result: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .take(
                self.sacrifice
                    .max(ctx.min)
                    .min(ctx.max.unwrap_or(usize::MAX)),
            )
            .map(|c| c.id)
            .collect();
        self.trace.push(json!({"choice":"objects","description":ctx.description,"minimum":ctx.min,"maximum":ctx.max,"selected":result.iter().map(|id|format!("{id:?}")).collect::<Vec<_>>()}));
        result
    }
}
fn setup() -> GameState {
    setup_players(2)
}
fn setup_players(players: usize) -> GameState {
    let mut game = GameState::new(
        ["Alice", "Bob", "Cara"][..players]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        20,
    );
    game.set_random_seed(71757432704846);
    game.turn.turn_number = 3;
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(symbol, 12);
    }
    game
}
fn creature(power: i32, spirit: bool) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Reference creature")
        .card_types(vec![CardType::Creature])
        .subtypes(if spirit {
            vec![Subtype::Spirit]
        } else {
            vec![]
        })
        .power_toughness(ironsmith::PowerToughness::fixed(power, 6))
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
        .build()
}
fn announce(
    game: &mut GameState,
    source: ObjectId,
    activate: bool,
    dm: &mut Choices,
) -> Result<Value, String> {
    let action = compute_legal_actions(game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| {
            if activate {
                matches!(a,LegalAction::ActivateAbility{source:id,..} if *id==source)
            } else {
                matches!(a,LegalAction::CastSpell{spell_id,casting_method,..} if *spell_id==source && (!dm.prefer_alternate || casting_method.is_alternative()))
            }
        })
        .ok_or("no legal requested action")?;
    let selected = format!("{action:?}");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_cast.is_none()
            && state.pending_activation.is_none()
            && !game.stack.is_empty()
        {
            return Ok(
                json!({"action":selected,"mana_spent":72-game.player(PlayerId(0)).unwrap().mana_pool.total(),"stack_targets":format!("{:?}",game.stack.last().unwrap().targets),"choices":dm.trace}),
            );
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("announcement budget".into())
}
fn finish(game: &mut GameState, dm: &mut Choices) -> Result<(), String> {
    let mut queue = TriggerQueue::new();
    for _ in 0..32 {
        advance_priority_with_dm(game, &mut queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
    }
    Err("resolution budget".into())
}
fn power(def: &CardDefinition, p: i32) -> Result<(Value, Value), String> {
    let mut game = setup();
    let target =
        game.create_object_from_definition(&creature(p, false), PlayerId(0), Zone::Battlefield);
    let activated = def.name() == "Wine of Blood and Iron";
    let source = game.create_object_from_definition(
        def,
        PlayerId(0),
        if activated {
            Zone::Battlefield
        } else {
            Zone::Hand
        },
    );
    let mut dm = Choices {
        targets: vec![Target::Object(target)],
        ..Default::default()
    };
    let evidence = announce(&mut game, source, activated, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    Ok((
        json!({"resolution_error":error,"target_power":game.calculated_power(target),"target_toughness":game.calculated_toughness(target)}),
        evidence,
    ))
}
fn greed(def: &CardDefinition, sacrificed: usize) -> Result<(Value, Value), String> {
    let mut game = setup();
    let spirit = creature(2, true);
    for _ in 0..3 {
        game.create_object_from_definition(&spirit, PlayerId(0), Zone::Battlefield);
    }
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = Choices {
        targets: vec![Target::Player(PlayerId(1))],
        sacrifice: sacrificed,
        ..Default::default()
    };
    let mut evidence = announce(&mut game, source, false, &mut dm)?;
    let paid = 3 - game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|o| o.name == "Reference creature")
        })
        .count();
    evidence["spirits_sacrificed_during_announcement"] = json!(paid);
    if paid != sacrificed {
        return Err(format!(
            "requested sacrifice{sacrificed}, paid{paid};{evidence}"
        ));
    }
    let error = finish(&mut game, &mut dm).err();
    Ok((
        json!({"resolution_error":error,"alice_life":game.player(PlayerId(0)).unwrap().life,"bob_life":game.player(PlayerId(1)).unwrap().life}),
        evidence,
    ))
}
fn rebirth(def: &CardDefinition, n: usize) -> Result<(Value, Value), String> {
    let mut game = setup();
    let token = creature(2, false);
    for i in 0..n {
        game.create_object_from_definition(&token, PlayerId((i % 2) as u8), Zone::Battlefield);
    }
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = Choices::default();
    let evidence = announce(&mut game, source, false, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    let horrors:Vec<_>=game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|o|o.kind==ironsmith::object::ObjectKind::Token&&o.subtypes.contains(&Subtype::Horror))).map(|id|json!({"power":game.calculated_power(*id),"toughness":game.calculated_toughness(*id)})).collect();
    let remaining = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|o| o.name == "Reference creature")
        })
        .count();
    Ok((
        json!({"resolution_error":error,"original_creatures_remaining":remaining,"living_horrors":horrors}),
        evidence,
    ))
}
fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: Value,
    expected: Value,
    result: Result<(Value, Value), String>,
) {
    let (status, actual, evidence) = match result {
        Ok((actual, evidence)) => (
            if actual == expected {
                "expected_result_observed"
            } else if !actual["resolution_error"].is_null() {
                "resolution_failed"
            } else {
                "semantic_mismatch"
            },
            actual,
            evidence,
        ),
        Err(error) => (
            "fixture_or_announcement_error",
            json!({"error":error}),
            Value::Null,
        ),
    };
    rows.push(json!({"card":card,"scenario":scenario,"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "report generation only"]
fn report_reference_values() {
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Onward",
        "Rush of Blood",
        "Wine of Blood and Iron",
        "Devouring Greed",
        "Phyrexian Rebirth",
        "Brute Force",
    ];
    let mut defs = HashMap::new();
    let mut artifacts = Vec::new();
    for payload in input["cards"].as_array().unwrap() {
        let name = payload["name"].as_str().unwrap();
        if !names.contains(&name) {
            continue;
        }
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            payload["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum}));
        defs.insert(name.to_string(), def);
    }
    let binary = std::env::current_exe().unwrap();
    let before = hash(&binary);
    let mut rows = Vec::new();
    for name in ["Onward", "Rush of Blood", "Wine of Blood and Iron"] {
        for p in [0, 2, 5] {
            record(
                &mut rows,
                name,
                json!({"target_power":p}),
                json!({"resolution_error":null,"target_power":p*2,"target_toughness":6}),
                power(&defs[name], p),
            );
        }
    }
    for n in [0, 1, 3] {
        record(
            &mut rows,
            "Devouring Greed",
            json!({"spirits_sacrificed":n}),
            json!({"resolution_error":null,"alice_life":22+2*n,"bob_life":18-2*n}),
            greed(&defs["Devouring Greed"], n),
        );
    }
    for p in [0, 2, 5] {
        record(
            &mut rows,
            "Brute Force",
            json!({"target_power":p,"role":"positive targeted-pump control"}),
            json!({"resolution_error":null,"target_power":p+3,"target_toughness":9}),
            power(&defs["Brute Force"], p),
        );
    }
    for n in [0, 1, 3] {
        record(
            &mut rows,
            "Phyrexian Rebirth",
            json!({"creatures_destroyed":n}),
            json!({"resolution_error":null,"original_creatures_remaining":0,"living_horrors":if n==0{vec![]}else{vec![json!({"power":n,"toughness":n})]}}),
            rebirth(&defs["Phyrexian Rebirth"], n),
        );
    }
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(inventory.parent().unwrap().join("manifest.json")).unwrap(),
    )
    .unwrap();
    let report = json!({"scope":"Strict canonical artifact, explicit legal targets/cost choices, paid engine action, normal priority/SBA boundary. Onward is a standalone face, not linked split-card transitions.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":before,"binary_unchanged":before==hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory),"cards_sha256":manifest["cards_sha256"]}});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/reference-value-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

fn rite(def: &CardDefinition, counters: u32) -> Result<(Value, Value), String> {
    let mut game = setup();
    let target =
        game.create_object_from_definition(&creature(2, false), PlayerId(1), Zone::Battlefield);
    game.add_counters(target, ironsmith::CounterType::PlusOnePlusOne, counters);
    let stable = game.object(target).unwrap().stable_id;
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = Choices {
        targets: vec![Target::Object(target)],
        ..Default::default()
    };
    let evidence = announce(&mut game, source, false, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    let zone = game
        .find_object_by_stable_id(stable)
        .and_then(|id| game.object(id))
        .map(|o| format!("{:?}", o.zone));
    let snakes = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id).is_some_and(|o| {
                o.kind == ironsmith::object::ObjectKind::Token
                    && o.subtypes.contains(&Subtype::Snake)
            })
        })
        .count();
    Ok((
        json!({"resolution_error":error,"target_zone":zone,"snake_tokens":snakes}),
        evidence,
    ))
}
fn lisette(
    def: &CardDefinition,
    revitalize: &CardDefinition,
    accept: bool,
) -> Result<(Value, Value), String> {
    let mut game = setup();
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    let friend =
        game.create_object_from_definition(&creature(2, false), PlayerId(0), Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Life trigger library")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..8 {
        game.create_object_from_definition(&filler, PlayerId(0), Zone::Library);
    }
    let spell = game.create_object_from_definition(revitalize, PlayerId(0), Zone::Hand);
    let mut dm = Choices {
        accept_optional: accept,
        ..Default::default()
    };
    let mut evidence = announce(&mut game, spell, false, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    evidence["resolution_choices"] = json!(dm.trace);
    evidence["total_mana_spent"] = json!(72 - game.player(PlayerId(0)).unwrap().mana_pool.total());
    let counters =
        [source, friend].map(|id| game.counter_count(id, ironsmith::CounterType::PlusOnePlusOne));
    let trample = [source, friend].map(|id| {
        game.object_has_static_ability_id(id, ironsmith::static_abilities::StaticAbilityId::Trample)
    });
    Ok((
        json!({"resolution_error":error,"life":game.player(PlayerId(0)).unwrap().life,"cards_drawn":game.player(PlayerId(0)).unwrap().hand.len(),"creature_counters":counters,"creatures_have_trample":trample}),
        evidence,
    ))
}
#[test]
#[ignore = "report generation only"]
fn report_trigger_value_context() {
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Rite of the Serpent",
        "Lisette, Dean of the Root",
        "Revitalize",
    ];
    let mut defs = HashMap::new();
    let mut artifacts = Vec::new();
    for payload in input["cards"].as_array().unwrap() {
        let name = payload["name"].as_str().unwrap();
        if !names.contains(&name) {
            continue;
        }
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            payload["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum}));
        defs.insert(name.to_string(), def);
    }
    let binary = std::env::current_exe().unwrap();
    let before = hash(&binary);
    let mut rows = Vec::new();
    for counters in [0, 1, 3] {
        record(
            &mut rows,
            "Rite of the Serpent",
            json!({"target_plus_one_counters":counters}),
            json!({"resolution_error":null,"target_zone":"Graveyard","snake_tokens":usize::from(counters>0)}),
            rite(&defs["Rite of the Serpent"], counters),
        );
    }
    for accept in [false, true] {
        record(
            &mut rows,
            "Lisette, Dean of the Root",
            json!({"accept_pay_one":accept,"life_source":"actual paid Revitalize"}),
            json!({"resolution_error":null,"life":23,"cards_drawn":1,"creature_counters":[u32::from(accept),u32::from(accept)],"creatures_have_trample":[accept,accept]}),
            lisette(
                &defs["Lisette, Dean of the Root"],
                &defs["Revitalize"],
                accept,
            ),
        );
    }
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(inventory.parent().unwrap().join("manifest.json")).unwrap(),
    )
    .unwrap();
    let report = json!({"scope":"Strict canonical artifact; actual paid targeted destroy and actual paid lifegain spell; triggered ability payment with normal priority/SBA boundary. Lisette tested as standalone face.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":before,"binary_unchanged":before==hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory),"cards_sha256":manifest["cards_sha256"]}});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/trigger-context-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

fn ebony(def: &CardDefinition, roll: u32, accept: bool) -> Result<(Value, Value), String> {
    let mut game = setup();
    game.force_next_die_roll(roll);
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    let mut dm = Choices {
        accept_optional: accept,
        ..Default::default()
    };
    let mut evidence = announce(&mut game, source, true, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    evidence["resolution_choices"] = json!(dm.trace);
    evidence["recorded_die_rolls"] = json!(
        game.turn_store
            .turn_history
            .die_rolls_this_turn
            .get(&PlayerId(0))
    );
    Ok((
        json!({"resolution_error":error,"power":game.calculated_power(source),"toughness":game.calculated_toughness(source),"flying":game.object_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Flying)}),
        evidence,
    ))
}
fn prairie(def: &CardDefinition, n: u32, activate: bool) -> Result<(Value, Value), String> {
    let mut game = setup();
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    let target =
        game.create_object_from_definition(&creature(2, false), PlayerId(0), Zone::Battlefield);
    let mut dm = Choices::default();
    let evidence = if activate {
        announce(&mut game, source, true, &mut dm)?
    } else {
        json!({"control":"no replacement activation"})
    };
    let mut error = finish(&mut game, &mut dm).err();
    let followed = error.is_none();
    if followed {
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(
            source,
            PlayerId(0),
            vec![ironsmith::Effect::put_counters(
                ironsmith::CounterType::PlusOnePlusOne,
                n as i32,
                ironsmith::target::ChooseSpec::SpecificObject(target),
            )],
        ));
        error = finish(&mut game, &mut dm).err();
    }
    Ok((
        json!({"resolution_error":error,"counter_effect_executed":followed,"target_counters":game.counter_count(target,ironsmith::CounterType::PlusOnePlusOne)}),
        evidence,
    ))
}
fn opportunity(def: &CardDefinition, foods: usize, accept: bool) -> Result<(Value, Value), String> {
    let mut game = setup();
    let food = CardDefinitionBuilder::new(CardId::new(), "Sacrificable Food fixture")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Food])
        .build();
    for _ in 0..foods {
        game.create_object_from_definition(&food, PlayerId(0), Zone::Battlefield);
    }
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = Choices {
        sacrifice: if accept { 2 } else { 0 },
        accept_optional: accept,
        ..Default::default()
    };
    let mut evidence = announce(&mut game, source, false, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    evidence["resolution_choices"] = json!(dm.trace);
    let subtype_count = |subtype| {
        game.battlefield
            .iter()
            .filter(|id| {
                game.object(**id).is_some_and(|o| {
                    o.kind == ironsmith::object::ObjectKind::Token && o.subtypes.contains(&subtype)
                })
            })
            .count()
    };
    let remaining = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|o| o.name == "Sacrificable Food fixture")
        })
        .count();
    Ok((
        json!({"resolution_error":error,"initial_foods_remaining":remaining,"food_tokens_created":subtype_count(Subtype::Food),"giant_tokens_created":subtype_count(Subtype::Giant)}),
        evidence,
    ))
}
#[test]
#[ignore = "report generation only"]
fn report_misc_value_context() {
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = ["Ebony Fly", "Prairie Dog", "Giant Opportunity"];
    let mut defs = HashMap::new();
    let mut artifacts = Vec::new();
    for payload in input["cards"].as_array().unwrap() {
        let name = payload["name"].as_str().unwrap();
        if !names.contains(&name) {
            continue;
        }
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            payload["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum}));
        defs.insert(name.to_string(), def);
    }
    let binary = std::env::current_exe().unwrap();
    let before = hash(&binary);
    let mut rows = Vec::new();
    for (roll, accept) in [(1, false), (1, true), (6, true)] {
        record(
            &mut rows,
            "Ebony Fly",
            json!({"forced_legal_roll":roll,"accept_animation":accept}),
            json!({"resolution_error":null,"power":if accept{Some(roll)}else{None},"toughness":if accept{Some(roll)}else{None},"flying":accept}),
            ebony(&defs["Ebony Fly"], roll, accept),
        );
    }
    for (n, activate) in [(1, false), (3, false), (1, true), (3, true)] {
        record(
            &mut rows,
            "Prairie Dog",
            json!({"counters_to_put":n,"activate_replacement":activate}),
            json!({"resolution_error":null,"counter_effect_executed":true,"target_counters":n+u32::from(activate)}),
            prairie(&defs["Prairie Dog"], n, activate),
        );
    }
    for (foods, accept) in [(0, false), (1, false), (2, false), (2, true)] {
        record(
            &mut rows,
            "Giant Opportunity",
            json!({"foods":foods,"accept_sacrifice_two":accept}),
            json!({"resolution_error":null,"initial_foods_remaining":foods-if accept{2}else{0},"food_tokens_created":if accept{0}else{3},"giant_tokens_created":usize::from(accept)}),
            opportunity(&defs["Giant Opportunity"], foods, accept),
        );
    }
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(inventory.parent().unwrap().join("manifest.json")).unwrap(),
    )
    .unwrap();
    let report = json!({"scope":"Strict canonical artifact; normal paid cast/activation with explicit optional decisions. Die fixture fixes legitimate outcomes1/6 while ordinary die effect emits events. Counter control uses ordinary PutCountersEffect; source ETBs/other turns not covered.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":before,"binary_unchanged":before==hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory),"cards_sha256":manifest["cards_sha256"]}});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/misc-value-context-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

fn evershrike(
    def: &CardDefinition,
    aura: &CardDefinition,
    x: u32,
    accept: bool,
    has_aura: bool,
) -> Result<(Value, Value), String> {
    let mut game = setup();
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Graveyard);
    let stable = game.object(source).unwrap().stable_id;
    let aura_stable = if has_aura {
        let id = game.create_object_from_definition(aura, PlayerId(0), Zone::Hand);
        Some(game.object(id).unwrap().stable_id)
    } else {
        None
    };
    let mut dm = Choices {
        x,
        accept_optional: accept,
        sacrifice: usize::from(accept),
        ..Default::default()
    };
    let mut evidence = announce(&mut game, source, true, &mut dm)?;
    if evidence["mana_spent"] != json!(x + 2) {
        return Err(format!("wrong X payment:{evidence}"));
    }
    let error = finish(&mut game, &mut dm).err();
    evidence["resolution_choices"] = json!(dm.trace);
    let current = game
        .find_object_by_stable_id(stable)
        .ok_or("missing source")?;
    let attachment = aura_stable
        .and_then(|s| game.find_object_by_stable_id(s))
        .and_then(|id| game.object(id))
        .is_some_and(|o| {
            o.zone == Zone::Battlefield
                && o.attached_to == Some(ironsmith::object::AttachmentTarget::Object(current))
        });
    Ok((
        json!({"resolution_error":error,"source_zone":format!("{:?}",game.object(current).unwrap().zone),"aura_attached":attachment}),
        evidence,
    ))
}
fn fill_player_zones(game: &mut GameState, player: PlayerId) {
    let filler = CardDefinitionBuilder::new(CardId::new(), "Followup zone filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..4 {
        game.create_object_from_definition(&filler, player, Zone::Hand);
    }
    for _ in 0..16 {
        game.create_object_from_definition(&filler, player, Zone::Library);
    }
}
fn mastery(
    def: &CardDefinition,
    alternate: bool,
    players: usize,
) -> Result<(Value, Value), String> {
    let mut game = setup_players(players);
    for player in (0..players).map(|i| PlayerId(i as u8)) {
        fill_player_zones(&mut game, player);
    }
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = Choices {
        prefer_alternate: alternate,
        sacrifice: 3,
        ..Default::default()
    };
    let mut evidence = announce(&mut game, source, false, &mut dm)?;
    if evidence["mana_spent"] != json!(if alternate { 4 } else { 5 }) {
        return Err(format!("wrong casting cost:{evidence}"));
    }
    let error = finish(&mut game, &mut dm).err();
    evidence["resolution_choices"] = json!(dm.trace);
    Ok((
        json!({"resolution_error":error,"alice_hand":game.player(PlayerId(0)).unwrap().hand.len(),"alice_library":game.player(PlayerId(0)).unwrap().library.len(),"bob_hand":game.player(PlayerId(1)).unwrap().hand.len()}),
        evidence,
    ))
}
fn intimations(def: &CardDefinition) -> Result<(Value, Value), String> {
    let mut game = setup();
    for player in [PlayerId(0), PlayerId(1)] {
        fill_player_zones(&mut game, player);
    }
    game.create_object_from_definition(&creature(2, false), PlayerId(1), Zone::Battlefield);
    game.create_object_from_definition(&creature(2, false), PlayerId(0), Zone::Graveyard);
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = Choices::default();
    let mut evidence = announce(&mut game, source, false, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    evidence["resolution_choices"] = json!(dm.trace);
    let bob_creatures = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id).is_some_and(|o| {
                game.controller_of(o) == PlayerId(1) && o.card_types.contains(&CardType::Creature)
            })
        })
        .count();
    Ok((
        json!({"resolution_error":error,"alice_hand":game.player(PlayerId(0)).unwrap().hand.len(),"bob_hand":game.player(PlayerId(1)).unwrap().hand.len(),"bob_creatures":bob_creatures}),
        evidence,
    ))
}
fn ritual(def: &CardDefinition) -> Result<(Value, Value), String> {
    let mut game = setup();
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Ritual nontoken permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_definition(&artifact, PlayerId(0), Zone::Battlefield);
    game.create_object_from_definition(&creature(2, false), PlayerId(1), Zone::Battlefield);
    game.create_object_from_definition(&artifact, PlayerId(1), Zone::Hand);
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = Choices {
        targets: vec![Target::Player(PlayerId(1))],
        ..Default::default()
    };
    let mut evidence = announce(&mut game, source, false, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    evidence["resolution_choices"] = json!(dm.trace);
    evidence["bob_life_after_attempt"] = json!(game.player(PlayerId(1)).unwrap().life);
    evidence["alice_permanents_after_attempt"] = json!(
        game.battlefield
            .iter()
            .filter(|id| game
                .object(**id)
                .is_some_and(|o| game.controller_of(o) == PlayerId(0)))
            .count()
    );
    Ok((json!({"resolution_error":error}), evidence))
}
#[test]
#[ignore = "report generation only"]
fn report_followup_context() {
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Evershrike",
        "Holy Strength",
        "Fervent Mastery",
        "Dark Intimations",
        "Forbidden Ritual",
    ];
    let mut defs = HashMap::new();
    let mut artifacts = Vec::new();
    for payload in input["cards"].as_array().unwrap() {
        let name = payload["name"].as_str().unwrap();
        if !names.contains(&name) {
            continue;
        }
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            payload["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum}));
        defs.insert(name.to_string(), def);
    }
    let binary = std::env::current_exe().unwrap();
    let before = hash(&binary);
    let mut rows = Vec::new();
    for (x, accept, has_aura) in [(0, false, false), (1, false, true), (1, true, true)] {
        record(
            &mut rows,
            "Evershrike",
            json!({"paid_x":x,"accept_aura":accept,"holy_strength_in_hand":has_aura}),
            json!({"resolution_error":null,"source_zone":if accept{"Battlefield"}else{"Exile"},"aura_attached":accept}),
            evershrike(
                &defs["Evershrike"],
                &defs["Holy Strength"],
                x,
                accept,
                has_aura,
            ),
        );
    }
    for (alternate, players) in [(false, 2), (true, 2), (false, 3), (true, 3)] {
        record(
            &mut rows,
            "Fervent Mastery",
            json!({"alternate_cost":alternate,"search_count":3,"players":players}),
            json!({"resolution_error":null,"alice_hand":4,"alice_library":13,"bob_hand":4}),
            mastery(&defs["Fervent Mastery"], alternate, players),
        );
    }
    record(
        &mut rows,
        "Dark Intimations",
        json!({"opponent_creature":true,"caster_graveyard_creature":true,"both_hands":4,"both_libraries":16}),
        json!({"resolution_error":null,"alice_hand":6,"bob_hand":3,"bob_creatures":0}),
        intimations(&defs["Dark Intimations"]),
    );
    record(
        &mut rows,
        "Forbidden Ritual",
        json!({"caster_nontoken_permanent":true,"target_opponent_has_permanent_and_card":true,"decline_repeat":true}),
        json!({"resolution_error":null}),
        ritual(&defs["Forbidden Ritual"]),
    );
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(inventory.parent().unwrap().join("manifest.json")).unwrap(),
    )
    .unwrap();
    let report = json!({"scope":"Strict canonical artifact, ordinary paid normal/alternative casts or graveyard activation, explicit X and targets, valid optional resources. Ritual checks dispatcher completion only.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":before,"binary_unchanged":before==hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory),"cards_sha256":manifest["cards_sha256"]}});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/followup-context-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

fn ravnica_color_gate(
    def: &CardDefinition,
    colors: usize,
    own: bool,
    land: bool,
) -> Result<(Value, Value), String> {
    use ironsmith::color::ColorSet;
    let mut game = setup();
    let color_set = match colors {
        0 => ColorSet::COLORLESS,
        1 => ColorSet::WHITE,
        2 => ColorSet::WHITE.union(ColorSet::BLUE),
        _ => ColorSet::WHITE.union(ColorSet::BLUE).union(ColorSet::BLACK),
    };
    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Ravnica color boundary target")
        .card_types(vec![if land {
            CardType::Land
        } else {
            CardType::Creature
        }])
        .color_indicator(color_set);
    if !land {
        builder = builder.power_toughness(ironsmith::PowerToughness::fixed(2, 6));
    }
    let target = game.create_object_from_definition(
        &builder.build(),
        PlayerId(if own { 0 } else { 1 }),
        Zone::Battlefield,
    );
    let stable = game.object(target).unwrap().stable_id;
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = Choices {
        targets: vec![Target::Object(target)],
        ..Default::default()
    };
    let mut evidence = announce(&mut game, source, false, &mut dm)?;
    let error = finish(&mut game, &mut dm).err();
    evidence["resolution_choices"] = json!(dm.trace);
    let current = game
        .find_object_by_stable_id(stable)
        .ok_or("boundary permanent disappeared")?;
    Ok((
        json!({"resolution_error":error,"target_zone":format!("{:?}",game.object(current).unwrap().zone)}),
        evidence,
    ))
}
#[test]
#[ignore = "report generator, not a card correctness test"]
fn report_numeric_color_gate() {
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let payload = input["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "Invasion of Ravnica")
        .unwrap();
    let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "Invasion of Ravnica"),
        payload["parse_input"].as_str().unwrap(),
        false,
    )
    .unwrap();
    let mut rows = Vec::new();
    for (colors, own, land) in [
        (0, false, false),
        (1, false, false),
        (2, false, false),
        (3, false, false),
        (1, true, false),
        (1, false, true),
    ] {
        record(
            &mut rows,
            "Invasion of Ravnica",
            json!({"target_color_count":colors,"own_permanent":own,"land":land,"method":"actual paid cast and generated ETB targets; siege defeat/transform outside scope"}),
            json!({"resolution_error":null,"target_zone":if colors!=2 && !own && !land{"Exile"}else{"Battlefield"}}),
            ravnica_color_gate(&def, colors, own, land),
        );
    }
    let binary = std::env::current_exe().unwrap();
    let report = json!({"scope":"Strict artifact actual paid Siege cast, generated ETB and color/owner/type target boundaries. Other abilities and linked transition remain outside scope.","rows":rows,"artifacts":[{"card":"Invasion of Ravnica","artifact_checksum":artifact.payload_checksum}],"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory)}});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/numeric-color-gate-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
