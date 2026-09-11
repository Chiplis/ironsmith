use super::*;
const TEXT: &str = "Trample, haste\nWhenever this creature attacks, attacking creatures get +1/+1 until end of turn. If a Food entered the battlefield under your control this turn, untap those creatures and they get an additional +2/+2 until end of turn.";
#[test]
fn conditional_attacker_bonus_includes_already_untapped_attackers() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Motivated Pony")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Horse])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(TEXT)
        .unwrap();
    let ability = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(trigger) => Some(trigger),
            _ => None,
        })
        .unwrap();
    for food_owner in [None, Some(0), Some(1)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let tapped = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let untapped = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let nonattacker = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.tap(source);
        game.tap(tapped);
        let mut combat = crate::combat_state::CombatState::default();
        combat.attackers = [source, tapped, untapped]
            .into_iter()
            .map(|creature| crate::combat_state::AttackerInfo {
                creature,
                target: crate::combat_state::AttackTarget::Player(bob),
            })
            .collect();
        game.combat = Some(combat);
        if let Some(index) = food_owner {
            let owner = game.players[index].id;
            let food = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Food")
                .card_types(vec![CardType::Artifact])
                .subtypes(vec![Subtype::Food])
                .build();
            let food = game.create_object_from_card(&food, owner, Zone::Hand);
            game.move_object_by_effect(food, Zone::Battlefield).unwrap();
        }
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        for effect in &ability.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state();
        let bonus = if food_owner == Some(0) { 3 } else { 1 };
        for id in [source, tapped, untapped] {
            let base = if id == source { 3 } else { 2 };
            assert_eq!(
                game.calculated_power(id),
                Some(base + bonus),
                "food={food_owner:?}, object={id:?}"
            );
            assert_eq!(game.calculated_toughness(id), Some(base + bonus));
            assert_eq!(game.is_tapped(id), food_owner != Some(0) && id != untapped);
        }
        assert_eq!(game.calculated_power(nonattacker), Some(2));
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        assert_eq!(game.calculated_power(source), Some(3));
        for id in [tapped, untapped, nonattacker] {
            assert_eq!(game.calculated_power(id), Some(2));
        }
    }
}
#[test]
fn conditional_attacker_bonus_renders_the_shared_plural_subject() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Motivated Pony")
        .card_types(vec![CardType::Creature])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn conditional_attacker_bonus_compaction_requires_the_entire_original_group() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Group bonus")
        .card_types(vec![CardType::Creature])
        .parse_text(TEXT)
        .unwrap();
    let ability = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(trigger) => Some(trigger),
            _ => None,
        })
        .unwrap();
    let original: Vec<Effect> = ability.effects.iter().cloned().collect();
    assert!(describe_group_pump_then_conditional_extra_bonus(&original).is_some());
    for change in 0..4 {
        let mut effects = original.clone();
        let mut conditional = effects[1]
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .unwrap()
            .clone();
        let mut sequence = conditional.if_true[0]
            .downcast_ref::<crate::effects::SequenceEffect>()
            .unwrap()
            .clone();
        match change {
            0 => {
                let mut capture = sequence.effects[0]
                    .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
                    .unwrap()
                    .clone();
                capture.filter.tapped = true;
                sequence.effects[0] = Effect::new(capture);
            }
            1 => {
                let mut untap = sequence.effects[1]
                    .downcast_ref::<crate::effects::UntapEffect>()
                    .unwrap()
                    .clone();
                untap.target = ChooseSpec::Source;
                sequence.effects[1] = Effect::new(untap);
            }
            2 => {
                let mut bonus = sequence.effects[2]
                    .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                    .unwrap()
                    .clone();
                bonus.target_spec = Some(ChooseSpec::Source);
                sequence.effects[2] = Effect::new(bonus);
            }
            _ => conditional.if_false.push(Effect::draw(1)),
        }
        conditional.if_true[0] = Effect::new(sequence);
        effects[1] = Effect::new(conditional);
        assert!(
            describe_group_pump_then_conditional_extra_bonus(&effects).is_none(),
            "change {change}"
        );
    }
}

#[test]
fn conditional_attacker_bonus_renders_structured_amounts() {
    let text = TEXT.replace("+1/+1", "+2/+1").replace("+2/+2", "+3/+4");
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Different group bonus")
            .card_types(vec![CardType::Creature])
            .parse_text(&text)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}
