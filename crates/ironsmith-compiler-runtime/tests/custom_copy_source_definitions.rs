//! Exact source-owned copy substitution, independent native and artifact routes.
mod custom_final_next_wave_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{SelectFirstDecisionMaker,LegalAction};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(source:&serde_json::Value)->[CardDefinition;2]{custom_final_next_wave_support::definitions(source).unwrap_or_else(|error|panic!("{}: {error}",source["name"]))}
fn simple(name:&str,text:&str)->[CardDefinition;2]{definitions(&serde_json::json!({"name":name,"executable_text":text}))}
fn actual_source()->serde_json::Value{serde_json::from_str::<serde_json::Value>(include_str!("../../../fixtures/custom_mechanics/msem-repository-warden-copy-source.json")).unwrap()["mechanics"][0]["representative_cards"][0].clone()}
fn provider(name:&str,definition_name:&str,text:&str)->serde_json::Value{serde_json::json!({
 "name":name,"executable_text":format!("Type: Enchantment\nIf a player would copy a spell or cast a copy, they copy the card named {definition_name} instead of anything else."),
 "supplied_card_definitions":[{"name":definition_name,"text":text,"customKeywordIdentities":[],"customKeywordPayments":[],"customSubtypes":[]}]
})}
fn stack_spell(game:&mut GameState,definition:&CardDefinition,player:PlayerId)->ObjectId{
 let id=game.create_object_from_definition(definition,player,Zone::Stack);game.stack.push(ironsmith::game_state::StackEntry::new(id,player));id
}
fn library(game:&mut GameState,player:PlayerId,count:usize){let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent library card","Type: Artifact",false).unwrap();for _ in 0..count{game.create_object_from_definition(&definition,player,Zone::Library);}}
fn cast_action(game:&GameState,donor:ObjectId)->LegalAction{ironsmith::decision::compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,from_zone:Zone::Graveyard,..} if *spell_id==donor)).expect("original donor permission is legally available")}
fn announce(game:&mut GameState,action:LegalAction){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){return;}let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&context,&mut dm).unwrap();}panic!("unfinished cast");
}
#[test]
fn exact_repository_warden_replaces_definition_not_copy_actor_count_or_original_program(){
 let original=simple("Independent original","Mana cost: {5}\nType: Instant\nDraw seven cards.");
 for (route,warden) in definitions(&actual_source()).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&warden,A,Zone::Battlefield);let original=stack_spell(&mut game,&original[route],A);library(&mut game,B,8);
  let mut copy=ironsmith::effects::CopySpellEffect::new(ChooseSpec::SpecificObject(original),2);copy.copier=ironsmith::target::PlayerFilter::Specific(B);
  let result=execute_effect(&mut game,&ironsmith::Effect::new(copy),&mut EffectContext::new_default(source,A)).unwrap();
  let copies=result.output_objects();assert_eq!(copies.len(),2);assert_eq!(game.stack.len(),3);
  for id in copies{let object=game.object(*id).unwrap();assert_eq!(object.name.as_str(),"Data-Point");assert_eq!(object.kind,ironsmith::object::ObjectKind::SpellCopy);assert_eq!(object.controller,B);assert_eq!(object.mana_cost.as_deref().unwrap().mana_value(),2);}
  for _ in 0..2{ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  assert_eq!(game.player(B).unwrap().hand.len(),2,"each supplied copy executes Data-Point's one-card program");assert!(game.player(A).unwrap().hand.is_empty());assert_eq!(game.stack.len(),1);assert_eq!(game.stack[0].object_id,original);assert_eq!(game.object(original).unwrap().name.as_str(),"Independent original");
 }
}
#[test]
fn replacement_definition_is_generic_and_never_found_in_original_copier_pool(){
 let source=provider("Independent provider","Different identity","Mana cost: {3}\nType: Sorcery\nDraw two cards.");
 let original=simple("Original donor","Type: Instant\nYou gain 9 life.");
 for (route,definition) in definitions(&source).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let provider=game.create_object_from_definition(&definition,A,Zone::Battlefield);let original=stack_spell(&mut game,&original[route],B);library(&mut game,B,4);
  let mut copy=ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(original));copy.copier=ironsmith::target::PlayerFilter::Specific(B);
  let outcome=execute_effect(&mut game,&ironsmith::Effect::new(copy),&mut EffectContext::new_default(provider,A)).unwrap();let id=outcome.output_objects()[0];assert_eq!(game.object(id).unwrap().name.as_str(),"Different identity");ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(B).unwrap().hand.len(),2);assert_eq!(game.player(B).unwrap().life,20);
 }
}
#[test]
fn spell_source_replacement_does_not_replace_ability_copies_or_override_original_copy_prohibition(){
 for warden in definitions(&actual_source()){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let provider=game.create_object_from_definition(&warden,A,Zone::Battlefield);
  let ability=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent ability","Type: Artifact\n{1}: You gain 3 life.",false).unwrap();let source=game.create_object_from_definition(&ability,B,Zone::Battlefield);
  let program=ability.abilities.iter().find_map(|a|if let ironsmith::ability::AbilityKind::Activated(a)=&a.kind{Some(a.effects.clone())}else{None}).unwrap();
  let mut entry=ironsmith::game_state::StackEntry::new(source,B);entry.is_ability=true;entry.ability_effects=Some(program);entry.ability_id=Some(game.new_object_id());game.stack.push(entry);
  let outcome=execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::Source)),&mut EffectContext::new_default(source,B)).unwrap();let id=outcome.output_objects()[0];assert_ne!(game.object(id).unwrap().name.as_str(),"Data-Point");assert!(game.stack.last().unwrap().is_ability);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(B).unwrap().life,23);
  game.stack.clear();let protected=ironsmith_compiler_runtime::compile_to_runtime_definition("Protected original","Type: Instant\nThis spell can't be copied.\nDraw a card.",false).unwrap();let target=stack_spell(&mut game,&protected,A);
  let outcome=execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(target))),&mut EffectContext::new_default(provider,A)).unwrap();assert!(outcome.output_objects().is_empty());assert_eq!(game.stack.len(),1);
 }
}
const FIXED_DONOR:&str="Mana cost: {7}\nType: Creature — Bird\nPower/Toughness: 8/8\nYou may cast a copy of this card from your graveyard by paying {1}{U} and exiling this card from your graveyard rather than paying its mana cost.";
#[test]
fn actual_warden_cast_copy_keeps_original_donor_move_alternative_price_and_origin_receipt(){
 let donors=simple("Independent cast donor",FIXED_DONOR);
 for (route,warden) in definitions(&actual_source()).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.create_object_from_definition(&warden,B,Zone::Battlefield);let donor=game.create_object_from_definition(&donors[route],A,Zone::Graveyard);let stable=game.object(donor).unwrap().stable_id;game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);library(&mut game,A,2);
  let action=cast_action(&game,donor);announce(&mut game,action);assert_eq!(game.stack.len(),1);let copy=game.stack[0].object_id;assert_eq!(game.object(copy).unwrap().name.as_str(),"Data-Point");assert_eq!(game.player(A).unwrap().mana_pool.total(),0);assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);let origin=game.cast_origin_snapshot(copy).unwrap();assert_eq!(origin.object_id,donor);assert_eq!(origin.stable_id,stable);assert_eq!(origin.name,"Independent cast donor");assert_eq!(origin.zone,Zone::Graveyard);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().hand.len(),1);
 }
}
#[test]
fn replacement_card_alternative_at_same_ordinal_does_not_steal_original_permission_price(){
 let source=provider("Independent price provider","Alternative identity","Mana cost: {2}\nType: Sorcery\nDraw a card.\nFlashback {9}");let donors=simple("Independent cast donor",FIXED_DONOR);
 for (route,definition) in definitions(&source).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.create_object_from_definition(&definition,B,Zone::Battlefield);let donor=game.create_object_from_definition(&donors[route],A,Zone::Graveyard);let stable=game.object(donor).unwrap().stable_id;game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  let action=cast_action(&game,donor);announce(&mut game,action);let copy=game.stack[0].object_id;assert_eq!(game.object(copy).unwrap().name.as_str(),"Alternative identity");assert_eq!(game.player(A).unwrap().mana_pool.total(),0);let paid_name=game.stack[0].optional_costs_paid.costs.iter().find_map(|(reference,count)|if *count>0{if let ironsmith_core::OptionalCostKind::AlternativeCast(alt)=&reference.kind{Some(alt.method_name())}else{None}}else{None});assert_eq!(paid_name,Some(donors[route].alternative_casts[0].name()),"the paid method receipt belongs to the donor's selected price");assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
 }
}
#[test]
fn cancelling_a_proposed_replaced_copy_restores_original_donor_and_resources(){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm};let donors=simple("Independent cast donor",FIXED_DONOR);
 for (route,warden) in definitions(&actual_source()).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.create_object_from_definition(&warden,B,Zone::Battlefield);let donor=game.create_object_from_definition(&donors[route],A,Zone::Graveyard);let stable=game.object(donor).unwrap().stable_id;game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  let action=cast_action(&game,donor);let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut SelectFirstDecisionMaker).unwrap();assert!(state.has_pending_action());assert!(state.rollback_action(&mut game));assert!(game.stack.is_empty());let original=game.find_object_by_stable_id(stable).unwrap();assert_eq!(original,donor);assert_eq!(game.object(original).unwrap().zone,Zone::Graveyard);assert_eq!(game.player(A).unwrap().mana_pool.total(),2);
 }
}
#[test]
fn target_requiring_fresh_definition_is_a_checked_boundary_not_inherited_original_targets(){
 let source=provider("Target choice provider","Targeted identity","Type: Instant\nTarget player draws a card.");let donors=simple("Original donor","Type: Instant\nYou gain 3 life.");
 for (route,definition) in definitions(&source).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let provider=game.create_object_from_definition(&definition,A,Zone::Battlefield);let target=stack_spell(&mut game,&donors[route],A);
  let result=execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(target))),&mut EffectContext::new_default(provider,A));assert!(result.is_err());assert_eq!(game.stack.len(),1);assert_eq!(game.stack[0].object_id,target);
 }
}
#[test]
fn original_own_mana_copy_price_reads_original_donor_instead_of_new_definition(){
 let donors=simple("Independent own-price donor","Mana cost: {3}{U}\nType: Creature — Bird\nPower/Toughness: 8/8\nAs long as you have seven or more cards in your graveyard, you may cast a copy of this card from your graveyard by paying this card's mana cost and exiling this card from your graveyard rather than paying its mana cost.");
 let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent graveyard card","Type: Artifact",false).unwrap();
 for (route,warden) in definitions(&actual_source()).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.create_object_from_definition(&warden,B,Zone::Battlefield);let donor=game.create_object_from_definition(&donors[route],A,Zone::Graveyard);let stable=game.object(donor).unwrap().stable_id;for _ in 0..6{game.create_object_from_definition(&filler,A,Zone::Graveyard);}game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,3);
  let action=cast_action(&game,donor);announce(&mut game,action);let copy=game.stack[0].object_id;assert_eq!(game.object(copy).unwrap().name.as_str(),"Data-Point");assert_eq!(game.player(A).unwrap().mana_pool.total(),0,"original {{3}}{{U}} permission price survives replacement by a card with printed cost {{2}}");assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert_eq!(game.cast_origin_snapshot(copy).unwrap().object_id,donor);
 }
}
#[test]
fn effect_created_card_copy_uses_normal_cast_owner_and_preserves_free_price(){
 let donors=simple("Independent copied card","Mana cost: {8}\nType: Sorcery\nAs an additional cost to cast this spell, discard a card.\nDraw seven cards.");
 for (route,warden) in definitions(&actual_source()).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&warden,B,Zone::Battlefield);let donor=game.create_object_from_definition(&donors[route],A,Zone::Graveyard);library(&mut game,A,2);
  let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(donor).unwrap(),&game);
  let mut ctx=EffectContext::new_default(source,A);ctx.set_tagged_objects("saved".into(),vec![snapshot]);
  let effect=ironsmith::effects::CastTaggedEffect::new("saved",ironsmith::target::PlayerFilter::You).as_copy().without_paying_mana_cost();
  let outcome=execute_effect(&mut game,&ironsmith::Effect::new(effect),&mut ctx).unwrap();assert_eq!(outcome.output_objects().len(),1);
  let copy=outcome.output_objects()[0];assert_eq!(game.object(copy).unwrap().name.as_str(),"Data-Point");assert_eq!(game.object(copy).unwrap().kind,ironsmith::object::ObjectKind::SpellCopy);assert_eq!(game.object(donor).unwrap().zone,Zone::Graveyard);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().hand.len(),1);
 }
}
#[test]
fn replacement_spell_timing_is_checked_and_discovery_does_not_guess_one_provider_order(){
 let donors=simple("Independent instant donor","Mana cost: {7}\nType: Instant\nDraw seven cards.\nYou may cast a copy of this card from your graveyard by paying {1}{U} and exiling this card from your graveyard rather than paying its mana cost.");
 let instant=provider("Independent instant provider","Instant identity","Mana cost: {2}\nType: Instant\nDraw two cards.");
 for (route,warden) in definitions(&actual_source()).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=None;game.create_object_from_definition(&warden,B,Zone::Battlefield);let donor=game.create_object_from_definition(&donors[route],A,Zone::Graveyard);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  let available=|game:&GameState|ironsmith::decision::compute_legal_actions(game,A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==donor));
  assert!(!available(&game),"the proposed sorcery cannot inherit the original instant's timing");
  game.create_object_from_definition(&definitions(&instant)[route],B,Zone::Battlefield);
  assert!(available(&game),"at least one legal replacement order yields an instant");assert_eq!(game.object(donor).unwrap().zone,Zone::Graveyard);assert!(game.stack.is_empty());assert_eq!(game.player(A).unwrap().mana_pool.total(),2);
 }
}
#[test]
fn original_own_mana_price_enters_normal_reductions_after_definition_selection(){
 let donors=simple("Independent priced donor","Mana cost: {3}{U}\nType: Creature — Bird\nPower/Toughness: 8/8\nAs long as you have seven or more cards in your graveyard, you may cast a copy of this card from your graveyard by paying this card's mana cost and exiling this card from your graveyard rather than paying its mana cost.");
 let reducer=simple("Independent price reducer","Type: Enchantment\nNoncreature spells you cast cost {1} less to cast.");let filler=simple("Independent graveyard filler","Type: Artifact");
 for (route,warden) in definitions(&actual_source()).into_iter().enumerate(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.create_object_from_definition(&warden,B,Zone::Battlefield);game.create_object_from_definition(&reducer[route],A,Zone::Battlefield);let donor=game.create_object_from_definition(&donors[route],A,Zone::Graveyard);for _ in 0..6{game.create_object_from_definition(&filler[route],A,Zone::Graveyard);}game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);
  let action=cast_action(&game,donor);announce(&mut game,action);assert_eq!(game.object(game.stack[0].object_id).unwrap().name.as_str(),"Data-Point");assert_eq!(game.player(A).unwrap().mana_pool.total(),0,"original raw price is reduced for the selected noncreature spell");
 }
}
#[test]
fn source_replacement_missing_supplied_definition_never_uses_global_card_catalog(){
 let source=serde_json::json!({"name":"Independent missing provider","executable_text":"Type: Enchantment\nIf a player would copy a spell or cast a copy, they copy the card named Lightning Bolt instead of anything else."});
 assert!(custom_final_next_wave_support::definitions(&source).is_err());
}
