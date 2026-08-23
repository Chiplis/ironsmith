#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::effect::{Effect, EffectPredicate};
use crate::effects::{
    ExecutionContext, FlipCoinEffect, RepeatProcessEffect, SequenceEffect, UnlessPaysEffect,
};

const CROOKED_SCALES_ORACLE: &str = "{4}, {T}: Flip a coin. If you win the flip, destroy target creature an opponent controls. If you lose the flip, destroy target creature you control unless you pay {3} and repeat this process.";

fn activated_ability(
    definition: &crate::cards::CardDefinition,
) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Crooked Scales should have an activated ability")
}

fn repeat_process(activated: &crate::ability::ActivatedAbility) -> &RepeatProcessEffect {
    activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<RepeatProcessEffect>())
        .expect("the coin-flip procedure should remain one typed repeat process")
}

fn loss_branch(
    repeat: &RepeatProcessEffect,
) -> (&WithIdEffect, &IfEffect, &SequenceEffect, &UnlessPaysEffect) {
    let branches = repeat
        .effects
        .iter()
        .filter(|effect| effect.downcast_ref::<TargetOnlyEffect>().is_none())
        .collect::<Vec<_>>();
    let [flip, win, loss] = branches.as_slice() else {
        panic!("the repeat body should contain the flip and its two result branches: {repeat:#?}");
    };
    let flip = flip
        .downcast_ref::<WithIdEffect>()
        .expect("the coin flip should publish its result");
    assert!(
        flip.effect.downcast_ref::<FlipCoinEffect>().is_some(),
        "the first repeat effect should be the called coin flip: {flip:#?}"
    );
    let win = win
        .downcast_ref::<WithIdEffect>()
        .map_or(*win, |with_id| with_id.effect.as_ref())
        .downcast_ref::<IfEffect>()
        .expect("the second repeat effect should be the win branch");
    assert_eq!(win.condition, flip.id);
    assert_eq!(win.predicate, EffectPredicate::Happened);

    let loss_result = loss
        .downcast_ref::<WithIdEffect>()
        .expect("the complete loss branch should publish the continuation result");
    let loss = loss_result
        .effect
        .downcast_ref::<IfEffect>()
        .expect("the repeat condition should wrap the loss branch");
    let [coordinated] = loss.then.as_slice() else {
        panic!("the loss branch should contain one coordinated payment consequence");
    };
    let coordinated = coordinated
        .downcast_ref::<SequenceEffect>()
        .expect("the payment-and-repeat alternative should retain its coordinated surface");
    let [unless] = coordinated.effects.as_slice() else {
        panic!("the coordinated loss body should contain one unless-payment effect");
    };
    let unless = unless
        .downcast_ref::<UnlessPaysEffect>()
        .expect("the losing creature is destroyed unless its controller pays");
    (loss_result, loss, coordinated, unless)
}

#[test]
fn crooked_scales_tracks_the_unpaid_loss_branch_and_round_trips_exactly() {
    let definition = parse_oracle_card_definition("Crooked Scales");
    let activated = activated_ability(&definition);
    let repeat = repeat_process(activated);
    let (loss_result, loss, coordinated, unless) = loss_branch(repeat);

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        CROOKED_SCALES_ORACLE
    );
    assert_eq!(repeat.condition, loss_result.id);
    let flip_result_id = repeat
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<WithIdEffect>())
        .filter(|with_id| with_id.effect.downcast_ref::<FlipCoinEffect>().is_some())
        .map(|with_id| with_id.id)
        .expect("the coin flip should have a distinct result ID");
    assert_ne!(
        repeat.condition, flip_result_id,
        "the continuation result must not overwrite the coin result used by both branches"
    );
    assert!(
        repeat
            .effects
            .iter()
            .all(|effect| effect.downcast_ref::<TargetOnlyEffect>().is_none()),
        "target declarations must resolve once outside the repeated process"
    );
    assert_eq!(repeat.predicate, EffectPredicate::WasDeclined);
    assert_eq!(loss.predicate, EffectPredicate::DidNotHappen);
    assert!(loss.else_.is_empty());
    assert_eq!(
        coordinated.surface,
        ironsmith_core::SequenceSurface::Coordinated
    );
    assert_eq!(unless.player, PlayerFilter::You);
    assert_eq!(unless.cost.display(), "{3}");

    let [opponent_target, own_target] = activated.choices.as_slice() else {
        panic!("the activated ability should declare its two distinct creature targets");
    };
    let ChooseSpec::Object(opponent_filter) = opponent_target.base() else {
        panic!("the first target should be an opponent-controlled creature");
    };
    let ChooseSpec::Object(own_filter) = own_target.base() else {
        panic!("the second target should be a creature you control");
    };
    assert_eq!(opponent_filter.card_types, [CardType::Creature]);
    assert_eq!(opponent_filter.controller, Some(PlayerFilter::Opponent));
    assert_eq!(own_filter.card_types, [CardType::Creature]);
    assert_eq!(own_filter.controller, Some(PlayerFilter::You));

    let [destroy] = unless.effects.as_slice() else {
        panic!("declining payment should perform exactly the destroy consequence");
    };
    let destroy_effect = destroy
        .downcast_ref::<TaggedEffect>()
        .map_or(destroy, |tagged| tagged.effect.as_ref());
    let destroy = destroy_effect
        .downcast_ref::<DestroyEffect>()
        .expect("the unpaid consequence should destroy the targeted creature");
    let ChooseSpec::Target(target) = &destroy.spec else {
        panic!("the unless-payment consequence should retain its target: {destroy:#?}");
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        panic!("the target should be a creature you control: {target:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
}

#[derive(Debug)]
struct PaymentDecisions {
    payments: Vec<bool>,
    payment_index: usize,
    coin_calls: usize,
}

impl DecisionMaker for PaymentDecisions {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert!(
            ctx.description.contains("{3}"),
            "the boolean decision should be the Crooked Scales payment: {ctx:#?}"
        );
        let choice = self
            .payments
            .get(self.payment_index)
            .copied()
            .expect("the process asked for more payment decisions than expected");
        self.payment_index += 1;
        choice
    }

    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let is_coin_call = ctx.options.len() == 2
            && ctx
                .options
                .iter()
                .any(|option| option.description == "Heads")
            && ctx
                .options
                .iter()
                .any(|option| option.description == "Tails");
        if is_coin_call {
            self.coin_calls += 1;
            return vec![0];
        }

        let mut selected = Vec::new();
        let mut points = 0usize;
        for option in ctx.options.iter().filter(|option| option.legal) {
            let cost = option.point_cost.max(1) as usize;
            if points.saturating_add(cost) > ctx.max {
                continue;
            }
            selected.push(option.index);
            points += cost;
            if points >= ctx.min {
                break;
            }
        }
        selected
    }
}

fn creature(raw_id: u32, name: &str) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

fn forced_loss_process(repeat: &RepeatProcessEffect) -> Effect {
    let mut forced = repeat.clone();
    let flip_index = forced
        .effects
        .iter()
        .position(|effect| {
            effect
                .downcast_ref::<WithIdEffect>()
                .is_some_and(|with_id| with_id.effect.downcast_ref::<FlipCoinEffect>().is_some())
        })
        .expect("the repeat process should contain a result-tracked flip");
    let flip_result = forced.effects[flip_index]
        .downcast_ref::<WithIdEffect>()
        .expect("the repeat process should begin with a result-tracked flip");
    let flip = flip_result
        .effect
        .downcast_ref::<FlipCoinEffect>()
        .expect("the tracked setup should be a coin flip")
        .clone()
        .with_forced_loser(PlayerFilter::You);
    let flip_id = flip_result.id;
    forced.effects[flip_index] = Effect::with_id(flip_id.0, Effect::new(flip));
    Effect::new(forced)
}

fn run_forced_loss_case(payments: Vec<bool>) -> (usize, usize, Zone, u32) {
    let definition = parse_oracle_card_definition("Crooked Scales");
    let activated = activated_ability(&definition);
    let process = forced_loss_process(repeat_process(activated));
    let mut replaced_repeat = false;
    let effects = activated
        .effects
        .flattened_default_effects()
        .iter()
        .map(|effect| {
            if effect.downcast_ref::<RepeatProcessEffect>().is_some() {
                replaced_repeat = true;
                process.clone()
            } else {
                effect.clone()
            }
        })
        .collect();
    assert!(
        replaced_repeat,
        "the executable program should contain the typed repeat process"
    );
    let program = crate::resolution::ResolutionProgram::from_effects(effects);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let opponent_creature = game.create_object_from_definition(
        &creature(99_101, "Opponent Creature"),
        bob,
        Zone::Battlefield,
    );
    let own_creature = game.create_object_from_definition(
        &creature(99_102, "Own Creature"),
        alice,
        Zone::Battlefield,
    );
    let own_stable = game
        .object(own_creature)
        .expect("own creature should exist")
        .stable_id;
    let targets = vec![
        crate::effects::ResolvedTarget::Object(opponent_creature),
        crate::effects::ResolvedTarget::Object(own_creature),
    ];
    let assignments: Vec<crate::game_state::TargetAssignment> = activated
        .choices
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, spec)| crate::game_state::TargetAssignment {
            spec,
            range: index..index + 1,
        })
        .collect();
    let mut decisions = PaymentDecisions {
        payments,
        payment_index: 0,
        coin_calls: 0,
    };
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions)
        .with_targets(targets)
        .with_target_assignments(assignments.clone());
    ctx.snapshot_targets(&game);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &program,
        None,
        &assignments,
    )
    .expect("the forced losing coin-flip process should resolve");
    drop(ctx);

    let own_current = game
        .find_object_by_stable_id(own_stable)
        .expect("the own creature should retain stable identity");
    (
        decisions.coin_calls,
        decisions.payment_index,
        game.object(own_current)
            .expect("the own creature should remain in the game")
            .zone,
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .total(),
    )
}

#[test]
fn crooked_scales_payment_prevents_destroying_and_repeats() {
    let (coin_calls, payment_calls, own_zone, mana_left) = run_forced_loss_case(vec![true]);
    assert_eq!(
        coin_calls, 2,
        "paying after the first loss should repeat the complete coin-flip process"
    );
    assert_eq!(payment_calls, 1);
    assert_eq!(
        own_zone,
        Zone::Graveyard,
        "the repeated loss should destroy the creature once no second payment is affordable"
    );
    assert_eq!(mana_left, 0);
}

#[test]
fn crooked_scales_declined_payment_destroys_and_stops() {
    let (coin_calls, payment_calls, own_zone, mana_left) = run_forced_loss_case(vec![false]);
    assert_eq!(
        coin_calls, 1,
        "declining the payment must stop after the first coin flip"
    );
    assert_eq!(payment_calls, 1);
    assert_eq!(own_zone, Zone::Graveyard);
    assert_eq!(mana_left, 3);
}
