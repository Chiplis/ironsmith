use super::super::super::activation_and_restrictions::parse_cant_restriction_clause;
use super::super::super::grammar::primitives::TokenWordView;
use super::super::super::lexer::OwnedLexToken;
use super::super::super::token_primitives::slice_ends_with as word_slice_ends_with;
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::effect::{Restriction, Until};
use crate::target::PlayerFilter;

pub(super) fn parse_next_turn_cant_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let lowered_word_view = TokenWordView::new(tokens);
    let lowered_words = lowered_word_view.to_word_refs();
    for suffix in [
        ["during", "that", "players", "next", "turn"].as_slice(),
        ["during", "that", "player's", "next", "turn"].as_slice(),
        ["during", "that", "player", "s", "next", "turn"].as_slice(),
    ] {
        if !word_slice_ends_with(&lowered_words, suffix) {
            continue;
        }

        let prefix_word_len = lowered_words.len().saturating_sub(suffix.len());
        let prefix_end = lowered_word_view
            .token_index_for_word_index(prefix_word_len)
            .unwrap_or(tokens.len());
        let prefix_tokens = &tokens[..prefix_end];
        let Some(parsed) = parse_cant_restriction_clause(prefix_tokens)? else {
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
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: Until::EndOfTurn,
                                    condition: None,
                                }],
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
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: Until::EndOfTurn,
                                    condition: None,
                                }],
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
            effects: vec![EffectAst::Cant {
                restriction: nested_restriction,
                duration: Until::EndOfTurn,
                condition: None,
            }],
        }));
    }

    Ok(None)
}
