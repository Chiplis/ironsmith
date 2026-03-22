#![allow(dead_code)]

use crate::cards::builders::{
    CardTextError, EffectAst, KeywordAction, LineAst, StaticAbilityAst, TargetAst, TriggerSpec,
};
use crate::effect::Value;
use crate::target::PlayerFilter;

use super::activation_and_restrictions::{
    parse_ability_phrase, parse_single_word_keyword_action, parse_triggered_times_each_turn_lexed,
};
use super::keyword_static::parse_static_ability_ast_line_lexed;
use super::lexer::{OwnedLexToken, TokenKind, split_lexed_sentences};
use super::native_tokens::LowercaseWordView;
use super::util::{
    parse_card_type, parse_color, parse_flashback_keyword_line, parse_subtype_flexible,
    split_on_and, split_on_comma_or_semicolon, trim_commas, words,
};

fn parse_protection_chain(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let mut words = words(tokens);
    if words.first().copied() == Some("and") {
        words.remove(0);
    }
    if words.len() < 3 {
        return None;
    }
    if words[0] != "protection" || words[1] != "from" {
        return None;
    }

    let mut actions = Vec::new();
    let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
        let value = *words.get(idx + 1)?;
        match value {
            "the"
                if words.get(idx + 2).copied() == Some("chosen")
                    && words.get(idx + 3).copied() == Some("player") =>
            {
                Some(KeywordAction::ProtectionFromChosenPlayer)
            }
            "colorless" => Some(KeywordAction::ProtectionFromColorless),
            "everything" => Some(KeywordAction::ProtectionFromEverything),
            "all" if matches!(words.get(idx + 2).copied(), Some("color") | Some("colors")) => {
                Some(KeywordAction::ProtectionFromAllColors)
            }
            _ => parse_color(value)
                .map(KeywordAction::ProtectionFrom)
                .or_else(|| parse_card_type(value).map(KeywordAction::ProtectionFromCardType))
                .or_else(|| {
                    parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype)
                }),
        }
    };

    let mut from_count = 0usize;
    let mut parsed_count = 0usize;
    for idx in 0..words.len().saturating_sub(1) {
        if words[idx] != "from" {
            continue;
        }
        from_count += 1;
        if let Some(action) = parse_from_target(&words, idx) {
            parsed_count += 1;
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
    }

    if actions.is_empty() || parsed_count < from_count {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn rewrite_parse_ability_line(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }

    let segments = split_on_comma_or_semicolon(tokens);
    let mut actions = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(protection_actions) = parse_protection_chain(&segment) {
            actions.extend(protection_actions);
            continue;
        }

        if let Some(action) = parse_ability_phrase(&segment) {
            actions.push(action);
        } else {
            let and_parts = split_on_and(&segment);
            if and_parts.len() > 1 {
                let mut all_ok = true;
                for part in &and_parts {
                    if part.is_empty() {
                        continue;
                    }
                    if let Some(action) = parse_ability_phrase(part) {
                        actions.push(action);
                    } else {
                        all_ok = false;
                        break;
                    }
                }
                if !all_ok {
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn rewrite_parse_ability_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<KeywordAction>> {
    fn parse_simple_keyword_phrase_lexed(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
        let words_view = LowercaseWordView::new(tokens);
        let mut words = words_view.to_word_refs();
        if words.first().copied() == Some("and") {
            words.remove(0);
        }
        if words.is_empty() {
            return None;
        }

        if words.len() == 1 {
            return parse_single_word_keyword_action(words[0]);
        }

        let parse_count_keyword =
            |expected: &str, ctor: fn(u32) -> KeywordAction| -> Option<KeywordAction> {
                (words.first().copied() == Some(expected))
                    .then(|| words.get(1)?.parse::<u32>().ok().map(ctor))
                    .flatten()
            };

        if let Some(action) = parse_count_keyword("ward", KeywordAction::Ward) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("toxic", KeywordAction::Toxic) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("afterlife", KeywordAction::Afterlife) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("fabricate", KeywordAction::Fabricate) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("renown", KeywordAction::Renown) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("backup", KeywordAction::Backup) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("bushido", KeywordAction::Bushido) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("bloodthirst", KeywordAction::Bloodthirst) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("rampage", KeywordAction::Rampage) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("annihilator", KeywordAction::Annihilator) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("soulshift", KeywordAction::Soulshift) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("modular", KeywordAction::Modular) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("graft", KeywordAction::Graft) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("fading", KeywordAction::Fading) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("vanishing", KeywordAction::Vanishing) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("mobilize", KeywordAction::Mobilize) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("casualty", KeywordAction::Casualty) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("devour", KeywordAction::Devour) {
            return Some(action);
        }

        if words.len() == 2 {
            return match words.as_slice() {
                ["first", "strike"] => Some(KeywordAction::FirstStrike),
                ["double", "strike"] => Some(KeywordAction::DoubleStrike),
                ["battle", "cry"] => Some(KeywordAction::BattleCry),
                ["split", "second"] => Some(KeywordAction::SplitSecond),
                ["for", "mirrodin"] => Some(KeywordAction::ForMirrodin),
                ["living", "weapon"] => Some(KeywordAction::LivingWeapon),
                ["umbra", "armor"] => Some(KeywordAction::UmbraArmor),
                ["doctor", "companion"] => Some(KeywordAction::Marker("doctor companion")),
                _ => None,
            };
        }

        None
    }

    fn parse_flashback_keyword_line_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
        if !tokens
            .first()
            .is_some_and(|token| token.is_word("flashback"))
        {
            return None;
        }
        let mut idx = 1usize;
        let mut cost = String::new();
        while let Some(token) = tokens.get(idx) {
            if token.kind != TokenKind::ManaGroup {
                break;
            }
            cost.push_str(token.slice.as_str());
            idx += 1;
        }
        if cost.is_empty() {
            return None;
        }

        let tail_view = LowercaseWordView::new(&tokens[idx..]);
        let tail = tail_view.to_word_refs();
        let mut text = format!("Flashback {cost}");
        if !tail.is_empty() {
            let mut tail_text = tail.join(" ");
            if let Some(first) = tail_text.chars().next() {
                let upper = first.to_ascii_uppercase().to_string();
                let rest = &tail_text[first.len_utf8()..];
                tail_text = format!("{upper}{rest}");
            }
            text.push_str(", ");
            text.push_str(&tail_text);
        }
        Some(vec![KeywordAction::MarkerText(text)])
    }

    fn parse_protection_chain_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
        let words_view = LowercaseWordView::new(tokens);
        let mut words = words_view.to_word_refs();
        if words.first().copied() == Some("and") {
            words.remove(0);
        }
        if words.len() < 3 || words[0] != "protection" || words[1] != "from" {
            return None;
        }

        let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
            let value = *words.get(idx + 1)?;
            match value {
                "the"
                    if words.get(idx + 2).copied() == Some("chosen")
                        && words.get(idx + 3).copied() == Some("player") =>
                {
                    Some(KeywordAction::ProtectionFromChosenPlayer)
                }
                "colorless" => Some(KeywordAction::ProtectionFromColorless),
                "everything" => Some(KeywordAction::ProtectionFromEverything),
                "all" if matches!(words.get(idx + 2).copied(), Some("color") | Some("colors")) => {
                    Some(KeywordAction::ProtectionFromAllColors)
                }
                _ => parse_color(value)
                    .map(KeywordAction::ProtectionFrom)
                    .or_else(|| parse_card_type(value).map(KeywordAction::ProtectionFromCardType))
                    .or_else(|| {
                        parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype)
                    }),
            }
        };

        let mut actions = Vec::new();
        let mut from_count = 0usize;
        let mut parsed_count = 0usize;
        for idx in 0..words.len().saturating_sub(1) {
            if words[idx] != "from" {
                continue;
            }
            from_count += 1;
            if let Some(action) = parse_from_target(&words, idx) {
                parsed_count += 1;
                if !actions.contains(&action) {
                    actions.push(action);
                }
            }
        }

        if actions.is_empty() || parsed_count < from_count {
            None
        } else {
            Some(actions)
        }
    }

    fn split_on_lexed_comma_or_semicolon(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
        let mut segments = Vec::new();
        let mut start = 0usize;
        for (idx, token) in tokens.iter().enumerate() {
            if matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon) {
                if start < idx {
                    segments.push(&tokens[start..idx]);
                }
                start = idx + 1;
            }
        }
        if start < tokens.len() {
            segments.push(&tokens[start..]);
        }
        segments
    }

    fn split_on_lexed_and(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
        let mut segments = Vec::new();
        let mut start = 0usize;
        for (idx, token) in tokens.iter().enumerate() {
            if token.is_word("and") {
                let segment = &tokens[start..idx];
                if !segment.is_empty() {
                    segments.push(segment);
                }
                start = idx + 1;
            }
        }
        let tail = &tokens[start..];
        if !tail.is_empty() {
            segments.push(tail);
        }
        segments
    }

    if let Some(actions) = parse_flashback_keyword_line_lexed(tokens) {
        return Some(actions);
    }

    let segments = split_on_lexed_comma_or_semicolon(tokens);
    let mut actions = Vec::new();
    for segment in segments {
        if segment.is_empty() {
            continue;
        }
        if let Some(protection_actions) = parse_protection_chain_lexed(segment) {
            actions.extend(protection_actions);
            continue;
        }

        if let Some(action) = parse_simple_keyword_phrase_lexed(segment) {
            actions.push(action);
            continue;
        }

        let and_parts = split_on_lexed_and(segment);
        if and_parts.len() > 1 {
            let mut all_ok = true;
            for part in &and_parts {
                if part.is_empty() {
                    continue;
                }
                if let Some(action) = parse_simple_keyword_phrase_lexed(part) {
                    actions.push(action);
                    continue;
                }
                if let Some(action) = parse_ability_phrase(part) {
                    actions.push(action);
                } else {
                    all_ok = false;
                    break;
                }
            }
            if !all_ok {
                return None;
            }
            continue;
        }

        if let Some(action) = parse_ability_phrase(segment) {
            actions.push(action);
            continue;
        }

        let and_parts = split_on_lexed_and(segment);
        if and_parts.len() > 1 {
            let mut all_ok = true;
            for part in &and_parts {
                if part.is_empty() {
                    continue;
                }
                if let Some(action) = parse_ability_phrase(part) {
                    actions.push(action);
                } else {
                    all_ok = false;
                    break;
                }
            }
            if !all_ok {
                return None;
            }
        } else {
            return None;
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn rewrite_parse_effect_sentences(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    super::effect_sentences::parse_effect_sentences(tokens)
}

pub(crate) fn rewrite_parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    super::effect_sentences::parse_effect_sentences_lexed(tokens)
}

pub(crate) fn rewrite_parse_triggered_line(
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    super::activation_and_restrictions::parse_triggered_line(tokens)
}

pub(crate) fn rewrite_parse_triggered_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if clause_words.starts_with(&[
        "when",
        "this",
        "becomes",
        "monstrous",
        "it",
        "deals",
        "damage",
        "to",
        "each",
        "opponent",
        "equal",
        "to",
    ]) && clause_words.contains(&"number")
        && clause_words.contains(&"cards")
        && clause_words.contains(&"hand")
    {
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::ThisBecomesMonstrous,
            effects: vec![EffectAst::ForEachOpponent {
                effects: vec![EffectAst::DealDamage {
                    amount: Value::CardsInHand(PlayerFilter::IteratedPlayer),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }],
            max_triggers_per_turn: None,
        });
    }

    fn looks_like_trigger_objectish_word(word: &str) -> bool {
        parse_card_type(word).is_some()
            || parse_subtype_flexible(word).is_some()
            || word.strip_suffix('s').is_some_and(|stem| {
                parse_card_type(stem).is_some() || parse_subtype_flexible(stem).is_some()
            })
    }

    fn looks_like_trigger_object_list_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
        if tokens.is_empty() {
            return false;
        }
        let words_view = LowercaseWordView::new(tokens);
        let words = words_view.to_word_refs();
        if words.is_empty() {
            return false;
        }
        let starts_with_conjunction =
            matches!(words.first().copied(), Some("or" | "and" | "and/or"));
        let first_candidate = if starts_with_conjunction {
            words.get(1).copied()
        } else {
            words.first().copied()
        };
        let Some(first_word) = first_candidate else {
            return false;
        };
        looks_like_trigger_objectish_word(first_word)
            && tokens.iter().any(|token| token.kind == TokenKind::Comma)
    }

    fn looks_like_trigger_discard_qualifier_tail_lexed(
        trigger_prefix_tokens: &[OwnedLexToken],
        tail_tokens: &[OwnedLexToken],
    ) -> bool {
        if tail_tokens.is_empty() {
            return false;
        }

        let prefix_words_view = LowercaseWordView::new(trigger_prefix_tokens);
        let prefix_words = prefix_words_view.to_word_refs();
        if !(prefix_words.contains(&"discard") || prefix_words.contains(&"discards")) {
            return false;
        }

        let tail_words_view = LowercaseWordView::new(tail_tokens);
        let tail_words = tail_words_view.to_word_refs();
        if tail_words.is_empty() {
            return false;
        }

        let Some(first_word) = tail_words.first().copied() else {
            return false;
        };
        let typeish = parse_card_type(first_word).is_some()
            || matches!(
                first_word,
                "artifact" | "artifacts" | "creature" | "creatures"
            )
            || matches!(first_word, "and" | "or");
        if !typeish {
            return false;
        }

        tail_tokens
            .iter()
            .position(|token| token.kind == TokenKind::Comma)
            .is_some_and(|comma_idx| {
                let before_words_view = LowercaseWordView::new(&tail_tokens[..comma_idx]);
                let before_words = before_words_view.to_word_refs();
                before_words.contains(&"card") || before_words.contains(&"cards")
            })
    }

    fn looks_like_trigger_type_list_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
        if tokens.is_empty() {
            return false;
        }
        let words_view = LowercaseWordView::new(tokens);
        let words = words_view.to_word_refs();
        if words.is_empty() {
            return false;
        }
        let first_is_card_type = parse_card_type(words[0]).is_some()
            || parse_subtype_flexible(words[0]).is_some()
            || words[0].strip_suffix('s').is_some_and(|word| {
                parse_card_type(word).is_some() || parse_subtype_flexible(word).is_some()
            });
        first_is_card_type
            && (words.contains(&"spell") || words.contains(&"spells"))
            && words.contains(&"or")
            && tokens.iter().any(|token| token.kind == TokenKind::Comma)
    }

    fn looks_like_trigger_color_list_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
        if tokens.is_empty() {
            return false;
        }
        let words_view = LowercaseWordView::new(tokens);
        let words = words_view.to_word_refs();
        if words.is_empty() {
            return false;
        }
        parse_color(words[0]).is_some()
            && words.contains(&"or")
            && tokens.iter().any(|token| token.kind == TokenKind::Comma)
    }

    fn looks_like_trigger_numeric_list_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
        if tokens.is_empty() {
            return false;
        }
        let words_view = LowercaseWordView::new(tokens);
        let words = words_view.to_word_refs();
        if words.len() < 3 || words[0].parse::<i32>().is_err() {
            return false;
        }
        words.iter().skip(1).any(|word| word.parse::<i32>().is_ok()) && words.contains(&"or")
    }

    fn trim_first_time_each_turn_suffix_lexed(
        trigger_tokens: &[OwnedLexToken],
    ) -> (&[OwnedLexToken], Option<u32>) {
        let trigger_words = LowercaseWordView::new(trigger_tokens);
        let words = trigger_words.to_word_refs();
        for suffix in [
            ["for", "the", "first", "time", "each", "turn"].as_slice(),
            ["for", "the", "first", "time", "this", "turn"].as_slice(),
        ] {
            if words.ends_with(suffix) {
                let trimmed_word_len = words.len().saturating_sub(suffix.len());
                let trimmed_token_len = trigger_words
                    .token_index_for_word_index(trimmed_word_len)
                    .unwrap_or(trigger_tokens.len());
                return (&trigger_tokens[..trimmed_token_len], Some(1));
            }
        }
        (trigger_tokens, None)
    }

    fn rewrite_attached_controller_trigger_effect_tokens_lexed(
        trigger_tokens: &[OwnedLexToken],
        effects_tokens: &[OwnedLexToken],
    ) -> Vec<OwnedLexToken> {
        let trigger_words_view = LowercaseWordView::new(trigger_tokens);
        let trigger_words = trigger_words_view.to_word_refs();
        let references_enchanted_controller = trigger_words.windows(3).any(|window| {
            window[0] == "enchanted"
                && matches!(
                    window[1],
                    "creature"
                        | "creatures"
                        | "permanent"
                        | "permanents"
                        | "artifact"
                        | "artifacts"
                        | "enchantment"
                        | "enchantments"
                        | "land"
                        | "lands"
                )
                && window[2] == "controller"
        });
        if !references_enchanted_controller {
            return effects_tokens.to_vec();
        }

        let mut rewritten = Vec::with_capacity(effects_tokens.len());
        let mut idx = 0usize;
        while idx < effects_tokens.len() {
            if idx + 1 < effects_tokens.len()
                && effects_tokens[idx].is_word("that")
                && effects_tokens[idx + 1].is_word("creature")
            {
                let mut enchanted = effects_tokens[idx].clone();
                enchanted.kind = TokenKind::Word;
                enchanted.slice = "enchanted".to_string();
                rewritten.push(enchanted);
                rewritten.push(effects_tokens[idx + 1].clone());
                idx += 2;
                continue;
            }
            if idx + 1 < effects_tokens.len()
                && effects_tokens[idx].is_word("that")
                && effects_tokens[idx + 1].is_word("permanent")
            {
                let mut enchanted = effects_tokens[idx].clone();
                enchanted.kind = TokenKind::Word;
                enchanted.slice = "enchanted".to_string();
                rewritten.push(enchanted);
                rewritten.push(effects_tokens[idx + 1].clone());
                idx += 2;
                continue;
            }
            rewritten.push(effects_tokens[idx].clone());
            idx += 1;
        }

        rewritten
    }

    fn parse_triggered_times_each_turn_lexed_from_sentences(
        tokens: &[OwnedLexToken],
    ) -> Option<u32> {
        split_lexed_sentences(tokens)
            .iter()
            .find_map(|sentence| parse_triggered_times_each_turn_lexed(sentence))
    }

    let start_idx = if tokens.first().is_some_and(|token| {
        token.is_word("whenever") || token.is_word("at") || token.is_word("when")
    }) {
        1
    } else {
        0
    };

    if start_idx < tokens.len() {
        let trigger_body = &tokens[start_idx..];
        let trigger_body_view = LowercaseWordView::new(trigger_body);
        let trigger_body_words = trigger_body_view.to_word_refs();
        let blocked_prefix_len = if trigger_body_words
            .starts_with(&["this", "creature", "becomes", "blocked"])
        {
            Some(4usize)
        } else if trigger_body_words.starts_with(&["this", "becomes", "blocked"]) {
            Some(3usize)
        } else {
            None
        };
        if let Some(prefix_len) = blocked_prefix_len
            && let Some(effect_start_rel) = trigger_body_view.token_index_after_words(prefix_len)
        {
            let split_idx = start_idx + effect_start_rel;
            let effects_tokens = trim_commas(&tokens[split_idx..]);
            if !effects_tokens.is_empty()
                && let Ok(effects) = rewrite_parse_effect_sentences_lexed(&effects_tokens)
            {
                return Ok(LineAst::Triggered {
                    trigger: TriggerSpec::ThisBecomesBlocked,
                    effects,
                    max_triggers_per_turn: None,
                });
            }
        }

        let leaves_prefix_len = if trigger_body_words.starts_with(&["this", "leaves", "the", "battlefield"]) {
            Some(4usize)
        } else if trigger_body_words
            .starts_with(&["this", "creature", "leaves", "the", "battlefield"])
        {
            Some(5usize)
        } else {
            None
        };
        if let Some(prefix_len) = leaves_prefix_len
            && let Some(effect_start_rel) = trigger_body_view.token_index_after_words(prefix_len)
        {
            let split_idx = start_idx + effect_start_rel;
            let trigger_tokens = trim_commas(&tokens[start_idx..split_idx]);
            let effects_tokens = trim_commas(&tokens[split_idx..]);
            if !effects_tokens.is_empty()
                && let Ok(trigger) = rewrite_parse_trigger_clause_lexed(&trigger_tokens)
                && let Ok(effects) = rewrite_parse_effect_sentences_lexed(&effects_tokens)
            {
                return Ok(LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn: None,
                });
            }
        }
    }

    if let Some(split_idx) = tokens.iter().position(|token| token.kind == TokenKind::Comma) {
        let trigger_tokens = &tokens[start_idx..split_idx];
        let trigger_word_view = LowercaseWordView::new(trigger_tokens);
        let trigger_words = trigger_word_view.to_word_refs();
        if let Some(attack_idx) = trigger_words
            .iter()
            .position(|word| *word == "attack" || *word == "attacks")
            && trigger_words.get(attack_idx + 1).copied() == Some("with")
        {
            let subject_words = &trigger_words[..attack_idx];
            if let Some(player) =
                super::activation_and_restrictions::parse_trigger_subject_player_filter(subject_words)
            {
                let Some(with_object_start) = trigger_word_view
                    .token_index_for_word_index(attack_idx + 2)
                else {
                    return Err(CardTextError::ParseError(format!(
                        "missing attacking-object filter in trigger clause (clause: '{}')",
                        trigger_words.join(" ")
                    )));
                };
                let mut object_tokens = &trigger_tokens[with_object_start..];
                let mut min_total_attackers = None;
                let mut one_or_more = false;
                if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_or_more_quantifier(object_tokens)
                {
                    one_or_more = true;
                    object_tokens = stripped;
                    if count > 1 {
                        min_total_attackers = Some(count);
                    }
                }
                if !object_tokens.is_empty() {
                    let mut filter = super::object_filters::parse_object_filter_lexed(
                        object_tokens,
                        false,
                    )
                    .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported attacking-object filter in trigger clause (clause: '{}')",
                            trigger_words.join(" ")
                        ))
                    })?;
                    if filter.controller.is_none() {
                        filter.controller = Some(player);
                    }
                    let trigger = if let Some(min_total_attackers) = min_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithMinTotal {
                            filter,
                            min_total_attackers,
                        }
                    } else if one_or_more {
                        TriggerSpec::AttacksOneOrMore(filter)
                    } else {
                        TriggerSpec::Attacks(filter)
                    };
                    let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                        trigger_tokens,
                        &tokens[split_idx + 1..],
                    );
                    let effects = rewrite_parse_effect_sentences_lexed(&effects_tokens)?;
                    return Ok(LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: None,
                    });
                }
            }
        }
    }

    if let Some(mut split_idx) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
        .or_else(|| tokens.iter().position(|token| token.is_word("then")))
    {
        if tokens
            .get(split_idx)
            .is_some_and(|token| token.kind == TokenKind::Comma)
            && tokens
                .first()
                .is_some_and(|token| token.is_word("whenever") || token.is_word("when"))
        {
            let trigger_prefix_tokens = &tokens[start_idx..split_idx];
            let tail = &tokens[split_idx + 1..];
            let looks_like_discard_qualifier_tail =
                looks_like_trigger_discard_qualifier_tail_lexed(trigger_prefix_tokens, tail);
            if looks_like_trigger_type_list_tail_lexed(tail)
                || looks_like_trigger_color_list_tail_lexed(tail)
                || looks_like_trigger_object_list_tail_lexed(tail)
                || looks_like_trigger_numeric_list_tail_lexed(tail)
                || looks_like_discard_qualifier_tail
            {
                let next_comma_rel = if looks_like_discard_qualifier_tail {
                    tail.iter().enumerate().find_map(|(idx, token)| {
                        if token.kind != TokenKind::Comma {
                            return None;
                        }
                        let before_words_view = LowercaseWordView::new(&tail[..idx]);
                        let before_words = before_words_view.to_word_refs();
                        if before_words.contains(&"card") || before_words.contains(&"cards") {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                } else if looks_like_trigger_numeric_list_tail_lexed(tail) {
                    tail.iter()
                        .enumerate()
                        .rev()
                        .find_map(|(idx, token)| (token.kind == TokenKind::Comma).then_some(idx))
                } else {
                    tail.iter()
                        .enumerate()
                        .find_map(|(idx, token)| {
                            if token.kind != TokenKind::Comma {
                                return None;
                            }
                            let before_words_view = LowercaseWordView::new(&tail[..idx]);
                            let before_words = before_words_view.to_word_refs();
                            if before_words.contains(&"spell") || before_words.contains(&"spells") {
                                Some(idx)
                            } else {
                                None
                            }
                        })
                        .or_else(|| {
                            if looks_like_trigger_color_list_tail_lexed(tail)
                                || looks_like_trigger_object_list_tail_lexed(tail)
                            {
                                tail.iter().enumerate().find_map(|(idx, token)| {
                                    if token.kind != TokenKind::Comma {
                                        return None;
                                    }
                                    let Some(next_word) =
                                        tail.get(idx + 1).and_then(OwnedLexToken::as_word)
                                    else {
                                        return None;
                                    };
                                    if matches!(next_word, "and" | "or" | "and/or") {
                                        return None;
                                    }

                                    let next_is_list_item =
                                        if looks_like_trigger_color_list_tail_lexed(tail) {
                                            parse_color(next_word).is_some()
                                        } else {
                                            looks_like_trigger_objectish_word(next_word)
                                        };
                                    if next_is_list_item {
                                        return None;
                                    }
                                    Some(idx)
                                })
                            } else {
                                None
                            }
                        })
                };
                if let Some(next_comma_rel) = next_comma_rel {
                    let candidate_idx = split_idx + 1 + next_comma_rel;
                    if candidate_idx > start_idx && candidate_idx + 1 < tokens.len() {
                        split_idx = candidate_idx;
                    }
                }
            }
        }

        let (trigger_tokens, max_triggers_from_trigger_clause) =
            trim_first_time_each_turn_suffix_lexed(&tokens[start_idx..split_idx]);
        if let Ok(trigger) = rewrite_parse_trigger_clause_lexed(trigger_tokens) {
            let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                trigger_tokens,
                &tokens[split_idx + 1..],
            );
            match rewrite_parse_effect_sentences_lexed(&effects_tokens) {
                Ok(effects) => {
                    let mut max_triggers_per_turn =
                        parse_triggered_times_each_turn_lexed_from_sentences(&effects_tokens);
                    if let Some(max) = max_triggers_from_trigger_clause {
                        max_triggers_per_turn =
                            Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                    }
                    return Ok(LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn,
                    });
                }
                Err(err) => return Err(err),
            }
        }
    }

    for split_idx in ((start_idx + 1)..tokens.len()).rev() {
        let (trigger_tokens, max_triggers_from_trigger_clause) =
            trim_first_time_each_turn_suffix_lexed(&tokens[start_idx..split_idx]);
        let effects_tokens = &tokens[split_idx..];
        if effects_tokens.is_empty() {
            continue;
        }
        if let Ok(trigger) = rewrite_parse_trigger_clause_lexed(trigger_tokens) {
            let rewritten_effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                trigger_tokens,
                effects_tokens,
            );
            let effects = rewrite_parse_effect_sentences_lexed(&rewritten_effects_tokens)
                .or_else(|_| {
                    let Some(stripped) = super::activation_and_restrictions::
                        maybe_strip_leading_damage_subject_tokens(&rewritten_effects_tokens)
                    else {
                        return Err(CardTextError::ParseError(String::new()));
                    };
                    rewrite_parse_effect_sentences_lexed(stripped)
                });
            if let Ok(effects) = effects {
                let mut max_triggers_per_turn =
                    parse_triggered_times_each_turn_lexed_from_sentences(&rewritten_effects_tokens);
                if let Some(max) = max_triggers_from_trigger_clause {
                    max_triggers_per_turn =
                        Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                }
                return Ok(LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                });
            }
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported triggered line (clause: '{}')",
        LowercaseWordView::new(tokens).to_word_refs().join(" ")
    )))
}

pub(crate) fn rewrite_parse_trigger_clause(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    super::activation_and_restrictions::parse_trigger_clause(tokens)
}

pub(crate) fn rewrite_parse_trigger_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    super::activation_and_restrictions::parse_trigger_clause_lexed(tokens)
}

pub(crate) fn rewrite_parse_static_ability_ast_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    super::keyword_static::parse_static_ability_ast_line(tokens)
}

pub(crate) fn rewrite_parse_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    parse_static_ability_ast_line_lexed(tokens)
}
