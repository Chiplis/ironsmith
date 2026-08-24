use super::*;

pub(super) fn filtered_return_phrase(
    tokens: &[OwnedLexToken],
    expected: &'static [&'static str],
) -> Option<bool> {
    let mut input = LexStream::new(tokens);
    let mut tapped = false;
    for expected_word in expected {
        loop {
            let token = super::super::next_word(&mut input).ok()?;
            let word = token.parser_text();
            if matches!(word, "a" | "an" | "the") {
                continue;
            }
            if word == "tapped" {
                tapped = true;
                continue;
            }
            if word != *expected_word {
                return None;
            }
            break;
        }
    }
    while let Ok(token) = super::super::next_word(&mut input) {
        if token.is_word("tapped") {
            tapped = true;
        } else if !matches!(token.parser_text(), "a" | "an" | "the") {
            return None;
        }
    }
    Some(tapped)
}

pub fn parse_return_tagged_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReturnTaggedBattlefieldShape> {
    let tapped = filtered_return_phrase(tokens, &["return", "those", "cards", "to", "battlefield"])
        .or_else(|| filtered_return_phrase(tokens, &["return", "them", "to", "battlefield"]))?;
    Some(ReturnTaggedBattlefieldShape { tapped })
}
