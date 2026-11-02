//! Complete Cache source cohort and independent linked-token behavior.
mod custom_final_next_wave_support;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixture()->serde_json::Value{serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-cache-linked.json")).unwrap()}
fn definitions(name:&str)->[CardDefinition;2]{let f=fixture();let source=f["mechanics"].as_array().unwrap().iter().flat_map(|m|m["representative_cards"].as_array().unwrap()).find(|c|c["name"]==name).unwrap();custom_final_next_wave_support::linked_definitions(source).unwrap().remove(0)}
fn ordinary(name:&str,text:&str)->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
fn game()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;g}
struct Choices{all:bool}
impl DecisionMaker for Choices{
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{true}
 fn decide_objects(&mut self,_:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{let n=if self.all{c.max.unwrap_or(c.candidates.len())}else{c.min};c.candidates.iter().filter(|c|c.legal).take(n).map(|c|c.id).collect()}
}
fn flush(game:&mut GameState,dm:&mut impl DecisionMaker)->usize{let mut q=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut q,dm).unwrap();let n=q.entries.len();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut q,dm).unwrap();n}
fn enter(game:&mut GameState,definition:&CardDefinition,dm:&mut impl DecisionMaker)->ObjectId{let hand=game.create_object_from_definition(definition,A,Zone::Hand);let stable=game.object(hand).unwrap().stable_id;let outcome=execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(hand),Zone::Battlefield,false)),&mut EffectContext::new_default(hand,A)).unwrap();for event in outcome.events{game.queue_trigger_event(Default::default(),event);}assert_eq!(flush(game,dm),1);resolve_stack_entry_with(game,dm).unwrap();game.find_object_by_stable_id(stable).unwrap()}
fn memories(game:&GameState)->Vec<ObjectId>{game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name=="Memory").collect()}
fn act(game:&mut GameState,action:LegalAction){let mut q=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;let mut progress=apply_priority_response_with_dm(game,&mut q,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else{panic!("{progress:?}");};progress=apply_decision_context_with_dm(game,&mut q,&mut state,&context,&mut dm).unwrap();}assert!(!state.has_pending_action());if !q.entries.is_empty(){ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut q,&mut dm).unwrap();}}
#[test]
fn all_twenty_nine_exact_cache_source_bodies_compile_on_both_routes(){let f=fixture();let mut keys=std::collections::BTreeSet::new();let mut errors=Vec::new();for row in f["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){assert!(keys.insert(card["source_card_key"].as_str().unwrap().to_string()));let label=format!("{} / {}",row["id"],card["source_card_key"]);match custom_final_next_wave_support::linked_definitions(card){Ok(faces)=>{assert_eq!(faces.len(),1);for definition in faces[0].iter(){assert_eq!(definition.card.name,card["name"].as_str().unwrap());}},Err(e)=>errors.push(format!("{label}: {e}"))}}}assert_eq!(keys.len(),29);assert!(errors.is_empty(),"{} exact-source failures:\n{}",errors.len(),errors.join("\n"));}
#[test]
fn top_card_cache_empty_library_and_independent_player_ownership(){for name in ["Buoyant Datagrazer","Assert Mediator"]{for definition in definitions(name){for present in [false,true]{let mut g=game();let card=ordinary("Cached payload","Type: Instant\nMana cost: {W}\nYou gain 1 life.");for player in [A,B]{if present{g.create_object_from_definition(&card,player,Zone::Library);}}enter(&mut g,&definition,&mut Choices{all:true});let expected=if present{if name=="Assert Mediator"{2}else{1}}else{0};let tokens=memories(&g);assert_eq!(tokens.len(),expected);for token in tokens{let linked=g.get_exiled_with_source_links(token);assert_eq!(linked.len(),1);let card=linked[0];let controller=g.current_controller(token).unwrap();assert!(g.has_custom_status(card,"cached"));assert!(g.can_player_look_at_face_down_exiled_card(card,controller));let other=if controller==A{B}else{A};assert!(!g.can_player_look_at_face_down_exiled_card(card,other));}}}}}
#[test]
fn counted_cache_is_sequential_one_to_one_and_zero_choice_is_no_action(){for (source,watcher) in definitions("Unending Possibility").into_iter().zip(definitions("Memorivore")){for all in [false,true]{let mut g=game();let observer=g.create_object_from_definition(&watcher,A,Zone::Battlefield);let before=g.current_power(observer).unwrap();let card=ordinary("Hand payload","Type: Instant\nMana cost: {0}\nYou gain 1 life.");g.create_object_from_definition(&card,A,Zone::Hand);g.create_object_from_definition(&card,A,Zone::Hand);let mut dm=Choices{all};enter(&mut g,&source,&mut dm);let tokens=memories(&g);assert_eq!(tokens.len(),if all{2}else{0});let members:Vec<_>=tokens.iter().map(|id|{let links=g.get_exiled_with_source_links(*id);assert_eq!(links.len(),1);links[0]}).collect();assert_eq!(members.iter().collect::<std::collections::BTreeSet<_>>().len(),members.len());assert_eq!(flush(&mut g,&mut dm),if all{2}else{0});while !g.stack.is_empty(){resolve_stack_entry_with(&mut g,&mut dm).unwrap();}assert_eq!(g.current_power(observer),Some(before+if all{4}else{0}));}}}
#[test]
fn memory_current_controller_gets_linked_permission_sacrifice_and_normal_price(){for definition in definitions("Buoyant Datagrazer"){let mut g=game();let card=ordinary("White payload","Type: Instant\nMana cost: {W}\nYou gain 1 life.");g.create_object_from_definition(&card,A,Zone::Library);enter(&mut g,&definition,&mut Choices{all:true});let memory=memories(&g)[0];let cached=g.get_exiled_with_source_links(memory)[0];execute_effect(&mut g,&Effect::new(ironsmith::effects::GainControlEffect::permanent(ironsmith::target::ChooseSpec::SpecificObject(memory))),&mut EffectContext::new_default(memory,B)).unwrap();g.refresh_continuous_state().unwrap();assert_eq!(g.current_controller(memory),Some(B));assert!(g.can_player_look_at_face_down_exiled_card(cached,B));assert!(g.can_player_look_at_face_down_exiled_card(cached,A),"CR406.3 earned inspection persists");g.turn.priority_player=Some(B);g.player_mut(B).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);let action=compute_legal_actions(&g,B).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==memory)).unwrap();act(&mut g,action);assert!(!g.battlefield.contains(&memory));assert_eq!(g.player(B).unwrap().mana_pool.total(),0);resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert!(!compute_legal_actions(&g,B).unwrap().iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==cached)),"Memory does not waive the card's mana cost");g.player_mut(B).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::White,1);let action=compute_legal_actions(&g,B).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==cached)).unwrap();act(&mut g,action);let spell=g.stack.last().unwrap().object_id;assert!(g.has_custom_status(spell,"cached"));resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(g.player(B).unwrap().life,21);assert_eq!(g.player(A).unwrap().life,20);}}

#[test]
fn olis_defers_cache_until_successful_resolution_and_never_caches_a_countered_spell(){
 for definition in definitions("Olis Mars, Final Archivist"){for countered in [false,true]{
  let mut g=game();g.create_object_from_definition(&definition,A,Zone::Battlefield);
  let payload=ordinary("Hand resolution","Type: Instant\nMana cost: {0}\nYou gain 1 life.");
  let hand=g.create_object_from_definition(&payload,A,Zone::Hand);
  let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==hand)).unwrap();act(&mut g,action);
  assert_eq!(g.stack.len(),2,"one cast and its ordinary source trigger");let spell=g.stack.first().unwrap().object_id;let stable=g.object(spell).unwrap().stable_id;
  resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert!(memories(&g).is_empty(),"trigger resolution only installs the future program");
  if countered{execute_effect(&mut g,&Effect::new(ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell))),&mut EffectContext::new_default(spell,B)).unwrap();}
  else{resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();}
  assert_eq!(memories(&g).len(),usize::from(!countered));let result=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(result).unwrap().zone,if countered{Zone::Graveyard}else{Zone::Exile});assert_eq!(g.player(A).unwrap().life,if countered{20}else{21});
  if countered{
   // End the new cast's own source trigger, so the old registration is the
   // only possible source of an incorrectly revived replacement.
   let hand=g.move_object(result,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();g.turn.priority_player=Some(A);
   let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==hand)).unwrap();act(&mut g,action);assert_eq!(g.stack.len(),2);
   let trigger=g.stack.last().unwrap().object_id;execute_effect(&mut g,&Effect::new(ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(trigger))),&mut EffectContext::new_default(trigger,B)).unwrap();resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert!(memories(&g).is_empty(),"old exact stack incarnation cannot cache a later cast");
  }
 }}
}

#[test]
fn cached_card_inspection_tracks_memory_control_after_its_abilities_are_removed(){
 for definition in definitions("Buoyant Datagrazer"){
  let mut g=game();let card=ordinary("Private payload","Type: Instant\nMana cost: {0}\nYou gain 1 life.");g.create_object_from_definition(&card,A,Zone::Library);enter(&mut g,&definition,&mut Choices{all:true});let memory=memories(&g)[0];let cached=g.get_exiled_with_source_links(memory)[0];
  let removal=ordinary("Remove abilities","Type: Instant\nTarget permanent loses all abilities until end of turn.");let source=g.create_object_from_definition(&removal,A,Zone::Stack);let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(memory)]);for effect in removal.spell_effect.as_ref().unwrap().all_effects(){execute_effect(&mut g,effect,&mut ctx).unwrap();}drop(ctx);g.remove_object(source);
  execute_effect(&mut g,&Effect::new(ironsmith::effects::GainControlEffect::permanent(ironsmith::target::ChooseSpec::SpecificObject(memory))),&mut EffectContext::new_default(memory,B)).unwrap();g.refresh_continuous_state().unwrap();g.turn.priority_player=Some(B);g.player_mut(B).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);
  assert!(!compute_legal_actions(&g,B).unwrap().iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==memory)),"the token really lost its activation");
  assert!(g.can_player_look_at_face_down_exiled_card(cached,B),"the exiled card's permission is independent of the token's abilities");g.remove_object(memory);assert!(g.can_player_look_at_face_down_exiled_card(cached,B),"inspection entitlement earned without looking persists under406.3");
 }
}
#[test]
fn several_created_sources_independently_supply_controller_inspection(){
 let mut g=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let c=PlayerId(2);let definition=ordinary("Inspection host","Type: Artifact");let first=g.create_object_from_definition(&definition,A,Zone::Battlefield);let second=g.create_object_from_definition(&definition,A,Zone::Battlefield);let card=g.create_object_from_definition(&definition,A,Zone::Exile);g.set_face_down(card);
 for source in [first,second]{execute_effect(&mut g,&Effect::new(ironsmith_core::GrantExileControllerInspectionEffect{target:ironsmith::target::ChooseSpec::SpecificObject(card),sources:ironsmith::target::ChooseSpec::SpecificObject(source)}),&mut EffectContext::new_default(source,A)).unwrap();}
 for (source,player) in [(first,B),(second,c)]{execute_effect(&mut g,&Effect::new(ironsmith::effects::GainControlEffect::permanent(ironsmith::target::ChooseSpec::SpecificObject(source))),&mut EffectContext::new_default(source,player)).unwrap();g.refresh_continuous_state().unwrap();assert!(g.can_player_look_at_face_down_exiled_card(card,player));}
 assert!(g.can_player_look_at_face_down_exiled_card(card,A));let hand=g.move_object(card,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();let returned=g.move_object(hand,Zone::Exile,ironsmith::events::EventCause::effect()).unwrap();g.set_face_down(returned);assert!(!g.can_player_look_at_face_down_exiled_card(returned,B));assert!(!g.can_player_look_at_face_down_exiled_card(returned,c));
}

#[test]
fn token_with_its_own_exile_family_cannot_borrow_its_creators_unrelated_card(){
 let raw:Vec<serde_json::Value>=serde_json::from_str(include_str!("../../../fixtures/linked_exile_static_permissions.json.fixture")).unwrap();let oracle=raw.iter().find(|row|row["name"]=="Bane Alley Broker").unwrap()["oracle_text"].as_str().unwrap().replace("Bane Alley Broker","Observer");
 let input=serde_json::json!({"name":"Creator","executable_text":"Type: Sorcery\nExile the top card of your library. Create a 2/2 colorless creature token named Observer.","custom_token_definitions":[{"name":"Observer","token":true,"executable_text":format!("Type: Creature — Human Rogue\nPower/Toughness: 2/2\n{oracle}")}]});
 for definition in custom_final_next_wave_support::definitions(&input).unwrap(){
  let mut g=game();let payload=ordinary("Unrelated exile","Type: Instant\nYou gain 1 life.");g.create_object_from_definition(&payload,A,Zone::Library);let source=g.create_object_from_definition(&definition,A,Zone::Stack);let mut ctx=EffectContext::new_default(source,A);for effect in definition.spell_effect.as_ref().unwrap().all_effects(){execute_effect(&mut g,effect,&mut ctx).unwrap();}drop(ctx);let token=*g.battlefield.iter().find(|id|g.object(**id).unwrap().name=="Observer").unwrap();assert_eq!(g.get_exiled_with_source_links(source).len(),1);assert!(g.get_exiled_with_source_links(token).is_empty(),"an internal token producer must retain its own acquisition");
  assert!(g.object(token).unwrap().abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability)if ability.source_exiled_inspection_pair().flatten().is_some())));
 }
}
