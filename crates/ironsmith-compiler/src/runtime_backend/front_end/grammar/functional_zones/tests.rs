use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn recognizes_static_zone_hints() {
    let graveyard = lex_line("You may cast this card from your graveyard.", 0).unwrap();
    assert_eq!(
        parse_static_functional_zones_tokens(&graveyard),
        Some(vec![Zone::Graveyard])
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
}
