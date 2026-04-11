use winnow::Parser;
use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode};

use crate::cards::builders::{CardTextError, IT_TAG};
use crate::effects::VOTE_WINNERS_TAG;
use crate::filter::ParityRequirement;
use crate::{
    CardType, Color, ColorSet, ObjectFilter, PlayerFilter, Supertype, TagKey,
    TaggedObjectConstraint, TaggedOpbjectRelation, Zone,
};

use super::grammar::primitives::{self as grammar_primitives, split_lexed_slices_on_or};
use super::keyword_static::parse_pt_modifier;
use super::lexer::{OwnedLexToken, TokenWordView};
use super::util::{
    apply_filter_keyword_constraint, is_article, is_demonstrative_object_head, is_non_outlaw_word,
    is_outlaw_word, is_permanent_type, is_source_reference_words, parse_alternative_cast_words,
    parse_card_type, parse_color, parse_counter_type_word, parse_filter_counter_constraint_words,
    parse_filter_keyword_constraint_words, parse_non_color, parse_non_subtype, parse_non_supertype,
    parse_non_type, parse_subtype_flexible, parse_subtype_word, parse_supertype_word,
    parse_unsigned_pt_word, parse_zone_word, push_outlaw_subtypes, token_index_for_word_index,
    trim_commas,
};
use super::value_helpers::parse_filter_comparison_tokens;

pub(super) fn normalized_token_index_for_word_index(
    tokens: &[OwnedLexToken],
    word_idx: usize,
) -> Option<usize> {
    token_index_for_word_index(tokens, word_idx)
}

pub(super) fn normalized_token_index_after_words(
    tokens: &[OwnedLexToken],
    word_count: usize,
) -> Option<usize> {
    if word_count == 0 {
        return Some(0);
    }

    let word_view = TokenWordView::new(tokens);
    let word_refs = word_view.to_word_refs();
    if word_count > word_refs.len() {
        return None;
    }

    token_index_for_word_index(tokens, word_count)
        .or_else(|| (word_count == word_refs.len()).then_some(tokens.len()))
}

use grammar_primitives::{
    WordSliceInput, parse_full_word_slice, parse_prefix_word_slice, word_slice_eq,
};

type WordInput<'a> = WordSliceInput<'a>;

fn word_slice_match<'a>(
    expected: &str,
) -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> + '_ {
    move |input: &mut WordInput<'a>| {
        let Some((word, rest)) = input.split_first() else {
            return Err(grammar_primitives::backtrack_err("word", "matching word"));
        };
        if word.eq_ignore_ascii_case(expected) {
            *input = rest;
            Ok(())
        } else {
            Err(grammar_primitives::backtrack_err("word", "matching word"))
        }
    }
}

pub(super) fn word_slice_phrase<'a, 'b>(
    expected: &'b [&'b str],
) -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> + 'b {
    move |input: &mut WordInput<'a>| {
        for word in expected {
            word_slice_match(word).parse_next(input)?;
        }
        Ok(())
    }
}

pub(super) fn word_slice_any_phrase<'a, 'b>(
    phrases: &'b [&'b [&'b str]],
) -> impl Parser<WordInput<'a>, &'b [&'b str], ErrMode<ContextError>> + 'b {
    move |input: &mut WordInput<'a>| {
        for phrase in phrases {
            let mut probe = *input;
            if word_slice_phrase(*phrase).parse_next(&mut probe).is_ok() {
                *input = probe;
                return Ok(*phrase);
            }
        }

        Err(grammar_primitives::backtrack_err(
            "word phrase choice",
            "one of the expected word phrases",
        ))
    }
}

pub(super) fn find_word_slice_parse<'a, O, P, F>(
    words: &'a [&'a str],
    make_parser: F,
) -> Option<(usize, O, usize)>
where
    F: Fn() -> P + Copy,
    P: Parser<WordInput<'a>, O, ErrMode<ContextError>>,
{
    (0..words.len()).find_map(|idx| {
        let mut input = &words[idx..];
        let parsed = make_parser().parse_next(&mut input).ok()?;
        Some((idx, parsed, words.len().saturating_sub(idx + input.len())))
    })
}

pub(super) fn find_word_slice_phrase_start(words: &[&str], phrase: &[&str]) -> Option<usize> {
    find_word_slice_parse(words, || word_slice_phrase(phrase).void()).map(|(idx, _, _)| idx)
}

fn contains_any_word_slice_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| find_word_slice_phrase_start(words, phrase).is_some())
}

#[derive(Clone)]
enum SimpleObjectFilterSuffix {
    Controller(PlayerFilter),
    Owner(PlayerFilter),
    OwnerZone(PlayerFilter, Zone),
    Zone(Zone),
}

#[derive(Clone, Copy)]
enum NamedObjectFilterWordAtom {
    ChosenColor,
    ChosenType,
    NonChosenType,
}

pub(super) fn parse_filter_face_state_words(words: &[&str]) -> Option<(bool, usize)> {
    parse_prefix_word_slice(
        words,
        alt((
            alt((word_slice_eq("face-down"), word_slice_eq("facedown"))).value((true, 1usize)),
            alt((word_slice_eq("face-up"), word_slice_eq("faceup"))).value((false, 1usize)),
            (word_slice_eq("face"), word_slice_eq("down")).value((true, 2usize)),
            (word_slice_eq("face"), word_slice_eq("up")).value((false, 2usize)),
        )),
    )
}

fn parse_named_object_filter_word_atom(
    words: &[&str],
) -> Option<(NamedObjectFilterWordAtom, usize)> {
    parse_prefix_word_slice(
        words,
        alt((
            (word_slice_eq("chosen"), word_slice_eq("color"))
                .value((NamedObjectFilterWordAtom::ChosenColor, 2usize)),
            (word_slice_eq("chosen"), word_slice_eq("type"))
                .value((NamedObjectFilterWordAtom::ChosenType, 2usize)),
            (word_slice_eq("nonchosen"), word_slice_eq("type"))
                .value((NamedObjectFilterWordAtom::NonChosenType, 2usize)),
        )),
    )
}

fn parse_simple_object_filter_suffix_inner<'a>(
    input: &mut WordInput<'a>,
) -> Result<SimpleObjectFilterSuffix, ErrMode<ContextError>> {
    alt((
        alt((
            (word_slice_eq("you"), word_slice_eq("control"))
                .value(SimpleObjectFilterSuffix::Controller(PlayerFilter::You)),
            (
                word_slice_eq("you"),
                alt((word_slice_eq("dont"), word_slice_eq("don't"))),
                word_slice_eq("control"),
            )
                .value(SimpleObjectFilterSuffix::Controller(PlayerFilter::NotYou)),
            (
                word_slice_eq("you"),
                word_slice_eq("do"),
                word_slice_eq("not"),
                word_slice_eq("control"),
            )
                .value(SimpleObjectFilterSuffix::Controller(PlayerFilter::NotYou)),
            alt((
                (word_slice_eq("opponents"), word_slice_eq("control")),
                (word_slice_eq("opponent"), word_slice_eq("controls")),
            ))
            .value(SimpleObjectFilterSuffix::Controller(PlayerFilter::Opponent)),
            (word_slice_eq("you"), word_slice_eq("own"))
                .value(SimpleObjectFilterSuffix::Owner(PlayerFilter::You)),
        )),
        alt((
            (
                alt((word_slice_eq("in"), word_slice_eq("from"))),
                word_slice_eq("your"),
                word_slice_eq("graveyard"),
            )
                .value(SimpleObjectFilterSuffix::OwnerZone(
                    PlayerFilter::You,
                    Zone::Graveyard,
                )),
            (
                alt((word_slice_eq("in"), word_slice_eq("from"))),
                word_slice_eq("your"),
                word_slice_eq("hand"),
            )
                .value(SimpleObjectFilterSuffix::OwnerZone(
                    PlayerFilter::You,
                    Zone::Hand,
                )),
            (
                alt((word_slice_eq("in"), word_slice_eq("from"))),
                word_slice_eq("your"),
                word_slice_eq("library"),
            )
                .value(SimpleObjectFilterSuffix::OwnerZone(
                    PlayerFilter::You,
                    Zone::Library,
                )),
        )),
        alt((
            (
                alt((word_slice_eq("in"), word_slice_eq("from"))),
                word_slice_eq("graveyard"),
            )
                .value(SimpleObjectFilterSuffix::Zone(Zone::Graveyard)),
            (
                alt((word_slice_eq("in"), word_slice_eq("from"))),
                word_slice_eq("hand"),
            )
                .value(SimpleObjectFilterSuffix::Zone(Zone::Hand)),
            (
                alt((word_slice_eq("in"), word_slice_eq("from"))),
                word_slice_eq("library"),
            )
                .value(SimpleObjectFilterSuffix::Zone(Zone::Library)),
            (
                alt((word_slice_eq("in"), word_slice_eq("from"))),
                word_slice_eq("exile"),
            )
                .value(SimpleObjectFilterSuffix::Zone(Zone::Exile)),
        )),
    ))
    .parse_next(input)
}

fn parse_simple_object_filter_suffix(words: &[&str]) -> Option<(SimpleObjectFilterSuffix, usize)> {
    for suffix_len in [4usize, 3, 2] {
        let tail = words.get(words.len().checked_sub(suffix_len)?..)?;
        if let Some(parsed) = parse_full_word_slice(tail, parse_simple_object_filter_suffix_inner) {
            return Some((parsed, suffix_len));
        }
    }
    None
}

pub(super) fn lower_words_find_index(
    words: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    for (idx, word) in words.iter().enumerate() {
        if predicate(word) {
            return Some(idx);
        }
    }
    None
}

pub(super) fn token_find_index(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        if predicate(token) {
            return Some(idx);
        }
    }
    None
}

pub(super) fn slice_has<T: PartialEq>(items: &[T], expected: &T) -> bool {
    items.iter().any(|item| item == expected)
}

pub(super) fn set_has<T: Eq + std::hash::Hash>(
    items: &std::collections::HashSet<T>,
    expected: &T,
) -> bool {
    items.iter().any(|item| item == expected)
}

pub(super) fn push_unique<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    if !slice_has(items, &value) {
        items.push(value);
    }
}

pub(super) fn strip_not_on_battlefield_phrase(tokens: &mut Vec<OwnedLexToken>) -> bool {
    let patterns: &[&[&str]] = &[
        &["that", "arent", "on", "the", "battlefield"],
        &["that", "aren't", "on", "the", "battlefield"],
        &["that", "isnt", "on", "the", "battlefield"],
        &["that", "isn't", "on", "the", "battlefield"],
        &["that", "are", "not", "on", "the", "battlefield"],
        &["that", "is", "not", "on", "the", "battlefield"],
        &["arent", "on", "the", "battlefield"],
        &["aren't", "on", "the", "battlefield"],
        &["isnt", "on", "the", "battlefield"],
        &["isn't", "on", "the", "battlefield"],
        &["are", "not", "on", "the", "battlefield"],
        &["is", "not", "on", "the", "battlefield"],
    ];

    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let Some((word_start, matched_phrase, _)) =
        find_word_slice_parse(&words, || word_slice_any_phrase(patterns))
    else {
        return false;
    };
    let Some(token_start) = normalized_token_index_for_word_index(tokens, word_start) else {
        return false;
    };
    let token_end =
        normalized_token_index_for_word_index(tokens, word_start + matched_phrase.len())
            .unwrap_or(tokens.len());
    tokens.drain(token_start..token_end);
    true
}

pub(super) fn trim_vote_winner_suffix(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let suffix = [
        "with", "most", "votes", "or", "tied", "for", "most", "votes",
    ];
    let Some(suffix_start) = words.len().checked_sub(suffix.len()) else {
        return (tokens.to_vec(), false);
    };
    if parse_full_word_slice(&words[suffix_start..], word_slice_phrase(&suffix)).is_none() {
        return (tokens.to_vec(), false);
    }

    let Some(token_end) = normalized_token_index_for_word_index(tokens, suffix_start) else {
        return (tokens.to_vec(), false);
    };
    (trim_commas(&tokens[..token_end]), true)
}

pub(super) fn apply_parity_filter_phrases(words: &[&str], filter: &mut ObjectFilter) {
    for (parity, phrases) in [
        (
            ParityRequirement::Odd,
            &[
                &["odd", "mana", "value"][..],
                &["odd", "mana", "values"][..],
            ][..],
        ),
        (
            ParityRequirement::Even,
            &[
                &["even", "mana", "value"][..],
                &["even", "mana", "values"][..],
            ][..],
        ),
    ] {
        if contains_any_word_slice_phrase(words, phrases) {
            filter.mana_value_parity = Some(parity);
        }
    }

    for (parity, phrases) in [
        (ParityRequirement::Odd, &[&["odd", "power"][..]][..]),
        (ParityRequirement::Even, &[&["even", "power"][..]][..]),
    ] {
        if contains_any_word_slice_phrase(words, phrases) {
            filter.power_parity = Some(parity);
        }
    }

    if contains_any_word_slice_phrase(
        words,
        &[
            &["power", "of", "chosen", "quality"],
            &["power", "of", "that", "quality"],
            &["power", "of", "the", "chosen", "quality"],
        ],
    ) {
        filter.power_parity = Some(ParityRequirement::Chosen);
    }

    if contains_any_word_slice_phrase(
        words,
        &[
            &["mana", "value", "of", "chosen", "quality"],
            &["mana", "value", "of", "that", "quality"],
            &["mana", "values", "of", "chosen", "quality"],
            &["mana", "values", "of", "that", "quality"],
            &["mana", "value", "of", "the", "chosen", "quality"],
            &["mana", "values", "of", "the", "chosen", "quality"],
        ],
    ) {
        filter.mana_value_parity = Some(ParityRequirement::Chosen);
    }
}

fn parse_simple_object_filter_lexed(tokens: &[OwnedLexToken], other: bool) -> Option<ObjectFilter> {
    let word_view = TokenWordView::new(tokens);
    let mut words: Vec<&str> = word_view
        .to_word_refs()
        .into_iter()
        .filter(|word| *word != "instead")
        .filter(|word| !is_article(word))
        .collect();
    if words.is_empty() {
        return None;
    }

    if words.iter().any(|word| {
        matches!(
            *word,
            "target"
                | "targets"
                | "that"
                | "which"
                | "whose"
                | "where"
                | "there"
                | "shares"
                | "share"
                | "dealt"
                | "entered"
                | "put"
                | "this"
                | "way"
        )
    }) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    if other {
        filter.other = true;
    }

    let mut saw_permanent_type = false;
    let mut saw_spell = false;
    let mut saw_card = false;
    let mut saw_permanent = false;

    let trim_suffix = |words: &mut Vec<&str>, suffix_len: usize| {
        let new_len = words.len().saturating_sub(suffix_len);
        words.truncate(new_len);
    };

    if let Some((suffix, suffix_len)) = parse_simple_object_filter_suffix(&words) {
        match suffix {
            SimpleObjectFilterSuffix::Controller(controller) => {
                filter.controller = Some(controller);
                filter.zone = Some(Zone::Battlefield);
            }
            SimpleObjectFilterSuffix::Owner(owner) => {
                filter.owner = Some(owner);
            }
            SimpleObjectFilterSuffix::OwnerZone(owner, zone) => {
                filter.owner = Some(owner);
                filter.zone = Some(zone);
            }
            SimpleObjectFilterSuffix::Zone(zone) => {
                filter.zone = Some(zone);
            }
        }
        trim_suffix(&mut words, suffix_len);
    }

    let mut idx = 0usize;
    while idx < words.len() {
        let word = words[idx];
        if word == "or" {
            idx += 1;
            continue;
        }
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            saw_spell = true;
            idx += consumed;
            continue;
        }
        if let Some((face_down, consumed)) = parse_filter_face_state_words(&words[idx..]) {
            filter.face_down = Some(face_down);
            idx += consumed;
            continue;
        }
        if matches!(word, "other" | "another") {
            filter.other = true;
            idx += 1;
            continue;
        }
        if matches!(word, "token" | "tokens") {
            filter.token = true;
            idx += 1;
            continue;
        }
        if word == "nontoken" {
            filter.nontoken = true;
            idx += 1;
            continue;
        }
        if word == "historic" {
            filter.historic = true;
            idx += 1;
            continue;
        }
        if word == "nonhistoric" {
            filter.nonhistoric = true;
            idx += 1;
            continue;
        }
        if word == "modified" {
            filter.modified = true;
            idx += 1;
            continue;
        }
        if word == "colorless" {
            filter.colorless = true;
            idx += 1;
            continue;
        }
        if word == "multicolored" {
            filter.multicolored = true;
            idx += 1;
            continue;
        }
        if word == "monocolored" {
            filter.monocolored = true;
            idx += 1;
            continue;
        }
        if matches!(word, "card" | "cards") {
            saw_card = true;
            idx += 1;
            continue;
        }
        if matches!(word, "permanent" | "permanents") {
            saw_permanent = true;
            idx += 1;
            continue;
        }
        if matches!(word, "spell" | "spells") {
            saw_spell = true;
            idx += 1;
            continue;
        }
        if let Some((atom, consumed)) = parse_named_object_filter_word_atom(&words[idx..]) {
            match atom {
                NamedObjectFilterWordAtom::ChosenColor => filter.chosen_color = true,
                NamedObjectFilterWordAtom::ChosenType => filter.chosen_creature_type = true,
                NamedObjectFilterWordAtom::NonChosenType => {
                    filter.excluded_chosen_creature_type = true
                }
            }
            idx += consumed;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            push_unique(&mut filter.card_types, card_type);
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
            idx += 1;
            continue;
        }
        if let Some(card_type) = parse_non_type(word) {
            push_unique(&mut filter.excluded_card_types, card_type);
            idx += 1;
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique(&mut filter.subtypes, subtype);
            idx += 1;
            continue;
        }
        if let Some(subtype) = parse_non_subtype(word) {
            push_unique(&mut filter.excluded_subtypes, subtype);
            idx += 1;
            continue;
        }
        if let Some(supertype) = parse_supertype_word(word) {
            push_unique(&mut filter.supertypes, supertype);
            idx += 1;
            continue;
        }
        if let Some(supertype) = parse_non_supertype(word) {
            push_unique(&mut filter.excluded_supertypes, supertype);
            idx += 1;
            continue;
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
            idx += 1;
            continue;
        }
        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            idx += 1;
            continue;
        }
        if is_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.subtypes);
            idx += 1;
            continue;
        }
        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            idx += 1;
            continue;
        }
        if matches!(word, "of" | "from" | "in") {
            return None;
        }
        return None;
    }

    if filter.zone.is_none() {
        if saw_spell {
            filter.zone = Some(Zone::Stack);
        } else if saw_permanent || saw_permanent_type || filter.token {
            filter.zone = Some(Zone::Battlefield);
        } else if saw_card && filter.zone.is_none() {
            filter.zone = None;
        }
    }
    if saw_spell {
        filter.has_mana_cost = true;
    }

    Some(filter)
}

pub(super) fn parse_attached_reference_or_another_disjunction(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let segments = split_lexed_slices_on_or(tokens);
    if segments.len() != 2 {
        return Ok(None);
    }

    let first_word_view = TokenWordView::new(segments[0]);
    let first_words: Vec<&str> = first_word_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let second_word_view = TokenWordView::new(segments[1]);
    let second_words: Vec<&str> = second_word_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let first_is_attached_reference = first_words
        .first()
        .is_some_and(|word| matches!(*word, "enchanted" | "equipped"));
    let second_starts_with_other = second_words
        .first()
        .is_some_and(|word| matches!(*word, "another" | "other"));
    if !first_is_attached_reference || !second_starts_with_other {
        return Ok(None);
    }

    let first_other = first_words
        .first()
        .is_some_and(|word| matches!(*word, "another" | "other"));
    let second_other = second_words
        .first()
        .is_some_and(|word| matches!(*word, "another" | "other"));

    let first_filter = parse_object_filter(segments[0], first_other)?;
    let second_filter = parse_object_filter(segments[1], second_other)?;

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = vec![first_filter, second_filter];
    Ok(Some(disjunction))
}

pub(crate) fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    super::grammar::filters::parse_object_filter_with_grammar_entrypoint(tokens, other)
}

pub(crate) fn parse_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let (trimmed_tokens, vote_winners_only) = trim_vote_winner_suffix(tokens);
    if let Some(mut filter) = parse_simple_object_filter_lexed(&trimmed_tokens, other) {
        if vote_winners_only {
            filter = filter.match_tagged(
                TagKey::from(VOTE_WINNERS_TAG),
                TaggedOpbjectRelation::IsTaggedObject,
            );
        }
        return Ok(filter);
    }
    parse_object_filter(&trimmed_tokens, other)
}

pub(crate) fn spell_filter_has_identity(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.colors.is_some()
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.total_counters_parity.is_some()
        || filter.cast_by.is_some()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || filter.alternative_cast.is_some()
        || !filter.any_of.is_empty()
}

pub(crate) fn merge_spell_filters(base: &mut ObjectFilter, extra: ObjectFilter) {
    for card_type in extra.card_types {
        push_unique(&mut base.card_types, card_type);
    }
    for card_type in extra.excluded_card_types {
        push_unique(&mut base.excluded_card_types, card_type);
    }
    for subtype in extra.subtypes {
        push_unique(&mut base.subtypes, subtype);
    }
    if let Some(colors) = extra.colors {
        let existing = base.colors.unwrap_or(ColorSet::new());
        base.colors = Some(existing.union(colors));
    }
    base.chosen_color |= extra.chosen_color;
    base.chosen_creature_type |= extra.chosen_creature_type;
    base.excluded_chosen_creature_type |= extra.excluded_chosen_creature_type;
    if base.alternative_cast.is_none() {
        base.alternative_cast = extra.alternative_cast;
    }
    if base.power.is_none() {
        base.power = extra.power;
    }
    if base.power_parity.is_none() {
        base.power_parity = extra.power_parity;
    }
    if base.toughness.is_none() {
        base.toughness = extra.toughness;
    }
    if base.mana_value.is_none() {
        base.mana_value = extra.mana_value;
    }
    if base.mana_value_parity.is_none() {
        base.mana_value_parity = extra.mana_value_parity;
    }
    if base.total_counters_parity.is_none() {
        base.total_counters_parity = extra.total_counters_parity;
    }
    if base.cast_by.is_none() {
        base.cast_by = extra.cast_by;
    }
    if base.targets_player.is_none() {
        base.targets_player = extra.targets_player;
    }
    if base.targets_object.is_none() {
        base.targets_object = extra.targets_object;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::compiler::util::tokenize_line;

    #[test]
    fn strip_not_on_battlefield_phrase_strips_contractions_without_word_view() {
        let mut tokens = tokenize_line("creatures that aren't on the battlefield", 0);

        assert!(strip_not_on_battlefield_phrase(&mut tokens));
        assert_eq!(
            crate::cards::builders::compiler::token_word_refs(&tokens),
            vec!["creatures"]
        );
    }

    #[test]
    fn trim_vote_winner_suffix_trims_vote_phrase_without_word_view() {
        let tokens = tokenize_line("creature with most votes or tied for most votes", 0);
        let (trimmed, vote_winners_only) = trim_vote_winner_suffix(&tokens);

        assert!(vote_winners_only);
        assert_eq!(
            crate::cards::builders::compiler::token_word_refs(&trimmed),
            vec!["creature"]
        );
    }

    #[test]
    fn parse_attached_reference_or_another_disjunction_handles_articles_without_word_view() {
        let tokens = tokenize_line("enchanted creature or another creature", 0);

        let filter = parse_attached_reference_or_another_disjunction(&tokens)
            .expect("attached-reference disjunction should parse")
            .expect("attached-reference disjunction should be recognized");

        assert_eq!(filter.any_of.len(), 2);
        assert!(
            filter.any_of[0]
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag.as_str() == "enchanted"
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                }),
            "{filter:?}"
        );
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[1].other);
    }

    #[test]
    fn parse_object_filter_lexed_parses_suffix_owned_zone() {
        let tokens = tokenize_line("artifact card from your graveyard", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
    }

    #[test]
    fn parse_object_filter_lexed_parses_split_face_state_and_chosen_type_atoms() {
        let tokens = tokenize_line("face down chosen type creatures", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.face_down, Some(true));
        assert!(filter.chosen_creature_type);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_parses_hyphenated_face_state_and_nonchosen_type_atoms() {
        let tokens = tokenize_line("face-up nonchosen type creatures", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.face_down, Some(false));
        assert!(filter.excluded_chosen_creature_type);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn apply_parity_filter_phrases_detects_chosen_quality_and_odd_mana_value() {
        let mut filter = ObjectFilter::default();

        apply_parity_filter_phrases(
            &[
                "odd", "mana", "value", "and", "power", "of", "chosen", "quality",
            ],
            &mut filter,
        );

        assert_eq!(filter.mana_value_parity, Some(ParityRequirement::Odd));
        assert_eq!(filter.power_parity, Some(ParityRequirement::Chosen));
    }
}

pub(crate) fn is_comparison_or_delimiter(tokens: &[OwnedLexToken], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("or")) {
        return false;
    }
    let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
    let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
    if matches!(next_word, Some("less" | "greater" | "more" | "fewer")) {
        return true;
    }
    if previous_word == Some("than") && next_word == Some("equal") {
        return true;
    }
    false
}
