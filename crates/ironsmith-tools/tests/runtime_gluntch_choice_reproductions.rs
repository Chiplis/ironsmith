//! Empty/exhausted player-choice candidates with real end-step triggers.
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
        candidates.sort_by_key(|c|!game.object(c.id).is_some_and(|o|o.card_types.contains(&CardType::Land)));
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
fn finish(g:&mut GameState,q:&mut TriggerQueue,dm:&mut Choices)->Result<(),String>{
    let mut state=PriorityLoopState::new(g.players_in_game());
    for _ in 0..24{
        advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;
        if g.stack.is_empty(){return Ok(());}
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game(){apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm).map_err(|e|e.to_string())?;}
    }
    Err("resolution budget".into())
}
fn phase(g:&mut GameState,end:bool,dm:&mut Choices)->Result<(),String>{
    g.turn.phase=if end{ironsmith::Phase::Ending}else{ironsmith::Phase::Combat};
    g.turn.step=Some(if end{ironsmith::Step::End}else{ironsmith::Step::BeginCombat});
    let mut q=TriggerQueue::new();ironsmith::game_loop::generate_and_queue_step_triggers(g,&mut q);finish(g,&mut q,dm)
}

fn run(defs:&HashMap<String,CardDefinition>,players:usize,first:u8,own_end:bool,no_creature:bool)->Result<(Value,Value),String>{
    let mut g=setup(players,0);
    for p in (0..players).map(|i|PlayerId(i as u8)){
        for _ in 0..8{let d=CardDefinitionBuilder::new(CardId::new(),"Player-choice library witness").card_types(vec![CardType::Instant]).build();g.create_object_from_definition(&d,p,Zone::Library);}
        if !(no_creature&&p==PlayerId(first)){let d=CardDefinitionBuilder::new(CardId::new(),"Player-choice creature witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,6)).build();g.create_object_from_definition(&d,p,Zone::Battlefield);}
    }
    let mut dm=Choices{object:None,player:PlayerId(1),chosen:PlayerId(first),trace:vec![]};
    let paid=cast(&mut g,&defs["Gluntch, the Bestower"],0,&mut dm)?;
    if paid["mana_paid"]!=3||!paid["resolution_error"].is_null(){return Err(format!("source cast failed:{paid}"));}
    if !own_end{g.next_turn();ironsmith::turn::execute_untap_step(&mut g);}
    let error=phase(&mut g,true,&mut dm).err();
    let counters:Vec<u32>=(0..players).map(|p|g.battlefield.iter().filter_map(|id|g.object(*id)).filter(|o|g.controller_of(o)==PlayerId(p as u8)).map(|o|o.counters.iter().filter(|(k,_)|k.description()==ironsmith::object::CounterType::PlusOnePlusOne.description()).map(|(_,n)|*n).sum::<u32>()).sum()).collect();
    let treasures:Vec<_>=(0..players).map(|p|g.battlefield.iter().filter_map(|id|g.object(*id)).filter(|o|g.controller_of(o)==PlayerId(p as u8)&&o.name=="Treasure").count()).collect();
    let actual=json!({"resolution_error":error,"plus_one_counters_by_player":counters,"hand":g.players.iter().map(|p|p.hand.len()).collect::<Vec<_>>(),"treasures":treasures});
    Ok((actual,json!({"source_cast":paid,"active_player":g.turn.active_player.0,"choices":dm.trace})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="end-step player-choice audit reporter"]
fn report_gluntch(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
    for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if n!="Gluntch, the Bestower"{continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
    let mut rows=vec![];
    for (players,first,own_end,no_creature) in [(2,0,true,false),(2,1,true,false),(3,0,true,false),(3,1,true,false),(3,2,true,false),(2,0,false,false),(3,0,false,false),(2,1,true,true),(3,1,true,true)]{
        let mut counters=vec![0;players];let mut hand=vec![0;players];let mut treasures=vec![0;players];
        if own_end{if !no_creature{counters[first as usize]=2;}let second=(0..players).find(|p|*p!=first as usize).unwrap();hand[second]=1;if let Some(third)=(0..players).find(|p|*p!=first as usize&&*p!=second){treasures[third]=2;}}
        let expected=json!({"resolution_error":null,"plus_one_counters_by_player":counters,"hand":hand,"treasures":treasures});
        let(status,actual,evidence)=match run(&defs,players,first,own_end,no_creature){Ok((a,e))=>(if a==expected{"expected_result_observed"}else if !a["resolution_error"].is_null(){"resolution_failed"}else{"semantic_mismatch"},a,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
        rows.push(json!({"card":"Gluntch, the Bestower","scenario":{"players":players,"first_player":first,"own_end_step":own_end,"first_player_has_no_creature":no_creature},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
    }
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Strict paid Gluntch and real end-step trigger generation; two/three-player distinct-choice exhaustion, all choices supplied through legal callbacks, empty-creature first-player and other-player-endstep controls. Exact per-player counters/cards/Treasures.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_gluntch_choice_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/gluntch-choice-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());
}
