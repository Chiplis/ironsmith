#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effects::{DealDistributedDamageEffect, ExecutionContext, ResolvedTarget};
use crate::game_state::{Target, TargetAssignment, TargetDistribution};

const ORACLE: &str = "Trample\nWhenever another creature dies, put a +1/+1 counter on Orca.\nWhen Orca dies, it deals damage equal to its power divided as you choose among any number of targets.";

fn distributed_trigger(
    definition: &CardDefinition,
) -> (
    &crate::ability::TriggeredAbility,
    &DealDistributedDamageEffect,
) {
    definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            let distributed = triggered
                .effects
                .flattened_default_effects()
                .into_iter()
                .find_map(|effect| {
                    effect
                        .downcast_ref::<TaggedEffect>()
                        .and_then(|tagged| {
                            tagged.effect.downcast_ref::<DealDistributedDamageEffect>()
                        })
                        .or_else(|| effect.downcast_ref::<DealDistributedDamageEffect>())
                })?;
            Some((triggered, distributed))
        })
        .expect("Orca should have one distributed-damage dies trigger")
}

fn durable_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 10))
        .build()
}

#[test]
fn orca_distributed_damage_reuses_the_triggering_lki_power_and_source() {
    let definition = parse_oracle_card_definition("Orca, Siege Demon");
    assert_eq!(unprocessed_compiled_lines(&definition).join("\n"), ORACLE);

    let (triggered, distributed) = distributed_trigger(&definition);
    let [segment] = triggered.effects.segments.as_slice() else {
        panic!("expected one dies-trigger resolution segment: {triggered:#?}");
    };
    let [tag_triggering, _] = segment.default_effects.as_slice() else {
        panic!("expected triggering-object prelude and damage: {segment:#?}");
    };
    let triggering_tag = &tag_triggering
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        .expect("triggering object must be snapshotted")
        .tag;
    assert!(matches!(
        distributed.source.unhinted(),
        ChooseSpec::Tagged(tag) if tag == triggering_tag
    ));
    assert!(matches!(
        distributed.amount.unhinted(),
        crate::effect::Value::PowerOf(spec)
            if matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag == triggering_tag)
                && spec.source_reference_surface()
                    == Some(&crate::target::SourceReferenceSurface::ThisPermanentType(
                        "it".to_string()
                    ))
    ));
    assert!(matches!(
        distributed.target.unhinted(),
        ChooseSpec::WithCount(inner, count)
            if matches!(inner.unhinted(), ChooseSpec::AnyTarget)
                && count.is_any_number()
    ));

    let mut changed_source = distributed.clone();
    changed_source.source = ChooseSpec::Tagged(crate::TagKey::from("unrelated"));
    let near_miss =
        crate::compiled_text::describe_effect(&crate::effect::Effect::new(changed_source));
    assert!(
        near_miss.contains("where X is") && !near_miss.contains("damage equal to its power"),
        "an unrelated source must not borrow the correlated reflexive surface: {near_miss}"
    );
}

#[test]
fn orca_uses_last_known_power_for_the_announced_damage_division() {
    let definition = parse_oracle_card_definition("Orca, Siege Demon");
    let (triggered, distributed) = distributed_trigger(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(source, crate::CounterType::PlusOnePlusOne, 2)
        .expect("Orca exists");
    let target = game.create_object_from_definition(
        &durable_creature("Damage Target"),
        bob,
        Zone::Battlefield,
    );
    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(source).expect("Orca exists"),
        &game,
    );
    assert_eq!(snapshot.power, Some(7));
    game.move_object_by_effect(source, Zone::Graveyard)
        .expect("Orca should die");
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            source,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(vec![snapshot]);

    let assignment = TargetAssignment {
        spec: distributed.target.clone(),
        range: 0..2,
    };
    let distribution = TargetDistribution {
        spec: distributed.target.clone(),
        range: 0..2,
        allocations: vec![(Target::Object(target), 3), (Target::Player(bob), 4)],
    };
    let mut context = ExecutionContext::new_default(source, alice)
        .with_triggering_event(event)
        .with_targets(vec![
            ResolvedTarget::Object(target),
            ResolvedTarget::Player(bob),
        ])
        .with_target_assignments(vec![assignment.clone()])
        .with_target_distributions(vec![distribution]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered.effects,
        None,
        &[assignment],
    )
    .expect("Orca's dies trigger should resolve from LKI");

    assert_eq!(game.damage_on(target), 3);
    assert_eq!(game.life_total(bob), 16);
}
