//! Exact zone identity differs from the presence of a phased-out source.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_state::StackEntry;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn definitions() -> [CardDefinition; 2] {
    let text = "Type: Creature\nPower/Toughness: 2/2\n{0}: If this object's zone is the battlefield, you gain 2 life. If this creature is still on the battlefield, you gain 3 life.";
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Zone witness", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn exact_zone_includes_phased_out_source_and_never_follows_a_new_incarnation() {
    for definition in definitions() { for state in 0..4 {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { ironsmith::ability::AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
        let mut entry = StackEntry::ability(source, A, ability.effects.clone());
        entry.source_snapshot = Some(ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game));
        game.push_to_stack(entry);
        if state == 1 { game.phase_out(source); }
        if state >= 2 {
            execute_effect(&mut game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Graveyard, false)), &mut EffectContext::new_default(source, A)).unwrap();
        }
        if state == 3 {
            let grave = game.find_object_by_stable_id(stable).unwrap();
            execute_effect(&mut game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(grave), Zone::Battlefield, false)), &mut EffectContext::new_default(source, A)).unwrap();
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().life, match state { 0 => 25, 1 => 22, _ => 20 });
    } }
}
