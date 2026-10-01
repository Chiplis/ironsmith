//! Canonical Aura player bindings and reflexive-payment execution contexts.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, DecisionContext, SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct Choices { object: Option<ObjectId>, player: PlayerId, accept: bool, trace: Vec<Value> }
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.trace.push(json!({"choice":"boolean","context":format!("{ctx:?}"),"selected":self.accept}));
        self.accept
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
fn cast(g: &mut GameState, def: &CardDefinition, dm: &mut Choices) -> Result<Value,String> {
    g.turn.priority_player=Some(PlayerId(0));
    let source=g.create_object_from_definition(def,PlayerId(0),Zone::Hand);
    let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).ok_or("intended cast unavailable")?;
    let mana=g.player(PlayerId(0)).unwrap().mana_pool.total();
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
    let mut evidence=json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",g.stack.last().unwrap().targets),"resolution_error":null});
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
fn libraries(g:&mut GameState){for p in [PlayerId(0),PlayerId(1),PlayerId(2)]{for _ in 0..16{let d=CardDefinitionBuilder::new(CardId::new(),"Aura context library witness").card_types(vec![CardType::Instant]).build();g.create_object_from_definition(&d,p,Zone::Library);}}}
fn count(g:&GameState,p:u8,z:Zone)->usize{g.objects_in_deterministic_order().iter().filter(|o|o.owner==PlayerId(p)&&o.zone==z).count()}
fn fraying(defs:&HashMap<String,CardDefinition>,enchanted:u8,mill:Option<u8>,active:u8,old_grave:usize)->Result<(Value,Value),String>{
    let mut g=setup(3,0);libraries(&mut g);
    for _ in 0..old_grave{let d=CardDefinitionBuilder::new(CardId::new(),"Previous turn grave witness").card_types(vec![CardType::Instant]).build();g.create_object_from_definition(&d,PlayerId(enchanted),Zone::Graveyard);}
    // These are established previous-turn cards, not entry events this turn.
    g.turn_store.turn_history=Default::default();
    let mut dm=Choices{object:None,player:PlayerId(enchanted),accept:false,trace:vec![]};
    let source=cast(&mut g,&defs["Fraying Sanity"],&mut dm)?;
    if !source["resolution_error"].is_null(){return Ok((json!({"resolution_error":source["resolution_error"]}),json!({"source_cast":source,"choices":dm.trace})));}
    while g.turn.active_player!=PlayerId(active){g.next_turn();}
    g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;
    g.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::Blue,5);
    let producer=if let Some(p)=mill{dm.player=PlayerId(p);cast(&mut g,&defs["Thought Scour"],&mut dm)?}else{Value::Null};
    if !producer.is_null()&&!producer["resolution_error"].is_null(){return Err(format!("mill producer failed:{producer}"));}
    let before:Vec<_>=(0..3).map(|p|count(&g,p,Zone::Graveyard)).collect();
    let error=phase(&mut g,true,&mut dm).err();
    Ok((json!({"resolution_error":error,"enchanted_graveyard":count(&g,enchanted,Zone::Graveyard),"enchanted_library":count(&g,enchanted,Zone::Library)}),json!({"source_cast":source,"producer_cast":producer,"graveyards_before_end_step":before,"active_player":active,"choices":dm.trace,"old_graveyard_cards":old_grave})))
}
fn quantum(defs:&HashMap<String,CardDefinition>,accept:bool,own_creature:bool,end_step:bool,own_turn:bool)->Result<(Value,Value),String>{
    let mut g=setup(3,0);
    let d=CardDefinitionBuilder::new(CardId::new(),"Quantum blink witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,6)).build();
    let target=g.create_object_from_definition(&d,PlayerId(u8::from(!own_creature)),Zone::Battlefield);
    let stable=g.object(target).unwrap().stable_id;
    let mut dm=Choices{object:Some(target),player:PlayerId(1),accept:if end_step{false}else{accept},trace:vec![]};
    let initial=g.player(PlayerId(0)).unwrap().mana_pool.total();
    let source=cast(&mut g,&defs["Quantum Entanglement"],&mut dm)?;
    let mut error=source["resolution_error"].as_str().map(str::to_string);
    let mut phase_paid=0;
    if end_step&&error.is_none(){
        if !own_turn{g.next_turn();}
        // Explicit resources in the end-step priority window.
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::White,5);g.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::Colorless,5);
        dm.accept=accept;let before=g.player(PlayerId(0)).unwrap().mana_pool.total();
        error=phase(&mut g,true,&mut dm).err();phase_paid=before-g.player(PlayerId(0)).unwrap().mana_pool.total();
    }
    let after=g.find_object_by_stable_id(stable);
    let actual=json!({"resolution_error":error,"target_blinked":after.is_some_and(|id|id!=target),"target_battlefield":after.is_some_and(|id|g.object(id).is_some_and(|o|o.zone==Zone::Battlefield))});
    Ok((actual,json!({"source_cast":source,"own_target":own_creature,"etb_total_mana_spent":if end_step{Value::Null}else{json!(initial-g.player(PlayerId(0)).unwrap().mana_pool.total())},"end_step_mana_spent":phase_paid,"choices":dm.trace})))
}
fn overencumbered(defs:&HashMap<String,CardDefinition>,enchanted:u8,accept:bool,begin_combat:bool,extra:usize,other_turn:bool)->Result<(Value,Value),String>{
    let mut g=setup(3,0);
    let mut dm=Choices{object:None,player:PlayerId(enchanted),accept,trace:vec![]};
    let source=cast(&mut g,&defs["Overencumbered"],&mut dm)?;
    let mut error=source["resolution_error"].as_str().map(str::to_string);
    let tokens:Vec<_>=g.battlefield.iter().filter_map(|id|g.object(*id)).filter(|o|o.kind==ironsmith::object::ObjectKind::Token).map(|o|json!({"name":o.name.to_string(),"controller":g.controller_of(o).0,"subtypes":format!("{:?}",o.subtypes)})).collect();
    let mut evidence=json!({"source_cast":source,"tokens":tokens});
    let mut payment=Value::Null;let mut allowed=Value::Null;
    if begin_combat&&error.is_none(){
        use ironsmith::combat_state::{AttackTarget,CombatState};
        use ironsmith::decision::AttackerDeclaration;
        for _ in 0..extra{let d=CardDefinitionBuilder::new(CardId::new(),"Additional tax artifact").card_types(vec![CardType::Artifact]).build();g.create_object_from_definition(&d,PlayerId(enchanted),Zone::Battlefield);}
        let active=if other_turn{if enchanted==1{2}else{1}}else{enchanted};
        let d=CardDefinitionBuilder::new(CardId::new(),"Tax attack witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,6)).build();
        let attacker=g.create_object_from_definition(&d,PlayerId(active),Zone::Battlefield);
        while g.turn.active_player!=PlayerId(active){g.next_turn();}
        g.remove_summoning_sickness(attacker);
        g.player_mut(PlayerId(enchanted)).unwrap().mana_pool.add(ManaSymbol::Colorless,(3+extra) as u32);
        let before=g.player(PlayerId(enchanted)).unwrap().mana_pool.total();
        error=phase(&mut g,false,&mut dm).err();
        payment=json!(before-g.player(PlayerId(enchanted)).unwrap().mana_pool.total());
        if error.is_none(){
            g.turn.step=Some(ironsmith::Step::DeclareAttackers);
            let mut cs=CombatState::default();let mut q=TriggerQueue::new();
            let attempt=ironsmith::game_loop::apply_attacker_declarations_with_dm(&mut g,&mut cs,&mut q,&[AttackerDeclaration{creature:attacker,target:AttackTarget::Player(PlayerId(0))}],&mut dm);
            allowed=json!(attempt.is_ok());evidence["attack_error"]=json!(attempt.err().map(|e|e.to_string()));
        }
        evidence["active_player"]=json!(active);evidence["artifacts_at_combat"]=json!(3+extra);
    }
    let actual=json!({"resolution_error":error,"enchanted_token_count":tokens.iter().filter(|t|t["controller"]==enchanted).count(),"other_token_count":tokens.iter().filter(|t|t["controller"]!=enchanted).count(),"mana_paid":payment,"attack_accepted":allowed});
    evidence["choices"]=json!(dm.trace);
    Ok((actual,evidence))
}
fn add(rows:&mut Vec<Value>,name:&str,scenario:Value,expected:Value,result:Result<(Value,Value),String>){
    let(status,actual,evidence)=match result{Ok((a,e))=>(if a==expected{"expected_result_observed"}else if !a["resolution_error"].is_null(){"resolution_failed"}else{"semantic_mismatch"},a,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
    rows.push(json!({"card":name,"scenario":scenario,"expected":expected,"actual":actual,"status":status,"fixture_evidence":evidence}));
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="expected-result audit reporter"]
fn report_aura_reflexive_context(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
    for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !["Quantum Entanglement","Fraying Sanity","Overencumbered","Thought Scour"].contains(&n){continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
    let mut rows=vec![];
    for (target,mill,active,old) in [(1,None,0,0),(1,Some(1),0,0),(1,Some(2),0,0),(2,Some(2),0,0),(1,Some(1),1,0),(1,None,0,3)]{
        let milled=usize::from(mill==Some(target))*2;
        add(&mut rows,"Fraying Sanity",json!({"enchanted":target,"milled_player":mill,"active":active,"previous_graveyard":old}),json!({"resolution_error":null,"enchanted_graveyard":old+2*milled,"enchanted_library":16-2*milled}),fraying(&defs,target,mill,active,old));
    }
    for (accept,creature,end,own) in [(false,true,false,true),(true,true,false,true),(false,false,false,true),(true,false,false,true),(true,true,true,true),(false,true,true,true),(true,true,true,false)]{
        add(&mut rows,"Quantum Entanglement",json!({"accept":accept,"own_creature":creature,"end_step":end,"own_turn":own}),json!({"resolution_error":null,"target_blinked":accept&&creature&&(!end||own),"target_battlefield":true}),quantum(&defs,accept,creature,end,own));
    }
    for (p,accept,combat,extra,other) in [(1,false,false,0,false),(1,true,false,0,false),(2,false,false,0,false),(2,true,false,0,false),(1,false,true,0,false),(1,true,true,0,false),(2,false,true,2,false),(2,true,true,2,false),(1,false,true,0,true),(1,true,true,0,true),(2,false,true,2,true),(2,true,true,2,true)]{
        add(&mut rows,"Overencumbered",json!({"enchanted":p,"optional_policy":accept,"begin_combat":combat,"extra_artifacts":extra,"other_players_turn":other}),json!({"resolution_error":null,"enchanted_token_count":3,"other_token_count":0,"mana_paid":if combat{json!(if accept&&!other{3+extra}else{0})}else{Value::Null},"attack_accepted":if combat{json!(accept||other)}else{Value::Null}}),overencumbered(&defs,p,accept,combat,extra,other));
    }
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let report=json!({"scope":"Strict canonical paid Aura/enchantment casts and actual Thought Scour; normal priority resolution. End-step positions use engine step-trigger generation after explicit turn transitions, with current-turn graveyard history intact.","rows":rows,"artifacts":artifacts,"provenance":{"inventory_sha256":hash(&input),"binary":binary,"binary_sha256":hash(&binary),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_aura_reflexive_context_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/aura-reflexive-context-reproductions.json"),serde_json::to_string_pretty(&report).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&report).unwrap());
}
