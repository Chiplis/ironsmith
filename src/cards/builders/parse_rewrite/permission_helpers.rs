use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, PlayerAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, TagKey, Token,
};
use crate::effect::{Until, Value};
use crate::static_abilities::StaticAbilityId;
use crate::target::ObjectFilter;
use crate::zone::Zone;

use super::object_filters::{merge_spell_filters, parse_object_filter, parse_spell_filter};
use super::util::{
    parse_value, starts_with_until_end_of_turn, token_index_for_word_index, trim_commas, words,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionLifetime {
    Immediate,
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
    Static,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PermissionClauseSpec {
    Tagged {
        player: PlayerAst,
        allow_land: bool,
        as_copy: bool,
        without_paying_mana_cost: bool,
        lifetime: PermissionLifetime,
    },
    GrantBySpec {
        player: PlayerAst,
        spec: crate::grant::GrantSpec,
        lifetime: PermissionLifetime,
    },
}

fn is_until_end_of_turn(words: &[&str]) -> bool {
    words == ["until", "end", "of", "turn"]
}

fn parse_tagged_cast_or_play_target(words: &[&str]) -> Option<(bool, usize)> {
    if words.starts_with(&["one", "of", "those", "cards"])
        || words.starts_with(&["one", "of", "those", "card"])
    {
        return Some((false, 4));
    }
    if words.starts_with(&["one", "of", "them"]) {
        return Some((false, 3));
    }
    if words.starts_with(&["it"]) || words.starts_with(&["them"]) {
        return Some((false, 1));
    }
    if words.starts_with(&["that", "card"])
        || words.starts_with(&["those", "cards"])
        || words.starts_with(&["that", "spell"])
        || words.starts_with(&["those", "spells"])
        || words.starts_with(&["the", "card"])
        || words.starts_with(&["the", "cards"])
    {
        return Some((false, 2));
    }
    if words.starts_with(&["the", "copy"])
        || words.starts_with(&["that", "copy"])
        || words.starts_with(&["a", "copy"])
    {
        return Some((true, 2));
    }
    None
}

fn parse_permission_duration_prefix(words: &[&str]) -> Option<(PermissionLifetime, usize)> {
    if words.starts_with(&["until", "the", "end", "of", "your", "next", "turn"]) {
        return Some((PermissionLifetime::UntilYourNextTurn, 7));
    }
    if words.starts_with(&["until", "end", "of", "your", "next", "turn"]) {
        return Some((PermissionLifetime::UntilYourNextTurn, 6));
    }
    if words.starts_with(&["until", "the", "end", "of", "turn"]) {
        return Some((PermissionLifetime::UntilEndOfTurn, 5));
    }
    if starts_with_until_end_of_turn(words) {
        return Some((PermissionLifetime::UntilEndOfTurn, 4));
    }
    None
}

fn is_without_paying_mana_cost_tail(words: &[&str]) -> bool {
    matches!(
        words,
        ["without", "paying", "its", "mana", "cost"]
            | ["without", "paying", "their", "mana", "cost"]
            | ["without", "paying", "their", "mana", "costs"]
            | ["without", "paying", "that", "card", "mana", "cost"]
            | ["without", "paying", "that", "cards", "mana", "cost"]
    )
}

fn parse_permission_tail(
    words: &[&str],
    default_lifetime: PermissionLifetime,
) -> Option<(PermissionLifetime, bool)> {
    if words.is_empty() {
        return Some((default_lifetime, false));
    }
    if is_without_paying_mana_cost_tail(words) {
        return Some((default_lifetime, true));
    }
    if words == ["this", "turn"] {
        return Some((PermissionLifetime::ThisTurn, false));
    }
    if words.starts_with(&["this", "turn"]) && is_without_paying_mana_cost_tail(&words[2..]) {
        return Some((PermissionLifetime::ThisTurn, true));
    }
    if is_until_end_of_turn(words) || words == ["until", "the", "end", "of", "turn"] {
        return Some((PermissionLifetime::UntilEndOfTurn, false));
    }
    if words == ["until", "end", "of", "your", "next", "turn"]
        || words == ["until", "the", "end", "of", "your", "next", "turn"]
    {
        return Some((PermissionLifetime::UntilYourNextTurn, false));
    }
    if (words.starts_with(&["until", "end", "of", "turn"])
        || words.starts_with(&["until", "the", "end", "of", "turn"]))
        && is_without_paying_mana_cost_tail(&words[if words[1] == "the" { 5 } else { 4 }..])
    {
        return Some((PermissionLifetime::UntilEndOfTurn, true));
    }
    None
}

fn normalize_permission_subject_filter(mut filter: ObjectFilter) -> ObjectFilter {
    filter.zone = None;
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter
}

fn parse_permission_subject_filter_tokens(
    filter_tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    for separator in ["and", "or"] {
        let Some(split_idx) = filter_tokens
            .iter()
            .position(|token| token.is_word(separator))
        else {
            continue;
        };
        let left_tokens = trim_commas(&filter_tokens[..split_idx]);
        let right_tokens = trim_commas(&filter_tokens[split_idx + 1..]);
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        let Ok(left) = parse_object_filter(&left_tokens, false) else {
            continue;
        };
        let Ok(right) = parse_object_filter(&right_tokens, false) else {
            continue;
        };
        return Ok(Some(ObjectFilter {
            any_of: vec![
                normalize_permission_subject_filter(left),
                normalize_permission_subject_filter(right),
            ],
            ..ObjectFilter::default()
        }));
    }

    if let Ok(filter) = parse_object_filter(filter_tokens, false) {
        return Ok(Some(normalize_permission_subject_filter(filter)));
    }

    Ok(None)
}

pub(crate) fn parse_permission_clause_spec(
    tokens: &[Token],
) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let (prefixed_lifetime, prefix_len) = parse_permission_duration_prefix(&clause_words)
        .map_or((None, 0usize), |(lifetime, consumed)| {
            (Some(lifetime), consumed)
        });
    let body = &clause_words[prefix_len..];

    let Some((player, allow_land, lead_len)) = (if body.starts_with(&["you", "may", "cast"]) {
        Some((PlayerAst::You, false, 3usize))
    } else if body.starts_with(&["you", "may", "play"]) {
        Some((PlayerAst::You, true, 3usize))
    } else if body.starts_with(&["cast"]) {
        Some((PlayerAst::Implicit, false, 1usize))
    } else if body.starts_with(&["play"]) {
        Some((PlayerAst::Implicit, true, 1usize))
    } else {
        None
    }) else {
        return Ok(None);
    };

    let rest = &body[lead_len..];
    if let Some((as_copy, consumed)) = parse_tagged_cast_or_play_target(rest) {
        let mut tail = &rest[consumed..];
        if tail.starts_with(&["from", "exile"]) {
            tail = &tail[2..];
        }

        let default_lifetime = prefixed_lifetime.unwrap_or(PermissionLifetime::Immediate);
        let Some((lifetime, without_paying_mana_cost)) =
            parse_permission_tail(tail, default_lifetime)
        else {
            if let Some(prefixed) = prefixed_lifetime {
                let label = match prefixed {
                    PermissionLifetime::UntilEndOfTurn => "until-end-of-turn",
                    PermissionLifetime::UntilYourNextTurn => "until-next-turn",
                    _ => "permission",
                };
                return Err(CardTextError::ParseError(format!(
                    "unsupported {label} play target (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(None);
        };

        let single_tagged_target = matches!(
            &rest[..consumed],
            ["it"] | ["that", "card"] | ["that", "spell"]
        );
        if matches!(
            lifetime,
            PermissionLifetime::ThisTurn
                | PermissionLifetime::UntilEndOfTurn
                | PermissionLifetime::UntilYourNextTurn
        ) && as_copy
        {
            let label = match lifetime {
                PermissionLifetime::UntilYourNextTurn => "until-next-turn",
                _ => "until-end-of-turn",
            };
            return Err(CardTextError::ParseError(format!(
                "unsupported {label} play target (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if without_paying_mana_cost
            && matches!(
                lifetime,
                PermissionLifetime::ThisTurn | PermissionLifetime::UntilEndOfTurn
            )
            && !single_tagged_target
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported temporary play/cast permission clause with alternative cost (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if lifetime == PermissionLifetime::UntilYourNextTurn
            && (!allow_land || without_paying_mana_cost)
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported until-next-turn play target (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(Some(PermissionClauseSpec::Tagged {
            player,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            lifetime,
        }));
    }

    if allow_land
        && rest.starts_with(&["lands", "and", "cast"])
        && let Some(from_idx) = rest.iter().position(|word| *word == "from")
    {
        let zone_words = &rest[from_idx..];
        if zone_words == ["from", "the", "top", "of", "your", "library"] {
            let subject_words = &rest[3..from_idx];
            let filter = if subject_words == ["spells"] {
                ObjectFilter::default()
            } else {
                let filter_start_word_idx = prefix_len + lead_len + 3;
                let Some(filter_start_token_idx) =
                    token_index_for_word_index(tokens, filter_start_word_idx)
                else {
                    return Ok(None);
                };
                let Some(filter_end_token_idx) =
                    token_index_for_word_index(tokens, filter_start_word_idx + subject_words.len())
                else {
                    return Ok(None);
                };
                let subject_tokens =
                    trim_commas(&tokens[filter_start_token_idx..filter_end_token_idx]);
                let Some(spell_filter) = parse_permission_subject_filter_tokens(&subject_tokens)?
                else {
                    return Ok(None);
                };
                ObjectFilter {
                    any_of: vec![ObjectFilter::land(), spell_filter],
                    ..ObjectFilter::default()
                }
            };

            return Ok(Some(PermissionClauseSpec::GrantBySpec {
                player,
                spec: crate::grant::GrantSpec::new(
                    crate::grant::Grantable::play_from(),
                    filter,
                    Zone::Library,
                ),
                lifetime: PermissionLifetime::Static,
            }));
        }
    }

    if !allow_land {
        let (spec, subject_len) = if rest.starts_with(&["spells"]) {
            (crate::grant::GrantSpec::flash_to_spells(), 1usize)
        } else if rest.starts_with(&["noncreature", "spells"]) {
            (
                crate::grant::GrantSpec::flash_to_noncreature_spells(),
                2usize,
            )
        } else {
            (crate::grant::GrantSpec::flash_to_spells(), 0usize)
        };
        if subject_len > 0 {
            let tail = &rest[subject_len..];
            if tail == ["as", "though", "they", "had", "flash"]
                || tail == ["as", "though", "they", "have", "flash"]
                || tail == ["this", "turn", "as", "though", "they", "had", "flash"]
                || tail == ["this", "turn", "as", "though", "they", "have", "flash"]
                || tail
                    == [
                        "until", "end", "of", "turn", "as", "though", "they", "had", "flash",
                    ]
                || tail
                    == [
                        "until", "the", "end", "of", "turn", "as", "though", "they", "had", "flash",
                    ]
            {
                let lifetime = if tail.starts_with(&["this", "turn"]) {
                    PermissionLifetime::ThisTurn
                } else if tail.starts_with(&["until"]) {
                    PermissionLifetime::UntilEndOfTurn
                } else {
                    PermissionLifetime::Static
                };
                return Ok(Some(PermissionClauseSpec::GrantBySpec {
                    player,
                    spec,
                    lifetime,
                }));
            }
        }

        let flash_tail_specs: &[(&[&str], PermissionLifetime)] = &[
            (
                &["as", "though", "they", "had", "flash"],
                PermissionLifetime::Static,
            ),
            (
                &["as", "though", "they", "have", "flash"],
                PermissionLifetime::Static,
            ),
            (
                &["this", "turn", "as", "though", "they", "had", "flash"],
                PermissionLifetime::ThisTurn,
            ),
            (
                &["this", "turn", "as", "though", "they", "have", "flash"],
                PermissionLifetime::ThisTurn,
            ),
            (
                &[
                    "until", "end", "of", "turn", "as", "though", "they", "had", "flash",
                ],
                PermissionLifetime::UntilEndOfTurn,
            ),
            (
                &[
                    "until", "the", "end", "of", "turn", "as", "though", "they", "had", "flash",
                ],
                PermissionLifetime::UntilEndOfTurn,
            ),
        ];
        for (tail, lifetime) in flash_tail_specs {
            if rest.len() <= tail.len() || !rest.ends_with(tail) {
                continue;
            }

            let subject_word_len = rest.len() - tail.len();
            let filter_start_word_idx = prefix_len + lead_len;
            let Some(filter_start_token_idx) =
                token_index_for_word_index(tokens, filter_start_word_idx)
            else {
                continue;
            };
            let Some(filter_end_token_idx) =
                token_index_for_word_index(tokens, filter_start_word_idx + subject_word_len)
            else {
                continue;
            };
            let filter_tokens = trim_commas(&tokens[filter_start_token_idx..filter_end_token_idx]);
            if filter_tokens.is_empty() {
                continue;
            }

            let Some(filter) = parse_permission_subject_filter_tokens(&filter_tokens)? else {
                continue;
            };

            return Ok(Some(PermissionClauseSpec::GrantBySpec {
                player,
                spec: crate::grant::GrantSpec::flash_to_spells_matching(filter),
                lifetime: *lifetime,
            }));
        }
    }

    if prefixed_lifetime.is_none() && !allow_land {
        let Some(from_hand_word_idx) = clause_words
            .windows(3)
            .position(|window| window == ["from", "your", "hand"])
        else {
            return Ok(None);
        };

        let suffix = &clause_words[from_hand_word_idx..];
        if !matches!(
            suffix,
            [
                "from", "your", "hand", "without", "paying", "their", "mana", "costs"
            ] | [
                "from", "your", "hand", "without", "paying", "their", "mana", "cost"
            ] | [
                "from", "your", "hand", "without", "paying", "its", "mana", "cost"
            ]
        ) {
            return Ok(None);
        }

        let filter_start_word_idx = prefix_len + lead_len;
        let Some(filter_start_token_idx) =
            token_index_for_word_index(tokens, filter_start_word_idx)
        else {
            return Ok(None);
        };
        let Some(filter_end_token_idx) = token_index_for_word_index(tokens, from_hand_word_idx)
        else {
            return Ok(None);
        };
        let filter_tokens = trim_commas(&tokens[filter_start_token_idx..filter_end_token_idx]);
        let filter_words = words(&filter_tokens);
        if filter_words.is_empty()
            || !filter_words
                .iter()
                .any(|word| *word == "spell" || *word == "spells")
        {
            return Ok(None);
        }

        let mut filter = ObjectFilter::nonland();
        merge_spell_filters(&mut filter, parse_spell_filter(&filter_tokens));
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec: crate::grant::GrantSpec::cast_from_hand_without_paying_mana_cost_matching(filter),
            lifetime: PermissionLifetime::Static,
        }));
    }

    Ok(None)
}

fn clause_has_may_play_or_cast(words: &[&str]) -> bool {
    words
        .windows(2)
        .any(|window| matches!(window, ["may", "play"] | ["may", "cast"]))
}

pub(crate) fn parse_unsupported_play_cast_permission_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    if clause_words
        == [
            "play", "any", "number", "of", "lands", "on", "each", "of", "your", "turns",
        ]
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported additional-land-play permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if clause_words.starts_with(&["for", "as", "long", "as"])
        && clause_has_may_play_or_cast(&clause_words)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported for-as-long-as play/cast permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if clause_words.starts_with(&["once", "during", "each", "of", "your", "turns"])
        && clause_words.contains(&"graveyard")
        && clause_has_may_play_or_cast(&clause_words)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported once-per-turn graveyard play/cast permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let _ = parse_permission_clause_spec(tokens)?;
    Ok(None)
}

pub(crate) fn parse_until_end_of_turn_may_play_tagged_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(PermissionClauseSpec::Tagged {
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost,
            lifetime: PermissionLifetime::UntilEndOfTurn,
        }) if player == PlayerAst::You => Ok(Some(EffectAst::GrantPlayTaggedUntilEndOfTurn {
            tag: TagKey::from(IT_TAG),
            player,
            allow_land,
            without_paying_mana_cost,
        })),
        _ => Ok(None),
    }
}

pub(crate) fn parse_until_your_next_turn_may_play_tagged_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(PermissionClauseSpec::Tagged {
            player,
            allow_land: true,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: PermissionLifetime::UntilYourNextTurn,
        }) if matches!(player, PlayerAst::You | PlayerAst::Implicit) => {
            Ok(Some(EffectAst::GrantPlayTaggedUntilYourNextTurn {
                tag: TagKey::from(IT_TAG),
                player: PlayerAst::You,
                allow_land: true,
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_additional_land_plays_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("play") {
        return Ok(None);
    }

    let rest_tokens = &tokens[1..];
    let (count, used) = if rest_tokens.first().is_some_and(|token| token.is_word("an"))
        || rest_tokens.first().is_some_and(|token| token.is_word("a"))
    {
        (Value::Fixed(1), 1usize)
    } else if let Some((value, used)) = parse_value(rest_tokens) {
        (value, used)
    } else {
        return Ok(None);
    };

    let tail_words = words(&rest_tokens[used..]);
    let singular = ["additional", "land", "this", "turn"];
    let plural = ["additional", "lands", "this", "turn"];
    if tail_words.as_slice() != singular && tail_words.as_slice() != plural {
        return Ok(None);
    }

    Ok(Some(EffectAst::AdditionalLandPlays {
        count,
        player: PlayerAst::Implicit,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_cast_spells_as_though_they_had_flash_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: PermissionLifetime::ThisTurn | PermissionLifetime::UntilEndOfTurn,
        }) if spec == crate::grant::GrantSpec::flash_to_spells()
            || spec == crate::grant::GrantSpec::flash_to_noncreature_spells() =>
        {
            Ok(Some(EffectAst::GrantBySpec {
                spec,
                player,
                duration: crate::grant::GrantDuration::UntilEndOfTurn,
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_cast_or_play_tagged_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut trimmed = trim_commas(tokens).to_vec();
    while trimmed
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        trimmed.remove(0);
    }

    match parse_permission_clause_spec(&trimmed)? {
        Some(PermissionClauseSpec::Tagged {
            player,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            lifetime: PermissionLifetime::Immediate,
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => {
            Ok(Some(EffectAst::CastTagged {
                tag: TagKey::from(IT_TAG),
                allow_land,
                as_copy,
                without_paying_mana_cost,
            }))
        }
        Some(PermissionClauseSpec::Tagged {
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost,
            lifetime: PermissionLifetime::ThisTurn | PermissionLifetime::UntilEndOfTurn,
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => {
            Ok(Some(EffectAst::GrantPlayTaggedUntilEndOfTurn {
                tag: TagKey::from(IT_TAG),
                player: PlayerAst::Implicit,
                allow_land,
                without_paying_mana_cost,
            }))
        }
        _ => Ok(None),
    }
}
