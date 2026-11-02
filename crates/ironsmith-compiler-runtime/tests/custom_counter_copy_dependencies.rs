//! Copied programs retain their own definition dependencies across extraction.
use ironsmith::{GameState,PlayerId,Zone,ObjectId};
use ironsmith::object::{Object,CounterType};
const A:PlayerId=PlayerId(0);
#[test]
fn copy_of_copy_native_snapshot_and_extraction_keep_donor_counter_programs_not_global_registry() {
    let (_,donor)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions(
        "Program donor","Type: Creature\nPower/Toughness: 2/2",&[("borrowed flight".into(),"Flying".into())],false).unwrap();
    let ordinary=ironsmith_compiler_runtime::compile_to_runtime_definition("Physical original","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
    let (_,unrelated)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions(
        "Unrelated vocabulary","Type: Creature\nPower/Toughness: 3/3",&[("unrelated reach".into(),"Reach".into())],false).unwrap();
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let source=game.create_object_from_definition(&ordinary,A,Zone::Battlefield);
    let donor_id=game.create_object_from_definition(&donor,A,Zone::Battlefield);
    game.create_object_from_definition(&unrelated,A,Zone::Battlefield);
    let donor_object=game.object(donor_id).unwrap().clone();
    game.object_mut(source).unwrap().copy_copiable_values_from(&donor_object);
    let copied=game.object(source).unwrap().clone();
    assert!(copied.physical_card_definition().ability_counter_definitions.is_empty(),"the physical card keeps its original source metadata");
    let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(&copied,&game);
    let retained=ironsmith::snapshot::RetainedCopiableValues::from(snapshot.copiable_values.clone());
    let values=ironsmith::snapshot::CopiableValues::from(retained);
    let mut restored_snapshot=snapshot.clone();restored_snapshot.copiable_values=values;
    let candidates=[
        copied.to_card_definition(),
        Object::token_copy_of(&copied,ObjectId(9001),A).to_card_definition(),
        Object::spell_copy_of(&copied,ObjectId(9002),A).to_card_definition(),
        Object::token_copy_from_snapshot(&restored_snapshot,ObjectId(9003),A).to_card_definition(),
    ];
    for candidate in candidates {
        assert_eq!(candidate.ability_counter_definitions.len(),1);
        assert!(candidate.ability_counter_definitions.contains_key("borrowed flight"));
        assert!(!candidate.ability_counter_definitions.contains_key("unrelated reach"));
        let mut rebuilt=GameState::new(vec!["A".into(),"B".into()],20);
        let id=rebuilt.create_object_from_definition(&candidate,A,Zone::Battlefield);
        rebuilt.add_counters(id,CounterType::Named("borrowed flight".into()),1).unwrap();
        assert!(rebuilt.current_abilities(id).unwrap().iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if ability.id()==ironsmith::static_abilities::StaticAbilityId::Flying)));
    }
    assert!(snapshot.copiable_values.definition_dependencies.is_some(),"the native copied snapshot retains its executable dependency witness");
    let public=ironsmith::snapshot::ObjectSnapshot::public_placeholder(snapshot.object_id,snapshot.stable_id,snapshot.owner,snapshot.controller,snapshot.zone);
    assert_eq!((public.object_id,public.stable_id,public.owner,public.controller,public.zone),(snapshot.object_id,snapshot.stable_id,snapshot.owner,snapshot.controller,snapshot.zone));
    assert!(public.is_public_claim_form());
    assert!(public.copiable_values.definition_dependencies.is_none());
}
