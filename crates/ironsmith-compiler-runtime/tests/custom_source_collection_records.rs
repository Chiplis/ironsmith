//! Explicit relationship primitive composed with existing static collection filters.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,ResolvedTarget,execute_effect};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(text:&str)->[CardDefinition;2]{
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Source collection witness",text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
    let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn move_to(game:&mut GameState,id:ironsmith::ObjectId,zone:Zone)->ironsmith::ObjectId{
    let stable=game.object(id).unwrap().stable_id;
    execute_effect(game,&ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(id),zone,false)),&mut EffectContext::new_default(id,A)).unwrap();game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn parsed_record_and_static_collection_consumer_keep_exact_source_and_host(){
    let actions=definitions("Type: Sorcery\nRecord target creature in the scripted host collection of this card while that source remains in exile and those members remain on the battlefield.");
    let sources=definitions("Type: Artifact\nAs long as this card is in exile, creatures from the scripted host collection get +2/+1.");
    let hosts=definitions("Type: Creature\nPower/Toughness: 2/2");
    for index in 0..2{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&sources[index],A,Zone::Exile);let host=game.create_object_from_definition(&hosts[index],A,Zone::Battlefield);let other=game.create_object_from_definition(&hosts[index],A,Zone::Battlefield);
        let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(host)]);
        for effect in actions[index].spell_effect.as_ref().unwrap().all_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
        assert_eq!(game.source_collection_objects(source,"scripted host"),vec![host]);
        assert_eq!(game.calculated_characteristics(host).unwrap().power,Some(4));assert_eq!(game.calculated_characteristics(other).unwrap().power,Some(2));
        game.set_current_controller(host,B).unwrap();assert_eq!(game.source_collection_objects(source,"scripted host"),vec![host]);assert_eq!(game.calculated_characteristics(host).unwrap().power,Some(4));
        let returned=move_to(&mut game,host,Zone::Hand);let returned=move_to(&mut game,returned,Zone::Battlefield);assert_eq!(game.calculated_characteristics(returned).unwrap().power,Some(2));assert!(game.source_collection_objects(source,"scripted host").is_empty());
    }
}
#[test]
fn source_departure_and_reexile_does_not_restore_an_old_relationship(){
    let sources=definitions("Type: Artifact\nAs long as this card is in exile, creatures from the scripted host collection get +2/+1.");let hosts=definitions("Type: Creature\nPower/Toughness: 2/2");
    for index in 0..2{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&sources[index],A,Zone::Exile);let host=game.create_object_from_definition(&hosts[index],A,Zone::Battlefield);
        let effect=ironsmith::Effect::new(ironsmith_core::RecordSourceCollectionEffect{target:ironsmith::target::ChooseSpec::SpecificObject(host),source_tag:None,name:"scripted host".into(),source_zone:Some(Zone::Exile),member_zone:Zone::Battlefield});execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A)).unwrap();assert_eq!(game.calculated_characteristics(host).unwrap().power,Some(4));
        let hand=move_to(&mut game,source,Zone::Hand);let exile=move_to(&mut game,hand,Zone::Exile);assert_ne!(source,exile);assert_eq!(game.calculated_characteristics(host).unwrap().power,Some(2));assert!(game.source_collection_objects(source,"scripted host").is_empty());assert!(game.source_collection_objects(exile,"scripted host").is_empty());
    }
}
#[test]
fn missing_or_wrong_zone_exact_owner_cannot_record_member(){
    let body=definitions("Type: Creature\nPower/Toughness: 2/2");
    for definition in body{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Hand);let host=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        for source_tag in [None,Some("missing".into())]{let effect=ironsmith::Effect::new(ironsmith_core::RecordSourceCollectionEffect{target:ironsmith::target::ChooseSpec::SpecificObject(host),source_tag,name:"scripted host".into(),source_zone:Some(Zone::Exile),member_zone:Zone::Battlefield});execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A)).unwrap();}
        assert!(game.source_collection_objects(source,"scripted host").is_empty());
    }
}
#[test]
fn death_consumer_looks_back_at_exact_departed_membership(){
    let sources=definitions("Type: Sorcery\nYou gain 1 life.\nWhen a creature from the scripted host collection dies, if this card is in exile, you gain 2 life.");let hosts=definitions("Type: Creature\nPower/Toughness: 2/2");
    for index in 0..2{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&sources[index],A,Zone::Exile);let host=game.create_object_from_definition(&hosts[index],A,Zone::Battlefield);
        let effect=ironsmith::Effect::new(ironsmith_core::RecordSourceCollectionEffect{target:ironsmith::target::ChooseSpec::SpecificObject(host),source_tag:None,name:"scripted host".into(),source_zone:Some(Zone::Exile),member_zone:Zone::Battlefield});execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A)).unwrap();
        move_to(&mut game,host,Zone::Graveyard);assert!(game.source_collection_objects(source,"scripted host").is_empty());
        let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert_eq!(game.stack.len(),1,"departing exact host was linked immediately before it died");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,22);
    }
}
#[test]
fn local_exile_receipt_owns_record_after_member_target_rebinding(){
    let sources=definitions("Type: Artifact\n{0}: Choose a creature you control. If you do, exile this card from your hand. If you do, record that creature in the scripted host collection of the exiled card while that source remains in exile and those members remain on the battlefield. Activate only if this card is in your hand.");let hosts=definitions("Type: Creature\nPower/Toughness: 2/2");
    for index in 0..2{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&sources[index],A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;let host=game.create_object_from_definition(&hosts[index],A,Zone::Battlefield);
        let activated=sources[index].abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Activated(ability)=>Some(ability),_=>None}).unwrap();
        game.push_to_stack(ironsmith::StackEntry::ability(source,A,activated.effects.clone()));let mut dm=ironsmith::decision::SelectFirstDecisionMaker;ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let exile=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exile).unwrap().zone,Zone::Exile);assert_eq!(game.source_collection_objects(exile,"scripted host"),vec![host]);assert!(game.source_collection_objects(host,"scripted host").is_empty(),"member target must never replace exact source receipt");
    }
}
#[test]
fn nested_local_loop_restores_outer_iterated_owner_and_two_sources_stay_independent(){
    let actions=definitions("Type: Sorcery\nFor each card in exile, do the following: [For each creature you control, do the following: [Choose a creature you control; Record that creature in the inner host collection of the iterated object while that source remains on the battlefield and those members remain on the battlefield]; Choose a creature you control; Record that creature in the outer host collection of the iterated card while that source remains in exile and those members remain on the battlefield].");
    let sources=definitions("Type: Artifact");let hosts=definitions("Type: Creature\nPower/Toughness: 2/2");
    for index in 0..2{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let first=game.create_object_from_definition(&sources[index],A,Zone::Exile);let second=game.create_object_from_definition(&sources[index],A,Zone::Exile);let host=game.create_object_from_definition(&hosts[index],A,Zone::Battlefield);let spell=game.create_object_from_definition(&actions[index],A,Zone::Stack);
        game.push_to_stack(ironsmith::StackEntry::new(spell,A));let mut dm=ironsmith::decision::SelectFirstDecisionMaker;ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(game.source_collection_objects(first,"outer host"),vec![host]);assert_eq!(game.source_collection_objects(second,"outer host"),vec![host]);assert_eq!(game.source_collection_objects(host,"inner host"),vec![host]);assert!(game.source_collection_objects(host,"outer host").is_empty());
        let hand=move_to(&mut game,first,Zone::Hand);let reexiled=move_to(&mut game,hand,Zone::Exile);assert!(game.source_collection_objects(first,"outer host").is_empty());assert!(game.source_collection_objects(reexiled,"outer host").is_empty());assert_eq!(game.source_collection_objects(second,"outer host"),vec![host]);
    }
}
