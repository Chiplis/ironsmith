//! Source-backed shared-subject event lists, independently composed with actions.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn definitions() -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact("Major Benjamin Stout",
                "Mana cost: {3}{W}{W}\nType: Legendary Creature — Rabbit Soldier\nPower/Toughness: 2/4\nDouble strike, vigilance\nWhenever Major Benjamin Stout enters, attacks, or dies, create a 0/0 white Soldier Army creature token. Then put a +1/+1 counter on each Army you control.", false)
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn entry_attack_and_death_each_execute_the_complete_shared_action_program() {
    for definition in definitions() {
        for event in ["entry", "attack", "death"] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let original = game.create_object_from_definition(&definition, A, if event == "entry" {Zone::Hand} else {Zone::Battlefield});
            let stable = game.object(original).unwrap().stable_id;
            match event {
                "entry" => { execute_effect(&mut game,
                    &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(original), Zone::Battlefield, false)),
                    &mut EffectContext::new_default(original, A)).unwrap(); }
                "attack" => { game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
                    ironsmith::events::CreatureAttackedEvent::new(original, ironsmith::events::AttackEventTarget::Player(PlayerId(1))), Default::default())); }
                "death" => { execute_effect(&mut game,
                    &Effect::new(ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(original))),
                    &mut EffectContext::new_default(original, A)).unwrap(); }
                _ => unreachable!(),
            }
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.len(), 1, "exactly one trigger for {event}");
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let armies: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|o| o.subtypes.contains(&ironsmith::types::Subtype::Army)).collect();
            assert_eq!(armies.len(), 1, "{event} must create the token");
            assert_eq!(game.counter_count(armies[0].id, ironsmith::object::CounterType::PlusOnePlusOne), 1);
            assert_eq!(game.calculated_toughness(armies[0].id), Some(1));
            assert!(game.stack.is_empty());
            let source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(source).unwrap().zone, if event == "death" {Zone::Graveyard} else {Zone::Battlefield});
        }
    }
}
