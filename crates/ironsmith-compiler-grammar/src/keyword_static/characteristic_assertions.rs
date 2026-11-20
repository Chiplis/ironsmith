use super::*;
use crate::grammar::effects::characteristic_assertions::{self as shape, Property};
pub(super) fn parse_supertype_assertion_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // "Aeve isn't legendary if it's a token": the assertion holds only for
    // the token incarnation (storm copies that became tokens, CR 702.40a /
    // 707.10f), so the condition narrows the subject to tokens.
    const TOKEN_CONDITIONS: &[&[&str]] = &[
        &["if", "it's", "a", "token"],
        &["if", "it’s", "a", "token"],
        &["if", "its", "a", "token"],
        &["if", "it", "s", "a", "token"],
        &["if", "it", "is", "a", "token"],
    ];
    let positions = crate::lexer::parser_token_word_positions(tokens);
    let words = positions.iter().map(|(_, word)| *word).collect::<Vec<_>>();
    let token_condition = TOKEN_CONDITIONS
        .iter()
        .find(|tail| words.len() > tail.len() && words.ends_with(tail));
    let (tokens, only_tokens) = match token_condition {
        Some(tail) => {
            let cut = words.len() - tail.len();
            crate::word_claims::claim_words(&words[cut..]);
            (&tokens[..positions[cut].0], true)
        }
        None => (tokens, false),
    };
    let Some(fact) = shape::parse(tokens) else {
        return Ok(None);
    };
    let Property::Supertype(kind) = fact.property else {
        return Ok(None);
    };
    if tokens
        .first()
        .is_some_and(|t| t.is_any_word(&["if", "as", "when", "whenever", "until"]))
        || fact.subject.iter().any(|t| t.is_word("target"))
    {
        return Ok(None);
    }
    // Existing Basic productions retain their route.
    if !fact.remove && kind == Supertype::Basic {
        return Ok(None);
    }
    let mut filter = parse_object_filter_lexed(fact.subject, false)?;
    if only_tokens {
        filter.token = true;
    }
    Ok(Some(if fact.remove {
        StaticAbility::remove_supertypes(filter, vec![kind])
    } else {
        StaticAbility::add_supertypes(filter, vec![kind])
    }))
}
