use crate::cards::builders::{CardDefinitionBuilder, CardTextError, ChoiceCount};
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityId;
use crate::types::{CardType, Subtype, Supertype};
use std::fs;
use std::path::{Path, PathBuf};

use super::TokenWordView;
use super::lexer::{LexCursor, render_token_slice};
use super::{
    RewriteKeywordLineKind, RewriteSemanticItem, lex_line, lower_activation_cost_cst,
    parse_activate_only_timing_lexed, parse_activation_condition_lexed,
    parse_activation_cost_rewrite, parse_activation_cost_tokens_rewrite,
    parse_cant_effect_sentence, parse_cant_effect_sentence_lexed, parse_cost_reduction_line,
    parse_count_word_rewrite, parse_effect_sentence_lexed, parse_mana_cost_rewrite,
    parse_mana_symbol_group_rewrite, parse_mana_usage_restriction_sentence_lexed,
    parse_restriction_duration, parse_restriction_duration_lexed, parse_text_to_semantic_document,
    parse_text_with_annotations_lowered, parse_triggered_times_each_turn_lexed,
    parse_type_line_rewrite, split_lexed_sentences, token_word_refs,
};

fn rewrite_line_info(text: &str) -> super::LineInfo {
    super::LineInfo {
        line_index: 0,
        display_line_index: 0,
        raw_line: text.to_string(),
        normalized: super::NormalizedLine {
            original: text.to_string(),
            normalized: text.to_string(),
            char_map: Vec::new(),
        },
    }
}

fn parse_error_message<T>(result: Result<T, CardTextError>) -> String {
    match result {
        Ok(_) => panic!("expected parse error"),
        Err(CardTextError::ParseError(message)) => message,
        Err(other) => panic!("expected parse error, got {other:?}"),
    }
}

#[test]
fn rewrite_lexer_tracks_spans_for_activation_lines() {
    let tokens = lex_line("{T}, Sacrifice a creature: Add {B}{B}.", 3)
        .expect("rewrite lexer should classify activation line");
    assert_eq!(tokens[0].slice, "{T}");
    assert_eq!(tokens[0].span.line, 3);
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 3);
    assert!(tokens.iter().any(|token| token.slice == ":"));
}

#[test]
fn rewrite_lexer_accepts_plus_prefixed_counter_words() {
    let tokens = lex_line("Put a +1/+1 counter on target creature.", 0)
        .expect("rewrite lexer should accept +1/+1 words");
    assert!(tokens.iter().any(|token| token.slice == "+1/+1"));
}

#[test]
fn rewrite_lexer_keeps_signed_counters_and_attached_word_punctuation_atomic() {
    let tokens = lex_line(
        "Return those creatures to their owners' hands and give them -1/-1 until end-of-turn.",
        0,
    )
    .expect("rewrite lexer should keep attached punctuation inside atomic words");
    let shapes = tokens
        .iter()
        .map(|token| (token.kind, token.slice.as_str()))
        .collect::<Vec<_>>();

    assert!(shapes.contains(&(super::lexer::TokenKind::Word, "owners'")));
    assert!(shapes.contains(&(super::lexer::TokenKind::Word, "-1/-1")));
    assert!(shapes.contains(&(super::lexer::TokenKind::Word, "end-of-turn")));
}

#[test]
fn rewrite_lexer_keeps_generic_slash_words_atomic_but_exposes_standalone_apostrophes() {
    let tokens = lex_line("'power/toughness can’t be 0.", 0)
        .expect("rewrite lexer should classify slash words and standalone apostrophes");
    let kinds = tokens
        .iter()
        .map(|token| (token.kind, token.slice.as_str()))
        .collect::<Vec<_>>();

    assert_eq!(kinds[0], (super::lexer::TokenKind::Apostrophe, "'"));
    assert_eq!(kinds[1], (super::lexer::TokenKind::Word, "power/toughness"));
    assert!(kinds.contains(&(super::lexer::TokenKind::Word, "can’t")));
}

#[test]
fn rewrite_lexer_distinguishes_structural_tokens() {
    let tokens =
        lex_line("(Mode 2) '", 0).expect("rewrite lexer should classify structural tokens");
    let kinds = tokens.iter().map(|token| token.kind).collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            super::lexer::TokenKind::LParen,
            super::lexer::TokenKind::Word,
            super::lexer::TokenKind::Number,
            super::lexer::TokenKind::RParen,
            super::lexer::TokenKind::Apostrophe,
        ]
    );
}

#[test]
fn rewrite_lexer_precomputes_parser_text() {
    let tokens = lex_line("Its controller's face-down creature gets 2.", 0)
        .expect("rewrite lexer should classify parser-text test line");

    assert_eq!(tokens[0].parser_text(), "its");
    assert_eq!(tokens[1].parser_text(), "controller's");
    assert_eq!(tokens[2].parser_text(), "face-down");
    assert_eq!(tokens[5].parser_text(), "2");
}

#[test]
fn rewrite_lexer_reports_line_and_span_for_unknown_tokens() {
    let error = parse_error_message(lex_line("@", 2));
    assert!(
        error.contains("unsupported token"),
        "expected unsupported-token context, got {error}"
    );
    assert!(
        error.contains("\"@\""),
        "expected offending token in lexer error, got {error}"
    );
    assert!(
        error.contains("line 3"),
        "expected human-readable line number in lexer error, got {error}"
    );
    assert!(
        error.contains("0..1"),
        "expected lexer span in error, got {error}"
    );
}

#[test]
fn rewrite_lex_cursor_supports_peek_and_advance() {
    let tokens = lex_line("Whenever this creature attacks, draw a card.", 2)
        .expect("rewrite lexer should classify triggered line");
    let mut cursor = LexCursor::new(&tokens);
    assert_eq!(
        cursor.peek().and_then(|token| token.as_word()),
        Some("Whenever")
    );
    assert_eq!(
        cursor.peek_n(1).and_then(|token| token.as_word()),
        Some("this")
    );
    assert_eq!(
        cursor.advance().and_then(|token| token.as_word()),
        Some("Whenever")
    );
    assert_eq!(cursor.position(), 1);
    assert_eq!(
        token_word_refs(cursor.remaining()).first().copied(),
        Some("this")
    );
}

#[test]
fn rewrite_sentence_splitter_respects_quotes() {
    let tokens = lex_line("Choose one. \"Draw a card.\" Create a token.", 0)
        .expect("rewrite lexer should classify modal text");
    let sentences = split_lexed_sentences(&tokens);
    let rendered = sentences
        .into_iter()
        .map(|sentence| {
            sentence
                .iter()
                .map(|token| token.slice.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec!["Choose one", "\" Draw a card . \"", "Create a token"]
    );
}

#[test]
fn rewrite_structure_sentence_splitter_respects_quotes() {
    let tokens = lex_line("Choose one. \"Draw a card.\" Create a token.", 0)
        .expect("rewrite lexer should classify structural sentence text");
    let sentences = super::grammar::structure::split_lexed_sentences(&tokens);
    let rendered = sentences
        .into_iter()
        .map(|sentence| {
            sentence
                .iter()
                .map(|token| token.slice.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        rendered,
        vec!["Choose one", "\" Draw a card . \"", "Create a token"]
    );
}

#[test]
fn rewrite_winnow_parse_all_reports_precise_token_failures() {
    use super::grammar::primitives::{parse_all, phrase};

    let tokens = lex_line("If you do", 0).expect("rewrite lexer should classify phrase line");
    let parsed = parse_all(&tokens, phrase(&["if", "you", "do"]), "test-phrase");
    assert!(
        parsed.is_ok(),
        "expected parse_all phrase success, got {parsed:?}"
    );

    let error = parse_error_message(parse_all(
        &tokens,
        phrase(&["if", "you", "play"]),
        "test-phrase",
    ));
    assert!(
        error.contains("line 1"),
        "expected line location in parse_all error, got {error}"
    );
    assert!(
        error.contains("near \"do\""),
        "expected failing token context in parse_all error, got {error}"
    );
    assert!(
        (error.contains("play") && error.contains("word phrase"))
            || error.contains("expected play")
            || error.contains("expected word phrase"),
        "expected phrase expectation in parse_all error, got {error}"
    );
}

#[test]
fn rewrite_winnow_punctuation_combinators_cover_structural_tokens() {
    use super::grammar::primitives::{
        colon, comma, end_of_block, kw, lparen, parse_all, quote, rparen, semicolon,
    };

    let tokens =
        lex_line("(Draw), \"card\": then;", 0).expect("rewrite lexer should classify punctuation");
    let parsed = parse_all(
        &tokens,
        (
            lparen(),
            kw("draw"),
            rparen(),
            comma(),
            quote(),
            kw("card"),
            quote(),
            colon(),
            kw("then"),
            semicolon(),
            end_of_block(),
        ),
        "punctuation-sequence",
    );

    assert!(
        parsed.is_ok(),
        "expected punctuation combinators to parse structural tokens, got {parsed:?}"
    );
}

#[test]
fn rewrite_winnow_boundary_combinators_cover_sentence_and_block_endings() {
    use super::grammar::primitives::{
        end_of_block, end_of_sentence, end_of_sentence_or_block, parse_all, period, phrase,
    };

    let with_period =
        lex_line("Draw a card.", 0).expect("rewrite lexer should classify sentence boundary");
    let without_period =
        lex_line("Draw a card", 0).expect("rewrite lexer should classify block boundary");

    assert!(
        parse_all(
            &with_period,
            (phrase(&["draw", "a", "card"]), period(), end_of_block()),
            "period-boundary",
        )
        .is_ok()
    );
    assert!(
        parse_all(
            &with_period,
            (
                phrase(&["draw", "a", "card"]),
                end_of_sentence(),
                end_of_block(),
            ),
            "sentence-boundary",
        )
        .is_ok()
    );
    assert!(
        parse_all(
            &without_period,
            (phrase(&["draw", "a", "card"]), end_of_sentence_or_block()),
            "block-boundary",
        )
        .is_ok()
    );
}

#[test]
fn rewrite_winnow_phrase_and_boundary_combinators_cover_quote_and_parenthesis_edges() {
    use super::grammar::primitives::{end_of_block, lparen, parse_all, phrase, quote, rparen};

    let parenthetical =
        lex_line("(Draw a card)", 0).expect("rewrite lexer should classify parenthetical phrase");
    assert!(
        parse_all(
            &parenthetical,
            (
                lparen(),
                phrase(&["draw", "a", "card"]),
                rparen(),
                end_of_block(),
            ),
            "parenthetical-phrase",
        )
        .is_ok()
    );

    let missing_rparen =
        lex_line("(Draw a card", 0).expect("rewrite lexer should classify open parenthetical");
    let parenthetical_error = parse_error_message(parse_all(
        &missing_rparen,
        (
            lparen(),
            phrase(&["draw", "a", "card"]),
            rparen(),
            end_of_block(),
        ),
        "parenthetical-phrase",
    ));
    assert!(
        parenthetical_error.contains("right parenthesis"),
        "expected right-parenthesis context, got {parenthetical_error}"
    );

    let quoted =
        lex_line("\"Draw a card\"", 0).expect("rewrite lexer should classify quoted phrase");
    assert!(
        parse_all(
            &quoted,
            (
                quote(),
                phrase(&["draw", "a", "card"]),
                quote(),
                end_of_block(),
            ),
            "quoted-phrase",
        )
        .is_ok()
    );

    let missing_quote =
        lex_line("\"Draw a card", 0).expect("rewrite lexer should classify unterminated quote");
    let quote_error = parse_error_message(parse_all(
        &missing_quote,
        (
            quote(),
            phrase(&["draw", "a", "card"]),
            quote(),
            end_of_block(),
        ),
        "quoted-phrase",
    ));
    assert!(
        quote_error.contains("quote"),
        "expected quote context, got {quote_error}"
    );
}

#[test]
fn rewrite_winnow_separator_slice_helpers_split_keyword_lists() {
    use super::grammar::primitives::{
        split_lexed_slices_on_and, split_lexed_slices_on_comma,
        split_lexed_slices_on_commas_or_semicolons, split_lexed_slices_on_or,
        split_lexed_slices_on_period,
    };

    let separated = lex_line("Flying, vigilance; trample", 0)
        .expect("rewrite lexer should classify comma and semicolon separators");
    let separated_words: Vec<Vec<&str>> = split_lexed_slices_on_commas_or_semicolons(&separated)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(
        separated_words,
        vec![vec!["Flying"], vec!["vigilance"], vec!["trample"],]
    );

    let compound = lex_line("Protection from blue and from black", 0)
        .expect("rewrite lexer should classify keyword conjunction");
    let compound_words: Vec<Vec<&str>> = split_lexed_slices_on_and(&compound)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(
        compound_words,
        vec![vec!["Protection", "from", "blue"], vec!["from", "black"],]
    );

    let disjunction = lex_line("Aura, Equipment, or Vehicle", 0)
        .expect("rewrite lexer should classify disjunction separators");
    let disjunction_words: Vec<Vec<&str>> = split_lexed_slices_on_or(&disjunction)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(
        disjunction_words,
        vec![vec!["Aura"], vec!["Equipment"], vec!["Vehicle"],]
    );

    let comparison = lex_line("mana value 3 or less", 0)
        .expect("rewrite lexer should classify comparison or delimiter");
    let comparison_words: Vec<Vec<&str>> = split_lexed_slices_on_or(&comparison)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(
        comparison_words,
        vec![vec!["mana", "value", "3", "or", "less"],]
    );

    let comparison_equal = lex_line("mana value less than or equal to 3", 0)
        .expect("rewrite lexer should classify comparison or-equal phrase");
    let comparison_equal_words: Vec<Vec<&str>> = split_lexed_slices_on_or(&comparison_equal)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(
        comparison_equal_words,
        vec![vec![
            "mana", "value", "less", "than", "or", "equal", "to", "3"
        ],]
    );

    let comma_separated = lex_line(
        "if turning artifact creatures you control face up causes an ability, that ability triggers an additional time",
        0,
    )
    .expect("rewrite lexer should classify comma separators");
    let comma_words: Vec<Vec<&str>> = split_lexed_slices_on_comma(&comma_separated)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(
        comma_words,
        vec![
            vec![
                "if",
                "turning",
                "artifact",
                "creatures",
                "you",
                "control",
                "face",
                "up",
                "causes",
                "an",
                "ability",
            ],
            vec!["that", "ability", "triggers", "an", "additional", "time"],
        ]
    );

    let periods = lex_line(
        "Choose a color before the game begins. This card is the chosen color.",
        0,
    )
    .expect("rewrite lexer should classify period separators");
    let period_words: Vec<Vec<&str>> = split_lexed_slices_on_period(&periods)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(
        period_words,
        vec![
            vec!["Choose", "a", "color", "before", "the", "game", "begins"],
            vec!["This", "card", "is", "the", "chosen", "color"],
        ]
    );

    let repeated = lex_line(", flying, vigilance,", 0)
        .expect("rewrite lexer should classify repeated separators");
    let repeated_words: Vec<Vec<&str>> = split_lexed_slices_on_comma(&repeated)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(repeated_words, vec![vec!["flying"], vec!["vigilance"],]);

    let quoted_period = lex_line("Choose \"one.\" Then choose another.", 0)
        .expect("rewrite lexer should classify quoted period separators");
    let quoted_period_words: Vec<Vec<&str>> = split_lexed_slices_on_period(&quoted_period)
        .into_iter()
        .map(super::token_word_refs)
        .collect();
    assert_eq!(
        quoted_period_words,
        vec![vec!["Choose", "one", "Then", "choose", "another"],]
    );
}

#[test]
fn rewrite_winnow_search_helpers_scan_anywhere_in_token_stream() {
    use super::grammar::primitives::{
        contains_any_phrase, contains_phrase, contains_word, find_phrase_start,
    };

    let tokens = lex_line("Draw a card, then discard a card.", 0)
        .expect("rewrite lexer should classify comma-then sentence");

    assert!(contains_word(&tokens, "discard"));
    assert!(contains_phrase(&tokens, &["discard", "a", "card"]));
    assert!(contains_any_phrase(
        &tokens,
        &[&["mill", "a", "card"], &["discard", "a", "card"]]
    ));
    assert_eq!(
        find_phrase_start(&tokens, &["discard", "a", "card"]),
        Some(5)
    );
}

#[test]
fn rewrite_winnow_suffix_slice_helpers_strip_trigger_suffixes() {
    use super::grammar::primitives::strip_lexed_suffix_phrases;

    let first_time = lex_line(
        "Whenever one or more creatures attack you for the first time each turn",
        0,
    )
    .expect("rewrite lexer should classify trigger frequency suffix");
    let first_time_suffixes = [&["for", "the", "first", "time", "each", "turn"][..]];
    let (matched, head) = strip_lexed_suffix_phrases(&first_time, &first_time_suffixes)
        .expect("expected grammar suffix helper to strip first-time suffix");
    assert_eq!(matched, &["for", "the", "first", "time", "each", "turn"]);
    assert_eq!(
        super::token_word_refs(head),
        vec![
            "Whenever",
            "one",
            "or",
            "more",
            "creatures",
            "attack",
            "you"
        ]
    );

    let capped = lex_line(
        "Whenever one or more creatures attack you. This ability triggers only once each turn",
        0,
    )
    .expect("rewrite lexer should classify trigger cap suffix");
    let cap_suffixes = [&[
        "this", "ability", "triggers", "only", "once", "each", "turn",
    ][..]];
    let (matched, head) = strip_lexed_suffix_phrases(&capped, &cap_suffixes)
        .expect("expected grammar suffix helper to strip trigger cap suffix");
    assert_eq!(
        matched,
        &[
            "this", "ability", "triggers", "only", "once", "each", "turn"
        ]
    );
    assert_eq!(
        super::token_word_refs(head),
        vec![
            "Whenever",
            "one",
            "or",
            "more",
            "creatures",
            "attack",
            "you",
        ]
    );
}

#[test]
fn rewrite_winnow_prefix_slice_helper_strips_turn_duration_phrase() {
    use super::grammar::primitives::strip_lexed_prefix_phrase;

    let prefixed = lex_line("Until the end of your next turn, you may play that card", 0)
        .expect("rewrite lexer should classify prefixed duration phrase");
    let rest = strip_lexed_prefix_phrase(
        &prefixed,
        &["until", "the", "end", "of", "your", "next", "turn"],
    )
    .expect("expected grammar prefix helper to strip turn-duration phrase");

    assert_eq!(
        super::token_word_refs(rest),
        vec!["you", "may", "play", "that", "card"]
    );
}

#[test]
fn rewrite_winnow_span_helper_tracks_token_subslice_offsets() {
    let tokens = lex_line("Draw a card, then draw another.", 2)
        .expect("rewrite lexer should classify comma-delimited sentence");
    let (head, rest) = super::grammar::primitives::split_lexed_once_on_comma(&tokens)
        .expect("expected grammar split helper to find comma separator");
    let span = super::span_from_tokens(head).expect("expected span helper to cover token slice");

    assert_eq!(render_token_slice(head), "Draw a card");
    assert_eq!(
        super::token_word_refs(rest),
        vec!["then", "draw", "another"]
    );
    assert_eq!(span.line, 2);
    assert_eq!(span.start, head.first().expect("head token").span().start);
    assert_eq!(span.end, head.last().expect("head token").span().end);
}

#[test]
fn rewrite_structure_metadata_line_parser_recognizes_supported_labels() {
    let mana_tokens = lex_line("Mana Cost: {2}{W}", 0)
        .expect("rewrite lexer should classify mana-cost metadata line");
    let mana_spec = super::grammar::structure::split_metadata_line_lexed(&mana_tokens)
        .expect("structure metadata helper should recognize mana-cost label");
    assert_eq!(
        mana_spec.kind,
        super::grammar::structure::MetadataLineKind::ManaCost
    );
    assert_eq!(
        mana_spec
            .value_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["{2}", "{W}"]
    );

    let type_tokens = lex_line("Type: Legendary Creature — Human", 0)
        .expect("rewrite lexer should classify type metadata line");
    let type_spec = super::grammar::structure::split_metadata_line_lexed(&type_tokens)
        .expect("structure metadata helper should recognize type label");
    assert_eq!(
        type_spec.kind,
        super::grammar::structure::MetadataLineKind::TypeLine
    );
    assert_eq!(
        super::token_word_refs(type_spec.value_tokens),
        vec!["Legendary", "Creature", "Human"]
    );
}

#[test]
fn rewrite_structure_untap_all_other_players_untap_step_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Untap all permanents you control during each other player's untap step.",
        0,
    )
    .expect("rewrite lexer should classify untap-all other-players untap-step line");
    assert!(
        super::grammar::structure::looks_like_untap_all_during_each_other_players_untap_step_line_lexed(&tokens)
    );
}

#[test]
fn rewrite_structure_next_turn_cast_lock_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Each opponent can't cast instant or sorcery spells during that player's next turn.",
        0,
    )
    .expect("rewrite lexer should classify next-turn cast-lock line");
    assert!(super::grammar::structure::looks_like_next_turn_cant_cast_line_lexed(&tokens));
}

#[test]
fn rewrite_structure_divvy_statement_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Separate all creatures target player controls into two piles. Destroy all creatures in the pile of your choice.",
        0,
    )
    .expect("rewrite lexer should classify divvy pile line");
    assert!(super::grammar::structure::looks_like_divvy_statement_line_lexed(&tokens));
}

#[test]
fn rewrite_structure_vote_statement_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Starting with you, each player votes for death or torture.",
        0,
    )
    .expect("rewrite lexer should classify vote statement line");
    assert!(super::grammar::structure::looks_like_vote_statement_line_lexed(&tokens));
}

#[test]
fn rewrite_structure_generic_statement_shape_parser_recognizes_heads() {
    for text in [
        "Draw a card.",
        "Each player discards a card.",
        "That target player sacrifices a creature.",
        "This spell deals 3 damage to any target.",
        "Target creature gets +2/+2 until end of turn.",
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify generic statement-head line");
        assert!(super::grammar::structure::looks_like_generic_statement_line_lexed(&tokens));
    }
}

#[test]
fn rewrite_structure_generic_static_shape_parser_recognizes_heads() {
    for text in [
        "This creature has flying.",
        "Enchanted creature gets +1/+1.",
        "As long as you control an artifact, this creature has hexproof.",
        "Your maximum hand size is reduced by four.",
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify generic static-head line");
        assert!(super::grammar::structure::looks_like_generic_static_line_lexed(&tokens));
    }
}

#[test]
fn rewrite_sentence_splitter_ignores_single_quotes_inside_double_quotes() {
    let tokens = lex_line(
        "\"Create a 0/0 colorless Construct artifact creature token with 'This creature gets +1/+1 for each artifact you control.'\"",
        0,
    )
    .expect("rewrite lexer should classify nested quote ability text");
    let sentences = split_lexed_sentences(&tokens);
    let rendered = sentences
        .into_iter()
        .map(|sentence| {
            sentence
                .iter()
                .map(|token| token.slice.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec![
            "\" Create a 0/0 colorless Construct artifact creature token with ' This creature gets +1/+1 for each artifact you control . ' \""
        ]
    );
}

#[test]
fn rewrite_structure_sentence_splitter_keeps_unterminated_tail_segment() {
    let tokens = lex_line("Draw a card. Exile target creature", 0)
        .expect("rewrite lexer should classify unterminated tail");
    let sentences = super::grammar::structure::split_lexed_sentences(&tokens);
    let rendered = sentences
        .into_iter()
        .map(|sentence| {
            sentence
                .iter()
                .map(|token| token.slice.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();

    assert_eq!(rendered, vec!["Draw a card", "Exile target creature"]);
}

#[test]
fn rewrite_structure_sentence_splitter_separates_broken_visage_followups() {
    let tokens = lex_line(
        "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        0,
    )
    .expect("rewrite lexer should classify Broken Visage text");
    let rendered = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|sentence| {
            sentence
                .iter()
                .map(|token| token.slice.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        rendered,
        vec![
            "Destroy target nonartifact attacking creature",
            "It can't be regenerated",
            "Create a black Spirit creature token",
            "Its power is equal to that creature's power and its toughness is equal to that creature's toughness",
            "Sacrifice the token at the beginning of the next end step",
        ]
    );
}

#[test]
fn rewrite_effect_sentence_parser_handles_broken_visage_sequence() {
    let tokens = lex_line(
        "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        0,
    )
    .expect("rewrite lexer should classify Broken Visage text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&tokens);
    assert!(
        parsed.is_ok(),
        "Broken Visage effect sentences should parse directly, got {parsed:?}"
    );
}

#[test]
fn rewrite_cant_be_regenerated_followup_detector_matches_plain_it_clause() {
    let tokens = lex_line("It can't be regenerated.", 0)
        .expect("rewrite lexer should classify can't-be-regenerated followup");
    assert!(
        super::effect_sentences::is_cant_be_regenerated_followup_sentence(&tokens),
        "expected plain can't-be-regenerated sentence to be recognized as followup"
    );
}

#[test]
fn rewrite_semantic_parse_handles_broken_visage_statement() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Broken Visage Variant")
        .card_types(vec![CardType::Instant]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.".to_string(),
        false,
    )?;

    assert!(
        matches!(doc.items.as_slice(), [RewriteSemanticItem::Statement(_)]),
        "expected Broken Visage to remain a statement line, got {:#?}",
        doc.items
    );

    Ok(())
}

#[test]
fn rewrite_structure_modal_header_flag_scan_tracks_commander_and_repeat_modes() {
    let tokens = lex_line(
        "Choose one. If you control a commander as you cast this spell, you may choose both instead. You may choose the same mode more than once",
        0,
    )
    .expect("rewrite lexer should classify modal flag line");
    let flags = super::grammar::structure::scan_modal_header_flags(&tokens);

    assert!(flags.commander_allows_both, "{flags:?}");
    assert!(flags.same_mode_more_than_once, "{flags:?}");
    assert!(!flags.mode_must_be_unchosen, "{flags:?}");
    assert!(!flags.mode_must_be_unchosen_this_turn, "{flags:?}");
}

#[test]
fn rewrite_structure_modal_gate_scan_marks_remove_mode_only_without_word_view() {
    let tokens = lex_line(
        "Remove a +1/+1 counter from this creature. If you removed it this way,",
        0,
    )
    .expect("rewrite lexer should classify trailing modal gate line");
    let gate = super::grammar::structure::split_trailing_modal_gate_clause(&tokens)
        .expect("structure helper should detect trailing modal gate");

    assert!(gate.remove_mode_only, "{gate:?}");
    assert_eq!(
        gate.predicate,
        crate::cards::builders::IfResultPredicate::Did
    );
    assert_eq!(
        gate.prefix_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Remove", "a", "+1/+1", "counter", "from", "this", "creature", ".",
        ]
    );
}

#[test]
fn rewrite_structure_if_result_predicate_parser_preserves_contractions() {
    let didnt_tokens = lex_line("you don't", 0).expect("rewrite lexer should classify predicate");
    let dies_tokens = lex_line("that creature dies this way", 0)
        .expect("rewrite lexer should classify dies-this-way predicate");

    assert_eq!(
        super::grammar::structure::parse_if_result_predicate(&didnt_tokens),
        Some(crate::cards::builders::IfResultPredicate::DidNot)
    );
    assert_eq!(
        super::grammar::structure::parse_if_result_predicate(&dies_tokens),
        Some(crate::cards::builders::IfResultPredicate::DiesThisWay)
    );
}

#[test]
fn rewrite_structure_if_result_predicate_parser_keeps_coin_flip_outcomes() {
    let win_tokens =
        lex_line("you win the flip", 0).expect("rewrite lexer should classify win-the-flip text");
    let lose_tokens =
        lex_line("you lose the flip", 0).expect("rewrite lexer should classify lose-the-flip text");

    assert_eq!(
        super::grammar::structure::parse_if_result_predicate(&win_tokens),
        Some(crate::cards::builders::IfResultPredicate::Did)
    );
    assert_eq!(
        super::grammar::structure::parse_if_result_predicate(&lose_tokens),
        Some(crate::cards::builders::IfResultPredicate::DidNot)
    );
}

#[test]
fn rewrite_structure_leading_result_prefix_parser_splits_when_prefix() {
    let tokens = lex_line("When you do, draw a card.", 0)
        .expect("rewrite lexer should classify leading result prefix sentence");
    let prefix = super::grammar::structure::split_leading_result_prefix_lexed(&tokens)
        .expect("structure helper should detect leading result prefix");

    assert_eq!(
        prefix.kind,
        super::grammar::structure::LeadingResultPrefixKind::When
    );
    assert_eq!(
        prefix.predicate,
        crate::cards::builders::IfResultPredicate::Did
    );
    assert_eq!(
        prefix
            .trailing_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["draw", "a", "card", "."]
    );
}

#[test]
fn rewrite_structure_leading_result_prefix_parser_splits_numeric_ranges() {
    let tokens = lex_line("1—9 | You may put that card on top of your library.", 0)
        .expect("rewrite lexer should classify numeric result prefix sentence");
    let prefix = super::grammar::structure::split_leading_result_prefix_lexed(&tokens)
        .expect("structure helper should detect numeric result prefix");

    assert_eq!(
        prefix.kind,
        super::grammar::structure::LeadingResultPrefixKind::If
    );
    assert_eq!(
        prefix.predicate,
        crate::cards::builders::IfResultPredicate::Value(
            crate::effect::Comparison::BetweenInclusive(1, 9)
        )
    );
    assert_eq!(
        render_token_slice(prefix.trailing_tokens),
        "You may put that card on top of your library."
    );
}

#[test]
fn rewrite_structure_trailing_if_clause_parser_splits_destroy_clause() {
    let tokens = lex_line("Destroy target creature if it's white", 0)
        .expect("rewrite lexer should classify trailing-if clause");
    let spec = super::grammar::structure::split_trailing_if_clause_lexed(&tokens)
        .expect("structure helper should detect trailing-if clause");

    assert_eq!(
        spec.leading_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["Destroy", "target", "creature"]
    );
    assert!(matches!(
        spec.predicate,
        crate::cards::builders::PredicateAst::ItMatches(_)
    ));
}

#[test]
fn rewrite_structure_if_clause_splitter_routes_commaless_conditional_sentence() {
    let tokens = lex_line(
        "If at least three blue mana was spent to cast this spell create a Food token.",
        0,
    )
    .expect("rewrite lexer should classify comma-less if clause");
    let spec = super::grammar::structure::split_if_clause_lexed(
        &tokens,
        super::effect_sentences::parse_effect_chain_lexed,
    )
    .expect("structure helper should split comma-less if clause");

    match spec.predicate {
        super::grammar::structure::IfClausePredicateSpec::Conditional(predicate) => {
            assert!(!matches!(
                predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
        }
        other => panic!("expected conditional predicate split, got {other:?}"),
    }
    assert!(matches!(
        spec.effects.as_slice(),
        [crate::cards::builders::EffectAst::CreateTokenWithMods { .. }]
    ));
}

#[test]
fn rewrite_structure_predicate_parse_entrypoint_matches_parser_root_output() {
    let text = "it's your turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");

    let grammar = super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
        .expect("grammar predicate entrypoint should parse");
    let parser_root = super::parse_predicate_lexed(&lexed)
        .expect("parser-root predicate entrypoint should parse");

    assert_eq!(grammar, parser_root);
}

#[test]
fn rewrite_structure_predicate_parse_entrypoint_matches_parser_root_output_for_conjoined_predicate()
{
    let text = "it's your turn and you have no cards in hand";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");

    let grammar = super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
        .expect("grammar predicate entrypoint should parse");
    let parser_root = super::parse_predicate_lexed(&lexed)
        .expect("parser-root predicate entrypoint should parse");
    let debug = format!("{grammar:?}");

    assert_eq!(grammar, parser_root);
    assert!(
        debug.contains("And("),
        "expected conjoined predicate AST, got {debug}"
    );
}

#[test]
fn rewrite_structure_if_tail_parser_extracts_predicate() {
    let tokens = lex_line("if it's white", 0).expect("rewrite lexer should classify if tail");
    let predicate = super::grammar::structure::parse_trailing_if_predicate_lexed(&tokens)
        .expect("structure helper should parse if tail predicate");
    let expected =
        super::parse_predicate_lexed(&tokens[1..]).expect("tail predicate should still parse");

    assert_eq!(predicate, expected);
}

#[test]
fn rewrite_structure_trailing_unless_clause_parser_splits_gain_control_clause() {
    let tokens = lex_line("target creature unless you control an artifact", 0)
        .expect("rewrite lexer should classify trailing-unless clause");
    let spec = super::grammar::structure::split_trailing_unless_clause_lexed(&tokens)
        .expect("structure helper should detect trailing-unless clause");

    assert_eq!(
        spec.leading_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["target", "creature"]
    );
    let expected_tokens =
        lex_line("you control an artifact", 0).expect("expected predicate should lex");
    let expected =
        super::parse_predicate_lexed(&expected_tokens).expect("expected predicate should parse");

    assert_eq!(spec.predicate, expected);
}

#[test]
fn rewrite_structure_who_player_predicate_parser_extracts_prefixed_player_predicate() {
    let tokens = lex_line("who controls an artifact", 0)
        .expect("rewrite lexer should classify who-player predicate tail");
    let predicate = super::grammar::structure::parse_who_player_predicate_lexed(&tokens)
        .expect("structure helper should parse who-player predicate");
    let expected_tokens =
        lex_line("that player controls an artifact", 0).expect("expected predicate should lex");
    let expected =
        super::parse_predicate_lexed(&expected_tokens).expect("expected predicate should parse");

    assert_eq!(predicate, expected);
}

#[test]
fn rewrite_structure_instead_if_tail_parser_extracts_predicate() {
    let tokens = lex_line(
        "instead if there are seven or more cards in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify instead-if tail");
    let predicate = super::grammar::structure::parse_trailing_instead_if_predicate_lexed(&tokens)
        .expect("structure helper should parse instead-if tail predicate");
    let expected =
        super::parse_predicate_lexed(&tokens[2..]).expect("tail predicate should still parse");

    assert_eq!(predicate, expected);
}

#[test]
fn rewrite_structure_conditional_predicate_tail_parser_splits_instead_if_branch() {
    let tokens = lex_line("it's white instead if you control an artifact instead", 0)
        .expect("rewrite lexer should classify nested conditional predicate tail");
    let spec = super::grammar::structure::parse_conditional_predicate_tail_lexed(&tokens)
        .expect("structure helper should parse conditional predicate tail");
    let expected_base_tokens = lex_line("it's white", 0).expect("base predicate should lex");
    let expected_outer_tokens =
        lex_line("you control an artifact", 0).expect("outer predicate should lex");
    let expected_base =
        super::parse_predicate_lexed(&expected_base_tokens).expect("base predicate should parse");
    let expected_outer =
        super::parse_predicate_lexed(&expected_outer_tokens).expect("outer predicate should parse");

    assert_eq!(
        spec,
        super::grammar::structure::ConditionalPredicateTailSpec::InsteadIf {
            base_predicate: expected_base,
            outer_predicate: expected_outer,
        }
    );
}

#[test]
fn rewrite_structure_triggered_conditional_clause_parser_splits_intervening_if() {
    let tokens = lex_line(
        "At the beginning of your upkeep, if you control an artifact, draw a card.",
        0,
    )
    .expect("rewrite lexer should classify triggered conditional line");
    let spec = super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
        .expect("structure helper should detect triggered conditional clause");

    assert_eq!(
        spec.trigger_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["the", "beginning", "of", "your", "upkeep"]
    );
    assert_eq!(
        spec.effects_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["draw", "a", "card", "."]
    );
    assert!(format!("{:?}", spec.predicate).contains("PlayerControls"));
}

#[test]
fn rewrite_structure_state_triggered_clause_parser_splits_when_condition() {
    let tokens = lex_line("When you control no Swamps, sacrifice this creature.", 0)
        .expect("rewrite lexer should classify state-trigger line");
    let spec = super::grammar::structure::split_state_triggered_clause_lexed(&tokens, 1, 5)
        .expect("structure helper should detect state-trigger clause");

    assert_eq!(
        spec.display_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["When", "you", "control", "no", "Swamps"]
    );
    assert_eq!(
        spec.effects_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["sacrifice", "this", "creature", "."]
    );
    assert!(format!("{:?}", spec.predicate).contains("Swamp"));
}

#[test]
fn rewrite_modal_header_parser_tracks_unchosen_turn_scope() {
    let text = "Whenever another creature you control enters, choose one that hasn't been chosen this turn —";
    let header = super::modal_support::parse_modal_header(&rewrite_line_info(text))
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(header.trigger.is_some(), "{header:?}");
    assert!(header.mode_must_be_unchosen, "{header:?}");
    assert!(header.mode_must_be_unchosen_this_turn, "{header:?}");
}

#[test]
fn rewrite_modal_header_parser_supports_activated_choose_header_directly() {
    let text = "{T}: Choose one —";
    let header = super::modal_support::parse_modal_header(&rewrite_line_info(text))
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(header.activated.is_some(), "{header:?}");
    assert!(header.trigger.is_none(), "{header:?}");
    assert_eq!(header.min, crate::effect::Value::Fixed(1));
    assert_eq!(header.max, Some(crate::effect::Value::Fixed(1)));
}

#[test]
fn rewrite_modal_header_parser_keeps_choose_one_when_later_choose_both_is_present() {
    let text = "Choose one. If you control a commander as you cast this spell, you may choose both instead.";
    let header = super::modal_support::parse_modal_header(&rewrite_line_info(text))
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert_eq!(header.min, crate::effect::Value::Fixed(1));
    assert_eq!(header.max, Some(crate::effect::Value::Fixed(1)));
}

#[test]
fn rewrite_modal_header_parser_tracks_x_replacement_without_word_view_scan() {
    let text = "Choose one. X is the number of spells you've cast this turn —";
    let header = super::modal_support::parse_modal_header(&rewrite_line_info(text))
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert_eq!(
        header.x_replacement,
        Some(crate::effect::Value::SpellsCastThisTurn(
            crate::target::PlayerFilter::You
        ))
    );
}

#[test]
fn rewrite_modal_header_parser_keeps_prefix_effect_and_result_gate() {
    let text = "Whenever this creature enters or attacks, draw a card. If you do, choose one —";
    let header = super::modal_support::parse_modal_header(&rewrite_line_info(text))
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(header.trigger.is_some(), "{header:?}");
    assert!(!header.prefix_effects_ast.is_empty(), "{header:?}");
    assert!(matches!(
        header.modal_gate,
        Some(crate::cards::builders::ParsedModalGate {
            predicate: crate::effect::EffectPredicate::Happened,
            remove_mode_only: false,
        })
    ));
}

#[test]
fn rewrite_modal_header_parser_marks_remove_mode_only_gate() {
    let text = "Whenever this creature attacks, remove a +1/+1 counter from it. If you removed it this way, choose one —";
    let header = super::modal_support::parse_modal_header(&rewrite_line_info(text))
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(matches!(
        header.modal_gate,
        Some(crate::cards::builders::ParsedModalGate {
            predicate: crate::effect::EffectPredicate::Happened,
            remove_mode_only: true,
        })
    ));
}

#[test]
fn rewrite_modal_header_parse_all_reports_invalid_choose_clause() {
    let error = parse_error_message(super::modal_support::parse_modal_header(
        &rewrite_line_info("Whenever this creature enters, choose nonsense —"),
    ));

    assert!(
        error.contains("modal-header"),
        "expected modal-header adapter context, got {error}"
    );
    assert!(
        error.contains("modal choice range"),
        "expected choose-range context, got {error}"
    );
    assert!(
        error.contains("nonsense"),
        "expected failing token in adapter error, got {error}"
    );
}

#[test]
fn rewrite_modal_header_error_reports_line_and_span_after_activation_prefix_discrimination() {
    let header_line = "{T}: Choose nonsense —";
    let text = format!("Flash\n{header_line}\n• Draw a card.");
    let builder = CardDefinitionBuilder::new(CardId::new(), "Broken Activated Modal")
        .card_types(vec![CardType::Artifact]);
    let start = header_line
        .find("nonsense")
        .expect("test header should contain nonsense token");
    let end = start + "nonsense".len();
    let error = parse_error_message(parse_text_with_annotations_lowered(builder, text, false));

    assert!(
        error.contains("modal-header"),
        "expected modal-header adapter context, got {error}"
    );
    assert!(
        error.contains("modal choice range"),
        "expected choose-range context after activation prefix, got {error}"
    );
    assert!(
        error.contains(&format!("line 2 at {start}..{end}")),
        "expected original line/span after activation prefix discrimination, got {error}"
    );
    assert!(
        error.contains("near \"nonsense\""),
        "expected failing token context after activation prefix discrimination, got {error}"
    );
}

#[test]
fn rewrite_modal_header_error_reports_line_and_eof_after_trigger_prefix_discrimination() {
    let header_line = "Whenever this creature attacks, choose up to";
    let text = format!("Flying\n{header_line}\n• Draw a card.");
    let builder = CardDefinitionBuilder::new(CardId::new(), "Broken Trigger Modal")
        .card_types(vec![CardType::Creature]);
    let start = header_line
        .find("up")
        .expect("test header should contain partial range token");
    let end = start + "up".len();
    let error = parse_error_message(parse_text_with_annotations_lowered(builder, text, false));

    assert!(
        error.contains("modal-header"),
        "expected modal-header adapter context, got {error}"
    );
    assert!(
        error.contains("modal choice range"),
        "expected choose-range cut context after trigger prefix, got {error}"
    );
    assert!(
        error.contains(&format!("line 2 at {start}..{end}")),
        "expected original line/span after trigger prefix discrimination, got {error}"
    );
    assert!(
        error.contains("near \"up\""),
        "expected failing token context after trigger prefix discrimination, got {error}"
    );
}

#[test]
fn rewrite_document_parser_supports_activate_only_once_each_turn_without_period() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Activated Limit Variant")
        .card_types(vec![CardType::Artifact]);
    let preprocessed =
        super::preprocess::preprocess_document(builder, "Equip {0}\nActivate only once each turn")
            .expect(
                "expected preprocessing to accept activate-only-once line without trailing period",
            );
    let cst = super::document_parser::parse_document_cst(&preprocessed, false).expect(
        "expected document parser to accept activate-only-once line without trailing period",
    );

    assert!(
        cst.lines.iter().any(|line| {
            matches!(
                line,
                super::cst::RewriteLineCst::Static(static_line)
                    if static_line.text == "activate only once each turn"
            )
        }),
        "expected static CST line for activate-only-once line, got {cst:?}"
    );
}

#[test]
fn rewrite_document_parser_splits_activation_cost_on_colon_outside_quotes() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Quoted Colon Variant")
        .card_types(vec![CardType::Artifact]);
    let preprocessed =
        super::preprocess::preprocess_document(builder, "{T}: Choose \"fire: ice\".")
            .expect("expected preprocessing to accept quoted-colon activation line");
    let cst = super::document_parser::parse_document_cst(&preprocessed, false)
        .expect("expected document parser to split activation on colon outside quotes");

    let activated = cst
        .lines
        .iter()
        .find_map(|line| match line {
            super::cst::RewriteLineCst::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated CST line");

    assert!(
        activated.effect_text.contains("fire: ice"),
        "expected quoted inner colon to stay in effect text, got {:?}",
        activated.effect_text
    );
}

#[test]
fn rewrite_document_parser_splits_nonactivation_colon_outside_quotes() {
    let tokens = lex_line("Reveal this card from your hand: \"fire: ice\".", 0)
        .expect("expected lexer to accept quoted-colon non-activation line");
    let (left, right) = super::document_parser::split_lexed_once_on_colon_outside_quotes(&tokens)
        .expect("expected shared colon helper to split on the outer colon only");

    assert_eq!(
        render_token_slice(left).trim(),
        "Reveal this card from your hand"
    );
    assert_eq!(render_token_slice(right).trim(), "\"fire: ice\".");
}

#[test]
fn rewrite_document_parser_dispatches_keyword_lines_by_head_phrase() -> Result<(), CardTextError> {
    let alt_preprocessed = super::preprocess::preprocess_document(
        CardDefinitionBuilder::new(CardId::new(), "Alt Cost Variant")
            .card_types(vec![CardType::Instant]),
        "If an opponent cast two or more spells this turn, you may pay {1}{R} rather than pay this spell's mana cost.",
    )?;
    let alt_cst = super::document_parser::parse_document_cst(&alt_preprocessed, false)?;
    assert!(matches!(
        alt_cst.lines.as_slice(),
        [super::cst::RewriteLineCst::Keyword(keyword)]
            if matches!(keyword.kind, super::cst::KeywordLineKindCst::AlternativeCast)
    ));

    let gift_preprocessed = super::preprocess::preprocess_document(
        CardDefinitionBuilder::new(CardId::new(), "Gift Variant")
            .card_types(vec![CardType::Sorcery]),
        "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)",
    )?;
    let gift_cst = super::document_parser::parse_document_cst(&gift_preprocessed, false)?;
    assert!(matches!(
        gift_cst.lines.as_slice(),
        [super::cst::RewriteLineCst::Keyword(keyword)]
            if matches!(keyword.kind, super::cst::KeywordLineKindCst::Gift)
    ));

    Ok(())
}

#[test]
fn rewrite_static_lowering_reuses_token_sentences_for_multi_sentence_lines()
-> Result<(), CardTextError> {
    let text =
        "this creature has flying. as long as you control an artifact, this creature has haste.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify multi-sentence static line");

    let parsed =
        super::lower_rewrite_static_to_chunk(rewrite_line_info(text), text, &tokens, None)?;

    match parsed {
        crate::cards::builders::LineAst::StaticAbilities(abilities) => {
            assert_eq!(abilities.len(), 2);
        }
        other => panic!("expected split static abilities, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_static_lowering_reuses_token_split_for_compound_unblockable_line()
-> Result<(), CardTextError> {
    let text = "enchanted creature gets +2/+2 and can't be blocked.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify compound buff static line");

    let parsed =
        super::lower_rewrite_static_to_chunk(rewrite_line_info(text), text, &tokens, None)?;

    match parsed {
        crate::cards::builders::LineAst::StaticAbilities(abilities) => {
            assert_eq!(abilities.len(), 2);
        }
        other => panic!("expected split compound static abilities, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_keyword_lowering_reuses_token_sentences_for_optional_cost_cast_trigger()
-> Result<(), CardTextError> {
    let text = "as an additional cost to cast this spell, you may sacrifice one or more creatures. when you do, copy this spell for each creature sacrificed this way.";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify additional-cost cast-trigger keyword line");

    let parsed = super::lower_rewrite_keyword_to_chunk(
        rewrite_line_info(text),
        text,
        &tokens,
        RewriteKeywordLineKind::AdditionalCost,
    )?;

    match parsed {
        crate::cards::builders::LineAst::OptionalCostWithCastTrigger {
            effects,
            followup_text,
            ..
        } => {
            assert!(!effects.is_empty());
            assert_eq!(
                followup_text,
                "When you do, copy this spell for each creature sacrificed this way"
            );
        }
        other => panic!("expected optional-cost cast trigger line, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_statement_lowering_reuses_full_token_slice_for_pact_line() -> Result<(), CardTextError> {
    let text = "search your library for a green creature card, reveal it, put it into your hand, then shuffle. at the beginning of your next upkeep, pay {2}{G}{G}. if you don't, you lose the game.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify pact statement line");

    let parsed_chunks = super::lower_rewrite_statement_token_groups_to_chunks(
        rewrite_line_info(text),
        text,
        &tokens,
        &[],
    )?;

    match parsed_chunks.as_slice() {
        [crate::cards::builders::LineAst::Statement { effects }] => {
            assert!(matches!(
                effects.last(),
                Some(crate::cards::builders::EffectAst::DelayedUntilNextUpkeep { .. })
            ));
        }
        other => panic!("expected single pact statement chunk, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_statement_lowering_uses_parse_tokens_when_groups_are_missing()
-> Result<(), CardTextError> {
    let token_text = "Meteor Strikes — Exile target artifact. When you do, draw a card.";
    let tokens =
        lex_line(token_text, 0).expect("rewrite lexer should classify statement token fallback");

    let parsed_chunks = super::lower_rewrite_statement_token_groups_to_chunks(
        rewrite_line_info("placeholder statement text"),
        "placeholder statement text",
        &tokens,
        &[],
    )?;

    match parsed_chunks.as_slice() {
        [crate::cards::builders::LineAst::Statement { effects }] => {
            let debug = format!("{effects:?}");
            assert!(debug.contains("Exile"), "{debug}");
            assert!(debug.contains("IfResult"), "{debug}");
            assert!(debug.contains("Draw"), "{debug}");
        }
        other => panic!("expected single rewritten statement chunk, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_triggered_lowering_uses_parse_tokens_when_text_fields_are_stale()
-> Result<(), CardTextError> {
    let full_text = "when this creature attacks, draw a card.";
    let trigger_text = "when this creature attacks";
    let effect_text = "draw a card.";
    let full_tokens =
        lex_line(full_text, 0).expect("rewrite lexer should classify triggered token fallback");
    let trigger_tokens =
        lex_line(trigger_text, 0).expect("rewrite lexer should classify triggered condition");
    let effect_tokens =
        lex_line(effect_text, 0).expect("rewrite lexer should classify triggered effect");

    let parsed = super::lower_rewrite_triggered_to_chunk(
        rewrite_line_info("placeholder triggered text"),
        "placeholder triggered text",
        &full_tokens,
        "placeholder trigger text",
        &trigger_tokens,
        "placeholder effect text",
        &effect_tokens,
        None,
        None,
    )?;

    let debug = format!("{parsed:?}");
    assert!(debug.contains("Triggered"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");

    Ok(())
}

#[test]
fn rewrite_gift_keyword_lowering_builds_closed_form_followup_effects() -> Result<(), CardTextError>
{
    let cases = [
        (
            "gift a card (you may promise an opponent a gift as you cast this spell. if you do, they draw a card before its other effects.)",
            "the chosen player draws a card.",
            crate::cards::builders::GiftTimingAst::SpellResolution,
        ),
        (
            "gift a tapped fish (you may promise an opponent a gift as you cast this spell. if you do, they create a tapped 1/1 blue fish creature token before its other effects.)",
            "the chosen player creates a tapped 1/1 blue Fish creature token.",
            crate::cards::builders::GiftTimingAst::SpellResolution,
        ),
        (
            "gift an extra turn (you may promise an opponent a gift as you cast this spell. if you do, they take an extra turn after this one before its other effects.)",
            "the chosen player takes an extra turn after this one.",
            crate::cards::builders::GiftTimingAst::SpellResolution,
        ),
    ];

    for (text, expected_followup, expected_timing) in cases {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify gift keyword line");
        let parsed = super::lower_rewrite_keyword_to_chunk(
            rewrite_line_info(text),
            text,
            &tokens,
            RewriteKeywordLineKind::Gift,
        )?;

        match parsed {
            crate::cards::builders::LineAst::GiftKeyword {
                effects,
                followup_text,
                timing,
                ..
            } => {
                assert_eq!(followup_text, expected_followup);
                assert!(
                    matches!(
                        (&timing, &expected_timing),
                        (
                            crate::cards::builders::GiftTimingAst::SpellResolution,
                            crate::cards::builders::GiftTimingAst::SpellResolution
                        ) | (
                            crate::cards::builders::GiftTimingAst::PermanentEtb,
                            crate::cards::builders::GiftTimingAst::PermanentEtb
                        )
                    ),
                    "expected gift timing {expected_timing:?}, got {timing:?}"
                );
                match expected_followup {
                    "the chosen player draws a card." => assert!(matches!(
                        effects.as_slice(),
                        [crate::cards::builders::EffectAst::Draw {
                            count: crate::effect::Value::Fixed(1),
                            player: crate::cards::builders::PlayerAst::Chosen,
                        }]
                    )),
                    "the chosen player creates a tapped 1/1 blue Fish creature token." => {
                        assert!(matches!(
                            effects.as_slice(),
                            [crate::cards::builders::EffectAst::CreateTokenWithMods {
                                name,
                                count: crate::effect::Value::Fixed(1),
                                player: crate::cards::builders::PlayerAst::Chosen,
                                tapped: true,
                                ..
                            }] if name == "1/1 blue Fish creature"
                        ))
                    }
                    "the chosen player takes an extra turn after this one." => assert!(matches!(
                        effects.as_slice(),
                        [crate::cards::builders::EffectAst::ExtraTurnAfterTurn {
                            player: crate::cards::builders::PlayerAst::Chosen,
                            anchor: crate::cards::builders::ExtraTurnAnchorAst::CurrentTurn,
                        }]
                    )),
                    other => panic!("unexpected gift followup in test case: {other}"),
                }
            }
            other => panic!("expected gift keyword line, got {other:?}"),
        }
    }

    Ok(())
}

#[test]
fn rewrite_token_word_view_caches_lower_words_and_word_token_indices() {
    let tokens = lex_line("Activate only during your turn.", 0)
        .expect("rewrite lexer should classify restriction text");
    let words = TokenWordView::new(&tokens);
    assert_eq!(words.get(0), Some("activate"));
    assert_eq!(words.get(3), Some("your"));
    assert_eq!(words.token_index_for_word_index(4), Some(4));
    assert!(words.starts_with(&["activate", "only"]));
    assert!(words.has_phrase(&["during", "your", "turn"]));
}

#[test]
fn rewrite_token_word_view_normalizes_parser_word_shapes() {
    let tokens = lex_line("Its controller's face-down creature gets {W/U}.", 0)
        .expect("rewrite lexer should classify mixed word shapes");
    let words = TokenWordView::new(&tokens);

    assert_eq!(
        words.to_word_refs(),
        vec![
            "its",
            "controllers",
            "face",
            "down",
            "creature",
            "gets",
            "w/u"
        ]
    );
    assert_eq!(words.token_index_for_word_index(2), Some(2));
    assert_eq!(words.token_index_after_words(4), Some(3));
    assert_eq!(words.token_index_after_words(5), Some(4));
}

#[test]
fn rewrite_token_word_view_centralizes_token_shape_policy() {
    let text = "Their owners' face-down power/toughness gets -1/-1 and {W/U}.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify shared token-shape line");
    let words = TokenWordView::new(&tokens);
    assert_eq!(
        words.to_word_refs(),
        vec![
            "their",
            "owners",
            "face",
            "down",
            "power",
            "toughness",
            "gets",
            "-1/-1",
            "and",
            "w/u"
        ]
    );
    assert_eq!(
        super::token_word_refs(&tokens),
        vec![
            "Their",
            "owners'",
            "face-down",
            "power/toughness",
            "gets",
            "-1/-1",
            "and"
        ]
    );
}

#[test]
fn rewrite_owned_lex_token_replace_word_refreshes_cached_parser_word_pieces() {
    let mut token = lex_line("face-down", 0)
        .expect("rewrite lexer should classify split word token")
        .into_iter()
        .next()
        .expect("expected one token");

    assert_eq!(
        super::lexer::parser_token_word_refs(std::slice::from_ref(&token)),
        vec!["face", "down"]
    );

    assert!(token.replace_word("controllers'"));
    assert_eq!(
        super::lexer::parser_token_word_refs(std::slice::from_ref(&token)),
        vec!["controllers"]
    );
}

#[test]
fn rewrite_rule_engine_lex_clause_view_normalizes_parser_word_shapes() {
    let tokens = lex_line(
        "Whenever its owner's face-down creature attacks, draw a card.",
        0,
    )
    .expect("rewrite lexer should classify rule-engine clause");
    let view = super::LexClauseView::from_tokens(&tokens);

    assert_eq!(view.head(), "whenever");
    assert_eq!(
        view.words.to_word_refs(),
        vec![
            "whenever", "its", "owners", "face", "down", "creature", "attacks", "draw", "a", "card"
        ]
    );
    assert_eq!(
        view.shape,
        super::RULE_SHAPE_STARTS_WHENEVER | super::RULE_SHAPE_HAS_COMMA
    );
    assert_eq!(
        view.display_text(),
        "whenever its owners face down creature attacks draw a card"
    );
}

#[test]
fn rewrite_token_word_view_find_preserves_word_indices() {
    let tokens = lex_line("Whenever one or more cards are exiled this way", 0)
        .expect("rewrite lexer should classify followup text");
    let words = TokenWordView::new(&tokens);

    assert_eq!(words.find_word("whenever"), Some(0));
    assert_eq!(words.find_word("this"), Some(7));
    assert_eq!(words.find_word("way"), Some(8));
}

#[test]
fn rewrite_parser_support_detects_this_way_followup_intro() {
    let tokens = lex_line("Whenever one or more cards are exiled this way", 0)
        .expect("rewrite lexer should classify followup text");
    let plain_tokens = lex_line("Whenever one or more cards are exiled", 0)
        .expect("rewrite lexer should classify non-followup text");

    assert!(super::looks_like_spell_resolution_followup_intro_lexed(
        &tokens
    ));
    assert!(!super::looks_like_spell_resolution_followup_intro_lexed(
        &plain_tokens
    ));
}

#[test]
fn rewrite_parser_support_detects_when_you_do_followup_intro() {
    let tokens = lex_line("When you do, exile target creature.", 0)
        .expect("rewrite lexer should classify reflexive followup text");
    let delayed_tokens = lex_line("At the beginning of the next end step, draw a card.", 0)
        .expect("rewrite lexer should classify delayed trigger text");

    assert!(super::looks_like_reflexive_followup_intro_lexed(&tokens));
    assert!(!super::looks_like_reflexive_followup_intro_lexed(
        &delayed_tokens
    ));
}

#[test]
fn rewrite_parser_support_splits_quoted_sentences_and_queues_restrictions() {
    let (parsed_sentences, restrictions) = super::parser_support::split_text_for_parse(
        "Draw a card. \"Choose one.\" Activate only during your turn.",
        "Draw a card. \"Choose one.\" Activate only during your turn.",
        0,
    );

    assert_eq!(
        parsed_sentences,
        vec!["Draw a card".to_string(), "\"Choose one.\"".to_string()]
    );
    assert_eq!(
        restrictions.activation,
        vec!["Activate only during your turn".to_string()]
    );
    assert!(restrictions.trigger.is_empty());
}

#[test]
fn rewrite_lexed_restriction_parsers_match_activation_trigger_and_mana_shapes() {
    let activate_only = lex_line("Activate only during your turn.", 0)
        .expect("rewrite lexer should classify activation restriction");
    let trigger_only = lex_line("This ability triggers only once each turn.", 0)
        .expect("rewrite lexer should classify trigger restriction");
    let mana_only = lex_line(
        "Spend this mana only to cast artifact spells of the chosen type and that spell can't be countered.",
        0,
    )
    .expect("rewrite lexer should classify mana restriction");

    assert_eq!(
        parse_activate_only_timing_lexed(&activate_only),
        Some(crate::ability::ActivationTiming::DuringYourTurn)
    );
    assert_eq!(
        parse_triggered_times_each_turn_lexed(&trigger_only),
        Some(1)
    );
    assert!(matches!(
        parse_mana_usage_restriction_sentence_lexed(&mana_only),
        Some(crate::ability::ManaUsageRestriction::CastSpell {
            card_types,
            subtype_requirement: Some(
                crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource
            ),
            restrict_to_matching_spell: true,
            grant_uncounterable: true,
            enters_with_counters,
        }) if card_types == vec![CardType::Artifact]
            && enters_with_counters.is_empty()
    ));
}

#[test]
fn rewrite_restriction_support_preserves_text_only_attack_conditions() {
    let mut attacked_ability = crate::ability::ActivatedAbility {
        mana_cost: crate::cost::TotalCost::default(),
        effects: crate::resolution::ResolutionProgram::default(),
        choices: vec![],
        timing: crate::ability::ActivationTiming::AnyTime,
        additional_restrictions: vec![],
        activation_restrictions: vec![],
        mana_output: None,
        activation_condition: None,
        mana_usage_restrictions: vec![],
    };
    super::restriction_support::apply_pending_activation_restriction(
        &mut attacked_ability,
        "Activate only once each turn and only if this creature attacked this turn",
    );

    assert_eq!(
        attacked_ability.timing,
        crate::ability::ActivationTiming::OncePerTurn
    );
    assert_eq!(
        attacked_ability.additional_restrictions,
        vec!["only if this creature attacked this turn".to_string()]
    );
    assert!(
        attacked_ability
            .activation_restrictions
            .iter()
            .any(|condition| matches!(condition, crate::ConditionExpr::SourceAttackedThisTurn))
    );

    let mut didnt_attack_ability = crate::ability::ActivatedAbility {
        mana_cost: crate::cost::TotalCost::default(),
        effects: crate::resolution::ResolutionProgram::default(),
        choices: vec![],
        timing: crate::ability::ActivationTiming::AnyTime,
        additional_restrictions: vec![],
        activation_restrictions: vec![],
        mana_output: None,
        activation_condition: None,
        mana_usage_restrictions: vec![],
    };
    super::restriction_support::apply_pending_activation_restriction(
        &mut didnt_attack_ability,
        "Activate only if it didn't attack this turn and only once each turn",
    );

    assert_eq!(
        didnt_attack_ability.timing,
        crate::ability::ActivationTiming::OncePerTurn
    );
    assert_eq!(
        didnt_attack_ability.additional_restrictions,
        vec!["activate only if it didn't attack this turn".to_string()]
    );
    assert!(
        didnt_attack_ability
            .activation_restrictions
            .iter()
            .any(|condition| matches!(
                condition,
                crate::ConditionExpr::Not(inner)
                    if matches!(inner.as_ref(), crate::ConditionExpr::SourceAttackedThisTurn)
            ))
    );
}

#[test]
fn rewrite_zone_counter_helpers_parse_put_or_remove_counter_modes() {
    let tokens = lex_line(
        "Put a +1/+1 counter on target creature or remove a counter from it",
        0,
    )
    .expect("rewrite lexer should classify put-or-remove counter clause");

    let parsed = super::parse_effect_sentence_lexed(&tokens)
        .expect("put-or-remove counter clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("UnlessAction"), "{debug}");
    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("RemoveUpToAnyCounters"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
}

#[test]
fn rewrite_zone_counter_helpers_parse_multiple_counter_sentence() {
    let tokens = lex_line(
        "Put a +1/+1 counter and a flying counter on target creature",
        0,
    )
    .expect("rewrite lexer should classify multi-counter clause");

    let parsed = super::parse_sentence_put_multiple_counters_on_target(&tokens)
        .expect("multi-counter clause should parse");

    assert_eq!(parsed.as_ref().map(Vec::len), Some(2), "{parsed:?}");
}

#[test]
fn rewrite_zone_counter_helpers_parse_for_each_spells_youve_cast_this_turn() {
    let tokens = lex_line(
        "Put a +1/+1 counter on target creature for each spell you've cast this turn.",
        0,
    )
    .expect("rewrite lexer should classify for-each spell-count counter clause");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("for-each spell-count counter clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("SpellsCastThisTurn(You)"), "{debug}");
}

#[test]
fn rewrite_zone_counter_helpers_keep_trailing_if_counter_clause_after_structure_cutover() {
    let tokens = lex_line("Put a +1/+1 counter on target creature if it's white.", 0)
        .expect("rewrite lexer should classify conditional counter clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("counter clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(matches!(
                predicate,
                crate::cards::builders::PredicateAst::ItMatches(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::PutCounters { .. }]
            ));
        }
        other => panic!("expected conditional put-counters clause, got {other:?}"),
    }
}

#[test]
fn rewrite_verb_handlers_keep_trailing_if_counter_clause_after_structure_cutover() {
    let tokens = lex_line("Counter target spell if it's white.", 0)
        .expect("rewrite lexer should classify conditional counter spell clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("counter spell clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(matches!(
                predicate,
                crate::cards::builders::PredicateAst::ItMatches(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::Counter { .. }]
            ));
        }
        other => panic!("expected conditional counter clause, got {other:?}"),
    }
}

#[test]
fn rewrite_verb_handlers_keep_trailing_if_damage_clause_after_structure_cutover() {
    let tokens = lex_line(
        "This creature deals 3 damage to target creature if it's white.",
        0,
    )
    .expect("rewrite lexer should classify conditional damage clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("damage clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(matches!(
                predicate,
                crate::cards::builders::PredicateAst::ItMatches(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::DealDamage { .. }]
            ));
        }
        other => panic!("expected conditional damage clause, got {other:?}"),
    }
}

#[test]
fn rewrite_verb_handlers_keep_trailing_instead_if_damage_clause_after_structure_cutover() {
    let tokens = lex_line(
        "This creature deals 5 damage to target creature instead if it's white.",
        0,
    )
    .expect("rewrite lexer should classify instead-if damage clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("instead-if damage clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(matches!(
                predicate,
                crate::cards::builders::PredicateAst::ItMatches(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::DealDamage { .. }]
            ));
        }
        other => panic!("expected conditional instead-if damage clause, got {other:?}"),
    }
}

#[test]
fn rewrite_verb_handlers_keep_trailing_if_draw_clause_after_structure_cutover() {
    let tokens = lex_line("Draw a card if you control an artifact.", 0)
        .expect("rewrite lexer should classify conditional draw clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("draw clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(!matches!(
                predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::Draw { .. }]
            ));
        }
        other => panic!("expected conditional draw clause, got {other:?}"),
    }
}

#[test]
fn rewrite_verb_handlers_keep_draw_for_each_player_condition_after_structure_cutover() {
    let tokens = lex_line("Draw a card for each player who controls an artifact.", 0)
        .expect("rewrite lexer should classify draw-for-each-player clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("draw-for-each clause should parse");

    match parsed.as_slice() {
        [crate::cards::builders::EffectAst::ForEachPlayer { effects }] => {
            match effects.as_slice() {
                [
                    crate::cards::builders::EffectAst::Conditional {
                        predicate,
                        if_true,
                        if_false,
                    },
                ] => {
                    assert!(if_false.is_empty());
                    assert!(!matches!(
                        predicate,
                        crate::cards::builders::PredicateAst::Unmodeled(_)
                    ));
                    assert!(matches!(
                        if_true.as_slice(),
                        [crate::cards::builders::EffectAst::Draw { .. }]
                    ));
                }
                other => panic!("expected conditional draw effect, got {other:?}"),
            }
        }
        other => panic!("expected for-each-player draw clause, got {other:?}"),
    }
}

#[test]
fn rewrite_verb_handlers_keep_conditional_gain_control_clause_after_structure_cutover() {
    let tokens = lex_line(
        "Gain control of target creature if you control an artifact until end of turn.",
        0,
    )
    .expect("rewrite lexer should classify conditional gain-control clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("conditional gain-control clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(!matches!(
                predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::GainControl { .. }]
            ));
        }
        other => panic!("expected conditional gain-control clause, got {other:?}"),
    }
}

#[test]
fn rewrite_verb_handlers_keep_unless_gain_control_clause_after_structure_cutover() {
    let tokens = lex_line(
        "Gain control of target creature unless you control an artifact until end of turn.",
        0,
    )
    .expect("rewrite lexer should classify unless gain-control clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("unless gain-control clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_true.is_empty());
            assert!(!matches!(
                predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
            assert!(matches!(
                if_false.as_slice(),
                [crate::cards::builders::EffectAst::GainControl { .. }]
            ));
        }
        other => panic!("expected unless gain-control clause, got {other:?}"),
    }
}

#[test]
fn rewrite_etb_where_x_source_stat_normalizes_apostrophe_shapes() {
    let tokens = lex_line("Where X is this creature's power", 0)
        .expect("rewrite lexer should classify where-x source-stat clause");

    let parsed = super::keyword_static::parse_where_x_source_stat_value(&tokens);

    assert!(matches!(parsed, Some(crate::effect::Value::SourcePower)));
}

#[test]
fn rewrite_etb_enters_tapped_filter_preserves_played_by_opponents_suffix() {
    let tokens = lex_line("Artifacts played by your opponents enter tapped.", 0)
        .expect("rewrite lexer should classify enters-tapped filter clause");

    let ability = super::keyword_static::parse_enters_tapped_for_filter_line(&tokens)
        .expect("enters-tapped filter clause should parse")
        .expect("enters-tapped filter clause should build a static ability");
    let debug = format!("{ability:?}");

    assert!(debug.contains("card_types: [Artifact]"), "{debug}");
    assert!(debug.contains("controller: Some(Opponent)"), "{debug}");
}

#[test]
fn rewrite_etb_where_x_aggregate_filter_routes_and_split_through_grammar_separator_helper() {
    let tokens = lex_line(
        "where x is the total power of creatures you control and creature cards in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify aggregate where-x clause");

    let parsed = super::keyword_static::parse_where_x_is_aggregate_filter_value(&tokens)
        .expect("aggregate where-x clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("TotalPower"), "{debug}");
    assert!(debug.contains("any_of"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
    assert!(debug.contains("zone: Some(Graveyard)"), "{debug}");
}

#[test]
fn rewrite_etb_where_x_total_power_of_sacrificed_creatures_uses_the_sacrifice_reference() {
    let tokens = lex_line("where x is the total power of the sacrificed creatures", 0)
        .expect("rewrite lexer should classify sacrificed aggregate clause");

    let parsed = super::keyword_static::parse_where_x_is_aggregate_filter_value(&tokens)
        .expect("sacrificed aggregate clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("TotalPower"), "{debug}");
    assert!(
        debug.contains("tag: TagKey(\"__it__\")") || debug.contains("tag: TagKey(\"sacrificed"),
        "expected sacrificed creatures to stay tied to a tag, got {debug}"
    );
    assert!(
        !debug.contains("zone: Some(Battlefield)"),
        "sacrificed creatures should not be collapsed to a battlefield-only filter, got {debug}"
    );
}

#[test]
fn rewrite_zone_handlers_keep_conditional_destroy_clause_after_structure_cutover() {
    let tokens = lex_line("Destroy target creature if it's white.", 0)
        .expect("rewrite lexer should classify conditional destroy clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("destroy clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(matches!(
                predicate,
                crate::cards::builders::PredicateAst::ItMatches(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::Destroy { .. }]
            ));
        }
        other => panic!("expected conditional destroy clause, got {other:?}"),
    }
}

#[test]
fn rewrite_zone_handlers_keep_nested_instead_if_destroy_clause_after_structure_cutover() {
    let tokens = lex_line(
        "Destroy target creature if it's white instead if you control an artifact.",
        0,
    )
    .expect("rewrite lexer should classify nested instead-if destroy clause");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("nested instead-if destroy clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate: outer_predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(!matches!(
                outer_predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
            match if_true.as_slice() {
                [
                    crate::cards::builders::EffectAst::Conditional {
                        predicate: base_predicate,
                        if_true: nested_if_true,
                        if_false: nested_if_false,
                    },
                ] => {
                    assert!(nested_if_false.is_empty());
                    assert!(matches!(
                        base_predicate,
                        crate::cards::builders::PredicateAst::ItMatches(_)
                    ));
                    assert!(matches!(
                        nested_if_true.as_slice(),
                        [crate::cards::builders::EffectAst::Destroy { .. }]
                    ));
                }
                other => panic!("expected nested conditional destroy branch, got {other:?}"),
            }
        }
        other => panic!("expected nested instead-if destroy clause, got {other:?}"),
    }
}

#[test]
fn rewrite_zone_handlers_keep_conditional_exile_clause_after_structure_cutover() {
    let tokens = lex_line("Exile target creature if it's white.", 0)
        .expect("rewrite lexer should classify conditional exile clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("exile clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(matches!(
                predicate,
                crate::cards::builders::PredicateAst::ItMatches(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::Exile { .. }]
            ));
        }
        other => panic!("expected conditional exile clause, got {other:?}"),
    }
}

#[test]
fn rewrite_zone_counter_helpers_parse_half_starting_life_total_variants() {
    let your_tokens = lex_line("half your starting life total", 0)
        .expect("rewrite lexer should classify half-life value");
    let target_tokens = lex_line("half target player's starting life total rounded down", 0)
        .expect("rewrite lexer should classify rounded-down half-life value");

    assert_eq!(
        super::parse_half_starting_life_total_value(
            &your_tokens,
            crate::cards::builders::PlayerAst::Implicit,
        ),
        Some(crate::effect::Value::HalfStartingLifeTotalRoundedUp(
            crate::target::PlayerFilter::You,
        ))
    );
    assert_eq!(
        super::parse_half_starting_life_total_value(
            &target_tokens,
            crate::cards::builders::PlayerAst::Target,
        ),
        Some(crate::effect::Value::HalfStartingLifeTotalRoundedDown(
            crate::target::PlayerFilter::target_player(),
        ))
    );
}

#[test]
fn rewrite_activation_helpers_cover_color_choice_mana_helpers() {
    let or_tokens =
        lex_line("{W}, {U}, or {B}", 0).expect("rewrite lexer should classify color choices");
    let combination_tokens = lex_line("any combination of {W}, {U}, or {R}", 0)
        .expect("rewrite lexer should classify any-combination mana");
    let land_filter_tokens = lex_line("that a land an opponent controls could produce", 0)
        .expect("rewrite lexer should classify land filter tail");

    assert_eq!(
        super::activation_helpers::parse_or_mana_color_choices(&or_tokens)
            .expect("or-choice mana colors should parse"),
        Some(vec![
            crate::color::Color::White,
            crate::color::Color::Blue,
            crate::color::Color::Black,
        ])
    );
    assert_eq!(
        super::activation_helpers::parse_any_combination_mana_colors(&combination_tokens)
            .expect("any-combination mana colors should parse"),
        Some(vec![
            crate::color::Color::White,
            crate::color::Color::Blue,
            crate::color::Color::Red,
        ])
    );
    assert!(matches!(
        super::activation_helpers::parse_land_could_produce_filter(&land_filter_tokens)
            .expect("land could produce tail should parse"),
        Some(filter)
            if filter.card_types == vec![CardType::Land]
                && filter.controller == Some(crate::target::PlayerFilter::Opponent)
    ));
}

#[test]
fn rewrite_activation_helpers_parse_add_mana_preserves_chosen_color_tail() {
    let tokens = lex_line("{R} or one mana of the chosen color", 0)
        .expect("rewrite lexer should classify chosen-color mana clause");

    assert!(matches!(
        super::activation_helpers::parse_add_mana(&tokens, None)
            .expect("chosen-color mana clause should parse"),
        crate::cards::builders::EffectAst::AddManaChosenColor {
            amount: crate::effect::Value::Fixed(1),
            player: crate::cards::builders::PlayerAst::Implicit,
            fixed_option: Some(crate::color::Color::Red),
        }
    ));
}

#[test]
fn rewrite_activation_helpers_parse_add_mana_wraps_instead_if_tail() {
    let tokens = lex_line(
        "{B}{B}{B}{B}{B} instead if there are seven or more cards in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify conditional mana clause");

    let effect = super::activation_helpers::parse_add_mana(&tokens, None)
        .expect("conditional mana clause should parse");

    match effect {
        crate::cards::builders::EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            assert!(if_false.is_empty());
            match if_true.as_slice() {
                [crate::cards::builders::EffectAst::AddMana { mana, player }] => {
                    assert_eq!(player, &crate::cards::builders::PlayerAst::Implicit);
                    assert_eq!(
                        mana.as_slice(),
                        &[
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                        ]
                    );
                }
                other => panic!("expected add-mana branch, got {other:?}"),
            }
            assert!(!matches!(
                predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
        }
        other => panic!("expected conditional add-mana effect, got {other:?}"),
    }
}

#[test]
fn rewrite_activation_helpers_parse_add_mana_accepts_player_choice_tail_without_word_view() {
    let tokens = lex_line("one mana of any color that player chooses", 0)
        .expect("rewrite lexer should classify player-choice mana clause");

    assert!(matches!(
        super::activation_helpers::parse_add_mana(&tokens, None)
            .expect("player-choice mana clause should parse"),
        crate::cards::builders::EffectAst::AddManaAnyColor {
            amount: crate::effect::Value::Fixed(1),
            player: crate::cards::builders::PlayerAst::Implicit,
            available_colors: None,
        }
    ));
}

#[test]
fn rewrite_activation_helpers_normalize_player_apostrophe_in_mana_pool_tail() {
    let tokens = lex_line("to that player's mana pool", 0)
        .expect("rewrite lexer should classify mana-pool tail");

    assert!(super::activation_helpers::is_mana_pool_tail_tokens(&tokens));
}

#[test]
fn rewrite_effect_sentence_parse_add_mana_wraps_instead_if_tail() {
    let tokens = lex_line(
        "Add {B}{B}{B}{B}{B} instead if there are seven or more cards in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify mana sentence");

    let effects = parse_effect_sentence_lexed(&tokens).expect("mana sentence should parse");

    match effects.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            match if_true.as_slice() {
                [crate::cards::builders::EffectAst::AddMana { mana, player }] => {
                    assert_eq!(player, &crate::cards::builders::PlayerAst::Implicit);
                    assert_eq!(
                        mana.as_slice(),
                        &[
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                        ]
                    );
                }
                other => panic!("expected add-mana branch, got {other:?}"),
            }
            assert!(!matches!(
                predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
        }
        other => panic!("expected single conditional add-mana effect, got {other:?}"),
    }
}

#[test]
fn rewrite_lexed_activation_condition_parser_handles_control_and_graveyard_conditions() {
    let graveyard = lex_line(
        "Activate only if there is an artifact card in your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify graveyard condition");
    let control = lex_line("Activate only if you control three or more artifacts.", 0)
        .expect("rewrite lexer should classify control condition");

    assert!(matches!(
        parse_activation_condition_lexed(&graveyard),
        Some(crate::ConditionExpr::CardInYourGraveyard { card_types, subtypes })
            if card_types == vec![CardType::Artifact] && subtypes.is_empty()
    ));
    assert!(matches!(
        parse_activation_condition_lexed(&control),
        Some(crate::ConditionExpr::PlayerControlsAtLeast {
            player: crate::target::PlayerFilter::You,
            count: 3,
            ..
        })
    ));
}

#[test]
fn rewrite_lexed_spell_filter_parser_preserves_native_shape() {
    let tokens = lex_line("face-down noncreature spells", 0)
        .expect("rewrite lexer should classify spell filter text");
    let filter = super::parse_spell_filter_lexed(&tokens);

    assert_eq!(filter.face_down, Some(true));
    assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
}

#[test]
fn rewrite_lexed_object_filter_tracks_spell_caster_and_origin_zone() {
    let tokens = lex_line("enchantment spells you cast from your hand", 0)
        .expect("rewrite lexer should classify spell grant filter text");
    let filter =
        super::parse_object_filter_lexed(&tokens, false).expect("spell grant filter should parse");

    assert_eq!(filter.zone, Some(crate::zone::Zone::Hand));
    assert_eq!(filter.cast_by, Some(crate::target::PlayerFilter::You));
    assert_eq!(filter.owner, None);
    assert_eq!(filter.card_types, vec![CardType::Enchantment]);
}

#[test]
fn rewrite_lexed_value_and_permission_helpers_match_existing_semantics() {
    let count_tokens = lex_line("equal to the number of creatures", 0)
        .expect("rewrite lexer should classify count-value clause");
    let permission_tokens = lex_line("You may cast it this turn", 0)
        .expect("rewrite lexer should classify permission clause");

    assert!(matches!(
        super::value_helpers::parse_equal_to_number_of_filter_value_lexed(&count_tokens),
        Some(crate::effect::Value::Count(filter)) if filter.card_types == vec![CardType::Creature]
    ));
    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&permission_tokens),
        Ok(Some(super::PermissionClauseSpec::Tagged {
            player: crate::cards::builders::PlayerAst::You,
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: super::PermissionLifetime::ThisTurn,
        }))
    ));
}

#[test]
fn rewrite_grammar_add_mana_equal_amount_value_entrypoint_matches_parser_root_output() {
    let tokens = lex_line("equal to its toughness plus 2", 0)
        .expect("rewrite lexer should classify equal-amount value text");

    let parsed = super::parse_add_mana_equal_amount_value(&tokens);
    let grammar_parsed = super::grammar::values::parse_add_mana_equal_amount_value_lexed(&tokens);

    assert_eq!(grammar_parsed, parsed);
    assert_eq!(
        grammar_parsed,
        Some(crate::effect::Value::Add(
            Box::new(crate::effect::Value::ToughnessOf(Box::new(
                crate::target::ChooseSpec::Source,
            ))),
            Box::new(crate::effect::Value::Fixed(2)),
        ))
    );
}

#[test]
fn rewrite_grammar_object_filter_entrypoint_matches_parser_root_lexed_output() {
    let text = "creature card with mana value equal to 3";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comparison filter");

    let grammar =
        super::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(&lexed, false)
            .expect("grammar-owned object filter entrypoint should parse");
    let parser_root = super::parse_object_filter_lexed(&lexed, false)
        .expect("parser-root object filter entrypoint should parse");

    assert_eq!(format!("{grammar:?}"), format!("{parser_root:?}"));
}

#[test]
fn rewrite_parser_root_nonlexed_object_filter_entrypoint_matches_grammar_lexed_output() {
    let tokens = lex_line("artifact card in your graveyard", 0)
        .expect("rewrite lexer should classify non-lexed object filter text");

    let parser_root = super::parse_object_filter(&tokens, false)
        .expect("parser-root non-lexed object filter entrypoint should parse");
    let grammar_lexed =
        super::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
            .expect("grammar-owned lexed object filter entrypoint should parse");

    assert_eq!(format!("{parser_root:?}"), format!("{grammar_lexed:?}"));
}

#[test]
fn rewrite_grammar_spell_filter_entrypoint_matches_parser_root_output() {
    let text = "creature spells with power or toughness 2 or less";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comparison spell filter");

    let grammar = super::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed(&lexed);
    let parser_root = super::parse_spell_filter_lexed(&lexed);

    assert_eq!(format!("{grammar:?}"), format!("{parser_root:?}"));
}

#[test]
fn rewrite_parser_root_nonlexed_spell_filter_entrypoint_matches_lexed_output() {
    let tokens = lex_line("face-down noncreature spells", 0)
        .expect("rewrite lexer should classify non-lexed spell filter text");

    let parser_root = super::parse_spell_filter(&tokens);
    let lexed = super::parse_spell_filter_lexed(&tokens);

    assert_eq!(format!("{parser_root:?}"), format!("{lexed:?}"));
}

#[test]
fn rewrite_lexed_cant_sentence_supports_next_turn_silence() {
    let text = "Each opponent can't cast instant or sorcery spells during that player's next turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify next-turn silence");

    let parsed =
        parse_cant_effect_sentence_lexed(&lexed).expect("lexed next-turn silence should parse");
    let sentence =
        super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sentence parser");

    assert!(
        parsed.is_some(),
        "expected next-turn silence helper to match"
    );
    assert!(
        !sentence.is_empty(),
        "expected sentence parser to produce next-turn silence effects"
    );
}

#[test]
fn rewrite_effect_sentence_routes_cant_family_through_grammar_entrypoint() {
    let text = "Each opponent can't cast instant or sorcery spells during that player's next turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify next-turn silence");

    let grammar =
        super::grammar::effects::parse_cant_effect_sentence_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar-owned cant sentence entrypoint should parse");
    let sentence = parse_effect_sentence_lexed(&lexed).expect("effect sentence parser");
    let grammar = grammar.unwrap_or_default();

    assert_eq!(format!("{sentence:?}"), format!("{grammar:?}"));
}

#[test]
fn rewrite_lexed_cant_sentence_preserves_hyphenated_spell_filter_for_next_turn_silence() {
    let text = "Each opponent can't cast non-Creature spells during that player's next turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify hyphenated next-turn silence");
    let parsed =
        parse_cant_effect_sentence_lexed(&lexed).expect("lexed next-turn silence should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("excluded_card_types: [Creature]"),
        "expected non-Creature spell filter to survive parsing, got {debug}"
    );
}

#[test]
fn rewrite_parse_target_phrase_preserves_hyphenated_filter_before_random_suffix() {
    let text = "target non-Vampire creature chosen at random";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify hyphenated random target phrase");
    let target =
        super::util::parse_target_phrase(&tokens).expect("hyphenated random target should parse");
    let debug = format!("{target:?}");

    assert!(
        debug.contains("random: true"),
        "expected target to remain random, got {debug}"
    );
    assert!(
        debug.contains("card_types: [Creature]"),
        "expected creature filter in parsed target, got {debug}"
    );
    assert!(
        debug.contains("excluded_subtypes: [Vampire]"),
        "expected excluded Vampire subtype in parsed target, got {debug}"
    );
}

#[test]
fn rewrite_parse_target_phrase_supports_enchanted_player() {
    let tokens = lex_line("enchanted player", 0)
        .expect("rewrite lexer should classify enchanted player target phrase");
    let target =
        super::util::parse_target_phrase(&tokens).expect("enchanted player target should parse");

    assert!(matches!(
        target,
        crate::cards::builders::TargetAst::Player(
            crate::target::PlayerFilter::TaggedPlayer(tag),
            _
        ) if tag.as_str() == "enchanted"
    ));
}

#[test]
fn semantic_document_supports_next_turn_silence() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Sphinx's Decree")
        .card_types(vec![CardType::Sorcery]);

    let parsed = parse_text_to_semantic_document(
        builder,
        "Each opponent can't cast instant or sorcery spells during that player's next turn."
            .to_string(),
        false,
    );

    parsed.expect("expected semantic document parse to succeed");
}

#[test]
fn rewrite_lexed_restriction_duration_handles_for_as_long_as_token_shapes() {
    let prefix = lex_line(
        "For as long as you control this, target creature can't attack.",
        0,
    )
    .expect("rewrite lexer should classify for-as-long-as prefix duration");
    let parsed = parse_restriction_duration_lexed(&prefix)
        .expect("prefix duration should parse")
        .expect("prefix duration should be present");
    assert_eq!(parsed.0, crate::effect::Until::YouStopControllingThis);
    assert_eq!(
        TokenWordView::new(&parsed.1).to_word_refs(),
        vec!["target", "creature", "cant", "attack"]
    );

    let suffix = lex_line(
        "Target creature can't attack for as long as this remains tapped.",
        0,
    )
    .expect("rewrite lexer should classify for-as-long-as suffix duration");
    let parsed = parse_restriction_duration_lexed(&suffix)
        .expect("suffix duration should parse")
        .expect("suffix duration should be present");
    assert_eq!(parsed.0, crate::effect::Until::ThisLeavesTheBattlefield);
    assert_eq!(
        TokenWordView::new(&parsed.1).to_word_refs(),
        vec!["target", "creature", "cant", "attack"]
    );
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("rewrite audit should read source directory");
    for entry in entries {
        let entry = entry.expect("rewrite audit should read directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn rewrite_runtime_sources_do_not_reintroduce_token_bridge_helpers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards/builders/parser");
    let removed_helper_names = [
        format!("{}_{}", "compat_tokens_from", "lexed"),
        format!("{}_{}", "lexed_tokens_from", "compat"),
    ];
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);

    let mut offenders = Vec::new();
    for path in files {
        if path.ends_with("tests.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("rewrite audit should read source file");
        let relative = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .expect("rewrite audit should relativize source path")
            .display()
            .to_string();

        if removed_helper_names
            .iter()
            .any(|helper_name| source.contains(helper_name))
        {
            offenders.push(relative);
        }
    }

    assert!(
        offenders.is_empty(),
        "token bridge helpers should stay removed: {}",
        offenders.join(", ")
    );
}

#[test]
fn rewrite_lexed_value_helpers_cover_offset_and_aggregate_counts() {
    let offset_tokens = lex_line("equal to the number of creatures plus 2", 0)
        .expect("rewrite lexer should classify offset count-value clause");
    let aggregate_tokens = lex_line("equal to the greatest power among creatures you control", 0)
        .expect("rewrite lexer should classify aggregate-value clause");
    let spells_cast_tokens = lex_line(
        "equal to the number of instant spells you cast this turn",
        0,
    )
    .expect("rewrite lexer should classify spells-cast count-value clause");

    assert!(matches!(
        super::value_helpers::parse_equal_to_number_of_filter_plus_or_minus_fixed_value_lexed(
            &offset_tokens
        ),
        Some(crate::effect::Value::Add(base, offset))
            if matches!(*base, crate::effect::Value::Count(_))
                && matches!(*offset, crate::effect::Value::Fixed(2))
    ));
    assert!(matches!(
        super::value_helpers::parse_equal_to_aggregate_filter_value_lexed(&aggregate_tokens),
        Some(crate::effect::Value::GreatestPower(filter))
            if filter.card_types == vec![CardType::Creature]
                && filter.controller == Some(crate::target::PlayerFilter::You)
    ));
    assert!(matches!(
        super::value_helpers::parse_equal_to_number_of_filter_value_lexed(&spells_cast_tokens),
        Some(crate::effect::Value::SpellsCastThisTurnMatching { player, filter, .. })
            if player == crate::target::PlayerFilter::You
                && filter.card_types == vec![CardType::Instant]
    ));
}

#[test]
fn rewrite_lexed_permission_helpers_cover_flash_and_free_cast_grants() {
    let flash_tokens = lex_line("You may cast creature spells as though they had flash", 0)
        .expect("rewrite lexer should classify flash permission clause");
    let free_cast_tokens = lex_line(
        "You may cast creature spells from your hand without paying their mana costs",
        0,
    )
    .expect("rewrite lexer should classify free-cast permission clause");

    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&flash_tokens),
        Ok(Some(super::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: super::PermissionLifetime::Static,
        })) if spec == crate::grant::GrantSpec::flash_to_spells_matching(
            crate::target::ObjectFilter {
                card_types: vec![CardType::Creature],
                ..crate::target::ObjectFilter::default()
            }
        )
    ));
    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&free_cast_tokens),
        Ok(Some(super::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: super::PermissionLifetime::Static,
        })) if !spec.filter.has_mana_cost
            && spec.filter.card_types == vec![CardType::Creature]
            && spec.zone == crate::zone::Zone::Hand
    ));
}

#[test]
fn rewrite_lexed_permission_helpers_route_subject_filters_through_grammar_entrypoint() {
    let tokens = lex_line("You may cast creature spells as though they had flash", 0)
        .expect("rewrite lexer should classify flash permission clause");

    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(super::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: super::PermissionLifetime::Static,
        })) if spec == crate::grant::GrantSpec::flash_to_spells_matching(
            crate::target::ObjectFilter {
                card_types: vec![CardType::Creature],
                ..crate::target::ObjectFilter::default()
            }
        )
    ));
}

#[test]
fn rewrite_lexed_permission_helpers_preserve_disjunctive_subject_filters_without_local_word_view() {
    let tokens = lex_line(
        "You may cast instant and sorcery spells as though they had flash",
        0,
    )
    .expect("rewrite lexer should classify disjunctive flash permission clause");

    let parsed = super::permission_helpers::parse_permission_clause_spec_lexed(&tokens)
        .expect("permission clause should parse")
        .expect("permission clause should build a grant spec");

    match parsed {
        super::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime,
        } => {
            assert_eq!(player, crate::cards::builders::PlayerAst::You);
            assert_eq!(lifetime, super::PermissionLifetime::Static);
            assert_eq!(spec.filter.any_of.len(), 2);
            assert!(
                spec.filter
                    .any_of
                    .iter()
                    .any(|filter| filter.card_types == vec![CardType::Instant])
            );
            assert!(
                spec.filter
                    .any_of
                    .iter()
                    .any(|filter| filter.card_types == vec![CardType::Sorcery])
            );
        }
        other => panic!("expected disjunctive flash grant, got {other:?}"),
    }
}

#[test]
fn rewrite_lexed_permission_helpers_route_free_cast_spell_filters_through_grammar_entrypoint() {
    let tokens = lex_line(
        "You may cast creature spells from your hand without paying their mana costs",
        0,
    )
    .expect("rewrite lexer should classify free-cast permission clause");

    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(super::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: super::PermissionLifetime::Static,
        })) if !spec.filter.has_mana_cost
            && spec.filter.card_types == vec![CardType::Creature]
            && spec.zone == crate::zone::Zone::Hand
    ));
}

#[test]
fn rewrite_lexed_permission_helpers_cover_until_next_turn_tagged_play() {
    let tokens = lex_line("Until the end of your next turn, you may play that card", 0)
        .expect("rewrite lexer should classify until-next-turn permission clause");

    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(super::PermissionClauseSpec::Tagged {
            player: crate::cards::builders::PlayerAst::You,
            allow_land: true,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: super::PermissionLifetime::UntilYourNextTurn,
        }))
    ));
}

#[test]
fn rewrite_token_primitives_cover_count_range_prefixes() {
    let up_to = lex_line("up to three target creatures", 0)
        .expect("rewrite lexer should classify up-to count range");
    let one = lex_line("one target creature", 0)
        .expect("rewrite lexer should classify exact-one count range");
    let one_or_more = lex_line("one or more creatures", 0)
        .expect("rewrite lexer should classify one-or-more count range");
    let one_or_both = lex_line("one or both modes", 0)
        .expect("rewrite lexer should classify one-or-both count range");

    let up_to_range = super::token_primitives::parse_count_range_prefix(&up_to)
        .expect("up-to prefix should parse");
    let one_range =
        super::token_primitives::parse_count_range_prefix(&one).expect("one prefix should parse");
    let one_or_more_range = super::token_primitives::parse_count_range_prefix(&one_or_more)
        .expect("one-or-more prefix should parse");
    let one_or_both_range = super::token_primitives::parse_count_range_prefix(&one_or_both)
        .expect("one-or-both prefix should parse");

    assert_eq!(
        up_to_range.0,
        (
            Some(crate::effect::Value::Fixed(0)),
            Some(crate::effect::Value::Fixed(3))
        )
    );
    assert_eq!(
        TokenWordView::new(up_to_range.1).to_word_refs(),
        vec!["target", "creatures"]
    );
    assert_eq!(
        one_range.0,
        (
            Some(crate::effect::Value::Fixed(1)),
            Some(crate::effect::Value::Fixed(1))
        )
    );
    assert_eq!(
        TokenWordView::new(one_range.1).to_word_refs(),
        vec!["target", "creature"]
    );
    assert_eq!(
        one_or_more_range.0,
        (Some(crate::effect::Value::Fixed(1)), None)
    );
    assert_eq!(
        TokenWordView::new(one_or_more_range.1).to_word_refs(),
        vec!["creatures"]
    );
    assert_eq!(
        one_or_both_range.0,
        (
            Some(crate::effect::Value::Fixed(1)),
            Some(crate::effect::Value::Fixed(2))
        )
    );
    assert_eq!(
        TokenWordView::new(one_or_both_range.1).to_word_refs(),
        vec!["modes"]
    );
}

#[test]
fn rewrite_token_primitives_cover_turn_duration_prefix_and_suffix_phrases() {
    let prefixed = lex_line("Until the end of your next turn, you may play that card", 0)
        .expect("rewrite lexer should classify prefixed duration phrase");
    let suffixed = lex_line("Target creature can't attack this turn", 0)
        .expect("rewrite lexer should classify suffixed duration phrase");

    let (prefix_duration, prefix_remainder) =
        super::token_primitives::parse_turn_duration_prefix(&prefixed)
            .expect("prefixed duration should parse");
    let (suffix_remainder, suffix_duration) =
        super::token_primitives::parse_turn_duration_suffix(&suffixed)
            .expect("suffixed duration should parse");

    assert_eq!(
        prefix_duration,
        super::token_primitives::TurnDurationPhrase::UntilYourNextTurn
    );
    assert_eq!(
        TokenWordView::new(prefix_remainder).to_word_refs(),
        vec!["you", "may", "play", "that", "card"]
    );
    assert_eq!(
        suffix_duration,
        super::token_primitives::TurnDurationPhrase::ThisTurn
    );
    assert_eq!(
        TokenWordView::new(suffix_remainder).to_word_refs(),
        vec!["target", "creature", "cant", "attack"]
    );
}

#[test]
fn rewrite_token_primitives_split_comma_then_with_bounded_parser() {
    let tokens = lex_line("Draw a card, then discard a card.", 0)
        .expect("rewrite lexer should classify comma-then sentence");

    let (head, tail) = super::token_primitives::split_lexed_once_on_comma_then(&tokens)
        .expect("comma-then splitter should find boundary");

    assert_eq!(token_word_refs(head), vec!["Draw", "a", "card"]);
    assert_eq!(token_word_refs(tail), vec!["discard", "a", "card"]);
}

#[test]
fn rewrite_token_primitives_cover_simple_restriction_duration_boundaries() {
    let prefixed = lex_line("Until end of combat, target creature gains menace", 0)
        .expect("rewrite lexer should classify combat duration prefix");
    let suffixed = lex_line(
        "Target creature can't attack during its controller's next untap step",
        0,
    )
    .expect("rewrite lexer should classify untap-step duration suffix");
    let forever = lex_line("That player can't gain life for the rest of the game", 0)
        .expect("rewrite lexer should classify forever duration suffix");

    let (prefix_duration, prefix_remainder) =
        super::token_primitives::parse_simple_restriction_duration_prefix(&prefixed)
            .expect("combat duration prefix should parse");
    let (suffix_remainder, suffix_duration) =
        super::token_primitives::parse_simple_restriction_duration_suffix(&suffixed)
            .expect("untap-step duration suffix should parse");
    let (forever_remainder, forever_duration) =
        super::token_primitives::parse_simple_restriction_duration_suffix(&forever)
            .expect("forever duration suffix should parse");

    assert_eq!(prefix_duration, crate::effect::Until::EndOfCombat);
    assert_eq!(
        TokenWordView::new(prefix_remainder).to_word_refs(),
        vec!["target", "creature", "gains", "menace"]
    );
    assert_eq!(
        suffix_duration,
        crate::effect::Until::ControllersNextUntapStep
    );
    assert_eq!(
        TokenWordView::new(suffix_remainder).to_word_refs(),
        vec!["target", "creature", "cant", "attack"]
    );
    assert_eq!(forever_duration, crate::effect::Until::Forever);
    assert_eq!(
        TokenWordView::new(forever_remainder).to_word_refs(),
        vec!["that", "player", "cant", "gain", "life"]
    );
}

#[test]
fn rewrite_token_primitives_cover_bare_value_comparison_phrases() {
    let equal = lex_line("equal to 3", 0).expect("rewrite lexer should classify bare equality");
    let not_equal =
        lex_line("not equal to 3", 0).expect("rewrite lexer should classify bare inequality");
    let less_than =
        lex_line("less than 3", 0).expect("rewrite lexer should classify bare less-than");
    let greater_equal = lex_line("greater than or equal to 3", 0)
        .expect("rewrite lexer should classify bare greater-or-equal");

    let (equal_op, equal_remainder) =
        super::token_primitives::parse_value_comparison_tokens(&equal)
            .expect("bare equality should parse");
    let (not_equal_op, not_equal_remainder) =
        super::token_primitives::parse_value_comparison_tokens(&not_equal)
            .expect("bare inequality should parse");
    let (less_than_op, less_than_remainder) =
        super::token_primitives::parse_value_comparison_tokens(&less_than)
            .expect("bare less-than should parse");
    let (greater_equal_op, greater_equal_remainder) =
        super::token_primitives::parse_value_comparison_tokens(&greater_equal)
            .expect("bare greater-or-equal should parse");

    assert_eq!(equal_op, crate::effect::ValueComparisonOperator::Equal);
    assert_eq!(
        TokenWordView::new(equal_remainder).to_word_refs(),
        vec!["3"]
    );
    assert_eq!(
        not_equal_op,
        crate::effect::ValueComparisonOperator::NotEqual
    );
    assert_eq!(
        TokenWordView::new(not_equal_remainder).to_word_refs(),
        vec!["3"]
    );
    assert_eq!(
        less_than_op,
        crate::effect::ValueComparisonOperator::LessThan
    );
    assert_eq!(
        TokenWordView::new(less_than_remainder).to_word_refs(),
        vec!["3"]
    );
    assert_eq!(
        greater_equal_op,
        crate::effect::ValueComparisonOperator::GreaterThanOrEqual
    );
    assert_eq!(
        TokenWordView::new(greater_equal_remainder).to_word_refs(),
        vec!["3"]
    );
}

#[test]
fn rewrite_values_count_range_prefix_parses_modal_ranges_directly() {
    let up_to_two = lex_line("up to two", 0).expect("rewrite lexer should classify count range");
    let one = lex_line("one target", 0).expect("rewrite lexer should classify choose range");
    let one_or_both =
        lex_line("one or both targets", 0).expect("rewrite lexer should classify choose range");

    let (up_to_range, up_to_remainder) =
        super::grammar::values::parse_count_range_prefix(&up_to_two)
            .expect("direct values count-range parser should accept up-to phrase");
    let (one_range, one_remainder) = super::grammar::values::parse_count_range_prefix(&one)
        .expect("direct values count-range parser should accept bare one");
    let (one_or_both_range, one_or_both_remainder) =
        super::grammar::values::parse_count_range_prefix(&one_or_both)
            .expect("direct values count-range parser should accept one-or-both phrase");

    assert_eq!(
        up_to_range,
        (
            Some(crate::effect::Value::Fixed(0)),
            Some(crate::effect::Value::Fixed(2))
        )
    );
    assert!(up_to_remainder.is_empty());
    assert_eq!(
        one_range,
        (
            Some(crate::effect::Value::Fixed(1)),
            Some(crate::effect::Value::Fixed(1))
        )
    );
    assert_eq!(
        TokenWordView::new(one_remainder).to_word_refs(),
        vec!["target"]
    );
    assert_eq!(
        one_or_both_range,
        (
            Some(crate::effect::Value::Fixed(1)),
            Some(crate::effect::Value::Fixed(2))
        )
    );
    assert_eq!(
        TokenWordView::new(one_or_both_remainder).to_word_refs(),
        vec!["targets"]
    );
}

#[test]
fn rewrite_values_comparison_parser_handles_suffix_forms_directly() {
    let suffix = lex_line("3 or less", 0).expect("rewrite lexer should classify suffix comparison");
    let prefixed_suffix = lex_line("is 4 or more", 0)
        .expect("rewrite lexer should classify prefixed suffix comparison");

    let (suffix_op, suffix_operand) =
        super::grammar::values::parse_value_comparison_tokens(&suffix)
            .expect("direct values comparison parser should accept suffix form");
    let (prefixed_op, prefixed_operand) =
        super::grammar::values::parse_value_comparison_tokens(&prefixed_suffix)
            .expect("direct values comparison parser should accept prefixed suffix form");

    assert_eq!(
        suffix_op,
        crate::effect::ValueComparisonOperator::LessThanOrEqual
    );
    assert_eq!(TokenWordView::new(suffix_operand).to_word_refs(), vec!["3"]);
    assert_eq!(
        prefixed_op,
        crate::effect::ValueComparisonOperator::GreaterThanOrEqual
    );
    assert_eq!(
        TokenWordView::new(prefixed_operand).to_word_refs(),
        vec!["4"]
    );
}

#[test]
fn rewrite_lexed_permission_helpers_cover_or_less_conditional_free_casts() {
    let tokens = lex_line(
        "Cast that card without paying its mana cost if its mana value is 3 or less",
        0,
    )
    .expect("rewrite lexer should classify conditional free-cast permission");

    let parsed = super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("conditional free-cast clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("LessThanOrEqual"), "{debug}");
    assert!(debug.contains("Fixed(3)"), "{debug}");
}

#[test]
fn rewrite_lexed_permission_helpers_preserve_any_color_cast_suffix() {
    let tokens = lex_line(
        "You may play that card this turn and mana of any type can be spent to cast it",
        0,
    )
    .expect("rewrite lexer should classify tagged permission with mana-spend suffix");

    let parsed = super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("tagged permission clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("GrantPlayTaggedUntilEndOfTurn"), "{debug}");
    assert!(debug.contains("allow_any_color_for_cast: true"), "{debug}");
}

#[test]
fn rewrite_lexed_keyword_line_and_static_cost_probe_work_natively() {
    let flashback_tokens = lex_line("Flashback {2}{R}", 0)
        .expect("rewrite lexer should classify flashback keyword line");
    let cost_probe_tokens = lex_line("If it is night, this spell costs {2} less to cast.", 0)
        .expect("rewrite lexer should classify this-spell cost probe");

    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&flashback_tokens),
        Some(actions) if matches!(
            actions.as_slice(),
            [crate::cards::builders::KeywordAction::MarkerText(text)]
                if text == "Flashback {2}{R}"
        )
    ));
    let split = super::grammar::abilities::split_if_this_spell_costs_line_lexed(&cost_probe_tokens)
        .expect("grammar-owned this-spell cost splitter should match");
    assert_eq!(
        crate::cards::builders::parser::token_word_refs(split.condition_tokens),
        vec!["it", "is", "night"],
    );
    assert_eq!(
        crate::cards::builders::parser::token_word_refs(split.tail_tokens),
        vec!["this", "spell", "costs", "less", "to", "cast"],
    );
    assert!(matches!(
        super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed(
            &cost_probe_tokens
        ),
        Ok(Some(ability))
            if ability.id() == crate::static_abilities::StaticAbilityId::ThisSpellCostReduction
    ));
}

#[test]
fn rewrite_lower_routes_next_spell_cost_reduction_filters_through_grammar_entrypoint() {
    let text = "{T}: The next noncreature spell you cast this turn costs {2} less to cast.";
    let builder = CardDefinitionBuilder::new(CardId::new(), "Cost Reducer")
        .card_types(vec![CardType::Artifact]);

    let (parsed, _) = parse_text_to_semantic_document(builder, text.to_string(), false).expect(
        "next-spell cost reduction should lower through the grammar-owned spell filter entrypoint",
    );
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ReduceNextSpellCostThisTurn"), "{debug}");
    assert!(debug.contains("excluded_card_types: [Creature]"), "{debug}");
}

#[test]
fn rewrite_anthem_grant_static_parses_flashback_tail_without_word_view() {
    let tokens = lex_line(
        "During your turn, each instant and sorcery card in your graveyard has flashback. Its flashback cost is equal to its mana cost.",
        0,
    )
    .expect("rewrite lexer should classify granted flashback static line");

    let parsed = super::keyword_static::parse_granted_keyword_static_line(&tokens)
        .expect("granted flashback static line should parse")
        .expect("granted flashback static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("FlashbackFromCardManaCost"), "{debug}");
    assert!(debug.contains("owner: Some(You)"), "{debug}");
    assert!(debug.contains("card_types: [Instant]"), "{debug}");
    assert!(debug.contains("card_types: [Sorcery]"), "{debug}");
}

#[test]
fn rewrite_anthem_grant_static_parses_escape_tail_without_word_view() {
    let tokens = lex_line(
        "Each nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify granted escape static line");

    let parsed = super::keyword_static::parse_granted_keyword_static_line(&tokens)
        .expect("granted escape static line should parse")
        .expect("granted escape static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("EscapeFromCardManaCost { exile_count: 3 }"),
        "{debug}"
    );
    assert!(debug.contains("zone: Graveyard"), "{debug}");
    assert!(debug.contains("excluded_card_types: [Land]"), "{debug}");
}

#[test]
fn rewrite_anthem_static_condition_normalizes_apostrophe_shapes() {
    let tokens = lex_line("It's enchanted", 0)
        .expect("rewrite lexer should classify static-condition clause");

    let parsed = super::keyword_static::parse_static_condition_clause(&tokens)
        .expect("static-condition clause should parse");

    assert!(matches!(parsed, crate::ConditionExpr::SourceIsEnchanted));
}

#[test]
fn rewrite_verb_handlers_parse_look_normalizes_target_player_apostrophe_shapes() {
    let tokens = lex_line("Look at target player's hand.", 0)
        .expect("rewrite lexer should classify target player's hand look clause");

    let parsed =
        super::parse_effect_clause_lexed(&tokens).expect("look-at-hand clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtHand"), "{debug}");
    assert!(debug.contains("Player(Target(Any)"), "{debug}");
}

#[test]
fn rewrite_verb_handlers_parse_look_normalizes_owner_apostrophe_shapes() {
    let tokens = lex_line("Look at the top card of its owner's library.", 0)
        .expect("rewrite lexer should classify owner-library look clause");

    let parsed =
        super::parse_effect_clause_lexed(&tokens).expect("owner-library look clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("player: ItsOwner"), "{debug}");
    assert!(debug.contains("count: Fixed(1)"), "{debug}");
}

#[test]
fn rewrite_sentence_primitives_delayed_next_upkeep_unless_pays_normalizes_player_apostrophe_shapes()
{
    let tokens = lex_line(
        "Exile that creature at the beginning of that player's next upkeep unless they pay {2}.",
        0,
    )
    .expect("rewrite lexer should classify delayed next-upkeep unless sentence");

    let parsed = super::parse_sentence_delayed_next_step_unless_pays(&tokens)
        .expect("delayed next-upkeep unless sentence should parse")
        .expect("delayed next-upkeep unless sentence should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("DelayedUntilNextUpkeep"), "{debug}");
    assert!(debug.contains("player: That"), "{debug}");
    assert!(debug.contains("UnlessPays"), "{debug}");
}

#[test]
fn rewrite_sentence_primitives_unless_clause_normalizes_controller_apostrophe_shapes() {
    let tokens = lex_line("Draw a card unless that spell's controller pays {2}.", 0)
        .expect("rewrite lexer should classify unless-controller sentence");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("unless-controller sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("UnlessPays"), "{debug}");
    assert!(debug.contains("player: ItsController"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
fn rewrite_zone_handlers_sacrifice_choice_suffix_normalizes_pronoun_phrase() {
    let tokens = lex_line(
        "Target opponent sacrifices a creature of his or her choice.",
        0,
    )
    .expect("rewrite lexer should classify sacrifice-choice sentence");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("sacrifice-choice sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Sacrifice"), "{debug}");
    assert!(debug.contains("player: TargetOpponent"), "{debug}");
    assert!(debug.contains("card_types: [Creature]"), "{debug}");
}

#[test]
fn rewrite_keyword_static_routes_spell_cost_modifier_filters_through_grammar_entrypoint() {
    let tokens = lex_line("Artifact spells you cast cost {2} less to cast.", 0)
        .expect("rewrite lexer should classify spell cost modifier line");

    let parsed = super::keyword_static::parse_spells_cost_modifier_line(&tokens)
        .expect("spell cost modifier line should parse")
        .expect("spell cost modifier should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("CostReduction"), "{debug}");
    assert!(debug.contains("reduction: Fixed(2)"), "{debug}");
    assert!(debug.contains("cast_by: Some(You)"), "{debug}");
}

#[test]
fn rewrite_keyword_static_routes_trigger_duplication_source_filters_through_grammar_entrypoint() {
    let tokens = lex_line(
        "if a triggered ability of artifact creatures you control triggers, it triggers an additional time.",
        0,
    )
    .expect("rewrite lexer should classify trigger-duplication static line");

    let parsed = super::keyword_static::parse_trigger_duplication_line_ast(&tokens)
        .expect("trigger-duplication static line should parse")
        .expect("trigger-duplication static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("controller: Some(You)"), "{debug}");
    assert!(
        debug.contains("card_types: [Artifact, Creature]"),
        "{debug}"
    );
}

#[test]
fn rewrite_keyword_static_routes_trigger_duplication_event_filters_through_grammar_entrypoint() {
    let tokens = lex_line(
        "if turning artifact creatures you control face up causes an ability of a permanent you control to trigger, that ability triggers an additional time.",
        0,
    )
    .expect("rewrite lexer should classify trigger-duplication event line");

    let parsed = super::keyword_static::parse_trigger_duplication_line_ast(&tokens)
        .expect("trigger-duplication event line should parse")
        .expect("trigger-duplication event line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PermanentTurnedFaceUpTrigger"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
    assert!(
        debug.contains("card_types: [Artifact, Creature]"),
        "{debug}"
    );
}

#[test]
fn rewrite_grammar_trigger_duplication_as_long_as_prefix_splitter_matches_static_shape() {
    let tokens = lex_line(
        "As long as you control an artifact, if a triggered ability of artifact creatures you control triggers, it triggers an additional time.",
        0,
    )
    .expect("rewrite lexer should classify conditional trigger-duplication static line");

    let spec = super::grammar::abilities::split_as_long_as_condition_prefix_lexed(&tokens)
        .expect("grammar-owned as-long-as prefix splitter should match");
    assert_eq!(
        crate::cards::builders::parser::token_word_refs(spec.condition_tokens),
        vec!["you", "control", "an", "artifact"],
    );
    assert_eq!(
        crate::cards::builders::parser::token_word_refs(spec.remainder_tokens),
        vec![
            "if",
            "a",
            "triggered",
            "ability",
            "of",
            "artifact",
            "creatures",
            "you",
            "control",
            "triggers",
            "it",
            "triggers",
            "an",
            "additional",
            "time",
        ],
    );

    let parsed = super::keyword_static::parse_trigger_duplication_line_ast(&tokens)
        .expect("conditional trigger-duplication static line should parse")
        .expect("conditional trigger-duplication static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConditionalStaticAbility"), "{debug}");
    assert!(debug.contains("you control an artifact"), "{debug}");
}

#[test]
fn rewrite_lexed_next_spell_cascade_grants_parse_natively() {
    let single_tokens = lex_line(
        "The next noncreature spell you cast this turn has cascade.",
        0,
    )
    .expect("rewrite lexer should classify single next-spell cascade grant");
    let dual_tokens = lex_line(
        "The next instant spell and the next sorcery spell you cast this turn each have cascade.",
        0,
    )
    .expect("rewrite lexer should classify dual next-spell cascade grant");

    let single_effects = super::effect_sentences::parse_effect_sentence_lexed(&single_tokens)
        .expect("single next-spell cascade grant should parse");
    let dual_effects = super::effect_sentences::parse_effect_sentence_lexed(&dual_tokens)
        .expect("dual next-spell cascade grant should parse");

    assert!(matches!(
        single_effects.as_slice(),
        [crate::cards::builders::EffectAst::GrantNextSpellAbilityThisTurn { .. }]
    ));
    assert_eq!(
        dual_effects.len(),
        2,
        "expected one grant per next-spell lane"
    );
    assert!(dual_effects.iter().all(|effect| matches!(
        effect,
        crate::cards::builders::EffectAst::GrantNextSpellAbilityThisTurn { .. }
    )));
}

#[test]
fn rewrite_lexed_cycling_parser_ignores_static_grant_clause_prefixes() {
    let tokens = lex_line(
        "Each Sliver card in each player's hand has slivercycling {3}.",
        0,
    )
    .expect("rewrite lexer should classify granted cycling clause");

    assert!(
        super::parse_cycling_line_lexed(&tokens)
            .expect("cycling parser should inspect granted clause")
            .is_none()
    );
}

#[test]
fn rewrite_search_library_head_splitter_tracks_direct_may_and_rejects_early_may() {
    let direct_may = lex_line(
        "Target player may search their library for a card, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify direct-may search text");
    let split = super::grammar::effects::split_search_library_sentence_head_lexed(&direct_may)
        .expect("grammar-owned search head splitter should match direct may");

    assert_eq!(
        render_token_slice(split.subject_tokens),
        "Target player",
        "subject tokens should stop before direct may"
    );
    assert!(split.sentence_has_direct_may);
    assert_eq!(
        render_token_slice(split.search_tokens),
        "search their library for a card, then shuffle.",
        "search tokens should start at the search verb"
    );

    let leading_chain = lex_line(
        "Discard a card, then search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify leading-chain search text");
    let split = super::grammar::effects::split_search_library_sentence_head_lexed(&leading_chain)
        .expect("grammar-owned search head splitter should match plain search");
    assert!(!split.sentence_has_direct_may);
    assert_eq!(
        render_token_slice(split.subject_tokens),
        "Discard a card, then",
        "subject tokens should preserve the leading chain before search"
    );

    let early_may = lex_line(
        "You may draw a card, then search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify early-may search text");
    assert!(
        super::grammar::effects::split_search_library_sentence_head_lexed(&early_may).is_none(),
        "non-direct may before search should stay out of the search-family parser"
    );
}

#[test]
fn rewrite_search_library_clause_marker_scan_tracks_destination_boundaries() {
    let reveal_put_shuffle = lex_line(
        "Search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify reveal/put/shuffle search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&reveal_put_shuffle)
            .expect("search head splitter should match reveal/put/shuffle search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse reveal/put/shuffle search");

    assert_eq!(markers.for_idx, 3);
    assert!(markers.put_idx.is_some());
    assert!(markers.reveal_idx.is_some());
    assert!(markers.shuffle_idx.is_some());
    assert!(markers.has_explicit_destination);
    assert_eq!(markers.filter_boundary, markers.put_idx.unwrap());

    let exile_it = lex_line(
        "Search target opponent's library for a card and exile it face down.",
        0,
    )
    .expect("rewrite lexer should classify exile search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&exile_it)
            .expect("search head splitter should match exile search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse exile search");

    assert!(markers.exile_idx.is_some());
    assert!(markers.has_explicit_destination);
    assert_eq!(markers.filter_boundary, markers.exile_idx.unwrap());
}

#[test]
fn rewrite_search_library_filter_boundary_scan_stops_before_reveal_or_then() {
    let reveal_put_shuffle = lex_line(
        "Search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify reveal/put/shuffle search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&reveal_put_shuffle)
            .expect("search head splitter should match reveal/put/shuffle search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse reveal/put/shuffle search");
    let boundary = super::grammar::effects::find_search_library_filter_boundary_lexed(
        search_tokens,
        markers.for_idx,
        markers.filter_boundary,
    );

    assert_eq!(
        render_token_slice(&search_tokens[markers.for_idx + 1..boundary.filter_end]),
        "a creature card",
        "filter boundary should stop before the reveal clause"
    );

    let face_down_exile = lex_line(
        "Search target opponent's library for a card and exile it face down.",
        0,
    )
    .expect("rewrite lexer should classify exile search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&face_down_exile)
            .expect("search head splitter should match exile search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse exile search");
    let boundary = super::grammar::effects::find_search_library_filter_boundary_lexed(
        search_tokens,
        markers.for_idx,
        markers.filter_boundary,
    );

    assert_eq!(
        render_token_slice(&search_tokens[markers.for_idx + 1..boundary.filter_end]),
        "a card",
        "filter boundary should stop before the exile-it destination clause"
    );
}

#[test]
fn rewrite_search_library_discard_followup_scan_finds_clause_before_shuffle() {
    let discard_then_shuffle = lex_line(
        "Search your library for a basic land card, put it onto the battlefield tapped, then discard a card, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify discard-before-shuffle search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&discard_then_shuffle)
            .expect("search head splitter should match discard-before-shuffle search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse discard-before-shuffle search");
    let followup =
        super::grammar::effects::find_search_library_discard_before_shuffle_followup_lexed(
            search_tokens,
            markers.put_idx,
        )
        .expect("discard-before-shuffle helper should find the discard clause");

    assert_eq!(
        render_token_slice(&search_tokens[followup.discard_idx..followup.discard_end]),
        "discard a card",
        "discard followup should stop before the trailing shuffle clause"
    );
    assert!(followup.shuffle_idx > followup.discard_end);
}

#[test]
fn rewrite_search_library_trailing_life_followup_scan_returns_life_clause_only() {
    let trailing_life = lex_line(
        "Search your library for a card, put that card into your hand, then shuffle and you gain 3 life.",
        0,
    )
    .expect("rewrite lexer should classify trailing-life search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&trailing_life)
            .expect("search head splitter should match trailing-life search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse trailing-life search");
    let trailing_tokens =
        super::grammar::effects::find_search_library_trailing_life_followup_lexed(
            search_tokens,
            markers.put_idx.unwrap_or(markers.filter_boundary),
        )
        .expect("trailing-life helper should find the life-gain clause");

    assert_eq!(
        render_token_slice(trailing_tokens),
        "you gain 3 life.",
        "trailing-life helper should strip the leading and-marker"
    );
}

#[test]
fn rewrite_search_library_effect_routing_tracks_destination_and_flags() {
    let reveal_put_shuffle = lex_line(
        "Search your library for a creature card, reveal it, put it onto the battlefield tapped, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify routed battlefield search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&reveal_put_shuffle)
            .expect("search head splitter should match routed battlefield search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse routed battlefield search");
    let routing = super::grammar::effects::derive_search_library_effect_routing_lexed(
        &reveal_put_shuffle,
        search_tokens,
        markers,
        false,
    );

    assert_eq!(routing.destination, crate::zone::Zone::Battlefield);
    assert!(routing.reveal);
    assert!(routing.shuffle);
    assert!(routing.has_tapped_modifier);
    assert!(!routing.face_down_exile);
    assert!(!routing.split_battlefield_and_hand);

    let split_destination = lex_line(
        "Search your library for two basic land cards, put one onto the battlefield tapped and the other into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify split-destination search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&split_destination)
            .expect("search head splitter should match split-destination search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse split-destination search");
    let routing = super::grammar::effects::derive_search_library_effect_routing_lexed(
        &split_destination,
        search_tokens,
        markers,
        false,
    );

    assert!(routing.split_battlefield_and_hand);
    assert!(routing.shuffle);
    assert!(routing.has_tapped_modifier);

    let face_down_exile = lex_line(
        "Search target opponent's library for a card and exile it face down.",
        0,
    )
    .expect("rewrite lexer should classify face-down exile search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&face_down_exile)
            .expect("search head splitter should match face-down exile search")
            .search_tokens;
    let markers = super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned clause markers should parse face-down exile search");
    let routing = super::grammar::effects::derive_search_library_effect_routing_lexed(
        &face_down_exile,
        search_tokens,
        markers,
        false,
    );

    assert_eq!(routing.destination, crate::zone::Zone::Exile);
    assert!(routing.face_down_exile);
    assert!(!routing.shuffle);
}

#[test]
fn rewrite_search_library_subject_routing_tracks_zone_owner_prefixes() {
    let target_opponent_multi_zone = lex_line(
        "Search target opponent's graveyard, hand, and library for a card.",
        0,
    )
    .expect("rewrite lexer should classify target-opponent multi-zone search text");
    let search_tokens = super::grammar::effects::split_search_library_sentence_head_lexed(
        &target_opponent_multi_zone,
    )
    .expect("search head splitter should match target-opponent multi-zone search")
    .search_tokens;
    let routing = super::grammar::effects::derive_search_library_subject_routing_lexed(
        search_tokens,
        crate::cards::builders::PlayerAst::You,
    )
    .expect("subject routing helper should parse target-opponent multi-zone prefix");

    assert_eq!(routing.player, crate::cards::builders::PlayerAst::That);
    assert!(routing.search_player_target.is_some());
    assert_eq!(
        routing.search_zones_override,
        Some(vec![
            crate::zone::Zone::Graveyard,
            crate::zone::Zone::Hand,
            crate::zone::Zone::Library,
        ])
    );

    let its_controller = lex_line("Search its controller's library for a card.", 0)
        .expect("rewrite lexer should classify controller-owned search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&its_controller)
            .expect("search head splitter should match controller-owned search")
            .search_tokens;
    let routing = super::grammar::effects::derive_search_library_subject_routing_lexed(
        search_tokens,
        crate::cards::builders::PlayerAst::You,
    )
    .expect("subject routing helper should parse controller-owned prefix");

    assert_eq!(
        routing.player,
        crate::cards::builders::PlayerAst::ItsController
    );
    assert!(routing.search_player_target.is_none());
    assert!(routing.search_zones_override.is_none());

    let your_multi_zone = lex_line(
        "Search your graveyard, hand, and library for a creature card.",
        0,
    )
    .expect("rewrite lexer should classify your multi-zone search text");
    let search_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&your_multi_zone)
            .expect("search head splitter should match your multi-zone search")
            .search_tokens;
    let routing = super::grammar::effects::derive_search_library_subject_routing_lexed(
        search_tokens,
        crate::cards::builders::PlayerAst::You,
    )
    .expect("subject routing helper should parse your multi-zone prefix");

    assert_eq!(
        routing.search_zones_override,
        Some(vec![
            crate::zone::Zone::Graveyard,
            crate::zone::Zone::Hand,
            crate::zone::Zone::Library,
        ])
    );
}

#[test]
fn rewrite_search_library_count_prefix_parser_tracks_search_modes() {
    let any_number = lex_line("search your library for any number of creature cards", 0)
        .expect("rewrite lexer should classify any-number search text");
    let count_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&any_number)
            .expect("search head splitter should match any-number search")
            .search_tokens[4..8]
            .to_vec();
    let parsed = super::grammar::effects::parse_search_library_count_prefix_lexed(&count_tokens);

    assert_eq!(
        format!("{:?}", parsed.count),
        format!("{:?}", crate::cards::builders::ChoiceCount::any_number())
    );
    assert_eq!(
        parsed.search_mode,
        crate::effect::SearchSelectionMode::Optional
    );
    assert_eq!(parsed.count_used, 3);

    let up_to_x = lex_line("search your library for up to X cards", 0)
        .expect("rewrite lexer should classify up-to-x search text");
    let count_tokens = super::grammar::effects::split_search_library_sentence_head_lexed(&up_to_x)
        .expect("search head splitter should match up-to-x search")
        .search_tokens[4..7]
        .to_vec();
    let parsed = super::grammar::effects::parse_search_library_count_prefix_lexed(&count_tokens);

    assert_eq!(
        format!("{:?}", parsed.count),
        format!("{:?}", crate::cards::builders::ChoiceCount::dynamic_x())
    );
    assert_eq!(
        parsed.search_mode,
        crate::effect::SearchSelectionMode::Optional
    );
    assert_eq!(parsed.count_used, 3);

    let all_cards = lex_line("search your library for all cards", 0)
        .expect("rewrite lexer should classify all-cards search text");
    let count_tokens =
        super::grammar::effects::split_search_library_sentence_head_lexed(&all_cards)
            .expect("search head splitter should match all-cards search")
            .search_tokens[4..5]
            .to_vec();
    let parsed = super::grammar::effects::parse_search_library_count_prefix_lexed(&count_tokens);

    assert_eq!(
        format!("{:?}", parsed.count),
        format!("{:?}", crate::cards::builders::ChoiceCount::any_number())
    );
    assert_eq!(
        parsed.search_mode,
        crate::effect::SearchSelectionMode::AllMatching
    );
    assert_eq!(parsed.count_used, 1);
}

#[test]
fn rewrite_search_library_same_name_tail_parser_splits_reference_suffixes() {
    let chosen_name = lex_line("artifact card with the chosen name", 0)
        .expect("rewrite lexer should classify chosen-name filter text");
    let chosen_words = crate::cards::builders::parser::token_word_refs(&chosen_name);
    let parsed = super::grammar::effects::parse_search_library_same_name_reference_lexed(
        &chosen_name,
        chosen_name.clone(),
        &chosen_words,
    )
    .expect("same-name helper should parse chosen-name suffix");

    assert_eq!(
        render_token_slice(&parsed.filter_tokens),
        "artifact card",
        "chosen-name suffix should be removed from the base filter"
    );
    assert!(matches!(
        parsed.same_name_reference,
        Some(super::grammar::effects::SearchLibrarySameNameReference::Tagged(_))
    ));

    let target_reference = lex_line("creature card with the same name as target creature", 0)
        .expect("rewrite lexer should classify target same-name filter text");
    let target_words = crate::cards::builders::parser::token_word_refs(&target_reference);
    let parsed = super::grammar::effects::parse_search_library_same_name_reference_lexed(
        &target_reference,
        target_reference.clone(),
        &target_words,
    )
    .expect("same-name helper should parse target-reference suffix");

    assert_eq!(
        render_token_slice(&parsed.filter_tokens),
        "creature card",
        "target same-name suffix should be removed from the base filter"
    );
    assert!(matches!(
        parsed.same_name_reference,
        Some(super::grammar::effects::SearchLibrarySameNameReference::Target(_))
    ));
}

#[test]
fn rewrite_search_library_helper_parsers_track_mana_and_same_name_suffixes() {
    let mana_tokens = lex_line("artifact card with mana value 2 or 3", 0)
        .expect("rewrite lexer should classify mana-value helper input");
    let (base_filter, constraint) = super::extract_search_library_mana_constraint(&mana_tokens)
        .expect("mana-value helper should split base filter and clause");
    assert_eq!(token_word_refs(&base_filter), vec!["artifact", "card"]);
    assert!(matches!(
        constraint,
        super::SearchLibraryManaConstraint::OneOf(values)
            if values == vec![2, 3]
    ));

    let same_name_tokens = lex_line("creature card with the same name as that card", 0)
        .expect("rewrite lexer should classify same-name helper input");
    let (base_filter, reference_tokens) =
        super::split_search_same_name_reference_filter(&same_name_tokens)
            .expect("same-name helper should split reference suffix");
    assert_eq!(token_word_refs(&base_filter), vec!["creature", "card"]);
    assert_eq!(token_word_refs(&reference_tokens), vec!["that", "card"]);
}

#[test]
fn rewrite_search_library_object_filter_parser_handles_named_and_disjunction_shapes() {
    let named_filter = lex_line("artifact card named Sol Ring", 0)
        .expect("rewrite lexer should classify named search filter text");
    let named_words = crate::cards::builders::parser::token_word_refs(&named_filter);
    let parsed = super::grammar::effects::parse_search_library_object_filter_lexed(
        &named_filter,
        &named_words,
    )
    .expect("search-library object-filter helper should parse named filter");

    assert_eq!(parsed.name.as_deref(), Some("sol ring"));

    let counted_named = lex_line("exactly two artifact cards named Sol Ring", 0)
        .expect("rewrite lexer should classify counted named search filter text");
    let counted_named_words = crate::cards::builders::parser::token_word_refs(&counted_named);
    let parsed = super::grammar::effects::parse_search_library_object_filter_lexed(
        &counted_named,
        &counted_named_words,
    )
    .expect("search-library object-filter helper should strip count prefixes before named filters");

    assert_eq!(parsed.name.as_deref(), Some("sol ring"));
    assert_eq!(parsed.card_types, vec![CardType::Artifact]);

    let negated_named = lex_line("artifact card not named Sol Ring", 0)
        .expect("rewrite lexer should classify negated named search filter text");
    let negated_named_words = crate::cards::builders::parser::token_word_refs(&negated_named);
    let parsed = super::grammar::effects::parse_search_library_object_filter_lexed(
        &negated_named,
        &negated_named_words,
    )
    .expect("search-library object-filter helper should parse negated named filter");

    assert_eq!(parsed.excluded_name.as_deref(), Some("sol ring"));
    assert_eq!(parsed.card_types, vec![CardType::Artifact]);

    let disjunction = lex_line("artifact or enchantment card", 0)
        .expect("rewrite lexer should classify disjunction search filter text");
    let disjunction_words = crate::cards::builders::parser::token_word_refs(&disjunction);
    let parsed = super::grammar::effects::parse_search_library_object_filter_lexed(
        &disjunction,
        &disjunction_words,
    )
    .expect("search-library object-filter helper should parse disjunction filter");

    assert!(
        !parsed.any_of.is_empty(),
        "disjunction search filter should retain any_of branches"
    );
}

#[test]
fn rewrite_grammar_mana_group_slash_marker_probe_matches_keyword_shape() {
    let tokens = lex_line("Prototype {3}{U} 2/2", 0)
        .expect("rewrite lexer should classify slash-marker keyword line");
    assert!(
        super::grammar::abilities::is_mana_group_slash_marker_line_lexed(&tokens),
        "mana-group slash marker probe should recognize slash-bearing keyword line"
    );
}

#[test]
fn rewrite_search_library_leading_prelude_and_top_probe_helpers_cover_remaining_shapes() {
    let leading_chain = lex_line(
        "Discard a card, then search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify leading-chain search text");
    let head_split =
        super::grammar::effects::split_search_library_sentence_head_lexed(&leading_chain)
            .expect("search head splitter should match leading-chain search");
    let prelude = super::grammar::effects::parse_search_library_leading_effect_prelude_lexed(
        head_split.subject_tokens,
    )
    .expect("leading-prelude helper should parse the pre-search effect chain");

    assert!(prelude.subject_tokens.is_empty());
    assert!(
        !prelude.leading_effects.is_empty(),
        "leading-prelude helper should lift the leading effect chain"
    );

    let direct_subject = lex_line(
        "Target player may search their library for a card, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify direct-subject search text");
    let head_split =
        super::grammar::effects::split_search_library_sentence_head_lexed(&direct_subject)
            .expect("search head splitter should match direct-subject search");
    let prelude = super::grammar::effects::parse_search_library_leading_effect_prelude_lexed(
        head_split.subject_tokens,
    )
    .expect("leading-prelude helper should leave plain subjects alone");

    assert_eq!(render_token_slice(prelude.subject_tokens), "Target player");
    assert!(prelude.leading_effects.is_empty());

    let unsupported_top = lex_line("Search your library for the third card from the top.", 0)
        .expect("rewrite lexer should classify nth-from-top search text");
    let unsupported_words = crate::cards::builders::parser::token_word_refs(&unsupported_top);
    assert!(
        super::grammar::effects::search_library_has_unsupported_top_position_probe(
            &unsupported_words
        ),
        "nth-from-top search text should stay rejected by the grammar-owned top-position probe"
    );

    let allowed_top = lex_line(
        "Search your library for a card and put that card on top of library.",
        0,
    )
    .expect("rewrite lexer should classify on-top-of-library search text");
    let allowed_words = crate::cards::builders::parser::token_word_refs(&allowed_top);
    assert!(
        !super::grammar::effects::search_library_has_unsupported_top_position_probe(&allowed_words),
        "explicit on-top-of-library destination text should not trip the rejection probe"
    );
}

#[test]
fn rewrite_search_library_head_body_helpers_cover_wrap_and_search_verb_probes() {
    let wrap_subject =
        lex_line("each of them", 0).expect("rewrite lexer should classify wrap-subject text");
    assert!(
        super::grammar::effects::search_library_subject_wraps_each_target_player_lexed(
            &wrap_subject
        ),
        "`each of them` should trigger the wrap helper"
    );

    let plain_subject =
        lex_line("target player", 0).expect("rewrite lexer should classify plain subject text");
    assert!(
        !super::grammar::effects::search_library_subject_wraps_each_target_player_lexed(
            &plain_subject
        ),
        "plain subjects should not trigger the wrap helper"
    );

    let search_tokens = lex_line("search your library for a card", 0)
        .expect("rewrite lexer should classify search-verb text");
    assert!(
        super::grammar::effects::search_library_starts_with_search_verb_lexed(&search_tokens),
        "search tokens should satisfy the search-verb sanity helper"
    );

    let non_search_tokens =
        lex_line("draw a card", 0).expect("rewrite lexer should classify non-search text");
    assert!(
        !super::grammar::effects::search_library_starts_with_search_verb_lexed(&non_search_tokens),
        "non-search text should fail the search-verb sanity helper"
    );
}

#[test]
fn rewrite_cant_sentence_negation_helpers_cover_supported_and_rejected_guards() {
    let supported = lex_line("Target artifact doesn't untap", 0)
        .expect("rewrite lexer should classify supported cant clause");
    let lowered =
        super::grammar::effects::cant_sentence_clause_tokens_for_restriction_scan_lexed(&supported);
    assert_eq!(
        super::grammar::effects::find_cant_sentence_negation_span_lexed(&lowered),
        Some((2, 3))
    );
    assert_eq!(
        super::token_word_refs(&lowered),
        vec!["Target", "artifact", "doesn't", "untap"]
    );
    assert!(
        super::grammar::effects::cant_sentence_has_supported_negation_gate_lexed(&lowered),
        "plain cant clause should pass the negation gate"
    );

    let rejected = lex_line("Target artifact and target creature don't untap", 0)
        .expect("rewrite lexer should classify rejected cant clause");
    let lowered =
        super::grammar::effects::cant_sentence_clause_tokens_for_restriction_scan_lexed(&rejected);
    assert!(
        !super::grammar::effects::cant_sentence_has_supported_negation_gate_lexed(&lowered),
        "clauses with an `and` before the negation span should stay rejected"
    );

    let split_negation = lex_line("Target artifact can not attack", 0)
        .expect("rewrite lexer should classify split-negation cant clause");
    assert_eq!(
        super::grammar::effects::find_cant_sentence_negation_span_lexed(&split_negation),
        Some((2, 4))
    );
}

#[test]
fn rewrite_cant_sentence_next_turn_prefix_splitter_tracks_supported_suffixes() {
    let player_apostrophe = lex_line(
        "Each opponent can't cast instant or sorcery spells during that player's next turn.",
        0,
    )
    .expect("rewrite lexer should classify next-turn silence text");
    let prefix =
        super::grammar::effects::split_cant_sentence_next_turn_prefix_lexed(&player_apostrophe)
            .expect("next-turn splitter should match apostrophe suffix");

    assert_eq!(
        super::token_word_refs(&prefix),
        vec![
            "Each", "opponent", "can't", "cast", "instant", "or", "sorcery", "spells",
        ]
    );

    let split_apostrophe = lex_line(
        "Each opponent can't cast instant or sorcery spells during that player s next turn.",
        0,
    )
    .expect("rewrite lexer should classify split-apostrophe next-turn silence text");
    assert!(
        super::grammar::effects::split_cant_sentence_next_turn_prefix_lexed(&split_apostrophe)
            .is_some(),
        "next-turn splitter should also match split-apostrophe suffixes"
    );

    let untap_step = lex_line(
        "Target artifact doesn't untap during its controller's next untap step.",
        0,
    )
    .expect("rewrite lexer should classify untap-step restriction text");
    assert!(
        super::grammar::effects::split_cant_sentence_next_turn_prefix_lexed(&untap_step).is_none(),
        "non-next-turn restriction text should stay out of the next-turn prefix helper"
    );
}

#[test]
fn rewrite_cant_sentence_clause_preparation_helper_tracks_supported_and_rejected_shapes() {
    let untap_step = lex_line(
        "Target artifact doesn't untap during its controller's next untap step.",
        0,
    )
    .expect("rewrite lexer should classify supported untap-step restriction text");
    let prepared =
        super::grammar::effects::prepare_cant_sentence_restriction_clause_lexed(&untap_step)
            .expect("cant clause preparation helper should not error")
            .expect("cant clause preparation helper should keep supported untap-step text");

    assert_eq!(
        super::token_word_refs(&prepared.clause_tokens),
        vec!["Target", "artifact", "doesn't", "untap"]
    );

    let positive_clause = lex_line(
        "Target artifact untaps during its controller's next untap step.",
        0,
    )
    .expect("rewrite lexer should classify positive untap-step text");
    assert!(
        super::grammar::effects::prepare_cant_sentence_restriction_clause_lexed(&positive_clause)
            .expect("cant clause preparation helper should not error")
            .is_none(),
        "clauses without a negation span should stay out of the prepared cant-clause helper"
    );
}

#[test]
fn rewrite_cant_sentence_source_tapped_duration_probe_tracks_supported_shapes() {
    let supported = lex_line(
        "Target creature can't attack for as long as this artifact remains tapped.",
        0,
    )
    .expect("rewrite lexer should classify source-tapped duration text");
    assert!(
        super::grammar::effects::cant_sentence_has_source_remains_tapped_duration(&supported),
        "source-tapped duration helper should recognize supported for-as-long-as remains-tapped text"
    );

    let unsupported = lex_line("Target creature can't attack until end of turn.", 0)
        .expect("rewrite lexer should classify simple cant sentence");
    assert!(
        !super::grammar::effects::cant_sentence_has_source_remains_tapped_duration(&unsupported),
        "non-source-tapped cant text should stay out of the remains-tapped helper"
    );
}

#[test]
fn rewrite_lexed_cant_sentence_marks_source_tapped_duration_condition() {
    let text = "Target creature can't attack for as long as this artifact remains tapped.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify source-tapped cant text");

    let parsed = parse_cant_effect_sentence_lexed(&lexed)
        .expect("lexed source-tapped cant sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SourceIsTapped"), "{debug}");
}

#[test]
fn rewrite_effect_sentence_routes_search_library_family_through_grammar_entrypoint() {
    let text = "Search your library for a creature card with mana value 3 or less, reveal it, put it into your hand, then shuffle.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify search-library text");

    let grammar =
        super::grammar::effects::parse_search_library_sentence_with_grammar_entrypoint_lexed(
            &lexed,
        )
        .expect("grammar-owned search-library sentence should parse")
        .unwrap_or_default();
    let sentence = parse_effect_sentence_lexed(&lexed).expect("effect sentence parser");

    assert_eq!(format!("{sentence:?}"), format!("{grammar:?}"));
}

#[test]
fn rewrite_lexed_spell_filter_preserves_comparison_shapes() {
    for text in [
        "noncreature spells with mana value equal to 3",
        "creature spells with power or toughness 2 or less",
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify comparison spell filter");
        let filter = super::parse_spell_filter_lexed(&tokens);
        let debug = format!("{filter:?}");

        if text.contains("mana value equal to 3") {
            assert!(debug.contains("excluded_card_types: [Creature]"), "{debug}");
            assert!(debug.contains("mana_value: Some(Equal(3))"), "{debug}");
        } else {
            assert!(debug.contains("any_of"), "{debug}");
            assert!(debug.contains("LessThanOrEqual(2)"), "{debug}");
        }
    }
}

#[test]
fn rewrite_lexed_search_library_sentence_parses_shared_mana_value_constraint() {
    let text = "Search your library for a creature card with mana value 3 or less, reveal it, put it into your hand, then shuffle.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify search-library text");

    let parsed = super::parse_search_library_sentence_lexed(&lexed)
        .expect("lexed search-library sentence should parse")
        .expect("search-library sentence should produce effects");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LessThanOrEqual(3)"), "{debug}");
}

#[test]
fn rewrite_lexed_search_library_sentence_parses_disjunction_filter_via_grammar_separator_helper() {
    let text = "Search your library for an artifact, enchantment, or creature card, reveal it, put it into your hand, then shuffle.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify search-library disjunction");

    let parsed = super::parse_search_library_sentence_lexed(&lexed)
        .expect("lexed search-library sentence should parse")
        .expect("search-library sentence should produce effects");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("any_of"), "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("Enchantment"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
}

#[test]
fn rewrite_gain_ability_keyword_lists_route_through_grammar_separator_helpers() {
    let text = "Target creature gains flying and vigilance until end of turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify gain-ability keyword list");

    let parsed =
        super::parse_effect_sentence_lexed(&lexed).expect("gain-ability sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Flying"), "{debug}");
    assert!(debug.contains("Vigilance"), "{debug}");
}

#[test]
fn rewrite_gain_ability_choice_list_routes_or_split_through_grammar_separator_helper() {
    let tokens = lex_line("your choice of flying, vigilance, or trample", 0)
        .expect("rewrite lexer should classify gain-ability choice list");

    let parsed =
        super::parse_choice_of_abilities(&tokens).expect("choice-of-abilities helper should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Flying"), "{debug}");
    assert!(debug.contains("Vigilance"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
}

#[test]
fn rewrite_activation_line_routes_period_split_through_grammar_separator_helper() {
    let tokens = lex_line("{T}: Add {G}. Activate only during your turn.", 0)
        .expect("rewrite lexer should classify activated line with trailing restriction");

    let parsed = super::parse_activated_line(&tokens)
        .expect("activated line should parse")
        .expect("activated line should produce an ability");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("DuringYourTurn"), "{debug}");
}

#[test]
fn rewrite_activation_line_collects_any_player_restriction_from_token_view() {
    let tokens = lex_line("{T}: Add {C}. Any player may activate this ability.", 0)
        .expect("rewrite lexer should classify activated line with any-player restriction");

    let parsed = super::parse_activated_line(&tokens)
        .expect("activated line should parse")
        .expect("activated line should produce an ability");

    match &parsed.ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            let restrictions = activated
                .additional_restrictions
                .iter()
                .map(|restriction| restriction.to_ascii_lowercase())
                .collect::<Vec<_>>();
            assert!(
                restrictions
                    .iter()
                    .any(|restriction| restriction == "any player may activate this ability"),
                "{restrictions:?}"
            );
        }
        other => panic!("expected activated ability, got {other:?}"),
    }
}

#[test]
fn rewrite_activation_line_collects_sentence_modifiers_via_activated_sentence_module() {
    let tokens = lex_line(
        "{T}: Add {C}. The next noncreature spell you cast this turn costs {2} less to cast. Spend this mana only to cast artifact spells of the chosen type and that spell can't be countered. Any player may activate this ability. Activate only once each turn.",
        0,
    )
    .expect("rewrite lexer should classify activated line with sentence modifiers");

    let parsed = super::parse_activated_line(&tokens)
        .expect("activated line should parse")
        .expect("activated line should produce an ability");
    let debug = format!("{parsed:#?}");

    match &parsed.ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            assert_eq!(
                activated.timing,
                crate::ability::ActivationTiming::OncePerTurn
            );
            assert!(matches!(
                activated.mana_usage_restrictions.as_slice(),
                [crate::ability::ManaUsageRestriction::CastSpell {
                    card_types,
                    subtype_requirement: Some(
                        crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource
                    ),
                    restrict_to_matching_spell: true,
                    grant_uncounterable: true,
                    enters_with_counters,
                }] if card_types == &vec![CardType::Artifact]
                    && enters_with_counters.is_empty()
            ));
            assert!(
                activated.additional_restrictions.iter().any(|restriction| {
                    restriction.eq_ignore_ascii_case("any player may activate this ability")
                }),
                "{:?}",
                activated.additional_restrictions
            );
        }
        other => panic!("expected activated ability, got {other:?}"),
    }

    assert!(debug.contains("ReduceNextSpellCostThisTurn"), "{debug}");
    assert!(debug.contains("excluded_card_types"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
}

#[test]
fn rewrite_activation_line_parses_biophagus_style_conditional_mana_bonus() {
    let tokens = lex_line(
        "{T}: Add one mana of any color. If this mana is spent to cast a creature spell, that creature enters with an additional +1/+1 counter on it.",
        0,
    )
    .expect("rewrite lexer should classify Biophagus-style mana bonus");

    let parsed = super::parse_activated_line(&tokens)
        .expect("Biophagus-style line should parse")
        .expect("Biophagus-style line should produce an ability");

    match &parsed.ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(matches!(
                activated.mana_usage_restrictions.as_slice(),
                [crate::ability::ManaUsageRestriction::CastSpell {
                    card_types,
                    subtype_requirement: None,
                    restrict_to_matching_spell: false,
                    grant_uncounterable: false,
                    enters_with_counters,
                }] if card_types == &vec![CardType::Creature]
                    && enters_with_counters
                        == &vec![(crate::object::CounterType::PlusOnePlusOne, 1)]
            ));
        }
        other => panic!("expected activated ability, got {other:?}"),
    }
}

#[test]
fn rewrite_keyword_static_combined_pregame_choose_color_routes_period_split_through_grammar_helper()
{
    let tokens = lex_line(
        "choose a color before the game begins. this card is the chosen color.",
        0,
    )
    .expect("rewrite lexer should classify combined pregame choose-color line");

    let parsed = super::parse_combined_pregame_choose_color_line(&tokens)
        .expect("combined pregame choose-color line should parse")
        .expect("combined pregame choose-color line should produce abilities");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ChooseColor"), "{debug}");
    assert!(debug.contains("chosen color"), "{debug}");
}

#[test]
fn rewrite_lexed_keyword_line_parses_simple_native_keyword_lists() {
    let keyword_tokens = lex_line("Flying and vigilance", 0)
        .expect("rewrite lexer should classify simple keyword line");
    let numeric_tokens =
        lex_line("Ward 2", 0).expect("rewrite lexer should classify numeric keyword line");

    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&keyword_tokens),
        Some(actions)
            if actions
                == vec![
                    crate::cards::builders::KeywordAction::Flying,
                    crate::cards::builders::KeywordAction::Vigilance,
                ]
    ));
    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&numeric_tokens),
        Some(actions)
            if actions
                == vec![crate::cards::builders::KeywordAction::Ward(2)]
    ));
}

#[test]
fn rewrite_lexed_keyword_line_parses_protection_chains_without_duplicates() {
    let protection_tokens = lex_line("Protection from everything and from everything", 0)
        .expect("rewrite lexer should classify protection chain");

    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&protection_tokens),
        Some(actions)
            if actions
                == vec![crate::cards::builders::KeywordAction::ProtectionFromEverything]
    ));
}

#[test]
fn rewrite_lexed_keyword_line_parses_mixed_protection_chain_targets() {
    let protection_tokens = lex_line("Protection from the chosen player and from all colors", 0)
        .expect("rewrite lexer should classify mixed protection chain");

    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&protection_tokens),
        Some(actions)
            if actions
                == vec![
                    crate::cards::builders::KeywordAction::ProtectionFromChosenPlayer,
                    crate::cards::builders::KeywordAction::ProtectionFromAllColors,
                ]
    ));
}

#[test]
fn rewrite_lexed_keyword_line_routes_separator_lists_through_grammar_primitives() {
    let keyword_tokens = lex_line("Flying, vigilance; trample and haste", 0)
        .expect("rewrite lexer should classify mixed keyword separator line");

    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&keyword_tokens),
        Some(actions)
            if actions
                == vec![
                    crate::cards::builders::KeywordAction::Flying,
                    crate::cards::builders::KeywordAction::Vigilance,
                    crate::cards::builders::KeywordAction::Trample,
                    crate::cards::builders::KeywordAction::Haste,
                ]
    ));
}

#[test]
fn rewrite_lexed_triggered_and_static_entrypoints_work_natively() {
    let triggered_tokens = lex_line(
        "Whenever you cast an Aura, Equipment, or Vehicle spell, draw a card.",
        0,
    )
    .expect("rewrite lexer should classify triggered probe");
    let static_tokens = lex_line(
        "Activated abilities of artifacts and creatures can't be activated.",
        0,
    )
    .expect("rewrite lexer should classify static probe");

    assert!(matches!(
        super::clause_support::parse_triggered_line_lexed(&triggered_tokens),
        Ok(crate::cards::builders::LineAst::Triggered { .. })
    ));
    assert!(matches!(
        super::clause_support::parse_static_ability_ast_line_lexed(&static_tokens),
        Ok(Some(abilities)) if !abilities.is_empty()
    ));
    assert_eq!(
        format!(
            "{:?}",
            super::keyword_static::parse_static_ability_ast_line_lexed(&static_tokens)
                .expect("static entrypoint should parse")
        ),
        format!(
            "{:?}",
            super::clause_support::parse_static_ability_ast_line_lexed(&static_tokens)
                .expect("lexed static entrypoint should parse")
        )
    );
}

#[test]
fn rewrite_grammar_activated_abilities_cant_be_activated_splitter_matches_keyword_static_shape() {
    let tokens = lex_line(
        "Activated abilities of artifacts and creatures can't be activated unless they're mana abilities.",
        0,
    )
    .expect("rewrite lexer should classify activated-abilities restriction");

    let spec =
        super::grammar::abilities::parse_activated_abilities_cant_be_activated_spec_lexed(&tokens)
            .expect("grammar-owned activated-abilities restriction splitter should match");

    assert_eq!(
        crate::cards::builders::parser::token_word_refs(spec.subject_tokens),
        vec!["artifacts", "and", "creatures"],
    );
    assert!(
        spec.non_mana_only,
        "splitter should preserve the unless-theyre-mana-abilities flag"
    );

    let parsed =
        super::keyword_static::parse_activated_abilities_cant_be_activated_line_lexed(&tokens)
            .expect("activated-abilities restriction should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::RuleRestriction
    ));
}

#[test]
fn rewrite_grammar_trigger_suppression_splitter_matches_keyword_static_shape() {
    let tokens = lex_line(
        "Creatures entering the battlefield don't cause abilities of artifacts to trigger.",
        0,
    )
    .expect("rewrite lexer should classify trigger-suppression line");

    let spec = super::grammar::abilities::parse_trigger_suppression_spec_lexed(&tokens)
        .expect("grammar-owned trigger-suppression splitter should match");

    assert_eq!(
        crate::cards::builders::parser::token_word_refs(spec.cause_tokens),
        vec!["Creatures", "entering", "the", "battlefield"],
    );
    assert_eq!(
        spec.source_filter_tokens
            .map(crate::cards::builders::parser::token_word_refs),
        Some(vec!["artifacts"]),
    );

    let parsed = super::keyword_static::parse_trigger_suppression_line_ast(&tokens)
        .expect("trigger-suppression line should parse");
    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::Static(ability))
            if ability.id()
                == crate::static_abilities::StaticAbilityId::SuppressMatchingTriggeredAbilities
    ));
}

#[test]
fn rewrite_keyword_static_marker_line_normalizes_doctors_companion_apostrophe() {
    let tokens = lex_line("Doctor's companion", 0)
        .expect("rewrite lexer should classify doctor's companion marker line");

    let ability = super::keyword_static::parse_static_text_marker_line(&tokens)
        .expect("doctor's companion marker line should parse");

    assert_eq!(
        ability.id(),
        crate::static_abilities::StaticAbilityId::DoctorsCompanion
    );
}

#[test]
fn rewrite_grammar_protection_and_ward_marker_probes_match_static_shapes() {
    let protection_tokens = lex_line("Protection from odd mana values.", 0)
        .expect("rewrite lexer should classify protection marker line");

    assert!(
        super::grammar::abilities::is_protection_mana_value_marker_line_lexed(&protection_tokens),
        "grammar-owned protection marker probe should match"
    );

    let protection = super::keyword_static::parse_static_text_marker_line(&protection_tokens)
        .expect("protection marker line should parse");
    let protection_debug = format!("{protection:?}");
    assert!(
        protection_debug.contains("Protection from odd mana values"),
        "{protection_debug}"
    );

    let ward_tokens =
        lex_line("Ward pay 3 life.", 0).expect("rewrite lexer should classify ward marker line");

    assert_eq!(
        super::grammar::abilities::parse_ward_pay_life_amount_lexed(&ward_tokens),
        Some(3)
    );

    let ward = super::keyword_static::parse_static_text_marker_line(&ward_tokens)
        .expect("ward marker line should parse");
    let debug = format!("{ward:?}");
    assert!(debug.contains("Ward—Pay 3 life"), "{debug}");
}

#[test]
fn rewrite_grammar_remaining_exact_marker_probes_match_static_shapes() {
    let odd_flash_tokens = lex_line("As long as this creature has odd power, it has flash.", 0)
        .expect("rewrite lexer should classify odd-power flash marker line");
    assert!(
        super::grammar::abilities::is_as_long_as_power_odd_or_even_flash_marker_line_lexed(
            &odd_flash_tokens
        ),
        "grammar-owned odd/even flash marker probe should match"
    );
    assert!(
        super::keyword_static::parse_static_text_marker_line(&odd_flash_tokens).is_some(),
        "odd/even flash marker line should parse"
    );

    let haste_tokens = lex_line(
        "This creature can attack as though it had haste unless it entered this turn.",
        0,
    )
    .expect("rewrite lexer should classify haste-unless-entered marker line");
    assert!(
        super::grammar::abilities::is_attack_as_haste_unless_entered_this_turn_marker_line_lexed(
            &haste_tokens
        ),
        "grammar-owned haste-unless-entered marker probe should match"
    );
    assert!(
        super::keyword_static::parse_static_text_marker_line(&haste_tokens).is_some(),
        "haste-unless-entered marker line should parse"
    );

    let sab_tokens = lex_line(
        "Sab-Sunen can't attack or block unless there are seven or more lands among cards in your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify Sab-Sunen marker line");
    assert!(
        super::grammar::abilities::is_sab_sunen_cant_attack_or_block_unless_line_lexed(&sab_tokens),
        "grammar-owned Sab-Sunen marker probe should match"
    );
    assert!(
        super::keyword_static::parse_static_text_marker_line(&sab_tokens).is_some(),
        "Sab-Sunen marker line should parse"
    );
}

#[test]
fn rewrite_keyword_static_doesnt_untap_line_normalizes_contraction() {
    let tokens = lex_line("This creature doesn't untap during your untap step", 0)
        .expect("rewrite lexer should classify doesn't untap line");

    assert!(matches!(
        super::grammar::abilities::parse_doesnt_untap_during_untap_step_spec_lexed(&tokens),
        Some(super::grammar::abilities::DoesntUntapDuringUntapStepSpec::Source { .. })
    ));

    let parsed = super::keyword_static::parse_doesnt_untap_during_untap_step_line(&tokens)
        .expect("doesn't untap line should parse");

    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::Static(ref ability))
            if ability.id() == crate::static_abilities::StaticAbilityId::DoesntUntap
    ));
}

#[test]
fn rewrite_grammar_doesnt_untap_line_matches_attached_subject_shape() {
    let tokens = lex_line(
        "Enchanted creature doesn't untap during its controller's untap step.",
        0,
    )
    .expect("rewrite lexer should classify attached doesnt-untap line");

    assert!(matches!(
        super::grammar::abilities::parse_doesnt_untap_during_untap_step_spec_lexed(&tokens),
        Some(super::grammar::abilities::DoesntUntapDuringUntapStepSpec::Attached { .. })
    ));

    let parsed = super::keyword_static::parse_doesnt_untap_during_untap_step_line(&tokens)
        .expect("attached doesnt-untap line should parse");

    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::AttachedStaticAbilityGrant { .. })
    ));
}

#[test]
fn rewrite_keyword_static_reveal_first_card_probe_uses_parser_text_words() {
    let tokens = lex_line(
        "You may reveal the first card you draw on each of your turns as you draw it.",
        0,
    )
    .expect("rewrite lexer should classify reveal-first-card static line");

    let spec =
        super::grammar::abilities::parse_reveal_first_card_you_draw_each_turn_spec_lexed(&tokens)
            .expect("grammar-owned reveal-first-card probe should match");
    assert!(
        spec.optional,
        "grammar probe should preserve optional prefix"
    );
    assert!(
        spec.your_turns_only,
        "grammar probe should preserve the on-each-of-your-turns variant"
    );

    let parsed = super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("reveal-first-card static line should parse")
        .expect("reveal-first-card static line should produce abilities");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::StaticAbilityAst::Static(ability)]
            if ability.id() == crate::static_abilities::StaticAbilityId::RevealFirstCardYouDrawEachTurn
    ));
}

#[test]
fn rewrite_keyword_static_craft_marker_uses_shared_marker_text_rendering() {
    let tokens = lex_line("Craft with artifact {3}{W}{W}", 0)
        .expect("rewrite lexer should classify craft marker line");

    let parsed = super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("craft marker line should parse")
        .expect("craft marker line should produce abilities");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Craft with artifact {3}{W}{W}"), "{debug}");
}

#[test]
fn rewrite_keyword_static_as_enters_choice_parsers_share_subject_tables() {
    let color_tokens = lex_line("as this aura enters, choose a color.", 0)
        .expect("rewrite lexer should classify choose-color static line");
    let player_tokens = lex_line("as this artifact enters, choose a player.", 0)
        .expect("rewrite lexer should classify choose-player static line");

    let color = super::keyword_static::parse_choose_color_as_enters_line(&color_tokens)
        .expect("choose-color static line should parse");
    let player = super::keyword_static::parse_choose_player_as_enters_line(&player_tokens)
        .expect("choose-player static line should parse");

    assert!(matches!(
        color,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::ChooseColorAsEnters
    ));
    assert!(matches!(
        player,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::ChoosePlayerAsEnters
    ));
}

#[test]
fn rewrite_grammar_exile_to_countered_exile_instead_of_graveyard_splitter_matches_static_shape() {
    let tokens = lex_line(
        "If a creature would be put into an opponent's graveyard from anywhere, exile it instead with a stun counter on it.",
        0,
    )
    .expect("rewrite lexer should classify exile-replacement static line");

    let spec =
        super::grammar::abilities::parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(
            &tokens,
        )
        .expect("grammar-owned exile-replacement splitter should match");
    assert_eq!(spec.player, crate::target::PlayerFilter::Opponent);
    assert_eq!(spec.counter_type, crate::object::CounterType::Stun);

    let parsed =
        super::keyword_static::parse_exile_to_countered_exile_instead_of_graveyard_line(&tokens)
            .expect("exile-replacement static line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
    ));
}

#[test]
fn rewrite_grammar_exile_to_countered_exile_splitter_accepts_instead_lead_word_order() {
    let tokens = lex_line(
        "If a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.",
        0,
    )
    .expect("rewrite lexer should classify exile-replacement static line");

    let spec =
        super::grammar::abilities::parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(
            &tokens,
        )
        .expect("grammar-owned exile-replacement splitter should match");
    assert_eq!(spec.player, crate::target::PlayerFilter::Opponent);
    assert_eq!(spec.counter_type, crate::object::CounterType::Void);
}

#[test]
fn rewrite_grammar_draw_replace_exile_top_face_down_probe_matches_static_shape() {
    let tokens = lex_line(
        "If you would draw a card, exile the top card of your library face down instead.",
        0,
    )
    .expect("rewrite lexer should classify draw-replacement static line");

    assert!(
        super::grammar::abilities::is_draw_replace_exile_top_face_down_line_lexed(&tokens),
        "grammar-owned draw-replacement probe should match"
    );

    let parsed = super::keyword_static::parse_draw_replace_exile_top_face_down_line(&tokens)
        .expect("draw-replacement static line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::DrawReplacementExileTopFaceDown
    ));
}

#[test]
fn rewrite_grammar_replacement_static_probes_match_keyword_static_shapes() {
    let library_tokens = lex_line(
        "If an effect causes you to discard a card, you may discard it to the top of your library instead of into your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify Library of Leng replacement line");
    assert!(
        super::grammar::abilities::is_library_of_leng_discard_replacement_line_lexed(
            &library_tokens
        ),
        "grammar-owned Library of Leng probe should match"
    );
    assert!(matches!(
        super::keyword_static::parse_library_of_leng_discard_replacement_line(&library_tokens)
            .expect("Library of Leng replacement line should parse"),
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::LibraryOfLengDiscardReplacement
    ));

    let shuffle_tokens = lex_line(
        "If Darksteel Colossus would be put into a graveyard from anywhere, reveal Darksteel Colossus and shuffle it into its owner's library instead.",
        0,
    )
    .expect("rewrite lexer should classify shuffle-into-library replacement line");
    assert!(
        super::grammar::abilities::is_shuffle_into_library_from_graveyard_line_lexed(
            &shuffle_tokens
        ),
        "grammar-owned shuffle-into-library probe should match"
    );
    assert!(matches!(
        super::keyword_static::parse_shuffle_into_library_from_graveyard_line(&shuffle_tokens)
            .expect("shuffle-into-library replacement line should parse"),
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
    ));

    let toph_tokens = lex_line(
        "Nontoken artifact lands you control are Mountains in addition to their other types.",
        0,
    )
    .expect("rewrite lexer should classify Toph static line");
    assert!(
        super::grammar::abilities::is_toph_first_metalbender_line_lexed(&toph_tokens),
        "grammar-owned Toph probe should match"
    );
    assert!(matches!(
        super::keyword_static::parse_toph_first_metalbender_line(&toph_tokens)
            .expect("Toph static line should parse"),
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::TophFirstMetalbender
    ));

    let discard_tokens = lex_line(
        "If Mox Diamond would enter the battlefield, you may discard a land card instead. If you don't, put it into its owner's graveyard.",
        0,
    )
    .expect("rewrite lexer should classify discard-or-redirect replacement line");
    assert!(
        super::grammar::abilities::is_discard_or_redirect_replacement_line_lexed(&discard_tokens),
        "grammar-owned discard-or-redirect probe should match"
    );
    assert!(matches!(
        super::keyword_static::parse_discard_or_redirect_replacement_line(&discard_tokens)
            .expect("discard-or-redirect replacement line should parse"),
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::DiscardOrRedirectReplacement
    ));
}

#[test]
fn rewrite_grammar_krrik_life_payment_probe_matches_static_line() {
    let tokens = lex_line(
        "For each {B} in a cost, you may pay 2 life rather than pay that mana.",
        0,
    )
    .expect("rewrite lexer should classify Krrik static line");

    assert!(
        super::grammar::abilities::is_krrik_black_mana_life_payment_line_lexed(&tokens),
        "grammar-owned Krrik probe should match the static line"
    );

    let parsed = super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("Krrik static line should parse")
        .expect("Krrik static line should produce abilities");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::StaticAbilityAst::Static(ability)]
            if ability.id() == crate::static_abilities::StaticAbilityId::BlackManaMayBePaidWithLife
    ));
}

#[test]
fn rewrite_grammar_untap_each_other_players_step_probe_splits_subject_tokens() {
    let tokens = lex_line(
        "Untap all permanents you control during each other player's untap step.",
        0,
    )
    .expect("rewrite lexer should classify untap-step static line");

    let spec =
        super::grammar::abilities::split_untap_each_other_players_untap_step_line_lexed(&tokens)
            .expect("grammar-owned untap-step probe should match the static line");

    assert_eq!(
        render_token_slice(spec.subject_tokens),
        "permanents you control",
        "subject tokens should stop before the untap-step suffix"
    );
}

#[test]
fn rewrite_grammar_players_cant_pay_life_or_sacrifice_probe_matches_static_line() {
    let tokens = lex_line(
        "Players can't pay life or sacrifice nonland permanents to cast spells or activate abilities.",
        0,
    )
    .expect("rewrite lexer should classify anti-life-payment static line");

    assert!(
        super::grammar::abilities::is_players_cant_pay_life_or_sacrifice_line_lexed(&tokens),
        "grammar-owned anti-life-payment probe should match the static line"
    );

    let parsed = super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("anti-life-payment static line should parse")
        .expect("anti-life-payment static line should produce abilities");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::StaticAbilityAst::Static(ability)]
            if ability.id()
                == crate::static_abilities::StaticAbilityId::CantPayLifeOrSacrificeNonlandForCastOrActivate
    ));
}

#[test]
fn rewrite_grammar_minimum_spell_total_mana_probe_matches_static_line() {
    let tokens = lex_line(
        "As long as Trinisphere is untapped, each spell that would cost less than three mana to cast costs three mana to cast.",
        0,
    )
    .expect("rewrite lexer should classify minimum-spell-total-mana static line");

    assert!(
        super::grammar::abilities::is_minimum_spell_total_mana_three_line_lexed(&tokens),
        "grammar-owned minimum-spell-total-mana probe should match the static line"
    );

    let parsed = super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("minimum-spell-total-mana static line should parse")
        .expect("minimum-spell-total-mana static line should produce abilities");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::StaticAbilityAst::Static(ability)]
            if ability.id() == crate::static_abilities::StaticAbilityId::MinimumSpellTotalMana
    ));
}

#[test]
fn cultist_of_the_absolute_static_line_parses_as_static_abilities() {
    let tokens = lex_line(
        "Commander creatures you own get +3/+3 and have flying, deathtouch, \"Ward—Pay 3 life,\" and \"At the beginning of your upkeep, sacrifice a creature.\"",
        0,
    )
    .expect("Cultist of the Absolute static line should lex");

    let trailing = super::keyword_static::parse_anthem_with_trailing_segments_line(&tokens)
        .expect("trailing anthem rule should not error");
    let trailing_debug = format!("{trailing:#?}");
    assert!(
        trailing.is_some(),
        "expected trailing anthem rule to parse Cultist, got {trailing_debug}"
    );

    let parsed = super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("Cultist of the Absolute static line should not error");
    let debug = format!("{parsed:#?}");

    assert!(parsed.is_some(), "expected static abilities, got {debug}");

    let builder = CardDefinitionBuilder::new(CardId::new(), "Cultist of the Absolute")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Background]);
    let preprocessed = super::preprocess::preprocess_document(
        builder,
        "Commander creatures you own get +3/+3 and have flying, deathtouch, \"Ward—Pay 3 life,\" and \"At the beginning of your upkeep, sacrifice a creature.\"",
    )
    .expect("Cultist document should preprocess");
    let cst = super::document_parser::parse_document_cst(&preprocessed, false)
        .expect("Cultist document should parse to CST");
    assert!(
        cst.lines
            .iter()
            .any(|line| matches!(line, super::cst::RewriteLineCst::Static(_))),
        "expected Cultist line to classify as static, got {cst:?}"
    );
}

#[test]
fn rewrite_grammar_permanents_enter_tapped_probe_matches_static_line() {
    let tokens = lex_line("Permanents enter tapped.", 0)
        .expect("rewrite lexer should classify permanents-enter-tapped static line");

    assert!(
        super::grammar::abilities::is_permanents_enter_tapped_line_lexed(&tokens),
        "grammar-owned permanents-enter-tapped probe should match the static line"
    );

    let parsed = super::keyword_static::parse_permanents_enter_tapped_line(&tokens)
        .expect("permanents-enter-tapped static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::AllPermanentsEnterTapped
    ));
}

#[test]
fn rewrite_grammar_creatures_entering_dont_trigger_probe_matches_static_line() {
    let tokens = lex_line("Creatures entering don't cause abilities to trigger.", 0)
        .expect("rewrite lexer should classify anti-trigger static line");

    assert!(
        super::grammar::abilities::is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed(
            &tokens
        ),
        "grammar-owned anti-trigger probe should match the static line"
    );

    let parsed =
        super::keyword_static::parse_creatures_entering_dont_cause_abilities_to_trigger_line(
            &tokens,
        )
        .expect("anti-trigger static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger
    ));
}

#[test]
fn rewrite_grammar_combat_damage_using_toughness_probe_tracks_subject_variant() {
    let tokens = lex_line(
        "Each creature you control assigns combat damage equal to its toughness rather than its power.",
        0,
    )
    .expect("rewrite lexer should classify toughness-damage static line");

    assert_eq!(
        super::grammar::abilities::parse_creatures_assign_combat_damage_using_toughness_line_lexed(
            &tokens
        ),
        Some(super::grammar::abilities::CombatDamageUsingToughnessSubject::EachCreatureYouControl),
        "grammar-owned toughness-damage probe should preserve the subject variant"
    );

    let parsed =
        super::keyword_static::parse_creatures_assign_combat_damage_using_toughness_line(&tokens)
            .expect("toughness-damage static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness
    ));
}

#[test]
fn rewrite_grammar_players_cant_cycle_probe_matches_static_line() {
    let tokens = lex_line("Players can't cycle cards.", 0)
        .expect("rewrite lexer should classify anti-cycle static line");

    assert!(
        super::grammar::abilities::is_players_cant_cycle_line_lexed(&tokens),
        "grammar-owned anti-cycle probe should match the static line"
    );

    let parsed = super::keyword_static::parse_players_cant_cycle_line(&tokens)
        .expect("anti-cycle static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::PlayersCantCycle
    ));
}

#[test]
fn rewrite_grammar_exact_static_line_probes_match_simple_keyword_static_shapes() {
    type Probe = fn(&[crate::cards::builders::parser::lexer::OwnedLexToken]) -> bool;
    type Parser = fn(
        &[crate::cards::builders::parser::lexer::OwnedLexToken],
    ) -> Result<Option<crate::static_abilities::StaticAbility>, CardTextError>;

    for (text, probe, parser, expected_id) in [
        (
            "Players skip their upkeep steps.",
            super::grammar::abilities::is_players_skip_upkeep_line_lexed as Probe,
            super::keyword_static::parse_players_skip_upkeep_line as Parser,
            crate::static_abilities::StaticAbilityId::PlayersSkipUpkeep,
        ),
        (
            "All permanents are colorless.",
            super::grammar::abilities::is_all_permanents_colorless_line_lexed as Probe,
            super::keyword_static::parse_all_permanents_colorless_line as Parser,
            crate::static_abilities::StaticAbilityId::MakeColorless,
        ),
        (
            "Nonbasic lands are Mountains.",
            super::grammar::abilities::is_blood_moon_line_lexed as Probe,
            super::keyword_static::parse_blood_moon_line as Parser,
            crate::static_abilities::StaticAbilityId::BloodMoon,
        ),
        (
            "All lands are no longer snow.",
            super::grammar::abilities::is_remove_snow_line_lexed as Probe,
            super::keyword_static::parse_remove_snow_line as Parser,
            crate::static_abilities::StaticAbilityId::RemoveSupertypes,
        ),
        (
            "You have no maximum hand size.",
            super::grammar::abilities::is_no_maximum_hand_size_line_lexed as Probe,
            super::keyword_static::parse_no_maximum_hand_size_line as Parser,
            crate::static_abilities::StaticAbilityId::NoMaximumHandSize,
        ),
        (
            "This can be your commander.",
            super::grammar::abilities::is_can_be_your_commander_line_lexed as Probe,
            super::keyword_static::parse_can_be_your_commander_line as Parser,
            crate::static_abilities::StaticAbilityId::CanBeCommander,
        ),
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify simple static line");

        assert!(
            probe(&tokens),
            "{text}: grammar-owned exact-shape probe should match"
        );

        let parsed = parser(&tokens).expect("simple static line should parse");
        assert!(
            matches!(parsed, Some(ref ability) if ability.id() == expected_id),
            "{text}: {parsed:?}"
        );
    }
}

#[test]
fn rewrite_grammar_creatures_cant_block_probe_matches_static_line() {
    let tokens = lex_line("Creatures can't block.", 0)
        .expect("rewrite lexer should classify cant-block static line");

    assert!(
        super::grammar::abilities::is_creatures_cant_block_line_lexed(&tokens),
        "grammar-owned cant-block probe should match the static line"
    );

    let parsed = super::keyword_static::parse_creatures_cant_block_line(&tokens)
        .expect("cant-block static line should parse");

    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::GrantStaticAbility { filter, ability, .. })
            if filter == crate::filter::ObjectFilter::creature()
                && matches!(
                    ability.as_ref(),
                    crate::cards::builders::StaticAbilityAst::Static(ability)
                        if ability.id() == crate::static_abilities::StaticAbilityId::CantBlock
                )
    ));
}

#[test]
fn rewrite_grammar_prevention_static_line_probes_match_keyword_static_shapes() {
    type Probe = fn(&[crate::cards::builders::parser::lexer::OwnedLexToken]) -> bool;
    type Parser = fn(
        &[crate::cards::builders::parser::lexer::OwnedLexToken],
    ) -> Result<Option<crate::static_abilities::StaticAbility>, CardTextError>;

    for (text, probe, parser, expected_id) in [
        (
            "Prevent all damage that would be dealt to creatures.",
            super::grammar::abilities::is_prevent_all_damage_dealt_to_creatures_line_lexed as Probe,
            super::keyword_static::parse_prevent_all_damage_dealt_to_creatures_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllDamageDealtToCreatures,
        ),
        (
            "If damage would be dealt to another creature you control, prevent that damage. Put a +1/+1 counter on that creature for each 1 damage prevented this way.",
            super::grammar::abilities::is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed as Probe,
            super::keyword_static::parse_prevent_damage_to_other_creature_you_control_put_counters_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventDamageToOtherCreatureYouControlPutCountersInstead,
        ),
        (
            "Prevent all combat damage that would be dealt to this creature.",
            super::grammar::abilities::is_prevent_all_combat_damage_to_source_line_lexed as Probe,
            super::keyword_static::parse_prevent_all_combat_damage_to_source_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllCombatDamageToSelf,
        ),
        (
            "Prevent all damage that would be dealt to this permanent by creatures.",
            super::grammar::abilities::is_prevent_all_damage_to_source_by_creatures_line_lexed as Probe,
            super::keyword_static::parse_prevent_all_damage_to_source_by_creatures_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllDamageToSelfByCreatures,
        ),
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify prevention static line");

        assert!(probe(&tokens), "{text}: grammar-owned prevention probe should match");

        let parsed = parser(&tokens).expect("prevention static line should parse");
        assert!(
            matches!(parsed, Some(ref ability) if ability.id() == expected_id),
            "{text}: {parsed:?}"
        );
    }
}

#[test]
fn rewrite_grammar_skulk_rules_text_probe_matches_static_line() {
    let tokens = lex_line(
        "Creatures with power less than this creature's power can't block this creature.",
        0,
    )
    .expect("rewrite lexer should classify skulk rules text");

    assert!(
        super::grammar::abilities::is_skulk_rules_text_line_lexed(&tokens),
        "grammar-owned skulk probe should match"
    );

    let parsed = super::keyword_static::parse_skulk_rules_text_line(&tokens)
        .expect("skulk line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::CantBeBlockedByLowerPowerThanSource
    ));
}

#[test]
fn rewrite_grammar_tap_status_and_max_cards_helpers_match_keyword_static_shapes() {
    for (text, expected) in [
        (
            "this creature is tapped",
            crate::ConditionExpr::SourceIsTapped,
        ),
        (
            "this permanent is untapped",
            crate::ConditionExpr::SourceIsUntapped,
        ),
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify tap-status condition");
        assert_eq!(
            super::grammar::abilities::parse_source_tap_status_condition_lexed(&tokens),
            Some(expected),
            "{text}: grammar-owned tap-status helper should match"
        );
    }

    let tokens = lex_line(
        "cards in the hand of the opponent with the most cards in hand",
        0,
    )
    .expect("rewrite lexer should classify max-cards-in-hand value");
    assert_eq!(
        super::grammar::values::parse_max_cards_in_hand_value_lexed(&tokens),
        Some(crate::effect::Value::MaxCardsInHand(
            crate::target::PlayerFilter::Opponent,
        )),
        "grammar-owned max-cards helper should match Adamaro-style wording"
    );
}

#[test]
fn rewrite_grammar_flying_block_probes_match_keyword_static_shapes() {
    let flying_only = lex_line(
        "This creature can't be blocked except by creatures with flying.",
        0,
    )
    .expect("rewrite lexer should classify flying-only restriction");
    assert_eq!(
        super::grammar::abilities::parse_flying_block_restriction_line_lexed(&flying_only),
        Some(super::grammar::abilities::FlyingBlockRestrictionKind::FlyingOnly),
        "grammar-owned flying-only probe should match"
    );
    let parsed_flying_only = super::keyword_static::parse_flying_restriction_line(&flying_only)
        .expect("flying-only restriction should parse");
    assert!(matches!(
        parsed_flying_only,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::FlyingOnlyRestriction
    ));

    let flying_or_reach = lex_line(
        "This can't be blocked except by creatures with flying or reach.",
        0,
    )
    .expect("rewrite lexer should classify flying-or-reach restriction");
    assert_eq!(
        super::grammar::abilities::parse_flying_block_restriction_line_lexed(&flying_or_reach),
        Some(super::grammar::abilities::FlyingBlockRestrictionKind::FlyingOrReach),
        "grammar-owned flying-or-reach probe should match"
    );
    let parsed_flying_or_reach =
        super::keyword_static::parse_flying_restriction_line(&flying_or_reach)
            .expect("flying-or-reach restriction should parse");
    assert!(matches!(
        parsed_flying_or_reach,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::FlyingRestriction
    ));

    let only_flying = lex_line("Can block only creatures with flying.", 0)
        .expect("rewrite lexer should classify can-block-only-flying restriction");
    assert!(
        super::grammar::abilities::is_can_block_only_flying_line_lexed(&only_flying),
        "grammar-owned can-block-only-flying probe should match"
    );
    let parsed_only_flying = super::keyword_static::parse_can_block_only_flying_line(&only_flying)
        .expect("can-block-only-flying restriction should parse");
    assert!(matches!(
        parsed_only_flying,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::CanBlockOnlyFlying
    ));
}

#[test]
fn rewrite_grammar_static_marker_exact_probes_match_keyword_static_shapes() {
    type Probe = fn(&[crate::cards::builders::parser::lexer::OwnedLexToken]) -> bool;

    for (text, probe, expected_id) in [
        (
            "You have shroud.",
            super::grammar::abilities::is_you_have_shroud_line_lexed as Probe,
            crate::static_abilities::StaticAbilityId::RuleRestriction,
        ),
        (
            "Creatures without flying can't attack.",
            super::grammar::abilities::is_creatures_without_flying_cant_attack_line_lexed
                as Probe,
            crate::static_abilities::StaticAbilityId::RuleRestriction,
        ),
        (
            "This creature can't attack alone.",
            super::grammar::abilities::is_this_creature_cant_attack_alone_line_lexed as Probe,
            crate::static_abilities::StaticAbilityId::RuleRestriction,
        ),
        (
            "This creature can't attack its owner.",
            super::grammar::abilities::is_this_creature_cant_attack_its_owner_line_lexed as Probe,
            crate::static_abilities::StaticAbilityId::CantAttackItsOwner,
        ),
        (
            "Lands don't untap during their controller's untap steps.",
            super::grammar::abilities::is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed as Probe,
            crate::static_abilities::StaticAbilityId::RuleRestriction,
        ),
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify static-marker exact line");
        assert!(probe(&tokens), "{text}: grammar-owned static-marker probe should match");

        let parsed =
            super::keyword_static::parse_static_text_marker_line(&tokens).expect("line should parse");
        assert_eq!(parsed.id(), expected_id, "{text}: {parsed:?}");
    }
}

#[test]
fn rewrite_grammar_assign_damage_as_unblocked_probe_matches_keyword_static_shape() {
    let tokens = lex_line(
        "You may have this creature assign its combat damage as though it weren't blocked.",
        0,
    )
    .expect("rewrite lexer should classify assign-damage-as-unblocked text");
    assert!(
        super::grammar::abilities::is_may_assign_damage_as_unblocked_line_lexed(&tokens),
        "grammar-owned assign-damage-as-unblocked probe should match"
    );

    let parsed = super::keyword_static::parse_assign_damage_as_unblocked_line(&tokens)
        .expect("assign-damage-as-unblocked line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
    ));
}

#[test]
fn rewrite_grammar_exact_permission_static_line_probes_match_keyword_static_shapes() {
    type Probe = fn(&[crate::cards::builders::parser::lexer::OwnedLexToken]) -> bool;
    type Parser = fn(
        &[crate::cards::builders::parser::lexer::OwnedLexToken],
    ) -> Result<Option<crate::static_abilities::StaticAbility>, CardTextError>;

    for (text, probe, parser, expected_id) in [
        (
            "You may look at the top card of your library any time.",
            super::grammar::abilities::is_you_may_look_top_card_any_time_line_lexed as Probe,
            super::keyword_static::parse_you_may_look_top_card_any_time_line as Parser,
            crate::static_abilities::StaticAbilityId::RuleFallbackText,
        ),
        (
            "You may cast this spell as though it had flash.",
            super::grammar::abilities::is_cast_this_spell_as_though_it_had_flash_line_lexed
                as Probe,
            super::keyword_static::parse_cast_this_spell_as_though_it_had_flash_line as Parser,
            crate::static_abilities::StaticAbilityId::Flash,
        ),
        (
            "You may play lands from your graveyard.",
            super::grammar::abilities::is_play_lands_from_graveyard_line_lexed as Probe,
            super::keyword_static::parse_play_lands_from_graveyard_line as Parser,
            crate::static_abilities::StaticAbilityId::Grants,
        ),
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify exact permission static line");

        assert!(
            probe(&tokens),
            "{text}: grammar-owned exact-shape probe should match"
        );

        let parsed = parser(&tokens).expect("exact permission static line should parse");
        assert!(
            matches!(parsed, Some(ref ability) if ability.id() == expected_id),
            "{text}: {parsed:?}"
        );
    }
}

#[test]
fn rewrite_grammar_chosen_type_static_line_probes_match_keyword_static_shapes() {
    type Probe = fn(&[crate::cards::builders::parser::lexer::OwnedLexToken]) -> bool;
    type Parser = fn(
        &[crate::cards::builders::parser::lexer::OwnedLexToken],
    ) -> Result<Option<crate::static_abilities::StaticAbility>, CardTextError>;

    for (text, probe, parser, expected_id) in [
        (
            "Enchanted land is the chosen type.",
            super::grammar::abilities::is_enchanted_land_is_chosen_type_line_lexed as Probe,
            super::keyword_static::parse_enchanted_land_is_chosen_type_line as Parser,
            crate::static_abilities::StaticAbilityId::EnchantedLandIsChosenType,
        ),
        (
            "Double all damage that sources you control of the chosen type would deal.",
            super::grammar::abilities::is_double_damage_from_sources_you_control_of_chosen_type_line_lexed as Probe,
            super::keyword_static::parse_double_damage_from_sources_you_control_of_chosen_type_line as Parser,
            crate::static_abilities::StaticAbilityId::DoubleDamageFromSourcesYouControlOfChosenType,
        ),
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify chosen-type static line");

        assert!(probe(&tokens), "{text}: grammar-owned chosen-type probe should match");

        let parsed = parser(&tokens).expect("chosen-type static line should parse");
        assert!(
            matches!(parsed, Some(ref ability) if ability.id() == expected_id),
            "{text}: {parsed:?}"
        );
    }

    let source_tokens = lex_line(
        "This creature is the chosen type in addition to its other types.",
        0,
    )
    .expect("rewrite lexer should classify chosen-type addition line");

    assert_eq!(
        super::grammar::abilities::parse_source_is_chosen_type_in_addition_line_lexed(
            &source_tokens
        ),
        Some("This creature is the chosen type in addition to its other types."),
        "grammar-owned chosen-type addition probe should preserve the display wording"
    );

    let parsed =
        super::keyword_static::parse_source_is_chosen_type_in_addition_line(&source_tokens)
            .expect("chosen-type addition line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::AddChosenCreatureType
    ));
}

#[test]
fn rewrite_lexed_triggered_line_supports_tivit_vote_trigger_body() {
    let triggered_tokens = lex_line(
        "Whenever this creature enters the battlefield or deals combat damage to a player, starting with you, each player votes for evidence or bribery. For each evidence vote, investigate. For each bribery vote, create a Treasure token. You may vote an additional time.",
        0,
    )
    .expect("rewrite lexer should classify tivit trigger probe");

    let parsed = super::clause_support::parse_triggered_line_lexed(&triggered_tokens);
    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
fn rewrite_lexed_vote_start_sentence_supports_object_votes() {
    let tokens = lex_line(
        "Starting with you, each player votes for a nonland permanent you don't control.",
        0,
    )
    .expect("rewrite lexer should classify council vote probe");

    let parsed = parse_effect_sentence_lexed(&tokens);
    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
fn rewrite_lexed_vote_followups_support_vote_conditions_and_winner_filters() {
    for text in [
        "If death gets more votes, each opponent sacrifices a creature of their choice.",
        "If torture gets more votes or the vote is tied, each opponent loses 4 life.",
        "Exile each permanent with the most votes or tied for most votes.",
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify vote follow-up probe");
        let parsed = parse_effect_sentence_lexed(&tokens);
        assert!(parsed.is_ok(), "{text}: {parsed:?}");
    }
}

#[test]
fn rewrite_lexed_vote_sentence_sequences_support_representative_line_families() {
    for text in [
        "Starting with you, each player votes for death or torture. If death gets more votes, each opponent sacrifices a creature of their choice. If torture gets more votes or the vote is tied, each opponent loses 4 life.",
        "Each player secretly votes for truth or consequences, then those votes are revealed. For each truth vote, draw a card. Then choose an opponent at random. For each consequences vote, Truth or Consequences deals 3 damage to that player.",
        "Starting with you, each player votes for death or taxes. For each death vote, each opponent sacrifices a creature of their choice. For each taxes vote, Each opponent discards a card.",
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify vote sequence probe");
        let parsed = super::clause_support::parse_effect_sentences_lexed(&tokens);
        assert!(parsed.is_ok(), "{text}: {parsed:?}");
    }
}

#[test]
fn rewrite_lexed_vote_sentence_sequences_keep_elrond_vote_branches() {
    let text = "Each player secretly votes for fellowship or aid, then those votes are revealed. For each fellowship vote, the voter chooses a creature they control. You gain control of each creature chosen this way, and they gain \"This creature can't attack its owner.\" Then for each aid vote, put a +1/+1 counter on each creature you control.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify elrond vote probe");
    let parsed = super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("elrond vote sequence should parse");
    let debug = format!("{parsed:#?}");
    assert!(debug.contains("VoteStart"), "{debug}");
    assert!(debug.contains("VoteOption"), "{debug}");
    assert!(debug.contains("fellowship"), "{debug}");
    assert!(debug.contains("aid"), "{debug}");
}

#[test]
fn rewrite_lexed_trigger_clause_parses_common_native_shapes() {
    let dies_tokens = lex_line("another creature dies", 0)
        .expect("rewrite lexer should classify dies trigger probe");
    let upkeep_tokens = lex_line("the beginning of your upkeep", 0)
        .expect("rewrite lexer should classify upkeep trigger probe");
    let etb_tokens = lex_line(
        "one or more goblins enter the battlefield under your control",
        0,
    )
    .expect("rewrite lexer should classify etb trigger probe");
    let spell_tokens = lex_line("you cast an aura, equipment, or vehicle spell", 0)
        .expect("rewrite lexer should classify spell-cast trigger probe");
    let counter_tokens = lex_line("you put one or more -1/-1 counters on a creature", 0)
        .expect("rewrite lexer should classify counter trigger probe");
    let graveyard_tokens = lex_line(
        "a nontoken creature is put into your graveyard from the battlefield",
        0,
    )
    .expect("rewrite lexer should classify graveyard trigger probe");
    let combat_tokens = lex_line("the beginning of each combat", 0)
        .expect("rewrite lexer should classify combat trigger probe");
    let second_main_tokens = lex_line("the beginning of your second main phase", 0)
        .expect("rewrite lexer should classify second-main trigger probe");
    let gift_tokens = lex_line("an opponent gives a gift", 0)
        .expect("rewrite lexer should classify gift-given trigger probe");
    let enchanted_upkeep_tokens = lex_line("the beginning of enchanted player's upkeep", 0)
        .expect("rewrite lexer should classify enchanted player's upkeep trigger probe");

    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&dies_tokens),
        Ok(crate::cards::builders::TriggerSpec::Dies(_))
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&upkeep_tokens),
        Ok(crate::cards::builders::TriggerSpec::BeginningOfUpkeep(
            crate::target::PlayerFilter::You
        ))
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&enchanted_upkeep_tokens),
        Ok(crate::cards::builders::TriggerSpec::BeginningOfUpkeep(
            crate::target::PlayerFilter::TaggedPlayer(tag)
        )) if tag.as_str() == "enchanted"
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&etb_tokens),
        Ok(
            crate::cards::builders::TriggerSpec::EntersBattlefieldOneOrMore(_)
                | crate::cards::builders::TriggerSpec::EntersBattlefield(_)
        )
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&spell_tokens),
        Ok(crate::cards::builders::TriggerSpec::SpellCast { .. })
    ));
    let counter = super::activation_and_restrictions::parse_trigger_clause_lexed(&counter_tokens);
    assert!(
        matches!(
            counter,
            Ok(crate::cards::builders::TriggerSpec::CounterPutOn {
                one_or_more: true,
                ..
            })
        ),
        "{counter:?}"
    );
    let graveyard =
        super::activation_and_restrictions::parse_trigger_clause_lexed(&graveyard_tokens);
    assert!(
        matches!(
            graveyard,
            Ok(crate::cards::builders::TriggerSpec::PutIntoGraveyardFromZone { .. })
        ),
        "{graveyard:?}"
    );
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&combat_tokens),
        Ok(crate::cards::builders::TriggerSpec::BeginningOfCombat(
            crate::target::PlayerFilter::Any
        ))
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&second_main_tokens),
        Ok(
            crate::cards::builders::TriggerSpec::BeginningOfPostcombatMain(
                crate::target::PlayerFilter::You
            )
        )
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&gift_tokens),
        Ok(crate::cards::builders::TriggerSpec::PlayerGivesGift(
            crate::target::PlayerFilter::Opponent
        ))
    ));
}

#[test]
fn rewrite_lexed_triggered_line_handles_punctuation_before_enter_verb() {
    let text = "Whenever one or more noncreature, nonland permanents you control enter, put a +1/+1 counter on target creature you control.";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify comma-separated enter trigger line");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
fn rewrite_lexed_triggered_line_parses_state_trigger_condition() {
    let text = "When you control no Swamps, sacrifice this creature.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify state trigger line");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("state-triggered line should parse");

    match parsed {
        crate::cards::builders::LineAst::Triggered { trigger, .. } => {
            assert!(
                matches!(
                    trigger,
                    crate::cards::builders::TriggerSpec::StateBased { .. }
                ),
                "expected state trigger, got {trigger:?}"
            );
        }
        other => panic!("expected triggered line, got {other:?}"),
    }
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_comma_then_chain() {
    let text = "Discard your hand, then draw four cards.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma-then effect");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper effect sentence parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed effect sentence parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_sentence_matches_wrapper_conditional_dispatch() {
    let text = "If you control an artifact, draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify conditional sentence");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper conditional sentence should parse");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed conditional sentence should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_predicate_parser_matches_wrapper_output() {
    let text = "it's your turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let native = super::parse_predicate_lexed(&lexed).expect("lexed predicate should parse");
    let wrapper = super::parse_predicate_lexed(&compat).expect("wrapper predicate should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_sentence_matches_wrapper_pre_diagnostic_clause_helpers() {
    for text in [
        "The next time a red source of your choice would deal damage to you this turn, prevent that damage.",
        "Double target creature's power until end of turn.",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify clause helper probe");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let wrapper = parse_effect_sentence_lexed(&compat)
            .expect("wrapper clause helper sentence should parse");
        let native =
            parse_effect_sentence_lexed(&lexed).expect("lexed clause helper sentence should parse");

        assert_eq!(format!("{native:?}"), format!("{wrapper:?}"), "{text}");
    }
}

#[test]
fn rewrite_lexed_triggered_line_lifts_intervening_if_simple_clause_after_structure_cutover() {
    let text = "At the beginning of your upkeep, if you control an artifact, draw a card.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify upkeep intervening-if");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("triggered intervening-if line should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BeginningOfUpkeep"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
fn rewrite_lexed_triggered_line_lifts_intervening_if_with_multisentence_body() {
    let text = "At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify postcombat intervening-if trigger");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("intervening-if trigger should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BeginningOfPostcombatMain"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_triggered_line_parses_double_sweep_body() {
    let tokens = lex_line(
        "At the beginning of each combat, double the power and toughness of each creature you control until end of turn.",
        0,
    )
    .expect("rewrite lexer should classify double sweep trigger");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("double sweep trigger should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ScalePowerToughnessAll"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sentence_parses_double_sweep_body() {
    let tokens = lex_line(
        "double the power and toughness of each creature you control until end of turn",
        0,
    )
    .expect("rewrite lexer should classify double sweep effect");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("double sweep effect should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ScalePowerToughnessAll"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sentence_parses_triple_target_pt_body() {
    let tokens = lex_line(
        "triple target creature's power and toughness until end of turn",
        0,
    )
    .expect("rewrite lexer should classify triple target pt effect");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("triple target pt effect should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Scaled"), "{debug}");
    assert!(debug.contains("PowerOf"), "{debug}");
    assert!(debug.contains("ToughnessOf"), "{debug}");
}

#[test]
fn rewrite_effect_entrypoint_keeps_exact_bundle_graveyard_copy_shape_without_lowering() {
    let tokens = lex_line(
        "If this spell was cast from a graveyard, copy this spell and you may choose a new target for the copy.",
        0,
    )
    .expect("rewrite lexer should classify exact bundle effect");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("exact bundle effect should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ThisSpellWasCastFromZone"), "{debug}");
    assert!(debug.contains("CopySpell"), "{debug}");
}

#[test]
fn rewrite_lexed_predicate_parser_matches_grammar_entrypoint_output() {
    let text = "it's your turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");

    let parser_root = super::parse_predicate_lexed(&lexed).expect("lexed predicate should parse");
    let grammar = super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
        .expect("grammar predicate entrypoint should parse");

    assert_eq!(format!("{parser_root:?}"), format!("{grammar:?}"));
}

#[test]
fn rewrite_lexed_predicate_parser_handles_color_contraction() {
    let text = "it's blue";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify color predicate text");

    let parser_root = super::parse_predicate_lexed(&lexed).expect("lexed predicate should parse");
    let grammar = super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
        .expect("grammar predicate entrypoint should parse");
    let debug = format!("{parser_root:?}");

    assert_eq!(debug, format!("{grammar:?}"));
    assert!(
        debug.contains("ItMatches"),
        "expected object-match predicate, got {debug}"
    );
    assert!(
        debug.contains("colors: Some("),
        "expected blue color constraint in predicate, got {debug}"
    );
}

#[test]
fn rewrite_lexed_predicate_parser_matches_grammar_entrypoint_for_this_spell_cast_from_zone() {
    let text = "this spell was cast from a graveyard";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify graveyard-cast predicate");

    let parser_root = super::parse_predicate_lexed(&lexed).expect("lexed predicate should parse");
    let grammar = super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
        .expect("grammar predicate entrypoint should parse");

    assert_eq!(format!("{parser_root:?}"), format!("{grammar:?}"));
    assert!(
        format!("{parser_root:?}").contains("ThisSpellWasCastFromZone(Graveyard)"),
        "expected graveyard-cast predicate AST, got {parser_root:?}"
    );
}

#[test]
fn rewrite_lexed_effect_sentence_keeps_where_x_trailing_clause_after_dispatch_inner_cutover() {
    let text = "Target creature gets +X/+0 until end of turn, where X is its power; target creature gains flying until end of turn.";
    let lexed = lex_line(text, 0)
        .expect("rewrite lexer should classify where-x sentence with trailing clause");

    let parsed =
        parse_effect_sentence_lexed(&lexed).expect("lexed where-x trailing sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PowerOf"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
}

#[test]
fn rewrite_dispatch_inner_split_choose_list_routes_separator_helpers() {
    let tokens = lex_line("an artifact, creature, and enchantment", 0)
        .expect("rewrite lexer should classify choose-list separators");

    let segments: Vec<Vec<String>> = super::split_choose_list(&tokens)
        .into_iter()
        .map(|segment| {
            super::token_word_refs(&segment)
                .into_iter()
                .map(ToString::to_string)
                .collect()
        })
        .collect();

    assert_eq!(
        segments,
        vec![
            vec!["an".to_string(), "artifact".to_string()],
            vec!["creature".to_string()],
            vec!["enchantment".to_string()],
        ]
    );
}

#[test]
fn rewrite_lexed_trigger_clause_supports_this_creature_leaves_battlefield() {
    let tokens = lex_line("this creature leaves the battlefield", 0)
        .expect("rewrite lexer should classify leaves-the-battlefield trigger");

    let parsed = super::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("lexed leaves-the-battlefield trigger should parse");

    assert_eq!(
        format!("{parsed:?}"),
        format!(
            "{:?}",
            crate::cards::builders::TriggerSpec::ThisLeavesBattlefield
        )
    );
}

#[test]
fn rewrite_lexed_triggered_line_supports_leave_battlefield_sacrifice_land() {
    let tokens = lex_line("When this leaves the battlefield, sacrifice a land.", 0)
        .expect("rewrite lexer should classify leave-battlefield sacrifice line");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
fn rewrite_lexed_triggered_line_supports_leave_battlefield_sacrifice_all_non_ogres() {
    let tokens = lex_line(
        "When this creature leaves the battlefield, sacrifice all non-Ogre creatures you control.",
        0,
    )
    .expect("rewrite lexer should classify leave-battlefield sacrifice-all line");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
fn rewrite_lexed_effect_sentence_supports_labeled_spent_to_cast_conditional() {
    let text =
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify labeled spent-to-cast sentence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
fn rewrite_grammar_conditional_family_head_parser_strips_labeled_and_then_if_prefixes() {
    let labeled = lex_line(
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token.",
        0,
    )
    .expect("rewrite lexer should classify labeled conditional sentence");
    let labeled_stripped =
        super::grammar::effects::split_conditional_sentence_family_head_lexed(&labeled)
            .expect("labeled conditional family head should strip to the if clause");

    assert_eq!(
        super::token_word_refs(labeled_stripped)
            .first()
            .map(|word| word.to_ascii_lowercase())
            .as_deref(),
        Some("if")
    );

    let then_if = lex_line("Then if it's blue, create a Treasure token.", 0)
        .expect("rewrite lexer should classify then-if conditional sentence");
    let then_if_stripped =
        super::grammar::effects::split_conditional_sentence_family_head_lexed(&then_if)
            .expect("then-if conditional family head should strip to the if clause");

    assert_eq!(
        super::token_word_refs(then_if_stripped)
            .into_iter()
            .map(|word| word.to_ascii_lowercase())
            .collect::<Vec<_>>(),
        vec!["if", "it's", "blue", "create", "a", "treasure", "token"]
    );
}

#[test]
fn rewrite_lexed_effect_sentence_supports_unlabeled_spent_to_cast_conditional() {
    let text = "If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify unlabeled spent-to-cast sentence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
fn rewrite_lexed_effect_sentence_routes_then_if_conditional_through_grammar_family() {
    let text = "Then if it's blue, create a Treasure token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify then-if sentence");

    let grammar = super::grammar::effects::parse_conditional_sentence_family_lexed(
        &lexed,
        super::effect_sentences::parse_effect_chain_lexed,
    )
    .expect("grammar conditional family parser should succeed")
    .expect("then-if conditional family should be recognized");
    let parsed = super::effect_sentences::parse_effect_sentence_lexed(&lexed)
        .expect("effect sentence parser should route then-if through grammar family");

    assert_eq!(format!("{parsed:?}"), format!("{grammar:?}"));
    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::EffectAst::Conditional { .. }]
    ));
}

#[test]
fn rewrite_lexed_conditional_parser_supports_spent_to_cast_conditional_directly() {
    let text = "If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify unlabeled spent-to-cast sentence");

    let parsed = super::effect_sentences::parse_conditional_sentence_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
fn rewrite_lexed_conditional_parser_routes_comma_clause_through_structure_splitter() {
    let text = "If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma if clause");

    let parsed = super::effect_sentences::parse_conditional_sentence_lexed(&lexed)
        .expect("comma if clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(!matches!(
                predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::CreateTokenWithMods { .. }]
            ));
        }
        other => panic!("expected conditional comma if clause, got {other:?}"),
    }
}

#[test]
fn rewrite_lexed_conditional_parser_routes_commaless_clause_through_structure_splitter() {
    let text = "If at least three blue mana was spent to cast this spell create a Food token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma-less if clause");

    let parsed = super::effect_sentences::parse_conditional_sentence_lexed(&lexed)
        .expect("comma-less if clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(!matches!(
                predicate,
                crate::cards::builders::PredicateAst::Unmodeled(_)
            ));
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::CreateTokenWithMods { .. }]
            ));
        }
        other => panic!("expected conditional comma-less if clause, got {other:?}"),
    }
}

#[test]
fn rewrite_lexed_conditional_parser_keeps_if_you_dont_result_predicate() {
    let text = "If you don't, create a Treasure token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify if-you-don't conditional");

    let parsed = super::effect_sentences::parse_conditional_sentence_lexed(&lexed)
        .expect("if-you-don't conditional should parse");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::DidNot,
            ..
        }]
    ));
}

#[test]
fn rewrite_sentence_primitive_routes_tagged_cards_remain_exiled_through_grammar_family() {
    let lexed = lex_line("If those cards remain exiled, create a Treasure token.", 0)
        .expect("rewrite lexer should classify tagged-cards-remain-exiled sentence");

    let parsed = super::effect_sentences::parse_sentence_if_tagged_cards_remain_exiled(&lexed)
        .expect("sentence primitive should succeed")
        .expect("sentence primitive should recognize tagged-cards-remain-exiled");
    let grammar =
        super::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed(
            &lexed,
            super::effect_sentences::parse_effect_chain_lexed,
        )
        .expect("grammar conditional entrypoint should parse tagged-cards-remain-exiled sentence");

    assert_eq!(format!("{parsed:?}"), format!("{grammar:?}"));
}

#[test]
fn rewrite_lexed_effect_sentence_keeps_when_you_do_result_prefix_after_structure_cutover() {
    let lexed = lex_line("When you do, draw a card.", 0)
        .expect("rewrite lexer should classify when-you-do sentence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("when-you-do sentence should parse through structure helper");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::EffectAst::WhenResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            ..
        }]
    ));
}

#[test]
fn rewrite_lexed_effect_sentence_keeps_trailing_if_clause_after_structure_cutover() {
    let lexed = lex_line("Destroy target creature if it's white.", 0)
        .expect("rewrite lexer should classify trailing-if sentence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("trailing-if sentence should parse through structure helper");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::EffectAst::Conditional { .. }]
    ));
}

#[test]
fn rewrite_copy_clause_keeps_trailing_if_after_structure_cutover() {
    let tokens = lex_line("Copy it if it's blue", 0)
        .expect("rewrite lexer should classify copy clause with trailing if");

    let parsed = super::clause_pattern_helpers::parse_copy_spell_clause(&tokens)
        .expect("copy clause parser should succeed")
        .expect("copy clause should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("CopySpell"), "{debug}");
    assert!(debug.contains("ItMatches"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sentence_supports_delayed_this_turn_trigger_via_lexed_trigger_parser() {
    let lexed = lex_line("When this creature dies this turn, exile this creature.", 0)
        .expect("rewrite lexer should classify delayed this-turn trigger sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("lexed delayed this-turn trigger sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
    assert!(debug.contains("ThisDies"), "{debug}");
    assert!(debug.contains("Exile"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sentence_preserves_conditional_for_leading_instead_followup() {
    let text = "If it's a Human, instead it gets +3/+3 and gains indestructible until end of turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify leading-instead conditional");

    let parsed = parse_effect_sentence_lexed(&lexed).expect("leading-instead conditional");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_preserves_for_each_player_doesnt_predicate() {
    let text = "Each player discards a card. Then each player who didn't discard a creature card this way loses 4 life.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify for-each-player-doesnt sequence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ForEachPlayerDoesNot"), "{debug}");
    assert!(debug.contains("PlayerTaggedObjectMatches"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_return_followup() {
    let text = "Return target creature card from your graveyard to your hand. If you gained 7 or more life this turn, return that card to the battlefield instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify return followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_damage_followup() {
    let text = "This creature deals 1 damage to any target. If that land is a Mountain, this creature deals 2 damage instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify damage followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_toxic_followup() {
    let text = "Target creature you control gets +1/+1 until end of turn. If that creature has toxic, instead it gets +2/+2 until end of turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify toxic followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_creatures_died_count_followup() {
    let text = "If a creature died this turn, you draw a card and you lose 1 life. If seven or more creatures died this turn, instead you draw seven cards and you lose 7 life.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify died-count followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_full_party_followup() {
    let text = "Creatures you control get +1/+0 until end of turn. If you have a full party, creatures you control get +3/+0 until end of turn instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify full-party followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_divvy_pile_choice_bundle() {
    let text = "Exile up to five target permanent cards from your graveyard and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard. (Piles can be empty.)";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify divvy pile text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("divvy_source"), "{debug}");
    assert!(debug.contains("divvy_chosen"), "{debug}");
    assert!(debug.contains("ChooseObjectsAcrossZones"), "{debug}");
    assert!(debug.contains("ReturnToHand"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_divvy_choose_one_of_them_bundle() {
    let text = "You may search your library for exactly two cards not named Burning-Rune Demon that have different names. If you do, reveal those cards. An opponent chooses one of them. Put the chosen card into your hand and the other into your graveyard, then shuffle.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify choose-one-of-them text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("May"), "{debug}");
    assert!(debug.contains("divvy_source"), "{debug}");
    assert!(debug.contains("divvy_chosen"), "{debug}");
    assert!(debug.contains("ChooseObjectsAcrossZones"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(debug.contains("ShuffleLibrary"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_consult_hand_bottom_family() {
    let text = "Reveal cards from the top of your library until you reveal an artifact card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify consult-hand-bottom text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_parses_prefixed_consult_sequence() {
    let text = "Draw a card. Reveal cards from the top of your library until you reveal an artifact card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify prefixed consult text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Draw"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_keeps_consult_cast_bottom_family_parseable() {
    let text = "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost. Put all cards exiled this way that weren't cast this way on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify consult-cast-bottom text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_keeps_reveal_consult_cast_bottom_family_parseable() {
    let text = "Reveal cards from the top of your library until you reveal a nonland card. You may cast that card without paying its mana cost. Then put all revealed cards not cast this way on the bottom of your library in a random order.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify reveal consult-cast-bottom text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_parses_target_opponent_consult_until_eot_cast() {
    let text = "Target opponent exiles cards from the top of their library until they exile a nonland card. Until end of turn, you may cast that card without paying its mana cost.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify target-opponent consult text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("TargetOpponent"), "{debug}");
    assert!(debug.contains("GrantPlayTaggedUntilEndOfTurn"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_target_opponent_consult_cast_bottom_family() {
    let text = "Target opponent exiles cards from the top of their library until they exile an instant or sorcery card. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast this way on the bottom of that library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify chaos-wand consult text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_parses_consult_dynamic_mana_value_gate() {
    let text = "Exile cards from the top of your library until you exile a nonland card. You may cast the exiled card without paying its mana cost if it's a spell with mana value less than or equal to this's power. Put the exiled cards not cast this way on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify dynamic consult gate");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("ManaValueOf(Tagged("), "{debug}");
    assert!(debug.contains("SourcePower"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_parses_consult_fixed_or_less_gate() {
    let text = "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if that spell's mana value is 3 or less. Put the exiled cards not cast this way on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify fixed consult gate");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("operator: LessThanOrEqual"), "{debug}");
    assert!(debug.contains("Fixed(3)"), "{debug}");
    assert!(debug.contains("May"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_copy_cast_cost_reduction_followup() {
    let text = "Copy that card and you may cast the copy. That copy costs {2} less to cast.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify copy-cast reduction text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("as_copy: true"), "{debug}");
    assert!(debug.contains("cost_reduction: Some"), "{debug}");
}

#[test]
fn rewrite_lexed_return_all_not_chosen_this_way_tracks_it_tag_exclusion() {
    let text = "Return all nonland permanents not chosen this way to their owners' hands.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify chosen-this-way return");

    let parsed = parse_effect_sentence_lexed(&lexed).expect("return-all sentence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ReturnAllToHand"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_tainted_pact_loop() {
    let text = "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify tainted pact text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("RepeatProcess"), "{debug}");
    assert!(debug.contains("ExileTopOfLibrary"), "{debug}");
    assert!(debug.contains("MayMoveToZone"), "{debug}");
}

#[test]
fn rewrite_semantic_parse_supports_adamant_spent_to_cast_statement_line()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Adamant Variant")
        .card_types(vec![CardType::Sorcery]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token."
            .to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.as_slice(),
        [RewriteSemanticItem::Statement(_)]
    ));
    Ok(())
}

#[test]
fn rewrite_lowered_supports_adamant_spent_to_cast_statement_line() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Adamant Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("CreateTokenEffect"), "{debug}");
    Ok(())
}

#[test]
fn rewrite_lexed_effect_sentence_supports_spent_to_cast_followup_on_that_permanent() {
    let text = "Tap target artifact or creature an opponent controls. If {S} was spent to cast this spell, that permanent doesn't untap during its controller's next untap step.";
    let lexed = lex_line(text, 0)
        .expect("rewrite lexer should classify Berg Strider-style effect sequence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("Berg Strider-style effect sequence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Tap"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("Untap"), "{debug}");
    assert!(
        debug.contains("Artifact") && debug.contains("Creature"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sentence_supports_radiance_shared_color_fanout() {
    let text = "Radiance — Target creature and each other creature that shares a color with it gain haste until end of turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify labeled radiance fanout sentence");

    let stripped = crate::cards::builders::parse_effect_sentence_lexed(
        lexed
            .split(|token| {
                matches!(
                    token.kind,
                    super::lexer::TokenKind::Dash | super::lexer::TokenKind::EmDash
                )
            })
            .nth(1)
            .expect("labeled sentence should contain body after dash"),
    )
    .expect("rewrite effect sentence parser should support radiance fanout");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should support radiance fanout");
    let direct = crate::cards::builders::parse_shared_color_target_fanout_sentence(
        lexed
            .split(|token| {
                matches!(
                    token.kind,
                    super::lexer::TokenKind::Dash | super::lexer::TokenKind::EmDash
                )
            })
            .nth(1)
            .expect("labeled sentence should contain body after dash"),
    )
    .expect("shared-color primitive should not error");
    let mut lowered_body = lexed
        .split(|token| {
            matches!(
                token.kind,
                super::lexer::TokenKind::Dash | super::lexer::TokenKind::EmDash
            )
        })
        .nth(1)
        .expect("labeled sentence should contain body after dash")
        .to_vec();
    for token in &mut lowered_body {
        token.lowercase_word();
    }
    let lowered_direct =
        crate::cards::builders::parse_shared_color_target_fanout_sentence(&lowered_body)
            .expect("lowered shared-color primitive should not error");
    let debug = format!("{parsed:?}");
    let direct_debug = format!("{direct:?}");
    let lowered_direct_debug = format!("{lowered_direct:?}");
    let stripped_debug = format!("{stripped:?}");

    assert!(
        direct_debug.contains("GrantAbilitiesAll"),
        "expected direct shared-color primitive to build fanout grant effect, got {direct_debug}"
    );
    assert!(
        direct_debug.contains("SharesColorWithTagged"),
        "expected direct shared-color primitive to keep shared-color tagged constraint, got {direct_debug}"
    );
    assert!(
        lowered_direct_debug.contains("GrantAbilitiesAll"),
        "expected lowered shared-color primitive to build fanout grant effect, got {lowered_direct_debug}"
    );
    assert!(
        stripped_debug.contains("GrantAbilitiesAll"),
        "expected stripped sentence parser to preserve fanout grant effect, got {stripped_debug}"
    );
    assert!(
        debug.contains("GrantAbilitiesAll"),
        "expected labeled sentence parser to preserve fanout grant effect, got {debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sentence_preserves_non_vampire_sacrifice_filter() {
    let text = "Each player sacrifices a non-Vampire creature of their choice.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify non-Vampire sacrifice sentence");
    let effects = parse_effect_sentence_lexed(&lexed)
        .expect("lexed non-Vampire sacrifice sentence should parse");
    let debug = format!("{effects:?}");

    assert!(
        debug.contains("card_types: [Creature]"),
        "expected creature filter in parsed effect, got {debug}"
    );
    assert!(
        debug.contains("excluded_subtypes: [Vampire]"),
        "expected excluded Vampire subtype in parsed effect, got {debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_supports_create_for_each_creatures_died() {
    let text = "Create a Treasure token for each creature that died this turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify create-for-each effect");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed create-for-each parser should succeed");

    let debug = format!("{native:?}");
    assert!(
        debug.contains("CreaturesDiedThisTurn"),
        "expected dynamic creature-died count in create clause, got {debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_supports_investigate_for_each_creatures_died() {
    let text = "Investigate for each creature that died this turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify investigate-for-each effect");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed investigate-for-each parser should succeed");

    let debug = format!("{native:?}");
    assert!(
        debug.contains("CreaturesDiedThisTurn"),
        "expected dynamic creature-died count in investigate clause, got {debug}"
    );
}

#[test]
fn rewrite_cost_reduction_line_rejects_unmodeled_activate_if_condition() {
    let tokens = lex_line(
        "this ability costs 1 less to activate if you control an artifact.",
        0,
    )
    .expect("rewrite lexer should classify activated cost reduction");
    let err = parse_cost_reduction_line(&tokens)
        .expect_err("unmodeled activated cost reduction condition should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported activated-ability cost reduction condition"),
        "expected explicit unsupported cost reduction condition, got {message}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_keeps_permission_may_as_grant() {
    let text = "You may play it this turn without paying its mana cost.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify permission sentence");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed permission sentence parser should succeed");

    let native_debug = format!("{native:?}");
    assert!(
        !native_debug.contains("May"),
        "permission-granting may clause should not be wrapped as a May effect: {native_debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_keeps_additional_land_play_as_permission() {
    let text = "You may play an additional land this turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify land-play permission");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed land-play permission parser should succeed");

    let native_debug = format!("{native:?}");
    assert!(
        native_debug.contains("AdditionalLandPlays"),
        "expected additional land-play effect, got {native_debug}"
    );
    assert!(
        !native_debug.contains("May"),
        "land-play permission clause should not be wrapped as a May effect: {native_debug}"
    );
}

#[test]
fn rewrite_count_word_parser_handles_digits_and_words() {
    assert_eq!(parse_count_word_rewrite("2").expect("digit count"), 2);
    assert_eq!(parse_count_word_rewrite("three").expect("word count"), 3);

    let digit_tokens = lex_line("2", 0).expect("lexer should classify digit count");
    assert_eq!(
        super::grammar::values::parse_count_word_tokens(&digit_tokens)
            .expect("token count parser should parse digits"),
        2
    );

    let word_tokens = lex_line("three", 0).expect("lexer should classify word count");
    assert_eq!(
        super::grammar::values::parse_count_word_tokens(&word_tokens)
            .expect("token count parser should parse count words"),
        3
    );
}

#[test]
fn rewrite_mana_symbol_group_parser_handles_hybrid_symbols() {
    let symbols =
        parse_mana_symbol_group_rewrite("{W/U}").expect("parser should parse hybrid mana group");
    assert_eq!(symbols, vec![ManaSymbol::White, ManaSymbol::Blue]);
}

#[test]
fn rewrite_mana_symbol_group_parser_handles_multiple_slash_separators() {
    let symbols = parse_mana_symbol_group_rewrite("{W/U/B}")
        .expect("parser should parse repeated slash-delimited mana symbols");
    assert_eq!(
        symbols,
        vec![ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black]
    );
}

#[test]
fn rewrite_parser_root_values_entrypoints_match_grammar_outputs() {
    let root_count = parse_count_word_rewrite("three")
        .expect("parser-root count-word entrypoint should succeed");
    let grammar_count = super::grammar::values::parse_count_word_rewrite("three")
        .expect("grammar count-word parser should succeed");
    assert_eq!(root_count, grammar_count);

    let root_symbols = parse_mana_symbol_group_rewrite("{W/U/B}")
        .expect("parser-root mana-group entrypoint should succeed");
    let grammar_symbols = super::grammar::values::parse_mana_symbol_group("{W/U/B}")
        .expect("grammar mana-group parser should succeed");
    assert_eq!(root_symbols, grammar_symbols);

    let root_mana_cost = parse_mana_cost_rewrite("{2}{W/U}{B}")
        .expect("parser-root mana-cost entrypoint should succeed");
    let grammar_mana_cost = super::grammar::values::parse_mana_cost_rewrite("{2}{W/U}{B}")
        .expect("grammar mana-cost parser should succeed");
    assert_eq!(root_mana_cost, grammar_mana_cost);

    let root_type_line = parse_type_line_rewrite("Legendary Creature — Elf Druid")
        .expect("parser-root type-line entrypoint should succeed");
    let grammar_type_line = super::grammar::values::parse_type_line_with(
        "Legendary Creature — Elf Druid",
        |word| match word {
            "Legendary" => Some(Supertype::Legendary),
            _ => None,
        },
        |word| match word {
            "Creature" => Some(CardType::Creature),
            _ => None,
        },
        |word| match word {
            "Elf" => Some(Subtype::Elf),
            "Druid" => Some(Subtype::Druid),
            _ => None,
        },
    )
    .expect("grammar type-line parser should succeed");
    assert_eq!(root_type_line.supertypes, grammar_type_line.0);
    assert_eq!(root_type_line.card_types, grammar_type_line.1);
    assert_eq!(root_type_line.subtypes, grammar_type_line.2);
}

#[test]
fn rewrite_shared_mana_cost_parser_keeps_scryfall_and_rewrite_entrypoints_in_sync() {
    let rewrite = parse_mana_cost_rewrite("{2}{W/U}{B}")
        .expect("rewrite mana-cost entrypoint should succeed");
    let scryfall = super::util::parse_scryfall_mana_cost("{2}{W/U}{B}")
        .expect("scryfall mana-cost entrypoint should succeed");

    assert_eq!(rewrite, scryfall);
    assert_eq!(
        super::util::parse_scryfall_mana_cost("").expect("blank scryfall mana cost is empty"),
        crate::mana::ManaCost::new()
    );

    let error = parse_error_message(parse_mana_cost_rewrite("—"));
    assert!(
        error.contains("mana-cost"),
        "expected rewrite mana-cost parser context, got {error}"
    );
}

#[test]
fn rewrite_type_line_parser_handles_supertypes_types_and_subtypes() {
    let parsed = parse_type_line_rewrite("Legendary Creature — Elf Druid")
        .expect("rewrite type-line parser should succeed");
    assert_eq!(parsed.supertypes, vec![Supertype::Legendary]);
    assert_eq!(parsed.card_types, vec![CardType::Creature]);
    assert_eq!(parsed.subtypes, vec![Subtype::Elf, Subtype::Druid]);
}

#[test]
fn rewrite_values_type_line_parser_keeps_front_face_only() {
    let parsed = super::grammar::values::parse_type_line_with(
        "Legendary Creature — Elf Druid // Sorcery",
        |word| match word {
            "Legendary" => Some(Supertype::Legendary),
            _ => None,
        },
        |word| match word {
            "Creature" => Some(CardType::Creature),
            _ => None,
        },
        |word| match word {
            "Elf" => Some(Subtype::Elf),
            "Druid" => Some(Subtype::Druid),
            _ => None,
        },
    )
    .expect("direct values type-line parser should keep the front face");

    assert_eq!(parsed.0, vec![Supertype::Legendary]);
    assert_eq!(parsed.1, vec![CardType::Creature]);
    assert_eq!(parsed.2, vec![Subtype::Elf, Subtype::Druid]);
}

#[test]
fn rewrite_shared_type_line_parser_keeps_conditionals_entrypoint_in_sync() {
    let parsed =
        super::effect_sentences::conditionals::parse_type_line("Legendary Creature — Elf Druid")
            .expect("shared type-line parser should support conditionals entrypoint");
    assert_eq!(parsed.0, vec![Supertype::Legendary]);
    assert_eq!(parsed.1, vec![CardType::Creature]);
    assert_eq!(parsed.2, vec![Subtype::Elf, Subtype::Druid]);
}

#[test]
fn rewrite_shared_scryfall_mana_cost_parser_handles_grouped_and_empty_costs() {
    let parsed = super::util::parse_scryfall_mana_cost("{2}{W/U}{B}")
        .expect("shared mana-cost parser should parse grouped mana costs");

    assert_eq!(
        parsed.pips(),
        vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White, ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]
    );
    assert_eq!(
        super::util::parse_scryfall_mana_cost("—").expect("emdash should mean no mana cost"),
        crate::mana::ManaCost::new()
    );
}

#[test]
fn rewrite_values_parse_value_from_lexed_trims_edge_punctuation() {
    let tokens = lex_line("\"three,\"", 0)
        .expect("rewrite lexer should classify punctuation-wrapped values");
    let (value, used) = super::grammar::values::parse_value_from_lexed(&tokens)
        .expect("direct values parser should trim edge punctuation");

    assert_eq!(value, crate::effect::Value::Fixed(3));
    assert_eq!(used, 1);
}

#[test]
fn rewrite_mana_symbol_group_error_mentions_mana_symbol() {
    let error = parse_error_message(parse_mana_symbol_group_rewrite("{Q}"));
    assert!(
        error.contains("mana-group"),
        "expected grouped mana parser context, got {error}"
    );
    assert!(
        error.contains("mana symbol"),
        "expected mana symbol context, got {error}"
    );
}

#[test]
fn rewrite_modal_header_parse_all_reports_cut_for_partial_choose_range() {
    use super::grammar::primitives::parse_all;

    let tokens = lex_line("Choose up to", 0)
        .expect("rewrite lexer should classify partial modal choose range");
    let error = parse_error_message(parse_all(
        &tokens,
        super::grammar::structure::parse_modal_header_choose_spec,
        "modal-header",
    ));

    assert!(
        error.contains("modal-header"),
        "expected modal-header adapter context, got {error}"
    );
    assert!(
        error.contains("modal choice range"),
        "expected choose-range cut context, got {error}"
    );
    assert!(
        error.contains("up") || error.contains("end of input"),
        "expected committed failure location, got {error}"
    );
}

#[test]
fn rewrite_type_line_error_mentions_type_line_subtypes_after_dash() {
    let error = parse_error_message(parse_type_line_rewrite("Legendary Creature — !"));
    assert!(
        error.contains("type-line"),
        "expected type-line context, got {error}"
    );
    assert!(
        error.contains("subtype"),
        "expected subtype context after em dash, got {error}"
    );
}

#[test]
fn rewrite_type_line_error_reports_cut_at_end_after_em_dash() {
    let error = parse_error_message(parse_type_line_rewrite("Legendary Creature —"));
    assert!(
        error.contains("type-line"),
        "expected type-line context, got {error}"
    );
    assert!(
        error.contains("subtype"),
        "expected subtype cut context after em dash, got {error}"
    );
    assert!(
        error.contains("end of input") || error.contains("line 1"),
        "expected committed end-of-input location, got {error}"
    );
}

#[test]
fn rewrite_activation_cost_parses_sacrifice_segments() {
    let cst = parse_activation_cost_rewrite("Sacrifice a creature")
        .expect("rewrite activation-cost parser should parse sacrifice segments");
    let lowered = lower_activation_cost_cst(&cst)
        .expect("rewrite sacrifice segment should lower to TotalCost");
    assert!(!lowered.is_free());

    let another = parse_activation_cost_rewrite("Sacrifice another creature")
        .expect("rewrite activation-cost parser should preserve 'another creature'");
    let rendered = another
        .segments
        .iter()
        .map(|segment| format!("{segment:?}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        rendered.contains("other: true"),
        "expected rewrite sacrifice CST to preserve 'another', got {rendered}"
    );
}

#[test]
fn rewrite_discard_cost_error_mentions_discard_segment() {
    let error = parse_error_message(parse_activation_cost_rewrite("Discard"));
    assert!(
        error.contains("discard"),
        "expected discard context, got {error}"
    );
}

#[test]
fn rewrite_sacrifice_cost_error_mentions_missing_filter() {
    let error = parse_error_message(parse_activation_cost_rewrite("Sacrifice"));
    assert!(
        error.contains("sacrifice"),
        "expected sacrifice context, got {error}"
    );
    assert!(
        error.contains("filter"),
        "expected missing filter context, got {error}"
    );
}

#[test]
fn rewrite_activation_cost_parses_energy_and_counter_variants() {
    let energy = parse_activation_cost_rewrite("Pay {E}{E}")
        .expect("rewrite activation-cost parser should parse energy payment");
    let bare_energy = parse_activation_cost_rewrite("{E}{E}")
        .expect("rewrite activation-cost parser should parse bare energy payment");
    let counter_add = parse_activation_cost_rewrite("Put a +1/+1 counter on this creature")
        .expect("parser should parse add-counter cost");
    let counter_remove = parse_activation_cost_rewrite("Remove a +1/+1 counter from this creature")
        .expect("parser should parse remove-counter cost");
    let exile_hand = parse_activation_cost_rewrite("Exile a blue card from your hand")
        .expect("parser should parse exile-from-hand cost");

    assert!(matches!(
        energy.segments.as_slice(),
        [super::ActivationCostSegmentCst::Energy(2)]
    ));
    assert!(matches!(
        bare_energy.segments.as_slice(),
        [super::ActivationCostSegmentCst::Energy(2)]
    ));
    assert!(matches!(
        counter_add.segments.as_slice(),
        [super::ActivationCostSegmentCst::PutCounters {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1
        }]
    ));
    assert!(matches!(
        counter_remove.segments.as_slice(),
        [super::ActivationCostSegmentCst::RemoveCounters {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1
        }]
    ));
    assert!(matches!(
        exile_hand.segments.as_slice(),
        [super::ActivationCostSegmentCst::ExileFromHand {
            count: 1,
            color_filter: Some(colors)
        }] if *colors == crate::color::ColorSet::BLUE
    ));
}

#[test]
fn rewrite_activation_cost_parses_pay_mana_life_exert_and_bare_symbols() {
    let pay_life = parse_activation_cost_rewrite("Pay 3 life")
        .expect("rewrite activation-cost parser should parse life payment");
    let pay_mana = parse_activation_cost_rewrite("Pay {W}{U}")
        .expect("rewrite activation-cost parser should parse mana payment");
    let bare_mana = parse_activation_cost_rewrite("{W}{U}")
        .expect("rewrite activation-cost parser should parse bare mana payment");
    let tap = parse_activation_cost_rewrite("{T}")
        .expect("rewrite activation-cost parser should parse tap symbol");
    let untap = parse_activation_cost_rewrite("{Q}")
        .expect("rewrite activation-cost parser should parse untap symbol");
    let exert = parse_activation_cost_rewrite("Exert this creature")
        .expect("rewrite activation-cost parser should parse exert costs");

    assert!(matches!(
        pay_life.segments.as_slice(),
        [super::ActivationCostSegmentCst::Life(3)]
    ));
    match pay_mana.segments.as_slice() {
        [super::ActivationCostSegmentCst::Mana(cost)] => assert_eq!(
            cost.pips(),
            vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]
        ),
        other => panic!("expected mana payment, got {other:?}"),
    }
    match bare_mana.segments.as_slice() {
        [super::ActivationCostSegmentCst::Mana(cost)] => assert_eq!(
            cost.pips(),
            vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]
        ),
        other => panic!("expected bare mana payment, got {other:?}"),
    }
    assert!(matches!(
        tap.segments.as_slice(),
        [super::ActivationCostSegmentCst::Tap]
    ));
    assert!(matches!(
        untap.segments.as_slice(),
        [super::ActivationCostSegmentCst::Untap]
    ));
    assert!(matches!(
        exert.segments.as_slice(),
        [super::ActivationCostSegmentCst::ExertSelf { display_text }]
            if display_text == "Exert this creature"
    ));
}

#[test]
fn rewrite_activation_cost_parses_loyalty_shorthand_without_fallback_escape_hatch() {
    let plus = parse_activation_cost_rewrite("+1")
        .expect("rewrite activation-cost parser should parse +1 loyalty shorthand");
    let minus = parse_activation_cost_rewrite("-2")
        .expect("rewrite activation-cost parser should parse -2 loyalty shorthand");
    let minus_x = parse_activation_cost_rewrite("-X")
        .expect("rewrite activation-cost parser should parse -X loyalty shorthand");
    let zero = parse_activation_cost_rewrite("0")
        .expect("rewrite activation-cost parser should parse zero loyalty shorthand");

    assert!(matches!(
        plus.segments.as_slice(),
        [super::ActivationCostSegmentCst::PutCounters {
            counter_type: CounterType::Loyalty,
            count: 1
        }]
    ));
    assert!(matches!(
        minus.segments.as_slice(),
        [super::ActivationCostSegmentCst::RemoveCounters {
            counter_type: CounterType::Loyalty,
            count: 2
        }]
    ));
    assert!(matches!(
        minus_x.segments.as_slice(),
        [super::ActivationCostSegmentCst::RemoveCountersDynamic {
            counter_type: Some(CounterType::Loyalty),
            display_x: true
        }]
    ));
    assert!(
        zero.segments.is_empty(),
        "zero loyalty shorthand should lower as a free cost"
    );
    assert!(
        lower_activation_cost_cst(&zero)
            .expect("zero loyalty shorthand should lower")
            .is_free()
    );
}

#[test]
fn rewrite_activation_cost_parses_shard_style_without_raw_branch_splitting() {
    let raw = parse_activation_cost_rewrite("{W}, {T} or {U}, {T}")
        .expect("rewrite activation-cost parser should parse shard-style costs");
    let tokens = lex_line("{W}, {T} or {U}, {T}", 0)
        .expect("lexer should classify shard-style activation cost");
    let lexed = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("token activation-cost parser should parse shard-style costs");

    assert_eq!(format!("{raw:?}"), format!("{lexed:?}"));
    match lexed.segments.as_slice() {
        [
            super::ActivationCostSegmentCst::Mana(cost),
            super::ActivationCostSegmentCst::Tap,
        ] => {
            assert_eq!(cost.pips(), vec![vec![ManaSymbol::White, ManaSymbol::Blue]]);
        }
        other => panic!("expected hybrid mana plus tap shard-style cost, got {other:?}"),
    }
}

#[test]
fn rewrite_activation_cost_token_entrypoint_parses_pay_bare_symbol_and_exert_variants() {
    let pay_energy_tokens =
        lex_line("Pay two {E}", 0).expect("lexer should classify counted-energy activation cost");
    let pay_energy_cst = parse_activation_cost_tokens_rewrite(&pay_energy_tokens)
        .expect("token activation-cost parser should parse counted-energy costs");
    assert!(matches!(
        pay_energy_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::Energy(2)]
    ));

    let pay_mana_tokens =
        lex_line("Pay {W}{U}", 0).expect("lexer should classify mana-payment activation cost");
    let pay_mana_cst = parse_activation_cost_tokens_rewrite(&pay_mana_tokens)
        .expect("token activation-cost parser should parse mana-payment costs");
    match pay_mana_cst.segments.as_slice() {
        [super::ActivationCostSegmentCst::Mana(cost)] => assert_eq!(
            cost.pips(),
            vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]
        ),
        other => panic!("expected mana payment, got {other:?}"),
    }

    let tap_tokens = lex_line("{T}", 0).expect("lexer should classify tap-symbol activation cost");
    let tap_cst = parse_activation_cost_tokens_rewrite(&tap_tokens)
        .expect("token activation-cost parser should parse tap-symbol costs");
    assert!(matches!(
        tap_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::Tap]
    ));

    let untap_tokens =
        lex_line("{Q}", 0).expect("lexer should classify untap-symbol activation cost");
    let untap_cst = parse_activation_cost_tokens_rewrite(&untap_tokens)
        .expect("token activation-cost parser should parse untap-symbol costs");
    assert!(matches!(
        untap_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::Untap]
    ));

    let exert_tokens =
        lex_line("Exert this creature", 0).expect("lexer should classify exert activation cost");
    let exert_cst = parse_activation_cost_tokens_rewrite(&exert_tokens)
        .expect("token activation-cost parser should parse exert costs");
    assert!(matches!(
        exert_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::ExertSelf { display_text }]
            if display_text == "Exert this creature"
    ));
}

#[test]
fn rewrite_activation_cost_token_entrypoint_preserves_named_card_commas() {
    let tokens = lex_line("Discard a card named Mishra, Lost to Phyrexia", 0)
        .expect("lexer should preserve punctuation in named-card costs");
    let cst = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("token activation-cost parser should keep named-card commas intact");

    assert!(matches!(
        cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::DiscardFiltered {
            name: Some(name),
            ..
        }] if name == "mishra, lost to phyrexia"
    ));
}

#[test]
fn rewrite_activation_cost_string_entrypoint_matches_named_card_token_path() {
    let raw = parse_activation_cost_rewrite("Discard a card named Mishra, Lost to Phyrexia")
        .expect("string activation-cost parser should preserve named-card punctuation");
    let tokens = lex_line("Discard a card named Mishra, Lost to Phyrexia", 0)
        .expect("lexer should classify named-card activation cost");
    let lexed = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("token activation-cost parser should preserve named-card punctuation");

    assert_eq!(format!("{raw:?}"), format!("{lexed:?}"));
}

#[test]
fn rewrite_activation_cost_token_entrypoint_parses_tap_return_and_exile_variants() {
    let tap_tokens = lex_line("Tap another untapped creature you control", 0)
        .expect("lexer should classify tap-chosen activation cost");
    let tap_cst = parse_activation_cost_tokens_rewrite(&tap_tokens)
        .expect("token activation-cost parser should parse tap-chosen costs");
    assert!(matches!(
        tap_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::TapChosen {
            count: 1,
            filter_text,
            other: true,
        }] if filter_text == "creature you control"
    ));

    let return_tokens = lex_line("Return a creature you control to its owner's hand", 0)
        .expect("lexer should classify return-to-hand activation cost");
    let return_cst = parse_activation_cost_tokens_rewrite(&return_tokens)
        .expect("token activation-cost parser should parse return-to-hand costs");
    assert!(matches!(
        return_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::ReturnChosenToHand { count: 1, filter_text }]
            if filter_text == "creature you control"
    ));

    let exile_tokens = lex_line("Exile one or more cards from your graveyard", 0)
        .expect("lexer should classify exile-from-graveyard activation cost");
    let exile_cst = parse_activation_cost_tokens_rewrite(&exile_tokens)
        .expect("token activation-cost parser should parse exile-from-graveyard costs");
    assert!(matches!(
        exile_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter_text,
        }] if *choice_count == ChoiceCount::at_least(1)
            && filter_text == "cards from your graveyard"
    ));

    let top_library_tokens = lex_line("Exile the top two cards of your library", 0)
        .expect("lexer should classify exile-top-library activation cost");
    let top_library_cst = parse_activation_cost_tokens_rewrite(&top_library_tokens)
        .expect("token activation-cost parser should parse exile-top-library costs");
    assert!(matches!(
        top_library_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::ExileTopLibrary { count: 2 }]
    ));
}

#[test]
fn rewrite_activation_cost_token_entrypoint_parses_counter_variants() {
    let put_tokens = lex_line("Put a +1/+1 counter on a creature you control", 0)
        .expect("lexer should classify put-counter activation cost");
    let put_cst = parse_activation_cost_tokens_rewrite(&put_tokens)
        .expect("token activation-cost parser should parse put-counter costs");
    assert!(matches!(
        put_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::PutCountersChosen {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1,
            filter_text,
        }] if filter_text == "a creature you control"
    ));

    let remove_tokens = lex_line(
        "Remove any number of charge counters from among artifacts you control",
        0,
    )
    .expect("lexer should classify remove-counter activation cost");
    let remove_cst = parse_activation_cost_tokens_rewrite(&remove_tokens)
        .expect("token activation-cost parser should parse remove-counter costs");
    assert!(matches!(
        remove_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type: Some(CounterType::Charge),
            count: 0,
            filter_text,
            display_x: false,
        }] if filter_text == "artifacts you control"
    ));
}

#[test]
fn rewrite_activation_cost_shared_parser_supports_behold_costs() {
    let cst = parse_activation_cost_rewrite("Behold an Elemental")
        .expect("shared activation-cost parser should support behold costs");
    assert!(matches!(
        cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::Behold {
            subtype: Subtype::Elemental,
            count: 1
        }]
    ));

    let tokens =
        lex_line("Behold an Elemental", 0).expect("lexer should classify behold activation cost");
    let lowered = super::parse_activation_cost(&tokens)
        .expect("activated ability entrypoint should use shared behold cost parser");
    assert!(
        !lowered.is_free(),
        "behold costs should survive lowering as a non-free activation cost"
    );
}

#[test]
fn rewrite_activation_cost_shared_parser_supports_mill_costs() {
    let cst = parse_activation_cost_rewrite("Mill two cards")
        .expect("shared activation-cost parser should support mill costs");
    assert!(matches!(
        cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::Mill(2)]
    ));

    let tokens = lex_line("Mill two cards", 0).expect("lexer should classify mill activation cost");
    let token_cst = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("token activation-cost parser should support mill costs");
    assert!(matches!(
        token_cst.segments.as_slice(),
        [super::ActivationCostSegmentCst::Mill(2)]
    ));

    let lowered = super::parse_activation_cost(&tokens)
        .expect("activated ability entrypoint should use shared mill cost parser");
    assert!(
        !lowered.is_free(),
        "mill costs should survive lowering as a non-free activation cost"
    );
}

#[test]
fn rewrite_lowered_simple_card_parses() -> Result<(), CardTextError> {
    let text = "Type: Creature — Spirit\n{1}: This creature gets +1/+1 until end of turn.";
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Spirit");
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;
    assert_eq!(definition.abilities.len(), 1);
    Ok(())
}

#[test]
fn rewrite_lowered_mana_ability_preserves_fixed_mana_groups() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Ring")
        .card_types(vec![CardType::Artifact]);
    let (definition, _) =
        parse_text_with_annotations_lowered(builder, "{T}: Add {C}{C}.".to_string(), false)?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");

    match &ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(activated.is_mana_ability());
            assert_eq!(
                activated.mana_symbols(),
                &[ManaSymbol::Colorless, ManaSymbol::Colorless]
            );
        }
        other => panic!("expected activated mana ability, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_semantic_parse_merges_multiline_spell_when_you_do_followup() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Followup Variant")
        .card_types(vec![CardType::Instant]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Sacrifice a creature.\nWhen you do, draw two cards.".to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.as_slice(),
        [RewriteSemanticItem::Statement(_)]
    ));
    Ok(())
}

#[test]
fn rewrite_semantic_parse_keeps_triggered_double_sweep_body() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Zopandrel Variant")
        .card_types(vec![CardType::Creature]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of each combat, double the power and toughness of each creature you control until end of turn.".to_string(),
        false,
    )?;

    match doc.items.as_slice() {
        [RewriteSemanticItem::Triggered(triggered)] => {
            let debug = format!("{:?}", triggered.parsed);
            assert!(debug.contains("ScalePowerToughnessAll"), "{debug}");
        }
        other => panic!("expected one triggered semantic item, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_semantic_parse_keeps_triggered_triple_sweep_body() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Triple Sweep Variant")
        .card_types(vec![CardType::Enchantment]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of each combat, triple the power and toughness of each creature you control until end of turn.".to_string(),
        false,
    )?;

    match doc.items.as_slice() {
        [RewriteSemanticItem::Triggered(triggered)] => {
            let debug = format!("{:?}", triggered.parsed);
            assert!(debug.contains("ScalePowerToughnessAll"), "{debug}");
            assert!(debug.contains("multiplier: 2"), "{debug}");
        }
        other => panic!("expected one triggered semantic item, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_semantic_parse_keeps_nested_combat_whenever_trigger() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Nested Combat Trigger Variant")
        .card_types(vec![CardType::Creature]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of each combat, unless you pay {1}, whenever this creature attacks, draw a card.".to_string(),
        false,
    )?;

    match doc.items.as_slice() {
        [RewriteSemanticItem::Triggered(triggered)] => {
            assert_eq!(triggered.trigger_text, "this creature attacks");
            assert_eq!(triggered.effect_text, "draw a card.");
        }
        other => panic!("expected one triggered semantic item, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_semantic_parse_keeps_toggo_rock_token_rules_tail() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Toggo, Goblin Weaponsmith")
        .card_types(vec![CardType::Creature]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Landfall — Whenever a land you control enters, create a colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.".to_string(),
        false,
    )?;

    let expect_toggo_token_name = |effects: &[crate::cards::builders::EffectAst]| match effects {
        [crate::cards::builders::EffectAst::CreateTokenWithMods { name, .. }] => {
            let lower_name = name.to_ascii_lowercase();
            assert!(
                lower_name.contains("named rock"),
                "expected named rock token payload, got {name}"
            );
            assert!(
                lower_name.contains("equipped creature has"),
                "expected equipment grant rules tail, got {name}"
            );
            assert!(
                lower_name.contains("equip {1}"),
                "expected equip text in token payload, got {name}"
            );

            let words: Vec<&str> = lower_name
                .split_whitespace()
                .map(|word| {
                    word.trim_matches(|ch: char| {
                        !ch.is_ascii_alphanumeric() && ch != '/' && ch != '+' && ch != '-'
                    })
                })
                .map(|word| match word {
                    "can't" | "cannot" => "cant",
                    "aren't" => "arent",
                    "isn't" => "isnt",
                    "they're" => "theyre",
                    "it's" => "its",
                    "you're" => "youre",
                    _ => word,
                })
                .filter(|word| !word.is_empty())
                .collect();
            let rules_text = super::compile_support::parse_equipment_rules_text(&words, name)
                .expect("toggo token payload should yield equipment rules text");
            let manual_def = CardDefinitionBuilder::new(CardId::new(), "Rock")
                    .token()
                    .card_types(vec![CardType::Artifact])
                    .subtypes(vec![Subtype::Equipment])
                    .with_ability(crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::make_colorless(
                            crate::target::ObjectFilter::source(),
                        ),
                    ))
                    .parse_text(&rules_text)
                    .unwrap_or_else(|err| {
                        panic!(
                            "toggo equipment rules text should parse: {err:?}\nname={name}\nrules_text={rules_text}"
                        )
                    });
            let manual_activated_texts = manual_def
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    crate::ability::AbilityKind::Activated(_) => ability.text.as_deref(),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                manual_activated_texts
                    .iter()
                    .any(|text| *text == "Equip {1}"),
                "expected manual token reparse to keep equip, got {manual_activated_texts:?}"
            );

            let def = super::compile_support::token_definition_for(name)
                .expect("toggo token payload should round-trip into a token definition");
            let activated_texts = def
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    crate::ability::AbilityKind::Activated(_) => ability.text.as_deref(),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                activated_texts.iter().any(|text| *text == "Equip {1}"),
                "expected round-tripped token to keep equip, got {activated_texts:?}"
            );
        }
        other => panic!("expected a single token creation effect, got {other:?}"),
    };

    match doc.items.as_slice() {
        [RewriteSemanticItem::Triggered(triggered)] => match &triggered.parsed {
            crate::cards::builders::LineAst::Triggered { effects, .. } => {
                expect_toggo_token_name(effects);
            }
            crate::cards::builders::LineAst::Ability(parsed) => {
                let Some(effects) = parsed.effects_ast.as_ref() else {
                    panic!("expected landfall ability to keep parsed effects ast");
                };
                expect_toggo_token_name(effects);
            }
            other => panic!("expected triggered line ast, got {other:?}"),
        },
        other => panic!("expected one triggered semantic item, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_semantic_parse_keeps_trigger_trigger_caps_and_first_time_suffixes()
-> Result<(), CardTextError> {
    let (capped_doc, _) = parse_text_to_semantic_document(
        CardDefinitionBuilder::new(CardId::new(), "Capped Trigger Variant")
            .card_types(vec![CardType::Enchantment]),
        "Whenever one or more creatures attack you, draw a card. This ability triggers only once each turn.".to_string(),
        false,
    )?;

    match capped_doc.items.as_slice() {
        [RewriteSemanticItem::Triggered(triggered)] => {
            assert_eq!(triggered.max_triggers_per_turn, Some(1));
        }
        other => panic!("expected one triggered semantic item, got {other:?}"),
    }

    let (first_time_doc, _) = parse_text_to_semantic_document(
        CardDefinitionBuilder::new(CardId::new(), "First Time Trigger Variant")
            .card_types(vec![CardType::Enchantment]),
        "Whenever one or more creatures attack you for the first time each turn, draw a card."
            .to_string(),
        false,
    )?;

    match first_time_doc.items.as_slice() {
        [RewriteSemanticItem::Triggered(triggered)] => {
            assert_eq!(triggered.max_triggers_per_turn, Some(1));
            assert_eq!(triggered.trigger_text, "one or more creatures attack you");
            assert_eq!(triggered.effect_text, "draw a card.");
        }
        other => panic!("expected one triggered semantic item, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_semantic_parse_keeps_intervening_if_trigger_split() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Intervening If Trigger Variant")
        .card_types(vec![CardType::Enchantment]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of your upkeep, if you control an artifact, draw a card.".to_string(),
        false,
    )?;

    match doc.items.as_slice() {
        [RewriteSemanticItem::Triggered(triggered)] => {
            assert_eq!(triggered.trigger_text, "the beginning of your upkeep");
            assert_eq!(triggered.effect_text, "draw a card.");
        }
        other => panic!("expected one triggered semantic item, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_semantic_parse_marks_plumb_additional_cost_as_non_choice() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Plumb Variant")
        .card_types(vec![CardType::Instant]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "As an additional cost to cast this spell, you may sacrifice one or more creatures. When you do, copy this spell for each creature sacrificed this way.\nYou draw a card and you lose 1 life.".to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.first(),
        Some(RewriteSemanticItem::Keyword(keyword))
            if keyword.kind == RewriteKeywordLineKind::AdditionalCost
    ));
    Ok(())
}

#[test]
fn rewrite_lowered_former_section9_cases_parse_without_fallback_text() -> Result<(), CardTextError>
{
    let cases = vec![
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Poison")
                .card_types(vec![CardType::Creature]),
            "Whenever this creature deals damage to a player, that player gets a poison counter. The player gets another poison counter at the beginning of their next upkeep unless they pay {2} before that step. (A player with ten or more poison counters loses the game.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Unearth")
                .card_types(vec![CardType::Artifact, CardType::Creature]),
            "Permanents you control have \"Ward—Sacrifice a permanent.\"\nEach artifact card in your graveyard has unearth {1}{B}{R}. ({1}{B}{R}: Return the card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Unearth only as a sorcery.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Sticker")
                .card_types(vec![CardType::Sorcery]),
            "Put an art sticker on a nonland permanent you own. Then ask a person outside the game to rate its new art on a scale from 1 to 5, where 5 is the best. When they rate the art, up to that many target creatures can't block this turn.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Can’t Block")
                .card_types(vec![CardType::Creature]),
            "This creature can't be blocked by more than one creature.\nEach creature you control with a +1/+1 counter on it can't be blocked by more than one creature.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 White Destroy")
                .card_types(vec![CardType::Sorcery]),
            "Destroy target creature if it's white. A creature destroyed this way can't be regenerated.\nDraw a card at the beginning of the next turn's upkeep.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Spent")
                .card_types(vec![CardType::Instant]),
            "Create two 1/1 white Kithkin Soldier creature tokens if {W} was spent to cast this spell. Counter up to one target creature spell if {U} was spent to cast this spell. (Do both if {W}{U} was spent.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Goats")
                .card_types(vec![CardType::Artifact]),
            "{T}: Add {C}.\n{4}, {T}: Create a 0/1 white Goat creature token.\n{T}, Sacrifice X Goats: Add X mana of any one color. You gain X life.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Exile Top")
                .card_types(vec![CardType::Sorcery]),
            "Shuffle your library, then exile the top four cards. You may cast any number of spells with mana value 5 or less from among them without paying their mana costs. Lands you control don't untap during your next untap step.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Cloak")
                .card_types(vec![CardType::Sorcery]),
            "Exile target nontoken creature you own and the top two cards of your library in a face-down pile, shuffle that pile, then cloak those cards. They enter tapped. (To cloak a card, put it onto the battlefield face down as a 2/2 creature with ward {2}. Turn it face up any time for its mana cost if it's a creature card.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Toughness")
                .card_types(vec![CardType::Instant]),
            "Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Or")
                .card_types(vec![CardType::Sorcery]),
            "Destroy all lands or all creatures. Creatures destroyed this way can't be regenerated.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Nonblack")
                .card_types(vec![CardType::Sorcery]),
            "Destroy two target nonblack creatures unless either one is a color the other isn't. They can't be regenerated.",
        ),
    ];

    let mut failures = Vec::new();

    for (builder, text) in cases {
        let (definition, _) =
            match parse_text_with_annotations_lowered(builder, text.to_string(), false) {
                Ok(parsed) => parsed,
                Err(err) => {
                    failures.push(format!(
                        "former section-9 case failed to parse: {text}\n{err:?}"
                    ));
                    continue;
                }
            };
        let has_fallback_text = crate::ability::extract_static_abilities(&definition.abilities)
            .iter()
            .any(|ability| {
                matches!(
                    ability.id(),
                    StaticAbilityId::RuleFallbackText | StaticAbilityId::KeywordFallbackText
                )
            });
        assert!(
            !has_fallback_text,
            "former section-9 case should lower without fallback text: {text}"
        );
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));

    Ok(())
}

#[test]
fn parse_subject_first_exile_top_library_then_play_permission_bundle() {
    let builder = CardDefinitionBuilder::new(CardId::from_raw(1), "Bundle Probe")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Target player exiles the top two cards of their library. Until end of turn, you may play those cards without paying their mana costs."
            .to_string(),
        false,
    )
    .expect("the Fallen Shinobi style bundle should lower cleanly");
    let debug = format!("{:#?}", definition.spell_effect).to_ascii_lowercase();

    assert!(
        debug.contains("exiletopoflibraryeffect"),
        "expected top-library exile in the bundle, got {debug}"
    );
    assert!(
        debug.contains("grantplaytaggedeffect"),
        "expected play-from-exile permission in the bundle, got {debug}"
    );
    assert!(
        debug.contains("granttaggedspellfreecastuntilendofturneffect"),
        "expected free-cast permission in the bundle, got {debug}"
    );
}

#[test]
fn rewrite_preprocess_expands_same_is_true_trigger_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thunderous Orator Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, it gains flying until end of turn if you control a creature with flying. The same is true for first strike and vigilance.",
        )
        .expect("same-is-true trigger chain should parse");

    let rendered = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("whenever this creature attacks, it gains flying until end of turn"),
        "expected flying branch to remain, got {rendered}"
    );
    assert!(
        rendered
            .contains("whenever this creature attacks, it gains first strike until end of turn")
            && rendered
                .contains("whenever this creature attacks, it gains vigilance until end of turn"),
        "expected remaining borrowed keyword branches, got {rendered}"
    );
}

#[test]
fn rewrite_preprocess_expands_same_is_true_static_graveyard_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cairn Wanderer Variant")
        .parse_text(
            "As long as a creature card with flying is in a graveyard, this creature has flying. The same is true for first strike and vigilance.",
        )
        .expect("same-is-true static graveyard chain should parse");

    let rendered = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("as long as there is a creature card with flying in a graveyard")
            && rendered.contains("this creature has flying"),
        "expected flying graveyard condition to be normalized, got {rendered}"
    );
    assert!(
        rendered.contains("as long as there is a creature card with first strike in a graveyard")
            && rendered.contains("this creature has first strike")
            && rendered
                .contains("as long as there is a creature card with vigilance in a graveyard")
            && rendered.contains("this creature has vigilance"),
        "expected same-is-true graveyard branches to expand, got {rendered}"
    );
}

#[test]
fn rewrite_preprocess_expands_same_is_true_static_exile_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Urborg Scavengers Variant")
        .parse_text(
            "This creature has flying as long as a card exiled with it has flying. The same is true for trample and vigilance.",
        )
        .expect("same-is-true exile chain should parse");

    let rendered = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "this creature has flying as long as there is a card exiled with it with flying"
        ),
        "expected exile-linked flying condition to be normalized, got {rendered}"
    );
    assert!(
        rendered.contains(
            "this creature has trample as long as there is a card exiled with it with trample"
        ) && rendered.contains(
            "this creature has vigilance as long as there is a card exiled with it with vigilance"
        ),
        "expected same-is-true exile branches to expand, got {rendered}"
    );
}

#[test]
fn parse_choose_then_do_same_for_filter_splits_one_of_mana_values() {
    let tokens = lex_line(
        "choose a creature card with mana value 1 in your graveyard, then do the same for creature cards with mana value 2 and 3.",
        0,
    )
    .expect("choose-then-do-the-same sentence should lex");

    let effects = super::parse_sentence_choose_then_do_same_for_filter(&tokens)
        .expect("choose-then-do-the-same primitive should not error")
        .expect("choose-then-do-the-same primitive should match");
    let debug = format!("{effects:#?}");

    assert_eq!(
        debug.matches("ChooseObjects").count(),
        3,
        "expected three choose-object AST nodes, got {debug}"
    );
    assert!(
        debug.contains("Equal(\n                    1,")
            && debug.contains("Equal(\n                    2,")
            && debug.contains("Equal(\n                    3,"),
        "expected mana values 1, 2, and 3 to be split into ordered choices, got {debug}"
    );
}

#[test]
fn parse_choose_then_do_same_for_filter_building_blocks_match() {
    let head = lex_line(
        "choose a creature card with mana value 1 in your graveyard",
        0,
    )
    .expect("head clause should lex");
    let head_parsed =
        super::parse_you_choose_objects_clause(&head).expect("head choose helper should not error");
    assert!(
        head_parsed.is_some(),
        "expected head choose helper to match"
    );

    let tail =
        lex_line("creature cards with mana value 2 and 3", 0).expect("tail filter should lex");
    let tail_filter = super::parse_object_filter(&tail, false).expect("tail filter should parse");
    assert!(
        tail_filter.zone == Some(crate::zone::Zone::Battlefield)
            && tail_filter.owner.is_none()
            && tail_filter.controller.is_none(),
        "expected followup filter to stay unowned/uncontrolled and keep the default battlefield zone, got {tail_filter:?}"
    );
    assert!(
        matches!(
            tail_filter.mana_value,
            Some(crate::filter::Comparison::OneOf(ref values)) if values == &[2, 3]
        ),
        "expected followup filter to preserve OneOf(2,3), got {tail_filter:?}"
    );
}

#[test]
fn rewrite_grammar_unique_hand_leader_predicate_parses() {
    let tokens = lex_line("a player has more cards in hand than each other player", 0)
        .expect("rewrite lexer should classify unique hand-leader predicate");

    assert_eq!(
        super::parse_predicate_lexed(&tokens).expect("predicate should parse"),
        crate::cards::builders::PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer {
            player: crate::cards::builders::PlayerAst::Any,
        }
    );
}

#[test]
fn rewrite_grammar_unique_life_leader_predicate_parses() {
    let tokens = lex_line("a player has more life than each other player", 0)
        .expect("rewrite lexer should classify unique life-leader predicate");

    assert_eq!(
        super::parse_predicate_lexed(&tokens).expect("predicate should parse"),
        crate::cards::builders::PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
            player: crate::cards::builders::PlayerAst::Any,
        }
    );
}

#[test]
fn rewrite_grammar_permanent_you_controlled_left_battlefield_predicate_parses() {
    let tokens = lex_line(
        "a permanent you controlled left the battlefield this turn",
        0,
    )
    .expect("rewrite lexer should classify revolt-style permanent-left predicate");

    assert_eq!(
        super::parse_predicate_lexed(&tokens).expect("predicate should parse"),
        crate::cards::builders::PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn
    );
}

#[test]
fn rewrite_parse_subject_player_with_most_cards_in_hand() {
    let tokens = lex_line("the player who has the most cards in hand", 0)
        .expect("rewrite lexer should classify most-cards subject");

    assert_eq!(
        super::util::parse_subject(&tokens),
        super::SubjectAst::Player(crate::cards::builders::PlayerAst::MostCardsInHand)
    );
}

#[test]
fn rewrite_parse_subject_with_most_life() {
    let tokens = lex_line("the player with the most life", 0)
        .expect("rewrite lexer should classify most-life subject");

    assert_eq!(
        super::util::parse_subject(&tokens),
        super::SubjectAst::Player(crate::cards::builders::PlayerAst::MostLifeTied)
    );
}

#[test]
fn rewrite_lexed_triggered_line_keeps_unique_life_leader_intervening_if() {
    let text = "At the beginning of your upkeep, if a player has more life than each other player, the player with the most life gains control of this creature.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify upkeep intervening-if");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("triggered intervening-if line should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BeginningOfUpkeep"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(
        debug.contains("PlayerHasMoreLifeThanEachOtherPlayer"),
        "{debug}"
    );
    assert!(debug.contains("MostLifeTied"), "{debug}");
}
