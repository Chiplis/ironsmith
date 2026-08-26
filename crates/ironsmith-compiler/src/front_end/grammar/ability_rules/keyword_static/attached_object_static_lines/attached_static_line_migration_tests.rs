use super::*;

#[test]
fn display_keeps_adjacent_mana_symbols_compact() {
    let tokens = crate::lexer::lex_line("t {w}, {u}", 0).unwrap();
    assert_eq!(display_text_for_tokens(&tokens, false), "{T}{W}, {U}");
}

#[test]
fn otherwise_keyword_grant_uses_typed_clause_boundary() {
    let tokens = crate::lexer::lex_line(
        "Equipped creature has deathtouch during your turn. Otherwise, it has reach.",
        0,
    )
    .unwrap();
    let abilities = parse_attached_conditional_keyword_otherwise_line(&tokens)
        .unwrap()
        .expect("conditional otherwise grant should parse");
    assert_eq!(abilities.len(), 2);
}

#[test]
fn otherwise_attached_damage_prevention_keeps_the_negated_typed_condition() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature has double strike as long as it's an enchantment. Otherwise, prevent all damage that would be dealt by enchanted creature.",
        0,
    )
    .expect("lex conditional attachment rule");
    let abilities = parse_attached_conditional_keyword_otherwise_line(&tokens)
        .expect("parse conditional attachment rule")
        .expect("conditional prevention should be recognized");
    assert_eq!(abilities.len(), 2, "{abilities:#?}");
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("DoubleStrike"), "{debug}");
    assert!(
        debug.contains("PreventAllDamageDealtByThisPermanent"),
        "{debug}"
    );
    assert!(debug.contains("Not("), "{debug}");
    assert!(debug.contains("AttachedToSourceMatches"), "{debug}");

    let changed_direction = crate::lexer::lex_line(
        "Enchanted creature has double strike as long as it's an enchantment. Otherwise, prevent all damage that would be dealt to enchanted creature.",
        0,
    )
    .expect("lex changed prevention direction");
    assert!(
        parse_attached_conditional_keyword_otherwise_line(&changed_direction)
            .expect("parse changed prevention direction")
            .is_none(),
        "the dealt-by family must not consume a dealt-to near miss"
    );
}

#[test]
fn attached_anthem_otherwise_base_and_block_restriction_stays_one_static_program() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature gets +2/+2 as long as it's a Detective you control. Otherwise, it has base power and toughness 1/1 and can't block Detectives.",
        0,
    )
    .expect("lex conditional attached anthem");
    let abilities = parse_attached_conditional_anthem_otherwise_base_and_restriction_line(&tokens)
        .expect("parse conditional attached anthem")
        .expect("conditional attached anthem should be recognized");
    let debug = format!("{abilities:#?}");
    assert_eq!(abilities.len(), 3, "{debug}");
    assert!(debug.contains("Anthem"), "{debug}");
    assert!(debug.contains("SetBasePowerToughness"), "{debug}");
    assert!(debug.contains("BlockSpecificAttacker"), "{debug}");
    assert_eq!(debug.matches("Not(").count(), 2, "{debug}");
    assert!(debug.contains("Detective"), "{debug}");

    let changed_subject = crate::lexer::lex_line(
        "Enchanted creature gets +2/+2 as long as it's a Detective you control. Otherwise, target creature has base power and toughness 1/1 and can't block Detectives.",
        0,
    )
    .expect("lex changed-subject near miss");
    assert!(
        parse_attached_conditional_anthem_otherwise_base_and_restriction_line(&changed_subject)
            .expect("changed-subject near miss should not error")
            .is_none(),
        "only the exact carried `it` branch belongs to this static program"
    );
}

#[test]
fn typed_attached_restriction_shapes_preserve_static_semantics() {
    let tokens = crate::lexer::lex_line("Enchanted creature can't attack or block.", 0).unwrap();
    assert!(
        parse_attached_cant_attack_or_block_line(&tokens)
            .unwrap()
            .is_some()
    );

    let tokens =
        crate::lexer::lex_line("All creatures able to block equipped creature do so.", 0).unwrap();
    assert!(
        parse_attached_all_creatures_able_to_block_line(&tokens)
            .unwrap()
            .is_some()
    );
}

#[test]
fn attached_color_condition_keeps_ability_loss_on_the_attached_creature() {
    let tokens = crate::lexer::lex_line(
        "As long as enchanted creature is red, it loses all abilities.",
        0,
    )
    .expect("lex attachment-relative ability loss");
    assert!(
        parse_attached_conditional_loses_all_abilities_line(&tokens)
            .expect("direct attachment-relative parser")
            .is_some(),
        "the specialist must recognize its exact owned surface: {:#?}",
        crate::lexer::parser_token_word_refs(&tokens)
    );
    let abilities = parse_static_ability_ast_line_lexed(&tokens)
        .expect("parse attachment-relative ability loss")
        .expect("attachment-relative ability loss should be recognized");
    let [
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            condition: Some(crate::ConditionExpr::AttachedToSourceMatches(filter)),
            ..
        },
    ] = abilities.as_slice()
    else {
        panic!("expected one conditional attached grant: {abilities:#?}");
    };
    assert_eq!(filter.colors, Some(crate::color::ColorSet::RED));
    assert!(
        matches!(
            ability.as_ref(),
            StaticAbilityAst::Static(ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter
        ),
        "the typed consequent must remove abilities from the attached object: {ability:#?}"
    );

    let global = crate::lexer::lex_line(
        "As long as you control a red creature, red creatures lose all abilities.",
        0,
    )
    .expect("lex global near miss");
    assert!(
        parse_attached_conditional_loses_all_abilities_line(&global)
            .expect("parse global near miss")
            .is_none(),
        "a controller condition must not be rewritten as an attachment condition"
    );
}

#[test]
fn attached_subject_carries_into_pronoun_loss_and_grant_sentence() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature can't attack or block. It loses all abilities and has \"{1}: Draw a card.\"",
        0,
    )
    .unwrap();
    let abilities = parse_carried_attached_subject_line(&tokens)
        .unwrap()
        .expect("attached subject should carry into the pronoun sentence");
    let debug = format!("{abilities:#?}");
    assert_eq!(abilities.len(), 3, "{debug}");
    assert!(debug.contains("AttackOrBlock"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert!(
        debug.contains("AttachedObjectAbilityGrant")
            || debug.contains("GrantObjectAbilityForFilter"),
        "{debug}"
    );
}

#[test]
fn attached_transform_subject_carries_into_combat_and_ability_loss_sentence() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature is a Turtle with base power and toughness 0/1. It can't attack and loses all abilities.",
        0,
    )
    .unwrap();
    let abilities = parse_carried_attached_subject_line(&tokens)
        .unwrap()
        .expect("transform subject should carry into the pronoun sentence");
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("SetCreatureSubtypes"), "{debug}");
    assert!(debug.contains("SetBasePowerToughness"), "{debug}");
    assert!(debug.contains("Attack"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert!(!debug.contains("ObjectFilter::source"), "{debug}");
}

#[test]
fn attached_transform_subject_carries_into_keyword_and_all_other_ability_loss() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature is a Citizen with base power and toughness 1/1. It has defender and loses all other abilities.",
        0,
    )
    .unwrap();
    let abilities = parse_carried_attached_subject_line(&tokens)
        .unwrap()
        .expect("transform subject should carry into the keyword/loss sentence");
    let debug = format!("{abilities:#?}");
    assert_eq!(abilities.len(), 5, "{debug}");
    assert!(debug.contains("SetCardTypes"), "{debug}");
    assert!(debug.contains("SetCreatureSubtypes"), "{debug}");
    assert!(debug.contains("SetBasePowerToughness"), "{debug}");
    assert!(debug.contains("Defender"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
}

#[test]
fn attached_anthem_subject_carries_through_leading_condition_chain() {
    let tokens = crate::lexer::lex_line(
        "Equipped creature gets +1/+1. As long as it's legendary, it gets an additional +2/+2. As long as it's red, it has trample.",
        0,
    )
    .unwrap();
    let abilities = parse_carried_attached_subject_line(&tokens)
        .unwrap()
        .expect("attached anthem subject should carry through both continuations");
    let debug = format!("{abilities:#?}");
    assert_eq!(abilities.len(), 3, "{debug}");
    assert_eq!(debug.matches("equipped").count(), 3, "{debug}");
    assert!(debug.contains("additional_surface: true"), "{debug}");
    assert!(debug.contains("Legendary"), "{debug}");
    assert!(debug.contains("Red"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");

    let changed_pronoun = crate::lexer::lex_line(
        "Equipped creature gets +1/+1. As long as it's legendary, this creature gets an additional +2/+2.",
        0,
    )
    .unwrap();
    assert!(
        parse_carried_attached_subject_line(&changed_pronoun)
            .unwrap()
            .is_none(),
        "only an exact carried `it` consequence belongs to this chain"
    );
}

#[test]
fn attached_object_controller_control_condition_binds_affected_target() {
    let tokens = crate::lexer::lex_line("its controller controls another creature", 0).unwrap();
    let condition = parse_static_condition_clause(&tokens)
        .expect("affected controller control condition should parse");
    let crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(filter),
        comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
        ..
    } = condition
    else {
        panic!("expected affected-controller count condition: {condition:#?}");
    };
    assert_eq!(
        filter.controller,
        Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
    );
    assert!(
        filter.other,
        "another must remain an other-object constraint"
    );

    let changed_actor = crate::lexer::lex_line("an opponent controls another creature", 0).unwrap();
    let changed = parse_static_condition_clause(&changed_actor)
        .expect("ordinary opponent condition should keep its existing route");
    assert!(
        !format!("{changed:#?}").contains("ControllerOf"),
        "only the affected-object possessive binds the target controller"
    );
}

#[test]
fn attached_treasure_transform_keeps_quoted_mana_ability_and_ability_loss() {
    let tokens = crate::lexer::lex_line(
        "Enchanted permanent is a Treasure artifact with \"{T}, Sacrifice this artifact: Add one mana of any color,\" and it loses all other abilities.",
        0,
    )
    .unwrap();
    let abilities = parse_attached_type_transform_line(&tokens)
        .unwrap()
        .expect("quoted activated ability transform should remain a static line");
    let debug = format!("{abilities:#?}");
    assert_eq!(abilities.len(), 4, "{debug}");
    assert!(debug.contains("SetCardTypes"), "{debug}");
    assert!(debug.contains("AddSubtypes"), "{debug}");
    assert!(debug.contains("AttachedObjectAbilityGrant"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");

    let no_loss = crate::lexer::lex_line(
        "Enchanted permanent is a Treasure artifact with \"{T}: Add one mana of any color.\"",
        0,
    )
    .unwrap();
    let abilities = parse_attached_type_transform_line(&no_loss)
        .unwrap()
        .expect("ordinary quoted transform remains supported");
    assert!(
        !format!("{abilities:#?}").contains("RemoveAllAbilities"),
        "loss must require the explicit trailing clause: {abilities:#?}"
    );
}

#[test]
fn attached_keyword_and_goaded_clause_keeps_both_continuous_abilities() {
    let tokens =
        crate::lexer::lex_line("Enchanted creature has indestructible and is goaded.", 0).unwrap();
    let abilities = parse_attached_has_keywords_and_is_goaded_line(&tokens)
        .unwrap()
        .expect("attached keyword-plus-goaded clause should parse");
    let debug = format!("{abilities:#?}");
    assert_eq!(abilities.len(), 2, "{debug}");
    assert!(debug.contains("Indestructible"), "{debug}");
    assert!(
        debug.contains("AttachedGoadedBySourceController"),
        "{debug}"
    );

    let dispatched = parse_static_ability_ast_line_lexed(&tokens)
        .unwrap()
        .expect("the static line dispatcher should retain both continuous abilities");
    assert_eq!(dispatched.len(), 2, "{dispatched:#?}");

    let ordinary = crate::lexer::lex_line("Enchanted creature has indestructible.", 0).unwrap();
    assert!(
        parse_attached_has_keywords_and_is_goaded_line(&ordinary)
            .unwrap()
            .is_none(),
        "an ordinary keyword grant must remain owned by the general attached-grant rule"
    );
}

#[test]
fn attached_keyword_grant_and_loss_dispatches_before_subject_filter_loss() {
    let tokens =
        crate::lexer::lex_line("Enchanted creature has defender and loses flying.", 0).unwrap();
    let routed = parse_static_ability_ast_line_lexed(&tokens)
        .unwrap()
        .expect("attached grant-and-loss clause should be claimed");
    let [
        StaticAbilityAst::GrantKeywordAction {
            filter: grant_filter,
            action: KeywordAction::Defender,
            condition: None,
        },
        StaticAbilityAst::RemoveKeywordAction {
            filter: loss_filter,
            action: KeywordAction::Flying,
            mode: ironsmith_core::AbilityLossMode::Lose,
        },
    ] = routed.as_slice()
    else {
        panic!("expected typed defender grant plus flying loss: {routed:#?}");
    };
    assert_eq!(grant_filter, loss_filter);
    assert!(
        grant_filter.static_abilities.is_empty(),
        "the granted keyword must not become an affected-object prerequisite: {grant_filter:#?}"
    );

    let filtered =
        crate::lexer::lex_line("Enchanted creature with defender loses flying.", 0).unwrap();
    let routed = parse_static_ability_ast_line_lexed(&filtered)
        .unwrap()
        .expect("ordinary qualified loss should remain supported");
    assert_eq!(
        routed.len(),
        1,
        "qualified loss is not a grant: {routed:#?}"
    );
    let StaticAbilityAst::RemoveKeywordAction { filter, .. } = &routed[0] else {
        panic!("expected one qualified keyword loss: {routed:#?}");
    };
    assert_eq!(
        filter.static_abilities,
        [crate::static_abilities::StaticAbilityId::Defender]
    );
}

#[test]
fn attached_restrictions_with_ignore_clause_become_static_special_action_semantics() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature can't attack or block, and its activated abilities can't be activated. That creature's controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn.",
        0,
    )
    .unwrap();
    let abilities = parse_attached_restrictions_with_ignore_special_action_line(&tokens)
        .unwrap()
        .expect("attached restriction plus ignore permission should parse");
    let debug = format!("{abilities:#?}");
    assert_eq!(abilities.len(), 3, "{debug}");
    assert!(debug.contains("AttackOrBlock"), "{debug}");
    assert!(debug.contains("ActivateAbilitiesOf"), "{debug}");
    assert!(
        debug.contains("AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn"),
        "{debug}"
    );

    let dispatched = parse_static_ability_ast_line_lexed(&tokens)
        .unwrap()
        .expect("static dispatcher should preserve the complete typed shape");
    assert_eq!(dispatched.len(), 3, "{dispatched:#?}");

    for unsupported in [
        "Enchanted creature can't attack or block, and its activated abilities can't be activated. That creature's controller may sacrifice a creature of their choice for that player to ignore this effect until end of turn.",
        "Enchanted creature can't attack or block, and its activated abilities can't be activated. That creature's controller may sacrifice a permanent of their choice for that player to ignore this effect until end of combat.",
    ] {
        let tokens = crate::lexer::lex_line(unsupported, 0).unwrap();
        assert!(
            parse_attached_restrictions_with_ignore_special_action_line(&tokens)
                .unwrap()
                .is_none(),
            "altered cost or duration must not be claimed: {unsupported}"
        );
    }
}

#[test]
fn attached_pt_compounds_preserve_both_characteristic_effects() {
    for (line, expected_secondary) in [
        (
            "Enchanted creature gets -5/-0 and loses all abilities.",
            "RemoveAllAbilities",
        ),
        (
            "Enchanted creature gets -3/-0 and loses all abilities.",
            "RemoveAllAbilities",
        ),
        (
            "Enchanted creature gets -2/-0 and loses all abilities.",
            "RemoveAllAbilities",
        ),
        ("Enchanted creature gets +3/+1 and is black.", "SetColors"),
    ] {
        let tokens = crate::lexer::lex_line(line, 0).unwrap();
        if line.contains(" is black") {
            let intended = parse_anthem_and_type_color_addition_line(&tokens)
                .unwrap()
                .expect("PT/color conjunction should match the compound characteristic parser");
            assert_eq!(intended.len(), 2, "{line}: {intended:#?}");
        }
        let routed = parse_static_ability_ast_line_lexed(&tokens)
            .unwrap()
            .expect("static rule dispatch should retain both characteristic effects");
        let routed_debug = format!("{routed:#?}");
        assert_eq!(routed.len(), 2, "{line}: {routed_debug}");
        assert!(routed_debug.contains("Anthem"), "{line}: {routed_debug}");
        assert!(
            routed_debug.contains(expected_secondary),
            "{line}: {routed_debug}"
        );
        let filters = routed
            .iter()
            .filter_map(|ability| {
                let StaticAbilityAst::Static(ability) = ability else {
                    return None;
                };
                match &ability.payload {
                    ironsmith_core::StaticAbilityPayload::Anthem(anthem) => anthem.filter.as_ref(),
                    ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(filter)
                    | ironsmith_core::StaticAbilityPayload::SetColors { filter, .. } => {
                        Some(filter)
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(filters.len(), 2, "{line}: {routed_debug}");
        assert_eq!(filters[0], filters[1], "{line}: {routed_debug}");
        assert!(
            filters[0]
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == "enchanted"),
            "{line}: {routed_debug}"
        );
    }

    let standalone =
        crate::lexer::lex_line("Creatures your opponents control lose all abilities.", 0).unwrap();
    let standalone = parse_static_ability_ast_line_lexed(&standalone)
        .unwrap()
        .expect("broad standalone ability loss should still route");
    let standalone_debug = format!("{standalone:#?}");
    assert_eq!(standalone.len(), 1, "{standalone_debug}");
    assert!(
        standalone_debug.contains("RemoveAllAbilities"),
        "{standalone_debug}"
    );
    assert!(!standalone_debug.contains("Anthem"), "{standalone_debug}");
}

#[test]
fn typed_attached_transform_and_prevention_shapes_lower() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature is a blue Frog with base power and toughness 3/3 in addition to its other types.",
        0,
    )
    .unwrap();
    assert!(
        parse_attached_type_transform_line(&tokens)
            .unwrap()
            .is_some()
    );

    let tokens = crate::lexer::lex_line(
        "If damage would be dealt to this creature, prevent that damage and remove a shield counter from it.",
        0,
    )
    .unwrap();
    assert!(
        parse_prevent_damage_to_source_remove_counter_line(&tokens)
            .unwrap()
            .is_some()
    );
}

#[test]
fn attached_land_type_setting_replaces_the_land_subtype_family() {
    let lush =
        crate::lexer::lex_line("Enchanted land is a Mountain, Forest, and Plains.", 0).unwrap();
    let lush = parse_attached_type_transform_line(&lush)
        .unwrap()
        .expect("land subtype setting should parse");
    let lush_debug = format!("{lush:#?}");
    assert!(lush_debug.contains("SetLandSubtypes"), "{lush_debug}");
    assert!(!lush_debug.contains("AddSubtypes"), "{lush_debug}");

    let song =
        crate::lexer::lex_line("Enchanted permanent is a colorless Forest land.", 0).unwrap();
    let song = parse_attached_type_transform_line(&song)
        .unwrap()
        .expect("card-type and land-subtype setting should parse");
    let song_debug = format!("{song:#?}");
    assert!(song_debug.contains("SetCardTypes"), "{song_debug}");
    assert!(song_debug.contains("SetLandSubtypes"), "{song_debug}");
    assert!(!song_debug.contains("AddSubtypes"), "{song_debug}");
}

#[test]
fn quoted_attached_activated_grants_parse_from_carried_tokens() {
    for (text, mana_cost, has_tap, mana_output) in [
        (
            "\"{2}: This creature gets +1/+0 until end of turn.\"",
            "{2}",
            false,
            None,
        ),
        (
            "\"{1}, {T}: Add {G}.\"",
            "{1}",
            true,
            Some(vec![crate::mana::ManaSymbol::Green]),
        ),
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let parsed = parse_attached_granted_activated_line(&tokens)
            .unwrap()
            .expect("quoted activated grant should parse from its carried tokens");
        let AbilityKind::Activated(activated) = parsed.kind() else {
            panic!("expected quoted grant to produce an activated ability");
        };
        assert_eq!(
            activated.mana_cost.mana_cost().unwrap().to_oracle(),
            mana_cost
        );
        assert_eq!(activated.mana_cost.has_non_mana_costs(), has_tap);
        assert_eq!(activated.mana_output, mana_output);
    }
}

#[test]
fn attached_has_clause_combines_keyword_and_quoted_activated_grants() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature has vigilance and \"{W}, {T}: Bolster 1.\" (To bolster 1, choose a creature with the least toughness among creatures you control and put a +1/+1 counter on it.)",
        0,
    )
    .unwrap();

    let abilities = parse_enchanted_creature_has_line(&tokens)
        .unwrap()
        .expect("mixed attached grants should parse");
    let debug = format!("{abilities:#?}");
    assert_eq!(abilities.len(), 2, "{debug}");
    assert!(debug.contains("Vigilance"), "{debug}");
    assert!(debug.contains("AttachedObjectAbilityGrant"), "{debug}");
    assert!(debug.contains("Bolster"), "{debug}");

    let dispatched = parse_static_ability_ast_line_lexed(&tokens)
        .unwrap()
        .expect("quoted Bolster must not be routed as a top-level keyword action");
    assert_eq!(dispatched.len(), 2, "{dispatched:#?}");
}

#[test]
fn attached_land_reset_lowers_loss_and_each_quoted_mana_ability() {
    let tokens = crate::lexer::lex_line(
        "Enchanted land loses all land types and abilities and has \"{T}: Add {C}\" and \"{T}, Pay 1 life: Add one mana of any color.\"",
        0,
    )
    .unwrap();

    let abilities = parse_attached_land_ability_reset_line(&tokens)
        .unwrap()
        .expect("attached land reset should parse");
    assert_eq!(abilities.len(), 4, "{abilities:#?}");
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("SetLandSubtypes"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert_eq!(
        debug.matches("AttachedObjectAbilityGrant").count(),
        2,
        "{debug}"
    );
    assert!(debug.contains("Life"), "{debug}");

    let dispatched = crate::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("attached land reset should reach the public static registry")
        .expect("attached land reset should be claimed by the public static registry");
    assert_eq!(dispatched.len(), 4, "{dispatched:#?}");
}
