//! Whole physical sources, strict claims, independent native/artifact routes.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use serde_json::Value;
use std::collections::BTreeSet;
fn fixture(revive:bool)->Value {serde_json::from_str(if revive {include_str!("../../../fixtures/custom_mechanics/msem-revive-whole-source-inputs.json")}else{include_str!("../../../fixtures/custom_mechanics/revolution-waste-bound.json")}).unwrap()}
fn cards(data:&Value)->Vec<&Value>{data["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).collect()}
fn key(card:&Value)->&str{card["raw_source_card_key"].as_str().or_else(||card["source_card_key"].as_str()).unwrap_or_else(||card["id"].as_str().unwrap()).trim_start_matches("rev:")}
fn collect(revive:bool,count:usize){
 let data=fixture(revive);let cards=cards(&data);let mut keys=BTreeSet::new();let mut failures=Vec::new();
 for card in cards {let key=key(card);if !keys.insert(key){failures.push(format!("duplicate physical source {key}"));continue;}
  assert_eq!(card["source_faces"].as_array().unwrap().len(),1,"full face evidence: {key}");
  assert_eq!(card["source_shape"],"normal","unsupported layout must not be silently flattened: {key}");
  assert!(card["original_text"].as_str().is_some_and(|text|!text.trim().is_empty()),"raw source {key}");
  match source_support::linked_definitions(card){Ok(routes)=>{assert_eq!(routes.len(),1,"{key}");},Err(error)=>failures.push(format!("{key}: {error}"))}
 }
 assert_eq!(keys.len(),count,"authoritative physical scope, not row/seed count");
 let expected:BTreeSet<_>=if revive{EXPECTED_REVIVE}else{EXPECTED_WASTE}.iter().copied().collect();assert_eq!(keys,expected,"exact authoritative physical keys, not merely equal counts");
 assert!(failures.is_empty(),"{} complete-source failures:\n{}",failures.len(),failures.join("\n"));
}
#[test]fn all_twenty_nine_waste_sources_strict_native_and_artifact(){collect(false,29)}
#[test]fn all_seventy_six_revive_sources_strict_native_and_artifact(){collect(true,76)}
fn named(name:&str)->[ironsmith::CardDefinition;2]{let data=fixture(true);let card=cards(&data).into_iter().find(|c|c["name"]==name).unwrap_or_else(||panic!("missing full source {name}"));source_support::definitions(card).unwrap()}
const A:ironsmith::PlayerId=ironsmith::PlayerId(0);
fn setup()->ironsmith::GameState{let mut game=ironsmith::GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game}
fn activate(game:&mut ironsmith::GameState,source:ironsmith::ObjectId){
 use ironsmith::decision::compute_legal_actions;use ironsmith::game_loop::*;
 let selected=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,ironsmith::decision::LegalAction::ActivateAbility{source:id,..}if *id==source)).expect("complete real source activation must be affordable");
 let mut dm=ironsmith::decision::SelectFirstDecisionMaker;let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(selected),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){break}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
 assert!(!state.has_pending_action());assert_eq!(game.stack.len(),1);resolve_stack_entry_with(game,&mut dm).unwrap();
}
#[test]fn real_revive_completion_binds_created_token_to_both_heterogeneous_observers(){
 for route in 0..2{let mut game=setup();let k=game.create_object_from_definition(&named("Kugoja, Perished Conduit")[route],A,ironsmith::Zone::Battlefield);let t=game.create_object_from_definition(&named("Tomb Scavenger")[route],A,ironsmith::Zone::Battlefield);let before=game.calculated_power(t).unwrap();let k_before=game.calculated_power(k).unwrap();
 let card=named("Revived Colpah")[route].clone();let source=game.create_object_from_definition(&card,A,ironsmith::Zone::Graveyard);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,3);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,1);activate(&mut game,source);
 let token=*game.battlefield.iter().find(|id|game.object(**id).unwrap().name=="Revived Colpah").unwrap();assert!(game.object(token).unwrap().mana_cost.is_none());let token_before=game.calculated_power(token).unwrap();
 let mut dm=ironsmith::decision::SelectFirstDecisionMaker;let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert_eq!(game.stack.len(),2,"one actual completion, not an ETB-from-graveyard duplicate");
 while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
 assert_eq!(game.calculated_power(token),Some(token_before+2));assert_eq!(game.calculated_power(t),Some(before+2));assert_eq!(game.calculated_power(k),Some(k_before));assert!(game.current_has_static_ability_id(token,ironsmith_core::StaticAbilityId::Haste));assert!(game.current_has_static_ability_id(token,ironsmith_core::StaticAbilityId::Menace));
 }
}

#[test]fn aura_revive_chooses_attachment_only_on_resolution_and_never_enters_unattached(){
 for route in 0..2{for eligible in [false,true]{let mut game=setup();
 let host=eligible.then(||{let d=ironsmith_compiler_runtime::compile_to_runtime_definition("Aura host","Type: Creature\nPower/Toughness: 5/5",false).unwrap();game.create_object_from_definition(&d,A,ironsmith::Zone::Battlefield)});
 let source=game.create_object_from_definition(&named("Enduring Plague")[route],A,ironsmith::Zone::Graveyard);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,6);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,1);activate(&mut game,source);
 let auras:Vec<_>=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.name=="Enduring Plague").collect();assert_eq!(auras.len(),usize::from(eligible));
 if let Some(host)=host{assert_eq!(auras[0].attached_to,Some(ironsmith::object::AttachmentTarget::Object(host)));assert!(auras[0].mana_cost.is_none());assert_eq!(game.calculated_power(host),Some(2));assert_eq!(game.calculated_toughness(host),Some(2));}
 }}
}
#[test]fn actual_static_grant_uses_recipient_mana_price_and_lifetime(){
 for route in 0..2{let mut game=setup();let grantor=game.create_object_from_definition(&named("Still-Smoke Stitcher")[route],A,ironsmith::Zone::Battlefield);
 let ordinary=ironsmith_compiler_runtime::compile_to_runtime_definition("Granted recipient","Mana cost: {1}{G}\nType: Creature\nPower/Toughness: 4/4",false).unwrap();let source=game.create_object_from_definition(&ordinary,A,ironsmith::Zone::Graveyard);
 game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green,1);activate(&mut game,source);
 let token=*game.battlefield.iter().find(|id|game.object(**id).unwrap().name=="Granted recipient").unwrap();assert!(game.object(token).unwrap().mana_cost.is_none());assert_eq!(game.calculated_power(token),Some(4));assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
 let second=game.create_object_from_definition(&ordinary,A,ironsmith::Zone::Graveyard);game.move_object_by_effect(grantor,ironsmith::Zone::Hand).unwrap();game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green,1);
 assert!(!ironsmith::decision::compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,ironsmith::decision::LegalAction::ActivateAbility{source,..}if *source==second)),"source-local static grant ends when grantor leaves");
 }
}
fn waste_named(name:&str)->[ironsmith::CardDefinition;2]{let data=fixture(false);let card=cards(&data).into_iter().find(|c|c["name"]==name).unwrap();source_support::definitions(card).unwrap()}
#[test]fn twenty_ninth_waste_source_observes_one_actual_two_land_batch(){
 for route in 0..2{let mut game=setup();let observer=game.create_object_from_definition(&waste_named("Realm Siphoner")[route],A,ironsmith::Zone::Battlefield);
 let land=ironsmith_compiler_runtime::compile_to_runtime_definition("Witness grove","Type: Land — Forest\n{T}: Add {G}.",false).unwrap();let own1=game.create_object_from_definition(&land,A,ironsmith::Zone::Battlefield);let own2=game.create_object_from_definition(&land,A,ironsmith::Zone::Battlefield);let opponent=game.create_object_from_definition(&land,ironsmith::PlayerId(1),ironsmith::Zone::Battlefield);
 let wastes=ironsmith_compiler_runtime::compile_to_runtime_definition("Wastes","Type: Basic Land\n{T}: Add {C}.",false).unwrap();let already=game.create_object_from_definition(&wastes,A,ironsmith::Zone::Battlefield);
 let draw=ironsmith_compiler_runtime::compile_to_runtime_definition("Draw witness","Type: Instant\nYou gain 1 life.",false).unwrap();game.create_object_from_definition(&draw,A,ironsmith::Zone::Library);
 let spell=waste_named("Landscape Decay")[route].clone();let source=game.create_object_from_definition(&spell,A,ironsmith::Zone::Stack);let mut dm=ironsmith::decision::SelectFirstDecisionMaker;let mut ctx=ironsmith::effects::EffectContext::new(source,A,&mut dm);
 for effect in spell.spell_effect.as_ref().unwrap().all_effects(){ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();}
 for id in [own1,own2,already]{assert_eq!(game.current_characteristics(id).unwrap().name.as_ref(),"Wastes");}assert_eq!(game.current_characteristics(opponent).unwrap().name.as_ref(),"Witness grove");
 let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert_eq!(game.stack.len(),1,"actual completed two-land batch triggers once");ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
 assert_eq!(game.player(A).unwrap().life,22);assert_eq!(game.player(ironsmith::PlayerId(1)).unwrap().life,18);assert_eq!(game.player(A).unwrap().hand.len(),1);assert!(game.object(observer).is_some());
 }
}

const EXPECTED_WASTE: &[&str] = &["Desperate Animist_VST", "Exiled Channeler_VST", "Forlorn Firstspear_VST", "Goresand Remnant_VST", "Harvester of Vigor_VST", "Landscape Decay_VST", "Leyline Feeder_VST", "Leyline Reaper_VST", "Leyline Router_VST", "Life Desecrator_VST", "Mirage Oddity_VST", "Radaran Reclaimer_VST", "Radaran Remnant_VST", "Radaran Stirrings_VST", "Radaran Undying_VST", "Rayak's Prelate_VST", "Realm Siphoner_VST", "Resigned Seraph_VST", "Ruinstorm Remnant_VST", "Sanctis Banneret_VST", "Sanctis Remnant_VST", "Sand Manipulator_VST", "Sandwarp_VST", "Saviour of the Lost_VST", "Sephius's Research_VST", "Sephius, Endtime Scholar_VST", "The Fall of Aethanis_VST", "The Source_VST", "Tradeway Remnant_VST"];

const EXPECTED_REVIVE: &[&str] = &["Adamant Cultist_ORP", "Arisen Haploc_OPH", "Beleaguered Conquistador_MSY", "Bleak Cultist_ORP", "Blinded Necromancer_BYK", "Blooming Martyr_BYK", "Bone Pit Raptor_GNJ", "Cackling Seer_BYK", "Chantwood Cultist_ORP", "Contagious Host_OPH", "Crimson Hellmaw_GNJ", "Deathless Disciple_TOJ", "Descent from the Highlands_OTH", "Diseased Prophet_GNJ", "Eighth-Life Companion_OTH", "Emissary of the Wicked_GNJ", "Enduring Plague_OTH", "Eternal Nightwalker_ORP", "Everember Phoenix_BYK", "Exultation of Might_DHL", "Fallow Grave_ORP", "Fatethief_ORP", "Ferocity of the Redeemed_DHL", "Fhinon, the Dirgechanter_SWR", "Fidelity of the Pure_DHL", "Fleshwrithe Ghoul_GNJ", "Fortune Dealer_BYK", "Hadik, Might of Mortals_BYK", "Hollyberry Murder_BYK", "Ichorsap Dryad_BYK", "Ill-Fated Seer_BYK", "Kai'Jun, the Annihilator_GNJ", "Kai'Shi, Anguished One_GNJ", "Kalak Culler_OPH", "Kareki Retainer_TOJ", "Kareki Vanguard_TOJ", "Kenleigh, Liege of Redemption_HZN", "Kinjiru Blackguard_TOJ", "Kugoja, Perished Conduit_GNJ", "Lha Reckoner_OPH", "Lifebreather_OPH", "Lives Redeemed_TOJ", "Lizhao, Lord of the Yaogu_GNJ", "Lunar Defractor_OPH", "Moranai Survivalist_TOJ", "Mortal Tapestry_TOJ", "Moth-Heart Husk_BYK", "Profaner of Souls_TOJ", "Raikai Raider_TOJ", "Ravaging Ghoul_GNJ", "Reclaimed Kalak_OPH", "Red Flower Assassin_TOJ", "Remembrance of Light_DHL", "Remnant of the Hunt_BYK", "Resolution of Stone_DHL", "Revived Colpah_ORP", "Rotted Kalak_ORP", "Scion of Fecundity_DHL", "Settlement Scout_BYK", "Shattered Mirrorwalker_ORP", "Shengin Renegade_TOJ", "Shredskin Sky Horror_GNJ", "Sightless Blight_GNJ", "Silver-Dawn Elder_BYK", "Skeletal Shredwing_GNJ", "Spirit Keeper_TOJ", "Stargaze Spirit_OPH", "Still-Smoke Hunter_BYK", "Still-Smoke Stitcher_BYK", "Strength of the Patient_DHL", "Tomb Scavenger_GNJ", "Twisted Echo_ORP", "Undead Archivist_GNJ", "Undeath Comes_GNJ", "Wild Hermit_ORP", "Wings of the Moon_OPH"];

#[test]fn ordinary_copy_token_with_revive_ability_is_not_a_revive_completion(){
 for route in 0..2{let mut game=setup();game.create_object_from_definition(&named("Kugoja, Perished Conduit")[route],A,ironsmith::Zone::Battlefield);game.create_object_from_definition(&named("Tomb Scavenger")[route],A,ironsmith::Zone::Battlefield);let original=game.create_object_from_definition(&named("Revived Colpah")[route],A,ironsmith::Zone::Battlefield);
 let copy=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary copy witness","Type: Sorcery
Create a token that's a copy of target creature.",false).unwrap();let source=game.create_object_from_definition(&copy,A,ironsmith::Zone::Stack);let mut dm=ironsmith::decision::SelectFirstDecisionMaker;let mut ctx=ironsmith::effects::EffectContext::new(source,A,&mut dm).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(original)]);for effect in copy.spell_effect.as_ref().unwrap().all_effects(){ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();}
 assert_eq!(game.battlefield.iter().filter(|id|game.object(**id).unwrap().name=="Revived Colpah").count(),2);
 let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert!(game.stack.is_empty(),"keyword marker and ordinary token ETB do not manufacture completion");
 }
}
