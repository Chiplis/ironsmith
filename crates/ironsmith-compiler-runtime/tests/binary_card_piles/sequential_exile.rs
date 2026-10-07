use super::*;
struct Piles {
    exposed: usize, chosen: usize,
    expected: [Vec<ironsmith::ids::StableId>; 2],
    views: Vec<(PlayerId, bool, Vec<ironsmith::ids::StableId>)>,
    choices: usize,
    pause_exposure: bool, pending: bool,
    pause_opening: bool, openings: Vec<Vec<ironsmith::ids::StableId>>,
}
impl DecisionMaker for Piles {
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(context.reveal_policy, ironsmith::decisions::context::SelectionRevealPolicy::Public);
        let ids: Vec<_> = context.candidates.iter().map(|candidate| candidate.id).collect();
        self.openings.push(ids.iter().map(|id| game.object(*id).unwrap().stable_id).collect());
        self.pending = self.pause_opening;
        ids
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(cara) = context.options.iter().find(|option| option.description == "Cara") {
            assert_eq!(context.player, A);
            return vec![cara.index];
        }
        if context.options.iter().any(|option| option.description.starts_with("Turn pile")) {
            assert_eq!(context.player, A);
            assert_eq!(context.options.len(), 2);
            assert!(context.options.iter().all(|option| option.legal));
            self.pending = self.pause_exposure;
            return vec![self.exposed];
        }
        if context.options.iter().any(|option| option.description.starts_with("Choose pile")) {
            assert_eq!(context.player, C);
            assert_eq!(context.options.len(), 2);
            assert!(context.options.iter().all(|option| option.legal));
            assert_eq!(game.player(A).unwrap().life, 20, "life loss is the final instruction");
            for (index, option) in context.options.iter().enumerate() {
                let shown = option.related_object_ids.as_ref().expect("each option previews only its chosen pile");
                assert_eq!(shown.len(), self.expected[index].len());
                assert!(shown.iter().all(|id| self.expected[index].contains(&game.object(*id).unwrap().stable_id)));
            }
            for (index, identities) in self.expected.iter().enumerate() {
                for identity in identities {
                    let id = game.find_object_by_stable_id(*identity).unwrap();
                    if game.object(id).unwrap().zone == Zone::Exile {
                        assert_eq!(game.is_face_down(id), index != self.exposed);
                        if index != self.exposed { assert!(game.can_player_look_at_face_down_exiled_card(id, A)); }
                    }
                }
            }
            self.choices += 1;
            return vec![self.chosen];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn view_cards(&mut self, game: &GameState, viewer: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
        assert_eq!(context.zone, Zone::Exile);
        assert_eq!(context.subject, A);
        let identities: Vec<_> = cards.iter().map(|id| game.object(*id).unwrap().stable_id).collect();
        assert!(identities.iter().all(|identity| self.expected.iter().any(|pile| pile.contains(identity))));
        if context.public {
            assert!(identities.iter().all(|identity| self.expected[self.exposed].contains(identity)));
        } else { assert_eq!(viewer, A); }
        self.views.push((viewer, context.public, identities));
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn setup(available: usize, exposed: usize, chosen: usize) -> (GameState, Vec<ObjectId>, Piles) {
    let mut game = game();
    let all = library(&mut game, A, available);
    let top: Vec<_> = all.iter().rev().map(|id| game.object(*id).unwrap().stable_id).collect();
    let piles = Piles { exposed, chosen,
        expected: [top.iter().take(3).copied().collect(), top.iter().skip(3).take(3).copied().collect()],
        views: vec![], choices: 0, pause_exposure: false, pending: false,
        pause_opening: false, openings: vec![],
    };
    (game, all, piles)
}
#[test]
fn hostile_has_two_sequential_finite_exile_groups_and_keeps_the_full_life_loss_tail() {
    for definition in definitions("Hostile Negotiations") {
        for available in 0..=8 {
            for exposed in 0..2 {
                for chosen in 0..2 {
                    let (mut game, all, mut dm) = setup(available, exposed, chosen);
                    cast(&mut game, &definition, &mut dm);
                    assert!(game.stack[0].targets.is_empty());
                    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert!(game.stack.is_empty());
                    assert_eq!(dm.choices, 1);
                    assert_eq!(game.player(A).unwrap().life, 17);
                    assert_eq!(game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::CardRevealed), 0,
                        "turning an exile pile face up is not an authored reveal action");
                    for (index, pile) in dm.expected.iter().enumerate() {
                        for identity in pile {
                            let id = game.find_object_by_stable_id(*identity).unwrap();
                            assert_eq!(game.object(id).unwrap().zone, if chosen == index { Zone::Hand } else { Zone::Graveyard });
                        }
                    }
                    assert_eq!(game.player(A).unwrap().library.as_slice(), &all[..available.saturating_sub(6)]);
                    assert_eq!(game.player(A).unwrap().hand.len(), dm.expected[chosen].len());
                }
            }
        }
    }
}

#[test]
fn hostile_pending_exposure_rolls_back_both_exiles_and_life_then_replays() {
    for definition in definitions("Hostile Negotiations") {
        let (mut game, all, mut dm) = setup(8, 1, 0);
        dm.pause_exposure = true;
        cast(&mut game, &definition, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.pending);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(A).unwrap().library, all);
        assert_eq!(game.player(A).unwrap().life, 20);
        dm.pending = false;
        dm.pause_exposure = false;
        dm.views.clear();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().life, 17);
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
    }
}

#[test]
fn prevented_first_exile_makes_the_second_group_from_the_new_actual_top() {
    for definition in definitions("Hostile Negotiations") {
        let (mut game, all, mut dm) = setup(8, 1, 1);
        let prevented = *all.last().unwrap();
        let identity = game.object(prevented).unwrap().stable_id;
        // First producer still processes its finite original three-card set.
        // The next producer sees the prevented top card again.
        dm.expected[0].retain(|id| *id != identity);
        dm.expected[1] = vec![identity,
            game.object(all[4]).unwrap().stable_id,
            game.object(all[3]).unwrap().stable_id];
        let source = cast(&mut game, &definition, &mut dm);
        game.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(source, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ironsmith::target::ObjectFilter::specific(prevented), Some(Zone::Library), Some(Zone::Exile)),
                ironsmith::replacement::ReplacementAction::Prevent));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
        assert_eq!(game.player(A).unwrap().library.as_slice(), &all[..3]);
        assert_eq!(game.player(A).unwrap().life, 17);
        let current = game.find_object_by_stable_id(identity).unwrap();
        assert_eq!(game.object(current).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn hostile_full_native_owner_and_peer_wait_for_the_exact_selected_exile_opening() {
    let face = compile_to_runtime_definition("Tracked library identity", "Type: Artifact", false).unwrap();
    for definition in definitions("Hostile Negotiations") {
        for owner_view in [false, true] {
            for exposed in 0..2 {
                for chosen in 0..2 {
                    let mut game = game();
                    let all: Vec<_> = (0..8).map(|slot|
                        game.create_hidden_card_placeholder(A, Zone::Library, slot, format!("hostile-{slot}"))).collect();
                    if owner_view {
                        for id in &all[2..] { game.reveal_hidden_card_with_definition(*id, &face).unwrap(); }
                    }
                    let top: Vec<_> = all.iter().rev().map(|id| game.object(*id).unwrap().stable_id).collect();
                    let mut dm = Piles {
                        exposed, chosen, expected: [top[..3].to_vec(), top[3..6].to_vec()],
                        views: vec![], choices: 0, pause_exposure: false, pending: false,
                        pause_opening: true, openings: vec![],
                    };
                    cast(&mut game, &definition, &mut dm);
                    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert!(dm.pending);
                    assert_eq!(dm.choices, 0, "opponent cannot choose before the exposure completes");
                    assert_eq!(game.player(A).unwrap().library, all);
                    assert_eq!(game.player(A).unwrap().life, 20);
                    assert_eq!(dm.openings.len(), 1);
                    assert_eq!(dm.openings[0].len(), 3);
                    assert!(dm.openings[0].iter().all(|id| dm.expected[exposed].contains(id)));
                    // Model authenticated dispatcher openings routed back to
                    // the retained original IDs before replay. The public
                    // destinations need identities; the hidden hand pile does
                    // not become visible to an unrelated peer.
                    if !owner_view {
                        for (index, pile) in dm.expected.iter().enumerate() {
                            if index == exposed || index != chosen {
                                for identity in pile {
                                    let id = game.find_object_by_stable_id(*identity).unwrap();
                                    game.reveal_hidden_card_with_definition(id, &face).unwrap();
                                }
                            }
                        }
                    }
                    dm.pending = false; dm.pause_opening = false; dm.views.clear();
                    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert!(game.stack.is_empty());
                    assert_eq!(game.player(A).unwrap().life, 17);
                    assert_eq!(game.player(A).unwrap().hand.len(), 3);
                    assert_eq!(game.player(A).unwrap().library.as_slice(), &all[..2]);
                    assert_eq!(dm.openings[0], dm.openings[1], "replay requests the same physical pile");
                    if !owner_view && chosen != exposed {
                        for identity in &dm.expected[chosen] {
                            let id = game.find_object_by_stable_id(*identity).unwrap();
                            assert!(game.is_hidden_card_placeholder(id), "the unexposed hand pile remains private on the peer");
                        }
                    }
                    assert_eq!(game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::CardRevealed), 0);
                }
            }
        }
    }
}
