use super::*;
/// A chosen name, a random publicly revealed hand subset, and a complete
/// name-matching discard of only that exact subset.
pub(crate) fn describe_named_random_reveal_discard(effects: &[Effect]) -> Option<String> {
    let unwrapped: Vec<_> = effects.iter().map(structural_unwrap_render_wrappers).collect();
    let actions: Vec<_> = unwrapped.iter().copied().filter(|effect|
        effect.downcast_ref::<crate::effects::TargetOnlyEffect>().is_none()).collect();
    let [name_effect, choose_effect, reveal_effect, discard_effect] = actions.as_slice() else { return None; };
    let name = name_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if name.chooser != PlayerFilter::You || name.filter.is_some()
        || !choose.count.is_random() || choose.is_search || choose.reveal
        || choose_primary_zone(choose) != Some(Zone::Hand) || reveal.tag != choose.tag
        || discard.random || discard.any_number
        || !player_filters_refer_to_same_player(&choose.chooser, &discard.player)
    { return None; }
    for effect in &unwrapped {
        if let Some(target) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() {
            let target_player = choose_spec_player_filter(&target.target)?;
            if target.explicit_declaration || !target.target.count().is_single()
                || !player_filters_refer_to_same_player(&target_player, &choose.chooser)
            { return None; }
        }
    }
    let filter = discard.card_filter.as_ref()?;
    let Value::Count(counted) = discard.count.unhinted() else { return None; };
    if counted != filter || filter.zone != Some(Zone::Hand)
        || !filter.owner.as_ref().is_some_and(|owner| player_filters_refer_to_same_player(owner, &discard.player))
    { return None; }
    let mut expected = ObjectFilter::default().in_zone(Zone::Hand)
        .match_tagged(choose.tag.clone(), crate::filter::TaggedOpbjectRelation::SameObjectId)
        .match_tagged(name.tag.clone(), crate::filter::TaggedOpbjectRelation::SameNameAsTagged);
    expected.owner = filter.owner.clone();
    if *filter != expected { return None; }
    let reveal_text = describe_random_hand_reveal_bundle(&[choose_effect, reveal_effect])?;
    Some(format!("Choose a card name. {reveal_text}. Then that player discards all cards with that name revealed this way"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_random_reveal_renders_x_and_requires_both_subset_and_name_relations() {
        let owner = PlayerFilter::target_opponent();
        let name = crate::TagKey::from("chosen_name");
        let selected = crate::TagKey::from("revealed");
        let matched = ObjectFilter::default().in_zone(Zone::Hand).owned_by(owner.clone())
            .match_tagged(selected.clone(), crate::filter::TaggedOpbjectRelation::SameObjectId)
            .match_tagged(name.clone(), crate::filter::TaggedOpbjectRelation::SameNameAsTagged);
        let mut effects = vec![
            Effect::new(crate::effects::ChooseCardNameEffect::new(PlayerFilter::You, None, name)),
            Effect::new(crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Hand).owned_by(owner.clone()),
                crate::ChoiceCount::dynamic_x().at_random(), owner.clone(), selected.clone(),
            ).in_zone(Zone::Hand)),
            Effect::new(crate::effects::RevealTaggedEffect::new(selected)),
            Effect::new(crate::effects::DiscardEffect::new_with_filter(Value::Count(matched.clone()), owner.clone(), false, Some(matched.clone()))),
        ];
        let text = describe_named_random_reveal_discard(&effects).unwrap();
        assert!(text.contains("reveals X cards at random from their hand"), "{text}");
        assert!(text.contains("discards all cards with that name revealed this way"), "{text}");
        let mut wrong = matched;
        wrong.tagged_constraints.retain(|constraint| constraint.relation != crate::filter::TaggedOpbjectRelation::SameObjectId);
        effects[3] = Effect::new(crate::effects::DiscardEffect::new_with_filter(Value::Count(wrong.clone()), owner, false, Some(wrong)));
        assert!(describe_named_random_reveal_discard(&effects).is_none(), "whole-hand named discard cannot masquerade as revealed subset discard");
    }
}
