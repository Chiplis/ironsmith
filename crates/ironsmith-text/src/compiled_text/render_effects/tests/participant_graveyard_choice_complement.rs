use super::*;

fn program(move_zone: Zone, complement_tag: TagKey) -> crate::effects::ForPlayersEffect {
    let choice_tag = TagKey::from("chosen_graveyard_cards");
    let mut domain = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::IteratedPlayer);
    domain.set_explicit_card_noun(true);
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            domain.clone(),
            ChoiceCount::exactly(2),
            PlayerFilter::IteratedPlayer,
            choice_tag,
        )
        .in_zone(Zone::Graveyard),
    );
    let complement = domain.not_tagged(complement_tag);
    let move_rest = Effect::new(
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Object(complement), move_zone, true)
            .with_actor_surface(PlayerFilter::IteratedPlayer),
    );
    crate::effects::ForPlayersEffect::new(PlayerFilter::Opponent, vec![choose, move_rest])
}

#[test]
fn exact_player_graveyard_choice_complement_renders_as_one_instruction() {
    let program = program(Zone::Exile, TagKey::from("chosen_graveyard_cards"));
    assert_eq!(
        describe_for_players_choose_graveyard_then_exile_rest(&program).as_deref(),
        Some("Each opponent chooses two cards in their graveyard and exiles the rest")
    );
}

#[test]
fn wrong_destination_or_complement_tag_does_not_claim_the_surface() {
    assert!(
        describe_for_players_choose_graveyard_then_exile_rest(&program(
            Zone::Hand,
            TagKey::from("chosen_graveyard_cards"),
        ))
        .is_none()
    );
    assert!(
        describe_for_players_choose_graveyard_then_exile_rest(&program(
            Zone::Exile,
            TagKey::from("unrelated"),
        ))
        .is_none()
    );
}

#[test]
fn watchers_of_the_dead_keeps_the_choice_and_complement_in_one_instruction() {
    const ORACLE: &str = "Exile this creature: Each opponent chooses two cards in their graveyard and exiles the rest.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Watchers of the Dead")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .parse_text(ORACLE)
            .expect("the participant graveyard complement should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        ORACLE
    );
}
