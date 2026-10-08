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
fn negation_span_ignores_quoted_granted_rules() {
    let quoted_only = lex_line(
        "It gains \"This creature can't be blocked.\" until end of turn.",
        0,
    )
    .unwrap();
    assert_eq!(parse_activation_negation_span_tokens(&quoted_only), None);

    let outer_restriction = lex_line(
        "It gains \"This creature can't be blocked,\" and it can't attack.",
        0,
    )
    .unwrap();
    let negation = parse_activation_negation_span_tokens(&outer_restriction).unwrap();
    assert_eq!(
        TokenWordView::new(&outer_restriction[negation.first..negation.end]).word_refs(),
        ["cant"]
    );
    assert!(
        outer_restriction[..negation.first]
            .iter()
            .any(|token| token.is_quote())
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
            crate::tag::CompilerReferenceTag::Damaged0.bind().into(),
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

#[test]
fn cast_restriction_retains_dynamic_mana_value_comparison() {
    let words = [
        "each",
        "opponent",
        "cant",
        "cast",
        "noncreature",
        "spells",
        "with",
        "mana",
        "value",
        "greater",
        "than",
        "the",
        "number",
        "of",
        "lands",
        "that",
        "player",
        "controls",
    ];
    let fact = parse_cant_cast_restriction_fact_words(&words).expect("dynamic restriction");
    let CantCastRestrictionFact::CastSpellsMatching { player, filter } = fact else {
        panic!("{fact:?}");
    };
    assert_eq!(player, crate::target::PlayerFilter::Opponent);
    assert_eq!(
        filter.excluded_card_types,
        vec![crate::types::CardType::Creature]
    );
    assert!(
        matches!(
            filter.mana_value,
            Some(crate::filter::Comparison::GreaterThanExpr(_))
        ),
        "{filter:?}"
    );
    let mut extra = words.to_vec();
    extra.push("nonsense");
    assert!(parse_cant_cast_restriction_fact_words(&extra).is_none());
}

#[test]
fn targeting_source_envelopes_keep_single_kinds_and_controller_qualified_pairs() {
    for text in ["be the target of spells", "be the targets of blue or black spells your opponents control"] {
        assert!(matches!(parse_target_restriction_envelope_tokens(&lex_line(text, 0).unwrap()), Some(TargetRestrictionEnvelope::SourceSpell { .. })), "{text}");
    }
    assert!(matches!(parse_target_restriction_envelope_tokens(&lex_line("be the target of abilities your opponents control", 0).unwrap()), Some(TargetRestrictionEnvelope::SourceAbility { .. })));
    assert!(matches!(parse_target_restriction_envelope_tokens(&lex_line("be the targets of spells or abilities", 0).unwrap()), Some(TargetRestrictionEnvelope::SpellsOrAbilities)));
    assert!(matches!(parse_target_restriction_envelope_tokens(&lex_line("be the target of nongreen spells your opponents control or abilities from nongreen sources your opponents control", 0).unwrap()), Some(TargetRestrictionEnvelope::PairedControlledSources { .. })));
    for text in ["be the target of spells unless it attacked", "be the target of abilities your opponents control this turn and draw a card"] {
        assert!(parse_target_restriction_envelope_tokens(&lex_line(text, 0).unwrap()).is_none(), "{text}");
    }
}

#[test]
fn source_exiled_names_are_typed_cast_restrictions_with_full_consumption() {
    for source in [vec!["this"], vec!["this", "permanent"], vec!["this", "creature"]] {
        let mut words = vec!["cast", "spells", "with", "the", "same", "name", "as", "a", "card", "exiled", "with"];
        words.extend(source);
        let filter = parse_cast_restriction_tail_filter_words(&words).unwrap();
        assert!(filter.zone.is_none());
        assert!(filter.tagged_constraints.iter().any(|constraint|
            constraint.tag.as_str() == ironsmith_core::SOURCE_EXILED_TAG
                && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged));
        words.push("unexpected");
        assert!(parse_cast_restriction_tail_filter_words(&words).is_none());
    }
}

#[test]
fn land_and_spell_origins_are_distinct_complete_action_facts() {
    let facts = parse_compound_player_action_restriction_words(&["play", "lands", "or", "cast", "spells", "from", "your", "hand"]).unwrap();
    let [PlayerActivationRestrictionTailFact::PlayLandsMatching(lands), PlayerActivationRestrictionTailFact::CastSpellsMatching(spells)] = facts.as_slice() else { panic!("separate actions"); };
    assert_eq!(lands.zone, Some(crate::zone::Zone::Hand));
    assert_eq!(spells.zone, Some(crate::zone::Zone::Hand));
    assert_eq!(lands.owner, Some(crate::target::PlayerFilter::You));
    assert_eq!(spells.owner, Some(crate::target::PlayerFilter::You));
    assert!(lands.card_types.contains(&crate::types::CardType::Land));
    assert!(parse_compound_player_action_restriction_words(&["play", "lands", "or", "cast", "spells", "from", "your", "hand", "unless", "you", "pay", "2"]).is_none());
}

#[test]
fn graveyard_activation_and_casting_remain_two_restrictions() {
    let facts = parse_compound_player_action_restriction_words(&["cast", "spells", "from", "graveyards", "or", "activate", "abilities", "of", "cards", "in", "graveyards"]).unwrap();
    let [PlayerActivationRestrictionTailFact::CastSpellsMatching(spells), PlayerActivationRestrictionTailFact::ActivateAbilitiesOf { filter: abilities, non_mana_only: false }] = facts.as_slice() else { panic!("both actions including mana abilities"); };
    assert_eq!(spells.zone, Some(crate::zone::Zone::Graveyard));
    assert_eq!(abilities.zone, Some(crate::zone::Zone::Graveyard));
    assert!(parse_compound_player_action_restriction_words(&["cast", "spells", "from", "graveyards", "or", "activate", "mystery", "abilities"]).is_none());
}

#[test]
fn spell_color_type_and_linked_names_keep_executable_filters() {
    let blue_creature = parse_cast_restriction_tail_filter_words(&["cast", "blue", "creature", "spells"]).unwrap();
    assert_eq!(blue_creature.colors, Some(crate::color::ColorSet::BLUE));
    assert_eq!(blue_creature.card_types, vec![crate::types::CardType::Creature]);
    let chosen = parse_cast_restriction_tail_filter_words(&["cast", "spells", "of", "the", "chosen", "color"]).unwrap();
    assert!(chosen.chosen_color);
    let linked = parse_cast_restriction_tail_filter_words(&["cast", "spells", "with", "the", "same", "name", "as", "the", "exiled", "card"]).unwrap();
    assert!(linked.tagged_constraints.iter().any(|tag| tag.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()));
    let land = parse_land_play_restriction_tail_words(&["play", "nonbasic", "lands", "with", "the", "same", "name", "as", "a", "nontoken", "permanent"]).unwrap();
    assert!(land.excluded_supertypes.contains(&crate::types::Supertype::Basic));
    assert!(land.characteristic_relations[0].comparison.nontoken);
    assert_eq!(land.characteristic_relations[0].comparison.zone, Some(crate::zone::Zone::Battlefield));
}

#[test]
fn loyalty_prohibition_does_not_become_all_activated_abilities() {
    assert!(matches!(parse_player_activation_restriction_tail_words(&["activate", "planeswalkers", "loyalty", "abilities"]), Some(PlayerActivationRestrictionTailFact::ActivateLoyaltyAbilitiesOf(_))));
    assert!(parse_player_activation_restriction_tail_words(&["activate", "planeswalkers", "loyalty", "abilities", "unless", "theyre", "mana", "abilities"]).is_none());
}

#[test]
fn ability_only_source_restriction_retains_its_distinct_envelope() {
    let tokens = lex_line("be the target of abilities from artifact sources", 0).unwrap();
    let TargetRestrictionEnvelope::AbilitiesFrom { source_descriptor_tokens } =
        parse_target_restriction_envelope_tokens(&tokens).unwrap() else { panic!() };
    assert_eq!(crate::lexer::token_word_refs(&tokens[source_descriptor_tokens]), vec!["artifact"]);
    for text in [
        "be the target of abilities from artifact sources and draw a card",
        "be the target of abilities from sources",
        "be the target of abilities from artifact sources this turn",
    ] {
        assert!(parse_target_restriction_envelope_tokens(&lex_line(text, 0).unwrap()).is_none(), "{text}");
    }
}

// Source-only HOLD evidence; UNRUN. This does not bless the current mistaken
// Restriction classification of the relative predicate as correct semantics.
#[test]
fn sinister_concierge_unowned_gain_suspend_clause_stays_fail_closed() {
    let tokens = lex_line(
        "Each card exiled this way that doesn't have suspend gains suspend.",
        0,
    )
    .unwrap();
    assert!(crate::effect_sentences::parse_effect_clause_lexed(&tokens).is_err());
}

#[test]
fn sinister_concierge_hold_does_not_relax_genuine_main_negated_heads() {
    use crate::grammar::effects::typed_clause_heads::{
        ClauseHeadFormAst, classify_typed_clause_head,
    };
    use crate::recognition::ParseOutcome;

    for text in [
        "Those creatures don't untap during their controllers' next untap steps.",
        "That card does not have suspend.",
        "Each card exiled this way can't be cast.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let ParseOutcome::Match(head) = classify_typed_clause_head(&tokens) else {
            panic!("missing restriction head: {text}");
        };
        assert_eq!(head.value.form, ClauseHeadFormAst::Restriction, "{text}");
        assert!(!head.value.permits_action_fallback(), "{text}");
    }
}
