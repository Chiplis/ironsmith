use ironsmith::bench_support::{EffectMix, battlefield_scale, complex_layer_cake_stress_report};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{CardType, Color, ColorSet, GameState, ObjectId, Subtype};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let verify = args.iter().any(|arg| arg == "--verify");
    let positional: Vec<_> = args
        .iter()
        .filter(|arg| arg.as_str() != "--verify")
        .collect();
    let creatures = positional
        .first()
        .map(|value| value.as_str())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(300);
    let effects = positional
        .get(1)
        .map(|value| value.as_str())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(12);

    if verify {
        verify_expected_characteristics(creatures, effects);
        return;
    }

    let report = complex_layer_cake_stress_report(creatures, effects);
    println!("{report:#?}");
}

fn verify_expected_characteristics(creatures: usize, effects: usize) {
    assert!(
        creatures >= 4,
        "expected at least four creatures to sample each synthetic subtype"
    );
    assert_eq!(
        effects, 72,
        "expected-characteristic verification is calibrated for 72 effects"
    );
    let scenario = battlefield_scale(creatures, EffectMix::ComplexLayerCake(effects));
    assert_token_characteristics(
        &scenario.game,
        scenario.battlefield[0],
        Subtype::Goblin,
        27,
        15,
        ColorSet::GREEN.union(ColorSet::WHITE).union(ColorSet::RED),
        &[StaticAbilityId::Flying],
    );
    assert_token_characteristics(
        &scenario.game,
        scenario.battlefield[1],
        Subtype::Elf,
        15,
        3,
        ColorSet::GREEN,
        &[StaticAbilityId::Vigilance],
    );
    assert_token_characteristics(
        &scenario.game,
        scenario.battlefield[2],
        Subtype::Soldier,
        9,
        15,
        ColorSet::GREEN,
        &[StaticAbilityId::Haste],
    );
    assert_token_characteristics(
        &scenario.game,
        scenario.battlefield[3],
        Subtype::Zombie,
        15,
        3,
        ColorSet::GREEN.union(ColorSet::from_color(Color::Black)),
        &[],
    );
    println!(
        "verified representative characteristics for {creatures} creatures + {effects} effects"
    );
}

fn assert_token_characteristics(
    game: &GameState,
    id: ObjectId,
    subtype: Subtype,
    power: i32,
    toughness: i32,
    colors: ColorSet,
    static_ability_ids: &[StaticAbilityId],
) {
    let chars = game
        .calculated_characteristics(id)
        .expect("token should have calculated characteristics");
    assert_eq!(chars.power, Some(power));
    assert_eq!(chars.toughness, Some(toughness));
    assert!(chars.card_types.contains(&CardType::Creature));
    assert!(chars.card_types.contains(&CardType::Artifact));
    assert!(chars.subtypes.contains(&subtype));
    assert_eq!(chars.colors, colors);
    for ability_id in static_ability_ids {
        assert!(
            chars
                .static_abilities
                .iter()
                .any(|ability| ability.id() == *ability_id),
            "expected token #{:?} to have static ability {:?}, got {:?}",
            id,
            ability_id,
            chars.static_abilities
        );
    }
}
