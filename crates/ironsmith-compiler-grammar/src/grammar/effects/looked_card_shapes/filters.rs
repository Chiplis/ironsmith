use crate::cards::builders::{ObjectFilter, TagKey, TextSpan};
use crate::lexer::{OwnedLexToken, parser_token_word_refs, trim_lexed_commas};
use crate::object_filters::{
    is_comparison_or_delimiter, parse_object_filter, parse_object_filter_lexed,
};
use crate::target::TaggedOpbjectRelation;
use crate::types::{CardType, Supertype};
use crate::util::{
    non_article_word_refs, parse_choice_count_token_prefix_consumed,
    strip_leading_article_word_refs,
};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{permission_shapes, primitives};
const SAME_NAME_SUFFIXES: &[&[&str]] = &[
    &["with", "that", "name"],
    &["with", "the", "chosen", "name"],
    &["with", "chosen", "name"],
];
const CHOSEN_CARD_PHRASES: &[&[&str]] = &[&["chosen", "card"], &["chosen", "cards"]];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct LookedDisjunctionQualifiers {
    distinct_names: bool,
    distinct_powers: bool,
    distinct_creature_types: bool,
    share_mana_value: bool,
}

fn is_card_word(word: &str) -> bool {
    matches!(word, "card" | "cards")
}

fn push_excluded_type(filter: &mut ObjectFilter, card_type: CardType) {
    if !filter.excluded_card_types.contains(&card_type) {
        filter.excluded_card_types.push(card_type);
    }
}

fn apply_same_name(mut filter: ObjectFilter, same_name: bool) -> ObjectFilter {
    if same_name {
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::ChosenName.key(),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
    }
    filter
}

fn split_same_name_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    primitives::strip_lexed_suffix_phrases(trim_lexed_commas(tokens), SAME_NAME_SUFFIXES)
        .map(|(_, head)| (trim_lexed_commas(head), true))
        .unwrap_or((trim_lexed_commas(tokens), false))
}

fn title_case_words(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut titled = String::new();
            titled.extend(first.to_uppercase());
            titled.push_str(chars.as_str());
            titled
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_named_card_filter_segment(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let all_words = parser_token_word_refs(tokens);
    let mut words = strip_leading_article_word_refs(&all_words).to_vec();
    if words.last().is_some_and(|word| is_card_word(word)) {
        words.pop();
    }
    if words.is_empty() {
        return None;
    }
    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_words(&words));
    Some(filter)
}

fn is_disjunction_token(tokens: &[OwnedLexToken], index: usize) -> bool {
    tokens.get(index).is_some_and(|token| {
        token.is_word("and/or")
            || (token.is_word("or") && !is_comparison_or_delimiter(tokens, index))
    })
}

fn split_filter_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let has_disjunction = tokens
        .iter()
        .enumerate()
        .any(|(index, _)| is_disjunction_token(tokens, index));
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if is_disjunction_token(tokens, index) || (has_disjunction && token.is_comma()) {
            while current
                .last()
                .is_some_and(|token: &OwnedLexToken| token.is_word("and"))
            {
                current.pop();
            }
            let segment = trim_lexed_commas(&current).to_vec();
            if !segment.is_empty() {
                segments.push(segment);
            }
            current.clear();
        } else {
            current.push(token.clone());
        }
    }
    while current.last().is_some_and(|token| token.is_word("and")) {
        current.pop();
    }
    let current = trim_lexed_commas(&current).to_vec();
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

fn parse_disjunction_qualifiers(words: &[&str]) -> LookedDisjunctionQualifiers {
    let explicit_and_or =
        words.contains(&"and/or") || permission_shapes::find_words(words, &["and", "or"]).is_some();
    LookedDisjunctionQualifiers {
        distinct_names: permission_shapes::find_words(words, &["with", "different", "names"])
            .is_some(),
        distinct_powers: permission_shapes::find_words(words, &["with", "different", "powers"])
            .is_some(),
        distinct_creature_types: permission_shapes::find_words(
            words,
            &["that", "share", "no", "creature", "types"],
        )
        .is_some(),
        share_mana_value: explicit_and_or
            && permission_shapes::find_words(words, &["with", "mana", "value"]).is_some(),
    }
}

fn apply_disjunction_qualifiers(
    filter: &mut ObjectFilter,
    qualifiers: LookedDisjunctionQualifiers,
) {
    if filter.any_of.len() < 2 {
        return;
    }
    if qualifiers.distinct_names {
        filter.distinct_names = true;
        for branch in &mut filter.any_of {
            branch.distinct_names = false;
        }
    }
    if qualifiers.distinct_powers {
        filter.distinct_powers = true;
        for branch in &mut filter.any_of {
            branch.distinct_powers = false;
        }
    }
    if qualifiers.distinct_creature_types {
        filter.distinct_creature_types = true;
        for branch in &mut filter.any_of {
            branch.distinct_creature_types = false;
        }
    }
    if qualifiers.share_mana_value
        && let Some(shared_mana_value) = filter
            .any_of
            .iter()
            .find_map(|branch| branch.mana_value.clone())
        && filter.any_of.iter().all(|branch| {
            branch.mana_value.is_none() || branch.mana_value.as_ref() == Some(&shared_mana_value)
        })
    {
        for branch in &mut filter.any_of {
            if branch.mana_value.is_none() {
                branch.mana_value = Some(shared_mana_value.clone());
            }
        }
    }
}

fn parse_noncreature_nonland_permanent(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<ObjectFilter> {
    if words.len() < 4
        || !permission_shapes::prefix_words(words, &["noncreature", "nonland", "permanent"])
        || !is_card_word(words[3])
    {
        return None;
    }
    let mut elided = Vec::new();
    let mut skipped_noncreature = false;
    let mut skipped_nonland = false;
    for token in tokens {
        if token.is_comma() {
            continue;
        }
        if !skipped_noncreature && token.is_word("noncreature") {
            skipped_noncreature = true;
        } else if !skipped_nonland && token.is_word("nonland") {
            skipped_nonland = true;
        } else {
            elided.push(token.clone());
        }
    }
    let mut filter = parse_object_filter_lexed(&elided, false)
        .ok()
        .unwrap_or_else(ObjectFilter::permanent_card);
    if filter.card_types.is_empty() && filter.all_card_types.is_empty() {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }
    push_excluded_type(&mut filter, CardType::Creature);
    push_excluded_type(&mut filter, CardType::Land);
    Some(filter)
}

/// A comma between consecutive negated characteristics is an adjective
/// separator, not an inclusive card-type union: "noncreature, nonland card"
/// means a card satisfying both exclusions. Explicit `or`/`and/or` lists keep
/// flowing to the disjunction parser below.
fn parse_conjunctive_negated_card_filter(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<ObjectFilter> {
    let (noun, modifiers) = words.split_last()?;
    if modifiers.len() < 2
        || !is_card_word(noun)
        || !tokens.iter().any(OwnedLexToken::is_comma)
        || tokens
            .iter()
            .any(|token| token.is_word("or") || token.is_word("and/or"))
        || !modifiers
            .iter()
            .all(|word| crate::word_primitives::parse_word_prefix(word, "non"))
    {
        return None;
    }
    let filter = parse_object_filter_lexed(tokens, false).ok()?;
    let exclusion_count = filter.excluded_card_types.len()
        + filter.excluded_subtypes.len()
        + filter.excluded_supertypes.len()
        + usize::from(!filter.excluded_colors.is_empty());
    (filter.any_of.is_empty() && exclusion_count >= 2).then_some(filter)
}

fn parse_land_or_legendary_permanent(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<ObjectFilter> {
    let prefix_len = if words.len() >= 5
        && permission_shapes::prefix_words(words, &["land", "and/or", "legendary", "permanent"])
        && is_card_word(words[4])
    {
        5
    } else if words.len() >= 6
        && permission_shapes::prefix_words(words, &["land", "and", "or", "legendary", "permanent"])
        && is_card_word(words[5])
    {
        6
    } else {
        return None;
    };
    if words.get(prefix_len).is_some_and(|word| *word != "with") {
        return None;
    }
    let base = parse_object_filter_lexed(tokens, false).ok()?;
    let mut land = base.clone();
    land.card_types = vec![CardType::Land];
    land.supertypes.clear();
    land.any_of.clear();

    let mut legendary_permanent = base;
    legendary_permanent.card_types = ObjectFilter::permanent_card().card_types;
    legendary_permanent.supertypes = vec![Supertype::Legendary];
    legendary_permanent.any_of.clear();

    let mut filter = ObjectFilter::default();
    filter.any_of = vec![land, legendary_permanent];
    filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    Some(filter)
}

fn parse_modified_permanent_cards(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<ObjectFilter> {
    if words.len() <= 2
        || words[words.len() - 2] != "permanent"
        || !is_card_word(words[words.len() - 1])
    {
        return None;
    }
    let (permanent, _, _) = primitives::find_prefix(tokens, || {
        (
            primitives::kw("permanent"),
            alt((primitives::kw("card"), primitives::kw("cards"))),
        )
            .void()
    })?;
    let mut elided = tokens.to_vec();
    elided.remove(permanent);
    let mut filter = parse_object_filter_lexed(&elided, false)
        .ok()
        .filter(|filter| filter.card_types.is_empty() && filter.all_card_types.is_empty())?;
    filter.card_types = ObjectFilter::permanent_card().card_types;
    Some(filter)
}

fn parse_filter_disjunction(tokens: &[OwnedLexToken], words: &[&str]) -> Option<ObjectFilter> {
    if !tokens
        .iter()
        .enumerate()
        .any(|(index, _)| is_disjunction_token(tokens, index))
    {
        return None;
    }
    let shared_card_suffix = words.last().is_some_and(|word| is_card_word(word));
    let segments = split_filter_segments(tokens);
    if segments.len() < 2 {
        return None;
    }
    let explicit_branch_articles = segments.iter().all(|segment| {
        let words = parser_token_word_refs(segment);
        words
            .first()
            .is_some_and(|word| matches!(*word, "a" | "an"))
            && words.iter().any(|word| is_card_word(word))
    });
    let mut branches = Vec::new();
    for mut segment in segments {
        if shared_card_suffix
            && !segment
                .last()
                .and_then(OwnedLexToken::as_word)
                .is_some_and(is_card_word)
        {
            segment.push(OwnedLexToken::word(
                "card".to_string(),
                TextSpan::synthetic(),
            ));
        }
        // Repeated complete card-noun arms carry independently scoped
        // predicates. Route those arms through the full object-filter parser:
        // the lexed simple fast path treats words inside an ability predicate
        // (for example, `doctor's` in `with doctor's companion`) as ordinary
        // subtype atoms before the predicate grammar gets a chance to own
        // them.
        let parsed = (if explicit_branch_articles {
            parse_object_filter(&segment, false)
        } else {
            parse_object_filter_lexed(&segment, false)
        })
        .ok()
        .filter(|filter| *filter != ObjectFilter::default())
        .or_else(|| parse_named_card_filter_segment(&segment))?;
        branches.push(parsed);
    }
    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    if tokens.iter().any(|token| token.is_word("and/or")) {
        filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    }
    filter.set_explicit_union_branch_articles(explicit_branch_articles);
    apply_disjunction_qualifiers(&mut filter, parse_disjunction_qualifiers(words));
    Some(filter)
}

fn parse_generic_disjunction_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let segments = primitives::split_lexed_slices_on_or(tokens);
    if segments.len() < 2 {
        return None;
    }
    let mut branches = Vec::new();
    for segment in segments {
        let segment = trim_lexed_commas(segment);
        if segment.is_empty() {
            return None;
        }
        branches.push(parse_object_filter_lexed(segment, false).ok()?);
    }
    if branches.len() < 2 {
        return None;
    }
    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    if tokens.iter().any(|token| token.is_word("and/or")) {
        filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    }
    Some(filter)
}

#[cfg(test)]
#[path = "filters_inline_tests.rs"]
mod tests;

#[path = "filters/library.rs"]
mod library_programs;
pub use library_programs::{
    parse_looked_card_reveal_filter_shape, strip_up_to_one_looked_card_choice_tokens,
};
