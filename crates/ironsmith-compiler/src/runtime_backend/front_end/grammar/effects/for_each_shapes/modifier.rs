use winnow::combinator::alt;
use winnow::prelude::*;

use crate::ConditionExpr;
use crate::effect::Until;
use crate::runtime_backend::front_end::grammar::{leaf, permission_shapes, primitives};
use crate::runtime_backend::front_end::lexer::OwnedLexToken;
use crate::runtime_backend::front_end::shared::util::trim_edge_punctuation_tokens;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ModifierTailAction<'a> {
    Complete,
    DynamicForEach(&'a [OwnedLexToken]),
    WhereX(&'a [OwnedLexToken]),
    Unsupported,
}

#[derive(Debug, Clone)]
pub(crate) struct ModifierTailShape<'a> {
    pub(crate) duration: Until,
    pub(crate) condition: Option<ConditionExpr>,
    pub(crate) action: ModifierTailAction<'a>,
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
    let Some(token) = rest.first() else {
        return false;
    };
    if leaf::parse_leaf_pt_modifier_values_complete(token.parser_text()).is_err() {
        return false;
    }
    let tail = trim_edge_punctuation_tokens(rest.get(1..).unwrap_or_default());
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

pub(crate) fn parse_modifier_tail_shape(tokens: &[OwnedLexToken]) -> ModifierTailShape<'_> {
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
    use crate::runtime_backend::front_end::lexer::lex_line;

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
}
