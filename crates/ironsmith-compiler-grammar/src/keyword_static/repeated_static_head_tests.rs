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
fn fixed_to_you_prevention_has_one_complete_canonical_owner() {
    for source in ["a source", "a source an opponent controls", "a green source",
        "a black source", "a red source", "a blue source", "a white source",
        "an artifact", "a creature"]
    {
        let text = format!("If {source} would deal damage to you, prevent 2 of that damage.");
        assert_reachable("parse_prevent_damage_to_you_from_source_filter_line", &text);
        let tokens = crate::lexer::lex_line(&text, 0).unwrap();
        assert!(parse_filtered_damage_prevention_line(&tokens).unwrap().is_none());
        let ability = parse_prevent_damage_to_you_from_source_filter_line(&tokens).unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventDamageToYouFromSourceFilter {
            amount, source_filter, ..
        } = ability.payload else { panic!("canonical fixed-to-you payload"); };
        assert_eq!(amount, 2);
        assert_eq!(source_filter.controller,
            (source == "a source an opponent controls").then_some(PlayerFilter::Opponent));
    }
    for text in [
        "If a source would deal combat damage to you, prevent 2 of that damage.",
        "If a source would deal 3 or less damage to you, prevent 2 of that damage.",
        "If a source would deal damage to a creature you control, prevent 2 of that damage.",
        "If a source would deal damage to you, prevent all but 2 of that damage.",
        "If a spell would deal damage to you, prevent 2 damage that spell would deal to that player.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(parse_prevent_damage_to_you_from_source_filter_line(&tokens).unwrap().is_none());
        // The repeated-recipient form must still preserve the authored scope;
        // unlike `you`, `that player` denotes Any and must be rejected.
        if !text.contains("that player") {
            assert_reachable("parse_filtered_damage_prevention_line", text);
        }
    }
}

#[test]
fn attached_source_prevention_readers_agree_on_aura_owned_typed_meaning() {
    for (text, combat_only) in [
        ("Prevent all damage that would be dealt by enchanted creature.", false),
        ("Prevent all combat damage that would be dealt by enchanted creature.", true),
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let general = parse_persistent_filtered_damage_prevention_line(&tokens).unwrap().unwrap();
        let attached = if combat_only {
            parse_attached_prevent_all_combat_damage_dealt_by_attached_line(&tokens)
        } else {
            parse_attached_prevent_all_damage_dealt_by_attached_line(&tokens)
        }.unwrap().unwrap();
        assert_eq!(attached, StaticAbilityAst::Static(general.clone()));
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = general.payload
            else { panic!("Aura-owned prevention"); };
        assert_eq!(spec.combat_only, combat_only);
        assert!(!spec.noncombat_only);
        assert_eq!(spec.amount, ironsmith_core::StaticDamagePreventionAmount::All);
        assert_eq!(spec.source_filter.tagged_constraints, vec![ironsmith_core::TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::Enchanted.bind().into(),
            relation: ironsmith_core::TaggedOpbjectRelation::IsTaggedObject,
        }]);
        assert!(spec.source_filter.with_attached_object.is_none());
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::Any));
        assert_eq!(spec.target_object_filter, Some(ObjectFilter::permanent()));
        assert_reachable("parse_persistent_filtered_damage_prevention_line", text);
    }
}

#[test]
fn complete_prevention_owners_do_not_swallow_unowned_tails() {
    for text in [
        "If a source would deal damage to you, prevent 1 of that damage and draw a card.",
        "If a source would deal damage to you, prevent 1 of that damage unless you pay 1 life.",
        "If a source would deal damage to you, prevent 1 of that damage this turn.",
        "If a source would deal damage to you, prevent 1 of that damage. Draw a card.",
        "Prevent all damage that would be dealt by enchanted creature this turn.",
        "Prevent all combat damage that would be dealt by enchanted creature unless you pay 1 life.",
        "Prevent all damage that would be dealt by enchanted creature. Draw a card.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(!matches!(parse_prevent_damage_to_you_from_source_filter_line(&tokens), Ok(Some(_))), "{text}");
        assert!(!matches!(parse_filtered_damage_prevention_line(&tokens), Ok(Some(_))), "{text}");
        assert!(!matches!(parse_persistent_filtered_damage_prevention_line(&tokens), Ok(Some(_))), "{text}");
        assert!(parse_attached_prevent_all_damage_dealt_by_attached_line(&tokens).unwrap().is_none());
        assert!(parse_attached_prevent_all_combat_damage_dealt_by_attached_line(&tokens).unwrap().is_none());
    }
}

#[test]
fn repeated_static_heads_preserve_land_animation_filters_and_pt() {
    for text in [
        "All lands are 1/1 creatures that are still lands.",
        "All lands are 2/2 creatures that are still lands.",
        "All Forests are 1/1 creatures that are still lands.",
        "All Swamps are 1/1 black creatures that are still lands.",
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
            "All Swamps are 1/1 black creatures that are still lands and have flying.",
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


#[test]
fn repeated_prevention_heads_keep_complete_permanent_source_qualifiers() {
    for text in [
        "Prevent all damage that would be dealt to this creature by artifact sources.",
        "Prevent all damage that would be dealt to this creature by artifact creatures.",
        "Prevent all damage that would be dealt to this creature by enchanted creatures.",
        "Prevent all damage that would be dealt to this creature by creatures with first strike.",
        "Prevent all damage that would be dealt to this creature by Deserts.",
    ] {
        assert_reachable("parse_permanent_self_damage_prevention_line", text);
    }
    assert_reachable("parse_prevent_all_damage_to_matching_permanents_line",
        "Prevent all damage that would be dealt to this creature.");
}
