//! Historical total power of an attack declaration (pack tactics).

use super::*;

pub(super) fn parse_attacked_with_total_power(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let prefix = &[
        "you",
        "attacked",
        "with",
        "creatures",
        "with",
        "total",
        "power",
    ];
    let suffix = &["or", "greater", "this", "combat"];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(prefix.len())),
        WinnowSequence::amount("power", WinnowCaptureKind::UntilPhrase(suffix)),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    if !surface::exact(matched.capture_clause("subject", clause)?, prefix)
        || !surface::exact(matched.capture_clause("window", clause)?, suffix)
    {
        return None;
    }
    let power_clause = matched.capture_clause("power", clause)?;
    let (power, used) = parse_number(power_clause.tokens())?;
    if used != power_clause.tokens().len() {
        return None;
    }
    Some(PredicateAst::TurnEvents(
        TurnEventPredicateAst::YouAttackedWithTotalPowerAtLeastThisCombat(power),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn pack_tactics_power_is_a_typed_historical_combat_threshold() {
        for (number, expected) in [("6", 6), ("seven", 7), ("12", 12)] {
            let tokens = lex_line(
                &format!(
                    "you attacked with creatures with total power {number} or greater this combat"
                ),
                0,
            )
            .unwrap();
            assert_eq!(
                parse_attacked_with_total_power(&tokens),
                Some(PredicateAst::TurnEvents(
                    TurnEventPredicateAst::YouAttackedWithTotalPowerAtLeastThisCombat(expected),
                )),
            );
        }
        for text in [
            "you attacked with creatures with total power 6 or greater this turn",
            "you attack with creatures with total power 6 or greater this combat",
            "an opponent attacked with creatures with total power 6 or greater this combat",
            "you attacked with creatures with total power 6 or less this combat",
            "you attacked with creatures with total toughness 6 or greater this combat",
            "you attacked with creatures with total power 6 or greater this combat and drew a card",
        ] {
            assert_eq!(
                parse_attacked_with_total_power(&lex_line(text, 0).unwrap()),
                None,
                "{text}"
            );
        }
    }
}
