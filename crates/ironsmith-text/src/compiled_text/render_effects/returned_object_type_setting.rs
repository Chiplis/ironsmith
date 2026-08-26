use super::*;

fn unwrap_render_wrappers(effect: &Effect) -> &Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_render_wrappers(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_render_wrappers(&tag_all.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_render_wrappers(&with_id.effect);
    }
    effect
}

fn outer_object_tag(effect: &Effect) -> Option<&TagKey> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tag_all.tag);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return outer_object_tag(&with_id.effect);
    }
    None
}

fn choose_spec_is_exact_tag(spec: &ChooseSpec, expected: &TagKey) -> bool {
    matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == expected)
}

fn plain_permanent_type_change<'a>(
    effect: &'a Effect,
    expected_target: &TagKey,
) -> Option<&'a crate::effects::ApplyContinuousEffect> {
    let apply =
        unwrap_render_wrappers(effect).downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::Forever
        || apply.condition.is_some()
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || apply.source_type.is_some()
        || apply.type_retention_surface.is_some()
        || apply.lock_filter_at_resolution
        || apply.resolve_set_pt_values_at_resolution
        || apply.require_creature_target
        || !apply
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_is_exact_tag(spec, expected_target))
    {
        return None;
    }
    Some(apply)
}

/// Render the common mill/return/type-setting pipeline as the three Oracle
/// sentences represented by its object tags. The tags prove that the return
/// consumes exactly the milled collection and that the animation consumes
/// exactly the returned permanents.
pub(super) fn describe_milled_creatures_returned_then_animated(
    effects: &[Effect],
) -> Option<String> {
    let [mill_players_effect, return_effect, animate_effect] = effects else {
        return None;
    };

    let mill_players = unwrap_render_wrappers(mill_players_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if mill_players.filter != PlayerFilter::Opponent
        || mill_players.effects.len() != 1
        || mill_players.starting_with_controller
        || mill_players.stop_after_first_happened
    {
        return None;
    }
    let mill_tag = outer_object_tag(&mill_players.effects[0])?;
    let mill = unwrap_render_wrappers(&mill_players.effects[0])
        .downcast_ref::<crate::effects::MillEffect>()?;
    if mill.player != PlayerFilter::IteratedPlayer {
        return None;
    }

    let returned_tag = outer_object_tag(return_effect)?;
    let returned = unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    // The tagged constraint identifies the source collection. Lowering may
    // normalize a return effect's filter to its destination battlefield zone,
    // while older producers retain the source graveyard zone.
    if !matches!(
        returned.filter.zone,
        Some(Zone::Graveyard | Zone::Battlefield)
    ) || returned.filter.card_types.as_slice() != [CardType::Creature]
        || !returned.face_down
        || returned.battlefield_controller != crate::effects::BattlefieldController::You
        || !returned.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == *mill_tag
        })
    {
        return None;
    }

    let animation = unwrap_render_wrappers(animate_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if animation.until != Until::Forever
        || !animation
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_is_exact_tag(spec, returned_tag))
        || !matches!(
            &animation.modification,
            Some(crate::continuous::Modification::AddCardTypes(card_types))
                if card_types.contains(&CardType::Artifact)
                    && card_types.contains(&CardType::Creature)
        )
    {
        return None;
    }

    let sentence = |effect: &Effect| {
        describe_effect(effect)
            .trim()
            .trim_end_matches('.')
            .to_string()
    };
    Some(format!(
        "{}. {}. {}",
        sentence(mill_players_effect),
        sentence(return_effect),
        sentence(animate_effect)
    ))
}

/// Render a death-trigger return followed by an exact, permanent type reset.
///
/// The generated object tags are part of the proof here: the type-setting
/// effect must consume the object produced by the graveyard return, not the
/// old graveyard snapshot. A following creature-type removal is accepted only
/// when it consumes that same result and is therefore redundant with setting
/// the complete card-type list to `Enchantment`.
pub(in crate::compiled_text) fn describe_returned_object_set_to_enchantment(
    effects: &[Effect],
) -> Option<String> {
    let [
        tag_triggering,
        return_effect,
        set_type_effect,
        trailing @ ..,
    ] = effects
    else {
        return None;
    };
    if trailing.len() > 1 {
        return None;
    }

    let triggering_tag = &tag_triggering
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?
        .tag;
    let returned_tag = outer_object_tag(return_effect)?;
    let returned =
        unwrap_render_wrappers(return_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if returned.zone != Zone::Battlefield
        || returned.to_top
        || returned.library_order.is_some()
        || returned.destination_player_surface.is_some()
        || returned.destination_player_reference_surface.is_some()
        || returned.battlefield_controller != crate::effects::BattlefieldController::Owner
        || returned.enters_tapped
        || returned.enters_attacking
        || returned.attack_target_mode.is_some()
        || returned.enters_face_down
        || returned.transfer_exiled_with_source_links
        || !choose_spec_is_exact_tag(&returned.target, triggering_tag)
    {
        return None;
    }

    let set_type = plain_permanent_type_change(set_type_effect, returned_tag)?;
    if !matches!(
        &set_type.modification,
        Some(crate::continuous::Modification::SetCardTypes(card_types))
            if card_types.as_slice() == [CardType::Enchantment]
    ) {
        return None;
    }

    if let [remove_creature_effect] = trailing {
        let remove_creature = plain_permanent_type_change(remove_creature_effect, returned_tag)
            .or_else(|| {
                outer_object_tag(set_type_effect).and_then(|set_type_result_tag| {
                    plain_permanent_type_change(remove_creature_effect, set_type_result_tag)
                })
            })?;
        if !matches!(
            &remove_creature.modification,
            Some(crate::continuous::Modification::RemoveCardTypes(card_types))
                if card_types.as_slice() == [CardType::Creature]
        ) {
            return None;
        }
    }

    Some("Return it to the battlefield under its owner's control. It's an enchantment".to_string())
}

/// Render a returned object that is reset to exact card types, receives
/// subtypes, and gains one quoted ability. Exact object tags prove that all
/// three characteristic changes apply to the newly returned object.
pub(super) fn describe_returned_object_exact_types_with_quoted_ability(
    effects: &[Effect],
) -> Option<String> {
    let [
        return_effect,
        set_type_effect,
        add_subtype_effect,
        grant_effect,
    ] = effects
    else {
        return None;
    };

    let returned_tag = outer_object_tag(return_effect)?;
    let returned =
        unwrap_render_wrappers(return_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if returned.zone != Zone::Battlefield || returned.to_top || returned.enters_face_down {
        return None;
    }

    let set_type = plain_permanent_type_change(set_type_effect, returned_tag)?;
    let Some(crate::continuous::Modification::SetCardTypes(card_types)) = &set_type.modification
    else {
        return None;
    };
    if card_types.is_empty() {
        return None;
    }

    let add_subtype = plain_permanent_type_change(add_subtype_effect, returned_tag)?;
    let Some(crate::continuous::Modification::AddSubtypes(subtypes)) = &add_subtype.modification
    else {
        return None;
    };
    if subtypes.is_empty() {
        return None;
    }

    let grant = plain_permanent_type_change(grant_effect, returned_tag)?;
    let Some(crate::continuous::Modification::AddAbilityGeneric(ability)) = &grant.modification
    else {
        return None;
    };

    let mut descriptor = subtypes.iter().map(ToString::to_string).collect::<Vec<_>>();
    descriptor.extend(
        card_types
            .iter()
            .map(|card_type| describe_card_type_word_local(*card_type).to_string()),
    );
    let descriptor = with_indefinite_article(&descriptor.join(" "));
    let self_subject = if card_types.len() == 1 {
        format!("this {}", describe_card_type_word_local(card_types[0]))
    } else {
        "this permanent".to_string()
    };
    let ability = describe_inline_ability_with_self_subject(ability, &self_subject)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if ability.is_empty() {
        return None;
    }
    let returned = describe_effect(return_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();

    Some(format!(
        "{returned}. It's {descriptor} with \"{ability},\" and it loses all other card types"
    ))
}

/// Render a battlefield return followed by a permanent animation using the
/// producer's cardinality. The shared result tag proves that the animation
/// applies to exactly the object or collection returned by the first effect.
pub(super) fn describe_returned_battlefield_object_then_animated(
    effects: &[Effect],
) -> Option<String> {
    let [return_effect, animation_effect] = effects else {
        return None;
    };

    describe_returned_battlefield_object_then_animated_pair(return_effect, animation_effect)
}

pub(in crate::compiled_text) fn describe_returned_battlefield_object_then_animated_pair(
    return_effect: &Effect,
    animation_effect: &Effect,
) -> Option<String> {
    let returned_tag = outer_object_tag(return_effect)?;
    let returned = unwrap_render_wrappers(return_effect);
    let plural = if let Some(move_to_zone) =
        returned.downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        (move_to_zone.zone == Zone::Battlefield).then(|| {
            move_to_zone.target_plural_surface || choose_spec_allows_multiple(&move_to_zone.target)
        })?
    } else if let Some(return_from_graveyard) =
        returned.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        choose_spec_allows_multiple(&return_from_graveyard.target)
    } else {
        return None;
    };

    let animation = unwrap_render_wrappers(animation_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if animation.until != Until::Forever
        || !animation
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_is_exact_tag(spec, returned_tag))
    {
        return None;
    }
    let animation = describe_returned_object_animation_effect(animation, plural)?;
    let returned = describe_effect(return_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!("{returned}. {animation}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tagged_type_change(
        target: impl Into<TagKey>,
        modification: crate::continuous::Modification,
        result: impl Into<TagKey>,
    ) -> Effect {
        Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(target.into()),
            modification,
            Until::Forever,
        ))
        .tag(result)
    }

    fn enduring_return_program(first_type_change: crate::continuous::Modification) -> Vec<Effect> {
        let triggering = TagKey::from("triggering");
        let returned = TagKey::from("returned");
        vec![
            Effect::tag_triggering_object(triggering.clone()),
            Effect::new(
                crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Tagged(triggering),
                    Zone::Battlefield,
                    false,
                )
                .under_owner_control(),
            )
            .tag(returned.clone()),
            tagged_type_change(returned.clone(), first_type_change, "typed"),
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::Tagged(returned),
                crate::continuous::Modification::RemoveCardTypes(vec![CardType::Creature]),
                Until::Forever,
            )),
        ]
    }

    #[test]
    fn exact_enchantment_type_reset_renders_as_two_sentences() {
        let effects = enduring_return_program(crate::continuous::Modification::SetCardTypes(vec![
            CardType::Enchantment,
        ]));

        assert_eq!(
            describe_returned_object_set_to_enchantment(&effects).as_deref(),
            Some("Return it to the battlefield under its owner's control. It's an enchantment")
        );
        assert_eq!(
            describe_effect_list(&effects),
            "Return it to the battlefield under its owner's control. It's an enchantment"
        );
        assert_eq!(
            describe_effect_clause_list(&effects).as_deref(),
            Some("return it to the battlefield under its owner's control. It's an enchantment")
        );
    }

    #[test]
    fn additive_type_change_cannot_claim_an_exact_enchantment_reset() {
        let effects = enduring_return_program(crate::continuous::Modification::AddCardTypes(vec![
            CardType::Enchantment,
        ]));

        assert_eq!(describe_returned_object_set_to_enchantment(&effects), None);
    }

    #[test]
    fn enduring_cycle_public_route_keeps_the_exact_type_sentence() {
        for name in [
            "Enduring Courage",
            "Enduring Curiosity",
            "Enduring Innocence",
            "Enduring Tenacity",
            "Enduring Vitality",
        ] {
            let oracle = format!(
                "When {name} dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment."
            );
            let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                name,
            )
            .card_types(vec![CardType::Enchantment, CardType::Creature])
            .parse_text(&oracle)
            .expect("Enduring-cycle return should compile");

            assert_eq!(
                crate::compiled_text::compiled_text_lines(&definition),
                [oracle],
                "{name}"
            );
        }
    }
}
