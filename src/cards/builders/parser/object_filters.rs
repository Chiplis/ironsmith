use crate::cards::builders::{CardTextError, IT_TAG};
use crate::{
    CardType, Color, ColorSet, ObjectFilter, PlayerFilter, Supertype, TagKey,
    TaggedObjectConstraint, TaggedOpbjectRelation, Zone,
};

use super::effect_sentences::{parse_subtype_word, parse_supertype_word};
use super::keyword_static::parse_pt_modifier;
use super::lexer::OwnedLexToken;
use super::native_tokens::LowercaseWordView;
use super::util::{
    apply_filter_keyword_constraint, is_article, is_demonstrative_object_head, is_non_outlaw_word,
    is_outlaw_word, is_permanent_type, is_source_reference_words, parse_alternative_cast_words,
    parse_card_type, parse_color, parse_counter_type_word, parse_filter_counter_constraint_words,
    parse_filter_keyword_constraint_words, parse_non_color, parse_non_subtype, parse_non_supertype,
    parse_non_type, parse_subtype_flexible, parse_unsigned_pt_word, parse_zone_word,
    push_outlaw_subtypes, split_on_and, trim_commas, words,
};
use super::value_helpers::parse_filter_comparison_tokens;

fn normalized_token_index_for_word_index(
    tokens: &[OwnedLexToken],
    word_idx: usize,
) -> Option<usize> {
    LowercaseWordView::new(tokens).token_index_for_word_index(word_idx)
}

fn normalized_token_index_after_words(
    tokens: &[OwnedLexToken],
    word_count: usize,
) -> Option<usize> {
    LowercaseWordView::new(tokens).token_index_after_words(word_count)
}

fn lower_words_end_with(words: &[&str], suffix: &[&str]) -> bool {
    words.len() >= suffix.len() && words[words.len() - suffix.len()..] == *suffix
}

fn parse_simple_object_filter_lexed(tokens: &[OwnedLexToken], other: bool) -> Option<ObjectFilter> {
    let word_view = LowercaseWordView::new(tokens);
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

    let apply_owner_zone_suffix = |filter: &mut ObjectFilter,
                                   owner: PlayerFilter,
                                   zone: Zone,
                                   words: &mut Vec<&str>,
                                   suffix_len: usize| {
        filter.owner = Some(owner);
        filter.zone = Some(zone);
        trim_suffix(words, suffix_len);
    };

    if lower_words_end_with(&words, &["you", "control"]) {
        filter.controller = Some(PlayerFilter::You);
        filter.zone = Some(Zone::Battlefield);
        trim_suffix(&mut words, 2);
    } else if lower_words_end_with(&words, &["you", "dont", "control"])
        || lower_words_end_with(&words, &["you", "don't", "control"])
        || lower_words_end_with(&words, &["you", "do", "not", "control"])
    {
        filter.controller = Some(PlayerFilter::NotYou);
        filter.zone = Some(Zone::Battlefield);
        let suffix_len = if words.ends_with(&["you", "do", "not", "control"]) {
            4
        } else {
            3
        };
        trim_suffix(&mut words, suffix_len);
    } else if lower_words_end_with(&words, &["opponents", "control"])
        || lower_words_end_with(&words, &["opponent", "controls"])
    {
        filter.controller = Some(PlayerFilter::Opponent);
        filter.zone = Some(Zone::Battlefield);
        trim_suffix(&mut words, 2);
    } else if lower_words_end_with(&words, &["you", "own"]) {
        filter.owner = Some(PlayerFilter::You);
        trim_suffix(&mut words, 2);
    } else if lower_words_end_with(&words, &["in", "your", "graveyard"])
        || lower_words_end_with(&words, &["from", "your", "graveyard"])
    {
        apply_owner_zone_suffix(
            &mut filter,
            PlayerFilter::You,
            Zone::Graveyard,
            &mut words,
            3,
        );
    } else if lower_words_end_with(&words, &["in", "your", "hand"])
        || lower_words_end_with(&words, &["from", "your", "hand"])
    {
        apply_owner_zone_suffix(&mut filter, PlayerFilter::You, Zone::Hand, &mut words, 3);
    } else if lower_words_end_with(&words, &["in", "your", "library"])
        || lower_words_end_with(&words, &["from", "your", "library"])
    {
        apply_owner_zone_suffix(&mut filter, PlayerFilter::You, Zone::Library, &mut words, 3);
    } else if lower_words_end_with(&words, &["in", "graveyard"])
        || lower_words_end_with(&words, &["from", "graveyard"])
    {
        filter.zone = Some(Zone::Graveyard);
        trim_suffix(&mut words, 2);
    } else if lower_words_end_with(&words, &["in", "hand"])
        || lower_words_end_with(&words, &["from", "hand"])
    {
        filter.zone = Some(Zone::Hand);
        trim_suffix(&mut words, 2);
    } else if lower_words_end_with(&words, &["in", "library"])
        || lower_words_end_with(&words, &["from", "library"])
    {
        filter.zone = Some(Zone::Library);
        trim_suffix(&mut words, 2);
    } else if lower_words_end_with(&words, &["in", "exile"])
        || lower_words_end_with(&words, &["from", "exile"])
    {
        filter.zone = Some(Zone::Exile);
        trim_suffix(&mut words, 2);
    }

    let mut idx = 0usize;
    while idx < words.len() {
        let word = words[idx];
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            saw_spell = true;
            idx += consumed;
            continue;
        }
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
        if word == "chosen" && idx + 1 < words.len() {
            if words[idx + 1] == "color" {
                filter.chosen_color = true;
                idx += 2;
                continue;
            }
            if words[idx + 1] == "type" {
                filter.chosen_creature_type = true;
                idx += 2;
                continue;
            }
        }
        if word == "nonchosen" && idx + 1 < words.len() && words[idx + 1] == "type" {
            filter.excluded_chosen_creature_type = true;
            idx += 2;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            if !filter.card_types.contains(&card_type) {
                filter.card_types.push(card_type);
            }
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
            idx += 1;
            continue;
        }
        if let Some(card_type) = parse_non_type(word) {
            if !filter.excluded_card_types.contains(&card_type) {
                filter.excluded_card_types.push(card_type);
            }
            idx += 1;
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !filter.subtypes.contains(&subtype) {
                filter.subtypes.push(subtype);
            }
            idx += 1;
            continue;
        }
        if let Some(subtype) = parse_non_subtype(word) {
            if !filter.excluded_subtypes.contains(&subtype) {
                filter.excluded_subtypes.push(subtype);
            }
            idx += 1;
            continue;
        }
        if let Some(supertype) = parse_supertype_word(word) {
            if !filter.supertypes.contains(&supertype) {
                filter.supertypes.push(supertype);
            }
            idx += 1;
            continue;
        }
        if let Some(supertype) = parse_non_supertype(word) {
            if !filter.excluded_supertypes.contains(&supertype) {
                filter.excluded_supertypes.push(supertype);
            }
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

fn parse_attached_reference_or_another_disjunction(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let segments = split_on_or(tokens);
    if segments.len() != 2 {
        return Ok(None);
    }

    let first_view = LowercaseWordView::new(&segments[0]);
    let first_words: Vec<&str> = first_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let second_view = LowercaseWordView::new(&segments[1]);
    let second_words: Vec<&str> = second_view
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

    let first_filter = parse_object_filter(&segments[0], first_other)?;
    let second_filter = parse_object_filter(&segments[1], second_other)?;

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = vec![first_filter, second_filter];
    Ok(Some(disjunction))
}

pub(crate) fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
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
            let target_view = LowercaseWordView::new(fragment_tokens);
            let target_words = target_view.to_word_refs();
            if target_words.starts_with(&["you"]) {
                return Ok((Some(PlayerFilter::You), None));
            }
            if target_words.starts_with(&["opponent"]) || target_words.starts_with(&["opponents"]) {
                return Ok((Some(PlayerFilter::Opponent), None));
            }
            if target_words.starts_with(&["player"]) || target_words.starts_with(&["players"]) {
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

        let target_view = LowercaseWordView::new(target_tokens);
        let target_words = target_view.to_word_refs();
        if let Some(or_word_idx) = target_words.iter().position(|word| *word == "or")
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
    if let Some(until_token_idx) = base_tokens.iter().position(|token| token.is_word("until"))
        && until_token_idx > 0
    {
        base_tokens.truncate(until_token_idx);
    }

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

    let all_words_view = LowercaseWordView::new(&base_tokens);
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
        if let Some(word_start) = all_words.windows(8).position(|window| window == phrase) {
            filter.entered_graveyard_this_turn = true;
            filter.entered_graveyard_from_battlefield_this_turn = true;
            all_words.drain(word_start..word_start + 8);

            let segment_view = LowercaseWordView::new(&segment_tokens);
            let segment_words = segment_view.to_word_refs();
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
                if let Some(seg_start) = segment_words
                    .windows(len)
                    .position(|window| window == segment_phrase)
                {
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
    let legendary_or_subtype = all_words.windows(3).find_map(|window| {
        if window[0] == "legendary" && window[1] == "or" {
            parse_subtype_word(window[2])
        } else {
            None
        }
    });

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
        if let Some(word_start) = all_words.windows(8).position(|window| window == phrase) {
            filter.entered_graveyard_this_turn = true;
            all_words.drain(word_start..word_start + 8);

            let segment_view = LowercaseWordView::new(&segment_tokens);
            let segment_words = segment_view.to_word_refs();
            if let Some(seg_start) = segment_words.windows(8).position(|window| window == phrase)
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
        if let Some(word_start) = all_words.windows(5).position(|window| window == phrase) {
            filter.entered_graveyard_from_battlefield_this_turn = true;
            all_words.drain(word_start + 1..word_start + 5);

            let segment_view = LowercaseWordView::new(&segment_tokens);
            let segment_words = segment_view.to_word_refs();
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
                if let Some(seg_start) = segment_words
                    .windows(len)
                    .position(|window| window == phrase)
                {
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
    for (idx, window) in all_words.windows(7).enumerate() {
        if window[0] == "entered"
            && window[1] == "battlefield"
            && window[2] == "under"
            && window[4] == "control"
            && window[5] == "this"
            && window[6] == "turn"
        {
            let controller = match window[3] {
                "your" => Some(PlayerFilter::You),
                "opponent" | "opponents" => Some(PlayerFilter::Opponent),
                _ => None,
            };
            entered_battlefield_match = Some((idx, 7, controller));
            break;
        }
    }
    if entered_battlefield_match.is_none() {
        if let Some(idx) = all_words
            .windows(4)
            .position(|window| window == ["entered", "battlefield", "this", "turn"])
        {
            entered_battlefield_match = Some((idx, 4, None));
        }
    }
    if let Some((word_start, len, controller)) = entered_battlefield_match {
        filter.entered_battlefield_this_turn = true;
        filter.entered_battlefield_controller = controller;
        filter.zone = Some(Zone::Battlefield);
        all_words.drain(word_start..word_start + len);

        let segment_view = LowercaseWordView::new(&segment_tokens);
        let segment_words = segment_view.to_word_refs();
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
            if let Some(seg_start) = segment_words
                .windows(len)
                .position(|window| window == phrase)
            {
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
            let segment_view = LowercaseWordView::new(&segment_tokens);
            let segment_words = segment_view.to_word_refs();
            let mut segment_match: Option<(usize, usize)> = None;
            for len in [13usize, 12usize] {
                let Some(idx) = segment_words.windows(len).position(|window| {
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

    if let Some(idx) = all_words
        .windows(4)
        .position(|window| window == ["that", "isnt", "all", "colors"])
    {
        filter.all_colors = Some(false);
        all_words.drain(idx..idx + 4);
    } else if let Some(idx) = all_words
        .windows(3)
        .position(|window| window == ["isnt", "all", "colors"])
    {
        filter.all_colors = Some(false);
        all_words.drain(idx..idx + 3);
    }

    if let Some(idx) = all_words
        .windows(5)
        .position(|window| window == ["that", "isnt", "exactly", "two", "colors"])
    {
        filter.exactly_two_colors = Some(false);
        all_words.drain(idx..idx + 5);
    } else if let Some(idx) = all_words
        .windows(4)
        .position(|window| window == ["isnt", "exactly", "two", "colors"])
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

    if let Some(idx) = all_words
        .windows(7)
        .position(|window| window == ["that", "entered", "since", "your", "last", "turn", "ended"])
    {
        filter.entered_since_your_last_turn_ended = true;
        all_words.drain(idx..idx + 7);
    } else if let Some(idx) = all_words
        .windows(6)
        .position(|window| window == ["entered", "since", "your", "last", "turn", "ended"])
    {
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

    if all_words
        .windows(3)
        .any(|window| window == ["entered", "this", "turn"])
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported entered-this-turn object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if all_words.windows(4).any(|window| {
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

    if let Some(not_named_idx) = all_words
        .windows(2)
        .position(|window| window == ["not", "named"])
    {
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

    if let Some(named_idx) = all_words.iter().position(|word| *word == "named") {
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

    if let Some(color_count_idx) = all_words.windows(4).position(|window| {
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
    let has_power_or_toughness_clause = all_words.windows(3).any(|window| {
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

    if let Some(its_attached_idx) = all_words
        .windows(3)
        .position(|window| window == ["its", "attached", "to"])
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

    if let Some(attached_idx) = all_words.iter().position(|word| *word == "attached")
        && all_words.get(attached_idx + 1) == Some(&"to")
    {
        let attached_to_words = &all_words[attached_idx + 2..];
        let references_it = attached_to_words.starts_with(&["it"])
            || attached_to_words.starts_with(&["that", "object"])
            || attached_to_words.starts_with(&["that", "creature"])
            || attached_to_words.starts_with(&["that", "permanent"])
            || attached_to_words.starts_with(&["that", "equipment"])
            || attached_to_words.starts_with(&["that", "aura"]);
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

    if let Some(relation_idx) = all_words.windows(6).position(|window| {
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

    let starts_with_exiled_card =
        all_words.starts_with(&["exiled", "card"]) || all_words.starts_with(&["exiled", "cards"]);
    if starts_with_exiled_card {
        filter.zone.get_or_insert(Zone::Exile);
    }
    let has_exiled_with_phrase = all_words
        .windows(2)
        .any(|window| window == ["exiled", "with"]);
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
        if let Some(exiled_with_idx) = all_words
            .windows(2)
            .position(|window| window == ["exiled", "with"])
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
        if let Some(exiled_with_idx) = segment_tokens
            .windows(2)
            .position(|window| window[0].is_word("exiled") && window[1].is_word("with"))
        {
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

    let has_share_card_type = (all_words.contains(&"share") || all_words.contains(&"shares"))
        && (all_words.contains(&"card") || all_words.contains(&"permanent"))
        && all_words.contains(&"type")
        && all_words.contains(&"it");
    let has_share_color =
        all_words.contains(&"shares") && all_words.contains(&"color") && all_words.contains(&"it");
    let has_same_mana_value = all_words
        .windows(4)
        .any(|window| window == ["same", "mana", "value", "as"]);
    let has_equal_or_lesser_mana_value = all_words
        .windows(5)
        .any(|window| window == ["equal", "or", "lesser", "mana", "value"]);
    let has_lte_mana_value_as_tagged = all_words.windows(8).any(|window| {
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
    }) || all_words.windows(9).any(|window| {
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
    let has_lt_mana_value_as_tagged = all_words
        .windows(3)
        .any(|window| window == ["lesser", "mana", "value"])
        && !has_equal_or_lesser_mana_value;
    let references_sacrifice_cost_object = all_words.windows(3).any(|window| {
        matches!(
            window,
            ["the", "sacrificed", "creature"]
                | ["the", "sacrificed", "artifact"]
                | ["the", "sacrificed", "permanent"]
                | ["a", "sacrificed", "creature"]
                | ["a", "sacrificed", "artifact"]
                | ["a", "sacrificed", "permanent"]
        )
    }) || all_words.windows(2).any(|window| {
        matches!(
            window,
            ["sacrificed", "creature"] | ["sacrificed", "artifact"] | ["sacrificed", "permanent"]
        )
    });
    let references_it_for_mana_value = all_words.iter().any(|word| matches!(*word, "it" | "its"))
        || all_words.windows(2).any(|window| {
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
    let has_same_name_as_tagged_object = all_words.windows(5).any(|window| {
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

    if all_words
        .windows(4)
        .any(|window| window == ["that", "convoked", "this", "spell"])
        || all_words
            .windows(3)
            .any(|window| window == ["that", "convoked", "it"])
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("convoked_this_spell"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if all_words
        .windows(5)
        .any(|window| window == ["that", "crewed", "it", "this", "turn"])
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("crewed_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if all_words
        .windows(5)
        .any(|window| window == ["that", "saddled", "it", "this", "turn"])
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("saddled_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if all_words.windows(3).any(|window| {
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

    let references_target_player = all_words
        .windows(2)
        .any(|window| matches!(window, ["target", "player"] | ["target", "players"]));
    let references_target_opponent = all_words
        .windows(2)
        .any(|window| matches!(window, ["target", "opponent"] | ["target", "opponents"]));
    let pronoun_player_filter = if references_target_opponent {
        PlayerFilter::target_opponent()
    } else if references_target_player {
        PlayerFilter::target_player()
    } else {
        PlayerFilter::IteratedPlayer
    };

    if all_words.len() >= 2 {
        for window in all_words.windows(2) {
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
        }
    }
    if all_words.len() >= 3 {
        for window in all_words.windows(3) {
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
        for window in all_words.windows(5) {
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
        }
    }
    if all_words.len() >= 2 {
        for window in all_words.windows(2) {
            match window {
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
        }
    }
    if all_words.len() >= 3 {
        for window in all_words.windows(3) {
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
                ["target", "opponent", "control"] | ["target", "opponent", "controls"] => {
                    filter.controller = Some(PlayerFilter::target_opponent());
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
        }
    }
    if all_words.len() >= 4 {
        for window in all_words.windows(4) {
            if window[1..] == ["your", "team", "control"]
                || window[1..] == ["your", "team", "controls"]
            {
                filter.controller = Some(PlayerFilter::You);
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

    let has_tap_activated_ability = all_words.windows(9).any(|window| {
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
    }) || all_words.windows(8).any(|window| {
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
    }) || all_words.windows(7).any(|window| {
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
            && !filter.excluded_card_types.contains(&card_type)
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
            && !filter.excluded_subtypes.contains(&subtype)
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
            && !filter.excluded_card_types.contains(&card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(target_idx);
        }
        if let Some(supertype) = parse_supertype_word(negated_word)
            && !filter.excluded_supertypes.contains(&supertype)
        {
            filter.excluded_supertypes.push(supertype);
            negated_word_indices.insert(target_idx);
        }
        if let Some(color) = parse_color(negated_word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(target_idx);
        }
        if let Some(subtype) = parse_subtype_word(negated_word)
            .or_else(|| negated_word.strip_suffix('s').and_then(parse_subtype_word))
            && !filter.excluded_subtypes.contains(&subtype)
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
        let is_negated_word = negated_word_indices.contains(&idx);
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
            "historic" if !negated_historic_indices.contains(&idx) => filter.historic = true,
            "modified" if !is_negated_word => filter.modified = true,
            _ => {}
        }

        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            continue;
        }

        if negated_word_indices.contains(&idx) {
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
            && !filter.excluded_supertypes.contains(&supertype)
        {
            filter.excluded_supertypes.push(supertype);
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
        }
        if let Some(subtype) = parse_non_subtype(word)
            && !filter.excluded_subtypes.contains(&subtype)
        {
            filter.excluded_subtypes.push(subtype);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(supertype) = parse_supertype_word(word)
            && !filter.supertypes.contains(&supertype)
        {
            filter.supertypes.push(supertype);
        }

        if let Some(card_type) = parse_card_type(word) {
            if !filter.card_types.contains(&card_type) {
                filter.card_types.push(card_type);
            }
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
        }

        if let Some(subtype) = parse_subtype_flexible(word) {
            if !filter.subtypes.contains(&subtype) {
                filter.subtypes.push(subtype);
            }
            saw_subtype = true;
        }
    }
    if saw_spell && source_linked_exile_reference {
        // "spell ... exiled with this" describes a stack spell with a relation
        // to source-linked exiled cards, not a spell object in exile.
        filter.zone = Some(Zone::Stack);
    }

    let segments = split_on_or(&segment_tokens);
    let mut segment_types = Vec::new();
    let mut segment_subtypes = Vec::new();
    let mut segment_marker_counts = Vec::new();
    let mut segment_words_lists: Vec<Vec<String>> = Vec::new();

    for segment in &segments {
        let segment_view = LowercaseWordView::new(segment);
        let segment_words: Vec<String> = segment_view
            .to_word_refs()
            .into_iter()
            .filter(|word| !is_article(word))
            .map(ToString::to_string)
            .collect();
        segment_words_lists.push(segment_words.clone());
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &segment_words {
            if let Some(card_type) = parse_card_type(word)
                && !types.contains(&card_type)
            {
                types.push(card_type);
            }
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
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
                if !any_types.contains(&card_type) {
                    any_types.push(card_type);
                }
            }
            for subtypes in segment_subtypes {
                let Some(subtype) = subtypes.first().copied() else {
                    continue;
                };
                if !any_subtypes.contains(&subtype) {
                    any_subtypes.push(subtype);
                }
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
        let has_and = all_words.contains(&"and");
        let has_or = all_words.contains(&"or");
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
    let and_segments = split_on_and(&segment_tokens);
    let and_segment_words_lists: Vec<Vec<String>> = and_segments
        .iter()
        .map(|segment| {
            LowercaseWordView::new(segment)
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
        segment
            .windows(2)
            .any(|window| matches!(window, [permanent, spell] if (permanent == "permanent" || permanent == "permanents") && (spell == "spell" || spell == "spells")))
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

    if target_player.is_some() || target_object.is_some() {
        filter = filter.targeting(target_player.take(), target_object.take());
    }

    if let Some(or_subtype) = legendary_or_subtype
        && filter.any_of.is_empty()
        && filter.supertypes.contains(&Supertype::Legendary)
        && filter.subtypes.contains(&or_subtype)
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

    let owner_or_controller_player = all_words
        .windows(4)
        .find_map(|window| match window {
            ["you", "own", "or", "control"]
            | ["you", "owns", "or", "controls"]
            | ["you", "control", "or", "own"]
            | ["you", "controls", "or", "owns"] => Some(PlayerFilter::You),
            ["opponent", "own", "or", "control"]
            | ["opponent", "owns", "or", "controls"]
            | ["opponents", "own", "or", "control"]
            | ["opponents", "owns", "or", "controls"]
            | ["opponent", "control", "or", "own"]
            | ["opponent", "controls", "or", "owns"]
            | ["opponents", "control", "or", "own"]
            | ["opponents", "controls", "or", "owns"] => Some(PlayerFilter::Opponent),
            ["they", "own", "or", "control"]
            | ["they", "owns", "or", "controls"]
            | ["they", "control", "or", "own"]
            | ["they", "controls", "or", "owns"] => Some(pronoun_player_filter.clone()),
            _ => None,
        })
        .or_else(|| {
            all_words.windows(5).find_map(|window| match window {
                ["your", "team", "own", "or", "control"]
                | ["your", "team", "owns", "or", "controls"]
                | ["your", "team", "control", "or", "own"]
                | ["your", "team", "controls", "or", "owns"] => Some(PlayerFilter::You),
                ["that", "player", "own", "or", "control"]
                | ["that", "player", "owns", "or", "controls"]
                | ["that", "player", "control", "or", "own"]
                | ["that", "player", "controls", "or", "owns"] => {
                    Some(PlayerFilter::IteratedPlayer)
                }
                ["target", "player", "own", "or", "control"]
                | ["target", "player", "owns", "or", "controls"]
                | ["target", "player", "control", "or", "own"]
                | ["target", "player", "controls", "or", "owns"] => {
                    Some(PlayerFilter::target_player())
                }
                ["target", "opponent", "own", "or", "control"]
                | ["target", "opponent", "owns", "or", "controls"]
                | ["target", "opponent", "control", "or", "own"]
                | ["target", "opponent", "controls", "or", "owns"] => {
                    Some(PlayerFilter::target_opponent())
                }
                _ => None,
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
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
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
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
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

    Ok(filter)
}

pub(crate) fn parse_spell_filter(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    let words: Vec<&str> = words(tokens)
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
        if let Some(card_type) = parse_card_type(word)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        }
        if let Some(card_type) = parse_non_type(word)
            && !filter.excluded_card_types.contains(&card_type)
        {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(subtype) = parse_subtype_flexible(word)
            && !filter.subtypes.contains(&subtype)
        {
            filter.subtypes.push(subtype);
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

pub(crate) fn parse_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    if let Some(filter) = parse_simple_object_filter_lexed(tokens, other) {
        return Ok(filter);
    }
    parse_object_filter(tokens, other)
}

pub(crate) fn parse_spell_filter_lexed(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    let word_view = LowercaseWordView::new(tokens);
    let words: Vec<&str> = word_view
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
        if let Some(card_type) = parse_card_type(word)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        }
        if let Some(card_type) = parse_non_type(word)
            && !filter.excluded_card_types.contains(&card_type)
        {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(subtype) = parse_subtype_flexible(word)
            && !filter.subtypes.contains(&subtype)
        {
            filter.subtypes.push(subtype);
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

pub(crate) fn spell_filter_has_identity(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.colors.is_some()
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.cast_by.is_some()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || filter.alternative_cast.is_some()
        || !filter.any_of.is_empty()
}

pub(crate) fn merge_spell_filters(base: &mut ObjectFilter, extra: ObjectFilter) {
    for card_type in extra.card_types {
        if !base.card_types.contains(&card_type) {
            base.card_types.push(card_type);
        }
    }
    for card_type in extra.excluded_card_types {
        if !base.excluded_card_types.contains(&card_type) {
            base.excluded_card_types.push(card_type);
        }
    }
    for subtype in extra.subtypes {
        if !base.subtypes.contains(&subtype) {
            base.subtypes.push(subtype);
        }
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
    if base.toughness.is_none() {
        base.toughness = extra.toughness;
    }
    if base.mana_value.is_none() {
        base.mana_value = extra.mana_value;
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

pub(crate) fn split_on_or(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        let is_separator =
            token.is_comma() || (token.is_word("or") && !is_comparison_or_delimiter(tokens, idx));
        if is_separator {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
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
