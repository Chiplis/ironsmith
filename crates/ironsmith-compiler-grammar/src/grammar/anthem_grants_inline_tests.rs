use super::super::super::lexer::lex_line;
use super::*;

#[test]
fn recognizes_only_the_complete_defending_player_most_creatures_condition() {
    let exact = lex_line(
        "defending player controls the most creatures or is tied for the most.",
        0,
    )
    .expect("lex exact condition");
    assert!(parse_defending_player_controls_most_creatures_or_tied_condition(&exact));

    let truncated = lex_line("defending player controls the most creatures.", 0)
        .expect("lex truncated condition");
    let ordinary =
        lex_line("defending player controls a creature.", 0).expect("lex ordinary condition");
    assert!(!parse_defending_player_controls_most_creatures_or_tied_condition(&truncated));
    assert!(!parse_defending_player_controls_most_creatures_or_tied_condition(&ordinary));
}

#[test]
fn parses_cards_drawn_thresholds() {
    let tokens = lex_line("You've drawn three or more cards this turn", 0).unwrap();
    assert_eq!(
        parse_cards_drawn_this_turn_threshold(&tokens),
        Some(TurnThreshold {
            player: TurnThresholdPlayer::You,
            count: 3,
        })
    );

    let tokens = lex_line("An opponent has drawn two or more card this turn", 0).unwrap();
    assert_eq!(
        parse_cards_drawn_this_turn_threshold(&tokens),
        Some(TurnThreshold {
            player: TurnThresholdPlayer::Opponent,
            count: 2,
        })
    );
}

#[test]
fn parses_dice_rolled_thresholds() {
    let tokens = lex_line("Players have rolled four or more dice this turn", 0).unwrap();
    assert_eq!(
        parse_dice_rolled_this_turn_threshold(&tokens),
        Some(TurnThreshold {
            player: TurnThresholdPlayer::Any,
            count: 4,
        })
    );

    let tokens = lex_line("You have rolled one or more die this turn", 0).unwrap();
    assert_eq!(
        parse_dice_rolled_this_turn_threshold(&tokens),
        Some(TurnThreshold {
            player: TurnThresholdPlayer::You,
            count: 1,
        })
    );
}

#[test]
fn parses_source_color_tails() {
    let tokens = lex_line("it is blue", 0).unwrap();
    assert_eq!(parse_if_source_is_color(&tokens), Some(ColorSet::BLUE));

    let tokens = lex_line("this creature is red", 0).unwrap();
    assert_eq!(parse_if_source_is_color(&tokens), Some(ColorSet::RED));

    let tokens = lex_line("it is blue and red", 0).unwrap();
    assert_eq!(parse_if_source_is_color(&tokens), None);
}

#[test]
fn parses_source_counter_count_clause() {
    let tokens = lex_line("three lore counters on this enchantment", 0).unwrap();
    let parsed = parse_source_counter_count_clause(&tokens).unwrap();
    assert_eq!(parsed.counter_type_word, "lore");
    assert!(parsed.starts_with_source_pronoun);
    assert_eq!(
        primitives::TokenWordView::new(parsed.source_tokens).word_refs(),
        ["this", "enchantment"]
    );

    let tokens = lex_line("counters on this enchantment", 0).unwrap();
    assert_eq!(parse_source_counter_count_clause(&tokens), None);
}

#[test]
fn first_spell_each_turn_clause_retains_cast_origin_words() {
    for (text, expected) in [
        (
            "The first spell you cast each turn",
            vec!["spell", "you", "cast"],
        ),
        (
            "The first noncreature spell you cast from exile each turn",
            vec!["noncreature", "spell", "you", "cast", "from", "exile"],
        ),
    ] {
        let tokens = lex_line(text, 0).expect("first-spell fixture should lex");
        let parsed = parse_first_spell_each_turn_clause(&tokens)
            .unwrap_or_else(|| panic!("first-spell clause should parse: {text}"));
        assert_eq!(
            TokenWordView::new(parsed.filter_tokens).word_refs(),
            expected,
            "{text}"
        );
        assert!(parsed.mana_source_tokens.is_none(), "{text}");
    }

    let mana_source = lex_line(
        "The first spell you cast each turn that mana from a Treasure was spent to cast",
        0,
    )
    .expect("mana-source first-spell fixture should lex");
    let parsed = parse_first_spell_each_turn_clause(&mana_source)
        .expect("mana-source first-spell clause should parse");
    assert_eq!(
        TokenWordView::new(parsed.filter_tokens).word_refs(),
        ["spell", "you", "cast"]
    );
    assert_eq!(
        TokenWordView::new(
            parsed
                .mana_source_tokens
                .expect("relative source tokens should be preserved")
        )
        .word_refs(),
        ["a", "treasure"]
    );

    let noncast = lex_line("The first spell revealed each turn", 0).unwrap();
    assert!(parse_first_spell_each_turn_clause(&noncast).is_none());

    let malformed = lex_line(
        "The first spell you cast each turn that mana from a Treasure was produced",
        0,
    )
    .unwrap();
    assert!(parse_first_spell_each_turn_clause(&malformed).is_none());
}

#[test]
fn lose_all_abilities_shape_does_not_steal_preceding_anthem() {
    let direct = lex_line("Enchanted creature loses all abilities.", 0).unwrap();
    assert!(parse_lose_all_abilities_shape(&direct).is_some());

    let compound = lex_line("Enchanted creature gets -5/-0 and loses all abilities.", 0).unwrap();
    assert!(parse_lose_all_abilities_shape(&compound).is_none());
}

#[test]
fn captures_keyword_and_maximum_blocker_clause_without_flattening_subject() {
    let tokens = lex_line(
        "Enchanted creature has hexproof and can't be blocked by more than one creature.",
        0,
    )
    .unwrap();
    let parsed = parse_keywords_and_cant_be_blocked_by_more_than_clause(&tokens)
        .expect("compound attached grant should parse");

    assert_eq!(
        TokenWordView::new(parsed.subject_tokens).word_refs(),
        ["enchanted", "creature"]
    );
    assert_eq!(
        TokenWordView::new(parsed.keyword_tokens).word_refs(),
        ["hexproof"]
    );
    assert_eq!(
        TokenWordView::new(parsed.blocker_threshold_tokens).word_refs(),
        ["more", "than", "one"]
    );
}

#[test]
fn subject_keyword_loss_preserves_have_or_gain_prohibition_mode() {
    let tokens = lex_line(
        "Creatures your opponents control lose flying and can't have or gain flying.",
        0,
    )
    .unwrap();
    let parsed = parse_subject_loses_keywords_clause(&tokens)
        .expect("Archetype-style keyword loss should parse");

    assert_eq!(
        parsed.loss_mode,
        ironsmith_core::AbilityLossMode::LoseAndCantHaveOrGain
    );
    assert_eq!(
        TokenWordView::new(parsed.loss_tokens).word_refs(),
        ["flying"]
    );
    assert_eq!(
        TokenWordView::new(
            parsed
                .additional_gain_tokens
                .expect("prohibited keyword should be retained"),
        )
        .word_refs(),
        ["flying"]
    );
}
