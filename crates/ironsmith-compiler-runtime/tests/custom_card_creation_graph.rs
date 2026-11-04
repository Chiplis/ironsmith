//! Independent native/runtime-wire graph behavior. Parent executes this target.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_core::{CardCreationLibraryPosition, CardCreationSelection, CreateCardsEffect, CardId, PlayerFilter};
const A:PlayerId=PlayerId(0);
fn creation(scope:CardId,refs:Vec<CardId>,nodes:Vec<CardDefinition>,name:&str)->Effect {
    Effect::new(CreateCardsEffect {definitions:nodes,definition_scope:Some(scope),definition_refs:refs,
        operation_label:None,selection:CardCreationSelection::Named(name.into()),count:1.into(),
        player:PlayerFilter::You,zone:Zone::Hand,library_position:CardCreationLibraryPosition::Top})
}
fn nodes()->Vec<CardDefinition>{
    let a=CardId::new();let b=CardId::new();
    vec![CardDefinitionBuilder::new(a,"Graph Alpha").card_types(vec![ironsmith::types::CardType::Sorcery])
        .with_spell_effect(vec![creation(a,vec![a,b],vec![],"Graph Beta")]).build(),
        CardDefinitionBuilder::new(b,"Graph Beta").card_types(vec![ironsmith::types::CardType::Sorcery])
        .with_spell_effect(vec![creation(a,vec![a,b],vec![],"Graph Alpha")]).build()]
}
fn closed(nodes:&[CardDefinition],index:usize)->CardDefinition{
    ironsmith_runtime_catalog::artifact_materializer::bind_card_creation_scope_runtime(nodes[index].clone(),nodes[0].card.id,nodes).unwrap()
}
fn resolve(game:&mut GameState,definition:&CardDefinition)->ironsmith::ObjectId{
    let source=game.create_object_from_definition(definition,A,Zone::Stack);
    let mut ctx=EffectContext::new_default(source,A);
    let mut ids=Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects(){
        ids.extend_from_slice(execute_effect(game,effect,&mut ctx).unwrap().output_objects());
    }
    assert_eq!(ids.len(),1);ids[0]
}
#[test]
fn mutual_graph_creation_exports_and_survives_restart_without_original_source(){
    let pool=nodes();let alpha=closed(&pool,0);
    for roundtrip in [false,true]{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let root=if roundtrip {let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(alpha.clone()).unwrap();
            let bytes=serde_json::to_vec(&wire).unwrap();let wire=serde_json::from_slice(&bytes).unwrap();
            ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).unwrap()}else{alpha.clone()};
        let beta=resolve(&mut game,&root);
        let generated=game.object(beta).unwrap().physical_card_definition();
        let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(generated.clone()).unwrap();
        let bytes=serde_json::to_vec(&wire).unwrap();assert!(bytes.len()<100_000,"catalog must remain shallow");
        let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_definition(serde_json::from_slice(&bytes).unwrap()).unwrap();
        let mut fresh=GameState::new(vec!["A".into(),"B".into()],20);
        let alpha_id=resolve(&mut fresh,&restored);assert_eq!(fresh.object(alpha_id).unwrap().name.as_str(),"Graph Alpha");
        fresh.restart_game(A,&[]);
        let restart_alpha=fresh.objects_in_deterministic_order().into_iter().find(|object|object.name.as_str()=="Graph Alpha").unwrap().physical_card_definition();
        let beta_id=resolve(&mut fresh,&restart_alpha);assert_eq!(fresh.object(beta_id).unwrap().name.as_str(),"Graph Beta");
    }
}
#[test]
fn graph_admission_rejects_dangling_ids_and_duplicate_catalog_identities(){
    let pool=nodes();let mut root=closed(&pool,0);
    let effect=creation(pool[0].card.id,vec![CardId::new()],pool.clone(),"Graph Alpha");
    root.spell_effect=Some(ironsmith_core::ResolutionProgram::from_effects(vec![effect]));
    let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(root).unwrap();
    assert!(ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).is_err());
    let mut bad=pool.clone();bad[1].card.id=bad[0].card.id;
    let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(pool[0].clone()).unwrap();
    let bad=bad.into_iter().map(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition).collect::<Result<Vec<_>,_>>().unwrap();
    assert!(ironsmith_runtime_catalog::artifact_materializer::bind_card_creation_scope(wire,pool[0].card.id,&bad).is_err());
}

#[test]
fn duplicate_uses_current_name_within_scope_instead_of_original_definition_or_token_recipe(){
    let mut pool=nodes();
    pool[0].card.other_face_name=Some("Graph Back".into());
    pool[0].card.linked_face_layout=ironsmith::card::LinkedFaceLayout::TransformLike;
    for roundtrip in [false,true] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let target=game.create_object_from_definition(&pool[0],A,Zone::Battlefield);
        game.object_mut(target).unwrap().name="Graph Beta".into();
        let payload=CreateCardsEffect {definitions:pool.clone(),definition_scope:Some(pool[0].card.id),
            definition_refs:pool.iter().map(|node|node.card.id).collect(),operation_label:None,
            selection:CardCreationSelection::Duplicate(ironsmith::target::ChooseSpec::SpecificObject(target)),
            count:1.into(),player:PlayerFilter::You,zone:Zone::Hand,library_position:CardCreationLibraryPosition::Top};
        let mut effect=Effect::new(payload);
        if roundtrip {let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(effect).unwrap();
            effect=ironsmith_runtime_catalog::artifact_materializer::materialize_effect(serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();}
        let mut ctx=EffectContext::new_default(target,A);
        let made=execute_effect(&mut game,&effect,&mut ctx).unwrap();
        assert_eq!(made.output_objects().len(),1);
        assert_eq!(game.object(made.output_objects()[0]).unwrap().card,Some(pool[1].card.id));
        game.object_mut(target).unwrap().name="Unadmitted token name".into();
        assert!(matches!(execute_effect(&mut game,&effect,&mut ctx),Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))),"unprovided current names are an explicit evidence failure");
        game.set_described_face_down(target);
        let made=execute_effect(&mut game,&effect,&mut ctx).unwrap();
        assert!(made.output_objects().is_empty(),"hidden original back-face metadata is not a current name");
    }
}

#[test]
fn repeated_self_creation_keeps_one_finite_immutable_definition_graph(){
    let id=CardId::new();
    let node=CardDefinitionBuilder::new(id,"Graph Self").card_types(vec![ironsmith::types::CardType::Sorcery])
        .with_spell_effect(vec![creation(id,vec![id],vec![],"Graph Self")]).build();
    let mut current=closed(&[node],0);
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let initial=serde_json::to_vec(&ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(current.clone()).unwrap()).unwrap().len();
    for _ in 0..5 {
        let made=resolve(&mut game,&current);
        assert_eq!(game.object(made).unwrap().kind,ironsmith::object::ObjectKind::Card);
        current=game.object(made).unwrap().physical_card_definition();
        let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(current.clone()).unwrap();
        let bytes=serde_json::to_vec(&wire).unwrap();
        assert_eq!(bytes.len(),initial,"self creation must not recursively embed the catalog");
        current=ironsmith_runtime_catalog::artifact_materializer::materialize_definition(serde_json::from_slice(&bytes).unwrap()).unwrap();
    }
}

#[test]
fn delayed_program_retains_lexical_catalog_after_enclosing_context_ends(){
    let mut pool=nodes();let scope=pool[0].card.id;
    let delayed=ironsmith::effects::ScheduleDelayedTriggerEffect::new(
        ironsmith::triggers::Trigger::beginning_of_end_step(PlayerFilter::Any),
        vec![creation(scope,pool.iter().map(|node|node.card.id).collect(),Vec::new(),"Graph Beta")],
        true,Vec::new(),PlayerFilter::You);
    pool[0].spell_effect=Some(ironsmith_core::ResolutionProgram::from_effects(vec![Effect::new(delayed)]));
    for roundtrip in [false,true]{
        let mut root=closed(&pool,0);
        if roundtrip{let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(root).unwrap();root=ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).unwrap();}
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&root,A,Zone::Stack);
        {
            let mut ctx=EffectContext::new_default(source,A);
            for effect in root.spell_effect.as_ref().unwrap().flattened_default_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
        }
        assert_eq!(game.effect_store.delayed_triggers.len(),1);
        let program=game.effect_store.delayed_triggers[0].effects.clone();
        // A retained body must be self-contained, including when transported to
        // a resolution frame with no original source or enclosing lexical scope.
        let mut fresh=GameState::new(vec!["A".into(),"B".into()],20);
        let source=fresh.new_object_id();let mut ctx=EffectContext::new_default(source,A);
        for effect in program.flattened_default_effects(){execute_effect(&mut fresh,effect,&mut ctx).unwrap();}
        assert_eq!(fresh.player(A).unwrap().hand.len(),1);
        assert_eq!(fresh.object(fresh.player(A).unwrap().hand[0]).unwrap().name.as_str(),"Graph Beta");
    }
}

#[test]
fn independent_lexical_catalogs_can_shadow_equal_local_scope_numbers(){
    let scope=CardId::new();
    let alpha=CardDefinitionBuilder::new(scope,"Lexical Alpha").card_types(vec![ironsmith::types::CardType::Sorcery]).build();
    let foreign=CardDefinitionBuilder::new(scope,"Lexical Foreign").card_types(vec![ironsmith::types::CardType::Sorcery]).build();
    let inner=Effect::new(ironsmith_core::card_creation::CardDefinitionScopeEffect{scope,definitions:vec![foreign],
        effect:Box::new(creation(scope,vec![scope],Vec::new(),"Lexical Foreign"))});
    let sequence=Effect::new(ironsmith::effects::SequenceEffect::new(vec![inner,creation(scope,vec![scope],Vec::new(),"Lexical Alpha")]));
    let outer=Effect::new(ironsmith_core::card_creation::CardDefinitionScopeEffect{scope,definitions:vec![alpha],effect:Box::new(sequence)});
    for roundtrip in [false,true]{
        let effect=if roundtrip{let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(outer.clone()).unwrap();ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap()}else{outer.clone()};
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.new_object_id();
        execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A)).unwrap();
        let names=game.player(A).unwrap().hand.iter().map(|id|game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>();
        assert_eq!(names,vec!["Lexical Foreign","Lexical Alpha"]);
    }
}

#[test]
fn duplicate_admits_an_exact_retained_real_card_outside_seed_catalog(){
    for roundtrip in [false,true]{
        let external=CardDefinitionBuilder::new(CardId::new(),"External retained real card")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .with_spell_effect(vec![Effect::gain_life(7)]).build();
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let target=game.create_object_from_definition(&external,PlayerId(1),Zone::Battlefield);
        let mut effect=Effect::new(CreateCardsEffect {definitions:Vec::<CardDefinition>::new(),definition_scope:None,
            definition_refs:Vec::new(),operation_label:None,
            selection:CardCreationSelection::Duplicate(ironsmith::target::ChooseSpec::SpecificObject(target)),
            count:1.into(),player:PlayerFilter::You,zone:Zone::Hand,library_position:CardCreationLibraryPosition::Top});
        if roundtrip{let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(effect).unwrap();effect=ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();}
        let source=game.new_object_id();let result=execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A)).unwrap();
        let created=game.object(result.output_objects()[0]).unwrap();
        assert_eq!(created.card,Some(external.card.id));assert_eq!(created.owner,A);
        assert_eq!(created.kind,ironsmith::object::ObjectKind::Card);
        let definition=created.physical_card_definition();
        let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).unwrap();
        let definition=ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).unwrap();
        let mut fresh=GameState::new(vec!["A".into(),"B".into()],20);let source=fresh.new_object_id();
        for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects(){execute_effect(&mut fresh,effect,&mut EffectContext::new_default(source,A)).unwrap();}
        assert_eq!(fresh.player(A).unwrap().life,27);
    }
}
#[test]
fn copied_identity_requires_admitted_current_definition_and_never_uses_original_or_copiable_blueprint(){
    for copied in [false,true]{for admitted in [false,true]{
        let original=CardDefinitionBuilder::new(CardId::new(),"Original retained identity").card_types(vec![ironsmith::types::CardType::Artifact]).build();
        let current=CardDefinitionBuilder::new(CardId::new(),"Current admitted identity").card_types(vec![ironsmith::types::CardType::Creature]).build();
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let target=game.create_object_from_definition(&original,A,Zone::Battlefield);
        let donor=game.create_object_from_definition(&current,A,Zone::Battlefield);
        if copied{let snapshot=game.object(donor).unwrap().clone();game.object_mut(target).unwrap().copy_copiable_values_from(&snapshot);}
        else{game.object_mut(target).unwrap().name=current.card.name.clone().into();}
        assert_eq!(game.object(target).unwrap().card,Some(original.card.id));
        let effect=Effect::new(CreateCardsEffect {definitions:if admitted{vec![current.clone()]}else{Vec::new()},definition_scope:None,definition_refs:Vec::new(),
            operation_label:None,selection:CardCreationSelection::Duplicate(ironsmith::target::ChooseSpec::SpecificObject(target)),
            count:1.into(),player:PlayerFilter::You,zone:Zone::Hand,library_position:CardCreationLibraryPosition::Top});
        let source=game.new_object_id();let result=execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A));
        if admitted{let result=result.unwrap();assert_eq!(game.object(result.output_objects()[0]).unwrap().card,Some(current.card.id));}
        else{assert!(matches!(result,Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))),"an unrelated admitted object does not authorize global name lookup");}
    }}
}
