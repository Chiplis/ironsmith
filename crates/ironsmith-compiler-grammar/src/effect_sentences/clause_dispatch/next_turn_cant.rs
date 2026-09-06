use crate::cards::builders::ForEachEffectAst;
use super::super::super::activation_and_restrictions::parse_cant_restriction_clause;
use super::super::super::grammar::effects::clause_dispatch_shapes::parse_next_turn_cant_shape_tokens;
use super::super::super::lexer::OwnedLexToken;
use crate::effect::{Restriction, RestrictionStart, Until};
use crate::host::{CardTextError, EffectAst};
use crate::target::PlayerFilter;

pub(super) fn parse_next_turn_cant_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = parse_next_turn_cant_shape_tokens(tokens) else {
        return Ok(None);
    };
    let Some(parsed) = parse_cant_restriction_clause(shape.restriction_tokens)? else {
        return Ok(None);
    };

    let (nested_restriction, for_each_opponent) = match parsed.restriction {
        Restriction::CastSpellsMatching(player, spell_filter) => match player {
            PlayerFilter::Opponent => (
                Restriction::cast_spells_matching(PlayerFilter::IteratedPlayer, spell_filter),
                true,
            ),
            PlayerFilter::IteratedPlayer => (
                Restriction::cast_spells_matching(PlayerFilter::IteratedPlayer, spell_filter),
                false,
            ),
            _ => return Ok(None),
        },
        Restriction::CastMoreThanOneSpellEachTurn(player, spell_filter) => match player {
            PlayerFilter::Opponent => (
                Restriction::CastMoreThanOneSpellEachTurn(
                    PlayerFilter::IteratedPlayer,
                    spell_filter,
                ),
                true,
            ),
            PlayerFilter::IteratedPlayer => (
                Restriction::CastMoreThanOneSpellEachTurn(
                    PlayerFilter::IteratedPlayer,
                    spell_filter,
                ),
                false,
            ),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let restriction = EffectAst::subject_verb_cant_starting(
        nested_restriction,
        Until::EndOfTurn,
        RestrictionStart::NextTurn(PlayerFilter::IteratedPlayer),
        None,
    );
    Ok(Some(if for_each_opponent {
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![restriction],
        })
    } else {
        restriction
    }))
}
