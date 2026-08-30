use super::*;
use crate::CardType;
use crate::lexer::lex_line;
use crate::model::ast::SubjectVerbEffectAst;

fn lex_tail(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("lex test tail")
}

#[test]
fn targeting_as_though_permission_keeps_affected_set_and_source_controller_scope() {
    let static_tokens = lex_tail(
        "Creatures your opponents control with hexproof can be the targets of spells and abilities you control as though they didn't have hexproof.",
    );
    let static_spec = parse_targeting_as_though_no_ability_spec(&static_tokens)
        .expect("static permission should parse")
        .expect("static permission should be claimed");
    assert_eq!(static_spec.ignored_ability, StaticAbilityId::Hexproof);
    assert_eq!(static_spec.sources_controlled_by, PlayerFilter::You);
    let objects = static_spec.objects.expect("creature set");
    assert_eq!(objects.card_types, [CardType::Creature]);
    assert_eq!(objects.controller, Some(PlayerFilter::Opponent));
    assert!(
        crate::effect_sentences::gain_ability::parse_gain_ability_sentence(&static_tokens)
            .expect("gain parser should reject the targeting domain")
            .is_none()
    );

    let temporary_spec = parse_targeting_as_though_no_ability_spec(&lex_tail(
            "Autumn Willow can be the target of spells and abilities controlled by target player as though it didn't have shroud.",
        ))
        .expect("temporary permission should parse")
        .expect("temporary permission should be claimed");
    assert_eq!(temporary_spec.ignored_ability, StaticAbilityId::Shroud);
    assert!(matches!(
        temporary_spec.sources_controlled_by,
        PlayerFilter::Target(_)
    ));
    assert!(temporary_spec.objects.is_some_and(|filter| filter.source));

    for near_miss in [
        "Creatures your opponents control lose hexproof.",
        "Creatures your opponents control can be the targets of spells and abilities you control.",
    ] {
        assert!(
            parse_targeting_as_though_no_ability_spec(&lex_tail(near_miss))
                .expect("near miss should parse safely")
                .is_none(),
            "claimed near miss: {near_miss}"
        );
    }
}

#[test]
fn only_authored_choose_target_clauses_are_explicit_declarations() {
    let authored = parse_effect_clause(&lex_tail("Choose target opponent."))
        .expect("parse authored target declaration");
    let authored = match authored {
        EffectAst::TagAffected { effect, .. } => *effect,
        effect => effect,
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TargetOnly {
                explicit_declaration: true,
                ..
            },
        ..
    }) = authored
    else {
        panic!("expected explicit target declaration");
    };

    let synthetic = EffectAst::subject_verb_target_only(TargetAst::Player(
        PlayerFilter::target_opponent(),
        None,
    ));
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TargetOnly {
                explicit_declaration: false,
                ..
            },
        ..
    }) = synthetic
    else {
        panic!("expected synthetic target prelude");
    };
}

#[test]
fn explicit_they_control_return_keeps_relative_player_surface() {
    let parsed = parse_effect_clause(&lex_tail(
        "That player returns a land they control to its owner's hand.",
    ))
    .expect("correlated player return should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ReturnToHand { target, .. },
        ..
    }) = parsed
    else {
        panic!("expected a player-owned hand return: {parsed:#?}");
    };
    let target = match target {
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => *inner,
        target => target,
    };
    let TargetAst::Object(filter, _, _) = target else {
        panic!("expected an object return target: {target:#?}");
    };

    assert!(filter.has_iterated_actor_pronoun_surface(), "{filter:#?}");
}

#[test]
fn explicit_opponent_creature_type_choice_keeps_typed_chooser() {
    let parsed = parse_effect_clause(&lex_tail("An opponent chooses a creature type."))
        .expect("parse opponent creature-type choice");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject,
        action:
            SubjectVerbActionAst::ChooseCreatureType {
                excluded_subtypes,
                family: crate::types::SubtypeFamily::Creature,
            },
    }) = parsed
    else {
        panic!("expected typed creature-type choice, got {parsed:#?}");
    };

    assert_eq!(subject.role, SubjectVerbRoleAst::Chooser);
    assert_eq!(subject.player, PlayerAst::Opponent);
    assert!(excluded_subtypes.is_empty());
}

#[test]
fn common_player_action_clause_classifies_core_shapes() {
    let subject = SubjectAst::Player(PlayerAst::TargetOpponent);
    for (verb, tail, expected) in [
        (
            Verb::Draw,
            "X cards where X is their devotion to black",
            CommonPlayerActionPattern::Amount,
        ),
        (
            Verb::Sacrifice,
            "a creature they control",
            CommonPlayerActionPattern::ObjectSelection,
        ),
        (
            Verb::Shuffle,
            "their graveyard into their library",
            CommonPlayerActionPattern::ZoneMovement,
        ),
        (Verb::Pay, "{2}", CommonPlayerActionPattern::Payment),
        (Verb::Scry, "X", CommonPlayerActionPattern::Choice),
    ] {
        let tail = lex_tail(tail);
        let clause = CommonPlayerActionClause::recognize(subject, verb, &tail)
            .expect("common player clause should be recognized");
        assert_eq!(clause.pattern(), expected, "{verb:?} {tail:?}");
    }
}

#[test]
fn turn_target_face_up_keeps_the_authored_target_filter() {
    let parsed = parse_effect_clause(&lex_tail(
        "Turn target face-down creature an opponent controls face up.",
    ))
    .expect("turn-target-face-up clause should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TurnFaceUp {
                target: TargetAst::Object(filter, _, _),
            },
        ..
    }) = parsed
    else {
        panic!("expected a typed turn-face-up action");
    };
    assert_eq!(filter.face_down, Some(true));
    assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
    assert!(filter.card_types.contains(&CardType::Creature));
}

#[test]
fn common_player_action_clause_recognizes_typed_clause_variants() {
    let subject = SubjectAst::Player(PlayerAst::TargetOpponent);
    for (verb, tail, assert_variant) in [
        (
            Verb::Draw,
            "X cards where X is their devotion to black",
            matches_amount as fn(CommonPlayerActionClause<'_>),
        ),
        (
            Verb::Sacrifice,
            "a creature they control",
            matches_object as fn(CommonPlayerActionClause<'_>),
        ),
        (
            Verb::Shuffle,
            "their graveyard into their library",
            matches_zone as fn(CommonPlayerActionClause<'_>),
        ),
        (
            Verb::Scry,
            "X",
            matches_choice as fn(CommonPlayerActionClause<'_>),
        ),
        (
            Verb::Pay,
            "{2}",
            matches_payment as fn(CommonPlayerActionClause<'_>),
        ),
    ] {
        let tail = lex_tail(tail);
        let clause = CommonPlayerActionClause::recognize(subject, verb, &tail)
            .expect("common player clause should be recognized");
        assert_variant(clause);
    }
}

fn matches_amount(clause: CommonPlayerActionClause<'_>) {
    assert!(matches!(clause, CommonPlayerActionClause::Amount(_)));
}

fn matches_object(clause: CommonPlayerActionClause<'_>) {
    assert!(matches!(clause, CommonPlayerActionClause::Object(_)));
}

fn matches_zone(clause: CommonPlayerActionClause<'_>) {
    assert!(matches!(clause, CommonPlayerActionClause::Zone(_)));
}

fn matches_choice(clause: CommonPlayerActionClause<'_>) {
    assert!(matches!(clause, CommonPlayerActionClause::Choice(_)));
}

fn matches_payment(clause: CommonPlayerActionClause<'_>) {
    assert!(matches!(clause, CommonPlayerActionClause::Payment(_)));
}

#[test]
fn common_player_action_clause_delegates_to_effect_parser() {
    for text in [
        "Target opponent draws a card",
        "Target opponent sacrifices a creature they control",
        "Target opponent shuffles their library",
        "Target opponent pays {2}",
        "Each opponent scries 1",
    ] {
        let tokens = lex_line(text, 0).expect("lex clause");
        parse_effect_clause(&tokens)
            .unwrap_or_else(|err| panic!("common player clause should parse: {text}: {err:?}"));
    }
}

#[test]
fn any_player_sacrifice_offer_keeps_sequential_player_semantics() {
    let tokens = lex_line("Any player may sacrifice two creatures of their choice.", 0)
        .expect("lex any-player sacrifice offer");
    let effect = parse_effect_clause(&tokens).expect("parse any-player sacrifice offer");

    let EffectAst::AnyPlayerMay { players, effects } = effect else {
        panic!("expected typed any-player offer, got {effect:#?}");
    };
    assert_eq!(players, PlayerFilter::Any);
    let [
        EffectAst::Sequence {
            effects: sacrifice_steps,
        },
    ] = effects.as_slice()
    else {
        panic!("expected a choose-and-sacrifice sequence, got {effects:#?}");
    };
    assert!(
        matches!(
            sacrifice_steps.as_slice(),
            [
                EffectAst::ChooseObjects {
                    player: PlayerAst::That,
                    ..
                },
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: crate::model::ast::SubjectVerbSubjectAst {
                        player: PlayerAst::That,
                        ..
                    },
                    action: SubjectVerbActionAst::SacrificeAll { .. },
                })
            ]
        ),
        "expected both choice and sacrifice to stay bound to the offered player, got {sacrifice_steps:#?}"
    );
}

#[test]
fn any_opponent_sacrifice_offer_keeps_filtered_sequential_semantics() {
    let tokens = lex_line("Any opponent may sacrifice a creature of their choice.", 0)
        .expect("lex any-opponent sacrifice offer");
    let effect = parse_effect_clause(&tokens).expect("parse any-opponent sacrifice offer");

    let EffectAst::AnyPlayerMay { players, effects } = effect else {
        panic!("expected typed filtered offer, got {effect:#?}");
    };
    assert_eq!(players, PlayerFilter::Opponent);
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: crate::model::ast::SubjectVerbSubjectAst {
                player: PlayerAst::That,
                ..
            },
            action: SubjectVerbActionAst::Sacrifice { count: 1, .. },
        })]
    ));
}

#[test]
fn any_player_half_life_payment_keeps_sequential_payer_relative_semantics() {
    let tokens = lex_line("Any player may pay half their life, rounded up.", 0)
        .expect("lex any-player half-life offer");
    let effect = parse_effect_clause(&tokens).expect("parse any-player half-life offer");

    let EffectAst::AnyPlayerMay { players, effects } = effect else {
        panic!("expected typed any-player offer, got {effect:#?}");
    };
    assert_eq!(players, PlayerFilter::Any);
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: crate::model::ast::SubjectVerbSubjectAst {
                player: PlayerAst::That,
                ..
            },
            action: SubjectVerbActionAst::PayLife {
                amount: Value::HalfLifeTotalRoundedUp(PlayerFilter::IteratedPlayer),
            },
        })]
    ));
}

#[test]
fn explicit_player_attach_clause_preserves_the_attachment_chooser() {
    let tokens = lex_line(
        "That player attaches this Aura to a land of their choice.",
        0,
    )
    .expect("lex explicit-player attach clause");
    let effect = parse_effect_clause(&tokens).expect("parse explicit-player attach clause");

    assert!(
        matches!(
            &effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: crate::model::ast::SubjectVerbSubjectAst {
                    player: PlayerAst::That,
                    ..
                },
                action: SubjectVerbActionAst::Attach {
                    target: TargetAst::WithCount(_, count),
                    ..
                },
            }) if count.is_single()
        ),
        "explicit attach actor and counted destination must survive parsing: {effect:#?}"
    );
}

#[test]
fn optional_target_player_subject_declares_and_reuses_the_player_target() {
    let tokens = lex_line(
        "Up to one target player mills cards equal to this creature's power.",
        0,
    )
    .expect("lex optional target-player mill clause");
    let subject_shape = clause_grammar::parse_clause_subject_verb_shape(&tokens)
        .expect("split optional target-player subject from mill verb");
    assert_eq!(
        crate::lexer::parser_token_word_refs(subject_shape.subject_tokens),
        ["up", "to", "one", "target", "player"]
    );
    let effect = parse_effect_clause(&tokens).expect("parse optional target-player mill clause");
    let EffectAst::Sequence { effects } = effect else {
        panic!("expected target declaration followed by mill: {effect:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TargetOnly { target, .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                crate::model::ast::SubjectVerbSubjectAst {
                    player: PlayerAst::That,
                    ..
                },
            action: SubjectVerbActionAst::Mill { .. },
        }),
    ] = effects.as_slice()
    else {
        panic!("expected correlated optional player target and mill: {effects:#?}");
    };
    assert!(matches!(
        target,
        TargetAst::WithCount(inner, count)
            if matches!(inner.as_ref(), TargetAst::Player(_, _))
                && count.min == 0
                && count.max == Some(1)
    ));
}

#[test]
fn targeted_graveyard_cast_permission_preserves_one_target_and_duration() {
    use crate::types::{CardType, Subtype};

    let cases = [
        (
            "You may cast target nonland card from your graveyard this turn.",
            vec![],
            vec![],
            true,
        ),
        (
            "You may cast target artifact card from your graveyard this turn.",
            vec![CardType::Artifact],
            vec![],
            false,
        ),
        (
            "You may cast target enchantment card from your graveyard this turn.",
            vec![CardType::Enchantment],
            vec![],
            false,
        ),
        (
            "You may cast target Zombie creature card from your graveyard this turn.",
            vec![CardType::Creature],
            vec![Subtype::Zombie],
            false,
        ),
    ];

    for (text, card_types, subtypes, excludes_land) in cases {
        let tokens = lex_line(text, 0).expect("lex targeted graveyard permission");
        let effect = parse_effect_clause(&tokens).expect("targeted graveyard permission");
        let EffectAst::Sequence { effects } = effect else {
            panic!("expected target-plus-grant sequence for {text}");
        };
        let [
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::TargetOnly { target, .. },
                ..
            }),
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                        tag,
                        player,
                        allow_land,
                        without_paying_mana_cost,
                        allow_any_color_for_cast,
                        while_on_top_of_library,
                        free_cast_from_current_zone: _,
                        surface: _,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one target followed by one tagged cast grant for {text}");
        };
        let TargetAst::Object(filter, _, _) = target else {
            panic!("expected a single object target for {text}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard), "{text}");
        assert_eq!(filter.owner, Some(PlayerFilter::You), "{text}");
        assert_eq!(filter.card_types, card_types, "{text}");
        assert_eq!(filter.subtypes, subtypes, "{text}");
        assert_eq!(
            filter.excluded_card_types.contains(&CardType::Land),
            excludes_land,
            "{text}"
        );
        assert_eq!(
            tag.as_str(),
            crate::tag::CompilerReferenceTag::It.as_str(),
            "{text}"
        );
        assert_eq!(*player, PlayerAst::You, "{text}");
        assert!(!*allow_land, "{text}");
        assert!(!*without_paying_mana_cost, "{text}");
        assert_eq!(
            *allow_any_color_for_cast,
            ironsmith_core::value_model::ManaSpendMode::Normal,
            "{text}"
        );
        assert!(!*while_on_top_of_library, "{text}");
    }
}

#[test]
fn leading_may_chain_reaches_targeted_graveyard_cast_permission() {
    let tokens = lex_line(
        "You may cast target Zombie creature card from your graveyard this turn.",
        0,
    )
    .expect("lex leading-may targeted graveyard permission");
    let effects = crate::effect_sentences::parse_effect_chain_lexed(&tokens)
        .expect("parse through production leading-may chain");

    let [EffectAst::Sequence { effects }] = effects.as_slice() else {
        panic!("expected the chain to preserve a target-plus-grant sequence: {effects:#?}");
    };
    assert!(matches!(
        effects.as_slice(),
        [
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::TargetOnly { .. },
                ..
            }),
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { .. },
                ..
            }),
        ]
    ));
}

#[test]
fn plural_demonstrative_pump_preserves_tagged_set() {
    let tokens =
        lex_line("Those creatures get +1/+1 until end of turn.", 0).expect("lex plural pump");
    let effect = parse_effect_clause(&tokens).expect("plural tagged pump should parse");
    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Pump {
                target: TargetAst::Object(filter, ..),
                set_quantifier_surface,
                ..
            },
        ..
    }) = effect
    else {
        panic!("expected a mass pump for a plural demonstrative subject");
    };
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert_eq!(
        set_quantifier_surface,
        Some(ironsmith_core::SetQuantifierSurface::Those)
    );
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
    }));
}

#[test]
fn plural_demonstrative_untap_preserves_typed_tagged_set() {
    let tokens = lex_line("Untap those creatures.", 0).expect("lex plural untap");
    let effect = parse_effect_clause(&tokens).expect("plural tagged untap should parse");

    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        action: SubjectVerbActionAst::UntapAll { filter },
        ..
    }) = effect
    else {
        panic!("expected a mass untap for a plural demonstrative subject");
    };
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
    }));
}

#[test]
fn each_other_player_subject_lowers_to_filtered_player_iteration() {
    let tokens = lex_line("Each other player loses X life.", 0).expect("lex clause");
    let effect = parse_effect_clause(&tokens).expect("each-other-player clause should parse");

    let EffectAst::ForEachPlayersFiltered { filter, effects } = effect else {
        panic!("expected filtered player iteration");
    };
    assert_eq!(filter, PlayerFilter::NotYou);
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                subject: crate::cards::builders::SubjectVerbSubjectAst {
                    player: PlayerAst::That,
                    ..
                },
                action: SubjectVerbActionAst::LoseLife { .. },
            }
        )]
    ));
}

#[test]
fn parses_control_target_player_during_next_turn_clause() {
    let tokens = lex_line(
        "You control target player during that player's next turn.",
        0,
    )
    .expect("lex clause");
    let effect =
        parse_effect_clause(&tokens).expect("control target player during next turn should parse");
    let debug = format!("{effect:?}").to_ascii_lowercase();
    assert!(
        debug.contains("controlplayer") && debug.contains("nextturn"),
        "expected control-player-next-turn effect, got {debug}"
    );
}

#[test]
fn counter_linked_land_subtype_followup_lowers_to_prior_tagged_land() {
    let tokens = lex_line(
            "That land is an Island in addition to its other types for as long as it has a flood counter on it.",
            0,
        )
        .unwrap();
    let effect = parse_effect_clause(&tokens).expect("typed land subtype followup");
    let debug = format!("{effect:#?}");
    assert!(debug.contains("AddSubtypes"), "{debug}");
    assert!(debug.contains("Island"), "{debug}");
    assert!(
        debug.contains(crate::tag::CompilerReferenceTag::It.as_str()),
        "{debug}"
    );
    assert!(
        debug.contains("ForAsLongAs") && debug.contains("Flood"),
        "{debug}"
    );
}

#[test]
fn explicit_target_damage_subject_owns_its_characteristic_and_controller() {
    let tokens = lex_line(
        "target enchantment deals damage equal to its mana value to its controller",
        0,
    )
    .expect("lex explicit damage-source clause");
    let effect = parse_effect_clause(&tokens).expect("explicit damage-source clause should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::DealDamageEqualToPower {
                source,
                amount,
                target,
                ..
            },
        ..
    }) = effect
    else {
        panic!("expected typed explicit damage source, got {effect:#?}");
    };
    assert!(matches!(
        source,
        TargetAst::Object(ref filter, _, _)
            if filter.card_types == [crate::types::CardType::Enchantment]
    ));
    assert!(matches!(
        amount.unhinted(),
        Value::ManaValueOf(spec)
            if matches!(spec.base(), crate::target::ChooseSpec::Source)
    ));
    let controller_reference = match &target {
        TargetAst::Player(PlayerFilter::ControllerOf(reference), _) => reference,
        _ => panic!("damage should go to the source controller: {target:#?}"),
    };
    assert!(
        matches!(controller_reference, crate::target::ObjectRef::Target)
            || matches!(
                controller_reference,
                crate::target::ObjectRef::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            ),
        "controller provenance should remain target-relative: {target:#?}"
    );
}

#[test]
fn filtered_combat_damage_prevention_keeps_non_subtype_source_filter() {
    let tokens = lex_line(
        "Prevent all combat damage non-Soldier creatures would deal this turn.",
        0,
    )
    .unwrap();
    effect_grammar::parse_prevent_damage_sentence_lexed(&tokens)
        .expect("typed prevention grammar should not error")
        .expect("typed prevention grammar should recognize filtered source");
    let effect = parse_effect_clause(&tokens).expect("typed filtered prevention");
    let debug = format!("{effect:#?}");
    assert!(debug.contains("PreventAllCombatDamage"), "{debug}");
    assert!(debug.contains("Soldier"), "{debug}");
    assert!(debug.contains("excluded_subtypes"), "{debug}");
}

#[test]
fn discarded_this_way_pump_split_keeps_typed_modifier_tail() {
    let tokens = lex_line(
        "target creature gets +2/+0 until end of turn for each card discarded this way",
        0,
    )
    .unwrap();
    let shape = clause_grammar::parse_clause_subject_verb_shape(&tokens).unwrap();
    assert!(
        clause_grammar::parse_discarded_this_way_modifier_shape(shape.action_tokens).is_some(),
        "{:?}",
        shape.action_tokens
    );
}

#[test]
fn authored_duration_before_for_each_is_retained_on_the_count() {
    let tokens = lex_line(
            "target creature gets -1/-1 until end of turn for each modified creature you controlled as you cast this spell",
            0,
        )
        .expect("dynamic modifier should lex");
    let shape = clause_grammar::parse_clause_subject_verb_shape(&tokens)
        .expect("dynamic modifier should have a subject/verb shape");
    let effect = parse_get_pump_clause(shape.subject_tokens, shape.action_tokens, &tokens)
        .expect("dynamic modifier should not error")
        .expect("dynamic modifier should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::PumpForEach { count, .. },
        ..
    }) = effect
    else {
        panic!("expected a typed per-object modifier, got {effect:#?}");
    };

    assert!(count.has_surface_hint(ValueSurfaceHint::DurationBeforeForEach));
    assert!(matches!(
        count.unhinted(),
        Value::Count(filter)
            if matches!(
                filter.tagged_constraints.as_slice(),
                [constraint]
                    if constraint.tag.as_str()
                        == crate::tag::CompilerReferenceTag::CastModifiedCreatures.as_str()
            )
    ));
}

#[test]
fn tagged_plural_pump_clause_lowers_directly() {
    let tokens = lex_line("they each get +2/+2 until end of turn", 0).unwrap();
    let shape = clause_grammar::parse_clause_subject_verb_shape(&tokens).unwrap();
    assert_eq!(
        ClauseDispatchCompatWords::new(shape.subject_tokens).word_refs(),
        ["they", "each"]
    );
    let effect = parse_get_pump_clause(shape.subject_tokens, shape.action_tokens, &tokens)
        .expect("tagged plural pump should not error")
        .expect("tagged plural pump should be recognized");
    assert!(
        matches!(effect, EffectAst::SubjectVerb(_)),
        "expected typed subject-verb pump, got {effect:?}"
    );
}

#[test]
fn target_restriction_preserves_leading_duration_surface() {
    let leading = parse_effect_clause(
        &lex_line(
            "Until end of turn, target creature can't be blocked by Walls.",
            0,
        )
        .unwrap(),
    )
    .expect("leading-duration restriction should parse");
    let trailing = parse_effect_clause(
        &lex_line("Target creature can't be blocked by Walls this turn.", 0).unwrap(),
    )
    .expect("trailing-duration restriction should parse");

    let duration_surface = |effect: &EffectAst| {
        let EffectAst::Sequence { effects } = effect else {
            panic!("expected target declaration plus restriction: {effect:#?}");
        };
        let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    duration: Until::EndOfTurn,
                    duration_surface,
                    ..
                },
            ..
        })) = effects.get(1)
        else {
            panic!("expected end-of-turn restriction: {effect:#?}");
        };
        *duration_surface
    };

    assert_eq!(
        duration_surface(&leading),
        crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
    );
    assert_eq!(
        duration_surface(&trailing),
        crate::effect::RestrictionDurationSurface::Default
    );
}

#[test]
fn face_down_return_if_permanent_then_turn_stays_a_resolution_condition() {
    let tokens = lex_line(
            "return it to the battlefield face down under its owner's control if it's a permanent card, then turn it face up",
            0,
        )
        .unwrap();
    let effect = parse_effect_clause(&tokens).expect("typed conditional return-turn clause");
    let EffectAst::TrailingIf { predicate, effects } = effect else {
        panic!("expected non-promotable trailing condition, got {effect:#?}");
    };
    assert_eq!(effects.len(), 2);
    assert!(matches!(
        &effects[0],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::MoveToZone {
                zone: Zone::Battlefield,
                battlefield_face_down: true,
                ..
            },
            ..
        })
    ));
    assert!(matches!(
        &effects[1],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TurnFaceUp { .. },
            ..
        })
    ));
    assert!(
        !format!("{predicate:#?}").contains("face_down: Some(false)"),
        "the follow-up words must not leak into the permanent-card predicate"
    );
}
