use super::*;

#[test]
fn it_deals_to_that_creature_ignores_prior_cost_object_provenance() {
    let lexed = crate::lexer::lex_line(
            "This deals 2 damage to target creature. It deals 4 damage to that creature instead if this spell's additional cost was paid.",
            0,
        )
        .expect("damage self-replacement should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("damage self-replacement should parse");
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
                action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    source: TargetAst::Source(_),
                    target: TargetAst::Object(filter, None, Some(_)),
                    ..
                }),
                ..
            })] if !filter.tagged_constraints.is_empty()
        ),
        "the replacement should directly reuse the default spell source and target: {if_true:#?}"
    );
    assert_eq!(if_false.len(), 1, "default damage should remain intact");
    assert!(
        !format!("{parsed:#?}").contains("TrailingIf"),
        "the authored trailing-if surface must be consumed by the typed self-replacement: {parsed:#?}"
    );

    let lowered = crate::compile_support::compile_statement_effects_with_imports(
        &parsed,
        &crate::model::reference_state::ReferenceImports::with_last_object_tag("counters_0"),
    )
    .expect("damage self-replacement should lower");
    let debug = format!("{lowered:#?}");
    assert!(debug.contains("ExecuteWithSourceEffect"), "{debug}");
    assert!(debug.contains("source: Source"), "{debug}");
    assert!(!debug.contains("ForEachObject"), "{debug}");
    assert!(!debug.contains("counters_0"), "{debug}");
}

#[test]
fn omitted_damage_target_reuses_the_default_target() {
    let lexed = crate::lexer::lex_line(
            "This deals 3 damage to target creature. It deals 5 damage instead if you control an artifact.",
            0,
        )
        .expect("damage self-replacement should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("damage self-replacement should parse");
    let [EffectAst::SelfReplacement { if_true, if_false, .. }] = parsed.as_slice() else {
        panic!("expected one typed self-replacement: {parsed:#?}");
    };
    assert_eq!(
        primary_damage_target_from_effect(&if_true[0]),
        primary_damage_target_from_effect(&if_false[0]),
        "the omitted replacement target must preserve the original declaration"
    );
    assert_eq!(
        primary_damage_source_from_effect(&if_true[0]),
        primary_damage_source_from_effect(&if_false[0]),
    );
    assert_eq!(sole_damage_payload(if_true), Some((Value::Fixed(5), false)));
    assert_eq!(sole_damage_payload(if_false), Some((Value::Fixed(3), false)));
}

#[test]
fn leading_conditional_amount_reuses_the_complete_damage_instruction() {
    for (base, tail) in [
        ("This deals 2 damage to any target", "If this spell was kicked, it deals 4 damage instead"),
        ("This deals 2 damage to target creature", "If you control an artifact, this spell deals 4 damage instead"),
        ("This deals 2 damage to target creature or planeswalker", "If that permanent is green, this spell deals 4 damage instead"),
    ] {
        let tokens = crate::lexer::lex_line(&format!("{base}. {tail}."), 0).unwrap();
        let (result, loss) = crate::parse_loss::capture(|| parse_effect_sentences_lexed(&tokens));
        let effects = result.unwrap();
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let [EffectAst::SelfReplacement { if_true, if_false, .. }] = effects.as_slice() else {
            panic!("one bound replacement expected: {effects:#?}");
        };
        assert_eq!(if_true.len(), 1);
        assert_eq!(if_false.len(), 1);
        assert_eq!(sole_damage_payload(if_true), Some((Value::Fixed(4), false)));
        assert_eq!(sole_damage_payload(if_false), Some((Value::Fixed(2), false)));
        assert_eq!(primary_damage_target_from_effect(&if_true[0]), primary_damage_target_from_effect(&if_false[0]));
        assert_eq!(primary_damage_source_from_effect(&if_true[0]), primary_damage_source_from_effect(&if_false[0]));
    }
}

#[test]
fn amount_replacement_does_not_claim_a_destination_rider_or_unbound_event_amount() {
    for tail in [
        "If this spell was kicked, it deals 4 damage to target player instead.",
        "If this spell was kicked, it deals 4 damage instead and you draw a card.",
        "If this spell was kicked, that creature deals 4 damage instead.",
        "If this spell was kicked, it deals twice that much damage instead.",
        "If this spell was kicked, it deals 4 damage instead instead.",
    ] {
        let base = crate::lexer::lex_line("This deals 2 damage to any target.", 0).unwrap();
        let mut effects = parse_effect_sentences_lexed(&base).unwrap();
        let original = effects.clone();
        let mut carried = None;
        let mut state = SentenceDispatchState { effects: &mut effects, carried_context: &mut carried };
        let tokens = crate::lexer::lex_line(tail, 0).unwrap();
        assert!(pre_rule_damage_amount_replacement(&mut state, &[], 0, &tokens).unwrap().is_none(), "{tail}");
        assert_eq!(effects, original, "{tail}");
    }
    let orphan = crate::lexer::lex_line("If this spell was kicked, it deals 4 damage instead.", 0).unwrap();
    assert!(parse_effect_sentences_lexed(&orphan).is_err());
}

#[test]
fn cast_time_control_cannot_be_substituted_with_current_control() {
    let tokens = crate::lexer::lex_line(
        "This spell deals X damage to target creature. If you controlled a modified creature as you cast this spell, it deals X plus 2 damage instead.",
        0,
    ).unwrap();
    let error = parse_effect_sentences_lexed(&tokens).unwrap_err();
    assert!(error.to_string().contains("retained cast-time control evidence"), "{error}");
}
