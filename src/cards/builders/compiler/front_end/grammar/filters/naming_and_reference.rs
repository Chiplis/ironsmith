use super::*;

pub(super) fn remove_word_range(words: &mut Vec<&str>, start: usize, end: usize) {
    let mut remaining = Vec::with_capacity(words.len());
    remaining.extend_from_slice(&words[..start]);
    remaining.extend_from_slice(&words[end..]);
    *words = remaining;
}

pub(super) fn try_apply_not_named_clause<'a, F, G>(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&'a str>,
    all_words_with_articles: &[&'a str],
    map_non_article_index: &F,
    map_non_article_end: &G,
) -> Result<bool, CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let Some(not_named_idx) = find_word_slice_phrase_start(all_words.as_slice(), &["not", "named"])
    else {
        return Ok(false);
    };
    let (name, name_end) = extract_name_clause_text(
        all_words.as_slice(),
        all_words_with_articles,
        not_named_idx,
        2,
        map_non_article_index,
        map_non_article_end,
        "not-named",
    )?;
    filter.excluded_name = Some(name);
    remove_word_range(all_words, not_named_idx, name_end);
    Ok(true)
}

pub(super) fn try_apply_named_clause<'a, F, G>(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&'a str>,
    all_words_with_articles: &[&'a str],
    map_non_article_index: &F,
    map_non_article_end: &G,
) -> Result<bool, CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let Some(named_idx) = lower_words_find_index(all_words.as_slice(), |word| word == "named")
    else {
        return Ok(false);
    };
    let (name, name_end) = extract_name_clause_text(
        all_words.as_slice(),
        all_words_with_articles,
        named_idx,
        1,
        map_non_article_index,
        map_non_article_end,
        "named",
    )?;
    filter.name = Some(name);
    remove_word_range(all_words, named_idx, name_end);
    Ok(true)
}

pub(super) fn parse_entered_since_your_last_turn_ended_words(words: &[&str]) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            primitives::word_slice_eq("entered"),
            primitives::word_slice_eq("since"),
            primitives::word_slice_eq("your"),
            primitives::word_slice_eq("last"),
            primitives::word_slice_eq("turn"),
            primitives::word_slice_eq("ended"),
        )
            .void(),
    ) {
        return Some(consumed);
    }
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("entered"),
            primitives::word_slice_eq("since"),
            primitives::word_slice_eq("your"),
            primitives::word_slice_eq("last"),
            primitives::word_slice_eq("turn"),
            primitives::word_slice_eq("ended"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

pub(super) fn try_apply_entered_since_your_last_turn_ended_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_entered_since_your_last_turn_ended_words,
    ) else {
        return false;
    };
    filter.entered_since_your_last_turn_ended = true;
    all_words.drain(idx..idx + consumed);
    true
}

pub(super) fn strip_object_filter_face_state_words(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) {
    let mut idx = 0usize;
    while idx < all_words.len() {
        let Some((face_down, consumed)) = parse_filter_face_state_words(&all_words[idx..]) else {
            idx += 1;
            continue;
        };
        filter.face_down = Some(face_down);
        all_words.drain(idx..idx + consumed);
    }
}

pub(super) fn strip_single_graveyard_phrase(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) {
    while let Some(idx) =
        find_word_slice_phrase_start(all_words.as_slice(), &["single", "graveyard"])
    {
        filter.single_graveyard = true;
        all_words.remove(idx);
    }
}

pub(super) fn parse_color_count_phrase_words(words: &[&str]) -> Option<(&'static str, usize)> {
    parse_filter_prefix_words(
        words,
        (
            alt((
                primitives::word_slice_eq("one").value("one"),
                primitives::word_slice_eq("two").value("two"),
                primitives::word_slice_eq("three").value("three"),
                primitives::word_slice_eq("four").value("four"),
                primitives::word_slice_eq("five").value("five"),
            )),
            primitives::word_slice_eq("or"),
            primitives::word_slice_eq("more"),
            alt((
                primitives::word_slice_eq("color"),
                primitives::word_slice_eq("colors"),
            )),
        )
            .map(|(count, _, _, _)| count),
    )
}

pub(super) fn try_apply_color_count_phrase(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> Result<bool, CardTextError> {
    let Some((color_count_idx, (count_word, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_color_count_phrase_words(&all_words[idx..]).map(|matched| (idx, matched))
        })
    else {
        return Ok(false);
    };

    if count_word == "one" {
        let any_color: ColorSet = Color::ALL.into_iter().collect();
        filter.colors = Some(any_color);
        all_words.drain(color_count_idx..color_count_idx + consumed);
        return Ok(true);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported color-count object filter (clause: '{}')",
        all_words.join(" ")
    )))
}

pub(super) fn try_apply_pt_literal_prefix(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((power, toughness)) = all_words
        .first()
        .and_then(|word| parse_unsigned_pt_word(word))
    else {
        return false;
    };
    filter.power = Some(crate::filter::Comparison::Equal(power));
    filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
    all_words.remove(0);
    true
}

pub(super) fn parse_not_all_colors_words(words: &[&str]) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            primitives::word_slice_eq("isnt"),
            primitives::word_slice_eq("all"),
            primitives::word_slice_eq("colors"),
        )
            .void(),
    ) {
        return Some(consumed);
    }
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("isnt"),
            primitives::word_slice_eq("all"),
            primitives::word_slice_eq("colors"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

pub(super) fn try_apply_not_all_colors_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_not_all_colors_words)
    else {
        return false;
    };
    filter.all_colors = Some(false);
    all_words.drain(idx..idx + consumed);
    true
}

pub(super) fn parse_not_exactly_two_colors_words(words: &[&str]) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            primitives::word_slice_eq("isnt"),
            primitives::word_slice_eq("exactly"),
            primitives::word_slice_eq("two"),
            primitives::word_slice_eq("colors"),
        )
            .void(),
    ) {
        return Some(consumed);
    }
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("isnt"),
            primitives::word_slice_eq("exactly"),
            primitives::word_slice_eq("two"),
            primitives::word_slice_eq("colors"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

pub(super) fn try_apply_not_exactly_two_colors_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_not_exactly_two_colors_words)
    else {
        return false;
    };
    filter.exactly_two_colors = Some(false);
    all_words.drain(idx..idx + consumed);
    true
}

pub(super) fn parse_mana_value_eq_counters_on_source_words(
    words: &[&str],
) -> Option<(crate::object::CounterType, usize)> {
    let window = words.get(..12)?;
    if window[0] != "with"
        || window[1] != "mana"
        || window[2] != "value"
        || window[3] != "equal"
        || window[4] != "to"
        || window[5] != "number"
        || window[6] != "of"
        || !matches!(window[8], "counter" | "counters")
        || window[9] != "on"
        || window[10] != "this"
        || window[11] != "artifact"
    {
        return None;
    }
    let counter_type = parse_counter_type_word(window[7])?;
    Some((counter_type, 12))
}

pub(super) fn try_apply_mana_value_eq_counters_on_source_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((idx, (counter_type, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_mana_value_eq_counters_on_source_words(&all_words[idx..])
                .map(|matched| (idx, matched))
        })
    else {
        return false;
    };
    filter.mana_value_eq_counters_on_source = Some(counter_type);
    all_words.drain(idx..idx + consumed);

    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();
    let segment_match = find_mana_value_equal_counter_phrase_bounds(&segment_words);
    if let Some((start_word_idx, end_word_idx)) = segment_match
        && let Some(start_token_idx) =
            normalized_token_index_for_word_index(segment_tokens.as_slice(), start_word_idx)
    {
        let end_token_idx =
            normalized_token_index_after_words(segment_tokens.as_slice(), end_word_idx)
                .unwrap_or(segment_tokens.len());
        if start_token_idx < end_token_idx && end_token_idx <= segment_tokens.len() {
            segment_tokens.drain(start_token_idx..end_token_idx);
        }
    }

    true
}

pub(super) fn try_apply_attached_exclusion_phrases(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) {
    let mut idx = 0usize;
    while idx + 2 < all_words.len() {
        if all_words[idx] != "other" || all_words[idx + 1] != "than" {
            idx += 1;
            continue;
        }

        let Some(tag) = (match all_words.get(idx + 2).copied() {
            Some("enchanted") => Some(TagKey::from("enchanted")),
            Some("equipped") => Some(TagKey::from("equipped")),
            _ => None,
        }) else {
            idx += 1;
            continue;
        };

        let mut drain_end = idx + 3;
        if all_words
            .get(drain_end)
            .is_some_and(|word| is_demonstrative_object_head(word))
        {
            drain_end += 1;
        }
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        all_words.drain(idx..drain_end);
    }
}

pub(super) fn strip_object_filter_leading_prefixes(all_words: &mut Vec<&str>) {
    while all_words.len() >= 2 && all_words[0] == "one" && all_words[1] == "of" {
        all_words.drain(0..2);
    }
    while all_words.len() >= 3
        && all_words[0] == "different"
        && all_words[1] == "one"
        && all_words[2] == "of"
    {
        all_words.drain(0..3);
    }
    while all_words
        .first()
        .is_some_and(|word| matches!(*word, "of" | "from"))
    {
        all_words.remove(0);
    }
}

pub(super) fn parse_spell_filter_power_or_toughness_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("power"),
            primitives::word_slice_eq("or"),
            primitives::word_slice_eq("toughness"),
        ),
    )
    .map(|(_, consumed)| consumed)
}

pub(super) fn apply_spell_filter_word_atoms(filter: &mut ObjectFilter, words: &[&str]) {
    let mut idx = 0usize;
    while idx < words.len() {
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            idx += consumed;
            continue;
        }

        let word = words[idx];
        if let Some((face_down, consumed)) = parse_filter_face_state_words(&words[idx..]) {
            filter.face_down = Some(face_down);
            idx += consumed;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            push_unique_filter_value(&mut filter.card_types, card_type);
        }
        if let Some(card_type) = parse_non_type(word) {
            push_unique_filter_value(&mut filter.excluded_card_types, card_type);
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique_filter_value(&mut filter.subtypes, subtype);
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
        idx += 1;
    }
}

pub(super) fn apply_spell_filter_comparisons(
    filter: &mut ObjectFilter,
    words: &[&str],
    clause_words: &[&str],
) {
    let mut cmp_idx = 0usize;
    while cmp_idx < words.len() {
        let Some((axis, axis_word_count)) =
            parse_spell_filter_comparison_axis_words(&words[cmp_idx..])
        else {
            cmp_idx += 1;
            continue;
        };

        let value_tokens = if cmp_idx + axis_word_count < words.len() {
            &words[cmp_idx + axis_word_count..]
        } else {
            &[]
        };
        let parsed = parse_filter_comparison_tokens(axis.as_str(), value_tokens, clause_words)
            .ok()
            .flatten();
        let Some((cmp, consumed)) = parsed else {
            cmp_idx += 1;
            continue;
        };

        axis.assign(filter, cmp);
        cmp_idx += axis_word_count + consumed;
    }
}

pub(super) fn build_spell_filter_power_or_toughness_disjunction(
    filter: &ObjectFilter,
    words: &[&str],
    clause_words: &[&str],
) -> Option<ObjectFilter> {
    for idx in 0..words.len() {
        let Some(consumed) = parse_spell_filter_power_or_toughness_words(&words[idx..]) else {
            continue;
        };
        let value_tokens = if idx + consumed < words.len() {
            &words[idx + consumed..]
        } else {
            &[]
        };
        let Some((cmp, _)) = parse_filter_comparison_tokens("power", value_tokens, clause_words)
            .ok()
            .flatten()
        else {
            continue;
        };

        let mut base = filter.clone();
        base.any_of.clear();
        base.power = None;
        base.toughness = None;

        let mut power_branch = base.clone();
        power_branch.power = Some(cmp.clone());

        let mut toughness_branch = base;
        toughness_branch.toughness = Some(cmp);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![power_branch, toughness_branch];
        return Some(disjunction);
    }

    None
}

pub(super) fn parse_spell_filter_from_words(words: &[&str]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();

    apply_spell_filter_word_atoms(&mut filter, words);
    apply_spell_filter_comparisons(&mut filter, words, words);
    apply_spell_filter_parity_phrases(words, &mut filter);

    build_spell_filter_power_or_toughness_disjunction(&filter, words, words).unwrap_or(filter)
}

pub(super) fn parse_with_no_abilities_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("no"),
            alt((
                primitives::word_slice_eq("ability"),
                primitives::word_slice_eq("abilities"),
            )),
        ),
    )
    .map(|(_, consumed)| consumed)
}

pub(super) fn try_apply_with_clause_tail(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some(consumed) = parse_with_no_abilities_words(words) {
        filter.no_abilities = true;
        return Some(consumed);
    }

    if let Some((_, no_consumed)) =
        parse_filter_prefix_words(words, primitives::word_slice_eq("no"))
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&words[no_consumed..])
    {
        filter.without_counter = Some(counter_constraint);
        return Some(no_consumed + consumed);
    }

    if let Some((kind, consumed)) = parse_alternative_cast_words(words) {
        filter.alternative_cast = Some(kind);
        return Some(consumed);
    }
    if let Some((counter_constraint, consumed)) = parse_filter_counter_constraint_words(words) {
        filter.with_counter = Some(counter_constraint);
        return Some(consumed);
    }

    if let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(words) {
        if let Some((_, or_consumed)) =
            parse_filter_prefix_words(&words[consumed..], primitives::word_slice_eq("or"))
            && let Some((rhs_constraint, rhs_consumed)) =
                parse_filter_keyword_constraint_words(&words[consumed + or_consumed..])
        {
            let mut left = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut left, constraint, false);
            let mut right = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut right, rhs_constraint, false);
            filter.any_of = vec![left, right];
            return Some(consumed + or_consumed + rhs_consumed);
        }

        apply_filter_keyword_constraint(filter, constraint, false);
        return Some(consumed);
    }

    None
}

pub(super) fn try_apply_without_clause_tail(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(words) {
        apply_filter_keyword_constraint(filter, constraint, true);
        return Some(consumed);
    }
    if let Some((counter_constraint, consumed)) = parse_filter_counter_constraint_words(words) {
        filter.without_counter = Some(counter_constraint);
        return Some(consumed);
    }

    None
}

pub(super) fn apply_spell_filter_parity_phrases(words: &[&str], filter: &mut ObjectFilter) {
    for (parity, phrases) in [
        (
            crate::filter::ParityRequirement::Odd,
            &[
                &["odd", "mana", "value"][..],
                &["odd", "mana", "values"][..],
            ][..],
        ),
        (
            crate::filter::ParityRequirement::Even,
            &[
                &["even", "mana", "value"][..],
                &["even", "mana", "values"][..],
            ][..],
        ),
    ] {
        if phrases
            .iter()
            .any(|phrase| find_word_slice_phrase_start(words, phrase).is_some())
        {
            filter.mana_value_parity = Some(parity);
        }
    }

    for (parity, phrases) in [
        (
            crate::filter::ParityRequirement::Odd,
            &[&["odd", "power"][..]][..],
        ),
        (
            crate::filter::ParityRequirement::Even,
            &[&["even", "power"][..]][..],
        ),
    ] {
        if phrases
            .iter()
            .any(|phrase| find_word_slice_phrase_start(words, phrase).is_some())
        {
            filter.power_parity = Some(parity);
        }
    }
}

pub(super) fn contains_any_filter_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| find_word_slice_phrase_start(words, phrase).is_some())
}

pub(super) fn find_any_filter_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    phrases
        .iter()
        .find_map(|phrase| find_word_slice_phrase_start(words, phrase))
}

pub(super) fn find_mana_value_equal_counter_phrase_bounds(
    words: &[&str],
) -> Option<(usize, usize)> {
    (0..words.len()).find_map(|idx| {
        let tail = &words[idx..];
        if tail.len() >= 13
            && find_word_slice_phrase_start(
                tail,
                &[
                    "with", "mana", "value", "equal", "to", "the", "number", "of",
                ],
            ) == Some(0)
            && parse_counter_type_word(tail[8]).is_some()
            && matches!(tail[9], "counter" | "counters")
            && tail[10] == "on"
            && tail[11] == "this"
            && tail[12] == "artifact"
        {
            return Some((idx, idx + 13));
        }
        if tail.len() >= 12
            && find_word_slice_phrase_start(
                tail,
                &["with", "mana", "value", "equal", "to", "number", "of"],
            ) == Some(0)
            && parse_counter_type_word(tail[7]).is_some()
            && matches!(tail[8], "counter" | "counters")
            && tail[9] == "on"
            && tail[10] == "this"
            && tail[11] == "artifact"
        {
            return Some((idx, idx + 12));
        }
        None
    })
}

pub(super) fn contains_filter_word(words: &[&str], word: &str) -> bool {
    find_word_slice_phrase_start(words, &[word]).is_some()
}

pub(super) fn starts_with_any_filter_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| find_word_slice_phrase_start(words, phrase) == Some(0))
}

pub(super) fn attacking_player_filter_from_words(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<PlayerFilter> {
    if contains_any_filter_phrase(
        words,
        &[
            &["attacking", "that", "player"],
            &["attacking", "that", "players"],
        ],
    ) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if contains_any_filter_phrase(
        words,
        &[
            &["attacking", "defending", "player"],
            &["attacking", "defending", "players"],
        ],
    ) {
        return Some(PlayerFilter::Defending);
    }
    if contains_any_filter_phrase(
        words,
        &[
            &["attacking", "target", "player"],
            &["attacking", "target", "players"],
        ],
    ) {
        return Some(PlayerFilter::target_player());
    }
    if contains_any_filter_phrase(
        words,
        &[
            &["attacking", "target", "opponent"],
            &["attacking", "target", "opponents"],
        ],
    ) {
        return Some(PlayerFilter::target_opponent());
    }
    if contains_any_filter_phrase(words, &[&["attacking", "you"]]) {
        return Some(PlayerFilter::You);
    }
    if contains_any_filter_phrase(words, &[&["attacking", "them"]]) {
        return Some(pronoun_player_filter.clone());
    }
    if contains_any_filter_phrase(
        words,
        &[&["attacking", "opponent"], &["attacking", "opponents"]],
    ) {
        return Some(PlayerFilter::Opponent);
    }

    None
}

pub(super) struct ReferenceTagStageResult {
    pub(super) source_linked_exile_reference: bool,
    pub(super) early_return: bool,
}

pub(super) fn apply_reference_and_tag_stage(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> ReferenceTagStageResult {
    if all_words.first().is_some_and(|word| *word == "equipped") {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("equipped"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    } else if all_words.first().is_some_and(|word| *word == "enchanted") {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("enchanted"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    }

    if is_source_reference_words(all_words) {
        filter.source = true;
    }

    if let Some(its_attached_idx) =
        find_word_slice_phrase_start(all_words, &["its", "attached", "to"])
    {
        let mut normalized = Vec::with_capacity(all_words.len() + 1);
        normalized.extend_from_slice(&all_words[..its_attached_idx]);
        normalized.extend(["attached", "to", "it"]);
        normalized.extend_from_slice(&all_words[its_attached_idx + 3..]);
        *all_words = normalized;
    }

    if let Some(attached_idx) = lower_words_find_index(all_words, |word| word == "attached")
        && all_words.get(attached_idx + 1) == Some(&"to")
    {
        let attached_to_words = &all_words[attached_idx + 2..];
        let references_it = starts_with_any_filter_phrase(
            attached_to_words,
            &[
                &["it"],
                &["that", "object"],
                &["that", "creature"],
                &["that", "permanent"],
                &["that", "equipment"],
                &["that", "aura"],
            ],
        );
        if references_it {
            let trim_start = if attached_idx >= 2
                && all_words[attached_idx - 2] == "that"
                && matches!(all_words[attached_idx - 1], "were" | "was" | "is" | "are")
            {
                attached_idx - 2
            } else {
                attached_idx
            };
            all_words.truncate(trim_start);
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: IT_TAG.into(),
                relation: TaggedOpbjectRelation::AttachedToTaggedObject,
            });
        }
    }

    if let Some(relation_idx) = find_any_filter_phrase_start(
        all_words,
        &[
            &["blocking", "or", "blocked", "by", "this", "creature"],
            &["blocking", "or", "blocked", "by", "this", "permanent"],
            &["blocking", "or", "blocked", "by", "this", "source"],
        ],
    ) {
        filter.in_combat_with_source = true;
        all_words.truncate(relation_idx);
    }

    let starts_with_exiled_card =
        starts_with_any_filter_phrase(all_words, &[&["exiled", "card"], &["exiled", "cards"]]);
    if starts_with_exiled_card {
        filter.zone.get_or_insert(Zone::Exile);
    }
    let has_exiled_with_phrase =
        find_word_slice_phrase_start(all_words, &["exiled", "with"]).is_some();
    let owner_only_tail_after_exiled_cards = starts_with_exiled_card
        && all_words
            .iter()
            .skip(2)
            .all(|word| matches!(*word, "you" | "your" | "they" | "their" | "own" | "owns"));
    let is_source_linked_exile_reference = has_exiled_with_phrase
        || (starts_with_exiled_card
            && (all_words.len() == 2 || owner_only_tail_after_exiled_cards));
    let mut source_linked_exile_reference = false;
    if is_source_linked_exile_reference {
        source_linked_exile_reference = true;
        filter.zone = Some(Zone::Exile);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        if let Some(exiled_with_idx) = find_word_slice_phrase_start(all_words, &["exiled", "with"])
        {
            let mut reference_end = exiled_with_idx + 2;
            if all_words
                .get(reference_end)
                .is_some_and(|word| matches!(*word, "this" | "that" | "the" | "it" | "them"))
            {
                reference_end += 1;
            }
            if all_words.get(reference_end).is_some_and(|word| {
                matches!(
                    *word,
                    "artifact" | "creature" | "permanent" | "card" | "spell" | "source"
                )
            }) {
                reference_end += 1;
            }
            if reference_end > exiled_with_idx + 1 {
                all_words.drain(exiled_with_idx + 1..reference_end);
            }
        }
        let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
        let segment_words = segment_words_view.to_word_refs();
        if let Some(exiled_with_idx) =
            find_word_slice_phrase_start(&segment_words, &["exiled", "with"])
            && let Some(exiled_with_token_idx) =
                normalized_token_index_for_word_index(segment_tokens.as_slice(), exiled_with_idx)
        {
            let mut reference_end = exiled_with_token_idx + 2;
            if segment_tokens.get(reference_end).is_some_and(|token| {
                token.is_word("this")
                    || token.is_word("that")
                    || token.is_word("the")
                    || token.is_word("it")
                    || token.is_word("them")
            }) {
                reference_end += 1;
            }
            if segment_tokens.get(reference_end).is_some_and(|token| {
                token.is_word("artifact")
                    || token.is_word("creature")
                    || token.is_word("permanent")
                    || token.is_word("card")
                    || token.is_word("spell")
                    || token.is_word("source")
            }) {
                reference_end += 1;
            }
            if reference_end > exiled_with_idx + 1 {
                segment_tokens.drain(exiled_with_token_idx + 1..reference_end);
            }
        }
    }

    if all_words
        .first()
        .is_some_and(|word| *word == "it" || *word == "them")
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        if all_words.len() == 1 {
            return ReferenceTagStageResult {
                source_linked_exile_reference,
                early_return: true,
            };
        }
        all_words.remove(0);
    }

    let has_share_card_type = (contains_filter_word(all_words, "share")
        || contains_filter_word(all_words, "shares"))
        && (contains_filter_word(all_words, "card")
            || contains_filter_word(all_words, "permanent"))
        && contains_filter_word(all_words, "type")
        && contains_filter_word(all_words, "it");
    let has_share_color = contains_filter_word(all_words, "shares")
        && contains_filter_word(all_words, "color")
        && contains_filter_word(all_words, "it");
    let has_same_mana_value =
        contains_any_filter_phrase(all_words, &[&["same", "mana", "value", "as"]]);
    let has_equal_or_lesser_mana_value =
        contains_any_filter_phrase(all_words, &[&["equal", "or", "lesser", "mana", "value"]]);
    let has_lte_mana_value_as_tagged = contains_any_filter_phrase(
        all_words,
        &[
            &[
                "equal", "or", "lesser", "mana", "value", "than", "that", "spell",
            ],
            &[
                "equal", "or", "lesser", "mana", "value", "than", "that", "card",
            ],
            &[
                "equal", "or", "lesser", "mana", "value", "than", "that", "object",
            ],
            &[
                "less", "than", "or", "equal", "to", "that", "spells", "mana", "value",
            ],
            &[
                "less", "than", "or", "equal", "to", "that", "cards", "mana", "value",
            ],
            &[
                "less", "than", "or", "equal", "to", "that", "objects", "mana", "value",
            ],
        ],
    ) || has_equal_or_lesser_mana_value;
    let has_lt_mana_value_as_tagged =
        contains_any_filter_phrase(all_words, &[&["lesser", "mana", "value"]])
            && !has_equal_or_lesser_mana_value;
    let references_sacrifice_cost_object = contains_any_filter_phrase(
        all_words,
        &[
            &["the", "sacrificed", "creature"],
            &["the", "sacrificed", "artifact"],
            &["the", "sacrificed", "permanent"],
            &["a", "sacrificed", "creature"],
            &["a", "sacrificed", "artifact"],
            &["a", "sacrificed", "permanent"],
            &["sacrificed", "creature"],
            &["sacrificed", "artifact"],
            &["sacrificed", "permanent"],
        ],
    );
    let references_it_for_mana_value = all_words.iter().any(|word| matches!(*word, "it" | "its"))
        || contains_any_filter_phrase(
            all_words,
            &[
                &["that", "object"],
                &["that", "creature"],
                &["that", "artifact"],
                &["that", "permanent"],
                &["that", "spell"],
                &["that", "card"],
            ],
        );
    let has_same_name_as_tagged_object = contains_any_filter_phrase(
        all_words,
        &[
            &["same", "name", "as", "that", "spell"],
            &["same", "name", "as", "that", "card"],
            &["same", "name", "as", "that", "object"],
            &["same", "name", "as", "that", "creature"],
            &["same", "name", "as", "that", "permanent"],
        ],
    );

    if has_share_card_type {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesCardType,
        });
    }
    if has_share_color {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesColorWithTagged,
        });
    }
    if has_same_mana_value && references_sacrifice_cost_object {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("sacrifice_cost_0"),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    } else if has_same_mana_value && references_it_for_mana_value {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    }
    if has_lte_mana_value_as_tagged
        && (references_it_for_mana_value || has_equal_or_lesser_mana_value)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::ManaValueLteTagged,
        });
    }
    if has_lt_mana_value_as_tagged {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::ManaValueLtTagged,
        });
    }
    if has_same_name_as_tagged_object {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    }

    if contains_any_filter_phrase(
        all_words,
        &[
            &["that", "convoked", "this", "spell"],
            &["that", "convoked", "it"],
        ],
    ) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("convoked_this_spell"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if contains_any_filter_phrase(all_words, &[&["that", "crewed", "it", "this", "turn"]]) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("crewed_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if contains_any_filter_phrase(all_words, &[&["that", "saddled", "it", "this", "turn"]]) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("saddled_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if contains_any_filter_phrase(
        all_words,
        &[
            &["army", "you", "amassed"],
            &["amassed", "army"],
            &["amassed", "armys"],
        ],
    ) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if contains_any_filter_phrase(
        all_words,
        &[
            &["exiled", "this", "way"],
            &["destroyed", "this", "way"],
            &["sacrificed", "this", "way"],
            &["revealed", "this", "way"],
            &["discarded", "this", "way"],
            &["milled", "this", "way"],
        ],
    ) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    ReferenceTagStageResult {
        source_linked_exile_reference,
        early_return: false,
    }
}
