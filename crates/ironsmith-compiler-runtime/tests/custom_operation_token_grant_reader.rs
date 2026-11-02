//! Complete ordinary quoted abilities under an explicit operation scope.
#[path="custom_final_next_wave_support.rs"] mod source_support;
#[test]
fn generic_operation_grant_retains_quoted_combat_program_on_both_routes(){
 let face=serde_json::json!({"name":"Scoped token program","executable_text":"Type: Creature\nPower/Toughness: 3/3\nAs you perform the forge bundle action, tokens you create gain \"Whenever this creature deals combat damage to a player, you gain 1 life.\""});
 for definition in source_support::definitions(&face).unwrap(){
  let rule=definition.abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Static(rule)=>rule.compiled_model(),_=>None}).unwrap();
  let ironsmith_core::StaticAbilityPayload::NamedActionTokenAbilityGrant{label,abilities,..}=&rule.payload else{panic!("complete grant model: {rule:?}")};
  assert_eq!(label,"forge bundle");assert_eq!(abilities.len(),1);
  assert!(matches!(abilities[0].kind,ironsmith_core::AbilityKind::Triggered(_)));
 }
}
#[test]
fn unread_child_is_not_accepted_as_a_complete_token_grant(){
 let text="Type: Creature\nPower/Toughness: 3/3\nAs you perform the forge bundle action, tokens you create gain \"Whenever this creature deals combat damage to a player, perform an undefined database operation.\"";
 assert!(ironsmith_compiler_runtime::compile_to_artifact("Rejected scoped grant",text,false).is_err());
}
