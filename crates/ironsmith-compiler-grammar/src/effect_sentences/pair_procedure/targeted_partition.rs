//! A declared pair, a resolution-time choice, and its captured complement.
use super::*;
use crate::cards::builders::ObjectChoiceEffectAst;

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else {
        return Ok(None);
    };
    let first_tokens = first.lowered();
    if !first_tokens
        .first()
        .is_some_and(|token| token.is_word("choose"))
    {
        return Ok(None);
    }
    let tokens = second.lowered();
    let Some(and) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    let head = &tokens[..and];
    let words = crate::lexer::parser_token_word_refs(head);
    if head.len() != words.len()
        || !matches!(
            words.as_slice(),
            ["exile", "one", "of", "those", "creatures"]
                | ["exile", "one", "of", "those", "permanents"]
        )
    {
        return Ok(None);
    }
    let tail = &tokens[and + 1..];
    let tail = if tail
        .last()
        .is_some_and(|t| t.kind == crate::lexer::TokenKind::Period)
    {
        &tail[..tail.len() - 1]
    } else {
        tail
    };
    if tail.len() < 4
        || !tail.first().is_some_and(|t| t.is_word("put"))
        || !tail[tail.len() - 3..]
            .iter()
            .zip(["on", "the", "other"])
            .all(|(t, w)| t.is_word(w))
    {
        return Ok(None);
    }
    let target = crate::util::parse_target_phrase(&first_tokens[1..])?;
    let TargetAst::WithCount(inner, count) = &target else {
        return Ok(None);
    };
    let TargetAst::Object(filter, Some(_), _) = inner.as_ref() else {
        return Ok(None);
    };
    if words.last() == Some(&"creatures")
        && !filter
            .card_types
            .contains(&crate::types::CardType::Creature)
    {
        return Ok(None);
    }
    if *count != ChoiceCount::exactly(2) || filter.zone.is_some_and(|z| z != Zone::Battlefield) {
        return Ok(None);
    }
    let Some(descriptor) =
        crate::grammar::effects::zone_counter_shapes::parse_counter_descriptor_shape(
            &tail[1..tail.len() - 3],
        )
    else {
        return Ok(None);
    };
    let pool = helper_tag_for_tokens(first_tokens, "announced_pool");
    let chosen = helper_tag_for_tokens(tokens, "chosen_subset");
    let remainder = helper_tag_for_tokens(tokens, "unselected_remainder");
    let mut effects = vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
            tag: pool.clone(),
        },
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: ObjectFilter::tagged(pool.clone()),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen.clone(),
            zone: Zone::Battlefield,
        }),
        // Capture the complement before exile replacements can change the
        // chosen object's destination or leave it on the battlefield.
        EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(pool).not_tagged(chosen.clone()),
            vec![Zone::Battlefield],
            remainder.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(chosen, None),
            Zone::Exile,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];
    effects.push(EffectAst::subject_verb_put_counters(
        descriptor.counter_type,
        Value::Fixed(descriptor.count as i32),
        TargetAst::Tagged(remainder, None),
        None,
        false,
    ));
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targeted_partition_requires_the_complete_pair_reference() {
        for (first, second, accepted) in [
            (
                "Choose two target creatures controlled by the same player",
                "Exile one of those creatures and put two +1/+1 counters on the other",
                true,
            ),
            (
                "Choose two target creatures",
                "Exile one of those creatures and put three -1/-1 counters on the other",
                true,
            ),
            (
                "Choose three target creatures",
                "Exile one of those creatures and put two +1/+1 counters on the other",
                false,
            ),
            (
                "Choose two target artifacts",
                "Exile one of those creatures and put two +1/+1 counters on the other",
                false,
            ),
            (
                "Choose two target creatures",
                "Exile one of those creatures and put two +1/+1 counters on any other target",
                false,
            ),
            (
                "Choose two target creatures",
                "Exile one of those creatures and put two +1/+1 counters on the other and draw a card",
                false,
            ),
            (
                "Choose two target creatures",
                "Exile one of those creatures and put two +1/+1 counters on the other {G}",
                false,
            ),
        ] {
            let first = crate::lexer::lex_line(first, 0).unwrap();
            let second = crate::lexer::lex_line(second, 0).unwrap();
            let result = read(
                &[
                    SentenceInput::from_lexed(&first),
                    SentenceInput::from_lexed(&second),
                ],
                0,
            )
            .unwrap();
            assert_eq!(result.is_some(), accepted);
        }
    }
}
