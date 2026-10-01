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
    for e in ironsmith::turn::execute_draw_step_with(g, dm) {
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
fn play_land(g:&mut GameState,def:&CardDefinition,actor:PlayerId,q:&mut TriggerQueue,dm:&mut Choices)->Result<ObjectId,String>{
 if g.turn.active_player!=actor{return Err("land producer wrong turn".into());}let id=g.create_object_from_definition(def,actor,Zone::Hand);g.turn.priority_player=Some(actor);let action=compute_legal_actions(g,actor).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::PlayLand{land_id}if *land_id==id)).ok_or("land producer unavailable")?;let mut state=PriorityLoopState::new(g.players_in_game());apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(action),dm).map_err(|e|e.to_string())?;resolve_all(g,q,dm)?;g.battlefield.iter().copied().find(|id|g.object(*id).is_some_and(|o|o.name==def.name())&&g.current_controller(*id)==Some(actor)).ok_or("land producer absent".into())
}
fn attack_probe(g:&mut GameState,q:&mut TriggerQueue,_dm:&mut Choices,source:ObjectId,preferred:PlayerId)->Result<Value,String>{
 use ironsmith::turn_runner::{TurnAction,TurnRunner,TurnState};
 use ironsmith::decisions::context::DecisionContext;
 use ironsmith::combat_state::AttackTarget;
 let mut runner=TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);let mut offered=vec![];let mut offered_context=None;let mut submitted=false;
 for _ in 0..80 {
  let action=match runner.advance(g,q){Ok(x)=>x,Err(e)=>return Ok(json!({"offered_defenders":offered,"submitted":submitted,"declaration_error":e.to_string(),"attacking":g.combat.as_ref().is_some_and(|c|ironsmith::combat_state::is_attacking(c,source)),"target":g.combat.as_ref().and_then(|c|ironsmith::combat_state::get_attack_target(c,source)).map(|t|format!("{t:?}")),"attacker_context":offered_context}))};
  match action {
   TurnAction::Continue=>{},
   TurnAction::RunPriority=>{if matches!(runner.state(),TurnState::DeclareAttackersPriority){return Ok(json!({"offered_defenders":offered,"submitted":submitted,"declaration_error":null,"attacking":g.combat.as_ref().is_some_and(|c|ironsmith::combat_state::is_attacking(c,source)),"target":g.combat.as_ref().and_then(|c|ironsmith::combat_state::get_attack_target(c,source)).map(|t|format!("{t:?}")),"attacker_context":offered_context}));}ironsmith::game_loop::run_priority_loop_with(g,q,&mut ironsmith::decision::AutoPassDecisionMaker).map_err(|e|e.to_string())?;runner.priority_done();},
   TurnAction::Decision(DecisionContext::Attackers(c))=>{offered_context=Some(format!("{c:?}"));offered=c.attacker_options.iter().find(|o|o.creature==source).map(|o|o.valid_targets.iter().filter_map(|t|match t{AttackTarget::Player(p)=>Some(p.index()),_=>None}).collect::<Vec<_>>()).unwrap_or_default();offered.sort();submitted=offered.contains(&preferred.index());runner.respond_attackers(if submitted{vec![ironsmith::decision::AttackerDeclaration{creature:source,target:AttackTarget::Player(preferred)}]}else{vec![]});},
   other=>return Err(format!("unexpected combat stage {other:?}")),
  }
 }
 Err("actual combat runner bound".into())
}
fn run(def:&CardDefinition,defs:&std::collections::HashMap<&str,CardDefinition>,mode:usize)->Result<Value,String>{
 let count=if mode>=2{3}else{2};let mut g=GameState::new((0..count).map(|n|format!("Player{n}")).collect(),20);g.set_random_seed(71757432704855);g.turn.turn_number=3;g.turn.active_player=PlayerId(0);g.turn.priority_player=Some(PlayerId(0));g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;mana(&mut g);
 let filler=CardDefinitionBuilder::new(CardId::new(),"Neutral library artifact").card_types(vec![CardType::Artifact]).build();for p in (0..count).map(|n|PlayerId(n as u8)){for _ in 0..12{g.create_object_from_definition(&filler,p,Zone::Library);}}
 let mut q=TriggerQueue::new();let mut dm=Choices{accept:true,target:None,trace:vec![],resources:vec![],x:0,activation_hint:None};
 // Own Island is present in every case so the separate no-Islands sacrifice
 // rule cannot remove a source and masquerade as its attack restriction.
 let own_island=play_land(&mut g,&defs["Island"],PlayerId(0),&mut q,&mut dm)?;
 let paid=cast(&mut g,def,PlayerId(0),&mut q,&mut dm)?;if paid!=def.card.mana_cost.as_ref().unwrap().mana_value(){return Err("source cost mismatch".into());}resolve_all(&mut g,&mut q,&mut dm)?;let source=find(&g,def.name())?;
 let mut lands=vec![json!({"player":0,"name":"Island","id":own_island.0})];
 for n in 1..count {advance_turn(&mut g,&mut q,&mut dm)?;let land_name=if (n==1&&(mode==1||mode==2))||(n==2&&mode==3){"Island"}else{"Forest"};let id=play_land(&mut g,&defs[land_name],PlayerId(n as u8),&mut q,&mut dm)?;lands.push(json!({"player":n,"name":land_name,"id":id.0}));}
 advance_turn(&mut g,&mut q,&mut dm)?;if g.turn.active_player!=PlayerId(0)||g.is_tapped(source)||g.is_summoning_sick(source){return Err("source not ready on actual next own turn".into());}
 let preferred=PlayerId(if mode==3{2}else{1});let before=json!({"source_paid":paid,"land_producers":lands,"source_ready":true,"actual_plus_one_counters":g.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),"source_current_abilities":format!("{:?}",g.current_abilities(source))});
 let probe=attack_probe(&mut g,&mut q,&mut dm,source,preferred)?;
 let expected=json!({"offered_defenders":if mode==0{vec![]}else{vec![preferred.index()]},"submitted":mode!=0,"declaration_error":null,"attacking":mode!=0,"target":if mode==0{None}else{Some(format!("{:?}",ironsmith::combat_state::AttackTarget::Player(preferred)))}});
 let mut actual=probe.clone();actual.as_object_mut().unwrap().remove("attacker_context");
 Ok(json!({"expected":expected,"actual":actual,"state_evidence":before,"combat_context":probe["attacker_context"],"execution_trace":dm.trace}))
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_island_attack_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root.join(
        "crates/ironsmith-tools/tests/runtime_ability_index_island_attack_reproductions.rs",
    );
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = ["Dreamwinder", "Floodchaser", "Kukemssa Serpent", "Marjhan", "Pirate Ship", "Sealock Monster", "Vodalian Knights"];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.into_iter().chain(["Island", "Forest"]) {
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
        for mode in 0..4 {
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
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"combat_context":out["combat_context"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Paid source, actual own and opposing land plays, real next-own-turn readiness, production TurnRunner attacker menu and actual declaration. An offered illegal attacker is submitted only through that real menu; validation rejection remains a separate interface observation, not a confirmed illegal declaration. Activated abilities and damage steps are outside this scope."}));
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
            root.join("reports/runtime-audit/ability-index-island_attack-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
