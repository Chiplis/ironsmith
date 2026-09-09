use super::*;
const TEXT: &str = "Kicker {8}{R}\nThis spell can't be countered.\nUrza's Rage deals 3 damage to any target. If this spell was kicked, instead it deals 10 damage to that permanent or player and the damage can't be prevented.";
#[test]
fn kicked_damage_replacement_preserves_announced_target_and_prevention() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Urza's Rage")
        .card_types(vec![CardType::Instant]).parse_text(TEXT).unwrap();
    for kicked in [false, true] { for prevent in [false, true] { for object_target in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id; let bob = game.players[1].id;
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Damage Target")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1, 20)).build();
        let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut paid = crate::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
        if kicked { paid.pay(0); }
        game.object_mut(spell).unwrap().optional_costs_paid = paid.clone();
        if prevent {
            let spec = if object_target { ChooseSpec::SpecificObject(target) } else { ChooseSpec::SpecificPlayer(bob) };
            let mut ctx = crate::effects::EffectContext::new_default(spell, alice);
            crate::effects::execute_effect(&mut game, &Effect::new(crate::effects::PreventAllDamageToTargetEffect::new(spec, Until::EndOfTurn)), &mut ctx).unwrap();
        }
        game.push_to_stack(crate::game_state::StackEntry::new(spell, alice)
            .with_optional_costs_paid(paid).with_targets(vec![if object_target { crate::game_state::Target::Object(target) } else { crate::game_state::Target::Player(bob) }]));
        {
            let mut ctx = crate::effects::EffectContext::new_default(target, bob);
            crate::effects::execute_effect(&mut game, &Effect::counter(ChooseSpec::SpecificObject(spell)), &mut ctx).unwrap();
        }
        assert!(game.stack.iter().any(|entry| entry.object_id == spell));
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        let damage = if kicked { 10 } else if prevent { 0 } else { 3 };
        assert_eq!(game.damage_on(target), if object_target { damage } else { 0 }, "kicked={kicked} prevent={prevent}");
        assert_eq!(game.player(bob).unwrap().life, 20 - if object_target { 0 } else { damage as i32 });
        assert_eq!(game.player(alice).unwrap().life, 20);
    }}}
}
#[test]
fn kicked_damage_replacement_renders_original_target_and_unpreventable_rider() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Urza's Rage")
        .card_types(vec![CardType::Instant]).parse_text(TEXT).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), TEXT);
}
