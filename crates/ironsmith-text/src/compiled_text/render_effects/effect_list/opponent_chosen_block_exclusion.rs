use super::*;

pub(super) fn describe_opponent_chosen_block_exclusion(effects: &[Effect]) -> Option<String> {
    let [target_effect, choice_effect, restriction_effect] = effects else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let choice = structural_unwrap_render_wrappers(choice_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let restriction = structural_unwrap_render_wrappers(restriction_effect)
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Block(restricted_filter) = &restriction.restriction else {
        return None;
    };

    let choice_filter = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    let mut expected_restricted_filter = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::target_opponent());
    expected_restricted_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: choice.tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
        });

    if target.target != ChooseSpec::target_opponent()
        || target.chooser.is_some()
        || target.explicit_declaration
        || choice.filter != choice_filter
        || !choice.count.is_single()
        || choice.count_value.is_some()
        || choice.aggregate_constraint.is_some()
        || choice.chooser != PlayerFilter::target_opponent()
        || choice.zone != Some(Zone::Battlefield)
        || !choice.additional_zones.is_empty()
        || choice.is_search
        || choice.reveal
        || choice.search_mode != crate::effect::SearchSelectionMode::Exact
        || choice.search_reveal_reference_surface.is_some()
        || choice.search_result_reference_surface.is_some()
        || choice.search_top_in_any_order_surface.is_some()
        || choice.top_only
        || choice.bottom_only
        || choice.replace_tagged_objects
        || choice.remember_as_chosen_object
        || restricted_filter != &expected_restricted_filter
        || restriction.duration != Until::EndOfTurn
        || restriction.start != crate::effect::RestrictionStart::Immediate
        || restriction.duration_surface != crate::effect::RestrictionDurationSurface::Default
    {
        return None;
    }

    Some(
        "Target opponent chooses a creature they control. Other creatures they control can't block this turn"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "Target opponent chooses a creature they control. Other creatures they control can't block this turn.";

    fn parsed_effects() -> (crate::CardDefinition, Vec<Effect>) {
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goblin War Cry")
                .card_types(vec![CardType::Sorcery])
                .parse_text(LINE)
                .expect("opponent choice and block exclusion should parse");
        let effects = definition
            .spell_effect
            .as_ref()
            .expect("spell program")
            .segments[0]
            .default_effects
            .clone();
        (definition, effects)
    }

    #[test]
    fn public_route_keeps_the_chosen_creature_exclusion() {
        let (definition, effects) = parsed_effects();
        assert_eq!(
            describe_opponent_chosen_block_exclusion(&effects).as_deref(),
            Some(LINE.trim_end_matches('.')),
        );
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [LINE]
        );
    }

    #[test]
    fn changed_exclusion_tag_is_not_compacted() {
        let (_, mut effects) = parsed_effects();
        let mut restriction = effects[2]
            .downcast_ref::<crate::effects::CantEffect>()
            .expect("block restriction")
            .clone();
        let crate::effect::Restriction::Block(filter) = &mut restriction.restriction else {
            unreachable!();
        };
        filter.tagged_constraints[0].tag = crate::TagKey::from("different_choice");
        effects[2] = Effect::new(restriction);

        assert!(describe_opponent_chosen_block_exclusion(&effects).is_none());
    }
}
