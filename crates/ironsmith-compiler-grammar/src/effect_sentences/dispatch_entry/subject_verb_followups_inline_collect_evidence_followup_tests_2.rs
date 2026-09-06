use super::*;

#[test]
fn optional_self_exile_collects_a_real_aggregate_evidence_set() {
    let tokens = crate::lexer::lex_line(
            "You may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.",
            0,
        )
        .expect("collect-evidence procedure should lex");
    let effects =
        parse_effect_sentences_lexed(&tokens).expect("collect-evidence procedure should parse");
    let [
        EffectAst::Permissions(PermissionEffectAst::May { effects: optional }),
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            effects: returned, ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one optional procedure and linked return: {effects:#?}");
    };
    let [
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
            filter,
            count,
            tag: evidence_tag,
            constraint,
            ..
        }),
        EffectAst::TagAffected {
            tag: source_exiled_tag,
            ..
        },
        EffectAst::MoveTaggedGroupToZone {
            tag: moved_evidence_tag,
            zone: Zone::Exile,
        },
    ] = optional.as_slice()
    else {
        panic!("expected choose, source exile, and evidence exile: {optional:#?}");
    };
    assert!(count.is_any_number());
    assert_eq!(evidence_tag, moved_evidence_tag);
    assert!(matches!(
        constraint.minimum.as_ref().map(|value| value.unhinted()),
        Some(Value::Fixed(4))
    ));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "triggering"
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    }));
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    target,
                    tapped: true,
                    ..
                }),
            ..
        }),
    ] = returned.as_slice()
    else {
        panic!("expected a tapped return from exile: {returned:#?}");
    };
    assert!(matches!(
        target,
        TargetAst::Tagged(tag, _) if tag == source_exiled_tag
    ));
}

#[test]
fn plain_optional_self_exile_does_not_gain_evidence_selection() {
    let tokens = crate::lexer::lex_line(
        "You may exile it. If you do, return this card to the battlefield tapped.",
        0,
    )
    .expect("plain optional exile should lex");
    let effects = parse_effect_sentences_lexed(&tokens).expect("plain optional exile should parse");
    assert!(
        !format!("{effects:#?}").contains("ChooseObjectsWithAggregateConstraint"),
        "evidence must not be inferred without the keyword action: {effects:#?}"
    );
}
