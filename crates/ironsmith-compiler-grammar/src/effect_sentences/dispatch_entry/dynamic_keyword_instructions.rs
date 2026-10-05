use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    match crate::activation_and_restrictions::keyword_action_costs::parse_dynamic_keyword_amount(tokens)? {
        crate::cards::builders::KeywordAction::BolsterValue { amount, .. } =>
            Some(EffectAst::subject_verb_bolster(amount)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_local_bolster_quantities_bind_the_keyword_only() {
        for text in [
            "Bolster X, where X is the number of tapped creatures you control.",
            "Bolster X, where X is the number of differently named artifact tokens you control.",
            "Bolster X, where X is the number of cards in your hand.",
            "Bolster X.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::KeywordActions(KeywordActionAst::Bolster { amount }), ..
            }) = parse(&tokens).unwrap() else { panic!("not bolster"); };
            if text.contains("where") { assert!(!matches!(amount.unhinted(), Value::X)); }
        }
        for text in ["Bolster X and draw two cards.", "Bolster X, where X is unknown information."] {
            assert!(parse(&crate::lexer::lex_line(text, 0).unwrap()).is_none());
        }
    }
    #[test]
    fn a_granted_mobilize_quantity_names_its_recipient() {
        let tokens = crate::lexer::lex_line("Mobilize X, where X is its power.", 0).unwrap();
        let Some(crate::cards::builders::KeywordAction::MobilizeValue { amount, .. }) =
            crate::activation_and_restrictions::keyword_action_costs::parse_dynamic_keyword_amount(&tokens)
        else { panic!("missing typed keyword"); };
        assert_eq!(amount, Value::SourcePower);
    }
}
