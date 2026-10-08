use super::*;

pub(super) fn parse_regenerate_then_gain_control_if_regenerates_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_regenerate_control_shape(first, second)?;
    let regenerate_target =
        crate::grammar::primitives::probe_shape(parse_target_phrase(shape.regenerate_target))?;
    let control_target =
        crate::grammar::primitives::probe_shape(parse_target_phrase(shape.control_target))?;
    let follow_up = EffectAst::subject_verb_gain_control(
        PlayerAst::Implicit,
        control_target,
        crate::effect::Until::Forever,
    );

    Some(vec![
        EffectAst::subject_verb_regenerate_with_follow_up_effects(
            regenerate_target,
            vec![follow_up],
        ),
    ])
}

/// "Regenerate this creature. When it regenerates this way, put a -1/-1
/// counter on it." (Matopi Golem, Skeleton Scavengers): the second sentence
/// is a reflexive trigger of the regeneration shield (CR 701.19, 603.12),
/// carried as the shield's `WhenResult` follow-up.
pub(super) fn parse_regenerate_then_when_regenerates_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(first);
    let first_words = crate::lexer::token_word_refs(&first);
    if first_words.first() != Some(&"regenerate") || first_words.len() < 2 {
        return Ok(None);
    }
    let second = trim_commas(second);
    let second_words = crate::lexer::token_word_refs(&second);
    let head_len = match second_words.as_slice() {
        ["when", "it", "regenerates", "this", "way", ..] => 5,
        ["when", "that", "creature", "regenerates", "this", "way", ..] => 6,
        _ => return Ok(None),
    };
    let view = crate::lexer::TokenWordView::new(&second);
    let Some(body_start) = view.map_word_to_token_start(head_len) else {
        return Ok(None);
    };
    let body = trim_commas(&second[body_start..]);
    if body.is_empty() {
        return Ok(None);
    }
    let first_view = crate::lexer::TokenWordView::new(&first);
    let Some(target_range) = first_view.token_span_for_words(1, first_words.len()) else {
        return Ok(None);
    };
    let target = parse_target_phrase(&first[target_range])?;
    let effects = effect_sentences::parse_effect_sentence_lexed(&body)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![
        EffectAst::subject_verb_regenerate_with_follow_up_effects(
            target,
            vec![EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                predicate: IfResultPredicate::Did,
                effects,
            })],
        ),
    ]))
}
