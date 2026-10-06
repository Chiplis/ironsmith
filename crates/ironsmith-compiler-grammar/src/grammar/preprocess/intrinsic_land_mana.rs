//! Typed recognition of standalone CR 305.6 reminder text.

use crate::lexer::{OwnedLexToken, TokenKind};
use crate::types::Subtype;

/// Recognize only the complete parenthesized tap/add reminder. The document
/// owner validates the resulting types against card metadata. Unparenthesized
/// activations and quoted grants remain authored abilities, even when equal.
pub fn parse_intrinsic_basic_land_mana_reminder_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<Subtype>> {
    if tokens.first()?.kind != TokenKind::LParen
        || tokens.last()?.kind != TokenKind::RParen
    {
        return None;
    }
    let mut body = &tokens[1..tokens.len() - 1];
    if body.last().is_some_and(OwnedLexToken::is_period) {
        body = &body[..body.len() - 1];
    }
    if body.first()?.kind != TokenKind::ManaGroup
        || !body.first()?.slice.eq_ignore_ascii_case("{t}")
        || body.get(1)?.kind != TokenKind::Colon
        || (body.get(2)?.kind != TokenKind::Word || body.get(2)?.parser_text() != "add")
    {
        return None;
    }
    let mut remaining = &body[3..];
    let mut types = Vec::new();
    let mut final_alternative = false;
    loop {
        let token = remaining.first()?;
        if token.kind != TokenKind::ManaGroup { return None; }
        let subtype = match token.parser_text() {
            "{w}" => Subtype::Plains,
            "{u}" => Subtype::Island,
            "{b}" => Subtype::Swamp,
            "{r}" => Subtype::Mountain,
            "{g}" => Subtype::Forest,
            _ => return None,
        };
        if types.contains(&subtype) { return None; }
        types.push(subtype);
        remaining = &remaining[1..];
        if remaining.is_empty() {
            return (types.len() == 1 || final_alternative).then_some(types);
        }
        if final_alternative { return None; }
        if remaining.first()?.kind == TokenKind::Comma {
            remaining = &remaining[1..];
            if remaining.first()?.kind == TokenKind::Word && remaining.first()?.parser_text() == "or" {
                if types.len() < 2 { return None; }
                final_alternative = true;
                remaining = &remaining[1..];
            }
        } else if remaining.first()?.kind == TokenKind::Word && remaining.first()?.parser_text() == "or" {
            final_alternative = true;
            remaining = &remaining[1..];
        } else {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn reminder_role_requires_the_whole_parenthesized_surface() {
        for (text, expected) in [
            ("({T}: Add {G}.)", vec![Subtype::Forest]),
            ("({T}: Add {W} or {U}.)", vec![Subtype::Plains, Subtype::Island]),
            ("({T}: Add {G}, {W}, or {U}.)", vec![Subtype::Forest, Subtype::Plains, Subtype::Island]),
        ] {
            assert_eq!(parse_intrinsic_basic_land_mana_reminder_tokens(&lex_line(text, 0).unwrap()), Some(expected));
        }
        for text in [
            "{T}: Add {G}.", "\"{T}: Add {G}.\"", "({T}: Add {C}.)",
            "({T}: Add {G}{G}.)", "({T}: Add {G} and {U}.)",
            "({T}: Add {G}. You gain 1 life.)", "({T}: Add {G}.) ({T}: Add {U}.)",
            "({T}: Add {G} or.)", "({T}: Add {G} or {G}.)",
            "({T}: Add {G}. Activate only during your turn.)",
        ] {
            assert_eq!(parse_intrinsic_basic_land_mana_reminder_tokens(&lex_line(text, 0).unwrap()), None, "{text}");
        }
    }
}
