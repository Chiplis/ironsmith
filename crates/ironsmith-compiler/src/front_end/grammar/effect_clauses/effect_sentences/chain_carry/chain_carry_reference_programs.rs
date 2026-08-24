use super::*;

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
                SubjectVerbActionAst::Draw { count }
                    if matches!(
                        subject_verb.subject.player,
                        PlayerAst::You | PlayerAst::Implicit
                    ) =>
                {
                    nested_draw_values.push(count.clone());
                }
                SubjectVerbActionAst::LoseLife { amount }
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

    fn reconcile_list(effects: &mut Vec<EffectAst>, tokens: &[OwnedLexToken]) {
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
                SubjectVerbActionAst::Draw { count }
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
                SubjectVerbActionAst::LoseLife { amount }
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
        reconcile_list(inner, tokens);
    } else {
        reconcile_list(effects, tokens);
    }
}

pub(super) fn bind_source_exiled_effect(effect: EffectAst, bind: bool) -> EffectAst {
    if bind {
        EffectAst::TagAffected {
            effect: Box::new(effect),
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
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

    let chosen_tag = crate::tag::CompilerReferenceTag::PhaseOutSelection.key();
    let mut available = ObjectFilter::default().in_zone(Zone::Battlefield);
    available
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let mut phase_out_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    phase_out_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: chosen_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Some(EffectAst::MayByPlayer {
        player: PlayerAst::You,
        effects: vec![
            EffectAst::ChooseObjects {
                filter: available,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag,
            },
            EffectAst::subject_verb_phase_out_all(phase_out_filter),
        ],
    })
}

pub fn collapse_for_each_player_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::ForEachPlayer { .. },
                EffectAst::ForEachPlayer {
                    effects: followup_effects,
                },
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
                EffectAst::ForEachPlayer {
                    effects: first_effects,
                },
                EffectAst::ForEachPlayer {
                    effects: mut followup_effects,
                },
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
            (EffectAst::ForEachObject { filter, .. }, followup) => {
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
            (EffectAst::ForEachObject { effects: inner, .. }, followup) => {
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
        TargetAst::Tagged(tag, _) if tag.as_str() != IT_TAG => Some(tag.clone()),
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .find(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() != IT_TAG
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
                SubjectVerbActionAst::MoveToZone { target, .. }
                | SubjectVerbActionAst::MayMoveToZone { target, .. }
                | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
                | SubjectVerbActionAst::PutOntoBattlefield { target, .. }
                | SubjectVerbActionAst::TurnFaceUp { target }
                | SubjectVerbActionAst::ReturnToHand { target, .. },
            ..
        }) => explicit_tagged_target(target),
        EffectAst::May { effects } | EffectAst::MayByPlayer { effects, .. }
            if effects.len() == 1 =>
        {
            explicit_effect_object_tag(&effects[0])
        }
        EffectAst::TagAffected { tag, .. } if tag.as_str() != IT_TAG => Some(tag.clone()),
        _ => None,
    }
}

pub(super) fn explicit_effect_object_target(effect: &EffectAst) -> Option<ChooseSpec> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone { target, .. }
                | SubjectVerbActionAst::MayMoveToZone { target, .. }
                | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
                | SubjectVerbActionAst::PutOntoBattlefield { target, .. }
                | SubjectVerbActionAst::TurnFaceUp { target }
                | SubjectVerbActionAst::ReturnToHand { target, .. },
            ..
        }) => explicit_target_choose_spec(target),
        EffectAst::May { effects } | EffectAst::MayByPlayer { effects, .. }
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
        Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) => {
            Value::PowerOf(Box::new(
                target
                    .clone()
                    .with_surface_hints(spec.surface_hints().iter().cloned()),
            ))
        }
        Value::ToughnessOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) => {
            Value::ToughnessOf(Box::new(
                target
                    .clone()
                    .with_surface_hints(spec.surface_hints().iter().cloned()),
            ))
        }
        Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) => {
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
                PredicateAst::TaggedMatches(tag, filter)
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

pub fn explicit_player_for_carry(effect: &EffectAst) -> Option<CarryContext> {
    if matches!(effect, EffectAst::ForEachPlayer { .. }) {
        return Some(CarryContext::ForEachPlayer);
    }
    if let EffectAst::ForEachTargetPlayers { count, .. } = effect {
        return Some(CarryContext::ForEachTargetPlayers(*count));
    }
    if matches!(effect, EffectAst::ForEachOpponent { .. }) {
        return Some(CarryContext::ForEachOpponent);
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::TargetOnly { target, .. } = &subject_verb.action
        && let Some(context) = player_target_carry_context(target)
    {
        return Some(context);
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::Exile { target, .. } = &subject_verb.action
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. },
        ..
    }) = effect
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileAll { filter, .. },
        ..
    }) = effect
        && let Some(owner) = filter.owner.as_ref()
        && let Some(player) = player_ast_from_filter_for_carry(owner)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::TapAll { filter },
        ..
    }) = effect
        && let Some(controller) = filter.controller.as_ref()
        && let Some(player) = player_ast_from_filter_for_carry(controller)
    {
        // In a clause such as "they tap all lands they control and lose all
        // unspent mana", the explicit player is represented by the tapped
        // objects' controller rather than the SubjectVerb subject.  Retain it
        // for the coordinated implicit player action that follows.
        return Some(CarryContext::Player(player));
    }
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ChoosePlayer { .. },
            ..
        })
    ) {
        return Some(CarryContext::Player(PlayerAst::That));
    }

    let player = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => {
            if !matches!(player, PlayerAst::Implicit) {
                *player
            } else if !matches!(chooser, PlayerAst::Implicit) {
                *chooser
            } else {
                return None;
            }
        }
        EffectAst::SubjectVerb(_) => subject_verb_player_action_player(effect)?,
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => *player,
        _ => return None,
    };

    if matches!(player, PlayerAst::Implicit) {
        None
    } else {
        Some(CarryContext::Player(player))
    }
}

pub fn effect_uses_implicit_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => matches!(*chooser, PlayerAst::Implicit) || matches!(*player, PlayerAst::Implicit),
        EffectAst::SubjectVerb(_) => {
            matches!(
                subject_verb_player_action_player(effect),
                Some(PlayerAst::Implicit)
            )
        }
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
            matches!(*player, PlayerAst::Implicit)
        }
        _ => false,
    }
}

pub(super) fn effect_uses_that_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => matches!(*chooser, PlayerAst::That) || matches!(*player, PlayerAst::That),
        EffectAst::SubjectVerb(_) => {
            matches!(
                subject_verb_player_action_player(effect),
                Some(PlayerAst::That)
            )
        }
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
            matches!(*player, PlayerAst::That)
        }
        _ => false,
    }
}

pub(super) fn subject_verb_player_action_player_mut(
    effect: &mut EffectAst,
) -> Option<&mut PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { player, .. },
            ..
        }) => Some(player),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::Draw { .. }
                | SubjectVerbActionAst::LoseLife { .. }
                | SubjectVerbActionAst::PayLife { .. }
                | SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::RevealHand
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::Scry { .. }
                | SubjectVerbActionAst::Surveil { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::DiscardHand
                | SubjectVerbActionAst::PoisonCounters { .. }
                | SubjectVerbActionAst::EnergyCounters { .. }
                | SubjectVerbActionAst::ExperienceCounters { .. }
                | SubjectVerbActionAst::TicketCounters { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayAnyLife { .. }
                | SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::DoubleManaPool
                | SubjectVerbActionAst::EmptyManaPool
                | SubjectVerbActionAst::SetLifeTotal { .. }
                | SubjectVerbActionAst::SkipTurn
                | SubjectVerbActionAst::EndTurn
                | SubjectVerbActionAst::SkipCombatPhases
                | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
                | SubjectVerbActionAst::SkipMainPhasesThisTurn
                | SubjectVerbActionAst::SkipCombatPhasesThisTurn
                | SubjectVerbActionAst::SkipDrawStep
                | SubjectVerbActionAst::RingTemptsYou
                | SubjectVerbActionAst::VentureIntoDungeon { .. }
                | SubjectVerbActionAst::BecomeMonarch
                | SubjectVerbActionAst::TakeInitiative
                | SubjectVerbActionAst::CreateEmblem { .. }
                | SubjectVerbActionAst::LoseGame
                | SubjectVerbActionAst::WinGame
                | SubjectVerbActionAst::FlipCoin
                | SubjectVerbActionAst::FlipCoinFaceOnly
                | SubjectVerbActionAst::RollDie { .. }
                | SubjectVerbActionAst::RollDiceChooseResult { .. }
                | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
                | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary { .. }
                | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { .. }
                | SubjectVerbActionAst::ExileTopOfLibrary { .. }
                | SubjectVerbActionAst::ReorderGraveyard
                | SubjectVerbActionAst::ChooseColor
                | SubjectVerbActionAst::ChooseCardType { .. }
                | SubjectVerbActionAst::ChooseNamedOption { .. }
                | SubjectVerbActionAst::ChooseCreatureType { .. }
                | SubjectVerbActionAst::ChooseLandType { .. }
                | SubjectVerbActionAst::ChooseCardName { .. }
                | SubjectVerbActionAst::ChoosePlayer { .. }
                | SubjectVerbActionAst::NoteLifeTotal
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::ReturnToBattlefield { .. }
                | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
                | SubjectVerbActionAst::ReturnToHand { .. }
                | SubjectVerbActionAst::ReturnAllToHand { .. }
                | SubjectVerbActionAst::MoveToZone { .. }
                | SubjectVerbActionAst::AdditionalLandPlays { .. }
                | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::Attach { .. }
                | SubjectVerbActionAst::ShuffleLibrary,
        }) => Some(player),
        _ => None,
    }
}

pub(super) fn subject_verb_player_action_player(effect: &EffectAst) -> Option<PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { player, .. },
            ..
        }) => Some(*player),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::Draw { .. }
                | SubjectVerbActionAst::LoseLife { .. }
                | SubjectVerbActionAst::PayLife { .. }
                | SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::RevealHand
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::Scry { .. }
                | SubjectVerbActionAst::Surveil { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::DiscardHand
                | SubjectVerbActionAst::PoisonCounters { .. }
                | SubjectVerbActionAst::EnergyCounters { .. }
                | SubjectVerbActionAst::ExperienceCounters { .. }
                | SubjectVerbActionAst::TicketCounters { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayAnyLife { .. }
                | SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::DoubleManaPool
                | SubjectVerbActionAst::EmptyManaPool
                | SubjectVerbActionAst::SetLifeTotal { .. }
                | SubjectVerbActionAst::SkipTurn
                | SubjectVerbActionAst::EndTurn
                | SubjectVerbActionAst::SkipCombatPhases
                | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
                | SubjectVerbActionAst::SkipMainPhasesThisTurn
                | SubjectVerbActionAst::SkipCombatPhasesThisTurn
                | SubjectVerbActionAst::SkipDrawStep
                | SubjectVerbActionAst::RingTemptsYou
                | SubjectVerbActionAst::VentureIntoDungeon { .. }
                | SubjectVerbActionAst::BecomeMonarch
                | SubjectVerbActionAst::TakeInitiative
                | SubjectVerbActionAst::CreateEmblem { .. }
                | SubjectVerbActionAst::LoseGame
                | SubjectVerbActionAst::WinGame
                | SubjectVerbActionAst::FlipCoin
                | SubjectVerbActionAst::FlipCoinFaceOnly
                | SubjectVerbActionAst::RollDie { .. }
                | SubjectVerbActionAst::RollDiceChooseResult { .. }
                | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
                | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary { .. }
                | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { .. }
                | SubjectVerbActionAst::ExileTopOfLibrary { .. }
                | SubjectVerbActionAst::ReorderGraveyard
                | SubjectVerbActionAst::ChooseColor
                | SubjectVerbActionAst::ChooseCardType { .. }
                | SubjectVerbActionAst::ChooseNamedOption { .. }
                | SubjectVerbActionAst::ChooseCreatureType { .. }
                | SubjectVerbActionAst::ChooseLandType { .. }
                | SubjectVerbActionAst::ChooseCardName { .. }
                | SubjectVerbActionAst::ChoosePlayer { .. }
                | SubjectVerbActionAst::NoteLifeTotal
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::ReturnToBattlefield { .. }
                | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
                | SubjectVerbActionAst::ReturnToHand { .. }
                | SubjectVerbActionAst::ReturnAllToHand { .. }
                | SubjectVerbActionAst::MoveToZone { .. }
                | SubjectVerbActionAst::AdditionalLandPlays { .. }
                | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::Attach { .. }
                | SubjectVerbActionAst::ShuffleLibrary,
        }) => Some(*player),
        _ => None,
    }
}

pub fn maybe_apply_carried_player(effect: &mut EffectAst, carried_context: CarryContext) {
    match carried_context {
        CarryContext::Player(carried_player) => {
            // When carrying an explicit target player/opponent into an implicit clause,
            // bind to the previously selected target ("that player") instead of creating
            // a fresh explicit target. This preserves shared-target semantics for chains
            // like "Target player mills..., draws..., and loses...".
            let carried_player = match carried_player {
                PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
                other => other,
            };
            match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::SearchLibrary { player, .. },
                    ..
                }) => {
                    // A bare `search` is imperative: its omitted actor is the
                    // spell or ability's controller. A target introduced by
                    // "target player's library" is the library owner, not a
                    // grammatical subject to carry into the chooser slot.
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                EffectAst::SubjectVerb(_) => {
                    if let Some(player) = subject_verb_player_action_player_mut(effect)
                        && *player == PlayerAst::Implicit
                    {
                        *player = carried_player;
                    }
                }
                EffectAst::ChooseObjects { player, .. }
                | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                _ => {}
            }
        }
        CarryContext::ForEachPlayer => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachPlayer {
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachTargetPlayers(count) => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachTargetPlayers {
                    count,
                    filter: PlayerFilter::Any,
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachOpponent => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachOpponent {
                    effects: vec![wrapped],
                };
            }
        }
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
            action: SubjectVerbActionAst::ExileTopOfLibrary { .. } | SubjectVerbActionAst::RevealTop,
        }) = effect
        && subject.player == PlayerAst::ItsController
    {
        subject.player = match carried_player {
            PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
            player => player,
        };
    }

    let imperative_collection_move = facts.imperative_collection_move
        && matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone { .. },
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
                action: SubjectVerbActionAst::ReturnToBattlefield { .. }
                    | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
                    | SubjectVerbActionAst::ReturnToHand { .. }
                    | SubjectVerbActionAst::ReturnAllToHand { .. },
            })
        );
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
                || (matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: SubjectVerbSubjectAst {
                            player: PlayerAst::Implicit,
                            ..
                        },
                        action: SubjectVerbActionAst::Draw { .. },
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
                        action: SubjectVerbActionAst::Scry { .. }
                            | SubjectVerbActionAst::Surveil { .. },
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
                    action: SubjectVerbActionAst::Draw { .. }
                        | SubjectVerbActionAst::Scry { .. }
                        | SubjectVerbActionAst::Surveil { .. },
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
        action: SubjectVerbActionAst::CreateTokenWithMods { player, .. },
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

pub fn bind_implicit_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::RetargetStackObject { .. },
        }) => {
            if matches!(subject.player, PlayerAst::Implicit) {
                subject.player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CopySpell {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::CopySpellForEachTarget {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::CastTagged {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player: effect_player,
                    ..
                },
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    player: effect_player,
                    chooser,
                    ..
                },
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
            if matches!(*chooser, PlayerAst::Implicit) {
                *chooser = player;
            }
        }
        EffectAst::SubjectVerb(_) => {
            if let Some(effect_player) = subject_verb_player_action_player_mut(effect)
                && matches!(*effect_player, PlayerAst::Implicit)
            {
                *effect_player = player;
            }
        }
        EffectAst::ChooseObjects {
            player: effect_player,
            ..
        }
        | EffectAst::ChooseObjectsWithAggregateConstraint {
            player: effect_player,
            ..
        }
        | EffectAst::ChooseObjectsAcrossZones {
            player: effect_player,
            ..
        } => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
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
    parse_player_may_prefix(&mut input).ok()
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
