#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "At the beginning of your upkeep, sacrifice this enchantment unless you pay 2 life.\nAt the beginning of the end step, destroy all untapped creatures that didn't attack this turn, except for creatures that couldn't attack.";

fn creature(name: &str, defender: bool) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2));
    if defender {
        builder = builder.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::defender(),
        ));
    }
    builder.build()
}

fn end_step_destroy_program(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| {
            triggered.effects.segments.iter().any(|segment| {
                segment
                    .default_effects
                    .iter()
                    .any(|effect| effect.downcast_ref::<DestroyEffect>().is_some())
            })
        })
        .map(|triggered| &triggered.effects)
        .expect("Season should retain its end-step destroy trigger")
}

#[test]
fn season_keeps_the_couldnt_attack_exception_as_one_typed_filter() {
    let definition = parse_oracle_card_definition("Season of the Witch");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let destroy = end_step_destroy_program(&definition)
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| effect.downcast_ref::<DestroyEffect>())
        .expect("typed destroy effect");
    let ChooseSpec::All(filter) = &destroy.spec else {
        panic!("expected an all-objects destroy set: {destroy:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
    assert!(filter.excluded_card_types.is_empty(), "{filter:#?}");
    assert!(filter.untapped && filter.didnt_attack_this_turn);
    assert!(filter.could_have_attacked_this_turn, "{filter:#?}");
}

#[test]
fn season_destroys_only_nonattackers_that_could_have_attacked() {
    let definition = parse_oracle_card_definition("Season of the Witch");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let eligible = game.create_object_from_definition(
        &creature("Eligible Nonattacker", false),
        alice,
        Zone::Battlefield,
    );
    let defender = game.create_object_from_definition(
        &creature("Defender Nonattacker", true),
        alice,
        Zone::Battlefield,
    );
    let summoning_sick = game.create_object_from_definition(
        &creature("New Nonattacker", false),
        alice,
        Zone::Battlefield,
    );
    game.remove_summoning_sickness(eligible);
    game.remove_summoning_sickness(defender);
    game.set_summoning_sick(summoning_sick);
    let eligible_stable = game.object(eligible).expect("eligible creature").stable_id;

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        end_step_destroy_program(&definition),
        None,
        &[],
    )
    .expect("Season end-step trigger should resolve");

    assert_eq!(
        game.find_object_by_stable_id(eligible_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        game.object(defender).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        game.object(summoning_sick).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
}
