use super::*;
use crate::effect::{
    Condition, EffectMetric, EffectMetricSource, PriorEffectAction, PriorEffectMetricQuery,
};
use crate::filter::{ObjectFilter, ObjectFilterExt as _};
use crate::game_state::StackEntry;
use crate::ids::PlayerId;
use crate::object::CounterType;
use crate::target::ChooseSpec;
use crate::zone::Zone;

fn create_large_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(10, 10))
            .build(),
        controller,
        Zone::Battlefield,
    )
}

fn activated_counter_damage_effects(partitioned_by_player: bool) -> Vec<Effect> {
    let counter_type = CounterType::PlusOnePlusOne;
    let removal_id = crate::effect::EffectId(31);
    let removed_count = Value::PriorEffectMetric {
        effect_id: removal_id,
        query: PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::Count)
            .with_action(PriorEffectAction::Removed)
            .with_counter_type(Some(counter_type)),
    };
    let removal = Effect::with_id(
        removal_id.0,
        Effect::remove_counters(
            counter_type,
            Value::CountersOnSource(counter_type),
            ChooseSpec::Source,
        ),
    );
    let global_creature_damage = Effect::for_each(
        ObjectFilter::creature().in_zone(Zone::Battlefield),
        vec![Effect::deal_damage(
            removed_count.clone(),
            ChooseSpec::Iterated,
        )],
    );
    let global_player_damage = Effect::for_players(
        PlayerFilter::Any,
        vec![Effect::deal_damage(
            removed_count.clone(),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer),
        )],
    );
    let damage = if partitioned_by_player {
        let mut controlled_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
        controlled_creatures.controller = Some(PlayerFilter::IteratedPlayer);
        vec![Effect::for_players(
            PlayerFilter::Any,
            vec![
                Effect::deal_damage(
                    removed_count.clone(),
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                ),
                Effect::for_each(
                    controlled_creatures,
                    vec![Effect::deal_damage(removed_count, ChooseSpec::Iterated)],
                ),
            ],
        )]
    } else {
        vec![global_creature_damage, global_player_damage]
    };
    let mut conditional_effects = vec![removal];
    conditional_effects.extend(damage);

    vec![
        Effect::put_counters_on_source(counter_type, 1),
        Effect::new(crate::effects::ConditionalEffect::if_only(
            Condition::ThisAbilityResolvedThisTurnExactly(3),
            vec![Effect::new(crate::effects::SequenceEffect::coordinated(
                conditional_effects,
            ))],
        )),
    ]
}

fn assert_third_resolution_removes_exact_count_and_damages_all_recipients(
    partitioned_by_player: bool,
) {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = create_large_creature(&mut game, "Counter Adept", alice);
    let other = create_large_creature(&mut game, "Training Partner", bob);
    let effects = activated_counter_damage_effects(partitioned_by_player);

    for resolution in 1..=3 {
        game.push_to_stack(
            StackEntry::ability(source, alice, effects.clone()).with_ability_index(0),
        );
        resolve_stack_entry(&mut game).expect("activated ability should resolve");

        if resolution < 3 {
            assert_eq!(
                game.counter_count(source, CounterType::PlusOnePlusOne),
                resolution,
                "the gated branch must not run before the third resolution"
            );
            assert_eq!(game.player(alice).expect("alice exists").life, 20);
            assert_eq!(game.player(bob).expect("bob exists").life, 20);
            assert_eq!(game.damage_on(source), 0);
            assert_eq!(game.damage_on(other), 0);
        }
    }

    assert_eq!(
        game.counter_count(source, CounterType::PlusOnePlusOne),
        0,
        "all three counters should be removed"
    );
    assert_eq!(game.player(alice).expect("alice exists").life, 17);
    assert_eq!(game.player(bob).expect("bob exists").life, 17);
    assert_eq!(game.damage_on(source), 3);
    assert_eq!(game.damage_on(other), 3);
}

#[test]
fn third_activated_resolution_removes_exact_count_and_damages_all_recipients() {
    assert_third_resolution_removes_exact_count_and_damages_all_recipients(false);
}

#[test]
fn player_partitioned_fanout_uses_the_same_removed_count_for_every_recipient() {
    assert_third_resolution_removes_exact_count_and_damages_all_recipients(true);
}
