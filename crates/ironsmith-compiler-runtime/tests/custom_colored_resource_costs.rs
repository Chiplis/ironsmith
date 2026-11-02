//! Generic matching constraint composes with ordinary tap/exile payment owners.
use ironsmith::{Effect,GameState,PlayerId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::costs::{Cost,CostContext,CostPayer};
use ironsmith::target::{ObjectFilter,ChooseSpec,PlayerFilter};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn body(mana:&str)->ironsmith::CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Contribution",&format!("Mana cost: {mana}\nType: Creature\nPower/Toughness: 2/2"),false).unwrap()}
fn contribution_cost(count:usize)->Cost{
 let mut filter=ObjectFilter::default();filter.any_of=vec![ObjectFilter::creature().you_control().untapped().in_zone(Zone::Battlefield),ObjectFilter::default().owned_by(PlayerFilter::You).in_zone(Zone::Graveyard)];
 let tag:ironsmith::tag::TagKey="contribution".into();
 let choose=ironsmith::effects::ChooseObjectsEffect::new(filter,ironsmith::ChoiceCount::exactly(count),PlayerFilter::You,tag.clone()).with_aggregate_constraint(ironsmith::effect::ChoiceAggregateConstraint{metric:ironsmith::effect::ChoiceAggregateMetric::DistinctColorRepresentatives,minimum:Some(ironsmith::effect::Value::Fixed(count as i32)),maximum:ironsmith::effect::Value::Fixed(count as i32)});
 let selected=ObjectFilter::default().match_tagged(tag,ironsmith::filter::TaggedOpbjectRelation::IsTaggedObject);
 Cost::try_effect(Effect::new(ironsmith::effects::SequenceEffect::new(vec![Effect::new(choose),Effect::new(ironsmith::effects::TapEffect::with_spec(ChooseSpec::All(selected.clone().in_zone(Zone::Battlefield)))),Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::All(selected.in_zone(Zone::Graveyard))))]))).expect("the complete resource program must pass public cost admission")
}
#[test]
fn multicolor_matching_can_use_mixed_tap_and_exile_resources_without_reusing_any_object(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let source=game.create_object_from_definition(&body("{0}"),A,Zone::Stack);
 let dual=game.create_object_from_definition(&body("{W}{U}"),A,Zone::Battlefield);
 let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);
 let red=game.create_object_from_definition(&body("{R}"),A,Zone::Graveyard);let red_stable=game.object(red).unwrap().stable_id;
 let enemy=game.create_object_from_definition(&body("{G}"),B,Zone::Battlefield);
 let tapped=game.create_object_from_definition(&body("{B}"),A,Zone::Battlefield);game.tap(tapped);
 let cost=contribution_cost(3);let mut dm=SelectFirstDecisionMaker;let mut ctx=CostContext::new(source,A,&mut dm);
 assert!(cost.can_pay(&game,&ctx).is_ok());cost.pay(&mut game,&mut ctx).unwrap();
 assert!(game.is_tapped(dual));assert!(game.is_tapped(white));assert!(!game.is_tapped(enemy));assert!(game.is_tapped(tapped));
 assert_eq!(game.object(game.find_object_by_stable_id(red_stable).unwrap()).unwrap().zone,Zone::Exile);
}
#[test]
fn colorless_or_same_color_objects_and_a_single_multicolor_object_cannot_fake_distinct_contributions(){
 for mono in [false,true]{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&body("{0}"),A,Zone::Stack);
 let first=game.create_object_from_definition(&body(if mono{"{W}"}else{"{W}{U}{B}{R}{G}"}),A,Zone::Battlefield);
 let rest=(0..4).map(|_|game.create_object_from_definition(&body(if mono{"{W}"}else{"{0}"}),A,Zone::Battlefield)).collect::<Vec<_>>();
 let cost=contribution_cost(2);let mut dm=SelectFirstDecisionMaker;let mut ctx=CostContext::new(source,A,&mut dm);
 assert!(cost.can_pay(&game,&ctx).is_err());assert!(cost.pay(&mut game,&mut ctx).is_err());assert!(!game.is_tapped(first));assert!(rest.iter().all(|id|!game.is_tapped(*id)));assert!(game.exile.is_empty());
 }}
#[test]
fn generic_color_representative_reader_survives_artifact_roundtrip(){
 let text="Type: Sorcery\nChoose two creatures you control, each representing a different color it has.";
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Representative colors",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(decoded,artifact);let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();assert_eq!(format!("{:?}",direct.spell_effect),format!("{:?}",restored.spell_effect));
}
#[test]
fn quoted_program_is_one_real_activation_cost_and_roundtrips_with_its_choice_bindings(){
 const TEXT:&str="Type: Creature\nPower/Toughness: 2/2\nPay the cost \"Choose two untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\": Draw a card.";
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Resource activation",TEXT,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
 for definition in [direct,restored]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);let blue=game.create_object_from_definition(&body("{U}"),A,Zone::Graveyard);let stable=game.object(blue).unwrap().stable_id;
  let cost=definition.abilities.iter().find_map(|a|if let ironsmith::ability::AbilityKind::Activated(a)=&a.kind{Some(a.mana_cost.clone())}else{None}).unwrap();let [cost]=cost.as_all().expect("ordinary activation price") else {panic!("one complete quoted cost component: {cost:?}")};let mut dm=SelectFirstDecisionMaker;let mut ctx=CostContext::new(source,A,&mut dm);assert!(cost.can_pay(&game,&ctx).is_ok());cost.pay(&mut game,&mut ctx).unwrap();assert!(game.is_tapped(white));assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
 }
}
#[test]
fn representative_target_relation_rejects_noninjective_partial_and_complete_sets(){
 use ironsmith::Target;
 use ironsmith::targeting::ResolvedTargetAggregateConstraint;
 for masks in [[1,1],[31,0],[3,1]]{
  let target1=Target::Object(ironsmith::ObjectId(10));let target2=Target::Object(ironsmith::ObjectId(11));
  let constraint=ResolvedTargetAggregateConstraint{metric:ironsmith::effect::ChoiceAggregateMetric::DistinctColorRepresentatives,maximum:2,target_values:vec![(target1,masks[0]),(target2,masks[1])],one_per_card_type:None};
  assert_eq!(constraint.allows(&[target1,target2]),masks==[3,1]);assert_eq!(constraint.supports_minimum(2),masks==[3,1]);
 }
}
#[test]
fn resolving_optional_program_uses_whole_payment_receipt_even_when_terminal_exile_is_empty(){
 use ironsmith::decision::DecisionMaker;
 struct Choices{accept:bool,offers:usize}
 impl DecisionMaker for Choices{
  fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.offers+=1;self.accept}
  fn decide_objects(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{SelectFirstDecisionMaker.decide_objects(g,c)}
 }
 const TEXT:&str="Type: Sorcery\nYou may pay the cost \"Choose two untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\". If you do, draw a card.";
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Optional resource program",TEXT,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
 for definition in [direct,restored]{for accept in [false,true]{for available in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);let blue=available.then(||game.create_object_from_definition(&body("{U}"),A,Zone::Battlefield));game.create_object_from_definition(&body("{0}"),A,Zone::Library);
  let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();let mut dm=Choices{accept,offers:0};ironsmith::effects::execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ironsmith::effects::EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(dm.offers,usize::from(available));assert_eq!(game.is_tapped(white),available&&accept);if let Some(blue)=blue{assert_eq!(game.is_tapped(blue),accept);}assert_eq!(game.player(A).unwrap().hand.len(),usize::from(available&&accept));assert!(game.exile.is_empty());
 }}}
}

#[test]
fn announced_variable_representative_cost_keeps_x_as_an_exact_selection_bound(){
 let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Variable Contributor","Type: Creature\nPower/Toughness: 2/2\nPay the cost \"Choose X untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\": Draw a card.",false).unwrap();
 let debug=format!("{definition:?}");
 assert!(debug.contains("DistinctColorRepresentatives"),"{debug}");
 assert!(debug.contains("dynamic_x: true"),"{debug}");
}

#[test]
fn resolving_variable_payment_binds_exact_chosen_count_and_restores_outer_x(){
 use ironsmith::decision::DecisionMaker;
 struct Choices{accept:bool,count:usize}
 impl DecisionMaker for Choices{
  fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.accept}
  fn decide_objects(&mut self,_:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{c.candidates.iter().filter(|candidate|candidate.legal).take(self.count).map(|candidate|candidate.id).collect()}
 }
 const TEXT:&str="Type: Sorcery\nYou may pay the cost \"Choose any number of untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\". With X equal to the number of objects chosen this way, do the following: If you do, draw X cards and gain 1 life.";
 let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact("Variable resolution program",TEXT,false).unwrap();
 let named_text=TEXT.replacen("You may pay the cost", "Perform the contribution action with objects chosen this way if the following cost is paid: You may pay the cost",1);
 let (named_artifact,named_direct)=ironsmith_compiler_runtime::compile_to_artifact("Named variable resolution program",&named_text,false).unwrap();
 let named_decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&named_artifact.to_json().unwrap()).unwrap();let named_restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&named_decoded).unwrap();
 let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
 for (route,definition) in [("direct",direct),("artifact",restored),("named-direct",named_direct),("named-artifact",named_restored)]{for accept in [false,true]{for count in [0,2]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);let blue=game.create_object_from_definition(&body("{U}"),A,Zone::Battlefield);
  for _ in 0..3{game.create_object_from_definition(&body("{0}"),A,Zone::Library);}
  let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();let mut dm=Choices{accept,count};let mut ctx=ironsmith::effects::EffectContext::new_default(source,A).with_decision_maker(&mut dm);ctx.x_value=Some(99);
  ironsmith::effects::execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ctx).unwrap();
  assert_eq!(ctx.x_value,Some(99),"route={route}, accept={accept}, count={count}");assert_eq!(game.player(A).unwrap().hand.len(),if accept{count}else{0},"route={route}, accept={accept}, count={count}; tags={:?}; effects={:?}",ctx.tagged_objects,definition.spell_effect);assert_eq!(game.player(A).unwrap().life,20+i32::from(accept));assert_eq!(game.is_tapped(white),accept&&count>0);assert_eq!(game.is_tapped(blue),accept&&count>0);
 }}}
}
