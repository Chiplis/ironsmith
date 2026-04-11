use super::*;

pub(super) fn infer_static_ability_functional_zones(normalized_line: &str) -> Option<Vec<Zone>> {
    let mut zones = Vec::new();
    for (needles, zone) in [
        (
            &[
                "this card is in your hand",
                "there is this card in your hand",
            ][..],
            Zone::Hand,
        ),
        (
            &[
                "this card is in your graveyard",
                "there is this card in your graveyard",
            ][..],
            Zone::Graveyard,
        ),
        (
            &[
                "this card is in your library",
                "there is this card in your library",
            ][..],
            Zone::Library,
        ),
        (
            &["this card is in exile", "there is this card in exile"][..],
            Zone::Exile,
        ),
        (
            &[
                "this card is in the command zone",
                "there is this card in the command zone",
            ][..],
            Zone::Command,
        ),
    ] {
        if needles
            .iter()
            .any(|needle| str_contains(normalized_line, needle))
        {
            zones.push(zone);
        }
    }
    if zones.is_empty() { None } else { Some(zones) }
}

pub(super) fn infer_triggered_ability_functional_zones(
    trigger: &TriggerSpec,
    normalized_line: &str,
) -> Vec<Zone> {
    let mut zones = match trigger {
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            ..
        } => vec![Zone::Graveyard],
        _ => vec![Zone::Battlefield],
    };

    let normalized = normalized_line.to_ascii_lowercase();
    for (needle, zone) in [
        ("if this card is in your hand", Zone::Hand),
        ("if this card is in your graveyard", Zone::Graveyard),
        ("if this card is in your library", Zone::Library),
        ("if this card is in exile", Zone::Exile),
        ("if this card is in the command zone", Zone::Command),
    ] {
        if str_contains(normalized.as_str(), needle) {
            zones = vec![zone];
            break;
        }
    }
    if str_contains(normalized.as_str(), "return this card from your graveyard") {
        zones = vec![Zone::Graveyard];
    }
    zones
}

fn filter_references_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tag)
        || filter
            .targets_object
            .as_deref()
            .is_some_and(|targets| filter_references_tag(targets, tag))
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(|targets| filter_references_tag(targets, tag))
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
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        replaced |= replace_filter_tag(targets, old_tag, new_tag);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        replaced |= replace_filter_tag(targets, old_tag, new_tag);
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
            EffectAst::Sacrifice { filter, .. } | EffectAst::SacrificeAll { filter, .. }
                if filter_references_tag(filter, IT_TAG) =>
            {
                replaced |= replace_filter_tag(filter, IT_TAG, &sacrificed_tag);
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
            crate::costs::Cost::try_from_runtime_effect(effect).map_err(CardTextError::ParseError)
        })
        .collect()
}

pub(super) fn rewrite_apply_pending_mechanic_linkages(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    let Some((haunt_effects, haunt_choices)) = state.haunt_linkage.take() else {
        return builder;
    };

    for ability in &mut builder.abilities {
        if ability.text.as_deref() == Some("Haunt")
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

pub(super) fn rewrite_normalize_spell_delayed_trigger_effects(
    mut builder: CardDefinitionBuilder,
) -> CardDefinitionBuilder {
    let is_spell = builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery));
    if !is_spell {
        return builder;
    }

    let mut delayed = Vec::new();
    builder.abilities.retain(|ability| {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            return true;
        };
        let ability_text = ability
            .text
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !str_contains(ability_text.as_str(), "this turn") {
            return true;
        }

        delayed.push(crate::effect::Effect::new(
            crate::effects::ScheduleDelayedTriggerEffect::new(
                triggered.trigger.clone(),
                triggered.effects.clone(),
                false,
                Vec::new(),
                PlayerFilter::You,
            )
            .until_end_of_turn(),
        ));
        false
    });

    if delayed.is_empty() {
        return builder;
    }

    builder
        .spell_effect
        .get_or_insert_with(crate::resolution::ResolutionProgram::default)
        .extend(crate::resolution::ResolutionProgram::from_effects(delayed));
    builder
}

pub(super) fn rewrite_normalize_take_to_the_streets_spell_effect(
    mut builder: CardDefinitionBuilder,
) -> CardDefinitionBuilder {
    use crate::continuous::Modification;
    use crate::effect::Value;
    use crate::effects::continuous::RuntimeModification;
    use crate::static_abilities::StaticAbilityId;
    use crate::types::Subtype;

    let Some(effects) = builder.spell_effect.as_ref() else {
        return builder;
    };
    if effects.segments.len() != 1 || effects.segments[0].default_effects.len() != 2 {
        return builder;
    }

    let Some(apply) = effects.segments[0].default_effects[1]
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    else {
        return builder;
    };
    if apply.until != crate::effect::Until::EndOfTurn {
        return builder;
    }
    let filter = match &apply.target {
        crate::continuous::EffectTarget::Filter(filter) => filter,
        _ => return builder,
    };
    if filter.controller != Some(PlayerFilter::You)
        || !slice_contains(filter.subtypes.as_slice(), &Subtype::Citizen)
    {
        return builder;
    }
    let is_vigilance = apply.modification.as_ref().is_some_and(|m| match m {
        Modification::AddAbility(ability) => ability.id() == StaticAbilityId::Vigilance,
        _ => false,
    });
    if !is_vigilance {
        return builder;
    }
    if apply
        .runtime_modifications
        .iter()
        .any(|m| matches!(m, RuntimeModification::ModifyPowerToughness { .. }))
    {
        return builder;
    }

    let mut updated = apply.clone();
    updated
        .runtime_modifications
        .push(RuntimeModification::ModifyPowerToughness {
            power: Value::Fixed(1),
            toughness: Value::Fixed(1),
        });

    let mut new_effects = effects.clone();
    new_effects.segments[0].default_effects[1] = crate::effect::Effect::new(updated);
    builder.spell_effect = Some(new_effects);
    builder
}

pub(super) fn rewrite_finalize_lowered_card(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    builder = rewrite_normalize_spell_delayed_trigger_effects(builder);
    builder = rewrite_normalize_take_to_the_streets_spell_effect(builder);
    rewrite_apply_pending_mechanic_linkages(builder, state)
}
