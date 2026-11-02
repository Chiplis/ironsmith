//! Deferred named-operation additions bind actual subjects independently of source.
use ironsmith::{Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
const A: PlayerId = PlayerId(0);
#[test]
fn named_additions_retain_exact_completed_subject_and_performer_with_authoritative_empty_case() {
    for empty_subjects in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Body", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let source = game.create_object_from_definition(&body, A, Zone::Battlefield);
        let subject = game.create_object_from_definition(&body, A, Zone::Battlefield);
        let unrelated = game.create_object_from_definition(&body, A, Zone::Battlefield);
        game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            source, A, ironsmith::events::other::WouldNamedActionMatcher { object_requirement: None,
                label: "assess".into(), source_filter: ObjectFilter::specific(source), performer_filter: Some(PlayerFilter::You),
            }, ironsmith::replacement::ReplacementAction::Additionally(vec![
                Effect::put_counters(CounterType::PlusOnePlusOne, 1, ChooseSpec::Tagged("it".into())),
                Effect::put_counters(CounterType::PlusOnePlusOne, 1, ChooseSpec::Tagged("__named_action_source".into())),
            ]),
        ));
        let program = Effect::new(ironsmith::effects::NamedActionProgramEffect {
            label: "assess".into(), player: PlayerFilter::You,
            completion: ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,
            objects: Some(if empty_subjects { ChooseSpec::Tagged("empty".into()) } else { ChooseSpec::SpecificObject(subject) }),
            effects: vec![if empty_subjects { Effect::gain_life(0) } else { Effect::tap(ChooseSpec::SpecificObject(subject)) }],
        });
        let mut ctx = EffectContext::new_default(source, A);
        ctx.tagged_objects.insert("empty".into(), vec![]);
        let result = execute_effect(&mut game, &program, &mut ctx).unwrap();
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1, "performer is an independent exact reference");
        assert_eq!(game.counter_count(subject, CounterType::PlusOnePlusOne), u32::from(!empty_subjects));
        assert_eq!(game.counter_count(unrelated, CounterType::PlusOnePlusOne), 0);
        assert_eq!(result.events.iter().filter(|event| event.downcast::<ironsmith::events::NamedActionEvent>().is_some()).count(), 1, "additions cannot manufacture another original action receipt");
    }
}
