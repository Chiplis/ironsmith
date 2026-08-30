use super::*;

pub fn parse_for_each_prevent_shape(tokens: &[OwnedLexToken]) -> Option<ForEachPreventShape<'_>> {
    let (prevent_token, _, after_prevent) =
        primitives::find_prefix(tokens, || primitives::kw("prevent"))?;
    let subject_tokens = trim_lexed_commas(tokens.get(..prevent_token)?);
    let unless = primitives::find_prefix(after_prevent, || primitives::kw("unless"));
    let (prevent_tokens, unless_token) = if let Some((relative, _, _)) = unless {
        (
            trim_lexed_commas(tokens.get(prevent_token..prevent_token + 1 + relative)?),
            Some(prevent_token + 1 + relative),
        )
    } else {
        (trim_lexed_commas(tokens.get(prevent_token..)?), None)
    };
    Some(ForEachPreventShape {
        subject_tokens,
        prevent_tokens,
        unless_token,
    })
}
