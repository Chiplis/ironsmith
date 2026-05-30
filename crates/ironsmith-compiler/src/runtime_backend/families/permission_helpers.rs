use crate::effect::{Until, Value, ValueComparisonOperator};
use crate::host::{CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, TagKey};
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use super::grammar::filters::{
    parse_object_filter_with_grammar_entrypoint_lexed,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
use super::grammar::primitives as grammar;
use super::grammar::structure::split_trailing_if_clause_lexed;
use super::grammar::values::parse_value_comparison_tokens;
use super::lexer::{
    LexStream, LexedClause, OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas,
    word_slice_contains_any_phrase, word_slice_contains_phrase, word_slice_contains_word,
    word_slice_ends_with, word_slice_starts_with,
};
use super::object_filters::merge_spell_filters;
use super::token_primitives::{
    TurnDurationPhrase, find_index as find_token_index, parse_i32_word_token, parse_lexed_prefix,
    parse_turn_duration_prefix, parse_turn_duration_suffix,
};
use super::util::{token_index_for_word_index, trim_commas};
use super::value_helpers::parse_value_from_lexed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionLifetime {
    Immediate,
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
    ForAsLongAsExiled,
    ForAsLongAsYouControlSource,
    Static,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PermissionClauseSpec {
    Tagged {
        tag: TagKey,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaggedPermissionTarget {
    tag: TagKey,
    as_copy: bool,
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
) -> Result<TaggedPermissionTarget, ErrMode<ContextError>> {
    alt((
        alt((
            grammar::phrase(&["spells", "from", "among", "those", "cards"]).value(
                TaggedPermissionTarget {
                    tag: TagKey::from(IT_TAG),
                    as_copy: false,
                },
            ),
            grammar::phrase(&["spells", "from", "among", "them"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            grammar::phrase(&["one", "of", "those", "cards"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            grammar::phrase(&["one", "of", "those", "card"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            grammar::phrase(&["one", "of", "them"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            grammar::phrase(&["it"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            grammar::phrase(&["them"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            grammar::phrase(&["that", "card"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            grammar::phrase(&["those", "cards"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
        )),
        alt((
            grammar::phrase(&["that", "spell"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            grammar::phrase(&["those", "spells"]).value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
            alt((
                grammar::phrase(&["that", "exiled", "card"]).value(TaggedPermissionTarget {
                    tag: TagKey::from(IT_TAG),
                    as_copy: false,
                }),
                grammar::phrase(&["the", "exiled", "card"]).value(TaggedPermissionTarget {
                    tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    as_copy: false,
                }),
                grammar::phrase(&["that", "revealed", "card"]).value(TaggedPermissionTarget {
                    tag: TagKey::from("__last_revealed__"),
                    as_copy: false,
                }),
                grammar::phrase(&["the", "revealed", "card"]).value(TaggedPermissionTarget {
                    tag: TagKey::from("__last_revealed__"),
                    as_copy: false,
                }),
                grammar::phrase(&["the", "card"]).value(TaggedPermissionTarget {
                    tag: TagKey::from(IT_TAG),
                    as_copy: false,
                }),
                grammar::phrase(&["the", "cards"]).value(TaggedPermissionTarget {
                    tag: TagKey::from(IT_TAG),
                    as_copy: false,
                }),
            )),
            alt((
                grammar::phrase(&["the", "copy"]).value(TaggedPermissionTarget {
                    tag: TagKey::from(IT_TAG),
                    as_copy: true,
                }),
                grammar::phrase(&["that", "copy"]).value(TaggedPermissionTarget {
                    tag: TagKey::from(IT_TAG),
                    as_copy: true,
                }),
                grammar::phrase(&["a", "copy"]).value(TaggedPermissionTarget {
                    tag: TagKey::from(IT_TAG),
                    as_copy: true,
                }),
            )),
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

fn combine_flash_permission_lifetime(
    prefixed_lifetime: Option<PermissionLifetime>,
    tail_lifetime: PermissionLifetime,
) -> PermissionLifetime {
    if tail_lifetime == PermissionLifetime::Static {
        prefixed_lifetime.unwrap_or(tail_lifetime)
    } else {
        tail_lifetime
    }
}

fn grant_spec_grants_flash_to_hand(spec: &crate::grant::GrantSpec) -> bool {
    matches!(
        &spec.grantable,
        crate::grant::Grantable::Ability(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::Flash
    ) && spec.zone == Zone::Hand
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
            &[
                "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of",
                "any", "color", "to", "cast", "it",
            ][..],
            &[
                "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of",
                "any", "color", "to", "cast", "that", "spell",
            ][..],
            &[
                "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of",
                "any", "color", "to", "cast", "them",
            ][..],
            &[
                "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of",
                "any", "color", "to", "cast", "those", "spells",
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

    if let Some(rest) = strip_prefix_phrase(
        tokens,
        &["for", "as", "long", "as", "it", "remains", "exiled"],
    ) {
        return (Some(PermissionLifetime::ForAsLongAsExiled), rest);
    }

    (None, tokens)
}

fn permission_lifetime_from_turn_duration(duration: TurnDurationPhrase) -> PermissionLifetime {
    match duration {
        TurnDurationPhrase::ThisTurn => PermissionLifetime::ThisTurn,
        TurnDurationPhrase::UntilEndOfTurn => PermissionLifetime::UntilEndOfTurn,
        TurnDurationPhrase::UntilYourNextTurn | TurnDurationPhrase::UntilYourNextTurnEnd => {
            PermissionLifetime::UntilYourNextTurn
        }
    }
}

fn strip_prefix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<&'a [OwnedLexToken]> {
    LexedClause::new(tokens)
        .strip_prefix_clause(phrase)
        .map(|rest| rest.tokens())
}

fn parse_permission_lead_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(PermissionLead, &'a [OwnedLexToken])> {
    parse_lexed_prefix(tokens, parse_permission_lead_inner)
}

fn parse_tagged_cast_or_play_target_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(TaggedPermissionTarget, &'a [OwnedLexToken])> {
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
    if let Some(stripped) = strip_allow_any_color_for_cast_suffix_tokens(tokens) {
        return parse_permission_tail_tokens(stripped, default_lifetime);
    }
    if tokens.is_empty() {
        return Some((default_lifetime, false));
    }
    if parse_without_paying_mana_cost_tail_tokens(tokens) {
        return Some((default_lifetime, true));
    }

    if strip_prefix_phrase(
        tokens,
        &["for", "as", "long", "as", "it", "remains", "exiled"],
    )
    .is_some_and(|rest| rest.is_empty())
    {
        return Some((PermissionLifetime::ForAsLongAsExiled, false));
    }

    if strip_prefix_phrase(
        tokens,
        &[
            "for", "as", "long", "as", "you", "control", "this", "creature",
        ],
    )
    .is_some_and(|rest| rest.is_empty())
    {
        return Some((PermissionLifetime::ForAsLongAsYouControlSource, false));
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
    if matches!(
        filter_words.as_slice(),
        ["aura", "spells", "with", "enchant", "creature"]
            | ["aura", "cards", "with", "enchant", "creature"]
    ) {
        return Ok(Some(ObjectFilter::default().with_subtype(Subtype::Aura)));
    }
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

fn parse_static_hand_free_cast_grant_spec_from_rest(
    rest_tokens: &[OwnedLexToken],
) -> Result<Option<crate::grant::GrantSpec>, CardTextError> {
    let Some(filter_tokens) =
        strip_cast_from_hand_without_paying_mana_cost_suffix_tokens(rest_tokens)
    else {
        return Ok(None);
    };
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
    Ok(Some(
        crate::grant::GrantSpec::cast_from_hand_without_paying_mana_cost_matching(filter),
    ))
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

    if clause_refs
        == [
            "once",
            "during",
            "each",
            "of",
            "your",
            "turns",
            "you",
            "may",
            "cast",
            "a",
            "permanent",
            "spell",
            "from",
            "your",
            "graveyard",
            "by",
            "sacrificing",
            "a",
            "land",
            "in",
            "addition",
            "to",
            "paying",
            "its",
            "other",
            "costs",
        ]
    {
        let permanent_spell_filter = ObjectFilter {
            card_types: vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Planeswalker,
                CardType::Battle,
            ],
            ..ObjectFilter::default()
        };
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player: PlayerAst::You,
            spec: crate::grant::GrantSpec::new(
                crate::grant::Grantable::once_each_turn_graveyard_cast_from_cards_mana_cost(vec![
                    crate::costs::Cost::sacrifice(ObjectFilter::land().you_control()),
                ]),
                permanent_spell_filter,
                Zone::Graveyard,
            ),
            lifetime: PermissionLifetime::Static,
        }));
    }

    if word_slice_starts_with(
        &clause_refs,
        &[
            "once", "during", "each", "of", "your", "turns", "you", "may", "cast",
        ],
    ) && word_slice_ends_with(&clause_refs, &["from", "your", "graveyard"])
    {
        let Some(from_idx) = find_token_index(tokens, |token| token.is_word("from")) else {
            return Ok(None);
        };
        let Some(subject_start) = token_index_for_word_index(tokens, 9) else {
            return Ok(None);
        };
        let subject_tokens = trim_lexed_commas(&tokens[subject_start..from_idx]);
        if let Some(filter) = parse_permission_subject_filter_tokens_lexed(subject_tokens)? {
            return Ok(Some(PermissionClauseSpec::GrantBySpec {
                player: PlayerAst::You,
                spec: crate::grant::GrantSpec::new(
                    crate::grant::Grantable::once_each_turn_graveyard_cast_from_cards_mana_cost(
                        Vec::new(),
                    ),
                    filter,
                    Zone::Graveyard,
                ),
                lifetime: PermissionLifetime::Static,
            }));
        }
    }

    if clause_refs
        == [
            "once",
            "during",
            "each",
            "of",
            "your",
            "turns",
            "you",
            "may",
            "cast",
            "a",
            "creature",
            "spell",
            "from",
            "your",
            "graveyard",
        ]
    {
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player: PlayerAst::You,
            spec: crate::grant::GrantSpec::new(
                crate::grant::Grantable::once_each_turn_graveyard_cast_from_cards_mana_cost(
                    Vec::new(),
                ),
                ObjectFilter {
                    card_types: vec![CardType::Creature],
                    ..ObjectFilter::default()
                },
                Zone::Graveyard,
            ),
            lifetime: PermissionLifetime::Static,
        }));
    }

    let (prefixed_lifetime, body_tokens) = parse_permission_duration_prefix_tokens(tokens);
    let body_tokens = trim_lexed_commas(body_tokens);
    let Some((lead, rest_tokens)) = parse_permission_lead_tokens(body_tokens) else {
        return Ok(None);
    };
    let player = lead.player;
    let allow_land = lead.allow_land;

    if let Some((target_ref, tagged_tail_tokens)) =
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
                    PermissionLifetime::ForAsLongAsExiled => "for-as-long-as-exiled",
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
                | PermissionLifetime::ForAsLongAsExiled
                | PermissionLifetime::ForAsLongAsYouControlSource
        ) && target_ref.as_copy
        {
            let label = match lifetime {
                PermissionLifetime::UntilYourNextTurn => "until-next-turn",
                PermissionLifetime::ForAsLongAsExiled => "for-as-long-as-exiled",
                PermissionLifetime::ForAsLongAsYouControlSource => {
                    "for-as-long-as-you-control-source"
                }
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
        if lifetime == PermissionLifetime::UntilYourNextTurn && without_paying_mana_cost {
            return Err(CardTextError::ParseError(format!(
                "unsupported until-next-turn play target (clause: '{}')",
                clause_refs.join(" ")
            )));
        }
        if lifetime == PermissionLifetime::ForAsLongAsExiled && without_paying_mana_cost {
            return Err(CardTextError::ParseError(format!(
                "unsupported for-as-long-as-exiled play target with alternative cost (clause: '{}')",
                clause_refs.join(" ")
            )));
        }

        if lifetime == PermissionLifetime::ForAsLongAsYouControlSource && without_paying_mana_cost {
            return Err(CardTextError::ParseError(format!(
                "unsupported for-as-long-as-you-control-source play target with alternative cost (clause: '{}')",
                clause_refs.join(" ")
            )));
        }

        return Ok(Some(PermissionClauseSpec::Tagged {
            tag: target_ref.tag,
            player,
            allow_land,
            as_copy: target_ref.as_copy,
            without_paying_mana_cost,
            lifetime,
        }));
    }

    let rest_words = token_word_refs(rest_tokens);
    if prefixed_lifetime.is_none()
        && player == PlayerAst::You
        && !allow_land
        && rest_words
            == [
                "a",
                "permanent",
                "spell",
                "from",
                "your",
                "graveyard",
                "by",
                "sacrificing",
                "a",
                "land",
                "in",
                "addition",
                "to",
                "paying",
                "its",
                "other",
                "costs",
            ]
        && word_slice_starts_with(
            &clause_refs,
            &["once", "during", "each", "of", "your", "turns"],
        )
    {
        let permanent_spell_filter = ObjectFilter {
            card_types: vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Planeswalker,
                CardType::Battle,
            ],
            ..ObjectFilter::default()
        };
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec: crate::grant::GrantSpec::new(
                crate::grant::Grantable::once_each_turn_graveyard_cast_from_cards_mana_cost(vec![
                    crate::costs::Cost::sacrifice(ObjectFilter::land().you_control()),
                ]),
                permanent_spell_filter,
                Zone::Graveyard,
            ),
            lifetime: PermissionLifetime::Static,
        }));
    }

    if matches!(
        rest_words.as_slice(),
        ["this", "card", "from", "your", "graveyard"]
            | ["this", "spell", "from", "your", "graveyard"]
    ) {
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec: crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                ObjectFilter::source(),
                Zone::Graveyard,
            ),
            lifetime: PermissionLifetime::Static,
        }));
    }

    if matches!(
        rest_words.as_slice(),
        [
            "this",
            "card",
            "from",
            "your",
            "graveyard",
            "as",
            "long",
            "as",
            "youve" | "you've",
            "rolled",
            "a",
            result,
            "this",
            "turn",
            "if",
            "you",
            "cast",
            "it",
            "this",
            "way",
            "and",
            "it",
            "would",
            "be",
            "put",
            "into",
            "your",
            "graveyard",
            "exile",
            "it",
            "instead",
        ] if result.parse::<u32>().is_ok()
    ) {
        let result = rest_words[11].parse::<u32>().map_err(|_| {
            CardTextError::ParseError("invalid die roll graveyard-cast condition".to_string())
        })?;
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec: crate::grant::GrantSpec::new(
                crate::grant::Grantable::graveyard_cast_from_cards_mana_cost_with_condition(
                    crate::static_abilities::ThisSpellCastCondition::ConditionExpr {
                        condition: crate::ConditionExpr::PlayerRolledResultThisTurn {
                            player: crate::target::PlayerFilter::You,
                            result,
                        },
                        display: format!("you've rolled a {result} this turn"),
                    },
                    true,
                ),
                ObjectFilter::source(),
                Zone::Graveyard,
            ),
            lifetime: PermissionLifetime::Static,
        }));
    }

    if allow_land && let Some(after_lands) = strip_prefix_phrase(rest_tokens, &["lands"]) {
        let zone_words = token_word_refs(after_lands);
        if zone_words == ["from", "the", "top", "of", "your", "library"] {
            return Ok(Some(PermissionClauseSpec::GrantBySpec {
                player,
                spec: crate::grant::GrantSpec::new(
                    crate::grant::Grantable::play_from(),
                    ObjectFilter::land(),
                    Zone::Library,
                ),
                lifetime: PermissionLifetime::Static,
            }));
        }
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
        let (zone_grant_tokens, zone_grant_lifetime) =
            if let Some((without_duration, duration)) = parse_turn_duration_suffix(rest_tokens) {
                (
                    trim_lexed_commas(without_duration),
                    Some(permission_lifetime_from_turn_duration(duration)),
                )
            } else {
                (rest_tokens, prefixed_lifetime)
            };
        if let Some(from_idx) = find_token_index(zone_grant_tokens, |token| token.is_word("from")) {
            let zone_words = token_word_refs(&zone_grant_tokens[from_idx..]);
            let zone = if zone_words == ["from", "the", "top", "of", "your", "library"] {
                Some(Zone::Library)
            } else if zone_words == ["from", "your", "graveyard"] {
                Some(Zone::Graveyard)
            } else {
                None
            };
            if let Some(zone) = zone {
                let subject_tokens = trim_lexed_commas(&zone_grant_tokens[..from_idx]);
                let subject_words = token_word_refs(subject_tokens);
                let filter = if subject_words == ["spells"] {
                    ObjectFilter::default()
                } else if let Some(filter) =
                    parse_permission_subject_filter_tokens_lexed(subject_tokens)?
                {
                    filter
                } else {
                    return Ok(None);
                };
                return Ok(Some(PermissionClauseSpec::GrantBySpec {
                    player,
                    spec: crate::grant::GrantSpec::new(
                        crate::grant::Grantable::play_from(),
                        filter,
                        zone,
                    ),
                    lifetime: zone_grant_lifetime.unwrap_or(PermissionLifetime::Static),
                }));
            }
        }

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
            if let Some(tail_lifetime) =
                parse_exact_lexed_prefix(tail_tokens, parse_flash_tail_inner)
            {
                let lifetime = combine_flash_permission_lifetime(prefixed_lifetime, tail_lifetime);
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
                let lifetime = combine_flash_permission_lifetime(prefixed_lifetime, lifetime);
                return Ok(Some(PermissionClauseSpec::GrantBySpec {
                    player,
                    spec: crate::grant::GrantSpec::flash_to_spells_matching(filter),
                    lifetime,
                }));
            }
        }
    }

    if prefixed_lifetime.is_none() && !allow_land {
        if let Some(spec) = parse_static_hand_free_cast_grant_spec_from_rest(rest_tokens)? {
            if clause_is_singular_free_cast_from_hand(&clause_refs) {
                return Ok(None);
            }
            return Ok(Some(PermissionClauseSpec::GrantBySpec {
                player,
                spec,
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

    if word_slice_starts_with(&clause_refs, &["for", "as", "long", "as"])
        && (word_slice_contains_phrase(&clause_refs, &["may", "play"])
            || word_slice_contains_phrase(&clause_refs, &["may", "cast"]))
    {
        if parse_cast_or_play_tagged_clause(tokens)?.is_some() {
            return Ok(None);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported for-as-long-as play/cast permission clause (clause: '{}')",
            clause_refs.join(" ")
        )));
    }

    if word_slice_starts_with(
        &clause_refs,
        &["once", "during", "each", "of", "your", "turns"],
    ) && word_slice_contains_word(&clause_refs, "graveyard")
        && (word_slice_contains_phrase(&clause_refs, &["may", "play"])
            || word_slice_contains_phrase(&clause_refs, &["may", "cast"]))
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
            tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost,
            lifetime: PermissionLifetime::UntilEndOfTurn,
        }) if player == PlayerAst::You => Ok(Some(
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                false,
            ),
        )),
        _ => Ok(None),
    }
}

pub(crate) fn parse_until_your_next_turn_may_play_tagged_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land: true,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: PermissionLifetime::UntilYourNextTurn,
        }) if matches!(player, PlayerAst::You | PlayerAst::Implicit) => Ok(Some(
            EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                tag,
                PlayerAst::You,
                true,
                false,
            ),
        )),
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

    Ok(Some(EffectAst::subject_verb_additional_land_plays(
        PlayerAst::Implicit,
        count,
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_cast_spells_as_though_they_had_flash_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime,
        }) if matches!(
            lifetime,
            PermissionLifetime::ThisTurn
                | PermissionLifetime::UntilEndOfTurn
                | PermissionLifetime::UntilYourNextTurn
        ) && grant_spec_grants_flash_to_hand(&spec) =>
        {
            let duration = match lifetime {
                PermissionLifetime::UntilYourNextTurn => {
                    crate::grant::GrantDuration::UntilYourNextTurnEnd
                }
                _ => crate::grant::GrantDuration::UntilEndOfTurn,
            };
            Ok(Some(EffectAst::subject_verb_grant_by_spec(
                spec, player, duration,
            )))
        }
        _ => Ok(None),
    }
}

fn grant_spec_is_free_cast_from_hand(spec: &crate::grant::GrantSpec) -> bool {
    spec.zone == Zone::Hand
        && matches!(
            &spec.grantable,
            crate::grant::Grantable::AlternativeCast(method)
                if method.mana_cost().is_none() && method.non_mana_costs().is_empty()
        )
}

fn clause_is_singular_free_cast_from_hand(clause_words: &[&str]) -> bool {
    word_slice_contains_phrase(clause_words, &["cast", "a", "spell"])
        || word_slice_contains_phrase(clause_words, &["cast", "one", "spell"])
}

fn parse_cast_with_tagged_mana_value_limit_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    fn parse_simple_spell_type_list_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
        let mut words = token_word_refs(tokens);
        if matches!(words.first().copied(), Some("a" | "an")) {
            words.remove(0);
        }
        if matches!(words.last().copied(), Some("spell" | "spells")) {
            words.pop();
        }
        if words.is_empty() {
            return None;
        }

        let mut card_types = Vec::new();
        for word in words {
            let card_type = match word {
                "or" | "and" => continue,
                "artifact" => CardType::Artifact,
                "battle" => CardType::Battle,
                "creature" => CardType::Creature,
                "enchantment" => CardType::Enchantment,
                "instant" => CardType::Instant,
                "land" => CardType::Land,
                "planeswalker" => CardType::Planeswalker,
                "sorcery" => CardType::Sorcery,
                _ => return None,
            };
            if !card_types.contains(&card_type) {
                card_types.push(card_type);
            }
        }
        if card_types.is_empty() {
            return None;
        }
        Some(ObjectFilter {
            card_types,
            ..ObjectFilter::default()
        })
    }

    let Some((lead, rest_tokens)) = parse_permission_lead_tokens(tokens) else {
        return Ok(None);
    };
    if lead.allow_land {
        return Ok(None);
    }

    let rest_words = token_word_refs(rest_tokens);
    let normalized_words: Vec<String> = rest_words
        .iter()
        .map(|word| {
            word.to_ascii_lowercase()
                .replace(['\'', '’'], "")
                .to_string()
        })
        .collect();
    let Some(from_idx) = normalized_words.iter().position(|word| word == "from") else {
        return Ok(None);
    };
    if from_idx == 0 {
        return Ok(None);
    }

    let normalized_tail = &normalized_words[from_idx..];
    if normalized_tail.len() < 12
        || normalized_tail[0] != "from"
        || normalized_tail[1] != "your"
        || !matches!(normalized_tail[2].as_str(), "graveyard" | "hand")
        || normalized_tail[3] != "with"
        || normalized_tail[4] != "mana"
        || normalized_tail[5] != "value"
    {
        return Ok(None);
    }

    let Some(without_idx) = normalized_tail
        .iter()
        .position(|word| word.as_str() == "without")
    else {
        return Ok(None);
    };
    if without_idx <= 6 {
        return Ok(None);
    }
    if normalized_tail[without_idx..] != ["without", "paying", "its", "mana", "cost"] {
        return Ok(None);
    }

    let Some(comparison_tokens_start) = token_index_for_word_index(rest_tokens, from_idx + 6)
    else {
        return Ok(None);
    };
    let Some(comparison_tokens_end) =
        token_index_for_word_index(rest_tokens, from_idx + without_idx)
    else {
        return Ok(None);
    };
    let comparison_tokens = &rest_tokens[comparison_tokens_start..comparison_tokens_end];
    let Some((operator, rhs_tokens)) = parse_value_comparison_tokens(comparison_tokens) else {
        return Ok(None);
    };

    let Some(filter_end) = token_index_for_word_index(rest_tokens, from_idx) else {
        return Ok(None);
    };
    let filter_tokens = trim_lexed_commas(&rest_tokens[..filter_end]);
    let Some(mut filter) = parse_simple_spell_type_list_filter(filter_tokens)
        .or(parse_permission_subject_filter_tokens_lexed(filter_tokens)?)
    else {
        return Ok(None);
    };
    filter.owner = Some(crate::target::PlayerFilter::You);

    let normalized_tail_refs: Vec<_> = normalized_tail.iter().map(String::as_str).collect();
    let graveyard_uses_tagged_spell_mana_value = matches!(normalized_tail[2].as_str(), "graveyard")
        && word_slice_contains_any_phrase(
            &normalized_tail_refs,
            &[
                &["that", "spell", "s", "mana", "value"],
                &["that", "spells", "mana", "value"],
            ],
        );
    if graveyard_uses_tagged_spell_mana_value {
        filter.mana_value = None;
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::filter::TaggedOpbjectRelation::ManaValueLteTagged,
            });
    } else {
        let Some((rhs_value, used)) = parse_value_from_lexed(rhs_tokens) else {
            return Ok(None);
        };
        if used != rhs_tokens.len() {
            return Ok(None);
        }
        filter.mana_value = Some(match operator {
            ValueComparisonOperator::Equal => {
                crate::filter::Comparison::EqualExpr(Box::new(rhs_value))
            }
            ValueComparisonOperator::NotEqual => {
                crate::filter::Comparison::NotEqualExpr(Box::new(rhs_value))
            }
            ValueComparisonOperator::LessThan => {
                crate::filter::Comparison::LessThanExpr(Box::new(rhs_value))
            }
            ValueComparisonOperator::LessThanOrEqual => {
                crate::filter::Comparison::LessThanOrEqualExpr(Box::new(rhs_value))
            }
            ValueComparisonOperator::GreaterThan => {
                crate::filter::Comparison::GreaterThanExpr(Box::new(rhs_value))
            }
            ValueComparisonOperator::GreaterThanOrEqual => {
                crate::filter::Comparison::GreaterThanOrEqualExpr(Box::new(rhs_value))
            }
        });
    }

    let zone = if normalized_tail[2] == "hand" {
        Zone::Hand
    } else {
        Zone::Graveyard
    };

    Ok(Some(
        EffectAst::may_cast_matching_spell_without_paying_mana_cost(lead.player, filter, zone),
    ))
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

    if let Some((lead, rest_tokens)) = parse_permission_lead_tokens(&trimmed)
        && matches!(lead.player, PlayerAst::Implicit | PlayerAst::You)
        && !lead.allow_land
        && clause_is_singular_free_cast_from_hand(&token_word_refs(&trimmed))
        && let Some(spec) = parse_static_hand_free_cast_grant_spec_from_rest(rest_tokens)?
    {
        return Ok(Some(
            EffectAst::may_cast_matching_spell_without_paying_mana_cost(
                lead.player,
                spec.filter,
                spec.zone,
            ),
        ));
    }

    if let Some(effect) = parse_cast_with_tagged_mana_value_limit_clause(&trimmed)? {
        return Ok(Some(effect));
    }

    if let Some(trailing_if) = split_trailing_if_clause_lexed(&trimmed)
        && let Some(base_effect) = parse_cast_or_play_tagged_clause(trailing_if.leading_tokens)?
    {
        return Ok(Some(EffectAst::Conditional {
            predicate: trailing_if.predicate,
            if_true: vec![base_effect],
            if_false: Vec::new(),
        }));
    }

    let conditional_tagged_permission = parse_permission_lead_tokens(&trimmed)
        .filter(|(lead, _)| lead.player == PlayerAst::Implicit)
        .and_then(|(lead, rest_tokens)| {
            parse_tagged_cast_or_play_target_tokens(rest_tokens).and_then(
                |(target_ref, tail_tokens)| {
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
                                    EffectAst::subject_verb_cast_tagged(
                                        target_ref.tag.clone(),
                                        lead.player,
                                        lead.allow_land,
                                        target_ref.as_copy,
                                        without_paying_mana_cost,
                                        None,
                                    )
                                } else {
                                    EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                                        target_ref.tag.clone(),
                                        PlayerAst::Implicit,
                                        lead.allow_land,
                                        without_paying_mana_cost,
                                        allow_any_color_for_cast,
                                    )
                                };
                                EffectAst::Conditional {
                                    predicate: PredicateAst::ValueComparison {
                                        left: Value::ManaValueOf(Box::new(
                                            crate::target::ChooseSpec::Tagged(
                                                target_ref.tag.clone(),
                                            ),
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
            tag,
            player,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            lifetime: PermissionLifetime::Immediate,
        }) => {
            let cast = EffectAst::subject_verb_cast_tagged(
                tag,
                player,
                allow_land,
                as_copy,
                without_paying_mana_cost,
                None,
            );
            if matches!(player, PlayerAst::Implicit | PlayerAst::You) {
                Ok(Some(cast))
            } else {
                Ok(Some(EffectAst::MayByPlayer {
                    player,
                    effects: vec![cast],
                }))
            }
        }
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost,
            lifetime: PermissionLifetime::ThisTurn | PermissionLifetime::UntilEndOfTurn,
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => Ok(Some(
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                tag,
                PlayerAst::Implicit,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
            ),
        )),
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: PermissionLifetime::UntilYourNextTurn,
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => Ok(Some(
            EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                tag,
                PlayerAst::Implicit,
                allow_land,
                allow_any_color_for_cast,
            ),
        )),
        Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime:
                lifetime @ (PermissionLifetime::ThisTurn
                | PermissionLifetime::UntilEndOfTurn
                | PermissionLifetime::UntilYourNextTurn),
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => {
            let duration = if lifetime == PermissionLifetime::UntilYourNextTurn {
                crate::grant::GrantDuration::UntilYourNextTurnEnd
            } else {
                crate::grant::GrantDuration::UntilEndOfTurn
            };
            Ok(Some(EffectAst::subject_verb_grant_by_spec(
                spec, player, duration,
            )))
        }
        Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: PermissionLifetime::Static,
        }) if (player == PlayerAst::Implicit || player == PlayerAst::You)
            && grant_spec_is_free_cast_from_hand(&spec)
            && clause_is_singular_free_cast_from_hand(&token_word_refs(&trimmed)) =>
        {
            Ok(Some(
                EffectAst::may_cast_matching_spell_without_paying_mana_cost(
                    player,
                    spec.filter,
                    spec.zone,
                ),
            ))
        }
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: PermissionLifetime::ForAsLongAsExiled,
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => Ok(Some(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                tag,
                PlayerAst::Implicit,
                allow_land,
                allow_any_color_for_cast,
            ),
        )),
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: PermissionLifetime::ForAsLongAsYouControlSource,
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => Ok(Some(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_you_control_source(
                tag,
                PlayerAst::Implicit,
                allow_land,
                allow_any_color_for_cast,
            ),
        )),
        _ => Ok(conditional_tagged_permission),
    }
}
