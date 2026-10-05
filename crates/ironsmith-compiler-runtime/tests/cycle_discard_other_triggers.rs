use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::effects::{DiscardEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{KeywordActionKind, ObjectFilter, PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/cycle_discard_other_triggers.json.fixture"
    ))
    .unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    [direct, registry.get(name).unwrap().clone()]
}

#[test]
fn cycle_discard_other_triggers_all_three_full_cards_preserve_both_filters_after_round_trip() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 3);
    for fixture in fixtures {
        for definition in definitions(
            fixture["name"].as_str().unwrap(),
            fixture["text"].as_str().unwrap(),
        ) {
            let (left, right) = definition
                .abilities
                .iter()
                .find_map(|ability| {
                    let AbilityKind::Triggered(triggered) = &ability.kind else {
                        return None;
                    };
                    let TriggerKind::Either { left, right } =
                        &triggered.trigger.compiled_model()?.kind
                    else {
                        return None;
                    };
                    Some((left, right))
                })
                .expect("cycle/discard remains one alternative trigger");
            assert_eq!(
                left.kind,
                TriggerKind::KeywordActionMatchingObject {
                    action: KeywordActionKind::Cycle,
                    player: PlayerFilter::You,
                    filter: ObjectFilter::default().other()
                }
            );
            assert_eq!(
                right.kind,
                TriggerKind::PlayerDiscardsCard {
                    player: PlayerFilter::You,
                    filter: Some(ObjectFilter::default().other()),
                    one_or_more: false
                }
            );
        }
    }
}

fn settle(game: &mut GameState, queue: &mut TriggerQueue) {
    let mut dm = SelectFirstDecisionMaker;
    for _ in 0..16 {
        put_triggers_on_stack_with_dm(game, queue, &mut dm).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("cycling and its bounded trigger work did not settle");
}

fn cycle(game: &mut GameState, card: ObjectId, actor: PlayerId, queue: &mut TriggerQueue) {
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = actor;
    game.turn.priority_player = Some(actor);
    game.player_mut(actor)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    let action = compute_legal_actions(game, actor).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == card)).expect("cycling is a legal hand activation");
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..16 {
        match progress {
            GameProgress::NeedsDecisionCtx(context) => {
                progress =
                    apply_decision_context_with_dm(game, queue, &mut state, &context, &mut dm)
                        .unwrap();
            }
            _ => {
                settle(game, queue);
                return;
            }
        }
    }
    panic!("cycling payment did not complete");
}

#[test]
fn cycle_discard_other_triggers_actual_cycling_cost_triggers_once_and_plain_discard_still_triggers()
{
    let fixture = fixtures()
        .into_iter()
        .find(|card| card["name"] == "Horror of the Broken Lands")
        .unwrap();
    for definition in definitions(
        "Horror of the Broken Lands",
        fixture["text"].as_str().unwrap(),
    ) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let observer = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let cycling =
            compile_to_runtime_definition("Cycle fixture", "Type: Artifact\nCycling {1}", false)
                .unwrap();
        let library =
            compile_to_runtime_definition("Draw fixture", "Type: Sorcery", false).unwrap();
        let mut queue = TriggerQueue::new();
        for actor in [B, A] {
            game.create_object_from_definition(&library, actor, Zone::Library);
            let card = game.create_object_from_definition(&cycling, actor, Zone::Hand);
            cycle(&mut game, card, actor, &mut queue);
            assert_eq!(
                game.player(actor).unwrap().mana_pool.total(),
                0,
                "cycling pays its cost"
            );
            assert_eq!(
                game.player(actor).unwrap().hand.len(),
                1,
                "the cycling ability draws once"
            );
            assert_eq!(
                game.calculated_power(observer),
                Some(if actor == B { 4 } else { 6 }),
                "cycle/discard alternatives must not trigger twice"
            );
        }
        let mut dm = SelectFirstDecisionMaker;
        let mut context = EffectContext::new(observer, A, &mut dm);
        let outcome = DiscardEffect::you(1)
            .execute(&mut game, &mut context)
            .unwrap();
        for event in outcome.events {
            game.queue_trigger_event(Default::default(), event);
        }
        settle(&mut game, &mut queue);
        assert_eq!(
            game.calculated_power(observer),
            Some(8),
            "an ordinary later discard remains a new trigger"
        );
    }
}

#[test]
fn cycle_discard_other_triggers_exclude_the_source_card_at_the_matcher_boundary() {
    // Isolate the source-relative filter from active-zone discovery: when a
    // card cycles itself from hand its battlefield ability is also inactive.
    // Matching the typed event directly proves "another" itself survives.
    use ironsmith::events::other::CardDiscardedEvent;
    use ironsmith::triggers::{TriggerEvent, matcher_trait::TriggerContext};

    for definition in definitions(
        "Source exclusion probe",
        "Type: Enchantment\nWhenever you cycle or discard another card, you gain 1 life.",
    ) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&definition, A, Zone::Hand);
        let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected the cycle/discard trigger");
        };
        let context = TriggerContext::for_source(source, A, &game);
        for (card, expected) in [(source, false), (other, true)] {
            let event = TriggerEvent::new_with_provenance(
                CardDiscardedEvent::new(A, card),
                Default::default(),
            );
            assert_eq!(triggered.trigger.matches(&event, &context), expected);
        }
    }
}
