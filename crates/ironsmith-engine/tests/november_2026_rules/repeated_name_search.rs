//! CR 701.23b, h. A single search may find any subset of the matching hidden cards.
use super::{A, card};
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::{DecisionMaker, GameState, ObjectId, PlayerId, StackEntry, Target, Zone};

#[test]
fn haunting_echoes_searches_once_for_every_name_before_exiling_the_found_cards() {
    struct Find {
        owner: PlayerId,
        mask: usize,
        prompts: Vec<(String, Vec<ObjectId>)>,
    }
    impl DecisionMaker for Find {
        fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
            let legal: Vec<_> = c
                .candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .collect();
            if !legal
                .first()
                .is_some_and(|id| g.object(*id).unwrap().zone == Zone::Library)
            {
                return legal.into_iter().take(c.min).collect();
            }
            assert_eq!(c.player, A);
            assert_eq!(c.min, 0);
            assert_eq!(g.player(self.owner).unwrap().library.len(), 6);
            self.prompts.push((c.description.clone(), legal.clone()));
            legal
                .into_iter()
                .filter(|id| {
                    let index = g
                        .player(self.owner)
                        .unwrap()
                        .library
                        .iter()
                        .position(|v| v == id)
                        .unwrap();
                    self.mask & (1 << index) != 0
                })
                .collect()
        }
    }

    let b = PlayerId(1);
    let echoes = card("haunting-echoes");
    let bear = card("grizzly-bears");
    let ring = card("sol-ring");
    let land = card("wastes");
    let censor = card("aven-mindcensor");
    let lock = card("stranglehold");
    for owner in [A, b] {
        for (restriction, can_search, limited) in [
            (None, true, false),
            (Some(&censor), true, true),
            (Some(&lock), false, false),
        ] {
            for with_names in [false, true] {
                let mut base = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                if let Some(def) = restriction {
                    base.create_object_from_definition(def, b, Zone::Battlefield);
                }
                base.create_object_from_definition(&land, owner, Zone::Graveyard);
                if with_names {
                    for def in [&ring, &bear, &ring] {
                        base.create_object_from_definition(def, owner, Zone::Graveyard);
                    }
                }
                // Library's top is last. In a top-four search the deeper matches
                // must stay inaccessible after any number of cards are found.
                let cards = [&ring, &bear, &land, &bear, &land, &ring]
                    .map(|def| base.create_object_from_definition(def, owner, Zone::Library));
                let src = base.create_object_from_definition(&echoes, A, Zone::Stack);
                let allowed = with_names && can_search;
                let expected: Vec<_> = [0, 1, 3, 5]
                    .into_iter()
                    .filter(|i| allowed && (!limited || *i >= 2))
                    .map(|i| cards[i])
                    .collect();
                for mask in [0, 1, 8, 32, 63] {
                    let mut g = base.clone();
                    let mut spell = StackEntry::new(src, A);
                    spell.targets.push(Target::Player(owner));
                    g.push_to_stack(spell);
                    let mut dm = Find {
                        owner,
                        mask,
                        prompts: vec![],
                    };
                    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
                    assert_eq!(
                        g.player(owner)
                            .unwrap()
                            .graveyard
                            .iter()
                            .filter(|id| g.object(**id).unwrap().name == "Wastes")
                            .count(),
                        1
                    );
                    let records = &g.turn_store.turn_history.event_records;
                    let searches: Vec<_> = records
                        .iter()
                        .filter_map(|r| r.event.downcast::<ironsmith::events::SearchLibraryEvent>())
                        .map(|e| (e.player, e.library_owner))
                        .collect();
                    let shuffles: Vec<_> = records
                        .iter()
                        .filter_map(|r| {
                            r.event.downcast::<ironsmith::events::ShuffleLibraryEvent>()
                        })
                        .map(|e| e.player)
                        .collect();
                    assert_eq!(
                        searches,
                        if allowed {
                            vec![(A, Some(owner))]
                        } else {
                            vec![]
                        }
                    );
                    assert_eq!(shuffles, vec![owner]);
                    if allowed {
                        let [(prompt, candidates)] = dm.prompts.as_slice() else {
                            panic!("expected one combined selection: {:?}", dm.prompts)
                        };
                        assert!(
                            prompt.contains(if owner == A {
                                "your library"
                            } else {
                                "Bob's library"
                            }),
                            "{prompt}"
                        );
                        assert!(prompt.contains("any of those cards"), "{prompt}");
                        assert_eq!(candidates.len(), expected.len());
                        assert!(expected.iter().all(|id| candidates.contains(id)));
                    } else {
                        assert!(dm.prompts.is_empty());
                    }
                    for (index, id) in cards.iter().enumerate() {
                        let found = expected.contains(id) && (mask & (1 << index)) != 0;
                        assert_eq!(g.player(owner).unwrap().library.contains(id), !found);
                    }
                }
            }
        }
    }
}

#[test]
fn doubling_chant_collects_the_offers_then_searches_once_for_distinct_cards() {
    use ironsmith::decisions::context::{BooleanContext, ViewCardsContext};
    struct Choices {
        accepts: usize,
        finds: usize,
        offered: usize,
        picks: usize,
        inspected: bool,
        chosen: Vec<ObjectId>,
    }
    impl DecisionMaker for Choices {
        fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
            assert_eq!(c.player, A);
            assert!(
                !self.inspected,
                "a library was inspected before the last offer"
            );
            let yes = self.accepts & (1 << self.offered) != 0;
            self.offered += 1;
            yes
        }
        fn view_cards(
            &mut self,
            _: &GameState,
            viewer: PlayerId,
            _: &[ObjectId],
            c: &ViewCardsContext,
        ) {
            if !c.public && c.zone == Zone::Library {
                assert_eq!((viewer, c.subject), (A, A));
                assert_eq!(self.offered, 3);
                self.inspected = true;
            }
        }
        fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
            let legal: Vec<_> = c
                .candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .collect();
            assert!(self.inspected);
            assert_eq!(g.player(A).unwrap().library.len(), 4);
            assert_eq!((c.min, c.max), (0, Some(1)));
            assert!(legal.iter().all(|id| !self.chosen.contains(id)));
            let take = self.finds & (1 << self.picks) != 0;
            self.picks += 1;
            if take {
                let card = legal[0];
                self.chosen.push(card);
                vec![card]
            } else {
                vec![]
            }
        }
    }
    let mut base = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let bear = card("grizzly-bears");
    let elf = card("llanowar-elves");
    for def in [&bear, &bear, &elf] {
        base.create_object_from_definition(def, A, Zone::Battlefield);
        base.create_object_from_definition(def, A, Zone::Library);
    }
    base.create_object_from_definition(&card("wastes"), A, Zone::Library);
    let src = base.create_object_from_definition(&card("doubling-chant"), A, Zone::Stack);
    for accepts in 0usize..8 {
        for finds in 0..8 {
            let mut g = base.clone();
            g.push_to_stack(StackEntry::new(src, A));
            let mut dm = Choices {
                accepts,
                finds,
                offered: 0,
                picks: 0,
                inspected: false,
                chosen: Vec::new(),
            };
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(dm.offered, 3);
            assert_eq!(dm.picks, accepts.count_ones() as usize);
            assert_eq!(dm.inspected, accepts != 0);
            assert_eq!(g.player(A).unwrap().library.len(), 4 - dm.chosen.len());
            for id in dm.chosen {
                let [found] = g.zone_change_result_ids(id) else {
                    panic!("missing found card")
                };
                assert!(g.battlefield.contains(found));
            }
            let events = &g.turn_store.turn_history.event_records;
            let searches = events
                .iter()
                .filter_map(|r| r.event.downcast::<ironsmith::events::SearchLibraryEvent>())
                .map(|e| e.player)
                .collect::<Vec<_>>();
            let shuffles = events
                .iter()
                .filter_map(|r| r.event.downcast::<ironsmith::events::ShuffleLibraryEvent>())
                .map(|e| e.player)
                .collect::<Vec<_>>();
            assert_eq!(searches, if accepts != 0 { vec![A] } else { vec![] });
            assert_eq!(shuffles, vec![A]);
        }
    }
}

#[test]
fn opposition_agent_exiles_the_completed_doubling_chant_selection() {
    use ironsmith::decisions::context::BooleanContext;
    struct TakeFirst(usize);
    impl DecisionMaker for TakeFirst {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            true
        }
        fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
            assert!(
                g.exile.is_empty(),
                "an earlier slot was already exiled during the search"
            );
            let id = c.candidates.iter().find(|v| v.legal).unwrap().id;
            assert!(
                c.description.contains(g.object(id).unwrap().name.as_str()),
                "{}",
                c.description
            );
            self.0 += 1;
            vec![id]
        }
    }
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.create_object_from_definition(&card("opposition-agent"), PlayerId(1), Zone::Battlefield);
    for slug in ["grizzly-bears", "grizzly-bears", "llanowar-elves"] {
        let def = card(slug);
        g.create_object_from_definition(&def, A, Zone::Battlefield);
        g.create_object_from_definition(&def, A, Zone::Library);
    }
    let src = g.create_object_from_definition(&card("doubling-chant"), A, Zone::Stack);
    g.push_to_stack(StackEntry::new(src, A));
    let mut take = TakeFirst(0);
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut take).unwrap();
    assert_eq!(take.0, 3);
    assert_eq!(g.exile.len(), 3);
    assert_eq!(g.battlefield.len(), 4);
}
