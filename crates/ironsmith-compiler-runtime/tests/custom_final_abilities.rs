//! Exact-source candidates. Parent owns execution and registration.
use ironsmith::{CardDefinition,Effect,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn definitions(id:&str)->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-abilities.json")).unwrap();
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|r|r["id"]==id).unwrap();let card=&row["representative_cards"][0];
 let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),card["name"].as_str().unwrap()).with_custom_keyword(row["keyword"].as_str().unwrap());
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,card["executable_text"].as_str().unwrap(),false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(decoded,artifact);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn disgraced_entry_and_only_own_combat_damage_to_player_remove_all_negative_counters(){
 use ironsmith::object::CounterType;
 for definition in definitions("msem-883"){for kind in ["combat-player","noncombat-player","combat-creature","other-source"]{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let hand=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(hand).unwrap().stable_id;
 execute_effect(&mut game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(hand),Zone::Battlefield,false)),&mut EffectContext::new_default(hand,A)).unwrap();
 let source=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.counter_count(source,CounterType::MinusOneMinusOne),1);
 let other=game.create_object_from_definition(&definition,A,Zone::Battlefield);
 // Counters accumulated after entry must also be removed by the trigger.
 game.object_mut(source).unwrap().counters.insert(CounterType::MinusOneMinusOne,3);
 let combat=kind!="noncombat-player";let target=if kind=="combat-creature"{ironsmith::events::DamageTarget::Object(other)}else{ironsmith::events::DamageTarget::Player(PlayerId(1))};
 let dealer=if kind=="other-source"{other}else{source};
 game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::DamageEvent::with_cause(dealer,target,1,combat,if combat{ironsmith::events::EventCause::combat_damage(dealer)}else{ironsmith::events::EventCause::effect()}),Default::default()));
 let mut q=ironsmith::triggers::TriggerQueue::new();let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut q,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut q,&mut dm).unwrap();
 while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
 assert_eq!(game.counter_count(source,CounterType::MinusOneMinusOne),if kind=="combat-player"{0}else{3},"{kind}");
 }}
}
#[test]
fn remaining_disgraced_source_variants_and_memory_compile_without_loss(){for id in ["msem-884","msem-886","msem-887","msem-917"]{let _=definitions(id);}}
#[test]
fn surpass_reduces_only_its_generic_price_for_all_counter_kinds_and_transforms_at_sorcery_speed(){
 use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
 use ironsmith::mana::ManaSymbol;
 use ironsmith::object::CounterType;
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-abilities.json")).unwrap();
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|r|r["id"]=="msem-906").unwrap();let face=&row["representative_cards"][0]["executable_faces"][1];
 for mut front in definitions("msem-906"){
 let mut back=ironsmith_compiler_runtime::compile_to_runtime_definition(face["name"].as_str().unwrap(),face["executable_text"].as_str().unwrap(),false).unwrap();
 front.card.other_face=Some(back.card.id);front.card.other_face_name=Some(back.card.name.clone());front.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::TransformLike;front.card.transforming_dfc=true;
 back.card.other_face=Some(front.card.id);back.card.other_face_name=Some(front.card.name.clone());back.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::TransformLike;back.card.transforming_dfc=true;
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
 game.register_linked_face_definition(&front);game.register_linked_face_definition(&back);let source=game.create_object_from_definition(&front,A,Zone::Battlefield);
 game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,1);game.object_mut(source).unwrap().counters.insert(CounterType::Charge,2);
 game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue,1);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);
 let action=|g:&GameState|compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source));
 assert!(action(&game).is_none(),"one generic mana short after all three counters");game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);
 game.turn.phase=ironsmith::Phase::Combat;assert!(action(&game).is_none());game.turn.phase=ironsmith::Phase::FirstMain;
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let chosen=action(&game).unwrap();let mut q=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(&mut game,&mut q,&mut state,&PriorityResponse::PriorityAction(chosen),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut q,&mut state,&ctx,&mut dm).unwrap();}
 assert!(!state.has_pending_action());assert_eq!(game.player(A).unwrap().mana_pool.total(),0);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.object(source).unwrap().name,back.card.name);
 assert_eq!(game.counter_count(source,CounterType::Charge),2,"transform preserves counters");
 }
}
#[test]
fn disgraced_token_grant_applies_entry_replacement_to_every_recipient(){
 for definition in definitions("msem-885"){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);
 let mut ctx=EffectContext::new_default(source,A);
 for effect in definition.spell_effect.as_ref().unwrap(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
 let tokens:Vec<_>=game.battlefield.iter().filter_map(|id|game.object(*id)).collect();assert_eq!(tokens.len(),3);
 for token in tokens {assert_eq!(game.counter_count(token.id,ironsmith::object::CounterType::MinusOneMinusOne),1);assert_eq!(game.calculated_toughness(token.id),Some(1));}
 }
}
#[test]
fn memory_requires_complete_graveyard_price_and_exiles_after_resolving(){
 use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};use ironsmith::mana::ManaSymbol;
 for definition in definitions("msem-917"){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
 let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);let stable=game.object(source).unwrap().stable_id;
 let action=|g:&GameState|compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source));
 game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue,1);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red,1);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,3);
 assert!(action(&game).is_none(),"mana cannot replace the additional discard");
 let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Memory payment","Type: Basic Land — Island",false).unwrap();
 game.create_object_from_definition(&filler,A,Zone::Hand);
 for _ in 0..6{game.create_object_from_definition(&filler,A,Zone::Graveyard);}for _ in 0..4{game.create_object_from_definition(&filler,A,Zone::Library);}
 let chosen=action(&game).unwrap();use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let mut q=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(&mut game,&mut q,&mut state,&PriorityResponse::PriorityAction(chosen),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut q,&mut state,&ctx,&mut dm).unwrap();}
 assert!(!state.has_pending_action());assert_eq!(game.player(A).unwrap().mana_pool.total(),0);assert!(game.player(A).unwrap().hand.is_empty(),"payment discarded before resolution");
 ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();let exiled=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);assert_eq!(game.player(A).unwrap().hand.len(),2);
 }
}
#[test]
fn fleeting_selected_price_schedules_sacrifice_but_normal_price_does_not(){
 use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};use ironsmith::mana::ManaSymbol;
 for definition in definitions("msem-888"){for fleeting in [false,true]{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
 let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
 let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Fleeting recipient","Type: Creature — Bird\nPower/Toughness: 2/2",false).unwrap();game.create_object_from_definition(&body,A,Zone::Battlefield);
 game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue,1);if !fleeting{game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,3);}
 let chosen=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source && (fleeting || matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Normal)))).unwrap();
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};let mut q=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(&mut game,&mut q,&mut state,&PriorityResponse::PriorityAction(chosen),&mut dm).unwrap();for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut q,&mut state,&ctx,&mut dm).unwrap();}
 assert!(!state.has_pending_action());ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);assert_eq!(game.effect_store.delayed_triggers.len(),usize::from(fleeting));
 let event=ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfEndStepEvent::new(PlayerId(1)),Default::default());let mut q=ironsmith::triggers::TriggerQueue::new();for trigger in ironsmith::triggers::check_delayed_triggers(&mut game,&event){q.add(trigger);}ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut q,&mut dm).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
 let final_id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(final_id).unwrap().zone,if fleeting{Zone::Graveyard}else{Zone::Battlefield});
 }}
}
struct BoundedSacrifice{count:usize,seen:bool}
impl ironsmith::decision::DecisionMaker for BoundedSacrifice{
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{true}
 fn decide_objects(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{self.seen=true;assert_eq!(ctx.max,Some(1),"Voracious1 preserves maximum");ctx.candidates.iter().filter(|c|c.legal).take(self.count).map(|c|c.id).collect()}
}
#[test]
fn voracious_bounds_optional_artifact_sacrifices_and_counts_only_chosen_permanents(){
 for definition in definitions("msem-890"){for count in [0usize,1]{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Artifact fodder","Type: Artifact",false).unwrap();
 let a=game.create_object_from_definition(&body,A,Zone::Battlefield);let b=game.create_object_from_definition(&body,A,Zone::Battlefield);let enemy=game.create_object_from_definition(&body,PlayerId(1),Zone::Battlefield);
 let hand=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=BoundedSacrifice{count,seen:false};let receipt=game.move_object_with_etb_processing_with_dm(hand,Zone::Battlefield,&mut dm).unwrap();let entered=receipt.original.into_result().unwrap().new_id;
 assert!(dm.seen);assert_eq!(game.counter_count(entered,ironsmith::object::CounterType::PlusOnePlusOne),count as u32);assert_eq!(game.calculated_toughness(entered),Some(1+count as i32));
 assert_eq!([a,b].into_iter().filter(|id|game.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).count(),2-count);assert_eq!(game.object(enemy).unwrap().zone,Zone::Battlefield,"opponents' artifacts cannot be sacrificed");
 }}
}

#[test]
fn token_claim_ownership_through_public_create_document_and_full_card_routes(){
 let text="Create three 2/2 red Noble creature tokens with custom keyword \"disgraced\", and \"This permanent enters with 1 -1/-1 counters on it.\", and \"Whenever this creature deals combat damage to a player, remove all -1/-1 counters from it.\".";
 for line in [0,4]{let tokens=ironsmith_compiler::lexer::lex_line(text,line).unwrap();
  let (create,claims)=ironsmith_compiler::word_claims::scoped(||ironsmith_compiler::effect_sentences::parse_create(&tokens,None));
  println!("direct line={line} success={} unread={:?} obligations={:?}",create.is_ok(),ironsmith_compiler::word_claims::unclaimed_words(&tokens,&claims),claims.obligations_debug());
  let (document,claims)=ironsmith_compiler::word_claims::scoped(||ironsmith_compiler::clause_support::parse_effect_sentences_lexed(&tokens));
  println!("document line={line} success={} unread={:?} obligations={:?}",document.is_ok(),ironsmith_compiler::word_claims::unclaimed_words(&tokens,&claims),claims.obligations_debug());
 }
 let source=format!("Mana cost: {{4}}{{B}}\nType: Sorcery\n{text}");
 let (loose,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(false,false,||ironsmith_compiler_runtime::compile_to_runtime_definition("Token ownership witness",&source,false)));
 println!("card loose={} loss={}",loose.is_ok(),loss.reasons_text());
 let strict=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_runtime_definition("Token ownership witness",&source,false));
 println!("card strict error={:?}",strict.as_ref().err());
 assert!(strict.is_ok(),"{strict:?}");
}
