use super::*;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::ZoneMoveActionAst;
use crate::effect::Value;
use crate::target::PlayerFilter;

pub fn derive_triggered_ability_functional_zones_from_facts(
    trigger: &TriggerSpec,
    facts: &crate::model::facts::TriggerFunctionalZoneFacts,
) -> Vec<Zone> {
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return derive_triggered_ability_functional_zones_from_facts(trigger, facts);
    }
    let mut zones = base_trigger_functional_zones(trigger);

    if let Some(explicit_zone) = &facts.explicit_zone
        && !ironsmith_compiler_semantic::model::trigger_zones::explicit_zone_belongs_to_branch(trigger, *explicit_zone)
    {
        zones = vec![*explicit_zone];
        if facts.explicit_zone_or_battlefield && *explicit_zone != Zone::Battlefield {
            zones.push(Zone::Battlefield);
        }
    }
    // A self-discard trigger observes the card in hand, even when its
    // resolution returns that card from the graveyard.
    if facts.discards_this_card {
        zones = vec![Zone::Hand];
    } else if facts.returns_self_from_graveyard && !trigger_references_attached_object(trigger) {
        zones = vec![Zone::Graveyard];
    }
    zones
}

/// The zones from which the trigger's event can be observed, before
/// whole-ability facts are applied. CR 113.6: an ability functions only
/// from its zones; an "A or B" trigger functions wherever either arm does
/// ("When you cast or cycle this card", CR 603.2 + CR 702.29c).
pub fn base_trigger_functional_zones(trigger: &TriggerSpec) -> Vec<Zone> {
    ironsmith_compiler_semantic::model::trigger_zones::base_trigger_functional_zones(trigger)
}

fn trigger_references_attached_object(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_references_attached_object(trigger),
        TriggerSpec::PutIntoGraveyard(filter) | TriggerSpec::PutIntoGraveyardOneOrMore(filter) => {
            filter_references_tag(filter, "enchanted") || filter_references_tag(filter, "equipped")
        }
        TriggerSpec::PutIntoGraveyardFromZone { filter, .. }
        | TriggerSpec::PutIntoGraveyardFromAnyExcept { filter, .. } => {
            filter_references_tag(filter, "enchanted") || filter_references_tag(filter, "equipped")
        }
        TriggerSpec::Either(left, right) => {
            trigger_references_attached_object(left) || trigger_references_attached_object(right)
        }
        _ => false,
    }
}

fn filter_references_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tag)
        || filter
            .could_be_targeted_by
            .as_ref()
        .is_some_and(|constraint| {
            matches!(&constraint.stack_object, crate::filter::ObjectRef::Tagged(object_tag) if object_tag.as_str() == tag)
        })
        || matches!(&filter.blocked_by, Some(crate::filter::ObjectRef::Tagged(object_tag)) if object_tag.as_str() == tag)
        || matches!(&filter.in_combat_with, Some(crate::filter::ObjectRef::Tagged(object_tag)) if object_tag.as_str() == tag)
        || filter
            .targets_object
            .as_deref()
            .is_some_and(|targets| filter_references_tag(targets, tag))
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(|targets| filter_references_tag(targets, tag))
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(|attached_to| filter_references_tag(attached_to, tag))
        || filter
            .blocked_or_was_blocked_by_this_turn
            .as_deref()
            .is_some_and(|combat_partner| filter_references_tag(combat_partner, tag))
        || filter
            .any_of
            .iter()
            .any(|branch| filter_references_tag(branch, tag))
}

fn replace_filter_tag(filter: &mut ObjectFilter, old_tag: &str, new_tag: &TagKey) -> bool {
    let mut replaced = false;
    for constraint in &mut filter.tagged_constraints {
        if constraint.tag.as_str() == old_tag {
            constraint.tag = new_tag.clone();
            replaced = true;
        }
    }
    if let Some(crate::filter::ObjectRef::Tagged(tag)) = &mut filter.blocked_by
        && tag.as_str() == old_tag
    {
        *tag = new_tag.clone();
        replaced = true;
    }
    if let Some(crate::filter::ObjectRef::Tagged(tag)) = &mut filter.in_combat_with
        && tag.as_str() == old_tag
    {
        *tag = new_tag.clone();
        replaced = true;
    }
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        replaced |= replace_filter_tag(targets, old_tag, new_tag);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        replaced |= replace_filter_tag(targets, old_tag, new_tag);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        replaced |= replace_filter_tag(attached_to, old_tag, new_tag);
    }
    if let Some(combat_partner) = filter.blocked_or_was_blocked_by_this_turn.as_deref_mut() {
        replaced |= replace_filter_tag(combat_partner, old_tag, new_tag);
    }
    for branch in &mut filter.any_of {
        replaced |= replace_filter_tag(branch, old_tag, new_tag);
    }
    replaced
}

pub(super) fn normalize_selected_sacrifice_tags(mut effects: Vec<EffectAst>) -> Vec<EffectAst> {
    let Some((first, rest)) = effects.split_first_mut() else {
        return effects;
    };

    let choose_tag = match first {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            tag, ..
        }) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() => tag,
        _ => return effects,
    };

    let sacrificed_tag = crate::tag::CompilerReferenceTag::Sacrificed0.bind();
    let mut replaced = false;
    for effect in rest {
        match effect {
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { filter, .. })
                        if filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
                ) =>
            {
                if let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
                    filter,
                    ..
                }) = &mut subject_verb.action
                {
                    replaced |= replace_filter_tag(
                        filter,
                        crate::tag::CompilerReferenceTag::It.as_str(),
                        &sacrificed_tag,
                    );
                }
            }
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter })
                        if filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
                ) =>
            {
                if let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }) =
                    &mut subject_verb.action
                {
                    replaced |= replace_filter_tag(
                        filter,
                        crate::tag::CompilerReferenceTag::It.as_str(),
                        &sacrificed_tag,
                    );
                }
            }
            _ => {}
        }
    }

    if replaced {
        *choose_tag = sacrificed_tag;
    }
    effects
}

pub(super) fn runtime_effects_to_costs(
    effects: Vec<crate::effect::Effect>,
) -> Result<Vec<crate::costs::Cost>, CardTextError> {
    effects
        .into_iter()
        .filter(|effect| !crate::costs::is_tagged_type_marker_effect(effect))
        .map(|effect| {
            crate::costs::payment_effect_to_cost(effect).map_err(CardTextError::ParseError)
        })
        .collect()
}

pub(super) fn apply_pending_mechanic_linkages(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    fn contains_haunted_creature_dies(trigger: &crate::triggers::Trigger) -> bool {
        match &trigger.kind {
            crate::triggers::TriggerKind::Custom { id, .. } => id == "haunted_creature_dies",
            crate::triggers::TriggerKind::Either { left, right } => {
                contains_haunted_creature_dies(left) || contains_haunted_creature_dies(right)
            }
            _ => false,
        }
    }

    let linkage = state.haunt_linkage.take().or_else(|| {
        builder.abilities.iter().find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            contains_haunted_creature_dies(&triggered.trigger).then(|| {
                (
                    triggered
                        .effects
                        .segments
                        .iter()
                        .flat_map(|segment| segment.default_effects.iter().cloned())
                        .collect(),
                    triggered.choices.clone(),
                )
            })
        })
    });
    let Some((haunt_effects, haunt_choices)) = linkage else {
        return builder;
    };

    for ability in &mut builder.abilities {
        if is_haunt_placeholder_ability(ability)
            && let crate::ability::AbilityKind::Triggered(ref mut triggered) = ability.kind
        {
            triggered.effects = crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::haunt_exile(haunt_effects, haunt_choices),
            ]);
            break;
        }
    }

    builder
}

fn apply_pending_backup_abilities(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    if state.pending_backups.is_empty() {
        return builder;
    }

    let original_abilities = std::mem::take(&mut builder.abilities);
    let pending_backups = std::mem::take(&mut state.pending_backups);
    let mut abilities = Vec::with_capacity(original_abilities.len() + pending_backups.len());
    let mut next_backup = 0;

    for boundary in 0..=original_abilities.len() {
        while pending_backups
            .get(next_backup)
            .is_some_and(|backup| backup.ability_boundary == boundary)
        {
            let backup = pending_backups[next_backup];
            let granted_abilities = original_abilities[boundary..].to_vec();
            abilities.push(Ability::triggered(
                crate::triggers::Trigger::this_enters_battlefield(),
                vec![crate::effect::Effect::backup(
                    backup.amount,
                    granted_abilities,
                )],
            ));
            next_backup += 1;
        }

        if let Some(ability) = original_abilities.get(boundary) {
            abilities.push(ability.clone());
        }
    }

    debug_assert_eq!(
        next_backup,
        pending_backups.len(),
        "Backup boundaries must follow lowered ability order"
    );
    builder.abilities = abilities;
    builder
}

fn apply_pending_cipher_effect(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    if !std::mem::take(&mut state.pending_cipher) {
        return builder;
    }

    let program = builder
        .spell_effect
        .get_or_insert_with(ResolutionProgram::default);
    let cipher = crate::effect::Effect::cipher();
    if program.segments.is_empty() {
        program.push(cipher);
    } else {
        program.push_segment(crate::resolution::ResolutionSegment {
            default_effects: vec![cipher],
            self_replacements: Vec::new(),
            starts_new_source_line: true,
        });
    }
    builder
}

fn is_haunt_placeholder_ability(ability: &Ability) -> bool {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return false;
    };
    triggered.effects.segments.iter().any(|segment| {
        segment.default_effects.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::ExileEffect>()
                .is_some_and(|exile| exile.spec == ChooseSpec::Source)
        })
    })
}

fn looked_collection_prelude(
    default_effects: &[crate::effect::Effect],
) -> Option<(TagKey, Vec<crate::effect::Effect>)> {
    let look = default_effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let mut prelude = vec![default_effects[0].clone()];
    if let Some(reveal) = default_effects
        .get(1)
        .and_then(|effect| effect.downcast_ref::<crate::effects::RevealTaggedEffect>())
        && reveal.tag == look.tag
    {
        prelude.push(default_effects[1].clone());
    }
    Some((look.tag.clone(), prelude))
}

fn looked_partition_source_tag(replacement_effects: &[crate::effect::Effect]) -> Option<TagKey> {
    let mut source = None;
    for choose in replacement_effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
    {
        if choose.zone != Some(Zone::Library) || choose.filter.zone != Some(Zone::Library) {
            continue;
        }
        let mut membership = choose
            .filter
            .tagged_constraints
            .iter()
            .filter(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
            .map(|constraint| constraint.tag.clone());
        let Some(tag) = membership.next() else {
            continue;
        };
        if membership.next().is_some() {
            return None;
        }
        if source.as_ref().is_some_and(|existing| existing != &tag) {
            return None;
        }
        source = Some(tag);
    }
    source
}

fn has_looked_partition_remainder(
    replacement_effects: &[crate::effect::Effect],
    source_tag: &TagKey,
) -> bool {
    replacement_effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
            .is_some_and(|remainder| &remainder.tag == source_tag)
            || effect
                .downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()
                .is_some_and(|for_each| &for_each.tag == source_tag)
    })
}

fn bind_looked_partition_source_tag(
    effect: crate::effect::Effect,
    old_tag: &TagKey,
    new_tag: &TagKey,
) -> crate::effect::Effect {
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        let mut choose = choose.clone();
        replace_filter_tag(&mut choose.filter, old_tag.as_str(), new_tag);
        return crate::effect::Effect::new(choose);
    }
    if let Some(remainder) =
        effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
    {
        let mut remainder = remainder.clone();
        if remainder.tag == *old_tag {
            remainder.tag = new_tag.clone();
        }
        return crate::effect::Effect::new(remainder);
    }
    if let Some(for_each) =
        effect.downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()
    {
        let mut for_each = for_each.clone();
        if for_each.tag == *old_tag {
            for_each.tag = new_tag.clone();
        }
        return crate::effect::Effect::new(for_each);
    }
    effect
}

/// A self-replacement substitutes the whole resolution segment. When both
/// branches partition one looked/revealed collection, the replacement branch
/// therefore needs the same collection-producing prelude as the default
/// branch. Reconcile independently generated helper tags at the same time.
fn preserve_looked_collection_self_replacement_preludes(program: &mut ResolutionProgram) {
    for segment in &mut program.segments {
        let Some((default_tag, prelude)) = looked_collection_prelude(&segment.default_effects)
        else {
            continue;
        };
        for branch in &mut segment.self_replacements {
            if branch.replacement_effects.iter().any(|effect| {
                effect
                    .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
                    .is_some()
            }) {
                continue;
            }
            let Some(source_tag) = looked_partition_source_tag(&branch.replacement_effects) else {
                continue;
            };
            if !has_looked_partition_remainder(&branch.replacement_effects, &source_tag) {
                continue;
            }

            let mut replacement_effects = std::mem::take(&mut branch.replacement_effects)
                .into_iter()
                .map(|effect| bind_looked_partition_source_tag(effect, &source_tag, &default_tag))
                .collect::<Vec<_>>();
            let mut with_prelude = prelude.clone();
            with_prelude.append(&mut replacement_effects);
            branch.replacement_effects = with_prelude;
        }
    }
}

/// Rebind the two ordered "revealed this way" partitions to the exact
/// LookAtTopCards collection after reference lowering. The union capture is
/// lowered first and advances ordinary last-object memory; only this exact
/// typed six-member partition program may use its already-proved union arms
/// to repair the two later standalone captures.
fn normalize_ordered_revealed_partition_tags(program: &mut ResolutionProgram) {
    fn collect_revealed_look_tags(effect: &crate::effect::Effect, tags: &mut Vec<TagKey>) {
        if let Some(look) = effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && look.reveal
        {
            tags.push(look.tag.clone());
        }
        effect.visit_child_effects(&mut |child| collect_revealed_look_tags(child, tags));
    }

    fn membership_tag(filter: &ObjectFilter) -> Option<&TagKey> {
        let [constraint] = filter.tagged_constraints.as_slice() else {
            return None;
        };
        (constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
            .then_some(&constraint.tag)
    }

    fn same_filter_except_membership(left: &ObjectFilter, right: &ObjectFilter) -> bool {
        if membership_tag(left).is_none() || membership_tag(right).is_none() {
            return false;
        }
        let mut left = left.clone();
        let mut right = right.clone();
        left.tagged_constraints.clear();
        right.tagged_constraints.clear();
        left == right
    }

    fn substitute_membership_tag(filter: &mut ObjectFilter, reveal_tag: &TagKey) -> bool {
        let [constraint] = filter.tagged_constraints.as_mut_slice() else {
            return false;
        };
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return false;
        }
        constraint.tag = reveal_tag.clone();
        true
    }

    fn partition_sequence_with_reveal_tag(
        effect: &crate::effect::Effect,
        reveal_tag: &TagKey,
    ) -> Option<crate::effect::Effect> {
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut rewritten = with_id.clone();
            rewritten.effect = Box::new(partition_sequence_with_reveal_tag(
                &with_id.effect,
                reveal_tag,
            )?);
            return Some(crate::effect::Effect::new(rewritten));
        }
        let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        if sequence.surface != ironsmith_core::SequenceSurface::CommaThen
            || sequence.result_label.is_some()
        {
            return None;
        }
        let [
            first_candidate,
            second_candidate,
            first_loop,
            second_effect,
            second_loop,
            remainder_effect,
        ] = sequence.effects.as_slice()
        else {
            return None;
        };
        let first_candidate =
            first_candidate.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        let second_candidate =
            second_candidate.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        let (union_index, first_index, union, first) = if first_candidate.filter.any_of.len() == 2 {
            (0, 1, first_candidate, second_candidate)
        } else if second_candidate.filter.any_of.len() == 2 {
            (1, 0, second_candidate, first_candidate)
        } else {
            return None;
        };
        let second = second_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        let first_loop = first_loop
            .downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()?;
        let second_loop = second_loop
            .downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()?;
        let remainder = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
        let [first_union, second_union] = union.filter.any_of.as_slice() else {
            return None;
        };
        let union_membership = membership_tag(first_union)?;
        if membership_tag(second_union) != Some(union_membership) {
            return None;
        }
        // Reference lowering visits these captures in order. The union capture
        // advances ordinary object memory to `union.tag`; the first subgroup
        // then advances it again to `first.tag`. An authored explicit reveal
        // collection can therefore arrive here as the exact causal chain
        // reveal -> union -> first subgroup. Accept only those intermediate
        // producer tags, while the union arms themselves still prove the
        // authoritative revealed collection.
        let first_membership_is_proven = membership_tag(&first.filter)
            .is_some_and(|tag| tag == reveal_tag || tag == &union.tag || tag == union_membership);
        let second_membership_is_proven = membership_tag(&second.filter).is_some_and(|tag| {
            tag == reveal_tag || tag == &union.tag || tag == &first.tag || tag == union_membership
        });
        if union.zone != Some(Zone::Library)
            || !union.additional_zones.is_empty()
            || !union.source_tags.is_empty()
            || first.zone != Some(Zone::Library)
            || second.zone != Some(Zone::Library)
            || first_loop.tag != first.tag
            || second_loop.tag != second.tag
            || (remainder.tag != *reveal_tag && remainder.tag != union.tag)
            || remainder.keep_tagged.as_ref() != Some(&union.tag)
            || !first_membership_is_proven
            || !second_membership_is_proven
            || !same_filter_except_membership(&first.filter, first_union)
            || !same_filter_except_membership(&second.filter, second_union)
        {
            return None;
        }

        let mut rewritten = sequence.clone();
        let mut union = union.clone();
        if !substitute_membership_tag(&mut union.filter.any_of[0], reveal_tag)
            || !substitute_membership_tag(&mut union.filter.any_of[1], reveal_tag)
        {
            return None;
        }
        let mut first = first.clone();
        first.filter = union.filter.any_of[0].clone();
        let mut second = second.clone();
        second.filter = union.filter.any_of[1].clone();
        let mut remainder = remainder.clone();
        remainder.tag = reveal_tag.clone();
        rewritten.effects[union_index] = crate::effect::Effect::new(union);
        rewritten.effects[first_index] = crate::effect::Effect::new(first);
        rewritten.effects[3] = crate::effect::Effect::new(second);
        rewritten.effects[5] = crate::effect::Effect::new(remainder);
        Some(crate::effect::Effect::new(rewritten))
    }

    let [producer, disposition] = program.segments.as_mut_slice() else {
        return;
    };
    if !producer.self_replacements.is_empty() || !disposition.self_replacements.is_empty() {
        return;
    }
    let mut reveal_tags = Vec::new();
    for effect in &producer.default_effects {
        collect_revealed_look_tags(effect, &mut reveal_tags);
    }
    let [reveal_tag] = reveal_tags.as_slice() else {
        return;
    };
    let [root] = disposition.default_effects.as_slice() else {
        return;
    };
    let Some(rewritten) = partition_sequence_with_reveal_tag(root, reveal_tag) else {
        return;
    };
    disposition.default_effects = vec![rewritten];
}

/// Correlate a singular selection from a freshly exiled pool with the
/// immediately following permission for "that card". Every guard is over
/// materialized typed effects; no source recognition occurs here.
fn bind_selected_exile_card_play_permission(program: &mut crate::resolution::ResolutionProgram) {
    let mut pool_tag = None;
    let mut selected_tag = None;
    let mut grant_location = None;

    for (segment_index, segment) in program.segments.iter().enumerate() {
        for (effect_index, effect) in segment.default_effects.iter().enumerate() {
            if let Some(exile) = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
                && exile.player == PlayerFilter::You
                && exile.moved_tags.len() == 1
                && exile.accumulated_tags.is_empty()
            {
                if pool_tag.is_some() {
                    return;
                }
                pool_tag = exile.moved_tags.first().cloned();
                continue;
            }
            if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
                && choose.count == crate::effect::ChoiceCount::exactly(1)
                && choose.chooser == PlayerFilter::You
                && choose.zone == Some(Zone::Exile)
                && choose.additional_zones.is_empty()
                && !choose.is_search
            {
                let Some(pool) = pool_tag.as_ref() else {
                    continue;
                };
                let has_exact_pool_constraint = choose.filter.tagged_constraints.len() == 1
                    && choose.filter.tagged_constraints[0]
                        == crate::filter::TaggedObjectConstraint {
                            tag: pool.clone(),
                            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                        };
                if has_exact_pool_constraint {
                    selected_tag = Some(choose.tag.clone());
                }
                continue;
            }
            if let Some(grant) = effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
                && pool_tag.as_ref() == Some(&grant.tag)
                && grant.player == PlayerFilter::You
                && grant.duration == crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd
                && grant.allow_land
                && grant.filter.is_none()
                && grant.max_plays.is_none()
                && grant.cast_pool_is_plural
            {
                if grant_location.is_some() {
                    return;
                }
                grant_location = Some((segment_index, effect_index));
            }
        }
    }

    let (Some(selected_tag), Some((segment_index, effect_index))) = (selected_tag, grant_location)
    else {
        return;
    };
    let Some(mut grant) = program.segments[segment_index].default_effects[effect_index]
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        .cloned()
    else {
        return;
    };
    grant.tag = selected_tag;
    grant.cast_pool_is_plural = false;
    grant.surface = Some(
        ironsmith_core::GrantPlayTaggedSurface::default()
            .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard),
    );
    program.segments[segment_index].default_effects[effect_index] =
        crate::effect::Effect::new(grant);
}

/// An entry-time rider after exiling a graveyard source refers to the
/// permanent spell that triggered the ability, not to the exiled source.
/// Require the parser's entry-counter markers so ordinary counter placement
/// retains its original target and timing.
fn bind_graveyard_cast_trigger_to_triggering_permanent_entry(ability: &mut Ability) {
    if ability.functional_zones.as_slice() != [Zone::Graveyard] {
        return;
    }
    let AbilityKind::Triggered(triggered) = &mut ability.kind else {
        return;
    };
    let filter = match &triggered.trigger.kind {
        ironsmith_core::TriggerKind::SpellCast { filter, .. }
        | ironsmith_core::TriggerKind::SpellCastQualified { filter, .. } => filter.as_ref(),
        _ => return,
    };
    let Some(filter) = filter else {
        return;
    };
    if filter.card_types.is_empty()
        || filter.card_types.iter().any(|kind| {
            matches!(
                kind,
                crate::types::CardType::Instant | crate::types::CardType::Sorcery
            )
        })
    {
        return;
    }
    if triggered
        .effects
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return;
    }
    let effects = triggered.effects.flattened_default_effects();
    let [move_effect, counter_effect] = effects else {
        return;
    };
    let exiles_source = move_effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .is_some_and(|effect| {
            effect.spec == ChooseSpec::Source && !effect.face_down && !effect.turn_face_up
        })
        || move_effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|effect| {
                effect.target == ChooseSpec::Source && effect.zone == Zone::Exile
            });
    let Some(counter) = counter_effect.downcast_ref::<crate::effects::PutCountersEffect>() else {
        return;
    };
    if !exiles_source
        || counter.distributed
        || counter.target_count.is_some()
        || !counter
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
        || !counter
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
        || counter.target.base()
            != &ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::SourceExiled.key())
    {
        return;
    }
    let tag = crate::tag::CompilerReferenceTag::TriggeringPermanentSpell.bind();
    let mut entry_filter = filter.clone();
    entry_filter.zone = Some(Zone::Battlefield);
    entry_filter.stack_kind = None;
    entry_filter.has_mana_cost = false;
    let register = crate::effects::RegisterNextBatchEnterWithCountersEffect::new(
        entry_filter,
        counter.counter_type,
        counter.amount.clone(),
    )
    .same_stable_id_as_tag(tag.clone());
    // Keep sentence boundaries and the unconditional rider even if the source
    // has already left its graveyard when this ability resolves.
    let last = triggered
        .effects
        .segments
        .last_mut()
        .expect("two effects have a segment");
    *last
        .default_effects
        .last_mut()
        .expect("counter rider exists") = crate::effect::Effect::new(register);
    triggered.effects.segments[0]
        .default_effects
        .insert(0, crate::effect::Effect::tag_triggering_object(tag));
    triggered.effects =
        crate::resolution::ResolutionProgram::new(std::mem::take(&mut triggered.effects.segments));
}

pub(super) fn finalize_lowered_card(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    builder = apply_pending_mechanic_linkages(builder, state);
    builder = apply_pending_backup_abilities(builder, state);
    builder = apply_pending_cipher_effect(builder, state);
    if let Some(spell_effect) = &mut builder.spell_effect {
        normalize_ordered_revealed_partition_tags(spell_effect);
        preserve_looked_collection_self_replacement_preludes(spell_effect);
        bind_quantified_player_damage_values(spell_effect);
        correlate_clash_win_return_replacement(spell_effect);
        correlate_additional_cost_damage_replacement(spell_effect);
        correlate_additional_cost_chosen_type_search_destination(spell_effect);
        strip_terminal_unconsumed_damage_aggregate_id(spell_effect);
        merge_whole_spell_instead_replacement(spell_effect);
        bind_unattach_to_attached_object(spell_effect);
        hoist_optional_target_declaration(spell_effect);
        distribute_unless_over_tagged_controllers(spell_effect);
        finalize_program_references(spell_effect);
    }
    for ability in &mut builder.abilities {
        bind_graveyard_cast_trigger_to_triggering_permanent_entry(ability);
        match &mut ability.kind {
            AbilityKind::Triggered(triggered) => {
                preserve_looked_collection_self_replacement_preludes(&mut triggered.effects);
                bind_quantified_player_damage_values(&mut triggered.effects);
                bind_selected_exile_card_play_permission(&mut triggered.effects);
                bind_unattach_to_attached_object(&mut triggered.effects);
                hoist_optional_target_declaration(&mut triggered.effects);
                finalize_program_references(&mut triggered.effects);
            }
            AbilityKind::Activated(activated) => {
                preserve_looked_collection_self_replacement_preludes(&mut activated.effects);
                bind_quantified_player_damage_values(&mut activated.effects);
                bind_selected_exile_card_play_permission(&mut activated.effects);
                bind_unattach_to_attached_object(&mut activated.effects);
                hoist_optional_target_declaration(&mut activated.effects);
                finalize_program_references(&mut activated.effects);
            }
            _ => {}
        }
    }
    link_alternative_cast_condition_references(&mut builder);
    builder
}

const REST_TAG: &str = "rest";

fn debug_mentions_rest_tag(effect: &crate::effect::Effect) -> (bool, bool) {
    let text = format!("{effect:?}");
    let needle = "TagKey(\"rest\")";
    let mut produces = false;
    let mut consumes = false;
    let mut offset = 0;
    while let Some(found) = text[offset..].find(needle) {
        let start = offset + found;
        if text[..start].ends_with("tag: ")
            && !text[..start].ends_with("TaggedObjectConstraint { tag: ")
        {
            produces = true;
        } else {
            consumes = true;
        }
        offset = start + needle.len();
    }
    (produces, consumes)
}

/// The set "the rest" is taken from, found by walking back from its
/// consumer: an inspected library/exile collection (the rest are its cards
/// still in that zone), or an ordinary choice among matching objects (the
/// rest are the matching objects not chosen).
fn rest_complement_producer(earlier: &[crate::effect::Effect]) -> Option<crate::effect::Effect> {
    use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
    let tagged_collection = |tag: &TagKey, zone: Zone| {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(zone);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        crate::effect::Effect::new(
            crate::effects::TagMatchingObjectsEffect::new(filter, REST_TAG).in_zone(zone),
        )
    };
    let zone_of_collection = |tag: &TagKey| -> Option<Zone> {
        earlier.iter().rev().find_map(|effect| {
            let text = format!("{effect:?}");
            let named = text.contains(&format!("TagKey({:?})", tag.as_str()));
            if !named {
                return None;
            }
            // An instruction that picks some members out of the collection
            // ("Exile one of those cards") moves only the pick; the rest of
            // the collection stays where it was put together.
            let picks_from_collection = |spec: &ChooseSpec| {
                matches!(
                    spec,
                    ChooseSpec::WithCount(picked, _) | ChooseSpec::WithCountValue(picked, _, _)
                        if matches!(picked.base(), ChooseSpec::Tagged(picked) if picked == tag)
                )
            };
            let mut unwrapped = effect;
            while let Some(tagged) = unwrapped.downcast_ref::<crate::effects::TaggedEffect>() {
                unwrapped = &tagged.effect;
            }
            if unwrapped.target_spec().is_some_and(|spec| {
                picks_from_collection(spec.base()) || picks_from_collection(spec)
            }) {
                return None;
            }
            if let Some(look) = effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
                && look.tag == *tag
            {
                return Some(Zone::Library);
            }
            if text.contains("ExileTopOfLibraryEffect") || text.contains("ExileEffect") {
                return Some(Zone::Exile);
            }
            None
        })
    };
    // The collection may have been built inside an earlier branch body
    // ("If you do, exile the top four cards ... Put the rest into your
    // hand"); walk every earlier instruction in execution order.
    fn flatten(effect: &crate::effect::Effect, out: &mut Vec<crate::effect::Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| flatten(child, out));
    }
    let mut ordered = Vec::new();
    for effect in earlier {
        flatten(effect, &mut ordered);
    }
    for inner in ordered.iter().rev() {
        if let Some(exile_top) = inner.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
            && let Some(tag) = exile_top.moved_tags.first()
        {
            return Some(tagged_collection(tag, Zone::Exile));
        }
        if let Some(look) = inner.downcast_ref::<crate::effects::LookAtTopCardsEffect>() {
            return Some(tagged_collection(&look.tag, Zone::Library));
        }
        // "Return two cards exiled with this Saga ... and put the rest ...":
        // a counted pick straight out of a tagged set.
        if let Some(ChooseSpec::WithCount(picked, _) | ChooseSpec::WithCountValue(picked, _, _)) =
            inner.target_spec()
            && let ChooseSpec::Tagged(tag) = picked.base()
        {
            let zone = if tag.as_str() == ironsmith_core::SOURCE_EXILED_TAG {
                Some(Zone::Exile)
            } else {
                zone_of_collection(tag)
            };
            if let Some(zone) = zone {
                return Some(tagged_collection(tag, zone));
            }
        }
        if let Some(choose) = inner.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            if let Some(member) = choose
                .filter
                .tagged_constraints
                .iter()
                .find(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
            {
                let zone = choose
                    .filter
                    .zone
                    .or_else(|| zone_of_collection(&member.tag))
                    .or(choose.zone)?;
                return Some(tagged_collection(&member.tag, zone));
            }
            let mut filter = choose.filter.clone();
            if filter.zone.is_none() {
                filter.zone = choose.zone;
            }
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: choose.tag.clone(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
            let zone = filter.zone.unwrap_or(Zone::Battlefield);
            return Some(crate::effect::Effect::new(
                crate::effects::TagMatchingObjectsEffect::new(filter, REST_TAG).in_zone(zone),
            ));
        }
    }
    None
}

/// "Sacrifice the rest" names every remaining object, not one of them.
fn rest_consumer_for_each(effect: &crate::effect::Effect) -> Option<crate::effect::Effect> {
    let sacrifice = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    let ChooseSpec::Object(filter) = sacrifice.target.base() else {
        return None;
    };
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == REST_TAG)
    {
        return None;
    }
    let mut per_object = sacrifice.clone();
    per_object.target = ChooseSpec::Iterated;
    Some(crate::effect::Effect::for_each_tagged(
        REST_TAG,
        vec![crate::effect::Effect::new(per_object)],
    ))
}

/// Returns `None` when nothing was bound, `Some(None)` when the producer
/// went into a nested body, and `Some(Some(index))` when it was inserted
/// into this list at `index`. `outer_earlier` holds the instructions that
/// run before this list starts.
fn bind_rest_complement_in_list(
    effects: &mut Vec<crate::effect::Effect>,
    outer_earlier: &[crate::effect::Effect],
) -> Option<Option<usize>> {
    let consumer_index = effects
        .iter()
        .position(|effect| debug_mentions_rest_tag(effect).1)?;
    // A modal spell's modes are separate instruction lists.
    if let Some(modal) = effects[consumer_index].downcast_ref::<crate::effects::ChooseModeEffect>()
    {
        let mut modal = modal.clone();
        let mut earlier = outer_earlier.to_vec();
        earlier.extend(effects[..consumer_index].iter().cloned());
        let mut bound = false;
        for mode in &mut modal.modes {
            if !mode
                .effects
                .iter()
                .any(|effect| debug_mentions_rest_tag(effect).1)
                || mode
                    .effects
                    .iter()
                    .any(|effect| debug_mentions_rest_tag(effect).0)
            {
                continue;
            }
            bound |= bind_rest_complement_in_list(&mut mode.effects, &earlier).is_some();
        }
        if !bound {
            return None;
        }
        effects[consumer_index] = crate::effect::Effect::new(modal);
        return Some(None);
    }
    if let Some(sequence) = effects[consumer_index].downcast_ref::<crate::effects::SequenceEffect>()
    {
        let mut sequence = sequence.clone();
        let mut earlier = outer_earlier.to_vec();
        earlier.extend(effects[..consumer_index].iter().cloned());
        if bind_rest_complement_in_list(&mut sequence.effects, &earlier).is_some() {
            effects[consumer_index] = crate::effect::Effect::new(sequence);
            return Some(None);
        }
        return None;
    }
    let mut earlier = outer_earlier.to_vec();
    earlier.extend(effects[..consumer_index].iter().cloned());
    let mut producer = rest_complement_producer(&earlier)?;
    exclude_kept_pick_from_rest(&mut effects[..consumer_index], &mut producer);
    if let Some(for_each) = rest_consumer_for_each(&effects[consumer_index]) {
        effects[consumer_index] = for_each;
    }
    effects.insert(consumer_index, producer);
    Some(Some(consumer_index))
}

const REST_KEPT_TAG: &str = "__rest_kept";

/// "Put one of those cards back on top of your library. Exile the rest.": the
/// picked card stays in the collection's zone, so "the rest" must also leave
/// out what the pick moved. Tag that pick and exclude it.
fn exclude_kept_pick_from_rest(
    earlier: &mut [crate::effect::Effect],
    producer: &mut crate::effect::Effect,
) {
    use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
    let Some(tag_matching) = producer.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
    else {
        return;
    };
    let Some(zone) = tag_matching.zone else {
        return;
    };
    let Some(collection) = tag_matching
        .filter
        .tagged_constraints
        .iter()
        .find(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
        .map(|constraint| constraint.tag.clone())
    else {
        return;
    };
    let is_kept_pick = |effect: &crate::effect::Effect| {
        effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|mv| {
                mv.zone == zone
                    && matches!(
                        mv.target.base(),
                        ChooseSpec::Tagged(tag) if *tag == collection
                    )
            })
    };
    let tag_pick = |effect: &crate::effect::Effect| -> Option<crate::effect::Effect> {
        if is_kept_pick(effect) {
            return Some(effect.clone().tag(REST_KEPT_TAG));
        }
        let may = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()?;
        let index = may.effects.iter().position(|inner| is_kept_pick(inner))?;
        let mut may = may.clone();
        may.effects[index] = may.effects[index].clone().tag(REST_KEPT_TAG);
        Some(crate::effect::Effect::new(may))
    };
    let Some((index, tagged)) = earlier
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, effect)| tag_pick(effect).map(|tagged| (index, tagged)))
    else {
        return;
    };
    earlier[index] = tagged;
    let mut tag_matching = tag_matching.clone();
    tag_matching
        .filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: REST_KEPT_TAG.into(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    *producer = crate::effect::Effect::new(tag_matching);
}

/// Run a whole-program lowering pass over the programs of delayed triggers
/// an instruction list schedules ("at the beginning of the next end step,
/// ... the rest"), which are separate resolutions.
fn for_each_scheduled_delayed_program(
    effects: &mut [crate::effect::Effect],
    pass: &mut dyn FnMut(&mut crate::resolution::ResolutionProgram),
) {
    for effect in effects.iter_mut() {
        if let Some(schedule) =
            effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        {
            let mut schedule = schedule.clone();
            let mut program = crate::resolution::ResolutionProgram::from_effects(std::mem::take(
                &mut schedule.effects,
            ));
            pass(&mut program);
            schedule.effects = program
                .segments
                .into_iter()
                .flat_map(|segment| segment.default_effects)
                .collect();
            *effect = crate::effect::Effect::new(schedule);
        } else if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            for_each_scheduled_delayed_program(&mut sequence.effects, pass);
            *effect = crate::effect::Effect::new(sequence);
        }
    }
}

/// "Return target card ... Put up to one other target card ... Exile this.
/// Adamant — If ..., instead return those cards to your hand and exile this":
/// a replacement that restates the spell's final instruction replaces the
/// whole instruction list, not just that last instruction, and "those cards"
/// are the spell's object targets (CR 614.1a, 608.2c).
fn merge_whole_spell_instead_replacement(program: &mut crate::resolution::ResolutionProgram) {
    let segment_count = program.segments.len();
    if segment_count < 2 {
        return;
    }
    let (earlier, last) = program.segments.split_at_mut(segment_count - 1);
    let last = &mut last[0];
    if last.self_replacements.len() != 1
        || earlier
            .iter()
            .any(|segment| !segment.self_replacements.is_empty())
    {
        return;
    }
    let Some(final_default) = last.default_effects.last() else {
        return;
    };
    let final_debug = format!("{final_default:?}");
    fn flatten(effect: &crate::effect::Effect, out: &mut Vec<crate::effect::Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| flatten(child, out));
    }
    let mut replacement_nodes = Vec::new();
    for effect in &last.self_replacements[0].replacement_effects {
        flatten(effect, &mut replacement_nodes);
    }
    if !replacement_nodes
        .iter()
        .any(|effect| format!("{effect:?}") == final_debug)
    {
        return;
    }
    // The spell's own object targets, in order.
    let mut target_specs: Vec<ChooseSpec> = Vec::new();
    for effect in earlier
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
    {
        let inner = effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .map(|tagged| tagged.effect.as_ref())
            .unwrap_or(effect);
        let spec = if let Some(ret) =
            inner.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
        {
            Some(ret.target.clone())
        } else {
            inner
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .map(|mv| mv.target.clone())
        };
        if let Some(spec) = spec
            && spec.is_target()
        {
            target_specs.push(spec);
        }
    }
    if target_specs.is_empty() {
        return;
    }
    fn names_those_targets(spec: &ChooseSpec) -> bool {
        matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == ironsmith_core::SOURCE_EXILED_TAG)
            && matches!(
                spec.source_reference_surface(),
                Some(crate::target::SourceReferenceSurface::ThisPermanentType(text))
                    if text == "those cards"
            )
    }
    fn rebind(
        effect: &crate::effect::Effect,
        targets: &[ChooseSpec],
    ) -> Vec<crate::effect::Effect> {
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = sequence
                .effects
                .iter()
                .flat_map(|child| rebind(child, targets))
                .collect();
            return vec![crate::effect::Effect::new(sequence)];
        }
        let inner = effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .map(|tagged| tagged.effect.as_ref())
            .unwrap_or(effect);
        if let Some(ret) = inner.downcast_ref::<crate::effects::ReturnToHandEffect>()
            && names_those_targets(&ret.spec)
        {
            return targets
                .iter()
                .map(|target| {
                    crate::effect::Effect::new(
                        crate::effects::ReturnFromGraveyardToHandEffect::new(target.clone(), false),
                    )
                })
                .collect();
        }
        vec![effect.clone()]
    }
    let branch = &mut last.self_replacements[0];
    branch.replacement_effects = branch
        .replacement_effects
        .iter()
        .flat_map(|effect| rebind(effect, &target_specs))
        .collect();
    let mut merged: Vec<crate::effect::Effect> = earlier
        .iter_mut()
        .flat_map(|segment| std::mem::take(&mut segment.default_effects))
        .collect();
    merged.append(&mut last.default_effects);
    last.default_effects = merged;
    let last = program.segments.pop().expect("last segment");
    program.segments = vec![last];
}

/// Rebuild an effect tree bottom-up through the common instruction
/// containers, letting `f` replace any node.
fn map_effect_tree(
    effect: &crate::effect::Effect,
    f: &mut dyn FnMut(&crate::effect::Effect) -> Option<crate::effect::Effect>,
) -> crate::effect::Effect {
    macro_rules! remap {
        ($type:ty, $($field:ident),+) => {
            if let Some(container) = effect.downcast_ref::<$type>() {
                let mut container = container.clone();
                $(container.$field = container.$field.iter().map(|child| map_effect_tree(child, f)).collect();)+
                let rebuilt = crate::effect::Effect::new(container);
                return f(&rebuilt).unwrap_or(rebuilt);
            }
        };
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let mut tagged = tagged.clone();
        tagged.effect = Box::new(map_effect_tree(&tagged.effect, f));
        let rebuilt = crate::effect::Effect::new(tagged);
        return f(&rebuilt).unwrap_or(rebuilt);
    }
    if let Some(with_id) = effect.as_with_id() {
        let rebuilt =
            crate::effect::Effect::with_id(with_id.id.0, map_effect_tree(&with_id.effect, f));
        return f(&rebuilt).unwrap_or(rebuilt);
    }
    remap!(crate::effects::SequenceEffect, effects);
    remap!(crate::effects::MayEffect<crate::effect::Effect>, effects);
    remap!(crate::effects::IfEffect, then, else_);
    remap!(crate::effects::ConditionalEffect, if_true, if_false);
    remap!(crate::effects::ScheduleDelayedTriggerEffect, effects);
    remap!(crate::effects::ReflexiveTriggerEffect, effects);
    f(effect).unwrap_or_else(|| effect.clone())
}

const ATTACHED_BY_THIS_TAG: &str = "__attached_by_this_effect";

/// "You may attach an Equipment you control to that creature. If you do,
/// unattach it at the beginning of the next end step": "it" is the Equipment
/// just attached, not the creature it was attached to. Record the attached
/// object with an explicit choice and point the unattach at it.
fn bind_unattach_to_attached_object(program: &mut crate::resolution::ResolutionProgram) {
    fn host_tag(spec: &ChooseSpec) -> Option<TagKey> {
        match spec.base() {
            ChooseSpec::Tagged(tag) => Some(tag.clone()),
            _ => None,
        }
    }
    // Find an attach whose objects are an untagged choice and whose
    // destination is a tagged object, and an unattach of that tagged object.
    let mut attach_host: Option<TagKey> = None;
    let mut unattach_hosts: Vec<TagKey> = Vec::new();
    fn scan(
        effect: &crate::effect::Effect,
        attach: &mut Option<TagKey>,
        unattach: &mut Vec<TagKey>,
    ) {
        if let Some(attach_effect) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>()
            && matches!(attach_effect.objects.base(), ChooseSpec::Object(filter) if filter.tagged_constraints.is_empty())
            && let Some(tag) = host_tag(&attach_effect.target)
        {
            attach.get_or_insert(tag);
        }
        if let Some(unattach_effect) =
            effect.downcast_ref::<crate::effects::UnattachObjectsEffect>()
            && let Some(tag) = host_tag(&unattach_effect.objects)
        {
            unattach.push(tag);
        }
        effect.visit_child_effects(&mut |child| scan(child, attach, unattach));
    }
    for segment in &program.segments {
        for effect in &segment.default_effects {
            scan(effect, &mut attach_host, &mut unattach_hosts);
        }
    }
    let Some(host) = attach_host else {
        return;
    };
    if !unattach_hosts.contains(&host) {
        return;
    }
    let mut rewrite = |effect: &crate::effect::Effect| -> Option<crate::effect::Effect> {
        if let Some(attach_effect) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>()
            && host_tag(&attach_effect.target).as_ref() == Some(&host)
            && let ChooseSpec::Object(filter) = attach_effect.objects.base()
        {
            let choose = crate::effects::ChooseObjectsEffect::new(
                filter.clone(),
                attach_effect.objects.count(),
                crate::target::PlayerFilter::You,
                ATTACHED_BY_THIS_TAG,
            );
            let mut attach_effect = attach_effect.clone();
            attach_effect.objects = ChooseSpec::Tagged(ATTACHED_BY_THIS_TAG.into())
                .with_surface_hints([crate::target::ChooseSpecSurfaceHint::SourceReference(
                    crate::target::SourceReferenceSurface::ThisPermanentType(
                        "the chosen object".to_string(),
                    ),
                )]);
            return Some(crate::effect::Effect::new(
                crate::effects::SequenceEffect::new(vec![
                    crate::effect::Effect::new(choose),
                    crate::effect::Effect::new(attach_effect),
                ]),
            ));
        }
        if let Some(unattach_effect) =
            effect.downcast_ref::<crate::effects::UnattachObjectsEffect>()
            && host_tag(&unattach_effect.objects).as_ref() == Some(&host)
        {
            return Some(crate::effect::Effect::new(
                crate::effects::UnattachObjectsEffect::new(
                    ChooseSpec::Tagged(ATTACHED_BY_THIS_TAG.into()).with_surface_hints([
                        crate::target::ChooseSpecSurfaceHint::SourceReference(
                            crate::target::SourceReferenceSurface::ThisPermanentType(
                                "it".to_string(),
                            ),
                        ),
                    ]),
                ),
            ));
        }
        None
    };
    for segment in &mut program.segments {
        segment.default_effects = segment
            .default_effects
            .iter()
            .map(|effect| map_effect_tree(effect, &mut rewrite))
            .collect();
    }
}

/// "You may tap or untap target creature. Then that creature's controller
/// may pay ...": the target is announced whether or not the optional action
/// is taken, so a later "that creature" must not depend on the optional
/// action having tagged it. Hoist the May's leading target declaration out of
/// the May, tagged with the name the later reference uses. (An UnlessPays
/// whose payer or payment names a target declared by its own leading
/// instruction is left alone: the runtime binds that declaration first.)
fn hoist_optional_target_declaration(program: &mut crate::resolution::ResolutionProgram) {
    for segment_index in 0..program.segments.len() {
        let mut index = 0;
        while index < program.segments[segment_index].default_effects.len() {
            let effect = program.segments[segment_index].default_effects[index].clone();
            let Some(may) =
                effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
            else {
                index += 1;
                continue;
            };
            let Some(target_only) = may
                .effects
                .first()
                .and_then(|first| first.downcast_ref::<crate::effects::TargetOnlyEffect>())
            else {
                index += 1;
                continue;
            };
            let Some(tag) = may.effects.iter().skip(1).find_map(|inner| {
                let tagged = inner.downcast_ref::<crate::effects::TaggedEffect>()?;
                Some(tagged.tag.clone())
            }) else {
                index += 1;
                continue;
            };
            // Referenced by an instruction after the May?
            let needle = format!("TagKey({:?})", tag.as_str());
            let referenced_later = program
                .segments
                .iter()
                .enumerate()
                .flat_map(|(seg, segment)| {
                    segment
                        .default_effects
                        .iter()
                        .enumerate()
                        .filter(move |(position, _)| {
                            seg > segment_index || (seg == segment_index && *position > index)
                        })
                        .map(|(_, effect)| effect)
                })
                .any(|later| format!("{later:?}").contains(&needle));
            if !referenced_later {
                index += 1;
                continue;
            }
            let declaration = crate::effect::Effect::new(target_only.clone()).tag(tag);
            let mut may = may.clone();
            may.effects.remove(0);
            let segment = &mut program.segments[segment_index];
            segment.default_effects[index] = crate::effect::Effect::new(may);
            segment.default_effects.insert(index, declaration);
            index += 2;
        }
    }
}

/// "Destroy all Plains. For each land destroyed this way, this deals 1 damage
/// to that land's controller unless that player pays {2}": one payment
/// decision per destroyed land, each made by that land's controller, rather
/// than a single payment by one representative controller. Rewrite
/// `unless(controller of tagged) { repeat N { damage controller of tagged } }`
/// into a per-controller loop that repeats the per-object unless for each of
/// that controller's tagged objects.
fn distribute_unless_over_tagged_controllers(program: &mut crate::resolution::ResolutionProgram) {
    use crate::target::PlayerFilter;
    fn controller_tag(player: &PlayerFilter) -> Option<TagKey> {
        match player {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
            | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag)) => {
                Some(tag.clone())
            }
            _ => None,
        }
    }
    for segment in &mut program.segments {
        for effect in segment.default_effects.iter_mut() {
            let Some(unless) =
                effect.downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
            else {
                continue;
            };
            let Some(tag) = controller_tag(&unless.player) else {
                continue;
            };
            let [repeat] = unless.effects.as_slice() else {
                continue;
            };
            let Some(repeat) = repeat.downcast_ref::<crate::effects::RepeatEffectsEffect>() else {
                continue;
            };
            let Some(per_object) = repeat
                .effects
                .iter()
                .map(|inner| {
                    let damage = inner.downcast_ref::<crate::effects::DealDamageEffect>()?;
                    let ChooseSpec::Player(player) = damage.target.base() else {
                        return None;
                    };
                    (controller_tag(player).as_ref() == Some(&tag)).then(|| {
                        let mut damage = damage.clone();
                        damage.target = ChooseSpec::Player(PlayerFilter::IteratedPlayer);
                        crate::effect::Effect::new(damage)
                    })
                })
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let mut per_object_unless = unless.clone();
            per_object_unless.player = PlayerFilter::IteratedPlayer;
            per_object_unless.effects = per_object;
            *effect = crate::effect::Effect::new(crate::effects::ForEachControllerOfTaggedEffect {
                tag: tag.clone(),
                effects: vec![crate::effect::Effect::repeat_effects(
                    crate::effect::Value::TaggedCount,
                    vec![crate::effect::Effect::new(per_object_unless)],
                )],
            });
        }
    }
}

/// "Look at the top five cards of your library. For each card, put that card
/// into your graveyard unless ...": an unqualified per-card loop right after a
/// look iterates the looked-at cards, not every object.
fn bind_bare_for_each_to_looked_cards(effects: &mut [crate::effect::Effect]) {
    use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
    for index in 1..effects.len() {
        let Some(look) = effects[index - 1].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        else {
            continue;
        };
        let tag = look.tag.clone();
        let Some(for_each) = effects[index].downcast_ref::<crate::effects::ForEachObject>() else {
            continue;
        };
        if for_each.filter.zone.is_some() || !for_each.filter.tagged_constraints.is_empty() {
            continue;
        }
        let mut probe = for_each.filter.clone();
        probe.union_surface = ObjectFilter::default().union_surface;
        probe.power_reference = ObjectFilter::default().power_reference;
        probe.toughness_reference = ObjectFilter::default().toughness_reference;
        if probe != ObjectFilter::default() && !for_each.filter.card_types.is_empty() {
            continue;
        }
        let mut for_each = for_each.clone();
        for_each.filter.zone = Some(Zone::Library);
        for_each
            .filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag,
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        effects[index] = crate::effect::Effect::new(for_each);
    }
}

/// "Choose two target creatures ... Their controller chooses one of them":
/// a choice among targeted permanents looks only on the battlefield, not in
/// hidden zones (which would route it through hidden-hand choice handling).
fn restrict_choice_among_targets_to_battlefield(effects: &mut [crate::effect::Effect]) {
    use crate::filter::TaggedOpbjectRelation;
    let mut battlefield_target_tags: Vec<TagKey> = Vec::new();
    for index in 0..effects.len() {
        if let Some(tagged) = effects[index].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(target_only) = tagged
                .effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
            && let ChooseSpec::Object(filter) = target_only.target.base()
            && filter.zone == Some(Zone::Battlefield)
        {
            battlefield_target_tags.push(tagged.tag.clone());
            continue;
        }
        let Some(choose) = effects[index].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        else {
            continue;
        };
        if choose.additional_zones.is_empty()
            || !choose.filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && battlefield_target_tags.contains(&constraint.tag)
            })
        {
            continue;
        }
        let mut choose = choose.clone();
        choose.zone = Some(Zone::Battlefield);
        choose.additional_zones.clear();
        effects[index] = crate::effect::Effect::new(choose);
    }
}

/// Card-level dataflow repairs applied to every resolution program.
fn finalize_program_references(program: &mut crate::resolution::ResolutionProgram) {
    let mut pass = |program: &mut crate::resolution::ResolutionProgram| {
        // Consecutive segments resolve as one instruction list.
        let mut flat: Vec<crate::effect::Effect> = Vec::new();
        let mut lengths = Vec::new();
        for segment in &mut program.segments {
            lengths.push(segment.default_effects.len());
            flat.append(&mut segment.default_effects);
        }
        bind_bare_for_each_to_looked_cards(&mut flat);
        restrict_choice_among_targets_to_battlefield(&mut flat);
        let mut rest = flat.into_iter();
        for (segment, length) in program.segments.iter_mut().zip(lengths) {
            segment.default_effects = rest.by_ref().take(length).collect();
        }
        crate::compile_support::link_unproduced_result_references_in_program(program);
        bind_unproduced_rest_complement(program);
    };
    for segment in &mut program.segments {
        for_each_scheduled_delayed_program(&mut segment.default_effects, &mut pass);
    }
    pass(program);
}

/// The same repairs for a bare instruction list (an opening-hand action's
/// scheduled consequence).
pub(crate) fn finalize_effect_list_references(
    effects: Vec<crate::effect::Effect>,
) -> Vec<crate::effect::Effect> {
    let mut program = crate::resolution::ResolutionProgram::from_effects(effects);
    finalize_program_references(&mut program);
    program
        .segments
        .into_iter()
        .flat_map(|segment| segment.default_effects)
        .collect()
}

/// "Put the rest ...", "destroy the rest", "exile the rest": a consumer of the
/// complement tag with no producer anywhere in the program would silently act
/// on nothing. Tag the complement of the collection it was carved from
/// immediately before it.
fn bind_unproduced_rest_complement(program: &mut crate::resolution::ResolutionProgram) {
    let mut produces = false;
    let mut consumes = false;
    for segment in &program.segments {
        for effect in segment.default_effects.iter().chain(
            segment
                .self_replacements
                .iter()
                .flat_map(|branch| branch.replacement_effects.iter()),
        ) {
            let (produced, consumed) = debug_mentions_rest_tag(effect);
            produces |= produced;
            consumes |= consumed;
        }
    }
    if produces || !consumes {
        return;
    }
    let mut flat: Vec<crate::effect::Effect> = Vec::new();
    let mut lengths = Vec::new();
    for segment in &mut program.segments {
        lengths.push(segment.default_effects.len());
        flat.append(&mut segment.default_effects);
    }
    let bound = bind_rest_complement_in_list(&mut flat, &[]);
    // The producer sits right before its consumer, inside the consumer's
    // segment.
    if let Some(Some(index)) = bound {
        let mut start = 0;
        for length in lengths.iter_mut() {
            if index < start + *length {
                *length += 1;
                break;
            }
            start += *length;
        }
    }
    let mut rest = flat.into_iter();
    for (segment, length) in program.segments.iter_mut().zip(lengths) {
        segment.default_effects = rest.by_ref().take(length).collect();
    }
    if bound.is_some() {
        return;
    }
    for segment in &mut program.segments {
        for branch in &mut segment.self_replacements {
            if bind_rest_complement_in_list(&mut branch.replacement_effects, &[]).is_some() {
                return;
            }
        }
    }
}

fn is_runtime_damage_aggregate_member(effect: &crate::effect::Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .is_some()
    {
        return true;
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return is_runtime_damage_aggregate_member(&tagged.effect);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        return !for_each.effects.is_empty()
            && for_each
                .effects
                .iter()
                .all(is_runtime_damage_aggregate_member);
    }
    false
}

/// A terminal coordinated damage aggregate is exported defensively so a
/// later source statement can consume its result. Once the complete card is
/// materialized and the aggregate is still the final instruction, no later
/// consumer exists; remove the otherwise-obscuring observation wrapper.
fn strip_terminal_unconsumed_damage_aggregate_id(
    program: &mut crate::resolution::ResolutionProgram,
) {
    let Some(segment) = program.segments.last_mut() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let Some(effect) = segment.default_effects.last_mut() else {
        return;
    };
    let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return;
    };
    let Some(sequence) = with_id
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()
    else {
        return;
    };
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || sequence.effects.len() < 2
        || !sequence
            .effects
            .iter()
            .all(is_runtime_damage_aggregate_member)
    {
        return;
    }
    *effect = with_id.effect.as_ref().clone();
    *program = crate::resolution::ResolutionProgram::new(program.segments.clone());
}

fn tagged_return_to_hand(
    effect: &crate::effect::Effect,
) -> Option<(&crate::tag::TagKey, &crate::effects::ReturnToHandEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let returned = tagged
        .effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    Some((&tagged.tag, returned))
}

/// A clash result can be observed before the later member of one authored
/// coordination. The following win sentence replaces that return's
/// destination; it does not execute a second move. Fold the two typed source
/// segments into an observed clash plus a local zone rewrite.
fn correlate_clash_win_return_replacement(program: &mut crate::resolution::ResolutionProgram) {
    let [clash_segment, win_segment] = program.segments.as_slice() else {
        return;
    };
    if !clash_segment.self_replacements.is_empty()
        || !win_segment.self_replacements.is_empty()
        || clash_segment.default_effects.len() != 1
        || win_segment.default_effects.len() != 1
    {
        return;
    }
    let Some(sequence) =
        clash_segment.default_effects[0].downcast_ref::<crate::effects::SequenceEffect>()
    else {
        return;
    };
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen
        || sequence.result_label.is_some()
    {
        return;
    }
    let [observed_clash, returned_effect] = sequence.effects.as_slice() else {
        return;
    };
    let Some(observed_clash) = observed_clash.downcast_ref::<crate::effects::WithIdEffect>() else {
        return;
    };
    if observed_clash
        .effect
        .downcast_ref::<crate::effects::ClashEffect>()
        .is_none()
    {
        return;
    }
    let Some((returned_tag, _)) = tagged_return_to_hand(returned_effect) else {
        return;
    };
    let Some(win) = win_segment.default_effects[0].downcast_ref::<crate::effects::IfEffect>()
    else {
        return;
    };
    if win.condition != observed_clash.id
        || win.predicate != crate::effect::EffectPredicate::Happened
        || !win.else_.is_empty()
        || win.per_player_result
        || win.prior_result_replacement_surface
    {
        return;
    }
    let [may_move] = win.then.as_slice() else {
        return;
    };
    let Some(may_move) =
        may_move.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
    else {
        return;
    };
    if may_move.decider.as_ref() != Some(&PlayerFilter::You) {
        return;
    }
    let [move_effect] = may_move.effects.as_slice() else {
        return;
    };
    let Some(tagged_move) = move_effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(move_to_zone) = tagged_move
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return;
    };
    let references_returned_object = match move_to_zone.target.unhinted() {
        ChooseSpec::Object(filter) => filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *returned_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }),
        ChooseSpec::Tagged(tag) => tag == returned_tag,
        _ => false,
    };
    if !references_returned_object
        || move_to_zone.zone != Zone::Library
        || !move_to_zone.to_top
        || move_to_zone.library_order.is_some()
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.enters_transformed
    {
        return;
    }

    let replacement = crate::effects::RegisterZoneReplacementEffect::new(
        ChooseSpec::Tagged(returned_tag.clone()),
        Some(Zone::Battlefield),
        Some(Zone::Hand),
        Zone::Library,
        crate::effects::ReplacementApplyMode::OneShot,
    )
    .with_library_placement(ironsmith_core::ZoneReplacementLibraryPlacement::Top)
    .optional("put it on top of its owner's library");
    let local_rewrite = crate::effect::Effect::new(crate::effects::LocalRewriteEffect::new(
        returned_effect.clone(),
        vec![replacement],
    ));
    let conditional = crate::effect::Effect::new(crate::effects::IfEffect::new(
        observed_clash.id,
        crate::effect::EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0)),
        vec![local_rewrite],
        vec![returned_effect.clone()],
    ));

    let mut clash_segment = clash_segment.clone();
    clash_segment.default_effects = vec![crate::effect::Effect::with_id(
        observed_clash.id.0,
        observed_clash.effect.as_ref().clone(),
    )];
    let mut win_segment = win_segment.clone();
    win_segment.default_effects = vec![conditional];
    *program = crate::resolution::ResolutionProgram::new(vec![clash_segment, win_segment]);
}

/// A searched card is still in the library when its destination is chosen.
/// Fold an exact later `put searched card onto the battlefield` rider into a
/// local Library -> Hand replacement, restricted to the searched card and the
/// source's chosen creature type. This makes the destination one zone-change
/// event rather than moving the card out of the hand afterward.
fn correlate_additional_cost_chosen_type_search_destination(
    program: &mut crate::resolution::ResolutionProgram,
) {
    let [search_segment, rider_segment] = program.segments.as_slice() else {
        return;
    };
    if !search_segment.self_replacements.is_empty()
        || !rider_segment.self_replacements.is_empty()
        || search_segment.default_effects.len() != 1
        || rider_segment.default_effects.len() != 1
    {
        return;
    }
    let Some(tagged_search) =
        search_segment.default_effects[0].downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return;
    };
    let Some(search) = tagged_search.effect.as_search() else {
        return;
    };
    if search.destination != Zone::Hand
        || !search.reveal
        || !matches!(
            search.filter.card_types.as_slice(),
            [crate::types::CardType::Creature]
        )
    {
        return;
    }
    let Some(conditional) =
        rider_segment.default_effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return;
    };
    if !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
        || !matches!(
            &conditional.condition,
            crate::effect::Condition::ThisSpellPaidLabel(label)
                if label.kind == crate::cost::OptionalCostKind::Additional
        )
    {
        return;
    }
    let Some(tagged_move) = conditional.if_true[0].downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return;
    };
    let Some(move_to_zone) = tagged_move
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return;
    };
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.enters_transformed
        || !matches!(
            move_to_zone.target.unhinted(),
            ChooseSpec::Tagged(tag) if tag == &tagged_search.tag
        )
    {
        return;
    }

    let mut chosen_type_card = search.filter.clone();
    chosen_type_card.chosen_creature_type = true;
    let replacement = crate::effects::RegisterZoneReplacementEffect::new(
        ChooseSpec::Object(chosen_type_card),
        Some(Zone::Library),
        Some(Zone::Hand),
        Zone::Battlefield,
        crate::effects::ReplacementApplyMode::OneShot,
    );
    let rewritten_search = crate::effect::Effect::new(crate::effects::LocalRewriteEffect::new(
        search_segment.default_effects[0].clone(),
        vec![replacement],
    ));
    let mut segment =
        crate::resolution::ResolutionSegment::from_effects(search_segment.default_effects.clone());
    segment.starts_new_source_line = search_segment.starts_new_source_line;
    segment
        .self_replacements
        .push(crate::resolution::SelfReplacementBranch::new(
            conditional.condition.clone(),
            vec![rewritten_search],
        ));
    *program = crate::resolution::ResolutionProgram::new(vec![segment]);
}

#[cfg(test)]
mod chosen_type_search_destination_tests {
    use super::*;
    use crate::target::PlayerFilter;

    fn program(move_tag: &str, reveal: bool) -> crate::resolution::ResolutionProgram {
        let search = crate::effect::Effect::new(crate::effects::SearchLibraryEffect::to_hand(
            ObjectFilter::creature().in_zone(Zone::Library),
            PlayerFilter::You,
            reveal,
        ))
        .tag("searched");
        let move_card = crate::effect::Effect::move_to_zone(
            ChooseSpec::Tagged(ironsmith_compiler_semantic::tag::declared_key(move_tag).into()),
            Zone::Battlefield,
            false,
        )
        .tag("moved");
        let rider = crate::effect::Effect::conditional(
            crate::effect::Condition::ThisSpellPaidLabel(crate::cost::OptionalCostRef::new(
                crate::cost::OptionalCostKind::Additional,
            )),
            vec![move_card],
            vec![],
        );
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![search]),
            crate::resolution::ResolutionSegment::from_effects(vec![rider]),
        ])
    }

    #[test]
    fn chosen_type_search_uses_one_library_zone_change() {
        let mut matching = program("searched", true);
        correlate_additional_cost_chosen_type_search_destination(&mut matching);
        assert_eq!(matching.segments.len(), 1);
        let [branch] = matching.segments[0].self_replacements.as_slice() else {
            panic!("expected one destination self-replacement: {matching:#?}");
        };
        let local = branch.replacement_effects[0]
            .downcast_ref::<crate::effects::LocalRewriteEffect>()
            .expect("replacement should wrap the original search");
        let [replacement] = local.zone_replacements.as_slice() else {
            panic!("expected one local zone replacement: {local:#?}");
        };
        assert_eq!(replacement.from_zone, Some(Zone::Library));
        assert_eq!(replacement.to_zone, Some(Zone::Hand));
        assert_eq!(replacement.replacement_zone, Zone::Battlefield);
        assert!(matches!(
            &replacement.target,
            ChooseSpec::Object(filter) if filter.chosen_creature_type
        ));
    }

    #[test]
    fn uncorrelated_or_unrevealed_searches_are_near_misses() {
        for mut near_miss in [program("other", true), program("searched", false)] {
            correlate_additional_cost_chosen_type_search_destination(&mut near_miss);
            assert_eq!(near_miss.segments.len(), 2, "{near_miss:#?}");
        }
    }
}

/// Turn the strict two-sentence surface
/// `Deal N damage to target ... . If the additional cost was paid, deal M
/// damage instead.` into one executable conditional replacement. The second
/// sentence's implicit player-or-planeswalker target is accepted only because
/// the preceding spell segment proves the explicit target; combat/source
/// flags must agree and every branch must be otherwise empty.
fn correlate_additional_cost_damage_replacement(
    program: &mut crate::resolution::ResolutionProgram,
) {
    fn tagged_damage(effect: &crate::effect::Effect) -> Option<&crate::effects::DealDamageEffect> {
        if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
            return Some(damage);
        }
        let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
        tagged
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
    }

    let [base_segment, replacement_segment] = program.segments.as_slice() else {
        return;
    };
    if !base_segment.self_replacements.is_empty()
        || !replacement_segment.self_replacements.is_empty()
        || base_segment.default_effects.len() != 1
        || replacement_segment.default_effects.len() != 1
    {
        return;
    }
    let Some(base_damage) = tagged_damage(&base_segment.default_effects[0]) else {
        return;
    };
    let Some(conditional) =
        replacement_segment.default_effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return;
    };
    if !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
        || !matches!(
            &conditional.condition,
            crate::effect::Condition::ThisSpellPaidLabel(label)
                if label.kind == crate::cost::OptionalCostKind::Additional
        )
    {
        return;
    }
    let Some(replacement_damage) =
        conditional.if_true[0].downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return;
    };
    if replacement_damage.source_is_combat != base_damage.source_is_combat
        || replacement_damage.unpreventable != base_damage.unpreventable
        || !matches!(
            replacement_damage.target.unhinted(),
            crate::target::ChooseSpec::PlayerOrPlaneswalker(crate::target::PlayerFilter::Any)
        )
        || !matches!(
            base_damage.target.base(),
            crate::target::ChooseSpec::Object(filter)
                if filter.card_types.contains(&crate::types::CardType::Planeswalker)
        )
    {
        return;
    }

    let mut replacement_damage = replacement_damage.clone();
    replacement_damage.target = base_damage.target.clone();
    let replacement = crate::effect::Effect::new(crate::effects::ConditionalEffect {
        condition: conditional.condition.clone(),
        if_true: vec![crate::effect::Effect::new(replacement_damage)],
        if_false: vec![crate::effect::Effect::new(base_damage.clone())],
        surface: conditional.surface,
        capture_condition_result: conditional.capture_condition_result,
    });
    *program = crate::resolution::ResolutionProgram::from_effects(vec![replacement]);
}

#[cfg(test)]
mod additional_cost_damage_replacement_tests {
    use super::*;

    fn base_target() -> crate::target::ChooseSpec {
        crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::any_of_types(&[
                crate::types::CardType::Creature,
                crate::types::CardType::Planeswalker,
            ]),
        ))
    }

    fn program(
        base_target: crate::target::ChooseSpec,
        replacement_target: crate::target::ChooseSpec,
        replacement_combat: bool,
    ) -> crate::resolution::ResolutionProgram {
        let base = crate::effect::Effect::new(crate::effects::DealDamageEffect::new(
            crate::effect::Value::Fixed(2),
            base_target,
        ));
        let mut replacement_damage = crate::effects::DealDamageEffect::new(
            crate::effect::Value::Fixed(4),
            replacement_target,
        );
        replacement_damage.source_is_combat = replacement_combat;
        let replacement = crate::effect::Effect::new(crate::effects::ConditionalEffect::if_only(
            crate::effect::Condition::ThisSpellPaidLabel(crate::cost::OptionalCostRef::new(
                crate::cost::OptionalCostKind::Additional,
            )),
            vec![crate::effect::Effect::new(replacement_damage)],
        ));
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![base]),
            crate::resolution::ResolutionSegment::from_effects(vec![replacement]),
        ])
    }

    #[test]
    fn refreshed_instead_correlates_only_matching_noncombat_implicit_damage_replacement() {
        let explicit = base_target();
        let implicit =
            crate::target::ChooseSpec::PlayerOrPlaneswalker(crate::target::PlayerFilter::Any);
        let mut matching = program(explicit.clone(), implicit.clone(), false);
        correlate_additional_cost_damage_replacement(&mut matching);
        assert_eq!(matching.segments.len(), 1);
        let conditional = matching.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .expect("matching program should become one conditional");
        assert_eq!(
            conditional.if_true[0]
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .expect("replacement damage")
                .target,
            explicit
        );

        let wrong_target = crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Any);
        let mut target_near_miss = program(explicit.clone(), wrong_target, false);
        correlate_additional_cost_damage_replacement(&mut target_near_miss);
        assert_eq!(target_near_miss.segments.len(), 2);

        let mut source_near_miss = program(explicit, implicit, true);
        correlate_additional_cost_damage_replacement(&mut source_near_miss);
        assert_eq!(source_near_miss.segments.len(), 2);
    }
}

/// Correlate later "that cost was paid" conditions with a real alternative
/// casting method on the same card. The broad condition grammar cannot know
/// which earlier line introduced a mana-only alternative cost, so it retains
/// the authored label temporarily. This final pass accepts only an exact,
/// unique method-name or canonical typed-mana match and replaces it with an
/// executable `AlternativeCast` reference.
fn link_alternative_cast_condition_references(builder: &mut CardDefinitionBuilder) {
    let [method] = builder.alternative_casts.as_slice() else {
        return;
    };
    let method_name = method.name();
    let mana_cost = method.mana_cost();

    fn link_condition(
        condition: &crate::effect::Condition,
        method_name: &str,
        mana_cost: Option<&crate::mana::ManaCost>,
        allow_that: bool,
    ) -> Option<crate::effect::Condition> {
        use ironsmith_core::{AlternativeCostReference, OptionalCostKind, OptionalCostRef};

        match condition {
            crate::effect::Condition::ThisSpellPaidLabel(label) => {
                let OptionalCostKind::CustomUnsupported(raw) = &label.kind else {
                    return None;
                };
                let reference = if mana_cost.is_some_and(|mana| raw == &mana.to_oracle()) {
                    AlternativeCostReference::by_mana_cost(method_name, mana_cost?)
                } else if raw.eq_ignore_ascii_case(method_name) {
                    AlternativeCostReference::by_name(method_name, mana_cost)
                } else if allow_that && raw.eq_ignore_ascii_case("that") {
                    AlternativeCostReference::as_that_cost(method_name, mana_cost)
                } else {
                    return None;
                };
                let mut linked = OptionalCostRef::new(OptionalCostKind::AlternativeCast(reference));
                linked.payment_window = label.payment_window;
                Some(crate::effect::Condition::ThisSpellPaidLabel(linked))
            }
            crate::effect::Condition::Not(inner) => {
                link_condition(inner, method_name, mana_cost, allow_that)
                    .map(|inner| crate::effect::Condition::Not(Box::new(inner)))
            }
            crate::effect::Condition::And(left, right) => {
                let left_rewritten = link_condition(left, method_name, mana_cost, allow_that);
                let right_rewritten = link_condition(right, method_name, mana_cost, allow_that);
                match (left_rewritten, right_rewritten) {
                    (None, None) => None,
                    (left_rewritten, right_rewritten) => Some(crate::effect::Condition::And(
                        Box::new(left_rewritten.unwrap_or_else(|| left.as_ref().clone())),
                        Box::new(right_rewritten.unwrap_or_else(|| right.as_ref().clone())),
                    )),
                }
            }
            crate::effect::Condition::Or(left, right) => {
                let left_rewritten = link_condition(left, method_name, mana_cost, allow_that);
                let right_rewritten = link_condition(right, method_name, mana_cost, allow_that);
                match (left_rewritten, right_rewritten) {
                    (None, None) => None,
                    (left_rewritten, right_rewritten) => Some(crate::effect::Condition::Or(
                        Box::new(left_rewritten.unwrap_or_else(|| left.as_ref().clone())),
                        Box::new(right_rewritten.unwrap_or_else(|| right.as_ref().clone())),
                    )),
                }
            }
            _ => None,
        }
    }

    fn link_effect(
        effect: &crate::effect::Effect,
        method_name: &str,
        mana_cost: Option<&crate::mana::ManaCost>,
        allow_that: bool,
    ) -> (crate::effect::Effect, bool) {
        if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
            let mut conditional = conditional.clone();
            let condition =
                link_condition(&conditional.condition, method_name, mana_cost, allow_that);
            let mut any = condition.is_some();
            if let Some(rewritten) = condition {
                conditional.condition = rewritten;
            }
            let branch_antecedent = allow_that || any;
            for branch in [&mut conditional.if_true, &mut conditional.if_false] {
                let mut local = branch_antecedent;
                *branch = branch.iter().map(|child| {
                    let (child, changed) = link_effect(child, method_name, mana_cost, local);
                    local |= changed;
                    any |= changed;
                    child
                }).collect();
            }
            return (crate::effect::Effect::new(conditional), any);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            let mut any = false;
            sequence.effects = sequence
                .effects
                .iter()
                .map(|child| {
                    let (child, changed) =
                        link_effect(child, method_name, mana_cost, allow_that || any);
                    any |= changed;
                    child
                })
                .collect();
            return (crate::effect::Effect::new(sequence), any);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            let (child, changed) = link_effect(&tagged.effect, method_name, mana_cost, allow_that);
            tagged.effect = Box::new(child);
            return (crate::effect::Effect::new(tagged), changed);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            let (child, changed) = link_effect(&with_id.effect, method_name, mana_cost, allow_that);
            with_id.effect = Box::new(child);
            return (crate::effect::Effect::new(with_id), changed);
        }
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        {
            let mut may = may.clone();
            let mut any = false;
            may.effects = may
                .effects
                .iter()
                .map(|child| {
                    let (child, changed) =
                        link_effect(child, method_name, mana_cost, allow_that || any);
                    any |= changed;
                    child
                })
                .collect();
            return (crate::effect::Effect::new(may), any);
        }
        (effect.clone(), false)
    }

    fn link_program(
        program: &mut crate::resolution::ResolutionProgram,
        method_name: &str,
        mana_cost: Option<&crate::mana::ManaCost>,
        header_matched: bool,
    ) {
        let mut prior_matched = header_matched;
        for segment in &mut program.segments {
            let mut local = prior_matched;
            segment.default_effects = segment.default_effects.iter().map(|effect| {
                let (effect, matched) = link_effect(effect, method_name, mana_cost, local);
                local |= matched;
                effect
            }).collect();
            for branch in &mut segment.self_replacements {
                let linked = link_condition(&branch.condition, method_name, mana_cost, prior_matched);
                let mut branch_matched = prior_matched || linked.is_some();
                if let Some(condition) = linked { branch.condition = condition; }
                branch.replacement_effects = branch.replacement_effects.iter().map(|effect| {
                    let (effect, matched) = link_effect(effect, method_name, mana_cost, branch_matched);
                    branch_matched |= matched;
                    effect
                }).collect();
            }
            prior_matched = local;
        }
        *program = crate::resolution::ResolutionProgram::new(std::mem::take(&mut program.segments));
    }

    if let Some(program) = builder.spell_effect.as_mut() {
        link_program(program, method_name, mana_cost, false);
    }
    for ability in &mut builder.abilities {
        match &mut ability.kind {
            crate::ability::AbilityKind::Triggered(triggered) => {
                let linked = triggered.intervening_if.as_ref()
                    .and_then(|condition| link_condition(condition, method_name, mana_cost, false));
                let header_matched = linked.is_some();
                if let Some(condition) = linked { triggered.intervening_if = Some(condition); }
                link_program(&mut triggered.effects, method_name, mana_cost, header_matched);
            }
            crate::ability::AbilityKind::Activated(activated) => {
                let linked = activated.activation_condition.as_ref()
                    .and_then(|condition| link_condition(condition, method_name, mana_cost, false));
                let mut header_matched = linked.is_some();
                if let Some(condition) = linked { activated.activation_condition = Some(condition); }
                for condition in &mut activated.activation_restrictions {
                    if let Some(linked) = link_condition(condition, method_name, mana_cost, false) {
                        *condition = linked;
                        header_matched = true;
                    }
                }
                link_program(&mut activated.effects, method_name, mana_cost, header_matched);
            }
            _ => {}
        }
    }
}

/// Bind a player-relative value inside a quantified damage action to the
/// same participant as its executable recipient. The broad player-value
/// grammar initially represents “that player's life total” as `Target(Any)`;
/// inside `ForPlayers`, an `IteratedPlayer` damage target proves the referent
/// without consulting source text or inventing a separate target choice.
fn bind_quantified_player_damage_values(program: &mut crate::resolution::ResolutionProgram) {
    fn bind_value(value: &mut crate::effect::Value) -> bool {
        match value {
            crate::effect::Value::SurfaceHinted { value, .. }
            | crate::effect::Value::HalfRoundedDown(value)
            | crate::effect::Value::DividedRoundedDown(value, _)
            | crate::effect::Value::Scaled(value, _) => bind_value(value),
            crate::effect::Value::LifeTotal(player)
                if *player
                    == crate::target::PlayerFilter::Target(Box::new(
                        crate::target::PlayerFilter::Any,
                    )) =>
            {
                *player = crate::target::PlayerFilter::IteratedPlayer;
                true
            }
            _ => false,
        }
    }

    fn bind_in_player_action(effect: &crate::effect::Effect) -> crate::effect::Effect {
        if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>()
            && matches!(
                damage.target.unhinted(),
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::IteratedPlayer)
            )
        {
            let mut damage = damage.clone();
            if bind_value(&mut damage.amount) {
                return crate::effect::Effect::new(damage);
            }
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = sequence.effects.iter().map(bind_in_player_action).collect();
            return crate::effect::Effect::new(sequence);
        }
        if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
            let mut conditional = conditional.clone();
            conditional.if_true = conditional
                .if_true
                .iter()
                .map(bind_in_player_action)
                .collect();
            conditional.if_false = conditional
                .if_false
                .iter()
                .map(bind_in_player_action)
                .collect();
            return crate::effect::Effect::new(conditional);
        }
        if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
            let mut if_effect = if_effect.clone();
            if_effect.then = if_effect.then.iter().map(bind_in_player_action).collect();
            if_effect.else_ = if_effect.else_.iter().map(bind_in_player_action).collect();
            return crate::effect::Effect::new(if_effect);
        }
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        {
            let mut may = may.clone();
            may.effects = may.effects.iter().map(bind_in_player_action).collect();
            return crate::effect::Effect::new(may);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            tagged.effect = Box::new(bind_in_player_action(&tagged.effect));
            return crate::effect::Effect::new(tagged);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(bind_in_player_action(&with_id.effect));
            return crate::effect::Effect::new(with_id);
        }
        if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        {
            let mut with_source = with_source.clone();
            with_source.effect = Box::new(bind_in_player_action(&with_source.effect));
            return crate::effect::Effect::new(with_source);
        }
        effect.clone()
    }

    fn rewrite(effect: &crate::effect::Effect) -> crate::effect::Effect {
        if let Some(for_players) =
            effect.downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
        {
            let mut for_players = for_players.clone();
            for_players.effects = for_players
                .effects
                .iter()
                .map(bind_in_player_action)
                .map(|effect| rewrite(&effect))
                .collect();
            return crate::effect::Effect::new(for_players);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = sequence.effects.iter().map(rewrite).collect();
            return crate::effect::Effect::new(sequence);
        }
        if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
            let mut conditional = conditional.clone();
            conditional.if_true = conditional.if_true.iter().map(rewrite).collect();
            conditional.if_false = conditional.if_false.iter().map(rewrite).collect();
            return crate::effect::Effect::new(conditional);
        }
        if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
            let mut if_effect = if_effect.clone();
            if_effect.then = if_effect.then.iter().map(rewrite).collect();
            if_effect.else_ = if_effect.else_.iter().map(rewrite).collect();
            return crate::effect::Effect::new(if_effect);
        }
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        {
            let mut may = may.clone();
            may.effects = may.effects.iter().map(rewrite).collect();
            return crate::effect::Effect::new(may);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            tagged.effect = Box::new(rewrite(&tagged.effect));
            return crate::effect::Effect::new(tagged);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(rewrite(&with_id.effect));
            return crate::effect::Effect::new(with_id);
        }
        if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        {
            let mut with_source = with_source.clone();
            with_source.effect = Box::new(rewrite(&with_source.effect));
            return crate::effect::Effect::new(with_source);
        }
        effect.clone()
    }

    for segment in &mut program.segments {
        segment.default_effects = segment.default_effects.iter().map(rewrite).collect();
        for branch in &mut segment.self_replacements {
            branch.replacement_effects = branch.replacement_effects.iter().map(rewrite).collect();
        }
    }
    *program = crate::resolution::ResolutionProgram::new(std::mem::take(&mut program.segments));
}

#[cfg(test)]
mod quantified_player_damage_value_tests {
    use super::*;

    fn program_with_damage_target(target: crate::target::PlayerFilter) -> ResolutionProgram {
        let amount =
            crate::effect::Value::HalfRoundedDown(Box::new(crate::effect::Value::LifeTotal(
                crate::target::PlayerFilter::Target(Box::new(crate::target::PlayerFilter::Any)),
            )));
        ResolutionProgram::from_effects(vec![crate::effect::Effect::new(
            crate::effects::ForPlayersEffect {
                filter: crate::target::PlayerFilter::Any,
                effects: vec![crate::effect::Effect::new(
                    crate::effects::DealDamageEffect::new(
                        amount,
                        crate::target::ChooseSpec::Player(target),
                    ),
                )],
                starting_with_controller: false,
                sequential: false,
                stop_after_first_happened: false,
            },
        )])
    }

    fn life_total_player(program: &ResolutionProgram) -> &crate::target::PlayerFilter {
        let for_players = program.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
            .expect("quantified player effect");
        let damage = for_players.effects[0]
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .expect("damage action");
        let crate::effect::Value::HalfRoundedDown(inner) = &damage.amount else {
            panic!("expected half-rounded-down amount: {damage:#?}");
        };
        let crate::effect::Value::LifeTotal(player) = inner.as_ref() else {
            panic!("expected life-total basis: {damage:#?}");
        };
        player
    }

    #[test]
    fn binds_that_players_life_total_to_the_quantified_damage_recipient() {
        let mut program = program_with_damage_target(crate::target::PlayerFilter::IteratedPlayer);
        bind_quantified_player_damage_values(&mut program);
        assert_eq!(
            life_total_player(&program),
            &crate::target::PlayerFilter::IteratedPlayer
        );
    }

    #[test]
    fn does_not_bind_a_value_when_damage_has_a_different_recipient() {
        let mut program = program_with_damage_target(crate::target::PlayerFilter::You);
        bind_quantified_player_damage_values(&mut program);
        assert_eq!(
            life_total_player(&program),
            &crate::target::PlayerFilter::Target(Box::new(crate::target::PlayerFilter::Any))
        );
    }
}

#[cfg(test)]
mod alternative_ability_reference_tests {
    use super::*;
    fn condition(label: &str) -> crate::effect::Condition {
        crate::effect::Condition::ThisSpellPaidLabel(label.into())
    }
    fn body(label: &str) -> crate::effect::Effect {
        crate::effect::Effect::conditional(condition(label), vec![], vec![])
    }
    fn seed() -> CardDefinitionBuilder {
        let mut builder = CardDefinitionBuilder::seed();
        builder.alternative_casts.push(crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
            "Sneak", Some(crate::mana::ManaCost::new()), vec![]));
        builder
    }
    fn linked(condition: &crate::effect::Condition) -> bool {
        matches!(condition, crate::effect::Condition::ThisSpellPaidLabel(reference)
            if matches!(&reference.kind, ironsmith_core::OptionalCostKind::AlternativeCast(method)
                if method.method_name() == "Sneak"))
    }
    #[test]
    fn header_body_and_top_level_replacement_link_without_cross_ability_antecedents() {
        let mut builder = seed();
        let mut first = crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(), vec![body("that")]);
        let crate::ability::AbilityKind::Triggered(triggered) = &mut first.kind else { unreachable!() };
        triggered.intervening_if = Some(crate::effect::Condition::ThisSpellPaidLabel(
            ironsmith_core::OptionalCostRef::from("Sneak").this_turn()));
        builder.abilities.push(first);
        builder.abilities.push(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(), vec![body("that")]));
        builder.spell_effect = Some(crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment {
                default_effects: vec![body("Prowl")],
                self_replacements: vec![crate::resolution::SelfReplacementBranch::new(condition("Sneak"), vec![body("that")])],
                starts_new_source_line: true,
            },
        ]));
        link_alternative_cast_condition_references(&mut builder);
        for (index, expected) in [true, false].into_iter().enumerate() {
            let crate::ability::AbilityKind::Triggered(triggered) = &builder.abilities[index].kind else { unreachable!() };
            let conditional = triggered.effects.segments[0].default_effects[0].downcast_ref::<crate::effects::ConditionalEffect>().unwrap();
            assert_eq!(linked(&conditional.condition), expected);
            if index == 0 {
                let header = triggered.intervening_if.as_ref().unwrap();
                assert!(linked(header));
                let crate::effect::Condition::ThisSpellPaidLabel(reference) = header else { unreachable!() };
                assert!(reference.requires_current_turn());
            }
        }
        let segment = &builder.spell_effect.as_ref().unwrap().segments[0];
        assert!(segment.starts_new_source_line);
        assert_eq!(segment.default_effects.len(), 1);
        assert!(linked(&segment.self_replacements[0].condition));
        let nested = segment.self_replacements[0].replacement_effects[0].downcast_ref::<crate::effects::ConditionalEffect>().unwrap();
        assert!(linked(&nested.condition));
    }
}
