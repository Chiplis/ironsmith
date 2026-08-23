#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::CounterType;

const ELSPETH_ORACLE: &str = "+2: You gain 1 life for each creature you control.\n−2: Create three 1/1 white Soldier creature tokens.\n−5: Destroy all other permanents except for lands and tokens.";
const HYDRA_ORACLE: &str = "This creature enters with X +1/+1 counters on it. If X is 5 or more, it enters with an additional X +1/+1 counters on it.\n{1}{R}, Remove a +1/+1 counter from this creature: It deals 1 damage to any target.";

fn elspeth_destroy_program(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated.effects.segments.iter().any(|segment| {
                    segment
                        .default_effects
                        .iter()
                        .any(|effect| effect.downcast_ref::<DestroyEffect>().is_some())
                }) =>
            {
                Some(&activated.effects)
            }
            _ => None,
        })
        .expect("Elspeth should retain her typed destroy loyalty ability")
}

#[test]
fn elspeth_tirel_keeps_both_destroy_exceptions_in_one_public_filter() {
    let definition = parse_oracle_card_definition("Elspeth Tirel");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        ELSPETH_ORACLE
    );

    let destroy = elspeth_destroy_program(&definition)
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| effect.downcast_ref::<DestroyEffect>())
        .expect("Elspeth should lower to a typed destroy effect");
    let ChooseSpec::All(filter) = &destroy.spec else {
        panic!("Elspeth should destroy one filtered all-object set: {destroy:#?}");
    };
    assert!(
        filter.other,
        "the source itself must be excluded: {filter:#?}"
    );
    assert!(filter.nontoken, "tokens must be excluded: {filter:#?}");
    assert_eq!(filter.excluded_card_types, [CardType::Land]);
    assert_eq!(
        filter.card_types,
        [
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ]
    );
}

fn simple_permanent(name: &str, card_type: CardType) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder.power_toughness(PowerToughness::fixed(2, 2)).build()
    } else {
        builder.build()
    }
}

#[test]
fn elspeth_tirel_destroys_only_other_nontoken_nonland_permanents() {
    let definition = parse_oracle_card_definition("Elspeth Tirel");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let artifact = game.create_object_from_definition(
        &simple_permanent("Ordinary Artifact", CardType::Artifact),
        alice,
        Zone::Battlefield,
    );
    let land = game.create_object_from_definition(
        &simple_permanent("Ordinary Land", CardType::Land),
        alice,
        Zone::Battlefield,
    );
    let token_definition = CardDefinitionBuilder::new(CardId::new(), "Creature Token")
        .token()
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let token = game.create_object_from_definition(&token_definition, alice, Zone::Battlefield);
    let artifact_stable = game.object(artifact).expect("artifact exists").stable_id;

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        elspeth_destroy_program(&definition),
        None,
        &[],
    )
    .expect("Elspeth's destroy loyalty ability should resolve");

    assert_eq!(
        game.object(source).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        game.object(land).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        game.object(token).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        game.find_object_by_stable_id(artifact_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Graveyard)
    );
}

#[test]
fn apocalypse_hydra_keeps_the_typed_x_threshold_publicly() {
    let definition = parse_oracle_card_definition("Apocalypse Hydra");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        HYDRA_ORACLE
    );

    let conditional = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.compiled_model(),
            _ => None,
        })
        .find(|model| {
            matches!(
                &model.payload,
                ironsmith_core::StaticAbilityPayload::Conditional { .. }
            )
        })
        .expect("Apocalypse Hydra should retain a conditional counter replacement");
    let ironsmith_core::StaticAbilityPayload::Conditional { ability, condition } =
        &conditional.payload
    else {
        unreachable!();
    };
    assert_eq!(condition, &ironsmith_core::Condition::XValueAtLeast(5));
    assert!(matches!(
        &ability.payload,
        ironsmith_core::StaticAbilityPayload::EntersWithCountersValue {
            counter: CounterType::PlusOnePlusOne,
            ..
        }
    ));
}

fn apocalypse_hydra_entry_counters(x_value: u32) -> u32 {
    let definition = parse_oracle_card_definition("Apocalypse Hydra");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.object_mut(source).expect("Hydra spell exists").x_value = Some(x_value);
    let stable = game.object(source).expect("Hydra spell exists").stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(source, alice).with_x(x_value));
    crate::game_loop::resolve_stack_entry(&mut game).expect("Hydra spell should resolve");
    let hydra = game
        .find_object_by_stable_id(stable)
        .expect("Hydra should enter the battlefield");
    game.object(hydra)
        .expect("Hydra should be on the battlefield")
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or_default()
}

#[test]
fn apocalypse_hydra_adds_the_second_x_counters_only_at_five_or_more() {
    assert_eq!(apocalypse_hydra_entry_counters(4), 4);
    assert_eq!(apocalypse_hydra_entry_counters(5), 10);
}

#[test]
fn x_threshold_entry_counter_surface_uses_the_authored_threshold() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Threshold Hydra")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .parse_text(
            "This creature enters with X +1/+1 counters on it. If X is 6 or more, it enters with an additional X +1/+1 counters on it.",
        )
        .expect("generic X-threshold entry-counter text should parse");

    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "This creature enters with X +1/+1 counters on it. If X is 6 or more, it enters with an additional X +1/+1 counters on it."
        ]
    );
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("XValueAtLeast") && debug.contains('6'),
        "{debug}"
    );
}
