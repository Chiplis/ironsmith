//! CR 701.22b–d and 121.7. Observe each private look and placement decision.
use super::*;
use ironsmith::decisions::context::{BooleanContext, PartitionContext, ViewCardsContext};
use ironsmith::effects::EachPlayerScryEffect;
use ironsmith::events::{
    KeywordActionEvent, KeywordActionKind, PlayerVote, PlayersFinishedVotingEvent,
};

const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

#[derive(Debug, PartialEq)]
enum Step {
    Offer(PlayerId),
    Look(PlayerId, usize),
    Place(PlayerId),
}
use Step::{Look, Offer, Place};

#[derive(Default)]
struct ScryChoices {
    steps: Vec<Step>,
    declined: Vec<PlayerId>,
    bottom_count: Option<usize>,
    pause_at: Option<PlayerId>,
    paused: bool,
    tops_at_placement: Vec<Vec<ObjectId>>,
    alice_hand_at_placement: Vec<usize>,
}

impl DecisionMaker for ScryChoices {
    fn awaiting_choice(&self) -> bool {
        self.paused
    }

    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.steps.push(Offer(ctx.player));
        !self.declined.contains(&ctx.player)
    }

    fn view_cards(
        &mut self,
        _: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        _: &ViewCardsContext,
    ) {
        self.steps.push(Look(viewer, cards.len()));
    }

    fn decide_partition(&mut self, g: &GameState, ctx: &PartitionContext) -> Vec<ObjectId> {
        self.steps.push(Place(ctx.player));
        self.tops_at_placement.push(
            g.players
                .iter()
                .filter_map(|p| p.library.last().copied())
                .collect(),
        );
        self.alice_hand_at_placement
            .push(g.player(A).unwrap().hand.len());
        self.paused = self.pause_at == Some(ctx.player);
        ctx.cards
            .iter()
            .take(self.bottom_count.unwrap_or(1))
            .map(|(id, _)| *id)
            .collect()
    }
}

fn fill_libraries(g: &mut GameState) -> Vec<ObjectId> {
    let land = card("wastes");
    (0..g.players.len())
        .map(|i| {
            let player = PlayerId(i as u8);
            g.create_object_from_definition(&land, player, Zone::Library);
            g.create_object_from_definition(&land, player, Zone::Library);
            g.create_object_from_definition(&land, player, Zone::Library)
        })
        .collect()
}

fn trigger(g: &GameState, source: ObjectId, controller: PlayerId) -> ironsmith::StackEntry {
    let program = g
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t.effects.clone()),
            _ => None,
        })
        .unwrap();
    ironsmith::StackEntry::ability(source, controller, program)
}

fn optional_steps(offers: &[PlayerId], accepted: &[PlayerId], count: usize) -> Vec<Step> {
    offers
        .iter()
        .copied()
        .map(Offer)
        .chain(accepted.iter().map(|player| Look(*player, count)))
        .chain(accepted.iter().copied().map(Place))
        .collect()
}

#[test]
fn eager_construct_offers_and_private_looks_precede_placements() {
    let construct = card("eager-construct");
    for seats in 2..=5u8 {
        let mut base = GameState::new((0..seats).map(|i| format!("P{i}")).collect(), 20);
        let tops = fill_libraries(&mut base);
        let source = base.create_object_from_definition(&construct, A, Zone::Battlefield);
        let ability = trigger(&base, source, A);
        for first in 0..seats {
            let order: Vec<_> = (first..seats).chain(0..first).map(PlayerId).collect();
            for mask in 0..(1u32 << seats) {
                let mut g = base.clone();
                g.turn.active_player = PlayerId(first);
                let (accepted, declined): (Vec<_>, Vec<_>) = order
                    .iter()
                    .copied()
                    .partition(|player| mask & (1 << player.0) != 0);
                let mut choices = ScryChoices {
                    declined,
                    ..Default::default()
                };
                g.push_to_stack(ability.clone());
                ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut choices).unwrap();
                assert_eq!(
                    choices.steps,
                    optional_steps(&order, &accepted, 1),
                    "seats={seats}, first={first}, accepted={accepted:?}"
                );
                assert!(choices.tops_at_placement.iter().all(|v| v == &tops));
                for &player in &order {
                    let library = &g.player(player).unwrap().library;
                    let position = if accepted.contains(&player) {
                        library.first()
                    } else {
                        library.last()
                    };
                    assert_eq!(position, Some(&tops[player.0 as usize]));
                }
            }
        }
    }
}

#[test]
fn mandatory_scry_applies_each_replacement_and_observes_each_bottom_choice_once() {
    let mut g = game();
    let tops = fill_libraries(&mut g);
    let source = put(&mut g, "eligeth-crossroads-augur", A, Zone::Battlefield);
    put(&mut g, "kenessos-priest-of-thassa", B, Zone::Battlefield);
    let mut choices = ScryChoices::default();
    let result = EachPlayerScryEffect::new(1, PlayerFilter::Any)
        .execute(&mut g, &mut EffectContext::new(source, A, &mut choices))
        .unwrap();
    assert_eq!(choices.steps, [Look(B, 2), Place(B)]);
    assert_eq!(choices.alice_hand_at_placement, [0]);
    assert_eq!(choices.tops_at_placement, [tops.clone()]);
    assert_eq!(g.player(A).unwrap().hand.len(), 1);
    assert_eq!(g.player(B).unwrap().library.first(), Some(&tops[1]));
    assert_eq!(result.as_count(), Some(2));
    let events: Vec<_> = result
        .events
        .iter()
        .filter_map(|e| e.downcast::<KeywordActionEvent>())
        .filter(|e| {
            matches!(
                e.action,
                KeywordActionKind::Scry | KeywordActionKind::ScryToBottom
            )
        })
        .map(|e| (e.action, e.player, e.amount))
        .collect();
    assert_eq!(
        events,
        [
            (KeywordActionKind::Scry, B, 2),
            (KeywordActionKind::ScryToBottom, B, 1)
        ]
    );
    let scry_position = result
        .events
        .iter()
        .position(|event| {
            event
                .downcast::<KeywordActionEvent>()
                .is_some_and(|action| action.action == KeywordActionKind::Scry)
        })
        .unwrap();
    let draw_position = result
        .events
        .iter()
        .position(|event| {
            event
                .downcast::<ironsmith::events::CardsDrawnEvent>()
                .is_some()
        })
        .unwrap();
    assert!(
        scry_position < draw_position,
        "the surviving scry precedes Eligeth's draw"
    );
}

#[test]
fn empty_libraries_still_scry_and_scry_zero_produces_no_event() {
    for count in [0, 2] {
        let mut g = game();
        let source = g.new_object_id();
        let mut choices = ScryChoices::default();
        let result = EachPlayerScryEffect::new(count, PlayerFilter::Any)
            .execute(&mut g, &mut EffectContext::new(source, A, &mut choices))
            .unwrap();
        let scries: Vec<_> = result
            .events
            .iter()
            .filter_map(|e| e.downcast::<KeywordActionEvent>())
            .map(|e| (e.action, e.player, e.amount))
            .collect();
        let expected = if count > 0 {
            vec![
                (KeywordActionKind::Scry, A, 0),
                (KeywordActionKind::Scry, B, 0),
            ]
        } else {
            vec![]
        };
        assert_eq!(scries, expected);
        assert!(choices.steps.is_empty());
    }
}

#[test]
fn myr_custodian_finishes_its_own_scry_then_the_opponents_scry_together() {
    let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    let tops = fill_libraries(&mut g);
    let source = put(&mut g, "myr-custodian", A, Zone::Battlefield);
    g.push_to_stack(trigger(&g, source, A));
    let mut choices = ScryChoices::default();
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut choices).unwrap();
    assert_eq!(
        choices.steps,
        [
            Look(A, 2),
            Place(A),
            Offer(B),
            Offer(C),
            Look(B, 1),
            Look(C, 1),
            Place(B),
            Place(C)
        ]
    );
    assert_eq!(choices.tops_at_placement[0], tops);
    assert_eq!(choices.tops_at_placement[1], choices.tops_at_placement[2]);
    assert_ne!(choices.tops_at_placement[1][0], tops[0]);
}

#[test]
fn model_of_unity_groups_its_controller_with_eligible_voters_in_turn_order() {
    let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    g.turn.active_player = B;
    let tops = fill_libraries(&mut g);
    let source = put(&mut g, "model-of-unity", A, Zone::Battlefield);
    let votes = [(A, 0), (B, 0), (C, 1)]
        .into_iter()
        .map(|(player, option_index)| PlayerVote {
            player,
            option_index,
            option_name: option_index.to_string(),
            object_vote: None,
        })
        .collect();
    let event = PlayersFinishedVotingEvent::new(
        source,
        B,
        votes,
        [(0, 2), (1, 1)].into(),
        vec!["0".into(), "1".into()],
    );
    g.push_to_stack(trigger(&g, source, A).with_triggering_event(
        ironsmith::triggers::TriggerEvent::new(event, ironsmith::ProvNodeId::default()),
    ));
    let mut choices = ScryChoices::default();
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut choices).unwrap();
    assert_eq!(
        choices.steps,
        [
            Offer(B),
            Offer(A),
            Look(B, 2),
            Look(A, 2),
            Place(B),
            Place(A)
        ]
    );
    assert!(choices.tops_at_placement.iter().all(|v| v == &tops));
}

#[test]
fn group_scry_respects_range_of_influence() {
    let mut g = GameState::new((0..5).map(|i| format!("P{i}")).collect(), 20);
    g.enable_limited_range_of_influence((0..5).map(PlayerId).collect(), vec![1; 5])
        .unwrap();
    let tops = fill_libraries(&mut g);
    let source = g.new_object_id();
    let mut choices = ScryChoices::default();
    EachPlayerScryEffect::new(1, PlayerFilter::Any)
        .execute(&mut g, &mut EffectContext::new(source, A, &mut choices))
        .unwrap();
    let e = PlayerId(4);
    assert_eq!(
        choices.steps,
        [
            Look(A, 1),
            Look(B, 1),
            Look(e, 1),
            Place(A),
            Place(B),
            Place(e)
        ]
    );
    assert!(choices.tops_at_placement.iter().all(|v| v == &tops));
    assert_eq!(g.player(C).unwrap().library.last(), Some(&tops[2]));
    assert_eq!(
        g.player(PlayerId(3)).unwrap().library.last(),
        Some(&tops[3])
    );
}

#[test]
fn the_temporal_anchor_exiles_exactly_the_cards_chosen_for_the_bottom() {
    for (group, bottom_count) in [(false, 1), (true, 0), (true, 1), (true, 2)] {
        let mut g = game();
        fill_libraries(&mut g);
        let anchor = put(&mut g, "the-temporal-anchor", A, Zone::Battlefield);
        let looked: Vec<_> = g
            .player(A)
            .unwrap()
            .library
            .iter()
            .rev()
            .take(2)
            .map(|id| g.object(*id).unwrap().stable_id)
            .collect();
        let mut choices = ScryChoices {
            bottom_count: Some(bottom_count),
            ..Default::default()
        };
        let effect = if group {
            ironsmith::Effect::new(EachPlayerScryEffect::new(2, PlayerFilter::Any))
        } else {
            ironsmith::Effect::scry(2)
        };
        g.push_to_stack(ironsmith::StackEntry::ability(anchor, A, vec![effect]));
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut choices).unwrap();
        let mut triggers = ironsmith::TriggerQueue::new();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut triggers).unwrap();
        assert_eq!(
            triggers
                .entries
                .iter()
                .filter(|t| t.source == anchor)
                .count(),
            usize::from(bottom_count > 0),
            "group={group}, bottom count={bottom_count}"
        );
        if bottom_count > 0 {
            ironsmith::game_loop::put_triggers_on_stack(&mut g, &mut triggers).unwrap();
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut choices).unwrap();
        }
        assert_eq!(g.exile.len(), bottom_count);
        assert!(
            g.exile
                .iter()
                .all(|id| looked.contains(&g.object(*id).unwrap().stable_id))
        );
    }
}

#[test]
fn pending_last_placement_rolls_back_all_scry_movements_and_private_knowledge() {
    let mut g = game();
    let tops = fill_libraries(&mut g);
    let source = g.new_object_id();
    let mut choices = ScryChoices {
        pause_at: Some(B),
        ..Default::default()
    };
    let effect = EachPlayerScryEffect::new(1, PlayerFilter::Any);
    let result = effect
        .execute(&mut g, &mut EffectContext::new(source, A, &mut choices))
        .unwrap();
    assert!(choices.paused);
    assert!(result.events.is_empty());
    for (i, player) in [A, B].into_iter().enumerate() {
        assert_eq!(g.player(player).unwrap().library.last(), Some(&tops[i]));
        assert!(g.known_library_positions(player, player).is_empty());
    }
    let mut replay = ScryChoices::default();
    effect
        .execute(&mut g, &mut EffectContext::new(source, A, &mut replay))
        .unwrap();
    assert_eq!(replay.steps, [Look(A, 1), Look(B, 1), Place(A), Place(B)]);
    for (i, player) in [A, B].into_iter().enumerate() {
        assert_eq!(g.player(player).unwrap().library.first(), Some(&tops[i]));
    }
}

#[test]
fn kaya_draws_before_offering_opponents_their_shared_scry() {
    let mut base = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    fill_libraries(&mut base);
    let source = put(&mut base, "kaya-intangible-slayer", A, Zone::Battlefield);
    let loyalty_zero = base
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_loyalty_ability => {
                Some(activated.effects.clone())
            }
            _ => None,
        })
        .nth(1)
        .unwrap();
    for (active, opponents) in [(A, [B, C]), (B, [B, C]), (C, [C, B])] {
        for decline in [None, Some(B), Some(C)] {
            let mut g = base.clone();
            g.turn.active_player = active;
            g.push_to_stack(ironsmith::StackEntry::ability(
                source,
                A,
                loyalty_zero.clone(),
            ));
            let mut choices = ScryChoices {
                declined: decline.into_iter().collect(),
                ..Default::default()
            };
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut choices).unwrap();
            let accepted: Vec<_> = opponents
                .into_iter()
                .filter(|p| Some(*p) != decline)
                .collect();
            assert_eq!(choices.steps, optional_steps(&opponents, &accepted, 1));
            assert!(
                choices
                    .alice_hand_at_placement
                    .iter()
                    .all(|count| *count == 2)
            );
            assert!(choices.tops_at_placement.windows(2).all(|w| w[0] == w[1]));
        }
    }
}

#[test]
fn repeated_partition_entries_count_a_bottomed_card_once() {
    struct DuplicateSelection;
    impl DecisionMaker for DuplicateSelection {
        fn decide_partition(&mut self, _: &GameState, ctx: &PartitionContext) -> Vec<ObjectId> {
            vec![ctx.cards[0].0, ctx.cards[0].0]
        }
    }
    let mut g = game();
    let tops = fill_libraries(&mut g);
    let anchor = put(&mut g, "the-temporal-anchor", A, Zone::Battlefield);
    let mut choices = DuplicateSelection;
    let selected = ironsmith::decisions::make_decision(
        &g,
        &mut choices,
        A,
        Some(anchor),
        ironsmith::decisions::ScrySpec::new(anchor, vec![tops[0]]),
    );
    assert_eq!(selected, [tops[0]]);

    g.push_to_stack(ironsmith::StackEntry::ability(
        anchor,
        A,
        vec![ironsmith::Effect::scry(1)],
    ));
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut choices).unwrap();
    assert_eq!(g.player(A).unwrap().library.first(), Some(&tops[0]));
    let mut triggers = ironsmith::TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut triggers).unwrap();
    ironsmith::game_loop::put_triggers_on_stack(&mut g, &mut triggers).unwrap();
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut choices).unwrap();
    assert_eq!(g.exile.len(), 1);
}

#[test]
fn player_orders_kenessos_and_eligeth_when_both_replace_their_scry() {
    struct ApplyFirst(ObjectId);
    impl DecisionMaker for ApplyFirst {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.player, A);
            vec![
                ctx.options
                    .iter()
                    .find(|option| option.object_id == Some(self.0))
                    .unwrap()
                    .index,
            ]
        }
    }
    let mut base = game();
    fill_libraries(&mut base);
    let eligeth = put(&mut base, "eligeth-crossroads-augur", A, Zone::Battlefield);
    let kenessos = put(&mut base, "kenessos-priest-of-thassa", A, Zone::Battlefield);
    for (first, draws) in [(kenessos, 2), (eligeth, 1)] {
        let mut g = base.clone();
        let mut dm = ApplyFirst(first);
        let outcome = EachPlayerScryEffect::new(1, PlayerFilter::Any)
            .execute(&mut g, &mut EffectContext::new(eligeth, A, &mut dm))
            .unwrap();
        assert_eq!(g.player(A).unwrap().hand.len(), draws);
        let scriers: Vec<_> = outcome
            .events
            .iter()
            .filter_map(|e| e.downcast::<KeywordActionEvent>())
            .filter(|e| e.action == KeywordActionKind::Scry)
            .map(|e| e.player)
            .collect();
        assert_eq!(scriers, [B]);
    }
}
