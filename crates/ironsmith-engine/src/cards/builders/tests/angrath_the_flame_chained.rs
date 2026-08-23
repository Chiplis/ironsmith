#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};

const MINUS_THREE_TEXT: &str = "Gain control of target creature until end of turn. Untap it. It gains haste until end of turn. Sacrifice it at the beginning of the next end step if it has mana value 3 or less.";

fn resolve_delayed_end_step_triggers(game: &mut crate::GameState, player: PlayerId) -> usize {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    );
    let entries = crate::triggers::check_delayed_triggers(game, &event);
    let count = entries.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    if count > 0 {
        crate::game_loop::put_triggers_on_stack(game, &mut queue)
            .expect("Angrath's delayed sacrifices should go on the stack");
        while !game.stack.is_empty() {
            crate::game_loop::resolve_stack_entry(game)
                .expect("Angrath's delayed sacrifice should resolve");
        }
    }
    count
}

fn creature_with_mana_value(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn angrath_checks_mana_value_when_the_delayed_sacrifice_resolves() {
    let definition = parse_oracle_card_definition("Angrath, the Flame-Chained");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line.contains(MINUS_THREE_TEXT)),
        "Angrath's compiled text must retain the delayed mana-value condition: {rendered:#?}"
    );

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if format!("{:?}", activated.effects).contains("ScheduleDelayedTriggerEffect") =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Angrath should have a delayed-sacrifice loyalty ability");
    let debug = format!("{:#?}", activated.effects);
    assert!(debug.contains("ConditionalEffect"), "{debug}");
    assert!(debug.contains("mana_value"), "{debug}");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let angrath = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let cheap = game.create_object_from_definition(
        &creature_with_mana_value("Cheap Creature", 3),
        bob,
        Zone::Battlefield,
    );
    let expensive = game.create_object_from_definition(
        &creature_with_mana_value("Expensive Creature", 4),
        bob,
        Zone::Battlefield,
    );
    let cheap_stable = game
        .object(cheap)
        .expect("cheap creature should exist")
        .stable_id;
    let expensive_stable = game
        .object(expensive)
        .expect("expensive creature should exist")
        .stable_id;

    for target in [cheap, expensive] {
        let mut ctx = ExecutionContext::new_default(angrath, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        for effect in &activated.effects {
            execute_effect(&mut game, effect, &mut ctx)
                .expect("Angrath's -3 effects should resolve");
        }
    }

    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        2,
        "each stolen creature should have its own delayed instruction"
    );
    assert_eq!(resolve_delayed_end_step_triggers(&mut game, alice), 2);

    let cheap = game
        .find_object_by_stable_id(cheap_stable)
        .expect("cheap creature should retain its stable identity");
    let expensive = game
        .find_object_by_stable_id(expensive_stable)
        .expect("expensive creature should retain its stable identity");
    assert_eq!(
        game.object(cheap)
            .expect("cheap creature should exist")
            .zone,
        Zone::Graveyard,
        "a creature with mana value 3 must be sacrificed"
    );
    assert_eq!(
        game.object(expensive)
            .expect("expensive creature should exist")
            .zone,
        Zone::Battlefield,
        "a creature with mana value 4 must survive the conditional sacrifice"
    );
}
