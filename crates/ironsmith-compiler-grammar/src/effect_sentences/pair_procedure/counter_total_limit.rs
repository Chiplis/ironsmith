//! A placement and its ability-local resulting-total limit are one instruction.
use super::*;
use crate::cards::builders::CounterActionAst;
use crate::lexer::{OwnedLexToken, TokenKind};

fn source(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::parser_token_word_refs(tokens);
    tokens.len() == words.len()
        && matches!(words.as_slice(), ["this"] | ["this", "creature"] | ["this", "permanent"] | ["this", "artifact"])
}

fn limit(tokens: &[OwnedLexToken]) -> Option<(crate::object::CounterType, u32)> {
    let tokens = if tokens.last().is_some_and(|token| token.kind == TokenKind::Period) {
        &tokens[..tokens.len() - 1]
    } else { tokens };
    let prefix = ["this", "ability", "can't", "cause", "the", "total", "number", "of"];
    if tokens.len() <= prefix.len() || !tokens.iter().zip(prefix).all(|(token, word)| {
        token.is_word(word) || (word == "can't" && (token.is_word("cant") || token.is_word("cannot")))
    }) { return None; }
    let noun = tokens.iter().enumerate().skip(prefix.len())
        .find(|(_, token)| token.is_word("counters"))?.0;
    let counter = crate::grammar::filters::parse_counter_type_from_tokens(&tokens[prefix.len()..=noun])?;
    let tail = tokens.get(noun + 1..)?;
    if !tail.first()?.is_word("on") { return None; }
    let to = tail.iter().position(|token| token.is_word("to"))?;
    if !source(&tail[1..to]) { return None; }
    let rest = tail.get(to..)?;
    if rest.len() < 5 || !rest.iter().zip(["to", "be", "greater", "than"]).all(|(token, word)| token.is_word(word)) {
        return None;
    }
    let (maximum, consumed) = crate::util::parse_number(&rest[4..])?;
    (consumed == rest.len() - 4).then_some((counter, maximum))
}

pub(super) fn read(sentences: &[SentenceInput], index: usize) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else { return Ok(None); };
    let Some((limited_counter, maximum)) = limit(second.lowered()) else { return Ok(None); };
    let mut effects = crate::effect_sentences::parse_effect_sentence_lexed(first.lowered())?;
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
            counter_type, maximum_total, target, target_count: None, distributed: false, ..
        }), ..
    })] = effects.as_mut_slice() else { return Ok(None); };
    let source_target = matches!(target, TargetAst::Source(_))
        || matches!(target, TargetAst::Object(filter, None, _) if filter.source);
    if !source_target || *counter_type != limited_counter { return Ok(None); }
    *maximum_total = Some(maximum);
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_limit_binds_only_matching_source_counter_placement() {
        for (first, second, expected) in [
            ("Put up to X +1/+0 counters on this creature", "This ability can't cause the total number of +1/+0 counters on this creature to be greater than four", Some(4)),
            ("Put up to X +1/+0 counters on this creature", "This ability can't cause the total number of +1/+0 counters on this creature to be greater than seven", Some(7)),
            ("Put up to X +1/+1 counters on this creature", "This ability can't cause the total number of +1/+0 counters on this creature to be greater than four", None),
            ("Put up to X +1/+0 counters on target creature", "This ability can't cause the total number of +1/+0 counters on this creature to be greater than four", None),
            ("Put up to X +1/+0 counters on this creature", "This ability can't cause the total number of +1/+0 counters on this creature to be greater than four {G}", None),
            ("Put up to X +1/+0 counters on this creature", "This ability can't cause the total number of +1/+0 counters on that creature to be greater than four", None),
        ] {
            let first = crate::lexer::lex_line(first, 0).unwrap();
            let second = crate::lexer::lex_line(second, 0).unwrap();
            let result = read(&[SentenceInput::from_lexed(&first), SentenceInput::from_lexed(&second)], 0).unwrap();
            let actual = result.and_then(|effects| match effects.into_iter().next().unwrap() {
                EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { maximum_total, .. }), .. }) => maximum_total,
                _ => panic!("typed counter placement"),
            });
            assert_eq!(actual, expected);
        }
    }
}
