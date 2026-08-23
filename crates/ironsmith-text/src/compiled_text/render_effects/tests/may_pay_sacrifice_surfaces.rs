use super::*;

fn pay_one() -> Effect {
    Effect::new(crate::effects::PayManaEffect::new(
        crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
        ChooseSpec::Player(PlayerFilter::You),
    ))
}

fn choose_artifact(tag: TagKey) -> Effect {
    Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::artifact()
                .controlled_by(PlayerFilter::You)
                .in_zone(Zone::Battlefield),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            tag,
        )
        .in_zone(Zone::Battlefield),
    )
}

fn sacrifice_tagged(tag: TagKey) -> Effect {
    Effect::sacrifice_player(ObjectFilter::tagged(tag), 1, PlayerFilter::You)
}

#[test]
fn sequential_may_payment_elides_the_sacrifice_choice_scaffolding() {
    let tag = TagKey::from("sacrificed_0");
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        pay_one(),
        choose_artifact(tag.clone()),
        sacrifice_tagged(tag),
    ]));
    let may = crate::effects::MayEffect::new(vec![sequence]);

    assert_eq!(
        describe_may_compound_payment(&may).as_deref(),
        Some("you may pay {1} and sacrifice an artifact")
    );
    assert_eq!(
        describe_effect_list(&[Effect::new(may)]),
        "You may pay {1} and sacrifice an artifact"
    );
}

#[test]
fn mismatched_sacrifice_tag_keeps_the_explicit_choice() {
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        pay_one(),
        choose_artifact(TagKey::from("chosen_0")),
        sacrifice_tagged(TagKey::from("different_0")),
    ]));
    let may = crate::effects::MayEffect::new(vec![sequence]);
    let rendered =
        describe_may_compound_payment(&may).expect("the remaining typed costs still render");

    assert!(
        rendered.contains("choose an artifact you control"),
        "{rendered}"
    );
    assert_ne!(rendered, "you may pay {1} and sacrifice an artifact");
}

#[test]
fn coordinated_sequence_is_not_reinterpreted_as_a_sequential_payment() {
    let tag = TagKey::from("sacrificed_0");
    let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        pay_one(),
        choose_artifact(tag.clone()),
        sacrifice_tagged(tag),
    ]));
    let may = crate::effects::MayEffect::new(vec![sequence]);

    assert!(describe_may_compound_payment(&may).is_none());
}
