//! Actual full Enspirit consumers; root prepares/materializes the preserved source fixture.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
use ironsmith::effects::{EffectContext,EffectExecutor};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn actual(name:&str)->[CardDefinition;2]{
 let path=std::env::var("IRONSMITH_MIDDLE_WHOLE_SOURCE_FIXTURE").unwrap_or_else(|_|format!("{}/../../fixtures/custom_mechanics/msem-middle-whole-source-input.json",env!("CARGO_MANIFEST_DIR")));
 let fixture:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();
 source_support::definitions(card).unwrap_or_else(|error|panic!("whole {name}: {error}"))
}
fn ordinary(name:&str,text:&str)->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
fn game()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;for player in[A,B]{for mana in[ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Black,ironsmith::mana::ManaSymbol::Red,ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Colorless]{g.player_mut(player).unwrap().mana_pool.add(mana,20);}}g}
struct Choices{resource:Option<ObjectId>,pay:bool}
impl DecisionMaker for Choices{
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.pay}
 fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{if c.description.starts_with("Choose optional costs"){return c.options.iter().filter(|o|self.pay&&o.legal).map(|o|o.index).collect();}SelectFirstDecisionMaker.decide_options(g,c)}
 fn decide_objects(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{if let Some(resource)=self.resource{if c.candidates.iter().any(|v|v.id==resource&&v.legal){return vec![resource];}}SelectFirstDecisionMaker.decide_objects(g,c)}
}
fn action(g:&mut GameState,a:LegalAction,dm:&mut Choices){use ironsmith::game_loop::*;let mut q=ironsmith::triggers::TriggerQueue::new();let mut s=PriorityLoopState::new(2);let mut p=apply_priority_response_with_dm(g,&mut q,&mut s,&PriorityResponse::PriorityAction(a),dm).unwrap();for _ in 0..80{if !s.has_pending_action(){return;}let ironsmith::GameProgress::NeedsDecisionCtx(c)=p else{panic!("{p:?}")};p=apply_decision_context_with_dm(g,&mut q,&mut s,&c,dm).unwrap();}panic!("announcement incomplete")}
fn cast(g:&mut GameState,player:PlayerId,d:&CardDefinition,dm:&mut Choices)->ObjectId{g.turn.priority_player=Some(player);let id=g.create_object_from_definition(d,player,Zone::Hand);let action=compute_legal_actions(g,player).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method:ironsmith::alternative_cast::CastingMethod::Normal,..}if *spell_id==id)).expect("actual normal cast");self::action(g,action,dm);g.stack.iter().find(|entry|!entry.is_ability).unwrap().object_id}
fn flush(g:&mut GameState,dm:&mut Choices){let mut q=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut q,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut q,dm).unwrap();}
fn resolve(g:&mut GameState,dm:&mut Choices){ironsmith::game_loop::resolve_stack_entry_with(g,dm).unwrap();}
fn move_to(g:&mut GameState,id:ObjectId,to:Zone)->ObjectId{let stable=g.object(id).unwrap().stable_id;g.move_object_by_effect(id,to).unwrap();g.find_object_by_stable_id(stable).unwrap()}
fn donor(g:&mut GameState)->ObjectId{let d=ordinary("Independent linked donor","Mana cost: {4}\nType: Creature — Elf\nPower/Toughness: 2/3");g.create_object_from_definition(&d,A,Zone::Battlefield)}
fn begin_upkeep(g:&mut GameState,dm:&mut Choices){g.turn.phase=ironsmith::Phase::Beginning;g.turn.step=Some(ironsmith::Step::Upkeep);g.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()));flush(g,dm);}
fn tokens(g:&GameState)->Vec<ObjectId>{g.battlefield.iter().copied().filter(|id|g.object(*id).is_some_and(|o|o.kind==ironsmith::ObjectKind::Token)).collect()}
#[test]
fn actual_thermal_ventcaller_copies_only_the_exact_still_linked_exile_card(){for definition in actual("Thermal Ventcaller"){
 let mut g=game();let resource=donor(&mut g);let resource_stable=g.object(resource).unwrap().stable_id;let mut dm=Choices{resource:Some(resource),pay:true};let spell=cast(&mut g,A,&definition,&mut dm);let stable=g.object(spell).unwrap().stable_id;let exiled=g.find_object_by_stable_id(resource_stable).unwrap();assert_eq!(g.object(exiled).unwrap().zone,Zone::Exile);resolve(&mut g,&mut dm);let source=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(source).unwrap().zone,Zone::Battlefield);
 let unrelated=ordinary("Unrelated exile card","Type: Creature\nPower/Toughness: 8/8");g.create_object_from_definition(&unrelated,A,Zone::Exile);
 begin_upkeep(&mut g,&mut dm);assert_eq!(g.stack.len(),1);resolve(&mut g,&mut dm);let copies=tokens(&g);assert_eq!(copies.len(),1);let copy=copies[0];assert_eq!(g.object(copy).unwrap().name.as_str(),"Independent linked donor");assert_eq!(g.current_power(copy),Some(2));assert!(g.current_has_static_ability_id(copy,ironsmith::static_abilities::StaticAbilityId::Haste));
 let other=ordinary("Independently linked donor","Type: Creature\nPower/Toughness: 9/9");let other=g.create_object_from_definition(&other,A,Zone::Battlefield);let other_stable=g.object(other).unwrap().stable_id;
 ironsmith::effects::ExileUntilEffect::source_leaves(ironsmith::target::ChooseSpec::SpecificObject(other)).execute(&mut g,&mut EffectContext::new(source,A,&mut dm)).unwrap();assert_eq!(g.object(g.find_object_by_stable_id(other_stable).unwrap()).unwrap().zone,Zone::Exile);
 let hand=move_to(&mut g,exiled,Zone::Hand);let fresh_exile=move_to(&mut g,hand,Zone::Exile);assert_ne!(fresh_exile,exiled);
 begin_upkeep(&mut g,&mut dm);resolve(&mut g,&mut dm);assert_eq!(tokens(&g),copies,"same physical card returned to exile is outside the original linked receipt");
}}
fn activate_jeama(g:&mut GameState,definition:&CardDefinition,dm:&mut Choices)->ObjectId{let source=g.create_object_from_definition(definition,A,Zone::Battlefield);g.remove_summoning_sickness(source);let selected=compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source)).unwrap();action(g,selected,dm);assert!(g.is_tapped(source));resolve(g,dm);source}
#[test]
fn actual_jeama_external_entry_followup_survives_ability_loss_but_not_a_later_incarnation(){for definition in actual("Jeama, Zoulev Queen"){for route in["ordinary","blink","countered"]{
 let mut g=game();let resource=donor(&mut g);let resource_stable=g.object(resource).unwrap().stable_id;let mut dm=Choices{resource:Some(resource),pay:true};let source=activate_jeama(&mut g,&definition,&mut dm);let source_stable=g.object(source).unwrap().stable_id;move_to(&mut g,source,Zone::Hand);
 let creature=ordinary("Next independent recipient","Mana cost: {3}\nType: Creature — Fox\nPower/Toughness: 2/2");let spell=cast(&mut g,A,&creature,&mut dm);let stable=g.object(spell).unwrap().stable_id;assert_eq!(g.object(g.find_object_by_stable_id(resource_stable).unwrap()).unwrap().zone,Zone::Exile);
 if route=="countered"{ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell)).execute(&mut g,&mut EffectContext::new_default(source,B)).unwrap();let graveyard=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(graveyard).unwrap().zone,Zone::Graveyard);let arrival=move_to(&mut g,graveyard,Zone::Battlefield);flush(&mut g,&mut dm);assert!(g.stack.is_empty());assert!(!g.current_has_card_type(arrival,ironsmith::CardType::Enchantment));continue;}
 let nullifier=ordinary("Independent battlefield ability suppressor","Type: Artifact");let nullifier=g.create_object_from_definition(&nullifier,A,Zone::Battlefield);
 let mut entrants=ironsmith::ObjectFilter::creature();entrants.zone=Some(Zone::Battlefield);
 let strip=ironsmith::continuous::ContinuousEffect::new(nullifier,A,ironsmith::continuous::EffectTarget::Filter(entrants),ironsmith::continuous::Modification::RemoveAllAbilities);g.effect_store.continuous_effects.add_effect(strip);
 resolve(&mut g,&mut dm);let mut arrival=g.find_object_by_stable_id(stable).unwrap();assert!(!g.current_has_card_type(arrival,ironsmith::CardType::Enchantment),"external followup has ordinary triggered timing, never as-entry replacement");assert!(g.current_abilities(arrival).expect("complete entrant ability state").is_empty(),"independent battlefield suppression applies before event matching; a recipient's own ETB trigger cannot supply this followup");flush(&mut g,&mut dm);assert_eq!(g.stack.len(),1,"recipient ability loss cannot remove the external delayed trigger");assert_eq!(g.stack[0].controller,A);assert_eq!(g.stack[0].source_stable_id,Some(source_stable));
 if route=="blink"{let hand=move_to(&mut g,arrival,Zone::Hand);arrival=move_to(&mut g,hand,Zone::Battlefield);}
 resolve(&mut g,&mut dm);assert_eq!(g.current_has_card_type(arrival,ironsmith::CardType::Enchantment),route=="ordinary","only the recorded entry arrival is affected");
 g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;dm.pay=false;let later=cast(&mut g,A,&creature,&mut dm);let later_stable=g.object(later).unwrap().stable_id;resolve(&mut g,&mut dm);flush(&mut g,&mut dm);assert!(g.stack.is_empty());assert!(!g.current_has_card_type(g.find_object_by_stable_id(later_stable).unwrap(),ironsmith::CardType::Enchantment),"one matching cast consumes the shared grant");
}}}
