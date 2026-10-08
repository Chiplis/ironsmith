//! "Choose first strike, vigilance, or lifelink. Creatures you control gain
//! that ability until end of turn." (Angelic Skirmisher, Linvala, Gabriel
//! Angelfire)
//!
//! The first statement names a closed list of keyword abilities; the second
//! grants "that ability" to a subject for a duration. The pair is the same
//! single choice as "<subject> gains your choice of <list> <duration>": the
//! choice is made as the ability resolves and only the chosen keyword is
//! granted. Both statements are read here so the bare choice never stands as
//! an object selection with no referent.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst, GrantedAbilityAst, KeywordAction, TargetAst};
use crate::grammar::effects::gain_ability_shapes as gain_shapes;
use crate::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};
use crate::zone::Zone;

pub(super) struct KeywordChoiceGroup {
    actions: Vec<KeywordAction>,
    effects: Vec<EffectAst>,
    closed: bool,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn trim_period(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    crate::util::trim_edge_punctuation_tokens(tokens)
}

/// "choose <keyword>, <keyword>, or <keyword>"
fn keyword_choice(sentence: &SentenceInput) -> Option<Vec<KeywordAction>> {
    let tokens = trim_period(sentence.lowered());
    let (first, rest) = tokens.split_first()?;
    if !first.is_word("choose") {
        return None;
    }
    let options = trim_lexed_commas(rest);
    if options.is_empty() {
        return None;
    }
    super::parse_choice_of_abilities(options)
}

/// "<subject> gain(s) that ability <duration>"
fn grant_that_ability(sentence: &SentenceInput, actions: &[KeywordAction]) -> Option<EffectAst> {
    let tokens = trim_period(sentence.lowered());
    let gain_idx = tokens
        .iter()
        .position(|token| token.is_any_word(&["gain", "gains"]))?;
    let subject_tokens = trim_lexed_commas(&tokens[..gain_idx]);
    if subject_tokens.is_empty() {
        return None;
    }
    let after = &tokens[gain_idx + 1..];
    let (that, rest) = after.split_first()?;
    let (ability, tail) = rest.split_first()?;
    if !that.is_word("that") || !ability.is_word("ability") {
        return None;
    }
    let tail_view = TokenWordView::new(tail);
    let tail_words = tail_view.to_word_refs();
    let duration = if tail_words.is_empty() {
        crate::effect::Until::Forever
    } else {
        let shape = gain_shapes::parse_simple_ability_duration_shape(&tail_words)?;
        if shape.start != 0 || shape.len != tail_words.len() || shape.condition.is_some() {
            return None;
        }
        shape.duration
    };
    let abilities = actions
        .iter()
        .cloned()
        .map(GrantedAbilityAst::from)
        .collect::<Vec<_>>();
    let subject_view = TokenWordView::new(subject_tokens);
    let subject_words = subject_view.to_word_refs();
    if crate::util::is_source_reference_words(&subject_words) {
        return Some(EffectAst::subject_verb_grant_abilities_choice_to_target(
            TargetAst::Source(crate::util::span_from_tokens(subject_tokens)),
            abilities,
            duration,
        ));
    }
    let mut filter =
        crate::grammar::primitives::probe_shape(crate::object_filters::parse_object_filter_lexed(
            subject_tokens,
            false,
        ))?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    Some(EffectAst::subject_verb_grant_abilities_choice_all(
        filter, abilities, duration,
    ))
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<KeywordChoiceGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    // Only a choice followed by its grant forms the procedure.
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let Some(actions) = keyword_choice(sentence) else {
        return Ok(None);
    };
    if grant_that_ability(next, &actions).is_none() {
        return Ok(None);
    }
    Ok(Some(KeywordChoiceGroup {
        actions,
        effects: Vec::new(),
        closed: false,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

pub(super) fn continue_with(
    group: &mut KeywordChoiceGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.closed {
        return Ok(false);
    }
    let Some(effect) = grant_that_ability(sentence, &group.actions) else {
        return Ok(false);
    };
    group.effects.push(effect);
    group.closed = true;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: KeywordChoiceGroup) -> Vec<EffectAst> {
    group.effects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sentences(text: &str) -> Vec<SentenceInput> {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        crate::lexer::split_lexed_sentences(&tokens)
            .into_iter()
            .map(SentenceInput::from_lexed)
            .collect()
    }

    #[test]
    fn choice_and_grant_form_one_choice_grant() {
        let inputs = sentences(
            "Choose first strike, vigilance, or lifelink. Creatures you control gain that ability until end of turn.",
        );
        let mut group = open(&inputs, 0).unwrap().expect("procedure opens");
        assert_eq!(group.actions.len(), 3);
        assert!(continue_with(&mut group, &inputs[1]).unwrap());
        let effects = finish(group);
        assert_eq!(effects.len(), 1);
        assert!(format!("{effects:?}").contains("GrantAbilitiesChoiceAll"), "{effects:?}");
    }

    #[test]
    fn a_choice_without_its_grant_does_not_open() {
        let inputs = sentences("Choose hexproof or indestructible. Draw a card.");
        assert!(open(&inputs, 0).unwrap().is_none());
    }
}
