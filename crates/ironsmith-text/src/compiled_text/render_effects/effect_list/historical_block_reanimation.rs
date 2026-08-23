use super::*;

fn exact_outcome_tag(effect: &Effect) -> Option<(&crate::TagKey, &Effect)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some((&tagged.tag, &tagged.effect));
    }
    // Compiler-model `TagAffected` lowers through the core effect model,
    // whose aggregate result tag converts to runtime `TaggedEffect`. The
    // linked ForEachTagged tag remains the semantic proof that this is the
    // complete successful destroy result set; accept both runtime wrapper
    // encodings without deriving provenance from tag spelling.
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| (&tagged.tag, tagged.effect.as_ref()))
}

/// Restore the compact historical-block wording only for the exact executable
/// program: one tagged blocker target, a successful no-regeneration destroy
/// set keyed to that blocker, and a per-destroyed-object reanimation whose
/// graveyard owner is bound from the latest matching block event.
pub(in crate::compiled_text) fn describe_historical_block_reanimation(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, destroy_effect, followup_effect] = effects else {
        return None;
    };

    let (blocker_tag, target_effect) = exact_outcome_tag(target_effect)?;
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target.explicit_declaration || target.chooser.is_some() {
        return None;
    }
    let blocker_target = describe_choose_spec(&target.target);
    let blocker_reference = blocker_target.strip_prefix("target ")?;
    let blocker_filter = match target.target.unhinted() {
        ChooseSpec::Target(inner) => match inner.unhinted() {
            ChooseSpec::Object(filter) => filter,
            _ => return None,
        },
        _ => return None,
    };
    let explicit_creature_filter = blocker_filter.zone == Some(Zone::Battlefield)
        && blocker_filter.card_types.as_slice() == [CardType::Creature];
    let subtype_only_creature_filter =
        blocker_filter.zone.is_none() && blocker_filter.card_types.is_empty();
    let mut blocker_base = blocker_filter.clone();
    blocker_base.zone = None;
    blocker_base.card_types.clear();
    let creature_subtype = matches!(
        blocker_base.subtypes.as_slice(),
        [subtype] if subtype.is_creature_type()
    );
    blocker_base.subtypes.clear();
    if !explicit_creature_filter && !subtype_only_creature_filter {
        return None;
    }
    if !creature_subtype || blocker_base != ObjectFilter::default() {
        return None;
    }

    let (destroyed_tag, destroy_effect) = exact_outcome_tag(destroy_effect)?;
    let destroy = destroy_effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()?;
    let ChooseSpec::All(destroyed_filter) = destroy.spec.unhinted() else {
        return None;
    };
    let mut expected_destroyed = ObjectFilter::creature();
    expected_destroyed.blocked_by = Some(crate::filter::ObjectRef::Tagged(blocker_tag.clone()));
    if destroyed_filter != &expected_destroyed {
        return None;
    }

    let followup = structural_unwrap_render_wrappers(followup_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if followup.tag != *destroyed_tag
        || followup.controller_at_last_blocked_by.as_ref() != Some(blocker_tag)
    {
        return None;
    }
    let [move_effect] = followup.effects.as_slice() else {
        return None;
    };
    let move_effect = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let mut creature_card = ObjectFilter::creature();
    creature_card.zone = Some(Zone::Graveyard);
    creature_card.owner = Some(PlayerFilter::IteratedPlayer);
    creature_card.set_explicit_card_noun(true);
    let expected_move = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Object(creature_card).with_count(ChoiceCount::exactly(1)),
        Zone::Battlefield,
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
    .under_owner_control();
    if move_effect != &expected_move {
        return None;
    }

    Some(format!(
        "Destroy all creatures that were blocked by {blocker_target} this turn. They can't be regenerated. For each creature that died this way, put a creature card from the graveyard of the player who controlled that creature the last time it became blocked by that {blocker_reference} onto the battlefield under its owner's control"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Subtype;

    fn program(historical_controller: bool) -> Vec<Effect> {
        let blocker_tag = crate::TagKey::from("historical_blocker");
        let destroyed_tag = crate::TagKey::from("destroyed");
        let target = Effect::new(crate::effects::TargetOnlyEffect::explicit(
            ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::creature().with_subtype(Subtype::Wall),
            )),
        ))
        .tag_all(blocker_tag.clone());

        let mut destroyed = ObjectFilter::creature();
        destroyed.blocked_by = Some(crate::filter::ObjectRef::Tagged(blocker_tag.clone()));
        let destroy = Effect::new(crate::effects::DestroyNoRegenerationEffect::all(destroyed))
            .tag_all(destroyed_tag.clone());

        let mut creature_card = ObjectFilter::creature();
        creature_card.zone = Some(Zone::Graveyard);
        creature_card.owner = Some(PlayerFilter::IteratedPlayer);
        creature_card.set_explicit_card_noun(true);
        let move_one = Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Object(creature_card).with_count(ChoiceCount::exactly(1)),
                Zone::Battlefield,
                false,
            )
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
            .under_owner_control(),
        );
        let followup = crate::effects::ForEachTaggedEffect::new(destroyed_tag, vec![move_one]);
        let followup = if historical_controller {
            followup.with_controller_at_last_blocked_by(blocker_tag)
        } else {
            followup
        };
        let followup = Effect::new(followup);
        vec![target, destroy, followup]
    }

    #[test]
    fn renders_exact_historical_controller_surface() {
        assert_eq!(
            describe_historical_block_reanimation(&program(true)).as_deref(),
            Some(
                "Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died this way, put a creature card from the graveyard of the player who controlled that creature the last time it became blocked by that Wall onto the battlefield under its owner's control"
            )
        );
    }

    #[test]
    fn rejects_current_controller_and_untagged_destroy_near_misses() {
        let current_controller = program(false);
        assert!(describe_historical_block_reanimation(&current_controller).is_none());

        let mut untagged_destroy = program(true);
        untagged_destroy[1] = Effect::new(crate::effects::DestroyNoRegenerationEffect::all(
            ObjectFilter::creature(),
        ));
        assert!(describe_historical_block_reanimation(&untagged_destroy).is_none());
    }

    #[test]
    fn accepts_compiler_model_tagged_result_wrappers() {
        let mut effects = program(true);
        let target = effects[0]
            .downcast_ref::<crate::effects::TagAllEffect>()
            .expect("target aggregate tag")
            .clone();
        effects[0] = target.effect.tag(target.tag);
        let destroy = effects[1]
            .downcast_ref::<crate::effects::TagAllEffect>()
            .expect("destroy aggregate tag")
            .clone();
        effects[1] = destroy.effect.tag(destroy.tag);
        effects[2] = Effect::with_id(41, effects[2].clone());

        assert_eq!(
            describe_historical_block_reanimation(&effects).as_deref(),
            Some(
                "Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died this way, put a creature card from the graveyard of the player who controlled that creature the last time it became blocked by that Wall onto the battlefield under its owner's control"
            )
        );
    }
}
