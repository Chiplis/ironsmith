#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn no_quarter_keeps_relative_power_and_the_exact_block_pair() {
    let oracle = "Whenever a creature becomes blocked by a creature with lesser power, destroy the blocking creature.\nWhenever a creature blocks a creature with lesser power, destroy the attacking creature.";
    let definition = parse_oracle_card_definition("No Quarter");
    let compiled = canonical_compiled_lines(&definition).join("\n");
    let debug = format!("{definition:#?}");

    assert_eq!(compiled, oracle, "{debug}");

    let becomes_blocked = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::BecomesBlockedByObjectWithLesserPowerTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("No Quarter must retain its becomes-blocked relative-power trigger");
    let blocks = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::BlocksObjectWithLesserPowerTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("No Quarter must retain its blocks relative-power trigger");

    let becomes_blocked_effects = becomes_blocked.effects.flattened_default_effects();
    let blocking_prelude = becomes_blocked_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::TagTriggeringBlockersEffect>())
        .expect("the becomes-blocked trigger must snapshot its blocking creature");
    assert_eq!(
        blocking_prelude.tag.as_str(),
        "blocking",
        "the blocking creature's event identity must not be confused with the blocked attacker"
    );

    for (trigger_name, effects, expected_tag) in [
        ("becomes blocked", becomes_blocked_effects, "blocking"),
        (
            "blocks",
            blocks.effects.flattened_default_effects(),
            "blocked",
        ),
    ] {
        let destroy = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::DestroyEffect>())
            .unwrap_or_else(|| panic!("{trigger_name} trigger must retain its destroy effect"));
        assert!(
            matches!(
                destroy.spec.base(),
                ChooseSpec::Object(filter)
                    if filter.tagged_constraints.len() == 1
                        && filter.tagged_constraints[0].tag.as_str() == expected_tag
            ),
            "{trigger_name} trigger must destroy the exact `{expected_tag}` event participant, got {destroy:#?}"
        );
    }
}

#[test]
fn no_quarter_destroys_only_the_lesser_power_member_of_each_block_pair() {
    fn creature_definition(raw_id: u32, name: &str, power: i32) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, power))
            .build()
    }

    fn resolve_block_pair(
        definition: &CardDefinition,
        attacker_power: i32,
        blocker_power: i32,
        raw_id: u32,
    ) -> (Zone, Zone) {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = game.create_object_from_definition(definition, alice, Zone::Battlefield);
        let attacker = game.create_object_from_definition(
            &creature_definition(raw_id, "Test Attacker", attacker_power),
            alice,
            Zone::Battlefield,
        );
        let blocker = game.create_object_from_definition(
            &creature_definition(raw_id + 1, "Test Blocker", blocker_power),
            bob,
            Zone::Battlefield,
        );
        let attacker_stable = game.object(attacker).expect("attacker exists").stable_id;
        let blocker_stable = game.object(blocker).expect("blocker exists").stable_id;
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::combat::CreatureBlockedEvent::with_snapshots(
                blocker,
                attacker,
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    game.object(blocker).expect("blocker exists"),
                    &game,
                ),
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    game.object(attacker).expect("attacker exists"),
                    &game,
                ),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let trigger_ctx = crate::triggers::TriggerContext::for_source(source, alice, &game);
        let mut matching = definition.abilities.iter().filter_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            triggered
                .trigger
                .matches(&event, &trigger_ctx)
                .then_some(triggered)
        });
        let triggered = matching
            .next()
            .expect("exactly one relative-power trigger must match this unequal block pair");
        assert!(
            matching.next().is_none(),
            "the opposite relative-power direction must not also trigger"
        );

        let mut decisions = crate::decision::AutoPassDecisionMaker;
        let mut execution = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
            .with_triggering_event(event);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut execution,
            alice,
            source,
            &triggered.effects,
            None,
            &[],
        )
        .expect("the matching No Quarter trigger must resolve");

        let stable_zone = |stable| {
            game.find_object_by_stable_id(stable)
                .and_then(|id| game.object(id))
                .map(|object| object.zone)
                .expect("the combat participant must remain represented")
        };
        (stable_zone(attacker_stable), stable_zone(blocker_stable))
    }

    let definition = parse_oracle_card_definition("No Quarter");
    assert_eq!(
        resolve_block_pair(&definition, 5, 2, 96_400),
        (Zone::Battlefield, Zone::Graveyard),
        "a larger attacker becoming blocked must destroy the lesser-power blocker"
    );
    assert_eq!(
        resolve_block_pair(&definition, 2, 5, 96_410),
        (Zone::Graveyard, Zone::Battlefield),
        "a larger blocker must destroy the lesser-power attacking creature"
    );
}
