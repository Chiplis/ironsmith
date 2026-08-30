use super::*;

pub(super) fn parse_prior_effect_where_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    let parsed_metric = alt((
        primitives::kw("power").value(Some(EffectMetric::FirstPower)),
        primitives::kw("toughness").value(Some(EffectMetric::FirstToughness)),
        primitives::phrase(&["mana", "value"]).value(Some(EffectMetric::FirstManaValue)),
        primitives::phrase(&["number", "of"]).value(None),
    ))
    .parse_next(input)?;
    if parsed_metric.is_some() {
        primitives::kw("of").parse_next(input)?;
    }
    let reference_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    if parsed_metric == Some(EffectMetric::FirstManaValue)
        && exact_exiled_card_reference(reference_tokens)
    {
        return Ok(WhereXValueShape::SourceExiledManaValue);
    }
    let source = prior_effect_source(reference_tokens)
        .ok_or_else(|| primitives::backtrack_err("prior effect reference", "remembered objects"))?;
    let metric = parsed_metric.unwrap_or(EffectMetric::Count);
    // Counter removal can be an activation cost rather than a preceding effect.
    // In that established shape, X is supplied by the cost payment itself; do
    // not turn it into a pending prior-effect query that has no producer to
    // bind to during reference resolution.
    if parsed_metric.is_none() && removed_counters_this_way(reference_tokens) {
        return Ok(WhereXValueShape::RemovedCountersThisWay);
    }
    let reference_words = parser_token_word_refs(reference_tokens);
    if let Some(this_way_start) =
        crate::word_primitives::parse_sequence_start(&reference_words, &["this", "way"])
    {
        let subject = &reference_words[..this_way_start];
        if let Some((action, action_start)) =
            crate::grammar::shared_util::value_helper_shapes::parse_prior_effect_action(subject)
        {
            let filter_words = &subject[..action_start];
            let query_source = if matches!(action, ironsmith_core::PriorEffectAction::Chosen) {
                EffectMetricSource::ChosenObjects
            } else if filter_words
                .iter()
                .any(|word| matches!(*word, "counter" | "counters" | "damage"))
            {
                EffectMetricSource::Outcome
            } else {
                source
            };
            let mut query = PriorEffectMetricQuery::new(query_source, metric).with_action(action);
            if !filter_words.is_empty()
                && !filter_words
                    .iter()
                    .any(|word| matches!(*word, "counter" | "counters" | "damage"))
            {
                let mut filter =
                    crate::object_filters::parse_object_filter_words(filter_words, false).map_err(
                        |_| {
                            primitives::backtrack_err(
                                "prior effect filter",
                                "object filter over remembered objects",
                            )
                        },
                    )?;
                if filter_words
                    .iter()
                    .any(|word| matches!(*word, "card" | "cards"))
                {
                    filter.set_explicit_card_noun(true);
                }
                query = query.with_filter(filter);
            }
            if matches!(action, ironsmith_core::PriorEffectAction::Destroyed)
                && subject.last() == Some(&"died")
            {
                return Ok(WhereXValueShape::DiedThisWayMetric(query));
            }
            return Ok(WhereXValueShape::PriorEffectMetric(query));
        }
    }
    Ok(WhereXValueShape::PriorEffectMetric(
        PriorEffectMetricQuery::new(source, metric),
    ))
}
