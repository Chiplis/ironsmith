use super::*;

const TEXT: &str = "Whenever a creature you control deals combat damage to a player, turn that creature face up or put a +1/+1 counter on it.";
struct FaceUpOrCounter {
    mode: usize,
    choices: usize,
}
impl crate::decision::DecisionMaker for FaceUpOrCounter {
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        assert_eq!(
            ctx.options.len(),
            2,
            "both turn-face-up and counter options are retained"
        );
        assert!(
            ctx.options
                .iter()
                .any(|option| option.index == self.mode && option.legal)
        );
        self.choices += 1;
        vec![self.mode]
    }
}

#[test]
fn combat_face_up_or_counter_preserves_both_choices_for_the_damage_source() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Staff Room")
        .card_types(vec![CardType::Enchantment])
        .parse_text(TEXT)
        .unwrap();
    for (face_down, mode) in [(true, 0), (true, 1), (false, 1)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let room = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(4, 4))
            .build();
        let dealer = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let other = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let opponent = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        if face_down {
            assert!(game.set_face_down(dealer));
        }
        game.refresh_continuous_state();
        let damage = |source, combat| {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::DamageEvent::with_cause(
                    source,
                    crate::events::DamageTarget::Player(bob),
                    2,
                    combat,
                    crate::events::cause::EventCause::from_sba(),
                ),
                crate::provenance::ProvNodeId::default(),
            )
        };
        assert!(crate::triggers::check_triggers(&game, &damage(dealer, false)).is_empty());
        assert!(crate::triggers::check_triggers(&game, &damage(opponent, true)).is_empty());
        let event = damage(dealer, true);
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut choices = FaceUpOrCounter { mode, choices: 0 };
        let mut ctx = crate::effects::EffectContext::new_default(room, alice)
            .with_triggering_event(event)
            .with_decision_maker(&mut choices);
        for segment in &triggers[0].ability.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        drop(ctx);
        assert_eq!(game.is_face_down(dealer), face_down && mode == 1);
        assert_eq!(
            game.counter_count(dealer, CounterType::PlusOnePlusOne),
            u32::from(mode == 1)
        );
        assert_eq!(game.counter_count(other, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(opponent, CounterType::PlusOnePlusOne), 0);
        assert_eq!(choices.choices, 1);
    }
}

#[test]
fn combat_face_up_or_counter_renders_both_actions() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Staff Room")
        .card_types(vec![CardType::Enchantment])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn combat_face_up_or_counter_inline_choice_keeps_mode_constraints() {
    let choice = crate::effects::ChooseModeEffect::choose_one(vec![
        crate::effect::EffectMode::new("", vec![Effect::draw(Value::Fixed(1))]),
        crate::effect::EffectMode::new("", vec![Effect::gain_life(2)]),
    ])
    .with_chooser(PlayerFilter::You);
    assert!(super::super::single_effects_early::describe_inline_action_choice(&choice).is_some());
    for variant in 0..5 {
        let mut changed = choice.clone();
        match variant {
            0 => changed.chooser = None,
            1 => changed.random = true,
            2 => changed.modes[0].source_text = "Draw a card".into(),
            3 => changed.common_prefix_effects.push(Effect::gain_life(1)),
            _ => changed.choose_count = Value::Fixed(2),
        }
        assert!(
            super::super::single_effects_early::describe_inline_action_choice(&changed).is_none()
        );
    }
}
