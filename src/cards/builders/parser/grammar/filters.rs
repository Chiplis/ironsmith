use super::super::activation_and_restrictions::parse_named_number;
use super::super::effect_sentences::{conditionals::parse_subtype_word, parse_supertype_word};
use super::super::keyword_static::parse_pt_modifier;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{
    apply_parity_filter_phrases, lower_words_contains, lower_words_find_index,
    lower_words_find_sequence, lower_words_find_window, lower_words_has_window,
    lower_words_starts_with, normalized_token_index_after_words,
    normalized_token_index_for_word_index, parse_attached_reference_or_another_disjunction,
    parse_object_filter_lexed, push_unique, set_has, slice_has, strip_not_on_battlefield_phrase,
    token_find_index, token_find_window, trim_vote_winner_suffix,
};
use super::super::token_primitives::{
    contains_window, find_index, find_window_index, rfind_index, slice_contains, slice_ends_with,
    slice_starts_with, slice_strip_prefix,
};
use super::super::util::{
    apply_filter_keyword_constraint, is_article, is_demonstrative_object_head, is_non_outlaw_word,
    is_outlaw_word, is_permanent_type, is_source_reference_words, parse_alternative_cast_words,
    parse_card_type, parse_color, parse_counter_type_word, parse_filter_counter_constraint_words,
    parse_filter_keyword_constraint_words, parse_mana_symbol_word_flexible, parse_non_color,
    parse_non_subtype, parse_non_supertype, parse_non_type, parse_number, parse_subtype_flexible,
    parse_unsigned_pt_word, parse_zone_word, push_outlaw_subtypes, trim_commas,
};
use super::super::value_helpers::parse_filter_comparison_tokens;
use super::primitives::{TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_or};
use super::values::parse_mana_symbol;
use crate::cards::TextSpan;
use crate::cards::builders::{CardTextError, IT_TAG, PlayerAst, PredicateAst, TagKey};
use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::effects::VOTE_WINNERS_TAG;
use crate::filter::TaggedObjectConstraint;
use crate::mana::ManaSymbol;
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::{CardType, Supertype};
use crate::zone::Zone;

type GrammarFilterNormalizedWords = TokenWordView;

fn synth_words_as_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect()
}

fn push_unique_filter_value<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    if !items.iter().any(|item| *item == value) {
        items.push(value);
    }
}

fn parse_spell_filter_parity_word(word: &str) -> Option<crate::filter::ParityRequirement> {
    match word {
        "odd" => Some(crate::filter::ParityRequirement::Odd),
        "even" => Some(crate::filter::ParityRequirement::Even),
        _ => None,
    }
}

fn apply_spell_filter_parity_phrases(words: &[&str], filter: &mut ObjectFilter) {
    let mut idx = 0usize;
    while idx + 2 < words.len() {
        let window = &words[idx..idx + 3];
        if let Some(parity) = parse_spell_filter_parity_word(window[0]) {
            match &window[1..] {
                ["mana", "value"] | ["mana", "values"] => filter.mana_value_parity = Some(parity),
                ["power", _] => {}
                _ => {}
            }
        }
        idx += 1;
    }

    let mut idx = 0usize;
    while idx + 1 < words.len() {
        let window = &words[idx..idx + 2];
        if let Some(parity) = parse_spell_filter_parity_word(window[0])
            && window[1] == "power"
        {
            filter.power_parity = Some(parity);
        }
        idx += 1;
    }
}

pub(crate) fn parse_object_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_lexed(tokens, other)
}

fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let (tokens, vote_winners_only) = trim_vote_winner_suffix(tokens);
    let mut filter = ObjectFilter::default();
    if other {
        filter.other = true;
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut base_tokens: Vec<OwnedLexToken> = tokens.to_vec();
    let mut targets_idx: Option<usize> = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_word("targets") || token.is_word("target") {
            if idx > 0 && tokens[idx - 1].is_word("that") {
                targets_idx = Some(idx);
                break;
            }
        }
    }
    if let Some(targets_idx) = targets_idx {
        let that_idx = targets_idx - 1;
        base_tokens = tokens[..that_idx].to_vec();
        let target_tokens = &tokens[targets_idx + 1..];
        let parse_target_fragment = |fragment_tokens: &[OwnedLexToken]| -> Result<
            (Option<PlayerFilter>, Option<ObjectFilter>),
            CardTextError,
        > {
            let target_words_view = GrammarFilterNormalizedWords::new(fragment_tokens);
            let target_words = target_words_view.to_word_refs();
            if lower_words_starts_with(&target_words, &["you"]) {
                return Ok((Some(PlayerFilter::You), None));
            }
            if lower_words_starts_with(&target_words, &["opponent"])
                || lower_words_starts_with(&target_words, &["opponents"])
            {
                return Ok((Some(PlayerFilter::Opponent), None));
            }
            if lower_words_starts_with(&target_words, &["player"])
                || lower_words_starts_with(&target_words, &["players"])
            {
                return Ok((Some(PlayerFilter::Any), None));
            }

            let mut target_filter_tokens = fragment_tokens;
            if target_filter_tokens
                .first()
                .is_some_and(|token| token.is_word("target"))
            {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if target_filter_tokens.is_empty() {
                return Ok((None, None));
            }
            Ok((
                None,
                Some(parse_object_filter(target_filter_tokens, false)?),
            ))
        };

        let target_words_view = GrammarFilterNormalizedWords::new(target_tokens);
        let target_words = target_words_view.to_word_refs();
        if let Some(or_word_idx) = lower_words_find_index(&target_words, |word| word == "or")
            && let Some(or_token_idx) =
                normalized_token_index_for_word_index(target_tokens, or_word_idx)
        {
            let left_tokens = trim_commas(&target_tokens[..or_token_idx]);
            let right_tokens = trim_commas(&target_tokens[or_token_idx + 1..]);
            let (left_player, left_object) = parse_target_fragment(&left_tokens)?;
            let (right_player, right_object) = parse_target_fragment(&right_tokens)?;
            target_player = left_player.or(right_player);
            target_object = left_object.or(right_object);
            if target_player.is_some() && target_object.is_some() {
                filter.targets_any_of = true;
            }
        } else {
            let (parsed_player, parsed_object) = parse_target_fragment(target_tokens)?;
            target_player = parsed_player;
            target_object = parsed_object;
        }
    }

    // Object filters should not absorb trailing duration clauses such as
    // "... until this enchantment leaves the battlefield".
    if let Some(until_token_idx) = token_find_index(&base_tokens, |token| token.is_word("until"))
        && until_token_idx > 0
    {
        base_tokens.truncate(until_token_idx);
    }

    let not_on_battlefield = strip_not_on_battlefield_phrase(&mut base_tokens);

    // "other than this/it/them ..." marks an exclusion, not an additional
    // type selector. Keep "other" but drop the self-reference tail.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if !(base_tokens[idx].is_word("other") && base_tokens[idx + 1].is_word("than")) {
            idx += 1;
            continue;
        }

        let mut end = idx + 2;
        let starts_with_self_reference = base_tokens[end].is_word("this")
            || base_tokens[end].is_word("it")
            || base_tokens[end].is_word("them");
        if !starts_with_self_reference {
            idx += 1;
            continue;
        }
        end += 1;

        if end < base_tokens.len()
            && base_tokens[end].as_word().is_some_and(|word| {
                matches!(
                    word,
                    "artifact"
                        | "artifacts"
                        | "battle"
                        | "battles"
                        | "card"
                        | "cards"
                        | "creature"
                        | "creatures"
                        | "enchantment"
                        | "enchantments"
                        | "land"
                        | "lands"
                        | "permanent"
                        | "permanents"
                        | "planeswalker"
                        | "planeswalkers"
                        | "spell"
                        | "spells"
                        | "token"
                        | "tokens"
                )
            })
        {
            end += 1;
        }

        base_tokens.drain(idx + 1..end);
    }
    if let Some(mut disjunction) = parse_attached_reference_or_another_disjunction(&base_tokens)? {
        if target_player.is_some() || target_object.is_some() {
            disjunction = disjunction.targeting(target_player.take(), target_object.take());
        }
        return Ok(disjunction);
    }
    let mut segment_tokens = base_tokens.clone();

    let all_words_view = GrammarFilterNormalizedWords::new(&base_tokens);
    let all_words_with_articles: Vec<&str> = all_words_view
        .to_word_refs()
        .into_iter()
        .filter(|word| *word != "instead")
        .collect();

    let map_non_article_index = |non_article_idx: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_idx {
                return Some(idx);
            }
            seen += 1;
        }
        None
    };

    let map_non_article_end = |non_article_end: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_end {
                return Some(idx);
            }
            seen += 1;
        }
        if seen == non_article_end {
            return Some(all_words_with_articles.len());
        }
        None
    };

    let mut all_words: Vec<&str> = all_words_with_articles
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();

    // "that were put there from the battlefield this turn" means the card entered
    // a graveyard from the battlefield this turn.
    for phrase in [
        [
            "that",
            "was",
            "put",
            "there",
            "from",
            "battlefield",
            "this",
            "turn",
        ],
        [
            "that",
            "were",
            "put",
            "there",
            "from",
            "battlefield",
            "this",
            "turn",
        ],
    ] {
        if let Some(word_start) = lower_words_find_sequence(&all_words, &phrase) {
            filter.entered_graveyard_this_turn = true;
            filter.entered_graveyard_from_battlefield_this_turn = true;
            all_words.drain(word_start..word_start + 8);

            let segment_words_view = GrammarFilterNormalizedWords::new(&segment_tokens);
            let segment_words = segment_words_view.to_word_refs();
            let mut segment_match: Option<(usize, usize)> = None;
            for (len, segment_phrase) in if phrase[1] == "was" {
                vec![
                    (
                        9usize,
                        &[
                            "that",
                            "was",
                            "put",
                            "there",
                            "from",
                            "the",
                            "battlefield",
                            "this",
                            "turn",
                        ][..],
                    ),
                    (
                        8usize,
                        &[
                            "that",
                            "was",
                            "put",
                            "there",
                            "from",
                            "battlefield",
                            "this",
                            "turn",
                        ][..],
                    ),
                ]
            } else {
                vec![
                    (
                        9usize,
                        &[
                            "that",
                            "were",
                            "put",
                            "there",
                            "from",
                            "the",
                            "battlefield",
                            "this",
                            "turn",
                        ][..],
                    ),
                    (
                        8usize,
                        &[
                            "that",
                            "were",
                            "put",
                            "there",
                            "from",
                            "battlefield",
                            "this",
                            "turn",
                        ][..],
                    ),
                ]
            } {
                if let Some(seg_start) = lower_words_find_sequence(&segment_words, segment_phrase) {
                    segment_match = Some((seg_start, len));
                    break;
                }
            }
            if let Some((seg_start, len)) = segment_match
                && let Some(start_token_idx) =
                    normalized_token_index_for_word_index(&segment_tokens, seg_start)
            {
                let end_word_idx = seg_start + len;
                let end_token_idx =
                    normalized_token_index_after_words(&segment_tokens, end_word_idx)
                        .unwrap_or(segment_tokens.len());
                segment_tokens.drain(start_token_idx..end_token_idx);
            }
            break;
        }
    }

    // "legendary or Rat card" (Nashi, Moon's Legacy) is a supertype/subtype disjunction.
    // We parse it by collecting both selectors and then expanding into an `any_of` filter
    // after the normal pass so other shared qualifiers (zone/owner/etc.) are preserved.
    let legendary_or_subtype = lower_words_find_window(&all_words, 3, |window| {
        window[0] == "legendary" && window[1] == "or" && parse_subtype_word(window[2]).is_some()
    })
    .and_then(|idx| parse_subtype_word(all_words[idx + 2]));

    // "in a graveyard that was put there from anywhere this turn" (Reenact the Crime)
    // means the card entered a graveyard this turn.
    for phrase in [
        [
            "that", "was", "put", "there", "from", "anywhere", "this", "turn",
        ],
        [
            "that", "were", "put", "there", "from", "anywhere", "this", "turn",
        ],
    ] {
        if let Some(word_start) = lower_words_find_sequence(&all_words, &phrase) {
            filter.entered_graveyard_this_turn = true;
            all_words.drain(word_start..word_start + 8);

            let segment_words_view = GrammarFilterNormalizedWords::new(&segment_tokens);
            let segment_words = segment_words_view.to_word_refs();
            if let Some(seg_start) = lower_words_find_sequence(&segment_words, &phrase)
                && let Some(start_token_idx) =
                    normalized_token_index_for_word_index(&segment_tokens, seg_start)
            {
                let end_word_idx = seg_start + 8;
                let end_token_idx =
                    normalized_token_index_after_words(&segment_tokens, end_word_idx)
                        .unwrap_or(segment_tokens.len());
                segment_tokens.drain(start_token_idx..end_token_idx);
            }
            break;
        }
    }

    // "... graveyard from the battlefield this turn" means the card entered a graveyard
    // from the battlefield this turn.
    for phrase in [
        ["graveyard", "from", "battlefield", "this", "turn"],
        ["graveyards", "from", "battlefield", "this", "turn"],
    ] {
        if let Some(word_start) = lower_words_find_sequence(&all_words, &phrase) {
            filter.entered_graveyard_from_battlefield_this_turn = true;
            all_words.drain(word_start + 1..word_start + 5);

            let segment_words_view = GrammarFilterNormalizedWords::new(&segment_tokens);
            let segment_words = segment_words_view.to_word_refs();
            let mut segment_match: Option<(usize, usize)> = None;
            for (len, phrase) in [
                (
                    6,
                    &["graveyard", "from", "the", "battlefield", "this", "turn"][..],
                ),
                (5, &["graveyard", "from", "battlefield", "this", "turn"][..]),
                (
                    6,
                    &["graveyards", "from", "the", "battlefield", "this", "turn"][..],
                ),
                (
                    5,
                    &["graveyards", "from", "battlefield", "this", "turn"][..],
                ),
            ] {
                if let Some(seg_start) = lower_words_find_sequence(&segment_words, phrase) {
                    segment_match = Some((seg_start, len));
                    break;
                }
            }
            if let Some((seg_start, len)) = segment_match
                && let Some(start_token_idx) =
                    normalized_token_index_for_word_index(&segment_tokens, seg_start + 1)
            {
                let end_word_idx = seg_start + len;
                let end_token_idx =
                    normalized_token_index_after_words(&segment_tokens, end_word_idx)
                        .unwrap_or(segment_tokens.len());
                segment_tokens.drain(start_token_idx..end_token_idx);
            }
            break;
        }
    }

    // "... entered the battlefield ... this turn" marks a battlefield entry this turn.
    let mut entered_battlefield_match: Option<(usize, usize, Option<PlayerFilter>)> = None;
    if let Some(idx) = lower_words_find_window(&all_words, 7, |window| {
        window[0] == "entered"
            && window[1] == "battlefield"
            && window[2] == "under"
            && window[4] == "control"
            && window[5] == "this"
            && window[6] == "turn"
    }) {
        let controller = match all_words[idx + 3] {
            "your" => Some(PlayerFilter::You),
            "opponent" | "opponents" => Some(PlayerFilter::Opponent),
            _ => None,
        };
        entered_battlefield_match = Some((idx, 7, controller));
    }
    if entered_battlefield_match.is_none() {
        if let Some(idx) =
            lower_words_find_sequence(&all_words, &["entered", "battlefield", "this", "turn"])
        {
            entered_battlefield_match = Some((idx, 4, None));
        }
    }
    if let Some((word_start, len, controller)) = entered_battlefield_match {
        filter.entered_battlefield_this_turn = true;
        filter.entered_battlefield_controller = controller;
        filter.zone = Some(Zone::Battlefield);
        all_words.drain(word_start..word_start + len);

        let segment_words_view = GrammarFilterNormalizedWords::new(&segment_tokens);
        let segment_words = segment_words_view.to_word_refs();
        let mut segment_match: Option<(usize, usize)> = None;
        for (len, phrase) in [
            (
                8,
                &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ][..],
            ),
            (
                7,
                &[
                    "entered",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ][..],
            ),
            (
                8,
                &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ][..],
            ),
            (
                8,
                &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "opponents",
                    "control",
                    "this",
                    "turn",
                ][..],
            ),
            (
                7,
                &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ][..],
            ),
            (
                7,
                &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponents",
                    "control",
                    "this",
                    "turn",
                ][..],
            ),
            (5, &["entered", "the", "battlefield", "this", "turn"][..]),
            (4, &["entered", "battlefield", "this", "turn"][..]),
        ] {
            if let Some(seg_start) = lower_words_find_sequence(&segment_words, phrase) {
                segment_match = Some((seg_start, len));
                break;
            }
        }
        if let Some((seg_start, len)) = segment_match
            && let Some(start_token_idx) =
                normalized_token_index_for_word_index(&segment_tokens, seg_start)
        {
            let end_word_idx = seg_start + len;
            let end_token_idx = normalized_token_index_after_words(&segment_tokens, end_word_idx)
                .unwrap_or(segment_tokens.len());
            segment_tokens.drain(start_token_idx..end_token_idx);
        }
    }

    // Avoid treating reference phrases like "... with mana value equal to the number of charge
    // counters on this artifact" as additional type selectors on the filtered object.
    // (Aether Vial: "put a creature card with mana value equal to the number of charge counters
    // on this artifact from your hand onto the battlefield.")
    let mut mv_eq_counter_idx = 0usize;
    while mv_eq_counter_idx + 11 < all_words.len() {
        let window = &all_words[mv_eq_counter_idx..mv_eq_counter_idx + 12];
        if window[0] == "with"
            && window[1] == "mana"
            && window[2] == "value"
            && window[3] == "equal"
            && window[4] == "to"
            && window[5] == "number"
            && window[6] == "of"
            && matches!(window[8], "counter" | "counters")
            && window[9] == "on"
            && window[10] == "this"
            && window[11] == "artifact"
            && let Some(counter_type) = parse_counter_type_word(window[7])
        {
            filter.mana_value_eq_counters_on_source = Some(counter_type);
            all_words.drain(mv_eq_counter_idx..mv_eq_counter_idx + 12);

            // Also drop the reference phrase from the token-backed segment list so later
            // card-type/subtype extraction doesn't incorrectly treat "artifact" as part of the
            // filtered object's identity.
            let segment_words_view = GrammarFilterNormalizedWords::new(&segment_tokens);
            let segment_words = segment_words_view.to_word_refs();
            let mut segment_match: Option<(usize, usize)> = None;
            for len in [13usize, 12usize] {
                let Some(idx) = lower_words_find_window(&segment_words, len, |window| {
                    if len == 13 {
                        window[0] == "with"
                            && window[1] == "mana"
                            && window[2] == "value"
                            && window[3] == "equal"
                            && window[4] == "to"
                            && window[5] == "the"
                            && window[6] == "number"
                            && window[7] == "of"
                            && matches!(window[9], "counter" | "counters")
                            && window[10] == "on"
                            && window[11] == "this"
                            && window[12] == "artifact"
                            && parse_counter_type_word(window[8]).is_some()
                    } else {
                        window[0] == "with"
                            && window[1] == "mana"
                            && window[2] == "value"
                            && window[3] == "equal"
                            && window[4] == "to"
                            && window[5] == "number"
                            && window[6] == "of"
                            && matches!(window[8], "counter" | "counters")
                            && window[9] == "on"
                            && window[10] == "this"
                            && window[11] == "artifact"
                            && parse_counter_type_word(window[7]).is_some()
                    }
                }) else {
                    continue;
                };
                segment_match = Some((idx, len));
                break;
            }
            if let Some((start_word_idx, len)) = segment_match
                && let Some(start_token_idx) =
                    normalized_token_index_for_word_index(&segment_tokens, start_word_idx)
            {
                let end_word_idx = start_word_idx + len;
                let end_token_idx =
                    normalized_token_index_after_words(&segment_tokens, end_word_idx)
                        .unwrap_or(segment_tokens.len());
                if start_token_idx < end_token_idx && end_token_idx <= segment_tokens.len() {
                    segment_tokens.drain(start_token_idx..end_token_idx);
                }
            }

            continue;
        }
        mv_eq_counter_idx += 1;
    }

    let mut attached_exclusion_idx = 0usize;
    while attached_exclusion_idx + 2 < all_words.len() {
        if all_words[attached_exclusion_idx] != "other"
            || all_words[attached_exclusion_idx + 1] != "than"
        {
            attached_exclusion_idx += 1;
            continue;
        }

        let Some((tag, mut drain_end)) = (match all_words.get(attached_exclusion_idx + 2).copied() {
            Some("enchanted") => Some((TagKey::from("enchanted"), attached_exclusion_idx + 3)),
            Some("equipped") => Some((TagKey::from("equipped"), attached_exclusion_idx + 3)),
            _ => None,
        }) else {
            attached_exclusion_idx += 1;
            continue;
        };

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
        all_words.drain(attached_exclusion_idx..drain_end);
    }

    if let Some((power, toughness)) = all_words
        .first()
        .and_then(|word| parse_unsigned_pt_word(word))
    {
        filter.power = Some(crate::filter::Comparison::Equal(power));
        filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
        all_words.remove(0);
    }

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

    if let Some(idx) = lower_words_find_sequence(&all_words, &["that", "isnt", "all", "colors"]) {
        filter.all_colors = Some(false);
        all_words.drain(idx..idx + 4);
    } else if let Some(idx) = lower_words_find_sequence(&all_words, &["isnt", "all", "colors"]) {
        filter.all_colors = Some(false);
        all_words.drain(idx..idx + 3);
    }

    if let Some(idx) =
        lower_words_find_sequence(&all_words, &["that", "isnt", "exactly", "two", "colors"])
    {
        filter.exactly_two_colors = Some(false);
        all_words.drain(idx..idx + 5);
    } else if let Some(idx) =
        lower_words_find_sequence(&all_words, &["isnt", "exactly", "two", "colors"])
    {
        filter.exactly_two_colors = Some(false);
        all_words.drain(idx..idx + 4);
    }

    if all_words.len() >= 2 && matches!(all_words[0], "that" | "those" | "chosen") {
        let noun_idx = if all_words.get(1).is_some_and(|word| *word == "other") {
            2
        } else {
            1
        };
        if all_words
            .get(noun_idx)
            .is_some_and(|word| is_demonstrative_object_head(word))
        {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
            all_words.remove(0);
        }
    }

    if all_words
        .first()
        .is_some_and(|word| matches!(*word, "it" | "them"))
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    }

    if let Some(idx) = lower_words_find_sequence(
        &all_words,
        &["that", "entered", "since", "your", "last", "turn", "ended"],
    ) {
        filter.entered_since_your_last_turn_ended = true;
        all_words.drain(idx..idx + 7);
    } else if let Some(idx) = lower_words_find_sequence(
        &all_words,
        &["entered", "since", "your", "last", "turn", "ended"],
    ) {
        filter.entered_since_your_last_turn_ended = true;
        all_words.drain(idx..idx + 6);
    }

    let mut face_state_idx = 0usize;
    while face_state_idx < all_words.len() {
        if matches!(all_words[face_state_idx], "face-down" | "facedown") {
            filter.face_down = Some(true);
            all_words.remove(face_state_idx);
            continue;
        }
        if matches!(all_words[face_state_idx], "face-up" | "faceup") {
            filter.face_down = Some(false);
            all_words.remove(face_state_idx);
            continue;
        }
        if face_state_idx + 1 < all_words.len() && all_words[face_state_idx] == "face" {
            if all_words[face_state_idx + 1] == "down" {
                filter.face_down = Some(true);
                all_words.drain(face_state_idx..face_state_idx + 2);
                continue;
            }
            if all_words[face_state_idx + 1] == "up" {
                filter.face_down = Some(false);
                all_words.drain(face_state_idx..face_state_idx + 2);
                continue;
            }
        }
        face_state_idx += 1;
    }

    if lower_words_has_window(&all_words, 3, |window| {
        window == ["entered", "this", "turn"]
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported entered-this-turn object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if lower_words_has_window(&all_words, 4, |window| {
        window == ["counter", "on", "it", "or"] || window == ["counter", "on", "them", "or"]
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-state object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if all_words.first().is_some_and(|word| *word == "single")
        && all_words.get(1).is_some_and(|word| *word == "graveyard")
    {
        filter.single_graveyard = true;
        all_words.remove(0);
    }
    let mut single_idx = 0usize;
    while single_idx + 1 < all_words.len() {
        if all_words[single_idx] == "single" && all_words[single_idx + 1] == "graveyard" {
            filter.single_graveyard = true;
            all_words.remove(single_idx);
            continue;
        }
        single_idx += 1;
    }

    if let Some(not_named_idx) = lower_words_find_sequence(&all_words, &["not", "named"]) {
        let mut name_end = all_words.len();
        for idx in (not_named_idx + 2)..all_words.len() {
            if idx == not_named_idx + 2 {
                continue;
            }
            if matches!(
                all_words[idx],
                "in" | "from"
                    | "with"
                    | "without"
                    | "that"
                    | "which"
                    | "who"
                    | "whose"
                    | "under"
                    | "among"
                    | "on"
                    | "you"
                    | "your"
                    | "opponent"
                    | "opponents"
                    | "their"
                    | "its"
                    | "controller"
                    | "controllers"
                    | "owner"
                    | "owners"
            ) {
                name_end = idx;
                break;
            }
        }
        let full_not_named_idx = map_non_article_index(not_named_idx).unwrap_or(not_named_idx);
        let full_name_end = map_non_article_end(name_end).unwrap_or(name_end);
        let name_words = if full_not_named_idx + 2 <= full_name_end
            && full_name_end <= all_words_with_articles.len()
        {
            &all_words_with_articles[full_not_named_idx + 2..full_name_end]
        } else {
            &all_words[not_named_idx + 2..name_end]
        };
        if name_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing card name in not-named object filter (clause: '{}')",
                all_words.join(" ")
            )));
        }
        filter.excluded_name = Some(name_words.join(" "));
        let mut remaining = Vec::with_capacity(all_words.len());
        remaining.extend_from_slice(&all_words[..not_named_idx]);
        remaining.extend_from_slice(&all_words[name_end..]);
        all_words = remaining;
    }

    if let Some(named_idx) = lower_words_find_index(&all_words, |word| word == "named") {
        let mut name_end = all_words.len();
        for idx in (named_idx + 1)..all_words.len() {
            if idx == named_idx + 1 {
                continue;
            }
            if matches!(
                all_words[idx],
                "in" | "from"
                    | "with"
                    | "without"
                    | "that"
                    | "which"
                    | "who"
                    | "whose"
                    | "under"
                    | "among"
                    | "on"
                    | "you"
                    | "your"
                    | "opponent"
                    | "opponents"
                    | "their"
                    | "its"
                    | "controller"
                    | "controllers"
                    | "owner"
                    | "owners"
            ) {
                name_end = idx;
                break;
            }
        }
        let full_named_idx = map_non_article_index(named_idx).unwrap_or(named_idx);
        let full_name_end = map_non_article_end(name_end).unwrap_or(name_end);
        let name_words = if full_named_idx + 1 <= full_name_end
            && full_name_end <= all_words_with_articles.len()
        {
            &all_words_with_articles[full_named_idx + 1..full_name_end]
        } else {
            &all_words[named_idx + 1..name_end]
        };
        if name_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing card name in named object filter (clause: '{}')",
                all_words.join(" ")
            )));
        }
        filter.name = Some(name_words.join(" "));
        let mut remaining = Vec::with_capacity(all_words.len());
        remaining.extend_from_slice(&all_words[..named_idx]);
        remaining.extend_from_slice(&all_words[name_end..]);
        all_words = remaining;
    }

    if let Some(color_count_idx) = lower_words_find_window(&all_words, 4, |window| {
        matches!(
            window,
            ["one", "or", "more", "colors"]
                | ["one", "or", "more", "color"]
                | ["two", "or", "more", "colors"]
                | ["two", "or", "more", "color"]
                | ["three", "or", "more", "colors"]
                | ["three", "or", "more", "color"]
                | ["four", "or", "more", "colors"]
                | ["four", "or", "more", "color"]
                | ["five", "or", "more", "colors"]
                | ["five", "or", "more", "color"]
        )
    }) {
        let count_word = all_words[color_count_idx];
        if count_word == "one" {
            let any_color: ColorSet = Color::ALL.into_iter().collect();
            filter.colors = Some(any_color);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported color-count object filter (clause: '{}')",
                all_words.join(" ")
            )));
        }
    }
    let has_power_or_toughness_clause = lower_words_has_window(&all_words, 3, |window| {
        window == ["power", "or", "toughness"] || window == ["toughness", "or", "power"]
    });
    if has_power_or_toughness_clause
        && !all_words
            .iter()
            .any(|word| matches!(*word, "spell" | "spells"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
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

    if is_source_reference_words(&all_words) {
        filter.source = true;
    }

    if let Some(its_attached_idx) =
        lower_words_find_sequence(&all_words, &["its", "attached", "to"])
    {
        // Oracle often writes "the creature it's attached to"; tokenizer
        // normalization yields "its attached to", so restore the object-link
        // form parse_object_filter already understands.
        let mut normalized = Vec::with_capacity(all_words.len() + 1);
        normalized.extend_from_slice(&all_words[..its_attached_idx]);
        normalized.extend(["attached", "to", "it"]);
        normalized.extend_from_slice(&all_words[its_attached_idx + 3..]);
        all_words = normalized;
    }

    if let Some(attached_idx) = lower_words_find_index(&all_words, |word| word == "attached")
        && all_words.get(attached_idx + 1) == Some(&"to")
    {
        let attached_to_words = &all_words[attached_idx + 2..];
        let references_it = lower_words_starts_with(attached_to_words, &["it"])
            || lower_words_starts_with(attached_to_words, &["that", "object"])
            || lower_words_starts_with(attached_to_words, &["that", "creature"])
            || lower_words_starts_with(attached_to_words, &["that", "permanent"])
            || lower_words_starts_with(attached_to_words, &["that", "equipment"])
            || lower_words_starts_with(attached_to_words, &["that", "aura"]);
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

    if let Some(relation_idx) = lower_words_find_window(&all_words, 6, |window| {
        matches!(
            window,
            ["blocking", "or", "blocked", "by", "this", "creature"]
                | ["blocking", "or", "blocked", "by", "this", "permanent"]
                | ["blocking", "or", "blocked", "by", "this", "source"]
        )
    }) {
        filter.in_combat_with_source = true;
        all_words.truncate(relation_idx);
    }

    let starts_with_exiled_card = lower_words_starts_with(&all_words, &["exiled", "card"])
        || lower_words_starts_with(&all_words, &["exiled", "cards"]);
    if starts_with_exiled_card {
        filter.zone.get_or_insert(Zone::Exile);
    }
    let has_exiled_with_phrase =
        lower_words_has_window(&all_words, 2, |window| window == ["exiled", "with"]);
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
        if let Some(exiled_with_idx) = lower_words_find_sequence(&all_words, &["exiled", "with"]) {
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
        if let Some(exiled_with_idx) = token_find_window(&segment_tokens, 2, |window| {
            window[0].is_word("exiled") && window[1].is_word("with")
        }) {
            let mut reference_end = exiled_with_idx + 2;
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
                segment_tokens.drain(exiled_with_idx + 1..reference_end);
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
            return Ok(filter);
        }
        all_words.remove(0);
    }

    let has_share_card_type = (lower_words_contains(&all_words, "share")
        || lower_words_contains(&all_words, "shares"))
        && (lower_words_contains(&all_words, "card")
            || lower_words_contains(&all_words, "permanent"))
        && lower_words_contains(&all_words, "type")
        && lower_words_contains(&all_words, "it");
    let has_share_color = lower_words_contains(&all_words, "shares")
        && lower_words_contains(&all_words, "color")
        && lower_words_contains(&all_words, "it");
    let has_same_mana_value = lower_words_has_window(&all_words, 4, |window| {
        window == ["same", "mana", "value", "as"]
    });
    let has_equal_or_lesser_mana_value = lower_words_has_window(&all_words, 5, |window| {
        window == ["equal", "or", "lesser", "mana", "value"]
    });
    let has_lte_mana_value_as_tagged = lower_words_has_window(&all_words, 8, |window| {
        matches!(
            window,
            [
                "equal", "or", "lesser", "mana", "value", "than", "that", "spell"
            ] | [
                "equal", "or", "lesser", "mana", "value", "than", "that", "card"
            ] | [
                "equal", "or", "lesser", "mana", "value", "than", "that", "object"
            ]
        )
    }) || lower_words_has_window(&all_words, 9, |window| {
        matches!(
            window,
            [
                "less", "than", "or", "equal", "to", "that", "spells", "mana", "value",
            ] | [
                "less", "than", "or", "equal", "to", "that", "cards", "mana", "value",
            ] | [
                "less", "than", "or", "equal", "to", "that", "objects", "mana", "value",
            ]
        )
    }) || has_equal_or_lesser_mana_value;
    let has_lt_mana_value_as_tagged = lower_words_has_window(&all_words, 3, |window| {
        window == ["lesser", "mana", "value"]
    }) && !has_equal_or_lesser_mana_value;
    let references_sacrifice_cost_object = lower_words_has_window(&all_words, 3, |window| {
        matches!(
            window,
            ["the", "sacrificed", "creature"]
                | ["the", "sacrificed", "artifact"]
                | ["the", "sacrificed", "permanent"]
                | ["a", "sacrificed", "creature"]
                | ["a", "sacrificed", "artifact"]
                | ["a", "sacrificed", "permanent"]
        )
    }) || lower_words_has_window(&all_words, 2, |window| {
        matches!(
            window,
            ["sacrificed", "creature"] | ["sacrificed", "artifact"] | ["sacrificed", "permanent"]
        )
    });
    let references_it_for_mana_value = all_words.iter().any(|word| matches!(*word, "it" | "its"))
        || lower_words_has_window(&all_words, 2, |window| {
            matches!(
                window,
                ["that", "object"]
                    | ["that", "creature"]
                    | ["that", "artifact"]
                    | ["that", "permanent"]
                    | ["that", "spell"]
                    | ["that", "card"]
            )
        });
    let has_same_name_as_tagged_object = lower_words_has_window(&all_words, 5, |window| {
        matches!(
            window,
            ["same", "name", "as", "that", "spell"]
                | ["same", "name", "as", "that", "card"]
                | ["same", "name", "as", "that", "object"]
                | ["same", "name", "as", "that", "creature"]
                | ["same", "name", "as", "that", "permanent"]
        )
    });

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

    if lower_words_has_window(&all_words, 4, |window| {
        window == ["that", "convoked", "this", "spell"]
    }) || lower_words_has_window(&all_words, 3, |window| window == ["that", "convoked", "it"])
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("convoked_this_spell"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if lower_words_has_window(&all_words, 5, |window| {
        window == ["that", "crewed", "it", "this", "turn"]
    }) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("crewed_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if lower_words_has_window(&all_words, 5, |window| {
        window == ["that", "saddled", "it", "this", "turn"]
    }) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("saddled_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if lower_words_has_window(&all_words, 3, |window| window == ["army", "you", "amassed"])
        || lower_words_has_window(&all_words, 2, |window| {
            matches!(window, ["amassed", "army"] | ["amassed", "armys"])
        })
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if lower_words_has_window(&all_words, 3, |window| {
        matches!(
            window,
            ["exiled", "this", "way"]
                | ["destroyed", "this", "way"]
                | ["sacrificed", "this", "way"]
                | ["revealed", "this", "way"]
                | ["discarded", "this", "way"]
                | ["milled", "this", "way"]
        )
    }) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    let references_target_player = lower_words_has_window(&all_words, 2, |window| {
        matches!(window, ["target", "player"] | ["target", "players"])
    });
    let references_target_opponent = lower_words_has_window(&all_words, 2, |window| {
        matches!(window, ["target", "opponent"] | ["target", "opponents"])
    });
    let pronoun_player_filter = if references_target_opponent {
        PlayerFilter::target_opponent()
    } else if references_target_player {
        PlayerFilter::target_player()
    } else {
        PlayerFilter::IteratedPlayer
    };

    if all_words.len() >= 2 {
        let mut idx = 0usize;
        while idx + 1 < all_words.len() {
            let window = &all_words[idx..idx + 2];
            match window {
                ["attacking", "you"] => {
                    filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::You);
                }
                ["attacking", "them"] => {
                    filter.attacking_player_or_planeswalker_controlled_by =
                        Some(pronoun_player_filter.clone());
                }
                ["attacking", "opponent"] | ["attacking", "opponents"] => {
                    filter.attacking_player_or_planeswalker_controlled_by =
                        Some(PlayerFilter::Opponent);
                }
                _ => {}
            }
            idx += 1;
        }
    }
    if all_words.len() >= 3 {
        let mut idx = 0usize;
        while idx + 2 < all_words.len() {
            let window = &all_words[idx..idx + 3];
            match window {
                ["attacking", "that", "player"] | ["attacking", "that", "players"] => {
                    filter.attacking_player_or_planeswalker_controlled_by =
                        Some(PlayerFilter::IteratedPlayer);
                }
                ["attacking", "defending", "player"] | ["attacking", "defending", "players"] => {
                    filter.attacking_player_or_planeswalker_controlled_by =
                        Some(PlayerFilter::Defending);
                }
                ["attacking", "target", "player"] | ["attacking", "target", "players"] => {
                    filter.attacking_player_or_planeswalker_controlled_by =
                        Some(PlayerFilter::target_player());
                }
                ["attacking", "target", "opponent"] | ["attacking", "target", "opponents"] => {
                    filter.attacking_player_or_planeswalker_controlled_by =
                        Some(PlayerFilter::target_opponent());
                }
                _ => {}
            }
            idx += 1;
        }
    }

    let is_tagged_spell_reference_at = |idx: usize| {
        all_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| matches!(*prev, "that" | "this" | "its" | "their"))
    };
    let contains_unqualified_spell_word = all_words.iter().enumerate().any(|(idx, word)| {
        matches!(*word, "spell" | "spells") && !is_tagged_spell_reference_at(idx)
    });
    let mentions_ability_word = all_words
        .iter()
        .any(|word| matches!(*word, "ability" | "abilities"));
    if contains_unqualified_spell_word && !mentions_ability_word {
        filter.has_mana_cost = true;
    }

    if all_words.len() >= 5 {
        let mut idx = 0usize;
        while idx + 4 < all_words.len() {
            let window = &all_words[idx..idx + 5];
            match window {
                ["you", "both", "own", "and", "control"]
                | ["you", "both", "own", "and", "controls"]
                | ["you", "both", "control", "and", "own"]
                | ["you", "both", "controls", "and", "own"] => {
                    filter.owner = Some(PlayerFilter::You);
                    filter.controller = Some(PlayerFilter::You);
                }
                ["opponent", "both", "own", "and", "control"]
                | ["opponent", "both", "own", "and", "controls"]
                | ["opponent", "both", "control", "and", "own"]
                | ["opponent", "both", "controls", "and", "own"]
                | ["opponents", "both", "own", "and", "control"]
                | ["opponents", "both", "own", "and", "controls"]
                | ["opponents", "both", "control", "and", "own"]
                | ["opponents", "both", "controls", "and", "own"] => {
                    filter.owner = Some(PlayerFilter::Opponent);
                    filter.controller = Some(PlayerFilter::Opponent);
                }
                ["they", "both", "own", "and", "control"]
                | ["they", "both", "own", "and", "controls"]
                | ["they", "both", "control", "and", "own"]
                | ["they", "both", "controls", "and", "own"] => {
                    filter.owner = Some(pronoun_player_filter.clone());
                    filter.controller = Some(pronoun_player_filter.clone());
                }
                _ => {}
            }
            idx += 1;
        }
    }
    if all_words.len() >= 2 {
        let mut idx = 0usize;
        while idx + 1 < all_words.len() {
            let window = &all_words[idx..idx + 2];
            match window {
                ["you", "cast"] | ["you", "casts"] => {
                    filter.cast_by = Some(PlayerFilter::You);
                }
                ["opponent", "cast"]
                | ["opponent", "casts"]
                | ["opponents", "cast"]
                | ["opponents", "casts"] => {
                    filter.cast_by = Some(PlayerFilter::Opponent);
                }
                ["they", "cast"] | ["they", "casts"] => {
                    filter.cast_by = Some(pronoun_player_filter.clone());
                }
                ["you", "control"] | ["you", "controls"] => {
                    filter.controller = Some(PlayerFilter::You);
                }
                ["you", "own"] | ["you", "owns"] => {
                    filter.owner = Some(PlayerFilter::You);
                }
                ["opponent", "control"]
                | ["opponent", "controls"]
                | ["opponents", "control"]
                | ["opponents", "controls"] => {
                    filter.controller = Some(PlayerFilter::Opponent);
                }
                ["opponent", "own"]
                | ["opponent", "owns"]
                | ["opponents", "own"]
                | ["opponents", "owns"] => {
                    filter.owner = Some(PlayerFilter::Opponent);
                }
                ["they", "control"] | ["they", "controls"] => {
                    filter.controller = Some(pronoun_player_filter.clone());
                }
                ["they", "own"] | ["they", "owns"] => {
                    filter.owner = Some(pronoun_player_filter.clone());
                }
                _ => {}
            }
            idx += 1;
        }
    }
    if all_words.len() >= 3 {
        let mut idx = 0usize;
        while idx + 2 < all_words.len() {
            let window = &all_words[idx..idx + 3];
            match window {
                ["chosen", "player", "graveyard"] | ["chosen", "players", "graveyard"] => {
                    filter.owner = Some(PlayerFilter::ChosenPlayer);
                    filter.zone = Some(Zone::Graveyard);
                }
                ["your", "team", "control"] | ["your", "team", "controls"] => {
                    filter.controller = Some(PlayerFilter::You);
                }
                ["your", "team", "own"] | ["your", "team", "owns"] => {
                    filter.owner = Some(PlayerFilter::You);
                }
                ["that", "player", "control"] | ["that", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::IteratedPlayer);
                }
                ["that", "player", "cast"] | ["that", "player", "casts"] => {
                    filter.cast_by = Some(PlayerFilter::IteratedPlayer);
                }
                ["defending", "player", "control"] | ["defending", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::Defending);
                }
                ["attacking", "player", "control"] | ["attacking", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::Attacking);
                }
                ["that", "player", "own"] | ["that", "player", "owns"] => {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                ["target", "player", "control"] | ["target", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::target_player());
                }
                ["target", "player", "cast"] | ["target", "player", "casts"] => {
                    filter.cast_by = Some(PlayerFilter::target_player());
                }
                ["target", "opponent", "control"] | ["target", "opponent", "controls"] => {
                    filter.controller = Some(PlayerFilter::target_opponent());
                }
                ["target", "opponent", "cast"] | ["target", "opponent", "casts"] => {
                    filter.cast_by = Some(PlayerFilter::target_opponent());
                }
                ["target", "player", "own"] | ["target", "player", "owns"] => {
                    filter.owner = Some(PlayerFilter::target_player());
                }
                ["target", "opponent", "own"] | ["target", "opponent", "owns"] => {
                    filter.owner = Some(PlayerFilter::target_opponent());
                }
                ["its", "controller", "control"]
                | ["its", "controller", "controls"]
                | ["its", "controllers", "control"]
                | ["its", "controllers", "controls"]
                | ["their", "controller", "control"]
                | ["their", "controller", "controls"]
                | ["their", "controllers", "control"]
                | ["their", "controllers", "controls"] => {
                    filter.controller =
                        Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
                }
                ["you", "dont", "control"] | ["you", "don't", "control"] => {
                    filter.controller = Some(PlayerFilter::NotYou);
                }
                ["you", "dont", "own"] | ["you", "don't", "own"] => {
                    filter.owner = Some(PlayerFilter::NotYou);
                }
                _ => {}
            }
            idx += 1;
        }
    }
    if all_words.len() >= 4 {
        let mut idx = 0usize;
        while idx + 3 < all_words.len() {
            let window = &all_words[idx..idx + 4];
            if window[1..] == ["your", "team", "control"]
                || window[1..] == ["your", "team", "controls"]
            {
                filter.controller = Some(PlayerFilter::You);
            } else if window == ["your", "opponents", "cast", "from"]
                || window == ["your", "opponents", "casts", "from"]
            {
                filter.cast_by = Some(PlayerFilter::Opponent);
            } else if window == ["the", "chosen", "player", "graveyard"]
                || window == ["the", "chosen", "players", "graveyard"]
            {
                filter.owner = Some(PlayerFilter::ChosenPlayer);
                filter.zone = Some(Zone::Graveyard);
            } else if window[1..] == ["your", "team", "own"]
                || window[1..] == ["your", "team", "owns"]
            {
                filter.owner = Some(PlayerFilter::You);
            } else if window == ["you", "do", "not", "control"] {
                filter.controller = Some(PlayerFilter::NotYou);
            } else if window == ["you", "do", "not", "own"] {
                filter.owner = Some(PlayerFilter::NotYou);
            }
            idx += 1;
        }
    }

    let mut with_idx = 0usize;
    while with_idx + 1 < all_words.len() {
        if all_words[with_idx] != "with" {
            with_idx += 1;
            continue;
        }

        if all_words
            .get(with_idx + 1)
            .is_some_and(|word| *word == "no")
            && all_words
                .get(with_idx + 2)
                .is_some_and(|word| matches!(*word, "ability" | "abilities"))
        {
            filter.no_abilities = true;
            with_idx += 3;
            continue;
        }

        if all_words
            .get(with_idx + 1)
            .is_some_and(|word| *word == "no")
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&all_words[with_idx + 2..])
        {
            filter.without_counter = Some(counter_constraint);
            with_idx += 2 + consumed;
            continue;
        }

        if let Some((kind, consumed)) = parse_alternative_cast_words(&all_words[with_idx + 1..]) {
            filter.alternative_cast = Some(kind);
            with_idx += 1 + consumed;
            continue;
        }
        if let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&all_words[with_idx + 1..])
        {
            filter.with_counter = Some(counter_constraint);
            with_idx += 1 + consumed;
            continue;
        }

        if let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&all_words[with_idx + 1..])
        {
            let after_constraint = with_idx + 1 + consumed;
            if all_words
                .get(after_constraint)
                .is_some_and(|word| *word == "or")
                && let Some((rhs_constraint, rhs_consumed)) =
                    parse_filter_keyword_constraint_words(&all_words[after_constraint + 1..])
            {
                // Model "with <keyword> or <keyword>" as an any-of filter.
                //
                // Each branch is deliberately "keyword-only"; the outer filter
                // keeps controller/type/etc qualifiers. Rendering is handled by
                // ObjectFilter::description() for simple any-of keyword lists.
                let mut left = ObjectFilter::default();
                apply_filter_keyword_constraint(&mut left, constraint, false);
                let mut right = ObjectFilter::default();
                apply_filter_keyword_constraint(&mut right, rhs_constraint, false);
                filter.any_of = vec![left, right];
                with_idx += 1 + consumed + 1 + rhs_consumed;
                continue;
            }

            apply_filter_keyword_constraint(&mut filter, constraint, false);
            with_idx += 1 + consumed;
            continue;
        }

        with_idx += 1;
    }

    let mut has_idx = 0usize;
    while has_idx + 1 < all_words.len() {
        if !matches!(all_words[has_idx], "has" | "have") {
            has_idx += 1;
            continue;
        }
        if filter.with_counter.is_none()
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&all_words[has_idx + 1..])
        {
            filter.with_counter = Some(counter_constraint);
            has_idx += 1 + consumed;
            continue;
        }
        has_idx += 1;
    }

    let mut without_idx = 0usize;
    while without_idx + 1 < all_words.len() {
        if all_words[without_idx] != "without" {
            without_idx += 1;
            continue;
        }

        if let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&all_words[without_idx + 1..])
        {
            apply_filter_keyword_constraint(&mut filter, constraint, true);
            without_idx += 1 + consumed;
            continue;
        }
        if let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&all_words[without_idx + 1..])
        {
            filter.without_counter = Some(counter_constraint);
            without_idx += 1 + consumed;
            continue;
        }

        without_idx += 1;
    }

    let has_tap_activated_ability = lower_words_has_window(&all_words, 9, |window| {
        window
            == [
                "has",
                "an",
                "activated",
                "ability",
                "with",
                "t",
                "in",
                "its",
                "cost",
            ]
    }) || lower_words_has_window(&all_words, 8, |window| {
        window
            == [
                "has",
                "activated",
                "ability",
                "with",
                "t",
                "in",
                "its",
                "cost",
            ]
    }) || lower_words_has_window(&all_words, 7, |window| {
        window
            == [
                "activated",
                "abilities",
                "with",
                "t",
                "in",
                "their",
                "costs",
            ]
    });
    if has_tap_activated_ability {
        filter.has_tap_activated_ability = true;
    }

    for idx in 0..all_words.len() {
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            let is_reference_zone_for_spell = if contains_unqualified_spell_word {
                idx > 0
                    && matches!(
                        all_words[idx - 1],
                        "controller"
                            | "controllers"
                            | "owner"
                            | "owners"
                            | "its"
                            | "their"
                            | "that"
                            | "this"
                    )
            } else {
                false
            };
            if is_reference_zone_for_spell {
                continue;
            }
            if filter.zone.is_none() {
                filter.zone = Some(zone);
            }
            if idx > 0 {
                match all_words[idx - 1] {
                    "your" => {
                        filter.owner = Some(PlayerFilter::You);
                    }
                    "opponent" | "opponents" => {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    "their" => {
                        filter.owner = Some(pronoun_player_filter.clone());
                    }
                    _ => {}
                }
            }
            if idx > 1 {
                let owner_pair = (all_words[idx - 2], all_words[idx - 1]);
                match owner_pair {
                    ("target", "player") | ("target", "players") => {
                        filter.owner = Some(PlayerFilter::target_player());
                    }
                    ("target", "opponent") | ("target", "opponents") => {
                        filter.owner = Some(PlayerFilter::target_opponent());
                    }
                    ("that", "player") | ("that", "players") => {
                        filter.owner = Some(PlayerFilter::IteratedPlayer);
                    }
                    _ => {}
                }
            }
        }
    }

    let clause_words = all_words.clone();
    for idx in 0..all_words.len() {
        let (is_base_reference, pt_word_idx) = if idx + 4 < all_words.len()
            && all_words[idx] == "base"
            && all_words[idx + 1] == "power"
            && all_words[idx + 2] == "and"
            && all_words[idx + 3] == "toughness"
        {
            (true, idx + 4)
        } else if idx + 3 < all_words.len()
            && all_words[idx] == "power"
            && all_words[idx + 1] == "and"
            && all_words[idx + 2] == "toughness"
            && (idx == 0 || all_words[idx - 1] != "base")
        {
            (false, idx + 3)
        } else {
            continue;
        };

        if let Ok((power, toughness)) = parse_pt_modifier(all_words[pt_word_idx]) {
            filter.power = Some(crate::filter::Comparison::Equal(power));
            filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
            filter.power_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
            filter.toughness_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
        }
    }

    let mut idx = 0usize;
    while idx < all_words.len() {
        let axis = match all_words[idx] {
            "power" => Some("power"),
            "toughness" => Some("toughness"),
            "mana" if idx + 1 < all_words.len() && all_words[idx + 1] == "value" => {
                Some("mana value")
            }
            _ => None,
        };
        let Some(axis) = axis else {
            idx += 1;
            continue;
        };
        let is_base_reference = idx > 0 && all_words[idx - 1] == "base";

        let axis_word_count = usize::from(axis == "mana value") + 1;
        let value_tokens = if idx + axis_word_count < all_words.len() {
            &all_words[idx + axis_word_count..]
        } else {
            &[]
        };
        let Some((cmp, consumed)) =
            parse_filter_comparison_tokens(axis, value_tokens, &clause_words)?
        else {
            idx += 1;
            continue;
        };

        match axis {
            "power" => {
                filter.power = Some(cmp);
                filter.power_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "toughness" => {
                filter.toughness = Some(cmp);
                filter.toughness_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        idx += axis_word_count + consumed;
    }

    apply_parity_filter_phrases(&clause_words, &mut filter);

    if lower_words_has_window(&clause_words, 6, |window| {
        window == ["power", "greater", "than", "its", "base", "power"]
    }) {
        filter.power_greater_than_base_power = true;
    }

    let mut saw_permanent = false;
    let mut saw_spell = false;
    let mut saw_permanent_type = false;

    let mut saw_subtype = false;
    let mut negated_word_indices = std::collections::HashSet::new();
    let mut negated_historic_indices = std::collections::HashSet::new();
    let is_text_negation_word =
        |word: &str| matches!(word, "not" | "isnt" | "isn't" | "arent" | "aren't");
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] != "non" {
            continue;
        }
        let next = all_words[idx + 1];
        if is_outlaw_word(next) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(card_type) = parse_card_type(next)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(idx + 1);
        }
        if next == "attacking" {
            filter.nonattacking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "blocking" {
            filter.nonblocking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "blocked" {
            filter.unblocked = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "commander" || next == "commanders" {
            filter.noncommander = true;
            negated_word_indices.insert(idx + 1);
        }
        if let Some(color) = parse_color(next) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(subtype) = parse_subtype_flexible(next)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(idx + 1);
        }
    }
    for idx in 0..all_words.len() {
        if !is_text_negation_word(all_words[idx]) {
            continue;
        }
        let mut target_idx = idx + 1;
        if target_idx >= all_words.len() {
            continue;
        }
        if is_article(all_words[target_idx]) {
            target_idx += 1;
            if target_idx >= all_words.len() {
                continue;
            }
        }

        let negated_word = all_words[target_idx];
        if negated_word == "attacking" {
            filter.nonattacking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "blocking" {
            filter.nonblocking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "blocked" {
            filter.unblocked = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "historic" {
            filter.nonhistoric = true;
            negated_historic_indices.insert(target_idx);
        }
        if negated_word == "commander" || negated_word == "commanders" {
            filter.noncommander = true;
            negated_word_indices.insert(target_idx);
        }
        if let Some(card_type) = parse_card_type(negated_word)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(target_idx);
        }
        if let Some(supertype) = parse_supertype_word(negated_word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
            negated_word_indices.insert(target_idx);
        }
        if let Some(color) = parse_color(negated_word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(target_idx);
        }
        if let Some(subtype) = parse_subtype_flexible(negated_word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(target_idx);
        }
    }
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] == "not" && all_words[idx + 1] == "historic" {
            filter.nonhistoric = true;
            negated_historic_indices.insert(idx + 1);
        }
    }

    for (idx, word) in all_words.iter().enumerate() {
        let is_negated_word = set_has(&negated_word_indices, &idx);
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => {
                if !is_tagged_spell_reference_at(idx) {
                    saw_spell = true;
                }
            }
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
            "other" => filter.other = true,
            "tapped" => filter.tapped = true,
            "untapped" => filter.untapped = true,
            "attacking" if !is_negated_word => filter.attacking = true,
            "nonattacking" => filter.nonattacking = true,
            "blocking" if !is_negated_word => filter.blocking = true,
            "nonblocking" => filter.nonblocking = true,
            "blocked" if !is_negated_word => filter.blocked = true,
            "unblocked" if !is_negated_word => filter.unblocked = true,
            "commander" | "commanders" => {
                let prev = idx.checked_sub(1).and_then(|i| all_words.get(i)).copied();
                let prev2 = idx.checked_sub(2).and_then(|i| all_words.get(i)).copied();
                let negated_by_phrase = prev.is_some_and(is_text_negation_word)
                    || (prev.is_some_and(is_article) && prev2.is_some_and(is_text_negation_word));
                if is_negated_word || negated_by_phrase {
                    filter.noncommander = true;
                } else {
                    filter.is_commander = true;
                    match prev {
                        Some("your") => filter.owner = Some(PlayerFilter::You),
                        Some("opponent") | Some("opponents") => {
                            filter.owner = Some(PlayerFilter::Opponent);
                        }
                        Some("their") => filter.owner = Some(pronoun_player_filter.clone()),
                        _ => {}
                    }
                }
            }
            "noncommander" | "noncommanders" => filter.noncommander = true,
            "nonbasic" => {
                filter = filter.without_supertype(Supertype::Basic);
            }
            "colorless" => filter.colorless = true,
            "multicolored" => filter.multicolored = true,
            "monocolored" => filter.monocolored = true,
            "nonhistoric" => filter.nonhistoric = true,
            "historic" if !set_has(&negated_historic_indices, &idx) => filter.historic = true,
            "modified" if !is_negated_word => filter.modified = true,
            _ => {}
        }

        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            continue;
        }

        if set_has(&negated_word_indices, &idx) {
            continue;
        }

        if is_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.subtypes);
            saw_subtype = true;
            continue;
        }

        if let Some(card_type) = parse_non_type(word) {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(supertype) = parse_non_supertype(word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
        }
        if let Some(subtype) = parse_non_subtype(word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(supertype) = parse_supertype_word(word)
            && !slice_has(&filter.supertypes, &supertype)
        {
            filter.supertypes.push(supertype);
        }

        if let Some(card_type) = parse_card_type(word) {
            push_unique(&mut filter.card_types, card_type);
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
        }

        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique(&mut filter.subtypes, subtype);
            saw_subtype = true;
        }
    }
    if saw_spell && source_linked_exile_reference {
        // "spell ... exiled with this" describes a stack spell with a relation
        // to source-linked exiled cards, not a spell object in exile.
        filter.zone = Some(Zone::Stack);
    }

    let segments = split_lexed_slices_on_or(&segment_tokens);
    let mut segment_types = Vec::new();
    let mut segment_subtypes = Vec::new();
    let mut segment_marker_counts = Vec::new();
    let mut segment_words_lists: Vec<Vec<String>> = Vec::new();

    for segment in &segments {
        let segment_words_view = GrammarFilterNormalizedWords::new(segment);
        let segment_words: Vec<String> = segment_words_view
            .to_word_refs()
            .into_iter()
            .filter(|word| !is_article(word))
            .map(ToString::to_string)
            .collect();
        segment_words_lists.push(segment_words.clone());
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &segment_words {
            if let Some(card_type) = parse_card_type(word) {
                push_unique(&mut types, card_type);
            }
            if let Some(subtype) = parse_subtype_flexible(word) {
                push_unique(&mut subtypes, subtype);
            }
        }
        segment_marker_counts.push(types.len() + subtypes.len());
        if !types.is_empty() {
            segment_types.push(types);
        }
        if !subtypes.is_empty() {
            segment_subtypes.push(subtypes);
        }
    }

    if segments.len() > 1 {
        let qualifier_in_all_segments = |qualifier: &str| {
            segment_words_lists
                .iter()
                .all(|segment| segment.iter().any(|word| word == qualifier))
        };
        let shared_leading_qualifier = |qualifier: &str, opposite: &str| {
            if qualifier_in_all_segments(qualifier) {
                return true;
            }
            if all_words.iter().any(|word| *word == opposite) {
                return false;
            }
            let Some(first_segment) = segment_words_lists.first() else {
                return false;
            };
            if !first_segment.iter().any(|word| word == qualifier) {
                return false;
            }
            segment_words_lists
                .iter()
                .skip(1)
                .all(|segment| !segment.iter().any(|word| word == opposite))
        };

        if filter.tapped && !shared_leading_qualifier("tapped", "untapped") {
            filter.tapped = false;
        }
        if filter.untapped && !shared_leading_qualifier("untapped", "tapped") {
            filter.untapped = false;
        }
    }

    if segments.len() > 1 {
        let type_list_candidate = !segment_marker_counts.is_empty()
            && segment_marker_counts.iter().all(|count| *count == 1);

        if type_list_candidate {
            let mut any_types = Vec::new();
            let mut any_subtypes = Vec::new();
            for types in segment_types {
                let Some(card_type) = types.first().copied() else {
                    continue;
                };
                push_unique(&mut any_types, card_type);
            }
            for subtypes in segment_subtypes {
                let Some(subtype) = subtypes.first().copied() else {
                    continue;
                };
                push_unique(&mut any_subtypes, subtype);
            }
            if !any_types.is_empty() {
                filter.card_types = any_types;
            }
            if !any_subtypes.is_empty() {
                filter.subtypes = any_subtypes;
            }
            if !filter.card_types.is_empty() && !filter.subtypes.is_empty() {
                filter.type_or_subtype_union = true;
            }
        }
    } else if let Some(types) = segment_types.into_iter().next() {
        let has_and = lower_words_contains(&all_words, "and");
        let has_or = lower_words_contains(&all_words, "or");
        if types.len() > 1 {
            if has_and && !has_or {
                filter.card_types = types;
            } else {
                filter.all_card_types = types;
            }
        } else if types.len() == 1 {
            filter.card_types = types;
        }
    }

    let permanent_type_defaults = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    let and_segments = split_lexed_slices_on_and(&segment_tokens);
    let and_segment_words_lists: Vec<Vec<String>> = and_segments
        .iter()
        .map(|segment| {
            GrammarFilterNormalizedWords::new(segment)
                .to_word_refs()
                .into_iter()
                .filter(|word| !is_article(word))
                .map(ToString::to_string)
                .collect()
        })
        .collect();

    let segment_has_standalone_spell = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| matches!(word.as_str(), "spell" | "spells"));
        if !contains_spell {
            return false;
        }

        !segment.iter().any(|word| {
            matches!(
                word.as_str(),
                "permanent" | "permanents" | "card" | "cards" | "source" | "sources"
            ) || parse_card_type(word).is_some()
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_nonspell_permanent_head = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| matches!(word.as_str(), "spell" | "spells"));
        if contains_spell {
            return false;
        }

        segment.iter().any(|word| {
            matches!(word.as_str(), "permanent" | "permanents")
                || parse_card_type(word).is_some_and(is_permanent_type)
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_permanent_spell_head = |segment: &[String]| {
        if segment.len() < 2 {
            return false;
        }
        let mut idx = 0usize;
        while idx + 1 < segment.len() {
            let permanent = &segment[idx];
            let spell = &segment[idx + 1];
            if (permanent == "permanent" || permanent == "permanents")
                && (spell == "spell" || spell == "spells")
            {
                return true;
            }
            idx += 1;
        }
        false
    };
    let has_standalone_spell_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_standalone_spell(segment));
    let has_nonspell_permanent_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_nonspell_permanent_head(segment));
    let has_split_permanent_spell_segments = and_segment_words_lists.len() > 1
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_permanent_spell_head(segment))
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_nonspell_permanent_head(segment));

    if saw_spell && has_standalone_spell_segment && has_nonspell_permanent_segment {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.card_types.clear();
        spell_filter.all_card_types.clear();
        spell_filter.subtypes.clear();
        spell_filter.type_or_subtype_union = false;

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent && has_split_permanent_spell_segments {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.has_mana_cost = false;
        if spell_filter.card_types.is_empty()
            && spell_filter.all_card_types.is_empty()
            && spell_filter.subtypes.is_empty()
        {
            spell_filter.card_types = permanent_type_defaults.clone();
        }

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent {
        if filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
        filter.zone = Some(Zone::Stack);
    } else {
        if saw_permanent && filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
    }

    if filter.any_of.is_empty() {
        if let Some(zone) = filter.zone {
            if saw_spell && zone != Zone::Stack {
                let is_spell_origin_zone = matches!(
                    zone,
                    Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command
                );
                if !is_spell_origin_zone {
                    return Err(CardTextError::ParseError(
                        "spell targets must be on the stack".to_string(),
                    ));
                }
            }
        } else if saw_spell {
            filter.zone = Some(Zone::Stack);
        } else if saw_permanent || saw_permanent_type || saw_subtype {
            filter.zone = Some(Zone::Battlefield);
        }
    }

    if contains_unqualified_spell_word
        && filter.cast_by.is_some()
        && matches!(
            filter.zone,
            Some(Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command)
        )
    {
        filter.owner = None;
    }

    if target_player.is_some() || target_object.is_some() {
        filter = filter.targeting(target_player.take(), target_object.take());
    }

    if let Some(or_subtype) = legendary_or_subtype
        && filter.any_of.is_empty()
        && slice_has(&filter.supertypes, &Supertype::Legendary)
        && slice_has(&filter.subtypes, &or_subtype)
    {
        let mut legendary_branch = filter.clone();
        legendary_branch.any_of.clear();
        legendary_branch
            .subtypes
            .retain(|subtype| *subtype != or_subtype);

        let mut subtype_branch = filter.clone();
        subtype_branch.any_of.clear();
        subtype_branch
            .supertypes
            .retain(|supertype| *supertype != Supertype::Legendary);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![legendary_branch, subtype_branch];
        filter = disjunction;
    }

    let owner_or_controller_player = lower_words_find_window(&all_words, 4, |window| {
        matches!(
            window,
            ["you", "own", "or", "control"]
                | ["you", "owns", "or", "controls"]
                | ["you", "control", "or", "own"]
                | ["you", "controls", "or", "owns"]
                | ["opponent", "own", "or", "control"]
                | ["opponent", "owns", "or", "controls"]
                | ["opponents", "own", "or", "control"]
                | ["opponents", "owns", "or", "controls"]
                | ["opponent", "control", "or", "own"]
                | ["opponent", "controls", "or", "owns"]
                | ["opponents", "control", "or", "own"]
                | ["opponents", "controls", "or", "owns"]
                | ["they", "own", "or", "control"]
                | ["they", "owns", "or", "controls"]
                | ["they", "control", "or", "own"]
                | ["they", "controls", "or", "owns"]
        )
    })
    .map(|idx| match &all_words[idx..idx + 4] {
        ["you", ..] => PlayerFilter::You,
        ["opponent", ..] | ["opponents", ..] => PlayerFilter::Opponent,
        ["they", ..] => pronoun_player_filter.clone(),
        _ => unreachable!("matched 4-word owner/controller phrase"),
    })
    .or_else(|| {
        lower_words_find_window(&all_words, 5, |window| {
            matches!(
                window,
                ["your", "team", "own", "or", "control"]
                    | ["your", "team", "owns", "or", "controls"]
                    | ["your", "team", "control", "or", "own"]
                    | ["your", "team", "controls", "or", "owns"]
                    | ["that", "player", "own", "or", "control"]
                    | ["that", "player", "owns", "or", "controls"]
                    | ["that", "player", "control", "or", "own"]
                    | ["that", "player", "controls", "or", "owns"]
                    | ["target", "player", "own", "or", "control"]
                    | ["target", "player", "owns", "or", "controls"]
                    | ["target", "player", "control", "or", "own"]
                    | ["target", "player", "controls", "or", "owns"]
                    | ["target", "opponent", "own", "or", "control"]
                    | ["target", "opponent", "owns", "or", "controls"]
                    | ["target", "opponent", "control", "or", "own"]
                    | ["target", "opponent", "controls", "or", "owns"]
            )
        })
        .map(|idx| match &all_words[idx..idx + 5] {
            ["your", "team", ..] => PlayerFilter::You,
            ["that", "player", ..] => PlayerFilter::IteratedPlayer,
            ["target", "player", ..] => PlayerFilter::target_player(),
            ["target", "opponent", ..] => PlayerFilter::target_opponent(),
            _ => unreachable!("matched 5-word owner/controller phrase"),
        })
    });
    if let Some(player_filter) = owner_or_controller_player
        && filter.any_of.is_empty()
    {
        let mut base = filter.clone();
        base.any_of.clear();
        base.owner = None;
        base.controller = None;

        let mut owner_branch = base.clone();
        owner_branch.owner = Some(player_filter.clone());

        let mut controller_branch = base;
        controller_branch.controller = Some(player_filter);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![owner_branch, controller_branch];
        filter = disjunction;
    }

    if has_power_or_toughness_clause && saw_spell {
        let mut power_or_toughness_cmp = None;
        for idx in 0..all_words.len() {
            let (_, value_tokens) = match all_words.get(idx..) {
                Some(["power", "or", "toughness", rest @ ..])
                | Some(["toughness", "or", "power", rest @ ..]) => {
                    (crate::filter::PtReference::Effective, rest)
                }
                _ => continue,
            };
            let Some((cmp, _)) =
                parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
            else {
                continue;
            };
            power_or_toughness_cmp = Some(cmp);
            break;
        }
        if let Some(cmp) = power_or_toughness_cmp {
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
            filter = disjunction;
        }
    }

    let has_constraints = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.other
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();

    if !has_constraints {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase (clause: '{}')",
            all_words.join(" ")
        )));
    }

    let has_object_identity = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.colors.is_some()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();
    if !has_object_identity {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase lacking object selector (clause: '{}')",
            all_words.join(" ")
        )));
    }

    if vote_winners_only {
        filter = filter.match_tagged(
            TagKey::from(VOTE_WINNERS_TAG),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }

    if not_on_battlefield && filter.any_of.is_empty() && !matches!(filter.zone, Some(Zone::Stack)) {
        let mut base = filter.clone();
        base.any_of.clear();
        base.zone = None;

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ]
        .into_iter()
        .map(|zone| {
            let mut branch = base.clone();
            branch.zone = Some(zone);
            branch
        })
        .collect();
        filter = disjunction;
    }

    Ok(filter)
}

pub(crate) fn parse_object_filter_with_grammar_entrypoint(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter(tokens, other)
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    let words_view = GrammarFilterNormalizedWords::new(tokens);
    let words: Vec<&str> = words_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let clause_words = words.clone();

    let mut idx = 0usize;
    while idx < words.len() {
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            idx += consumed;
            continue;
        }

        let word = words[idx];
        if matches!(word, "face-down" | "facedown") {
            filter.face_down = Some(true);
            idx += 1;
            continue;
        }
        if matches!(word, "face-up" | "faceup") {
            filter.face_down = Some(false);
            idx += 1;
            continue;
        }
        if word == "face" && idx + 1 < words.len() {
            if words[idx + 1] == "down" {
                filter.face_down = Some(true);
                idx += 2;
                continue;
            }
            if words[idx + 1] == "up" {
                filter.face_down = Some(false);
                idx += 2;
                continue;
            }
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

    let mut cmp_idx = 0usize;
    while cmp_idx < words.len() {
        let axis = match words[cmp_idx] {
            "power" => Some("power"),
            "toughness" => Some("toughness"),
            "mana" if cmp_idx + 1 < words.len() && words[cmp_idx + 1] == "value" => {
                Some("mana value")
            }
            _ => None,
        };
        let Some(axis) = axis else {
            cmp_idx += 1;
            continue;
        };

        let axis_word_count = usize::from(axis == "mana value") + 1;
        let value_tokens = if cmp_idx + axis_word_count < words.len() {
            &words[cmp_idx + axis_word_count..]
        } else {
            &[]
        };
        let parsed = parse_filter_comparison_tokens(axis, value_tokens, &clause_words)
            .ok()
            .flatten();
        let Some((cmp, consumed)) = parsed else {
            cmp_idx += 1;
            continue;
        };

        match axis {
            "power" => filter.power = Some(cmp),
            "toughness" => filter.toughness = Some(cmp),
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        cmp_idx += axis_word_count + consumed;
    }

    apply_spell_filter_parity_phrases(&clause_words, &mut filter);

    for idx in 0..words.len() {
        let Some(value_tokens) = (match words.get(idx..) {
            Some(["power", "or", "toughness", rest @ ..]) => Some(rest),
            _ => None,
        }) else {
            continue;
        };
        let Some((cmp, _)) = parse_filter_comparison_tokens("power", value_tokens, &clause_words)
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
        return disjunction;
    }

    filter
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    let words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let clause_words = words.clone();

    let mut idx = 0usize;
    while idx < words.len() {
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            idx += consumed;
            continue;
        }

        let word = words[idx];
        if matches!(word, "face-down" | "facedown") {
            filter.face_down = Some(true);
            idx += 1;
            continue;
        }
        if matches!(word, "face-up" | "faceup") {
            filter.face_down = Some(false);
            idx += 1;
            continue;
        }
        if word == "face" && idx + 1 < words.len() {
            if words[idx + 1] == "down" {
                filter.face_down = Some(true);
                idx += 2;
                continue;
            }
            if words[idx + 1] == "up" {
                filter.face_down = Some(false);
                idx += 2;
                continue;
            }
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

    apply_spell_filter_parity_phrases(&words, &mut filter);

    let mut cmp_idx = 0usize;
    while cmp_idx < words.len() {
        let axis = match words[cmp_idx] {
            "power" => Some("power"),
            "toughness" => Some("toughness"),
            "mana" if cmp_idx + 1 < words.len() && words[cmp_idx + 1] == "value" => {
                Some("mana value")
            }
            _ => None,
        };
        let Some(axis) = axis else {
            cmp_idx += 1;
            continue;
        };

        let axis_word_count = usize::from(axis == "mana value") + 1;
        let value_tokens = if cmp_idx + axis_word_count < words.len() {
            &words[cmp_idx + axis_word_count..]
        } else {
            &[]
        };
        let parsed = parse_filter_comparison_tokens(axis, value_tokens, &clause_words)
            .ok()
            .flatten();
        let Some((cmp, consumed)) = parsed else {
            cmp_idx += 1;
            continue;
        };

        match axis {
            "power" => filter.power = Some(cmp),
            "toughness" => filter.toughness = Some(cmp),
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        cmp_idx += axis_word_count + consumed;
    }

    apply_spell_filter_parity_phrases(&clause_words, &mut filter);

    for idx in 0..words.len() {
        let Some(value_tokens) = (match words.get(idx..) {
            Some(["power", "or", "toughness", rest @ ..]) => Some(rest),
            _ => None,
        }) else {
            continue;
        };
        let Some((cmp, _)) = parse_filter_comparison_tokens("power", value_tokens, &clause_words)
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
        return disjunction;
    }

    filter
}

fn parse_meld_subject_filter(words: &[&str]) -> Result<ObjectFilter, CardTextError> {
    if words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing meld predicate subject".to_string(),
        ));
    }
    if is_source_reference_words(words) {
        return Ok(ObjectFilter::source());
    }

    let tokens = synth_words_as_tokens(words);
    parse_object_filter(&tokens, false)
        .or_else(|_| Ok(ObjectFilter::default().named(words.join(" "))))
}

fn is_plausible_meld_subject_start(word: &str) -> bool {
    matches!(
        word,
        "a" | "an"
            | "another"
            | "this"
            | "that"
            | "source"
            | "artifact"
            | "battle"
            | "card"
            | "creature"
            | "enchantment"
            | "land"
            | "nonland"
            | "permanent"
            | "planeswalker"
    )
}

fn find_meld_subject_split(words: &[&str]) -> Option<usize> {
    words
        .iter()
        .enumerate()
        .find_map(|(idx, word)| {
            (*word == "and"
                && words
                    .get(idx + 1)
                    .is_some_and(|next| is_plausible_meld_subject_start(next)))
            .then_some(idx)
        })
        .or_else(|| find_index(words, |word| *word == "and"))
}

pub(super) fn parse_predicate(tokens: &[OwnedLexToken]) -> Result<PredicateAst, CardTextError> {
    let raw_words_view = GrammarFilterNormalizedWords::new(tokens);
    let raw_words = raw_words_view.to_word_refs();
    let mut filtered: Vec<&str> = raw_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }

    for (phrase, zone) in [
        (
            ["this", "card", "is", "in", "your", "hand"].as_slice(),
            Zone::Hand,
        ),
        (
            ["this", "card", "is", "in", "your", "graveyard"].as_slice(),
            Zone::Graveyard,
        ),
        (
            ["this", "card", "is", "in", "your", "library"].as_slice(),
            Zone::Library,
        ),
        (
            ["this", "card", "is", "in", "exile"].as_slice(),
            Zone::Exile,
        ),
        (
            ["this", "card", "is", "in", "the", "command", "zone"].as_slice(),
            Zone::Command,
        ),
    ] {
        if filtered.as_slice() == phrase {
            return Ok(PredicateAst::SourceIsInZone(zone));
        }
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(gets_idx) = find_index(&filtered, |word| *word == "gets")
        && gets_idx > 0
        && filtered[gets_idx + 1..] == ["more", "votes"]
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotes {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if let Some(gets_idx) = find_index(&filtered, |word| *word == "gets")
        && gets_idx > 0
        && filtered[gets_idx + 1..] == ["more", "votes", "or", "vote", "is", "tied"]
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotesOrTied {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if filtered.len() >= 4
        && filtered[0] == "no"
        && filtered[filtered.len() - 2..] == ["got", "votes"]
    {
        let filter_tokens = filtered[1..filtered.len() - 2]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::NoVoteObjectsMatched { filter });
    }

    if let Some(attacking_idx) = find_window_index(
        &filtered,
        &[
            "are",
            "attacking",
            "and",
            "you",
            "both",
            "own",
            "and",
            "control",
            "them",
        ],
    ) && let Some(and_idx) = find_meld_subject_split(&filtered[..attacking_idx])
    {
        let left_words = &filtered[..and_idx];
        let right_words = &filtered[and_idx + 1..attacking_idx];
        if !left_words.is_empty() && !right_words.is_empty() {
            let mut left_filter = parse_meld_subject_filter(left_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate subject (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            left_filter.controller = Some(PlayerFilter::You);
            left_filter.attacking = true;

            let mut right_filter = parse_meld_subject_filter(right_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate tail (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            right_filter.controller = Some(PlayerFilter::You);
            right_filter.attacking = true;

            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if filtered.len() >= 8
        && filtered[0] == "you"
        && filtered[1] == "both"
        && filtered[2] == "own"
        && filtered[3] == "and"
        && (filtered[4] == "control" || filtered[4] == "controls")
        && let Some(and_idx) = find_meld_subject_split(&filtered[5..])
    {
        let and_idx = 5 + and_idx;
        if and_idx > 5 && and_idx + 1 < filtered.len() {
            let mut left_filter =
                parse_meld_subject_filter(&filtered[5..and_idx]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate subject (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            left_filter.controller = Some(PlayerFilter::You);
            let mut right_filter =
                parse_meld_subject_filter(&filtered[and_idx + 1..]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            right_filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if let Some(and_idx) = find_index(&filtered, |word| *word == "and")
        && and_idx > 0
        && and_idx + 1 < filtered.len()
    {
        let right_first = filtered.get(and_idx + 1).copied();
        if matches!(right_first, Some("have") | Some("you")) {
            let left_words = &filtered[..and_idx];
            let mut right_words = filtered[and_idx + 1..].to_vec();
            if right_words.first().copied() == Some("have") {
                right_words.insert(0, "you");
            }
            let left_tokens = left_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let right_tokens = right_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let left = parse_predicate(&left_tokens)?;
            let right = parse_predicate(&right_tokens)?;
            return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
        }
    }

    if filtered.as_slice() == ["this", "tapped"]
        || filtered.as_slice() == ["thiss", "tapped"]
        || ((filtered.first().copied() == Some("this")
            || filtered.first().copied() == Some("thiss"))
            && filtered.last().copied() == Some("tapped"))
    {
        return Ok(PredicateAst::SourceIsTapped);
    }

    if filtered.as_slice() == ["this", "untapped"]
        || filtered.as_slice() == ["thiss", "untapped"]
        || filtered.as_slice() == ["this", "is", "untapped"]
        || filtered.as_slice() == ["this", "creature", "is", "untapped"]
        || filtered.as_slice() == ["this", "permanent", "is", "untapped"]
        || ((filtered.first().copied() == Some("this")
            || filtered.first().copied() == Some("thiss"))
            && filtered.last().copied() == Some("untapped"))
    {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)));
    }

    if filtered.as_slice() == ["this", "creature", "isnt", "saddled"]
        || filtered.as_slice() == ["this", "permanent", "isnt", "saddled"]
        || filtered.as_slice() == ["this", "isnt", "saddled"]
        || filtered.as_slice() == ["it", "isnt", "saddled"]
    {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)));
    }

    if filtered.as_slice() == ["this", "creature", "is", "saddled"]
        || filtered.as_slice() == ["this", "permanent", "is", "saddled"]
        || filtered.as_slice() == ["this", "is", "saddled"]
        || filtered.as_slice() == ["it", "is", "saddled"]
    {
        return Ok(PredicateAst::SourceIsSaddled);
    }

    if slice_starts_with(&filtered, &["there", "are", "no"])
        && slice_contains(&filtered, &"counters")
        && contains_window(&filtered, &["on", "this"])
        && let Some(counters_idx) = find_index(&filtered, |word| *word == "counters")
        && counters_idx >= 4
        && let Some(counter_type) = parse_counter_type_word(filtered[counters_idx - 1])
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let source_has_counter_prefix_len = if slice_starts_with(&raw_words, &["this", "has"]) {
        Some(2)
    } else if raw_words.len() >= 3
        && raw_words[0] == "this"
        && matches!(
            raw_words[1],
            "creature"
                | "permanent"
                | "artifact"
                | "enchantment"
                | "land"
                | "planeswalker"
                | "battle"
        )
        && raw_words[2] == "has"
    {
        Some(3)
    } else {
        None
    };
    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && raw_words[prefix_len] == "no"
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len + 1])
        && matches!(raw_words[prefix_len + 2], "counter" | "counters")
        && raw_words[prefix_len + 3] == "on"
        && matches!(
            raw_words.get(prefix_len + 4).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let triggering_object_had_no_counter_prefix_len =
        if slice_starts_with(&raw_words, &["it", "had", "no"]) {
            Some(3)
        } else if slice_starts_with(&raw_words, &["this", "creature", "had", "no"])
            || slice_starts_with(&raw_words, &["that", "creature", "had", "no"])
            || slice_starts_with(&raw_words, &["this", "permanent", "had", "no"])
            || slice_starts_with(&raw_words, &["that", "permanent", "had", "no"])
        {
            Some(4)
        } else {
            None
        };
    if let Some(prefix_len) = triggering_object_had_no_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len])
        && matches!(raw_words[prefix_len + 1], "counter" | "counters")
        && raw_words[prefix_len + 2] == "on"
        && matches!(
            raw_words[prefix_len + 3],
            "it" | "them" | "this" | "that" | "itself"
        )
    {
        return Ok(PredicateAst::TriggeringObjectHadNoCounter(counter_type));
    }

    if slice_starts_with(&raw_words, &["there", "are"])
        && raw_words.get(3).copied() == Some("or")
        && raw_words.get(4).copied() == Some("more")
        && raw_words
            .iter()
            .any(|w| *w == "counter" || *w == "counters")
    {
        if let Some((count, used)) = parse_number(&tokens[2..]) {
            let rest = &tokens[2 + used..];
            let rest_words = crate::cards::builders::parser::token_word_refs(rest);
            if rest_words.len() >= 4
                && rest_words[0] == "or"
                && rest_words[1] == "more"
                && (rest_words[3] == "counter" || rest_words[3] == "counters")
                && let Some(counter_type) = parse_counter_type_word(rest_words[2])
            {
                return Ok(PredicateAst::SourceHasCounterAtLeast {
                    counter_type,
                    count,
                });
            }
        }
    }

    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 6
        && let Some(count) = parse_named_number(raw_words[prefix_len])
        && raw_words[prefix_len + 1] == "or"
        && raw_words[prefix_len + 2] == "more"
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len + 3])
        && matches!(raw_words[prefix_len + 4], "counter" | "counters")
        && raw_words[prefix_len + 5] == "on"
        && matches!(
            raw_words.get(prefix_len + 6).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        });
    }

    if filtered.len() == 7
        && matches!(
            &filtered[..4],
            ["this", "creature", "power", "is"]
                | ["this", "creatures", "power", "is"]
                | ["this", "permanent", "power", "is"]
                | ["this", "permanents", "power", "is"]
        )
        && filtered[5] == "or"
        && filtered[6] == "more"
        && let Some(count_word) = filtered.get(4).copied()
        && let Some(count) = parse_named_number(count_word)
    {
        return Ok(PredicateAst::SourcePowerAtLeast(count));
    }

    if filtered.len() >= 10 && filtered[0] == "there" && filtered[1] == "are" {
        let mut idx = 2usize;
        if let Some(count) = parse_named_number(filtered[idx]) {
            idx += 1;
            if filtered.get(idx).copied() == Some("or")
                && filtered.get(idx + 1).copied() == Some("more")
            {
                idx += 2;
            }
            let looks_like_basic_land_type_clause = filtered.get(idx).copied() == Some("basic")
                && filtered.get(idx + 1).copied() == Some("land")
                && matches!(filtered.get(idx + 2).copied(), Some("type" | "types"))
                && filtered.get(idx + 3).copied() == Some("among")
                && matches!(filtered.get(idx + 4).copied(), Some("land" | "lands"));
            if looks_like_basic_land_type_clause {
                let tail = &filtered[idx + 5..];
                let player = if tail == ["that", "player", "controls"]
                    || tail == ["that", "player", "control"]
                    || tail == ["that", "players", "controls"]
                {
                    PlayerAst::That
                } else if tail == ["you", "control"] || tail == ["you", "controls"] {
                    PlayerAst::You
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported basic-land-types predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    )));
                };

                return Ok(PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player,
                    count,
                });
            }
        }
    }

    let parse_graveyard_card_types_subject = |words: &[&str]| -> Option<PlayerAst> {
        match words {
            [first, second] if *first == "your" && *second == "graveyard" => Some(PlayerAst::You),
            [first, second, third]
                if *first == "that"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::That)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::Target)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "opponent" || *second == "opponents")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::TargetOpponent)
            }
            [first, second]
                if (*first == "opponent" || *first == "opponents") && *second == "graveyard" =>
            {
                Some(PlayerAst::Opponent)
            }
            _ => None,
        }
    };
    if filtered.len() >= 11 {
        let (count_idx, subject_start, constrained_player) =
            if filtered[0] == "there" && filtered[1] == "are" {
                (2usize, 10usize, None)
            } else if filtered[0] == "you" && filtered[1] == "have" {
                (2usize, 10usize, Some(PlayerAst::You))
            } else {
                (usize::MAX, usize::MAX, None)
            };
        if count_idx != usize::MAX
            && filtered.get(count_idx + 1).copied() == Some("or")
            && filtered.get(count_idx + 2).copied() == Some("more")
            && filtered.get(count_idx + 3).copied() == Some("card")
            && matches!(filtered.get(count_idx + 4).copied(), Some("type" | "types"))
            && filtered.get(count_idx + 5).copied() == Some("among")
            && matches!(filtered.get(count_idx + 6).copied(), Some("card" | "cards"))
            && filtered.get(count_idx + 7).copied() == Some("in")
            && subject_start <= filtered.len()
            && let Some(count) = parse_named_number(filtered[count_idx])
            && let Some(player) = parse_graveyard_card_types_subject(&filtered[subject_start..])
            && constrained_player.map_or(true, |expected| expected == player)
        {
            return Ok(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count });
        }
    }

    let parse_comparison_player_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            [first, second, ..] if *first == "that" && *second == "player" => {
                Some((PlayerAst::That, 2))
            }
            [first, second, ..] if *first == "target" && *second == "player" => {
                Some((PlayerAst::Target, 2))
            }
            [first, second, ..] if *first == "target" && *second == "opponent" => {
                Some((PlayerAst::TargetOpponent, 2))
            }
            [first, second, ..] if *first == "each" && *second == "opponent" => {
                Some((PlayerAst::Opponent, 2))
            }
            [first, second, ..] if *first == "defending" && *second == "player" => {
                Some((PlayerAst::Defending, 2))
            }
            [first, second, ..] if *first == "attacking" && *second == "player" => {
                Some((PlayerAst::Attacking, 2))
            }
            [first, ..] if *first == "you" => Some((PlayerAst::You, 1)),
            [first, ..] if *first == "opponent" || *first == "opponents" => {
                Some((PlayerAst::Opponent, 1))
            }
            [first, second, ..] if *first == "player" && *second == "who" => {
                Some((PlayerAst::That, 1))
            }
            _ => None,
        }
    };
    let parse_life_total_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            ["your", "life", "total", ..] => Some((PlayerAst::You, 3)),
            ["their", "life", "total", ..] => Some((PlayerAst::That, 3)),
            ["that", "players", "life", "total", ..] => Some((PlayerAst::That, 4)),
            ["target", "players", "life", "total", ..] => Some((PlayerAst::Target, 4)),
            ["target", "opponents", "life", "total", ..] => Some((PlayerAst::TargetOpponent, 4)),
            ["opponents", "life", "total", ..] | ["opponent", "life", "total", ..] => {
                Some((PlayerAst::Opponent, 3))
            }
            ["defending", "players", "life", "total", ..] => Some((PlayerAst::Defending, 4)),
            ["attacking", "players", "life", "total", ..] => Some((PlayerAst::Attacking, 4)),
            _ => None,
        }
    };
    let half_starting_tail_matches = |tail: &[&str]| {
        matches!(
            tail,
            ["half", "your", "starting", "life", "total"]
                | ["half", "their", "starting", "life", "total"]
                | ["half", "that", "players", "starting", "life", "total"]
                | ["half", "target", "players", "starting", "life", "total"]
                | ["half", "target", "opponents", "starting", "life", "total"]
                | ["half", "opponents", "starting", "life", "total"]
                | ["half", "defending", "players", "starting", "life", "total"]
                | ["half", "attacking", "players", "starting", "life", "total"]
        )
    };
    if let Some((player, subject_len)) = parse_life_total_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("is")
    {
        let tail = &filtered[subject_len + 1..];
        if let Some(rest) = slice_strip_prefix(tail, &["less", "than", "or", "equal", "to"])
            && half_starting_tail_matches(rest)
        {
            return Ok(PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player });
        }
        if let Some(rest) = slice_strip_prefix(tail, &["less", "than"])
            && half_starting_tail_matches(rest)
        {
            return Ok(PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player });
        }
    }
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && matches!(
            filtered.get(subject_len).copied(),
            Some("control" | "controls")
        )
        && filtered.get(subject_len + 1).copied() == Some("more")
        && let Some(than_offset) = find_index(&filtered[subject_len + 2..], |word| *word == "than")
    {
        let than_idx = subject_len + 2 + than_offset;
        let tail = &filtered[than_idx..];
        if matches!(tail, ["than", "you"] | ["than", "you", "do"]) {
            let filter_tokens = filtered[subject_len + 2..than_idx]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if !filter_tokens.is_empty() {
                let other = filter_tokens
                    .first()
                    .is_some_and(|token| token.is_word("another") || token.is_word("other"));
                if let Ok(filter) = parse_object_filter(&filter_tokens, other)
                    && filter != ObjectFilter::default()
                {
                    return Ok(PredicateAst::PlayerControlsMoreThanYou { player, filter });
                }
            }
        }
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "life", "than", "you"] | ["more", "life", "than", "you", "do"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreLifeThanYou { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "card", "in", "hand", "than", "you"]
                | ["more", "cards", "in", "hand", "than", "you"]
                | ["more", "card", "in", "their", "hand", "than", "you"]
                | ["more", "cards", "in", "their", "hand", "than", "you"]
                | ["more", "card", "in", "hand", "than", "you", "do"]
                | ["more", "cards", "in", "hand", "than", "you", "do"]
                | ["more", "card", "in", "their", "hand", "than", "you", "do"]
                | ["more", "cards", "in", "their", "hand", "than", "you", "do"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreCardsInHandThanYou { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && let Some(count_word) = filtered.get(subject_len + 1).copied()
        && let Some(count) = parse_named_number(count_word)
        && filtered.get(subject_len + 2).copied() == Some("or")
        && let Some(comp_word) = filtered.get(subject_len + 3).copied()
        && matches!(comp_word, "more" | "fewer" | "less")
        && matches!(
            filtered.get(subject_len + 4).copied(),
            Some("card" | "cards")
        )
        && filtered.get(subject_len + 5).copied() == Some("in")
        && filtered.get(subject_len + 6).copied() == Some("hand")
        && filtered.len() == subject_len + 7
    {
        return Ok(if comp_word == "more" {
            PredicateAst::PlayerCardsInHandOrMore { player, count }
        } else {
            PredicateAst::PlayerCardsInHandOrFewer { player, count }
        });
    }

    if filtered.as_slice() == ["you", "have", "no", "cards", "in", "hand"] {
        return Ok(PredicateAst::YouHaveNoCardsInHand);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "your", "turn"] | ["its", "your", "turn"] | ["your", "turn"]
    ) {
        return Ok(PredicateAst::YourTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["creature", "died", "this", "turn"] | ["creatures", "died", "this", "turn"]
    ) {
        return Ok(PredicateAst::CreatureDiedThisTurn);
    }

    if filtered.len() == 7
        && let Some(count) = parse_named_number(filtered[0])
        && filtered[1..] == ["or", "more", "creatures", "died", "this", "turn"]
    {
        return Ok(PredicateAst::CreatureDiedThisTurnOrMore(count));
    }

    if matches!(
        filtered.as_slice(),
        [
            "permanent",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "permanents",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        [
            "you",
            "had",
            "land",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "land",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
            player: PlayerAst::You,
        });
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "gained"
        && let Some((count, used)) = parse_number(&tokens[2..])
        && filtered[2 + used..] == ["or", "more", "life", "this", "turn"]
    {
        return Ok(PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: count as u32,
        });
    }

    if filtered.as_slice() == ["you", "gained", "life", "this", "turn"] {
        return Ok(PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: 1,
        });
    }

    if filtered.as_slice() == ["you", "attacked", "this", "turn"] {
        return Ok(PredicateAst::YouAttackedThisTurn);
    }

    if filtered.len() == 9
        && filtered[0] == "you"
        && filtered[1] == "attacked"
        && filtered[2] == "with"
        && filtered[3] == "exactly"
        && matches!(filtered[5], "other" | "others")
        && matches!(filtered[6], "creature" | "creatures")
        && filtered[7] == "this"
        && filtered[8] == "combat"
        && let Some(count) = parse_named_number(filtered[4])
    {
        return Ok(PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count));
    }

    if matches!(
        filtered.as_slice(),
        [
            "this", "creature", "attacked", "or", "blocked", "this", "turn"
        ] | [
            "this",
            "permanent",
            "attacked",
            "or",
            "blocked",
            "this",
            "turn"
        ] | ["this", "attacked", "or", "blocked", "this", "turn"]
            | ["it", "attacked", "or", "blocked", "this", "turn"]
    ) {
        return Ok(PredicateAst::SourceAttackedOrBlockedThisTurn);
    }

    if filtered.as_slice() == ["you", "cast", "it"]
        || filtered.as_slice() == ["you", "cast", "this", "spell"]
    {
        return Ok(PredicateAst::SourceWasCast);
    }

    if filtered.len() >= 6
        && filtered[0] == "this"
        && filtered[1] == "spell"
        && filtered[2] == "was"
        && filtered[3] == "cast"
        && filtered[4] == "from"
    {
        let zone_words = &filtered[5..];
        let zone = if zone_words.len() == 1 {
            parse_zone_word(zone_words[0])
        } else if zone_words.len() == 2 && is_article(zone_words[0]) {
            parse_zone_word(zone_words[1])
        } else if zone_words.len() == 2 && zone_words[0] == "the" {
            parse_zone_word(zone_words[1])
        } else {
            None
        };

        if let Some(zone) = zone {
            return Ok(PredicateAst::ThisSpellWasCastFromZone(zone));
        }
    }

    if filtered.as_slice() == ["no", "spells", "were", "cast", "last", "turn"]
        || filtered.as_slice() == ["no", "spell", "was", "cast", "last", "turn"]
    {
        return Ok(PredicateAst::NoSpellsWereCastLastTurn);
    }
    if filtered.as_slice() == ["this", "spell", "was", "kicked"] {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if filtered.as_slice() == ["this", "spell", "was", "bargained"]
        || filtered.as_slice() == ["it", "was", "bargained"]
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Bargain".to_string()));
    }
    if filtered.len() == 4
        && matches!(filtered[0], "a" | "an")
        && parse_subtype_word(filtered[1]).is_some()
        && matches!(filtered[2], "was" | "were")
        && filtered[3] == "beheld"
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.len() == 3
        && parse_subtype_word(filtered[0]).is_some()
        && matches!(filtered[1], "was" | "were")
        && filtered[2] == "beheld"
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.as_slice() == ["gift", "was", "promised"] {
        return Ok(PredicateAst::ThisSpellPaidLabel("Gift".to_string()));
    }
    if filtered.len() == 6
        && filtered[0] == "this"
        && matches!(
            filtered[1],
            "spell's" | "card's" | "creature's" | "permanent's"
        )
        && filtered[3] == "cost"
        && filtered[4] == "was"
        && filtered[5] == "paid"
    {
        let mut chars = filtered[2].chars();
        let Some(first) = chars.next() else {
            return Err(CardTextError::ParseError(
                "missing paid-cost label in predicate".to_string(),
            ));
        };
        let label = format!(
            "{}{}",
            first.to_ascii_uppercase(),
            chars.as_str().to_ascii_lowercase()
        );
        return Ok(PredicateAst::ThisSpellPaidLabel(label));
    }
    if filtered.as_slice() == ["it", "was", "kicked"]
        || filtered.as_slice() == ["that", "was", "kicked"]
    {
        return Ok(PredicateAst::TargetWasKicked);
    }

    if filtered.as_slice() == ["you", "have", "full", "party"] {
        return Ok(PredicateAst::YouHaveFullParty);
    }
    if filtered.as_slice() == ["its", "controller", "poisoned"]
        || filtered.as_slice() == ["that", "spells", "controller", "poisoned"]
    {
        return Ok(PredicateAst::TargetSpellControllerIsPoisoned);
    }
    if filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "that", "spell"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "that", "spell"]
    {
        return Ok(PredicateAst::TargetSpellNoManaSpentToCast);
    }
    if filtered.as_slice()
        == [
            "you",
            "control",
            "more",
            "creatures",
            "than",
            "that",
            "spells",
            "controller",
        ]
        || filtered.as_slice()
            == [
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "its",
                "controller",
            ]
    {
        return Ok(PredicateAst::YouControlMoreCreaturesThanTargetSpellController);
    }
    if filtered.len() == 7
        && matches!(filtered[0], "w" | "u" | "b" | "r" | "g" | "c")
        && filtered[1] == "was"
        && filtered[2] == "spent"
        && filtered[3] == "to"
        && filtered[4] == "cast"
        && filtered[5] == "this"
        && filtered[6] == "spell"
        && let Ok(symbol) = parse_mana_symbol(filtered[0])
    {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    }

    if let Some((amount, symbol)) = parse_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol });
    }

    if filtered.len() >= 5
        && matches!(
            filtered.as_slice(),
            ["this", "permanent", "attached", "to", ..]
                | ["that", "permanent", "attached", "to", ..]
                | ["this", "permanent", "is", "attached", "to", ..]
                | ["that", "permanent", "is", "attached", "to", ..]
        )
    {
        let attached_start = if filtered.get(2).copied() == Some("is") {
            5
        } else {
            4
        };
        let attached_tokens = filtered[attached_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&attached_tokens, false)?;
        if filter.card_types.is_empty() {
            filter.card_types.push(CardType::Creature);
        }
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from("enchanted"),
            filter,
        ));
    }

    if filtered.len() >= 4 && filtered[0] == "sacrificed" && filtered[2] == "was" {
        let sacrificed_head = filtered[1];
        let subject_card_type =
            parse_card_type(sacrificed_head).filter(|card_type| is_permanent_type(*card_type));
        let subject_is_permanent = sacrificed_head == "permanent" || subject_card_type.is_some();

        if subject_is_permanent {
            let descriptor_tokens = filtered[3..]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&descriptor_tokens, false)?;
            if filter.card_types.is_empty() {
                if let Some(card_type) = subject_card_type {
                    filter.card_types.push(card_type);
                }
            }
            if filter.zone.is_none() && sacrificed_head == "permanent" {
                filter.zone = Some(Zone::Battlefield);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
    }

    if filtered.as_slice()
        == [
            "this", "is", "fourth", "time", "this", "ability", "has", "resolved", "this", "turn",
        ]
    {
        return Ok(PredicateAst::Unmodeled(filtered.join(" ")));
    }

    if matches!(
        filtered.as_slice(),
        ["any", "of", "those", "cards", "remain", "exiled"]
            | ["those", "cards", "remain", "exiled"]
            | ["that", "card", "remains", "exiled"]
            | ["it", "remains", "exiled"]
    ) {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter::default().in_zone(Zone::Exile),
        ));
    }

    if filtered[0] == "its" {
        filtered[0] = "it";
    }
    if filtered.len() >= 2 && filtered[0] == "it" && filtered[1] == "s" {
        filtered.remove(1);
    }

    let demonstrative_reference_len = if filtered.first().copied() == Some("it") {
        Some(1usize)
    } else if filtered.len() >= 2
        && filtered[0] == "that"
        && matches!(
            filtered[1],
            "artifact"
                | "card"
                | "creature"
                | "land"
                | "object"
                | "permanent"
                | "source"
                | "spell"
                | "token"
        )
    {
        Some(2usize)
    } else {
        None
    };

    let is_it_soulbond_paired = matches!(
        filtered.as_slice(),
        ["it", "paired", "with", "creature"]
            | ["it", "paired", "with", "another", "creature"]
            | ["it", "s", "paired", "with", "creature"]
            | ["it", "s", "paired", "with", "another", "creature"]
    );
    if is_it_soulbond_paired {
        return Ok(PredicateAst::ItIsSoulbondPaired);
    }

    if filtered.len() >= 2 {
        let tag = if slice_starts_with(&filtered, &["equipped", "creature"]) {
            Some("equipped")
        } else if slice_starts_with(&filtered, &["enchanted", "creature"]) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = remainder
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let onto_battlefield_idx = find_window_index(&filtered, &["onto", "battlefield"])
        .or_else(|| find_window_index(&filtered, &["onto", "the", "battlefield"]));
    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "put"
        && slice_ends_with(&filtered, &["this", "way"])
        && let Some(onto_idx) = onto_battlefield_idx
    {
        let filter_words = &filtered[2..onto_idx];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    let is_it = demonstrative_reference_len == Some(1);
    let has_card = demonstrative_reference_len
        .map(|reference_len| slice_contains(&filtered[reference_len..], &"card"))
        .unwrap_or(false);

    if is_it {
        if filtered
            .get(1)
            .is_some_and(|word| *word == "has" || *word == "have")
        {
            filtered.remove(1);
        }
        if filtered.len() >= 3 && filtered[1] == "mana" && filtered[2] == "value" {
            let mana_value_tail = if filtered
                .get(3)
                .is_some_and(|word| matches!(*word, "is" | "are" | "was" | "were"))
            {
                &filtered[4..]
            } else {
                &filtered[3..]
            };
            let compares_to_colors_spent = mana_value_tail
                == [
                    "less", "than", "or", "equal", "to", "number", "of", "colors", "of", "mana",
                    "spent", "to", "cast", "this", "spell",
                ]
                || mana_value_tail
                    == [
                        "less", "than", "or", "equal", "to", "number", "of", "color", "of", "mana",
                        "spent", "to", "cast", "this", "spell",
                    ];
            if compares_to_colors_spent {
                return Ok(PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell);
            }

            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("mana value", mana_value_tail, &filtered)?
            {
                return Ok(PredicateAst::ItMatches(ObjectFilter {
                    mana_value: Some(cmp),
                    ..Default::default()
                }));
            }
        }

        if filtered.len() >= 3 && (filtered[1] == "power" || filtered[1] == "toughness") {
            let axis = filtered[1];
            let value_tail = &filtered[2..];
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if axis == "power" {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if let Some(reference_len) = demonstrative_reference_len {
        let mut descriptor_words = filtered[reference_len..].to_vec();
        if descriptor_words.as_slice() == ["has", "toxic"]
            || descriptor_words.as_slice() == ["have", "toxic"]
        {
            let mut filter = ObjectFilter::default().with_ability_marker("toxic");
            if filtered.get(1).copied() == Some("creature") {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
        if descriptor_words
            .first()
            .is_some_and(|word| matches!(*word, "is" | "are"))
        {
            descriptor_words.remove(0);
        }
        if slice_starts_with(&descriptor_words, &["not", "token"]) {
            descriptor_words.drain(0..2);
            descriptor_words.insert(0, "nontoken");
        }
        if !descriptor_words.is_empty() {
            let descriptor_tokens = descriptor_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let Ok(filter) = parse_object_filter_lexed(&descriptor_tokens, false)
                && filter != ObjectFilter::default()
            {
                if has_card
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    return Ok(PredicateAst::ItIsLandCard);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && (filtered[2] == "no" || filtered[2] == "neither")
    {
        let control_tokens = filtered[3..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(mut filter) = parse_object_filter(&control_tokens, false) {
            filter.controller = Some(PlayerFilter::You);
            if filtered[2] == "neither" {
                filter = filter
                    .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
            }
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    let you_dont_control_filter_start = if filtered.len() >= 4
        && filtered[0] == "you"
        && matches!(filtered[1], "dont" | "don't")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        Some(3usize)
    } else if filtered.len() >= 5
        && filtered[0] == "you"
        && filtered[1] == "do"
        && filtered[2] == "not"
        && (filtered[3] == "control" || filtered[3] == "controls")
    {
        Some(4usize)
    } else {
        None
    };
    if let Some(filter_start) = you_dont_control_filter_start {
        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && let Some(or_idx) = find_index(&filtered, |word| *word == "or")
        && or_idx > 2
    {
        let left_tokens = filtered[2..or_idx]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut right_words = filtered[or_idx + 1..].to_vec();
        if right_words.first().copied() == Some("there") {
            right_words = right_words[1..].to_vec();
        }
        if slice_contains(&right_words, &"graveyard") && slice_contains(&right_words, &"your") {
            let right_tokens = right_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let (Ok(mut control_filter), Ok(mut graveyard_filter)) = (
                parse_object_filter(&left_tokens, false),
                parse_object_filter(&right_tokens, false),
            ) {
                control_filter.controller = Some(PlayerFilter::You);
                if graveyard_filter.zone.is_none() {
                    graveyard_filter.zone = Some(Zone::Graveyard);
                }
                if graveyard_filter.owner.is_none() {
                    graveyard_filter.owner = Some(PlayerFilter::You);
                }
                return Ok(PredicateAst::PlayerControlsOrHasCardInGraveyard {
                    player: PlayerAst::You,
                    control_filter,
                    graveyard_filter,
                });
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
    {
        let mut filter_start = 2usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(2)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && filtered.get(3).copied() == Some("or")
            && filtered.get(4).copied() == Some("more")
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        } else if filtered.get(2).copied() == Some("exactly")
            && let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 4;
        } else if filtered.get(2).copied() == Some("at")
            && filtered.get(3).copied() == Some("least")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        }

        let mut control_words = filtered[filter_start..].to_vec();
        let mut requires_different_powers = false;
        if slice_ends_with(&control_words, &["with", "different", "powers"])
            || slice_ends_with(&control_words, &["with", "different", "power"])
        {
            requires_different_powers = true;
            control_words.truncate(control_words.len().saturating_sub(3));
        }
        let control_tokens = control_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                if requires_different_powers {
                    return Ok(PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                        player: PlayerAst::You,
                        filter,
                        count,
                    });
                }
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 4
        && filtered[0] == "that"
        && (filtered[1] == "player" || filtered[1] == "players")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        let mut filter_start = 3usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && filtered.get(4).copied() == Some("or")
            && filtered.get(5).copied() == Some("more")
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        } else if filtered.get(3).copied() == Some("exactly")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 5;
        } else if filtered.get(3).copied() == Some("at")
            && filtered.get(4).copied() == Some("least")
            && let Some(raw_count) = filtered.get(5)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        }

        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(filter) = parse_object_filter(&control_tokens, other) {
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            });
        }
    }

    if filtered.as_slice() == ["you", "controlled", "that", "permanent"]
        || filtered.as_slice() == ["you", "control", "that", "permanent"]
    {
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default(),
        });
    }

    if filtered.as_slice() == ["it", "entered", "under", "your", "control"]
        || filtered.as_slice() == ["that", "card", "entered", "under", "your", "control"]
        || filtered.as_slice() == ["that", "permanent", "entered", "under", "your", "control"]
    {
        return Ok(PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        });
    }

    if filtered.len() >= 8
        && filtered[0] == "you"
        && filtered[1] == "put"
        && slice_ends_with(&filtered, &["onto", "the", "battlefield", "this", "way"])
    {
        let filter_words = &filtered[2..filtered.len() - 5];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    if filtered.as_slice() == ["it", "wasnt", "blocking"]
        || filtered.as_slice() == ["it", "was", "not", "blocking"]
        || filtered.as_slice() == ["that", "creature", "wasnt", "blocking"]
    {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter {
                nonblocking: true,
                ..Default::default()
            },
        ));
    }

    if filtered.as_slice() == ["no", "creatures", "are", "on", "battlefield"] {
        return Ok(PredicateAst::PlayerControlsNo {
            player: PlayerAst::Any,
            filter: ObjectFilter::creature(),
        });
    }

    if filtered.as_slice() == ["you", "have", "citys", "blessing"]
        || filtered.as_slice() == ["you", "have", "city", "blessing"]
        || slice_starts_with(
            &filtered,
            &["you", "have", "citys", "blessing", "for", "each"],
        )
        || slice_starts_with(
            &filtered,
            &["you", "have", "city", "blessing", "for", "each"],
        )
    {
        return Ok(PredicateAst::PlayerHasCitysBlessing {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["youre", "the", "monarch"]
        || filtered.as_slice() == ["youre", "monarch"]
        || filtered.as_slice() == ["you", "are", "the", "monarch"]
        || filtered.as_slice() == ["you", "are", "monarch"]
    {
        return Ok(PredicateAst::PlayerIsMonarch {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["you", "have", "the", "initiative"]
        || filtered.as_slice() == ["you", "have", "initiative"]
    {
        return Ok(PredicateAst::PlayerHasInitiative {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["youve", "completed", "a", "dungeon"]
        || filtered.as_slice() == ["you", "have", "completed", "a", "dungeon"]
    {
        return Ok(PredicateAst::PlayerCompletedDungeon {
            player: PlayerAst::You,
            dungeon_name: None,
        });
    }

    if (slice_starts_with(&filtered, &["youve", "completed"]) && filtered.len() > 2)
        || (slice_starts_with(&filtered, &["you", "have", "completed"]) && filtered.len() > 3)
    {
        let name_start = if filtered[1] == "have" { 3 } else { 2 };
        let dungeon_name = filtered[name_start..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(PredicateAst::PlayerCompletedDungeon {
            player: PlayerAst::You,
            dungeon_name: Some(dungeon_name),
        });
    }

    if (slice_starts_with(&filtered, &["you", "havent", "completed"]) && filtered.len() > 3)
        || (slice_starts_with(&filtered, &["you", "have", "not", "completed"])
            && filtered.len() > 4)
    {
        let name_start = if filtered[1] == "have" { 4 } else { 3 };
        let dungeon_name = filtered[name_start..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::PlayerCompletedDungeon {
                player: PlayerAst::You,
                dungeon_name: Some(dungeon_name),
            },
        )));
    }

    if filtered.as_slice() == ["youve", "cast", "another", "spell", "this", "turn"]
        || filtered.as_slice() == ["you", "have", "cast", "another", "spell", "this", "turn"]
        || filtered.as_slice() == ["you", "cast", "another", "spell", "this", "turn"]
    {
        return Ok(PredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: PlayerAst::You,
            count: 2,
        });
    }

    let spell_cast_prefix = if slice_starts_with(&filtered, &["opponent", "has", "cast"]) {
        Some((3usize, PlayerFilter::Opponent))
    } else if slice_starts_with(&filtered, &["opponents", "have", "cast"]) {
        Some((3usize, PlayerFilter::Opponent))
    } else if slice_starts_with(&filtered, &["youve", "cast"]) {
        Some((2usize, PlayerFilter::You))
    } else if slice_starts_with(&filtered, &["you", "have", "cast"]) {
        Some((3usize, PlayerFilter::You))
    } else if slice_starts_with(&filtered, &["you", "cast"]) {
        Some((2usize, PlayerFilter::You))
    } else {
        None
    };
    if let Some((prefix_len, player)) = spell_cast_prefix
        && filtered.len() > prefix_len + 2
        && filtered[filtered.len() - 2..] == ["this", "turn"]
    {
        let filter_words = &filtered[prefix_len..filtered.len() - 2];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(filter) = parse_object_filter_lexed(&filter_tokens, false) {
            return Ok(PredicateAst::ValueComparison {
                left: Value::SpellsCastThisTurnMatching {
                    player,
                    filter,
                    exclude_source: false,
                },
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            });
        }
    }

    if filtered.len() == 5
        && filtered[0] == "x"
        && filtered[1] == "is"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        if let Some(amount) = filtered[2]
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(filtered[2]).map(|n| n as i32))
        {
            return Ok(PredicateAst::ValueComparison {
                left: Value::X,
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(amount),
            });
        }
    }

    let unsupported_unmodeled = filtered.as_slice() == ["you", "gained", "life", "this", "turn"]
        || filtered.as_slice() == ["you", "dont", "cast", "it"]
        || filtered.as_slice() == ["it", "has", "odd", "number", "of", "counters", "on", "it"]
        || filtered.as_slice() == ["it", "has", "even", "number", "of", "counters", "on", "it"]
        || filtered.as_slice() == ["opponent", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["opponents", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["an", "opponent", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["this", "card", "in", "your", "graveyard"]
        || filtered.as_slice() == ["this", "artifact", "untapped"]
        || filtered.as_slice() == ["this", "has", "luck", "counter", "on", "it"]
        || filtered.as_slice() == ["it", "had", "revival", "counter", "on", "it"]
        || filtered.as_slice() == ["that", "creature", "would", "die", "this", "turn"]
        || filtered.as_slice()
            == [
                "this", "second", "time", "this", "ability", "has", "resolved", "this", "turn",
            ]
        || filtered.as_slice()
            == [
                "this", "fourth", "time", "this", "ability", "has", "resolved", "this", "turn",
            ]
        || filtered.as_slice()
            == [
                "this",
                "fourth",
                "time",
                "this",
                "ability",
                "has",
                "triggered",
                "this",
                "turn",
            ]
        || filtered.as_slice()
            == [
                "this",
                "ability",
                "has",
                "been",
                "activated",
                "four",
                "or",
                "more",
                "times",
                "this",
                "turn",
            ]
        || filtered.as_slice() == ["it", "first", "combat", "phase", "of", "turn"]
        || filtered.as_slice()
            == [
                "you", "would", "begin", "your", "turn", "while", "this", "artifact", "is",
                "tapped",
            ]
        || filtered.as_slice() == ["player", "is", "dealt", "damage", "this", "way"]
        || filtered.as_slice()
            == [
                "two",
                "or",
                "more",
                "creatures",
                "are",
                "tied",
                "for",
                "least",
                "power",
            ]
        || filtered.as_slice()
            == [
                "card",
                "would",
                "be",
                "put",
                "into",
                "opponents",
                "graveyard",
                "from",
                "anywhere",
            ]
        || filtered.as_slice() == ["the", "number", "is", "odd"]
        || filtered.as_slice() == ["the", "number", "is", "even"]
        || filtered.as_slice() == ["number", "is", "odd"]
        || filtered.as_slice() == ["number", "is", "even"]
        || filtered.as_slice() == ["the", "number", "of", "permanents", "is", "odd"]
        || filtered.as_slice() == ["the", "number", "of", "permanents", "is", "even"]
        || filtered.as_slice() == ["number", "of", "permanents", "is", "odd"]
        || filtered.as_slice() == ["number", "of", "permanents", "is", "even"];
    if unsupported_unmodeled {
        return Ok(PredicateAst::Unmodeled(filtered.join(" ")));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}

fn parse_graveyard_threshold_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let (count, tail_start, constrained_player) = if filtered.len() >= 5
        && filtered[0] == "there"
        && filtered[1] == "are"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Ok(None);
        };
        (count, 5usize, None)
    } else if filtered.len() >= 5
        && filtered[0] == "you"
        && filtered[1] == "have"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Ok(None);
        };
        (count, 5usize, Some(PlayerAst::You))
    } else {
        return Ok(None);
    };

    let tail = &filtered[tail_start..];
    let Some(in_idx) = rfind_index(tail, |word| *word == "in") else {
        return Ok(None);
    };
    if in_idx == 0 || in_idx + 1 >= tail.len() {
        return Ok(None);
    }

    let graveyard_owner_words = &tail[in_idx + 1..];
    let player = match graveyard_owner_words {
        ["your", "graveyard"] => PlayerAst::You,
        ["that", "player", "graveyard"] | ["that", "players", "graveyard"] => PlayerAst::That,
        ["target", "player", "graveyard"] | ["target", "players", "graveyard"] => PlayerAst::Target,
        ["target", "opponent", "graveyard"] | ["target", "opponents", "graveyard"] => {
            PlayerAst::TargetOpponent
        }
        ["opponent", "graveyard"] | ["opponents", "graveyard"] => PlayerAst::Opponent,
        _ => return Ok(None),
    };
    if constrained_player.is_some_and(|expected| expected != player) {
        return Ok(None);
    }

    let raw_filter_words = &tail[..in_idx];
    if raw_filter_words.is_empty()
        || slice_contains(raw_filter_words, &"type")
        || slice_contains(raw_filter_words, &"types")
    {
        return Ok(None);
    }

    let mut normalized_filter_words = Vec::with_capacity(raw_filter_words.len());
    for (idx, word) in raw_filter_words.iter().enumerate() {
        if *word == "and"
            && raw_filter_words
                .get(idx + 1)
                .is_some_and(|next| *next == "or")
        {
            continue;
        }
        normalized_filter_words.push(*word);
    }
    if normalized_filter_words.is_empty() {
        return Ok(None);
    }

    let mut filter = if matches!(normalized_filter_words.as_slice(), ["card"] | ["cards"]) {
        ObjectFilter::default()
    } else {
        let filter_tokens = normalized_filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let Ok(filter) = parse_object_filter(&filter_tokens, false) else {
            return Ok(None);
        };
        filter
    };
    filter.zone = Some(Zone::Graveyard);

    Ok(Some(PredicateAst::PlayerControlsAtLeast {
        player,
        filter,
        count,
    }))
}

fn parse_mana_spent_to_cast_predicate(words: &[&str]) -> Option<(u32, Option<ManaSymbol>)> {
    if words.len() < 10 || words[0] != "at" || words[1] != "least" {
        return None;
    }

    let amount_tokens = vec![OwnedLexToken::word(
        words[2].to_string(),
        TextSpan::synthetic(),
    )];
    let (amount, _) = parse_number(&amount_tokens)?;

    let mut idx = 3;
    if words.get(idx).copied() == Some("of") {
        idx += 1;
    }

    let symbol = if let Some(word) = words.get(idx).copied() {
        if let Some(parsed) = parse_mana_symbol_word(word) {
            idx += 1;
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    let tail = &words[idx..];
    let canonical_tail = ["mana", "was", "spent", "to", "cast", "this", "spell"];
    let plural_tail = ["mana", "were", "spent", "to", "cast", "this", "spell"];
    if tail == canonical_tail || tail == plural_tail {
        return Some((amount, symbol));
    }

    None
}

fn parse_mana_symbol_word(word: &str) -> Option<ManaSymbol> {
    parse_mana_symbol_word_flexible(word)
}
