//! Per-unit life quantities are numeric instruction counts, not object filters.
//! Single actions select/apply one batch; an unless-payment program repeats its
//! decisions with a count frozen before the first iteration.
use crate::cards::builders::*;
use crate::effect::{ChoiceCount, Value};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

pub(crate) fn is_life_unit_count(value: &Value) -> bool {
    match value.unhinted() {
        Value::PendingPriorEffectMetric(query) => {
            query.source == ironsmith_core::EffectMetricSource::Outcome
                && matches!(
                    query.metric,
                    ironsmith_core::EffectMetric::LifeGained
                        | ironsmith_core::EffectMetric::LifeLost
                )
                && query.action.is_none()
                && query.filter.is_none()
                && query.counter_type.is_none()
        }
        Value::Scaled(inner, _) | Value::DividedRoundedDown(inner, _) => is_life_unit_count(inner),
        _ => false,
    }
}
fn scale(count: Value, multiplier: u32) -> Option<Value> {
    let multiplier = i32::try_from(multiplier).ok()?;
    Some(if multiplier == 1 {
        count
    } else {
        Value::Scaled(Box::new(count), multiplier)
    })
}
pub(crate) fn batch_sacrifice(
    tokens: &[OwnedLexToken],
    sacrifice: &EffectAst,
    count: Value,
) -> Option<EffectAst> {
    let EffectAst::SubjectVerb(subject) = sacrifice else {
        return None;
    };
    let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
        filter,
        count: each,
        target: None,
        ..
    }) = &subject.action
    else {
        return None;
    };
    let player = if subject.subject.player == PlayerAst::Implicit {
        PlayerAst::You
    } else {
        subject.subject.player
    };
    let participant = match player {
        PlayerAst::Implicit | PlayerAst::You => PlayerFilter::You,
        PlayerAst::That => PlayerFilter::IteratedPlayer,
        _ => return None,
    };
    let mut filter = filter.clone();
    filter.zone = Some(Zone::Battlefield);
    filter.controller = Some(participant);
    let tag = crate::util::helper_tag_for_tokens(tokens, "sacrificed");
    Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::dynamic_x(),
                count_value: Some(scale(count, *each)?),
                player,
                tag: crate::tag::TagRef::of(tag.clone()),
            }),
            EffectAst::subject_verb_sacrifice_all(player, ObjectFilter::tagged(tag)),
        ],
    })
}
fn mixed_permanent_private_card_exile(
    tokens: &[OwnedLexToken],
    count: Value,
) -> Option<Vec<EffectAst>> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let zones = match words.as_slice() {
        [
            "exile",
            "a",
            "permanent",
            "you",
            "control",
            "or",
            "a",
            "card",
            "from",
            "your",
            "hand",
            "or",
            "graveyard",
        ]
        | [
            "exile",
            "a",
            "permanent",
            "you",
            "control",
            "or",
            "a",
            "card",
            "from",
            "your",
            "graveyard",
            "or",
            "hand",
        ] => vec![Zone::Battlefield, Zone::Hand, Zone::Graveyard],
        _ => return None,
    };
    let filter = ObjectFilter {
        any_of: vec![
            ObjectFilter::permanent().controlled_by(PlayerFilter::You),
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You),
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        ],
        ..ObjectFilter::default()
    };
    let tag = crate::util::helper_tag_for_tokens(tokens, "exiled");
    Some(vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::dynamic_x(),
            count_value: Some(count),
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(tag.clone()),
            zones,
            search_mode: None,
        }),
        EffectAst::subject_verb_exile(TargetAst::Tagged(crate::tag::TagRef::of(tag), None), false),
    ])
}
pub(crate) fn parse_prefix(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if crate::lexer::split_lexed_sentences(tokens).len() != 1 {
        return Ok(None);
    }
    let words = crate::lexer::parser_token_word_refs(tokens);
    let Some((count, used)) = crate::util::parse_for_each_count_value_words(&words) else {
        return Ok(None);
    };
    if !is_life_unit_count(&count) {
        return Ok(None);
    }
    let positions = crate::lexer::parser_token_word_positions(tokens);
    let Some(&(body_start, _)) = positions.get(used) else {
        return Ok(None);
    };
    let quantity_end = positions[used - 1].0 + 1;
    if !tokens[quantity_end..body_start]
        .iter()
        .any(|token| token.is_comma())
    {
        return Ok(None);
    }
    let body = &tokens[body_start..];
    if let Some(effects) = mixed_permanent_private_card_exile(body, count.clone()) {
        return Ok(Some(effects));
    }
    let effects = crate::clause_support::parse_effect_sentences_lexed(body)?;
    if effects.is_empty() {
        return Ok(None);
    }
    if body.iter().any(|token| token.is_word("unless")) {
        return Ok(Some(vec![EffectAst::ForEach(
            ForEachEffectAst::RepeatEffects { count, effects },
        )]));
    }
    if let [effect] = effects.as_slice()
        && let Some(batch) = batch_sacrifice(tokens, effect, count.clone())
    {
        return Ok(Some(vec![batch]));
    }
    Err(CardTextError::ParseError(
        "per-unit life instruction has no simultaneous action lowering".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unit_life_domain_is_numeric_and_complete() {
        let words = ["for", "each", "1", "life", "you", "lost"];
        let (count, used) = crate::util::parse_for_each_count_value_words(&words).unwrap();
        assert_eq!(used, words.len());
        assert!(is_life_unit_count(&count));
        let Value::PendingPriorEffectMetric(query) = count.unhinted() else {
            panic!("lost typed query");
        };
        assert_eq!(query.metric, ironsmith_core::EffectMetric::LifeLost);
        assert_eq!(query.player, Some(PlayerFilter::You));
        assert!(
            crate::util::parse_for_each_count_value_words(&[
                "for", "each", "0", "life", "you", "lost"
            ])
            .is_none()
        );
    }
    #[test]
    fn exile_is_one_cross_zone_selection_and_unless_is_a_frozen_repeat() {
        let exile = crate::lexer::lex_line("For each 1 life you lost, exile a permanent you control or a card from your hand or graveyard.", 0).unwrap();
        let effects = parse_prefix(&exile).unwrap().unwrap();
        assert!(
            matches!(&effects[0], EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { zones, count_value: Some(_), .. }) if zones == &[Zone::Battlefield, Zone::Hand, Zone::Graveyard])
        );
        assert_eq!(effects.len(), 2);
        let unless = crate::lexer::lex_line("For each 1 life you lost, sacrifice a permanent other than this enchantment unless you discard a card.", 0).unwrap();
        let effects = parse_prefix(&unless).unwrap().unwrap();
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatEffects { .. })]
        ));
    }
}
