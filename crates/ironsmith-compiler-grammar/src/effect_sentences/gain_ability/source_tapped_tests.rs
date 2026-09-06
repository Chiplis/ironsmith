use crate::cards::builders::SourcePredicateAst;
use crate::cards::builders::StatChangeActionAst;
use crate::cards::builders::GrantActionAst;
use super::super::super::util::tokenize_line;
use super::*;

#[test]
fn source_tapped_keyword_grants_keep_typed_duration_and_condition() {
    let tokens = tokenize_line(
        "Target creature you control other than this creature has shroud for as long as this creature remains tapped.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("source-tapped grant should parse")
        .expect("source-tapped grant should produce effects");

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                    target: TargetAst::Object(filter, ..),
                    duration,
                    condition,
                    ..
                }),
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one targeted grant, got {effects:#?}");
    };
    assert_eq!(*duration, Until::SourceUntaps);
    assert_eq!(*condition, Some(PredicateAst::Source(SourcePredicateAst::SourceIsTapped)));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.other);

    let dispatched = parse_effect_sentence_lexed(&tokens)
        .expect("top-level sentence dispatch should preserve the typed grant");
    assert!(matches!(
        dispatched.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                duration: Until::SourceUntaps,
                ..
            }),
            ..
        })]
    ));
    let lowered = compile_statement_effects(&dispatched)
        .expect("source-tapped grant should lower through the normal effect path");
    let lowered_debug = format!("{lowered:#?}");
    assert!(
        string_contains(&lowered_debug, "Shroud")
            && string_contains(&lowered_debug, "SourceUntaps"),
        "lowered grant must retain the keyword and source-tapped duration: {lowered_debug}"
    );
}

#[test]
fn source_tapped_compound_pump_and_hexproof_share_the_typed_duration() {
    let tokens = tokenize_line(
        "Target Wizard creature gets +2/+2 and has hexproof for as long as this creature remains tapped.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("compound source-tapped grant should parse")
        .expect("compound source-tapped grant should produce effects");

    let effect_slice = match effects.as_slice() {
        [EffectAst::Coordination(coordination)] => coordination.effects().collect::<Vec<_>>(),
        effects => effects.iter().collect::<Vec<_>>(),
    };
    assert_eq!(
        effect_slice.len(),
        2,
        "expected pump plus grant: {effects:#?}"
    );
    for effect in effect_slice {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            panic!("expected subject-verb effect, got {effect:#?}");
        };
        match action {
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { duration, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { duration, .. }) => {
                assert_eq!(*duration, Until::SourceUntaps)
            }
            _ => panic!("unexpected compound effect: {effect:#?}"),
        }
    }
}
