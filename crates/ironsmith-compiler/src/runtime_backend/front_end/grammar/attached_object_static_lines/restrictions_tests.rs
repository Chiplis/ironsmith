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
