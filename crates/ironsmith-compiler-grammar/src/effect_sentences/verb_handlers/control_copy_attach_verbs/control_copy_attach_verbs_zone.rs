use super::*;

#[path = "control_copy_attach_verbs_zone/put_clause_readings.rs"]
mod put_clause_readings;

fn split_and_or_articled_card_pair(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let is_article = |token: &OwnedLexToken| token.is_any_word(&["a", "an"]);
    if !tokens.first().is_some_and(is_article) {
        return None;
    }
    let and_or = tokens.iter().position(|token| token.is_word("and/or"))?;
    if and_or < 2
        || !tokens[and_or - 1].is_word("card")
        || !tokens.get(and_or + 1).is_some_and(is_article)
    {
        return None;
    }
    let second_card = and_or
        + 1
        + tokens[and_or + 1..]
            .iter()
            .position(|token| token.is_word("card"))?;
    let tail = &tokens[second_card + 1..];
    if !tail.first().is_some_and(|token| token.is_word("from"))
        || !tail.iter().any(|token| token.is_word("onto"))
    {
        return None;
    }
    let up_to_one = || crate::lexer::synthetic_word_tokens(["up", "to", "one"]);
    let mut first = up_to_one();
    first.extend_from_slice(&tokens[1..and_or]);
    first.extend_from_slice(tail);
    let mut second = up_to_one();
    second.extend_from_slice(&tokens[and_or + 2..]);
    Some((first, second))
}

pub fn parse_put_into_hand(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    if let Some(choice) = parse_put_destination_choice(tokens, subject)? {
        return Ok(choice);
    }
    let authored_tokens = tokens;
    let tokens = if tokens
        .first()
        .is_some_and(|token| token.is_word("put") || token.is_word("puts"))
    {
        &tokens[1..]
    } else {
        tokens
    };

    // The source zone may follow the destination: "put a card onto the
    // battlefield from their hand". Move the complete bounded location in
    // front of `onto` so every native target/filter reader sees the same
    // source restriction. Do not accept arbitrary destination residue.
    if let Some(onto) = tokens.iter().position(|token| token.is_word("onto"))
        && let Some(from) = tokens.iter().enumerate().skip(onto + 1)
            .find_map(|(index, token)| token.is_word("from").then_some(index))
    {
        let words = crate::lexer::token_word_refs(&tokens[from..]);
        let source_location = matches!(words.as_slice(),
            ["from", "your" | "their", "hand" | "graveyard" | "library"]
            | ["from", "that", "player" | "players" | "player's", "hand" | "graveyard" | "library"]
        );
        if source_location {
            let source_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[from..]);
            if source_tokens.iter()
                .any(|token| token.as_word().is_none())
            {
                return Err(CardTextError::ParseError("unsupported token in battlefield source location".into()));
            }
            let mut reordered = tokens[..onto].to_vec();
            reordered.extend_from_slice(source_tokens);
            reordered.extend_from_slice(&tokens[onto..from]);
            return parse_put_into_hand(&reordered, subject);
        }
    }

    // "put a creature card and/or a land card from your hand onto the
    // battlefield" (Yuna's Decision): each articled card is its own optional
    // selection, so either or both may be put.
    if let Some((first, second)) = split_and_or_articled_card_pair(tokens) {
        return Ok(EffectAst::Sequence {
            effects: vec![
                parse_put_into_hand(&first, subject.clone())?,
                parse_put_into_hand(&second, subject)?,
            ],
        });
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::lexer::token_word_refs(tokens);
    // The subject/verb dispatcher may already have consumed `put`. Preserve
    // the same source-linked move wording on both entry paths.
    let exiled_with_source_surface = parse_exiled_with_source_move_surface(authored_tokens)
        .or_else(|| {
            parse_exiled_with_source_move_surface_inner(
                tokens,
                Some(ironsmith_core::ExiledWithSourceMoveVerbSurface::Put),
            )
        });
    let input = put_clause_readings::PutClause {
        tokens,
        player,
        subject,
        clause_words: &clause_words,
        exiled_with_source_surface: &exiled_with_source_surface,
        authored_tokens,
        read_by_cache: Default::default(),
    };
    match put_clause_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    if cca_shapes::contains_sticker(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported sticker clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported put clause (clause: '{}')",
        clause_words.join(" ")
    )))
}
