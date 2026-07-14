use super::*;

/// Render a looked-card pool whose optional singleton selection is returned to
/// the top of the same library and whose complement goes to the bottom.
///
/// The matcher intentionally requires an unfiltered `up to one` selection from
/// the exact tagged pool. That is the structural proof for the compact Oracle
/// wording "up to one of them"; filtered or differently sized selections fall
/// through to their more explicit renderers.
pub(super) fn describe_looked_up_to_one_top_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    let (_, move_to_top) = for_each_tagged_for_compaction(move_effect)?;
    if look_at_top.player != choose.chooser
        || choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.top_only
        || choose.bottom_only
        || choose.count != ChoiceCount::up_to(1)
        || !for_each_moves_tag_to_library_top(move_to_top, choose.tag.as_str())
        || remainder.tag != look_at_top.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != look_at_top.player
    {
        return None;
    }

    let looked_constraints = choose
        .filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.tag == look_at_top.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        .count();
    if looked_constraints != 1 {
        return None;
    }
    let mut unfiltered_pool = choose.filter.clone();
    unfiltered_pool.zone = None;
    unfiltered_pool.tagged_constraints.retain(|constraint| {
        !(constraint.tag == look_at_top.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
    });
    if unfiltered_pool != ObjectFilter::default() {
        return None;
    }

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let actor = if choose.chooser == PlayerFilter::You {
        "Put".to_string()
    } else {
        format!(
            "{} puts",
            capitalize_first(&describe_player_filter(&choose.chooser))
        )
    };
    let opener = if look_at_top.reveal {
        "Reveal"
    } else {
        "Look at"
    };
    let (count_text, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };

    Some(format!(
        "{opener} the top {count_text} {noun} of {owner} library{where_clause}. {actor} up to one of them on top of {owner} library and the rest on the bottom of {owner} library{order}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looked_top_bottom_effects(
        count: Value,
        order: crate::effects::consult_helpers::LibraryBottomOrder,
    ) -> Vec<Effect> {
        let looked = crate::TagKey::from("looked_cards");
        let chosen = crate::TagKey::from("chosen_card");
        let choose = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::tagged(looked.clone()).in_zone(Zone::Library),
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            chosen.clone(),
        )
        .in_zone(Zone::Library);
        let move_to_top =
            crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Library, true);

        vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                count,
                looked.clone(),
            )),
            Effect::new(choose),
            Effect::new(crate::effects::ForEachTaggedEffect::new(
                chosen.clone(),
                vec![Effect::new(move_to_top)],
            )),
            Effect::new(
                crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                    looked,
                    Some(chosen),
                    order,
                    PlayerFilter::You,
                ),
            ),
        ]
    }

    #[test]
    fn dynamic_looked_pool_compacts_up_to_one_top_rest_bottom_random() {
        let count = Value::Devotion {
            player: PlayerFilter::You,
            color: crate::color::Color::Blue,
        }
        .with_surface_hint(ValueSurfaceHint::WhereXIs);
        let effects = looked_top_bottom_effects(
            count,
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
        );

        assert_eq!(
            describe_effect_list(&effects),
            "Look at the top X cards of your library, where X is your devotion to blue. Put up to one of them on top of your library and the rest on the bottom of your library in a random order"
        );
        assert_eq!(
            describe_effect_clause_list(&effects).as_deref(),
            Some(
                "Look at the top X cards of your library, where X is your devotion to blue. Put up to one of them on top of your library and the rest on the bottom of your library in a random order"
            )
        );
    }

    #[test]
    fn looked_pool_preserves_chooser_selected_bottom_order() {
        let effects = looked_top_bottom_effects(
            Value::Fixed(4),
            crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
        );

        assert_eq!(
            describe_effect_list(&effects),
            "Look at the top four cards of your library. Put up to one of them on top of your library and the rest on the bottom of your library in any order"
        );
    }
}
