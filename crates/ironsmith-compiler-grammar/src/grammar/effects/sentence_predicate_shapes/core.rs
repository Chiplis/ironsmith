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

    if let Some(metric) = parsed_metric
        && exact_exiled_card_reference(reference_tokens)
    {
        let metric = match metric {
            EffectMetric::FirstPower => WhereXMetricShape::Power,
            EffectMetric::FirstToughness => WhereXMetricShape::Toughness,
            EffectMetric::FirstManaValue => WhereXMetricShape::ManaValue,
            _ => unreachable!("only characteristic metrics are parsed above"),
        };
        return Ok(WhereXValueShape::SourceExiledCharacteristic(metric));
    }
    let source = prior_effect_source(reference_tokens)
        .ok_or_else(|| primitives::backtrack_err("prior effect reference", "remembered objects"))?;
    let metric = parsed_metric.unwrap_or(EffectMetric::Count);
    // A cost or a resolution instruction may supply this receipt. Preserve
    // the exact counter kind; the shared resolver binds the actual removal,
    // which can differ from an announced quantity after a replacement.
    if parsed_metric.is_none()
        && let Some(counter_type) = removed_counters_this_way(reference_tokens)
    {
        let mut query = PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::Count)
            .with_action(ironsmith_core::PriorEffectAction::Removed);
        query.counter_type = counter_type;
        return Ok(WhereXValueShape::PriorEffectMetric(query));
    }
    let reference_words = parser_token_word_refs(reference_tokens);
    if reference_words.iter().any(|word| matches!(*word, "counter" | "counters"))
        && reference_words.contains(&"removed")
    {
        return Err(primitives::backtrack_err("removed-counter quantity", "complete typed counter descriptor and this-way scope"));
    }
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
