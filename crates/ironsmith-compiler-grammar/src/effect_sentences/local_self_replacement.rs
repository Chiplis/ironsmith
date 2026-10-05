//! Complete trailing local action replacements, before broad verb fallbacks.
use crate::cards::builders::{CardTextError, ConditionalEffectAst, EffectAst, ForEachEffectAst,
    PlayerAst, PlayerPredicateAst, PredicateAst, TargetAst};
use crate::lexer::{OwnedLexToken, TokenKind};

pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(tokens)
        .map_or(tokens, |label| label.body_tokens);
    let tokens = if tokens.last().is_some_and(|token| token.kind == TokenKind::Period) {
        &tokens[..tokens.len() - 1]
    } else { tokens };
    if tokens.iter().any(|token| token.kind == TokenKind::Period) { return Ok(None); }
    let Some(index) = tokens.iter().position(|token| token.is_word("instead")) else { return Ok(None); };
    let action = &tokens[..index];
    let words = crate::lexer::parser_token_word_refs(action);
    let counter = words.as_slice() == ["counter", "that", "spell"];
    let sacrifice = words.starts_with(&["each", "player", "sacrifices", "all"])
        && words.ends_with(&["they", "control"]);
    if !counter && !sacrifice { return Ok(None); }
    // Both adopted action heads have word/number predicates. An embedded
    // symbol or a second replacement marker must not disappear in a fallback.
    if tokens.iter().any(|token| !matches!(token.kind, TokenKind::Word | TokenKind::Number))
        || tokens.iter().filter(|token| token.is_word("instead")).count() != 1
        || !tokens.get(index + 1).is_some_and(|token| token.is_word("if"))
    {
        return Err(CardTextError::ParseError("incomplete trailing local replacement".into()));
    }
    let mut predicate = crate::grammar::filters::parse_condition_predicate_lexed(&tokens[index + 2..])?;
    if counter && let PredicateAst::Player(PlayerPredicateAst::PlayerHasPoisonCountersOrMore {
        player: PlayerAst::ItsController, count,
    }) = &predicate {
        // Only this complete replacement repeats the original counter target.
        // Its gate runs before the original counter action can export a tag.
        // A shared `its controller` predicate keeps its ordinary antecedent.
        predicate = PredicateAst::ValueComparison {
            left: crate::effect::Value::PlayerCounters(
                crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Target),
                crate::object::CounterType::Poison),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(i32::try_from(*count).map_err(|_| CardTextError::ParseError("poison threshold is out of range".into()))?),
        };
    }
    let effects = if counter {
        vec![EffectAst::subject_verb_counter(TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None))]
    } else {
        let filter_tokens = &action[4..action.len() - 2];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError("missing all-sacrifice object domain".into()));
        }
        // This exact quantified domain has no relational suffix: the printed
        // controller relation belongs to the iterated sacrificing player.
        let mut filter = crate::object_filters::parse_object_filter_lexed(filter_tokens, false)?;
        if filter.controller.is_some() || filter.owner.is_some() {
            return Err(CardTextError::ParseError("conflicting all-sacrifice actor".into()));
        }
        filter.controller = Some(crate::target::PlayerFilter::IteratedPlayer);
        vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_sacrifice_all(PlayerAst::That, filter)],
        })]
    };
    Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects })]))
}

pub(super) fn validate(tokens: &[OwnedLexToken]) -> Result<(), CardTextError> {
    for sentence in crate::lexer::split_lexed_sentences(tokens) { read(sentence)?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trailing_replacement_keeps_typed_condition_and_whole_program() {
        for text in [
            "Threshold — Each player sacrifices all lands they control instead if there are seven or more cards in your graveyard.",
            "Corrupted — Counter that spell instead if its controller has three or more poison counters.",
        ] {
            let effects = read(&crate::lexer::lex_line(text, 0).unwrap()).unwrap().unwrap();
            assert!(matches!(effects.as_slice(), [EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { .. })]));
        }
    }

    #[test]
    fn only_the_complete_counter_replacement_binds_controller_poison_to_target() {
        let effects = read(&crate::lexer::lex_line(
            "Counter that spell instead if its controller has three or more poison counters.", 0).unwrap()).unwrap().unwrap();
        let [EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: PredicateAst::ValueComparison { left: crate::effect::Value::PlayerCounters(player, _), .. }, ..
        })] = effects.as_slice() else { panic!("{effects:?}"); };
        assert_eq!(*player, crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Target));
    }
}
