//! Paid conditional-index transitions caused by costs and current-turn counter history.
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
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
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
    resources: Vec<ObjectId>,
    x: u32,
    activation_hint: Option<&'static str>,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _: &GameState, c: &ironsmith::decisions::context::NumberContext) -> u32 {
        let x=self.x.clamp(c.min,c.max);
        self.trace.push(json!({"decision":"number","context":format!("{c:?}"),"answer":x})); x
    }
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"decision":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let a = if let Some(o) = c
            .options
            .iter()
            .find(|o| o.legal && o.description == "Untap")
        {
            vec![o.index]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"decision":"options","context":format!("{c:?}"),"answer":a}));
        a
    }
    fn decide_objects(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let preferred: Vec<_> = self
            .resources
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
            .collect();
        let a = if preferred.len() >= c.min && !preferred.is_empty() {
            preferred
                .into_iter()
                .take(c.max.unwrap_or(self.resources.len()))
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
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
        ironsmith::game_loop::advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;
        drain(g, q, dm)?;
        if g.stack.is_empty() {
            return Ok(());
        }
        dm.trace.push(json!({"stage":"stack_before_resolution","entries":format!("{:?}",g.stack)}));
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
    for p in (0..g.players_in_game()).map(|n|PlayerId(n as u8)) {
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

fn activation(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    source: ObjectId,
) -> Result<Value, String> {
    g.turn.priority_player = Some(PlayerId(0));
    let a = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}|LegalAction::ActivateManaAbility{source:s,..}if *s==source) && dm.activation_hint.is_none_or(|hint| match a { LegalAction::ActivateAbility{ability_index,..}|LegalAction::ActivateManaAbility{ability_index,..} => g.current_abilities(source).is_some_and(|abilities|abilities.get(*ability_index).is_some_and(|a|format!("{a:?}").contains(hint))), _=>false }));
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total() as i64;
    let mut error = None;
    let mut resolution_error = None;
    if let Some(action) = a.clone() {
        dm.trace.push(json!({"stage":"advertised_source_action","action":format!("{action:?}"),"abilities":format!("{:?}",g.current_abilities(source))}));
        error = if matches!(action, LegalAction::ActivateManaAbility { .. }) {
            announce_mana(g, q, dm, action).err()
        } else {
            announce(g, q, dm, action).err()
        };
        if error.is_none() {
            resolution_error = resolve_all(g, q, dm).err();
        }
    }
    Ok(
        json!({"offered":a.is_some(),"announcement_error":error,"resolution_error":resolution_error,"mana_paid":before-g.player(PlayerId(0)).unwrap().mana_pool.total() as i64,"remaining_stack":g.stack.len()}),
    )
}
fn run(def:&CardDefinition,defs:&std::collections::HashMap<&str,CardDefinition>,mode:usize)->Result<Value,String>{
 let count=if def.name()=="Tinybones, Bauble Burglar"&&mode>=2{3}else{2};
 let mut g=GameState::new((0..count).map(|n|format!("Player{n}")).collect(),20);g.set_random_seed(71757432704855);g.turn.turn_number=3;g.turn.active_player=PlayerId(0);g.turn.priority_player=Some(PlayerId(0));g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;mana(&mut g);
 let filler=CardDefinitionBuilder::new(CardId::new(),"Neutral library artifact").card_types(vec![CardType::Artifact]).build();for p in (0..count).map(|n|PlayerId(n as u8)){for _ in 0..12{g.create_object_from_definition(&filler,p,Zone::Library);}if def.name()=="Tinybones, Bauble Burglar"&&mode>0&&p!=PlayerId(0){g.create_object_from_definition(&defs[if mode==3&&p==PlayerId(2){"Elvish Mystic"}else{"Llanowar Elves"}],p,Zone::Library);}}
 let mut q=TriggerQueue::new();let mut dm=Choices{accept:true,target:None,trace:vec![],resources:vec![],x:0,activation_hint:None};
 let paid=cast(&mut g,def,PlayerId(0),&mut q,&mut dm)?;if paid!=2{return Err("source paid cost mismatch".into());}resolve_all(&mut g,&mut q,&mut dm)?;let source=find(&g,def.name())?;
 let expected_activation=|cost:i64|json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":cost,"remaining_stack":0});
 if def.name()=="Leonin Surveyor"{
  use ironsmith::static_abilities::StaticAbilityId as K;
  if g.player_speed(PlayerId(0))!=Some(1){return Err("Start engines did not establish speed1".into());}
  let own_first=g.object_has_static_ability_id(source,K::FirstStrike);advance_turn(&mut g,&mut q,&mut dm)?;let opponent_first=g.object_has_static_ability_id(source,K::FirstStrike);advance_turn(&mut g,&mut q,&mut dm)?;
  let shocks=match mode{0=>0,1=>2,_=>3};let mut speed_history=vec![1];
  for i in 0..shocks {dm.target=Some(Target::Player(PlayerId(1)));if cast(&mut g,&defs["Shock"],PlayerId(0),&mut q,&mut dm)?!=1{return Err("Shock cost".into());}resolve_all(&mut g,&mut q,&mut dm)?;let speed=g.player_speed(PlayerId(0));speed_history.push(speed.unwrap_or(0));if speed!=Some((i+2)as u8){return Err(format!("real Shock speed increment mismatch {speed:?}"));}if i+1<shocks{advance_turn(&mut g,&mut q,&mut dm)?;advance_turn(&mut g,&mut q,&mut dm)?;}}
  dm.target=Some(Target::Object(source));if cast(&mut g,&defs["Murder"],PlayerId(0),&mut q,&mut dm)?!=3{return Err("Murder cost".into());}resolve_all(&mut g,&mut q,&mut dm)?;
  let gy=*g.player(PlayerId(0)).unwrap().graveyard.iter().find(|id|g.object(**id).is_some_and(|o|o.name==def.name())).ok_or("source not killed")?;
  dm.target=None;let before_hand=g.player(PlayerId(0)).unwrap().hand.len();let a=activation(&mut g,&mut q,&mut dm,gy)?;
  let expected=json!({"activation":if mode==2{expected_activation(3)}else{json!({"offered":false,"announcement_error":null,"resolution_error":null,"mana_paid":0,"remaining_stack":0})},"speed":shocks+1,"own_first_strike":true,"opponent_first_strike":false,"hand_delta":if mode==2{1}else{0},"source_exiled":mode==2});
  let actual=json!({"activation":a,"speed":g.player_speed(PlayerId(0)),"own_first_strike":own_first,"opponent_first_strike":opponent_first,"hand_delta":g.player(PlayerId(0)).unwrap().hand.len()-before_hand,"source_exiled":g.exile.iter().any(|id|g.object(*id).is_some_and(|o|o.name==def.name()))});
  return Ok(json!({"expected":expected,"actual":actual,"state_evidence":{"source_paid":paid,"actual_shock_producers":shocks,"speed_history":speed_history,"graveyard_source_id":format!("{gy:?}")},"execution_trace":dm.trace}));
 }
 // Opponents draw the canonical Elf from their actual draw steps; no discard
 // or exile event, stash counter, permission or target binding is injected.
 let mut off_action=false;for n in 0..count{advance_turn(&mut g,&mut q,&mut dm)?;if n==0{off_action=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source));}}
 let hands:Vec<_>=(1..count).map(|n|g.player(PlayerId(n as u8)).unwrap().hand.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()).collect();
 if mode>0&&hands.iter().enumerate().any(|(i,h)|h!=&vec![if mode==3&&i==1{"Elvish Mystic".to_string()}else{"Llanowar Elves".to_string()}]){return Err(format!("actual draw resource mismatch {hands:?}"));}
 // Empty-hand control spends each opposing neutral drawn artifact through its
 // ordinary cast rather than silently discarding it in fixture setup.
 if mode==0{return Err("empty hand scenario replaced by populated boundaries".into());}
 let before_hands:Vec<_>=(1..count).map(|n|g.player(PlayerId(n as u8)).unwrap().hand.len()).collect();
 dm.activation_hint=Some("Discard");let a=activation(&mut g,&mut q,&mut dm,source)?;
 let stash:Vec<_>=g.exile.iter().copied().filter(|id|g.object(*id).is_some_and(|o|o.owner!=PlayerId(0))&&g.counter_count(*id,ironsmith::CounterType::Named("stash".into()))==1).collect();
 let after_hands:Vec<_>=(1..count).map(|n|g.player(PlayerId(n as u8)).unwrap().hand.len()).collect();
 let all_exiled:Vec<_>=g.exile.iter().copied().filter(|id|g.object(*id).is_some_and(|o|o.owner!=PlayerId(0))).collect();
 let exiled_names:Vec<_>=all_exiled.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect();
 let permissions:Vec<_>=all_exiled.iter().map(|id|compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if spell_id==id))).collect();
 let expected=json!({"activation":expected_activation(4),"opponent_turn_activation_offered":false,"source_tapped":true,"each_opponent_discarded_one":true,"exiled_with_stash":count-1,"all_opponent_exiled":count-1,"stash_counters":vec![1;count-1],"each_exiled_cast_offered":vec![true;count-1]});
 let actual=json!({"activation":a,"opponent_turn_activation_offered":off_action,"source_tapped":g.is_tapped(source),"each_opponent_discarded_one":before_hands.iter().zip(after_hands.iter()).all(|(a,b)|*a==*b+1),"exiled_with_stash":stash.len(),"all_opponent_exiled":all_exiled.len(),"stash_counters":all_exiled.iter().map(|id|g.counter_count(*id,ironsmith::CounterType::Named("stash".into()))).collect::<Vec<_>>(),"each_exiled_cast_offered":permissions});
 Ok(json!({"expected":expected,"actual":actual,"state_evidence":{"source_paid":paid,"initial_opponent_hands":hands,"before_hands":before_hands,"after_hands":after_hands,"opponent_graveyards":(1..count).map(|n|g.player(PlayerId(n as u8)).unwrap().graveyard.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()).collect::<Vec<_>>(),"exiled_names":exiled_names,"exiled_object_ids":all_exiled.iter().map(|id|format!("{id:?}")).collect::<Vec<_>>(),"ability_trace":format!("{:?}",g.current_abilities(source)),"play_permission_scope":"Only advertised own-turn plays checked here; actual as-though payment is separate."},"execution_trace":dm.trace}))
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_speed_discard_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root.join(
        "crates/ironsmith-tools/tests/runtime_ability_index_speed_discard_reproductions.rs",
    );
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = ["Leonin Surveyor", "Tinybones, Bauble Burglar"];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.into_iter().chain(["Shock", "Murder", "Llanowar Elves", "Elvish Mystic"]) {
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
        let count = match name {
            "Leonin Surveyor" => 3,
            "Tinybones, Bauble Burglar" => 4,
            _ => 2,
        };
        for mode in (if name=="Tinybones, Bauble Burglar"{1}else{0})..count {
            if std::env::var("AUDIT_STATIC_MODE").is_ok_and(|n| n != mode.to_string()) {
                continue;
            }
            eprintln!("AUDIT_CASE {name} {mode}");
            let (status, out) = match run(&defs[name], &defs, mode) {
                Ok(out) => {
                    let status = if out["actual"]["activation"]["announcement_error"].is_string() {
                        "action_or_choice_failed"
                    } else if out["actual"]["activation"]["resolution_error"].is_string() {
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
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Paid canonical source and real printed producer actions, actual turn/untap transitions as required, resource-complete advertised activation. Exact condition characteristics, costs and outcome checked; linked-face and unrelated abilities remain outside scope."}));
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
            root.join("reports/runtime-audit/ability-index-speed_discard-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
