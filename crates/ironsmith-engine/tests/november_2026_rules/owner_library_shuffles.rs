use super::*;
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::events::ShuffleLibraryEvent;
use ironsmith::game_state::TargetAssignment;
use ironsmith::{CounterType, StackEntry, Target};

const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

#[derive(Default)]
struct Decisions {
    decline_shuffle: Option<PlayerId>,
    choices: Vec<(PlayerId, bool, Vec<usize>)>,
}

impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, g: &GameState, c: &BooleanContext) -> bool {
        let commander = c.description.contains("command zone");
        self.choices.push((
            c.player,
            commander,
            g.players.iter().map(|p| p.hand.len()).collect(),
        ));
        commander || Some(c.player) != self.decline_shuffle
    }
}

fn shuffles(g: &GameState) -> Vec<PlayerId> {
    g.turn_store
        .turn_history
        .event_records
        .iter()
        .filter_map(|record| {
            record
                .event
                .downcast::<ShuffleLibraryEvent>()
                .map(|e| e.player)
        })
        .collect()
}

fn resolve(g: &mut GameState, entry: StackEntry, dm: &mut Decisions) {
    g.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(g, dm).unwrap();
}

#[test]
fn each_player_chooses_before_any_hand_or_graveyard_is_shuffled() {
    let land = card("wastes");
    let commander = card("isamaru-hound-of-konda");
    for route in [
        "timetwister",
        "sway-of-the-stars",
        "step-between-worlds",
        "turtles-in-time",
    ] {
        let optional = matches!(route, "step-between-worlds" | "turtles-in-time");
        let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        g.turn.active_player = B;
        for player in [A, B, C] {
            for _ in 0..10 {
                g.create_object_from_definition(&land, player, Zone::Library);
            }
            let general = g.create_object_from_definition(&commander, player, Zone::Hand);
            g.set_as_commander(general, player);
            g.create_object_from_definition(&land, player, Zone::Hand);
            g.create_object_from_definition(&land, player, Zone::Graveyard);
        }
        let mut dm = Decisions {
            decline_shuffle: optional.then_some(C),
            ..Default::default()
        };
        let source = put(&mut g, route, A, Zone::Stack);
        resolve(&mut g, StackEntry::new(source, A), &mut dm);
        let participants = if optional { vec![B, A] } else { vec![B, C, A] };
        let mut expected: Vec<_> = if optional {
            [B, C, A]
                .into_iter()
                .map(|p| (p, false, vec![2; 3]))
                .collect()
        } else {
            vec![]
        };
        expected.extend(participants.iter().map(|p| (*p, true, vec![2; 3])));
        assert_eq!(dm.choices, expected, "{route}");
        assert_eq!(shuffles(&g), participants, "{route}");
        for player in [A, B, C] {
            let took_part = !optional || player != C;
            let state = g.player(player).unwrap();
            assert_eq!(state.hand.len(), if took_part { 7 } else { 2 }, "{route}");
            assert_eq!(
                state
                    .graveyard
                    .iter()
                    .filter(|id| g.object(**id).unwrap().name.as_str() == "Wastes")
                    .count(),
                usize::from(!took_part),
                "{route}"
            );
        }
    }
}

fn activated(g: &GameState, source: ObjectId, target: Option<ObjectId>) -> StackEntry {
    let ability = g
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    let mut entry = StackEntry::ability(
        source,
        g.controller_of_id(source).unwrap(),
        ability.effects.clone(),
    );
    if let Some(target) = target {
        entry.targets.push(Target::Object(target));
        entry.target_assignments.push(TargetAssignment {
            spec: ability.choices[0].clone(),
            range: 0..1,
        });
    }
    entry
}

#[test]
fn void_stalker_moves_the_union_and_each_owner_shuffles_once() {
    for (target_owner, self_target) in [(A, false), (B, false), (A, true)] {
        let mut g = game();
        g.turn.active_player = B;
        let source = put(&mut g, "void-stalker", A, Zone::Battlefield);
        let target = if self_target {
            source
        } else {
            put(&mut g, "grizzly-bears", target_owner, Zone::Battlefield)
        };
        let entry = activated(&g, source, Some(target));
        resolve(&mut g, entry, &mut Decisions::default());
        assert_eq!(
            shuffles(&g),
            if target_owner == A {
                vec![A]
            } else {
                vec![B, A]
            }
        );
        assert_eq!(
            g.player(A).unwrap().library.len(),
            1 + usize::from(!self_target && target_owner == A)
        );
        assert_eq!(
            g.player(B).unwrap().library.len(),
            usize::from(target_owner == B)
        );
    }
}

#[test]
fn putting_combatants_on_top_then_shuffling_remembers_a_redirected_commander() {
    for route in ["gomazoa", "vortex-elemental"] {
        let mut g = game();
        g.turn.active_player = B;
        g.turn.phase = Phase::Combat;
        let source = put(&mut g, route, A, Zone::Battlefield);
        let attacker = put(&mut g, "isamaru-hound-of-konda", B, Zone::Battlefield);
        g.set_as_commander(attacker, B);
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(A),
        });
        combat.blockers.insert(attacker, vec![source]);
        combat.blocked_attackers.insert(attacker);
        combat.block_declaration_complete = true;
        g.combat = Some(combat);
        let entry = activated(&g, source, None);
        resolve(&mut g, entry, &mut Decisions::default());
        assert!(g.object(source).is_none());
        assert_eq!(g.player(A).unwrap().library.len(), 1);
        assert_eq!(g.player(B).unwrap().library.len(), 0);
        assert_eq!(shuffles(&g), vec![B, A], "{route}");
    }
}

#[test]
fn owners_shuffle_a_set_of_targeted_graveyard_cards_once() {
    for route in ["turn-the-earth", "those-old-scientists"] {
        for owners in [vec![], vec![A, A], vec![B, A], vec![A, B]] {
            let mut g = game();
            g.turn.active_player = B;
            let targets: Vec<_> = owners
                .iter()
                .map(|p| Target::Object(put(&mut g, "grizzly-bears", *p, Zone::Graveyard)))
                .collect();
            let saga = route == "those-old-scientists";
            let source = put(
                &mut g,
                route,
                A,
                if saga { Zone::Battlefield } else { Zone::Stack },
            );
            let (mut entry, spec) = if saga {
                let chapter = g
                    .object(source)
                    .unwrap()
                    .abilities
                    .iter()
                    .rev()
                    .find_map(|a| match &a.kind {
                        AbilityKind::Triggered(t) => Some(t),
                        _ => None,
                    })
                    .unwrap();
                (
                    StackEntry::ability(source, A, chapter.effects.clone()),
                    chapter.choices[0].clone(),
                )
            } else {
                let requirements =
                    ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
                        &g,
                        g.object(source).unwrap().spell_effect.as_deref().unwrap(),
                        A,
                        Some(source),
                        None,
                    );
                (StackEntry::new(source, A), requirements[0].spec.clone())
            };
            entry.target_assignments.push(TargetAssignment {
                spec,
                range: 0..targets.len(),
            });
            entry.targets = targets;
            resolve(&mut g, entry, &mut Decisions::default());
            for p in [A, B] {
                assert_eq!(
                    g.player(p).unwrap().library.len(),
                    owners.iter().filter(|owner| **owner == p).count()
                );
            }
            assert_eq!(
                shuffles(&g),
                [B, A]
                    .into_iter()
                    .filter(|p| owners.contains(p))
                    .collect::<Vec<_>>(),
                "{route} {owners:?}"
            );
            if !saga {
                assert_eq!(g.player(A).unwrap().life, 22);
            }
        }
    }
}

#[test]
fn each_targeted_permanents_controller_consults_their_own_library() {
    for stolen in [false, true] {
        let mut g = game();
        let first = put(
            &mut g,
            "grizzly-bears",
            if stolen { B } else { A },
            Zone::Battlefield,
        );
        if stolen {
            g.set_current_controller(first, A).unwrap();
        }
        let second = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        for p in [A, B] {
            put(&mut g, "sol-ring", p, Zone::Library);
        }
        let source = put(&mut g, "guff-rewrites-history", A, Zone::Stack);
        let req = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
            &g,
            g.object(source).unwrap().spell_effect.as_deref().unwrap(),
            A,
            Some(source),
            None,
        );
        assert_eq!(req.len(), 1, "one target per player must be announceable");
        assert_eq!((req[0].min_targets, req[0].max_targets), (2, Some(2)));
        let entry = StackEntry::new(source, A)
            .with_targets(vec![Target::Object(first), Target::Object(second)])
            .with_target_assignments(vec![TargetAssignment {
                spec: req[0].spec.clone(),
                range: 0..2,
            }]);
        resolve(&mut g, entry, &mut Decisions::default());
        assert_eq!(shuffles(&g), if stolen { vec![B] } else { vec![A, B] });
        for p in [A, B] {
            assert_eq!(
                g.exile
                    .iter()
                    .filter(|id| g.object(**id).unwrap().owner == p)
                    .count(),
                1,
                "former controller {p:?} must exile one nonland"
            );
        }
    }
}

#[test]
fn a_named_player_shuffles_even_when_they_own_no_targeted_cards() {
    let mut g = game();
    let source = put(&mut g, "loaming-shaman", A, Zone::Battlefield);
    let existing = put(&mut g, "grizzly-bears", B, Zone::Graveyard);
    let trigger = g
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .unwrap();
    let mut entry = StackEntry::ability(source, A, trigger.effects.clone());
    for spec in &trigger.choices {
        let start = entry.targets.len();
        if matches!(spec.base(), ironsmith::target::ChooseSpec::Player(_)) {
            entry.targets.push(Target::Player(B));
        }
        entry.target_assignments.push(TargetAssignment {
            spec: spec.clone(),
            range: start..entry.targets.len(),
        });
    }
    resolve(&mut g, entry, &mut Decisions::default());
    assert_eq!(shuffles(&g), vec![B]);
    assert_eq!(g.object(existing).unwrap().zone, Zone::Graveyard);
}

#[test]
fn a_named_card_that_left_its_graveyard_still_causes_a_shuffle() {
    use ironsmith::events::cause::EventCause;
    for route in [
        "guile",
        "alabaster-dragon",
        "serra-avatar",
        "worldspine-wurm",
        "dread",
        "hostility",
        "purity",
        "vigor",
        "cavalier-of-gales",
        "angel-of-fury",
    ] {
        for returns_to_graveyard in [false, true] {
            let mut g = game();
            let original = put(&mut g, route, B, Zone::Battlefield);
            let grave = g
                .move_object(original, Zone::Graveyard, EventCause::from_game_rule())
                .unwrap();
            let mut queue = ironsmith::TriggerQueue::new();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut queue).unwrap();
            let triggers = queue.entries.len();
            assert_eq!(
                triggers,
                if route == "worldspine-wurm" { 2 } else { 1 },
                "{route}"
            );
            ironsmith::game_loop::put_triggers_on_stack(&mut g, &mut queue).unwrap();
            let exiled = g
                .move_object(grave, Zone::Exile, EventCause::from_game_rule())
                .unwrap();
            let later = if returns_to_graveyard {
                g.move_object(exiled, Zone::Graveyard, EventCause::from_game_rule())
                    .unwrap()
            } else {
                exiled
            };
            for _ in 0..triggers {
                ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut Decisions::default())
                    .unwrap();
            }
            assert!(
                g.object(later).is_some(),
                "{route} must leave the new object alone"
            );
            assert_eq!(
                shuffles(&g),
                vec![B],
                "{route} returned={returns_to_graveyard}"
            );
            assert!(g.player(B).unwrap().library.is_empty());
        }
    }
}

fn shuffle_trigger_count(g: &mut GameState, witness: ObjectId) -> usize {
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(g, &mut queue).unwrap();
    queue
        .entries
        .iter()
        .filter(|entry| entry.source == witness)
        .count()
}

#[test]
fn the_player_shuffle_triggers_look_at_who_performed_the_shuffle() {
    for stolen in [false, true] {
        let mut g = game();
        let me = put(&mut g, "cosi-s-trickster", A, Zone::Battlefield);
        let elixir = put(&mut g, "elixir-of-immortality", B, Zone::Battlefield);
        let actor = if stolen { A } else { B };
        if stolen {
            g.set_current_controller(elixir, A).unwrap();
        }
        put(&mut g, "wastes", actor, Zone::Graveyard);
        let entry = activated(&g, elixir, None);
        resolve(&mut g, entry, &mut Decisions::default());
        assert_eq!(
            g.player(B).unwrap().library.len(),
            if stolen { 1 } else { 2 }
        );
        assert_eq!(shuffle_trigger_count(&mut g, me), usize::from(!stolen));
    }
    // The source and the resolving controller's graveyard name two libraries
    // even when the source leaves before resolution and that graveyard is empty.
    let mut g = game();
    let elixir = put(&mut g, "elixir-of-immortality", B, Zone::Battlefield);
    g.set_current_controller(elixir, A).unwrap();
    let entry = activated(&g, elixir, None);
    g.push_to_stack(entry);
    let exile = g
        .move_object(
            elixir,
            Zone::Exile,
            ironsmith::events::cause::EventCause::from_game_rule(),
        )
        .unwrap();
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut Decisions::default()).unwrap();
    assert_eq!(shuffles(&g), vec![A, B]);
    assert_eq!(g.object(exile).unwrap().zone, Zone::Exile);
    // A flashback spell can leave the stack for exile and still shuffle its owner's library.
    let mut g = game();
    let witness = put(&mut g, "cosi-s-trickster", A, Zone::Battlefield);
    let zenith = put(&mut g, "black-sun-s-zenith", B, Zone::Stack);
    g.object_mut(zenith).unwrap().cast_alternative_method = Some(Box::new(
        ironsmith::alternative_cast::AlternativeCastingMethod::Flashback {
            total_cost: ironsmith::cost::TotalCost::free(),
            x_minimum: 0,
        },
    ));
    let mut entry = StackEntry::new(zenith, B)
        .with_casting_method(ironsmith::alternative_cast::CastingMethod::GrantedFlashback);
    entry.x_value = Some(0);
    resolve(&mut g, entry, &mut Decisions::default());
    assert_eq!(g.exile.len(), 1);
    assert_eq!(shuffles(&g), vec![B]);
    assert_eq!(shuffle_trigger_count(&mut g, witness), 1);
}

#[test]
fn shuffling_the_source_and_targets_is_one_instruction_per_library() {
    for route in ["floodpits-drowner", "serene-remembrance"] {
        for target_owner in [A, B] {
            let mut g = game();
            let permanent = route == "floodpits-drowner";
            let source = put(
                &mut g,
                route,
                A,
                if permanent {
                    Zone::Battlefield
                } else {
                    Zone::Stack
                },
            );
            let target = put(
                &mut g,
                "grizzly-bears",
                target_owner,
                if permanent {
                    Zone::Battlefield
                } else {
                    Zone::Graveyard
                },
            );
            let witness = put(&mut g, "cosi-s-trickster", A, Zone::Battlefield);
            let entry = if permanent {
                g.add_counters(target, CounterType::Stun, 1).unwrap();
                activated(&g, source, Some(target))
            } else {
                let req = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
                    &g,
                    g.object(source).unwrap().spell_effect.as_ref().unwrap(),
                    A,
                    Some(source),
                    None,
                );
                assert_eq!(req.len(), 1);
                StackEntry::new(source, A)
                    .with_targets(vec![Target::Object(target)])
                    .with_target_assignments(vec![TargetAssignment {
                        spec: req[0].spec.clone(),
                        range: 0..1,
                    }])
            };
            resolve(&mut g, entry, &mut Decisions::default());
            assert_eq!(
                shuffles(&g),
                if target_owner == A {
                    vec![A]
                } else {
                    vec![A, B]
                },
                "{route}"
            );
            assert_eq!(
                g.player(A).unwrap().library.len(),
                1 + usize::from(target_owner == A),
                "{route}"
            );
            assert_eq!(
                g.player(B).unwrap().library.len(),
                usize::from(target_owner == B),
                "{route}"
            );
            assert_eq!(shuffle_trigger_count(&mut g, witness), 0, "{route}");
        }
    }
}

#[test]
fn explicit_owner_shuffles_and_imperative_shuffles_trigger_differently() {
    for owner_shuffles in [false, true] {
        let mut g = game();
        let witness = put(&mut g, "cosi-s-trickster", A, Zone::Battlefield);
        let target = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        g.set_current_controller(target, A).unwrap();
        let entry = if owner_shuffles {
            let spell = put(&mut g, "chaos-warp", A, Zone::Stack);
            StackEntry::new(spell, A).with_targets(vec![Target::Object(target)])
        } else {
            let pawnshop = put(&mut g, "rishadan-pawnshop", A, Zone::Battlefield);
            activated(&g, pawnshop, Some(target))
        };
        resolve(&mut g, entry, &mut Decisions::default());
        assert_eq!(shuffles(&g), vec![B]);
        assert_eq!(
            shuffle_trigger_count(&mut g, witness),
            usize::from(owner_shuffles)
        );
    }
}

#[test]
fn guff_can_be_cast_only_with_a_permanent_target_for_every_player() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let mut g = game();
    let spell = put(&mut g, "guff-rewrites-history", A, Zone::Hand);
    g.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ironsmith::ManaSymbol::Red, 3);
    let first = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    let second = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
    let can_cast = |g: &GameState| {
        compute_legal_actions(g, A)
            .unwrap()
            .iter()
            .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==spell))
    };
    assert!(can_cast(&g), "two controllers have legal targets");
    g.set_current_controller(second, A).unwrap();
    assert!(!can_cast(&g), "both targets have the same controller");
    g.set_current_controller(first, B).unwrap();
    assert!(
        can_cast(&g),
        "a stolen permanent is still a legal target for its controller"
    );
}

#[test]
fn only_legal_graveyard_targets_contribute_owners_at_resolution() {
    for all_illegal in [false, true] {
        let mut g = game();
        let first = put(&mut g, "grizzly-bears", A, Zone::Graveyard);
        let second = put(&mut g, "grizzly-bears", B, Zone::Graveyard);
        let spell = put(&mut g, "turn-the-earth", A, Zone::Stack);
        let req = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
            &g,
            g.object(spell).unwrap().spell_effect.as_ref().unwrap(),
            A,
            Some(spell),
            None,
        );
        let entry = StackEntry::new(spell, A)
            .with_targets(vec![Target::Object(first), Target::Object(second)])
            .with_target_assignments(vec![TargetAssignment {
                spec: req[0].spec.clone(),
                range: 0..2,
            }]);
        g.push_to_stack(entry);
        g.move_object(
            first,
            Zone::Exile,
            ironsmith::events::cause::EventCause::from_game_rule(),
        )
        .unwrap();
        if all_illegal {
            g.move_object(
                second,
                Zone::Exile,
                ironsmith::events::cause::EventCause::from_game_rule(),
            )
            .unwrap();
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut Decisions::default()).unwrap();
        assert_eq!(shuffles(&g), if all_illegal { vec![] } else { vec![B] });
        assert_eq!(g.player(A).unwrap().life, if all_illegal { 20 } else { 22 });
    }
}

#[test]
fn owners_choose_commander_replacements_in_turn_order_for_a_single_shuffle() {
    let mut g = game();
    g.turn.active_player = B;
    let mut targets = Vec::new();
    for owner in [A, B] {
        let commander = put(&mut g, "isamaru-hound-of-konda", owner, Zone::Graveyard);
        g.set_as_commander(commander, owner);
        targets.push(Target::Object(commander));
    }
    let spell = put(&mut g, "turn-the-earth", A, Zone::Stack);
    let req = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        &g,
        g.object(spell).unwrap().spell_effect.as_ref().unwrap(),
        A,
        Some(spell),
        None,
    );
    let entry = StackEntry::new(spell, A)
        .with_targets(targets)
        .with_target_assignments(vec![TargetAssignment {
            spec: req[0].spec.clone(),
            range: 0..2,
        }]);
    let mut dm = Decisions::default();
    resolve(&mut g, entry, &mut dm);
    assert_eq!(
        dm.choices.iter().map(|(p, _, _)| *p).collect::<Vec<_>>(),
        vec![B, A]
    );
    assert_eq!(shuffles(&g), vec![B, A]);
    assert!(g.players.iter().all(|p| p.library.is_empty()));
}

#[test]
fn a_departed_source_uses_its_block_relationships_at_the_moment_it_left() {
    use ironsmith::effects::RemoveFromCombatEffect;
    use ironsmith::events::{cause::EventCause, combat::CreatureBlockedEvent};
    use ironsmith::target::ChooseSpec;
    for (route, source_attacks) in [
        ("gomazoa", false),
        ("vortex-elemental", false),
        ("vortex-elemental", true),
    ] {
        for before_departure in [false, true] {
            let mut g = game();
            g.turn.phase = Phase::Combat;
            g.turn.active_player = if source_attacks { A } else { B };
            let source = put(&mut g, route, A, Zone::Battlefield);
            let other = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
            let (attacker, blocker, defender) = if source_attacks {
                (source, other, B)
            } else {
                (other, source, A)
            };
            let mut combat = CombatState::default();
            combat.attackers.push(AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(defender),
            });
            combat.blockers.insert(attacker, vec![blocker]);
            combat.blocked_attackers.insert(attacker);
            combat.block_declaration_complete = true;
            g.combat = Some(combat);
            let block = CreatureBlockedEvent::new(blocker, attacker)
                .with_combat_phase(g.turn_store.combat_phases_started_this_turn);
            g.turn_store.turn_history.record_event(
                &ironsmith::triggers::TriggerEvent::new(block, ironsmith::ProvNodeId::default()),
                None,
                None,
            );
            g.push_to_stack(activated(&g, source, None));
            let remove_other = |g: &mut GameState| {
                let effect = ironsmith::Effect::new(RemoveFromCombatEffect::with_spec(
                    ChooseSpec::SpecificObject(other),
                ));
                resolve(
                    g,
                    StackEntry::ability(other, B, vec![effect]),
                    &mut Decisions::default(),
                );
            };
            if before_departure {
                remove_other(&mut g);
            }
            let bounced = g
                .move_object(source, Zone::Hand, EventCause::from_game_rule())
                .unwrap();
            if !before_departure {
                remove_other(&mut g);
            }
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut Decisions::default())
                .unwrap();
            assert_eq!(g.object(bounced).unwrap().zone, Zone::Hand);
            assert_eq!(
                shuffles(&g),
                if before_departure { vec![] } else { vec![B] },
                "{route}, source attacks={source_attacks}, removed first={before_departure}"
            );
            assert_eq!(g.object(other).is_some(), before_departure);
        }
    }
}

#[test]
fn a_departed_source_shuffles_its_owners_library_after_moving_its_counters() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::events::cause::EventCause;
    for last_zone in [None, Some(Zone::Exile), Some(Zone::Graveyard)] {
        let mut g = game();
        let recipient = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
        let source = put(&mut g, "sin-unending-cataclysm", B, Zone::Battlefield);
        g.set_current_controller(source, A).unwrap();
        g.add_counters(source, CounterType::PlusOnePlusOne, 3)
            .unwrap();
        let grave = g
            .move_object(source, Zone::Graveyard, EventCause::from_game_rule())
            .unwrap();
        let mut queue = ironsmith::TriggerQueue::new();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut queue).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut g,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        let later = last_zone.map(|zone| {
            let exile = g
                .move_object(grave, Zone::Exile, EventCause::from_game_rule())
                .unwrap();
            if zone == Zone::Graveyard {
                g.move_object(exile, zone, EventCause::from_game_rule())
                    .unwrap()
            } else {
                exile
            }
        });
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(g.counter_count(recipient, CounterType::PlusOnePlusOne), 3);
        assert_eq!(shuffles(&g), vec![B]);
        assert_eq!(
            g.player(B).unwrap().library.len(),
            usize::from(last_zone.is_none())
        );
        if let Some(later) = later {
            assert_eq!(g.object(later).map(|object| object.zone), last_zone);
        }
    }
}

#[test]
fn a_targeted_opponent_can_shuffle_a_library_owned_by_someone_else() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    for owner in [A, B] {
        let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let witness = put(&mut g, "cosi-s-trickster", C, Zone::Battlefield);
        // The first choice returns the land. The target then shuffles the creature.
        put(&mut g, "wastes", B, Zone::Battlefield);
        let creature = put(&mut g, "grizzly-bears", owner, Zone::Battlefield);
        g.set_current_controller(creature, B).unwrap();
        let source = put(&mut g, "teferi-temporal-pilgrim", A, Zone::Battlefield);
        let ultimate = g
            .object(source)
            .unwrap()
            .abilities
            .iter()
            .rev()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(ability) => Some(ability),
                _ => None,
            })
            .unwrap();
        let entry = StackEntry::ability(source, A, ultimate.effects.clone())
            .with_targets(vec![Target::Player(B)]);
        g.push_to_stack(entry);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(g.player(B).unwrap().hand.len(), 1);
        assert_eq!(shuffles(&g), vec![owner]);
        assert_eq!(
            shuffle_trigger_count(&mut g, witness),
            usize::from(owner == B)
        );
    }
}
