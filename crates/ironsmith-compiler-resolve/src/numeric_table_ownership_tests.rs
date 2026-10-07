//! Authored source assertions only; UNVALIDATED / UNRUN.
use super::*;
use crate::cards::builders::SubjectVerbRoleAst;

fn draw() -> EffectAst {
    EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: Value::Fixed(1) }),
    )
}

fn row(comparison: crate::effect::Comparison, effects: Vec<EffectAst>) -> EffectAst {
    EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::DieValue(comparison),
        effects,
    })
}

fn annotate(effects: &[EffectAst]) -> Result<AnnotatedEffectSequence, CardTextError> {
    annotate_effect_sequence(effects, &ReferenceImports::default(),
        EffectReferenceResolutionConfig::default(), IdGenContext::default())
}

#[test]
fn typed_numeric_rows_reject_generic_stale_optional_and_missing_die_owners() {
    let roll = EffectAst::subject_verb_roll_die(PlayerAst::You, 20);
    let high = row(crate::effect::Comparison::GreaterThanOrEqual(15), vec![draw()]);
    for effects in [
        vec![high.clone()],
        vec![draw(), high.clone()],
        vec![roll.clone(), draw(), high.clone()],
        vec![EffectAst::Permissions(PermissionEffectAst::May { effects: vec![roll] }), high],
    ] {
        assert!(annotate(&effects).is_err(), "an incompatible instruction cannot own a numeric die row");
    }
    // The new die-only requirement does not redefine ordinary numeric
    // result gates used by other effect families.
    assert!(annotate(&[draw(), EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::Value(crate::effect::Comparison::Equal(1)),
        effects: vec![draw()],
    })]).is_ok());
}

#[test]
fn sibling_die_rows_and_reannotation_keep_the_terminal_rolls_exact_id() {
    let effects = vec![
        draw(),
        EffectAst::subject_verb_roll_die(PlayerAst::You, 20),
        row(crate::effect::Comparison::LessThanOrEqual(14), vec![
            EffectAst::subject_verb_roll_die(PlayerAst::You, 20),
        ]),
        row(crate::effect::Comparison::GreaterThanOrEqual(15), vec![draw()]),
    ];
    let first = annotate(&effects).expect("terminal die owns both rows");
    let die_id = first.effects[1].assigned_effect_id.expect("die instruction exports its own ID");
    for gate in &first.effects[2..] {
        assert!(matches!(&gate.effect,
            EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult { condition, predicate: IfResultPredicate::DieValue(_), .. })
                if *condition == die_id));
    }
    let resolved = first.effects.iter().map(|effect| effect.effect.clone()).collect::<Vec<_>>();
    let second = annotate(&resolved).expect("lowering reannotation preserves the exact receipt");
    assert_eq!(second.effects[1].assigned_effect_id, Some(die_id));
    for gate in &second.effects[2..] {
        assert!(matches!(&gate.effect,
            EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult { condition, .. })
                if *condition == die_id));
    }
}

#[test]
fn wrapped_numeric_rows_reuse_the_resolved_die_id_with_a_nonzero_lowering_allocator() {
    let wrap = |effect| EffectAst::SourceSentence {
        effects: vec![effect], leading_then: false, starting_with_controller: false,
    };
    let effects = vec![
        EffectAst::subject_verb_roll_die(PlayerAst::You, 20),
        wrap(row(crate::effect::Comparison::LessThanOrEqual(14), vec![draw()])),
        wrap(row(crate::effect::Comparison::GreaterThanOrEqual(15), vec![draw()])),
    ];
    let first = annotate(&effects).expect("wrapped rows resolve against the terminal die");
    let die_id = first.effects[0].assigned_effect_id.unwrap();
    let resolved = first.effects.iter().map(|effect| effect.effect.clone()).collect::<Vec<_>>();
    let second = annotate_effect_sequence(&resolved, &ReferenceImports::default(),
        EffectReferenceResolutionConfig::default(), IdGenContext { next_effect_id: 100, ..Default::default() })
        .expect("wrapped resolved gates preserve their ID during lowering");
    assert_eq!(second.effects[0].assigned_effect_id, Some(die_id));
    for gate in &second.effects[1..] {
        assert_eq!(resolved_result_gate_id(&gate.effect), Some(die_id));
    }
}
