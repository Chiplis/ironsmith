use super::super::activation_and_restrictions::parse_single_word_keyword_action;
use super::super::grammar::effects::{
    NextSpellGrantAbilitySurface, NextSpellKeywordActionShape, parse_next_spell_grant_tokens,
    parse_next_spell_keyword_action_tokens,
};
use super::super::lexer::OwnedLexToken;
use crate::cards::builders::{CardTextError, EffectAst, GrantedAbilityAst};

fn parse_next_spell_grant_ability(
    surface: NextSpellGrantAbilitySurface<'_>,
) -> Option<GrantedAbilityAst> {
    if surface == NextSpellGrantAbilitySurface::CantBeCountered {
        return Some(GrantedAbilityAst::KeywordAction(Box::new(
            crate::payload::KeywordAction::CantBeCountered,
        )));
    }
    let NextSpellGrantAbilitySurface::Keyword(tokens) = surface else {
        return None;
    };
    if crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(tokens),
        &["affinity", "for", "artifacts"],
    ) {
        return Some(GrantedAbilityAst::KeywordAction(Box::new(
            crate::payload::KeywordAction::AffinityForArtifacts,
        )));
    }
    let action = match parse_next_spell_keyword_action_tokens(tokens)? {
        NextSpellKeywordActionShape::Known(action) => action,
        NextSpellKeywordActionShape::SingleWord(word) => parse_single_word_keyword_action(word)?,
    };
    action.lowers_to_static_ability().then(|| action.into())
}

/// "Rather than pay the mana cost of the next spell you cast this turn, you
/// may pay life equal to that spell's mana value." (Marshland Bloodcaster):
/// a one-shot alternative cost for the next matching spell. The spell
/// description is the one the free-cast grant reads.
fn parse_next_spell_life_alternative_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    const HEAD: &[&str] = &["rather", "than", "pay", "the", "mana", "cost", "of"];
    let Some(comma) = tokens.iter().position(OwnedLexToken::is_comma) else {
        return Ok(None);
    };
    let head_words = crate::lexer::token_word_refs(&tokens[..comma]);
    if !head_words.starts_with(HEAD) {
        return Ok(None);
    }
    let tail_words = crate::lexer::token_word_refs(&tokens[comma + 1..]);
    let pays_life = matches!(
        tail_words.as_slice(),
        ["you", "may", "pay", "life", "equal", "to", "that", possessive, "mana", "value"]
            if matches!(*possessive, "spell's" | "spells" | "spell’s")
    );
    if !pays_life {
        return Ok(None);
    }
    // The described spell, read as "<spell> can be cast without paying its
    // mana cost" by the next-spell grant grammar.
    let Some(spell_start) = crate::lexer::TokenWordView::new(&tokens[..comma])
        .token_index_after_words(HEAD.len())
    else {
        return Ok(None);
    };
    let mut described = tokens[spell_start..comma].to_vec();
    described.extend(crate::lexer::synthetic_word_tokens([
        "can", "be", "cast", "without", "paying", "its", "mana", "cost",
    ]));
    let Some(shape) = parse_next_spell_grant_tokens(&described)? else {
        return Ok(None);
    };
    if shape.ability
        != super::super::grammar::effects::NextSpellGrantAbilitySurface::WithoutPayingManaCost
    {
        return Ok(None);
    }
    let Ok([filter]) = <[_; 1]>::try_from(shape.filters) else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::subject_verb_pay_life_for_next_spell_this_turn(
        shape.player,
        filter,
    )]))
}

pub fn parse_next_spell_grant_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(effects) = parse_next_spell_life_alternative_sentence(tokens)? {
        return Ok(Some(effects));
    }
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
    let Some(split) = crate::slice_primitives::find_window_by(tokens, 3, |window| {
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
    if shape.ability
        == super::super::grammar::effects::NextSpellGrantAbilitySurface::WithoutPayingManaCost
    {
        let [filter] = <[_; 1]>::try_from(shape.filters).ok()?;
        return Some(vec![
            EffectAst::subject_verb_free_cast_next_spell_this_turn(shape.player, filter),
        ]);
    }
    if matches!(shape.ability, NextSpellGrantAbilitySurface::CastTiming | NextSpellGrantAbilitySurface::PlayTiming) {
        return Some(shape.filters.into_iter().map(|filter| {
            EffectAst::subject_verb_next_play_timing_this_turn(
                shape.player, filter, shape.ability == NextSpellGrantAbilitySurface::PlayTiming,
            )
        }).collect());
    }
    let ability = parse_next_spell_grant_ability(shape.ability)?;
    let effects = shape
        .filters
        .into_iter()
        .map(|filter| {
            EffectAst::subject_verb_grant_next_spell_ability_this_turn(
                shape.player,
                filter,
                ability.clone(),
            )
        })
        .collect::<Vec<_>>();
    Some(if effects.len() > 1 {
        vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]
    } else {
        effects
    })
}
