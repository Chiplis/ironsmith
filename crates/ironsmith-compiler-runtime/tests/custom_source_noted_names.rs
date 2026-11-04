//! Exact source-incarnation name notes with independent native and wire routes.
#[path="custom_final_next_wave_support.rs"]mod source_support;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::{ChooseSpec,PlayerFilter};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
const CANONICAL:&str="Type: Enchantment\nWhenever a card participates in the conjure action performed by you, if the triggering card's name isn't noted for this permanent, you may note the triggering card's name for this permanent.\nSpells you cast with a name noted for this permanent cost {1} less to cast.";
struct Accept{yes:bool,prompts:usize}
impl DecisionMaker for Accept{
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.prompts+=1;self.yes}
}
fn game()->GameState{let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game}
fn definitions()->[CardDefinition;2]{source_support::definitions(&serde_json::json!({"name":"Source-owned name memory","executable_text":CANONICAL})).unwrap()}
fn witness(name:&str,cost:ironsmith::mana::ManaCost)->CardDefinition{CardDefinitionBuilder::new(ironsmith::CardId::new(),name).mana_cost(cost).card_types(vec![ironsmith::CardType::Sorcery]).with_spell_effect(vec![Effect::gain_life(3)]).build()}
fn create(game:&mut GameState,definition:&CardDefinition,count:i32,actor:PlayerId)->Vec<ObjectId>{
 let creator=CardDefinitionBuilder::new(ironsmith::CardId::new(),"Creation performer").card_types(vec![ironsmith::CardType::Artifact]).build();
 let source=game.create_object_from_definition(&creator,actor,Zone::Battlefield);
 game.take_pending_trigger_events();
 let tag:ironsmith::TagKey="created-name-witnesses".into();
 let creation=Effect::new(ironsmith_core::CreateCardsEffect{definitions:vec![definition.clone()],definition_scope:None,definition_refs:Vec::new(),
  operation_label:Some("conjure".into()),selection:ironsmith_core::CardCreationSelection::Named(definition.card.name.clone()),count:count.into(),
  player:PlayerFilter::You,zone:Zone::Hand,library_position:ironsmith_core::CardCreationLibraryPosition::Top});
 let effect=Effect::new(ironsmith::effects::NamedActionProgramEffect{label:"conjure".into(),player:PlayerFilter::You,
  effects:vec![Effect::new(ironsmith::effects::TaggedEffect{tag:tag.clone(),effect:Box::new(creation),outcome_only:true,action:None})],
  completion:ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,objects:Some(ChooseSpec::Tagged(tag))});
 let outcome=execute_effect(game,&effect,&mut EffectContext::new_default(source,actor)).unwrap();
 for event in &outcome.events{if event.downcast::<ironsmith::events::NamedActionEvent>().is_some(){game.queue_trigger_event(event.provenance(),event.clone());}}
 game.player(actor).unwrap().hand.iter().copied().filter(|id|game.object(*id).unwrap().card==Some(definition.card.id)).collect()
}
fn queue(game:&mut GameState)->ironsmith::triggers::TriggerQueue{
 let mut queue=ironsmith::triggers::TriggerQueue::new();
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();queue
}
fn settle(game:&mut GameState,accept:&mut Accept)->usize{
 let mut total=0;
 for _ in 0..16{
  let mut pending=queue(game);total+=pending.entries.len();
  ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut pending,accept).unwrap();
  if game.stack_is_empty(){return total;}
  ironsmith::game_loop::resolve_stack_entry_with(game,accept).unwrap();
 }
 panic!("name-note trigger processing did not settle")
}
fn can_cast(game:&GameState,id:ObjectId,player:PlayerId)->bool{compute_legal_actions(game,player).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id))}
fn cast(game:&mut GameState,id:ObjectId){
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id)).unwrap();
 let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut pending=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
 let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(game,&mut pending,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(game,&mut pending,&mut state,&ctx,&mut dm).unwrap();}
 assert!(!state.has_pending_action());ironsmith::game_loop::resolve_stack_entry_with(game,&mut dm).unwrap();
}
// Public selector route for the engine's private filter matcher: the effect
// context's filter frame carries the exact source and linked-ability owner.
fn filter_matches(game:&GameState,filter:&ironsmith::target::ObjectFilter,id:ObjectId,source:ObjectId,owner:ironsmith::linked_exile::LinkedExileOwner)->bool{
 let mut filter=filter.clone();filter.zone=Some(game.object(id).unwrap().zone);
 let ctx=EffectContext::new_default(source,A).with_source_number_owner(Some(owner));
 ironsmith::effects::helpers::resolve_objects_from_spec(game,&ChooseSpec::All(filter),&ctx).unwrap().contains(&id)
}
#[test]
fn source_filter_action_and_condition_compile_on_both_independent_routes(){
 for definition in definitions(){
  assert_eq!(definition.abilities.len(),2);
  assert!(format!("{:?}",definition.abilities).contains("NoteObjectNamesEffect"));
  assert!(format!("{:?}",definition.abilities).contains("name_noted_for"));
 }
}
#[test]
fn accepted_singular_batch_notes_each_name_once_and_decline_does_not_note(){
 for definition in definitions(){for yes in [false,true]{
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=witness("Same generated name",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
  create(&mut game,&card,2,A);
  let mut accept=Accept{yes,prompts:0};assert_eq!(settle(&mut game,&mut accept),2);
  assert_eq!(game.noted_card_names_for_source(source).len(),usize::from(yes));
  assert_eq!(accept.prompts,if yes{1}else{2},"intervening condition rechecks after the first accepted note");
  create(&mut game,&card,1,A);let before=accept.prompts;
  assert_eq!(settle(&mut game,&mut accept),usize::from(!yes));
  assert_eq!(accept.prompts-before,usize::from(!yes));
 }}
}
#[test]
fn source_notes_do_not_copy_to_another_source_and_survive_ability_loss_but_not_a_new_incarnation(){
 for definition in definitions(){
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=witness("Discounted remembered spell",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
  let ids=create(&mut game,&card,1,A);settle(&mut game,&mut Accept{yes:true,prompts:0});
  let another=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let copied=game.object(source).unwrap().clone();game.object_mut(another).unwrap().copy_copiable_values_from(&copied);game.refresh_continuous_state().unwrap();
  assert!(game.noted_card_names_for_source(another).is_empty(),"copying rules does not copy source-owned notes");
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  assert!(can_cast(&game,ids[0],A));
  game.object_mut(source).unwrap().abilities=Vec::new().into();game.refresh_continuous_state().unwrap();
  assert_eq!(game.noted_card_names_for_source(source),vec![card.card.name.clone()]);
  assert!(!can_cast(&game,ids[0],A),"removed discount does not remain active via memory");
  game.object_mut(source).unwrap().abilities=definition.abilities.clone().into();game.refresh_continuous_state().unwrap();
  assert!(can_cast(&game,ids[0],A),"restored ability sees the same incarnation's notes");
  let departed=game.move_object(source,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();
  let returned=game.move_object(departed,Zone::Battlefield,ironsmith::events::EventCause::effect()).unwrap();
  assert_ne!(returned,source);assert!(game.noted_card_names_for_source(returned).is_empty());
  assert!(!can_cast(&game,ids[0],A),"return does not inherit the old object's noted names");
 }
}
#[test]
fn note_uses_current_name_for_same_incarnation_and_exact_departure_lki_after_zone_change(){
 for definition in definitions(){for departed in [false,true]{
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=witness("Original creation name",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
  let id=create(&mut game,&card,1,A)[0];let mut pending=queue(&mut game);assert_eq!(pending.entries.len(),1);
  game.object_mut(id).unwrap().name="Name before departure".into();
  if departed{let new=game.move_object(id,Zone::Graveyard,ironsmith::events::EventCause::effect()).unwrap();game.object_mut(new).unwrap().name="Later incarnation name".into();}
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut pending,&mut Accept{yes:true,prompts:0}).unwrap();
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut Accept{yes:true,prompts:0}).unwrap();
  assert_eq!(game.noted_card_names_for_source(source),vec!["Name before departure".to_owned()]);
 }}
}
#[test]
fn another_performers_action_does_not_note_and_generated_object_departure_does_not_erase_a_note(){
 for definition in definitions(){
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=witness("Persistently remembered spell",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
  create(&mut game,&card,1,B);assert_eq!(settle(&mut game,&mut Accept{yes:true,prompts:0}),0);
  let created=create(&mut game,&card,1,A)[0];settle(&mut game,&mut Accept{yes:true,prompts:0});
  game.move_object(created,Zone::Exile,ironsmith::events::EventCause::effect()).unwrap();assert_eq!(game.noted_card_names_for_source(source).len(),1);
  let matching=game.create_object_from_definition(&card,A,Zone::Hand);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  assert!(can_cast(&game,matching,A));cast(&mut game,matching);assert_eq!(game.player(A).unwrap().life,23);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
 }
}
#[test]
fn actual_neural_dreams_full_source_compiles_and_reduces_a_colored_spell(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-conjure-whole-family-source-inputs.json")).unwrap();
 let source=fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["source_card_key"]=="Neural Dreams_BTB").unwrap();
 assert_eq!(source["mana_cost"],"{G}{U}");assert!(source["original_text"].as_str().unwrap().contains("one mana of any color less"));
 for definition in source_support::definitions(source).unwrap(){
  let mut game=game();let observer=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=witness("Actual source colored witness",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Green]));
  let created=create(&mut game,&card,1,A)[0];settle(&mut game,&mut Accept{yes:true,prompts:0});
  assert_eq!(game.noted_card_names_for_source(observer),vec![card.card.name.clone()]);assert!(can_cast(&game,created,A));
  cast(&mut game,created);assert_eq!(game.player(A).unwrap().life,23);
 }
}

#[test]
fn notes_persist_under_control_change_and_discount_follows_current_controller(){
 for definition in definitions(){
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=witness("Transferred source memory",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
  let a=create(&mut game,&card,1,A)[0];settle(&mut game,&mut Accept{yes:true,prompts:0});
  game.object_mut(source).unwrap().initial_controller=B;game.refresh_continuous_state().unwrap();
  assert_eq!(game.controller_of_id(source),Some(B));assert_eq!(game.noted_card_names_for_source(source).len(),1);
  let b=game.create_object_from_definition(&card,B,Zone::Hand);
  for (player,id,expected) in [(A,a,2),(B,b,1)]{
   let object=game.object(id).unwrap();let cost=ironsmith::decision::calculate_effective_mana_cost(&game,player,object,object.mana_cost.as_ref().unwrap());
   assert_eq!(cost.mana_value(),expected);
  }
 }
}

#[test]
fn generic_note_executor_and_source_filter_use_exact_object_memory_without_an_observer(){
 let mut game=game();let body=CardDefinitionBuilder::new(ironsmith::CardId::new(),"Memory holder").card_types(vec![ironsmith::CardType::Artifact]).build();
 let first=game.create_object_from_definition(&body,A,Zone::Battlefield);let second=game.create_object_from_definition(&body,A,Zone::Battlefield);
 let named=witness("Executor noted name",ironsmith::mana::ManaCost::new());let card=game.create_object_from_definition(&named,A,Zone::Hand);
 let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(card).unwrap(),&game);
 let pair=ironsmith_core::LinkedExilePair{definition:ironsmith_core::LinkedExileDefinition([83;32]),pair:0};
 let owner=ironsmith::linked_exile::LinkedExileOwner{host:first,pair,acquisition:ironsmith::linked_exile::LinkedExileAcquisition::Printed};
 let action=Effect::new(ironsmith_core::NoteObjectNamesEffect{pair:Some(pair),object:ChooseSpec::Tagged("input-name".into()),for_object:ChooseSpec::Source});
 for roundtrip in [false,true]{
  let action=if roundtrip{let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(action.clone()).unwrap();ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap()}else{action.clone()};
  let mut ctx=EffectContext::new_default(first,A).with_source_number_owner(Some(owner.clone()));ctx.tag_object("input-name",snapshot.clone());
  let result=execute_effect(&mut game,&action,&mut ctx).unwrap();assert_eq!(result.as_count(),Some(if roundtrip{0}else{1}));
 }
 let filter=ironsmith::target::ObjectFilter{name_noted_for:Some(Box::new(ChooseSpec::Source)),noted_name_pair:Some(pair),..Default::default()};
 assert!(filter_matches(&game,&filter,card,first,owner));
 assert!(!filter_matches(&game,&filter,card,second,ironsmith::linked_exile::LinkedExileOwner{host:second,pair,acquisition:ironsmith::linked_exile::LinkedExileAcquisition::Printed}));
 let new=game.move_object(first,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();assert_ne!(new,first);
 assert!(game.noted_card_names_for_source(new).is_empty());assert_eq!(game.noted_card_names_for_source(first),vec![named.card.name]);
}

#[test]
fn old_pending_triggers_do_not_rebind_name_memory_to_a_returned_source(){
 for definition in definitions(){
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=witness("Pending source incarnation name",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
  create(&mut game,&card,2,A);let mut pending=queue(&mut game);assert_eq!(pending.entries.len(),2);
  let mut accept=Accept{yes:true,prompts:0};ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut pending,&mut accept).unwrap();
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut accept).unwrap();assert_eq!(accept.prompts,1);
  let departed=game.move_object(source,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();let returned=game.move_object(departed,Zone::Battlefield,ironsmith::events::EventCause::effect()).unwrap();
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut accept).unwrap();
  assert_eq!(accept.prompts,1,"old source LKI still knows the first trigger's note");
  assert!(game.noted_card_names_for_source(returned).is_empty());
  assert_eq!(game.noted_card_names_for_source(source),vec![card.card.name]);
 }
}

#[test]
fn another_link_on_the_same_source_cannot_supply_names_to_the_discount(){
 for definition in definitions(){
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let proper=definition.abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Triggered(trigger)=>trigger.effects.source_number_pair,_=>None}).unwrap();
  let card=witness("Unrelated note identity",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
  let id=game.create_object_from_definition(&card,A,Zone::Hand);
  let unrelated=ironsmith::linked_exile::LinkedExileOwner{host:source,pair:ironsmith_core::LinkedExilePair{pair:proper.pair+1,..proper},acquisition:ironsmith::linked_exile::LinkedExileAcquisition::Printed};
  assert!(game.note_card_name_for_owner(unrelated,card.card.name.clone()));
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  assert!(!can_cast(&game,id,A),"same host alone does not prove a CR 607.2e relationship");
  create(&mut game,&card,1,A);settle(&mut game,&mut Accept{yes:true,prompts:0});
  assert!(can_cast(&game,id,A));
 }
}

#[test]
fn same_pair_separately_acquired_on_the_same_source_has_separate_name_memory(){
 for definition in definitions(){
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let pair=definition.abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Triggered(trigger)=>trigger.effects.source_number_pair,_=>None}).unwrap();
  let card=witness("Acquisition-owned note",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
  let id=game.create_object_from_definition(&card,A,Zone::Hand);
  let original=ironsmith::linked_exile::LinkedExileOwner{host:source,pair,acquisition:ironsmith::linked_exile::LinkedExileAcquisition::Printed};
  let gained=ironsmith::linked_exile::LinkedExileOwner{host:source,pair,acquisition:ironsmith::linked_exile::LinkedExileAcquisition::EffectGroup(ironsmith::continuous::ContinuousEffectGroupId(719))};
  assert!(game.note_card_name_for_owner(gained.clone(),card.card.name.clone()));
  let filter=ironsmith::target::ObjectFilter{name_noted_for:Some(Box::new(ChooseSpec::Source)),noted_name_pair:Some(pair),..Default::default()};
  assert!(!filter_matches(&game,&filter,id,source,original.clone()));
  assert!(filter_matches(&game,&filter,id,source,gained));
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  assert!(!can_cast(&game,id,A),"printed reader cannot use a separately acquired record with the same definition pair");
  assert!(game.note_card_name_for_owner(original,card.card.name.clone()));assert!(can_cast(&game,id,A));
 }
}

#[test]
fn separately_granted_equal_discount_keeps_its_own_acquisition_in_the_cost_calculator(){
 for definition in definitions(){
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=witness("Independent discount acquisitions",ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(3)]));
  let ids=create(&mut game,&card,1,A);settle(&mut game,&mut Accept{yes:true,prompts:0});
  let discount=definition.abilities.iter().find(|ability|matches!(ability.kind,ironsmith::ability::AbilityKind::Static(_))).unwrap().clone();
  let granted=Effect::new(ironsmith::effects::ApplyContinuousEffect::new(ironsmith::continuous::EffectTarget::Specific(source),
   ironsmith::continuous::Modification::AddAbilityGeneric(discount),ironsmith::effect::Until::EndOfTurn));
  execute_effect(&mut game,&granted,&mut EffectContext::new_default(source,A)).unwrap();
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);
  assert!(can_cast(&game,ids[0],A),"the original reader still sees its own notes when another equal reader is granted");
  game.player_mut(A).unwrap().mana_pool=Default::default();game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  assert!(!can_cast(&game,ids[0],A),"gaining a reader does not gain the original reader's notes");
  let chars=game.calculated_characteristics(source).unwrap();
  let owner=chars.abilities.iter().enumerate().find_map(|(slot,ability)|{
   let ironsmith::ability::AbilityKind::Static(rule)=&ability.kind else{return None;};
   let pair=rule.cost_reduction_mana_cost()?.filter.noted_name_pair?;
   let owner=ironsmith::linked_exile::LinkedExileOwner::capture(source,Some(pair),chars.abilities.origin(slot))?;
   (!matches!(owner.acquisition,ironsmith::linked_exile::LinkedExileAcquisition::Printed)).then_some(owner)
  }).unwrap();
  assert!(game.note_card_name_for_owner(owner,card.card.name.clone()));assert!(can_cast(&game,ids[0],A));
 }
}
