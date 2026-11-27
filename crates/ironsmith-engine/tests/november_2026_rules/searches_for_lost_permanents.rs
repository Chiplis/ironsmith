//! CR 608.2e–f and 701.23h–i, when multiple permanents generate library searches.
use super::{A, card};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, ViewCardsContext};
use ironsmith::{
    AbilityKind, CardDefinition, DecisionMaker, GameState, ObjectId, PlayerId, StackEntry, Zone,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Step {
    Offer(PlayerId),
    Inspect(PlayerId),
    Choose(PlayerId),
}
struct Searches {
    accepting: u8,
    take: usize,
    steps: Vec<Step>,
    choices: Vec<(PlayerId, usize)>,
}
impl DecisionMaker for Searches {
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.steps.push(Step::Offer(c.player));
        self.accepting & (1 << c.player.0) != 0
    }
    fn view_cards(
        &mut self,
        _: &GameState,
        viewer: PlayerId,
        _: &[ObjectId],
        c: &ViewCardsContext,
    ) {
        if c.zone == Zone::Library && !c.public {
            assert_eq!(viewer, c.subject);
            if !self.steps.contains(&Step::Inspect(viewer)) {
                self.steps.push(Step::Inspect(viewer));
            }
        }
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let available: Vec<_> = c
            .candidates
            .iter()
            .filter(|v| v.legal)
            .map(|v| v.id)
            .collect();
        let library = available
            .first()
            .is_some_and(|id| g.object(*id).unwrap().zone == Zone::Library);
        let max = c.max.unwrap_or(available.len());
        if library {
            assert!(
                g.battlefield
                    .iter()
                    .all(|id| g.object(*id).unwrap().name != "Wastes"),
                "a found card entered before the searches finished"
            );
            assert!(
                available
                    .iter()
                    .all(|id| g.object(*id).unwrap().owner == c.player)
            );
            self.steps.push(Step::Choose(c.player));
            self.choices.push((c.player, max));
        }
        available
            .into_iter()
            .take(if library { max.min(self.take) } else { max })
            .collect()
    }
}
fn entry(g: &mut GameState, slug: &str, def: &CardDefinition) -> StackEntry {
    let source = g.create_object_from_definition(
        def,
        A,
        if slug == "krenko-s-buzzcrusher" {
            Zone::Battlefield
        } else {
            Zone::Stack
        },
    );
    match slug {
        "winds-of-abandon" => {
            let ironsmith::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
                &def.alternative_casts[0]
            else {
                panic!("missing overload program")
            };
            StackEntry::ability(source, A, effects.clone())
        }
        "krenko-s-buzzcrusher" => {
            let effects = def
                .abilities
                .iter()
                .find_map(|a| match &a.kind {
                    AbilityKind::Triggered(t) => Some(t.effects.clone()),
                    _ => None,
                })
                .unwrap();
            StackEntry::ability(source, A, effects)
        }
        _ => StackEntry::new(source, A),
    }
}

#[test]
fn affected_controllers_search_in_turn_order_once_each_and_only_then_put_cards_and_shuffle() {
    let seats = [A, PlayerId(1), PlayerId(2)];
    let tower = card("command-tower");
    let bear = card("grizzly-bears");
    let basic = card("wastes");
    let ring = card("sol-ring");
    let citadel = card("darksteel-citadel");
    for (slug, optional, tapped) in [
        ("from-the-ashes", true, false),
        ("wave-of-vitriol", true, true),
        ("winds-of-abandon", false, true),
        ("krenko-s-buzzcrusher", true, true),
    ] {
        let mut base = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let mut counts = [1usize, 2, 1];
        for p in [PlayerId(2), PlayerId(1), PlayerId(1), A] {
            base.create_object_from_definition(
                if slug == "winds-of-abandon" {
                    &bear
                } else {
                    &tower
                },
                p,
                Zone::Battlefield,
            );
        }
        base.create_object_from_definition(&ring, A, Zone::Battlefield);
        if matches!(slug, "from-the-ashes" | "wave-of-vitriol") {
            base.create_object_from_definition(&citadel, PlayerId(2), Zone::Battlefield);
        }
        match slug {
            "winds-of-abandon" => counts[0] = 0,
            "krenko-s-buzzcrusher" => counts[1] = 1,
            "wave-of-vitriol" => counts[2] += 1,
            _ => {}
        }
        for p in seats {
            for _ in 0..4 {
                base.create_object_from_definition(&basic, p, Zone::Library);
            }
        }
        let spell = entry(&mut base, slug, &card(slug));
        for active in 0..3 {
            let order: Vec<_> = (active..3)
                .chain(0..active)
                .map(PlayerId)
                .filter(|p| counts[p.0 as usize] > 0)
                .collect();
            for accepting in 0u8..=if optional { 7 } else { 0 } {
                let yes: Vec<_> = order
                    .iter()
                    .copied()
                    .filter(|p| !optional || accepting & (1 << p.0) != 0)
                    .collect();
                for take in 0..=2 {
                    let mut g = base.clone();
                    g.turn.active_player = PlayerId(active);
                    g.push_to_stack(spell.clone());
                    let mut dm = Searches {
                        accepting,
                        take,
                        steps: vec![],
                        choices: vec![],
                    };
                    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
                    let mut expected = Vec::new();
                    if optional {
                        expected.extend(order.iter().copied().map(Step::Offer));
                    }
                    expected.extend(yes.iter().copied().map(Step::Inspect));
                    expected.extend(yes.iter().copied().map(Step::Choose));
                    assert_eq!(
                        dm.steps, expected,
                        "{slug}, active={active}, accepting={accepting}, take={take}"
                    );
                    let bounds: Vec<_> = yes.iter().map(|p| (*p, counts[p.0 as usize])).collect();
                    assert_eq!(dm.choices, bounds, "{slug}");
                    let mut events = Vec::new();
                    for record in g.turn_store.turn_history.event_records.iter() {
                        if let Some(ev) = record
                            .event
                            .downcast::<ironsmith::events::SearchLibraryEvent>()
                        {
                            events.push(('s', ev.player));
                        }
                        if let Some(ev) = record
                            .event
                            .downcast::<ironsmith::events::ShuffleLibraryEvent>()
                        {
                            events.push(('h', ev.player));
                        }
                    }
                    let expected: Vec<_> = yes
                        .iter()
                        .map(|p| ('s', *p))
                        .chain(yes.iter().map(|p| ('h', *p)))
                        .collect();
                    assert_eq!(events, expected, "{slug}");
                    for p in seats {
                        let found = if yes.contains(&p) {
                            take.min(counts[p.0 as usize])
                        } else {
                            0
                        };
                        assert_eq!(g.player(p).unwrap().library.len(), 4 - found, "{slug}");
                        let lands: Vec<_> = g
                            .battlefield
                            .iter()
                            .copied()
                            .filter(|id| {
                                g.object(*id).is_some_and(|o| {
                                    o.name == "Wastes" && g.current_controller(o.id) == Some(p)
                                })
                            })
                            .collect();
                        assert_eq!(lands.len(), found, "{slug}");
                        assert!(lands.iter().all(|id| g.is_tapped(*id) == tapped));
                    }
                }
            }
        }
    }
}

#[test]
fn winds_uses_the_creatures_last_controller_after_a_real_normal_or_overload_cast() {
    use ironsmith::decision::GameProgress;
    use ironsmith::{
        CastingMethod, LegalAction, ManaSymbol, Phase, PriorityLoopState, PriorityResponse, Target,
    };
    let bears = card("grizzly-bears");
    let wastes = card("wastes");
    let winds = card("winds-of-abandon");
    let plane = card("living-plane");
    let land = card("command-tower");
    for method in [CastingMethod::Normal, CastingMethod::Alternative(0)] {
        for animated_land in [false, true] {
            let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            g.turn.active_player = A;
            g.turn.priority_player = Some(A);
            g.turn.phase = Phase::FirstMain;
            g.turn.step = None;
            g.turn.turn_number = 2;
            let b = PlayerId(1);
            if animated_land {
                g.create_object_from_definition(&plane, A, Zone::Battlefield);
            }
            let stolen = g.create_object_from_definition(
                if animated_land { &land } else { &bears },
                A,
                Zone::Battlefield,
            );
            assert!(g.current_has_card_type(stolen, ironsmith::CardType::Creature));
            g.set_current_controller(stolen, b).unwrap();
            g.create_object_from_definition(&bears, b, Zone::Battlefield);
            for _ in 0..3 {
                g.create_object_from_definition(&wastes, b, Zone::Library);
            }
            let hand = g.create_object_from_definition(&winds, A, Zone::Hand);
            g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White, 6);
            let mut state = PriorityLoopState::new(g.players_in_game());
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut dm = Searches {
                accepting: 7,
                take: 2,
                steps: vec![],
                choices: vec![],
            };
            let action = LegalAction::CastSpell {
                spell_id: hand,
                from_zone: Zone::Hand,
                casting_method: method.clone(),
            };
            let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut g,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            for _ in 0..16 {
                if let Some(pending) = &state.pending_method_selection {
                    let index = pending
                        .available_methods
                        .iter()
                        .position(|m| m.method == method)
                        .unwrap();
                    progress = ironsmith::game_loop::apply_priority_response_with_dm(
                        &mut g,
                        &mut queue,
                        &mut state,
                        &PriorityResponse::CastingMethodChoice(index),
                        &mut dm,
                    )
                    .unwrap();
                } else if state.pending_cast.is_some() {
                    let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                        panic!("pending cast without decision")
                    };
                    progress = ironsmith::game_loop::apply_decision_context_with_dm(
                        &mut g, &mut queue, &mut state, &ctx, &mut dm,
                    )
                    .unwrap();
                } else {
                    break;
                }
            }
            assert!(state.pending_cast.is_none());
            let announced = g.stack.last().unwrap();
            let count = if method == CastingMethod::Normal {
                assert!(
                    matches!(announced.targets.as_slice(), [Target::Object(id)] if *id == stolen)
                );
                1
            } else {
                assert!(announced.targets.is_empty());
                2
            };
            dm.steps.clear();
            dm.choices.clear();
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(dm.choices, vec![(b, count)]);
            assert_eq!(g.player(b).unwrap().library.len(), 3 - count);
            assert_eq!(
                g.battlefield
                    .iter()
                    .filter(|id| g.object(**id).is_some_and(
                        |o| o.name == "Wastes" && g.current_controller(o.id) == Some(b)
                    ))
                    .count(),
                count
            );
        }
    }
}

#[test]
fn a_combined_search_uses_one_mindcensor_window_and_shuffles_only_the_required_libraries() {
    let b = PlayerId(1);
    let creature = card("grizzly-bears");
    let basic = card("wastes");
    let nonbasic = card("command-tower");
    for slug in ["from-the-ashes", "winds-of-abandon"] {
        for restriction in ["aven-mindcensor", "stranglehold"] {
            let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            g.create_object_from_definition(&card(restriction), A, Zone::Battlefield);
            for _ in 0..3 {
                g.create_object_from_definition(
                    if slug == "from-the-ashes" {
                        &nonbasic
                    } else {
                        &creature
                    },
                    b,
                    Zone::Battlefield,
                );
            }
            // One legal land in the top four; two other legal lands below it.
            for def in [&basic, &basic, &creature, &creature, &creature, &basic] {
                g.create_object_from_definition(def, b, Zone::Library);
            }
            let spell = entry(&mut g, slug, &card(slug));
            g.push_to_stack(spell);
            let mut dm = Searches {
                accepting: 7,
                take: 3,
                steps: vec![],
                choices: vec![],
            };
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            let allowed = restriction == "aven-mindcensor";
            assert_eq!(
                g.player(b).unwrap().library.len(),
                if allowed { 5 } else { 6 },
                "{slug}, {restriction}"
            );
            let shuffles = g
                .turn_store
                .turn_history
                .event_records
                .iter()
                .filter_map(|r| r.event.downcast::<ironsmith::events::ShuffleLibraryEvent>())
                .map(|e| e.player)
                .collect::<Vec<_>>();
            let searches = g
                .turn_store
                .turn_history
                .event_records
                .iter()
                .filter_map(|r| r.event.downcast::<ironsmith::events::SearchLibraryEvent>())
                .map(|e| e.player)
                .collect::<Vec<_>>();
            assert_eq!(
                searches,
                if allowed { vec![b] } else { vec![] },
                "{slug}, {restriction}"
            );
            // From the Ashes names players who searched. Winds names the
            // controllers of the exiled creatures and still instructs them to shuffle.
            assert_eq!(
                shuffles,
                if allowed || slug == "winds-of-abandon" {
                    vec![b]
                } else {
                    vec![]
                },
                "{slug}, {restriction}"
            );
        }
    }
}

#[test]
fn a_shuffle_printed_inside_each_separate_search_still_happens_per_occurrence() {
    use ironsmith::effects::ForEachTaggedEffect;
    use ironsmith::{EffectContext, EffectExecutor};
    let b = PlayerId(1);
    let def = card("krenko-s-buzzcrusher");
    let each = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(t) => Some(&t.effects),
            _ => None,
        })
        .flat_map(|program| program.flattened_default_effects())
        .find_map(|effect| effect.downcast_ref::<ForEachTaggedEffect>())
        .unwrap();
    let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
    let land = card("command-tower");
    let mut snapshots = Vec::new();
    for _ in 0..2 {
        let id = g.create_object_from_definition(&land, b, Zone::Battlefield);
        snapshots.push(ironsmith::snapshot::ObjectSnapshot::from_object_id(&g, id).unwrap());
    }
    g.create_object_from_definition(&card("wastes"), b, Zone::Library);
    let mut dm = Searches {
        accepting: 7,
        take: 0,
        steps: vec![],
        choices: vec![],
    };
    let mut ctx = EffectContext::new(source, A, &mut dm);
    ctx.set_tagged_objects(each.tag.clone(), snapshots);
    let outcome = each.execute(&mut g, &mut ctx).unwrap();
    let events: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|raw| {
            if let Some(e) = raw.downcast::<ironsmith::events::SearchLibraryEvent>() {
                Some(('s', e.player))
            } else {
                raw.downcast::<ironsmith::events::ShuffleLibraryEvent>()
                    .map(|e| ('h', e.player))
            }
        })
        .collect();
    assert_eq!(events, vec![('s', b), ('h', b), ('s', b), ('h', b)]);
}

#[test]
fn winds_counts_a_melded_permanent_and_a_token_as_one_creature_each() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::MeldEffect;
    use ironsmith::{CardId, CardType, EffectContext, EffectExecutor, PowerToughness};
    let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
    let b = PlayerId(1);
    let halves = ["Graf Rats", "Midnight Scavengers", "Chittering Host"].map(|name| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    });
    for def in &halves {
        g.register_linked_face_definition(def);
    }
    let first = g.create_object_from_definition(&halves[0], A, Zone::Battlefield);
    g.create_object_from_definition(&halves[1], A, Zone::Battlefield);
    let melded = MeldEffect::new("Chittering Host")
        .execute(&mut g, &mut EffectContext::new_default(first, A))
        .unwrap()
        .first_output_object()
        .unwrap();
    g.set_current_controller(melded, b).unwrap();
    g.create_token_object_from_definition(&card("grizzly-bears"), b, Zone::Battlefield);
    for _ in 0..4 {
        g.create_object_from_definition(&card("wastes"), b, Zone::Library);
    }
    let spell = entry(&mut g, "winds-of-abandon", &card("winds-of-abandon"));
    g.push_to_stack(spell);
    let mut dm = Searches {
        accepting: 7,
        take: 4,
        steps: vec![],
        choices: vec![],
    };
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    for name in ["Graf Rats", "Midnight Scavengers"] {
        assert!(
            g.exile
                .iter()
                .any(|id| g.object(*id).is_some_and(|o| o.name == name))
        );
    }
    assert_eq!(dm.steps, vec![Step::Inspect(b), Step::Choose(b)]);
    assert_eq!(dm.choices, vec![(b, 2)]);
    assert_eq!(g.player(b).unwrap().library.len(), 2);
}
