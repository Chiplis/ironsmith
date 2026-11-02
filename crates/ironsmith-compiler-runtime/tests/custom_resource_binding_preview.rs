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
fn mixed_resource_query_publishes_only_scoped_partition_bindings(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let source=game.create_object_from_definition(&body("{0}"),A,Zone::Stack);
 let dual=game.create_object_from_definition(&body("{W}{U}"),A,Zone::Battlefield);
 let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);
 let red=game.create_object_from_definition(&body("{R}"),A,Zone::Graveyard);let red_stable=game.object(red).unwrap().stable_id;
 let enemy=game.create_object_from_definition(&body("{G}"),B,Zone::Battlefield);
 let tapped=game.create_object_from_definition(&body("{B}"),A,Zone::Battlefield);game.tap(tapped);
 let cost=contribution_cost(3);let mut dm=SelectFirstDecisionMaker;let mut ctx=CostContext::new(source,A,&mut dm);
 assert!(ctx.tagged_objects.is_empty());
 assert!(cost.can_pay(&game,&ctx).is_ok());
 assert!(cost.can_pay(&game,&ctx).is_ok());
 assert!(ctx.tagged_objects.is_empty());assert!(!game.is_tapped(dual));assert!(!game.is_tapped(white));assert_eq!(game.object(red).unwrap().zone,Zone::Graveyard);
 cost.pay(&mut game,&mut ctx).unwrap();
 assert!(game.is_tapped(dual));assert!(game.is_tapped(white));assert!(!game.is_tapped(enemy));assert!(game.is_tapped(tapped));
 assert_eq!(game.object(game.find_object_by_stable_id(red_stable).unwrap()).unwrap().zone,Zone::Exile);
}

#[test]
fn filtered_capture_preview_preserves_original_partition_and_empty_evidence() {
 use ironsmith::effects::{CostExecutableEffect,EffectContext,TagMatchingObjectsEffect};
 use ironsmith::snapshot::ObjectSnapshot;
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let source=game.create_object_from_definition(&body("{0}"),A,Zone::Stack);
 let white=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);
 let blue=game.create_object_from_definition(&body("{U}"),A,Zone::Graveyard);
 let capture=TagMatchingObjectsEffect::new(ObjectFilter::default().in_zone(Zone::Battlefield),"partition")
  .from_filtered_tagged_sources(vec!["selection".into()]);
 let mut ctx=EffectContext::new_default(source,A);
 assert!(capture.preview_payment_bindings(&game,&mut ctx).is_err());
 ctx.set_tagged_objects("selection",vec![ObjectSnapshot::from_object_id(&game,white).unwrap(),ObjectSnapshot::from_object_id(&game,blue).unwrap()]);
 capture.preview_payment_bindings(&game,&mut ctx).unwrap();
 let selected=ctx.get_tagged_all("partition").unwrap();
 assert_eq!(selected.len(),1);assert_eq!(selected[0].object_id,white);
 assert!(!game.is_tapped(white));assert_eq!(game.object(blue).unwrap().zone,Zone::Graveyard);
 ctx.set_tagged_objects("selection",vec![]);
 capture.preview_payment_bindings(&game,&mut ctx).unwrap();
 assert!(ctx.get_tagged_all("partition").unwrap().is_empty());
}
