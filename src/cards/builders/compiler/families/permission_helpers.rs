use crate::cards::builders::{CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, TagKey};
use crate::effect::{Until, Value, ValueComparisonOperator};
use crate::target::ObjectFilter;
use crate::zone::Zone;
use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use super::grammar::filters::{
    parse_object_filter_with_grammar_entrypoint_lexed,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
use super::grammar::primitives as grammar;
use super::grammar::values::parse_value_comparison_tokens;
use super::lexer::{LexStream, OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use super::object_filters::merge_spell_filters;
use super::token_primitives::{
    TurnDurationPhrase, contains_window as word_slice_has_sequence, find_index as find_token_index,
    parse_i32_word_token, parse_lexed_prefix, parse_turn_duration_prefix,
    parse_turn_duration_suffix, slice_contains as word_slice_has,
    slice_starts_with as word_slice_has_prefix,
};
use super::util::{token_index_for_word_index, trim_commas};
use super::value_helpers::parse_value_from_lexed;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PermissionLead {
    player: PlayerAst,
    allow_land: bool,
}

fn parse_permission_lead_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<PermissionLead, ErrMode<ContextError>> {
    alt((
        grammar::phrase(&["you", "may", "cast"]).value(PermissionLead {
            player: PlayerAst::You,
            allow_land: false,
        }),
        grammar::phrase(&["you", "may", "play"]).value(PermissionLead {
            player: PlayerAst::You,
            allow_land: true,
        }),
        grammar::phrase(&["any", "player", "may", "cast"]).value(PermissionLead {
            player: PlayerAst::Any,
            allow_land: false,
        }),
        grammar::phrase(&["any", "player", "may", "play"]).value(PermissionLead {
            player: PlayerAst::Any,
            allow_land: true,
        }),
        grammar::phrase(&["cast"]).value(PermissionLead {
            player: PlayerAst::Implicit,
            allow_land: false,
        }),
        grammar::phrase(&["play"]).value(PermissionLead {
            player: PlayerAst::Implicit,
            allow_land: true,
        }),
    ))
    .parse_next(input)
}

fn parse_tagged_cast_or_play_target_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<bool, ErrMode<ContextError>> {
    alt((
        alt((
            grammar::phrase(&["spells", "from", "among", "those", "cards"]).value(false),
            grammar::phrase(&["spells", "from", "among", "them"]).value(false),
            grammar::phrase(&["one", "of", "those", "cards"]).value(false),
            grammar::phrase(&["one", "of", "those", "card"]).value(false),
            grammar::phrase(&["one", "of", "them"]).value(false),
            grammar::phrase(&["it"]).value(false),
            grammar::phrase(&["them"]).value(false),
            grammar::phrase(&["that", "card"]).value(false),
            grammar::phrase(&["those", "cards"]).value(false),
        )),
        alt((
            grammar::phrase(&["that", "spell"]).value(false),
            grammar::phrase(&["those", "spells"]).value(false),
            grammar::phrase(&["that", "exiled", "card"]).value(false),
            grammar::phrase(&["the", "exiled", "card"]).value(false),
            grammar::phrase(&["the", "card"]).value(false),
            grammar::phrase(&["the", "cards"]).value(false),
            grammar::phrase(&["the", "copy"]).value(true),
            grammar::phrase(&["that", "copy"]).value(true),
            grammar::phrase(&["a", "copy"]).value(true),
        )),
    ))
    .parse_next(input)
}

fn parse_without_paying_mana_cost_tail_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        grammar::phrase(&["without", "paying", "its", "mana", "cost"]),
        grammar::phrase(&["without", "paying", "their", "mana", "cost"]),
        grammar::phrase(&["without", "paying", "their", "mana", "costs"]),
        grammar::phrase(&["without", "paying", "that", "card", "mana", "cost"]),
        grammar::phrase(&["without", "paying", "that", "cards", "mana", "cost"]),
    ))
    .void()
    .parse_next(input)
}

fn parse_tagged_permission_mana_value_condition_prefix_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        grammar::phrase(&["if", "it's", "a", "spell", "with", "mana", "value"]),
        grammar::phrase(&["if", "it", "is", "a", "spell", "with", "mana", "value"]),
        grammar::phrase(&["if", "the", "spell's", "mana", "value"]),
        grammar::phrase(&["if", "the", "spells", "mana", "value"]),
        grammar::phrase(&["if", "that", "spell's", "mana", "value"]),
        grammar::phrase(&["if", "that", "spells", "mana", "value"]),
        grammar::phrase(&["if", "its", "mana", "value"]),
    ))
    .void()
    .parse_next(input)
}

fn parse_flash_tail_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<PermissionLifetime, ErrMode<ContextError>> {
    alt((
        grammar::phrase(&["as", "though", "they", "had", "flash"])
            .value(PermissionLifetime::Static),
        grammar::phrase(&["as", "though", "they", "have", "flash"])
            .value(PermissionLifetime::Static),
        grammar::phrase(&["this", "turn", "as", "though", "they", "had", "flash"])
            .value(PermissionLifetime::ThisTurn),
        grammar::phrase(&["this", "turn", "as", "though", "they", "have", "flash"])
            .value(PermissionLifetime::ThisTurn),
        grammar::phrase(&[
            "until", "end", "of", "turn", "as", "though", "they", "had", "flash",
        ])
        .value(PermissionLifetime::UntilEndOfTurn),
        grammar::phrase(&[
            "until", "the", "end", "of", "turn", "as", "though", "they", "had", "flash",
        ])
        .value(PermissionLifetime::UntilEndOfTurn),
    ))
    .parse_next(input)
}

fn parse_exact_lexed_prefix<'a, O>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
) -> Option<O> {
    parse_lexed_prefix(tokens, parser).and_then(|(parsed, rest)| rest.is_empty().then_some(parsed))
}

fn strip_flash_tail_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], PermissionLifetime)> {
    const PHRASES: &[&[&str]] = &[
        &["as", "though", "they", "had", "flash"],
        &["as", "though", "they", "have", "flash"],
        &["this", "turn", "as", "though", "they", "had", "flash"],
        &["this", "turn", "as", "though", "they", "have", "flash"],
        &[
            "until", "end", "of", "turn", "as", "though", "they", "had", "flash",
        ],
        &[
            "until", "the", "end", "of", "turn", "as", "though", "they", "had", "flash",
        ],
    ];
    let (phrase, rest) = grammar::strip_lexed_suffix_phrases(tokens, PHRASES)?;
    let lifetime = match *phrase {
        ["as", "though", "they", "had", "flash"] | ["as", "though", "they", "have", "flash"] => {
            PermissionLifetime::Static
        }
        ["this", "turn", "as", "though", "they", "had", "flash"]
        | ["this", "turn", "as", "though", "they", "have", "flash"] => PermissionLifetime::ThisTurn,
        [
            "until",
            "end",
            "of",
            "turn",
            "as",
            "though",
            "they",
            "had",
            "flash",
        ]
        | [
            "until",
            "the",
            "end",
            "of",
            "turn",
            "as",
            "though",
            "they",
            "had",
            "flash",
        ] => PermissionLifetime::UntilEndOfTurn,
        _ => return None,
    };
    Some((rest, lifetime))
}

fn strip_cast_from_hand_without_paying_mana_cost_suffix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<&'a [OwnedLexToken]> {
    grammar::strip_lexed_suffix_phrases(
        tokens,
        &[
            &[
                "from", "your", "hand", "without", "paying", "their", "mana", "costs",
            ][..],
            &[
                "from", "your", "hand", "without", "paying", "their", "mana", "cost",
            ][..],
            &[
                "from", "your", "hand", "without", "paying", "its", "mana", "cost",
            ][..],
        ],
    )
    .map(|(_, rest)| rest)
}

fn strip_allow_any_color_for_cast_suffix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<&'a [OwnedLexToken]> {
    grammar::strip_lexed_suffix_phrases(
        tokens,
        &[
            &[
                "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "them",
            ][..],
            &[
                "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "it",
            ][..],
            &[
                "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "that",
                "spell",
            ][..],
        ],
    )
    .map(|(_, rest)| rest)
}

fn parse_without_paying_mana_cost_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    parse_exact_lexed_prefix(tokens, parse_without_paying_mana_cost_tail_inner).is_some()
}

fn parse_permission_duration_prefix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> (Option<PermissionLifetime>, &'a [OwnedLexToken]) {
    if let Some((duration, rest)) = parse_turn_duration_prefix(tokens) {
        return (Some(permission_lifetime_from_turn_duration(duration)), rest);
    }

    (None, tokens)
}

fn permission_lifetime_from_turn_duration(duration: TurnDurationPhrase) -> PermissionLifetime {
    match duration {
        TurnDurationPhrase::ThisTurn => PermissionLifetime::ThisTurn,
        TurnDurationPhrase::UntilEndOfTurn => PermissionLifetime::UntilEndOfTurn,
        TurnDurationPhrase::UntilYourNextTurn => PermissionLifetime::UntilYourNextTurn,
    }
}

fn strip_prefix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<&'a [OwnedLexToken]> {
    grammar::parse_prefix(tokens, grammar::phrase(phrase)).map(|(_, rest)| rest)
}

fn parse_permission_lead_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(PermissionLead, &'a [OwnedLexToken])> {
    parse_lexed_prefix(tokens, parse_permission_lead_inner)
}

fn parse_tagged_cast_or_play_target_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(bool, &'a [OwnedLexToken])> {
    parse_lexed_prefix(tokens, parse_tagged_cast_or_play_target_inner)
}

fn parse_tagged_permission_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(ValueComparisonOperator, Value)> {
    let (_, after_prefix) = parse_lexed_prefix(
        tokens,
        parse_tagged_permission_mana_value_condition_prefix_inner,
    )?;
    let (operator, operand_tokens) = parse_value_comparison_tokens(after_prefix)?;
    let (value, trailing) = parse_lexed_prefix(operand_tokens, parse_i32_word_token)?;
    if trailing.is_empty() {
        return Some((operator, Value::Fixed(value)));
    }

    None
}

fn parse_permission_tail_tokens(
    tokens: &[OwnedLexToken],
    default_lifetime: PermissionLifetime,
) -> Option<(PermissionLifetime, bool)> {
    if tokens.is_empty() {
        return Some((default_lifetime, false));
    }
    if parse_without_paying_mana_cost_tail_tokens(tokens) {
        return Some((default_lifetime, true));
    }

    if let Some((duration, rest)) = parse_turn_duration_prefix(tokens) {
        if rest.is_empty() {
            return Some((permission_lifetime_from_turn_duration(duration), false));
        }
        if parse_without_paying_mana_cost_tail_tokens(rest) {
            return Some((permission_lifetime_from_turn_duration(duration), true));
        }
    }

    if let Some((rest, duration)) = parse_turn_duration_suffix(tokens) {
        if rest.is_empty() {
            return Some((permission_lifetime_from_turn_duration(duration), false));
        }
        if parse_without_paying_mana_cost_tail_tokens(rest) {
            return Some((permission_lifetime_from_turn_duration(duration), true));
        }
    }

    None
}

fn normalize_permission_subject_filter(mut filter: ObjectFilter) -> ObjectFilter {
    filter.zone = None;
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter
}

fn parse_permission_subject_filter_tokens_lexed(
    filter_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let filter_words = token_word_refs(filter_tokens);
    for separator in ["and", "or"] {
        let Some(split_idx) = find_token_index(filter_words.as_slice(), |word| *word == separator)
        else {
            continue;
        };
        let Some(split_token_idx) = token_index_for_word_index(filter_tokens, split_idx) else {
            continue;
        };
        let left_tokens = trim_lexed_commas(&filter_tokens[..split_token_idx]);
        let right_tokens = trim_lexed_commas(&filter_tokens[split_token_idx + 1..]);
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        let Ok(left) = parse_object_filter_with_grammar_entrypoint_lexed(left_tokens, false) else {
            continue;
        };
        let Ok(right) = parse_object_filter_with_grammar_entrypoint_lexed(right_tokens, false)
        else {
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

    if let Ok(filter) = parse_object_filter_with_grammar_entrypoint_lexed(filter_tokens, false) {
        return Ok(Some(normalize_permission_subject_filter(filter)));
    }

    Ok(None)
}

pub(crate) fn parse_permission_clause_spec(
    tokens: &[OwnedLexToken],
) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    parse_permission_clause_spec_lexed(tokens)
}

pub(crate) fn parse_unsupported_play_cast_permission_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_unsupported_play_cast_permission_clause_lexed(tokens)
}

pub(crate) fn parse_permission_clause_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let mut tokens = trim_lexed_commas(tokens);
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Period))
    {
        tokens = &tokens[..tokens.len() - 1];
        tokens = trim_lexed_commas(tokens);
    }

    let clause_refs = token_word_refs(tokens);
    if clause_refs.is_empty() {
        return Ok(None);
    }

    let (prefixed_lifetime, body_tokens) = parse_permission_duration_prefix_tokens(tokens);
    let body_tokens = trim_lexed_commas(body_tokens);
    let Some((lead, rest_tokens)) = parse_permission_lead_tokens(body_tokens) else {
        return Ok(None);
    };
    let player = lead.player;
    let allow_land = lead.allow_land;

    if let Some((as_copy, tagged_tail_tokens)) =
        parse_tagged_cast_or_play_target_tokens(rest_tokens)
    {
        let target_len = rest_tokens.len() - tagged_tail_tokens.len();
        let target_tokens = &rest_tokens[..target_len];
        let mut tail_tokens = tagged_tail_tokens;
        if let Some(rest) = strip_prefix_phrase(tail_tokens, &["from", "exile"]) {
            tail_tokens = rest;
        }

        let default_lifetime = prefixed_lifetime.unwrap_or(PermissionLifetime::Immediate);
        let Some((lifetime, without_paying_mana_cost)) =
            parse_permission_tail_tokens(tail_tokens, default_lifetime)
        else {
            if let Some(prefixed) = prefixed_lifetime {
                let label = match prefixed {
                    PermissionLifetime::UntilEndOfTurn => "until-end-of-turn",
                    PermissionLifetime::UntilYourNextTurn => "until-next-turn",
                    _ => "permission",
                };
                return Err(CardTextError::ParseError(format!(
                    "unsupported {label} play target (clause: '{}')",
                    clause_refs.join(" ")
                )));
            }
            return Ok(None);
        };

        let single_tagged_target = matches!(
            token_word_refs(target_tokens).as_slice(),
            ["it"] | ["that", "card"] | ["that", "spell"]
        );
        let plural_tagged_cards_target = matches!(
            token_word_refs(target_tokens).as_slice(),
            ["those", "cards"]
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
                clause_refs.join(" ")
            )));
        }
        if without_paying_mana_cost
            && matches!(
                lifetime,
                PermissionLifetime::ThisTurn | PermissionLifetime::UntilEndOfTurn
            )
            && !single_tagged_target
            && !plural_tagged_cards_target
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported temporary play/cast permission clause with alternative cost (clause: '{}')",
                clause_refs.join(" ")
            )));
        }
        if lifetime == PermissionLifetime::UntilYourNextTurn
            && (!allow_land || without_paying_mana_cost)
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported until-next-turn play target (clause: '{}')",
                clause_refs.join(" ")
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
        && let Some(after_lands_and_cast) =
            strip_prefix_phrase(rest_tokens, &["lands", "and", "cast"])
        && let Some(from_idx) =
            find_token_index(after_lands_and_cast, |token| token.is_word("from"))
    {
        let zone_words = token_word_refs(&after_lands_and_cast[from_idx..]);
        if zone_words == ["from", "the", "top", "of", "your", "library"] {
            let subject_tokens = trim_lexed_commas(&after_lands_and_cast[..from_idx]);
            let subject_words = token_word_refs(subject_tokens);
            let filter = if subject_words == ["spells"] {
                ObjectFilter::default()
            } else {
                let Some(spell_filter) =
                    parse_permission_subject_filter_tokens_lexed(subject_tokens)?
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
        let (spec, subject_tokens) = if let Some(rest) =
            strip_prefix_phrase(rest_tokens, &["spells"])
        {
            (crate::grant::GrantSpec::flash_to_spells(), Some(rest))
        } else if let Some(rest) = strip_prefix_phrase(rest_tokens, &["noncreature", "spells"]) {
            (
                crate::grant::GrantSpec::flash_to_noncreature_spells(),
                Some(rest),
            )
        } else {
            (crate::grant::GrantSpec::flash_to_spells(), None)
        };
        if let Some(tail_tokens) = subject_tokens {
            if let Some(lifetime) = parse_exact_lexed_prefix(tail_tokens, parse_flash_tail_inner) {
                return Ok(Some(PermissionClauseSpec::GrantBySpec {
                    player,
                    spec,
                    lifetime,
                }));
            }
        }

        if let Some((filter_tokens, lifetime)) = strip_flash_tail_tokens(rest_tokens) {
            let filter_tokens = trim_lexed_commas(filter_tokens);
            if !filter_tokens.is_empty()
                && let Some(filter) = parse_permission_subject_filter_tokens_lexed(filter_tokens)?
            {
                return Ok(Some(PermissionClauseSpec::GrantBySpec {
                    player,
                    spec: crate::grant::GrantSpec::flash_to_spells_matching(filter),
                    lifetime,
                }));
            }
        }
    }

    if prefixed_lifetime.is_none() && !allow_land {
        if let Some(filter_tokens) =
            strip_cast_from_hand_without_paying_mana_cost_suffix_tokens(rest_tokens)
        {
            let filter_tokens = trim_lexed_commas(filter_tokens);
            let filter_refs = token_word_refs(filter_tokens);
            if filter_refs.is_empty()
                || !filter_refs
                    .iter()
                    .any(|word| *word == "spell" || *word == "spells")
            {
                return Ok(None);
            }

            let mut filter = ObjectFilter::nonland();
            merge_spell_filters(
                &mut filter,
                parse_spell_filter_with_grammar_entrypoint_lexed(filter_tokens),
            );
            return Ok(Some(PermissionClauseSpec::GrantBySpec {
                player,
                spec: crate::grant::GrantSpec::cast_from_hand_without_paying_mana_cost_matching(
                    filter,
                ),
                lifetime: PermissionLifetime::Static,
            }));
        }
    }

    Ok(None)
}

pub(crate) fn parse_unsupported_play_cast_permission_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_refs = token_word_refs(tokens);
    if clause_refs.is_empty() {
        return Ok(None);
    }

    if clause_refs
        == [
            "play", "any", "number", "of", "lands", "on", "each", "of", "your", "turns",
        ]
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported additional-land-play permission clause (clause: '{}')",
            clause_refs.join(" ")
        )));
    }

    if word_slice_has_prefix(&clause_refs, &["for", "as", "long", "as"])
        && (word_slice_has_sequence(&clause_refs, &["may", "play"])
            || word_slice_has_sequence(&clause_refs, &["may", "cast"]))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported for-as-long-as play/cast permission clause (clause: '{}')",
            clause_refs.join(" ")
        )));
    }

    if word_slice_has_prefix(
        &clause_refs,
        &["once", "during", "each", "of", "your", "turns"],
    ) && word_slice_has(&clause_refs, &"graveyard")
        && (word_slice_has_sequence(&clause_refs, &["may", "play"])
            || word_slice_has_sequence(&clause_refs, &["may", "cast"]))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported once-per-turn graveyard play/cast permission clause (clause: '{}')",
            clause_refs.join(" ")
        )));
    }

    let _ = parse_permission_clause_spec_lexed(tokens)?;
    Ok(None)
}

pub(crate) fn parse_until_end_of_turn_may_play_tagged_clause(
    tokens: &[OwnedLexToken],
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
            allow_any_color_for_cast: false,
        })),
        _ => Ok(None),
    }
}

pub(crate) fn parse_until_your_next_turn_may_play_tagged_clause(
    tokens: &[OwnedLexToken],
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
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_additional_land_plays_clause_lexed(tokens)
}

pub(crate) fn parse_additional_land_plays_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_refs = token_word_refs(tokens);
    if clause_refs.first().copied() != Some("play") {
        return Ok(None);
    }

    let Some(rest_start) = token_index_for_word_index(tokens, 1) else {
        return Ok(None);
    };
    let rest_tokens = &tokens[rest_start..];
    let (count, used) = if rest_tokens.first().is_some_and(|token| token.is_word("an"))
        || rest_tokens.first().is_some_and(|token| token.is_word("a"))
    {
        (Value::Fixed(1), 1usize)
    } else {
        let Some((value, used)) = parse_value_from_lexed(rest_tokens) else {
            return Ok(None);
        };
        (value, used)
    };

    let tail = &clause_refs[1 + used..];
    let singular = ["additional", "land", "this", "turn"];
    let plural = ["additional", "lands", "this", "turn"];
    if tail != singular && tail != plural {
        return Ok(None);
    }

    Ok(Some(EffectAst::AdditionalLandPlays {
        count,
        player: PlayerAst::Implicit,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_cast_spells_as_though_they_had_flash_clause(
    tokens: &[OwnedLexToken],
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
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut trimmed = trim_commas(tokens).to_vec();
    while trimmed
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        trimmed.remove(0);
    }

    let mut allow_any_color_for_cast = false;
    if let Some(stripped) = strip_allow_any_color_for_cast_suffix_tokens(&trimmed) {
        allow_any_color_for_cast = true;
        trimmed.truncate(stripped.len());
    }

    let conditional_tagged_permission = parse_permission_lead_tokens(&trimmed)
        .filter(|(lead, _)| lead.player == PlayerAst::Implicit)
        .and_then(|(lead, rest_tokens)| {
            parse_tagged_cast_or_play_target_tokens(rest_tokens).and_then(
                |(as_copy, tail_tokens)| {
                    let (lifetime, without_paying_mana_cost, condition_tokens) = if let Some(rest) =
                        strip_prefix_phrase(
                            tail_tokens,
                            &["without", "paying", "its", "mana", "cost"],
                        ) {
                        (PermissionLifetime::Immediate, true, rest)
                    } else if let Some(rest) = strip_prefix_phrase(
                        tail_tokens,
                        &["this", "turn", "without", "paying", "its", "mana", "cost"],
                    ) {
                        (PermissionLifetime::ThisTurn, true, rest)
                    } else {
                        (PermissionLifetime::Immediate, false, &tail_tokens[0..0])
                    };

                    without_paying_mana_cost.then(|| {
                        parse_tagged_permission_mana_value_condition_tokens(condition_tokens).map(
                            |(operator, right)| {
                                let inner = if lifetime == PermissionLifetime::Immediate {
                                    EffectAst::CastTagged {
                                        tag: TagKey::from(IT_TAG),
                                        allow_land: lead.allow_land,
                                        as_copy,
                                        without_paying_mana_cost,
                                        cost_reduction: None,
                                    }
                                } else {
                                    EffectAst::GrantPlayTaggedUntilEndOfTurn {
                                        tag: TagKey::from(IT_TAG),
                                        player: PlayerAst::Implicit,
                                        allow_land: lead.allow_land,
                                        without_paying_mana_cost,
                                        allow_any_color_for_cast,
                                    }
                                };
                                EffectAst::Conditional {
                                    predicate: PredicateAst::ValueComparison {
                                        left: Value::ManaValueOf(Box::new(
                                            crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG)),
                                        )),
                                        operator,
                                        right,
                                    },
                                    if_true: vec![inner],
                                    if_false: Vec::new(),
                                }
                            },
                        )
                    })?
                },
            )
        });

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
                cost_reduction: None,
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
                allow_any_color_for_cast,
            }))
        }
        _ => Ok(conditional_tagged_permission),
    }
}
