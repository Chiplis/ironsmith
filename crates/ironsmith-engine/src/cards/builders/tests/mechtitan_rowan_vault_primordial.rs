#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn frozen_cluster_keeps_exact_public_cards_json_surfaces() {
    let mechtitan = parse_oracle_card_definition("Mechtitan Core");
    let expected_mechtitan = "{5}, Exile this Vehicle and four other artifact creatures and/or Vehicles you control: Create Mechtitan, a legendary 10/10 Construct artifact creature token with flying, vigilance, trample, lifelink, and haste that's all colors. When that token leaves the battlefield, return all cards exiled with this Vehicle except this card to the battlefield tapped under their owners' control.\nCrew 2";
    assert_eq!(
        canonical_compiled_lines(&mechtitan).join("\n"),
        expected_mechtitan
    );
    let duplicate_face_row = parse_oracle_card_definition("Mechtitan Core // Mechtitan Core");
    assert_eq!(
        canonical_compiled_lines(&duplicate_face_row).join("\n"),
        expected_mechtitan,
        "the reversible duplicate row must compile through the same generic face text"
    );

    let rowan = parse_oracle_card_definition("Rowan, Scion of War");
    assert_eq!(
        canonical_compiled_lines(&rowan).join("\n"),
        "Menace\n{T}: Spells you cast this turn that are black and/or red cost {X} less to cast, where X is the amount of life you lost this turn. Activate only as a sorcery."
    );

    let vault = parse_oracle_card_definition("Vault 13: Dweller's Journey");
    assert_eq!(
        canonical_compiled_lines(&vault).join("\n"),
        "I — For each player, exile up to one other target enchantment or creature that player controls until this Saga leaves the battlefield.\nII — You gain 2 life and scry 2.\nIII — Return two cards exiled with this Saga to the battlefield under their owners' control and put the rest on the bottom of their owners' libraries."
    );

    let mist = parse_oracle_card_definition("Primordial Mist");
    assert_eq!(
        canonical_compiled_lines(&mist).join("\n"),
        "At the beginning of your end step, you may manifest the top card of your library.\nExile a face-down permanent you control face up: You may play that card this turn."
    );
}

#[test]
fn rowan_retains_color_domain_and_life_lost_value() {
    let definition = parse_oracle_card_definition("Rowan, Scion of War");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Rowan should have one activated reduction");
    let [effect] = activated.effects.flattened_default_effects() else {
        panic!("expected one typed reduction: {activated:#?}");
    };
    let reduction = effect
        .downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()
        .expect("typed spell-cost reduction");
    assert_eq!(
        reduction.filter.colors,
        Some(crate::color::ColorSet::BLACK.union(crate::color::ColorSet::RED))
    );
    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    assert!(matches!(
        reduction
            .generic_reduction
            .as_ref()
            .map(crate::effect::Value::unhinted),
        Some(crate::effect::Value::LifeLostThisTurn(PlayerFilter::You))
    ));
    assert!(reduction.applies_to_all_matching_this_turn);
}

#[test]
fn primordial_mist_cost_turns_the_selected_face_down_permanent_face_up() {
    let definition = parse_oracle_card_definition("Primordial Mist");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Primordial Mist should retain its activated permission");
    let costs = activated.mana_cost.costs();
    let [choose_cost, exile_cost] = costs else {
        panic!("expected choose-and-exile cost: {costs:#?}");
    };
    let choose = choose_cost
        .effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .expect("typed face-down permanent choice");
    let exile = exile_cost
        .effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
        .expect("typed exile cost");
    assert_eq!(choose.filter.face_down, Some(true));
    assert_eq!(choose.filter.controller, Some(PlayerFilter::You));
    assert!(exile.turn_face_up);
    assert!(!exile.face_down);
}

fn vault_chapter_three(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::SagaChapterTrigger>()
                    .is_some_and(|chapter| chapter.chapters == [3]) =>
            {
                Some(&triggered.effects)
            }
            _ => None,
        })
        .expect("Vault 13 should retain chapter III")
}

#[test]
fn vault_returns_exactly_two_and_bottoms_the_original_source_exiled_remainder() {
    let definition = parse_oracle_card_definition("Vault 13: Dweller's Journey");
    let program = vault_chapter_three(&definition);
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let card = CardDefinitionBuilder::new(CardId::new(), "Vault Partition Card")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let mut tracked = Vec::new();
    let mut source_exiled = Vec::new();
    for owner in [alice, bob, alice, bob] {
        let id = game.create_object_from_definition(&card, owner, Zone::Exile);
        let object = game.object(id).expect("linked exile fixture");
        tracked.push(object.stable_id);
        source_exiled.push(crate::ObjectSnapshot::from_object(object, &game));
        game.add_exiled_with_source_link(source, id);
    }

    let chapter_event = crate::events::RawEvent::new(
        crate::events::other::CounterPlacedEvent::new(source, CounterType::Lore, 1),
        crate::provenance::ProvNodeId::default(),
    );
    let mut context = crate::effects::ExecutionContext::new_default(source, alice)
        .with_triggering_event(chapter_event);
    // Stack resolution installs the source-linked exile set on the execution
    // context. This direct program harness mirrors that boundary explicitly so
    // the remainder iteration observes the original set after two cards move.
    context.set_tagged_objects(crate::tag::SOURCE_EXILED_TAG, source_exiled);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("chapter III source-exile partition should resolve");

    let zones = tracked
        .into_iter()
        .map(|stable| {
            let id = game
                .find_object_by_stable_id(stable)
                .expect("moved card should retain stable identity");
            game.object(id).expect("moved card exists").zone
        })
        .collect::<Vec<_>>();
    assert_eq!(
        zones
            .iter()
            .filter(|zone| **zone == Zone::Battlefield)
            .count(),
        2,
        "chapter III must retain the exact selected set before partitioning the remainder: {zones:#?}",
    );
    assert_eq!(
        zones.iter().filter(|zone| **zone == Zone::Library).count(),
        2,
        "chapter III must bottom the stable-identity complement of the returned set: {zones:#?}",
    );
}
