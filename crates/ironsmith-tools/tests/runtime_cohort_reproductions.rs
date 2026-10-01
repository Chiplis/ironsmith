//! Cohort activation cost resource and summoning-sickness probes.
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

struct Choices { object: Option<ObjectId>, player: PlayerId, chosen: PlayerId, trace: Vec<Value> }
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn decide_options(&mut self, g: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let name=["Alice","Bob","Cara"][self.chosen.0 as usize];
        let selected=ctx.options.iter().find(|o|o.legal&&o.description==name).map(|o|vec![o.index]).unwrap_or_else(||ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,ctx));
        self.trace.push(json!({"choice":"options","context":format!("{ctx:?}"),"selected":selected}));
        selected
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let preferred = [self.object.map(Target::Object), Some(Target::Player(self.player))];
        let mut chosen = Vec::new();
        for requirement in &ctx.requirements {
            let target = preferred.iter().flatten().find(|t|requirement.legal_targets.contains(t))
                .expect("fixture target must be explicitly legal");
            chosen.push(*target);
        }
        self.trace.push(json!({"choice":"targets","context":format!("{ctx:?}"),"selected":format!("{chosen:?}")}));
        chosen
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut candidates:Vec<_> = ctx.candidates.iter().filter(|o|o.legal).collect();
        candidates.sort_by_key(|c|!game.object(c.id).is_some_and(|o|o.name=="Cliffside Lookout"));
        let selected:Vec<_> = candidates.iter().take(ctx.min).map(|o|o.id).collect();
        self.trace.push(json!({"choice":"objects","context":format!("{ctx:?}"),"selected":format!("{selected:?}")}));
        selected
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

fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(),String> {
    let mut state=PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;
        if g.stack.is_empty() {return Ok(());}
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm).map_err(|e|e.to_string())?;
        }
    }
    Err("resolution window budget".into())
}
fn main_phase(g:&mut GameState,actor:u8){
    while g.turn.active_player!=PlayerId(actor){g.next_turn();ironsmith::turn::execute_untap_step(g);}
    g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;g.turn.priority_player=Some(PlayerId(actor));
    for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless]{g.player_mut(PlayerId(actor)).unwrap().mana_pool.add(color,12);}
}
fn establish(g:&mut GameState,defs:&HashMap<String,CardDefinition>,name:&str,actor:u8,dm:&mut Choices)->Result<(ObjectId,Value),String>{
    main_phase(g,actor);
    let e=cast(g,&defs[name],actor,dm)?;
    if !e["resolution_error"].is_null(){return Err(format!("source cast failed:{e}"));}
    let cost=match name{"Akoum Flameseeker"=>3,"Drana's Chosen"=>4,"Malakir Soothsayer"=>5,"Munda's Vanguard"=>5,"Ondu War Cleric"=>2,"Spawnbinder Mage"=>4,"Stoneforge Acolyte"=>1,"Zada's Commando"=>2,"Zulaport Chainmage"=>4,"Cliffside Lookout"=>1,_=>panic!("unrecognized fixture source")};
    if e["mana_paid"]!=cost{return Err(format!("source cost wrong:{e}"));}
    Ok((find(g,name)?,e))
}
fn activate(g:&mut GameState,action:LegalAction,dm:&mut Choices)->Result<Value,String>{
    let mut q=TriggerQueue::new();let mut state=PriorityLoopState::new(g.players_in_game());
    let mut progress=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),dm).map_err(|e|e.to_string())?;
    for _ in 0..24{
        if state.pending_activation.is_none()&&!g.stack.is_empty(){break;}
        let GameProgress::NeedsDecisionCtx(ctx)=progress else{return Err(format!("announcement stalled:{progress:?}"));};
        if matches!(ctx,DecisionContext::Priority(_)){return Err("announcement returned priority without stack".into());}
        progress=apply_decision_context_with_dm(g,&mut q,&mut state,&ctx,dm).map_err(|e|e.to_string())?;
    }
    if g.stack.is_empty(){return Err("activation announcement budget".into());}
    let announced=json!({"stack_targets":format!("{:?}",g.stack.last().unwrap().targets)});
    finish(g,&mut q,dm)?;
    Ok(announced)
}
fn run(defs:&HashMap<String,CardDefinition>,name:&str,mode:&str)->Result<(Value,Value),String>{
    let mut g=setup(3,0);let mut dm=Choices{object:None,player:PlayerId(1),chosen:PlayerId(0),trace:vec![]};
    for p in [PlayerId(0),PlayerId(1),PlayerId(2)]{for _ in 0..8{let d=CardDefinitionBuilder::new(CardId::new(),"Cohort library witness").card_types(vec![CardType::Instant]).build();g.create_object_from_definition(&d,p,Zone::Library);}}
    let (source,source_cast)=establish(&mut g,defs,name,0,&mut dm)?;
    let mut companion=None;let mut companion_cast=Value::Null;
    if ["ready_ally","tapped_ally","source_new"].contains(&mode){let (id,e)=establish(&mut g,defs,"Cliffside Lookout",0,&mut dm)?;companion=Some(id);companion_cast=e;}
    if mode!="source_new"{g.next_turn();ironsmith::turn::execute_untap_step(&mut g);main_phase(&mut g,0);}
    if mode=="new_ally"{let(id,e)=establish(&mut g,defs,"Cliffside Lookout",0,&mut dm)?;companion=Some(id);companion_cast=e;}
    if mode=="opponent_ally"{let(id,e)=establish(&mut g,defs,"Cliffside Lookout",1,&mut dm)?;companion=Some(id);companion_cast=e;main_phase(&mut g,0);}
    if mode=="nonally"{let d=CardDefinitionBuilder::new(CardId::new(),"Cohort non-Ally witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,6)).build();companion=Some(g.create_object_from_definition(&d,PlayerId(0),Zone::Battlefield));}
    if mode=="tapped_ally"{g.tap(companion.unwrap());}
    let targetdef=CardDefinitionBuilder::new(CardId::new(),"Cohort target witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,6)).build();
    let target=g.create_object_from_definition(&targetdef,PlayerId(1),Zone::Battlefield);
    if name=="Spawnbinder Mage"{dm.object=Some(target);}
    // These are resources for the effect, not substitutes for its cost-produced context.
    if name=="Akoum Flameseeker"{let d=CardDefinitionBuilder::new(CardId::new(),"Cohort discard witness").card_types(vec![CardType::Instant]).build();g.create_object_from_definition(&d,PlayerId(0),Zone::Hand);}
    if g.is_summoning_sick(source)!=(mode=="source_new"){return Err("source sickness fixture invalid".into());}
    let before=json!({"source_tapped":g.is_tapped(source),"source_summoning_sick":g.is_summoning_sick(source),"companion_tapped":companion.map(|id|g.is_tapped(id)),"companion_summoning_sick":companion.map(|id|g.is_summoning_sick(id)),"hand":g.player(PlayerId(0)).unwrap().hand.len(),"library":g.player(PlayerId(0)).unwrap().library.len(),"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>()});
    g.turn.priority_player=Some(PlayerId(0));
    let actions:Vec<_>=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().filter(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source)).collect();
    let offered=!actions.is_empty();let mut response=Value::Null;
    let error=if let Some(action)=actions.first(){match activate(&mut g,action.clone(),&mut dm){Ok(e)=>{response=e;None},Err(e)=>Some(e)}}else{None};
    let actual=json!({"activation_offered":offered,"activation_error":error,"source_tapped":g.is_tapped(source),"companion_tapped":companion.map(|id|g.is_tapped(id)),"companion_zone":companion.and_then(|id|g.object(id).map(|o|format!("{:?}",o.zone)))});
    Ok((actual,json!({"source_cast":source_cast,"companion_cast":companion_cast,"state_before":before,"offered_actions":format!("{actions:?}"),"announcement":response,"state_after":{"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"hand":g.player(PlayerId(0)).unwrap().hand.len(),"library":g.player(PlayerId(0)).unwrap().library.len(),"target_tapped":g.is_tapped(target)},"choices":dm.trace})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="audit reporter, inspect reviewed outcomes; not blanket card correctness"]
fn report_cohort(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
    let names=["Akoum Flameseeker","Drana's Chosen","Malakir Soothsayer","Munda's Vanguard","Ondu War Cleric","Spawnbinder Mage","Stoneforge Acolyte","Zada's Commando","Zulaport Chainmage"];
    for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !names.contains(&n)&&n!="Cliffside Lookout"{continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
    let mut rows=vec![];
    for name in names{for mode in ["ready_ally","new_ally","no_ally","tapped_ally","opponent_ally","nonally","source_new"]{
        let valid=["ready_ally","new_ally"].contains(&mode);let companion=mode!="no_ally";
        let expected=json!({"activation_offered":valid,"activation_error":null,"source_tapped":valid,"companion_tapped":if companion{Some(valid||mode=="tapped_ally")}else{None},"companion_zone":if companion{Some("Battlefield")}else{None}});
        let(status,actual,evidence)=match run(&defs,name,mode){Ok((a,e))=>(if a==expected{"expected_result_observed"}else if !a["activation_error"].is_null(){if valid{"action_failed"}else{"unpayable_action_offered"}}else{"semantic_mismatch"},a,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
        rows.push(json!({"card":name,"scenario":{"companion_mode":mode},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
    }}
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Exact canonical paid Cohort sources and Cliffside Lookout companion, actual next-turn transitions, ready/new/tapped/opponent/non-Ally/absent resource controls; printed source tap cost is never bypassed. Outcomes focus on legality and cost payment; effect state is recorded as evidence, not a whole-card oracle.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_cohort_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/cohort-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());
}
