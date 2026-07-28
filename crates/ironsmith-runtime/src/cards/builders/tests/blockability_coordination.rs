#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn sneaky_homunculus_preserves_coordinated_blocking_restrictions() {
    let oracle = "This creature can't block or be blocked by creatures with power 2 or greater.";
    let definition = parse_oracle_card_definition("Sneaky Homunculus");

    assert_eq!(canonical_compiled_lines(&definition).join("\n"), oracle);

    let static_ids = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&StaticAbilityId::CantBlock)
            && static_ids.contains(&StaticAbilityId::CantBeBlockedByPowerOrGreater),
        "both independently enforced restrictions must remain typed: {static_ids:?}"
    );
}

#[test]
fn hooded_horror_preserves_and_enforces_the_defending_player_maximum() {
    let oracle = "This creature can't be blocked as long as defending player controls the most creatures or is tied for the most.";
    let definition = parse_oracle_card_definition("Hooded Horror");

    assert_eq!(canonical_compiled_lines(&definition).join("\n"), oracle);
    assert!(
        definition.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(ability)
                if ability.id()
                    == StaticAbilityId::CantBeBlockedWhileDefendingPlayerControlsMostCreatures
        )),
        "the maximum/tie condition must remain a typed combat rule: {definition:#?}"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let attacker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = |name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    };
    let blocker = game.create_object_from_definition(&creature("Blocker"), bob, Zone::Battlefield);

    assert!(
        !crate::rules::combat::can_block(
            game.object(attacker).expect("attacker exists"),
            game.object(blocker).expect("blocker exists"),
            &game,
        ),
        "the defending player is tied for the most creatures at one each"
    );

    game.create_object_from_definition(
        &creature("Extra Attacking Creature"),
        alice,
        Zone::Battlefield,
    );
    assert!(
        crate::rules::combat::can_block(
            game.object(attacker).expect("attacker exists"),
            game.object(blocker).expect("blocker exists"),
            &game,
        ),
        "blocking becomes legal when another player controls strictly more creatures"
    );
}
