use super::super::activation_and_restrictions::parse_single_word_keyword_action;
use super::super::clause_support::parse_triggered_line_lexed;
use super::super::compile_support::compile_statement_effects;
use super::super::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::super::native_tokens::LowercaseWordView;
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed, split_on_or};
use super::super::util::{
    is_article, is_source_reference_words, parse_mana_symbol, parse_target_phrase,
    span_from_tokens, split_on_and, token_index_for_word_index, trim_commas, words,
};
use super::dispatch_inner::trim_edge_punctuation;
use super::lex_chain_helpers::find_verb_lexed;
use super::sentence_helpers::*;
#[allow(unused_imports)]
use super::{Verb, find_verb, parse_effect_chain};
use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, KeywordAction, LineAst, ReferenceImports,
    TagKey, TargetAst, TextSpan,
};
use crate::effect::Until;
use crate::mana::ManaCost;
use crate::target::PlayerFilter;
use crate::zone::Zone;

fn display_text_for_tokens(tokens: &[OwnedLexToken]) -> String {
    let mut text = String::new();
    let mut needs_space = false;
    let mut in_effect_text = false;

    for token in tokens {
        if let Some(word) = token.as_word() {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            let numeric_like = word
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | 'X' | '+' | '-' | '/'));
            let rendered = match word {
                "t" => "{T}".to_string(),
                "q" => "{Q}".to_string(),
                _ if in_effect_text && numeric_like => word.to_string(),
                _ => parse_mana_symbol(word)
                    .map(|symbol| ManaCost::from_symbols(vec![symbol]).to_oracle())
                    .unwrap_or_else(|_| word.to_ascii_lowercase()),
            };
            text.push_str(&rendered);
            needs_space = true;
        } else if token.kind == TokenKind::ManaGroup {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            text.push_str(token.slice.as_str());
            needs_space = true;
        } else if token.is_colon() {
            text.push(':');
            needs_space = true;
            in_effect_text = true;
        } else if token.is_comma() {
            text.push(',');
            needs_space = true;
        } else if token.is_period() {
            text.push('.');
            needs_space = true;
        } else if token.is_semicolon() {
            text.push(';');
            needs_space = true;
        }
    }

    text
}

fn grants_protection_from_everything(ability: &GrantedAbilityAst) -> bool {
    matches!(
        ability,
        GrantedAbilityAst::KeywordAction(KeywordAction::ProtectionFromEverything)
    )
}

fn player_gain_effects_for_abilities(
    abilities: &[GrantedAbilityAst],
    duration: &Until,
    subject_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let player_target =
        TargetAst::Player(PlayerFilter::You, span_from_lexed_tokens(subject_tokens));
    let mut effects = Vec::new();

    for ability in abilities {
        match ability {
            GrantedAbilityAst::KeywordAction(KeywordAction::HexproofFrom(filter)) => {
                effects.push(EffectAst::Cant {
                    restriction: crate::effect::Restriction::be_targeted_player_from(
                        PlayerFilter::You,
                        filter.clone(),
                    ),
                    duration: duration.clone(),
                    condition: None,
                });
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::ProtectionFromEverything) => {
                effects.push(EffectAst::Cant {
                    restriction: crate::effect::Restriction::be_targeted_player(PlayerFilter::You),
                    duration: duration.clone(),
                    condition: None,
                });
                effects.push(EffectAst::PreventAllDamageToTarget {
                    target: player_target.clone(),
                    duration: duration.clone(),
                });
            }
            _ => return None,
        }
    }

    Some(effects)
}

fn parse_granted_ability_component_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let ability_tokens = trim_edge_punctuation(ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let ability_word_view = LowercaseWordView::new(&ability_tokens);
    let ability_words = ability_word_view.to_word_refs();
    if ability_words.starts_with(&["hexproof", "from"]) {
        let filter_tokens = ability_tokens[2..]
            .iter()
            .filter(|token| !token.is_word("and") && !token.is_word("from"))
            .cloned()
            .collect::<Vec<_>>();
        if !filter_tokens.is_empty()
            && let Ok(filter) = parse_object_filter_lexed(&filter_tokens, false)
        {
            return Ok(Some(vec![GrantedAbilityAst::from(
                KeywordAction::HexproofFrom(filter),
            )]));
        }
    }

    if let Some(granted) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![granted]));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        return Ok(Some(
            actions.into_iter().map(GrantedAbilityAst::from).collect(),
        ));
    }

    if let Some(action) = ability_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .filter(|_| ability_tokens.len() == 1)
        .and_then(parse_single_word_keyword_action)
    {
        return Ok(Some(vec![GrantedAbilityAst::from(action)]));
    }

    Ok(None)
}

fn parse_granted_abilities_for_gain_clause(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
    allow_choice: bool,
) -> Result<(Vec<GrantedAbilityAst>, bool), CardTextError> {
    if let Some(abilities) = parse_granted_ability_component_for_gain(ability_tokens, clause_words)?
    {
        return Ok((abilities, false));
    }

    if allow_choice && let Some(actions) = parse_choice_of_abilities(ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        return Ok((
            actions.into_iter().map(GrantedAbilityAst::from).collect(),
            true,
        ));
    }

    let segments = split_on_and(ability_tokens);
    if segments.len() <= 1 {
        return Ok((Vec::new(), false));
    }

    let mut abilities = Vec::new();
    for segment in segments {
        let Some(parsed) = parse_granted_ability_component_for_gain(&segment, clause_words)? else {
            return Ok((Vec::new(), false));
        };
        abilities.extend(parsed);
    }

    Ok((abilities, false))
}

pub(crate) fn parse_simple_ability_duration(
    words_after_verb: &[&str],
) -> Option<(usize, usize, Until)> {
    if let Some(idx) = words_after_verb.windows(4).position(is_until_end_of_turn) {
        return Some((idx, 4, Until::EndOfTurn));
    }
    if let Some(idx) = words_after_verb.windows(4).position(|window| {
        window == ["until", "your", "next", "turn"] || window == ["until", "your", "next", "upkeep"]
    }) {
        return Some((idx, 4, Until::YourNextTurn));
    }
    if let Some(idx) = words_after_verb.windows(5).position(|window| {
        window == ["until", "your", "next", "untap", "step"]
            || window == ["during", "your", "next", "untap", "step"]
    }) {
        return Some((idx, 5, Until::YourNextTurn));
    }
    if let Some(idx) = words_after_verb
        .windows(6)
        .position(|window| window == ["for", "as", "long", "as", "you", "control"])
    {
        return Some((
            idx,
            words_after_verb.len().saturating_sub(idx),
            Until::YouStopControllingThis,
        ));
    }
    None
}

pub(crate) fn parse_simple_gain_ability_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, false)
}

pub(crate) fn parse_simple_lose_ability_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, true)
}

fn lexed_token_index_for_word_index(tokens: &[OwnedLexToken], word_idx: usize) -> Option<usize> {
    token_index_for_word_index(tokens, word_idx)
}

fn span_from_lexed_tokens(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => Some(TextSpan {
            line: first.span.line,
            start: first.span.start,
            end: last.span.end,
        }),
        _ => None,
    }
}

fn parse_simple_ability_modifier_clause_lexed(
    tokens: &[OwnedLexToken],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let verb_idx = clause_words.iter().position(|word| {
        if losing {
            matches!(*word, "lose" | "loses")
        } else {
            matches!(*word, "gain" | "gains")
        }
    });
    let Some(verb_idx) = verb_idx else {
        return Ok(None);
    };
    let implied_it_subject = verb_idx == 0;
    let Some(verb_token_idx) = lexed_token_index_for_word_index(tokens, verb_idx) else {
        return Ok(None);
    };

    if !losing && matches!(clause_words[verb_idx], "gain" | "gains") {
        let starts_with_life = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "life");
        let starts_with_control = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let subject_tokens = trim_lexed_commas(&tokens[..verb_token_idx]);
    if subject_tokens.is_empty() && !implied_it_subject {
        return Ok(None);
    }

    if !losing
        && !subject_tokens.is_empty()
        && let Some((subject_verb, _)) = find_verb_lexed(subject_tokens)
        && subject_verb != Verb::Get
    {
        let subject_words = LowercaseWordView::new(subject_tokens);
        let subject_word_refs = subject_words.to_word_refs();
        let target_phrase_with_controller_tail = subject_word_refs.first().copied()
            == Some("target")
            && (subject_word_refs.contains(&"control") || subject_word_refs.contains(&"controls"));
        if !target_phrase_with_controller_tail {
            return Ok(None);
        }
    }

    let words_after_verb = &clause_words[verb_idx + 1..];
    if words_after_verb.is_empty() {
        return Ok(None);
    }

    let duration_phrase = parse_simple_ability_duration(words_after_verb);
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .unwrap_or(Until::Forever);

    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start, _, _)| verb_idx + 1 + *start)
        .unwrap_or(clause_words.len());
    let ability_end_token_idx =
        lexed_token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
    let ability_tokens = trim_lexed_commas(&tokens[verb_token_idx + 1..ability_end_token_idx]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let (abilities, _) =
        parse_granted_abilities_for_gain_clause(ability_tokens, &clause_words, false)?;
    if abilities.is_empty() {
        return Ok(None);
    }
    let abilities = abilities;

    if let Some((start, len, _)) = duration_phrase {
        let tail_word_idx = verb_idx + 1 + start + len;
        if let Some(tail_token_idx) = lexed_token_index_for_word_index(tokens, tail_word_idx) {
            let trailing = trim_lexed_commas(&tokens[tail_token_idx..]);
            if !trailing.is_empty() {
                return Ok(None);
            }
        }
    }

    let subject_words = LowercaseWordView::new(subject_tokens);
    let subject_word_refs = subject_words.to_word_refs();
    let is_pronoun_subject =
        implied_it_subject || matches!(subject_word_refs.as_slice(), ["it"] | ["they"] | ["them"]);
    if is_pronoun_subject {
        let target =
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_lexed_tokens(subject_tokens));
        if losing {
            return Ok(Some(EffectAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            }));
        }
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        }));
    }

    let is_demonstrative_subject = subject_words
        .first()
        .is_some_and(|word| word == "that" || word == "those");
    if is_demonstrative_subject || subject_words.find("target").is_some() {
        let target = parse_target_phrase(subject_tokens)?;
        if losing {
            return Ok(Some(EffectAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            }));
        }
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        }));
    }

    let filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in {}-ability clause (clause: '{}')",
            if losing { "lose" } else { "gain" },
            clause_words.join(" ")
        ))
    })?;
    if losing {
        return Ok(Some(EffectAst::RemoveAbilitiesAll {
            filter,
            abilities,
            duration,
        }));
    }
    Ok(Some(EffectAst::GrantAbilitiesAll {
        filter,
        abilities,
        duration,
    }))
}

pub(crate) fn parse_simple_gain_ability_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, false)
}

pub(crate) fn parse_simple_lose_ability_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, true)
}

pub(crate) fn parse_simple_ability_modifier_clause(
    tokens: &[OwnedLexToken],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let verb_idx = clause_words.iter().position(|word| {
        if losing {
            matches!(*word, "lose" | "loses")
        } else {
            matches!(*word, "gain" | "gains")
        }
    });
    let Some(verb_idx) = verb_idx else {
        return Ok(None);
    };
    let implied_it_subject = verb_idx == 0;
    let Some(verb_token_idx) = token_index_for_word_index(tokens, verb_idx) else {
        return Ok(None);
    };

    if !losing && matches!(clause_words[verb_idx], "gain" | "gains") {
        let starts_with_life = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "life");
        let starts_with_control = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let subject_tokens = trim_commas(&tokens[..verb_token_idx]);
    if subject_tokens.is_empty() && !implied_it_subject {
        return Ok(None);
    }

    if !losing
        && !subject_tokens.is_empty()
        && let Some((subject_verb, _)) = find_verb(&subject_tokens)
        && subject_verb != Verb::Get
    {
        let subject_words = words(&subject_tokens);
        let target_phrase_with_controller_tail = subject_words.first().copied() == Some("target")
            && subject_words.contains(&"control")
            || subject_words.contains(&"controls");
        if !target_phrase_with_controller_tail {
            return Ok(None);
        }
    }

    let words_after_verb = &clause_words[verb_idx + 1..];
    if words_after_verb.is_empty() {
        return Ok(None);
    }

    let duration_phrase = parse_simple_ability_duration(words_after_verb);
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .unwrap_or(Until::Forever);

    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start, _, _)| verb_idx + 1 + *start)
        .unwrap_or(clause_words.len());
    let ability_end_token_idx =
        token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
    let ability_tokens = trim_commas(&tokens[verb_token_idx + 1..ability_end_token_idx]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let (abilities, _) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, false)?;
    if abilities.is_empty() {
        return Ok(None);
    }
    let abilities = abilities;

    if let Some((start, len, _)) = duration_phrase {
        let tail_word_idx = verb_idx + 1 + start + len;
        if let Some(tail_token_idx) = token_index_for_word_index(tokens, tail_word_idx) {
            let trailing = trim_commas(&tokens[tail_token_idx..]);
            if !trailing.is_empty() {
                return Ok(None);
            }
        }
    }

    let subject_words = LowercaseWordView::new(&subject_tokens);
    let subject_word_refs = subject_words.to_word_refs();
    let is_pronoun_subject =
        implied_it_subject || matches!(subject_word_refs.as_slice(), ["it"] | ["they"] | ["them"]);
    if is_pronoun_subject {
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&subject_tokens));
        if losing {
            return Ok(Some(EffectAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            }));
        }
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        }));
    }

    let is_demonstrative_subject = subject_word_refs
        .first()
        .is_some_and(|word| *word == "that" || *word == "those");
    if is_demonstrative_subject || subject_word_refs.contains(&"target") {
        let target = parse_target_phrase(&subject_tokens)?;
        if losing {
            return Ok(Some(EffectAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            }));
        }
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        }));
    }

    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in {}-ability clause (clause: '{}')",
            if losing { "lose" } else { "gain" },
            clause_words.join(" ")
        ))
    })?;
    if losing {
        return Ok(Some(EffectAst::RemoveAbilitiesAll {
            filter,
            abilities,
            duration,
        }));
    }
    Ok(Some(EffectAst::GrantAbilitiesAll {
        filter,
        abilities,
        duration,
    }))
}

pub(crate) fn parse_gain_ability_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let word_view = LowercaseWordView::new(tokens);
    let word_list = word_view.to_word_refs();
    let looks_like_can_attack_no_defender = word_list
        .windows(2)
        .any(|window| window == ["can", "attack"])
        && word_list
            .windows(2)
            .any(|window| window == ["as", "though"])
        && word_list.contains(&"defender");
    if looks_like_can_attack_no_defender {
        return Ok(None);
    }
    let gain_idx = word_list
        .iter()
        .position(|word| matches!(*word, "gain" | "gains" | "has" | "have" | "lose" | "loses"));
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };
    let Some(gain_token_idx) = token_index_for_word_index(tokens, gain_idx) else {
        return Ok(None);
    };
    let losing = matches!(word_list[gain_idx], "lose" | "loses");

    let after_gain = &word_list[gain_idx + 1..];
    if matches!(word_list[gain_idx], "gain" | "gains") {
        let starts_with_life = after_gain.first().is_some_and(|word| *word == "life");
        let starts_with_control = after_gain.first().is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let leading_duration_phrase = if starts_with_until_end_of_turn(&word_list) {
        Some((4usize, Until::EndOfTurn))
    } else if word_list.starts_with(&["until", "your", "next", "turn"])
        || word_list.starts_with(&["until", "your", "next", "upkeep"])
    {
        Some((4usize, Until::YourNextTurn))
    } else if word_list.starts_with(&["until", "your", "next", "untap", "step"])
        || word_list.starts_with(&["during", "your", "next", "untap", "step"])
    {
        Some((5usize, Until::YourNextTurn))
    } else {
        None
    };
    let subject_start_word_idx = leading_duration_phrase
        .as_ref()
        .map(|(len, _)| *len)
        .unwrap_or(0);
    let subject_start_token_idx = if subject_start_word_idx == 0 {
        0usize
    } else if let Some(idx) = token_index_for_word_index(tokens, subject_start_word_idx) {
        idx
    } else {
        return Ok(None);
    };
    if subject_start_token_idx < gain_token_idx
        && let Some((subject_verb, _)) = find_verb(&tokens[subject_start_token_idx..gain_token_idx])
        && subject_verb != Verb::Get
    {
        return Ok(None);
    }

    let duration_phrase = if let Some(idx) = after_gain.windows(4).position(is_until_end_of_turn) {
        Some((idx, 4usize, Until::EndOfTurn))
    } else if let Some(idx) = after_gain.windows(4).position(|window| {
        window == ["until", "your", "next", "turn"] || window == ["until", "your", "next", "upkeep"]
    }) {
        Some((idx, 4usize, Until::YourNextTurn))
    } else if let Some(idx) = after_gain.windows(5).position(|window| {
        window == ["until", "your", "next", "untap", "step"]
            || window == ["during", "your", "next", "untap", "step"]
    }) {
        Some((idx, 5usize, Until::YourNextTurn))
    } else if let Some(idx) = after_gain
        .windows(6)
        .position(|window| window == ["for", "as", "long", "as", "you", "control"])
    {
        // Consume the remainder of the phrase as the duration clause.
        Some((
            idx,
            after_gain.len().saturating_sub(idx),
            Until::YouStopControllingThis,
        ))
    } else {
        None
    };
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| {
            leading_duration_phrase
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .unwrap_or(Until::Forever);
    let has_explicit_duration =
        duration_phrase.is_some() || leading_duration_phrase.as_ref().is_some();

    let mut trailing_tail_tokens: Vec<OwnedLexToken> = Vec::new();
    if let Some((start_rel, len_words, _)) = duration_phrase {
        let tail_word_idx = gain_idx + 1 + start_rel + len_words;
        if let Some(tail_token_idx) = token_index_for_word_index(tokens, tail_word_idx) {
            let mut tail_tokens = trim_commas(&tokens[tail_token_idx..]).to_vec();
            while tail_tokens
                .first()
                .is_some_and(|token| token.is_word("and") || token.is_word("then"))
            {
                tail_tokens.remove(0);
            }
            if !tail_tokens.is_empty() {
                trailing_tail_tokens = tail_tokens;
            }
        }
    }
    let mut grants_must_attack = false;
    if !trailing_tail_tokens.is_empty() {
        let tail_view = LowercaseWordView::new(&trailing_tail_tokens);
        let mut tail_words = tail_view.to_word_refs();
        if tail_words.first().is_some_and(|word| *word == "and") {
            tail_words = tail_words[1..].to_vec();
        }
        if tail_words.as_slice() == ["attacks", "this", "combat", "if", "able"]
            || tail_words.as_slice() == ["attack", "this", "combat", "if", "able"]
        {
            grants_must_attack = true;
            trailing_tail_tokens.clear();
        }
    }

    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel);
    let ability_end_token_idx = if let Some(end_word_idx) = ability_end_word_idx {
        token_index_for_word_index(tokens, end_word_idx).unwrap_or(tokens.len())
    } else {
        tokens.len()
    };
    let ability_start_token_idx = gain_token_idx + 1;
    if ability_start_token_idx > ability_end_token_idx || ability_start_token_idx >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = trim_commas(&tokens[ability_start_token_idx..ability_end_token_idx]);

    let (mut abilities, grant_is_choice) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &word_list, !losing)?;
    if abilities.is_empty() && !grants_must_attack {
        return Ok(None);
    }
    if grants_must_attack {
        abilities.push(GrantedAbilityAst::MustAttack);
    }

    // Check for "gets +X/+Y and gains/has/loses ..." patterns - if there's a pump
    // modifier before the ability verb, extract it as a separate Pump/PumpAll effect.
    let before_gain = &word_list[subject_start_word_idx..gain_idx];
    let get_idx = before_gain.iter().position(|w| *w == "get" || *w == "gets");
    let pump_effect = if let Some(gi) = get_idx {
        let mod_word = before_gain.get(gi + 1).copied().unwrap_or("");
        if let Ok((power, toughness)) = parse_pt_modifier_values(mod_word) {
            Some((power, toughness, subject_start_word_idx + gi))
        } else {
            None
        }
    } else {
        None
    };
    let has_have_verb = matches!(word_list[gain_idx], "has" | "have");
    if has_have_verb && pump_effect.is_none() && !has_explicit_duration {
        return Ok(None);
    }

    // Determine the real subject (before "get"/"gets" if pump is present)
    let real_subject_end_word_idx = pump_effect
        .as_ref()
        .map(|(_, _, gi)| *gi)
        .unwrap_or(gain_idx);
    let real_subject_end_token_idx =
        token_index_for_word_index(tokens, real_subject_end_word_idx).unwrap_or(gain_token_idx);
    if subject_start_token_idx >= real_subject_end_token_idx {
        return Ok(None);
    }
    let real_subject_tokens =
        trim_commas(&tokens[subject_start_token_idx..real_subject_end_token_idx]);

    let mut effects = Vec::new();

    // Check for pronoun subjects ("it", "they") that reference a prior tagged object.
    let real_subject_word_view = LowercaseWordView::new(&real_subject_tokens);
    let real_subject_words = real_subject_word_view.to_word_refs();
    let is_pronoun_subject =
        real_subject_words.as_slice() == ["it"] || real_subject_words.as_slice() == ["they"];
    if is_pronoun_subject {
        let span = span_from_tokens(&real_subject_tokens);
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span);
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
                condition: None,
            });
        }
        if losing {
            effects.push(EffectAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            });
        } else if grant_is_choice {
            effects.push(EffectAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            });
        } else {
            effects.push(EffectAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
            });
        }
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let is_demonstrative_subject = real_subject_words
        .first()
        .is_some_and(|word| *word == "that" || *word == "those");
    if is_demonstrative_subject {
        let target = parse_target_phrase(&real_subject_tokens)?;
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
                condition: None,
            });
        }
        if losing {
            effects.push(EffectAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            });
        } else if grant_is_choice {
            effects.push(EffectAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            });
        } else {
            effects.push(EffectAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
            });
        }
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if before_gain.contains(&"target") {
        let has_pump_effect = pump_effect.is_some();
        let target = parse_target_phrase(&real_subject_tokens)?;
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
                condition: None,
            });
        }
        let grant_target = if has_pump_effect {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&real_subject_tokens))
        } else {
            target
        };
        if losing {
            effects.push(EffectAst::RemoveAbilitiesFromTarget {
                target: grant_target,
                abilities,
                duration,
            });
        } else if grant_is_choice {
            effects.push(EffectAst::GrantAbilitiesChoiceToTarget {
                target: grant_target,
                abilities,
                duration,
            });
        } else {
            effects.push(EffectAst::GrantAbilitiesToTarget {
                target: grant_target,
                abilities,
                duration,
            });
        }
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing && real_subject_words.as_slice() == ["you"] {
        let has_protection_from_everything =
            abilities.iter().any(grants_protection_from_everything);
        if has_protection_from_everything {
            let player_target =
                TargetAst::Player(PlayerFilter::You, span_from_tokens(&real_subject_tokens));
            effects.push(EffectAst::Cant {
                restriction: crate::effect::Restriction::be_targeted_player(PlayerFilter::You),
                duration: duration.clone(),
                condition: None,
            });
            effects.push(EffectAst::PreventAllDamageToTarget {
                target: player_target,
                duration: duration.clone(),
            });
            effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
            return Ok(Some(effects));
        }
    }

    if !losing && real_subject_words.as_slice() == ["you", "and", "permanents", "you", "control"] {
        let permanent_filter = crate::target::ObjectFilter::permanent().you_control();
        let Some(mut player_effects) =
            player_gain_effects_for_abilities(&abilities, &duration, &real_subject_tokens)
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed player/permanent gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects.push(EffectAst::GrantAbilitiesAll {
            filter: permanent_filter,
            abilities,
            duration,
        });
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let filter = parse_object_filter(&real_subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in {}-ability clause (clause: '{}')",
            if losing { "lose" } else { "gain" },
            word_list.join(" ")
        ))
    })?;

    if let Some((power, toughness, _)) = pump_effect {
        effects.push(EffectAst::PumpAll {
            filter: filter.clone(),
            power,
            toughness,
            duration: duration.clone(),
        });
    }
    if losing {
        effects.push(EffectAst::RemoveAbilitiesAll {
            filter,
            abilities,
            duration,
        });
    } else if grant_is_choice {
        effects.push(EffectAst::GrantAbilitiesChoiceAll {
            filter,
            abilities,
            duration,
        });
    } else {
        effects.push(EffectAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        });
    }
    effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;

    Ok(Some(effects))
}

pub(crate) fn parse_granted_activated_or_triggered_ability_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<GrantedAbilityAst>, CardTextError> {
    let ability_tokens = trim_edge_punctuation(ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let has_colon = ability_tokens.iter().any(|token| token.is_colon());
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.is_word("when")
            || token.is_word("whenever")
            || (token.is_word("at")
                && ability_tokens
                    .get(1)
                    .is_some_and(|next| next.is_word("the")))
    });
    if !has_colon && !looks_like_trigger {
        return Ok(None);
    }

    let display = display_text_for_tokens(&ability_tokens);
    let parsed_ability = if has_colon {
        let Some(parsed) = parse_activated_line(&ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        parsed
    } else {
        match parse_triggered_line_lexed(&ability_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => parsed_triggered_ability(
                trigger,
                effects,
                vec![Zone::Battlefield],
                Some(display.clone()),
                max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
                ReferenceImports::default(),
            ),
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated/triggered ability clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    Ok(Some(GrantedAbilityAst::ParsedObjectAbility {
        ability: parsed_ability,
        display,
    }))
}

pub(crate) fn append_gain_ability_trailing_effects(
    mut effects: Vec<EffectAst>,
    trailing_tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if trailing_tokens.is_empty() {
        return Ok(effects);
    }

    let trimmed = trim_commas(trailing_tokens);
    if trimmed.first().is_some_and(|token| token.is_word("unless")) {
        if let Some(unless_effect) = try_build_unless(effects, &trimmed, 0)? {
            return Ok(vec![unless_effect]);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing unless gain-ability clause (clause: '{}')",
            words(&trimmed).join(" ")
        )));
    }

    if let Ok(parsed_tail) = parse_effect_chain(&trimmed)
        && !parsed_tail.is_empty()
    {
        effects.extend(parsed_tail);
    }
    Ok(effects)
}

pub(crate) fn parse_choice_of_abilities(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let tokens = trim_commas(tokens);
    let words = words(&tokens);
    let prefix_words = if words.starts_with(&["your", "choice", "of"]) {
        3usize
    } else if words.starts_with(&["your", "choice", "from"]) {
        3usize
    } else {
        return None;
    };
    if words.len() <= prefix_words + 1 {
        return None;
    }

    let start_idx = token_index_for_word_index(&tokens, prefix_words)?;
    let option_tokens = trim_commas(&tokens[start_idx..]);
    if option_tokens.is_empty() {
        return None;
    }

    let mut actions = Vec::new();
    for segment in split_on_or(&option_tokens) {
        let segment = trim_commas(&segment);
        if segment.is_empty() {
            continue;
        }
        let action = parse_ability_phrase(&segment)?;
        if !actions.contains(&action) {
            actions.push(action);
        }
    }

    if actions.len() < 2 {
        return None;
    }
    Some(actions)
}

pub(crate) fn parse_gain_ability_to_source_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let gain_idx = clause_words
        .iter()
        .position(|word| *word == "gain" || *word == "gains");
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };

    let subject_tokens = &tokens[..gain_idx];
    let subject_words: Vec<&str> = words(subject_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !is_source_reference_words(&subject_words) {
        return Ok(None);
    }

    let ability_tokens = trim_edge_punctuation(&tokens[gain_idx + 1..]);
    if let Some(parsed) = parse_activated_line(&ability_tokens)? {
        return Ok(Some(EffectAst::GrantAbilityToSource { ability: parsed }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::super::super::util::tokenize_line;
    use super::*;
    use crate::CardId;
    use crate::ability::AbilityKind;
    use crate::cards::builders::CardDefinitionBuilder;

    #[test]
    fn gain_ability_to_source_keeps_parsed_ability_until_lowering() {
        let tokens = tokenize_line("This creature gains {T}: Draw a card.", 0);
        let effect = parse_gain_ability_to_source_sentence(&tokens)
            .expect("gain-to-source sentence should parse")
            .expect("gain-to-source sentence should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            debug.contains("GrantAbilityToSource"),
            "expected source grant effect, got {debug}"
        );
        assert!(
            debug.contains("effects_ast: Some"),
            "expected parsed ability to remain unlowered in the AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("grant-to-source effect should lower");
        assert!(
            format!("{compiled:?}").contains("GrantObjectAbilityEffect"),
            "expected source grant effect after lowering, got {compiled:?}"
        );
    }

    #[test]
    fn target_gain_activated_ability_stays_unlowered_until_compile() {
        let tokens = tokenize_line(
            "Target creature gains {T}: Draw a card until end of turn.",
            0,
        );
        let effect = parse_simple_gain_ability_clause(&tokens)
            .expect("target gain clause should parse")
            .expect("target gain clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            debug.contains("ParsedObjectAbility"),
            "expected parsed granted ability in AST, got {debug}"
        );
        assert!(
            debug.contains("effects_ast: Some"),
            "expected granted ability to remain unlowered in AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("target gain clause should lower");
        let compiled_debug = format!("{compiled:?}");
        assert!(
            compiled_debug.contains("ApplyContinuousEffect")
                && (compiled_debug.contains("AddAbilityGeneric")
                    || compiled_debug.contains("GrantObjectAbilityForFilter")),
            "expected lowered granted ability effect, got {compiled_debug}"
        );
    }

    #[test]
    fn target_lose_activated_ability_stays_unlowered_until_compile() {
        let tokens = tokenize_line(
            "Target creature loses {T}: Draw a card until end of turn.",
            0,
        );
        let effect = parse_simple_lose_ability_clause(&tokens)
            .expect("target lose clause should parse")
            .expect("target lose clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            debug.contains("ParsedObjectAbility"),
            "expected parsed removed ability in AST, got {debug}"
        );
        assert!(
            debug.contains("effects_ast: Some"),
            "expected removed ability to remain unlowered in AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("target lose clause should lower");
        let compiled_debug = format!("{compiled:?}");
        assert!(
            compiled_debug.contains("RemoveAbility"),
            "expected lowered remove-ability effect, got {compiled_debug}"
        );
        assert!(
            compiled_debug.contains("GrantObjectAbilityForFilter"),
            "expected removed granted object ability after lowering, got {compiled_debug}"
        );
    }

    #[test]
    fn lexed_target_gain_activated_ability_matches_wrapper_clause() {
        let text = "Target creature gains {T}: Draw a card until end of turn.";
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify gain clause");
        let compat = tokenize_line(text, 0);

        let lexed_effect =
            parse_simple_gain_ability_clause_lexed(&lexed).expect("lexed gain clause should parse");
        let compat_effect =
            parse_simple_gain_ability_clause(&compat).expect("wrapper gain clause should parse");

        assert_eq!(
            format!("{lexed_effect:?}"),
            format!("{compat_effect:?}"),
            "lexed simple gain-ability clause should match wrapper output"
        );
    }

    #[test]
    fn lexed_target_lose_activated_ability_matches_wrapper_clause() {
        let text = "Target creature loses {T}: Draw a card until end of turn.";
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify lose clause");
        let compat = tokenize_line(text, 0);

        let lexed_effect =
            parse_simple_lose_ability_clause_lexed(&lexed).expect("lexed lose clause should parse");
        let compat_effect =
            parse_simple_lose_ability_clause(&compat).expect("wrapper lose clause should parse");

        assert_eq!(
            format!("{lexed_effect:?}"),
            format!("{compat_effect:?}"),
            "lexed simple lose-ability clause should match wrapper output"
        );
    }

    #[test]
    fn pump_and_lose_ability_sentence_keeps_shared_until_your_next_turn() {
        let tokens = tokenize_line(
            "Target creature gets -2/-0 and loses flying until your next turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("pump-and-lose sentence should parse")
            .expect("pump-and-lose sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            debug.contains("Pump") && debug.contains("RemoveAbilitiesFromTarget"),
            "expected pump plus remove-ability effects, got {debug}"
        );
        assert!(
            debug.matches("YourNextTurn").count() >= 2,
            "expected shared duration to apply to both effects, got {debug}"
        );
    }

    #[test]
    fn gain_landwalk_until_next_upkeep_sentence_parses() {
        let tokens = tokenize_line(
            "Target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("gain-until-next-upkeep sentence should parse")
            .expect("gain-until-next-upkeep sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            debug.contains("GrantAbilitiesToTarget"),
            "expected target ability grant, got {debug}"
        );
        assert!(
            debug.contains("Landwalk(Subtype { subtype: Forest, snow: false })")
                && debug.contains("YourNextTurn"),
            "expected forestwalk grant to keep next-upkeep duration, got {debug}"
        );
    }

    #[test]
    fn lexed_gain_landwalk_until_next_upkeep_sentence_parses() {
        let mut tokens = lex_line(
            "Target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
            0,
        )
        .expect("rewrite lexer should classify landwalk gain clause");
        for token in &mut tokens {
            if let Some(word) = token.word_mut() {
                *word = word.to_ascii_lowercase();
            }
        }
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("lexed gain-until-next-upkeep sentence should parse")
            .expect("lexed gain-until-next-upkeep sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            debug.contains("GrantAbilitiesToTarget"),
            "expected target ability grant, got {debug}"
        );
        assert!(
            debug.contains("Landwalk(Subtype { subtype: Forest, snow: false })")
                && debug.contains("YourNextTurn"),
            "expected forestwalk grant to keep next-upkeep duration, got {debug}"
        );
    }

    #[test]
    fn quoted_granted_trigger_keeps_all_sentences_inside_the_grant() {
        let tokens = tokenize_line(
            "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("quoted granted trigger should parse")
            .expect("quoted granted trigger should produce effects");

        assert_eq!(
            effects.len(),
            1,
            "quoted granted trigger should stay inside a single grant effect: {effects:?}"
        );

        let debug = format!("{effects:?}");
        assert!(
            debug.contains("GrantAbilitiesAll"),
            "expected a global grant effect, got {debug}"
        );
        assert!(
            debug.contains("ParsedObjectAbility"),
            "expected parsed granted ability payload, got {debug}"
        );
        assert!(
            debug.contains("LoseLife"),
            "expected lose-life text to remain inside the granted ability payload, got {debug}"
        );
    }

    #[test]
    fn hellish_rebuke_lowering_keeps_lose_life_inside_granted_trigger() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellish Rebuke")
            .parse_text(
                "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
            )
            .expect("hellish rebuke grant line should parse");

        let spell_effects = def
            .spell_effect
            .as_ref()
            .expect("hellish rebuke should compile to spell effects");
        assert_eq!(
            spell_effects.len(),
            1,
            "lose life should not be hoisted to a top-level spell effect: {spell_effects:?}"
        );

        let apply = spell_effects[0]
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("top-level spell effect should be a continuous grant");
        let granted = apply
            .modification
            .as_ref()
            .and_then(|modification| match modification {
                crate::continuous::Modification::AddAbilityGeneric(ability) => Some(ability),
                crate::continuous::Modification::AddAbility(static_ability) => {
                    static_ability.granted_inline_ability()
                }
                _ => None,
            })
            .expect("continuous effect should grant an inline ability");

        let AbilityKind::Triggered(triggered) = &granted.kind else {
            panic!("expected granted inline ability to be triggered: {granted:?}");
        };
        assert_eq!(
            triggered.effects.len(),
            2,
            "granted trigger should keep both sacrifice and lose-life effects: {triggered:?}"
        );
        assert!(
            triggered.effects.iter().any(|effect| effect
                .downcast_ref::<crate::effects::LoseLifeEffect>()
                .is_some()),
            "granted trigger should include lose-life effect: {triggered:?}"
        );

        let trigger_debug = format!("{:?}", triggered.trigger);
        assert!(
            trigger_debug.contains("damaged_player: Some("),
            "granted trigger should constrain the damaged player: {trigger_debug}"
        );
    }

    #[test]
    fn mixed_keyword_and_quoted_trigger_grant_stays_targeted() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Strength of Will")
            .parse_text(
                "Until end of turn, target creature you control gains indestructible and \"Whenever this creature is dealt damage, put that many +1/+1 counters on it.\"",
            )
            .expect("strength of will grant line should parse");

        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains("Indestructible"),
            "grant should keep the keyword ability: {debug}"
        );
        assert!(
            debug.contains("TriggeredAbility"),
            "grant should keep the quoted triggered ability: {debug}"
        );

        let rendered = crate::compiled_text::canonical_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains("target creature you control gains indestructible")
                && rendered.contains("whenever this creature is dealt damage")
                && rendered.contains("put that many +1/+1 counters on it"),
            "grant should stay targeted in compiled text: {rendered}"
        );
    }
}
