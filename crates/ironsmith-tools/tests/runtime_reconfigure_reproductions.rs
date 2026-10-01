//! Reconfigure dispatch, attachment and sorcery timing probes.
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
    fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool{self.trace.push(json!({"choice":"boolean","context":format!("{ctx:?}"),"answer":true}));true}
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
    let mut g=setup(3,0);let witness=CardDefinitionBuilder::new(CardId::new(),"Reconfigure attachment witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,6)).build();
    let target=g.create_object_from_definition(&witness,PlayerId(0),Zone::Battlefield);
    let mut dm=Choices{object:Some(target),player:PlayerId(1),chosen:PlayerId(0),trace:vec![]};
    for p in [PlayerId(0),PlayerId(1),PlayerId(2)]{for _ in 0..8{let d=CardDefinitionBuilder::new(CardId::new(),"Reconfigure library witness").card_types(vec![CardType::Instant]).build();g.create_object_from_definition(&d,p,Zone::Library);}}
    let mut producer=Value::Null;
    let attached=mode=="detach";
    if attached{producer=cast(&mut g,&defs["Hammer of Nazahn"],0,&mut dm)?;if producer["mana_paid"]!=4||!producer["resolution_error"].is_null(){return Err(format!("Hammer setup failed:{producer}"));}}
    let source_cast=cast(&mut g,&defs[name],0,&mut dm)?;
    if !source_cast["resolution_error"].is_null(){return Err(format!("source cast failed:{source_cast}"));}
    if source_cast["mana_paid"]!=if name=="Rabbit Battery"{1}else{2}{return Err(format!("source payment wrong:{source_cast}"));}
    let source=find(&g,name)?;
    let initial_attachment=g.object(source).unwrap().attached_to;
    if attached&&initial_attachment!=Some(ironsmith::object::AttachmentTarget::Object(target)){return Ok((json!({"setup_error":"Hammer did not attach source Equipment; detach not attempted"}),json!({"setup_failure":true,"source_cast":source_cast,"attachment_producer":producer,"initial_attachment":format!("{initial_attachment:?}"),"choices":dm.trace})));}
    if !attached&&initial_attachment.is_some(){return Err("source unexpectedly attached".into());}
    if mode=="end_step"{g.turn.phase=ironsmith::Phase::Ending;g.turn.step=Some(ironsmith::Step::End);}
    if mode=="opponent_turn"{g.next_turn();ironsmith::turn::execute_untap_step(&mut g);g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;}
    g.turn.priority_player=Some(PlayerId(0));
    let mut actions:Vec<_>=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().filter(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source)).collect();
    actions.sort_by_key(|a|match a{LegalAction::ActivateAbility{ability_index,..}=>*ability_index,_=>usize::MAX});
    let chosen=if attached{actions.last()}else{actions.first()}.cloned();
    let advertised_lookup=chosen.as_ref().map(|a|match a{LegalAction::ActivateAbility{ability_index,..}=>format!("{:?}",g.current_ability(source,*ability_index)),_=>String::new()});
    let current_abilities=format!("{:?}",g.current_abilities(source));
    let mana=g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut announced=Value::Null;let error=if let Some(action)=chosen.clone(){match activate(&mut g,action,&mut dm){Ok(e)=>{announced=e;None},Err(e)=>Some(e)}}else{None};
    let actual=json!({"activation_offered":chosen.is_some(),"activation_error":error,"attached_to_witness":g.object(source).unwrap().attached_to==Some(ironsmith::object::AttachmentTarget::Object(target)),"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total()});
    Ok((actual,json!({"source_cast":source_cast,"attachment_producer":producer,"initial_attachment":format!("{initial_attachment:?}"),"offered_actions":format!("{actions:?}"),"selected_action":format!("{chosen:?}"),"announcement":announced,"choices":dm.trace,"dispatcher_current_ability":advertised_lookup,"current_abilities_before_dispatch":current_abilities,"source_base_abilities":format!("{:?}",g.object(source).unwrap().abilities)})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="audit reporter, inspect outcomes and fixture validity"]
fn report_reconfigure(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
    let names=["The Reality Chip","Rabbit Battery","Lizard Blades"];
    for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !names.contains(&n)&&n!="Hammer of Nazahn"{continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
    let mut rows=vec![];
    for name in names{for mode in ["attach","detach","end_step","opponent_turn"]{
        let valid=["attach","detach"].contains(&mode);let cost=match name{"The Reality Chip"=>3,"Rabbit Battery"=>1,_=>2};
        let expected=json!({"activation_offered":valid,"activation_error":null,"attached_to_witness":mode=="attach","mana_paid":if valid{cost}else{0}});
        let(status,actual,evidence)=match run(&defs,name,mode){Ok((a,e))=>(if e["setup_failure"]==true{"fixture_or_producer_error"}else if a==expected{"expected_result_observed"}else if !a["activation_error"].is_null(){"action_failed"}else{"semantic_mismatch"},a,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
        rows.push(json!({"card":name,"scenario":{"action":mode},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
    }}
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Exact canonical paid sources and Hammer of Nazahn attachment producer. Actual advertised reconfigure dispatch in own main, own endstep and opponent turn; attach/detach payment and attachment assertions. No injected attachment or effect context.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_reconfigure_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/reconfigure-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());
}
