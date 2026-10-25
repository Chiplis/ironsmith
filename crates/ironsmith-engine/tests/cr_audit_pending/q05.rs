//! CR audit (in-review completion pass, package q05): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{
    BooleanContext, DistributeContext, NumberContext, SelectObjectsContext, SelectOptionsContext,
    TargetsContext, ViewCardsContext,
};
use ironsmith::effects::{
    ApplyContinuousEffect, DealDamageEffect, ExileEffect, MoveToZoneEffect, PhaseOutEffect,
};
use ironsmith::game_state::{Phase, Target};
use ironsmith::triggers::{Trigger, TriggerQueue};
use ironsmith::{
    Ability, CardType, ChooseSpec, DecisionMaker, Effect, EffectExecutor, ManaSymbol, ObjectId,
    Until, Zone,
};

// ------------------------------------------------------------------ helpers

fn compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture card {name} must compile: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture card {name} must be fully implemented"
    );
    def
}

fn two_player_main(active: PlayerId) -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}

#[derive(Default)]
struct Dm {
    accept: bool,
    target_pref: Vec<Target>,
    max_targets: bool,
    distribution: Option<Vec<u32>>,
    distribute_min_seen: Vec<u32>,
    public_views: Vec<(PlayerId, Vec<ObjectId>)>,
}

impl DecisionMaker for Dm {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.accept && ctx.can_accept
    }
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        ctx.min
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .take(ctx.min)
            .map(|c| c.id)
            .collect()
    }
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        ctx.options
            .iter()
            .filter(|o| o.legal)
            .take(ctx.min)
            .map(|o| o.index)
            .collect()
    }
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut out = Vec::new();
        for req in &ctx.requirements {
            let want = if self.max_targets {
                req.max_targets.unwrap_or(req.legal_targets.len()).max(1)
            } else {
                req.min_targets.max(1)
            };
            let mut picked: Vec<Target> = self
                .target_pref
                .iter()
                .filter(|t| req.legal_targets.contains(t))
                .cloned()
                .take(want)
                .collect();
            for t in &req.legal_targets {
                if picked.len() >= want {
                    break;
                }
                if !picked.contains(t) {
                    picked.push(*t);
                }
            }
            out.extend(picked);
        }
        out
    }
    fn decide_distribute(
        &mut self,
        _game: &GameState,
        ctx: &DistributeContext,
    ) -> Vec<(Target, u32)> {
        self.distribute_min_seen.push(ctx.min_per_target);
        let split = self.distribution.clone().unwrap_or_default();
        ctx.targets
            .iter()
            .zip(split)
            .map(|(t, n)| (t.target, n))
            .collect()
    }
    fn view_cards(
        &mut self,
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &ViewCardsContext,
    ) {
        if ctx.public {
            self.public_views.push((viewer, cards.to_vec()));
        }
    }
}

fn act_with<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: LegalAction,
    dm: &mut D,
    queue: &mut TriggerQueue,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    game.turn.priority_player = Some(player);
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..24 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        if let Some(pending) = &state.pending_method_selection {
            let index = pending
                .available_methods
                .iter()
                .position(|choice| Some(&choice.method) == requested_method.as_ref())
                .ok_or_else(|| "requested casting method lost".to_string())?;
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                dm,
            );
        } else {
            let context = match &result {
                Ok(GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending native action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    if state.pending_cast.is_some()
        || state.pending_method_selection.is_some()
        || state.pending_activation.is_some()
    {
        return Err("native action exceeded decision bound".into());
    }
    Ok(())
}

fn cast_actions(game: &GameState, player: PlayerId, spell: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .collect()
}

fn cast_action(game: &GameState, player: PlayerId, spell: ObjectId) -> Option<LegalAction> {
    cast_actions(game, player, spell).into_iter().next()
}

fn can_activate(game: &GameState, player: PlayerId, source: ObjectId) -> bool {
    compute_legal_actions(game, player)
        .unwrap()
        .iter()
        .any(|a| {
            matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source)
                || matches!(a, LegalAction::ActivateManaAbility { source: s, .. } if *s == source)
        })
}

fn activate_first(game: &GameState, player: PlayerId, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
}

fn settle(game: &mut GameState, dm: &mut Dm, queue: &mut TriggerQueue) {
    for _ in 0..64 {
        ironsmith::game_loop::try_drain_pending_trigger_events(game, queue).unwrap();
        ironsmith::game_loop::check_and_apply_sbas_with(game, queue, dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("fixture did not settle");
}

fn resolve_effects(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    effects: Vec<Effect>,
) {
    game.stack.push(ironsmith::game_state::StackEntry::ability(
        source, controller, effects,
    ));
    ironsmith::game_loop::resolve_stack_entry(game).unwrap();
}

fn life(game: &GameState, p: PlayerId) -> i32 {
    game.player(p).unwrap().life
}

fn native(name: &str, types: Vec<CardType>) -> ironsmith::cards::builders::CardDefinitionBuilder {
    ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), name)
        .card_types(types)
        .mana_cost(ironsmith::ManaCost::new())
}

fn bear(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let def = native("Bear", vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
    let id = game.create_object_from_definition(&def, owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn pass_all(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut Dm) -> GameProgress {
    let mut state = ironsmith::PriorityLoopState::new(game.players_in_game());
    let mut last = GameProgress::Continue;
    for _ in 0..game.players_in_game() {
        last = ironsmith::game_loop::apply_priority_response_with_dm(
            game,
            queue,
            &mut state,
            &ironsmith::PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .unwrap();
        if matches!(last, GameProgress::Continue) {
            break;
        }
    }
    last
}

/// Drive the real cleanup scheduler from its discard checkpoint to turn end.
/// Returns (number of RunPriority windows, stack sizes seen at each window,
/// priority player at each window).
fn run_cleanup(
    game: &mut GameState,
    queue: &mut TriggerQueue,
    dm: &mut Dm,
    on_priority: &mut dyn FnMut(&mut GameState),
) -> (usize, Vec<usize>, Vec<Option<PlayerId>>, usize) {
    use ironsmith::{Step, TurnAction, TurnRunner, TurnRunnerState};
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(Step::Cleanup);
    game.turn.priority_player = None;
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::CleanupDiscard);
    let mut windows = 0;
    let mut stacks = Vec::new();
    let mut prio = Vec::new();
    let mut applies = 0;
    for _ in 0..256 {
        if matches!(runner.state(), TurnRunnerState::CleanupApply) {
            applies += 1;
        }
        match runner.advance(game, queue).unwrap() {
            TurnAction::Continue => {}
            TurnAction::Decision(_) => {
                let active = game.turn.active_player;
                let hand = game.player(active).unwrap().hand.clone();
                let excess = hand.len().saturating_sub(7);
                runner.respond_discard(hand.into_iter().take(excess).collect());
            }
            TurnAction::RunPriority => {
                windows += 1;
                assert_eq!(game.turn.step, Some(Step::Cleanup));
                ironsmith::game_loop::advance_priority_with_dm(game, queue, dm).unwrap();
                stacks.push(game.stack.len());
                prio.push(game.turn.priority_player);
                on_priority(game);
                for _ in 0..16 {
                    let progress = pass_all(game, queue, dm);
                    if matches!(progress, GameProgress::Continue) && game.stack.is_empty() {
                        break;
                    }
                    if game.stack.is_empty() {
                        break;
                    }
                }
                runner.priority_done();
            }
            TurnAction::TurnComplete => return (windows, stacks, prio, applies),
            TurnAction::GameOver(r) => panic!("unexpected game over {r:?}"),
        }
    }
    panic!("cleanup did not finish");
}

// ------------------------------------------------------------------ 514.1 / 514.3

#[test]
fn cr_514_1_514_3_cleanup_discards_active_player_to_seven_without_priority() {
    let def = native("Hand filler", vec![CardType::Artifact]).build();
    let mut game = two_player_main(alice());
    for owner in [alice(), bob()] {
        for _ in 0..9 {
            game.create_object_from_definition(&def, owner, Zone::Hand);
        }
    }
    let mut queue = TriggerQueue::new();
    let mut dm = Dm::default();
    let (windows, _, _, _) = run_cleanup(&mut game, &mut queue, &mut dm, &mut |_| {});
    assert_eq!(
        (
            game.player(alice()).unwrap().hand.len(),
            game.player(alice()).unwrap().graveyard.len(),
            game.player(bob()).unwrap().hand.len(),
        ),
        (7, 2, 9),
        "CR 514.1: only the active player discards down to their maximum hand size of seven"
    );
    assert_eq!(
        windows, 0,
        "CR 514.3: with no state-based actions or triggers, no player receives priority during cleanup"
    );
    assert!(
        game.stack.is_empty(),
        "CR 514.1: the discard turn-based action does not use the stack"
    );
}

// ------------------------------------------------------------------ 514.3a

#[test]
fn cr_514_3a_cleanup_trigger_grants_active_player_priority_then_another_cleanup_step() {
    let def = native("Hand filler", vec![CardType::Artifact]).build();
    let mut game = two_player_main(alice());
    for _ in 0..8 {
        game.create_object_from_definition(&def, alice(), Zone::Hand);
    }
    let creature = bear(&mut game, alice());
    let observer = native("Discard observer", vec![CardType::Artifact])
        .with_ability(Ability::triggered(
            Trigger::player_discards_card(ironsmith::PlayerFilter::Any, None),
            vec![
                Effect::gain_life(1),
                Effect::new(DealDamageEffect::new(
                    1,
                    ChooseSpec::SpecificObject(creature),
                )),
            ],
        ))
        .build();
    game.create_object_from_definition(&observer, bob(), Zone::Battlefield);
    let mut queue = TriggerQueue::new();
    let mut dm = Dm::default();
    let (windows, stacks, prio, applies) = run_cleanup(&mut game, &mut queue, &mut dm, &mut |_| {});
    assert_eq!(
        windows, 1,
        "CR 514.3a: the discard trigger opens exactly one cleanup priority window"
    );
    assert_eq!(
        stacks,
        vec![1],
        "CR 514.3a: the waiting trigger is put on the stack before priority"
    );
    assert_eq!(
        prio,
        vec![Some(alice())],
        "CR 514.3a: the active player receives priority"
    );
    assert_eq!(
        life(&game, bob()),
        21,
        "the trigger resolved during cleanup"
    );
    assert!(
        applies >= 2,
        "CR 514.3a: once the stack empties and all pass, another cleanup step begins (cleanup applications: {applies})"
    );
    assert_eq!(
        game.damage_on(creature),
        0,
        "CR 514.3a: the new cleanup step removes damage dealt during the cleanup priority window"
    );
}

// ------------------------------------------------------------------ 601.2d

#[test]
fn cr_601_2d_division_requires_at_least_one_per_target_for_damage_and_counters() {
    let damage_spell = "Mana cost: {0}\nType: Sorcery\nThis spell deals 3 damage divided as you choose among one or two target creatures.";
    let counter_spell = "Mana cost: {0}\nType: Sorcery\nDistribute three +1/+1 counters among one or two target creatures.";
    let mut issues = Vec::new();
    for (kind, text) in [("damage", damage_spell), ("counters", counter_spell)] {
        for split in [vec![1u32, 2], vec![0, 3]] {
            let def = compile("Division spell", text);
            let mut game = two_player_main(alice());
            let a = bear(&mut game, bob());
            let b = bear(&mut game, bob());
            let card = game.create_object_from_definition(&def, alice(), Zone::Hand);
            let mut dm = Dm {
                max_targets: true,
                target_pref: vec![Target::Object(a), Target::Object(b)],
                distribution: Some(split.clone()),
                ..Dm::default()
            };
            let mut queue = TriggerQueue::new();
            let action = cast_action(&game, alice(), card).expect("division spell castable");
            let result = act_with(&mut game, alice(), action, &mut dm, &mut queue);
            if dm.distribute_min_seen.iter().any(|m| *m < 1) {
                issues.push(format!(
                    "{kind}: division prompt minimum {:?}",
                    dm.distribute_min_seen
                ));
            }
            if split.contains(&0) {
                if result.is_ok() && game.stack.len() == 1 {
                    issues.push(format!("{kind}: zero share {split:?} was accepted"));
                }
            } else {
                assert!(
                    result.is_ok(),
                    "{kind}: legal division must cast: {result:?}"
                );
                settle(&mut game, &mut dm, &mut queue);
                let got: Vec<u32> = [a, b]
                    .iter()
                    .map(|id| {
                        if kind == "damage" {
                            game.damage_on(*id)
                        } else {
                            game.counter_count(*id, ironsmith::CounterType::PlusOnePlusOne)
                        }
                    })
                    .collect();
                let mut sorted = got.clone();
                sorted.sort();
                if sorted != vec![1, 2] {
                    issues.push(format!("{kind}: announced 1/2 division produced {got:?}"));
                }
            }
        }
    }
    assert!(
        issues.is_empty(),
        "CR 601.2d: divisions are announced while casting and each target receives at least one: {issues:?}"
    );
}

// ------------------------------------------------------------------ 601.2f

#[test]
fn cr_601_2f_total_cost_applies_increases_and_reductions_and_floors_at_zero() {
    let reducer = compile(
        "Artifact reducer",
        "Mana cost: {0}\nType: Artifact\nArtifact spells you cast cost {1} less to cast.",
    );
    let increaser = compile(
        "Artifact taxer",
        "Mana cost: {0}\nType: Artifact\nArtifact spells cost {1} more to cast.",
    );
    // (spell cost, reducers, increasers, colorless available, expected left over)
    let cases: [(u32, usize, usize, u32, u32); 4] = [
        (1, 2, 0, 0, 0),
        (1, 2, 0, 1, 1),
        (1, 1, 1, 1, 0),
        (2, 1, 0, 1, 0),
    ];
    let mut issues = Vec::new();
    for (cost, reducers, increasers, pool, left) in cases {
        let spell = compile(
            "Costed artifact",
            &format!("Mana cost: {{{cost}}}\nType: Artifact"),
        );
        let mut game = two_player_main(alice());
        for _ in 0..reducers {
            game.create_object_from_definition(&reducer, alice(), Zone::Battlefield);
        }
        for _ in 0..increasers {
            game.create_object_from_definition(&increaser, bob(), Zone::Battlefield);
        }
        game.refresh_continuous_state().unwrap();
        if pool > 0 {
            game.player_mut(alice())
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, pool);
        }
        let card = game.create_object_from_definition(&spell, alice(), Zone::Hand);
        let mut dm = Dm::default();
        let mut queue = TriggerQueue::new();
        let Some(action) = cast_action(&game, alice(), card) else {
            issues.push(format!(
                "cost{cost} r{reducers} i{increasers} pool{pool}: cast not offered"
            ));
            continue;
        };
        let result = act_with(&mut game, alice(), action, &mut dm, &mut queue);
        let remaining = game.player(alice()).unwrap().mana_pool.total();
        if result.is_err() || game.stack.len() != 1 || remaining != left {
            issues.push(format!(
                "cost{cost} r{reducers} i{increasers} pool{pool}: result {result:?}, stack {}, mana left {remaining} (expected {left})",
                game.stack.len()
            ));
        }
    }
    assert!(
        issues.is_empty(),
        "CR 601.2f: total cost is mana cost plus increases minus reductions, never below {{0}}: {issues:?}"
    );
}

// ------------------------------------------------------------------ 602.1 / 602.1a / 602.1b / 602.5d

#[test]
fn cr_602_1a_1b_activator_pays_cost_and_sorcery_instruction_restricts_timing() {
    let def = compile(
        "Sorcery-speed relic",
        "Mana cost: {0}\nType: Artifact\n{2}, {T}: You gain 1 life. Activate only as a sorcery.",
    );
    // Bob's own main phase, empty stack: legal; Bob pays; Bob gains.
    let mut game = two_player_main(bob());
    let relic = game.create_object_from_definition(&def, bob(), Zone::Battlefield);
    game.player_mut(bob())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    game.player_mut(alice())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    assert!(
        !can_activate(&game, alice(), relic),
        "CR 602.2: a non-controller cannot activate the ability"
    );
    let action = activate_first(&game, bob(), relic)
        .expect("CR 602.5d: sorcery-timed ability legal in own main phase with empty stack");
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    act_with(&mut game, bob(), action, &mut dm, &mut queue).unwrap();
    assert!(
        game.is_tapped(relic),
        "CR 602.1a: {{T}} is part of the paid activation cost"
    );
    assert_eq!(
        (
            game.player(bob()).unwrap().mana_pool.total(),
            game.player(alice()).unwrap().mana_pool.total()
        ),
        (0, 2),
        "CR 602.1a: the activating player pays the activation cost"
    );
    assert_eq!(life(&game, bob()), 20, "the effect waits for resolution");
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(
        life(&game, bob()),
        21,
        "CR 602.1: the effect after the colon happens on resolution"
    );
    assert_eq!(life(&game, alice()), 20);

    // Opponent's turn: the activation instruction forbids it even with priority and mana.
    let mut game = two_player_main(alice());
    let relic = game.create_object_from_definition(&def, bob(), Zone::Battlefield);
    game.player_mut(bob())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    game.turn.priority_player = Some(bob());
    assert!(
        !can_activate(&game, bob(), relic),
        "CR 602.1b / 602.5d: 'Activate only as a sorcery' forbids activation on another player's turn"
    );
    // Own main phase with a nonempty stack: also forbidden.
    let mut game = two_player_main(bob());
    let relic = game.create_object_from_definition(&def, bob(), Zone::Battlefield);
    game.player_mut(bob())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let pending = native("Pending ability source", vec![CardType::Artifact]).build();
    let pending = game.create_object_from_definition(&pending, alice(), Zone::Battlefield);
    game.stack.push(ironsmith::game_state::StackEntry::ability(
        pending,
        alice(),
        vec![Effect::gain_life(1)],
    ));
    assert!(
        !can_activate(&game, bob(), relic),
        "CR 602.5d: sorcery timing requires an empty stack"
    );
}

// ------------------------------------------------------------------ 602.2

#[test]
fn cr_602_2_only_controller_or_owner_activates_and_control_change_moves_permission() {
    let def = native("Free relic", vec![CardType::Artifact])
        .with_ability(Ability::activated(
            ironsmith::TotalCost::free(),
            vec![Effect::gain_life(1)],
        ))
        .build();
    let mut game = two_player_main(alice());
    let relic = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
    game.turn.priority_player = Some(bob());
    assert!(
        !can_activate(&game, bob(), relic),
        "CR 602.2: only the controller may activate"
    );
    game.turn.priority_player = Some(alice());
    assert!(can_activate(&game, alice(), relic));
    ApplyContinuousEffect::new(
        EffectTarget::Specific(relic),
        Modification::ChangeController(bob()),
        Until::Forever,
    )
    .execute(&mut game, &mut EffectContext::new_default(relic, bob()))
    .unwrap();
    game.refresh_continuous_state().unwrap();
    assert!(
        !can_activate(&game, alice(), relic),
        "CR 602.2: the owner no longer controls it and cannot activate"
    );
    game.turn.priority_player = Some(bob());
    assert!(
        can_activate(&game, bob(), relic),
        "CR 602.2: the new controller may activate"
    );

    // An object with no controller (a card in hand) is activated by its owner only.
    let mut hand_ability =
        Ability::activated(ironsmith::TotalCost::free(), vec![Effect::gain_life(1)]);
    hand_ability.functional_zones = vec![Zone::Hand];
    let hand_def = native("Hand ability card", vec![CardType::Artifact])
        .with_ability(hand_ability)
        .build();
    let mut game = two_player_main(alice());
    let card = game.create_object_from_definition(&hand_def, alice(), Zone::Hand);
    assert!(
        can_activate(&game, alice(), card),
        "CR 602.2: the owner activates a hand ability"
    );
    game.turn.priority_player = Some(bob());
    assert!(
        !can_activate(&game, bob(), card),
        "CR 602.2: a non-owner cannot activate a hand ability"
    );
}

// ------------------------------------------------------------------ 602.2a

#[test]
fn cr_602_2a_activated_ability_is_topmost_noncard_object_controlled_by_activator() {
    let def = native("Free relic", vec![CardType::Artifact])
        .with_ability(Ability::activated(
            ironsmith::TotalCost::free(),
            vec![Effect::gain_life(1)],
        ))
        .build();
    let mut game = two_player_main(alice());
    let relic = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(relic),
        Modification::ChangeController(bob()),
        Until::Forever,
    )
    .execute(&mut game, &mut EffectContext::new_default(relic, bob()))
    .unwrap();
    game.refresh_continuous_state().unwrap();
    let below = native("Earlier ability source", vec![CardType::Artifact]).build();
    let below = game.create_object_from_definition(&below, alice(), Zone::Battlefield);
    game.stack.push(ironsmith::game_state::StackEntry::ability(
        below,
        alice(),
        vec![Effect::gain_life(5)],
    ));
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    game.turn.priority_player = Some(bob());
    let action = activate_first(&game, bob(), relic).expect("controller may activate");
    act_with(&mut game, bob(), action, &mut dm, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 2);
    let top = game.stack.last().unwrap();
    assert!(
        top.is_ability,
        "CR 602.2a: the ability on the stack is not a card"
    );
    assert_eq!(top.object_id, relic);
    assert_eq!(
        top.controller,
        bob(),
        "CR 602.2a: its controller is the activating player (not the owner)"
    );
    assert_eq!(
        game.object(relic).unwrap().zone,
        Zone::Battlefield,
        "CR 602.2a: activation leaves the source where it is"
    );
    let progress = pass_all(&mut game, &mut queue, &mut dm);
    let _ = progress;
    assert_eq!(
        (life(&game, bob()), life(&game, alice())),
        (21, 20),
        "CR 602.2a / 608.1: the topmost ability (Bob's) resolves first"
    );
}

#[test]
fn cr_602_2a_activating_from_hand_reveals_the_card() {
    let mut hand_ability =
        Ability::activated(ironsmith::TotalCost::free(), vec![Effect::gain_life(1)]);
    hand_ability.functional_zones = vec![Zone::Hand];
    let def = native("Hidden-zone ability card", vec![CardType::Artifact])
        .with_ability(hand_ability)
        .build();
    let mut game = two_player_main(alice());
    let card = game.create_object_from_definition(&def, alice(), Zone::Hand);
    let observer = native("Reveal observer", vec![CardType::Artifact])
        .with_ability(Ability::triggered(
            Trigger::player_reveals_card(
                ironsmith::PlayerFilter::Any,
                ironsmith::ObjectFilter::default(),
                false,
            ),
            vec![Effect::gain_life(1)],
        ))
        .build();
    game.create_object_from_definition(&observer, bob(), Zone::Battlefield);
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    let action = activate_first(&game, alice(), card).expect("hand ability activatable");
    act_with(&mut game, alice(), action, &mut dm, &mut queue).unwrap();
    // CR 603.3: the observer's reveal trigger is put on the stack the next
    // time a player would receive priority, right after the activation.
    assert!(!game.stack.is_empty(), "the ability was activated");
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(life(&game, alice()), 21);
    let revealed_to_bob = dm
        .public_views
        .iter()
        .any(|(viewer, cards)| *viewer == bob() && cards.contains(&card));
    assert!(
        revealed_to_bob || life(&game, bob()) == 21,
        "CR 602.2a: activating an ability from a hidden zone reveals the card that has it (public views: {:?}, reveal-observer life {})",
        dm.public_views,
        life(&game, bob())
    );
}

// ------------------------------------------------------------------ 602.5

#[test]
fn cr_602_5_prohibited_artifact_abilities_cannot_begin_activation() {
    let ouphe = compile(
        "Collector Ouphe",
        "Mana cost: {1}{G}\nType: Creature — Ouphe\nPower/Toughness: 2/2\nActivated abilities of artifacts can't be activated.",
    );
    let relic = native("Free relic", vec![CardType::Artifact])
        .with_ability(Ability::activated(
            ironsmith::TotalCost::free(),
            vec![Effect::gain_life(1)],
        ))
        .build();
    let rock = compile("Mana rock", "Mana cost: {0}\nType: Artifact\n{T}: Add {C}.");
    let mut game = two_player_main(alice());
    let relic = game.create_object_from_definition(&relic, alice(), Zone::Battlefield);
    let rock = game.create_object_from_definition(&rock, alice(), Zone::Battlefield);
    assert!(can_activate(&game, alice(), relic) && can_activate(&game, alice(), rock));
    let blocker = game.create_object_from_definition(&ouphe, bob(), Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(
        !can_activate(&game, alice(), relic),
        "CR 602.5: a prohibited activated ability cannot be activated"
    );
    assert!(
        !can_activate(&game, alice(), rock),
        "CR 602.5: the prohibition also stops artifact mana abilities"
    );
    resolve_effects(
        &mut game,
        relic,
        alice(),
        vec![Effect::new(ExileEffect::with_spec(
            ChooseSpec::SpecificObject(blocker),
        ))],
    );
    game.refresh_continuous_state().unwrap();
    assert!(
        can_activate(&game, alice(), relic),
        "CR 602.5: once the prohibition ends the ability is activatable again"
    );
}

// ------------------------------------------------------------------ 602.5a

#[test]
fn cr_602_5a_tap_ability_needs_continuous_control_since_turn_start_except_haste_or_noncreature() {
    let tapper = compile(
        "Tapper",
        "Mana cost: {0}\nType: Creature — Elf\nPower/Toughness: 1/1\n{T}: You gain 1 life.",
    );
    let hasty = compile(
        "Hasty tapper",
        "Mana cost: {0}\nType: Creature — Elf\nPower/Toughness: 1/1\nHaste\n{T}: You gain 1 life.",
    );
    let relic = compile(
        "Tap relic",
        "Mana cost: {0}\nType: Artifact\n{T}: You gain 1 life.",
    );
    let mut game = two_player_main(bob());
    let mut ids = Vec::new();
    for def in [&tapper, &hasty, &relic] {
        let id = game.create_object_from_definition(def, alice(), Zone::Battlefield);
        game.remove_summoning_sickness(id);
        ids.push(id);
    }
    // Alice has controlled them continuously: she could activate (checked on her turn).
    let mut alice_turn = game.clone();
    alice_turn.turn.active_player = alice();
    alice_turn.turn.priority_player = Some(alice());
    assert!(
        can_activate(&alice_turn, alice(), ids[0]),
        "CR 602.5a: continuously controlled creature may use {{T}}"
    );
    for id in &ids {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(*id),
            Modification::ChangeController(bob()),
            Until::Forever,
        )
        .execute(&mut game, &mut EffectContext::new_default(*id, bob()))
        .unwrap();
    }
    game.refresh_continuous_state().unwrap();
    game.turn.priority_player = Some(bob());
    let got = (
        can_activate(&game, bob(), ids[0]),
        can_activate(&game, bob(), ids[1]),
        can_activate(&game, bob(), ids[2]),
    );
    assert_eq!(
        got,
        (false, true, true),
        "CR 602.5a: a creature gained this turn cannot use {{T}} unless it has haste; noncreature artifacts are unaffected"
    );
}

// ------------------------------------------------------------------ 603.6a

#[test]
fn cr_603_6a_simultaneous_entrants_see_each_other_and_existing_permanents_see_both() {
    let watcher = compile(
        "Entry watcher",
        "Mana cost: {0}\nType: Creature — Elf\nPower/Toughness: 1/1\nWhenever another creature enters, you gain 1 life.",
    );
    let mass = compile(
        "Mass return",
        "Mana cost: {0}\nType: Sorcery\nReturn all creature cards from your graveyard to the battlefield.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&watcher, alice(), Zone::Battlefield);
    game.create_object_from_definition(&watcher, alice(), Zone::Graveyard);
    game.create_object_from_definition(&watcher, alice(), Zone::Graveyard);
    let spell = game.create_object_from_definition(&mass, alice(), Zone::Hand);
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    let action = cast_action(&game, alice(), spell).unwrap();
    act_with(&mut game, alice(), action, &mut dm, &mut queue).unwrap();
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|id| game.current_is_creature(**id))
            .count(),
        3
    );
    assert_eq!(
        life(&game, alice()),
        24,
        "CR 603.6a: each newcomer sees the other newcomer (2) and the existing watcher sees both (2)"
    );
}

// ------------------------------------------------------------------ 603.11 / 607.2h

#[test]
fn cr_603_11_607_2h_rowen_static_reveal_linked_trigger_only_for_its_own_reveals() {
    let rowen = compile(
        "Rowen",
        "Mana cost: {2}{G}{G}\nType: Enchantment\nReveal the first card you draw each turn. Whenever you reveal a basic land card this way, draw a card.",
    );
    let forest = compile("Forest", "Type: Basic Land — Forest");
    let filler = native("Library filler", vec![CardType::Artifact]).build();
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&rowen, alice(), Zone::Battlefield);
    for _ in 0..5 {
        game.create_object_from_definition(&filler, alice(), Zone::Library);
    }
    // Library top is the last created card in most fixtures; put Forests on top.
    game.create_object_from_definition(&forest, alice(), Zone::Library);
    let src = native("Draw source", vec![CardType::Artifact]).build();
    let src = game.create_object_from_definition(&src, alice(), Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let top = *game.player(alice()).unwrap().library.last().unwrap();
    let top_is_forest = game.object(top).unwrap().name == "Forest";
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    resolve_effects(&mut game, src, alice(), vec![Effect::draw(1)]);
    settle(&mut game, &mut dm, &mut queue);
    let after_first = game.player(alice()).unwrap().hand.len();
    // A second draw this turn is not revealed by Rowen.
    resolve_effects(&mut game, src, alice(), vec![Effect::draw(1)]);
    settle(&mut game, &mut dm, &mut queue);
    let after_second = game.player(alice()).unwrap().hand.len();
    if top_is_forest {
        assert_eq!(
            (after_first, after_second),
            (2, 3),
            "CR 603.11 / 607.2h: the linked trigger draws once for the static ability's first-draw reveal of a basic land, and not for later draws"
        );
    } else {
        // Library top is not the Forest under this fixture's ordering: the first
        // draw reveals a nonland, so no linked draw.
        assert_eq!(
            (after_first, after_second),
            (1, 2),
            "CR 607.2h: nonbasic reveal draws nothing extra"
        );
    }
    assert!(
        top_is_forest,
        "fixture: the Forest must be the top library card"
    );
}

// ------------------------------------------------------------------ 604.3a

#[test]
fn cr_604_3a_only_self_defining_printed_abilities_function_as_cdas_off_battlefield() {
    let cda = compile(
        "All-color courier",
        "Mana cost: {3}\nType: Artifact Creature — Golem\nPower/Toughness: 1/1\nThis card is all colors.",
    );
    let lord = compile(
        "Blue lord",
        "Mana cost: {R}\nType: Creature — Goblin\nPower/Toughness: 1/1\nCreatures you control are blue.",
    );
    let mut game = two_player_main(alice());
    let courier = game.create_object_from_definition(&cda, alice(), Zone::Hand);
    let lord_card = game.create_object_from_definition(&lord, alice(), Zone::Hand);
    game.refresh_continuous_state().unwrap();
    let courier_colors = game.current_colors(courier).unwrap();
    assert!(
        courier_colors.contains_all(
            ironsmith::color::ColorSet::WHITE
                .union(ironsmith::color::ColorSet::BLUE)
                .union(ironsmith::color::ColorSet::BLACK)
                .union(ironsmith::color::ColorSet::RED)
                .union(ironsmith::color::ColorSet::GREEN)
        ),
        "CR 604.3 / 604.3a: a printed self-only color definition is a CDA and works in hand: {courier_colors:?}"
    );
    let lord_colors = game.current_colors(lord_card).unwrap();
    assert!(
        !lord_colors.contains(ironsmith::color::Color::Blue),
        "CR 604.3a(3): an ability that affects other objects is not a CDA and does not function in hand: {lord_colors:?}"
    );
}

// ------------------------------------------------------------------ 608.1

#[test]
fn cr_608_1_all_passing_resolves_only_the_top_object() {
    let def = native("Ability source", vec![CardType::Artifact]).build();
    let mut game = two_player_main(alice());
    let src = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
    game.stack.push(ironsmith::game_state::StackEntry::ability(
        src,
        alice(),
        vec![Effect::gain_life(1)],
    ));
    game.stack.push(ironsmith::game_state::StackEntry::ability(
        src,
        bob(),
        vec![Effect::gain_life(2)],
    ));
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    pass_all(&mut game, &mut queue, &mut dm);
    assert_eq!(
        (game.stack.len(), life(&game, alice()), life(&game, bob())),
        (1, 20, 22),
        "CR 608.1: one round of passing resolves exactly the top object"
    );
    ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    pass_all(&mut game, &mut queue, &mut dm);
    assert_eq!(
        (game.stack.len(), life(&game, alice()), life(&game, bob())),
        (0, 21, 22),
        "CR 608.1: the next round resolves the next object"
    );
}

// ------------------------------------------------------------------ 608.3b

#[test]
fn cr_608_3b_illegal_target_spell_does_not_resolve_but_bestow_becomes_a_creature() {
    // Ordinary spell with its only target gone.
    let spell = compile(
        "Blessing",
        "Mana cost: {0}\nType: Instant\nPut a +1/+1 counter on target creature. You gain 3 life.",
    );
    let mut game = two_player_main(alice());
    let target = bear(&mut game, bob());
    let card = game.create_object_from_definition(&spell, alice(), Zone::Hand);
    let stable = game.object(card).map(|o| o.stable_id).unwrap();
    let mut dm = Dm {
        target_pref: vec![Target::Object(target)],
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    let action = cast_action(&game, alice(), card).unwrap();
    act_with(&mut game, alice(), action, &mut dm, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    resolve_effects(
        &mut game,
        target,
        bob(),
        vec![Effect::new(ExileEffect::with_spec(
            ChooseSpec::SpecificObject(target),
        ))],
    );
    settle(&mut game, &mut dm, &mut queue);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        (life(&game, alice()), game.object(now).unwrap().zone),
        (20, Zone::Graveyard),
        "CR 608.3b: a spell whose only target is illegal does not resolve and goes to its owner's graveyard"
    );

    // Bestowed Aura spell whose target became illegal resolves as a creature.
    let siren = compile(
        "Bestow siren",
        "Mana cost: {0}\nType: Enchantment Creature — Siren\nPower/Toughness: 1/1\nBestow {0} (If you cast this card for its bestow cost, it's an Aura spell with enchant creature. It becomes a creature again if it's not attached.)\nEnchanted creature gets +1/+1.",
    );
    let mut game = two_player_main(alice());
    let host = bear(&mut game, bob());
    let card = game.create_object_from_definition(&siren, alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let bestow = cast_actions(&game, alice(), card)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { casting_method, .. } if *casting_method != ironsmith::CastingMethod::Normal))
        .expect("bestow cast offered");
    let mut dm = Dm {
        target_pref: vec![Target::Object(host)],
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    act_with(&mut game, alice(), bestow, &mut dm, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    resolve_effects(
        &mut game,
        host,
        bob(),
        vec![Effect::new(ExileEffect::with_spec(
            ChooseSpec::SpecificObject(host),
        ))],
    );
    settle(&mut game, &mut dm, &mut queue);
    let now = game.find_object_by_stable_id(stable).unwrap();
    let obj = game.object(now).unwrap();
    assert_eq!(
        (
            obj.zone,
            game.current_is_creature(now),
            obj.attached_to.is_some()
        ),
        (Zone::Battlefield, true, false),
        "CR 608.3b / 702.103e: a bestowed Aura spell with an illegal target resolves as an unattached creature"
    );
}

// ------------------------------------------------------------------ 609.3

#[test]
fn cr_609_3_impossible_discard_and_mill_do_as_much_as_possible() {
    let def = native("Card", vec![CardType::Artifact]).build();
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&def, alice(), Zone::Hand);
    game.create_object_from_definition(&def, alice(), Zone::Library);
    let src = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
    resolve_effects(
        &mut game,
        src,
        alice(),
        vec![Effect::new(ironsmith::effects::DiscardEffect::you(2))],
    );
    assert_eq!(
        (
            game.player(alice()).unwrap().hand.len(),
            game.player(alice()).unwrap().graveyard.len()
        ),
        (0, 1),
        "CR 609.3: discard two with one card in hand discards that card"
    );
    resolve_effects(&mut game, src, alice(), vec![Effect::mill(3)]);
    assert_eq!(
        (
            game.player(alice()).unwrap().library.len(),
            game.player(alice()).unwrap().graveyard.len()
        ),
        (0, 2),
        "CR 609.3: milling three from a one-card library moves the one card"
    );
}

// ------------------------------------------------------------------ 610.4

#[test]
fn cr_610_4_phase_out_until_source_leaves_phases_in_when_it_leaves() {
    let watcher = native("Phase watcher", vec![CardType::Artifact]).build();
    for owner in [alice(), bob()] {
        let mut game = two_player_main(alice());
        let creature = bear(&mut game, owner);
        let src = game.create_object_from_definition(&watcher, alice(), Zone::Battlefield);
        resolve_effects(
            &mut game,
            src,
            alice(),
            vec![Effect::new(
                PhaseOutEffect::with_spec(ChooseSpec::SpecificObject(creature))
                    .until_source_leaves(),
            )],
        );
        assert!(game.is_phased_out(creature));
        let mut dm = Dm::default();
        let mut queue = TriggerQueue::new();
        settle(&mut game, &mut dm, &mut queue);
        assert!(
            game.is_phased_out(creature),
            "the duration holds while the source remains"
        );
        resolve_effects(
            &mut game,
            src,
            alice(),
            vec![Effect::new(ExileEffect::with_spec(
                ChooseSpec::SpecificObject(src),
            ))],
        );
        settle(&mut game, &mut dm, &mut queue);
        assert!(
            !game.is_phased_out(creature),
            "CR 610.4: when the specified event happens, a second one-shot effect phases the permanent in"
        );
        assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.current_controller(creature), Some(owner));
    }
}

// ------------------------------------------------------------------ 611.3c

#[test]
fn cr_611_3c_static_pt_applies_as_creature_enters_before_etb_trigger_check() {
    let anthem = compile(
        "Anthem",
        "Mana cost: {0}\nType: Enchantment\nCreatures you control get +1/+1.",
    );
    let watcher = compile(
        "Big entry watcher",
        "Mana cost: {0}\nType: Enchantment\nWhenever a creature with power 3 or greater enters, you gain 1 life.",
    );
    let creature = compile(
        "Two-power entrant",
        "Mana cost: {0}\nType: Creature — Elf\nPower/Toughness: 2/2",
    );
    let mut results = Vec::new();
    for with_anthem in [false, true] {
        let mut game = two_player_main(alice());
        if with_anthem {
            game.create_object_from_definition(&anthem, alice(), Zone::Battlefield);
        }
        game.create_object_from_definition(&watcher, alice(), Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let card = game.create_object_from_definition(&creature, alice(), Zone::Hand);
        let mut dm = Dm::default();
        let mut queue = TriggerQueue::new();
        let action = cast_action(&game, alice(), card).unwrap();
        act_with(&mut game, alice(), action, &mut dm, &mut queue).unwrap();
        settle(&mut game, &mut dm, &mut queue);
        results.push(life(&game, alice()));
    }
    assert_eq!(
        results,
        vec![20, 21],
        "CR 611.3c: the anthem applies as the 2/2 enters, so it enters as a 3/3 and triggers the power-3 watcher"
    );
}

// ------------------------------------------------------------------ 605.4

#[test]
fn cr_605_4_triggered_mana_ability_triggers_from_mana_event_and_resolves_immediately() {
    let heartbeat = compile(
        "Heartbeat of Spring",
        "Mana cost: {2}{G}\nType: Enchantment\nWhenever a player taps a land for mana, that player adds one mana of any type that land produced.",
    );
    let forest = compile("Forest", "Type: Basic Land — Forest");
    let rock = compile("Mana rock", "Mana cost: {0}\nType: Artifact\n{T}: Add {C}.");
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&heartbeat, bob(), Zone::Battlefield);
    let land = game.create_object_from_definition(&forest, alice(), Zone::Battlefield);
    let rock = game.create_object_from_definition(&rock, alice(), Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    let tap_land = compute_legal_actions(&game, alice())
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == land))
        .expect("Forest mana ability offered");
    act_with(&mut game, alice(), tap_land, &mut dm, &mut queue).unwrap();
    let pool = &game.player(alice()).unwrap().mana_pool;
    assert_eq!(
        (pool.green, game.stack.len()),
        (2, 0),
        "CR 605.4 / 605.4a: the triggered mana ability triggers from the mana ability and resolves immediately without using the stack"
    );
    let tap_rock = compute_legal_actions(&game, alice())
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == rock))
        .expect("rock mana ability offered");
    act_with(&mut game, alice(), tap_rock, &mut dm, &mut queue).unwrap();
    let pool = &game.player(alice()).unwrap().mana_pool;
    assert_eq!(
        (pool.green, pool.colorless, game.stack.len()),
        (2, 1, 0),
        "CR 605.4 / 603.2: like any trigger, it only triggers on its own event (a land, not an artifact)"
    );
}

// ------------------------------------------------------------------ 611.3d

#[test]
fn cr_611_3d_cast_permission_haste_rider_lasts_after_provider_leaves_and_cleanup() {
    use ironsmith::grant::{GrantSpec, Grantable};
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith::{ObjectFilter, PlayerFilter};
    fn has_haste(game: &GameState, id: ObjectId) -> bool {
        game.current_abilities(id).unwrap().iter().any(|a| {
            matches!(&a.kind, ironsmith::AbilityKind::Static(s) if s.id() == StaticAbility::haste().id())
        })
    }
    let mut issues = Vec::new();
    for provider_leaves in [false, true] {
        let mut game = two_player_main(alice());
        let mut permission = GrantSpec::new(
            Grantable::PlayFrom,
            ObjectFilter::creature()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
            Zone::Graveyard,
        );
        // No stated duration: modeled by the indefinite recipient rider
        // (cast_this_way_grants is the explicit until-end-of-turn rider).
        permission
            .permanent_this_way_grants
            .push(StaticAbility::haste());
        let provider_def = native("Static permission provider", vec![CardType::Artifact])
            .with_ability(Ability::static_ability(StaticAbility::grants(permission)))
            .build();
        let provider =
            game.create_object_from_definition(&provider_def, alice(), Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let subject_def = native("Permission recipient", vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(2, 3))
            .build();
        let old = game.create_object_from_definition(&subject_def, alice(), Zone::Graveyard);
        let stable = game.object(old).unwrap().stable_id;
        let action = compute_legal_actions(&game, alice())
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, .. } if *spell_id == old))
            .expect("static permission offers the graveyard cast");
        let mut dm = Dm::default();
        let mut queue = TriggerQueue::new();
        act_with(&mut game, alice(), action, &mut dm, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        if provider_leaves {
            resolve_effects(
                &mut game,
                provider,
                bob(),
                vec![Effect::new(ExileEffect::with_spec(
                    ChooseSpec::SpecificObject(provider),
                ))],
            );
        }
        settle(&mut game, &mut dm, &mut queue);
        let creature = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
        let on_entry = has_haste(&game, creature);
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        let after_cleanup = has_haste(&game, creature);
        if !(on_entry && after_cleanup) {
            issues.push(format!(
                "provider_leaves={provider_leaves}: haste on entry {on_entry}, after cleanup {after_cleanup}"
            ));
        }
    }
    assert!(
        issues.is_empty(),
        "CR 611.3d: a battlefield ability granted with a cast permission and no stated duration lasts until the end of the game, even after the permission source leaves: {issues:?}"
    );
}
