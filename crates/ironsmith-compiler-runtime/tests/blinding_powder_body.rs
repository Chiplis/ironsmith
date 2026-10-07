//! Frozen full-body regressions, source-authored and UNRUN during the campaign pause.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{compute_legal_actions, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext};
use ironsmith::effects::ChooseObjectsEffect;
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::game_loop::{apply_decision_context_with_dm, apply_priority_response_with_dm,
    resolve_stack_entry_with, PriorityLoopState, PriorityResponse};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::AttachmentTarget;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const BODY: &str = "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature has \"Unattach Blinding Powder: Prevent all combat damage that would be dealt to this creature this turn.\"\nEquip {2} ({2}: Attach to target creature you control. Equip only as a sorcery.)";

fn definitions() -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition("Blinding Powder", BODY, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact("Blinding Powder", BODY, false));
    let (artifact, _) = compiled.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &materialized] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, materialized]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}

fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), "Powder recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 8)).build(), owner, Zone::Battlefield)
}

#[derive(Default)]
struct Choices { target: Option<Target>, unattach: Option<ObjectId>, target_calls: usize }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.target_calls += 1;
        if let Some(target) = self.target {
            assert!(context.requirements.iter().all(|requirement| requirement.legal_targets.contains(&target)));
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(expected) = self.unattach {
            let legal = context.candidates.iter().filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id).collect::<Vec<_>>();
            assert_eq!(legal, vec![expected], "only the granting Powder may pay its cost");
            vec![expected]
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}

fn unattach_index(game: &GameState, host: ObjectId, powder: ObjectId) -> Option<usize> {
    game.current_abilities(host)?.iter().enumerate().find_map(|(index, ability)| {
        let AbilityKind::Activated(activated) = &ability.kind else { return None; };
        activated.mana_cost.costs().iter().filter_map(|cost| cost.effect_ref()).any(|effect| {
            let mut effect = effect;
            while let Some(inner) = effect.transparent_child_effect() { effect = inner; }
            effect.downcast_ref::<ChooseObjectsEffect>()
                .is_some_and(|choose| choose.filter.specific == Some(powder))
        }).then_some(index)
    })
}

fn activate(game: &mut GameState, source: ObjectId, index: usize, choices: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, ability_index }
            if *id == source && *ability_index == index)).expect("native activation must be legal");
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..32 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    assert!(!state.has_pending_action());
    assert_eq!(game.stack.len(), 1);
}

fn damage(game: &mut GameState, source: ObjectId, target: ObjectId, combat: bool, unpreventable: bool) -> (u32, u32) {
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(game, source,
        DamageTarget::Object(target), 3, combat, unpreventable, EventCause::effect(), None).unwrap();
    let remaining = result.assignments.iter().map(|assignment| assignment.amount).sum();
    let prevented = game.take_pending_trigger_events().iter()
        .filter_map(|event| event.downcast::<DamagePreventedEvent>().map(|event| event.amount)).sum();
    (remaining, prevented)
}

#[test]
fn full_powder_body_keeps_equip_cost_target_and_sorcery_timing() {
    for definition in definitions() {
        let mut game = game();
        let host = creature(&mut game, A);
        let powder = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let index = game.current_abilities(powder).unwrap().iter().position(|ability|
            matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        game.turn.phase = ironsmith::Phase::Combat;
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateAbility { source, ability_index }
                if *source == powder && *ability_index == index)));
        game.turn.phase = ironsmith::Phase::FirstMain;
        let mut choices = Choices { target: Some(Target::Object(host)), ..Default::default() };
        activate(&mut game, powder, index, &mut choices);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.object(powder).unwrap().attached_to, Some(AttachmentTarget::Object(host)));
        assert_eq!(choices.target_calls, 1);
        assert!(unattach_index(&game, host, powder).is_some());
    }
}

#[test]
fn each_powder_cost_uses_its_granter_even_under_another_players_control() {
    for definition in definitions() {
        for foreign_controller in [false, true] {
            for aftermath in ["detached", "granter left", "reattached", "host control changed", "host blinked"] {
                let mut game = game();
                let host = creature(&mut game, A);
                let other_host = creature(&mut game, A);
                let source = creature(&mut game, B);
                let powder = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let other_powder = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                for equipment in [powder, other_powder] {
                    assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(host)));
                }
                if foreign_controller { game.set_current_controller(powder, B).unwrap(); }
                let index = unattach_index(&game, host, powder).unwrap();
                let mut choices = Choices { unattach: Some(powder), ..Default::default() };
                activate(&mut game, host, index, &mut choices);
                assert_eq!(game.object(powder).unwrap().attached_to, None);
                assert_eq!(game.object(other_powder).unwrap().attached_to, Some(AttachmentTarget::Object(host)));
                assert!(!game.is_tapped(host));
                assert!(game.stack.last().unwrap().targets.is_empty());
                assert_eq!(choices.target_calls, 0);
                assert!(unattach_index(&game, host, powder).is_none());
                let mut recipient = host;
                match aftermath {
                    "granter left" => { game.move_object_by_effect(powder, Zone::Graveyard).unwrap(); }
                    "reattached" => { assert!(game.attach_object_to_target(powder, AttachmentTarget::Object(other_host))); }
                    "host control changed" => { game.set_current_controller(host, B).unwrap(); }
                    "host blinked" => {
                        let stable = game.object(host).unwrap().stable_id;
                        game.move_object_by_effect(host, Zone::Exile).unwrap();
                        let exiled = game.find_object_by_stable_id(stable).unwrap();
                        game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
                        recipient = game.find_object_by_stable_id(stable).unwrap();
                        assert_ne!(recipient, host);
                    }
                    _ => {}
                }
                game = game.clone();
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                let expected = if aftermath == "host blinked" { (3, 0) } else { (0, 3) };
                assert_eq!(damage(&mut game, source, recipient, true, false), expected, "{aftermath}");
                assert_eq!(damage(&mut game, source, other_host, true, false), (3, 0));
                assert_eq!(damage(&mut game, source, recipient, false, false), (3, 0));
                assert_eq!(damage(&mut game, source, recipient, true, true), (3, 0));
                game.effect_store.prevention_effects.cleanup_end_of_turn();
                assert_eq!(damage(&mut game, source, recipient, true, false), (3, 0));
            }
        }
    }
}
