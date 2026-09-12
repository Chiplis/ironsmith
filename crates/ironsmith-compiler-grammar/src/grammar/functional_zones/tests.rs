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

#[test]
fn source_exile_from_graveyard_sets_trigger_zone_without_affecting_other_cards() {
    for source in ["this card", "this"] {
        let tokens = lex_line(
            &format!("When you cast a creature spell, exile {source} from your graveyard."),
            0,
        )
        .unwrap();
        assert_eq!(
            parse_trigger_functional_zone_facts_tokens(&tokens).explicit_zone,
            Some(Zone::Graveyard)
        );
    }
    let tokens = lex_line(
        "When you cast a creature spell, exile a card from your graveyard.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_trigger_functional_zone_facts_tokens(&tokens).explicit_zone,
        None
    );
}

#[test]
fn rule_113_6m_coordinated_source_moves_and_exile() {
    let cost = lex_line("{1}{B}{R}{G}, Sacrifice a Saproling", 0).unwrap();
    for text in [
        "Return this card and up to one other target creature card from your graveyard to the battlefield",
        "Return this card and target land card from your graveyard to the battlefield tapped",
    ] {
        let effect = lex_line(text, 0).unwrap();
        assert_eq!(
            parse_activated_functional_zones_tokens(&cost, &[&effect]),
            vec![Zone::Graveyard],
            "{text}"
        );
    }
    let effect = lex_line("Put this card from exile onto the battlefield tapped", 0).unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&cost, &[&effect]),
        vec![Zone::Exile]
    );
    let cost = lex_line(
        "Exile this card and two other cards named Example from your graveyard",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&cost, &[]),
        vec![Zone::Graveyard]
    );
}

#[test]
fn rule_113_6m_prior_source_moves_and_unrelated_origins() {
    for (cost, effects, expected) in [
        (
            "Discard this card",
            "Shuffle this card into your library from your graveyard, then draw a card",
            Zone::Hand,
        ),
        (
            "Sacrifice this creature",
            "Return this card from your graveyard to the battlefield",
            Zone::Battlefield,
        ),
        (
            "Exile this creature",
            "At the beginning of the next end step, return this card from exile to the battlefield",
            Zone::Battlefield,
        ),
        (
            "{2}",
            "Exile this creature, then return this card from exile to the battlefield",
            Zone::Battlefield,
        ),
        (
            "{2}",
            "Return this creature to its owner's hand and return target Griffin card from your graveyard to your hand",
            Zone::Battlefield,
        ),
        (
            "{2}",
            "Return target creature card from your graveyard to the battlefield",
            Zone::Battlefield,
        ),
        (
            "{2}",
            "At the beginning of the next end step, return this card from your graveyard to the battlefield",
            Zone::Graveyard,
        ),
        (
            "{2}",
            "Shuffle this card into your library from your graveyard",
            Zone::Graveyard,
        ),
        (
            "{2}",
            "Put this card onto the battlefield from your hand",
            Zone::Hand,
        ),
        (
            "{2}",
            "Exile this card. Activate only if this card is in your graveyard",
            Zone::Graveyard,
        ),
    ] {
        let cost = lex_line(cost, 0).unwrap();
        let effect = lex_line(effects, 0).unwrap();
        assert_eq!(
            parse_activated_functional_zones_tokens(&cost, &[&effect]),
            vec![expected],
            "{effects}"
        );
    }
    let cost = lex_line("{2}", 0).unwrap();
    let effect = lex_line("Return this card to its owner's hand. Activate only if this card is on the battlefield or in your graveyard", 0).unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&cost, &[&effect]),
        vec![Zone::Battlefield, Zone::Graveyard]
    );
    let effect = lex_line(
        "Return this card from your graveyard or from exile to your hand",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&cost, &[&effect]),
        vec![Zone::Graveyard, Zone::Exile]
    );
}

#[test]
fn rule_113_6m_granted_abilities_have_their_own_source() {
    let cost = lex_line("{2}", 0).unwrap();
    let effect = lex_line(r#"Target creature gains "{B}: Return this card from your graveyard to your hand" until end of turn"#, 0).unwrap();
    assert_eq!(
        parse_activated_functional_zones_tokens(&cost, &[&effect]),
        vec![Zone::Battlefield]
    );
}
