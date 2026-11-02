//! Candidate exact Way source tests; materialize overlay through the frontend
//! after integrating iteration/action readers before registering this target.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone,StackEntry};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use std::collections::VecDeque;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn way_definitions()->[CardDefinition;2] {
    let fixture=read_source_fixture();
    let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]=="msem-440").unwrap();let source=&row["representative_cards"][0];
    let text=source["executable_text"].as_str().expect("parent must materialize exact Way source after reader integration");
    let (result,loss)=with_vocabulary(||ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(source_builder(source),text,false))));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
    let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn inscription(name:&str,life:u32)->CardDefinition {
    let text=format!("Mana cost: {{U}}\nType: Sorcery — Inscription\nYou gain {life} life.");
    let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),name).with_custom_keyword("Inscribe");
    with_vocabulary(||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,&text,false)).unwrap().1
}
struct Choices{hosts:Vec<ObjectId>,next:usize,accept:VecDeque<bool>}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.accept.pop_front().unwrap_or(true)}
    fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{
        let host=self.hosts[self.next.min(self.hosts.len()-1)];
        if ctx.candidates.iter().any(|candidate|candidate.id==host&&candidate.legal){self.next+=1;vec![host]}else{SelectFirstDecisionMaker.decide_objects(game,ctx)}
    }
}
fn attack(game:&mut GameState,host:ObjectId,dm:&mut Choices){
    game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(host,ironsmith::events::AttackEventTarget::Player(B)),Default::default()));
    let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();
}
fn resolve_attacks(game:&mut GameState,count:usize,dm:&mut Choices){
    assert_eq!(game.stack.len(),count);
    for _ in 0..count {ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();}
    assert!(game.stack.is_empty());
}
#[test]
fn exact_way_source_unions_both_zones_reexiles_existing_cards_and_captures_each_card(){
    for definition in way_definitions(){for separate_hosts in [false,true]{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
        let host_def=ironsmith_compiler_runtime::compile_to_runtime_definition("Way host","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
        let hosts=[game.create_object_from_definition(&host_def,A,Zone::Battlefield),game.create_object_from_definition(&host_def,A,Zone::Battlefield)];
        let first=game.create_object_from_definition(&inscription("Graveyard inscription",1),A,Zone::Graveyard);let first_stable=game.object(first).unwrap().stable_id;
        let second=game.create_object_from_definition(&inscription("Already exiled inscription",2),A,Zone::Exile);let second_stable=game.object(second).unwrap().stable_id;
        let foreign=game.create_object_from_definition(&inscription("Foreign inscription",4),B,Zone::Exile);
        let plain=ironsmith_compiler_runtime::compile_to_runtime_definition("Noninscription","Type: Sorcery\nYou gain 8 life.",false).unwrap();let unrelated=game.create_object_from_definition(&plain,A,Zone::Exile);
        let mut dm=Choices{hosts:if separate_hosts{hosts.to_vec()}else{vec![hosts[0],hosts[0]]},next:0,accept:vec![true,true].into()};
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);game.push_to_stack(StackEntry::new(source,A));ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let first=game.find_object_by_stable_id(first_stable).unwrap();let second_arrival=game.find_object_by_stable_id(second_stable).unwrap();
        assert_eq!(game.object(first).unwrap().zone,Zone::Exile);assert_ne!(second_arrival,second,"exile-to-exile changes incarnation and retires old links");
        assert_eq!(game.source_collection_objects(first,"inscribed host"),vec![hosts[0]]);assert_eq!(game.source_collection_objects(second_arrival,"inscribed host"),vec![if separate_hosts{hosts[1]}else{hosts[0]}]);
        assert!(game.object(foreign).is_some());assert!(game.object(unrelated).is_some());assert_eq!(dm.next,2,"one distinct optional recipient choice for each qualifying owned card");
        if separate_hosts {attack(&mut game,hosts[0],&mut dm);resolve_attacks(&mut game,1,&mut dm);attack(&mut game,hosts[1],&mut dm);resolve_attacks(&mut game,1,&mut dm);}else{attack(&mut game,hosts[0],&mut dm);resolve_attacks(&mut game,2,&mut dm);}
        assert_eq!(game.player(A).unwrap().life,23,"each trigger copies its own card, not the resolving Way or another iteration");
        assert_eq!(game.object(first).unwrap().zone,Zone::Exile);assert_eq!(game.object(second_arrival).unwrap().zone,Zone::Exile);
    }}
}
#[test]
fn optional_mass_link_can_decline_one_card_without_declining_the_other(){
    for definition in way_definitions(){
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Optional host","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let host=game.create_object_from_definition(&body,A,Zone::Battlefield);
        game.create_object_from_definition(&inscription("Accepted inscription",1),A,Zone::Graveyard);game.create_object_from_definition(&inscription("Declined inscription",2),A,Zone::Graveyard);
        let mut dm=Choices{hosts:vec![host],next:0,accept:vec![true,false].into()};let source=game.create_object_from_definition(&definition,A,Zone::Stack);game.push_to_stack(StackEntry::new(source,A));ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(dm.next,1);attack(&mut game,host,&mut dm);resolve_attacks(&mut game,1,&mut dm);assert!([21,22].contains(&game.player(A).unwrap().life),"declining one iteration leaves exactly the accepted card's independent link");
    }
}

fn read_source_fixture()->serde_json::Value{
    let path=std::env::var("IRONSMITH_INSCRIBE_SOURCE_FIXTURE").unwrap_or_else(|_|format!("{}/../../fixtures/custom_mechanics/msem-inscribe-all-sources-candidate.json",env!("CARGO_MANIFEST_DIR")));
    serde_json::from_str(&std::fs::read_to_string(path).expect("parent must materialize full-source Inscribe candidate fixture first")).unwrap()
}
fn with_vocabulary<T>(body:impl FnOnce()->T)->T{
    use ironsmith_core::{SubtypeFamily,custom_subtypes::CustomSubtypeDefinition};
    let vocabulary=[CustomSubtypeDefinition{singular:"Inscription".into(),plural:"Inscriptions".into(),family:SubtypeFamily::Spell},CustomSubtypeDefinition{singular:"Inscription".into(),plural:"Inscriptions".into(),family:SubtypeFamily::Artifact}];
    ironsmith_compiler::with_custom_subtypes(&vocabulary,body).unwrap()
}
fn source_builder(source:&serde_json::Value)->ironsmith_compiler::CardDefinitionBuilder{
    let mut builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),source["name"].as_str().unwrap());
    if let Some(identities)=source.get("custom_keyword_identities").or_else(||source.get("executable_faces").and_then(|faces|faces.get(0)).and_then(|face|face.get("custom_keyword_identities"))).and_then(|value|value.as_array()){
        for identity in identities{builder=builder.with_custom_keyword(identity.as_str().unwrap());}
    }
    builder
}

#[path="custom_final_next_wave_support.rs"] mod optional_foreach_source_routes;
#[test]
fn generic_bounded_foreach_asks_independently_and_keeps_the_whole_optional_body(){
 let input=serde_json::json!({"name":"Independent optional body","executable_text":"Type: Sorcery\nFor each creature you control, you may do the following: [You gain 1 life; You gain 2 life]. You gain 4 life."});
 struct IndependentChoices{answers:VecDeque<bool>,prompts:usize}
 impl DecisionMaker for IndependentChoices{
  fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool{assert_eq!(ctx.player,A);self.prompts+=1;self.answers.pop_front().expect("one decision per own subject")}
 }
 for definition in optional_foreach_source_routes::definitions(&input).unwrap(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary subject","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
  game.create_object_from_definition(&body,A,Zone::Battlefield);game.create_object_from_definition(&body,A,Zone::Battlefield);game.create_object_from_definition(&body,B,Zone::Battlefield);
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);let mut dm=IndependentChoices{answers:vec![false,true].into(),prompts:0};
  ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned())),&mut ironsmith::effects::EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(dm.prompts,2);assert!(dm.answers.is_empty());assert_eq!(game.player(A).unwrap().life,27,"declined subject does neither body statement; accepted subject does both; tail executes once");assert_eq!(game.player(B).unwrap().life,20);
 }
}

#[test]
fn malformed_optional_foreach_header_cannot_be_losslessly_admitted_by_another_owner(){
 for body in ["For each creature you control, you may do following: [You gain 1 life]","For each creature you control, you might do the following: [You gain 1 life]"]{
  let input=serde_json::json!({"name":"Invalid optional iteration","executable_text":format!("Type: Sorcery\n{body}")});
  assert!(optional_foreach_source_routes::definitions(&input).is_err(),"{body}");
 }
}
