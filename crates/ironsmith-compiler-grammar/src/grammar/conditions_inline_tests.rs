use super::*;
use crate::cards::builders::PlayerAst;

#[test]
fn parses_removed_from_draft_characteristic_condition_with_exact_group_name() {
    let tokens = lex_line(
        "you removed a creature card with flying from the draft with cards named Draft Mimic",
        0,
    )
    .expect("lex removed-from-draft condition");
    let parsed =
        parse_removed_from_draft_condition(&tokens).expect("typed removed-from-draft condition");

    assert_eq!(parsed.player, PlayerAst::You);
    assert!(parsed.filter.card_types.contains(&CardType::Creature));
    assert!(parsed.filter.has_explicit_card_noun());
    assert_eq!(
        parsed.filter.static_abilities,
        vec![crate::static_abilities::StaticAbilityId::Flying]
    );
    assert_eq!(parsed.with_cards_named, "Draft Mimic");

    let lowercase_source_name = lex_line(
            "you removed a creature card with flying from the draft with cards named animus of predation",
            0,
        )
        .expect("lex lowercase removed-from-draft condition");
    let lowercase_source_name = parse_removed_from_draft_condition(&lowercase_source_name)
        .expect("lowercase named group must remain characteristic data");
    assert_eq!(lowercase_source_name.filter.zone, None);
    assert!(
        lowercase_source_name
            .with_cards_named
            .eq_ignore_ascii_case("animus of predation"),
        "the named draft group may restore the current source's authored casing: {lowercase_source_name:#?}"
    );

    let near_miss = lex_line(
        "you removed a creature card with flying from your graveyard with cards named Draft Mimic",
        0,
    )
    .expect("lex near miss");
    assert!(parse_removed_from_draft_condition(&near_miss).is_none());
}
use crate::lexer::lex_line;

#[test]
fn life_change_condition_accepts_any_player_subject() {
    let tokens = lex_line("a player lost 4 or more life this turn", 0).expect("lex");
    assert_eq!(
        parse_player_life_change_this_turn_condition(&tokens),
        Some(PlayerLifeChangeThisTurnConditionAst {
            player: PlayerFilter::Any,
            direction: PlayerLifeChangeDirectionAst::Lost,
            comparison: Comparison::GreaterThanOrEqual(4),
        })
    );
}

#[test]
fn parse_subject_status_condition_uses_shared_capture_shape() {
    let cases = [
        (
            "this creature is untapped",
            SubjectStatusConditionAst {
                subject: StatusConditionSubjectAst::Source,
                state: StatusConditionStateAst::Untapped,
            },
        ),
        (
            "this tapped",
            SubjectStatusConditionAst {
                subject: StatusConditionSubjectAst::Source,
                state: StatusConditionStateAst::Tapped,
            },
        ),
        (
            "equipped creature attacking",
            SubjectStatusConditionAst {
                subject: StatusConditionSubjectAst::EquippedCreature,
                state: StatusConditionStateAst::Attacking,
            },
        ),
        (
            "it is attacking alone",
            SubjectStatusConditionAst {
                subject: StatusConditionSubjectAst::Source,
                state: StatusConditionStateAst::AttackingAlone,
            },
        ),
    ];

    for (text, expected) in cases {
        let tokens = lex_line(text, 0).expect("lex");

        let parsed = parse_subject_status_condition(&tokens).expect(text);

        assert_eq!(parsed, expected, "{text}");
    }
}

#[test]
fn parse_subject_descriptor_condition_uses_shared_capture_shape() {
    let cases = [
        (
            "enchanted permanent is a creature",
            SubjectDescriptorConditionSubjectAst::EnchantedPermanent,
            ObjectDescriptorAst::CardType(CardType::Creature),
        ),
        (
            "equipped creature is a human",
            SubjectDescriptorConditionSubjectAst::AttachedObject,
            ObjectDescriptorAst::Subtype(Subtype::Human),
        ),
    ];

    for (text, expected_subject, expected_descriptor) in cases {
        let tokens = lex_line(text, 0).expect("lex");

        let parsed = parse_subject_descriptor_condition(&tokens).expect(text);

        assert_eq!(parsed.subject, expected_subject, "{text}");
        assert_eq!(parsed.descriptor, expected_descriptor, "{text}");
        assert!(!parsed.filter.tagged_constraints.is_empty(), "{text}");
    }
}

#[test]
fn parses_object_attachment_relationship_condition() {
    let tokens = lex_line(
        "an Equipment named Groom's Finery is attached to a creature you control",
        0,
    )
    .expect("lex attachment condition");
    let parsed =
        parse_object_attached_to_object_condition(&tokens).expect("typed attachment condition");
    assert_eq!(
        parsed.attachment_filter.name.as_deref(),
        Some("grooms finery")
    );
    assert!(
        parsed
            .attached_to_filter
            .card_types
            .contains(&CardType::Creature)
    );
    assert_eq!(
        parsed.attached_to_filter.controller,
        Some(PlayerFilter::You)
    );
    assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(1));
    assert_eq!(
        parsed.display,
        "an Equipment named Groom's Finery is attached to a creature you control"
    );

    let tokens = lex_line("two or more Equipment are attached to it", 0)
        .expect("lex counted attachment condition");
    let parsed =
        parse_object_attached_to_object_condition(&tokens).expect("counted attachment condition");
    assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(2));
    assert!(
        parsed
            .attachment_filter
            .subtypes
            .contains(&Subtype::Equipment)
    );
    assert!(
        parsed
            .attached_to_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == "__it__"
                    && matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    )
            })
    );

    let tokens = lex_line("another Aura is attached to enchanted creature", 0)
        .expect("lex other-Aura attachment condition");
    let parsed = parse_object_attached_to_object_condition(&tokens)
        .expect("other-Aura attachment condition");
    assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(1));
    assert!(parsed.attachment_filter.other);
    assert!(parsed.attachment_filter.subtypes.contains(&Subtype::Aura));
}

#[test]
fn parse_ownership_condition_uses_shared_capture_shape() {
    let cases = [
        (
            "you own three or more artifacts",
            OwnershipConditionAst {
                player: PlayerAst::You,
                player_filter: Some(PlayerFilter::You),
                comparison: Comparison::GreaterThanOrEqual(3),
                quantity_token_count: 3,
                quantity_words: vec!["three".to_string(), "or".to_string(), "more".to_string()],
                object_words: vec!["artifacts".to_string()],
                filter: ObjectFilter::artifact().owned_by(PlayerFilter::You),
            },
        ),
        (
            "an opponent owns exactly two lands",
            OwnershipConditionAst {
                player: PlayerAst::Opponent,
                player_filter: Some(PlayerFilter::Opponent),
                comparison: Comparison::Equal(2),
                quantity_token_count: 2,
                quantity_words: vec!["exactly".to_string(), "two".to_string()],
                object_words: vec!["lands".to_string()],
                filter: ObjectFilter::land().owned_by(PlayerFilter::Opponent),
            },
        ),
    ];

    for (text, expected) in cases {
        let tokens = lex_line(text, 0).expect("lex");

        let parsed = parse_ownership_condition(
            &tokens,
            OwnershipConditionOptions {
                allow_opponent_players: true,
                bind_filter_owner_to_subject: true,
                default_filter_zone: None,
            },
        )
        .expect(text);

        assert_eq!(parsed, expected, "{text}");
    }
}

#[test]
fn parse_control_condition_preserves_another_as_object_modifier() {
    let tokens = lex_line("you control another artifact", 0).expect("lex");

    let parsed = parse_control_condition(
        &tokens,
        ControlConditionOptions {
            bind_filter_controller_to_subject: true,
            ..ControlConditionOptions::default()
        },
    )
    .expect("control condition should parse");

    assert_eq!(parsed.at_least_count(), Some(1));
    assert_eq!(parsed.filter.card_types, vec![CardType::Artifact]);
    assert_eq!(parsed.filter.controller, Some(PlayerFilter::You));
    assert!(parsed.filter.other, "{parsed:?}");
}

#[test]
fn parse_control_condition_supports_opt_in_defending_player_subject() {
    let tokens = lex_line("defending player controls a snow land", 0).expect("lex");

    let parsed = parse_control_condition(
        &tokens,
        ControlConditionOptions {
            allow_defending_player: true,
            bind_filter_controller_to_subject: false,
            ..ControlConditionOptions::default()
        },
    )
    .expect("defending-player control condition should parse");

    assert_eq!(parsed.player, PlayerAst::Defending);
    assert_eq!(parsed.player_filter, Some(PlayerFilter::Defending));
    assert_eq!(parsed.at_least_count(), Some(1));
    assert_eq!(parsed.filter.card_types, vec![CardType::Land]);
    assert!(
        parsed
            .filter
            .supertypes
            .contains(&crate::types::Supertype::Snow)
    );
}

#[test]
fn player_status_subjects_lower_typed_contextual_references() {
    let tokens = lex_line("that player is the monarch", 0).expect("lex");
    let parsed = parse_player_status_condition(&tokens).expect("that-player status");
    assert_eq!(parsed.player, PlayerAst::That);
    assert_eq!(parsed.status, PlayerStatusAst::Monarch);
}

#[test]
fn deferred_player_subject_modes_keep_contextual_lowering_distinct() {
    let that_player = lex_line("that player", 0).expect("lex");
    let that_player = LexedClause::new(&that_player);
    assert_eq!(
        parse_player_has_quantity_subject_clause(that_player),
        Some(PlayerAst::That)
    );
    assert_eq!(
        parse_life_relation_player_subject_clause(that_player),
        Some(PlayerFilter::IteratedPlayer)
    );
    assert_eq!(
        parse_spell_cast_this_turn_subject_clause(that_player),
        Some(PlayerFilter::Active)
    );

    let source_contraction = lex_line("you've", 0).expect("lex");
    assert_eq!(
        parse_spell_cast_this_turn_subject_clause(LexedClause::new(&source_contraction)),
        Some(PlayerFilter::You)
    );

    let odd_each = lex_line("each opponents", 0).expect("lex");
    assert_eq!(
        parse_life_relation_player_subject_clause(LexedClause::new(&odd_each)),
        Some(PlayerFilter::Opponent)
    );
}

#[test]
fn parse_player_has_quantity_object_condition_uses_shared_capture_shape() {
    let opponents = lex_line("you have two or more opponents", 0).expect("lex");
    let parsed = parse_player_has_quantity_object_condition(
        &opponents,
        &[&["opponents"]],
        "opponents condition",
    )
    .expect("player has opponents condition should parse");

    assert_eq!(parsed.player, PlayerAst::You);
    assert_eq!(
        comparison_to_strict_at_least_threshold(&parsed.comparison),
        Some(2)
    );

    let life = lex_line("a player has 13 or less life", 0).expect("lex");
    let parsed = parse_player_has_quantity_object_condition(&life, &[&["life"]], "life condition")
        .expect("player has life condition should parse");

    assert_eq!(parsed.player, PlayerAst::Any);
    assert_eq!(parsed.comparison, Comparison::LessThanOrEqual(13));
}

#[test]
fn typed_zone_change_shapes_preserve_condition_semantics() {
    let death = lex_line("Two creatures died under your control this turn.", 0).expect("lex");
    let death = parse_object_death_this_turn_condition(&death).expect("death condition");
    assert_eq!(death.event, ObjectDeathThisTurnEventAst::Died);
    assert_eq!(death.comparison, Comparison::Equal(2));
    assert_eq!(death.under_controller, Some(PlayerFilter::You));

    let entry = lex_line(
        "Another creature entered the battlefield under your control this turn.",
        0,
    )
    .expect("lex");
    let BattlefieldEntryConditionAst::ObjectEntered {
        filter,
        window,
        min_count: _,
    } = parse_battlefield_entry_condition(&entry).expect("entry condition")
    else {
        panic!("expected object entry condition");
    };
    assert_eq!(window, BattlefieldEntryTurnWindowAst::ThisTurn);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.other);

    let left = lex_line("No permanents left the battlefield this turn.", 0).expect("lex");
    assert_eq!(
        parse_battlefield_change_this_turn_condition(&left),
        Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: true })
    );
}

#[test]
fn typed_zone_change_shapes_parse_one_or_more_creatures_died() {
    let death = lex_line("one or more creatures died this turn", 0).expect("lex");
    let death = parse_object_death_this_turn_condition(&death).expect("death condition");
    assert_eq!(death.comparison, Comparison::GreaterThanOrEqual(1));
}

#[test]
fn typed_event_references_and_actions_preserve_condition_semantics() {
    let spell = lex_line("No mana was spent to cast it.", 0).expect("lex");
    assert_eq!(
        parse_spell_context_condition(&spell),
        Some(SpellContextConditionAst::NoManaSpentToCast {
            spell: SpellContextReferenceAst::TargetSpell,
        })
    );

    let action = lex_line("You would begin an extra turn.", 0).expect("lex");
    assert_eq!(
        parse_player_would_action_condition(&action),
        Some(PlayerWouldActionConditionAst {
            player: PlayerFilter::You,
            action: PlayerWouldActionAst::BeginExtraTurn,
        })
    );
}
