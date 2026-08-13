use super::*;

#[test]
fn dynamic_soulshift_and_special_phrases_are_typed() {
    let soulshift = parse_dynamic_soulshift_words(&[
        "soulshift",
        "x",
        "where",
        "x",
        "is",
        "the",
        "number",
        "of",
        "spirits",
        "you",
        "control",
    ])
    .unwrap();
    assert!(soulshift.count_filter.subtypes.contains(&Subtype::Spirit));
    assert_eq!(
        parse_special_ability_phrase_words(&["start", "your", "engines"]),
        Some(SpecialAbilityPhraseKind::StartYourEngines)
    );
    assert_eq!(
        parse_special_ability_phrase_words(&[
            "casualty",
            "x",
            "the",
            "copy",
            "isnt",
            "legendary",
            "and",
            "has",
            "starting",
            "loyalty",
            "x",
            "where",
            "x",
            "is",
            "the",
            "sacrificed",
            "creatures",
            "power",
        ]),
        Some(SpecialAbilityPhraseKind::VariableCasualtyPlaneswalkerCopy)
    );
}

#[test]
fn typed_static_grant_migration_parses_dynamic_soulshift_tokens() {
    let tokens = lex("Soulshift X, where X is the number of Spirits you control.");
    let parsed = parse_dynamic_soulshift_tokens(&tokens).expect("typed soulshift");
    assert!(parsed.count_filter.subtypes.contains(&Subtype::Spirit));
}

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    super::super::super::lexer::lex_line(raw, 0).expect("test text should lex")
}

#[test]
fn parses_untap_restriction_shapes() {
    assert_eq!(
        parse_keyword_untap_restriction_words(&["untap"]),
        Some(KeywordUntapRestriction::Bare)
    );
    assert_eq!(
        parse_keyword_untap_restriction_words(&[
            "untaps",
            "during",
            "its",
            "controllers",
            "next",
            "untap",
            "step",
        ]),
        Some(KeywordUntapRestriction::DuringStep)
    );
    assert_eq!(
        parse_keyword_untap_restriction_words(&["untaps", "during", "combat"]),
        None
    );
}

#[test]
fn separates_payment_or_but_not_comparison_or() {
    let payment = lex("{2} or pay 3 life");
    assert_eq!(
        parse_payment_alternative_split_tokens(&payment),
        Some(PaymentAlternativeSplit { delimiter: 1 })
    );

    let comparison = lex("mana value 3 or less");
    assert_eq!(parse_payment_alternative_split_tokens(&comparison), None);
}

#[test]
fn parses_dynamic_payment_surfaces() {
    let energy = lex("an amount of {E} equal to X");
    assert!(matches!(
        parse_keyword_dynamic_payment_tokens(&energy),
        Some(KeywordDynamicPaymentShape::Energy { value }) if value == (6..7)
    ));

    let mana = lex("{X}{G}, where X is the number of creatures you control");
    let Some(KeywordDynamicPaymentShape::Mana {
        cost,
        trailing_first,
    }) = parse_keyword_dynamic_payment_tokens(&mana)
    else {
        panic!("expected dynamic mana surface");
    };
    assert!(cost.has_x());
    let tail = &mana[trailing_first + 1..];
    assert_eq!(
        parse_keyword_dynamic_mana_tail_tokens(tail),
        KeywordDynamicManaTail::WhereX {
            same_name_in_graveyard: false,
        }
    );

    assert_eq!(
        parse_keyword_dynamic_mana_tail_tokens(&lex("and three life")),
        KeywordDynamicManaTail::Life { value: Some(1..2) }
    );
    assert_eq!(
        parse_keyword_dynamic_mana_tail_tokens(&lex("and three life or discard a card")),
        KeywordDynamicManaTail::Life { value: None }
    );
}

#[test]
fn parses_ability_heads_and_reminders() {
    let cumulative = lex("and cumulative upkeep—Pay 2 life. (Reminder.)");
    let surface = parse_keyword_ability_surface_tokens(&cumulative).unwrap();
    assert_eq!(surface.phrase_first, 1);
    assert!(matches!(
        surface.head,
        KeywordAbilityHead::CumulativeUpkeep { cost } if cost == (3..7)
    ));

    let crew = lex("Crew 3 (Activate only as a sorcery. Activate only once each turn.)");
    let surface = parse_keyword_ability_surface_tokens(&crew).unwrap();
    assert_eq!(surface.head, KeywordAbilityHead::Crew);
    assert!(surface.sorcery_speed_reminder);
    assert!(surface.once_per_turn_reminder);

    assert_eq!(
        parse_keyword_ability_surface_tokens(&lex("Modular sunburst"))
            .unwrap()
            .head,
        KeywordAbilityHead::Modular { sunburst: true }
    );
    assert_eq!(
        parse_keyword_ability_surface_tokens(&lex("Battle cry"))
            .unwrap()
            .head,
        KeywordAbilityHead::BattleCry
    );

    let unblockable = lex("This creature can't be blocked.");
    assert!(
        parse_keyword_ability_surface_tokens(&unblockable)
            .unwrap()
            .unblockable_tail
    );
}

#[test]
fn parses_trigger_object_heads_through_leaf_grammar() {
    assert_eq!(
        parse_keyword_trigger_object_head("creatures"),
        Some(KeywordTriggerObjectHead::CardType(CardType::Creature))
    );
    assert!(matches!(
        parse_keyword_trigger_object_head("goblins"),
        Some(KeywordTriggerObjectHead::Subtype(Subtype::Goblin))
    ));
    assert_eq!(parse_keyword_trigger_object_head("quickly"), None);
}
