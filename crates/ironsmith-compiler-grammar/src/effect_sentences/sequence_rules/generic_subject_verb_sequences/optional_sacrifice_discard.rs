use super::super::super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, EffectAst, IfResultPredicate, ObjectFilter, PlayerAst,
    TargetAst,
};
use crate::effect::Value;
use crate::target::PlayerFilter;
use crate::zone::Zone;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse_pair(first: &str, second: &str) -> Option<Vec<EffectAst>> {
        let first = lex_line(first, 0).expect("first sentence should lex");
        let second = lex_line(second, 0).expect("second sentence should lex");
        let sentences = [
            SentenceInput::from_lexed(&first),
            SentenceInput::from_lexed(&second),
        ];
        crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
            .expect("pair parser should not error")
    }

    #[test]
    fn keeps_optional_modes_and_correlated_opponent_failure() {
        let effects = parse_pair(
            "Each opponent may sacrifice a nonland permanent of their choice or discard a card.",
            "Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way.",
        )
        .expect("exact pair should parse");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("VillainousChoice"), "{debug}");
        assert!(debug.contains("ForEachOpponentDid"), "{debug}");
        assert!(debug.contains("DidNot"), "{debug}");
        assert!(debug.contains("DealDamageEqualToPower"), "{debug}");

        let lowered = crate::compile_support::compile_statement_effects(&effects)
            .expect("correlated opponent choice should lower");
        let lowered_debug = format!("{lowered:#?}");
        assert!(
            lowered_debug.contains("ForPlayersEffect"),
            "{lowered_debug}"
        );
        assert!(lowered_debug.contains("WithIdEffect"), "{lowered_debug}");
        assert!(lowered_debug.contains("MayEffect"), "{lowered_debug}");
        assert!(
            lowered_debug.contains("VillainousChoiceEffect"),
            "{lowered_debug}"
        );
        assert!(
            lowered_debug.contains("predicate: DidNotHappen"),
            "{lowered_debug}"
        );
        assert!(
            lowered_debug.matches("IteratedPlayer").count() >= 4,
            "{lowered_debug}"
        );
    }

    #[test]
    fn does_not_claim_a_required_or_nonland_specific_near_miss() {
        assert!(
            parse_pair(
                "Each opponent sacrifices a nonland permanent of their choice or discards a card.",
                "Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way.",
            )
            .is_none()
        );
        assert!(
            parse_pair(
                "Each opponent may sacrifice a permanent of their choice or discard a card.",
                "Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way.",
            )
            .is_none()
        );
    }
}
