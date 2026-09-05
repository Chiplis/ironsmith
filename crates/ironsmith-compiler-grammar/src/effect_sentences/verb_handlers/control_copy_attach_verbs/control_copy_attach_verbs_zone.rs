use super::*;

#[path = "control_copy_attach_verbs_zone/put_clause_readings.rs"]
mod put_clause_readings;

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

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::lexer::token_word_refs(tokens);
    let exiled_with_source_surface = parse_exiled_with_source_move_surface(authored_tokens);
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
