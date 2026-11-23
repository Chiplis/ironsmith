use super::*;

pub fn parse_repeated_tagged_mana_payment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RepeatedTaggedManaPayment> {
    let repeats =
        primitives::find_prefix(tokens, || primitives::phrase(&["for", "each"])).is_some();
    let references_tagged_choice = primitives::find_prefix(tokens, || {
        alt((primitives::kw("those"), primitives::kw("them"))).void()
    })
    .is_some()
        || primitives::find_prefix(tokens, || primitives::phrase(&["chosen", "this", "way"]))
            .is_some();
    if !repeats || !references_tagged_choice {
        return None;
    }

    // "for each <noun> chosen this way": the tagged choice the payment repeats over.
    fn text(token: &OwnedLexToken) -> Option<&str> {
        token.as_word().map(|_| token.parser_text())
    }
    if let Some(at) = tokens.windows(6).position(|window| {
        text(&window[0]) == Some("for")
            && text(&window[1]) == Some("each")
            && text(&window[2]).is_some()
            && text(&window[3]) == Some("chosen")
            && text(&window[4]) == Some("this")
            && text(&window[5]) == Some("way")
    }) {
        crate::word_claims::claim_tokens(&tokens[at..at + 6]);
    }
    // "for each of those <noun>" / "for each of them" closing the payment: the
    // tagged set the payment repeats over.
    let words: Vec<(usize, &str)> = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| text(token).map(|word| (index, word)))
        .collect();
    let tail: Vec<&str> = words.iter().map(|(_, word)| *word).collect();
    let tail_len = match tail.as_slice() {
        [.., "for", "each", "of", "those", _] => 5,
        [.., "for", "each", "of", "them"] => 4,
        _ => 0,
    };
    if tail_len > 0 {
        let start = words[words.len() - tail_len].0;
        crate::word_claims::claim_tokens(&tokens[start..]);
    }

    let mut stream = LexStream::new(tokens);
    let pip_groups =
        crate::grammar::primitives::take_leaf(&mut stream, primitives::collect_mana_pip_groups)?;
    Some(RepeatedTaggedManaPayment { pip_groups })
}
