use super::*;

#[test]
fn display_keeps_adjacent_mana_symbols_compact() {
    let tokens = crate::runtime_backend::lexer::lex_line("t {w}, {u}", 0).unwrap();
    assert_eq!(display_text_for_tokens(&tokens, false), "{T}{W}, {U}");
}

#[test]
fn otherwise_keyword_grant_uses_typed_clause_boundary() {
    let tokens = crate::runtime_backend::lexer::lex_line(
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
fn typed_attached_restriction_shapes_preserve_static_semantics() {
    let tokens =
        crate::runtime_backend::lexer::lex_line("Enchanted creature can't attack or block.", 0)
            .unwrap();
    assert!(
        parse_attached_cant_attack_or_block_line(&tokens)
            .unwrap()
            .is_some()
    );

    let tokens = crate::runtime_backend::lexer::lex_line(
        "All creatures able to block equipped creature do so.",
        0,
    )
    .unwrap();
    assert!(
        parse_attached_all_creatures_able_to_block_line(&tokens)
            .unwrap()
            .is_some()
    );
}

#[test]
fn attached_pt_and_all_ability_loss_preserves_both_characteristic_effects() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Enchanted creature gets -5/-0 and loses all abilities.",
        0,
    )
    .unwrap();
    let abilities = parse_attached_gets_and_cant_block_line(&tokens)
        .unwrap()
        .expect("compound attached characteristic line should parse");
    assert_eq!(abilities.len(), 2, "{abilities:#?}");

    let debug = format!("{abilities:#?}");
    assert!(debug.contains("Anthem"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert_eq!(debug.matches("TagKey(\"enchanted\")").count(), 2, "{debug}");

    let routed = parse_static_ability_ast_line_lexed(&tokens)
        .unwrap()
        .expect("static rule dispatch should retain the narrow attached compound");
    let routed_debug = format!("{routed:#?}");
    assert_eq!(routed.len(), 2, "{routed_debug}");
    assert!(routed_debug.contains("Anthem"), "{routed_debug}");
    assert!(
        routed_debug.contains("RemoveAllAbilities"),
        "{routed_debug}"
    );

    let standalone = crate::runtime_backend::lexer::lex_line(
        "Creatures your opponents control lose all abilities.",
        0,
    )
    .unwrap();
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
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Enchanted creature is a blue Frog with base power and toughness 3/3 in addition to its other types.",
        0,
    )
    .unwrap();
    assert!(
        parse_attached_type_transform_line(&tokens)
            .unwrap()
            .is_some()
    );

    let tokens = crate::runtime_backend::lexer::lex_line(
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
    let lush = crate::runtime_backend::lexer::lex_line(
        "Enchanted land is a Mountain, Forest, and Plains.",
        0,
    )
    .unwrap();
    let lush = parse_attached_type_transform_line(&lush)
        .unwrap()
        .expect("land subtype setting should parse");
    let lush_debug = format!("{lush:#?}");
    assert!(lush_debug.contains("SetLandSubtypes"), "{lush_debug}");
    assert!(!lush_debug.contains("AddSubtypes"), "{lush_debug}");

    let song = crate::runtime_backend::lexer::lex_line(
        "Enchanted permanent is a colorless Forest land.",
        0,
    )
    .unwrap();
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
        let tokens = crate::runtime_backend::lexer::lex_line(text, 0).unwrap();
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
fn attached_land_reset_lowers_loss_and_each_quoted_mana_ability() {
    let tokens = crate::runtime_backend::lexer::lex_line(
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
}
