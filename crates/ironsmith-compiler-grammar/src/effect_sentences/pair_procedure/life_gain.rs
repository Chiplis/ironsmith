//! A complete conditional gain replaces the preceding gain, never adds to it.
use super::*;
use crate::cards::builders::LifeResourceActionAst;
use crate::lexer::{OwnedLexToken, TokenKind};
use crate::util::trim_edge_punctuation_tokens;
use winnow::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Recipient { You, TargetPlayer, ThatPlayer }

fn fixed_gain(tokens: &[OwnedLexToken]) -> Option<(Recipient, i32)> {
    use crate::grammar::{leaf, primitives};
    use winnow::combinator::alt;
    primitives::probe_all(trim_edge_punctuation_tokens(tokens), (
        alt((
            primitives::phrase(&["you", "gain"]).value(Recipient::You),
            primitives::phrase(&["target", "player", "gains"]).value(Recipient::TargetPlayer),
            primitives::phrase(&["that", "player", "gains"]).value(Recipient::ThatPlayer),
        )),
        leaf::parse_leaf_number_prefix_lexed.try_map(i32::try_from),
        primitives::kw("life"),
        primitives::sentence_end(),
    ).map(|(recipient, amount, _, _)| (recipient, amount)), "fixed life-gain statement")
}

fn replacement(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PredicateAst, Recipient, i32)>, CardTextError> {
    let tokens = crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(tokens)
        .map_or(tokens, |label| label.body_tokens);
    let tokens = trim_edge_punctuation_tokens(tokens);
    if tokens.iter().any(|token| token.kind == TokenKind::Period) {
        return Ok(None);
    }
    let markers = tokens.iter().enumerate().filter(|(_, token)| token.is_word("instead"))
        .map(|(index, _)| index).collect::<Vec<_>>();
    let [instead] = markers.as_slice() else { return Ok(None); };
    let (condition, action) = if tokens.first().is_some_and(|token| token.is_word("if")) {
        if *instead + 1 != tokens.len() { return Ok(None); }
        let Some(comma) = tokens.iter().position(|token| token.kind == TokenKind::Comma)
        else { return Ok(None); };
        (&tokens[1..comma], &tokens[comma + 1..*instead])
    } else {
        if !tokens.get(*instead + 1).is_some_and(|token| token.is_word("if")) { return Ok(None); }
        (&tokens[*instead + 2..], &tokens[..*instead])
    };
    let Some((recipient, amount)) = fixed_gain(action) else { return Ok(None); };
    let predicate = crate::grammar::filters::parse_condition_predicate_lexed(condition)?;
    Ok(Some((predicate, recipient, amount)))
}

pub(super) fn recognizes_replacement_sentence(tokens: &[OwnedLexToken]) -> bool {
    matches!(replacement(tokens), Ok(Some(_)))
}

pub(super) fn read(
    sentences: &[SentenceInput], sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else { return Ok(None); };
    let Some((base_recipient, default_amount)) = fixed_gain(first.lowered())
    else { return Ok(None); };
    let Some((predicate, replacement_recipient, replacement_amount)) = replacement(second.lowered())?
    else { return Ok(None); };
    let player = match (base_recipient, replacement_recipient) {
        (Recipient::You, Recipient::You) => PlayerAst::You,
        (Recipient::TargetPlayer, Recipient::ThatPlayer) => PlayerAst::Target,
        // Changed or newly targeted recipients belong to a different program.
        _ => return Ok(None),
    };
    let gain = |amount| EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer, player,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount: Value::Fixed(amount) }),
    );
    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate,
        if_true: vec![gain(replacement_amount)],
        if_false: vec![gain(default_amount)],
        attach_to_previous_ability: false,
    }]))
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    #[test]
    fn complete_conditional_life_gains_are_mutually_exclusive_branches() {
        for text in [
            "You gain 4 life. If a creature died this turn, you gain 8 life instead.",
            "You gain 5 life. You gain 10 life instead if you control a creature with power 4 or greater.",
            "Target player gains 4 life. If you had a land enter the battlefield under your control this turn, that player gains 8 life instead.",
            "You gain 1 life. If you control creatures named Power Plant Worker and Tower Worker, you gain 3 life instead.",
        ] {
            let effects = crate::effect_sentences::parse_effect_sentences_lexed(&lex_line(text, 0).unwrap()).unwrap();
            let [EffectAst::SelfReplacement { if_true, if_false, attach_to_previous_ability, .. }] = effects.as_slice()
            else { panic!("one self-replacement program required: {text}: {effects:?}") };
            assert!(!attach_to_previous_ability);
            assert_eq!(if_true.len(), 1);
            assert_eq!(if_false.len(), 1);
        }
    }
    #[test]
    fn an_isolated_or_recipient_changing_gain_is_not_a_prior_gain_replacement() {
        assert!(crate::effect_sentences::parse_effect_sentences_lexed(&lex_line("You gain 8 life instead.", 0).unwrap()).is_err());
        let sentences = ["You gain 4 life.", "If a creature died this turn, target player gains 8 life instead."]
            .into_iter().map(|text| SentenceInput::from_lexed(&lex_line(text, 0).unwrap())).collect::<Vec<_>>();
        assert!(read(&sentences, 0).unwrap().is_none());
    }
}
