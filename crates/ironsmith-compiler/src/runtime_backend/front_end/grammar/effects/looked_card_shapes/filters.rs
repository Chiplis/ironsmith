use crate::cards::builders::{ObjectFilter, TagKey, TextSpan};
use crate::runtime_backend::front_end::lexer::{
    OwnedLexToken, parser_token_word_refs, trim_lexed_commas,
};
use crate::runtime_backend::object_filters::{
    is_comparison_or_delimiter, parse_object_filter_lexed,
};
use crate::runtime_backend::util::{
    non_article_word_refs, parse_choice_count_token_prefix_consumed,
    strip_leading_article_word_refs,
};
use crate::target::TaggedOpbjectRelation;
use crate::types::{CardType, Supertype};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{permission_shapes, primitives};

const CHOSEN_NAME_TAG: &str = "__chosen_name__";
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
    if !filter
        .excluded_card_types
        .iter()
        .any(|existing| *existing == card_type)
    {
        filter.excluded_card_types.push(card_type);
    }
}

fn apply_same_name(mut filter: ObjectFilter, same_name: bool) -> ObjectFilter {
    if same_name {
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
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
    let explicit_and_or = words.iter().any(|word| *word == "and/or")
        || permission_shapes::find_words(words, &["and", "or"]).is_some();
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
        let parsed = parse_object_filter_lexed(&segment, false)
            .ok()
            .filter(|filter| *filter != ObjectFilter::default())
            .or_else(|| parse_named_card_filter_segment(&segment))?;
        branches.push(parsed);
    }
    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
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
    Some(filter)
}

pub(crate) fn parse_looked_card_reveal_filter_shape(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let (filter_tokens, same_name) = split_same_name_suffix(tokens);
    let all_words = parser_token_word_refs(filter_tokens);
    let words = non_article_word_refs(&all_words);

    if CHOSEN_CARD_PHRASES
        .iter()
        .any(|expected| permission_shapes::exact_words(&words, expected))
    {
        return Some(apply_same_name(ObjectFilter::default(), true));
    }
    if words.len() == 1 && is_card_word(words[0]) {
        return Some(apply_same_name(ObjectFilter::default(), same_name));
    }
    if words.len() == 4
        && is_card_word(words[0])
        && (permission_shapes::exact_words(&words[1..], &["of", "chosen", "type"])
            || permission_shapes::exact_words(&words[1..], &["of", "that", "type"]))
    {
        let mut filter = ObjectFilter::default();
        filter.chosen_creature_type = true;
        return Some(apply_same_name(filter, same_name));
    }
    if permission_shapes::exact_words(&words, &["permanent", "card"])
        || permission_shapes::exact_words(&words, &["permanent", "cards"])
    {
        return Some(apply_same_name(ObjectFilter::permanent_card(), same_name));
    }
    if permission_shapes::exact_words(&words, &["historic", "card"])
        || permission_shapes::exact_words(&words, &["historic", "cards"])
    {
        let mut filter = ObjectFilter::default();
        filter.historic = true;
        return Some(apply_same_name(filter, same_name));
    }
    if permission_shapes::exact_words(&words, &["nonland", "permanent", "card"])
        || permission_shapes::exact_words(&words, &["nonland", "permanent", "cards"])
    {
        let mut filter = ObjectFilter::permanent_card();
        filter.excluded_card_types.push(CardType::Land);
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_noncreature_nonland_permanent(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_land_or_legendary_permanent(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_modified_permanent_cards(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_filter_disjunction(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }

    let filter = parse_generic_disjunction_filter(filter_tokens)
        .or_else(|| parse_object_filter_lexed(filter_tokens, false).ok())?;
    Some(apply_same_name(filter, same_name))
}

pub(crate) fn strip_up_to_one_looked_card_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let tokens = trim_lexed_commas(tokens);
    let Some((count, used)) = parse_choice_count_token_prefix_consumed(tokens) else {
        return tokens.to_vec();
    };
    if count == crate::effect::ChoiceCount::up_to(1) {
        trim_lexed_commas(tokens.get(used..).unwrap_or_default()).to_vec()
    } else {
        tokens.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn parse(raw: &str) -> ObjectFilter {
        parse_looked_card_reveal_filter_shape(&lex_line(raw, 0).unwrap()).unwrap()
    }

    #[test]
    fn parses_typed_special_looked_card_filters() {
        assert_eq!(parse("a permanent card").card_types.len(), 6);
        assert!(
            parse("a nonland permanent card")
                .excluded_card_types
                .contains(&CardType::Land)
        );
        assert_eq!(
            parse("a land and/or legendary permanent card").any_of.len(),
            2
        );
        assert_eq!(
            parse("a card with the chosen name")
                .tagged_constraints
                .len(),
            1
        );

        let shared =
            parse("a permanent card that shares a card type with the sacrificed permanent");
        assert!(shared.tagged_constraints.iter().any(|constraint| {
            constraint.tag == TagKey::from("sacrificed_0")
                && constraint.relation == TaggedOpbjectRelation::SharesCardType
        }));
    }
}
