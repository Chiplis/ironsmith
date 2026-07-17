use super::*;

fn graveyard_choice_and_return(
    filter: ObjectFilter,
    chooser: PlayerFilter,
    tapped: bool,
    top_only: bool,
) -> [Effect; 2] {
    let tag = TagKey::from("chosen_return_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        filter,
        ChoiceCount::exactly(1),
        chooser,
        tag.clone(),
    )
    .in_zone(Zone::Graveyard);
    let choose = if top_only { choose.top_only() } else { choose };
    let returned = Effect::return_from_graveyard_to_battlefield(ChooseSpec::Tagged(tag), tapped)
        .tag("returned_0");

    [Effect::new(choose), returned]
}

#[test]
fn adjacent_graveyard_choice_and_return_render_as_one_precise_action() {
    let filter = ObjectFilter::permanent_card()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .with_mana_value(crate::filter::Comparison::LessThanOrEqual(3));
    let [choose, returned] = graveyard_choice_and_return(filter, PlayerFilter::You, true, false);

    assert_eq!(
        describe_choose_then_return_from_graveyard(&choose, &returned).as_deref(),
        Some(
            "you return a permanent card with mana value 3 or less from your graveyard to the battlefield tapped"
        )
    );
}

#[test]
fn clause_list_elides_the_runtime_choice_without_altering_its_ast() {
    let filter = ObjectFilter::creature()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    let [choose, returned] = graveyard_choice_and_return(filter, PlayerFilter::You, false, false);
    let effects = vec![Effect::mill(Value::Fixed(4)), choose, returned];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("mill four cards, then return a creature card from your graveyard to the battlefield")
    );
    assert!(
        effects[1]
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .is_some(),
        "compiled-text rendering must not remove the runtime choice"
    );
}

#[test]
fn compactor_preserves_a_noncontroller_chooser_and_owner() {
    let filter = ObjectFilter::creature()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::Opponent);
    let [choose, returned] =
        graveyard_choice_and_return(filter, PlayerFilter::Opponent, false, false);

    assert_eq!(
        describe_choose_then_return_from_graveyard(&choose, &returned).as_deref(),
        Some("an opponent returns a creature card from an opponent's graveyard to the battlefield")
    );
}

#[test]
fn top_only_graveyard_choice_preserves_the_filtered_top_card_surface() {
    let filter = ObjectFilter::creature()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    let [choose, returned] = graveyard_choice_and_return(filter, PlayerFilter::You, false, true);

    assert_eq!(
        describe_choose_then_return_from_graveyard(&choose, &returned).as_deref(),
        Some("you return the top creature card of your graveyard to the battlefield")
    );
}

#[test]
fn top_only_graveyard_exile_preserves_filtered_top_card_surface() {
    let tag = TagKey::from("chosen_top_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .top_only();
    let exile = crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag));

    assert_eq!(
        describe_choose_then_exile(&choose, &exile).as_deref(),
        Some("you exile the top creature card of your graveyard")
    );
}

#[test]
fn ordinary_graveyard_exile_remains_a_choice_from_the_zone() {
    let tag = TagKey::from("chosen_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    );
    let exile = crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag));

    assert_eq!(
        describe_choose_then_exile(&choose, &exile).as_deref(),
        Some("you exile a creature card from your graveyard")
    );
}

#[test]
fn top_only_graveyard_to_library_bottom_renders_as_one_ordered_move() {
    let tag = TagKey::from("chosen_top_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .top_only();
    let move_to_library =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Library, false);

    assert_eq!(
        describe_choose_then_move_to_library(&choose, &move_to_library).as_deref(),
        Some("you put the top card of your graveyard on the bottom of your library")
    );
}
