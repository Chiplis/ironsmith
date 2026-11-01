//! Source-backed departure predicate, independently composed with token/counter actions.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn definitions() -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact("Endless Infantry",
                "Mana cost: {2}{B}{G}{W}\nType: Enchantment\nWhenever an Army you control dies, if it wasn't sacrificed, create a 0/0 white Soldier Army creature token. Then put a +1/+1 counter on each Army you control.", false)
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn nonsacrifice_death_triggers_recruit_but_sacrifice_does_not() {
    use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
    for definition in definitions() {
        for sacrificed in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let watcher = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Departing Army", "Type: Creature — Soldier Army\nPower/Toughness: 2/2", false).unwrap();
            let army = game.create_object_from_definition(&body, A, Zone::Battlefield);
            let effect = if sacrificed {
                Effect::new(ironsmith::effects::SacrificeEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You))
            } else {
                Effect::new(ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(army)))
            };
            execute_effect(&mut game, &effect, &mut EffectContext::new_default(watcher, A)).unwrap();
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            // execute_effect captures real zone-change events in the pending
            // queue; do not re-scan its already-consumed outcome event list.
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.len(), usize::from(!sacrificed));
            while !game.stack.is_empty() { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
            let tokens: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|o| o.id != watcher).collect();
            assert_eq!(tokens.len(), usize::from(!sacrificed));
            if !sacrificed {
                let token = tokens[0];
                assert!(token.subtypes.contains(&ironsmith::types::Subtype::Army));
                assert_eq!(game.counter_count(token.id, ironsmith::object::CounterType::PlusOnePlusOne), 1);
                assert_eq!(game.calculated_toughness(token.id), Some(1));
            }
        }
    }
}
