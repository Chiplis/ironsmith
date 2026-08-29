use super::super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn serial_keyword_filter_commas_are_not_effect_boundaries() {
    let tokens = lex_line(
            "It deals 1 damage to each creature that doesn't have first strike, double strike, vigilance, or haste.",
            0,
        )
        .expect("lex");
    let segments = split_segments_on_comma_effect_head_tokens(vec![&tokens]);
    assert_eq!(segments, vec![tokens.as_slice()]);
}

#[test]
fn serial_creature_subtype_subject_is_not_an_effect_boundary() {
    let tokens = lex_line(
        "Birds, Frogs, Otters, and Rats you control get +1/+1 until end of turn.",
        0,
    )
    .expect("lex");
    assert_eq!(
        split_effect_chain_on_and_tokens(&tokens, true),
        vec![tokens.as_slice()]
    );

    let coordinated = lex_line(
        "Birds you control get +1/+1 and Rats you control gain flying.",
        0,
    )
    .expect("lex");
    assert_eq!(
        split_effect_chain_on_and_tokens(&coordinated, true).len(),
        2,
        "a completed subtype-subject action must still split from the next action"
    );
}

#[test]
fn quoted_granted_ability_is_not_an_effect_chain_boundary() {
    let tokens = lex_line(
            "Until end of turn, target creature gains trample and \"Whenever this creature attacks, draw a card and gain 1 life.\"",
            0,
        )
        .unwrap();
    assert_eq!(split_effect_chain_on_and_tokens(&tokens, true).len(), 1);

    let actual_chain = lex_line("Until end of turn, draw a card and gain 1 life.", 0).unwrap();
    assert_eq!(
        split_effect_chain_on_and_tokens(&actual_chain, true).len(),
        2
    );
}

#[test]
fn explicit_player_token_creation_keeps_adjacent_quoted_rules_atomic() {
    let tokens = lex_line(
            "That player creates a 0/1 colorless Goblin Construct artifact creature token with \"This token can't block\" and \"At the beginning of your upkeep, this token deals 1 damage to you.\"",
            0,
        )
        .expect("multi-rule token creation should lex");
    assert_eq!(
        split_effect_chain_on_and_tokens(&tokens, true),
        vec![tokens.as_slice()],
        "the conjunction between quoted token rules belongs to the token blueprint"
    );

    let outer_action = lex_line(
        "That player creates a 0/1 Goblin creature token and that player draws a card.",
        0,
    )
    .expect("token creation followed by a real outer action should lex");
    assert_eq!(
        split_effect_chain_on_and_tokens(&outer_action, true).len(),
        2,
        "an executable action outside quotes must remain a coordination boundary"
    );
}

#[test]
fn explicit_comma_then_is_distinct_from_other_chain_surfaces() {
    let comma_then = lex_line("Target player draws a card, then discards a card.", 0).unwrap();
    assert!(has_explicit_comma_then_boundary_tokens(&comma_then, |_| {
        false
    }));
    assert!(has_authored_comma_then_surface_tokens(&comma_then));

    let coordinated = lex_line("Target player draws a card and discards a card.", 0).unwrap();
    assert!(!has_explicit_comma_then_boundary_tokens(
        &coordinated,
        |_| false
    ));
    assert!(!has_authored_comma_then_surface_tokens(&coordinated));

    let leading_then = lex_line("Then target player draws a card.", 0).unwrap();
    assert!(!has_explicit_comma_then_boundary_tokens(
        &leading_then,
        |_| false
    ));
    assert!(!has_authored_comma_then_surface_tokens(&leading_then));

    let create_then_copy = lex_line(
        "Create a 1/1 Soldier creature token, then copy that spell.",
        0,
    )
    .unwrap();
    assert!(
        has_explicit_comma_then_boundary_tokens(&create_then_copy, |_| false),
        "copy is an executable effect head and `that spell` is its typed back-reference"
    );

    let copy_then_return = lex_line(
        "Copy target instant or sorcery spell, then return it to its owner's hand.",
        0,
    )
    .unwrap();
    assert!(
        has_explicit_comma_then_boundary_tokens(&copy_then_return, |_| false),
        "a zone-moving return can consume the head action's typed target"
    );

    let exile_then_return =
        lex_line("Exile it, then return that card to its owner's hand.", 0).unwrap();
    assert!(
        has_explicit_comma_then_boundary_tokens(&exile_then_return, |_| false),
        "a returned card can consume the immediately preceding exile result"
    );

    let gain_then_optional_payment = lex_line(
            "You get {E}{E}{E}{E}, then you may pay an amount of {E} equal to that permanent's mana value.",
            0,
        )
        .unwrap();
    assert!(
        has_explicit_comma_then_boundary_tokens(&gain_then_optional_payment, |_| false),
        "an explicit optional tail is an independent action even when it refers to the head's target"
    );

    let draw_then_optional_cast = lex_line(
            "Draw a card, then you may cast a spell from your hand with mana value less than or equal to that damage without paying its mana cost.",
            0,
        )
        .unwrap();
    assert!(
        has_explicit_comma_then_boundary_tokens(&draw_then_optional_cast, |_| false),
        "an explicit optional cast tail is an independent action even when it refers to the head result"
    );

    let return_then_choose = lex_line(
        "Return target card from your graveyard to your hand, then choose an opponent.",
        0,
    )
    .unwrap();
    assert!(
        has_explicit_comma_then_boundary_tokens(&return_then_choose, |_| false),
        "a nonverb choice head is an independent ordered action"
    );

    let create_then_source_damage = lex_line(
        "Create three 1/1 red Hamster creature tokens, then it deals X damage to any target.",
        0,
    )
    .unwrap();
    assert!(
        has_explicit_comma_then_boundary_tokens(&create_then_source_damage, |_| false),
        "a complete source-pronoun dynamic-damage tail is an independent ordered action"
    );

    let counters_then_dynamic_phase_out = lex_line(
            "Put that many +1/+1 counters on this creature, then up to that many other target artifacts, creatures, and/or enchantments phase out.",
            0,
        )
        .unwrap();
    assert!(
        has_explicit_comma_then_boundary_tokens(&counters_then_dynamic_phase_out, |_| false),
        "a dynamic target-count phase-out tail is an independent ordered action"
    );
}

#[test]
fn authored_comma_then_surface_survives_a_pronoun_tail_but_not_a_quote() {
    let pronoun_tail =
        lex_line("It explores, then it explores again.", 0).expect("pronoun tail should lex");
    assert!(
        !has_explicit_comma_then_boundary_tokens(&pronoun_tail, |_| false),
        "the pre-parse splitter should remain conservative around `it`"
    );
    assert!(has_authored_comma_then_surface_tokens(&pronoun_tail));

    let quoted = lex_line(
        "It gains \"Whenever this creature attacks, then draw a card.\"",
        0,
    )
    .expect("quoted rule should lex");
    assert!(!has_authored_comma_then_surface_tokens(&quoted));
}

#[test]
fn comma_then_puts_back_referenced_card_onto_battlefield_splits() {
    let tokens = lex_line(
            "Reveal the top card of their library, then put it onto the battlefield if it's a permanent card.",
            0,
        )
        .unwrap();
    let segments = split_segments_on_comma_then_tokens(vec![&tokens], |_| false);

    assert_eq!(segments.len(), 2, "{segments:#?}");
    assert_eq!(
        crate::lexer::parser_token_word_refs(segments[0]),
        ["reveal", "the", "top", "card", "of", "their", "library"]
    );
    assert_eq!(
        crate::lexer::parser_token_word_refs(segments[1]),
        [
            "put",
            "it",
            "onto",
            "the",
            "battlefield",
            "if",
            "its",
            "a",
            "permanent",
            "card"
        ]
    );
}

#[test]
fn repeated_comma_then_boundaries_split_every_ordered_action() {
    let tokens = lex_line("Scry 1, then scry 2, then scry 3.", 0)
        .expect("three-action scry chain should lex");
    let segments = split_segments_on_comma_then_tokens(vec![&tokens], |_| false);

    assert_eq!(segments.len(), 3, "{segments:#?}");
    let words = segments
        .iter()
        .map(|segment| crate::lexer::parser_token_word_refs(segment))
        .collect::<Vec<_>>();
    assert_eq!(
        words,
        vec![vec!["scry", "1"], vec!["scry", "2"], vec!["scry", "3"]]
    );
}

#[test]
fn token_copy_soulbond_exception_is_not_split_as_ability_removal() {
    let tokens = lex_line(
        "Create a token that's a copy of this creature, except it has haste and loses soulbond.",
        0,
    )
    .expect("copy exception should lex");

    assert_eq!(
        split_effect_chain_on_and_tokens(&tokens, true).len(),
        1,
        "the complete copy exception must reach typed copy-modifier lowering"
    );
}

#[test]
fn token_copy_half_pt_exception_remains_one_create_action() {
    for text in [
        "Create two tokens that are copies of target creature, except their power is half that creature's power and their toughness is half that creature's toughness.",
        "If that creature dies this way, its controller creates two tokens that are copies of that creature, except their base power is half that creature's power and their base toughness is half that creature's toughness.",
    ] {
        let tokens = lex_line(text, 0).expect("half-P/T copy exception should lex");
        let comma_segments = split_segments_on_comma_effect_head_tokens(vec![&tokens]);
        assert_eq!(comma_segments.len(), 1, "{comma_segments:#?}");
        assert_eq!(
            split_effect_chain_on_and_tokens(&tokens, true).len(),
            1,
            "the complete half-P/T exception must reach typed copy lowering"
        );
    }
}

#[test]
fn tapped_and_attacking_token_modifier_is_not_an_action_boundary() {
    for source in [
        "Create a tapped and attacking token that's a copy of target creature.",
        "Create a 4/4 white Angel creature token with flying that's tapped and attacking.",
    ] {
        let tokens = lex_line(source, 0).expect("token modifier should lex");
        assert_eq!(
            split_effect_chain_on_and_tokens(&tokens, true),
            vec![tokens.as_slice()],
            "tapped-and-attacking must stay in its token blueprint: {source}"
        );
    }
}

#[test]
fn object_unions_and_where_bindings_are_not_action_boundaries() {
    for source in [
        "Target creature or planeswalker gets -2/-2 until end of turn.",
        "It deals 3 damage to target planeswalker and each creature that player controls.",
    ] {
        let tokens = lex_line(source, 0).expect("coordination fixture should lex");
        assert_eq!(
            split_effect_chain_on_and_tokens(&tokens, true),
            vec![tokens.as_slice()],
            "a noun/value conjunction must stay inside its owning action: {source}"
        );
    }

    let source = "Target creature gains trample and gets +X/+X until end of turn, where X is the number of creatures you control.";
    let tokens = lex_line(source, 0).expect("where-binding fixture should lex");
    let segments = split_effect_chain_on_and_tokens(&tokens, true);
    assert_eq!(segments.len(), 2, "grant and pump are distinct actions");
    assert!(
        crate::lexer::parser_token_word_refs(segments[1])
            .starts_with(&["gets", "+x/+x", "until", "end", "of", "turn", "where", "x",]),
        "the where binding must stay with the pump action: {segments:#?}"
    );
}
