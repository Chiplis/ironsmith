//! A die-result table whose rows only fix X for the sentences between the
//! roll and the table.
//!
//! "{4}, {T}: Roll a d20. Each opponent exiles ... . You may cast up to X ...
//! . 1—9 | X is one. 10—19 | X is two. 20 | X is three." (Wand of Wonder):
//! the row the result selects (CR 706.2) decides X, and the sentences printed
//! between the roll and the table use that X. Each row becomes the result
//! branch that runs those sentences with its X (an `EffectAst::BindX`), so
//! exactly one copy of them runs.

use crate::cards::builders::{CardTextError, ConditionalEffectAst, EffectAst, OwnedLexToken};
use crate::effect::Value;
use crate::grammar::primitives;
use crate::lexer::{TokenKind, split_lexed_sentences};

/// The value a die-result row assigns to X ("X is one."), if that is all
/// the row says.
pub(crate) fn die_row_x_assignment(row_body: &[OwnedLexToken]) -> Option<Value> {
    let body = match row_body.split_last() {
        Some((last, rest)) if last.kind == TokenKind::Period => rest,
        _ => row_body,
    };
    let ((_, _), value_tokens) =
        primitives::parse_prefix(body, (primitives::kw("x"), primitives::kw("is")))?;
    let (value, used) = crate::util::parse_value(value_tokens)?;
    (used == value_tokens.len() && matches!(value.unhinted(), Value::Fixed(_)))
        .then(|| value.unhinted().clone())
}

/// A die-result row ("N—M | X is K.") that only assigns X.
pub(crate) fn die_x_row(
    sentence: &[OwnedLexToken],
) -> Option<(crate::cards::builders::IfResultPredicate, Value)> {
    let (predicate, body) =
        crate::grammar::structure::split_leading_numeric_result_prefix_lexed(sentence)?;
    let value = die_row_x_assignment(body)?;
    Some((predicate, value))
}

fn starts_with_roll(sentence: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(sentence, primitives::kw("roll")).is_some()
}

fn mentions_x(sentence: &[OwnedLexToken]) -> bool {
    sentence.iter().any(|token| token.is_word("x"))
}

/// Whether a roll in `owner` is followed by sentences that read X, so a
/// following X-assignment row is that roll's result table.
pub(crate) fn owner_rolls_before_x_sentences(owner: &[OwnedLexToken]) -> bool {
    let sentences = split_lexed_sentences(owner);
    let Some(roll) = sentences.iter().position(|sentence| starts_with_roll(sentence)) else {
        return false;
    };
    sentences[roll + 1..]
        .iter()
        .any(|sentence| die_x_row(sentence).is_none() && mentions_x(sentence))
}

fn offset_in(outer: &[OwnedLexToken], inner: &[OwnedLexToken]) -> usize {
    (inner.as_ptr() as usize - outer.as_ptr() as usize) / std::mem::size_of::<OwnedLexToken>()
}

/// `[..., roll, body..., X rows...]` with every trailing row assigning X: the
/// roll keeps its place, and each row runs the body with its X.
pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let rows_start = sentences
        .iter()
        .rposition(|sentence| die_x_row(sentence).is_none())
        .map_or(0, |last_other| last_other + 1);
    if rows_start == sentences.len() {
        return Ok(None);
    }
    let Some(roll) = sentences[..rows_start]
        .iter()
        .rposition(|sentence| starts_with_roll(sentence))
    else {
        return Ok(None);
    };
    let body = &sentences[roll + 1..rows_start];
    if body.is_empty() || !body.iter().any(|sentence| mentions_x(sentence)) {
        return Ok(None);
    }
    let body_start = offset_in(tokens, body[0]);
    let rows_offset = offset_in(tokens, sentences[rows_start]);
    let head_effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens[..body_start])?;
    let body_effects =
        crate::effect_sentences::parse_effect_sentences_lexed(&tokens[body_start..rows_offset])?;
    if head_effects.is_empty() || body_effects.is_empty() {
        return Ok(None);
    }
    let mut effects = head_effects;
    for row in &sentences[rows_start..] {
        let Some((predicate, value)) = die_x_row(row) else {
            return Ok(None);
        };
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate,
            effects: vec![EffectAst::BindX {
                value,
                effects: body_effects.clone(),
            }],
        }));
    }
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexed(text: &str) -> Vec<OwnedLexToken> {
        crate::lexer::lex_line(text, 0).unwrap()
    }

    #[test]
    fn x_rows_assign_only_x() {
        assert_eq!(
            die_x_row(&lexed("1—9 | X is one.")).map(|(_, value)| value),
            Some(Value::Fixed(1))
        );
        assert!(die_x_row(&lexed("1—9 | Draw a card.")).is_none());
    }

    #[test]
    fn owner_needs_a_roll_before_an_x_sentence() {
        assert!(owner_rolls_before_x_sentences(&lexed(
            "Roll a d20. You may cast up to X instant spells from your hand."
        )));
        assert!(!owner_rolls_before_x_sentences(&lexed("Roll a d20. Draw a card.")));
    }
}
