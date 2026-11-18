use super::*;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::ChoiceActionAst;
use crate::cards::builders::ControlActionAst;
use crate::cards::builders::CounterActionAst;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::GameActionAst;
use crate::cards::builders::GrantActionAst;
use crate::cards::builders::KeywordActionAst;
use crate::cards::builders::LibraryActionAst;
use crate::cards::builders::PermissionEffectAst;
use crate::cards::builders::RandomActionAst;
use crate::cards::builders::RevealLookActionAst;
use crate::cards::builders::StackActionAst;
use crate::cards::builders::TurnStructureActionAst;

pub(super) fn target_ast_is_source(target: &TargetAst) -> bool {
    match target {
        TargetAst::Source(_) => true,
        TargetAst::Object(filter, _, _) => filter.source,
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_ast_is_source(inner)
        }
        _ => false,
    }
}

/// Keep one authored target declaration for a coordinated draw/life-loss X
/// clause whose shared basis names a single target player's zone. Isolated
/// value parsing can synthesize the same TargetOnly prelude once per X use;
/// the lexical one-target proof distinguishes that from two independently
/// authored target slots.
pub fn dedupe_shared_target_player_draw_lose_x(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let words = token_word_refs(tokens);
    if words.iter().filter(|word| **word == "target").count() != 1
        || words.iter().filter(|word| **word == "x").count() < 3
        || !crate::word_primitives::sequence_occurs(&words, &["where", "x", "is"])
    {
        return;
    }

    let mut target_only_count = 0usize;
    let mut nested_draw_values = Vec::new();
    let mut nested_lose_values = Vec::new();
    let mut inspect = |nested: &[EffectAst]| {
        for effect in nested {
            let EffectAst::SubjectVerb(subject_verb) = effect else {
                continue;
            };
            match &subject_verb.action {
                SubjectVerbActionAst::TargetOnly { .. } => target_only_count += 1,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count })
                    if matches!(
                        subject_verb.subject.player,
                        PlayerAst::You | PlayerAst::Implicit
                    ) =>
                {
                    nested_draw_values.push(count.clone());
                }
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
                    if matches!(
                        subject_verb.subject.player,
                        PlayerAst::You | PlayerAst::Implicit
                    ) =>
                {
                    nested_lose_values.push(amount.clone());
                }
                _ => {}
            }
        }
    };
    inspect(effects);
    for effect in effects.iter() {
        for_each_nested_effects(effect, true, &mut inspect);
    }
    drop(inspect);
    if target_only_count == 0
        && let ([draw_value], [lose_value]) =
            (nested_draw_values.as_slice(), nested_lose_values.as_slice())
        && draw_value.unhinted() == lose_value.unhinted()
        && matches!(
            draw_value.unhinted(),
            Value::Count(filter) if matches!(filter.owner, Some(PlayerFilter::Target(_)))
        )
    {
        effects.insert(
            0,
            EffectAst::subject_verb_explicit_target_only(TargetAst::Player(
                PlayerFilter::Any,
                span_from_tokens(tokens),
            )),
        );
        return;
    }

    fn bind_effect_list_references(effects: &mut Vec<EffectAst>, tokens: &[OwnedLexToken]) {
        let mut target: Option<TargetAst> = None;
        let mut target_indices = Vec::new();
        let mut draw_value: Option<Value> = None;
        let mut lose_value: Option<Value> = None;
        for (index, effect) in effects.iter().enumerate() {
            let EffectAst::SubjectVerb(subject_verb) = effect else {
                return;
            };
            match &subject_verb.action {
                SubjectVerbActionAst::TargetOnly {
                    target: candidate,
                    explicit_declaration: false,
                } => {
                    if let Some(existing) = &target {
                        if existing != candidate {
                            return;
                        }
                    } else {
                        target = Some(candidate.clone());
                    }
                    target_indices.push(index);
                }
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count })
                    if matches!(
                        subject_verb.subject.player,
                        crate::cards::builders::PlayerAst::You
                            | crate::cards::builders::PlayerAst::Implicit
                    ) =>
                {
                    if draw_value.replace(count.clone()).is_some() {
                        return;
                    }
                }
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
                    if matches!(
                        subject_verb.subject.player,
                        crate::cards::builders::PlayerAst::You
                            | crate::cards::builders::PlayerAst::Implicit
                    ) =>
                {
                    if lose_value.replace(amount.clone()).is_some() {
                        return;
                    }
                }
                _ => return,
            }
        }
        let (Some(draw_value), Some(lose_value)) = (draw_value, lose_value) else {
            return;
        };
        if draw_value.unhinted() != lose_value.unhinted() {
            return;
        }

        if target_indices.is_empty() {
            let value_uses_target_player_zone = matches!(
                draw_value.unhinted(),
                Value::Count(filter)
                    if matches!(filter.owner, Some(PlayerFilter::Target(_)))
            );
            if !value_uses_target_player_zone {
                return;
            }
            effects.insert(
                0,
                EffectAst::subject_verb_explicit_target_only(TargetAst::Player(
                    PlayerFilter::Any,
                    span_from_tokens(tokens),
                )),
            );
            return;
        }

        for index in target_indices.into_iter().skip(1).rev() {
            effects.remove(index);
        }
    }

    if let [EffectAst::Coordinated { effects: inner, .. }] = effects.as_mut_slice() {
        bind_effect_list_references(inner, tokens);
    } else {
        bind_effect_list_references(effects, tokens);
    }
}

pub(super) fn bind_source_exiled_effect(effect: EffectAst, bind: bool) -> EffectAst {
    if bind {
        EffectAst::TagAffected {
            effect: Box::new(effect),
            tag: crate::tag::CompilerReferenceTag::SourceExiled.bind(),
        }
    } else {
        effect
    }
}

pub fn parse_may_have_any_number_tagged_phase_out_lexed(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if !crate::word_primitives::parse_sequence_complete(
        &token_word_refs(tokens),
        &[
            "you", "may", "have", "any", "number", "of", "them", "phase", "out",
        ],
    ) {
        return None;
    }

    let chosen_tag = crate::tag::CompilerReferenceTag::PhaseOutSelection.bind();
    let mut available = ObjectFilter::default().in_zone(Zone::Battlefield);
    available
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let mut phase_out_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    phase_out_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: chosen_tag.clone().into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Some(EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
        player: PlayerAst::You,
        effects: vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: available,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag,
            }),
            EffectAst::subject_verb_phase_out_all(phase_out_filter),
        ],
    }))
}

pub fn collapse_for_each_player_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { .. }),
                EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                    effects: followup_effects,
                }),
            ) => effects_reference_it_tag(followup_effects),
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (
                EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                    effects: first_effects,
                }),
                EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                    effects: mut followup_effects,
                }),
            ) => {
                first_effects.append(&mut followup_effects);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

pub fn collapse_for_each_object_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (EffectAst::ForEach(ForEachEffectAst::ForEachObject { filter, .. }), followup) => {
                effects_reference_it_tag(std::slice::from_ref(followup))
                    || (for_each_revealed_this_way_filter(filter)
                        && is_revealed_this_way_scalar_reward(followup))
            }
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (
                EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects: inner, .. }),
                followup,
            ) => {
                inner.push(followup);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

pub(super) fn explicit_tagged_target(target: &TargetAst) -> Option<TagKey> {
    match target {
        TargetAst::Tagged(tag, _)
            if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() =>
        {
            Some(tag.clone().into())
        }
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .find(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
            })
            .map(|constraint| constraint.tag.clone()),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            explicit_tagged_target(inner)
        }
        _ => None,
    }
}

pub(super) fn explicit_effect_object_tag(effect: &EffectAst) -> Option<TagKey> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { target, .. })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { target, .. })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    target, ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield {
                    target, ..
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp { target })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceDown { target })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. }),
            ..
        }) => explicit_tagged_target(target),
        EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
            if effects.len() == 1 =>
        {
            explicit_effect_object_tag(&effects[0])
        }
        EffectAst::TagAffected { tag, .. }
            if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() =>
        {
            Some(tag.clone().into())
        }
        _ => None,
    }
}

pub(super) fn explicit_effect_object_target(effect: &EffectAst) -> Option<ChooseSpec> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { target, .. })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { target, .. })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    target, ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield {
                    target, ..
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp { target })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceDown { target })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. }),
            ..
        }) => explicit_target_choose_spec(target),
        EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
            if effects.len() == 1 =>
        {
            explicit_effect_object_target(&effects[0])
        }
        _ => None,
    }
}

pub(super) fn bind_it_metric_to_explicit_target(value: Value, target: &ChooseSpec) -> Value {
    match value {
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_it_metric_to_explicit_target(*value, target)),
            hints,
        },
        Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()) => {
            Value::PowerOf(Box::new(
                target
                    .clone()
                    .with_surface_hints(spec.surface_hints().iter().cloned()),
            ))
        }
        Value::ToughnessOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()) => {
            Value::ToughnessOf(Box::new(
                target
                    .clone()
                    .with_surface_hints(spec.surface_hints().iter().cloned()),
            ))
        }
        Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()) => {
            Value::ManaValueOf(Box::new(
                target
                    .clone()
                    .with_surface_hints(spec.surface_hints().iter().cloned()),
            ))
        }
        other => other,
    }
}

pub(super) fn bind_trailing_it_predicate_to_explicit_effect_target(
    predicate: PredicateAst,
    effect: &EffectAst,
) -> PredicateAst {
    match predicate {
        PredicateAst::ItMatches(filter) => {
            let explicit_target = explicit_effect_object_target(effect);
            let demonstrative_land = filter.demonstrative_antecedent_surface()
                == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land);
            let explicit_target_is_land = explicit_target.as_ref().is_some_and(|target| {
                matches!(
                    target.base(),
                    ChooseSpec::Object(target_filter)
                        if target_filter.card_types.contains(&crate::CardType::Land)
                            || target_filter
                                .subtypes
                                .iter()
                                .any(crate::Subtype::is_basic_land_type)
                )
            });
            // A typed demonstrative can deliberately skip over the target in
            // this replacement clause. In Emeria Shepherd, “that land” still
            // means the landfall event object, not the nonland graveyard card
            // the optional return action targets.
            if demonstrative_land && !explicit_target_is_land {
                return PredicateAst::ItMatches(filter);
            }
            if let Some(tag) = explicit_effect_object_tag(effect) {
                PredicateAst::TaggedMatches(crate::tag::TagRef::of(tag), filter)
            } else if explicit_target.is_some() {
                PredicateAst::TargetMatches(filter)
            } else {
                PredicateAst::ItMatches(filter)
            }
        }
        PredicateAst::ValueComparison {
            left,
            operator,
            right,
        } if explicit_effect_object_target(effect).is_some() => {
            let target = explicit_effect_object_target(effect)
                .expect("guarded explicit effect target should remain available");
            PredicateAst::ValueComparison {
                left: bind_it_metric_to_explicit_target(left, &target),
                operator,
                right: bind_it_metric_to_explicit_target(right, &target),
            }
        }
        other => other,
    }
}

pub fn target_is_generic_token_filter(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.token
        && filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.tagged_constraints.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
}

pub fn player_ast_from_filter_for_carry(filter: &PlayerFilter) -> Option<PlayerAst> {
    match filter {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(inner) => {
            if matches!(inner.as_ref(), PlayerFilter::Opponent) {
                Some(PlayerAst::TargetOpponent)
            } else {
                Some(PlayerAst::Target)
            }
        }
        PlayerFilter::AliasedTarget(_) => Some(PlayerAst::That),
        _ => None,
    }
}

pub fn player_owner_filter_from_target_for_carry(target: &TargetAst) -> Option<PlayerAst> {
    match target {
        TargetAst::Player(filter, _) => player_ast_from_filter_for_carry(filter),
        TargetAst::Object(filter, _, _) => {
            if !matches!(
                filter.zone,
                Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
            ) {
                return None;
            }
            filter
                .owner
                .as_ref()
                .and_then(player_ast_from_filter_for_carry)
        }
        TargetAst::WithCount(inner, _) => player_owner_filter_from_target_for_carry(inner),
        _ => None,
    }
}

pub(super) fn player_target_carry_context(target: &TargetAst) -> Option<CarryContext> {
    match target {
        TargetAst::Player(filter, _) => {
            player_ast_from_filter_for_carry(filter).map(CarryContext::Player)
        }
        TargetAst::WithCount(inner, count) => {
            let inner_context = player_target_carry_context(inner.as_ref())?;
            if count.min > 1 && count.max == Some(count.min) {
                Some(CarryContext::ForEachTargetPlayers(*count))
            } else {
                Some(inner_context)
            }
        }
        _ => None,
    }
}

fn carries_subject_player(action: &SubjectVerbActionAst) -> bool {
    matches!(
        action,
        SubjectVerbActionAst::LifeResources(
            LifeResourceActionAst::Draw { .. }
                | LifeResourceActionAst::LoseLife { .. }
                | LifeResourceActionAst::PayLife { .. }
                | LifeResourceActionAst::GainLife { .. }
                | LifeResourceActionAst::PayEnergy { .. }
                | LifeResourceActionAst::PayAnyEnergy { .. }
                | LifeResourceActionAst::PayAnyLife { .. }
                | LifeResourceActionAst::NoteLifeTotal
                | LifeResourceActionAst::RedistributeLifeTotals
        ) | SubjectVerbActionAst::RevealLook(
            RevealLookActionAst::RevealHand
                | RevealLookActionAst::RevealTop
                | RevealLookActionAst::RevealCardsFromHand { .. }
        ) | SubjectVerbActionAst::Library(
            LibraryActionAst::Mill { .. }
                | LibraryActionAst::ShuffleHandAndGraveyardIntoLibrary
                | LibraryActionAst::ShuffleGraveyardIntoLibrary { .. }
                | LibraryActionAst::ShuffleObjectsIntoLibrary { .. }
                | LibraryActionAst::ExileTopOfLibrary { .. }
                | LibraryActionAst::ReorderGraveyard
                | LibraryActionAst::ReorderGraveyardAtRandom
                | LibraryActionAst::ShuffleLibrary
        ) | SubjectVerbActionAst::KeywordActions(
            KeywordActionAst::Scry { .. }
                | KeywordActionAst::Surveil { .. }
                | KeywordActionAst::Investigate { .. }
                | KeywordActionAst::RingTemptsYou
                | KeywordActionAst::VentureIntoDungeon { .. }
                | KeywordActionAst::TakeInitiative
        ) | SubjectVerbActionAst::ZoneMoves(
            ZoneMoveActionAst::Discard { .. }
                | ZoneMoveActionAst::DiscardHand
                | ZoneMoveActionAst::ReturnToBattlefield { .. }
                | ZoneMoveActionAst::ReturnAllToBattlefield { .. }
                | ZoneMoveActionAst::ReturnToHand { .. }
                | ZoneMoveActionAst::ReturnAllToHand { .. }
                | ZoneMoveActionAst::MoveToZone { .. }
                | ZoneMoveActionAst::Sacrifice { .. }
        ) | SubjectVerbActionAst::Counters(
            CounterActionAst::PoisonCounters { .. }
                | CounterActionAst::EnergyCounters { .. }
                | CounterActionAst::ExperienceCounters { .. }
                | CounterActionAst::RadCounters { .. }
                | CounterActionAst::TicketCounters { .. }
        ) | SubjectVerbActionAst::Mana(
            ManaActionAst::PayMana { .. }
                | ManaActionAst::DoubleManaPool
                | ManaActionAst::EmptyManaPool
                | ManaActionAst::ActivateManaAbilityOfEachLand
                | ManaActionAst::AddMana { .. }
                | ManaActionAst::AddManaScaled { .. }
                | ManaActionAst::AddManaAnyColor { .. }
                | ManaActionAst::AddManaAnyOneColor { .. }
                | ManaActionAst::AddManaChosenColor { .. }
                | ManaActionAst::AddManaNotedType { .. }
                | ManaActionAst::AddManaLostThisWay { .. }
                | ManaActionAst::AddManaFromLandCouldProduce { .. }
                | ManaActionAst::AddManaAnyColorAmong { .. }
                | ManaActionAst::AddManaDraftChosenColors { .. }
                | ManaActionAst::AddManaCommanderIdentity { .. }
        ) | SubjectVerbActionAst::Characteristics(
            CharacteristicActionAst::SetLifeTotal { .. }
                | CharacteristicActionAst::ReduceSpeed { .. }
                | CharacteristicActionAst::BecomeMonarch
        ) | SubjectVerbActionAst::TurnStructure(
            TurnStructureActionAst::SkipTurn
                | TurnStructureActionAst::SkipCombatPhases
                | TurnStructureActionAst::SkipNextCombatPhaseThisTurn
                | TurnStructureActionAst::SkipMainPhasesThisTurn
                | TurnStructureActionAst::SkipCombatPhasesThisTurn
                | TurnStructureActionAst::SkipDrawStep
                | TurnStructureActionAst::SkipScheduled { .. }
                | TurnStructureActionAst::AdditionalLandPlays { .. }
                | TurnStructureActionAst::AdditionalSteps { .. }
        ) | SubjectVerbActionAst::Game(
            GameActionAst::EndTurn
                | GameActionAst::LoseGame
                | GameActionAst::WinGame
                | GameActionAst::ExtraTurnAfterTurn { .. }
        ) | SubjectVerbActionAst::Tokens(TokenActionAst::CreateEmblem { .. })
            | SubjectVerbActionAst::Random(
                RandomActionAst::FlipCoin
                    | RandomActionAst::FlipCoinFaceOnly
                    | RandomActionAst::RollDie { .. }
                    | RandomActionAst::ChooseNumberAtRandom { .. }
                    | RandomActionAst::RollDiceChooseResult { .. }
                    | RandomActionAst::RollDiceTotal { .. }
            )
            | SubjectVerbActionAst::Choices(
                ChoiceActionAst::ChooseColor
                    | ChoiceActionAst::ChooseColorAmong { .. }
                    | ChoiceActionAst::ChooseCardType { .. }
                    | ChoiceActionAst::ChooseNamedOption { .. }
                    | ChoiceActionAst::ChooseNumber { .. }
                    | ChoiceActionAst::ChooseCreatureType { .. }
                    | ChoiceActionAst::ChooseLandType { .. }
                    | ChoiceActionAst::ChooseCardName { .. }
                    | ChoiceActionAst::ChoosePlayer { .. }
            )
            | SubjectVerbActionAst::Control(ControlActionAst::Attach { .. })
    )
}

// Matching on the caller's borrow gives the same fields for inspection and binding.
// A search carries its library owner first, with the chooser as a fallback.
macro_rules! carry_player_slots {
    ($effect:expr) => {
        match $effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary { player, chooser, .. }),
                ..
            }) => (Some(player), Some(chooser)),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Tokens(
                    TokenActionAst::CreateTokenCopy { player, .. }
                    | TokenActionAst::CreateTokenWithMods { player, .. }
                ),
                ..
            })
            | EffectAst::ObjectChoices(
                ObjectChoiceEffectAst::ChooseObjects { player, .. }
                | ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { player, .. }
                | ObjectChoiceEffectAst::ChooseSourceLinkedCollection { player, .. }
            ) => (Some(player), None),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: SubjectVerbSubjectAst { player, .. },
                action,
            }) if carries_subject_player(action) => (Some(player), None),
            _ => (None, None),
        }
    };
}

fn players_for_carry(effect: &EffectAst) -> impl Iterator<Item = PlayerAst> {
    let (primary, search_chooser) = carry_player_slots!(effect);
    primary.into_iter().chain(search_chooser).copied()
}

pub fn explicit_player_for_carry(effect: &EffectAst) -> Option<CarryContext> {
    let context = match effect {
        EffectAst::Sequence { effects } => effects.iter().find_map(explicit_player_for_carry),
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { .. }) => {
            Some(CarryContext::ForEachPlayer)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers { count, .. }) => {
            Some(CarryContext::ForEachTargetPlayers(*count))
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { .. }) => {
            Some(CarryContext::ForEachOpponent)
        }
        EffectAst::SubjectVerb(subject) => match &subject.action {
            SubjectVerbActionAst::TargetOnly { target, .. } => player_target_carry_context(target),
            SubjectVerbActionAst::ZoneMoves(
                ZoneMoveActionAst::Exile { target, .. }
                | ZoneMoveActionAst::ExileUntilSourceLeaves { target, .. },
            ) => player_owner_filter_from_target_for_carry(target).map(CarryContext::Player),
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. }) => filter
                .owner
                .as_ref()
                .and_then(player_ast_from_filter_for_carry)
                .map(CarryContext::Player),
            // In "they tap all lands they control and lose all unspent mana",
            // the tapped objects' controller supplies the subject of "lose".
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { filter }) => {
                filter
                    .controller
                    .as_ref()
                    .and_then(player_ast_from_filter_for_carry)
                    .map(CarryContext::Player)
            }
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChoosePlayer { .. }) => {
                Some(CarryContext::Player(PlayerAst::That))
            }
            _ => None,
        },
        _ => None,
    };
    context.or_else(|| {
        players_for_carry(effect)
            .find(|player| *player != PlayerAst::Implicit)
            .map(CarryContext::Player)
    })
}

pub fn effect_uses_implicit_player(effect: &EffectAst) -> bool {
    players_for_carry(effect).any(|player| player == PlayerAst::Implicit)
}

fn wrap_carried_player_effect(
    effect: &mut EffectAst,
    wrap: impl FnOnce(Vec<EffectAst>) -> ForEachEffectAst,
) {
    if players_for_carry(effect)
        .any(|player| matches!(player, PlayerAst::Implicit | PlayerAst::That))
    {
        let inner = std::mem::replace(
            effect,
            EffectAst::Sequence {
                effects: Vec::new(),
            },
        );
        *effect = EffectAst::ForEach(wrap(vec![inner]));
    }
}

pub fn maybe_apply_carried_player(effect: &mut EffectAst, carried_context: CarryContext) {
    match carried_context {
        CarryContext::Player(carried_player) => {
            // When carrying an explicit target player/opponent into an implicit clause,
            // bind to the previously selected target ("that player") instead of creating
            // a fresh explicit target. This preserves shared-target semantics for chains
            // like "Target player mills..., draws..., and loses...".
            let carried_player = carried_player.in_followup();
            // An imperative search can inherit the library owner. Its omitted
            // chooser still refers to the controller of the spell or ability.
            let (primary, _) = carry_player_slots!(effect);
            if let Some(player) = primary
                && *player == PlayerAst::Implicit
            {
                *player = carried_player;
            }
        }
        CarryContext::ForEachPlayer => wrap_carried_player_effect(effect, |effects| {
            ForEachEffectAst::ForEachPlayer { effects }
        }),
        CarryContext::ForEachTargetPlayers(count) => {
            wrap_carried_player_effect(effect, |effects| ForEachEffectAst::ForEachTargetPlayers {
                count,
                filter: PlayerFilter::Any,
                effects,
            })
        }
        CarryContext::ForEachOpponent => wrap_carried_player_effect(effect, |effects| {
            ForEachEffectAst::ForEachOpponent { effects }
        }),
    }
}

pub fn maybe_apply_carried_player_with_clause_lexed(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    let facts =
        super::super::super::grammar::effects::coordination::recognize_coordination_clause_facts(
            clause_tokens,
        );
    maybe_apply_carried_player_with_clause_facts(effect, carried_context, facts);
}

pub(super) fn maybe_apply_carried_player_with_clause_facts(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    facts: super::super::super::grammar::effects::coordination::CoordinationClauseFacts,
) {
    // The library-owner grammar deliberately represents a bare `their
    // library` as `ItsController` until an outer clause supplies the
    // antecedent. In a shared-subject chain that antecedent is the carried
    // player itself (`The owner of target ... shuffles it, then exiles the
    // top card of their library`). Rebind only that grammar-proven anaphoric
    // surface; an explicit `its controller's library` does not set this fact
    // and remains controller-relative.
    if facts.anaphoric_library_owner
        && let CarryContext::Player(carried_player) = carried_context
        && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary { .. })
                | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTop),
        }) = effect
        && subject.player == PlayerAst::ItsController
    {
        subject.player = carried_player.in_followup();
    }

    let imperative_collection_move = facts.imperative_collection_move
        && matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { .. }),
                ..
            })
        );
    let imperative_return = facts.imperative_return
        && matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: SubjectVerbSubjectAst {
                    player: PlayerAst::Implicit,
                    ..
                },
                action: SubjectVerbActionAst::ZoneMoves(
                    ZoneMoveActionAst::ReturnToBattlefield { .. }
                ) | SubjectVerbActionAst::ZoneMoves(
                    ZoneMoveActionAst::ReturnAllToBattlefield { .. }
                ) | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { .. })
                    | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { .. }),
            })
        );
    // "Each player sacrifices a creature. Roll a d20 ..." (Danse Macabre): an
    // imperative roll is performed by the spell's controller, never by each
    // player of an earlier sentence.
    if !facts.explicitly_conjugated_player_action
        && matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: SubjectVerbSubjectAst {
                    player: PlayerAst::Implicit,
                    ..
                },
                action: SubjectVerbActionAst::Random(
                    crate::cards::builders::RandomActionAst::RollDie { .. }
                        | crate::cards::builders::RandomActionAst::RollDiceChooseResult { .. }
                        | crate::cards::builders::RandomActionAst::RollDiceTotal { .. }
                ),
            })
        )
        && !matches!(carried_context, CarryContext::Player(_))
    {
        return;
    }
    if facts.head == chain_grammar::CarryClauseHead::Choose
        && normalize_imperative_choose_player(effect)
    {
        return;
    }
    if facts.head == chain_grammar::CarryClauseHead::Create
        && normalize_imperative_create_player(effect)
    {
        return;
    }
    let should_skip = match carried_context {
        CarryContext::Player(_) => {
            imperative_return
                // "Target opponent mills three cards. Put a land card from
                // their graveyard onto the battlefield ..." (Realmbreaker):
                // the uninflected "put" is the controller's imperative, never
                // the earlier sentence's third-person subject.
                || (imperative_collection_move
                    && matches!(
                        effect,
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            subject: SubjectVerbSubjectAst {
                                player: PlayerAst::Implicit,
                                ..
                            },
                            ..
                        })
                    ))
                || (matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: SubjectVerbSubjectAst {
                            player: PlayerAst::Implicit,
                            ..
                        },
                        action: SubjectVerbActionAst::LifeResources(
                            LifeResourceActionAst::Draw { .. }
                        ),
                    })
                ) && facts.head == chain_grammar::CarryClauseHead::Draw)
                    && !facts.explicitly_conjugated_player_action
                || (matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: SubjectVerbSubjectAst {
                            player: PlayerAst::Implicit,
                            ..
                        },
                        action: SubjectVerbActionAst::KeywordActions(KeywordActionAst::Scry { .. })
                            | SubjectVerbActionAst::KeywordActions(
                                KeywordActionAst::Surveil { .. }
                            ),
                    })
                ) && matches!(
                    facts.head,
                    chain_grammar::CarryClauseHead::Scry | chain_grammar::CarryClauseHead::Surveil
                ) && !facts.explicitly_conjugated_player_action)
        }
        CarryContext::ForEachPlayer
        | CarryContext::ForEachTargetPlayers(_)
        | CarryContext::ForEachOpponent => {
            let is_implicit_vision_effect = matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::Implicit,
                        ..
                    },
                    action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { .. })
                        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Scry { .. })
                        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Surveil { .. }),
                })
            );
            imperative_collection_move
                || (is_implicit_vision_effect
                    && matches!(
                        facts.head,
                        chain_grammar::CarryClauseHead::Draw
                            | chain_grammar::CarryClauseHead::Scry
                            | chain_grammar::CarryClauseHead::Surveil
                    )
                    && !facts.explicitly_conjugated_player_action)
        }
    };
    if should_skip {
        return;
    }
    maybe_apply_carried_player(effect, carried_context);
}

pub(super) fn normalize_imperative_create_player(effect: &mut EffectAst) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { player, .. }),
        ..
    }) = effect
    else {
        return false;
    };

    if matches!(
        player,
        PlayerAst::Implicit | PlayerAst::Target | PlayerAst::TargetOpponent | PlayerAst::That
    ) {
        *player = PlayerAst::You;
        return true;
    }
    false
}

/// The chain grammar reads a leading bare `discard` as the controller's
/// imperative. When the chain is the body of another player's offer
/// ("<player> may discard a card"), the omitted actor is that player, so
/// return the leading discard to the offer's implicit subject.
pub fn release_offered_leading_discard_actor(
    body_tokens: &[OwnedLexToken],
    effects: &mut [EffectAst],
) {
    if !body_tokens
        .first()
        .is_some_and(|token| token.is_word("discard"))
    {
        return;
    }
    if let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject,
        action:
            SubjectVerbActionAst::ZoneMoves(
                ZoneMoveActionAst::Discard { .. } | ZoneMoveActionAst::DiscardHand,
            ),
    })) = effects.first_mut()
        && subject.player == PlayerAst::You
    {
        subject.player = PlayerAst::Implicit;
    }
}

pub fn bind_implicit_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject { .. })
                | SubjectVerbActionAst::Control(ControlActionAst::GainControl { .. })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { .. })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { .. })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap { .. })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll { .. })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { .. })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { .. }),
        }) => {
            if matches!(subject.player, PlayerAst::Implicit) {
                subject.player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                    player: effect_player,
                    ..
                })
                | SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget {
                    player: effect_player,
                    ..
                })
                | SubjectVerbActionAst::Stack(StackActionAst::CastTagged {
                    player: effect_player,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: effect_player,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    player: effect_player,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player: effect_player,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player: effect_player,
                    ..
                }),
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        // "Target opponent may sacrifice any number of creatures": the
        // implicit sacrificer of the chosen set is the same actor that chose
        // it (CR 701.21a), not the ability's controller.
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }),
        }) if !filter.tagged_constraints.is_empty() => {
            if matches!(subject.player, PlayerAst::Implicit) {
                // An announced target actor is referenced, not re-targeted:
                // the enclosing offer owns the one target declaration.
                subject.player = player.in_followup();
            }
        }
        EffectAst::SubjectVerb(_) => {
            let (primary, search_chooser) = carry_player_slots!(effect);
            for slot in primary.into_iter().chain(search_chooser) {
                if *slot == PlayerAst::Implicit {
                    *slot = player;
                }
            }
        }
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            player: effect_player,
            filter,
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
                // "each player who controls the most lands sacrifices any
                // number of lands", "target opponent may sacrifice any number
                // of creatures": an implicit chooser's own permanents were
                // read as "you control" (CR 701.21a); they belong to the
                // player the implicit actor now names.
                if filter.controller == Some(crate::target::PlayerFilter::You)
                    && matches!(
                        player,
                        PlayerAst::That | PlayerAst::Target | PlayerAst::TargetOpponent
                    )
                    && let Some(controller) =
                        crate::activation_and_restrictions::trigger_subject_filters::controller_filter_for_token_player(player)
                {
                    filter.controller = Some(controller);
                }
            }
        }
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
            player: effect_player,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseSourceLinkedCollection {
            player: effect_player,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            player: effect_player,
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        // "<player> may cast a spell from their hand without paying its mana
        // cost": the subject casts. "Their hand" names the subject's own hand
        // unless the subject is you, where it names the player the preceding
        // instruction referred to (Kuroki, Thief of Talents).
        EffectAst::Permissions(PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player: effect_player,
            zone_owner,
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
                if *zone_owner == PlayerAst::That
                    && !matches!(player, PlayerAst::You | PlayerAst::Implicit)
                {
                    *zone_owner = player;
                }
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                bind_implicit_player_context(nested_effect, player);
            }
        }),
    }
}

pub(super) fn parse_leading_player_may_words(words: &[&str]) -> Option<PlayerAst> {
    type WordInput<'a> = grammar::WordSliceInput<'a>;
    use grammar::word_slice_exact as word_eq;

    fn player_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((word_eq("player"), word_eq("players"))).void()
    }

    fn opponent_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((word_eq("opponent"), word_eq("opponents"))).void()
    }

    fn controller_subject_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((
            word_eq("creatures"),
            word_eq("lands"),
            word_eq("permanents"),
            word_eq("planeswalkers"),
            word_eq("sources"),
            word_eq("spells"),
        ))
        .void()
    }

    fn controller_or_owner_subject_word<'a>()
    -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((
            word_eq("creatures"),
            word_eq("lands"),
            word_eq("permanents"),
            word_eq("sources"),
            word_eq("spells"),
        ))
        .void()
    }

    fn leading_conjunctions<'a>(input: &mut WordInput<'a>) -> Result<(), ErrMode<ContextError>> {
        repeat::<_, _, (), _, _>(0.., alt((word_eq("then"), word_eq("and")))).parse_next(input)
    }

    fn parse_player_may_prefix<'a>(
        input: &mut WordInput<'a>,
    ) -> Result<PlayerAst, ErrMode<ContextError>> {
        (
            leading_conjunctions,
            alt((
                alt((
                    (word_eq("you"), word_eq("may")).value(PlayerAst::You),
                    (word_eq("any"), player_word(), word_eq("may")).value(PlayerAst::Any),
                    (word_eq("any"), opponent_word(), word_eq("may")).value(PlayerAst::Opponent),
                )),
                alt((
                    (word_eq("target"), opponent_word(), word_eq("may"))
                        .value(PlayerAst::TargetOpponent),
                    (word_eq("target"), player_word(), word_eq("may")).value(PlayerAst::Target),
                    (word_eq("that"), player_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("that"), opponent_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("they"), word_eq("may")).value(PlayerAst::That),
                    (
                        word_eq("that"),
                        word_eq("player"),
                        word_eq("or"),
                        word_eq("that"),
                        controller_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ThatPlayerOrTargetController),
                    (
                        word_eq("that"),
                        controller_or_owner_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (
                        word_eq("that"),
                        controller_or_owner_subject_word(),
                        word_eq("owner"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsOwner),
                )),
                alt((
                    (word_eq("the"), player_word(), word_eq("may")).value(PlayerAst::That),
                    // "Then the spell's controller may choose new targets for
                    // it" (Sudden Substitution): the definite article names
                    // the same referenced object as "that spell's".
                    (
                        word_eq("the"),
                        controller_or_owner_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (word_eq("defending"), word_eq("player"), word_eq("may"))
                        .value(PlayerAst::Defending),
                    alt((
                        (word_eq("attacking"), word_eq("player"), word_eq("may"))
                            .value(PlayerAst::Attacking),
                        (
                            word_eq("that"),
                            word_eq("attacking"),
                            word_eq("player"),
                            word_eq("may"),
                        )
                            .value(PlayerAst::Attacking),
                        (
                            word_eq("the"),
                            word_eq("attacking"),
                            word_eq("player"),
                            word_eq("may"),
                        )
                            .value(PlayerAst::Attacking),
                    )),
                    (
                        alt((word_eq("its"), word_eq("their"))),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (
                        alt((word_eq("its"), word_eq("their"))),
                        word_eq("owner"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsOwner),
                    alt((
                        (opponent_word(), word_eq("may")).value(PlayerAst::Opponent),
                        (word_eq("an"), word_eq("opponent"), word_eq("may"))
                            .value(PlayerAst::Opponent),
                    )),
                )),
            )),
        )
            .map(|(_, player)| player)
            .parse_next(input)
    }

    let mut input = words;
    crate::grammar::primitives::take_leaf(&mut input, parse_player_may_prefix)
}

pub fn parse_leading_player_may_lexed(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    parse_leading_player_may_words(&words)
}

pub fn normalize_source_references_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Result<Vec<OwnedLexToken>, CardTextError> {
    crate::util::normalize_source_reference_tokens_with_context(context, tokens)
}

pub fn parse_effect_chain_with_subject_verb_primitives(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_with_subject_verb_primitives_lexed(tokens)
}

pub fn maybe_apply_carried_player_with_clause(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    maybe_apply_carried_player_with_clause_lexed(effect, carried_context, clause_tokens);
}
