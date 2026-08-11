#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "At the beginning of your end step, sacrifice each other creature you control.\nDiscard two cards, Sacrifice this creature: Choose target opponent. Return this creature to the battlefield under that player's control at the beginning of their next upkeep.";

fn activated_ability(definition: &CardDefinition) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Plague Reaver should have one activated ability")
}

fn delayed_return(
    activated: &crate::ability::ActivatedAbility,
) -> &crate::effects::ScheduleDelayedTriggerEffect {
    activated
        .effects
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .expect("the activated ability should register one delayed return")
}

#[test]
fn plague_reaver_keeps_the_target_opponents_delayed_control_and_each_surface() {
    let definition = parse_oracle_card_definition("Plague Reaver");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let activated = activated_ability(&definition);
    let schedule = delayed_return(activated);
    assert!(schedule.one_shot);
    assert!(schedule.start_next_turn);
    assert!(
        schedule
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
            .is_some_and(|upkeep| {
                upkeep.player == PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent))
            })
    );

    let delayed_effects = schedule.effects.flattened_default_effects();
    let [put] = delayed_effects else {
        panic!("expected one typed delayed battlefield move: {schedule:#?}");
    };
    let put = put
        .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
        .expect("the delayed action should use the typed battlefield-entry effect");
    assert!(matches!(put.target.unhinted(), ChooseSpec::Source));
    assert_eq!(
        put.controller,
        PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent))
    );
    assert!(!put.tapped);
    assert!(put.enters_with_counters.is_empty());
}

#[test]
fn plague_reaver_returns_only_on_the_chosen_opponents_next_upkeep_under_their_control() {
    let definition = parse_oracle_card_definition("Plague Reaver");
    let activated = activated_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
    let source_stable_id = game.object(source).expect("Plague Reaver exists").stable_id;

    game.push_to_stack(
        crate::game_state::StackEntry::ability(source, alice, activated.effects.clone())
            .with_targets(vec![crate::game_state::Target::Player(bob)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("the activated ability should register its delayed return");
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert_eq!(
        game.effect_store.delayed_triggers[0]
            .tagged_players
            .get(crate::tag::DELAYED_TARGET_PLAYERS_TAG),
        Some(&vec![bob]),
        "the delayed registration must retain the exact targeted opponent"
    );

    let upkeep = |player| {
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfUpkeepEvent::new(player),
            crate::provenance::ProvNodeId::default(),
        )
    };
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &upkeep(bob)).is_empty(),
        "start-next-turn must prevent a same-turn upkeep from firing"
    );

    game.turn.turn_number += 1;
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &upkeep(alice)).is_empty(),
        "the delayed trigger must ignore an unchosen player's upkeep"
    );
    let fired = crate::triggers::check_delayed_triggers(&mut game, &upkeep(bob));
    assert_eq!(
        fired.len(),
        1,
        "the chosen opponent's next upkeep fires once"
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in fired {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("the delayed return should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("the delayed return should resolve");

    let returned = game
        .find_object_by_stable_id(source_stable_id)
        .and_then(|id| game.object(id))
        .expect("Plague Reaver should retain its stable identity");
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(game.controller_of(returned), bob);
}
