use super::*;
#[test]
fn target_self_damage_hits_the_targeted_creature_instead_of_the_spell() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Wisecrack")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature deals damage equal to its power to itself. If that creature is attacking, Wisecrack deals 2 damage to that creature's controller.").unwrap();
    for attacking in [false, true] { for counters in [0, 2] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id; let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Target")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(4, 8))
            .with_ability(Ability::static_ability(crate::static_abilities::StaticAbility::lifelink())).build();
        let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        if counters > 0 { game.add_counters(target, crate::CounterType::PlusOnePlusOne, counters); }
        if attacking {
            let mut combat = crate::combat_state::CombatState::default();
            combat.attackers.push(crate::combat_state::AttackerInfo { creature: target, target: crate::combat_state::AttackTarget::Player(alice) });
            game.combat = Some(combat);
        }
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.snapshot_targets(&game);
        for effect in definition.spell_effect.as_ref().unwrap() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.damage_on(target), 4 + counters);
        assert_eq!(game.player(bob).unwrap().life, 24 + counters as i32 - if attacking { 2 } else { 0 });
        assert_eq!(game.player(alice).unwrap().life, 20);
    }}
}

#[test]
fn target_self_damage_renders_reflexive_recipient_and_attack_condition() {
    let text = "Target creature deals damage equal to its power to itself. If that creature is attacking, Wisecrack deals 2 damage to that creature's controller.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Wisecrack")
        .card_types(vec![CardType::Instant]).parse_text(text).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join(" "), text);
}
