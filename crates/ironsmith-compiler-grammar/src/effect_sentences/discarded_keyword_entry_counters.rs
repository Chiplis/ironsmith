//! "As this creature enters, discard any number of creature cards. It enters
//! with a flying counter on it if a card discarded this way has flying. The
//! same is true for first strike, ..." (Indominus Rex, Alpha): after the
//! preprocessor spells out one sentence per keyword, each sentence is a rider
//! on the entry program's discard. The discard is tagged once; each rider
//! adds that keyword counter to the entering object (CR 614.1c, 122.1b) when
//! any card discarded this way has the keyword. The as-enters program runs
//! while the object enters, so a counter put on the source is an entry
//! counter.

use crate::cards::builders::{
    ConditionalEffectAst, EffectAst, PredicateAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    TargetAst, ZoneMoveActionAst,
};
use crate::lexer::OwnedLexToken;

const DISCARDED_TAG_PREFIX: &str = "discarded_for_entry_counters";

fn split_condition(sentence: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], Vec<String>)> {
    let tokens = crate::util::trim_edge_punctuation_tokens(sentence);
    let if_idx = tokens.iter().position(|token| token.is_word("if"))?;
    let condition = crate::lexer::parser_token_word_refs(&tokens[if_idx + 1..])
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    Some((&tokens[..if_idx], condition))
}

fn is_discard(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard { .. }),
            ..
        })
    )
}

/// The tag of the statement's discard, tagging it on first use.
fn discarded_tag(effects: &mut [EffectAst], sentence: &[OwnedLexToken]) -> Option<crate::tag::TagRef> {
    for effect in effects.iter_mut().rev() {
        if let EffectAst::TagAffected { effect: inner, tag } = effect
            && is_discard(inner)
            && tag.key.as_str().contains(DISCARDED_TAG_PREFIX)
        {
            return Some(tag.clone());
        }
        if is_discard(effect) {
            let tag = crate::util::helper_tag_for_tokens(sentence, DISCARDED_TAG_PREFIX);
            let discard = std::mem::replace(effect, EffectAst::Sequence { effects: Vec::new() });
            *effect = EffectAst::TagAffected {
                effect: Box::new(discard),
                tag: tag.clone(),
            };
            return Some(tag);
        }
    }
    None
}

/// Bind "it enters with <counter> on it if a card discarded this way has
/// <keyword>" to the preceding discard.
pub(super) fn bind_discarded_keyword_entry_counter(
    effects: &mut Vec<EffectAst>,
    sentence: &[OwnedLexToken],
) -> bool {
    let Some((entry_tokens, condition)) = split_condition(sentence) else {
        return false;
    };
    if !crate::lexer::parser_token_word_refs(entry_tokens).starts_with(&["it", "enters", "with"]) {
        return false;
    }
    let condition = condition.iter().map(String::as_str).collect::<Vec<_>>();
    let ["a" | "any", "card", "discarded", "this", "way", "has", keyword @ ..] =
        condition.as_slice()
    else {
        return false;
    };
    if keyword.is_empty() {
        return false;
    }
    let mut filter_words = vec!["card", "with"];
    filter_words.extend(keyword.iter().copied());
    let filter_tokens = crate::lexer::synthetic_word_tokens(filter_words);
    let Ok(mut has_keyword) = crate::object_filters::parse_object_filter(&filter_tokens, false)
    else {
        return false;
    };
    has_keyword.zone = None;
    let Ok(Some(abilities)) = crate::keyword_static::parse_enters_with_counters_line(entry_tokens)
    else {
        return false;
    };
    let [ability] = abilities.as_slice() else {
        return false;
    };
    let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count } =
        ability.payload.clone()
    else {
        return false;
    };
    let Some(tag) = discarded_tag(effects, sentence) else {
        return false;
    };
    effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(tag, has_keyword),
        if_true: vec![EffectAst::subject_verb_put_counters(
            counter,
            count,
            TargetAst::Source(None),
            None,
            false,
        )],
        if_false: Vec::new(),
    }));
    true
}
