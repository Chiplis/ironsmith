//! Completion is an explicit program boundary or an owning accepted payment.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::DecisionMaker;
use ironsmith::effect::EffectPredicateRuntimeExt;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
struct Decision { accept: bool, calls: usize }
impl DecisionMaker for Decision {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.calls += 1; self.accept }
    fn decide_objects(&mut self, _: &GameState, _: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> { vec![] }
}
fn pairs(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact("Named operation", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn receipts(outcome: &ironsmith::effect::EffectOutcome) -> usize {
    outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::NamedActionEvent>().is_some()).count()
}
#[test]
fn legal_zero_result_program_completes_once_and_outer_decline_does_not_invoke_it() {
    for body in ["Create zero 1/1 green Saproling creature tokens", "Choose up to one creature you control"] {
    for definition in pairs(&format!("Type: Sorcery\nPerform the assess action as follows: {body}.")) {
        for outer_optional in [false, true] { for accepted in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Stack);
            let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
            let effect = if outer_optional { Effect::new(ironsmith::effects::MayEffect::new(effects)) }
                else { Effect::new(ironsmith::effects::SequenceEffect::new(effects)) };
            let mut decision = Decision { accept: accepted, calls: 0 };
            let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut decision);
            let outcome = execute_effect(&mut game, &effect, &mut ctx).unwrap();
            assert_eq!(game.battlefield.len(), 0);
            assert_eq!(outcome.events.len(), receipts(&outcome), "legal zero-result body emits no material-action event");
            assert_eq!(receipts(&outcome), usize::from(!outer_optional || accepted));
            drop(ctx);
            assert_eq!(decision.calls, usize::from(outer_optional));
        }}
    }
    }
}
#[test]
fn paid_operation_uses_accepted_zero_payment_and_retains_primary_result_for_followups() {
    let source_definition = pairs("Type: Artifact")[0].clone();
    for accepted in [false, true] { for export_empty_objects in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&source_definition, A, Zone::Battlefield);
        let effect = Effect::new(ironsmith::effects::NamedActionProgramEffect {
            label: "fund".into(), player: PlayerFilter::You,
            completion: ironsmith_core::effect::NamedActionProgramCompletion::AcceptedPayment,
            objects: export_empty_objects.then(|| ChooseSpec::Tagged("empty_chosen".into())),
            effects: vec![Effect::new(ironsmith::effects::MayEffect::new(vec![Effect::pay_life(0)]).with_pay_as_cost(true))],
        });
        let mut decision = Decision { accept: accepted, calls: 0 };
        let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut decision);
        ctx.tagged_objects.insert("empty_chosen".into(), vec![]);
        let outcome = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(outcome.execution_facts().contains(&ironsmith::effect::ExecutionFact::Accepted), accepted);
        assert_eq!(ironsmith::effect::EffectPredicate::Happened.evaluate_outcome(&outcome), accepted);
        assert_eq!(outcome.instruction_result().value.as_count().unwrap_or(0), 0, "receipt count must not replace original zero payment result");
        assert_eq!(receipts(&outcome), usize::from(accepted));
        drop(ctx);
        assert_eq!(decision.calls, 1);
    }}
}
#[test]
fn payment_policy_refuses_unrelated_optional_acceptance_as_its_owner() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source_definition = pairs("Type: Artifact")[0].clone();
    let source = game.create_object_from_definition(&source_definition, A, Zone::Battlefield);
    let effect = Effect::new(ironsmith::effects::NamedActionProgramEffect {
        label: "fund".into(), player: PlayerFilter::You,
        completion: ironsmith_core::effect::NamedActionProgramCompletion::AcceptedPayment, objects: None,
        effects: vec![Effect::new(ironsmith::effects::MayEffect::new(vec![Effect::gain_life(3)]))],
    });
    let mut ctx = EffectContext::new_default(source, A);
    assert!(execute_effect(&mut game, &effect, &mut ctx).is_err());
    assert_eq!(game.player(A).unwrap().life, 20, "invalid policy cannot execute unrelated body");
}
#[test]
fn suspended_internal_decision_cannot_publish_completion_or_partial_work() {
    struct Suspend { pending: bool }
    impl DecisionMaker for Suspend {
        fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.pending = true; true }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    for definition in pairs("Type: Sorcery\nPerform the assess action as follows: You may gain 1 life.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut decision = Suspend { pending: false };
        let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut decision);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            let outcome = execute_effect(&mut game, effect, &mut ctx).unwrap();
            assert_eq!(receipts(&outcome), 0);
        }
        assert_eq!(game.player(A).unwrap().life, 20);
        drop(ctx);
        assert!(decision.pending);
    }
}

#[test]
fn optional_choice_count_operands_are_read_independently_of_their_enclosing_program() {
    for body in [
        "Choose up to one creature you control.",
        "Choose up to two artifacts you control.",
        "You may do the following: Choose up to one creature you control.",
        "Perform the tally action as follows: Choose up to two artifacts you control.",
    ] {
        for definition in pairs(&format!("Type: Sorcery\n{body}")) {
            assert!(definition.spell_effect.is_some(),"the standalone choice and surrounding program both retain executable structure");
        }
    }
}
