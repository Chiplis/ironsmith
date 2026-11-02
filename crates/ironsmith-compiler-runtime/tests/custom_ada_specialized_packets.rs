//! Retained Ada target groups are native atomic recipients, never sibling copies.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{CardDefinition, CardId, GameState, ObjectId, PlayerId, Target, Zone};

fn providers()->[CardDefinition;2] {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
    let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Ada, the Free_OPH").unwrap();
    source_routes::definitions(card).unwrap()
}
fn definitions(name:&str,text:&str)->[CardDefinition;2] {
    source_routes::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()
}
fn linked(game:&mut GameState,route:usize,name:&str,back_type:&str,back_rules:&str)->CardDefinition {
    let ids=[CardId::new(),CardId::new()];let mut faces=Vec::new();
    for (i,(face,body)) in [(format!("{name} Front"),"Type: Creature — Human\nPower/Toughness: 2/2".to_string()),(format!("{name} Back"),format!("Type: {back_type}\nPower/Toughness: 3/3\n{back_rules}"))].into_iter().enumerate() {
        let builder=ironsmith_compiler::CardDefinitionBuilder::new(ids[i],&face).other_face(ids[1-i]).other_face_name(if i==0{format!("{name} Back")}else{format!("{name} Front")}).linked_face_layout(ironsmith::card::LinkedFaceLayout::TransformLike).transforming_dfc(true);
        let (compiled,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::CompilerFacade::new().compile_definition(builder,body.clone(),ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
        assert!(!loss.is_lossy(),"{}",loss.reasons_text());let compiled=compiled.unwrap().definition;
        let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.clone()).unwrap();
        let wire=ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled).unwrap();
        let artifact=ironsmith_compiled_artifact::CompiledCardArtifact::new(ironsmith_compiled_artifact::ArtifactCardIdentity{local_id:ironsmith_compiled_artifact::ArtifactCardId(ids[i].0),name:face,face_name:None,other_face:Some(ironsmith_compiled_artifact::ArtifactCardId(ids[1-i].0)),linked_face_layout:Some("transform_like".into())},ironsmith_compiled_artifact::CompiledCardPayload{definition:wire,canonical_text:ironsmith_text::compiled_text_lines(&direct).join("\n"),ability_labels:ironsmith_text::ability_surface_texts(&direct)},"ada-specialized-target-test",body.as_bytes());
        let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
        let face=if route==0{direct}else{ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()};game.register_linked_face_definition(&face);faces.push(face);
    }
    faces.remove(0)
}
fn settle(game:&mut GameState, dm:&mut play::Script) {
    let mut queue=ironsmith::triggers::TriggerQueue::new();
    for _ in 0..32 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();
        if game.stack.is_empty(){return;}
        ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();
    }
    panic!("unsettled target packet");
}
fn spell(route:usize,body:&str)->CardDefinition { definitions("Independent recipient instruction",&format!("Mana cost: {{0}}\nType: Instant\n{body}"))[route].clone() }

#[test]
fn every_original_transforms_before_any_as_transform_program_and_sibling_runs_once() {
    for (route,provider) in providers().into_iter().enumerate() {
        let mut game=play::game();game.create_object_from_definition(&provider,play::A,Zone::Battlefield);
        let pair=linked(&mut game,route,"Batch", "Artifact Creature — Construct","As this creature transforms into Batch Back, you gain 1 life for each artifact you control.");
        let left=game.create_object_from_definition(&pair,play::A,Zone::Battlefield);let right=game.create_object_from_definition(&pair,play::A,Zone::Battlefield);let untouched=game.create_object_from_definition(&pair,play::A,Zone::Battlefield);
        let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(left),Target::Object(right)],..Default::default()};
        play::cast(&mut game,play::A,&spell(route,"Transform target creature. You gain 1 life."),&mut dm);settle(&mut game,&mut dm);
        assert_eq!(game.object(left).unwrap().name,"Batch Back");assert_eq!(game.object(right).unwrap().name,"Batch Back");assert_eq!(game.object(untouched).unwrap().name,"Batch Front");
        assert_eq!(play::life(&game,play::A),25,"each original sees both artifact backs; the untargeted sibling executes once");
    }
}
#[test]
fn transform_program_actor_and_body_are_captured_before_an_earlier_child_retransforms_it() {
    for (route,provider) in providers().into_iter().enumerate() {
        let mut game=play::game();game.create_object_from_definition(&provider,play::A,Zone::Battlefield);
        let first=linked(&mut game,route,"First", "Creature — Human","As this creature transforms into First Back, gain control of all creatures until end of turn. Transform each other creature.");
        let second=linked(&mut game,route,"Second", "Creature — Human","As this creature transforms into Second Back, you gain 3 life.\nWhen this creature transforms into Second Back, you gain 5 life.");
        let left=game.create_object_from_definition(&first,play::A,Zone::Battlefield);let right=game.create_object_from_definition(&second,play::B,Zone::Battlefield);
        let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(left),Target::Object(right)],..Default::default()};play::cast(&mut game,play::A,&spell(route,"Transform target creature."),&mut dm);settle(&mut game,&mut dm);
        assert_eq!(game.object(left).unwrap().name,"First Back");assert_eq!(game.object(right).unwrap().name,"Second Front");assert_eq!(game.transform_count(right),2);
        assert_eq!(game.current_controller(right),Some(play::A));assert_eq!(play::life(&game,play::B),28,"the acquired immediate and triggered back programs still belong to their event-time controller");assert_eq!(play::life(&game,play::A),20);
    }
}
#[test]
fn convert_consumes_the_retained_group_without_repeating_a_sibling() {
    for (route,provider) in providers().into_iter().enumerate() {
        let mut game=play::game();game.create_object_from_definition(&provider,play::A,Zone::Battlefield);let pair=linked(&mut game,route,"Converted", "Creature — Human","");
        let left=game.create_object_from_definition(&pair,play::B,Zone::Battlefield);let right=game.create_object_from_definition(&pair,play::C,Zone::Battlefield);let untouched=game.create_object_from_definition(&pair,play::B,Zone::Battlefield);
        let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(left),Target::Object(right)],..Default::default()};play::cast(&mut game,play::A,&spell(route,"Convert target creature. You gain 1 life."),&mut dm);settle(&mut game,&mut dm);
        assert_eq!(game.object(left).unwrap().name,"Converted Back");assert_eq!(game.object(right).unwrap().name,"Converted Back");assert_eq!(game.object(untouched).unwrap().name,"Converted Front");assert_eq!(play::life(&game,play::A),21);
    }
}
#[test]
fn counter_spell_group_counters_both_and_leaves_an_unselected_stack_object() {
    for (route,provider) in providers().into_iter().enumerate() {
        let mut game=play::game();game.create_object_from_definition(&provider,play::A,Zone::Battlefield);let card=definitions("Stack witness","Mana cost: {0}\nType: Sorcery\nYou gain 1 life.")[route].clone();
        let mut targets=Vec::new();let mut stables=Vec::new();
        for controller in [play::B,play::C,play::B] {let id=game.create_object_from_definition(&card,controller,Zone::Stack);stables.push(game.object(id).unwrap().stable_id);targets.push(id);game.stack.push(ironsmith::StackEntry::new(id,controller));}
        let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(targets[0]),Target::Object(targets[1])],..Default::default()};play::cast(&mut game,play::A,&spell(route,"Counter target spell. You gain 1 life."),&mut dm);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        for stable in &stables[..2] {let id=game.find_object_by_stable_id(*stable).unwrap();assert_eq!(game.object(id).unwrap().zone,Zone::Graveyard);}
        assert!(game.stack.iter().any(|entry|entry.object_id==targets[2]));assert_eq!(game.object(targets[2]).unwrap().zone,Zone::Stack);assert_eq!(play::life(&game,play::A),21);
    }
}
#[test]
fn regeneration_and_goad_consume_the_whole_packet_with_the_original_actor() {
    for (route,provider) in providers().into_iter().enumerate() {for regenerate in [false,true] {
        let mut game=play::game();let actor=game.create_object_from_definition(&provider,play::A,Zone::Battlefield);let card=definitions("Creature witness","Type: Creature — Bear\nPower/Toughness: 2/2")[route].clone();
        let left=game.create_object_from_definition(&card,play::B,Zone::Battlefield);let right=game.create_object_from_definition(&card,play::C,Zone::Battlefield);let untouched=game.create_object_from_definition(&card,play::B,Zone::Battlefield);let untouched_stable=game.object(untouched).unwrap().stable_id;
        let body=if regenerate{"Regenerate target creature. You gain 1 life."}else{"Goad target creature. You gain 1 life."};let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(left),Target::Object(right)],..Default::default()};play::cast(&mut game,play::A,&spell(route,body),&mut dm);settle(&mut game,&mut dm);assert_eq!(play::life(&game,play::A),21);
        if regenerate {
            for target in [left,right,untouched] {play::apply(&mut game,actor,ironsmith::Effect::destroy(ironsmith::ChooseSpec::SpecificObject(target)));}
            assert_eq!(game.object(left).unwrap().zone,Zone::Battlefield);assert_eq!(game.object(right).unwrap().zone,Zone::Battlefield);assert!(game.is_tapped(left)&&game.is_tapped(right));let dead=game.find_object_by_stable_id(untouched_stable).unwrap();assert_eq!(game.object(dead).unwrap().zone,Zone::Graveyard);
        }else{assert!(game.is_goaded(left)&&game.is_goaded(right));assert!(!game.is_goaded(untouched));assert!(game.effect_store.goad_effects.iter().all(|effect|effect.goaded_by==play::A));}
    }}
}
#[test]
fn an_empty_retained_group_never_borrows_another_group_or_repeats_a_sibling() {
    for (route,provider) in providers().into_iter().enumerate() {
        for verb in ["Transform", "Regenerate", "Goad"] {
            let mut game=play::game();let ada=game.create_object_from_definition(&provider,play::A,Zone::Battlefield);
            let card=definitions("Empty-group witness","Type: Creature — Bear\nPower/Toughness: 2/2")[route].clone();
            let left=game.create_object_from_definition(&card,play::A,Zone::Battlefield);let right=game.create_object_from_definition(&card,play::A,Zone::Battlefield);let other=game.create_object_from_definition(&card,play::B,Zone::Battlefield);let stable=game.object(other).unwrap().stable_id;
            let body=format!("{verb} target creature you control. Target creature an opponent controls gets +1/+1 until end of turn. You gain 1 life.");
            let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(left),Target::Object(right),Target::Object(other)],..Default::default()};play::cast(&mut game,play::A,&spell(route,&body),&mut dm);
            game.move_object_by_effect(left,Zone::Exile).unwrap();game.move_object_by_effect(right,Zone::Exile).unwrap();
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            assert_eq!(play::life(&game,play::A),21);assert_eq!(game.calculated_power(other),Some(3));assert!(!game.is_goaded(other)&&!game.is_goaded(ada));
            play::apply(&mut game,ada,ironsmith::Effect::destroy(ironsmith::ChooseSpec::SpecificObject(other)));
            let dead=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(dead).unwrap().zone,Zone::Graveyard,"the surviving second group must not acquire the empty group's regeneration shield");
        }
    }
}
