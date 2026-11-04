//! Next-wave witnesses; unregistered until parent integrates the scratch patch.
use ironsmith::{CardDefinition,GameState,GameProgress,ObjectId,PlayerId,Zone};
use ironsmith::decision::{compute_legal_actions,LegalAction,SelectFirstDecisionMaker};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,put_triggers_on_stack,resolve_stack_entry_with};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(body:&str)->[CardDefinition;2] {
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Independent battlefield cast",body,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn game()->GameState {let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;g}
fn offered(g:&GameState,source:ObjectId,player:PlayerId)->Vec<LegalAction> {compute_legal_actions(g,player).unwrap().into_iter().filter(|action|matches!(action,LegalAction::CastSpell{spell_id,from_zone:Zone::Battlefield,..} if *spell_id==source)).collect()}
fn cast(g:&mut GameState,action:LegalAction) {
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(g,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64 {
  if !state.has_pending_action(){put_triggers_on_stack(g,&mut queue).unwrap();return;}
  let GameProgress::NeedsDecisionCtx(ctx)=progress else {panic!("{progress:?}");};
  progress=apply_decision_context_with_dm(g,&mut queue,&mut state,&ctx,&mut dm).unwrap();
 }
 panic!("cast did not complete");
}
const PRICE:&str="You may cast this spell from the battlefield for {1}{U} rather than pay its mana cost.";
#[test]
fn battlefield_cast_is_a_real_costed_cast_and_new_incarnation() {
 for definition in definitions(&format!("Type: Creature\nMana cost: {{4}}{{U}}\nPower/Toughness: 2/3\nWhen this creature enters, you gain 2 life.\nWhen this creature leaves the battlefield, you gain 1 life.\n{PRICE}")) {
  let mut g=game();let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);let stable=g.object(source).unwrap().stable_id;
  g.add_counters(source,ironsmith::CounterType::PlusOnePlusOne,2);g.tap(source);
  assert!(offered(&g,source,A).is_empty());g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  let actions=offered(&g,source,A);assert_eq!(actions.len(),1,"live native grant must not produce duplicate prices");
  cast(&mut g,actions[0].clone());let stacked=g.find_object_by_stable_id(stable).unwrap();
  assert_ne!(stacked,source);assert_eq!(g.object(stacked).unwrap().zone,Zone::Stack);assert_eq!(g.counter_count(stacked,ironsmith::CounterType::PlusOnePlusOne),0);assert!(!g.is_tapped(stacked));
  assert_eq!(g.cast_origin_snapshot(stacked).unwrap().zone,Zone::Battlefield);assert!(g.object_completed_cast_from(source,Zone::Battlefield));
  assert_eq!(g.player(A).unwrap().mana_pool.total(),0);
  while !g.stack.is_empty(){resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();}
  let returned=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(returned).unwrap().zone,Zone::Battlefield);assert_ne!(returned,stacked);
  assert_eq!(g.player(A).unwrap().life,23,"one leave trigger and one fresh enter trigger");
 }
}
#[test]
fn permission_obeys_timing_control_and_live_ability_loss() {
 for definition in definitions(&format!("Type: Creature\nMana cost: {{4}}{{U}}\nPower/Toughness: 2/3\n{PRICE}")) {
  let mut g=game();let source=g.create_object_from_definition(&definition,B,Zone::Battlefield);g.set_current_controller(source,A).unwrap();
  for p in [A,B]{g.player_mut(p).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,10);g.player_mut(p).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,10);}
  assert_eq!(offered(&g,source,A).len(),1,"controller may cast an opponent-owned permanent");assert!(offered(&g,source,B).is_empty());
  g.turn.phase=ironsmith::game_state::Phase::Combat;assert!(offered(&g,source,A).is_empty());g.turn.phase=ironsmith::game_state::Phase::FirstMain;
  let abilities=g.object(source).unwrap().abilities.clone();g.object_mut(source).unwrap().abilities=Default::default();g.refresh_continuous_state().unwrap();assert!(offered(&g,source,A).is_empty(),"printed alternative price does not survive losing its live permission ability");
  g.object_mut(source).unwrap().abilities=abilities;g.refresh_continuous_state().unwrap();assert_eq!(offered(&g,source,A).len(),1);
  let hand=g.create_object_from_definition(&definition,A,Zone::Hand);assert!(offered(&g,hand,A).is_empty());
 }
}
#[test]
fn granted_battlefield_price_uses_recipient_scope_and_disappears_with_grantor() {
 let grants=definitions("Type: Enchantment\nNontoken creatures you control have \"You may cast this spell from the battlefield for {2} rather than pay its mana cost.\".");
 for grant in grants {
  let mut g=game();let grantor=g.create_object_from_definition(&grant,A,Zone::Battlefield);
  let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nMana cost: {5}\nPower/Toughness: 2/2",false).unwrap();
  let recipient=g.create_object_from_definition(&creature,A,Zone::Battlefield);let opposing=g.create_object_from_definition(&creature,B,Zone::Battlefield);
  g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);assert_eq!(offered(&g,recipient,A).len(),1);assert!(offered(&g,opposing,A).is_empty());
  g.move_object_by_effect(grantor,Zone::Graveyard).unwrap();assert!(offered(&g,recipient,A).is_empty());
 }
}
#[test]
fn battlefield_price_pays_nonmana_cost_and_preserves_custom_payment_receipt() {
 for mut definition in definitions("Type: Creature\nMana cost: {5}{B}\nPower/Toughness: 2/2\nYou may cast this spell from the battlefield for {B}, Pay 3 life rather than pay its mana cost.") {
  assert!(definition.attribute_custom_keyword_payment("relocate"));
  let mut g=game();let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,1);g.player_mut(A).unwrap().life=2;
  assert!(offered(&g,source,A).is_empty());g.player_mut(A).unwrap().life=20;
  let action=offered(&g,source,A).remove(0);cast(&mut g,action);assert_eq!(g.player(A).unwrap().life,17);
  let spell=g.stack.iter().find(|entry|!entry.is_ability).unwrap();assert!(spell.optional_costs_paid.paid_custom_keyword("relocate"));assert!(!spell.optional_costs_paid.paid_custom_keyword("unrelated"));
  assert!(g.object_completed_cast_from(source,Zone::Battlefield));
 }
}
#[test]
fn a_granted_custom_price_records_the_granted_designation_on_its_recipient_cast() {
 for grant in definitions("Type: Enchantment\nCreatures you control have custom keyword \"relocate\", and \"You may cast this spell from the battlefield for {2} rather than pay its mana cost.\".") {
  let mut g=game();g.create_object_from_definition(&grant,A,Zone::Battlefield);
  let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Borrower","Type: Creature\nMana cost: {5}\nPower/Toughness: 2/2",false).unwrap();let recipient=g.create_object_from_definition(&creature,A,Zone::Battlefield);
  g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);let action=offered(&g,recipient,A).remove(0);cast(&mut g,action);
  assert!(g.stack.iter().find(|entry|!entry.is_ability).unwrap().optional_costs_paid.paid_custom_keyword("relocate"));
 }
}
#[test]
fn named_alternative_price_prohibition_blocks_only_its_selected_price() {
 for mut definition in definitions("Type: Creature\nMana cost: {5}\nPower/Toughness: 2/2\nYou may cast this spell from the battlefield for {2} rather than pay its mana cost.") {
  assert!(definition.attribute_custom_keyword_payment("relocate"));
  let mut g=game();let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,10);
  assert_eq!(offered(&g,source,A).len(),1);
  let restriction=ironsmith_compiler_runtime::compile_to_runtime_definition("Price restriction","Type: Enchantment\nCreatures you control can't be cast using \"relocate\".",false).unwrap();
  let restriction=g.create_object_from_definition(&restriction,A,Zone::Battlefield);
  assert!(offered(&g,source,A).is_empty());
  g.move_object_by_effect(restriction,Zone::Graveyard);assert_eq!(offered(&g,source,A).len(),1);
  let other=ironsmith_compiler_runtime::compile_to_runtime_definition("Other restriction","Type: Enchantment\nCreatures you control can't be cast using \"unrelated\".",false).unwrap();g.create_object_from_definition(&other,A,Zone::Battlefield);
  assert_eq!(offered(&g,source,A).len(),1,"an unrelated named price is still permitted");
 }
}
#[test]
fn external_enter_watcher_checks_entering_objects_named_price_receipt() {
 for watcher in definitions("Type: Enchantment\nWhenever a creature you control enters, if it was cast using \"relocate\", you gain 3 life.") {
  for wandered in [false,true] {
   let mut source=ironsmith_compiler_runtime::compile_to_runtime_definition("Returning witness","Type: Creature\nMana cost: {5}\nPower/Toughness: 2/2\nYou may cast this spell from the battlefield for {2} rather than pay its mana cost.",false).unwrap();assert!(source.attribute_custom_keyword_payment("relocate"));
   let mut g=game();g.create_object_from_definition(&watcher,A,Zone::Battlefield);let source=g.create_object_from_definition(&source,A,if wandered{Zone::Battlefield}else{Zone::Hand});g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,10);
   let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==source)).unwrap();cast(&mut g,action);
   while !g.stack.is_empty(){resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();}
   assert_eq!(g.player(A).unwrap().life,if wandered{23}else{20},"watcher is not the caster or entering object");
  }
 }
}
#[test]
fn enchanted_permanent_price_prohibition_tracks_only_current_attachment() {
 for aura in definitions("Type: Enchantment — Aura\nEnchant permanent\nEnchanted permanent can't attack or block.\nEnchanted permanent can't be cast using \"relocate\".") {
  let mut definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Moving witness","Type: Creature\nPower/Toughness: 2/2\nYou may cast this spell from the battlefield for {2} rather than pay its mana cost.",false).unwrap();assert!(definition.attribute_custom_keyword_payment("relocate"));
  let mut g=game();let first=g.create_object_from_definition(&definition,A,Zone::Battlefield);let second=g.create_object_from_definition(&definition,A,Zone::Battlefield);let aura=g.create_object_from_definition(&aura,B,Zone::Battlefield);
  g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,10);
  assert!(g.attach_object_to_target(aura,ironsmith::object::AttachmentTarget::Object(first)));assert!(offered(&g,first,A).is_empty());assert_eq!(offered(&g,second,A).len(),1);
  assert!(g.attach_object_to_target(aura,ironsmith::object::AttachmentTarget::Object(second)));assert_eq!(offered(&g,first,A).len(),1);assert!(offered(&g,second,A).is_empty());
 }
}
