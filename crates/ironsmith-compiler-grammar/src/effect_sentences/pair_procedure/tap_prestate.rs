//! A preexisting state is observed before the action it qualifies.
use super::*;
use crate::cards::builders::ConditionalEffectAst;
use crate::lexer::TokenKind;

pub(super) fn read(sentences: &[SentenceInput], index: usize)
    -> Result<Option<Vec<EffectAst>>, CardTextError>
{
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1))
        else { return Ok(None); };
    let first = crate::util::trim_edge_punctuation_tokens(first.lowered());
    if !first.first().is_some_and(|token| token.is_word("tap")) { return Ok(None); }
    let second = crate::util::trim_edge_punctuation_tokens(second.lowered());
    if !second.first().is_some_and(|token| token.is_word("if")) { return Ok(None); }
    let Some(comma) = second.iter().position(|token| token.kind == TokenKind::Comma)
        else { return Ok(None); };
    let header = &second[1..comma];
    if header.len() < 4 || !header[header.len()-3..].iter()
        .zip(["was", "already", "tapped"]).all(|(token, word)| token.is_word(word))
    { return Ok(None); }
    // Read the same subject with an ordinary present-state predicate. This
    // conditional owns evaluation before the tap; the original tap executes
    // once in either branch, including when prevented or already satisfied.
    let mut state = header[..header.len()-3].to_vec();
    state.extend(crate::lexer::synthetic_word_tokens(["is", "tapped"]));
    let predicate = crate::grammar::filters::parse_condition_predicate_lexed(&state)?;
    let action = crate::effect_sentences::parse_effect_sentences_lexed(first)?;
    let followup = crate::effect_sentences::parse_effect_sentences_lexed(&second[comma+1..])?;
    if action.is_empty() || followup.is_empty() { return Ok(None); }
    crate::word_claims::claim_tokens(&second[..=comma]);
    let mut if_true = action.clone();
    if_true.extend(followup);
    Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate, if_true, if_false: action,
    })]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn already_tapped_gate_precedes_tap_and_keeps_action_in_both_branches() {
        let tokens = crate::lexer::lex_line("Tap this creature. If this creature was already tapped, you gain 1 life.", 0).unwrap();
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
        use crate::cards::builders::{LifeResourceActionAst, PermanentStateActionAst, SourcePredicateAst};
        let [EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate, if_true, if_false,
        })] = effects.as_slice() else { panic!("one prestate conditional: {effects:?}"); };
        assert!(matches!(predicate, PredicateAst::Source(SourcePredicateAst::SourceIsTapped)));
        let [tap, gain] = if_true.as_slice() else { panic!("tap then gain: {if_true:?}"); };
        let [other_tap] = if_false.as_slice() else { panic!("exactly one false-branch tap: {if_false:?}"); };
        assert_eq!(tap, other_tap, "both branches execute the same original tap");
        assert!(matches!(tap, EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap {
                target: TargetAst::Source(_),
            }), ..
        })));
        assert!(matches!(gain, EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                amount: Value::Fixed(1),
            }), ..
        })));
    }
}
