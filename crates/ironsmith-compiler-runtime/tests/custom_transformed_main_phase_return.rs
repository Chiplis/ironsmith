use ironsmith_compiled_artifact::CompiledCardArtifact;
#[test]
fn transformed_return_and_main_phase_schedule_are_independent_structured_operations() {
    let text="Type: Creature — Fox\nPower/Toughness: 2/2\n{1}, Exile Guide: Return Guide to the battlefield transformed at the beginning of your next main phase.";
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Guide",text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());
    let (artifact,direct)=result.unwrap();
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let materialized=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [direct,materialized] {
        let activated=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Activated(activated)=>Some(activated),_=>None}).unwrap();
        let effects=activated.effects.all_effects_owned();
        let schedule=effects.iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>()).unwrap();
        assert!(schedule.one_shot);
        assert!(!schedule.start_next_turn,"the next main phase can be later in the same turn");
        assert!(schedule.effects.all_effects_owned().iter().any(|effect| effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>().is_some_and(|moved| moved.enters_transformed && moved.zone==ironsmith::Zone::Battlefield)));
    }
}
