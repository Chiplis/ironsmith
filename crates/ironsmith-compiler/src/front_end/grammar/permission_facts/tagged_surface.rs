//! Typed surface facts for tagged cast and play permission clauses.
//!
//! This module deliberately stops at grammar-owned facts.  The permission
//! family remains responsible for turning these facts into grants and effects.

use crate::effect::{Value, ValueComparisonOperator};
use crate::front_end::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};

use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::{effects, leaf, primitives, values};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionActor {
    You,
    AnyPlayer,
    ItsOwner,
    Implicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionVerb {
    Cast,
    Play,
}

impl PermissionVerb {
    pub(crate) fn allows_land(self) -> bool {
        self == Self::Play
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PermissionLeadFact<'a> {
    pub(crate) actor: PermissionActor,
    pub(crate) verb: PermissionVerb,
    pub(crate) rest_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaggedPermissionReference {
    LastTagged,
    SourceExiled,
    LastRevealed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaggedPermissionTargetSurface {
    It,
    ThatCard,
    ThatSpell,
    Them,
    ThoseCards,
    SpellsFromAmongThoseCards,
    SpellsFromAmongThoseExiledCards,
    SpellFromAmongSourceExiledCards,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaggedPermissionTargetFact<'a> {
    pub(crate) reference: TaggedPermissionReference,
    pub(crate) as_copy: bool,
    pub(crate) surface: TaggedPermissionTargetSurface,
    /// Total uses shared by the tagged collection. This preserves deferred
    /// choices such as "play one of those cards" without selecting a card
    /// when the permission is created.
    pub(crate) max_plays: Option<u32>,
    pub(crate) rest_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UntilSourceExilesAnotherPermissionFact<'a> {
    pub(crate) actor: PermissionActor,
    pub(crate) verb: PermissionVerb,
    pub(crate) reference: TaggedPermissionReference,
    pub(crate) target_surface: TaggedPermissionTargetSurface,
    pub(crate) source_reference_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionLifetimeFact {
    Immediate,
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
    UntilYourNextEndStep,
    ForAsLongAsExiled,
    ForAsLongAsYouControlSource,
    Static,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PermissionLifetimePrefixFact<'a> {
    pub(crate) lifetime: PermissionLifetimeFact,
    pub(crate) rest_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManaSpendCastReference {
    It,
    ThatSpell,
    Them,
    ThoseSpells,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AllowAnyColorForCastSuffixFact<'a> {
    pub(crate) body_tokens: &'a [OwnedLexToken],
    pub(crate) mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
    pub(crate) reference: ManaSpendCastReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PermissionTailFact {
    pub(crate) lifetime: PermissionLifetimeFact,
    pub(crate) without_paying_mana_cost: bool,
    pub(crate) allow_any_color_for_cast: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaggedPermissionTailFact<'a> {
    pub(crate) from_exile: bool,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConditionalTaggedFreeCastTailFact<'a> {
    pub(crate) lifetime: PermissionLifetimeFact,
    pub(crate) condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaggedManaValueConditionFact {
    pub(crate) operator: ValueComparisonOperator,
    pub(crate) right: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnsupportedPermissionFact {
    AdditionalLandEachTurn,
    ForAsLongAsPlayCast,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AdditionalLandPlayFact<'a> {
    pub(crate) count: Value,
    pub(crate) count_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RevealedTopLibraryPermissionFact<'a> {
    pub(crate) permission_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaggedLookReference {
    It,
    ThatCard,
    Them,
    ThoseCards,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ForAsLongAsLookAtTaggedFact<'a> {
    pub(crate) lifetime: PermissionLifetimeFact,
    pub(crate) reference: TaggedLookReference,
    pub(crate) permission_tokens: &'a [OwnedLexToken],
}

/// A spell-subject-qualified reference to the collection tagged by a preceding
/// action, such as "noncreature spells from among those cards".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellsFromTaggedFact<'a> {
    pub(crate) reference: TaggedPermissionReference,
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) surface: TaggedPermissionTargetSurface,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_permission_lead_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PermissionLeadFact<'_>> {
    let ((actor, verb), rest_tokens) =
        primitives::parse_prefix(tokens, parse_permission_lead_lexed)?;
    Some(PermissionLeadFact {
        actor,
        verb,
        rest_tokens,
    })
}

pub(crate) fn parse_tagged_permission_target_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TaggedPermissionTargetFact<'_>> {
    let ((reference, as_copy, surface, max_plays), rest_tokens) =
        primitives::parse_prefix(tokens, parse_tagged_permission_target_lexed)?;
    Some(TaggedPermissionTargetFact {
        reference,
        as_copy,
        surface,
        max_plays,
        rest_tokens,
    })
}

pub(crate) fn parse_tagged_permission_target_surface_tokens(
    tokens: &[OwnedLexToken],
) -> TaggedPermissionTargetSurface {
    primitives::parse_all(
        tokens,
        parse_tagged_permission_target_surface_lexed,
        "tagged-permission-target-surface",
    )
    .unwrap_or(TaggedPermissionTargetSurface::Other)
}

pub(crate) fn parse_until_source_exiles_another_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<UntilSourceExilesAnotherPermissionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_until_source_exiles_another_permission_lexed,
        "until-source-exiles-another-permission",
    )
    .ok()
}

pub(crate) fn parse_permission_lifetime_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PermissionLifetimePrefixFact<'_>> {
    let (lifetime, rest_tokens) =
        primitives::parse_prefix(tokens, parse_permission_lifetime_lexed)?;
    Some(PermissionLifetimePrefixFact {
        lifetime,
        rest_tokens,
    })
}

pub(crate) fn parse_permission_duration_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PermissionLifetimePrefixFact<'_>> {
    if let Some((lifetime, rest_tokens)) =
        primitives::parse_prefix(tokens, parse_permission_turn_duration_lexed)
    {
        return Some(PermissionLifetimePrefixFact {
            lifetime,
            rest_tokens,
        });
    }
    let parsed = parse_permission_lifetime_prefix_tokens(tokens)?;
    (parsed.lifetime == PermissionLifetimeFact::ForAsLongAsExiled).then_some(parsed)
}

pub(crate) fn parse_allow_any_color_for_cast_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AllowAnyColorForCastSuffixFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_allow_any_color_for_cast_suffix_lexed,
        "allow-any-color-for-cast-suffix",
    )
    .ok()
}

pub(crate) fn parse_permission_tail_tokens(
    tokens: &[OwnedLexToken],
    default_lifetime: PermissionLifetimeFact,
) -> Option<PermissionTailFact> {
    let (body_tokens, allow_any_color_for_cast) =
        if let Some(parsed) = parse_allow_any_color_for_cast_suffix_tokens(tokens) {
            (trim_lexed_commas(parsed.body_tokens), true)
        } else {
            (tokens, false)
        };
    let (lifetime, without_paying_mana_cost) = primitives::parse_all(
        body_tokens,
        |input: &mut LexStream<'_>| parse_permission_tail_lexed(input, default_lifetime),
        "permission-tail",
    )
    .ok()?;
    Some(PermissionTailFact {
        lifetime,
        without_paying_mana_cost,
        allow_any_color_for_cast,
    })
}

pub(crate) fn parse_tagged_permission_tail_tokens(
    tokens: &[OwnedLexToken],
) -> TaggedPermissionTailFact<'_> {
    if let Some(((), tail_tokens)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["from", "exile"]))
    {
        return TaggedPermissionTailFact {
            from_exile: true,
            tail_tokens,
        };
    }
    TaggedPermissionTailFact {
        from_exile: false,
        tail_tokens: tokens,
    }
}

pub(crate) fn parse_conditional_tagged_free_cast_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalTaggedFreeCastTailFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_conditional_tagged_free_cast_tail_lexed,
        "conditional-tagged-free-cast-tail",
    )
    .ok()
}

pub(crate) fn parse_tagged_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TaggedManaValueConditionFact> {
    let (_, comparison_tokens) =
        primitives::parse_prefix(tokens, parse_tagged_mana_value_condition_intro_lexed)?;
    let (operator, right_tokens) = values::parse_value_comparison_tokens(comparison_tokens)?;
    let right = effects::parse_consult_condition_value_shape(right_tokens)?;
    Some(TaggedManaValueConditionFact { operator, right })
}

pub(crate) fn parse_additional_land_play_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalLandPlayFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_additional_land_play_lexed,
        "additional-land-play",
    )
    .ok()
}

pub(crate) fn parse_unsupported_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<UnsupportedPermissionFact> {
    primitives::parse_all(
        tokens,
        parse_unsupported_permission_lexed,
        "unsupported-permission",
    )
    .ok()
}

pub(crate) fn parse_revealed_top_library_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RevealedTopLibraryPermissionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_revealed_top_library_permission_lexed,
        "revealed-top-library-permission",
    )
    .ok()
}

pub(crate) fn parse_for_as_long_as_look_at_tagged_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ForAsLongAsLookAtTaggedFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_for_as_long_as_look_at_tagged_lexed,
        "for-as-long-as-look-at-tagged",
    )
    .ok()
}

pub(crate) fn parse_spells_from_tagged_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SpellsFromTaggedFact<'_>> {
    let (scope_start, surface, tail_tokens) = primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["from", "among", "those", "cards"])
                .value(TaggedPermissionTargetSurface::SpellsFromAmongThoseCards),
            primitives::phrase(&["from", "among", "those", "exiled", "cards"])
                .value(TaggedPermissionTargetSurface::SpellsFromAmongThoseExiledCards),
            primitives::phrase(&["from", "among", "them"])
                .value(TaggedPermissionTargetSurface::Other),
        ))
    })?;
    let subject_tokens = trim_lexed_commas(&tokens[..scope_start]);
    if subject_tokens.is_empty()
        || primitives::find_prefix(subject_tokens, || {
            alt((primitives::kw("spell"), primitives::kw("spells"))).void()
        })
        .is_none()
    {
        return None;
    }

    Some(SpellsFromTaggedFact {
        reference: TaggedPermissionReference::LastTagged,
        subject_tokens,
        surface,
        tail_tokens,
    })
}

fn parse_permission_lead_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(PermissionActor, PermissionVerb)> {
    alt((
        (
            alt((
                primitives::kw("you").value(PermissionActor::You),
                primitives::phrase(&["any", "player"]).value(PermissionActor::AnyPlayer),
                primitives::phrase(&["its", "owner"]).value(PermissionActor::ItsOwner),
            )),
            primitives::kw("may"),
            parse_permission_verb_lexed,
        )
            .map(|(actor, _, verb)| (actor, verb)),
        parse_permission_verb_lexed.map(|verb| (PermissionActor::Implicit, verb)),
    ))
    .parse_next(input)
}

fn parse_permission_verb_lexed<'a>(input: &mut LexStream<'a>) -> WResult<PermissionVerb> {
    alt((
        primitives::kw("cast").value(PermissionVerb::Cast),
        primitives::kw("play").value(PermissionVerb::Play),
    ))
    .parse_next(input)
}

fn parse_tagged_permission_target_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(
    TaggedPermissionReference,
    bool,
    TaggedPermissionTargetSurface,
    Option<u32>,
)> {
    alt((
        alt((
            primitives::kw("it").value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::It,
                None,
            )),
            primitives::phrase(&["that", "card"]).value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::ThatCard,
                None,
            )),
            primitives::phrase(&["that", "spell"]).value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::ThatSpell,
                None,
            )),
            primitives::kw("them").value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::Them,
                None,
            )),
            primitives::phrase(&["those", "cards"]).value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::ThoseCards,
                None,
            )),
            primitives::phrase(&["spells", "from", "among", "those", "exiled", "cards"]).value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::SpellsFromAmongThoseExiledCards,
                None,
            )),
            primitives::phrase(&["spells", "from", "among", "those", "cards"]).value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::SpellsFromAmongThoseCards,
                None,
            )),
            primitives::any_phrase(&[
                &["one", "of", "those", "cards"],
                &["one", "of", "those", "card"],
                &["one", "of", "them"],
            ])
            .value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::Other,
                Some(1),
            )),
            primitives::any_phrase(&[
                &["spells", "from", "among", "them"],
                &["them"],
                &["the", "exiled", "cards"],
                &["exiled", "cards"],
                &["those", "spells"],
                &["that", "exiled", "card"],
                &["the", "card"],
                &["the", "cards"],
            ])
            .value((
                TaggedPermissionReference::LastTagged,
                false,
                TaggedPermissionTargetSurface::Other,
                None,
            )),
        )),
        alt((
            primitives::phrase(&["the", "exiled", "card"]).value((
                TaggedPermissionReference::SourceExiled,
                false,
                TaggedPermissionTargetSurface::Other,
                None,
            )),
            primitives::any_phrase(&[&["that", "revealed", "card"], &["the", "revealed", "card"]])
                .value((
                    TaggedPermissionReference::LastRevealed,
                    false,
                    TaggedPermissionTargetSurface::Other,
                    None,
                )),
            primitives::any_phrase(&[&["the", "copy"], &["that", "copy"], &["a", "copy"]]).value((
                TaggedPermissionReference::LastTagged,
                true,
                TaggedPermissionTargetSurface::Other,
                None,
            )),
        )),
    ))
    .parse_next(input)
}

fn parse_tagged_permission_target_surface_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TaggedPermissionTargetSurface> {
    alt((
        (primitives::kw("it"), primitives::sentence_end()).value(TaggedPermissionTargetSurface::It),
        (
            primitives::phrase(&["that", "card"]),
            primitives::sentence_end(),
        )
            .value(TaggedPermissionTargetSurface::ThatCard),
        (
            primitives::phrase(&["that", "spell"]),
            primitives::sentence_end(),
        )
            .value(TaggedPermissionTargetSurface::ThatSpell),
        (
            primitives::phrase(&["those", "cards"]),
            primitives::sentence_end(),
        )
            .value(TaggedPermissionTargetSurface::ThoseCards),
        (
            primitives::phrase(&["spells", "from", "among", "those", "exiled", "cards"]),
            primitives::sentence_end(),
        )
            .value(TaggedPermissionTargetSurface::SpellsFromAmongThoseExiledCards),
        (
            primitives::phrase(&["spells", "from", "among", "those", "cards"]),
            primitives::sentence_end(),
        )
            .value(TaggedPermissionTargetSurface::SpellsFromAmongThoseCards),
        (
            repeat_till::<_, _, (), _, _, _, _>(
                0..,
                any.void(),
                peek(primitives::phrase(&[
                    "from", "among", "cards", "exiled", "with",
                ])),
            ),
            primitives::phrase(&["from", "among", "cards", "exiled", "with"]),
            repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end())),
            primitives::sentence_end(),
        )
            .value(TaggedPermissionTargetSurface::SpellFromAmongSourceExiledCards),
    ))
    .parse_next(input)
}

fn parse_until_source_exiles_another_permission_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<UntilSourceExilesAnotherPermissionFact<'a>> {
    let (actor, verb) = parse_permission_lead_lexed.parse_next(input)?;
    let (reference, as_copy, target_surface, _max_plays) =
        parse_tagged_permission_target_lexed.parse_next(input)?;
    if as_copy {
        return Err(primitives::backtrack_err(
            "until-source-exiles-another permission",
            "a tagged card rather than a copy",
        ));
    }
    primitives::phrase(&["until", "you", "exile", "another", "card", "with"]).parse_next(input)?;
    let source_reference_tokens = sentence_body_tokens(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(UntilSourceExilesAnotherPermissionFact {
        actor,
        verb,
        reference,
        target_surface,
        source_reference_tokens,
    })
}

fn parse_permission_lifetime_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PermissionLifetimeFact> {
    alt((
        parse_for_as_long_as_exiled_lexed.value(PermissionLifetimeFact::ForAsLongAsExiled),
        primitives::phrase(&[
            "for", "as", "long", "as", "you", "control", "this", "creature",
        ])
        .value(PermissionLifetimeFact::ForAsLongAsYouControlSource),
    ))
    .parse_next(input)
}

fn parse_for_as_long_as_exiled_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::any_phrase(&[
        &["for", "as", "long", "as", "it", "remains", "exiled"],
        &[
            "for", "as", "long", "as", "that", "card", "remains", "exiled",
        ],
        &[
            "for", "as", "long", "as", "those", "cards", "remain", "exiled",
        ],
        &["for", "as", "long", "as", "they", "remain", "exiled"],
    ])
    .void()
    .parse_next(input)
}

fn parse_allow_any_color_for_cast_suffix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AllowAnyColorForCastSuffixFact<'a>> {
    let body_tokens = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(parse_allow_any_color_for_cast_lexed),
    )
    .map(|((), _reference)| ())
    .take()
    .parse_next(input)?;
    let (mana_spend_mode, reference) = parse_allow_any_color_for_cast_lexed.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(AllowAnyColorForCastSuffixFact {
        body_tokens,
        mana_spend_mode,
        reference,
    })
}

fn parse_allow_any_color_for_cast_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(
    ironsmith_core::value_model::ManaSpendMode,
    ManaSpendCastReference,
)> {
    let mode = alt((
        primitives::phrase(&[
            "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast",
        ])
        .value(ironsmith_core::value_model::ManaSpendMode::AnyType),
        primitives::phrase(&[
            "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of",
            "any", "color", "to", "cast",
        ])
        .value(ironsmith_core::value_model::ManaSpendMode::AnyColor),
    ))
    .parse_next(input)?;
    let reference = alt((
        primitives::kw("it").value(ManaSpendCastReference::It),
        primitives::phrase(&["that", "spell"]).value(ManaSpendCastReference::ThatSpell),
        primitives::kw("them").value(ManaSpendCastReference::Them),
        primitives::phrase(&["those", "spells"]).value(ManaSpendCastReference::ThoseSpells),
    ))
    .parse_next(input)?;
    Ok((mode, reference))
}

fn parse_without_paying_mana_cost_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["without", "paying"]).parse_next(input)?;
    alt((
        primitives::phrase(&["its", "mana", "cost"]),
        primitives::phrase(&["their", "mana", "cost"]),
        primitives::phrase(&["their", "mana", "costs"]),
        primitives::phrase(&["that", "card", "mana", "cost"]),
        primitives::phrase(&["that", "cards", "mana", "cost"]),
    ))
    .void()
    .parse_next(input)
}

fn parse_permission_tail_lexed<'a>(
    input: &mut LexStream<'a>,
    default_lifetime: PermissionLifetimeFact,
) -> WResult<(PermissionLifetimeFact, bool)> {
    alt((
        (
            parse_permission_turn_duration_lexed,
            opt(parse_without_paying_mana_cost_lexed),
            primitives::sentence_end(),
        )
            .map(|(duration, free, ())| (duration, free.is_some())),
        (
            parse_without_paying_mana_cost_lexed,
            parse_permission_turn_duration_lexed,
            primitives::sentence_end(),
        )
            .map(|(_, duration, ())| (duration, true)),
        (parse_permission_lifetime_lexed, primitives::sentence_end())
            .map(|(lifetime, ())| (lifetime, false)),
        (
            parse_without_paying_mana_cost_lexed,
            primitives::sentence_end(),
        )
            .value((default_lifetime, true)),
        eof.value((default_lifetime, false)),
    ))
    .parse_next(input)
}

fn parse_permission_turn_duration_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PermissionLifetimeFact> {
    alt((
        primitives::phrase(&["until", "your", "next", "end", "step"])
            .value(PermissionLifetimeFact::UntilYourNextEndStep),
        leaf::parse_leaf_turn_duration_phrase_lexed.map(lifetime_from_turn_duration),
    ))
    .parse_next(input)
}

fn parse_conditional_tagged_free_cast_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalTaggedFreeCastTailFact<'a>> {
    let lifetime = alt((
        (
            primitives::phrase(&["this", "turn"]),
            parse_without_paying_mana_cost_lexed,
        )
            .value(PermissionLifetimeFact::ThisTurn),
        parse_without_paying_mana_cost_lexed.value(PermissionLifetimeFact::Immediate),
    ))
    .parse_next(input)?;
    let condition_tokens = sentence_body_tokens(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ConditionalTaggedFreeCastTailFact {
        lifetime,
        condition_tokens,
    })
}

fn parse_tagged_mana_value_condition_intro_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::any_phrase(&[
        &["if", "it's", "a", "spell", "with", "mana", "value"],
        &[
            "if", "it's", "an", "instant", "spell", "with", "mana", "value",
        ],
        &["if", "its", "a", "spell", "with", "mana", "value"],
        &[
            "if", "its", "an", "instant", "spell", "with", "mana", "value",
        ],
        &["if", "it", "is", "a", "spell", "with", "mana", "value"],
        &[
            "if", "it", "is", "an", "instant", "spell", "with", "mana", "value",
        ],
        &["if", "the", "spell's", "mana", "value"],
        &["if", "the", "spells", "mana", "value"],
        &["if", "that", "spell's", "mana", "value"],
        &["if", "that", "spells", "mana", "value"],
        &["if", "its", "mana", "value"],
    ])
    .void()
    .parse_next(input)
}

fn parse_additional_land_play_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AdditionalLandPlayFact<'a>> {
    primitives::kw("play").parse_next(input)?;
    let count_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::any_phrase(&[
            &["additional", "land", "this", "turn"],
            &["additional", "lands", "this", "turn"],
        ])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::any_phrase(&[
        &["additional", "land", "this", "turn"],
        &["additional", "lands", "this", "turn"],
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let (count, used) = values::parse_value_prefix_lexed(count_tokens)
        .ok_or_else(|| primitives::backtrack_err("additional land count", "typed count value"))?;
    if used != count_tokens.len() {
        return Err(primitives::backtrack_err(
            "additional land count",
            "complete typed count value",
        ));
    }
    Ok(AdditionalLandPlayFact {
        count,
        count_tokens,
    })
}

fn parse_unsupported_permission_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<UnsupportedPermissionFact> {
    alt((
        (
            primitives::phrase(&[
                "play", "any", "number", "of", "lands", "on", "each", "of", "your", "turns",
            ]),
            primitives::sentence_end(),
        )
            .value(UnsupportedPermissionFact::AdditionalLandEachTurn),
        parse_for_as_long_as_play_cast_lexed.value(UnsupportedPermissionFact::ForAsLongAsPlayCast),
    ))
    .parse_next(input)
}

fn parse_for_as_long_as_play_cast_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "as", "long", "as"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::comma()))
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    parse_permission_lead_lexed.parse_next(input)?;
    sentence_body_tokens(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_revealed_top_library_permission_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RevealedTopLibraryPermissionFact<'a>> {
    primitives::phrase(&["until", "end", "of", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["for", "as", "long", "as"]).parse_next(input)?;
    primitives::any_phrase(&[
        &["that", "card"],
        &["that", "revealed", "card"],
        &["the", "revealed", "card"],
    ])
    .parse_next(input)?;
    primitives::phrase(&["remains", "on", "top", "of", "your", "library"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "play", "with", "the", "top", "card", "of", "your", "library", "revealed", "and",
    ])
    .parse_next(input)?;
    let permission_tokens = sentence_body_tokens(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RevealedTopLibraryPermissionFact { permission_tokens })
}

fn parse_for_as_long_as_look_at_tagged_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ForAsLongAsLookAtTaggedFact<'a>> {
    parse_for_as_long_as_exiled_lexed.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let reference = alt((
        primitives::phrase(&["you", "may", "look", "at", "it"]).value(TaggedLookReference::It),
        primitives::phrase(&["you", "may", "look", "at", "that", "card"])
            .value(TaggedLookReference::ThatCard),
        primitives::phrase(&["you", "may", "look", "at", "them"]).value(TaggedLookReference::Them),
        primitives::phrase(&["you", "may", "look", "at", "those", "cards"])
            .value(TaggedLookReference::ThoseCards),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    opt(primitives::kw("and")).parse_next(input)?;
    let permission_tokens = sentence_body_tokens(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ForAsLongAsLookAtTaggedFact {
        lifetime: PermissionLifetimeFact::ForAsLongAsExiled,
        reference,
        permission_tokens,
    })
}

fn lifetime_from_turn_duration(duration: leaf::LeafTurnDurationPhrase) -> PermissionLifetimeFact {
    match duration {
        leaf::LeafTurnDurationPhrase::ThisTurn => PermissionLifetimeFact::ThisTurn,
        leaf::LeafTurnDurationPhrase::UntilEndOfTurn => PermissionLifetimeFact::UntilEndOfTurn,
        leaf::LeafTurnDurationPhrase::UntilYourNextTurn
        | leaf::LeafTurnDurationPhrase::UntilYourNextTurnEnd => {
            PermissionLifetimeFact::UntilYourNextTurn
        }
    }
}

fn sentence_body_tokens<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn permission_leads_and_tagged_targets_are_typed() {
        let tokens = lex("You may cast that spell this turn");
        let lead = parse_permission_lead_tokens(&tokens).unwrap();
        assert_eq!(lead.actor, PermissionActor::You);
        assert_eq!(lead.verb, PermissionVerb::Cast);
        assert!(!lead.verb.allows_land());

        let target = parse_tagged_permission_target_tokens(lead.rest_tokens).unwrap();
        assert_eq!(target.reference, TaggedPermissionReference::LastTagged);
        assert_eq!(target.surface, TaggedPermissionTargetSurface::ThatSpell);
        assert_eq!(
            TokenWordView::new(target.rest_tokens).word_refs(),
            ["this", "turn"]
        );
    }

    #[test]
    fn one_of_tagged_collection_preserves_shared_deferred_limit() {
        let tokens = lex("You may play one of those cards until your next end step");
        let lead = parse_permission_lead_tokens(&tokens).unwrap();
        let target = parse_tagged_permission_target_tokens(lead.rest_tokens).unwrap();
        assert_eq!(target.reference, TaggedPermissionReference::LastTagged);
        assert_eq!(target.max_plays, Some(1));
        assert_eq!(
            TokenWordView::new(target.rest_tokens).word_refs(),
            ["until", "your", "next", "end", "step"]
        );
    }

    #[test]
    fn source_exile_bounded_permission_keeps_target_and_source_surfaces() {
        let cases = [
            (
                "You may play that card until you exile another card with this enchantment",
                TaggedPermissionTargetSurface::ThatCard,
                "this enchantment",
            ),
            (
                "You may play it until you exile another card with this artifact",
                TaggedPermissionTargetSurface::It,
                "this artifact",
            ),
        ];
        for (text, target_surface, source_surface) in cases {
            let tokens = lex(text);
            let parsed = parse_until_source_exiles_another_permission_tokens(&tokens).unwrap();
            assert_eq!(parsed.actor, PermissionActor::You);
            assert_eq!(parsed.verb, PermissionVerb::Play);
            assert_eq!(parsed.reference, TaggedPermissionReference::LastTagged);
            assert_eq!(parsed.target_surface, target_surface);
            assert_eq!(
                TokenWordView::new(parsed.source_reference_tokens).word_refs(),
                source_surface.split(' ').collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn lifetime_and_tail_facts_preserve_free_cast_and_mana_permissions() {
        let prefix = lex("For as long as those cards remain exiled, you may cast them");
        let prefix = parse_permission_lifetime_prefix_tokens(&prefix).unwrap();
        assert_eq!(prefix.lifetime, PermissionLifetimeFact::ForAsLongAsExiled);

        let tail = lex(
            "this turn without paying its mana cost and mana of any type can be spent to cast it",
        );
        let parsed =
            parse_permission_tail_tokens(&tail, PermissionLifetimeFact::Immediate).unwrap();
        assert_eq!(parsed.lifetime, PermissionLifetimeFact::ThisTurn);
        assert!(parsed.without_paying_mana_cost);
        assert!(parsed.allow_any_color_for_cast);

        let any_type = parse_allow_any_color_for_cast_suffix_tokens(&tail).unwrap();
        assert_eq!(
            any_type.mana_spend_mode,
            ironsmith_core::value_model::ManaSpendMode::AnyType
        );
        assert_eq!(any_type.reference, ManaSpendCastReference::It);

        let any_color =
            lex("this turn, and you may spend mana as though it were mana of any color to cast it");
        let any_color = parse_allow_any_color_for_cast_suffix_tokens(&any_color).unwrap();
        assert_eq!(
            any_color.mana_spend_mode,
            ironsmith_core::value_model::ManaSpendMode::AnyColor
        );
        assert_eq!(any_color.reference, ManaSpendCastReference::It);
    }

    #[test]
    fn temporary_permission_references_preserve_distinct_collection_surfaces() {
        let cases = [
            ("them", TaggedPermissionTargetSurface::Them),
            ("those cards", TaggedPermissionTargetSurface::ThoseCards),
            (
                "spells from among those cards",
                TaggedPermissionTargetSurface::SpellsFromAmongThoseCards,
            ),
            (
                "spells from among those exiled cards",
                TaggedPermissionTargetSurface::SpellsFromAmongThoseExiledCards,
            ),
            (
                "spells from among them",
                TaggedPermissionTargetSurface::Other,
            ),
        ];
        for (text, expected) in cases {
            let tokens = lex(text);
            let parsed = parse_tagged_permission_target_tokens(&tokens).unwrap();
            assert_eq!(parsed.surface, expected, "{text}");
        }

        let suffix = lex("and mana of any type can be spent to cast those spells");
        assert_eq!(
            parse_allow_any_color_for_cast_suffix_tokens(&suffix)
                .unwrap()
                .reference,
            ManaSpendCastReference::ThoseSpells
        );
    }

    #[test]
    fn tagged_look_revealed_top_and_permanent_pool_facts_keep_boundaries() {
        let singular = lex(
            "For as long as it remains exiled, you may look at that card and you may cast it if it's a creature spell",
        );
        let singular = parse_for_as_long_as_look_at_tagged_tokens(&singular).unwrap();
        assert_eq!(singular.reference, TaggedLookReference::ThatCard);
        assert_eq!(
            TokenWordView::new(singular.permission_tokens).word_refs(),
            [
                "you", "may", "cast", "it", "if", "its", "a", "creature", "spell"
            ]
        );

        let look = lex(
            "For as long as those cards remain exiled, you may look at them, and you may cast permanent spells from among them",
        );
        let look = parse_for_as_long_as_look_at_tagged_tokens(&look).unwrap();
        assert_eq!(look.reference, TaggedLookReference::Them);
        let permanent = parse_permission_lead_tokens(look.permission_tokens).unwrap();
        let permanent = parse_spells_from_tagged_tokens(permanent.rest_tokens).unwrap();
        assert_eq!(
            TokenWordView::new(permanent.subject_tokens).word_refs(),
            ["permanent", "spells"]
        );
        assert!(permanent.tail_tokens.is_empty());

        let revealed = lex(
            "Until end of turn, for as long as that revealed card remains on top of your library, play with the top card of your library revealed and you may play that card",
        );
        let revealed = parse_revealed_top_library_permission_tokens(&revealed).unwrap();
        assert_eq!(
            TokenWordView::new(revealed.permission_tokens).word_refs(),
            ["you", "may", "play", "that", "card"]
        );
    }

    #[test]
    fn qualified_spell_pool_fact_preserves_subject_surface_and_tail() {
        let tokens =
            lex("noncreature spells from among those cards without paying their mana costs");
        let parsed = parse_spells_from_tagged_tokens(&tokens).unwrap();
        assert_eq!(parsed.reference, TaggedPermissionReference::LastTagged);
        assert_eq!(
            TokenWordView::new(parsed.subject_tokens).word_refs(),
            ["noncreature", "spells"]
        );
        assert_eq!(
            parsed.surface,
            TaggedPermissionTargetSurface::SpellsFromAmongThoseCards
        );
        assert_eq!(
            TokenWordView::new(parsed.tail_tokens).word_refs(),
            ["without", "paying", "their", "mana", "costs"]
        );

        let permanent = lex("permanent spells from among them this turn");
        let permanent = parse_spells_from_tagged_tokens(&permanent).unwrap();
        assert_eq!(
            TokenWordView::new(permanent.subject_tokens).word_refs(),
            ["permanent", "spells"]
        );
        assert_eq!(permanent.surface, TaggedPermissionTargetSurface::Other);
        assert_eq!(
            TokenWordView::new(permanent.tail_tokens).word_refs(),
            ["this", "turn"]
        );
    }

    #[test]
    fn additional_land_and_conditional_free_cast_facts_are_semantic() {
        let land_tokens = lex("Play an additional land this turn");
        let land = parse_additional_land_play_tokens(&land_tokens).unwrap();
        assert_eq!(land.count, Value::Fixed(1));

        let tail = lex("without paying its mana cost if its mana value is 3 or less");
        let tail = parse_conditional_tagged_free_cast_tail_tokens(&tail).unwrap();
        assert_eq!(tail.lifetime, PermissionLifetimeFact::Immediate);
        let condition = parse_tagged_mana_value_condition_tokens(tail.condition_tokens).unwrap();
        assert_eq!(condition.operator, ValueComparisonOperator::LessThanOrEqual);
        assert_eq!(condition.right, Value::Fixed(3));
    }

    #[test]
    fn unsupported_permission_shapes_are_typed() {
        assert_eq!(
            parse_unsupported_permission_tokens(&lex(
                "Play any number of lands on each of your turns"
            )),
            Some(UnsupportedPermissionFact::AdditionalLandEachTurn)
        );
    }
}
