use crate::lexer::lex_line;

use super::*;

#[test]
fn token_copy_modifier_returns_typed_followup_kind() {
    let haste = lex_line("It gains haste until end of turn.", 0).expect("lex fixture");
    assert_eq!(
        parse_token_copy_modifier_kind(&haste),
        Some(TokenCopyModifierKind::GainHasteUntilEndOfTurn)
    );
    let plural_haste = lex_line("Those tokens gain haste.", 0).expect("plural token haste fixture");
    assert_eq!(
        parse_token_copy_modifier_kind(&plural_haste),
        Some(TokenCopyModifierKind::HasHaste)
    );
    let temporary_plural_haste = lex_line("Those tokens gain haste until end of turn.", 0)
        .expect("temporary plural token haste fixture");
    assert_eq!(
        parse_token_copy_modifier_kind(&temporary_plural_haste),
        Some(TokenCopyModifierKind::GainHasteUntilEndOfTurn)
    );

    let sacrifice =
        lex_line("Sacrifice it at the beginning of the next end step.", 0).expect("lex fixture");
    assert_eq!(
        parse_token_copy_modifier_kind(&sacrifice),
        Some(TokenCopyModifierKind::SacrificeAtNextEndStep)
    );

    let conditional = lex_line(
        "Sacrifice it at the beginning of the next end step if it has mana value 3 or less.",
        0,
    )
    .expect("conditional delayed sacrifice fixture");
    assert_eq!(
        parse_token_copy_modifier_kind(&conditional),
        None,
        "a behavior-bearing suffix must be parsed by the delayed-action grammar"
    );

    let attacking =
        lex_line("The token enters tapped and attacking that player.", 0).expect("lex fixture");
    assert_eq!(
        parse_token_copy_modifier_kind(&attacking),
        Some(TokenCopyModifierKind::EnterTappedAndAttackingThatPlayer)
    );
}
