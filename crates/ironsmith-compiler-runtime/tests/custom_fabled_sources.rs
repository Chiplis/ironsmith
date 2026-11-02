//! Full authored Fabled front/back compilation and native transformed Aura attachment.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith_compiled_artifact::{CompiledCardArtifact, ArtifactCardId, ArtifactCardIdentity, CompiledCardPayload, wire_definition_from_serializable};
const A:PlayerId=PlayerId(0);
fn compile(builder: ironsmith_compiler::CardDefinitionBuilder, text: String, payment: Option<&str>) -> (CompiledCardArtifact, CardDefinition) {
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(builder, text.clone(), ironsmith_compiler::CompilePolicy {allow_unsupported: false})
        }));
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let mut compiled = result.unwrap();
        // Match browser/audit ordering: attribute on the compiler definition,
        // before wire serialization or runtime materialization.
        if let Some(keyword) = payment {
            assert!(compiled.definition.attribute_custom_keyword_payment(keyword), "one uniquely attributable generic price is required");
        }
        let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
        let wire = ironsmith_text::with_choice_decision_labels(wire, |effect| ironsmith_runtime_catalog::artifact_materializer::materialize_effect(effect).map_err(|error| error.to_string())).unwrap();
        let direct = ironsmith_compiler_runtime::into_runtime_definition(compiled.definition.clone()).unwrap();
        if let Some(keyword) = payment {assert_eq!(direct.alternative_casts[0].name(), keyword);}
        let artifact = CompiledCardArtifact::new(ArtifactCardIdentity {
            local_id: ArtifactCardId(compiled.definition.card.id.0), name: compiled.definition.card.name.clone(), face_name: None,
            other_face: compiled.definition.card.other_face.map(|id| ArtifactCardId(id.0)),
            linked_face_layout: Some("transform_like".into()),
        }, CompiledCardPayload {definition: wire, canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"), ability_labels: ironsmith_text::ability_surface_texts(&direct)},
        "front-spell-entry-test", text.as_bytes());
        (artifact, direct)
    }

fn source_pairs(name:&str)->[(CardDefinition,CardDefinition);2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-fabled-source-complete.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();
 let faces=card["executable_faces"].as_array().unwrap();assert_eq!(faces.len(),2);
 let front_id=ironsmith_compiler::ids::CardId::new();let back_id=ironsmith_compiler::ids::CardId::new();
 let front=ironsmith_compiler::CardDefinitionBuilder::new(front_id,faces[0]["name"].as_str().unwrap()).other_face(back_id).other_face_name(faces[1]["name"].as_str().unwrap()).linked_face_layout(ironsmith_compiler::card::LinkedFaceLayout::TransformLike).transforming_dfc(true).with_custom_keyword("Fabled");
 let back=ironsmith_compiler::CardDefinitionBuilder::new(back_id,faces[1]["name"].as_str().unwrap()).other_face(front_id).other_face_name(faces[0]["name"].as_str().unwrap()).linked_face_layout(ironsmith_compiler::card::LinkedFaceLayout::TransformLike).transforming_dfc(true);
 let colors=|face:&serde_json::Value|{let mut colors=ironsmith::color::ColorSet::COLORLESS;for color in face["color_indicator"].as_array().unwrap(){colors=colors.union(match color.as_str().unwrap(){"White"=>ironsmith::color::ColorSet::WHITE,"Blue"=>ironsmith::color::ColorSet::BLUE,"Black"=>ironsmith::color::ColorSet::BLACK,"Red"=>ironsmith::color::ColorSet::RED,"Green"=>ironsmith::color::ColorSet::GREEN,_=>panic!("unrecognized source color")});}colors};
 let front=front.color_indicator(colors(&faces[0]));let back=back.color_indicator(colors(&faces[1]));
 let(fa,f)=compile(front,faces[0]["executable_text"].as_str().unwrap().into(),Some("Fabled"));let(ba,b)=compile(back,faces[1]["executable_text"].as_str().unwrap().into(),None);
 assert!(f.alternative_casts[0].battlefield_instructions().attach_to_resolving_spell_target);
 let restore=|artifact:CompiledCardArtifact|{let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(decoded,artifact);ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()};
 [(f,b),(restore(fa),restore(ba))]
}
fn announce(game: &mut GameState, action: LegalAction) {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
}
fn setup(front: &CardDefinition, back: &CardDefinition, zone: Zone) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.register_linked_face_definition(front); game.register_linked_face_definition(back);
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
    let source = game.create_object_from_definition(front, A, zone);
    (game, source)
}

#[test]fn all_twenty_nine_exact_sources_compile_both_faces_and_roundtrip(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-fabled-source-complete.json")).unwrap();let mut count=0;
 for row in fixture["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){source_pairs(card["name"].as_str().unwrap());count+=1;}}assert_eq!(count,29);
}
fn cast_selected(game:&mut GameState,source:ObjectId,selected:bool){
 for color in [ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Black,ironsmith::mana::ManaSymbol::Red,ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Colorless]{game.player_mut(A).unwrap().mana_pool.add(color,10);}
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,casting_method,..} if *spell_id==source && matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Alternative(_))==selected)).expect("authored front remains castable with native targets");announce(game,action);
}
#[test]fn actual_creature_front_resolves_before_attached_back_entry_and_normal_price_stays_front(){for(front,back)in source_pairs("Arrogant Training"){for selected in [false,true]{
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Attachment witness","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let target=game.create_object_from_definition(&witness,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,selected);assert_eq!(game.stack.len(),1);assert!(game.object(game.stack[0].object_id).unwrap().card_types.contains(&ironsmith::types::CardType::Instant));
 ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let result=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(result).unwrap().zone,if selected{Zone::Battlefield}else{Zone::Graveyard});if selected{assert_eq!(game.object(result).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(target)));assert_eq!(game.object(result).unwrap().name,"Imperious Hero");}
}}}
#[test]fn actual_resurrection_front_attaches_to_its_completed_return_arrival(){for(front,back)in source_pairs("Mortal Resurrection"){
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Returned witness","Mana cost: {1}\nType: Creature\nPower/Toughness: 2/2",false).unwrap();let old=game.create_object_from_definition(&witness,A,Zone::Graveyard);let target_stable=game.object(old).unwrap().stable_id;let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,true);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let arrival=game.find_object_by_stable_id(target_stable).unwrap();assert_ne!(arrival,old);let aura=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(aura).unwrap().zone,Zone::Battlefield);assert_eq!(game.object(aura).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(arrival)));
}}
#[test]fn actual_land_and_player_fronts_attach_to_their_authored_target_kinds(){for name in ["Till the Fields","Hasty Intervention"]{for(front,back)in source_pairs(name){
 let(mut game,source)=setup(&front,&back,Zone::Hand);let land=ironsmith_compiler_runtime::compile_to_runtime_definition("Land witness","Type: Land",false).unwrap();let target=game.create_object_from_definition(&land,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,true);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let aura=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(aura).unwrap().zone,Zone::Battlefield);assert_eq!(game.object(aura).unwrap().attached_to,Some(if name=="Till the Fields"{ironsmith::object::AttachmentTarget::Object(target)}else{ironsmith::object::AttachmentTarget::Player(A)}));
}}}

#[test]fn protection_from_back_face_prevents_entry_after_front_effect_resolves(){for(front,back)in source_pairs("Arrogant Training"){
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Protected witness","Type: Creature\nPower/Toughness: 2/2\nProtection from enchantments",false).unwrap();let target=game.create_object_from_definition(&witness,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,true);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let result=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(result).unwrap().zone,Zone::Graveyard);assert!(game.object(target).unwrap().attachments.is_empty());assert_eq!(game.calculated_power(target),Some(3));
}}
#[test]fn illegal_original_target_and_countered_front_never_enter(){for(front,back)in source_pairs("Arrogant Training"){for countered in [false,true]{
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Absent witness","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let target=game.create_object_from_definition(&witness,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,true);let stack=game.stack[0].object_id;
 let effect=if countered{ironsmith::Effect::new(ironsmith::effects::CounterEffect::new(ironsmith::ChooseSpec::SpecificObject(stack)))}else{ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(target),Zone::Graveyard,false))};
 ironsmith::effects::execute_effect(&mut game,&effect,&mut ironsmith::effects::EffectContext::new_default(stack,A)).unwrap();
 if !countered{ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
 let result=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(result).unwrap().zone,Zone::Graveyard);assert!(!game.objects_in_zone(Zone::Battlefield).into_iter().any(|id|game.object(id).is_some_and(|object|object.name=="Imperious Hero")));
}}}

#[test]fn actual_planeswalker_front_uses_planeswalker_attachment(){for(front,back)in source_pairs("Hat Trick"){
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Loyalty witness","Type: Planeswalker\nLoyalty: 3\n+1: You gain 1 life.",false).unwrap();let target=game.create_object_from_definition(&witness,A,Zone::Battlefield);let other=game.create_object_from_definition(&witness,A,Zone::Battlefield);let first_activation=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source,ability_index:0}if *source==target)).unwrap();announce(&mut game,first_activation);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source,ability_index:0}if *source==target)),"ordinary loyalty limit is already used before the front spell resolves");let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,true);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let aura=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(aura).unwrap().zone,Zone::Battlefield);assert_eq!(game.object(aura).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(target)));assert_eq!(game.named_turn_counter(&format!("loyalty_allowance:extra:object:{}",target.0)),1);assert_eq!(game.named_turn_counter(&format!("loyalty_allowance:extra:object:{}",other.0)),0,"the permission is bound to the announced target, not every planeswalker");let extra_activation=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source,ability_index:0}if *source==target)).expect("the front spell grants one genuine extra loyalty activation");announce(&mut game,extra_activation);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().life,22);assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source,ability_index:0}if *source==target)),"the grant does not permit a third activation");
}}

#[test]fn back_face_retains_authored_white_when_front_presentation_is_colorless(){for(mut front,back)in source_pairs("Arrogant Training"){
 assert_eq!(back.card.color_indicator,Some(ironsmith::color::ColorSet::WHITE));
 // A colorless front presentation isolates the back-face protection check.
 // The actual linked Aura keeps its independently authored white indicator.
 front.card.color_indicator=Some(ironsmith::color::ColorSet::COLORLESS);
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("White protection witness","Type: Creature\nPower/Toughness: 2/2\nProtection from white",false).unwrap();let target=game.create_object_from_definition(&witness,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,true);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let result=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(result).unwrap().zone,Zone::Graveyard);assert_eq!(game.calculated_power(target),Some(3));
}}

#[test]fn copied_selected_spell_resolves_front_instructions_without_entering(){for(front,back)in source_pairs("Arrogant Training"){
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Copy target","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let target=game.create_object_from_definition(&witness,A,Zone::Battlefield);cast_selected(&mut game,source,true);let original=game.stack.last().unwrap().object_id;let effect=ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::single(ironsmith::ChooseSpec::SpecificObject(original)));ironsmith::effects::execute_effect(&mut game,&effect,&mut ironsmith::effects::EffectContext::new_default(original,A)).unwrap();let copied=game.stack.last().unwrap().object_id;assert_ne!(copied,original);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.calculated_power(target),Some(3));assert!(game.object(target).unwrap().attachments.is_empty());assert_eq!(game.stack.len(),1);assert!(!game.objects_in_zone(Zone::Battlefield).into_iter().any(|id|game.object(id).is_some_and(|object|object.name=="Imperious Hero")));
}}
#[test]fn unrelated_target_leave_and_return_before_resolution_is_not_rebound(){for(front,back)in source_pairs("Arrogant Training"){
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Blink target","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let target=game.create_object_from_definition(&witness,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;let target_stable=game.object(target).unwrap().stable_id;cast_selected(&mut game,source,true);let departed=game.move_object_by_effect(target,Zone::Hand).unwrap();let returned=game.move_object_by_effect(departed,Zone::Battlefield).unwrap();assert_ne!(target,returned);assert_eq!(game.object(returned).unwrap().stable_id,target_stable);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let card=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(card).unwrap().zone,Zone::Graveyard);assert!(game.object(returned).unwrap().attachments.is_empty());
}}
#[test]fn partially_illegal_front_target_cannot_receive_the_transformed_aura(){for(mut front,back)in source_pairs("Arrogant Training"){
 let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Two target program","Type: Instant\nTarget creature you control gets +1/+1 until end of turn. Target creature you don't control gets +1/+1 until end of turn.",false).unwrap();front.spell_effect=body.spell_effect;
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Target witness","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let first=game.create_object_from_definition(&witness,A,Zone::Battlefield);let second=game.create_object_from_definition(&witness,PlayerId(1),Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,true);let change=ironsmith::Effect::new(ironsmith::effects::GainControlEffect::permanent(ironsmith::ChooseSpec::SpecificObject(first)));let stack_source=game.stack[0].object_id;ironsmith::effects::execute_effect(&mut game,&change,&mut ironsmith::effects::EffectContext::new_default(stack_source,PlayerId(1))).unwrap();
 ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let aura=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(aura).unwrap().zone,Zone::Battlefield);assert_eq!(game.object(aura).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(second)));assert!(game.object(first).unwrap().attachments.is_empty());
}}

#[test]fn actual_deaths_door_requires_this_turn_battlefield_history_then_attaches_to_return(){for(front,back)in source_pairs("Death's Door"){
 let(mut game,source)=setup(&front,&back,Zone::Hand);let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("History witness","Type: Creature\nPower/Toughness: 2/2",false).unwrap();game.create_object_from_definition(&witness,A,Zone::Graveyard);
 for color in [ironsmith::mana::ManaSymbol::Black,ironsmith::mana::ManaSymbol::Colorless]{game.player_mut(A).unwrap().mana_pool.add(color,10);}
 assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)),"direct graveyard creation cannot forge battlefield history");
 let fallen=game.create_object_from_definition(&witness,A,Zone::Battlefield);let target_stable=game.object(fallen).unwrap().stable_id;let movement=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(fallen),Zone::Graveyard,false));ironsmith::effects::execute_effect(&mut game,&movement,&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();let stable=game.object(source).unwrap().stable_id;cast_selected(&mut game,source,true);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let arrival=game.find_object_by_stable_id(target_stable).unwrap();let aura=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrival).unwrap().zone,Zone::Battlefield);assert_eq!(game.object(aura).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(arrival)));assert!(game.is_tapped(arrival));
}}

#[test]
fn exact_devils_fair_share_keeps_control_and_power_toughness_on_the_same_turn_condition() {
    for (_,back) in source_pairs("Deal with the Devil") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let opponent=PlayerId(1);
        let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Attached witness","Type: Creature\nPower/Toughness: 3/4",false).unwrap();
        let target=game.create_object_from_definition(&creature,opponent,Zone::Battlefield);
        let aura=game.create_object_from_definition(&back,A,Zone::Battlefield);
        assert!(game.attach_object_to_target(aura,ironsmith::object::AttachmentTarget::Object(target)));
        for (active,controller,power,toughness) in [(A,A,5,5),(opponent,opponent,3,4),(A,A,5,5)] {
            game.turn.active_player=active;
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_controller(target),Some(controller));
            assert_eq!(game.calculated_power(target),Some(power));
            assert_eq!(game.calculated_toughness(target),Some(toughness));
        }
        game.move_object_by_effect(aura,Zone::Graveyard).unwrap();
        assert_eq!(game.current_controller(target),Some(opponent));
        assert_eq!(game.calculated_power(target),Some(3));
    }
}
