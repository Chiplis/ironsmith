//! "Choose first strike, vigilance, or lifelink. Creatures you control gain
//! that ability until end of turn." (Angelic Skirmisher, Linvala, Gabriel
//! Angelfire)
//!
//! The first statement names a closed list of keyword abilities; the second
//! grants "that ability" to a subject for a duration. The pair is the same
//! single choice as "<subject> gains your choice of <list> <duration>": the
//! choice is made as the ability resolves and only the chosen keyword is
//! granted. An explicit "at random" makes that resolution-time choice random.
//! Both statements are read here so the bare choice never stands as an object
//! selection with no referent.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, EffectAst, GrantedAbilityAst, KeywordAction,
    ObjectChoiceEffectAst, TargetAst,
};
use crate::grammar::effects::gain_ability_shapes as gain_shapes;
use crate::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};
use crate::zone::Zone;

pub(super) struct KeywordChoiceGroup {
    actions: Vec<KeywordAction>,
    random_mode_descriptions: Option<Vec<String>>,
    effects: Vec<EffectAst>,
    closed: bool,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn trim_period(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    crate::util::trim_edge_punctuation_tokens(tokens)
}

/// "choose <keyword>, <keyword>, or <keyword> [at random]"
fn keyword_choice(sentence: &SentenceInput) -> Option<(Vec<KeywordAction>, Option<Vec<String>>)> {
    let tokens = trim_period(sentence.lowered());
    let (first, rest) = tokens.split_first()?;
    if !first.is_word("choose") {
        return None;
    }
    let (options, random) = if rest.len() >= 2
        && rest[rest.len() - 2].is_word("at")
        && rest[rest.len() - 1].is_word("random")
    {
        (&rest[..rest.len() - 2], true)
    } else {
        (rest, false)
    };
    let options = trim_lexed_commas(options);
    if options.is_empty() {
        return None;
    }
    let actions = super::parse_choice_of_abilities(options)?;
    if !random {
        return Some((actions, None));
    }
    let shape = gain_shapes::parse_ability_choice_shape(options)?;
    // Duplicate options must not be collapsed: that would change the odds.
    if shape.options.len() != actions.len() {
        return None;
    }
    let descriptions = shape
        .options
        .iter()
        .map(|option| crate::lexer::token_word_refs(option).join(" "))
        .collect();
    Some((actions, Some(descriptions)))
}

/// "<subject> gain(s) that ability <duration>"
fn grant_that_ability(
    sentence: &SentenceInput,
    actions: &[KeywordAction],
    random_mode_descriptions: Option<&[String]>,
) -> Option<EffectAst> {
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
    let subject_view = TokenWordView::new(subject_tokens);
    let subject_words = subject_view.to_word_refs();
    if crate::util::is_source_reference_words(&subject_words) {
        let target = TargetAst::Source(crate::util::span_from_tokens(subject_tokens));
        if let Some(descriptions) = random_mode_descriptions {
            let modes = actions.iter().zip(descriptions).map(|(action, description)| {
                ChooseOneModeAst {
                    description: description.clone(),
                    effects: vec![EffectAst::subject_verb_grant_abilities_to_target(
                        target.clone(), vec![GrantedAbilityAst::from(action.clone())], duration.clone(),
                    )],
                }
            }).collect();
            return Some(EffectAst::ObjectChoices(
                ObjectChoiceEffectAst::ChooseOneOfAtRandom { modes },
            ));
        }
        return Some(EffectAst::subject_verb_grant_abilities_choice_to_target(
            target,
            actions.iter().cloned().map(GrantedAbilityAst::from).collect(),
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
    if let Some(descriptions) = random_mode_descriptions {
        let modes = actions.iter().zip(descriptions).map(|(action, description)| {
            ChooseOneModeAst {
                description: description.clone(),
                effects: vec![EffectAst::subject_verb_grant_abilities_all(
                    filter.clone(), vec![GrantedAbilityAst::from(action.clone())], duration.clone(),
                )],
            }
        }).collect();
        return Some(EffectAst::ObjectChoices(
            ObjectChoiceEffectAst::ChooseOneOfAtRandom { modes },
        ));
    }
    Some(EffectAst::subject_verb_grant_abilities_choice_all(
        filter,
        actions.iter().cloned().map(GrantedAbilityAst::from).collect(),
        duration,
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
    let Some((actions, random_mode_descriptions)) = keyword_choice(sentence) else {
        return Ok(None);
    };
    if grant_that_ability(next, &actions, random_mode_descriptions.as_deref()).is_none() {
        return Ok(None);
    }
    Ok(Some(KeywordChoiceGroup {
        actions,
        random_mode_descriptions,
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
    let Some(effect) = grant_that_ability(
        sentence, &group.actions, group.random_mode_descriptions.as_deref(),
    ) else {
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

    #[test]
    fn random_choice_grants_only_the_selected_keyword_to_source() {
        let inputs = sentences(
            "Choose flying or indestructible at random. This Vehicle gains that ability until end of turn.",
        );
        let mut group = open(&inputs, 0).unwrap().expect("random keyword procedure opens");
        assert!(continue_with(&mut group, &inputs[1]).unwrap());
        let effects = finish(group);
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOfAtRandom { modes })] =
            effects.as_slice()
        else {
            panic!("expected runtime random mode choice: {effects:?}");
        };
        assert_eq!(modes.len(), 2);
        assert_eq!(modes[0].description, "flying");
        assert_eq!(modes[1].description, "indestructible");
        for mode in modes {
            assert_eq!(mode.effects.len(), 1);
            assert!(
                format!("{:?}", mode.effects[0]).contains("GrantAbilitiesToTarget"),
                "{:?}", mode.effects
            );
        }
        assert_ne!(modes[0].effects, modes[1].effects);
    }

    #[test]
    fn random_keyword_choice_without_linked_grant_does_not_claim_sentence() {
        let inputs = sentences("Choose flying or indestructible at random. Draw a card.");
        assert!(open(&inputs, 0).unwrap().is_none());
    }

    #[test]
    fn random_keyword_procedure_wins_before_generic_object_choice() {
        let tokens = crate::lexer::lex_line(
            "Choose flying or indestructible at random. This Vehicle gains that ability until end of turn.",
            0,
        ).unwrap();
        let effects = super::super::dispatch_entry::parse_effect_sentences_lexed(&tokens)
            .expect("complete linked choice parses");
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOfAtRandom { modes })]
                if modes.len() == 2
        ), "{effects:?}");
    }
}
