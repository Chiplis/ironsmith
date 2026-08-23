use crate::effect::{Value, ValueComparisonOperator};
use crate::grammar::values as value_grammar;
use crate::grammar::{filters, permission_shapes, primitives};
use crate::lexer::{OwnedLexToken, TokenWordView};

use super::super::sequence_any_phrase;

#[derive(Debug, Clone, PartialEq)]
pub struct ConsultManaValueConditionShape {
    pub operator: ValueComparisonOperator,
    pub right: Value,
}

const MANA_VALUE_CONDITION_PREFIXES: &[&[&str]] = &[
    &["if", "it's", "a", "spell", "with", "mana", "value"],
    &[
        "if", "it's", "an", "instant", "spell", "with", "mana", "value",
    ],
    &["if", "its", "a", "spell", "with", "mana", "value"],
    &[
        "if", "its", "an", "instant", "spell", "with", "mana", "value",
    ],
    &["if", "it", "is", "a", "spell", "with", "mana", "value"],
    &[
        "if", "it", "is", "an", "instant", "spell", "with", "mana", "value",
    ],
    &["if", "the", "spell's", "mana", "value"],
    &["if", "the", "spells", "mana", "value"],
    &["if", "that", "spell's", "mana", "value"],
    &["if", "that", "spells", "mana", "value"],
    &["if", "its", "mana", "value"],
];

pub fn parse_consult_condition_value_shape(tokens: &[OwnedLexToken]) -> Option<Value> {
    if permission_shapes::exact_tokens_any(tokens, &[&["thiss", "power"], &["this", "power"]]) {
        return Some(Value::SourcePower);
    }

    if let Some((value, used)) = value_grammar::parse_value_prefix_lexed(tokens)
        && TokenWordView::new(&tokens[used..]).is_empty()
    {
        return Some(value);
    }

    let (_, filter_tokens) = primitives::parse_prefix(
        tokens,
        sequence_any_phrase(&[&["the", "number", "of"], &["number", "of"]]),
    )?;
    if TokenWordView::new(filter_tokens).is_empty() {
        return None;
    }
    let filter = filters::parse_object_filter_with_grammar_entrypoint(filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub fn parse_consult_mana_value_condition_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConsultManaValueConditionShape> {
    let (_, comparison_tokens) =
        primitives::parse_prefix(tokens, sequence_any_phrase(MANA_VALUE_CONDITION_PREFIXES))?;
    let (operator, right_tokens) = value_grammar::parse_value_comparison_tokens(comparison_tokens)?;
    let right = parse_consult_condition_value_shape(right_tokens)?;
    Some(ConsultManaValueConditionShape { operator, right })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_consult_values_and_mana_value_conditions() {
        assert_eq!(
            parse_consult_condition_value_shape(&lex("this's power")),
            Some(Value::SourcePower)
        );
        assert!(matches!(
            parse_consult_condition_value_shape(&lex("the number of creature cards you own")),
            Some(Value::Count(_))
        ));
        let condition = parse_consult_mana_value_condition_shape(&lex(
            "if its mana value is less than this's power",
        ))
        .unwrap();
        assert_eq!(condition.operator, ValueComparisonOperator::LessThan);
        assert_eq!(condition.right, Value::SourcePower);
    }
}
