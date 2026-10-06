//! Complete noun-domain readings used when a mixed player/object target owns
//! punctuation. The general relational filter scanner is not a consumption
//! contract: some of its legacy readings accept recognized words anywhere.
//! Prove the complete surface here before using its established semantics.

use super::*;
use crate::lexer::TokenKind;

/// Read the bounded object domain of a leading-player target union. Every
/// token must belong to a simple selector, a complete player relation, or a
/// list separator. Unsupported relational forms remain errors, rather than
/// being reinterpreted as a shorter target or as executable coordination.
pub(crate) fn parse_complete_mixed_target_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    if !complete_object_domain(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported complete mixed target object domain: {}",
            crate::lexer::render_token_slice(tokens),
        )));
    }
    let (filter, loss) = crate::parse_loss::capture(|| {
        crate::object_filters::parse_object_filter(tokens, other)
    });
    let filter = filter?;
    if loss.is_lossy() {
        return Err(CardTextError::ParseError(format!(
            "mixed target object domain lost content: {}",
            loss.reasons_text(),
        )));
    }
    Ok(filter)
}

fn complete_object_domain(tokens: &[OwnedLexToken]) -> bool {
    // Word projection must never erase quotes, parentheticals, sentence or
    // action separators. Commas are consumed by the list grammar below.
    if tokens.is_empty() || tokens.iter().any(|token| {
        token.kind != TokenKind::Comma && token.as_word().is_none()
    }) {
        return false;
    }
    if !tokens.iter().any(OwnedLexToken::is_comma) && complete_object_arm(tokens) {
        return true;
    }
    // `slice::split` retains empty arms. The general list utilities omit
    // those, which cannot establish complete grammar ownership here.
    let arms = if tokens.iter().any(OwnedLexToken::is_comma) {
        tokens.split(OwnedLexToken::is_comma).collect::<Vec<_>>()
    } else {
        tokens.split(|token| token.is_word("or") || token.is_word("and/or"))
            .collect::<Vec<_>>()
    };
    arms.len() >= 2 && arms.iter().enumerate().all(|(index, arm)| {
        let arm = if index > 0 && arm.first().is_some_and(|token| {
            token.is_word("or") || token.is_word("and/or")
        }) {
            &arm[1..]
        } else {
            arm
        };
        // Each comma arm can itself contain a disjunction. Recursing only
        // through shorter slices guarantees progress and owns every token.
        arm.len() < tokens.len() && complete_object_domain(arm)
    })
}

fn complete_object_arm(tokens: &[OwnedLexToken]) -> bool {
    let tokens = if tokens.first().is_some_and(|token| {
        token.is_word("a") || token.is_word("an") || token.is_word("the")
    }) {
        &tokens[1..]
    } else {
        tokens
    };
    if tokens.is_empty() || tokens.iter().any(|token| token.as_word().is_none()) {
        return false;
    }
    let is_connector = |token: &OwnedLexToken| token.is_any_word(&["and", "or", "and/or"]);
    if tokens.first().is_some_and(is_connector)
        || tokens.last().is_some_and(is_connector)
        || tokens.windows(2).any(|pair| is_connector(&pair[0]) && is_connector(&pair[1]))
    {
        return false;
    }
    // The simple reader removes articles and `instead` for its legacy
    // callers. Here articles are admitted only at an owned noun/relation
    // head, and `instead` remains the enclosing instruction's connective.
    let has_discarded_word = |tokens: &[OwnedLexToken]| tokens.iter().any(|token| {
        token.is_any_word(&["a", "an", "the", "instead"])
    });
    if !has_discarded_word(tokens) && parse_simple_object_filter_lexed(tokens, false).is_some() {
        return true;
    }
    (1..tokens.len()).any(|split| {
        let (head, relation) = tokens.split_at(split);
        if has_discarded_word(head) {
            return false;
        }
        let Some(mut filter) = parse_simple_object_filter_lexed(head, false) else {
            return false;
        };
        let relation = if relation.first().is_some_and(|token| {
            token.is_word("a") || token.is_word("an") || token.is_word("the")
        }) {
            &relation[1..]
        } else {
            relation
        };
        if relation.is_empty() || has_discarded_word(relation) {
            return false;
        }
        let words = crate::lexer::parser_token_word_refs(relation);
        try_apply_player_relation_clause(&mut filter, &words, &PlayerFilter::IteratedPlayer)
            .or_else(|| try_apply_passive_player_relation_clause(
                &mut filter, &words, &PlayerFilter::IteratedPlayer,
            ))
            .is_some_and(|consumed| consumed == words.len())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_mixed_object_reader_owns_relative_suffixes_and_list_separators() {
        for text in [
            "creature an opponent controls",
            "creature you control or planeswalker an opponent controls",
            "artifact, creature, or planeswalker an opponent controls",
            "planeswalker, or Sliver creature",
            "creature controlled by your opponents",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse_complete_mixed_target_object_filter(&tokens, false).is_ok(), "{text}");
        }
    }

    #[test]
    fn complete_mixed_object_reader_rejects_unowned_tokens_even_if_scanner_knows_a_noun() {
        for text in [
            "creature an opponent controls unsupported",
            "creature an opponent controls and draw a card",
            "creature an opponent controls or draw a card",
            "creature an opponent controls, then draw a card",
            "creature an opponent controls with \"creature\"",
            "\"creature an opponent controls\"",
            "creature an opponent controls the",
            "creature an opponent controls instead",
            "creature, , or planeswalker",
            "creature or",
            "or creature",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse_complete_mixed_target_object_filter(&tokens, false).is_err(), "{text}");
        }
    }
}
