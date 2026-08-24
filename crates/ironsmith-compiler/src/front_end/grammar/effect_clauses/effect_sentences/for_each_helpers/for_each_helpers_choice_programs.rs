use super::*;

pub(super) fn parse_participant_choice_complement_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut full_clause = Vec::with_capacity(tokens.len() + 2);
    full_clause.push(OwnedLexToken::word(
        "each".to_string(),
        TextSpan::synthetic(),
    ));
    full_clause.push(OwnedLexToken::word(
        "player".to_string(),
        TextSpan::synthetic(),
    ));
    full_clause.extend_from_slice(tokens);

    let Some(effect) = super::super::parse_choice_complement_subject_verb(&full_clause)? else {
        return Ok(None);
    };
    let EffectAst::ForEachPlayer { effects } = effect else {
        return Ok(None);
    };
    Ok(Some(effects))
}
