//! A complete conditional keyword action replaces the preceding action.
use super::*;
use crate::cards::builders::KeywordActionAst;
use crate::lexer::{OwnedLexToken, TokenKind};
use crate::util::trim_edge_punctuation_tokens;

fn action(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    if tokens.iter().any(|token| token.kind == TokenKind::Period) {
        return Ok(None);
    }
    match tokens.first().and_then(OwnedLexToken::as_word) {
        Some("investigate") => super::super::creation_handlers::parse_investigate(
            &tokens[1..], None,
        ).map(Some),
        Some("amass") => super::super::clause_pattern_helpers::parse_keyword_mechanic_clause(tokens),
        _ => Ok(None),
    }
}

fn replacement(tokens: &[OwnedLexToken]) -> Result<Option<(PredicateAst, EffectAst)>, CardTextError> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    if !tokens.first().is_some_and(|token| token.is_word("if"))
        || !tokens.last().is_some_and(|token| token.is_word("instead"))
        || tokens.iter().filter(|token| token.is_word("instead")).count() != 1
        || tokens.iter().any(|token| token.kind == TokenKind::Period)
    {
        return Ok(None);
    }
    let Some(comma) = tokens.iter().position(|token| token.kind == TokenKind::Comma)
    else { return Ok(None); };
    let Some(effect) = action(&tokens[comma + 1..tokens.len() - 1])?
    else { return Ok(None); };
    let predicate = crate::grammar::filters::parse_condition_predicate_lexed(&tokens[1..comma])?;
    Ok(Some((predicate, effect)))
}

pub(super) fn recognizes_replacement_sentence(tokens: &[OwnedLexToken]) -> bool {
    matches!(replacement(tokens), Ok(Some(_)))
}

pub(super) fn read(
    sentences: &[SentenceInput], sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else { return Ok(None); };
    let Some((predicate, replacement)) = replacement(second.lowered())?
    else { return Ok(None); };
    let Some(original) = action(first.lowered())?
    else { return Ok(None); };
    let same_action = match (&original, &replacement) {
        (EffectAst::SubjectVerb(left), EffectAst::SubjectVerb(right)) => {
            left.subject == right.subject && match (&left.action, &right.action) {
                (SubjectVerbActionAst::KeywordActions(KeywordActionAst::Investigate { .. }),
                 SubjectVerbActionAst::KeywordActions(KeywordActionAst::Investigate { .. })) => true,
                (SubjectVerbActionAst::KeywordActions(KeywordActionAst::Amass { subtype: left, .. }),
                 SubjectVerbActionAst::KeywordActions(KeywordActionAst::Amass { subtype: right, .. })) => left == right,
                _ => false,
            }
        }
        _ => false,
    };
    if !same_action { return Ok(None); }
    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate,
        if_true: vec![replacement],
        if_false: vec![original],
        attach_to_previous_ability: false,
    }]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn keyword_replacement_keeps_both_complete_actions_and_origin_gate() {
        for text in [
            "Investigate. If this spell was cast from a graveyard, investigate twice instead.",
            "Amass Goblins 1. If this spell was cast from a graveyard, amass Goblins 3 instead.",
        ] {
            let effects = crate::effect_sentences::parse_effect_sentences_lexed(&lex_line(text, 0).unwrap()).unwrap();
            let [EffectAst::SelfReplacement { predicate, if_true, if_false, attach_to_previous_ability }] = effects.as_slice()
            else { panic!("one self-replacement required: {text}: {effects:?}"); };
            assert!(!attach_to_previous_ability);
            assert_eq!(if_true.len(), 1);
            assert_eq!(if_false.len(), 1);
            assert_ne!(if_true, if_false);
            assert!(format!("{predicate:?}").contains("Graveyard"));
        }
    }

    #[test]
    fn keyword_replacement_rejects_dropped_tails_or_changed_actions() {
        for text in [
            "If this spell was cast from a graveyard, investigate twice and draw a card instead.",
            "If this spell was cast from a graveyard, amass Goblins 3 nonsense instead.",
            "If this spell was cast from a graveyard, investigate twice instead instead.",
        ] {
            assert!(!matches!(replacement(&lex_line(text, 0).unwrap()), Ok(Some(_))), "{text}");
        }
        let sentences = ["Investigate.", "If this spell was cast from a graveyard, amass Goblins 3 instead."]
            .into_iter().map(|text| SentenceInput::from_lexed(&lex_line(text, 0).unwrap())).collect::<Vec<_>>();
        assert!(read(&sentences, 0).unwrap().is_none());
    }
    #[test]
    fn investigate_then_keeps_targeted_scaling_and_proliferate_again_is_one_action() {
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&lex_line(
            "Investigate, then target creature gets +1/+1 until end of turn for each Clue you control.", 0,
        ).unwrap()).unwrap();
        let debug = format!("{effects:?}");
        assert!(debug.contains("Investigate") && debug.contains("Clue"), "{debug}");
        assert!(debug.contains("UntilEndOfTurn") || debug.contains("EndOfTurn"), "{debug}");
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&lex_line(
            "Proliferate, then proliferate again.", 0,
        ).unwrap()).unwrap();
        fn repetitions(effect: &EffectAst) -> usize {
            let mut n = usize::from(matches!(effect, EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::KeywordActions(KeywordActionAst::Proliferate { count: Value::Fixed(1) }), ..
            })));
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                n += nested.iter().map(repetitions).sum::<usize>();
            });
            n
        }
        assert_eq!(effects.iter().map(repetitions).sum::<usize>(), 2, "{effects:?}");
        assert!(crate::effect_sentences::parse_effect_sentences_lexed(&lex_line(
            "Proliferate again nonsense.", 0,
        ).unwrap()).is_err());
    }

    #[test]
    fn investigate_count_preserves_strict_hand_predicate_and_for_each_surface() {
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&lex_line(
            "Investigate once for each opponent who has more cards in hand than you.", 0,
        ).unwrap()).unwrap();
        let [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::KeywordActions(KeywordActionAst::Investigate { count }), ..
        })] = effects.as_slice() else { panic!("{effects:?}"); };
        assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        assert_eq!(count.unhinted(), &Value::CountPlayers(PlayerFilter::CardsInHandAtLeastMoreThanYou {
            base: Box::new(PlayerFilter::Opponent), count: 1,
        }));
        assert!(crate::grammar::shared_util::reference_shapes::parse_hand_advantage_player(
            &["opponent", "who", "has", "at", "least", "more", "cards", "in", "hand", "than", "you"],
        ).is_none());
        assert!(crate::grammar::shared_util::reference_shapes::parse_hand_advantage_player(
            &["opponent", "who", "has", "more", "cards", "in", "hand", "than", "you", "nonsense"],
        ).is_none());
    }

    #[test]
    fn malformed_investigate_player_counts_cannot_become_permanent_counts() {
        for text in [
            "Investigate once for each opponent who has more cards in hand than you nonsense.",
            "Investigate once for each opponent who has at least more cards in hand than you.",
        ] {
            assert!(crate::effect_sentences::parse_effect_sentences_lexed(&lex_line(text, 0).unwrap()).is_err(), "{text}");
        }
    }

}
