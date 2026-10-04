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
 fn decide_number(&mut self,_:&GameState,c:&ironsmith::decisions::context::NumberContext)->u32{let selected=self.amount.max(c.min).min(c.max);self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"selected":selected}));selected}
 fn decide_targets(&mut self,_:&GameState,c:&TargetsContext)->Vec<Target>{self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));self.targets.clone()}
 fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{let mode_word=if self.mode==0{"charge counters"}else{"+1/+1 counters"};let s=if c.options.iter().any(|o|o.description.contains("charge counters"))&&c.options.iter().any(|o|o.description.contains("+1/+1 counters")){c.options.iter().find(|o|o.legal&&o.description.contains(mode_word)).map(|o|vec![o.index]).unwrap()}else{ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,c)};self.trace.push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));s}
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


fn finish(g:&mut GameState,q:&mut TriggerQueue,dm:&mut Choices)->Result<(),String>{let mut state=PriorityLoopState::new(g.players_in_game());for _ in 0..24{advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;if g.stack.is_empty(){return Ok(());}state.reset_for_new_priority_window(g);for _ in 0..g.players_in_game(){apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm).map_err(|e|e.to_string())?;}}Err("finish budget".into())}
fn activate(g:&mut GameState,source:ObjectId,index:usize,dm:&mut Choices)->Result<Value,String>{
 let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index)).ok_or("intended activation unavailable")?;
 let before=g.player(PlayerId(0)).unwrap().mana_pool.total();let mut q=TriggerQueue::new();let mut st=PriorityLoopState::new(g.players_in_game());let mut progress=apply_priority_response_with_dm(g,&mut q,&mut st,&PriorityResponse::PriorityAction(action.clone()),dm).map_err(|e|e.to_string())?;
 for _ in 0..24{if st.pending_activation.is_none()&&!g.stack.is_empty(){break;}let GameProgress::NeedsDecisionCtx(ctx)=progress else{return Err(format!("activation stopped:{progress:?}"));};if matches!(ctx,DecisionContext::Priority(_)){return Err("activation returned priority without stack".into());}progress=apply_decision_context_with_dm(g,&mut q,&mut st,&ctx,dm).map_err(|e|e.to_string())?;}
 let paid=before-g.player(PlayerId(0)).unwrap().mana_pool.total();let error=finish(g,&mut q,dm).err();Ok(json!({"action":format!("{action:?}"),"mana_paid":paid,"resolution_error":error}))
}
fn find_cost_index(def:&CardDefinition,ordinal:usize)->Result<usize,String>{def.abilities.iter().enumerate().filter(|(_,a)|match &a.kind{ironsmith::ability::AbilityKind::Activated(a)=>a.mana_cost.costs().iter().any(|c|c.effect_ref().is_some_and(|e|e.downcast_ref::<ironsmith::effects::RemoveAnyCountersAmongEffect>().is_some())),_=>false}).nth(ordinal).map(|(i,_)|i).ok_or("intended counter cost index absent".into())}
const NAMES:[&str;6]=["Arcee, Sharpshooter","Arcee, Sharpshooter // Arcee, Acrobatic Coupe","Jetfire, Ingenious Scientist","Jetfire, Ingenious Scientist // Jetfire, Air Guardian","Moxite Refinery","Retribution of the Ancients"];
fn run(defs:&HashMap<String,CardDefinition>,name:&str,amount:u32,variant:usize)->Result<(Value,Value,Value),String>{
 use ironsmith::CounterType;
 let mut g=setup(3,0);let wd=CardDefinitionBuilder::new(CardId::new(),"Variable counter resource").card_types(vec![CardType::Artifact,CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(5,8)).build();let witness=g.create_object_from_definition(&wd,PlayerId(0),Zone::Battlefield);let second=g.create_object_from_definition(&wd,PlayerId(0),Zone::Battlefield);let enemy=g.create_object_from_definition(&wd,PlayerId(1),Zone::Battlefield);
 let mut dm=Choices{amount,mode:variant,targets:vec![],counter_objects:vec![],trace:vec![]};let source_cast=cast(&mut g,&defs[name],0,&mut dm)?;
 let source=*g.battlefield.iter().find(|id|g.object(**id).is_some_and(|o|o.name==defs[name].name())).ok_or("source absent after actual cast")?;
 let arcee=name.starts_with("Arcee");let jetfire=name.starts_with("Jetfire");let moxite=name=="Moxite Refinery";let retribution=name=="Retribution of the Ancients";
 let source_cost=if arcee{3}else if jetfire{5}else if moxite{2}else{1};if source_cast["mana_paid"]!=source_cost||!source_cast["resolution_error"].is_null(){return Err(format!("source normal cast failed:{source_cast}"));}
 let first=if arcee{source}else{witness};let other=if !arcee&&!moxite&&variant==1{second}else{first};dm.targets=vec![Target::Object(first),Target::Object(other)];let producer=cast(&mut g,&defs["Common Bond"],0,&mut dm)?;
 if producer["mana_paid"]!=3||g.counter_count(first,CounterType::PlusOnePlusOne)!=if first==other{2}else{1}||!producer["resolution_error"].is_null(){return Err("actual Common Bond did not produce exact intended counters".into());}
 dm.counter_objects=if first==other{vec![first]}else{vec![first,other]};dm.targets=if jetfire{vec![Target::Player(PlayerId(1))]}else if moxite&&variant==0{vec![Target::Object(source)]}else{vec![Target::Object(enemy)]};
 let index=find_cost_index(&defs[name],0)?;let activation=activate(&mut g,source,index,&mut dm)?;
 let paid=if arcee||retribution{1}else if moxite{2}else{0};if activation["mana_paid"]!=paid{return Err(format!("activation mana mismatch expected{paid}:{activation}"));}
 let remaining=2-amount;let (first_left,second_left)=if first==other{(remaining,0)}else{(1u32.saturating_sub(amount),if amount>=2{0}else{1})};
 let actual=json!({"error":activation["resolution_error"],"first_plus":g.counter_count(first,CounterType::PlusOnePlusOne),"second_plus":if first==other{0}else{g.counter_count(other,CounterType::PlusOnePlusOne)},"source_tapped":g.is_tapped(source),"source_charge":g.counter_count(source,CounterType::Charge),"enemy_damage":g.damage_on(enemy),"enemy_plus":g.counter_count(enemy,CounterType::PlusOnePlusOne),"enemy_power":g.calculated_power(enemy),"enemy_toughness":g.calculated_toughness(enemy),"bob_mana":g.player(PlayerId(1)).unwrap().mana_pool.total()});
 let expected=json!({"error":null,"first_plus":first_left,"second_plus":second_left,"source_tapped":moxite,"source_charge":if moxite&&variant==0{amount}else{0},"enemy_damage":if arcee{amount}else{0},"enemy_plus":if moxite&&variant==1{amount}else{0},"enemy_power":if retribution{5-amount}else if moxite&&variant==1{5+amount}else{5},"enemy_toughness":if retribution{8-amount}else if moxite&&variant==1{8+amount}else{8},"bob_mana":if jetfire{amount}else{0}});
 Ok((expected,actual,json!({"source_cast":source_cast,"counter_producer":producer,"activation":activation,"choice_trace":dm.trace,"ability_index":index,"linked_face_metadata":format!("{:?}",defs[name].card.linked_face_layout),"conversion_exercised":false})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="variable counter costs canonical audit"]
fn report_variable_counter_costs(){let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
 for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !NAMES.contains(&n)&&n!="Common Bond"{continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),p["parse_name"].as_str().unwrap_or(n)),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
 let mut rows=vec![];for name in NAMES{let min=if name.starts_with("Arcee")||name.starts_with("Jetfire"){1}else{0};let variants=if name.starts_with("Arcee"){1}else{2};for amount in min..=2{for variant in 0..variants{let(status,expected,actual,evidence)=match run(&defs,name,amount,variant){Ok((e,a,f))=>(if e==a{"expected_result_observed"}else if !a["error"].is_null(){"execution_failed"}else{"semantic_mismatch"},e,a,f),Err(e)=>("fixture_or_producer_error",Value::Null,json!({"error":e}),Value::Null)};rows.push(json!({"card":name,"scenario":{"amount":amount,"variant":variant},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));}}}
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Actual paid full canonical sources and Common Bond; explicit variable counter amount and payment. X0/1/2 or one-or-more1/2; Moxite both modes; Jetfire/Retribution one or split resource objects. Arcee/Jetfire conversion excluded because canonical single-face payloads lack linked-face metadata. Jetfire restriction consumption outside scope; mana amount checked.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_variable_counter_cost_reproductions.rs")),"runtime_stack_bytes":67108864}});std::fs::write(root.join("reports/runtime-audit/variable-counter-cost-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());}
