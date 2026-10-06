//! Paid advertised activations across turn-conditioned static ability grants.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
struct Choices {
    accept: bool,
    target: Option<Target>,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"decision":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let a = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace
            .push(json!({"decision":"options","context":format!("{c:?}"),"answer":a}));
        a
    }
    fn decide_objects(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let a = SelectFirstDecisionMaker.decide_objects(g, c);
        self.trace.push(
            json!({"decision":"objects","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
        a
    }
    fn decide_targets(&mut self, _g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let a = c
            .requirements
            .iter()
            .filter_map(|r| {
                self.target
                    .filter(|t| r.legal_targets.contains(t))
                    .or_else(|| r.legal_targets.first().copied())
            })
            .collect::<Vec<_>>();
        self.trace.push(
            json!({"decision":"targets","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
        a
    }
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: PlayerId,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<u32, String> {
    eprintln!("AUDIT_STAGE cast {}", def.name());
    g.turn.priority_player = Some(actor);
    let id = g.create_object_from_definition(def, actor, Zone::Hand);
    let action = compute_legal_actions(g, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or_else(|| format!("{} normal cast unavailable", def.name()))?;
    let before = g.player(actor).unwrap().mana_pool.total();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace.push(json!({"stage":"legal_cast","card":def.name(),"actor":actor.index(),"action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_cast.is_none()
            && g.stack.iter().any(|e| {
                !e.is_ability && g.object(e.object_id).is_some_and(|o| o.name == def.name())
            })
        {
            let paid = before - g.player(actor).unwrap().mana_pool.total();
            dm.trace.push(json!({"stage":"cast_complete","card":def.name(),"paid":paid,"stack":format!("{:?}",g.stack)}));
            return Ok(paid);
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("cast announcement bound".into())
}
fn resolve_one(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    state.reset_for_new_priority_window(g);
    // The final pass invokes the production resolver, which retains the trigger
    // queue and therefore produces dedicated ETB events as well as zone changes.
    for _ in 0..g.players_in_game() {
        let progress = apply_priority_response_with_dm(
            g,
            q,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .map_err(|e| e.to_string())?;
        if let GameProgress::NeedsDecisionCtx(ctx) = &progress {
            if !matches!(
                ctx,
                ironsmith::decisions::context::DecisionContext::Priority(_)
            ) {
                return Err(format!(
                    "priority resolution requires unhandled decision:{ctx:?}"
                ));
            }
        }
    }
    Ok(())
}
fn drain(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    drain_pending_trigger_events(g, q);
    put_triggers_on_stack_with_dm(g, q, dm).map_err(|e| e.to_string())
}
fn resolve_all(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    for _ in 0..24 {
        drain(g, q, dm)?;
        if g.stack.is_empty() {
            return Ok(());
        }
        resolve_one(g, q, dm)?;
    }
    Err("bounded resolution did not empty stack".into())
}
fn announce(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    g.turn.priority_player = Some(PlayerId(0));
    eprintln!("AUDIT_STAGE activate {action:?}");
    let old = g.stack.len();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":"activation_action","action":format!("{action:?}")}));
    let mut p = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_activation.is_none() && g.stack.len() > old {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = p else {
            return Err(format!("activation stalled:{p:?}"));
        };
        p = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("activation bound".into())
}

fn mana(g: &mut GameState) {
    for p in [PlayerId(0), PlayerId(1)] {
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(symbol, 12);
        }
    }
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or_else(|| format!("{name} absent from battlefield"))
}

fn advance_turn(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    for e in ironsmith::turn::execute_draw_step_with(g, dm).unwrap() {
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t);
        }
    }
    resolve_all(g, q, dm)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(PlayerId(0));
    mana(g);
    Ok(())
}
fn announce_mana(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if let GameProgress::NeedsDecisionCtx(ctx) = progress {
            if matches!(
                ctx,
                ironsmith::decisions::context::DecisionContext::Priority(_)
            ) {
                return Ok(());
            }
            progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
                .map_err(|e| e.to_string())?;
        } else {
            return Ok(());
        }
    }
    Err("mana announcement bound".into())
}
fn run(def: &CardDefinition, opponent_turn: bool) -> Result<Value, String> {
    use ironsmith::static_abilities::StaticAbilityId as K;
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704855);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    mana(&mut g);
    let mut q = TriggerQueue::new();
    let mut dm = Choices {
        accept: false,
        target: None,
        trace: vec![],
    };
    let creature = CardDefinitionBuilder::new(CardId::new(), "Neutral sacrifice/pump witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 6))
        .build();
    let witness = g.create_object_from_definition(&creature, PlayerId(0), Zone::Battlefield);
    let artifact =
        CardDefinitionBuilder::new(CardId::new(), "Neutral untapped artifact cost witness")
            .card_types(vec![CardType::Artifact])
            .build();
    let artifacts = [
        g.create_object_from_definition(&artifact, PlayerId(0), Zone::Battlefield),
        g.create_object_from_definition(&artifact, PlayerId(0), Zone::Battlefield),
    ];
    let gy = CardDefinitionBuilder::new(CardId::new(), "Neutral graveyard Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![ironsmith::Subtype::Equipment])
        .build();
    let gy_id = g.create_object_from_definition(&gy, PlayerId(0), Zone::Graveyard);
    for player in [PlayerId(0), PlayerId(1)] {
        for _ in 0..6 {
            g.create_object_from_definition(
                &CardDefinitionBuilder::new(CardId::new(), "Neutral library card")
                    .card_types(vec![CardType::Artifact])
                    .build(),
                player,
                Zone::Library,
            );
        }
    }
    let cost = def.card.mana_cost.as_ref().unwrap().mana_value();
    let paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
    if paid != cost {
        return Err(format!("source paid{paid} expected{cost}"));
    }
    resolve_all(&mut g, &mut q, &mut dm)?;
    let source = find(&g, def.name())?;
    advance_turn(&mut g, &mut q, &mut dm)?;
    advance_turn(&mut g, &mut q, &mut dm)?;
    if opponent_turn {
        advance_turn(&mut g, &mut q, &mut dm)?;
    }
    if g.turn.active_player != PlayerId(if opponent_turn { 1 } else { 0 })
        || g.is_summoning_sick(source)
    {
        return Err("source/turn readiness invalid".into());
    }
    dm.target = Some(Target::Object(gy_id));
    g.turn.priority_player = Some(PlayerId(0));
    let actions:Vec<_>=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().filter(|a|matches!(a,LegalAction::ActivateAbility{source:s,..}|LegalAction::ActivateManaAbility{source:s,..}if *s==source)).collect();
    let action = actions.first().cloned();
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total() as i64;
    let mut error = None;
    let mut resolution_error = None;
    let mut evidence = json!({"opponent_turn":opponent_turn,"actual_turn":g.turn.turn_number,"active_player":g.turn.active_player.0,"source_cast_paid":paid,"actions":format!("{actions:?}"),"current_abilities":format!("{:?}",g.current_abilities(source)),"source_first_strike_before":g.object_has_static_ability_id(source,K::FirstStrike),"source_flying_before":g.object_has_static_ability_id(source,K::Flying),"source_sick":g.is_summoning_sick(source)});
    if let Some(a) = action.clone() {
        if let LegalAction::ActivateAbility { ability_index, .. }
        | LegalAction::ActivateManaAbility { ability_index, .. } = &a
        {
            evidence["selected_dispatcher_ability"] =
                json!(format!("{:?}", g.current_ability(source, *ability_index)));
        }
        let is_mana = matches!(a, LegalAction::ActivateManaAbility { .. });
        error = if is_mana {
            announce_mana(&mut g, &mut q, &mut dm, a).err()
        } else {
            announce(&mut g, &mut q, &mut dm, a).err()
        };
        if error.is_none() {
            resolution_error = resolve_all(&mut g, &mut q, &mut dm).err();
        }
    }
    let mut expected = json!({"activation_offered":true,"announcement_error":null,"resolution_error":null,"remaining_stack":0,"mana_delta":0,"source_tapped":false,"source_power":0,"bob_life":20,"first_strike":!opponent_turn&&matches!(def.name(),"Ahn-Crop Invader"|"Bearer of Glory"|"Shao Jun"|"Skilled Battlecarver"),"flying":!opponent_turn&&matches!(def.name(),"Cid, Freeflier Pilot"|"Freya Crescent"|"Shao Jun")});
    // Source power is compared against independent printed scalar values below.
    let (power, delta, tapped) = match def.name() {
        "Ahn-Crop Invader" => (4, -1, false),
        "Bearer of Glory" => (3, -5, false),
        "Cid, Freeflier Pilot" => (2, -2, true),
        "Freya Crescent" => (1, 1, true),
        "Shao Jun" => (3, 0, false),
        "Skilled Battlecarver" => (3, -2, false),
        _ => unreachable!(),
    };
    expected["source_power"] = json!(power);
    expected["mana_delta"] = json!(delta);
    expected["source_tapped"] = json!(tapped);
    let mut actual = json!({"activation_offered":action.is_some(),"announcement_error":error,"resolution_error":resolution_error,"remaining_stack":g.stack.len(),"mana_delta":g.player(PlayerId(0)).unwrap().mana_pool.total() as i64-before,"source_tapped":g.is_tapped(source),"source_power":g.calculated_power(source),"bob_life":g.player(PlayerId(1)).unwrap().life,"first_strike":g.object_has_static_ability_id(source,K::FirstStrike),"flying":g.object_has_static_ability_id(source,K::Flying)});
    match def.name() {
        "Ahn-Crop Invader" => {
            expected["sacrificed_witness"] = json!(true);
            actual["sacrificed_witness"] = json!(
                g.player(PlayerId(0))
                    .unwrap()
                    .graveyard
                    .iter()
                    .any(|id| g.object(*id).is_some_and(|o| o.name == creature.name()))
            );
        }
        "Bearer of Glory" => {
            expected["witness_power"] = json!(3);
            expected["witness_toughness"] = json!(7);
            expected["source_toughness"] = json!(2);
            actual["witness_power"] = json!(g.calculated_power(witness));
            actual["witness_toughness"] = json!(g.calculated_toughness(witness));
            actual["source_toughness"] = json!(g.calculated_toughness(source));
        }
        "Cid, Freeflier Pilot" => {
            expected["returned_equipment"] = json!(true);
            actual["returned_equipment"] = json!(
                g.player(PlayerId(0))
                    .unwrap()
                    .hand
                    .iter()
                    .any(|id| g.object(*id).is_some_and(|o| o.name == gy.name()))
            );
        }
        "Shao Jun" => {
            expected["bob_life"] = json!(19);
            expected["tapped_artifacts"] = json!(2);
            actual["tapped_artifacts"] =
                json!(artifacts.iter().filter(|id| g.is_tapped(**id)).count());
        }
        _ => {}
    }
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped turn-conditioned ability index report"]
fn report_turn_grant_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_ability_index_turn_grant_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Ahn-Crop Invader",
        "Bearer of Glory",
        "Cid, Freeflier Pilot",
        "Freya Crescent",
        "Shao Jun",
        "Skilled Battlecarver",
    ];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(name),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name, def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_STATIC_CARD").is_ok_and(|n| n != name) {
            continue;
        }
        for mode in 0..2 {
            if std::env::var("AUDIT_STATIC_MODE").is_ok_and(|n| n != mode.to_string()) {
                continue;
            }
            eprintln!("AUDIT_CASE {name} {mode}");
            let (status, out) = match run(&defs[name], mode == 1) {
                Ok(out) => {
                    let status = if out["actual"]["announcement_error"].is_string() {
                        "action_or_choice_failed"
                    } else if out["actual"]["resolution_error"].is_string() {
                        "resolution_failed"
                    } else if out["expected"] == out["actual"] {
                        "expected_result_observed"
                    } else {
                        "semantic_mismatch"
                    };
                    (status, out)
                }
                Err(e) => (
                    "fixture_or_execution_error",
                    json!({"expected":null,"actual":{"error":e}}),
                ),
            };
            rows.push(json!({"card":name,"scenario":{"opponent_turn":mode==1},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Paid canonical source, actual turn/untap transitions, correct resource-ready advertised activation during own/opponent first main. Exact costs, conditional keywords and named ability output checked. Freya mana amount/tap checked; spending restriction is outside scope."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    let out = std::env::var("AUDIT_STATIC_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            root.join("reports/runtime-audit/ability-index-turn-grant-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
