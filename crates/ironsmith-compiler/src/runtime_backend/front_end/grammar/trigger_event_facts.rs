use crate::cards::builders::CardTextError;
use crate::model::ast::{EffectAst, PredicateAst, TriggerSpec};
use crate::model::symbols::Cardinality;
use crate::model::{
    CompilerControlFlowAst, CompilerTriggerEventAst, CompilerTriggeredAbilityAst,
    ConditionPositionAst, ControlConditionAst, ControlFlowNodeAst, ControlFlowSemanticAst,
    ControlPredicateAst, DelayedScheduleAst, LinkedTriggerEffectAst, NestedProgramAst,
    NestedProgramKindAst, TriggerBindingsAst, TriggerFrequencyAst, TriggerKindAst,
    TriggerReferenceAst, TriggerReferenceSurfaceAst, TriggerSubjectAst, TriggerZoneTransitionAst,
};
use crate::parse_context::ParseContextView;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use super::super::lexer::OwnedLexToken;
use super::{primitives, trigger_surface};

const TRIGGER_REFERENCE_PHRASES: &[(&[&str], TriggerReferenceSurfaceAst)] = &[
    (
        &["the", "sacrificed"],
        TriggerReferenceSurfaceAst::SacrificedObject,
    ),
    (
        &["triggering", "object"],
        TriggerReferenceSurfaceAst::TriggeringObject,
    ),
    (&["those"], TriggerReferenceSurfaceAst::Those),
    (&["that"], TriggerReferenceSurfaceAst::That),
    (&["it"], TriggerReferenceSurfaceAst::It),
];

pub(crate) fn build_compiler_triggered_ability(
    context: ParseContextView<'_>,
    full_tokens: &[OwnedLexToken],
    result_tokens: &[OwnedLexToken],
    semantics: TriggerSpec,
    effects: Vec<EffectAst>,
    intervening_if: Option<PredicateAst>,
    max_triggers_per_turn: Option<u32>,
    functional_zones: Vec<Zone>,
) -> Result<CompilerTriggeredAbilityAst, CardTextError> {
    let intro =
        trigger_surface::parse_trigger_intro_surface_tokens(full_tokens).ok_or_else(|| {
            CardTextError::ParseError("typed trigger event is missing an introducer".to_string())
        })?;
    let reference_surfaces = parse_trigger_reference_surfaces(result_tokens);
    let object_cardinality = triggering_object_cardinality(&semantics)
        .or_else(|| (!reference_surfaces.is_empty()).then_some(Cardinality::ExactlyOne));
    let bindings =
        TriggerBindingsAst::allocate(context, object_cardinality, None).map_err(|error| {
            CardTextError::ParseError(format!("trigger symbol binding failed: {error:?}"))
        })?;
    let references: Vec<TriggerReferenceAst> = bindings
        .triggering_object
        .map(|reference| {
            reference_surfaces
                .into_iter()
                .map(|surface| TriggerReferenceAst { surface, reference })
                .collect()
        })
        .unwrap_or_default();
    let zones = trigger_zone_transition(&semantics);
    let kind = trigger_kind(full_tokens, &semantics);
    let frequency = if kind == TriggerKindAst::State {
        TriggerFrequencyAst::StateUntilFalse
    } else {
        max_triggers_per_turn
            .map(TriggerFrequencyAst::AtMostPerTurn)
            .unwrap_or(TriggerFrequencyAst::EachOccurrence)
    };
    let linked_effects = if references.is_empty() {
        Vec::new()
    } else {
        effects
            .iter()
            .enumerate()
            .map(|(effect_index, _)| LinkedTriggerEffectAst {
                effect_index,
                triggering_object: bindings.triggering_object,
                triggering_event: bindings.triggering_event,
            })
            .collect()
    };
    let program = if let Some(predicate) = intervening_if.clone() {
        CompilerControlFlowAst::new(
            ControlFlowSemanticAst::ControlFlow,
            ControlFlowNodeAst::Condition {
                condition: ControlConditionAst {
                    position: ConditionPositionAst::InterveningCondition,
                    predicate: ControlPredicateAst::State(predicate),
                    negated_surface: false,
                    provenance: None,
                },
                consequence_program: 0,
                alternative_program: None,
                reflexive: false,
            },
            vec![NestedProgramAst::new(
                NestedProgramKindAst::Consequence,
                effects.clone(),
            )],
            None,
        )
    } else {
        CompilerControlFlowAst::new(
            ControlFlowSemanticAst::ControlFlow,
            ControlFlowNodeAst::Delayed {
                schedule: DelayedScheduleAst::Event,
                duration: None,
                program: 0,
                one_shot: false,
                reflexive: kind == TriggerKindAst::Reflexive,
                watched_references: bindings.triggering_object.into_iter().collect(),
            },
            vec![NestedProgramAst::new(
                if kind == TriggerKindAst::Reflexive {
                    NestedProgramKindAst::Reflexive
                } else {
                    NestedProgramKindAst::Delayed
                },
                effects.clone(),
            )],
            None,
        )
    }
    .map_err(|error| {
        CardTextError::InvariantViolation(format!(
            "failed to build branch-scoped trigger program: {error:?}"
        ))
    })?;

    Ok(CompilerTriggeredAbilityAst {
        event: CompilerTriggerEventAst {
            intro,
            kind,
            subject: trigger_subject(&semantics),
            zones,
            condition: event_qualification(&semantics),
            frequency,
            semantics,
            bindings,
            provenance: None,
        },
        program,
        effects,
        intervening_if,
        linked_effects,
        references,
        functional_zones,
        provenance: None,
    })
}

fn core_semantics(trigger: &TriggerSpec) -> &TriggerSpec {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. }
        | TriggerSpec::ConditionQualified { trigger, .. } => core_semantics(trigger),
        trigger => trigger,
    }
}

fn event_qualification(trigger: &TriggerSpec) -> Option<PredicateAst> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => event_qualification(trigger),
        TriggerSpec::ConditionQualified { condition, .. } => Some(condition.clone()),
        _ => None,
    }
}

fn trigger_kind(full_tokens: &[OwnedLexToken], trigger: &TriggerSpec) -> TriggerKindAst {
    if starts_with_phrase(full_tokens, &["when", "you", "do"])
        || starts_with_phrase(full_tokens, &["when", "you", "don't"])
        || starts_with_phrase(full_tokens, &["when", "you", "dont"])
    {
        return TriggerKindAst::Reflexive;
    }

    match core_semantics(trigger) {
        TriggerSpec::StateBased { .. } => TriggerKindAst::State,
        TriggerSpec::ThisDies
        | TriggerSpec::ThisDiesOrIsExiled
        | TriggerSpec::ThisDiesOrIsExiledWithSurface(_)
        | TriggerSpec::Dies(_)
        | TriggerSpec::DiesOneOrMore(_)
        | TriggerSpec::DiesDuringTurn { .. }
        | TriggerSpec::DiesDuringCombat { .. }
        | TriggerSpec::HauntedCreatureDies => TriggerKindAst::Dies,
        trigger if trigger_zone_transition(trigger).is_some() => TriggerKindAst::ZoneChange,
        _ => TriggerKindAst::Normal,
    }
}

fn trigger_subject(trigger: &TriggerSpec) -> TriggerSubjectAst {
    match core_semantics(trigger) {
        TriggerSpec::Attacks(filter)
        | TriggerSpec::AttacksAndIsntBlocked(filter)
        | TriggerSpec::AttacksWhileSaddled(filter)
        | TriggerSpec::AttacksOneOrMore(filter)
        | TriggerSpec::AttacksAlone(filter)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControl(filter)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(filter)
        | TriggerSpec::Blocks(filter)
        | TriggerSpec::BlocksOneOrMore(filter)
        | TriggerSpec::BecomesBlocked(filter)
        | TriggerSpec::ThisBecomesBlockedByObject(filter)
        | TriggerSpec::PermanentBecomesTapped(filter)
        | TriggerSpec::TurnedFaceUp(filter)
        | TriggerSpec::BecomesTargeted(filter)
        | TriggerSpec::ThisBecomesTargetedBySpell(filter)
        | TriggerSpec::ThisBecomesTargetedByStackObject(filter)
        | TriggerSpec::ThisDealsDamageTo(filter)
        | TriggerSpec::ThisDealsCombatDamageTo(filter)
        | TriggerSpec::IsDealtDamage(filter)
        | TriggerSpec::IsDealtCombatDamage(filter)
        | TriggerSpec::IsDealtExcessNoncombatDamage(filter)
        | TriggerSpec::LeavesBattlefield(filter)
        | TriggerSpec::Dies(filter)
        | TriggerSpec::DiesOneOrMore(filter)
        | TriggerSpec::PutIntoGraveyard(filter)
        | TriggerSpec::PutIntoGraveyardOneOrMore(filter) => {
            TriggerSubjectAst::Object(filter.clone())
        }
        TriggerSpec::SpellCast {
            filter: Some(filter),
            ..
        }
        | TriggerSpec::SpellCopied {
            filter: Some(filter),
            ..
        }
        | TriggerSpec::SpellCountered {
            filter: Some(filter),
            ..
        }
        | TriggerSpec::EntersBattlefield { filter, .. }
        | TriggerSpec::EntersBattlefieldOneOrMore { filter, .. }
        | TriggerSpec::EntersBattlefieldFromZone { filter, .. }
        | TriggerSpec::EntersBattlefieldTapped { filter, .. }
        | TriggerSpec::EntersBattlefieldUntapped { filter, .. }
        | TriggerSpec::PutIntoGraveyardFromZone { filter, .. }
        | TriggerSpec::PutIntoGraveyardFromAnyExcept { filter, .. }
        | TriggerSpec::PutIntoExileFromZones { filter, .. } => {
            TriggerSubjectAst::Object(filter.clone())
        }
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::PlayerLosesLife(player)
        | TriggerSpec::PlayersLoseLifeOneOrMore(player)
        | TriggerSpec::PlayerLosesGame(player)
        | TriggerSpec::PlayerDrawsCard(player)
        | TriggerSpec::PlayerDrawsCardExceptFirstInDrawStep(player)
        | TriggerSpec::PlayerGivesGift(player)
        | TriggerSpec::PlayerSearchesLibrary(player) => TriggerSubjectAst::Player(player.clone()),
        TriggerSpec::YouGainLife | TriggerSpec::YouDrawCard | TriggerSpec::YouCastThisSpell => {
            TriggerSubjectAst::Player(PlayerFilter::You)
        }
        _ => TriggerSubjectAst::Source,
    }
}

fn trigger_zone_transition(trigger: &TriggerSpec) -> Option<TriggerZoneTransitionAst> {
    match core_semantics(trigger) {
        TriggerSpec::ThisDies
        | TriggerSpec::Dies(_)
        | TriggerSpec::DiesOneOrMore(_)
        | TriggerSpec::DiesDuringTurn { .. }
        | TriggerSpec::DiesDuringCombat { .. }
        | TriggerSpec::HauntedCreatureDies => Some(TriggerZoneTransitionAst {
            from: Some(Zone::Battlefield),
            to: Some(Zone::Graveyard),
        }),
        TriggerSpec::ThisLeavesBattlefield
        | TriggerSpec::ThisLeavesBattlefieldWithSurface(_)
        | TriggerSpec::LeavesBattlefield(_)
        | TriggerSpec::LeavesBattlefieldWithoutDying { .. } => Some(TriggerZoneTransitionAst {
            from: Some(Zone::Battlefield),
            to: None,
        }),
        TriggerSpec::PutIntoGraveyardFromZone { from, .. } => Some(TriggerZoneTransitionAst {
            from: Some(*from),
            to: Some(Zone::Graveyard),
        }),
        TriggerSpec::PutIntoGraveyard(_)
        | TriggerSpec::PutIntoGraveyardOneOrMore(_)
        | TriggerSpec::PutIntoGraveyardFromAnyExcept { .. } => Some(TriggerZoneTransitionAst {
            from: None,
            to: Some(Zone::Graveyard),
        }),
        TriggerSpec::PutIntoExileFromZones { .. } => Some(TriggerZoneTransitionAst {
            from: None,
            to: Some(Zone::Exile),
        }),
        TriggerSpec::EntersBattlefieldFromZone { from, .. }
        | TriggerSpec::ThisEntersBattlefieldFromZone { from, .. } => {
            Some(TriggerZoneTransitionAst {
                from: Some(*from),
                to: Some(Zone::Battlefield),
            })
        }
        TriggerSpec::EntersBattlefield { .. }
        | TriggerSpec::EntersBattlefieldOneOrMore { .. }
        | TriggerSpec::EntersBattlefieldTapped { .. }
        | TriggerSpec::EntersBattlefieldUntapped { .. }
        | TriggerSpec::ThisEntersBattlefield { .. }
        | TriggerSpec::ThisEntersBattlefieldWithSurface { .. } => Some(TriggerZoneTransitionAst {
            from: None,
            to: Some(Zone::Battlefield),
        }),
        _ => None,
    }
}

fn triggering_object_cardinality(trigger: &TriggerSpec) -> Option<Cardinality> {
    match core_semantics(trigger) {
        TriggerSpec::BeginningOfUpkeep(_)
        | TriggerSpec::BeginningOfDrawStep(_)
        | TriggerSpec::BeginningOfCombat(_)
        | TriggerSpec::BeginningOfEndStep(_)
        | TriggerSpec::BeginningOfTheEndStep
        | TriggerSpec::BeginningOfMonarchEndStep
        | TriggerSpec::BeginningOfMainPhase { .. }
        | TriggerSpec::BeginningOfPrecombatMain(_)
        | TriggerSpec::BeginningOfPostcombatMain { .. }
        | TriggerSpec::YouGainLife
        | TriggerSpec::YouDrawCard
        | TriggerSpec::DayNightChanged
        | TriggerSpec::StateBased { .. } => None,
        TriggerSpec::AttacksOneOrMore(_)
        | TriggerSpec::AttacksOneOrMoreWithMinTotal { .. }
        | TriggerSpec::AttacksOneOrMoreWithExactTotal { .. }
        | TriggerSpec::AttacksOneOrMoreWithAggregate { .. }
        | TriggerSpec::BlocksOneOrMore(_)
        | TriggerSpec::DiesOneOrMore(_)
        | TriggerSpec::PutIntoGraveyardOneOrMore(_)
        | TriggerSpec::EntersBattlefieldOneOrMore { .. }
        | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. } => Some(Cardinality::OneOrMore),
        _ => Some(Cardinality::ExactlyOne),
    }
}

fn parse_trigger_reference_surfaces(tokens: &[OwnedLexToken]) -> Vec<TriggerReferenceSurfaceAst> {
    let mut references = Vec::new();
    for &(phrase, surface) in TRIGGER_REFERENCE_PHRASES {
        if primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some() {
            references.push(surface);
        }
    }
    references
}

fn starts_with_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(phrase)).is_some()
}
