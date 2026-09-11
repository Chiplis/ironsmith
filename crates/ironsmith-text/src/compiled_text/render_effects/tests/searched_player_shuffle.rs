use super::*;

struct SearchDecision(Vec<crate::ids::PlayerId>);
impl crate::decision::DecisionMaker for SearchDecision {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.0.contains(&ctx.player)
    }
}

#[test]
fn searched_player_followup_shuffles_each_actual_searcher_after_all_searches() {
    for all_players in [false, true] {
        let subject = if all_players {
            "each player"
        } else {
            "each other player"
        };
        let oracle = format!(
            "When this creature enters, {subject} may search their library for a land card and put that card onto the battlefield. Then each player who searched their library this way shuffles."
        );
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Search Followup Probe")
                .card_types(vec![CardType::Creature])
                .parse_text(&oracle)
                .unwrap();
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(
            rendered.contains("Then each player who searched their library this way shuffles."),
            "{rendered}"
        );
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind
        else {
            panic!("expected entry trigger")
        };
        for mask in 0..4 {
            for blocked in [false, true] {
                for finds_land in [false, true] {
                    let mut game = crate::game_state::GameState::new(
                        vec!["Alice".into(), "Bob".into(), "Carol".into()],
                        20,
                    );
                    let alice = game.players[0].id;
                    let bob = game.players[1].id;
                    let carol = game.players[2].id;
                    let source =
                        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                    let card =
                        crate::card::CardBuilder::new(crate::ids::CardId::new(), "Library Card")
                            .card_types(vec![if finds_land {
                                CardType::Land
                            } else {
                                CardType::Creature
                            }])
                            .build();
                    for player in [alice, bob, carol] {
                        game.create_object_from_card(&card, player, Zone::Library);
                    }
                    let mut selected = Vec::new();
                    if mask & 1 != 0 {
                        selected.push(bob);
                    }
                    if mask & 2 != 0 {
                        selected.push(carol);
                    }
                    let expected = selected
                        .iter()
                        .copied()
                        .filter(|player| !blocked || *player != bob)
                        .collect::<Vec<_>>();
                    let mut decision = SearchDecision(selected);
                    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                        .with_decision_maker(&mut decision);
                    if blocked {
                        crate::effects::execute_effect(
                            &mut game,
                            &Effect::cant_until(
                                crate::effect::Restriction::SearchLibraries(
                                    PlayerFilter::Specific(bob),
                                ),
                                Until::EndOfTurn,
                            ),
                            &mut ctx,
                        )
                        .unwrap();
                    }
                    let mut events = Vec::new();
                    for segment in &triggered.effects.segments {
                        for effect in &segment.default_effects {
                            events.extend(
                                crate::effects::execute_effect(&mut game, effect, &mut ctx)
                                    .unwrap()
                                    .events,
                            );
                        }
                    }
                    let searches = events
                        .iter()
                        .filter_map(|event| {
                            event
                                .downcast::<crate::events::SearchLibraryEvent>()
                                .map(|event| event.player)
                        })
                        .collect::<Vec<_>>();
                    let shuffles = events
                        .iter()
                        .filter_map(|event| {
                            event
                                .downcast::<crate::events::ShuffleLibraryEvent>()
                                .map(|event| event.player)
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        searches, expected,
                        "mask={mask}, blocked={blocked}, land={finds_land}, all={all_players}"
                    );
                    assert_eq!(
                        shuffles, expected,
                        "mask={mask}, blocked={blocked}, land={finds_land}, all={all_players}"
                    );
                    if let Some(first_shuffle) = events.iter().position(|event| {
                        event
                            .downcast::<crate::events::ShuffleLibraryEvent>()
                            .is_some()
                    }) {
                        assert!(
                            events[first_shuffle..].iter().all(|event| event
                                .downcast::<crate::events::SearchLibraryEvent>()
                                .is_none()),
                            "finish the search round before shuffling"
                        );
                    }
                }
            }
        }
    }
}
