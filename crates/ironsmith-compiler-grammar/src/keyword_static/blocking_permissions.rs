use super::*;

/// An unlimited-capacity static rule, scoped to its actual source or recipient.
/// Explicit temporary rules belong to effect grammar instead.
pub fn parse_can_block_any_number_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = crate::grammar::blocking_permissions::parse_can_block_any_number(tokens)
    else {
        return Ok(None);
    };
    if shape.this_turn || shape.subject_tokens.is_empty() {
        return Ok(None);
    }
    let words = crate::lexer::parser_token_word_refs(shape.subject_tokens);
    if words.iter().any(|word| *word == "target") {
        return Ok(None);
    }
    let permission = StaticAbilityAst::Static(StaticAbility::can_block_any_number());
    if matches!(words.as_slice(), ["enchanted" | "equipped", "creature"]) {
        return Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(permission),
            display: format!("{} can block any number of creatures", words.join(" ")),
            condition: None,
        }));
    }
    Ok(Some(match parse_anthem_subject(shape.subject_tokens)? {
        AnthemSubjectAst::Source => permission,
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(permission),
            condition: None,
        },
    }))
}
