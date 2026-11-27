use super::*;
use ironsmith::combat_state::{
    AttackTarget, AttackerInfo, CombatState, is_attacking_alone, is_blocking, is_blocking_alone,
};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::events::cause::EventCause;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::{Ability, Effect, ObjectFilter, StackEntry, Target};

const B: PlayerId = PlayerId(1);

fn combat(attacker: ObjectId, blocker: ObjectId, defender: PlayerId) -> CombatState {
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(defender),
    });
    combat.blockers.insert(attacker, vec![blocker]);
    combat.blocked_attackers.insert(attacker);
    combat.block_declaration_complete = true;
    combat
}

fn targets(g: &GameState, spell: ObjectId) -> Vec<Target> {
    ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        g,
        g.object(spell).unwrap().spell_effect.as_ref().unwrap(),
        B,
        Some(spell),
        None,
    )
    .into_iter()
    .flat_map(|requirement| requirement.legal_targets)
    .collect()
}

#[test]
fn a_spell_on_another_stack_can_target_and_destroy_a_nearby_attacker() {
    let seats: Vec<_> = (0..8).map(PlayerId).collect();
    let mut g = GameState::new(seats.iter().map(|p| p.0.to_string()).collect(), 20);
    g.restore_grand_melee(seats).unwrap();
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    let attacker = put(&mut g, "adanto-vanguard", A, Zone::Battlefield);
    let blocker = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
    g.combat = Some(combat(attacker, blocker, B));
    let kill = put(&mut g, "kill-shot", B, Zone::Stack);
    let buff = put(&mut g, "righteousness", B, Zone::Hand);
    let alarum = put(&mut g, "alarum", B, Zone::Hand);
    g.select_grand_melee_turn_marker(2).unwrap();
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    let far = put(&mut g, "grizzly-bears", PlayerId(4), Zone::Battlefield);
    let far_blocker = put(&mut g, "grizzly-bears", PlayerId(5), Zone::Battlefield);
    g.combat = Some(combat(far, far_blocker, PlayerId(5)));
    // Adjoining players put instants on this stack until the defender has
    // priority on both stacks. The far combat stays outside their range.
    for seat in [PlayerId(3), PlayerId(2)] {
        g.select_grand_melee_stack_for_player(seat, 2).unwrap();
        let spell = put(&mut g, "opt", seat, Zone::Stack);
        g.push_to_stack(StackEntry::new(spell, seat));
    }
    for lane in [1, 2] {
        g.select_grand_melee_stack_for_player(B, lane).unwrap();
        assert!(
            targets(&g, kill).contains(&Target::Object(attacker)),
            "lane {lane}"
        );
        assert!(!targets(&g, kill).contains(&Target::Object(far)));
        assert!(targets(&g, buff).contains(&Target::Object(blocker)));
        assert!(!targets(&g, alarum).contains(&Target::Object(attacker)));
        assert_eq!(g.calculated_power(attacker), Some(3));
        for (a, b) in [(attacker, blocker), (far, far_blocker)] {
            let a = ObjectSnapshot::from_object_id(&g, a).unwrap();
            let b = ObjectSnapshot::from_object_id(&g, b).unwrap();
            assert!(a.attacking && a.attacking_alone && b.blocking && b.blocking_alone);
        }
    }
    g.push_to_stack(StackEntry::new(kill, B).with_targets(vec![Target::Object(attacker)]));
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    assert!(g.object(attacker).is_none());
    assert_eq!(g.player(A).unwrap().graveyard.len(), 1);
    assert!(g.object(far).is_some());
}

#[test]
fn removing_an_attacker_leaves_its_blocker_blocking_without_an_opponent() {
    for leaves_battlefield in [false, true] {
        let mut g = game();
        let attacker = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
        let blocker = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        g.combat = Some(combat(attacker, blocker, B));
        let buff = put(&mut g, "righteousness", B, Zone::Stack);
        if leaves_battlefield {
            g.move_object(attacker, Zone::Hand, EventCause::from_game_rule())
                .unwrap();
        } else {
            let effect = Effect::new(ironsmith::effects::RemoveFromCombatEffect::with_spec(
                ironsmith::target::ChooseSpec::SpecificObject(attacker),
            ));
            g.push_to_stack(StackEntry::ability(attacker, A, vec![effect]));
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker)
                .unwrap();
        }
        let active = g.combat.as_ref().unwrap();
        assert!(is_blocking(active, blocker));
        assert!(ironsmith::combat_state::get_blockers(active, attacker).is_empty());
        assert!(is_blocking_alone(&g, blocker));
        g.push_to_stack(StackEntry::new(buff, B).with_targets(vec![Target::Object(blocker)]));
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(g.calculated_power(blocker), Some(9));
        ironsmith::combat_state::end_combat(g.combat.as_mut().unwrap());
        assert!(!is_blocking(g.combat.as_ref().unwrap(), blocker));
    }
}

#[test]
fn attacking_alone_counts_both_teammates_in_the_same_combat() {
    let c = PlayerId(2);
    let d = PlayerId(3);
    let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    g.set_teams(vec![vec![A, B], vec![c, d]]).unwrap();
    g.enable_shared_team_turns().unwrap();
    let first = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    let second = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo {
        creature: first,
        target: AttackTarget::Player(c),
    });
    g.combat = Some(combat);
    assert!(is_attacking_alone(&g, first));
    g.combat.as_mut().unwrap().attackers.push(AttackerInfo {
        creature: second,
        target: AttackTarget::Player(d),
    });
    for id in [first, second] {
        assert!(!is_attacking_alone(&g, id));
        assert!(
            !ObjectSnapshot::from_object_id(&g, id)
                .unwrap()
                .attacking_alone
        );
    }
}

#[test]
fn deaths_match_the_creatures_last_combat_role() {
    for (attacking, blocking, blocked, unblocked, nonblocking, expected) in [
        (false, true, false, false, false, vec![2]),
        (true, true, false, false, false, vec![0, 1, 2]),
        (false, false, false, false, true, vec![0, 1, 3]),
        (false, false, true, false, false, vec![0]),
        (false, false, false, true, false, vec![1]),
    ] {
        let mut g = game();
        let ids = [A, A, B, B].map(|p| put(&mut g, "grizzly-bears", p, Zone::Battlefield));
        g.combat = Some(combat(ids[0], ids[2], B));
        g.combat.as_mut().unwrap().attackers.push(AttackerInfo {
            creature: ids[1],
            target: AttackTarget::Player(B),
        });
        let mut filter = ObjectFilter::creature();
        filter.attacking = attacking;
        filter.blocking = blocking;
        filter.blocked = blocked;
        filter.unblocked = unblocked;
        filter.nonblocking = nonblocking;
        let def = CardDefinitionBuilder::new(CardId::new(), "Combat witness")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                ironsmith::triggers::Trigger::dies(filter),
                vec![Effect::gain_life(1)],
            ))
            .build();
        let witness = g.create_object_from_definition(&def, B, Zone::Battlefield);
        let mut matched = Vec::new();
        for (i, id) in ids.into_iter().enumerate() {
            let mut position = g.clone();
            position
                .move_object(id, Zone::Graveyard, EventCause::from_game_rule())
                .unwrap();
            let mut q = ironsmith::TriggerQueue::new();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut position, &mut q).unwrap();
            if q.entries.iter().any(|e| e.source == witness) {
                matched.push(i);
            }
        }
        assert_eq!(matched, expected);
    }
}

/// Players in seats 3, 2, and 1 respond in sequence to give seat 0 priority
/// on the second stack (CR 807.5a). Both turns are in their declare blockers step.
fn overlapping_stacks() -> GameState {
    let seats: Vec<_> = (0..8).map(PlayerId).collect();
    let mut g = GameState::new(seats.iter().map(|p| p.0.to_string()).collect(), 20);
    g.restore_grand_melee(seats).unwrap();
    for marker in [1, 2] {
        g.select_grand_melee_turn_marker(marker).unwrap();
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
        g.combat = Some(CombatState {
            block_declaration_complete: true,
            ..Default::default()
        });
    }
    for seat in [PlayerId(3), PlayerId(2), B] {
        g.select_grand_melee_stack_for_player(seat, 2).unwrap();
        let spell = put(&mut g, "opt", seat, Zone::Stack);
        g.push_to_stack(StackEntry::new(spell, seat));
    }
    g.select_grand_melee_stack_for_player(A, 1).unwrap();
    g
}

fn block_event(g: &mut GameState, blocker: ObjectId, attacker: ObjectId) {
    let event = ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::CreatureBlockedEvent::new(blocker, attacker)
            .with_combat_phase(g.turn_store.combat_phases_started_this_turn),
        ironsmith::ProvNodeId::default(),
    );
    g.turn_store.turn_history.record_event(&event, None, None);
}

struct BlockAttacker(ObjectId);
impl DecisionMaker for BlockAttacker {
    fn decide_objects(
        &mut self,
        _: &GameState,
        choice: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if choice
            .candidates
            .iter()
            .any(|candidate| candidate.legal && candidate.id == self.0)
        {
            vec![self.0]
        } else {
            choice
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .take(choice.min)
                .map(|candidate| candidate.id)
                .collect()
        }
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
}

#[test]
fn spells_and_abilities_change_the_combat_their_targets_belong_to() {
    let mut base = overlapping_stacks();
    let [first, unblocked, third] =
        [A; 3].map(|p| put(&mut base, "grizzly-bears", p, Zone::Battlefield));
    let [left, right] = [B; 2].map(|p| put(&mut base, "grizzly-bears", p, Zone::Battlefield));
    let mut near = combat(first, left, B);
    for id in [unblocked, third] {
        near.attackers.push(AttackerInfo {
            creature: id,
            target: AttackTarget::Player(B),
        });
    }
    near.blockers.insert(third, vec![right]);
    near.blocked_attackers.insert(third);
    base.combat = Some(near);
    block_event(&mut base, left, first);
    block_event(&mut base, right, third);
    for route in [
        "curtain-of-light",
        "dazzling-beauty",
        "general-jarkeld",
        "sorrow-s-path",
        "false-orders",
        "balduvian-warlord",
    ] {
        let ability = matches!(
            route,
            "general-jarkeld" | "sorrow-s-path" | "balduvian-warlord"
        );
        for marker in [1, 2] {
            let mut g = base.clone();
            g.select_grand_melee_stack_for_player(A, marker).unwrap();
            let source = put(
                &mut g,
                route,
                A,
                if ability {
                    Zone::Battlefield
                } else {
                    Zone::Stack
                },
            );
            let mut entry = if ability {
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
                StackEntry::ability(source, A, ability.effects.clone())
            } else {
                StackEntry::new(source, A)
            };
            entry.targets = match route {
                "general-jarkeld" => vec![first, third],
                "sorrow-s-path" => vec![left, right],
                "false-orders" | "balduvian-warlord" => vec![left],
                _ => vec![unblocked],
            }
            .into_iter()
            .map(Target::Object)
            .collect();
            g.push_to_stack(entry);
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut BlockAttacker(unblocked))
                .unwrap();
            if matches!(route, "general-jarkeld" | "sorrow-s-path") {
                let mut events = g
                    .turn_store
                    .turn_history
                    .event_records
                    .iter()
                    .filter_map(|record| {
                        record
                            .event
                            .downcast::<ironsmith::events::CreatureBlockedEvent>()
                    })
                    .filter(|event| event.added_by_effect)
                    .map(|event| (event.attacker, event.blocker, event.blocker_was_blocking))
                    .collect::<Vec<_>>();
                events.sort_unstable();
                let already_blocking = route == "general-jarkeld";
                assert_eq!(
                    events,
                    vec![
                        (first, right, already_blocking),
                        (third, left, already_blocking)
                    ],
                    "{route}, marker {marker}"
                );
            }
            g.select_grand_melee_turn_marker(1).unwrap();
            if matches!(route, "sorrow-s-path" | "general-jarkeld") {
                let history = &g.turn_store.turn_history;
                let phase = g.turn_store.combat_phases_started_this_turn;
                assert!(history.blockers_of_in_combat(first, phase).contains(&right));
                assert!(history.blockers_of_in_combat(third, phase).contains(&left));
            }
            let c = g.combat.as_ref().unwrap();
            match route {
                "general-jarkeld" | "sorrow-s-path" => {
                    assert_eq!(ironsmith::combat_state::get_blockers(c, first), [right]);
                    assert_eq!(ironsmith::combat_state::get_blockers(c, third), [left]);
                }
                "false-orders" if marker == 2 => {
                    // The target must be controlled by the defending player of
                    // this spell's combat. Seat B defends only on marker one.
                    assert_eq!(ironsmith::combat_state::get_blockers(c, first), [left]);
                    assert!(ironsmith::combat_state::is_unblocked(c, unblocked));
                }
                "balduvian-warlord" | "false-orders" => {
                    assert!(
                        !ironsmith::combat_state::is_blocked(c, first),
                        "{route}, marker {marker}"
                    );
                    assert_eq!(ironsmith::combat_state::get_blockers(c, unblocked), [left]);
                }
                _ => assert!(
                    ironsmith::combat_state::is_blocked(c, unblocked),
                    "{route}, marker {marker}"
                ),
            }
            g.select_grand_melee_turn_marker(2).unwrap();
            assert!(g.combat.as_ref().unwrap().blocked_attackers.is_empty());
        }
    }
}

#[test]
fn ninjutsu_uses_the_returned_attackers_combat_and_defender() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let mut base = overlapping_stacks();
    let attacker = put(&mut base, "grizzly-bears", A, Zone::Battlefield);
    base.combat.as_mut().unwrap().attackers.push(AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(B),
    });
    // Marker two grants priority but is outside combat. The activation
    // timing and payment must both use the attacking player's turn.
    base.select_grand_melee_turn_marker(2).unwrap();
    base.turn.phase = Phase::FirstMain;
    base.turn.step = None;
    for marker in [1, 2] {
        let mut g = base.clone();
        g.select_grand_melee_stack_for_player(A, marker).unwrap();
        g.turn.priority_player = Some(A);
        g.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::Blue, 2);
        let ninja = put(&mut g, "ninja-of-the-deep-hours", A, Zone::Hand);
        assert!(compute_legal_actions(&g, A).unwrap().iter().any(
            |action| matches!(action,LegalAction::ActivateAbility{source,..} if *source==ninja)
        ));
        let mut ctx = EffectContext::new_default(ninja, A);
        ironsmith::effects::NinjutsuCostEffect::new()
            .execute(&mut g, &mut ctx)
            .unwrap();
        ironsmith::effects::NinjutsuEffect::new()
            .execute(&mut g, &mut ctx)
            .unwrap();
        assert!(g.object(attacker).is_none());
        g.select_grand_melee_turn_marker(1).unwrap();
        let attackers = &g.combat.as_ref().unwrap().attackers;
        assert_eq!(attackers.len(), 1);
        let entering = attackers[0].creature;
        assert_eq!(
            g.object(entering).unwrap().name.as_str(),
            "Ninja of the Deep Hours"
        );
        assert!(g.is_tapped(entering));
        assert!(ironsmith::combat_state::is_unblocked(
            g.combat.as_ref().unwrap(),
            entering
        ));
        assert_eq!(attackers[0].target, AttackTarget::Player(B));
        g.select_grand_melee_turn_marker(2).unwrap();
        assert!(g.combat.as_ref().unwrap().attackers.is_empty());
    }
}

#[test]
fn sneak_follows_the_casters_declare_blockers_step_on_either_stack() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_state::Step;

    let mut base = overlapping_stacks();
    let attacker = put(&mut base, "grizzly-bears", A, Zone::Battlefield);
    base.combat.as_mut().unwrap().attackers.push(AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(B),
    });
    put(&mut base, "grizzly-bears", A, Zone::Graveyard);
    let card = put(&mut base, "leonardo-s-technique", A, Zone::Hand);
    base.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ironsmith::ManaSymbol::White, 2);
    for (own_step, other_phase, other_step, expected) in [
        (Step::DeclareBlockers, Phase::FirstMain, None, true),
        (
            Step::CombatDamage,
            Phase::Combat,
            Some(Step::DeclareBlockers),
            false,
        ),
    ] {
        let mut g = base.clone();
        g.turn.step = Some(own_step);
        g.select_grand_melee_turn_marker(2).unwrap();
        g.turn.phase = other_phase;
        g.turn.step = other_step;
        for marker in [1, 2] {
            g.select_grand_melee_stack_for_player(A, marker).unwrap();
            g.turn.priority_player = Some(A);
            let offered = compute_legal_actions(&g, A).unwrap().iter().any(
                |action| matches!(action, LegalAction::CastSpell {spell_id, ..} if *spell_id == card));
            assert_eq!(offered, expected, "own step {own_step:?}, marker {marker}");
        }
    }
}

#[test]
fn sorrows_path_exchanges_a_blocker_whose_attacker_left() {
    let mut g = overlapping_stacks();
    let [first, second] = [A; 2].map(|p| put(&mut g, "grizzly-bears", p, Zone::Battlefield));
    let [left, right] = [B; 2].map(|p| put(&mut g, "grizzly-bears", p, Zone::Battlefield));
    let mut c = combat(first, left, B);
    c.attackers.push(AttackerInfo {
        creature: second,
        target: AttackTarget::Player(B),
    });
    c.blockers.insert(second, vec![right]);
    c.blocked_attackers.insert(second);
    g.combat = Some(c);
    g.move_object(first, Zone::Hand, EventCause::from_game_rule())
        .unwrap();
    g.select_grand_melee_stack_for_player(A, 2).unwrap();
    let path = put(&mut g, "sorrow-s-path", A, Zone::Battlefield);
    let effects = g
        .object(path)
        .unwrap()
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a.effects.clone()),
            _ => None,
        })
        .unwrap();
    g.push_to_stack(
        StackEntry::ability(path, A, effects)
            .with_targets(vec![Target::Object(left), Target::Object(right)]),
    );
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    g.select_grand_melee_turn_marker(1).unwrap();
    let c = g.combat.as_ref().unwrap();
    assert_eq!(ironsmith::combat_state::get_blockers(c, second), [left]);
    assert!(!is_blocking(c, right));
}

#[test]
fn portal_mage_reselects_the_attacker_on_either_stack() {
    let mut base = overlapping_stacks();
    for marker in [2, 1] {
        base.select_grand_melee_turn_marker(marker).unwrap();
        base.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        base.combat.as_mut().unwrap().block_declaration_complete = false;
    }
    let walker = CardDefinitionBuilder::new(CardId::new(), "Defending planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .build();
    let walker = base.create_object_from_definition(&walker, B, Zone::Battlefield);
    let attacker = put(&mut base, "grizzly-bears", A, Zone::Battlefield);
    base.combat.as_mut().unwrap().attackers.push(AttackerInfo {
        creature: attacker,
        target: AttackTarget::Planeswalker(walker),
    });
    let mage = put(&mut base, "portal-mage", A, Zone::Battlefield);
    let effects = base
        .object(mage)
        .unwrap()
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(a) => Some(a.effects.clone()),
            _ => None,
        })
        .unwrap();
    for marker in [1, 2] {
        let mut g = base.clone();
        g.select_grand_melee_stack_for_player(A, marker).unwrap();
        g.push_to_stack(
            StackEntry::ability(mage, A, effects.clone())
                .with_targets(vec![Target::Object(attacker)]),
        );
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut BlockAttacker(attacker))
            .unwrap();
        g.select_grand_melee_turn_marker(1).unwrap();
        assert_eq!(
            g.combat.as_ref().unwrap().attackers[0].target,
            AttackTarget::Player(B),
            "marker {marker}"
        );
    }
}

#[test]
fn a_sneaked_permanent_enters_its_controllers_combat_on_either_stack() {
    let mut base = overlapping_stacks();
    let walker = CardDefinitionBuilder::new(CardId::new(), "Defending planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .build();
    let walker = base.create_object_from_definition(&walker, B, Zone::Battlefield);
    for marker in [1, 2] {
        for defender_leaves in [false, true] {
            let mut g = base.clone();
            g.select_grand_melee_stack_for_player(A, marker).unwrap();
            let spell = put(&mut g, "donatello-gadget-master", A, Zone::Stack);
            g.record_sneak_attack_target(spell, AttackTarget::Planeswalker(walker));
            let mut entry = StackEntry::new(spell, A);
            entry.casting_method = ironsmith::CastingMethod::Alternative(0);
            g.push_to_stack(entry);
            if defender_leaves {
                g.move_object(walker, Zone::Hand, EventCause::from_game_rule())
                    .unwrap();
            }
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker)
                .unwrap();
            let entered = g
                .battlefield
                .iter()
                .copied()
                .find(|id| g.object(*id).unwrap().name.as_str() == "Donatello, Gadget Master")
                .unwrap();
            assert!(g.is_tapped(entered));
            g.select_grand_melee_turn_marker(1).unwrap();
            let near = g.combat.as_ref().unwrap();
            assert_eq!(
                ironsmith::combat_state::get_attack_target(near, entered),
                (!defender_leaves).then_some(&AttackTarget::Planeswalker(walker)),
                "marker {marker}"
            );
            assert_eq!(
                near.attacked_permanent_types.contains_key(&walker),
                !defender_leaves
            );
            g.select_grand_melee_turn_marker(2).unwrap();
            assert!(g.combat.as_ref().unwrap().attackers.is_empty());
        }
    }
}

#[test]
fn attacking_a_player_enables_their_cost_reductions_on_either_stack() {
    use ironsmith::ManaSymbol;
    use ironsmith::decision::{LegalAction, compute_legal_actions};

    let mut base = overlapping_stacks();
    let attacker = put(&mut base, "grizzly-bears", A, Zone::Battlefield);
    let walker = CardDefinitionBuilder::new(CardId::new(), "Defending planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .build();
    let walker = base.create_object_from_definition(&walker, B, Zone::Battlefield);
    put(&mut base, "grizzly-bears", B, Zone::Graveyard);
    for land in ["forest", "plains"] {
        let land = put(&mut base, land, B, Zone::Battlefield);
        base.tap(land);
    }
    for (card, symbol, mana) in [
        ("heroic-return", ManaSymbol::White, 4),
        ("swat-away", ManaSymbol::Blue, 2),
        ("red-alert", ManaSymbol::Red, 1),
        ("qasali-ambusher", ManaSymbol::White, 0),
    ] {
        for destination in [AttackTarget::Player(B), AttackTarget::Planeswalker(walker)] {
            let mut g = base.clone();
            let spell = put(&mut g, card, B, Zone::Hand);
            g.player_mut(B).unwrap().mana_pool.add(symbol, mana);
            g.combat.as_mut().unwrap().attackers.push(AttackerInfo {
                creature: attacker,
                target: destination.clone(),
            });
            for marker in [1, 2] {
                g.select_grand_melee_stack_for_player(B, marker).unwrap();
                g.turn.priority_player = Some(B);
                let offered = compute_legal_actions(&g, B).unwrap().iter().any(|action| {
                    matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)
                });
                assert_eq!(
                    offered,
                    destination == AttackTarget::Player(B),
                    "{card}, marker {marker}"
                );
            }
        }
    }
}

#[test]
fn capricopian_can_be_activated_by_the_attacked_player_on_either_stack() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_state::Step;

    let mut base = overlapping_stacks();
    let goat = put(&mut base, "capricopian", A, Zone::Battlefield);
    base.add_counters(goat, CounterType::PlusOnePlusOne, 2);
    let walker = CardDefinitionBuilder::new(CardId::new(), "Defending planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .build();
    let walker = base.create_object_from_definition(&walker, B, Zone::Battlefield);
    base.add_counters(walker, CounterType::Loyalty, 3);
    base.player_mut(B)
        .unwrap()
        .mana_pool
        .add(ironsmith::ManaSymbol::Colorless, 2);

    for (step, target, expected) in [
        (Step::DeclareAttackers, AttackTarget::Player(B), true),
        (
            Step::DeclareAttackers,
            AttackTarget::Planeswalker(walker),
            false,
        ),
        (Step::DeclareBlockers, AttackTarget::Player(B), false),
    ] {
        let mut g = base.clone();
        g.turn.step = Some(step);
        let combat = g.combat.as_mut().unwrap();
        combat.block_declaration_complete = step == Step::DeclareBlockers;
        combat.attackers.push(AttackerInfo {
            creature: goat,
            target,
        });
        for marker in [1, 2] {
            g.select_grand_melee_stack_for_player(B, marker).unwrap();
            g.turn.priority_player = Some(B);
            let offered = compute_legal_actions(&g, B).unwrap().iter().any(|action| {
                matches!(action, LegalAction::ActivateAbility { source, .. } if *source == goat)
            });
            assert_eq!(offered, expected, "{step:?}, marker {marker}");
        }
    }
}

#[test]
fn veteran_bodyguard_uses_the_last_combat_role_of_a_sacrificed_damage_source() {
    for (attacking, blocked) in [(true, false), (true, true), (false, false)] {
        let mut g = game();
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
        let fanatic = put(&mut g, "mogg-fanatic", A, Zone::Battlefield);
        let bodyguard = put(&mut g, "veteran-bodyguard", B, Zone::Battlefield);
        let blocker = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        let mut c = CombatState {
            block_declaration_complete: true,
            ..Default::default()
        };
        if attacking {
            c.attackers.push(AttackerInfo {
                creature: fanatic,
                target: AttackTarget::Player(B),
            });
        }
        if blocked {
            c.blockers.insert(fanatic, vec![blocker]);
            c.blocked_attackers.insert(fanatic);
        }
        g.combat = Some(c);
        let effects = g
            .object(fanatic)
            .unwrap()
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Activated(a) => Some(a.effects.clone()),
                _ => None,
            })
            .unwrap();
        let mut entry =
            StackEntry::ability(fanatic, A, effects).with_targets(vec![Target::Player(B)]);
        entry.source_snapshot = ObjectSnapshot::from_object_id(&g, fanatic);
        g.push_to_stack(entry);
        g.move_object(fanatic, Zone::Graveyard, EventCause::from_game_rule())
            .unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker)
            .unwrap();
        let redirect = attacking && !blocked;
        assert_eq!(g.player(B).unwrap().life, if redirect { 20 } else { 19 });
        assert_eq!(g.damage_on(bodyguard), u32::from(redirect));
    }
}

#[test]
fn an_attackers_static_ability_counts_its_own_defenders_lands() {
    let mut g = overlapping_stacks();
    let source = put(&mut g, "elturel-survivors", A, Zone::Battlefield);
    g.combat.as_mut().unwrap().attackers.push(AttackerInfo {
        creature: source,
        target: AttackTarget::Player(B),
    });
    put(&mut g, "forest", B, Zone::Battlefield);
    put(&mut g, "plains", B, Zone::Battlefield);
    let distant = PlayerId(5);
    put(&mut g, "forest", distant, Zone::Battlefield);
    g.select_grand_melee_turn_marker(2).unwrap();
    let attacker = put(&mut g, "grizzly-bears", PlayerId(4), Zone::Battlefield);
    g.combat.as_mut().unwrap().attackers.push(AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(distant),
    });
    for marker in [1, 2] {
        g.select_grand_melee_stack_for_player(B, marker).unwrap();
        assert_eq!(g.calculated_power(source), Some(2));
    }
    g.select_grand_melee_turn_marker(1).unwrap();
    g.combat.as_mut().unwrap().attackers[0].target = AttackTarget::Nothing {
        defending_player: Some(B),
        was_planeswalker: true,
    };
    for marker in [1, 2] {
        g.select_grand_melee_stack_for_player(B, marker).unwrap();
        assert_eq!(
            ironsmith::combat_state::defending_player_for_attack_event(
                &g,
                ironsmith::triggers::AttackEventTarget::Nothing,
                source
            ),
            Some(B)
        );
        assert_eq!(g.calculated_power(source), Some(2));
    }
}

#[test]
fn general_jarkeld_can_exchange_blockers_only_across_legal_defenders() {
    let other_defender = PlayerId(2);
    for shared in [false, true] {
        let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
        if shared {
            g.set_teams(vec![vec![A, PlayerId(3)], vec![B, other_defender]])
                .unwrap();
            g.enable_shared_team_turns().unwrap();
        }
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
        let [first, second] = [A; 2].map(|p| put(&mut g, "grizzly-bears", p, Zone::Battlefield));
        let [left, right] =
            [B, other_defender].map(|p| put(&mut g, "grizzly-bears", p, Zone::Battlefield));
        let mut c = combat(first, left, B);
        c.attackers.push(AttackerInfo {
            creature: second,
            target: AttackTarget::Player(other_defender),
        });
        c.blockers.insert(second, vec![right]);
        c.blocked_attackers.insert(second);
        g.combat = Some(c);
        let source = put(&mut g, "general-jarkeld", A, Zone::Battlefield);
        let effects = g
            .object(source)
            .unwrap()
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Activated(a) => Some(a.effects.clone()),
                _ => None,
            })
            .unwrap();
        g.push_to_stack(
            StackEntry::ability(source, A, effects)
                .with_targets(vec![Target::Object(first), Target::Object(second)]),
        );
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker)
            .unwrap();
        let c = g.combat.as_ref().unwrap();
        assert_eq!(
            ironsmith::combat_state::get_blockers(c, first),
            [if shared { right } else { left }]
        );
        assert_eq!(
            ironsmith::combat_state::get_blockers(c, second),
            [if shared { left } else { right }]
        );
    }
}
