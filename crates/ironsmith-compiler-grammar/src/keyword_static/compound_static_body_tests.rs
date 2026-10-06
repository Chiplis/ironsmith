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

#[test]
fn chosen_type_damage_uses_one_canonical_multiplier_with_source_scope() {
    let text = "Double all damage that sources you control of the chosen type would deal.";
    let tokens = crate::lexer::lex_line(text, 0).unwrap();
    assert_eq!(parse_double_damage_from_sources_you_control_of_chosen_type_line(&tokens).unwrap(),
        parse_double_damage_amount_replacement_line(&tokens).unwrap());
    let parsed = parse(text);
    let [StaticAbilityAst::Static(ability)] = parsed.as_slice() else { panic!("one multiplier"); };
    let ironsmith_core::StaticAbilityPayload::DoubleDamageAmountReplacement { source_filter, factor, .. } = &ability.payload else {
        panic!("common typed multiplier: {ability:#?}");
    };
    assert_eq!(*factor, 2);
    assert!(source_filter.chosen_creature_type);
    assert_eq!(source_filter.controller, Some(PlayerFilter::You));
    assert!(source_filter.card_types.is_empty());
    assert!(source_filter.zone.is_none());
}

#[test]
fn complete_entry_characteristics_do_not_become_a_granted_keyword() {
    let text = "As a historic permanent you control enters, it becomes a 7/7 Dinosaur creature in addition to its other types.";
    let tokens = crate::lexer::lex_line(text, 0).unwrap();
    assert!(parse_granted_keyword_static_line(&tokens).unwrap().is_none());
    let parsed = parse(text);
    let [StaticAbilityAst::Static(ability)] = parsed.as_slice() else { panic!("one complete replacement"); };
    let ironsmith_core::StaticAbilityPayload::EntersWithCharacteristicsForFilter {
        filter, card_types, subtypes, power, toughness,
    } = &ability.payload else { panic!("typed entry replacement: {ability:#?}"); };
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(card_types, &[CardType::Creature]);
    assert_eq!(subtypes, &[crate::Subtype::Dinosaur]);
    assert_eq!((*power, *toughness), (7, 7));
}

#[test]
fn chosen_parity_protection_is_a_typed_complete_quality_not_a_marker() {
    let text = "This creature has protection from each mana value of the chosen quality.";
    let tokens = crate::lexer::lex_line(text, 0).unwrap();
    assert!(parse_static_text_marker_line(&tokens).is_none());
    let parsed = parse(text);
    let [StaticAbilityAst::KeywordAction(KeywordAction::ProtectionFromFilter(filter))] = parsed.as_slice() else {
        panic!("typed source protection: {parsed:#?}");
    };
    assert_eq!(filter.mana_value_parity, Some(ironsmith_core::ParityRequirement::Chosen));
    assert!(filter.card_types.is_empty() && filter.zone.is_none());
    for (text, parity) in [("Protection from odd mana values.", ironsmith_core::ParityRequirement::Odd),
        ("Protection from even mana values.", ironsmith_core::ParityRequirement::Even)] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let parsed = crate::clause_support::parse_protection_chain(&tokens).unwrap();
        assert!(matches!(parsed.as_slice(), [KeywordAction::ProtectionFromFilter(filter)] if filter.mana_value_parity == Some(parity)));
    }
    let unknown = crate::lexer::lex_line("Protection from each mana value of the chosen quality and draw a card.", 0).unwrap();
    assert!(crate::clause_support::parse_protection_chain(&unknown).is_none());
}

#[test]
fn parity_protection_rejects_hidden_symbol_and_punctuation_tails_in_live_readers() {
    for quality in [
        "Protection {R} from odd mana values.",
        "Protection from {R} odd mana values.",
        "Protection from odd {R} mana values.",
        "Protection from even mana values {R}.",
        "Protection from odd mana values:",
        "Protection from each mana value of the chosen {R} quality.",
        "Protection from each mana value of the chosen quality {R}.",
        "Protection from each mana value of the chosen quality:",
    ] {
        let tokens = crate::lexer::lex_line(quality, 0).unwrap();
        assert!(crate::grammar::clause_support::parse_protection_chain_tokens(&tokens).is_none(), "{quality}");
        assert!(crate::clause_support::parse_protection_chain(&tokens).is_none(), "{quality}");
        assert!(parse_ability_line(&tokens).is_none(), "{quality}");
        let granted = crate::lexer::lex_line(&format!("This creature has {quality}"), 0).unwrap();
        assert!(!matches!(parse_static_ability_ast_line_lexed(&granted), Ok(Some(_))), "{quality}");
    }
}


#[test]
fn miracle_grant_readers_share_the_whole_derived_cost_and_hand_subject() {
    let text = "Each enchantment card in your hand has miracle. Its miracle cost is equal to its mana cost reduced by {4}.";
    let tokens = crate::lexer::lex_line(text, 0).unwrap();
    assert_eq!(parse_filter_has_granted_ability_line(&tokens).unwrap(), parse_granted_keyword_static_line(&tokens).unwrap());
    let parsed = parse(text);
    let [StaticAbilityAst::Static(ability)] = parsed.as_slice() else { panic!("one complete grant"); };
    let ironsmith_core::StaticAbilityPayload::Grants(spec) = &ability.payload else { panic!("typed grant"); };
    assert_eq!(spec.zone, Zone::Hand);
    assert_eq!(spec.filter.card_types, vec![CardType::Enchantment]);
    assert!(matches!(spec.grantable, ironsmith_core::Grantable::DerivedAlternativeCast(
        ironsmith_core::DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction: 4 })));
    for malformed in [
        "Each enchantment card in your hand has miracle. Its miracle cost is equal to its mana cost reduced by {4}. Draw a card.",
        "Each enchantment card in your hand has miracle. Its miracle cost is equal to its mana cost reduced by {4} and draw a card.",
        "Each enchantment card in your hand has miracle. Its miracle cost is equal to its mana cost reduced by {4}:",
        "Each {R} enchantment card in your hand has miracle. Its miracle cost is equal to its mana cost reduced by {4}.",
        "Each enchantment card in {R} your hand has miracle. Its miracle cost is equal to its mana cost reduced by {4}.",
        "Each enchantment card in your hand: has miracle. Its miracle cost is equal to its mana cost reduced by {4}.",
    ] {
        let tokens = crate::lexer::lex_line(malformed, 0).unwrap();
        for result in [parse_filter_has_granted_ability_line(&tokens), parse_granted_keyword_static_line(&tokens)] {
            assert!(!matches!(result, Ok(Some(_))), "whole cost grant must reject {malformed}");
        }
    }
}

#[test]
fn complete_protection_lists_and_live_color_scopes_share_keyword_owners() {
    for (text, count) in [
        ("Protection from monocolored", 1),
        ("Protection from snow", 1),
        ("Protection from blue, from black, and from red", 3),
        ("Protection from Vampires, from Werewolves, and from Zombies", 3),
        ("Protection from each of its colors", 1),
        ("Protection from each color among permanents you control", 1),
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let chain = crate::clause_support::parse_protection_chain(&tokens).unwrap();
        assert_eq!(chain.len(), count, "{text}");
        assert_eq!(parse_ability_line(&tokens), Some(chain), "{text}");
    }
    let tokens = crate::lexer::lex_line("Protection from snow", 0).unwrap();
    assert_eq!(crate::clause_support::parse_protection_chain(&tokens), Some(vec![
        KeywordAction::ProtectionFromFilter(ObjectFilter::default().with_supertype(crate::types::Supertype::Snow))]));
    let own = parse("Each creature has protection from each of its colors.");
    assert!(!own.is_empty());
    let population = parse("This creature has protection from each color among permanents you control.");
    assert!(!population.is_empty());
    let pledge = parse("Enchanted creature has protection from each color among permanents you control. This effect doesn't remove this Aura.");
    assert_eq!(pledge.len(), 2);
    assert!(pledge.iter().any(|ability| matches!(ability, StaticAbilityAst::AttachedKeywordActionGrant {
        action: KeywordAction::ProtectionFromColorsAmong(_), .. })));
    assert!(pledge.iter().any(|ability| matches!(ability, StaticAbilityAst::Static(rule)
        if rule.id() == crate::static_abilities::StaticAbilityId::ProtectionDoesntRemoveThisAura)));
}

#[test]
fn protection_qualities_consume_symbols_separators_and_all_tail_words() {
    for text in [
        "Protection from monocolored {R}", "Protection from snow:",
        "Protection from blue, from {R} black, and from red", "Protection from blue, and, from black",
        "Protection from blue from black", "Protection from red and", "Protection from red,",
        "Protection from snow nonsense", "Protection from monocolored nonsense",
        "Protection from each of its colors {R}", "Protection from each color among permanents you control:",
        "Protection from each color among permanents you control nonsense",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(crate::clause_support::parse_protection_chain(&tokens).is_none(), "{text}");
    }
}

#[test]
fn raw_protection_grant_delimiters_and_quoted_continuous_context_cannot_be_trimmed_away() {
    for text in [
        "Each creature has protection from each of its colors,",
        "Each creature has , protection from each of its colors.",
        "Each creature has flying, protection from each of its colors,.",
        "This creature has protection from each color among permanents you control,.",
        "Enchanted creature has protection from each color among permanents you control,. This effect doesn't remove this Aura.",
        "Creatures have \"This creature has protection from each color among permanents you control.\"",
        "Creatures have flying and \"This creature has protection from each color among permanents you control.\"",
        "Enchanted creature has \"This creature has protection from each color among permanents you control.\"",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(crate::clause_support::validate_protection_static_line(&tokens).is_err(), "{text}");
        for result in [parse_filter_has_granted_ability_line(&tokens),
            parse_granted_keyword_static_line(&tokens), parse_enchanted_creature_has_line(&tokens)] {
            assert!(result.is_err(), "raw grant reader must reject {text}");
        }
    }
    for text in [
        "If this creature has flying, target creature gains \"This creature has protection from each color among permanents you control.\" until end of turn.",
        "{T}: Target creature gains \"This creature has protection from each color among permanents you control.\" until end of turn.",
        "Whenever this creature attacks, target creature gains \"This creature has protection from each color among permanents you control.\" until end of turn.",
        "Human creatures you control have \"{T}: Add one mana of any of this creature's colors.\"",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(crate::clause_support::validate_protection_static_line(&tokens).is_ok(), "{text}");
    }
}
