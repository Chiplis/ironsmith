use super::*;

const SECOND_SUNRISE_TEXT: &str = "Each player returns to the battlefield all artifact, creature, enchantment, and land cards in their graveyard that were put there from the battlefield this turn";

#[test]
fn each_player_destination_first_return_renders_graveyard_history() {
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
    ];
    filter.entered_graveyard_this_turn = true;
    filter.entered_graveyard_from_battlefield_this_turn = true;
    filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::All));
    filter.set_return_destination_first_surface(true);
    filter.set_explicit_card_noun(true);

    let returned = Effect::new(
        crate::effects::ReturnAllToBattlefieldEffect::new(filter, false).under_owner_control(),
    );
    let each_player = Effect::new(crate::effects::ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![returned],
    ));

    assert_eq!(describe_effect(&each_player), SECOND_SUNRISE_TEXT);
}
