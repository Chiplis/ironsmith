use super::*;

/// Lift a trailing mana-value qualifier out of a card-type union when Oracle
/// supplies the object noun only once after the final type.
///
/// `instant or sorcery card with mana value ...` gives both type arms the
/// same qualifier. Parsing the arms independently can otherwise leave the
/// comparison only on the final `sorcery card` arm. By contrast,
/// `land card or creature card with mana value ...` repeats the noun and
/// deliberately keeps the qualifier branch-local.
pub(super) fn lift_shared_trailing_mana_value_from_type_union(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    if filter.any_of.is_empty() || filter.mana_value.is_some() {
        return;
    }

    let words = parser_token_word_refs(tokens);
    let Some(mana_idx) = crate::word_primitives::parse_sequence_start(&words, &["mana", "value"])
    else {
        return;
    };
    let Some(connector_idx) =
        crate::slice_primitives::select_last_position(&words[..mana_idx], |word| {
            matches!(*word, "or" | "and/or")
        })
    else {
        return;
    };
    let selector_count = words[..mana_idx]
        .iter()
        .filter_map(|word| parse_card_type(word))
        .collect::<std::collections::HashSet<_>>()
        .len();
    if selector_count < 2 {
        return;
    }
    let is_shared_noun = |word: &&str| {
        matches!(
            *word,
            "card" | "cards" | "spell" | "spells" | "permanent" | "permanents"
        )
    };
    if words[..connector_idx].iter().any(is_shared_noun)
        || words[connector_idx + 1..mana_idx]
            .iter()
            .filter(|word| is_shared_noun(word))
            .count()
            > 1
    {
        return;
    }

    fn collect_mana_value(
        filter: &ObjectFilter,
        shared: &mut Option<crate::filter::Comparison>,
    ) -> bool {
        if let Some(comparison) = &filter.mana_value {
            match shared {
                Some(existing) if existing != comparison => return false,
                Some(_) => {}
                None => *shared = Some(comparison.clone()),
            }
        }
        filter
            .any_of
            .iter()
            .all(|branch| collect_mana_value(branch, shared))
    }

    let mut shared = None;
    if !filter
        .any_of
        .iter()
        .all(|branch| collect_mana_value(branch, &mut shared))
    {
        return;
    }
    let Some(shared) = shared else {
        return;
    };

    fn clear_mana_value(filter: &mut ObjectFilter) {
        filter.mana_value = None;
        for branch in &mut filter.any_of {
            clear_mana_value(branch);
        }
    }
    for branch in &mut filter.any_of {
        clear_mana_value(branch);
    }
    filter.mana_value = Some(shared);

    let shared_zone = filter
        .any_of
        .iter()
        .filter_map(|branch| branch.zone)
        .try_fold(None, |current, zone| match current {
            Some(existing) if existing != zone => None,
            Some(existing) => Some(Some(existing)),
            None => Some(Some(zone)),
        })
        .flatten();
    if filter.zone.is_none() {
        filter.zone = shared_zone;
    }
    let shared_controller = filter
        .any_of
        .iter()
        .filter_map(|branch| branch.controller.clone())
        .try_fold(None, |current, controller| match current {
            Some(existing) if existing != controller => None,
            Some(existing) => Some(Some(existing)),
            None => Some(Some(controller)),
        })
        .flatten();
    if filter.controller.is_none() {
        filter.controller = shared_controller;
    }
    let shared_owner = filter
        .any_of
        .iter()
        .filter_map(|branch| branch.owner.clone())
        .try_fold(None, |current, owner| match current {
            Some(existing) if existing != owner => None,
            Some(existing) => Some(Some(existing)),
            None => Some(Some(owner)),
        })
        .flatten();
    if filter.owner.is_none() {
        filter.owner = shared_owner;
    }
    for branch in &mut filter.any_of {
        if branch.zone == filter.zone {
            branch.zone = None;
        }
        if branch.controller == filter.controller {
            branch.controller = None;
        }
        if branch.owner == filter.owner {
            branch.owner = None;
        }
    }

    let mut card_types = Vec::new();
    for branch in &filter.any_of {
        let [card_type] = branch.card_types.as_slice() else {
            return;
        };
        let mut remainder = branch.clone();
        remainder.card_types.clear();
        remainder.union_surface = Default::default();
        remainder.type_or_subtype_union = false;
        if remainder != ObjectFilter::default() {
            return;
        }
        if !card_types.iter().any(|candidate| candidate == card_type) {
            card_types.push(*card_type);
        }
    }
    if card_types.len() < 2 {
        return;
    }

    filter.card_types = card_types;
    filter.any_of.clear();
    filter.type_or_subtype_union = true;
    filter.set_explicit_card_noun(true);
    filter.set_terminal_noun_after_type_subtype_union_surface(true);
}

pub(super) fn try_apply_distinct_mana_values_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["with", "different", "mana", "values"].as_slice(),
        ["that", "have", "different", "mana", "values"].as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;
        filter.distinct_mana_values = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}
