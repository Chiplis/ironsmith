#![allow(unused_imports)]
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::*;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;
#[cfg(test)]
use ironsmith_compiler_lowering::CardDefinitionBuilder;
use ironsmith_core::card::CardBuilder;

#[test]
pub(super) fn parser_sentence_helpers_do_not_use_retired_fixed_helper_tags() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let helper_paths = [
        "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_entry.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/mod.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/sentence_shape_predicates.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/labeled_prefixes.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/copy_and_next_spell_shapes.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/unsupported_shape_diagnostics.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/search_library.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/mod.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/choice_damage_family.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/registry.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/counter_marker_family.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/token_copy_control_family.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/combat_and_damage_family.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/delayed_step_family.rs",
        "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/mechanic_marker_family.rs",
    ];

    for relative_path in helper_paths {
        let path = workspace_root.join(relative_path);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        for retired_tag in [
            "\"exiled_0\"",
            "\"looked_0\"",
            "\"chosen_0\"",
            "\"revealed_0\"",
        ] {
            assert!(
                !source.contains(retired_tag),
                "retired fixed helper tag {retired_tag} should not appear in {}",
                path.display()
            );
        }
    }
}

#[test]
pub(super) fn grammar_facade_non_test_crate_reexports_stay_minimal() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let mod_rs_path = workspace_root.join("crates/ironsmith-compiler-grammar/src/lib.rs");
    let mod_rs = fs::read_to_string(&mod_rs_path).unwrap_or_else(|err| {
        panic!(
            "grammar lib.rs should be readable at {}: {err}",
            mod_rs_path.display()
        )
    });
    let allowed: [&str; 0] = [];

    let mut non_test_reexports = Vec::new();
    let mut prev_cfg_test = false;
    let mut current_reexport = None::<String>;
    for line in mod_rs.lines() {
        let trimmed = line.trim();
        if let Some(current) = current_reexport.as_mut() {
            if !trimmed.is_empty() {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(trimmed);
            }
            if trimmed.ends_with(';') {
                if !prev_cfg_test {
                    non_test_reexports
                        .push(current.split_whitespace().collect::<Vec<_>>().join(" "));
                }
                current_reexport = None;
                prev_cfg_test = false;
            }
            continue;
        }
        if trimmed == "#[cfg(test)]" {
            prev_cfg_test = true;
            continue;
        }
        if trimmed.starts_with("pub(crate) use ") {
            current_reexport = Some(trimmed.to_string());
            if trimmed.ends_with(';') {
                if !prev_cfg_test {
                    non_test_reexports
                        .push(trimmed.split_whitespace().collect::<Vec<_>>().join(" "));
                }
                current_reexport = None;
                prev_cfg_test = false;
            }
            continue;
        }
        if !trimmed.is_empty() {
            prev_cfg_test = false;
        }
    }

    assert_eq!(
        non_test_reexports, allowed,
        "non-test parser reexports changed; prefer importing concrete modules directly"
    );
}

#[test]
pub(super) fn rewrite_lexer_tracks_spans_for_activation_lines() {
    let tokens = lex_line("{T}, Sacrifice a creature: Add {B}{B}.", 3)
        .expect("rewrite lexer should classify activation line");
    assert_eq!(tokens[0].slice, "{T}");
    assert_eq!(tokens[0].span.line, 3);
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 3);
    assert!(tokens.iter().any(|token| token.slice == ":"));
}

#[test]
pub(super) fn rewrite_lexer_accepts_plus_prefixed_counter_words() {
    let tokens = lex_line("Put a +1/+1 counter on target creature.", 0)
        .expect("rewrite lexer should accept +1/+1 words");
    assert!(tokens.iter().any(|token| token.slice == "+1/+1"));
}

#[test]
pub(super) fn rewrite_lexer_keeps_signed_counters_and_attached_word_punctuation_atomic() {
    let tokens = lex_line(
        "Return those creatures to their owners' hands and give them -1/-1 until end-of-turn.",
        0,
    )
    .expect("rewrite lexer should keep attached punctuation inside atomic words");
    let shapes = tokens
        .iter()
        .map(|token| (token.kind, token.slice.as_str()))
        .collect::<Vec<_>>();

    assert!(shapes.contains(&(super::super::lexer::TokenKind::Word, "owners'")));
    assert!(shapes.contains(&(super::super::lexer::TokenKind::Word, "-1/-1")));
    assert!(shapes.contains(&(super::super::lexer::TokenKind::Word, "end-of-turn")));
}

#[test]
pub(super) fn rewrite_lexer_keeps_generic_slash_words_atomic_but_exposes_standalone_apostrophes() {
    let tokens = lex_line("'power/toughness can’t be 0.", 0)
        .expect("rewrite lexer should classify slash words and standalone apostrophes");
    let kinds = tokens
        .iter()
        .map(|token| (token.kind, token.slice.as_str()))
        .collect::<Vec<_>>();

    assert_eq!(kinds[0], (super::super::lexer::TokenKind::Apostrophe, "'"));
    assert_eq!(
        kinds[1],
        (super::super::lexer::TokenKind::Word, "power/toughness")
    );
    assert!(kinds.contains(&(super::super::lexer::TokenKind::Word, "can’t")));
}

#[test]
pub(super) fn rewrite_lexer_keeps_double_slash_words_atomic_for_source_names() {
    let tokens = lex_line("When SP//dr enters, draw a card.", 0)
        .expect("rewrite lexer should classify double-slash source names");
    let kinds = tokens
        .iter()
        .map(|token| (token.kind, token.slice.as_str(), token.parser_text()))
        .collect::<Vec<_>>();

    assert!(kinds.contains(&(super::super::lexer::TokenKind::Word, "SP//dr", "sp//dr")));
}

#[test]
pub(super) fn rewrite_lexer_distinguishes_structural_tokens() {
    let tokens =
        lex_line("(Mode 2) '", 0).expect("rewrite lexer should classify structural tokens");
    let kinds = tokens.iter().map(|token| token.kind).collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            super::super::lexer::TokenKind::LParen,
            super::super::lexer::TokenKind::Word,
            super::super::lexer::TokenKind::Number,
            super::super::lexer::TokenKind::RParen,
            super::super::lexer::TokenKind::Apostrophe,
        ]
    );
}

#[test]
pub(super) fn rewrite_lexer_precomputes_parser_text() {
    let tokens = lex_line("Its controller's face-down creature gets 2.", 0)
        .expect("rewrite lexer should classify parser-text test line");

    assert_eq!(tokens[0].parser_text(), "its");
    assert_eq!(tokens[1].parser_text(), "controller's");
    assert_eq!(tokens[2].parser_text(), "face-down");
    assert_eq!(tokens[5].parser_text(), "2");
}

#[test]
pub(super) fn rewrite_lexer_reports_line_and_span_for_unknown_tokens() {
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
pub(super) fn rewrite_lex_cursor_supports_peek_and_advance() {
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
pub(super) fn rewrite_sentence_splitter_respects_quotes() {
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
pub(super) fn rewrite_structure_sentence_splitter_respects_quotes() {
    let tokens = lex_line("Choose one. \"Draw a card.\" Create a token.", 0)
        .expect("rewrite lexer should classify structural sentence text");
    let sentences = super::super::grammar::structure::split_lexed_sentences(&tokens);
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
pub(super) fn rewrite_winnow_parse_all_reports_precise_token_failures() {
    use super::super::grammar::primitives::{parse_all, phrase};

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
pub(super) fn rewrite_winnow_punctuation_combinators_cover_structural_tokens() {
    use super::super::grammar::primitives::{
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
pub(super) fn rewrite_winnow_boundary_combinators_cover_sentence_and_block_endings() {
    use super::super::grammar::primitives::{
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
pub(super) fn rewrite_winnow_phrase_and_boundary_combinators_cover_quote_and_parenthesis_edges() {
    use super::super::grammar::primitives::{
        end_of_block, lparen, parse_all, phrase, quote, rparen,
    };

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
pub(super) fn rewrite_winnow_separator_slice_helpers_split_keyword_lists() {
    use super::super::grammar::primitives::{
        split_lexed_slices_on_and, split_lexed_slices_on_comma,
        split_lexed_slices_on_commas_or_semicolons, split_lexed_slices_on_or,
        split_lexed_slices_on_period,
    };

    let separated = lex_line("Flying, vigilance; trample", 0)
        .expect("rewrite lexer should classify comma and semicolon separators");
    let separated_words: Vec<Vec<&str>> = split_lexed_slices_on_commas_or_semicolons(&separated)
        .into_iter()
        .map(super::super::token_word_refs)
        .collect();
    assert_eq!(
        separated_words,
        vec![vec!["Flying"], vec!["vigilance"], vec!["trample"],]
    );

    let compound = lex_line("Protection from blue and from black", 0)
        .expect("rewrite lexer should classify keyword conjunction");
    let compound_words: Vec<Vec<&str>> = split_lexed_slices_on_and(&compound)
        .into_iter()
        .map(super::super::token_word_refs)
        .collect();
    assert_eq!(
        compound_words,
        vec![vec!["Protection", "from", "blue"], vec!["from", "black"],]
    );

    let disjunction = lex_line("Aura, Equipment, or Vehicle", 0)
        .expect("rewrite lexer should classify disjunction separators");
    let disjunction_words: Vec<Vec<&str>> = split_lexed_slices_on_or(&disjunction)
        .into_iter()
        .map(super::super::token_word_refs)
        .collect();
    assert_eq!(
        disjunction_words,
        vec![vec!["Aura"], vec!["Equipment"], vec!["Vehicle"],]
    );

    let comparison = lex_line("mana value 3 or less", 0)
        .expect("rewrite lexer should classify comparison or delimiter");
    let comparison_words: Vec<Vec<&str>> = split_lexed_slices_on_or(&comparison)
        .into_iter()
        .map(super::super::token_word_refs)
        .collect();
    assert_eq!(
        comparison_words,
        vec![vec!["mana", "value", "3", "or", "less"],]
    );

    let comparison_equal = lex_line("mana value less than or equal to 3", 0)
        .expect("rewrite lexer should classify comparison or-equal phrase");
    let comparison_equal_words: Vec<Vec<&str>> = split_lexed_slices_on_or(&comparison_equal)
        .into_iter()
        .map(super::super::token_word_refs)
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
        .map(super::super::token_word_refs)
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
        .map(super::super::token_word_refs)
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
        .map(super::super::token_word_refs)
        .collect();
    assert_eq!(repeated_words, vec![vec!["flying"], vec!["vigilance"],]);

    let quoted_period = lex_line("Choose \"one.\" Then choose another.", 0)
        .expect("rewrite lexer should classify quoted period separators");
    let quoted_period_words: Vec<Vec<&str>> = split_lexed_slices_on_period(&quoted_period)
        .into_iter()
        .map(super::super::token_word_refs)
        .collect();
    assert_eq!(
        quoted_period_words,
        vec![vec!["Choose", "one", "Then", "choose", "another"],]
    );
}

#[test]
pub(super) fn rewrite_winnow_search_helpers_scan_anywhere_in_token_stream() {
    use super::super::grammar::primitives::{
        contains_word, find_phrase_start, has_any_phrase, has_phrase,
    };

    let tokens = lex_line("Draw a card, then discard a card.", 0)
        .expect("rewrite lexer should classify comma-then sentence");

    assert!(contains_word(&tokens, "discard"));
    assert!(has_phrase(&tokens, &["discard", "a", "card"]));
    assert!(has_any_phrase(
        &tokens,
        &[&["mill", "a", "card"], &["discard", "a", "card"]]
    ));
    assert_eq!(
        find_phrase_start(&tokens, &["discard", "a", "card"]),
        Some(5)
    );
}

#[test]
pub(super) fn rewrite_winnow_suffix_slice_helpers_strip_trigger_suffixes() {
    use super::super::grammar::primitives::strip_lexed_suffix_phrases;

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
        super::super::token_word_refs(head),
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
        super::super::token_word_refs(head),
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
pub(super) fn rewrite_winnow_prefix_slice_helper_strips_turn_duration_phrase() {
    use super::super::grammar::primitives::strip_lexed_prefix_phrase;

    let prefixed = lex_line("Until the end of your next turn, you may play that card", 0)
        .expect("rewrite lexer should classify prefixed duration phrase");
    let rest = strip_lexed_prefix_phrase(
        &prefixed,
        &["until", "the", "end", "of", "your", "next", "turn"],
    )
    .expect("expected grammar prefix helper to strip turn-duration phrase");

    assert_eq!(
        super::super::token_word_refs(rest),
        vec!["you", "may", "play", "that", "card"]
    );
}

#[test]
pub(super) fn rewrite_winnow_span_helper_tracks_token_subslice_offsets() {
    let tokens = lex_line("Draw a card, then draw another.", 2)
        .expect("rewrite lexer should classify comma-delimited sentence");
    let (head, rest) = super::super::grammar::primitives::split_lexed_once_on_comma(&tokens)
        .expect("expected grammar split helper to find comma separator");
    let span =
        crate::util::span_from_tokens(head).expect("expected span helper to cover token slice");

    assert_eq!(render_token_slice(head), "Draw a card");
    assert_eq!(
        super::super::token_word_refs(rest),
        vec!["then", "draw", "another"]
    );
    assert_eq!(span.line, 2);
    assert_eq!(span.start, head.first().expect("head token").span().start);
    assert_eq!(span.end, head.last().expect("head token").span().end);
}

#[test]
pub(super) fn rewrite_structure_metadata_line_parser_recognizes_supported_labels() {
    let mana_tokens = lex_line("Mana Cost: {2}{W}", 0)
        .expect("rewrite lexer should classify mana-cost metadata line");
    let mana_spec = super::super::grammar::structure::split_metadata_line_lexed(&mana_tokens)
        .expect("structure metadata helper should recognize mana-cost label");
    assert_eq!(
        mana_spec.kind,
        super::super::grammar::structure::MetadataLineKind::ManaCost
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
    let type_spec = super::super::grammar::structure::split_metadata_line_lexed(&type_tokens)
        .expect("structure metadata helper should recognize type label");
    assert_eq!(
        type_spec.kind,
        super::super::grammar::structure::MetadataLineKind::TypeLine
    );
    assert_eq!(
        super::super::token_word_refs(type_spec.value_tokens),
        vec!["Legendary", "Creature", "Human"]
    );
}

#[test]
pub(super) fn rewrite_structure_untap_all_other_players_untap_step_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Untap all permanents you control during each other player's untap step.",
        0,
    )
    .expect("rewrite lexer should classify untap-all other-players untap-step line");
    assert_eq!(
        super::super::grammar::structure::classify_static_line_family_lexed(&tokens),
        Some(super::super::grammar::structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep)
    );
}

#[test]
pub(super) fn rewrite_structure_untap_singular_other_players_untap_step_shape_parser_recognizes_line()
 {
    let tokens = lex_line(
        "Untap this artifact during each other player's untap step.",
        0,
    )
    .expect("rewrite lexer should classify singular untap other-players untap-step line");
    assert_eq!(
        super::super::grammar::structure::classify_static_line_family_lexed(&tokens),
        Some(super::super::grammar::structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep)
    );
}

#[test]
pub(super) fn rewrite_structure_next_turn_cast_lock_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Each opponent can't cast instant or sorcery spells during that player's next turn.",
        0,
    )
    .expect("rewrite lexer should classify next-turn cast-lock line");
    assert_eq!(
        super::super::grammar::structure::classify_statement_line_family_lexed(&tokens),
        Some(super::super::grammar::structure::StatementLineFamily::NextTurnCantCast)
    );
}

#[test]
pub(super) fn rewrite_structure_divvy_statement_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Separate all creatures target player controls into two piles. Destroy all creatures in the pile of your choice.",
        0,
    )
    .expect("rewrite lexer should classify divvy pile line");
    assert_eq!(
        super::super::grammar::structure::classify_statement_line_family_lexed(&tokens),
        Some(super::super::grammar::structure::StatementLineFamily::Divvy)
    );
}

#[test]
pub(super) fn rewrite_structure_vote_statement_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Starting with you, each player votes for death or torture.",
        0,
    )
    .expect("rewrite lexer should classify vote statement line");
    assert_eq!(
        super::super::grammar::structure::classify_statement_line_family_lexed(&tokens),
        Some(super::super::grammar::structure::StatementLineFamily::Vote)
    );
}

#[test]
pub(super) fn rewrite_structure_art_rating_statement_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Ask a person outside the game to rate its new art on a scale from 1 to 5.",
        0,
    )
    .expect("rewrite lexer should classify art-rating statement line");
    assert_eq!(
        super::super::grammar::structure::classify_statement_line_family_lexed(&tokens),
        Some(super::super::grammar::structure::StatementLineFamily::ArtRating)
    );
}

#[test]
pub(super) fn rewrite_structure_exile_play_costs_more_statement_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs 2 more to cast.",
        0,
    )
    .expect("rewrite lexer should classify exile-play-costs-more statement line");
    assert_eq!(
        super::super::grammar::structure::classify_statement_line_family_lexed(&tokens),
        Some(super::super::grammar::structure::StatementLineFamily::ExilePlayCostsMore)
    );
}

#[test]
pub(super) fn rewrite_structure_generic_statement_shape_parser_recognizes_heads() {
    for text in [
        "Draw a card.",
        "Each player discards a card.",
        "That target player sacrifices a creature.",
        "This spell deals 3 damage to any target.",
        "Target creature gets +2/+2 until end of turn.",
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify generic statement-head line");
        assert_eq!(
            super::super::grammar::structure::classify_statement_line_family_lexed(&tokens),
            Some(super::super::grammar::structure::StatementLineFamily::Generic)
        );
    }
}

#[test]
pub(super) fn rewrite_structure_generic_static_shape_parser_recognizes_heads() {
    for text in [
        "This creature has flying.",
        "Enchanted creature gets +1/+1.",
        "As long as you control an artifact, this creature has hexproof.",
        "Your maximum hand size is reduced by four.",
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify generic static-head line");
        assert_eq!(
            super::super::grammar::structure::classify_static_line_family_lexed(&tokens),
            Some(super::super::grammar::structure::StaticLineFamily::Generic)
        );
    }
}

#[test]
pub(super) fn rewrite_structure_granted_quoted_static_shape_parser_recognizes_line() {
    let tokens = lex_line(
        "It has \"When this token dies, it deals 1 damage to any target.\"",
        0,
    )
    .expect("rewrite lexer should classify granted quoted static line");
    assert_eq!(
        super::super::grammar::structure::classify_static_line_family_lexed(&tokens),
        Some(super::super::grammar::structure::StaticLineFamily::GrantedQuotedAbility)
    );
}

#[test]
pub(super) fn rewrite_sentence_splitter_ignores_single_quotes_inside_double_quotes() {
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
pub(super) fn rewrite_structure_sentence_splitter_keeps_unterminated_tail_segment() {
    let tokens = lex_line("Draw a card. Exile target creature", 0)
        .expect("rewrite lexer should classify unterminated tail");
    let sentences = super::super::grammar::structure::split_lexed_sentences(&tokens);
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
pub(super) fn rewrite_structure_sentence_splitter_separates_broken_visage_followups() {
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
pub(super) fn rewrite_effect_sentence_parser_handles_broken_visage_sequence() {
    let tokens = lex_line(
        "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        0,
    )
    .expect("rewrite lexer should classify Broken Visage text");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&tokens);
    assert!(
        parsed.is_ok(),
        "Broken Visage effect sentences should parse directly, got {parsed:?}"
    );
    let effects = parsed.expect("Broken Visage effects");
    let Some(EffectAst::SubjectVerb(subject_verb)) = effects.last() else {
        panic!("expected final token creation effect, got {effects:#?}");
    };
    let crate::model::ast::SubjectVerbActionAst::CreateTokenWithMods {
        dynamic_power_toughness,
        sacrifice_at_next_end_step,
        ..
    } = &subject_verb.action
    else {
        panic!("expected typed token creation action, got {subject_verb:#?}");
    };
    assert!(matches!(
        dynamic_power_toughness,
        Some((Value::PowerOf(_), Value::ToughnessOf(_)))
    ));
    assert!(*sacrifice_at_next_end_step);
}

#[test]
pub(super) fn rewrite_effect_sentence_parser_merges_quoted_token_rule_reminder() {
    let tokens = lex_line(
        "Create a 1/1 red Devil creature token. It has \"When this token dies, it deals 1 damage to any target.\"",
        0,
    )
    .expect("rewrite lexer should classify standalone token rule reminder");
    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("standalone token rule reminder should merge into the create effect");

    let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
        panic!("expected one token creation effect, got {effects:#?}");
    };
    let crate::model::ast::SubjectVerbActionAst::CreateTokenWithMods { definition, .. } =
        &subject_verb.action
    else {
        panic!("expected typed token creation action, got {subject_verb:#?}");
    };
    let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) = definition else {
        panic!("expected a typed creature token definition, got {definition:#?}");
    };
    assert_eq!(creature.rules.dies_damage_any_target, Some(1));
}

#[test]
pub(super) fn rewrite_effect_sentence_parser_keeps_create_around_quoted_token_trigger() {
    let tokens = lex_line(
        "Create two 0/2 blue Illusion creature tokens with \"Whenever this token blocks a creature, that creature doesn't untap during its controller's next untap step.\"",
        0,
    )
    .expect("quoted token trigger should lex");
    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("quoted token trigger should attach to its create effect");

    let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
        panic!("expected one token creation effect, got {effects:#?}");
    };
    let crate::model::ast::SubjectVerbActionAst::CreateTokenWithMods {
        count,
        granted_abilities,
        ..
    } = &subject_verb.action
    else {
        panic!("expected typed token creation action, got {subject_verb:#?}");
    };
    assert_eq!(*count, Value::Fixed(2));

    let debug = format!("{granted_abilities:#?}");
    assert!(debug.contains("ThisBlocksObject"), "{debug}");
    assert!(debug.contains("Cant"), "{debug}");
    assert!(debug.contains("ControllersNextUntapStep"), "{debug}");
}

#[test]
pub(super) fn rewrite_cant_be_regenerated_followup_detector_matches_plain_it_clause() {
    let tokens = lex_line("It can't be regenerated.", 0)
        .expect("rewrite lexer should classify can't-be-regenerated followup");
    assert!(
        super::super::effect_sentences::is_cant_be_regenerated_followup_sentence(&tokens),
        "expected plain can't-be-regenerated sentence to be recognized as followup"
    );
}

#[test]
pub(super) fn rewrite_semantic_parse_handles_broken_visage_statement() -> Result<(), CardTextError>
{
    let card = CardBuilder::new(CardId::new(), "Broken Visage Variant")
        .card_types(vec![CardType::Instant]);
    let (doc, _) = parse_text_to_semantic_document(
        card,
        "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.".to_string(),
        false,
    )?;

    assert!(
        matches!(doc.items.as_slice(), [RewriteSemanticItem::ParsedLine(_)]),
        "expected Broken Visage to remain a statement line, got {:#?}",
        doc.items
    );

    Ok(())
}

#[test]
pub(super) fn rewrite_structure_modal_header_flag_scan_tracks_commander_and_repeat_modes() {
    let tokens = lex_line(
        "Choose one. If you control a commander as you cast this spell, you may choose both instead. You may choose the same mode more than once",
        0,
    )
    .expect("rewrite lexer should classify modal flag line");
    let flags = super::super::grammar::structure::scan_modal_header_flags(&tokens);

    assert!(flags.commander_allows_both, "{flags:?}");
    assert!(flags.choose_both_control_card_types.is_empty(), "{flags:?}");
    assert!(flags.same_mode_more_than_once, "{flags:?}");
    assert!(!flags.mode_must_be_unchosen, "{flags:?}");
    assert!(!flags.mode_must_be_unchosen_this_turn, "{flags:?}");
}

#[test]
pub(super) fn rewrite_structure_modal_header_flag_scan_tracks_choose_both_control_card_types() {
    let tokens = lex_line(
        "Choose one. If you control an artifact and an enchantment as you cast this spell, you may choose both instead.",
        0,
    )
    .expect("rewrite lexer should classify choose-both control card types line");
    let flags = super::super::grammar::structure::scan_modal_header_flags(&tokens);

    assert!(!flags.commander_allows_both, "{flags:?}");
    assert_eq!(
        flags.choose_both_control_card_types,
        vec![CardType::Artifact, CardType::Enchantment],
        "{flags:?}"
    );
}

#[test]
pub(super) fn rewrite_structure_modal_header_flag_scan_tracks_choose_both_exact_life_total() {
    let tokens = lex_line(
        "Choose one. If you have exactly 13 life, you may choose both instead.",
        0,
    )
    .expect("rewrite lexer should classify exact-life choose-both line");
    let flags = super::super::grammar::structure::scan_modal_header_flags(&tokens);

    assert!(!flags.commander_allows_both, "{flags:?}");
    assert!(flags.choose_both_control_card_types.is_empty(), "{flags:?}");
    assert_eq!(flags.choose_both_exact_life_total, Some(13), "{flags:?}");
}

#[test]
pub(super) fn rewrite_structure_modal_gate_scan_marks_remove_mode_only_without_word_view() {
    let tokens = lex_line(
        "Remove a +1/+1 counter from this creature. If you removed it this way,",
        0,
    )
    .expect("rewrite lexer should classify trailing modal gate line");
    let gate = super::super::grammar::structure::split_trailing_modal_gate_clause(&tokens)
        .expect("structure helper should detect trailing modal gate");

    assert!(gate.remove_mode_only, "{gate:?}");
    assert!(!gate.reflexive, "{gate:?}");
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
pub(super) fn rewrite_structure_if_result_predicate_parser_preserves_contractions() {
    let didnt_tokens = lex_line("you don't", 0).expect("rewrite lexer should classify predicate");
    let dies_tokens = lex_line("that creature dies this way", 0)
        .expect("rewrite lexer should classify dies-this-way predicate");

    assert_eq!(
        super::super::grammar::structure::parse_if_result_predicate(&didnt_tokens),
        Some(crate::cards::builders::IfResultPredicate::DidNot)
    );
    assert_eq!(
        super::super::grammar::structure::parse_if_result_predicate(&dies_tokens),
        Some(crate::cards::builders::IfResultPredicate::DiesThisWay)
    );
}

#[test]
pub(super) fn rewrite_structure_if_result_predicate_parser_keeps_coin_flip_outcomes() {
    let win_tokens =
        lex_line("you win the flip", 0).expect("rewrite lexer should classify win-the-flip text");
    let lose_tokens =
        lex_line("you lose the flip", 0).expect("rewrite lexer should classify lose-the-flip text");

    assert_eq!(
        super::super::grammar::structure::parse_if_result_predicate(&win_tokens),
        Some(crate::cards::builders::IfResultPredicate::Did)
    );
    assert_eq!(
        super::super::grammar::structure::parse_if_result_predicate(&lose_tokens),
        Some(crate::cards::builders::IfResultPredicate::DidNot)
    );
}

#[test]
pub(super) fn rewrite_structure_leading_result_prefix_parser_splits_when_prefix() {
    let tokens = lex_line("When you do, draw a card.", 0)
        .expect("rewrite lexer should classify leading result prefix sentence");
    let prefix = super::super::grammar::structure::split_leading_result_prefix_lexed(&tokens)
        .expect("structure helper should detect leading result prefix");

    assert_eq!(
        prefix.kind,
        super::super::grammar::structure::LeadingResultPrefixKind::When
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
pub(super) fn rewrite_structure_leading_result_prefix_parser_splits_numeric_ranges() {
    let tokens = lex_line("1—9 | You may put that card on top of your library.", 0)
        .expect("rewrite lexer should classify numeric result prefix sentence");
    let prefix = super::super::grammar::structure::split_leading_result_prefix_lexed(&tokens)
        .expect("structure helper should detect numeric result prefix");

    assert_eq!(
        prefix.kind,
        super::super::grammar::structure::LeadingResultPrefixKind::If
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

    let compact_ascii_tokens = lex_line("10-19 | You draw a card.", 0)
        .expect("rewrite lexer should classify compact ASCII numeric result prefix");
    let compact_ascii_prefix =
        super::super::grammar::structure::split_leading_result_prefix_lexed(&compact_ascii_tokens)
            .expect("structure helper should detect compact ASCII numeric result prefix");
    assert_eq!(
        compact_ascii_prefix.predicate,
        crate::cards::builders::IfResultPredicate::Value(
            crate::effect::Comparison::BetweenInclusive(10, 19)
        )
    );
    assert_eq!(
        render_token_slice(compact_ascii_prefix.trailing_tokens),
        "You draw a card."
    );
}

#[test]
pub(super) fn rewrite_structure_trailing_if_clause_parser_splits_destroy_clause() {
    let tokens = lex_line("Destroy target creature if it's white", 0)
        .expect("rewrite lexer should classify trailing-if clause");
    let spec = super::super::grammar::structure::split_trailing_if_clause_lexed(&tokens)
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
pub(super) fn rewrite_structure_if_clause_splitter_routes_commaless_conditional_sentence() {
    let tokens = lex_line(
        "If at least three blue mana was spent to cast this spell create a Food token.",
        0,
    )
    .expect("rewrite lexer should classify comma-less if clause");
    let spec = super::super::grammar::structure::split_if_clause_lexed(
        &tokens,
        super::super::effect_sentences::parse_effect_chain_lexed,
    )
    .expect("structure helper should split comma-less if clause");

    match spec.predicate {
        super::super::grammar::structure::IfClausePredicateSpec::Conditional(_) => {}
        other => panic!("expected conditional predicate split, got {other:?}"),
    }
    assert!(matches!(
        spec.effects.as_slice(),
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::CreateTokenWithMods { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_structure_if_clause_splitter_routes_life_tie_choice_sequence() {
    let tokens = lex_line(
        "If two or more players are tied for lowest life total, you choose one of them, and that player gains control of this creature.",
        0,
    )
    .expect("rewrite lexer should classify life-tie choice clause");
    let spec = super::super::grammar::structure::split_if_clause_lexed(
        &tokens,
        super::super::effect_sentences::parse_effect_chain_lexed,
    )
    .expect("structure helper should split life-tie choice clause at its first comma");

    assert!(matches!(
        spec.predicate,
        super::super::grammar::structure::IfClausePredicateSpec::Conditional(
            crate::cards::builders::PredicateAst::ValueComparison { .. }
        )
    ));
    assert!(!spec.effects.is_empty());
}

#[test]
pub(super) fn rewrite_structure_if_clause_splitter_keeps_player_may_search_subject() {
    let tokens = lex_line(
        "If a land was destroyed this way its controller may search their library for a basic land card.",
        0,
    )
    .expect("rewrite lexer should classify commaless controller may-search if clause");
    let spec = super::super::grammar::structure::split_if_clause_lexed(
        &tokens,
        super::super::effect_sentences::parse_effect_chain_lexed,
    )
    .expect("structure helper should split controller may-search if clause");

    match spec.predicate {
        super::super::grammar::structure::IfClausePredicateSpec::Conditional(
            crate::cards::builders::PredicateAst::TaggedMatches(_, filter),
        ) => {
            assert!(
                filter.card_types.contains(&CardType::Land),
                "expected land-destroyed predicate filter, got {filter:?}"
            );
        }
        other => panic!("expected destroyed-land conditional predicate, got {other:?}"),
    }
    assert!(
        matches!(
            spec.effects.as_slice(),
            [crate::cards::builders::EffectAst::MayByPlayer {
                player: crate::cards::builders::PlayerAst::ItsController,
                ..
            }]
        ),
        "expected full 'its controller may search' effect subject, got {:?}",
        spec.effects
    );
}

#[test]
pub(super) fn rewrite_structure_predicate_parse_entrypoint_matches_parser_root_output() {
    let text = "it's your turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");

    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse");
    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("parser-root predicate entrypoint should parse");

    assert_eq!(grammar, parser_root);
}

#[test]
pub(super) fn rewrite_structure_predicate_parse_entrypoint_parses_not_your_turn() {
    let text = "it's not your turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");

    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse");
    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("parser-root predicate entrypoint should parse");
    let debug = format!("{grammar:?}");

    assert_eq!(grammar, parser_root);
    assert!(
        debug.contains("Not") && debug.contains("YourTurn"),
        "expected negated your-turn predicate AST, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_structure_predicate_parse_entrypoint_matches_parser_root_output_for_conjoined_predicate()
 {
    let text = "it's your turn and you have no cards in hand";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");

    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse");
    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("parser-root predicate entrypoint should parse");
    let debug = format!("{grammar:?}");

    assert_eq!(grammar, parser_root);
    assert!(
        debug.contains("And("),
        "expected conjoined predicate AST, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_structure_predicate_parses_you_have_one_or_fewer_cards_in_hand() {
    let text = "you have one or fewer cards in hand";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");

    let predicate =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("predicate should parse for you-have subject");
    let debug = format!("{predicate:?}");

    assert!(
        debug.contains("PlayerCardsInHandOrFewer"),
        "expected cards-in-hand threshold predicate, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_structure_if_tail_parser_extracts_predicate() {
    let tokens = lex_line("if it's white", 0).expect("rewrite lexer should classify if tail");
    let predicate = super::super::grammar::structure::parse_trailing_if_predicate_lexed(&tokens)
        .expect("structure helper should parse if tail predicate");
    let expected =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens[1..])
            .expect("tail predicate should still parse");

    assert_eq!(predicate, expected);
}

#[test]
pub(super) fn rewrite_structure_trailing_unless_clause_parser_splits_gain_control_clause() {
    let tokens = lex_line("target creature unless you control an artifact", 0)
        .expect("rewrite lexer should classify trailing-unless clause");
    let spec = super::super::grammar::structure::split_trailing_unless_clause_lexed(&tokens)
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
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&expected_tokens)
            .expect("expected predicate should parse");

    assert_eq!(spec.predicate, expected);
}

#[test]
pub(super) fn rewrite_structure_who_player_predicate_parser_extracts_prefixed_player_predicate() {
    let tokens = lex_line("who controls an artifact", 0)
        .expect("rewrite lexer should classify who-player predicate tail");
    let predicate = super::super::grammar::structure::parse_who_player_predicate_lexed(&tokens)
        .expect("structure helper should parse who-player predicate");
    let expected_tokens =
        lex_line("that player controls an artifact", 0).expect("expected predicate should lex");
    let expected =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&expected_tokens)
            .expect("expected predicate should parse");

    assert_eq!(predicate, expected);
}

#[test]
pub(super) fn rewrite_structure_instead_if_tail_parser_extracts_predicate() {
    let tokens = lex_line(
        "instead if there are seven or more cards in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify instead-if tail");
    let predicate =
        super::super::grammar::structure::parse_trailing_instead_if_predicate_lexed(&tokens)
            .expect("structure helper should parse instead-if tail predicate");
    let expected =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens[2..])
            .expect("tail predicate should still parse");

    assert_eq!(predicate, expected);
}

#[test]
pub(super) fn rewrite_structure_conditional_predicate_tail_parser_splits_instead_if_branch() {
    let tokens = lex_line("it's white instead if you control an artifact instead", 0)
        .expect("rewrite lexer should classify nested conditional predicate tail");
    let spec = super::super::grammar::structure::parse_conditional_predicate_tail_lexed(&tokens)
        .expect("structure helper should parse conditional predicate tail");
    let expected_base_tokens = lex_line("it's white", 0).expect("base predicate should lex");
    let expected_outer_tokens =
        lex_line("you control an artifact", 0).expect("outer predicate should lex");
    let expected_base = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
        &expected_base_tokens,
    )
    .expect("base predicate should parse");
    let expected_outer = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
        &expected_outer_tokens,
    )
    .expect("outer predicate should parse");

    assert_eq!(
        spec,
        super::super::grammar::structure::ConditionalPredicateTailSpec::InsteadIf {
            base_predicate: expected_base,
            outer_predicate: expected_outer,
        }
    );
}

#[test]
pub(super) fn rewrite_structure_triggered_conditional_clause_parser_splits_intervening_if() {
    let tokens = lex_line(
        "At the beginning of your upkeep, if you control an artifact, draw a card.",
        0,
    )
    .expect("rewrite lexer should classify triggered conditional line");
    let spec =
        super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
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
pub(super) fn rewrite_structure_attack_while_most_life_keeps_typed_condition() {
    let tokens = lex_line(
        "this creature attacks while you have the most life or are tied for most life",
        0,
    )
    .expect("most-life attack condition should lex");
    let spec = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("the authored while condition must be preserved at event time");
    assert!(matches!(
        spec,
        super::super::model::ast::TriggerSpec::ConditionQualified {
            trigger,
            condition:
                super::super::model::ast::PredicateAst::PlayerHasNoOpponentWithMoreLifeThan {
                    player: crate::PlayerAst::You,
                },
            ..
        } if matches!(*trigger, super::super::model::ast::TriggerSpec::ThisAttacks)
    ));
}

#[test]
pub(super) fn rewrite_structure_cast_source_intervening_if_keeps_multi_sentence_effect_body() {
    let tokens = lex_line(
        "When this creature enters, if you cast it, counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard, then you may cast it without paying its mana cost.",
        0,
    )
    .expect("cast-source conditional trigger should lex");
    let spec =
        super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
            .expect("cast-source conditional trigger should split");

    assert_eq!(
        spec.predicate,
        super::super::model::ast::PredicateAst::SourceWasCast
    );
    assert!(
        spec.effects_tokens
            .iter()
            .filter_map(|token| token.as_word())
            .collect::<Vec<_>>()
            .starts_with(&["counter", "target", "spell"])
    );
}

#[test]
pub(super) fn rewrite_transcendent_dragon_keeps_cast_gate_counter_exile_and_free_cast() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Transcendent Dragon")
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
            ManaSymbol::Generic(4),
            ManaSymbol::Blue,
            ManaSymbol::Blue,
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .power_toughness(crate::card::PowerToughness::fixed(4, 5))
        .parse_text(
            "Flash\nFlying\nWhen this creature enters, if you cast it, counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard, then you may cast it without paying its mana cost.",
        )
        .expect("Transcendent Dragon should parse without a card-specific definition");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Transcendent Dragon should retain its ETB trigger");
    let debug = format!("{triggered:#?}");

    assert!(debug.contains("SourceWasCast"), "{debug}");
    assert!(debug.contains("Counter"), "{debug}");
    assert!(debug.contains("LocalRewriteEffect"), "{debug}");
    assert!(debug.contains("RegisterZoneReplacementEffect"), "{debug}");
    assert!(
        debug.contains("without_paying_mana_cost: true") || debug.contains("WithoutPayingManaCost"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_structure_conditional_damage_action_is_not_absorbed_into_predicate() {
    let tokens = lex_line(
        "When this creature enters, if an opponent lost life this turn, it deals X damage to up to one target creature or planeswalker, where X is the number of Vampires you control.",
        0,
    )
    .expect("conditional damage trigger should lex");
    let spec =
        super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
            .expect("conditional damage trigger should split");

    assert!(format!("{:?}", spec.predicate).contains("OpponentLostLifeThisTurn"));
    assert_eq!(
        spec.effects_tokens
            .iter()
            .filter_map(|token| token.as_word())
            .collect::<Vec<_>>(),
        vec![
            "it",
            "deals",
            "X",
            "damage",
            "to",
            "up",
            "to",
            "one",
            "target",
            "creature",
            "or",
            "planeswalker",
            "where",
            "X",
            "is",
            "the",
            "number",
            "of",
            "Vampires",
            "you",
            "control",
        ]
    );
}

#[test]
pub(super) fn rewrite_structure_triggered_conditional_clause_parser_keeps_graveyard_count_gate() {
    let tokens = lex_line(
        "At the beginning of your upkeep, if twenty or more creature cards are in your graveyard, you win the game.",
        0,
    )
    .expect("rewrite lexer should classify Mortal Combat conditional trigger");
    let spec =
        super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
            .expect("structure helper should detect Mortal Combat intervening-if trigger");

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
        vec!["you", "win", "the", "game", "."]
    );
    let predicate_debug = format!("{:?}", spec.predicate);
    assert!(
        predicate_debug.contains("ValueComparison"),
        "{predicate_debug}"
    );
    assert!(
        predicate_debug.contains("GreaterThanOrEqual"),
        "{predicate_debug}"
    );
    assert!(predicate_debug.contains("Fixed(20)"), "{predicate_debug}");
    assert!(predicate_debug.contains("Creature"), "{predicate_debug}");
    assert!(predicate_debug.contains("Graveyard"), "{predicate_debug}");
}

#[test]
pub(super) fn rewrite_structure_triggered_conditional_clause_parser_keeps_count_based_battlefield_gate()
 {
    let tokens = lex_line(
        "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature.",
        0,
    )
    .expect("rewrite lexer should classify Portcullis conditional trigger");
    let spec =
        super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
            .expect("structure helper should detect Portcullis conditional trigger");

    assert_eq!(
        spec.trigger_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "creature", "enters"]
    );
    let predicate_debug = format!("{:?}", spec.predicate);
    assert!(
        predicate_debug.contains("ValueComparison"),
        "expected count-based battlefield gate to lower as a value comparison, got {predicate_debug}"
    );
    assert!(
        predicate_debug.contains("Count(") || predicate_debug.contains("CountScaled("),
        "expected battlefield-count gate to reference a count value, got {predicate_debug}"
    );
}

#[test]
pub(super) fn rewrite_structure_triggered_conditional_clause_parser_keeps_source_crew_count_gate() {
    let tokens = lex_line(
        "Whenever this Vehicle becomes crewed for the first time each turn, if it was crewed by exactly two creatures, it gains \"Whenever this creature deals combat damage to a player, draw two cards\" until end of turn.",
        0,
    )
    .expect("rewrite lexer should classify crew-count conditional trigger");
    let spec =
        super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
            .expect("structure helper should detect crew-count conditional trigger");

    assert_eq!(
        spec.trigger_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec![
            "this", "Vehicle", "becomes", "crewed", "for", "the", "first", "time", "each", "turn"
        ]
    );
    let predicate_debug = format!("{:?}", spec.predicate);
    assert!(
        predicate_debug.contains("SourceCrewedByExactly")
            && predicate_debug.contains("count: 2")
            && predicate_debug.contains("Creature"),
        "expected crew-count gate to stay modeled as a source predicate, got {predicate_debug}"
    );
    assert_eq!(
        spec.effects_tokens
            .iter()
            .filter_map(|token| token.as_word())
            .collect::<Vec<_>>(),
        vec![
            "it", "gains", "Whenever", "this", "creature", "deals", "combat", "damage", "to", "a",
            "player", "draw", "two", "cards", "until", "end", "of", "turn"
        ]
    );
}

#[test]
pub(super) fn rewrite_structure_triggered_conditional_clause_parser_keeps_counter_count_gate() {
    let tokens = lex_line(
        "At the beginning of your end step, if two or more permanents you don't control have an aim counter on them, destroy one of those permanents at random.",
        0,
    )
    .expect("rewrite lexer should classify counter-count conditional trigger");
    let spec =
        super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
            .expect("structure helper should detect counter-count conditional trigger");

    assert_eq!(
        spec.trigger_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["the", "beginning", "of", "your", "end", "step"]
    );
    let predicate_debug = format!("{:?}", spec.predicate);
    assert!(
        predicate_debug.contains("ValueComparison"),
        "{predicate_debug}"
    );
    assert!(predicate_debug.contains("Count("), "{predicate_debug}");
    assert!(predicate_debug.contains("Aim"), "{predicate_debug}");
    assert!(predicate_debug.contains("NotYou"), "{predicate_debug}");
}

#[test]
pub(super) fn rewrite_structure_triggered_conditional_clause_parser_splits_happily_ever_after_gate()
{
    let tokens = lex_line(
        "At the beginning of your upkeep, if there are five colors among permanents you control, there are six or more card types among permanents you control and/or cards in your graveyard, and your life total is greater than or equal to your starting life total, you win the game.",
        0,
    )
    .expect("rewrite lexer should classify Happily Ever After conditional trigger");
    let spec =
        super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
            .expect("structure helper should detect Happily Ever After intervening-if trigger");

    assert_eq!(
        spec.effects_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["you", "win", "the", "game", "."]
    );
    let predicate_debug = format!("{:?}", spec.predicate);
    assert!(
        predicate_debug.contains("And(")
            && predicate_debug.contains("ColorsAmong")
            && predicate_debug.contains("CardTypesAmong")
            && predicate_debug.contains("StartingLifeTotal"),
        "expected Happily Ever After's full gate to stay modeled, got {predicate_debug}"
    );
}

#[test]
pub(super) fn rewrite_structure_triggered_conditional_clause_keeps_search_action_after_gate() {
    for (text, predicate_marker, expected_effect_words) in [
        (
            "When this Aura enters, if it was kicked, you may search your library for a card named Gigantiform, put it onto the battlefield, then shuffle.",
            "ThisSpellWasKicked",
            &[
                "you",
                "may",
                "search",
                "your",
                "library",
                "for",
                "a",
                "card",
                "named",
                "Gigantiform",
                "put",
                "it",
                "onto",
                "the",
                "battlefield",
                "then",
                "shuffle",
            ][..],
        ),
        (
            "When this creature dies, if it had no time counters on it, you may search your library for an enchantment card, put it onto the battlefield, then shuffle.",
            "TriggeringObjectHadNoCounter",
            &[
                "you",
                "may",
                "search",
                "your",
                "library",
                "for",
                "an",
                "enchantment",
                "card",
                "put",
                "it",
                "onto",
                "the",
                "battlefield",
                "then",
                "shuffle",
            ][..],
        ),
        (
            "When this creature enters, if it was kicked, search your library for a land card with a basic land type, reveal it, put it into your hand, then shuffle.",
            "ThisSpellWasKicked",
            &[
                "search", "your", "library", "for", "a", "land", "card", "with", "a", "basic",
                "land", "type", "reveal", "it", "put", "it", "into", "your", "hand", "then",
                "shuffle",
            ][..],
        ),
    ] {
        let tokens = lex_line(text, 0).expect("conditional search line should lex");
        let spec =
            super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
                .expect("conditional search line should split at its intervening-if gate");
        let predicate_debug = format!("{:?}", spec.predicate);
        assert!(
            predicate_debug.contains(predicate_marker),
            "expected {predicate_marker} gate, got {predicate_debug}"
        );
        let effect_words = spec
            .effects_tokens
            .iter()
            .filter_map(|token| token.as_word())
            .collect::<Vec<_>>();
        assert_eq!(
            effect_words.as_slice(),
            expected_effect_words,
            "search action must remain wholly inside the effect clause for {text}"
        );
    }
}

#[test]
pub(super) fn rewrite_structure_state_triggered_clause_parser_splits_when_condition() {
    let tokens = lex_line("When you control no Swamps, sacrifice this creature.", 0)
        .expect("rewrite lexer should classify state-trigger line");
    let spec = super::super::grammar::structure::split_state_triggered_clause_lexed(&tokens, 1, 5)
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
pub(super) fn rewrite_structure_state_triggered_clause_parser_splits_state_with_gate() {
    let tokens = lex_line(
        "When an opponent controls a creature with power 4 or greater, if this permanent is an enchantment, it becomes a 4/4 Beast creature.",
        0,
    )
    .expect("rewrite lexer should classify Hidden Predators state-trigger line");
    let spec = super::super::grammar::structure::split_state_triggered_clause_lexed(&tokens, 1, 18)
        .expect("structure helper should detect gated state-trigger clause");

    assert_eq!(
        spec.effects_tokens
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>(),
        vec!["it", "becomes", "a", "4/4", "Beast", "creature", "."]
    );
    let predicate_debug = format!("{:?}", spec.predicate);
    assert!(
        predicate_debug.contains("PlayerControls")
            && predicate_debug.contains("SourceMatches")
            && predicate_debug.contains("Enchantment"),
        "expected opponent-control state and enchantment gate, got {predicate_debug}"
    );
}

#[test]
pub(super) fn rewrite_modal_header_parser_tracks_unchosen_turn_scope() {
    let text = "Whenever another creature you control enters, choose one that hasn't been chosen this turn —";
    let header = parse_modal_header_for_test(text)
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(header.trigger.is_some(), "{header:?}");
    assert!(header.mode_must_be_unchosen, "{header:?}");
    assert!(header.mode_must_be_unchosen_this_turn, "{header:?}");
}

#[test]
pub(super) fn rewrite_modal_header_parser_supports_activated_choose_header_directly() {
    let text = "{T}: Choose one —";
    let header = parse_modal_header_for_test(text)
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(header.activated.is_some(), "{header:?}");
    assert!(header.trigger.is_none(), "{header:?}");
    assert_eq!(header.min, crate::effect::Value::Fixed(1));
    assert_eq!(header.max, Some(crate::effect::Value::Fixed(1)));
}

#[test]
pub(super) fn rewrite_modal_header_parser_accepts_pawprint_worth_clause() {
    let text = "Choose up to five {P} worth of modes. You may choose the same mode more than once.";
    let header = parse_modal_header_for_test(text)
        .expect("Season of the Burrow modal header should parse")
        .expect("Season of the Burrow modal header should be recognized");

    assert_eq!(header.min, crate::effect::Value::Fixed(0));
    assert_eq!(header.max, Some(crate::effect::Value::Fixed(5)));
    assert!(header.weighted_mode_points, "{header:?}");
    assert!(header.same_mode_more_than_once, "{header:?}");
}

#[test]
pub(super) fn rewrite_lowered_pawprint_modal_uses_typed_header_and_mode_costs()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Pawprint Modal Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Choose up to two {P} worth of modes.\n{P} — Draw a card.\n{P} — You gain 1 life."
            .to_string(),
        false,
    )?;
    let choose_mode = definition
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(crate::effect::Effect::as_choose_mode)
        })
        .expect("pawprint modal should lower to a choose-mode effect");

    assert_eq!(choose_mode.mode_point_costs, vec![1, 1]);
    Ok(())
}

#[test]
pub(super) fn rewrite_modal_header_parser_keeps_choose_one_when_later_choose_both_is_present() {
    let text = "Choose one. If you control a commander as you cast this spell, you may choose both instead.";
    let header = parse_modal_header_for_test(text)
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert_eq!(header.min, crate::effect::Value::Fixed(1));
    assert_eq!(header.max, Some(crate::effect::Value::Fixed(1)));
}

#[test]
pub(super) fn refreshed_instead_modal_header_parses_typed_conditional_selection_ranges_and_labels()
{
    use crate::model::compiler_semantic::ConditionalModeSelection;

    for (text, expected, label) in [
        (
            "Choose one. If you descended this turn, you may choose both instead.",
            ConditionalModeSelection::BothOrTwo,
            None,
        ),
        (
            "Delirium — Choose one. If there are four or more card types among cards in your graveyard, choose one or more instead.",
            ConditionalModeSelection::OneOrMore,
            Some("Delirium"),
        ),
        (
            "Choose one. If this spell was kicked, choose any number instead.",
            ConditionalModeSelection::AnyNumber,
            None,
        ),
    ] {
        let header = parse_modal_header_for_test(text)
            .expect("modal header should parse")
            .expect("modal header should be recognized");
        assert_eq!(
            header
                .conditional_mode_change
                .as_ref()
                .map(|change| change.selection),
            Some(expected),
            "{text}: {header:#?}"
        );
        assert_eq!(
            header
                .presentation_label
                .as_ref()
                .and_then(|label| label.display_prefix())
                .as_deref(),
            label,
            "{text}: {header:#?}"
        );
    }
}

#[test]
pub(super) fn refreshed_instead_modal_header_near_misses_do_not_invent_conditional_selection() {
    for text in [
        "Choose one. If you descended this turn, draw a card instead.",
        "Choose one. When you descend, choose both.",
        "Choose one. If you descended this turn, choose a target instead.",
    ] {
        let header = parse_modal_header_for_test(text)
            .expect("modal header should parse")
            .expect("modal header should be recognized");
        assert!(
            header.conditional_mode_change.is_none(),
            "{text}: {header:#?}"
        );
    }
}

#[test]
pub(super) fn refreshed_instead_mastery_alternative_cost_conditions_correlate_without_unsupported_predicates()
 {
    for (name, text, condition_surface) in [
        (
            "Ingenious Mastery",
            "You may pay {2}{U} rather than pay this spell's mana cost.\nIf the {2}{U} cost was paid, you draw three cards, then an opponent creates two Treasure tokens and they scry 2. If that cost wasn't paid, you draw X cards.",
            "ManaCost",
        ),
        (
            "Fervent Mastery",
            "You may pay {2}{R}{R} rather than pay this spell's mana cost.\nIf the {2}{R}{R} cost was paid, an opponent discards any number of cards, then draws that many cards.\nSearch your library for up to three cards, put them into your hand, shuffle, then discard three cards at random.",
            "ManaCost",
        ),
        (
            "Devastating Mastery",
            "You may pay {2}{W}{W} rather than pay this spell's mana cost.\nIf the {2}{W}{W} cost was paid, an opponent chooses up to two nonland permanents they control and returns them to their owner's hand.\nDestroy all nonland permanents.",
            "ManaCost",
        ),
        (
            "The Last Ronin's Technique",
            "Sneak {1}{W} (You may cast this spell for {1}{W} if you also return an unblocked attacker you control to hand during the declare blockers step.)\nCreate three 1/1 white Ninja Turtle Spirit creature tokens. If this spell's sneak cost was paid, they enter tapped and attacking.",
            "NamedCost",
        ),
    ] {
        let compiled = super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::new(), name),
            text,
            false,
        )
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
        let debug = format!("{:#?}", compiled.definition);
        assert!(debug.contains("AlternativeCast"), "{name}: {debug}");
        assert!(debug.contains(condition_surface), "{name}: {debug}");
        assert!(!debug.contains("CustomUnsupported"), "{name}: {debug}");
    }
}

#[test]
pub(super) fn refreshed_instead_conditional_token_entry_followup_branches_the_typed_token_producer_only()
 {
    let tokens = lex_line(
        "Create three 1/1 white Ninja Turtle Spirit creature tokens. If this spell's sneak cost was paid, they enter tapped and attacking.",
        0,
    )
    .expect("conditional token-entry fixture should lex");
    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("conditional token-entry fixture should parse");
    let [
        EffectAst::Conditional {
            if_true, if_false, ..
        },
    ] = effects.as_slice()
    else {
        panic!("expected one executable conditional token creation: {effects:#?}");
    };

    let creation_flags = |branch: &[EffectAst]| {
        let [EffectAst::SubjectVerb(subject_verb)] = branch else {
            panic!("expected one token creation branch: {branch:#?}");
        };
        let crate::model::ast::SubjectVerbActionAst::CreateTokenWithMods {
            tapped, attacking, ..
        } = &subject_verb.action
        else {
            panic!("expected a typed token producer: {subject_verb:#?}");
        };
        (*tapped, *attacking)
    };
    assert_eq!(creation_flags(if_true), (true, true));
    assert_eq!(creation_flags(if_false), (false, false));

    let near_miss = lex_line(
        "Create three 1/1 white Ninja Turtle Spirit creature tokens. If this spell's sneak cost was paid, they gain haste until end of turn.",
        0,
    )
    .expect("conditional token-grant near miss should lex");
    let near_miss = super::super::clause_support::parse_effect_sentences_lexed(&near_miss)
        .expect("conditional token-grant near miss should use the ordinary parser");
    assert!(
        matches!(near_miss.first(), Some(EffectAst::SubjectVerb(_))),
        "a different modifier must not replace the prior creation: {near_miss:#?}"
    );
}

#[test]
pub(super) fn refreshed_instead_opponent_choice_then_return_preserves_chooser_filter_and_chosen_set()
 {
    let tokens = lex_line(
        "An opponent chooses up to two nonland permanents they control and returns them to their owner's hand.",
        0,
    )
    .expect("opponent return-choice fixture should lex");
    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("opponent return-choice fixture should parse");
    let debug = format!("{parsed:#?}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("player: Opponent"), "{debug}");
    assert!(debug.contains("excluded_card_types"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
    assert!(debug.contains("ReturnAllToHand"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");

    let near_miss = lex_line(
        "An opponent chooses up to two nonland permanents they control and returns a creature to its owner's hand.",
        0,
    )
    .expect("different-return-object near miss should lex");
    let near_miss = super::super::clause_support::parse_effect_sentences_lexed(&near_miss);
    assert!(
        match &near_miss {
            Err(_) => true,
            Ok(effects) => !format!("{effects:#?}").contains("ReturnAllToHand"),
        },
        "a different return object must not be correlated to the chosen set: {near_miss:#?}"
    );
}

#[test]
pub(super) fn rewrite_modal_header_parser_tracks_x_replacement_without_word_view_scan() {
    let text = "Choose one. X is the number of spells you've cast this turn —";
    let header = parse_modal_header_for_test(text)
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
pub(super) fn rewrite_modal_header_parser_keeps_prefix_effect_and_result_gate() {
    let text = "Whenever this creature enters or attacks, draw a card. If you do, choose one —";
    let header = parse_modal_header_for_test(text)
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(header.trigger.is_some(), "{header:?}");
    assert!(!header.prefix_effects_ast.is_empty(), "{header:?}");
    assert!(matches!(
        header.modal_gate,
        Some(crate::cards::builders::ParsedModalGate {
            predicate: crate::effect::EffectPredicate::Happened,
            remove_mode_only: false,
            reflexive: false,
        })
    ));
}

#[test]
pub(super) fn rewrite_modal_header_parser_keeps_post_choice_common_effect_separate() {
    let text = "Whenever you scry, choose one and this creature gets +1/+1 until end of turn.";
    let header = parse_modal_header_for_test(text)
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(header.trigger.is_some(), "{header:?}");
    assert!(header.prefix_effects_ast.is_empty(), "{header:?}");
    assert_eq!(header.common_prefix_effects_ast.len(), 1, "{header:?}");
    assert!(
        matches!(
            header.common_prefix_effects_ast.as_slice(),
            [crate::model::ast::EffectAst::SubjectVerb(subject_verb)]
                if matches!(
                    &subject_verb.action,
                    crate::model::ast::SubjectVerbActionAst::Pump {
                        power: crate::effect::Value::Fixed(1),
                        toughness: crate::effect::Value::Fixed(1),
                        ..
                    }
                )
        ),
        "{:#?}",
        header.common_prefix_effects_ast
    );

    let ordinary = parse_modal_header_for_test("Choose one —")
        .expect("ordinary modal header should parse")
        .expect("ordinary modal header should be recognized");
    assert!(
        ordinary.common_prefix_effects_ast.is_empty(),
        "{ordinary:?}"
    );
}

#[test]
pub(super) fn rewrite_modal_header_parser_keeps_common_effect_before_bullet_dash() {
    let text = "At the beginning of your first main phase, choose one that hasn't been chosen and you lose 2 life —";
    let header = parse_modal_header_for_test(text)
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(header.trigger.is_some(), "{header:?}");
    assert!(header.prefix_effects_ast.is_empty(), "{header:?}");
    assert_eq!(header.common_prefix_effects_ast.len(), 1, "{header:?}");
    assert!(
        matches!(
            header.common_prefix_effects_ast.as_slice(),
            [crate::model::ast::EffectAst::SubjectVerb(subject_verb)]
                if matches!(
                    &subject_verb.action,
                    crate::model::ast::SubjectVerbActionAst::LoseLife {
                        amount: crate::effect::Value::Fixed(2),
                    }
                )
        ),
        "{:#?}",
        header.common_prefix_effects_ast
    );
}

#[test]
pub(super) fn rewrite_modal_header_parser_preserves_reflexive_result_gate() {
    let text =
        "At the beginning of combat on your turn, you may pay {E}{E}. When you do, choose one —";
    let header = parse_modal_header_for_test(text)
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(matches!(
        header.modal_gate,
        Some(crate::cards::builders::ParsedModalGate {
            predicate: crate::effect::EffectPredicate::Happened,
            remove_mode_only: false,
            reflexive: true,
        })
    ));
}

#[test]
pub(super) fn rewrite_modal_header_parser_marks_remove_mode_only_gate() {
    let text = "Whenever this creature attacks, remove a +1/+1 counter from it. If you removed it this way, choose one —";
    let header = parse_modal_header_for_test(text)
        .expect("modal header should parse")
        .expect("modal header should be recognized");

    assert!(matches!(
        header.modal_gate,
        Some(crate::cards::builders::ParsedModalGate {
            predicate: crate::effect::EffectPredicate::Happened,
            remove_mode_only: true,
            reflexive: false,
        })
    ));
}

#[test]
pub(super) fn rewrite_lowered_modal_when_you_do_uses_reflexive_trigger() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Voltstorm Angel")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "At the beginning of combat on your turn, you may pay {E}{E}. When you do, choose one —\n• This creature gains vigilance and lifelink until end of turn.\n• Other creatures you control get +1/+1 until end of turn.".to_string(),
        false,
    )?;
    let ability_debug = format!("{:#?}", definition.abilities);

    assert!(
        ability_debug.contains("ReflexiveTriggerEffect"),
        "{ability_debug}"
    );
    assert!(
        ability_debug.contains("ChooseModeEffect"),
        "{ability_debug}"
    );
    assert!(!ability_debug.contains("IfEffect"), "{ability_debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_modal_header_parse_all_reports_invalid_choose_clause() {
    let error = parse_error_message(parse_modal_header_for_test(
        "Whenever this creature enters, choose nonsense —",
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
pub(super) fn rewrite_modal_header_error_reports_line_and_span_after_activation_prefix_discrimination()
 {
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
pub(super) fn rewrite_modal_header_error_reports_line_and_eof_after_trigger_prefix_discrimination()
{
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
pub(super) fn rewrite_document_parser_supports_activate_only_once_each_turn_without_period() {
    let card = CardBuilder::new(CardId::new(), "Activated Limit Variant")
        .card_types(vec![CardType::Artifact]);
    let preprocessed = super::super::preprocess::preprocess_document(
        card,
        "Equip {0}\nActivate only once each turn",
    )
    .expect("expected preprocessing to accept activate-only-once line without trailing period");
    let cst = super::super::document_parser::recognize_document(&preprocessed, false).expect(
        "expected document parser to accept activate-only-once line without trailing period",
    );

    assert!(
        cst.lines.iter().any(|line| {
            matches!(
                line,
                super::super::recognized_document::RecognizedLine::Static(static_line)
                    if render_token_slice(&static_line.parse_tokens).trim()
                        == "activate only once each turn"
            )
        }),
        "expected static CST line for activate-only-once line, got {cst:?}"
    );
}

#[test]
pub(super) fn rewrite_document_parser_supports_equip_with_subtype_qualifier() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Subtype Equip Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment]);

    let def = builder
        .parse_text("Equip Soldier {W}")
        .expect("expected parser to support subtype-qualified equip");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("subtypes:") && abilities_debug.contains("Soldier"),
        "expected equip target filter to include Soldier subtype, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("AttachObjectsEffect"),
        "expected equip ability to remain an attach activation, got {abilities_debug}"
    );
}

#[test]
pub(super) fn rewrite_document_parser_supports_robe_of_the_archmagi() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Robe of the Archmagi")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment]);

    let def = builder
        .parse_text(
            "Whenever equipped creature deals combat damage to a player, you draw that many cards.\n\
             Equip {4}\n\
             Equip Shaman, Warlock, or Wizard {1}",
        )
        .expect("expected Robe of the Archmagi to parse strictly");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("Shaman")
            && abilities_debug.contains("Warlock")
            && abilities_debug.contains("Wizard"),
        "expected subtype-disjunction equip target filter, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("DrawCardsEffect")
            && abilities_debug.contains("EventValue(")
            && abilities_debug.contains("Amount"),
        "expected triggered draw-that-many effect to be preserved, got {abilities_debug}"
    );
}

#[test]
pub(super) fn rewrite_document_parser_splits_activation_cost_on_colon_outside_quotes() {
    let card = CardBuilder::new(CardId::new(), "Quoted Colon Variant")
        .card_types(vec![CardType::Artifact]);
    let preprocessed =
        super::super::preprocess::preprocess_document(card, "{T}: Choose \"fire: ice\".")
            .expect("expected preprocessing to accept quoted-colon activation line");
    let cst = super::super::document_parser::recognize_document(&preprocessed, false)
        .expect("expected document parser to split activation on colon outside quotes");

    let activated = cst
        .lines
        .iter()
        .find_map(|line| match line {
            super::super::recognized_document::RecognizedLine::Activated(activated) => {
                Some(activated)
            }
            _ => None,
        })
        .expect("expected activated CST line");

    let effect_text = render_token_slice(&activated.effect_parse_tokens);
    assert!(
        effect_text.contains("fire: ice"),
        "expected quoted inner colon to stay in effect text, got {effect_text:?}"
    );
}

#[test]
pub(super) fn rewrite_document_parser_splits_nonactivation_colon_outside_quotes() {
    let tokens = lex_line("Reveal this card from your hand: \"fire: ice\".", 0)
        .expect("expected lexer to accept quoted-colon non-activation line");
    let (left, right) =
        super::super::document_parser::split_lexed_once_on_colon_outside_quotes(&tokens)
            .expect("expected shared colon helper to split on the outer colon only");

    assert_eq!(
        render_token_slice(left).trim(),
        "Reveal this card from your hand"
    );
    assert_eq!(render_token_slice(right).trim(), "\"fire: ice\".");
}

#[test]
pub(super) fn rewrite_document_parser_dispatches_keyword_lines_by_head_phrase()
-> Result<(), CardTextError> {
    let alt_preprocessed = super::super::preprocess::preprocess_document(
        CardBuilder::new(CardId::new(), "Alt Cost Variant").card_types(vec![CardType::Instant]),
        "If an opponent cast two or more spells this turn, you may pay {1}{R} rather than pay this spell's mana cost.",
    )?;
    let alt_cst = super::super::document_parser::recognize_document(&alt_preprocessed, false)?;
    assert!(matches!(
        alt_cst.lines.as_slice(),
        [super::super::recognized_document::RecognizedLine::Keyword(keyword)]
            if matches!(keyword.kind, super::super::recognized_document::KeywordLineKind::AlternativeCast)
    ));

    let gift_preprocessed = super::super::preprocess::preprocess_document(
        CardBuilder::new(CardId::new(), "Gift Variant").card_types(vec![CardType::Sorcery]),
        "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)",
    )?;
    let gift_cst = super::super::document_parser::recognize_document(&gift_preprocessed, false)?;
    assert!(matches!(
        gift_cst.lines.as_slice(),
        [super::super::recognized_document::RecognizedLine::Keyword(keyword)]
            if matches!(keyword.kind, super::super::recognized_document::KeywordLineKind::Gift)
    ));

    Ok(())
}

#[test]
pub(super) fn rewrite_splice_keyword_lines_lower_typed_subject_and_cost_without_alternative_casting()
-> Result<(), CardTextError> {
    for (line, expected_label) in [
        (
            "Splice onto Arcane {1}{R} (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "Splice onto Arcane {1}{R}",
        ),
        (
            "Splice onto instant or sorcery {2}{U} (As you cast an instant or sorcery spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "Splice onto instant or sorcery {2}{U}",
        ),
    ] {
        let preprocessed = super::super::preprocess::preprocess_document(
            CardBuilder::new(CardId::new(), "Keyword Probe").card_types(vec![CardType::Instant]),
            line,
        )?;
        let cst = super::super::document_parser::recognize_document(&preprocessed, false)?;
        assert!(matches!(
            cst.lines.as_slice(),
            [super::super::recognized_document::RecognizedLine::Keyword(keyword)]
                if keyword.kind == super::super::recognized_document::KeywordLineKind::Splice
        ));

        let definition = CardDefinitionBuilder::new(CardId::new(), "Keyword Probe")
            .card_types(vec![CardType::Instant])
            .parse_text(line)?;
        assert!(
            definition.alternative_casts.is_empty(),
            "splice must remain a static hand ability, not an AlternativeCastingMethod"
        );
        let splice = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => match &static_ability.payload {
                    crate::static_abilities::StaticAbilityPayload::Splice(spec) => {
                        Some((static_ability, spec))
                    }
                    _ => None,
                },
                _ => None,
            })
            .expect("typed splice line should lower to an executable splice payload");
        let (static_ability, spec) = splice;
        assert_eq!(static_ability.id(), StaticAbilityId::Splice);
        assert_eq!(static_ability.display(), expected_label);
        assert_eq!(
            spec.cost.display(),
            expected_label.rsplit_once(' ').unwrap().1
        );
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_splice_keyword_lines_lower_canonical_nonmana_costs()
-> Result<(), CardTextError> {
    for line in [
        "Splice onto Arcane—Exile four cards from your graveyard. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
        "Splice onto Arcane—Tap an untapped white creature you control. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
        "Splice onto Arcane—An opponent gains 5 life. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
        "Splice onto Arcane—Sacrifice two Mountains. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
        "Splice onto Arcane—Return a blue creature you control to its owner's hand. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Nonmana Splice Probe")
            .card_types(vec![CardType::Instant])
            .parse_text(line)?;
        let spec = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => match &static_ability.payload {
                    crate::static_abilities::StaticAbilityPayload::Splice(spec) => Some(spec),
                    _ => None,
                },
                _ => None,
            })
            .expect("canonical nonmana splice cost should lower to a typed payload");
        assert!(spec.cost.has_non_mana_costs(), "line: {line}");
        assert_ne!(spec.cost.display(), "Free", "line: {line}");
    }
    Ok(())
}

#[test]
pub(super) fn rewrite_static_lowering_reuses_token_sentences_for_multi_sentence_lines()
-> Result<(), CardTextError> {
    let text =
        "this creature has flying. as long as you control an artifact, this creature has haste.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify multi-sentence static line");

    let parsed =
        crate::semantic_line_parsing::parse_static_line(rewrite_line_info(text), &tokens, None)?;

    match parsed {
        crate::cards::builders::LineAst::StaticAbilities(abilities) => {
            assert_eq!(abilities.len(), 2);
        }
        other => panic!("expected split static abilities, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_static_lowering_reuses_token_split_for_compound_unblockable_line()
-> Result<(), CardTextError> {
    let text = "enchanted creature gets +2/+2 and can't be blocked.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify compound buff static line");

    let parsed =
        crate::semantic_line_parsing::parse_static_line(rewrite_line_info(text), &tokens, None)?;

    match parsed {
        crate::cards::builders::LineAst::StaticAbilities(abilities) => {
            assert_eq!(abilities.len(), 2);
        }
        other => panic!("expected split compound static abilities, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_keyword_lowering_reuses_token_sentences_for_optional_cost_cast_trigger()
-> Result<(), CardTextError> {
    let text = "as an additional cost to cast this spell, you may sacrifice one or more creatures. when you do, copy this spell for each creature sacrificed this way.";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify additional-cost cast-trigger keyword line");

    let parsed = crate::semantic_line_parsing::parse_keyword_line_for_test(
        rewrite_line_info(text),
        text,
        &tokens,
        RewriteKeywordLineKind::AdditionalCost,
    )?;

    match parsed {
        crate::cards::builders::LineAst::OptionalCostWithCastTrigger { effects, .. } => {
            assert!(!effects.is_empty());
        }
        other => panic!("expected optional-cost cast trigger line, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn chosen_type_behold_cost_preempts_generic_behold_exile_shape()
-> Result<(), CardTextError> {
    let text = "As an additional cost to cast this spell, you may choose a creature type and behold two creatures of that type.";
    let tokens = lex_line(text, 0).expect("chosen-type behold cost should lex");

    let parsed = crate::semantic_line_parsing::parse_keyword_line_for_test(
        rewrite_line_info(text),
        text,
        &tokens,
        RewriteKeywordLineKind::AdditionalCost,
    )?;

    let crate::cards::builders::LineAst::OptionalCost(optional) = parsed else {
        panic!("expected a typed optional additional cost, got {parsed:?}");
    };
    let costs = optional
        .cost
        .as_all()
        .expect("chosen-type behold cost should be a cost sequence");
    assert_eq!(costs.len(), 2);
    assert!(format!("{costs:#?}").contains("ChooseCreatureType"));
    assert!(format!("{costs:#?}").contains("beheld_chosen_type"));
    assert!(!format!("{costs:#?}").contains("MoveToZone"));
    Ok(())
}

pub(super) fn assert_composed_keyword_cost(
    parsed: crate::cards::builders::LineAst,
    expected_name: &str,
    expected_cost: &str,
) {
    let crate::cards::builders::LineAst::AlternativeCastingMethod(method) = parsed else {
        panic!("expected {expected_name} alternative casting method");
    };
    let (name, mana_cost) = (method.name(), method.mana_cost());
    assert_eq!(name, expected_name);
    assert_eq!(
        mana_cost
            .expect("keyword alternative cost should contain mana")
            .to_oracle(),
        expected_cost
    );
}

#[test]
pub(super) fn rewrite_keyword_lowering_uses_carried_surge_tokens_after_cst_rewrite()
-> Result<(), CardTextError> {
    let raw = "Surge {3}{U}{U} (You may cast this spell for its surge cost if you or a teammate has cast another spell this turn.)";
    let parse_tokens = lex_line(
        "If you've cast another spell this turn, you may pay {3}{U}{U} rather than pay this spell's mana cost.",
        0,
    )?;
    let full_parse_tokens = lex_line(raw, 0)?;
    let parsed = crate::semantic_line_parsing::parse_keyword_line_with_full_tokens_for_test(
        rewrite_line_info(raw),
        raw,
        &parse_tokens,
        &full_parse_tokens,
        RewriteKeywordLineKind::AlternativeCast,
    )?;

    assert_composed_keyword_cost(parsed, "Surge", "{3}{U}{U}");
    Ok(())
}

#[test]
pub(super) fn rewrite_keyword_lowering_uses_carried_freerunning_tokens_after_cst_rewrite()
-> Result<(), CardTextError> {
    let raw = "Freerunning {2}{R} (You may cast this spell for its freerunning cost if you dealt combat damage to a player this turn with an Assassin or commander.)";
    let parse_tokens = lex_line(
        "If you dealt combat damage to a player this turn with an Assassin or commander, you may pay {2}{R} rather than pay this spell's mana cost.",
        0,
    )?;
    let full_parse_tokens = lex_line(raw, 0)?;
    let parsed = crate::semantic_line_parsing::parse_keyword_line_with_full_tokens_for_test(
        rewrite_line_info(raw),
        raw,
        &parse_tokens,
        &full_parse_tokens,
        RewriteKeywordLineKind::AlternativeCast,
    )?;

    assert_composed_keyword_cost(parsed, "Freerunning", "{2}{R}");
    Ok(())
}

#[test]
pub(super) fn rewrite_keyword_lowering_uses_normalized_sneak_cost_and_full_form_tokens()
-> Result<(), CardTextError> {
    let raw = "Sneak {1}{B} (You may cast this spell for {1}{B} if you also return an unblocked attacker you control to hand during the declare blockers step.)";
    let parse_tokens = lex_line("Sneak {1}{B}", 0)?;
    let full_parse_tokens = lex_line(raw, 0)?;
    let parsed = crate::semantic_line_parsing::parse_keyword_line_with_full_tokens_for_test(
        rewrite_line_info(raw),
        raw,
        &parse_tokens,
        &full_parse_tokens,
        RewriteKeywordLineKind::AlternativeCast,
    )?;

    assert_composed_keyword_cost(parsed, "Sneak", "{1}{B}");
    Ok(())
}

#[test]
pub(super) fn rewrite_keyword_lowering_does_not_relex_stale_text_for_cost()
-> Result<(), CardTextError> {
    let raw = "Freerunning {2}{R}";
    let tokens = lex_line(raw, 0)?;
    let parsed = crate::semantic_line_parsing::parse_keyword_line_for_test(
        rewrite_line_info(raw),
        "Freerunning {9}{U}",
        &tokens,
        RewriteKeywordLineKind::AlternativeCast,
    )?;

    assert_composed_keyword_cost(parsed, "Freerunning", "{2}{R}");
    Ok(())
}

#[test]
pub(super) fn rewrite_statement_lowering_reuses_full_token_slice_for_pact_line()
-> Result<(), CardTextError> {
    let text = "search your library for a green creature card, reveal it, put it into your hand, then shuffle. at the beginning of your next upkeep, pay {2}{G}{G}. if you don't, you lose the game.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify pact statement line");

    let parsed_chunks = crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        rewrite_line_info(text),
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
pub(super) fn rewrite_statement_lowering_chains_prevention_and_pact_programs()
-> Result<(), CardTextError> {
    let text = "The next time a source of your choice would deal damage to you this turn, prevent that damage. You gain life equal to the damage prevented this way. At the beginning of your next upkeep, pay {1}{W}{W}. If you don't, you lose the game.";
    let tokens = lex_line(text, 0).expect("combined prevention and Pact line should lex");

    let parsed_chunks = crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        rewrite_line_info(text),
        &tokens,
        &[],
    )?;

    let [crate::cards::builders::LineAst::Statement { effects }] = parsed_chunks.as_slice() else {
        panic!("expected one combined statement chunk, got {parsed_chunks:#?}");
    };
    assert_eq!(effects.len(), 2, "{effects:#?}");
    assert!(
        matches!(
            effects.last(),
            Some(crate::cards::builders::EffectAst::SourceSentence { effects, .. })
                if matches!(effects.as_slice(), [crate::cards::builders::EffectAst::DelayedUntilNextUpkeep { .. }])
        ),
        "{effects:#?}"
    );
    let debug = format!("{effects:#?}");
    assert!(debug.contains("PreventNextTimeDamage"), "{debug}");
    assert!(debug.contains("GainLife"), "{debug}");
    assert!(debug.contains("LoseGame"), "{debug}");

    Ok(())
}

#[test]
pub(super) fn rewrite_statement_keeps_graveyard_card_copy_cast_as_one_typed_sequence()
-> Result<(), CardTextError> {
    let text = "Exile target instant or sorcery card from a graveyard and copy it. You may cast the copy without paying its mana cost.";
    let tokens = lex_line(text, 0).expect("graveyard card-copy statement should lex");

    let parsed_chunks = crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        rewrite_line_info(text),
        &tokens,
        &[],
    )?;
    let [crate::cards::builders::LineAst::Statement { effects }] = parsed_chunks.as_slice() else {
        panic!("expected one statement chunk, got {parsed_chunks:#?}");
    };
    let debug = format!("{effects:#?}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("as_copy: true"), "{debug}");
    assert!(!debug.contains("CopySpell"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_statement_lowering_uses_parse_tokens_when_groups_are_missing()
-> Result<(), CardTextError> {
    let token_text = "Meteor Strikes — Exile target artifact. When you do, draw a card.";
    let tokens =
        lex_line(token_text, 0).expect("rewrite lexer should classify statement token fallback");

    let parsed_chunks = crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        rewrite_line_info("placeholder statement text"),
        &tokens,
        &[],
    )?;

    match parsed_chunks.as_slice() {
        [crate::cards::builders::LineAst::Statement { effects }] => {
            let debug = format!("{effects:?}");
            assert!(debug.contains("Exile"), "{debug}");
            assert!(debug.contains("WhenResult"), "{debug}");
            assert!(debug.contains("Draw"), "{debug}");
        }
        other => panic!("expected single rewritten statement chunk, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_statement_lowering_parses_soul_partition_via_parser_path()
-> Result<(), CardTextError> {
    let text = "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {2} more to cast.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify Soul Partition text");

    let parsed_chunks = crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        rewrite_line_info(text),
        &tokens,
        &[],
    )?;

    match parsed_chunks.as_slice() {
        [crate::cards::builders::LineAst::Statement { effects }] => {
            let debug = format!("{effects:#?}");
            assert!(
                debug.contains("GrantBySpec") || debug.contains("GrantPlayTaggedForAsLongAsExiled"),
                "{debug}"
            );
            assert!(debug.contains("GrantToTarget"), "{debug}");
            assert!(debug.contains("CostIncreaseManaCost"), "{debug}");
        }
        other => panic!("expected single Soul Partition statement chunk, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_statement_lowering_parses_empty_laboratory_via_parser_path()
-> Result<(), CardTextError> {
    let text = "Sacrifice X Zombies, then reveal cards from the top of your library until you reveal a number of Zombie creature cards equal to the number of Zombies sacrificed this way. Put those cards onto the battlefield and the rest on the bottom of your library in a random order.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify Empty Laboratory text");

    let parsed_chunks = crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        rewrite_line_info(text),
        &tokens,
        &[],
    )?;

    match parsed_chunks.as_slice() {
        [crate::cards::builders::LineAst::Statement { effects }] => {
            let debug = format!("{effects:#?}");
            assert!(debug.contains("ChooseObjects"), "{debug}");
            assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
            assert!(
                debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
                "{debug}"
            );
        }
        other => panic!("expected single Empty Laboratory statement chunk, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_statement_lowering_parses_shape_anew_via_parser_path()
-> Result<(), CardTextError> {
    let text = "The controller of target artifact sacrifices it, then reveals cards from the top of their library until they reveal an artifact card. That player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify Shape Anew text");

    let parsed_chunks = crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        rewrite_line_info(text),
        &tokens,
        &[],
    )?;

    match parsed_chunks.as_slice() {
        [crate::cards::builders::LineAst::Statement { effects }] => {
            let debug = format!("{effects:#?}");
            assert!(debug.contains("Sacrifice"), "{debug}");
            assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
            assert!(debug.contains("ShuffleLibrary"), "{debug}");
        }
        other => panic!("expected single Shape Anew statement chunk, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_for_each_exiled_reveal_until_then_bottom_uses_consult() {
    let text = "For each creature exiled this way, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then puts the rest on the bottom of their library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify Chaos Mutation followup");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("for-each exiled reveal-until-bottom sentence should parse");
    let [EffectAst::ForEachTagged { tag, effects }] = parsed.as_slice() else {
        panic!("expected canonical tagged exile iteration, got {parsed:#?}");
    };
    assert_eq!(
        tag.as_str(),
        crate::tag::CompilerReferenceTag::SourceExiled.as_str()
    );

    let inner = format!("{effects:#?}");
    assert!(inner.contains("ConsultTopOfLibrary"), "{inner}");
    assert!(
        inner.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{inner}"
    );
}

#[test]
pub(super) fn rewrite_statement_lowering_parses_nissas_encouragement_via_parser_path()
-> Result<(), CardTextError> {
    let text = "Search your library and graveyard for a card named Forest, a card named Brambleweft Behemoth, and a card named Nissa, Genesis Mage. Reveal those cards, put them into your hand, then shuffle.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify Nissa's Encouragement text");

    let parsed_chunks = crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        rewrite_line_info(text),
        &tokens,
        &[],
    )?;

    match parsed_chunks.as_slice() {
        [crate::cards::builders::LineAst::Statement { effects }] => {
            let debug = format!("{effects:#?}");
            assert!(debug.contains("SearchLibrarySlots"), "{debug}");
            assert!(debug.contains("\"Forest\""), "{debug}");
            assert!(debug.contains("\"Brambleweft Behemoth\""), "{debug}");
            assert!(debug.contains("\"Nissa, Genesis Mage\""), "{debug}");
            assert!(debug.contains("destination: Hand"), "{debug}");
            assert!(debug.contains("reveal: true"), "{debug}");
        }
        other => panic!("expected single Nissa's Encouragement statement chunk, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_exception_module_is_removed_from_lowering_tree() {
    let rewrite_exceptions = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/cards/builders/compiler/lowering/rewrite_exceptions.rs");

    assert!(
        !rewrite_exceptions.exists(),
        "expected rewrite exception module to be removed, found {}",
        rewrite_exceptions.display()
    );
}

#[test]
pub(super) fn rewrite_triggered_lowering_uses_parse_tokens_when_text_fields_are_stale()
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

    let parsed = crate::semantic_line_parsing::parse_triggered_line(
        rewrite_line_info("placeholder triggered text"),
        "placeholder triggered text",
        &full_tokens,
        &trigger_tokens,
        &effect_tokens,
        None,
        None,
        None,
        None,
    )?;

    let debug = format!("{parsed:?}");
    assert!(debug.contains("Triggered"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");

    Ok(())
}

#[test]
pub(super) fn rewrite_triggered_keeps_linked_exile_permission_tax_in_resolution()
-> Result<(), CardTextError> {
    let full_text = "When this creature enters, look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to cast.";
    let trigger_text = "When this creature enters";
    let effect_text = "look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to cast.";
    let full_tokens = lex_line(full_text, 0).expect("linked exile-tax trigger should lex");
    let trigger_tokens = lex_line(trigger_text, 0).expect("enter trigger should lex");
    let effect_tokens = lex_line(effect_text, 0).expect("linked exile-tax effects should lex");

    let parsed = crate::semantic_line_parsing::parse_triggered_line(
        rewrite_line_info(full_text),
        full_text,
        &full_tokens,
        &trigger_tokens,
        &effect_tokens,
        None,
        None,
        None,
        None,
    )?;

    let effects = match parsed {
        crate::cards::builders::LineAst::Triggered { effects, .. } => effects,
        crate::cards::builders::LineAst::Ability(parsed) => parsed
            .effects_ast
            .expect("linked exile-tax ability must retain parsed effects"),
        other => {
            panic!("linked exile-tax program must remain one triggered ability, got {other:#?}")
        }
    };
    let debug = format!("{effects:#?}");
    assert_eq!(effects.len(), 4, "{debug}");
    assert!(debug.contains("LookAtHand"), "{debug}");
    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(debug.contains("GrantToTarget"), "{debug}");
    assert!(debug.contains("CostIncreaseManaCost"), "{debug}");

    Ok(())
}

#[test]
pub(super) fn rewrite_combat_death_blocked_damage_special_case_uses_parse_tokens()
-> Result<(), CardTextError> {
    let full_text = "when this creature dies during combat, it deals 2 damage to each creature it blocked this combat.";
    let trigger_text = "when this creature dies during combat";
    let effect_text = "it deals 2 damage to each creature it blocked this combat.";
    let full_tokens = lex_line(full_text, 0)
        .expect("rewrite lexer should classify combat death blocked-damage trigger");
    let trigger_tokens =
        lex_line(trigger_text, 0).expect("rewrite lexer should classify combat death trigger");
    let effect_tokens =
        lex_line(effect_text, 0).expect("rewrite lexer should classify blocked-damage effect");

    let parsed = crate::semantic_line_parsing::parse_triggered_line(
        rewrite_line_info("placeholder triggered text"),
        "placeholder triggered text",
        &full_tokens,
        &trigger_tokens,
        &effect_tokens,
        None,
        None,
        None,
        None,
    )?;

    let debug = format!("{parsed:?}");
    assert!(debug.contains("Triggered"), "{debug}");
    assert!(debug.contains("DealDamage"), "{debug}");

    Ok(())
}

#[test]
pub(super) fn rewrite_gift_keyword_lowering_builds_closed_form_followup_effects()
-> Result<(), CardTextError> {
    let cases = [
        (
            "gift a card (you may promise an opponent a gift as you cast this spell. if you do, they draw a card before its other effects.)",
            "draw",
            crate::cards::builders::GiftTimingAst::SpellResolution,
        ),
        (
            "gift a tapped fish (you may promise an opponent a gift as you cast this spell. if you do, they create a tapped 1/1 blue fish creature token before its other effects.)",
            "fish",
            crate::cards::builders::GiftTimingAst::SpellResolution,
        ),
        (
            "gift an extra turn (you may promise an opponent a gift as you cast this spell. if you do, they take an extra turn after this one before its other effects.)",
            "extra-turn",
            crate::cards::builders::GiftTimingAst::SpellResolution,
        ),
    ];

    for (text, expected_effect, expected_timing) in cases {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify gift keyword line");
        let parsed = crate::semantic_line_parsing::parse_keyword_line_for_test(
            rewrite_line_info(text),
            text,
            &tokens,
            RewriteKeywordLineKind::Gift,
        )?;

        match parsed {
            crate::cards::builders::LineAst::GiftKeyword {
                effects, timing, ..
            } => {
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
                match expected_effect {
                    "draw" => assert!(matches!(
                        effects.as_slice(),
                        [crate::cards::builders::EffectAst::SubjectVerb(
                            crate::cards::builders::SubjectVerbEffectAst {
                                subject: crate::cards::builders::SubjectVerbSubjectAst {
                                    player: crate::cards::builders::PlayerAst::Chosen,
                                    ..
                                },
                                action: crate::cards::builders::SubjectVerbActionAst::Draw {
                                    count: crate::effect::Value::Fixed(1),
                                },
                            }
                        )]
                    )),
                    "fish" => {
                        assert!(matches!(
                            effects.as_slice(),
                            [crate::cards::builders::EffectAst::SubjectVerb(
                                crate::cards::builders::SubjectVerbEffectAst {
                                    action:
                                        crate::cards::builders::SubjectVerbActionAst::CreateTokenWithMods {
                                            name,
                                            count: crate::effect::Value::Fixed(1),
                                            player: crate::cards::builders::PlayerAst::Chosen,
                                            tapped: true,
                                            ..
                                        },
                                    ..
                                }
                            )] if name == "1/1 blue Fish creature"
                        ))
                    }
                    "extra-turn" => assert!(matches!(
                        effects.as_slice(),
                        [
                            crate::cards::builders::EffectAst::SubjectVerb(
                                crate::cards::builders::SubjectVerbEffectAst {
                                    subject: crate::cards::builders::SubjectVerbSubjectAst {
                                        player: crate::cards::builders::PlayerAst::Chosen,
                                        ..
                                    },
                                    action: crate::cards::builders::SubjectVerbActionAst::ExtraTurnAfterTurn {
                                        anchor: crate::cards::builders::ExtraTurnAnchorAst::CurrentTurn,
                                    },
                                }
                            )
                        ]
                    )),
                    other => panic!("unexpected gift effect variant: {other}"),
                }
            }
            other => panic!("expected gift keyword line, got {other:?}"),
        }
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_token_word_view_caches_lower_words_and_word_token_indices() {
    let tokens = lex_line("Activate only during your turn.", 0)
        .expect("rewrite lexer should classify restriction text");
    let words = TokenWordView::new(&tokens);
    assert_eq!(words.get(0), Some("activate"));
    assert_eq!(words.get(3), Some("your"));
    assert_eq!(words.map_word_to_token_boundary(4), Some(4));
    assert!(words.parses_prefix(&["activate", "only"]));
}

#[test]
pub(super) fn rewrite_token_word_view_normalizes_parser_word_shapes() {
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
    assert_eq!(words.map_word_to_token_boundary(2), Some(2));
    assert_eq!(words.token_index_after_words(4), Some(3));
    assert_eq!(words.token_index_after_words(5), Some(4));
}

#[test]
pub(super) fn rewrite_token_word_view_centralizes_token_shape_policy() {
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
        super::super::token_word_refs(&tokens),
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
pub(super) fn rewrite_owned_lex_token_replace_word_refreshes_cached_parser_word_pieces() {
    let mut token = lex_line("face-down", 0)
        .expect("rewrite lexer should classify split word token")
        .into_iter()
        .next()
        .expect("expected one token");

    assert_eq!(
        super::super::lexer::parser_token_word_refs(std::slice::from_ref(&token)),
        vec!["face", "down"]
    );

    assert!(token.replace_word("controllers'"));
    assert_eq!(
        super::super::lexer::parser_token_word_refs(std::slice::from_ref(&token)),
        vec!["controllers"]
    );
}

#[test]
pub(super) fn rewrite_rule_engine_lex_clause_view_normalizes_parser_word_shapes() {
    let tokens = lex_line(
        "Whenever its owner's face-down creature attacks, draw a card.",
        0,
    )
    .expect("rewrite lexer should classify rule-engine clause");
    let view = crate::rule_engine::LexClauseView::from_tokens(&tokens);

    assert_eq!(view.head(), "whenever");
    assert_eq!(
        view.words.to_word_refs(),
        vec![
            "whenever", "its", "owners", "face", "down", "creature", "attacks", "draw", "a", "card"
        ]
    );
    assert_eq!(
        view.shape,
        crate::rule_engine::RULE_SHAPE_STARTS_WHENEVER | crate::rule_engine::RULE_SHAPE_HAS_COMMA
    );
    assert_eq!(
        view.display_text(),
        "whenever its owners face down creature attacks draw a card"
    );
}

#[test]
pub(super) fn rewrite_parser_support_detects_this_way_followup_intro() {
    let tokens = lex_line("Whenever one or more cards are exiled this way", 0)
        .expect("rewrite lexer should classify followup text");
    let plain_tokens = lex_line("Whenever one or more cards are exiled", 0)
        .expect("rewrite lexer should classify non-followup text");

    assert!(super::super::looks_like_spell_resolution_followup_intro_lexed(&tokens));
    assert!(!super::super::looks_like_spell_resolution_followup_intro_lexed(&plain_tokens));
}

#[test]
pub(super) fn rewrite_parser_support_detects_when_you_do_followup_intro() {
    let tokens = lex_line("When you do, exile target creature.", 0)
        .expect("rewrite lexer should classify reflexive followup text");
    let delayed_tokens = lex_line("At the beginning of the next end step, draw a card.", 0)
        .expect("rewrite lexer should classify delayed trigger text");

    assert!(super::super::looks_like_reflexive_followup_intro_lexed(
        &tokens
    ));
    assert!(!super::super::looks_like_reflexive_followup_intro_lexed(
        &delayed_tokens
    ));
}

#[test]
pub(super) fn rewrite_parser_support_splits_quoted_sentences_and_queues_restrictions() {
    let tokens = lex_line(
        "Draw a card. \"Choose one.\" Activate only during your turn.",
        0,
    )
    .expect("rewrite lexer should classify quoted sentences and restrictions");
    let (parsed_sentence_tokens, restrictions) =
        super::super::parser_support::split_tokens_for_parse(&tokens);
    let parsed_sentences = parsed_sentence_tokens
        .iter()
        .map(|tokens| {
            super::super::lexer::render_token_slice(tokens)
                .trim()
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        parsed_sentences,
        vec!["Draw a card".to_string(), "\"Choose one.\"".to_string()]
    );
    assert_eq!(
        restrictions
            .activation
            .iter()
            .map(|restriction| restriction.presentation_text.as_str())
            .collect::<Vec<_>>(),
        vec!["Activate only during your turn"]
    );
    assert!(restrictions.trigger.is_empty());
}

#[test]
pub(super) fn rewrite_lexed_restriction_parsers_match_activation_trigger_and_mana_shapes() {
    let activate_only = lex_line("Activate only during your turn.", 0)
        .expect("rewrite lexer should classify activation restriction");
    let trigger_only = lex_line("This ability triggers only once each turn.", 0)
        .expect("rewrite lexer should classify trigger restriction");
    let do_this_only = lex_line("Do this only once each turn.", 0)
        .expect("rewrite lexer should classify do-this-only trigger cap");
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
    assert_eq!(
        parse_triggered_times_each_turn_lexed(&do_this_only),
        Some(1)
    );
    assert!(matches!(
        parse_mana_usage_restriction_sentence_lexed(&mana_only),
        Some(ironsmith_core::ManaUsageRestriction::CastSpell {
            card_types,
            subtype_requirement: Some(
                crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource
            ),
            restrict_to_matching_spell: true,
            grant_uncounterable: true,
            enters_with_counters,
            granted_abilities,
        }) if card_types == vec![CardType::Artifact]
            && enters_with_counters.is_empty()
            && granted_abilities.is_empty()
    ));
}

#[test]
pub(super) fn rewrite_lexed_mana_restrictions_parse_supported_spell_filter_shapes() {
    fn parse_filter(text: &str) -> crate::target::ObjectFilter {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify mana restriction");
        match parse_mana_usage_restriction_sentence_lexed(&tokens) {
            Some(ironsmith_core::ManaUsageRestriction::CastSpellMatching { filter, .. }) => filter,
            other => panic!("expected CastSpellMatching restriction for {text:?}, got {other:?}"),
        }
    }

    let commander = parse_filter("Spend this mana only to cast your commander.");
    assert!(commander.is_commander);
    assert_eq!(commander.owner, Some(crate::target::PlayerFilter::You));

    let graveyard = parse_filter("Spend this mana only to cast a spell from your graveyard.");
    assert_eq!(graveyard.zone, Some(crate::zone::Zone::Graveyard));
    assert_eq!(graveyard.owner, Some(crate::target::PlayerFilter::You));

    let flashback =
        parse_filter("Spend this mana only to cast spells with flashback from a graveyard.");
    assert_eq!(flashback.zone, Some(crate::zone::Zone::Graveyard));
    assert_eq!(
        flashback.alternative_cast,
        Some(crate::filter::AlternativeCastKind::Flashback)
    );

    let exile = parse_filter("Spend this mana only to cast spells from exile.");
    assert_eq!(exile.zone, Some(crate::zone::Zone::Exile));

    let devoid = parse_filter("Spend this mana only to cast a spell with devoid.");
    assert_eq!(
        devoid.static_abilities,
        vec![StaticAbilityId::MakeColorless]
    );

    let no_abilities =
        parse_filter("Spend this mana only to cast creature spells with no abilities.");
    assert_eq!(no_abilities.card_types, vec![CardType::Creature]);
    assert!(no_abilities.no_abilities);

    let unowned = parse_filter("Spend this mana only to cast spells you don't own.");
    assert_eq!(unowned.owner, Some(crate::target::PlayerFilter::NotYou));
}

#[test]
pub(super) fn rewrite_spell_mana_restriction_wraps_preceding_mana_effect() {
    let tokens = lex_line(
        "Add one mana of any one color. Spend this mana only to cast creature spells.",
        0,
    )
    .expect("rewrite lexer should classify spell mana restriction");

    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("effect sentences should parse");

    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ManaRestricted"),
        "expected mana restriction wrapper in parsed effects, got {debug}"
    );
    assert!(
        debug.contains("AddManaAnyOneColor"),
        "expected wrapped mana-producing effect, got {debug}"
    );
    assert!(
        debug.contains("Creature"),
        "expected creature spell usage restriction, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_activate_ability_mana_restriction_parses() {
    let tokens = lex_line("Spend this mana only to activate abilities.", 0)
        .expect("rewrite lexer should classify mana restriction");

    assert!(matches!(
        parse_mana_usage_restriction_sentence_lexed(&tokens),
        Some(ironsmith_core::ManaUsageRestriction::ActivateAbility)
    ));
}

#[test]
pub(super) fn rewrite_cast_or_activate_source_mana_restriction_parses() {
    let tokens = lex_line(
        "Spend this mana only to cast an Ally spell or activate an ability of an Ally source.",
        0,
    )
    .expect("rewrite lexer should classify cast-or-activate mana restriction");

    match parse_mana_usage_restriction_sentence_lexed(&tokens) {
        Some(ironsmith_core::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
            spell_filter,
            ability_source_filter,
        }) => {
            assert_eq!(spell_filter.subtypes, vec![Subtype::Ally]);
            assert_eq!(ability_source_filter.subtypes, vec![Subtype::Ally]);
        }
        other => panic!("expected cast-or-activate Ally restriction, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_cast_unlock_or_turn_face_up_mana_restriction_parses() {
    let tokens = lex_line(
        "Spend this mana only to cast an enchantment spell, unlock a door, or turn a permanent face up.",
        0,
    )
    .expect("rewrite lexer should classify cast/unlock/turn-face-up mana restriction");

    match parse_mana_usage_restriction_sentence_lexed(&tokens) {
        Some(ironsmith_core::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp {
            spell_filter,
        }) => {
            assert_eq!(spell_filter.card_types, vec![CardType::Enchantment]);
        }
        other => panic!("expected cast/unlock/turn-face-up restriction, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_cant_be_spent_mana_restriction_preserves_the_negative_transaction() {
    for text in [
        "This mana can't be spent to cast a nonartifact spell.",
        "This mana can't be spent to cast nonartifact spells.",
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify negative mana restriction");

        match parse_mana_usage_restriction_sentence_lexed(&tokens) {
            Some(ironsmith_core::ManaUsageRestriction::PaymentTransaction {
                restriction: Some(crate::ability::ManaPaymentPredicate::Not(forbidden)),
                on_spend,
            }) => {
                assert!(on_spend.is_empty(), "{text}");
                let crate::ability::ManaPaymentPredicate::All(parts) = forbidden.as_ref() else {
                    panic!(
                        "expected a compound forbidden cast transaction for {text}: {forbidden:?}"
                    );
                };
                assert!(parts.iter().any(|part| matches!(
                    part,
                    crate::ability::ManaPaymentPredicate::Purpose(
                        crate::ability::ManaPaymentPurpose::CastSpell
                    )
                )));
                assert!(parts.iter().any(|part| matches!(
                    part,
                    crate::ability::ManaPaymentPredicate::SourceMatches(filter)
                        if filter.excluded_card_types == vec![CardType::Artifact]
                )));
            }
            other => panic!("expected a negative payment transaction for {text:?}, got {other:?}"),
        }
    }
}

#[test]
pub(super) fn rewrite_hand_and_artifact_source_mana_restrictions_are_typed_transactions() {
    let hand = lex_line("This mana can't be spent to cast spells from your hand.", 0)
        .expect("hand restriction should lex");
    assert!(matches!(
        parse_mana_usage_restriction_sentence_lexed(&hand),
        Some(ironsmith_core::ManaUsageRestriction::PaymentTransaction {
            restriction: Some(crate::ability::ManaPaymentPredicate::Not(forbidden)),
            ref on_spend,
        }) if on_spend.is_empty()
            && matches!(
                forbidden.as_ref(),
                crate::ability::ManaPaymentPredicate::All(parts)
                    if parts.iter().any(|part| matches!(
                        part,
                        crate::ability::ManaPaymentPredicate::SourceMatches(filter)
                            if filter.zone == Some(crate::zone::Zone::Hand)
                                && filter.owner == Some(crate::target::PlayerFilter::You)
                    ))
            )
    ));

    let activation = lex_line(
        "Spend this mana only to activate abilities of artifact sources.",
        0,
    )
    .expect("artifact-source activation restriction should lex");
    assert!(matches!(
        parse_mana_usage_restriction_sentence_lexed(&activation),
        Some(ironsmith_core::ManaUsageRestriction::PaymentTransaction {
            restriction: Some(crate::ability::ManaPaymentPredicate::All(parts)),
            ref on_spend,
        }) if on_spend.is_empty()
            && parts.iter().any(|part| matches!(
                part,
                crate::ability::ManaPaymentPredicate::SourceMatches(filter)
                    if filter.card_types == vec![CardType::Artifact]
            ))
            && parts.iter().any(|part| matches!(
                part,
                crate::ability::ManaPaymentPredicate::AnyOf(purposes)
                    if purposes.iter().any(|purpose| matches!(
                        purpose,
                        crate::ability::ManaPaymentPredicate::Purpose(
                            crate::ability::ManaPaymentPurpose::ActivateAbility
                        )
                    )) && purposes.iter().any(|purpose| matches!(
                        purpose,
                        crate::ability::ManaPaymentPredicate::Purpose(
                            crate::ability::ManaPaymentPurpose::ActivateManaAbility
                        )
                    ))
            ))
    ));
}

#[test]
pub(super) fn rewrite_counter_removal_cost_binds_that_much_for_mana_addition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Jetfire-style ability")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "Remove one or more +1/+1 counters from among artifacts you control: Target player adds that much {C}. This mana can't be spent to cast nonartifact spells.",
        )
        .expect("counter-removal mana ability should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("AddScaledManaEffect") || debug.contains("AddManaAnyColor"),
        "expected parsed mana ability to retain scaled mana amount, got {debug}"
    );
    assert!(
        debug.contains("mana_usage_restrictions") && debug.contains("Artifact"),
        "expected parsed mana ability to carry artifact-only usage restriction, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_removed_counter_mana_scalars_keep_this_way_surface_hint() {
    for text in [
        "{T}, Remove any number of storage counters from this land: Add {B} for each storage counter removed this way.",
        "{T}, Remove any number of charge counters from this artifact: Add {B}, then add an additional {B} for each charge counter removed this way.",
    ] {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Removed Counter Mana Variant")
            .card_types(vec![CardType::Artifact])
            .parse_text(text)
            .expect("removed-counter mana scalar should parse");
        let debug = format!("{def:#?}");

        assert!(debug.contains("AddScaledManaEffect"), "{debug}");
        assert!(debug.contains("CountersRemovedThisWay"), "{debug}");
        assert!(debug.contains("X"), "{debug}");
    }
}

#[test]
pub(super) fn learn_keyword_line_lowers_to_real_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Learn Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("You gain 4 life.\nLearn.")
        .expect("learn keyword action should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("GainLifeEffect") && debug.contains("LearnEffect"),
        "expected learn to lower to a real effect, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_activation_line_attaches_special_mana_restriction_filters() {
    let tokens = lex_line(
        "{T}, Sacrifice this artifact: Add three mana of any one color. Spend this mana only to cast your commander.",
        0,
    )
    .expect("rewrite lexer should classify Jeweled Lotus-style activated line");

    let parsed = crate::activation_and_restrictions::parse_activated_line(&tokens)
        .expect("activated line should parse")
        .expect("activated line should produce an ability");

    match parsed.kind() {
        crate::model::CompilerAbilityKindCore::Activated(activated) => {
            let [ironsmith_core::ManaUsageRestriction::CastSpellMatching { filter, .. }] =
                activated.mana_usage_restrictions.as_slice()
            else {
                panic!(
                    "expected one commander mana usage restriction, got {:?}",
                    activated.mana_usage_restrictions
                );
            };
            assert!(filter.is_commander);
            assert_eq!(filter.owner, Some(crate::target::PlayerFilter::You));
        }
        other => panic!("expected activated ability, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_restriction_support_preserves_text_only_attack_conditions() {
    let attacked_restriction =
        super::super::grammar::restriction_facts::parse_activation_restriction_tokens(
            &lex_line(
                "Activate only once each turn and only if this creature attacked this turn",
                0,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        attacked_restriction.timing,
        Some(crate::ability::ActivationTiming::OncePerTurn)
    );
    assert_eq!(
        attacked_restriction.normalization,
        crate::model::compiler_semantic::ActivationRestrictionNormalizationFact::Residual(
            "only if this creature attacked this turn".to_string()
        )
    );
    assert!(matches!(
        attacked_restriction.text_only_condition,
        Some(PredicateAst::SourceAttackedThisTurn)
    ));

    let didnt_attack_restriction =
        super::super::grammar::restriction_facts::parse_activation_restriction_tokens(
            &lex_line(
                "Activate only if it didn't attack this turn and only once each turn",
                0,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        didnt_attack_restriction.timing,
        Some(crate::ability::ActivationTiming::OncePerTurn)
    );
    assert_eq!(
        didnt_attack_restriction.normalization,
        crate::model::compiler_semantic::ActivationRestrictionNormalizationFact::Residual(
            "activate only if it didn't attack this turn".to_string()
        )
    );
    assert!(didnt_attack_restriction.once_per_turn_after_other_restrictions);
    assert!(matches!(
        didnt_attack_restriction.text_only_condition,
        Some(PredicateAst::Not(inner))
            if matches!(inner.as_ref(), PredicateAst::SourceAttackedThisTurn)
    ));
}

#[test]
pub(super) fn rewrite_zone_counter_helpers_parse_put_or_remove_counter_modes() {
    let tokens = lex_line(
        "Put a +1/+1 counter on target creature or remove a counter from it",
        0,
    )
    .expect("rewrite lexer should classify put-or-remove counter clause");

    let parsed = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("put-or-remove counter clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PutOrRemoveCounters"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
}

#[test]
pub(super) fn rewrite_parse_lose_life_unless_you_attacked_this_turn_clause() {
    let tokens = lex_line("You lose 4 life unless you attacked this turn.", 0)
        .expect("rewrite lexer should classify life-loss unless clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("life-loss unless clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ControlFlow"), "{debug}");
    assert!(debug.contains("LoseLife"), "{debug}");
    assert!(debug.contains("YouAttackedThisTurn"), "{debug}");
}

#[test]
pub(super) fn rewrite_zone_counter_helpers_parse_multiple_counter_sentence() {
    let tokens = lex_line(
        "Put a +1/+1 counter and a flying counter on target creature",
        0,
    )
    .expect("rewrite lexer should classify multi-counter clause");

    let parsed = crate::effect_sentences::parse_sentence_put_multiple_counters_on_target(&tokens)
        .expect("multi-counter clause should parse");

    assert_eq!(parsed.as_ref().map(Vec::len), Some(2), "{parsed:?}");
}

#[test]
pub(super) fn rewrite_zone_counter_helpers_parse_for_each_spells_youve_cast_this_turn() {
    let tokens = lex_line(
        "Put a +1/+1 counter on target creature for each spell you've cast this turn.",
        0,
    )
    .expect("rewrite lexer should classify for-each spell-count counter clause");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("for-each spell-count counter clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("SpellsCastThisTurn"), "{debug}");
}

#[test]
pub(super) fn rewrite_zone_counter_helpers_parse_difference_counter_amount() {
    let tokens = lex_line(
        "Put a number of +1/+1 counters on this equal to the difference.",
        0,
    )
    .expect("rewrite lexer should classify difference counter clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("difference counter clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("Add(PowerOf(Tagged"), "{debug}");
    assert!(debug.contains("Scaled(PowerOf"), "{debug}");
    assert!(debug.contains("Source"), "{debug}");
    assert!(debug.contains("-1"), "{debug}");
}

#[test]
pub(super) fn rewrite_zone_counter_helpers_parse_equal_to_named_source_power_counter_amount() {
    let tokens = lex_line(
        "Put a number of +1/+1 counters equal to Jenova's power on up to one other target creature.",
        0,
    )
    .expect("rewrite lexer should classify source-power counter clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("source-power counter clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("PowerOf(Source)"), "{debug}");
    assert!(debug.contains("target_count: Some"), "{debug}");
    assert!(debug.contains("max: Some(1)"), "{debug}");
}

#[test]
pub(super) fn rewrite_zone_counter_helpers_parse_target_before_exiled_card_mana_value_amount() {
    let tokens = lex_line(
        "Put a number of +1/+1 counters on target creature you control equal to the mana value of the exiled card.",
        0,
    )
    .expect("rewrite lexer should classify source-exiled mana-value counter clause");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("source-exiled mana-value counter clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("__source_exiled__"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
}

#[test]
pub(super) fn the_aesir_escape_valhalla_lowers_source_exiled_counter_and_return_pair() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "The Aesir Escape Valhalla")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .parse_text(
            "I — Exile a permanent card from your graveyard. You gain life equal to its mana value.\n\
             II — Put a number of +1/+1 counters on target creature you control equal to the mana value of the exiled card.\n\
             III — Return this Saga and the exiled card to their owner's hand.",
        )
        .expect("The Aesir Escape Valhalla should parse strictly");

    let debug = format!("{def:#?}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("__source_exiled__"), "{debug}");
    assert!(!debug.contains("__it__"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("ReturnToHandEffect"), "{debug}");
    assert!(debug.contains("source: true"), "{debug}");
    assert!(debug.contains("any_of"), "{debug}");
}

#[test]
pub(super) fn saga_source_exile_then_return_uses_battlefield_move_not_graveyard_return() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Source Blink Saga")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text("I — Exile this Saga, then return it to the battlefield.")
        .expect("source-exile return should parse strictly");

    let debug = format!("{def:#?}");
    assert!(debug.contains("__source_exiled__"), "{debug}");
    assert!(debug.contains("MoveToZoneEffect"), "{debug}");
    assert!(
        !debug.contains("ReturnFromGraveyardToBattlefieldEffect"),
        "source-exiled return should not lower as a graveyard return: {debug}"
    );
}

#[test]
pub(super) fn rewrite_triggered_it_damage_source_binds_to_triggering_object() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Warstorm Surge Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a creature you control enters, it deals damage equal to its power to any target.",
        )
        .expect("triggered it-damage source should parse");
    let rendered = format!("{def:#?}");
    let compact = rendered.split_whitespace().collect::<String>();

    assert!(
        compact.contains("TagTriggeringObjectEffect")
            && compact.contains("ExecuteWithSourceEffect")
            && compact.contains("source:Tagged")
            && compact.contains("TagKey(\"triggering\"")
            && compact.contains("PowerOf")
            && compact.contains("spec:Tagged"),
        "expected 'it' to bind to the triggering object as damage source, got {rendered}"
    );
    assert!(
        !compact.contains("zone:Some(Hand)"),
        "triggered 'it' should not fall back to a hand-card antecedent, got {rendered}"
    );
}

#[test]
pub(super) fn dealt_damage_trigger_binds_it_to_damaged_object_and_tests_player_result() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dealt Damage Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature is dealt damage, it deals that much damage to any other target. If a player is dealt damage this way, they can't gain life for the rest of the game.",
        )
        .expect("dealt-damage trigger and player-result followup should parse");
    let rendered = format!("{def:#?}");
    let compact = rendered.split_whitespace().collect::<String>();

    assert!(
        compact.contains("TagTriggeringDamageTargetEffect")
            && compact.contains("TagKey(\"damaged\"")
            && compact.contains("ExecuteWithSourceEffect")
            && compact.contains("source:Tagged")
            && compact.contains("predicate:DealtDamageToPlayer"),
        "expected the damaged object to deal damage and the followup to test player damage, got {rendered}"
    );
    assert!(
        !compact.contains("predicate:Happened"),
        "player-damage followup must not accept an arbitrary successful damage result: {rendered}"
    );
}

#[test]
pub(super) fn rewrite_zone_counter_helpers_keep_trailing_if_counter_clause_after_structure_cutover()
{
    let tokens = lex_line("Put a +1/+1 counter on target creature if it's white.", 0)
        .expect("rewrite lexer should classify conditional counter clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("counter clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected conditional put-counters clause, got {parsed:?}");
    };
    let (predicate, if_true, if_false) = super::shard_01::conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::ItMatches(_)
    ));
    assert!(matches!(
        if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::PutCounters { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_verb_handlers_keep_trailing_if_counter_clause_after_structure_cutover() {
    let tokens = lex_line("Counter target spell if it's white.", 0)
        .expect("rewrite lexer should classify conditional counter spell clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("counter spell clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected conditional counter clause, got {parsed:?}");
    };
    let (predicate, if_true, if_false) = super::shard_01::conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::ItMatches(_)
    ));
    assert!(matches!(
        if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Counter { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_verb_handlers_keep_trailing_if_damage_clause_after_structure_cutover() {
    let tokens = lex_line(
        "This creature deals 3 damage to target creature if it's white.",
        0,
    )
    .expect("rewrite lexer should classify conditional damage clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("damage clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected conditional damage clause, got {parsed:?}");
    };
    let (predicate, if_true, if_false) = super::shard_01::conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::ItMatches(_)
    ));
    assert!(matches!(
        if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::DealDamage { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_if_clause_binds_that_enchantment_and_created_token_references() {
    let tokens = lex_line(
        "If that enchantment is an Aura, you may attach it to the token.",
        0,
    )
    .expect("rewrite lexer should classify conditional attach clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("conditional attach clause should parse");

    let [
        crate::cards::builders::EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected conditional attach clause, got {parsed:?}");
    };
    assert!(if_false.is_empty());
    let crate::cards::builders::PredicateAst::TaggedMatches(tag, filter) = predicate else {
        panic!("expected that-enchantment predicate to bind triggering object, got {predicate:?}");
    };
    assert_eq!(tag.as_str(), "triggering");
    assert_eq!(filter.subtypes, vec![crate::Subtype::Aura]);

    let effects = match if_true.as_slice() {
        [crate::cards::builders::EffectAst::May { effects }]
        | [
            crate::cards::builders::EffectAst::MayByPlayer {
                player: crate::cards::builders::PlayerAst::You,
                effects,
            },
        ] => effects,
        other => panic!("expected may-wrapper around attach effect, got {other:?}"),
    };
    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Attach { object, target },
                ..
            },
        ),
    ] = effects.as_slice()
    else {
        panic!("expected attach effect, got {effects:?}");
    };
    assert!(
        matches!(
            object,
            crate::cards::builders::TargetAst::Tagged(tag, _) if tag.as_str() == "triggering"
        ),
        "expected attachment object to bind the triggering enchantment, got {object:?}"
    );
    assert!(
        matches!(
            target,
            crate::cards::builders::TargetAst::Tagged(tag, _)
                if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        ),
        "expected attachment destination to bind the created token, got {target:?}"
    );
}
