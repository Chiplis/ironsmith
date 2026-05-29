use super::super::super::activation_and_restrictions::parse_cant_restriction_clause;
use super::super::super::lexer::{LexedClause, OwnedLexToken};
use crate::effect::{Restriction, Until};
use crate::host::{CardTextError, EffectAst, PlayerAst};
use crate::target::PlayerFilter;

pub(super) fn parse_next_turn_cant_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    for suffix in [
        ["during", "that", "players", "next", "turn"].as_slice(),
        ["during", "that", "player's", "next", "turn"].as_slice(),
        ["during", "that", "player", "s", "next", "turn"].as_slice(),
    ] {
        let Some(prefix_clause) = clause.strip_suffix_clause(suffix) else {
            continue;
        };
        let Some(parsed) = parse_cant_restriction_clause(prefix_clause.tokens())? else {
            continue;
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
                    _ => continue,
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
                    _ => continue,
                }
            }
            _ => continue,
        };

        return Ok(Some(EffectAst::DelayedUntilNextUpkeep {
            player: PlayerAst::That,
            effects: vec![EffectAst::subject_verb_cant(
                nested_restriction,
                Until::EndOfTurn,
                None,
            )],
        }));
    }

    Ok(None)
}
