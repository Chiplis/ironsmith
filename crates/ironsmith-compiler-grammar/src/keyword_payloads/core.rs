use crate::cards::builders::ConditionalEffectAst;
use super::*;

pub(in super::super) fn parse_escalate(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let Some((cost, display)) = parse_escalate_line_lexed(tokens)? else {
        return Ok(None);
    };
    Ok(ast(LineAst::StaticAbility(
        crate::model::CompilerStaticAbilityCore::escalate_with_cost_surface(cost, Some(display))
            .into(),
    )))
}

pub(in super::super) fn parse_eternalize(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    Ok(parse_eternalize_line_lexed(tokens)?.map(|cost| {
        KeywordLinePayload::ast(LineAst::Abilities(vec![
            crate::cards::builders::KeywordAction::Eternalize(cost),
        ]))
    }))
}

pub(in super::super) fn parse_evoke(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let Some(method) = parse_evoke_line_lexed(tokens)? else {
        return Ok(None);
    };
    Ok(ast(LineAst::Multiple(vec![
        LineAst::AlternativeCastingMethod(method),
        LineAst::Triggered {
            trigger: TriggerSpec::ThisEntersBattlefield {
                origin_condition: None,
            },
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ThisSpellPaidLabel("Evoke".into()),
                if_true: vec![EffectAst::subject_verb_sacrifice(
                    PlayerAst::ItsController,
                    crate::target::ObjectFilter::source(),
                    1,
                    Some(TargetAst::Source(None)),
                )],
                if_false: Vec::new(),
            })],
            max_triggers_per_turn: None,
        },
    ])))
}

pub(in super::super) fn parse_epic(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !parse_epic_line_lexed(tokens) {
        return Ok(None);
    }
    Ok(ast(LineAst::StaticAbility(
        crate::model::CompilerStaticAbilityCore::keyword_marker("Epic").into(),
    )))
}

pub(in super::super) fn parse_exploit(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if parse_keyword_prefix_shape_tokens(tokens) != Some(KeywordPrefixShape::Exploit) {
        return Ok(None);
    }
    Ok(ast(LineAst::Triggered {
        trigger: TriggerSpec::ThisEntersBattlefield {
            origin_condition: None,
        },
        effects: vec![EffectAst::subject_verb_exploit()],
        max_triggers_per_turn: None,
    }))
}
