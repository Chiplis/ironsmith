use super::super::effects_sentences::find_color_choice_phrase;
use crate::cards::builders::parse_parsing::keyword_static::parse_add_mana_that_much_value;
use crate::cards::builders::parse_parsing::{
    controller_filter_for_token_player, extract_subject_player, intern_counter_name,
    parse_add_mana_equal_amount_value, parse_devotion_value_from_add_clause,
    parse_dynamic_cost_modifier_value, parse_get_modifier_values_with_tail, parse_mana_symbol,
    parse_mana_symbol_group, parse_number, parse_number_word_i32, parse_pt_modifier,
    parse_pt_modifier_values, parse_restriction_duration, parse_subtype_word,
    parse_value_expr_words, parse_where_x_is_number_of_filter_value, parse_zone_word,
    parser_trace_stack, trim_edge_punctuation,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, ObjectRefAst, PlayerAst, PredicateAst, ReturnControllerAst,
    SharedTypeConstraintAst, SubjectAst, TagKey, TargetAst, Token, is_article, parse_color,
    parse_counter_type_word, parse_object_filter, parse_predicate, parse_target_phrase,
    parse_value, span_from_tokens, target_ast_to_object_filter, token_index_for_word_index,
    trim_commas, words,
};
use crate::effect::{Until, Value};
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

pub(crate) fn parse_tap(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "tap clause missing target".to_string(),
        ));
    }
    let words = words(tokens);
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
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.contains(&"unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported sacrifice-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_greatest_mana_value = clause_words.contains(&"greatest")
        && clause_words.contains(&"mana")
        && clause_words.contains(&"value");
    if has_greatest_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported greatest-mana-value sacrifice clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_for_each_graveyard_history = clause_words.contains(&"for")
        && clause_words.contains(&"each")
        && clause_words.contains(&"graveyard")
        && clause_words.contains(&"turn");
    if has_for_each_graveyard_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported graveyard-history sacrifice clause (clause: '{}')",
            clause_words.join(" ")
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
        let mut filter = parse_object_filter(&tokens[idx..], other)?;
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
            words(tokens).join(" ")
        )));
    }
    let mut filter = if let Ok(target) = parse_target_phrase(filter_tokens) {
        target_ast_to_object_filter(target).unwrap_or(parse_object_filter(filter_tokens, other)?)
    } else {
        parse_object_filter(filter_tokens, other)?
    };
    if other {
        filter.other = true;
    }
    if filter.source && count != 1 {
        return Err(CardTextError::ParseError(format!(
            "source sacrifice only supports count 1 (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let sacrifice_words = words(tokens);
    let excludes_attached_object = sacrifice_words.windows(3).any(|window| {
        matches!(
            window,
            ["than", "enchanted", "creature"]
                | ["than", "enchanted", "permanent"]
                | ["than", "equipped", "creature"]
                | ["than", "equipped", "permanent"]
        )
    });
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

pub(crate) fn trim_sacrifice_choice_suffix_tokens(tokens: &[Token]) -> &[Token] {
    let token_words = words(tokens);
    let suffix_word_count = if token_words.ends_with(&["of", "their", "choice"])
        || token_words.ends_with(&["of", "your", "choice"])
        || token_words.ends_with(&["of", "its", "choice"])
    {
        3usize
    } else if token_words.ends_with(&["of", "his", "or", "her", "choice"]) {
        5usize
    } else {
        0usize
    };

    if suffix_word_count == 0 {
        return tokens;
    }

    let keep_words = token_words.len().saturating_sub(suffix_word_count);
    let cut_idx = token_index_for_word_index(tokens, keep_words).unwrap_or(tokens.len());
    &tokens[..cut_idx]
}

pub(crate) fn parse_discard(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = words(tokens);
    if clause_words.contains(&"hand") {
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

    let (count, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing discard count (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    let rest_words = words(rest);
    let Some(card_word_idx) = rest_words
        .iter()
        .position(|word| *word == "card" || *word == "cards")
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
        } else if let Some(filter) = parse_discard_color_qualifier_filter(&qualifier_tokens) {
            filter
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported discard card qualifier (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        filter.zone = Some(Zone::Hand);
        discard_filter = Some(filter);
    }

    let trailing_tokens = if card_word_idx + 1 < rest_words.len() {
        let trailing_token_idx =
            token_index_for_word_index(rest, card_word_idx + 1).unwrap_or(rest.len());
        &rest[trailing_token_idx..]
    } else {
        &[]
    };
    let trailing_words = words(trailing_tokens);
    let random = trailing_words.as_slice() == ["at", "random"];
    if !trailing_words.is_empty() && !random {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing discard clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::Discard {
        count,
        player,
        random,
        filter: discard_filter,
        tag: None,
    })
}

pub(crate) fn parse_discard_color_qualifier_filter(tokens: &[Token]) -> Option<ObjectFilter> {
    let qualifier_words: Vec<&str> = words(tokens)
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

pub(crate) fn parse_return(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.contains(&"unless") {
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

    let to_idx = (0..tokens.len())
        .rev()
        .find(|idx| {
            if !tokens[*idx].is_word("to") {
                return false;
            }
            let tail_words = words(&tokens[*idx + 1..]);
            tail_words.contains(&"hand")
                || tail_words.contains(&"hands")
                || tail_words.contains(&"battlefield")
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing return destination (clause: '{}')",
                words(tokens).join(" ")
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
    let destination_words_full = words(destination_tokens_full);
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

    let mut destination_words = words(destination_tokens);
    let mut destination_excluded_subtypes: Vec<Subtype> = Vec::new();
    if let Some(except_idx) = destination_words
        .windows(2)
        .position(|window| window == ["except", "for"])
    {
        let exception_words = &destination_words[except_idx + 2..];
        if exception_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing return exception qualifiers (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        for word in exception_words {
            if matches!(*word, "and" | "or") {
                continue;
            }
            let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported return exception qualifier '{}' (clause: '{}')",
                    word,
                    words(tokens).join(" ")
                )));
            };
            if !destination_excluded_subtypes.contains(&subtype) {
                destination_excluded_subtypes.push(subtype);
            }
        }
        if destination_excluded_subtypes.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing subtype return exception qualifiers (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        destination_words.truncate(except_idx);
    }
    let is_hand = destination_words.contains(&"hand") || destination_words.contains(&"hands");
    let is_battlefield = destination_words.contains(&"battlefield");
    let tapped = destination_words.contains(&"tapped");
    let return_controller = if destination_words
        .windows(3)
        .any(|window| window == ["under", "your", "control"])
    {
        ReturnControllerAst::You
    } else if destination_words
        .iter()
        .any(|word| *word == "owner" || *word == "owners")
        && destination_words.contains(&"control")
    {
        ReturnControllerAst::Owner
    } else {
        ReturnControllerAst::Preserve
    };
    if destination_words.contains(&"transformed") {
        return Err(CardTextError::ParseError(format!(
            "unsupported transformed return clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let has_delayed_timing_words = destination_words_full.contains(&"beginning")
        || destination_words_full.contains(&"upkeep")
        || destination_words_full
            .windows(3)
            .any(|window| window == ["end", "of", "combat"])
        || destination_words_full.contains(&"end")
            && (destination_words_full.contains(&"next")
                || destination_words_full.contains(&"step"));
    if delayed_timing.is_none() && has_delayed_timing_words {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed return timing clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    if !is_hand && !is_battlefield {
        return Err(CardTextError::ParseError(format!(
            "unsupported return destination (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let target_words = words(target_tokens);
    if let Some(and_idx) = target_tokens.iter().position(|token| token.is_word("and"))
        && and_idx > 0
    {
        let tail_words = words(&target_tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (tail_words.starts_with(&["up", "to"]) && tail_words.contains(&"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target return clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }
    if !target_words.contains(&"target")
        && target_words.contains(&"exiled")
        && target_words.contains(&"cards")
    {
        let filter = parse_object_filter(target_tokens, false)?;
        let effect = if is_battlefield {
            EffectAst::ReturnAllToBattlefield { filter, tapped }
        } else {
            EffectAst::ReturnAllToHand { filter }
        };
        return Ok(wrap_return_with_delayed_timing(effect, delayed_timing));
    }
    if target_words
        .first()
        .is_some_and(|word| *word == "all" || *word == "each")
    {
        let has_unsupported_return_all_qualifier = target_words.contains(&"dealt")
            || target_words.contains(&"without") && target_words.contains(&"counter");
        if has_unsupported_return_all_qualifier {
            return Err(CardTextError::ParseError(format!(
                "unsupported qualified return-all filter (clause: '{}')",
                words(tokens).join(" ")
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
                    words(tokens).join(" ")
                )));
            }
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter before color-choice clause (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
            let mut filter = parse_object_filter(&base_filter_tokens, false)?;
            for subtype in destination_excluded_subtypes {
                if !filter.excluded_subtypes.contains(&subtype) {
                    filter.excluded_subtypes.push(subtype);
                }
            }
            return Ok(wrap_return_with_delayed_timing(
                EffectAst::ReturnAllToHandOfChosenColor { filter },
                delayed_timing,
            ));
        }
        let mut filter = parse_object_filter(return_filter_tokens, false)?;
        for subtype in destination_excluded_subtypes {
            if !filter.excluded_subtypes.contains(&subtype) {
                filter.excluded_subtypes.push(subtype);
            }
        }
        let effect = if is_battlefield {
            EffectAst::ReturnAllToBattlefield { filter, tapped }
        } else {
            EffectAst::ReturnAllToHand { filter }
        };
        return Ok(wrap_return_with_delayed_timing(effect, delayed_timing));
    }
    if !destination_excluded_subtypes.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported return exception on non-return-all clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let target = parse_target_phrase(target_tokens)?;
    let effect = if is_battlefield {
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            controller: return_controller,
        }
    } else {
        EffectAst::ReturnToHand { target, random }
    };
    Ok(wrap_return_with_delayed_timing(effect, delayed_timing))
}

fn rewrite_destination_first_return_clause(tokens: &[Token]) -> Option<Vec<Token>> {
    let clause_words = words(tokens);
    let hand_or_battlefield_idx = clause_words
        .iter()
        .position(|word| matches!(*word, "hand" | "hands" | "battlefield"))?;
    let mut split_word_idx = hand_or_battlefield_idx + 1;

    if clause_words.get(split_word_idx).copied() == Some("under") {
        let control_rel_idx = clause_words[split_word_idx + 1..]
            .iter()
            .position(|word| *word == "control")?;
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

pub(crate) fn parse_exchange(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["control", "of"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    // Heterogeneous "exchange control of A and B" forms (e.g. "this artifact and target ...")
    // cannot be represented by the current single-filter ExchangeControl primitive.
    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) {
        let left_words = words(&tokens[..and_idx]);
        let right_words = words(&tokens[and_idx + 1..]);
        let left_mentions_this = left_words.contains(&"this");
        let right_mentions_this = right_words.contains(&"this");
        let left_mentions_target = left_words.contains(&"target");
        let right_mentions_target = right_words.contains(&"target");
        if left_mentions_this
            || right_mentions_this
            || left_mentions_target
            || right_mentions_target
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported heterogeneous exchange clause (clause: '{}')",
                clause_words.join(" ")
            )));
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

    let mut shared_type = None;
    let mut filter_tokens = &tokens[idx..];
    let tail_words = words(filter_tokens);
    if let Some(rel_word_idx) = tail_words
        .windows(2)
        .position(|window| window[0] == "that" && matches!(window[1], "share" | "shares"))
    {
        let rel_token_idx =
            token_index_for_word_index(filter_tokens, rel_word_idx).unwrap_or(filter_tokens.len());
        let (head, tail) = filter_tokens.split_at(rel_token_idx);
        filter_tokens = head;

        let share_words = words(tail);
        let share_head = if share_words.starts_with(&["that", "share"]) {
            &share_words[2..]
        } else if share_words.starts_with(&["that", "shares"]) {
            &share_words[2..]
        } else {
            &share_words[..]
        };
        let share_head = if share_head.first().copied() == Some("a") {
            &share_head[1..]
        } else {
            share_head
        };
        if share_head.starts_with(&["permanent", "type"]) {
            shared_type = Some(SharedTypeConstraintAst::PermanentType);
        } else if share_head.starts_with(&["card", "type"]) {
            shared_type = Some(SharedTypeConstraintAst::CardType);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported exchange share-type clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(EffectAst::ExchangeControl {
        filter,
        count,
        shared_type,
    })
}

pub(crate) fn parse_become(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported become clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    };

    let amount = parse_value(tokens).map(|(value, _)| value).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life total amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    Ok(EffectAst::SetLifeTotal { amount, player })
}

pub(crate) fn parse_switch(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    use crate::effect::Until;

    let clause_words = words(tokens);

    // Split off trailing duration, if present.
    let (duration, remainder) =
        if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, trim_commas(tokens).to_vec())
        };

    let remainder_words = words(&remainder);
    let Some(power_idx) = remainder.iter().position(|token| token.is_word("power")) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    // Target phrase is everything up to "power".
    let target_tokens = &remainder[..power_idx];
    let target_words = words(target_tokens);
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
    if !remainder_words.contains(&"power") || !remainder_words.contains(&"toughness") {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::SwitchPowerToughness { target, duration })
}

pub(crate) fn parse_skip(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    let (player, words) = match subject {
        Some(SubjectAst::Player(player)) => (player, clause_words),
        _ => {
            if clause_words.starts_with(&["your"]) {
                (PlayerAst::You, clause_words[1..].to_vec())
            } else if clause_words.starts_with(&["their"]) {
                (PlayerAst::That, clause_words[1..].to_vec())
            } else if clause_words.starts_with(&["that", "player"])
                || clause_words.starts_with(&["that", "players"])
            {
                (PlayerAst::That, clause_words[2..].to_vec())
            } else if clause_words.starts_with(&["his", "or", "her"]) {
                (PlayerAst::That, clause_words[3..].to_vec())
            } else if clause_words.starts_with(&["target", "player"])
                || clause_words.starts_with(&["target", "players"])
            {
                (PlayerAst::Target, clause_words[2..].to_vec())
            } else if clause_words.starts_with(&["target", "opponent"])
                || clause_words.starts_with(&["target", "opponents"])
            {
                (PlayerAst::TargetOpponent, clause_words[2..].to_vec())
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    let skips_next_combat_phase_this_turn = words.contains(&"combat")
        && words.contains(&"phase")
        && words.contains(&"next")
        && words.contains(&"this")
        && words.contains(&"turn");
    if skips_next_combat_phase_this_turn {
        return Ok(EffectAst::SkipNextCombatPhaseThisTurn { player });
    }
    if words.contains(&"combat")
        && (words.contains(&"phase") || words.contains(&"phases"))
        && words.contains(&"turn")
    {
        return Ok(EffectAst::SkipCombatPhases { player });
    }
    if words.contains(&"draw") && words.contains(&"step") {
        return Ok(EffectAst::SkipDrawStep { player });
    }
    if words.contains(&"turn") {
        return Ok(EffectAst::SkipTurn { player });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported skip clause (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn parse_transform(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Transform {
            target: TargetAst::Source(None),
        });
    }
    let target_words = words(tokens);
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
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Transform { target })
}

pub(crate) fn parse_flip(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Flip {
            target: TargetAst::Source(None),
        });
    }

    let target_words = words(tokens);
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

pub(crate) fn parse_regenerate(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let words = words(tokens);
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
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    let starts_with_card_keyword = tokens
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word == "card" || word == "cards");

    let (count, used) = if starts_with_card_keyword {
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
        let trailing_words: Vec<&str> = rest.iter().filter_map(Token::as_word).collect();
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mill clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    } else {
        if rest
            .first()
            .and_then(Token::as_word)
            .is_some_and(|word| word != "card" && word != "cards")
        {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        let trailing_words: Vec<&str> = rest.iter().skip(1).filter_map(Token::as_word).collect();
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

pub(crate) fn parse_equal_to_number_of_filter_value(tokens: &[Token]) -> Option<Value> {
    let words_all = words(tokens);
    let equal_idx = words_all
        .windows(2)
        .position(|window| window == ["equal", "to"])?;
    let mut number_word_idx = equal_idx + 2;
    if words_all.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if words_all.get(number_word_idx).copied() != Some("number")
        || words_all.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }
    let filter_start_word_idx = number_word_idx + 2;
    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let filter_tokens = trim_edge_punctuation(&tokens[filter_start_token_idx..]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[Token],
) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["equal", "to"]) {
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
    let operator_word_idx = (filter_start_word_idx + 1..clause_words.len())
        .find(|idx| matches!(clause_words[*idx], "plus" | "minus"))?;
    let operator = clause_words[operator_word_idx];

    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let operator_token_idx = token_index_for_word_index(tokens, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_start_token_idx..operator_token_idx]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;

    let offset_start_token_idx = token_index_for_word_index(tokens, operator_word_idx + 1)?;
    let offset_tokens = trim_commas(&tokens[offset_start_token_idx..]);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    let trailing_words = words(&offset_tokens[used..]);
    if !trailing_words.is_empty() {
        return None;
    }

    let signed_offset = if operator == "minus" {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(Value::Count(filter)),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn parse_equal_to_number_of_opponents_you_have_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
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
    tokens: &[Token],
) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["equal", "to"]) {
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

pub(crate) fn parse_equal_to_aggregate_filter_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    let equal_idx = clause_words
        .windows(2)
        .position(|window| window == ["equal", "to"])?;

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
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_get(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.contains(&"poison")
        && (clause_words.contains(&"counter") || clause_words.contains(&"counters"))
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

    let energy_count = tokens.iter().filter(|token| token.is_word("e")).count();
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

    let modifier_start = if clause_words.starts_with(&["an", "additional"]) {
        2usize
    } else if clause_words.starts_with(&["additional"]) {
        1usize
    } else {
        0usize
    };
    if modifier_start > 0
        && let Some(mod_token) = tokens.get(modifier_start).and_then(Token::as_word)
        && let Ok((power_per, toughness_per)) = parse_pt_modifier(mod_token)
    {
        let tail_tokens = tokens.get(modifier_start + 1..).unwrap_or_default();
        let tail_words = words(tail_tokens);
        if tail_words.starts_with(&["until", "end", "of", "turn", "for", "each"]) {
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

    if let Some(mod_token) = tokens.first().and_then(Token::as_word)
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

    Err(CardTextError::ParseError(format!(
        "unsupported get clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_add_mana(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    parser_trace_stack("parse_add_mana:entry", tokens);
    let clause_words = words(tokens);
    let wrap_instead_if_tail = |base_effect: EffectAst,
                                tail_tokens: &[Token]|
     -> Result<Option<EffectAst>, CardTextError> {
        let tail_words = words(tail_tokens);
        if !tail_words.starts_with(&["instead", "if"]) {
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
    if clause_words.contains(&"exiled") && has_card_word && clause_words.contains(&"colors") {
        return Ok(EffectAst::AddManaImprintedColors);
    }

    if (clause_words.contains(&"commander") || clause_words.contains(&"commanders"))
        && clause_words.contains(&"color")
        && clause_words.contains(&"identity")
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
        .filter_map(Token::as_word)
        .any(|word| parse_mana_symbol(word).is_ok());
    if !has_explicit_symbol
        && let Some(chosen_idx) = clause_words
            .windows(2)
            .position(|window| window == ["chosen", "color"])
    {
        let prefix = &clause_words[..chosen_idx];
        let references_mana_of_chosen_color =
            prefix.ends_with(&["mana", "of", "the"]) || prefix.ends_with(&["mana", "of"]);
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
    if clause_words.starts_with(&["an", "amount", "of", "mana", "of", "that", "color"]) {
        return Ok(EffectAst::AddManaChosenColor {
            amount: Value::Fixed(1),
            player,
            fixed_option: None,
        });
    }

    let any_one = clause_words
        .windows(3)
        .any(|window| window == ["any", "one", "color"] || window == ["any", "one", "type"]);
    let any_color = clause_words
        .windows(2)
        .any(|window| window == ["any", "color"] || window == ["one", "color"]);
    let any_type = clause_words
        .windows(2)
        .any(|window| window == ["any", "type"] || window == ["one", "type"]);
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

        let tail_words = words(tail_tokens);
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

    let for_each_idx = tokens
        .windows(2)
        .position(|window| window[0].is_word("for") && window[1].is_word("each"));
    let mana_scan_end = for_each_idx.unwrap_or(tokens.len());

    let mut mana = Vec::new();
    let mut last_mana_idx = None;
    for (idx, token) in tokens[..mana_scan_end].iter().enumerate() {
        if let Some(word) = token.as_word() {
            if word == "mana" || word == "to" || word == "your" || word == "pool" {
                continue;
            }
            if let Ok(symbol) = parse_mana_symbol(word) {
                mana.push(symbol);
                last_mana_idx = Some(idx);
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
                    words(tokens).join(" ")
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
            words(&tokens[last_idx + 1..])
        } else {
            Vec::new()
        };
        if !trailing_words.is_empty() {
            let chosen_color_tail =
                trailing_words.starts_with(&["or", "one", "mana", "of", "the", "chosen", "color"]);
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
        if !trailing_words.is_empty() && !has_only_pool_tail {
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
    tokens: &[Token],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"or") {
        return Ok(None);
    }

    let mut colors = Vec::new();
    let mut has_or = false;
    for token in tokens {
        let Some(word) = token.as_word() else {
            continue;
        };
        if word == "or" {
            has_or = true;
            continue;
        }
        if matches!(word, "to" | "your" | "their" | "its" | "mana" | "pool") {
            continue;
        }
        if let Ok(symbol) = parse_mana_symbol(word) {
            let Some(color) = mana_symbol_to_color(symbol) else {
                return Ok(None);
            };
            if !colors.contains(&color) {
                colors.push(color);
            }
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
    tokens: &[Token],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_words = words(tokens);
    let Some(combination_idx) = clause_words
        .windows(3)
        .position(|window| window == ["any", "combination", "of"])
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
                if !colors.contains(&color) {
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
        if !colors.contains(&color) {
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

pub(crate) fn trim_leading_commas(tokens: &[Token]) -> &[Token] {
    let start = tokens
        .iter()
        .position(|token| !matches!(token, Token::Comma(_)))
        .unwrap_or(tokens.len());
    &tokens[start..]
}

pub(crate) fn is_mana_pool_tail_tokens(tokens: &[Token]) -> bool {
    let words = words(tokens);
    if words.is_empty() || words[0] != "to" || !words.contains(&"mana") || !words.contains(&"pool")
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
    tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = words(tokens);
    if words.len() < 3 || words[0] != "that" {
        return Ok(None);
    }

    let marker_word_idx = if let Some(could_idx) = words
        .windows(2)
        .position(|window| window == ["could", "produce"])
    {
        if could_idx + 2 != words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (tail: '{}')",
                words.join(" ")
            )));
        }
        could_idx
    } else if let Some(produced_idx) = words.iter().position(|word| *word == "produced") {
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

fn parse_counter_type_from_descriptor_tokens(tokens: &[Token]) -> Option<CounterType> {
    let words = words(tokens);
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

pub(crate) fn parse_remove(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if let Some(from_idx) = tokens.iter().position(|token| token.is_word("from")) {
        let tail_words = words(&tokens[from_idx + 1..]);
        if tail_words == ["combat"] {
            let target_tokens = trim_commas(&tokens[..from_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing remove-from-combat target (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::RemoveFromCombat { target });
        }
    }

    if tokens.first().is_some_and(|token| token.is_word("all"))
        && let Some(counter_idx) = tokens
            .iter()
            .position(|token| token.is_word("counter") || token.is_word("counters"))
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

        let target_words = words(&target_tokens);
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
            words(tokens).join(" ")
        ))
    })?;
    idx += used;

    let counter_idx = tokens[idx..]
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
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

    let for_each_idx = (0..target_tokens.len().saturating_sub(1))
        .find(|i| target_tokens[*i].is_word("for") && target_tokens[*i + 1].is_word("each"));
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

pub(crate) fn parse_destroy(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let original_clause_words = words(tokens);
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
    let clause_words = words(&core_tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing destroy target before delayed timing clause (clause: '{}')",
            original_clause_words.join(" ")
        )));
    }

    if delayed_timing.is_none()
        && (original_clause_words
            .windows(3)
            .any(|window| window == ["end", "of", "combat"])
            || (original_clause_words.contains(&"beginning")
                && original_clause_words.contains(&"end")))
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
    let has_combat_history = (clause_words.contains(&"dealt")
        && clause_words.contains(&"damage")
        && clause_words.contains(&"turn"))
        || clause_words
            .windows(2)
            .any(|window| matches!(window, ["was", "blocked"] | ["was", "blocking"]))
        || clause_words.windows(2).any(|window| {
            matches!(
                window,
                ["blocking", "it"] | ["blocked", "it"] | ["it", "blocked"]
            )
        });
    if has_combat_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history destroy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
        if let Some(attached_idx) = core_tokens
            .iter()
            .position(|token| token.is_word("attached"))
            && core_tokens
                .get(attached_idx + 1)
                .is_some_and(|token| token.is_word("to"))
            && attached_idx > 1
        {
            let mut filter_tokens = trim_commas(&core_tokens[1..attached_idx]).to_vec();
            while filter_tokens
                .last()
                .and_then(Token::as_word)
                .is_some_and(|word| matches!(word, "that" | "were" | "was" | "is" | "are"))
            {
                filter_tokens.pop();
            }
            let target_tokens = trim_commas(&core_tokens[attached_idx + 2..]);
            let target_words = words(&target_tokens);
            let has_timing_tail = target_words.iter().any(|word| {
                matches!(
                    *word,
                    "at" | "beginning" | "end" | "combat" | "turn" | "step" | "until"
                )
            });
            let supported_target = target_words.starts_with(&["target"])
                || target_words == ["it"]
                || target_words.starts_with(&["that", "creature"])
                || target_words.starts_with(&["that", "permanent"])
                || target_words.starts_with(&["that", "land"])
                || target_words.starts_with(&["that", "artifact"])
                || target_words.starts_with(&["that", "enchantment"]);
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
        if let Some(except_for_idx) = core_tokens
            .windows(2)
            .position(|window| window[0].is_word("except") && window[1].is_word("for"))
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

    if clause_words.contains(&"unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported destroy-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(if_idx) = core_tokens.iter().position(|token| token.is_word("if")) {
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

        if let Some(instead_if_idx) = predicate_tokens
            .windows(2)
            .position(|window| window[0].is_word("instead") && window[1].is_word("if"))
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
    if let Some(and_idx) = core_tokens.iter().position(|token| token.is_word("and")) {
        let tail_words = words(&core_tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (tail_words.starts_with(&["up", "to"]) && tail_words.contains(&"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if clause_words.starts_with(&["target", "blocked"]) {
        let mut target_tokens = core_tokens.to_vec();
        if let Some(blocked_idx) = target_tokens
            .iter()
            .position(|token| token.is_word("blocked"))
        {
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
    tokens: &[Token],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(that_idx) = clause_words
        .windows(6)
        .position(|window| window == ["that", "was", "dealt", "damage", "this", "turn"])
    else {
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
        if !base.excluded_card_types.contains(&card_type) {
            base.excluded_card_types.push(card_type);
        }
    }
    for subtype in exception.subtypes.iter().copied() {
        if !base.excluded_subtypes.contains(&subtype) {
            base.excluded_subtypes.push(subtype);
        }
    }
}

pub(crate) fn parse_exile(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let (tokens, face_down) = split_exile_face_down_suffix(tokens);
    let clause_words = words(tokens);
    let has_face_down_manifest_tail = (clause_words.contains(&"face-down")
        || clause_words.contains(&"facedown")
        || clause_words.contains(&"manifest")
        || clause_words.contains(&"pile"))
        && clause_words.contains(&"then");
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

    if clause_words.contains(&"dealt")
        && clause_words.contains(&"damage")
        && clause_words.contains(&"turn")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_until_total_mana_value = clause_words.contains(&"until")
        && clause_words.contains(&"exiled")
        && clause_words.contains(&"total")
        && clause_words.contains(&"mana")
        && clause_words.contains(&"value");
    if has_until_total_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported iterative exile-total clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_attached_bundle = clause_words.contains(&"and")
        && clause_words.contains(&"all")
        && clause_words.contains(&"attached");
    if has_attached_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported attached-object exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_same_name_token_bundle = clause_words.contains(&"and")
        && clause_words.contains(&"tokens")
        && clause_words.contains(&"same")
        && clause_words.contains(&"name");
    if has_same_name_token_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported same-name token exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and"))
        && and_idx > 0
    {
        let tail_words = words(&tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (tail_words.starts_with(&["up", "to"]) && tail_words.contains(&"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target exile clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if let Some(if_idx) = tokens.iter().position(|token| token.is_word("if"))
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
    tokens: &[Token],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !(clause_words.starts_with(&["all", "cards"]) || clause_words.starts_with(&["all", "card"]))
        || !clause_words
            .windows(3)
            .any(|window| window == ["with", "that", "name"])
    {
        return Ok(None);
    }

    let Some(from_idx) = clause_words.iter().position(|word| *word == "from") else {
        return Ok(None);
    };
    let Some(first_zone_idx) = (from_idx + 1..clause_words.len()).find(|idx| {
        matches!(
            clause_words[*idx],
            "hand" | "hands" | "graveyard" | "graveyards"
        )
    }) else {
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
        if !matches!(zone, Zone::Hand | Zone::Graveyard) || zones.contains(&zone) {
            continue;
        }
        zones.push(zone);
    }
    if zones.len() != 2 || !zones.contains(&Zone::Hand) || !zones.contains(&Zone::Graveyard) {
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

pub(crate) fn split_until_source_leaves_tail(tokens: &[Token]) -> (&[Token], bool) {
    let Some(until_idx) = tokens.iter().rposition(|token| token.is_word("until")) else {
        return (tokens, false);
    };
    if until_idx == 0 {
        return (tokens, false);
    }
    let tail_words = words(&tokens[until_idx + 1..]);
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

pub(crate) fn split_exile_face_down_suffix(tokens: &[Token]) -> (&[Token], bool) {
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

pub(crate) fn parse_target_player_graveyard_filter(tokens: &[Token]) -> Option<ObjectFilter> {
    let words = words(tokens);
    if words.as_slice() == ["target", "player", "graveyard"]
        || words.as_slice() == ["target", "players", "graveyard"]
        || words.as_slice() == ["that", "player", "graveyard"]
        || words.as_slice() == ["that", "players", "graveyard"]
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::target_player());
        return Some(filter);
    }
    if words.as_slice() == ["target", "opponent", "graveyard"]
        || words.as_slice() == ["target", "opponents", "graveyard"]
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)));
        return Some(filter);
    }
    if words.as_slice() == ["its", "controller", "graveyard"]
        || words.as_slice() == ["its", "controllers", "graveyard"]
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged("triggering"),
        ));
        return Some(filter);
    }
    if words.as_slice() == ["its", "owner", "graveyard"]
        || words.as_slice() == ["its", "owners", "graveyard"]
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
            "triggering",
        )));
        return Some(filter);
    }
    None
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

pub(crate) fn parse_untap(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "untap clause missing target".to_string(),
        ));
    }
    let words = words(tokens);
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
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing scry count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(EffectAst::Scry { count, player })
}

pub(crate) fn parse_surveil(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing surveil count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(EffectAst::Surveil { count, player })
}

pub(crate) fn parse_pay(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = words(tokens);
    if clause_words.starts_with(&["any", "amount", "of"]) && clause_words.contains(&"e") {
        return Ok(EffectAst::PayEnergy {
            amount: Value::Fixed(0),
            player,
        });
    }
    if clause_words.len() >= 4
        && clause_words.contains(&"for")
        && clause_words.contains(&"each")
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
    if tokens.iter().any(|token| token.is_word("e")) {
        let mut energy_count = 0u32;
        for token in tokens {
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
                words(tokens).join(" ")
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
            words(tokens).join(" ")
        )));
    }

    if pips.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing payment cost (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::PayMana {
        cost: ManaCost::from_pips(pips),
        player,
    })
}

pub(crate) fn parse_filter_comparison_tokens(
    axis: &str,
    tokens: &[&str],
    clause_words: &[&str],
) -> Result<Option<(crate::filter::Comparison, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let to_comparison = |kind: &str, operand: Value| -> crate::filter::Comparison {
        use crate::filter::Comparison;

        match (kind, operand) {
            ("eq", Value::Fixed(value)) => Comparison::Equal(value),
            ("neq", Value::Fixed(value)) => Comparison::NotEqual(value),
            ("lt", Value::Fixed(value)) => Comparison::LessThan(value),
            ("lte", Value::Fixed(value)) => Comparison::LessThanOrEqual(value),
            ("gt", Value::Fixed(value)) => Comparison::GreaterThan(value),
            ("gte", Value::Fixed(value)) => Comparison::GreaterThanOrEqual(value),
            ("eq", operand) => Comparison::EqualExpr(Box::new(operand)),
            ("neq", operand) => Comparison::NotEqualExpr(Box::new(operand)),
            ("lt", operand) => Comparison::LessThanExpr(Box::new(operand)),
            ("lte", operand) => Comparison::LessThanOrEqualExpr(Box::new(operand)),
            ("gt", operand) => Comparison::GreaterThanExpr(Box::new(operand)),
            ("gte", operand) => Comparison::GreaterThanOrEqualExpr(Box::new(operand)),
            _ => unreachable!("unsupported comparison kind"),
        }
    };

    let parse_operand = |operand_tokens: &[&str],
                         comparison_kind: &str|
     -> Result<(crate::filter::Comparison, usize), CardTextError> {
        let Some((operand, used)) = parse_value_expr_words(operand_tokens) else {
            let quoted = operand_tokens
                .first()
                .copied()
                .unwrap_or_default()
                .to_string();
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        Ok((to_comparison(comparison_kind, operand), used))
    };

    let parse_numeric_token = |word: &str| -> Option<i32> {
        if let Ok(value) = word.parse::<i32>() {
            return Some(value);
        }
        parse_number_word_i32(word)
    };

    let first = tokens[0];
    if let Some(value) = parse_numeric_token(first) {
        if tokens
            .get(1)
            .is_some_and(|word| matches!(*word, "plus" | "minus"))
        {
            let (cmp, used) = parse_operand(tokens, "eq")?;
            return Ok(Some((cmp, used)));
        }
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| matches!(*word, "greater" | "more"))
        {
            return Ok(Some((
                crate::filter::Comparison::GreaterThanOrEqual(value),
                3,
            )));
        }
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| matches!(*word, "less" | "fewer"))
        {
            return Ok(Some((crate::filter::Comparison::LessThanOrEqual(value), 3)));
        }
        let mut values = vec![value];
        let mut consumed = 1usize;
        while consumed < tokens.len() {
            let token = tokens[consumed];
            if matches!(token, "and" | "or" | "and/or") {
                consumed += 1;
                continue;
            }
            if let Some(next_value) = parse_numeric_token(token) {
                values.push(next_value);
                consumed += 1;
                continue;
            }
            break;
        }
        if values.len() > 1 {
            return Ok(Some((crate::filter::Comparison::OneOf(values), consumed)));
        }
        return Ok(Some((crate::filter::Comparison::Equal(value), 1)));
    }

    if first == "equal" && tokens.get(1) == Some(&"to") {
        let Some(operand) = tokens.get(2).copied() else {
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after 'equal to' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let _ = operand;
        let (cmp, used) = parse_operand(&tokens[2..], "eq")?;
        return Ok(Some((cmp, 2 + used)));
    }

    if matches!(first, "less" | "greater") && tokens.get(1) == Some(&"than") {
        let mut operand_idx = 2usize;
        let mut inclusive = false;
        if tokens.get(operand_idx) == Some(&"or")
            && tokens.get(operand_idx + 1) == Some(&"equal")
            && tokens.get(operand_idx + 2) == Some(&"to")
        {
            inclusive = true;
            operand_idx += 3;
        }
        let Some(operand) = tokens.get(operand_idx).copied() else {
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after '{first} than' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let _ = operand;
        let (cmp, used) = parse_operand(
            &tokens[operand_idx..],
            match (first, inclusive) {
                ("less", true) => "lte",
                ("less", false) => "lt",
                ("greater", true) => "gte",
                ("greater", false) => "gt",
                _ => unreachable!("first is constrained above"),
            },
        )?;
        let parsed = match (first, inclusive, cmp) {
            ("less", true, crate::filter::Comparison::Equal(v)) => {
                crate::filter::Comparison::LessThanOrEqual(v)
            }
            ("less", false, crate::filter::Comparison::Equal(v)) => {
                crate::filter::Comparison::LessThan(v)
            }
            ("greater", true, crate::filter::Comparison::Equal(v)) => {
                crate::filter::Comparison::GreaterThanOrEqual(v)
            }
            ("greater", false, crate::filter::Comparison::Equal(v)) => {
                crate::filter::Comparison::GreaterThan(v)
            }
            (_, _, other) => other,
        };
        return Ok(Some((parsed, operand_idx + used)));
    }

    if first == "not" && tokens.get(1) == Some(&"equal") && tokens.get(2) == Some(&"to") {
        let (cmp, used) = parse_operand(&tokens[3..], "neq")?;
        return Ok(Some((cmp, 3 + used)));
    }

    if first == "equal" && tokens.get(1) == Some(&"to") && tokens.get(2) == Some(&"x") {
        return Ok(Some((crate::filter::Comparison::GreaterThanOrEqual(0), 3)));
    }

    if let Some((value, used)) = parse_value_expr_words(tokens) {
        if let Value::Fixed(fixed) = value
            && used == 1
        {
            return Ok(Some((crate::filter::Comparison::Equal(fixed), used)));
        }
        return Ok(Some((
            crate::filter::Comparison::EqualExpr(Box::new(value)),
            used,
        )));
    }

    if first == "x" {
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| matches!(*word, "less" | "fewer" | "greater" | "more"))
        {
            return Ok(Some((crate::filter::Comparison::GreaterThanOrEqual(0), 3)));
        }
        return Ok(Some((crate::filter::Comparison::GreaterThanOrEqual(0), 1)));
    }

    Ok(None)
}
