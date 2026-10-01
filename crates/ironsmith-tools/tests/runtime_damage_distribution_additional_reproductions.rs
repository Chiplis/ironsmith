//! Additional fixed-total creature-target distribution probes.
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


struct Choices{targets:Vec<Target>,amounts:Vec<u32>,trace:Vec<Value>}
impl DecisionMaker for Choices{
    fn answers_player_choices(&self)->bool{true}
    fn decide_targets(&mut self,_:&GameState,c:&TargetsContext)->Vec<Target>{
        assert_eq!(c.requirements.len(),1,"one printed divided damage target group");
        let req=&c.requirements[0];
        assert!(self.targets.len()>=req.min_targets&&req.max_targets.is_none_or(|m|self.targets.len()<=m),"requested target count must be offered");
        assert!(self.targets.iter().all(|t|req.legal_targets.contains(t)),"every requested target is legal");
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));self.targets.clone()
    }
    fn decide_distribute(&mut self,_:&GameState,c:&ironsmith::decisions::context::DistributeContext)->Vec<(Target,u32)>{
        let chosen:Vec<_>=self.targets.iter().copied().zip(self.amounts.iter().copied()).collect();
        self.trace.push(json!({"choice":"distribution","context":format!("{c:?}"),"selected":format!("{chosen:?}"),"expected_total":self.amounts.iter().sum::<u32>()}));chosen
    }
    fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{
        let selected=ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,c);self.trace.push(json!({"choice":"options","context":format!("{c:?}"),"selected":selected}));selected
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
fn find(g: &GameState, name: &str) -> Result<ObjectId,String> {
    g.battlefield.iter().copied().find(|id|g.object(*id).is_some_and(|o|o.name==name)).ok_or(format!("{name} absent"))
}
fn lands(g: &GameState) -> usize {
    g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.name=="Trigger sacrifice land")).count()
}

fn run(defs:&HashMap<String,CardDefinition>,name:&str,amounts:&[u32],mode:&str)->Result<(Value,Value),String>{
    let mut g=setup(3,0);
    for p in [PlayerId(0),PlayerId(1),PlayerId(2)]{for _ in 0..8{let d=CardDefinitionBuilder::new(CardId::new(),"Distribution library witness").card_types(vec![CardType::Instant]).build();g.create_object_from_definition(&d,p,Zone::Library);}}
    let witness=CardDefinitionBuilder::new(CardId::new(),"Distribution creature witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,10)).build();
    let creature_a=g.create_object_from_definition(&witness,PlayerId(0),Zone::Battlefield);let creature_b=g.create_object_from_definition(&witness,PlayerId(1),Zone::Battlefield);let creature_c=g.create_object_from_definition(&witness,PlayerId(2),Zone::Battlefield);let creature_d=g.create_object_from_definition(&witness,PlayerId(0),Zone::Battlefield);
    let targets=if mode=="stable"{[Target::Object(creature_a),Target::Object(creature_b),Target::Object(creature_c),Target::Object(creature_d)][..amounts.len()].to_vec()}else{[Target::Object(creature_a),Target::Object(creature_b)][..amounts.len()].to_vec()};
    let mut dm=Choices{targets,amounts:amounts.to_vec(),trace:vec![]};
    let requested_targets=dm.targets.clone();
    let mut response=Value::Null;
    let result=if mode=="stable"{cast(&mut g,&defs[name],0,&mut dm)}else{
        let(_,mut evidence)=announce(&mut g,&defs[name],0,&mut dm)?;
        dm.targets=vec![Target::Object(creature_a)];dm.amounts=vec![];
        response=cast(&mut g,&defs["Unsummon"],0,&mut dm)?;
        if response["mana_paid"]!=1{return Err(format!("response payment wrong:{response}"));}
        evidence["resolution_error"]=response["resolution_error"].clone();Ok(evidence)
    };
    let (error,cast)=match result{Ok(e)=>(e["resolution_error"].as_str().map(str::to_string),e),Err(e)=>(Some(e),Value::Null)};
    let actual=json!({"error":error,"first_creature_on_battlefield":g.object(creature_a).is_some_and(|o|o.zone==Zone::Battlefield),"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"creature_damage":[g.damage_on(creature_a),g.damage_on(creature_b),g.damage_on(creature_c),g.damage_on(creature_d)],"hand":g.player(PlayerId(0)).unwrap().hand.len(),"library":g.player(PlayerId(0)).unwrap().library.len()});
    Ok((actual,json!({"cast":cast,"response":response,"choices":dm.trace,"requested_targets":format!("{requested_targets:?}"),"requested_distribution":amounts})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="distribution audit reporter; inspect expected and actual states"]
fn report_distribution_additional(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];let mut compile_rows=vec![];
    let names=["Boulderfall","Flameshot","Forked Lightning","Pyrokinesis","Spreading Flames","Violent Eruption","Volley of Boulders","Avacyn's Judgment","Mythos of Vadrok"];
    for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !names.contains(&n)&&n!="Unsummon"{continue;}match ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false){Ok((a,d))=>{artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);},Err(e)=>compile_rows.push(json!({"card":n,"status":"compile_unavailable","error":format!("{e:?}"),"full_input":p["parse_input"]}))}}
    let mut rows=vec![];
    for name in names{if !defs.contains_key(name){continue;}
        let(total,cost): (u32,u32)=match name{"Boulderfall"=>(5,8),"Flameshot"=>(3,4),"Forked Lightning"=>(4,4),"Pyrokinesis"=>(4,6),"Spreading Flames"=>(6,7),"Violent Eruption"=>(4,4),"Volley of Boulders"=>(6,9),"Avacyn's Judgment"=>(2,2),"Mythos of Vadrok"=>(5,4),_=>unreachable!()};
        let mut splits=vec![vec![total],vec![total-1,1]];
        if total>2{splits.push(vec![1,total-1]);splits.push(vec![1,1,total-2]);}
        if total>=4&&name!="Forked Lightning"{splits.push(vec![1,1,1,total-3]);}
        for (mode,amounts) in splits.into_iter().map(|v|("stable",v)).chain([("remove_one",vec![1,total-1]),("remove_all",vec![total])]){
            let damage:Vec<_>=(0..4).map(|i|if mode!="stable"&&i==0{0}else{amounts.get(i).copied().unwrap_or(0)}).collect();
            let expected=json!({"error":null,"first_creature_on_battlefield":mode=="stable","life":[20,20,20],"creature_damage":damage,"hand":usize::from(mode!="stable"),"library":8});
            let(status,actual,evidence)=match run(&defs,name,&amounts,mode){Ok((a,e))=>(if !e["cast"].is_null()&&e["cast"]["mana_paid"]!=cost{"fixture_payment_error"}else if a==expected{"expected_result_observed"}else if !a["error"].is_null(){"execution_failed"}else{"semantic_mismatch"},a,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
            rows.push(json!({"card":name,"scenario":{"distribution":amounts,"targets":mode},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Strict canonical paid fixed-total divided damage spells; explicit legal targets and exact positive announced distributions; target-by-target marked-damage and life assertions, three players. Normal casting costs only; alternative/madness/flashback and Mythos additional restriction clauses are not certified. Actual paid Unsummon responses remove one or all targets; remaining damage keeps the announced split. Does not use SelectFirst's empty distribution response.","rows":rows,"compile_rows":compile_rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_damage_distribution_additional_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/damage-distribution-additional-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());
}
