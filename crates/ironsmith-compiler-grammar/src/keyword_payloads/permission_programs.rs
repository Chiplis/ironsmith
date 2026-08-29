use super::*;

pub(in super::super) fn parse_cast_this_spell_only(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    Ok(parse_cast_this_spell_only_line_lexed(tokens)?
        .map(|ability| KeywordLinePayload::ast(LineAst::StaticAbility(ability.into()))))
}
