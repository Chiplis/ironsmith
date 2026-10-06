//! Named frontend owner for the atomic authored-word substitution instruction.
use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::{LexedClause, OwnedLexToken};
use crate::util::{parse_subtype_flexible, parse_target_phrase};
use ironsmith_core::{TextChangeSelection, Until};

pub(super) fn parse_text_change(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if !words.starts_with(&["change", "the", "text", "of"]) { return Ok(None); }
    let marker = ["by", "replacing", "all", "instances", "of", "one"];
    let Some(boundary) = words.windows(marker.len()).position(|words| words == marker) else {
        return Err(CardTextError::ParseError("text change is missing a complete word-replacement clause".into()));
    };
    let start = clause.token_index_after_words(4).ok_or_else(|| CardTextError::ParseError("missing text-change target".into()))?;
    let end = clause.token_index_after_words(boundary).ok_or_else(|| CardTextError::ParseError("missing replacement boundary".into()))?;
    let target_words = &words[4..boundary];
    let target = if target_words.ends_with(&["that", "doesnt", "have", "cumulative", "upkeep"]) {
        let qualifier = clause.token_index_after_words(boundary - 5)
            .ok_or_else(|| CardTextError::ParseError("missing cumulative-upkeep target predicate".into()))?;
        let mut target = parse_target_phrase(&tokens[start..qualifier])?;
        let crate::cards::builders::TargetAst::Object(filter, _, _) = &mut target else {
            return Err(CardTextError::ParseError("cumulative-upkeep predicate requires an object target".into()));
        };
        filter.has_cumulative_upkeep = Some(false);
        target
    } else { parse_target_phrase(&tokens[start..end])? };
    let choice_start = clause.token_index_after_words(boundary + marker.len())
        .ok_or_else(|| CardTextError::ParseError("missing word-choice body".into()))?;
    let complete_end = if tokens.last().is_some_and(|token| token.kind == crate::lexer::TokenKind::Period) {
        tokens.len() - 1
    } else { tokens.len() };
    if tokens[choice_start..complete_end].iter().any(|token| token.as_word().is_none()) {
        return Err(CardTextError::ParseError("text-change instruction has an unsupported trailing token".into()));
    }
    let tail = &words[boundary + marker.len()..];
    let (tail, duration) = if let Some(tail) = tail.strip_suffix(&["until", "end", "of", "turn"]) {
        (tail, Until::EndOfTurn)
    } else { (tail, Until::Forever) };
    let selection = match tail {
        ["color", "word", "with", "another"] => TextChangeSelection::Color,
        ["basic", "land", "type", "with", "another"] => TextChangeSelection::BasicLand,
        ["color", "word", "with", "another", "or", "one", "basic", "land", "type", "with", "another"] =>
            TextChangeSelection::ColorOrBasicLand,
        ["creature", "type", "with", "another"] => TextChangeSelection::Creature { excluded_new: Vec::new() },
        ["creature", "type", "with", destination] => TextChangeSelection::CreatureTo(
            parse_subtype_flexible(destination).filter(|subtype| subtype.is_creature_type())
                .ok_or_else(|| CardTextError::ParseError("fixed text replacement requires a creature type".into()))?),
        _ => return Err(CardTextError::ParseError("unsupported or incomplete text-change word choice".into())),
    };
    Ok(Some(EffectAst::subject_verb_change_text(target, selection, duration)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{CharacteristicActionAst, SubjectVerbActionAst, SubjectVerbEffectAst};

    #[test]
    fn word_families_and_complete_duration_lower_to_an_atomic_typed_instruction() {
        for (text, expected, until) in [
            ("Change the text of target spell or permanent by replacing all instances of one color word with another.", TextChangeSelection::Color, Until::Forever),
            ("Change the text of target permanent by replacing all instances of one basic land type with another.", TextChangeSelection::BasicLand, Until::Forever),
            ("Change the text of target permanent by replacing all instances of one color word with another or one basic land type with another until end of turn.", TextChangeSelection::ColorOrBasicLand, Until::EndOfTurn),
            ("Change the text of that creature by replacing all instances of one creature type with Vampire.", TextChangeSelection::CreatureTo(ironsmith_core::Subtype::Vampire), Until::Forever),
        ] {
            let parsed = parse_text_change(&crate::lexer::lex_line(text, 0).unwrap()).unwrap().unwrap();
            let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::ChangeText { selection, duration, .. }), .. }) = parsed else { panic!("typed instruction"); };
            assert_eq!(selection, expected);
            assert_eq!(duration, until);
        }
    }

    #[test]
    fn unrelated_change_target_and_partial_replacement_text_do_not_become_word_changes() {
        assert!(parse_text_change(&crate::lexer::lex_line("Change the target of target spell to this creature.", 0).unwrap()).unwrap().is_none());
        for text in [
            "Change the text of target permanent by replacing all instances of one color word with another until your next turn.",
            "Change the text of target permanent by replacing all instances of one color word with another except on Tuesdays.",
            "Change the text of target permanent by replacing all instances of one creature type with Equipment.",
            "Change the text of target permanent by replacing all instances of one color word with another!",
        ] {
            assert!(parse_text_change(&crate::lexer::lex_line(text, 0).unwrap()).is_err(), "{text}");
        }
    }
}
