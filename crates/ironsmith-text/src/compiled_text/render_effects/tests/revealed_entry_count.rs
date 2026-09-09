use super::*;
const TEXT: &str = "As this creature enters, you may reveal any number of other artifact cards from your hand. This creature enters with a +1/+1 counter on it for each card revealed this way.";
struct RevealChoices {
    count: usize,
    source: crate::ids::ObjectId,
    choices: Vec<crate::ids::ObjectId>,
}
impl crate::decision::DecisionMaker for RevealChoices {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
    fn decide_objects(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        let legal: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        assert!(!legal.contains(&self.source));
        assert!(
            legal
                .iter()
                .all(|id| game.current_has_card_type(*id, CardType::Artifact))
        );
        self.choices = legal.into_iter().take(self.count).collect();
        self.choices.clone()
    }
}
#[test]
fn revealed_entry_count_uses_only_chosen_other_artifacts() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Arsenal Thresher")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    for doubled in [false, true] {
        for stale in [false, true] {
            for count in 0..=3 {
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
                for _ in 0..3 {
                    game.create_object_from_definition(&definition, alice, Zone::Hand);
                }
                game.create_object_from_definition(&definition, bob, Zone::Hand);
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Nonartifact")
                    .card_types(vec![CardType::Creature])
                    .build();
                let nonartifact = game.create_object_from_card(&card, alice, Zone::Hand);
                if stale {
                    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                        game.object(nonartifact).unwrap(),
                        &game,
                    );
                    game.object_mut(source).unwrap().cast_tagged_objects.insert(
                        TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
                        vec![snapshot],
                    );
                }
                if doubled {
                    let doubler = game.create_object_from_card(&card, alice, Zone::Battlefield);
                    game.object_mut(doubler).unwrap().abilities_mut().push(
                        crate::ability::Ability::static_ability(
                            crate::static_abilities::StaticAbility::double_counters_replacement(
                                ObjectFilter::permanent(),
                                Some(CounterType::PlusOnePlusOne),
                                "Double +1/+1 counters".into(),
                            ),
                        ),
                    );
                }
                let mut dm = RevealChoices {
                    count,
                    source,
                    choices: vec![],
                };
                let entered = game
                    .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut dm)
                    .unwrap();
                assert_eq!(
                    game.counter_count(entered.new_id, CounterType::PlusOnePlusOne),
                    count as u32 * if doubled { 2 } else { 1 }
                );
                assert!(game.player(alice).unwrap().hand.contains(&nonartifact));
                assert_eq!(game.player(alice).unwrap().hand.len(), 4);
                let retained = &game.object(entered.new_id).unwrap().cast_tagged_objects
                    [&TagKey::from(crate::effects::PUBLIC_REVEALED_TAG)];
                assert_eq!(
                    retained.len(),
                    count,
                    "entry reveal replaces stale collection even for an empty choice"
                );
                for id in dm.choices {
                    assert!(game.player(alice).unwrap().hand.contains(&id));
                }
            }
        }
    }
}
#[test]
fn revealed_entry_count_preserves_reveal_reference() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Arsenal Thresher")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        TEXT
    );
}
