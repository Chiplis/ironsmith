//! Typed surface facts for tagged cast and play permission clauses.
//!
//! This module deliberately stops at grammar-owned facts.  The permission
//! family remains responsible for turning these facts into grants and effects.

use crate::effect::{Value, ValueComparisonOperator};
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};

use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::{effects, leaf, primitives, values};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionActor {
    You,
    AnyPlayer,
    ItsOwner,
    Implicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionVerb {
    Cast,
    Play,
}

impl PermissionVerb {
    pub fn allows_land(self) -> bool {
        self == Self::Play
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionLeadFact<'a> {
    pub actor: PermissionActor,
    pub verb: PermissionVerb,
    pub rest_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedPermissionReference {
    LastTagged,
    SourceExiled,
    LastRevealed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedPermissionTargetSurface {
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
pub struct TaggedPermissionTargetFact<'a> {
    pub reference: TaggedPermissionReference,
    pub as_copy: bool,
    pub surface: TaggedPermissionTargetSurface,
    /// Total uses shared by the tagged collection. This preserves deferred
    /// choices such as "play one of those cards" without selecting a card
    /// when the permission is created.
    pub max_plays: Option<u32>,
    pub rest_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UntilSourceExilesAnotherPermissionFact<'a> {
    pub actor: PermissionActor,
    pub verb: PermissionVerb,
    pub reference: TaggedPermissionReference,
    pub target_surface: TaggedPermissionTargetSurface,
    pub source_reference_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLifetimeFact {
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
pub struct PermissionLifetimePrefixFact<'a> {
    pub lifetime: PermissionLifetimeFact,
    pub rest_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSpendCastReference {
    It,
    ThatSpell,
    Them,
    ThoseSpells,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllowAnyColorForCastSuffixFact<'a> {
    pub body_tokens: &'a [OwnedLexToken],
    pub mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
    pub reference: ManaSpendCastReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionTailFact {
    pub lifetime: PermissionLifetimeFact,
    pub without_paying_mana_cost: bool,
    pub allow_any_color_for_cast: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaggedPermissionTailFact<'a> {
    pub from_exile: bool,
    pub tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalTaggedFreeCastTailFact<'a> {
    pub lifetime: PermissionLifetimeFact,
    pub condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaggedManaValueConditionFact {
    pub operator: ValueComparisonOperator,
    pub right: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedPermissionFact {
    AdditionalLandEachTurn,
    ForAsLongAsPlayCast,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdditionalLandPlayFact<'a> {
    pub count: Value,
    pub count_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealedTopLibraryPermissionFact<'a> {
    pub permission_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedLookReference {
    It,
    ThatCard,
    Them,
    ThoseCards,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForAsLongAsLookAtTaggedFact<'a> {
    pub lifetime: PermissionLifetimeFact,
    pub reference: TaggedLookReference,
    pub permission_tokens: &'a [OwnedLexToken],
}

/// A spell-subject-qualified reference to the collection tagged by a preceding
/// action, such as "noncreature spells from among those cards".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellsFromTaggedFact<'a> {
    pub reference: TaggedPermissionReference,
    pub subject_tokens: &'a [OwnedLexToken],
    pub surface: TaggedPermissionTargetSurface,
    pub tail_tokens: &'a [OwnedLexToken],
}

pub fn parse_permission_lead_tokens(tokens: &[OwnedLexToken]) -> Option<PermissionLeadFact<'_>> {
    let ((actor, verb), rest_tokens) =
        primitives::parse_prefix(tokens, parse_permission_lead_lexed)?;
    Some(PermissionLeadFact {
        actor,
        verb,
        rest_tokens,
    })
}

pub fn parse_tagged_permission_target_tokens(
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

pub fn parse_tagged_permission_target_surface_tokens(
    tokens: &[OwnedLexToken],
) -> TaggedPermissionTargetSurface {
    primitives::parse_all(
        tokens,
        parse_tagged_permission_target_surface_lexed,
        "tagged-permission-target-surface",
    )
    .unwrap_or(TaggedPermissionTargetSurface::Other)
}

pub fn parse_until_source_exiles_another_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<UntilSourceExilesAnotherPermissionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_until_source_exiles_another_permission_lexed,
        "until-source-exiles-another-permission",
    )
    .ok()
}

pub fn parse_permission_lifetime_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PermissionLifetimePrefixFact<'_>> {
    let (lifetime, rest_tokens) =
        primitives::parse_prefix(tokens, parse_permission_lifetime_lexed)?;
    Some(PermissionLifetimePrefixFact {
        lifetime,
        rest_tokens,
    })
}

pub fn parse_permission_duration_prefix_tokens(
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

pub fn parse_allow_any_color_for_cast_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AllowAnyColorForCastSuffixFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_allow_any_color_for_cast_suffix_lexed,
        "allow-any-color-for-cast-suffix",
    )
    .ok()
}

pub fn parse_permission_tail_tokens(
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

pub fn parse_tagged_permission_tail_tokens(
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

pub fn parse_conditional_tagged_free_cast_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalTaggedFreeCastTailFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_conditional_tagged_free_cast_tail_lexed,
        "conditional-tagged-free-cast-tail",
    )
    .ok()
}

pub fn parse_tagged_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TaggedManaValueConditionFact> {
    let (_, comparison_tokens) =
        primitives::parse_prefix(tokens, parse_tagged_mana_value_condition_intro_lexed)?;
    let (operator, right_tokens) = values::parse_value_comparison_tokens(comparison_tokens)?;
    let right = effects::parse_consult_condition_value_shape(right_tokens)?;
    Some(TaggedManaValueConditionFact { operator, right })
}

pub fn parse_additional_land_play_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalLandPlayFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_additional_land_play_lexed,
        "additional-land-play",
    )
    .ok()
}

pub fn parse_unsupported_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<UnsupportedPermissionFact> {
    primitives::parse_all(
        tokens,
        parse_unsupported_permission_lexed,
        "unsupported-permission",
    )
    .ok()
}

pub fn parse_revealed_top_library_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RevealedTopLibraryPermissionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_revealed_top_library_permission_lexed,
        "revealed-top-library-permission",
    )
    .ok()
}

pub fn parse_for_as_long_as_look_at_tagged_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ForAsLongAsLookAtTaggedFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_for_as_long_as_look_at_tagged_lexed,
        "for-as-long-as-look-at-tagged",
    )
    .ok()
}

pub fn parse_spells_from_tagged_tokens(
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

#[cfg(test)]
#[path = "tagged_surface_inline_tests.rs"]
mod tests;

#[path = "tagged_surface/object_action_programs.rs"]
mod object_action_programs;
use object_action_programs::sentence_body_tokens;
#[path = "tagged_surface/resource_programs.rs"]
mod resource_programs;
use resource_programs::lifetime_from_turn_duration;
#[path = "tagged_surface/library_programs.rs"]
mod library_programs;
use library_programs::{
    parse_for_as_long_as_look_at_tagged_lexed, parse_revealed_top_library_permission_lexed,
};
#[path = "tagged_surface/permission_programs.rs"]
mod permission_programs;
use permission_programs::parse_for_as_long_as_play_cast_lexed;
