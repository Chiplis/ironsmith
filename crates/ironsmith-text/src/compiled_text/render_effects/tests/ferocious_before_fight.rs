use super::*;

#[test]
fn ferocious_before_fight_pumps_the_selected_fighter_before_damage() {
    for separator in ["\n", " "] {
        let oracle = ["Target creature you control fights target creature you don't control.", "Ferocious — The creature you control gets +2/+2 until end of turn before it fights if you control a creature with power 4 or greater."].join(separator);
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Savage Punch")
                .card_types(vec![CardType::Sorcery])
                .parse_text(&oracle)
                .unwrap();
        for (power, own_qualifier, opponent_qualifier) in [
            (3, false, false),
            (3, true, false),
            (3, false, true),
            (4, false, false),
        ] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let creature = |power| {
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Fighter")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(power, 8))
                    .build()
            };
            let own = game.create_object_from_card(&creature(power), alice, Zone::Battlefield);
            let opponent = game.create_object_from_card(&creature(3), bob, Zone::Battlefield);
            let qualifier = if own_qualifier || opponent_qualifier {
                Some(game.create_object_from_card(
                    &creature(4),
                    if own_qualifier { alice } else { bob },
                    Zone::Battlefield,
                ))
            } else {
                None
            };
            let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let program = game.object(spell).unwrap().spell_effect_owned().unwrap();
            let requirements =
                crate::game_loop::extract_target_requirements_from_program_with_modes(
                    &game,
                    &program,
                    alice,
                    Some(spell),
                    None,
                );
            assert_eq!(requirements.len(), 2);
            let mut ctx =
                crate::effects::EffectContext::new_default(spell, alice).with_targets(vec![
                    crate::effects::ResolvedTarget::Object(own),
                    crate::effects::ResolvedTarget::Object(opponent),
                ]);
            ctx.snapshot_targets(&game);
            for segment in &program.segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            let bonus = if own_qualifier || power >= 4 { 2 } else { 0 };
            assert_eq!(game.current_power(own), Some(power + bonus));
            assert_eq!(
                game.damage_on(opponent),
                (power + bonus) as u32,
                "pump must precede the fight"
            );
            assert_eq!(game.damage_on(own), 3, "fight happens once");
            if let Some(qualifier) = qualifier {
                assert_eq!(game.damage_on(qualifier), 0);
                assert_eq!(game.current_power(qualifier), Some(4));
            }
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.refresh_continuous_state();
            assert_eq!(game.current_power(own), Some(power));
        }
    }
}

#[test]
fn conditional_bonus_before_fight_renders_bound_sequence() {
    let text = "Target creature you control fights target creature you don't control. The creature you control gets +2/+2 until end of turn before it fights if you control a creature with power 4 or greater.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Before Fight Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn conditional_bonus_before_fight_rejects_unrelated_fighter_or_delegated_choice() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Before Fight Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target creature you control fights target creature you don't control. The creature you control gets +3/+1 until end of turn before it fights if you control a creature with power 5 or greater.").unwrap();
    let effects = definition
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects();
    let render = |effects: &[Effect]| {
        crate::compiled_text::render_effects::effect_lists::describe_conditional_bonus_before_fight(
            &effects.iter().collect::<Vec<_>>(),
        )
    };
    assert!(render(effects).unwrap().contains("+3/+1"));
    let mut changed = effects.to_vec();
    let mut fight = changed
        .last()
        .unwrap()
        .downcast_ref::<crate::effects::FightEffect>()
        .unwrap()
        .clone();
    fight.creature1 = ChooseSpec::Source;
    *changed.last_mut().unwrap() = Effect::new(fight);
    assert!(
        render(&changed).is_none(),
        "a different fighter cannot inherit the bonus"
    );
    let mut changed = effects.to_vec();
    let mut declaration = changed[0]
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .unwrap()
        .clone();
    declaration.chooser = Some(PlayerFilter::Opponent);
    changed[0] = Effect::new(declaration);
    assert!(
        render(&changed).is_none(),
        "delegated target choices must stay explicit"
    );
}
