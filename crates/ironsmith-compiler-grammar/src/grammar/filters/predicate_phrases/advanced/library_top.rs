//! "the top card of your library is <quality>" (Mul Daya Channelers,
//! Vampire Nocturnus, Crown of Convergence): a live test of the card on top
//! of the controller's library (CR 401.1). The whole remainder must read.
use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    if tokens.iter().any(|token| token.as_word().is_none()) {
        return None;
    }
    let words = crate::lexer::token_word_refs(tokens);
    let rest = match words.as_slice() {
        ["the", "top", "card", "of", "your", "library", "is", rest @ ..] => rest,
        _ => return None,
    };
    if rest.is_empty() {
        return None;
    }
    // "is black": a bare color adjective.
    if let [color] = rest
        && let Some(colors) = crate::util::parse_color(color)
    {
        let mut filter = ObjectFilter::default();
        filter.colors = Some(colors);
        filter.zone = Some(crate::zone::Zone::Library);
        return Some(PredicateAst::TopCardOfYourLibraryMatches(filter));
    }
    // "is a creature card", "is an artifact or creature card", "is a Goblin
    // card": an indefinite card noun phrase.
    if !matches!(rest.first(), Some(&("a" | "an"))) {
        return None;
    }
    let start = tokens.len() - rest.len();
    let mut filter = parse_object_filter(&tokens[start..], false).ok()?;
    filter.zone = Some(crate::zone::Zone::Library);
    Some(PredicateAst::TopCardOfYourLibraryMatches(filter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_card_qualities_read_completely() {
        for text in [
            "the top card of your library is a creature card",
            "the top card of your library is black",
            "the top card of your library is an artifact or creature card",
            "the top card of your library is a land card",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(
                matches!(parse(&tokens), Some(PredicateAst::TopCardOfYourLibraryMatches(_))),
                "{text}"
            );
        }
        let tokens = crate::lexer::lex_line("the top card of your library is", 0).unwrap();
        assert!(parse(&tokens).is_none());
    }
}
