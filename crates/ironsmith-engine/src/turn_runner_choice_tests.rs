use super::*;
use crate::ability::Ability;
use crate::card::{CardBuilder, PowerToughness};
use crate::combat_state::{AttackTarget, AttackerInfo};
use crate::events::damage::matchers::DamageFromSourceMatcher;
use crate::ids::CardId;
use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;
use crate::types::CardType;
use crate::zone::Zone;

fn create_permanent(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    creature: bool,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![if creature {
            CardType::Creature
        } else {
            CardType::Artifact
        }])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_card(&card, player, Zone::Battlefield)
}

#[test]
fn combat_replacement_choices_pause_and_commit_the_whole_damage_batch_once() {
    for first_strike in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = create_permanent(&mut game, alice, "First attacker", true);
        let second = create_permanent(&mut game, alice, "Second attacker", true);
        if first_strike {
            for id in [first, second] {
                game.object_mut(id)
                    .unwrap()
                    .abilities_mut()
                    .push(Ability::static_ability(StaticAbility::first_strike()));
            }
        }
        let add = create_permanent(&mut game, alice, "Add one", false);
        let double = create_permanent(&mut game, alice, "Double damage", false);
        for (source, modification) in [
            (add, EventModification::Add(1)),
            (double, EventModification::Multiply(2)),
        ] {
            game.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    DamageFromSourceMatcher::new(ObjectFilter::creature()),
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
                creature: first,
                target: AttackTarget::Player(bob),
            },
            AttackerInfo {
                creature: second,
                target: AttackTarget::Player(bob),
            },
        ];
        game.combat = Some(runner.combat.clone());
        let mut tq = TriggerQueue::new();

        // Choose opposite orders for the two events: 3*2+1 and (3+1)*2.
        for effect_source in [double, add] {
            let TurnAction::Decision(DecisionContext::SelectOptions(ctx)) =
                runner.advance(&mut game, &mut tq).unwrap()
            else {
                panic!("damage replacement order must reach the player");
            };
            assert_eq!(ctx.player, bob);
            assert_eq!(ctx.options.iter().filter(|option| option.legal).count(), 2);
            assert_eq!(
                game.player(bob).unwrap().life,
                20,
                "no partial damage while any answer is pending"
            );
            assert!(tq.is_empty());
            let choice = ctx
                .options
                .iter()
                .find(|option| option.object_id == Some(effect_source))
                .unwrap()
                .index;
            // Polling the runner again cannot consume a default answer.
            assert!(matches!(
                runner.advance(&mut game, &mut tq).unwrap(),
                TurnAction::Decision(DecisionContext::SelectOptions(_))
            ));
            assert_eq!(game.player(bob).unwrap().life, 20);
            runner.respond_options(vec![choice]);
        }
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::Continue
        ));
        assert_eq!(game.player(bob).unwrap().life, 5);
        assert!(!runner.has_pending_replay_choice());
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(
            game.player(bob).unwrap().life,
            5,
            "the batch must not execute twice"
        );
    }
}

#[test]
fn combat_prevention_allocation_waits_for_the_defenders_number_choice() {
    use crate::prevention::{PreventionShield, PreventionTarget};
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let first = create_permanent(&mut game, alice, "Lifelink attacker", true);
    let second = create_permanent(&mut game, alice, "Second attacker", true);
    game.object_mut(first)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::lifelink()));
    let shield_source = create_permanent(&mut game, bob, "Two point shield", false);
    game.effect_store
        .prevention_effects
        .add_shield(PreventionShield::prevent_next_n(
            shield_source,
            bob,
            PreventionTarget::Player(bob),
            2,
        ));
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::CombatDamage);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::CombatDamageRegularAssign);
    runner.combat.attackers = vec![
        AttackerInfo {
            creature: first,
            target: AttackTarget::Player(bob),
        },
        AttackerInfo {
            creature: second,
            target: AttackTarget::Player(bob),
        },
    ];
    game.combat = Some(runner.combat.clone());
    let mut tq = TriggerQueue::new();
    for _ in 0..2 {
        let TurnAction::Decision(DecisionContext::Number(ctx)) =
            runner.advance(&mut game, &mut tq).unwrap()
        else {
            panic!("the defending player must allocate their limited prevention shield");
        };
        assert_eq!(ctx.player, bob);
        assert_eq!(ctx.min, 0);
        assert_eq!(ctx.max, 2);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(
            game.effect_store.prevention_effects.shields()[0].amount_remaining,
            Some(2)
        );
    }
    // Spend the shield on the lifelink creature, rather than the default 0.
    runner.respond_number(2);
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::Continue
    ));
    assert_eq!(game.player(bob).unwrap().life, 16);
    assert_eq!(game.player(alice).unwrap().life, 21);
    assert!(game.effect_store.prevention_effects.shields().is_empty());
}
