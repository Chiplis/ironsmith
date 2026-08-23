#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const COUNTER_PREVENTION_LINE: &str = "For each 1 damage that would be dealt to this creature, if it has a +1/+1 counter on it, remove a +1/+1 counter from it and prevent that 1 damage.";

#[test]
fn rock_hydra_prevents_only_one_damage_for_each_counter_it_can_remove() {
    let definition = parse_oracle_card_definition("Rock Hydra");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == COUNTER_PREVENTION_LINE),
        "Rock Hydra must retain its per-damage counter replacement: {rendered:#?}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let hydra = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(hydra, CounterType::PlusOnePlusOne, 2);

    let damage_source_definition = CardDefinitionBuilder::new(CardId::new(), "Hydra Hurter")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let damage_source =
        game.create_object_from_definition(&damage_source_definition, bob, Zone::Battlefield);
    game.update_replacement_effects();

    let (remaining, prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        damage_source,
        crate::events::DamageTarget::Object(hydra),
        5,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(remaining, 3, "two counters must prevent exactly two damage");
    assert!(
        prevented || remaining < 5,
        "the damage event must reflect partial prevention"
    );
    assert_eq!(
        game.counter_count(hydra, CounterType::PlusOnePlusOne),
        0,
        "one counter must be removed for each prevented damage"
    );

    game.add_counters(hydra, CounterType::PlusOnePlusOne, 3);
    let (remaining, prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        damage_source,
        crate::events::DamageTarget::Object(hydra),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(remaining, 0, "enough counters must prevent all the damage");
    assert!(prevented || remaining < 2);
    assert_eq!(
        game.counter_count(hydra, CounterType::PlusOnePlusOne),
        1,
        "the replacement must not remove more counters than damage prevented"
    );
}
