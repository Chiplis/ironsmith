use super::*;

pub(in super::super) fn parse_gift(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !is_standard_gift_keyword_tokens_lexed(tokens) {
        return Ok(None);
    }
    let context = rewrite_context(line, tokens, full_tokens, KeywordLineKind::Gift);
    Ok(ast(parse_gift_keyword_line(&context)?))
}
