//! One authored amount damages the complete union in one existing damage batch.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
const A:PlayerId=PlayerId::from_index(0);const B:PlayerId=PlayerId::from_index(1);
fn definitions(text:&str)->[CardDefinition;2]{
    let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Union damage",text,false)));
    let(artifact,direct)=result.unwrap();assert!(!loss.is_lossy(),"{}",loss.reasons_text());
    let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn mixed_inherited_each_list_damages_players_creatures_and_planeswalkers_in_one_batch(){
    for source in definitions("Type: Sorcery\nUnion damage deals 1 damage to each creature, player, and planeswalker."){
        let mut g=GameState::new(vec!["A".into(),"B".into()],20);
        let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Creature","Type: Creature\nPower/Toughness: 4/4",false).unwrap();
        let walker=ironsmith_compiler_runtime::compile_to_runtime_definition("Walker","Type: Planeswalker\nLoyalty: 4",false).unwrap();
        let body=g.create_object_from_definition(&creature,A,Zone::Battlefield);let pw=g.create_object_from_definition(&walker,B,Zone::Battlefield);
        let spell=g.create_object_from_definition(&source,A,Zone::Stack);let mut ctx=ironsmith::effects::EffectContext::new_default(spell,A);
        let effects=source.spell_effect.as_ref().unwrap().all_effects();assert_eq!(effects.len(),1,"recipient union must remain one executable instruction");
        let outcome=ironsmith::effects::execute_effect(&mut g,effects[0],&mut ctx).unwrap();
        let events=outcome.events.iter().filter(|event|event.downcast::<ironsmith::events::DamageEvent>().is_some()).collect::<Vec<_>>();
        assert_eq!(events.len(),4);let batch=events[0].simultaneous_batch();assert!(batch.is_some());assert!(events.iter().all(|event|event.simultaneous_batch()==batch));
        assert_eq!(g.player(A).unwrap().life,19);assert_eq!(g.player(B).unwrap().life,19);assert_eq!(g.damage_on(body),1);assert_eq!(g.counter_count(pw,ironsmith::object::CounterType::Loyalty),3);
    }
}
#[test]
fn overlapping_object_groups_are_deduplicated_and_nonrecipients_are_excluded(){
    for source in definitions("Type: Sorcery\nThis spell deals 2 damage to each creature, permanent, and player."){
        let mut g=GameState::new(vec!["A".into(),"B".into()],20);
        let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Creature","Type: Creature\nPower/Toughness: 4/4",false).unwrap();
        let artifact=ironsmith_compiler_runtime::compile_to_runtime_definition("Relic","Type: Artifact",false).unwrap();
        let body=g.create_object_from_definition(&creature,A,Zone::Battlefield);let relic=g.create_object_from_definition(&artifact,A,Zone::Battlefield);let hand=g.create_object_from_definition(&creature,A,Zone::Hand);
        let spell=g.create_object_from_definition(&source,A,Zone::Stack);let mut ctx=ironsmith::effects::EffectContext::new_default(spell,A);
        let effects=source.spell_effect.as_ref().unwrap().all_effects();assert_eq!(effects.len(),1);
        let outcome=ironsmith::effects::execute_effect(&mut g,effects[0],&mut ctx).unwrap();
        let events=outcome.events.iter().filter_map(|event|event.downcast::<ironsmith::events::DamageEvent>()).collect::<Vec<_>>();
        assert_eq!(events.len(),3,"one creature and two players; ordinary artifacts cannot be dealt damage");assert_eq!(g.damage_on(body),2);assert_eq!(g.damage_on(relic),0);assert_eq!(g.damage_on(hand),0);assert_eq!(g.player(B).unwrap().life,18);
    }
}
