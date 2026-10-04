//! Canonical counter-removal costs with actual producers and explicit full payment choices.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, SelectOptionsContext, TargetsContext, SelectObjectsContext, CountersContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;


struct Choices{targets:Vec<Target>,counter_objects:Vec<ObjectId>,trace:Vec<Value>}
impl DecisionMaker for Choices{
 fn answers_player_choices(&self)->bool{true}
 fn decide_targets(&mut self,_:&GameState,c:&TargetsContext)->Vec<Target>{self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));self.targets.clone()}
 fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{let s=ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,c);self.trace.push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));s}
 fn decide_objects(&mut self,g:&GameState,c:&SelectObjectsContext)->Vec<ObjectId>{let preferred:Vec<_>=self.counter_objects.iter().copied().filter(|id|c.candidates.iter().any(|o|o.id==*id&&o.legal)).collect();let s=if preferred.is_empty(){ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g,c)}else{preferred.into_iter().take(c.max.unwrap_or(self.counter_objects.len())).collect()};self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{s:?}")}));s}
 fn decide_distribute(&mut self,g:&GameState,c:&ironsmith::decisions::context::DistributeContext)->Vec<(Target,u32)>{let mut needed=c.total;let mut chosen=vec![];for id in &self.counter_objects{if c.targets.iter().any(|t|t.target==Target::Object(*id)){let available=g.object(*id).map(|o|o.counters.values().copied().sum::<u32>()).unwrap_or(0);let take=available.min(needed);if take>0{chosen.push((Target::Object(*id),take));needed-=take;}}}self.trace.push(json!({"choice":"counter-object-distribution","context":format!("{c:?}"),"selected":format!("{chosen:?}"),"unavailable_required":needed}));chosen}
 fn decide_counters(&mut self,_:&GameState,c:&CountersContext)->Vec<(ironsmith::CounterType,u32)>{let mut needed=c.max_total;let mut s=vec![];for (kind,n)in &c.available_counters{let take=u32::try_from(needed.min(u64::from(*n))).unwrap();if take>0{s.push((*kind,take));needed-=u64::from(take);}}self.trace.push(json!({"choice":"counters","context":format!("{c:?}"),"selected":format!("{s:?}"),"unavailable_required":needed}));s}
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


const NAMES:[&str;6]=["Bolrac-Clan Crusher","Brambleback Brute","Burdened Stoneback","Gnarlbark Elm","Ghost-Spider","Hopeful Initiate"];
fn finish(g:&mut GameState,q:&mut TriggerQueue,dm:&mut Choices)->Result<(),String>{let mut state=PriorityLoopState::new(g.players_in_game());for _ in 0..24{advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;if g.stack.is_empty(){return Ok(());}state.reset_for_new_priority_window(g);for _ in 0..g.players_in_game(){apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm).map_err(|e|e.to_string())?;}}Err("finish budget".into())}
fn activate(g:&mut GameState,source:ObjectId,dm:&mut Choices)->Result<Value,String>{
 let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source)).ok_or("intended activation unavailable")?;
 let before=g.player(PlayerId(0)).unwrap().mana_pool.total();let mut q=TriggerQueue::new();let mut st=PriorityLoopState::new(g.players_in_game());let mut progress=apply_priority_response_with_dm(g,&mut q,&mut st,&PriorityResponse::PriorityAction(action.clone()),dm).map_err(|e|e.to_string())?;
 for _ in 0..24{if st.pending_activation.is_none()&&!g.stack.is_empty(){break;}let GameProgress::NeedsDecisionCtx(ctx)=progress else{return Err(format!("activation stopped:{progress:?}"));};if matches!(ctx,DecisionContext::Priority(_)){return Err("activation returned priority without stack".into());}progress=apply_decision_context_with_dm(g,&mut q,&mut st,&ctx,dm).map_err(|e|e.to_string())?;}
 let paid=before-g.player(PlayerId(0)).unwrap().mana_pool.total();let error=finish(g,&mut q,dm).err();Ok(json!({"action":format!("{action:?}"),"mana_paid":paid,"resolution_error":error}))
}
fn run(defs:&HashMap<String,CardDefinition>,name:&str,variant:usize)->Result<(Value,Value,Value),String>{
 use ironsmith::CounterType;use ironsmith::static_abilities::StaticAbilityId;
 let mut g=setup(3,0);let wd=CardDefinitionBuilder::new(CardId::new(),"Counter-cost creature witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(5,8)).build();let witness=g.create_object_from_definition(&wd,PlayerId(0),Zone::Battlefield);let target=g.create_object_from_definition(&wd,PlayerId(1),Zone::Battlefield);
 let ad=CardDefinitionBuilder::new(CardId::new(),"Counter-cost artifact witness").card_types(vec![CardType::Artifact]).build();let artifact=g.create_object_from_definition(&ad,PlayerId(1),Zone::Battlefield);
 g.create_object_from_definition(&defs["Plains"],PlayerId(0),Zone::Library);
 let mut dm=Choices{targets:vec![],counter_objects:vec![],trace:vec![]};let source_cast=cast(&mut g,&defs[name],0,&mut dm)?;
 let source=*g.battlefield.iter().find(|id|g.object(**id).is_some_and(|o|o.name==name)).ok_or("source absent after actual cast")?;
 let source_cost=match name{"Bolrac-Clan Crusher"|"Ghost-Spider"=>5,"Brambleback Brute"|"Gnarlbark Elm"=>3,"Burdened Stoneback"=>2,_=>1};if source_cast["mana_paid"]!=source_cost{return Err(format!("source payment wrong:{source_cast}"));}
 let positive=["Bolrac-Clan Crusher","Ghost-Spider","Hopeful Initiate"].contains(&name);let mut producer=Value::Null;
 let (first,second)=if name=="Bolrac-Clan Crusher"&&variant==1{(witness,witness)}else if name=="Hopeful Initiate"&&variant==1{(source,witness)}else{(source,source)};
 if positive{dm.targets=vec![Target::Object(first),Target::Object(second)];producer=cast(&mut g,&defs["Common Bond"],0,&mut dm)?;if producer["mana_paid"]!=3{return Err("Common Bond payment wrong".into());}if g.counter_count(first,CounterType::PlusOnePlusOne)!=if first==second{2}else{1}{return Err("Common Bond did not establish exact counters".into());}}
 else if g.counter_count(source,CounterType::MinusOneMinusOne)!=2{return Err("actual ETB did not establish two minus counters".into());}
 if name=="Bolrac-Clan Crusher"{g.next_turn();ironsmith::turn::execute_untap_step(&mut g);while g.turn.active_player!=PlayerId(0){g.next_turn();ironsmith::turn::execute_untap_step(&mut g);}g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;g.turn.priority_player=Some(PlayerId(0));}
 dm.counter_objects=if first==second{vec![first]}else{vec![first,second]};
 dm.targets=match name{"Bolrac-Clan Crusher"=>vec![Target::Player(PlayerId(1))],"Ghost-Spider"=>vec![],"Hopeful Initiate"=>vec![Target::Object(artifact)],_=>vec![Target::Object(target)]};
 let times=if ["Brambleback Brute","Burdened Stoneback"].contains(&name){variant+1}else{1};let mut activations=vec![];for _ in 0..times{activations.push(activate(&mut g,source,&mut dm)?);}
 let expected_paid=match name{"Brambleback Brute"|"Burdened Stoneback"=>2,"Gnarlbark Elm"|"Hopeful Initiate"=>3,_=>0};if activations.iter().any(|a|a["mana_paid"]!=expected_paid){return Err(format!("activation payment wrong:{activations:?}"));}
 let actual=json!({"error":activations.iter().find_map(|a|a["resolution_error"].as_str()),"source_plus":g.counter_count(source,CounterType::PlusOnePlusOne),"witness_plus":g.counter_count(witness,CounterType::PlusOnePlusOne),"source_minus":g.counter_count(source,CounterType::MinusOneMinusOne),"source_tapped":g.is_tapped(source),"bob_life":g.player(PlayerId(1)).unwrap().life,"target_power":g.calculated_power(target),"target_toughness":g.calculated_toughness(target),"target_can_block":g.can_block(target),"target_indestructible":g.object_has_static_ability_id(target,StaticAbilityId::Indestructible),"artifact_battlefield":g.object(artifact).is_some_and(|o|o.zone==Zone::Battlefield),"plains_exiled":g.objects_in_deterministic_order().iter().any(|o|o.name=="Plains"&&o.zone==Zone::Exile)});
 let expected=json!({"error":null,"source_plus":if name=="Bolrac-Clan Crusher"&&variant==0{1}else{0},"witness_plus":if name=="Bolrac-Clan Crusher"&&variant==1{1}else{0},"source_minus":if ["Brambleback Brute","Burdened Stoneback"].contains(&name){2-times}else{0},"source_tapped":name=="Bolrac-Clan Crusher","bob_life":if name=="Bolrac-Clan Crusher"{18}else{20},"target_power":if name=="Gnarlbark Elm"{3}else{5},"target_toughness":if name=="Gnarlbark Elm"{6}else{8},"target_can_block":name!="Brambleback Brute","target_indestructible":name=="Burdened Stoneback","artifact_battlefield":name!="Hopeful Initiate","plains_exiled":name=="Ghost-Spider"});
 Ok((expected,actual,json!({"source_cast":source_cast,"counter_producer":producer,"activations":activations,"choices":dm.trace})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="explicit counter-removal decision audit"]
fn report_counter_choices(){let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
 for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !NAMES.contains(&n)&&!["Common Bond","Plains"].contains(&n){continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
 let mut rows=vec![];for name in NAMES{let variants=if ["Ghost-Spider","Gnarlbark Elm"].contains(&name){1}else{2};for variant in 0..variants{let(status,expected,actual,evidence)=match run(&defs,name,variant){Ok((e,a,f))=>(if e==a{"expected_result_observed"}else if !a["error"].is_null(){"execution_failed"}else{"semantic_mismatch"},e,a,f),Err(e)=>("fixture_or_producer_error",Value::Null,json!({"error":e}),Value::Null)};rows.push(json!({"card":name,"scenario":{"variant":variant},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));}}
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Actual paid canonical sources, inherent ETB counters or paid Common Bond; explicit legal object and counter removal decisions, exact costs/counters/immediate effects. Bolrac waits a full turn cycle with actual untap. Ghost exile result checked, not subsequent permission use. Bramble block eligibility queried, not full combat.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_counter_choice_reproductions.rs")),"runtime_stack_bytes":67108864}});std::fs::write(root.join("reports/runtime-audit/counter-choice-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());}
