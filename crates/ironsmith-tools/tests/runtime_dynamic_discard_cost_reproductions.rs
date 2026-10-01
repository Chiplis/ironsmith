//! Counted Object-target damage family probes with real payments and target choices.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, SelectOptionsContext, TargetsContext, NumberContext, SelectObjectsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;


struct Choices { name:String, x:usize, targets:Vec<Target>, discards:Vec<ObjectId>, chosen_count:usize, trace:Vec<Value> }
impl DecisionMaker for Choices {
 fn answers_player_choices(&self)->bool{true}
 fn decide_targets(&mut self,_:&GameState,c:&TargetsContext)->Vec<Target>{
  let mut selected=self.targets.clone();
  if self.name=="Slight Malfunction" {
   let max=c.requirements.iter().map(|r|r.max_targets.unwrap_or(usize::MAX)).min().unwrap_or(0);
   selected.truncate(max.min(self.x));
  }
  self.chosen_count=selected.len();
  self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{selected:?}")}));selected
 }
 fn decide_number(&mut self,_:&GameState,c:&NumberContext)->u32{
  let chosen=if c.is_x_value{self.x as u32}else{c.min};self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"selected":chosen}));chosen
 }
 fn decide_objects(&mut self,g:&GameState,c:&SelectObjectsContext)->Vec<ObjectId>{
  let chosen=if c.description.to_lowercase().contains("discard"){self.discards.iter().copied().take(self.x).collect()}else{ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g,c)};
  self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{chosen:?}")}));chosen
 }
 fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{
  let chosen=if self.name=="Slight Malfunction"&&c.options.iter().any(|o|o.description.to_lowercase().contains("roll")) {c.options.iter().find(|o|o.legal&&o.description.to_lowercase().contains("roll")).map(|o|vec![o.index]).unwrap()}else{ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,c)};
  self.trace.push(json!({"choice":"options","context":format!("{c:?}"),"selected":chosen}));chosen
 }
}
fn setup(players: usize, lands: usize) -> GameState {
    let mut g=GameState::new(["Alice","Bob","Cara"][..players].iter().map(|s|s.to_string()).collect(),20);
    g.turn.turn_number=3;
    g.turn.active_player=PlayerId(0);
    g.turn.priority_player=Some(PlayerId(0));
    g.turn.phase=ironsmith::Phase::FirstMain;
    g.turn.step=None;
    for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color,12);
    }
    for _ in 0..lands {
        let def=CardDefinitionBuilder::new(CardId::new(),"Trigger sacrifice land").card_types(vec![CardType::Land]).build();
        g.create_object_from_definition(&def,PlayerId(0),Zone::Battlefield);
    }
    g
}
fn announce(g: &mut GameState, def: &CardDefinition, actor: u8, dm: &mut Choices) -> Result<(TriggerQueue,Value),String> {
    g.turn.priority_player=Some(PlayerId(actor));
    let source=g.create_object_from_definition(def,PlayerId(actor),Zone::Hand);
    let action=compute_legal_actions(g,PlayerId(actor)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).ok_or("intended cast unavailable")?;
    let mana=g.player(PlayerId(actor)).unwrap().mana_pool.total();
    let mut q=TriggerQueue::new();
    let mut state=PriorityLoopState::new(g.players_in_game());
    let mut progress=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),dm).map_err(|e|e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && !g.stack.is_empty() { break; }
        let GameProgress::NeedsDecisionCtx(ctx)=progress else { return Err(format!("announcement stopped:{progress:?}")); };
        if matches!(ctx,DecisionContext::Priority(_)) { return Err("announcement returned priority without spell".into()); }
        progress=apply_decision_context_with_dm(g,&mut q,&mut state,&ctx,dm).map_err(|e|e.to_string())?;
    }
    if g.stack.is_empty() { return Err("announcement budget".into()); }
    let mut evidence=json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",g.stack.last().unwrap().targets),"resolution_error":null});
    if !g.stack.last().is_some_and(|entry|g.object(entry.object_id).is_some_and(|o|o.name==def.name())){return Err("intended spell did not reach top of stack".into());}
    Ok((q,evidence))
}
fn cast(g: &mut GameState, def: &CardDefinition, actor: u8, dm: &mut Choices) -> Result<Value,String> {
    let(mut q,mut evidence)=announce(g,def,actor,dm)?;
    let mut state=PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        if let Err(error)=advance_priority_with_dm(g,&mut q,dm) {
            evidence["resolution_error"]=json!(error.to_string());return Ok(evidence);
        }
        if g.stack.is_empty() { return Ok(evidence); }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            if let Err(error)=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm) {
                evidence["resolution_error"]=json!(error.to_string());return Ok(evidence);
            }
        }
    }
    Err("resolution decision budget".into())
}



const NAMES:[&str;13]=["Abandon Hope","Aether Tide","Channeled Force","Devastating Dreams","Firestorm","Insidious Dreams","Nahiri's Wrath","Nostalgic Dreams","Restless Dreams","Scorched Earth","Sickening Dreams","Turbulent Dreams","Vengeful Dreams"];
fn run(defs:&HashMap<String,CardDefinition>,name:&str,resources:usize)->Result<(Value,Value,Value),String>{
 let mut g=setup(3,0);
 let witness=CardDefinitionBuilder::new(CardId::new(),"Cost control damage witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,10)).build();
 let guard=g.create_object_from_definition(&witness,PlayerId(1),Zone::Battlefield);
 for _ in 0..resources {
  for n in ["Plains","Grizzly Bears","Lightning Bolt"]{g.create_object_from_definition(&defs[n],PlayerId(0),Zone::Hand);}
  g.create_object_from_definition(&defs["Grizzly Bears"],PlayerId(0),Zone::Graveyard);
  g.create_object_from_definition(&defs["Grizzly Bears"],PlayerId(1),Zone::Battlefield);
  g.create_object_from_definition(&defs["Plains"],PlayerId(1),Zone::Battlefield);
 }
 let source=g.create_object_from_definition(&defs[name],PlayerId(0),Zone::Hand);
 let actions=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state");
 let offered=actions.iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source));
 let check=ironsmith::costs::CostCheckContext::new(source,PlayerId(0)).with_x(0).with_reason(ironsmith::costs::PaymentReason::CastSpell);
 let cost_checks:Vec<_>=defs[name].additional_cost.costs().iter().map(|c|format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*c.0,&g,&check))).collect();
 let state=json!({"phase":format!("{:?}",g.turn.phase),"priority_player":format!("{:?}",g.turn.priority_player),"stack_size":g.stack.len(),"mana_available":g.player(PlayerId(0)).unwrap().mana_pool.total(),"hand_names":g.player(PlayerId(0)).unwrap().hand.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>(),"legal_actions":format!("{actions:?}"),"additional_cost_checks_with_intended_x_zero":cost_checks});
 let mut control=Choices{name:"Lightning Bolt".into(),x:0,targets:vec![Target::Object(guard)],discards:vec![],chosen_count:0,trace:vec![]};
 let paid_control=cast(&mut g,&defs["Lightning Bolt"],0,&mut control)?;
 if paid_control["mana_paid"]!=1||g.damage_on(guard)!=3{return Err(format!("paid control failed:{paid_control}"));}
 Ok((json!({"cast_available":true}),json!({"cast_available":offered}),json!({"stage":"legal action discovery before announcement; X0 legal regardless of available discard resources; X is not announced or paid","state":state,"paid_control":paid_control,"control_choices":control.trace})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="dynamic discard additional-cost audit reporter"]
fn report_dynamic_discard_cost(){
 let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];let mut compile_rows=vec![];
 for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !NAMES.contains(&n)&&!["Lightning Bolt","Plains","Grizzly Bears"].contains(&n){continue;}
 match ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false){Ok((a,d))=>{artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);},Err(e)=>compile_rows.push(json!({"card":n,"status":"compile_unavailable","error":format!("{e:?}")})),}}
 let mut rows=vec![];
 for name in NAMES{if !defs.contains_key(name){continue;}for resources in [0,3]{let(status,expected,actual,evidence)=match run(&defs,name,resources){Ok((e,a,f))=>(if e==a{"expected_result_observed"}else{"semantic_mismatch"},e,a,f),Err(e)=>("fixture_or_producer_error",Value::Null,json!({"error":e}),Value::Null)};
 rows.push(json!({"card":name,"scenario":{"extra_resource_sets":resources,"intended_x":0},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));}}
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"All13 frozen typed non-Fixed DiscardEffect mandatory additional-cost spell candidates. Full canonical metadata, own main empty stack,72mana; intended X0 needs no discard resources or X targets. Zero/three sets of canonical hand, graveyard and battlefield resources. Actual paid Lightning Bolt control in each state. Tests discovery; no source spell announcement/payment/execution is claimed.","rows":rows,"compile_unavailable":compile_rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_dynamic_discard_cost_reproductions.rs")),"runtime_stack_bytes":67108864}});
 std::fs::write(root.join("reports/runtime-audit/dynamic-discard-cost-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());
}
