use super::*;

pub(super) fn reject_unsupported_lost_abilities(
    losing: bool,
    abilities: &[GrantedAbilityAst],
) -> Result<(), CardTextError> {
    if !losing {
        return Ok(());
    }
    if abilities.iter().any(|ability| {
        matches!(ability, GrantedAbilityAst::KeywordAction(action) if matches!(action.as_ref(), KeywordAction::Soulbond))
    }) {
        return Err(CardTextError::ParseError(
            "removing soulbond requires non-marker semantics".to_string(),
        ));
    }
    Ok(())
}
