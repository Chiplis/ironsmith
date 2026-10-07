//! An amount/randomness replacement keeps the original player's declaration.
use super::*;
use crate::lexer::{OwnedLexToken, TokenKind};
use winnow::prelude::*;

fn sentence(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if tokens.last().is_some_and(|token| token.kind == TokenKind::Period) {
        &tokens[..tokens.len() - 1]
    } else { tokens }
}

fn discard(tokens: &[OwnedLexToken]) -> Option<(PlayerAst, i32, bool)> {
    use crate::grammar::{leaf, primitives};
    use winnow::combinator::{alt, opt};
    primitives::probe_all(sentence(tokens), (
        alt((
            primitives::phrase(&["target", "player", "discards"]).value(PlayerAst::Target),
            primitives::phrase(&["target", "opponent", "discards"]).value(PlayerAst::TargetOpponent),
            primitives::phrase(&["that", "player", "discards"]).value(PlayerAst::That),
        )),
        alt((primitives::kw("a").value(1),
            leaf::parse_leaf_number_prefix_lexed.try_map(i32::try_from))),
        alt((primitives::kw("card"), primitives::kw("cards"))),
        opt(primitives::phrase(&["at", "random"])),
    ).map(|(player, count, _, random)| (player, count, random.is_some())),
        "complete fixed discard action")
}

fn replacement(tokens: &[OwnedLexToken]) -> Result<Option<(PredicateAst, i32, bool)>, CardTextError> {
    let tokens = crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(tokens)
        .map_or(tokens, |label| label.body_tokens);
    let tokens = sentence(tokens);
    let markers = tokens.iter().enumerate().filter(|(_, token)| token.is_word("instead"))
        .map(|(index, _)| index).collect::<Vec<_>>();
    let [instead] = markers.as_slice() else { return Ok(None); };
    let (condition, action) = if tokens.first().is_some_and(|token| token.is_word("if")) {
        if *instead + 1 != tokens.len() { return Ok(None); }
        let Some(comma) = tokens.iter().position(OwnedLexToken::is_comma) else { return Ok(None); };
        (&tokens[1..comma], &tokens[comma + 1..*instead])
    } else {
        if !tokens.get(*instead + 1).is_some_and(|token| token.is_word("if")) { return Ok(None); }
        (&tokens[*instead + 2..], &tokens[..*instead])
    };
    let Some((PlayerAst::That, count, random)) = discard(action) else { return Ok(None); };
    // These state predicates have no mana or punctuation atoms. Keep the
    // original tokens authoritative before the shared predicate vocabulary.
    if condition.iter().any(|token| !matches!(token.kind, TokenKind::Word | TokenKind::Number)) {
        return Err(CardTextError::ParseError("invalid token in discard replacement condition".into()));
    }
    let predicate = crate::grammar::filters::parse_condition_predicate_lexed(condition)?;
    Ok(Some((predicate, count, random)))
}

pub(super) fn recognizes_replacement_sentence(tokens: &[OwnedLexToken]) -> bool {
    matches!(replacement(tokens), Ok(Some(_)))
}

pub(super) fn validate(tokens: &[OwnedLexToken]) -> Result<(), CardTextError> {
    for tokens in crate::lexer::split_lexed_sentences(tokens) {
        let words = crate::lexer::parser_token_word_refs(tokens);
        let state_head = [
            &["if", "this", "spell", "was", "kicked"][..],
            &["if", "you", "cast", "this", "spell"],
            &["card", "types", "among", "cards", "in", "your", "graveyard"],
        ].iter().any(|head| words.windows(head.len()).any(|part| part == *head));
        if state_head && words.contains(&"instead") && words.windows(3).any(|part| part == ["that", "player", "discards"]) {
            replacement(tokens)?.ok_or_else(|| CardTextError::ParseError(
                "incomplete conditional discard replacement".into()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_retains_one_player_and_exact_randomness() {
        for (first, second, player, random) in [
            ("Target player discards two cards.", "If this spell was kicked, that player discards three cards instead.", PlayerAst::Target, false),
            ("Target opponent discards a card at random.", "Delirium — If there are four or more card types among cards in your graveyard, that player discards two cards at random instead.", PlayerAst::TargetOpponent, true),
        ] {
            let text = format!("{first} {second}");
            let effects = crate::effect_sentences::parse_effect_sentences_lexed(&crate::lexer::lex_line(&text, 0).unwrap()).unwrap();
            let [EffectAst::SelfReplacement { if_true, if_false, .. }] = effects.as_slice() else { panic!("{effects:?}"); };
            for effects in [if_true, if_false] {
                let [EffectAst::SubjectVerb(action)] = effects.as_slice() else { panic!("{effects:?}"); };
                assert_eq!(action.subject.player, player);
                assert!(matches!(&action.action, SubjectVerbActionAst::ZoneMoves(crate::cards::builders::ZoneMoveActionAst::Discard { random: actual, .. }) if *actual == random));
            }
        }
    }
}

pub(super) fn read(sentences: &[SentenceInput], index: usize) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else { return Ok(None); };
    let Some((player @ (PlayerAst::Target | PlayerAst::TargetOpponent), count, random)) = discard(first.lexed())
        else { return Ok(None); };
    let Some((predicate, replacement_count, replacement_random)) = replacement(second.lexed())?
        else { return Ok(None); };
    let action = |count, random| EffectAst::subject_verb_discard(player, Value::Fixed(count), random, false, None, None);
    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate,
        if_true: vec![action(replacement_count, replacement_random)],
        if_false: vec![action(count, random)],
        attach_to_previous_ability: false,
    }]))
}
