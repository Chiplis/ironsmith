use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_attached_restriction_shapes() {
    let tokens = lex_line("Enchanted creature can't attack or block.", 0).unwrap();
    assert_eq!(
        parse_attached_combat_restriction_tokens(&tokens).map(|spec| spec.kind),
        Some(AttachedCombatRestrictionKind::CantAttackOrBlock)
    );
    let tokens = lex_line("All creatures able to block equipped creature do so.", 0).unwrap();
    assert_eq!(
        parse_all_creatures_block_attached_tokens(&tokens),
        Some(AttachedSubject::EquippedCreature)
    );

    let tokens = lex_line(
        "Enchanted creature can't attack or block and has \"{7}: Its controller sacrifices it and draws a card. Activate only as a sorcery.\"",
        0,
    )
    .unwrap();
    let shape = parse_attached_combat_restriction_grant_tokens(&tokens).unwrap();
    assert_eq!(shape.kind, AttachedCombatRestrictionKind::CantAttackOrBlock);
    assert!(!shape.ability_tokens.is_empty());
}

#[test]
fn attached_action_restriction_lists_preserve_actions_and_decline_other_clauses() {
    for (text, expected) in [
        (
            "Enchanted creature can't attack, block, or transform.",
            vec!["attack", "block", "transform"],
        ),
        (
            "Equipped creature can't transform or attack.",
            vec!["transform", "attack"],
        ),
        (
            "Enchanted permanent can't untap, transform or block.",
            vec!["untap", "transform", "block"],
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let (_, actions) = parse_attached_action_restriction_list_tokens(&tokens).unwrap();
        assert_eq!(actions, expected);
    }
    for text in [
        "Enchanted creature can't attack, block, or transform unless you pay {2}.",
        "Enchanted creature can't attack and transform.",
        "Enchanted creature can't attack, block, transform.",
        "Enchanted creature can't attack or transform and has flying.",
        "Enchanted creature can't attack or attack.",
        "Enchanted creature can't attack, block, or.",
        "Target creature can't attack or transform.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(
            parse_attached_action_restriction_list_tokens(&tokens).is_none(),
            "{text}"
        );
    }
}
