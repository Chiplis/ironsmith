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
        TriggerSpec::PutIntoGraveyardFromZone { filter, .. } => {
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
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        replaced |= replace_filter_tag(targets, old_tag, new_tag);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        replaced |= replace_filter_tag(targets, old_tag, new_tag);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        replaced |= replace_filter_tag(attached_to, old_tag, new_tag);
    }
    for branch in &mut filter.any_of {
        replaced |= replace_filter_tag(branch, old_tag, new_tag);
    }
    replaced
}

pub(super) fn rewrite_normalize_additional_cost_sacrifice_tags(
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

    builder
        .spell_effect
        .get_or_insert_with(ResolutionProgram::default)
        .push(crate::effect::Effect::cipher());
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

pub(super) fn rewrite_finalize_lowered_card(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    builder = rewrite_apply_pending_mechanic_linkages(builder, state);
    builder = rewrite_apply_pending_backup_abilities(builder, state);
    rewrite_apply_pending_cipher_effect(builder, state)
}
