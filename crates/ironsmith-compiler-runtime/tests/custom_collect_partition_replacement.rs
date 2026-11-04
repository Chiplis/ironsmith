//! Initial resource routes remain disjoint when the first payment is replaced.
use ironsmith::{Effect,GameState,PlayerId,Zone};
use ironsmith::costs::{Cost,CostContext};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::target::{ChooseSpec,ObjectFilter,PlayerFilter};
const A:PlayerId=PlayerId(0);
fn body(mana:&str)->ironsmith::CardDefinition {ironsmith_compiler_runtime::compile_to_runtime_definition("Resource",&format!("Mana cost: {mana}\nType: Creature\nPower/Toughness: 1/1"),false).unwrap()}
#[test]
fn replaced_tap_cannot_route_the_same_selected_resource_into_the_exile_payment() {
    for explicit_capture in [false,true] {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let source=game.create_object_from_definition(&body("{0}"),A,Zone::Stack);
    let battlefield=game.create_object_from_definition(&body("{W}"),A,Zone::Battlefield);
    let graveyard=game.create_object_from_definition(&body("{U}"),A,Zone::Graveyard);
    let battlefield_stable=game.object(battlefield).unwrap().stable_id;
    let graveyard_stable=game.object(graveyard).unwrap().stable_id;
    let chosen:ironsmith::tag::TagKey="chosen_resource_set".into();
    let battlefield_tag:ironsmith::tag::TagKey="initial_battlefield_resources".into();
    let graveyard_tag:ironsmith::tag::TagKey="initial_graveyard_resources".into();
    let mut filter=ObjectFilter::default();filter.any_of=vec![ObjectFilter::creature().you_control().untapped().in_zone(Zone::Battlefield),ObjectFilter::default().owned_by(PlayerFilter::You).in_zone(Zone::Graveyard)];
    let choose=ironsmith::effects::ChooseObjectsEffect::new(filter,ironsmith::ChoiceCount::exactly(2),PlayerFilter::You,chosen.clone()).with_aggregate_constraint(ironsmith::effect::ChoiceAggregateConstraint{metric:ironsmith::effect::ChoiceAggregateMetric::DistinctColorRepresentatives,minimum:Some(ironsmith::effect::Value::Fixed(2)),maximum:ironsmith::effect::Value::Fixed(2)});
    let freeze_battlefield=ironsmith::effects::TagMatchingObjectsEffect::new(ObjectFilter::default().in_zone(Zone::Battlefield),battlefield_tag.clone()).from_filtered_tagged_sources([chosen.clone()]);
    let freeze_graveyard=ironsmith::effects::TagMatchingObjectsEffect::new(ObjectFilter::default().in_zone(Zone::Graveyard),graveyard_tag.clone()).from_filtered_tagged_sources([chosen.clone()]);
    let payment = if explicit_capture {
        vec![Effect::new(choose),Effect::new(freeze_battlefield),Effect::new(freeze_graveyard),Effect::new(ironsmith::effects::TapEffect::with_spec(ChooseSpec::Tagged(battlefield_tag))),Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::Tagged(graveyard_tag)))]
    } else {
        let selected = ObjectFilter::default().match_tagged(chosen,ironsmith::filter::TaggedOpbjectRelation::IsTaggedObject);
        vec![Effect::new(choose),Effect::new(ironsmith::effects::TapEffect::with_spec(ChooseSpec::All(selected.clone().in_zone(Zone::Battlefield)))),Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::All(selected.in_zone(Zone::Graveyard))))]
    };
    let cost=Cost::try_effect(Effect::new(ironsmith::effects::SequenceEffect::new(payment))).expect("the complete resource program must pass public cost admission");
    game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(source,A,ironsmith::events::permanents::matchers::WouldBecomeTappedMatcher::new(ObjectFilter::specific(battlefield)),ironsmith::replacement::ReplacementAction::Instead(vec![Effect::move_to_zone(ChooseSpec::SpecificObject(battlefield),Zone::Graveyard,false)])));
    let mut dm=SelectFirstDecisionMaker;let mut context=CostContext::new(source,A,&mut dm);
    assert!(cost.can_pay(&game,&context).is_ok());
    cost.pay(&mut game,&mut context).unwrap();
    let moved_battlefield=game.find_object_by_stable_id(battlefield_stable).unwrap();
    let moved_graveyard=game.find_object_by_stable_id(graveyard_stable).unwrap();
    assert_eq!(game.object(moved_battlefield).unwrap().zone,Zone::Graveyard,"initial battlefield resource pays one replaced tap, never the exile branch");
    assert_eq!(game.object(moved_graveyard).unwrap().zone,Zone::Exile,"initial graveyard resource supplies the independent exile branch");
    }
}
