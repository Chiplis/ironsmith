//! Counted Object-target damage family probes with real payments and target choices.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;


struct Choices { targets: Vec<Target>, trace: Vec<Value> }
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets", "context":format!("{c:?}"), "selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected=ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,c);
        self.trace.push(json!({"choice":"options", "context":format!("{c:?}"), "selected":selected})); selected
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

const NAMES: [&str; 8] = ["Cast into the Fire","Dual Shot","Jagged Lightning","Sparkmage's Gambit","Swelter","Twinstrike","Volcanic Salvo","Wrap in Flames"];
fn selected_count(name:&str)->usize {if name=="Wrap in Flames"{3}else{2}}
fn cost(name:&str)->usize {match name {"Dual Shot"=>1,"Cast into the Fire"|"Sparkmage's Gambit"=>2,"Swelter"|"Wrap in Flames"=>4,"Jagged Lightning"|"Twinstrike"|"Volcanic Salvo"=>5,_=>unreachable!()}}
fn damage(name:&str)->usize {match name {"Jagged Lightning"=>3,"Swelter"|"Twinstrike"=>2,"Volcanic Salvo"=>6,_=>1}}
fn snapshot(g: &GameState, ids: &[ObjectId], error: Option<String>) -> Value {
    use ironsmith::object::CounterType;
    use ironsmith::static_abilities::StaticAbilityId;
    let witnesses:Vec<_>=ids.iter().map(|id| {
        let live=g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield);
        json!({"battlefield":live,
            "power":if live{g.calculated_power(*id)}else{None},"toughness":if live{g.calculated_toughness(*id)}else{None},
            "damage":g.damage_on(*id),"plus":g.counter_count(*id,CounterType::PlusOnePlusOne),"minus":g.counter_count(*id,CounterType::MinusOneMinusOne),
            "vigilance":live&&g.object_has_static_ability_id(*id,StaticAbilityId::Vigilance),
            "first_strike":live&&g.object_has_static_ability_id(*id,StaticAbilityId::FirstStrike),
            "lifelink":live&&g.object_has_static_ability_id(*id,StaticAbilityId::Lifelink)})
    }).collect();
    json!({"error":error,"witnesses":witnesses})
}
fn expected(name:&str, count:usize, removed:Option<usize>) -> Value {
    let mut ws=vec![];
    for i in 0..4 {
        if removed==Some(i) { ws.push(json!({"battlefield":false,"power":null,"toughness":null,"damage":0,"plus":0,"minus":0,"vigilance":false,"first_strike":false,"lifelink":false}));continue; }
        let (mut p,mut t)=(2+i as i32,10+i as i32);let(mut plus,mut minus,mut damage)=(0,0,0);let(mut vigilance,mut first_strike,mut lifelink)=(false,false,false);
        if i<count {damage=crate::damage(name);}
        ws.push(json!({"battlefield":true,"power":p,"toughness":t,"damage":damage,"plus":plus,"minus":minus,"vigilance":vigilance,"first_strike":first_strike,"lifelink":lifelink}));
    }
    json!({"error":null,"witnesses":ws})
}
fn run(defs:&HashMap<String,CardDefinition>,name:&str,count:usize,removed:Option<usize>)->Result<(Value,Value),String>{
    let mut g=setup(3,0);let mut ids=vec![];
    let hand_guard=CardDefinitionBuilder::new(CardId::new(),"Non-hellbent hand witness").card_types(vec![CardType::Artifact]).build();
    g.create_object_from_definition(&hand_guard,PlayerId(0),Zone::Hand);
    for i in 0..4 {
        let d=CardDefinitionBuilder::new(CardId::new(),format!("Target witness {i}")).card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2+i,10+i)).build();
        ids.push(g.create_object_from_definition(&d,PlayerId((i%3) as u8),Zone::Battlefield));
    }
    let mut dm=Choices{targets:ids[..count].iter().copied().map(Target::Object).collect(),trace:vec![]};
    let requested=dm.targets.clone();let mut response=Value::Null;
    let result=if let Some(i)=removed {
        let(_,mut e)=announce(&mut g,&defs[name],0,&mut dm)?;
        if e["mana_paid"]!=cost(name) {return Err(format!("source payment wrong:{e}"));}
        dm.targets=vec![Target::Object(ids[i])];response=cast(&mut g,&defs["Unsummon"],0,&mut dm)?;
        if response["mana_paid"]!=1 {return Err(format!("response payment wrong:{response}"));}
        e["resolution_error"]=response["resolution_error"].clone();e
    } else {cast(&mut g,&defs[name],0,&mut dm)?};
    if result["mana_paid"]!=cost(name) {return Err(format!("source payment wrong:{result}"));}
    let error=result["resolution_error"].as_str().map(str::to_string);
    Ok((snapshot(&g,&ids,error),json!({"source_cast":result,"response_cast":response,"choices":dm.trace,"requested_targets":format!("{requested:?}")})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="counted damage audit reporter"]
fn report_counted_damage(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];let mut compile_rows=vec![];
    for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !NAMES.contains(&n)&&n!="Unsummon"{continue;}
        match ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false) {
            Ok((a,d))=>{artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);},
            Err(e)=>compile_rows.push(json!({"card":n,"status":"compile_unavailable","error":format!("{e:?}"),"full_input":p["parse_input"]})),
        }
    }
    let mut rows=vec![];
    for name in NAMES {if !defs.contains_key(name){continue;}let full=selected_count(name);
        let counts:Vec<usize>=if ["Jagged Lightning","Swelter","Twinstrike"].contains(&name){vec![full]}else{(0..=full).collect()};
        for count in counts {for removed in std::iter::once(None).chain((0..count).map(Some)) {
            let expected=expected(name,count,removed);
            let(status,actual,evidence)=match run(&defs,name,count,removed){Ok((a,e))=>(if a==expected{"expected_result_observed"}else if !a["error"].is_null(){"execution_failed"}else{"semantic_mismatch"},a,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
            rows.push(json!({"card":name,"scenario":{"selected_targets":count,"removed_target_index":removed},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }}
    }
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();
    let r=json!({"scope":"Strict full canonical counted Object-target damage spells, normal costs paid, explicit targets; actual Unsummon removes one target. Exact per-witness damage with untargeted control. Twinstrike hand guard prevents hellbent. Sparkmage Gambit/Wrap block restrictions are outside this damage-only scope; Cast into the Fire uses damage mode.","rows":rows,"compile_unavailable":compile_rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_counted_damage_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/counted-damage-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());
}
