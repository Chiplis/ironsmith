use crate::cards::builders::GameActionAst;
use crate::cards::builders::ControlActionAst;
use crate::cards::builders::TokenActionAst;
use crate::cards::builders::StackActionAst;
use crate::cards::builders::DamageActionAst;
use crate::cards::builders::LifeResourceActionAst;
use crate::cards::builders::RevealLookActionAst;
use crate::cards::builders::PermanentStateActionAst;
use crate::cards::builders::KeywordActionAst;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::TurnStructureActionAst;
use crate::cards::builders::LibraryActionAst;
use crate::cards::builders::ManaActionAst;
use crate::cards::builders::GrantActionAst;
use crate::cards::builders::CounterActionAst;
use super::*;

pub(super) fn handles_action(action: &SubjectVerbActionAst) -> bool {
    matches!(
        action,
        SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalPhases { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeMonarch)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ClearSuspected { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::ControlPlayer { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Convert { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::Counter { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { .. })
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateEmblem { .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Detain { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DiscardHand)
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnEach { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::DoubleManaPool)
            | SubjectVerbActionAst::Mana(ManaActionAst::EmptyManaPool)
            | SubjectVerbActionAst::Game(GameActionAst::EndCombatPhase)
            | SubjectVerbActionAst::Game(GameActionAst::EndTurn)
            | SubjectVerbActionAst::Counters(CounterActionAst::EnergyCounters { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::ExperienceCounters { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Flip { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilityToSource { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn { .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::HealDamage { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand { .. })
            | SubjectVerbActionAst::Game(GameActionAst::LoseGame)
            | SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::MoveOneCounter { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyEnergy { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayEnergy { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::PayMana { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseIn { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PlayFromGraveyardUntilEot)
            | SubjectVerbActionAst::Counters(CounterActionAst::PoisonCounters { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfChosenKind { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCountersAll { .. })
            | SubjectVerbActionAst::PutSticker { .. }
            | SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::ReduceNextSpellCostThisTurn { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RegenerateAll { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveCountersAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { .. })
            | SubjectVerbActionAst::Game(GameActionAst::ReverseTurnOrder)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RingTemptsYou)
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetLifeTotal { .. })
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhases)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhasesThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipDrawStep)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipMainPhasesThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipNextCombatPhaseThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipTurn)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Suspect { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::TakeInitiative)
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::TicketCounters { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::UnlockRoomDoor)
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::VentureIntoDungeon { .. })
            | SubjectVerbActionAst::Game(GameActionAst::WinGame)
    )
}

fn attachment_reference_tag(spec: &ChooseSpec) -> Option<TagKey> {
    if spec.is_target() {
        return None;
    }
    match spec.base() {
        ChooseSpec::Tagged(tag) => Some(tag.clone()),
        ChooseSpec::Object(filter) => watch_tag_from_filter(filter),
        _ => None,
    }
}

fn return_graveyard_player_surface(
    target: &TargetAst,
    ctx: &EffectLoweringContext,
) -> Result<Option<PlayerFilter>, CardTextError> {
    let target = match target {
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => inner.as_ref(),
        target => target,
    };
    let TargetAst::Object(filter, _, _) = target else {
        return Ok(None);
    };
    if filter.zone != Some(Zone::Graveyard) {
        return Ok(None);
    }
    Ok(resolve_it_tag(filter, &current_reference_env(ctx))?.owner)
}

pub(super) fn compile_return_to_hand(
    subject_verb: &SubjectVerbEffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<EffectCompileOutcome, CardTextError> {
    let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
        target,
        random,
        destination_player_surface,
        exiled_with_source_surface,
        set_quantifier_surface,
        set_reference_surface,
    }) = &subject_verb.action
    else {
        unreachable!("typed return-to-hand route requires a ReturnToHand action")
    };
    let role = subject_verb_role(subject_verb.subject.role);
    let player = subject_verb.subject.player;
    let graveyard_player_surface = return_graveyard_player_surface(target, ctx)?;
    let (mut spec, mut choices) =
        resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
    let actor_surface = if role == SubjectRole::Actor && !matches!(player, PlayerAst::Implicit) {
        let actor = resolve_subject_verb_subject(role, player, ctx, true, true, false)?;
        for choice in actor.into_choices() {
            push_choice(&mut choices, choice);
        }
        Some(actor.clone_player_filter())
    } else {
        None
    };
    // A plural demonstrative in a later per-player sentence refers to the
    // collection chosen by the preceding quantified sentence. `Iterated`
    // cannot resolve an object while only a player loop is active, so retain
    // the producer tag explicitly instead.
    if ctx.iterated_player
        && !ctx.iterated_object
        && set_reference_surface.is_some()
        && matches!(spec.base(), ChooseSpec::Iterated)
        && let Some(tag) = ctx.last_object_tag.as_ref()
    {
        spec = ChooseSpec::Tagged(tag.clone());
    }
    let destination_player_surface = destination_player_surface
        .map(|player| resolve_non_target_player_filter(player, &current_reference_env(ctx)))
        .transpose()?;
    let from_graveyard = target_mentions_graveyard(target);
    if from_graveyard
        && !ctx.iterated_player
        && ctx.last_player_filter.as_ref() != Some(&PlayerFilter::IteratedPlayer)
        && choose_spec_mentions_iterated_player(&spec)
    {
        replace_iterated_player_with_target_player_in_choose_spec(&mut spec);
    }
    let move_effect = if from_graveyard {
        let mut effect =
            crate::effects::ReturnFromGraveyardToHandEffect::new(spec.clone(), *random);
        if let Some(player) = actor_surface.clone() {
            effect = effect.with_actor_surface(player);
        }
        if let Some(player) = graveyard_player_surface {
            effect = effect.with_graveyard_player_surface(player);
        }
        if let Some(player) = destination_player_surface.clone() {
            effect = effect.with_destination_player_surface(player);
        }
        Effect::new(effect)
    } else {
        let mut effect = crate::effects::ReturnToHandEffect::with_spec(spec.clone());
        if let Some(player) = actor_surface {
            effect = effect.with_actor_surface(player);
        }
        if let Some(player) = destination_player_surface.clone() {
            effect = effect.with_destination_player_surface(player);
        }
        if let Some(surface) = exiled_with_source_surface {
            effect = effect.with_exiled_with_source_surface(surface.clone());
        }
        effect = effect.with_set_quantifier_surface(*set_quantifier_surface);
        effect = effect.with_set_reference_surface(set_reference_surface.clone());
        Effect::new(effect)
    };
    let effect = tag_object_target_effect(move_effect, &spec, ctx, "returned");
    ctx.last_player_filter = Some(if spec.is_target() {
        PlayerFilter::AliasedOwnerOf(ObjectRef::Target)
    } else if let Some(tag) = ctx.last_object_tag.clone() {
        PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(tag))
    } else {
        PlayerFilter::AliasedOwnerOf(ObjectRef::Target)
    });
    Ok((vec![effect], choices))
}

/// Preserve a mandatory complete-set discard after subject/player lowering.
///
/// `discard all <matching cards>` is parsed as both an eligible-card filter
/// and a `Value::Count` over that same filter. Player resolution can turn an
/// authored target-player reference into a follow-up alias on the eligible
/// filter. Apply that same canonical filter to the count so the runtime still
/// discards exactly the complete eligible set.
fn replace_complete_discard_count_filter(value: &mut Value, filter: &ObjectFilter) {
    match value {
        Value::SurfaceHinted { value, .. } => {
            replace_complete_discard_count_filter(value, filter);
        }
        Value::Count(count_filter) => *count_filter = filter.clone(),
        _ => {}
    }
}

/// `other` is normally evaluated relative to the resolving ability source.
/// In "A deals ... to another target ...", however, it is relative to the
/// grammatical damage source `A`, which may be a tagged trigger participant
/// rather than the Aura/Equipment carrying the ability. Preserve the authored
/// `other` surface and add the exact tagged-identity exclusion used by target
/// legality.
fn bind_other_damage_target_to_tagged_source(target: &mut ChooseSpec, source: &ChooseSpec) {
    let ChooseSpec::Tagged(source_tag) = source.base() else {
        return;
    };

    fn bind(target: &mut ChooseSpec, source_tag: &TagKey) {
        match target {
            ChooseSpec::SurfaceHinted { spec, .. }
            | ChooseSpec::Target(spec)
            | ChooseSpec::WithCount(spec, _)
            | ChooseSpec::WithCountValue(spec, _, _) => bind(spec, source_tag),
            ChooseSpec::Object(filter) | ChooseSpec::ObjectOrPlayer(filter, _) if filter.other => {
                if !filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == source_tag.as_str()
                        && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
                }) {
                    filter
                        .tagged_constraints
                        .push(crate::filter::TaggedObjectConstraint {
                            tag: source_tag.clone(),
                            relation: TaggedOpbjectRelation::IsNotTaggedObject,
                        });
                }
            }
            _ => {}
        }
    }

    bind(target, source_tag);
}

pub(super) fn compile_put_counters_action(
    subject_verb: &SubjectVerbEffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<EffectCompileOutcome, CardTextError> {
    let SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
        counter_type,
        count,
        target,
        target_count,
        distributed,
    }) = &subject_verb.action
    else {
        unreachable!("typed put-counters route requires a PutCounters action")
    };
    let (base_spec, _) = resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
    let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
    let mut spec = base_spec;
    if let Some(target_count) = target_count {
        spec = with_target_count_preserving_value(spec, *target_count);
    }
    let mut put_counters =
        crate::effects::PutCountersEffect::new(*counter_type, resolved_count, spec.clone());
    if let Some(target_count) = target_count {
        put_counters = put_counters.with_target_count(*target_count);
    }
    if *distributed {
        put_counters = put_counters.with_distributed(true);
    }
    let effect = tag_object_target_effect(Effect::new(put_counters), &spec, ctx, "counters");
    let choices = if spec.is_target() {
        vec![spec.clone()]
    } else {
        Vec::new()
    };
    Ok((vec![effect], choices))
}

pub(super) fn compile_subject_verb_late(
    subject_verb: &SubjectVerbEffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError> {
    let role = subject_verb_role(subject_verb.subject.role);
    let player = subject_verb.subject.player;
    let result = match &subject_verb.action {
        SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilityToSource { ability, duration }) => {
            let lowered = lower_parsed_ability(ability.as_ref().clone())?;
            Ok((
                vec![Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec(
                        crate::target::ChooseSpec::Source,
                        crate::continuous::Modification::AddAbilityGeneric(lowered),
                        duration.clone(),
                    ),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp { target }) => {
            let (effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "turned_face_up", |spec| {
                    Effect::turn_face_up(spec)
                })?;
            Ok((effects, choices))
        }
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
            amount,
            target,
            unpreventable,
        }) => {
            let mut target_bound_amount = amount.clone();
            if let TargetAst::Player(filter, Some(_))
            | TargetAst::PlayerOrPlaneswalker(filter, Some(_)) = target
            {
                // The explicit player target is a typed same-clause
                // antecedent for an authored "that player" inside the damage
                // amount. This is local to this action, so it takes precedence
                // even when an enclosing trigger or loop also carries an
                // iterated-player binding.
                bind_relative_iterated_player_in_value_to_player_filter(
                    &mut target_bound_amount,
                    &PlayerFilter::Target(Box::new(filter.clone())),
                );
            }
            // Bind the same-clause player first. Contextual reference
            // resolution may otherwise consume the IteratedPlayer placeholder
            // as an older trigger/loop antecedent, losing the nearer explicit
            // target provenance before the binder can see it.
            let resolved_amount =
                resolve_value_it_tag(&target_bound_amount, &current_reference_env(ctx))?;
            let (mut effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                    if *unpreventable {
                        Effect::deal_unpreventable_damage(resolved_amount.clone(), spec)
                    } else {
                        Effect::deal_damage(resolved_amount.clone(), spec)
                    }
                })?;
            if let TargetAst::Player(filter, explicit_target_span) = target {
                ctx.last_player_filter = Some(if explicit_target_span.is_some() {
                    PlayerFilter::Target(Box::new(filter.clone()))
                } else {
                    as_followup_player_alias(filter.clone())
                });
            } else if let TargetAst::PlayerOrPlaneswalker(filter, _) = target {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            } else if target_is_any_damage_target(target) {
                let tag = ctx.next_tag("damaged");
                ctx.last_object_tag = Some(tag.clone());
                if let Some(effect) = effects.pop() {
                    effects.push(effect.tag(tag));
                }
                ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
            }
            Ok((effects, choices))
        }
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, filter }) => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let tag = ctx.next_tag("damaged");
            ctx.last_object_tag = Some(tag.clone());
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::deal_damage(resolved_amount, ChooseSpec::Iterated).tag(tag)],
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
            amount,
            target,
            source,
            chooser,
            distribution,
        }) => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let (source_spec, source_choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            let (mut effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                    Effect::new(
                        crate::effects::DealDistributedDamageEffect::new(
                            resolved_amount.clone(),
                            spec,
                        )
                        .with_source(source_spec.clone())
                        .with_chooser(chooser.clone())
                        .with_distribution(*distribution),
                    )
                })?;
            let mut choices = choices;
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }
            if target_is_any_damage_target(target) {
                let tag = ctx.next_tag("damaged");
                ctx.last_object_tag = Some(tag.clone());
                if let Some(effect) = effects.pop() {
                    effects.push(effect.tag(tag));
                }
            }
            Ok((effects, choices))
        }
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
            source,
            amount,
            target,
            unpreventable,
        }) => {
            let (source_spec, mut choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            // A bare "it" damage subject inside a becomes-blocked trigger
            // refers to the trigger's source; last-object memory is seeded
            // with the BLOCKER there (for "that creature" references), so
            // the pronoun must not inherit it as the damage source.
            let source_spec = if matches!(
                source,
                TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            ) && matches!(
                &source_spec,
                ChooseSpec::Tagged(tag) if tag.as_str() == "blocking"
            ) {
                ChooseSpec::Source
            } else {
                source_spec
            };
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let mut damage_target_spec = if source == target {
                source_spec.clone()
            } else {
                let (mut target_spec, mut target_choices) =
                    resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                bind_other_damage_target_to_tagged_source(&mut target_spec, &source_spec);
                for choice in &mut target_choices {
                    bind_other_damage_target_to_tagged_source(choice, &source_spec);
                }
                for choice in target_choices {
                    push_choice(&mut choices, choice);
                }
                target_spec
            };

            let mut effects = Vec::new();
            let mut damage_source_spec = source_spec.clone();
            // `this creature` still names the resolving ability's source
            // when another object deals damage. Capture it before the
            // execution wrapper temporarily changes the damage source.
            if matches!(damage_target_spec.base(), ChooseSpec::Source)
                && !matches!(damage_source_spec.base(), ChooseSpec::Source)
            {
                let original_source = ctx.next_tag("damage_recipient_source");
                effects.push(Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                    ObjectFilter::source(), original_source.clone(),
                )));
                let mut hints = damage_target_spec.surface_hints().to_vec();
                if hints.is_empty() {
                    hints.push(crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::ThisPermanentType("this source".into()),
                    ));
                }
                damage_target_spec = ChooseSpec::Tagged(original_source.into()).with_surface_hints(hints);
            }
            let per_target_source_spec = if source == target {
                ChooseSpec::Iterated
            } else {
                source_spec.clone()
            };
            // An explicit target becomes the local source of
            // `ExecuteWithSourceEffect`, so its characteristic values remain
            // source-relative. An anaphoric `it`, however, has already been
            // resolved through the reference environment; bind the same
            // concrete identity into `its power`/`its toughness` values.
            let damage_amount = if matches!(
                source,
                TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            ) && !matches!(
                source_spec.base(),
                ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            ) {
                bind_source_value_to_damage_source(&amount, &source_spec)
            } else {
                amount.clone()
            };

            if source_spec.is_target() {
                let source_tag = reserved_or_next_object_tag(ctx, "damage_source");
                effects.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(source_spec.clone()))
                        .tag(source_tag.clone()),
                );
                ctx.last_object_tag = Some(source_tag.clone());
                damage_source_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                if source == target {
                    damage_target_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                }
            }

            let mass_damage_filter = if damage_target_spec.is_target() {
                None
            } else {
                match damage_target_spec.base() {
                    ChooseSpec::All(filter) => Some(filter),
                    ChooseSpec::Object(filter) if filter.has_plural_object_noun_surface() => {
                        Some(filter)
                    }
                    _ => None,
                }
            };
            if let Some(filter) = mass_damage_filter {
                // In "it deals damage to each creature blocking it", the
                // filter's source-relative relation names the grammatical
                // damage source, not necessarily the source of the resolving
                // ability (an Equipment is the latter, its equipped creature
                // is the former).
                let recipient_filter_uses_damage_source = filter.in_combat_with_source;
                let damage = if *unpreventable {
                    Effect::deal_unpreventable_damage(amount.clone(), ChooseSpec::Iterated)
                } else {
                    Effect::deal_damage(amount.clone(), ChooseSpec::Iterated)
                };
                let mut per_target_damage = if recipient_filter_uses_damage_source {
                    damage
                } else {
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        per_target_source_spec.clone(),
                        damage,
                    ))
                };
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("damaged");
                    ctx.last_object_tag = Some(tag.clone());
                    per_target_damage = per_target_damage.tag(tag);
                }
                let fanout = Effect::for_each(filter.clone(), vec![per_target_damage]);
                effects.push(if recipient_filter_uses_damage_source {
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        per_target_source_spec.clone(),
                        fanout,
                    ))
                } else {
                    fanout
                });
            } else {
                let damage = if *unpreventable {
                    Effect::deal_unpreventable_damage(
                        damage_amount.clone(),
                        damage_target_spec.clone(),
                    )
                } else {
                    Effect::deal_damage(damage_amount.clone(), damage_target_spec.clone())
                };
                let damage_effect = tag_object_target_effect(
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        damage_source_spec.clone(),
                        damage,
                    )),
                    &damage_target_spec,
                    ctx,
                    "damaged",
                );
                effects.push(damage_effect);
            }

            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            } else if matches!(
                target,
                TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_)
            ) {
                ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
            }

            Ok((effects, choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::tap(spec.clone())
            } else {
                Effect::new(crate::effects::TapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "tapped");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::untap(spec.clone())
            } else {
                Effect::new(crate::effects::UntapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "untapped");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { filter }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("tapped");
                prelude.push(Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                    resolved_filter.clone(),
                    tag.clone(),
                )));
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(Effect::tap_all(resolved_filter));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter }) => {
            let refs = current_reference_env(ctx);
            let unresolved_demonstrative_set = refs.known_last_object_tag().is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
            let resolved_filter = resolve_it_tag(filter, &refs)?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("untapped");
                prelude.push(Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                    resolved_filter.clone(),
                    tag.clone(),
                )));
                ctx.last_object_tag = Some(tag);
            }
            // If "those permanents" arrived without a usable antecedent, do
            // not broaden it into every matching permanent.  A single
            // non-target choice is the conservative executable fallback and
            // preserves the old surface until the missing choice loop is
            // represented explicitly.
            if unresolved_demonstrative_set {
                prelude.push(Effect::untap(ChooseSpec::Object(resolved_filter)));
            } else {
                prelude.push(Effect::untap_all(resolved_filter));
            }
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let modes = vec![
                EffectMode {
                    source_text: "Tap".to_string(),
                    effects: vec![Effect::tap(spec.clone())],
                },
                EffectMode {
                    source_text: "Untap".to_string(),
                    effects: vec![Effect::untap(spec.clone())],
                },
            ];
            let effect =
                tag_object_target_effect(Effect::choose_one(modes), &spec, ctx, "tap_or_untap");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll {
            tap_filter,
            untap_filter,
        }) => {
            let resolved_tap = resolve_it_tag(tap_filter, &current_reference_env(ctx))?;
            let resolved_untap = resolve_it_tag(untap_filter, &current_reference_env(ctx))?;
            let (mut prelude, mut choices) = target_context_prelude_for_filter(&resolved_tap);
            let (_, untap_choices) = target_context_prelude_for_filter(&resolved_untap);
            for choice in untap_choices {
                push_choice(&mut choices, choice);
            }
            let modes = vec![
                EffectMode {
                    source_text: "Tap".to_string(),
                    effects: vec![Effect::tap_all(resolved_tap)],
                },
                EffectMode {
                    source_text: "Untap".to_string(),
                    effects: vec![Effect::untap_all(resolved_untap)],
                },
            ];
            prelude.push(Effect::choose_one(modes));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut {
            target,
            duration,
            source_surface,
        }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = match spec {
                ChooseSpec::Object(filter) if filter.set_quantifier_surface().is_some() => {
                    ChooseSpec::All(filter)
                }
                other => other,
            };
            let mut phase_out = crate::effects::PhaseOutEffect::with_spec(spec.clone());
            phase_out.duration = *duration;
            phase_out.source_surface = source_surface.clone();
            let base_effect = Effect::new(phase_out);
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "phased_out");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll {
            filter,
            duration,
            source_surface,
        }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut phase_out =
                crate::effects::PhaseOutEffect::with_spec(ChooseSpec::all(resolved_filter));
            phase_out.duration = *duration;
            phase_out.source_surface = source_surface.clone();
            prelude.push(Effect::new(phase_out));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseIn { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::phase_in(spec.clone())
            } else {
                Effect::new(crate::effects::PhaseInEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "phased_in");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { filter }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::new(crate::effects::PhaseInEffect::with_spec(
                ChooseSpec::all(resolved_filter),
            )));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform { target }) => {
            compile_tagged_effect_for_target(target, ctx, "transformed", Effect::transform)
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Convert { target }) => {
            compile_tagged_effect_for_target(target, ctx, "converted", Effect::convert)
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
            target,
            no_regeneration,
            creature_destroyed_this_way_surface,
        }) => compile_tagged_effect_for_target(target, ctx, "destroyed", |spec| {
            if *no_regeneration {
                Effect::new(
                    crate::effects::DestroyNoRegenerationEffect::with_spec(spec)
                        .with_creature_destroyed_this_way_surface(
                            *creature_destroyed_this_way_surface,
                        ),
                )
            } else {
                Effect::new(crate::effects::DestroyEffect::with_spec(spec))
            }
        }),
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll {
            filter,
            no_regeneration,
            creature_destroyed_this_way_surface,
        }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut effect = if *no_regeneration {
                Effect::new(
                    crate::effects::DestroyNoRegenerationEffect::all(resolved_filter)
                        .with_creature_destroyed_this_way_surface(
                            *creature_destroyed_this_way_surface,
                        ),
                )
            } else {
                Effect::destroy_all(resolved_filter)
            };
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor {
            filter,
            no_regeneration,
            creature_destroyed_this_way_surface,
        }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ];
            let auto_tag = if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                ctx.last_object_tag = Some(tag.clone());
                Some(tag)
            } else {
                None
            };
            for color in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = if *no_regeneration {
                    format!(
                        "Destroy all {}. They can't be regenerated.",
                        filter.description()
                    )
                } else {
                    format!("Destroy all {}.", filter.description())
                };
                let mut effect = if *no_regeneration {
                    Effect::new(
                        crate::effects::DestroyNoRegenerationEffect::all(filter)
                            .with_creature_destroyed_this_way_surface(
                                *creature_destroyed_this_way_surface,
                            ),
                    )
                } else {
                    Effect::destroy_all(filter)
                };
                if let Some(tag) = &auto_tag {
                    effect = effect.tag(tag.clone());
                }
                modes.push(EffectMode {
                    source_text: description,
                    effects: vec![effect],
                });
            }
            prelude.push(Effect::choose_one(modes));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { filter, target }) => {
            let (target_spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut prelude = Vec::new();
            let mut choices = choices;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(player_filter) = match target_spec.base() {
                ChooseSpec::Player(player_filter) => Some(player_filter.clone()),
                ChooseSpec::SourceController => Some(PlayerFilter::You),
                _ => None,
            } {
                resolved_filter.attached_to_player = Some(player_filter);
                ctx.last_object_tag = None;
            } else {
                let target_tag = if let Some(tag) = attachment_reference_tag(&target_spec) {
                    tag.clone()
                } else {
                    if !choose_spec_targets_object(&target_spec) || !target_spec.is_target() {
                        return Err(CardTextError::ParseError(
                            "destroy-attached target must be an object, player, or tagged object"
                                .to_string(),
                        ));
                    }
                    let tag = ctx.next_tag("attachment_target");
                    prelude.push(
                        Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()))
                            .tag(tag.clone()),
                    );
                    tag
                };
                ctx.last_object_tag = Some(target_tag.clone());

                resolved_filter
                    .tagged_constraints
                    .push(TaggedObjectConstraint {
                        tag: target_tag,
                        relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                    });
            }

            let (mut filter_prelude, filter_choices) =
                target_context_prelude_for_filter(&resolved_filter);
            for choice in filter_choices {
                push_choice(&mut choices, choice);
            }

            let mut effect = Effect::destroy_all(resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.append(&mut filter_prelude);
            prelude.push(effect);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo {
            filter,
            target,
            face_down,
        }) => {
            let (target_spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut prelude = Vec::new();
            let mut choices = choices;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let target_tag = if let Some(tag) = attachment_reference_tag(&target_spec) {
                tag.clone()
            } else {
                if !choose_spec_targets_object(&target_spec) || !target_spec.is_target() {
                    return Err(CardTextError::ParseError(
                        "exile-attached target must be a target object or tagged object"
                            .to_string(),
                    ));
                }
                let tag = ctx.next_tag("attachment_target");
                prelude.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()))
                        .tag(tag.clone()),
                );
                tag
            };
            ctx.last_object_tag = Some(target_tag.clone());

            resolved_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: target_tag.clone(),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                });

            let (mut filter_prelude, filter_choices) =
                target_context_prelude_for_filter(&resolved_filter);
            for choice in filter_choices {
                push_choice(&mut choices, choice);
            }
            prelude.append(&mut filter_prelude);
            prelude.push(Effect::new(
                crate::effects::ExileEffect::all(resolved_filter).with_face_down(*face_down),
            ));

            let tagged_target = ChooseSpec::Tagged(target_tag);
            let target_exile = if *face_down {
                Effect::new(
                    crate::effects::ExileEffect::with_spec(tagged_target).with_face_down(true),
                )
            } else {
                Effect::move_to_zone(tagged_target, Zone::Exile, true)
            };
            prelude.push(target_exile);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
            target,
            face_down,
            source_top_only,
            target_plural_surface,
        }) => {
            if *source_top_only {
                let (spec, choices) =
                    resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                let collection_is_plural = !spec.count().is_single();
                let (choose, chosen_spec) =
                    lower_source_top_only_choice(&spec, player, ctx, "chosen_top")?;
                let chosen_tag = match chosen_spec.base() {
                    ChooseSpec::Tagged(tag) => tag.clone(),
                    _ => unreachable!("ordered source choice always lowers to a tagged object"),
                };
                ctx.last_exiled_collection_tag = Some(chosen_tag.clone());
                ctx.last_exiled_collection_is_plural = collection_is_plural;
                let exile = Effect::new(
                    crate::effects::ExileEffect::with_spec(chosen_spec).with_face_down(*face_down),
                );
                return Ok(Some((vec![choose, exile], choices)));
            }
            if let Some(compiled) = lower_hand_exile_target(target, *face_down, ctx)? {
                return Ok(Some(compiled));
            }
            if let Some(compiled) = lower_counted_non_target_exile_target(target, *face_down, ctx)?
            {
                return Ok(Some(compiled));
            }
            if let Some(compiled) = lower_single_non_target_exile_target(target, *face_down, ctx)? {
                return Ok(Some(compiled));
            }
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = if spec.count().is_single() && !*face_down {
                let mut move_effect =
                    crate::effects::MoveToZoneEffect::new(spec.clone(), Zone::Exile, true);
                if !matches!(
                    player,
                    PlayerAst::Implicit | PlayerAst::Target | PlayerAst::TargetOpponent
                ) {
                    move_effect = move_effect.with_actor_surface(resolve_non_target_player_filter(
                        player,
                        &current_reference_env(ctx),
                    )?);
                }
                if *target_plural_surface {
                    move_effect = move_effect.with_target_plural_surface();
                }
                Effect::new(move_effect)
            } else {
                Effect::new(
                    crate::effects::ExileEffect::with_spec(spec.clone()).with_face_down(*face_down),
                )
            };
            if ctx.auto_tag_object_targets {
                if let ChooseSpec::Tagged(tag) = spec.base()
                    && is_sentence_helper_exiled_collection_tag(tag)
                {
                    effect = effect.tag(tag.clone());
                    ctx.last_object_tag = Some(tag.clone());
                } else if spec.is_target() {
                    let tag = ctx.next_tag("exiled");
                    effect = effect.tag(tag.clone());
                    ctx.last_object_tag = Some(tag);
                } else if choose_spec_targets_object(&spec)
                    || matches!(spec.base(), ChooseSpec::Source)
                {
                    // MoveToZone/Exile populate the source-exiled link without
                    // needing a second runtime tag wrapper.
                    ctx.last_object_tag =
                        Some((crate::tag::CompilerReferenceTag::SourceExiled.bind()).into());
                }
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, face_down }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if let Some(player_filter) = player_filter_from_object_filter(&resolved_filter) {
                ctx.last_player_filter = Some(player_filter);
            }
            let keep_last_object_tag =
                resolved_filter.tagged_constraints.iter().any(|constraint| {
                    matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                    )
                });
            let mut effect = Effect::new(
                crate::effects::ExileEffect::all(resolved_filter).with_face_down(*face_down),
            );
            if ctx.auto_tag_object_targets {
                if keep_last_object_tag {
                    if let Some(tag) = ctx.last_object_tag.clone() {
                        effect = effect.tag(tag);
                    }
                } else {
                    let tag = ctx.next_tag("exiled");
                    effect = effect.tag(tag.clone());
                    ctx.last_exiled_collection_tag = Some(tag.clone());
                    ctx.last_exiled_collection_is_plural = true;
                    ctx.last_object_tag = Some(tag);
                }
            }
            prelude.push(effect);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand { target }) => {
            let refs = current_reference_env(ctx);
            let (spec, choices) = resolve_target_spec_with_choices(target, &refs)?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::LookAtHandEffect::new(spec.clone())),
                &spec,
                ctx,
                "targeted",
            );
            match spec.unhinted() {
                ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
                    ctx.last_player_filter = Some(filter.clone());
                }
                ChooseSpec::Target(inner) => match inner.unhinted() {
                    ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
                        ctx.last_player_filter =
                            Some(PlayerFilter::Target(Box::new(filter.clone())));
                    }
                    _ => {}
                },
                _ => {}
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Stack(StackActionAst::Counter { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect =
                tag_object_target_effect(Effect::counter(spec.clone()), &spec, ctx, "countered");
            if let Some(tag) = ctx.last_object_tag.clone() {
                ctx.last_player_filter = Some(PlayerFilter::ControllerOf(ObjectRef::tagged(tag)));
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { target, cost }) => {
            let cost =
                crate::lowering::cost_materialization::materialize_compiler_core_total_cost(cost)?;
            let cost = resolve_total_cost_it_tags(&cost, &current_reference_env(ctx))?;
            let compiled = compile_tagged_effect_for_target(target, ctx, "countered", |spec| {
                Effect::counter_unless_pays_total_cost(spec, cost.clone())
            })?;
            if let Some(tag) = ctx.last_object_tag.clone() {
                ctx.last_player_filter = Some(PlayerFilter::ControllerOf(ObjectRef::tagged(tag)));
            }
            Ok(compiled)
        }
        SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { .. }) => compile_put_counters_action(subject_verb, ctx),
        SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice {
            counter_types,
            count,
            mode_texts,
            target,
            target_count,
        }) => {
            use crate::effect::EffectMode;

            let (base_spec, _) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = with_target_count_preserving_value(spec, *target_count);
            }

            let modes = counter_types
                .iter()
                .enumerate()
                .map(|(idx, counter_type)| EffectMode {
                    source_text: mode_texts
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| format!("Put a {} counter", counter_type.description())),
                    effects: vec![Effect::put_counters(
                        *counter_type,
                        resolved_count.clone(),
                        spec.clone(),
                    )],
                })
                .collect();

            let effect = tag_object_target_effect(
                Effect::new(
                    crate::effects::ChooseModeEffect::choose_one(modes)
                        .with_chooser(PlayerFilter::You),
                ),
                &spec,
                ctx,
                "counters",
            );
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::PutCountersAll {
            counter_type,
            count,
            filter,
        }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let mut effect = Effect::for_each(
                resolved_filter,
                vec![Effect::put_counters(
                    *counter_type,
                    resolved_count,
                    ChooseSpec::Iterated,
                )],
            );
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("counters");
                effect = effect.tag_all(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters {
            amount,
            target,
            counter_type,
            up_to,
            distributed_across_all,
            all_of_them,
        }) => {
            if *all_of_them {
                return Err(CardTextError::ParseError(
                    "unable to resolve 'all of them' counter reference".to_string(),
                ));
            }
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let (mut spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            if *distributed_across_all {
                spec = match spec.unhinted() {
                    ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
                        ChooseSpec::All(filter.clone())
                    }
                    _ => {
                        return Err(CardTextError::ParseError(
                            "counter distribution requires an object filter".to_string(),
                        ));
                    }
                };
            }
            let resolved_amount = match (&resolved_amount, counter_type) {
                (Value::CountersOn(counter_source, amount_counter_type), Some(counter_type))
                    if matches!(counter_source.as_ref(), ChooseSpec::Source)
                        && amount_counter_type == &Some(*counter_type) =>
                {
                    Value::CountersOn(Box::new(spec.clone()), Some(*counter_type))
                }
                (Value::CountersOn(counter_source, None), None)
                    if matches!(counter_source.as_ref(), ChooseSpec::Source) =>
                {
                    Value::CountersOn(Box::new(spec.clone()), None)
                }
                _ => resolved_amount,
            };
            let effect = if let Some(counter_type) = counter_type {
                if *up_to {
                    Effect::remove_up_to_counters(*counter_type, resolved_amount, spec.clone())
                } else {
                    Effect::remove_counters(*counter_type, resolved_amount, spec.clone())
                }
            } else if *up_to {
                Effect::remove_up_to_any_counters(resolved_amount, spec.clone())
            } else {
                Effect::remove_any_counters(resolved_amount, spec.clone())
            };
            let effect =
                tag_object_target_effect(Effect::with_id(id.0, effect), &spec, ctx, "counters");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters { from, to }) => {
            let (from_spec, mut choices) =
                resolve_target_spec_with_choices(from, &current_reference_env(ctx))?;
            let (to_spec, to_choices) =
                resolve_target_spec_with_choices(to, &current_reference_env(ctx))?;
            for choice in to_choices {
                push_choice(&mut choices, choice);
            }
            let effect = tag_object_target_effect(
                tag_object_target_effect(
                    Effect::move_all_counters(from_spec.clone(), to_spec.clone()),
                    &from_spec,
                    ctx,
                    "from",
                ),
                &to_spec,
                ctx,
                "to",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::MoveOneCounter { from, to }) => {
            let (from_spec, mut choices) =
                resolve_target_spec_with_choices(from, &current_reference_env(ctx))?;
            let (to_spec, to_choices) =
                resolve_target_spec_with_choices(to, &current_reference_env(ctx))?;
            for choice in to_choices {
                push_choice(&mut choices, choice);
            }
            let effect = tag_object_target_effect(
                tag_object_target_effect(
                    Effect::move_one_counter(from_spec.clone(), to_spec.clone()),
                    &from_spec,
                    ctx,
                    "from",
                ),
                &to_spec,
                ctx,
                "to",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove {
            target,
            counter_source,
            all_kinds,
            fixed_counter_type,
            optional_action,
            put_only,
            choose_target_per_kind,
        }) => {
            let (mut spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let counter_source_spec = if let Some(counter_source) = counter_source {
                let (source_spec, source_choices) =
                    if let TargetAst::Object(filter, explicit_target_span, _) = counter_source
                        && explicit_target_span.is_none()
                    {
                        (
                            ChooseSpec::All(resolve_it_tag(filter, &current_reference_env(ctx))?),
                            Vec::new(),
                        )
                    } else {
                        resolve_target_spec_with_choices(
                            counter_source,
                            &current_reference_env(ctx),
                        )?
                    };
                for choice in source_choices {
                    push_choice(&mut choices, choice);
                }
                Some(source_spec)
            } else {
                None
            };
            if fixed_counter_type.is_some()
                && let TargetAst::Object(filter, explicit_target_span, _) = target
                && explicit_target_span.is_none()
            {
                spec = ChooseSpec::All(resolve_it_tag(filter, &current_reference_env(ctx))?);
            }
            let effect = if *put_only
                && *choose_target_per_kind
                && let Some(counter_source_spec) = counter_source_spec
            {
                crate::effects::ForEachCounterKindPutOrRemoveEffect::put_each_kind_from(
                    counter_source_spec,
                    spec,
                )
            } else if let Some(counter_type) = fixed_counter_type {
                crate::effects::ForEachCounterKindPutOrRemoveEffect::fixed_counter_type(
                    spec,
                    *counter_type,
                    *optional_action,
                )
            } else if *all_kinds {
                crate::effects::ForEachCounterKindPutOrRemoveEffect::new(spec)
            } else {
                crate::effects::ForEachCounterKindPutOrRemoveEffect::one_kind(spec)
            };
            Ok((vec![Effect::new(effect)], choices))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfChosenKind { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            Ok((
                vec![Effect::new(
                    crate::effects::PutCounterOfChosenKindEffect::new(spec),
                )],
                choices,
            ))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { .. }) => compile_return_to_hand(subject_verb, ctx),
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand {
            filter,
            destination_player_surface,
            exiled_with_source_surface,
        }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let destination_player_surface = destination_player_surface
                .map(|player| resolve_non_target_player_filter(player, &current_reference_env(ctx)))
                .transpose()?;
            let mut effect = crate::effects::ReturnToHandEffect::all(resolved_filter);
            if let Some(player) = destination_player_surface {
                effect = effect.with_destination_player_surface(player);
            }
            if let Some(surface) = exiled_with_source_surface {
                effect = effect.with_exiled_with_source_surface(surface.clone());
            }
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { filter }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ];
            for color in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = format!(
                    "Return all {} to their owners' hands.",
                    filter.description()
                );
                modes.push(EffectMode {
                    source_text: description,
                    effects: vec![Effect::return_all_to_hand(filter)],
                });
            }
            prelude.push(Effect::choose_one(modes));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop { target, position }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = Effect::new(crate::effects::MoveToLibraryNthFromTopEffect::new(
                spec.clone(),
                position.clone(),
            ));
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnEach {
            counter_type,
            filter,
        }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let effect = Effect::double_counters(*counter_type, ChooseSpec::All(resolved_filter));
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget {
            counter_type,
            target,
        }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = Effect::double_counters(*counter_type, spec);
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Counters(CounterActionAst::RemoveCountersAll {
            amount,
            filter,
            counter_type,
            up_to,
        }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let iterated = ChooseSpec::Iterated;
            let inner = if let Some(counter_type) = counter_type {
                if *up_to {
                    Effect::remove_up_to_counters(*counter_type, resolved_amount, iterated.clone())
                } else {
                    Effect::remove_counters(*counter_type, resolved_amount, iterated.clone())
                }
            } else {
                Effect::remove_up_to_any_counters(resolved_amount, iterated.clone())
            };
            let effect = Effect::for_each(resolved_filter, vec![inner]);
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::PutSticker { target, action } => match target {
            TargetAst::Object(filter, explicit_target_span, _)
                if explicit_target_span.is_none() =>
            {
                let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
                let tag = ctx.next_tag("stickered");
                let tag_key = tag.clone();
                let choose_effect = crate::effects::ChooseObjectsEffect::new(
                    resolved_filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag_key.clone(),
                )
                .in_zone(choice_zone);
                ctx.last_object_tag = Some(tag.clone());
                Ok((
                    vec![
                        Effect::new(choose_effect),
                        Effect::put_sticker(ChooseSpec::Tagged(tag_key), *action),
                    ],
                    Vec::new(),
                ))
            }
            _ => compile_effect_for_target(target, ctx, |spec| Effect::put_sticker(spec, *action)),
        },
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::UnlockRoomDoor) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let room_filter = ObjectFilter::default()
                .with_subtype(Subtype::Room)
                .you_control()
                .in_zone(Zone::Battlefield);
            Ok((
                vec![Effect::unlock_room_door(
                    subject.into_player_filter(),
                    room_filter,
                )],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { target, duration }) => {
            compile_tagged_effect_for_target(target, ctx, "switched_pt", |spec| {
                Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec(
                        spec,
                        crate::continuous::Modification::SwitchPowerToughness,
                        duration.clone(),
                    )
                    .require_creature_target(),
                )
            })
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll {
            filter,
            power,
            toughness,
            multiplier,
            duration,
        }) => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let scaled_stat = |value: Value| {
                if *multiplier == 1 {
                    value
                } else {
                    Value::Scaled(Box::new(value), *multiplier)
                }
            };
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        ChooseSpec::Iterated,
                        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                            power: if *power {
                                scaled_stat(Value::PowerOf(Box::new(ChooseSpec::Iterated)))
                            } else {
                                Value::Fixed(0)
                            },
                            toughness: if *toughness {
                                scaled_stat(Value::ToughnessOf(Box::new(ChooseSpec::Iterated)))
                            } else {
                                Value::Fixed(0)
                            },
                        },
                        duration.clone(),
                    )
                    .require_creature_target(),
                )],
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard {
            count,
            random,
            any_number,
            filter,
            tag,
        }) => {
            let count_names_complete_discard_set = filter.as_ref().is_some_and(|filter| {
                matches!(count.unhinted(), Value::Count(count_filter) if count_filter == filter)
            });
            let discard_references_revealed_hand_choice = filter.as_ref().is_some_and(|filter| {
                filter.zone == Some(Zone::Hand)
                    && filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
            });
            let resolved_filter = if let Some(filter) = filter {
                let mut resolved = resolve_it_tag(filter, &current_reference_env(ctx))?;
                if resolved.zone.is_none() {
                    resolved.zone = Some(Zone::Hand);
                }
                if discard_references_revealed_hand_choice
                    && resolved.zone == Some(Zone::Hand)
                    && let Some(revealed_player) = ctx.last_revealed_player_filter.clone()
                {
                    resolved.owner = Some(revealed_player);
                    resolved.controller = None;
                }
                Some(resolved)
            } else {
                None
            };
            let explicit_full_hand_owner = count
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand)
                .then(|| match count.unhinted() {
                    Value::CardsInHand(player) => Some(player.clone()),
                    Value::Count(filter) => filter.owner.clone(),
                    _ => None,
                })
                .flatten();
            let (resolved_player, choices) =
                if matches!(subject_verb.subject.player, PlayerAst::Implicit) {
                    if let Some(inferred_player) = resolved_filter
                        .as_ref()
                        .and_then(|filter| {
                            if discard_references_revealed_hand_choice
                                && filter.zone == Some(Zone::Hand)
                            {
                                ctx.last_revealed_player_filter.clone()
                            } else {
                                player_filter_from_object_filter(filter)
                            }
                        })
                        // An explicit possessive full-hand phrase supplies its
                        // own actor. Do not let a prior damaged/targeted player
                        // rebind `Discard all the cards in your hand`.
                        .or(explicit_full_hand_owner)
                        .or_else(|| {
                            ctx.last_player_filter
                                .clone()
                                .filter(|player| !matches!(player, PlayerFilter::Defending))
                        })
                    {
                        (inferred_player, Vec::new())
                    } else {
                        let subject = LoweredSubject::resolve_affected_player(
                            subject_verb.subject.player,
                            ctx,
                            true,
                            true,
                            true,
                        )?;
                        (subject.into_player_filter(), subject.into_choices())
                    }
                } else if matches!(subject_verb.subject.player, PlayerAst::That)
                    && let Some(inferred_player) = resolved_filter
                        .as_ref()
                        .and_then(player_filter_from_object_filter)
                {
                    (inferred_player, Vec::new())
                } else {
                    let subject = LoweredSubject::resolve_affected_player(
                        subject_verb.subject.player,
                        ctx,
                        true,
                        true,
                        true,
                    )?;
                    (subject.into_player_filter(), subject.into_choices())
                };
            let subject = LoweredSubject::from_resolved(resolved_player.clone(), choices);
            let mut resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            subject.apply_player_refs_to_value(&mut resolved_count, ctx);
            let resolved_filter = resolved_filter
                .map(|resolved| subject.bind_discard_filter(&resolved, ctx))
                .transpose()?;
            if count_names_complete_discard_set && let Some(filter) = resolved_filter.as_ref() {
                replace_complete_discard_count_filter(&mut resolved_count, filter);
            }
            let tag = tag.clone().unwrap_or_else(|| crate::tag::TagRef::of(ctx.next_tag("discarded")));
            ctx.last_object_tag = Some(tag.clone().into());
            let effect = Effect::new(
                crate::effects::DiscardEffect::new_with_filter(
                    resolved_count,
                    resolved_player,
                    *random,
                    resolved_filter,
                )
                .with_any_number(*any_number)
                .with_tag(tag),
            );
            Ok((vec![effect], subject.into_choices()))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DiscardHand) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let (player_filter, choices) = subject.into_parts();
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                Effect::discard_hand,
                Effect::discard_hand_player,
            )
        }
        SubjectVerbActionAst::Counters(CounterActionAst::PoisonCounters { count }) => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            true,
            true,
            true,
            false,
            Effect::poison_counters,
            Effect::poison_counters_player,
        ),
        SubjectVerbActionAst::Counters(CounterActionAst::EnergyCounters { count }) => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            true,
            true,
            true,
            false,
            Effect::energy_counters,
            Effect::energy_counters_player,
        ),
        SubjectVerbActionAst::Counters(CounterActionAst::ExperienceCounters { count }) => {
            compile_subject_verb_player_value_effect(
                role,
                player,
                count,
                ctx,
                true,
                true,
                true,
                false,
                Effect::experience_counters,
                Effect::experience_counters_player,
            )
        }
        SubjectVerbActionAst::Counters(CounterActionAst::TicketCounters { count }) => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            true,
            true,
            true,
            false,
            Effect::ticket_counters,
            Effect::ticket_counters_player,
        ),
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayEnergy { amount }) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let amount = subject.bind_player_refs_in_value(amount, ctx)?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    Effect::new(crate::effects::PayEnergyEffect::new(
                        amount.clone(),
                        ChooseSpec::Player(PlayerFilter::You),
                    ))
                },
                |filter| {
                    Effect::new(crate::effects::PayEnergyEffect::new(
                        amount.clone(),
                        ChooseSpec::Player(filter),
                    ))
                },
            )
        }
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyEnergy { min_amount }) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    Effect::new(crate::effects::PayAnyEnergyEffect::new(
                        ChooseSpec::Player(PlayerFilter::You),
                        *min_amount,
                    ))
                },
                |filter| {
                    Effect::new(crate::effects::PayAnyEnergyEffect::new(
                        ChooseSpec::Player(filter),
                        *min_amount,
                    ))
                },
            )
        }
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife { min_amount }) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    Effect::new(crate::effects::PayAnyLifeEffect::new(
                        ChooseSpec::Player(PlayerFilter::You),
                        *min_amount,
                    ))
                },
                |filter| {
                    Effect::new(crate::effects::PayAnyLifeEffect::new(
                        ChooseSpec::Player(filter),
                        *min_amount,
                    ))
                },
            )
        }
        SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
            cost,
            x_value,
            x_maximum,
        }) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let x_value = x_value
                .as_ref()
                .map(|value| subject.resolve_object_refs_and_bind_player_refs_in_value(value, ctx))
                .transpose()?;
            let x_maximum = x_maximum
                .as_ref()
                .map(|value| subject.resolve_object_refs_and_bind_player_refs_in_value(value, ctx))
                .transpose()?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    let mut effect = crate::effects::PayManaEffect::new(
                        cost.clone(),
                        ChooseSpec::Player(PlayerFilter::You),
                    );
                    if let Some(x_value) = x_value.clone() {
                        effect = effect.with_x_value(x_value);
                    }
                    if let Some(x_maximum) = x_maximum.clone() {
                        effect = effect.with_x_maximum(x_maximum);
                    }
                    Effect::new(effect)
                },
                |filter| {
                    let mut effect = crate::effects::PayManaEffect::new(
                        cost.clone(),
                        ChooseSpec::Player(filter),
                    );
                    if let Some(x_value) = x_value.clone() {
                        effect = effect.with_x_value(x_value);
                    }
                    if let Some(x_maximum) = x_maximum.clone() {
                        effect = effect.with_x_maximum(x_maximum);
                    }
                    Effect::new(effect)
                },
            )
        }
        SubjectVerbActionAst::Mana(ManaActionAst::DoubleManaPool) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::double_mana_pool_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::Mana(ManaActionAst::EmptyManaPool) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::empty_mana_pool_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetLifeTotal { amount }) => compile_subject_verb_player_value_effect(
            role,
            player,
            amount,
            ctx,
            true,
            true,
            true,
            false,
            |value| Effect::set_life_total_player(value, PlayerFilter::You),
            Effect::set_life_total_player,
        ),
        SubjectVerbActionAst::Game(GameActionAst::ReverseTurnOrder) => Ok((
            vec![Effect::new(crate::effects::ReverseTurnOrderEffect::new())],
            Vec::new(),
        )),
        SubjectVerbActionAst::Game(GameActionAst::EndTurn) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::end_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::Game(GameActionAst::EndCombatPhase) => Ok((vec![Effect::end_combat_phase()], Vec::new())),
        SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipTurn) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhases) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_combat_phases_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipNextCombatPhaseThisTurn) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_next_combat_phase_this_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipMainPhasesThisTurn) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_main_phases_this_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhasesThisTurn) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_combat_phases_this_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipDrawStep) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_draw_step_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalPhases { phases }) => {
            Ok((vec![Effect::additional_phases(phases.clone())], Vec::new()))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PlayFromGraveyardUntilEot) => {
            compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                Effect::grant_play_from_graveyard_until_eot(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::Control(ControlActionAst::ControlPlayer {
            player: target_player,
            duration,
        }) => {
            let _subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let (start, duration) = match duration {
                ControlDurationAst::UntilEndOfTurn => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::UntilYourNextTurnEnd => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::DuringNextTurn => (
                    crate::game_state::PlayerControlStart::NextTurn,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::Forever => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::Forever,
                ),
                ControlDurationAst::AsLongAsYouControlSource => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilSourceLeaves,
                ),
            };

            let mut choices = Vec::new();
            if let PlayerFilter::Target(inner) = target_player {
                let spec = ChooseSpec::target(ChooseSpec::Player((**inner).clone()));
                choices.push(spec);
                ctx.last_player_filter = Some(PlayerFilter::target_player());
            } else {
                ctx.last_player_filter = Some(target_player.clone());
            }

            let effect = Effect::control_player(target_player.clone(), start, duration);
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Stack(StackActionAst::ReduceNextSpellCostThisTurn { filter, reduction }) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let mut player_filter = subject.into_player_filter();
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            Ok((
                vec![Effect::new(
                    crate::effects::GrantNextSpellCostReductionEffect::new(
                        player_filter,
                        resolved_filter,
                        reduction.clone(),
                    ),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn {
            filter,
            reduction,
            duration,
            next_only,
        }) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let mut player_filter = subject.into_player_filter();
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            let reduction_effect = if *next_only {
                crate::effects::GrantNextSpellCostReductionEffect::next_matching_this_turn(
                    player_filter,
                    resolved_filter,
                    reduction.clone(),
                )
            } else {
                crate::effects::GrantNextSpellCostReductionEffect::all_matching_until(
                    player_filter,
                    resolved_filter,
                    reduction.clone(),
                    duration.clone(),
                )
            };
            Ok((vec![Effect::new(reduction_effect)], Vec::new()))
        }
        SubjectVerbActionAst::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn { filter, ability }) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let mut player_filter = subject.clone_player_filter();
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            let lowered = lower_granted_abilities_ast_to_object_abilities(std::slice::from_ref(
                ability.as_ref(),
            ))?;
            if lowered.is_empty() {
                return Err(CardTextError::ParseError(
                    "temporary next-spell grant did not lower to an object ability".to_string(),
                ));
            }
            Ok((
                lowered
                    .into_iter()
                    .map(|ability| {
                        Effect::grant_next_spell_ability_this_turn(
                            player_filter.clone(),
                            resolved_filter.clone(),
                            ability,
                        )
                    })
                    .collect(),
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::RingTemptsYou) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::ring_tempts_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::VentureIntoDungeon {
            undercity_if_no_active,
        }) => compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
            if *undercity_if_no_active {
                Effect::venture_into_undercity_player(subject.into_player_filter())
            } else {
                Effect::venture_into_dungeon_player(subject.into_player_filter())
            }
        }),
        SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeMonarch) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::become_monarch_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::TakeInitiative) => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::take_initiative_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateEmblem { emblem }) => {
            let emblem = compile_emblem_description(emblem)?;
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let filter = subject.clone_player_filter();
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::create_emblem(emblem)
            } else {
                Effect::for_players(filter, vec![Effect::create_emblem(emblem)])
            };
            Ok((vec![effect], subject.into_choices()))
        }
        SubjectVerbActionAst::Game(GameActionAst::LoseGame) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let (player_filter, choices) = subject.into_parts();
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                Effect::lose_the_game,
                Effect::lose_the_game_player,
            )
        }
        SubjectVerbActionAst::Game(GameActionAst::WinGame) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let (player_filter, choices) = subject.into_parts();
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                Effect::win_the_game,
                Effect::win_the_game_player,
            )
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::Detain { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect =
                tag_object_target_effect(Effect::detain(spec.clone()), &spec, ctx, "detained");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad { target, duration }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty()
                || matches!(
                    spec.base(),
                    ChooseSpec::Object(filter) if filter.set_quantifier_surface().is_some()
                ) {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect = tag_object_target_effect(
                Effect::goad_for(spec.clone(), duration.clone()),
                &spec,
                ctx,
                "goaded",
            );
            track_selected_object_player_provenance(&spec, ctx);
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::Suspect { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect =
                tag_object_target_effect(Effect::suspect(spec.clone()), &spec, ctx, "suspected");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::ClearSuspected { target }) => {
            let Some(target) = target else {
                return Ok(Some((vec![Effect::clear_all_suspected()], Vec::new())));
            };
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect = tag_object_target_effect(
                Effect::clear_suspected(spec.clone()),
                &spec,
                ctx,
                "no_longer_suspected",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Damage(DamageActionAst::HealDamage { target, amount }) => {
            compile_tagged_effect_for_target(target, ctx, "healed", |spec| match amount {
                Some(amount) => Effect::heal_damage(spec, amount.clone()),
                None => Effect::heal_all_damage(spec),
            })
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat { target }) => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::RemoveFromCombatEffect::with_spec(
                    spec.clone(),
                )),
                &spec,
                ctx,
                "removed_from_combat",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Flip { target }) => {
            compile_tagged_effect_for_target(target, ctx, "flipped", Effect::flip)
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate {
            target,
            follow_up_effects,
        }) => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut follow_ups = Vec::new();
            if !follow_up_effects.is_empty() {
                let saved_last_object_tag = ctx.last_object_tag.clone();
                ctx.last_object_tag = Some((crate::tag::CompilerReferenceTag::It.bind()).into());
                let (compiled_follow_ups, follow_up_choices) =
                    compile_effects(follow_up_effects, ctx)?;
                follow_ups = compiled_follow_ups;
                for choice in follow_up_choices {
                    push_choice(&mut choices, choice);
                }
                ctx.last_object_tag = saved_last_object_tag;
            }
            let regenerate = crate::effects::RegenerateEffect::new(
                spec.clone(),
                crate::effect::Until::EndOfTurn,
            )
            .with_follow_up_effects(follow_ups);
            let effect =
                tag_object_target_effect(Effect::new(regenerate), &spec, ctx, "regenerated");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::RegenerateAll { filter }) => {
            let (mut prelude, choices) = target_context_prelude_for_filter(filter);
            prelude.push(Effect::regenerate(
                ChooseSpec::all(filter.clone()),
                crate::effect::Until::EndOfTurn,
            ));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
            filter,
            count,
            target,
            one_of_referenced_set,
        }) => {
            if let Some(target) = target {
                let (effects, mut choices) =
                    compile_tagged_effect_for_target(target, ctx, "sacrificed", |spec| {
                        Effect::new(crate::effects::SacrificeTargetEffect::new(spec))
                    })?;
                let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
                let chooser = subject.into_player_filter();
                ctx.last_player_filter = Some(chooser);
                for choice in subject.into_choices() {
                    push_choice(&mut choices, choice);
                }
                return Ok(Some((effects, choices)));
            }
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let chooser = subject.clone_player_filter();
            let target_prelude = subject.target_prelude();
            let refs = current_reference_env(ctx);
            let bare_it_with_source_antecedent = !*one_of_referenced_set
                && !refs.iterated_object
                && refs.has_source_object_antecedent()
                && refs.known_last_object_tag().is_none_or(|tag| {
                    tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        && !refs.last_it_choice_is_set
                })
                && object_filter_as_tagged_reference(filter).is_some_and(|tag| {
                    tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                });
            let mut resolved_filter = if bare_it_with_source_antecedent {
                ObjectFilter::source()
            } else {
                match subject.bind_sacrifice_filter(filter, ctx) {
                    Ok(resolved) => resolved,
                    Err(_)
                        if filter.tagged_constraints.len() == 1
                            && filter.tagged_constraints[0].tag.as_str()
                                == crate::tag::CompilerReferenceTag::It.as_str() =>
                    {
                        ObjectFilter::source()
                    }
                    Err(err) => return Err(err),
                }
            };
            if resolved_filter.source {
                if *count != 1 {
                    return Err(CardTextError::ParseError(format!(
                        "source sacrifice only supports count 1 (count: {})",
                        count
                    )));
                }
                if !matches!(chooser, PlayerFilter::You) {
                    return Err(CardTextError::ParseError(
                        "source sacrifice requires source controller chooser".to_string(),
                    ));
                }
                let mut effects = target_prelude;
                let source = resolved_filter
                    .source_surface
                    .clone()
                    .map(|surface| {
                        ChooseSpec::Source.with_surface_hint(
                            crate::target::ChooseSpecSurfaceHint::SourceReference(surface),
                        )
                    })
                    .unwrap_or(ChooseSpec::Source);
                effects.push(Effect::new(crate::effects::SacrificeTargetEffect::new(
                    source,
                )));
                return Ok(Some((effects, subject.into_choices())));
            }
            if !*one_of_referenced_set
                && *count == 1
                && let Some(tag) = object_filter_as_tagged_reference(&resolved_filter)
            {
                let mut effects = target_prelude;
                effects.push(Effect::new(crate::effects::SacrificeTargetEffect::new(
                    ChooseSpec::tagged(tag),
                )));
                return Ok(Some((effects, subject.into_choices())));
            }

            if *one_of_referenced_set {
                resolved_filter.set_one_of_tagged_set_surface(true);
            }
            let tag = ctx.next_tag("sacrificed");
            ctx.last_object_tag = Some(tag.clone());
            let choose = Effect::choose_objects(
                resolved_filter,
                *count as usize,
                chooser.clone(),
                tag.clone(),
            );
            let sacrifice =
                Effect::sacrifice_player(ObjectFilter::tagged(tag), *count, chooser.clone());
            let mut effects = target_prelude;
            effects.push(choose);
            effects.push(sacrifice);
            Ok((effects, subject.into_choices()))
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }) => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let chooser = subject.clone_player_filter();
            let resolved_filter = subject.bind_sacrifice_filter(filter, ctx)?;
            let count = Value::Count(resolved_filter.clone());
            let effect = Effect::sacrifice_player(resolved_filter, count, chooser.clone());
            let mut effects = subject.target_prelude();
            effects.push(effect);
            Ok((effects, subject.into_choices()))
        }
        _ => return Ok(None),
    };
    result.map(Some)
}
