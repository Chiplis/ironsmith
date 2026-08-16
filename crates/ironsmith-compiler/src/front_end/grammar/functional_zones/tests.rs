use super::*;
use crate::lexer::lex_line;

#[test]
fn recognizes_static_zone_hints() {
    let graveyard = lex_line("You may cast this card from your graveyard.", 0).unwrap();
    assert_eq!(
        parse_static_functional_zones_tokens(&graveyard),
        Some(vec![Zone::Graveyard])
    );
}

#[test]
fn recognizes_typed_source_graveyard_cast_permission() {
    let tokens = lex_line(
        "You may cast this creature from your graveyard if you pay {1} more to cast it for each other creature card in your graveyard.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_static_functional_zones_tokens(&tokens),
        Some(vec![Zone::Graveyard])
    );
}

#[test]
fn recognizes_source_not_on_battlefield_as_every_nonbattlefield_zone() {
    let tokens = lex_line(
        "As long as this isn't on the battlefield, it's a creature.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_static_functional_zones_tokens(&tokens),
        Some(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ])
    );
}

#[test]
fn recognizes_trigger_zone_facts() {
    let tokens = lex_line(
        "When you discard this card, if this card is in your hand, draw a card.",
        0,
    )
    .unwrap();
    let facts = parse_trigger_functional_zone_facts_tokens(&tokens);
    assert_eq!(facts.explicit_zone, Some(Zone::Hand));
    assert!(facts.discards_this_card);

    let only_creature = lex_line(
        "At the beginning of your upkeep, if this card is the only creature card in your graveyard, you may return this card to the battlefield.",
        0,
    )
    .unwrap();
    let facts = parse_trigger_functional_zone_facts_tokens(&only_creature);
    assert_eq!(facts.explicit_zone, Some(Zone::Graveyard));
}

#[test]
fn recognizes_activated_functional_zone_facts() {
    let hand_cost = lex_line("Discard this card", 0).unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&hand_cost, &[]),
        vec![Zone::Hand]
    );

    let free = lex_line("{0}", 0).unwrap();
    let stack = lex_line(
        "Any player may activate this ability only if this is on the stack",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&free, &[&stack]),
        vec![Zone::Stack]
    );

    let move_commanders = lex_line(
        "Put all commanders you own from the command zone and from your graveyard into your hand",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&free, &[&move_commanders]),
        vec![Zone::Battlefield],
        "a destination set's command-zone qualifier must not relocate the source ability"
    );

    let move_source = lex_line("Return this card from the command zone to your hand", 0).unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&free, &[&move_source]),
        vec![Zone::Command]
    );
}
