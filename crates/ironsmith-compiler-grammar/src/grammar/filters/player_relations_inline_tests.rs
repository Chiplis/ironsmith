use super::*;

#[test]
fn parses_comparison_axes_with_consumed_words() {
    assert_eq!(
        parse_spell_filter_comparison_axis_words(&["power", "greater"]),
        Some((SpellFilterComparisonAxis::Power, 1))
    );
    assert_eq!(
        parse_spell_filter_comparison_axis_words(&["toughness", "less"]),
        Some((SpellFilterComparisonAxis::Toughness, 1))
    );
    assert_eq!(
        parse_spell_filter_comparison_axis_words(&["mana", "value", "equal"]),
        Some((SpellFilterComparisonAxis::ManaValue, 2))
    );
}

#[test]
fn parses_player_relation_subjects_directly() {
    let pronoun = PlayerFilter::ChosenPlayer;
    for (words, expected, consumed) in [
        (
            &[
                "that",
                "opponent",
                "or",
                "that",
                "planeswalkers",
                "controller",
                "controls",
            ][..],
            PlayerFilter::TargetPlayerOrControllerOfTarget,
            6,
        ),
        (
            &[
                "that",
                "player",
                "or",
                "that",
                "planeswalkers",
                "controller",
                "controls",
            ][..],
            PlayerFilter::TargetPlayerOrControllerOfTarget,
            6,
        ),
        (
            &["your", "team", "controls"][..],
            PlayerFilter::your_team(),
            2,
        ),
        (
            &["your", "opponents", "control"][..],
            PlayerFilter::Opponent,
            2,
        ),
        (
            &["that", "player", "owns"][..],
            PlayerFilter::IteratedPlayer,
            2,
        ),
        (
            &["target", "opponent", "controls"][..],
            PlayerFilter::target_opponent(),
            2,
        ),
        (
            &["those", "opponents", "control"][..],
            PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)),
            2,
        ),
        (
            &["attacking", "player", "controls"][..],
            PlayerFilter::Attacking,
            2,
        ),
        (
            &["their", "controllers", "control"][..],
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            2,
        ),
        (&["they", "control"][..], pronoun.clone(), 1),
        (&["voter", "owns"][..], PlayerFilter::IteratedPlayer, 1),
    ] {
        assert_eq!(
            parse_player_relation_subject(words, &pronoun),
            Some((expected, consumed))
        );
    }
}

#[test]
fn player_or_planeswalker_controller_reference_does_not_expand_counted_object_types() {
    for subject in [
        "that opponent or that planeswalker's controller",
        "that player or that planeswalker's controller",
    ] {
        let tokens = crate::lexer::lex_line(&format!("creatures {subject} controls"), 0)
            .expect("controller-relative object filter should lex");
        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
            .expect("controller-relative object filter should parse");

        assert_eq!(filter.card_types, vec![CardType::Creature], "{subject}");
        assert_eq!(
            filter.controller,
            Some(PlayerFilter::TargetPlayerOrControllerOfTarget),
            "{subject}"
        );
        assert_eq!(filter.zone, Some(Zone::Battlefield), "{subject}");
        assert!(
            !filter.card_types.contains(&CardType::Planeswalker),
            "{subject}: {filter:#?}"
        );
    }
}

#[test]
fn applies_passive_voter_owner_relation() {
    let mut filter = ObjectFilter::default();
    assert_eq!(
        try_apply_passive_player_relation_clause(
            &mut filter,
            &["owned", "by", "voter", "tail"],
            &PlayerFilter::Any,
        ),
        Some(3)
    );
    assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
}

#[test]
fn applies_negated_you_relations() {
    let mut control_filter = ObjectFilter::default();
    assert_eq!(
        try_apply_negated_you_relation_clause(
            &mut control_filter,
            &["you", "do", "not", "control", "creatures"],
            &PlayerFilter::IteratedPlayer,
        ),
        Some(4)
    );
    assert_eq!(control_filter.controller, Some(PlayerFilter::NotYou));

    let mut owner_filter = ObjectFilter::default();
    assert_eq!(
        try_apply_negated_you_relation_clause(
            &mut owner_filter,
            &["don't", "owns", "cards"],
            &PlayerFilter::IteratedPlayer,
        ),
        Some(2)
    );
    assert_eq!(owner_filter.owner, Some(PlayerFilter::NotYou));

    let mut participant_filter = ObjectFilter::default();
    assert_eq!(
        try_apply_negated_you_relation_clause(
            &mut participant_filter,
            &["they", "don't", "control", "permanents"],
            &PlayerFilter::IteratedPlayer,
        ),
        Some(3)
    );
    assert_eq!(
        participant_filter.controller,
        Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::IteratedPlayer,
        ))
    );
}

#[test]
fn applies_joint_neither_owned_nor_controlled_relation() {
    for words in [
        &["you", "neither", "own", "nor", "control", "permanents"][..],
        &["you", "neither", "control", "nor", "own", "permanents"][..],
    ] {
        let mut filter = ObjectFilter::default();
        assert_eq!(
            try_apply_neither_owned_nor_controlled_clause(&mut filter, words),
            Some(5)
        );
        assert_eq!(filter.owner, Some(PlayerFilter::NotYou));
        assert_eq!(filter.controller, Some(PlayerFilter::NotYou));
    }
}

#[test]
fn applies_chosen_player_graveyard_fact() {
    let mut filter = ObjectFilter::default();
    assert_eq!(
        try_apply_chosen_player_graveyard_clause(
            &mut filter,
            &["the", "chosen", "players", "graveyard", "cards"]
        ),
        Some(4)
    );
    assert_eq!(filter.owner, Some(PlayerFilter::ChosenPlayer));
    assert_eq!(filter.zone, Some(Zone::Graveyard));
}

#[test]
fn parses_joint_and_disjunctive_owner_controller_relations() {
    let mut filter = ObjectFilter::default();
    assert_eq!(
        try_apply_joint_owner_controller_clause(
            &mut filter,
            &["you", "both", "own", "and", "controls", "cards"],
            &PlayerFilter::Any,
        ),
        Some(5)
    );
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.controller, Some(PlayerFilter::You));

    assert_eq!(
        parse_owner_or_controller_disjunction_player(
            &["opponents", "control", "or", "owns", "cards"],
            &PlayerFilter::Any,
        ),
        Some((PlayerFilter::Opponent, 4))
    );
    assert_eq!(
        parse_owner_or_controller_disjunction_player(
            &["you", "own", "or", "owns", "cards"],
            &PlayerFilter::Any,
        ),
        None
    );
}

#[test]
fn parses_entered_battlefield_variants() {
    for (words, expected_controller, expected_consumed) in [
        (
            &[
                "entered",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
                "this",
                "turn",
                "tail",
            ][..],
            Some(PlayerFilter::You),
            8,
        ),
        (
            &[
                "entered",
                "battlefield",
                "under",
                "opponent",
                "control",
                "this",
                "turn",
            ][..],
            Some(PlayerFilter::Opponent),
            7,
        ),
        (
            &["entered", "under", "opponents", "control", "this", "turn"][..],
            Some(PlayerFilter::Opponent),
            6,
        ),
        (
            &["entered", "the", "battlefield", "this", "turn"][..],
            None,
            5,
        ),
        (&["entered", "battlefield", "this", "turn"][..], None, 4),
        (&["entered", "this", "turn"][..], None, 3),
    ] {
        assert_eq!(
            parse_entered_battlefield_this_turn_words(words),
            Some((expected_controller, expected_consumed))
        );
    }
}

#[test]
fn parses_graveyard_and_drawn_turn_events() {
    assert_eq!(
        parse_put_there_from_battlefield_this_turn_words(&[
            "that",
            "were",
            "put",
            "there",
            "from",
            "battlefield",
            "this",
            "turn",
            "tail",
        ]),
        Some(8)
    );
    assert_eq!(
        parse_put_there_from_anywhere_this_turn_words(&[
            "that", "was", "put", "there", "from", "anywhere", "this", "turn",
        ]),
        Some(8)
    );
    assert_eq!(
        parse_graveyard_from_battlefield_this_turn_words(&[
            "graveyards",
            "from",
            "battlefield",
            "this",
            "turn",
        ]),
        Some(5)
    );
    assert_eq!(
        parse_drawn_this_turn_words(&["drawn", "this", "turn", "tail"]),
        Some(3)
    );
    assert_eq!(
        parse_drawn_this_turn_words(&["drawn", "last", "turn"]),
        None
    );
}

#[test]
fn applies_was_dealt_damage_history_without_conflating_active_voice() {
    for mut words in [
        vec![
            "target", "creature", "that", "was", "dealt", "damage", "this", "turn",
        ],
        vec!["each", "creature", "dealt", "damage", "this", "turn"],
    ] {
        let mut filter = ObjectFilter::default();
        let mut tokens = Vec::new();
        assert!(try_apply_was_dealt_damage_this_turn_clause(
            &mut filter,
            &mut words,
            &mut tokens,
        ));
        assert!(filter.was_dealt_damage_this_turn);
    }

    let mut active_words = vec![
        "target", "creature", "that", "dealt", "damage", "this", "turn",
    ];
    let mut active_filter = ObjectFilter::default();
    let mut tokens = Vec::new();
    assert!(!try_apply_was_dealt_damage_this_turn_clause(
        &mut active_filter,
        &mut active_words,
        &mut tokens,
    ));
    assert!(!active_filter.was_dealt_damage_this_turn);
}

#[test]
fn target_choice_references_retain_the_authored_chooser() {
    for (mut words, expected_tag) in [
        (
            vec!["creature", "you", "chose"],
            ABILITY_CONTROLLER_TARGET_CHOICE_TAG,
        ),
        (
            vec!["creature", "your", "opponent", "chose"],
            OPPONENT_TARGET_CHOICE_TAG,
        ),
    ] {
        let mut filter = ObjectFilter::default();
        assert!(try_apply_target_choice_attribution_reference(
            &mut filter,
            &mut words,
        ));
        assert_eq!(words, ["creature"]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == expected_tag
        }));
    }
}

#[test]
fn plural_demonstrative_reference_preserves_plural_noun_surface() {
    let mut words = vec!["those", "creature", "cards"];
    let mut filter = ObjectFilter::default();

    assert!(try_apply_leading_tagged_reference_prefix(
        &mut filter,
        &mut words,
    ));
    assert_eq!(words, ["creature", "cards"]);
    assert!(filter.has_plural_object_noun_surface());
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == IT_TAG
    }));
}
