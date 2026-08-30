use super::*;
use crate::cards::builders::TurnHistoryPredicateAst;

fn assert_it_characteristic_threshold(predicate: &PredicateAst, toughness: bool) {
    match predicate {
        PredicateAst::ValueComparison {
            left,
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            ..
        } => {
            let spec = match left {
                Value::ToughnessOf(spec) if toughness => spec,
                Value::ManaValueOf(spec) if !toughness => spec,
                _ => panic!("expected the authored target characteristic: {predicate:#?}"),
            };
            assert!(
                matches!(
                    spec.base(),
                    ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                ),
                "the threshold must remain linked to the targeted object: {predicate:#?}"
            );
        }
        PredicateAst::ItMatches(filter) | PredicateAst::TargetMatches(filter)
            if matches!(
                (&filter.toughness, &filter.mana_value, toughness),
                (Some(crate::filter::Comparison::LessThanOrEqual(_)), _, true)
                    | (
                        _,
                        Some(crate::filter::Comparison::LessThanOrEqual(_)),
                        false
                    )
            ) => {}
        _ => panic!("expected a typed at-most threshold: {predicate:#?}"),
    }
}

#[test]
fn trailing_instead_if_rebinds_the_nested_it_threshold_but_not_the_revolt_gate() {
    let lexed = crate::lexer::lex_line(
        "Destroy target creature if it has mana value 2 or less. Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.",
        0,
    )
    .expect("nested target threshold replacement should lex");
    let parsed = parse_effect_sentences_lexed(&lexed)
        .expect("nested target threshold replacement should parse");

    let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability: false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one target self-replacement: {parsed:#?}");
    };
    assert!(
        matches!(
            predicate,
            PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn { .. }
        ),
        "the outer replacement gate must remain the turn-history predicate: {predicate:#?}"
    );
    trailing_threshold(if_false, false);
    trailing_threshold(if_true, false);
}

fn trailing_threshold(effects: &[EffectAst], toughness: bool) {
    match effects {
        [EffectAst::TagAffected { effect, .. }] => {
            trailing_threshold(std::slice::from_ref(effect.as_ref()), toughness);
        }
        [EffectAst::TrailingIf { predicate, effects }] => {
            assert_it_characteristic_threshold(predicate, toughness);
            assert_eq!(effects.len(), 1, "threshold branch must retain one action");
        }
        [EffectAst::ControlFlow(control)] => {
            let crate::model::control_flow::ControlFlowNodeAst::Condition {
                condition,
                consequence_program,
                alternative_program: None,
                ..
            } = &control.node
            else {
                panic!("expected one trailing target threshold: {effects:#?}");
            };
            assert_eq!(
                condition.position,
                crate::model::control_flow::ConditionPositionAst::Postcondition
            );
            let crate::model::control_flow::ControlPredicateAst::State(predicate) =
                &condition.predicate
            else {
                panic!("expected a state threshold: {condition:#?}");
            };
            assert_it_characteristic_threshold(predicate, toughness);
            assert_eq!(
                control
                    .program(*consequence_program)
                    .expect("threshold consequence")
                    .effects
                    .len(),
                1,
                "threshold branch must retain one action"
            );
        }
        _ => panic!("expected one trailing target threshold: {effects:#?}"),
    }
}

#[test]
fn madness_replacement_keeps_both_target_toughness_thresholds() {
    let lexed = crate::lexer::lex_line(
            "Gain control of target creature if its toughness is 2 or less. If this spell's madness cost was paid, instead gain control of that creature if its toughness is X or less.",
            0,
        )
        .expect("conditional target replacement should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("conditional target replacement should parse");

    let [
        EffectAst::SelfReplacement {
            predicate: PredicateAst::ThisSpellPaidLabel(label),
            if_true,
            if_false,
            attach_to_previous_ability: false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one paid-cost self-replacement: {parsed:#?}");
    };
    assert!(label.display_label().eq_ignore_ascii_case("Madness"));
    trailing_threshold(if_false, true);
    trailing_threshold(if_true, true);
}

#[test]
fn kicked_replacement_keeps_both_target_mana_value_thresholds() {
    let lexed = crate::lexer::lex_line(
            "Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead.",
            0,
        )
        .expect("conditional target replacement should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("conditional target replacement should parse");

    let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability: false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one kicked self-replacement: {parsed:#?}");
    };
    assert!(
        matches!(
            predicate,
            PredicateAst::ThisSpellWasKicked
                | PredicateAst::TurnHistory(TurnHistoryPredicateAst::SourceWasKicked { .. })
        ),
        "expected a kicked-source predicate: {predicate:#?}"
    );
    trailing_threshold(if_false, false);
    trailing_threshold(if_true, false);
}

#[test]
fn kicked_target_replacement_carries_the_common_exile_life_suffix_into_both_arms() {
    let lexed = crate::lexer::lex_line(
            "Choose target creature with mana value 3 or less. If this spell was kicked, instead choose target creature. Exile the chosen creature, then its controller gains life equal to its mana value.",
            0,
        )
        .expect("kicked target replacement should lex");
    let parsed = parse_effect_sentences_lexed(&lexed)
        .expect("kicked target replacement and common suffix should parse");

    let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability: false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one executable self-replacement: {parsed:#?}");
    };
    assert!(matches!(
        predicate,
        PredicateAst::ThisSpellWasKicked
            | PredicateAst::TurnHistory(TurnHistoryPredicateAst::SourceWasKicked { .. })
    ));
    for branch in [if_false, if_true] {
        let debug = format!("{branch:#?}");
        assert!(debug.contains("Exile"), "missing common exile: {debug}");
        assert!(
            debug.contains("GainLife"),
            "missing common life gain: {debug}"
        );
        assert!(
            debug.contains("ManaValueOf"),
            "life amount lost its chosen-object basis: {debug}"
        );
    }
}

#[test]
fn a_non_instead_kicked_choice_is_not_rewritten_as_a_self_replacement() {
    let lexed = crate::lexer::lex_line(
            "Choose target creature with mana value 3 or less. If this spell was kicked, choose target creature.",
            0,
        )
        .expect("conditional choice near miss should lex");
    let parsed = parse_effect_sentences_lexed(&lexed)
        .expect("conditional choice near miss should remain parseable");
    assert!(
        !parsed
            .iter()
            .any(|effect| matches!(effect, EffectAst::SelfReplacement { .. })),
        "only an authored instead clause can replace the default choice: {parsed:#?}"
    );
}
