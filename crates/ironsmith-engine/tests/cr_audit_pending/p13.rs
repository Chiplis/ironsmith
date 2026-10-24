//! CR audit (pending-entry pass, package p13): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{self, AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration, LegalAction};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_state::Target;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CastingMethod, CounterType, DecisionMaker, ManaSymbol, ObjectId, Phase, Zone};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
const CAROL: PlayerId = PlayerId(2);

fn p13_compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture card {name} must compile: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture card {name} compiled with unimplemented content"
    );
    def
}

fn p13_try_compile(name: &str, text: &str) -> Result<CardDefinition, String> {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .map_err(|e| format!("{e:?}"))?;
    if ironsmith::cards::generated_definition_has_unimplemented_content(&def) {
        return Err("unimplemented content".into());
    }
    Ok(def)
}

fn p13_game(players: usize) -> GameState {
    let names = ["Alice", "Bob", "Carol", "Dana"][..players]
        .iter()
        .map(|n| n.to_string())
        .collect();
    let mut game = GameState::new(names, 20);
    game.turn.active_player = ALICE;
    game.turn.priority_player = Some(ALICE);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn p13_vanilla(power: i32, toughness: i32) -> CardDefinition {
    p13_compile(
        "Vanilla",
        &format!("Mana cost: {{1}}\nType: Creature — Bear\nPower/Toughness: {power}/{toughness}"),
    )
}

fn p13_put(game: &mut GameState, def: &CardDefinition, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = game.create_object_from_definition(def, owner, zone);
    if zone == Zone::Battlefield {
        game.remove_summoning_sickness(id);
    }
    id
}

fn p13_fill_library(game: &mut GameState, owner: PlayerId, n: usize) {
    let def = p13_vanilla(1, 1);
    for _ in 0..n {
        game.create_object_from_definition(&def, owner, Zone::Library);
    }
}

fn p13_owned_in(game: &GameState, owner: PlayerId, zone: Zone) -> Vec<ObjectId> {
    game.objects_in_zone(zone)
        .into_iter()
        .filter(|id| game.object(*id).is_some_and(|o| o.owner == owner))
        .collect()
}

fn p13_controlled_battlefield(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    game.objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| game.current_controller(*id) == Some(controller))
        .collect()
}

fn p13_named(game: &GameState, zone: Zone, name: &str) -> Vec<ObjectId> {
    game.objects_in_zone(zone)
        .into_iter()
        .filter(|id| game.object(*id).is_some_and(|o| o.name == name))
        .collect()
}

fn p13_plus(game: &GameState, id: ObjectId) -> u32 {
    game.counter_count(id, CounterType::PlusOnePlusOne)
}

fn p13_mana(game: &mut GameState, player: PlayerId, symbols: &[(ManaSymbol, u32)]) {
    for (symbol, n) in symbols {
        game.player_mut(player).unwrap().mana_pool.add(*symbol, *n);
    }
}

/// Scripted decision maker: answers booleans with `yes`, prefers option
/// descriptions containing any of `prefer`, and logs every prompt.
struct P13Dm {
    yes: bool,
    prefer: Vec<String>,
    targets: Option<Vec<Target>>,
    objects: Option<Vec<ObjectId>>,
    number: Option<u32>,
    log: Vec<String>,
}

impl P13Dm {
    fn new(yes: bool) -> Self {
        Self {
            yes,
            prefer: Vec::new(),
            targets: None,
            objects: None,
            number: None,
            log: Vec::new(),
        }
    }
    fn prefer(mut self, s: &str) -> Self {
        self.prefer.push(s.to_ascii_lowercase());
        self
    }
}

impl DecisionMaker for P13Dm {
    fn decide_boolean(&mut self, _g: &GameState, ctx: &BooleanContext) -> bool {
        self.log.push(format!("bool:{}", ctx.description));
        self.yes && ctx.can_accept
    }
    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.log.push(format!("objects:{}", ctx.description));
        if let Some(wanted) = &self.objects {
            let chosen: Vec<ObjectId> = wanted
                .iter()
                .copied()
                .filter(|id| ctx.candidates.iter().any(|c| c.id == *id && c.legal))
                .collect();
            if !chosen.is_empty() && chosen.len() >= ctx.min {
                return chosen;
            }
        }
        ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, ctx)
    }
    fn decide_options(&mut self, g: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.log.push(format!(
            "options:{}:[{}]",
            ctx.description,
            ctx.options
                .iter()
                .map(|o| o.description.clone())
                .collect::<Vec<_>>()
                .join("|")
        ));
        if !self.prefer.is_empty() {
            let mut picked: Vec<usize> = ctx
                .options
                .iter()
                .filter(|o| {
                    o.legal
                        && self
                            .prefer
                            .iter()
                            .any(|p| o.description.to_ascii_lowercase().contains(p.as_str()))
                })
                .map(|o| o.index)
                .take(ctx.max.max(1))
                .collect();
            if !picked.is_empty() {
                for o in ctx.options.iter().filter(|o| o.legal) {
                    if picked.len() >= ctx.min {
                        break;
                    }
                    if !picked.contains(&o.index) {
                        picked.push(o.index);
                    }
                }
                return picked;
            }
        }
        ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, ctx)
    }
    fn decide_number(
        &mut self,
        _g: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        self.log.push(format!(
            "number:{}:{}..{}",
            ctx.description, ctx.min, ctx.max
        ));
        self.number.unwrap_or(ctx.max).clamp(ctx.min, ctx.max)
    }
    fn decide_targets(&mut self, g: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.log.push(format!("targets:{}", ctx.context));
        if let Some(t) = &self.targets {
            return t.clone();
        }
        ironsmith::decision::SelectFirstDecisionMaker.decide_targets(g, ctx)
    }
}

/// Drain triggers, apply SBAs, and resolve the stack until it is empty, by
/// having every player pass priority (the native resolution path, which
/// publishes resolution-time trigger events such as "becomes crewed").
fn p13_settle<D: DecisionMaker>(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut D) {
    use ironsmith::decision::GameProgress;
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let mut state = PriorityLoopState::new(game.players_in_game());
    for _ in 0..400 {
        ironsmith::game_loop::try_drain_pending_trigger_events(game, queue).unwrap();
        ironsmith::game_loop::check_and_apply_sbas(game, queue).unwrap();
        ironsmith::game_loop::try_drain_pending_trigger_events(game, queue).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, dm).unwrap();
        if game.stack.is_empty() && queue.entries.is_empty() {
            return;
        }
        if game.turn.priority_player.is_none() {
            game.turn.priority_player = Some(game.turn.active_player);
        }
        let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
            game,
            queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        );
        for _ in 0..32 {
            match &result {
                Ok(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(_)))
                | Ok(GameProgress::Continue)
                | Ok(GameProgress::StackResolved)
                | Ok(GameProgress::GameOver(_)) => break,
                Ok(GameProgress::NeedsDecisionCtx(ctx)) => {
                    let ctx = ctx.clone();
                    result = ironsmith::game_loop::apply_decision_context_with_dm(
                        game, queue, &mut state, &ctx, dm,
                    );
                }
                Err(e) => panic!("fixture resolution failed: {e:?}"),
            }
        }
        if matches!(result, Ok(GameProgress::GameOver(_))) {
            return;
        }
    }
    panic!("fixture stack did not settle");
}

/// Take a priority action through the native pipeline with a scripted decision maker.
fn p13_act<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: LegalAction,
    dm: &mut D,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    game.turn.priority_player = Some(player);
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
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
                &mut queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                dm,
            );
        } else {
            let context = match &result {
                Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, &mut queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    if state.pending_cast.is_some()
        || state.pending_method_selection.is_some()
        || state.pending_activation.is_some()
    {
        return Err("action exceeded decision bound".into());
    }
    // Cast triggers live in the local queue.
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    Ok(())
}

fn p13_legal(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    ironsmith::compute_legal_actions(game, player).unwrap()
}

fn p13_cast_action(spell: ObjectId, from: Zone, method: CastingMethod) -> LegalAction {
    LegalAction::CastSpell {
        spell_id: spell,
        from_zone: from,
        casting_method: method,
    }
}

/// Declare attackers, then resolve attack triggers.
fn p13_attack<D: DecisionMaker>(
    game: &mut GameState,
    decls: &[(ObjectId, AttackTarget)],
    dm: &mut D,
) -> Result<(), String> {
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let mut combat = combat_state::new_combat();
    let mut queue = TriggerQueue::new();
    let decls: Vec<AttackerDeclaration> = decls
        .iter()
        .map(|(c, t)| AttackerDeclaration {
            creature: *c,
            target: t.clone(),
        })
        .collect();
    ironsmith::game_loop::apply_attacker_declarations_with_dm(
        game,
        &mut combat,
        &mut queue,
        &decls,
        dm,
    )
    .map_err(|e| format!("{e:?}"))?;
    game.combat = Some(combat);
    p13_settle(game, &mut queue, dm);
    Ok(())
}

fn p13_block<D: DecisionMaker>(
    game: &mut GameState,
    defender: PlayerId,
    decls: &[(ObjectId, ObjectId)],
    dm: &mut D,
) -> Result<(), String> {
    game.turn.step = Some(ironsmith::Step::DeclareBlockers);
    let mut combat = game.combat.clone().unwrap();
    let mut queue = TriggerQueue::new();
    let decls: Vec<BlockerDeclaration> = decls
        .iter()
        .map(|(b, a)| BlockerDeclaration {
            blocker: *b,
            blocking: *a,
        })
        .collect();
    ironsmith::game_loop::apply_blocker_declarations(
        game,
        &mut combat,
        &mut queue,
        &decls,
        defender,
    )
    .map_err(|e| format!("{e:?}"))?;
    game.combat = Some(combat);
    p13_settle(game, &mut queue, dm);
    Ok(())
}

fn p13_damage<D: DecisionMaker>(game: &mut GameState, dm: &mut D) {
    game.turn.step = Some(ironsmith::Step::CombatDamage);
    let combat = game.combat.clone().unwrap();
    let events = ironsmith::game_loop::execute_combat_damage_step_with_dm(game, &combat, false, dm);
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::try_queue_combat_damage_triggers(game, &events, &mut queue).unwrap();
    p13_settle(game, &mut queue, dm);
}

fn p13_end_combat<D: DecisionMaker>(game: &mut GameState, dm: &mut D) {
    game.turn.step = Some(ironsmith::Step::EndCombat);
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::generate_and_queue_step_triggers(game, &mut queue);
    p13_settle(game, &mut queue, dm);
}

fn p13_full_unblocked_combat<D: DecisionMaker>(
    game: &mut GameState,
    decls: &[(ObjectId, AttackTarget)],
    dm: &mut D,
) {
    p13_attack(game, decls, dm).unwrap();
    p13_block(game, BOB, &[], dm).unwrap();
    p13_damage(game, dm);
}

// ---------------------------------------------------------------------------
// 702.112 Renown
// ---------------------------------------------------------------------------

#[test]
fn cr_702_112a_b_c_renown_triggers_once_and_renowned_ends_on_zone_change() {
    let text = "Mana cost: {1}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/2\nVigilance\nRenown 1";
    let topan = p13_compile("Topan Freeblade", text);
    let mut game = p13_game(2);
    let creature = p13_put(&mut game, &topan, ALICE, Zone::Battlefield);
    assert!(!game.is_renowned(creature));
    let mut dm = P13Dm::new(true);
    p13_full_unblocked_combat(&mut game, &[(creature, AttackTarget::Player(BOB))], &mut dm);
    assert_eq!(game.player(BOB).unwrap().life, 18);
    assert_eq!(
        p13_plus(&game, creature),
        1,
        "CR 702.112a: renown 1 adds one +1/+1 counter"
    );
    assert!(
        game.is_renowned(creature),
        "CR 702.112a: the creature becomes renowned"
    );
    // Second combat damage: intervening if fails.
    p13_full_unblocked_combat(&mut game, &[(creature, AttackTarget::Player(BOB))], &mut dm);
    assert_eq!(game.player(BOB).unwrap().life, 15);
    assert_eq!(
        p13_plus(&game, creature),
        1,
        "CR 702.112a: a renowned creature gets no more counters"
    );

    // 702.112c: two instances trigger separately, the second does nothing.
    let double = p13_compile(
        "Double Renown",
        "Mana cost: {1}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/2\nRenown 1\nRenown 2",
    );
    let mut game = p13_game(2);
    let creature = p13_put(&mut game, &double, ALICE, Zone::Battlefield);
    p13_attack(&mut game, &[(creature, AttackTarget::Player(BOB))], &mut dm).unwrap();
    p13_block(&mut game, BOB, &[], &mut dm).unwrap();
    game.turn.step = Some(ironsmith::Step::CombatDamage);
    let combat = game.combat.clone().unwrap();
    let events = ironsmith::game_loop::execute_combat_damage_step_with_dm(
        &mut game, &combat, false, &mut dm,
    );
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::try_queue_combat_damage_triggers(&mut game, &events, &mut queue).unwrap();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
    assert_eq!(
        queue.entries.len(),
        2,
        "CR 702.112c: each renown instance triggers separately"
    );
    p13_settle(&mut game, &mut queue, &mut dm);
    let counters = p13_plus(&game, creature);
    assert!(
        counters == 1 || counters == 2,
        "CR 702.112c: only the first renown ability to resolve has an effect, got {counters} counters"
    );
    assert!(game.is_renowned(creature));

    // 702.112b: renowned ends when the permanent leaves the battlefield.
    let stable = game.object(creature).unwrap().stable_id;
    game.move_object_by_effect(creature, Zone::Hand).unwrap();
    let in_hand = game.find_object_by_stable_id(stable).unwrap_or(creature);
    let back = game
        .move_object_by_effect(in_hand, Zone::Battlefield)
        .unwrap();
    let back = game.find_object_by_stable_id(stable).unwrap_or(back);
    assert!(
        !game.is_renowned(back),
        "CR 702.112b: renowned stays only until the permanent leaves the battlefield"
    );
}

// ---------------------------------------------------------------------------
// 702.114 Devoid, 702.115 Ingest
// ---------------------------------------------------------------------------

const P13_MIST_INTRUDER: &str = "Mana cost: {1}{U}\nType: Creature — Eldrazi Drone\nPower/Toughness: 1/2\nDevoid\nFlying\nIngest";

#[test]
fn cr_702_114a_devoid_is_colorless_in_every_zone() {
    let def = p13_compile("Mist Intruder", P13_MIST_INTRUDER);
    let mut game = p13_game(2);
    for zone in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Battlefield,
        Zone::Exile,
    ] {
        let id = p13_put(&mut game, &def, ALICE, zone);
        let colors = game.current_colors(id).unwrap();
        assert!(
            colors.is_empty(),
            "CR 702.114a: devoid card in {zone:?} must be colorless, got {colors:?}"
        );
    }
    // On the stack, via a real cast.
    let id = p13_put(&mut game, &def, ALICE, Zone::Hand);
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Blue, 2)]);
    let mut dm = P13Dm::new(true);
    p13_act(
        &mut game,
        ALICE,
        p13_cast_action(id, Zone::Hand, CastingMethod::Normal),
        &mut dm,
    )
    .unwrap();
    let spell = game.stack.last().unwrap().object_id;
    assert!(
        game.current_colors(spell).unwrap().is_empty(),
        "CR 702.114a: devoid spell is colorless"
    );
}

#[test]
fn cr_702_115a_b_ingest_exiles_top_card_per_instance() {
    for (text, expected) in [
        (P13_MIST_INTRUDER.to_string(), 1usize),
        ("Mana cost: {1}{U}\nType: Creature — Eldrazi Drone\nPower/Toughness: 1/2\nIngest\nIngest".to_string(), 2),
    ] {
        let def = p13_compile("Ingester", &text);
        let mut game = p13_game(2);
        p13_fill_library(&mut game, BOB, 5);
        let top = game.player(BOB).unwrap().library.iter().last().copied();
        let creature = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
        let mut dm = P13Dm::new(true);
        p13_full_unblocked_combat(&mut game, &[(creature, AttackTarget::Player(BOB))], &mut dm);
        let exiled = p13_owned_in(&game, BOB, Zone::Exile);
        assert_eq!(exiled.len(), expected, "CR 702.115a/b: each ingest instance exiles one card from the damaged player's library");
        assert_eq!(game.player(BOB).unwrap().library.len(), 5 - expected);
        assert!(p13_owned_in(&game, ALICE, Zone::Exile).is_empty());
        let _ = top;
    }
}

// ---------------------------------------------------------------------------
// 702.121 Melee
// ---------------------------------------------------------------------------

#[test]
fn cr_702_121a_b_melee_counts_opponents_attacked_and_stacks() {
    for (text, per) in [
        (
            "Mana cost: {1}{W}\nType: Creature — Bird\nPower/Toughness: 1/1\nFlying\nMelee",
            1,
        ),
        (
            "Mana cost: {1}{W}\nType: Creature — Bird\nPower/Toughness: 1/1\nMelee\nMelee",
            2,
        ),
    ] {
        for attack_carol in [false, true] {
            let def = p13_compile("Melee creature", text);
            let mut game = p13_game(3);
            let melee = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
            let other = p13_put(&mut game, &p13_vanilla(2, 2), ALICE, Zone::Battlefield);
            let mut decls = vec![(melee, AttackTarget::Player(BOB))];
            if attack_carol {
                decls.push((other, AttackTarget::Player(CAROL)));
            }
            let mut dm = P13Dm::new(true);
            p13_attack(&mut game, &decls, &mut dm).unwrap();
            let opponents = if attack_carol { 2 } else { 1 };
            assert_eq!(
                game.calculated_power(melee),
                Some(1 + per * opponents),
                "CR 702.121a/b: melee x{per} with {opponents} opponents attacked"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 702.130 Afflict
// ---------------------------------------------------------------------------

#[test]
fn cr_702_130a_b_afflict_drains_defending_player_when_blocked() {
    for (text, loss) in [
        (
            "Mana cost: {2}{B}\nType: Creature — Zombie\nPower/Toughness: 3/3\nAfflict 2",
            2,
        ),
        (
            "Mana cost: {2}{B}\nType: Creature — Zombie\nPower/Toughness: 3/3\nAfflict 2\nAfflict 1",
            3,
        ),
    ] {
        for blocked in [false, true] {
            let def = p13_compile("Afflicter", text);
            let mut game = p13_game(2);
            let attacker = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
            let blocker = p13_put(&mut game, &p13_vanilla(0, 5), BOB, Zone::Battlefield);
            let mut dm = P13Dm::new(true);
            p13_attack(&mut game, &[(attacker, AttackTarget::Player(BOB))], &mut dm).unwrap();
            let decl = if blocked {
                vec![(blocker, attacker)]
            } else {
                vec![]
            };
            p13_block(&mut game, BOB, &decl, &mut dm).unwrap();
            assert_eq!(
                game.player(BOB).unwrap().life,
                20 - if blocked { loss } else { 0 },
                "CR 702.130a/b: afflict life loss when blocked={blocked}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 702.134 Mentor
// ---------------------------------------------------------------------------

#[test]
fn cr_702_134a_b_mentor_targets_lesser_power_attacker_per_instance() {
    for (text, instances) in [
        (
            "Mana cost: {4}{R}\nType: Creature — Minotaur Soldier\nPower/Toughness: 4/2\nHaste\nMentor",
            1u32,
        ),
        (
            "Mana cost: {4}{R}\nType: Creature — Minotaur Soldier\nPower/Toughness: 4/2\nMentor\nMentor",
            2,
        ),
    ] {
        let def = p13_compile("Mentor creature", text);
        let mut game = p13_game(2);
        let mentor = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
        let small = p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Battlefield);
        let equal = p13_put(&mut game, &p13_vanilla(4, 4), ALICE, Zone::Battlefield);
        let mut dm = P13Dm::new(true);
        dm.targets = None;
        p13_attack(
            &mut game,
            &[
                (mentor, AttackTarget::Player(BOB)),
                (small, AttackTarget::Player(BOB)),
                (equal, AttackTarget::Player(BOB)),
            ],
            &mut dm,
        )
        .unwrap();
        assert_eq!(
            p13_plus(&game, equal),
            0,
            "CR 702.134a: equal power is not a legal mentor target"
        );
        assert_eq!(p13_plus(&game, mentor), 0);
        assert_eq!(
            p13_plus(&game, small),
            instances,
            "CR 702.134a/b: each mentor instance puts a counter on the lesser-power attacker"
        );
    }
}

#[test]
fn cr_702_134c_mentors_trigger_fires_when_mentor_resolves() {
    let aegis_like = p13_try_compile(
        "Mentor watcher",
        "Mana cost: {4}{R}\nType: Creature — Minotaur Soldier\nPower/Toughness: 4/2\nMentor\nWhenever this creature mentors a creature, put a shield counter on that creature.",
    );
    let def = aegis_like
        .expect("CR 702.134c: a 'whenever this creature mentors a creature' trigger must compile");
    let mut game = p13_game(2);
    let mentor = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
    let small = p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Battlefield);
    let mut dm = P13Dm::new(true);
    p13_attack(
        &mut game,
        &[
            (mentor, AttackTarget::Player(BOB)),
            (small, AttackTarget::Player(BOB)),
        ],
        &mut dm,
    )
    .unwrap();
    assert_eq!(p13_plus(&game, small), 1);
    assert_eq!(
        game.counter_count(small, CounterType::Shield),
        1,
        "CR 702.134c: the mentors trigger fires when the mentor ability resolves targeting the creature"
    );
}

// ---------------------------------------------------------------------------
// 702.149 Training
// ---------------------------------------------------------------------------

#[test]
fn cr_702_149a_b_c_training_requires_greater_power_attacker_and_trains_trigger() {
    for (text, instances) in [
        (
            "Mana cost: {1}{G}\nType: Creature — Human\nPower/Toughness: 1/1\nTraining\nWhenever this creature trains, you gain 2 life.",
            1u32,
        ),
        (
            "Mana cost: {1}{G}\nType: Creature — Human\nPower/Toughness: 1/1\nTraining\nTraining\nWhenever this creature trains, you gain 2 life.",
            2,
        ),
    ] {
        for partner_power in [1, 3] {
            let def = p13_compile("Trainee", text);
            let mut game = p13_game(2);
            let trainee = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
            let partner = p13_put(
                &mut game,
                &p13_vanilla(partner_power, 3),
                ALICE,
                Zone::Battlefield,
            );
            let mut dm = P13Dm::new(true);
            p13_attack(
                &mut game,
                &[
                    (trainee, AttackTarget::Player(BOB)),
                    (partner, AttackTarget::Player(BOB)),
                ],
                &mut dm,
            )
            .unwrap();
            let trains = if partner_power > 1 { instances } else { 0 };
            assert_eq!(
                p13_plus(&game, trainee),
                trains,
                "CR 702.149a/b: training x{instances}, partner power {partner_power}"
            );
            assert_eq!(
                game.player(ALICE).unwrap().life,
                20 + 2 * trains as i32,
                "CR 702.149c: 'whenever this creature trains' fires per resolving training that adds counters"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 702.147 Decayed
// ---------------------------------------------------------------------------

#[test]
fn cr_702_147a_decayed_cannot_block_and_is_sacrificed_after_attacking() {
    let def = p13_compile(
        "Decayed Zombie",
        "Mana cost: {B}\nType: Creature — Zombie\nPower/Toughness: 2/2\nDecayed",
    );
    let mut game = p13_game(2);
    let attacker = p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Battlefield);
    let decayed_blocker = p13_put(&mut game, &def, BOB, Zone::Battlefield);
    let mut dm = P13Dm::new(true);
    p13_attack(&mut game, &[(attacker, AttackTarget::Player(BOB))], &mut dm).unwrap();
    assert!(
        p13_block(&mut game, BOB, &[(decayed_blocker, attacker)], &mut dm).is_err(),
        "CR 702.147a: a decayed creature can't block"
    );
    // Attacking with decayed: sacrificed at end of combat.
    let mut game = p13_game(2);
    let decayed = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
    p13_full_unblocked_combat(&mut game, &[(decayed, AttackTarget::Player(BOB))], &mut dm);
    assert_eq!(
        game.object(decayed).map(|o| o.zone),
        Some(Zone::Battlefield),
        "decayed creature survives until end of combat"
    );
    p13_end_combat(&mut game, &mut dm);
    assert!(
        p13_named(&game, Zone::Battlefield, "Decayed Zombie").is_empty(),
        "CR 702.147a: an attacking decayed creature is sacrificed at end of combat"
    );
    assert_eq!(p13_named(&game, Zone::Graveyard, "Decayed Zombie").len(), 1);
}

// ---------------------------------------------------------------------------
// 702.116 Myriad
// ---------------------------------------------------------------------------

#[test]
fn cr_702_116a_b_myriad_creates_attacking_copies_for_other_opponents() {
    for (text, instances) in [
        (
            "Mana cost: {2}{R}\nType: Creature — Goblin\nPower/Toughness: 2/2\nMyriad",
            1usize,
        ),
        (
            "Mana cost: {2}{R}\nType: Creature — Goblin\nPower/Toughness: 2/2\nMyriad\nMyriad",
            2,
        ),
    ] {
        let def = p13_compile("Myriad Goblin", text);
        let mut game = p13_game(4);
        let creature = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
        let mut dm = P13Dm::new(true);
        p13_attack(&mut game, &[(creature, AttackTarget::Player(BOB))], &mut dm).unwrap();
        let combat = game.combat.clone().unwrap();
        let tokens: Vec<ObjectId> = p13_named(&game, Zone::Battlefield, "Myriad Goblin")
            .into_iter()
            .filter(|id| *id != creature)
            .collect();
        assert_eq!(
            tokens.len(),
            2 * instances,
            "CR 702.116a/b: one copy per other opponent per myriad instance"
        );
        let mut defenders: Vec<PlayerId> = Vec::new();
        for t in &tokens {
            assert!(
                game.is_tapped(*t),
                "CR 702.116a: myriad tokens enter tapped"
            );
            let info = combat
                .attackers
                .iter()
                .find(|a| a.creature == *t)
                .expect("CR 702.116a: myriad token is attacking");
            match info.target {
                AttackTarget::Player(p) => defenders.push(p),
                ref other => panic!("unexpected attack target {other:?}"),
            }
        }
        assert!(
            !defenders.contains(&BOB),
            "CR 702.116a: copies attack opponents other than the defending player"
        );
        assert_eq!(defenders.iter().filter(|p| **p == CAROL).count(), instances);
        p13_block(&mut game, BOB, &[], &mut dm).unwrap();
        p13_damage(&mut game, &mut dm);
        p13_end_combat(&mut game, &mut dm);
        let remaining = p13_named(&game, Zone::Battlefield, "Myriad Goblin");
        assert_eq!(
            remaining,
            vec![creature],
            "CR 702.116a: tokens are exiled at end of combat"
        );
    }
}

// ---------------------------------------------------------------------------
// 702.123 Fabricate, 702.135 Afterlife, 702.136 Riot
// ---------------------------------------------------------------------------

fn p13_cast_and_resolve(
    game: &mut GameState,
    def: &CardDefinition,
    mana: &[(ManaSymbol, u32)],
    dm: &mut P13Dm,
) -> ObjectId {
    let card = p13_put(game, def, ALICE, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    p13_mana(game, ALICE, mana);
    p13_act(
        game,
        ALICE,
        p13_cast_action(card, Zone::Hand, CastingMethod::Normal),
        dm,
    )
    .unwrap();
    let mut queue = TriggerQueue::new();
    p13_settle(game, &mut queue, dm);
    game.find_object_by_stable_id(stable).unwrap()
}

#[test]
fn cr_702_123a_b_fabricate_counters_or_servos_per_instance() {
    for (text, n, instances) in [
        (
            "Mana cost: {2}{B}\nType: Creature — Aetherborn Artificer\nPower/Toughness: 0/1\nFabricate 2",
            2u32,
            1u32,
        ),
        (
            "Mana cost: {2}{B}\nType: Creature — Aetherborn Artificer\nPower/Toughness: 0/1\nFabricate 1\nFabricate 1",
            1,
            2,
        ),
    ] {
        for counters in [true, false] {
            let def = p13_compile("Fabricator", text);
            let mut game = p13_game(2);
            let mut dm = P13Dm::new(counters).prefer(if counters { "counter" } else { "servo" });
            let id = p13_cast_and_resolve(&mut game, &def, &[(ManaSymbol::Black, 3)], &mut dm);
            let servos: Vec<ObjectId> = p13_controlled_battlefield(&game, ALICE)
                .into_iter()
                .filter(|s| {
                    game.object(*s)
                        .unwrap()
                        .subtypes
                        .contains(&ironsmith::Subtype::Servo)
                })
                .collect();
            if counters {
                assert_eq!(
                    p13_plus(&game, id),
                    n * instances,
                    "CR 702.123a/b: counters chosen; log={:?}",
                    dm.log
                );
                assert!(servos.is_empty());
            } else {
                assert_eq!(p13_plus(&game, id), 0, "log={:?}", dm.log);
                assert_eq!(
                    servos.len() as u32,
                    n * instances,
                    "CR 702.123a/b: N Servos per instance; log={:?}",
                    dm.log
                );
                for s in servos {
                    assert_eq!(game.calculated_power(s), Some(1));
                    assert!(
                        game.current_colors(s).unwrap().is_empty(),
                        "CR 702.123a: Servos are colorless"
                    );
                    assert!(
                        game.object(s)
                            .unwrap()
                            .card_types
                            .contains(&ironsmith::CardType::Artifact)
                    );
                }
            }
        }
    }
}

#[test]
fn cr_702_135a_b_afterlife_creates_flying_spirits_per_instance() {
    for (text, expected) in [
        (
            "Mana cost: {W}{B}\nType: Creature — Human Cleric\nPower/Toughness: 2/1\nVigilance\nAfterlife 1",
            1usize,
        ),
        (
            "Mana cost: {W}{B}\nType: Creature — Human Cleric\nPower/Toughness: 2/1\nAfterlife 1\nAfterlife 2",
            3,
        ),
    ] {
        let def = p13_compile("Oligarch", text);
        let mut game = p13_game(2);
        let id = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
        let mut dm = P13Dm::new(true);
        let mut ctx = EffectContext::new_default(id, ALICE);
        use ironsmith::EffectExecutor;
        ironsmith::effects::DestroyEffect::with_spec(ironsmith::ChooseSpec::SpecificObject(id))
            .execute(&mut game, &mut ctx)
            .unwrap();
        let mut queue = TriggerQueue::new();
        p13_settle(&mut game, &mut queue, &mut dm);
        let spirits: Vec<ObjectId> = p13_controlled_battlefield(&game, ALICE);
        assert_eq!(spirits.len(), expected, "CR 702.135a/b: afterlife spirits");
        for s in spirits {
            let o = game.object(s).unwrap();
            assert!(o.subtypes.contains(&ironsmith::Subtype::Spirit));
            let c = game.current_colors(s).unwrap();
            assert!(
                c.contains(ironsmith::Color::White) && c.contains(ironsmith::Color::Black),
                "CR 702.135a: white and black"
            );
            assert!(
                game.object_has_ability(s, &ironsmith::StaticAbility::flying()),
                "CR 702.135a: flying"
            );
            assert_eq!(game.calculated_power(s), Some(1));
        }
    }
}

#[test]
fn cr_702_136a_b_riot_counter_or_haste_each_instance_separately() {
    let single = "Mana cost: {1}{R}{G}\nType: Creature — Ogre Warrior\nPower/Toughness: 3/3\nRiot";
    for counter in [true, false] {
        let def = p13_compile("Rioter", single);
        let mut game = p13_game(2);
        let mut dm = P13Dm::new(counter).prefer(if counter { "counter" } else { "haste" });
        let id = p13_cast_and_resolve(
            &mut game,
            &def,
            &[(ManaSymbol::Red, 1), (ManaSymbol::Green, 2)],
            &mut dm,
        );
        let haste = game.object_has_ability(id, &ironsmith::StaticAbility::haste());
        assert_eq!(
            p13_plus(&game, id),
            counter as u32,
            "CR 702.136a: riot counter choice; log={:?}",
            dm.log
        );
        assert_eq!(
            haste, !counter,
            "CR 702.136a: riot haste iff no counter; log={:?}",
            dm.log
        );
    }
    let double =
        "Mana cost: {1}{R}{G}\nType: Creature — Ogre Warrior\nPower/Toughness: 3/3\nRiot\nRiot";
    let def = p13_compile("Double Rioter", double);
    let mut game = p13_game(2);
    let mut dm = P13Dm::new(true).prefer("counter");
    let id = p13_cast_and_resolve(
        &mut game,
        &def,
        &[(ManaSymbol::Red, 1), (ManaSymbol::Green, 2)],
        &mut dm,
    );
    assert_eq!(
        p13_plus(&game, id),
        2,
        "CR 702.136b: each riot instance works separately; log={:?}",
        dm.log
    );
}

// ---------------------------------------------------------------------------
// Casting-cost keywords
// ---------------------------------------------------------------------------

fn p13_forest() -> CardDefinition {
    p13_compile("Forest", "Type: Basic Land — Forest")
}

fn p13_pool_total(game: &GameState, player: PlayerId) -> u32 {
    let p = &game.player(player).unwrap().mana_pool;
    p.white + p.blue + p.black + p.red + p.green + p.colorless
}

fn p13_offers(
    game: &GameState,
    player: PlayerId,
    card: ObjectId,
    pred: impl Fn(&CastingMethod) -> bool,
) -> bool {
    p13_legal(game, player).iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card && pred(casting_method)))
}

fn p13_cast_free_spell(game: &mut GameState, player: PlayerId, text: &str) {
    let def = p13_compile("Free spell", text);
    let id = p13_put(game, &def, player, Zone::Hand);
    let mut dm = P13Dm::new(true);
    p13_act(
        game,
        player,
        p13_cast_action(id, Zone::Hand, CastingMethod::Normal),
        &mut dm,
    )
    .unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(game, &mut q, &mut dm);
}

const P13_BUSHWHACKER: &str = "Mana cost: {2}{R}\nType: Creature — Goblin Warrior Ally\nPower/Toughness: 2/1\nSurge {1}{R}\nHaste\nWhen this creature enters, if its surge cost was paid, other creatures you control get +1/+0 and gain haste until end of turn.";

#[test]
fn cr_702_117a_surge_requires_another_spell_this_turn() {
    let def = p13_compile("Reckless Bushwhacker", P13_BUSHWHACKER);
    let mut game = p13_game(2);
    let ally = p13_put(&mut game, &p13_vanilla(2, 2), ALICE, Zone::Battlefield);
    let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Red, 2)]);
    assert!(
        !p13_offers(&game, ALICE, card, |m| matches!(
            m,
            CastingMethod::Alternative(_)
        )),
        "CR 702.117a: surge is unavailable before another spell was cast this turn"
    );
    p13_cast_free_spell(
        &mut game,
        ALICE,
        "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",
    );
    assert!(
        p13_offers(&game, ALICE, card, |m| matches!(
            m,
            CastingMethod::Alternative(_)
        )),
        "CR 702.117a: surge is available after casting another spell this turn"
    );
    let mut dm = P13Dm::new(true);
    p13_act(
        &mut game,
        ALICE,
        p13_cast_action(card, Zone::Hand, CastingMethod::Alternative(0)),
        &mut dm,
    )
    .unwrap();
    assert_eq!(
        p13_pool_total(&game, ALICE),
        0,
        "CR 702.117a: surge cost {{1}}{{R}} was paid instead of {{2}}{{R}}"
    );
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert_eq!(
        game.calculated_power(ally),
        Some(3),
        "CR 702.117a: 'if its surge cost was paid' sees the surge payment"
    );
}

#[test]
fn cr_702_119a_c_emerge_sacrifices_and_reduces_by_mana_value() {
    let gryff = p13_compile(
        "Wretched Gryff",
        "Mana cost: {7}{U}\nType: Creature — Eldrazi Hippogriff\nPower/Toughness: 3/4\nEmerge {5}{U}\nWhen you cast this spell, draw a card.\nFlying",
    );
    let three = p13_compile(
        "Three drop",
        "Mana cost: {2}{G}\nType: Creature — Bear\nPower/Toughness: 3/3",
    );
    let mut game = p13_game(2);
    p13_fill_library(&mut game, ALICE, 3);
    let small = p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Battlefield);
    let sac = p13_put(&mut game, &three, ALICE, Zone::Battlefield);
    let sac_stable = game.object(sac).unwrap().stable_id;
    let card = p13_put(&mut game, &gryff, ALICE, Zone::Hand);
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 2)],
    );
    let mut dm = P13Dm::new(true).prefer("three drop");
    dm.objects = Some(vec![sac]);
    let hand_before = game.player(ALICE).unwrap().hand.len();
    let result = p13_act(
        &mut game,
        ALICE,
        p13_cast_action(card, Zone::Hand, CastingMethod::Alternative(0)),
        &mut dm,
    );
    assert!(
        result.is_ok(),
        "CR 702.119a: emerge with a mana value 3 sacrifice costs {{2}}{{U}}: {result:?} log={:?}",
        dm.log
    );
    assert_eq!(p13_pool_total(&game, ALICE), 0);
    let moved = game.find_object_by_stable_id(sac_stable);
    assert!(
        moved.is_none() || game.object(moved.unwrap()).unwrap().zone != Zone::Battlefield,
        "CR 702.119c: the chosen creature is sacrificed while paying the total cost"
    );
    assert_eq!(game.object(small).unwrap().zone, Zone::Battlefield);
    assert_eq!(p13_owned_in(&game, ALICE, Zone::Graveyard).len(), 1);
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert_eq!(
        game.player(ALICE).unwrap().hand.len(),
        hand_before,
        "cast trigger drew a card (Gryff left hand, one card drawn)"
    );
    assert_eq!(
        p13_named(&game, Zone::Battlefield, "Wretched Gryff").len(),
        1
    );
}

#[test]
fn cr_702_119b_emerge_from_quality_restricts_sacrifice() {
    let crab = p13_compile(
        "Crabomination",
        "Mana cost: {4}{B}{B}\nType: Creature — Crab Demon\nPower/Toughness: 5/5\nEmerge from artifact {5}{B}{B}",
    );
    let artifact = p13_compile("Two artifact", "Mana cost: {2}\nType: Artifact");
    for with_artifact in [false, true] {
        let mut game = p13_game(2);
        let creature = p13_put(
            &mut game,
            &p13_compile(
                "Big",
                "Mana cost: {4}\nType: Creature — Bear\nPower/Toughness: 4/4",
            ),
            ALICE,
            Zone::Battlefield,
        );
        let art = with_artifact.then(|| p13_put(&mut game, &artifact, ALICE, Zone::Battlefield));
        let card = p13_put(&mut game, &crab, ALICE, Zone::Hand);
        p13_mana(
            &mut game,
            ALICE,
            &[(ManaSymbol::Black, 2), (ManaSymbol::Colorless, 3)],
        );
        let mut dm = P13Dm::new(true);
        dm.objects = art.map(|a| vec![a]);
        let result = p13_act(
            &mut game,
            ALICE,
            p13_cast_action(card, Zone::Hand, CastingMethod::Alternative(0)),
            &mut dm,
        );
        assert_eq!(
            game.object(creature).map(|o| o.zone),
            Some(Zone::Battlefield),
            "CR 702.119b: a nonartifact creature can't be sacrificed for emerge from artifact"
        );
        if with_artifact {
            assert!(
                result.is_ok(),
                "CR 702.119b: emerge from artifact sacrificing a mana value 2 artifact costs {{3}}{{B}}{{B}}: {result:?}"
            );
            assert_eq!(p13_pool_total(&game, ALICE), 0);
            assert!(
                game.object(art.unwrap())
                    .map_or(true, |o| o.zone != Zone::Battlefield)
            );
        } else {
            assert!(
                result.is_err()
                    || !game.stack.iter().any(|e| game
                        .object(e.object_id)
                        .is_some_and(|o| o.name == "Crabomination")),
                "CR 702.119b: emerge from artifact is impossible without an artifact"
            );
        }
    }
}

#[test]
fn cr_702_120a_escalate_charges_per_extra_mode() {
    let text = "Mana cost: {1}{B}\nType: Sorcery\nEscalate—Discard a card.\nChoose one or more —\n• Target opponent reveals their hand. You choose an instant or sorcery card from it. That player discards that card.\n• Target creature gets -2/-2 until end of turn.\n• Target opponent loses 2 life and you gain 2 life.";
    for two_modes in [false, true] {
        let def = p13_compile("Collective Brutality", text);
        let mut game = p13_game(2);
        let victim = p13_put(&mut game, &p13_vanilla(2, 2), BOB, Zone::Battlefield);
        for _ in 0..2 {
            p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Hand);
        }
        let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
        p13_mana(&mut game, ALICE, &[(ManaSymbol::Black, 2)]);
        let mut dm = P13Dm::new(true).prefer("loses 2");
        if two_modes {
            dm = dm.prefer("-2/-2");
        }
        dm.targets = None;
        p13_act(
            &mut game,
            ALICE,
            p13_cast_action(card, Zone::Hand, CastingMethod::Normal),
            &mut dm,
        )
        .unwrap();
        let mut q = TriggerQueue::new();
        p13_settle(&mut game, &mut q, &mut dm);
        assert_eq!(game.player(BOB).unwrap().life, 18, "log={:?}", dm.log);
        assert_eq!(
            game.player(ALICE).unwrap().hand.len(),
            if two_modes { 1 } else { 2 },
            "CR 702.120a: escalate discards one card per mode beyond the first; log={:?}",
            dm.log
        );
        assert_eq!(
            game.object(victim).map(|o| o.zone) == Some(Zone::Battlefield),
            !two_modes
        );
    }
}

#[test]
fn cr_702_125a_b_c_undaunted_reduces_per_current_opponent() {
    // (players, left, instances, mana available, castable)
    for (players, leave, instances, generic, castable) in [
        (4usize, false, 1usize, 3u32, true),
        (4, false, 1, 2, false),
        (4, true, 1, 3, false),
        (4, true, 1, 4, true),
        (4, false, 2, 0, true),
    ] {
        let text = format!(
            "Mana cost: {{6}}{{W}}\nType: Sorcery\n{}Destroy all creatures.",
            "Undaunted\n".repeat(instances)
        );
        let def = p13_compile("Sublime Exhalation", &text);
        let mut game = p13_game(players);
        if leave {
            game.leave_game(PlayerId(3)).unwrap();
        }
        let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
        p13_mana(
            &mut game,
            ALICE,
            &[(ManaSymbol::White, 1), (ManaSymbol::Colorless, generic)],
        );
        let mut dm = P13Dm::new(true);
        let result = p13_act(
            &mut game,
            ALICE,
            p13_cast_action(card, Zone::Hand, CastingMethod::Normal),
            &mut dm,
        );
        let on_stack = game.stack.iter().any(|e| {
            e.object_id != card
                && game
                    .object(e.object_id)
                    .is_some_and(|o| o.name == "Sublime Exhalation")
        }) || game.stack.iter().any(|e| {
            game.object(e.object_id)
                .is_some_and(|o| o.name == "Sublime Exhalation")
        });
        assert_eq!(
            on_stack, castable,
            "CR 702.125a/b/c: players={players} left={leave} instances={instances} generic={generic}: result={result:?}"
        );
    }
}

#[test]
fn cr_702_126a_b_c_improvise_taps_artifacts_for_generic_after_total_cost() {
    for (instances, artifacts, tax) in [(1usize, 3usize, false), (2, 3, false), (1, 4, true)] {
        let text = format!(
            "Mana cost: {{3}}{{U}}{{U}}\nType: Sorcery\n{}Draw three cards.",
            "Improvise\n".repeat(instances)
        );
        let def = p13_compile("Reverse Engineer", &text);
        let art = p13_compile("Trinket", "Mana cost: {1}\nType: Artifact");
        let mut game = p13_game(2);
        p13_fill_library(&mut game, ALICE, 5);
        if tax {
            let thalia = p13_compile(
                "Taxer",
                "Mana cost: {1}{W}\nType: Creature — Human\nPower/Toughness: 2/1\nNoncreature spells cost {1} more to cast.",
            );
            p13_put(&mut game, &thalia, BOB, Zone::Battlefield);
        }
        let arts: Vec<ObjectId> = (0..artifacts)
            .map(|_| p13_put(&mut game, &art, ALICE, Zone::Battlefield))
            .collect();
        let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
        p13_mana(&mut game, ALICE, &[(ManaSymbol::Blue, 2)]);
        let mut dm = P13Dm::new(true);
        let result = p13_act(
            &mut game,
            ALICE,
            p13_cast_action(card, Zone::Hand, CastingMethod::Normal),
            &mut dm,
        );
        assert!(
            result.is_ok(),
            "CR 702.126a/b/c: improvise pays generic (instances={instances}, tax={tax}) by tapping artifacts: {result:?}"
        );
        let tapped = arts.iter().filter(|a| game.is_tapped(**a)).count();
        assert_eq!(
            tapped,
            if tax { 4 } else { 3 },
            "CR 702.126a/b/c: one artifact per generic mana of the total cost (redundant instances)"
        );
    }
}

#[test]
fn cr_702_132a_assist_lets_another_player_pay_generic() {
    let def = p13_compile(
        "Huddle Up",
        "Mana cost: {2}{U}\nType: Sorcery\nAssist\nTwo target players each draw a card.",
    );
    let mut game = p13_game(2);
    p13_fill_library(&mut game, ALICE, 2);
    p13_fill_library(&mut game, BOB, 2);
    let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Blue, 1)]);
    p13_mana(&mut game, BOB, &[(ManaSymbol::Colorless, 2)]);
    let mut dm = P13Dm::new(true).prefer("bob");
    let result = p13_act(
        &mut game,
        ALICE,
        p13_cast_action(card, Zone::Hand, CastingMethod::Normal),
        &mut dm,
    );
    assert!(
        result.is_ok(),
        "CR 702.132a: assisted cast must succeed: {result:?} log={:?}",
        dm.log
    );
    assert_eq!(p13_pool_total(&game, ALICE), 0);
    assert_eq!(
        p13_pool_total(&game, BOB),
        0,
        "CR 702.132a: the assisting player paid the generic mana"
    );
    assert!(game.stack.iter().any(|e| {
        game.object(e.object_id)
            .is_some_and(|o| o.name == "Huddle Up")
    }));
}

fn p13_split(
    left_name: &str,
    left: &str,
    right_name: &str,
    right: &str,
) -> (CardDefinition, CardDefinition) {
    let mut l = p13_compile(left_name, left);
    let mut r = p13_compile(right_name, right);
    l.card.other_face = Some(r.card.id);
    l.card.other_face_name = Some(right_name.to_string());
    l.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
    r.card.other_face = Some(l.card.id);
    r.card.other_face_name = Some(left_name.to_string());
    r.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
    (l, r)
}

#[test]
fn cr_702_127a_aftermath_half_only_from_graveyard_then_exiled() {
    let (cut, ribbons) = p13_split(
        "Cut",
        "Mana cost: {1}{R}\nType: Sorcery\nCut deals 4 damage to target creature.",
        "Ribbons",
        "Mana cost: {X}{B}{B}\nType: Sorcery\nAftermath\nEach opponent loses X life.",
    );
    let mut game = p13_game(2);
    game.register_linked_face_definition(&cut);
    game.register_linked_face_definition(&ribbons);
    p13_put(&mut game, &p13_vanilla(2, 2), BOB, Zone::Battlefield);
    let in_hand = p13_put(&mut game, &cut, ALICE, Zone::Hand);
    p13_mana(
        &mut game,
        ALICE,
        &[
            (ManaSymbol::Black, 2),
            (ManaSymbol::Red, 2),
            (ManaSymbol::Colorless, 2),
        ],
    );
    let hand_methods: Vec<CastingMethod> = p13_legal(&game, ALICE)
        .into_iter()
        .filter_map(|a| match a {
            LegalAction::CastSpell {
                spell_id,
                casting_method,
                ..
            } if spell_id == in_hand => Some(casting_method),
            _ => None,
        })
        .collect();
    assert!(
        hand_methods.contains(&CastingMethod::Normal),
        "Cut is castable from hand"
    );
    assert!(
        hand_methods.iter().all(|m| *m == CastingMethod::Normal),
        "CR 702.127a: the aftermath half can't be cast from hand; offered {hand_methods:?}"
    );
    let mut game2 = game.clone();
    let gy = game2
        .move_object_by_effect(in_hand, Zone::Graveyard)
        .unwrap();
    let gy = game2
        .objects_in_zone(Zone::Graveyard)
        .into_iter()
        .find(|id| game2.object(*id).is_some_and(|o| o.owner == ALICE))
        .unwrap_or(gy);
    let gy_actions: Vec<LegalAction> = p13_legal(&game2, ALICE)
        .into_iter()
        .filter(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == gy))
        .collect();
    assert!(
        !gy_actions.iter().any(|a| matches!(
            a,
            LegalAction::CastSpell {
                casting_method: CastingMethod::Normal,
                ..
            }
        )),
        "CR 702.127a: the non-aftermath half isn't castable from the graveyard: {gy_actions:?}"
    );
    assert_eq!(
        gy_actions.len(),
        1,
        "CR 702.127a: the aftermath half is castable from the graveyard: {gy_actions:?}"
    );
    let mut dm = P13Dm::new(true);
    dm.number = Some(2);
    p13_act(&mut game2, ALICE, gy_actions[0].clone(), &mut dm).unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game2, &mut q, &mut dm);
    assert_eq!(
        game2.player(BOB).unwrap().life,
        18,
        "Ribbons X=2 resolved; log={:?}",
        dm.log
    );
    assert_eq!(
        p13_owned_in(&game2, ALICE, Zone::Exile).len(),
        1,
        "CR 702.127a: aftermath spell is exiled instead of going anywhere else"
    );
    assert!(p13_owned_in(&game2, ALICE, Zone::Graveyard).is_empty());
}

#[test]
fn cr_702_133a_jump_start_from_graveyard_discards_and_exiles() {
    let def = p13_compile(
        "Chemister's Insight",
        "Mana cost: {3}{U}\nType: Instant\nDraw two cards.\nJump-start",
    );
    let mut game = p13_game(2);
    p13_fill_library(&mut game, ALICE, 4);
    let card = p13_put(&mut game, &def, ALICE, Zone::Graveyard);
    let fodder = p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Hand);
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 3)],
    );
    let actions: Vec<LegalAction> = p13_legal(&game, ALICE)
        .into_iter()
        .filter(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
        .collect();
    assert_eq!(
        actions.len(),
        1,
        "CR 702.133a: jump-start allows casting from the graveyard: {actions:?}"
    );
    let mut dm = P13Dm::new(true);
    dm.objects = Some(vec![fodder]);
    p13_act(&mut game, ALICE, actions[0].clone(), &mut dm).unwrap();
    assert!(
        game.object(fodder).map_or(true, |o| o.zone != Zone::Hand),
        "CR 702.133a: discarding a card is an additional cost"
    );
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert_eq!(game.player(ALICE).unwrap().hand.len(), 2);
    assert_eq!(
        p13_named(&game, Zone::Exile, "Chemister's Insight").len(),
        1,
        "CR 702.133a: a jump-started spell is exiled after leaving the stack"
    );
    // Without a card to discard, jump-start can't be used.
    let mut game = p13_game(2);
    let card = p13_put(&mut game, &def, ALICE, Zone::Graveyard);
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 3)],
    );
    assert!(
        !p13_offers(&game, ALICE, card, |_| true),
        "CR 702.133a: jump-start requires discarding a card"
    );
}

#[test]
fn cr_702_137a_spectacle_requires_opponent_life_loss() {
    let def = p13_compile(
        "Skewer the Critics",
        "Mana cost: {2}{R}\nType: Sorcery\nSpectacle {R}\nSkewer the Critics deals 3 damage to any target.",
    );
    let mut game = p13_game(2);
    let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Red, 1)]);
    assert!(
        !p13_offers(&game, ALICE, card, |m| matches!(
            m,
            CastingMethod::Alternative(_)
        )),
        "CR 702.137a: spectacle needs an opponent to have lost life this turn"
    );
    p13_cast_free_spell(
        &mut game,
        ALICE,
        "Mana cost: {0}\nType: Sorcery\nEach opponent loses 1 life.",
    );
    assert_eq!(game.player(BOB).unwrap().life, 19);
    assert!(
        p13_offers(&game, ALICE, card, |m| matches!(
            m,
            CastingMethod::Alternative(_)
        )),
        "CR 702.137a: spectacle available after an opponent lost life"
    );
    let mut dm = P13Dm::new(true);
    dm.targets = Some(vec![Target::Player(BOB)]);
    p13_act(
        &mut game,
        ALICE,
        p13_cast_action(card, Zone::Hand, CastingMethod::Alternative(0)),
        &mut dm,
    )
    .unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert_eq!(game.player(BOB).unwrap().life, 16);
}

#[test]
fn cr_702_138a_b_c_escape_from_graveyard_with_counter() {
    let ox = p13_compile(
        "Ox of Agonas",
        "Mana cost: {3}{R}{R}\nType: Creature — Ox\nPower/Toughness: 4/2\nWhen this creature enters, discard your hand, then draw three cards.\nEscape—{R}{R}, Exile eight other cards from your graveyard.\nThis creature escapes with a +1/+1 counter on it.",
    );
    for (others, escape) in [(7usize, false), (8, true)] {
        let mut game = p13_game(2);
        p13_fill_library(&mut game, ALICE, 5);
        let card = p13_put(&mut game, &ox, ALICE, Zone::Graveyard);
        let stable = game.object(card).unwrap().stable_id;
        for _ in 0..others {
            p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Graveyard);
        }
        p13_mana(&mut game, ALICE, &[(ManaSymbol::Red, 2)]);
        let actions: Vec<LegalAction> = p13_legal(&game, ALICE)
            .into_iter()
            .filter(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
            .collect();
        assert_eq!(
            !actions.is_empty(),
            escape,
            "CR 702.138a: escape needs eight other graveyard cards (have {others})"
        );
        if !escape {
            continue;
        }
        let mut dm = P13Dm::new(true);
        p13_act(&mut game, ALICE, actions[0].clone(), &mut dm).unwrap();
        assert_eq!(
            p13_owned_in(&game, ALICE, Zone::Exile).len(),
            8,
            "CR 702.138a: exile eight other cards as part of the escape cost"
        );
        let mut q = TriggerQueue::new();
        p13_settle(&mut game, &mut q, &mut dm);
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            p13_plus(&game, permanent),
            1,
            "CR 702.138b/c: an escaped permanent enters with its escape counters"
        );
        assert_eq!(game.player(ALICE).unwrap().hand.len(), 3);
    }
    // Normal cast: no counter.
    let mut game = p13_game(2);
    p13_fill_library(&mut game, ALICE, 5);
    let mut dm = P13Dm::new(true);
    let id = p13_cast_and_resolve(
        &mut game,
        &ox,
        &[(ManaSymbol::Red, 2), (ManaSymbol::Colorless, 3)],
        &mut dm,
    );
    assert_eq!(
        p13_plus(&game, id),
        0,
        "CR 702.138c: only an escaped permanent gets the counter"
    );
}

#[test]
fn cr_702_138d_escapes_with_ability_grants_it_only_when_escaped() {
    let robber = p13_try_compile(
        "Skyway Robber",
        "Mana cost: {3}{U}\nType: Creature — Bird Rogue\nPower/Toughness: 3/3\nFlying\nEscape—{3}{U}, Exile five other cards from your graveyard.\nThis creature escapes with \"Whenever this creature deals combat damage to a player, you may cast an artifact, instant, or sorcery spell from among cards exiled with this creature without paying its mana cost.\"",
    );
    let def = robber.unwrap_or_else(|e| {
        panic!("CR 702.138d: 'escapes with [ability]' (Skyway Robber) must be supported: {e}")
    });
    let mut game = p13_game(2);
    let card = p13_put(&mut game, &def, ALICE, Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    for _ in 0..5 {
        p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Graveyard);
    }
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 3)],
    );
    let actions: Vec<LegalAction> = p13_legal(&game, ALICE)
        .into_iter()
        .filter(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
        .collect();
    assert!(!actions.is_empty(), "CR 702.138a: Skyway Robber can escape");
    let mut dm = P13Dm::new(true);
    p13_act(&mut game, ALICE, actions[0].clone(), &mut dm).unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    let escaped_triggers = game
        .current_characteristics(permanent)
        .unwrap()
        .abilities
        .iter()
        .filter(|a| matches!(a.kind, ironsmith::AbilityKind::Triggered(_)))
        .count();
    // Control: the same card cast normally from hand.
    let mut game2 = p13_game(2);
    let mut dm2 = P13Dm::new(true);
    let normal = p13_cast_and_resolve(
        &mut game2,
        &def,
        &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 3)],
        &mut dm2,
    );
    let normal_triggers = game2
        .current_characteristics(normal)
        .unwrap()
        .abilities
        .iter()
        .filter(|a| matches!(a.kind, ironsmith::AbilityKind::Triggered(_)))
        .count();
    assert_eq!(
        (normal_triggers, escaped_triggers),
        (0, 1),
        "CR 702.138d: 'escapes with [ability]' gives the ability only to a permanent that escaped (normal, escaped)"
    );
}

#[test]
fn cr_702_148a_b_cleave_removes_bracketed_text() {
    let def = p13_compile(
        "Dig Up",
        "Mana cost: {G}\nType: Sorcery\nCleave {1}{B}{B}{G}\nSearch your library for a [basic land] card, [reveal it,] put it into your hand, then shuffle.",
    );
    for cleave in [false, true] {
        let mut game = p13_game(2);
        let nonland = p13_put(&mut game, &p13_vanilla(5, 5), ALICE, Zone::Library);
        let nonland_stable = game.object(nonland).unwrap().stable_id;
        let forest = p13_put(&mut game, &p13_forest(), ALICE, Zone::Library);
        let forest_stable = game.object(forest).unwrap().stable_id;
        let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
        p13_mana(
            &mut game,
            ALICE,
            &[
                (ManaSymbol::Green, 1),
                (ManaSymbol::Black, 2),
                (ManaSymbol::Colorless, 1),
            ],
        );
        let mut dm = P13Dm::new(true);
        dm.objects = Some(vec![nonland]);
        let method = if cleave {
            CastingMethod::Alternative(0)
        } else {
            CastingMethod::Normal
        };
        p13_act(
            &mut game,
            ALICE,
            p13_cast_action(card, Zone::Hand, method),
            &mut dm,
        )
        .unwrap();
        let mut q = TriggerQueue::new();
        p13_settle(&mut game, &mut q, &mut dm);
        let in_hand = |stable| {
            game.find_object_by_stable_id(stable)
                .and_then(|id| game.object(id))
                .is_some_and(|o| o.zone == Zone::Hand)
        };
        if cleave {
            assert!(
                in_hand(nonland_stable),
                "CR 702.148a/b: cleave removes [basic land], so any card can be found; log={:?}",
                dm.log
            );
        } else {
            assert!(
                !in_hand(nonland_stable),
                "CR 702.148a: without cleave only a basic land can be found"
            );
            assert!(
                in_hand(forest_stable),
                "normal Dig Up finds the basic land; log={:?}",
                dm.log
            );
        }
    }
}

#[test]
fn cr_702_113a_b_awaken_adds_land_target_only_when_paid() {
    let def = p13_compile(
        "Ruinous Path",
        "Mana cost: {1}{B}{B}\nType: Sorcery\nDestroy target creature or planeswalker.\nAwaken 4—{5}{B}{B}",
    );
    for awaken in [false, true] {
        let mut game = p13_game(2);
        let victim = p13_put(&mut game, &p13_vanilla(2, 2), BOB, Zone::Battlefield);
        let land = p13_put(&mut game, &p13_forest(), ALICE, Zone::Battlefield);
        let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
        p13_mana(
            &mut game,
            ALICE,
            &[(ManaSymbol::Black, 2), (ManaSymbol::Colorless, 5)],
        );
        let mut dm = P13Dm::new(true);
        let method = if awaken {
            CastingMethod::Alternative(0)
        } else {
            CastingMethod::Normal
        };
        p13_act(
            &mut game,
            ALICE,
            p13_cast_action(card, Zone::Hand, method),
            &mut dm,
        )
        .unwrap();
        assert_eq!(
            p13_pool_total(&game, ALICE),
            if awaken { 0 } else { 4 },
            "CR 702.113a: awaken cost replaces the mana cost"
        );
        let spell = game.stack.last().unwrap().clone();
        let land_targeted = spell.targets.iter().any(|t| *t == Target::Object(land));
        assert_eq!(
            land_targeted, awaken,
            "CR 702.113b: the awaken land target is chosen only when awaken is paid; targets={:?}",
            spell.targets
        );
        let mut q = TriggerQueue::new();
        p13_settle(&mut game, &mut q, &mut dm);
        assert_ne!(game.object(victim).map(|o| o.zone), Some(Zone::Battlefield));
        assert_eq!(
            game.current_is_creature(land),
            awaken,
            "CR 702.113a: awakened land becomes a creature"
        );
        if awaken {
            assert_eq!(p13_plus(&game, land), 4);
            assert_eq!(
                game.calculated_power(land),
                Some(4),
                "CR 702.113a: 0/0 Elemental with four +1/+1 counters"
            );
            assert!(
                game.object_has_ability(land, &ironsmith::StaticAbility::haste()),
                "CR 702.113a: awakened land has haste"
            );
            assert!(
                game.object(land)
                    .unwrap()
                    .subtypes
                    .contains(&ironsmith::Subtype::Elemental)
                    || game
                        .current_characteristics(land)
                        .unwrap()
                        .subtypes
                        .contains(&ironsmith::Subtype::Elemental)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Activated keywords: crew, embalm, eternalize, encore, boast
// ---------------------------------------------------------------------------

fn p13_activations(game: &GameState, player: PlayerId, source: ObjectId) -> Vec<LegalAction> {
    p13_legal(game, player)
        .into_iter()
        .filter(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
        .collect()
}

const P13_COPTER: &str =
    "Mana cost: {2}\nType: Artifact — Vehicle\nPower/Toughness: 3/3\nFlying\nCrew 1";

#[test]
fn cr_702_122a_crew_taps_other_creatures_with_enough_power() {
    let copter = p13_compile("Smuggler's Copter", P13_COPTER);
    for power in [0, 1, 3] {
        let mut game = p13_game(2);
        let vehicle = p13_put(&mut game, &copter, ALICE, Zone::Battlefield);
        let crew = p13_put(&mut game, &p13_vanilla(power, 2), ALICE, Zone::Battlefield);
        assert!(!game.current_is_creature(vehicle));
        let acts = p13_activations(&game, ALICE, vehicle);
        assert_eq!(
            !acts.is_empty(),
            power >= 1,
            "CR 702.122a: crew 1 needs total power 1 (have {power})"
        );
        if acts.is_empty() {
            continue;
        }
        let mut dm = P13Dm::new(true);
        dm.objects = Some(vec![crew]);
        p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
        assert!(
            game.is_tapped(crew),
            "CR 702.122a: the crewing creature is tapped as a cost"
        );
        let mut q = TriggerQueue::new();
        p13_settle(&mut game, &mut q, &mut dm);
        assert!(
            game.current_is_creature(vehicle),
            "CR 702.122a: the Vehicle becomes an artifact creature"
        );
        assert!(
            game.object(vehicle)
                .unwrap()
                .card_types
                .contains(&ironsmith::CardType::Artifact)
        );
        assert!(
            !game.is_tapped(vehicle),
            "CR 702.122a: a Vehicle can't tap itself to crew"
        );
    }
}

#[test]
fn cr_702_122b_c_e_crews_and_becomes_crewed_triggers() {
    let ace = p13_compile(
        "Speedway Fanatic",
        "Mana cost: {1}{R}\nType: Creature — Human Pilot\nPower/Toughness: 2/1\nHaste\nWhenever this creature crews a Vehicle, that Vehicle gains haste until end of turn.",
    );
    let ark = p13_compile(
        "Crewed Watcher",
        "Mana cost: {4}\nType: Artifact — Vehicle\nPower/Toughness: 3/3\nWhenever this Vehicle becomes crewed, you gain 1 life.\nCrew 2",
    );
    let mut game = p13_game(2);
    let vehicle = p13_put(&mut game, &ark, ALICE, Zone::Battlefield);
    game.set_summoning_sick(vehicle);
    let pilot = p13_put(&mut game, &ace, ALICE, Zone::Battlefield);
    let acts = p13_activations(&game, ALICE, vehicle);
    let mut dm = P13Dm::new(true);
    dm.objects = Some(vec![pilot]);
    p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert!(
        game.object_has_ability(vehicle, &ironsmith::StaticAbility::haste()),
        "CR 702.122b: the tapped creature crewed the Vehicle, so its crews trigger affects that Vehicle"
    );
    assert_eq!(
        game.player(ALICE).unwrap().life,
        21,
        "CR 702.122e: 'becomes crewed' triggers when the crew ability resolves"
    );
}

#[test]
fn cr_702_122e_becomes_crewed_intervening_if_reads_crewing_creatures() {
    let servant = p13_try_compile(
        "Mighty Servant of Leuk-o",
        "Mana cost: {3}\nType: Artifact — Vehicle\nPower/Toughness: 6/6\nTrample\nWhenever this Vehicle becomes crewed for the first time each turn, if it was crewed by exactly two creatures, it gains \"Whenever this creature deals combat damage to a player, draw two cards\" until end of turn.\nCrew 4",
    )
    .unwrap_or_else(|e| panic!("CR 702.122e: Mighty Servant of Leuk-o must compile: {e}"));
    for crew_count in [1usize, 2] {
        let mut game = p13_game(2);
        p13_fill_library(&mut game, ALICE, 5);
        let vehicle = p13_put(&mut game, &servant, ALICE, Zone::Battlefield);
        let crew: Vec<ObjectId> = if crew_count == 1 {
            vec![p13_put(
                &mut game,
                &p13_vanilla(4, 4),
                ALICE,
                Zone::Battlefield,
            )]
        } else {
            vec![
                p13_put(&mut game, &p13_vanilla(2, 2), ALICE, Zone::Battlefield),
                p13_put(&mut game, &p13_vanilla(2, 2), ALICE, Zone::Battlefield),
            ]
        };
        let acts = p13_activations(&game, ALICE, vehicle);
        let mut dm = P13Dm::new(true);
        dm.objects = Some(crew.clone());
        p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
        let mut q = TriggerQueue::new();
        p13_settle(&mut game, &mut q, &mut dm);
        let hand = game.player(ALICE).unwrap().hand.len();
        p13_full_unblocked_combat(&mut game, &[(vehicle, AttackTarget::Player(BOB))], &mut dm);
        assert_eq!(
            game.player(ALICE).unwrap().hand.len() - hand,
            if crew_count == 2 { 2 } else { 0 },
            "CR 702.122e: the intervening if counts only the creatures that crewed it (crewed by {crew_count})"
        );
    }
}

#[test]
fn cr_702_122d_creature_that_cant_crew_cannot_pay_crew_cost() {
    let copter = p13_compile("Smuggler's Copter", P13_COPTER);
    let jail = p13_compile(
        "Crew Jail",
        "Mana cost: {2}\nType: Enchantment\nCreatures your opponents control can't attack, block, or crew Vehicles.",
    );
    let mut game = p13_game(2);
    let vehicle = p13_put(&mut game, &copter, ALICE, Zone::Battlefield);
    p13_put(&mut game, &p13_vanilla(2, 2), ALICE, Zone::Battlefield);
    assert!(!p13_activations(&game, ALICE, vehicle).is_empty());
    p13_put(&mut game, &jail, BOB, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(
        p13_activations(&game, ALICE, vehicle).is_empty(),
        "CR 702.122d: a creature that can't crew Vehicles can't be tapped to pay a crew cost"
    );
}

#[test]
fn cr_702_128a_b_embalm_creates_white_zombie_token_without_mana_cost() {
    let cat = p13_compile(
        "Sacred Cat",
        "Mana cost: {W}\nType: Creature — Cat\nPower/Toughness: 1/1\nLifelink\nEmbalm {W}",
    );
    let mut game = p13_game(2);
    let card = p13_put(&mut game, &cat, ALICE, Zone::Graveyard);
    p13_mana(&mut game, ALICE, &[(ManaSymbol::White, 1)]);
    let mut instant_speed = game.clone();
    instant_speed.turn.active_player = BOB;
    assert!(
        p13_activations(&instant_speed, ALICE, card).is_empty(),
        "CR 702.128a: embalm only as a sorcery"
    );
    let acts = p13_activations(&game, ALICE, card);
    assert_eq!(
        acts.len(),
        1,
        "CR 702.128a: embalm is activatable from the graveyard"
    );
    let mut dm = P13Dm::new(true);
    p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
    assert_eq!(
        p13_named(&game, Zone::Exile, "Sacred Cat").len(),
        1,
        "CR 702.128a: exiling the card is part of the cost"
    );
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    let tokens = p13_named(&game, Zone::Battlefield, "Sacred Cat");
    assert_eq!(tokens.len(), 1);
    let t = tokens[0];
    let chars = game.current_characteristics(t).unwrap();
    assert!(
        game.object(t).unwrap().kind == ironsmith::ObjectKind::Token
            || format!("{:?}", game.object(t).unwrap().kind).contains("Token")
    );
    assert_eq!(
        game.current_colors(t).unwrap(),
        ironsmith::ColorSet::WHITE,
        "CR 702.128a: the token is white"
    );
    assert!(
        chars.subtypes.contains(&ironsmith::Subtype::Zombie)
            && chars.subtypes.contains(&ironsmith::Subtype::Cat),
        "CR 702.128a: Zombie in addition"
    );
    assert!(
        chars
            .mana_cost
            .as_ref()
            .map_or(true, |c| c.mana_value() == 0),
        "CR 702.128a: no mana cost"
    );
    assert!(
        game.object_has_ability(t, &ironsmith::StaticAbility::lifelink()),
        "copy keeps lifelink"
    );
}

#[test]
fn cr_702_129a_eternalize_creates_black_4_4_zombie_token() {
    let khenra = p13_compile(
        "Proven Combatant",
        "Mana cost: {U}\nType: Creature — Human Warrior\nPower/Toughness: 1/1\nEternalize {4}{U}{U}",
    );
    let mut game = p13_game(2);
    let card = p13_put(&mut game, &khenra, ALICE, Zone::Graveyard);
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Blue, 2), (ManaSymbol::Colorless, 4)],
    );
    let acts = p13_activations(&game, ALICE, card);
    assert_eq!(
        acts.len(),
        1,
        "CR 702.129a: eternalize activatable from graveyard"
    );
    let mut dm = P13Dm::new(true);
    p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    let tokens = p13_named(&game, Zone::Battlefield, "Proven Combatant");
    assert_eq!(tokens.len(), 1);
    let t = tokens[0];
    let chars = game.current_characteristics(t).unwrap();
    assert_eq!(
        game.current_colors(t).unwrap(),
        ironsmith::ColorSet::BLACK,
        "CR 702.129a: black"
    );
    assert_eq!(
        (game.calculated_power(t), game.calculated_toughness(t)),
        (Some(4), Some(4)),
        "CR 702.129a: 4/4"
    );
    assert!(
        chars.subtypes.contains(&ironsmith::Subtype::Zombie)
            && chars.subtypes.contains(&ironsmith::Subtype::Warrior)
    );
    assert!(
        chars
            .mana_cost
            .as_ref()
            .map_or(true, |c| c.mana_value() == 0),
        "CR 702.129a: no mana cost"
    );
}

#[test]
fn cr_702_141a_encore_token_per_opponent_attacks_and_is_sacrificed() {
    let def = p13_compile(
        "Briarblade Adept",
        "Mana cost: {4}{B}\nType: Creature — Elf Assassin\nPower/Toughness: 3/4\nEncore {3}{B}",
    );
    let mut game = p13_game(3);
    let card = p13_put(&mut game, &def, ALICE, Zone::Graveyard);
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Black, 1), (ManaSymbol::Colorless, 3)],
    );
    let acts = p13_activations(&game, ALICE, card);
    assert_eq!(
        acts.len(),
        1,
        "CR 702.141a: encore activatable from graveyard"
    );
    let mut dm = P13Dm::new(true);
    p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
    assert_eq!(p13_named(&game, Zone::Exile, "Briarblade Adept").len(), 1);
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    let tokens = p13_named(&game, Zone::Battlefield, "Briarblade Adept");
    assert_eq!(tokens.len(), 2, "CR 702.141a: one token per opponent");
    for t in &tokens {
        assert!(
            game.object_has_ability(*t, &ironsmith::StaticAbility::haste()),
            "CR 702.141a: tokens gain haste"
        );
    }
    // Attack requirement: each must attack its opponent.
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let mut combat = combat_state::new_combat();
    let mut q = TriggerQueue::new();
    let wrong: Vec<AttackerDeclaration> = tokens
        .iter()
        .map(|t| AttackerDeclaration {
            creature: *t,
            target: AttackTarget::Player(BOB),
        })
        .collect();
    let none = ironsmith::game_loop::apply_attacker_declarations(
        &mut game.clone(),
        &mut combat.clone(),
        &mut q.clone(),
        &[],
    );
    assert!(
        none.is_err(),
        "CR 702.141a: encore tokens attack this turn if able"
    );
    let both_bob = ironsmith::game_loop::apply_attacker_declarations(
        &mut game.clone(),
        &mut combat.clone(),
        &mut q.clone(),
        &wrong,
    );
    assert!(
        both_bob.is_err(),
        "CR 702.141a: each token must attack its designated opponent"
    );
    let _ = &mut combat;
    // Sacrificed at the beginning of the next end step.
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(ironsmith::Step::End);
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut q);
    p13_settle(&mut game, &mut q, &mut dm);
    assert!(
        p13_named(&game, Zone::Battlefield, "Briarblade Adept").is_empty(),
        "CR 702.141a: tokens sacrificed at the next end step"
    );
}

#[test]
fn cr_702_142a_b_boast_only_after_attacking_once_per_turn() {
    let def = p13_compile(
        "Duskwielder",
        "Mana cost: {B}\nType: Creature — Elf Berserker\nPower/Toughness: 1/2\nBoast — {1}: Target opponent loses 1 life and you gain 1 life.",
    );
    let watcher: Result<CardDefinition, String> = Err("unused".into());
    let mut game = p13_game(2);
    let creature = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
    let has_watcher = watcher.is_ok();
    if let Ok(w) = &watcher {
        p13_put(&mut game, w, ALICE, Zone::Battlefield);
    }
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Colorless, 2)]);
    assert!(
        p13_activations(&game, ALICE, creature).is_empty(),
        "CR 702.142a: boast requires having attacked this turn"
    );
    let mut dm = P13Dm::new(true);
    p13_attack(&mut game, &[(creature, AttackTarget::Player(BOB))], &mut dm).unwrap();
    let acts = p13_activations(&game, ALICE, creature);
    assert_eq!(
        acts.len(),
        1,
        "CR 702.142a: boast available after attacking"
    );
    p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert_eq!(game.player(BOB).unwrap().life, 19);
    assert!(
        p13_activations(&game, ALICE, creature).is_empty(),
        "CR 702.142a: boast only once each turn"
    );
    let _ = has_watcher;
}

#[test]
fn cr_702_142b_boasting_is_activating_a_boast_ability() {
    let def = p13_compile(
        "Duskwielder",
        "Mana cost: {B}\nType: Creature — Elf Berserker\nPower/Toughness: 1/2\nBoast — {1}: Target opponent loses 1 life and you gain 1 life.",
    );
    let birgi = p13_try_compile(
        "Birgi, God of Storytelling",
        "Mana cost: {2}{R}\nType: Legendary Creature — God\nPower/Toughness: 3/3\nWhenever a creature you control boasts, add {R}. Until end of turn, you don't lose this mana as steps and phases end.\nCreatures you control can boast twice.",
    )
    .or_else(|_| p13_try_compile(
        "Birgi, God of Storytelling",
        "Mana cost: {2}{R}\nType: Legendary Creature — God\nPower/Toughness: 3/3\nWhenever a creature you control boasts, add {R}.",
    ));
    assert!(
        birgi.is_ok(),
        "CR 702.142b: an effect referring to a creature boasting (Birgi, God of Storytelling) must be supported: {:?}",
        birgi.as_ref().err()
    );
    let birgi = birgi.unwrap();
    let mut game = p13_game(2);
    let creature = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
    p13_put(&mut game, &birgi, ALICE, Zone::Battlefield);
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Colorless, 1)]);
    let mut dm = P13Dm::new(true);
    p13_attack(&mut game, &[(creature, AttackTarget::Player(BOB))], &mut dm).unwrap();
    let acts = p13_activations(&game, ALICE, creature);
    p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert_eq!(
        game.player(ALICE).unwrap().mana_pool.red,
        1,
        "CR 702.142b: activating the boast ability is boasting"
    );
}

// ---------------------------------------------------------------------------
// 702.131 Ascend
// ---------------------------------------------------------------------------

const P13_SWORDTOOTH: &str = "Mana cost: {2}{G}\nType: Creature — Dinosaur\nPower/Toughness: 5/5\nAscend\nYou may play an additional land on each of your turns.\nThis creature can't attack or block unless you have the city's blessing.";

fn p13_fill_permanents(game: &mut GameState, owner: PlayerId, n: usize) {
    let rock = p13_compile("Rock", "Mana cost: {1}\nType: Artifact");
    for _ in 0..n {
        p13_put(game, &rock, owner, Zone::Battlefield);
    }
}

#[test]
fn cr_702_131b_c_permanent_ascend_grants_blessing_at_ten_permanents() {
    let def = p13_compile("Wayward Swordtooth", P13_SWORDTOOTH);
    let mut game = p13_game(2);
    let tooth = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
    let bob_tooth = p13_put(&mut game, &def, BOB, Zone::Battlefield);
    p13_fill_permanents(&mut game, ALICE, 8);
    p13_fill_permanents(&mut game, BOB, 9);
    game.refresh_continuous_state().unwrap();
    assert!(
        !game.has_citys_blessing(ALICE),
        "CR 702.131b: nine permanents is not enough"
    );
    assert!(
        game.has_citys_blessing(BOB),
        "CR 702.131b: ten permanents grants the blessing"
    );
    let mut dm = P13Dm::new(true);
    assert!(
        p13_attack(
            &mut game.clone(),
            &[(tooth, AttackTarget::Player(BOB))],
            &mut dm
        )
        .is_err()
    );
    p13_fill_permanents(&mut game, ALICE, 1);
    game.refresh_continuous_state().unwrap();
    assert!(
        game.has_citys_blessing(ALICE) && game.has_citys_blessing(BOB),
        "CR 702.131c: any number of players may have the blessing"
    );
    // Losing permanents doesn't remove it (rest of the game).
    let rocks = p13_named(&game, Zone::Battlefield, "Rock");
    let mine: Vec<ObjectId> = rocks
        .iter()
        .copied()
        .filter(|r| game.current_controller(*r) == Some(ALICE))
        .take(5)
        .collect();
    for r in mine.iter() {
        game.move_object_by_effect(*r, Zone::Graveyard).unwrap();
    }
    game.refresh_continuous_state().unwrap();
    assert!(
        game.has_citys_blessing(ALICE),
        "CR 702.131b/c: the blessing lasts for the rest of the game"
    );
    assert!(
        p13_attack(&mut game, &[(tooth, AttackTarget::Player(BOB))], &mut dm).is_ok(),
        "Swordtooth attacks with the blessing"
    );
    let _ = bob_tooth;
}

#[test]
fn cr_702_131a_spell_ascend_checks_on_resolution() {
    let def = p13_compile(
        "Secrets of the Golden City",
        "Mana cost: {1}{U}{U}\nType: Sorcery\nAscend\nDraw two cards. If you have the city's blessing, draw three cards instead.",
    );
    for (permanents, blessed) in [(9usize, false), (10, true)] {
        let mut game = p13_game(2);
        p13_fill_library(&mut game, ALICE, 5);
        p13_fill_permanents(&mut game, ALICE, permanents);
        let mut dm = P13Dm::new(true);
        let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
        p13_mana(
            &mut game,
            ALICE,
            &[(ManaSymbol::Blue, 2), (ManaSymbol::Colorless, 1)],
        );
        p13_act(
            &mut game,
            ALICE,
            p13_cast_action(card, Zone::Hand, CastingMethod::Normal),
            &mut dm,
        )
        .unwrap();
        let mut q = TriggerQueue::new();
        p13_settle(&mut game, &mut q, &mut dm);
        assert_eq!(
            game.has_citys_blessing(ALICE),
            blessed,
            "CR 702.131a: ascend spell with {permanents} permanents"
        );
        assert_eq!(
            game.player(ALICE).unwrap().hand.len(),
            if blessed { 3 } else { 2 }
        );
    }
}

#[test]
fn cr_702_131d_blessing_applies_before_trigger_checks() {
    let tooth = p13_compile("Wayward Swordtooth", P13_SWORDTOOTH);
    let watcher = p13_compile(
        "Blessing watcher",
        "Mana cost: {1}{W}\nType: Creature — Human\nPower/Toughness: 1/1\nWhenever another creature you control enters, if you have the city's blessing, you gain 1 life.",
    );
    let mut game = p13_game(2);
    p13_put(&mut game, &tooth, ALICE, Zone::Battlefield);
    p13_put(&mut game, &watcher, ALICE, Zone::Battlefield);
    p13_fill_permanents(&mut game, ALICE, 7);
    game.refresh_continuous_state().unwrap();
    assert!(!game.has_citys_blessing(ALICE));
    let mut dm = P13Dm::new(true);
    p13_cast_and_resolve(
        &mut game,
        &p13_vanilla(1, 1),
        &[(ManaSymbol::Colorless, 1)],
        &mut dm,
    );
    assert!(game.has_citys_blessing(ALICE));
    assert_eq!(
        game.player(ALICE).unwrap().life,
        21,
        "CR 702.131d: the tenth permanent's entry grants the blessing before its trigger condition is checked"
    );
}

// ---------------------------------------------------------------------------
// 702.139 Companion
// ---------------------------------------------------------------------------

fn p13_companion_def() -> CardDefinition {
    use ironsmith::{
        Ability, CardBuilder, CardId, CardType, CompanionDeckCondition, ManaCost, StaticAbility,
    };
    let card = CardBuilder::new(CardId::new(), "Small Companion")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 2))
        .build();
    CardDefinition::with_abilities(
        card,
        vec![Ability::static_ability(StaticAbility::companion(
            CompanionDeckCondition::PermanentManaValueAtMost(2),
            "Companion test",
        ))],
    )
}

#[test]
fn cr_702_139b_starting_deck_excludes_sideboard_cards() {
    let mut game = p13_game(2);
    let companion =
        game.create_object_from_definition(&p13_companion_def(), ALICE, Zone::OutsideGame);
    let small = p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Library);
    let big_def = p13_compile(
        "Big",
        "Mana cost: {5}\nType: Creature — Bear\nPower/Toughness: 5/5",
    );
    let sideboard_big = game.create_object_from_definition(&big_def, ALICE, Zone::OutsideGame);
    assert!(
        game.clone()
            .designate_companion(ALICE, companion, &[small, sideboard_big], 2)
            .is_err(),
        "control: a big card in the starting deck fails the condition"
    );
    game.designate_companion(ALICE, companion, &[small], 1)
        .expect(
            "CR 702.139b: sideboard cards are set aside before the companion condition is checked",
        );
}

#[test]
fn cr_702_139c_d_companion_put_into_hand_stays_in_game_including_commander() {
    use ironsmith::special_actions::{SpecialAction, perform};
    let mut game = p13_game(2);
    let commander_def = p13_compile(
        "Cmdr",
        "Mana cost: {1}{G}\nType: Legendary Creature — Elf\nPower/Toughness: 2/2",
    );
    let commander = game.create_object_from_definition(&commander_def, ALICE, Zone::Command);
    game.set_as_commander(commander, ALICE);
    let companion =
        game.create_object_from_definition(&p13_companion_def(), ALICE, Zone::OutsideGame);
    let small = p13_put(&mut game, &p13_vanilla(1, 1), ALICE, Zone::Library);
    game.designate_companion(ALICE, companion, &[small], 1)
        .unwrap();
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Colorless, 3)]);
    let action = SpecialAction::Companion { card_id: companion };
    assert!(
        p13_legal(&game, ALICE).contains(&LegalAction::SpecialAction(action.clone())),
        "CR 702.139d: companion enters a Commander game from outside the game"
    );
    let mut dm = P13Dm::new(true);
    perform(action, &mut game, ALICE, &mut dm).unwrap();
    let in_hand = game
        .player(ALICE)
        .unwrap()
        .hand
        .iter()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(in_hand.len(), 1);
    let card = in_hand[0];
    game.move_object_by_effect(card, Zone::Graveyard).unwrap();
    assert_eq!(
        p13_owned_in(&game, ALICE, Zone::Graveyard).len(),
        1,
        "CR 702.139c: the companion remains in the game"
    );
    assert!(p13_owned_in(&game, ALICE, Zone::OutsideGame).is_empty());
}

// ---------------------------------------------------------------------------
// 702.150 Compleated
// ---------------------------------------------------------------------------

#[test]
fn cr_702_150a_compleated_enters_with_fewer_loyalty_when_life_paid() {
    let def = p13_compile(
        "Tamiyo, Compleated Sage",
        "Mana cost: {2}{G}{G/U/P}{U}\nType: Legendary Planeswalker — Tamiyo\nLoyalty: 5\nCompleated\n+1: Tap up to one target artifact or creature. It doesn't untap during its controller's next untap step.",
    );
    for pay_life in [false, true] {
        let mut game = p13_game(2);
        let mut dm = P13Dm::new(true).prefer(if pay_life { "life" } else { "{u}" });
        let mut mana = vec![
            (ManaSymbol::Green, 1),
            (ManaSymbol::Blue, 1),
            (ManaSymbol::Colorless, 2),
        ];
        if !pay_life {
            mana.push((ManaSymbol::Blue, 1));
        }
        let id = p13_cast_and_resolve(&mut game, &def, &mana, &mut dm);
        assert_eq!(
            game.player(ALICE).unwrap().life,
            if pay_life { 18 } else { 20 },
            "log={:?}",
            dm.log
        );
        assert_eq!(
            game.counter_count(id, CounterType::Loyalty),
            if pay_life { 3 } else { 5 },
            "CR 702.150a: two fewer loyalty counters per Phyrexian symbol paid with life (life paid: {pay_life}); log={:?}",
            dm.log
        );
    }
}

// ---------------------------------------------------------------------------
// 702.143 Foretell
// ---------------------------------------------------------------------------

#[test]
fn cr_702_143a_b_c_foretell_special_action_and_later_cast() {
    use ironsmith::special_actions::SpecialAction;
    let def = p13_compile(
        "Augury Raven",
        "Mana cost: {3}{U}\nType: Creature — Bird\nPower/Toughness: 3/1\nFlying\nForetell {1}{U}",
    );
    let watcher = p13_compile(
        "Foretell watcher",
        "Mana cost: {1}{U}\nType: Creature — Human\nPower/Toughness: 1/1\nWhenever you foretell a card, you gain 2 life.",
    );
    let mut game = p13_game(2);
    p13_put(&mut game, &watcher, ALICE, Zone::Battlefield);
    let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
    let bob_card = p13_put(&mut game, &def, BOB, Zone::Hand);
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Colorless, 2)]);
    p13_mana(&mut game, BOB, &[(ManaSymbol::Colorless, 2)]);
    let mut bobs_view = game.clone();
    bobs_view.turn.priority_player = Some(BOB);
    assert!(
        !p13_legal(&bobs_view, BOB).contains(&LegalAction::SpecialAction(
            SpecialAction::Foretell { card_id: bob_card }
        )),
        "CR 702.143a: foretell only during your own turn"
    );
    let action = LegalAction::SpecialAction(SpecialAction::Foretell { card_id: card });
    assert!(
        p13_legal(&game, ALICE).contains(&action),
        "CR 702.143a: foretell on your turn for {{2}}"
    );
    let stack_before = game.stack.len();
    let mut dm = P13Dm::new(true);
    p13_act(&mut game, ALICE, action, &mut dm).unwrap();
    assert_eq!(
        p13_pool_total(&game, ALICE),
        0,
        "CR 702.143a: foretelling costs {{2}}"
    );
    let exiled = p13_owned_in(&game, ALICE, Zone::Exile);
    assert_eq!(exiled.len(), 1);
    let foretold = exiled[0];
    assert!(game.is_face_down(foretold), "CR 702.143a: exiled face down");
    let mut q = TriggerQueue::new();
    // The special action itself doesn't use the stack (CR 702.143b): only the watcher trigger may be there.
    assert!(
        game.stack.len() <= stack_before + 1,
        "CR 702.143b: foretell doesn't use the stack"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|e| e.object_id == foretold || e.object_id == card),
        "CR 702.143b: the foretold card is not on the stack"
    );
    p13_settle(&mut game, &mut q, &mut dm);
    assert_eq!(
        game.player(ALICE).unwrap().life,
        22,
        "CR 702.143c: foretelling a card is the foretell special action"
    );
    // Not castable the same turn.
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 1)],
    );
    assert!(
        !p13_offers(&game, ALICE, foretold, |_| true),
        "CR 702.143a: can't cast a foretold card the turn it was foretold"
    );
    game.turn.turn_number += 1;
    assert!(
        p13_offers(&game, ALICE, foretold, |_| true),
        "CR 702.143a: castable for its foretell cost after the turn it was foretold"
    );
}

#[test]
fn cr_702_143d_effect_makes_card_foretold_with_cost() {
    let valkyrie = p13_compile(
        "Ethereal Valkyrie",
        "Mana cost: {4}{W}{U}\nType: Creature — Spirit Angel\nPower/Toughness: 4/4\nFlying\nWhenever this creature enters or attacks, draw a card, then exile a card from your hand face down. It becomes foretold. Its foretell cost is its mana cost reduced by {2}.",
    );
    let spell = p13_compile(
        "Big Draw",
        "Mana cost: {3}{U}\nType: Sorcery\nDraw two cards.",
    );
    let mut game = p13_game(2);
    p13_fill_library(&mut game, ALICE, 5);
    let big = p13_put(&mut game, &spell, ALICE, Zone::Hand);
    let big_stable = game.object(big).unwrap().stable_id;
    let mut dm = P13Dm::new(true);
    dm.objects = Some(vec![big]);
    p13_cast_and_resolve(
        &mut game,
        &valkyrie,
        &[
            (ManaSymbol::White, 1),
            (ManaSymbol::Blue, 1),
            (ManaSymbol::Colorless, 4),
        ],
        &mut dm,
    );
    let foretold = game
        .find_object_by_stable_id(big_stable)
        .filter(|id| game.object(*id).unwrap().zone == Zone::Exile);
    let foretold = foretold.unwrap_or_else(|| {
        panic!(
            "CR 702.143d: the chosen card is exiled and becomes foretold; log={:?}",
            dm.log
        )
    });
    assert!(
        game.foretold_turn(foretold).is_some(),
        "CR 702.143d: it becomes a foretold card"
    );
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 1)],
    );
    assert!(
        !p13_offers(&game, ALICE, foretold, |_| true),
        "CR 702.143d: not castable the turn it became foretold"
    );
    game.turn.turn_number += 1;
    let mut dm2 = P13Dm::new(true);
    let action = p13_legal(&game, ALICE)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == foretold));
    let action = action
        .expect("CR 702.143d: foretold card castable for its foretell cost {1}{U} on a later turn");
    p13_act(&mut game, ALICE, action, &mut dm2).unwrap();
    assert_eq!(
        p13_pool_total(&game, ALICE),
        0,
        "CR 702.143d: paid mana cost reduced by {{2}}"
    );
}

// ---------------------------------------------------------------------------
// 702.146 Disturb
// ---------------------------------------------------------------------------

#[test]
fn cr_702_146a_b_disturb_casts_transformed_and_enters_back_face_up() {
    let mut front = p13_compile(
        "Beloved Beggar",
        "Mana cost: {1}{W}\nType: Creature — Human Peasant\nPower/Toughness: 0/4\nDisturb {4}{W}{W}",
    );
    let mut back = p13_compile(
        "Generous Soul",
        "Type: Creature — Spirit\nColor indicator: White\nPower/Toughness: 1/1\nFlying\nVigilance",
    );
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some("Generous Soul".into());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    front.card.transforming_dfc = true;
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some("Beloved Beggar".into());
    back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    back.card.transforming_dfc = true;
    let mut game = p13_game(2);
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let card = p13_put(&mut game, &front, ALICE, Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::White, 2), (ManaSymbol::Colorless, 4)],
    );
    let mut dm = P13Dm::new(true);
    let r = p13_act(
        &mut game,
        ALICE,
        p13_cast_action(card, Zone::Graveyard, CastingMethod::Alternative(0)),
        &mut dm,
    );
    assert!(
        r.is_ok(),
        "CR 702.146a: disturb casts the card from the graveyard: {r:?}"
    );
    assert_eq!(p13_pool_total(&game, ALICE), 0);
    let spell = game.stack.last().unwrap().object_id;
    assert_eq!(
        game.object(spell).unwrap().name,
        "Generous Soul",
        "CR 702.146a: cast transformed (back face on the stack)"
    );
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.object(permanent).unwrap().name,
        "Generous Soul",
        "CR 702.146b: enters with its back face up"
    );
    assert!(game.object_has_ability(permanent, &ironsmith::StaticAbility::flying()));
}

// ---------------------------------------------------------------------------
// 702.124 Partner (runtime parts)
// ---------------------------------------------------------------------------

fn p13_two_commanders(game: &mut GameState) -> (ObjectId, ObjectId) {
    let a = p13_compile(
        "Partner Alpha",
        "Mana cost: {1}{G}\nType: Legendary Creature — Elf\nPower/Toughness: 11/11\nPartner",
    );
    let b = p13_compile(
        "Partner Beta",
        "Mana cost: {1}{R}\nType: Legendary Creature — Goblin\nPower/Toughness: 10/10\nPartner",
    );
    let a = game.create_object_from_definition(&a, ALICE, Zone::Command);
    let b = game.create_object_from_definition(&b, ALICE, Zone::Command);
    game.set_as_commander(a, ALICE);
    game.set_as_commander(b, ALICE);
    (a, b)
}

#[test]
fn cr_702_124d_partner_commanders_track_tax_and_damage_independently() {
    let mut game = p13_game(2);
    game.player_mut(BOB).unwrap().life = 100;
    game.set_commander_damage_loss_enabled(true);
    let (a, b) = p13_two_commanders(&mut game);
    let a_stable = game.object(a).unwrap().stable_id;
    let b_stable = game.object(b).unwrap().stable_id;
    let mut dm = P13Dm::new(true);
    // Cast A, return it to the command zone, then B costs no tax.
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Green, 1), (ManaSymbol::Colorless, 1)],
    );
    p13_act(
        &mut game,
        ALICE,
        p13_cast_action(a, Zone::Command, CastingMethod::Normal),
        &mut dm,
    )
    .unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    let a_now = game.find_object_by_stable_id(a_stable).unwrap();
    assert_eq!(game.object(a_now).unwrap().zone, Zone::Battlefield);
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Red, 1), (ManaSymbol::Colorless, 1)],
    );
    let b_now = game.find_object_by_stable_id(b_stable).unwrap();
    let r = p13_act(
        &mut game,
        ALICE,
        p13_cast_action(b_now, Zone::Command, CastingMethod::Normal),
        &mut dm,
    );
    assert!(
        r.is_ok(),
        "CR 702.124d: casting one partner commander ignores the other's cast count: {r:?}"
    );
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    let b_now = game.find_object_by_stable_id(b_stable).unwrap();
    game.remove_summoning_sickness(a_now);
    game.remove_summoning_sickness(b_now);
    // 11 + 10 combat damage: no single commander dealt 21.
    p13_full_unblocked_combat(
        &mut game,
        &[
            (a_now, AttackTarget::Player(BOB)),
            (b_now, AttackTarget::Player(BOB)),
        ],
        &mut dm,
    );
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert!(
        !game.player(BOB).unwrap().has_lost,
        "CR 702.124d: commander damage from two partners is tracked separately (11 and 10)"
    );
    game.untap(a_now);
    p13_full_unblocked_combat(&mut game, &[(a_now, AttackTarget::Player(BOB))], &mut dm);
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    assert!(
        game.player(BOB).unwrap().has_lost,
        "CR 702.124d/704.6c: 22 combat damage from one partner commander loses the game"
    );
}

#[test]
fn cr_702_124e_your_commander_refers_to_either_partner() {
    let beacon = p13_compile(
        "Command Beacon",
        "Type: Land\n{T}: Add {C}.\n{T}, Sacrifice this land: Put your commander into your hand from the command zone.",
    );
    for pick_second in [false, true] {
        let mut game = p13_game(2);
        let (a, b) = p13_two_commanders(&mut game);
        let land = p13_put(&mut game, &beacon, ALICE, Zone::Battlefield);
        let chosen = if pick_second { b } else { a };
        let chosen_stable = game.object(chosen).unwrap().stable_id;
        let acts = p13_activations(&game, ALICE, land);
        // Decline the CR 903.9b command-zone replacement so the card stays in hand.
        let mut dm = P13Dm::new(false);
        dm.objects = Some(vec![chosen]);
        p13_act(&mut game, ALICE, acts.last().unwrap().clone(), &mut dm).unwrap();
        let mut q = TriggerQueue::new();
        p13_settle(&mut game, &mut q, &mut dm);
        let moved = game.find_object_by_stable_id(chosen_stable).unwrap();
        assert_eq!(
            game.object(moved).unwrap().zone,
            Zone::Hand,
            "CR 702.124e: the player chooses which of two commanders 'your commander' refers to (picked second={pick_second}); log={:?}",
            dm.log
        );
        assert_eq!(game.player(ALICE).unwrap().hand.len(), 1);
    }
}

#[test]
fn cr_702_143f_foretold_cards_revealed_at_game_end() {
    use ironsmith::special_actions::SpecialAction;
    let def = p13_compile(
        "Saw It Coming",
        "Mana cost: {1}{U}{U}\nType: Instant\nCounter target spell.\nForetell {1}{U}",
    );
    let mut game = p13_game(2);
    let card = p13_put(&mut game, &def, ALICE, Zone::Hand);
    p13_mana(&mut game, ALICE, &[(ManaSymbol::Colorless, 2)]);
    let mut dm = P13Dm::new(true);
    p13_act(
        &mut game,
        ALICE,
        LegalAction::SpecialAction(SpecialAction::Foretell { card_id: card }),
        &mut dm,
    )
    .unwrap();
    let foretold = p13_owned_in(&game, ALICE, Zone::Exile)[0];
    assert!(game.is_face_down(foretold));
    let stable = game.object(foretold).unwrap().stable_id;
    game.player_mut(BOB).unwrap().life = 0;
    let mut q = TriggerQueue::new();
    let mut progress = None;
    for _ in 0..4 {
        let p = ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut q, &mut dm).unwrap();
        if matches!(p, ironsmith::decision::GameProgress::GameOver(_)) {
            progress = Some(p);
            break;
        }
    }
    assert!(progress.is_some(), "fixture: Bob at 0 life ends the game");
    let revealed = game
        .ui_effect_events()
        .any(|e| e.kind == "reveal" && e.stable_ids.contains(&stable));
    assert!(
        revealed,
        "CR 702.143f: all face-down foretold cards are revealed at the end of the game"
    );
}

#[test]
fn cr_702_124j_partner_with_etb_tutors_named_partner() {
    let amy = p13_compile(
        "Amy Pond",
        "Mana cost: {2}{R}\nType: Legendary Creature — Human\nPower/Toughness: 2/2\nPartner with Rory Williams",
    );
    let rory = p13_compile(
        "Rory Williams",
        "Mana cost: {2}{W}{U}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 3/3",
    );
    let mut game = p13_game(2);
    p13_fill_library(&mut game, ALICE, 3);
    let rory_id = p13_put(&mut game, &rory, ALICE, Zone::Library);
    let rory_stable = game.object(rory_id).unwrap().stable_id;
    let mut dm = P13Dm::new(true);
    dm.targets = Some(vec![Target::Player(ALICE)]);
    dm.objects = Some(vec![rory_id]);
    p13_cast_and_resolve(
        &mut game,
        &amy,
        &[(ManaSymbol::Red, 1), (ManaSymbol::Colorless, 2)],
        &mut dm,
    );
    let rory_now = game.find_object_by_stable_id(rory_stable).unwrap();
    assert_eq!(
        game.object(rory_now).unwrap().zone,
        Zone::Hand,
        "CR 702.124j: partner with's ETB lets the target player search for the named partner; log={:?}",
        dm.log
    );
}

#[test]
fn cr_702_124n_reference_to_partner_covers_partner_family_not_background() {
    let akroma = p13_try_compile(
        "Akroma, Vision of Ixidor",
        "Mana cost: {5}{W}{W}\nType: Legendary Creature — Angel\nPower/Toughness: 6/6\nFlying, first strike, vigilance, trample\nAt the beginning of each combat, until end of turn, each other creature you control gets +1/+1 if it has flying, +1/+1 if it has first strike, and so on for double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, vigilance, and partner.\nPartner",
    );
    assert!(
        akroma.is_ok(),
        "CR 702.124n: Akroma, Vision of Ixidor (refers to partner) must be supported: {:?}",
        akroma.as_ref().err()
    );
    let akroma = akroma.unwrap();
    let cases = [
        ("Partner", 1),
        ("Partner with Rory Williams", 1),
        ("Partner—Friends forever", 1),
        ("Choose a Background", 0),
    ];
    let mut wrong = Vec::new();
    for (keyword, bonus) in cases {
        let def = p13_compile(
            "Partner probe",
            &format!(
                "Mana cost: {{2}}\nType: Legendary Creature — Human\nPower/Toughness: 2/2\n{keyword}"
            ),
        );
        let mut game = p13_game(2);
        p13_put(&mut game, &akroma, ALICE, Zone::Battlefield);
        let probe = p13_put(&mut game, &def, ALICE, Zone::Battlefield);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::Step::BeginCombat);
        let mut q = TriggerQueue::new();
        ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut q);
        let mut dm = P13Dm::new(true);
        p13_settle(&mut game, &mut q, &mut dm);
        if game.calculated_power(probe) != Some(2 + bonus) {
            wrong.push(format!(
                "{keyword}: power {:?}, required {}",
                game.calculated_power(probe),
                2 + bonus
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "CR 702.124n: 'partner' refers to partner, partner—[text], and partner with, not other variants: {wrong:?}"
    );
}

#[test]
fn cr_702_128b_embalmed_token_is_identified_by_vizier_of_many_faces() {
    let vizier = p13_try_compile(
        "Vizier of Many Faces",
        "Mana cost: {2}{U}{U}\nType: Creature — Shapeshifter Cleric\nPower/Toughness: 0/0\nYou may have this creature enter as a copy of any creature on the battlefield, except if this creature was embalmed, the token has no mana cost, it's white, and it's a Zombie in addition to its other types.\nEmbalm {3}{U}{U}",
    );
    assert!(
        vizier.is_ok(),
        "CR 702.128b: Vizier of Many Faces (refers to being embalmed) must be supported: {:?}",
        vizier.as_ref().err()
    );
    let vizier = vizier.unwrap();
    let bear = p13_compile(
        "Green Bear",
        "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 3/3",
    );
    let mut game = p13_game(2);
    let model = p13_put(&mut game, &bear, BOB, Zone::Battlefield);
    let card = p13_put(&mut game, &vizier, ALICE, Zone::Graveyard);
    p13_mana(
        &mut game,
        ALICE,
        &[(ManaSymbol::Blue, 2), (ManaSymbol::Colorless, 3)],
    );
    let acts = p13_activations(&game, ALICE, card);
    assert_eq!(acts.len(), 1, "embalm is offered");
    let mut dm = P13Dm::new(true).prefer("copy of");
    dm.objects = Some(vec![model]);
    p13_act(&mut game, ALICE, acts[0].clone(), &mut dm).unwrap();
    let mut q = TriggerQueue::new();
    p13_settle(&mut game, &mut q, &mut dm);
    let tokens = p13_named(&game, Zone::Battlefield, "Green Bear")
        .into_iter()
        .filter(|id| *id != model)
        .collect::<Vec<_>>();
    assert_eq!(
        tokens.len(),
        1,
        "embalmed Vizier entered as a copy of the bear; log={:?}",
        dm.log
    );
    let t = tokens[0];
    let chars = game.current_characteristics(t).unwrap();
    assert!(
        game.current_colors(t).unwrap() == ironsmith::ColorSet::WHITE
            && chars.subtypes.contains(&ironsmith::Subtype::Zombie)
            && chars
                .mana_cost
                .as_ref()
                .map_or(true, |c| c.mana_value() == 0),
        "CR 702.128b: the token created by the embalm ability is embalmed, so the copy exception applies (colors {:?}, subtypes {:?})",
        game.current_colors(t),
        chars.subtypes
    );
}
