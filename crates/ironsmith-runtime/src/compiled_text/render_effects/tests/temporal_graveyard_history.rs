use super::*;

fn target_graveyard_card(
    mut filter: ObjectFilter,
    surface: ironsmith_core::GraveyardEntryHistorySurface,
    from_battlefield: bool,
) -> ChooseSpec {
    filter.zone = Some(Zone::Graveyard);
    filter.set_explicit_card_noun(true);
    filter.entered_graveyard_this_turn = true;
    filter.entered_graveyard_from_battlefield_this_turn = from_battlefield;
    filter.set_graveyard_entry_history_surface(Some(surface));
    ChooseSpec::target(ChooseSpec::Object(filter))
}

#[test]
fn target_graveyard_exile_distinguishes_plain_and_from_anywhere_history() {
    let plain = target_graveyard_card(
        ObjectFilter::creature(),
        ironsmith_core::GraveyardEntryHistorySurface::PutThereThisTurn,
        false,
    );
    assert_eq!(
        describe_effect(&Effect::move_to_zone(plain, Zone::Exile, false)),
        "Exile target creature card from a graveyard that was put there this turn"
    );

    let mut nonland = ObjectFilter::default();
    nonland.excluded_card_types.push(CardType::Land);
    let anywhere = target_graveyard_card(
        nonland,
        ironsmith_core::GraveyardEntryHistorySurface::PutThereFromAnywhereThisTurn,
        false,
    );
    assert_eq!(
        describe_effect(&Effect::move_to_zone(anywhere, Zone::Exile, false)),
        "Exile target nonland card from a graveyard that was put there from anywhere this turn"
    );
}

#[test]
fn destination_first_return_preserves_from_battlefield_history() {
    let mut creature = ObjectFilter::creature().owned_by(PlayerFilter::You);
    creature.set_return_destination_first_surface(true);
    let target = target_graveyard_card(
        creature,
        ironsmith_core::GraveyardEntryHistorySurface::PutThereFromBattlefieldThisTurn,
        true,
    );
    let returned = Effect::return_from_graveyard_to_battlefield(target, false);

    assert_eq!(
        describe_effect(&returned),
        "Return to the battlefield target creature card in your graveyard that was put there from the battlefield this turn"
    );
}

#[test]
fn graveyard_history_surface_without_semantic_predicate_does_not_render() {
    let mut filter = ObjectFilter::creature().in_zone(Zone::Graveyard);
    filter.set_graveyard_entry_history_surface(Some(
        ironsmith_core::GraveyardEntryHistorySurface::PutThereFromAnywhereThisTurn,
    ));
    let target = ChooseSpec::target(ChooseSpec::Object(filter));

    assert_eq!(
        describe_effect(&Effect::move_to_zone(target, Zone::Exile, false)),
        "Exile target creature from a graveyard"
    );
}
