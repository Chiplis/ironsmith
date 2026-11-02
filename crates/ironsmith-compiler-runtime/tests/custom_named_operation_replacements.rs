//! Replacement proposals precede named programs; completion is a separate receipt.
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::types::Subtype;
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const PROGRAM: &str = "Perform the germinate action as follows: You may put a +1/+1 counter on a creature; If you don't, create a 1/1 green Saproling creature token.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact(name, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
struct Choices { original_allowed: bool, accept: bool, booleans: usize }
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
        assert!(self.original_allowed, "the replaced original program must not ask its optional decision");
        self.booleans += 1;
        self.accept
    }
    fn decide_objects(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        context.candidates.iter().filter(|object| object.legal).take(context.min).map(|object| object.id).collect()
    }
}
fn counters(game: &GameState) -> u32 { game.battlefield.iter().map(|id| game.counter_count(*id, CounterType::PlusOnePlusOne)).sum() }
fn saprolings(game: &GameState) -> usize { game.battlefield.iter().filter(|id| game.object(**id).unwrap().subtypes.contains(&Subtype::Saproling)).count() }

#[test]
fn named_proposal_replaces_whole_original_program_without_emitting_original_completion() {
    let replacement = "Mana cost: {1}{W}\nType: Creature — Fungus Scout\nPower/Toughness: 2/2\nIf you would perform the germinate action, instead create a 1/1 green Saproling creature token, then put a +1/+1 counter on a creature you control.\n{2}{W}{W}: Perform the germinate action as follows: You may put a +1/+1 counter on a creature; If you don't, create a 1/1 green Saproling creature token.";
    let replacement_definitions = definitions("Ontarachie Vagabond", replacement);
    for (index, program) in definitions("Independent action source", &format!("Mana cost: {{0}}\nType: Sorcery\n{PROGRAM}")).into_iter().enumerate() {
        for actor in [A, B] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.create_object_from_definition(&replacement_definitions[index], A, Zone::Battlefield);
            let source = game.create_object_from_definition(&program, actor, Zone::Stack);
            let mut choices = Choices { original_allowed: actor != A, accept: true, booleans: 0 };
            let mut context = EffectContext::new_default(source, actor).with_decision_maker(&mut choices);
            let mut completions = 0;
            for effect in program.spell_effect.as_ref().unwrap().all_effects() {
                let outcome = execute_effect(&mut game, effect, &mut context).unwrap();
                completions += outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::NamedActionEvent>().is_some_and(|action| action.label == "germinate")).count();
                assert!(!outcome.events.iter().any(|event| event.downcast::<ironsmith::events::other::NamedActionProposalEvent>().is_some()), "proposals are not completed history");
            }
            drop(context);
            assert_eq!(counters(&game), 1, "only one program may put its counter");
            if actor == A {
                assert_eq!(saprolings(&game), 1);
                assert_eq!(choices.booleans, 0);
                assert_eq!(completions, 0, "replacement did not perform the original named operation");
            } else {
                assert_eq!(saprolings(&game), 0, "your replacement does not replace an opponent's operation");
                assert_eq!(choices.booleans, 1);
                assert_eq!(completions, 1);
            }
        }
    }
}

#[test]
fn declined_internal_branch_keeps_fallback_inside_one_completed_named_operation() {
    for program in definitions("Independent action source", &format!("Mana cost: {{0}}\nType: Sorcery\n{PROGRAM}")) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&program, A, Zone::Stack);
        let mut choices = Choices { original_allowed: true, accept: false, booleans: 0 };
        let mut context = EffectContext::new_default(source, A).with_decision_maker(&mut choices);
        let mut completions = 0;
        for effect in program.spell_effect.as_ref().unwrap().all_effects() {
            let outcome = execute_effect(&mut game, effect, &mut context).unwrap();
            completions += outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::NamedActionEvent>().is_some_and(|action| action.label == "germinate")).count();
        }
        assert_eq!(saprolings(&game), 1);
        assert_eq!(completions, 1);
    }
}

#[test]
fn prevented_named_proposal_never_runs_or_completes_the_original_program() {
    for program in definitions("Independent action source", &format!("Mana cost: {{0}}\nType: Sorcery\n{PROGRAM}")) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Body", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        game.create_object_from_definition(&body, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&program, A, Zone::Stack);
        game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            source, A, ironsmith::events::other::WouldNamedActionMatcher { object_requirement: None,
                label: "germinate".into(), source_filter: ironsmith::target::ObjectFilter::default(),
                performer_filter: Some(ironsmith::target::PlayerFilter::You),
            }, ironsmith::replacement::ReplacementAction::Prevent,
        ));
        let mut choices = Choices { original_allowed: false, accept: true, booleans: 0 };
        let mut context = EffectContext::new_default(source, A).with_decision_maker(&mut choices);
        for effect in program.spell_effect.as_ref().unwrap().all_effects() {
            let outcome = execute_effect(&mut game, effect, &mut context).unwrap();
            assert!(!outcome.events.iter().any(|event| event.downcast::<ironsmith::events::NamedActionEvent>().is_some()));
        }
        assert_eq!(counters(&game), 0);
        assert_eq!(saprolings(&game), 0);
    }
}
