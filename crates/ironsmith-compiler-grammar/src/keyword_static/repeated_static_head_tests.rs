use super::*;

fn rule_named(name: &str) -> (usize, &StaticAbilityLineRuleDef) {
    static_ability_ast_line_rules()
        .iter()
        .enumerate()
        .find(|(_, rule)| rule.id.as_str() == name)
        .expect("specialist must be registered")
}

/// Compare the complete specialist AST with registry dispatch, not merely the
/// absence of an error: routing must preserve the existing executable meaning.
fn assert_reachable(name: &str, text: &str) {
    let tokens = crate::lexer::lex_line(text, 0).expect("fixture should lex");
    let (index, rule) = rule_named(name);
    let words = parser_token_word_refs(&tokens);
    assert!(
        STATIC_ABILITY_AST_LINE_RULE_INDEX
            .candidate_indices(words[0], words.get(1).copied())
            .contains(&index),
        "{name} must be eligible for {text}"
    );
    let ParseOutcome::Match(direct) =
        run_static_ability_ast_line_rule(rule.id, rule.parse, &tokens)
    else {
        panic!("the complete specialist must support {text}");
    };
    let ParseOutcome::Match(dispatched) = recognize_static_ability_ast_line_registry(&tokens)
    else {
        panic!("registry must dispatch {text}");
    };
    assert_eq!(direct.value, dispatched.value, "{text}");
    assert!(
        parse_static_ability_ast_line_lexed(&tokens)
            .expect("full static parse must not error")
            .is_some(),
        "{text}"
    );
}

#[test]
fn repeated_static_heads_preserve_toughness_subjects() {
    for text in [
        "This creature assigns combat damage equal to its toughness rather than its power.",
        "Each creature assigns combat damage equal to its toughness rather than its power.",
        "Each creature you control assigns combat damage equal to its toughness rather than its power.",
    ] {
        assert_reachable(
            "parse_creatures_assign_combat_damage_using_toughness_line",
            text,
        );
    }
}

#[test]
fn repeated_static_heads_preserve_attached_prevention_direction_and_combat_scope() {
    for (name, text) in [
        (
            "parse_attached_prevent_all_damage_dealt_to_and_by_attached_line",
            "Prevent all damage that would be dealt to and dealt by enchanted creature.",
        ),
        (
            "parse_attached_prevent_all_damage_dealt_by_attached_line",
            "Prevent all damage that would be dealt by enchanted creature.",
        ),
        (
            "parse_attached_prevent_all_combat_damage_dealt_by_attached_line",
            "Prevent all combat damage that would be dealt by enchanted creature.",
        ),
        (
            "parse_attached_prevent_all_damage_dealt_to_attached_line",
            "Prevent all damage that would be dealt to enchanted creature.",
        ),
    ] {
        assert_reachable(name, text);
    }
}

#[test]
fn repeated_static_heads_preserve_land_animation_filters_and_pt() {
    for text in [
        "All lands are 1/1 creatures that are still lands.",
        "All lands are 2/2 creatures that are still lands.",
        "All Forests are 1/1 creatures that are still lands.",
        "Forests you control are 3/4 creatures that are still lands.",
        "Nonbasic lands are 2/3 creatures that are still lands.",
    ] {
        assert_reachable("parse_lands_are_pt_creatures_still_lands_line", text);
    }
}

#[test]
fn repeated_static_heads_preserve_damage_redirection_destinations() {
    assert_reachable(
        "parse_damage_redirect_to_source_line",
        "All damage that would be dealt to you and other permanents you control is dealt to this creature instead.",
    );
    assert_reachable(
        "parse_damage_redirect_to_source_controller_line",
        "If a creature would deal damage to you, it deals that damage to its controller instead.",
    );
}

#[test]
fn repeated_static_heads_preserve_attachment_color_choice() {
    for text in [
        "As this Equipment becomes attached to a creature, choose a color.",
        "As this Aura becomes attached to a permanent, choose a color.",
    ] {
        assert_reachable("parse_choose_color_as_becomes_attached_line", text);
    }
}

#[test]
fn repeated_static_heads_do_not_widen_specialist_semantic_guards() {
    for (name, text) in [
        (
            "parse_creatures_assign_combat_damage_using_toughness_line",
            "Each creature you control with toughness greater than its power assigns combat damage equal to its toughness rather than its power.",
        ),
        (
            "parse_creatures_assign_combat_damage_using_toughness_line",
            "This creature assigns combat damage equal to its toughness rather than its power until end of turn.",
        ),
        (
            "parse_attached_prevent_all_damage_dealt_by_attached_line",
            "Prevent all damage that would be dealt by target creature this turn.",
        ),
        (
            "parse_attached_prevent_all_damage_dealt_to_attached_line",
            "Prevent all damage that would be dealt to enchanted creature by artifact sources.",
        ),
        (
            "parse_attached_prevent_all_combat_damage_dealt_by_attached_line",
            "Prevent all noncombat damage that would be dealt by enchanted creature.",
        ),
        (
            "parse_lands_are_pt_creatures_still_lands_line",
            "All Swamps are 1/1 black creatures that are still lands.",
        ),
        (
            "parse_lands_are_pt_creatures_still_lands_line",
            "All lands are 1/1 creatures that are still lands until end of turn.",
        ),
        (
            "parse_damage_redirect_to_source_line",
            "All damage that would be dealt to you is dealt to this creature instead.",
        ),
        (
            "parse_damage_redirect_to_source_controller_line",
            "If a creature would deal damage to you, it deals that damage to target player instead.",
        ),
        (
            "parse_choose_color_as_becomes_attached_line",
            "As this Equipment becomes attached to a creature, choose a creature type.",
        ),
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let (_, rule) = rule_named(name);
        assert!(
            matches!(
                run_static_ability_ast_line_rule(rule.id, rule.parse, &tokens),
                ParseOutcome::NoMatch
            ),
            "{name} must not erase the changed qualifier in {text}"
        );
    }
}
