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

    #[test]
    fn both_live_keyword_readers_keep_complete_local_definitions_and_authenticate_prefixes() {
        for read in [
            crate::clause_support::parse_ability_line_lexed as fn(&[OwnedLexToken]) -> Option<Vec<crate::cards::builders::KeywordAction>>,
            crate::keyword_static::parse_ability_line,
        ] {
            for text in [
                "Mobilize X, where X is the number of creature cards in your graveyard.",
                "Menace and mobilize X, where X is its power.",
                "Menace, mobilize X, where X is its power.",
                "Bolster X, where X is the number of cards in your hand.",
                "Bolster X, where X is the number of differently named artifact tokens you control.",
            ] {
                let actions = read(&crate::lexer::lex_line(text, 0).unwrap()).unwrap();
                if text.starts_with("Menace") {
                    assert_eq!(actions.len(), 2);
                    assert!(matches!(actions[0], crate::cards::builders::KeywordAction::Menace));
                }
                let amount = match actions.last().unwrap() {
                    crate::cards::builders::KeywordAction::MobilizeValue { amount, .. }
                    | crate::cards::builders::KeywordAction::BolsterValue { amount, .. } => amount,
                    other => panic!("lost dynamic amount: {other:?}"),
                };
                assert!(!matches!(amount.unhinted(), Value::X));
                if text.contains("differently named") {
                    assert!(matches!(amount.unhinted(), Value::DistinctNames(_)));
                }
            }
            for text in [
                "Invented and mobilize X, where X is its power.",
                "Mobilize X, where X is unknown information.",
                "Menace and mobilize X, where X is its power and draw a card.",
                "Mobilize X, where X is its power {R}.",
                "Menace and mobilize X, where X is its power {R}.",
                "Mobilize X, where X is its {R} power.",
                "Mobilize X, where X is its power:.",
                "Bolster X, where X is the number of cards in your hand and draw a card.",
                "Bolster X, where X is the number of cards in your hand nonsense.",
                "Bolster 2 nonsense.",
                "Mobilize 2 nonsense.",
                "Mobilize 2, where X is its power.",
            ] {
                assert!(read(&crate::lexer::lex_line(text, 0).unwrap()).is_none(), "{text}");
            }
            for text in ["Bolster 2.", "Mobilize 2.", "Menace and mobilize 2."] {
                assert!(read(&crate::lexer::lex_line(text, 0).unwrap()).is_some(), "{text}");
            }
        }
    }
}
