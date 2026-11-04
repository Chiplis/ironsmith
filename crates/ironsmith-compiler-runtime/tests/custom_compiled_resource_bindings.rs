//! Generic matching constraint composes with ordinary tap/exile payment owners.
use ironsmith::{Effect,GameState,PlayerId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::costs::{Cost,CostContext};
use ironsmith::target::{ObjectFilter,ChooseSpec,PlayerFilter};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn body(mana:&str)->ironsmith::CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Contribution",&format!("Mana cost: {mana}\nType: Creature\nPower/Toughness: 2/2"),false).unwrap()}
fn contribution_cost(count:usize)->Cost{
 let mut filter=ObjectFilter::default();filter.any_of=vec![ObjectFilter::creature().you_control().untapped().in_zone(Zone::Battlefield),ObjectFilter::default().owned_by(PlayerFilter::You).in_zone(Zone::Graveyard)];
 let tag:ironsmith::tag::TagKey="contribution".into();
 let choose=ironsmith::effects::ChooseObjectsEffect::new(filter,ironsmith::ChoiceCount::exactly(count),PlayerFilter::You,tag.clone()).with_aggregate_constraint(ironsmith::effect::ChoiceAggregateConstraint{metric:ironsmith::effect::ChoiceAggregateMetric::DistinctColorRepresentatives,minimum:Some(ironsmith::effect::Value::Fixed(count as i32)),maximum:ironsmith::effect::Value::Fixed(count as i32)});
 let mut selected=ObjectFilter::default().match_tagged(tag,ironsmith::filter::TaggedOpbjectRelation::IsTaggedObject);selected.set_plural_object_noun_surface(true);
 Cost::try_effect(Effect::new(ironsmith::effects::SequenceEffect::new(vec![Effect::new(ironsmith::effects::WithIdEffect::new(ironsmith::effect::EffectId(17),Effect::new(choose))),Effect::new(ironsmith::effects::TapEffect::with_spec(ChooseSpec::Object(selected.clone().in_zone(Zone::Battlefield)))),Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::Object(selected.in_zone(Zone::Graveyard))))]))).expect("the complete resource program must pass public cost admission")
}
#[test]
fn transparent_choice_identity_and_plural_references_preserve_complete_payment_scope(){
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
fn quoted_compiled_program_binds_decorated_choices_before_both_zone_consumers(){
 let text="Type: Creature\nPower/Toughness: 2/2\nPay the cost \"Choose two untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\": Draw a card.";
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::CompilerFacade::new().compile_definition(ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),"Complete resource program"),text.to_string(),ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());
 let direct=ironsmith_compiler_runtime::into_runtime_definition(result.unwrap().definition).unwrap();
 let (artifact,_)=ironsmith_compiler_runtime::compile_to_artifact("Complete resource program",text,false).unwrap();
 let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
 for definition in [direct,restored]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);
  let blue=game.create_object_from_definition(&body("{U}"),A,Zone::Graveyard);let blue_stable=game.object(blue).unwrap().stable_id;
  let total=definition.abilities.iter().find_map(|ability|if let ironsmith::ability::AbilityKind::Activated(ability)=&ability.kind{Some(&ability.mana_cost)}else{None}).unwrap();
  let [cost]=total.as_all().unwrap()else{panic!("single quoted program")};
  let mut dm=SelectFirstDecisionMaker;let mut ctx=CostContext::new(source,A,&mut dm);
  let admission=cost.can_pay(&game,&ctx);assert!(admission.is_ok(),"{admission:?}; {cost:?}");
  cost.pay(&mut game,&mut ctx).unwrap();assert!(game.is_tapped(white));assert_eq!(game.object(game.find_object_by_stable_id(blue_stable).unwrap()).unwrap().zone,Zone::Exile);
 }
}
