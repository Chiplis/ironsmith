use super::super::super::activation_and_restrictions::parse_cant_restriction_clause;
use super::super::super::grammar::effects::clause_dispatch_shapes::parse_next_turn_cant_shape_tokens;
use super::super::super::lexer::OwnedLexToken;
use crate::effect::{Restriction, Until};
use crate::host::{CardTextError, EffectAst, PlayerAst};
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

    let nested_restriction = match parsed.restriction {
        Restriction::CastSpellsMatching(player, spell_filter) => {
            let nested = Restriction::cast_spells_matching(PlayerFilter::Active, spell_filter);
            match player {
                PlayerFilter::Opponent => {
                    return Ok(Some(EffectAst::ForEachOpponent {
                        effects: vec![EffectAst::DelayedUntilNextUpkeep {
                            player: PlayerAst::That,
                            effects: vec![EffectAst::subject_verb_cant(
                                nested,
                                Until::EndOfTurn,
                                None,
                            )],
                        }],
                    }));
                }
                PlayerFilter::IteratedPlayer => nested,
                _ => return Ok(None),
            }
        }
        Restriction::CastMoreThanOneSpellEachTurn(player, spell_filter) => {
            let nested =
                Restriction::CastMoreThanOneSpellEachTurn(PlayerFilter::Active, spell_filter);
            match player {
                PlayerFilter::Opponent => {
                    return Ok(Some(EffectAst::ForEachOpponent {
                        effects: vec![EffectAst::DelayedUntilNextUpkeep {
                            player: PlayerAst::That,
                            effects: vec![EffectAst::subject_verb_cant(
                                nested,
                                Until::EndOfTurn,
                                None,
                            )],
                        }],
                    }));
                }
                PlayerFilter::IteratedPlayer => nested,
                _ => return Ok(None),
            }
        }
        _ => return Ok(None),
    };

    Ok(Some(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::That,
        effects: vec![EffectAst::subject_verb_cant(
            nested_restriction,
            Until::EndOfTurn,
            None,
        )],
    }))
}
