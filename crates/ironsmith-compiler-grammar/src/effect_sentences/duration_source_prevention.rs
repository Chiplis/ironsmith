//! Complete targeted-source prevention clauses. One target binds both directions.
use crate::cards::builders::{CardTextError, EffectAst, TargetAst};
use crate::effect::Until;
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView};
use crate::target::{ObjectFilter, PlayerFilter};

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if tokens.iter().any(|token| {
        matches!(
            token.kind,
            TokenKind::Quote | TokenKind::Colon | TokenKind::Period
        )
    }) {
        return Ok(None);
    }
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    let (start, end, duration) = if let Some(shape) =
        crate::grammar::effects::gain_ability_shapes::parse_gain_ability_duration_shape(&words)
    {
        if shape.condition.is_some() {
            return Ok(None);
        }
        if shape.start == 0 {
            (shape.len, words.len(), shape.duration)
        } else if shape.start + shape.len == words.len() {
            (0, shape.start, shape.duration)
        } else {
            return Ok(None);
        }
    } else if words.ends_with(&["this", "turn"]) {
        (0, words.len() - 2, Until::EndOfTurn)
    } else {
        // The chain owner may already have consumed an authored duration. Its
        // carry only replaces Forever, so never invent EndOfTurn here.
        (0, words.len(), Until::Forever)
    };
    let body = &words[start..end];
    let (source_start, source_end, bidirectional) = if body.starts_with(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to", "and", "dealt", "by",
    ]) {
        (start + 11, end, true)
    } else if body.starts_with(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "by",
    ]) {
        (start + 8, end, false)
    } else if body.starts_with(&["prevent", "all", "damage"]) && body.ends_with(&["would", "deal"])
    {
        (start + 3, end - 2, false)
    } else {
        return Ok(None);
    };
    if source_start >= source_end {
        return Ok(None);
    }
    let source_words = &words[source_start..source_end];
    // Source sets and a source of your choice have different selection owners.
    // This reader claims only the complete explicitly targeted source form.
    if source_words
        .iter()
        .filter(|word| **word == "target")
        .count()
        != 1
        || source_words
            .iter()
            .any(|word| matches!(*word, "would" | "prevent" | "until" | "unless" | "instead"))
    {
        return Ok(None);
    }
    let Some(range) = view.map_word_span_to_token_range(source_start, source_end) else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(&tokens[range])?;
    Ok(Some(if bidirectional {
        EffectAst::subject_verb_prevent_all_damage_to_and_by_target(target, duration)
    } else {
        EffectAst::subject_verb_prevent_all_damage_to_target_from_target_source(
            TargetAst::ObjectOrPlayer(ObjectFilter::default(), PlayerFilter::Any, None),
            target,
            duration,
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{DamagePreventionActionAst, SubjectVerbActionAst};
    #[test]
    fn one_declared_target_retains_both_directions_and_the_complete_duration() {
        for (text, expected, both) in [
            (
                "Until your next turn, prevent all damage that would be dealt to and dealt by target permanent an opponent controls.",
                Until::YourNextTurn,
                true,
            ),
            (
                "Until your next turn, prevent all damage target permanent would deal.",
                Until::YourNextTurn,
                false,
            ),
            (
                "Prevent all damage target instant or sorcery spell would deal this turn.",
                Until::EndOfTurn,
                false,
            ),
            (
                "Prevent all damage that would be dealt by up to one target creature for as long as this Saga remains on the battlefield.",
                Until::while_source_remains_on_battlefield(),
                false,
            ),
        ] {
            let ast = parse(&crate::lexer::lex_line(text, 0).unwrap())
                .unwrap()
                .unwrap();
            let EffectAst::SubjectVerb(subject) = ast else {
                panic!("typed subject");
            };
            let SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTarget {
                    source_target: Some(_),
                    duration,
                    protect_source_target,
                    ..
                },
            ) = subject.action
            else {
                panic!("typed source shield");
            };
            assert_eq!(duration, expected, "{text}");
            assert_eq!(protect_source_target, both);
        }
    }
    #[test]
    fn unknown_tail_extra_target_or_secondary_instruction_is_never_discarded() {
        for text in [
            "Until your next turn, prevent all damage target creature would deal unless its controller pays 1 life.",
            "Prevent all damage target creature would deal this turn. Draw a card.",
            "Prevent all damage that would be dealt to target creature and dealt by target creature this turn.",
            "Until your next turn, prevent all damage target permanent would deal this turn.",
            "Prevent all damage a red source of your choice would deal this turn.",
        ] {
            assert!(
                !matches!(
                    parse(&crate::lexer::lex_line(text, 0).unwrap()),
                    Ok(Some(_))
                ),
                "{text}"
            );
        }
    }
}

#[cfg(test)]
mod conditional_emblem_tests {
    #[test]
    fn conditional_game_outcome_prohibitions_keep_both_player_scopes() {
        let tokens=crate::lexer::lex_line("As long as you control a Gideon planeswalker, you can't lose the game and your opponents can't win the game.",0).unwrap();
        let abilities = crate::clause_support::parse_static_ability_ast_line_lexed(&tokens)
            .unwrap()
            .unwrap();
        assert_eq!(abilities.len(), 2);
        let debug = format!("{abilities:?}");
        assert!(debug.contains("Gideon"), "{debug}");
        assert!(debug.contains("Planeswalker"), "{debug}");
        assert!(debug.contains("YouCantLoseGame"), "{debug}");
        assert!(debug.contains("OpponentsCantWinGame"), "{debug}");
        assert!(debug.matches("Conditional").count() >= 2, "{debug}");
    }
}
