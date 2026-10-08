//! Source-authored cases. UNRUN while campaign execution is deferred.
use super::clause_pattern_helpers::{parse_prevent_all_damage_clause, parse_prevent_next_damage_clause};
use crate::cards::builders::{DamagePreventionActionAst, EffectAst, SubjectVerbActionAst, TargetAst};
use crate::effect::Until;

#[test]
fn chosen_color_keeps_the_declared_recipient_and_fixed_source_qualities() {
    for text in [
        "Prevent all damage that would be dealt to another target creature this turn by sources of the color of your choice.",
        "Prevent all damage that would be dealt to target player or planeswalker this turn by sources of the color of your choice.",
        "Prevent all damage that artifact sources of the color of your choice would deal to target creature this turn.",
    ] {
        let ast = parse_prevent_all_damage_clause(&crate::lexer::lex_line(text, 0).unwrap()).unwrap().unwrap();
        let EffectAst::SubjectVerb(subject) = ast else { panic!("typed subject") };
        let SubjectVerbActionAst::DamagePrevention(
            DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                target, duration, of_chosen_color, ..
            },
        ) = subject.action else { panic!("filtered target shield") };
        assert!(of_chosen_color);
        assert_eq!(duration, Until::EndOfTurn);
        assert!(!matches!(target, TargetAst::Source(_)));
    }
}

#[test]
fn finite_combat_kind_reaches_the_typed_action_without_changing_amount() {
    let ast = parse_prevent_next_damage_clause(&crate::lexer::lex_line(
        "Prevent the next 1 combat damage that would be dealt to you this turn.", 0,
    ).unwrap()).unwrap().unwrap();
    let EffectAst::SubjectVerb(subject) = ast else { panic!("typed subject") };
    assert!(matches!(subject.action, SubjectVerbActionAst::DamagePrevention(
        DamagePreventionActionAst::PreventDamage { combat_only: true, amount: crate::effect::Value::Fixed(1), .. }
    )));
}
