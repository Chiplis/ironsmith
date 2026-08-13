use super::super::super::lexer::{TokenWordView, lex_line};
use super::*;

#[test]
fn negation_span_and_or_split_are_typed() {
    let tokens = lex_line("creatures can't attack or activate abilities", 0).unwrap();
    let negation = parse_activation_negation_span_tokens(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(&tokens[negation.first..negation.end]).word_refs(),
        ["cant"]
    );
    let split = parse_cant_restriction_or_split_tokens(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(&split.first).word_refs(),
        ["creatures", "cant", "attack"]
    );
    assert_eq!(
        TokenWordView::new(&split.second).word_refs(),
        ["creatures", "cant", "activate", "abilities"]
    );
}

#[test]
fn attack_or_block_is_one_restriction_tail() {
    let tokens = lex_line("this creature can't attack or block", 0).unwrap();
    assert!(parse_cant_restriction_or_split_tokens(&tokens).is_none());
}

#[test]
fn unspent_mana_retention_surface_is_typed() {
    assert_eq!(
        parse_unspent_mana_retention_tail_words(&[
            "lose", "unspent", "red", "mana", "as", "steps", "and", "phases", "end",
        ]),
        Some(UnspentManaRetentionTail {
            color: Some(Color::Red),
        })
    );
    assert_eq!(
        parse_unspent_mana_retention_static_words(&[
            "each", "player", "dont", "lose", "unspent", "mana", "as", "steps",
        ]),
        Some(UnspentManaRetentionStatic {
            subject: ManaRetentionSubject::AnyPlayer,
            color: None,
        })
    );
}

#[test]
fn cast_qualifier_possessive_and_condition_envelopes_are_typed() {
    let qualifier =
        parse_activation_cast_limit_qualifier_words(&["noncreature", "spells"]).unwrap();
    assert_eq!(qualifier.consumed, 1);
    assert!(
        qualifier
            .filter
            .excluded_card_types
            .contains(&crate::types::CardType::Creature)
    );

    let possessive = lex_line("artifacts'", 0).unwrap();
    assert_eq!(
        TokenWordView::new(&parse_activation_possessive_owner_tokens(&possessive)).word_refs(),
        ["artifact"]
    );

    let prefixed = lex_line("during your turn, creatures can't block", 0).unwrap();
    assert!(matches!(
        parse_static_restriction_condition_shape_tokens(&prefixed),
        Some(StaticRestrictionConditionShape::Timing {
            timing: ActivationTiming::DuringYourTurn,
            ..
        })
    ));
    let conditional = lex_line("if you control a creature, players can't gain life", 0).unwrap();
    assert!(matches!(
        parse_static_restriction_condition_shape_tokens(&conditional),
        Some(StaticRestrictionConditionShape::Condition {
            kind: StaticRestrictionConditionKind::If,
            ..
        })
    ));

    let extra_turn_suffix = lex_line("this can't attack during extra turns", 0).unwrap();
    let Some(StaticRestrictionConditionShape::ExtraTurn {
        remainder_first,
        remainder_end,
    }) = parse_static_restriction_condition_shape_tokens(&extra_turn_suffix)
    else {
        panic!("extra-turn suffix should be a typed static condition");
    };
    assert_eq!(
        TokenWordView::new(&extra_turn_suffix[remainder_first..remainder_end]).word_refs(),
        ["this", "cant", "attack"]
    );
}

#[test]
fn global_and_player_restriction_surfaces_return_typed_facts() {
    assert_eq!(
        parse_global_cant_restriction_words(&[
            "your",
            "opponents",
            "cant",
            "block",
            "with",
            "creatures",
            "with",
            "odd",
            "mana",
            "values",
        ]),
        Some(GlobalCantRestrictionFact::OpponentsBlockManaValueParity(
            crate::filter::ParityRequirement::Odd,
        ))
    );
    assert_eq!(
        parse_player_restriction_subject_words(&["players", "dealt", "damage", "this", "way"]),
        Some(crate::target::PlayerFilter::TaggedPlayer(
            crate::cards::builders::TagKey::from("damaged_0"),
        ))
    );
    assert_eq!(
        parse_player_restriction_tail_words(&["draw", "more", "than", "one", "card"]),
        Some(PlayerRestrictionTailKind::DrawExtraCards)
    );
    assert!(
        parse_or_win_game_tail_words(&["lose", "the", "game", "or", "win", "the", "game",])
            .is_some()
    );
}

#[test]
fn cast_restriction_grammar_owns_filters_and_typed_numbers() {
    let fact = parse_cant_cast_restriction_fact_words(&[
        "players",
        "cant",
        "cast",
        "noncreature",
        "spells",
    ])
    .unwrap();
    let CantCastRestrictionFact::CastSpellsMatching { player, filter } = fact else {
        panic!("expected matching-spell restriction fact");
    };
    assert_eq!(player, crate::target::PlayerFilter::Any);
    assert!(
        filter
            .excluded_card_types
            .contains(&crate::types::CardType::Creature)
    );

    let filter = parse_spell_restriction_subject_filter_words(&[
        "creature", "spells", "with", "mana", "value", "three", "or", "less",
    ])
    .unwrap();
    assert_eq!(
        filter.mana_value,
        Some(crate::filter::Comparison::LessThanOrEqual(3))
    );
    assert!(
        parse_spell_restriction_subject_filter_words(&[
            "spells",
            "with",
            "the",
            "chosen",
            "name",
            "unexpected",
        ])
        .is_none()
    );

    assert!(matches!(
        parse_player_activation_restriction_tail_words(&[
            "activate",
            "abilities",
            "of",
            "artifacts",
            "unless",
            "theyre",
            "mana",
            "abilities",
        ]),
        Some(PlayerActivationRestrictionTailFact::ActivateAbilitiesOf {
            non_mana_only: true,
            ..
        })
    ));
}

#[test]
fn object_restriction_envelopes_preserve_typed_boundaries() {
    assert_eq!(
        parse_simple_object_restriction_words(&["attack", "or", "block", "this", "turn"]),
        Some(SimpleObjectRestrictionKind::AttackOrBlock)
    );
    assert_eq!(
        parse_simple_object_restriction_words(&["phase", "in"]),
        Some(SimpleObjectRestrictionKind::PhaseIn)
    );
    assert_eq!(
        parse_negated_object_tail_words(&["be", "blocked", "except", "by", "Walls"]),
        Some(NegatedObjectTailShape::BeBlockedExceptBy { payload_words: 4 })
    );

    let tokens = lex_line(
        "be the target of blue spells or abilities from red sources",
        0,
    )
    .unwrap();
    let TargetRestrictionEnvelope::FilteredSources {
        spell_descriptor_tokens,
        source_descriptor_tokens,
    } = parse_target_restriction_envelope_tokens(&tokens).unwrap()
    else {
        panic!("expected filtered-source envelope");
    };
    assert_eq!(
        TokenWordView::new(&tokens[spell_descriptor_tokens.unwrap()]).word_refs(),
        ["blue"]
    );
    assert_eq!(
        TokenWordView::new(&tokens[source_descriptor_tokens]).word_refs(),
        ["red"]
    );
}

#[test]
fn target_and_activated_owner_prefixes_are_typed() {
    for text in [
        "target creature",
        "up to two target creatures",
        "up to one other target creature",
        "one other target creature",
        "one or two target creatures",
        "on another target creature",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(parse_target_indicator_tokens(&tokens).is_some(), "{text}");
    }

    let tokens = lex_line("activated abilities with t in their costs of artifacts", 0).unwrap();
    let shape = parse_activated_ability_owner_shape_tokens(&tokens).unwrap();
    assert_eq!(shape.scope, ActivatedAbilityOwnerScope::TapCostOnly);
    assert_eq!(
        TokenWordView::new(&tokens[shape.owner_tokens]).word_refs(),
        ["artifacts"]
    );

    let possessive = lex_line("their activated abilities cant be activated", 0).unwrap();
    assert!(parse_possessive_activated_ability_subject_tokens(&possessive).is_some());
}

#[test]
fn mana_retention_and_subject_markers_are_typed() {
    assert_eq!(
        parse_mana_retention_negated_clause_words(&[
            "you", "dont", "lose", "this", "mana", "as", "steps",
        ]),
        Some(ManaRetentionNegatedClause {
            tail: ManaRetentionTailKind::ThisMana,
        })
    );
    assert_eq!(
        parse_restriction_subject_surface_words(&["that", "damage"]),
        Some(RestrictionSubjectSurface::Damage)
    );
    assert_eq!(
        parse_restriction_subject_surface_words(&["this"]),
        Some(RestrictionSubjectSurface::Source)
    );
    assert!(
        parse_dealt_damage_this_way_words(&["creatures", "dealt", "damage", "this", "way",])
            .is_some()
    );
}
