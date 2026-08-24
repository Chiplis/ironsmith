use super::*;


pub(super) fn parse_generic_vote_start(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = GENERIC_VOTE_START_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(voters_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(options_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };

    let voters_clause = voters_clause.trimmed();
    if EACH_PLAYER_VOTER_PATTERN.locate_in(voters_clause).is_none() {
        return Ok(None);
    }
    let secret = SECRET_VOTER_PATTERN.locate_in(voters_clause).is_some();
    let starting_with_controller = STARTING_WITH_CONTROLLER_VOTER_PATTERN
        .locate_in(voters_clause)
        .is_some();

    let option_clause = vote_options_clause_before_reveal_tail(options_clause);
    if let Some(options) = named_vote_options_from_clause(option_clause) {
        return Ok(Some(EffectAst::VoteStart {
            options,
            secret,
            starting_with_controller,
        }));
    }

    let option_tokens = option_clause.tokens().to_vec();
    if let Ok(target) = parse_target_phrase(&option_tokens) {
        match target {
            TargetAst::Player(filter, _) => {
                let exclude_voter = option_clause
                    .first_word()
                    .is_some_and(|word| matches!(word, "other" | "another"));
                let filter = if exclude_voter && matches!(filter, PlayerFilter::NotYou) {
                    PlayerFilter::Any
                } else {
                    filter
                };
                return Ok(Some(EffectAst::VoteStartPlayers {
                    filter,
                    exclude_voter,
                    secret,
                    starting_with_controller,
                }));
            }
            TargetAst::Object(filter, _, _) => {
                return Ok(Some(EffectAst::VoteStartObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    secret,
                    starting_with_controller,
                }));
            }
            TargetAst::WithCount(inner, count) => {
                if let TargetAst::Object(filter, _, _) = *inner {
                    return Ok(Some(EffectAst::VoteStartObjects {
                        filter,
                        count,
                        secret,
                        starting_with_controller,
                    }));
                }
            }
            _ => {}
        }
    }
    if let Ok(filter) = parse_object_filter_lexed(&option_tokens, false)
        && filter != ObjectFilter::default()
    {
        return Ok(Some(EffectAst::VoteStartObjects {
            filter,
            count: ChoiceCount::exactly(1),
            secret,
            starting_with_controller,
        }));
    }

    let Some(options) = named_vote_options_from_clause(option_clause) else {
        return Err(CardTextError::ParseError(
            "vote clause requires at least two options".to_string(),
        ));
    };

    Ok(Some(EffectAst::VoteStart {
        options,
        secret,
        starting_with_controller,
    }))
}


pub(super) fn parse_generic_vote_option_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = parse_generic_player_vote_received_effects(tokens)? {
        return Ok(Some(effect));
    }

    let Some(shape) = effect_grammar::parse_named_vote_option_effects_shape(tokens) else {
        return Ok(None);
    };
    let option_clause = LexedClause::new(shape.option_tokens);
    let Some(option) = captured_non_article_label(option_clause) else {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    };

    let effect_tokens = trim_commas(shape.effect_tokens);
    let effects = parse_effect_chain_lexed(&effect_tokens)?;
    Ok(Some(EffectAst::VoteOption { option, effects }))
}


pub(super) fn parse_generic_player_vote_received_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = GENERIC_PLAYER_VOTE_RECEIVED_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(player_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(effect_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    let player_tokens = captured_non_article_tokens(player_clause);
    if player_tokens.is_empty() {
        return Ok(None);
    }
    let TargetAst::Player(filter, _) = parse_target_phrase(&player_tokens)? else {
        return Ok(None);
    };
    let effect_tokens = trim_commas(effect_clause.tokens());
    let effects = parse_effect_chain_lexed(&effect_tokens)?;
    if filter == PlayerFilter::You {
        return Ok(Some(EffectAst::RepeatEffects {
            count: Value::PlayerVoteCount(PlayerFilter::You),
            effects,
        }));
    }
    Ok(Some(EffectAst::ForEachPlayersFiltered {
        filter,
        effects: vec![EffectAst::RepeatEffects {
            count: Value::PlayerVoteCount(PlayerFilter::IteratedPlayer),
            effects,
        }],
    }))
}


pub(super) fn parse_generic_extra_vote(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let clause = LexedClause::new(tokens).trimmed();
    if OPTIONAL_EXTRA_VOTE_PATTERN.parse_full(clause).is_some() {
        return Some(EffectAst::VoteExtra {
            count: 1,
            optional: true,
        });
    }
    if REQUIRED_EXTRA_VOTE_PATTERN.parse_full(clause).is_some() {
        return Some(EffectAst::VoteExtra {
            count: 1,
            optional: false,
        });
    }
    if SUBJECTLESS_EXTRA_VOTE_PATTERN.parse_full(clause).is_some() {
        return Some(EffectAst::VoteExtra {
            count: 1,
            optional: false,
        });
    }
    None
}
