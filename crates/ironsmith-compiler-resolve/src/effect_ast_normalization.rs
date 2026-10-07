use crate::cards::builders::{
    CharacteristicActionAst, ChoiceActionAst, ConditionalEffectAst, CounterActionAst,
    DamageActionAst, DelayedEffectAst, EffectAst, GrantActionAst, LibraryActionAst,
    LifeResourceActionAst, ObjectChoiceEffectAst, PermissionEffectAst, PredicateAst,
    RevealLookActionAst, StatChangeActionAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    TargetAst, VoteEffectAst, ZoneMoveActionAst,
};
use crate::effect::Value;
use ironsmith_compiler_semantic::model::ForEachEffectAst;
use ironsmith_core::ValueSurfaceHint;

fn source_counter_removal(effect: &EffectAst) -> Option<crate::object::CounterType> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters {
                amount,
                target: TargetAst::Source(_),
                counter_type: Some(counter_type),
                up_to: false,
                distributed_across_all: false,
                all_of_them: false,
            }),
        ..
    }) = effect
    else {
        return None;
    };
    matches!(amount.unhinted(), Value::CountersOnSource(kind) if kind == counter_type)
        .then_some(*counter_type)
}

fn bind_damage_amount_to_removed_counter_count(
    effect: &mut EffectAst,
    counter_type: crate::object::CounterType,
) -> usize {
    let mut bound = 0;
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect {
        let amount = match action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                amount,
                ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
                amount, ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageToRecipients {
                amount,
                ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageBySources {
                amount, ..
            }) => Some(amount),
            _ => None,
        };
        if let Some(amount) = amount
            && matches!(
                amount.unhinted(),
                Value::EventValue(crate::effect::EventValueSpec::Amount)
            )
        {
            let hints = amount.surface_hints().to_vec();
            *amount = Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::Removed)
                .with_counter_type(Some(counter_type)),
            )
            .with_surface_hints(hints);
            bound += 1;
        }
    }
    super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
        for child in nested {
            bound += bind_damage_amount_to_removed_counter_count(child, counter_type);
        }
    });
    bound
}

fn is_removed_counter_damage_fanout_member(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => matches!(
            action,
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageToRecipients { .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageBySources { .. })
        ),
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. }) => {
            !effects.is_empty() && effects.iter().all(is_removed_counter_damage_fanout_member)
        }
        _ => false,
    }
}

fn bind_removed_counter_damage_fanout(effects: &mut [EffectAst]) -> bool {
    let [removal, damage @ ..] = effects else {
        return false;
    };
    let Some(counter_type) = source_counter_removal(removal) else {
        return false;
    };
    if damage.is_empty() || !damage.iter().all(is_removed_counter_damage_fanout_member) {
        return false;
    }
    let mut rebound = damage.to_vec();
    let bound = rebound
        .iter_mut()
        .map(|effect| bind_damage_amount_to_removed_counter_count(effect, counter_type))
        .sum::<usize>();
    if bound < 2 {
        return false;
    }
    damage.clone_from_slice(&rebound);
    true
}

pub fn normalize_effects_ast(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut normalized = effects.to_vec();
    normalize_effects_ast_in_place(&mut normalized);
    normalized
}

pub fn normalize_effects_ast_in_place(effects: &mut Vec<EffectAst>) {
    bind_typed_where_x_references(effects, None);
    normalize_effects_vec(effects);
    release_unbound_tapped_this_way_markers(effects);
}

/// "Tap all creatures target player controls. ... choose up to that many
/// creatures tapped this way": the set is the creatures the tap actually
/// tapped, not every matching creature ("those creatures"). Record the
/// matching untapped creatures just before the tap and bind the marker to
/// that record.
fn bind_tapped_this_way_to_tap_all(effects: &mut Vec<EffectAst>) {
    use ironsmith_core::tag::TagKeyWalk;
    let marker = crate::tag::CompilerReferenceTag::TappedThisWay.as_str();
    let mut index = 0;
    while index < effects.len() {
        let tap_filter = match sentence_tail(&effects[index]) {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PermanentState(
                        crate::cards::builders::PermanentStateActionAst::TapAll { filter },
                    ),
                ..
            }) => Some(filter.clone()),
            _ => None,
        };
        let Some(mut tap_filter) = tap_filter else {
            index += 1;
            continue;
        };
        let mut referenced = false;
        for later in &effects[index + 1..] {
            later.for_each_tag_key(&mut |tag| referenced |= tag.as_str() == marker);
        }
        if !referenced {
            index += 1;
            continue;
        }
        let result = crate::tag::CompilerReferenceTag::TappedThisWayResult.bind();
        for later in &mut effects[index + 1..] {
            later.map_tag_keys(&mut |tag| {
                if tag.as_str() == marker {
                    *tag = result.key.clone();
                }
            });
        }
        tap_filter.untapped = true;
        let record = EffectAst::subject_verb_tag_matching_objects(
            tap_filter,
            vec![crate::zone::Zone::Battlefield],
            result,
        );
        index += if insert_into_sentence(effects, index, record) {
            2
        } else {
            1
        };
    }
}

/// Insert `effect` so it runs just before `effects[at]`, inside that
/// instruction's authored sentence when it has one (the sentence keeps its
/// resolution boundary). Returns whether a new top-level entry was added.
fn insert_into_sentence(effects: &mut Vec<EffectAst>, at: usize, effect: EffectAst) -> bool {
    if let Some(EffectAst::SourceSentence { effects: inner, .. }) = effects.get_mut(at) {
        inner.insert(0, effect);
        return false;
    }
    effects.insert(at, effect);
    true
}

fn release_unbound_tapped_this_way_markers(effects: &mut [EffectAst]) {
    use ironsmith_core::tag::TagKeyWalk;
    let marker = crate::tag::CompilerReferenceTag::TappedThisWay.as_str();
    for effect in effects {
        effect.map_tag_keys(&mut |tag| {
            if tag.as_str() == marker {
                *tag = crate::tag::CompilerReferenceTag::It.key();
            }
        });
    }
}

/// "You may search your library ...": the search's implicit chooser is the
/// player given the option, not whichever player was mentioned last (a
/// target named by a preceding condition, for example).
fn bind_may_player_to_implicit_search_choosers(effect: &mut EffectAst) {
    use crate::cards::builders::{ObjectChoiceEffectAst, PlayerAst};
    let EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
        player: may_player @ PlayerAst::You,
        effects,
    }) = effect
    else {
        return;
    };
    for inner in effects {
        if let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            player: chooser @ PlayerAst::Implicit,
            search_mode: Some(_),
            ..
        }) = inner
        {
            *chooser = *may_player;
        }
    }
}

/// "You may cast a spell from your hand without paying its mana cost if it
/// has the same name as a spell that was cast this turn": `it` is the spell
/// being cast, so the name test restricts which spell may be cast. Tag the
/// comparison set first, then cast only a spell sharing a name with it.
fn bind_same_name_cast_condition_to_cast_filter(effects: &mut Vec<EffectAst>) {
    use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
    let mut index = 0;
    while index < effects.len() {
        let EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate:
                PredicateAst::CountComparison {
                    count:
                        crate::static_abilities::AnthemCountExpression::MatchingFilter(comparison_set),
                    comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                    ..
                },
            effects: gated,
        }) = &effects[index]
        else {
            index += 1;
            continue;
        };
        let names_the_cast_spell = |constraint: &TaggedObjectConstraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::SameNameAsTagged
        };
        let [
            EffectAst::Permissions(
                PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost { .. },
            ),
        ] = gated.as_slice()
        else {
            index += 1;
            continue;
        };
        if comparison_set.tagged_constraints.len() != 1
            || !comparison_set
                .tagged_constraints
                .iter()
                .all(names_the_cast_spell)
        {
            index += 1;
            continue;
        }
        let mut set_filter = comparison_set.clone();
        set_filter.tagged_constraints.clear();
        let zones = set_filter.zone.into_iter().collect::<Vec<_>>();
        let set_tag =
            ironsmith_compiler_semantic::tag::TagRef::of(ironsmith_core::SPELLS_CAST_THIS_TURN_TAG);
        let EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { effects: gated, .. }) =
            effects.remove(index)
        else {
            unreachable!("trailing condition shape was proven above");
        };
        let mut cast = gated;
        if let Some(EffectAst::Permissions(
            PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost { filter, .. },
        )) = cast.first_mut()
        {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: set_tag.key.clone(),
                relation: TaggedOpbjectRelation::SameNameAsTagged,
            });
        }
        effects.insert(
            index,
            EffectAst::subject_verb_tag_matching_objects(set_filter, zones, set_tag),
        );
        for (offset, effect) in cast.into_iter().enumerate() {
            effects.insert(index + 1 + offset, effect);
        }
        index += 2;
    }
}

/// "If target opponent controls more lands than you, ...": when a condition
/// is where the spell's player target is first named, declare that target
/// ahead of the conditional so the predicate reads the chosen player.
fn declare_predicate_introduced_player_targets(effects: &mut Vec<EffectAst>) {
    use crate::cards::builders::{PlayerAst, PlayerPredicateAst, TargetAst};
    let mut index = 0;
    while index < effects.len() {
        let introduced = match &effects[index] {
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate:
                    PredicateAst::Player(PlayerPredicateAst::PlayerControlsMoreThanYou {
                        player: player @ (PlayerAst::Target | PlayerAst::TargetOpponent),
                        ..
                    }),
                ..
            }) => Some(*player),
            _ => None,
        };
        let already_declared = effects[..index].iter().any(|earlier| {
            matches!(
                crate::cards::builders::primary_target_from_effect(earlier),
                Some(TargetAst::Player(_, Some(_)))
            )
        });
        if let Some(player) = introduced
            && !already_declared
        {
            let filter = if matches!(player, PlayerAst::TargetOpponent) {
                crate::target::PlayerFilter::Opponent
            } else {
                crate::target::PlayerFilter::Any
            };
            effects.insert(
                index,
                EffectAst::subject_verb_target_only(TargetAst::Player(
                    filter,
                    Some(crate::diagnostics::TextSpan::synthetic()),
                )),
            );
            index += 1;
        }
        index += 1;
    }
}

fn typed_where_x_binding(effect: &EffectAst) -> Option<Value> {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return None;
    };
    let SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { count, .. }) =
        &subject_verb.action
    else {
        return None;
    };
    let Value::SurfaceHinted { value, hints } = count else {
        return None;
    };
    hints
        .contains(&ValueSurfaceHint::WhereXIs)
        .then(|| value.as_ref().clone())
}

fn replace_bound_x_in_value(value: &mut Value, replacement: &Value) {
    match value {
        Value::X => *value = replacement.clone(),
        Value::XTimes(multiplier) => {
            let multiplier = *multiplier;
            *value = if multiplier == 1 {
                replacement.clone()
            } else if let Value::Fixed(fixed) = replacement {
                Value::Fixed(fixed * multiplier)
            } else {
                Value::Scaled(Box::new(replacement.clone()), multiplier)
            };
        }
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => replace_bound_x_in_value(value, replacement),
        Value::Add(left, right) | Value::Min(left, right) => {
            replace_bound_x_in_value(left, replacement);
            replace_bound_x_in_value(right, replacement);
        }
        _ => {}
    }
}

fn replace_bound_x_in_predicate(predicate: &mut PredicateAst, replacement: &Value) {
    match predicate {
        PredicateAst::ValueComparison { left, right, .. } => {
            replace_bound_x_in_value(left, replacement);
            replace_bound_x_in_value(right, replacement);
        }
        PredicateAst::ValueIsPrime(value) => replace_bound_x_in_value(value, replacement),
        PredicateAst::Not(inner) => replace_bound_x_in_predicate(inner, replacement),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            replace_bound_x_in_predicate(left, replacement);
            replace_bound_x_in_predicate(right, replacement);
        }
        _ => {}
    }
}

fn bind_typed_where_x_references(effects: &mut [EffectAst], inherited: Option<Value>) {
    let mut binding = inherited;
    for effect in effects {
        match effect {
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            })
            | EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                ..
            } => {
                if let Some(replacement) = binding.as_ref() {
                    replace_bound_x_in_predicate(predicate, replacement);
                }
                bind_typed_where_x_references(if_true, binding.clone());
                bind_typed_where_x_references(if_false, binding.clone());
            }
            EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects })
            | EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless {
                predicate,
                effects,
            }) => {
                if let Some(replacement) = binding.as_ref() {
                    replace_bound_x_in_predicate(predicate, replacement);
                }
                bind_typed_where_x_references(effects, binding.clone());
            }
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { modes, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::VillainousChoice { modes, .. }) => {
                for mode in modes {
                    bind_typed_where_x_references(&mut mode.effects, binding.clone());
                }
            }
            EffectAst::Conditionals(ConditionalEffectAst::IfEffectDidNotHappen {
                effect,
                otherwise,
            }) => {
                bind_typed_where_x_references(
                    std::slice::from_mut(effect.as_mut()),
                    binding.clone(),
                );
                bind_typed_where_x_references(otherwise, binding.clone());
            }
            EffectAst::Conditionals(ConditionalEffectAst::IfEffectResult {
                effect,
                if_true,
                ..
            }) => {
                bind_typed_where_x_references(
                    std::slice::from_mut(effect.as_mut()),
                    binding.clone(),
                );
                bind_typed_where_x_references(if_true, binding.clone());
            }
            EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
                bind_typed_where_x_references(
                    std::slice::from_mut(effect.as_mut()),
                    binding.clone(),
                )
            }
            _ => super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
                bind_typed_where_x_references(nested, binding.clone())
            }),
        }

        if let Some(next_binding) = typed_where_x_binding(effect) {
            binding = Some(next_binding);
        }
    }
}

/// "If you win the flip, target Orc creature gets +2/+0 ... If you lose the
/// flip, it gets -0/-2 ...": a target named inside one result branch is chosen
/// when the ability is put on the stack, so a sibling branch's `it` names the
/// same object. Declare it ahead of the branches; the introducing branch then
/// reads the declaration, so neither branch reads a tag only the other sets.
fn declare_branch_introduced_object_targets(effects: &mut Vec<EffectAst>) {
    fn peel_mut(effect: &mut EffectAst) -> &mut EffectAst {
        let single =
            matches!(effect, EffectAst::SourceSentence { effects, .. } if effects.len() == 1);
        if !single {
            return effect;
        }
        let EffectAst::SourceSentence { effects, .. } = effect else {
            unreachable!("checked above");
        };
        peel_mut(&mut effects[0])
    }
    fn peel(effect: &EffectAst) -> &EffectAst {
        match effect {
            EffectAst::SourceSentence { effects, .. } if effects.len() == 1 => peel(&effects[0]),
            _ => effect,
        }
    }
    fn result_branch_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
        match peel_mut(effect) {
            EffectAst::Conditionals(
                ConditionalEffectAst::IfResult { effects, .. }
                | ConditionalEffectAst::ResolvedIfResult { effects, .. },
            ) => Some(effects),
            _ => None,
        }
    }
    fn introduced_target_mut(branch: &mut [EffectAst]) -> Option<&mut TargetAst> {
        match peel_mut(branch.first_mut()?) {
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => match action {
                SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { target, .. })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                    target,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget { target, .. })
                | SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                    target, ..
                }) if matches!(target, TargetAst::Object(_, Some(_), _)) => Some(target),
                _ => None,
            },
            _ => None,
        }
    }
    fn branch_reads_it(branch: &[EffectAst]) -> bool {
        branch
            .first()
            .map(peel)
            .and_then(crate::cards::builders::primary_target_from_effect)
            .is_some_and(|target| {
                matches!(target, TargetAst::Tagged(tag, _)
                    if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
            })
    }
    let mut index = 0;
    while index < effects.len() {
        let (head, tail) = effects.split_at_mut(index + 1);
        let sibling_reads_it = tail
            .iter_mut()
            .any(|effect| result_branch_mut(effect).is_some_and(|branch| branch_reads_it(branch)));
        let declared = if sibling_reads_it {
            result_branch_mut(&mut head[index])
                .and_then(|branch| introduced_target_mut(branch))
                .map(|target| {
                    std::mem::replace(
                        target,
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    )
                })
        } else {
            None
        };
        if let Some(target) = declared {
            // Declare ahead of the result producer ("Flip a coin"), so each
            // branch still reads that producer's result.
            let mut first_branch = index;
            while first_branch > 0 && result_branch_mut(&mut effects[first_branch - 1]).is_some() {
                first_branch -= 1;
            }
            let insert_at = first_branch.saturating_sub(1);
            if insert_into_sentence(
                effects,
                insert_at,
                EffectAst::subject_verb_target_only(target),
            ) {
                index += 1;
            }
        }
        index += 1;
    }
}

/// "<it> can't <act> for as long as <duration>" is a restriction the
/// resolution creates on the referenced object (CR 611.2), not an ability
/// granted to it: a granted static never sees resolution tags, and a
/// phased-out permanent's abilities don't function (CR 702.26b).
fn durational_anaphoric_restriction_grant_to_cant(effect: &mut EffectAst) {
    use crate::effect::{Restriction, Until};
    fn single_filter_mut(restriction: &mut Restriction) -> Option<&mut crate::ObjectFilter> {
        match restriction {
            Restriction::PreventDamageFrom {
                sources: filter, ..
            }
            | Restriction::ActivateLoyaltyAbilitiesOf(filter)
            | Restriction::MustAttack(filter)
            | Restriction::MustBlock(filter)
            | Restriction::Attack(filter)
            | Restriction::Block(filter)
            | Restriction::Untap(filter)
            | Restriction::BeBlocked(filter)
            | Restriction::BeDestroyed(filter)
            | Restriction::BeRegenerated(filter)
            | Restriction::BeSacrificed(filter)
        | Restriction::BecomeSuspected(filter)
        | Restriction::MaximumBlockers { filter, .. }
            | Restriction::HaveCountersPlaced(filter)
            | Restriction::BeTargeted(filter)
            | Restriction::TurnFaceUp(filter)
            | Restriction::Transform(filter)
            | Restriction::PhaseOut(filter)
            | Restriction::PhaseIn(filter)
            | Restriction::AttackOrBlock(filter)
            | Restriction::ActivateAbilitiesOf(filter)
            | Restriction::ActivateTapAbilitiesOf(filter)
            | Restriction::ActivateNonManaAbilitiesOf(filter) => Some(filter),
            _ => None,
        }
    }
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return;
    };
    let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
        target: TargetAst::Tagged(target_tag, _),
        abilities,
        duration,
        condition: None,
        ..
    }) = &subject_verb.action
    else {
        return;
    };
    if matches!(duration, Until::Forever) {
        return;
    }
    let [crate::cards::builders::GrantedAbilityAst::StaticAbility(ability)] = abilities.as_slice()
    else {
        return;
    };
    let crate::cards::builders::StaticAbilityAst::Static(ability) = ability.as_ref() else {
        return;
    };
    let ironsmith_core::StaticAbilityPayload::RuleRestriction {
        restriction,
        additional_restrictions,
        ..
    } = &ability.payload
    else {
        return;
    };
    if !additional_restrictions.is_empty() {
        return;
    }
    let mut restriction = restriction.clone();
    let Some(filter) = single_filter_mut(&mut restriction) else {
        return;
    };
    let it = crate::tag::CompilerReferenceTag::It.as_str();
    let [constraint] = filter.tagged_constraints.as_mut_slice() else {
        return;
    };
    if constraint.tag.as_str() != it
        || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return;
    }
    let mut bare = filter.clone();
    bare.tagged_constraints.clear();
    if bare != crate::ObjectFilter::default() {
        return;
    }
    let target_key: crate::TagKey = target_tag.clone().into();
    filter.tagged_constraints[0].tag = target_key;
    let duration = duration.clone();
    *effect = EffectAst::subject_verb_cant(restriction, duration, None);
}

/// A reflexive follow-up naming the tapped object belongs to each tap in
/// the preceding player loop. Keeping it outside merges local results and
/// loses both the tapped identity and "that player" before targets are chosen.
fn transport_tap_quantity_reflexive_into_player_loop(effects: &mut Vec<EffectAst>) {
    use ironsmith_core::tag::TagKeyWalk;
    let mut index = 0;
    while index + 1 < effects.len() {
        let follower = sentence_tail(&effects[index + 1]);
        let is_reflexive = matches!(
            follower,
            EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                predicate: crate::cards::builders::IfResultPredicate::Did,
                ..
            })
        );
        let mut names_tapped = false;
        follower.for_each_tag_key(&mut |tag| {
            names_tapped |= tag.as_str() == crate::tag::PRIOR_TAPPED_OBJECT_QUANTITY_TAG
        });
        if !is_reflexive || !names_tapped {
            index += 1;
            continue;
        }
        let body = match sentence_tail(&effects[index]) {
            EffectAst::ForEach(
                ForEachEffectAst::ForEachOpponent { effects }
                | ForEachEffectAst::ForEachPlayer { effects }
                | ForEachEffectAst::ForEachPlayersFiltered { effects, .. },
            ) => effects,
            _ => {
                index += 1;
                continue;
            }
        };
        if !body.last().map(sentence_tail).is_some_and(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::PermanentState(
                        crate::cards::builders::PermanentStateActionAst::Tap { .. }
                    ),
                    ..
                })
            )
        }) {
            index += 1;
            continue;
        }
        let follower = effects.remove(index + 1);
        let EffectAst::ForEach(
            ForEachEffectAst::ForEachOpponent { effects: body }
            | ForEachEffectAst::ForEachPlayer { effects: body }
            | ForEachEffectAst::ForEachPlayersFiltered { effects: body, .. },
        ) = sentence_tail_mut(&mut effects[index])
        else {
            unreachable!("checked player loop");
        };
        body.push(follower);
        index += 1;
    }
}

/// A result follow-up to reflexive damage resolves in that triggered body,
/// where its damage receipt exists, rather than in the parent instruction.
fn transport_reflexive_damage_followups(effects: &mut Vec<EffectAst>) {
    let mut index = 0;
    while index + 1 < effects.len() {
        let follows_damage = matches!(
            sentence_tail(&effects[index + 1]),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::ExcessDamageDealt,
                ..
            })
        );
        let has_reflexive_damage = match sentence_tail(&effects[index]) {
            EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects: body, .. }) => {
                body.iter().any(|effect| {
                    matches!(
                        sentence_tail(effect),
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::Damage(_),
                            ..
                        })
                    )
                })
            }
            _ => false,
        };
        if follows_damage && has_reflexive_damage {
            let follower = effects.remove(index + 1);
            if let EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                effects: body, ..
            }) = sentence_tail_mut(&mut effects[index])
            {
                body.push(follower);
            }
        } else {
            index += 1;
        }
    }
}

/// A same-name library search consumes the object chosen only by a successful
/// result branch. Its complete search action (including disposition and shuffle)
/// therefore belongs to that branch even when authored as a following sentence.
/// This correlation is established before references are resolved, from the
/// choice producer and the search filter's typed relation, never from card text.
fn correlate_result_gated_choice_search_followups(effects: &mut Vec<EffectAst>) {
    let mut index = 0;
    while index + 1 < effects.len() {
        let choice_tag = match sentence_tail(&effects[index]) {
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::Did,
                effects: branch,
            }) => match branch.as_slice() {
                [choice] => match sentence_tail(choice) {
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                        tag, count, count_value: None, ..
                    }) if count.min == 1 && count.max == Some(1) => Some(tag.as_str()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let consumes_choice = choice_tag.is_some_and(|choice_tag| {
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                    filter, ..
                }),
                ..
            }) = sentence_tail(&effects[index + 1]) else {
                return false;
            };
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                    && (constraint.tag.as_str() == choice_tag
                        || constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        || constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str())
            })
        });
        if consumes_choice {
            let followup = effects.remove(index + 1);
            let EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects: branch, .. }) =
                sentence_tail_mut(&mut effects[index])
            else {
                unreachable!("checked result branch");
            };
            branch.push(followup);
        }
        index += 1;
    }
}

fn normalize_effects_vec(effects: &mut Vec<EffectAst>) {
    transport_reflexive_damage_followups(effects);
    transport_tap_quantity_reflexive_into_player_loop(effects);
    bind_tapped_this_way_to_tap_all(effects);
    declare_predicate_introduced_player_targets(effects);
    declare_branch_introduced_object_targets(effects);
    bind_same_name_cast_condition_to_cast_filter(effects);
    for effect in effects.iter_mut() {
        bind_may_player_to_implicit_search_choosers(effect);
        normalize_nested_effects(effect);
        collapse_single_nested_coordination(effect);
        // The generic "you may" parser can wrap an already optional repeat
        // marker. Expose that marker so cross-sentence normalization can bind
        // it to the preceding process, and leave exactly one continuation choice.
        let optional_repeat = match effect {
            EffectAst::Permissions(PermissionEffectAst::May { effects })
            | EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player: crate::cards::builders::PlayerAst::You,
                effects,
            }) => matches!(
                effects.as_slice(),
                [EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessMay)]
            ),
            _ => false,
        };
        if optional_repeat {
            *effect = EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessMay);
        }
        normalize_singular_source_exiled_move(effect);
        durational_anaphoric_restriction_grant_to_cant(effect);
        bind_coordinated_rest_sacrifice_to_chosen_complement(effect);
    }
    // A full-card parse can normalize a named source reference only after the
    // narrow removal/damage sentence recognizer has run. Recover the same
    // typed shared-result provenance at this common AST boundary once the
    // producer is provably a source-bound counter removal and every following
    // member belongs to the damage fanout.
    bind_removed_counter_damage_fanout(effects);
    correlate_result_gated_choice_search_followups(effects);
    bind_explicit_chosen_object_followups(effects);
    bind_other_group_to_explicit_target_choice(effects);
    correlate_conditional_quantified_choice_followups(effects);
    correlate_split_for_each_player_choice_complements(effects);
    bind_all_players_subtype_choices_to_destroy_exclusion(effects);
    bind_all_players_subtype_choices_to_return_inclusion(effects);
    bind_quantified_choice_collections_to_destroy_followups(effects);
    bind_counted_set_followups(effects);
    bind_drawn_cards_to_reveal_followups(effects);
    bind_until_next_turn_permissions_to_prior_exiled_collection(effects);
    transport_exiled_card_permissions_into_delayed_trigger(effects);
    fold_per_object_random_player_control_changes(effects);
    bind_choice_remainder_to_choice_domain(effects);
    bind_consult_remainder_to_revealed_collection(effects);
    bind_plural_total_power_to_prior_object_set(effects);
    if let Some(rewritten) = rewrite_repeat_process(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_repeat_process_may(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_repeat_process_once(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_return_as_aura(effects) {
        *effects = rewritten;
    }
    effects.retain(|effect| !is_noop_effect(effect));
}

/// "Each player puts a vow counter on a creature they control and sacrifices
/// the rest": `the rest` is the complement of the singular object the shared
/// subject just chose, within the same domain. Rewrite the unresolved
/// remainder sacrifice to "sacrifice all <domain> other than it".
fn bind_coordinated_rest_sacrifice_to_chosen_complement(effect: &mut EffectAst) {
    let EffectAst::Coordination(coordination) = effect else {
        return;
    };
    let mut chosen_domain: Option<crate::filter::ObjectFilter> = None;
    for member in &mut coordination.members {
        for effect in &mut member.effects {
            let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
                continue;
            };
            match action {
                SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                    target: TargetAst::Object(filter, _, _),
                    target_count: Some(count),
                    ..
                }) if count.is_single() => {
                    chosen_domain = Some(filter.clone());
                }
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
                    filter,
                    target: None,
                    ..
                }) if filter.tagged_constraints.len() == 1
                    && filter.tagged_constraints[0].tag.as_str()
                        == crate::tag::CompilerReferenceTag::Rest.as_str()
                    && filter.tagged_constraints[0].relation
                        == crate::filter::TaggedOpbjectRelation::IsTaggedObject =>
                {
                    let Some(domain) = chosen_domain.as_ref() else {
                        continue;
                    };
                    *action = SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll {
                        filter: domain
                            .clone()
                            .not_tagged(crate::tag::CompilerReferenceTag::It.key()),
                    });
                }
                _ => {}
            }
        }
    }
}

/// "For each nonland permanent, choose a player at random. Then each player
/// gains control of each permanent for which they were chosen. Untap those
/// permanents.": each chosen player belongs to exactly one permanent, so the
/// control change happens per permanent, under the player chosen for it.
/// Fold the participant loop into the object loop (the unfolded shape would
/// hand every permanent to every player in turn) and untap that same domain.
fn fold_per_object_random_player_control_changes(effects: &mut Vec<EffectAst>) {
    use crate::cards::builders::{ControlActionAst, PermanentStateActionAst};
    let mut index = 1;
    while index < effects.len() {
        let (before, after) = effects.split_at_mut(index);
        let EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter: domain,
            effects: loop_body,
        }) = sentence_tail_mut(&mut before[index - 1])
        else {
            index += 1;
            continue;
        };
        let chooses_random_player = matches!(
            loop_body.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Choices(ChoiceActionAst::ChoosePlayer {
                    random: true,
                    ..
                }),
                ..
            })]
        );
        let EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: player_body,
        }) = sentence_tail(&after[0])
        else {
            index += 1;
            continue;
        };
        let control_change = match player_body.as_slice() {
            [
                EffectAst::SubjectVerb(
                    subject_verb @ SubjectVerbEffectAst {
                        action:
                            SubjectVerbActionAst::Control(ControlActionAst::GainControl {
                                target: TargetAst::Object(filter, None, _),
                                controller_reference: None,
                                ..
                            }),
                        ..
                    },
                ),
            ] if chooses_random_player
                && filter.tagged_constraints.is_empty()
                && filter.zone == domain.zone =>
            {
                let mut control_change = subject_verb.clone();
                if let SubjectVerbActionAst::Control(ControlActionAst::GainControl {
                    target, ..
                }) = &mut control_change.action
                {
                    *target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
                }
                control_change
            }
            _ => {
                index += 1;
                continue;
            }
        };
        // The control change names the player chosen in this iteration.
        // Give that choice an explicit identity so the reference pass and
        // lowering agree on the player tag.
        if let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Choices(ChoiceActionAst::ChoosePlayer { tag, .. }),
                ..
            }),
        ] = loop_body.as_mut_slice()
            && tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        {
            *tag = ironsmith_compiler_semantic::tag::declared_key("chosen_player_for_object");
        }
        loop_body.push(EffectAst::SubjectVerb(control_change));
        let domain = domain.clone();
        effects.remove(index);
        if let Some(next) = effects.get_mut(index)
            && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter }),
                ..
            }) = sentence_tail_mut(next)
            && filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            })
        {
            *filter = domain;
        }
    }
}

/// The last authored effect of a (possibly sentence-wrapped) statement.
fn sentence_tail(effect: &EffectAst) -> &EffectAst {
    match effect {
        EffectAst::SourceSentence { effects, .. } => effects.last().map_or(effect, sentence_tail),
        _ => effect,
    }
}

fn sentence_tail_mut(effect: &mut EffectAst) -> &mut EffectAst {
    let is_wrapped =
        matches!(effect, EffectAst::SourceSentence { effects, .. } if !effects.is_empty());
    if !is_wrapped {
        return effect;
    }
    let EffectAst::SourceSentence { effects, .. } = effect else {
        unreachable!("checked above");
    };
    sentence_tail_mut(effects.last_mut().expect("non-empty sentence"))
}

/// The grammar reads "their total power" as the creatures of an attack-group
/// trigger. After an instruction that removes a whole set of objects
/// ("Destroy all creatures target opponent controls. ... equal to their total
/// power"; "sacrifice any number of other creatures, then ... their total
/// power") the plural pronoun instead names that set, so rebind it to the
/// ordinary object antecedent.
fn bind_plural_total_power_to_prior_object_set(effects: &mut [EffectAst]) {
    use ironsmith_core::tag::TagKeyWalk;
    let group = crate::tag::CompilerReferenceTag::AttackingGroup.as_str();
    let mut after_object_set = false;
    for effect in effects.iter_mut() {
        if after_object_set {
            effect.map_tag_keys(&mut |tag| {
                if tag.as_str() == group {
                    *tag = crate::tag::CompilerReferenceTag::It.key();
                }
            });
        }
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = sentence_tail(effect) {
            match action {
                SubjectVerbActionAst::ZoneMoves(
                    ZoneMoveActionAst::DestroyAll { .. }
                    | ZoneMoveActionAst::ExileAll { .. }
                    | ZoneMoveActionAst::SacrificeAll { .. },
                ) => after_object_set = true,
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
                    filter, ..
                }) if !filter.source => after_object_set = true,
                _ => {}
            }
        }
    }
}

/// Resolve “the rest of the revealed cards” against the two collections
/// exported by the nearest typed library consult. The generic move grammar
/// deliberately leaves `rest` unresolved; this canonical AST pass owns the
/// cross-sentence set difference and never consults source text.
fn bind_consult_remainder_to_revealed_collection(effects: &mut [EffectAst]) {
    let mut latest_consult = None;
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                    all_tag,
                    match_tag,
                    player,
                    ..
                }),
            ..
        }) = effect
        {
            latest_consult = Some((all_tag.clone(), match_tag.clone(), *player));
            continue;
        }

        let Some((all_tag, match_tag, player)) = latest_consult.clone() else {
            continue;
        };
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            continue;
        };
        let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
            target: TargetAst::Tagged(tag, _),
            zone,
            library_order,
            library_order_chooser,
            ..
        }) = &subject_verb.action
        else {
            continue;
        };
        if tag.as_str() != crate::tag::CompilerReferenceTag::Rest.as_str() {
            continue;
        }
        subject_verb.action = if *zone == crate::zone::Zone::Library {
            let Some(order) = *library_order else {
                continue;
            };
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag: all_tag,
                keep_tagged: Some(match_tag),
                order,
                player: if matches!(
                    library_order_chooser,
                    crate::cards::builders::PlayerAst::Implicit
                ) {
                    player
                } else {
                    *library_order_chooser
                },
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            })
        } else {
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                tag: all_tag,
                keep_tagged: match_tag,
                zone: *zone,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            })
        };
    }
}

/// Bind an adjacent object choice's `the rest` consumer to the exact
/// complement of that choice domain. Inside a quantified-player loop, the
/// chosen tag is iteration-local, so the complement must retain the same
/// owner/controller and zone constraints as its producer.
fn bind_choice_remainder_to_choice_domain(effects: &mut [EffectAst]) {
    for index in 1..effects.len() {
        let (before, after) = effects.split_at_mut(index);
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, tag, .. }) =
            &before[index - 1]
        else {
            continue;
        };
        let EffectAst::SubjectVerb(subject_verb) = &mut after[0] else {
            continue;
        };
        let target = match &mut subject_verb.action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { target, .. }) => target,
            _ => continue,
        };
        if !matches!(
            target,
            TargetAst::Tagged(rest, _)
                if rest.as_str() == crate::tag::CompilerReferenceTag::Rest.as_str()
        ) {
            continue;
        }
        *target = TargetAst::Object(filter.clone().not_tagged(tag.clone()), None, None);
    }
}

/// A document-level clause can preserve coordination around a sentence that
/// was already recognized as one coordinated typed effect. The outer
/// one-member wrapper carries no additional operator or scope; retaining it
/// produces a nested runtime sequence and hides the actual executable
/// members from consumers. Canonicalize that redundant wrapper before
/// reference annotation and lowering.
fn collapse_single_nested_coordination(effect: &mut EffectAst) {
    loop {
        let EffectAst::Coordinated { effects, .. } = effect else {
            return;
        };
        let replacement = match effects.as_slice() {
            [nested @ EffectAst::Coordinated { .. }] => Some(nested.clone()),
            [
                sentence @ EffectAst::SourceSentence {
                    effects: sentence_effects,
                    ..
                },
            ] if matches!(sentence_effects.as_slice(), [EffectAst::Coordinated { .. }]) => {
                Some(sentence.clone())
            }
            _ => None,
        };
        let Some(replacement) = replacement else {
            return;
        };
        *effect = replacement;
    }
}

/// Bind a later temporary play permission to the exact collection produced by
/// a prior top-of-library exile, including when that producer lives inside a
/// delayed or quantified wrapper.
///
/// Standalone permission sentences initially use `crate::tag::CompilerReferenceTag::It.as_str()`.  Resolving that
/// through the generic last-object channel is wrong when the intervening
/// producer is wrapped: the wrapper's watched/iterated object remains the
/// generic antecedent even though "it" / "those cards" refers to the newly
/// exiled collection.  Preserve explicit tags and require one unambiguous
/// exile collection before carrying the tag across the boundary.
fn bind_until_next_turn_permissions_to_prior_exiled_collection(effects: &mut [EffectAst]) {
    fn collect_exiled_tags(effect: &EffectAst, tags: &mut Vec<crate::tag::TagKey>) {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                    tags: moved_tags,
                    accumulated_tags,
                    ..
                }),
            ..
        }) = effect
        {
            for tag in moved_tags.iter().chain(accumulated_tags) {
                if !tags.contains(tag) {
                    tags.push(tag.clone().into());
                }
            }
        }
        super::effect_ast_traversal::for_each_nested_effects(effect, true, |nested| {
            for child in nested {
                collect_exiled_tags(child, tags);
            }
        });
    }

    fn rebind_unresolved_permissions(effect: &mut EffectAst, exiled_tag: &crate::tag::TagKey) {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn {
                    tag,
                    ..
                }),
            ..
        }) = effect
            && (tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                || tag.as_str().starts_with("damaged_")
                || tag.as_str().starts_with("pumped_"))
        {
            *tag = ironsmith_compiler_semantic::tag::TagRef::of(exiled_tag.clone());
        }
        super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
            for child in nested {
                rebind_unresolved_permissions(child, exiled_tag);
            }
        });
    }

    let mut prior_exiled_tag = None;
    for effect in effects.iter_mut() {
        if let Some(tag) = prior_exiled_tag.as_ref() {
            rebind_unresolved_permissions(effect, tag);
        }

        let mut tags = Vec::new();
        collect_exiled_tags(effect, &mut tags);
        prior_exiled_tag = match tags.as_slice() {
            [tag] => Some(tag.clone()),
            [] => prior_exiled_tag,
            _ => None,
        };
    }
}

/// "Until end of turn, whenever a creature you control dies, exile the top
/// card of your library. You may play it until the end of your next turn.":
/// the card exists only once the delayed trigger resolves, so a permission
/// naming exactly that trigger's exile result belongs inside its body rather
/// than on the scheduling instruction, where the tag is still empty.
fn transport_exiled_card_permissions_into_delayed_trigger(effects: &mut Vec<EffectAst>) {
    fn delayed_body(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
        match sentence_tail_mut(effect) {
            EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, .. })
            | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration { effects, .. }) => {
                Some(effects)
            }
            _ => None,
        }
    }
    fn body_exiled_tag(body: &[EffectAst]) -> Option<crate::tag::TagKey> {
        let mut found = None;
        for effect in body {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                        tags: moved_tags,
                        ..
                    }),
                ..
            }) = effect
                && let [tag] = moved_tags.as_slice()
            {
                found = Some(tag.key.clone());
            }
        }
        found
    }
    fn permission_tag(effect: &EffectAst) -> Option<&crate::tag::TagKey> {
        if let EffectAst::SourceSentence { effects, .. } = effect {
            return match effects.as_slice() {
                [single] => permission_tag(single),
                _ => None,
            };
        }
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Grants(
                        GrantActionAst::GrantPlayTaggedUntilEndOfTurn { tag, .. }
                        | GrantActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. },
                    ),
                ..
            }) => Some(&tag.key),
            _ => None,
        }
    }
    let mut index = 1;
    while index < effects.len() {
        let (before, after) = effects.split_at_mut(index);
        let moved = if let Some(body) = delayed_body(&mut before[index - 1])
            && let Some(exiled) = body_exiled_tag(body)
            && permission_tag(&after[0]) == Some(&exiled)
        {
            let mut permission = after[0].clone();
            while let EffectAst::SourceSentence { effects, .. } = &mut permission
                && effects.len() == 1
            {
                permission = effects.remove(0);
            }
            body.push(permission);
            true
        } else {
            false
        };
        if moved {
            effects.remove(index);
        } else {
            index += 1;
        }
    }
}

/// one-object move even though the generic non-target subject path defaults
/// object filters to `all`. The explicit singular surface plus the typed
/// source-exile identity are both required before removing that broadening.
fn normalize_singular_source_exiled_move(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target: TargetAst::Object(filter, ..),
                zone,
                target_plural_surface,
                all,
                ..
            }),
        ..
    }) = effect
        && *all
        && !*target_plural_surface
        && *zone == crate::zone::Zone::Graveyard
        && let [constraint] = filter.tagged_constraints.as_slice()
        && constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
        && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        let mut remainder = filter.clone();
        remainder.zone = None;
        remainder.tagged_constraints.clear();
        remainder.union_surface = Default::default();
        if remainder == crate::filter::ObjectFilter::default() {
            *all = false;
        }
        return;
    }

    super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
        for child in nested {
            normalize_singular_source_exiled_move(child);
        }
    });
}

fn single_subtype_choice_family(effect: &EffectAst) -> Option<crate::types::SubtypeFamily> {
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. } => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            single_subtype_choice_family(effect)
        }
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType {
                family, ..
            }) => Some(*family),
            _ => None,
        },
        _ => None,
    }
}

fn all_players_choose_one_subtype_family(
    effect: &EffectAst,
) -> Option<crate::types::SubtypeFamily> {
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. } => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            all_players_choose_one_subtype_family(effect)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            single_subtype_choice_family(effect)
        }
        _ => None,
    }
}

fn filter_is_misbound_chosen_subtype_result(
    filter: &crate::filter::ObjectFilter,
    family: crate::types::SubtypeFamily,
) -> bool {
    if family != crate::types::SubtypeFamily::Creature
        || !matches!(
            filter.card_types.as_slice(),
            [crate::types::CardType::Creature]
        )
        || filter.tagged_constraints.len() != 1
    {
        return false;
    }
    let mut expected = crate::filter::ObjectFilter::creature().match_tagged(
        crate::tag::CompilerReferenceTag::It.bind(),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    // The ordinary object-filter route may leave the implicit permanent zone
    // unstated until lowering. Treat those two forms as the same exact shape.
    if filter.zone.is_none() {
        expected.zone = None;
    }
    filter == &expected
}

/// A subtype chosen "this way" is characteristic data stored on the source,
/// not an object-result collection. Some public multi-sentence routes see the
/// terminal words first and temporarily bind `chosen this way` to the generic
/// object tag. Repair only the exact all-players subtype-choice followed by an
/// otherwise-plain destroy-all object filter, retaining ordinary chosen-object
/// procedures unchanged.
fn bind_all_players_subtype_choices_to_destroy_exclusion(effects: &mut [EffectAst]) {
    for consumer_index in 1..effects.len() {
        let Some(family) = all_players_choose_one_subtype_family(&effects[consumer_index - 1])
        else {
            continue;
        };
        let Some(filter) = direct_destroy_filter_mut(&mut effects[consumer_index]) else {
            continue;
        };
        if !filter_is_misbound_chosen_subtype_result(filter, family) {
            continue;
        }

        filter.tagged_constraints.clear();
        filter.excluded_any_chosen_creature_type = true;
        filter.set_chosen_type_this_way_surface(true);
    }
}

fn all_players_return_all_filter_mut(
    effect: &mut EffectAst,
) -> Option<&mut crate::filter::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            all_players_return_all_filter_mut(effect)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            match effect {
                EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                    SubjectVerbActionAst::ZoneMoves(
                        ZoneMoveActionAst::ReturnAllToBattlefield { filter, .. },
                    ) => Some(filter),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// Bind an all-players subtype choice to a following all-players return. The
/// source stores every simultaneously chosen subtype, so the consumer must
/// match the union of those choices rather than only the last submitted one.
fn bind_all_players_subtype_choices_to_return_inclusion(effects: &mut [EffectAst]) {
    for consumer_index in 1..effects.len() {
        if all_players_choose_one_subtype_family(&effects[consumer_index - 1])
            != Some(crate::types::SubtypeFamily::Creature)
        {
            continue;
        }
        let Some(filter) = all_players_return_all_filter_mut(&mut effects[consumer_index]) else {
            continue;
        };
        if filter.zone != Some(crate::zone::Zone::Graveyard)
            || filter.owner != Some(crate::target::PlayerFilter::IteratedPlayer)
            || !matches!(
                filter.card_types.as_slice(),
                [crate::types::CardType::Creature]
            )
            || filter.prior_effect_action_surface()
                != Some(ironsmith_core::PriorEffectAction::Chosen)
        {
            continue;
        }
        filter.chosen_creature_type = true;
        filter.set_chosen_type_this_way_surface(true);
    }
}

fn quantified_player_choice_effects_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. }) => {
            Some(effects)
        }
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            quantified_player_choice_effects_mut(effect)
        }
        _ => None,
    }
}

fn retag_quantified_choice_collection(effect: &mut EffectAst) -> bool {
    let Some(choice_effects) = quantified_player_choice_effects_mut(effect) else {
        return false;
    };
    let Some(original_tag) = common_object_choice_tag(choice_effects) else {
        return false;
    };
    if !choice_collection_tag_can_accumulate(&original_tag) {
        return false;
    }
    for effect in choice_effects {
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }) = effect
        else {
            return false;
        };
        *tag = crate::tag::CompilerReferenceTag::ChosenObjects.bind();
    }
    true
}

/// Return whether `effect` is made entirely from object-choice producers and
/// whether at least one of those choices is repeated/quantified. This keeps
/// the later binding conservative: ordinary one-off target choices continue
/// to use the normal antecedent resolver, while a union built across players
/// or repetitions receives a durable accumulating tag.
fn choice_collection_producer_is_quantified(effect: &EffectAst) -> Option<bool> {
    fn sequence_kind(effects: &[EffectAst]) -> Option<bool> {
        if effects.is_empty() {
            return None;
        }
        let mut quantified = false;
        for effect in effects {
            quantified |= choice_collection_producer_is_quantified(effect)?;
        }
        Some(quantified)
    }

    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { .. }) => {
            Some(false)
        }
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. }) => {
            sequence_kind(effects).map(|_| true)
        }
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => {
            sequence_kind(effects)
        }
        EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
            choice_collection_producer_is_quantified(effect)
        }
        EffectAst::Coordination(coordination) => {
            let mut quantified = false;
            let mut any = false;
            for effect in coordination.effects() {
                any = true;
                quantified |= choice_collection_producer_is_quantified(effect)?;
            }
            any.then_some(quantified)
        }
        _ => None,
    }
}

fn choice_collection_tag_can_accumulate(tag: &crate::tag::TagKey) -> bool {
    tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        || tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        || tag.as_str().starts_with("participant_choice_l")
}

fn choice_collection_producer_has_accumulating_tags(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
            tag,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
            tag,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            tag, ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            tag, ..
        }) => choice_collection_tag_can_accumulate(tag),
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => {
            !effects.is_empty()
                && effects
                    .iter()
                    .all(choice_collection_producer_has_accumulating_tags)
        }
        EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
            choice_collection_producer_has_accumulating_tags(effect)
        }
        EffectAst::Coordination(coordination) => {
            let mut effects = coordination.effects();
            effects
                .next()
                .is_some_and(choice_collection_producer_has_accumulating_tags)
                && effects.all(choice_collection_producer_has_accumulating_tags)
        }
        _ => false,
    }
}

fn retag_choice_collection_producer(effect: &mut EffectAst, durable_tag: &crate::tag::TagKey) {
    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
            tag,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
            tag,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            tag, ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            tag, ..
        }) => {
            if choice_collection_tag_can_accumulate(tag) {
                *tag = ironsmith_compiler_semantic::tag::TagRef::of(durable_tag.clone());
            }
        }
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => {
            for effect in effects {
                retag_choice_collection_producer(effect, durable_tag);
            }
        }
        EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
            retag_choice_collection_producer(effect, durable_tag)
        }
        EffectAst::Coordination(coordination) => {
            for effect in coordination.effects_mut() {
                retag_choice_collection_producer(effect, durable_tag);
            }
        }
        _ => unreachable!("choice producer shape changed between inspection and retagging"),
    }
}

fn target_only_collection_tag_mut(effect: &mut EffectAst) -> Option<&mut crate::tag::TagKey> {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TargetOnly { .. },
            ..
        })
    ) {
        let target_only = std::mem::replace(
            effect,
            EffectAst::Sequence {
                effects: Vec::new(),
            },
        );
        *effect = EffectAst::TagAffected {
            effect: Box::new(target_only),
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        };
    }
    match effect {
        EffectAst::TagAffected { effect, tag }
            if matches!(
                effect.as_ref(),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::TargetOnly { .. },
                    ..
                })
            ) =>
        {
            Some(&mut tag.key)
        }
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            target_only_collection_tag_mut(effect)
        }
        EffectAst::Coordination(coordination) => {
            let mut effects = coordination.effects_mut();
            let effect = effects.next()?;
            if effects.next().is_some() {
                return None;
            }
            target_only_collection_tag_mut(effect)
        }
        _ => None,
    }
}

/// Give an ordinary object choice the durable chosen-set tag when an
/// immediately following effect explicitly refers to "the chosen objects".
/// The parser uses `__it__` for standalone choices, while the consumer uses
/// the reserved chosen-set alias; assigning the producer that same durable
/// tag preserves both runtime collection identity and authored rendering.
/// "Choose up to one target creature, then airbend all other creatures"
/// (Avatar's Wrath), "Choose target creature you control. Each other
/// creature becomes a copy of that creature" (Nanogene Conversion): a group
/// "other <kind>" immediately following an explicit target declaration of
/// that kind contrasts the group with the declared target, not with the
/// source. Exclude the declared target (the follow-up's `it`) by identity.
fn bind_other_group_to_explicit_target_choice(effects: &mut [EffectAst]) {
    fn declared_target_filter(effect: &EffectAst) -> Option<&crate::filter::ObjectFilter> {
        match effect {
            EffectAst::TagAffected { effect, .. } => declared_target_filter(effect),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::TargetOnly {
                        target,
                        explicit_declaration: true,
                    },
                ..
            }) => target_filter(target),
            _ => None,
        }
    }
    fn target_filter(target: &TargetAst) -> Option<&crate::filter::ObjectFilter> {
        match target {
            TargetAst::Object(filter, Some(_), _) => Some(filter),
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
                target_filter(inner)
            }
            _ => None,
        }
    }
    fn group_filter_mut(effect: &mut EffectAst) -> Option<&mut crate::filter::ObjectFilter> {
        match effect {
            EffectAst::ForEach(ForEachEffectAst::ForEachObject { filter, .. }) => Some(filter),
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => match action {
                SubjectVerbActionAst::KeywordActions(
                    crate::cards::builders::KeywordActionAst::Airbend {
                        target: TargetAst::Object(filter, None, _),
                    },
                )
                | SubjectVerbActionAst::ZoneMoves(
                    ZoneMoveActionAst::DestroyAll { filter, .. }
                    | ZoneMoveActionAst::ExileAll { filter, .. }
                    | ZoneMoveActionAst::ReturnAllToHand { filter, .. },
                )
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                    filter, ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                    filter, ..
                })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach {
                    filter, ..
                }) => Some(filter),
                _ => None,
            },
            _ => None,
        }
    }
    fn bind_pair(previous: &EffectAst, next: &mut EffectAst) {
        let Some(declared) = declared_target_filter(previous) else {
            return;
        };
        let declared_types = declared.card_types.clone();
        if declared_types.is_empty() {
            return;
        }
        let Some(group) = group_filter_mut(next) else {
            return;
        };
        if !group.other
            || !matches!(group.zone, None | Some(crate::zone::Zone::Battlefield))
            || !group
                .card_types
                .iter()
                .any(|card_type| declared_types.contains(card_type))
        {
            return;
        }
        group.other = false;
        group
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::It.key(),
                relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }
    for index in 1..effects.len() {
        let (before, after) = effects.split_at_mut(index);
        bind_pair(&before[index - 1], &mut after[0]);
    }
    // "Choose up to one target creature, then airbend all other creatures":
    // a comma-then sentence keeps its members inside one coordination.
    for effect in effects.iter_mut() {
        if let EffectAst::Coordination(coordination) = effect {
            let mut members: Vec<&mut EffectAst> = coordination.effects_mut().collect();
            for index in 1..members.len() {
                let (before, after) = members.split_at_mut(index);
                bind_pair(before[index - 1], after[0]);
            }
        }
    }
}

fn bind_explicit_chosen_object_followups(effects: &mut [EffectAst]) {
    for consumer_index in 1..effects.len() {
        if !super::compile_support::effect_references_tag(
            &effects[consumer_index],
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str(),
        ) && !iterated_object_effect_uses_prior_choice(&effects[consumer_index])
        {
            continue;
        }
        if choice_collection_producer_has_accumulating_tags(&effects[consumer_index - 1]) {
            retag_choice_collection_producer(
                &mut effects[consumer_index - 1],
                &crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
            );
            continue;
        }

        // Explicit target declarations are also choice producers. A later
        // "the chosen cards" consumer can be separated by an additional cost
        // and its result branch, so bind the nearest preceding declaration
        // rather than requiring adjacency.
        if let Some(tag) = effects[..consumer_index]
            .iter_mut()
            .rev()
            .find_map(target_only_collection_tag_mut)
        {
            *tag = (crate::tag::CompilerReferenceTag::ChosenObjects.bind()).into();
        }
    }
}

fn iterated_object_effect_uses_prior_choice(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. }) => effects.iter().any(|effect| match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy { source, .. }),
                ..
            }) => target_has_demonstrative_it_reference(source),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. }),
                ..
            }) => target_has_demonstrative_it_reference(target),
            _ => false,
        }),
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::SourceSentence { effects, .. } => {
            effects.iter().any(iterated_object_effect_uses_prior_choice)
        }
        EffectAst::Coordination(coordination) => coordination
            .effects()
            .any(iterated_object_effect_uses_prior_choice),
        _ => false,
    }
}

fn target_has_demonstrative_it_reference(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) && matches!(
        filter.source_surface.as_ref(),
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(surface))
            if surface.starts_with("that ")
    )
}

fn normalized_choice_collection_object_kind(
    filter: &crate::filter::ObjectFilter,
) -> crate::filter::ObjectFilter {
    let mut kind = filter.clone();
    kind.zone = None;
    kind.controller = None;
    kind.owner = None;
    kind.other = false;
    kind.tagged_constraints.clear();
    kind
}

fn choice_collection_producer_matches_object_kind(
    effect: &EffectAst,
    expected: &crate::filter::ObjectFilter,
) -> bool {
    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
            filter,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
            filter,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone {
            filter, ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter,
            ..
        }) => normalized_choice_collection_object_kind(filter) == *expected,
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => {
            !effects.is_empty()
                && effects
                    .iter()
                    .all(|effect| choice_collection_producer_matches_object_kind(effect, expected))
        }
        EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
            choice_collection_producer_matches_object_kind(effect, expected)
        }
        EffectAst::Coordination(coordination) => {
            let mut effects = coordination.effects();
            effects.next().is_some_and(|effect| {
                choice_collection_producer_matches_object_kind(effect, expected)
            }) && effects
                .all(|effect| choice_collection_producer_matches_object_kind(effect, expected))
        }
        _ => false,
    }
}

fn direct_destroy_filter_mut(effect: &mut EffectAst) -> Option<&mut crate::filter::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            direct_destroy_filter_mut(effect)
        }
        EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
            direct_destroy_filter_mut(effect)
        }
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. }) => {
                Some(filter)
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                target: TargetAst::Object(filter, _, _),
                ..
            }) => Some(filter),
            _ => None,
        },
        _ => None,
    }
}

fn direct_destroy_filter(effect: &EffectAst) -> Option<&crate::filter::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            direct_destroy_filter(effect)
        }
        EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
            direct_destroy_filter(effect)
        }
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. }) => {
                Some(filter)
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                target: TargetAst::Object(filter, _, _),
                ..
            }) => Some(filter),
            _ => None,
        },
        _ => None,
    }
}

fn direct_destroy_consumes_choice_collection(effect: &EffectAst) -> bool {
    let Some(filter) = direct_destroy_filter(effect) else {
        return false;
    };
    filter.other
        || filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                || constraint.tag.as_str()
                    == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        })
}

/// Bind a quantified/repeated choice union to the immediately following
/// destroy instruction. `ChooseObjectsEffect` accumulates explicit tags at
/// runtime, so changing the producer from the ephemeral `it` tag to the
/// reserved chosen-set tag preserves every iteration. A surface `all other`
/// destroy is then made explicit as the complement of that same union.
fn bind_quantified_choice_collections_to_destroy_followups(effects: &mut [EffectAst]) {
    for consumer_index in 1..effects.len() {
        if !direct_destroy_consumes_choice_collection(&effects[consumer_index]) {
            continue;
        }

        let mut producer_start = consumer_index;
        let mut has_quantified_producer = false;
        while producer_start > 0 {
            let Some(quantified) =
                choice_collection_producer_is_quantified(&effects[producer_start - 1])
            else {
                break;
            };
            has_quantified_producer |= quantified;
            producer_start -= 1;
        }
        if producer_start == consumer_index || !has_quantified_producer {
            continue;
        }

        let producers = &effects[producer_start..consumer_index];
        if !producers
            .iter()
            .all(choice_collection_producer_has_accumulating_tags)
        {
            continue;
        }
        let Some(destroy_filter) = direct_destroy_filter(&effects[consumer_index]) else {
            continue;
        };
        if destroy_filter.other {
            let destroy_kind = normalized_choice_collection_object_kind(destroy_filter);
            if !producers.iter().all(|producer| {
                choice_collection_producer_matches_object_kind(producer, &destroy_kind)
            }) {
                continue;
            }
        }

        let durable_tag = crate::tag::CompilerReferenceTag::ChosenObjects.bind();
        for producer in &mut effects[producer_start..consumer_index] {
            retag_choice_collection_producer(producer, &durable_tag);
        }

        let Some(filter) = direct_destroy_filter_mut(&mut effects[consumer_index]) else {
            continue;
        };
        for constraint in &mut filter.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                constraint.tag = durable_tag.clone().into();
            }
        }
        if filter.other {
            filter.other = false;
            if !filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == durable_tag.key.clone()
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            }) {
                filter
                    .tagged_constraints
                    .push(crate::filter::TaggedObjectConstraint {
                        tag: durable_tag.key.clone(),
                        relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                    });
            }
        }
    }
}

fn direct_destroy_references_chosen_collection(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_slice() else {
                return false;
            };
            direct_destroy_references_chosen_collection(effect)
        }
        EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
            direct_destroy_references_chosen_collection(effect)
        }
        EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { .. })
            ) =>
        {
            super::compile_support::effect_references_tag(
                effect,
                crate::tag::CompilerReferenceTag::ChosenObjects.as_str(),
            )
        }
        _ => false,
    }
}

/// A plural chosen-set continuation can be authored as a new sentence after
/// a leading conditional. If that action consumes the reserved aggregate
/// choice tag, it is both a semantic continuation of the quantified choice
/// and a no-op when the condition is false. Keep the producer and consumer in
/// one branch and give them the same durable tag before reference lowering.
pub fn correlate_conditional_quantified_choice_followups(effects: &mut Vec<EffectAst>) -> bool {
    let mut index = 0usize;
    let mut changed = false;
    while index + 1 < effects.len() {
        let follows_conditional_choice = {
            let (before, after) = effects.split_at_mut(index + 1);
            let EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                if_true,
                if_false,
                ..
            }) = &mut before[index]
            else {
                index += 1;
                continue;
            };
            if !if_false.is_empty()
                || if_true.len() != 1
                || !direct_destroy_references_chosen_collection(&after[0])
            {
                false
            } else {
                retag_quantified_choice_collection(&mut if_true[0])
            }
        };
        if follows_conditional_choice {
            let followup = effects.remove(index + 1);
            let EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_true, .. }) =
                &mut effects[index]
            else {
                unreachable!("the checked effect must remain conditional")
            };
            if_true.push(followup);
            changed = true;
        }
        index += 1;
    }
    changed
}

fn source_sentence_for_each_player_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) => Some(effects),
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            filter: crate::filter::PlayerFilter::Any,
            effects,
            ..
        }) => Some(effects),
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            source_sentence_for_each_player_effects_mut(effect)
        }
        _ => None,
    }
}

fn common_object_choice_tag(effects: &[EffectAst]) -> Option<crate::tag::TagKey> {
    let mut common = None;
    for effect in effects {
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }) = effect
        else {
            return None;
        };
        if let Some(expected) = common.as_ref()
            && expected != tag
        {
            return None;
        }
        common = Some(tag.clone());
    }
    common.map(Into::into)
}

fn replace_correlated_filter_tag(
    filter: &mut crate::filter::ObjectFilter,
    old: &crate::tag::TagKey,
    new: &crate::tag::TagKey,
) -> bool {
    let mut replaced = false;
    for constraint in &mut filter.tagged_constraints {
        if constraint.tag == *old {
            constraint.tag = new.clone();
            replaced = true;
        }
    }
    if let Some(crate::filter::ObjectRef::Tagged(tag)) = &mut filter.in_combat_with
        && tag == old
    {
        *tag = new.clone();
        replaced = true;
    }
    for comparison in &mut filter.no_shared_creature_types_with {
        let comparison_replaced = replace_correlated_filter_tag(comparison, old, new);
        if comparison_replaced && comparison.controller.is_none() {
            // The durable tag contains one choice set per player.  Restrict a
            // relational comparison to the choice made for the active player
            // iteration instead of comparing against every player's choice.
            comparison.controller = Some(crate::filter::PlayerFilter::IteratedPlayer);
        }
        replaced |= comparison_replaced;
    }
    for relation in &mut filter.characteristic_relations {
        let comparison_replaced = replace_correlated_filter_tag(&mut relation.comparison, old, new);
        if comparison_replaced && relation.comparison.controller.is_none() {
            // The durable tag contains one choice set per player. Restrict a
            // relational comparison to the choice made for the active player
            // iteration instead of comparing against every player's choice.
            relation.comparison.controller = Some(crate::filter::PlayerFilter::IteratedPlayer);
        }
        replaced |= comparison_replaced;
    }
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(targets, old, new);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(targets, old, new);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(attached_to, old, new);
    }
    if let Some(combat_partner) = filter.blocked_or_was_blocked_by_this_turn.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(combat_partner, old, new);
    }
    for branch in &mut filter.any_of {
        replaced |= replace_correlated_filter_tag(branch, old, new);
    }
    replaced
}

fn split_player_complement_filter_mut(
    effects: &mut [EffectAst],
) -> Option<&mut crate::filter::ObjectFilter> {
    let [EffectAst::SubjectVerb(subject_verb)] = effects else {
        return None;
    };
    match &mut subject_verb.action {
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }) => {
            Some(filter)
        }
        _ => None,
    }
}

/// Link a player-by-player choice sentence to a following player-by-player
/// complement sentence.  A durable tag accumulates all locked-in choices;
/// the complement filter then excludes that chosen set.  This preserves the
/// required two-phase ordering (all choices first, then the action) without
/// collapsing the sentences into a sequential choose/action loop.
fn correlate_split_for_each_player_choice_complements(effects: &mut [EffectAst]) {
    for index in 0..effects.len().saturating_sub(1) {
        let (before, after) = effects.split_at_mut(index + 1);
        let Some(choice_effects) = source_sentence_for_each_player_effects_mut(&mut before[index])
        else {
            continue;
        };
        if choice_effects.is_empty() {
            continue;
        }
        let Some(original_tag) = common_object_choice_tag(choice_effects) else {
            continue;
        };
        let Some(complement_effects) = source_sentence_for_each_player_effects_mut(&mut after[0])
        else {
            continue;
        };
        let Some(complement_filter) = split_player_complement_filter_mut(complement_effects) else {
            continue;
        };
        if complement_filter.controller != Some(crate::filter::PlayerFilter::IteratedPlayer)
            || !complement_filter.other
        {
            continue;
        }

        let durable_tag = if original_tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        {
            crate::tag::CompilerReferenceTag::ChosenForEachPlayer.bind()
        } else {
            ironsmith_compiler_semantic::tag::TagRef::of(original_tag.clone())
        };
        for effect in choice_effects {
            let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter, tag, ..
            }) = effect
            else {
                continue;
            };
            replace_correlated_filter_tag(filter, &original_tag, &durable_tag);
            *tag = durable_tag.clone();
        }
        replace_correlated_filter_tag(complement_filter, &original_tag, &durable_tag);
        if !complement_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == durable_tag.key.clone()
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            })
        {
            complement_filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: durable_tag.key.clone(),
                    relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                });
        }
        // `other` was an unresolved surface relation to the chosen set.  Once
        // encoded explicitly it must not also exempt the source permanent.
        complement_filter.other = false;
    }
}

fn count_filter(value: &Value) -> Option<&crate::filter::ObjectFilter> {
    match value {
        Value::Count(filter) => Some(filter),
        Value::SurfaceHinted { value, .. } => count_filter(value),
        _ => None,
    }
}

/// Bind a demonstrative plural grant to the set counted by the immediately
/// preceding draw. For example, in "Draw a card for each creature ... Those
/// creatures gain indestructible," the grant applies to the counted creatures,
/// not to the source spell represented by the parser's unresolved `it` target.
fn bind_counted_set_followups(effects: &mut [EffectAst]) {
    for index in 1..effects.len() {
        let (before, after) = effects.split_at_mut(index);
        let EffectAst::SubjectVerb(draw) = &before[index - 1] else {
            continue;
        };
        let SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count }) =
            &draw.action
        else {
            continue;
        };
        let Some(filter) = count_filter(count).cloned() else {
            continue;
        };

        let EffectAst::SubjectVerb(grant) = &mut after[0] else {
            continue;
        };
        let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
            target,
            set_quantifier_surface:
                Some(
                    ironsmith_core::SetQuantifierSurface::Each
                    | ironsmith_core::SetQuantifierSurface::Those,
                ),
            ..
        }) = &mut grant.action
        else {
            continue;
        };
        let unresolved_set_reference = match target {
            TargetAst::Source(_) => true,
            TargetAst::Tagged(tag, _) => {
                tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            }
            _ => false,
        };
        if unresolved_set_reference {
            *target = TargetAst::Object(filter, None, None);
        }
    }
}

/// "Draw a card and reveal it": a draw produces no object result of its own,
/// so a following "reveal it/them" has no antecedent unless the drawn cards
/// are tagged. Tag exactly the cards the draw moved to hand and point the
/// reveal (and, through it, later "them"/"one of them" references) at them.
fn bind_drawn_cards_to_reveal(draw: &mut EffectAst, reveal: &mut EffectAst) -> bool {
    if !matches!(
        draw,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { .. }),
            ..
        })
    ) {
        return false;
    }
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { tag }),
        ..
    }) = reveal
    else {
        return false;
    };
    if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() {
        return false;
    }
    // A sentence-helper "revealed" tag renders as an ordinary "it"/"that
    // card" reference. Reference resolution only mints `_s0_` spellings, so
    // this one never collides with a generated tag; a later draw-and-reveal
    // in the same ability simply rebinds it.
    let drawn = ironsmith_compiler_semantic::tag::sentence_helper_tag("revealed", 0, 1, 0);
    *tag = drawn.clone();
    let inner = std::mem::replace(
        draw,
        EffectAst::Sequence {
            effects: Vec::new(),
        },
    );
    *draw = EffectAst::TagAffected {
        effect: Box::new(inner),
        tag: drawn,
    };
    true
}

fn bind_drawn_cards_to_reveal_followups(effects: &mut [EffectAst]) {
    for index in 1..effects.len() {
        let (before, after) = effects.split_at_mut(index);
        bind_drawn_cards_to_reveal(&mut before[index - 1], &mut after[0]);
    }
}

fn normalize_nested_effects(effect: &mut EffectAst) {
    match effect {
        EffectAst::ForEach(ForEachEffectAst::RepeatProcess { effects, .. }) => {
            // This body already owns its continuation marker and indexes its
            // condition by position. Rewriting the whole vector would wrap an
            // optional repeat in another loop and invalidate that index.
            for effect in effects {
                normalize_nested_effects(effect);
                collapse_single_nested_coordination(effect);
                normalize_singular_source_exiled_move(effect);
            }
        }
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        })
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            normalize_effects_vec(if_true);
            normalize_effects_vec(if_false);
        }
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless { effects, .. })
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { effects, .. })
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
        | EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::ResolvedWhenResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTaggedWithControllerAtLastBlockedBy {
            effects,
            ..
        })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDoesNot { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDoesNot { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDid { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDid { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::Votes(VoteEffectAst::BidLife {
            winner_effects: effects,
            ..
        })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextCleanupStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUntapStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUpkeep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextDrawStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextMainPhase { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextFirstMainPhase {
            effects, ..
        })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndStepOfExtraTurn {
            effects, ..
        })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat { effects })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectDiesThisTurn {
            effects, ..
        })
        | EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectLeavesBattlefield {
            effects,
            ..
        })
        | EffectAst::Votes(VoteEffectAst::VoteOption { effects, .. })
        | EffectAst::ManaRestricted { effects, .. } => normalize_effects_vec(effects),
        EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
            effects,
            alternative,
            ..
        }) => {
            normalize_effects_vec(effects);
            normalize_effects_vec(alternative);
        }
        // NOTE: this walker stays hand-rolled (rather than routing through
        // effect_ast_traversal's shared helper) because normalize_effects_vec
        // resizes/replaces the Vec (retain + whole-Vec rewrites), which the
        // slice-exposing helper cannot express. New wrapper variants must be
        // added here and kept in sync with the traversal macro.
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { modes, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::VillainousChoice { modes, .. }) => {
            for mode in modes {
                normalize_effects_vec(&mut mode.effects);
            }
        }
        EffectAst::Conditionals(ConditionalEffectAst::IfEffectDidNotHappen {
            effect,
            otherwise,
        }) => {
            normalize_nested_effects(effect);
            normalize_singular_source_exiled_move(effect);
            normalize_effects_vec(otherwise);
        }
        EffectAst::Conditionals(ConditionalEffectAst::IfEffectResult {
            effect, if_true, ..
        }) => {
            normalize_nested_effects(effect);
            normalize_singular_source_exiled_move(effect);
            normalize_effects_vec(if_true);
        }
        EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
            normalize_nested_effects(effect);
            normalize_singular_source_exiled_move(effect);
        }
        EffectAst::Coordination(coordination) => {
            for member in &mut coordination.members {
                normalize_effects_vec(&mut member.effects);
            }
            // "Draw three cards and reveal them": the reveal is a separate
            // coordinated member whose pronoun names the drawn cards.
            for index in 1..coordination.members.len() {
                let (before, after) = coordination.members.split_at_mut(index);
                if let (Some(draw), Some(reveal)) = (
                    before[index - 1].effects.last_mut(),
                    after[0].effects.first_mut(),
                ) {
                    bind_drawn_cards_to_reveal(draw, reveal);
                }
            }
        }
        EffectAst::ControlFlow(control) => {
            for program in &mut control.programs {
                normalize_effects_vec(&mut program.effects);
            }
        }
        _ => {}
    }
}

/// Remove only a terminal marker, retaining every executable preceding member.
fn pop_required_repeat_marker(effects: &mut Vec<EffectAst>) -> bool {
    let Some(last) = effects.last_mut() else { return false; };
    if matches!(last, EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)) {
        effects.pop();
        return true;
    }
    let removed = match last {
        EffectAst::Sequence { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. } => pop_required_repeat_marker(effects),
        EffectAst::Coordination(coordination) => {
            let Some(member) = coordination.members.last_mut() else { return false; };
            if !pop_required_repeat_marker(&mut member.effects) { return false; }
            if member.effects.is_empty() {
                coordination.members.pop();
                coordination.boundaries.pop();
            }
            true
        }
        _ => false,
    };
    if removed {
        let empty = match effects.last() {
            Some(EffectAst::Sequence { effects } | EffectAst::SourceSentence { effects, .. }
                | EffectAst::CommaThen { effects } | EffectAst::Coordinated { effects, .. }) => effects.is_empty(),
            Some(EffectAst::Coordination(coordination)) => coordination.members.is_empty(),
            _ => false,
        };
        if empty { effects.pop(); }
    }
    removed
}

fn rewrite_repeat_process(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    for end in 2..=effects.len() {
        if let Some(mut rewritten) = rewrite_repeat_process_result(&effects[..end]) {
            rewritten.extend_from_slice(&effects[end..]);
            return Some(rewritten);
        }
        if let EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate, if_true, if_false,
        }) = &effects[end - 1]
            && if_false.is_empty()
        {
            let mut branch = if_true.clone();
            if !pop_required_repeat_marker(&mut branch) { continue; }
            let mut body = effects[..end - 1].to_vec();
            body.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: predicate.clone(), if_true: branch, if_false: Vec::new(),
            }));
            let mut rewritten = vec![EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                effects: body,
                continue_effect_index: end - 1,
                continue_predicate: crate::cards::builders::IfResultPredicate::ConditionMatched,
            })];
            rewritten.extend_from_slice(&effects[end..]);
            return Some(rewritten);
        }
    }
    None
}

fn rewrite_repeat_process_result(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 {
        return None;
    }

    let last_index = effects.len() - 1;
    let EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate,
        effects: tail_effects,
    }) = &effects[last_index]
    else {
        return None;
    };
    let marker_is_direct = matches!(
        tail_effects.last(),
        Some(EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess))
    );
    let marker_is_coordinated = matches!(
        tail_effects.last(),
        Some(EffectAst::Coordinated { effects, .. })
            if matches!(effects.last(), Some(EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)))
    );
    // "put a loyalty counter on Grist and repeat this process" arrives as a
    // planned coordination whose last member is the bare repeat marker.
    let marker_is_coordination_member = matches!(
        tail_effects.last(),
        Some(EffectAst::Coordination(coordination))
            if coordination.members.last().is_some_and(|member| {
                matches!(
                    member.effects.as_slice(),
                    [EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)]
                )
            })
    );
    if !marker_is_direct && !marker_is_coordinated && !marker_is_coordination_member {
        return None;
    }
    // In "<action> unless <player> pays ... and repeat this process", paying is
    // the alternative action that both prevents the consequence and repeats
    // the process. Track the complete final IfResult: it is declined only when
    // this branch is selected and the UnlessPays payment succeeds.
    let repeat_follows_unless_payment = matches!(
        tail_effects.last(),
        Some(EffectAst::Coordinated { effects, .. })
            if matches!(
                effects.as_slice(),
                [EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { .. }), EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)]
            )
    );
    let continue_effect_index = if repeat_follows_unless_payment {
        last_index
    } else {
        last_index.saturating_sub(1)
    };
    let continue_predicate = if repeat_follows_unless_payment {
        crate::cards::builders::IfResultPredicate::WasDeclined
    } else {
        predicate.clone()
    };
    let mut body = effects.to_vec();
    let EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. }) =
        &mut body[last_index]
    else {
        return None;
    };
    if marker_is_direct {
        effects.pop();
    } else if let Some(EffectAst::Coordinated { effects, .. }) = effects.last_mut() {
        effects.pop();
    } else if let Some(EffectAst::Coordination(coordination)) = effects.last_mut() {
        coordination.members.pop();
        coordination.boundaries.pop();
        if let [single] = coordination.members.as_mut_slice() {
            // A one-member coordination is just its effects.
            let remaining = std::mem::take(&mut single.effects);
            effects.pop();
            effects.extend(remaining);
        }
    }
    if effects.is_empty() {
        body.pop();
    }

    Some(vec![EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
        effects: body,
        continue_effect_index,
        continue_predicate,
    })])
}

fn rewrite_repeat_process_once(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    for index in 1..effects.len() {
        let EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate, if_true, if_false,
        }) = &effects[index] else { continue; };
        if !if_false.is_empty() || !matches!(if_true.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessOnce)]) { continue; }
        let body = effects[..index].to_vec();
        let mut process = body.clone();
        process.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: predicate.clone(), if_true: body, if_false: Vec::new(),
        }));
        // The condition is evaluated after the first complete execution. A
        // suffix such as "If you searched this way, shuffle" observes the
        // entire process receipt, including a search only in the first pass.
        // CommaThen lowers to one real SequenceEffect. Plain Sequence is a
        // flattening AST container and would attach a later result ID only
        // to the final conditional, dropping an initial-only search receipt.
        let mut rewritten = vec![EffectAst::CommaThen { effects: process }];
        rewritten.extend_from_slice(&effects[index + 1..]);
        return Some(rewritten);
    }
    let marker_index = effects.iter().position(|effect| matches!(effect,
        EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessOnce)
            | EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessAdditional { .. })
    ))?;
    if marker_index == 0 { return None; }
    let count = match &effects[marker_index] {
        EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessOnce) => Value::Fixed(2)
            .with_surface_hint(ValueSurfaceHint::RepeatThisProcessOnce),
        EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessAdditional { count }) =>
            Value::Add(Box::new(Value::Fixed(1)), Box::new(count.clone())),
        _ => return None,
    };
    // The initial execution belongs to the same program as all additional
    // executions. In particular X = 0 still executes the complete body once.
    // Instructions after the marker are a suffix, never part of the loop.
    let mut rewritten = vec![EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
        count,
        effects: effects[..marker_index].to_vec(),
    })];
    rewritten.extend_from_slice(&effects[marker_index + 1..]);
    Some(rewritten)
}

fn rewrite_repeat_process_may(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    let marker_index = effects.iter().position(|effect| match effect {
        EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessMay) => true,
        EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => matches!(
            effects.as_slice(), [EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessMay)]),
        _ => false,
    })?;
    if marker_index == 0 { return None; }
    let mut rewritten = vec![EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
        effects: effects[..=marker_index].to_vec(),
        continue_effect_index: marker_index,
        continue_predicate: crate::cards::builders::IfResultPredicate::Did,
    })];
    rewritten.extend_from_slice(&effects[marker_index + 1..]);
    Some(rewritten)
}

fn rewrite_return_as_aura(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    use crate::cards::builders::{ReturnAsAuraAst, SubjectVerbActionAst, TargetAst};

    let has_return_aura_pair = effects.windows(2).any(|pair| {
        let [
            EffectAst::SubjectVerb(return_subject_verb),
            EffectAst::SubjectVerb(aura_subject_verb),
        ] = pair
        else {
            return false;
        };
        matches!(
            &return_subject_verb.action,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                as_aura: None,
                ..
            })
        ) && matches!(
            &aura_subject_verb.action,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                target: TargetAst::Tagged(tag, _),
                ..
            }) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        )
    });
    if !has_return_aura_pair {
        return None;
    }

    let mut rewritten = Vec::with_capacity(effects.len());
    let mut index = 0;
    let mut changed = false;
    while index < effects.len() {
        let Some(EffectAst::SubjectVerb(return_subject_verb)) = effects.get(index) else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
            as_aura: None,
            ..
        }) = &return_subject_verb.action
        else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let Some(EffectAst::SubjectVerb(aura_subject_verb)) = effects.get(index + 1) else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
            target,
            attachment_filter,
            granted_abilities,
            ..
        }) = &aura_subject_verb.action
        else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        if !matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
        {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        }

        let mut remove_all_abilities = false;
        let mut consumed = 2;
        if let Some(EffectAst::SubjectVerb(remove_subject_verb)) = effects.get(index + 2)
            && is_return_as_aura_remove_all_marker(&remove_subject_verb.action)
        {
            remove_all_abilities = true;
            consumed = 3;
        }

        let mut combined = effects[index].clone();
        if let EffectAst::SubjectVerb(subject_verb) = &mut combined
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                as_aura,
                ..
            }) = &mut subject_verb.action
        {
            *as_aura = Some(ReturnAsAuraAst {
                attachment_filter: attachment_filter.clone(),
                remove_all_abilities,
                granted_abilities: granted_abilities.clone(),
            });
        }
        rewritten.push(combined);
        index += consumed;
        changed = true;
    }

    changed.then_some(rewritten)
}

fn is_return_as_aura_remove_all_marker(action: &SubjectVerbActionAst) -> bool {
    match action {
        SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
            abilities,
            duration,
            ..
        }) => abilities.is_empty() && matches!(duration, crate::effect::Until::Forever),
        SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
            target,
            abilities,
            duration,
        }) => {
            abilities.is_empty()
                && matches!(duration, crate::effect::Until::Forever)
                && matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
        }
        _ => false,
    }
}

fn is_noop_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                crate::cards::builders::SubjectVerbActionAst::Grants(
                    GrantActionAst::GrantAbilitiesAll { abilities, .. },
                )
                | crate::cards::builders::SubjectVerbActionAst::Grants(
                    GrantActionAst::GrantAbilitiesChoiceAll { abilities, .. },
                ),
            ..
        }) => abilities.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::IfResultPredicate;
    use crate::cards::builders::{
        EffectAst, PlayerAst, PredicateAst, SubjectVerbActionAst, TagKey, TargetAst,
    };
    use crate::effect::{ChoiceCount, Until, Value};
    use crate::filter::{
        ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
    };
    use crate::zone::Zone;
    use ironsmith_compiler_semantic::model::ConditionalEffectAst;
    use ironsmith_compiler_semantic::model::DelayedEffectAst;
    use ironsmith_compiler_semantic::model::ForEachEffectAst;
    use ironsmith_compiler_semantic::model::GrantActionAst;
    use ironsmith_compiler_semantic::model::LibraryActionAst;
    use ironsmith_compiler_semantic::model::LifeResourceActionAst;
    use ironsmith_compiler_semantic::model::ObjectChoiceEffectAst;
    use ironsmith_compiler_semantic::model::PermissionEffectAst;
    use ironsmith_compiler_semantic::model::RandomActionAst;
    use ironsmith_compiler_semantic::model::ZoneMoveActionAst;
    use ironsmith_core::ValueSurfaceHint;

    use super::normalize_effects_ast;

    #[test]
    fn normalize_removes_empty_global_grant_effect() {
        let effects = vec![EffectAst::subject_verb_grant_abilities_all(
            ObjectFilter::default(),
            Vec::new(),
            Until::EndOfTurn,
        )];

        let normalized = normalize_effects_ast(&effects);
        assert!(normalized.is_empty());
    }

    #[test]
    fn normalize_removes_empty_global_grant_effect_inside_wrappers() {
        let effects = vec![EffectAst::Permissions(PermissionEffectAst::May {
            effects: vec![
                EffectAst::subject_verb_grant_abilities_all(
                    ObjectFilter::default(),
                    Vec::new(),
                    Until::EndOfTurn,
                ),
                EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::LifeResources(
                        LifeResourceActionAst::Draw {
                            count: Value::Fixed(1),
                        },
                    ),
                ),
            ],
        })];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::Permissions(PermissionEffectAst::May { effects }) = &normalized[0] else {
            panic!("expected wrapped may effect");
        };
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::LifeResources(
                    LifeResourceActionAst::Draw { .. }
                ),
                ..
            })
        ));
    }

    #[test]
    fn normalize_treats_all_players_chosen_subtypes_as_characteristics_not_objects() {
        let choose_type = EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_choose_creature_type(
                PlayerAst::That,
                Vec::new(),
            )],
        });
        let misbound = ObjectFilter::creature().match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );

        let normalized =
            normalize_effects_ast(&[choose_type, EffectAst::subject_verb_destroy_all(misbound)]);
        let filter = super::direct_destroy_filter(&normalized[1]).expect("destroy filter");
        assert!(filter.tagged_constraints.is_empty(), "{filter:#?}");
        assert!(filter.excluded_any_chosen_creature_type, "{filter:#?}");
        assert!(filter.has_chosen_type_this_way_surface(), "{filter:#?}");
    }

    #[test]
    fn normalize_keeps_all_players_chosen_object_destroy_procedures_tagged() {
        let choose_object = EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::ObjectChoices(
                ObjectChoiceEffectAst::ChooseObjects {
                    filter: ObjectFilter::creature(),
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: crate::tag::CompilerReferenceTag::It.bind(),
                },
            )],
        });
        let chosen = ObjectFilter::creature().match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );

        let normalized =
            normalize_effects_ast(&[choose_object, EffectAst::subject_verb_destroy_all(chosen)]);
        let filter = super::direct_destroy_filter(&normalized[1]).expect("destroy filter");
        assert!(!filter.excluded_any_chosen_creature_type, "{filter:#?}");
        assert_eq!(filter.tagged_constraints.len(), 1, "{filter:#?}");
    }

    #[test]
    fn normalize_binds_direct_choice_to_explicit_chosen_set_value() {
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        });
        let chosen_filter = ObjectFilter::creature().match_tagged(
            crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        let difference = Value::absolute_difference(
            Value::GreatestPower(chosen_filter.clone()),
            Value::LeastPower(chosen_filter),
        )
        .with_surface_hint(ValueSurfaceHint::Difference);
        let draw = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: difference }),
        );

        let normalized = normalize_effects_ast(&[choose, draw]);
        let [
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }),
            _,
        ] = normalized.as_slice()
        else {
            panic!("expected choice followed by draw: {normalized:#?}");
        };
        assert_eq!(
            tag.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
    }

    #[test]
    fn normalize_correlates_conditional_quantified_choice_with_chosen_set_destroy() {
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        });
        let mut chosen_permanents = ObjectFilter::permanent();
        chosen_permanents
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::ChosenObjects.bind()).into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        let effects = vec![
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ThisSpellWasCastFromZone(Zone::Exile),
                if_true: vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
                    effects: vec![choose],
                })],
                if_false: Vec::new(),
            }),
            EffectAst::subject_verb_destroy(TargetAst::Object(chosen_permanents, None, None)),
        ];

        let normalized = normalize_effects_ast(&effects);
        let [
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                if_true, if_false, ..
            }),
        ] = normalized.as_slice()
        else {
            panic!("expected one correlated conditional: {normalized:#?}");
        };
        assert!(if_false.is_empty());
        let [
            EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }),
            destroy,
        ] = if_true.as_slice()
        else {
            panic!("expected choice and destroy in the true branch: {if_true:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })] =
            effects.as_slice()
        else {
            panic!("expected one quantified object choice: {effects:#?}");
        };
        assert_eq!(
            tag.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
        assert!(super::direct_destroy_references_chosen_collection(destroy));
    }

    #[test]
    fn normalize_binds_repeated_choices_to_destroy_complement() {
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::creature(),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        });
        let mut complement = ObjectFilter::creature();
        complement.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        let normalized = normalize_effects_ast(&[
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
                count: Value::DistinctPowers(ObjectFilter::creature()),
                effects: vec![choose],
            }),
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. }),
            destroy,
        ] = normalized.as_slice()
        else {
            panic!("expected repeated choice followed by destroy: {normalized:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })] =
            effects.as_slice()
        else {
            panic!("expected repeated object choice: {effects:#?}");
        };
        assert_eq!(
            tag.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_unions_direct_and_per_player_choices_before_destroying_others() {
        let choice = || {
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter::permanent(),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: crate::tag::CompilerReferenceTag::It.bind(),
            })
        };
        let mut complement = ObjectFilter::permanent();
        complement.other = true;
        let normalized = normalize_effects_ast(&[
            choice(),
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                sequential: false,
                filter: PlayerFilter::NotYou,
                effects: vec![choice()],
            }),
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag: direct, .. }),
            quantified,
            destroy,
        ] = normalized.as_slice()
        else {
            panic!("expected two choice producers and a destroy: {normalized:#?}");
        };
        let EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. }) =
            quantified
        else {
            panic!("expected quantified choice: {quantified:#?}");
        };
        let [
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                tag: repeated, ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected quantified object choice: {effects:#?}");
        };
        assert_eq!(
            direct.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
        assert_eq!(
            repeated.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(!filter.other);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_does_not_bind_an_unrelated_destroy_after_a_repeated_choice() {
        let mut unrelated_complement = ObjectFilter::artifact();
        unrelated_complement.other = true;
        let normalized = normalize_effects_ast(&[
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![EffectAst::ObjectChoices(
                    ObjectChoiceEffectAst::ChooseObjects {
                        filter: ObjectFilter::creature(),
                        count: ChoiceCount::exactly(1),
                        count_value: None,
                        player: PlayerAst::You,
                        tag: crate::tag::CompilerReferenceTag::It.bind(),
                    },
                )],
            }),
            EffectAst::subject_verb_destroy_all(unrelated_complement),
        ]);

        let [
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. }),
            destroy,
        ] = normalized.as_slice()
        else {
            panic!("expected unchanged repeated choice: {normalized:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })] =
            effects.as_slice()
        else {
            panic!("expected object choice: {effects:#?}");
        };
        assert_eq!(tag.as_str(), crate::tag::CompilerReferenceTag::It.as_str());
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.other);
        assert!(filter.tagged_constraints.is_empty());
    }

    #[test]
    fn normalize_preserves_custom_choice_collection_tags() {
        let custom_tag = ironsmith_compiler_semantic::tag::declared_key("custom_choice_collection");
        let mut complement = ObjectFilter::creature();
        complement.other = true;
        let normalized = normalize_effects_ast(&[
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![EffectAst::ObjectChoices(
                    ObjectChoiceEffectAst::ChooseObjects {
                        filter: ObjectFilter::creature(),
                        count: ChoiceCount::exactly(1),
                        count_value: None,
                        player: PlayerAst::You,
                        tag: custom_tag.clone(),
                    },
                )],
            }),
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. }),
            destroy,
        ] = normalized.as_slice()
        else {
            panic!("expected unchanged repeated choice: {normalized:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })] =
            effects.as_slice()
        else {
            panic!("expected object choice: {effects:#?}");
        };
        assert_eq!(tag, &custom_tag);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.other);
        assert!(filter.tagged_constraints.is_empty());
    }

    #[test]
    fn normalize_binds_demonstrative_grant_to_draw_counted_set() {
        let counted = ObjectFilter::creature().you_control();
        let draw = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                count: Value::Count(counted.clone()),
            }),
        );
        let mut grant = EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Source(None),
            Vec::new(),
            Until::EndOfTurn,
        );
        let EffectAst::SubjectVerb(grant_subject) = &mut grant else {
            panic!("expected targeted grant");
        };
        let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
            set_quantifier_surface,
            ..
        }) = &mut grant_subject.action
        else {
            panic!("expected targeted grant action");
        };
        *set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);

        let normalized = normalize_effects_ast(&[draw, grant]);
        assert!(matches!(
            &normalized[1],
            EffectAst::SubjectVerb(subject)
                if matches!(
                    &subject.action,
                    SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Object(filter, _, _),
                        ..
                    }) if filter == &counted
                )
        ));
    }

    #[test]
    fn normalize_binds_later_predicate_x_to_typed_where_x_value() {
        let where_x =
            Value::CardsInHand(PlayerFilter::You).with_surface_hint(ValueSurfaceHint::WhereXIs);
        let effects = vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                where_x,
                ironsmith_compiler_semantic::tag::declared_key("looked"),
            ),
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::X,
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
                if_true: Vec::new(),
                if_false: Vec::new(),
            }),
        ];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ValueComparison { left, .. },
            ..
        }) = &normalized[1]
        else {
            panic!("expected typed value comparison");
        };
        assert_eq!(*left, Value::CardsInHand(PlayerFilter::You));
    }

    #[test]
    fn normalize_rewrites_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::Permissions(PermissionEffectAst::May {
                effects: vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::LifeResources(
                        LifeResourceActionAst::Draw {
                            count: Value::Fixed(1),
                        },
                    ),
                )],
            }),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![
                    EffectAst::subject_verb(
                        crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        crate::cards::builders::SubjectVerbActionAst::LifeResources(
                            LifeResourceActionAst::GainLife {
                                amount: Value::Fixed(1),
                            },
                        ),
                    ),
                    EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess),
                ],
            }),
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::Did,
                ..
            })]
        ));
    }

    #[test]
    fn normalize_unless_payment_as_the_repeat_continuation_gate() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::Random(RandomActionAst::FlipCoin),
            ),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                        count: Value::Fixed(1),
                    }),
                )],
            }),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: vec![EffectAst::Coordinated {
                    effects: vec![
                        EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
                            effects: vec![EffectAst::subject_verb(
                                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                                PlayerAst::You,
                                SubjectVerbActionAst::LifeResources(
                                    LifeResourceActionAst::LoseLife {
                                        amount: Value::Fixed(1),
                                    },
                                ),
                            )],
                            player: PlayerAst::You,
                            cost: ironsmith_core::TotalCost::from_cost(
                                crate::model::CompilerCost::Mana(
                                    crate::mana::ManaCost::from_symbols(vec![
                                        crate::mana::ManaSymbol::Generic(3),
                                    ]),
                                ),
                            ),
                            before_delayed_step: false,
                        }),
                        EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess),
                    ],
                    leading_duration: false,
                    result_conjunction: false,
                }],
            }),
        ];

        let normalized = normalize_effects_ast(&effects);
        let [
            EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                effects,
                continue_effect_index,
                continue_predicate,
            }),
        ] = normalized.as_slice()
        else {
            panic!("expected one typed repeat process: {normalized:#?}");
        };
        assert_eq!(*continue_effect_index, 2);
        assert_eq!(*continue_predicate, IfResultPredicate::WasDeclined);
        assert!(matches!(
            effects.as_slice(),
            [
                EffectAst::SubjectVerb(_),
                EffectAst::Conditionals(ConditionalEffectAst::IfResult { .. }),
                EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    effects: loss_effects,
                    ..
                })
            ] if matches!(
                loss_effects.as_slice(),
                [EffectAst::Coordinated {
                    effects,
                    ..
                }] if matches!(effects.as_slice(), [EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { .. })])
            )
        ));
    }

    #[test]
    fn normalize_removes_empty_clash_result_marker_from_repeat_body() {
        let effects = vec![
            EffectAst::subject_verb_clash(crate::cards::builders::ClashOpponentAst::Opponent),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::WonClash,
                effects: vec![EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)],
            }),
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                effects,
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::WonClash,
            })] if effects.len() == 1
        ));
    }

    #[test]
    fn normalize_rewrites_optional_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LifeResources(
                    LifeResourceActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                ),
            ),
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LifeResources(
                    LifeResourceActionAst::LoseLife {
                        amount: Value::Fixed(1),
                    },
                ),
            ),
            EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessMay),
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                continue_effect_index: 2,
                continue_predicate: IfResultPredicate::Did,
                ..
            })]
        ));
        assert_eq!(
            normalize_effects_ast(&normalized),
            normalized,
            "normalizing an existing optional loop must preserve its continuation index"
        );
        for tail in [
            EffectAst::Permissions(PermissionEffectAst::May {
                effects: vec![effects.last().unwrap().clone()],
            }),
            EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player: PlayerAst::You,
                effects: vec![effects.last().unwrap().clone()],
            }),
        ] {
            let mut wrapped = effects.clone();
            *wrapped.last_mut().unwrap() = tail;
            assert_eq!(
                normalize_effects_ast(&wrapped),
                normalized,
                "the generic optional wrapper must not hide the loop or add a second prompt"
            );
        }
    }

    #[test]
    fn normalize_preserves_repeat_this_process_once_as_a_typed_repeat() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LifeResources(
                    LifeResourceActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                ),
            ),
            EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessOnce),
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, effects })]
                if count.unhinted() == &Value::Fixed(2)
                    && count.has_surface_hint(ValueSurfaceHint::RepeatThisProcessOnce)
                    && effects.len() == 1
        ));
    }

    #[test]
    fn finite_additional_repetition_keeps_initial_execution_and_final_suffix() {
        let body = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer, PlayerAst::You,
            crate::cards::builders::SubjectVerbActionAst::LifeResources(
                LifeResourceActionAst::Draw { count: Value::Fixed(1) }));
        let suffix = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer, PlayerAst::You,
            crate::cards::builders::SubjectVerbActionAst::LifeResources(
                LifeResourceActionAst::Draw { count: Value::Fixed(2) }));
        for extra in [Value::Fixed(0), Value::Fixed(6), Value::X] {
            let normalized = normalize_effects_ast(&[
                body.clone(),
                EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessAdditional { count: extra.clone() }),
                suffix.clone(),
            ]);
            assert_eq!(normalized, vec![
                EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
                    count: Value::Add(Box::new(Value::Fixed(1)), Box::new(extra)),
                    effects: vec![body.clone()],
                }), suffix.clone(),
            ]);
            assert_eq!(normalize_effects_ast(&normalized), normalized);
        }
    }

    #[test]
    fn live_condition_repeat_once_observes_initial_body_and_exports_both_search_receipts() {
        let initial = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer, PlayerAst::You,
            crate::cards::builders::SubjectVerbActionAst::LifeResources(
                LifeResourceActionAst::Draw { count: Value::Fixed(1) }));
        let gate = PredicateAst::LifeTotalOrLess(20);
        let suffix = EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::SearchedLibrary, effects: vec![initial.clone()],
        });
        let normalized = normalize_effects_ast(&[
            initial.clone(),
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: gate.clone(),
                if_true: vec![EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessOnce)],
                if_false: Vec::new(),
            }), suffix.clone(),
        ]);
        assert_eq!(normalized, vec![EffectAst::CommaThen { effects: vec![
            initial.clone(), EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: gate, if_true: vec![initial], if_false: Vec::new(),
            }),
        ] }, suffix]);
        assert_eq!(normalize_effects_ast(&normalized), normalized);
    }

    #[test]
    fn normalize_binds_player_choice_remainder_to_the_same_graveyard_domain() {
        let tag = crate::tag::CompilerReferenceTag::It.bind();
        let choice_filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(crate::target::PlayerFilter::IteratedPlayer);
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: choice_filter,
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: PlayerAst::That,
            tag: tag.clone(),
        });
        let exile_rest = EffectAst::subject_verb_exile(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Rest.bind(), None),
            false,
        );

        let normalized = normalize_effects_ast(&[choose, exile_rest]);
        let EffectAst::SubjectVerb(subject_verb) = &normalized[1] else {
            panic!("expected move-to-exile consumer");
        };
        let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
            target: TargetAst::Object(filter, ..),
            ..
        }) = &subject_verb.action
        else {
            panic!("the rest must be an executable complement: {subject_verb:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(
            filter.owner,
            Some(crate::target::PlayerFilter::IteratedPlayer)
        );
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == tag.key.clone()
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_binds_consult_remainder_to_revealed_minus_matched_collection() {
        let revealed = ironsmith_compiler_semantic::tag::declared_key("consult_revealed");
        let matched = ironsmith_compiler_semantic::tag::declared_key("consult_matched");
        let consult = EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::You,
            crate::cards::builders::LibraryConsultModeAst::Reveal,
            ObjectFilter::creature(),
            crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
            revealed.clone(),
            matched.clone(),
        );
        let remainder = EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Rest.bind(), None),
            Zone::Library,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )
        .with_library_order(
            Some(crate::cards::builders::LibraryBottomOrderAst::Random),
            PlayerAst::You,
        );

        let normalized = normalize_effects_ast(&[consult, remainder]);
        assert!(matches!(
            normalized.as_slice(),
            [
                _,
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: Some(keep_tagged),
                        order: crate::cards::builders::LibraryBottomOrderAst::Random,
                        player: PlayerAst::You,
                        ..
                    }),
                    ..
                })
            ] if tag == &revealed && keep_tagged == &matched
        ));
    }

    #[test]
    fn normalize_binds_next_turn_permission_to_exile_inside_delayed_trigger() {
        let exiled_tag = ironsmith_compiler_semantic::tag::declared_key("delayed_exiled_cards");
        let delayed = EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::Dies(ObjectFilter::creature()),
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::You,
                Value::Fixed(1),
                vec![exiled_tag.clone()],
                Vec::new(),
            )],
            one_shot: false,
            duration: Until::EndOfTurn,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
        });
        let grant = EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
            crate::tag::CompilerReferenceTag::It.bind(),
            PlayerAst::You,
            true,
            false,
        );

        let normalized = normalize_effects_ast(&[delayed, grant]);
        assert!(matches!(
            normalized.first(),
            Some(EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration { effects, .. }))
                if matches!(effects.get(1), Some(EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }), ..
                })) if tag == &exiled_tag)
        ));
    }

    #[test]
    fn normalize_keeps_explicit_next_turn_permission_tag() {
        let delayed = EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::Dies(ObjectFilter::creature()),
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::You,
                Value::Fixed(1),
                vec![ironsmith_compiler_semantic::tag::declared_key(
                    "delayed_exiled_cards",
                )],
                Vec::new(),
            )],
            one_shot: false,
            duration: Until::EndOfTurn,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
        });
        let explicit_tag =
            ironsmith_compiler_semantic::tag::declared_key("explicit_permission_pool");
        let grant = EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
            explicit_tag.clone(),
            PlayerAst::You,
            true,
            false,
        );

        let normalized = normalize_effects_ast(&[delayed, grant]);
        assert!(matches!(
            normalized.get(1),
            Some(EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }),
                ..
            })) if tag == &explicit_tag
        ));
    }
}

#[cfg(test)]
mod tap_reflexive_iteration_tests {
    use super::*;
    #[test]
    fn tapped_quantity_followup_is_inside_each_player_iteration_not_after_the_group() {
        let tap = EffectAst::subject_verb_tap(TargetAst::Object(
            crate::filter::ObjectFilter::creature(),
            None,
            None,
        ));
        let follow = EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: vec![EffectAst::subject_verb_damage(
                Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(
                    crate::tag::PRIOR_TAPPED_OBJECT_QUANTITY_TAG.into(),
                ))),
                TargetAst::Player(crate::filter::PlayerFilter::IteratedPlayer, None),
            )],
        });
        let mut effects = vec![
            EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects: vec![tap] }),
            follow,
        ];
        transport_tap_quantity_reflexive_into_player_loop(&mut effects);
        assert_eq!(effects.len(), 1);
        let EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects: body }) = &effects[0]
        else {
            panic!()
        };
        assert_eq!(body.len(), 2);
        assert!(matches!(
            body[1],
            EffectAst::Conditionals(ConditionalEffectAst::WhenResult { .. })
        ));
    }
}

#[cfg(test)]
mod result_gated_same_name_search_tests {
    use super::*;
    use crate::cards::builders::{IfResultPredicate, PlayerAst};
    use crate::effect::{ChoiceCount, SearchResultReferenceSurface, SearchSelectionMode};
    use crate::filter::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    use crate::zone::Zone;

    fn sentence(effect: EffectAst, leading_then: bool) -> EffectAst {
        EffectAst::SourceSentence {
            effects: vec![effect], leading_then, starting_with_controller: false,
        }
    }

    fn program(same_name: bool) -> Vec<EffectAst> {
        let tag = crate::tag::CompilerReferenceTag::It.bind();
        let choice = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::land(), count: ChoiceCount::exactly(1), count_value: None,
            player: PlayerAst::You, tag: tag.clone(),
        });
        let mut filter = ObjectFilter::land();
        if same_name {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: tag.into(), relation: TaggedOpbjectRelation::SameNameAsTagged,
            });
        }
        let search = EffectAst::subject_verb_search_library(
            filter, Zone::Battlefield, PlayerAst::You, PlayerAst::You,
            SearchSelectionMode::Optional, false, None, true, ChoiceCount::up_to(2),
            None, None, SearchResultReferenceSurface::ThoseCards, false, true, false,
        );
        vec![
            sentence(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did, effects: vec![choice],
            }), false),
            sentence(search, true),
        ]
    }

    #[test]
    fn dependent_search_keeps_move_shuffle_and_then_inside_success_branch() {
        let normalized = normalize_effects_ast(&program(true));
        assert_eq!(normalized.len(), 1);
        let EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. }) =
            sentence_tail(&normalized[0]) else { panic!("result branch disappeared"); };
        assert_eq!(effects.len(), 2);
        assert!(matches!(&effects[1], EffectAst::SourceSentence { leading_then: true, .. }));
        assert!(matches!(sentence_tail(&effects[1]),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                    destination: Zone::Battlefield, shuffle: true, tapped: true, ..
                }), ..
            })
        ));
    }

    #[test]
    fn unrelated_following_search_keeps_its_independent_sentence() {
        assert_eq!(normalize_effects_ast(&program(false)).len(), 2);
    }
}
