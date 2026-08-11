use super::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed;
use super::grammar::permission_facts::{
    graveyard_source as permission_graveyard_facts,
    source_exiled as permission_source_exiled_facts, subject_filters as permission_subject_facts,
    tagged_surface as permission_tagged_facts, zone_free_cast as permission_zone_facts,
};
use super::grammar::values::parse_value_comparison_tokens;
use super::lexer::{OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use super::object_filters::merge_spell_filters;
use super::token_primitives::{TurnDurationPhrase, parse_turn_duration_suffix};
use super::util::{parse_target_phrase, strip_leading_token_words_any, trim_commas};
use crate::effect::{Until, Value, ValueComparisonOperator};
use crate::host::{CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, TagKey, TargetAst};
use crate::runtime_backend::GrantedAbilityAst;
use crate::runtime_backend::grammar::shared_util::value_semantics::{
    parse_value_prefix_lexed, starts_explicit_ordered_comparison,
};
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionLifetime {
    Immediate,
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
    UntilYourNextEndStep,
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
        /// Total plays shared by the tagged collection, if authored.
        max_plays: Option<u32>,
        without_paying_mana_cost: bool,
        lifetime: PermissionLifetime,
        filter: Option<ObjectFilter>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
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
    max_plays: Option<u32>,
    surface: Option<ironsmith_core::GrantPlayTaggedObjectSurface>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnsupportedPermissionShape {
    AdditionalLandEachTurn,
    ForAsLongAsPlayCast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdditionalLandPlayClause<'a> {
    count_tokens: &'a [OwnedLexToken],
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
    from_exile: bool,
    tail_tokens: &'a [OwnedLexToken],
}

fn tagged_permission_target_surface(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::GrantPlayTaggedObjectSurface> {
    tagged_permission_object_surface(
        permission_tagged_facts::parse_tagged_permission_target_surface_tokens(tokens),
    )
}

fn tagged_permission_object_surface(
    surface: permission_tagged_facts::TaggedPermissionTargetSurface,
) -> Option<ironsmith_core::GrantPlayTaggedObjectSurface> {
    match surface {
        permission_tagged_facts::TaggedPermissionTargetSurface::It => {
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::It)
        }
        permission_tagged_facts::TaggedPermissionTargetSurface::ThatCard => {
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard)
        }
        permission_tagged_facts::TaggedPermissionTargetSurface::ThatSpell => {
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThatSpell)
        }
        permission_tagged_facts::TaggedPermissionTargetSurface::Them => {
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::Them)
        }
        permission_tagged_facts::TaggedPermissionTargetSurface::ThoseCards => {
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThoseCards)
        }
        permission_tagged_facts::TaggedPermissionTargetSurface::SpellsFromAmongThoseCards => {
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseCards)
        }
        permission_tagged_facts::TaggedPermissionTargetSurface::SpellsFromAmongThoseExiledCards => {
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseExiledCards)
        }
        permission_tagged_facts::TaggedPermissionTargetSurface::SpellFromAmongSourceExiledCards
        | permission_tagged_facts::TaggedPermissionTargetSurface::Other => None,
    }
}

fn unsupported_permission_shape(tokens: &[OwnedLexToken]) -> Option<UnsupportedPermissionShape> {
    match permission_tagged_facts::parse_unsupported_permission_tokens(tokens)? {
        permission_tagged_facts::UnsupportedPermissionFact::AdditionalLandEachTurn => {
            Some(UnsupportedPermissionShape::AdditionalLandEachTurn)
        }
        permission_tagged_facts::UnsupportedPermissionFact::ForAsLongAsPlayCast => {
            Some(UnsupportedPermissionShape::ForAsLongAsPlayCast)
        }
    }
}

fn parse_additional_land_play_clause(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalLandPlayClause<'_>> {
    let parsed = permission_tagged_facts::parse_additional_land_play_tokens(tokens)?;
    Some(AdditionalLandPlayClause {
        count_tokens: parsed.count_tokens,
    })
}

fn permission_lifetime_from_tagged_fact(
    lifetime: permission_tagged_facts::PermissionLifetimeFact,
) -> PermissionLifetime {
    match lifetime {
        permission_tagged_facts::PermissionLifetimeFact::Immediate => PermissionLifetime::Immediate,
        permission_tagged_facts::PermissionLifetimeFact::ThisTurn => PermissionLifetime::ThisTurn,
        permission_tagged_facts::PermissionLifetimeFact::UntilEndOfTurn => {
            PermissionLifetime::UntilEndOfTurn
        }
        permission_tagged_facts::PermissionLifetimeFact::UntilYourNextTurn => {
            PermissionLifetime::UntilYourNextTurn
        }
        permission_tagged_facts::PermissionLifetimeFact::UntilYourNextEndStep => {
            PermissionLifetime::UntilYourNextEndStep
        }
        permission_tagged_facts::PermissionLifetimeFact::ForAsLongAsExiled => {
            PermissionLifetime::ForAsLongAsExiled
        }
        permission_tagged_facts::PermissionLifetimeFact::ForAsLongAsYouControlSource => {
            PermissionLifetime::ForAsLongAsYouControlSource
        }
        permission_tagged_facts::PermissionLifetimeFact::Static => PermissionLifetime::Static,
    }
}

fn permission_lifetime_to_tagged_fact(
    lifetime: PermissionLifetime,
) -> permission_tagged_facts::PermissionLifetimeFact {
    match lifetime {
        PermissionLifetime::Immediate => permission_tagged_facts::PermissionLifetimeFact::Immediate,
        PermissionLifetime::ThisTurn => permission_tagged_facts::PermissionLifetimeFact::ThisTurn,
        PermissionLifetime::UntilEndOfTurn => {
            permission_tagged_facts::PermissionLifetimeFact::UntilEndOfTurn
        }
        PermissionLifetime::UntilYourNextTurn => {
            permission_tagged_facts::PermissionLifetimeFact::UntilYourNextTurn
        }
        PermissionLifetime::UntilYourNextEndStep => {
            permission_tagged_facts::PermissionLifetimeFact::UntilYourNextEndStep
        }
        PermissionLifetime::ForAsLongAsExiled => {
            permission_tagged_facts::PermissionLifetimeFact::ForAsLongAsExiled
        }
        PermissionLifetime::ForAsLongAsYouControlSource => {
            permission_tagged_facts::PermissionLifetimeFact::ForAsLongAsYouControlSource
        }
        PermissionLifetime::Static => permission_tagged_facts::PermissionLifetimeFact::Static,
    }
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

fn parse_play_from_zone_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<PlayFromZoneRest<'a>> {
    let fact = permission_zone_facts::parse_play_from_zone_tokens(rest_tokens)?;
    Some(PlayFromZoneRest {
        filter_tokens: fact.filter_tokens,
        zone: fact.zone,
    })
}

fn parse_lands_from_top_library_permission_rest_tokens(tokens: &[OwnedLexToken]) -> bool {
    permission_zone_facts::parse_lands_from_top_library_tokens(tokens).is_some()
}

fn parse_lands_and_cast_from_top_library_permission_rest_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LandsAndCastFromLibraryPermission<'a>> {
    let fact = permission_zone_facts::parse_lands_and_cast_from_top_library_tokens(tokens)?;
    Some(LandsAndCastFromLibraryPermission {
        spell_filter_tokens: fact.spell_filter_tokens,
    })
}

fn parse_flash_grant_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<FlashGrantRest<'a>> {
    let fact = permission_zone_facts::parse_flash_grant_tokens(rest_tokens)?;
    let lifetime = match fact.lifetime {
        permission_zone_facts::ZonePermissionLifetimeFact::Static => PermissionLifetime::Static,
        permission_zone_facts::ZonePermissionLifetimeFact::ThisTurn => PermissionLifetime::ThisTurn,
        permission_zone_facts::ZonePermissionLifetimeFact::UntilEndOfTurn => {
            PermissionLifetime::UntilEndOfTurn
        }
    };
    Some(FlashGrantRest {
        filter_tokens: fact.filter_tokens,
        lifetime,
    })
}

fn parse_revealed_top_library_permission_intro_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<RevealedTopLibraryPermissionIntro<'a>> {
    let fact = permission_tagged_facts::parse_revealed_top_library_permission_tokens(tokens)?;
    Some(RevealedTopLibraryPermissionIntro {
        permission_tokens: trim_lexed_commas(fact.permission_tokens),
    })
}

fn strip_for_as_long_as_look_at_tagged_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let look = permission_tagged_facts::parse_for_as_long_as_look_at_tagged_tokens(tokens)?;
    let prefix = permission_tagged_facts::parse_permission_lifetime_prefix_tokens(tokens)?;
    let prefix_len = tokens.len().checked_sub(prefix.rest_tokens.len())?;
    let mut permission_tokens = tokens[..prefix_len].to_vec();
    permission_tokens.extend_from_slice(look.permission_tokens);
    Some(permission_tokens)
}

fn parse_filtered_spells_from_among_tagged_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Result<Option<(TaggedPermissionTarget, &'a [OwnedLexToken], ObjectFilter)>, CardTextError> {
    let Some(fact) = permission_tagged_facts::parse_spells_from_tagged_tokens(tokens) else {
        return Ok(None);
    };
    let Some(mut filter) =
        permission_subject_facts::parse_cast_permission_filter_tokens(fact.subject_tokens)?
    else {
        return Ok(None);
    };
    mark_generic_spell_filter_nonland(&mut filter, fact.subject_tokens);
    Ok(Some((
        TaggedPermissionTarget {
            tag: TagKey::from(IT_TAG),
            as_copy: false,
            max_plays: None,
            surface: tagged_permission_object_surface(fact.surface),
        },
        fact.tail_tokens,
        filter,
    )))
}

/// "a [creature] spell from among cards exiled with this <source-type>"
/// (Prosper window grants, e.g. "Until end of turn, you may cast a spell from
/// among cards exiled with this enchantment without paying its mana cost.")
fn parse_spell_from_among_source_exiled_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(
    TaggedPermissionTarget,
    &'a [OwnedLexToken],
    Option<ObjectFilter>,
)> {
    let fact = permission_source_exiled_facts::parse_spell_from_source_exiled_tokens(tokens)?;
    let filter = match fact.kind {
        permission_source_exiled_facts::SourceExiledSpellKind::Any => None,
        permission_source_exiled_facts::SourceExiledSpellKind::Creature => {
            Some(ObjectFilter::creature())
        }
    };
    Some((
        TaggedPermissionTarget {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            as_copy: false,
            max_plays: None,
            surface: Some(
                ironsmith_core::GrantPlayTaggedObjectSurface::SpellFromAmongCardsExiledWithSource {
                    creature_spell: matches!(
                        fact.kind,
                        permission_source_exiled_facts::SourceExiledSpellKind::Creature
                    ),
                    source: fact.reference.surface,
                },
            ),
        },
        fact.tail_tokens,
        filter,
    ))
}

fn parse_once_each_turn_graveyard_cast_rest_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<OnceEachTurnGraveyardCastRest<'a>> {
    let fact = permission_graveyard_facts::parse_once_each_turn_graveyard_cast_tokens(tokens)?;
    Some(OnceEachTurnGraveyardCastRest {
        subject_tokens: fact.subject_tokens,
        cost_tokens: fact.cost_tokens,
        exiles_after_resolution: fact.exiles_after_resolution,
    })
}

fn parse_free_cast_from_your_zone_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<FreeCastFromYourZoneRest<'a>> {
    let fact = permission_zone_facts::parse_free_cast_from_zone_tokens(rest_tokens)?;
    (fact.mana_value.is_none() && fact.zone != Zone::Command).then_some(FreeCastFromYourZoneRest {
        filter_tokens: fact.filter_tokens,
        zone: fact.zone,
    })
}

fn parse_mana_value_limited_free_cast_from_your_zone_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<ManaValueLimitedFreeCastFromYourZoneRest<'a>> {
    let fact = permission_zone_facts::parse_free_cast_from_zone_tokens(rest_tokens)?;
    let comparison = fact.mana_value?;
    (comparison.placement == permission_zone_facts::ManaValuePlacementFact::BeforeZone).then_some(
        ManaValueLimitedFreeCastFromYourZoneRest {
            filter_tokens: fact.filter_tokens,
            comparison_tokens: comparison.tokens,
            zone: fact.zone,
        },
    )
}

fn parse_zone_first_mana_value_limited_free_cast_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<ZoneFirstManaValueLimitedFreeCastRest<'a>> {
    let fact = permission_zone_facts::parse_free_cast_from_zone_tokens(rest_tokens)?;
    let comparison = fact.mana_value?;
    (comparison.placement == permission_zone_facts::ManaValuePlacementFact::AfterZone
        && matches!(fact.zone, Zone::Hand | Zone::Graveyard))
    .then_some(ZoneFirstManaValueLimitedFreeCastRest {
        filter_tokens: fact.filter_tokens,
        comparison_tokens: comparison.tokens,
        zone: fact.zone,
    })
}

fn parse_command_zone_free_cast_rest_tokens<'a>(
    rest_tokens: &'a [OwnedLexToken],
) -> Option<CommandZoneFreeCastRest<'a>> {
    let fact = permission_zone_facts::parse_free_cast_from_zone_tokens(rest_tokens)?;
    (fact.zone == Zone::Command && fact.mana_value.is_none()).then_some(CommandZoneFreeCastRest {
        filter_tokens: fact.filter_tokens,
    })
}

fn free_cast_filter_mentions_singular_spell(filter_tokens: &[OwnedLexToken]) -> bool {
    let facts = permission_subject_facts::parse_spell_subject_facts(filter_tokens);
    facts.contains_singular_spell && !facts.contains_plural_spells
}

fn strip_allow_any_color_for_cast_suffix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<permission_tagged_facts::AllowAnyColorForCastSuffixFact<'a>> {
    permission_tagged_facts::parse_allow_any_color_for_cast_suffix_tokens(tokens)
}

fn mana_spend_reference_surface(
    reference: permission_tagged_facts::ManaSpendCastReference,
) -> ironsmith_core::GrantPlayTaggedManaReferenceSurface {
    match reference {
        permission_tagged_facts::ManaSpendCastReference::It => {
            ironsmith_core::GrantPlayTaggedManaReferenceSurface::It
        }
        permission_tagged_facts::ManaSpendCastReference::ThatSpell => {
            ironsmith_core::GrantPlayTaggedManaReferenceSurface::ThatSpell
        }
        permission_tagged_facts::ManaSpendCastReference::Them => {
            ironsmith_core::GrantPlayTaggedManaReferenceSurface::Them
        }
        permission_tagged_facts::ManaSpendCastReference::ThoseSpells => {
            ironsmith_core::GrantPlayTaggedManaReferenceSurface::ThoseSpells
        }
    }
}

fn with_mana_reference_surface(
    surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    reference: Option<permission_tagged_facts::ManaSpendCastReference>,
) -> Option<ironsmith_core::GrantPlayTaggedSurface> {
    match reference {
        Some(reference) => Some(
            surface
                .unwrap_or_default()
                .with_mana_reference(mana_spend_reference_surface(reference)),
        ),
        None => surface,
    }
}

fn parse_permission_duration_prefix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> (Option<PermissionLifetime>, &'a [OwnedLexToken]) {
    let Some(fact) = permission_tagged_facts::parse_permission_duration_prefix_tokens(tokens)
    else {
        return (None, tokens);
    };
    (
        Some(permission_lifetime_from_tagged_fact(fact.lifetime)),
        fact.rest_tokens,
    )
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
    let fact = permission_tagged_facts::parse_permission_lead_tokens(tokens)?;
    let player = match fact.actor {
        permission_tagged_facts::PermissionActor::You => PlayerAst::You,
        permission_tagged_facts::PermissionActor::AnyPlayer => PlayerAst::Any,
        permission_tagged_facts::PermissionActor::ItsOwner => PlayerAst::ItsOwner,
        permission_tagged_facts::PermissionActor::Implicit => PlayerAst::Implicit,
    };
    Some((
        PermissionLead {
            player,
            allow_land: fact.verb.allows_land(),
        },
        fact.rest_tokens,
    ))
}

fn parse_tagged_cast_or_play_target_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(TaggedPermissionTarget, &'a [OwnedLexToken])> {
    let fact = permission_tagged_facts::parse_tagged_permission_target_tokens(tokens)?;
    let tag = match fact.reference {
        permission_tagged_facts::TaggedPermissionReference::LastTagged => TagKey::from(IT_TAG),
        permission_tagged_facts::TaggedPermissionReference::SourceExiled => {
            TagKey::from(crate::tag::SOURCE_EXILED_TAG)
        }
        permission_tagged_facts::TaggedPermissionReference::LastRevealed => {
            TagKey::from("__last_revealed__")
        }
    };
    Some((
        TaggedPermissionTarget {
            tag,
            as_copy: fact.as_copy,
            max_plays: fact.max_plays,
            surface: tagged_permission_object_surface(fact.surface),
        },
        fact.rest_tokens,
    ))
}

fn parse_until_source_exiles_another_permission(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let fact =
        permission_tagged_facts::parse_until_source_exiles_another_permission_tokens(tokens)?;
    let player = match fact.actor {
        permission_tagged_facts::PermissionActor::You => PlayerAst::You,
        permission_tagged_facts::PermissionActor::Implicit => PlayerAst::Implicit,
        permission_tagged_facts::PermissionActor::AnyPlayer
        | permission_tagged_facts::PermissionActor::ItsOwner => return None,
    };
    let tag = match fact.reference {
        permission_tagged_facts::TaggedPermissionReference::LastTagged => TagKey::from(IT_TAG),
        permission_tagged_facts::TaggedPermissionReference::SourceExiled => {
            TagKey::from(crate::tag::SOURCE_EXILED_TAG)
        }
        permission_tagged_facts::TaggedPermissionReference::LastRevealed => {
            TagKey::from("__last_revealed__")
        }
    };
    let object_surface = tagged_permission_object_surface(fact.target_surface)?;
    let source_words = token_word_refs(fact.source_reference_tokens);
    let source_surface = super::util::source_reference_surface_for_words(&source_words)
        .or_else(|| super::util::this_source_surface_for_words(&source_words))?;
    Some(
        EffectAst::subject_verb_grant_play_tagged_until_source_exiles_another(
            tag,
            player,
            fact.verb.allows_land(),
            source_surface,
            object_surface,
        ),
    )
}

fn parse_tagged_permission_tail_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> TaggedPermissionTail<'a> {
    let fact = permission_tagged_facts::parse_tagged_permission_tail_tokens(tokens);
    TaggedPermissionTail {
        from_exile: fact.from_exile,
        tail_tokens: fact.tail_tokens,
    }
}

fn parse_tagged_permission_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(ValueComparisonOperator, Value)> {
    let fact = permission_tagged_facts::parse_tagged_mana_value_condition_tokens(tokens)?;
    Some((fact.operator, fact.right))
}

fn parse_conditional_tagged_free_cast_tail_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<ConditionalTaggedFreeCastTail<'a>> {
    let fact = permission_tagged_facts::parse_conditional_tagged_free_cast_tail_tokens(tokens)?;
    Some(ConditionalTaggedFreeCastTail {
        lifetime: permission_lifetime_from_tagged_fact(fact.lifetime),
        condition_tokens: fact.condition_tokens,
    })
}

fn parse_permission_tail_tokens(
    tokens: &[OwnedLexToken],
    default_lifetime: PermissionLifetime,
) -> Option<(PermissionLifetime, bool)> {
    let fact = permission_tagged_facts::parse_permission_tail_tokens(
        tokens,
        permission_lifetime_to_tagged_fact(default_lifetime),
    )?;
    Some((
        permission_lifetime_from_tagged_fact(fact.lifetime),
        fact.without_paying_mana_cost,
    ))
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

fn exclude_lands_from_spell_filter(filter: &mut ObjectFilter) {
    if !filter
        .excluded_card_types
        .iter()
        .any(|card_type| card_type == &CardType::Land)
    {
        filter.excluded_card_types.push(CardType::Land);
    }
}

fn mark_generic_spell_filter_nonland(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    if permission_subject_facts::generic_spell_subject_requires_nonland(tokens) {
        exclude_lands_from_spell_filter(filter);
    }
}

fn build_temporary_tagged_permission_effect(
    tokens: &[OwnedLexToken],
    tag: TagKey,
    player: PlayerAst,
    allow_land: bool,
    without_paying_mana_cost: bool,
    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
    surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    filter: Option<ObjectFilter>,
) -> EffectAst {
    let grant = |tag| {
        EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            mana_spend_mode,
            surface,
        )
    };
    let Some(mut filter) = filter else {
        return grant(tag);
    };

    // Both the ordinary play permission and the alternative-cost grant are
    // tag-scoped. Narrow the preceding exiled collection to the authored
    // spell subject first so neither grant leaks to excluded cards.
    let narrowed_tag = super::util::helper_tag_for_tokens(tokens, "castable");
    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    EffectAst::Sequence {
        effects: vec![
            EffectAst::subject_verb_tag_matching_objects(
                filter,
                vec![Zone::Exile],
                narrowed_tag.clone(),
            ),
            grant(narrowed_tag),
        ],
    }
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
    if !permission_subject_facts::parse_spell_subject_facts(filter_tokens).contains_spell {
        return Ok(None);
    }
    if !allow_singular_spell_filter && free_cast_filter_mentions_singular_spell(filter_tokens) {
        return Ok(None);
    }

    let mut filter = ObjectFilter::nonland();
    let parsed_filter =
        permission_subject_facts::parse_permission_subject_filter_tokens(filter_tokens)?
            .unwrap_or_else(|| parse_spell_filter_with_grammar_entrypoint_lexed(filter_tokens));
    merge_spell_filters(&mut filter, parsed_filter);
    if let Some(comparison_tokens) = mana_value_comparison_tokens {
        let Some((operator, rhs_tokens)) = parse_value_comparison_tokens(comparison_tokens) else {
            return Ok(None);
        };
        let Some((rhs_value, used)) = parse_value_prefix_lexed(rhs_tokens) else {
            return Ok(None);
        };
        if used != rhs_tokens.len() {
            return Ok(None);
        }
        filter.mana_value = Some(mana_value_filter_comparison(
            comparison_tokens,
            operator,
            rhs_value,
        ));
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

fn parse_graveyard_cast_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::costs::Cost>, CardTextError> {
    let Some(fact) = permission_graveyard_facts::parse_graveyard_additional_cost_tokens(tokens)
    else {
        return Ok(None);
    };
    match fact {
        permission_graveyard_facts::GraveyardAdditionalCostFact::Sacrifice { filter_tokens } => {
            let Some(filter) =
                permission_subject_facts::parse_permission_subject_filter_tokens(filter_tokens)?
            else {
                return Ok(None);
            };
            Ok(Some(crate::costs::Cost::sacrifice(filter.you_control())))
        }
        permission_graveyard_facts::GraveyardAdditionalCostFact::ExileCards {
            count,
            card_types,
        } => Ok(Some(crate::costs::Cost::exile_from_graveyard(
            count, card_types,
        ))),
    }
}

fn parse_source_graveyard_cast_additional_cost_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<SourceGraveyardCastAdditionalCost<'a>> {
    let fact = permission_graveyard_facts::parse_source_graveyard_additional_cost_tokens(tokens)?;
    Some(SourceGraveyardCastAdditionalCost {
        cost_tokens: fact.cost_tokens,
    })
}

fn parse_source_cast_permission_tokens(tokens: &[OwnedLexToken]) -> Option<SourceCastPermission> {
    let fact = permission_graveyard_facts::parse_source_cast_permission_tokens(tokens)?;
    Some(SourceCastPermission { zone: fact.zone })
}

fn parse_source_graveyard_die_roll_cast_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceGraveyardDieRollCastPermission> {
    let fact = permission_graveyard_facts::parse_source_graveyard_die_roll_cast_tokens(tokens)?;
    Some(SourceGraveyardDieRollCastPermission {
        result: fact.result,
    })
}

fn parse_once_each_turn_graveyard_cast_permission(
    tokens: &[OwnedLexToken],
) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some(parsed) = parse_once_each_turn_graveyard_cast_rest_tokens(tokens) else {
        return Ok(None);
    };
    let Some(filter) =
        permission_subject_facts::parse_permission_subject_filter_tokens(parsed.subject_tokens)?
    else {
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
    let fact =
        permission_graveyard_facts::parse_once_each_turn_top_library_shared_type_tokens(tokens)?;
    Some(OnceEachTurnTopLibrarySharedTypeCast {
        subject_tokens: fact.subject_tokens,
        source_reference_tokens: fact.source_reference_tokens,
    })
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

    if !allow_land
        && prefixed_lifetime.is_none()
        && let Some(parsed) =
            permission_source_exiled_facts::parse_spells_from_source_exiled_tokens(rest_tokens)
        && trim_lexed_commas(parsed.tail_tokens).is_empty()
    {
        let Some(mut filter) =
            permission_subject_facts::parse_cast_permission_filter_tokens(parsed.subject_tokens)?
        else {
            return Ok(None);
        };
        mark_generic_spell_filter_nonland(&mut filter, parsed.subject_tokens);
        filter.zone = Some(Zone::Exile);
        if parsed.owned_by_you {
            filter.owner = Some(PlayerFilter::You);
        }
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        let spec =
            crate::grant::GrantSpec::new(crate::grant::Grantable::play_from(), filter, Zone::Exile)
                .with_source_exiled_surface(crate::grant::SourceExiledGrantSurface {
                    source: parsed.reference.surface,
                    plural_spell_subject: true,
                    generic_card_pool: true,
                    generic_cast_this_way_subject: true,
                });
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: PermissionLifetime::Static,
        }));
    }

    if prefixed_lifetime.is_none()
        && !allow_land
        && matches!(player, PlayerAst::Implicit | PlayerAst::You)
        && rest_is_singular_free_cast_from_hand(rest_tokens)
    {
        return Ok(None);
    }

    let filtered_tagged_target = parse_filtered_spells_from_among_tagged_tokens(rest_tokens)?
        .map(|(target_ref, tail, filter)| (target_ref, tail, Some(filter)));
    if let Some((target_ref, tagged_tail_tokens, filter)) = filtered_tagged_target
        .or_else(|| parse_spell_from_among_source_exiled_tokens(rest_tokens))
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
                    PermissionLifetime::UntilYourNextEndStep => "until-next-end-step",
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

        let mut target_surface = target_ref
            .surface
            .clone()
            .or_else(|| tagged_permission_target_surface(target_tokens));
        if tail.from_exile
            && target_surface == Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard)
        {
            target_surface = Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCardFromExile);
        }
        if matches!(
            lifetime,
            PermissionLifetime::ThisTurn
                | PermissionLifetime::UntilEndOfTurn
                | PermissionLifetime::UntilYourNextTurn
                | PermissionLifetime::UntilYourNextEndStep
                | PermissionLifetime::ForAsLongAsExiled
                | PermissionLifetime::ForAsLongAsYouControlSource
        ) && target_ref.as_copy
        {
            let label = match lifetime {
                PermissionLifetime::UntilYourNextTurn => "until-next-turn",
                PermissionLifetime::UntilYourNextEndStep => "until-next-end-step",
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
                target_surface.as_ref(),
                Some(
                    ironsmith_core::GrantPlayTaggedObjectSurface::It
                        | ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard
                        | ironsmith_core::GrantPlayTaggedObjectSurface::ThatCardFromExile
                        | ironsmith_core::GrantPlayTaggedObjectSurface::ThatSpell
                        | ironsmith_core::GrantPlayTaggedObjectSurface::Them
                        | ironsmith_core::GrantPlayTaggedObjectSurface::ThoseCards
                        | ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseCards
                        | ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseExiledCards
                        | ironsmith_core::GrantPlayTaggedObjectSurface::SpellFromAmongCardsExiledWithSource {
                            ..
                        }
                )
            )
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported temporary play/cast permission clause with alternative cost (clause: '{}')",
                clause_refs.join(" ")
            )));
        }
        if matches!(
            lifetime,
            PermissionLifetime::UntilYourNextTurn | PermissionLifetime::UntilYourNextEndStep
        ) && without_paying_mana_cost
        {
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

        let leading_duration = prefixed_lifetime.is_some();
        let surface = (leading_duration
            || target_surface.is_some()
            || lifetime == PermissionLifetime::ForAsLongAsYouControlSource)
            .then(|| {
                let mut surface = ironsmith_core::GrantPlayTaggedSurface::default()
                    .with_leading_duration(leading_duration);
                if let Some(object) = target_surface {
                    surface = surface.with_object(object);
                }
                if lifetime == PermissionLifetime::ForAsLongAsYouControlSource {
                    surface = surface.with_control_source(
                        ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                            "this creature".to_string(),
                        ),
                    );
                }
                surface
            });

        return Ok(Some(PermissionClauseSpec::Tagged {
            tag: target_ref.tag,
            player,
            allow_land,
            as_copy: target_ref.as_copy,
            max_plays: target_ref.max_plays,
            without_paying_mana_cost,
            lifetime,
            filter,
            surface,
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
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..ObjectFilter::default()
                },
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
        let filter = if matches!(
            permission_subject_facts::parse_exact_permission_subject(subject_tokens),
            Some(
                permission_subject_facts::ExactPermissionSubject::GenericSpell
                    | permission_subject_facts::ExactPermissionSubject::GenericSpells
            )
        ) {
            ObjectFilter::default()
        } else {
            let Some(mut spell_filter) =
                permission_subject_facts::parse_permission_subject_filter_tokens(subject_tokens)?
            else {
                return Ok(None);
            };
            exclude_lands_from_spell_filter(&mut spell_filter);
            ObjectFilter {
                any_of: vec![
                    ObjectFilter {
                        card_types: vec![CardType::Land],
                        ..ObjectFilter::default()
                    },
                    spell_filter,
                ],
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

    // This compact permission wording combines land-play and spell-cast
    // portions in one graveyard clause. It is a source-wide static
    // permission, not a tagged temporary play effect.
    if allow_land
        && token_word_refs(rest_tokens)
            == [
                "lands",
                "and",
                "cast",
                "spells",
                "from",
                "your",
                "graveyard",
            ]
    {
        return Ok(Some(PermissionClauseSpec::GrantBySpec {
            player,
            spec: crate::grant::GrantSpec::play_from_graveyard(),
            lifetime: prefixed_lifetime.unwrap_or(PermissionLifetime::Static),
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
            let mut filter = if matches!(
                permission_subject_facts::parse_exact_permission_subject(subject_tokens),
                Some(
                    permission_subject_facts::ExactPermissionSubject::GenericSpell
                        | permission_subject_facts::ExactPermissionSubject::GenericSpells
                )
            ) {
                ObjectFilter::default()
            } else if let Some(filter) =
                permission_subject_facts::parse_permission_subject_filter_tokens(subject_tokens)?
            {
                filter
            } else {
                return Ok(None);
            };
            exclude_lands_from_spell_filter(&mut filter);
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
            let spec =
                if permission_subject_facts::parse_exact_permission_subject(parsed.filter_tokens)
                    == Some(permission_subject_facts::ExactPermissionSubject::GenericSpells)
                {
                    crate::grant::GrantSpec::flash_to_spells()
                } else if permission_subject_facts::parse_exact_permission_subject(
                    parsed.filter_tokens,
                ) == Some(
                    permission_subject_facts::ExactPermissionSubject::NoncreatureSpells,
                ) {
                    crate::grant::GrantSpec::flash_to_noncreature_spells()
                } else if let Some(filter) =
                    permission_subject_facts::parse_permission_subject_filter_tokens(
                        parsed.filter_tokens,
                    )?
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

    if !allow_land
        && matches!(
            prefixed_lifetime,
            None | Some(
                PermissionLifetime::ThisTurn
                    | PermissionLifetime::UntilEndOfTurn
                    | PermissionLifetime::UntilYourNextTurn
            )
        )
    {
        if let Some(spec) = parse_static_hand_free_cast_grant_spec_from_rest(rest_tokens)? {
            if rest_is_singular_free_cast_from_hand(rest_tokens) {
                return Ok(None);
            }
            return Ok(Some(PermissionClauseSpec::GrantBySpec {
                player,
                spec,
                lifetime: prefixed_lifetime.unwrap_or(PermissionLifetime::Static),
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
        None => {}
    }

    let _ = parse_permission_clause_spec_lexed(tokens)?;
    Ok(None)
}

pub(crate) fn parse_until_end_of_turn_may_play_tagged_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let trimmed = trim_commas(tokens);
    let mana_suffix = strip_allow_any_color_for_cast_suffix_tokens(&trimmed);
    let mana_spend_mode = mana_suffix
        .as_ref()
        .map(|fact| fact.mana_spend_mode)
        .unwrap_or_default();
    let mana_reference = mana_suffix.map(|fact| fact.reference);
    match parse_permission_clause_spec(tokens)? {
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost,
            lifetime: PermissionLifetime::UntilEndOfTurn,
            filter,
            surface,
            ..
        }) if player == PlayerAst::You => Ok(Some(build_temporary_tagged_permission_effect(
            &trimmed,
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            mana_spend_mode,
            with_mana_reference_surface(surface, mana_reference),
            filter,
        ))),
        _ => Ok(None),
    }
}

pub(crate) fn parse_until_your_next_turn_may_play_tagged_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let trimmed = trim_commas(tokens);
    let mana_spend_mode = strip_allow_any_color_for_cast_suffix_tokens(&trimmed)
        .map(|fact| fact.mana_spend_mode)
        .unwrap_or_default();
    match parse_permission_clause_spec(tokens)? {
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land: true,
            as_copy: false,
            max_plays,
            without_paying_mana_cost: false,
            lifetime,
            ..
        }) if matches!(
            lifetime,
            PermissionLifetime::UntilYourNextTurn | PermissionLifetime::UntilYourNextEndStep
        ) && matches!(player, PlayerAst::You | PlayerAst::Implicit) =>
        {
            Ok(Some(
                if lifetime == PermissionLifetime::UntilYourNextEndStep {
                    EffectAst::subject_verb_grant_play_tagged_until_your_next_end_step(
                        tag,
                        PlayerAst::You,
                        true,
                        mana_spend_mode,
                    )
                    .with_tagged_play_max_plays(max_plays)
                } else {
                    EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                        tag,
                        PlayerAst::You,
                        true,
                        mana_spend_mode,
                    )
                    .with_tagged_play_max_plays(max_plays)
                },
            ))
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
    let Some(parsed) = parse_additional_land_play_clause(tokens) else {
        return Ok(None);
    };
    let count_tokens = parsed.count_tokens;
    let count_words = token_word_refs(count_tokens);
    let Some((count, used)) = parse_value_prefix_lexed(count_tokens) else {
        return Ok(None);
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

fn rest_is_any_number_free_cast_from_hand(rest_tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(rest_tokens);
    words
        .windows(3)
        .any(|window| window == ["any", "number", "of"])
        && parse_free_cast_from_your_zone_rest_tokens(rest_tokens)
            .is_some_and(|parsed| parsed.zone == Zone::Hand)
}

fn parse_any_number_free_cast_from_hand_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some((lead, rest_tokens)) = parse_permission_lead_tokens(tokens) else {
        return Ok(None);
    };
    if lead.allow_land
        || !matches!(lead.player, PlayerAst::Implicit | PlayerAst::You)
        || !rest_is_any_number_free_cast_from_hand(rest_tokens)
    {
        return Ok(None);
    }
    let Some(spec) = parse_hand_free_cast_grant_spec_from_rest(rest_tokens, false)? else {
        return Ok(None);
    };

    // "Cast any number" is a choice of an arbitrary subset, not a one-shot
    // cast permission. Iterate the eligible hand cards and offer each cast
    // independently; this keeps both zero and multiple casts executable.
    let mut filter = spec.filter;
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(crate::target::PlayerFilter::You);
    Ok(Some(EffectAst::ForEachObject {
        filter,
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::subject_verb_cast_tagged(
                TagKey::from(IT_TAG),
                lead.player,
                false,
                false,
                true,
                None,
            )],
        }],
    }))
}

fn mana_value_filter_comparison(
    comparison_tokens: &[OwnedLexToken],
    operator: ValueComparisonOperator,
    mut rhs_value: Value,
) -> crate::filter::Comparison {
    let comparison_words = token_word_refs(comparison_tokens);
    if starts_explicit_ordered_comparison(&comparison_words, operator)
        && !matches!(rhs_value.unhinted(), Value::Fixed(_))
    {
        rhs_value =
            rhs_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ExplicitComparison);
    }
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

        let filter_tokens = parsed.filter_tokens;
        let Some(mut filter) = parse_simple_spell_type_list_filter(filter_tokens)
            .or(permission_subject_facts::parse_cast_permission_filter_tokens(filter_tokens)?)
        else {
            return Ok(None);
        };
        mark_generic_spell_filter_nonland(&mut filter, filter_tokens);
        filter.owner = Some(crate::target::PlayerFilter::You);

        if let Some(values) =
            permission_zone_facts::parse_mana_value_one_of_tokens(parsed.comparison_tokens)
        {
            filter.mana_value = Some(crate::filter::Comparison::OneOf(values));
        } else {
            let Some((operator, rhs_tokens)) =
                parse_value_comparison_tokens(parsed.comparison_tokens)
            else {
                return Ok(None);
            };
            let Some((rhs_value, used)) = parse_value_prefix_lexed(rhs_tokens) else {
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
                filter.mana_value = Some(mana_value_filter_comparison(
                    parsed.comparison_tokens,
                    operator,
                    rhs_value,
                ));
            }
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

    // A singular spell permission with an unqualified "equal or lesser mana
    // value" compares against the object established by the immediately
    // preceding instruction. Keep that relation typed instead of feeding the
    // word `or` to the ordinary object-filter union parser.
    if matches!(
        token_word_refs(rest_tokens).as_slice(),
        [
            "a",
            "spell",
            "with",
            "equal",
            "or",
            "lesser" | "less",
            "mana",
            "value",
            "from",
            "your",
            "hand",
            "without",
            "paying",
            "its",
            "mana",
            "cost"
        ]
    ) {
        let mut filter = ObjectFilter::nonland()
            .owned_by(crate::target::PlayerFilter::You)
            .match_tagged(
                TagKey::from(IT_TAG),
                crate::filter::TaggedOpbjectRelation::ManaValueLteTagged,
            );
        filter.union_surface = filter.union_surface.with_equal_or_lesser_mana_value(true);
        return Ok(Some(
            EffectAst::may_cast_matching_spell_without_paying_mana_cost(
                lead.player,
                filter,
                Zone::Hand,
            ),
        ));
    }

    if let Some(parsed) = parse_command_zone_free_cast_rest_tokens(rest_tokens) {
        if permission_subject_facts::parse_exact_permission_subject(parsed.filter_tokens)
            == Some(permission_subject_facts::ExactPermissionSubject::YourCommander)
        {
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
        permission_subject_facts::parse_simple_spell_type_list_filter_tokens,
    )? {
        return Ok(Some(effect));
    }

    if let Some(parsed) = parse_free_cast_from_your_zone_rest_tokens(rest_tokens) {
        let filter_tokens = parsed.filter_tokens;
        let Some(mut filter) =
            permission_subject_facts::parse_cast_permission_filter_tokens(filter_tokens)?
        else {
            return Ok(None);
        };
        mark_generic_spell_filter_nonland(&mut filter, filter_tokens);
        filter.owner = Some(crate::target::PlayerFilter::You);
        if lead.player == PlayerAst::Implicit
            && parsed.zone == Zone::Graveyard
            && !permission_subject_facts::parse_spell_subject_facts(filter_tokens).contains_spell
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
    let filter_tokens = parsed.filter_tokens;
    let Some(mut filter) =
        permission_subject_facts::parse_cast_permission_filter_tokens(filter_tokens)?
    else {
        return Ok(None);
    };
    mark_generic_spell_filter_nonland(&mut filter, filter_tokens);
    filter.owner = Some(crate::target::PlayerFilter::You);

    if let Some(values) =
        permission_zone_facts::parse_mana_value_one_of_tokens(parsed.comparison_tokens)
    {
        filter.mana_value = Some(crate::filter::Comparison::OneOf(values));
        return Ok(Some(
            EffectAst::may_cast_matching_spell_without_paying_mana_cost(
                lead.player,
                filter,
                parsed.zone,
            ),
        ));
    }

    let Some((operator, rhs_tokens)) = parse_value_comparison_tokens(parsed.comparison_tokens)
    else {
        return Ok(None);
    };
    let Some((rhs_value, used)) = parse_value_prefix_lexed(rhs_tokens) else {
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
            filter.mana_value = Some(mana_value_filter_comparison(
                parsed.comparison_tokens,
                operator,
                rhs_value,
            ));
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

    if let Some(effect) = parse_until_source_exiles_another_permission(&trimmed) {
        return Ok(Some(effect));
    }

    if let Some(shape) = super::grammar::effects::clause_dispatch_shapes::parse_cast_target_from_your_graveyard_this_turn_shape(&trimmed)
    {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                    TagKey::from(IT_TAG),
                    PlayerAst::You,
                    false,
                    false,
                    false,
                ),
            ],
        }));
    }

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

    let (mana_spend_mode, mana_reference) = if let Some((body_len, mode, reference)) =
        strip_allow_any_color_for_cast_suffix_tokens(&trimmed).map(|parsed| {
            (
                parsed.body_tokens.len(),
                parsed.mana_spend_mode,
                parsed.reference,
            )
        }) {
        trimmed.truncate(body_len);
        (mode, Some(reference))
    } else {
        (ironsmith_core::value_model::ManaSpendMode::Normal, None)
    };

    if let Some(effect) = parse_any_number_free_cast_from_hand_clause(&trimmed)? {
        return Ok(Some(effect));
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
                            mana_spend_mode,
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
            filter,
            ..
        }) => {
            let (tag, narrowing) = if let Some(mut filter) = filter {
                let narrowed_tag = super::util::helper_tag_for_tokens(&trimmed, "castable");
                filter.zone = Some(Zone::Exile);
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag,
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
                (
                    narrowed_tag.clone(),
                    Some(EffectAst::subject_verb_tag_matching_objects(
                        filter,
                        vec![Zone::Exile],
                        narrowed_tag,
                    )),
                )
            } else {
                (tag, None)
            };
            let cast = EffectAst::subject_verb_cast_tagged(
                tag,
                player,
                allow_land,
                as_copy,
                without_paying_mana_cost,
                None,
            );
            let cast = if matches!(player, PlayerAst::Implicit | PlayerAst::You) {
                cast
            } else {
                EffectAst::MayByPlayer {
                    player,
                    effects: vec![cast],
                }
            };
            if let Some(narrowing) = narrowing {
                Ok(Some(EffectAst::Sequence {
                    effects: vec![narrowing, cast],
                }))
            } else {
                Ok(Some(cast))
            }
        }
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost,
            lifetime: PermissionLifetime::ThisTurn | PermissionLifetime::UntilEndOfTurn,
            filter,
            surface,
            ..
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => {
            let surface = with_mana_reference_surface(surface, mana_reference);
            Ok(Some(build_temporary_tagged_permission_effect(
                &trimmed,
                tag,
                PlayerAst::Implicit,
                allow_land,
                without_paying_mana_cost,
                mana_spend_mode,
                surface,
                filter,
            )))
        }
        Some(PermissionClauseSpec::Tagged {
            tag,
            player,
            allow_land,
            as_copy: false,
            max_plays,
            without_paying_mana_cost: false,
            lifetime,
            ..
        }) if matches!(
            lifetime,
            PermissionLifetime::UntilYourNextTurn | PermissionLifetime::UntilYourNextEndStep
        ) && (player == PlayerAst::Implicit || player == PlayerAst::You) =>
        {
            Ok(Some(
                if lifetime == PermissionLifetime::UntilYourNextEndStep {
                    EffectAst::subject_verb_grant_play_tagged_until_your_next_end_step(
                        tag,
                        PlayerAst::Implicit,
                        allow_land,
                        mana_spend_mode,
                    )
                    .with_tagged_play_max_plays(max_plays)
                } else {
                    EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                        tag,
                        PlayerAst::Implicit,
                        allow_land,
                        mana_spend_mode,
                    )
                    .with_tagged_play_max_plays(max_plays)
                },
            ))
        }
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
            ..
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
                    mana_spend_mode,
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
            surface,
            ..
        }) if player == PlayerAst::Implicit || player == PlayerAst::You => Ok(Some(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_you_control_source(
                tag,
                PlayerAst::Implicit,
                allow_land,
                mana_spend_mode,
                surface,
            ),
        )),
        _ => Ok(conditional_tagged_permission),
    }
}

#[cfg(test)]
mod source_exile_duration_tests {
    use super::*;
    use crate::runtime_backend::ast::{SubjectVerbActionAst, SubjectVerbEffectAst};
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn authored_from_exile_survives_temporary_tagged_play_permission() {
        let tokens = lex_line("You may play that card from exile this turn", 0)
            .expect("permission should lex");
        let effect = parse_cast_or_play_tagged_clause(&tokens)
            .expect("permission parsing should not error")
            .expect("tagged permission should parse");
        assert!(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    allow_land: true,
                    surface: Some(ironsmith_core::GrantPlayTaggedSurface {
                        object: Some(
                            ironsmith_core::GrantPlayTaggedObjectSurface::ThatCardFromExile
                        ),
                        ..
                    }),
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn equal_or_lesser_hand_spell_permission_keeps_one_nonland_tagged_filter() {
        let text = "You may cast a spell with equal or lesser mana value from your hand without paying its mana cost";
        let tokens = lex_line(text, 0).expect("permission should lex");
        let effect = parse_cast_or_play_tagged_clause(&tokens)
            .expect("permission parsing should not error")
            .expect("permission should parse");
        let EffectAst::MayCastMatchingSpellWithoutPayingManaCost { filter, zone, .. } = effect
        else {
            panic!("expected one matching-spell cast permission: {effect:#?}");
        };
        assert_eq!(zone, Zone::Hand);
        assert_eq!(filter.excluded_card_types, [crate::types::CardType::Land]);
        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert_eq!(filter.tagged_constraints.len(), 1, "{filter:#?}");
        assert!(filter.union_surface.equal_or_lesser_mana_value());
        let constraint = &filter.tagged_constraints[0];
        assert_eq!(constraint.tag, TagKey::from(IT_TAG));
        assert_eq!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::ManaValueLteTagged
        );
    }

    #[test]
    fn equal_or_lesser_permission_near_miss_keeps_its_authored_zone() {
        let text = "You may cast a spell with equal or lesser mana value from your graveyard without paying its mana cost";
        let tokens = lex_line(text, 0).expect("permission should lex");
        let effect = parse_cast_or_play_tagged_clause(&tokens)
            .expect("permission parsing should not error")
            .expect("ordinary graveyard permission should remain parseable");
        let debug = format!("{effect:#?}");
        assert!(debug.contains("Graveyard"), "{debug}");
        assert!(!debug.contains("zone: Hand"), "{debug}");
    }

    #[test]
    fn narset_spell_pool_is_narrowed_before_play_and_free_cast_grants() {
        let text = "Until end of turn, you may cast noncreature spells from among those cards without paying their mana costs";
        let tokens = lex_line(text, 0).expect("permission should lex");
        let effect = parse_cast_or_play_tagged_clause(&tokens)
            .expect("permission parsing should not error")
            .expect("permission should parse");
        let debug = format!("{effect:#?}");
        let compact = debug.split_whitespace().collect::<String>();
        assert!(debug.contains("TagMatchingObjects"), "{debug}");
        assert!(
            compact.contains("excluded_card_types:[Creature,Land,]"),
            "{debug}"
        );
        assert!(
            debug.contains("GrantPlayTaggedUntilEndOfTurn")
                && debug.contains("without_paying_mana_cost: true"),
            "{debug}"
        );
        assert!(debug.contains("SpellsFromAmongThoseCards"), "{debug}");
    }

    #[test]
    fn event_bounded_play_permission_stays_typed_through_family_parsing() {
        let cases = [
            (
                "You may play that card until you exile another card with this enchantment",
                "this enchantment",
            ),
            (
                "You may play it until you exile another card with this artifact",
                "this artifact",
            ),
            (
                "You may play that card until you exile another card with this creature",
                "this creature",
            ),
        ];
        for (text, source_surface) in cases {
            let tokens = lex_line(text, 0).expect("permission should lex");
            let effect = parse_cast_or_play_tagged_clause(&tokens)
                .expect("permission parsing should not error")
                .expect("permission should parse");
            let debug = format!("{effect:#?}");
            assert!(
                debug.contains("until_source_exiles_another: true"),
                "{text}: {debug}"
            );
            assert!(debug.contains(source_surface), "{text}: {debug}");
            assert!(
                debug.contains("allow_land: true"),
                "play permission must include lands: {debug}"
            );
        }
    }

    #[test]
    fn source_control_duration_preserves_authored_source_noun() {
        let text = "You may play that card for as long as you control this creature";
        let tokens = lex_line(text, 0).expect("permission should lex");
        let effect = parse_cast_or_play_tagged_clause(&tokens)
            .expect("permission parsing should not error")
            .expect("permission should parse");
        let debug = format!("{effect:#?}");
        assert!(
            debug.contains("GrantPlayTaggedForAsLongAsYouControlSource"),
            "{debug}"
        );
        assert!(debug.contains("control_source"), "{debug}");
        assert!(debug.contains("this creature"), "{debug}");
    }
}
