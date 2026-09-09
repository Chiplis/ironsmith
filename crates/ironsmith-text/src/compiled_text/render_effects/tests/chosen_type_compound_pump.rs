use super::*;
const TEXT: &str = "Morph {1}{G}\nWhen this creature is turned face up, creatures of the creature type of your choice get +2/+2 and gain trample until end of turn.";
struct ChooseType {
    chosen: Subtype,
    calls: usize,
}
impl crate::decision::DecisionMaker for ChooseType {
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.calls += 1;
        vec![
            ctx.options
                .iter()
                .find(|o| o.legal && o.description == self.chosen.to_string())
                .unwrap()
                .index,
        ]
    }
}
#[test]
fn chosen_type_compound_pump_restricts_both_changes_to_the_chosen_type() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Tribal Forcemage")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf, Subtype::Wizard])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .parse_text(TEXT)
            .unwrap();
    let trigger = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .unwrap();
    for (chosen, source_leaves) in [Subtype::Elf, Subtype::Goblin]
        .into_iter()
        .flat_map(|chosen| [false, true].map(|leaves| (chosen, leaves)))
    {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source_snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let mut creatures = vec![(source, Subtype::Elf, 1)];
        if source_leaves {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            creatures.clear();
        }
        for player in [alice, bob] {
            for subtype in [Subtype::Elf, Subtype::Goblin] {
                let card =
                    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature Probe")
                        .card_types(vec![CardType::Creature])
                        .subtypes(vec![subtype])
                        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
                        .build();
                creatures.push((
                    game.create_object_from_card(&card, player, Zone::Battlefield),
                    subtype,
                    3,
                ));
            }
        }
        let mut dm = ChooseType { chosen, calls: 0 };
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
        ctx.source_snapshot = Some(source_snapshot);
        for effect in &trigger.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(dm.calls, 1, "the resolution must ask for one creature type");
        game.refresh_continuous_state();
        for (id, subtype, base) in &creatures {
            let matches = *subtype == chosen;
            assert_eq!(
                game.calculated_power(*id),
                Some(base + if matches { 2 } else { 0 })
            );
            assert_eq!(
                game.calculated_toughness(*id),
                Some(base + if matches { 2 } else { 0 })
            );
            assert_eq!(
                game.current_has_static_ability_id(
                    *id,
                    crate::static_abilities::StaticAbilityId::Trample
                ),
                matches
            );
        }
        let late_card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Late Creature")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![chosen])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .build();
        let late = game.create_object_from_card(&late_card, alice, Zone::Battlefield);
        game.refresh_continuous_state();
        assert_eq!(game.calculated_power(late), Some(3));
        assert!(!game.current_has_static_ability_id(
            late,
            crate::static_abilities::StaticAbilityId::Trample
        ));
        creatures.push((late, chosen, 3));
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        for (id, _, base) in creatures {
            assert_eq!(game.calculated_power(id), Some(base));
            assert_eq!(game.calculated_toughness(id), Some(base));
            assert!(!game.current_has_static_ability_id(
                id,
                crate::static_abilities::StaticAbilityId::Trample
            ));
        }
    }
}
#[test]
fn chosen_type_compound_pump_renders_the_choice_and_shared_duration() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Tribal Forcemage")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn chosen_type_compound_pump_also_scopes_keyword_removal() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Chosen Type Loss Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Creatures of the creature type of your choice get +2/+2 and lose flying until end of turn.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut creatures = Vec::new();
    for subtype in [Subtype::Elf, Subtype::Goblin] {
        let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Flying Creature")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .parse_text("Flying")
            .unwrap();
        creatures.push((
            game.create_object_from_definition(&card, alice, Zone::Battlefield),
            subtype,
        ));
    }
    let mut dm = ChooseType {
        chosen: Subtype::Elf,
        calls: 0,
    };
    let mut ctx =
        crate::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
    for effect in definition.spell_effect.as_ref().unwrap() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(dm.calls, 1);
    game.refresh_continuous_state();
    for (id, subtype) in &creatures {
        assert_eq!(
            game.calculated_power(*id),
            Some(if *subtype == Subtype::Elf { 5 } else { 3 })
        );
        assert_eq!(
            game.current_has_static_ability_id(
                *id,
                crate::static_abilities::StaticAbilityId::Flying
            ),
            *subtype != Subtype::Elf
        );
    }
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    for (id, _) in creatures {
        assert_eq!(game.calculated_power(id), Some(3));
        assert!(
            game.current_has_static_ability_id(
                id,
                crate::static_abilities::StaticAbilityId::Flying
            )
        );
    }
}
