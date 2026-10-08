//! Authored source assertions only; UNVALIDATED / UNRUN.
use super::*;

fn sentence(effects: Vec<EffectAst>) -> EffectAst {
    EffectAst::SourceSentence { effects, leading_then: false, starting_with_controller: false }
}

fn draw() -> EffectAst {
    EffectAst::subject_verb(SubjectVerbRoleAst::AffectedPlayer, PlayerAst::You,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: Value::Fixed(1) }))
}

fn row(comparison: crate::effect::Comparison) -> EffectAst {
    sentence(vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::DieValue(comparison), effects: vec![draw()],
    })])
}

#[test]
fn source_wrapped_rows_enter_the_exact_terminal_roll_program_without_losing_source_wrappers() {
    let roll = EffectAst::subject_verb_roll_die(PlayerAst::You, 20);
    let low = row(crate::effect::Comparison::LessThanOrEqual(14));
    let high = row(crate::effect::Comparison::GreaterThanOrEqual(15));
    for comma_then in [false, true] {
        let preceding = if comma_then {
            EffectAst::CommaThen { effects: vec![draw(), roll.clone()] }
        } else {
            sentence(vec![draw(), roll.clone()])
        };
        let expected = if comma_then {
            EffectAst::CommaThen { effects: vec![draw(), roll.clone(), low.clone(), high.clone()] }
        } else {
            sentence(vec![draw(), roll.clone(), low.clone(), high.clone()])
        };
        let mut effects = vec![preceding, low.clone(), high.clone()];
        transport_die_result_rows_into_owner(&mut effects);
        assert_eq!(effects, vec![expected]);
    }
}

#[test]
fn stale_and_optional_rolls_do_not_acquire_following_numeric_rows() {
    let roll = EffectAst::subject_verb_roll_die(PlayerAst::You, 20);
    for owner in [
        sentence(vec![roll.clone(), draw()]),
        sentence(vec![EffectAst::Permissions(PermissionEffectAst::May { effects: vec![roll] })]),
    ] {
        let mut effects = vec![owner, row(crate::effect::Comparison::GreaterThanOrEqual(15))];
        let unchanged = effects.clone();
        transport_die_result_rows_into_owner(&mut effects);
        assert_eq!(effects, unchanged, "ambiguous owners must be rejected by resolution");
    }
}
