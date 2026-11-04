//! Generic external triggered classes use choices of occurrences and normal trigger announcements.
use ironsmith::{CardDefinition,Effect,GameState,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(name:&str,text:&str)->[CardDefinition;2]{let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);let indirect=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&indirect));
 let expected=text.lines().filter(|line|line.trim_start().starts_with("Externally triggered ability named ")).count();
 if expected>0 { for definition in [&direct,&indirect] {
  let actual=definition.abilities.iter().filter(|ability|matches!(&ability.kind,ironsmith::AbilityKind::Triggered(triggered)
   if triggered.trigger.downcast_ref::<ironsmith::triggers::special::CustomTrigger>().is_some_and(|marker|marker.id.starts_with("externally-triggered:")))).count();
  assert_eq!(actual,expected,"every external header must own a retained triggered program before invocation");
  assert!(definition.spell_effect.is_none(),"an external ability body cannot escape as a spell program");
 }}
 [direct,indirect]}
fn invoke(game:&mut GameState,definition:&CardDefinition,dm:&mut dyn DecisionMaker){let source=game.create_object_from_definition(definition,A,Zone::Stack);let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new(source,A,dm)).unwrap();}
fn settle(game:&mut GameState){let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();}}
#[test]
fn choices_filter_class_current_zone_and_controller_and_never_execute_effects_inline(){
 for action in definitions("Invoker","Type: Sorcery\nTrigger up to one resonance triggered ability of permanents you control."){for donor in definitions("Donor","Type: Artifact\nExternally triggered ability named resonance — You gain 2 life."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&donor,B,Zone::Battlefield);game.create_object_from_definition(&donor,A,Zone::Graveyard);let own=game.create_object_from_definition(&donor,A,Zone::Battlefield);
  let other=definitions("Other class","Type: Artifact\nExternally triggered ability named echoing — You gain 9 life.")[0].clone();game.create_object_from_definition(&other,A,Zone::Battlefield);
  invoke(&mut game,&action,&mut SelectFirstDecisionMaker);assert_eq!(game.player(A).unwrap().life,20);assert!(game.stack.is_empty(),"invoked triggers wait until enclosing resolution ends");
  let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.stack.len(),1);assert_eq!(game.stack[0].object_id,own);assert_eq!(game.stack[0].controller,A);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().life,22);assert_eq!(game.player(B).unwrap().life,20);
 }}
}
struct Decline;
impl DecisionMaker for Decline{fn decide_options(&mut self,_:&GameState,_:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{vec![]}}
#[test]
fn up_to_can_decline_and_separate_invocations_can_select_the_same_ability(){
 for donor in definitions("Repeated class","Type: Artifact\nExternally triggered ability named resonance — You gain 2 life."){for action in definitions("Invoker","Type: Sorcery\nTrigger up to one resonance triggered ability of permanents you control."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&donor,A,Zone::Battlefield);invoke(&mut game,&action,&mut Decline);settle(&mut game);assert_eq!(game.player(A).unwrap().life,20);invoke(&mut game,&action,&mut SelectFirstDecisionMaker);invoke(&mut game,&action,&mut SelectFirstDecisionMaker);settle(&mut game);assert_eq!(game.player(A).unwrap().life,24);
 }}
}
#[test]
fn distinct_occurrences_on_one_permanent_are_both_selectable_and_normal_queue_chooses_targets(){
 for donor in definitions("Two class occurrences","Type: Artifact\nExternally triggered ability named resonance — You gain 2 life.\nExternally triggered ability named resonance — Target creature gets +1/+1 until end of turn."){for action in definitions("Invoker","Type: Sorcery\nTrigger up to two resonance triggered abilities of permanents you control."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&donor,A,Zone::Battlefield);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Only creature","Type: Creature\nPower/Toughness: 1/1",false).unwrap();let target=game.create_object_from_definition(&body,A,Zone::Battlefield);invoke(&mut game,&action,&mut SelectFirstDecisionMaker);settle(&mut game);assert_eq!(game.player(A).unwrap().life,22);assert_eq!(game.current_power(target),Some(2));
 }}
}

struct ChooseAllAndDouble { accept: bool, prompts: usize }
impl DecisionMaker for ChooseAllAndDouble {
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{(0..ctx.max.min(ctx.options.len())).collect()}
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.prompts+=1;self.accept}
}
#[test]
fn action_scoped_extra_choice_is_distinct_and_preserves_the_base_budget_elsewhere(){
 for modifier in definitions("Extra class choice","Type: Artifact\nAs you assemble, you may choose another resonance ability to trigger.") {
  let donor=definitions("Two occurrences","Type: Artifact\nExternally triggered ability named resonance — You gain 2 life.\nExternally triggered ability named resonance — You gain 3 life.")[0].clone();
  let matching=definitions("Named action","Type: Sorcery\nTrigger up to one resonance triggered ability of permanents you control as the assemble action.")[0].clone();
  let other=definitions("Other action","Type: Sorcery\nTrigger up to one resonance triggered ability of permanents you control as the research action.")[0].clone();
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&modifier,B,Zone::Battlefield);game.create_object_from_definition(&donor,A,Zone::Battlefield);
  invoke(&mut game,&matching,&mut SelectFirstDecisionMaker);settle(&mut game);assert_eq!(game.player(A).unwrap().life,22,"opponent's static instruction does not apply");
  game.create_object_from_definition(&modifier,A,Zone::Battlefield);invoke(&mut game,&other,&mut SelectFirstDecisionMaker);settle(&mut game);assert_eq!(game.player(A).unwrap().life,24,"other action retains one choice");
  invoke(&mut game,&matching,&mut SelectFirstDecisionMaker);settle(&mut game);assert_eq!(game.player(A).unwrap().life,29,"both distinct occurrences of one permanent may be selected");
 }
}
#[test]
fn optional_class_doubling_consumes_its_once_per_turn_receipt_only_when_accepted(){
 for modifier in definitions("Optional multiplier","Type: Artifact\nOnce each turn, if you would trigger a resonance ability of a permanent you control, you may have that ability trigger twice instead.") {
  let donor=definitions("Single occurrence","Type: Artifact\nExternally triggered ability named resonance — You gain 2 life.")[0].clone();let action=definitions("Invoke","Type: Sorcery\nTrigger up to one resonance triggered ability of permanents you control.")[0].clone();
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&modifier,A,Zone::Battlefield);game.create_object_from_definition(&donor,A,Zone::Battlefield);
  let mut dm=ChooseAllAndDouble{accept:false,prompts:0};invoke(&mut game,&action,&mut dm);settle(&mut game);assert_eq!(game.player(A).unwrap().life,22);assert_eq!(dm.prompts,1);
  dm.accept=true;invoke(&mut game,&action,&mut dm);settle(&mut game);assert_eq!(game.player(A).unwrap().life,26);assert_eq!(dm.prompts,2);
  invoke(&mut game,&action,&mut dm);settle(&mut game);assert_eq!(game.player(A).unwrap().life,28);assert_eq!(dm.prompts,2,"accepted receipt prevents another offer this turn");
 }
}
