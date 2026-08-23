#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effect::Condition;
use crate::effects::{ConditionalEffect, ExecutionContext};
use crate::object::ObjectKind;
use crate::types::Subtype;

const SHIPBREAKER: &str = "{6}{U}{U}: Monstrosity 4.\nWhen this creature becomes monstrous, tap up to four target creatures. Those creatures don't untap during their controllers' untap steps for as long as you control this creature.";
const VOLCANIC: &str = "Destroy target nonbasic land you don't control and target nonbasic land of an opponent's choice you don't control.\nVolcanic Offering deals 7 damage to target creature you don't control and 7 damage to target creature of an opponent's choice you don't control.";
const CACHE: &str = "Mill four cards. You may put a permanent card from among the cards milled this way into your hand. If you control a Squirrel or returned a Squirrel card to your hand this way, create a Food token.";

#[test]
fn shipbreaker_current_source_keeps_the_plural_tagged_lock_set() {
    let definition = parse_oracle_card_definition("Shipbreaker Kraken");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        SHIPBREAKER
    );
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered.effects.segments.iter().any(|segment| {
                    segment.default_effects.iter().any(|effect| {
                        effect
                            .downcast_ref::<crate::effects::CantEffect>()
                            .is_some()
                    })
                }) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("monstrous trigger should retain its untap restriction");
    let cant = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| effect.downcast_ref::<crate::effects::CantEffect>())
        .expect("tagged creatures should receive one persistent restriction");
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        panic!("expected an untap restriction: {cant:#?}");
    };
    assert_eq!(cant.duration, crate::effect::Until::YouStopControllingThis);
    assert_eq!(filter.tagged_constraints.len(), 1, "{filter:#?}");
}

#[test]
fn volcanic_offering_keeps_both_opponent_chosen_targets_typed() {
    let definition = parse_oracle_card_definition("Volcanic Offering");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), VOLCANIC);
    let program = definition.spell_effect.as_ref().expect("spell program");
    fn count_delegated(effect: &crate::effect::Effect) -> usize {
        let mut count = usize::from(
            effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some_and(|target| {
                    target.chooser == Some(PlayerFilter::Opponent) && target.explicit_declaration
                }),
        );
        effect.visit_child_effects(&mut |child| count += count_delegated(child));
        count
    }
    let delegated = program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .map(count_delegated)
        .sum::<usize>();
    assert_eq!(delegated, 2, "land and creature choices: {program:#?}");
}

fn contains_returned_squirrel_result(condition: &Condition) -> bool {
    match condition {
        Condition::Or(left, right) | Condition::And(left, right) => {
            contains_returned_squirrel_result(left) || contains_returned_squirrel_result(right)
        }
        Condition::PlayerTaggedObjectMatches {
            player,
            filter,
            mode,
            ..
        } => {
            *player == PlayerFilter::You
                && filter.zone == Some(Zone::Hand)
                && filter.subtypes == [Subtype::Squirrel]
                && *mode == crate::effect::TaggedObjectMatchMode::CurrentOrLastKnown
        }
        _ => false,
    }
}

#[test]
fn cache_grab_returned_squirrel_is_a_tagged_result_not_a_hand_control_test() {
    let definition = parse_oracle_card_definition("Cache Grab");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), CACHE);
    let program = definition.spell_effect.as_ref().expect("spell program");
    fn effect_has_returned_squirrel_result(effect: &crate::effect::Effect) -> bool {
        if let Some(conditional) = effect.downcast_ref::<ConditionalEffect>()
            && contains_returned_squirrel_result(&conditional.condition)
        {
            return true;
        }
        let mut found = false;
        effect
            .visit_child_effects(&mut |child| found |= effect_has_returned_squirrel_result(child));
        found
    }
    assert!(
        program
            .segments
            .iter()
            .flat_map(|segment| &segment.default_effects)
            .any(effect_has_returned_squirrel_result),
        "the return result must be executable provenance: {program:#?}"
    );
}

#[test]
fn cache_grab_creates_food_when_the_selected_milled_permanent_is_a_squirrel() {
    let definition = parse_oracle_card_definition("Cache Grab");
    let program = definition.spell_effect.as_ref().expect("spell program");
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let squirrel = CardDefinitionBuilder::new(CardId::new(), "Milled Squirrel")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Squirrel])
        .build();
    let squirrel_id = game.create_object_from_definition(&squirrel, alice, Zone::Library);
    let squirrel_stable = game.object(squirrel_id).expect("milled Squirrel").stable_id;
    let filler = CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Instant])
        .build();
    for _ in 0..3 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }

    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Cache Grab should resolve");

    let moved_squirrel = game
        .find_object_by_stable_id(squirrel_stable)
        .and_then(|id| game.object(id))
        .expect("the chosen Squirrel remains represented after changing zones");
    assert_eq!(moved_squirrel.zone, Zone::Hand);
    assert!(
        game.battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|object| {
                matches!(object.kind, ObjectKind::Token) && object.subtypes.contains(&Subtype::Food)
            })
    );
}
