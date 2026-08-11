#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::{ExecutionContext, ResolvedTarget};

fn damage_payload_through_bookkeeping(
    effect: &crate::effect::Effect,
) -> Option<&crate::effects::DealDamageEffect> {
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some(damage);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return damage_payload_through_bookkeeping(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return damage_payload_through_bookkeeping(&with_id.effect);
    }
    if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        return damage_payload_through_bookkeeping(&with_source.effect);
    }
    None
}

#[test]
fn kicked_unpreventable_damage_is_replacement_local_and_renders_exactly() {
    let name = "Urza's Rage";
    let oracle = oracle_text_by_name()
        .get(name)
        .expect("Urza's Rage should be present in cards.json");
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .parse_text(format!(
            "Mana cost: {{2}}{{R}}\nType: Instant\nFirst printed set: Invasion\n{oracle}"
        ))
        .expect("the authoritative metadata-backed payload should parse");
    let [segment] = definition
        .spell_effect
        .as_ref()
        .expect("Urza's Rage should have a spell-resolution program")
        .segments
        .as_slice()
    else {
        panic!("Urza's Rage should lower to one replacement segment");
    };
    let [default_effect] = segment.default_effects.as_slice() else {
        panic!("Urza's Rage should have one default damage action: {segment:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("Urza's Rage should have one kicked replacement: {segment:#?}");
    };
    let [replacement_effect] = branch.replacement_effects.as_slice() else {
        panic!("Urza's Rage should have one replacement damage action: {branch:#?}");
    };
    let default_damage = damage_payload_through_bookkeeping(default_effect)
        .expect("the default branch should retain typed damage");
    let replacement_damage = damage_payload_through_bookkeeping(replacement_effect)
        .expect("the kicked branch should retain typed damage");

    assert_eq!(default_damage.amount, Value::Fixed(3));
    assert!(
        !default_damage.unpreventable,
        "the ordinary 3 damage remains preventable: {default_damage:#?}"
    );
    assert_eq!(replacement_damage.amount, Value::Fixed(10));
    assert!(
        replacement_damage.unpreventable,
        "only the kicked replacement receives the prevention rider: {replacement_damage:#?}"
    );
    assert_eq!(default_damage.target, replacement_damage.target);
    assert!(
        matches!(
            branch.condition,
            crate::effect::Condition::ThisSpellWasKicked
                | crate::effect::Condition::TurnHistory(
                    ironsmith_core::TurnHistoryCondition::SourceWasKicked { .. }
                )
        ),
        "the replacement must retain a typed kicked condition: {branch:#?}"
    );
    assert!(branch.leading_instead_surface);

    let compiled = canonical_compiled_lines(&definition);
    assert_eq!(compiled.join("\n"), oracle.as_str());
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "{name} must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

#[test]
fn summary_judgment_addendum_damages_the_original_creature_target() {
    let definition = parse_oracle_card_definition("Summary Judgment");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;

    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Judgment Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 8))
        .build();
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Battlefield);
    game.tap(target);

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut cast_facts = crate::cost::OptionalCostsPaid::default();
    cast_facts.mark_label_paid("CastDuringYourMainPhase");
    game.object_mut(spell)
        .expect("Summary Judgment should be on the stack")
        .optional_costs_paid = cast_facts.clone();
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(target)])
            .with_optional_costs_paid(cast_facts),
    );

    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Summary Judgment's addendum branch should resolve");

    assert_eq!(
        game.damage_on(target),
        5,
        "addendum must replace 3 with 5 while retaining the announced creature target"
    );
    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        20,
        "the replacement must not invent a new player target"
    );
    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        20,
        "the replacement must not invent a new player target"
    );
}

#[test]
fn slaying_fire_adamant_damages_the_original_target() {
    let definition = parse_oracle_card_definition("Slaying Fire");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Slaying Fire Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 8))
        .build();
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Battlefield);

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.object_mut(spell)
        .expect("Slaying Fire should be on the stack")
        .mana_spent_to_cast = crate::player::ManaPool {
        red: 3,
        ..Default::default()
    };
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(target)]),
    );

    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Slaying Fire's adamant branch should resolve");

    assert_eq!(
        game.damage_on(target),
        4,
        "adamant must replace 3 with 4 while retaining the announced target"
    );
    assert_eq!(game.player(alice).expect("Alice should exist").life, 20);
    assert_eq!(game.player(bob).expect("Bob should exist").life, 20);
}

fn resolve_named_damage_replacement_through_prevention(
    name: &str,
    graveyard_cards: usize,
    attacked: bool,
) -> i32 {
    let definition = parse_oracle_card_definition(name);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    for index in 0..graveyard_cards {
        let fodder = CardDefinitionBuilder::new(
            CardId::from_raw(98_700 + index as u32),
            format!("Replacement Graveyard Card {index}"),
        )
        .card_types(vec![CardType::Creature])
        .build();
        game.create_object_from_definition(&fodder, alice, Zone::Graveyard);
    }
    if attacked {
        game.turn_store
            .turn_history
            .players_attacked_this_turn
            .insert(alice);
    }

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
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
        .expect("the source-specific prevention shield should register");
    }
    {
        let mut resolution_ctx = ExecutionContext::new(spell, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut resolution_ctx,
            alice,
            spell,
            definition
                .spell_effect
                .as_ref()
                .expect("the damage spell should have a resolution program"),
            None,
            &[],
        )
        .expect("the damage spell should resolve");
    }
    game.player(bob).expect("Bob should exist").life
}

#[test]
fn raid_and_threshold_replacements_are_unpreventable_only_when_their_conditions_hold() {
    assert_eq!(
        resolve_named_damage_replacement_through_prevention("Arrow Storm", 0, false),
        20,
        "without raid, the ordinary 4 damage must be stopped by the shield"
    );
    assert_eq!(
        resolve_named_damage_replacement_through_prevention("Arrow Storm", 0, true),
        15,
        "with raid, the replacement's 5 damage must bypass the shield"
    );
    assert_eq!(
        resolve_named_damage_replacement_through_prevention("Lightning Surge", 6, false),
        20,
        "below threshold, the ordinary 4 damage must be stopped by the shield"
    );
    assert_eq!(
        resolve_named_damage_replacement_through_prevention("Lightning Surge", 7, false),
        14,
        "at threshold, the replacement's 6 damage must bypass the shield"
    );
}

fn resolve_urzas_rage_through_prevention(kicked: bool) -> i32 {
    let definition = parse_oracle_card_definition("Urza's Rage");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
    if kicked {
        paid.pay(0);
    }
    game.object_mut(spell)
        .expect("Urza's Rage should exist on the stack")
        .optional_costs_paid = paid.clone();

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
        .expect("the source-specific prevention shield should register");
    }
    {
        let mut resolution_ctx = ExecutionContext::new(spell, alice, &mut decision_maker)
            .with_optional_costs_paid(paid)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut resolution_ctx,
            alice,
            spell,
            definition
                .spell_effect
                .as_ref()
                .expect("Urza's Rage should have a resolution program"),
            None,
            &[],
        )
        .expect("Urza's Rage should resolve through its replacement program");
    }
    game.player(bob).expect("Bob should exist").life
}

#[test]
fn kicker_selects_only_the_unpreventable_ten_damage_branch() {
    assert_eq!(
        resolve_urzas_rage_through_prevention(false),
        20,
        "the prevention shield must stop the ordinary 3 damage"
    );
    assert_eq!(
        resolve_urzas_rage_through_prevention(true),
        10,
        "the kicked replacement must deal 10 damage through prevention"
    );
}
