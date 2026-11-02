//! Generic subtype-static sibling and source-zone lifetime regressions.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith_core::{Subtype,SubtypeFamily};
use ironsmith_core::custom_subtypes::CustomSubtype;
#[path="custom_final_next_wave_support.rs"] mod source_routes;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(text:&str)->[CardDefinition;2]{
 source_routes::definitions(&serde_json::json!({
  "name":"Subtype grant witness","executable_text":text,
  "custom_subtypes":[
   {"singular":"Inscription","plural":"Inscriptions","family":"Spell"},
   {"singular":"Inscription","plural":"Inscriptions","family":"Artifact"}
  ]
 })).unwrap()
}
fn inscription()->Subtype{Subtype::Custom(CustomSubtype::new_with_families("Inscription",&[SubtypeFamily::Spell,SubtypeFamily::Artifact]).unwrap())}
#[test]
fn own_source_subtype_static_functions_in_every_applicable_zone(){
 for definition in definitions("Type: Sorcery\nDraw a card.\nThis card is an Inscription in addition to its other types."){
  for zone in [Zone::Hand,Zone::Library,Zone::Graveyard,Zone::Exile,Zone::Stack]{
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);let id=game.create_object_from_definition(&definition,A,zone);
   assert!(game.calculated_characteristics(id).unwrap().subtypes.contains(&inscription()),"{zone:?}");
  }
 }
}
#[test]
fn palace_shaped_grant_supplies_subtype_outside_battlefield_and_stops_when_grantor_leaves(){
 let grantors=definitions("Type: Artifact\nEach instant and sorcery card you own that isn't on the battlefield has \"This card is an Inscription in addition to its other types.\"");
 let spells=definitions("Type: Sorcery\nDraw a card.");
 for index in 0..2{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let grantor=game.create_object_from_definition(&grantors[index],A,Zone::Battlefield);
  let ids=[Zone::Hand,Zone::Library,Zone::Graveyard,Zone::Exile,Zone::Stack].map(|zone|game.create_object_from_definition(&spells[index],A,zone));
  for id in ids{assert!(game.calculated_characteristics(id).unwrap().subtypes.contains(&inscription()));}
  let foreign=game.create_object_from_definition(&spells[index],B,Zone::Hand);assert!(!game.calculated_characteristics(foreign).unwrap().subtypes.contains(&inscription()));
  ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(grantor),Zone::Hand,false)),&mut ironsmith::effects::EffectContext::new_default(grantor,A)).unwrap();
  for id in ids{assert!(!game.calculated_characteristics(id).unwrap().subtypes.contains(&inscription()));}
 }
}
#[test]
fn hand_only_grant_does_not_supply_the_subtype_in_exile(){
 let grantors=definitions("Type: Creature\nPower/Toughness: 1/2\nEach instant and sorcery card in your hand has \"This card is an Inscription in addition to its other types.\"");let spells=definitions("Type: Sorcery\nDraw a card.");
 for index in 0..2{let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&grantors[index],A,Zone::Battlefield);let hand=game.create_object_from_definition(&spells[index],A,Zone::Hand);let exile=game.create_object_from_definition(&spells[index],A,Zone::Exile);assert!(game.calculated_characteristics(hand).unwrap().subtypes.contains(&inscription()));assert!(!game.calculated_characteristics(exile).unwrap().subtypes.contains(&inscription()));}
}

#[test]
fn subtype_static_can_add_shared_spell_and_artifact_identity_to_equipment(){
 for definition in definitions("Type: Artifact — Equipment\nThis card is an Inscription in addition to its other types."){
  for zone in [Zone::Hand,Zone::Library,Zone::Graveyard,Zone::Exile,Zone::Stack,Zone::Battlefield]{
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);let id=game.create_object_from_definition(&definition,A,zone);
   assert!(game.calculated_characteristics(id).unwrap().subtypes.contains(&inscription()),"{zone:?}");
  }
 }
}

#[test]
fn ordinary_grants_do_not_become_all_zone_cdas_and_explicit_functional_zones_survive(){
 use ironsmith::continuous::{ContinuousEffect,EffectTarget,Modification};
 use ironsmith::target::ObjectFilter;
 let plain=definitions("Type: Artifact");
 let printed=definitions("Type: Artifact\nThis card is an Inscription in addition to its other types.");
 for route in 0..2{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let provider=game.create_object_from_definition(&plain[route],A,Zone::Battlefield);
  let true_cda=game.create_object_from_definition(&printed[route],A,Zone::Exile);
  let offzone=game.create_object_from_definition(&plain[route],A,Zone::Exile);
  let battlefield=game.create_object_from_definition(&plain[route],A,Zone::Battlefield);
  let explicit=game.create_object_from_definition(&plain[route],A,Zone::Exile);
  let mixed_hand=game.create_object_from_definition(&plain[route],A,Zone::Hand);
  let subtype=ironsmith::static_abilities::StaticAbility::add_subtypes(ObjectFilter::source(),vec![inscription()]);
  for target in [offzone,battlefield]{game.effect_store.continuous_effects.add_effect(
   ContinuousEffect::new(provider,A,EffectTarget::Specific(target),Modification::AddAbility(subtype.clone())));}
  let mixed=ObjectFilter {any_of:vec![ObjectFilter::default().in_zone(Zone::Exile),
   ObjectFilter {card_types:vec![ironsmith::CardType::Artifact],..Default::default()}],..Default::default()};
  game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(provider,A,
   EffectTarget::Filter(mixed),Modification::AddAbility(subtype.clone())));
  let ability=ironsmith::ability::Ability::static_ability(subtype).in_zones(vec![Zone::Exile]);
  game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(provider,A,EffectTarget::Specific(explicit),Modification::AddAbilityGeneric(ability)));
  assert!(game.calculated_characteristics(true_cda).unwrap().subtypes.contains(&inscription()));
  assert!(!game.calculated_characteristics(offzone).unwrap().subtypes.contains(&inscription()),"an unrestricted ordinary grant is not a CDA");
  assert!(!game.calculated_characteristics(mixed_hand).unwrap().subtypes.contains(&inscription()),"a scoped OR sibling must not authorize an unscoped branch's offzone grant");
  assert!(game.calculated_characteristics(battlefield).unwrap().subtypes.contains(&inscription()));
  assert!(game.calculated_characteristics(explicit).unwrap().subtypes.contains(&inscription()),"explicit Exile restriction must survive AddAbilityGeneric");
 }
}

#[test]
fn strict_word_claims_keep_designated_static_sibling_and_its_ability_group() {
 for label in ["inscribe", "Keyword ability \"inscribe\""] {
  let text=format!("Type: Sorcery\nDraw a card.\n{label} — This card is an Inscription in addition to its other types.");
  for definition in ironsmith_compiler::word_claims::with_enforcement_options(true,true,||definitions(&text)) {
   assert!(definition.abilities.iter().any(|ability|ability.has_designation("inscribe")),"the authored group must survive lowering");
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let card=game.create_object_from_definition(&definition,A,Zone::Exile);
   assert!(game.calculated_characteristics(card).unwrap().subtypes.contains(&inscription()));
  }
 }
}
#[test]
fn static_label_ownership_does_not_hide_unread_body_or_grant_a_missing_group() {
 for definition in ironsmith_compiler::word_claims::with_enforcement_options(true,true,||definitions("Type: Sorcery\nThis card is an Inscription in addition to its other types.")) {
  assert!(definition.abilities.iter().all(|ability|!ability.has_designation("inscribe")));
 }
 let vocabulary=[ironsmith_core::custom_subtypes::CustomSubtypeDefinition{singular:"Inscription".into(),plural:"Inscriptions".into(),family:SubtypeFamily::Spell}];
 let result=ironsmith_compiler::with_custom_subtypes(&vocabulary,||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_runtime_definition("Unread body witness","Type: Sorcery\ninscribe — This card is an Inscription in addition to its other types nonsense.",false))).unwrap();
 assert!(result.is_err(),"claiming the label must not authorize a malformed body");
}
