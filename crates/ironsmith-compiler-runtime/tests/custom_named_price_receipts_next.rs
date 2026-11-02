//! Price decorators publish one semantic completion after the complete native cost.
use ironsmith::{Effect,GameState,PlayerId,Zone};
use ironsmith::costs::{CostContext,CostPayer};
use ironsmith::decision::SelectFirstDecisionMaker;
const A:PlayerId=PlayerId(0);
fn body(mana:&str)->ironsmith::CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Resource",&format!("Mana cost: {mana}\nType: Creature\nPower/Toughness: 2/2"),false).unwrap()}
fn pairs(text:&str)->[ironsmith::CardDefinition;2]{
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Named price",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn actual_activation_price_records_once_for_mixed_or_zero_resources_and_never_on_failed_total(){
 for count in [0,2,3]{
 let text=format!("Type: Artifact\nPay the cost \"Choose {count} untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\" as the support action: Draw a card.");
 for definition in pairs(&text){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Witness","Type: Enchantment\nWhenever you perform the support action with an amount of 1 or more, draw a card.",false).unwrap();game.create_object_from_definition(&witness,A,Zone::Battlefield);
  let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);let blue=game.create_object_from_definition(&body("{U}"),A,Zone::Graveyard);let stable=game.object(blue).unwrap().stable_id;
  let cost=definition.abilities.iter().find_map(|ability|if let ironsmith::ability::AbilityKind::Activated(activation)=&ability.kind{Some(activation.mana_cost.clone())}else{None}).unwrap();
  let [cost]=cost.as_all().unwrap()else{panic!("one complete named price component")};
  let mut dm=SelectFirstDecisionMaker;let mut ctx=CostContext::new(source,A,&mut dm);
  assert_eq!(cost.can_pay(&game,&ctx).is_ok(),count<=2);assert_eq!(cost.pay(&mut game,&mut ctx).is_ok(),count<=2);
  assert_eq!(game.named_action_count(source,"support",true),u32::from(count<=2));
  assert_eq!(game.is_tapped(white),count==2);assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,if count==2{Zone::Exile}else{Zone::Graveyard});
  let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(queue.entries.len(),usize::from(count==2));
 }
 }
}
#[test]
fn named_price_effect_is_an_acknowledged_payment_not_an_optional_offer(){
 use ironsmith::effects::{EffectContext,execute_effect};
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&body("{0}"),A,Zone::Battlefield);
 let choose=ironsmith::effects::ChooseObjectsEffect::new(ironsmith::target::ObjectFilter::creature().you_control(),ironsmith::ChoiceCount::exactly(0),ironsmith::target::PlayerFilter::You,"selected".into());
 let price=Effect::new(ironsmith::effects::NamedActionProgramEffect{label:"support".into(),player:ironsmith::target::PlayerFilter::You,effects:vec![Effect::new(choose)],completion:ironsmith_core::effect::NamedActionProgramCompletion::PaidCost,objects:Some(ironsmith::target::ChooseSpec::Tagged("selected".into()))});
 let mut ctx=EffectContext::new_default(source,A);let outcome=execute_effect(&mut game,&price,&mut ctx).unwrap();
 assert!(outcome.instruction_result().execution_facts().contains(&ironsmith::effect::ExecutionFact::Accepted));
 let events=outcome.events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).collect::<Vec<_>>();assert_eq!(events.len(),1);assert_eq!(events[0].amount,0);assert!(events[0].objects.is_empty());
 assert_eq!(game.named_action_count(source,"support",true),1);
}

#[test]
fn mandatory_and_optional_additional_price_components_retain_the_same_named_payment_owner(){
 for optional in [false,true]{for amount in ["two","X"]{
 let text=format!("Type: Sorcery\nAs an additional cost to cast this spell, {}pay the cost \"Choose {amount} untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\" as the support action.\nDraw a card.",if optional{"you may "}else{""});
 for definition in pairs(&text){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);game.create_object_from_definition(&body("{U}"),A,Zone::Graveyard);
  let total=if optional{definition.optional_costs[0].cost.clone()}else{definition.additional_cost.clone()};let costs=total.as_all().unwrap();assert_eq!(costs.len(),1);let cost=costs[0].clone();let mut dm=SelectFirstDecisionMaker;let mut ctx=CostContext::new(source,A,&mut dm);ctx.x_value=Some(2);assert!(cost.can_pay(&game,&ctx).is_ok());cost.pay(&mut game,&mut ctx).unwrap();assert_eq!(game.named_action_count(source,"support",true),1);assert!(game.is_tapped(white));
 }
 }
 }
}

#[test]
fn named_price_cannot_reuse_the_source_already_reserved_for_a_tap_symbol(){
 let text="Mana cost: {W}\nType: Creature\nPower/Toughness: 2/2\nHaste\n{T}, Pay the cost \"Choose one untapped creature you control, each representing a different color it has; Tap the chosen creatures on the battlefield\" as the support action: Draw a card.";
 for definition in pairs(text){for enough in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let extra=enough.then(||game.create_object_from_definition(&body("{U}"),A,Zone::Battlefield));
  game.turn.active_player=A;game.turn.priority_player=Some(A);
  let action=ironsmith::decision::compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,ironsmith::decision::LegalAction::ActivateAbility{source:id,..}if *id==source));assert_eq!(action.is_some(),enough);
  if let Some(action)=action {
   let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
   let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
   for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&context,&mut dm).unwrap();}
   assert!(!state.has_pending_action());
  }
  assert_eq!(game.is_tapped(source),enough);if let Some(extra)=extra{assert!(game.is_tapped(extra));}assert_eq!(game.named_action_count(source,"support",true),u32::from(enough));
 }}
}
