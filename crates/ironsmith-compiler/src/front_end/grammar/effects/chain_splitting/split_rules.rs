use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::recognition::{
    comma_boundary_facts, preserve_and_reason, starts_with_each_player_or_opponent,
    then_followup_facts,
};

pub(crate) fn split_effect_chain_on_and_tokens(
    tokens: &[OwnedLexToken],
    extended: bool,
) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut input = LexStream::new(tokens);
    let mut inside_quotes = false;
    while !input.is_empty() {
        let idx = tokens.len().saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            break;
        };
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }
        if !is_word(token, "and") {
            continue;
        }
        let current = trim_lexed_commas(tokens.get(start..idx).unwrap_or_default());
        let remaining = trim_lexed_commas(tokens.get(idx + 1..).unwrap_or_default());
        if preserve_and_reason(current, remaining, extended).is_some() {
            continue;
        }
        if !current.is_empty() {
            segments.push(current);
        }
        start = idx + 1;
    }
    let tail = trim_lexed_commas(tokens.get(start..).unwrap_or_default());
    if !tail.is_empty() {
        segments.push(tail);
    }
    segments
}

pub(crate) fn split_segments_on_comma_then_tokens<'a>(
    segments: Vec<&'a [OwnedLexToken]>,
    mut is_ability_head: impl FnMut(&[OwnedLexToken]) -> bool,
) -> Vec<&'a [OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        // A source sentence may contain more than one authored `, then`
        // boundary. Keep splitting the unconsumed tail so an n-ary ordered
        // chain does not leave its final actions inside a prefix-tolerant
        // parser for the second arm.
        let mut remaining = segment;
        while let Some(split) = find_then_split(remaining, &mut is_ability_head) {
            let first = trim_lexed_commas(remaining.get(..split.separator_idx).unwrap_or_default());
            let second = trim_lexed_commas(remaining.get(split.then_idx + 1..).unwrap_or_default());
            if !first.is_empty() {
                result.push(first);
            }
            if second.is_empty() || second.len() >= remaining.len() {
                remaining = &[];
                break;
            }
            remaining = second;
        }
        if !remaining.is_empty() {
            result.push(remaining);
        }
    }
    result
}

/// Return whether the generic chain grammar accepts an authored `, then`
/// boundary. A bare same-sentence `then`, sentence-leading `Then`, and quoted
/// text remain distinct surfaces.
pub(crate) fn has_explicit_comma_then_boundary_tokens(
    tokens: &[OwnedLexToken],
    mut is_ability_head: impl FnMut(&[OwnedLexToken]) -> bool,
) -> bool {
    find_then_split(tokens, &mut is_ability_head).is_some_and(|split| split.explicit_comma_then)
}

/// Return whether the source lexically contains an authored `, then`
/// connective outside quoted rules text.
///
/// This is intentionally broader than [`has_explicit_comma_then_boundary_tokens`].
/// The latter answers whether the generic chain splitter can safely separate
/// the clauses before parsing, so it rejects some pronoun-bearing tails. Once
/// a specialist has already produced multiple typed effects, those safety
/// heuristics must not erase the connective's presentation provenance.
pub(crate) fn has_authored_comma_then_surface_tokens(tokens: &[OwnedLexToken]) -> bool {
    let mut inside_quotes = false;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes
            && token.kind == TokenKind::Comma
            && tokens
                .get(idx + 1)
                .is_some_and(|next| is_word(next, "then"))
        {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Copy)]
struct ThenSplit {
    separator_idx: usize,
    then_idx: usize,
    explicit_comma_then: bool,
}

fn find_then_split(
    segment: &[OwnedLexToken],
    is_ability_head: &mut impl FnMut(&[OwnedLexToken]) -> bool,
) -> Option<ThenSplit> {
    let starts_with_for_each = starts_with_each_player_or_opponent(segment);
    let mut input = LexStream::new(segment);
    let mut inside_quotes = false;
    while !input.is_empty() {
        let idx = segment.len().saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            break;
        };
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }
        let (then_idx, explicit_comma_then) = if token.kind == TokenKind::Comma
            && segment
                .get(idx + 1)
                .is_some_and(|next| is_word(next, "then"))
        {
            (idx + 1, true)
        } else if is_word(token, "then") {
            (idx, false)
        } else {
            continue;
        };
        let before = trim_lexed_commas(segment.get(..idx).unwrap_or_default());
        let after = trim_lexed_commas(segment.get(then_idx + 1..).unwrap_or_default());
        let facts = then_followup_facts(before, after, starts_with_for_each);
        if facts.should_split(is_ability_head(after)) {
            return Some(ThenSplit {
                separator_idx: idx,
                then_idx,
                explicit_comma_then,
            });
        }
    }
    None
}

pub(crate) fn split_segments_on_comma_effect_head_tokens(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        let mut start = 0usize;
        let mut split_any = false;
        let mut input = LexStream::new(segment);
        let mut inside_quotes = false;
        while !input.is_empty() {
            let idx = segment.len().saturating_sub(input.len());
            let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
            let Ok(token) = parsed else {
                break;
            };
            if token.kind == TokenKind::Quote {
                inside_quotes = !inside_quotes;
                continue;
            }
            if inside_quotes || token.kind != TokenKind::Comma {
                continue;
            }
            let before = trim_lexed_commas(segment.get(start..idx).unwrap_or_default());
            let after = trim_lexed_commas(segment.get(idx + 1..).unwrap_or_default());
            if before.is_empty() || after.is_empty() {
                continue;
            }
            let facts = comma_boundary_facts(before, after);
            if facts.preserve_boundary {
                continue;
            }
            if facts.before_has_verb && facts.after_starts_effect {
                if std::env::var_os("IRONSMITH_CHOICE_TRACE").is_some() {
                    eprintln!(
                        "comma-effect-head split: before='{}' after='{}'",
                        crate::token_word_refs(before).join(" "),
                        crate::token_word_refs(after).join(" ")
                    );
                }
                result.push(before);
                start = idx + 1;
                split_any = true;
            }
        }
        if split_any {
            let tail = trim_lexed_commas(segment.get(start..).unwrap_or_default());
            if !tail.is_empty() {
                result.push(tail);
            }
        } else {
            result.push(segment);
        }
    }
    result
}

fn is_word(token: &OwnedLexToken, expected: &'static str) -> bool {
    let mut input = LexStream::new(std::slice::from_ref(token));
    (
        super::super::super::primitives::kw(expected),
        super::super::super::primitives::end_of_block(),
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}

#[cfg(test)]
mod tests {
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
            crate::runtime_backend::front_end::lexer::parser_token_word_refs(segments[0]),
            ["reveal", "the", "top", "card", "of", "their", "library"]
        );
        assert_eq!(
            crate::runtime_backend::front_end::lexer::parser_token_word_refs(segments[1]),
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
            .map(|segment| {
                crate::runtime_backend::front_end::lexer::parser_token_word_refs(segment)
            })
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
}
