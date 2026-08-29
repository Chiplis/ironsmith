use super::*;

pub(super) fn positive_relative_characteristic_union(
    words: &[&str],
) -> Option<(
    usize,
    Vec<RelativeCharacteristicSelector>,
    ObjectFilterUnionConnective,
    bool,
)> {
    let (relation_start, characteristic_start) =
        words
            .iter()
            .enumerate()
            .find_map(|(idx, word)| match *word {
                "that's" | "thats" => Some((idx, idx + 1)),
                _ if matches!(
                    words.get(idx..idx + 2),
                    Some(["that", "is"] | ["that", "are"])
                ) =>
                {
                    Some((idx, idx + 2))
                }
                _ => None,
            })?;
    let characteristic_words = words.get(characteristic_start..)?;
    let connective = if characteristic_words.contains(&"and/or") {
        ObjectFilterUnionConnective::AndOr
    } else if characteristic_words.contains(&"or") {
        ObjectFilterUnionConnective::Or
    } else {
        return None;
    };

    let mut selectors = Vec::new();
    let mut selector_occurrences = 0usize;
    let mut selectors_with_articles = 0usize;
    for (idx, word) in characteristic_words.iter().enumerate() {
        if matches!(
            *word,
            "a" | "an" | "the" | "and" | "or" | "and/or" | "card" | "cards"
        ) {
            continue;
        }
        let selector = if matches!(*word, "token" | "tokens") {
            RelativeCharacteristicSelector::Token
        } else if let Some(card_type) = parse_card_type(word) {
            RelativeCharacteristicSelector::CardType(card_type)
        } else if let Some(subtype) = parse_subtype_flexible(word) {
            RelativeCharacteristicSelector::Subtype(subtype)
        } else {
            return None;
        };
        selector_occurrences += 1;
        if idx
            .checked_sub(1)
            .and_then(|previous| characteristic_words.get(previous))
            .is_some_and(|previous| matches!(*previous, "a" | "an"))
        {
            selectors_with_articles += 1;
        }
        if !selectors.contains(&selector) {
            selectors.push(selector);
        }
    }
    (selectors.len() >= 2).then_some((
        relation_start,
        selectors,
        connective,
        selector_occurrences >= 2 && selectors_with_articles == selector_occurrences,
    ))
}

pub(super) fn preserve_relative_characteristic_list_surface(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    let words = parser_token_word_refs(tokens);
    let has_negative_relative_copula = words
        .iter()
        .any(|word| matches!(*word, "isn't" | "isnt" | "aren't" | "arent"))
        || crate::word_primitives::any_sequence_occurs(&words, &[&["is", "not"], &["are", "not"]]);
    if has_negative_relative_copula && filter.subtypes.len() + filter.excluded_subtypes.len() >= 2 {
        filter.set_relative_characteristic_list_surface(true);
        return;
    }

    let Some((relation_start, selectors, connective, explicit_branch_articles)) =
        positive_relative_characteristic_union(&words)
    else {
        return;
    };
    if !filter.any_of.is_empty() {
        return;
    }

    let prefix_words = &words[..relation_start];
    let prefix_card_types = prefix_words
        .iter()
        .filter_map(|word| parse_card_type(word))
        .collect::<Vec<_>>();
    let prefix_subtypes = prefix_words
        .iter()
        .filter_map(|word| parse_subtype_flexible(word))
        .collect::<Vec<_>>();
    let prefix_is_token = prefix_words
        .iter()
        .any(|word| matches!(*word, "token" | "tokens"));

    let mut base = filter.clone();
    for selector in &selectors {
        match selector {
            RelativeCharacteristicSelector::CardType(card_type) => {
                base.card_types.retain(|candidate| candidate != card_type);
                base.all_card_types
                    .retain(|candidate| candidate != card_type);
            }
            RelativeCharacteristicSelector::Subtype(subtype) => {
                base.subtypes.retain(|candidate| candidate != subtype);
            }
            RelativeCharacteristicSelector::Token => base.token = false,
        }
    }
    for card_type in prefix_card_types {
        push_unique(&mut base.card_types, card_type);
    }
    for subtype in prefix_subtypes {
        push_unique(&mut base.subtypes, subtype);
    }
    if prefix_is_token {
        base.token = true;
    }
    base.type_or_subtype_union = false;
    base.set_union_connective(connective);
    base.set_explicit_union_branch_articles(explicit_branch_articles);
    base.set_relative_characteristic_list_surface(true);

    if selectors
        .iter()
        .all(|selector| matches!(selector, RelativeCharacteristicSelector::Subtype(_)))
    {
        for selector in selectors {
            let RelativeCharacteristicSelector::Subtype(subtype) = selector else {
                unreachable!("all relative selectors were checked as subtypes");
            };
            push_unique(&mut base.subtypes, subtype);
        }
        *filter = base;
        return;
    }

    base.any_of = selectors
        .into_iter()
        .map(|selector| match selector {
            RelativeCharacteristicSelector::CardType(card_type) => {
                ObjectFilter::default().with_type(card_type)
            }
            RelativeCharacteristicSelector::Subtype(subtype) => {
                ObjectFilter::default().with_subtype(subtype)
            }
            RelativeCharacteristicSelector::Token => ObjectFilter::default().token(),
        })
        .collect();
    *filter = base;
}

/// Keep a comparison next to the characteristic arm it grammatically
/// qualifies instead of distributing it over the whole inclusive union.
pub(super) fn preserve_branch_scoped_comparison_union(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    if !filter.type_or_subtype_union || !filter.any_of.is_empty() || filter.token {
        return;
    }
    let connector_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("and/or").then_some(idx))
        .collect::<Vec<_>>();
    let [connector_idx] = connector_indices.as_slice() else {
        return;
    };
    let left_tokens = trim_commas(&tokens[..*connector_idx]);
    let right_tokens = trim_commas(&tokens[*connector_idx + 1..]);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return;
    }

    let Ok(left) = parse_object_filter_lexed(&left_tokens, false) else {
        return;
    };
    let Ok(right) = parse_object_filter_lexed(&right_tokens, false) else {
        return;
    };
    if !left.any_of.is_empty()
        || !right.any_of.is_empty()
        || left.card_types.len() + left.subtypes.len() != 1
        || right.card_types.len() + right.subtypes.len() != 1
        || !left.all_card_types.is_empty()
        || !right.all_card_types.is_empty()
        || !left.all_subtypes.is_empty()
        || !right.all_subtypes.is_empty()
    {
        return;
    }

    let power_is_branch_local = filter.power.is_some()
        && ((left.power == filter.power && right.power.is_none())
            || (right.power == filter.power && left.power.is_none()));
    let toughness_is_branch_local = filter.toughness.is_some()
        && ((left.toughness == filter.toughness && right.toughness.is_none())
            || (right.toughness == filter.toughness && left.toughness.is_none()));
    let mana_value_is_branch_local = filter.mana_value.is_some()
        && ((left.mana_value == filter.mana_value && right.mana_value.is_none())
            || (right.mana_value == filter.mana_value && left.mana_value.is_none()));
    if !power_is_branch_local && !toughness_is_branch_local && !mana_value_is_branch_local {
        return;
    }

    let mut outer = filter.clone();
    outer.card_types.clear();
    outer.all_card_types.clear();
    outer.subtypes.clear();
    outer.all_subtypes.clear();
    outer.type_or_subtype_union = false;
    if power_is_branch_local {
        outer.power = None;
    }
    if toughness_is_branch_local {
        outer.toughness = None;
    }
    if mana_value_is_branch_local {
        outer.mana_value = None;
    }

    let mut branches = vec![left, right];
    for branch in &mut branches {
        if branch.zone == outer.zone {
            branch.zone = None;
        }
        if branch.controller == outer.controller {
            branch.controller = None;
        }
        if branch.owner == outer.owner {
            branch.owner = None;
        }
        if outer.other && branch.other {
            branch.other = false;
        }
    }
    outer.any_of = branches;
    *filter = outer;
}

pub(super) fn relation_clause_is_inside_aggregate_scope(
    words: &[&str],
    relation_start: usize,
) -> bool {
    let Some(with_fact) = parse_last_word_choice_before(words, &[WITH_WORD], relation_start) else {
        return false;
    };
    let with_idx = with_fact.index;
    let prefix = &words[with_idx + 1..relation_start];
    let has_aggregate = prefix
        .iter()
        .any(|word| parse_word_choice(word, AGGREGATE_SCOPE_WORDS).is_some());
    let has_scope_marker =
        parse_word_choice_anywhere(prefix, AGGREGATE_SCOPE_MARKER_WORDS).is_some();
    has_aggregate && has_scope_marker
}

pub(super) fn apply_basic_land_exception(filter: &mut ObjectFilter) {
    let mut nonland_branch = filter.clone();
    nonland_branch.any_of.clear();
    push_unique(&mut nonland_branch.excluded_card_types, CardType::Land);

    let mut nonbasic_branch = filter.clone();
    nonbasic_branch.any_of.clear();
    push_unique(&mut nonbasic_branch.excluded_supertypes, Supertype::Basic);

    *filter = ObjectFilter {
        any_of: vec![nonland_branch, nonbasic_branch],
        ..Default::default()
    };
}

pub(super) fn try_apply_distinct_powers_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["with", "different", "powers"].as_slice(),
        ["that", "have", "different", "powers"].as_slice(),
        ["that", "has", "different", "powers"].as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;
        filter.distinct_powers = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

pub(super) fn try_apply_distinct_creature_types_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["that", "share", "no", "creature", "types"].as_slice(),
        ["that", "shares", "no", "creature", "types"].as_slice(),
        ["with", "no", "creature", "types", "in", "common"].as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;
        filter.distinct_creature_types = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}
