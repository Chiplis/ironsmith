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
fn comma_then_sequence_elides_the_linked_graveyard_choice() {
    let filter = ObjectFilter::creature()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    let [choose, returned] = graveyard_choice_and_return(filter, PlayerFilter::You, false, false);
    let sequence = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        Effect::mill(Value::Fixed(5)),
        choose,
        returned,
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Mill five cards, then return a creature card from your graveyard to the battlefield"
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
fn untap_preserves_an_authored_plural_pronoun_reference() {
    let mut affected = ObjectFilter::default().in_zone(Zone::Battlefield);
    affected.set_plural_pronoun_reference_surface(true);
    let untap = Effect::new(crate::effects::UntapEffect::all(affected));

    assert_eq!(describe_effect(&untap), "Untap them");
}

#[test]
fn grouped_control_untap_haste_uses_plural_followup_pronouns() {
    let controlled = TagKey::from("controlled_0");
    let untapped = TagKey::from("untapped_1");
    let controlled_filter = ObjectFilter::artifact()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::Opponent);
    let mut control = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    );
    control.target_spec = Some(ChooseSpec::Object(controlled_filter));
    control.modification = None;
    control.runtime_modifications =
        vec![crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController];

    let mut affected = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .match_tagged(controlled.clone(), TaggedOpbjectRelation::IsTaggedObject);
    affected.set_plural_pronoun_reference_surface(true);
    let untap = crate::effects::UntapEffect::all(affected);

    let mut haste = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    );
    haste.target_spec = Some(ChooseSpec::Tagged(untapped.clone()));

    let rendered = describe_gain_control_untap_haste_structural(&[
        Effect::new(crate::effects::TaggedEffect::new(
            controlled,
            Effect::new(control),
        )),
        Effect::new(crate::effects::TaggedEffect::new(
            untapped,
            Effect::new(untap),
        )),
        Effect::new(crate::effects::TaggedEffect::new(
            "granted",
            Effect::new(haste),
        )),
    ])
    .expect("the tagged group should compact");

    assert!(
        rendered.ends_with("Untap them. They gain haste until end of turn"),
        "{rendered}"
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
