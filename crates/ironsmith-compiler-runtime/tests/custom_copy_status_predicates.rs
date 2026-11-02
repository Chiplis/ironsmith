//! An exact copy property composes with arbitrary keyword identity, not payment.
mod custom_final_next_wave_support;
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::object::ObjectKind;
use ironsmith::filter::ObjectFilterExt as _;
const A:PlayerId=PlayerId(0);
#[test]
fn exact_copy_and_keyword_predicate_ignores_payment_and_rejects_uncast_originals(){
 for label in ["arbitrary spark","tapped story"] {
 let input=serde_json::json!({"name":"Independent copy predicate","custom_keyword_identities":[label],"executable_text":format!("Type: Instant\nIf this spell is a copy with custom keyword \"{label}\", you gain 3 life.")});
 for definition in custom_final_next_wave_support::definitions(&input).unwrap(){for copy in [false,true]{for identity in [false,true]{for paid in [false,true]{
  let mut game=GameState::new(vec!["A".into()],20);let id=game.create_object_from_definition(&definition,A,Zone::Stack);
  if copy{game.object_mut(id).unwrap().kind=ObjectKind::SpellCopy;}
  if !identity{game.object_mut(id).unwrap().abilities_mut().retain(|ability|!matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(s)if s.custom_keyword_name().is_some_and(|name|name==label)));}
  if paid{let mut receipt=ironsmith_core::OptionalCostRef::from("Additional");receipt.custom_keyword=Some(label.into());game.object_mut(id).unwrap().optional_costs_paid.costs.push((receipt,1));}
  let filter=ironsmith::target::ObjectFilter{zone:Some(Zone::Stack),spell_copy:true,ability_markers:vec![format!("custom keyword:{label}")],..Default::default()};let context=game.filter_context_for(A,Some(id));
  let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(id).unwrap(),&game);
  assert_eq!(filter.matches_snapshot(&snapshot,&context,&game),copy&&identity,"LKI retains exact copy kind and keyword identity");
  ironsmith::effects::execute_effect(&mut game,&definition.spell_effect.as_ref().unwrap()[0].0,&mut ironsmith::effects::EffectContext::new_default(id,A)).unwrap();
  assert_eq!(game.player(A).unwrap().life,if copy&&identity{23}else{20},"copy={copy} identity={identity} paid={paid}");
 }}}}
 }
}
#[test]
fn old_filter_wire_defaults_do_not_reclassify_objects_as_copies(){
 let value=serde_json::to_value(ironsmith_core::ObjectFilter::default()).unwrap();assert!(value.get("spell_copy").is_none());let decoded:ironsmith_core::ObjectFilter=serde_json::from_value(value).unwrap();assert!(!decoded.spell_copy);
}
