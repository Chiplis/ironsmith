#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const EZURIS_PREDATION_ORACLE: &str = "For each creature your opponents control, create a 4/4 green Phyrexian Beast creature token. Each of those tokens fights a different one of those creatures.";

fn ezuris_program(
    definition: &CardDefinition,
) -> &crate::effects::ForEachObjectCorrelatedResultEffect {
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Ezuri's Predation must compile as a resolving spell");
    let [segment] = program.segments.as_slice() else {
        panic!("the linked two-sentence program must lower atomically: {program:#?}");
    };
    let [effect] = segment.default_effects.as_slice() else {
        panic!("the linked program must contain one correlated effect: {segment:#?}");
    };
    effect
        .downcast_ref::<crate::effects::ForEachObjectCorrelatedResultEffect>()
        .expect("the result consumer must retain its per-source correlation")
}

fn creature_definition(card_id: u32, name: &str, power: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(card_id), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, 10))
        .build()
}

#[test]
fn ezuris_predation_renders_exact_oracle_text() {
    let definition = parse_oracle_card_definition("Ezuri's Predation");
    assert_eq!(
        canonical_compiled_lines(&definition).join(" "),
        EZURIS_PREDATION_ORACLE
    );
}

#[test]
fn ezuris_predation_retains_exact_source_result_and_distinct_fight_bindings() {
    let definition = parse_oracle_card_definition("Ezuri's Predation");
    let correlated = ezuris_program(&definition);
    assert_eq!(correlated.filter.zone, Some(Zone::Battlefield));
    assert_eq!(
        correlated.filter.card_types.as_slice(),
        [CardType::Creature]
    );
    assert!(matches!(
        correlated.filter.controller.as_ref(),
        Some(PlayerFilter::Opponent | PlayerFilter::NotYou)
    ));

    let [producer_effect] = correlated.producer_effects.as_slice() else {
        panic!("each source must have one token producer: {correlated:#?}");
    };
    let producer = producer_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the created token must expose its exact result set");
    assert_eq!(producer.tag, correlated.result_tag);
    let create = producer
        .effect
        .downcast_ref::<crate::effects::CreateTokenEffect>()
        .expect("the producer must remain typed token creation");
    assert_eq!(create.count, crate::effect::Value::Fixed(1));
    assert_eq!(create.controller, PlayerFilter::You);

    assert_ne!(
        correlated.source_binding_tag, correlated.result_binding_tag,
        "the source creature and produced token need independent bindings"
    );
    let [consumer_effect] = correlated.consumer_effects.as_slice() else {
        panic!("each pair must have one fight consumer: {correlated:#?}");
    };
    let fight = consumer_effect
        .downcast_ref::<crate::effects::FightEffect>()
        .expect("the consumer must remain a typed fight");
    assert!(matches!(
        &fight.creature1,
        ChooseSpec::Tagged(tag) if tag == &correlated.result_binding_tag
    ));
    assert!(matches!(
        &fight.creature2,
        ChooseSpec::Tagged(tag) if tag == &correlated.source_binding_tag
    ));
}

#[test]
fn ezuris_predation_creates_one_token_per_opposing_creature_and_fights_distinctly() {
    let definition = parse_oracle_card_definition("Ezuri's Predation");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Ezuri's Predation must have a spell effect");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let first_opponent = game.create_object_from_definition(
        &creature_definition(96_400, "First Opponent", 1),
        bob,
        Zone::Battlefield,
    );
    let second_opponent = game.create_object_from_definition(
        &creature_definition(96_401, "Second Opponent", 2),
        bob,
        Zone::Battlefield,
    );
    let friendly = game.create_object_from_definition(
        &creature_definition(96_402, "Friendly Creature", 3),
        alice,
        Zone::Battlefield,
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let mut damage_events = Vec::new();
    for effect in program.flattened_default_effects() {
        let outcome = crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Ezuri's Predation must resolve its complete linked program");
        damage_events.extend(
            outcome
                .events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::DamageEvent>())
                .cloned(),
        );
    }

    let tokens = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                matches!(object.kind, crate::object::ObjectKind::Token)
                    && object.subtypes.contains(&Subtype::Phyrexian)
                    && object.subtypes.contains(&Subtype::Beast)
                    && game.controller_of(object) == alice
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        tokens.len(),
        2,
        "one token must be created per opponent creature"
    );
    assert_eq!(game.damage_on(first_opponent), 4);
    assert_eq!(game.damage_on(second_opponent), 4);
    assert_eq!(game.damage_on(friendly), 0);

    let mut token_damage = tokens
        .iter()
        .map(|token| game.damage_on(*token))
        .collect::<Vec<_>>();
    token_damage.sort_unstable();
    assert_eq!(
        token_damage,
        vec![1, 2],
        "the two tokens must fight the two different opposing creatures"
    );
    assert!(
        damage_events.iter().all(|event| {
            !matches!(
                event.target,
                crate::events::DamageTarget::Object(target) if target == event.source
            )
        }),
        "no produced token may be rebound as both sides of its own fight: {damage_events:#?}"
    );
}
