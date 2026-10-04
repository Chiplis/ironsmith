use super::*;
use crate::cards::builders::{DamagePreventionActionAst, SubjectVerbEffectAst};

#[test]
fn combat_recipient_prevention_has_a_typed_scope_and_duration() {
    for text in [
        "Prevent all combat damage that would be dealt to target creature this turn.",
        "Prevent all combat damage that would be dealt this turn to Dogs you control.",
        "Prevent all combat damage that would be dealt to you and creatures you control this turn.",
    ] {
        let ast = parse_prevent_damage_sentence_lexed(&lex_line(text, 0).unwrap()).unwrap().unwrap();
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget { combat_only, duration, .. }), .. }) = ast else { panic!("typed combat recipient") };
        assert!(combat_only);
        assert_eq!(duration, crate::effect::Until::EndOfTurn);
    }
}

#[test]
fn combat_prevention_does_not_omit_recipient_condition_or_expiry() {
    for text in [
        "Prevent all combat damage target creature would deal to you this turn.",
        "Prevent all combat damage that would be dealt to you this combat this turn.",
        "Prevent all combat damage that would be dealt to you this turn. Draw a card.",
        "Prevent all combat damage that would be dealt to target creature this turn unless its controller pays 1 life.",
    ] {
        assert!(!matches!(parse_prevent_damage_sentence_lexed(&lex_line(text, 0).unwrap()), Ok(Some(_))), "{text}");
    }
}

#[test]
fn active_source_clauses_keep_targets_and_their_cardinality() {
    let ast = crate::effect_sentences::clause_pattern_helpers::parse_prevent_all_damage_clause(&lex_line("Prevent all damage one or two target creatures would deal this turn.", 0).unwrap()).unwrap().unwrap();
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget { source_target: Some(TargetAst::WithCount(_, count)), combat_only, .. }), .. }) = ast else { panic!("counted source target retained") };
    assert!(!combat_only);
    assert_eq!(count.min, 1);
    assert_eq!(count.max, Some(2));
    let ast = crate::effect_sentences::clause_pattern_helpers::parse_prevent_all_damage_clause(&lex_line("Prevent all damage that would be dealt by creatures this turn.", 0).unwrap()).unwrap().unwrap();
    assert!(matches!(ast, EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageFromSourceFilter { .. }), .. })));
}


#[test]
fn this_combat_is_retained_as_combat_duration() {
    let ast = parse_prevent_damage_sentence_lexed(&lex_line("Prevent all combat damage that would be dealt to target creature this combat.", 0).unwrap()).unwrap().unwrap();
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget { combat_only, duration, .. }), .. }) = ast else { panic!("typed combat recipient") };
    assert!(combat_only);
    assert_eq!(duration, crate::effect::Until::EndOfCombat);
}
