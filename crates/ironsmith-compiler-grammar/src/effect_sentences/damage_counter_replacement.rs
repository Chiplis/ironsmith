//! "[choose that] for each 1 damage that would be dealt to you until your
//! next upkeep, you remove an echo counter from this enchantment instead."
//! (Soul Echo)
//!
//! One resolution-registered replacement (CR 614.1a): each 1 damage that
//! would be dealt to the player removes one counter of the named kind from
//! the named object instead, until the stated time. A leading "choose that"
//! is the deciding player's election of the whole replacement; the enclosing
//! "<player> may" owns that choice.

use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;

type Words<'a> = Vec<(usize, &'a OwnedLexToken)>;

fn eat(words: &Words<'_>, mut pos: usize, expected: &[&[&str]]) -> Option<usize> {
    for alternatives in expected {
        let (_, token) = words.get(pos)?;
        if !alternatives.iter().any(|word| token.is_word(word)) {
            return None;
        }
        pos += 1;
    }
    Some(pos)
}

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let words: Words<'_> = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.as_word().is_some())
        .collect();
    let mut pos = eat(&words, 0, &[&["choose"], &["that"]]).unwrap_or(0);
    let Some(next) = eat(
        &words,
        pos,
        &[
            &["for"],
            &["each"],
            &["1", "one"],
            &["damage"],
            &["that"],
            &["would"],
            &["be"],
            &["dealt"],
            &["to"],
            &["you"],
        ],
    ) else {
        return Ok(None);
    };
    pos = next;
    let (expiry, next) = if let Some(next) =
        eat(&words, pos, &[&["until"], &["your"], &["next"], &["upkeep"]])
    {
        (ironsmith_core::RegisteredReplacementExpiry::YourNextUpkeep, next)
    } else if let Some(next) = eat(&words, pos, &[&["until"], &["your"], &["next"], &["turn"]]) {
        (ironsmith_core::RegisteredReplacementExpiry::YourNextTurn, next)
    } else if let Some(next) = eat(&words, pos, &[&["this"], &["turn"]]) {
        (ironsmith_core::RegisteredReplacementExpiry::EndOfTurn, next)
    } else {
        return Ok(None);
    };
    pos = next;
    let Some(next) = eat(&words, pos, &[&["you"], &["remove"], &["a", "an", "one"]]) else {
        return Ok(None);
    };
    pos = next;
    let Some(counter_offset) = words[pos..]
        .iter()
        .position(|(_, token)| token.is_word("counter"))
    else {
        return Ok(None);
    };
    if counter_offset == 0 {
        return Ok(None);
    }
    let counter_words = words[pos..=pos + counter_offset]
        .iter()
        .filter_map(|(_, token)| token.as_word().map(str::to_ascii_lowercase))
        .collect::<Vec<_>>();
    let counter_words = counter_words.iter().map(String::as_str).collect::<Vec<_>>();
    let Some(counter_type) = crate::grammar::filters::parse_counter_type_words(&counter_words)
    else {
        return Ok(None);
    };
    pos += counter_offset + 1;
    let Some(object_start) = eat(&words, pos, &[&["from"]]) else {
        return Ok(None);
    };
    let Some(((instead_index, instead), object_words)) = words[object_start..].split_last() else {
        return Ok(None);
    };
    if !instead.is_word("instead") || object_words.is_empty() {
        return Ok(None);
    }
    let object_tokens = crate::util::trim_edge_punctuation_tokens(
        &tokens[object_words[0].0..*instead_index],
    );
    let object = crate::util::parse_target_phrase(object_tokens)?;
    Ok(Some(EffectAst::RegisterDamageCounterReplacement {
        player: PlayerFilter::You,
        counter_type,
        object,
        expiry,
    }))
}
