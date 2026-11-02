//! Source attack/payment programs emit named receipts only for completed payments.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::object::CounterType;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Choices(bool);
impl DecisionMaker for Choices{fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}}
fn definitions()->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-ability-links.json")).unwrap();let row=fixture["mechanics"].as_array().unwrap().iter().find(|m|m["id"]=="msem-688").unwrap();let card=&row["representative_cards"][0];
 let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),card["name"].as_str().unwrap()).with_custom_keyword("assault");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,card["executable_text"].as_str().unwrap(),false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn flush(game:&mut GameState,dm:&mut Choices){let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();}
#[test]
fn source_attack_payment_and_intervening_if_control_the_named_receipt(){
 for definition in definitions(){for accept in [false,true]{for (initial,response) in [(0,false),(1,false),(0,true)]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let mut dm=Choices(accept);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  if initial>0{game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,initial);}
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  game.remove_summoning_sickness(source);game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();
  let mut combat=ironsmith::combat_state::CombatState::default();let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::apply_attacker_declarations(&mut game,&mut combat,&mut queue,&[ironsmith::decision::AttackerDeclaration{creature:source,target:ironsmith::combat_state::AttackTarget::Player(B)}]).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
  assert_eq!(game.stack.len(),usize::from(initial==0));if response{game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,1);}
  if !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
  let paid=accept&&initial==0&&!response;assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),if initial>0||response||paid{1}else{0});assert_eq!(game.player(A).unwrap().mana_pool.total(),if paid{0}else{2});
  flush(&mut game,&mut dm);assert_eq!(game.stack.len(),usize::from(paid));if paid{ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  assert_eq!(game.current_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Flying),paid);
 }}}
}
