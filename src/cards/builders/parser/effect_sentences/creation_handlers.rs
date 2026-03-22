use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, ObjectRefAst, OwnedLexToken, PlayerAst, SubjectAst, TagKey,
    TargetAst,
};
use crate::color::ColorSet;
use crate::effect::{EventValueSpec, Value};
use crate::static_abilities::{Anthem, AnthemCountExpression, AnthemValue, StaticAbility};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::super::object_filters::parse_object_filter;
use super::super::util::{
    is_article, parse_card_type, parse_color, parse_number, parse_target_phrase, parse_value,
    token_index_for_word_index, trim_commas, words,
};
use super::clause_pattern_helpers::extract_subject_player;
use super::conditionals::parse_subtype_word;
use super::dispatch_entry::target_references_it;
use super::lex_chain_helpers::starts_with_inline_token_rules_tail;

pub(crate) fn looks_like_pt_word(word: &str) -> bool {
    let Some((power, toughness)) = word.split_once('/') else {
        return false;
    };
    let is_component = |part: &str| {
        let part = part.trim_matches(|ch| matches!(ch, '+' | '-'));
        part == "x" || part == "*" || part.parse::<i32>().is_ok()
    };
    is_component(power) && is_component(toughness)
}

pub(crate) fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    let (power, toughness) = word.split_once('/')?;
    if power.starts_with('+')
        || toughness.starts_with('+')
        || power.starts_with('-')
        || toughness.starts_with('-')
    {
        return None;
    }
    let power = power.parse::<i32>().ok()?;
    let toughness = toughness.parse::<i32>().ok()?;
    Some((power, toughness))
}

pub(crate) fn is_probable_token_name_word(word: &str) -> bool {
    if !word
        .chars()
        .all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-')
    {
        return false;
    }
    !matches!(
        word,
        "legendary"
            | "artifact"
            | "enchantment"
            | "creature"
            | "token"
            | "tokens"
            | "white"
            | "blue"
            | "black"
            | "red"
            | "green"
            | "colorless"
    )
}

pub(crate) fn parse_copy_modifiers_from_tail(
    tail_words: &[&str],
) -> (
    Option<ColorSet>,
    Option<Vec<CardType>>,
    Option<Vec<Subtype>>,
    Vec<CardType>,
    Vec<Subtype>,
    Vec<Supertype>,
    Option<(i32, i32)>,
    Vec<StaticAbility>,
) {
    let mut set_colors = None;
    let mut set_card_types = None;
    let mut set_subtypes = None;
    let mut added_card_types = Vec::new();
    let mut added_subtypes = Vec::new();
    let mut removed_supertypes = Vec::new();
    let mut set_base_power_toughness = None;
    let mut granted_abilities = Vec::new();

    let except_idx = tail_words.iter().rposition(|word| *word == "except");
    let modifier_words = except_idx
        .map(|idx| &tail_words[idx + 1..])
        .unwrap_or_default();
    if modifier_words.is_empty() {
        return (
            set_colors,
            set_card_types,
            set_subtypes,
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        );
    }

    if modifier_words
        .windows(2)
        .any(|window| window == ["isnt", "legendary"] || window == ["isn't", "legendary"])
        || modifier_words
            .windows(3)
            .any(|window| window == ["is", "not", "legendary"])
    {
        removed_supertypes.push(Supertype::Legendary);
    }

    if let Some((power, toughness)) = modifier_words
        .iter()
        .find_map(|word| parse_unsigned_pt_word(word))
    {
        set_base_power_toughness = Some((power, toughness));
    }

    let has_grant_verb = modifier_words.contains(&"has")
        || modifier_words.contains(&"have")
        || modifier_words.contains(&"gain")
        || modifier_words.contains(&"gains");
    let has_modifier_keyword = |keyword: &str| {
        modifier_words
            .windows(2)
            .any(|window| window == ["with", keyword])
            || (has_grant_verb && modifier_words.contains(&keyword))
    };
    if has_modifier_keyword("flying") {
        granted_abilities.push(StaticAbility::flying());
    }
    if has_modifier_keyword("trample") {
        granted_abilities.push(StaticAbility::trample());
    }
    if let Some(idx) = modifier_words
        .windows(6)
        .position(|window| window == ["this", "token", "gets", "+1/+1", "for", "each"])
        .or_else(|| {
            modifier_words
                .windows(6)
                .position(|window| window == ["this", "creature", "gets", "+1/+1", "for", "each"])
        })
    {
        let mut tail = modifier_words.get(idx + 6..).unwrap_or_default();
        while tail
            .first()
            .is_some_and(|word| is_article(word) || matches!(*word, "a" | "an" | "the"))
        {
            tail = &tail[1..];
        }
        if let Some(subtype_word) = tail.first().copied() {
            let subtype = parse_subtype_word(subtype_word)
                .or_else(|| subtype_word.strip_suffix('s').and_then(parse_subtype_word));
            let you_control = tail.windows(2).any(|window| window == ["you", "control"]);
            if let Some(subtype) = subtype
                && you_control
            {
                let mut filter = ObjectFilter::default();
                filter.zone = Some(Zone::Battlefield);
                filter.controller = Some(PlayerFilter::You);
                filter.subtypes = vec![subtype];
                let count = AnthemCountExpression::MatchingFilter(filter);
                let anthem = Anthem::for_source(0, 0).with_values(
                    AnthemValue::scaled(1, count.clone()),
                    AnthemValue::scaled(1, count),
                );
                granted_abilities.push(StaticAbility::new(anthem));
            }
        }
    }

    let addition_idx = modifier_words.windows(6).position(|window| {
        window == ["in", "addition", "to", "its", "other", "types"]
            || window == ["in", "addition", "to", "their", "other", "types"]
    });
    if let Some(addition_idx) = addition_idx {
        let descriptor_words = &modifier_words[..addition_idx];
        for word in descriptor_words {
            if let Some(card_type) = parse_card_type(word)
                && !added_card_types.contains(&card_type)
            {
                added_card_types.push(card_type);
            }
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !added_subtypes.contains(&subtype)
            {
                added_subtypes.push(subtype);
            }
        }
    } else {
        let starts_with_identity_clause = modifier_words.starts_with(&["its"])
            || modifier_words.starts_with(&["it", "is"])
            || modifier_words.starts_with(&["it", "s"])
            || modifier_words.starts_with(&["it's"])
            || modifier_words.starts_with(&["it’s"])
            || modifier_words.starts_with(&["theyre"])
            || modifier_words.starts_with(&["they", "re"])
            || modifier_words.starts_with(&["they're"])
            || modifier_words.starts_with(&["they’re"])
            || modifier_words.starts_with(&["they", "are"]);
        if starts_with_identity_clause {
            let descriptor_end = modifier_words
                .iter()
                .position(|word| matches!(*word, "with" | "has" | "have" | "gain" | "gains"))
                .unwrap_or(modifier_words.len());
            let descriptor_words = &modifier_words[..descriptor_end];
            let mut colors = ColorSet::new();
            let mut card_types = Vec::new();
            let mut subtypes = Vec::new();
            for word in descriptor_words {
                if is_article(word)
                    || matches!(
                        *word,
                        "its"
                            | "it"
                            | "is"
                            | "s"
                            | "it's"
                            | "it’s"
                            | "they"
                            | "are"
                            | "re"
                            | "theyre"
                            | "they're"
                            | "they’re"
                    )
                    || looks_like_pt_word(word)
                {
                    continue;
                }
                if let Some(color) = parse_color(word) {
                    colors = colors.union(color);
                }
                if let Some(card_type) = parse_card_type(word)
                    && !card_types.contains(&card_type)
                {
                    card_types.push(card_type);
                }
                if let Some(subtype) = parse_subtype_word(word)
                    .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                    && !subtypes.contains(&subtype)
                {
                    subtypes.push(subtype);
                }
            }
            if !colors.is_empty() {
                set_colors = Some(colors);
            }
            if !card_types.is_empty() {
                set_card_types = Some(card_types);
            }
            if !subtypes.is_empty() {
                set_subtypes = Some(subtypes);
            }
        }
    }

    (
        set_colors,
        set_card_types,
        set_subtypes,
        added_card_types,
        added_subtypes,
        removed_supertypes,
        set_base_power_toughness,
        granted_abilities,
    )
}

pub(crate) fn parse_next_end_step_token_delay_flags(tail_words: &[&str]) -> (bool, bool) {
    let has_beginning_of_end_step = tail_words
        .windows(6)
        .any(|window| window == ["beginning", "of", "the", "next", "end", "step"])
        || tail_words
            .windows(5)
            .any(|window| window == ["beginning", "of", "next", "end", "step"])
        || tail_words
            .windows(5)
            .any(|window| window == ["beginning", "of", "the", "end", "step"])
        || tail_words
            .windows(4)
            .any(|window| window == ["beginning", "of", "end", "step"]);
    if !has_beginning_of_end_step {
        return (false, false);
    }

    let has_sacrifice_reference = tail_words.contains(&"sacrifice")
        && (tail_words.contains(&"token")
            || tail_words.contains(&"tokens")
            || tail_words.contains(&"permanent")
            || tail_words.contains(&"permanents")
            || tail_words.contains(&"it")
            || tail_words.contains(&"them"));
    let has_exile_reference = tail_words.contains(&"exile")
        && (tail_words.contains(&"token")
            || tail_words.contains(&"tokens")
            || tail_words.contains(&"permanent")
            || tail_words.contains(&"permanents")
            || tail_words.contains(&"it")
            || tail_words.contains(&"them"));

    (has_sacrifice_reference, has_exile_reference)
}

pub(crate) fn trailing_create_at_next_end_step_clause(
    tail_words: &[&str],
) -> Option<(usize, PlayerFilter)> {
    let suffixes: &[(&[&str], PlayerFilter)] = &[
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "end",
                "step",
            ],
            PlayerFilter::You,
        ),
        (
            &["at", "the", "beginning", "of", "the", "next", "end", "step"],
            PlayerFilter::Any,
        ),
        (
            &["at", "the", "beginning", "of", "next", "end", "step"],
            PlayerFilter::Any,
        ),
        (
            &["at", "the", "beginning", "of", "the", "end", "step"],
            PlayerFilter::Any,
        ),
        (
            &["at", "the", "beginning", "of", "end", "step"],
            PlayerFilter::Any,
        ),
    ];

    for (suffix, player) in suffixes {
        if tail_words.len() < suffix.len() {
            continue;
        }
        let start = tail_words.len() - suffix.len();
        if tail_words[start..] != **suffix {
            continue;
        }
        if tail_words[..start]
            .iter()
            .any(|word| matches!(*word, "when" | "whenever"))
        {
            continue;
        }
        return Some((start, player.clone()));
    }

    None
}

pub(crate) fn split_copy_source_tail_modifiers(
    source_tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, bool, bool) {
    let mut split_idx: Option<usize> = None;
    for idx in 0..source_tokens.len() {
        if !source_tokens[idx].is_word("and") {
            continue;
        }
        let tail_tokens = trim_commas(&source_tokens[idx + 1..]);
        let tail_words = words(&tail_tokens);
        if tail_words.is_empty() {
            continue;
        }
        let starts_reference = matches!(
            tail_words.first().copied(),
            Some("that" | "it" | "those" | "thats" | "its")
        );
        if !starts_reference {
            continue;
        }
        if !tail_words.contains(&"tapped") && !tail_words.contains(&"attacking") {
            continue;
        }
        split_idx = Some(idx);
        break;
    }

    let Some(split_idx) = split_idx else {
        return (source_tokens.to_vec(), false, false);
    };

    let modifier_tokens = trim_commas(&source_tokens[split_idx + 1..]);
    let modifier_words = words(&modifier_tokens);
    let enters_tapped = modifier_words.contains(&"tapped");
    let enters_attacking = modifier_words.contains(&"attacking");
    let source_tokens = trim_commas(&source_tokens[..split_idx]).to_vec();
    (source_tokens, enters_tapped, enters_attacking)
}

pub(crate) fn split_copy_source_inline_combat_modifiers(
    source_tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, bool, bool, Option<PlayerAst>) {
    let source_words = words(source_tokens);
    let modifier_start_word_idx = source_words
        .iter()
        .position(|word| *word == "thats")
        .or_else(|| {
            source_words
                .windows(2)
                .position(|window| window == ["that", "is"] || window == ["that", "are"])
        });

    let Some(modifier_start_word_idx) = modifier_start_word_idx else {
        return (source_tokens.to_vec(), false, false, None);
    };

    let modifier_words = &source_words[modifier_start_word_idx..];
    let enters_tapped = modifier_words.contains(&"tapped");
    let enters_attacking = modifier_words.contains(&"attacking");
    if !enters_tapped && !enters_attacking {
        return (source_tokens.to_vec(), false, false, None);
    }

    let attack_target_player_or_planeswalker_controlled_by = modifier_words
        .windows(7)
        .any(|window| {
            matches!(
                window,
                [
                    "that",
                    "player",
                    "or",
                    "a",
                    "planeswalker",
                    "they",
                    "control"
                ] | ["that", "player", "or", "planeswalker", "they", "control"]
                    | [
                        "that",
                        "player",
                        "or",
                        "a",
                        "planeswalker",
                        "they",
                        "controls"
                    ]
                    | ["that", "player", "or", "planeswalker", "they", "controls"]
                    | [
                        "that",
                        "player",
                        "or",
                        "a",
                        "planeswalker",
                        "their",
                        "control"
                    ]
                    | ["that", "player", "or", "planeswalker", "their", "control"]
            )
        })
        .then_some(PlayerAst::That);

    let Some(modifier_start_token_idx) =
        token_index_for_word_index(source_tokens, modifier_start_word_idx)
    else {
        return (
            source_tokens.to_vec(),
            enters_tapped,
            enters_attacking,
            attack_target_player_or_planeswalker_controlled_by,
        );
    };
    let source_tokens = trim_commas(&source_tokens[..modifier_start_token_idx]).to_vec();
    (
        source_tokens,
        enters_tapped,
        enters_attacking,
        attack_target_player_or_planeswalker_controlled_by,
    )
}

pub(crate) fn parse_create(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let clause_words = words(tokens);
    let has_unsupported_dynamic_count = clause_words.starts_with(&["a", "number", "of"])
        || clause_words.starts_with(&["the", "number", "of"]);
    if has_unsupported_dynamic_count {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic token count in create clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut idx = 0;
    let mut count_value = Value::Fixed(1);
    if tokens.first().is_some_and(|token| token.is_word("that"))
        && tokens.get(1).is_some_and(|token| token.is_word("many"))
    {
        count_value = Value::EventValue(EventValueSpec::Amount);
        idx = 2;
    } else if tokens.first().is_some_and(|token| token.is_word("x")) {
        count_value = Value::X;
        idx = 1;
    } else if let Some((parsed_count, used)) = parse_number(tokens) {
        count_value = Value::Fixed(parsed_count as i32);
        idx = used;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        idx += 1;
    }

    let remaining_words = words(&tokens[idx..]);
    let token_idx = remaining_words
        .iter()
        .position(|word| *word == "token" || *word == "tokens")
        .ok_or_else(|| CardTextError::ParseError("create clause missing token".to_string()))?;

    let mut name_words: Vec<&str> = remaining_words[..token_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    let mut tail_tokens = tokens[idx + token_idx + 1..].to_vec();
    let mut delayed_create_player = None;
    let initial_tail_words = words(&tail_tokens);
    if let Some((clause_start, player)) =
        trailing_create_at_next_end_step_clause(&initial_tail_words)
    {
        delayed_create_player = Some(player);
        if let Some(cut_idx) = token_index_for_word_index(&tail_tokens, clause_start) {
            tail_tokens.truncate(cut_idx);
        }
    }
    let mut attached_to_target: Option<TargetAst> = None;
    let pre_attach_tail_words = words(&tail_tokens);
    let pre_attach_for_each_idx = pre_attach_tail_words
        .windows(2)
        .position(|window| window == ["for", "each"]);
    if let Some(attached_word_idx) = pre_attach_tail_words
        .iter()
        .position(|word| *word == "attached")
        && pre_attach_tail_words.get(attached_word_idx + 1) == Some(&"to")
        && (pre_attach_for_each_idx.is_none()
            || pre_attach_for_each_idx.is_some_and(|for_each_idx| attached_word_idx < for_each_idx))
        && let Some(attached_token_idx) =
            token_index_for_word_index(&tail_tokens, attached_word_idx)
    {
        let target_tokens = trim_commas(&tail_tokens[attached_token_idx + 2..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing attachment target in create clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        attached_to_target = Some(parse_target_phrase(&target_tokens)?);
        tail_tokens.truncate(attached_token_idx);
    }
    let tail_words = words(&tail_tokens);
    if attached_to_target.is_some()
        && tail_words
            .iter()
            .any(|word| *word == "copy" || *word == "copies")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported aura-copy attachment fanout clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let with_idx = tail_words.iter().position(|word| *word == "with");
    let raw_for_each_idx = tail_words
        .windows(2)
        .position(|window| window == ["for", "each"]);
    let for_each_idx = raw_for_each_idx.filter(|idx| {
        let prefix_words = &tail_words[..*idx];
        let looks_like_token_rules_text = prefix_words.windows(2).any(|window| {
            matches!(
                window,
                ["it", "has"]
                    | ["it", "gains"]
                    | ["it", "gets"]
                    | ["this", "token"]
                    | ["that", "token"]
            )
        }) || (prefix_words.contains(&"token")
            && (prefix_words.contains(&"has")
                || prefix_words.contains(&"have")
                || prefix_words.contains(&"gets")
                || prefix_words.contains(&"gains")));
        if looks_like_token_rules_text {
            return false;
        }

        let Some(with_idx) = with_idx else {
            return true;
        };
        if with_idx >= *idx {
            return true;
        }
        let between_with_and_for_each = &tail_words[with_idx + 1..*idx];
        let has_rules_text_hint = between_with_and_for_each.iter().any(|word| {
            matches!(
                *word,
                "this"
                    | "that"
                    | "it"
                    | "token"
                    | "tokens"
                    | "gets"
                    | "get"
                    | "gains"
                    | "gain"
                    | "has"
                    | "have"
                    | "when"
                    | "whenever"
                    | "at"
                    | "sacrifice"
                    | "draw"
                    | "add"
                    | "deals"
                    | "deal"
                    | "counter"
                    | "counters"
            )
        });
        !has_rules_text_hint
    });
    let mut for_each_dynamic_count: Option<Value> = None;
    let mut for_each_object_filter: Option<ObjectFilter> = None;
    if let Some(for_each_idx) = for_each_idx {
        let filter_tokens = &tail_tokens[for_each_idx + 2..];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'for each' in create clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(dynamic) = parse_create_for_each_dynamic_count(filter_tokens) {
            for_each_dynamic_count = Some(dynamic);
        } else {
            let filter = parse_object_filter(filter_tokens, false)?;
            for_each_object_filter = Some(filter);
        }
    }
    let resolve_create_count = |references_iterated_object: bool| {
        if let Some(dynamic) = for_each_dynamic_count.clone() {
            return dynamic;
        }
        if let Some(filter) = for_each_object_filter.clone() {
            if references_iterated_object {
                return count_value.clone();
            }
            return Value::Count(filter);
        }
        count_value.clone()
    };
    let wrap_for_each_when_needed = |effect: EffectAst, references_iterated_object: bool| {
        if references_iterated_object && let Some(filter) = for_each_object_filter.clone() {
            EffectAst::ForEachObject {
                filter,
                effects: vec![effect],
            }
        } else {
            effect
        }
    };
    let wrap_delayed_create = |effect: EffectAst| {
        if let Some(player) = delayed_create_player {
            EffectAst::DelayedUntilNextEndStep {
                player,
                effects: vec![effect],
            }
        } else {
            effect
        }
    };
    let mut tapped = false;
    let mut attacking = false;
    let mut modifier_tail_words = tail_words.clone();
    let mut rules_text_range: Option<(usize, usize)> = None;
    if let Some(named_idx) = tail_words.iter().position(|word| *word == "named") {
        let range_end = for_each_idx.unwrap_or(tail_words.len());
        if named_idx + 1 < range_end {
            let after_named = &tail_words[named_idx + 1..range_end];
            let name_end = after_named
                .iter()
                .position(|word| matches!(*word, "with" | "that" | "which" | "thats"))
                .map(|offset| named_idx + 1 + offset)
                .unwrap_or(range_end);
            if named_idx + 1 < name_end {
                name_words.push("named");
                name_words.extend(tail_words[named_idx + 1..name_end].iter().copied());
            }
        }
    }
    name_words.retain(|word| {
        if *word == "tapped" {
            tapped = true;
            return false;
        }
        if *word == "attacking" {
            attacking = true;
            return false;
        }
        true
    });
    name_words.retain(|word| !matches!(*word, "and" | "or"));
    let name_words_primary_len = name_words.len();
    if name_words.is_empty() {
        if tail_words
            .iter()
            .any(|word| *word == "copy" || *word == "copies")
        {
            let (
                set_colors,
                set_card_types,
                set_subtypes,
                added_card_types,
                added_subtypes,
                removed_supertypes,
                set_base_power_toughness,
                granted_abilities,
            ) = parse_copy_modifiers_from_tail(&tail_words);
            let half_pt = tail_words.contains(&"half")
                && tail_words.contains(&"power")
                && tail_words.contains(&"toughness");
            let has_haste = tail_words.windows(2).any(|window| {
                matches!(
                    window,
                    ["has", "haste"] | ["gain", "haste"] | ["gains", "haste"]
                )
            }) || tail_words.contains(&"haste");
            let mut enters_tapped = false;
            let mut enters_attacking = false;
            let mut attack_target_player_or_planeswalker_controlled_by = None;
            let (sacrifice_at_next_end_step, exile_at_next_end_step) =
                parse_next_end_step_token_delay_flags(&tail_words);
            if let Some(of_idx) = tail_tokens.iter().position(|token| token.is_word("of")) {
                let source_tokens = &tail_tokens[of_idx + 1..];
                let source_end = source_tokens
                    .iter()
                    .position(|token| token.is_comma() || token.is_word("except"))
                    .unwrap_or(source_tokens.len());
                let mut source_end = source_end;
                for idx in 1..source_end {
                    let remaining_words = words(&source_tokens[idx..]);
                    if starts_with_inline_token_rules_tail(&remaining_words)
                        || (source_tokens[idx].is_word("and")
                            && starts_with_inline_token_rules_tail(&words(
                                &source_tokens[idx + 1..],
                            )))
                    {
                        source_end = idx;
                        break;
                    }
                }
                let source_tokens = &source_tokens[..source_end];
                let (source_tokens, tail_tapped, tail_attacking) =
                    split_copy_source_tail_modifiers(source_tokens);
                let (source_tokens, inline_tapped, inline_attacking, inline_attack_target_player) =
                    split_copy_source_inline_combat_modifiers(&source_tokens);
                enters_tapped = tail_tapped || inline_tapped;
                enters_attacking = tail_attacking || inline_attacking;
                attack_target_player_or_planeswalker_controlled_by = inline_attack_target_player;
                if !source_tokens.is_empty() {
                    let source = parse_target_phrase(&source_tokens)?;
                    let references_iterated_object = target_references_it(&source);
                    let create = EffectAst::CreateTokenCopyFromSource {
                        source,
                        count: resolve_create_count(references_iterated_object),
                        player,
                        enters_tapped,
                        enters_attacking,
                        attack_target_player_or_planeswalker_controlled_by,
                        half_power_toughness_round_up: half_pt,
                        has_haste,
                        exile_at_end_of_combat: false,
                        sacrifice_at_next_end_step,
                        exile_at_next_end_step,
                        set_colors,
                        set_card_types,
                        set_subtypes,
                        added_card_types,
                        added_subtypes,
                        removed_supertypes,
                        set_base_power_toughness,
                        granted_abilities,
                    };
                    return Ok(wrap_delayed_create(wrap_for_each_when_needed(
                        create,
                        references_iterated_object,
                    )));
                }
            }
            let references_iterated_object = true;
            let create = EffectAst::CreateTokenCopy {
                object: ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
                count: resolve_create_count(references_iterated_object),
                player,
                enters_tapped,
                enters_attacking,
                attack_target_player_or_planeswalker_controlled_by,
                half_power_toughness_round_up: half_pt,
                has_haste,
                exile_at_end_of_combat: false,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                set_colors,
                set_card_types,
                set_subtypes,
                added_card_types,
                added_subtypes,
                removed_supertypes,
                set_base_power_toughness,
                granted_abilities,
            };
            return Ok(wrap_delayed_create(wrap_for_each_when_needed(
                create,
                references_iterated_object,
            )));
        }
        return Err(CardTextError::ParseError(
            "create clause missing token name".to_string(),
        ));
    }
    if let Some(with_idx) = tail_words.iter().position(|word| *word == "with") {
        let with_tail_end = for_each_idx.unwrap_or(tail_words.len());
        if with_idx + 1 < with_tail_end {
            let with_words = &tail_words[with_idx + 1..with_tail_end];
            let rules_text_start = with_words.iter().position(|word| {
                matches!(
                    *word,
                    "when"
                        | "whenever"
                        | "if"
                        | "t"
                        | "this"
                        | "that"
                        | "it"
                        | "those"
                        | "sacrifice"
                        | "add"
                        | "draw"
                        | "deals"
                        | "deal"
                )
            });
            let mut include_end = rules_text_start.unwrap_or(with_words.len());
            if include_end > 0
                && let Some(named_pos) = with_words[..include_end]
                    .iter()
                    .position(|word| *word == "named")
            {
                include_end = named_pos;
            }
            let preserve_rules_tail = rules_text_start
                .is_some_and(|start| start < with_words.len())
                && with_words[include_end..].iter().any(|word| {
                    matches!(
                        *word,
                        "when"
                            | "whenever"
                            | "at"
                            | "sacrifice"
                            | "return"
                            | "counter"
                            | "draw"
                            | "add"
                            | "deals"
                            | "deal"
                            | "gets"
                            | "gain"
                            | "gains"
                            | "cant"
                            | "can"
                            | "block"
                    )
                });
            if preserve_rules_tail {
                let start = with_idx + 1 + include_end;
                if start < with_tail_end {
                    rules_text_range = Some((start, with_tail_end));
                }
            }
            if include_end > 0 {
                name_words.extend(with_words[..include_end].iter().copied());
                if preserve_rules_tail {
                    // Keep quoted token rules text tails so token lowering can
                    // reconstruct granted abilities instead of dropping them.
                    name_words.extend(with_words[include_end..].iter().copied());
                }
            } else {
                // Preserve quoted token rules text so token compilation can
                // attach the ability to the created token definition.
                name_words.extend(with_words.iter().copied());
            }
        }
    }
    let mut dynamic_power_toughness = None;
    if let Some(pt_idx) = name_words.iter().position(|word| looks_like_pt_word(word))
        && pt_idx < name_words_primary_len
    {
        if name_words[pt_idx].eq_ignore_ascii_case("x/x") {
            dynamic_power_toughness = Some((Value::X, Value::X));
            name_words[pt_idx] = "0/0";
        }
        let prefix_words = &name_words[..pt_idx];
        let keep_prefix = prefix_words.contains(&"legendary")
            || prefix_words
                .first()
                .is_some_and(|word| is_probable_token_name_word(word));
        if !keep_prefix {
            name_words = name_words[pt_idx..].to_vec();
        }
    }
    let name = normalize_token_name(&name_words);

    if let Some((start, end)) = rules_text_range {
        if start < end && end <= modifier_tail_words.len() {
            modifier_tail_words = modifier_tail_words[..start]
                .iter()
                .chain(modifier_tail_words[end..].iter())
                .copied()
                .collect();
        }
    }

    tapped |= modifier_tail_words.contains(&"tapped");
    attacking |= modifier_tail_words.contains(&"attacking");
    let (sacrifice_at_next_end_step, exile_at_next_end_step) =
        parse_next_end_step_token_delay_flags(&modifier_tail_words);
    let references_iterated_object = attached_to_target
        .as_ref()
        .is_some_and(target_references_it);
    let create = EffectAst::CreateTokenWithMods {
        name,
        count: resolve_create_count(references_iterated_object),
        dynamic_power_toughness,
        player,
        attached_to: attached_to_target,
        tapped,
        attacking,
        exile_at_end_of_combat: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step,
        exile_at_next_end_step,
    };
    Ok(wrap_delayed_create(wrap_for_each_when_needed(
        create,
        references_iterated_object,
    )))
}

pub(crate) fn parse_create_for_each_dynamic_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = words(tokens);
    if clause_words.starts_with(&["creature", "that", "died", "this", "turn"])
        || clause_words.starts_with(&["creatures", "that", "died", "this", "turn"])
    {
        return Some(Value::CreaturesDiedThisTurn);
    }
    if (clause_words.contains(&"spell") || clause_words.contains(&"spells"))
        && (clause_words.contains(&"cast") || clause_words.contains(&"casts"))
        && clause_words.contains(&"turn")
    {
        let player = if clause_words
            .iter()
            .any(|word| matches!(*word, "you" | "your" | "youve"))
        {
            PlayerFilter::You
        } else if clause_words
            .iter()
            .any(|word| matches!(*word, "opponent" | "opponents"))
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };

        let other_than_first = clause_words
            .windows(4)
            .any(|window| window == ["other", "than", "the", "first"]);
        if other_than_first {
            return Some(Value::Add(
                Box::new(Value::SpellsCastThisTurn(player)),
                Box::new(Value::Fixed(-1)),
            ));
        }
        if clause_words.contains(&"this") && clause_words.contains(&"turn") {
            return Some(Value::SpellsCastThisTurn(player));
        }
    }
    if clause_words.starts_with(&[
        "color", "of", "mana", "spent", "to", "cast", "this", "spell",
    ]) || clause_words.starts_with(&[
        "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
    ]) || clause_words
        .starts_with(&["color", "of", "mana", "used", "to", "cast", "this", "spell"])
        || clause_words.starts_with(&[
            "colors", "of", "mana", "used", "to", "cast", "this", "spell",
        ])
    {
        return Some(Value::ColorsOfManaSpentToCastThisSpell);
    }
    if clause_words.starts_with(&["basic", "land", "type", "among", "lands", "you", "control"])
        || clause_words.starts_with(&["basic", "land", "types", "among", "lands", "you", "control"])
        || clause_words.starts_with(&[
            "basic", "land", "type", "among", "the", "lands", "you", "control",
        ])
        || clause_words.starts_with(&[
            "basic", "land", "types", "among", "the", "lands", "you", "control",
        ])
    {
        return Some(Value::BasicLandTypesAmong(
            ObjectFilter::land().you_control(),
        ));
    }
    None
}

pub(crate) fn normalize_token_name(words: &[&str]) -> String {
    words.join(" ")
}

pub(crate) fn parse_investigate(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Investigate {
            count: Value::Fixed(1),
        });
    }

    let (count, used) = if let Some(first) = tokens.first().and_then(OwnedLexToken::as_word) {
        match first {
            "once" => (Value::Fixed(1), 1),
            "twice" => (Value::Fixed(2), 1),
            _ => parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing investigate count (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?,
        }
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing investigate count (clause: '{}')",
            words(tokens).join(" ")
        )));
    };

    let trailing = trim_commas(&tokens[used..]);
    let trailing_words = words(&trailing);
    let trailing_ok = trailing_words.is_empty()
        || trailing_words.as_slice() == ["time"]
        || trailing_words.as_slice() == ["times"];
    if !trailing_ok {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing investigate clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::Investigate { count })
}
