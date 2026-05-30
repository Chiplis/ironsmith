use super::super::keyword_static::parse_ability_line;
use super::super::lexer::{
    LexedClause, OwnedLexToken, word_slice_at_is, word_slice_ends_with,
    word_slice_find_phrase_start, word_slice_find_word, word_slice_first_is_any,
    word_slice_starts_with,
};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::util::strip_leading_article_word_refs;
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst, TextSpan};
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};

fn synth_word_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect()
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
        if word_slice_ends_with(words, suffix) {
            return Some(suffix);
        }
    }
    None
}

fn parse_next_spell_grant_ability(
    words: &[&str],
) -> Option<crate::cards::builders::GrantedAbilityAst> {
    if matches!(
        words,
        ["cant", "be", "countered"] | ["can't", "be", "countered"]
    ) {
        return Some(crate::cards::builders::GrantedAbilityAst::StaticAbility(
            StaticAbility::cant_be_countered_ability(),
        ));
    }

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
    if !word_slice_starts_with(clause_words, &["when"]) {
        return Ok(None);
    }
    let Some(next_idx) = word_slice_find_word(clause_words, "next") else {
        return Ok(None);
    };
    if next_idx == 0 || !word_slice_at_is(clause_words, next_idx + 1, "cast") {
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

    let Some(this_turn_idx) = word_slice_find_phrase_start(clause_words, &["this", "turn"]) else {
        return Ok(None);
    };
    if this_turn_idx <= next_idx + 2 {
        return Ok(None);
    }
    let subject_words = strip_leading_article_word_refs(&clause_words[next_idx + 2..this_turn_idx]);
    if subject_words.is_empty() {
        return Ok(None);
    }

    let after_turn = &clause_words[this_turn_idx + 2..];
    let ability_words = if word_slice_starts_with(after_turn, &["it", "gains"]) {
        &after_turn[2..]
    } else if word_slice_starts_with(after_turn, &["it", "has"]) {
        &after_turn[2..]
    } else {
        return Ok(None);
    };
    let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
        return Ok(None);
    };

    let filter_words =
        if let Some(from_idx) = word_slice_find_phrase_start(subject_words, &["from"]) {
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
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    if let Some(effects) = parse_when_next_cast_grant_sentence(&clause_words)? {
        return Ok(Some(effects));
    }
    if !clause.starts_with(&["the", "next"]) {
        return Ok(None);
    }

    if let Some(have_idx) = clause.find_phrase_start(&["each", "have"]) {
        let subject_words = &clause_words[..have_idx];
        let ability_words = &clause_words[have_idx + 2..];
        let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
            return Ok(None);
        };
        if !word_slice_ends_with(subject_words, &["this", "turn"]) {
            return Ok(None);
        }
        let subject_without_turn = &subject_words[..subject_words.len() - 2];
        let Some(shared_cast_words) = next_spell_grant_shared_cast_suffix(subject_without_turn)
        else {
            return Ok(None);
        };
        let shared_prefix =
            &subject_without_turn[2..subject_without_turn.len() - shared_cast_words.len()];
        let Some(split_idx) = word_slice_find_phrase_start(shared_prefix, &["and", "the", "next"])
        else {
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

    let (subject_words, ability_words) = if let Some(has_idx) = clause_words
        .iter()
        .position(|word| matches!(*word, "has" | "have"))
    {
        (&clause_words[..has_idx], &clause_words[has_idx + 1..])
    } else if let Some(cant_idx) = clause
        .find_phrase_start(&["cant", "be", "countered"])
        .or_else(|| clause.find_phrase_start(&["can't", "be", "countered"]))
    {
        (&clause_words[..cant_idx], &clause_words[cant_idx..])
    } else {
        return Ok(None);
    };
    let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
        return Ok(None);
    };
    if !word_slice_ends_with(subject_words, &["this", "turn"]) {
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
