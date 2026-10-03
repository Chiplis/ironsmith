use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{DiscardEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{put_triggers_on_stack, resolve_stack_entry};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

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

fn assert_typed_filter(definition: &CardDefinition, kind: CardType, excluded: bool, grouped: bool) {
    let (player, filter, one_or_more) = definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            let TriggerKind::PlayerDiscardsCard {
                player,
                filter,
                one_or_more,
            } = &triggered.trigger.compiled_model()?.kind
            else {
                return None;
            };
            Some((player, filter, one_or_more))
        })
        .expect("discard event retains a typed card filter and grouping");
    assert_eq!(player, &PlayerFilter::You);
    assert_eq!(*one_or_more, grouped);
    let filter = filter.as_ref().unwrap();
    assert_eq!(
        filter.zone, None,
        "event snapshots are in hand, not on the battlefield"
    );
    if excluded {
        assert_eq!(filter.excluded_card_types, vec![kind]);
    } else {
        assert_eq!(filter.card_types, vec![kind]);
    }
}

#[test]
fn filtered_discard_triggers_all_five_complete_cards_materialize_and_round_trip() {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/filtered_discard_triggers.json.fixture"
    ))
    .unwrap();
    assert_eq!(fixtures.len(), 5);
    for fixture in fixtures {
        let name = fixture["name"].as_str().unwrap();
        let (kind, excluded) = match name {
            "Conspiracy Theorist" | "Veronica, Dissident Scribe" => (CardType::Land, true),
            "Doctor Doom, King of Latveria" => (CardType::Land, false),
            _ => (CardType::Artifact, false),
        };
        for definition in definitions(name, fixture["text"].as_str().unwrap()) {
            assert_typed_filter(&definition, kind, excluded, true);
        }
    }
}

fn hand_card(game: &mut GameState, actor: PlayerId, kind: CardType) {
    let definition = CardDefinitionBuilder::new(CardId::new(), format!("{kind:?} hand fixture"))
        .card_types(vec![kind])
        .build();
    game.create_object_from_definition(&definition, actor, Zone::Hand);
}

fn discard(game: &mut GameState, source: ObjectId, actor: PlayerId, count: i32) -> usize {
    let mut dm = SelectFirstDecisionMaker;
    let mut context = EffectContext::new(source, actor, &mut dm);
    let outcome = DiscardEffect::you(count)
        .execute(game, &mut context)
        .unwrap();
    for event in outcome.events {
        game.queue_trigger_event(Default::default(), event);
    }
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack(game, &mut queue).unwrap();
    let count = game.stack.len();
    while !game.stack_is_empty() {
        resolve_stack_entry(game).unwrap();
    }
    count
}

#[test]
fn filtered_discard_triggers_preserve_types_players_and_batch_counts() {
    for (qualifier, kind, excluded, matching, other) in [
        (
            "artifact",
            CardType::Artifact,
            false,
            CardType::Artifact,
            CardType::Land,
        ),
        (
            "land",
            CardType::Land,
            false,
            CardType::Land,
            CardType::Creature,
        ),
        (
            "nonland",
            CardType::Land,
            true,
            CardType::Creature,
            CardType::Land,
        ),
    ] {
        for grouped in [false, true] {
            let cards = if grouped {
                format!("one or more {qualifier} cards")
            } else {
                format!("a {qualifier} card")
            };
            for definition in definitions(
                "Discard probe",
                &format!("Type: Enchantment\nWhenever you discard {cards}, you gain 1 life."),
            ) {
                assert_typed_filter(&definition, kind, excluded, grouped);
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                hand_card(&mut game, B, matching);
                assert_eq!(discard(&mut game, source, B, 1), 0);
                hand_card(&mut game, A, other);
                assert_eq!(discard(&mut game, source, A, 1), 0);
                for card in [matching, other, matching] {
                    hand_card(&mut game, A, card);
                }
                let expected = if grouped { 1 } else { 2 };
                assert_eq!(discard(&mut game, source, A, 3), expected, "{cards}");
                assert_eq!(game.player(A).unwrap().life, 20 + expected as i32);
                assert!(game.player(A).unwrap().hand.is_empty());
                hand_card(&mut game, A, matching);
                assert_eq!(
                    discard(&mut game, source, A, 1),
                    1,
                    "later discard is a separate batch"
                );
            }
        }
    }
}
