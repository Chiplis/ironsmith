use super::*;

pub(in super::super) fn parse_exert_attack(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if parse_keyword_special_form_shape_tokens(tokens) != Some(KeywordSpecialFormShape::ExertAttack)
    {
        return Ok(None);
    }
    let context = rewrite_context(line, tokens, full_tokens, KeywordLineKind::ExertAttack);
    Ok(ast(parse_exert_attack_keyword_line(&context, tokens)?))
}
