use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::PartitionContext;
use ironsmith::effects::{EffectContext, EffectExecutor, ScryEffect, SurveilEffect};
use ironsmith::events::KeywordActionKind;
use ironsmith::game_loop::{put_triggers_on_stack, resolve_stack_entry};
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/keyword_action_alternatives.json.fixture"
    ))
    .unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let bytes = serde_json::to_vec(&artifact).unwrap();
    let restored: CompiledCardArtifact = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    [direct, registry.get(name).unwrap().clone()]
}

fn real_card(name: &str) -> [CardDefinition; 2] {
    let fixture = fixtures()
        .into_iter()
        .find(|card| card["name"] == name)
        .unwrap();
    definitions(
        fixture["compile_name"].as_str().unwrap(),
        fixture["text"].as_str().unwrap(),
    )
}

fn assert_actions(
    definition: &CardDefinition,
    left_player: PlayerFilter,
    right_player: PlayerFilter,
) {
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(ability) = &ability.kind else {
                return None;
            };
            let model = ability.trigger.compiled_model()?;
            matches!(model.kind, TriggerKind::Either { .. }).then_some(model)
        })
        .expect("a single typed alternative trigger must survive materialization");
    let TriggerKind::Either { left, right } = &trigger.kind else {
        unreachable!()
    };
    assert_eq!(
        left.kind,
        TriggerKind::KeywordAction {
            action: KeywordActionKind::Scry,
            player: left_player
        }
    );
    assert_eq!(
        right.kind,
        TriggerKind::KeywordAction {
            action: KeywordActionKind::Surveil,
            player: right_player
        }
    );
}

#[test]
fn keyword_action_alternatives_all_seven_complete_fronts_materialize_and_round_trip() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 7);
    for fixture in fixtures {
        // Prepared cards retain their complete linked-face source in the
        // fixture; this assertion covers the canonical/front compilation.
        for definition in definitions(
            fixture["compile_name"].as_str().unwrap(),
            fixture["text"].as_str().unwrap(),
        ) {
            assert_actions(&definition, PlayerFilter::You, PlayerFilter::You);
        }
    }
}

struct MoveFirst;
impl DecisionMaker for MoveFirst {
    fn decide_partition(&mut self, _: &GameState, context: &PartitionContext) -> Vec<ObjectId> {
        context
            .cards
            .first()
            .map(|(id, _)| vec![*id])
            .unwrap_or_default()
    }
}

fn add_library_card(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_definition(&definition, owner, Zone::Library)
}

// Use real production action executors and their emitted events, followed by
// the normal trigger queue and stack resolver, rather than fabricate a match.
fn perform(
    game: &mut GameState,
    source: ObjectId,
    actor: PlayerId,
    action: KeywordActionKind,
    count: i32,
) -> usize {
    let mut dm = MoveFirst;
    let mut context = EffectContext::new(source, actor, &mut dm);
    let outcome = match action {
        KeywordActionKind::Scry => ScryEffect::you(count).execute(game, &mut context).unwrap(),
        KeywordActionKind::Surveil => SurveilEffect::you(count)
            .execute(game, &mut context)
            .unwrap(),
        _ => unreachable!(),
    };
    let mut queue = TriggerQueue::new();
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack(game, &mut queue).unwrap();
    let count = game.stack.len();
    while !game.stack_is_empty() {
        resolve_stack_entry(game).unwrap();
    }
    count
}

#[test]
fn keyword_action_alternatives_matoya_draws_after_each_action_and_ignores_opponents_and_zero() {
    for definition in real_card("Matoya, Archon Elder") {
        for action in [KeywordActionKind::Scry, KeywordActionKind::Surveil] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let matoya = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let expected_draw = add_library_card(&mut game, A, "Kept card");
            let moved = add_library_card(&mut game, A, "Moved card");
            add_library_card(&mut game, B, "Opponent card");
            assert_eq!(perform(&mut game, matoya, B, action, 1), 0);
            assert_eq!(perform(&mut game, matoya, A, action, 0), 0);
            assert!(game.player(A).unwrap().hand.is_empty());
            assert_eq!(perform(&mut game, matoya, A, action, 1), 1);
            let hand = &game.player(A).unwrap().hand;
            assert_eq!(hand.len(), 1);
            assert_eq!(game.object(hand[0]).unwrap().name, "Kept card");
            assert!(!game.player(A).unwrap().library.contains(&expected_draw));
            let destination = if action == KeywordActionKind::Scry {
                Zone::Library
            } else {
                Zone::Graveyard
            };
            // Surveil uses a new identity after its zone change.
            assert!(
                game.objects_in_deterministic_order()
                    .iter()
                    .any(|object| object.name == "Moved card" && object.zone == destination)
            );
            if action == KeywordActionKind::Scry {
                assert!(game.player(A).unwrap().library.contains(&moved));
            }
        }
    }
}

#[test]
fn keyword_action_alternatives_preserve_explicit_players_and_deduplicate_overlap() {
    for (clause, players, expected) in [
        (
            "an opponent scries or surveils",
            (PlayerFilter::Opponent, PlayerFilter::Opponent),
            [0, 0, 1, 1],
        ),
        (
            "you scry or an opponent surveils",
            (PlayerFilter::You, PlayerFilter::Opponent),
            [1, 0, 0, 1],
        ),
    ] {
        for definition in definitions(
            "Player probe",
            &format!("Type: Enchantment\nWhenever {clause}, you gain 1 life."),
        ) {
            assert_actions(&definition, players.0.clone(), players.1.clone());
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for ((actor, action), expected) in [
                (A, KeywordActionKind::Scry),
                (A, KeywordActionKind::Surveil),
                (B, KeywordActionKind::Scry),
                (B, KeywordActionKind::Surveil),
            ]
            .into_iter()
            .zip(expected)
            {
                assert_eq!(
                    perform(&mut game, source, actor, action, 1),
                    expected,
                    "{clause}"
                );
            }
            assert_eq!(game.player(A).unwrap().life, 22);
        }
    }
    for definition in definitions(
        "Overlap probe",
        "Type: Enchantment\nWhenever you scry or scry, you gain 1 life.",
    ) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(perform(&mut game, source, A, KeywordActionKind::Scry, 1), 1);
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}

#[test]
fn keyword_action_alternatives_share_one_once_per_turn_limit() {
    for definition in real_card("Saheeli, Consul of Oversight") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(perform(&mut game, source, A, KeywordActionKind::Scry, 1), 1);
        assert_eq!(
            perform(&mut game, source, A, KeywordActionKind::Surveil, 1),
            0
        );
        assert_eq!(game.battlefield.len(), 2);
        game.next_turn();
        assert_eq!(
            perform(&mut game, source, A, KeywordActionKind::Surveil, 1),
            1
        );
        assert_eq!(game.battlefield.len(), 3);
    }
}
