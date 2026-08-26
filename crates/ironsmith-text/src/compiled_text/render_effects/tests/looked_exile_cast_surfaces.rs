use super::*;

fn looked_exile_cast_effects(selected: TagKey, keep: TagKey) -> Vec<Effect> {
    let looked = TagKey::from("looked_cards");
    let look = Effect::new(crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You,
        Value::Fixed(3),
        looked.clone(),
    ));
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Library);
    filter.excluded_card_types.push(CardType::Land);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            filter,
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            selected.clone(),
        )
        .in_zone(Zone::Library),
    );
    let exile = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(selected.clone()),
        Zone::Exile,
        true,
    ));
    let rest = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            looked,
            Some(keep),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );
    let cast = Effect::new(
        crate::effects::CastTaggedEffect::new(selected, PlayerFilter::You)
            .without_paying_mana_cost(),
    );
    vec![look, choose, exile, rest, cast]
}

#[test]
fn compound_up_to_exile_keeps_its_authored_conjunction() {
    let selected = TagKey::from("__sentence_helper_exiled_up_to_test");
    let effects = looked_exile_cast_effects(selected.clone(), selected);

    assert_eq!(
        describe_effect_list(&effects),
        "Look at the top three cards of your library. Exile up to one nonland card from among them and put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost"
    );
}

#[test]
fn may_exile_and_wrong_complement_do_not_claim_the_up_to_surface() {
    let selected = TagKey::from("ordinary_exiled_card");
    let may_effects = looked_exile_cast_effects(selected.clone(), selected);
    assert_eq!(
        describe_effect_list(&may_effects),
        "Look at the top three cards of your library. You may exile a nonland card from among them. Put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost"
    );

    let up_to = TagKey::from("__sentence_helper_exiled_up_to_test");
    let wrong_complement = looked_exile_cast_effects(up_to, TagKey::from("another_card"));
    assert_ne!(
        describe_effect_list(&wrong_complement),
        "Look at the top three cards of your library. Exile up to one nonland card from among them and put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost"
    );
}
