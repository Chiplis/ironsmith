//! Existing phasing composed with resolution-time stack targeting and blocking predicates.
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const ACTION: &str = "Choose target creature. If that creature is a nonblocking permanent that isn't the target of a spell or ability on the stack, give that creature the voyaging status until it phases in, then it phases out.";
fn definitions() -> [CardDefinition; 2] {
    let text = format!("Type: Creature\nPower/Toughness: 2/2\n{{0}}: {ACTION}");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Voyage operation", &text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn phasing_condition_checks_other_stack_entries_and_current_blocking_at_resolution() {
    for definition in definitions() { for blocking in [false, true] { for other_targeting in [false, true] { for other_is_ability in [false, true] { for same_source in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Voyage recipient", "Type: Creature\nPower/Toughness: 3/3", false).unwrap();
        let recipient = game.create_object_from_definition(&body, A, Zone::Battlefield);
        let irrelevant = game.create_object_from_definition(&body, B, Zone::Battlefield);
        if blocking {
            let mut combat = ironsmith::combat_state::CombatState::default();
            combat.blockers.insert(irrelevant, vec![recipient]);
            game.combat = Some(combat);
        }
        let mut lower = if other_is_ability {
            StackEntry::ability(if same_source { source } else { irrelevant }, B, Vec::<ironsmith::Effect>::new())
        } else {
            let spell = ironsmith_compiler_runtime::compile_to_runtime_definition("Other spell", "Type: Instant\nYou gain 1 life.", false).unwrap();
            let id = game.create_object_from_definition(&spell, B, Zone::Stack);
            StackEntry::new(id, B)
        };
        lower.targets = vec![Target::Object(if other_targeting { recipient } else { irrelevant })];
        game.push_to_stack(lower);
        let activation = definition.abilities.iter().find_map(|a| match &a.kind { ironsmith::ability::AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
        game.push_to_stack(StackEntry::ability(source, A, activation.effects.clone()).with_targets(vec![Target::Object(recipient)]));
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let allowed = !blocking && !other_targeting;
        assert_eq!(game.is_phased_out(recipient), allowed, "blocking={blocking} other_targeting={other_targeting} other_is_ability={other_is_ability} same_source={same_source}");
        assert_eq!(game.has_custom_status(recipient, "voyaging"), allowed);
        assert_eq!(game.stack.len(), 1, "the resolving ability is excluded because it was popped; other source-identical abilities remain relevant");
        if allowed {
            game.phase_in(recipient);
            assert!(!game.has_custom_status(recipient, "voyaging"));
        }
    } } } } }
}
