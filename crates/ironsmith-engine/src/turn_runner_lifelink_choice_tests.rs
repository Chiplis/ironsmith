use super::*;
use crate::ability::Ability;
use crate::card::{CardBuilder, PowerToughness};
use crate::combat_state::{AttackTarget, AttackerInfo};
use crate::events::life::matchers::WouldGainLifeMatcher;
use crate::ids::CardId;
use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use crate::static_abilities::StaticAbility;
use crate::types::CardType;
use crate::zone::Zone;

fn permanent(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    stats: Option<(i32, i32)>,
) -> ObjectId {
    let mut card = CardBuilder::new(CardId::new(), name);
    if let Some((power, toughness)) = stats {
        card = card
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness));
    } else {
        card = card.card_types(vec![CardType::Enchantment]);
    }
    game.create_object_from_card(&card.build(), player, Zone::Battlefield)
}

#[test]
fn combat_lifelink_replacement_order_keeps_the_whole_damage_step_pending() {
    for first_strike in [false, true] {
        for blocked in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let lifelinker = permanent(&mut game, alice, "Lifelink attacker", Some((3, 3)));
            let other = permanent(&mut game, alice, "Other attacker", Some((3, 3)));
            let blocker = permanent(&mut game, bob, "Blocker", Some((0, 6)));
            game.object_mut(lifelinker)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::lifelink()));
            if first_strike {
                for id in [lifelinker, other] {
                    game.object_mut(id)
                        .unwrap()
                        .abilities_mut()
                        .push(Ability::static_ability(StaticAbility::first_strike()));
                }
            }
            // The generic capabilities behind Cleric Class and Boon Reflection.
            let add = permanent(&mut game, alice, "Gain one additional life", None);
            let double = permanent(&mut game, alice, "Double life gain", None);
            for (source, modification) in [
                (add, EventModification::Add(1)),
                (double, EventModification::Multiply(2)),
            ] {
                game.effect_store.replacement_effects.add_resolution_effect(
                    ReplacementEffect::with_matcher(
                        source,
                        alice,
                        WouldGainLifeMatcher::you(),
                        ReplacementAction::Modify(modification),
                    ),
                );
            }
            game.turn.active_player = alice;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::CombatDamage);
            let mut runner = TurnRunner::from_state_for_sync(if first_strike {
                TurnState::CombatDamageFirstStrikeAssign
            } else {
                TurnState::CombatDamageRegularAssign
            });
            runner.combat.attackers = vec![
                AttackerInfo {
                    creature: lifelinker,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: other,
                    target: AttackTarget::Player(bob),
                },
            ];
            if blocked {
                runner.combat.blockers.insert(lifelinker, vec![blocker]);
            }
            game.combat = Some(runner.combat.clone());
            let mut tq = TriggerQueue::new();

            let mut answer = None;
            for _ in 0..2 {
                let TurnAction::Decision(DecisionContext::SelectOptions(ctx)) =
                    runner.advance(&mut game, &mut tq).unwrap()
                else {
                    panic!("lifelink replacement order must reach its controller");
                };
                assert_eq!(ctx.player, alice);
                assert_eq!(ctx.options.iter().filter(|option| option.legal).count(), 2);
                let choice = ctx
                    .options
                    .iter()
                    .find(|option| option.object_id == Some(double))
                    .unwrap()
                    .index;
                assert_ne!(
                    choice,
                    ctx.options
                        .iter()
                        .find(|option| option.legal)
                        .unwrap()
                        .index
                );
                assert_eq!(answer.get_or_insert(choice), &choice);
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert_eq!(game.damage_on(blocker), 0);
                assert!(
                    tq.is_empty(),
                    "no trigger can escape the incomplete damage batch"
                );
            }
            runner.respond_options(vec![answer.unwrap()]);
            assert!(matches!(
                runner.advance(&mut game, &mut tq).unwrap(),
                TurnAction::Continue
            ));
            assert_eq!(
                game.player(alice).unwrap().life,
                27,
                "choose 3*2+1, not (3+1)*2"
            );
            assert_eq!(
                game.player(bob).unwrap().life,
                if blocked { 17 } else { 14 }
            );
            assert_eq!(game.damage_on(blocker), if blocked { 3 } else { 0 });
            assert!(!runner.has_pending_replay_choice());
            assert!(matches!(
                runner.advance(&mut game, &mut tq).unwrap(),
                TurnAction::RunPriority
            ));
            assert_eq!(
                game.player(alice).unwrap().life,
                27,
                "resuming must not repeat lifelink"
            );
        }
    }
}

fn regular_combat_runner(game: &mut GameState, attackers: &[ObjectId]) -> TurnRunner {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::CombatDamage);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::CombatDamageRegularAssign);
    runner.combat.attackers = attackers
        .iter()
        .map(|creature| AttackerInfo {
            creature: *creature,
            target: AttackTarget::Player(bob),
        })
        .collect();
    game.combat = Some(runner.combat.clone());
    runner
}

fn pending_nonfirst_replacement(
    runner: &mut TurnRunner,
    game: &mut GameState,
    tq: &mut TriggerQueue,
    chooser: PlayerId,
    desired_source: ObjectId,
) -> usize {
    let TurnAction::Decision(DecisionContext::SelectOptions(ctx)) =
        runner.advance(game, tq).unwrap()
    else {
        panic!("combat damage consequences must surface their replacement choice");
    };
    assert_eq!(ctx.player, chooser);
    assert_eq!(ctx.options.iter().filter(|option| option.legal).count(), 2);
    let choice = ctx
        .options
        .iter()
        .find(|option| option.object_id == Some(desired_source))
        .unwrap()
        .index;
    assert_ne!(
        choice,
        ctx.options
            .iter()
            .find(|option| option.legal)
            .unwrap()
            .index
    );
    assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 20);
    assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 20);
    assert!(
        tq.is_empty(),
        "the incomplete batch cannot queue damage triggers"
    );
    choice
}

#[test]
fn combat_life_loss_replacement_order_replays_every_assignment() {
    use crate::events::life::matchers::WouldLoseLifeMatcher;
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let first = permanent(&mut game, alice, "First attacker", Some((1, 3)));
    let second = permanent(&mut game, alice, "Second attacker", Some((1, 3)));
    let first_double = permanent(&mut game, bob, "First life-loss doubler", None);
    let double = permanent(&mut game, bob, "Second life-loss doubler", None);
    for source in [first_double, double] {
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                bob,
                WouldLoseLifeMatcher::you(),
                ReplacementAction::Double,
            ),
        );
    }
    let mut runner = regular_combat_runner(&mut game, &[first, second]);
    let mut tq = TriggerQueue::new();
    for _ in 0..2 {
        let choice = pending_nonfirst_replacement(&mut runner, &mut game, &mut tq, bob, double);
        assert_eq!(
            pending_nonfirst_replacement(&mut runner, &mut game, &mut tq, bob, double),
            choice
        );
        runner.respond_options(vec![choice]);
    }
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::Continue
    ));
    assert_eq!(
        game.player(bob).unwrap().life,
        12,
        "both doublers apply to each one-point damage assignment"
    );
    assert!(!runner.has_pending_replay_choice());
}

#[test]
fn combat_infect_and_toxic_poison_replacements_keep_earlier_damage_pending() {
    use crate::object::CounterType;
    use crate::target::PlayerFilter;
    for infect in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let normal = permanent(&mut game, alice, "Normal attacker", Some((3, 3)));
        let poison = permanent(&mut game, alice, "Poison attacker", Some((3, 3)));
        game.object_mut(poison)
            .unwrap()
            .abilities_mut()
            .push(Ability::static_ability(if infect {
                StaticAbility::infect()
            } else {
                StaticAbility::toxic(2)
            }));
        let add = permanent(&mut game, bob, "One additional poison counter", None);
        let double = permanent(&mut game, bob, "Double poison counters", None);
        for (source, ability) in [
            (
                add,
                StaticAbility::add_player_counters_placement_replacement(
                    PlayerFilter::You,
                    Some(CounterType::Poison),
                    1,
                    "One additional poison counter".into(),
                ),
            ),
            (
                double,
                StaticAbility::double_player_counters_replacement(
                    PlayerFilter::You,
                    Some(CounterType::Poison),
                    "Double poison counters".into(),
                ),
            ),
        ] {
            game.object_mut(source)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(ability));
        }
        let mut runner = regular_combat_runner(&mut game, &[normal, poison]);
        let mut tq = TriggerQueue::new();
        let choice = pending_nonfirst_replacement(&mut runner, &mut game, &mut tq, bob, double);
        assert_eq!(game.player(bob).unwrap().poison_counters, 0);
        assert_eq!(
            pending_nonfirst_replacement(&mut runner, &mut game, &mut tq, bob, double),
            choice
        );
        runner.respond_options(vec![choice]);
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::Continue
        ));
        assert_eq!(
            game.player(bob).unwrap().poison_counters,
            if infect { 7 } else { 5 }
        );
        assert_eq!(game.player(bob).unwrap().life, if infect { 17 } else { 14 });
        assert!(!runner.has_pending_replay_choice());
    }
}

#[test]
fn combat_infect_and_wither_counter_replacements_keep_earlier_damage_pending() {
    use crate::object::CounterType;
    use crate::target::ObjectFilter;
    for infect in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let normal = permanent(&mut game, alice, "Normal attacker", Some((3, 3)));
        let witherer = permanent(&mut game, alice, "Counter attacker", Some((3, 3)));
        let blocker = permanent(&mut game, bob, "Blocker", Some((0, 10)));
        game.object_mut(witherer)
            .unwrap()
            .abilities_mut()
            .push(Ability::static_ability(if infect {
                StaticAbility::infect()
            } else {
                StaticAbility::wither()
            }));
        let add = permanent(&mut game, bob, "One additional minus counter", None);
        let double = permanent(&mut game, bob, "Double minus counters", None);
        for (source, ability) in [
            (
                add,
                StaticAbility::add_counters_placement_replacement(
                    ObjectFilter::creature(),
                    Some(CounterType::MinusOneMinusOne),
                    1,
                    "One additional minus counter".into(),
                ),
            ),
            (
                double,
                StaticAbility::double_counters_replacement(
                    ObjectFilter::creature(),
                    Some(CounterType::MinusOneMinusOne),
                    "Double minus counters".into(),
                ),
            ),
        ] {
            game.object_mut(source)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(ability));
        }
        let mut runner = regular_combat_runner(&mut game, &[normal, witherer]);
        runner.combat.blockers.insert(witherer, vec![blocker]);
        game.combat = Some(runner.combat.clone());
        let mut tq = TriggerQueue::new();
        let choice = pending_nonfirst_replacement(&mut runner, &mut game, &mut tq, bob, double);
        assert_eq!(
            game.counter_count(blocker, CounterType::MinusOneMinusOne),
            0
        );
        assert_eq!(game.damage_on(blocker), 0);
        assert_eq!(
            pending_nonfirst_replacement(&mut runner, &mut game, &mut tq, bob, double),
            choice
        );
        runner.respond_options(vec![choice]);
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::Continue
        ));
        assert_eq!(
            game.counter_count(blocker, CounterType::MinusOneMinusOne),
            7
        );
        assert_eq!(
            game.damage_on(blocker),
            0,
            "infect and wither use counters instead of marking damage"
        );
        assert_eq!(game.player(bob).unwrap().life, 17);
        assert!(!runner.has_pending_replay_choice());
    }
}
