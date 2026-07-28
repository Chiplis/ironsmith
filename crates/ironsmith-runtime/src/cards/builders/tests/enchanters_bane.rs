#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::CardDefinition;
use crate::decision::{AutoPassDecisionMaker, SelectFirstDecisionMaker};
use crate::effect::{Effect, Value};
use crate::effects::{
    DealDamageEffect, ExecuteWithSourceEffect, ExecutionContext, SacrificeEffect,
    SacrificeTargetEffect, UnlessPaysEffect, execute_effect,
};
use crate::mana::{ManaCost, ManaSymbol};
use crate::snapshot::ObjectSnapshot;

fn enchanters_bane_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Enchanter's Bane should have one triggered ability")
}

fn enchanters_bane_unless(definition: &CardDefinition) -> &UnlessPaysEffect {
    let triggered = enchanters_bane_trigger(definition);
    let [effect] = triggered.effects.segments[0].default_effects.as_slice() else {
        panic!(
            "the trigger should lower to one unless wrapper: {:#?}",
            triggered.effects
        );
    };
    effect
        .downcast_ref::<UnlessPaysEffect>()
        .expect("the trigger consequence should be an unless-payment effect")
}

fn tagged_damage_source(unless: &UnlessPaysEffect) -> (&TagKey, &TargetOnlyEffect) {
    let [source_declaration, _damage] = unless.effects.as_slice() else {
        panic!("the consequence should declare one source and deal damage: {unless:#?}");
    };
    let tagged = source_declaration
        .downcast_ref::<TaggedEffect>()
        .expect("the targeted damage source should receive a stable tag");
    let target_only = tagged
        .effect
        .downcast_ref::<TargetOnlyEffect>()
        .expect("the tagged source declaration should retain its target");
    (&tagged.tag, target_only)
}

#[test]
fn enchanters_bane_preserves_target_source_controller_value_and_exact_text() {
    let oracle = "At the beginning of your end step, target enchantment deals damage equal to its mana value to its controller unless that player sacrifices it.";
    let definition = parse_oracle_card_definition("Enchanter's Bane");

    assert_eq!(canonical_compiled_lines(&definition).join("\n"), oracle);

    let triggered = enchanters_bane_trigger(&definition);
    let [choice] = triggered.choices.as_slice() else {
        panic!("the ability should declare exactly one target: {triggered:#?}");
    };
    assert!(choice.is_target(), "{choice:#?}");
    assert_eq!(choice.count(), crate::effect::ChoiceCount::exactly(1));
    let ChooseSpec::Object(choice_filter) = choice.base() else {
        panic!("the declared target should be an enchantment: {choice:#?}");
    };
    assert_eq!(choice_filter.zone, Some(Zone::Battlefield));
    assert_eq!(choice_filter.card_types, [CardType::Enchantment]);

    let unless = enchanters_bane_unless(&definition);
    let (source_tag, target_only) = tagged_damage_source(unless);
    let ChooseSpec::Target(source_choice) = &target_only.target else {
        panic!("the source declaration should remain a target: {target_only:#?}");
    };
    let ChooseSpec::Object(source_filter) = source_choice.as_ref() else {
        panic!("the damage source should be the targeted enchantment: {target_only:#?}");
    };
    assert_eq!(source_filter.card_types, [CardType::Enchantment]);

    let execute = unless.effects[1]
        .downcast_ref::<ExecuteWithSourceEffect>()
        .expect("damage should execute with the selected enchantment as its source");
    assert_eq!(
        execute.source.base(),
        &ChooseSpec::Tagged(source_tag.clone()),
        "{execute:#?}"
    );
    let damage = execute
        .effect
        .downcast_ref::<DealDamageEffect>()
        .expect("the explicit-source body should deal damage");
    let Value::SurfaceHinted { value, .. } = &damage.amount else {
        panic!("the authored equal-to surface should be retained: {damage:#?}");
    };
    let Value::ManaValueOf(amount_source) = value.as_ref() else {
        panic!("the damage amount should be the source's mana value: {damage:#?}");
    };
    assert!(
        matches!(amount_source.base(), ChooseSpec::Source)
            || matches!(
                amount_source.base(),
                ChooseSpec::Tagged(tag) if tag == source_tag
            ),
        "the mana value must come from the targeted damage source: {damage:#?}"
    );
    let recipient_is_source_controller = match damage.target.base() {
        ChooseSpec::Player(PlayerFilter::ControllerOf(ObjectRef::Target)) => true,
        ChooseSpec::Player(PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)))
        | ChooseSpec::Player(PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag))) => {
            tag == source_tag
        }
        _ => false,
    };
    assert!(
        recipient_is_source_controller,
        "the damage recipient must be the targeted enchantment's controller: {damage:#?}"
    );
    assert!(
        matches!(
            &unless.player,
            PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag)) if tag == source_tag
        ),
        "the sacrifice decision belongs to that same controller: {unless:#?}"
    );

    let [cost] = unless.cost.costs() else {
        panic!("the unless clause should have one sacrifice cost: {unless:#?}");
    };
    let cost_effect = cost
        .effect_ref()
        .expect("the referential sacrifice should remain an executable cost");
    let sacrifices_tagged_source = cost_effect
        .downcast_ref::<SacrificeTargetEffect>()
        .is_some_and(|sacrifice| {
            sacrifice.target.base() == &ChooseSpec::Tagged(source_tag.clone())
        })
        || cost_effect
            .downcast_ref::<SacrificeEffect>()
            .is_some_and(|sacrifice| {
                sacrifice.filter == ObjectFilter::tagged(source_tag.clone())
                    && sacrifice.count == Value::Fixed(1)
                    && sacrifice.player == PlayerFilter::You
            });
    assert!(
        sacrifices_tagged_source,
        "the cost should sacrifice exactly the selected enchantment: {cost_effect:#?}"
    );
}

fn runtime_case(
    pay_with_sacrifice: bool,
) -> (crate::game_state::GameState, PlayerId, ObjectId, ObjectId) {
    let definition = parse_oracle_card_definition("Enchanter's Bane");
    let unless = enchanters_bane_unless(&definition).clone();
    let (source_tag, _) = tagged_damage_source(&unless);
    let source_tag = source_tag.clone();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let ability_source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Four-Mana Enchantment")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Battlefield);
    let target_snapshot =
        ObjectSnapshot::from_object(game.object(target).expect("target exists"), &game);

    if pay_with_sacrifice {
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(ability_source, alice, &mut dm)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.set_tagged_objects(source_tag, vec![target_snapshot]);
        execute_effect(&mut game, &Effect::new(unless), &mut ctx)
            .expect("the controller should be able to sacrifice the tagged enchantment");
    } else {
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(ability_source, alice, &mut dm)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.set_tagged_objects(source_tag, vec![target_snapshot]);
        execute_effect(&mut game, &Effect::new(unless), &mut ctx)
            .expect("declining the sacrifice should execute the damage");
    }

    (game, bob, ability_source, target)
}

#[test]
fn enchanters_bane_runtime_uses_the_target_for_damage_and_sacrifice() {
    let (damage_game, bob, ability_source, target) = runtime_case(false);
    assert_eq!(damage_game.player(bob).expect("Bob exists").life, 16);
    assert!(
        damage_game.source_dealt_damage_to_player_this_turn(target, bob),
        "the targeted enchantment, not Enchanter's Bane, must be the damage source"
    );
    assert!(
        !damage_game.source_dealt_damage_to_player_this_turn(ability_source, bob),
        "the ability source must not inherit the target's damage"
    );
    assert!(
        damage_game
            .objects_in_zone(Zone::Battlefield)
            .contains(&target),
        "declining the sacrifice should leave the target on the battlefield"
    );

    let (paid_game, bob, _ability_source, _target) = runtime_case(true);
    assert_eq!(paid_game.player(bob).expect("Bob exists").life, 20);
    assert!(
        paid_game
            .objects_in_zone(Zone::Graveyard)
            .into_iter()
            .any(|id| paid_game
                .object(id)
                .is_some_and(|object| { object.name == "Four-Mana Enchantment" })),
        "paying the unless cost should sacrifice that same targeted enchantment"
    );
}
