//! A conditional base-size alternative modifies the previous full animation.
//! It must not replace the type/color/ability part with a standalone P/T action.
use super::*;
use crate::cards::builders::CharacteristicActionAst;

pub(super) fn pre_rule_animation_base_pt_replacement(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if !tokens.first().is_some_and(|token| token.is_word("if"))
        || !tokens.last().is_some_and(|token| token.is_word("instead"))
    {
        return Ok(None);
    }
    let Some(comma) = tokens.iter().position(OwnedLexToken::is_comma) else {
        return Ok(None);
    };
    let body = crate::util::trim_edge_punctuation_tokens(&tokens[comma + 1..tokens.len() - 1]);
    let words = crate::lexer::parser_token_word_refs(body);
    if !words.starts_with(&["it", "has", "base", "power", "and", "toughness"]) {
        return Ok(None);
    }
    let Some(previous) = state.effects.last() else {
        return Ok(None);
    };
    let mut replacement = previous.clone();
    let EffectAst::SubjectVerb(subject) = &mut replacement else {
        return Ok(None);
    };
    let SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
        base_power_toughness: Some(pair),
        duration,
        ..
    }) = &mut subject.action
    else {
        return Ok(None);
    };
    let Some(EffectAst::SubjectVerb(size)) =
        crate::effect_sentences::for_each_helpers::parse_has_base_power_toughness_clause(body)?
    else {
        return Ok(None);
    };
    let (power, toughness, replacement_duration) = match size.action {
        SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
            power,
            toughness,
            duration,
            ..
        }) => (power, toughness, duration),
        SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
            base_power_toughness: Some((power, toughness)),
            duration,
            ..
        }) => (power, toughness, duration),
        _ => return Ok(None),
    };
    // A size with a different expiration needs a separate timed modification,
    // not a rewrite of the whole animation's lifetime.
    if *duration != replacement_duration {
        return Err(CardTextError::ParseError(
            "animation base-size replacement has a different duration".into(),
        ));
    }
    *pair = (power, toughness);
    let predicate = crate::grammar::filters::parse_condition_predicate_lexed(&tokens[1..comma])?;
    let previous = state.effects.pop().expect("previous complete animation");
    state.effects.push(EffectAst::SelfReplacement {
        predicate,
        if_true: vec![replacement],
        if_false: vec![previous],
        attach_to_previous_ability: false,
    });
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some("animation-base-pt-self-replacement"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(text: &str) -> Result<Vec<EffectAst>, CardTextError> {
        crate::effect_sentences::parse_effect_sentences_lexed(
            &crate::lexer::lex_line(text, 0).unwrap(),
        )
    }
    #[test]
    fn size_override_clones_the_animation_and_changes_only_its_size() {
        let effects = parse("Until end of turn, target artifact or creature becomes an artifact creature with base power and toughness 4/3. If evidence was collected, it has base power and toughness 1/1 until end of turn instead.").unwrap();
        let [
            EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("{effects:?}");
        };
        assert!(format!("{predicate:?}").contains("Evidence"));
        let mut expected = if_false.clone();
        let EffectAst::SubjectVerb(subject) = &mut expected[0] else {
            panic!("animation");
        };
        let SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
            base_power_toughness,
            ..
        }) = &mut subject.action
        else {
            panic!("animation");
        };
        *base_power_toughness = Some((Value::Fixed(1), Value::Fixed(1)));
        assert_eq!(&expected, if_true);
    }
    #[test]
    fn neither_an_orphan_instead_nor_a_different_duration_is_silently_accepted() {
        assert!(parse("If evidence was collected, it has base power and toughness 1/1 until end of turn instead.").is_err());
        assert!(parse("Until end of turn, target artifact becomes an artifact creature with base power and toughness 4/3. If evidence was collected, it has base power and toughness 1/1 until your next turn instead.").is_err());
    }
    #[test]
    fn a_separate_non_instead_stat_change_is_not_rewritten_as_self_replacement() {
        let effects = parse("Until end of turn, target artifact becomes an artifact creature with base power and toughness 4/3. If evidence was collected, it has base power and toughness 1/1 until end of turn.").unwrap();
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, EffectAst::SelfReplacement { .. }))
        );
    }
}
