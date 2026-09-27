use super::*;
use crate::ability::Ability;
use crate::card::{CardBuilder, PowerToughness};
use crate::decision::DecisionMaker;
use crate::decisions::context::SelectOptionsContext;
use crate::events::life::matchers::{WouldGainLifeMatcher, WouldLoseLifeMatcher};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::object::CounterType;
use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;
use crate::zone::Zone;

struct ReplayOptions<'a> {
    answers: &'a [usize],
    cursor: usize,
    pending: Option<SelectOptionsContext>,
}

impl DecisionMaker for ReplayOptions<'_> {
    fn awaiting_choice(&self) -> bool {
        self.pending.is_some()
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        assert!(
            self.pending.is_none(),
            "stop after the first unanswered decision"
        );
        if let Some(answer) = self.answers.get(self.cursor).copied() {
            self.cursor += 1;
            assert!(
                ctx.options
                    .iter()
                    .any(|option| option.legal && option.index == answer)
            );
            vec![answer]
        } else {
            self.pending = Some(ctx.clone());
            // Like UI replay, return a placeholder while marking it unanswered.
            vec![
                ctx.options
                    .iter()
                    .find(|option| option.legal)
                    .unwrap()
                    .index,
            ]
        }
    }
}

/// Use the UI's checkpoint contract: incomplete effects are discarded and the
/// same action is replayed with all answers before publishing its state/events.
fn replay_damage(
    game: &mut GameState,
    source: ObjectId,
    amount: u32,
    targets: &[ResolvedTarget],
    answers: &[usize],
) -> Result<EffectOutcome, SelectOptionsContext> {
    let mut trial = game.clone();
    let mut dm = ReplayOptions {
        answers,
        cursor: 0,
        pending: None,
    };
    let target = if targets.len() == 1 {
        ChooseSpec::AnyTarget
    } else {
        ChooseSpec::AnyTarget.with_count(crate::effect::ChoiceCount::up_to(targets.len()))
    };
    let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut dm)
        .with_targets(targets.to_vec());
    if targets.len() > 1 {
        ctx = ctx.with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target.clone(),
            range: 0..targets.len(),
        }]);
    }
    let outcome = DealDamageEffect::new(amount as i32, target)
        .execute(&mut trial, &mut ctx)
        .expect("damage action must replay");
    assert_eq!(
        dm.cursor,
        answers.len(),
        "every recorded answer must be used"
    );
    if let Some(prompt) = dm.pending {
        assert!(
            outcome.events.is_empty(),
            "pending damage must not publish provisional trigger events"
        );
        assert_eq!(outcome.count_or_zero(), 0);
        Err(prompt)
    } else {
        *game = trial;
        Ok(outcome)
    }
}

fn permanent(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    abilities: Vec<StaticAbility>,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 10))
        .build();
    let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
    for ability in abilities {
        game.object_mut(id)
            .unwrap()
            .abilities_mut()
            .push(Ability::static_ability(ability));
    }
    id
}

fn choose_nonfirst(prompt: SelectOptionsContext, player: PlayerId, source: ObjectId) -> usize {
    assert_eq!(prompt.player, player);
    assert_eq!(
        prompt.options.iter().filter(|option| option.legal).count(),
        2
    );
    let choice = prompt
        .options
        .iter()
        .find(|option| option.object_id == Some(source))
        .unwrap()
        .index;
    assert_ne!(
        choice,
        prompt
            .options
            .iter()
            .find(|option| option.legal)
            .unwrap()
            .index
    );
    choice
}

fn add_lifelink_modifiers(game: &mut GameState, alice: PlayerId) -> ObjectId {
    let add = permanent(game, alice, "Add one life", vec![]);
    let double = permanent(game, alice, "Double life gain", vec![]);
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
    double
}

#[test]
fn single_damage_replays_life_loss_then_lifelink_replacement_choices() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = permanent(
        &mut game,
        alice,
        "Lifelink source",
        vec![StaticAbility::lifelink()],
    );
    let gain_double = add_lifelink_modifiers(&mut game, alice);
    let loss_first = permanent(&mut game, bob, "First life-loss doubler", vec![]);
    let loss_second = permanent(&mut game, bob, "Second life-loss doubler", vec![]);
    for id in [loss_first, loss_second] {
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                id,
                bob,
                WouldLoseLifeMatcher::you(),
                ReplacementAction::Double,
            ),
        );
    }
    let targets = [ResolvedTarget::Player(bob)];
    let first = choose_nonfirst(
        replay_damage(&mut game, source, 3, &targets, &[]).unwrap_err(),
        bob,
        loss_second,
    );
    assert_eq!(game.player(bob).unwrap().life, 20);
    let second = choose_nonfirst(
        replay_damage(&mut game, source, 3, &targets, &[first]).unwrap_err(),
        alice,
        gain_double,
    );
    assert_eq!(
        game.player(bob).unwrap().life,
        20,
        "life loss waits for the later lifelink choice too"
    );
    assert_eq!(game.player(alice).unwrap().life, 20);
    let outcome = replay_damage(&mut game, source, 3, &targets, &[first, second]).unwrap();
    assert_eq!(game.player(bob).unwrap().life, 8);
    assert_eq!(game.player(alice).unwrap().life, 27);
    assert_eq!(outcome.count_or_zero(), 3);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| event.downcast::<DamageEvent>().is_some())
            .count(),
        1
    );
    assert_eq!(
        outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<LifeGainEvent>())
            .unwrap()
            .amount,
        7
    );
}

#[test]
fn simultaneous_damage_replays_one_lifelink_gain_for_all_recipients() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = permanent(
        &mut game,
        alice,
        "Lifelink source",
        vec![StaticAbility::lifelink()],
    );
    let creature = permanent(&mut game, bob, "Damage recipient", vec![]);
    let double = add_lifelink_modifiers(&mut game, alice);
    let targets = [
        ResolvedTarget::Object(creature),
        ResolvedTarget::Player(bob),
    ];
    let choice = choose_nonfirst(
        replay_damage(&mut game, source, 3, &targets, &[]).unwrap_err(),
        alice,
        double,
    );
    assert_eq!(game.player(bob).unwrap().life, 20);
    assert_eq!(game.damage_on(creature), 0);
    let outcome = replay_damage(&mut game, source, 3, &targets, &[choice]).unwrap();
    assert_eq!(game.player(bob).unwrap().life, 17);
    assert_eq!(game.damage_on(creature), 3);
    assert_eq!(game.player(alice).unwrap().life, 33, "(3+3)*2+1 life");
    assert_eq!(outcome.count_or_zero(), 6);
    let gains = outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<LifeGainEvent>())
        .collect::<Vec<_>>();
    assert_eq!(gains.len(), 1);
    assert_eq!(gains[0].amount, 13);
}

#[test]
fn noncombat_infect_replays_poison_replacement_choice() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = permanent(
        &mut game,
        alice,
        "Infect source",
        vec![StaticAbility::infect()],
    );
    permanent(
        &mut game,
        bob,
        "Add poison",
        vec![StaticAbility::add_player_counters_placement_replacement(
            PlayerFilter::You,
            Some(CounterType::Poison),
            1,
            "Add poison".into(),
        )],
    );
    let double = permanent(
        &mut game,
        bob,
        "Double poison",
        vec![StaticAbility::double_player_counters_replacement(
            PlayerFilter::You,
            Some(CounterType::Poison),
            "Double poison".into(),
        )],
    );
    let targets = [ResolvedTarget::Player(bob)];
    let choice = choose_nonfirst(
        replay_damage(&mut game, source, 3, &targets, &[]).unwrap_err(),
        bob,
        double,
    );
    assert_eq!(game.player(bob).unwrap().poison_counters, 0);
    let outcome = replay_damage(&mut game, source, 3, &targets, &[choice]).unwrap();
    assert_eq!(game.player(bob).unwrap().poison_counters, 7);
    assert_eq!(game.player(bob).unwrap().life, 20);
    assert_eq!(outcome.count_or_zero(), 3);
}

#[test]
fn simultaneous_infect_and_wither_replay_each_creatures_counter_choice() {
    for infect in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = permanent(
            &mut game,
            alice,
            "Counter source",
            vec![if infect {
                StaticAbility::infect()
            } else {
                StaticAbility::wither()
            }],
        );
        let first = permanent(&mut game, bob, "First recipient", vec![]);
        let second = permanent(&mut game, bob, "Second recipient", vec![]);
        permanent(
            &mut game,
            bob,
            "Add minus counter",
            vec![StaticAbility::add_counters_placement_replacement(
                ObjectFilter::creature(),
                Some(CounterType::MinusOneMinusOne),
                1,
                "Add minus counter".into(),
            )],
        );
        let double = permanent(
            &mut game,
            bob,
            "Double minus counters",
            vec![StaticAbility::double_counters_replacement(
                ObjectFilter::creature(),
                Some(CounterType::MinusOneMinusOne),
                "Double minus counters".into(),
            )],
        );
        let targets = [
            ResolvedTarget::Object(first),
            ResolvedTarget::Object(second),
        ];
        let mut answers = Vec::new();
        for _ in 0..2 {
            let prompt = replay_damage(&mut game, source, 3, &targets, &answers).unwrap_err();
            answers.push(choose_nonfirst(prompt, bob, double));
            assert_eq!(game.counter_count(first, CounterType::MinusOneMinusOne), 0);
            assert_eq!(game.counter_count(second, CounterType::MinusOneMinusOne), 0);
        }
        let outcome = replay_damage(&mut game, source, 3, &targets, &answers).unwrap();
        for target in [first, second] {
            assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), 7);
            assert_eq!(game.damage_on(target), 0);
        }
        assert_eq!(outcome.count_or_zero(), 6);
    }
}
