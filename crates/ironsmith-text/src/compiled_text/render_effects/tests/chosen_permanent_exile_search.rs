use super::*;
const ORACLE: &str = "For each opponent, choose up to one target artifact or enchantment that player controls. For each permanent chosen this way, its controller may exile it. Then if one or more of the chosen permanents are still on the battlefield, you search your library for up to that many land cards, put them onto the battlefield tapped, then shuffle.";
struct Decisions {
    opponents: Vec<crate::ids::PlayerId>,
    accept: u8,
    calls: usize,
    survivors: usize,
    searches: usize,
}
impl crate::decision::DecisionMaker for Decisions {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert_eq!(
            ctx.player, self.opponents[self.calls],
            "each permanent's controller decides for that permanent"
        );
        let accept = self.accept & (1 << self.calls) != 0;
        self.calls += 1;
        accept
    }
    fn decide_objects(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        self.searches += 1;
        assert!(
            self.survivors > 0,
            "no land search when every chosen permanent is gone"
        );
        assert_eq!(ctx.max, Some(self.survivors));
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .take(self.survivors)
            .map(|c| c.id)
            .collect()
    }
}
#[test]
fn chosen_permanent_exile_search_counts_only_surviving_chosen_permanents() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Disorienting Choice")
            .card_types(vec![CardType::Sorcery])
            .parse_text(ORACLE)
            .unwrap();
    for chosen_count in 0..=2 {
        for accept in 0u8..(1 << chosen_count) {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Carol".into()],
                20,
            );
            let alice = game.players[0].id;
            let opponents = vec![game.players[1].id, game.players[2].id];
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let artifact =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Chosen artifact")
                    .card_types(vec![CardType::Artifact])
                    .build();
            let targets = opponents
                .iter()
                .take(chosen_count)
                .map(|p| game.create_object_from_card(&artifact, *p, Zone::Battlefield))
                .collect::<Vec<_>>();
            let unrelated =
                game.create_object_from_card(&artifact, opponents[0], Zone::Battlefield);
            let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Library land")
                .card_types(vec![CardType::Land])
                .build();
            for _ in 0..4 {
                game.create_object_from_card(&land, alice, Zone::Library);
            }
            let survivors = chosen_count - accept.count_ones() as usize;
            let mut dm = Decisions {
                opponents,
                accept,
                calls: 0,
                survivors,
                searches: 0,
            };
            {
                let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm)
                    .with_targets(
                        targets
                            .iter()
                            .map(|id| crate::effects::ResolvedTarget::Object(*id))
                            .collect(),
                    );
                for segment in &definition.spell_effect.as_ref().unwrap().segments {
                    for effect in &segment.default_effects {
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                }
            }
            assert_eq!(dm.calls, chosen_count);
            assert_eq!(
                game.objects_in_zone(Zone::Exile).len(),
                accept.count_ones() as usize
            );
            for (index, target) in targets.iter().enumerate() {
                assert_eq!(
                    game.object(*target)
                        .is_some_and(|o| o.zone == Zone::Battlefield),
                    accept & (1 << index) == 0
                );
            }
            let fetched = game
                .objects_in_zone(Zone::Battlefield)
                .into_iter()
                .filter(|id| game.object(*id).unwrap().name == "Library land")
                .collect::<Vec<_>>();
            assert_eq!(fetched.len(), survivors);
            assert!(fetched.iter().all(|id| game.is_tapped(*id)));
            assert_eq!(game.player(alice).unwrap().library.len(), 4 - survivors);
            assert_eq!(dm.searches, usize::from(survivors > 0));
            assert!(
                game.object(unrelated)
                    .is_some_and(|o| o.zone == Zone::Battlefield)
            );
        }
    }
}

#[test]
fn chosen_permanent_exile_search_text_preserves_survivor_condition() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Disorienting Choice")
            .card_types(vec![CardType::Sorcery])
            .parse_text(ORACLE)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        ORACLE
    );
}
