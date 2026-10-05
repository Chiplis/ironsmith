//! Resolving, explicitly timed combat rules do not grant creature abilities.
use crate::cards::builders::{CardTextError, EffectAst};
use crate::effect::{Restriction, Until};
use crate::lexer::{OwnedLexToken, TokenKind};

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    if tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Quote | TokenKind::Colon))
    {
        return Ok(None);
    }
    let (duration, body) = if let Some(prefix) =
        crate::grammar::leaf::parse_leaf_turn_duration_prefix_tokens(tokens)
    {
        (prefix.duration, prefix.rest)
    } else if let Some(suffix) =
        crate::grammar::leaf::parse_leaf_turn_duration_suffix_tokens(tokens)
    {
        (suffix.duration, suffix.rest)
    } else {
        return Ok(None);
    };
    let body = crate::util::trim_edge_punctuation_tokens(body);
    let Some((index, (), rest)) = crate::grammar::primitives::find_prefix(body, || {
        use winnow::Parser;
        winnow::combinator::alt((
            crate::grammar::primitives::phrase(&["attack", "each", "combat", "if", "able"]),
            crate::grammar::primitives::phrase(&["attacks", "each", "combat", "if", "able"]),
        ))
        .void()
    }) else {
        return Ok(None);
    };
    if !crate::util::trim_edge_punctuation_tokens(rest).is_empty() || index == 0 {
        return Ok(None);
    }
    let subject = &body[..index];
    if subject
        .iter()
        .any(|token| token.is_any_word(&["target", "that", "it", "chosen"]))
    {
        return Ok(None);
    }
    let filter = crate::object_filters::parse_object_filter(subject, false)?;
    if !filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        return Ok(None);
    }
    let duration = match duration {
        crate::grammar::leaf::LeafTurnDurationPhrase::ThisTurn
        | crate::grammar::leaf::LeafTurnDurationPhrase::UntilEndOfTurn => Until::EndOfTurn,
        crate::grammar::leaf::LeafTurnDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        crate::grammar::leaf::LeafTurnDurationPhrase::UntilYourNextTurnEnd => {
            Until::YourNextTurnEnd
        }
    };
    Ok(Some(EffectAst::subject_verb_cant(
        Restriction::must_attack(filter),
        duration,
        None,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_temporary_rule_is_distinct_from_an_untimed_or_quoted_ability() {
        for text in [
            "Until your next turn, creatures your opponents control attack each combat if able.",
            "Creatures your opponents control attack each combat if able until your next turn.",
        ] {
            let effect = parse(&crate::lexer::lex_line(text, 0).unwrap())
                .unwrap()
                .unwrap();
            let debug = format!("{effect:?}");
            assert!(debug.contains("MustAttack"));
            assert!(debug.contains("YourNextTurn"));
            assert!(!debug.contains("GrantAbilities"));
        }
        for text in [
            "Creatures your opponents control attack each combat if able.",
            "Until your next turn, creatures your opponents control have \"This creature attacks each combat if able.\"",
            "Until your next turn, creatures your opponents control attack each combat if able and can't block.",
            "Until your next turn, target creature attacks each combat if able.",
        ] {
            assert!(
                parse(&crate::lexer::lex_line(text, 0).unwrap())
                    .unwrap()
                    .is_none(),
                "{text}"
            );
        }
    }
}
