//! Selected-price composition preserves the original spell, physical card and copy provenance.
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
#[path = "custom_final_next_wave_support.rs"] mod source_support;
const A:PlayerId=PlayerId(0);
const RULES:&str="You may cast this spell from your graveyard for {2} rather than pay its mana cost. If you cast this spell this way, put it onto the battlefield as a Horror creature as it resolves. If this spell is kindred, it also keeps its creature types. Copies of this spell cannot enter the battlefield. If this spell would leave the stack other than being put onto the battlefield, exile it instead. If the permanent this spell becomes would leave the battlefield, exile it instead.";
fn definitions(kindred:bool,effect:&str)->[CardDefinition;2]{
 let text=format!("Mana cost: {{1}}\nType: {}Sorcery — {}Revelation\nPower/Toughness: 7/8\n{effect}\n{RULES}",if kindred{"Kindred "}else{""},if kindred{"Elf "}else{""});
 // Synthetic controls explicitly supply the sourced format vocabulary as
 // input data; the admission scope belongs to this one source compilation.
 let registry:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/source-subtypes.json")).unwrap();
 let type_line=text.lines().find_map(|line|line.strip_prefix("Type: ")).unwrap();
 let type_words:std::collections::BTreeSet<_>=type_line.split(|ch:char|!ch.is_alphanumeric()).filter(|word|!word.is_empty()).map(str::to_lowercase).collect();
 let vocabulary:Vec<_>=registry["definitions"].as_array().unwrap().iter().filter(|entry|entry["scope"]=="msem" && type_words.contains(&entry["singular"].as_str().unwrap().to_lowercase())).cloned().collect();
 source_support::definitions(&serde_json::json!({"name":"Physical resolution witness","executable_text":text,"custom_subtypes":vocabulary})).unwrap()

}
fn setup(definition:&CardDefinition,zone:Zone)->(GameState,ObjectId){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.register_linked_face_definition(definition);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
 let source=game.create_object_from_definition(definition,A,zone);(game,source)
}
fn cast(game:&mut GameState,source:ObjectId){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).expect("source has a legal casting price");
 let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}assert!(!state.has_pending_action());
}
fn move_to(game:&mut GameState,source:ObjectId,zone:Zone)->ObjectId{
 let stable=game.object(source).unwrap().stable_id;execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(source),zone,false)),&mut EffectContext::new_default(source,A)).unwrap();game.find_object_by_stable_id(stable).unwrap()
}
fn mana(game:&mut GameState,n:u32){game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,n);}
#[test]
fn selected_price_runs_spell_then_enters_as_horror_and_restores_physical_card_on_every_departure(){
 for kindred in [false,true]{for definition in definitions(kindred,"You gain 1 life."){
  let(mut game,source)=setup(&definition,Zone::Graveyard);mana(&mut game,1);assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)),"complete graveyard cost required");mana(&mut game,1);cast(&mut game,source);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
  let stack=game.stack[0].object_id;let stable=game.object(stack).unwrap().stable_id;assert!(game.object(stack).unwrap().card_types.contains(&ironsmith::CardType::Sorcery));assert_eq!(game.player(A).unwrap().life,20);
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().life,21);
  let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);assert_eq!(game.object(permanent).unwrap().kind,ironsmith::object::ObjectKind::Card);assert_eq!(game.current_power(permanent),Some(7));assert_eq!(game.current_toughness(permanent),Some(8));
  assert_eq!(game.object(permanent).unwrap().card_types.as_slice(),&[ironsmith::CardType::Creature]);assert!(game.object(permanent).unwrap().subtypes.contains(&ironsmith::Subtype::Horror));assert_eq!(game.object(permanent).unwrap().subtypes.contains(&ironsmith::Subtype::Elf),kindred);
  for destination in [Zone::Hand,Zone::Graveyard,Zone::Library,Zone::Exile]{let mut departed=game.clone();let exile=move_to(&mut departed,permanent,destination);let object=departed.object(exile).unwrap();assert_eq!(object.zone,Zone::Exile);assert!(object.card_types.contains(&ironsmith::CardType::Sorcery));assert!(!object.card_types.contains(&ironsmith::CardType::Creature));let hand=move_to(&mut departed,exile,Zone::Hand);let grave=move_to(&mut departed,hand,Zone::Graveyard);assert_eq!(departed.object(grave).unwrap().zone,Zone::Graveyard);}
 }}
}
#[test]
fn normal_cast_remains_a_spell_and_countered_selected_spell_never_enters(){
 for definition in definitions(false,"You gain 1 life."){
  let(mut normal,source)=setup(&definition,Zone::Hand);mana(&mut normal,1);cast(&mut normal,source);let stable=normal.object(normal.stack[0].object_id).unwrap().stable_id;ironsmith::game_loop::resolve_stack_entry_with(&mut normal,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(normal.object(normal.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);assert!(normal.battlefield.is_empty());assert_eq!(normal.player(A).unwrap().life,21);
  let(mut game,source)=setup(&definition,Zone::Graveyard);mana(&mut game,2);cast(&mut game,source);let stack=game.stack[0].object_id;let stable=game.object(stack).unwrap().stable_id;execute_effect(&mut game,&Effect::new(ironsmith::effects::CounterEffect::new(ChooseSpec::SpecificObject(stack))),&mut EffectContext::new_default(stack,A)).unwrap();assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert!(game.battlefield.is_empty());assert_eq!(game.player(A).unwrap().life,20);
 }
}
#[test]
fn all_illegal_targets_prevent_entry_and_copies_resolve_effects_without_becoming_tokens(){
 for definition in definitions(false,"Destroy target creature."){
  let(mut game,source)=setup(&definition,Zone::Graveyard);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Only target","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let target=game.create_object_from_definition(&body,A,Zone::Battlefield);mana(&mut game,2);cast(&mut game,source);let stable=game.object(game.stack[0].object_id).unwrap().stable_id;move_to(&mut game,target,Zone::Exile);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert!(game.battlefield.is_empty());
 }
 for definition in definitions(true,"You gain 1 life."){
  let(mut game,source)=setup(&definition,Zone::Graveyard);mana(&mut game,2);cast(&mut game,source);let original=game.stack[0].object_id;
  execute_effect(&mut game,&Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(original))),&mut EffectContext::new_default(original,A)).unwrap();assert_eq!(game.stack.len(),2);let copy=game.stack.last().unwrap().object_id;assert_eq!(game.object(copy).unwrap().kind,ironsmith::object::ObjectKind::SpellCopy);
  let mut external=game.clone();execute_effect(&mut external,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(copy),Zone::Battlefield,false)),&mut EffectContext::new_default(copy,A)).unwrap();assert!(external.battlefield.is_empty(),"external effects cannot put the copied spell onto battlefield either");
  let mut changed=game.clone();let new_face=ironsmith_compiler_runtime::compile_to_runtime_definition("Changed copied creature","Type: Creature\nPower/Toughness: 1/1",false).unwrap();changed.object_mut(copy).unwrap().apply_definition_face(&new_face);execute_effect(&mut changed,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(copy),Zone::Battlefield,false)),&mut EffectContext::new_default(copy,A)).unwrap();assert!(changed.battlefield.is_empty(),"copy provenance survives a changed presentation");
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().life,21);assert!(game.battlefield.is_empty(),"copy resolves effects but cannot become token");ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().life,22);assert_eq!(game.battlefield.len(),1);
 }
}
#[test]
fn all_sixty_one_source_coalesce_programs_round_trip(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/coalesce-composed-resolution.json")).unwrap();let mut count=0;let mut failures=Vec::new();
 for row in fixture["mechanics"].as_array().unwrap(){
  for card in row["representative_cards"].as_array().unwrap(){
   assert_eq!(card["rewrite_status"],"frontend_materialized_unverified");
   for face in card["executable_faces"].as_array().unwrap(){
    let name=face["name"].as_str().unwrap();
    match source_support::definitions(face) {
     Ok([direct,indirect]) => if ironsmith_text::compiled_text_lines(&direct)!=ironsmith_text::compiled_text_lines(&indirect) { failures.push(format!("{name}: native/artifact output differs")); },
     Err(error) => failures.push(format!("{name}: {error}")),
    }
    count+=1;
   }
  }
 }
 assert_eq!(count,61);assert!(failures.is_empty(),"All source failures:\n{}",failures.join("\n"));
}
#[test]
fn copied_departed_spell_uses_exact_selected_price_lki_and_still_cannot_enter(){
 for definition in definitions(false,"You gain 1 life."){
  let(mut game,source)=setup(&definition,Zone::Graveyard);mana(&mut game,2);cast(&mut game,source);let original=game.stack[0].object_id;let saved=game.stack[0].clone();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::CounterEffect::new(ChooseSpec::SpecificObject(original))),&mut EffectContext::new_default(original,A)).unwrap();
  let mut context=EffectContext::new_default(original,A).with_casting_method(saved.casting_method.clone());context.source_snapshot=saved.source_snapshot.clone();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::Source)),&mut context).unwrap();assert_eq!(game.stack.len(),1,"copy uses departed spell's exact LKI");
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().life,21);assert!(game.battlefield.is_empty(),"departed selected-price spell copy cannot become a token");
 }
}

#[test]
fn source_possessive_and_mana_spent_value_survive_both_coalesce_routes() {
 for definition in definitions(false, "Physical resolution witness's power and toughness are each equal to your life total.") {
  let (mut game, source) = setup(&definition, Zone::Graveyard);
  game.player_mut(A).unwrap().life = 13;
  mana(&mut game, 2); cast(&mut game, source);
  let stable = game.object(game.stack[0].object_id).unwrap().stable_id;
  ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
  let permanent = game.find_object_by_stable_id(stable).unwrap();
  assert_eq!(game.current_power(permanent), Some(13));
  assert_eq!(game.current_toughness(permanent), Some(13));
 }
 for definition in definitions(false, "Add X mana in any combination of colors, where X is the amount of mana spent to cast this.") {
  let (mut game, source) = setup(&definition, Zone::Graveyard);
  mana(&mut game, 2); cast(&mut game, source);
  ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(game.battlefield.len(), 1);
  assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
 }
}

#[test]
fn bare_looked_partition_moves_only_selected_viewed_cards_and_exact_remainder() {
 for (body, selected_zone, remainder_zone) in [
  ("Look at the top three cards of your library. Put one into your hand and the rest into your graveyard.", Zone::Hand, Zone::Graveyard),
  ("Look at the top three cards of your library. Put one back and exile the rest.", Zone::Library, Zone::Exile),
 ] {
  for definition in definitions(false, body) {
   let (mut game, source) = setup(&definition, Zone::Graveyard);
   let filler = ironsmith_compiler_runtime::compile_to_runtime_definition("Viewed control", "Type: Artifact", false).unwrap();
   let ids: Vec<_> = (0..4).map(|_| {
    let id=game.create_object_from_definition(&filler,A,Zone::Library);
    game.object(id).unwrap().stable_id
   }).collect();
   let before = game.player(A).unwrap().library.clone();
   let viewed: Vec<_> = before.iter().rev().take(3).map(|id| game.object(*id).unwrap().stable_id).collect();
   let outside = ids.iter().find(|id| !viewed.contains(id)).copied().unwrap();
   mana(&mut game,2);cast(&mut game,source);
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
   let zone_of = |stable| game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone;
   assert_eq!(zone_of(outside),Zone::Library);
   assert_eq!(viewed.iter().filter(|id| zone_of(**id)==selected_zone).count(),1);
   assert_eq!(viewed.iter().filter(|id| zone_of(**id)==remainder_zone).count(),2);
  }
 }
}

#[test]
fn conditional_entry_uses_pre_entry_type_and_shields_only_actual_incarnation() {
 let body="You may put a creature or Revelation card from your hand onto the battlefield. Exile that permanent if it would leave the battlefield. If it's a Revelation, it enters as a Horror creature.";
 for (route,definition) in definitions(false,body).into_iter().enumerate() {
  for revelation in [false,true] {
   let(mut game,source)=setup(&definition,Zone::Graveyard);
   let target_definition=if revelation {definitions(false,"You gain 1 life.")[route].clone()}
    else {ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary selected creature","Type: Creature — Elf\nPower/Toughness: 2/2",false).unwrap()};
   let target=game.create_object_from_definition(&target_definition,A,Zone::Hand);
   let stable=game.object(target).unwrap().stable_id;
   mana(&mut game,2);cast(&mut game,source);
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
   let entered=game.find_object_by_stable_id(stable).unwrap();
   assert_eq!(game.object(entered).unwrap().zone,Zone::Battlefield,
    "conditional described entry route={route} revelation={revelation}; source program: {:?}", definition);
   assert!(game.current_has_card_type(entered,ironsmith::CardType::Creature));
   let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(entered).unwrap(),&game);
   if revelation {
    assert!(snapshot.subtypes.contains(&ironsmith::Subtype::Horror));
    assert!(!snapshot.card_types.contains(&ironsmith::CardType::Sorcery));
    assert_eq!(game.current_power(entered),Some(7));
   } else {
    assert!(snapshot.subtypes.contains(&ironsmith::Subtype::Elf));
    assert!(!snapshot.subtypes.contains(&ironsmith::Subtype::Horror));
   }
   let exiled=move_to(&mut game,entered,Zone::Graveyard);
   assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);
   let hand=move_to(&mut game,exiled,Zone::Hand);
   let later=move_to(&mut game,hand,Zone::Graveyard);
   assert_eq!(game.object(later).unwrap().zone,Zone::Graveyard);
  }
 }
}

#[test]
fn control_loss_prohibition_blocks_acquisition_and_expired_control_reversion_until_source_leaves() {
 const B:PlayerId=PlayerId(1);
 for definition in definitions(false,"As long as Physical resolution witness is on the battlefield, you can't lose control of permanents.") {
  let(mut game,protector)=setup(&definition,Zone::Graveyard);
  let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Borrowed control","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
  let artifact=ironsmith_compiler_runtime::compile_to_runtime_definition("Control duration source","Type: Artifact",false).unwrap();
  let target=game.create_object_from_definition(&creature,B,Zone::Battlefield);
  let duration_source=game.create_object_from_definition(&artifact,A,Zone::Battlefield);
  execute_effect(&mut game,&Effect::new(ironsmith::effects::GainControlEffect::while_source_remains(ChooseSpec::SpecificObject(target))),&mut EffectContext::new_default(duration_source,A)).unwrap();
  assert_eq!(game.current_controller(target),Some(A));
  mana(&mut game,2);cast(&mut game,protector);
  let stable=game.object(game.stack[0].object_id).unwrap().stable_id;
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  let protector=game.find_object_by_stable_id(stable).unwrap();
  let opposing_source=game.create_object_from_definition(&artifact,B,Zone::Battlefield);
  execute_effect(&mut game,&Effect::new(ironsmith::effects::GainControlEffect::permanent(ChooseSpec::SpecificObject(target))),&mut EffectContext::new_default(opposing_source,B)).unwrap();
  assert_eq!(game.current_controller(target),Some(A));
  move_to(&mut game,duration_source,Zone::Graveyard);
  assert_eq!(game.current_controller(target),Some(A),"the accepted controller survives expiry while protected");
  move_to(&mut game,protector,Zone::Exile);
  assert_eq!(game.current_controller(target),Some(B),"ending protection releases the retained controller");
 }
}

#[test]
fn chosen_milled_subset_cast_and_hand_remainder_preserve_all_three_choice_outcomes() {
 struct MilledDecisions { choose:bool,cast:bool }
 impl ironsmith::decision::DecisionMaker for MilledDecisions {
  fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool {ctx.can_accept}
  fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {
   if (ctx.max==Some(2) && !self.choose) || (ctx.max==Some(1) && !self.cast) {return Vec::new();}
   ironsmith::decision::DecisionMaker::decide_objects(&mut SelectFirstDecisionMaker,game,ctx)
  }
 }
 let body="Mill five cards. You may choose up to two nonland cards with mana value 2 or less milled this way. You may cast one without paying its mana cost. Put the rest into your hand.";
 for definition in definitions(false,body) {
  for (choose,cast_one) in [(false,false),(true,false),(true,true)] {
   let(mut game,source)=setup(&definition,Zone::Graveyard);
   let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Milled control","Type: Artifact",false).unwrap();
   let ids:Vec<_>=(0..6).map(|_| {
    let id=game.create_object_from_definition(&filler,A,Zone::Library);
    game.object(id).unwrap().stable_id
   }).collect();
   let milled:Vec<_>=game.player(A).unwrap().library.iter().rev().take(5).map(|id|game.object(*id).unwrap().stable_id).collect();
   let outside=ids.iter().find(|id|!milled.contains(id)).copied().unwrap();
   mana(&mut game,2);cast(&mut game,source);
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut MilledDecisions {choose,cast:cast_one}).unwrap();
   let zone_of=|stable|game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone;
   let selected=if choose {2}else{0};let cast_count=usize::from(cast_one);
   assert_eq!(milled.iter().filter(|id|zone_of(**id)==Zone::Hand).count(),selected-cast_count);
   assert_eq!(milled.iter().filter(|id|zone_of(**id)==Zone::Graveyard).count(),5-selected);
   assert_eq!(milled.iter().filter(|id|zone_of(**id)==Zone::Stack).count(),cast_count);
   assert_eq!(zone_of(outside),Zone::Library);
   assert_eq!(game.stack.len(),cast_count);
  }
 }
}

#[test]
fn untap_cap_is_opponent_only_and_ends_with_its_source() {
 const B:PlayerId=PlayerId(1);
 let body="Opponents can't untap more than two permanents during their untap steps while Physical resolution witness is on the battlefield.";
 for definition in definitions(false,body) {
  let(mut game,source)=setup(&definition,Zone::Graveyard);
  mana(&mut game,2);cast(&mut game,source);
  let stable=game.object(game.stack[0].object_id).unwrap().stable_id;
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  let source=game.find_object_by_stable_id(stable).unwrap();
  let artifact=ironsmith_compiler_runtime::compile_to_runtime_definition("Untap cap control","Type: Artifact",false).unwrap();
  let a:Vec<_>=(0..3).map(|_|game.create_object_from_definition(&artifact,A,Zone::Battlefield)).collect();
  let b:Vec<_>=(0..3).map(|_|game.create_object_from_definition(&artifact,B,Zone::Battlefield)).collect();
  for id in a.iter().chain(b.iter()) {game.tap(*id);}
  game.turn.active_player=B;game.turn.phase=ironsmith::Phase::Beginning;game.turn.step=Some(ironsmith::Step::Untap);
  ironsmith::turn::execute_untap_step_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(b.iter().filter(|id|!game.is_tapped(**id)).count(),2);
  game.turn.active_player=A;
  ironsmith::turn::execute_untap_step_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  assert!(a.iter().all(|id|!game.is_tapped(*id)));
  move_to(&mut game,source,Zone::Exile);
  for id in &b {game.tap(*id);}
  game.turn.active_player=B;
  ironsmith::turn::execute_untap_step_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  assert!(b.iter().all(|id|!game.is_tapped(*id)));
 }
}

#[test]
fn source_subtype_addition_functions_only_while_the_card_is_a_creature() {
 for definition in definitions(false,"Physical resolution witness is an Illusion in addition to its other types while it's a creature.") {
  let(mut game,source)=setup(&definition,Zone::Graveyard);
  assert!(!game.current_subtypes(source).unwrap_or_default().contains(&ironsmith::Subtype::Illusion));
  mana(&mut game,2);cast(&mut game,source);
  let stable=game.object(game.stack[0].object_id).unwrap().stable_id;
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  let source=game.find_object_by_stable_id(stable).unwrap();
  assert!(game.current_subtypes(source).unwrap_or_default().contains(&ironsmith::Subtype::Illusion));
  let exiled=move_to(&mut game,source,Zone::Exile);
  assert!(!game.current_subtypes(exiled).unwrap_or_default().contains(&ironsmith::Subtype::Illusion));
 }
}

#[test]
fn filtered_shuffle_union_preserves_excluded_cards_in_each_players_graveyard() {
 const B:PlayerId=PlayerId(1);
 let body="Each player shuffles their hand and each non-Revelation card from their graveyard into their library, then draws seven cards.";
 for (route,definition) in definitions(false,body).into_iter().enumerate() {
  let(mut game,source)=setup(&definition,Zone::Graveyard);
  let ordinary=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary shuffle control","Type: Artifact",false).unwrap();
  let revelation=definitions(false,"You gain 1 life.")[route].clone();
  let mut excluded=Vec::new();let mut included=Vec::new();
  for owner in [A,B] {
   let id=game.create_object_from_definition(&revelation,owner,Zone::Graveyard);excluded.push(game.object(id).unwrap().stable_id);
   for zone in [Zone::Graveyard,Zone::Hand] {let id=game.create_object_from_definition(&ordinary,owner,zone);included.push(game.object(id).unwrap().stable_id);}
   for _ in 0..7 {game.create_object_from_definition(&ordinary,owner,Zone::Library);}
  }
  mana(&mut game,2);cast(&mut game,source);
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  for stable in excluded {assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);}
  for stable in included {assert!(matches!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Library|Zone::Hand));}
  assert_eq!(game.player(A).unwrap().hand.len(),7);assert_eq!(game.player(B).unwrap().hand.len(),7);
 }
}
