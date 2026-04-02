#![allow(dead_code)]

use crate::cards::builders::scan_helpers::{
    find_index, find_window_by, find_window_index, rfind_index, slice_contains, slice_ends_with,
    slice_starts_with, str_strip_suffix,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, ExchangeValueAst, ExchangeValueKindAst, IT_TAG, ObjectRefAst,
    OwnedLexToken, PlayerAst, PredicateAst, ReturnControllerAst, SharedTypeConstraintAst,
    SubjectAst, TagKey, TargetAst,
};
use crate::effect::{EventValueSpec, Until, Value};
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::Subtype;
use crate::zone::Zone;

use super::super::activation_and_restrictions::{
    controller_filter_for_token_player, parse_devotion_value_from_add_clause,
};
use super::super::keyword_static::parse_where_x_is_number_of_filter_value;
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_add_mana_that_much_value,
    parse_dynamic_cost_modifier_value, parse_pt_modifier, parse_pt_modifier_values,
};
use super::super::lexer::TokenKind;
use super::super::native_tokens::LowercaseWordView;
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::util::{
    intern_counter_name, is_article, mana_pips_from_token, parse_color, parse_counter_type_word,
    parse_mana_symbol, parse_number, parse_target_phrase, parse_value, parse_zone_word,
    parser_trace_stack, span_from_tokens, token_index_for_word_index, trim_commas, words,
};
use super::super::value_helpers::parse_filter_comparison_tokens;
use super::clause_pattern_helpers::extract_subject_player;
use super::conditionals::{parse_mana_symbol_group, parse_predicate, parse_subtype_word};
use super::dispatch_inner::trim_edge_punctuation;
use super::for_each_helpers::parse_get_modifier_values_with_tail;
use super::search_library::parse_restriction_duration;
use super::sentence_primitives::find_color_choice_phrase;

pub(crate) fn collapse_leading_signed_pt_modifier_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let sign = match tokens.first()?.kind {
        crate::cards::builders::parser::lexer::TokenKind::Dash => "-",
        crate::cards::builders::parser::lexer::TokenKind::Plus => "+",
        _ => return None,
    };
    let modifier = tokens.get(1)?.as_word()?;
    if !modifier.chars().any(|ch| ch == '/') {
        return None;
    }

    let mut collapsed = Vec::with_capacity(tokens.len().saturating_sub(1));
    collapsed.push(OwnedLexToken::word(
        format!("{sign}{modifier}"),
        tokens[0].span(),
    ));
    collapsed.extend(tokens.iter().skip(2).cloned());
    Some(collapsed)
}

fn split_chosen_creature_type_qualifier(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, bool, bool)> {
    let words = crate::cards::builders::parser::lexed_words(tokens);
    let patterns: [(&[&str], bool, bool); 5] = [
        (
            &["that", "arent", "of", "the", "chosen", "type"],
            false,
            true,
        ),
        (
            &["that", "aren't", "of", "the", "chosen", "type"],
            false,
            true,
        ),
        (
            &["that", "are", "not", "of", "the", "chosen", "type"],
            false,
            true,
        ),
        (&["of", "the", "chosen", "type"], true, false),
        (&["that", "are", "of", "the", "chosen", "type"], true, false),
    ];

    for (suffix, chosen_type, excluded_chosen_type) in patterns {
        if words.len() < suffix.len() || &words[words.len() - suffix.len()..] != suffix {
            continue;
        }
        let cutoff = words.len() - suffix.len();
        let token_cutoff = token_index_for_word_index(tokens, cutoff).unwrap_or(tokens.len());
        let base_tokens = trim_commas(&tokens[..token_cutoff]).to_vec();
        return Some((base_tokens, chosen_type, excluded_chosen_type));
    }

    None
}

fn split_chosen_this_way_qualifier(tokens: &[OwnedLexToken]) -> Option<(Vec<OwnedLexToken>, bool)> {
    let words = crate::cards::builders::parser::lexed_words(tokens);
    let suffixes: [(&[&str], bool); 7] = [
        (&["not", "chosen", "this", "way"], true),
        (&["that", "weren't", "chosen", "this", "way"], true),
        (&["that", "werent", "chosen", "this", "way"], true),
        (&["that", "were", "not", "chosen", "this", "way"], true),
        (&["chosen", "this", "way"], false),
        (&["that", "were", "chosen", "this", "way"], false),
        (&["that", "was", "chosen", "this", "way"], false),
    ];

    for (suffix, excluded) in suffixes {
        if words.len() < suffix.len() || &words[words.len() - suffix.len()..] != suffix {
            continue;
        }
        let cutoff = words.len() - suffix.len();
        let token_cutoff = if cutoff == 0 {
            0
        } else {
            token_index_for_word_index(tokens, cutoff).unwrap_or(tokens.len())
        };
        let base_tokens = trim_commas(&tokens[..token_cutoff]).to_vec();
        return Some((base_tokens, excluded));
    }

    None
}

pub(crate) fn parse_tap(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "tap clause missing target".to_string(),
        ));
    }
    let words = crate::cards::builders::parser::lexed_words(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::TapAll { filter });
    }
    // Handle "tap or untap <target>" as a choice between tapping and untapping.
    if tokens.first().is_some_and(|t| t.is_word("or"))
        && tokens.get(1).is_some_and(|t| t.is_word("untap"))
    {
        let target_tokens = &tokens[2..];
        let target = parse_target_phrase(target_tokens)?;
        return Ok(EffectAst::TapOrUntap {
            target: target.clone(),
        });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Tap { target })
}

pub(crate) fn parse_sacrifice(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let mut tokens = tokens;
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let mut normalized_words = clause_words.as_slice();
    if let Some(unless_idx) = find_index(&normalized_words, |word| *word == "unless") {
        let tail = &normalized_words[unless_idx..];
        if tail == ["unless", "it", "escaped"] {
            let cut_idx = token_index_for_word_index(tokens, unless_idx).unwrap_or(tokens.len());
            tokens = &tokens[..cut_idx];
            normalized_words = &normalized_words[..unless_idx];
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported sacrifice-unless clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }
    let has_greatest_mana_value = slice_contains(&normalized_words, &"greatest")
        && slice_contains(&normalized_words, &"mana")
        && slice_contains(&normalized_words, &"value");
    if has_greatest_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported greatest-mana-value sacrifice clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }
    let has_for_each_graveyard_history = slice_contains(&normalized_words, &"for")
        && slice_contains(&normalized_words, &"each")
        && slice_contains(&normalized_words, &"graveyard")
        && slice_contains(&normalized_words, &"turn");
    if has_for_each_graveyard_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported graveyard-history sacrifice clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if tokens
        .first()
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        let mut idx = 1usize;
        let mut other = false;
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_word("other") || token.is_word("another"))
        {
            other = true;
            idx += 1;
        }
        let mut filter = parse_object_filter_lexed(&tokens[idx..], other)?;
        if other {
            filter.other = true;
        }
        return Ok(EffectAst::SacrificeAll { filter, player });
    }

    let mut idx = 0;
    let mut count = 1u32;
    let mut other = false;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        other = true;
        idx += 1;
    }
    if count == 1
        && let Some((value, used)) = parse_number(&tokens[idx..])
    {
        count = value;
        idx += used;
    }

    let filter_tokens = trim_sacrifice_choice_suffix_tokens(&tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object after chooser suffix (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        )));
    }
    let mut filter = parse_object_filter_lexed(filter_tokens, other)?;
    if other {
        filter.other = true;
    }
    if filter.source && count != 1 {
        return Err(CardTextError::ParseError(format!(
            "source sacrifice only supports count 1 (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        )));
    }
    let sacrifice_words = crate::cards::builders::parser::lexed_words(tokens);
    let excludes_attached_object = find_window_by(&sacrifice_words, 3, |window| {
        matches!(
            window,
            ["than", "enchanted", "creature"]
                | ["than", "enchanted", "permanent"]
                | ["than", "equipped", "creature"]
                | ["than", "equipped", "permanent"]
        )
    })
    .is_some();
    if excludes_attached_object
        && filter.controller.is_none()
        && let Some(controller) = controller_filter_for_token_player(player)
    {
        filter.controller = Some(controller);
    }

    Ok(EffectAst::Sacrifice {
        filter,
        player,
        count,
    })
}

pub(crate) fn trim_sacrifice_choice_suffix_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let word_view = LowercaseWordView::new(tokens);
    let token_words = word_view.to_word_refs();
    let suffix_word_count = if slice_ends_with(&token_words, &["of", "their", "choice"])
        || slice_ends_with(&token_words, &["of", "your", "choice"])
        || slice_ends_with(&token_words, &["of", "its", "choice"])
    {
        3usize
    } else if slice_ends_with(&token_words, &["of", "his", "or", "her", "choice"]) {
        5usize
    } else {
        0usize
    };

    if suffix_word_count == 0 {
        return tokens;
    }

    let keep_words = token_words.len().saturating_sub(suffix_word_count);
    let cut_idx = word_view
        .token_index_for_word_index(keep_words)
        .unwrap_or(tokens.len());
    &tokens[..cut_idx]
}

pub(crate) fn parse_discard(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if slice_contains(&clause_words, &"hand") {
        return Ok(EffectAst::DiscardHand { player });
    }

    if matches!(clause_words.as_slice(), ["it"] | ["that", "card"]) {
        let mut tagged_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        tagged_filter.zone = Some(Zone::Hand);
        return Ok(EffectAst::Discard {
            count: Value::Fixed(1),
            player,
            random: false,
            filter: Some(tagged_filter),
            tag: None,
        });
    }

    let count_tokens = if slice_starts_with(&clause_words, &["up", "to"]) {
        &tokens[2..]
    } else {
        tokens
    };
    let count_offset = tokens.len().saturating_sub(count_tokens.len());
    let uses_all_count = count_tokens
        .first()
        .is_some_and(|token| token.is_word("all"));
    let (mut count, used) = if uses_all_count {
        (Value::Fixed(0), count_offset + 1)
    } else {
        let (count, used_relative) = parse_value(count_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing discard count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        (count, count_offset + used_relative)
    };

    let rest = &tokens[used..];
    let rest_words = crate::cards::builders::parser::lexed_words(rest);
    let Some(card_word_idx) = find_index(&rest_words, |word| *word == "card" || *word == "cards")
    else {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    };

    let card_token_idx = token_index_for_word_index(rest, card_word_idx).unwrap_or(rest.len());
    let qualifier_tokens = trim_commas(&rest[..card_token_idx]);
    let mut discard_filter = None;
    if !qualifier_tokens.is_empty() {
        let mut filter = if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
            filter
        } else if let Some(filter) = parse_discard_chosen_color_qualifier_filter(&qualifier_tokens)
        {
            filter
        } else if let Some(filter) = parse_discard_color_qualifier_filter(&qualifier_tokens) {
            filter
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported discard card qualifier (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        filter.zone = Some(Zone::Hand);
        if uses_all_count
            && let Some(owner) = discard_subject_owner_filter(subject)
            && filter.owner.is_none()
        {
            filter.owner = Some(owner);
        }
        discard_filter = Some(filter);
    }

    let trailing_tokens = if card_word_idx + 1 < rest_words.len() {
        let trailing_token_idx =
            token_index_for_word_index(rest, card_word_idx + 1).unwrap_or(rest.len());
        &rest[trailing_token_idx..]
    } else {
        &[]
    };
    let trailing_words = crate::cards::builders::parser::lexed_words(trailing_tokens);
    let random = trailing_words.as_slice() == ["at", "random"];
    if !trailing_words.is_empty() && !random {
        let trailing_filter = if let Ok(filter) = parse_object_filter(trailing_tokens, false) {
            Some(filter)
        } else if let Some(filter) = parse_discard_chosen_color_qualifier_filter(trailing_tokens) {
            Some(filter)
        } else if let Some(filter) = parse_discard_color_qualifier_filter(trailing_tokens) {
            Some(filter)
        } else {
            None
        };

        if let Some(mut filter) = trailing_filter {
            filter.zone = Some(Zone::Hand);
            if uses_all_count
                && let Some(owner) = discard_subject_owner_filter(subject)
                && filter.owner.is_none()
            {
                filter.owner = Some(owner);
            }
            discard_filter = Some(filter);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing discard clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if uses_all_count {
        count = if let Some(filter) = discard_filter.as_ref() {
            Value::Count(filter.clone())
        } else if let Some(owner) = discard_subject_owner_filter(subject) {
            Value::CardsInHand(owner)
        } else {
            return Err(CardTextError::ParseError(format!(
                "missing discard count (clause: '{}')",
                clause_words.join(" ")
            )));
        };
    }

    Ok(EffectAst::Discard {
        count,
        player,
        random,
        filter: discard_filter,
        tag: None,
    })
}

pub(crate) fn parse_discard_color_qualifier_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let qualifier_words: Vec<&str> = crate::cards::builders::parser::lexed_words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if qualifier_words.is_empty() {
        return None;
    }

    let mut colors = crate::color::ColorSet::new();
    let mut saw_color = false;
    for word in qualifier_words {
        if word == "or" {
            continue;
        }
        let color = parse_color(word)?;
        colors = colors.union(color);
        saw_color = true;
    }

    if !saw_color {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.colors = Some(colors);
    Some(filter)
}

pub(crate) fn parse_discard_chosen_color_qualifier_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let qualifier_words: Vec<&str> = crate::cards::builders::parser::lexed_words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !matches!(
        qualifier_words.as_slice(),
        ["of", "that", "color"]
            | ["that", "color"]
            | ["of", "the", "chosen", "color"]
            | ["the", "chosen", "color"]
    ) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.chosen_color = true;
    Some(filter)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DelayedReturnTimingAst {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

pub(crate) fn parse_delayed_return_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    if matches!(
        words,
        ["at", "end", "of", "combat"] | ["at", "the", "end", "of", "combat"]
    ) {
        return Some(DelayedReturnTimingAst::EndOfCombat);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "end", "step"]
            | ["at", "beginning", "of", "the", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "the", "next", "end", "step"]
    ) {
        return Some(DelayedReturnTimingAst::NextEndStep(PlayerFilter::Any));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "your", "next", "end", "step"]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "end",
                "step"
            ]
    ) {
        return Some(DelayedReturnTimingAst::NextEndStep(PlayerFilter::You));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "the", "next", "upkeep"]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::Any));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "your", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "your", "next", "upkeep"]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::You));
    }

    None
}

pub(crate) fn wrap_return_with_delayed_timing(
    effect: EffectAst,
    timing: Option<DelayedReturnTimingAst>,
) -> EffectAst {
    let Some(timing) = timing else {
        return effect;
    };

    match timing {
        DelayedReturnTimingAst::NextEndStep(player) => EffectAst::DelayedUntilNextEndStep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::NextUpkeep(player) => EffectAst::DelayedUntilNextUpkeep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::EndOfCombat => EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        },
    }
}

pub(crate) fn parse_return(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if slice_contains(&clause_words, &"unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported return-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if tokens.first().is_some_and(|token| token.is_word("to"))
        && let Some(rewritten) = rewrite_destination_first_return_clause(tokens)
    {
        return parse_return(&rewritten);
    }

    let mut to_idx = None;
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if !tokens[idx].is_word("to") {
            continue;
        }
        let tail_words = crate::cards::builders::parser::lexed_words(&tokens[idx + 1..]);
        if slice_contains(&tail_words, &"hand")
            || slice_contains(&tail_words, &"hands")
            || slice_contains(&tail_words, &"battlefield")
            || slice_contains(&tail_words, &"graveyard")
            || slice_contains(&tail_words, &"graveyards")
        {
            to_idx = Some(idx);
            break;
        }
    }
    let to_idx = to_idx.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing return destination (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        ))
    })?;

    let mut target_tokens_vec = tokens[..to_idx].to_vec();
    let mut random = false;
    let mut random_idx = 0usize;
    while random_idx + 1 < target_tokens_vec.len() {
        if target_tokens_vec[random_idx].is_word("at")
            && target_tokens_vec[random_idx + 1].is_word("random")
        {
            random = true;
            target_tokens_vec.drain(random_idx..random_idx + 2);
            break;
        }
        random_idx += 1;
    }
    let target_tokens = target_tokens_vec.as_slice();
    let destination_tokens_full = &tokens[to_idx + 1..];
    let destination_words_full =
        crate::cards::builders::parser::lexed_words(destination_tokens_full);
    let mut delayed_timing = None;
    let mut destination_word_cutoff = destination_words_full.len();
    for word_idx in 0..destination_words_full.len() {
        if destination_words_full[word_idx] != "at" {
            continue;
        }
        if let Some(timing) = parse_delayed_return_timing_words(&destination_words_full[word_idx..])
        {
            delayed_timing = Some(timing);
            destination_word_cutoff = word_idx;
            break;
        }
    }

    let destination_tokens = if destination_word_cutoff < destination_words_full.len() {
        let token_cutoff =
            token_index_for_word_index(destination_tokens_full, destination_word_cutoff)
                .unwrap_or(destination_tokens_full.len());
        &destination_tokens_full[..token_cutoff]
    } else {
        destination_tokens_full
    };

    let mut destination_words = crate::cards::builders::parser::lexed_words(destination_tokens);
    let mut destination_excluded_subtypes: Vec<Subtype> = Vec::new();
    if let Some(except_idx) = find_window_index(&destination_words, &["except", "for"]) {
        let exception_words = &destination_words[except_idx + 2..];
        if exception_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing return exception qualifiers (clause: '{}')",
                crate::cards::builders::parser::lexed_words(tokens).join(" ")
            )));
        }
        for word in exception_words {
            if matches!(*word, "and" | "or") {
                continue;
            }
            let Some(subtype) = parse_subtype_word(word)
                .or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported return exception qualifier '{}' (clause: '{}')",
                    word,
                    crate::cards::builders::parser::lexed_words(tokens).join(" ")
                )));
            };
            if !slice_contains(&destination_excluded_subtypes, &subtype) {
                destination_excluded_subtypes.push(subtype);
            }
        }
        if destination_excluded_subtypes.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing subtype return exception qualifiers (clause: '{}')",
                crate::cards::builders::parser::lexed_words(tokens).join(" ")
            )));
        }
        destination_words.truncate(except_idx);
    }
    let is_hand =
        slice_contains(&destination_words, &"hand") || slice_contains(&destination_words, &"hands");
    let is_battlefield = slice_contains(&destination_words, &"battlefield");
    let is_graveyard = slice_contains(&destination_words, &"graveyard")
        || slice_contains(&destination_words, &"graveyards");
    let tapped = slice_contains(&destination_words, &"tapped");
    let transformed = slice_contains(&destination_words_full, &"transformed");
    let converted = slice_contains(&destination_words_full, &"converted");
    let return_controller =
        if find_window_index(&destination_words, &["under", "your", "control"]).is_some() {
            ReturnControllerAst::You
        } else if destination_words
            .iter()
            .any(|word| *word == "owner" || *word == "owners")
            && slice_contains(&destination_words, &"control")
        {
            ReturnControllerAst::Owner
        } else {
            ReturnControllerAst::Preserve
        };
    let has_delayed_timing_words = slice_contains(&destination_words_full, &"beginning")
        || slice_contains(&destination_words_full, &"upkeep")
        || find_window_index(&destination_words_full, &["end", "of", "combat"]).is_some()
        || slice_contains(&destination_words_full, &"end")
            && (slice_contains(&destination_words_full, &"next")
                || slice_contains(&destination_words_full, &"step"));
    if delayed_timing.is_none() && has_delayed_timing_words {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed return timing clause (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        )));
    }
    if !is_hand && !is_battlefield && !is_graveyard {
        return Err(CardTextError::ParseError(format!(
            "unsupported return destination (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        )));
    }

    let target_words = crate::cards::builders::parser::lexed_words(target_tokens);
    if let Some(and_idx) = find_index(target_tokens, |token| token.is_word("and"))
        && and_idx > 0
    {
        let tail_words = crate::cards::builders::parser::lexed_words(&target_tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (slice_starts_with(&tail_words, &["up", "to"])
                && slice_contains(&tail_words, &"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target return clause (clause: '{}')",
                crate::cards::builders::parser::lexed_words(tokens).join(" ")
            )));
        }
    }
    if !slice_contains(&target_words, &"target")
        && slice_contains(&target_words, &"exiled")
        && slice_contains(&target_words, &"cards")
    {
        let filter = parse_object_filter(target_tokens, false)?;
        let effect = if is_battlefield {
            EffectAst::ReturnAllToBattlefield { filter, tapped }
        } else if is_graveyard {
            EffectAst::MoveToZone {
                target: TargetAst::Object(filter, None, None),
                zone: Zone::Graveyard,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            }
        } else {
            EffectAst::ReturnAllToHand { filter }
        };
        return Ok(wrap_return_with_delayed_timing(effect, delayed_timing));
    }
    if target_words
        .first()
        .is_some_and(|word| *word == "all" || *word == "each")
    {
        let has_unsupported_return_all_qualifier = slice_contains(&target_words, &"dealt")
            || slice_contains(&target_words, &"without")
                && slice_contains(&target_words, &"counter");
        if has_unsupported_return_all_qualifier {
            return Err(CardTextError::ParseError(format!(
                "unsupported qualified return-all filter (clause: '{}')",
                crate::cards::builders::parser::lexed_words(tokens).join(" ")
            )));
        }
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing return-all filter".to_string(),
            ));
        }
        let return_filter_tokens = &target_tokens[1..];
        if is_hand
            && let Some((choice_idx, consumed)) = find_color_choice_phrase(return_filter_tokens)
        {
            let base_filter_tokens = trim_commas(&return_filter_tokens[..choice_idx]);
            let trailing = trim_commas(&return_filter_tokens[choice_idx + consumed..]);
            if !trailing.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing color-choice return-all clause (clause: '{}')",
                    crate::cards::builders::parser::lexed_words(tokens).join(" ")
                )));
            }
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter before color-choice clause (clause: '{}')",
                    crate::cards::builders::parser::lexed_words(tokens).join(" ")
                )));
            }
            let mut filter = parse_object_filter(&base_filter_tokens, false)?;
            for subtype in destination_excluded_subtypes {
                if !slice_contains(&filter.excluded_subtypes, &subtype) {
                    filter.excluded_subtypes.push(subtype);
                }
            }
            return Ok(wrap_return_with_delayed_timing(
                EffectAst::ReturnAllToHandOfChosenColor { filter },
                delayed_timing,
            ));
        }
        let (return_filter_tokens, chosen_this_way_excluded) =
            if let Some((base_tokens, excluded)) =
                split_chosen_this_way_qualifier(return_filter_tokens)
            {
                (base_tokens, Some(excluded))
            } else {
                (return_filter_tokens.to_vec(), None)
            };
        let (base_filter_tokens, chosen_creature_type, excluded_chosen_creature_type) =
            if let Some((base_tokens, chosen_type, excluded_chosen_type)) =
                split_chosen_creature_type_qualifier(&return_filter_tokens)
            {
                (base_tokens, chosen_type, excluded_chosen_type)
            } else {
                (return_filter_tokens, false, false)
            };
        let mut filter = parse_object_filter(&base_filter_tokens, false)?;
        filter.chosen_creature_type |= chosen_creature_type;
        filter.excluded_chosen_creature_type |= excluded_chosen_creature_type;
        for subtype in destination_excluded_subtypes {
            if !slice_contains(&filter.excluded_subtypes, &subtype) {
                filter.excluded_subtypes.push(subtype);
            }
        }
        if let Some(excluded) = chosen_this_way_excluded {
            filter = if excluded {
                filter.not_tagged(TagKey::from(IT_TAG))
            } else {
                filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject)
            };
        }
        let effect = if is_battlefield {
            EffectAst::ReturnAllToBattlefield { filter, tapped }
        } else if is_graveyard {
            EffectAst::MoveToZone {
                target: TargetAst::Object(filter, None, None),
                zone: Zone::Graveyard,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            }
        } else {
            EffectAst::ReturnAllToHand { filter }
        };
        return Ok(wrap_return_with_delayed_timing(effect, delayed_timing));
    }
    if !destination_excluded_subtypes.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported return exception on non-return-all clause (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        )));
    }

    let target = if matches!(
        target_words.as_slice(),
        ["it"] | ["them"] | ["that", "card"] | ["those", "cards"]
    ) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
    } else {
        parse_target_phrase(target_tokens)?
    };
    let effect = if is_battlefield {
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            transformed,
            converted,
            controller: return_controller,
        }
    } else if is_graveyard {
        EffectAst::MoveToZone {
            target,
            zone: Zone::Graveyard,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        }
    } else {
        EffectAst::ReturnToHand { target, random }
    };
    Ok(wrap_return_with_delayed_timing(effect, delayed_timing))
}

fn rewrite_destination_first_return_clause(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let hand_or_battlefield_idx = find_index(&clause_words, |word| {
        matches!(*word, "hand" | "hands" | "battlefield")
    })?;
    let mut split_word_idx = hand_or_battlefield_idx + 1;

    if clause_words.get(split_word_idx).copied() == Some("under") {
        let control_rel_idx = find_index(&clause_words[split_word_idx + 1..], |word| {
            *word == "control"
        })?;
        split_word_idx = split_word_idx + 1 + control_rel_idx + 1;
    }

    while clause_words
        .get(split_word_idx)
        .is_some_and(|word| *word == "tapped")
    {
        split_word_idx += 1;
    }

    let split_token_idx = token_index_for_word_index(tokens, split_word_idx)?;
    if split_token_idx >= tokens.len() {
        return None;
    }

    let target_tokens = trim_commas(&tokens[split_token_idx..]);
    let destination_tokens = trim_commas(&tokens[..split_token_idx]);
    if target_tokens.is_empty() || destination_tokens.is_empty() {
        return None;
    }

    let mut rewritten = target_tokens.to_vec();
    rewritten.extend(destination_tokens.to_vec());
    Some(rewritten)
}

fn parse_exchange_life_totals_player(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    match crate::cards::builders::parser::lexed_words(tokens).as_slice() {
        ["you"] => Some(PlayerAst::You),
        ["target", "player"] | ["target", "players"] => Some(PlayerAst::Target),
        ["target", "opponent"] | ["target", "opponents"] => Some(PlayerAst::TargetOpponent),
        ["that", "player"] | ["that", "players"] => Some(PlayerAst::That),
        ["opponent"] | ["opponents"] | ["an", "opponent"] => Some(PlayerAst::Opponent),
        _ => None,
    }
}

fn parse_exchange_shared_type_clause(
    tokens: &[OwnedLexToken],
) -> Result<(&[OwnedLexToken], Option<SharedTypeConstraintAst>), CardTextError> {
    let tail_words = crate::cards::builders::parser::lexed_words(tokens);
    let Some(rel_word_idx) = find_window_by(&tail_words, 2, |window| {
        window[0] == "that" && matches!(window[1], "share" | "shares")
    }) else {
        return Ok((tokens, None));
    };

    let rel_token_idx = token_index_for_word_index(tokens, rel_word_idx).unwrap_or(tokens.len());
    let (head, tail) = tokens.split_at(rel_token_idx);
    let share_words = crate::cards::builders::parser::lexed_words(tail);
    let share_head = if slice_starts_with(&share_words, &["that", "share"]) {
        &share_words[2..]
    } else if slice_starts_with(&share_words, &["that", "shares"]) {
        &share_words[2..]
    } else {
        &share_words[..]
    };
    let share_head = if share_head.first().copied() == Some("a") {
        &share_head[1..]
    } else {
        share_head
    };

    let shared_type = if slice_starts_with(&share_head, &["permanent", "type"])
        || slice_starts_with(&share_head, &["one", "of", "those", "permanent", "types"])
    {
        SharedTypeConstraintAst::PermanentType
    } else if slice_starts_with(&share_head, &["card", "type"])
        || slice_starts_with(&share_head, &["one", "of", "those", "types"])
    {
        SharedTypeConstraintAst::CardType
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange share-type clause (clause: '{}')",
            tail_words.join(" ")
        )));
    };

    Ok((head, Some(shared_type)))
}

fn parse_exchange_zone_owner_prefix(words: &[&str]) -> Option<(PlayerAst, usize)> {
    if slice_starts_with(&words, &["your"]) {
        return Some((PlayerAst::You, 1));
    }
    if slice_starts_with(&words, &["target", "player"])
        || slice_starts_with(&words, &["target", "players"])
    {
        return Some((PlayerAst::Target, 2));
    }
    if slice_starts_with(&words, &["target", "opponent"])
        || slice_starts_with(&words, &["target", "opponents"])
    {
        return Some((PlayerAst::TargetOpponent, 2));
    }
    if slice_starts_with(&words, &["an", "opponent"])
        || slice_starts_with(&words, &["opponent"])
        || slice_starts_with(&words, &["opponents"])
    {
        return Some((PlayerAst::Opponent, if words[0] == "an" { 2 } else { 1 }));
    }
    None
}

fn parse_exchange_zones(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let (player, consumed) = parse_exchange_zone_owner_prefix(&clause_words)?;
    let zone1 = parse_zone_word(*clause_words.get(consumed)?)?;
    if clause_words.get(consumed + 1).copied() != Some("and") {
        return None;
    }
    let zone2 = parse_zone_word(*clause_words.get(consumed + 2)?)?;
    if consumed + 3 != clause_words.len() {
        return None;
    }
    Some(EffectAst::ExchangeZones {
        player,
        zone1,
        zone2,
    })
}

fn parse_exchange_value_operand(tokens: &[OwnedLexToken]) -> Option<ExchangeValueAst> {
    match crate::cards::builders::parser::lexed_words(tokens).as_slice() {
        ["your", "life", "total"] => return Some(ExchangeValueAst::LifeTotal(PlayerAst::You)),
        ["target", "player", "life", "total"]
        | ["target", "players", "life", "total"]
        | ["target", "player's", "life", "total"]
        | ["target", "players'", "life", "total"] => {
            return Some(ExchangeValueAst::LifeTotal(PlayerAst::Target));
        }
        ["target", "opponent", "life", "total"]
        | ["target", "opponents", "life", "total"]
        | ["target", "opponent's", "life", "total"]
        | ["target", "opponents'", "life", "total"] => {
            return Some(ExchangeValueAst::LifeTotal(PlayerAst::TargetOpponent));
        }
        ["an", "opponent", "life", "total"]
        | ["opponent", "life", "total"]
        | ["opponents", "life", "total"] => {
            return Some(ExchangeValueAst::LifeTotal(PlayerAst::Opponent));
        }
        ["its", "power"]
        | ["this", "power"]
        | ["thiss", "power"]
        | ["this's", "power"]
        | ["this", "creature", "power"]
        | ["this", "creature's", "power"]
        | ["thiss", "creature", "power"]
        | ["thiss", "creature's", "power"]
        | ["this", "creatures", "power"]
        | ["thiss", "creatures", "power"] => {
            return Some(ExchangeValueAst::Stat {
                target: TargetAst::Source(span_from_tokens(tokens)),
                kind: ExchangeValueKindAst::Power,
            });
        }
        ["its", "toughness"]
        | ["this", "toughness"]
        | ["thiss", "toughness"]
        | ["this's", "toughness"]
        | ["this", "creature", "toughness"]
        | ["this", "creature's", "toughness"]
        | ["thiss", "creature", "toughness"]
        | ["thiss", "creature's", "toughness"]
        | ["this", "creatures", "toughness"]
        | ["thiss", "creatures", "toughness"] => {
            return Some(ExchangeValueAst::Stat {
                target: TargetAst::Source(span_from_tokens(tokens)),
                kind: ExchangeValueKindAst::Toughness,
            });
        }
        _ => {}
    }

    let token_words = crate::cards::builders::parser::lexed_words(tokens);
    let power_prefix = if slice_starts_with(&token_words, &["the", "power", "of"]) {
        Some((ExchangeValueKindAst::Power, 3))
    } else if slice_starts_with(&token_words, &["power", "of"]) {
        Some((ExchangeValueKindAst::Power, 2))
    } else if slice_starts_with(&token_words, &["the", "toughness", "of"]) {
        Some((ExchangeValueKindAst::Toughness, 3))
    } else if slice_starts_with(&token_words, &["toughness", "of"]) {
        Some((ExchangeValueKindAst::Toughness, 2))
    } else {
        None
    }?;

    let (kind, used) = power_prefix;
    let target = parse_target_phrase(&tokens[used..]).ok()?;
    Some(ExchangeValueAst::Stat { target, kind })
}

fn parse_exchange_values(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let (duration, remainder) =
        if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, trim_commas(tokens).to_vec())
        };

    let split_idx = find_index(&remainder, |token: &OwnedLexToken| {
        token.is_word("with") || token.is_word("and")
    })
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        ))
    })?;
    let left_tokens = trim_commas(&remainder[..split_idx]);
    let right_tokens = trim_commas(&remainder[split_idx + 1..]);
    let left = parse_exchange_value_operand(&left_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported exchange value operand (clause: '{}')",
            crate::cards::builders::parser::lexed_words(&left_tokens).join(" ")
        ))
    })?;
    let right = parse_exchange_value_operand(&right_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported exchange value operand (clause: '{}')",
            crate::cards::builders::parser::lexed_words(&right_tokens).join(" ")
        ))
    })?;

    Ok(EffectAst::ExchangeValues {
        left,
        right,
        duration,
    })
}

fn parse_exchange_life_totals(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if clause_words.as_slice() == ["life", "totals"] {
        return match subject {
            Some(SubjectAst::Player(PlayerAst::Target)) => Ok(EffectAst::ExchangeLifeTotals {
                player1: PlayerAst::Target,
                player2: PlayerAst::Target,
            }),
            _ => Err(CardTextError::ParseError(format!(
                "unsupported life-total exchange clause (clause: '{}')",
                clause_words.join(" ")
            ))),
        };
    }

    if !slice_starts_with(&clause_words, &["life", "totals", "with"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let player2 = parse_exchange_life_totals_player(&tokens[3..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported life-total exchange partner (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let player1 = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::You,
    };

    Ok(EffectAst::ExchangeLifeTotals { player1, player2 })
}

fn parse_exchange_text_boxes(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let remainder = if slice_starts_with(&clause_words, &["the", "text", "boxes", "of"]) {
        &tokens[4..]
    } else if slice_starts_with(&clause_words, &["text", "boxes", "of"]) {
        &tokens[3..]
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported text-box exchange clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let target = parse_target_phrase(remainder).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported text-box exchange target (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(EffectAst::ExchangeTextBoxes { target })
}

pub(crate) fn parse_exchange(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if slice_starts_with(&clause_words, &["life", "totals"]) {
        return parse_exchange_life_totals(tokens, subject);
    }
    if slice_starts_with(&clause_words, &["the", "text", "boxes", "of"])
        || slice_starts_with(&clause_words, &["text", "boxes", "of"])
    {
        return parse_exchange_text_boxes(tokens);
    }
    if let Some(effect) = parse_exchange_zones(tokens) {
        return Ok(effect);
    }
    if !slice_starts_with(&clause_words, &["control", "of"]) {
        if slice_contains(&clause_words, &"life")
            || slice_contains(&clause_words, &"power")
            || slice_contains(&clause_words, &"toughness")
        {
            return parse_exchange_values(tokens);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(and_idx) = find_index(tokens, |token| token.is_word("and")) {
        let left_target = parse_target_phrase(&tokens[2..and_idx]).ok();
        let (right_tokens, shared_type) =
            parse_exchange_shared_type_clause(&tokens[and_idx + 1..])?;
        let right_target = parse_target_phrase(right_tokens).ok();
        if let (Some(permanent1), Some(permanent2)) = (left_target, right_target) {
            return Ok(EffectAst::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                shared_type,
            });
        }
    }

    let mut idx = 2usize;
    let mut count = 2u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing exchange target filter".to_string(),
        ));
    }

    let (filter_tokens, shared_type) = parse_exchange_shared_type_clause(&tokens[idx..])?;

    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(EffectAst::ExchangeControl {
        filter,
        count,
        shared_type,
    })
}

pub(crate) fn parse_become(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported become clause (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        )));
    };

    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if clause_words.as_slice() == ["the", "monarch"] || clause_words.as_slice() == ["monarch"] {
        return Ok(EffectAst::BecomeMonarch { player });
    }

    let amount = parse_value(tokens)
        .map(|(value, _)| value)
        .or_else(|| parse_half_starting_life_total_value(tokens, player))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing life total amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    Ok(EffectAst::SetLifeTotal { amount, player })
}

fn player_filter_for_set_life_total_reference(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::ThatPlayerOrTargetController
        | PlayerAst::ItsController
        | PlayerAst::ItsOwner => None,
    }
}

pub(crate) fn parse_half_starting_life_total_value(
    tokens: &[OwnedLexToken],
    player: PlayerAst,
) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let inferred_player_filter = || match clause_words.as_slice() {
        ["half", "your", "starting", "life", "total"]
        | ["half", "your", "starting", "life", "total", "rounded", "up"]
        | [
            "half",
            "your",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => Some(PlayerFilter::You),
        ["half", "target", "players", "starting", "life", "total"]
        | [
            "half",
            "target",
            "players",
            "starting",
            "life",
            "total",
            "rounded",
            "up",
        ]
        | [
            "half",
            "target",
            "players",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => Some(PlayerFilter::target_player()),
        ["half", "an", "opponents", "starting", "life", "total"]
        | [
            "half",
            "an",
            "opponents",
            "starting",
            "life",
            "total",
            "rounded",
            "up",
        ]
        | [
            "half",
            "an",
            "opponents",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => Some(PlayerFilter::Opponent),
        _ => None,
    };
    let player_filter =
        player_filter_for_set_life_total_reference(player).or_else(inferred_player_filter)?;

    let rounded_up = match clause_words.as_slice() {
        ["half", "your", "starting", "life", "total"]
        | ["half", "your", "starting", "life", "total", "rounded", "up"] => {
            player_filter == PlayerFilter::You
        }
        ["half", "target", "players", "starting", "life", "total"]
        | [
            "half",
            "target",
            "players",
            "starting",
            "life",
            "total",
            "rounded",
            "up",
        ] => player_filter == PlayerFilter::target_player(),
        ["half", "an", "opponents", "starting", "life", "total"]
        | [
            "half",
            "an",
            "opponents",
            "starting",
            "life",
            "total",
            "rounded",
            "up",
        ] => player_filter == PlayerFilter::Opponent,
        _ => false,
    };
    if rounded_up {
        return Some(Value::HalfStartingLifeTotalRoundedUp(player_filter));
    }

    let rounded_down = match clause_words.as_slice() {
        [
            "half",
            "your",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => player_filter == PlayerFilter::You,
        [
            "half",
            "target",
            "players",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => player_filter == PlayerFilter::target_player(),
        [
            "half",
            "an",
            "opponents",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => player_filter == PlayerFilter::Opponent,
        _ => false,
    };
    if rounded_down {
        return Some(Value::HalfStartingLifeTotalRoundedDown(player_filter));
    }

    None
}

pub(crate) fn parse_switch(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    use crate::effect::Until;

    let clause_words = crate::cards::builders::parser::lexed_words(tokens);

    // Split off trailing duration, if present.
    let (duration, remainder) =
        if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, trim_commas(tokens).to_vec())
        };

    let remainder_words = crate::cards::builders::parser::lexed_words(&remainder);
    let Some(power_idx) = find_index(&remainder, |token| token.is_word("power")) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    // Target phrase is everything up to "power".
    let target_tokens = &remainder[..power_idx];
    let target_words = crate::cards::builders::parser::lexed_words(target_tokens);
    let target = if target_words.is_empty()
        || matches!(
            target_words.as_slice(),
            ["this"]
                | ["this", "creature"]
                | ["this", "creatures"]
                | ["this", "permanent"]
                | ["it"]
        ) {
        if target_words == ["it"] {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
        } else {
            TargetAst::Source(span_from_tokens(target_tokens))
        }
    } else {
        parse_target_phrase(target_tokens)?
    };

    // Require "... power and toughness ..." somewhere in remainder.
    if !slice_contains(&remainder_words, &"power")
        || !slice_contains(&remainder_words, &"toughness")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::SwitchPowerToughness { target, duration })
}

pub(crate) fn parse_skip(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let (player, words) = match subject {
        Some(SubjectAst::Player(player)) => (player, clause_words),
        _ => {
            if slice_starts_with(&clause_words, &["your"]) {
                (PlayerAst::You, clause_words[1..].to_vec())
            } else if slice_starts_with(&clause_words, &["their"]) {
                (PlayerAst::That, clause_words[1..].to_vec())
            } else if slice_starts_with(&clause_words, &["that", "player"])
                || slice_starts_with(&clause_words, &["that", "players"])
            {
                (PlayerAst::That, clause_words[2..].to_vec())
            } else if slice_starts_with(&clause_words, &["his", "or", "her"]) {
                (PlayerAst::That, clause_words[3..].to_vec())
            } else if slice_starts_with(&clause_words, &["target", "player"])
                || slice_starts_with(&clause_words, &["target", "players"])
            {
                (PlayerAst::Target, clause_words[2..].to_vec())
            } else if slice_starts_with(&clause_words, &["target", "opponent"])
                || slice_starts_with(&clause_words, &["target", "opponents"])
            {
                (PlayerAst::TargetOpponent, clause_words[2..].to_vec())
            } else if slice_starts_with(&clause_words, &["that", "turn"])
                || slice_starts_with(&clause_words, &["turn"])
            {
                (PlayerAst::Implicit, clause_words)
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    let skips_next_combat_phase_this_turn = slice_contains(&words, &"combat")
        && slice_contains(&words, &"phase")
        && slice_contains(&words, &"next")
        && slice_contains(&words, &"this")
        && slice_contains(&words, &"turn");
    if skips_next_combat_phase_this_turn {
        return Ok(EffectAst::SkipNextCombatPhaseThisTurn { player });
    }
    if slice_contains(&words, &"combat")
        && (slice_contains(&words, &"phase") || slice_contains(&words, &"phases"))
        && slice_contains(&words, &"turn")
    {
        return Ok(EffectAst::SkipCombatPhases { player });
    }
    if slice_contains(&words, &"draw") && slice_contains(&words, &"step") {
        return Ok(EffectAst::SkipDrawStep { player });
    }
    if slice_contains(&words, &"turn") {
        return Ok(EffectAst::SkipTurn { player });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported skip clause (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn parse_transform(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Transform {
            target: TargetAst::Source(None),
        });
    }
    let target_words = crate::cards::builders::parser::lexed_words(tokens);
    if target_words == ["it"]
        || target_words == ["this"]
        || target_words == ["this", "creature"]
        || target_words == ["this", "land"]
        || target_words == ["this", "permanent"]
    {
        return Ok(EffectAst::Transform {
            target: TargetAst::Source(span_from_tokens(tokens)),
        });
    }
    let target = match parse_target_phrase(tokens) {
        Ok(target) => target,
        Err(_)
            if target_words.len() <= 3
                && !target_words.iter().any(|word| {
                    matches!(
                        *word,
                        "target" | "another" | "other" | "each" | "all" | "that" | "those"
                    )
                }) =>
        {
            TargetAst::Source(span_from_tokens(tokens))
        }
        Err(err) => return Err(err),
    };
    Ok(EffectAst::Transform { target })
}

pub(crate) fn parse_convert(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Convert {
            target: TargetAst::Source(None),
        });
    }
    let target_words = crate::cards::builders::parser::lexed_words(tokens);
    if target_words == ["it"]
        || target_words == ["this"]
        || target_words == ["this", "creature"]
        || target_words == ["this", "land"]
        || target_words == ["this", "permanent"]
    {
        return Ok(EffectAst::Convert {
            target: TargetAst::Source(span_from_tokens(tokens)),
        });
    }
    let target = match parse_target_phrase(tokens) {
        Ok(target) => target,
        Err(_)
            if target_words.len() <= 3
                && !target_words.iter().any(|word| {
                    matches!(
                        *word,
                        "target" | "another" | "other" | "each" | "all" | "that" | "those"
                    )
                }) =>
        {
            TargetAst::Source(span_from_tokens(tokens))
        }
        Err(err) => return Err(err),
    };
    Ok(EffectAst::Convert { target })
}

pub(crate) fn parse_flip(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Flip {
            target: TargetAst::Source(None),
        });
    }

    let target_words = crate::cards::builders::parser::lexed_words(tokens);
    if target_words == ["it"]
        || target_words == ["this"]
        || target_words == ["this", "creature"]
        || target_words == ["this", "permanent"]
    {
        return Ok(EffectAst::Flip {
            target: TargetAst::Source(span_from_tokens(tokens)),
        });
    }

    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Flip { target })
}

pub(crate) fn parse_regenerate(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let words = crate::cards::builders::parser::lexed_words(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        if tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "regenerate clause missing filter after each/all".to_string(),
            ));
        }
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::RegenerateAll { filter });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Regenerate { target })
}

pub(crate) fn parse_mill(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let starts_with_card_keyword = tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "card" || word == "cards");

    let (count, used) = if slice_starts_with(&clause_words, &["that", "many"]) {
        (Value::EventValue(EventValueSpec::Amount), 2usize)
    } else if starts_with_card_keyword {
        if let Some((count, used_after_cards)) = parse_value(&tokens[1..]) {
            (count, 1 + used_after_cards)
        } else if let Some(count) = parse_add_mana_equal_amount_value(&tokens[1..]) {
            // Mill clauses like "cards equal to its toughness" place the amount after "cards".
            (count, tokens.len())
        } else {
            return Err(CardTextError::ParseError(format!(
                "missing mill count (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    } else {
        parse_value(tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing mill count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    let rest = &tokens[used..];
    if starts_with_card_keyword {
        let trailing_words: Vec<&str> = rest.iter().filter_map(OwnedLexToken::as_word).collect();
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mill clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    } else {
        if rest
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word != "card" && word != "cards")
        {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        let trailing_words: Vec<&str> = rest
            .iter()
            .skip(1)
            .filter_map(OwnedLexToken::as_word)
            .collect();
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mill clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(EffectAst::Mill { count, player })
}

pub(crate) fn parse_equal_to_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words_all = crate::cards::builders::parser::lexed_words(tokens);
    let equal_idx = find_window_index(&words_all, &["equal", "to"])?;
    let mut number_word_idx = equal_idx + 2;
    if words_all.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if words_all.get(number_word_idx).copied() != Some("number")
        || words_all.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }

    let value_start_token_idx = token_index_for_word_index(tokens, number_word_idx)?;
    let value_tokens = trim_edge_punctuation(&tokens[value_start_token_idx..]);
    if let Some((value, used)) = parse_value(&value_tokens)
        && crate::cards::builders::parser::lexed_words(&value_tokens[used..]).is_empty()
    {
        return Some(value);
    }

    let filter_start_word_idx = number_word_idx + 2;
    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let filter_tokens = trim_edge_punctuation(&tokens[filter_start_token_idx..]);
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        return Some(value);
    }
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if !slice_starts_with(&clause_words, &["equal", "to"]) {
        return None;
    }

    let mut number_word_idx = 2usize;
    if clause_words.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if clause_words.get(number_word_idx).copied() != Some("number")
        || clause_words.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }

    let filter_start_word_idx = number_word_idx + 2;
    let operator_word_idx = find_index(&clause_words[filter_start_word_idx + 1..], |word| {
        matches!(*word, "plus" | "minus")
    })
    .map(|offset| filter_start_word_idx + 1 + offset)?;
    let operator = clause_words[operator_word_idx];

    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let operator_token_idx = token_index_for_word_index(tokens, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_start_token_idx..operator_token_idx]);
    let base_value =
        if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
            value
        } else {
            Value::Count(parse_object_filter(&filter_tokens, false).ok()?)
        };

    let offset_start_token_idx = token_index_for_word_index(tokens, operator_word_idx + 1)?;
    let offset_tokens = trim_commas(&tokens[offset_start_token_idx..]);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    let trailing_words = crate::cards::builders::parser::lexed_words(&offset_tokens[used..]);
    if !trailing_words.is_empty() {
        return None;
    }

    let signed_offset = if operator == "minus" {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(base_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

fn parse_spells_cast_this_turn_matching_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let filter_words = crate::cards::builders::parser::lexed_words(tokens);
    if !(slice_contains(&filter_words, &"spell") || slice_contains(&filter_words, &"spells"))
        || !(slice_contains(&filter_words, &"cast") || slice_contains(&filter_words, &"casts"))
        || !slice_contains(&filter_words, &"this")
        || !slice_contains(&filter_words, &"turn")
    {
        return None;
    }

    let suffix_patterns: &[(&[&str], PlayerFilter)] = &[
        (
            &["theyve", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["they", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["that", "player", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (&["youve", "cast", "this", "turn"], PlayerFilter::You),
        (&["you", "cast", "this", "turn"], PlayerFilter::You),
        (
            &["an", "opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponents", "have", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (&["cast", "this", "turn"], PlayerFilter::Any),
    ];

    for (suffix, player) in suffix_patterns {
        if !slice_ends_with(&filter_words, suffix) {
            continue;
        }
        let filter_word_len = filter_words.len().saturating_sub(suffix.len());
        let filter_token_end =
            token_index_for_word_index(tokens, filter_word_len).unwrap_or(tokens.len());
        let filter_tokens = trim_commas(&tokens[..filter_token_end]);
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        let exclude_source = filter_tokens.iter().any(|token| token.is_word("other"));
        return Some(Value::SpellsCastThisTurnMatching {
            player: player.clone(),
            filter,
            exclude_source,
        });
    }

    None
}

pub(crate) fn parse_equal_to_number_of_opponents_you_have_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if matches!(
        clause_words.as_slice(),
        [
            "equal",
            "to",
            "the",
            "number",
            "of",
            "opponents",
            "you",
            "have"
        ] | ["equal", "to", "number", "of", "opponents", "you", "have"]
    ) {
        return Some(Value::CountPlayers(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_equal_to_number_of_counters_on_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if !slice_starts_with(&clause_words, &["equal", "to"]) {
        return None;
    }

    let mut idx = 2usize;
    if clause_words.get(idx).copied() == Some("the") {
        idx += 1;
    }
    if clause_words.get(idx).copied() != Some("number")
        || clause_words.get(idx + 1).copied() != Some("of")
    {
        return None;
    }
    idx += 2;

    if clause_words
        .get(idx)
        .is_some_and(|word| is_article(word) || *word == "one")
    {
        idx += 1;
    }

    let mut counter_type = None;
    if let Some(word) = clause_words.get(idx).copied()
        && let Some(parsed) = parse_counter_type_word(word)
    {
        counter_type = Some(parsed);
        idx += 1;
    }

    if !matches!(clause_words.get(idx).copied(), Some("counter" | "counters")) {
        return None;
    }
    idx += 1;

    if clause_words.get(idx).copied() != Some("on") {
        return None;
    }
    idx += 1;

    let reference = &clause_words[idx..];
    if reference.is_empty() {
        return None;
    }

    if matches!(
        reference,
        ["it"] | ["this"] | ["this", "creature"] | ["this", "permanent"] | ["this", "source"]
    ) {
        return Some(match counter_type {
            Some(counter_type) => Value::CountersOnSource(counter_type),
            None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
        });
    }

    if matches!(
        reference,
        ["that"]
            | ["that", "creature"]
            | ["that", "permanent"]
            | ["that", "object"]
            | ["those"]
            | ["those", "creatures"]
            | ["those", "permanents"]
    ) {
        return Some(Value::CountersOn(
            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            counter_type,
        ));
    }

    None
}

pub(crate) fn parse_equal_to_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let equal_idx = find_window_index(&clause_words, &["equal", "to"])?;

    let mut idx = equal_idx + 2;
    if clause_words.get(idx).copied() == Some("the") {
        idx += 1;
    }

    let aggregate = match clause_words.get(idx).copied() {
        Some("total") => "total",
        Some("greatest") => "greatest",
        _ => return None,
    };
    idx += 1;

    let value_kind = if clause_words.get(idx).copied() == Some("power") {
        idx += 1;
        "power"
    } else if clause_words.get(idx).copied() == Some("toughness") {
        idx += 1;
        "toughness"
    } else if clause_words.get(idx).copied() == Some("mana")
        && clause_words.get(idx + 1).copied() == Some("value")
    {
        idx += 2;
        "mana_value"
    } else {
        return None;
    };

    if !matches!(clause_words.get(idx).copied(), Some("of" | "among")) {
        return None;
    }
    idx += 1;

    let object_start_token_idx = token_index_for_word_index(tokens, idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "toughness") => Some(Value::GreatestToughness(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_get(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if slice_contains(&clause_words, &"poison")
        && (slice_contains(&clause_words, &"counter") || slice_contains(&clause_words, &"counters"))
    {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = if matches!(
            clause_words.first().copied(),
            Some("a" | "an" | "another" | "one")
        ) {
            Value::Fixed(1)
        } else {
            parse_value(tokens)
                .map(|(value, _)| value)
                .unwrap_or(Value::Fixed(1))
        };
        return Ok(EffectAst::PoisonCounters { count, player });
    }

    let energy_count = tokens
        .iter()
        .filter(|token| {
            token.is_word("e")
                || (token.kind == TokenKind::ManaGroup
                    && token
                        .slice
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .eq_ignore_ascii_case("e"))
        })
        .count();
    if energy_count > 0 {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = parse_add_mana_equal_amount_value(tokens)
            .or(parse_equal_to_number_of_filter_value(tokens))
            .or(parse_dynamic_cost_modifier_value(tokens)?)
            .unwrap_or(Value::Fixed(energy_count as i32));
        return Ok(EffectAst::EnergyCounters { count, player });
    }

    if clause_words.as_slice() == ["tk"] {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        return Ok(EffectAst::EnergyCounters {
            count: Value::Fixed(1),
            player,
        });
    }

    if slice_starts_with(&clause_words, &["an", "emblem", "with"])
        || slice_starts_with(&clause_words, &["emblem", "with"])
    {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let text_words = if clause_words[0] == "an" {
            &clause_words[3..]
        } else {
            &clause_words[2..]
        };
        if text_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing emblem text (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let text = if slice_starts_with(&text_words, &["at", "the", "beginning", "of"])
            && let Some(this_idx) = find_index(&text_words, |word| *word == "this")
        {
            let head = text_words[..this_idx].join(" ");
            let tail = text_words[this_idx..].join(" ");
            format!(
                "{}{}, {}.",
                head[..1].to_ascii_uppercase(),
                &head[1..],
                tail
            )
        } else {
            let joined = text_words.join(" ");
            format!("{}{}.", joined[..1].to_ascii_uppercase(), &joined[1..])
        };
        return Ok(EffectAst::CreateEmblem { player, text });
    }

    let modifier_start = if slice_starts_with(&clause_words, &["an", "additional"]) {
        2usize
    } else if slice_starts_with(&clause_words, &["additional"]) {
        1usize
    } else {
        0usize
    };
    if modifier_start > 0
        && let Some(mod_token) = tokens.get(modifier_start).and_then(OwnedLexToken::as_word)
        && let Ok((power_per, toughness_per)) = parse_pt_modifier(mod_token)
    {
        let tail_tokens = tokens.get(modifier_start + 1..).unwrap_or_default();
        let tail_words = crate::cards::builders::parser::lexed_words(tail_tokens);
        if slice_starts_with(&tail_words, &["until", "end", "of", "turn", "for", "each"]) {
            let filter_tokens = &tail_tokens[6..];
            let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported get-for-each filter (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            let target = match subject {
                Some(SubjectAst::This) => TargetAst::Source(None),
                _ => {
                    return Err(CardTextError::ParseError(
                        "unsupported get clause (missing subject)".to_string(),
                    ));
                }
            };
            return Ok(EffectAst::PumpForEach {
                power_per,
                toughness_per,
                target,
                count: Value::Count(filter),
                duration: Until::EndOfTurn,
            });
        }
    }

    if let Some(mod_token) = tokens.first().and_then(OwnedLexToken::as_word)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        let (power, toughness, duration, condition) =
            parse_get_modifier_values_with_tail(tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
            condition,
        });
    }

    if let Some(collapsed_tokens) = collapse_leading_signed_pt_modifier_tokens(tokens)
        && let Some(mod_token) = collapsed_tokens.first().and_then(OwnedLexToken::as_word)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        let (power, toughness, duration, condition) =
            parse_get_modifier_values_with_tail(&collapsed_tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
            condition,
        });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported get clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_add_mana(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    parser_trace_stack("parse_add_mana:entry", tokens);
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let wrap_instead_if_tail = |base_effect: EffectAst,
                                tail_tokens: &[OwnedLexToken]|
     -> Result<Option<EffectAst>, CardTextError> {
        let tail_words = crate::cards::builders::parser::lexed_words(tail_tokens);
        if !slice_starts_with(&tail_words, &["instead", "if"]) {
            return Ok(None);
        }
        let predicate_tokens = trim_commas(&tail_tokens[2..]);
        if predicate_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let predicate = parse_predicate(&predicate_tokens).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        Ok(Some(EffectAst::Conditional {
            predicate,
            if_true: vec![base_effect],
            if_false: Vec::new(),
        }))
    };

    let has_card_word = clause_words
        .iter()
        .any(|word| *word == "card" || *word == "cards");
    if slice_contains(&clause_words, &"exiled")
        && has_card_word
        && slice_contains(&clause_words, &"colors")
    {
        return Ok(EffectAst::AddManaImprintedColors);
    }

    if (slice_contains(&clause_words, &"commander") || slice_contains(&clause_words, &"commanders"))
        && slice_contains(&clause_words, &"color")
        && slice_contains(&clause_words, &"identity")
    {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::AddManaCommanderIdentity { amount, player });
    }

    if let Some(available_colors) = parse_any_combination_mana_colors(tokens)? {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::AddManaAnyColor {
            amount,
            player,
            available_colors: Some(available_colors),
        });
    }

    if let Some(available_colors) = parse_or_mana_color_choices(tokens)? {
        return Ok(EffectAst::AddManaAnyColor {
            amount: Value::Fixed(1),
            player,
            available_colors: Some(available_colors),
        });
    }

    // "Add one mana of the chosen color."
    let has_explicit_symbol = tokens
        .iter()
        .any(|token| mana_pips_from_token(token).is_some());
    if !has_explicit_symbol
        && let Some(chosen_idx) = find_window_index(&clause_words, &["chosen", "color"])
    {
        let prefix = &clause_words[..chosen_idx];
        let references_mana_of_chosen_color = slice_ends_with(prefix, &["mana", "of", "the"])
            || slice_ends_with(prefix, &["mana", "of"]);
        if references_mana_of_chosen_color {
            let tail_words = &clause_words[chosen_idx + 2..];
            let has_only_pool_tail = tail_words.is_empty()
                || tail_words.iter().all(|word| {
                    matches!(
                        *word,
                        "to" | "your"
                            | "their"
                            | "its"
                            | "that"
                            | "player"
                            | "players"
                            | "mana"
                            | "pool"
                    )
                });
            if has_only_pool_tail {
                let amount = parse_value(tokens)
                    .map(|(value, _)| value)
                    .unwrap_or(Value::Fixed(1));
                return Ok(EffectAst::AddManaChosenColor {
                    amount,
                    player,
                    fixed_option: None,
                });
            }
        }
    }
    if slice_starts_with(
        &clause_words,
        &["an", "amount", "of", "mana", "of", "that", "color"],
    ) {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::AddManaChosenColor {
            amount,
            player,
            fixed_option: None,
        });
    }

    let any_one = find_window_by(&clause_words, 3, |window| {
        window == ["any", "one", "color"] || window == ["any", "one", "type"]
    })
    .is_some();
    let any_color = find_window_by(&clause_words, 2, |window| {
        window == ["any", "color"] || window == ["one", "color"]
    })
    .is_some();
    let any_type = find_window_by(&clause_words, 2, |window| {
        window == ["any", "type"] || window == ["one", "type"]
    })
    .is_some();
    if any_color || any_type {
        let mut amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        let allow_colorless = any_type;
        let phrase_end = tokens
            .iter()
            .enumerate()
            .find_map(|(idx, token)| {
                let word = token.as_word()?;
                if (word == "color" && any_color) || (word == "type" && any_type) {
                    Some(idx + 1)
                } else {
                    None
                }
            })
            .unwrap_or(tokens.len());
        let tail_tokens = trim_leading_commas(&tokens[phrase_end..]);

        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        if let Some(filter) = parse_land_could_produce_filter(tail_tokens)? {
            parser_trace_stack("parse_add_mana:land-could-produce", tokens);
            return Ok(EffectAst::AddManaFromLandCouldProduce {
                amount,
                player,
                land_filter: filter,
                allow_colorless,
                same_type: any_one,
            });
        }

        if matches!(amount, Value::X)
            && let Some(dynamic_amount) = parse_where_x_is_number_of_filter_value(tail_tokens)
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        let tail_words = crate::cards::builders::parser::lexed_words(tail_tokens);
        let chosen_by_player_tail = matches!(
            tail_words.as_slice(),
            ["they", "choose"]
                | ["that", "player", "chooses"]
                | ["they", "choose", "to", "their", "mana", "pool"]
                | ["that", "player", "chooses", "to", "their", "mana", "pool"]
        );
        if chosen_by_player_tail {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }
        if slice_starts_with(&tail_words, &["for", "each"])
            && slice_ends_with(&tail_words, &["removed", "this", "way"])
            && let Some(dynamic_amount) = parse_dynamic_cost_modifier_value(tail_tokens)?
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        if tail_words.first().copied() == Some("among") {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        let base_effect = if any_one {
            EffectAst::AddManaAnyOneColor { amount, player }
        } else {
            EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            }
        };
        if let Some(conditional) = wrap_instead_if_tail(base_effect, tail_tokens)? {
            return Ok(conditional);
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let for_each_idx = find_window_by(tokens, 2, |window: &[OwnedLexToken]| {
        window[0].is_word("for") && window[1].is_word("each")
    });
    let mana_scan_end = for_each_idx.unwrap_or(tokens.len());

    let mut mana = Vec::new();
    let mut last_mana_idx = None;
    for (idx, token) in tokens[..mana_scan_end].iter().enumerate() {
        if let Some(group) = mana_pips_from_token(token) {
            mana.extend(group);
            last_mana_idx = Some(idx);
            continue;
        }
        if let Some(word) = token.as_word() {
            if word.eq_ignore_ascii_case("mana")
                || word.eq_ignore_ascii_case("to")
                || word.eq_ignore_ascii_case("your")
                || word.eq_ignore_ascii_case("pool")
            {
                continue;
            }
        }
    }

    if !mana.is_empty() {
        if let Some(amount) = parse_add_mana_that_much_value(tokens) {
            parser_trace_stack("parse_add_mana:scaled-that-much", tokens);
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(amount) = parse_devotion_value_from_add_clause(tokens)? {
            parser_trace_stack("parse_add_mana:scaled-devotion", tokens);
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(for_each_idx) = for_each_idx {
            let amount_tokens = &tokens[for_each_idx..];
            let amount = parse_dynamic_cost_modifier_value(amount_tokens)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic mana amount (clause: '{}')",
                    crate::cards::builders::parser::lexed_words(tokens).join(" ")
                ))
            })?;
            parser_trace_stack("parse_add_mana:scaled", tokens);
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(amount) = parse_add_mana_equal_amount_value(tokens) {
            parser_trace_stack("parse_add_mana:scaled-equal", tokens);
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        let trailing_words = if let Some(last_idx) = last_mana_idx {
            crate::cards::builders::parser::lexed_words(&tokens[last_idx + 1..])
        } else {
            Vec::new()
        };
        if !trailing_words.is_empty() {
            let chosen_color_tail = slice_starts_with(
                &trailing_words,
                &["or", "one", "mana", "of", "the", "chosen", "color"],
            );
            let pool_tail = if chosen_color_tail {
                trailing_words[7..].to_vec()
            } else {
                Vec::new()
            };
            let has_only_pool_tail = chosen_color_tail
                && (pool_tail.is_empty()
                    || pool_tail
                        .iter()
                        .all(|word| matches!(*word, "to" | "your" | "mana" | "pool")));
            if chosen_color_tail && has_only_pool_tail {
                if mana.len() != 1 {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported chosen-color mana clause with multiple symbols (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                let Some(color) = mana_symbol_to_color(mana[0]) else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported chosen-color mana clause with non-colored symbol (clause: '{}')",
                        clause_words.join(" ")
                    )));
                };
                parser_trace_stack("parse_add_mana:chosen-color-option", tokens);
                return Ok(EffectAst::AddManaChosenColor {
                    amount: Value::Fixed(1),
                    player,
                    fixed_option: Some(color),
                });
            }
        }
        let has_only_pool_tail = !trailing_words.is_empty()
            && trailing_words
                .iter()
                .all(|word| matches!(*word, "to" | "your" | "mana" | "pool"));
        let has_only_instead_tail = trailing_words.as_slice() == ["instead"];
        if !trailing_words.is_empty() && !has_only_pool_tail && !has_only_instead_tail {
            if let Some(last_idx) = last_mana_idx
                && let Some(conditional) = wrap_instead_if_tail(
                    EffectAst::AddMana {
                        mana: mana.clone(),
                        player,
                    },
                    trim_leading_commas(&tokens[last_idx + 1..]),
                )?
            {
                return Ok(conditional);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        parser_trace_stack("parse_add_mana:flat", tokens);
        return Ok(EffectAst::AddMana { mana, player });
    }

    Err(CardTextError::ParseError(format!(
        "missing mana symbols (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn mana_symbol_to_color(symbol: ManaSymbol) -> Option<crate::color::Color> {
    match symbol {
        ManaSymbol::White => Some(crate::color::Color::White),
        ManaSymbol::Blue => Some(crate::color::Color::Blue),
        ManaSymbol::Black => Some(crate::color::Color::Black),
        ManaSymbol::Red => Some(crate::color::Color::Red),
        ManaSymbol::Green => Some(crate::color::Color::Green),
        _ => None,
    }
}

pub(crate) fn parse_or_mana_color_choices(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !slice_contains(&clause_words, &"or") {
        return Ok(None);
    }

    let mut colors = Vec::new();
    let mut has_or = false;
    for token in tokens {
        if token.is_word("or") {
            has_or = true;
            continue;
        }
        if let Some(group) = mana_pips_from_token(token) {
            for symbol in group {
                let Some(color) = mana_symbol_to_color(symbol) else {
                    return Ok(None);
                };
                if !slice_contains(&colors, &color) {
                    colors.push(color);
                }
            }
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(
            word.to_ascii_lowercase().as_str(),
            "to" | "your" | "their" | "its" | "mana" | "pool"
        ) {
            continue;
        }
        return Ok(None);
    }

    if !has_or || colors.len() < 2 {
        return Ok(None);
    }

    Ok(Some(colors))
}

pub(crate) fn parse_any_combination_mana_colors(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let Some(combination_idx) = find_window_index(&clause_words, &["any", "combination", "of"])
    else {
        return Ok(None);
    };

    let mut colors = Vec::new();
    for word in &clause_words[combination_idx + 3..] {
        if *word == "where" {
            break;
        }
        if matches!(
            *word,
            "and" | "or" | "and/or" | "mana" | "to" | "your" | "their" | "its" | "pool"
        ) {
            continue;
        }
        if matches!(*word, "color" | "colors") {
            for color in [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ] {
                if !slice_contains(&colors, &color) {
                    colors.push(color);
                }
            }
            continue;
        }
        let symbol = parse_mana_symbol(word).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported restricted mana symbol '{}' in any-combination clause (clause: '{}')",
                word,
                clause_words.join(" ")
            ))
        })?;
        let color = mana_symbol_to_color(symbol).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported non-colored mana symbol '{}' in any-combination clause (clause: '{}')",
                word,
                clause_words.join(" ")
            ))
        })?;
        if !slice_contains(&colors, &color) {
            colors.push(color);
        }
    }

    if colors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing color options in any-combination mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(colors))
}

pub(crate) fn trim_leading_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let start =
        find_index(tokens, |token: &OwnedLexToken| !token.is_comma()).unwrap_or(tokens.len());
    &tokens[start..]
}

pub(crate) fn is_mana_pool_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::parser::lexed_words(tokens);
    if words.is_empty()
        || words[0] != "to"
        || !slice_contains(&words, &"mana")
        || !slice_contains(&words, &"pool")
    {
        return false;
    }
    words.iter().all(|word| {
        matches!(
            *word,
            "to" | "your" | "their" | "its" | "that" | "player" | "players" | "mana" | "pool"
        )
    })
}

pub(crate) fn parse_land_could_produce_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = crate::cards::builders::parser::lexed_words(tokens);
    if words.len() < 3 || words[0] != "that" {
        return Ok(None);
    }

    let marker_word_idx = if let Some(could_idx) = find_window_index(&words, &["could", "produce"])
    {
        if could_idx + 2 != words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (tail: '{}')",
                words.join(" ")
            )));
        }
        could_idx
    } else if let Some(produced_idx) = find_index(&words, |word| *word == "produced") {
        if produced_idx + 1 != words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (tail: '{}')",
                words.join(" ")
            )));
        }
        produced_idx
    } else {
        return Ok(None);
    };

    let marker_token_idx =
        token_index_for_word_index(tokens, marker_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing mana production marker in tail '{}'",
                words.join(" ")
            ))
        })?;
    let filter_tokens = trim_leading_commas(&tokens[1..marker_token_idx]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing land filter in mana clause (tail: '{}')",
            words.join(" ")
        )));
    }
    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(filter))
}

fn parse_counter_type_from_descriptor_tokens(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    let words = crate::cards::builders::parser::lexed_words(tokens);
    let last = *words.last()?;
    if let Some(counter_type) = parse_counter_type_word(last) {
        return Some(counter_type);
    }
    if last == "strike" && words.len() >= 2 {
        return match words[words.len() - 2] {
            "double" => Some(CounterType::DoubleStrike),
            "first" => Some(CounterType::FirstStrike),
            _ => None,
        };
    }
    if matches!(
        last,
        "a" | "an" | "one" | "two" | "three" | "four" | "five" | "six" | "another"
    ) {
        return None;
    }
    if last.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(CounterType::Named(intern_counter_name(last)));
    }
    None
}

pub(crate) fn parse_remove(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if let Some(from_idx) = find_index(tokens, |token| token.is_word("from")) {
        let tail_words = crate::cards::builders::parser::lexed_words(&tokens[from_idx + 1..]);
        if tail_words == ["combat"] {
            let target_tokens = trim_commas(&tokens[..from_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing remove-from-combat target (clause: '{}')",
                    crate::cards::builders::parser::lexed_words(tokens).join(" ")
                )));
            }
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::RemoveFromCombat { target });
        }
    }

    if tokens.first().is_some_and(|token| token.is_word("all"))
        && let Some(counter_idx) = find_index(tokens, |token: &OwnedLexToken| {
            token.is_word("counter") || token.is_word("counters")
        })
        && counter_idx > 1
    {
        let counter_descriptor = trim_commas(&tokens[1..counter_idx]);
        let counter_type = parse_counter_type_from_descriptor_tokens(&counter_descriptor);
        let mut target_tokens = trim_commas(&tokens[counter_idx + 1..]);
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("from"))
        {
            target_tokens = trim_commas(&target_tokens[1..]);
        }

        let target_words = crate::cards::builders::parser::lexed_words(&target_tokens);
        let source_like_target = matches!(
            target_words.as_slice(),
            ["it"]
                | ["this"]
                | ["this", "creature"]
                | ["this", "artifact"]
                | ["this", "enchantment"]
                | ["this", "permanent"]
                | ["this", "card"]
        );
        if source_like_target {
            let amount = match counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            };
            return Ok(EffectAst::RemoveUpToAnyCounters {
                amount,
                target: TargetAst::Source(span_from_tokens(&target_tokens)),
                counter_type,
                up_to: false,
            });
        }
    }

    let mut idx = 0;
    let mut up_to = false;
    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        up_to = true;
        idx += 2;
    }

    let (amount, used) = parse_value(&tokens[idx..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter removal amount (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        ))
    })?;
    idx += used;

    let counter_idx = find_index(&tokens[idx..], |token: &OwnedLexToken| {
        token.is_word("counter") || token.is_word("counters")
    })
    .map(|offset| idx + offset)
    .ok_or_else(|| CardTextError::ParseError("missing counter keyword".to_string()))?;
    let counter_descriptor = trim_commas(&tokens[idx..counter_idx]);
    let counter_type = parse_counter_type_from_descriptor_tokens(&counter_descriptor);
    if counter_idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing counter keyword".to_string(),
        ));
    }
    idx = counter_idx + 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("from")) {
        idx += 1;
    }

    let target_tokens = trim_commas(&tokens[idx..]);
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("each") || token.is_word("all"))
    {
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(EffectAst::RemoveCountersAll {
            amount,
            filter,
            counter_type,
            up_to,
        });
    }

    let for_each_idx = find_window_by(&target_tokens, 2, |window: &[OwnedLexToken]| {
        window[0].is_word("for") && window[1].is_word("each")
    });
    if let Some(for_each_idx) = for_each_idx {
        let base_target_tokens = trim_commas(&target_tokens[..for_each_idx]);
        let count_filter_tokens = trim_commas(&target_tokens[for_each_idx + 2..]);
        if !base_target_tokens.is_empty() && !count_filter_tokens.is_empty() {
            if let (Ok(target), Ok(count_filter)) = (
                parse_target_phrase(&base_target_tokens),
                parse_object_filter(&count_filter_tokens, false),
            ) {
                return Ok(EffectAst::ForEachObject {
                    filter: count_filter,
                    effects: vec![EffectAst::RemoveUpToAnyCounters {
                        amount,
                        target,
                        counter_type,
                        up_to,
                    }],
                });
            }
        }
    }

    let target_tokens = trim_commas(&tokens[idx..]);
    let target = parse_target_phrase(&target_tokens)?;

    Ok(EffectAst::RemoveUpToAnyCounters {
        amount,
        target,
        counter_type,
        up_to,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedDestroyTimingAst {
    EndOfCombat,
    NextEndStep,
}

pub(crate) fn parse_delayed_destroy_timing_words(
    words: &[&str],
) -> Option<DelayedDestroyTimingAst> {
    if matches!(
        words,
        ["at", "end", "of", "combat"] | ["at", "the", "end", "of", "combat"]
    ) {
        return Some(DelayedDestroyTimingAst::EndOfCombat);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "end", "step"]
            | ["at", "beginning", "of", "the", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "the", "next", "end", "step"]
    ) {
        return Some(DelayedDestroyTimingAst::NextEndStep);
    }

    None
}

pub(crate) fn wrap_destroy_with_delayed_timing(
    effect: EffectAst,
    timing: Option<DelayedDestroyTimingAst>,
) -> EffectAst {
    let Some(timing) = timing else {
        return effect;
    };

    match timing {
        DelayedDestroyTimingAst::EndOfCombat => EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        },
        DelayedDestroyTimingAst::NextEndStep => EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![effect],
        },
    }
}

pub(crate) fn parse_destroy(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let original_clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let mut delayed_timing = None;
    let mut timing_cut_word_idx = original_clause_words.len();
    for word_idx in 0..original_clause_words.len() {
        if original_clause_words[word_idx] != "at" {
            continue;
        }
        if let Some(timing) = parse_delayed_destroy_timing_words(&original_clause_words[word_idx..])
        {
            delayed_timing = Some(timing);
            timing_cut_word_idx = word_idx;
            break;
        }
    }

    let core_tokens = if timing_cut_word_idx < original_clause_words.len() {
        let token_cutoff =
            token_index_for_word_index(tokens, timing_cut_word_idx).unwrap_or(tokens.len());
        trim_commas(&tokens[..token_cutoff])
    } else {
        trim_commas(tokens)
    };
    let clause_words = crate::cards::builders::parser::lexed_words(&core_tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing destroy target before delayed timing clause (clause: '{}')",
            original_clause_words.join(" ")
        )));
    }

    if delayed_timing.is_none()
        && (find_window_index(&original_clause_words, &["end", "of", "combat"]).is_some()
            || (slice_contains(&original_clause_words, &"beginning")
                && slice_contains(&original_clause_words, &"end")))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed destroy timing clause (clause: '{}')",
            original_clause_words.join(" ")
        )));
    }
    if let Some(target) = parse_destroy_combat_history_target(&core_tokens)? {
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::Destroy { target },
            delayed_timing,
        ));
    }
    let has_combat_history = (slice_contains(&clause_words, &"dealt")
        && slice_contains(&clause_words, &"damage")
        && slice_contains(&clause_words, &"turn"))
        || find_window_by(&clause_words, 2, |window| {
            matches!(window, ["was", "blocked"] | ["was", "blocking"])
        })
        .is_some()
        || find_window_by(&clause_words, 2, |window| {
            matches!(
                window,
                ["blocking", "it"] | ["blocked", "it"] | ["it", "blocked"]
            )
        })
        .is_some();
    if has_combat_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history destroy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
        if let Some(attached_idx) = find_index(&core_tokens, |token: &OwnedLexToken| {
            token.is_word("attached")
        }) && core_tokens
            .get(attached_idx + 1)
            .is_some_and(|token| token.is_word("to"))
            && attached_idx > 1
        {
            let mut filter_tokens = trim_commas(&core_tokens[1..attached_idx]).to_vec();
            while filter_tokens
                .last()
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| matches!(word, "that" | "were" | "was" | "is" | "are"))
            {
                filter_tokens.pop();
            }
            let target_tokens = trim_commas(&core_tokens[attached_idx + 2..]);
            let target_words = crate::cards::builders::parser::lexed_words(&target_tokens);
            let has_timing_tail = target_words.iter().any(|word| {
                matches!(
                    *word,
                    "at" | "beginning" | "end" | "combat" | "turn" | "step" | "until"
                )
            });
            let supported_target = slice_starts_with(&target_words, &["target"])
                || target_words == ["it"]
                || slice_starts_with(&target_words, &["that", "creature"])
                || slice_starts_with(&target_words, &["that", "permanent"])
                || slice_starts_with(&target_words, &["that", "land"])
                || slice_starts_with(&target_words, &["that", "artifact"])
                || slice_starts_with(&target_words, &["that", "enchantment"]);
            if !filter_tokens.is_empty()
                && !target_tokens.is_empty()
                && supported_target
                && !has_timing_tail
            {
                let filter = parse_object_filter(&filter_tokens, false)?;
                let target = parse_target_phrase(&target_tokens)?;
                return Ok(wrap_destroy_with_delayed_timing(
                    EffectAst::DestroyAllAttachedTo { filter, target },
                    delayed_timing,
                ));
            }
        }
        if let Some(except_for_idx) =
            find_window_by(&core_tokens, 2, |window: &[OwnedLexToken]| {
                window[0].is_word("except") && window[1].is_word("for")
            })
            && except_for_idx > 1
        {
            let base_filter_tokens = trim_commas(&core_tokens[1..except_for_idx]);
            let exception_tokens = trim_commas(&core_tokens[except_for_idx + 2..]);
            if !base_filter_tokens.is_empty() && !exception_tokens.is_empty() {
                let mut filter = parse_object_filter(&base_filter_tokens, false)?;
                let exception_filter = parse_object_filter(&exception_tokens, false)?;
                apply_except_filter_exclusions(&mut filter, &exception_filter);
                return Ok(wrap_destroy_with_delayed_timing(
                    EffectAst::DestroyAll { filter },
                    delayed_timing,
                ));
            }
        }
        let filter_tokens = &core_tokens[1..];
        if let Some((choice_idx, consumed)) = find_color_choice_phrase(filter_tokens) {
            let base_filter_tokens = trim_commas(&filter_tokens[..choice_idx]);
            let trailing = trim_commas(&filter_tokens[choice_idx + consumed..]);
            if !trailing.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing color-choice destroy-all clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing destroy-all filter before color-choice clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let filter = parse_object_filter(&base_filter_tokens, false)?;
            return Ok(wrap_destroy_with_delayed_timing(
                EffectAst::DestroyAllOfChosenColor { filter },
                delayed_timing,
            ));
        }
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::DestroyAll { filter },
            delayed_timing,
        ));
    }

    if slice_contains(&clause_words, &"unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported destroy-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(if_idx) = find_index(&core_tokens, |token: &OwnedLexToken| token.is_word("if")) {
        let mut target_tokens = trim_commas(&core_tokens[..if_idx]).to_vec();
        while target_tokens
            .last()
            .is_some_and(|token| token.is_word("instead"))
        {
            target_tokens.pop();
        }
        let target_tokens = trim_commas(&target_tokens);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported conditional destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let mut predicate_tokens = trim_commas(&core_tokens[if_idx + 1..]).to_vec();
        while predicate_tokens
            .last()
            .is_some_and(|token| token.is_word("instead"))
        {
            predicate_tokens.pop();
        }
        let predicate_tokens = trim_commas(&predicate_tokens);
        if predicate_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported conditional destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let target = parse_target_phrase(&target_tokens)?;

        if let Some(instead_if_idx) =
            find_window_by(&predicate_tokens, 2, |window: &[OwnedLexToken]| {
                window[0].is_word("instead") && window[1].is_word("if")
            })
        {
            let base_predicate_tokens = trim_commas(&predicate_tokens[..instead_if_idx]);
            let outer_predicate_tokens = trim_commas(&predicate_tokens[instead_if_idx + 2..]);
            if base_predicate_tokens.is_empty() || outer_predicate_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported conditional destroy clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let base_predicate = parse_predicate(&base_predicate_tokens).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported conditional destroy clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            let outer_predicate = parse_predicate(&outer_predicate_tokens).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported conditional destroy clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;

            return Ok(wrap_destroy_with_delayed_timing(
                EffectAst::Conditional {
                    predicate: outer_predicate,
                    if_true: vec![EffectAst::Conditional {
                        predicate: base_predicate,
                        if_true: vec![EffectAst::Destroy {
                            target: target.clone(),
                        }],
                        if_false: Vec::new(),
                    }],
                    if_false: Vec::new(),
                },
                delayed_timing,
            ));
        }

        let predicate = parse_predicate(&predicate_tokens).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported conditional destroy clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::Conditional {
                predicate,
                if_true: vec![EffectAst::Destroy { target }],
                if_false: Vec::new(),
            },
            delayed_timing,
        ));
    }
    if let Some(and_idx) = find_index(&core_tokens, |token: &OwnedLexToken| token.is_word("and")) {
        let tail_words = crate::cards::builders::parser::lexed_words(&core_tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (slice_starts_with(&tail_words, &["up", "to"])
                && slice_contains(&tail_words, &"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if slice_starts_with(&clause_words, &["target", "blocked"]) {
        let mut target_tokens = core_tokens.to_vec();
        if let Some(blocked_idx) = find_index(&target_tokens, |token: &OwnedLexToken| {
            token.is_word("blocked")
        }) {
            target_tokens.remove(blocked_idx);
        }
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::Conditional {
                predicate: PredicateAst::TargetIsBlocked,
                if_true: vec![EffectAst::Destroy { target }],
                if_false: Vec::new(),
            },
            delayed_timing,
        ));
    }

    let target = parse_target_phrase(&core_tokens)?;
    Ok(wrap_destroy_with_delayed_timing(
        EffectAst::Destroy { target },
        delayed_timing,
    ))
}

pub(crate) fn parse_destroy_combat_history_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    let Some(that_idx) = find_window_index(
        &clause_words,
        &["that", "was", "dealt", "damage", "this", "turn"],
    ) else {
        return Ok(None);
    };
    if that_idx == 0 || that_idx + 6 != clause_words.len() {
        return Ok(None);
    }
    let target_cutoff = token_index_for_word_index(tokens, that_idx).unwrap_or(tokens.len());
    let target_tokens = trim_commas(&tokens[..target_cutoff]);
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let target = parse_target_phrase(&target_tokens)?;
    let TargetAst::Object(mut filter, target_span, it_span) = target else {
        return Ok(None);
    };
    filter.was_dealt_damage_this_turn = true;
    Ok(Some(TargetAst::Object(filter, target_span, it_span)))
}

pub(crate) fn apply_except_filter_exclusions(base: &mut ObjectFilter, exception: &ObjectFilter) {
    for card_type in exception
        .card_types
        .iter()
        .copied()
        .chain(exception.all_card_types.iter().copied())
    {
        if !slice_contains(&base.excluded_card_types, &card_type) {
            base.excluded_card_types.push(card_type);
        }
    }
    for subtype in exception.subtypes.iter().copied() {
        if !slice_contains(&base.excluded_subtypes, &subtype) {
            base.excluded_subtypes.push(subtype);
        }
    }
}

pub(crate) fn parse_exile(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let (tokens, face_down) = split_exile_face_down_suffix(tokens);
    let tokens = split_exile_graveyard_replacement_suffix(tokens);
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if slice_contains(&clause_words, &"unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported exile-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_face_down_manifest_tail = (slice_contains(&clause_words, &"face-down")
        || slice_contains(&clause_words, &"facedown")
        || slice_contains(&clause_words, &"manifest")
        || slice_contains(&clause_words, &"pile"))
        && slice_contains(&clause_words, &"then");
    if has_face_down_manifest_tail {
        return Err(CardTextError::ParseError(format!(
            "unsupported face-down/manifest exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_same_name_exile_hand_and_graveyard_clause(
        tokens,
        subject,
        until_source_leaves,
        face_down,
    )? {
        return Ok(effect);
    }
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
        let filter_tokens = &tokens[1..];
        let mut filter = parse_object_filter(filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        return Ok(if until_source_leaves {
            EffectAst::ExileUntilSourceLeaves {
                target: TargetAst::Object(filter, None, None),
                face_down,
            }
        } else {
            EffectAst::ExileAll { filter, face_down }
        });
    }
    if let Some(filter) = parse_target_player_graveyard_filter(tokens) {
        return Ok(if until_source_leaves {
            EffectAst::ExileUntilSourceLeaves {
                target: TargetAst::Object(filter, None, None),
                face_down,
            }
        } else {
            EffectAst::ExileAll { filter, face_down }
        });
    }

    if slice_contains(&clause_words, &"dealt")
        && slice_contains(&clause_words, &"damage")
        && slice_contains(&clause_words, &"turn")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_until_total_mana_value = slice_contains(&clause_words, &"until")
        && slice_contains(&clause_words, &"exiled")
        && slice_contains(&clause_words, &"total")
        && slice_contains(&clause_words, &"mana")
        && slice_contains(&clause_words, &"value");
    if has_until_total_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported iterative exile-total clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_attached_bundle = slice_contains(&clause_words, &"and")
        && slice_contains(&clause_words, &"all")
        && slice_contains(&clause_words, &"attached");
    if has_attached_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported attached-object exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_same_name_token_bundle = slice_contains(&clause_words, &"and")
        && slice_contains(&clause_words, &"tokens")
        && slice_contains(&clause_words, &"same")
        && slice_contains(&clause_words, &"name");
    if has_same_name_token_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported same-name token exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(and_idx) = find_index(tokens, |token| token.is_word("and"))
        && and_idx > 0
    {
        let tail_words = crate::cards::builders::parser::lexed_words(&tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (slice_starts_with(&tail_words, &["up", "to"])
                && slice_contains(&tail_words, &"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target exile clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if let Some(if_idx) = find_index(tokens, |token| token.is_word("if"))
        && if_idx > 0
    {
        let target_tokens = trim_commas(&tokens[..if_idx]);
        let predicate_tokens = trim_commas(&tokens[if_idx + 1..]);
        if target_tokens.is_empty() || predicate_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported conditional exile clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut target = parse_target_phrase(&target_tokens)?;
        apply_exile_subject_hand_owner_context(&mut target, subject);
        let predicate = parse_predicate(&predicate_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![if until_source_leaves {
                EffectAst::ExileUntilSourceLeaves { target, face_down }
            } else {
                EffectAst::Exile { target, face_down }
            }],
            if_false: Vec::new(),
        });
    }

    let mut target = parse_target_phrase(tokens)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    Ok(if until_source_leaves {
        EffectAst::ExileUntilSourceLeaves { target, face_down }
    } else {
        EffectAst::Exile { target, face_down }
    })
}

pub(crate) fn parse_same_name_exile_hand_and_graveyard_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if !(slice_starts_with(&clause_words, &["all", "cards"])
        || slice_starts_with(&clause_words, &["all", "card"]))
        || find_window_index(&clause_words, &["with", "that", "name"]).is_none()
    {
        return Ok(None);
    }

    let Some(from_idx) = find_index(&clause_words, |word| *word == "from") else {
        return Ok(None);
    };
    let Some(first_zone_idx) = find_index(&clause_words[from_idx + 1..], |word| {
        matches!(*word, "hand" | "hands" | "graveyard" | "graveyards")
    })
    .map(|offset| from_idx + 1 + offset) else {
        return Ok(None);
    };

    let owner_words = &clause_words[from_idx + 1..first_zone_idx];
    let owner_from_subject = match subject {
        Some(SubjectAst::Player(player)) => controller_filter_for_token_player(player),
        _ => None,
    };
    let owner = match owner_words {
        ["target", "player"] | ["target", "players"] => Some(PlayerFilter::target_player()),
        ["target", "opponent"] | ["target", "opponents"] => Some(PlayerFilter::target_opponent()),
        ["that", "player"] | ["that", "players"] => Some(PlayerFilter::IteratedPlayer),
        ["your"] => Some(PlayerFilter::You),
        ["their"] | ["his", "or", "her"] => {
            owner_from_subject.or(Some(PlayerFilter::IteratedPlayer))
        }
        [] => owner_from_subject,
        _ => return Ok(None),
    };
    let Some(owner) = owner else {
        return Ok(None);
    };

    let mut zones = Vec::new();
    for word in &clause_words[first_zone_idx..] {
        let Some(zone) = parse_zone_word(word) else {
            continue;
        };
        if !matches!(zone, Zone::Hand | Zone::Graveyard) || slice_contains(&zones, &zone) {
            continue;
        }
        zones.push(zone);
    }
    if zones.len() != 2
        || !slice_contains(&zones, &Zone::Hand)
        || !slice_contains(&zones, &Zone::Graveyard)
    {
        return Ok(None);
    }

    let mut filter = ObjectFilter::default();
    filter.owner = Some(owner);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.any_of = zones
        .into_iter()
        .map(|zone| ObjectFilter::default().in_zone(zone))
        .collect();

    Ok(Some(if until_source_leaves {
        EffectAst::ExileUntilSourceLeaves {
            target: TargetAst::Object(filter, None, None),
            face_down,
        }
    } else {
        EffectAst::ExileAll { filter, face_down }
    }))
}

pub(crate) fn split_until_source_leaves_tail(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    let Some(until_idx) = rfind_index(tokens, |token| token.is_word("until")) else {
        return (tokens, false);
    };
    if until_idx == 0 {
        return (tokens, false);
    }
    let tail_words = crate::cards::builders::parser::lexed_words(&tokens[until_idx + 1..]);
    let has_source_leaves_tail = tail_words.len() >= 3
        && tail_words[tail_words.len() - 3] == "leaves"
        && tail_words[tail_words.len() - 2] == "the"
        && tail_words[tail_words.len() - 1] == "battlefield";
    if has_source_leaves_tail {
        (&tokens[..until_idx], true)
    } else {
        (tokens, false)
    }
}

pub(crate) fn split_exile_face_down_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    if tokens.is_empty() {
        return (tokens, false);
    }

    let mut end = tokens.len();
    if end > 0 && tokens[end - 1].is_word("instead") {
        end -= 1;
    }

    if end > 0 && (tokens[end - 1].is_word("face-down") || tokens[end - 1].is_word("facedown")) {
        return (&tokens[..end - 1], true);
    }

    if end >= 2 && tokens[end - 2].is_word("face") && tokens[end - 1].is_word("down") {
        return (&tokens[..end - 2], true);
    }

    (tokens, false)
}

pub(crate) fn split_exile_graveyard_replacement_suffix(
    tokens: &[OwnedLexToken],
) -> &[OwnedLexToken] {
    let Some(instead_idx) = find_index(tokens, |token| token.is_word("instead")) else {
        return tokens;
    };
    if instead_idx == 0 {
        return tokens;
    }

    let tail_words = crate::cards::builders::parser::lexed_words(&tokens[instead_idx..]);
    let is_graveyard_replacement = slice_starts_with(&tail_words, &["instead", "of", "putting"])
        && tail_words
            .iter()
            .any(|word| *word == "graveyard" || *word == "graveyards");
    if is_graveyard_replacement {
        &tokens[..instead_idx]
    } else {
        tokens
    }
}

pub(crate) fn parse_graveyard_owner_prefix(words: &[&str]) -> Option<(PlayerAst, usize)> {
    if slice_starts_with(&words, &["your", "graveyard"]) {
        return Some((PlayerAst::You, 2));
    }
    if slice_starts_with(&words, &["their", "graveyard"]) {
        return Some((PlayerAst::That, 2));
    }
    if slice_starts_with(&words, &["that", "player", "graveyard"])
        || slice_starts_with(&words, &["that", "players", "graveyard"])
    {
        return Some((PlayerAst::That, 3));
    }
    if slice_starts_with(&words, &["target", "player", "graveyard"])
        || slice_starts_with(&words, &["target", "players", "graveyard"])
    {
        return Some((PlayerAst::Target, 3));
    }
    if slice_starts_with(&words, &["target", "opponent", "graveyard"])
        || slice_starts_with(&words, &["target", "opponents", "graveyard"])
    {
        return Some((PlayerAst::TargetOpponent, 3));
    }
    if slice_starts_with(&words, &["its", "controller", "graveyard"])
        || slice_starts_with(&words, &["its", "controllers", "graveyard"])
    {
        return Some((PlayerAst::ItsController, 3));
    }
    if slice_starts_with(&words, &["its", "owner", "graveyard"])
        || slice_starts_with(&words, &["its", "owners", "graveyard"])
    {
        return Some((PlayerAst::ItsOwner, 3));
    }
    if slice_starts_with(&words, &["his", "or", "her", "graveyard"]) {
        return Some((PlayerAst::That, 4));
    }
    None
}

pub(crate) fn parse_target_player_graveyard_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let words = crate::cards::builders::parser::lexed_words(tokens);
    let (player, consumed) = parse_graveyard_owner_prefix(&words)?;
    if consumed != words.len() {
        return None;
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = match player {
        PlayerAst::You => Some(PlayerFilter::You),
        PlayerAst::That | PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent))),
        PlayerAst::ItsController => Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged("triggering"),
        )),
        PlayerAst::ItsOwner => Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
            "triggering",
        ))),
        _ => None,
    };
    filter.owner.as_ref()?;
    Some(filter)
}

pub(crate) fn apply_exile_subject_hand_owner_context(
    target: &mut TargetAst,
    subject: Option<SubjectAst>,
) {
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Hand) {
        return;
    }
    apply_exile_subject_owner_context(filter, subject);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::parser::util::tokenize_line;

    #[test]
    fn parse_graveyard_owner_prefix_handles_shared_phrases() {
        assert_eq!(
            parse_graveyard_owner_prefix(&["that", "player", "graveyard", "as", "you", "choose"]),
            Some((PlayerAst::That, 3))
        );
        assert_eq!(
            parse_graveyard_owner_prefix(&["its", "owner", "graveyard"]),
            Some((PlayerAst::ItsOwner, 3))
        );
    }

    #[test]
    fn parse_target_player_graveyard_filter_uses_shared_owner_prefix() {
        let tokens = tokenize_line("target opponent graveyard", 0);
        let filter = parse_target_player_graveyard_filter(&tokens).expect("target graveyard");

        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(matches!(
            filter.owner,
            Some(PlayerFilter::Target(ref target)) if **target == PlayerFilter::Opponent
        ));
    }
}

pub(crate) fn apply_exile_subject_owner_context(
    filter: &mut ObjectFilter,
    subject: Option<SubjectAst>,
) {
    let Some(owner_filter) = exile_subject_owner_filter(subject) else {
        return;
    };
    let direct_zone_ok = matches!(
        filter.zone,
        Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
    );
    let any_of_zone_ok = filter.any_of.iter().any(|nested| {
        matches!(
            nested.zone,
            Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
        )
    });
    if !direct_zone_ok && !any_of_zone_ok {
        return;
    }
    match filter.owner {
        Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) | None => {
            filter.owner = Some(owner_filter);
        }
        _ => {}
    }
}

pub(crate) fn apply_shuffle_subject_graveyard_owner_context(
    target: &mut TargetAst,
    subject: SubjectAst,
) {
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Graveyard) {
        return;
    }

    let owner_filter = match subject {
        SubjectAst::Player(PlayerAst::Target) => Some(PlayerFilter::target_player()),
        SubjectAst::Player(PlayerAst::TargetOpponent) => Some(PlayerFilter::target_opponent()),
        SubjectAst::Player(PlayerAst::You) => Some(PlayerFilter::You),
        _ => None,
    };
    let Some(owner_filter) = owner_filter else {
        return;
    };

    match filter.owner {
        Some(PlayerFilter::IteratedPlayer) | Some(PlayerFilter::Target(_)) | None => {
            filter.owner = Some(owner_filter);
        }
        _ => {}
    }
}

pub(crate) fn exile_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject {
        Some(SubjectAst::Player(PlayerAst::Target)) => Some(PlayerFilter::target_player()),
        Some(SubjectAst::Player(PlayerAst::TargetOpponent)) => {
            Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        }
        Some(SubjectAst::Player(PlayerAst::That)) => Some(PlayerFilter::IteratedPlayer),
        Some(SubjectAst::Player(PlayerAst::You)) => Some(PlayerFilter::You),
        _ => None,
    }
}

pub(crate) fn discard_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject {
        Some(SubjectAst::Player(PlayerAst::Target)) => Some(PlayerFilter::target_player()),
        Some(SubjectAst::Player(PlayerAst::TargetOpponent)) => {
            Some(PlayerFilter::target_opponent())
        }
        Some(SubjectAst::Player(PlayerAst::That)) => Some(PlayerFilter::IteratedPlayer),
        Some(SubjectAst::Player(PlayerAst::You)) => Some(PlayerFilter::You),
        _ => None,
    }
}

pub(crate) fn target_object_filter_mut(target: &mut TargetAst) -> Option<&mut ObjectFilter> {
    match target {
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::WithCount(inner, _) => target_object_filter_mut(inner),
        _ => None,
    }
}

pub(crate) fn merge_it_match_filter_into_target(
    target: &mut TargetAst,
    it_filter: &ObjectFilter,
) -> bool {
    if let TargetAst::Tagged(tag, span) = target {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        *target = TargetAst::Object(filter, span.clone(), None);
    }

    let Some(filter) = target_object_filter_mut(target) else {
        return false;
    };
    if !it_filter.card_types.is_empty() {
        filter.card_types = it_filter.card_types.clone();
    }
    if !it_filter.subtypes.is_empty() {
        filter.subtypes = it_filter.subtypes.clone();
    }
    if let Some(power) = &it_filter.power {
        filter.power = Some(power.clone());
        filter.power_reference = it_filter.power_reference;
    }
    if let Some(toughness) = &it_filter.toughness {
        filter.toughness = Some(toughness.clone());
        filter.toughness_reference = it_filter.toughness_reference;
    }
    if let Some(mana_value) = &it_filter.mana_value {
        filter.mana_value = Some(mana_value.clone());
    }
    true
}

pub(crate) fn parse_untap(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "untap clause missing target".to_string(),
        ));
    }
    let words = crate::cards::builders::parser::lexed_words(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::UntapAll { filter });
    }
    if words.as_slice() == ["them"] {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(EffectAst::UntapAll { filter });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Untap { target })
}

pub(crate) fn parse_scry(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing scry count (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(EffectAst::Scry { count, player })
}

pub(crate) fn parse_surveil(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing surveil count (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(EffectAst::Surveil { count, player })
}

pub(crate) fn parse_pay(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let energy_symbol_count = tokens
        .iter()
        .filter(|token| {
            token.is_word("e")
                || (token.kind == TokenKind::ManaGroup
                    && token
                        .slice
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .eq_ignore_ascii_case("e"))
        })
        .count();

    let clause_words = crate::cards::builders::parser::lexed_words(tokens);
    if slice_starts_with(&clause_words, &["any", "amount", "of"])
        && (slice_contains(&clause_words, &"e") || energy_symbol_count > 0)
    {
        return Ok(EffectAst::PayEnergy {
            amount: Value::Fixed(0),
            player,
        });
    }
    if clause_words.len() >= 4
        && slice_contains(&clause_words, &"for")
        && slice_contains(&clause_words, &"each")
        && let Ok(symbols) = parse_mana_symbol_group(clause_words[0])
    {
        return Ok(EffectAst::PayMana {
            cost: ManaCost::from_pips(vec![symbols]),
            player,
        });
    }

    if let Some((amount, used)) = parse_value(tokens)
        && tokens.get(used).is_some_and(|token| token.is_word("life"))
    {
        return Ok(EffectAst::LoseLife { amount, player });
    }
    if let Some((amount, used)) = parse_value(tokens)
        && tokens
            .get(used)
            .is_some_and(|token| token.is_word("energy"))
    {
        return Ok(EffectAst::PayEnergy { amount, player });
    }
    if energy_symbol_count > 0 {
        let mut energy_count = 0u32;
        for token in tokens {
            if token.kind == TokenKind::ManaGroup
                && token
                    .slice
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .eq_ignore_ascii_case("e")
            {
                energy_count += 1;
                continue;
            }
            let Some(word) = token.as_word() else {
                continue;
            };
            if is_article(word)
                || word == "and"
                || word == "or"
                || word == "energy"
                || word == "counter"
                || word == "counters"
            {
                continue;
            }
            if word == "e" {
                energy_count += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported pay clause token '{word}' (clause: '{}')",
                crate::cards::builders::parser::lexed_words(tokens).join(" ")
            )));
        }
        if energy_count > 0 {
            return Ok(EffectAst::PayEnergy {
                amount: Value::Fixed(energy_count as i32),
                player,
            });
        }
    }

    let mut pips = Vec::new();
    for token in tokens {
        if let Some(group) = mana_pips_from_token(token) {
            pips.push(group);
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if is_article(word) || word == "mana" {
            continue;
        }
        if let Ok(symbols) = parse_mana_symbol_group(&word) {
            pips.push(symbols);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported pay clause token '{word}' (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        )));
    }

    if pips.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing payment cost (clause: '{}')",
            crate::cards::builders::parser::lexed_words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::PayMana {
        cost: ManaCost::from_pips(pips),
        player,
    })
}
