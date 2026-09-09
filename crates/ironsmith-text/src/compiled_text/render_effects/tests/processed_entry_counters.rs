use super::*;
const TEXT: &str = "As this creature enters, you may put two cards your opponents own from exile into their owners' graveyards. If you do, this creature enters with four +1/+1 counters on it.";
struct ProcessChoices {
    accept: bool,
    choices: Vec<crate::ids::ObjectId>,
    controller: crate::ids::PlayerId,
}
impl crate::decision::DecisionMaker for ProcessChoices {
    fn decide_boolean(
        &mut self,
        game: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        // Do not force an illegal optional action when its exact-two choice is unavailable.
        self.accept
            && game
                .exile
                .iter()
                .filter(|id| game.object(**id).unwrap().owner != self.controller)
                .count()
                >= 2
    }
    fn decide_objects(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        let mut legal: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        assert!(legal.iter().all(|id| {
            let card = game.object(*id).unwrap();
            card.owner != self.controller && card.zone == Zone::Exile
        }));
        legal.sort();
        self.choices = legal.into_iter().take(2).collect();
        self.choices.clone()
    }
}
#[test]
fn processed_entry_counters_require_two_opponent_owned_cards() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ulamog's Despoiler")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(5, 5))
            .parse_text(TEXT)
            .unwrap();
    for modifier in 0..3 {
        for intrinsic in [false, true] {
            for available in [2, 3, 0, 1] {
                for accept in [false, true] {
                    let mut game = crate::game_state::GameState::new(
                        vec!["Alice".into(), "Bob".into(), "Carol".into()],
                        20,
                    );
                    let alice = game.players[0].id;
                    let bob = game.players[1].id;
                    let carol = game.players[2].id;
                    let filler =
                        crate::card::CardBuilder::new(crate::ids::CardId::new(), "Exiled Card")
                            .build();
                    let own = game.create_object_from_card(&filler, alice, Zone::Exile);
                    let mut opponent_cards = vec![];
                    for index in 0..available {
                        let owner = if index % 2 == 0 { bob } else { carol };
                        opponent_cards.push((
                            game.create_object_from_card(&filler, owner, Zone::Exile),
                            owner,
                        ));
                    }
                    if modifier != 0 {
                        let enchantment = crate::card::CardBuilder::new(
                            crate::ids::CardId::new(),
                            "Counter Doubler",
                        )
                        .card_types(vec![CardType::Enchantment])
                        .build();
                        let doubler =
                            game.create_object_from_card(&enchantment, alice, Zone::Battlefield);
                        game.object_mut(doubler).unwrap().abilities_mut().push(
                        crate::ability::Ability::static_ability(
                            if modifier == 1 {
                                crate::static_abilities::StaticAbility::double_counters_replacement(
                                    ObjectFilter::permanent(), Some(CounterType::PlusOnePlusOne), "Double counters".into(),
                                )
                            } else {
                                crate::static_abilities::StaticAbility::add_counters_placement_replacement(
                                    ObjectFilter::permanent(), Some(CounterType::PlusOnePlusOne), 1, "Add one counter".into(),
                                )
                            },
                        ),
                    );
                    }
                    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
                    if intrinsic {
                        game.object_mut(source).unwrap().abilities_mut().push(
                            crate::ability::Ability::static_ability(
                                crate::static_abilities::StaticAbility::enters_with_counters_value(
                                    CounterType::PlusOnePlusOne,
                                    Value::Fixed(2),
                                ),
                            ),
                        );
                    }

                    let mut dm = ProcessChoices {
                        accept,
                        choices: vec![],
                        controller: alice,
                    };
                    let entered = game
                        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut dm)
                        .unwrap_or_else(|| {
                            panic!("entry failed: available={available} accept={accept}")
                        });
                    let paid = accept && available >= 2;
                    let base = u32::from(paid) * 4 + u32::from(intrinsic) * 2;
                    let counters = match modifier {
                        1 => base * 2,
                        2 if base > 0 => base + 1,
                        _ => base,
                    };
                    assert_eq!(
                        game.counter_count(entered.new_id, CounterType::PlusOnePlusOne),
                        counters,
                        "available={available} accept={accept} modifier={modifier} intrinsic={intrinsic}"
                    );
                    assert_eq!(
                        game.calculated_power(entered.new_id),
                        Some(5 + counters as i32)
                    );
                    assert_eq!(game.object(own).unwrap().zone, Zone::Exile);
                    assert_eq!(
                        game.players[1].graveyard.len() + game.players[2].graveyard.len(),
                        if paid { 2 } else { 0 }
                    );
                    for (id, owner) in opponent_cards {
                        if paid && dm.choices.contains(&id) {
                            assert!(game.object(id).is_none());
                            assert_eq!(game.player(owner).unwrap().graveyard.len(), 1);
                        } else {
                            assert_eq!(game.object(id).unwrap().zone, Zone::Exile);
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn processed_entry_counters_render_the_optional_processing_and_source() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ulamog's Despoiler")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        TEXT
    );
}
