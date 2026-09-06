use super::*;
use crate::util::tokenize_line;

#[test]
fn while_qualified_triggers_keep_event_time_conditions() {
    for text in [
        "this creature attacks while you don't control another Dinosaur",
        "this creature attacks while you control two or more artifacts",
        "this creature attacks or blocks while you control a Dinosaur",
        "you attack while you control a creature with power 4 or greater",
        "this creature attacks while you have the most life or are tied for most life",
        "you cast this spell while you control your commander",
    ] {
        let tokens = tokenize_line(text, 0);
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .unwrap_or_else(|error| panic!("failed to parse {text:?}: {error}"));
        assert!(
            matches!(
                parsed,
                crate::model::ast::TriggerSpec::ConditionQualified { .. }
            ),
            "{text}: {parsed:#?}"
        );
    }

    let tokens = tokenize_line("this creature attacks", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("ordinary attack trigger should parse");
    assert!(matches!(
        parsed,
        crate::model::ast::TriggerSpec::ThisAttacks
    ));
}

#[test]
fn filtered_damage_source_death_trigger_keeps_victim_and_damager_filters_distinct() {
    for text in [
        "another creature dealt damage this turn by a Spider you controlled dies",
        "another creature dealt damage by a Spider you controlled this turn dies",
    ] {
        let tokens = tokenize_line(text, 0);
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .expect("filtered damage-history death trigger should parse");
        let crate::model::ast::TriggerSpec::DiesCreatureDealtDamageByFilteredSourceThisTurn {
            victim,
            damager_filter,
        } = parsed
        else {
            panic!("expected typed filtered-damager death trigger, got {parsed:#?}");
        };
        assert!(victim.other);
        assert_eq!(victim.card_types, [CardType::Creature]);
        assert!(victim.subtypes.is_empty());
        assert_eq!(victim.controller, None);
        assert_eq!(damager_filter.subtypes, [Subtype::Spider]);
        assert_eq!(damager_filter.controller, Some(PlayerFilter::You));
    }
}

#[test]
fn filtered_damage_source_death_trigger_requires_turn_history_wording() {
    let tokens = tokenize_line(
        "another creature dealt damage by a Spider you controlled dies",
        0,
    );
    if let Ok(parsed) = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens) {
        assert!(
            !matches!(
                parsed,
                crate::model::ast::TriggerSpec::DiesCreatureDealtDamageByFilteredSourceThisTurn { .. }
            ),
            "without `this turn`, the trigger must not acquire turn-history semantics"
        );
    }
}

#[test]
fn chosen_player_loses_game_trigger_keeps_chosen_player_scope() {
    let tokens = tokenize_line("the chosen player loses the game", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("chosen-player loss trigger should parse");
    assert_eq!(
        parsed,
        crate::model::ast::TriggerSpec::PlayerLosesGame(PlayerFilter::ChosenPlayer)
    );
}

#[test]
fn crime_trigger_preserves_during_your_turn_as_typed_timing() {
    for (text, player) in [
        ("you commit a crime during your turn", PlayerFilter::You),
        (
            "an opponent commits a crime during your turn",
            PlayerFilter::Opponent,
        ),
        (
            "a player commits a crime during your turn",
            PlayerFilter::Any,
        ),
    ] {
        let tokens = tokenize_line(text, 0);
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .expect("turn-qualified crime trigger should parse");
        assert_eq!(
            parsed,
            crate::model::ast::TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::CommitCrime,
                player,
                source_filter: None,
                during_your_turn: true,
            }
        );
    }

    let untimed = tokenize_line("you commit a crime", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&untimed)
        .expect("ordinary crime trigger should still parse");
    assert!(matches!(
        parsed,
        crate::model::ast::TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            during_your_turn: false,
            ..
        }
    ));
}

#[test]
fn your_graveyard_from_library_preempts_the_broad_any_origin_trigger() {
    let tokens = tokenize_line(
        "one or more cards are put into your graveyard from your library",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("origin-qualified graveyard trigger should parse");

    let crate::model::ast::TriggerSpec::PutIntoGraveyardFromZone {
        filter,
        from: Zone::Library,
        one_or_more: true,
    } = parsed
    else {
        panic!("expected exact library-origin trigger, got {parsed:#?}");
    };
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(filter.nontoken);
}

#[test]
fn graveyard_from_anywhere_except_battlefield_keeps_the_excluded_origin() {
    let tokens = tokenize_line(
        "a creature card is put into a graveyard from anywhere other than the battlefield",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("excluded-origin graveyard trigger should parse");

    let crate::model::ast::TriggerSpec::PutIntoGraveyardFromAnyExcept {
        filter,
        excluded: Zone::Battlefield,
        one_or_more: false,
    } = parsed
    else {
        panic!("expected an excluded-battlefield origin trigger, got {parsed:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.nontoken);

    let ordinary = tokenize_line("a creature card is put into a graveyard from anywhere", 0);
    assert!(matches!(
        crate::activation_and_restrictions::parse_trigger_clause_lexed(&ordinary,)
            .expect("ordinary any-origin trigger should still parse"),
        crate::model::ast::TriggerSpec::PutIntoGraveyard(_)
    ));
}

#[test]
fn parses_excess_noncombat_damage_recipient_as_a_passive_qualified_trigger() {
    let tokens = tokenize_line(
        "a creature or planeswalker an opponent controls is dealt excess noncombat damage",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("qualified passive damage trigger should parse");

    let crate::model::ast::TriggerSpec::IsDealtExcessNoncombatDamage(filter) = parsed else {
        panic!("expected a typed excess-noncombat dealt-damage trigger");
    };
    assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
    assert!(filter.card_types.contains(&CardType::Creature));
    assert!(filter.card_types.contains(&CardType::Planeswalker));
}

#[test]
fn parses_one_or_more_excess_recipients_as_a_grouped_trigger() {
    let tokens = tokenize_line(
        "one or more creatures your opponents control are dealt excess noncombat damage",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("grouped excess-damage trigger should parse");
    let crate::model::ast::TriggerSpec::IsDealtExcessNoncombatDamage(filter) = parsed else {
        panic!("expected a typed grouped excess-damage trigger");
    };
    assert!(filter.union_is_one_or_more());
    assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
    assert_eq!(filter.card_types, [CardType::Creature]);
}

#[test]
fn named_source_or_another_dies_stays_two_matcher_branches() {
    let tokens = tokenize_line("Blood Artist or another creature dies", 0);
    let context = crate::parse_context::ParseContext::for_fragment(
        "Blood Artist",
        vec![CardType::Creature],
        vec![Subtype::Vampire],
        "Blood Artist or another creature dies",
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("named source-or-another dies trigger should parse");

    let crate::model::ast::TriggerSpec::Either(source, other) = parsed else {
        panic!("expected distinct source and another-object branches");
    };
    assert!(
        matches!(*source, crate::model::ast::TriggerSpec::ThisDies),
        "expected canonical source-dies branch, got {source:#?}"
    );
    let crate::model::ast::TriggerSpec::Dies(other_filter) = *other else {
        panic!("expected another-creature dies branch");
    };

    assert!(!other_filter.source, "{other_filter:#?}");
    assert!(other_filter.other, "{other_filter:#?}");
    assert_eq!(other_filter.card_types, [CardType::Creature]);
}

#[test]
fn normalized_this_or_another_dies_stays_two_matcher_branches() {
    let tokens = tokenize_line("this or another creature you control dies", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("normalized source-or-another dies trigger should parse");

    let crate::model::ast::TriggerSpec::Either(source, other) = parsed else {
        panic!("expected distinct source and another-object branches: {parsed:#?}");
    };
    assert!(matches!(*source, crate::model::ast::TriggerSpec::ThisDies));
    let crate::model::ast::TriggerSpec::Dies(other_filter) = *other else {
        panic!("expected another-creature dies branch");
    };
    assert_eq!(other_filter.card_types, [CardType::Creature]);
    assert_eq!(other_filter.controller, Some(PlayerFilter::You));
}

#[test]
fn named_source_and_or_one_or_more_other_etb_keeps_both_event_arms() {
    for (name, subject, card_types, subtypes, expected_filter) in [
        (
            "Satoru, the Infiltrator",
            "Satoru and/or one or more other nontoken creatures you control enter",
            vec![CardType::Creature],
            vec![Subtype::Human, Subtype::Ninja, Subtype::Rogue],
            "one or more other nontoken creatures you control",
        ),
        (
            "Anje, Maid of Dishonor",
            "Anje and/or one or more other Vampires you control enter",
            vec![CardType::Creature],
            vec![Subtype::Vampire],
            "one or more other Vampires you control",
        ),
    ] {
        let tokens = tokenize_line(subject, 0);
        let context =
            crate::parse_context::ParseContext::for_fragment(name, card_types, subtypes, subject);
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
            context.view(),
            &tokens,
        )
        .expect("source-and/or-batched-other ETB trigger should parse");

        let crate::model::ast::TriggerSpec::Either(source, other) = &parsed else {
            panic!("expected distinct source and batched-other branches: {parsed:#?}");
        };
        assert!(matches!(
            source.as_ref(),
            crate::model::ast::TriggerSpec::ThisEntersBattlefieldWithSurface {
                subject_number: ironsmith_core::trigger_model::TriggerSubjectNumber::Singular,
                ..
            }
        ));
        let crate::model::ast::TriggerSpec::EntersBattlefieldOneOrMore { filter, .. } =
            other.as_ref()
        else {
            panic!("expected one-or-more ETB branch: {other:#?}");
        };
        assert!(filter.other, "{filter:#?}");
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(
            filter.union_connective(),
            crate::filter::ObjectFilterUnionConnective::AndOr
        );
        assert_eq!(
            crate::compile_support::compile_trigger_spec(parsed).display(),
            format!(
                "Whenever {} and/or {expected_filter} enter",
                name.split(',').next().unwrap()
            )
        );
    }
}

#[test]
fn parses_generic_sticker_trigger_with_source_recipient() {
    let tokens = tokenize_line("you put a sticker on this enchantment", 0);
    let context = crate::parse_context::ParseContext::for_fragment(
        "_____ Balls of Fire",
        vec![CardType::Enchantment],
        vec![],
        "you put a sticker on this enchantment",
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("generic sticker trigger should parse");

    let crate::model::ast::TriggerSpec::KeywordAction {
        action,
        player,
        source_filter: Some(source_filter),
        ..
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
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("typed sticker trigger should parse");

    let crate::model::ast::TriggerSpec::KeywordAction {
        action,
        player,
        source_filter: Some(source_filter),
        ..
    } = &parsed
    else {
        panic!("expected a keyword-action trigger, got {parsed:#?}");
    };
    assert_eq!(*action, crate::events::KeywordActionKind::AbilitySticker);
    assert_eq!(*player, PlayerFilter::Opponent);
    assert_eq!(source_filter.card_types, [CardType::Creature]);
}

#[test]
fn parses_each_of_your_main_phases_as_one_typed_either_phase_trigger() {
    let tokens = tokenize_line("the beginning of each of your main phases", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("each-main-phase trigger should parse");

    assert_eq!(
        parsed,
        crate::model::ast::TriggerSpec::BeginningOfMainPhase {
            player: PlayerFilter::You,
            surface: ironsmith_core::trigger_model::MainPhaseSurface::EachOfMainPhases,
        }
    );
}

#[test]
fn parses_split_possessive_unpaid_cumulative_upkeep_trigger() {
    let tokens = tokenize_line(
        "a player doesn't pay this enchantment's cumulative upkeep",
        0,
    );
    let context = crate::parse_context::ParseContext::for_fragment(
        "Heart of Bogardan",
        vec![CardType::Enchantment],
        vec![],
        "a player doesn't pay this enchantment's cumulative upkeep",
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("unpaid cumulative upkeep trigger should parse");

    assert_eq!(
        parsed,
        crate::model::ast::TriggerSpec::KeywordActionFromSource {
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
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("causal life-gain trigger should parse");

    let crate::model::ast::TriggerSpec::YouGainLifeCausedBy(source) = &parsed else {
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
        crate::compile_support::compile_trigger_spec(parsed).display(),
        "Whenever a white instant or sorcery spell causes you to gain life"
    );
}

#[test]
fn shared_spell_noun_damage_source_keeps_stack_controller_and_mana_facts() {
    let tokens = tokenize_line(
        "an instant or sorcery spell you control with mana value 3 or greater deals damage",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("shared-noun spell damage trigger should parse");

    let crate::model::ast::TriggerSpec::DealsDamage { source, .. } = &parsed else {
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

    let compiled = crate::compile_support::compile_trigger_spec(parsed);
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
fn source_or_spell_damage_subject_keeps_independent_filter_branches() {
    let tokens = tokenize_line(
        "this or an instant or sorcery spell you control deals damage to a player",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("source-or-spell damage trigger should parse");

    let crate::model::ast::TriggerSpec::DealsDamageToPlayer { source, player, .. } = &parsed else {
        panic!("expected filtered damage-to-player trigger, got {parsed:#?}");
    };
    assert_eq!(*player, PlayerFilter::Any);
    assert_eq!(source.any_of.len(), 2, "{source:#?}");
    assert!(source.any_of[0].source, "{source:#?}");
    assert_eq!(source.any_of[1].zone, Some(crate::Zone::Stack));
    assert_eq!(source.any_of[1].controller, Some(PlayerFilter::You));
    assert_eq!(
        source.any_of[1].stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert_eq!(
        source.any_of[1].card_types,
        [crate::CardType::Instant, crate::CardType::Sorcery],
        "{source:#?}"
    );
    assert!(
        !source.source,
        "the outer union must not require source identity"
    );
    assert_eq!(source.zone, None, "{source:#?}");
    assert_eq!(source.controller, None, "{source:#?}");
}

#[test]
fn parses_clash_and_win_as_the_winner_aware_trigger() {
    let tokens = tokenize_line("you clash and win", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("clash-and-win trigger should parse");

    assert_eq!(
        parsed,
        crate::model::ast::TriggerSpec::WinsClash {
            player: PlayerFilter::You,
            surface: ironsmith_core::ClashWinTriggerSurface::ClashAndWin,
        }
    );
    assert_eq!(
        crate::compile_support::compile_trigger_spec(parsed).display(),
        "Whenever you clash and win"
    );
}

#[test]
fn parses_passive_damage_by_qualified_source_union() {
    let tokens = tokenize_line(
        "an opponent is dealt damage by a red instant or sorcery spell you control or by a red planeswalker you control",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("passive qualified-source damage trigger should parse");

    let crate::model::ast::TriggerSpec::DealsDamageToPlayer {
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
        crate::compile_support::compile_trigger_spec(parsed).display(),
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
fn cast_or_land_entry_stays_two_trigger_branches() {
    let tokens = tokenize_line("you cast a white spell or a Plains you control enters", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("cast-or-entry trigger should parse");
    assert!(
        matches!(parsed, crate::model::ast::TriggerSpec::Either(_, _)),
        "{parsed:#?}"
    );
}

#[test]
fn shared_attack_or_block_subject_is_preserved_on_both_trigger_arms() {
    let tokens = tokenize_line("enchanted creature attacks or blocks", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("shared attack/block trigger should parse");

    let crate::model::ast::TriggerSpec::Either(left, right) = parsed else {
        panic!("expected an either trigger");
    };
    let (
        crate::model::ast::TriggerSpec::Attacks(attacks),
        crate::model::ast::TriggerSpec::Blocks(blocks),
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
fn source_and_another_attacking_different_players_is_typed() {
    let tokens = tokenize_line(
        "this creature and another creature attack different players",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("different-player attack relation should parse");
    assert!(matches!(
        parsed,
        crate::model::ast::TriggerSpec::ThisAndAnotherAttackDifferentPlayers
    ));
}

#[test]
fn source_attacks_player_with_most_life_keeps_the_attacked_player_filter() {
    let tokens = tokenize_line(
        "this creature attacks the player with the most life or tied for most life",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("most-life attack trigger should parse");
    let crate::model::ast::TriggerSpec::Attacks(filter) = parsed else {
        panic!("expected typed attacks trigger: {parsed:#?}");
    };
    assert!(filter.source);
    assert_eq!(
        filter.attacking_player_or_planeswalker_controlled_by,
        Some(crate::target::PlayerFilter::MostLifeTied)
    );
    assert_eq!(
        filter.targets_only_player,
        Some(crate::target::PlayerFilter::MostLifeTied)
    );
}

#[test]
fn shared_spell_cast_or_copy_subject_is_preserved_on_both_trigger_arms() {
    let tokens = tokenize_line(
        "enchanted player casts a spell other than the first spell they cast each turn or copies a spell",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("shared cast/copy trigger should parse");

    let crate::model::ast::TriggerSpec::Either(left, right) = parsed else {
        panic!("expected a cast-or-copy trigger union");
    };
    let (
        crate::model::ast::TriggerSpec::SpellCast {
            filter: Some(filter),
            caster,
            min_spells_this_turn,
            ..
        },
        crate::model::ast::TriggerSpec::SpellCopied { copier, .. },
    ) = (*left, *right)
    else {
        panic!("expected spell-cast and spell-copied trigger arms");
    };
    let enchanted = PlayerFilter::TaggedPlayer(crate::tag::CompilerReferenceTag::Enchanted.bind().into());
    assert_eq!(caster, enchanted);
    assert_eq!(copier, enchanted);
    assert_eq!(min_spells_this_turn, Some(2));
    assert!(
        !filter.other,
        "the ordinal exclusion belongs to the cast-count qualifier, not the spell object filter"
    );
}

#[test]
fn spell_other_than_your_first_uses_a_minimum_cast_count_without_other_object_filter() {
    let tokens = tokenize_line("you cast a spell other than your first spell each turn", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("minimum spell-count trigger should parse");

    let crate::model::ast::TriggerSpec::SpellCast {
        filter: Some(filter),
        caster,
        min_spells_this_turn,
        exact_spells_this_turn,
        ..
    } = parsed
    else {
        panic!("expected qualified spell-cast trigger: {parsed:#?}");
    };
    assert_eq!(caster, PlayerFilter::You);
    assert_eq!(min_spells_this_turn, Some(2));
    assert_eq!(exact_spells_this_turn, None);
    assert!(!filter.other);
}

#[test]
fn spell_from_their_hand_uses_the_caster_not_an_owner_constraint() {
    let tokens = tokenize_line("a player casts a spell from their hand", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("hand-origin cast trigger should parse");
    let crate::model::ast::TriggerSpec::SpellCast {
        filter: Some(filter),
        caster: PlayerFilter::Any,
        ..
    } = parsed
    else {
        panic!("expected any-player hand-origin cast trigger: {parsed:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(filter.owner, None);
}

#[test]
fn targets_only_relation_preserves_named_source_exclusion() {
    let tokens = tokenize_line(
        "a player casts a spell that targets only a single creature other than Ivy",
        0,
    );
    let context = crate::parse_context::ParseContext::for_fragment(
        "Ivy, Gleeful Spellthief",
        vec![CardType::Creature],
        vec![Subtype::Faerie, Subtype::Rogue],
        "a player casts a spell that targets only a single creature other than Ivy",
    );
    let normalized =
        crate::util::normalize_source_reference_tokens_with_context(context.view(), &tokens)
            .expect("source exclusion should normalize");
    assert_eq!(
        crate::lexer::render_token_slice(&normalized),
        "a player casts a spell that targets only a single creature other than this creature"
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("source-excluding single-target spell trigger should parse");

    let crate::model::ast::TriggerSpec::SpellCast {
        filter: Some(spell),
        caster: PlayerFilter::Any,
        ..
    } = parsed
    else {
        panic!("expected a player spell-cast trigger: {parsed:#?}");
    };
    assert_eq!(
        spell.target_count,
        Some(crate::effect::ChoiceCount::exactly(1))
    );
    let target = spell
        .targets_only_object
        .as_deref()
        .expect("trigger should retain its sole creature target class");
    assert_eq!(target.card_types, [CardType::Creature]);
    assert!(
        target.other,
        "source identity must be excluded: {target:#?}"
    );
    assert_eq!(
        target.source_surface,
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string()
        ))
    );
    assert_eq!(context.source().card_name, "Ivy, Gleeful Spellthief");
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
fn parses_numbered_counter_placement_as_an_ordinal_trigger() {
    let tokens = tokenize_line("the fourth plan counter is put on this enchantment", 0);
    let context = crate::parse_context::ParseContext::for_fragment(
        "Plan Probe",
        vec![CardType::Enchantment],
        vec![],
        "the fourth plan counter is put on this enchantment",
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("ordinal counter trigger should parse");

    let crate::model::ast::TriggerSpec::NthCounterPutOn {
        filter,
        counter_type,
        counter_number,
    } = parsed
    else {
        panic!("expected a numbered counter trigger, got {parsed:#?}");
    };
    assert!(filter.source);
    assert_eq!(counter_type, CounterType::Named("plan".into()));
    assert_eq!(counter_number, 4);
}

#[test]
fn parses_last_named_counter_removed_from_typed_source() {
    let tokens = tokenize_line("the last ore counter is removed from this Aura", 0);
    let context = crate::parse_context::ParseContext::for_fragment(
        "Mine Probe",
        vec![CardType::Enchantment],
        vec![crate::types::Subtype::Aura],
        "the last ore counter is removed from this Aura",
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("last named counter trigger should parse");
    let crate::model::ast::TriggerSpec::CounterRemovedFrom {
        filter,
        counter_type,
        last,
        caused_by_source,
        ..
    } = parsed
    else {
        panic!("expected a typed counter-removal trigger: {parsed:#?}");
    };
    assert!(filter.source);
    assert_eq!(counter_type, Some(CounterType::Named("ore".into())));
    assert!(last);
    assert!(!caused_by_source);
}

#[test]
fn parses_grouped_loyalty_counters_removed_from_named_source() {
    let tokens = tokenize_line("one or more loyalty counters are removed from Chandra", 0);
    let context = crate::parse_context::ParseContext::for_fragment(
        "Chandra, Fire Artisan",
        vec![CardType::Planeswalker],
        vec![crate::types::Subtype::Chandra],
        "one or more loyalty counters are removed from Chandra",
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("named grouped loyalty-removal trigger should parse");
    let crate::model::ast::TriggerSpec::CounterRemovedFrom {
        filter,
        counter_type,
        last,
        one_or_more,
        caused_by_source,
    } = parsed
    else {
        panic!("expected a typed counter-removal trigger: {parsed:#?}");
    };
    assert!(filter.source);
    assert_eq!(counter_type, Some(CounterType::Loyalty));
    assert!(!last);
    assert!(one_or_more);
    assert!(!caused_by_source);

    let near_miss = tokenize_line(
        "one or more loyalty counters are removed from a planeswalker",
        0,
    );
    assert!(!matches!(
        crate::activation_and_restrictions::parse_trigger_clause_lexed(&near_miss),
        Ok(crate::model::ast::TriggerSpec::CounterRemovedFrom { .. })
    ));
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
    assert!(!parsed.chosen_type_reference);

    let chosen_type = tokenize_line("an ability of a planeswalker of that type", 0);
    let parsed = parse_ability_of_object_tail(&chosen_type).unwrap();
    assert_eq!(parsed.filter, 3..5);
    assert!(!parsed.non_mana_only);
    assert!(parsed.chosen_type_reference);
}

#[test]
fn named_activated_ability_reaches_the_typed_trigger_ast() {
    let tokens = tokenize_line("you activate a ninjutsu ability", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("named activated-ability trigger should parse");
    let crate::model::ast::TriggerSpec::AbilityActivated {
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
fn chosen_subtype_activated_ability_reaches_the_typed_trigger_filter() {
    let tokens = tokenize_line("you activate an ability of a planeswalker of that type", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("chosen-subtype activated-ability trigger should parse");
    let crate::model::ast::TriggerSpec::AbilityActivated {
        filter,
        loyalty_only,
        ..
    } = parsed
    else {
        panic!("expected an activated-ability trigger");
    };

    assert_eq!(filter.card_types, [CardType::Planeswalker]);
    assert!(filter.chosen_creature_type);
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
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();
    assert_eq!(
        parsed,
        crate::model::ast::TriggerSpec::PlayerRollsDie {
            player: PlayerFilter::You,
            one_or_more: true,
        }
    );
}

#[test]
fn grouped_opponent_life_loss_reaches_the_typed_trigger_ast() {
    let tokens = tokenize_line("one or more opponents lose life", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();
    assert_eq!(
        parsed,
        crate::model::ast::TriggerSpec::PlayersLoseLifeOneOrMore(PlayerFilter::Opponent,)
    );
    assert_eq!(
        crate::compile_support::compile_trigger_spec(parsed),
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
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .unwrap_or_else(|err| panic!("{text}: {err:?}"));
        let crate::model::ast::TriggerSpec::CounterRemovedFrom {
            filter,
            one_or_more,
            caused_by_source,
            ..
        } = &parsed
        else {
            panic!("{text}: expected CounterRemovedFrom, got {parsed:?}");
        };
        assert!(filter.source, "{text}");
        assert_eq!(*one_or_more, expected_one_or_more, "{text}");
        assert!(*caused_by_source, "{text}");

        let lowered = crate::compile_support::compile_trigger_spec(parsed);
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
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();

    assert_eq!(
        parsed,
        crate::model::ast::TriggerSpec::AbilityTriggered {
            another: true,
            source_filter: None,
            caused_by_source_entering: false,
        }
    );
    let lowered = crate::compile_support::compile_trigger_spec(parsed);
    assert_eq!(lowered.display(), "Whenever another ability triggers");
    assert!(matches!(
        lowered.kind,
        ironsmith_core::trigger_model::TriggerKind::AbilityTriggered {
            another: true,
            source_filter: None,
            caused_by_source_entering: false,
        }
    ));
}

#[test]
fn ability_triggered_by_its_own_sources_entry_reaches_typed_model() {
    let tokens = tokenize_line(
        "a creature entering under an opponent's control causes a triggered ability of that creature to trigger",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("qualified ability-trigger clause should parse");

    let crate::model::ast::TriggerSpec::AbilityTriggered {
        another,
        source_filter: Some(filter),
        caused_by_source_entering,
    } = parsed
    else {
        panic!("expected a typed source-qualified ability trigger, got {parsed:#?}");
    };
    assert!(!another);
    assert!(caused_by_source_entering);
    assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
    assert_eq!(
        filter.controller,
        Some(crate::target::PlayerFilter::Opponent)
    );
}

#[test]
fn extraordinary_journey_keeps_exile_entry_or_cast_provenance() {
    let tokens = tokenize_line(
        "one or more nontoken creatures enter, if one or more of them entered from exile or was cast from exile",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();
    assert!(matches!(
        &parsed,
        crate::model::ast::TriggerSpec::EntersBattlefieldOneOrMore {
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
        crate::compile_support::compile_trigger_spec(parsed).display(),
        "Whenever one or more nontoken creatures enter the battlefield, if one or more of them entered from exile or was cast from exile"
    );
}

#[test]
fn generic_etb_during_your_turn_keeps_the_active_turn_qualifier() {
    let tokens = tokenize_line("a land enters during your turn", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("qualified ETB trigger should parse");
    let crate::model::ast::TriggerSpec::EntersBattlefield {
        filter,
        during_turn,
        ..
    } = &parsed
    else {
        panic!("expected an ETB trigger, got {parsed:?}");
    };
    assert_eq!(filter.card_types, vec![crate::types::CardType::Land]);
    assert_eq!(during_turn, &Some(crate::target::PlayerFilter::You));

    let compiled = crate::compile_support::compile_trigger_spec(parsed);
    let crate::triggers::TriggerKind::ZoneChange(zone_change) = &compiled.kind else {
        panic!("expected an executable zone-change trigger, got {compiled:?}");
    };
    assert_eq!(
        zone_change.during_turn,
        Some(crate::target::PlayerFilter::You)
    );
    assert_eq!(
        compiled.display(),
        "Whenever a land enters the battlefield during your turn"
    );
}

#[test]
fn player_puts_etb_keeps_causative_surface_and_mixed_union_semantics() {
    for (subject, expected) in [
        (
            "a nontoken creature",
            "Whenever a player puts a nontoken creature onto the battlefield",
        ),
        (
            "an Island or blue permanent",
            "Whenever a player puts an Island or blue permanent onto the battlefield",
        ),
    ] {
        let text = format!("a player puts {subject} onto the battlefield");
        let tokens = tokenize_line(&text, 0);
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .expect("causative ETB trigger should parse");
        let crate::model::ast::TriggerSpec::EntersBattlefield { filter, .. } = &parsed else {
            panic!("expected an ETB trigger, got {parsed:#?}");
        };
        assert!(filter.has_player_puts_onto_battlefield_surface());
        if subject.contains(" or ") {
            assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        }
        assert_eq!(
            crate::compile_support::compile_trigger_spec(parsed).display(),
            expected
        );
    }
}

#[test]
fn parses_and_or_player_object_damage_recipients_as_both_trigger_branches() {
    let tokens = tokenize_line(
        "a red source you control deals damage to one or more permanents and/or players",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();
    assert_eq!(
        crate::compile_support::compile_trigger_spec(parsed.clone()).display(),
        "Whenever a red source you control deals damage to one or more permanents and/or players"
    );

    let crate::model::ast::TriggerSpec::Either(object, player) = parsed else {
        panic!("expected player/object damage union");
    };
    let crate::model::ast::TriggerSpec::DealsDamageTo {
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
    let crate::model::ast::TriggerSpec::DealsDamageToPlayer {
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
fn exact_damage_to_permanent_or_player_stays_one_correlated_trigger() {
    let tokens = tokenize_line(
        "another source you control deals exactly 1 damage to a permanent or player",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("exact object-or-player damage trigger should parse");

    let crate::model::ast::TriggerSpec::DealsExactDamageToObjectOrPlayer {
        source,
        object,
        player,
        player_first,
        amount,
        source_surface,
    } = parsed.clone()
    else {
        panic!("expected one correlated exact-damage trigger, got {parsed:?}");
    };
    assert!(source.other);
    assert_eq!(source.controller, Some(PlayerFilter::You));
    assert_eq!(source.zone, None);
    assert_eq!(object.description(), "permanent");
    assert_eq!(player, PlayerFilter::Any);
    assert!(!player_first);
    assert_eq!(amount, 1);
    assert_eq!(source_surface, crate::triggers::DamageSourceSurface::Source);

    let compiled = crate::compile_support::compile_trigger_spec(parsed);
    assert_eq!(
        compiled.display(),
        "Whenever another source you control deals exactly 1 damage to a permanent or player"
    );
    assert!(matches!(
        compiled.kind,
        crate::triggers::TriggerKind::DealsExactDamageToObjectOrPlayer {
            amount: 1,
            player_first: false,
            ..
        }
    ));

    let non_exact = tokenize_line(
        "another source you control deals 1 damage to a permanent or player",
        0,
    );
    let non_exact = crate::activation_and_restrictions::parse_trigger_clause_lexed(&non_exact)
        .expect("ordinary object-or-player damage trigger should retain its existing route");
    assert!(
        matches!(non_exact, crate::model::ast::TriggerSpec::Either(_, _)),
        "a bare numeric amount must not acquire the authored `exactly` surface: {non_exact:?}"
    );
}

#[test]
fn passive_sacrifice_or_destroy_keeps_one_typed_event_union() {
    let tokens = tokenize_line(
        "Whenever a noncreature artifact is sacrificed or destroyed",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("passive sacrifice-or-destroy trigger should parse");
    let crate::model::ast::TriggerSpec::WithIntro { trigger, .. } = parsed else {
        panic!("expected preserved trigger intro");
    };
    let crate::model::ast::TriggerSpec::AnyOf(branches) = *trigger else {
        panic!("expected a typed event union");
    };
    let [
        crate::model::ast::TriggerSpec::PermanentSacrificed(sacrificed),
        crate::model::ast::TriggerSpec::PermanentDestroyed(destroyed),
    ] = branches.as_slice()
    else {
        panic!("expected passive sacrifice and destroy branches: {branches:#?}");
    };
    assert_eq!(sacrificed, destroyed);
    assert_eq!(sacrificed.card_types, [crate::types::CardType::Artifact]);
    assert_eq!(
        sacrificed.excluded_card_types,
        [crate::types::CardType::Creature]
    );
}

#[test]
fn generic_source_to_player_keeps_authored_surface_in_ast_and_model() {
    let tokens = tokenize_line("a source an opponent controls deals damage to you", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();

    let crate::model::ast::TriggerSpec::DealsDamageToPlayer {
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

    let compiled = crate::compile_support::compile_trigger_spec(parsed);
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
fn grouped_noncombat_damage_to_opponents_during_your_turn_keeps_each_qualifier() {
    let tokens = tokenize_line(
        "a source you control deals noncombat damage to one or more of your opponents during your turn",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("qualified noncombat-damage trigger should parse");

    let crate::model::ast::TriggerSpec::DealsNoncombatDamageToPlayer {
        source,
        player,
        source_surface,
        damaged_player_one_or_more,
        during_turn,
    } = parsed.clone()
    else {
        panic!("expected qualified noncombat-damage trigger, got {parsed:#?}");
    };
    assert_eq!(source.controller, Some(PlayerFilter::You));
    assert_eq!(source.zone, None);
    assert_eq!(player, PlayerFilter::Opponent);
    assert_eq!(source_surface, crate::triggers::DamageSourceSurface::Source);
    assert!(damaged_player_one_or_more);
    assert_eq!(during_turn, Some(PlayerFilter::You));

    let compiled = crate::compile_support::compile_trigger_spec(parsed);
    assert_eq!(
        compiled.display(),
        "Whenever a source you control deals noncombat damage to one or more of your opponents during your turn"
    );
    assert!(matches!(
        compiled.kind,
        crate::triggers::TriggerKind::DealsNoncombatDamageToPlayer {
            damaged_player_one_or_more: true,
            during_turn: Some(PlayerFilter::You),
            ..
        }
    ));

    let unqualified = tokenize_line(
        "a source you control deals noncombat damage to an opponent",
        0,
    );
    let unqualified = crate::activation_and_restrictions::parse_trigger_clause_lexed(&unqualified)
        .expect("ordinary noncombat-damage trigger should retain its existing route");
    assert!(matches!(
        unqualified,
        crate::model::ast::TriggerSpec::DealsNoncombatDamageToPlayer {
            damaged_player_one_or_more: false,
            during_turn: None,
            ..
        }
    ));
}

#[test]
fn passive_player_combat_damage_keeps_the_combat_qualifier() {
    for (clause, expected_player) in [
        ("you're dealt combat damage", PlayerFilter::You),
        ("an opponent is dealt combat damage", PlayerFilter::Opponent),
    ] {
        let tokens = tokenize_line(clause, 0);
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .expect("passive player combat-damage trigger should parse");

        let crate::model::ast::TriggerSpec::DealsCombatDamageToPlayer { source, player } =
            parsed.clone()
        else {
            panic!("expected combat-damage-to-player trigger for {clause:?}, got {parsed:#?}");
        };
        assert_eq!(source, ObjectFilter::default());
        assert_eq!(player, expected_player);
    }
}

#[test]
fn one_or_more_damage_cardinality_survives_typed_lowering() {
    let arashin = tokenize_line(
        "this creature deals combat damage to one or more blocking creatures",
        0,
    );
    let arashin = crate::activation_and_restrictions::parse_trigger_clause_lexed(&arashin)
        .expect("Arashin-style grouped damage recipient should parse");
    let crate::model::ast::TriggerSpec::ThisDealsCombatDamageTo(arashin_target) = &arashin else {
        panic!("expected source combat-damage recipient trigger, got {arashin:?}");
    };
    assert!(arashin_target.union_is_one_or_more());

    let briarbridge = tokenize_line("this creature deals damage to one or more creatures", 0);
    let briarbridge = crate::activation_and_restrictions::parse_trigger_clause_lexed(&briarbridge)
        .expect("Briarbridge-style grouped damage recipient should parse");
    let crate::model::ast::TriggerSpec::ThisDealsDamageTo(briarbridge_target) = &briarbridge else {
        panic!("expected source damage recipient trigger, got {briarbridge:?}");
    };
    assert!(briarbridge_target.union_is_one_or_more());

    let thing = tokenize_line("one or more Heroes you control deal damage to a player", 0);
    let thing = crate::activation_and_restrictions::parse_trigger_clause_lexed(&thing)
        .expect("Thing-style grouped damage source should parse");
    let crate::model::ast::TriggerSpec::DealsDamageToPlayer { source, player, .. } = &thing else {
        panic!("expected grouped source damage-to-player trigger, got {thing:?}");
    };
    assert!(source.union_is_one_or_more());
    assert_eq!(*player, PlayerFilter::Any);
}

#[test]
fn noncreature_source_without_recipient_is_zone_less_in_ast_and_model() {
    let tokens = tokenize_line("a noncreature source you control deals damage", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();

    let crate::model::ast::TriggerSpec::DealsDamage {
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

    let compiled = crate::compile_support::compile_trigger_spec(parsed);
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
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();

    let crate::model::ast::TriggerSpec::Either(blocks, becomes_blocked) = parsed else {
        panic!("expected blocks/becomes-blocked trigger union");
    };
    let crate::model::ast::TriggerSpec::ThisBlocksObject {
        filter: blocker,
        min_blocked_objects,
    } = *blocks
    else {
        panic!("expected filtered blocks branch");
    };
    let crate::model::ast::TriggerSpec::ThisBecomesBlockedByObject(blocking) = *becomes_blocked
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
    let becomes = crate::activation_and_restrictions::parse_trigger_clause_lexed(&becomes_tokens)
        .expect("relative-power becomes-blocked trigger should parse");
    let crate::model::ast::TriggerSpec::BecomesBlockedByObjectWithLesserPower { blocked, blocker } =
        becomes
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
    let blocks = crate::activation_and_restrictions::parse_trigger_clause_lexed(&blocks_tokens)
        .expect("relative-power blocks trigger should parse");
    let crate::model::ast::TriggerSpec::BlocksObjectWithLesserPower { blocker, blocked } = blocks
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
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("minimum-cardinality block trigger should parse");
    let crate::model::ast::TriggerSpec::ThisBlocksObject {
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
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .unwrap_or_else(|error| panic!("failed to parse {clause:?}: {error}"));

        let crate::model::ast::TriggerSpec::AttacksOneOrMoreWithMinTotal {
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
fn attack_group_total_power_is_not_a_per_attacker_filter() {
    let tokens = tokenize_line(
        "you attack with creatures with total power 12 or greater",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("aggregate-power attack trigger should parse");
    let crate::model::ast::TriggerSpec::AttacksOneOrMoreWithAggregate {
        filter,
        metric,
        comparison,
    } = parsed
    else {
        panic!("expected aggregate-power attack trigger, got {parsed:#?}");
    };
    assert_eq!(metric, crate::effect::ChoiceAggregateMetric::Power);
    assert_eq!(
        comparison,
        crate::filter::Comparison::GreaterThanOrEqual(12)
    );
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.power.is_none());
}

#[test]
fn serial_shared_opponent_attack_draw_cast_union_keeps_all_three_events() {
    let tokens = tokenize_line(
        "an opponent attacks you with two or more creatures, draws their second card each turn, or casts their second spell each turn",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("serial shared-opponent trigger union should parse");
    let crate::model::ast::TriggerSpec::AnyOf(branches) = parsed else {
        panic!("expected three independent trigger branches, got {parsed:#?}");
    };
    let [attack, draw, cast] = branches.as_slice() else {
        panic!("expected exactly three trigger branches: {branches:#?}");
    };
    let crate::model::ast::TriggerSpec::AttacksOneOrMoreWithMinTotal {
        filter,
        min_total_attackers,
    } = attack
    else {
        panic!("expected a minimum-cardinality attack branch: {attack:#?}");
    };
    assert_eq!(*min_total_attackers, 2);
    assert_eq!(filter.controller.as_ref(), Some(&PlayerFilter::Opponent));
    assert_eq!(
        filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref(),
        Some(&PlayerFilter::You)
    );
    assert_eq!(
        filter.targets_only_player.as_ref(),
        Some(&PlayerFilter::You)
    );
    assert!(matches!(
        draw,
        crate::model::ast::TriggerSpec::PlayerDrawsNthCardEachTurn {
            player: PlayerFilter::Opponent,
            card_number: 2,
        }
    ));
    assert!(matches!(
        cast,
        crate::model::ast::TriggerSpec::SpellCast {
            caster: PlayerFilter::Opponent,
            exact_spells_this_turn: Some(2),
            ..
        }
    ));
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
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .unwrap_or_else(|error| panic!("failed to parse {clause:?}: {error}"));

        let filter = match &parsed {
            crate::model::ast::TriggerSpec::Attacks(filter) if !expected_one_or_more => filter,
            crate::model::ast::TriggerSpec::AttacksOneOrMore(filter) if expected_one_or_more => {
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
fn player_attacking_planeswalkers_keeps_planeswalker_only_target_semantics() {
    let tokens = tokenize_line(
        "an opponent attacks one or more planeswalkers you control",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("planeswalker-only player attack trigger should parse");

    let crate::model::ast::TriggerSpec::PlayerAttacksOneOrMore { attacker, target } =
        parsed.clone()
    else {
        panic!("expected typed player-attack trigger, got {parsed:#?}");
    };
    assert_eq!(attacker, PlayerFilter::Opponent);
    assert_eq!(
        target,
        ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(PlayerFilter::You)
    );
    let lowered = crate::compile_support::compile_trigger_spec(parsed);
    assert!(matches!(
        lowered.kind,
        crate::triggers::TriggerKind::PlayerAttacksOneOrMore {
            attacker: PlayerFilter::Opponent,
            target: ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(
                PlayerFilter::You
            ),
        }
    ));
}

#[test]
fn player_attacking_one_planeswalker_with_a_group_keeps_per_defender_semantics() {
    let tokens = tokenize_line(
        "an opponent attacks a planeswalker you control with one or more creatures",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("per-defender player attack trigger should parse");

    let crate::model::ast::TriggerSpec::PlayerAttacksTargetWithOneOrMore { attacker, target } =
        parsed.clone()
    else {
        panic!("expected typed per-defender player-attack trigger, got {parsed:#?}");
    };
    assert_eq!(attacker, PlayerFilter::Opponent);
    assert_eq!(
        target,
        ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(PlayerFilter::You)
    );
    assert_eq!(
        crate::compile_support::inferred_trigger_player_filter(&parsed),
        Some(PlayerFilter::AliasedControllerOf(
            crate::target::ObjectRef::tagged(crate::tag::CompilerReferenceTag::Triggering.bind())
        ))
    );

    let lowered = crate::compile_support::compile_trigger_spec(parsed);
    assert!(matches!(
        lowered.kind,
        crate::triggers::TriggerKind::PlayerAttacksTargetWithOneOrMore {
            attacker: PlayerFilter::Opponent,
            target: ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(
                PlayerFilter::You
            ),
        }
    ));
}

#[test]
fn initiative_holder_attack_target_keeps_dynamic_player_reference() {
    let tokens = tokenize_line("you attack the player who has the initiative", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("initiative-holder attack trigger should parse");

    let crate::model::ast::TriggerSpec::AttacksOneOrMore(filter) = &parsed else {
        panic!("expected a group attack trigger, got {parsed:#?}");
    };
    let expected =
        PlayerFilter::TaggedPlayer(crate::tag::CompilerReferenceTag::InitiativeHolder.bind().into());
    assert_eq!(
        filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref(),
        Some(&expected)
    );
    assert_eq!(filter.targets_only_player.as_ref(), Some(&expected));
}

#[test]
fn monarch_end_step_is_an_event_qualified_trigger_surface() {
    let tokens = tokenize_line("at the beginning of the monarch's end step", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("monarch end-step trigger should parse");
    assert!(matches!(
        parsed,
        crate::model::ast::TriggerSpec::BeginningOfMonarchEndStep
    ));
}

#[test]
fn normalized_monarch_end_step_is_still_event_qualified() {
    for text in [
        "at the beginning of the monarch end step",
        "at beginning of monarch end step",
        "the beginning of the monarch's end step",
        "the beginning of the monarch end step",
        "beginning of monarch end step",
    ] {
        let tokens = tokenize_line(text, 0);
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .expect("normalized monarch end-step trigger should parse");
        assert!(
            matches!(
                parsed,
                crate::model::ast::TriggerSpec::BeginningOfMonarchEndStep
            ),
            "{text}: {parsed:#?}"
        );
    }
}

#[test]
fn repeated_attack_intro_resolves_enchanted_player_pronoun_on_both_branches() {
    fn without_intro(trigger: &crate::model::ast::TriggerSpec) -> &crate::model::ast::TriggerSpec {
        match trigger {
            crate::model::ast::TriggerSpec::WithIntro { trigger, .. } => without_intro(trigger),
            trigger => trigger,
        }
    }

    let tokens = tokenize_line(
        "when you attack enchanted opponent or a planeswalker they control or when they attack you or a planeswalker you control",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("repeated-intro attack union should parse");
    let crate::model::ast::TriggerSpec::Either(left, right) = &parsed else {
        panic!("expected a two-branch attack trigger, got {parsed:#?}");
    };
    let crate::model::ast::TriggerSpec::AttacksOneOrMore(left_filter) = without_intro(left) else {
        panic!("expected controller attack branch, got {left:#?}");
    };
    let crate::model::ast::TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(right_filter) =
        without_intro(right)
    else {
        panic!("expected enchanted-player counterattack branch, got {right:#?}");
    };
    let enchanted = PlayerFilter::TaggedPlayer(crate::tag::CompilerReferenceTag::Enchanted.bind().into());
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
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("subject-first attack group should parse");

    let crate::model::ast::TriggerSpec::AttacksOneOrMore(filter) = &parsed else {
        panic!("expected one-or-more attack trigger, got {parsed:?}");
    };
    assert!(filter.suspected);
    assert!(
        !filter.union_is_one_or_more(),
        "the union flag is reserved for explicit 'attack with one or more' surface"
    );
}

#[test]
fn repeated_intro_player_attack_with_group_keeps_both_trigger_branches() {
    let tokens = tokenize_line(
        "Whenever you attack with one or more Gods and whenever a God dies",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("repeated-intro group attack should parse");
    let crate::model::ast::TriggerSpec::Either(left, right) = &parsed else {
        panic!("expected two executable trigger branches, got {parsed:#?}");
    };
    let crate::model::ast::TriggerSpec::WithIntro { trigger: left, .. } = left.as_ref() else {
        panic!("left branch should retain its authored intro: {left:#?}");
    };
    let crate::model::ast::TriggerSpec::AttacksOneOrMore(filter) = left.as_ref() else {
        panic!("expected group attack branch, got {left:#?}");
    };
    assert_eq!(filter.controller.as_ref(), Some(&PlayerFilter::You));
    assert_eq!(filter.subtypes, vec![crate::types::Subtype::God]);
    assert!(filter.union_is_one_or_more());
    assert!(matches!(
        right.as_ref(),
        crate::model::ast::TriggerSpec::WithIntro { .. }
    ));

    assert_eq!(
        crate::compile_support::compile_trigger_spec(parsed).display(),
        "Whenever you attack with one or more Gods and whenever a God dies"
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
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
            .unwrap_or_else(|error| panic!("failed to parse {clause:?}: {error}"));
        assert_eq!(
            crate::compile_support::compile_trigger_spec(parsed).display(),
            expected,
            "unexpected compiled trigger surface for {clause:?}"
        );
    }
}

#[test]
fn chosen_object_leaves_keeps_the_persistent_choice_tag() {
    let tokens = tokenize_line("the chosen creature leaves the battlefield", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("chosen-object leave trigger should parse");
    let crate::model::ast::TriggerSpec::LeavesBattlefield(filter) = parsed else {
        panic!("expected a filtered leaves-battlefield trigger, got {parsed:#?}");
    };

    assert!(
        filter.card_types.contains(&CardType::Creature),
        "{filter:#?}"
    );
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }),
        "{filter:#?}"
    );
}

#[test]
fn leaves_without_dying_keeps_excluded_destination_and_batch_count() {
    let tokens = tokenize_line(
        "one or more other creatures you control leave the battlefield without dying",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("destination-excluding leave trigger should parse");
    let crate::model::ast::TriggerSpec::LeavesBattlefieldWithoutDying {
        filter,
        one_or_more,
    } = &parsed
    else {
        panic!("expected a destination-excluding leaves trigger, got {parsed:#?}");
    };
    assert!(*one_or_more);
    assert!(filter.other);
    assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
    assert_eq!(filter.card_types, vec![CardType::Creature]);

    let compiled = crate::compile_support::compile_trigger_spec(parsed);
    let crate::triggers::TriggerKind::ZoneChange(zone_change) = &compiled.kind else {
        panic!("expected a typed zone-change trigger, got {compiled:#?}");
    };
    assert_eq!(zone_change.from, Some(crate::zone::Zone::Battlefield));
    assert_eq!(zone_change.to, None);
    assert_eq!(zone_change.to_excluded, Some(crate::zone::Zone::Graveyard));
    assert_eq!(zone_change.count, crate::triggers::CountMode::OneOrMore);
}

#[test]
fn combat_damage_recipients_resolve_registered_source_names_before_subtypes() {
    let context = crate::parse_context::ParseContext::for_fragment(
        "Vraska the Unseen",
        vec![CardType::Planeswalker],
        vec![],
        "a creature deals combat damage to Vraska",
    );
    for recipient in ["Vraska the Unseen", "Vraska"] {
        let tokens = tokenize_line(&format!("a creature deals combat damage to {recipient}"), 0);
        let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
            context.view(),
            &tokens,
        )
        .unwrap_or_else(|error| panic!("failed to parse {recipient:?}: {error}"));
        let crate::model::ast::TriggerSpec::DealsCombatDamageTo { target, .. } = parsed else {
            panic!("expected combat-damage-to trigger for {recipient:?}: {parsed:#?}");
        };

        assert!(target.source, "{recipient:?} must resolve to the source");
        assert_eq!(
            target.source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this planeswalker".to_string(),
            ))
        );
        assert!(
            target.subtypes.is_empty(),
            "{recipient:?} must not fall through to subtype parsing: {target:#?}"
        );
    }
    assert_eq!(context.source().card_name, "Vraska the Unseen");
}

#[test]
fn named_source_attack_trigger_uses_explicit_context_identity() {
    let context = crate::parse_context::ParseContext::for_fragment(
        "Altaïr Ibn-La'Ahad",
        vec![CardType::Creature],
        vec![],
        "Altaïr attacks",
    );
    let tokens = tokenize_line("Altaïr attacks", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("named-source attack trigger should parse");
    let crate::model::ast::TriggerSpec::Attacks(filter) = parsed else {
        panic!("expected surfaced source attack trigger, got {parsed:#?}");
    };
    assert!(filter.source, "{filter:#?}");
    assert_eq!(
        filter.source_surface,
        Some(crate::target::SourceReferenceSurface::ShortName(
            "Altaïr".to_string()
        ))
    );

    let ordinary = tokenize_line("this creature attacks", 0);
    assert!(matches!(
        crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
            context.view(),
            &ordinary,
        )
        .expect("ordinary source attack should parse"),
        crate::model::ast::TriggerSpec::ThisAttacks
    ));
    assert_eq!(context.source().card_name, "Altaïr Ibn-La'Ahad");
}

#[test]
fn named_source_dies_trigger_keeps_source_identity_and_authored_surface() {
    let context = crate::parse_context::ParseContext::for_fragment(
        "Old-Growth Troll",
        vec![CardType::Creature],
        vec![Subtype::Troll, Subtype::Warrior],
        "Old-Growth Troll dies",
    );
    let tokens = tokenize_line("Old-Growth Troll dies", 0);
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("named-source death trigger should parse");
    let crate::model::ast::TriggerSpec::Dies(filter) = parsed else {
        panic!("expected a surfaced source death trigger, got {parsed:#?}");
    };
    assert!(filter.source, "{filter:#?}");
    assert_eq!(
        filter.source_surface,
        Some(crate::target::SourceReferenceSurface::FullName(
            "Old-Growth Troll".to_string()
        ))
    );

    let ordinary = tokenize_line("this creature dies", 0);
    assert!(matches!(
        crate::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
            context.view(),
            &ordinary,
        )
        .expect("ordinary source death trigger should parse"),
        crate::model::ast::TriggerSpec::ThisDies
    ));
}

#[test]
fn grist_style_singular_origin_clause_scopes_zone_owner_and_caster_on_the_trigger() {
    let tokens = tokenize_line(
        "this creature or another creature you control enters, if it entered from your graveyard or you cast it from your graveyard",
        0,
    );
    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens).unwrap();

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

    let crate::model::ast::TriggerSpec::Either(left, right) = &parsed else {
        panic!("expected an either-union of this-enters and another-enters, got {parsed:?}");
    };
    match left.as_ref() {
        crate::model::ast::TriggerSpec::ThisEntersBattlefield { origin_condition }
        | crate::model::ast::TriggerSpec::ThisEntersBattlefieldWithSurface {
            origin_condition,
            ..
        } => assert_origin(origin_condition, "this-enters branch"),
        other => panic!("expected a this-enters branch, got {other:?}"),
    }
    match right.as_ref() {
        crate::model::ast::TriggerSpec::EntersBattlefield {
            origin_condition, ..
        } => assert_origin(origin_condition, "another-enters branch"),
        other => panic!("expected an enters-battlefield branch, got {other:?}"),
    }
}
