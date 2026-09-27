use super::*;
use crate::ability::Ability;
use crate::card::{CardBuilder, PowerToughness};
use crate::color::Color;
use crate::combat_state::{AttackTarget, AttackerInfo};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::{ChoiceCount, Effect};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

fn permanent(game: &mut GameState, player: PlayerId, name: &str, creature: bool) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![if creature {
            CardType::Creature
        } else {
            CardType::Artifact
        }])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let id = game.create_object_from_card(&card, player, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn lotus(game: &mut GameState, player: PlayerId) -> ObjectId {
    let id = permanent(game, player, "Gilded Lotus", false);
    game.object_mut(id)
        .unwrap()
        .abilities_mut()
        .push(Ability::activated(
            TotalCost::from_cost(Cost::tap()),
            vec![Effect::add_mana_of_any_one_color(3)],
        ));
    id
}

fn choose_source(runner: &mut TurnRunner, action: TurnAction, source: ObjectId) {
    let TurnAction::Decision(DecisionContext::SelectOptions(context)) = action else {
        panic!("expected mana window, got {action:?}");
    };
    let index = context
        .options
        .iter()
        .find(|option| option.object_id == Some(source))
        .unwrap()
        .index;
    runner.respond_options(vec![index]);
}

fn finish_fixed_payment(runner: &mut TurnRunner, game: &mut GameState, tq: &mut TriggerQueue) {
    for _ in 0..8 {
        match runner.advance(game, tq).unwrap() {
            TurnAction::RunPriority => return,
            TurnAction::Decision(DecisionContext::SelectOptions(context)) => {
                let option = context
                    .options
                    .iter()
                    .find(|option| {
                        option.description == "Finish activating mana abilities"
                            || option.description == "Confirm payment"
                    })
                    .unwrap_or_else(|| panic!("unexpected payment prompt: {context:?}"));
                runner.respond_options(vec![option.index]);
            }
            other => panic!("unexpected payment progress: {other:?}"),
        }
    }
    panic!("payment did not finish");
}

#[test]
fn attack_tax_mana_color_waits_and_uses_selected_blue() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let attacker = permanent(&mut game, alice, "Attacker", true);
    let lotus = lotus(&mut game, alice);
    let prison = permanent(&mut game, bob, "Ghostly Prison", false);
    game.object_mut(prison)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::attack_cost(
            ObjectFilter::creature(),
            false,
            TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]])),
            "Pay {2} to attack",
        )));
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareAttackersDecision);
    let mut tq = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::Decision(DecisionContext::Attackers(_))
    ));
    runner.respond_attackers(vec![AttackerDeclaration {
        creature: attacker,
        target: AttackTarget::Player(bob),
    }]);
    let window = runner.advance(&mut game, &mut tq).unwrap();
    choose_source(&mut runner, window, lotus);

    for _ in 0..2 {
        let prompt = runner.advance(&mut game, &mut tq).unwrap();
        assert!(
            matches!(prompt, TurnAction::Decision(DecisionContext::Colors(_))),
            "{prompt:?}"
        );
        assert!(
            !game.is_tapped(lotus),
            "unanswered activation must not tap its source"
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }
    runner.respond_colors(vec![Color::Blue; 3]);
    finish_fixed_payment(&mut runner, &mut game, &mut tq);
    assert!(game.is_tapped(lotus));
    assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
    assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
}

#[test]
fn block_tax_mana_color_waits_and_uses_selected_red() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let attacker = permanent(&mut game, alice, "Attacker", true);
    let blocker = permanent(&mut game, bob, "Blocker", true);
    let lotus = lotus(&mut game, bob);
    game.object_mut(attacker)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::block_cost(
            ObjectFilter::creature(),
            ObjectFilter::source(),
            TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]])),
            "Pay {2} to block",
        )));
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareBlockersCheck);
    runner.combat_mut().attackers.push(AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(bob),
    });
    let mut tq = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::Decision(DecisionContext::Blockers(_))
    ));
    runner.respond_blockers(
        vec![BlockerDeclaration {
            blocker,
            blocking: attacker,
        }],
        bob,
    );
    let window = runner.advance(&mut game, &mut tq).unwrap();
    choose_source(&mut runner, window, lotus);

    for _ in 0..2 {
        let prompt = runner.advance(&mut game, &mut tq).unwrap();
        assert!(
            matches!(prompt, TurnAction::Decision(DecisionContext::Colors(_))),
            "{prompt:?}"
        );
        assert!(!game.is_tapped(lotus));
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
    }
    runner.respond_colors(vec![Color::Red; 3]);
    finish_fixed_payment(&mut runner, &mut game, &mut tq);
    assert!(game.is_tapped(lotus));
    assert_eq!(game.player(bob).unwrap().mana_pool.red, 1);
    assert_eq!(game.player(bob).unwrap().mana_pool.green, 0);
    assert_eq!(
        runner.combat().blockers.get(&attacker),
        Some(&vec![blocker])
    );
}

#[test]
fn blocking_cost_keeps_payment_tentative_until_second_helper_is_chosen() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let attacker = permanent(&mut game, alice, "Attacker", true);
    let blocker = permanent(&mut game, bob, "Hollow Warrior", true);
    let first = permanent(&mut game, bob, "First helper", true);
    let second = permanent(&mut game, bob, "Second helper", true);
    let mut eligible = ObjectFilter::creature().you_control();
    eligible.untapped = true;
    eligible.nonattacking = true;
    eligible.nonblocking = true;
    let tag = crate::tag::TagKey::from("hollow_warrior_tap_cost");
    let cost = TotalCost::from_costs(vec![
        Cost::try_effect(Effect::choose_objects(
            eligible,
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            tag.clone(),
        ))
        .unwrap(),
        Cost::try_effect(Effect::tap(ChooseSpec::tagged(tag))).unwrap(),
    ]);
    game.object_mut(blocker)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::block_cost(
            ObjectFilter::source(),
            ObjectFilter::creature(),
            cost,
            "This creature can't block unless you tap an eligible creature",
        )));
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareBlockersCheck);
    runner.combat_mut().attackers.push(AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(bob),
    });
    let mut tq = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::Decision(DecisionContext::Blockers(_))
    ));
    runner.respond_blockers(
        vec![BlockerDeclaration {
            blocker,
            blocking: attacker,
        }],
        bob,
    );
    let order = runner.advance(&mut game, &mut tq).unwrap();
    assert!(
        matches!(&order, TurnAction::Decision(DecisionContext::SelectOptions(context))
        if context.description.contains("order") && context.options.len() == 2),
        "{order:?}"
    );
    assert!(!game.is_tapped(first) && !game.is_tapped(second));
    runner.respond_options(vec![0, 1]);
    for _ in 0..2 {
        let prompt = runner.advance(&mut game, &mut tq).unwrap();
        let TurnAction::Decision(DecisionContext::SelectObjects(context)) = prompt else {
            panic!("expected helper choice, got {prompt:?}");
        };
        assert_eq!(
            context
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>(),
            vec![first, second]
        );
        assert!(!game.is_tapped(first) && !game.is_tapped(second));
    }
    runner.respond_discard(vec![second]);
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::RunPriority
    ));
    assert!(!game.is_tapped(first));
    assert!(game.is_tapped(second));
    assert!(!game.is_tapped(blocker));
}
