#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Flying\nWhenever this creature attacks, the next instant or sorcery spell you cast this turn costs {X} less to cast, where X is this creature's power as this ability resolves.";

fn attack_reduction(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected attack-triggered cost reduction")
}

#[test]
fn maelstrom_muse_keeps_next_spell_and_resolution_time_power_surface() {
    let definition = parse_oracle_card_definition("Maelstrom Muse");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let effects = attack_reduction(&definition)
        .effects
        .flattened_default_effects();
    let [effect] = effects else {
        panic!("expected one reduction effect: {definition:#?}");
    };
    let reduction = effect
        .downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()
        .expect("typed next-spell reduction");
    assert!(!reduction.applies_to_all_matching_this_turn);
    let dynamic = reduction
        .generic_reduction
        .as_ref()
        .expect("dynamic source-power reduction");
    assert!(dynamic.has_surface_hint(ironsmith_core::ValueSurfaceHint::AsThisAbilityResolves));
    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    assert!(reduction.filter.card_types.contains(&CardType::Instant));
    assert!(reduction.filter.card_types.contains(&CardType::Sorcery));
}

#[test]
fn next_spell_reduction_freezes_source_power_at_resolution_and_matches_once() {
    let definition = parse_oracle_card_definition("Maelstrom Muse");
    let triggered = attack_reduction(&definition).clone();
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.object_mut(source).expect("source exists").base_power =
        Some(crate::card::PtValue::Fixed(5));
    assert_eq!(game.current_power(source), Some(5));

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("attack trigger should resolve");

    let [temporary] = game.effect_store.temporary_spell_cost_reductions.as_slice() else {
        panic!("expected one temporary reduction");
    };
    assert_eq!(temporary.remaining_uses, 1);
    assert!(!temporary.applies_to_all_matching_this_turn);
    assert!(matches!(
        temporary.generic_reduction.as_ref(),
        Some(crate::effect::Value::Fixed(5))
    ));

    // The registered reduction is frozen at trigger resolution, so a later
    // power change cannot alter the amount paid for the next matching spell.
    game.object_mut(source).expect("source exists").base_power =
        Some(crate::card::PtValue::Fixed(1));
    let instant_definition = CardDefinitionBuilder::new(CardId::new(), "Muse Instant Probe")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(7),
        ]]))
        .card_types(vec![CardType::Instant])
        .build();
    let creature_definition = CardDefinitionBuilder::new(CardId::new(), "Muse Creature Probe")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(7),
        ]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let second_instant_definition =
        CardDefinitionBuilder::new(CardId::new(), "Second Muse Instant Probe")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Generic(7),
            ]]))
            .card_types(vec![CardType::Instant])
            .build();
    let instant = game.create_object_from_definition(&instant_definition, alice, Zone::Hand);
    let creature = game.create_object_from_definition(&creature_definition, alice, Zone::Hand);
    let second_instant =
        game.create_object_from_definition(&second_instant_definition, alice, Zone::Hand);

    let instant_object = game.object(instant).expect("instant exists");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(
            &game,
            alice,
            instant_object,
            instant_object
                .mana_cost
                .as_ref()
                .expect("instant mana cost"),
        )
        .to_oracle(),
        "{2}",
    );
    let creature_object = game.object(creature).expect("creature exists");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(
            &game,
            alice,
            creature_object,
            creature_object
                .mana_cost
                .as_ref()
                .expect("creature mana cost"),
        )
        .to_oracle(),
        "{7}",
    );

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(crate::mana::ManaSymbol::Colorless, 7);
    let mut priority_state = crate::game_loop::PriorityLoopState::new(2);
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut priority_state,
        &crate::game_loop::PriorityResponse::PriorityAction(
            crate::decision::LegalAction::CastSpell {
                spell_id: instant,
                from_zone: Zone::Hand,
                casting_method: crate::alternative_cast::CastingMethod::Normal,
            },
        ),
    )
    .expect("the reduced instant should be castable");
    assert_eq!(
        game.effect_store.temporary_spell_cost_reductions[0].remaining_uses, 0,
        "the first matching spell must consume the reduction"
    );

    let second_instant_object = game.object(second_instant).expect("second instant exists");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(
            &game,
            alice,
            second_instant_object,
            second_instant_object
                .mana_cost
                .as_ref()
                .expect("second instant mana cost"),
        )
        .to_oracle(),
        "{7}",
        "only the next matching spell is reduced",
    );
}
