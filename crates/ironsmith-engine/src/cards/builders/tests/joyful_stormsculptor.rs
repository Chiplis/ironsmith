#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn joyful_stormsculptor_keeps_convoke_and_the_shared_opponent_battle_damage_set() {
    let definition = parse_oracle_card_definition("Joyful Stormsculptor");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "When this creature enters, create two 1/1 blue and red Elemental creature tokens.",
            "Whenever you cast a spell that has convoke, this creature deals 1 damage to each opponent and each battle they protect.",
        ]
        .map(str::to_string),
        "{definition:#?}"
    );

    let triggered = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| {
            triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>()
                .is_some()
        })
        .expect("Joyful Stormsculptor must retain its spell-cast trigger");
    let cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .expect("the selected trigger must be spell cast");
    let filter = cast
        .filter
        .as_ref()
        .expect("the spell-cast trigger must retain its convoke filter");
    assert_eq!(cast.caster, PlayerFilter::You);
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert_eq!(
        filter.static_abilities,
        vec![crate::static_abilities::StaticAbilityId::Convoke]
    );

    fn protected_battle_filter(effect: &Effect) -> Option<ObjectFilter> {
        if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>()
            && for_each.filter.card_types == [CardType::Battle]
        {
            return Some(for_each.filter.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = protected_battle_filter(child);
            }
        });
        found
    }
    let protected_battle = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(protected_battle_filter)
        .expect("the damage program must retain its protected-Battle iteration");
    assert_eq!(
        protected_battle.protected_by,
        Some(PlayerFilter::IteratedPlayer),
        "{:#?}",
        triggered.effects
    );
}

#[test]
fn joyful_stormsculptor_damages_each_opponent_and_only_the_battle_they_protect() {
    let definition = parse_oracle_card_definition("Joyful Stormsculptor");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::SpellCastTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("spell-cast trigger");

    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let battle = |name, siege| {
        let builder = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Battle])
            .defense(5);
        if siege {
            builder.subtypes(vec![Subtype::Siege]).build()
        } else {
            builder.build()
        }
    };
    let bob_battle =
        game.create_object_from_definition(&battle("Bob's Battle", true), alice, Zone::Battlefield);
    let cara_battle = game.create_object_from_definition(
        &battle("Cara's Battle", true),
        alice,
        Zone::Battlefield,
    );
    let unprotected_battle = game.create_object_from_definition(
        &battle("Unprotected Battle", false),
        alice,
        Zone::Battlefield,
    );
    assert!(game.set_battle_protector(bob_battle, bob));
    assert!(game.set_battle_protector(cara_battle, cara));
    assert_eq!(
        game.battle_protector(unprotected_battle),
        Some(alice),
        "a non-Siege Battle is protected by its controller, outside the opponent set"
    );

    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Joyful Stormsculptor trigger should resolve");

    assert_eq!(game.player(bob).map(|player| player.life), Some(19));
    assert_eq!(game.player(cara).map(|player| player.life), Some(19));
    assert_eq!(
        game.counter_count(bob_battle, crate::object::CounterType::Defense),
        4,
        "Bob's battle must be damaged once, not once per opponent"
    );
    assert_eq!(
        game.counter_count(cara_battle, crate::object::CounterType::Defense),
        4,
        "Cara's battle must be damaged once, not once per opponent"
    );
    assert_eq!(
        game.counter_count(unprotected_battle, crate::object::CounterType::Defense),
        5,
        "a Battle outside every iterated opponent's protected set must not be damaged"
    );
}
