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

#[test]
fn base_stat_grants_and_negated_copulas_materialize_one_shared_subject() {
    let tokens = lex_line(
        "this creature has base power and toughness 5/3, gains trample, and isn't a Human",
        0,
    )
    .unwrap();
    let ParseOutcome::Match(plan) = recognize_coordination(&tokens) else {
        panic!("base stats, grant, and negated copula must form a coordinated clause");
    };
    assert_eq!(plan.value.members.len(), 3, "{plan:#?}");
    assert!(
        plan.value
            .boundaries
            .iter()
            .all(|boundary| boundary.omission == CoordinationOmissionAst::Subject)
    );
    let segments = plan.value.materialized_segments().unwrap();
    assert_eq!(
        crate::lexer::parser_token_word_refs(&segments[1]),
        ["this", "creature", "gains", "trample"]
    );
    let last = crate::lexer::parser_token_word_refs(&segments[2]);
    assert_eq!(&last[..2], &["this", "creature"]);
    assert!(matches!(last[2], "isnt" | "isn't"));
    assert_eq!(&last[3..], &["a", "human"]);
}

#[test]
fn copular_relative_filters_are_not_shared_subject_actions() {
    for text in [
        "target creature that is red",
        "target artifact which isn't a creature",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(find_coordination_verb_tokens(&tokens).is_none(), "{text}");
    }
}

#[test]
fn shared_subject_subtype_removal_uses_the_existing_typed_duration_scoped_action() {
    use crate::cards::builders::{
        EffectAst, StatChangeActionAst, SubjectVerbActionAst, SubjectVerbEffectAst, TargetAst,
    };
    use crate::effect::Until;
    use crate::types::Subtype;
    fn removals(effects: &[EffectAst], found: &mut Vec<(TargetAst, Vec<Subtype>, Until)>) {
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes {
                        target,
                        subtypes,
                        duration,
                    }),
                ..
            }) = effect
            {
                found.push((target.clone(), subtypes.clone(), duration.clone()));
            }
            crate::model::visit::for_each_nested_effects(effect, false, |nested| {
                removals(nested, found)
            });
        }
    }
    for (text, subtype) in [
        (
            "Until end of turn, this creature has base power and toughness 5/3, gains trample, and isn't a Human.",
            Subtype::Human,
        ),
        (
            "Until end of turn, this creature has base power and toughness 3/2, gains flying, and isn't an Elf.",
            Subtype::Elf,
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let effects = crate::effect_sentences::parse_effect_chain_lexed(&tokens).unwrap();
        let mut found = Vec::new();
        removals(&effects, &mut found);
        assert_eq!(found.len(), 1, "{effects:#?}");
        assert!(matches!(found[0].0, TargetAst::Source(_)), "{found:#?}");
        assert_eq!(found[0].1, vec![subtype]);
        assert_eq!(found[0].2, Until::EndOfTurn);
    }
}
