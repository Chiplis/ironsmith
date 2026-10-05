//! Live combat and completed-action history at the alternative-cost boundary.
use crate::lexer::OwnedLexToken;
use crate::static_abilities::ThisSpellCostCondition;
use ironsmith_core::{
    Condition, ObjectFilter, PlayerFilter, TurnHistoryCount, Value, ValueComparisonOperator,
};

fn words_are(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    crate::lexer::parser_token_word_refs(tokens) == expected
}
fn filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(tokens, false).ok()
}
fn quantified_filter(
    tokens: &[OwnedLexToken],
) -> Option<(i32, ValueComparisonOperator, ObjectFilter)> {
    let (tokens, exact) = if tokens.first()?.is_word("exactly") {
        (&tokens[1..], true)
    } else {
        (tokens, false)
    };
    let (number, used) = if tokens.first()?.is_any_word(&["a", "an"]) {
        (1, 1)
    } else {
        crate::util::parse_number(tokens)?
    };
    let mut rest = &tokens[used..];
    if rest.len() >= 2 && words_are(&rest[..2], &["or", "more"]) {
        rest = &rest[2..];
    }
    Some((
        i32::try_from(number).ok()?,
        if exact {
            ValueComparisonOperator::Equal
        } else {
            ValueComparisonOperator::GreaterThanOrEqual
        },
        filter(rest)?,
    ))
}
fn opponent_effect() -> ironsmith_core::CauseFilter {
    ironsmith_core::CauseFilter::effect_like()
        .with_controller(ironsmith_core::ControllerFilter::ContextOpponent)
}

pub(super) fn read(tokens: &[OwnedLexToken]) -> Option<ThisSpellCostCondition> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let (value, operator, threshold) = if tokens.len() > 2
        && (words_are(&tokens[tokens.len() - 2..], &["are", "attacking"])
            || words_are(&tokens[tokens.len() - 2..], &["is", "attacking"]))
    {
        let (amount, operator, mut filter) = quantified_filter(&tokens[..tokens.len() - 2])?;
        filter.attacking = true;
        (Value::Count(filter), operator, amount)
    } else if words_are(
        tokens,
        &[
            "an", "opponent", "searched", "their", "library", "this", "turn",
        ],
    ) {
        (
            Value::TurnHistoryCount(TurnHistoryCount::LibrarySearches {
                player: PlayerFilter::Opponent,
                own_library_only: true,
            }),
            ValueComparisonOperator::GreaterThanOrEqual,
            1,
        )
    } else if tokens.len() > 11
        && words_are(&tokens[..3], &["an", "opponent", "had"])
        && words_are(
            &tokens[tokens.len() - 8..],
            &[
                "enter",
                "the",
                "battlefield",
                "under",
                "their",
                "control",
                "this",
                "turn",
            ],
        )
    {
        let (amount, operator, filter) = quantified_filter(&tokens[3..tokens.len() - 8])?;
        (
            Value::TurnHistoryCount(TurnHistoryCount::MaxEnteredBattlefieldByController {
                player: PlayerFilter::Opponent,
                filter,
            }),
            operator,
            amount,
        )
    } else if let Some(was) = tokens.iter().position(|token| token.is_word("was")) {
        if was >= 3
            && words_are(
                &tokens[was..],
                &[
                    "was",
                    "destroyed",
                    "this",
                    "turn",
                    "by",
                    "a",
                    "spell",
                    "or",
                    "ability",
                    "an",
                    "opponent",
                    "controlled",
                ],
            )
            && words_are(&tokens[was - 3..was], &["under", "your", "control"])
        {
            let mut filter = filter(&tokens[..was - 3])?;
            filter.controller = Some(PlayerFilter::You);
            (
                Value::TurnHistoryCount(TurnHistoryCount::DestroyedBy {
                    filter,
                    cause: opponent_effect(),
                }),
                ValueComparisonOperator::GreaterThanOrEqual,
                1,
            )
        } else if was >= 4
            && words_are(&tokens[was - 4..was], &["you", "cast", "this", "turn"])
            && words_are(
                &tokens[was..],
                &[
                    "was",
                    "countered",
                    "by",
                    "a",
                    "spell",
                    "or",
                    "ability",
                    "an",
                    "opponent",
                    "controlled",
                ],
            )
        {
            (
                Value::TurnHistoryCount(TurnHistoryCount::CastSpellsCounteredBy {
                    caster: PlayerFilter::You,
                    filter: filter(&tokens[..was - 4])?,
                    cause: opponent_effect(),
                }),
                ValueComparisonOperator::GreaterThanOrEqual,
                1,
            )
        } else {
            return None;
        }
    } else {
        return None;
    };
    Some(ThisSpellCostCondition::ConditionExpr {
        condition: Condition::ValueComparison {
            left: value,
            operator,
            right: Value::Fixed(threshold),
        },
        display: crate::lexer::parser_token_word_refs(tokens).join(" "),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_family_has_a_typed_complete_condition() {
        for text in [
            "four or more creatures are attacking",
            "exactly one creature is attacking",
            "a white creature is attacking",
            "a black creature with flying is attacking",
            "an opponent searched their library this turn",
            "an opponent had an artifact enter the battlefield under their control this turn",
            "an opponent had a green creature enter the battlefield under their control this turn",
            "an opponent had two or more creatures enter the battlefield under their control this turn",
            "a noncreature permanent under your control was destroyed this turn by a spell or ability an opponent controlled",
            "a creature spell you cast this turn was countered by a spell or ability an opponent controlled",
        ] {
            assert!(
                read(&crate::lexer::lex_line(text, 0).unwrap()).is_some(),
                "{text}"
            );
        }
        for text in [
            "an opponent searched a library this turn",
            "an opponent had two or more creatures enter the battlefield under your control this turn",
            "a creature spell was countered",
            "a noncreature permanent under your control died this turn",
            "four or more creatures are attacking banana",
        ] {
            assert!(
                read(&crate::lexer::lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }
}
