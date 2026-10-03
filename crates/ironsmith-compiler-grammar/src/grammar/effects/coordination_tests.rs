use super::*;
use crate::lexer::lex_line;

#[test]
fn modified_type_union_stays_inside_one_return_operand() {
    for text in [
        "Return target artifact or non-Aura enchantment card from your graveyard to the battlefield with X additional +1/+1 counters on it.",
        "Return up to one target artifact, creature, or non-Aura enchantment card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it.",
        "Return target artifact or legendary creature card from your graveyard to the battlefield with a shield counter on it.",
        "Return target artifact or non-Equipment enchantment card from your graveyard to the battlefield with a shield counter on it.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let plan = recognize_coordination(&tokens);
        assert!(matches!(plan, ParseOutcome::NoMatch), "{text}: {plan:#?}");
        let segments =
            super::super::chain_splitting::split_segments_on_comma_effect_head_tokens(vec![
                &tokens,
            ]);
        assert_eq!(segments, vec![tokens.as_slice()], "{text}");
        crate::effect_sentences::parse_effect_chain_lexed(&tokens).unwrap_or_else(|error| {
            panic!("the complete return operand must parse: {text}: {error}")
        });
    }
}

#[test]
fn modified_type_union_preserves_following_action_boundaries() {
    for (operator, ordering) in [
        ("and", EffectOrderingAst::Unordered),
        (", then", EffectOrderingAst::Ordered),
        ("or", EffectOrderingAst::Alternative),
    ] {
        let text = format!(
            "Return target artifact or non-Aura enchantment card from your graveyard to the battlefield {operator} draw a card."
        );
        let tokens = lex_line(&text, 0).unwrap();
        let ParseOutcome::Match(plan) = recognize_coordination(&tokens) else {
            panic!("the authored follow-up remains a separate action: {text}");
        };
        assert_eq!(plan.value.members.len(), 2, "{text}: {plan:#?}");
        assert_eq!(plan.value.boundaries[0].ordering, ordering, "{text}");
        assert!(
            plan.value.members[0]
                .tokens
                .iter()
                .any(|token| token.is_word("battlefield"))
        );
        assert!(plan.value.members[1].tokens[0].is_word("draw"));
    }
}

#[test]
fn explicit_alternative_actions_do_not_become_modified_type_arms() {
    for text in [
        "Destroy target artifact or tap target non-Aura enchantment.",
        "Destroy target artifact or an opponent sacrifices a creature.",
        "Destroy target artifact or a tapped creature you control deals 1 damage to each opponent.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let ParseOutcome::Match(plan) = recognize_coordination(&tokens) else {
            panic!("explicit alternatives must remain coordinated: {text}");
        };
        assert_eq!(plan.value.members.len(), 2, "{text}: {plan:#?}");
        assert_eq!(
            plan.value.boundaries[0].ordering,
            EffectOrderingAst::Alternative
        );
    }
}
