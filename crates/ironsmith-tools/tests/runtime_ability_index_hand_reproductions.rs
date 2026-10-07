//! Scoped paid action probes for conditional Anthem ability ordering.
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
fn basic(name: &str, kind: CardType) -> CardDefinition {
    let b = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![kind]);
    if kind == CardType::Creature {
        b.power_toughness(PowerToughness::fixed(2, 6)).build()
    } else {
        b.build()
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
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
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
fn start_next_turn(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    let events = ironsmith::turn::execute_draw_step_with(g, dm).unwrap();
    for event in events {
        for t in ironsmith::triggers::check_triggers(g, &event) {
            q.add(t);
        }
    }
    resolve_all(g, q, dm)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(g.turn.active_player);
    dm.trace.push(json!({"stage":"next_turn_main","active":g.turn.active_player.index(),"turn":g.turn.turn_number,"phase":format!("{:?}",g.turn.phase)}));
    Ok(())
}
fn next_own_turn(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    for _ in 0..g.players_in_game() {
        start_next_turn(g, q, dm)?;
    }
    Ok(())
}

use ironsmith::Supertype;

use ironsmith::Subtype;
fn mana(g: &mut GameState) {
    for sym in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(sym, 12);
    }
}
fn run(def: &CardDefinition, opponent_turn: bool, _: usize) -> Result<Value,String> {
    let mut g=GameState::new(vec!["Alice".into(),"Bob".into()],20);g.set_random_seed(71757432704855);
    g.turn.turn_number=3;g.turn.active_player=PlayerId(if opponent_turn {1}else{0});g.turn.priority_player=Some(PlayerId(0));g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;mana(&mut g);
    let witness=g.create_object_from_definition(&basic("Established recipient",CardType::Creature),PlayerId(0),Zone::Battlefield);
    for _ in 0..3{g.create_object_from_definition(&basic("Neutral library card",CardType::Artifact),PlayerId(0),Zone::Library);}
    let source=g.create_object_from_definition(def,PlayerId(0),Zone::Hand);
    let mut q=TriggerQueue::new();let mut dm=Choices{accept:true,target:Some(Target::Object(witness)),trace:vec![]};
    let actions=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state");
    let action=actions.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source)).cloned();
    let offered=action.is_some();let mut announcement_error=None;let mut resolution_error=None;
    let before=g.player(PlayerId(0)).unwrap().mana_pool.total();
    if let Some(action)=action {announcement_error=announce(&mut g,&mut q,&mut dm,action).err();if announcement_error.is_none(){resolution_error=resolve_all(&mut g,&mut q,&mut dm).err();}}
    let paid=before-g.player(PlayerId(0)).unwrap().mana_pool.total();
    let channel=matches!(def.name(),"Born to Drive"|"Careful Cultivation"|"Favor of Jukai");
    let want_paid=match def.name(){"Born to Drive"=>3,"River Serpent"=>1,_=>2};
    let graveyard=g.player(PlayerId(0)).unwrap().graveyard.iter().filter(|id|g.object(**id).is_some_and(|o|o.name==def.name())).count();
    let token_count=g.battlefield.len()-1;
    let reach=g.current_has_static_ability_id(witness,ironsmith::static_abilities::StaticAbilityId::Reach);
    let expected=json!({"activation_offered":true,"announcement_error":null,"resolution_error":null,"remaining_stack":0,"paid":want_paid,"source_graveyard_count":1,"hand_count":if channel{0}else{1},"new_permanents":match def.name(){"Born to Drive"=>2,"Careful Cultivation"=>1,_=>0},"recipient_power":if def.name()=="Favor of Jukai"{5}else{2},"recipient_toughness":if def.name()=="Favor of Jukai"{9}else{6},"recipient_reach":def.name()=="Favor of Jukai"});
    let actual=json!({"activation_offered":offered,"announcement_error":announcement_error,"resolution_error":resolution_error,"remaining_stack":g.stack.len(),"paid":paid,"source_graveyard_count":graveyard,"hand_count":g.player(PlayerId(0)).unwrap().hand.len(),"new_permanents":token_count,"recipient_power":g.calculated_power(witness),"recipient_toughness":g.calculated_toughness(witness),"recipient_reach":reach});
    Ok(json!({"expected":expected,"actual":actual,"state_evidence":{"source_zone":"Hand","active_player":g.turn.active_player.0,"source":source.0,"legal_actions":format!("{actions:?}")},"execution_trace":dm.trace}))
}
#[test]
#[ignore = "manual scoped activation-index report"]
fn report_ability_index_hand() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let candidates = root.join("reports/runtime-audit/ability-index-structural-candidates.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_ability_index_hand_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &candidates, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names: Vec<String> = ["Born to Drive", "Careful Cultivation", "Favor of Jukai", "River Serpent", "Slipstream Eel", "Chitin Gravestalker", "Hollow One"]
    .iter()
    .map(|n| n.to_string())
    .collect();
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.iter().map(String::as_str) {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name.to_string(), def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_INDEX_CARD").is_ok_and(|selected|selected != name) {continue;}
        let def = &defs[&name];
        let activations = 1;
        for condition_on in [false, true] {
            if std::env::var("AUDIT_INDEX_CONDITION").is_ok_and(|selected|selected != condition_on.to_string()) {continue;}
            eprintln!("AUDIT_CASE {name} {condition_on}");
            for ordinal in 0..activations {
                let (status, out) = match run(def, condition_on, ordinal)
                {
                    Ok(v) => {
                        let status = if v["actual"]["announcement_error"].is_string() {
                            "action_or_choice_failed"
                        } else if v["actual"]["resolution_error"].is_string() {
                            "resolution_failed"
                        } else if v["actual"] == v["expected"] {
                            "expected_result_observed"
                        } else {
                            "semantic_mismatch"
                        };
                        (status, v)
                    }
                    Err(e) => (
                        "fixture_or_execution_error",
                        json!({"expected":null,"actual":{"error":e}}),
                    ),
                };
                rows.push(json!({"card":name,"scenario":{"condition_on":condition_on,"source_activation_ordinal":ordinal},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Strict canonical source in hand, ordinary printed Channel/Cycling activation in own or opponent main phase, explicit legal target/resources and actual mana/discard costs. Exact draw/token/pump outcomes and final source graveyard location asserted."}));
            }
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    std::fs::write(
        std::env::var("AUDIT_INDEX_OUTPUT").map(std::path::PathBuf::from).unwrap_or_else(|_|root.join("reports/runtime-audit/ability-index-hand-reproductions.json")),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
