use super::*;

struct KeepHandCards {
    keep: usize,
    selected: Vec<crate::ids::ObjectId>,
}
impl crate::decision::DecisionMaker for KeepHandCards {
    fn decide_objects(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        for (index, player) in game.players.iter().enumerate() {
            assert_eq!(
                player.hand.len(),
                [4, 10, 0][index],
                "all players choose before any hand is shuffled"
            );
        }
        let legal = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        for id in &legal {
            assert!(
                game.player(ctx.player).unwrap().hand.contains(id),
                "each player chooses only from their own hand"
            );
        }
        let chosen = legal.into_iter().take(self.keep).collect::<Vec<_>>();
        self.selected.extend_from_slice(&chosen);
        chosen
    }
}

#[test]
fn hand_remainder_shuffle_preserves_chosen_cards_and_empties_mana() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Worldpurge")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return all permanents to their owners' hands. Each player chooses up to seven cards in their hand, then shuffles the rest into their library. Each player loses all unspent mana.").unwrap();
    for keep in [0, 7] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let players = game
            .players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<_>>();
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Hand card")
            .card_types(vec![CardType::Land])
            .build();
        for (index, player) in players.iter().copied().enumerate() {
            for _ in 0..[3, 9, 0][index] {
                game.create_object_from_card(&card, player, Zone::Hand);
            }
            for _ in 0..2 {
                game.create_object_from_card(&card, player, Zone::Library);
            }
            if index < 2 {
                let permanent = game.create_object_from_card(&card, player, Zone::Battlefield);
                game.set_current_controller(permanent, players[1]);
            }
            game.player_mut(player)
                .unwrap()
                .mana_pool
                .add(crate::mana::ManaSymbol::Red, 3);
        }
        let source = game.create_object_from_definition(&definition, players[0], Zone::Stack);
        let mut decisions = KeepHandCards {
            keep,
            selected: vec![],
        };
        let mut ctx = crate::effects::EffectContext::new_default(source, players[0])
            .with_decision_maker(&mut decisions);
        let mut shuffled = Vec::new();
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects {
                let outcome = crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                shuffled.extend(outcome.events.iter().filter_map(|event| {
                    event
                        .downcast::<crate::events::ShuffleLibraryEvent>()
                        .map(|event| event.player)
                }));
            }
        }
        drop(ctx);
        assert!(game.battlefield.is_empty());
        assert_eq!(shuffled.len(), players.len());
        assert!(players.iter().all(|player| shuffled.contains(player)));
        for (index, player) in players.iter().copied().enumerate() {
            let hand_before = [4, 10, 0][index];
            let expected_kept = keep.min(hand_before);
            assert_eq!(
                game.player(player).unwrap().hand.len(),
                expected_kept,
                "keep={keep}, player={index}"
            );
            assert_eq!(
                game.player(player).unwrap().library.len(),
                2 + hand_before - expected_kept
            );
            assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
        }
        for id in decisions.selected {
            assert_eq!(game.object(id).unwrap().zone, Zone::Hand);
        }
    }
}

#[test]
fn hand_remainder_shuffle_renders_the_same_selected_complement() {
    let oracle = "Return all permanents to their owners' hands. Each player chooses up to seven cards in their hand, then shuffles the rest into their library. Each player loses all unspent mana.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Worldpurge")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

#[test]
fn hand_remainder_shuffle_renderer_rejects_a_different_remainder() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Hand partition")
        .card_types(vec![CardType::Sorcery]).parse_text("Each player chooses up to three cards in their hand, then shuffles the rest into their library.").unwrap();
    let players = definition
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
        .iter()
        .find_map(|effect| crate::compiled_text::render_effects::effect_lists::structural_unwrap_render_wrappers(effect).downcast_ref::<crate::effects::ForPlayersEffect>())
        .unwrap();
    let render = super::super::player_and_zone_effects::describe_for_players_keep_hand_then_shuffle_remainder;
    assert!(render(players).unwrap().contains("up to three"));
    for change_owner in [false, true] {
        let mut broken = players.clone();
        let mut shuffle = crate::compiled_text::render_effects::effect_lists::structural_unwrap_render_wrappers(&broken.effects[1])
            .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()
            .unwrap()
            .clone();
        if change_owner {
            shuffle.player = PlayerFilter::You;
        } else {
            let ChooseSpec::All(filter) = &mut shuffle.target else {
                panic!("all remainder");
            };
            filter.tagged_constraints.clear();
        }
        broken.effects[1] = Effect::new(shuffle);
        assert!(render(&broken).is_none());
    }
}
