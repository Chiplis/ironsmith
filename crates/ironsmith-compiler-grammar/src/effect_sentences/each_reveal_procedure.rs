//! "Two target players each reveal the top card of their library. They each
//! lose life equal to the mana value of the card revealed by the other
//! player. Then they each put the card they revealed into their hand."
//! (Parker Luck)
//!
//! Every target player's top card joins one "revealed this way" pool (a
//! revealing look inside a player loop adds to the pool rather than starting
//! it over). The following statements read that pool per player: the cards
//! the *other* players revealed, or the player's own revealed card.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ForEachEffectAst, LifeResourceActionAst, ObjectFilter,
    PlayerAst, ReturnControllerAst, SubjectVerbActionAst, SubjectVerbRoleAst, TargetAst, Value,
};
use crate::target::PlayerFilter;
use crate::zone::Zone;

pub(super) struct EachRevealGroup {
    count: ChoiceCount,
    effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
    /// A fixed-shape contest that already consumed all of its statements.
    closed: bool,
}

fn words_are(words: &[String], expected: &[&str]) -> bool {
    words.len() == expected.len()
        && words
            .iter()
            .zip(expected)
            .all(|(word, expected)| word.as_str() == *expected)
}

/// "Each player reveals the top card of their library. The player who
/// reveals the card with the greatest mana value may change the target or
/// targets. If two or more cards are tied for greatest, the target or targets
/// remain unchanged. [Changing targets this way doesn't trigger abilities of
/// permanents named <this card>.]" (Psychic Battle): a reveal contest whose
/// sole winner may choose new targets for the triggering spell or ability.
fn open_targets_contest(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Option<EachRevealGroup> {
    let words = |offset: usize| sentences.get(sentence_idx + offset).map(sentence_words);
    if !words_are(
        &words(0)?,
        &["each", "player", "reveals", "the", "top", "card", "of", "their", "library"],
    ) || !words_are(
        &words(1)?,
        &[
            "the", "player", "who", "reveals", "the", "card", "with", "the", "greatest", "mana",
            "value", "may", "change", "the", "target", "or", "targets",
        ],
    ) || !words_are(
        &words(2)?,
        &[
            "if", "two", "or", "more", "cards", "are", "tied", "for", "greatest", "the", "target",
            "or", "targets", "remain", "unchanged",
        ],
    ) {
        return None;
    }
    let doesnt_trigger_same_named_sources = words(3).is_some_and(|words| {
        let text = words.join(" ").replace('\'', "");
        ["changing targets this way doesnt trigger", "changing targets this way doesn t trigger"]
            .iter()
            .any(|head| text.starts_with(head))
            && text
                .split_once(" abilities of permanents named ")
                .is_some_and(|(_, name)| !name.trim().is_empty())
    });
    Some(EachRevealGroup {
        count: ChoiceCount::exactly(0),
        effects: vec![EffectAst::RevealContestWinnerMayChangeTriggeringTargets {
            doesnt_trigger_same_named_sources,
        }],
        first_sentence: sentence_idx,
        consumed: if doesnt_trigger_same_named_sources { 4 } else { 3 },
        closed: true,
    })
}

fn sentence_words(sentence: &SentenceInput) -> Vec<String> {
    crate::lexer::token_word_refs(sentence.lowered())
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn revealed_pool() -> ObjectFilter {
    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::RevealedThisWay.bind())
}

fn each_target_player(count: ChoiceCount, effect: EffectAst) -> EffectAst {
    EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers {
        count,
        filter: PlayerFilter::Any,
        effects: vec![effect],
    })
}

/// "<N> target players each reveal the top card of their library".
fn opening(sentence: &SentenceInput) -> Option<ChoiceCount> {
    let words = sentence_words(sentence);
    let [amount, rest @ ..] = words.as_slice() else {
        return None;
    };
    let expected = [
        "target", "players", "each", "reveal", "the", "top", "card", "of", "their", "library",
    ];
    if rest.len() != expected.len() || rest.iter().zip(expected).any(|(word, expected)| word.as_str() != expected)
    {
        return None;
    }
    let count = crate::util::parse_number_word_u32(amount)?;
    (count >= 2).then(|| ChoiceCount::exactly(count as usize))
}

/// A statement over the pool, as one more loop over the same target players.
fn statement(sentence: &SentenceInput, count: ChoiceCount) -> Option<EffectAst> {
    let words = sentence_words(sentence);
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let words = match words.as_slice() {
        ["then", rest @ ..] => rest,
        all => all,
    };
    match words {
        [
            "they", "each", "lose", "life", "equal", "to", "the", "mana", "value", "of", "the",
            "card", "revealed", "by", "the", "other", "player",
        ] => {
            let mut others = revealed_pool();
            others.owner = Some(PlayerFilter::Excluding {
                base: Box::new(PlayerFilter::Any),
                excluded: Box::new(PlayerFilter::IteratedPlayer),
            });
            Some(each_target_player(
                count,
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::That,
                    SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                        amount: Value::TotalManaValue(others),
                    }),
                ),
            ))
        }
        [
            "they", "each", "put", "the", "card", "they", "revealed", "into", "their", "hand",
        ] => {
            let mut own = revealed_pool();
            own.owner = Some(PlayerFilter::IteratedPlayer);
            Some(each_target_player(
                count,
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Object(own, None, None),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
            ))
        }
        _ => None,
    }
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<EachRevealGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    if let Some(group) = open_targets_contest(sentences, sentence_idx) {
        return Ok(Some(group));
    }
    let Some(count) = opening(sentence) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if statement(next, count).is_none() {
        return Ok(None);
    }
    Ok(Some(EachRevealGroup {
        count,
        effects: vec![each_target_player(
            count,
            EffectAst::subject_verb_reveal_top_cards(
                PlayerAst::That,
                Value::Fixed(1),
                crate::tag::CompilerReferenceTag::RevealedThisWay.bind(),
            ),
        )],
        first_sentence: sentence_idx,
        consumed: 1,
        closed: false,
    }))
}

pub(super) fn continue_with(
    group: &mut EachRevealGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.closed {
        return Ok(false);
    }
    let Some(effect) = statement(sentence, group.count) else {
        return Ok(false);
    };
    group.effects.push(effect);
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: EachRevealGroup) -> Vec<EffectAst> {
    group.effects
}
