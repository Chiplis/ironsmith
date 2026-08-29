use super::*;

pub fn is_keyword_bundle_choice_filter(tokens: &[OwnedLexToken]) -> bool {
    let mut input = LexStream::new(tokens);
    let mut segments = 0usize;
    while let Ok(token) = super::super::next_word(&mut input) {
        if !matches!(token.parser_text(), "a" | "an" | "the") {
            continue;
        }
        let mut probe = input.clone();
        let Ok(card) = super::super::next_word(&mut probe) else {
            continue;
        };
        if !matches!(card.parser_text(), "card" | "cards") {
            continue;
        }
        let Ok(with) = super::super::next_word(&mut probe) else {
            continue;
        };
        if !with.is_word("with") {
            continue;
        }
        segments += 1;
        if segments >= 2 {
            return true;
        }
    }
    false
}
