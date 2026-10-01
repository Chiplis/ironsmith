//! Cultural Exchange exact distinct target player and creature selection probes.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;


struct Choices{players:Vec<PlayerId>,count:usize,trace:Vec<Value>}
impl DecisionMaker for Choices{
    fn answers_player_choices(&self)->bool{true}
    fn decide_targets(&mut self,_:&GameState,c:&TargetsContext)->Vec<Target>{
        let mut chosen=vec![];
        for req in &c.requirements{let intended=if c.requirements.len()==1{self.players.len()}else{1};for p in &self.players{let t=Target::Player(*p);if !chosen.contains(&t)&&req.legal_targets.contains(&t){chosen.push(t);if chosen.len()>=if c.requirements.len()==1{intended}else{c.requirements.iter().take_while(|r|!std::ptr::eq(*r,req)).count()+1}{break;}}}}
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{chosen:?}")}));chosen
    }
    fn decide_objects(&mut self,_:&GameState,c:&SelectObjectsContext)->Vec<ObjectId>{
        let mut ids:Vec<_>=c.candidates.iter().filter(|o|o.legal).map(|o|o.id).collect();ids.sort_by_key(|id|id.0);
        let selected:Vec<_>=ids.into_iter().take(self.count.min(c.max.unwrap_or(self.count))).collect();
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{selected:?}")}));selected
    }
    fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{
        let first=["Alice","Bob","Cara"][self.players[0].0 as usize];let selected=c.options.iter().find(|o|o.legal&&o.description==first).map(|o|vec![o.index]).unwrap_or_else(||ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,c));self.trace.push(json!({"choice":"options","context":format!("{c:?}"),"selected":selected}));selected
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
fn cast(g: &mut GameState, def: &CardDefinition, actor: u8, dm: &mut Choices) -> Result<Value,String> {
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

fn run(def:&CardDefinition,players:usize,a:u8,b:u8,count:usize)->Result<(Value,Value),String>{
    let mut g=setup(players,0);let mut ids=vec![];
    for p in 0..players{for i in 0..3{let d=CardDefinitionBuilder::new(CardId::new(),format!("Exchange witness {p}-{i}")).card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,6)).build();ids.push(g.create_object_from_definition(&d,PlayerId(p as u8),Zone::Battlefield));}}
    let mut dm=Choices{players:vec![PlayerId(a),PlayerId(b)],count,trace:vec![]};let result=cast(&mut g,def,0,&mut dm)?;
    if result["mana_paid"]!=6{return Err(format!("cast payment wrong:{result}"));}
    let actual=json!({"resolution_error":result["resolution_error"],"controllers":ids.iter().map(|id|g.controller_of_id(*id).map(|p|p.0)).collect::<Vec<_>>()});
    Ok((actual,json!({"source_cast":result,"choices":dm.trace})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="Cultural Exchange player-choice reporter"]
fn report_cultural(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let p=payloads["cards"].as_array().unwrap().iter().find(|p|p["name"]=="Cultural Exchange").unwrap();let(a,def)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),"Cultural Exchange"),p["parse_input"].as_str().unwrap(),false).unwrap();
    let mut rows=vec![];
    for (players,a,b) in [(2usize,0u8,1u8),(3,0,1),(3,1,2)]{for count in 0..=2{
        let controllers:Vec<_>=(0..players).flat_map(|p|(0..3).map(move|i|if i<count&&p==a as usize{b}else if i<count&&p==b as usize{a}else{p as u8})).collect();let expected=json!({"resolution_error":null,"controllers":controllers});
        let(status,actual,evidence)=match run(&def,players,a,b,count){Ok((v,e))=>(if v==expected{"expected_result_observed"}else if !v["resolution_error"].is_null(){"resolution_failed"}else{"semantic_mismatch"},v,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
        rows.push(json!({"card":"Cultural Exchange","scenario":{"players":players,"first_player":a,"second_player":b,"creatures_each":count},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
    }}
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Strict canonical paid Cultural Exchange; two/three-player distinct targets and zero/one/two chosen creatures each. Actual legal target and object decisions, exact controllers for all nine/six witnesses.","rows":rows,"artifacts":[{"card":"Cultural Exchange","checksum":a.payload_checksum,"definition":a.payload.definition}],"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_cultural_choice_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/cultural-choice-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());
}
