#![cfg(ironsmith_runtime_parser_tests)]

use std::collections::VecDeque;

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::mana::ManaSymbol;

const COMPILED_TEXT: &str = "Lifelink\nWhen this creature enters, you may pay {1}{W} any number of times. When you pay this cost one or more times, put that many valor counters on this creature.\nEach creature you control gets +1/+1 for each valor counter on this creature.";

struct BooleanScript {
    responses: VecDeque<bool>,
}

impl BooleanScript {
    fn new(responses: impl IntoIterator<Item = bool>) -> Self {
        Self {
            responses: responses.into_iter().collect(),
        }
    }
}

impl DecisionMaker for BooleanScript {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.responses.pop_front().unwrap_or(false)
    }
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn intrepid_adversary_definition() -> CardDefinition {
    let mut definition = parse_oracle_card_definition("Intrepid Adversary");
    // The shared oracle parser helper supplies type line and rules text but
    // deliberately omits printed P/T metadata. Restore it for gameplay.
    definition.card.power_toughness = Some(PowerToughness::fixed(3, 1));
    definition
}

fn resolve_enters_trigger(
    game: &mut crate::GameState,
    source: ObjectId,
    controller: PlayerId,
    decisions: &mut dyn DecisionMaker,
) -> usize {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            source,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let matching = crate::triggers::check_triggers(game, &event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    let count = matching.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in matching {
        queue.add(trigger);
    }
    if count > 0 {
        crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, decisions)
            .expect("Intrepid Adversary's enters trigger should go on the stack");
        while !game.stack_is_empty() {
            crate::game_loop::resolve_stack_entry_with(game, decisions)
                .expect("Intrepid Adversary's enters/reflexive trigger should resolve");
        }
    }
    assert_eq!(
        game.current_controller(source),
        Some(controller),
        "the enters trigger must not change the Adversary's controller"
    );
    count
}

#[test]
fn intrepid_adversary_named_definition_preserves_counted_payment_and_dynamic_anthem() {
    let definition = intrepid_adversary_definition();
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        COMPILED_TEXT
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("Lifelink"), "{debug}");
    assert!(debug.contains("RepeatProcessEffect"), "{debug}");
    assert!(debug.contains("PayManaEffect"), "{debug}");
    assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
    assert!(debug.contains("valor"), "{debug}");
    assert!(debug.contains("PerCount"), "{debug}");
}

#[test]
fn intrepid_adversary_declined_payment_adds_no_counters_or_bonus() {
    let definition = intrepid_adversary_definition();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::White, 1);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    let mut decisions = BooleanScript::new([false]);

    assert_eq!(
        resolve_enters_trigger(&mut game, source, alice, &mut decisions),
        1
    );
    assert_eq!(
        game.counter_count(source, crate::object::CounterType::Named("valor")),
        0
    );
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 2);
    assert_eq!(game.calculated_power(source), Some(3));
    assert_eq!(game.calculated_toughness(source), Some(1));
}

#[test]
fn intrepid_adversary_counts_only_successful_payments_and_buffs_own_creatures() {
    let definition = intrepid_adversary_definition();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ally =
        game.create_object_from_definition(&creature("Alice Ally", 2, 2), alice, Zone::Battlefield);
    let opponent =
        game.create_object_from_definition(&creature("Bob Creature", 2, 2), bob, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::White, 2);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let mut decisions = BooleanScript::new([true, true, false]);

    assert_eq!(
        resolve_enters_trigger(&mut game, source, alice, &mut decisions),
        1
    );
    assert_eq!(
        game.counter_count(source, crate::object::CounterType::Named("valor")),
        2,
        "two successful {{1}}{{W}} payments should produce exactly two valor counters"
    );
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);
    assert_eq!(
        (
            game.calculated_power(source),
            game.calculated_toughness(source)
        ),
        (Some(5), Some(3))
    );
    assert_eq!(
        (game.calculated_power(ally), game.calculated_toughness(ally)),
        (Some(4), Some(4))
    );
    assert_eq!(
        (
            game.calculated_power(opponent),
            game.calculated_toughness(opponent)
        ),
        (Some(2), Some(2))
    );
}

#[test]
fn intrepid_adversary_cannot_count_an_unaffordable_extra_payment() {
    let definition = intrepid_adversary_definition();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::White, 1);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    let mut decisions = BooleanScript::new([true, true]);

    assert_eq!(
        resolve_enters_trigger(&mut game, source, alice, &mut decisions),
        1
    );
    assert_eq!(
        game.counter_count(source, crate::object::CounterType::Named("valor")),
        1,
        "only the affordable payment may contribute to the repeat count"
    );
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);
}
