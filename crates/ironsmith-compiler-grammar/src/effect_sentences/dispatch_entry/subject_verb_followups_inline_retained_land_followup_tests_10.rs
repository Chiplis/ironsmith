use super::*;

fn animation() -> EffectAst {
    EffectAst::subject_verb_become_base_pt_creature(
        Value::Fixed(3),
        Value::Fixed(3),
        TargetAst::Source(None),
        vec![crate::types::CardType::Creature],
        Vec::new(),
        Vec::new(),
        None,
        Vec::new(),
        Vec::new(),
        false,
        None,
        Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness),
        None,
        Until::EndOfTurn,
    )
}

#[test]
fn still_land_followup_reaches_animation_inside_conditional_may() {
    let mut effects = vec![EffectAst::Conditional {
        predicate: PredicateAst::SourceIsTapped,
        if_true: vec![EffectAst::May {
            effects: vec![animation()],
        }],
        if_false: Vec::new(),
    }];

    assert!(mark_last_animation_as_still_a_land(&mut effects));
    let EffectAst::Conditional { if_true, .. } = &effects[0] else {
        panic!("expected conditional wrapper");
    };
    let [EffectAst::May { effects }] = if_true.as_slice() else {
        panic!("expected may wrapper");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeBasePtCreature {
                    preserve_other_types,
                    type_retention_surface,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected nested animation");
    };

    assert!(*preserve_other_types);
    assert_eq!(
        *type_retention_surface,
        Some(ironsmith_core::TypeRetentionSurface::StillALand)
    );
}
