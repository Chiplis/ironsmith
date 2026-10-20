use super::*;

#[test]
fn additive_commander_draw_keeps_base_cards_and_matching_bonus_player() {
    let target = PlayerFilter::Target(Box::new(PlayerFilter::Any));
    let effect = Effect::new(crate::effects::DrawCardsEffect {
        count: Value::Add(Box::new(Value::X), Box::new(Value::CommanderCastCount(target.clone()))),
        player: target.clone(),
    });
    let text = describe_effect(&effect);
    assert!(text.contains("draws X cards plus an additional card for each time they've cast a commander"), "{text}");
    let changed = Effect::new(crate::effects::DrawCardsEffect {
        count: Value::Add(Box::new(Value::X), Box::new(Value::CommanderCastCount(PlayerFilter::You))),
        player: target,
    });
    assert!(!describe_effect(&changed).contains("they've cast"));
}
