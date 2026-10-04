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


struct Choices{amount:u32,mode:usize,targets:Vec<Target>,counter_objects:Vec<ObjectId>,trace:Vec<Value>}
impl DecisionMaker for Choices{
 fn answers_player_choices(&self)->bool{true}
 fn decide_boolean(&mut self,_:&GameState,c:&ironsmith::decisions::context::BooleanContext)->bool{self.trace.push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":true}));true}
 fn decide_number(&mut self,_:&GameState,c:&ironsmith::decisions::context::NumberContext)->u32{let selected=self.amount.max(c.min).min(c.max);self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"selected":selected}));selected}
 fn decide_targets(&mut self,_:&GameState,c:&TargetsContext)->Vec<Target>{self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));self.targets.clone()}
 fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{let mode_word=if self.mode==0{"charge counters"}else{"+1/+1 counters"};let s=if c.options.iter().any(|o|o.legal&&o.description.to_lowercase().contains("untap")){c.options.iter().find(|o|o.legal&&o.description.to_lowercase().contains("untap")).map(|o|vec![o.index]).unwrap()}else if c.options.iter().any(|o|o.description.contains("charge counters"))&&c.options.iter().any(|o|o.description.contains("+1/+1 counters")){c.options.iter().find(|o|o.legal&&o.description.contains(mode_word)).map(|o|vec![o.index]).unwrap()}else{ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,c)};self.trace.push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));s}
 fn decide_objects(&mut self,g:&GameState,c:&SelectObjectsContext)->Vec<ObjectId>{let preferred:Vec<_>=if c.description.to_lowercase().contains("search"){c.candidates.iter().filter(|o|o.legal&&g.object(o.id).is_some_and(|o|o.name=="Plains")).take(1).map(|o|o.id).collect()}else{self.counter_objects.iter().copied().filter(|id|c.candidates.iter().any(|o|o.id==*id&&o.legal)).collect()};let s=if preferred.is_empty(){ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g,c)}else{preferred.into_iter().take(c.max.unwrap_or(self.counter_objects.len())).collect()};self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{s:?}")}));s}
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


fn finish(g:&mut GameState,q:&mut TriggerQueue,dm:&mut Choices)->Result<(),String>{let mut state=PriorityLoopState::new(g.players_in_game());for _ in 0..24{advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;if g.stack.is_empty(){return Ok(());}state.reset_for_new_priority_window(g);for _ in 0..g.players_in_game(){apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm).map_err(|e|e.to_string())?;}}Err("finish budget".into())}
fn activate(g:&mut GameState,source:ObjectId,index:usize,dm:&mut Choices)->Result<Value,String>{
 let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index)).ok_or("intended activation unavailable")?;
 let before=g.player(PlayerId(0)).unwrap().mana_pool.total();let mut q=TriggerQueue::new();let mut st=PriorityLoopState::new(g.players_in_game());let mut progress=apply_priority_response_with_dm(g,&mut q,&mut st,&PriorityResponse::PriorityAction(action.clone()),dm).map_err(|e|e.to_string())?;
 for _ in 0..24{if st.pending_activation.is_none()&&!g.stack.is_empty(){break;}let GameProgress::NeedsDecisionCtx(ctx)=progress else{return Err(format!("activation stopped:{progress:?}"));};if matches!(ctx,DecisionContext::Priority(_)){return Err("activation returned priority without stack".into());}progress=apply_decision_context_with_dm(g,&mut q,&mut st,&ctx,dm).map_err(|e|e.to_string())?;}
 let paid=before-g.player(PlayerId(0)).unwrap().mana_pool.total();let error=finish(g,&mut q,dm).err();Ok(json!({"action":format!("{action:?}"),"mana_paid":paid,"resolution_error":error}))
}
fn find_cost_index(def:&CardDefinition,ordinal:usize)->Result<usize,String>{def.abilities.iter().enumerate().filter(|(_,a)|match &a.kind{ironsmith::ability::AbilityKind::Activated(a)=>a.mana_cost.costs().iter().any(|c|c.effect_ref().is_some_and(|e|e.downcast_ref::<ironsmith::effects::RemoveAnyCountersAmongEffect>().is_some())),_=>false}).nth(ordinal).map(|(i,_)|i).ok_or("intended counter cost index absent".into())}
const NAMES:[&str;8]=["Iron Spider, Stark Upgrade","Quillspike","Reaping Willow","Reckoner Bankbuster","Reckoner Bankbuster // Reckoner Bankbuster","Rift Elemental","Scholar of New Horizons","Sigil of Distinction"];
fn turn_cycle(g:&mut GameState){g.next_turn();ironsmith::turn::execute_untap_step(g);while g.turn.active_player!=PlayerId(0){g.next_turn();ironsmith::turn::execute_untap_step(g);}g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;g.turn.priority_player=Some(PlayerId(0));for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless]{g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color,12);}}
fn run(defs:&HashMap<String,CardDefinition>,name:&str,variant:usize)->Result<(Value,Value,Value),String>{
 use ironsmith::CounterType;
 let mut g=setup(3,0);let wd=CardDefinitionBuilder::new(CardId::new(),"Misc counter resource").card_types(vec![CardType::Artifact,CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(5,8)).build();let witness=g.create_object_from_definition(&wd,PlayerId(0),Zone::Battlefield);
 for _ in 0..4{g.create_object_from_definition(&defs["Plains"],PlayerId(0),Zone::Library);}
 if name=="Scholar of New Horizons"&&variant==1{g.create_object_from_definition(&defs["Plains"],PlayerId(1),Zone::Battlefield);}
 let mut dm=Choices{amount:2,mode:0,targets:vec![],counter_objects:vec![],trace:vec![]};let source_cast=cast(&mut g,&defs[name],0,&mut dm)?;
 let source=*g.battlefield.iter().find(|id|g.object(**id).is_some_and(|o|o.name==defs[name].name())).ok_or("source absent after actual cast")?;
 let bank=name.starts_with("Reckoner Bankbuster");let iron=name=="Iron Spider, Stark Upgrade";let scholar=name=="Scholar of New Horizons";let sigil=name=="Sigil of Distinction";let quill=name=="Quillspike";let rift=name=="Rift Elemental";let willow=name=="Reaping Willow";
 let source_cost=if iron||quill{3}else if willow{4}else if rift{1}else{2};if source_cast["mana_paid"]!=source_cost||!source_cast["resolution_error"].is_null(){return Err(format!("source normal cast failed:{source_cast}"));}
 let mut producers=vec![];let mut resource=source;let mut reanimate=None;
 if iron{dm.targets=if variant==0{vec![Target::Object(source),Target::Object(source)]}else{vec![Target::Object(source),Target::Object(witness)]};let p=cast(&mut g,&defs["Common Bond"],0,&mut dm)?;if p["mana_paid"]!=3{return Err("Common Bond mana".into());}producers.push(p);if g.counter_count(source,CounterType::PlusOnePlusOne)!=if variant==0{2}else{1}{return Err("Common Bond resource count".into());}}
 if quill||rift{let helper=if quill{"Brambleback Brute"}else{"Chronozoa"};let p=cast(&mut g,&defs[helper],0,&mut dm)?;if p["mana_paid"]!=if quill{3}else{4}{return Err("helper producer mana".into());}producers.push(p);resource=*g.battlefield.iter().find(|id|g.object(**id).is_some_and(|o|o.name==helper)).ok_or("resource helper absent")?;if g.counter_count(resource,if quill{CounterType::MinusOneMinusOne}else{CounterType::Time})!=if quill{2}else{3}{return Err("actual resource helper ETB counters".into());}}
 if willow{let p=cast(&mut g,&defs["Grizzly Bears"],0,&mut dm)?;if p["mana_paid"]!=2{return Err("Bears payment".into());}producers.push(p);let bear=*g.battlefield.iter().find(|id|g.object(**id).is_some_and(|o|o.name=="Grizzly Bears")).ok_or("Bears absent")?;dm.targets=vec![Target::Object(bear)];let p=cast(&mut g,&defs["Murder"],0,&mut dm)?;if p["mana_paid"]!=3{return Err("Murder payment".into());}producers.push(p);reanimate=g.objects_in_deterministic_order().iter().find(|o|o.name=="Grizzly Bears"&&o.zone==Zone::Graveyard).map(|o|o.id);if reanimate.is_none(){return Err("Murder did not establish graveyard card".into());}}
 if bank&&g.counter_count(source,CounterType::Charge)!=3{return Err("Bankbuster charge producer".into());}if sigil&&g.counter_count(source,CounterType::Charge)!=2{return Err("Sigil paid X2 counters absent".into());}if scholar{if g.counter_count(source,CounterType::PlusOnePlusOne)!=1{return Err("Scholar entry counter absent".into());}turn_cycle(&mut g);}
 dm.counter_objects=if iron&&variant==1{vec![source,witness]}else{vec![resource]};
 let times=if bank{if variant==0{1}else{3}}else if quill||rift||sigil{variant+1}else{1};let index=find_cost_index(&defs[name],0)?;let mut acts=vec![];
 for round in 0..times{
  if bank&&round>0{dm.targets=vec![Target::Object(source)];let p=cast(&mut g,&defs["Twiddle"],0,&mut dm)?;if p["mana_paid"]!=1||g.is_tapped(source){return Err(format!("Twiddle did not untap:{p}"));}producers.push(p);}
  dm.targets=if sigil{vec![Target::Object(witness)]}else if willow{vec![Target::Object(reanimate.unwrap())]}else{vec![]};let a=activate(&mut g,source,index,&mut dm).map_err(|e|format!("{e}; choices:{:?}",dm.trace))?;let paid=if iron||bank||rift||willow{2}else if quill{1}else{0};if a["mana_paid"]!=paid{return Err(format!("activation mana mismatch expected{paid}:{a}"));}acts.push(a);
 }
 let tokens:Vec<_>=g.battlefield.iter().filter_map(|id|g.object(*id).filter(|o|o.kind==ironsmith::object::ObjectKind::Token).map(|o|json!({"name":o.name.to_string(),"power":g.calculated_power(*id),"toughness":g.calculated_toughness(*id)}))).collect();
 let mut tokens=tokens;tokens.sort_by_key(|t|t["name"].as_str().unwrap().to_owned());
 let actual=json!({"errors":acts.iter().filter_map(|a|a["resolution_error"].as_str()).collect::<Vec<_>>(),"source_plus":g.counter_count(source,CounterType::PlusOnePlusOne),"witness_plus":g.counter_count(witness,CounterType::PlusOnePlusOne),"source_minus":g.counter_count(source,CounterType::MinusOneMinusOne),"source_charge":g.counter_count(source,CounterType::Charge),"resource_minus":if quill{g.counter_count(resource,CounterType::MinusOneMinusOne)}else{0},"resource_time":if rift{g.counter_count(resource,CounterType::Time)}else{0},"source_power":g.calculated_power(source),"source_toughness":g.calculated_toughness(source),"source_tapped":g.is_tapped(source),"witness_power":g.calculated_power(witness),"witness_toughness":g.calculated_toughness(witness),"source_attached":g.object(source).and_then(|o|o.attached_to).map(|id|id==ironsmith::object::AttachmentTarget::Object(witness)).unwrap_or(false),"hand_plains":g.objects_in_deterministic_order().iter().filter(|o|o.name=="Plains"&&o.zone==Zone::Hand&&o.owner==PlayerId(0)).count(),"own_battlefield_plains":g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.name=="Plains"&&o.owner==PlayerId(0))).count(),"own_plains_tapped":g.battlefield.iter().any(|id|g.object(*id).is_some_and(|o|o.name=="Plains"&&o.owner==PlayerId(0))&&g.is_tapped(*id)),"bears_battlefield":g.battlefield.iter().any(|id|g.object(*id).is_some_and(|o|o.name=="Grizzly Bears")),"tokens":tokens});
 let source_pt=if iron{(Some(2),Some(3))}else if quill{(Some(1+3*times as i32),Some(1+3*times as i32))}else if rift{(Some(1+2*times as i32),Some(1))}else if willow{(Some(3),Some(6))}else if scholar{(Some(1),Some(1))}else if bank{(Some(4),Some(4))}else{(None,None)};
 let expected=json!({"errors":[],"source_plus":0,"witness_plus":0,"source_minus":0,"source_charge":if bank{3-times}else if sigil{2-times}else{0},"resource_minus":if quill{2-times}else{0},"resource_time":if rift{3-times}else{0},"source_power":source_pt.0,"source_toughness":source_pt.1,"source_tapped":bank||scholar,"witness_power":if sigil{5+2-times}else{5},"witness_toughness":if sigil{8+2-times}else{8},"source_attached":sigil,"hand_plains":if bank{times}else if iron||(scholar&&variant==0){1}else{0},"own_battlefield_plains":if scholar&&variant==1{1}else{0},"own_plains_tapped":scholar&&variant==1,"bears_battlefield":willow,"tokens":if bank&&times==3{json!([{"name":"Pilot","power":1,"toughness":1},{"name":"Treasure","power":null,"toughness":null}])}else{json!([])}});
 Ok((expected,actual,json!({"source_cast":source_cast,"producers_and_responses":producers,"activations":acts,"choice_trace":dm.trace,"ability_index":index})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="miscellaneous counter-cost canonical audit"]
fn report_counter_cost_misc(){let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
 for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !NAMES.contains(&n)&&!["Common Bond","Plains","Brambleback Brute","Chronozoa","Grizzly Bears","Murder","Twiddle"].contains(&n){continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),p["parse_name"].as_str().unwrap_or(n)),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
 let mut rows=vec![];for name in NAMES{let variants=if name=="Reaping Willow"{1}else{2};for variant in 0..variants{let(status,expected,actual,evidence)=match run(&defs,name,variant){Ok((e,a,f))=>(if e==a{"expected_result_observed"}else if !a["errors"].as_array().unwrap().is_empty(){"execution_failed"}else{"semantic_mismatch"},e,a,f),Err(e)=>("fixture_or_producer_error",Value::Null,json!({"error":e}),Value::Null)};rows.push(json!({"card":name,"scenario":{"variant":variant},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));}}
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Actual full canonical paid sources, ETB/Common Bond counter producers; Quillspike/Rift use real Brambleback/Chronozoa. Reaping uses paid Bears+Murder; Bankbuster uses paid Twiddle between repeated draws. Scholar own/opponent land-count branches, Sigil paidX2 and repeated Equip. Pilot crew ability and subsequent token activations outside scope.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_counter_cost_misc_reproductions.rs")),"runtime_stack_bytes":67108864}});std::fs::write(root.join("reports/runtime-audit/counter-cost-misc-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());}
