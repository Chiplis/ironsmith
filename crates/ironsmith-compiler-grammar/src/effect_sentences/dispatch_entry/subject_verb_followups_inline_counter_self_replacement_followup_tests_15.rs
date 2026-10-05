use super::*;

#[test]
fn double_counters_on_that_creature_reuses_the_default_target() {
    let lexed = crate::lexer::lex_line(
            "Put a +1/+1 counter on target creature you control. If this is the second time this ability has resolved this turn, double the number of +1/+1 counters on that creature instead.",
            0,
        )
        .expect("counter self-replacement should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("counter self-replacement should parse");
    let [
        EffectAst::SelfReplacement {
            if_true, if_false, ..
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one typed self-replacement: {parsed:#?}");
    };

    assert!(
        matches!(
            if_true.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Counters(
                    CounterActionAst::DoubleCountersOnTarget { .. }
                ),
                ..
            })]
        ),
        "the replacement branch should keep the typed double-counter action: {if_true:#?}"
    );
    let default_target = primary_target_from_effect(&if_false[0])
        .expect("the default counter effect should have a target");
    let replacement_target = primary_target_from_effect(&if_true[0])
        .expect("the replacement counter effect should reuse that target");
    if replacement_target == default_target {
        assert!(target_is_explicitly_chosen(&replacement_target));
    } else {
        let TargetAst::Object(filter, ..) = &replacement_target else {
            panic!("replacement must retain the target alias: {parsed:#?}");
        };
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint
                .tag
                .as_str()
                .starts_with("__sentence_helper_self_replacement_antecedent_")
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(target_is_explicitly_chosen(&default_target));
    }

    let lowered = crate::compile_support::compile_statement_effects_with_imports(
        &parsed,
        &crate::model::reference_state::ReferenceImports::default(),
    )
    .expect("counter self-replacement should lower");
    let [segment] = lowered.effects.segments.as_slice() else {
        panic!(
            "expected one self-replacement segment: {:#?}",
            lowered.effects
        );
    };
    let [put_counters] = segment.default_effects.as_slice() else {
        panic!("one default counter action: {segment:#?}");
    };
    let tagged = put_counters
        .downcast_ref::<crate::effects::TaggedEffect>()
        .unwrap();
    let target_tag = tagged.tag.clone();
    let put_counters = tagged
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .unwrap();
    assert!(
        matches!(put_counters.target.unhinted(), ChooseSpec::Target(_)),
        "the authored target must remain announced: {put_counters:#?}"
    );
    let [replacement] = segment.self_replacements[0].replacement_effects.as_slice() else {
        panic!("expected one replacement counter action: {segment:#?}");
    };
    let replacement = replacement
        .downcast_ref::<crate::effects::DoubleCountersEffect>()
        .expect("the replacement branch should double counters");
    assert!(
        matches!(replacement.target.base(), ChooseSpec::Tagged(tag) if tag == &target_tag),
        "the replacement must reuse the one authored target alias: {replacement:#?}"
    );
    assert!(
        lowered.choices.is_empty(),
        "the canonical segment owns its target declaration instead of duplicating it in the outer choice list: {:#?}",
        lowered.choices
    );
}
