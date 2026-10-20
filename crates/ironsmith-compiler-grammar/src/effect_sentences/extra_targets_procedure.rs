//! "Destroy target artifact. For each additional {1}{R} you paid, destroy
//! another target artifact. For each additional {1}{G} you paid, destroy
//! another target artifact, and you gain 1 life." (Primitive Justice)
//!
//! The first instruction names one target; each following "for each <paid
//! additional cost>, <same verb> another target <same filter>" adds that many
//! more targets of the same kind. All of them are chosen as the spell is cast
//! (CR 601.2c), so the program is one instruction over 1 + (each count)
//! targets. The instructions then execute in printed order, with each
//! trailing ", and <rider>" performed even if its individual target is illegal.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ForEachEffectAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, TargetAst, Value, ZoneMoveActionAst,
};
use crate::lexer::OwnedLexToken;

pub(super) struct ExtraTargetsGroup {
    base: EffectAst,
    verb: String,
    filter_words: Vec<String>,
    extras: Vec<(Value, Vec<EffectAst>)>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    crate::util::trim_edge_punctuation_tokens(tokens)
}

fn destroy_target_mut(effect: &mut EffectAst) -> Option<&mut TargetAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { target, .. }),
            ..
        }) => Some(target),
        _ => None,
    }
}

/// "destroy target <filter>": the verb and the filter words.
fn base_instruction(sentence: &SentenceInput) -> Option<(EffectAst, String, Vec<String>)> {
    let tokens = trimmed(sentence.lowered());
    let words = crate::lexer::token_word_refs(tokens);
    let [verb, "target", filter @ ..] = words.as_slice() else {
        return None;
    };
    if *verb != "destroy" || filter.is_empty() || filter.contains(&"and") {
        return None;
    }
    let effects = super::parse_effect_sentence_lexed(tokens).ok()?;
    let [mut effect] = <[EffectAst; 1]>::try_from(effects).ok()?;
    if !matches!(destroy_target_mut(&mut effect)?, TargetAst::Object(_, Some(_), _)) {
        return None;
    }
    Some((
        effect,
        (*verb).to_string(),
        filter.iter().map(|word| (*word).to_string()).collect(),
    ))
}

/// "for each additional {1}{R} you paid": the times that additional payment
/// was made.
fn paid_additional_count(head: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::lexer::token_word_refs(head);
    if !words.starts_with(&["for", "each", "additional"]) || !words.ends_with(&["you", "paid"]) {
        return None;
    }
    let additional = head.iter().position(|token| token.is_word("additional"))?;
    let you = head.iter().rposition(|token| token.is_word("you"))?;
    let mut symbols = Vec::new();
    for token in head.get(additional + 1..you)? {
        let pip = crate::grammar::leaf::parse_leaf_surface_mana_pip_token(token)?;
        symbols.extend(pip.into_pip());
    }
    if symbols.is_empty() {
        return None;
    }
    let mana = crate::mana::ManaCost::from_symbols(symbols);
    Some(
        Value::TimesPaidLabel(crate::util::additional_mana_payment_reference(&mana))
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
    )
}

/// "for each <count>, <verb> another target <filter>[, and <rider>]".
fn extra_instruction(
    sentence: &SentenceInput,
    verb: &str,
    filter_words: &[String],
) -> Result<Option<(Value, Vec<EffectAst>)>, CardTextError> {
    let tokens = trimmed(sentence.lowered());
    let Some(comma) = tokens.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    let Some(count) = paid_additional_count(&tokens[..comma]) else {
        return Ok(None);
    };
    let body = &tokens[comma + 1..];
    let body_words = crate::lexer::token_word_refs(body);
    let head_len = 3 + filter_words.len();
    if body_words.len() < head_len
        || body_words[0] != verb
        || body_words[1] != "another"
        || body_words[2] != "target"
        || body_words[3..head_len]
            .iter()
            .zip(filter_words)
            .any(|(word, expected)| *word != expected.as_str())
    {
        return Ok(None);
    }
    // Token index just past the filter's last word.
    let mut seen_words = 0;
    let mut filter_end = None;
    for (index, token) in body.iter().enumerate() {
        if token.as_word().is_some() {
            seen_words += 1;
            if seen_words == head_len {
                filter_end = Some(index + 1);
                break;
            }
        }
    }
    let Some(filter_end) = filter_end else {
        return Ok(None);
    };
    let mut rest = &body[filter_end..];
    while rest.first().is_some_and(|token| token.is_comma()) {
        rest = &rest[1..];
    }
    if rest.is_empty() {
        return Ok(Some((count, Vec::new())));
    }
    if !rest[0].is_word("and") {
        return Ok(None);
    }
    let rider = trimmed(&rest[1..]);
    if rider.is_empty() {
        return Ok(None);
    }
    let effects = super::parse_effect_sentence_lexed(rider)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some((count, effects)))
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<ExtraTargetsGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let Some((base, verb, filter_words)) = base_instruction(sentence) else {
        return Ok(None);
    };
    // The following sentence must be an extra-target instruction for this
    // one; the loop then reads it (and any more) through `continue_with`.
    if extra_instruction(next, &verb, &filter_words)?.is_none() {
        return Ok(None);
    }
    Ok(Some(ExtraTargetsGroup {
        base,
        verb,
        filter_words,
        extras: Vec::new(),
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

pub(super) fn continue_with(
    group: &mut ExtraTargetsGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    let Some((count, rider)) = extra_instruction(sentence, &group.verb, &group.filter_words)?
    else {
        return Ok(false);
    };
    group.extras.push((count, rider));
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: ExtraTargetsGroup) -> Vec<EffectAst> {
    let ExtraTargetsGroup { base, extras, .. } = group;
    let Some(single) = (|| {
        let mut copy = base.clone();
        destroy_target_mut(&mut copy).cloned()
    })() else { return vec![base]; };
    let total = extras.iter().fold(Value::Fixed(1), |sum, (count, _)| {
        Value::Add(Box::new(sum), Box::new(count.clone()))
    });
    let target_spec = TargetAst::WithCountValue(
        Box::new(single), ChoiceCount::dynamic_x(), total,
    );
    let mut effects = vec![EffectAst::subject_verb_target_only(target_spec.clone())];
    let mut start = Value::Fixed(0);
    effects.push(EffectAst::ForEach(ForEachEffectAst::AnnouncedTargetSlotRange {
        target_spec: target_spec.clone(),
        start: start.clone(),
        count: Value::Fixed(1),
        target_action: Box::new(base.clone()),
        riders: Vec::new(),
    }));
    start = Value::Fixed(1);
    for (count, riders) in extras {
        effects.push(EffectAst::ForEach(ForEachEffectAst::AnnouncedTargetSlotRange {
            target_spec: target_spec.clone(),
            start: start.clone(),
            count: count.clone(),
            target_action: Box::new(base.clone()),
            riders,
        }));
        start = Value::Add(Box::new(start), Box::new(count));
    }
    effects
}
