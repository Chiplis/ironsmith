#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Kicker {X}. X can't be 0.\nFlying\nWhen this creature enters, if it was kicked, gain control of target artifact with mana value X or less. If that artifact is an Equipment, attach it to this creature.";

fn etb_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Thieving Skydiver should have an ETB trigger")
}

fn artifact(name: &str, equipment: bool) -> CardDefinition {
    let mut builder =
        CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Artifact]);
    if equipment {
        builder = builder.subtypes(vec![Subtype::Equipment]);
    }
    builder.build()
}

#[test]
fn thieving_skydiver_keeps_nonzero_kicker_and_exact_attachment_surface() {
    let definition = parse_oracle_card_definition("Thieving Skydiver");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let minimum = definition.abilities.iter().find_map(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        static_ability.this_spell_x_minimum_value()
    });
    assert_eq!(minimum, Some(crate::effect::Value::Fixed(1)));
}

#[test]
fn kicked_skydiver_controls_the_target_and_attaches_only_equipment() {
    for equipment in [false, true] {
        let definition = parse_oracle_card_definition("Thieving Skydiver");
        let trigger = etb_trigger(&definition).clone();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let target = game.create_object_from_definition(
            &artifact(if equipment { "Equipment" } else { "Relic" }, equipment),
            bob,
            Zone::Battlefield,
        );
        let target_assignment = crate::game_state::TargetAssignment {
            spec: ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::artifact().in_zone(Zone::Battlefield),
            )),
            range: 0..1,
        };
        let mut context = crate::effects::ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
            .with_target_assignments(vec![target_assignment.clone()]);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut context,
            alice,
            source,
            &trigger.effects,
            None,
            &[target_assignment],
        )
        .expect("the kicked ETB procedure should resolve");

        assert_eq!(game.controller_of_id(target), Some(alice));
        let attached_to = game.object(target).expect("target remains").attached_to;
        if equipment {
            assert_eq!(
                attached_to,
                Some(crate::object::AttachmentTarget::Object(source))
            );
        } else {
            assert_eq!(attached_to, None, "a non-Equipment must not attach");
        }
    }
}
