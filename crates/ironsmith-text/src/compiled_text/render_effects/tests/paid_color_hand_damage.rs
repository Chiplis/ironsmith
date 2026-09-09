use super::*;

const TEXT: &str = "Flying\nWhenever Darigaaz deals combat damage to a player, you may pay {2}{R}. If you do, choose a color, then that player reveals their hand and Darigaaz deals damage to the player equal to the number of cards of that color revealed this way.";

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
fn paid_color_hand_damage_only_affects_the_damaged_players_chosen_color() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Darigaaz, the Igniter")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(6, 6))
            .parse_text(TEXT)
            .unwrap();
    for hand_size in [0, 5] {
        for matching_color in [false, true] {
            for (pay, enough_mana) in [(false, true), (true, true), (true, false)] {
                let mut game = crate::game_state::GameState::new(
                    vec!["Alice".into(), "Bob".into(), "Carol".into()],
                    20,
                );
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let carol = game.players[2].id;
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let colors = [
                    crate::color::ColorSet::RED,
                    crate::color::ColorSet::RED,
                    crate::color::ColorSet::BLUE,
                    crate::color::ColorSet::RED.union(crate::color::ColorSet::GREEN),
                    crate::color::ColorSet::default(),
                ];
                let mut hands = Vec::new();
                for player in [alice, bob, carol] {
                    for (index, color) in colors.into_iter().take(hand_size).enumerate() {
                        let color = if matching_color {
                            color
                        } else {
                            crate::color::ColorSet::BLUE
                        };
                        let card =
                            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Hand card")
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
                    .add(crate::mana::ManaSymbol::Red, 1);
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
                    "only the damaged player reveals their hand"
                );
                for (player, _, id) in hands {
                    assert!(game.player(player).unwrap().hand.contains(&id));
                }
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(
                    game.player(bob).unwrap().life,
                    if paid && matching_color && hand_size > 0 {
                        17
                    } else {
                        20
                    }
                );
                assert_eq!(game.player(carol).unwrap().life, 20);
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
    }
}

#[test]
fn paid_color_hand_damage_renders_the_color_choice_and_shared_player() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Darigaaz, the Igniter")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT.replace(
            "and Darigaaz deals damage to the player",
            "and this creature deals damage to that player"
        )
    );
}
