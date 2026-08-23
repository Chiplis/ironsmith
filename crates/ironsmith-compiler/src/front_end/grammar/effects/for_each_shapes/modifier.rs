use winnow::combinator::alt;
use winnow::prelude::*;

use crate::ConditionExpr;
use crate::effect::Until;
use crate::grammar::{leaf, permission_shapes, primitives};
use crate::lexer::{OwnedLexToken, TokenKind};
use crate::util::trim_edge_punctuation_tokens;

#[derive(Debug, Clone, Copy)]
pub enum ModifierTailAction<'a> {
    Complete,
    DynamicForEach(&'a [OwnedLexToken]),
    WhereX(&'a [OwnedLexToken]),
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct ModifierTailShape<'a> {
    pub duration: Until,
    pub condition: Option<ConditionExpr>,
    pub action: ModifierTailAction<'a>,
}

#[derive(Debug, Clone)]
pub struct FixedPtAlternativeShape<'a> {
    pub first_modifier: OwnedLexToken,
    pub second_modifier: OwnedLexToken,
    pub trailing_tokens: &'a [OwnedLexToken],
}

fn normalized_pt_modifier_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(OwnedLexToken, &[OwnedLexToken])> {
    let first = tokens.first()?;
    if leaf::parse_leaf_pt_modifier_values_complete(first.parser_text()).is_ok() {
        return Some((first.clone(), &tokens[1..]));
    }

    let sign = match first.kind {
        TokenKind::Dash => "-",
        TokenKind::Plus => "+",
        _ => return None,
    };
    let unsigned = tokens.get(1)?.as_word()?;
    let modifier = format!("{sign}{unsigned}");
    leaf::parse_leaf_pt_modifier_values_complete(&modifier).ok()?;
    Some((OwnedLexToken::word(modifier, first.span()), &tokens[2..]))
}

pub fn parse_fixed_pt_alternative_shape(
    tokens: &[OwnedLexToken],
) -> Option<FixedPtAlternativeShape<'_>> {
    let (first_modifier, rest) = normalized_pt_modifier_prefix(tokens)?;
    let (or_token, rest) = rest.split_first()?;
    if or_token.as_word() != Some("or") {
        return None;
    }
    let (second_modifier, trailing_tokens) = normalized_pt_modifier_prefix(rest)?;
    Some(FixedPtAlternativeShape {
        first_modifier,
        second_modifier,
        trailing_tokens,
    })
}

fn contains_any(tokens: &[OwnedLexToken], words: &[&'static str]) -> bool {
    words
        .iter()
        .any(|word| primitives::contains_word(tokens, word))
}

fn duration_prefix(tokens: &[OwnedLexToken]) -> (Until, &[OwnedLexToken]) {
    let Some(parsed) = leaf::parse_leaf_restriction_duration_prefix_tokens(tokens) else {
        return (Until::EndOfTurn, tokens);
    };
    let duration = match parsed.duration {
        leaf::LeafDurationPhrase::UntilEndOfTurn => Until::EndOfTurn,
        leaf::LeafDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        leaf::LeafDurationPhrase::UntilEndOfCombat => Until::EndOfCombat,
        _ => return (Until::EndOfTurn, tokens),
    };
    (duration, trim_edge_punctuation_tokens(parsed.rest))
}

fn is_eot_tail(tokens: &[OwnedLexToken]) -> bool {
    leaf::parse_leaf_restriction_duration_prefix_tokens(tokens).is_some_and(|parsed| {
        parsed.duration == leaf::LeafDurationPhrase::UntilEndOfTurn
            && trim_edge_punctuation_tokens(parsed.rest).is_empty()
    })
}

fn accepted_keyword_tail(tokens: &[OwnedLexToken]) -> bool {
    let starts_and = primitives::parse_prefix(tokens, primitives::kw("and")).is_some();
    if !starts_and {
        return false;
    }
    let has_gain = contains_any(tokens, &["gain", "gains", "has", "have"]);
    let has_keyword = contains_any(tokens, &["trample", "haste", "first", "strike", "infect"]);
    (has_gain && has_keyword && primitives::has_phrase(tokens, &["until", "end", "of", "turn"]))
        || contains_any(tokens, &["control", "controls"])
}

fn accepted_alternative_modifier(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::kw("or")) else {
        return false;
    };
    let Some((_, tail)) = normalized_pt_modifier_prefix(rest) else {
        return false;
    };
    let tail = trim_edge_punctuation_tokens(tail);
    tail.is_empty() || is_eot_tail(tail)
}

fn accepted_fixed_tail(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::exact_tokens(tokens, &["instead"])
        || primitives::parse_prefix(tokens, primitives::phrase(&["instead", "if"])).is_some()
        || permission_shapes::exact_tokens(
            tokens,
            &["and", "must", "be", "blocked", "this", "turn", "if", "able"],
        )
        || permission_shapes::exact_tokens(
            tokens,
            &["and", "cant", "be", "blocked", "this", "turn"],
        )
        || accepted_keyword_tail(tokens)
        || accepted_alternative_modifier(tokens)
}

pub fn parse_modifier_tail_shape(tokens: &[OwnedLexToken]) -> ModifierTailShape<'_> {
    let after_modifier = trim_edge_punctuation_tokens(tokens.get(1..).unwrap_or_default());
    let (mut duration, tail) = duration_prefix(after_modifier);
    let tail = trim_edge_punctuation_tokens(tail);
    if tail.is_empty() || accepted_fixed_tail(tail) {
        return ModifierTailShape {
            duration,
            condition: None,
            action: ModifierTailAction::Complete,
        };
    }
    if leaf::parse_leaf_conditional_duration_kind_tokens(tail)
        == Some(leaf::LeafConditionalDurationKind::SourceRemainsTapped)
    {
        duration = Until::SourceUntaps;
        return ModifierTailShape {
            duration,
            condition: Some(ConditionExpr::SourceIsTapped),
            action: ModifierTailAction::Complete,
        };
    }
    if primitives::parse_prefix(
        tail,
        alt((
            primitives::phrase(&["for", "each"]),
            primitives::kw("each").void(),
        ))
        .void(),
    )
    .is_some()
    {
        return ModifierTailShape {
            duration,
            condition: None,
            action: ModifierTailAction::DynamicForEach(tail),
        };
    }
    if primitives::parse_prefix(tail, primitives::phrase(&["where", "x", "is"])).is_some() {
        return ModifierTailShape {
            duration,
            condition: None,
            action: ModifierTailAction::WhereX(tail),
        };
    }
    ModifierTailShape {
        duration,
        condition: None,
        action: ModifierTailAction::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn classifies_duration_and_dynamic_tails() {
        let timed = lex_line("+2/+2 until your next turn", 0).unwrap();
        assert_eq!(
            parse_modifier_tail_shape(&timed).duration,
            Until::YourNextTurn
        );

        let dynamic = lex_line("+1/+1 for each creature you control", 0).unwrap();
        assert!(matches!(
            parse_modifier_tail_shape(&dynamic).action,
            ModifierTailAction::DynamicForEach(_)
        ));
    }

    #[test]
    fn splits_two_signed_pt_modifiers_and_their_shared_tail() {
        let tokens = lex_line("+1/-1 or -1/+1 until end of turn", 0).unwrap();
        let shape = parse_fixed_pt_alternative_shape(&tokens)
            .expect("inline P/T alternative should have two modifier branches");

        assert_eq!(shape.first_modifier.parser_text(), "+1/-1");
        assert_eq!(shape.second_modifier.parser_text(), "-1/+1");
        assert!(is_eot_tail(shape.trailing_tokens));
    }
}
