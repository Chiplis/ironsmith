use super::super::grammar::primitives::{self as grammar, split_lexed_slices_on_or};
use super::super::grammar::values::parse_value_comparison_tokens;
use super::super::lexer::{OwnedLexToken, TokenKind, lex_line, token_word_refs, trim_lexed_commas};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::token_primitives::{
    contains_window as word_slice_contains_sequence, find_any_str_index as word_slice_find_any,
    find_index as find_token_index, find_str_index as word_slice_find,
    find_window_index as word_slice_find_sequence, parse_simple_restriction_duration_prefix,
    parse_simple_restriction_duration_suffix, rfind_index as rfind_token_index,
    slice_contains_all as word_slice_has_all, slice_contains_any as word_slice_contains_any,
    slice_contains_str as word_slice_contains, slice_ends_with as word_slice_ends_with,
    slice_starts_with as word_slice_starts_with,
};
use super::super::util::{
    helper_tag_for_tokens, is_article, parse_number, parse_subject, parse_target_phrase,
    parse_zone_word, span_from_tokens, token_index_for_word_index, trim_commas, words,
};
use super::sentence_helpers::*;
use super::{
    find_verb, parse_effect_chain, parse_effect_chain_with_sentence_primitives, parse_effect_clause,
};
use crate::cards::builders::{
    CardTextError, CarryContext, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, PlayerAst, ReturnControllerAst, SubjectAst,
    TagKey, TargetAst, TextSpan,
};
use crate::effect::SearchSelectionMode;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[derive(Clone)]
pub(crate) enum SearchLibraryManaConstraint {
    Equal(u32),
    LessThanOrEqual(u32),
    GreaterThanOrEqual(u32),
    OneOf(Vec<u32>),
}

fn token_words<'a>(tokens: &'a [OwnedLexToken]) -> Vec<&'a str> {
    token_word_refs(tokens)
}

fn token_slice_contains_word(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    tokens
        .iter()
        .enumerate()
        .any(|(idx, _)| grammar::parse_prefix(&tokens[idx..], grammar::kw(expected)).is_some())
}

fn token_slice_contains_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    tokens
        .iter()
        .enumerate()
        .any(|(idx, _)| grammar::parse_prefix(&tokens[idx..], grammar::phrase(phrase)).is_some())
}

fn find_phrase_token_bounds(
    tokens: &[OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<(usize, usize)> {
    if phrase.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    while idx < tokens.len() {
        if let Some((_, rest)) = grammar::parse_prefix(&tokens[idx..], grammar::phrase(phrase)) {
            let end_idx = idx + (tokens[idx..].len() - rest.len());
            return Some((idx, end_idx));
        }
        idx += 1;
    }

    None
}

pub(crate) fn word_slice_starts_with_any(words: &[&str], prefixes: &[&[&str]]) -> bool {
    prefixes
        .iter()
        .any(|prefix| word_slice_starts_with(words, prefix))
}

pub(crate) fn word_slice_mentions_nth_from_top(words: &[&str]) -> bool {
    let mut idx = 0usize;
    while idx + 3 < words.len() {
        if words[idx + 1] == "from" && words[idx + 2] == "the" && words[idx + 3] == "top" {
            return true;
        }
        idx += 1;
    }
    false
}

fn is_source_reference_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    [
        "this",
        "thiss",
        "source",
        "artifact",
        "creature",
        "permanent",
    ]
    .iter()
    .any(|word| token_slice_contains_word(tokens, word))
}

fn is_as_long_as_you_control_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    token_slice_contains_word(tokens, "you")
        && token_slice_contains_word(tokens, "control")
        && is_source_reference_duration_tokens(tokens)
}

fn is_source_remains_tapped_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    token_slice_contains_phrase(tokens, &["for", "as", "long", "as"])
        && token_slice_contains_word(tokens, "remains")
        && token_slice_contains_word(tokens, "tapped")
        && is_source_reference_duration_tokens(tokens)
}

fn remove_this_turn_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut cleaned = Vec::new();
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx].is_word("this")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("turn"))
        {
            idx += 2;
            continue;
        }
        cleaned.push(tokens[idx].clone());
        idx += 1;
    }
    cleaned
}

pub(crate) fn zone_slice_contains(zones: &[Zone], expected: Zone) -> bool {
    zones.iter().any(|zone| *zone == expected)
}

fn card_type_slice_contains(card_types: &[CardType], expected: CardType) -> bool {
    card_types.iter().any(|card_type| *card_type == expected)
}

fn word_has_fragment(word: &str, fragment: &str) -> bool {
    word.match_indices(fragment).next().is_some()
}

fn strip_known_possessive_suffix(word: &str) -> &str {
    for suffix in ["'s", "’s", "s'", "s’"] {
        let start = word.len().saturating_sub(suffix.len());
        if word.get(start..) == Some(suffix) {
            return word.get(..start).unwrap_or("");
        }
    }

    word
}

pub(crate) fn parse_search_library_disjunction_filter(
    filter_tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let segments = split_lexed_slices_on_or(filter_tokens);
    if segments.len() < 2 {
        return None;
    }

    let mut branches = Vec::new();
    for segment in segments {
        let trimmed = trim_commas(segment);
        if trimmed.is_empty() {
            return None;
        }
        let Ok(filter) = parse_object_filter(&trimmed, false) else {
            return None;
        };
        branches.push(filter);
    }

    if branches.len() < 2 {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Some(filter)
}

pub(crate) fn parse_restriction_duration_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    use crate::effect::Until;

    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some((duration, rest)) = parse_simple_restriction_duration_prefix(tokens) {
        return Ok(Some((duration, trim_lexed_commas(rest).to_vec())));
    }

    if token_words(tokens).len() < 2 {
        return Ok(None);
    }

    if grammar::parse_prefix(tokens, grammar::phrase(&["for", "as", "long", "as"])).is_some() {
        if !is_as_long_as_you_control_duration_tokens(tokens) {
            return Ok(None);
        }
        let Some((_before, after)) =
            grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        else {
            return Err(CardTextError::ParseError(
                "missing comma after duration prefix".to_string(),
            ));
        };
        let remainder = trim_lexed_commas(after).to_vec();
        return Ok(Some((Until::YouStopControllingThis, remainder)));
    }

    if let Some((rest, duration)) = parse_simple_restriction_duration_suffix(tokens) {
        let remainder = trim_lexed_commas(rest).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((duration, remainder)));
        }
    }

    if let Some((token_idx, _)) = find_phrase_token_bounds(tokens, &["for", "as", "long", "as"]) {
        let suffix_tokens = &tokens[token_idx..];
        if is_source_remains_tapped_duration_tokens(suffix_tokens) {
            let remainder = trim_lexed_commas(&tokens[..token_idx]).to_vec();
            return Ok(Some((Until::ThisLeavesTheBattlefield, remainder)));
        }
        if is_as_long_as_you_control_duration_tokens(suffix_tokens) {
            let remainder = trim_lexed_commas(&tokens[..token_idx]).to_vec();
            return Ok(Some((Until::YouStopControllingThis, remainder)));
        }
    }

    if token_slice_contains_phrase(tokens, &["this", "turn"]) {
        let cleaned = remove_this_turn_tokens(tokens);
        let remainder = trim_lexed_commas(&cleaned).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((Until::EndOfTurn, remainder)));
        }
    }

    Ok(None)
}

pub(crate) fn extract_search_library_mana_constraint(
    filter_tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, SearchLibraryManaConstraint)> {
    let (clause_token_start, clause_token_end) =
        find_phrase_token_bounds(filter_tokens, &["with", "mana", "cost"])
            .or_else(|| find_phrase_token_bounds(filter_tokens, &["with", "mana", "value"]))?;
    let base_filter_tokens = trim_commas(&filter_tokens[..clause_token_start]);
    if base_filter_tokens.is_empty() {
        return None;
    }

    let clause_tokens = trim_lexed_commas(&filter_tokens[clause_token_end..]);
    if clause_tokens.is_empty() {
        return None;
    }

    let parse_single_u32_clause = |tokens: &[OwnedLexToken]| -> Option<u32> {
        let [token] = tokens else {
            return None;
        };
        token.parser_text().parse::<u32>().ok()
    };
    let constraint = if let Some(value) = parse_single_u32_clause(clause_tokens) {
        SearchLibraryManaConstraint::Equal(value)
    } else if let Some((operator, value_tokens)) = parse_value_comparison_tokens(clause_tokens) {
        let value = parse_single_u32_clause(value_tokens)?;
        match operator {
            crate::effect::ValueComparisonOperator::LessThanOrEqual => {
                SearchLibraryManaConstraint::LessThanOrEqual(value)
            }
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
                SearchLibraryManaConstraint::GreaterThanOrEqual(value)
            }
            _ => return None,
        }
    } else {
        let [left, middle, right] = clause_tokens else {
            return None;
        };
        if !middle.is_word("or") {
            return None;
        }
        SearchLibraryManaConstraint::OneOf(vec![
            left.parser_text().parse::<u32>().ok()?,
            right.parser_text().parse::<u32>().ok()?,
        ])
    };

    Some((base_filter_tokens, constraint))
}

pub(crate) fn apply_search_library_mana_constraint(
    filter: &mut ObjectFilter,
    constraint: SearchLibraryManaConstraint,
) {
    if !filter.any_of.is_empty() {
        for nested in &mut filter.any_of {
            apply_search_library_mana_constraint(nested, constraint.clone());
        }
        return;
    }

    let build_branch = |base: &ObjectFilter, mana_value: crate::filter::Comparison| {
        let mut branch = base.clone();
        branch.has_mana_cost = true;
        branch.no_x_in_cost = true;
        branch.mana_value = Some(mana_value);
        branch
    };

    match constraint {
        SearchLibraryManaConstraint::Equal(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::Equal(value as i32));
        }
        SearchLibraryManaConstraint::LessThanOrEqual(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(value as i32));
        }
        SearchLibraryManaConstraint::GreaterThanOrEqual(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::GreaterThanOrEqual(value as i32));
        }
        SearchLibraryManaConstraint::OneOf(values) => {
            let base = filter.clone();
            *filter = ObjectFilter::default();
            filter.any_of = values
                .into_iter()
                .map(|value| build_branch(&base, crate::filter::Comparison::Equal(value as i32)))
                .collect();
        }
    }
}

pub(crate) fn split_search_same_name_reference_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let (start_token_idx, end_token_idx) =
        find_phrase_token_bounds(tokens, &["with", "the", "same", "name", "as"])
            .or_else(|| find_phrase_token_bounds(tokens, &["with", "same", "name", "as"]))?;
    let base_filter_tokens = trim_commas(&tokens[..start_token_idx]);
    let reference_tokens = trim_commas(&tokens[end_token_idx..]);
    Some((base_filter_tokens, reference_tokens))
}

pub(crate) fn is_same_name_that_reference_words(words: &[&str]) -> bool {
    matches!(
        words,
        ["that", "card"]
            | ["that", "cards"]
            | ["that", "creature"]
            | ["that", "creatures"]
            | ["that", "artifact"]
            | ["that", "artifacts"]
            | ["that", "enchantment"]
            | ["that", "enchantments"]
            | ["that", "land"]
            | ["that", "lands"]
            | ["that", "permanent"]
            | ["that", "permanents"]
            | ["that", "spell"]
            | ["that", "spells"]
            | ["that", "object"]
            | ["that", "objects"]
            | ["those", "cards"]
            | ["those", "creatures"]
            | ["those", "artifacts"]
            | ["those", "enchantments"]
            | ["those", "lands"]
            | ["those", "permanents"]
            | ["those", "spells"]
            | ["those", "objects"]
    )
}

pub(crate) fn normalize_search_library_filter(filter: &mut ObjectFilter) {
    filter.zone = None;
    if filter.subtypes.iter().any(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
                | Subtype::Desert
        )
    }) && !card_type_slice_contains(&filter.card_types, CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    for nested in &mut filter.any_of {
        normalize_search_library_filter(nested);
    }
}

pub(crate) fn parse_shuffle_graveyard_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = token_words(&clause_tokens);
    if !clause_words
        .iter()
        .any(|word| *word == "shuffle" || *word == "shuffles")
        || !grammar::contains_word(&clause_tokens, "graveyard")
        || !grammar::contains_word(&clause_tokens, "library")
    {
        return Ok(None);
    }

    let Some(shuffle_idx) = find_token_index(&clause_tokens, |token| {
        token.is_word("shuffle") || token.is_word("shuffles")
    }) else {
        return Ok(None);
    };

    // Keep this primitive focused on shuffle-led clauses so we don't swallow
    // earlier effects in chains like "... then shuffle your graveyard ...".
    if shuffle_idx > 3 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&clause_tokens[..shuffle_idx]);
    let each_player_subject = {
        let subject_words = token_words(&subject_tokens);
        word_slice_starts_with_any(&subject_words, &[&["each", "player"], &["each", "players"]])
    };
    let subject = if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else if each_player_subject {
        SubjectAst::Player(PlayerAst::Implicit)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let body_tokens = trim_commas(&clause_tokens[shuffle_idx + 1..]);
    if body_tokens.is_empty() {
        return Ok(None);
    }

    let Some(into_idx) = find_token_index(&body_tokens, |token| token.is_word("into")) else {
        return Ok(None);
    };
    if into_idx == 0 {
        return Ok(None);
    }

    let destination_tokens = trim_commas(&body_tokens[into_idx + 1..]);
    let destination_words = token_words(&destination_tokens);
    if !grammar::contains_word(&destination_tokens, "library") {
        return Ok(None);
    }
    let owner_library_destination = destination_words
        .iter()
        .any(|word| word_has_fragment(word, "owner"));
    let trailing_tokens = find_token_index(&destination_tokens, |token| {
        token.is_word("library") || token.is_word("libraries")
    })
    .map(|idx| trim_commas(&destination_tokens[idx + 1..]).to_vec())
    .unwrap_or_default();
    let append_trailing =
        |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
            if trailing_tokens.is_empty() {
                return Ok(Some(effects));
            }
            let mut trailing_effects = parse_effect_chain(&trailing_tokens)?;
            if each_player_subject {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player(effect, CarryContext::ForEachPlayer);
                }
            } else {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player_with_clause(
                        effect,
                        CarryContext::Player(player),
                        &trailing_tokens,
                    );
                }
            }
            effects.extend(trailing_effects);
            Ok(Some(effects))
        };

    let target_tokens = trim_commas(&body_tokens[..into_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target_words = token_words(&target_tokens);
    if !grammar::contains_word(&target_tokens, "graveyard") {
        return Ok(None);
    }

    let has_target_selector = grammar::contains_word(&target_tokens, "target");
    if !has_target_selector {
        let mut effects = Vec::new();
        let has_source_and_graveyard_clause = word_slice_starts_with_any(
            &target_words,
            &[
                &["this", "artifact", "and"],
                &["this", "permanent", "and"],
                &["this", "card", "and"],
            ],
        );
        if has_source_and_graveyard_clause {
            effects.push(EffectAst::MoveToZone {
                target: TargetAst::Source(None),
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            });
            if owner_library_destination {
                effects.push(EffectAst::ShuffleLibrary {
                    player: PlayerAst::ItsOwner,
                });
            }
        }
        if each_player_subject && grammar::contains_word(&target_tokens, "hand") {
            let mut hand_filter = ObjectFilter::default();
            hand_filter.zone = Some(Zone::Hand);
            hand_filter.owner = Some(PlayerFilter::IteratedPlayer);
            effects.push(EffectAst::MoveToZone {
                target: TargetAst::Object(hand_filter, None, None),
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            });
        }
        effects.push(EffectAst::ShuffleGraveyardIntoLibrary { player });
        if each_player_subject {
            return append_trailing(vec![EffectAst::ForEachPlayer { effects }]);
        }
        return append_trailing(effects);
    }

    let mut target = parse_target_phrase(&target_tokens)?;
    apply_shuffle_subject_graveyard_owner_context(&mut target, subject);

    append_trailing(vec![EffectAst::ShuffleObjectsIntoLibrary {
        target,
        player,
    }])
}

pub(crate) fn parse_shuffle_object_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = token_words(&clause_tokens);
    if !clause_words
        .iter()
        .any(|word| *word == "shuffle" || *word == "shuffles")
        || !grammar::contains_word(&clause_tokens, "library")
        || grammar::contains_word(&clause_tokens, "graveyard")
    {
        return Ok(None);
    }

    let Some(shuffle_idx) = find_token_index(&clause_tokens, |token| {
        token.is_word("shuffle") || token.is_word("shuffles")
    }) else {
        return Ok(None);
    };
    if shuffle_idx > 3 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&clause_tokens[..shuffle_idx]);
    let subject = if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let body_tokens = trim_commas(&clause_tokens[shuffle_idx + 1..]);
    let Some(into_idx) = find_token_index(&body_tokens, |token| token.is_word("into")) else {
        return Ok(None);
    };
    if into_idx == 0 {
        return Ok(None);
    }

    let destination_tokens = trim_commas(&body_tokens[into_idx + 1..]);
    if !grammar::contains_word(&destination_tokens, "library") {
        return Ok(None);
    }

    let target_tokens = trim_commas(&body_tokens[..into_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target_words = token_words(&target_tokens);
    if matches!(subject, SubjectAst::Player(PlayerAst::ItsOwner))
        && matches!(
            target_words.as_slice(),
            ["them"] | ["those", "cards"] | ["those", "objects"] | ["those"]
        )
    {
        return Ok(Some(vec![EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![
                EffectAst::MoveToZone {
                    target: TargetAst::Tagged(
                        TagKey::from(IT_TAG),
                        span_from_tokens(&target_tokens),
                    ),
                    zone: Zone::Library,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                },
                EffectAst::ShuffleLibrary {
                    player: PlayerAst::ItsOwner,
                },
            ],
        }]));
    }
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(vec![EffectAst::ShuffleObjectsIntoLibrary {
        target,
        player,
    }]))
}

pub(crate) fn parse_exile_hand_and_graveyard_bundle_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_possessive_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_known_possessive_suffix(word)),
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    if tokens.is_empty() {
        return Ok(None);
    }

    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    if grammar::words_match_prefix(&clause_tokens, &["exile", "all", "cards", "from"]).is_none() {
        return Ok(None);
    }
    if !grammar::contains_word(&clause_tokens, "hand")
        && !grammar::contains_word(&clause_tokens, "hands")
    {
        return Ok(None);
    }
    if !grammar::contains_word(&clause_tokens, "graveyard")
        && !grammar::contains_word(&clause_tokens, "graveyards")
    {
        return Ok(None);
    }
    let clause_words = token_words(&clause_tokens);

    let first_zone_idx =
        word_slice_find_any(&clause_words, &["hand", "hands", "graveyard", "graveyards"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing zone in exile hand+graveyard clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
    if first_zone_idx <= 4 {
        return Ok(None);
    }

    let owner_words = normalize_possessive_words(&clause_words[4..first_zone_idx]);
    let owner = match owner_words.as_slice() {
        ["target", "player"] | ["target", "players"] => PlayerFilter::target_player(),
        ["target", "opponent"] | ["target", "opponents"] => PlayerFilter::target_opponent(),
        ["your"] => PlayerFilter::You,
        _ => return Ok(None),
    };

    let Some(first_zone) = parse_zone_word(clause_words[first_zone_idx]) else {
        return Ok(None);
    };
    if !matches!(first_zone, Zone::Hand | Zone::Graveyard) {
        return Ok(None);
    }

    let Some(and_word) = clause_words.get(first_zone_idx + 1) else {
        return Ok(None);
    };
    if *and_word != "and" {
        return Ok(None);
    }

    let mut second_zone_idx = first_zone_idx + 2;
    while clause_words
        .get(second_zone_idx)
        .is_some_and(|word| matches!(*word, "all" | "cards" | "from"))
    {
        second_zone_idx += 1;
    }
    let Some(second_zone_word) = clause_words.get(second_zone_idx) else {
        return Ok(None);
    };
    if clause_words.len() != second_zone_idx + 1 {
        return Ok(None);
    }
    let Some(second_zone) = parse_zone_word(second_zone_word) else {
        return Ok(None);
    };
    if !matches!(second_zone, Zone::Hand | Zone::Graveyard) || second_zone == first_zone {
        return Ok(None);
    }

    let mut first_filter = ObjectFilter::default().in_zone(first_zone);
    first_filter.owner = Some(owner.clone());
    let mut second_filter = ObjectFilter::default().in_zone(second_zone);
    second_filter.owner = Some(owner);

    Ok(Some(vec![
        EffectAst::ExileAll {
            filter: first_filter,
            face_down: false,
        },
        EffectAst::ExileAll {
            filter: second_filter,
            face_down: false,
        },
    ]))
}

pub(crate) fn parse_target_player_exiles_creature_and_graveyard_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_tokens = trim_commas(tokens);
    let clause_words = token_words(&clause_tokens);
    if clause_words.len() < 8 {
        return Ok(None);
    }

    let (subject_player, subject_filter) =
        if grammar::words_match_prefix(&clause_tokens, &["target", "opponent"]).is_some() {
            (PlayerAst::TargetOpponent, PlayerFilter::target_opponent())
        } else if grammar::words_match_prefix(&clause_tokens, &["target", "player"]).is_some() {
            (PlayerAst::Target, PlayerFilter::target_player())
        } else {
            return Ok(None);
        };

    let verb_idx = 2usize;
    if !matches!(
        clause_words.get(verb_idx).copied(),
        Some("exile") | Some("exiles")
    ) {
        return Ok(None);
    }

    let tail_words = &clause_words[verb_idx + 1..];
    let Some(and_idx) = word_slice_find(tail_words, "and") else {
        return Ok(None);
    };
    let creature_words = &tail_words[..and_idx];
    let graveyard_words = &tail_words[and_idx + 1..];

    if graveyard_words != ["their", "graveyard"] {
        return Ok(None);
    }

    let creature_words = if creature_words.first().is_some_and(|word| is_article(word)) {
        &creature_words[1..]
    } else {
        creature_words
    };
    let creature_clause_matches = creature_words == ["creature", "they", "control"]
        || creature_words == ["creature", "that", "player", "controls"];
    if !creature_clause_matches {
        return Ok(None);
    }

    let mut creature_filter = ObjectFilter::creature();
    creature_filter.controller = Some(subject_filter.clone());

    let mut graveyard_filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    graveyard_filter.owner = Some(subject_filter);

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: creature_filter,
            count: ChoiceCount::exactly(1),
            player: subject_player,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::Exile {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
            face_down: false,
        },
        EffectAst::ExileAll {
            filter: graveyard_filter,
            face_down: false,
        },
    ]))
}

pub(crate) fn parse_for_each_exiled_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if grammar::words_match_prefix(tokens, &["for", "each", "permanent", "exiled", "this", "way"])
        .is_none()
    {
        return Ok(None);
    }
    let words_all = token_words(tokens);
    if !grammar::contains_word(tokens, "shares")
        || !grammar::contains_word(tokens, "card")
        || !grammar::contains_word(tokens, "type")
        || !grammar::contains_word(tokens, "library")
        || !grammar::contains_word(tokens, "battlefield")
    {
        return Ok(None);
    }

    let filter_tokens = lex_line("a permanent that shares a card type with it", 0)?;
    let filter = parse_object_filter_lexed(&filter_tokens, false)?;
    let revealed_tag = helper_tag_for_tokens(tokens, "revealed");
    let matched_tag = helper_tag_for_tokens(tokens, "chosen");

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects: vec![
            EffectAst::ConsultTopOfLibrary {
                player: PlayerAst::Implicit,
                mode: LibraryConsultModeAst::Reveal,
                filter,
                stop_rule: LibraryConsultStopRuleAst::FirstMatch,
                all_tag: revealed_tag.clone(),
                match_tag: matched_tag.clone(),
            },
            EffectAst::MoveToZone {
                target: TargetAst::Tagged(matched_tag.clone(), None),
                zone: Zone::Battlefield,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            },
            EffectAst::PutTaggedRemainderOnBottomOfLibrary {
                tag: revealed_tag,
                keep_tagged: Some(matched_tag),
                order: LibraryBottomOrderAst::Random,
                player: PlayerAst::Implicit,
            },
        ],
    }]))
}

pub(crate) fn parse_each_player_put_permanent_cards_exiled_with_source_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = token_words(tokens);
    let starts_with_each_player_turns_face_up = grammar::words_match_prefix(
        tokens,
        &["each", "player", "turns", "face", "up", "all", "cards"],
    ).is_some();
    if !starts_with_each_player_turns_face_up {
        return Ok(None);
    }
    let has_exiled_with_this =
        grammar::words_find_phrase(tokens, &["exiled", "with", "this"]).is_some();
    if !has_exiled_with_this {
        return Ok(None);
    }
    let has_puts_all_permanent_cards =
        grammar::words_find_phrase(tokens, &["then", "puts", "all", "permanent", "cards"]).is_some();
    let has_among_them_onto_battlefield =
        grammar::words_find_phrase(tokens, &["among", "them", "onto", "battlefield"]).is_some()
            || grammar::words_find_phrase(
                tokens,
                &["among", "them", "onto", "the", "battlefield"],
            ).is_some();
    if !has_puts_all_permanent_cards || !has_among_them_onto_battlefield {
        return Ok(None);
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![EffectAst::ReturnAllToBattlefield {
            filter,
            tapped: false,
        }],
    }]))
}

pub(crate) fn parse_for_each_destroyed_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if grammar::words_match_prefix(tokens, &["for", "each"]).is_none() {
        return Ok(None);
    }
    let refers_to_destroyed =
        grammar::words_find_phrase(tokens, &["destroyed", "this", "way"]).is_some();
    let refers_to_died =
        grammar::words_find_phrase(tokens, &["died", "this", "way"]).is_some();
    let words_all = token_words(tokens);
    if !refers_to_destroyed && !refers_to_died {
        return Ok(None);
    }

    let (_before, after_comma) = grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing comma after 'for each ... this way' clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
    let effect_tokens = trim_commas(after_comma);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_earthbend_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_words(tokens);
    if words.first().copied() != Some("earthbend") {
        return Ok(None);
    }

    let count_tokens = &tokens[1..];
    let (count, _) = parse_number(count_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing earthbend count (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Earthbend { counters: count }))
}

pub(crate) fn parse_enchant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_words(tokens);
    if words.is_empty() || words[0] != "enchant" {
        return Ok(None);
    }

    let remaining = if tokens.len() > 1 { &tokens[1..] } else { &[] };
    let filter = match words.get(1..) {
        Some(["player"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::Any),
        Some(["opponent"]) | Some(["an", "opponent"]) => {
            crate::object::AuraAttachmentFilter::Player(PlayerFilter::Opponent)
        }
        Some(["you"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::You),
        _ => crate::object::AuraAttachmentFilter::Object(parse_object_filter(remaining, false)?),
    };
    Ok(Some(EffectAst::Enchant { filter }))
}

pub(crate) fn parse_restriction_duration(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    parse_restriction_duration_lexed(tokens)
}

pub(crate) fn parse_play_from_graveyard_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some((duration, rest)) = parse_simple_restriction_duration_prefix(tokens) else {
        return Ok(None);
    };
    if duration != crate::effect::Until::EndOfTurn {
        return Ok(None);
    };

    let remaining_words: Vec<&str> = token_words(trim_lexed_commas(rest))
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let expected = [
        "you",
        "may",
        "play",
        "lands",
        "and",
        "cast",
        "spells",
        "from",
        "your",
        "graveyard",
    ];

    if remaining_words == expected {
        return Ok(Some(EffectAst::PlayFromGraveyardUntilEot {
            player: PlayerAst::You,
        }));
    }

    Ok(None)
}

pub(crate) fn parse_exile_instead_of_graveyard_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.first().copied() != Some("if") {
        return Ok(None);
    }

    let has_graveyard_clause =
        grammar::words_find_phrase(tokens, &["into", "your", "graveyard", "from"]).is_some()
            || grammar::words_find_phrase(tokens, &["your", "graveyard", "from"]).is_some()
            || (grammar::contains_word(tokens, "your")
                && grammar::contains_word(tokens, "graveyard"));
    let has_would_put = grammar::words_find_phrase(tokens, &["card", "would", "be", "put"]).is_some();
    let has_this_turn =
        grammar::contains_word(tokens, "this") && grammar::contains_word(tokens, "turn");
    if !has_graveyard_clause || !has_would_put || !has_this_turn {
        return Ok(None);
    }

    let remainder = if let Some((_before, after)) =
        grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
    {
        after
    } else {
        return Ok(None);
    };

    let remaining_words: Vec<&str> = token_words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let expected = ["exile", "that", "card", "instead"];
    if remaining_words == expected {
        return Ok(Some(EffectAst::ExileInsteadOfGraveyardThisTurn {
            player: PlayerAst::You,
        }));
    }

    Ok(None)
}
