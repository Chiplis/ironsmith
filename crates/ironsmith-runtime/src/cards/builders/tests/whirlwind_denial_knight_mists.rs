#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn find_nested<T: Clone + 'static>(effect: &crate::effect::Effect) -> Option<T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested::<T>(child);
        }
    });
    found
}

fn stack_spell(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(1),
        ]]))
        .build()
}

fn permanent(name: &str, subtype: Option<Subtype>) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2));
    if let Some(subtype) = subtype {
        builder = builder.subtypes(vec![subtype]);
    }
    builder.build()
}

fn zone_of_stable(game: &crate::GameState, stable_id: StableId) -> Zone {
    let current = game
        .find_object_by_stable_id(stable_id)
        .expect("object should remain findable by stable identity");
    game.object(current).expect("object should exist").zone
}

#[test]
fn whirlwind_denial_keeps_one_universal_spell_and_ability_domain() {
    let definition = parse_oracle_card_definition("Whirlwind Denial");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Whirlwind Denial"]
    );
    let program = definition.spell_effect.as_ref().expect("spell program");
    let for_each = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| find_nested::<crate::effects::ForEachObject>(effect))
        .unwrap_or_else(|| panic!("expected quantified Stack domain: {program:#?}"));

    assert_eq!(for_each.filter.zone, Some(Zone::Stack));
    assert_eq!(
        for_each.filter.stack_kind,
        Some(crate::filter::StackObjectKind::SpellOrAbility)
    );
    assert_eq!(for_each.filter.controller, Some(PlayerFilter::Opponent));
    assert!(!for_each.filter.has_mana_cost);
    assert!(for_each.filter.has_conjunctive_set_surface());
    assert!(
        for_each
            .effects
            .iter()
            .any(|effect| format!("{effect:#?}").contains("UnlessPaysEffect")),
        "each Stack object should receive its own counter-unless-payment: {for_each:#?}"
    );
}

#[test]
fn whirlwind_denial_counters_opposing_spells_activated_and_triggered_abilities() {
    let definition = parse_oracle_card_definition("Whirlwind Denial");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let opposing_spell =
        game.create_object_from_definition(&stack_spell("Opposing Spell"), bob, Zone::Stack);
    let opposing_spell_stable = game
        .object(opposing_spell)
        .expect("opposing spell exists")
        .stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(opposing_spell, bob));

    let activated_source = game.create_object_from_definition(
        &permanent("Opposing Activated Source", None),
        bob,
        Zone::Battlefield,
    );
    game.push_to_stack(crate::game_state::StackEntry::ability(
        activated_source,
        bob,
        vec![crate::effect::Effect::draw(1)],
    ));

    let triggered_source = game.create_object_from_definition(
        &permanent("Opposing Triggered Source", None),
        bob,
        Zone::Battlefield,
    );
    let triggering_event = crate::events::RawEvent::new(
        crate::events::AbilityActivatedEvent::new(triggered_source, bob, false),
        crate::provenance::ProvNodeId::default(),
    );
    game.push_to_stack(
        crate::game_state::StackEntry::ability(
            triggered_source,
            bob,
            vec![crate::effect::Effect::draw(1)],
        )
        .with_triggering_event(triggering_event),
    );

    let friendly_spell =
        game.create_object_from_definition(&stack_spell("Friendly Spell"), alice, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(friendly_spell, alice));

    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    for effect in definition
        .spell_effect
        .as_ref()
        .expect("spell program")
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Whirlwind Denial should resolve");
    }

    assert_eq!(
        game.stack
            .iter()
            .map(|entry| entry.object_id)
            .collect::<Vec<_>>(),
        vec![friendly_spell],
        "the controller's own spell is outside the universal opponent domain"
    );
    assert_eq!(
        zone_of_stable(&game, opposing_spell_stable),
        Zone::Graveyard
    );
    assert_eq!(
        game.object(activated_source)
            .expect("activated source remains")
            .zone,
        Zone::Battlefield
    );
    assert_eq!(
        game.object(triggered_source)
            .expect("triggered source remains")
            .zone,
        Zone::Battlefield
    );
}

fn knight_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Knight of the Mists should have its enters trigger")
}

#[test]
fn knight_of_the_mists_keeps_the_same_knight_target_on_no_regeneration_destroy() {
    let definition = parse_oracle_card_definition("Knight of the Mists");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Knight of the Mists"]
    );
    let triggered = knight_trigger(&definition);
    let [choice] = triggered.choices.as_slice() else {
        panic!("the enters trigger should choose one Knight: {triggered:#?}");
    };
    let ChooseSpec::Object(choice_filter) = choice.base() else {
        panic!("the enters trigger should target a Knight: {choice:#?}");
    };
    assert_eq!(choice_filter.subtypes, [Subtype::Knight]);

    let destroy = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| find_nested::<crate::effects::DestroyNoRegenerationEffect>(effect))
        .unwrap_or_else(|| panic!("expected typed no-regeneration destroy: {triggered:#?}"));
    let ChooseSpec::Target(destroy_target) = destroy.spec.unhinted() else {
        panic!("the destroy should consume the declared target: {destroy:#?}");
    };
    let ChooseSpec::Object(destroy_filter) = destroy_target.unhinted() else {
        panic!("the destroy target should remain an object: {destroy:#?}");
    };
    assert_eq!(destroy_filter, choice_filter);

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let knight = game.create_object_from_definition(
        &permanent("Legal Knight", Some(Subtype::Knight)),
        alice,
        Zone::Battlefield,
    );
    let non_knight = game.create_object_from_definition(
        &permanent("Ordinary Creature", None),
        alice,
        Zone::Battlefield,
    );
    let filter_ctx = game.filter_context_for(alice, None);
    assert!(choice_filter.matches(game.object(knight).unwrap(), &filter_ctx, &game));
    assert!(!choice_filter.matches(game.object(non_knight).unwrap(), &filter_ctx, &game));
}

fn resolve_knight_trigger(pay: bool) -> (Zone, u32, u32) {
    let definition = parse_oracle_card_definition("Knight of the Mists");
    let triggered = knight_trigger(&definition).clone();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let victim = game.create_object_from_definition(
        &permanent("Shielded Knight", Some(Subtype::Knight)),
        alice,
        Zone::Battlefield,
    );
    let victim_stable = game.object(victim).expect("victim exists").stable_id;
    game.add_regeneration_shield(victim, 1);
    if pay {
        game.player_mut(alice)
            .expect("Alice exists")
            .mana_pool
            .add(crate::mana::ManaSymbol::Blue, 1);
    }

    let mut pay_decisions = crate::decision::SelectFirstDecisionMaker;
    let mut decline_decisions = crate::decision::AutoPassDecisionMaker;
    let decisions: &mut dyn crate::decision::DecisionMaker = if pay {
        &mut pay_decisions
    } else {
        &mut decline_decisions
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, decisions)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(victim)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: triggered.choices[0].clone(),
            range: 0..1,
        }]);
    ctx.snapshot_targets(&game);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Knight of the Mists trigger should resolve");
    }
    drop(ctx);

    (
        zone_of_stable(&game, victim_stable),
        game.regenerated_this_turn_count(victim),
        game.regeneration_shield_count(victim),
    )
}

#[test]
fn knight_of_the_mists_payment_and_no_regeneration_branches_execute() {
    let (unpaid_zone, unpaid_regenerations, _) = resolve_knight_trigger(false);
    assert_eq!(unpaid_zone, Zone::Graveyard);
    assert_eq!(
        unpaid_regenerations, 0,
        "the unpaid typed destroy must bypass the regeneration shield"
    );

    let (paid_zone, paid_regenerations, paid_shields) = resolve_knight_trigger(true);
    assert_eq!(paid_zone, Zone::Battlefield);
    assert_eq!(
        paid_regenerations, 0,
        "paying should prevent the destroy rather than consume the shield"
    );
    assert_eq!(paid_shields, 1, "the paid branch must not touch the shield");
}
