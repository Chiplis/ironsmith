use super::*;

const COCOON_ORACLE: &str = "Enchant creature you control\nWhen this Aura enters, tap enchanted creature and put three pupa counters on this Aura.\nEnchanted creature doesn't untap during your untap step if this Aura has a pupa counter on it.\nAt the beginning of your upkeep, remove a pupa counter from this Aura. If you can't, sacrifice it, put a +1/+1 counter on enchanted creature, and that creature gains flying.";

#[test]
fn attached_your_untap_uses_source_counters_and_source_controller() {
    for full_card in [false, true] {
        let oracle = if full_card { COCOON_ORACLE } else { "Enchant creature\nEnchanted creature doesn't untap during your untap step if this Aura has a pupa counter on it." };
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cocoon")
            .card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura]).parse_text(oracle).unwrap();
        assert!(!format!("{:?}", definition.spell_effect).contains("UntapEffect"), "a static untap restriction must not become an untap spell");
        for counters in [0, 1] {
            for aura_controller_index in [0, 1] {
                if full_card && aura_controller_index == 1 { continue; }
                for active_index in [0, 1] {
                    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let players = game.players.iter().map(|p| p.id).collect::<Vec<_>>();
                    game.turn.active_player = players[active_index];
                    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Attached fixture")
                        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
                    let target = game.create_object_from_card(&creature, players[0], Zone::Battlefield);
                    let unrelated = game.create_object_from_card(&creature, players[active_index], Zone::Battlefield);
                    let source = game.create_object_from_definition(&definition, players[aura_controller_index], Zone::Battlefield);
                    assert!(game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(target)));
                    let pupa = crate::object::CounterType::Named("pupa".into());
                    game.add_counters(source, pupa, counters);
                    game.add_counters(target, pupa, 1);
                    game.tap(target);
                    game.tap(unrelated);
                    game.refresh_continuous_state();
                    let restricted = counters > 0 && aura_controller_index == active_index;
                    assert_eq!(game.current_has_static_ability_id(target, crate::static_abilities::StaticAbilityId::DoesntUntap), restricted,
                        "full={full_card}, counters={counters}, aura={aura_controller_index}, active={active_index}");
                    crate::turn::execute_untap_step(&mut game);
                    assert_eq!(game.is_tapped(target), active_index != 0 || restricted);
                    assert!(!game.is_tapped(unrelated));
                }
            }
        }
    }
}

#[test]
fn cocoon_upkeep_waits_until_no_counter_can_be_removed_and_keeps_attachment_reference() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cocoon")
        .card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura]).parse_text(COCOON_ORACLE).unwrap();
    let triggers = definition.abilities.iter().filter_map(|a| if let AbilityKind::Triggered(t) = &a.kind { Some(t) } else { None }).collect::<Vec<_>>();
    assert_eq!(triggers.len(), 2);
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Enchanted fixture")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
    let target = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(target)));
    let pupa = crate::object::CounterType::Named("pupa".into());
    for step in 0..=4 {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_source_snapshot(snapshot);
        let trigger = triggers[usize::from(step > 0)];
        for segment in &trigger.effects.segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|error| panic!("step={step}: {error:?}, tags={:?}", ctx.tagged_objects.keys().collect::<Vec<_>>())); }
        }
        game.refresh_continuous_state();
        if step < 4 {
            assert_eq!(game.object(source).unwrap().counters.get(&pupa).copied().unwrap_or(0), 3 - step);
            assert_eq!(game.object(target).unwrap().counters.get(&pupa).copied().unwrap_or(0), 0);
            assert!(game.is_tapped(target));
            assert!(!game.current_has_static_ability_id(target, crate::static_abilities::StaticAbilityId::Flying));
            assert_eq!(game.current_power(target), Some(2));
        } else {
            assert!(game.object(source).is_none(), "sacrifice the Aura when removing a pupa counter fails");
            assert_eq!(game.object(target).unwrap().counters.get(&crate::object::CounterType::PlusOnePlusOne).copied().unwrap_or(0), 1);
            assert_eq!(game.current_power(target), Some(3));
            assert!(game.current_has_static_ability_id(target, crate::static_abilities::StaticAbilityId::Flying));
        }
    }
}
