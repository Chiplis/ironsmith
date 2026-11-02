use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_core::trigger_model::{Trigger,TriggerKind};
fn has_three_creature_attack(trigger:&Trigger)->bool {
    match &trigger.kind {
        TriggerKind::AttacksOneOrMoreWithMinTotal {filter,min_total_attackers,per_attacking_player} => *min_total_attackers==3 && !*per_attacking_player && filter.controller==Some(ironsmith::target::PlayerFilter::You),
        TriggerKind::Either {left,right} => has_three_creature_attack(left)||has_three_creature_attack(right),
        _=>false,
    }
}
#[test]
fn attack_group_is_preserved_inside_an_alternative_event_with_artifact_parity() {
    let text="Type: Creature — Fox\nPower/Toughness: 2/2\nWhenever Guide enters or you attack with three or more creatures, draw a card.";
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Guide",text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());
    let (artifact,direct)=result.unwrap();
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let materialized=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [direct,materialized] {
        let trigger=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger),_=>None}).unwrap();
        let model=trigger.trigger.compiled_model().unwrap();
        assert!(matches!(&model.kind,TriggerKind::Either{..}));
        assert!(has_three_creature_attack(model));
    }
}
