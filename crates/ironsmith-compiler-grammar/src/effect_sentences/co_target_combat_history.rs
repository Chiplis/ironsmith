//! A counted instruction whose recipient was blocked by another declared target.
use crate::util::parse_target_phrase;
use crate::cards::builders::{CardTextError, EffectAst, TargetAst};
use crate::lexer::{OwnedLexToken, TokenWordView};

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let Some(first) = sentences.first() else { return Ok(None); };
    let first = crate::util::trim_edge_punctuation_tokens(first);
    let view = TokenWordView::new(first);
    let words = view.word_refs();
    if !words.starts_with(&["put", "x"]) { return Ok(None); }
    let Some(on) = words.iter().position(|word| *word == "on") else { return Ok(None); };
    let Some(that) = words.windows(2).position(|pair| pair == ["that", "target"]) else { return Ok(None); };
    let Some(blocked) = words.iter().position(|word| *word == "blocked") else { return Ok(None); };
    if words.get(blocked..blocked + 3) != Some(&["blocked", "this", "turn"][..])
        || words.get(blocked + 3..) != Some(&["where", "x", "is", "the", "power", "of", "that", "blocked", "creature"][..])
        || on + 1 >= that || that + 2 >= blocked { return Ok(None); }
    let span = |start, end| view.token_span_for_words(start, end);
    let Some(counter_span) = span(2, on) else { return Ok(None); };
    let Some(counter) = crate::util::parse_counter_type_from_tokens(&first[counter_span]) else { return Ok(None); };
    let Some(creature_span) = span(on + 1, that) else { return Ok(None); };
    let Some(blocker_span) = span(that + 1, blocked) else { return Ok(None); };
    let blocker = parse_target_phrase(&first[blocker_span])?;
    let mut creature = parse_target_phrase(&first[creature_span])?;
    let TargetAst::Object(filter, _, _) = &mut creature else { return Ok(None); };
    filter.blocked_by = Some(crate::filter::ObjectRef::Target);
    let blocker_tag = crate::util::helper_tag_for_tokens(first, "co_target_blocker");
    let creature_tag = crate::util::helper_tag_for_tokens(first, "co_target_attacker");
    let mut effects = vec![
        EffectAst::TagAffected { effect: Box::new(EffectAst::subject_verb_explicit_target_only(blocker)), tag: blocker_tag },
        EffectAst::TagAffected { effect: Box::new(EffectAst::subject_verb_explicit_target_only(creature)), tag: creature_tag.clone() },
        EffectAst::subject_verb_put_counters(counter,
            crate::effect::Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(creature_tag.clone().into()))),
            TargetAst::Tagged(creature_tag, None), None, false),
    ];
    // The recipient is the last tagged target, so ordinary quoted-ability
    // grants keep their normal reference-resolution and duration handling.
    for sentence in sentences.iter().skip(1) {
        effects.extend(super::dispatch_entry::parse_effect_sentences_lexed(sentence)?);
    }
    Ok(Some(effects))
}
