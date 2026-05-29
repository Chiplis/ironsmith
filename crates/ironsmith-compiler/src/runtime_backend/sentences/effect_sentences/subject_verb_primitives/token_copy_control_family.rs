use super::*;
use crate::runtime_backend::lexer::{
    word_slice_contains_word, word_slice_eq, word_slice_eq_any, word_slice_starts_with,
};

pub(crate) fn parse_sentence_each_player_return_with_additional_counter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_return_with_additional_counter_sentence(clause)
}

pub(crate) fn parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_comma_segments();
    if segments.len() != 3 {
        return Ok(None);
    }

    let reveal_clause = segments[0];
    let reveal_words = reveal_clause.word_refs();
    let reveal_prefix = [
        "each", "player", "reveals", "a", "number", "of", "cards", "from", "the", "top", "of",
        "their", "library", "equal", "to",
    ];
    if !reveal_words.starts_with(&reveal_prefix) {
        return Ok(None);
    }
    let Some(count_clause) = reveal_clause.after_words(reveal_prefix.len()) else {
        return Ok(None);
    };
    let mut synthetic_where_clause =
        SubjectVerbPrimitiveOwnedClause::synthetic_words(&["where", "x", "is"]);
    synthetic_where_clause.append_clause(count_clause);
    let Some(count) = parse_value_binding_clause(synthetic_where_clause.tokens()) else {
        return Ok(None);
    };

    let put_clause = segments[1];
    if put_clause
        .strip_prefix(&["puts", "all", "permanent", "cards"])
        .is_none()
        || !put_clause.contains_phrase(&["revealed", "this", "way"])
        || !put_clause.contains_phrase(&["onto", "the", "battlefield"])
    {
        return Ok(None);
    }

    let rest_words = segments[2].without_leading_connectors_clause().word_refs();
    let rest_words = rest_words.as_slice();
    if !word_slice_eq(
        rest_words,
        &["puts", "the", "rest", "into", "their", "graveyard"],
    ) {
        return Ok(None);
    }

    let revealed_tag_key = helper_tag_for_tokens(clause.tokens(), "revealed");
    let iterated_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::That,
                count,
                revealed_tag_key.clone(),
            ),
            EffectAst::subject_verb_reveal_tagged(revealed_tag_key.clone()),
            EffectAst::ForEachTagged {
                tag: revealed_tag_key,
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::ItMatches(ObjectFilter::permanent_card()),
                    if_true: vec![EffectAst::subject_verb_move_to_zone(
                        iterated_target.clone(),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Owner,
                        false,
                        None,
                    )],
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        iterated_target,
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            },
        ],
    }]))
}

pub(crate) fn parse_return_then_do_same_for_subtypes_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause.first_is_word("return") {
        return Ok(None);
    }
    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let tail_words = tail_clause.word_refs();
    if tail_clause
        .strip_prefix(&["do", "the", "same", "for"])
        .is_none()
    {
        return Ok(None);
    }
    let subtype_words = &tail_words[4..];
    if subtype_words.is_empty() {
        return Ok(None);
    }

    let mut extra_subtypes = Vec::new();
    for word in subtype_words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        extra_subtypes.push(subtype);
    }
    if extra_subtypes.is_empty() {
        return Ok(None);
    }

    let mut effects = parse_effect_chain(head_clause.tokens())?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let base_effect = effects[0].clone();
    for subtype in extra_subtypes {
        let Some(cloned) = clone_return_effect_with_subtype(&base_effect, subtype) else {
            return Ok(None);
        };
        effects.push(cloned);
    }

    Ok(Some(effects))
}

fn split_choose_same_followup_filters(filter: &ObjectFilter) -> Vec<ObjectFilter> {
    match filter.mana_value.clone() {
        Some(crate::filter::Comparison::OneOf(values)) if !values.is_empty() => values
            .into_iter()
            .map(|value| {
                let mut cloned = filter.clone();
                cloned.mana_value = Some(crate::filter::Comparison::Equal(value));
                cloned
            })
            .collect(),
        _ => vec![filter.clone()],
    }
}

pub(crate) fn parse_choose_then_do_same_for_filter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause.first_is_word("choose") {
        return Ok(None);
    }
    let Some((head_clause, tail_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    if tail_clause
        .strip_prefix(&["do", "the", "same", "for"])
        .is_none()
    {
        return Ok(None);
    }

    let followup_filter_clause = tail_clause.from(4);
    if followup_filter_clause.is_empty() {
        return Ok(None);
    }

    let Some((player, base_filter, count)) = parse_you_choose_objects_clause(head_clause.tokens())?
        .or_else(|| {
            parse_target_player_choose_objects_clause(head_clause.tokens())
                .ok()
                .flatten()
        })
    else {
        return Ok(None);
    };
    let tag = TagKey::from(IT_TAG);

    let followup_filter = parse_object_filter(followup_filter_clause.tokens(), false)?;
    if followup_filter.controller.is_some() || followup_filter.owner.is_some() {
        return Ok(None);
    }

    let merged_filter = merge_filters(&base_filter, &followup_filter);
    let followup_filters = split_choose_same_followup_filters(&merged_filter);
    if followup_filters.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::ChooseObjects {
        filter: base_filter.clone(),
        count: count.clone(),
        count_value: None,
        player: player.clone(),
        tag: tag.clone(),
    }];
    for filter in followup_filters {
        effects.push(EffectAst::ChooseObjects {
            filter,
            count: count.clone(),
            count_value: None,
            player: player.clone(),
            tag: tag.clone(),
        });
    }

    Ok(Some(effects))
}

fn parse_choose_objects_clause_for_chain(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    if let Some(parsed) = clause.parse_value_with_lexed(parse_you_choose_objects_clause)? {
        return Ok(Some(parsed));
    }
    clause.parse_value_with_lexed(parse_target_player_choose_objects_clause)
}

fn choose_clause_trails_from_it(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    matches!(
        words.as_slice(),
        [.., "from", "it"] | [.., "from", "them"] | [.., "in", "it"] | [.., "in", "them"]
    )
}

fn preserve_choose_clause_it_reference(
    clause: SubjectVerbPrimitiveClause<'_>,
    filter: &mut ObjectFilter,
) {
    if !choose_clause_trails_from_it(clause) {
        return;
    }
    if filter.zone.is_none() || filter.zone == Some(Zone::Battlefield) {
        filter.zone = Some(Zone::Hand);
    }
    filter.controller = None;
    filter.owner = None;
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

pub(crate) fn parse_choose_then_choose_objects_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let Some((first_player, mut first_filter, first_count)) =
        parse_choose_objects_clause_for_chain(head_clause)?
    else {
        return Ok(None);
    };
    let Some((mut second_player, mut second_filter, second_count)) =
        parse_choose_objects_clause_for_chain(tail_clause)?
    else {
        return Ok(None);
    };

    preserve_choose_clause_it_reference(head_clause, &mut first_filter);
    preserve_choose_clause_it_reference(tail_clause, &mut second_filter);

    if second_player == PlayerAst::Implicit {
        second_player = first_player.clone();
    }

    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: first_filter,
            count: first_count,
            count_value: None,
            player: first_player,
            tag: tag.clone(),
        },
        EffectAst::ChooseObjects {
            filter: second_filter,
            count: second_count,
            count_value: None,
            player: second_player,
            tag,
        },
    ]))
}

pub(crate) fn parse_sentence_return_then_do_same_for_subtypes(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_then_do_same_for_subtypes_sentence(clause)
}

pub(crate) fn parse_sentence_choose_then_choose_objects(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_choose_objects_sentence(clause)
}

pub(crate) fn parse_sentence_choose_then_do_same_for_filter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_do_same_for_filter_sentence(clause)
}

pub(crate) fn parse_sacrifice_any_number_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        if head.is_empty() {
            return Ok(None);
        }
        (head, Some(tail))
    } else {
        (clause, None)
    };

    if !head_clause.first_is_word("sacrifice") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if !(head_clause
        .token(idx)
        .is_some_and(|token| token.is_word("any"))
        && head_clause
            .token(idx + 1)
            .is_some_and(|token| token.is_word("number")))
    {
        return Ok(None);
    }
    idx += 2;
    if head_clause
        .token(idx)
        .is_some_and(|token| token.is_word("of"))
    {
        idx += 1;
    }
    if idx >= head_clause.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            clause.text()
        )));
    }

    let filter_clause = head_clause.from(idx).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            clause.text()
        )));
    }

    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    let tag = TagKey::from(IT_TAG);

    let mut effects = vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_sacrifice_all(PlayerAst::Implicit, ObjectFilter::tagged(tag)),
    ];
    if let Some(tail_clause) = tail_clause
        && !tail_clause.is_empty()
    {
        let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
        effects.append(&mut tail_effects);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_sacrifice_any_number(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_any_number_sentence(clause)
}

pub(crate) fn parse_sacrifice_one_or_more_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause.first_is_word("sacrifice") {
        return Ok(None);
    }

    let mut idx = 1usize;
    let Some((minimum, used)) = parse_number(clause.from(idx).tokens()) else {
        return Ok(None);
    };
    idx += used;
    if !(clause.token(idx).is_some_and(|token| token.is_word("or"))
        && clause
            .token(idx + 1)
            .is_some_and(|token| token.is_word("more")))
    {
        return Ok(None);
    }
    idx += 2;
    if clause.token(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }
    if idx >= clause.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            clause.text()
        )));
    }

    let filter_clause = clause.from(idx).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            clause.text()
        )));
    }
    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::at_least(minimum as usize),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_sacrifice_all(PlayerAst::Implicit, ObjectFilter::tagged(tag)),
    ]))
}

pub(crate) fn parse_sentence_sacrifice_one_or_more(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_one_or_more_sentence(clause)
}

pub(crate) fn parse_sentence_keyword_then_chain(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };

    let Some(head_effect) = parse_keyword_mechanic_clause(head_clause.tokens())? else {
        return Ok(None);
    };

    if tail_clause.is_empty() {
        return Ok(Some(vec![head_effect]));
    }

    let mut effects = vec![head_effect];
    if let Some(mut counter_effects) = parse_sentence_put_counter_sequence(tail_clause)? {
        effects.append(&mut counter_effects);
        return Ok(Some(effects));
    }

    let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
    effects.append(&mut tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_chain_then_keyword(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let Some(keyword_effect) = parse_keyword_mechanic_clause(tail_clause.tokens())? else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    head_effects.push(keyword_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_return_then_create(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = clause.split_once_on_then_trimmed();
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    if head_slice.is_empty() || tail_slice.is_empty() {
        return Ok(None);
    }

    if !head_slice.first_is_word("return") || !tail_slice.first_is_word("create") {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_slice.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(tail_slice.tokens())?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_exile_then_may_put_from_exile(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = clause.split_once_on_then_trimmed();
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    if head_slice.is_empty() || tail_slice.is_empty() {
        return Ok(None);
    }

    if tail_slice
        .strip_prefix(&["you", "may", "put", "any", "number", "of"])
        .is_none()
        || !tail_slice.contains_word("from")
        || !tail_slice.contains_word("exile")
        || !tail_slice.contains_word("battlefield")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_slice.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    let mut tail_effects = parse_effect_chain(tail_slice.tokens())?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_then_shuffle_graveyard_into_library_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = clause.split_once_on_then_trimmed();
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    if head_slice.is_empty() || tail_slice.is_empty() {
        return Ok(None);
    }

    let head_words = head_slice.word_refs();
    if !head_words.first().is_some_and(|word| *word == "exile")
        && !(head_words.first().is_some_and(|word| *word == "you")
            && head_words.get(1).is_some_and(|word| *word == "exile"))
    {
        return Ok(None);
    }

    let tail_words = tail_slice.word_refs();
    if !tail_words
        .first()
        .is_some_and(|word| *word == "shuffle" || *word == "shuffles")
    {
        return Ok(None);
    }
    if !tail_words
        .iter()
        .any(|word| *word == "graveyard" || *word == "graveyards")
        || !tail_words
            .iter()
            .any(|word| *word == "library" || *word == "libraries")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_slice.tokens())?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Exile { .. },
                ..
            }) | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileAll { .. },
                ..
            }) | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileUntilSourceLeaves { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(tail_slice.tokens())?;
    if !tail_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ShuffleGraveyardIntoLibrary,
                ..
            })
        )
    }) {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_source_with_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "exile <source> with <counter descriptor> on it/them"
    let Some(after_exile) = clause.strip_prefix_clause(&["exile"]) else {
        return Ok(None);
    };
    let Some((source_name_clause, counter_clause)) = after_exile.split_once_on_word("with") else {
        return Ok(None);
    };

    let source_name_clause = source_name_clause.trimmed();
    if source_name_clause.is_empty() {
        return Ok(None);
    }
    let source_name_words = source_name_clause.word_refs();
    if !is_likely_named_or_source_reference_words(&source_name_words) {
        return Ok(None);
    }

    let counter_clause = counter_clause.trimmed();
    let Some(on_idx) = counter_clause.rfind_token_word("on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause.len() {
        return Ok(None);
    }

    let on_target_words = counter_clause.from(on_idx + 1).word_refs();
    if !word_slice_eq_any(&on_target_words, &[&["it"], &["them"]]) {
        return Ok(None);
    }

    let descriptor_clause = counter_clause.before(on_idx).trimmed();
    if descriptor_clause.is_empty() {
        return Ok(None);
    }
    let (count, counter_type) = parse_counter_descriptor(descriptor_clause.tokens())?;

    let source_target = TargetAst::Source(clause.span());
    Ok(Some(vec![
        EffectAst::subject_verb_exile(source_target.clone(), false),
        EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            source_target,
            None,
            false,
        ),
    ]))
}

pub(crate) fn parse_sentence_exile_source_with_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_source_with_counters_sentence(clause)
}

pub(crate) fn parse_sentence_comma_then_chain_special(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word: &&str| !word.is_empty())
            .collect()
    }

    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let head_word_storage = head_clause.word_refs();
    let tail_word_storage = tail_clause.word_refs();
    let head_words = normalize_words(&head_word_storage);
    let tail_words = normalize_words(&tail_word_storage);
    let is_that_player_tail = word_slice_starts_with(&tail_words, &["that", "player"]);
    let is_return_source_tail = word_slice_starts_with(&tail_words, &["return", "this"])
        && word_slice_contains_word(&tail_words, "owner")
        && word_slice_contains_word(&tail_words, "hand");
    let is_put_source_on_top_of_library_tail =
        word_slice_starts_with(&tail_words, &["put", "this"])
            && word_slice_contains_word(&tail_words, "top")
            && word_slice_contains_word(&tail_words, "owner")
            && tail_words.last().copied() == Some("library");
    let is_choose_card_name_tail = crate::runtime_backend::lexer::word_slice_starts_with_any(
        &tail_words,
        &[
            &["choose", "any", "card", "name"],
            &["choose", "a", "card", "name"],
        ],
    ) && head_words.first().copied() == Some("look");
    if !is_that_player_tail
        && !is_return_source_tail
        && !is_put_source_on_top_of_library_tail
        && !is_choose_card_name_tail
    {
        return Ok(None);
    }
    if is_return_source_tail
        && !head_words
            .first()
            .is_some_and(|word| matches!(*word, "tap" | "untap"))
    {
        return Ok(None);
    }
    if is_put_source_on_top_of_library_tail
        && !head_words.first().is_some_and(|word| *word == "draw")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_destroy_then_land_controller_graveyard_count_damage_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let tail_words = tail_clause.word_refs();
    let suffix = [
        "damage",
        "to",
        "that",
        "lands",
        "controller",
        "equal",
        "to",
        "the",
        "number",
        "of",
        "land",
        "cards",
        "in",
        "that",
        "players",
        "graveyard",
    ];
    let Some(suffix_start) = tail_clause.find_phrase_start(&suffix) else {
        return Ok(None);
    };
    if suffix_start == 0 || !matches!(tail_words[suffix_start - 1], "deal" | "deals") {
        return Ok(None);
    }
    if suffix_start + suffix.len() != tail_words.len() {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Destroy { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    let mut count_filter = ObjectFilter::default();
    count_filter.zone = Some(Zone::Graveyard);
    let tagged_ref = crate::target::ObjectRef::tagged(IT_TAG);
    count_filter.owner = Some(PlayerFilter::ControllerOf(tagged_ref.clone()));
    count_filter.card_types.push(CardType::Land);
    head_effects.push(EffectAst::subject_verb_damage(
        Value::Count(count_filter),
        TargetAst::Player(PlayerFilter::ControllerOf(tagged_ref), tail_clause.span()),
    ));
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_destroy_all_attached_to_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "destroy all/each <filter> attached to <target>"
    if !clause.first_is_word("destroy") {
        return Ok(None);
    }
    if !clause
        .token(1)
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        return Ok(None);
    }

    let Some((filter_clause, target_clause)) =
        clause.from(2).split_once_on_phrase(&["attached", "to"])
    else {
        return Ok(None);
    };

    let filter_clause =
        filter_clause.without_trailing_words_clause(&["that", "were", "was", "is", "are"]);
    let target_clause = target_clause.trimmed();
    let has_timing_tail = target_clause.contains_any_word(&[
        "at",
        "beginning",
        "end",
        "combat",
        "turn",
        "step",
        "until",
    ]);
    const SUPPORTED_THAT_TARGET_PREFIXES: &[&[&str]] = &[
        &["that", "creature"],
        &["that", "permanent"],
        &["that", "land"],
        &["that", "artifact"],
        &["that", "enchantment"],
    ];

    let supported_target = target_clause.first_is_word("target")
        || target_clause.contains_word("you") && target_clause.len() == 1
        || target_clause.contains_word("it") && target_clause.len() == 1
        || target_clause
            .strip_any_prefix(SUPPORTED_THAT_TARGET_PREFIXES)
            .is_some();
    if filter_clause.is_empty() || target_clause.is_empty() || !supported_target || has_timing_tail
    {
        return Ok(None);
    }

    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    let target = parse_target_phrase(target_clause.tokens())?;
    Ok(Some(vec![EffectAst::subject_verb_destroy_all_attached_to(
        filter, target,
    )]))
}

pub(crate) fn parse_sentence_destroy_then_land_controller_graveyard_count_damage(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_then_land_controller_graveyard_count_damage_sentence(clause)
}

pub(crate) fn find_creature_type_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const CREATURE_TYPE_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "creature", "type", "of", "your", "choice"],
        &["of", "creature", "type", "of", "your", "choice"],
    ];
    clause.find_any_phrase_span(CREATURE_TYPE_CHOICE_PHRASES)
}

pub(super) fn find_type_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const TYPE_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "chosen", "type"],
        &["of", "chosen", "type"],
        &["of", "that", "type"],
        &["that", "type"],
    ];
    find_creature_type_choice_phrase(clause)
        .or_else(|| clause.find_any_phrase_span(TYPE_CHOICE_PHRASES))
}

pub(crate) fn find_color_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const COLOR_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "color", "of", "your", "choice"],
        &["of", "the", "color", "of", "their", "choice"],
        &["of", "color", "of", "your", "choice"],
        &["of", "color", "of", "their", "choice"],
    ];
    clause.find_any_phrase_span(COLOR_CHOICE_PHRASES)
}
