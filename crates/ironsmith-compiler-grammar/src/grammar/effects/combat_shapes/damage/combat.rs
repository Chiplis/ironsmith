use super::*;

pub fn parse_combat_except_filter_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatExceptFilterShape<'_>> {
    let (except_idx, (), after_except) =
        primitives::find_prefix(tokens, || primitives::phrase(&["except", "for"]).void())?;
    let included_filter_tokens = trim_lexed_commas(&tokens[..except_idx]);
    let excluded_filter_tokens = trim_lexed_commas(after_except);
    (!included_filter_tokens.is_empty() && !excluded_filter_tokens.is_empty()).then_some(
        CombatExceptFilterShape {
            included_filter_tokens,
            excluded_filter_tokens,
        },
    )
}

pub fn parse_combat_damage_target_shape_lexed(
    tokens: &[OwnedLexToken],
    used: usize,
) -> Result<CombatDamageTargetShape<'_>, CombatDamageTargetShapeError> {
    let target_tokens = normalize_damage_target_tokens(tokens, used)?;

    if let Some((instead_idx, (), predicate_tokens)) =
        primitives::find_prefix(target_tokens, || {
            primitives::phrase(&["instead", "if"]).void()
        })
    {
        return Ok(CombatDamageTargetShape::InsteadIf {
            target_tokens: trim_lexed_commas(&target_tokens[..instead_idx]),
            predicate_tokens: trim_lexed_commas(predicate_tokens),
            instead_tail_tokens: &target_tokens[instead_idx..],
        });
    }
    if let Some((unless_idx, (), predicate_tokens)) =
        primitives::find_prefix(target_tokens, || primitives::kw("unless").void())
    {
        let leading_tokens = trim_lexed_commas(&target_tokens[..unless_idx]);
        let predicate_tokens = trim_lexed_commas(predicate_tokens);
        if !leading_tokens.is_empty()
            && !predicate_tokens.is_empty()
            && let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)
        {
            return Ok(CombatDamageTargetShape::TrailingUnless {
                target_tokens: leading_tokens,
                predicate,
            });
        }
    }
    if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
        return Ok(CombatDamageTargetShape::TrailingIf {
            target_tokens: spec.leading_tokens,
            predicate: spec.predicate,
        });
    }
    if primitives::parse_prefix(target_tokens, primitives::kw("if")).is_some() {
        let predicate = parse_trailing_if_predicate_lexed(target_tokens)
            .ok_or(CombatDamageTargetShapeError::UnsupportedTrailingIfClause)?;
        return Ok(CombatDamageTargetShape::OmittedTargetIf { predicate });
    }
    if primitives::find_prefix(target_tokens, || primitives::kw("if")).is_some() {
        return Err(CombatDamageTargetShapeError::UnsupportedEmbeddedIfClause);
    }

    if let Some(shape) = parse_combat_simple_damage_target_shape_lexed(target_tokens) {
        return Ok(CombatDamageTargetShape::Simple {
            shape,
            target_tokens,
        });
    }
    if let Some((_prefix, each_of_tokens)) =
        primitives::parse_prefix(target_tokens, primitives::phrase(&["each", "of"]))
    {
        if let Some((count, used)) = parse_choice_count_before_target_prefix(each_of_tokens)
            && each_of_tokens.len() == used + 1
        {
            return Ok(CombatDamageTargetShape::EachOfCount {
                count,
                span_tokens: each_of_tokens,
            });
        }
        if has_target_marker(each_of_tokens) {
            return Ok(CombatDamageTargetShape::EachOfTarget {
                target_tokens: each_of_tokens,
            });
        }
    }
    if let Some(shape) = parse_combat_player_damage_target_shape_lexed(target_tokens, false) {
        return Ok(CombatDamageTargetShape::PlayerGroup(shape));
    }

    let each_or_all = primitives::parse_prefix(
        target_tokens,
        primitives::any_phrase(&[&["each"], &["all"]]),
    );
    let max_speed_players = each_or_all.is_some()
        && one_of_words_occurs(target_tokens, &["player", "players"])
        && phrase_occurs(target_tokens, &["max", "speed"]);
    if max_speed_players {
        let negated =
            one_of_words_occurs(target_tokens, &["does", "doesnt", "doesn", "dont", "not"])
                || phrase_occurs(target_tokens, &["does", "not"]);
        return Ok(CombatDamageTargetShape::MaxSpeedPlayers {
            has_max_speed: !negated,
        });
    }

    if primitives::parse_prefix(
        target_tokens,
        primitives::any_phrase(&[&["each", "opponent", "who"], &["each", "opponents", "who"]]),
    )
    .is_some()
        && phrase_occurs(target_tokens, &["this", "way"])
    {
        return Ok(CombatDamageTargetShape::OpponentWho {
            predicate_tokens: target_tokens.get(2..).unwrap_or_default(),
        });
    }
    if primitives::parse_prefix(
        target_tokens,
        primitives::any_phrase(&[&["each", "player", "who"], &["each", "players", "who"]]),
    )
    .is_some()
        && phrase_occurs(target_tokens, &["this", "way"])
    {
        return Ok(CombatDamageTargetShape::PlayerWho {
            predicate_tokens: target_tokens.get(2..).unwrap_or_default(),
        });
    }

    if let Some((and_idx, _phrase, _after)) = primitives::find_prefix(target_tokens, || {
        primitives::any_phrase(&[&["and", "each"], &["and", "all"]])
    }) && and_idx > 0
    {
        let player_tokens = trim_lexed_commas(&target_tokens[..and_idx]);
        let filter_tokens = trim_lexed_commas(&target_tokens[and_idx + 1..]);
        if !player_tokens.is_empty()
            && !filter_tokens.is_empty()
            && one_of_words_occurs(filter_tokens, &["creature", "creatures"])
            && let Ok(TargetAst::Player(player_filter, player_span)) =
                parse_target_phrase(player_tokens)
        {
            return Ok(CombatDamageTargetShape::PlayerAndObjects {
                player_filter,
                player_span,
                filter_tokens,
            });
        }
    }

    if each_or_all.is_some()
        && let Some((and_idx, _phrase, after_phrase)) =
            primitives::find_prefix(target_tokens, || {
                primitives::any_phrase(&[&["and", "each", "player"], &["and", "each", "players"]])
            })
        && and_idx >= 1
        && parser_token_word_refs(after_phrase).is_empty()
    {
        return Ok(CombatDamageTargetShape::EachObjectsAndPlayer {
            filter_tokens: &target_tokens[1..and_idx],
        });
    }

    if primitives::parse_prefix(
        target_tokens,
        primitives::phrase(&["each", "opponent", "and", "each"]),
    )
    .is_some()
        && one_of_words_occurs(target_tokens, &["creature"])
        && one_of_words_occurs(target_tokens, &["planeswalker"])
        && (phrase_occurs(target_tokens, &["they", "control"])
            || phrase_occurs(target_tokens, &["that", "player", "controls"]))
    {
        return Ok(CombatDamageTargetShape::OpponentAndControlledCreaturePlaneswalker);
    }

    if let Some((history_idx, (), after_history)) = primitives::find_prefix(target_tokens, || {
        primitives::phrase(&["it", "has", "dealt", "damage", "to", "this", "game"]).void()
    }) && parser_token_word_refs(after_history).is_empty()
    {
        let domains = trim_lexed_commas(&target_tokens[..history_idx]);
        if let Some((and_idx, (), _)) =
            primitives::find_prefix(domains, || primitives::kw("and").void())
        {
            let player_tokens = trim_lexed_commas(&domains[..and_idx]);
            let filter_tokens = trim_lexed_commas(&domains[and_idx + 1..]);
            if !filter_tokens.is_empty()
                && let Some(players) =
                    parse_combat_player_damage_target_shape_lexed(player_tokens, false)
            {
                return Ok(CombatDamageTargetShape::HistoricalDamageRecipients {
                    players,
                    filter_tokens,
                });
            }
        }
    }

    if let Some((_head, filter_tokens)) = each_or_all {
        if filter_tokens.is_empty() {
            return Err(CombatDamageTargetShapeError::MissingEachFilter);
        }
        return Ok(CombatDamageTargetShape::EachFilter { filter_tokens });
    }

    if let Some((at_idx, (), _after_at)) =
        primitives::find_prefix(target_tokens, || primitives::kw("at").void())
        && at_idx >= 1
        && exact_phrase(
            &target_tokens[at_idx..],
            &[
                &["at", "end", "of", "combat"],
                &["at", "the", "end", "of", "combat"],
            ],
        )
    {
        let target_tokens = trim_lexed_commas(&target_tokens[..at_idx]);
        if !target_tokens.is_empty() {
            return Ok(CombatDamageTargetShape::DelayedEndOfCombat { target_tokens });
        }
    }

    Ok(CombatDamageTargetShape::General { target_tokens })
}
