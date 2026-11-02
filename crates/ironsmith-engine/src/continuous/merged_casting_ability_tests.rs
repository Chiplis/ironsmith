//! Casting abilities are copiable abilities even when held outside AbilityKind.
use super::*;
use crate::{CardDefinition,CardDefinitionBuilder,CardId,CardType,GameState,ObjectId,PlayerId,PowerToughness,Zone};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::mana::ManaCost;
use crate::mana::ManaSymbol;
fn creature(name:&str,casting:Option<u8>)->CardDefinition {
 let mut builder=CardDefinitionBuilder::new(CardId::new(),name).card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,3));
 if let Some(amount)=casting {
  let cost=ManaCost::from_symbols(vec![ManaSymbol::Generic(amount)]);
  builder=builder.alternative_cast(AlternativeCastingMethod::Mutate {cost:cost.clone()})
   .kicker_mana(cost);
 }
 builder.build()
}
fn copies_keep(game:&mut GameState,source:ObjectId,expected:usize) {
 let values=game.calculated_copiable_values(source).unwrap();
 assert_eq!(values.alternative_casts.len(),expected,"the layer-one envelope retains all component casting abilities");
 assert_eq!(values.optional_costs.len(),expected,"optional-cost abilities use the same merged acquisition rule");
 let mut snapshot=crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),game);
 // The copy packet is the calculated layer-one result, not the object's
 // independently materialized physical component fields.
 snapshot.copiable_values=values.clone();
 let token=crate::object::Object::token_copy_from_snapshot(&snapshot,game.new_object_id(),PlayerId(0));
 assert_eq!(token.alternative_casts.as_slice(),values.alternative_casts.as_slice());
 assert_eq!(token.optional_costs.as_slice(),values.optional_costs.as_slice());
 let mut ordinary=crate::object::Object::from_card_definition(game.new_object_id(),&creature("Ordinary copier",Some(9)),PlayerId(0),Zone::Battlefield);
 ordinary.copy_copiable_values_from_values(&values);
 assert_eq!(ordinary.alternative_casts.as_slice(),values.alternative_casts.as_slice(),"ordinary copying replaces its own prior casting ability");
 assert_eq!(ordinary.optional_costs.as_slice(),values.optional_costs.as_slice());
 assert_eq!(game.calculated_copiable_values(source).unwrap(),values,"copy capture does not mutate its source");
}
#[test]
fn merging_above_and_below_preserves_each_copiable_casting_ability_occurrence() {
 for host_has_cost in [false,true] {for top in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let actor=PlayerId(0);
  let host=game.create_object_from_definition(&creature("Merge host",host_has_cost.then_some(1)),actor,Zone::Battlefield);
  for iteration in 0..2 {
   // Repeated equal declarations must not be deduplicated by display or cost.
   let incoming=game.create_object_from_definition(&creature("Incoming component",Some(2)),actor,Zone::Stack);
   assert_eq!(game.merge_mutating_creature_spell(incoming,host,top),Some(host));
   copies_keep(&mut game,host,usize::from(host_has_cost)+iteration+1);
  }
 }}
}
#[test]
fn a_face_down_component_does_not_contribute_its_hidden_casting_abilities() {
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let actor=PlayerId(0);
 let host=game.create_object_from_definition(&creature("Hidden merge host",Some(7)),actor,Zone::Battlefield);
 assert!(game.set_face_down(host));
 let incoming=game.create_object_from_definition(&creature("Visible component",Some(2)),actor,Zone::Stack);
 assert_eq!(game.merge_mutating_creature_spell(incoming,host,true),Some(host));
 copies_keep(&mut game,host,1);
 let values=game.calculated_copiable_values(host).unwrap();
 assert_eq!(values.alternative_casts[0].mutate_cost(),Some(&ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])));
}
