//! Complete source and layer3 behavior; parent runs after subset compiler integration.
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::continuous::{ContinuousEffect,Modification};
use ironsmith::effect::Until;
use ironsmith_compiled_artifact::CompiledCardArtifact;
fn compile(text:&str)->ironsmith::cards::CardDefinition {
 ironsmith_compiler_runtime::compile_to_artifact("Optional document",text,false).unwrap().1
}
fn activation(game:&GameState,id:ironsmith::ObjectId)->ironsmith::resolution::ResolutionProgram {
 game.calculated_characteristics(id).unwrap().abilities.iter().find_map(|ability| match &ability.kind {
  ironsmith::ability::AbilityKind::Activated(ability)=>Some(ability.effects.clone()),_=>None
 }).unwrap()
}
#[test]
fn full_sources_and_receipt_readers_compile_losslessly_and_transport_all_subsets(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-glitch-fragments.json")).unwrap();
 let boundaries=fixture["unresolved_source_boundaries"].as_array().expect("explicit unresolved-source matrix");
 assert_eq!(boundaries.len(),1);assert_eq!(boundaries[0]["source_card_key"],"Glitch, Cosmic Dreamer_OTH");
 assert_eq!(boundaries[0]["status"],"explicit_user_boundary");
 let mut n=0;let mut unresolved=0;let mut failures=Vec::new();
 for row in fixture["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){
  if card["source_card_key"]==boundaries[0]["source_card_key"] {
   assert_eq!(card["unresolved_source_boundary"],boundaries[0]);
   assert!(card["original_text"].as_str().unwrap().contains(boundaries[0]["ability_text"].as_str().unwrap()));
   assert!(card["executable_text"].is_null());assert_eq!(card["executable_faces"].as_array().unwrap().len(),0);
   assert_eq!(card["rewrite_status"],"explicit_user_boundary");
   unresolved+=1;continue; // Deliberately NOT a whole-source compilation/semantic pass.
  }
  if card["rewrite_status"]!="frontend_materialized_unverified"{failures.push(format!("{}: {}",card["name"],card["rewrite_status"]));continue}
  for face in card["executable_faces"].as_array().unwrap(){
   let name=face["name"].as_str().unwrap();let text=face["executable_text"].as_str().unwrap();
   let vocabulary:Vec<ironsmith_compiler::CustomSubtypeDefinition>=serde_json::from_value(face.get("custom_subtypes").or_else(||card.get("custom_subtypes")).cloned().unwrap_or_else(||serde_json::json!([]))).expect("complete source subtype metadata");
   let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::with_custom_subtypes(&vocabulary,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)).expect("valid source subtype scope")));
   if loss.is_lossy(){failures.push(format!("{name}: {}",loss.reasons_text()));}
   let(artifact,direct)=match result{Ok(v)=>v,Err(e)=>{failures.push(format!("{name}: {e}"));continue}};
   let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
   let indirect=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
   assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&indirect));n+=1;
  }
 }}assert!(failures.is_empty(),"{}",failures.join("\n"));assert_eq!(n,10);assert_eq!(unresolved,1);assert_eq!(n+unresolved,11);
}
#[test]
fn lasting_deletion_replays_and_does_not_capture_temporary_word_changes(){
 let definition=compile("Type: Artifact\n{T}: Destroy target black creature [with flying].\n{1}: Remove one bracketed text fragment from this permanent.");
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let a=PlayerId(0);
 let id=game.create_object_from_definition(&definition,a,Zone::Battlefield);
 let original=game.calculated_copiable_values(id).unwrap();
 let manager=game.calculated_characteristics(id).unwrap().abilities.iter().filter_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Activated(ability)=>Some(ability.effects.activation_definition),_=>None}).nth(1).unwrap();
 assert!(manager.is_some(),"the persistent manager needs a stable authored activation identity");
 let change=game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id,a,vec![id],Modification::RewriteText(ironsmith_core::TextChange::color(ironsmith_core::Color::Black,ironsmith_core::Color::White).unwrap())).until(Until::EndOfTurn));
 game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id,a,vec![id],Modification::RemoveOptionalTextFragments(1)).until(Until::Forever));
 game.refresh_continuous_state().unwrap();
 let white=compile("Type: Artifact\n{T}: Destroy target white creature.");let expected=game.create_object_from_definition(&white,a,Zone::Battlefield);
 assert_eq!(activation(&game,id)[0].0.get_target_spec(),activation(&game,expected)[0].0.get_target_spec());
 assert_eq!(game.calculated_copiable_values(id).unwrap(),original);
 let retained=game.calculated_characteristics(id).unwrap().abilities.iter().filter_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Activated(ability)=>Some(ability.effects.activation_definition),_=>None}).nth(1).unwrap();
 assert_eq!(retained,manager);
 game.effect_store.continuous_effects.remove_effect(change);game.refresh_continuous_state().unwrap();
 let black=compile("Type: Artifact\n{T}: Destroy target black creature.");let expected=game.create_object_from_definition(&black,a,Zone::Battlefield);
 assert_eq!(activation(&game,id)[0].0.get_target_spec(),activation(&game,expected)[0].0.get_target_spec());
 assert_eq!(game.calculated_copiable_values(id).unwrap(),original);
}

#[test]
fn announced_subsets_change_target_program_and_excess_payments_keep_full_removal(){
 let source="Type: Instant\nAs you cast this spell, you may pay the following additional cost any number of times: {1}.\nFor each time you pay this spell's optional additional cost, remove one bracketed text fragment from it.\nDestroy target [black] creature [with flying].";
 let definition=compile(source);let a=PlayerId(0);
 for (count,mask,expected_text) in [(0,0,"Destroy target black creature with flying."),(1,1,"Destroy target creature with flying."),(1,2,"Destroy target black creature."),(2,3,"Destroy target creature."),(19,3,"Destroy target creature.")] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let id=game.create_object_from_definition(&definition,a,Zone::Stack);
  let object=game.object_mut(id).unwrap();
  // This test injects an already-announced receipt rather than casting. New
  // uncast objects correctly have no paid slots; use the ordinary authored
  // cost initialization before recording synthetic completed payments.
  object.optional_costs_paid=ironsmith::cost::OptionalCostsPaid::from_costs(&object.optional_costs);
  object.optional_costs_paid.pay_times(0,count);object.optional_costs_paid.set_text_fragment_choice(0,mask);
  assert_eq!(object.optional_costs_paid.times_paid(0),count);
  game.refresh_continuous_state().unwrap();
  let expected=compile(&format!("Type: Instant\n{expected_text}"));let expected=game.create_object_from_definition(&expected,a,Zone::Stack);
  assert_eq!(game.current_spell_program(id).unwrap()[0].0.get_target_spec(),game.current_spell_program(expected).unwrap()[0].0.get_target_spec(),"count={count} mask={mask}");
 }
}

#[test]
fn persistent_manager_uses_its_selected_body_and_replays_composed_words_once(){
 let definition=compile("Type: Artifact\n{1}: Remove one bracketed text fragment from target green permanent. [You gain 1 life.]");
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let a=PlayerId(0);let id=game.create_object_from_definition(&definition,a,Zone::Battlefield);
 let before=activation(&game,id);assert_eq!(before.len(),2);let stamp=before.activation_definition;assert!(stamp.is_some());
 let declared=definition.abilities.iter().find_map(|ability|match &ability.kind {
  ironsmith::AbilityKind::Static(rule)=>rule.canonical_model().and_then(|model|match model.payload {ironsmith_core::StaticAbilityPayload::OptionalTextFragments(spec)=>Some(spec),_=>None}),_=>None,
 }).expect("the original manager retains a typed subset plan");
 let removed=declared.variant(1).expect("the single removal subset is present");
 let ironsmith_core::AbilityKind::Activated(removed_manager)=&removed.abilities[0].kind else{panic!("same acquired manager slot")};
 assert_eq!(removed_manager.effects.len(),1,"the precompiled selected manager body, before any layers, must omit its own optional gain");
 assert_eq!(removed_manager.effects.activation_definition,stamp,"subset body replacement keeps the exact original acquisition identity");

 for (from,to) in [(ironsmith_core::Color::Red,ironsmith_core::Color::Blue),(ironsmith_core::Color::Green,ironsmith_core::Color::Red)]{
  game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id,a,vec![id],Modification::RewriteText(ironsmith_core::TextChange::color(from,to).unwrap())).until(Until::Forever));
 }
 game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id,a,vec![id],Modification::RemoveOptionalTextFragments(1)).until(Until::Forever));game.refresh_continuous_state().unwrap();
 let applied=game.calculated_characteristics(id).unwrap().abilities.iter().find_map(|ability|match &ability.kind {
  ironsmith::AbilityKind::Static(rule)=>rule.canonical_model().and_then(|model|match model.payload {ironsmith_core::StaticAbilityPayload::OptionalTextFragments(spec)=>Some(spec.current_removed),_=>None}),_=>None,
 }).expect("subset selection keeps the typed plan marker");
 assert_eq!(applied,1,"the lasting layer-three deletion must select its exact authored subset after word changes");
 let selected=activation(&game,id);assert_eq!(selected.len(),1,"the manager's own deleted life-gain sentence must disappear");assert_eq!(selected.activation_definition,stamp);
 let expected=compile("Type: Artifact\n{1}: Remove one bracketed text fragment from target red permanent.");let expected=game.create_object_from_definition(&expected,a,Zone::Battlefield);
 assert_eq!(selected[0].0.get_target_spec(),activation(&game,expected)[0].0.get_target_spec(),"green becomes red once; preserving an already rewritten manager would incorrectly turn it blue");
 let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(selected[0].clone()).unwrap();
 let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();
 assert_eq!(restored.0.get_target_spec(),selected[0].0.get_target_spec(),"the text-rewritten native fragment manager retains complete typed target transport");
}

#[test]
fn movement_artifact_retains_nonempty_typed_arrival_modifications_and_exact_lifetime() {
 use ironsmith_compiled_artifact::{ArtifactCardId,ArtifactCardIdentity,CompiledCardPayload,wire_definition_from_serializable};
 use ironsmith::target::ChooseSpec;
 let mut model=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Arrival instructions")
  .card_types(vec![ironsmith_core::CardType::Sorcery]).build();
 let mut movement=ironsmith_compiler::effects::MoveToZoneEffect::new(ChooseSpec::Tagged("arrival".into()),Zone::Battlefield,false).under_you_control();
 movement.entry_modifications=vec![ironsmith_compiler::continuous::Modification::SetName("Arrived form".into()),ironsmith_compiler::continuous::Modification::AddSubtypes(vec![ironsmith_core::Subtype::Wizard])];
 model.spell_effect=Some(vec![ironsmith_compiler::effect::Effect::new(movement)].into());
 model.spell_effect.as_mut().unwrap().object_reference_imports.push("arrival".into());
 let direct=ironsmith_compiler_runtime::into_runtime_definition(model.clone()).unwrap();
 let artifact=CompiledCardArtifact::new(ArtifactCardIdentity{local_id:ArtifactCardId(model.card.id.0),name:model.card.name.clone(),face_name:None,other_face:None,linked_face_layout:None},CompiledCardPayload{
  definition:wire_definition_from_serializable(&model).unwrap(),canonical_text:String::new(),ability_labels:Vec::new(),
 },"typed-arrival-control",b"typed-arrival-control");
 let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
 for definition in [direct,restored] {
  let movement=definition.spell_effect.as_ref().unwrap()[0].downcast_ref::<ironsmith::effects::MoveToZoneEffect>().unwrap();
  assert_eq!(movement.entry_modifications.len(),2,"the typed codec must preserve arrival modifications rather than erase them to unit payloads");
  let a=PlayerId(0);let b=PlayerId(1);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&definition,a,Zone::Stack);
  let victim=compile("Type: Creature — Human\nPower/Toughness: 2/2");
  let target=game.create_object_from_definition(&victim,b,Zone::Graveyard);let stable=game.object(target).unwrap().stable_id;
  let mut context=ironsmith::effects::EffectContext::new_default(source,a);
  context.tagged_objects.insert("arrival".into(),vec![ironsmith::snapshot::ObjectSnapshot::from_object(game.object(target).unwrap(),&game)]);
  ironsmith::effects::execute_effect(&mut game,&definition.spell_effect.as_ref().unwrap()[0],&mut context).unwrap();
  let arrived=game.find_object_by_stable_id(stable).unwrap();
  assert_eq!(game.current_controller(arrived),Some(a));
  let chars=game.current_characteristics(arrived).unwrap();assert_eq!(chars.name.as_ref(),"Arrived form");
  assert!(chars.subtypes.contains(&ironsmith_core::Subtype::Human));assert!(chars.subtypes.contains(&ironsmith_core::Subtype::Wizard));
  assert_eq!(game.calculated_copiable_values(arrived).unwrap().name.as_str(),"Optional document","arrival changes do not overwrite copiable definition metadata");
  game.move_object_by_effect(arrived,Zone::Hand).unwrap();let returned=game.find_object_by_stable_id(stable).unwrap();
  assert_eq!(game.current_characteristics(returned).unwrap().name.as_ref(),"Optional document");
  assert!(!game.current_characteristics(returned).unwrap().subtypes.contains(&ironsmith_core::Subtype::Wizard),"a new incarnation cannot inherit the old arrival modification");
 }
}

#[test]
fn synthetic_subset_receipts_still_reject_an_unpaid_nonzero_mask() {
 let source="Type: Instant\nAs you cast this spell, you may pay the following additional cost any number of times: {1}.\nFor each time you pay this spell's optional additional cost, remove one bracketed text fragment from it.\nDestroy target [black] creature.";
 let definition=compile(source);let a=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let id=game.create_object_from_definition(&definition,a,Zone::Stack);
 let object=game.object_mut(id).unwrap();object.optional_costs_paid=ironsmith::cost::OptionalCostsPaid::from_costs(&object.optional_costs);
 object.optional_costs_paid.set_text_fragment_choice(0,1);
 assert_eq!(object.optional_costs_paid.times_paid(0),0);
 assert!(game.refresh_continuous_state().is_err(),"initializing an authored slot must not turn an unpaid removal mask into a completed payment");
}
