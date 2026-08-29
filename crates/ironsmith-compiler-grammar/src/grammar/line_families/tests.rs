use super::*;
use crate::lexer::{lex_line, render_token_slice};

#[test]
fn visible_line_boundary_ignores_nested_ability_punctuation() {
    let tokens = lex_line(
        "Enchanted creature has vigilance and \"{W}, {T}: Bolster 1.\" (To bolster 1, choose a creature.)",
        0,
    )
    .expect("lex quoted grant with reminder");

    assert_eq!(
        render_token_slice(parse_visible_line_tokens(&tokens)),
        "Enchanted creature has vigilance and \"{W}, {T}: Bolster 1.\""
    );
}

#[test]
fn parses_keyword_station_partner_and_kicker_shapes() {
    let surge = lex_line("Surge {1}{U} (Reminder.)", 0).expect("lex");
    assert!(
        !parse_surge_line(&surge)
            .expect("surge body")
            .cost_tokens
            .is_empty()
    );

    let station = lex_line("8+ | Flying", 0).expect("lex");
    assert_eq!(
        parse_station_threshold(&station)
            .expect("station")
            .threshold,
        8
    );

    let station_reminder =
        lex_line("Station (This artifact is an artifact creature at 12+.)", 0).expect("lex");
    assert_eq!(
        parse_station_creature_threshold(&station_reminder),
        Some(12)
    );

    let partner = lex_line("Partner — Friends forever", 0).expect("lex");
    assert!(parse_partner_variant(&partner).is_some());

    let kicker = lex_line("Kicker {1}{U} and/or {2}{B} (Reminder.)", 0).expect("lex");
    assert!(parse_kicker_branches(&kicker).is_some());

    let ticket_marker = lex_line("{TK}{TK} — Prize sticker", 0).expect("lex");
    assert!(parse_sticker_ticket_marker(&ticket_marker).is_some());
}

#[test]
fn typed_line_family_migration_parses_document_dispatch_shapes() {
    let start = lex_line("Start your engines!", 0).expect("lex");
    assert_eq!(
        parse_simple_document_line(&start),
        Some(SimpleDocumentLineShape::StartYourEngines)
    );

    let draft =
        lex_line("Each player passes the booster pack to the next player.", 0).expect("lex");
    assert!(parse_draft_rule_line(&draft).is_some());

    let champion = lex_line("Champion a Goblin (Reminder.)", 0).expect("lex");
    let champion = parse_champion_line(&champion).expect("champion");
    assert_eq!(
        TokenWordView::new(champion.filter_tokens).word_refs(),
        vec!["goblin"]
    );

    let max_speed = lex_line("Max speed — Whenever you attack, draw a card.", 0).expect("lex");
    let max_speed = parse_max_speed_line(&max_speed).expect("max speed");
    assert!(max_speed.trigger_intro.is_some());

    let unless = lex_line("Unless you pay {2}, sacrifice this permanent.", 0).expect("lex");
    let unless = parse_leading_unless_line(&unless).expect("unless");
    assert!(!unless.condition_tokens.is_empty());
    assert!(!unless.effect_tokens.is_empty());
}

#[test]
fn typed_line_family_migration_parses_exact_special_lines() {
    for (text, expected) in [
        (
            "You may look at the top card of your library any time, and you may play lands from the top of your library.",
            SpecialLineShape::SplitTopLookAndLandPlay,
        ),
        (
            "Enchanted creature's controller may have it assign its combat damage as though it weren't blocked.",
            SpecialLineShape::AssignDamageAsUnblockedEnchanted,
        ),
        (
            "You may cast this card from your graveyard or from exile.",
            SpecialLineShape::GraveyardOrExileCast,
        ),
        (
            "After this main phase, there is an additional combat phase followed by an additional main phase.",
            SpecialLineShape::AdditionalCombatAfterMainPhase,
        ),
    ] {
        let tokens = lex_line(text, 0).expect("special line should lex");
        assert_eq!(parse_special_line(&tokens), Some(expected), "{text}");
    }
}

#[test]
fn typed_line_family_migration_parses_statement_routing_preferences() {
    let linked = lex_line(
        "For as long as that card remains exiled, spells with the same name cost {2} more to cast.",
        0,
    )
    .expect("lex");
    assert_eq!(
        parse_linked_statement_preference(&linked),
        Some(LinkedStatementPreference::ExiledCardCostsMore)
    );

    let equip = lex_line(
        "You may pay {0} rather than pay the equip cost of the first equip ability you activate each turn.",
        0,
    )
    .expect("lex");
    assert_eq!(
        parse_statement_static_preference(&equip),
        Some(StatementStaticPreference::FirstEquipCostAlternative)
    );
}

#[test]
fn classifies_document_effect_and_static_preference_shapes() {
    let multi = lex_line("Draw a card. Gain 2 life.", 0).expect("lex");
    assert!(parse_multi_sentence_effect_head(&multi).is_some());

    let prevention_then_trigger = lex_line(
        "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage and remove that many +1/+1 counters from it. When one or more counters are removed from this creature this way, it deals that much damage to any target.",
        0,
    )
    .expect("lex");
    let shape = parse_remove_counter_prevention_then_trigger(&prevention_then_trigger)
        .expect("typed prevention followed by trigger");
    assert_eq!(
        TokenWordView::new(shape.prevention_tokens)
            .word_refs()
            .first()
            .copied(),
        Some("if")
    );
    assert_eq!(
        TokenWordView::new(shape.trigger_tokens)
            .word_refs()
            .first()
            .copied(),
        Some("when")
    );

    let draw = lex_line("If you would draw a card, you may mill a card instead.", 0).expect("lex");
    assert_eq!(
        parse_statement_static_preference(&draw),
        Some(StatementStaticPreference::DrawReplacement)
    );

    let discard_or_redirect = lex_line(
        "If Mox Diamond would enter the battlefield, you may discard a land card instead. If you don't, put it into its owner's graveyard.",
        0,
    )
    .expect("lex");
    assert_eq!(
        parse_statement_static_preference(&discard_or_redirect),
        Some(StatementStaticPreference::DiscardOrRedirectReplacement)
    );

    let blocking = lex_line(
        "This creature can block an additional two creatures each combat.",
        0,
    )
    .expect("lex");
    assert_eq!(
        parse_statement_static_preference(&blocking),
        Some(StatementStaticPreference::BlocksAdditionalCreatures)
    );

    let filter_tail = lex_line("red, blue, or green permanents", 0).expect("lex");
    assert!(parse_filter_list_continuation(&filter_tail).is_some());

    let keyword_tail =
        lex_line("double strike, vigilance, or haste, transform this", 0).expect("lex");
    assert!(parse_filter_list_continuation(&keyword_tail).is_some());

    let final_keyword_tail = lex_line("or haste, transform this", 0).expect("lex");
    assert!(parse_filter_list_continuation(&final_keyword_tail).is_some());
}
