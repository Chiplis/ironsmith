use super::*;
use crate::runtime_backend::util::tokenize_line;

#[test]
fn parses_generic_sticker_trigger_with_source_recipient() {
    let tokens = tokenize_line("you put a sticker on this enchantment", 0);
    let parsed =
        crate::runtime_backend::front_end::shared::util::with_card_source_reference_context(
            "_____ Balls of Fire",
            &[CardType::Enchantment],
            &[],
            || {
                crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
                    &tokens,
                )
            },
        )
        .expect("generic sticker trigger should parse");

    let crate::runtime_backend::ast::TriggerSpec::KeywordAction {
        action,
        player,
        source_filter: Some(source_filter),
    } = parsed
    else {
        panic!("expected a keyword-action trigger");
    };
    assert_eq!(action, crate::events::KeywordActionKind::Sticker);
    assert_eq!(player, PlayerFilter::You);
    assert!(source_filter.source, "{source_filter:#?}");
    assert_eq!(
        source_filter.source_surface,
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this enchantment".to_string()
        )),
        "{source_filter:#?}"
    );
}

#[test]
fn parses_typed_sticker_trigger_with_object_recipient() {
    let tokens = tokenize_line("an opponent puts an ability sticker on a creature", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("typed sticker trigger should parse");

    let crate::runtime_backend::ast::TriggerSpec::KeywordAction {
        action,
        player,
        source_filter: Some(source_filter),
    } = &parsed
    else {
        panic!("expected a keyword-action trigger, got {parsed:#?}");
    };
    assert_eq!(*action, crate::events::KeywordActionKind::AbilitySticker);
    assert_eq!(*player, PlayerFilter::Opponent);
    assert_eq!(source_filter.card_types, [CardType::Creature]);
}

#[test]
fn parses_split_possessive_unpaid_cumulative_upkeep_trigger() {
    let tokens = tokenize_line(
        "a player doesn't pay this enchantment's cumulative upkeep",
        0,
    );
    let parsed =
        crate::runtime_backend::front_end::shared::util::with_card_source_reference_context(
            "Heart of Bogardan",
            &[CardType::Enchantment],
            &[],
            || {
                crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
                    &tokens,
                )
            },
        )
        .expect("unpaid cumulative upkeep trigger should parse");

    assert_eq!(
        parsed,
        crate::runtime_backend::ast::TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::CumulativeUpkeepNotPaid,
            player: PlayerFilter::Any,
        }
    );
}

#[test]
fn parses_spell_causes_you_to_gain_life_as_a_causal_trigger() {
    let tokens = tokenize_line(
        "a white instant or sorcery spell causes you to gain life",
        0,
    );
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("causal life-gain trigger should parse");

    let crate::runtime_backend::ast::TriggerSpec::YouGainLifeCausedBy(source) = &parsed else {
        panic!("expected a causal life-gain trigger, got {parsed:?}");
    };
    assert_eq!(source.zone, Some(crate::Zone::Stack), "{source:#?}");
    assert_eq!(
        source.card_types,
        [crate::CardType::Instant, crate::CardType::Sorcery],
        "{source:#?}"
    );
    assert!(
        source
            .colors
            .is_some_and(|colors| colors.contains(crate::Color::White)),
        "{source:#?}"
    );
    assert_eq!(
        crate::runtime_backend::compile_support::compile_trigger_spec(parsed).display(),
        "Whenever a white instant or sorcery spell causes you to gain life"
    );
}

#[test]
fn shared_spell_noun_damage_source_keeps_stack_controller_and_mana_facts() {
    let tokens = tokenize_line(
        "an instant or sorcery spell you control with mana value 3 or greater deals damage",
        0,
    );
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("shared-noun spell damage trigger should parse");

    let crate::runtime_backend::ast::TriggerSpec::DealsDamage { source, .. } = &parsed else {
        panic!("expected filtered damage-source trigger, got {parsed:?}");
    };
    assert!(source.any_of.is_empty(), "{source:#?}");
    assert_eq!(
        source.card_types,
        [crate::CardType::Instant, crate::CardType::Sorcery],
        "{source:#?}"
    );
    assert_eq!(source.zone, Some(crate::Zone::Stack), "{source:#?}");
    assert_eq!(source.controller, Some(PlayerFilter::You), "{source:#?}");
    assert_eq!(
        source.stack_kind,
        Some(crate::filter::StackObjectKind::Spell),
        "{source:#?}"
    );
    assert!(source.has_mana_cost, "{source:#?}");
    assert_eq!(
        source.mana_value,
        Some(crate::filter::Comparison::GreaterThanOrEqual(3)),
        "{source:#?}"
    );

    let compiled = crate::runtime_backend::compile_support::compile_trigger_spec(parsed);
    let crate::triggers::TriggerKind::DealsDamage { filter, .. } = compiled.kind else {
        panic!("expected lowered damage-source trigger, got {compiled:?}");
    };
    assert_eq!(filter.zone, Some(crate::Zone::Stack), "{filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell),
        "{filter:#?}"
    );
    assert!(filter.has_mana_cost, "{filter:#?}");
    assert_eq!(
        filter.mana_value,
        Some(crate::filter::Comparison::GreaterThanOrEqual(3)),
        "{filter:#?}"
    );
}

#[test]
fn parses_clash_and_win_as_the_winner_aware_trigger() {
    let tokens = tokenize_line("you clash and win", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("clash-and-win trigger should parse");

    assert_eq!(
        parsed,
        crate::runtime_backend::ast::TriggerSpec::WinsClash {
            player: PlayerFilter::You,
            surface: ironsmith_core::ClashWinTriggerSurface::ClashAndWin,
        }
    );
    assert_eq!(
        crate::runtime_backend::compile_support::compile_trigger_spec(parsed).display(),
        "Whenever you clash and win"
    );
}

#[test]
fn parses_passive_damage_by_qualified_source_union() {
    let tokens = tokenize_line(
        "an opponent is dealt damage by a red instant or sorcery spell you control or by a red planeswalker you control",
        0,
    );
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("passive qualified-source damage trigger should parse");

    let crate::runtime_backend::ast::TriggerSpec::DealsDamageToPlayer {
        source,
        player,
        source_surface,
    } = &parsed
    else {
        panic!("expected qualified damage-to-player trigger, got {parsed:#?}");
    };
    assert_eq!(*player, PlayerFilter::Opponent);
    assert_eq!(
        *source_surface,
        crate::triggers::DamageSourceSurface::PassiveBy
    );
    assert_eq!(source.any_of.len(), 2, "{source:#?}");
    assert!(
        source.any_of.iter().all(|branch| {
            branch.controller == Some(PlayerFilter::You)
                && branch
                    .colors
                    .is_some_and(|colors| colors.contains(crate::Color::Red))
        }),
        "{source:#?}"
    );
    assert!(
        source.any_of[0]
            .card_types
            .contains(&crate::CardType::Instant)
            && source.any_of[0]
                .card_types
                .contains(&crate::CardType::Sorcery),
        "{source:#?}"
    );
    assert!(
        source.any_of[1]
            .card_types
            .contains(&crate::CardType::Planeswalker),
        "{source:#?}"
    );

    assert_eq!(
        crate::runtime_backend::compile_support::compile_trigger_spec(parsed).display(),
        "Whenever an opponent is dealt damage by a red instant or sorcery spell you control or a red planeswalker you control"
    );
}

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
fn shared_attack_or_block_subject_is_preserved_on_both_trigger_arms() {
    let tokens = tokenize_line("enchanted creature attacks or blocks", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("shared attack/block trigger should parse");

    let crate::runtime_backend::ast::TriggerSpec::Either(left, right) = parsed else {
        panic!("expected an either trigger");
    };
    let (
        crate::runtime_backend::ast::TriggerSpec::Attacks(attacks),
        crate::runtime_backend::ast::TriggerSpec::Blocks(blocks),
    ) = (*left, *right)
    else {
        panic!("expected matching attacks and blocks arms");
    };
    assert_eq!(attacks, blocks);
    assert!(
        attacks
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == "enchanted")
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
                ironsmith_core::trigger_model::ZoneChangeOriginCondition::MovedFromOrCastFrom {
                    zone: Zone::Exile,
                    zone_owner: None,
                    caster: None,
                    ..
                }
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
    let crate::runtime_backend::ast::TriggerSpec::ThisBlocksObject {
        filter: blocker,
        min_blocked_objects,
    } = *blocks
    else {
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
    assert_eq!(min_blocked_objects, Some(1));
    assert_eq!(blocker.card_types, [CardType::Creature]);
}

#[test]
fn parses_lesser_power_block_relationships_as_typed_pair_triggers() {
    let becomes_tokens = tokenize_line(
        "a creature becomes blocked by an artifact creature with lesser power",
        0,
    );
    let becomes =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &becomes_tokens,
        )
        .expect("relative-power becomes-blocked trigger should parse");
    let crate::runtime_backend::ast::TriggerSpec::BecomesBlockedByObjectWithLesserPower {
        blocked,
        blocker,
    } = becomes
    else {
        panic!("expected typed relative-power becomes-blocked trigger");
    };
    assert_eq!(blocked.card_types, [CardType::Creature]);
    assert!(
        blocker.card_types.contains(&CardType::Artifact)
            || blocker.all_card_types.contains(&CardType::Artifact)
    );
    assert!(
        blocker.card_types.contains(&CardType::Creature)
            || blocker.all_card_types.contains(&CardType::Creature)
    );

    let blocks_tokens = tokenize_line("a red creature blocks a creature with lesser power", 0);
    let blocks =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &blocks_tokens,
        )
        .expect("relative-power blocks trigger should parse");
    let crate::runtime_backend::ast::TriggerSpec::BlocksObjectWithLesserPower { blocker, blocked } =
        blocks
    else {
        panic!("expected typed relative-power blocks trigger");
    };
    assert_eq!(blocker.colors, Some(crate::color::ColorSet::RED));
    assert_eq!(blocker.card_types, [CardType::Creature]);
    assert_eq!(blocked.card_types, [CardType::Creature]);
}

#[test]
fn this_blocks_group_preserves_minimum_cardinality() {
    let tokens = tokenize_line("this creature blocks two or more creatures", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("minimum-cardinality block trigger should parse");
    let crate::runtime_backend::ast::TriggerSpec::ThisBlocksObject {
        filter,
        min_blocked_objects,
    } = &parsed
    else {
        panic!("expected a grouped this-blocks trigger, got {parsed:?}");
    };

    assert_eq!(*min_blocked_objects, Some(2));
    assert!(filter.card_types.contains(&CardType::Creature));
}

#[test]
fn attack_group_quantifiers_preserve_their_minimum_cardinality() {
    for (clause, expected_minimum) in [
        ("two or more creatures your opponents control attack", 2),
        ("three or more creatures you control with flying attack", 3),
    ] {
        let tokens = tokenize_line(clause, 0);
        let parsed =
            crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
                &tokens,
            )
            .unwrap_or_else(|error| panic!("failed to parse {clause:?}: {error}"));

        let crate::runtime_backend::ast::TriggerSpec::AttacksOneOrMoreWithMinTotal {
            min_total_attackers,
            ..
        } = &parsed
        else {
            panic!("expected a minimum-cardinality attack trigger for {clause:?}: {parsed:?}");
        };
        assert_eq!(*min_total_attackers, expected_minimum, "{clause}");
    }
}

#[test]
fn opponent_or_opponent_planeswalker_attack_surfaces_reach_the_typed_trigger() {
    for (clause, expected_one_or_more) in [
        (
            "a creature attacks one of your opponents or a planeswalker an opponent controls",
            false,
        ),
        (
            "one or more creatures attack one of your opponents or a planeswalker they control",
            true,
        ),
    ] {
        let tokens = tokenize_line(clause, 0);
        let parsed =
            crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
                &tokens,
            )
            .unwrap_or_else(|error| panic!("failed to parse {clause:?}: {error}"));

        let filter = match &parsed {
            crate::runtime_backend::ast::TriggerSpec::Attacks(filter) if !expected_one_or_more => {
                filter
            }
            crate::runtime_backend::ast::TriggerSpec::AttacksOneOrMore(filter)
                if expected_one_or_more =>
            {
                filter
            }
            _ => panic!("unexpected attack-trigger shape for {clause:?}: {parsed:#?}"),
        };
        assert_eq!(
            filter
                .attacking_player_or_planeswalker_controlled_by
                .as_ref(),
            Some(&PlayerFilter::Opponent),
            "{clause}"
        );
        assert!(filter.targets_only_player.is_none(), "{clause}");
    }
}

#[test]
fn initiative_holder_attack_target_keeps_dynamic_player_reference() {
    let tokens = tokenize_line("you attack the player who has the initiative", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("initiative-holder attack trigger should parse");

    let crate::runtime_backend::ast::TriggerSpec::AttacksOneOrMore(filter) = &parsed else {
        panic!("expected a group attack trigger, got {parsed:#?}");
    };
    let expected =
        PlayerFilter::TaggedPlayer(crate::TagKey::from(ironsmith_core::INITIATIVE_HOLDER_TAG));
    assert_eq!(
        filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref(),
        Some(&expected)
    );
    assert_eq!(filter.targets_only_player.as_ref(), Some(&expected));
}

#[test]
fn repeated_attack_intro_resolves_enchanted_player_pronoun_on_both_branches() {
    fn without_intro(
        trigger: &crate::runtime_backend::ast::TriggerSpec,
    ) -> &crate::runtime_backend::ast::TriggerSpec {
        match trigger {
            crate::runtime_backend::ast::TriggerSpec::WithIntro { trigger, .. } => {
                without_intro(trigger)
            }
            trigger => trigger,
        }
    }

    let tokens = tokenize_line(
        "when you attack enchanted opponent or a planeswalker they control or when they attack you or a planeswalker you control",
        0,
    );
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("repeated-intro attack union should parse");
    let crate::runtime_backend::ast::TriggerSpec::Either(left, right) = &parsed else {
        panic!("expected a two-branch attack trigger, got {parsed:#?}");
    };
    let crate::runtime_backend::ast::TriggerSpec::AttacksOneOrMore(left_filter) =
        without_intro(left)
    else {
        panic!("expected controller attack branch, got {left:#?}");
    };
    let crate::runtime_backend::ast::TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(
        right_filter,
    ) = without_intro(right)
    else {
        panic!("expected enchanted-player counterattack branch, got {right:#?}");
    };
    let enchanted = PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted"));
    assert_eq!(
        left_filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref(),
        Some(&enchanted)
    );
    assert!(left_filter.targets_only_player.is_none());
    assert_eq!(right_filter.controller.as_ref(), Some(&enchanted));
}

#[test]
fn subject_first_attack_group_does_not_claim_attack_with_surface() {
    let tokens = tokenize_line("one or more suspected creatures you control attack", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("subject-first attack group should parse");

    let crate::runtime_backend::ast::TriggerSpec::AttacksOneOrMore(filter) = &parsed else {
        panic!("expected one-or-more attack trigger, got {parsed:?}");
    };
    assert!(filter.suspected);
    assert!(
        !filter.union_is_one_or_more(),
        "the union flag is reserved for explicit 'attack with one or more' surface"
    );
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
            "Whenever another Villain and/or artifact you control enters the battlefield",
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

#[test]
fn grist_style_singular_origin_clause_scopes_zone_owner_and_caster_on_the_trigger() {
    let tokens = tokenize_line(
        "this creature or another creature you control enters, if it entered from your graveyard or you cast it from your graveyard",
        0,
    );
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();

    fn assert_origin(
        origin: &Option<ironsmith_core::trigger_model::ZoneChangeOriginCondition>,
        context: &str,
    ) {
        let Some(ironsmith_core::trigger_model::ZoneChangeOriginCondition::MovedFromOrCastFrom {
            zone,
            zone_owner,
            caster,
            ..
        }) = origin
        else {
            panic!("{context} must carry a moved-or-cast origin condition, got {origin:?}");
        };
        assert_eq!(*zone, Zone::Graveyard, "{context}");
        assert_eq!(
            *zone_owner,
            Some(crate::target::PlayerFilter::You),
            "{context}"
        );
        assert_eq!(*caster, Some(crate::target::PlayerFilter::You), "{context}");
    }

    let crate::runtime_backend::ast::TriggerSpec::Either(left, right) = &parsed else {
        panic!("expected an either-union of this-enters and another-enters, got {parsed:?}");
    };
    match left.as_ref() {
        crate::runtime_backend::ast::TriggerSpec::ThisEntersBattlefield { origin_condition }
        | crate::runtime_backend::ast::TriggerSpec::ThisEntersBattlefieldWithSurface {
            origin_condition,
            ..
        } => assert_origin(origin_condition, "this-enters branch"),
        other => panic!("expected a this-enters branch, got {other:?}"),
    }
    match right.as_ref() {
        crate::runtime_backend::ast::TriggerSpec::EntersBattlefield {
            origin_condition, ..
        } => assert_origin(origin_condition, "another-enters branch"),
        other => panic!("expected an enters-battlefield branch, got {other:?}"),
    }
}
