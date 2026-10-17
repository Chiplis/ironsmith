//! "<this> deals X damage to any target. If <this> would deal damage to a
//! creature, that damage can't be prevented or dealt instead to another
//! permanent or player." (Lava Burst): a rule about the damage this object
//! deals to a kind of recipient, keyed on both (CR 615 prevention, CR 614.9
//! redirection). The rider follows the instruction it governs in the text,
//! but it must already apply as that damage is dealt, so it is installed
//! before the instruction.
use crate::cards::builders::{CardTextError, EffectAst, OwnedLexToken};
use crate::effect::{Restriction, Until};
use crate::lexer::parser_token_word_refs;
use crate::target::ObjectFilter;

/// The rider sentence: its source filter, recipient filter and whether
/// redirection is barred as well as prevention.
fn rider(sentence: &[OwnedLexToken]) -> Result<Option<Restriction>, CardTextError> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let Some(comma) = sentence.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    let header = &sentence[..comma];
    let header_words = parser_token_word_refs(header);
    let ["if", rest @ ..] = header_words.as_slice() else {
        return Ok(None);
    };
    let Some(would) = rest.iter().position(|word| *word == "would") else {
        return Ok(None);
    };
    let (subject, predicate) = (&rest[..would], &rest[would + 1..]);
    if !crate::util::is_source_reference_words(subject)
        || predicate.len() < 4
        || predicate[..3] != ["deal", "damage", "to"]
    {
        return Ok(None);
    }
    let or_redirected = match parser_token_word_refs(&sentence[comma + 1..]).as_slice() {
        ["that", "damage", "can't" | "cant" | "cannot", "be", "prevented"] => false,
        [
            "that", "damage", "can't" | "cant" | "cannot", "be", "prevented", "or", "dealt",
            "instead", "to", "another", "permanent", "or", "player",
        ] => true,
        _ => return Ok(None),
    };
    // The recipient words are the header's trailing tokens ("a creature").
    let recipient_count = predicate.len() - 3;
    let word_tokens = header
        .iter()
        .filter(|token| token.as_word().is_some())
        .count();
    if word_tokens != header_words.len() || recipient_count > header.len() {
        return Ok(None);
    }
    let recipient_tokens = &header[header.len() - recipient_count..];
    let Ok(recipients) = crate::object_filters::parse_object_filter(recipient_tokens, false) else {
        return Ok(None);
    };
    if recipients == ObjectFilter::default() || recipients.source {
        return Ok(None);
    }
    let mut sources = ObjectFilter::source();
    sources.source_surface = crate::util::source_reference_surface_for_words(subject);
    Ok(Some(Restriction::PreventDamageFromTo {
        sources,
        recipients,
        or_redirected,
    }))
}

pub(crate) fn read(sentences: &[&[OwnedLexToken]]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let &[instruction, rider_sentence] = sentences else {
        return Ok(None);
    };
    let Some(restriction) = rider(rider_sentence)? else {
        return Ok(None);
    };
    // The governed instruction is this object's own damage.
    if !parser_token_word_refs(instruction)
        .iter()
        .any(|word| matches!(*word, "deal" | "deals"))
    {
        return Ok(None);
    }
    let instruction_effects = super::parse_effect_chain(instruction)?;
    if instruction_effects.is_empty() {
        return Ok(None);
    }
    let mut effects = vec![EffectAst::subject_verb_cant(
        restriction,
        Until::EndOfTurn,
        None,
    )];
    effects.extend(instruction_effects);
    Ok(Some(effects))
}
