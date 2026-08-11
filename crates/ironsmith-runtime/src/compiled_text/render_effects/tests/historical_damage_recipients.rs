use super::*;

fn historical_recipient_effects() -> Vec<Effect> {
    let amount = Value::Fixed(1);
    let players = Effect::new(crate::effects::ForPlayersEffect::new(
        PlayerFilter::was_dealt_damage_by_source_this_game(PlayerFilter::Opponent),
        vec![Effect::deal_damage(
            amount.clone(),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer),
        )],
    ));
    let mut planeswalkers = ObjectFilter::planeswalker();
    planeswalkers.was_dealt_damage_by_source_this_game = true;
    planeswalkers.set_explicit_card_type_noun(Some(CardType::Planeswalker));
    let objects = Effect::new(crate::effects::ForEachObject::new(
        planeswalkers,
        vec![Effect::deal_damage(amount, ChooseSpec::Iterated)],
    ));
    vec![players, objects]
}

#[test]
fn exact_historical_player_planeswalker_union_uses_authored_surface() {
    assert_eq!(
        describe_structural_multisentence_effect_list(&historical_recipient_effects()).as_deref(),
        Some("Deal 1 damage to each opponent and planeswalker it has dealt damage to this game")
    );
}

#[test]
fn historical_recipient_union_rejects_different_amount_or_missing_history() {
    let mut different_amount = historical_recipient_effects();
    let objects = different_amount[1]
        .downcast_ref::<crate::effects::ForEachObject>()
        .expect("object loop");
    let mut changed_objects = objects.clone();
    changed_objects.effects = vec![Effect::deal_damage(Value::Fixed(2), ChooseSpec::Iterated)];
    different_amount[1] = Effect::new(changed_objects);
    assert!(describe_structural_multisentence_effect_list(&different_amount).is_none());

    let mut missing_history = historical_recipient_effects();
    let objects = missing_history[1]
        .downcast_ref::<crate::effects::ForEachObject>()
        .expect("object loop");
    let mut changed_objects = objects.clone();
    changed_objects.filter.was_dealt_damage_by_source_this_game = false;
    missing_history[1] = Effect::new(changed_objects);
    assert!(describe_structural_multisentence_effect_list(&missing_history).is_none());
}
