use super::*;

fn retarget_prefix<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    alt((
        primitives::kw("and").void(),
        primitives::comma().void(),
        primitives::end_of_sentence(),
    ))
    .parse_next(input)?;
    opt(alt((
        primitives::kw("you").void(),
        primitives::phrase(&["that", "player"]),
    )))
    .parse_next(input)?;
    let may = opt(primitives::kw("may")).parse_next(input)?.is_some();
    primitives::kw("choose").parse_next(input)?;
    Ok(may)
}

fn boundary_before<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> Option<(usize, O)>
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    let (prefix, parsed): (Vec<&OwnedLexToken>, O) =
        repeat_till(0.., any, parser).parse_next(&mut input).ok()?;
    Some((prefix.len(), parsed))
}

fn word_boundary(tokens: &[OwnedLexToken], expected: &'static str) -> Option<usize> {
    boundary_before(tokens, primitives::kw(expected)).map(|(index, _)| index)
}

fn word_slice_boundary(words: &[&str], expected: &'static str) -> Option<usize> {
    let mut input = words;
    let (((), ()), prefix) = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::word_slice_exact(expected).void(),
    )
    .with_taken()
    .parse_next(&mut input)
    .ok()?;
    Some(prefix.len())
}

pub fn parse_copy_tail_shape_tokens(tokens: &[OwnedLexToken]) -> CopyTailShape {
    let retarget = boundary_before(tokens, retarget_prefix)
        .and_then(|(index, may)| {
            let tail = tokens.get(index + 1..).unwrap_or_default();
            let has_target = marker_anywhere(tail, primitives::kw("target"));
            let has_targets = marker_anywhere(tail, primitives::kw("targets"));
            ((has_target || has_targets) && marker_anywhere(tail, primitives::kw("copy")))
                .then_some((index, may, has_target && !has_targets))
        })
        .or_else(|| {
            // A period is also an authored boundary between the copy action
            // and its retarget permission. Parse the following sentence as a
            // complete retarget clause so punctuation cannot hide it from the
            // prefix scanner.
            tokens.iter().enumerate().find_map(|(index, token)| {
                token.is_period().then(|| {
                    parse_copy_retarget_shape_tokens(tokens.get(index + 1..).unwrap_or_default())
                        .map(|shape| (index, shape.may_choose, shape.single_target))
                })?
            })
        });
    CopyTailShape {
        retarget_split: retarget.map(|(index, _, _)| index),
        retarget_may: retarget.is_some_and(|(_, may, _)| may),
        retarget_single_target: retarget.is_some_and(|(_, _, single)| single),
        exception_split: word_boundary(tokens, "except"),
        then_split: word_boundary(tokens, "then"),
        for_each_split: boundary_before(tokens, primitives::phrase(&["for", "each"]))
            .map(|(index, _)| index),
    }
}

pub fn parse_copy_clause_shape_tokens(tokens: &[OwnedLexToken]) -> Option<CopyClauseShape> {
    let copy_word = boundary_before(
        tokens,
        alt((primitives::kw("copy"), primitives::kw("copies"))),
    )?
    .0;
    let exception_word = word_slice_boundary(&parser_token_word_refs(tokens), "except");
    let before_exception = exception_word.map_or(tokens, |index| {
        let words = parser_token_word_refs(tokens);
        let token_end = words
            .get(index..)
            .and_then(|_| crate::lexer::TokenWordView::new(tokens).token_boundary_for_word(index))
            .unwrap_or(tokens.len());
        &tokens[..token_end]
    });
    let simple_reference = copy_word == 0
        && (primitives::parse_prefix(
            tokens.get(1..).unwrap_or_default(),
            alt((
                primitives::kw("it"),
                primitives::kw("this"),
                primitives::kw("that"),
            )),
        )
        .is_some()
            || primitives::parse_all(
                tokens,
                alt((
                    (
                        primitives::phrase(&["copy", "that", "card"]),
                        primitives::sentence_end(),
                    )
                        .void(),
                    (
                        primitives::phrase(&["copy", "the", "exiled", "card"]),
                        primitives::sentence_end(),
                    )
                        .void(),
                )),
                "simple copy reference",
            )
            .is_ok());
    Some(CopyClauseShape {
        copy_word,
        exception_word,
        emblem_with: marker_anywhere(tokens, primitives::phrase(&["emblem", "with"])),
        simple_reference,
        mentions_spell_or_ability: marker_anywhere(
            before_exception,
            alt((
                primitives::kw("spell"),
                primitives::kw("spells"),
                primitives::kw("ability"),
                primitives::kw("abilities"),
            )),
        ),
        removed_legendary: marker_anywhere(tokens, primitives::kw("legendary"))
            && marker_anywhere(
                tokens,
                alt((primitives::kw("except"), primitives::kw("isnt"))),
            ),
        tail: parse_copy_tail_shape_tokens(tokens.get(copy_word + 1..).unwrap_or_default()),
    })
}

fn copy_target_reference<'a>(input: &mut LexStream<'a>) -> WResult<CopyTargetShape<'a>> {
    alt((
        (
            primitives::phrase(&["this", "spell"]),
            primitives::sentence_end(),
        )
            .value(CopyTargetShape::Source),
        (
            alt((
                primitives::phrase(&["that", "spell", "or", "ability"]),
                primitives::phrase(&["that", "ability", "or", "spell"]),
                primitives::phrase(&["that", "spell"]),
            )),
            primitives::sentence_end(),
        )
            .value(CopyTargetShape::Triggering),
        (
            primitives::phrase(&["that", "ability"]),
            primitives::sentence_end(),
        )
            .value(CopyTargetShape::TriggeringSource),
        (
            alt((
                primitives::kw("it").void(),
                primitives::kw("that").void(),
                primitives::phrase(&["that", "card"]),
            )),
            primitives::sentence_end(),
        )
            .value(CopyTargetShape::TaggedIt),
        (
            primitives::phrase(&["the", "exiled", "card"]),
            primitives::sentence_end(),
        )
            .value(CopyTargetShape::PriorExiledCard),
    ))
    .parse_next(input)
}

pub fn parse_copy_target_shape_tokens(tokens: &[OwnedLexToken]) -> CopyTargetShape<'_> {
    let trimmed = trim_lexed_commas(tokens);
    primitives::parse_all(trimmed, copy_target_reference, "copy target reference")
        .unwrap_or(CopyTargetShape::Explicit(trimmed))
}

fn parse_copy_retarget_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CopyRetargetShape> {
    opt(alt((
        primitives::kw("you").void(),
        primitives::phrase(&["that", "player"]),
    )))
    .parse_next(input)?;
    let may_choose = opt(primitives::kw("may")).parse_next(input)?.is_some();
    primitives::kw("choose").parse_next(input)?;
    let tail = input.as_ref();
    if !marker_anywhere(
        tail,
        alt((primitives::kw("target"), primitives::kw("targets"))),
    ) || !marker_anywhere(tail, primitives::kw("copy"))
    {
        return Err(primitives::backtrack_err(
            "copy retarget",
            "targets and copy",
        ));
    }
    let has_new = marker_anywhere(tail, primitives::kw("new"));
    let has_target = marker_anywhere(tail, primitives::kw("target"));
    let has_targets = marker_anywhere(tail, primitives::kw("targets"));
    while any::<_, ErrMode<ContextError>>.parse_next(input).is_ok() {}
    Ok(CopyRetargetShape {
        may_choose,
        has_new,
        single_target: has_target && !has_targets,
    })
}

pub fn parse_copy_retarget_shape_tokens(tokens: &[OwnedLexToken]) -> Option<CopyRetargetShape> {
    primitives::parse_all(
        trim_lexed_commas(tokens),
        parse_copy_retarget_lexed,
        "copy retarget",
    )
    .ok()
    .or_else(|| {
        // Sentence splitting can leave authored punctuation around this
        // follow-up. Recover the same narrow shape from normalized words
        // without requiring the whole token slice to be one parser block.
        let words = parser_token_word_refs(tokens);
        let choose = words.iter().position(|word| *word == "choose")?;
        if choose > 3
            || words[..choose]
                .iter()
                .any(|word| !matches!(*word, "you" | "that" | "player" | "may"))
        {
            return None;
        }
        let tail = words.get(choose + 1..)?;
        let has_target = tail.contains(&"target");
        let has_targets = tail.contains(&"targets");
        if !(has_target || has_targets)
            || !tail.iter().any(|word| matches!(*word, "copy" | "copies"))
        {
            return None;
        }
        Some(CopyRetargetShape {
            may_choose: words[..choose].contains(&"may"),
            has_new: tail.contains(&"new"),
            single_target: has_target && !has_targets,
        })
    })
}
