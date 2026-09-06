use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::GrantActionAst;
use super::*;

fn animation_pt_surface(text: &str) -> ironsmith_core::AnimationPtSurface {
    let subject =
        crate::lexer::lex_line("target artifact", 0).expect("animation subject should lex");
    let animation = crate::lexer::lex_line(text, 0).expect("animation predicate should lex");
    let effect = parse_become_clause(&subject, &animation)
        .expect("animation should parse through the generic become clause");
    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        action:
            crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                animation_pt_surface: Some(surface),
                ..
            }),
        ..
    }) = effect
    else {
        panic!("expected typed animation surface, got {effect:#?}");
    };
    surface
}

#[test]
fn leading_and_explicit_base_pt_animation_surfaces_remain_distinct() {
    assert_eq!(
        animation_pt_surface("a 4/4 Angel artifact creature"),
        ironsmith_core::AnimationPtSurface::LeadingPowerToughness
    );
    assert_eq!(
        animation_pt_surface("an Angel artifact creature with base power and toughness 4/4"),
        ironsmith_core::AnimationPtSurface::ExplicitBasePowerToughness
    );
}

#[test]
fn triggering_spell_color_protection_becomes_exact_color_gated_grants() {
    let subject =
        crate::lexer::lex_line("this enchantment", 0).expect("animation subject should lex");
    let body = crate::lexer::lex_line(
        "a 4/4 Giant creature with protection from each of that spell's colors",
        0,
    )
    .expect("dynamic protection animation should lex");
    let effect = parse_become_clause(&subject, &body)
        .expect("dynamic protection animation should parse structurally");
    let EffectAst::Coordinated { effects, .. } = effect else {
        panic!("expected a coordinated animation and grants: {effect:#?}");
    };
    assert_eq!(effects.len(), 6, "{effects:#?}");
    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        action: crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature { subtypes, .. }),
        ..
    }) = &effects[0]
    else {
        panic!(
            "first effect should retain the animation: {:#?}",
            effects[0]
        );
    };
    assert_eq!(subtypes, &[crate::types::Subtype::Giant]);

    for effect in &effects[1..] {
        let EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(tag, filter),
            if_true,
            if_false,
        }) = effect
        else {
            panic!("expected a tagged-color conditional grant: {effect:#?}");
        };
        assert_eq!(tag.as_str(), "triggering");
        assert!(filter.colors.is_some());
        assert!(if_false.is_empty());
        assert!(matches!(
            if_true.as_slice(),
            [EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Source(_),
                        ..
                    }),
                    ..
                }
            )]
        ));
    }
}

#[test]
fn leading_and_trailing_animation_durations_remain_distinct() {
    let leading_subject = crate::lexer::lex_line("until end of turn target land you control", 0)
        .expect("leading-duration animation subject should lex");
    let trailing_subject = crate::lexer::lex_line("target land you control", 0)
        .expect("trailing-duration animation subject should lex");
    let leading_body = crate::lexer::lex_line("a 4/4 Dinosaur creature with reach and haste", 0)
        .expect("leading-duration animation body should lex");
    let trailing_body = crate::lexer::lex_line(
        "a 4/4 Dinosaur creature with reach and haste until end of turn",
        0,
    )
    .expect("trailing-duration animation body should lex");

    let leading = parse_become_clause(&leading_subject, &leading_body)
        .expect("leading-duration animation should parse");
    let trailing = parse_become_clause(&trailing_subject, &trailing_body)
        .expect("trailing-duration animation should parse");
    let duration_surface = |effect: EffectAst| {
        let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                    animation_duration_surface,
                    duration,
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected typed animation duration surface, got {effect:#?}");
        };
        assert_eq!(duration, Until::EndOfTurn);
        animation_duration_surface
    };

    assert_eq!(
        duration_surface(leading),
        Some(ironsmith_core::AnimationDurationSurface::Leading)
    );
    assert_eq!(duration_surface(trailing), None);
}

#[test]
fn duration_inside_unclosed_sentence_quote_is_not_taken_as_outer_duration() {
    let tokens = crate::lexer::lex_line(
            "a 2/4 Wizard creature with \"Whenever you cast an instant or sorcery spell, this creature gets +1/+0 until end of turn.",
            0,
        )
        .expect("quoted animation should lex");
    let (_, remainder) = parse_restriction_duration(&tokens)
        .expect("duration parsing should succeed")
        .expect("inner duration should be recognized as a suffix");

    assert!(trailing_duration_belongs_to_quoted_ability(
        &tokens, &remainder
    ));
}

#[test]
fn duration_after_balanced_quote_remains_the_outer_duration() {
    let tokens = crate::lexer::lex_line(
        "a 1/1 Skeleton creature with \"{B}: Regenerate this creature.\" until end of turn",
        0,
    )
    .expect("quoted animation should lex");
    let (_, remainder) = parse_restriction_duration(&tokens)
        .expect("duration parsing should succeed")
        .expect("outer duration should be recognized as a suffix");

    assert!(!trailing_duration_belongs_to_quoted_ability(
        &tokens, &remainder
    ));
}

#[test]
fn aura_animation_preserves_balanced_quoted_ability_grant() {
    let subject = crate::lexer::lex_line("it", 0).expect("lex subject");
    let body = crate::lexer::lex_line(
            "an Aura enchantment with enchant creature you control and \"{G}{W}: Enchanted creature gains indestructible until end of turn,\"",
            0,
        )
        .expect("lex Aura animation");
    let effect = parse_become_clause(&subject, &body).expect("parse Aura animation");
    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        action:
            crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                attachment_filter,
                granted_abilities,
                ..
            }),
        ..
    }) = effect
    else {
        panic!("expected typed Aura animation with grant: {effect:#?}");
    };
    assert_eq!(attachment_filter, ObjectFilter::creature().you_control());
    assert_eq!(granted_abilities.len(), 1, "{granted_abilities:#?}");

    let plain_body =
        crate::lexer::lex_line("an Aura enchantment with enchant creature you control", 0)
            .expect("lex plain Aura animation");
    let plain = parse_become_clause(&subject, &plain_body).expect("parse plain Aura animation");
    assert!(matches!(
        plain,
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                    granted_abilities,
                    ..
                }),
            ..
        }) if granted_abilities.is_empty()
    ));
}

#[test]
fn unclosed_sentence_quote_keeps_animation_descriptor_and_granted_trigger() {
    let subject = crate::lexer::lex_line("until end of turn enchanted Plains", 0)
        .expect("animation subject should lex");
    let body = crate::lexer::lex_line(
            "a 2/5 white Spirit creature with \"Whenever this creature deals damage, its controller gains that much life",
            0,
        )
        .expect("quoted animation body should lex");
    let effect = parse_become_clause(&subject, &body).expect("quoted land animation should parse");
    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        action:
            crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                subtypes,
                colors: Some(colors),
                granted_abilities,
                ..
            }),
        ..
    }) = effect
    else {
        panic!("expected a typed animation bundle, got {effect:#?}");
    };

    assert_eq!(subtypes, vec![crate::types::Subtype::Spirit]);
    assert!(colors.contains(crate::color::Color::White), "{colors:?}");
    assert_eq!(granted_abilities.len(), 1, "{granted_abilities:#?}");
}
