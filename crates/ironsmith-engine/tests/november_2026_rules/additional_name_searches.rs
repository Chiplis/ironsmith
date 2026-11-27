//! CR 701.23b, h. Libraries identified through an earlier choice or target.
use super::{A, card};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::{DecisionMaker, GameState, ObjectId, PlayerId, StackEntry, Target, Zone};

struct NameChoices {
    initial: Option<Vec<ObjectId>>,
    accepts: usize,
    offers: usize,
    take: bool,
    searches: Vec<(String, Vec<ObjectId>)>,
}
impl DecisionMaker for NameChoices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        assert!(self.searches.is_empty());
        let yes = self.accepts & (1 << self.offers) != 0;
        self.offers += 1;
        yes
    }
    fn decide_objects(&mut self, _: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal: Vec<_> = c
            .candidates
            .iter()
            .filter(|item| item.legal)
            .map(|item| item.id)
            .collect();
        if let Some(ids) = self.initial.take() {
            assert!(ids.iter().all(|id| legal.contains(id)));
            return ids;
        }
        assert_eq!(c.min, 0);
        self.searches.push((c.description.clone(), legal.clone()));
        if self.take {
            legal
                .into_iter()
                .take(c.max.unwrap_or(usize::MAX))
                .collect()
        } else {
            vec![]
        }
    }
}
fn events(g: &GameState) -> Vec<(char, PlayerId, PlayerId)> {
    g.turn_store
        .turn_history
        .event_records
        .iter()
        .filter_map(|r| {
            if let Some(s) = r.event.downcast::<ironsmith::events::SearchLibraryEvent>() {
                Some(('s', s.player, s.library_owner.unwrap()))
            } else {
                r.event
                    .downcast::<ironsmith::events::ShuffleLibraryEvent>()
                    .map(|s| ('h', s.player, s.player))
            }
        })
        .collect()
}

#[test]
fn name_slots_refer_to_the_chosen_permanents_or_the_target_opponents_tapped_nonlands() {
    let b = PlayerId(1);
    let c = PlayerId(2);
    let templates = [
        "grizzly-bears",
        "grizzly-bears",
        "sol-ring",
        "wastes",
        "command-tower",
    ]
    .map(card);
    for slug in ["clarion-ultimatum", "dichotomancy"] {
        let optional = slug == "clarion-ultimatum";
        for owner in if optional { vec![A] } else { vec![b, c] } {
            for tapped in if optional {
                vec![false]
            } else {
                vec![false, true]
            } {
                let mut base =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                let mut initial = Vec::new();
                let mut library = Vec::new();
                for def in &templates {
                    let permanent =
                        base.create_object_from_definition(def, owner, Zone::Battlefield);
                    if tapped {
                        base.tap(permanent);
                    }
                    initial.push(permanent);
                    library.push(base.create_object_from_definition(def, owner, Zone::Library));
                }
                // An untapped creature in the target's pool and another player's tapped
                // creature must not grant an extra search slot.
                let spare = card("llanowar-elves");
                base.create_object_from_definition(&spare, owner, Zone::Battlefield);
                let other = if owner == b { c } else { b };
                let other_creature =
                    base.create_object_from_definition(&spare, other, Zone::Battlefield);
                base.tap(other_creature);
                let untouched = base.create_object_from_definition(&spare, owner, Zone::Library);
                let src = base.create_object_from_definition(&card(slug), A, Zone::Stack);
                for accepts in 0usize..if optional { 32 } else { 1 } {
                    let slots = if optional {
                        accepts.count_ones() as usize
                    } else if tapped {
                        3
                    } else {
                        0
                    };
                    for take in [false, true] {
                        let mut g = base.clone();
                        let mut entry = StackEntry::new(src, A);
                        if !optional {
                            entry.targets.push(Target::Player(owner));
                        }
                        let mut dm = NameChoices {
                            initial: optional.then(|| initial.clone()),
                            accepts,
                            take,
                            offers: 0,
                            searches: vec![],
                        };
                        g.push_to_stack(entry);
                        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
                        assert_eq!(dm.offers, if optional { 5 } else { 0 }, "{slug}");
                        assert_eq!(dm.searches.len(), slots, "{slug}");
                        let player = g.player(owner).unwrap();
                        let found = if take { slots } else { 0 };
                        assert_eq!(player.library.len(), 6 - found);
                        assert!(player.library.contains(&untouched));
                        for id in library.iter().filter(|id| !player.library.contains(id)) {
                            let [new] = g.zone_change_result_ids(*id) else {
                                panic!("missing found card")
                            };
                            assert!(g.battlefield.contains(new));
                            assert_eq!(g.current_controller(*new), Some(A));
                            assert_eq!(g.is_tapped(*new), optional);
                            assert_eq!(g.object(*new).unwrap().owner, owner);
                        }
                        let mut expected = vec![];
                        if slots > 0 {
                            expected.push(('s', A, owner));
                        }
                        expected.push(('h', owner, owner));
                        assert_eq!(events(&g), expected, "{slug} {owner:?} {accepts}");
                    }
                }
            }
        }
    }
}

#[test]
fn reap_intellect_makes_one_choice_across_all_permitted_zones_and_shuffles_the_target() {
    let b = PlayerId(1);
    let bear = card("grizzly-bears");
    let ring = card("sol-ring");
    let land = card("wastes");
    for blocked in [false, true] {
        let mut base = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        if blocked {
            base.create_object_from_definition(&card("stranglehold"), b, Zone::Battlefield);
        }
        let mut hand = Vec::new();
        let mut originals = Vec::new();
        for zone in [Zone::Hand, Zone::Graveyard, Zone::Library] {
            for def in [&bear, &ring, &bear, &land] {
                let id = base.create_object_from_definition(def, b, zone);
                originals.push((id, zone, def.card.name.to_string()));
                if zone == Zone::Hand && def.card.name != "Wastes" {
                    hand.push(id);
                }
            }
        }
        let src = base.create_object_from_definition(&card("reap-intellect"), A, Zone::Stack);
        for x in 0..=3 {
            for take in [false, true] {
                let mut g = base.clone();
                let mut entry = StackEntry::new(src, A);
                entry.targets.push(Target::Player(b));
                entry.x_value = Some(x as u32);
                g.push_to_stack(entry);
                let selected = hand[..x].to_vec();
                let mut dm = NameChoices {
                    initial: Some(selected.clone()),
                    accepts: 0,
                    offers: 0,
                    take,
                    searches: vec![],
                };
                ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
                let mut expected = vec![];
                if x > 0 && !blocked {
                    expected.push(('s', A, b));
                }
                expected.push(('h', b, b));
                assert_eq!(events(&g), expected, "x={x} blocked={blocked}");
                let legal: Vec<_> = originals
                    .iter()
                    .filter(|(id, zone, name)| {
                        !selected.contains(id)
                            && (!blocked || *zone != Zone::Library)
                            && ((name == "Grizzly Bears" && x >= 1)
                                || (name == "Sol Ring" && x >= 2))
                    })
                    .map(|(id, _, _)| *id)
                    .collect();
                if x > 0 {
                    let [(prompt, candidates)] = dm.searches.as_slice() else {
                        panic!("expected one selection {:?}", dm.searches)
                    };
                    assert_eq!(candidates.len(), legal.len());
                    assert!(legal.iter().all(|id| candidates.contains(id)));
                    assert!(
                        prompt.contains("Bob's")
                            && prompt.contains("hand")
                            && prompt.contains("graveyard"),
                        "{prompt}"
                    );
                    assert_eq!(prompt.contains("library"), !blocked, "{prompt}");
                }
                for (id, zone, _) in &originals {
                    let exiled = selected.contains(id) || (take && legal.contains(id));
                    if exiled {
                        assert!(
                            g.zone_change_result_ids(*id)
                                .iter()
                                .any(|new| g.exile.contains(new))
                        );
                    } else {
                        assert_eq!(g.object(*id).unwrap().zone, *zone);
                    }
                }
            }
        }
    }
}

#[test]
fn dichotomancy_with_a_legal_target_resolves_even_if_nothing_is_tapped() {
    use ironsmith::decision::GameProgress;
    use ironsmith::{
        CastingMethod, LegalAction, ManaSymbol, Phase, PriorityLoopState, PriorityResponse,
        TriggerQueue,
    };
    let b = PlayerId(1);
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.turn_number = 2;
    let hand = g.create_object_from_definition(&card("dichotomancy"), A, Zone::Hand);
    g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 9);
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut dm = NameChoices {
        initial: None,
        accepts: 0,
        offers: 0,
        take: true,
        searches: vec![],
    };
    let action = LegalAction::CastSpell {
        spell_id: hand,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    };
    let mut next = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..16 {
        if let Some(methods) = &state.pending_method_selection {
            let normal = methods
                .available_methods
                .iter()
                .position(|m| m.method == CastingMethod::Normal)
                .unwrap();
            next = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut g,
                &mut q,
                &mut state,
                &PriorityResponse::CastingMethodChoice(normal),
                &mut dm,
            )
            .unwrap();
        } else if state.pending_cast.is_some() {
            let GameProgress::NeedsDecisionCtx(ctx) = next else {
                panic!("missing pending cast decision")
            };
            next = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut g, &mut q, &mut state, &ctx, &mut dm,
            )
            .unwrap();
        } else {
            break;
        }
    }
    assert!(state.pending_cast.is_none());
    assert_eq!(
        g.stack.last().unwrap().targets.as_slice(),
        &[Target::Player(b)]
    );
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    assert!(g.stack.is_empty());
    assert_eq!(events(&g), vec![('h', b, b)]);
    assert!(
        g.player(A)
            .unwrap()
            .graveyard
            .iter()
            .any(|id| g.object(*id).unwrap().name == "Dichotomancy")
    );
}
