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


fn finish(g:&mut GameState,q:&mut TriggerQueue,dm:&mut Choices)->Result<(),String>{let mut state=PriorityLoopState::new(g.players_in_game());for _ in 0..24{advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;if g.stack.is_empty(){return Ok(());}state.reset_for_new_priority_window(g);for _ in 0..g.players_in_game(){apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm).map_err(|e|e.to_string())?;}}Err("finish budget".into())}
fn activate(g:&mut GameState,source:ObjectId,index:usize,dm:&mut Choices)->Result<Value,String>{
 let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index)).ok_or("intended activation unavailable")?;
 let before=g.player(PlayerId(0)).unwrap().mana_pool.total();let mut q=TriggerQueue::new();let mut st=PriorityLoopState::new(g.players_in_game());let mut progress=apply_priority_response_with_dm(g,&mut q,&mut st,&PriorityResponse::PriorityAction(action.clone()),dm).map_err(|e|e.to_string())?;
 for _ in 0..24{if st.pending_activation.is_none()&&!g.stack.is_empty(){break;}let GameProgress::NeedsDecisionCtx(ctx)=progress else{return Err(format!("activation stopped:{progress:?}"));};if matches!(ctx,DecisionContext::Priority(_)){return Err("activation returned priority without stack".into());}progress=apply_decision_context_with_dm(g,&mut q,&mut st,&ctx,dm).map_err(|e|e.to_string())?;}
 let paid=before-g.player(PlayerId(0)).unwrap().mana_pool.total();let error=finish(g,&mut q,dm).err();Ok(json!({"action":format!("{action:?}"),"mana_paid":paid,"resolution_error":error}))
}
const NAMES:[&str;19]=["Fain, the Broker","Ghave, Guru of Spores","Hexavus","Hovel Hurler","Ion Storm","Korozda Gorgon","Loch Mare","Meldweb Strider","Moonlit Lamenter","Novijen Sages","O'aka, Traveling Merchant","Ray Fillet, Man Ray","Sage of Fables","Shapers of Nature","Soul Diviner","Spike Rogue","Sunstar Chaplain","The Duke, Rebel Sentry","Zameck Guildmage"];
fn find_cost_index(def:&CardDefinition,ordinal:usize)->Result<usize,String>{def.abilities.iter().enumerate().filter(|(_,a)|match &a.kind{ironsmith::ability::AbilityKind::Activated(a)=>a.mana_cost.costs().iter().any(|c|c.effect_ref().is_some_and(|e|e.downcast_ref::<ironsmith::effects::RemoveAnyCountersAmongEffect>().is_some())),_=>false}).nth(ordinal).map(|(i,_)|i).ok_or("intended counter cost index absent".into())}
fn run(defs:&HashMap<String,CardDefinition>,name:&str,variant:usize)->Result<(Value,Value,Value),String>{
 use ironsmith::CounterType;use ironsmith::static_abilities::StaticAbilityId;
 let mut g=setup(3,0);let wd=CardDefinitionBuilder::new(CardId::new(),"Counter sibling witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(5,8)).build();let witness=g.create_object_from_definition(&wd,PlayerId(0),Zone::Battlefield);let enemy=g.create_object_from_definition(&wd,PlayerId(1),Zone::Battlefield);
 for _ in 0..3{g.create_object_from_definition(&defs["Plains"],PlayerId(0),Zone::Library);}
 let mut dm=Choices{targets:vec![],counter_objects:vec![],trace:vec![]};let source_cast=cast(&mut g,&defs[name],0,&mut dm)?;
 let source=*g.battlefield.iter().find(|id|g.object(**id).is_some_and(|o|o.name==name)).ok_or("source absent after actual cast")?;
 let source_cost=match name{"Ghave, Guru of Spores"|"Hovel Hurler"|"Korozda Gorgon"|"Meldweb Strider"=>5,"Hexavus"|"Novijen Sages"=>6,"Ray Fillet, Man Ray"=>4,"Loch Mare"|"O'aka, Traveling Merchant"|"Soul Diviner"|"Sunstar Chaplain"|"Zameck Guildmage"=>2,"The Duke, Rebel Sentry"=>1,_=>3};
 if source_cast["mana_paid"]!=source_cost||!source_cast["resolution_error"].is_null(){return Err(format!("source cast failed payment or resolution:{source_cast}"));}
 let inherently=match name{"Ghave, Guru of Spores"=>5,"Hexavus"=>6,"Novijen Sages"=>4,"Spike Rogue"=>2,"The Duke, Rebel Sentry"=>1,_=>0};
 let minus=match name{"Hovel Hurler"=>2,"Loch Mare"=>3,"Moonlit Lamenter"=>1,_=>0};
 if g.counter_count(source,CounterType::PlusOnePlusOne)!=inherently||g.counter_count(source,CounterType::MinusOneMinusOne)!=minus{return Err("entry counter oracle mismatch".into());}
 if name=="Meldweb Strider"&&g.counter_count(source,CounterType::Oil)!=1{return Err("Strider ETB oil absent".into());}
 let force_source=["Hovel Hurler","Loch Mare","Meldweb Strider","Moonlit Lamenter","The Duke, Rebel Sentry"].contains(&name)||(["Hexavus","Spike Rogue"].contains(&name)&&variant==0);
 let resource=if force_source{source}else if name=="Ion Storm"||variant==1{witness}else{source};
 let needs_positive=minus==0&&name!="Meldweb Strider";
 let mut producer=Value::Null;
 if name=="Shapers of Nature"{dm.targets=vec![Target::Object(resource)];let index=defs[name].abilities.iter().enumerate().find_map(|(i,a)|matches!(a.kind,ironsmith::ability::AbilityKind::Activated(_)).then_some(i)).unwrap();producer=activate(&mut g,source,index,&mut dm)?;if producer["mana_paid"]!=4||g.counter_count(resource,CounterType::PlusOnePlusOne)!=1{return Err("Shapers real counter production failed".into());}}
 else if needs_positive&&!(resource==source&&inherently>0){dm.targets=vec![Target::Object(resource),Target::Object(resource)];producer=cast(&mut g,&defs["Common Bond"],0,&mut dm)?;if producer["mana_paid"]!=3||g.counter_count(resource,CounterType::PlusOnePlusOne)!=2{return Err("Common Bond counter production failed".into());}}
 if ["Fain, the Broker","O'aka, Traveling Merchant","Soul Diviner","The Duke, Rebel Sentry"].contains(&name){g.next_turn();ironsmith::turn::execute_untap_step(&mut g);while g.turn.active_player!=PlayerId(0){g.next_turn();ironsmith::turn::execute_untap_step(&mut g);}g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;g.turn.priority_player=Some(PlayerId(0));for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless]{g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color,12);}}
 let before_source_plus=g.counter_count(source,CounterType::PlusOnePlusOne);let before_witness_plus=g.counter_count(witness,CounterType::PlusOnePlusOne);
 let before_token_ids:Vec<_>=g.battlefield.iter().copied().filter(|id|g.object(*id).is_some_and(|o|o.kind==ironsmith::object::ObjectKind::Token)).collect();
 let before_tokens=g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.kind==ironsmith::object::ObjectKind::Token)).count();
 let ordinal=if ["Hexavus","Spike Rogue","Loch Mare"].contains(&name){variant}else{0};let index=if ["Hexavus","Spike Rogue"].contains(&name){defs[name].abilities.iter().enumerate().filter(|(_,a)|matches!(a.kind,ironsmith::ability::AbilityKind::Activated(_))).nth(ordinal).map(|(i,_)|i).ok_or("explicit activated ordinal absent")?}else{find_cost_index(&defs[name],ordinal)?};
 dm.counter_objects=vec![resource];dm.targets=match name{"Ion Storm"=>vec![Target::Player(PlayerId(1))],"Hexavus"|"Spike Rogue" if variant==0=>vec![Target::Object(witness)],"Hovel Hurler"|"The Duke, Rebel Sentry"=>vec![Target::Object(witness)],"Korozda Gorgon"|"Sunstar Chaplain"=>vec![Target::Object(enemy)],"Loch Mare" if variant==1=>vec![Target::Object(enemy)],_=>vec![]};
 let activation=activate(&mut g,source,index,&mut dm)?;
 let paid=match name{"Fain, the Broker"|"Meldweb Strider"|"O'aka, Traveling Merchant"|"Soul Diviner"|"The Duke, Rebel Sentry"=>0,"Ghave, Guru of Spores"|"Hexavus"|"Novijen Sages"=>1,"Shapers of Nature"=>3,"Loch Mare" if variant==1=>3,_=>2};if activation["mana_paid"]!=paid{return Err(format!("activation payment wrong expected{paid}:{activation}"));}
 let draw=["Moonlit Lamenter","Novijen Sages","O'aka, Traveling Merchant","Ray Fillet, Man Ray","Sage of Fables","Shapers of Nature","Soul Diviner","Zameck Guildmage"].contains(&name)||(name=="Loch Mare"&&variant==0);
 let mut sp=before_source_plus;let mut wp=before_witness_plus;let removed=if name=="Novijen Sages"{2}else{1};if needs_positive{if resource==source{sp-=removed}else{wp-=removed}}
 if name=="Spike Rogue"{if variant==0{wp+=1}else{sp+=1}}if name=="Hexavus"&&variant==1{sp+=1}if name=="The Duke, Rebel Sentry"{wp+=1}
 let flying=name=="Hovel Hurler"||(name=="Hexavus"&&variant==0);let expected_minus=minus-if minus>0{if name=="Loch Mare"&&variant==1{2}else{1}}else{0};
 let target_tapped=name=="Sunstar Chaplain"||(name=="Loch Mare"&&variant==1);
 let created_tokens:Vec<_>=g.battlefield.iter().filter(|id|!before_token_ids.contains(id)).filter_map(|id|g.object(*id).filter(|o|o.kind==ironsmith::object::ObjectKind::Token).map(|o|json!({"name":o.name.to_string(),"controller":g.controller_of_id(*id).map(|p|p.index()),"power":g.calculated_power(*id),"toughness":g.calculated_toughness(*id),"types":g.calculated_characteristics(*id).map(|c|c.card_types.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>())}))).collect();
 let expected_tokens=match name{"Fain, the Broker"=>json!([{"name":"Treasure","controller":0,"power":null,"toughness":null,"types":["Artifact"]}]),"Ghave, Guru of Spores"=>json!([{"name":"Saproling","controller":0,"power":1,"toughness":1,"types":["Creature"]}]),_=>json!([])};
 let actual=json!({"created_tokens":created_tokens,"error":activation["resolution_error"],"source_plus":g.counter_count(source,CounterType::PlusOnePlusOne),"witness_plus":g.counter_count(witness,CounterType::PlusOnePlusOne),"source_minus":g.counter_count(source,CounterType::MinusOneMinusOne),"source_oil":g.counter_count(source,CounterType::Oil),"source_tapped":g.is_tapped(source),"source_creature":g.calculated_characteristics(source).is_some_and(|c|c.card_types.contains(&CardType::Creature)),"bob_life":g.player(PlayerId(1)).unwrap().life,"enemy_power":g.calculated_power(enemy),"enemy_toughness":g.calculated_toughness(enemy),"enemy_tapped":g.is_tapped(enemy),"enemy_stun":g.counter_count(enemy,CounterType::Stun),"witness_power":g.calculated_power(witness),"witness_toughness":g.calculated_toughness(witness),"witness_flying":g.object_has_static_ability_id(witness,StaticAbilityId::Flying),"witness_flying_counter":g.counter_count(witness,CounterType::Flying),"witness_hexproof":g.object_has_static_ability_id(witness,StaticAbilityId::Hexproof),"hand_plains":g.objects_in_deterministic_order().iter().filter(|o|o.name=="Plains"&&o.zone==Zone::Hand).count(),"new_tokens":g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.kind==ironsmith::object::ObjectKind::Token)).count()-before_tokens});
 let expected=json!({"created_tokens":expected_tokens,"error":null,"source_plus":sp,"witness_plus":wp,"source_minus":expected_minus,"source_oil":0,"source_tapped":(["Fain, the Broker","O'aka, Traveling Merchant","Soul Diviner","The Duke, Rebel Sentry"].contains(&name)),"source_creature":name!="Ion Storm","bob_life":if name=="Ion Storm"{18}else{20},"enemy_power":if name=="Korozda Gorgon"{4}else{5},"enemy_toughness":if name=="Korozda Gorgon"{7}else{8},"enemy_tapped":target_tapped,"enemy_stun":if name=="Loch Mare"&&variant==1{1}else{0},"witness_power":5+wp+if name=="Hovel Hurler"{1}else{0},"witness_toughness":8+wp,"witness_flying":flying,"witness_flying_counter":if name=="Hexavus"&&variant==0{1}else{0},"witness_hexproof":name=="The Duke, Rebel Sentry","hand_plains":usize::from(draw),"new_tokens":if ["Fain, the Broker","Ghave, Guru of Spores"].contains(&name){1}else{0}});
 Ok((expected,actual,json!({"source_cast":source_cast,"counter_producer":producer,"activation":activation,"choice_trace":dm.trace,"resource":format!("{resource:?}"),"ability_index":index})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="counter-removal sibling full-payment audit"]
fn report_counter_choice_siblings(){let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
 for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !NAMES.contains(&n)&&!["Common Bond","Plains"].contains(&n){continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
 let mut rows=vec![];for name in NAMES{let variants=if ["Hovel Hurler","Meldweb Strider","Moonlit Lamenter","The Duke, Rebel Sentry","Ion Storm"].contains(&name){1}else{2};for variant in 0..variants{let(status,expected,actual,evidence)=match run(&defs,name,variant){Ok((e,a,f))=>(if e==a{"expected_result_observed"}else if !a["error"].is_null(){"execution_failed"}else{"semantic_mismatch"},e,a,f),Err(e)=>("fixture_or_producer_error",Value::Null,json!({"error":e}),Value::Null)};rows.push(json!({"card":name,"scenario":{"variant":variant},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));}}
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Actual paid canonical sources, inherent ETB counters or paid Common Bond or Shapers counter producer; explicit full counter payment. Exact mana/counters and immediate effect fields. Tap creatures wait actual turn cycle and untap. Later expiry/combat/token activations outside scope.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_counter_choice_sibling_reproductions.rs")),"runtime_stack_bytes":67108864}});std::fs::write(root.join("reports/runtime-audit/counter-choice-sibling-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());}
