use super::*;

pub(crate) fn infer_triggered_ability_functional_zones_from_facts(
    trigger: &TriggerSpec,
    facts: &crate::runtime_backend::shared_types::TriggerFunctionalZoneFacts,
) -> Vec<Zone> {
    let mut zones = match trigger {
        TriggerSpec::WithIntro { trigger, .. } => {
            return infer_triggered_ability_functional_zones_from_facts(trigger, facts);
        }
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            ..
        } => vec![Zone::Graveyard],
        _ => vec![Zone::Battlefield],
    };

    if let Some(explicit_zone) = &facts.explicit_zone {
        zones = vec![explicit_zone.clone()];
    }
    if facts.returns_self_from_graveyard && !trigger_references_attached_object(trigger) {
        zones = vec![Zone::Graveyard];
    } else if facts.discards_this_card {
        zones = vec![Zone::Hand];
    }
    zones
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

pub(super) fn rewrite_normalize_selected_sacrifice_tags(
    mut effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    let Some((first, rest)) = effects.split_first_mut() else {
        return effects;
    };

    let choose_tag = match first {
        EffectAst::ChooseObjects { tag, .. } | EffectAst::ChooseObjectsAcrossZones { tag, .. }
            if tag.as_str() == IT_TAG =>
        {
            tag
        }
        _ => return effects,
    };

    let sacrificed_tag = TagKey::from("sacrificed_0");
    let mut replaced = false;
    for effect in rest {
        match effect {
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::Sacrifice { filter, .. }
                        if filter_references_tag(filter, IT_TAG)
                ) =>
            {
                if let SubjectVerbActionAst::Sacrifice { filter, .. } = &mut subject_verb.action {
                    replaced |= replace_filter_tag(filter, IT_TAG, &sacrificed_tag);
                }
            }
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::SacrificeAll { filter }
                        if filter_references_tag(filter, IT_TAG)
                ) =>
            {
                if let SubjectVerbActionAst::SacrificeAll { filter } = &mut subject_verb.action {
                    replaced |= replace_filter_tag(filter, IT_TAG, &sacrificed_tag);
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

pub(super) fn rewrite_apply_pending_mechanic_linkages(
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

fn rewrite_apply_pending_backup_abilities(
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

fn rewrite_apply_pending_cipher_effect(
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

fn rebind_looked_partition_source_tag(
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
                .map(|effect| rebind_looked_partition_source_tag(effect, &source_tag, &default_tag))
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

    fn rebind_membership(filter: &mut ObjectFilter, reveal_tag: &TagKey) -> bool {
        let [constraint] = filter.tagged_constraints.as_mut_slice() else {
            return false;
        };
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return false;
        }
        constraint.tag = reveal_tag.clone();
        true
    }

    fn rewrite_partition_sequence(
        effect: &crate::effect::Effect,
        reveal_tag: &TagKey,
    ) -> Option<crate::effect::Effect> {
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut rewritten = with_id.clone();
            rewritten.effect = Box::new(rewrite_partition_sequence(&with_id.effect, reveal_tag)?);
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
        if !rebind_membership(&mut union.filter.any_of[0], reveal_tag)
            || !rebind_membership(&mut union.filter.any_of[1], reveal_tag)
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
    let Some(rewritten) = rewrite_partition_sequence(root, reveal_tag) else {
        return;
    };
    disposition.default_effects = vec![rewritten];
}

pub(super) fn rewrite_finalize_lowered_card(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    builder = rewrite_apply_pending_mechanic_linkages(builder, state);
    builder = rewrite_apply_pending_backup_abilities(builder, state);
    builder = rewrite_apply_pending_cipher_effect(builder, state);
    if let Some(spell_effect) = &mut builder.spell_effect {
        normalize_ordered_revealed_partition_tags(spell_effect);
        preserve_looked_collection_self_replacement_preludes(spell_effect);
    }
    for ability in &mut builder.abilities {
        match &mut ability.kind {
            AbilityKind::Triggered(triggered) => {
                preserve_looked_collection_self_replacement_preludes(&mut triggered.effects);
            }
            AbilityKind::Activated(activated) => {
                preserve_looked_collection_self_replacement_preludes(&mut activated.effects);
            }
            _ => {}
        }
    }
    builder
}
