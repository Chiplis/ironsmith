use super::*;
use crate::ability::AbilityKind;
use crate::alternative_cast::CastingMethod;
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::{CardId, PlayerId};
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[test]
fn black_life_permission_scan_is_needed_only_for_costs_with_a_black_symbol() {
    let plain_black = ManaCost::from_pips(vec![vec![ManaSymbol::Black]]);
    let hybrid_black = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2), ManaSymbol::Black]]);
    let phyrexian_black = ManaCost::from_pips(vec![vec![ManaSymbol::Black, ManaSymbol::Life(2)]]);
    let nonblack_x = ManaCost::from_pips(vec![vec![ManaSymbol::X], vec![ManaSymbol::Green]]);

    assert!(mana_cost_has_black_symbol(&plain_black));
    assert!(mana_cost_has_black_symbol(&hybrid_black));
    assert!(mana_cost_has_black_symbol(&phyrexian_black));
    assert!(!mana_cost_has_black_symbol(&nonblack_x));
}

#[test]
fn absent_optional_life_reduction_skips_dirty_layered_battlefield_scan() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardBuilder::new(CardId::new(), "Layered Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creatures = (0..96)
        .map(|_| game.create_object_from_card(&creature, alice, Zone::Battlefield))
        .collect::<Vec<_>>();
    for card_type in [CardType::Artifact, CardType::Enchantment] {
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                creatures[0],
                alice,
                crate::continuous::EffectTarget::AllCreatures,
                crate::continuous::Modification::AddCardTypes(vec![card_type]),
            ));
    }

    let spell = CardBuilder::new(CardId::new(), "Stack Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
    game.refresh_continuous_state();
    game.object_mut(spell_id)
        .expect("spell should exist")
        .optional_costs_paid = Default::default();

    let before = game.work_counters();
    let costs =
        optional_life_cost_reduction_costs_for_cast(&game, alice, spell_id, &CastingMethod::Normal);
    let after = game.work_counters();

    assert!(costs.is_empty());
    assert_eq!(
        after.characteristics_full_recomputes, before.characteristics_full_recomputes,
        "absence should be established from the sparse modifier-source scan"
    );
    assert_eq!(
        after.dependency_sorts, before.dependency_sorts,
        "an absent optional-life reducer must not sort layers once per permanent"
    );
}

#[test]
fn cost_presence_payment_calculation_reuses_view_for_minimum_spell_mana_scan() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let creature = CardBuilder::new(CardId::new(), "Cost Scan Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let mut sources = Vec::new();
    for _ in 0..32 {
        sources.push(game.create_object_from_card(&creature, alice, Zone::Battlefield));
    }
    for &source in sources.iter().take(6) {
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                source,
                alice,
                crate::continuous::EffectTarget::AllPermanents,
                crate::continuous::Modification::AddCardTypes(vec![CardType::Artifact]),
            ));
    }

    let base_cost = ManaCost::from_pips(vec![vec![ManaSymbol::Green]]);
    let spell = CardBuilder::new(CardId::new(), "Cost Scan Spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(base_cost.clone())
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    game.refresh_continuous_state();

    let before_public_query = game.work_counters();
    assert_eq!(game.minimum_total_spell_mana_payment(), None);
    let after_public_query = game.work_counters();
    assert_eq!(
        after_public_query.dependency_sorts, before_public_query.dependency_sorts,
        "the public minimum-spell-mana query should use the derived-view presence scan"
    );

    let before_payment = game.work_counters();
    let payment_adjusted = calculate_effective_mana_cost_for_payment_with_chosen_targets(
        &game,
        alice,
        game.object(spell_id).expect("spell should exist"),
        &base_cost,
        &[],
    );
    let after_payment = game.work_counters();

    assert_eq!(payment_adjusted, base_cost);
    assert_eq!(
        after_payment.dependency_sorts, before_payment.dependency_sorts,
        "payment-stage minimum-spell-mana discovery should reuse the cost view"
    );

    let before_final_validation = game.work_counters();
    let final_adjusted = calculate_effective_mana_cost_with_chosen_targets(
        &game,
        alice,
        game.object(spell_id).expect("spell should exist"),
        &base_cost,
        &[],
    );
    let after_final_validation = game.work_counters();

    assert_eq!(final_adjusted, base_cost);
    assert_eq!(
        after_final_validation.dependency_sorts, before_final_validation.dependency_sorts,
        "final cast validation should reuse the cost view instead of sorting once per permanent"
    );
}

#[test]
fn mana_search_uses_snapshotted_life_capacity_in_both_source_count_paths() {
    let game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let policy = game.mana_spend_policy(alice, None);
    let pips = vec![vec![ManaSymbol::Life(2)]];
    let pool = crate::player::ManaPool::default();
    let sources = Vec::<AvailableManaSource>::new();

    let small_search = |max_life_payment| {
        can_pay_expanded_pips(
            &game,
            alice,
            &pips,
            0,
            pool.clone(),
            &sources,
            0,
            0,
            max_life_payment,
            &policy,
            None,
            &mut std::collections::HashSet::new(),
        )
    };
    let large_search = |max_life_payment| {
        can_pay_expanded_pips_large_source_count(
            &game,
            alice,
            &pips,
            0,
            pool.clone(),
            &sources,
            &mut [],
            0,
            max_life_payment,
            &policy,
            None,
        )
    };

    assert!(!small_search(1));
    assert!(small_search(2));
    assert!(!large_search(1));
    assert!(large_search(2));
}

#[cfg(ironsmith_runtime_parser_tests)]
fn cavern_hoard_dragon_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(119_956), "Cavern-Hoard Dragon")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(7)],
                vec![ManaSymbol::Red],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Dragon])
            .power_toughness(PowerToughness::fixed(6, 6))
            .parse_text(
                "This spell costs {X} less to cast, where X is the greatest number of artifacts an opponent controls.\nFlying, trample, haste\nWhenever this creature deals combat damage to a player, you create a Treasure token for each artifact that player controls.",
            )
            .expect("Cavern-Hoard Dragon should parse for cost runtime test")
}

#[cfg(ironsmith_runtime_parser_tests)]
fn create_artifact(game: &mut GameState, owner: PlayerId, name: &str) {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn cavern_hoard_dragon_cost_reduction_uses_greatest_opponent_artifact_count() {
    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    for idx in 0..7 {
        create_artifact(&mut game, alice, &format!("Alice Artifact {idx}"));
    }
    for idx in 0..2 {
        create_artifact(&mut game, bob, &format!("Bob Artifact {idx}"));
    }
    for idx in 0..5 {
        create_artifact(&mut game, charlie, &format!("Charlie Artifact {idx}"));
    }

    let def = cavern_hoard_dragon_definition();
    let dragon_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let dragon = game.object(dragon_id).expect("dragon should be in hand");
    let reduction = dragon
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.this_spell_cost_reduction(),
            _ => None,
        })
        .expect("Cavern-Hoard Dragon should have a this-spell cost reduction");

    assert_eq!(
        resolve_this_spell_cost_reduction_value(&game, alice, dragon, reduction),
        5,
        "cost reduction should use the greatest single opponent artifact count, not your artifacts or all opponents combined"
    );

    let adjusted = apply_spell_cost_modifiers(
        &game,
        alice,
        dragon,
        dragon
            .mana_cost
            .as_ref()
            .expect("dragon should have a mana cost"),
        0,
        &[],
        &CastingMethod::Normal,
    );
    assert_eq!(
        adjusted.to_oracle(),
        "{2}{R}{R}",
        "{{7}}{{R}}{{R}} reduced by the greatest opponent artifact count of five should cost {{2}}{{R}}{{R}}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn cavern_hoard_dragon_cost_reduction_is_zero_without_opponent_artifacts() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    for idx in 0..3 {
        create_artifact(&mut game, alice, &format!("Alice Artifact {idx}"));
    }

    let def = cavern_hoard_dragon_definition();
    let dragon_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let dragon = game.object(dragon_id).expect("dragon should be in hand");
    let reduction = dragon
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.this_spell_cost_reduction(),
            _ => None,
        })
        .expect("Cavern-Hoard Dragon should have a this-spell cost reduction");

    assert_eq!(
        resolve_this_spell_cost_reduction_value(&game, alice, dragon, reduction),
        0,
        "artifacts controlled by Cavern-Hoard Dragon's caster must not reduce its cost"
    );
}
