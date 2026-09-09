use super::*;

const TEXT: &str = "Flying\nWhenever Crosis deals combat damage to a player, you may pay {2}{B}. If you do, choose a color, then that player reveals their hand and discards all cards of that color.";

struct ChooseRed {
    pay: bool,
    controller: crate::ids::PlayerId,
    color_choices: usize,
}
impl crate::decision::DecisionMaker for ChooseRed {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.pay
    }
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        assert_eq!(ctx.player, self.controller);
        self.color_choices += 1;
        vec![
            ctx.options
                .iter()
                .find(|option| option.legal && option.description.eq_ignore_ascii_case("red"))
                .expect("red color option")
                .index,
        ]
    }
}

#[test]
fn paid_color_hand_discard_only_affects_the_damaged_players_chosen_color() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Crosis, the Purger")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(6, 6))
            .parse_text(TEXT)
            .unwrap();
    for (pay, enough_mana) in [(false, true), (true, true), (true, false)] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let colors = [
            crate::color::ColorSet::RED,
            crate::color::ColorSet::RED,
            crate::color::ColorSet::BLUE,
            crate::color::ColorSet::RED.union(crate::color::ColorSet::GREEN),
            crate::color::ColorSet::default(),
        ];
        let mut hands = Vec::new();
        for player in [alice, bob, carol] {
            for (index, color) in colors.into_iter().enumerate() {
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Hand card")
                    .card_types(vec![CardType::Sorcery])
                    .color_indicator(color)
                    .build();
                let id = game.create_object_from_card(&card, player, Zone::Hand);
                hands.push((player, index, id));
            }
        }
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Black, 1);
        if enough_mana {
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(crate::mana::ManaSymbol::Colorless, 2);
        }
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Player(bob),
                6,
                true,
                crate::events::cause::EventCause::from_sba(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut choices = ChooseRed {
            pay,
            controller: alice,
            color_choices: 0,
        };
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_triggering_event(event)
            .with_decision_maker(&mut choices);
        for segment in &triggers[0].ability.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        let revealed = ctx
            .get_tagged_all(crate::effects::REVEALED_THIS_WAY_TAG)
            .map(|cards| cards.iter().map(|card| card.object_id).collect::<Vec<_>>())
            .unwrap_or_default();
        drop(ctx);
        let paid = pay && enough_mana;
        assert_eq!(choices.color_choices, usize::from(paid));
        let expected_revealed = hands
            .iter()
            .filter(|(player, _, _)| paid && *player == bob)
            .map(|(_, _, id)| *id)
            .collect::<Vec<_>>();
        assert_eq!(
            revealed, expected_revealed,
            "the full hand is revealed before any cards are discarded"
        );
        for (player, index, id) in hands {
            let discarded = paid && player == bob && [0, 1, 3].contains(&index);
            assert_eq!(game.player(player).unwrap().hand.contains(&id), !discarded);
        }
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            if paid {
                0
            } else if enough_mana {
                3
            } else {
                1
            }
        );
    }
}

#[test]
fn paid_color_hand_discard_renders_the_color_choice_and_shared_player() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Crosis, the Purger")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn paid_color_hand_discard_renderer_requires_the_complete_shared_filter() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Crosis, the Purger")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| {
            if let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind {
                Some(triggered)
            } else {
                None
            }
        })
        .unwrap();
    let branch = triggered.effects.flattened_default_effects()[1]
        .downcast_ref::<crate::effects::IfEffect>()
        .unwrap();
    let sequence = branch.then[0]
        .downcast_ref::<crate::effects::SequenceEffect>()
        .unwrap();
    for variant in 0..6 {
        let mut effects = sequence.effects.clone();
        let mut reveal = effects[1]
            .downcast_ref::<crate::effects::LookAtHandEffect>()
            .unwrap()
            .clone();
        let mut discard = effects[2]
            .downcast_ref::<crate::effects::DiscardEffect>()
            .unwrap()
            .clone();
        match variant {
            1 => reveal.reveal = false,
            2 => discard.player = PlayerFilter::You,
            3 => discard.count = Value::Fixed(1),
            4 => discard
                .card_filter
                .as_mut()
                .unwrap()
                .card_types
                .push(CardType::Creature),
            5 => {
                effects[0] = Effect::new(crate::effects::ChooseColorEffect::new(
                    PlayerFilter::Opponent,
                ))
            }
            _ => {}
        }
        effects[1] = Effect::new(reveal);
        effects[2] = Effect::new(discard);
        assert_eq!(
            describe_choose_color_reveal_hand_and_discard(&effects).is_some(),
            variant == 0,
            "variant {variant}"
        );
    }
}
