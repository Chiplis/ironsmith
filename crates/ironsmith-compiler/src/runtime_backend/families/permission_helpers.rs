use crate::effect::{Until, Value, ValueComparisonOperator};
use crate::host::{CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, TagKey, TargetAst};
use crate::runtime_backend::GrantedAbilityAst;
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use super::activation_and_restrictions::parse_named_number;
use super::effect_sentences::parse_consult_condition_value;
use super::grammar::filters::{
    parse_object_filter_with_grammar_entrypoint_lexed,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
use super::grammar::primitives as grammar;
use super::grammar::values::parse_value_comparison_tokens;
use super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom};
use super::lexer::{
    LexStream, LexedClause, OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas,
    word_slice_ends_with,
};
use super::object_filters::merge_spell_filters;
use super::token_primitives::{
    TurnDurationPhrase, parse_lexed_prefix, parse_turn_duration_prefix, parse_turn_duration_suffix,
};
use super::util::{
    strip_leading_article_word_refs, strip_leading_token_words_any, token_index_for_word_index,
    trim_commas,
};
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
        filter: Option<ObjectFilter>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaggedPermissionTargetSurface {
    SingleTaggedObject,
    PluralTaggedCards,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnsupportedPermissionShape {
    AdditionalLandEachTurn,
    ForAsLongAsPlayCast,
    OnceEachTurnGraveyard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdditionalLandPlayClause<'a> {
    count_tokens: &'a [OwnedLexToken],
}

fn clause_matches_phrase(clause: LexedClause<'_>, phrase: &[&str]) -> bool {
    LexPattern::new(&[LexPattern::phrase(phrase)]).matches_clause(clause)
}

fn clause_matches_any_phrase(clause: LexedClause<'_>, phrases: &[&[&str]]) -> bool {
    LexPattern::new(&[LexPattern::any_phrase(phrases)]).matches_clause(clause)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FreeCastFromYourZoneRest<'a> {
    filter_tokens: &'a [OwnedLexToken],
    zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManaValueLimitedFreeCastFromYourZoneRest<'a> {
    filter_tokens: &'a [OwnedLexToken],
    comparison_tokens: &'a [OwnedLexToken],
    zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZoneFirstManaValueLimitedFreeCastRest<'a> {
    filter_tokens: &'a [OwnedLexToken],
    comparison_tokens: &'a [OwnedLexToken],
    zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommandZoneFreeCastRest<'a> {
    filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlayFromZoneRest<'a> {
    filter_tokens: &'a [OwnedLexToken],
    zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FlashGrantRest<'a> {
    filter_tokens: &'a [OwnedLexToken],
    lifetime: PermissionLifetime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RevealedTopLibraryPermissionIntro<'a> {
    permission_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PermissionLifetimePrefix<'a> {
    lifetime: PermissionLifetime,
    rest_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OnceEachTurnGraveyardCastRest<'a> {
    subject_tokens: &'a [OwnedLexToken],
    cost_tokens: Option<&'a [OwnedLexToken]>,
    exiles_after_resolution: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OnceEachTurnTopLibrarySharedTypeCast<'a> {
    subject_tokens: &'a [OwnedLexToken],
    source_reference_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceGraveyardCastAdditionalCost<'a> {
    cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceCastPermission {
    zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceGraveyardDieRollCastPermission {
    result: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LandsAndCastFromLibraryPermission<'a> {
    spell_filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConditionalTaggedFreeCastTail<'a> {
    lifetime: PermissionLifetime,
    condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaggedPermissionTail<'a> {
    tail_tokens: &'a [OwnedLexToken],
}

const PERMISSION_FROM_PREPOSITION_PHRASES: &[&[&str]] = &[&["from"]];
const PERMISSION_FROM_PREPOSITION_WORDS: &[&str] = &["from"];
const FLASH_GRANT_TAILS: &[&[&str]] = &[
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
const WITH_MANA_VALUE_PHRASE: &[&str] = &["with", "mana", "value"];
const WITHOUT_PAYING_ITS_MANA_COST_PHRASE: &[&str] = &["without", "paying", "its", "mana", "cost"];
const WITHOUT_PAYING_MANA_COST_PHRASES: &[&[&str]] = &[
    &["without", "paying", "their", "mana", "costs"],
    &["without", "paying", "their", "mana", "cost"],
    WITHOUT_PAYING_ITS_MANA_COST_PHRASE,
];
const COMMAND_ZONE_FREE_CAST_TAIL: &[&str] = &[
    "from", "the", "command", "zone", "without", "paying", "its", "mana", "cost",
];
const REVEALED_TOP_LIBRARY_PERMISSION_PREFIX: &[&str] =
    &["until", "end", "of", "turn", "for", "as", "long", "as"];
const REVEALED_TOP_LIBRARY_REMAINS_TOP_TAIL: &[&str] = &[
    "remains", "on", "top", "of", "your", "library", "play", "with", "the", "top", "card", "of",
    "your", "library", "revealed", "and",
];
const PERMISSION_LIFETIME_PREFIXES: &[&[&str]] = &[
    &["for", "as", "long", "as", "it", "remains", "exiled"],
    &[
        "for", "as", "long", "as", "that", "card", "remains", "exiled",
    ],
    &[
        "for", "as", "long", "as", "those", "cards", "remain", "exiled",
    ],
    &["for", "as", "long", "as", "they", "remain", "exiled"],
    &[
        "for", "as", "long", "as", "you", "control", "this", "creature",
    ],
];
const ALLOW_ANY_COLOR_FOR_CAST_SUFFIXES: &[&[&str]] = &[
    &[
        "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "them",
    ],
    &[
        "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "it",
    ],
    &[
        "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "that", "spell",
    ],
    &[
        "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color", "to", "cast", "it",
    ],
    &[
        "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color", "to", "cast", "that", "spell",
    ],
    &[
        "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color", "to", "cast", "them",
    ],
    &[
        "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color", "to", "cast", "those", "spells",
    ],
];
const ONCE_EACH_TURN_GRAVEYARD_CAST_PREFIX: &[&str] = &[
    "once", "during", "each", "of", "your", "turns", "you", "may", "cast",
];
const GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX: &[&str] =
    &["in", "addition", "to", "paying", "its", "other", "costs"];
const GRAVEYARD_CAST_EXILE_AFTER_RESOLUTION_SUFFIX: &[&str] = &[
    "if",
    "a",
    "spell",
    "cast",
    "this",
    "way",
    "would",
    "be",
    "put",
    "into",
    "your",
    "graveyard",
    "exile",
    "it",
    "instead",
];

fn tagged_permission_target_surface(tokens: &[OwnedLexToken]) -> TaggedPermissionTargetSurface {
    const SINGLE_TAGGED_TARGET_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::object(
            "target",
            LexCaptureKind::OneOfPhrase(&[&["it"], &["that", "card"], &["that", "spell"]]),
        )]);
    const PLURAL_TAGGED_CARDS_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::object(
            "target",
            LexCaptureKind::OneOfPhrase(&[&["those", "cards"]]),
        )]);

    let clause = LexedClause::new(tokens);
    if SINGLE_TAGGED_TARGET_PATTERN.match_clause(clause).is_some() {
        TaggedPermissionTargetSurface::SingleTaggedObject
    } else if PLURAL_TAGGED_CARDS_PATTERN.match_clause(clause).is_some() {
        TaggedPermissionTargetSurface::PluralTaggedCards
    } else {
        TaggedPermissionTargetSurface::Other
    }
}

fn unsupported_permission_shape(tokens: &[OwnedLexToken]) -> Option<UnsupportedPermissionShape> {
    const ADDITIONAL_LAND_EACH_TURN_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::phrase(&[
            "play", "any", "number", "of", "lands", "on", "each", "of", "your", "turns",
        ])]);
    const FOR_AS_LONG_AS_PERMISSION_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["for", "as", "long", "as"]),
        LexPattern::tail("permission", LexCaptureKind::Rest),
    ]);
    const ONCE_EACH_TURN_PERMISSION_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["once", "during", "each", "of", "your", "turns"]),
        LexPattern::tail("permission", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    if ADDITIONAL_LAND_EACH_TURN_PATTERN
        .match_clause(clause)
        .is_some()
    {
        return Some(UnsupportedPermissionShape::AdditionalLandEachTurn);
    }

    if let Some(matched) = FOR_AS_LONG_AS_PERMISSION_PATTERN.match_clause(clause)
        && let Some(permission_clause) =
            matched.capture_clause_by_role(LexCaptureRole::Tail, clause)
        && clause_matches_any_phrase(permission_clause, &[&["may", "play"], &["may", "cast"]])
    {
        return Some(UnsupportedPermissionShape::ForAsLongAsPlayCast);
    }

    if let Some(matched) = ONCE_EACH_TURN_PERMISSION_PATTERN.match_clause(clause)
        && let Some(permission_clause) =
            matched.capture_clause_by_role(LexCaptureRole::Tail, clause)
        && permission_clause.contains_word("graveyard")
        && clause_matches_any_phrase(permission_clause, &[&["may", "play"], &["may", "cast"]])
    {
        return Some(UnsupportedPermissionShape::OnceEachTurnGraveyard);
    }

    None
}

fn parse_additional_land_play_clause(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalLandPlayClause<'_>> {
    const ADDITIONAL_LAND_PLAY_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("play"),
        LexPattern::amount(
            "count",
            LexCaptureKind::UntilAnyPhrase(&[
                &["additional", "land", "this", "turn"],
                &["additional", "lands", "this", "turn"],
            ]),
        ),
        LexPattern::any_phrase(&[
            &["additional", "land", "this", "turn"],
            &["additional", "lands", "this", "turn"],
        ]),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = ADDITIONAL_LAND_PLAY_PATTERN.match_clause(clause)?;
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    Some(AdditionalLandPlayClause {
        count_tokens: count_clause.tokens(),
    })
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
        grammar::phrase(&["its", "owner", "may", "cast"]).value(PermissionLead {
            player: PlayerAst::ItsOwner,
            allow_land: false,
        }),
        grammar::phrase(&["its", "owner", "may", "play"]).value(PermissionLead {
            player: PlayerAst::ItsOwner,
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
            alt((
                grammar::phrase(&["spells", "from", "among", "those", "cards"]),
                grammar::phrase(&["spells", "from", "among", "those", "exiled", "cards"]),
            ))
            .value(TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            }),
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
            alt((
                grammar::phrase(&["those", "cards"]),
                grammar::phrase(&["the", "exiled", "cards"]),
                grammar::phrase(&["exiled", "cards"]),
            ))
            .value(TaggedPermissionTarget {
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
        alt((
            grammar::phrase(&["if", "it's", "a", "spell", "with", "mana", "value"]),
            grammar::phrase(&[
                "if", "it's", "an", "instant", "spell", "with", "mana", "value",
            ]),
            grammar::phrase(&["if", "its", "a", "spell", "with", "mana", "value"]),
            grammar::phrase(&[
                "if", "its", "an", "instant", "spell", "with", "mana", "value",
            ]),
            grammar::phrase(&["if", "it", "is", "a", "spell", "with", "mana", "value"]),
            grammar::phrase(&[
                "if", "it", "is", "an", "instant", "spell", "with", "mana", "value",
            ]),
        )),
        alt((
            grammar::phrase(&["if", "the", "spell's", "mana", "value"]),
            grammar::phrase(&["if", "the", "spells", "mana", "value"]),
            grammar::phrase(&["if", "that", "spell's", "mana", "value"]),
            grammar::phrase(&["if", "that", "spells", "mana", "value"]),
            grammar::phrase(&["if", "its", "mana", "value"]),
        )),
    ))
    .void()
    .parse_next(input)
}

fn parse_exact_lexed_prefix<'a, O>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
) -> Option<O> {
    parse_lexed_prefix(tokens, parser).and_then(|(parsed, rest)| rest.is_empty().then_some(parsed))
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

fn permission_zone_from_location_words(words: &[&str]) -> Option<Zone> {
    match words {
        ["the", "top", "of", "your", "library"] => Some(Zone::Library),
        ["your", "graveyard"] => Some(Zone::Graveyard),
        ["your", "hand"] => Some(Zone::Hand),
        ["exile"] => Some(Zone::Exile),
        _ => None,
    }
}

fn source_zone_prefix_from_clause<'a>(clause: LexedClause<'a>) -> Option<(Zone, LexedClause<'a>)> {
    let atoms = [
        LexPattern::word("this"),
        LexPattern::object("source_kind", LexCaptureKind::OneOf(&["card", "spell"])),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::OneOrMoreWords),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source_kind = matched.capture_clause("source_kind", clause)?;
    let location = matched.capture_clause("location", clause)?;
    let zone = permission_zone_from_location_words(&location.word_refs())?;
    Some((zone, source_kind))
}

fn parse_play_from_zone_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<PlayFromZoneRest<'a>> {
    const PLAY_FROM_ZONE_REST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object(
            "spell_filter",
            LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
        ),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(rest_tokens);
    let matched = PLAY_FROM_ZONE_REST_PATTERN.match_clause(clause)?;
    let filter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let location_clause = matched.capture_clause("location", clause)?;
    let zone = permission_zone_from_location_words(&location_clause.word_refs())?;
    Some(PlayFromZoneRest {
        filter_tokens: trim_lexed_commas(filter_clause.tokens()),
        zone,
    })
}

fn parse_lands_from_top_library_permission_rest_tokens(tokens: &[OwnedLexToken]) -> bool {
    const LANDS_FROM_TOP_LIBRARY_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("lands"),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = LANDS_FROM_TOP_LIBRARY_PATTERN.match_clause(clause) else {
        return false;
    };
    let Some(location_clause) = matched.capture_clause("location", clause) else {
        return false;
    };
    permission_zone_from_location_words(&location_clause.word_refs()) == Some(Zone::Library)
}

fn parse_lands_and_cast_from_top_library_permission_rest_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LandsAndCastFromLibraryPermission<'a>> {
    const LANDS_AND_CAST_FROM_TOP_LIBRARY_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["lands", "and", "cast"]),
        LexPattern::object(
            "spell_filter",
            LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
        ),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = LANDS_AND_CAST_FROM_TOP_LIBRARY_PATTERN.match_clause(clause)?;
    let location_clause = matched.capture_clause("location", clause)?;
    if permission_zone_from_location_words(&location_clause.word_refs()) != Some(Zone::Library) {
        return None;
    }
    let spell_filter = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let spell_filter_tokens = trim_lexed_commas(spell_filter.tokens());
    (!spell_filter_tokens.is_empty()).then_some(LandsAndCastFromLibraryPermission {
        spell_filter_tokens,
    })
}

fn flash_lifetime_from_tail_clause(clause: LexedClause<'_>) -> Option<PermissionLifetime> {
    if clause_matches_any_phrase(
        clause,
        &[
            &["as", "though", "they", "had", "flash"],
            &["as", "though", "they", "have", "flash"],
        ],
    ) {
        return Some(PermissionLifetime::Static);
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &["this", "turn", "as", "though", "they", "had", "flash"],
            &["this", "turn", "as", "though", "they", "have", "flash"],
        ],
    ) {
        return Some(PermissionLifetime::ThisTurn);
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &[
                "until", "end", "of", "turn", "as", "though", "they", "had", "flash",
            ],
            &[
                "until", "the", "end", "of", "turn", "as", "though", "they", "had", "flash",
            ],
        ],
    ) {
        return Some(PermissionLifetime::UntilEndOfTurn);
    }
    None
}

fn parse_flash_grant_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<FlashGrantRest<'a>> {
    const FLASH_GRANT_REST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object(
            "spell_filter",
            LexCaptureKind::UntilAnyPhrase(FLASH_GRANT_TAILS),
        ),
        LexPattern::tail("flash_tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(rest_tokens);
    let matched = FLASH_GRANT_REST_PATTERN.match_clause(clause)?;
    let filter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    let lifetime = flash_lifetime_from_tail_clause(tail_clause)?;
    let filter_tokens = trim_lexed_commas(filter_clause.tokens());
    (!filter_tokens.is_empty()).then_some(FlashGrantRest {
        filter_tokens,
        lifetime,
    })
}

fn parse_revealed_top_library_permission_intro_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<RevealedTopLibraryPermissionIntro<'a>> {
    const REVEALED_TOP_LIBRARY_PERMISSION_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(REVEALED_TOP_LIBRARY_PERMISSION_PREFIX),
        LexPattern::object(
            "referenced_card",
            LexCaptureKind::UntilPhrase(REVEALED_TOP_LIBRARY_REMAINS_TOP_TAIL),
        ),
        LexPattern::phrase(REVEALED_TOP_LIBRARY_REMAINS_TOP_TAIL),
        LexPattern::tail("permission", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = REVEALED_TOP_LIBRARY_PERMISSION_PATTERN.match_clause(clause)?;
    let referenced_card_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !clause_matches_any_phrase(
        referenced_card_clause,
        &[
            &["that", "card"],
            &["that", "revealed", "card"],
            &["the", "revealed", "card"],
        ],
    ) {
        return None;
    }
    let permission_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    let permission_tokens = trim_lexed_commas(permission_clause.tokens());
    (!permission_tokens.is_empty())
        .then_some(RevealedTopLibraryPermissionIntro { permission_tokens })
}

fn permission_lifetime_from_prefix_words(words: &[&str]) -> Option<PermissionLifetime> {
    match words {
        ["for", "as", "long", "as", "it", "remains", "exiled"]
        | [
            "for",
            "as",
            "long",
            "as",
            "that",
            "card",
            "remains",
            "exiled",
        ]
        | [
            "for",
            "as",
            "long",
            "as",
            "those",
            "cards",
            "remain",
            "exiled",
        ] => Some(PermissionLifetime::ForAsLongAsExiled),
        [
            "for",
            "as",
            "long",
            "as",
            "you",
            "control",
            "this",
            "creature",
        ] => Some(PermissionLifetime::ForAsLongAsYouControlSource),
        _ => None,
    }
}

fn parse_permission_lifetime_prefix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<PermissionLifetimePrefix<'a>> {
    const PERMISSION_LIFETIME_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(PERMISSION_LIFETIME_PREFIXES),
        LexPattern::tail("rest", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PERMISSION_LIFETIME_PREFIX_PATTERN.match_clause(clause)?;
    let tail_capture = matched.capture_by_role(LexCaptureRole::Tail)?;
    let words = clause.word_refs();
    let lifetime = permission_lifetime_from_prefix_words(&words[..tail_capture.word_range.start])?;
    let rest_clause = tail_capture.clause(clause)?;
    Some(PermissionLifetimePrefix {
        lifetime,
        rest_tokens: rest_clause.tokens(),
    })
}

fn strip_for_as_long_as_look_at_tagged_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let parsed = parse_permission_lifetime_prefix_tokens(tokens)?;
    if parsed.lifetime != PermissionLifetime::ForAsLongAsExiled {
        return None;
    }
    let prefix_len = tokens.len().checked_sub(parsed.rest_tokens.len())?;
    let rest_tokens = trim_lexed_commas(parsed.rest_tokens);
    let rest_words = token_word_refs(rest_tokens);
    let look_word_count = if rest_words.starts_with(&["you", "may", "look", "at", "them"]) {
        5
    } else if rest_words.starts_with(&["you", "may", "look", "at", "those", "cards"]) {
        6
    } else {
        return None;
    };
    let after_look_idx = token_index_for_word_index(rest_tokens, look_word_count)?;
    let after_look = trim_lexed_commas(strip_leading_token_words_any(
        trim_lexed_commas(&rest_tokens[after_look_idx..]),
        &["and"],
    ));
    if after_look.is_empty() {
        return None;
    }

    let mut permission_tokens = tokens[..prefix_len].to_vec();
    permission_tokens.extend_from_slice(after_look);
    Some(permission_tokens)
}

fn parse_permanent_spells_from_among_tagged_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(TaggedPermissionTarget, &'a [OwnedLexToken], ObjectFilter)> {
    for phrase in [
        &["permanent", "spells", "from", "among", "them"][..],
        &["permanent", "spells", "from", "among", "those", "cards"][..],
    ] {
        let words = token_word_refs(tokens);
        if !words.starts_with(phrase) {
            continue;
        }
        let rest_idx = if words.len() == phrase.len() {
            tokens.len()
        } else {
            token_index_for_word_index(tokens, phrase.len())?
        };
        return Some((
            TaggedPermissionTarget {
                tag: TagKey::from(IT_TAG),
                as_copy: false,
            },
            &tokens[rest_idx..],
            permanent_spell_filter(),
        ));
    }
    None
}

fn parse_once_each_turn_graveyard_cast_rest_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<OnceEachTurnGraveyardCastRest<'a>> {
    const EXILE_AFTER_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::phrase(
        GRAVEYARD_CAST_EXILE_AFTER_RESOLUTION_SUFFIX,
    )];
    const PLAIN_EXILE_AFTER_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(ONCE_EACH_TURN_GRAVEYARD_CAST_PREFIX),
        LexPattern::object(
            "subject",
            LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
        ),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object(
            "location",
            LexCaptureKind::UntilPhrase(GRAVEYARD_CAST_EXILE_AFTER_RESOLUTION_SUFFIX),
        ),
        LexPattern::phrase(GRAVEYARD_CAST_EXILE_AFTER_RESOLUTION_SUFFIX),
    ]);
    const PLAIN_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(ONCE_EACH_TURN_GRAVEYARD_CAST_PREFIX),
        LexPattern::object(
            "subject",
            LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
        ),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::OneOrMoreWords),
    ]);
    const COST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(ONCE_EACH_TURN_GRAVEYARD_CAST_PREFIX),
        LexPattern::object(
            "subject",
            LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
        ),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::UntilPhrase(&["by"])),
        LexPattern::word("by"),
        LexPattern::modifier(
            "cost",
            LexCaptureKind::UntilPhrase(GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX),
        ),
        LexPattern::phrase(GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX),
        LexPattern::optional(EXILE_AFTER_ATOMS),
    ]);

    let clause = LexedClause::new(tokens);
    if let Some(matched) = COST_PATTERN.match_clause(clause) {
        let location_clause = matched.capture_clause("location", clause)?;
        if permission_zone_from_location_words(&location_clause.word_refs())
            != Some(Zone::Graveyard)
        {
            return None;
        }
        let subject_clause = matched.capture_clause("subject", clause)?;
        let cost_clause = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
        let exiles_after_resolution = clause
            .word_refs()
            .ends_with(GRAVEYARD_CAST_EXILE_AFTER_RESOLUTION_SUFFIX);
        let subject_tokens = trim_lexed_commas(subject_clause.tokens());
        let cost_tokens = trim_lexed_commas(cost_clause.tokens());
        if subject_tokens.is_empty() || cost_tokens.is_empty() {
            return None;
        }
        return Some(OnceEachTurnGraveyardCastRest {
            subject_tokens,
            cost_tokens: Some(cost_tokens),
            exiles_after_resolution,
        });
    }

    let (matched, exiles_after_resolution) =
        if let Some(matched) = PLAIN_EXILE_AFTER_PATTERN.match_clause(clause) {
            (matched, true)
        } else {
            (PLAIN_PATTERN.match_clause(clause)?, false)
        };
    let location_clause = matched.capture_clause("location", clause)?;
    if permission_zone_from_location_words(&location_clause.word_refs()) != Some(Zone::Graveyard) {
        return None;
    }
    let subject_clause = matched.capture_clause("subject", clause)?;
    let subject_tokens = trim_lexed_commas(subject_clause.tokens());
    if subject_tokens.is_empty() {
        return None;
    }
    Some(OnceEachTurnGraveyardCastRest {
        subject_tokens,
        cost_tokens: None,
        exiles_after_resolution,
    })
}

fn parse_free_cast_from_your_zone_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<FreeCastFromYourZoneRest<'a>> {
    const FREE_CAST_FROM_YOUR_ZONE_REST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object(
            "spell_filter",
            LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
        ),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object(
            "location",
            LexCaptureKind::UntilAnyPhrase(WITHOUT_PAYING_MANA_COST_PHRASES),
        ),
        LexPattern::tail(
            "without_paying",
            LexCaptureKind::OneOfPhrase(WITHOUT_PAYING_MANA_COST_PHRASES),
        ),
    ]);

    let clause = LexedClause::new(rest_tokens);
    let matched = FREE_CAST_FROM_YOUR_ZONE_REST_PATTERN.match_clause(clause)?;
    let filter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let location_clause = matched.capture_clause("location", clause)?;
    let zone = permission_zone_from_location_words(&location_clause.word_refs())?;
    let filter_tokens = trim_lexed_commas(filter_clause.tokens());
    (!filter_tokens.is_empty()).then_some(FreeCastFromYourZoneRest {
        filter_tokens,
        zone,
    })
}

fn parse_mana_value_limited_free_cast_from_your_zone_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<ManaValueLimitedFreeCastFromYourZoneRest<'a>> {
    const MANA_VALUE_FREE_CAST_FROM_YOUR_ZONE_REST_PATTERN: LexPattern<'static> =
        LexPattern::new(&[
            LexPattern::object(
                "spell_filter",
                LexCaptureKind::UntilPhrase(WITH_MANA_VALUE_PHRASE),
            ),
            LexPattern::phrase(WITH_MANA_VALUE_PHRASE),
            LexPattern::amount(
                "mana_value_comparison",
                LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
            ),
            LexPattern::action(
                "from",
                LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
            ),
            LexPattern::object(
                "location",
                LexCaptureKind::UntilAnyPhrase(WITHOUT_PAYING_MANA_COST_PHRASES),
            ),
            LexPattern::tail(
                "without_paying",
                LexCaptureKind::OneOfPhrase(WITHOUT_PAYING_MANA_COST_PHRASES),
            ),
        ]);

    let clause = LexedClause::new(rest_tokens);
    let matched = MANA_VALUE_FREE_CAST_FROM_YOUR_ZONE_REST_PATTERN.match_clause(clause)?;
    let filter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let comparison_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let location_clause = matched.capture_clause("location", clause)?;
    let zone = permission_zone_from_location_words(&location_clause.word_refs())?;
    let filter_tokens = trim_lexed_commas(filter_clause.tokens());
    let comparison_tokens = trim_lexed_commas(comparison_clause.tokens());
    (!filter_tokens.is_empty() && !comparison_tokens.is_empty()).then_some(
        ManaValueLimitedFreeCastFromYourZoneRest {
            filter_tokens,
            comparison_tokens,
            zone,
        },
    )
}

fn parse_zone_first_mana_value_limited_free_cast_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<ZoneFirstManaValueLimitedFreeCastRest<'a>> {
    let atoms = [
        LexPattern::object(
            "spell_filter",
            LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
        ),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object(
            "location",
            LexCaptureKind::UntilPhrase(WITH_MANA_VALUE_PHRASE),
        ),
        LexPattern::phrase(WITH_MANA_VALUE_PHRASE),
        LexPattern::amount(
            "mana_value_comparison",
            LexCaptureKind::UntilPhrase(WITHOUT_PAYING_ITS_MANA_COST_PHRASE),
        ),
        LexPattern::phrase(WITHOUT_PAYING_ITS_MANA_COST_PHRASE),
    ];
    let clause = LexedClause::new(rest_tokens);
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let filter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let location_clause = matched.capture_clause("location", clause)?;
    let zone = permission_zone_from_location_words(&location_clause.word_refs())?;
    if !matches!(zone, Zone::Hand | Zone::Graveyard) {
        return None;
    }
    let comparison_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let filter_tokens = trim_lexed_commas(filter_clause.tokens());
    let comparison_tokens = trim_lexed_commas(comparison_clause.tokens());
    (!filter_tokens.is_empty() && !comparison_tokens.is_empty()).then_some(
        ZoneFirstManaValueLimitedFreeCastRest {
            filter_tokens,
            comparison_tokens,
            zone,
        },
    )
}

fn parse_command_zone_free_cast_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<CommandZoneFreeCastRest<'a>> {
    const COMMAND_ZONE_FREE_CAST_REST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object(
            "spell_filter",
            LexCaptureKind::UntilPhrase(COMMAND_ZONE_FREE_CAST_TAIL),
        ),
        LexPattern::phrase(COMMAND_ZONE_FREE_CAST_TAIL),
    ]);

    let clause = LexedClause::new(rest_tokens);
    let matched = COMMAND_ZONE_FREE_CAST_REST_PATTERN.match_clause(clause)?;
    let filter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let filter_tokens = trim_lexed_commas(filter_clause.tokens());
    (!filter_tokens.is_empty()).then_some(CommandZoneFreeCastRest { filter_tokens })
}

fn free_cast_filter_mentions_singular_spell(filter_tokens: &[OwnedLexToken]) -> bool {
    filter_tokens_contain_singular_spell_subject(filter_tokens)
        && !filter_tokens_contain_plural_spell_subject(filter_tokens)
}

fn strip_allow_any_color_for_cast_suffix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<&'a [OwnedLexToken]> {
    const ALLOW_ANY_COLOR_SUFFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::modifier(
            "body",
            LexCaptureKind::UntilLastAnyPhrase(ALLOW_ANY_COLOR_FOR_CAST_SUFFIXES),
        ),
        LexPattern::any_phrase(ALLOW_ANY_COLOR_FOR_CAST_SUFFIXES),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = ALLOW_ANY_COLOR_SUFFIX_PATTERN.match_clause(clause)?;
    let body = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    Some(trim_lexed_commas(body.tokens()))
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

    if let Some(parsed) = parse_permission_lifetime_prefix_tokens(tokens)
        && matches!(parsed.lifetime, PermissionLifetime::ForAsLongAsExiled)
    {
        return (Some(parsed.lifetime), parsed.rest_tokens);
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

fn parse_tagged_permission_tail_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> TaggedPermissionTail<'a> {
    const FROM_EXILE_TAIL_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::WordCount(1)),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    if let Some(matched) = FROM_EXILE_TAIL_PATTERN.match_clause(clause) {
        let location = matched.capture_clause("location", clause);
        if location
            .as_ref()
            .and_then(|location| permission_zone_from_location_words(&location.word_refs()))
            == Some(Zone::Exile)
            && let Some(tail) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)
        {
            return TaggedPermissionTail {
                tail_tokens: tail.tokens(),
            };
        }
    }

    TaggedPermissionTail {
        tail_tokens: tokens,
    }
}

fn parse_tagged_permission_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(ValueComparisonOperator, Value)> {
    let (_, after_prefix) = parse_lexed_prefix(
        tokens,
        parse_tagged_permission_mana_value_condition_prefix_inner,
    )?;
    let (operator, operand_tokens) = parse_value_comparison_tokens(after_prefix)?;
    let value = parse_consult_condition_value(operand_tokens)?;
    Some((operator, value))
}

fn parse_conditional_tagged_free_cast_tail_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<ConditionalTaggedFreeCastTail<'a>> {
    const IMMEDIATE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(WITHOUT_PAYING_ITS_MANA_COST_PHRASE),
        LexPattern::condition("condition", LexCaptureKind::Rest),
    ]);
    const THIS_TURN_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["this", "turn"]),
        LexPattern::phrase(WITHOUT_PAYING_ITS_MANA_COST_PHRASE),
        LexPattern::condition("condition", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    if let Some(matched) = IMMEDIATE_PATTERN.match_clause(clause) {
        let condition = matched.capture_clause_by_role(LexCaptureRole::Condition, clause)?;
        return Some(ConditionalTaggedFreeCastTail {
            lifetime: PermissionLifetime::Immediate,
            condition_tokens: condition.tokens(),
        });
    }
    if let Some(matched) = THIS_TURN_PATTERN.match_clause(clause) {
        let condition = matched.capture_clause_by_role(LexCaptureRole::Condition, clause)?;
        return Some(ConditionalTaggedFreeCastTail {
            lifetime: PermissionLifetime::ThisTurn,
            condition_tokens: condition.tokens(),
        });
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

    if let Some(parsed) = parse_permission_lifetime_prefix_tokens(tokens)
        && parsed.rest_tokens.is_empty()
    {
        return Some((parsed.lifetime, false));
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

fn parse_revealed_top_library_permission_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim_lexed_commas(tokens);
    let Some(intro) = parse_revealed_top_library_permission_intro_tokens(tokens) else {
        return Ok(None);
    };
    let permission = match parse_permission_clause_spec(intro.permission_tokens)? {
        Some(PermissionClauseSpec::Tagged {
            mut tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost,
            ..
        }) if matches!(player, PlayerAst::You | PlayerAst::Implicit) => {
            if tag.as_str() == IT_TAG {
                tag = TagKey::from("__last_revealed__");
            }
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_while_on_top_of_library(
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                false,
            )
        }
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported revealed top-library play permission clause (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }
    };

    Ok(Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::subject_verb_grant_abilities_to_target_with_condition(
                TargetAst::Source(None),
                vec![GrantedAbilityAst::StaticAbility(
                    StaticAbility::all_players_look_at_your_top_library_card(),
                )],
                crate::effect::Until::EndOfTurn,
                crate::ConditionExpr::TaggedObjectIsTopOfLibrary {
                    tag: TagKey::from("__last_revealed__"),
                    player: crate::target::PlayerFilter::You,
                },
            ),
            permission,
        ],
    }))
}

fn normalize_permission_subject_filter(mut filter: ObjectFilter) -> ObjectFilter {
    filter.zone = None;
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter
}

fn filter_tokens_contain_spell_subject(tokens: &[OwnedLexToken]) -> bool {
    const SPELL_SUBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
        "spell",
        LexCaptureKind::OneOf(&["spell", "spells"]),
    )]);

    SPELL_SUBJECT_PATTERN
        .find_in_clause(LexedClause::new(tokens))
        .is_some()
}

fn filter_tokens_contain_singular_spell_subject(tokens: &[OwnedLexToken]) -> bool {
    const SINGULAR_SPELL_SUBJECT_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::object(
            "spell",
            LexCaptureKind::OneOf(&["spell"]),
        )]);

    SINGULAR_SPELL_SUBJECT_PATTERN
        .find_in_clause(LexedClause::new(tokens))
        .is_some()
}

fn filter_tokens_contain_plural_spell_subject(tokens: &[OwnedLexToken]) -> bool {
    const PLURAL_SPELL_SUBJECT_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::object(
            "spells",
            LexCaptureKind::OneOf(&["spells"]),
        )]);

    PLURAL_SPELL_SUBJECT_PATTERN
        .find_in_clause(LexedClause::new(tokens))
        .is_some()
}

fn filter_tokens_are_generic_spell_subject(tokens: &[OwnedLexToken]) -> bool {
    const GENERIC_SPELL_SUBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::optional(&[LexPattern::any_word(&["a", "an", "the"])]),
        LexPattern::object("spell", LexCaptureKind::OneOf(&["spell", "spells"])),
    ]);

    GENERIC_SPELL_SUBJECT_PATTERN.matches_clause(LexedClause::new(tokens))
}

fn filter_tokens_are_exact_words(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
    LexPattern::new(&[LexPattern::phrase(words)]).matches_clause(LexedClause::new(tokens))
}

fn filter_tokens_match_any_exact_phrase(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    LexPattern::new(&[LexPattern::any_phrase(phrases)]).matches_clause(LexedClause::new(tokens))
}

fn filter_tokens_start_with_generic_spell_subject(tokens: &[OwnedLexToken]) -> bool {
    const GENERIC_SPELL_SUBJECT_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::optional(&[LexPattern::any_word(&["a", "an", "the"])]),
        LexPattern::object("spell", LexCaptureKind::OneOf(&["spell", "spells"])),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    GENERIC_SPELL_SUBJECT_PREFIX_PATTERN.matches_clause(LexedClause::new(tokens))
}

fn token_slice_is_spell_word(tokens: &[OwnedLexToken]) -> bool {
    const SPELL_WORD_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
        "spell",
        LexCaptureKind::OneOf(&["spell", "spells"]),
    )]);

    SPELL_WORD_PATTERN.matches_clause(LexedClause::new(tokens))
}

fn mark_generic_spell_filter_nonland(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    if filter_tokens_start_with_generic_spell_subject(tokens)
        && !filter.excluded_card_types.contains(&CardType::Land)
    {
        filter.excluded_card_types.push(CardType::Land);
    }
}

fn parse_cast_permission_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if filter_tokens_start_with_generic_spell_subject(tokens) {
        return Ok(Some(ObjectFilter::default()));
    }
    if let Some(filter) = parse_simple_spell_type_list_filter_tokens(tokens) {
        return Ok(Some(filter));
    }
    parse_permission_subject_filter_tokens_lexed(tokens)
}

fn permanent_spell_filter() -> ObjectFilter {
    ObjectFilter {
        card_types: vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Planeswalker,
            CardType::Battle,
        ],
        ..ObjectFilter::default()
    }
}

fn parse_simple_spell_type_list_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut start = 0;
    if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "a" | "an" | "the"))
    {
        start = 1;
    }
    let mut end = tokens.len();
    let has_spell_word = end > 0 && token_slice_is_spell_word(&tokens[end.saturating_sub(1)..end]);
    if has_spell_word {
        end = end.saturating_sub(1);
    }
    if start >= end {
        return has_spell_word.then(ObjectFilter::default);
    }

    let mut card_types = Vec::new();
    let mut saw_or_separator = false;
    let mut saw_separator = false;
    let mut expect_type = true;
    let mut saw_type = false;
    for token in &tokens[start..end] {
        if token.kind == TokenKind::Comma {
            if !saw_type {
                return None;
            }
            saw_separator = true;
            expect_type = true;
            continue;
        }
        let word = token.as_word()?;
        if matches!(word, "or" | "and") {
            if !saw_type {
                return None;
            }
            if word == "or" {
                saw_or_separator = true;
            }
            saw_separator = true;
            expect_type = true;
            continue;
        }
        if !expect_type {
            return None;
        }
        let card_type = match word {
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
        crate::slice_primitives::push_unique(&mut card_types, card_type);
        saw_type = true;
        expect_type = false;
    }
    if !saw_or_separator || !saw_separator || expect_type || card_types.is_empty() {
        return None;
    }
    Some(ObjectFilter {
        card_types,
        ..ObjectFilter::default()
    })
}

fn parse_permission_subject_filter_tokens_lexed(
    filter_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    if filter_tokens_match_any_exact_phrase(
        filter_tokens,
        &[
            &["aura", "spells", "with", "enchant", "creature"],
            &["aura", "cards", "with", "enchant", "creature"],
        ],
    ) {
        return Ok(Some(ObjectFilter::default().with_subtype(Subtype::Aura)));
    }
    if filter_tokens_match_any_exact_phrase(
        filter_tokens,
        &[
            &["permanent", "spell"],
            &["permanent", "spells"],
            &["a", "permanent", "spell"],
            &["a", "permanent", "spells"],
            &["an", "permanent", "spell"],
            &["an", "permanent", "spells"],
            &["the", "permanent", "spell"],
            &["the", "permanent", "spells"],
        ],
    ) {
        return Ok(Some(permanent_spell_filter()));
    }
    if let Some(filter) = parse_simple_spell_type_list_filter_tokens(filter_tokens) {
        return Ok(Some(filter));
    }
    if let Some(filter) = parse_binary_permission_subject_filter_tokens(filter_tokens)? {
        return Ok(Some(filter));
    }

    if let Ok(mut filter) = parse_object_filter_with_grammar_entrypoint_lexed(filter_tokens, false)
    {
        if filter.all_card_types.is_empty()
            && filter.card_types.len() > 1
            && !filter_tokens.iter().any(|token| {
                token.kind == TokenKind::Comma
                    || token
                        .as_word()
                        .is_some_and(|word| matches!(word, "and" | "or"))
            })
        {
            filter.all_card_types = std::mem::take(&mut filter.card_types);
        }
        return Ok(Some(normalize_permission_subject_filter(filter)));
    }

    Ok(None)
}

fn parse_binary_permission_subject_filter_tokens(
    filter_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    for separator in ["and", "or"] {
        let separator_words = [separator];
        let atoms = [
            LexPattern::object("left", LexCaptureKind::UntilPhrase(&separator_words)),
            LexPattern::action("separator", LexCaptureKind::OneOf(&separator_words)),
            LexPattern::object("right", LexCaptureKind::Rest),
        ];
        let clause = LexedClause::new(filter_tokens);
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let Some(left_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause)
        else {
            continue;
        };
        let Some(right_clause) = matched.capture_clause("right", clause) else {
            continue;
        };
        let left_tokens = trim_lexed_commas(left_clause.tokens());
        let right_tokens = trim_lexed_commas(right_clause.tokens());
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

    Ok(None)
}

fn parse_hand_free_cast_grant_spec_from_rest(
    rest_tokens: &[OwnedLexToken],
    allow_singular_spell_filter: bool,
) -> Result<Option<crate::grant::GrantSpec>, CardTextError> {
    let (filter_tokens, mana_value_comparison_tokens) = if let Some(parsed) =
        parse_mana_value_limited_free_cast_from_your_zone_rest_tokens(rest_tokens)
    {
        if parsed.zone != Zone::Hand {
            return Ok(None);
        }
        (parsed.filter_tokens, Some(parsed.comparison_tokens))
    } else if let Some(parsed) =
        parse_zone_first_mana_value_limited_free_cast_rest_tokens(rest_tokens)
    {
        if parsed.zone != Zone::Hand {
            return Ok(None);
        }
        (parsed.filter_tokens, Some(parsed.comparison_tokens))
    } else if let Some(parsed) = parse_free_cast_from_your_zone_rest_tokens(rest_tokens) {
        if parsed.zone != Zone::Hand {
            return Ok(None);
        }
        (parsed.filter_tokens, None)
    } else {
        return Ok(None);
    };
    if !filter_tokens_contain_spell_subject(filter_tokens) {
        return Ok(None);
    }
    if !allow_singular_spell_filter && free_cast_filter_mentions_singular_spell(filter_tokens) {
        return Ok(None);
    }

    let mut filter = ObjectFilter::nonland();
    let parsed_filter = parse_permission_subject_filter_tokens_lexed(filter_tokens)?
        .unwrap_or_else(|| parse_spell_filter_with_grammar_entrypoint_lexed(filter_tokens));
    merge_spell_filters(&mut filter, parsed_filter);
    if let Some(comparison_tokens) = mana_value_comparison_tokens {
        let Some((operator, rhs_tokens)) = parse_value_comparison_tokens(comparison_tokens) else {
            return Ok(None);
        };
        let Some((rhs_value, used)) = parse_value_from_lexed(rhs_tokens) else {
            return Ok(None);
        };
        if used != rhs_tokens.len() {
            return Ok(None);
        }
        filter.mana_value = Some(mana_value_filter_comparison(operator, rhs_value));
    }
    Ok(Some(
        crate::grant::GrantSpec::cast_from_hand_without_paying_mana_cost_matching(filter),
    ))
}

fn parse_static_hand_free_cast_grant_spec_from_rest(
    rest_tokens: &[OwnedLexToken],
) -> Result<Option<crate::grant::GrantSpec>, CardTextError> {
    parse_hand_free_cast_grant_spec_from_rest(rest_tokens, false)
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

fn parse_sacrificing_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::costs::Cost>, CardTextError> {
    const SACRIFICING_COST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("sacrificing"),
        LexPattern::object("filter", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = SACRIFICING_COST_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(filter_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
        return Ok(None);
    };
    let Some(filter) = parse_permission_subject_filter_tokens_lexed(filter_clause.tokens())? else {
        return Ok(None);
    };

    Ok(Some(crate::costs::Cost::sacrifice(filter.you_control())))
}

fn parse_card_type_word(word: &str) -> Option<CardType> {
    match word {
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "creature" | "creatures" => Some(CardType::Creature),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "instant" | "instants" => Some(CardType::Instant),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        _ => None,
    }
}

fn parse_exiling_graveyard_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::costs::Cost>, CardTextError> {
    const CARD_WORD_PHRASES: &[&[&str]] = &[&["card"], &["cards"]];
    const EXILING_GRAVEYARD_COST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("exiling"),
        LexPattern::amount("count", LexCaptureKind::WordCount(1)),
        LexPattern::object(
            "card_types",
            LexCaptureKind::UntilAnyPhrase(CARD_WORD_PHRASES),
        ),
        LexPattern::any_phrase(CARD_WORD_PHRASES),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = EXILING_GRAVEYARD_COST_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(location_clause) = matched.capture_clause("location", clause) else {
        return Ok(None);
    };
    if permission_zone_from_location_words(&location_clause.word_refs()) != Some(Zone::Graveyard) {
        return Ok(None);
    }
    let Some(count_clause) = matched.capture_clause_by_role(LexCaptureRole::Amount, clause) else {
        return Ok(None);
    };
    let count_words = count_clause.word_refs();
    let Some(count_word) = count_words.first() else {
        return Ok(None);
    };
    let Some(count) = parse_named_number(count_word) else {
        return Ok(None);
    };
    let Some(card_types_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause)
    else {
        return Ok(None);
    };

    let mut card_types = Vec::new();
    for word in card_types_clause.word_refs() {
        if matches!(word, "and" | "or" | "and/or") {
            continue;
        }
        let Some(card_type) = parse_card_type_word(word) else {
            return Ok(None);
        };
        if !card_types.contains(&card_type) {
            card_types.push(card_type);
        }
    }

    Ok(Some(crate::costs::Cost::exile_from_graveyard(
        count, card_types,
    )))
}

fn parse_graveyard_cast_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::costs::Cost>, CardTextError> {
    if let Some(cost) = parse_sacrificing_additional_cost_tokens(tokens)? {
        return Ok(Some(cost));
    }
    parse_exiling_graveyard_additional_cost_tokens(tokens)
}

fn parse_source_graveyard_cast_additional_cost_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<SourceGraveyardCastAdditionalCost<'a>> {
    const SOURCE_GRAVEYARD_CAST_ADDITIONAL_COST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("this"),
        LexPattern::object("source_kind", LexCaptureKind::OneOf(&["card", "spell"])),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object("location", LexCaptureKind::UntilPhrase(&["by"])),
        LexPattern::word("by"),
        LexPattern::modifier(
            "cost",
            LexCaptureKind::UntilPhrase(GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX),
        ),
        LexPattern::phrase(GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = SOURCE_GRAVEYARD_CAST_ADDITIONAL_COST_PATTERN.match_clause(clause)?;
    let location_clause = matched.capture_clause("location", clause)?;
    if permission_zone_from_location_words(&location_clause.word_refs()) != Some(Zone::Graveyard) {
        return None;
    }
    let cost_clause = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let cost_tokens = trim_lexed_commas(cost_clause.tokens());
    (!cost_tokens.is_empty()).then_some(SourceGraveyardCastAdditionalCost { cost_tokens })
}

fn parse_source_cast_permission_tokens(tokens: &[OwnedLexToken]) -> Option<SourceCastPermission> {
    let clause = LexedClause::new(tokens);
    let (zone, _source_kind) = source_zone_prefix_from_clause(clause)?;
    matches!(zone, Zone::Graveyard | Zone::Exile).then_some(SourceCastPermission { zone })
}

fn parse_source_graveyard_die_roll_cast_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceGraveyardDieRollCastPermission> {
    const DIE_ROLL_PERMISSION_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["this", "card"]),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object(
            "location",
            LexCaptureKind::UntilPhrase(&["as", "long", "as"]),
        ),
        LexPattern::phrase(&["as", "long", "as"]),
        LexPattern::subject("player", LexCaptureKind::OneOf(&["youve", "you've"])),
        LexPattern::word("rolled"),
        LexPattern::word("a"),
        LexPattern::amount("result", LexCaptureKind::WordCount(1)),
        LexPattern::phrase(&[
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
        ]),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = DIE_ROLL_PERMISSION_PATTERN.match_clause(clause)?;
    let location_clause = matched.capture_clause("location", clause)?;
    if permission_zone_from_location_words(&location_clause.word_refs()) != Some(Zone::Graveyard) {
        return None;
    }
    let result_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let result_words = result_clause.word_refs();
    let result = parse_named_number(result_words.first().copied()?)?;
    Some(SourceGraveyardDieRollCastPermission { result })
}

fn parse_once_each_turn_graveyard_cast_permission(
    tokens: &[OwnedLexToken],
) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some(parsed) = parse_once_each_turn_graveyard_cast_rest_tokens(tokens) else {
        return Ok(None);
    };
    let Some(filter) = parse_permission_subject_filter_tokens_lexed(parsed.subject_tokens)? else {
        return Ok(None);
    };

    let additional_costs = if let Some(cost_tokens) = parsed.cost_tokens {
        let Some(cost) = parse_graveyard_cast_additional_cost_tokens(cost_tokens)? else {
            return Ok(None);
        };
        vec![cost]
    } else {
        Vec::new()
    };

    let grantable =
        crate::grant::Grantable::once_each_turn_graveyard_cast_from_cards_mana_cost_exiles_after_resolution(
            additional_costs,
            parsed.exiles_after_resolution,
        );

    Ok(Some(PermissionClauseSpec::GrantBySpec {
        player: PlayerAst::You,
        spec: crate::grant::GrantSpec::new(grantable, filter, Zone::Graveyard),
        lifetime: PermissionLifetime::Static,
    }))
}

fn parse_once_each_turn_top_library_shared_type_cast_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<OnceEachTurnTopLibrarySharedTypeCast<'a>> {
    const CAST_PREFIX: &[&str] = &["once", "each", "turn", "you", "may", "cast"];
    const SHARES_CARD_TYPE_WITH_PHRASE: &[&str] =
        &["if", "it", "shares", "a", "card", "type", "with"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(CAST_PREFIX),
        LexPattern::object(
            "subject",
            LexCaptureKind::UntilAnyPhrase(PERMISSION_FROM_PREPOSITION_PHRASES),
        ),
        LexPattern::action(
            "from",
            LexCaptureKind::OneOf(PERMISSION_FROM_PREPOSITION_WORDS),
        ),
        LexPattern::object(
            "location",
            LexCaptureKind::UntilPhrase(SHARES_CARD_TYPE_WITH_PHRASE),
        ),
        LexPattern::phrase(SHARES_CARD_TYPE_WITH_PHRASE),
        LexPattern::object("source_reference", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let location = matched.capture_clause("location", clause)?;
    if permission_zone_from_location_words(&location.word_refs()) != Some(Zone::Library) {
        return None;
    }
    let subject = matched.capture_clause("subject", clause)?.trimmed();
    if !clause_matches_any_phrase(subject, &[&["a", "spell"], &["spells"]]) {
        return None;
    }

    let source_reference = matched
        .capture_clause("source_reference", clause)?
        .trimmed();
    if !source_reference_is_card_exiled_with_this(source_reference) {
        return None;
    }

    Some(OnceEachTurnTopLibrarySharedTypeCast {
        subject_tokens: subject.tokens(),
        source_reference_tokens: source_reference.tokens(),
    })
}

fn source_reference_is_card_exiled_with_this(clause: LexedClause<'_>) -> bool {
    const SOURCE_EXILED_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["a", "card", "exiled", "with", "this"]),
        LexPattern::object("source_kind", LexCaptureKind::WordCount(1)),
    ]);

    SOURCE_EXILED_PATTERN.matches_clause(clause)
}

fn parse_once_each_turn_top_library_cast_shares_source_exiled_type_permission(
    tokens: &[OwnedLexToken],
) -> Option<PermissionClauseSpec> {
    let parsed = parse_once_each_turn_top_library_shared_type_cast_tokens(tokens)?;
    let _subject_tokens = parsed.subject_tokens;
    let _source_reference_tokens = parsed.source_reference_tokens;

    let mut filter = ObjectFilter::nonland();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::SharesCardType,
    });

    Some(PermissionClauseSpec::GrantBySpec {
        player: PlayerAst::You,
        spec: crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            filter,
            Zone::Library,
        )
        .with_usage_limit(crate::grant::GrantUsageLimit::OnceEachTurn),
        lifetime: PermissionLifetime::Static,
    })
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

    if let Some(spec) = parse_once_each_turn_graveyard_cast_permission(tokens)? {
        return Ok(Some(spec));
    }
    if let Some(spec) =
        parse_once_each_turn_top_library_cast_shares_source_exiled_type_permission(tokens)
    {
        return Ok(Some(spec));
    }

    let (prefixed_lifetime, body_tokens) = parse_permission_duration_prefix_tokens(tokens);
    let body_tokens = trim_lexed_commas(body_tokens);
    let Some((lead, rest_tokens)) = parse_permission_lead_tokens(body_tokens) else {
        return Ok(None);
    };
    let player = lead.player;
    let allow_land = lead.allow_land;

    if prefixed_lifetime.is_none()
        && !allow_land
        && matches!(player, PlayerAst::Implicit | PlayerAst::You)
        && rest_is_singular_free_cast_from_hand(rest_tokens)
    {
        return Ok(None);
    }

    if let Some((target_ref, tagged_tail_tokens, filter)) =
        parse_permanent_spells_from_among_tagged_tokens(rest_tokens)
            .map(|(target_ref, tail, filter)| (target_ref, tail, Some(filter)))
            .or_else(|| {
                parse_tagged_cast_or_play_target_tokens(rest_tokens)
                    .map(|(target_ref, tail)| (target_ref, tail, None))
            })
    {
        let target_len = rest_tokens.len() - tagged_tail_tokens.len();
        let target_tokens = &rest_tokens[..target_len];
        let tail = parse_tagged_permission_tail_tokens(tagged_tail_tokens);
        let tail_tokens = tail.tail_tokens;

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

        let target_surface = tagged_permission_target_surface(target_tokens);
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
            && !matches!(
                target_surface,
                TaggedPermissionTargetSurface::SingleTaggedObject
                    | TaggedPermissionTargetSurface::PluralTaggedCards
            )
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
            filter,
        }));
    }

    if let Some(parsed) = parse_source_graveyard_cast_additional_cost_tokens(rest_tokens) {
        let Some(cost) = parse_graveyard_cast_additional_cost_tokens(parsed.cost_tokens)? else {
            return Ok(None);
        };
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec: crate::grant::GrantSpec::new(
                crate::grant::Grantable::graveyard_cast_from_cards_mana_cost(vec![cost], false),
                ObjectFilter::source(),
                Zone::Graveyard,
            ),
            lifetime: PermissionLifetime::Static,
        }));
    }

    if let Some(parsed) = parse_source_cast_permission_tokens(rest_tokens) {
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec: crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                ObjectFilter::source(),
                parsed.zone,
            ),
            lifetime: PermissionLifetime::Static,
        }));
    }

    if let Some(parsed) = parse_source_graveyard_die_roll_cast_permission_tokens(rest_tokens) {
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec: crate::grant::GrantSpec::new(
                crate::grant::Grantable::graveyard_cast_from_cards_mana_cost_with_condition(
                    crate::static_abilities::ThisSpellCastCondition::ConditionExpr {
                        condition: crate::ConditionExpr::PlayerRolledResultThisTurn {
                            player: crate::target::PlayerFilter::You,
                            result: parsed.result,
                        },
                        display: format!("you've rolled a {} this turn", parsed.result),
                    },
                    true,
                ),
                ObjectFilter::source(),
                Zone::Graveyard,
            ),
            lifetime: PermissionLifetime::Static,
        }));
    }

    if allow_land && parse_lands_from_top_library_permission_rest_tokens(rest_tokens) {
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

    if allow_land
        && let Some(parsed) =
            parse_lands_and_cast_from_top_library_permission_rest_tokens(rest_tokens)
    {
        let subject_tokens = parsed.spell_filter_tokens;
        let filter = if filter_tokens_are_generic_spell_subject(subject_tokens) {
            ObjectFilter::default()
        } else {
            let Some(spell_filter) = parse_permission_subject_filter_tokens_lexed(subject_tokens)?
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
        if let Some(parsed) = parse_play_from_zone_rest_tokens(zone_grant_tokens) {
            let subject_tokens = parsed.filter_tokens;
            let filter = if filter_tokens_are_generic_spell_subject(subject_tokens) {
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
                    parsed.zone,
                ),
                lifetime: zone_grant_lifetime.unwrap_or(PermissionLifetime::Static),
            }));
        }

        if let Some(parsed) = parse_flash_grant_rest_tokens(rest_tokens) {
            let spec = if filter_tokens_are_exact_words(parsed.filter_tokens, &["spells"]) {
                crate::grant::GrantSpec::flash_to_spells()
            } else if filter_tokens_are_exact_words(
                parsed.filter_tokens,
                &["noncreature", "spells"],
            ) {
                crate::grant::GrantSpec::flash_to_noncreature_spells()
            } else if let Some(filter) =
                parse_permission_subject_filter_tokens_lexed(parsed.filter_tokens)?
            {
                crate::grant::GrantSpec::flash_to_spells_matching(filter)
            } else {
                return Ok(None);
            };
            let lifetime = combine_flash_permission_lifetime(prefixed_lifetime, parsed.lifetime);
            return Ok(Some(PermissionClauseSpec::GrantBySpec {
                player,
                spec,
                lifetime,
            }));
        }
    }

    if prefixed_lifetime.is_none() && !allow_land {
        if let Some(spec) = parse_static_hand_free_cast_grant_spec_from_rest(rest_tokens)? {
            if rest_is_singular_free_cast_from_hand(rest_tokens) {
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

    match unsupported_permission_shape(tokens) {
        Some(UnsupportedPermissionShape::AdditionalLandEachTurn) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported additional-land-play permission clause (clause: '{}')",
                clause_refs.join(" ")
            )));
        }
        Some(UnsupportedPermissionShape::ForAsLongAsPlayCast) => {
            if parse_cast_or_play_tagged_clause(tokens)?.is_some() {
                return Ok(None);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported for-as-long-as play/cast permission clause (clause: '{}')",
                clause_refs.join(" ")
            )));
        }
        Some(UnsupportedPermissionShape::OnceEachTurnGraveyard) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported once-per-turn graveyard play/cast permission clause (clause: '{}')",
                clause_refs.join(" ")
            )));
        }
        None => {}
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
            ..
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
            ..
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
    let Some(parsed) = parse_additional_land_play_clause(tokens) else {
        return Ok(None);
    };
    let count_tokens = parsed.count_tokens;
    let count_words = token_word_refs(count_tokens);
    let (count, used) = if matches!(count_words.as_slice(), ["a"] | ["an"]) {
        (Value::Fixed(1), 1usize)
    } else {
        let Some((value, used)) = parse_value_from_lexed(count_tokens) else {
            return Ok(None);
        };
        (value, used)
    };

    if count_words.len() != used {
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

fn rest_is_singular_free_cast_from_hand(rest_tokens: &[OwnedLexToken]) -> bool {
    if let Some(parsed) = parse_mana_value_limited_free_cast_from_your_zone_rest_tokens(rest_tokens)
    {
        return parsed.zone == Zone::Hand
            && free_cast_filter_mentions_singular_spell(parsed.filter_tokens);
    }
    if let Some(parsed) = parse_zone_first_mana_value_limited_free_cast_rest_tokens(rest_tokens) {
        return parsed.zone == Zone::Hand
            && free_cast_filter_mentions_singular_spell(parsed.filter_tokens);
    }
    if let Some(parsed) = parse_free_cast_from_your_zone_rest_tokens(rest_tokens) {
        return parsed.zone == Zone::Hand
            && free_cast_filter_mentions_singular_spell(parsed.filter_tokens);
    }
    false
}

fn clause_is_singular_free_cast_from_hand(tokens: &[OwnedLexToken]) -> bool {
    let Some((lead, rest_tokens)) = parse_permission_lead_tokens(tokens) else {
        return false;
    };
    !lead.allow_land
        && matches!(lead.player, PlayerAst::Implicit | PlayerAst::You)
        && rest_is_singular_free_cast_from_hand(rest_tokens)
}

fn mana_value_filter_comparison(
    operator: ValueComparisonOperator,
    rhs_value: Value,
) -> crate::filter::Comparison {
    match (operator, rhs_value) {
        (ValueComparisonOperator::Equal, Value::Fixed(value)) => {
            crate::filter::Comparison::Equal(value)
        }
        (ValueComparisonOperator::NotEqual, Value::Fixed(value)) => {
            crate::filter::Comparison::NotEqual(value)
        }
        (ValueComparisonOperator::LessThan, Value::Fixed(value)) => {
            crate::filter::Comparison::LessThan(value)
        }
        (ValueComparisonOperator::LessThanOrEqual, Value::Fixed(value)) => {
            crate::filter::Comparison::LessThanOrEqual(value)
        }
        (ValueComparisonOperator::GreaterThan, Value::Fixed(value)) => {
            crate::filter::Comparison::GreaterThan(value)
        }
        (ValueComparisonOperator::GreaterThanOrEqual, Value::Fixed(value)) => {
            crate::filter::Comparison::GreaterThanOrEqual(value)
        }
        (ValueComparisonOperator::Equal, value) => {
            crate::filter::Comparison::EqualExpr(Box::new(value))
        }
        (ValueComparisonOperator::NotEqual, value) => {
            crate::filter::Comparison::NotEqualExpr(Box::new(value))
        }
        (ValueComparisonOperator::LessThan, value) => {
            crate::filter::Comparison::LessThanExpr(Box::new(value))
        }
        (ValueComparisonOperator::LessThanOrEqual, value) => {
            crate::filter::Comparison::LessThanOrEqualExpr(Box::new(value))
        }
        (ValueComparisonOperator::GreaterThan, value) => {
            crate::filter::Comparison::GreaterThanExpr(Box::new(value))
        }
        (ValueComparisonOperator::GreaterThanOrEqual, value) => {
            crate::filter::Comparison::GreaterThanOrEqualExpr(Box::new(value))
        }
    }
}

fn value_is_tagged_it_mana_value(value: &Value) -> bool {
    matches!(
        value,
        Value::ManaValueOf(spec)
            if matches!(
                spec.as_ref(),
                crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG
            )
    )
}

fn parse_cast_with_tagged_mana_value_limit_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    fn parse_cast_with_prefixed_mana_value_limit(
        rest_tokens: &[OwnedLexToken],
        player: PlayerAst,
        parse_simple_spell_type_list_filter: fn(&[OwnedLexToken]) -> Option<ObjectFilter>,
    ) -> Result<Option<EffectAst>, CardTextError> {
        let Some(parsed) =
            parse_mana_value_limited_free_cast_from_your_zone_rest_tokens(rest_tokens)
        else {
            return Ok(None);
        };
        let Some((operator, rhs_tokens)) = parse_value_comparison_tokens(parsed.comparison_tokens)
        else {
            return Ok(None);
        };

        let filter_tokens = parsed.filter_tokens;
        let Some(mut filter) = parse_simple_spell_type_list_filter(filter_tokens)
            .or(parse_cast_permission_filter_tokens(filter_tokens)?)
        else {
            return Ok(None);
        };
        mark_generic_spell_filter_nonland(&mut filter, filter_tokens);
        filter.owner = Some(crate::target::PlayerFilter::You);

        let Some((rhs_value, used)) = parse_value_from_lexed(rhs_tokens) else {
            return Ok(None);
        };
        if used != rhs_tokens.len() {
            return Ok(None);
        }
        if let (ValueComparisonOperator::Equal, Value::CountersOnSource(counter_type)) =
            (&operator, &rhs_value)
        {
            filter.mana_value_eq_counters_on_source = Some(*counter_type);
        } else {
            filter.mana_value = Some(mana_value_filter_comparison(operator, rhs_value));
        }

        Ok(Some(
            EffectAst::may_cast_matching_spell_without_paying_mana_cost(
                player,
                filter,
                parsed.zone,
            ),
        ))
    }

    let Some((lead, rest_tokens)) = parse_permission_lead_tokens(tokens) else {
        return Ok(None);
    };
    if lead.allow_land {
        return Ok(None);
    }

    if let Some(parsed) = parse_command_zone_free_cast_rest_tokens(rest_tokens) {
        if filter_tokens_are_exact_words(parsed.filter_tokens, &["your", "commander"]) {
            return Ok(Some(
                EffectAst::may_cast_matching_spell_without_paying_mana_cost(
                    lead.player,
                    ObjectFilter::default()
                        .commander()
                        .owned_by(crate::target::PlayerFilter::You),
                    Zone::Command,
                ),
            ));
        }
    }

    if let Some(effect) = parse_cast_with_prefixed_mana_value_limit(
        rest_tokens,
        lead.player,
        parse_simple_spell_type_list_filter_tokens,
    )? {
        return Ok(Some(effect));
    }

    if let Some(parsed) = parse_free_cast_from_your_zone_rest_tokens(rest_tokens) {
        let filter_tokens = parsed.filter_tokens;
        let Some(mut filter) = parse_cast_permission_filter_tokens(filter_tokens)? else {
            return Ok(None);
        };
        mark_generic_spell_filter_nonland(&mut filter, filter_tokens);
        filter.owner = Some(crate::target::PlayerFilter::You);
        if lead.player == PlayerAst::Implicit
            && parsed.zone == Zone::Graveyard
            && !filter_tokens_contain_spell_subject(filter_tokens)
        {
            return Ok(None);
        }
        return Ok(Some(
            EffectAst::may_cast_matching_spell_without_paying_mana_cost(
                lead.player,
                filter,
                parsed.zone,
            ),
        ));
    }

    let Some(parsed) = parse_zone_first_mana_value_limited_free_cast_rest_tokens(rest_tokens)
    else {
        return Ok(None);
    };
    let Some((operator, rhs_tokens)) = parse_value_comparison_tokens(parsed.comparison_tokens)
    else {
        return Ok(None);
    };

    let filter_tokens = parsed.filter_tokens;
    let Some(mut filter) = parse_cast_permission_filter_tokens(filter_tokens)? else {
        return Ok(None);
    };
    mark_generic_spell_filter_nonland(&mut filter, filter_tokens);
    filter.owner = Some(crate::target::PlayerFilter::You);

    let Some((rhs_value, used)) = parse_value_from_lexed(rhs_tokens) else {
        return Ok(None);
    };
    if used != rhs_tokens.len() {
        return Ok(None);
    }

    let graveyard_uses_tagged_spell_mana_value =
        parsed.zone == Zone::Graveyard && value_is_tagged_it_mana_value(&rhs_value);
    if graveyard_uses_tagged_spell_mana_value {
        filter.mana_value = None;
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::filter::TaggedOpbjectRelation::ManaValueLteTagged,
            });
    } else {
        if let (ValueComparisonOperator::Equal, Value::CountersOnSource(counter_type)) =
            (&operator, &rhs_value)
        {
            filter.mana_value_eq_counters_on_source = Some(*counter_type);
        } else {
            filter.mana_value = Some(mana_value_filter_comparison(operator, rhs_value));
        }
    }

    Ok(Some(
        EffectAst::may_cast_matching_spell_without_paying_mana_cost(
            lead.player,
            filter,
            parsed.zone,
        ),
    ))
}

pub(crate) fn parse_cast_or_play_tagged_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let trimmed_tokens = trim_commas(tokens);
    let mut trimmed = strip_leading_token_words_any(&trimmed_tokens, &["then", "and"]).to_vec();

    if let Some(effect) = parse_revealed_top_library_permission_clause(&trimmed)? {
        return Ok(Some(effect));
    }

    if let Some(permission_tokens) = strip_for_as_long_as_look_at_tagged_prefix_tokens(&trimmed)
        && let Some(permission) = parse_cast_or_play_tagged_clause(&permission_tokens)?
    {
        let mut look_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        look_filter.zone = Some(Zone::Exile);
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_look_at_objects(PlayerAst::You, look_filter),
                permission,
            ],
        }));
    }

    let mut allow_any_color_for_cast = false;
    if let Some(stripped) = strip_allow_any_color_for_cast_suffix_tokens(&trimmed) {
        allow_any_color_for_cast = true;
        trimmed.truncate(stripped.len());
    }

    if let Some(effect) = parse_cast_with_tagged_mana_value_limit_clause(&trimmed)? {
        return Ok(Some(effect));
    }

    if let Some((lead, rest_tokens)) = parse_permission_lead_tokens(&trimmed)
        && matches!(lead.player, PlayerAst::Implicit | PlayerAst::You)
        && !lead.allow_land
        && rest_is_singular_free_cast_from_hand(rest_tokens)
        && let Some(spec) = parse_hand_free_cast_grant_spec_from_rest(rest_tokens, true)?
    {
        return Ok(Some(
            EffectAst::may_cast_matching_spell_without_paying_mana_cost(
                lead.player,
                spec.filter,
                spec.zone,
            ),
        ));
    }

    let conditional_tagged_permission = parse_permission_lead_tokens(&trimmed)
        .filter(|(lead, _)| lead.player == PlayerAst::Implicit)
        .and_then(|(lead, rest_tokens)| {
            parse_tagged_cast_or_play_target_tokens(rest_tokens).and_then(
                |(target_ref, tail_tokens)| {
                    let tail = parse_conditional_tagged_free_cast_tail_tokens(tail_tokens)?;
                    let (operator, right) =
                        parse_tagged_permission_mana_value_condition_tokens(tail.condition_tokens)?;
                    let inner = if tail.lifetime == PermissionLifetime::Immediate {
                        EffectAst::subject_verb_cast_tagged(
                            target_ref.tag.clone(),
                            lead.player,
                            lead.allow_land,
                            target_ref.as_copy,
                            true,
                            None,
                        )
                    } else {
                        EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                            target_ref.tag.clone(),
                            PlayerAst::Implicit,
                            lead.allow_land,
                            true,
                            allow_any_color_for_cast,
                        )
                    };
                    Some(EffectAst::Conditional {
                        predicate: PredicateAst::ValueComparison {
                            left: Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(
                                target_ref.tag.clone(),
                            ))),
                            operator,
                            right,
                        },
                        if_true: vec![inner],
                        if_false: Vec::new(),
                    })
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
            ..
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
            ..
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
            ..
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
            && clause_is_singular_free_cast_from_hand(&trimmed) =>
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
            without_paying_mana_cost,
            lifetime: PermissionLifetime::ForAsLongAsExiled,
            filter,
        }) if matches!(
            player,
            PlayerAst::Implicit | PlayerAst::You | PlayerAst::ItsOwner
        ) =>
        {
            Ok(Some(
                EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                    tag,
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    filter,
                ),
            ))
        }
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: PermissionLifetime::ForAsLongAsYouControlSource,
            ..
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
