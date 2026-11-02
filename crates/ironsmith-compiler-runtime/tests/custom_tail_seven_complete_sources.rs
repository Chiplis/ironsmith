//! Complete source equations, strict independent routes; no excluded Lucid augmentation.
mod custom_final_next_wave_support;
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use serde_json::Value;use std::collections::BTreeSet;
const A:PlayerId=PlayerId(0);
fn fixture()->Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-tail-seven-complete-sources.json")).unwrap()}
fn source(key:&str)->Value {fixture()["mechanics"].as_array().unwrap().iter().flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["source_card_key"]==key).unwrap().clone()}
fn definitions(key:&str)->[CardDefinition;2]{custom_final_next_wave_support::definitions(&source(key)).unwrap_or_else(|error|panic!("{key}: {error}"))}
#[test]
fn exact_seven_in_scope_whole_families_compile_every_complete_source_on_both_independent_routes(){
 let fixture=fixture();let mut errors=Vec::new();let mut total=0;
 for (family,expected) in [("derange", &["Nayando Found_HQZ"][..]),("distorted", &["Abandoned Hermit_ORP","Ada, the Free_OPH","Alyss, Lawweaver_OPH","Arbiter of Shal'Iv_OTH","Aurora, Soulweaver_OPH","Better Half_OPH","Chantwood Guards_ORP","Claw Crusher_ORP","Contorted Reflection_OPH","Crowned Outcast_OPH","Crystallizer_OPH","Deranged Seeker_OPH","Dinah, Lifeweaver_OPH","Duela Duelist_ORP","Echo Cleric_ORP","Essence Weaver_ORP","Glittering Seeker_OPH","Hall of Mirrors_OPH","Kohmo the Grizzled_ORP","Lha Striker_ORP","Light Knight_OPH","Lightbolt Archer_OPH","Lusterform_OPH","Lusterwing Owl_OPH","Mirrorform_ORP","Mirrorstalk_ORP","Mirrorwalk_OPH","Omhok the Revealer_OPH","Oslyd Dreamer_ORP","Oslyd Heretic_ORP","Promising Visage_OPH","Rangewalker Pyromancer_ORP","Relitian Renegade_OTH","Seeker's Garment_OPH","Shard Sword_ORP","Sluiceway Shrieker_ORP","Snowy Misthorn_OPH","Spring Recaller_ORP","Treasurer of Thought_OPH","Twisted Echo_ORP","Twisted Legacy_ORP","Twisted-Heart Duelist_OTH","Warped Reader_OPH"][..]),("figment", &["Ambivalence_G5L","Blinded Stomper_G5L","Bonded Hunter_G5L","Danaj War Lizard_G5L","Div of a Thousand Grains_G5L","Hadak's Gilded Guard_G5L","Mirageblade Arcanist_G5L","Mirages of Dunesea_G5L","Muktann Mirage_G5L","Muktann Scorpion_G5L","Roadworn Dromedary_G5L","Rotten Veilcaster_G5L","Splintered Devotee_G5L","Sunshroud Coil_G5L","Wonderlight Magus_G5L"][..]),("husk", &["Skull Splice_MSY"][..]),("living ritual", &["Black Tesseract_HQZ","Blackened Heart_HQZ","Crooked Whistle_HQZ","Hateful Eye_HQZ","Liar's Tongue_HQZ","Maddening Trinket_HQZ","Traitor's Finger_HQZ","Twisting Puzzlebox_HQZ"][..]),("multitude", &["Binding Whisper_BLR","Collector of the Lost_BLR","Drift Off_BLR","Glimpse of Something Great_BLR","Lichbane Forerunner_BLR","Matron of Lost Glades_BLR","Outerworld Witness_HQZ","Scion of the Wildcourt_BLR","Stalwart Grovekeepers_BLR","Sylph of the Tugae_BLR","Tugae Corpsemaker_BLR","Tugae Rallier_BLR","Unending Chant_BLR","Unrivaled Slayer_BLR","Wildcourt Conserver_BLR","Wildcourt Wishgiver_BLR","Wildkin Imposter_BLR"][..]),("practiced", &["Mistress of Cats_MSY"][..])]{
  let mut keys=BTreeSet::new();
  for row in fixture["mechanics"].as_array().unwrap().iter().filter(|r|r["keyword"].as_str().unwrap().eq_ignore_ascii_case(family)){
   for card in row["representative_cards"].as_array().unwrap(){
    let key=card["source_card_key"].as_str().unwrap();assert!(keys.insert(key.to_string()),"duplicate:{family}/{key}");total+=1;
    let originals=card["source_faces"].as_array().unwrap();assert!(!originals.is_empty(),"{key}: missing preserved whole source");
    match custom_final_next_wave_support::linked_definitions(card){
     Err(error)=>errors.push(format!("row={}/card={key}/faces={:?}: {error}",row["id"],originals.iter().map(|f|f["name"].clone()).collect::<Vec<_>>())),
     Ok(routes)=>{assert_eq!(routes.len(),originals.len(),"{key}: sibling lost");for(i,pair)in routes.iter().enumerate(){assert_eq!(pair[0].card.name.as_str(),originals[i]["name"].as_str().unwrap());assert_eq!(pair[1].card.name,pair[0].card.name);}}
    }
   }
  }
  assert_eq!(keys,expected.iter().map(|s|s.to_string()).collect::<BTreeSet<_>>(),"exactsourceequation:{family}");
 }
 assert_eq!(total,86);assert!(errors.is_empty(),"aggregate strict whole-source failures:\n{}",errors.join("\n"));
}
fn announce(game:&mut GameState,action:LegalAction){
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..96{if !state.has_pending_action(){ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut dm).unwrap();return}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("unfinished cast")
}
#[test]
fn actual_sylph_multitude_is_one_alternative_total_price_and_only_paid_cast_has_copy_trigger(){
 for definition in definitions("Sylph of the Tugae_BLR"){for alternative in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.turn.active_player=A;game.turn.priority_player=Some(A);
  let source=game.create_object_from_definition(&definition,A,Zone::Hand);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::White,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,5);
  let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source&&matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Alternative(_))==alternative)).unwrap();announce(&mut game,action);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),if alternative{0}else{3},"printed3 versus alternative6, not printed plus additional6");
  assert_eq!(game.stack.len(),if alternative{2}else{1});
  while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  let creatures=game.battlefield.iter().filter(|id|game.object(**id).unwrap().card_types.contains(&ironsmith::CardType::Creature)).collect::<Vec<_>>();assert_eq!(creatures.len(),if alternative{2}else{1});
  assert_eq!(creatures.iter().filter(|id|game.object(***id).unwrap().kind==ironsmith::object::ObjectKind::Token).count(),usize::from(alternative));
  for id in creatures{assert!(game.object(*id).unwrap().has_static_ability_id(ironsmith::static_abilities::StaticAbilityId::Vigilance));}
 }}
}
struct Choice(usize);impl DecisionMaker for Choice{fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{ctx.options.iter().find(|o|o.legal&&o.index==self.0).or_else(||ctx.options.iter().find(|o|o.legal)).map(|o|vec![o.index]).unwrap_or_default()}}
#[test]
fn every_actual_living_ritual_source_preserves_named_blueprint_and_attaches_the_selected_actual_token(){
 let keys=["Black Tesseract_HQZ","Blackened Heart_HQZ","Crooked Whistle_HQZ","Hateful Eye_HQZ","Liar's Tongue_HQZ","Maddening Trinket_HQZ","Traitor's Finger_HQZ","Twisting Puzzlebox_HQZ"];
 for key in keys {for definition in definitions(key){for choice in [0,1]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  for i in 0..2{let mut helper=definition.clone();helper.card.id=ironsmith::CardId::new();helper.card.name=format!("Different implement {i}").into();helper.abilities.clear();helper.spell_effect=None;game.create_object_from_definition(&helper,A,Zone::Battlefield);}
  let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
  ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::Source,Zone::Battlefield,false),&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();
  let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=Choice(choice);
  for _ in 0..12{ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();if game.stack.is_empty(){break}ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
  assert!(game.stack.is_empty());let arrived=game.find_object_by_stable_id(stable).unwrap();let tokens=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().kind==ironsmith::object::ObjectKind::Token).collect::<Vec<_>>();assert_eq!(tokens.len(),1,"{key}: selectedtoken remains under three distinct Implement names");let token=tokens[0];assert_eq!(game.object(arrived).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(token)));
  if choice==1{assert_eq!(game.current_name(token).as_deref(),Some("Sulmina, the Final Age"));assert!(game.object(token).unwrap().colors().contains(ironsmith::color::Color::Black));assert!(game.object(token).unwrap().has_static_ability_id(ironsmith::static_abilities::StaticAbilityId::Flying));assert!(game.object(token).unwrap().has_static_ability_id(ironsmith::static_abilities::StaticAbilityId::Indestructible));}else{assert!(game.object(token).unwrap().subtypes.contains(&ironsmith::types::Subtype::Devil));assert!(game.object(token).unwrap().colors().contains(ironsmith::color::Color::Red));}
 }}}
}
