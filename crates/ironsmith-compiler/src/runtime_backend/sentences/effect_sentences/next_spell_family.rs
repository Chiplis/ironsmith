use super::super::activation_and_restrictions::parse_single_word_keyword_action;
use super::super::lexer::{
    LexedClause, OwnedLexToken, word_slice_at_is, word_slice_ends_with, word_slice_ends_with_any,
    word_slice_eq_any, word_slice_find_any_word, word_slice_find_phrase_start,
    word_slice_find_word, word_slice_starts_with,
};
use super::super::object_filters::parse_simple_object_filter_words;
use super::super::util::{
    parse_card_type, parse_color, parse_subtype_flexible, strip_leading_article_word_refs,
};
use crate::cards::builders::{CardTextError, EffectAst, KeywordAction, PlayerAst};
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};

const SHARED_CAST_SUFFIXES: &[&[&str]] = &[
    &["you", "cast"],
    &["they", "cast"],
    &["that", "player", "cast"],
    &["target", "player", "cast"],
    &["target", "opponent", "cast"],
    &["opponent", "cast"],
    &["opponents", "cast"],
];
const CANT_BE_COUNTERED_CLAUSES: &[&[&str]] =
    &[&["cant", "be", "countered"], &["can't", "be", "countered"]];
const THIS_TURN_PHRASE: &[&str] = &["this", "turn"];
const AND_THE_NEXT_PHRASE: &[&str] = &["and", "the", "next"];
const IT_GAINS_PREFIX: &[&str] = &["it", "gains"];
const IT_HAS_PREFIX: &[&str] = &["it", "has"];
const HAS_OR_HAVE_WORDS: &[&str] = &["has", "have"];

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

fn next_spell_cast_suffix_player_filter(suffix: &[&str]) -> Option<PlayerFilter> {
    match suffix {
        ["you", "cast"] => Some(PlayerFilter::You),
        ["they", "cast"] | ["that", "player", "cast"] => Some(PlayerFilter::IteratedPlayer),
        ["target", "player", "cast"] => Some(PlayerFilter::Target(Box::new(PlayerFilter::Any))),
        ["target", "opponent", "cast"] => {
            Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        }
        ["opponent", "cast"] | ["opponents", "cast"] => Some(PlayerFilter::Opponent),
        _ => None,
    }
}

fn parse_next_spell_grant_ability(
    words: &[&str],
) -> Option<crate::cards::builders::GrantedAbilityAst> {
    if word_slice_eq_any(words, CANT_BE_COUNTERED_CLAUSES) {
        return Some(crate::cards::builders::GrantedAbilityAst::StaticAbility(
            StaticAbility::cant_be_countered_ability(),
        ));
    }

    let action = parse_next_spell_keyword_action_words(words)?;
    if !action.lowers_to_static_ability() {
        return None;
    }
    Some(action.into())
}

fn parse_next_spell_keyword_action_words(words: &[&str]) -> Option<KeywordAction> {
    match words {
        ["first", "strike"] => Some(KeywordAction::FirstStrike),
        ["double", "strike"] => Some(KeywordAction::DoubleStrike),
        ["battle", "cry"] => Some(KeywordAction::BattleCry),
        ["split", "second"] => Some(KeywordAction::SplitSecond),
        ["read", "ahead"] => Some(KeywordAction::ReadAhead),
        ["umbra", "armor"] => Some(KeywordAction::UmbraArmor),
        ["doctor", "companion"] => Some(KeywordAction::Marker("doctor companion")),
        ["protection", "from", value] => parse_color(value)
            .map(KeywordAction::ProtectionFrom)
            .or_else(|| (*value == "everything").then_some(KeywordAction::ProtectionFromEverything))
            .or_else(|| parse_card_type(value).map(KeywordAction::ProtectionFromCardType))
            .or_else(|| parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype)),
        [word] => parse_single_word_keyword_action(word),
        _ => None,
    }
}

fn next_spell_split_cast_suffix<'a>(
    words: &'a [&'a str],
) -> Option<(&'a [&'a str], &'a [&'a str])> {
    if !word_slice_ends_with_any(words, SHARED_CAST_SUFFIXES) {
        return None;
    }
    let suffix = SHARED_CAST_SUFFIXES
        .iter()
        .find(|suffix| word_slice_ends_with(words, suffix))?;
    let split = words.len() - suffix.len();
    Some((&words[..split], &words[split..]))
}

fn parse_next_spell_subject_filter(words: &[&str]) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((object_words, cast_suffix)) = next_spell_split_cast_suffix(words) else {
        return Ok(None);
    };
    if object_words.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_simple_object_filter_words(object_words, false).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported next-spell subject filter (clause: '{}')",
            words.join(" ")
        ))
    })?;
    filter.zone = Some(crate::zone::Zone::Stack);
    filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    filter.has_mana_cost = true;
    let Some(cast_by) = next_spell_cast_suffix_player_filter(cast_suffix) else {
        return Ok(None);
    };
    filter.cast_by = Some(cast_by);
    Ok(Some(filter))
}

fn parse_when_next_cast_grant_sentence(
    clause: &LexedClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = clause.word_refs();
    if !word_slice_starts_with(&clause_words, &["when"]) {
        return Ok(None);
    }
    let Some(next_idx) = word_slice_find_word(&clause_words, "next") else {
        return Ok(None);
    };
    if next_idx == 0 || !word_slice_at_is(&clause_words, next_idx + 1, "cast") {
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

    let Some(this_turn_idx) = word_slice_find_phrase_start(&clause_words, THIS_TURN_PHRASE) else {
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
    let ability_words = if word_slice_starts_with(after_turn, IT_GAINS_PREFIX) {
        &after_turn[2..]
    } else if word_slice_starts_with(after_turn, IT_HAS_PREFIX) {
        &after_turn[2..]
    } else {
        return Ok(None);
    };
    let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
        return Ok(None);
    };

    let filter_words = if let Some(from_idx) = word_slice_find_word(subject_words, "from") {
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
    if let Some(effects) = parse_when_next_cast_grant_sentence(&clause)? {
        return Ok(Some(effects));
    }
    if !word_slice_starts_with(&clause_words, &["the", "next"]) {
        return Ok(None);
    }

    if let Some(have_idx) = clause.find_phrase_start(&["each", "have"]) {
        let subject_words = &clause_words[..have_idx];
        let ability_words = &clause_words[have_idx + 2..];
        let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
            return Ok(None);
        };
        if !clause.before_word(have_idx).is_some_and(|subject_clause| {
            word_slice_ends_with(&subject_clause.word_refs(), THIS_TURN_PHRASE)
        }) {
            return Ok(None);
        }
        let subject_without_turn = &subject_words[..subject_words.len() - 2];
        let Some((shared_prefix, shared_cast_words)) =
            next_spell_split_cast_suffix(subject_without_turn)
        else {
            return Ok(None);
        };
        if !word_slice_starts_with(shared_prefix, &["the", "next"]) {
            return Ok(None);
        }
        let shared_prefix = &shared_prefix[2..];
        let Some(split_idx) = word_slice_find_phrase_start(shared_prefix, AND_THE_NEXT_PHRASE)
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

    let (subject_words, subject_end_idx, ability_words) =
        if let Some(has_idx) = word_slice_find_any_word(&clause_words, HAS_OR_HAVE_WORDS) {
            (
                &clause_words[..has_idx],
                has_idx,
                &clause_words[has_idx + 1..],
            )
        } else if let Some(cant_idx) = clause
            .find_phrase_start(&["cant", "be", "countered"])
            .or_else(|| clause.find_phrase_start(&["can't", "be", "countered"]))
        {
            (
                &clause_words[..cant_idx],
                cant_idx,
                &clause_words[cant_idx..],
            )
        } else {
            return Ok(None);
        };
    let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
        return Ok(None);
    };
    if !clause
        .before_word(subject_end_idx)
        .is_some_and(|subject_clause| {
            word_slice_ends_with(&subject_clause.word_refs(), THIS_TURN_PHRASE)
        })
    {
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
