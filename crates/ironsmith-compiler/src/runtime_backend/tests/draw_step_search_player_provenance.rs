use super::*;

fn assert_draw_step_search_uses_the_active_player(
    name: &str,
    card_type: CardType,
    rules_text: &str,
) {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .parse_text(rules_text)
        .expect("the draw-step search ability should compile");
    let program = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(&triggered.effects),
            _ => None,
        })
        .expect("the card should contain a triggered ability");
    let effects = program.flattened_default_effects();
    let life_loss = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::LoseLifeEffect>(effect))
        .expect("the draw-step trigger should make its player lose life");
    let search = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::SearchLibraryEffect>(effect))
        .expect("the draw-step trigger should search that player's library");

    assert_eq!(
        life_loss.player,
        crate::target::PlayerFilter::IteratedPlayer,
        "{name}: the life loss should use the draw-step event player"
    );
    assert_eq!(
        search.chooser,
        crate::target::PlayerFilter::IteratedPlayer,
        "{name}: the same player should choose the searched card"
    );
    assert_eq!(
        search.player,
        crate::target::PlayerFilter::IteratedPlayer,
        "{name}: the same player's library should be searched and shuffled"
    );
    assert_eq!(
        search.filter.owner,
        Some(crate::target::PlayerFilter::IteratedPlayer),
        "{name}: the search filter should remain scoped to that player's library"
    );
}

#[test]
fn maralen_draw_step_search_keeps_the_draw_step_player() {
    assert_draw_step_search_uses_the_active_player(
        "Maralen of the Mornsong",
        CardType::Creature,
        "Players can't draw cards.\nAt the beginning of each player's draw step, that player loses 3 life, searches their library for a card, puts it into their hand, then shuffles.",
    );
}

#[test]
fn mornsong_aria_draw_step_search_keeps_the_draw_step_player() {
    assert_draw_step_search_uses_the_active_player(
        "Mornsong Aria",
        CardType::Enchantment,
        "Players can't draw cards or gain life.\nAt the beginning of each player's draw step, that player loses 3 life, searches their library for a card, puts it into their hand, then shuffles.",
    );
}
