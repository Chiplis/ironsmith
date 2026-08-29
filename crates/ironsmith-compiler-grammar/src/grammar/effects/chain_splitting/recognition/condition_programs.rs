use super::*;

pub(super) fn last_non_quantifier_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    let mut last = None;
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            return last;
        };
        if let Some(word) = token.as_word()
            && !matches!(word, "a" | "an" | "the" | "all" | "each")
        {
            last = Some(word);
        }
    }
}
