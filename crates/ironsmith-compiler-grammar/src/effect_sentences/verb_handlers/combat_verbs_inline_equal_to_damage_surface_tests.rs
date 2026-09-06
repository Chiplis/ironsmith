use super::*;
use crate::lexer::lex_line;

#[test]
fn historical_mixed_damage_recipients_keep_both_typed_filters() {
    let tokens = lex_line(
        "1 damage to each opponent and planeswalker it has dealt damage to this game",
        0,
    )
    .expect("historical damage clause should lex");
    let effect = parse_deal_damage(&tokens).expect("historical damage clause should parse");
    let debug = format!("{effect:#?}");
    assert!(
        debug.contains("WasDealtDamageBySourceThisGame"),
        "player history filter missing: {debug}"
    );
    assert!(
        debug.contains("was_dealt_damage_by_source_this_game: true"),
        "object history filter missing: {debug}"
    );
    assert!(
        debug.contains("Planeswalker"),
        "object domain missing: {debug}"
    );
    assert_eq!(debug.matches("DealDamage").count(), 2, "{debug}");
}

#[test]
fn public_sentence_route_keeps_historical_player_object_union_atomic() {
    let body = lex_line(
        "1 damage to each opponent and planeswalker it has dealt damage to this game",
        0,
    )
    .expect("historical damage body should lex");
    assert!(is_historical_player_object_damage_recipient_clause(&body));

    let tokens = lex_line(
        "This creature deals 1 damage to each opponent and planeswalker it has dealt damage to this game.",
        0,
    )
    .expect("historical damage sentence should lex");
    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("public sentence route should parse");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("WasDealtDamageBySourceThisGame"), "{debug}");
    assert!(
        debug.contains("was_dealt_damage_by_source_this_game: true"),
        "{debug}"
    );

    let changed = lex_line(
        "This creature deals 1 damage to each opponent and planeswalker it has dealt damage to this turn.",
        0,
    )
    .expect("changed-duration sentence should lex");
    if let Ok(effects) = crate::effect_sentences::parse_effect_sentence_lexed(&changed) {
        let debug = format!("{effects:#?}");
        assert!(!debug.contains("WasDealtDamageBySourceThisGame"), "{debug}");
    }
}

#[test]
fn damage_to_each_of_those_preserves_the_demonstrative_set_surface() {
    let tokens = lex_line("X damage to each of those creatures", 0)
        .expect("demonstrative damage clause should lex");
    let effect = parse_deal_damage(&tokens).expect("demonstrative damage clause should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { filter, .. }),
        ..
    }) = effect
    else {
        panic!("expected typed damage fanout: {effect:#?}");
    };

    assert_eq!(
        filter.set_quantifier_surface(),
        Some(ironsmith_core::SetQuantifierSurface::Those)
    );
}

#[test]
fn each_damage_except_your_keyword_bearers_keeps_the_boolean_complement() {
    let tokens = lex_line(
        "2 damage to each creature except for creatures you control with flying",
        0,
    )
    .expect("excluded damage clause should lex");
    let effect = parse_deal_damage(&tokens).expect("excluded damage clause should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { filter, .. }),
        ..
    }) = effect
    else {
        panic!("expected typed damage fanout: {effect:#?}");
    };

    assert_eq!(filter.card_types, [CardType::Creature]);
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(filter.any_of[0].controller, Some(PlayerFilter::NotYou));
    assert_eq!(
        filter.any_of[1].excluded_static_abilities,
        [crate::static_abilities::StaticAbilityId::Flying]
    );

    let near_miss = lex_line(
        "2 damage to each creature except for creatures an opponent controls with flying",
        0,
    )
    .unwrap();
    let near_miss = parse_deal_damage(&near_miss).unwrap();
    let debug = format!("{near_miss:#?}");
    assert!(!debug.contains("NotYou"), "{debug}");
}

#[test]
fn each_damage_keeps_a_complete_serial_negative_keyword_filter() {
    use crate::static_abilities::StaticAbilityId::{DoubleStrike, FirstStrike, Haste, Vigilance};

    let tokens = lex_line(
        "1 damage to each creature that doesn't have first strike, double strike, vigilance, or haste",
        0,
    )
    .expect("serial negative keyword damage clause should lex");
    let effect = parse_deal_damage(&tokens)
        .expect("serial negative keyword damage clause should parse at the verb boundary");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { filter, .. }),
        ..
    }) = effect
    else {
        panic!("expected typed damage fanout: {effect:#?}");
    };

    assert_eq!(
        filter.excluded_static_abilities,
        [FirstStrike, DoubleStrike, Vigilance, Haste],
        "the damage handler must own the complete filter without a dispatcher repair"
    );
    let each_index = tokens
        .iter()
        .position(|token| token.is_word("each"))
        .expect("damage clause should contain each");
    let direct_filter =
        crate::object_filters::parse_object_filter_lexed(&tokens[each_index + 1..], false)
            .expect("the same complete filter should parse directly");
    assert_eq!(
        filter, direct_filter,
        "the damage handler must not normalize away typed filter facts"
    );
}

#[test]
fn fixed_plus_count_damage_keeps_equal_to_surface() {
    let tokens = lex_line(
        "damage equal to 2 plus the number of Lesson cards in your graveyard to target creature",
        0,
    )
    .expect("equal-to damage should lex");
    let effect = parse_deal_damage_equal_to_clause(&tokens)
        .expect("equal-to damage should parse")
        .expect("equal-to damage should match");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, .. }),
        ..
    }) = effect
    else {
        panic!("expected typed damage effect");
    };

    assert!(amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo));
    assert!(matches!(amount.unhinted(), Value::Add(_, _)));
}

#[test]
fn equal_to_damage_keeps_authored_optional_single_target() {
    for (target_words, expected_count) in [
        (
            "up to one target creature or planeswalker",
            ChoiceCount::up_to(1),
        ),
        ("target creature or planeswalker", ChoiceCount::exactly(1)),
    ] {
        let tokens = lex_line(
            &format!("damage equal to that card's mana value to {target_words}"),
            0,
        )
        .expect("linked damage clause should lex");
        let effect = parse_deal_damage_equal_to_clause(&tokens)
            .expect("linked damage clause should parse")
            .expect("linked damage clause should match");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, target, .. }),
            ..
        }) = effect
        else {
            panic!("expected typed damage effect");
        };
        let actual_count = match target {
            TargetAst::WithCount(_, count) => count,
            _ => ChoiceCount::exactly(1),
        };

        assert_eq!(actual_count, expected_count, "{target_words}");
        assert!(amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo));
        assert!(matches!(amount.unhinted(), Value::ManaValueOf(_)));
    }
}

#[test]
fn relative_controller_count_has_unique_typed_amount_ownership() {
    let tokens = lex_line(
            "damage to target creature equal to the number of nonbasic lands that creature's controller controls",
            0,
        )
        .expect("relative-controller damage should lex");
    let effect = parse_deal_damage(&tokens).expect("relative-controller damage should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                amount,
                target: TargetAst::Object(target, Some(_), _),
                ..
            }),
        ..
    }) = effect
    else {
        panic!("expected typed targeted damage: {effect:#?}");
    };
    let Value::Count(counted) = amount.unhinted() else {
        panic!("expected typed land count: {amount:#?}");
    };

    assert_eq!(target.card_types, vec![crate::CardType::Creature]);
    assert_eq!(counted.card_types, vec![crate::CardType::Land]);
    assert!(!counted.card_types.contains(&crate::CardType::Creature));
    assert!(
        counted
            .excluded_supertypes
            .contains(&crate::Supertype::Basic)
    );
    assert_eq!(
        counted.controller,
        Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
    );
}

#[test]
fn player_or_planeswalker_controller_count_stays_inside_damage_amount() {
    let body = lex_line(
        "damage to you equal to the number of creatures that opponent or that planeswalker's controller controls",
        0,
    )
    .expect("controller-relative damage body should lex");
    let effect = parse_deal_damage(&body).expect("controller-relative damage body should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, .. }),
        ..
    }) = effect
    else {
        panic!("expected typed damage effect: {effect:#?}");
    };
    let Value::Count(filter) = amount.unhinted() else {
        panic!("expected typed creature count: {amount:#?}");
    };
    assert_eq!(
        filter.controller,
        Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
    );

    let sentence = lex_line(
        "This artifact deals damage to you equal to the number of creatures that opponent or that planeswalker's controller controls.",
        0,
    )
    .expect("complete damage sentence should lex");
    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&sentence)
        .expect("complete damage sentence should remain atomic");
    assert_eq!(effects.len(), 1, "{effects:#?}");

    let changed = lex_line(
        "This artifact deals damage to you equal to the number of creatures that opponent or that planeswalker's controller destroys.",
        0,
    )
    .expect("changed action should lex");
    if let Ok(changed_effects) = crate::effect_sentences::parse_effect_sentence_lexed(&changed) {
        let debug = format!("{changed_effects:#?}");
        assert!(
            !debug.contains("TargetPlayerOrControllerOfTarget"),
            "a different terminal action must not acquire controller-count semantics: {debug}"
        );
    }
}

#[test]
fn authored_damage_recipient_pronoun_is_preserved_on_the_amount() {
    for (text, expects_hint) in [
        ("2 damage to them", true),
        ("2 damage to that player", false),
    ] {
        let tokens = lex_line(text, 0).expect("damage clause should lex");
        let effect = parse_deal_damage(&tokens).expect("damage clause should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                    amount,
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, _),
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected iterated-player damage for {text}: {effect:#?}");
        };

        assert_eq!(
            amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::DamageRecipientPronoun),
            expects_hint,
            "{text}"
        );
    }
}

#[test]
fn target_spell_controller_damage_materializes_the_spell_target_first() {
    for text in [
        "damage to target spell's controller equal to that spell's mana value",
        "damage equal to that spell's mana value to target spell's controller",
    ] {
        let tokens = lex_line(text, 0).expect("stack-target damage should lex");
        let effect = if text.starts_with("damage to") {
            parse_deal_damage_to_target_equal_to_clause(&tokens)
        } else {
            parse_deal_damage_equal_to_clause(&tokens)
        }
        .expect("stack-target damage should parse")
        .expect("stack-target damage should match");
        let EffectAst::Sequence { effects } = effect else {
            panic!("expected target prelude plus damage for {text}: {effect:#?}");
        };
        let [target, damage] = effects.as_slice() else {
            panic!("expected exactly two typed effects for {text}: {effects:#?}");
        };
        assert!(matches!(
            target,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::TargetOnly {
                    target: TargetAst::Spell(Some(_)),
                    explicit_declaration: false,
                },
                ..
            })
        ));
        assert!(matches!(
            damage,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                        amount,
                        target:
                            TargetAst::Player(
                                PlayerFilter::ControllerOf(
                                    crate::target::ObjectRef::Tagged(tag)
                                ),
                                None
                            ),
                        ..
                    }),
                ..
            }) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && matches!(
                    amount.unhinted(),
                    Value::ManaValueOf(spec)
                        if matches!(
                            spec.unhinted(),
                            ChooseSpec::Tagged(value_tag) if value_tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        )
                )
        ));
    }
}
