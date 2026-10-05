use super::*;

fn parse(text: &str) -> Vec<StaticAbilityAst> {
    let tokens = crate::lexer::lex_line(text, 0).unwrap();
    let (result, loss) = crate::parse_loss::capture(|| parse_static_ability_ast_line_lexed(&tokens));
    let abilities = result.unwrap_or_else(|error| panic!("{text}: {error}")).expect(text);
    assert!(!loss.is_lossy(), "{text}: {}", loss.reasons_text());
    abilities
}

#[test]
fn complete_compound_static_predicates_have_one_faithful_registry_reading() {
    for (text, count) in [
        ("Equipped creature gets +2/+2 for each artifact you control and is an Artificer in addition to its other types.", 2),
        ("Equipped creature gets +1/+2, has reach, and can't be blocked by more than one creature.", 3),
        ("As long as you control three or more artifacts, this creature gets +2/+2 and can attack as though it didn't have defender.", 2),
        ("This creature gets +0/+2 as long as you control a Plains, has flying as long as you control an Island, gets +2/+0 as long as you control a Swamp, has first strike as long as you control a Mountain, and has trample as long as you control a Forest.", 5),
        ("This creature and enchanted creature each get +X/+X, where X is the number of creature cards in all graveyards.", 2),
        ("As long as an opponent has eight or more cards in their graveyard, this creature can attack as though it didn't have defender and it can't be blocked.", 2),
        ("Equipped creature has base power and toughness 7/7 and can't be blocked by creatures with power 2 or less.", 2),
    ] {
        let abilities = parse(text);
        assert_eq!(abilities.len(), count, "{text}: {abilities:#?}");
    }
}

#[test]
fn counterfactual_have_is_never_a_defender_grant() {
    let parsed = parse("As long as you control three or more artifacts, this creature gets +2/+2 and can attack as though it didn't have defender.");
    let StaticAbilityAst::ConditionalStaticAbility { ability, .. } = &parsed[1] else {
        panic!("permission must keep the artifact threshold: {parsed:#?}");
    };
    let StaticAbilityAst::Static(ability) = ability.as_ref() else { panic!("typed permission"); };
    assert_eq!(ability, &StaticAbility::can_attack_as_though_no_defender());
}

#[test]
fn base_stats_and_unquoted_block_rule_keep_distinct_layers_and_subjects() {
    let parsed = parse("Equipped creature has base power and toughness 7/7 and can't be blocked by creatures with power 2 or less.");
    let StaticAbilityAst::Static(rule) = &parsed[1] else {
        panic!("unquoted blocking rule must not become a granted recipient ability: {parsed:#?}");
    };
    let ironsmith_core::StaticAbilityPayload::RuleRestriction { restriction, .. } = &rule.payload else {
        panic!("typed blocking rule: {rule:#?}");
    };
    let crate::effect::Restriction::BlockSpecificAttacker { blockers, attacker } = restriction else {
        panic!("specific recipient block restriction");
    };
    assert_eq!(blockers.power, Some(crate::filter::Comparison::LessThanOrEqual(2)));
    assert_eq!(blockers.card_types, [CardType::Creature]);
    assert!(attacker.with_attached_object.is_some());
}

#[test]
fn partial_readers_decline_complete_compounds_and_unknown_tails_stay_rejected() {
    let tokens = crate::lexer::lex_line("Equipped creature gets +2/+2 for each artifact you control and is an Artificer in addition to its other types.", 0).unwrap();
    assert!(parse_anthem_line(&tokens).unwrap().is_none());
    let tokens = crate::lexer::lex_line("Equipped creature gets +1/+2, has reach, and can't be blocked by more than one creature.", 0).unwrap();
    assert!(parse_subject_has_keywords_and_cant_be_blocked_by_more_than_line(&tokens).unwrap().is_none());
    for text in [
        "As long as an opponent has eight or more cards in their graveyard, this creature can attack as though it didn't have defender and it can't be blocked and draws a card.",
        "Equipped creature has base power and toughness 7/7 and can't be blocked by creatures with power 2 or less and dances.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let owner = if text.starts_with("As") {
            parse_conditional_no_defender_and_unblockable_line(&tokens)
        } else {
            parse_base_pt_and_blocker_restriction_line(&tokens)
        };
        assert!(!matches!(owner, Ok(Some(_))), "no partial compound: {text}");
    }
}

#[test]
fn a_shared_condition_guards_the_stat_and_type_addition_layers() {
    let parsed = parse("As long as you control three or more artifacts, equipped creature gets +2/+2 for each artifact you control and is an Artificer in addition to its other types.");
    assert_eq!(parsed.len(), 2);
    let StaticAbilityAst::Static(anthem) = &parsed[0] else { panic!("typed anthem"); };
    let ironsmith_core::StaticAbilityPayload::Anthem(anthem) = &anthem.payload else { panic!("anthem payload"); };
    let StaticAbilityAst::Static(addition) = &parsed[1] else { panic!("typed type addition"); };
    let ironsmith_core::StaticAbilityPayload::Conditional { condition, .. } = &addition.payload else {
        panic!("type addition must retain the shared condition: {addition:#?}");
    };
    assert_eq!(anthem.condition.as_ref(), Some(condition));
}
