use crate::lexer::lex_line;

use super::*;

#[test]
fn parses_damage_head_and_target_shapes() {
    let tokens = lex_line(
        "Deals damage to each opponent equal to the number of cards in their hand",
        0,
    )
    .unwrap();
    let shape = parse_combat_damage_head_shape_lexed(&tokens);
    assert!(shape.direct_hand_size_each_opponent);
    assert!(!shape.divided);

    let tokens = lex_line("2 damage to each other player.", 0).unwrap();
    assert!(matches!(
        parse_combat_damage_target_shape_lexed(&tokens, 1),
        Ok(CombatDamageTargetShape::PlayerGroup(
            CombatPlayerDamageTargetShape::EachOtherPlayer
        ))
    ));

    let tokens = lex_line("each other opponent", 0).unwrap();
    assert_eq!(
        parse_combat_player_damage_target_shape_lexed(&tokens, false),
        Some(CombatPlayerDamageTargetShape::EachOtherOpponent)
    );
    let tokens = lex_line("each other player", 0).unwrap();
    assert_eq!(
        parse_combat_player_damage_target_shape_lexed(&tokens, false),
        Some(CombatPlayerDamageTargetShape::EachOtherPlayer)
    );

    let tokens = lex_line(
        "divided as its controller chooses among any number of those Wolves",
        0,
    )
    .unwrap();
    let shape = parse_combat_divided_target_shape_lexed(&tokens).unwrap();
    assert!(shape.count.is_any_number());
    assert_eq!(
        parser_token_word_refs(shape.target_tokens),
        ["those", "wolves"]
    );
}

#[test]
fn parses_trailing_unless_before_the_damage_target_fallback() {
    for text in [
        "4 damage to that player unless they control a commander",
        "2 damage to that player unless they control two or more basic lands",
        "2 damage to that player unless they have exactly three or exactly four cards in hand",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let shape = parse_combat_damage_target_shape_lexed(&tokens, 1).unwrap();
        let CombatDamageTargetShape::TrailingUnless {
            target_tokens,
            predicate,
        } = shape
        else {
            panic!("expected trailing-unless shape for {text}");
        };
        assert_eq!(parser_token_word_refs(target_tokens), ["that", "player"]);
        let predicate_debug = format!("{predicate:?}");
        assert!(
            predicate_debug.contains("Player") || predicate_debug.contains("ValueComparison"),
            "unexpected predicate for {text}: {predicate_debug}"
        );
    }
}

#[test]
fn parses_player_object_union_with_full_game_source_damage_history() {
    let tokens = lex_line(
        "1 damage to each opponent and planeswalker it has dealt damage to this game",
        0,
    )
    .unwrap();
    let shape = parse_combat_damage_target_shape_lexed(&tokens, 1).unwrap();
    let CombatDamageTargetShape::HistoricalDamageRecipients {
        players,
        filter_tokens,
    } = shape
    else {
        panic!("expected historical mixed-recipient shape");
    };
    assert_eq!(players, CombatPlayerDamageTargetShape::EachOpponent);
    assert_eq!(parser_token_word_refs(filter_tokens), ["planeswalker"]);

    let near_miss = lex_line(
        "1 damage to each opponent and planeswalker it has dealt damage to this turn",
        0,
    )
    .unwrap();
    assert!(!matches!(
        parse_combat_damage_target_shape_lexed(&near_miss, 1),
        Ok(CombatDamageTargetShape::HistoricalDamageRecipients { .. })
    ));
}

#[test]
fn parses_damage_pronouns_as_the_bound_event_player() {
    for text in ["the player", "that player", "them"] {
        let tokens = lex_line(text, 0).unwrap();
        assert_eq!(
            parse_combat_simple_damage_target_shape_lexed(&tokens),
            Some(CombatSimpleDamageTargetShape::IteratedPlayer),
            "damage recipient should use the typed event-player binding: {text}"
        );
    }
}

#[test]
fn recognizes_spell_target_inside_controller_recipient() {
    for text in ["target spell's controller", "target spells controller"] {
        let tokens = lex_line(text, 0).unwrap();
        assert_eq!(
            parse_combat_embedded_target_controller_shape_lexed(&tokens),
            Some(CombatEmbeddedTargetControllerShape::Spell),
            "{text}"
        );
    }
    let tokens = lex_line("that spell's controller", 0).unwrap();
    assert_eq!(
        parse_combat_embedded_target_controller_shape_lexed(&tokens),
        None
    );
}

#[test]
fn distinguishes_even_rounded_down_from_chosen_distribution() {
    let evenly = lex_line(
        "damage divided evenly, rounded down, among any number of targets",
        0,
    )
    .unwrap();
    assert!(matches!(
        parse_combat_divided_amount_shape_lexed(&evenly, 0).unwrap(),
        CombatDividedAmountShape::Distributed {
            evenly_rounded_down: true,
            ..
        }
    ));

    let chosen = lex_line(
        "damage divided as you choose among any number of targets",
        0,
    )
    .unwrap();
    assert!(matches!(
        parse_combat_divided_amount_shape_lexed(&chosen, 0).unwrap(),
        CombatDividedAmountShape::Distributed {
            evenly_rounded_down: false,
            ..
        }
    ));
}
