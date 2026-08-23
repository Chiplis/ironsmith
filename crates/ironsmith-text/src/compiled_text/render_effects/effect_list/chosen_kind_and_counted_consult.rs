use super::*;

fn is_exact_card_type_filter(filter: &ObjectFilter, card_type: CardType) -> bool {
    if filter.zone.is_some() || filter.card_types.as_slice() != [card_type] {
        return false;
    }
    let mut normalized = filter.clone();
    normalized.card_types.clear();
    normalized == ObjectFilter::default()
}

fn is_exact_nonland_filter(filter: &ObjectFilter) -> bool {
    filter == &ObjectFilter::nonland()
}

fn consult_put_match_in_hand_rest_bottom_branch(
    effects: &[Effect],
    expected_filter: impl FnOnce(&ObjectFilter) -> bool,
) -> Option<()> {
    let [consult_effect, move_effect, bottom_effect] = effects else {
        return None;
    };
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.stop_rule != crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        || consult.max_exposed.is_some()
        || !expected_filter(&consult.filter)
    {
        return None;
    }

    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let expected_move = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(consult.match_tag.clone()),
        Zone::Hand,
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put);
    if move_to_zone != &expected_move {
        return None;
    }

    let bottom = structural_unwrap_render_wrappers(bottom_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let expected_bottom = crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
        consult.all_tag.clone(),
        Some(consult.match_tag.clone()),
        crate::effects::consult_helpers::LibraryBottomOrder::Random,
        PlayerFilter::You,
    );
    (bottom == &expected_bottom).then_some(())
}

/// Collapse a binary land/nonland choice whose two typed branches implement
/// the same exact consult partition for complementary card kinds. The choice,
/// chosen-option condition, and producer/consumer tags prove the authored
/// "chosen kind" reference without relying on a card name or tag spelling.
pub(super) fn describe_land_or_nonland_chosen_kind_consult(effects: &[Effect]) -> Option<String> {
    let [choose_effect, conditional_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseNamedOptionEffect>()?;
    let options = choose
        .options
        .iter()
        .map(|option| option.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if choose.chooser != PlayerFilter::You || options != ["land", "nonland"] {
        return None;
    }

    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf {
        return None;
    }
    let crate::effect::Condition::SourceChosenOption(chosen_option) = &conditional.condition else {
        return None;
    };

    let (land_branch, nonland_branch) = if chosen_option.eq_ignore_ascii_case("land") {
        (&conditional.if_true, &conditional.if_false)
    } else if chosen_option.eq_ignore_ascii_case("nonland") {
        (&conditional.if_false, &conditional.if_true)
    } else {
        return None;
    };
    consult_put_match_in_hand_rest_bottom_branch(land_branch, |filter| {
        is_exact_card_type_filter(filter, CardType::Land)
    })?;
    consult_put_match_in_hand_rest_bottom_branch(nonland_branch, is_exact_nonland_filter)?;

    Some(
        "Choose land or nonland. Reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and the rest on the bottom of your library in a random order"
            .to_string(),
    )
}

fn exact_consult_match_collection(
    target: &ChooseSpec,
    consult: &crate::effects::ConsultTopOfLibraryEffect,
) -> bool {
    if matches!(target.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag) {
        return true;
    }
    let ChooseSpec::All(filter) = target.base() else {
        return false;
    };
    if filter.zone != Some(Zone::Battlefield)
        || filter.tagged_constraints.as_slice()
            != [crate::filter::TaggedObjectConstraint {
                tag: consult.match_tag.clone(),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            }]
        || !filter.union_surface.explicit_card_noun()
        || filter.union_surface.prior_effect_action()
            != Some(crate::effect::PriorEffectAction::Revealed)
    {
        return false;
    }

    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized.tagged_constraints.clear();
    normalized == consult.filter
}

/// Render a variable-count reveal consult followed by moving exactly the
/// matched collection to your graveyard and the exact complement to the
/// library bottom. This accepts both direct tagged targets and the authored
/// `all ... revealed this way` filter surface, but only when the consult tags
/// prove both consumers cover the same partition.
pub(super) fn describe_counted_consult_matches_to_graveyard_then_bottom(
    consult_effect: &Effect,
    move_effect: &Effect,
    bottom_effect: &Effect,
) -> Option<String> {
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) = &consult.stop_rule else {
        return None;
    };
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.max_exposed.is_some()
        || matches!(count.unhinted(), Value::Fixed(1))
    {
        return None;
    }

    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !exact_consult_match_collection(&move_to_zone.target, consult) {
        return None;
    }
    let expected_move =
        crate::effects::MoveToZoneEffect::new(move_to_zone.target.clone(), Zone::Graveyard, false)
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
            .with_target_plural_surface()
            .with_destination_player_surface(PlayerFilter::You);
    if move_to_zone != &expected_move {
        return None;
    }

    let bottom = structural_unwrap_render_wrappers(bottom_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let expected_bottom = crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
        consult.all_tag.clone(),
        Some(consult.match_tag.clone()),
        crate::effects::consult_helpers::LibraryBottomOrder::Random,
        PlayerFilter::You,
    );
    if bottom != &expected_bottom {
        return None;
    }

    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let plural_selection = pluralize_noun_phrase(&selection);
    Some(format!(
        "Reveal cards from the top of your library until you reveal {} {plural_selection}. Put all {plural_selection} revealed this way into your graveyard, then put the rest on the bottom of your library in a random order",
        describe_value(count)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hand_bottom_branch(filter: ObjectFilter, tag_stem: &str) -> Vec<Effect> {
        let all_tag = TagKey::from(format!("{tag_stem}_all"));
        let match_tag = TagKey::from(format!("{tag_stem}_match"));
        vec![
            Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
                PlayerFilter::You,
                crate::effects::consult_helpers::LibraryConsultMode::Reveal,
                filter,
                crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
                all_tag.clone(),
                match_tag.clone(),
            )),
            Effect::new(
                crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Tagged(match_tag.clone()),
                    Zone::Hand,
                    false,
                )
                .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put),
            )
            .tag(format!("{tag_stem}_moved")),
            Effect::new(
                crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                    all_tag,
                    Some(match_tag),
                    crate::effects::consult_helpers::LibraryBottomOrder::Random,
                    PlayerFilter::You,
                ),
            ),
        ]
    }

    #[test]
    fn complementary_land_choice_renders_as_one_chosen_kind_consult() {
        let mut land = ObjectFilter::default();
        land.card_types.push(CardType::Land);
        let effects = vec![
            Effect::choose_named_option(
                PlayerFilter::You,
                vec!["land".to_string(), "nonland".to_string()],
            ),
            Effect::conditional(
                Condition::SourceChosenOption("land".to_string()),
                hand_bottom_branch(land, "land"),
                hand_bottom_branch(ObjectFilter::nonland(), "nonland"),
            ),
        ];

        assert_eq!(
            describe_effect_list(&effects),
            "Choose land or nonland. Reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and the rest on the bottom of your library in a random order"
        );
    }

    #[test]
    fn counted_reveal_uses_typed_collection_and_exact_remainder() {
        let all_tag = TagKey::from("revealed");
        let match_tag = TagKey::from("matched");
        let mut creature_card = ObjectFilter::default();
        creature_card.card_types.push(CardType::Creature);
        let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::You,
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            creature_card.clone(),
            crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::X),
            all_tag.clone(),
            match_tag.clone(),
        ));

        let mut moved_filter = ObjectFilter::creature();
        moved_filter.union_surface = moved_filter
            .union_surface
            .with_explicit_card_noun(true)
            .with_prior_effect_action(Some(crate::effect::PriorEffectAction::Revealed));
        moved_filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: match_tag.clone(),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let move_matches = Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::All(moved_filter),
                Zone::Graveyard,
                false,
            )
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
            .with_target_plural_surface()
            .with_destination_player_surface(PlayerFilter::You),
        );
        let bottom = Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                all_tag,
                Some(match_tag),
                crate::effects::consult_helpers::LibraryBottomOrder::Random,
                PlayerFilter::You,
            ),
        );

        let flattened = [consult, move_matches, bottom];
        assert_eq!(
            describe_cross_segment_consult_bundle(&flattened).as_deref(),
            Some(
                "Reveal cards from the top of your library until you reveal X creature cards. Put all creature cards revealed this way into your graveyard, then put the rest on the bottom of your library in a random order"
            )
        );
    }
}
