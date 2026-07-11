use super::super::activation_and_restrictions::parse_single_word_keyword_action;
use super::super::grammar::effects::{
    NextSpellGrantAbilitySurface, NextSpellKeywordActionShape, parse_next_spell_grant_tokens,
    parse_next_spell_keyword_action_tokens,
};
use super::super::lexer::OwnedLexToken;
use crate::cards::builders::{CardTextError, EffectAst, GrantedAbilityAst};
use crate::static_abilities::StaticAbility;

fn parse_next_spell_grant_ability(
    surface: NextSpellGrantAbilitySurface<'_>,
) -> Option<GrantedAbilityAst> {
    if surface == NextSpellGrantAbilitySurface::CantBeCountered {
        return Some(GrantedAbilityAst::StaticAbility(
            StaticAbility::cant_be_countered_ability(),
        ));
    }
    let NextSpellGrantAbilitySurface::Keyword(tokens) = surface else {
        return None;
    };
    let action = match parse_next_spell_keyword_action_tokens(tokens)? {
        NextSpellKeywordActionShape::Known(action) => action,
        NextSpellKeywordActionShape::SingleWord(word) => parse_single_word_keyword_action(word)?,
    };
    action.lowers_to_static_ability().then(|| action.into())
}

pub(crate) fn parse_next_spell_grant_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = parse_next_spell_grant_tokens(tokens)? else {
        return Ok(None);
    };
    let Some(ability) = parse_next_spell_grant_ability(shape.ability) else {
        return Ok(None);
    };
    let effects = shape
        .filters
        .into_iter()
        .map(|filter| {
            EffectAst::subject_verb_grant_next_spell_ability_this_turn(
                shape.player.clone(),
                filter,
                ability.clone(),
            )
        })
        .collect();
    Ok(Some(effects))
}
