use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_compound_attached_grant_shapes() {
    let tokens = lex_line("Enchanted creature has flying and loses first strike.", 0).unwrap();
    let parsed = parse_attached_has_and_loses_tokens(&tokens).unwrap();
    assert_eq!(parsed.subject, AttachedSubject::EnchantedCreature);

    let tokens = lex_line("Equipped creature gets +2/+2 and can't be blocked.", 0).unwrap();
    assert!(matches!(
        parse_attached_gets_tail_tokens(&tokens).map(|spec| spec.tail),
        Some(AttachedGetsTailKind::Restriction(
            AttachedCombatRestrictionKind::CantBeBlocked
        ))
    ));

    let tokens = lex_line("Enchanted creature gets -5/-0 and loses all abilities.", 0).unwrap();
    let parsed = parse_attached_gets_tail_tokens(&tokens).unwrap();
    let AttachedGetsTailKind::Loses(loss_tokens) = parsed.tail else {
        panic!("expected an attached ability-loss tail");
    };
    assert_eq!(
        super::super::super::super::lexer::token_word_refs(loss_tokens),
        ["all", "abilities"]
    );
}

#[test]
fn parses_land_type_ability_reset_with_multiple_quoted_grants() {
    let tokens = lex_line(
        "Enchanted land loses all land types and abilities and has \"{T}: Add {C}\" and \"{T}, Pay 1 life: Add one mana of any color.\"",
        0,
    )
    .unwrap();

    let parsed = parse_attached_land_ability_reset_tokens(&tokens).unwrap();
    assert_eq!(parsed.granted_abilities.len(), 2);
    assert_eq!(
        super::super::super::super::lexer::render_token_slice(parsed.granted_abilities[0]),
        "{T}: Add {C}"
    );
    assert_eq!(
        super::super::super::super::lexer::render_token_slice(parsed.granted_abilities[1]),
        "{T}, Pay 1 life: Add one mana of any color."
    );
}

#[test]
fn attached_grant_splits_ignore_nested_rule_and_reminder_conjunctions() {
    let tokens = lex_line(
        "vigilance and \"{W}, {T}: Bolster 1 and draw a card.\" (Choose a creature and put a counter on it.)",
        0,
    )
    .unwrap();

    let splits = parse_attached_ability_splits_tokens(&tokens);
    assert_eq!(splits.len(), 1, "{splits:#?}");
    assert_eq!(
        super::super::super::super::lexer::render_token_slice(splits[0].keyword_tokens),
        "vigilance"
    );
}
