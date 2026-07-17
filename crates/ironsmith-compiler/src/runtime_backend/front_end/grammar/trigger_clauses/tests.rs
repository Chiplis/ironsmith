use super::*;
use crate::runtime_backend::util::tokenize_line;

#[test]
fn maps_semantic_atoms_across_punctuation() {
    let tokens = tokenize_line("a creature, attacks", 0);
    assert_eq!(
        parse_trigger_clause_atom_token(&tokens, TriggerClauseAtom::Attack),
        Some(3)
    );
    assert_eq!(
        parse_trigger_word_span_tokens(&tokens, 2),
        Some(TriggerClauseTokenSpan { first: 3, end: 4 })
    );
}

#[test]
fn parses_activation_tap_cost_qualifiers() {
    let tokens = tokenize_line("an ability without {T} in its activation cost", 0);
    let parsed = parse_activation_cost_tap_condition(&tokens).unwrap();
    assert!(!parsed.required);
    assert_eq!(parsed.condition_word, 2);
    assert_eq!(parsed.condition_token, 2);
}

#[test]
fn keeps_list_or_but_splits_clause_or() {
    let list = tokenize_line("one or more creatures", 0);
    assert_eq!(parse_trigger_or_split(&list), None);

    let clauses = tokenize_line("this attacks or this blocks", 0);
    assert_eq!(
        parse_trigger_or_split(&clauses),
        Some(TriggerOrSplit { separator: 2 })
    );
}

#[test]
fn parses_counter_spans_and_recipient() {
    let tokens = tokenize_line("one or more charge counters are put on a creature", 0);
    let descriptor = parse_counter_descriptor_spans(&tokens, 0, 4).unwrap();
    assert_eq!(descriptor.descriptor, 0..4);
    assert_eq!(
        parse_trigger_counter_type(&tokens[descriptor.with_counter]),
        Some(CounterType::Charge)
    );
    assert_eq!(
        parse_counter_recipient_span(&tokens, 8).unwrap().tokens,
        9..10
    );
}

#[test]
fn parses_ability_owner_tails() {
    let named = tokenize_line("a ninjutsu ability", 0);
    assert_eq!(
        parse_named_ability_tail(&named),
        Some(NamedAbilityTail {
            marker: "ninjutsu".to_string(),
        })
    );

    let possessive = tokenize_line("a creatures boast ability", 0);
    let parsed = parse_possessive_ability_tail(&possessive).unwrap();
    assert_eq!(parsed.owner, 0..2);
    assert_eq!(parsed.marker.as_deref(), Some("boast"));

    let of_object = tokenize_line("an ability of a creature that isnt a mana ability", 0);
    let parsed = parse_ability_of_object_tail(&of_object).unwrap();
    assert_eq!(parsed.filter, 3..5);
    assert!(parsed.non_mana_only);
}

#[test]
fn named_activated_ability_reaches_the_typed_trigger_ast() {
    let tokens = tokenize_line("you activate a ninjutsu ability", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("named activated-ability trigger should parse");
    let crate::runtime_backend::ast::TriggerSpec::AbilityActivated {
        activator,
        filter,
        loyalty_only,
        ..
    } = parsed
    else {
        panic!("expected an activated-ability trigger");
    };

    assert_eq!(activator, PlayerFilter::You);
    assert_eq!(filter.ability_markers, vec!["ninjutsu".to_string()]);
    assert!(!loyalty_only);
}

#[test]
fn parses_players_attacked_subject_as_typed_span() {
    let tokens = tokenize_line("one or more opponents are attacked", 0);
    let parsed = parse_players_attacked_clause(&tokens).unwrap();
    assert_eq!(parsed.player, 0..4);
}

#[test]
fn parses_fully_unlock_room_as_typed_keyword_action() {
    for text in ["you fully unlock a Room", "you fully unlocked a Room"] {
        let tokens = tokenize_line(text, 0);
        let parsed = parse_fully_unlock_room_trigger(&tokens).expect(text);
        assert_eq!(parsed.action, KeywordActionKind::UnlockDoor);
        assert_eq!(parsed.player, PlayerFilter::You);
        assert_eq!(parsed.source_filter.subtypes, [Subtype::Room]);
    }
}

#[test]
fn parses_trigger_subject_and_origin_surfaces_as_typed_facts() {
    assert_eq!(
        parse_not_during_turn_draw_suffix_words(
            &["a", "card", "if", "it", "isnt", "your", "turn",]
        ),
        Some(PlayerFilter::You)
    );
    assert_eq!(
        parse_enters_origin_clause_words(&["from", "your", "graveyard"]),
        Some(EntersOriginClause {
            zone: Zone::Graveyard,
            owner: Some(PlayerFilter::You),
        })
    );

    let source = parse_source_trigger_subject_words(&["this", "artifact", "creature"]);
    assert_eq!(source.filter.card_types, [CardType::Creature]);

    let compound = parse_you_or_controlled_object_subject_words(&[
        "you", "or", "a", "creature", "you", "control",
    ])
    .unwrap();
    assert_eq!(compound.player, PlayerFilter::You);
    assert_eq!(compound.filter.card_types, [CardType::Creature]);
    assert_eq!(compound.filter.controller, Some(PlayerFilter::You));

    assert_eq!(
        parse_opponents_each_lose_exact_life_words(&[
            "one",
            "or",
            "more",
            "opponents",
            "each",
            "lose",
            "exactly",
            "three",
            "life",
        ]),
        Some(3)
    );
    assert_eq!(
        parse_roll_result_words(&["a", "dies", "highest", "natural", "result"]),
        Some(RollResultShape::HighestNatural)
    );
    assert_eq!(
        parse_roll_result_words(&["six"]),
        Some(RollResultShape::Fixed(6))
    );
    assert_eq!(
        parse_roll_result_words(&["one", "or", "more", "dice"]),
        Some(RollResultShape::OneOrMoreDice)
    );
}

#[test]
fn grouped_dice_roll_reaches_the_typed_trigger_ast() {
    let tokens = tokenize_line("you roll one or more dice", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();
    assert_eq!(
        parsed,
        crate::runtime_backend::ast::TriggerSpec::PlayerRollsDie {
            player: PlayerFilter::You,
            one_or_more: true,
        }
    );
}

#[test]
fn grouped_opponent_life_loss_reaches_the_typed_trigger_ast() {
    let tokens = tokenize_line("one or more opponents lose life", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();
    assert_eq!(
        parsed,
        crate::runtime_backend::ast::TriggerSpec::PlayersLoseLifeOneOrMore(PlayerFilter::Opponent,)
    );
    assert_eq!(
        crate::runtime_backend::compile_support::compile_trigger_spec(parsed),
        crate::triggers::Trigger::players_lose_life_one_or_more(PlayerFilter::Opponent),
    );
}

#[test]
fn counter_removed_from_source_this_way_keeps_grouping_and_provenance() {
    for (text, expected_one_or_more) in [
        (
            "one or more counters are removed from this creature this way",
            true,
        ),
        ("a counter is removed from this permanent this way", false),
    ] {
        let tokens = tokenize_line(text, 0);
        let parsed = crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .unwrap_or_else(|err| panic!("{text}: {err:?}"));
        let crate::runtime_backend::ast::TriggerSpec::CounterRemovedFrom {
            filter,
            one_or_more,
            caused_by_source,
        } = &parsed
        else {
            panic!("{text}: expected CounterRemovedFrom, got {parsed:?}");
        };
        assert!(filter.source, "{text}");
        assert_eq!(*one_or_more, expected_one_or_more, "{text}");
        assert!(*caused_by_source, "{text}");

        let lowered = crate::runtime_backend::compile_support::compile_trigger_spec(parsed);
        let ironsmith_core::trigger_model::TriggerKind::CounterRemovedFrom(lowered) = lowered.kind
        else {
            panic!("{text}: expected typed lowered counter-removal trigger");
        };
        assert_eq!(lowered.one_or_more, expected_one_or_more, "{text}");
        assert!(lowered.caused_by_source, "{text}");
    }
}

#[test]
fn another_ability_trigger_reaches_typed_model() {
    let tokens = tokenize_line("another ability triggers", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();

    assert_eq!(
        parsed,
        crate::runtime_backend::ast::TriggerSpec::AbilityTriggered { another: true }
    );
    let lowered = crate::runtime_backend::compile_support::compile_trigger_spec(parsed);
    assert_eq!(lowered.display(), "Whenever another ability triggers");
    assert!(matches!(
        lowered.kind,
        ironsmith_core::trigger_model::TriggerKind::AbilityTriggered { another: true }
    ));
}

#[test]
fn extraordinary_journey_keeps_exile_entry_or_cast_provenance() {
    let tokens = tokenize_line(
        "one or more nontoken creatures enter, if one or more of them entered from exile or was cast from exile",
        0,
    );
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();
    assert!(matches!(
        &parsed,
        crate::runtime_backend::ast::TriggerSpec::EntersBattlefieldOneOrMore {
            origin_condition: Some(
                ironsmith_core::trigger_model::ZoneChangeOriginCondition::MovedFromOrCastFrom(
                    Zone::Exile
                )
            ),
            ..
        }
    ));
    assert_eq!(
        crate::runtime_backend::compile_support::compile_trigger_spec(parsed).display(),
        "Whenever one or more nontoken creatures enter the battlefield, if one or more of them entered from exile or was cast from exile"
    );
}

#[test]
fn parses_and_or_player_object_damage_recipients_as_both_trigger_branches() {
    let tokens = tokenize_line(
        "a red source you control deals damage to one or more permanents and/or players",
        0,
    );
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();

    assert_eq!(
        crate::runtime_backend::compile_support::compile_trigger_spec(parsed.clone()).display(),
        "Whenever a red source you control deals damage to one or more permanents and/or players"
    );

    let crate::runtime_backend::ast::TriggerSpec::Either(object, player) = parsed else {
        panic!("expected player/object damage union");
    };
    let crate::runtime_backend::ast::TriggerSpec::DealsDamageTo {
        source,
        target,
        source_surface,
    } = *object
    else {
        panic!("expected object damage branch");
    };
    assert_eq!(source.zone, None);
    assert_eq!(source.controller, Some(PlayerFilter::You));
    assert_eq!(source.colors, Some(crate::color::ColorSet::RED));
    assert_eq!(source_surface, crate::triggers::DamageSourceSurface::Source);
    assert_eq!(
        target.union_connective(),
        crate::filter::ObjectFilterUnionConnective::AndOr
    );
    assert!(target.union_is_one_or_more());
    let crate::runtime_backend::ast::TriggerSpec::DealsDamageToPlayer {
        source: player_source,
        player,
        source_surface: player_source_surface,
    } = *player
    else {
        panic!("expected player damage branch");
    };
    assert_eq!(player, PlayerFilter::Any);
    assert_eq!(player_source, source);
    assert_eq!(player_source_surface, source_surface);
}

#[test]
fn generic_source_to_player_keeps_authored_surface_in_ast_and_model() {
    let tokens = tokenize_line("a source an opponent controls deals damage to you", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();

    let crate::runtime_backend::ast::TriggerSpec::DealsDamageToPlayer {
        source,
        player,
        source_surface,
    } = parsed.clone()
    else {
        panic!("expected source-damage-to-player trigger");
    };
    assert_eq!(source.zone, None);
    assert_eq!(source.controller, Some(PlayerFilter::Opponent));
    assert_eq!(player, PlayerFilter::You);
    assert_eq!(source_surface, crate::triggers::DamageSourceSurface::Source);

    let compiled = crate::runtime_backend::compile_support::compile_trigger_spec(parsed);
    let crate::triggers::TriggerKind::DealsDamageToPlayer {
        source,
        player,
        source_surface,
    } = compiled.kind
    else {
        panic!("expected compiled source-damage-to-player trigger");
    };
    assert_eq!(source.zone, None);
    assert_eq!(source.controller, Some(PlayerFilter::Opponent));
    assert_eq!(player, PlayerFilter::You);
    assert_eq!(source_surface, crate::triggers::DamageSourceSurface::Source);
}

#[test]
fn one_or_more_damage_cardinality_survives_typed_lowering() {
    let arashin = tokenize_line(
        "this creature deals combat damage to one or more blocking creatures",
        0,
    );
    let arashin =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &arashin,
        )
        .expect("Arashin-style grouped damage recipient should parse");
    let crate::runtime_backend::ast::TriggerSpec::ThisDealsCombatDamageTo(arashin_target) =
        &arashin
    else {
        panic!("expected source combat-damage recipient trigger, got {arashin:?}");
    };
    assert!(arashin_target.union_is_one_or_more());

    let briarbridge = tokenize_line("this creature deals damage to one or more creatures", 0);
    let briarbridge =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &briarbridge,
        )
        .expect("Briarbridge-style grouped damage recipient should parse");
    let crate::runtime_backend::ast::TriggerSpec::ThisDealsDamageTo(briarbridge_target) =
        &briarbridge
    else {
        panic!("expected source damage recipient trigger, got {briarbridge:?}");
    };
    assert!(briarbridge_target.union_is_one_or_more());

    let thing = tokenize_line("one or more Heroes you control deal damage to a player", 0);
    let thing =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &thing,
        )
        .expect("Thing-style grouped damage source should parse");
    let crate::runtime_backend::ast::TriggerSpec::DealsDamageToPlayer { source, player, .. } =
        &thing
    else {
        panic!("expected grouped source damage-to-player trigger, got {thing:?}");
    };
    assert!(source.union_is_one_or_more());
    assert_eq!(*player, PlayerFilter::Any);
}

#[test]
fn noncreature_source_without_recipient_is_zone_less_in_ast_and_model() {
    let tokens = tokenize_line("a noncreature source you control deals damage", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();

    let crate::runtime_backend::ast::TriggerSpec::DealsDamage {
        source,
        source_surface,
    } = parsed.clone()
    else {
        panic!("expected generic source-damage trigger");
    };
    assert_eq!(source.zone, None);
    assert_eq!(source.controller, Some(PlayerFilter::You));
    assert_eq!(source.excluded_card_types, [CardType::Creature]);
    assert_eq!(source_surface, crate::triggers::DamageSourceSurface::Source);

    let compiled = crate::runtime_backend::compile_support::compile_trigger_spec(parsed);
    let crate::triggers::TriggerKind::DealsDamage {
        filter,
        source_surface,
    } = compiled.kind
    else {
        panic!("expected compiled generic source-damage trigger");
    };
    assert_eq!(filter.zone, None);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.excluded_card_types, [CardType::Creature]);
    assert_eq!(source_surface, crate::triggers::DamageSourceSurface::Source);
}

#[test]
fn parses_one_or_more_and_or_blockers_as_shared_union_metadata() {
    let tokens = tokenize_line(
        "this creature blocks or becomes blocked by one or more blue and/or black creatures",
        0,
    );
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();

    let crate::runtime_backend::ast::TriggerSpec::Either(blocks, becomes_blocked) = parsed else {
        panic!("expected blocks/becomes-blocked trigger union");
    };
    let crate::runtime_backend::ast::TriggerSpec::ThisBlocksObject(blocker) = *blocks else {
        panic!("expected filtered blocks branch");
    };
    let crate::runtime_backend::ast::TriggerSpec::ThisBecomesBlockedByObject(blocking) =
        *becomes_blocked
    else {
        panic!("expected filtered becomes-blocked branch");
    };

    assert_eq!(blocker, blocking);
    assert_eq!(
        blocker.union_connective(),
        crate::filter::ObjectFilterUnionConnective::AndOr
    );
    assert!(blocker.union_is_one_or_more());
    assert_eq!(blocker.card_types, [CardType::Creature]);
}

#[test]
fn zone_change_unions_reach_runtime_with_quantifier_and_connective_surfaces() {
    let cases = [
        (
            "one or more other creatures and/or artifacts you control die",
            "Whenever one or more other creatures and/or artifacts you control die",
        ),
        (
            "one or more other Rabbits, Bats, Birds, and/or Mice you control enter",
            "Whenever one or more other Rabbits, Bats, Birds, and/or Mice you control enter the battlefield",
        ),
        (
            "another Villain and/or artifact you control enters",
            "Whenever another artifact and/or Villain you control enters the battlefield",
        ),
    ];

    for (clause, expected) in cases {
        let tokens = tokenize_line(clause, 0);
        let parsed = crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap_or_else(|error| panic!("failed to parse {clause:?}: {error}"));
        assert_eq!(
            crate::runtime_backend::compile_support::compile_trigger_spec(parsed).display(),
            expected,
            "unexpected compiled trigger surface for {clause:?}"
        );
    }
}

#[test]
fn combat_damage_recipients_prefer_registered_source_names_over_subtypes() {
    crate::runtime_backend::util::with_source_reference_context("Vraska the Unseen", || {
        for (recipient, expected_surface) in [
            (
                "Vraska the Unseen",
                crate::target::SourceReferenceSurface::FullName("Vraska the Unseen".to_string()),
            ),
            (
                "Vraska",
                crate::target::SourceReferenceSurface::ShortName("Vraska".to_string()),
            ),
        ] {
            let tokens =
                tokenize_line(&format!("a creature deals combat damage to {recipient}"), 0);
            let parsed = crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
                &tokens,
            )
            .unwrap_or_else(|error| panic!("failed to parse {recipient:?}: {error}"));
            let crate::runtime_backend::ast::TriggerSpec::DealsCombatDamageTo { target, .. } =
                parsed
            else {
                panic!("expected combat-damage-to trigger for {recipient:?}: {parsed:#?}");
            };

            assert!(target.source, "{recipient:?} must resolve to the source");
            assert_eq!(target.source_surface, Some(expected_surface));
            assert!(
                target.subtypes.is_empty(),
                "{recipient:?} must not fall through to subtype parsing: {target:#?}"
            );
        }
    });
}
