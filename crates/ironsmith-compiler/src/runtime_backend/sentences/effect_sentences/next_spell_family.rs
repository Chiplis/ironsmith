use super::super::keyword_static::parse_ability_line;
use super::super::lexer::{OwnedLexToken, TokenWordView};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::util::parse_subject;
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst, SubjectAst, TextSpan};
use crate::target::{ObjectFilter, PlayerFilter};

fn synth_word_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect()
}

fn find_phrase_start(words: &[&str], phrase: &[&str]) -> Option<usize> {
    if phrase.is_empty() || words.len() < phrase.len() {
        return None;
    }

    (0..=words.len() - phrase.len()).find(|start| {
        words[*start..*start + phrase.len()]
            .iter()
            .zip(phrase.iter())
            .all(|(word, expected)| word == expected)
    })
}

fn find_word_index(words: &[&str], predicate: impl Fn(&str) -> bool) -> Option<usize> {
    words.iter().position(|word| predicate(word))
}

fn ends_with_words(words: &[&str], suffix: &[&str]) -> bool {
    words.len() >= suffix.len()
        && words[words.len() - suffix.len()..]
            .iter()
            .zip(suffix.iter())
            .all(|(word, expected)| word == expected)
}

fn next_spell_grant_player_ast(filter: &ObjectFilter) -> Option<PlayerAst> {
    match filter.cast_by.as_ref()? {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(base) => match base.as_ref() {
            PlayerFilter::Any => Some(PlayerAst::Target),
            PlayerFilter::Opponent => Some(PlayerAst::TargetOpponent),
            _ => None,
        },
        _ => None,
    }
}

fn next_spell_grant_shared_cast_suffix<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    for suffix in [
        &["you", "cast"][..],
        &["they", "cast"][..],
        &["that", "player", "cast"][..],
        &["target", "player", "cast"][..],
        &["target", "opponent", "cast"][..],
        &["opponent", "cast"][..],
        &["opponents", "cast"][..],
    ] {
        if ends_with_words(words, suffix) {
            return Some(suffix);
        }
    }
    None
}

fn parse_next_spell_grant_ability(
    words: &[&str],
) -> Option<crate::cards::builders::GrantedAbilityAst> {
    let tokens = synth_word_tokens(words);
    let actions = parse_ability_line(&tokens)?;
    let [action] = actions.as_slice() else {
        return None;
    };
    if !action.lowers_to_static_ability() {
        return None;
    }
    Some(action.clone().into())
}

fn parse_next_spell_subject_filter(words: &[&str]) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = synth_word_tokens(words);
    let filter = parse_object_filter_lexed(&tokens, false)?;
    if filter.cast_by.is_none() {
        return Ok(None);
    }
    Ok(Some(filter))
}

fn parse_when_next_cast_grant_sentence(
    clause_words: &[&str],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause_words.starts_with(&["when"]) {
        return Ok(None);
    }
    let Some(next_idx) = find_word_index(clause_words, |word| word == "next") else {
        return Ok(None);
    };
    if next_idx == 0 || clause_words.get(next_idx + 1) != Some(&"cast") {
        return Ok(None);
    }
    let caster_words = &clause_words[1..next_idx];
    let player = match caster_words {
        ["you"] => PlayerAst::You,
        ["an", "opponent"] | ["opponent"] => PlayerAst::Opponent,
        ["that", "player"] => PlayerAst::That,
        ["target", "player"] => PlayerAst::Target,
        ["target", "opponent"] => PlayerAst::TargetOpponent,
        _ => return Ok(None),
    };
    let cast_by_words: &[&str] = match caster_words {
        ["you"] => &["you", "cast"],
        ["an", "opponent"] | ["opponent"] => &["opponent", "cast"],
        ["that", "player"] => &["that", "player", "cast"],
        ["target", "player"] => &["target", "player", "cast"],
        ["target", "opponent"] => &["target", "opponent", "cast"],
        _ => return Ok(None),
    };

    let Some(this_turn_idx) = find_phrase_start(clause_words, &["this", "turn"]) else {
        return Ok(None);
    };
    if this_turn_idx <= next_idx + 2 {
        return Ok(None);
    }
    let mut subject_words = &clause_words[next_idx + 2..this_turn_idx];
    while matches!(subject_words.first(), Some(&"a" | &"an" | &"the")) {
        subject_words = &subject_words[1..];
    }
    if subject_words.is_empty() {
        return Ok(None);
    }

    let after_turn = &clause_words[this_turn_idx + 2..];
    let ability_words = if after_turn.starts_with(&["it", "gains"]) {
        &after_turn[2..]
    } else if after_turn.starts_with(&["it", "has"]) {
        &after_turn[2..]
    } else {
        return Ok(None);
    };
    let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
        return Ok(None);
    };

    let filter_words = if let Some(from_idx) = find_phrase_start(subject_words, &["from"]) {
        [
            &subject_words[..from_idx],
            cast_by_words,
            &subject_words[from_idx..],
        ]
        .concat()
    } else {
        [subject_words, cast_by_words].concat()
    };
    let Some(filter) = parse_next_spell_subject_filter(&filter_words)? else {
        return Ok(None);
    };

    Ok(Some(vec![
        EffectAst::subject_verb_grant_next_spell_ability_this_turn(player, filter, ability),
    ]))
}

pub(crate) fn parse_next_spell_grant_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_word_view = TokenWordView::new(tokens);
    let clause_words = clause_word_view.word_refs();
    if let Some(effects) = parse_when_next_cast_grant_sentence(&clause_words)? {
        return Ok(Some(effects));
    }
    if !clause_words.starts_with(&["the", "next"]) {
        return Ok(None);
    }

    if let Some(have_idx) = find_phrase_start(&clause_words, &["each", "have"]) {
        let subject_words = &clause_words[..have_idx];
        let ability_words = &clause_words[have_idx + 2..];
        let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
            return Ok(None);
        };
        if !subject_words.ends_with(&["this", "turn"]) {
            return Ok(None);
        }
        let subject_without_turn = &subject_words[..subject_words.len() - 2];
        let Some(shared_cast_words) = next_spell_grant_shared_cast_suffix(subject_without_turn)
        else {
            return Ok(None);
        };
        let shared_prefix =
            &subject_without_turn[2..subject_without_turn.len() - shared_cast_words.len()];
        let Some(split_idx) = find_phrase_start(shared_prefix, &["and", "the", "next"]) else {
            return Ok(None);
        };
        let first_subject = &shared_prefix[..split_idx];
        let second_subject = &shared_prefix[split_idx + 3..];
        if first_subject.is_empty() || second_subject.is_empty() {
            return Ok(None);
        }

        let first_filter_words = [first_subject, shared_cast_words].concat();
        let second_filter_words = [second_subject, shared_cast_words].concat();
        let Some(first_filter) = parse_next_spell_subject_filter(&first_filter_words)? else {
            return Ok(None);
        };
        let Some(second_filter) = parse_next_spell_subject_filter(&second_filter_words)? else {
            return Ok(None);
        };
        let Some(player) = next_spell_grant_player_ast(&first_filter) else {
            return Ok(None);
        };

        return Ok(Some(vec![
            EffectAst::subject_verb_grant_next_spell_ability_this_turn(
                player,
                first_filter,
                ability.clone(),
            ),
            EffectAst::subject_verb_grant_next_spell_ability_this_turn(
                player,
                second_filter,
                ability,
            ),
        ]));
    }

    let Some(has_idx) = find_word_index(&clause_words, |word| matches!(word, "has" | "have"))
    else {
        return Ok(None);
    };
    let subject_words = &clause_words[..has_idx];
    let ability_words = &clause_words[has_idx + 1..];
    let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
        return Ok(None);
    };
    if !subject_words.ends_with(&["this", "turn"]) {
        return Ok(None);
    }

    let filter_words = &subject_words[2..subject_words.len() - 2];
    let Some(filter) = parse_next_spell_subject_filter(filter_words)? else {
        return Ok(None);
    };
    let Some(player) = next_spell_grant_player_ast(&filter) else {
        return Ok(None);
    };
    Ok(Some(vec![
        EffectAst::subject_verb_grant_next_spell_ability_this_turn(player, filter, ability),
    ]))
}
