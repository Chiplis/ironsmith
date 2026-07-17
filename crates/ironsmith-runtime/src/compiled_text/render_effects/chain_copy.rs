use super::*;

fn target_noun_from_spec(spec: &ChooseSpec) -> Option<&'static str> {
    let ChooseSpec::Object(filter) = spec.base() else {
        return None;
    };

    let allowed_type_count = filter
        .card_types
        .iter()
        .filter(|card_type| !filter.excluded_card_types.contains(card_type))
        .count();
    if allowed_type_count == 1 {
        for (card_type, noun) in [
            (CardType::Creature, "creature"),
            (CardType::Land, "land"),
            (CardType::Artifact, "artifact"),
            (CardType::Enchantment, "enchantment"),
        ] {
            if filter.card_types.contains(&card_type)
                && !filter.excluded_card_types.contains(&card_type)
            {
                return Some(noun);
            }
        }
    }

    (filter.zone == Some(Zone::Battlefield)).then_some("permanent")
}

fn tagged_target_noun(effect: &Effect, tag: &crate::TagKey) -> Option<&'static str> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && &tagged.tag == tag
    {
        return first_target_noun_in_effect(&tagged.effect);
    }

    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = tagged_target_noun(child, tag);
        }
    });
    found
}

fn first_target_noun_in_effect(effect: &Effect) -> Option<&'static str> {
    if let Some(spec) = effect.0.get_target_spec()
        && let Some(noun) = target_noun_from_spec(spec)
    {
        return Some(noun);
    }

    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = first_target_noun_in_effect(child);
        }
    });
    found
}

fn first_target_noun(effects: &[Effect]) -> Option<&'static str> {
    effects.iter().find_map(first_target_noun_in_effect)
}

fn chain_actor_introduction(decider: &PlayerFilter, antecedent: &[Effect]) -> Option<String> {
    match decider {
        PlayerFilter::You => Some("you".to_string()),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            Some("that player or that permanent's controller".to_string())
        }
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
        | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag)) => {
            let noun = antecedent
                .iter()
                .find_map(|effect| tagged_target_noun(effect, tag))
                .or_else(|| first_target_noun(antecedent))
                .unwrap_or("permanent");
            Some(format!("that {noun}'s controller"))
        }
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
        | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target) => {
            let noun = first_target_noun(antecedent).unwrap_or("permanent");
            Some(format!("that {noun}'s controller"))
        }
        _ => None,
    }
}

fn may_action_tail(may_effect: &Effect) -> Option<String> {
    let rendered = describe_effect(may_effect);
    let rendered = rendered.trim().trim_end_matches('.');
    let may_index = rendered.to_ascii_lowercase().find(" may ")?;
    Some(rendered[may_index + " may ".len()..].to_string())
}

fn same_chain_actor(left: &PlayerFilter, right: &PlayerFilter) -> bool {
    if left == right {
        return true;
    }

    matches!(
        (left, right),
        (
            PlayerFilter::ControllerOf(left_ref),
            PlayerFilter::AliasedControllerOf(right_ref),
        ) | (
            PlayerFilter::AliasedControllerOf(left_ref),
            PlayerFilter::ControllerOf(right_ref),
        ) if left_ref == right_ref
    )
}

fn tagged_selection_is_controlled_by(
    effect: &Effect,
    tag: &crate::TagKey,
    actor: &PlayerFilter,
) -> bool {
    if let Some(choice) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && &choice.tag == tag
        && same_chain_actor(&choice.chooser, actor)
        && choice
            .filter
            .controller
            .as_ref()
            .is_some_and(|controller| same_chain_actor(controller, actor))
    {
        return true;
    }

    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found {
            found = tagged_selection_is_controlled_by(child, tag, actor);
        }
    });
    found
}

fn chain_actor_matches_enabling_action(
    actor: &PlayerFilter,
    enabling_actor: &PlayerFilter,
    enabling_effects: &[Effect],
) -> bool {
    if same_chain_actor(actor, enabling_actor) {
        return true;
    }

    let tag = match actor {
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
        | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag)) => tag,
        _ => return false,
    };
    enabling_effects
        .iter()
        .any(|effect| tagged_selection_is_controlled_by(effect, tag, enabling_actor))
}

/// Render the generic result-gated chain-copy composition as one oracle-shaped
/// continuation. The executable structure remains ordinary `May`, `WithId`,
/// `If`, `CopySpell`, and `ChooseNewTargets` effects; this recognizer only
/// preserves their shared actor and sentence surface.
pub(crate) fn describe_chain_copy_effect_list(effects: &[Effect]) -> Option<String> {
    if effects.len() < 3 {
        return None;
    }
    let (antecedent, continuation) = effects.split_at(effects.len() - 2);
    let [enabling_effect, conditional_effect] = continuation else {
        return None;
    };

    let enabling_with_id = enabling_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let enabling_may = enabling_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let enabling_decider = enabling_may.decider.as_ref()?;

    let conditional = conditional_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != enabling_with_id.id
        || conditional.predicate != EffectPredicate::Happened
        || !conditional.else_.is_empty()
    {
        return None;
    }
    let [copy_may_effect] = conditional.then.as_slice() else {
        return None;
    };
    let copy_may = copy_may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let copy_decider = copy_may.decider.as_ref()?;
    let [copy_effect, retarget_effect] = copy_may.effects.as_slice() else {
        return None;
    };

    let tagged_copy = copy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let copy_with_id = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy = copy_with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    let retarget = retarget_effect.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if !matches!(copy.target.unhinted(), ChooseSpec::Source)
        || copy.count != Value::Fixed(1)
        || !copy.removed_supertypes.is_empty()
        || !chain_actor_matches_enabling_action(
            copy_decider,
            enabling_decider,
            &enabling_may.effects,
        )
        || !same_chain_actor(&copy.copier, copy_decider)
        || retarget.from_effect != copy_with_id.id
        || !retarget.may
        || !retarget
            .chooser
            .as_ref()
            .is_some_and(|chooser| same_chain_actor(chooser, copy_decider))
    {
        return None;
    }

    let actor = chain_actor_introduction(enabling_decider, antecedent)?;
    let action = may_action_tail(&enabling_with_id.effect)?;
    let antecedent_text = describe_effect_list(antecedent)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if antecedent_text.is_empty() {
        return None;
    }

    let (condition_actor, copy_actor) = if matches!(enabling_decider, PlayerFilter::You) {
        ("you", "you")
    } else {
        ("the player", "they")
    };
    let retarget_text = if retarget.single_target_surface {
        "a new target"
    } else {
        "new targets"
    };

    Some(format!(
        "{antecedent_text}. Then {actor} may {action}. If {condition_actor} does, {copy_actor} may copy this spell and may choose {retarget_text} for that copy"
    ))
}
