//! Source-authored routing regressions. UNRUN under the campaign execution gate.
use super::*;
use super::typed_clause_heads::{ClauseHeadFormAst, classify_typed_clause_head};
use crate::effect::{Restriction, Until};
use crate::recognition::ParseOutcome;

fn assert_one_object_bound_untap(effects: &[EffectAst]) {
    let mut restrictions = 0;
    fn visit(effect: &EffectAst, restrictions: &mut usize) {
        if let EffectAst::SubjectVerb(subject_verb) = effect {
            match &subject_verb.action {
                SubjectVerbActionAst::Cant {
                    restriction: Restriction::Untap(filter),
                    duration: Until::ControllersNextUntapStep,
                    ..
                } => {
                    *restrictions += 1;
                    assert!(filter.tagged_constraints.iter().any(|constraint| {
                        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                    }), "the subject must retain the prior resolving object set: {filter:?}");
                }
                SubjectVerbActionAst::PermanentState(
                    crate::cards::builders::PermanentStateActionAst::Untap { .. }
                    | crate::cards::builders::PermanentStateActionAst::UntapAll { .. }
                ) => panic!("a negated clause must never become an affirmative untap"),
                _ => {}
            }
        }
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            for effect in nested { visit(effect, restrictions); }
        });
    }
    for effect in effects { visit(effect, &mut restrictions); }
    assert_eq!(restrictions, 1, "{effects:#?}");
}

#[test]
fn authored_and_normalized_plural_restrictions_keep_their_head_and_complete_owner() {
    for subject in ["Those creatures", "Those permanents", "They"] {
        for negation in ["don't", "don’t", "dont", "do not", "can't", "cannot"] {
            for controller in ["controllers'", "controllers’", "controllers"] {
                let text = format!("{subject} {negation} untap during their {controller} next untap steps.");
                let tokens = lex_line(&text, 0).unwrap();
                let ParseOutcome::Match(head) = classify_typed_clause_head(&tokens) else {
                    panic!("missing restriction head: {text}");
                };
                assert_eq!(head.value.form, ClauseHeadFormAst::Restriction, "{text}");
                assert!(!head.value.permits_action_fallback(), "{text}");
                assert_one_object_bound_untap(&parse_cant_effect_sentence(&tokens).unwrap().unwrap());
                let indexed = crate::effect_sentences::run_subject_verb_primitives_lexed(
                    &tokens,
                    crate::effect_sentences::POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
                    &crate::effect_sentences::POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
                ).unwrap().expect("restriction-family dispatch must not require a cant first word");
                assert_one_object_bound_untap(&indexed);
                let clause = crate::effect_sentences::parse_effect_clause_lexed(&tokens).unwrap();
                assert_one_object_bound_untap(&[clause]);
                let sentence = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
                assert_one_object_bound_untap(&sentence);
            }
        }
    }
}

#[test]
fn negative_words_inside_operands_and_conditions_do_not_reclassify_outer_actions() {
    for text in [
        "Untap target creature you don't control.",
        "Tap that creature and it doesn't untap during its controller's next untap step.",
        "Scry 1 and those creatures don't untap during their controllers' next untap steps.",
        "Goad that creature and it can't block this turn.",
        "For each creature that doesn't untap, draw a card.",
        "Create a token with \"This token doesn't untap during its controller's next untap step.\"",
        "If you don't, untap those creatures.",
        "When this creature enters, those creatures don't untap during their controllers' next untap steps.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let ParseOutcome::Match(head) = classify_typed_clause_head(&tokens) else {
            panic!("missing outer head: {text}");
        };
        assert_ne!(head.value.form, ClauseHeadFormAst::Restriction, "{text}");
    }
}

#[test]
fn malformed_or_different_lifetimes_do_not_fall_through_to_affirmative_untap() {
    for text in [
        "Those creatures don't untap during their controllers' next two untap steps.",
        "Those creatures don't untap during their controllers' next untap steps instead nonsense.",
        "Those creatures don't untap until their controllers' next untap steps.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(crate::effect_sentences::parse_effect_clause_lexed(&tokens).is_err(), "{text}");
    }
    let named = lex_line("Those creatures don't untap during that player's next untap step.", 0).unwrap();
    let effects = parse_cant_effect_sentence(&named).unwrap().unwrap();
    assert!(matches!(effects.as_slice(), [EffectAst::SubjectVerb(subject_verb)]
        if matches!(&subject_verb.action, SubjectVerbActionAst::Cant {
            duration: Until::PlayersNextUntapStep { .. }, ..
        })));
    // The existing strict owner guard must still reject an untap-step lifetime
    // borrowed by an unsupported characteristic/permission action.
    let characteristic = lex_line("Those creatures get +1/+1 during that player's next untap step.", 0).unwrap();
    assert!(parse_cant_effect_sentence(&characteristic).unwrap().is_none());
}

#[test]
fn conditional_documents_keep_the_predicate_outside_the_untap_restriction() {
    for predicate in [
        "If you control a creature with power 4 or greater",
        "If there are two or more instant and/or sorcery cards in your graveyard",
    ] {
        let text = format!("{predicate}, those creatures don't untap during their controllers' next untap steps.");
        let tokens = lex_line(&text, 0).unwrap();
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
        let [EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        })] = effects.as_slice() else {
            panic!("the document must retain its resolution-time condition: {effects:#?}");
        };
        assert!(if_false.is_empty());
        assert_one_object_bound_untap(if_true);
    }
}

#[test]
fn conditional_restrictions_do_not_hide_invalid_predicates_or_durations() {
    for text in [
        "If the moon tastes blue, those creatures don't untap during their controllers' next untap steps.",
        "If you control a creature with power 4 or greater, those creatures don't untap during their controllers' next two untap steps.",
        "If you control a creature with power 4 or greater, those creatures don't untap during their controllers' next untap steps except on Tuesdays.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(crate::effect_sentences::parse_effect_sentences_lexed(&tokens).is_err(), "{text}");
    }
}
