use super::*;
use crate::lexer::lex_line;

fn parse_coordinated(text: &str) -> Vec<EffectAst> {
    let tokens = lex_line(text, 0).expect("fixture should lex");
    let parsed = parse_effect_sentence_lexed(&tokens).expect("fixture should parse");
    let [
        EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one coordinated clause, got {parsed:#?}");
    };
    effects.clone()
}

fn assert_target_pump_and_must_be_blocked(text: &str, amount: i32) {
    let effects = parse_coordinated(text);
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Pump {
                    power,
                    toughness,
                    target,
                    duration,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::MustBeBlocked(filter),
                    duration: restriction_duration,
                    condition: None,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected target pump followed by must-be-blocked, got {effects:#?}");
    };

    assert_eq!(power, &Value::Fixed(amount));
    assert_eq!(toughness, &Value::Fixed(amount));
    assert_eq!(duration, &Until::EndOfTurn);
    assert_eq!(restriction_duration, &Until::EndOfTurn);
    assert!(
        matches!(target, TargetAst::Object(filter, _, _) if filter.card_types == vec![CardType::Creature]),
        "pump should keep the original target creature: {target:#?}"
    );
    assert_eq!(
        filter,
        &ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key())
    );
}

#[test]
fn target_pump_and_must_be_blocked_share_one_target() {
    assert_target_pump_and_must_be_blocked(
        "Target creature gets +3/+3 until end of turn and must be blocked this turn if able.",
        3,
    );
    assert_target_pump_and_must_be_blocked(
        "Target creature gets +5/+5 until end of turn and must be blocked this turn if able.",
        5,
    );
}

#[test]
fn group_pump_and_must_be_blocked_reuse_the_same_filter() {
    let effects = parse_coordinated(
        "Each creature you control gets +3/+3 until end of turn and must be blocked this turn if able.",
    );
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PumpAll {
                    filter: pump_filter,
                    power,
                    toughness,
                    duration,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::MustBeBlocked(restriction_filter),
                    duration: restriction_duration,
                    condition: None,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected group pump followed by must-be-blocked, got {effects:#?}");
    };

    assert_eq!((power, toughness), (&Value::Fixed(3), &Value::Fixed(3)));
    assert_eq!(duration, &Until::EndOfTurn);
    assert_eq!(restriction_duration, &Until::EndOfTurn);
    assert_eq!(pump_filter.controller, Some(PlayerFilter::You));
    assert_eq!(pump_filter.card_types, vec![CardType::Creature]);
    assert_eq!(restriction_filter, pump_filter);
}

#[test]
fn target_grant_and_must_be_blocked_share_one_target() {
    let effects = parse_coordinated(
        "Target creature gains deathtouch until end of turn and must be blocked this turn if able.",
    );
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target,
                    abilities,
                    duration,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::MustBeBlocked(filter),
                    duration: restriction_duration,
                    condition: None,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected ability grant followed by must-be-blocked, got {effects:#?}");
    };

    assert!(
        matches!(target, TargetAst::Object(filter, _, _) if filter.card_types == vec![CardType::Creature]),
        "grant should keep the original target creature: {target:#?}"
    );
    assert_eq!(duration, &Until::EndOfTurn);
    assert_eq!(restriction_duration, &Until::EndOfTurn);
    assert_eq!(
        filter,
        &ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key())
    );
    assert!(
        format!("{abilities:#?}").contains("Deathtouch"),
        "expected deathtouch grant, got {abilities:#?}"
    );
}
