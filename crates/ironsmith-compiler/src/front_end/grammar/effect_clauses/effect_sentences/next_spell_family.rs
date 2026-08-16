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
    if crate::lexer::token_word_refs(tokens) == ["affinity", "for", "artifacts"] {
        return Some(GrantedAbilityAst::StaticAbility(
            StaticAbility::affinity_for_artifacts(),
        ));
    }
    let action = match parse_next_spell_keyword_action_tokens(tokens)? {
        NextSpellKeywordActionShape::Known(action) => action,
        NextSpellKeywordActionShape::SingleWord(word) => parse_single_word_keyword_action(word)?,
    };
    action.lowers_to_static_ability().then(|| action.into())
}

pub(crate) fn parse_next_spell_grant_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(shape) = parse_next_spell_grant_tokens(tokens)?
        && let Some(effects) = lower_next_spell_grant(shape)
    {
        return Ok(Some(effects));
    }

    // A next-spell grant can be coordinated with an independent optional
    // action, as in “The next spell ... has cascade and you may planeswalk.”
    // Parse the one-shot grant on the left before the broad static-grant
    // fallback can discard both `next` and `this turn`, then route the right
    // clause through the ordinary effect grammar.
    let Some(split) = tokens.windows(3).position(|window| {
        window[0].is_word("and") && window[1].is_word("you") && window[2].is_word("may")
    }) else {
        return Ok(None);
    };
    let Some(shape) = parse_next_spell_grant_tokens(&tokens[..split])? else {
        return Ok(None);
    };
    let Some(mut effects) = lower_next_spell_grant(shape) else {
        return Ok(None);
    };
    effects.extend(super::parse_effect_sentence_lexed(&tokens[split + 1..])?);
    Ok(Some(effects))
}

fn lower_next_spell_grant(
    shape: super::super::grammar::effects::NextSpellGrantShape<'_>,
) -> Option<Vec<EffectAst>> {
    let ability = parse_next_spell_grant_ability(shape.ability)?;
    Some(
        shape
            .filters
            .into_iter()
            .map(|filter| {
                EffectAst::subject_verb_grant_next_spell_ability_this_turn(
                    shape.player,
                    filter,
                    ability.clone(),
                )
            })
            .collect(),
    )
}
