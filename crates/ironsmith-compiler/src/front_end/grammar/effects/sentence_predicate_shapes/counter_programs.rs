use super::*;

pub(super) fn parse_counter_reference_where_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["number", "of"]).parse_next(input)?;
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
        primitives::kw("one"),
    )))
    .parse_next(input)?;
    let descriptor = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(counter_noun))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    counter_noun.parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let reference_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let counter_type = if descriptor.is_empty() {
        None
    } else {
        filters::parse_counter_type_from_tokens(descriptor)
    };
    if !descriptor.is_empty() && counter_type.is_none() {
        return Err(primitives::backtrack_err(
            "counter type",
            "known single-word counter type",
        ));
    }

    let reference_words = parser_token_word_refs(reference_tokens);
    let reference = if leaf::parse_leaf_source_anaphor_words(&reference_words).is_some() {
        WhereXReferenceShape::Source
    } else {
        primitives::parse_all(
            reference_tokens,
            (
                alt((primitives::kw("that"), primitives::kw("those"))),
                opt(alt((
                    primitives::kw("card"),
                    primitives::kw("cards"),
                    primitives::kw("creature"),
                    primitives::kw("creatures"),
                    primitives::kw("object"),
                    primitives::kw("objects"),
                    primitives::kw("permanent"),
                    primitives::kw("permanents"),
                ))),
                eof,
            )
                .void(),
            "tagged counter reference",
        )
        .map_err(|_| {
            primitives::backtrack_err("counter reference", "source or tagged object reference")
        })?;
        WhereXReferenceShape::TaggedIt
    };
    Ok(WhereXValueShape::CountersOn {
        reference,
        counter_type,
    })
}
