#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::{ExecutionContext, ResolvedTarget};

fn resolve_through_prevention(name: &str, x_value: u32, keep_card_in_hand: bool) -> (bool, i32) {
    let definition = parse_oracle_card_definition(name);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    if keep_card_in_hand {
        let hand_card = CardDefinitionBuilder::new(CardId::new(), "Protection Probe Hand Card")
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_definition(&hand_card, alice, Zone::Hand);
    }

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.object_mut(spell)
        .expect("spell should be on the stack")
        .x_value = Some(x_value);
    game.push_to_stack(crate::game_state::StackEntry::new(spell, alice).with_x(x_value));
    game.update_cant_effects();
    let counter_protected = !game.can_be_countered(spell);

    let mut decision_maker = SelectFirstDecisionMaker;
    let shield = crate::effects::PreventNextTimeDamageEffect::new(
        crate::effects::PreventNextTimeDamageSource::Target(ChooseSpec::Source),
        crate::effects::PreventNextTimeDamageTarget::AnyTarget,
    );
    {
        let mut shield_ctx = ExecutionContext::new(spell, alice, &mut decision_maker);
        crate::effects::execute_effect(
            &mut game,
            &crate::effect::Effect::new(shield),
            &mut shield_ctx,
        )
        .expect("damage-prevention shield should register");
    }
    {
        let mut resolution_ctx = ExecutionContext::new(spell, alice, &mut decision_maker)
            .with_x(x_value)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut resolution_ctx,
            alice,
            spell,
            definition
                .spell_effect
                .as_ref()
                .expect("spell should have a resolution program"),
            None,
            &[],
        )
        .expect("conditional spell-protection probe should resolve");
    }

    (
        counter_protected,
        game.player(bob).expect("Bob should exist").life,
    )
}

#[test]
fn conditional_spell_protection_keeps_one_typed_condition_and_oracle_surface() {
    for (name, expected) in [
        (
            "Banefire",
            "Banefire deals X damage to any target.\nIf X is 5 or more, this spell can't be countered and the damage can't be prevented.",
        ),
        (
            "Demonfire",
            "Demonfire deals X damage to any target. If a creature dealt damage this way would die this turn, exile it instead.\nHellbent — If you have no cards in hand, this spell can't be countered and the damage can't be prevented.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            compiled_text_lines(&definition).join("\n"),
            expected,
            "{name}: {:#?}",
            definition.abilities
        );

        let conditions =
            definition
                .abilities
                .iter()
                .filter(|ability| ability.functions_in(&Zone::Stack))
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Static(static_ability) => static_ability
                        .compiled_model()
                        .and_then(|model| match &model.payload {
                            ironsmith_core::StaticAbilityPayload::Conditional {
                                condition, ..
                            } => Some(condition.clone()),
                            _ => None,
                        }),
                    _ => None,
                })
                .collect::<Vec<_>>();
        assert_eq!(conditions.len(), 2, "{name}: {:#?}", definition.abilities);
        assert_eq!(conditions[0], conditions[1], "{name}");
    }
}

#[test]
fn x_threshold_spell_protection_has_distinct_false_and_true_branches() {
    assert_eq!(
        resolve_through_prevention("Banefire", 4, false),
        (false, 20),
        "below the threshold the spell is counterable and its damage is preventable"
    );
    assert_eq!(
        resolve_through_prevention("Banefire", 5, false),
        (true, 15),
        "at the threshold both spell protections apply"
    );
}

#[test]
fn empty_hand_spell_protection_has_distinct_false_and_true_branches() {
    assert_eq!(
        resolve_through_prevention("Demonfire", 3, true),
        (false, 20),
        "with a card in hand the spell is counterable and its damage is preventable"
    );
    assert_eq!(
        resolve_through_prevention("Demonfire", 3, false),
        (true, 17),
        "with an empty hand both spell protections apply"
    );
}
