//! CR audit (pending-entry pass, package p05): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, EffectExecutor, GameState, PlayerId};
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::DecisionContext;
use ironsmith::effects::{ApplyContinuousEffect, MoveToZoneEffect};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardId, CardType, ChooseSpec, CounterType, DecisionMaker, ObjectId, Phase, PriorityLoopState,
    PriorityResponse, Step, TurnAction, TurnRunner, TurnRunnerState, Until, Zone,
};

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}

fn card(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("{name} must compile: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "{name}: printed text must be fully supported"
    );
    def
}

fn creature(name: &str, p: i32, t: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(p, t))
        .build()
}

fn filler(game: &mut GameState, player: PlayerId, zone: Zone, n: usize) -> Vec<ObjectId> {
    (0..n)
        .map(|i| {
            let def = creature(&format!("Filler {i}"), 1, 1);
            game.create_object_from_definition(&def, player, zone)
        })
        .collect()
}

fn main_game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.turn_number = 2;
    g
}

fn find_stable(game: &GameState, ids: &[ObjectId], stable: ironsmith::ids::StableId) -> Option<ObjectId> {
    ids.iter().copied().find(|id| game.object(*id).is_some_and(|o| o.stable_id == stable))
}

fn cast_with<D: DecisionMaker>(
    g: &mut GameState,
    q: &mut TriggerQueue,
    id: ObjectId,
    dm: &mut D,
) -> ObjectId {
    let player = g.turn.priority_player.expect("someone has priority");
    let before = g.stack.len();
    let action = compute_legal_actions(g, player)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
        .unwrap_or_else(|| panic!("cast action must be available for {:?}", g.object(id).map(|o| o.name.to_string())));
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .expect("cast must start");
    for _ in 0..48 {
        if state.pending_cast.is_none() && g.stack.len() > before {
            return g.stack.last().unwrap().object_id;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("cast stopped before reaching the stack: {progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .expect("cast decisions");
    }
    panic!("cast did not finish");
}

fn cast(g: &mut GameState, q: &mut TriggerQueue, id: ObjectId) -> ObjectId {
    cast_with(g, q, id, &mut SelectFirstDecisionMaker)
}

fn pass_round<D: DecisionMaker>(g: &mut GameState, q: &mut TriggerQueue, d: &mut D) {
    let mut s = PriorityLoopState::new(g.players_in_game());
    for _ in 0..g.players_in_game() {
        ironsmith::game_loop::apply_priority_response_with_dm(
            g,
            q,
            &mut s,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            d,
        )
        .unwrap();
    }
    ironsmith::game_loop::advance_priority_with_dm(g, q, d).unwrap();
}

fn settle_with<D: DecisionMaker>(g: &mut GameState, q: &mut TriggerQueue, d: &mut D) {
    for _ in 0..64 {
        ironsmith::game_loop::advance_priority_with_dm(g, q, d).unwrap();
        if g.stack.is_empty() {
            return;
        }
        pass_round(g, q, d);
    }
    panic!("stack must settle");
}

fn settle(g: &mut GameState, q: &mut TriggerQueue) {
    settle_with(g, q, &mut SelectFirstDecisionMaker)
}

fn finish_window<D: DecisionMaker>(g: &mut GameState, q: &mut TriggerQueue, d: &mut D) {
    let mut s = PriorityLoopState::new(g.players_in_game());
    for _ in 0..16 {
        let p = ironsmith::game_loop::apply_priority_response_with_dm(
            g,
            q,
            &mut s,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            d,
        )
        .unwrap();
        if matches!(p, GameProgress::Continue) {
            return;
        }
    }
    panic!("priority window must complete");
}

/// Drive one turn through the real runner. At each priority window `hook`
/// runs first (it may cast spells), then the stack settles and all pass.
fn run_turn(
    g: &mut GameState,
    q: &mut TriggerQueue,
    mut runner: TurnRunner,
    mut hook: impl FnMut(&mut GameState, &mut TriggerQueue),
) -> Vec<(Phase, Option<Step>)> {
    let mut windows = Vec::new();
    let mut d = SelectFirstDecisionMaker;
    for _ in 0..1024 {
        match runner.advance(g, q).unwrap() {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                windows.push((g.turn.phase, g.turn.step));
                ironsmith::game_loop::advance_priority_with_dm(g, q, &mut d).unwrap();
                hook(g, q);
                settle_with(g, q, &mut d);
                finish_window(g, q, &mut d);
                runner.priority_done();
            }
            TurnAction::Decision(DecisionContext::Attackers(_)) => runner.respond_attackers(vec![]),
            TurnAction::Decision(DecisionContext::Blockers(c)) => {
                runner.respond_blockers(vec![], c.player)
            }
            TurnAction::Decision(DecisionContext::SelectObjects(c)) => {
                let picks = d.decide_objects(g, &c);
                runner.respond_discard(picks)
            }
            TurnAction::TurnComplete => return windows,
            other => panic!("unexpected runner action: {other:?}"),
        }
    }
    panic!("turn must complete");
}

// ---------------------------------------------------------------------------
// 400: zones
// ---------------------------------------------------------------------------

#[test]
fn cr_400_1_each_player_has_own_library_hand_graveyard_and_shares_other_zones() {
    let mut g = main_game();
    assert_eq!(
        [Zone::Library, Zone::Hand, Zone::Battlefield, Zone::Graveyard, Zone::Stack, Zone::Exile, Zone::Command, Zone::Ante]
            .map(|z| z.name()),
        ["library", "hand", "battlefield", "graveyard", "stack", "exile", "command", "ante"],
        "CR 400.1: seven zones plus ante are modeled"
    );
    let a = g.create_object_from_definition(&creature("Alice card", 1, 1), alice(), Zone::Battlefield);
    let b = g.create_object_from_definition(&creature("Bob card", 1, 1), bob(), Zone::Battlefield);
    assert!(g.battlefield.contains(&a) && g.battlefield.contains(&b), "CR 400.1: shared battlefield");
    let (sa, sb) = (g.object(a).unwrap().stable_id, g.object(b).unwrap().stable_id);
    for (id, zone) in [(a, Zone::Graveyard), (b, Zone::Graveyard)] {
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(id), zone, false)
            .execute(&mut g, &mut EffectContext::new_default(id, alice()))
            .unwrap();
    }
    let ag = g.player(alice()).unwrap().graveyard.clone();
    let bg = g.player(bob()).unwrap().graveyard.clone();
    assert!(find_stable(&g, &ag, sa).is_some() && find_stable(&g, &ag, sb).is_none(), "CR 400.1: Alice's own graveyard");
    assert!(find_stable(&g, &bg, sb).is_some() && find_stable(&g, &bg, sa).is_none(), "CR 400.1: Bob's own graveyard");
    let x = g.create_object_from_definition(&creature("Exiled A", 1, 1), alice(), Zone::Exile);
    let y = g.create_object_from_definition(&creature("Exiled B", 1, 1), bob(), Zone::Exile);
    assert!(g.exile.contains(&x) && g.exile.contains(&y), "CR 400.1: shared exile zone");
    g.validate_zone_consistency().unwrap();
}

#[test]
fn cr_400_2_public_and_hidden_zone_classification() {
    for z in [Zone::Graveyard, Zone::Battlefield, Zone::Stack, Zone::Exile, Zone::Ante, Zone::Command] {
        assert!(z.is_public() && !z.is_hidden(), "CR 400.2: {z:?} is public");
    }
    for z in [Zone::Library, Zone::Hand] {
        assert!(z.is_hidden() && !z.is_public(), "CR 400.2: {z:?} is hidden");
    }
}

#[test]
fn cr_400_5_404_2_graveyard_and_library_order_is_preserved() {
    let mut g = main_game();
    let cards = filler(&mut g, alice(), Zone::Battlefield, 4);
    let stables: Vec<_> = cards.iter().map(|id| g.object(*id).unwrap().stable_id).collect();
    for id in &cards {
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(*id), Zone::Graveyard, false)
            .execute(&mut g, &mut EffectContext::new_default(*id, alice()))
            .unwrap();
    }
    let order = |g: &GameState| -> Vec<_> {
        g.player(alice()).unwrap().graveyard.iter().map(|id| g.object(*id).unwrap().stable_id).collect()
    };
    assert_eq!(order(&g), stables, "CR 404.1/400.5: each card goes on top in event order");
    let second = g.player(alice()).unwrap().graveyard[1];
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(second), Zone::Exile, false)
        .execute(&mut g, &mut EffectContext::new_default(second, alice()))
        .unwrap();
    assert_eq!(order(&g), vec![stables[0], stables[2], stables[3]], "CR 400.5: removal does not reorder the graveyard");
    let lib = filler(&mut g, alice(), Zone::Library, 4);
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(lib[1]), Zone::Hand, false)
        .execute(&mut g, &mut EffectContext::new_default(lib[1], alice()))
        .unwrap();
    assert_eq!(
        g.player(alice()).unwrap().library,
        vec![lib[0], lib[2], lib[3]],
        "CR 400.5: library order unchanged except by the instruction"
    );
}

fn darksteel_colossus() -> CardDefinition {
    card(
        "Darksteel Colossus",
        "Mana cost: {11}\nType: Artifact Creature — Golem\nPower/Toughness: 11/11\nTrample, indestructible\nIf Darksteel Colossus would be put into a graveyard from anywhere, reveal Darksteel Colossus and shuffle it into its owner's library instead.",
    )
}

#[test]
fn cr_400_6_moving_objects_own_replacement_applies_even_from_hidden_zone() {
    for origin in [Zone::Hand, Zone::Library, Zone::Battlefield] {
        let mut g = main_game();
        filler(&mut g, alice(), Zone::Library, 3);
        let colossus = g.create_object_from_definition(&darksteel_colossus(), alice(), origin);
        let stable = g.object(colossus).unwrap().stable_id;
        g.refresh_continuous_state().unwrap();
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(colossus), Zone::Graveyard, false)
            .execute(&mut g, &mut EffectContext::new_default(colossus, bob()))
            .unwrap();
        let gy = g.player(alice()).unwrap().graveyard.clone();
        let lib = g.player(alice()).unwrap().library.clone();
        assert!(
            find_stable(&g, &gy, stable).is_none() && find_stable(&g, &lib, stable).is_some(),
            "CR 400.6: the object's own replacement effect applies to its move from {origin:?}"
        );
    }
}

#[test]
fn cr_403_1_404_1_battlefield_graveyards_and_stack_start_empty() {
    let g = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    assert!(g.battlefield.is_empty(), "CR 403.1: battlefield starts empty");
    assert!(g.stack.is_empty() && g.exile.is_empty());
    for p in 0..3 {
        assert!(g.player(PlayerId::from_index(p)).unwrap().graveyard.is_empty(), "CR 404.1: graveyard starts empty");
    }
}

#[test]
fn cr_403_4_entering_the_battlefield_creates_a_new_object() {
    let mut g = main_game();
    let c = g.create_object_from_definition(&creature("Blinked", 2, 2), alice(), Zone::Battlefield);
    let stable = g.object(c).unwrap().stable_id;
    g.add_counters(c, CounterType::PlusOnePlusOne, 2);
    ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificObject(c))
        .execute(&mut g, &mut EffectContext::new_default(c, alice()))
        .unwrap();
    ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(c),
        ironsmith::continuous::Modification::ModifyPowerToughness { power: 3, toughness: 3 },
        Until::EndOfTurn,
    )
    .execute(&mut g, &mut EffectContext::new_default(c, alice()))
    .unwrap();
    for (zone, label) in [(Zone::Exile, "exile"), (Zone::Battlefield, "battlefield")] {
        let id = find_stable(&g, &g.exile.clone(), stable)
            .or_else(|| find_stable(&g, &g.battlefield.clone(), stable))
            .unwrap();
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(id), zone, false)
            .execute(&mut g, &mut EffectContext::new_default(id, alice()))
            .unwrap();
        let _ = label;
    }
    let new = find_stable(&g, &g.battlefield.clone(), stable).expect("returned");
    assert_ne!(new, c, "CR 403.4: a new object");
    assert_eq!(
        (g.counter_count(new, CounterType::PlusOnePlusOne), g.damage_on(new), g.current_power(new)),
        (0, 0, Some(2)),
        "CR 403.4: no relation to the previous permanent"
    );
}

#[test]
fn cr_403_2_global_effects_check_only_the_battlefield() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let wrath = card("Audit Wrath", "Mana cost: {0}\nType: Sorcery\nDestroy all creatures.");
    let anthem = card("Audit Anthem", "Mana cost: {0}\nType: Enchantment\nCreatures you control get +1/+1.");
    g.create_object_from_definition(&anthem, alice(), Zone::Battlefield);
    let on_bf = g.create_object_from_definition(&creature("On battlefield", 2, 2), alice(), Zone::Battlefield);
    let in_hand = g.create_object_from_definition(&creature("In hand", 2, 2), alice(), Zone::Hand);
    let in_gy = g.create_object_from_definition(&creature("In graveyard", 2, 2), alice(), Zone::Graveyard);
    let in_exile = g.create_object_from_definition(&creature("In exile", 2, 2), alice(), Zone::Exile);
    let in_lib = g.create_object_from_definition(&creature("In library", 2, 2), alice(), Zone::Library);
    g.refresh_continuous_state().unwrap();
    assert_eq!(g.current_power(on_bf), Some(3));
    assert_eq!(g.current_power(in_hand), Some(2), "CR 403.2: anthem does not affect a card in hand");
    assert_eq!(g.current_power(in_gy), Some(2), "CR 403.2: anthem does not affect a graveyard card");
    let w = g.create_object_from_definition(&wrath, alice(), Zone::Hand);
    cast(&mut g, &mut q, w);
    settle(&mut g, &mut q);
    assert!(!g.battlefield.contains(&on_bf), "battlefield creature destroyed");
    for (id, zone) in [(in_hand, Zone::Hand), (in_gy, Zone::Graveyard), (in_exile, Zone::Exile), (in_lib, Zone::Library)] {
        assert_eq!(g.object(id).map(|o| o.zone), Some(zone), "CR 403.2: Destroy all creatures ignores {zone:?}");
    }
}

#[test]
fn cr_404_1_moved_objects_go_on_top_of_their_owners_graveyard() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    // destroyed (Bob's creature) then the resolved sorcery on top of Alice's graveyard
    let victim = g.create_object_from_definition(&creature("Bob victim", 2, 2), bob(), Zone::Battlefield);
    let vs = g.object(victim).unwrap().stable_id;
    let kill = card("Audit Murder", "Mana cost: {0}\nType: Sorcery\nDestroy target creature.");
    let k = g.create_object_from_definition(&kill, alice(), Zone::Hand);
    let ks = g.object(k).unwrap().stable_id;
    cast(&mut g, &mut q, k);
    settle(&mut g, &mut q);
    let top = |g: &GameState, p: PlayerId| g.player(p).unwrap().graveyard.last().map(|id| g.object(*id).unwrap().stable_id);
    assert_eq!(top(&g, bob()), Some(vs), "CR 404.1: destroyed creature on top of owner's graveyard");
    assert_eq!(top(&g, alice()), Some(ks), "CR 404.1: finished sorcery on top of caster's graveyard");
    // sacrificed and discarded
    let fodder = g.create_object_from_definition(&creature("Sac fodder", 1, 1), alice(), Zone::Battlefield);
    let fs = g.object(fodder).unwrap().stable_id;
    let edict = card("Audit Edict", "Mana cost: {0}\nType: Instant\nYou sacrifice a creature.");
    let e = g.create_object_from_definition(&edict, alice(), Zone::Hand);
    cast(&mut g, &mut q, e);
    pass_round(&mut g, &mut q, &mut SelectFirstDecisionMaker);
    let gy = g.player(alice()).unwrap().graveyard.clone();
    let pos = |s| gy.iter().position(|id| g.object(*id).unwrap().stable_id == s);
    assert!(pos(fs).is_some(), "CR 404.1: sacrificed creature in owner's graveyard");
    let disc_card = g.create_object_from_definition(&creature("Discarded", 1, 1), bob(), Zone::Hand);
    let ds = g.object(disc_card).unwrap().stable_id;
    let discard = card("Audit Discard", "Mana cost: {0}\nType: Sorcery\nTarget player discards a card.");
    let dsp = g.create_object_from_definition(&discard, alice(), Zone::Hand);
    struct TargetBob;
    impl DecisionMaker for TargetBob {
        fn decide_targets(&mut self, _: &GameState, c: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> {
            vec![ironsmith::Target::Player(PlayerId::from_index(1))]
        }
        fn decide_objects(&mut self, _: &GameState, c: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            c.candidates.iter().filter(|x| x.legal).map(|x| x.id).take(c.min.max(1)).collect()
        }
    }
    cast_with(&mut g, &mut q, dsp, &mut TargetBob);
    settle_with(&mut g, &mut q, &mut TargetBob);
    let bob_top = g.player(bob()).unwrap().graveyard.last().map(|id| g.object(*id).unwrap().stable_id);
    assert_eq!(bob_top, Some(ds), "CR 404.1: discarded card on top of owner's graveyard");
    // countered
    let bear = g.create_object_from_definition(&card("Audit Bear", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2"), alice(), Zone::Hand);
    let bs = g.object(bear).unwrap().stable_id;
    let counter = card("Audit Counterspell", "Mana cost: {0}\nType: Instant\nCounter target spell.");
    let cs = g.create_object_from_definition(&counter, alice(), Zone::Hand);
    let css = g.object(cs).unwrap().stable_id;
    cast(&mut g, &mut q, bear);
    cast(&mut g, &mut q, cs);
    settle(&mut g, &mut q);
    let gy = g.player(alice()).unwrap().graveyard.clone();
    let st: Vec<_> = gy.iter().map(|id| g.object(*id).unwrap().stable_id).collect();
    assert_eq!(&st[st.len() - 2..], &[bs, css], "CR 404.1: countered spell then the finished counterspell on top");
}

#[test]
fn cr_405_2_405_5_500_2_505_2_stack_lifo_and_empty_stack_passes_end_the_phase() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let mut d = SelectFirstDecisionMaker;
    let one = card("Audit Gain One", "Mana cost: {0}\nType: Instant\nYou gain 1 life.");
    let two = card("Audit Gain Two", "Mana cost: {0}\nType: Instant\nYou gain 2 life.");
    let a = g.create_object_from_definition(&one, alice(), Zone::Hand);
    let b = g.create_object_from_definition(&two, alice(), Zone::Hand);
    let sa = cast(&mut g, &mut q, a);
    let sb = cast(&mut g, &mut q, b);
    assert_eq!(g.stack.iter().map(|e| e.object_id).collect::<Vec<_>>(), vec![sa, sb], "CR 405.2: each new object goes on top");
    pass_round(&mut g, &mut q, &mut d);
    assert_eq!(g.player(alice()).unwrap().life, 22, "CR 405.5: only the top object resolves");
    assert_eq!(g.stack.len(), 1);
    pass_round(&mut g, &mut q, &mut d);
    assert_eq!(g.player(alice()).unwrap().life, 23);
    assert!(g.stack.is_empty());
    assert_eq!(g.turn.phase, Phase::FirstMain, "CR 500.2: stack becoming empty does not end the phase");
    assert_eq!(g.turn.priority_player, Some(alice()), "CR 500.2: active player gets priority again");
    let mut s = PriorityLoopState::new(2);
    let first = ironsmith::game_loop::apply_priority_response_with_dm(&mut g, &mut q, &mut s, &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut d).unwrap();
    assert!(!matches!(first, GameProgress::Continue), "CR 500.2: one pass is not all players passing");
    let second = ironsmith::game_loop::apply_priority_response_with_dm(&mut g, &mut q, &mut s, &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut d).unwrap();
    assert!(matches!(second, GameProgress::Continue), "CR 405.5/505.2: all pass on empty stack ends the main phase");
}

fn grove() -> CardDefinition {
    card("Audit Grove", "Type: Land\n{T}: Add {G}.")
}

#[test]
fn cr_405_6c_405_6d_mana_abilities_and_land_plays_skip_the_stack() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let mut d = SelectFirstDecisionMaker;
    let land = g.create_object_from_definition(&grove(), alice(), Zone::Hand);
    let play = compute_legal_actions(&g, alice()).unwrap().into_iter()
        .find(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == land)).expect("land play available");
    let mut s = PriorityLoopState::new(2);
    ironsmith::game_loop::apply_priority_response_with_dm(&mut g, &mut q, &mut s, &PriorityResponse::PriorityAction(play), &mut d).unwrap();
    let on_bf = g.battlefield.iter().copied().find(|id| g.object(*id).unwrap().name.as_ref() == "Audit Grove").expect("CR 305/116: land on battlefield");
    assert!(g.stack.is_empty(), "CR 405.6d: special action does not use the stack");
    assert_eq!(g.turn.priority_player, Some(alice()), "CR 116: player keeps priority after special action");
    let mana = compute_legal_actions(&g, alice()).unwrap().into_iter()
        .find(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == on_bf)).expect("mana ability available");
    ironsmith::game_loop::apply_priority_response_with_dm(&mut g, &mut q, &mut s, &PriorityResponse::PriorityAction(mana), &mut d).unwrap();
    assert!(g.stack.is_empty(), "CR 405.6c: mana ability does not use the stack");
    assert_eq!(g.player(alice()).unwrap().mana_pool.green, 1, "CR 405.6c: mana produced immediately");
    assert_eq!(g.turn.priority_player, Some(alice()), "CR 405.6c: priority returns to the activator");
}

#[test]
fn cr_505_6b_land_play_requires_main_phase_empty_stack_priority_and_unused_allowance() {
    let land_action = |g: &GameState, p: PlayerId| {
        compute_legal_actions(g, p).unwrap().into_iter().any(|a| matches!(a, LegalAction::PlayLand { .. }))
    };
    // main phase, empty stack, priority: allowed; second land: not allowed
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let mut d = SelectFirstDecisionMaker;
    let l1 = g.create_object_from_definition(&grove(), alice(), Zone::Hand);
    g.create_object_from_definition(&grove(), alice(), Zone::Hand);
    g.create_object_from_definition(&grove(), bob(), Zone::Hand);
    assert!(land_action(&g, alice()), "CR 505.6b: land play legal in main phase");
    let mut s = PriorityLoopState::new(2);
    ironsmith::game_loop::apply_priority_response_with_dm(&mut g, &mut q, &mut s, &PriorityResponse::PriorityAction(LegalAction::PlayLand { land_id: l1 }), &mut d).unwrap();
    assert!(!land_action(&g, alice()), "CR 505.6b: only one land per turn");
    // stack not empty
    let mut g = main_game();
    g.create_object_from_definition(&grove(), alice(), Zone::Hand);
    let inst = g.create_object_from_definition(&card("Audit Gain One", "Mana cost: {0}\nType: Instant\nYou gain 1 life."), alice(), Zone::Hand);
    cast(&mut g, &mut q, inst);
    assert_eq!(g.turn.priority_player, Some(alice()));
    assert!(!land_action(&g, alice()), "CR 505.6b: stack must be empty");
    // upkeep
    let mut g = main_game();
    g.turn.phase = Phase::Beginning;
    g.turn.step = Some(Step::Upkeep);
    g.create_object_from_definition(&grove(), alice(), Zone::Hand);
    assert!(!land_action(&g, alice()), "CR 505.6b: only during a main phase");
    // nonactive player with priority in main phase
    let mut g = main_game();
    g.turn.priority_player = Some(bob());
    g.create_object_from_definition(&grove(), bob(), Zone::Hand);
    assert!(!land_action(&g, bob()), "CR 505.6b: only the active player");
}

#[test]
fn cr_505_6a_sorcery_speed_spells_only_in_active_players_main_phase_with_empty_stack() {
    let bear = || card("Audit Bear", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let can_cast = |g: &GameState, p: PlayerId, id: ObjectId| {
        compute_legal_actions(g, p).unwrap().into_iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if spell_id == id))
    };
    for phase in [Phase::FirstMain, Phase::NextMain] {
        let mut g = main_game();
        g.turn.phase = phase;
        let b = g.create_object_from_definition(&bear(), alice(), Zone::Hand);
        assert!(can_cast(&g, alice(), b), "CR 505.6a: active player may cast creature spell in {phase:?}");
    }
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let b = g.create_object_from_definition(&bear(), alice(), Zone::Hand);
    let inst = g.create_object_from_definition(&card("Audit Gain One", "Mana cost: {0}\nType: Instant\nYou gain 1 life."), alice(), Zone::Hand);
    cast(&mut g, &mut q, inst);
    assert!(!can_cast(&g, alice(), b), "CR 505.6a/307.1: not while the stack is nonempty");
    let mut g = main_game();
    g.turn.phase = Phase::Beginning;
    g.turn.step = Some(Step::Upkeep);
    let b = g.create_object_from_definition(&bear(), alice(), Zone::Hand);
    assert!(!can_cast(&g, alice(), b), "CR 505.6a: not outside a main phase");
    let mut g = main_game();
    g.turn.priority_player = Some(bob());
    let b = g.create_object_from_definition(&bear(), bob(), Zone::Hand);
    assert!(!can_cast(&g, bob(), b), "CR 505.6a: nonactive player cannot");
}

#[test]
fn cr_405_6b_static_abilities_apply_without_using_the_stack() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let c = g.create_object_from_definition(&creature("Anthem subject", 2, 2), alice(), Zone::Battlefield);
    let anthem = card("Audit Anthem", "Mana cost: {0}\nType: Enchantment\nCreatures you control get +1/+1.");
    let a = g.create_object_from_definition(&anthem, alice(), Zone::Hand);
    cast(&mut g, &mut q, a);
    pass_round(&mut g, &mut q, &mut SelectFirstDecisionMaker);
    assert!(g.stack.is_empty(), "CR 405.6b: no stack object for the static ability");
    assert_eq!(g.current_power(c), Some(3), "CR 405.6b: static effect applies immediately");
}

#[test]
fn cr_405_6f_state_based_actions_do_not_use_the_stack() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let c = g.create_object_from_definition(&creature("Doomed", 2, 2), alice(), Zone::Battlefield);
    ironsmith::effects::DealDamageEffect::new(2, ChooseSpec::SpecificObject(c))
        .execute(&mut g, &mut EffectContext::new_default(c, alice()))
        .unwrap();
    let progress = ironsmith::game_loop::advance_priority_with_dm(&mut g, &mut q, &mut SelectFirstDecisionMaker).unwrap();
    assert!(matches!(progress, GameProgress::NeedsDecisionCtx(_)));
    assert!(g.object(c).is_none() && g.stack.is_empty(), "CR 405.6f/704.3: SBA performed before priority without the stack");
    assert_eq!(g.player(alice()).unwrap().graveyard.len(), 1);
}

#[test]
fn cr_405_6e_turn_based_draw_happens_without_the_stack() {
    let mut g = main_game();
    g.turn.phase = Phase::Beginning;
    g.turn.step = Some(Step::Draw);
    filler(&mut g, alice(), Zone::Library, 3);
    let mut q = TriggerQueue::new();
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Draw);
    let mut got_priority = false;
    for _ in 0..8 {
        match runner.advance(&mut g, &mut q).unwrap() {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                got_priority = true;
                break;
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(got_priority);
    assert_eq!(g.player(alice()).unwrap().hand.len(), 1, "CR 405.6e/504.1: draw happened as the step began");
    assert!(g.stack.is_empty(), "CR 405.6e: turn-based action does not use the stack");
}

// ---------------------------------------------------------------------------
// 500-505: turn structure
// ---------------------------------------------------------------------------

fn turn_game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = alice();
    g.turn.priority_player = None;
    g.turn.phase = Phase::Beginning;
    g.turn.step = None;
    g.turn.turn_number = 2;
    filler(&mut g, alice(), Zone::Library, 6);
    filler(&mut g, bob(), Zone::Library, 6);
    g
}

fn pos(windows: &[(Phase, Option<Step>)], w: (Phase, Option<Step>)) -> usize {
    windows.iter().position(|x| *x == w).unwrap_or_else(|| panic!("window {w:?} missing in {windows:?}"))
}

#[test]
fn cr_500_1_501_1_505_1_505_6_turn_runs_phases_and_steps_in_order() {
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let mut main_priority = Vec::new();
    let w = run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        if matches!(g.turn.phase, Phase::FirstMain | Phase::NextMain) {
            main_priority.push(g.turn.priority_player);
        }
    });
    assert!(!w.iter().any(|x| x.1 == Some(Step::Untap) || x.1 == Some(Step::Cleanup)), "CR 500.3/502.4: no priority in untap or ordinary cleanup: {w:?}");
    assert_eq!(w[0], (Phase::Beginning, Some(Step::Upkeep)), "CR 501.1: first priority is upkeep (untap gives none)");
    let order = [
        (Phase::Beginning, Some(Step::Upkeep)),
        (Phase::Beginning, Some(Step::Draw)),
        (Phase::FirstMain, None),
        (Phase::Combat, Some(Step::BeginCombat)),
        (Phase::Combat, Some(Step::EndCombat)),
        (Phase::NextMain, None),
        (Phase::Ending, Some(Step::End)),
    ];
    let idx: Vec<_> = order.iter().map(|o| pos(&w, *o)).collect();
    assert!(idx.windows(2).all(|p| p[0] < p[1]), "CR 500.1/501.1/505.1: phase order {w:?}");
    assert_eq!(main_priority, vec![Some(alice()), Some(alice())], "CR 505.6: active player receives priority in each main phase");
}

#[test]
fn cr_502_4_503_1a_500_6_untap_triggers_wait_for_upkeep_priority() {
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let inspired = card("Audit Inspired", "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1\nWhenever this creature becomes untapped, you gain 1 life.");
    let upkeep = card("Audit Upkeep Gain", "Mana cost: {1}\nType: Enchantment\nAt the beginning of your upkeep, you gain 2 life.");
    let c = g.create_object_from_definition(&inspired, alice(), Zone::Battlefield);
    g.create_object_from_definition(&upkeep, alice(), Zone::Battlefield);
    g.tap(c);
    let mut first = None;
    run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        if first.is_none() {
            first = Some((g.turn.step, g.stack.len(), g.player(alice()).unwrap().life, g.is_tapped(c)));
        }
    });
    assert_eq!(
        first,
        Some((Some(Step::Upkeep), 2, 20, false)),
        "CR 502.4/503.1a/500.6: untap trigger held and put on the stack with the upkeep trigger before AP priority"
    );
    assert_eq!(g.player(alice()).unwrap().life, 23);
}

#[test]
fn cr_500_5a_until_end_of_combat_expires_at_end_of_combat_phase() {
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let c = g.create_object_from_definition(&creature("Combat buff subject", 2, 2), alice(), Zone::Battlefield);
    let mut seen = Vec::new();
    run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        if g.turn.step == Some(Step::BeginCombat) {
            ApplyContinuousEffect::new(
                ironsmith::continuous::EffectTarget::Specific(c),
                ironsmith::continuous::Modification::ModifyPowerToughness { power: 2, toughness: 2 },
                Until::EndOfCombat,
            )
            .execute(g, &mut EffectContext::new_default(c, alice()))
            .unwrap();
        }
        seen.push((g.turn.phase, g.turn.step, g.current_power(c)));
    });
    let at = |p: Phase, s: Option<Step>| seen.iter().find(|x| x.0 == p && x.1 == s).map(|x| x.2);
    assert_eq!(at(Phase::Combat, Some(Step::EndCombat)), Some(Some(4)), "CR 500.5a: still active during end of combat step: {seen:?}");
    assert_eq!(at(Phase::NextMain, None), Some(Some(2)), "CR 500.5a: expired once the combat phase ended");
}

#[test]
fn cr_500_4_until_next_end_step_expires_as_that_step_begins() {
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let c = g.create_object_from_definition(&creature("Step buff subject", 2, 2), alice(), Zone::Battlefield);
    let mut seen = Vec::new();
    run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        if g.turn.phase == Phase::FirstMain {
            ApplyContinuousEffect::new(
                ironsmith::continuous::EffectTarget::Specific(c),
                ironsmith::continuous::Modification::ModifyPowerToughness { power: 2, toughness: 2 },
                Until::NextEndStep,
            )
            .execute(g, &mut EffectContext::new_default(c, alice()))
            .unwrap();
        }
        seen.push((g.turn.phase, g.turn.step, g.current_power(c)));
    });
    let at = |p: Phase, s: Option<Step>| seen.iter().find(|x| x.0 == p && x.1 == s).map(|x| x.2);
    assert_eq!(at(Phase::NextMain, None), Some(Some(4)));
    assert_eq!(at(Phase::Ending, Some(Step::End)), Some(Some(2)), "CR 500.4: effect lasting until the end step expires as it begins: {seen:?}");
}

#[test]
fn cr_500_11_skipped_step_is_passed_as_though_it_did_not_exist() {
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let draw_trigger = card("Audit Draw Gain", "Mana cost: {1}\nType: Enchantment\nAt the beginning of your draw step, you gain 1 life.");
    g.create_object_from_definition(&draw_trigger, alice(), Zone::Battlefield);
    g.skip_next_step(alice(), Step::Draw);
    let w = run_turn(&mut g, &mut q, TurnRunner::new(), |_, _| {});
    assert!(!w.iter().any(|x| x.1 == Some(Step::Draw)), "CR 500.11: no draw-step priority: {w:?}");
    assert_eq!(
        (g.player(alice()).unwrap().hand.len(), g.player(alice()).unwrap().life),
        (0, 20),
        "CR 500.11/614.10: no draw and no beginning-of-draw-step trigger"
    );
}

#[test]
fn cr_500_9_additional_steps_after_the_same_step_run_newest_first() {
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let mut added = false;
    let w = run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        if !added && g.turn.step == Some(Step::Upkeep) {
            added = true;
            g.add_step_after(Step::Upkeep, Step::Draw); // older
            g.add_step_after(Step::Draw, Step::Draw); // newer
        }
    });
    let beginning: Vec<_> = w.iter().take_while(|x| x.0 == Phase::Beginning).map(|x| x.1).collect();
    assert_eq!(
        beginning,
        vec![Some(Step::Upkeep), Some(Step::Draw), Some(Step::Draw), Some(Step::Upkeep)],
        "CR 500.9: steps added after the draw step run directly after it, most recently created first"
    );
    assert_eq!(g.player(alice()).unwrap().hand.len(), 2, "additional draw step draws");
}

#[test]
fn cr_500_8_additional_phases_follow_the_phase_newest_first() {
    use ironsmith::effects::AdditionalPhasesEffect;
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let src = g.create_object_from_definition(&creature("Phase source", 1, 1), alice(), Zone::Battlefield);
    let mut added = false;
    let w = run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        if !added && g.turn.phase == Phase::FirstMain {
            added = true;
            AdditionalPhasesEffect::combat().execute(g, &mut EffectContext::new_default(src, alice())).unwrap();
            AdditionalPhasesEffect::combat_then_main().execute(g, &mut EffectContext::new_default(src, alice())).unwrap();
        }
    });
    let tokens: Vec<&str> = w
        .iter()
        .filter_map(|x| match x {
            (Phase::FirstMain, _) => Some("M1"),
            (Phase::Combat, Some(Step::BeginCombat)) => Some("C"),
            (Phase::NextMain, _) => Some("M2"),
            (Phase::Ending, Some(Step::End)) => Some("E"),
            _ => None,
        })
        .collect();
    assert_eq!(
        tokens,
        vec!["M1", "C", "M2", "C", "C", "M2", "E"],
        "CR 500.8: the most recently created extra phases occur first, directly after the main phase"
    );
}

#[test]
fn cr_500_10_500_10a_added_upkeep_after_phase_creates_a_beginning_phase_with_only_upkeep() {
    use ironsmith::effects::{AdditionalStepAnchor, AdditionalStepKind, AdditionalStepsEffect};
    for controller in [alice(), bob()] {
        let mut g = turn_game();
        let mut q = TriggerQueue::new();
        let upkeep = card("Audit Upkeep Gain", "Mana cost: {1}\nType: Enchantment\nAt the beginning of your upkeep, you gain 1 life.");
        g.create_object_from_definition(&upkeep, alice(), Zone::Battlefield);
        let tapped = g.create_object_from_definition(&creature("Stays tapped", 1, 1), alice(), Zone::Battlefield);
        let mut added = false;
        let mut extra = Vec::new();
        let w = run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
            if !added && g.turn.phase == Phase::FirstMain {
                added = true;
                g.tap(tapped);
                AdditionalStepsEffect {
                    step: AdditionalStepKind::Upkeep,
                    count: ironsmith::Value::Fixed(1),
                    anchor: AdditionalStepAnchor::AfterThisPhase,
                    player: Some(ironsmith::PlayerFilter::You),
                }
                .execute(g, &mut EffectContext::new_default(tapped, controller))
                .unwrap();
            } else if added && g.turn.step == Some(Step::Upkeep) {
                extra.push((g.player(alice()).unwrap().hand.len(), g.is_tapped(tapped)));
            }
        });
        let main = pos(&w, (Phase::FirstMain, None));
        if controller == alice() {
            assert_eq!(w[main + 1], (Phase::Beginning, Some(Step::Upkeep)), "CR 500.10: beginning phase with upkeep right after the main phase: {w:?}");
            assert_eq!(extra, vec![(1, true)], "CR 500.10: the added phase has no untap or draw step");
            assert!(!w[main + 1..].iter().any(|x| x.1 == Some(Step::Draw)));
            assert_eq!(g.player(alice()).unwrap().life, 22, "beginning-of-upkeep trigger fires in the added upkeep");
        } else {
            assert!(extra.is_empty() && w[main + 1].0 == Phase::Combat, "CR 500.10a: 'you get' on another player's turn adds nothing: {w:?}");
        }
    }
}

#[test]
fn cr_505_1a_505_1b_extra_main_phases_are_postcombat_and_counted_in_order() {
    use ironsmith::effects::AdditionalPhasesEffect;
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let pre = card("Audit Precombat", "Mana cost: {1}\nType: Enchantment\nAt the beginning of your precombat main phase, you gain 100 life.");
    let post = card("Audit Postcombat", "Mana cost: {1}\nType: Enchantment\nAt the beginning of your postcombat main phase, you gain 1 life.");
    let second = card("Audit Second Main", "Mana cost: {1}\nType: Enchantment\nAt the beginning of your second main phase, you gain 10 life.");
    for d in [&pre, &post, &second] {
        g.create_object_from_definition(d, alice(), Zone::Battlefield);
    }
    let src = g.create_object_from_definition(&creature("Phase source", 1, 1), alice(), Zone::Battlefield);
    let mut added = false;
    let mut life_at_main = Vec::new();
    run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        if matches!(g.turn.phase, Phase::FirstMain | Phase::NextMain) {
            life_at_main.push(g.player(alice()).unwrap().life);
        }
        if !added && g.turn.phase == Phase::FirstMain {
            added = true;
            AdditionalPhasesEffect::combat_then_main().execute(g, &mut EffectContext::new_default(src, alice())).unwrap();
        }
    });
    assert_eq!(
        g.player(alice()).unwrap().life,
        20 + 100 + 1 + 1 + 10,
        "CR 505.1a/505.1b: one precombat main, two postcombat mains, and the extra main is the turn's second main phase (life at main windows {life_at_main:?})"
    );
}

#[test]
fn cr_503_2_after_upkeep_spell_is_castable_during_an_additional_upkeep() {
    use ironsmith::effects::{AdditionalStepAnchor, AdditionalStepKind, AdditionalStepsEffect};
    let mut g = turn_game();
    let mut q = TriggerQueue::new();
    let reset = card("Reset", "Mana cost: {0}\nType: Instant\nCast this spell only during an opponent's turn after their upkeep step.\nUntap all lands you control.");
    let r = g.create_object_from_definition(&reset, bob(), Zone::Hand);
    let src = g.create_object_from_definition(&creature("Step source", 1, 1), alice(), Zone::Battlefield);
    let mut upkeeps = 0;
    let mut castable = Vec::new();
    run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        if g.turn.step == Some(Step::Upkeep) {
            upkeeps += 1;
            if upkeeps == 1 {
                AdditionalStepsEffect {
                    step: AdditionalStepKind::Upkeep,
                    count: ironsmith::Value::Fixed(1),
                    anchor: AdditionalStepAnchor::AfterThisStep,
                    player: None,
                }
                .execute(g, &mut EffectContext::new_default(src, alice()))
                .unwrap();
            }
            let saved = g.turn.priority_player;
            g.turn.priority_player = Some(bob());
            castable.push(compute_legal_actions(g, bob()).unwrap().into_iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if spell_id == r)));
            g.turn.priority_player = saved;
        }
    });
    assert_eq!(castable, vec![false, true], "CR 503.2: castable any time after the first upkeep step ends, including a second upkeep step");
}

// ---------------------------------------------------------------------------
// 400.7 exceptions, 400.12, 401, 405.4, 408
// ---------------------------------------------------------------------------

struct TargetPlayer(PlayerId);
impl DecisionMaker for TargetPlayer {
    fn decide_targets(&mut self, _: &GameState, c: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> {
        c.requirements
            .iter()
            .map(|r| {
                let want = ironsmith::Target::Player(self.0);
                if r.legal_targets.contains(&want) { want } else { r.legal_targets[0].clone() }
            })
            .collect()
    }
}

fn spell_permanent(g: &GameState, name: &str) -> Option<ObjectId> {
    g.battlefield.iter().copied().find(|id| g.object(*id).unwrap().name.as_ref() == name)
}

#[test]
fn cr_400_7b_400_7h_permission_granted_haste_applies_to_the_resulting_permanent() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let dragon = card("Thundermane Dragon", "Mana cost: {3}{R}\nType: Creature — Dragon\nPower/Toughness: 4/4\nFlying\nYou may look at the top card of your library any time.\nYou may cast creature spells with power 4 or greater from the top of your library. If you cast a creature spell this way, it gains haste until end of turn.");
    g.create_object_from_definition(&dragon, alice(), Zone::Battlefield);
    filler(&mut g, alice(), Zone::Library, 2);
    let giant = card("Audit Giant", "Mana cost: {0}\nType: Creature — Giant\nPower/Toughness: 4/4");
    let top = g.create_object_from_definition(&giant, alice(), Zone::Library);
    assert_eq!(g.player(alice()).unwrap().library.last(), Some(&top));
    g.refresh_continuous_state().unwrap();
    cast(&mut g, &mut q, top);
    settle(&mut g, &mut q);
    let perm = spell_permanent(&g, "Audit Giant").expect("giant resolved");
    assert!(
        g.object_has_ability(perm, &ironsmith::StaticAbility::haste()),
        "CR 400.7b/400.7h/611.3d: haste from the cast permission applies to the permanent the spell becomes"
    );
}

#[test]
fn cr_400_7c_prevention_for_a_permanent_spell_applies_to_the_permanent() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let bear = g.create_object_from_definition(&card("Audit Bear", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2"), alice(), Zone::Hand);
    let hallow = g.create_object_from_definition(&card("Hallow", "Mana cost: {0}\nType: Instant\nPrevent all damage target spell would deal this turn. You gain life equal to the damage prevented this way."), alice(), Zone::Hand);
    cast(&mut g, &mut q, bear);
    cast(&mut g, &mut q, hallow);
    settle(&mut g, &mut q);
    let perm = spell_permanent(&g, "Audit Bear").expect("bear resolved");
    ironsmith::effects::DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(bob()))
        .execute(&mut g, &mut EffectContext::new_default(perm, alice()))
        .unwrap();
    assert_eq!(g.player(bob()).unwrap().life, 20, "CR 400.7c: damage from the permanent the spell became is prevented");
}

#[test]
fn cr_400_7d_permanent_reads_mana_spent_on_its_spell() {
    for white in [3u32, 1] {
        let mut g = main_game();
        let mut q = TriggerQueue::new();
        let paladin = card("Ardenvale Paladin", "Mana cost: {3}{W}\nType: Creature — Human Knight\nPower/Toughness: 2/5\nAdamant — If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it.");
        let p = g.create_object_from_definition(&paladin, alice(), Zone::Hand);
        {
            let pool = &mut g.player_mut(alice()).unwrap().mana_pool;
            pool.white = white;
            pool.colorless = 4 - white;
        }
        cast(&mut g, &mut q, p);
        settle(&mut g, &mut q);
        let perm = spell_permanent(&g, "Ardenvale Paladin").expect("resolved");
        assert_eq!(
            g.counter_count(perm, CounterType::PlusOnePlusOne),
            u32::from(white == 3),
            "CR 400.7d: the permanent references mana spent to cast its spell (white={white})"
        );
    }
}

#[test]
fn cr_400_7f_dies_trigger_finds_the_aura_put_into_the_graveyard_by_sba() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let host = g.create_object_from_definition(&creature("Destiny host", 1, 1), alice(), Zone::Battlefield);
    let destiny = card("Angelic Destiny", "Mana cost: {0}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +4/+4, has flying and first strike, and is an Angel in addition to its other types.\nWhen enchanted creature dies, return Angelic Destiny to its owner's hand.");
    let aura = g.create_object_from_definition(&destiny, alice(), Zone::Hand);
    let aura_stable = g.object(aura).unwrap().stable_id;
    cast(&mut g, &mut q, aura);
    settle(&mut g, &mut q);
    assert_eq!(g.current_power(host), Some(5), "aura attached");
    let murder = g.create_object_from_definition(&card("Audit Murder", "Mana cost: {0}\nType: Instant\nDestroy target creature."), alice(), Zone::Hand);
    cast(&mut g, &mut q, murder);
    settle(&mut g, &mut q);
    let hand = g.player(alice()).unwrap().hand.clone();
    assert!(
        find_stable(&g, &hand, aura_stable).is_some(),
        "CR 400.7f: the leaves trigger finds the Aura that went to the graveyard as an SBA and returns it"
    );
}

#[test]
fn cr_400_7g_granted_flashback_keeps_applying_to_the_spell_on_the_stack() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let gain = g.create_object_from_definition(&card("Audit Gain Two", "Mana cost: {0}\nType: Sorcery\nYou gain 2 life."), alice(), Zone::Graveyard);
    let stable = g.object(gain).unwrap().stable_id;
    let pif = g.create_object_from_definition(&card("Past in Flames", "Mana cost: {0}\nType: Sorcery\nEach instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.\nFlashback {4}{R}"), alice(), Zone::Hand);
    cast(&mut g, &mut q, pif);
    settle(&mut g, &mut q);
    let gy = g.player(alice()).unwrap().graveyard.clone();
    let card_now = find_stable(&g, &gy, stable).expect("still in graveyard");
    cast(&mut g, &mut q, card_now);
    settle(&mut g, &mut q);
    assert_eq!(g.player(alice()).unwrap().life, 22);
    assert!(
        find_stable(&g, &g.exile.clone(), stable).is_some(),
        "CR 400.7g/702.34a: the granted flashback still exiles the card after it resolves"
    );
}

#[test]
fn cr_400_7i_effect_finds_the_land_played_with_its_permission() {
    let inquisitor = card("Lightstall Inquisitor", "Mana cost: {W}\nType: Creature — Angel Wizard\nPower/Toughness: 2/1\nVigilance\nWhen this creature enters, each opponent exiles a card from their hand and may play that card for as long as it remains exiled. Each spell cast this way costs {1} more to cast. Each land played this way enters tapped.");
    let mut offered = Vec::new();
    for land_case in [false, true] {
        let mut g = main_game();
        let mut q = TriggerQueue::new();
        let def = if land_case { grove() } else { card("Audit Bear", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2") };
        let exiled_card = g.create_object_from_definition(&def, alice(), Zone::Hand);
        let stable = g.object(exiled_card).unwrap().stable_id;
        g.player_mut(alice()).unwrap().mana_pool.colorless = 1;
        let inq = g.create_object_from_definition(&inquisitor, bob(), Zone::Hand);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(inq), Zone::Battlefield, false)
            .execute(&mut g, &mut EffectContext::new_default(inq, bob()))
            .unwrap();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut q).unwrap();
        settle(&mut g, &mut q);
        let exiled = find_stable(&g, &g.exile.clone(), stable).expect("card exiled by the trigger");
        g.turn.priority_player = Some(alice());
        let action = compute_legal_actions(&g, alice()).unwrap().into_iter().find(|a| match a {
            LegalAction::PlayLand { land_id } => *land_id == exiled,
            LegalAction::CastSpell { spell_id, .. } => *spell_id == exiled,
            _ => false,
        });
        offered.push(action.is_some());
        if land_case {
            let play = action.unwrap_or_else(|| panic!("CR 400.7i: the effect allows this land card to be played (permission offered: nonland={:?}, land=false)", offered[0]));
            let mut s = PriorityLoopState::new(2);
            ironsmith::game_loop::apply_priority_response_with_dm(&mut g, &mut q, &mut s, &PriorityResponse::PriorityAction(play), &mut SelectFirstDecisionMaker).unwrap();
            let on_bf = find_stable(&g, &g.battlefield.clone(), stable).expect("land played");
            assert!(g.is_tapped(on_bf), "CR 400.7i: the permission effect finds the land it allowed to be played (enters tapped)");
        }
    }
}

#[test]
fn cr_400_7j_effect_and_cost_find_objects_moved_to_public_zones() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let c = g.create_object_from_definition(&creature("Shifted", 2, 2), alice(), Zone::Battlefield);
    let stable = g.object(c).unwrap().stable_id;
    let shift = g.create_object_from_definition(&card("Cloudshift", "Mana cost: {0}\nType: Instant\nExile target creature you control, then return that card to the battlefield under your control."), alice(), Zone::Hand);
    cast(&mut g, &mut q, shift);
    settle(&mut g, &mut q);
    let back = find_stable(&g, &g.battlefield.clone(), stable);
    assert!(back.is_some_and(|id| id != c), "CR 400.7j: the effect finds the card it exiled");
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    g.create_object_from_definition(&creature("Flung", 3, 3), alice(), Zone::Battlefield);
    let fling = g.create_object_from_definition(&card("Fling", "Mana cost: {0}\nType: Instant\nAs an additional cost to cast this spell, sacrifice a creature.\nFling deals damage equal to the sacrificed creature's power to any target."), alice(), Zone::Hand);
    let mut d = TargetPlayer(bob());
    cast_with(&mut g, &mut q, fling, &mut d);
    settle_with(&mut g, &mut q, &mut d);
    assert_eq!(g.player(bob()).unwrap().life, 17, "CR 400.7j: the spell's effect finds the object its cost sacrificed");
}

#[test]
fn cr_400_7k_discarded_madness_card_put_into_graveyard_is_found_by_discard_trigger() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let watcher = card("Audit Discard Watcher", "Mana cost: {0}\nType: Enchantment\nWhenever a player discards a card, exile that card.");
    g.create_object_from_definition(&watcher, alice(), Zone::Battlefield);
    // control: an ordinary discarded card is found and exiled by the watcher
    {
        let mut g = main_game();
        let mut q = TriggerQueue::new();
        let watcher = card("Audit Discard Watcher", "Mana cost: {0}\nType: Enchantment\nWhenever a player discards a card, exile that card.");
        g.create_object_from_definition(&watcher, alice(), Zone::Battlefield);
        let plain = g.create_object_from_definition(&creature("Plain discard", 1, 1), bob(), Zone::Hand);
        let ps = g.object(plain).unwrap().stable_id;
        let discard = g.create_object_from_definition(&card("Audit Discard", "Mana cost: {0}\nType: Sorcery\nTarget player discards a card."), alice(), Zone::Hand);
        let mut d = TargetPlayer(bob());
        cast_with(&mut g, &mut q, discard, &mut d);
        settle_with(&mut g, &mut q, &mut d);
        assert!(find_stable(&g, &g.exile.clone(), ps).is_some(), "control: watcher exiles an ordinary discarded card");
    }
    let alms = card("Alms of the Vein", "Mana cost: {2}{B}\nType: Sorcery\nTarget opponent loses 3 life and you gain 3 life.\nMadness {B}");
    let m = g.create_object_from_definition(&alms, bob(), Zone::Hand);
    let stable = g.object(m).unwrap().stable_id;
    let discard = g.create_object_from_definition(&card("Audit Discard", "Mana cost: {0}\nType: Sorcery\nTarget player discards a card."), alice(), Zone::Hand);
    struct Decline;
    impl DecisionMaker for Decline {
        fn decide_targets(&mut self, _: &GameState, c: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> {
            c.requirements.iter().map(|_| ironsmith::Target::Player(PlayerId::from_index(1))).collect()
        }
    }
    let mut d = Decline;
    cast_with(&mut g, &mut q, discard, &mut d);
    settle_with(&mut g, &mut q, &mut d);
    let bgy = g.player(bob()).unwrap().graveyard.clone();
    assert_eq!(g.player(alice()).unwrap().life, 20, "madness cast declined");
    assert!(
        find_stable(&g, &bgy, stable).is_none() && find_stable(&g, &g.exile.clone(), stable).is_some(),
        "CR 400.7k: after the madness trigger puts the uncast card into the graveyard, the discard trigger finds and exiles it (in graveyard: {}, in exile: {}, hand: {})",
        find_stable(&g, &bgy, stable).is_some(),
        find_stable(&g, &g.exile.clone(), stable).is_some(),
        find_stable(&g, &g.player(bob()).unwrap().hand.clone(), stable).is_some()
    );
}

#[test]
fn cr_400_7m_stickers_follow_public_to_public_moves() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    let mut g = main_game();
    let c = g.create_object_from_definition(&creature("Sticker subject", 2, 2), alice(), Zone::Battlefield);
    g.add_accessible_name_sticker(alice(), "Bright");
    PutStickerEffect::new(ChooseSpec::SpecificObject(c), KeywordActionKind::NameSticker)
        .execute(&mut g, &mut EffectContext::new_default(c, alice()))
        .unwrap();
    let mut id = c;
    for zone in [Zone::Graveyard, Zone::Exile, Zone::Command] {
        id = g.move_object_by_effect(id, zone).unwrap();
        assert_eq!(
            (g.sticker_count_on_object(id, KeywordActionKind::Sticker, None), g.current_name(id).as_deref().map(str::to_string)),
            (1, Some("Bright Sticker subject".to_string())),
            "CR 400.7m/123.5: sticker and its effect retained after public move to {zone:?}"
        );
    }
}

#[test]
fn cr_400_12_instruction_on_a_zone_acts_on_all_cards_in_it() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let hand = filler(&mut g, alice(), Zone::Hand, 3);
    filler(&mut g, alice(), Zone::Library, 2);
    let stables: Vec<_> = hand.iter().map(|id| g.object(*id).unwrap().stable_id).collect();
    let spell = g.create_object_from_definition(&card("Audit Shuffle Hand", "Mana cost: {0}\nType: Sorcery\nShuffle your hand into your library."), alice(), Zone::Hand);
    cast(&mut g, &mut q, spell);
    settle(&mut g, &mut q);
    let lib = g.player(alice()).unwrap().library.clone();
    assert!(g.player(alice()).unwrap().hand.is_empty(), "CR 400.12: every card in the hand is moved");
    assert_eq!(lib.len(), 5);
    assert!(stables.iter().all(|s| find_stable(&g, &lib, *s).is_some()));
}

#[test]
fn cr_401_5_new_top_card_is_not_revealed_until_the_spell_becomes_cast() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let fs = card("Future Sight", "Mana cost: {2}{U}{U}{U}\nType: Enchantment\nPlay with the top card of your library revealed.\nYou may play lands and cast spells from the top of your library.");
    g.create_object_from_definition(&fs, alice(), Zone::Battlefield);
    filler(&mut g, alice(), Zone::Library, 2);
    let top = g.create_object_from_definition(&card("Audit Target Gain", "Mana cost: {0}\nType: Instant\nTarget player gains 1 life."), alice(), Zone::Library);
    g.refresh_continuous_state().unwrap();
    struct Watch(Vec<bool>);
    impl DecisionMaker for Watch {
        fn decide_targets(&mut self, g: &GameState, c: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> {
            self.0.push(g.static_library_top_visible_during_announcements(PlayerId::from_index(0)));
            c.requirements.iter().map(|r| r.legal_targets[0].clone()).collect()
        }
    }
    let mut w = Watch(Vec::new());
    cast_with(&mut g, &mut q, top, &mut w);
    assert!(!w.0.is_empty(), "target choice happened mid-cast");
    assert!(w.0.iter().all(|v| !v), "CR 401.5: the new top card is not revealed while the spell is being cast");
    assert!(
        !g.has_library_top_announcement() || g.static_library_top_visible_during_announcements(alice()),
        "CR 401.5: the new top card is revealed once the spell is cast"
    );
}

#[test]
fn cr_401_6_revealed_top_card_that_stops_being_revealed_becomes_a_new_object() {
    let mut g = main_game();
    let spy = card("Goblin Spy", "Mana cost: {2}{R}\nType: Creature — Goblin Rogue\nPower/Toughness: 1/2\nPlay with the top card of your library revealed.");
    g.create_object_from_definition(&spy, alice(), Zone::Battlefield);
    filler(&mut g, alice(), Zone::Library, 1);
    let revealed = g.create_object_from_definition(&creature("Revealed top", 1, 1), alice(), Zone::Library);
    let stable = g.object(revealed).unwrap().stable_id;
    g.refresh_continuous_state().unwrap();
    let cover = g.create_object_from_definition(&creature("Cover", 1, 1), alice(), Zone::Hand);
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(cover), Zone::Library, true)
        .execute(&mut g, &mut EffectContext::new_default(cover, alice()))
        .unwrap();
    let cover_now = *g.player(alice()).unwrap().library.last().unwrap();
    assert_ne!(cover_now, revealed);
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(cover_now), Zone::Hand, false)
        .execute(&mut g, &mut EffectContext::new_default(cover_now, alice()))
        .unwrap();
    let top = *g.player(alice()).unwrap().library.last().unwrap();
    assert_eq!(g.object(top).unwrap().stable_id, stable);
    assert_ne!(top, revealed, "CR 401.6: the card stopped being revealed, so it must be a new object when revealed again");
}

#[test]
fn cr_401_7_nth_from_top_with_too_few_cards_goes_to_the_bottom() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let only = filler(&mut g, bob(), Zone::Library, 1)[0];
    let victim = g.create_object_from_definition(&creature("Tucked", 2, 2), bob(), Zone::Battlefield);
    let stable = g.object(victim).unwrap().stable_id;
    let tuck = g.create_object_from_definition(&card("Audit Tuck", "Mana cost: {0}\nType: Instant\nPut target creature into its owner's library third from the top."), alice(), Zone::Hand);
    cast(&mut g, &mut q, tuck);
    settle(&mut g, &mut q);
    let lib = g.player(bob()).unwrap().library.clone();
    assert_eq!(lib.len(), 2);
    assert_eq!(g.object(lib[0]).unwrap().stable_id, stable, "CR 401.7: with fewer than N cards the card goes on the bottom");
    assert_eq!(lib[1], only);
}

#[test]
fn cr_405_4_stack_objects_characteristics_and_controllers() {
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let bear = g.create_object_from_definition(&card("Audit Bear", "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/3"), alice(), Zone::Hand);
    g.player_mut(alice()).unwrap().mana_pool.colorless = 1;
    let s = cast(&mut g, &mut q, bear);
    let entry = g.stack.last().unwrap().clone();
    assert_eq!(entry.controller, alice(), "CR 405.4: controller of a spell is its caster");
    assert_eq!((g.current_power(s), g.current_toughness(s), g.object(s).unwrap().name.to_string()), (Some(2), Some(3), "Audit Bear".to_string()), "CR 405.4: spell has the card's characteristics");
    settle(&mut g, &mut q);
    let observer = card("Audit Death Watch", "Mana cost: {0}\nType: Enchantment\nWhenever a creature dies, you gain 1 life.");
    let o = g.create_object_from_definition(&observer, bob(), Zone::Battlefield);
    ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(o),
        ironsmith::continuous::Modification::ChangeController(alice()),
        Until::EndOfTurn,
    )
    .execute(&mut g, &mut EffectContext::new_default(o, alice()))
    .unwrap();
    let victim = g.create_object_from_definition(&creature("Dies", 1, 1), bob(), Zone::Battlefield);
    ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificObject(victim))
        .execute(&mut g, &mut EffectContext::new_default(victim, bob()))
        .unwrap();
    ironsmith::game_loop::advance_priority_with_dm(&mut g, &mut q, &mut SelectFirstDecisionMaker).unwrap();
    let trig = g.stack.last().expect("death trigger on stack").clone();
    assert!(trig.is_ability);
    assert_eq!(trig.controller, alice(), "CR 405.4: triggered ability controlled by the source's controller when it triggered");
    settle(&mut g, &mut q);
    assert_eq!((g.player(alice()).unwrap().life, g.player(bob()).unwrap().life), (21, 20));
}

#[test]
fn cr_408_1_command_zone_objects_are_not_permanents_and_cannot_be_destroyed() {
    let mut g = main_game();
    let c = g.create_object_from_definition(&creature("Command object", 2, 2), alice(), Zone::Command);
    ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(c))
        .execute(&mut g, &mut EffectContext::new_default(c, alice()))
        .unwrap();
    assert_eq!(g.object(c).map(|o| o.zone), Some(Zone::Command), "CR 408.1: objects in the command zone cannot be destroyed");
    assert!(!g.battlefield.contains(&c), "CR 403.3: not a permanent");
}

#[test]
fn cr_505_3_archenemy_sets_a_scheme_in_motion_only_in_its_precombat_main_phase() {
    use ironsmith::effects::AdditionalPhasesEffect;
    use ironsmith::{ArchenemyVariant, Supertype};
    let mut g = turn_game();
    let deck = (0..20)
        .map(|i| {
            CardDefinitionBuilder::new(CardId::new(), format!("Audit scheme {i}"))
                .card_types(vec![CardType::Scheme])
                .supertypes(vec![Supertype::Ongoing])
                .build()
        })
        .collect();
    g.enable_archenemy(ArchenemyVariant::Default, vec![(alice(), deck)]).unwrap();
    let src = g.create_object_from_definition(&creature("Phase source", 1, 1), alice(), Zone::Battlefield);
    let mut q = TriggerQueue::new();
    let mut seen = Vec::new();
    let mut added = false;
    run_turn(&mut g, &mut q, TurnRunner::new(), |g, _| {
        seen.push((g.turn.phase, g.turn.step, g.scheme_deck(alice()).unwrap().len()));
        if !added && g.turn.phase == Phase::FirstMain {
            added = true;
            AdditionalPhasesEffect::combat_then_main().execute(g, &mut EffectContext::new_default(src, alice())).unwrap();
        }
    });
    let deck_at = |p: Phase| seen.iter().find(|x| x.0 == p).map(|x| x.2);
    assert_eq!(deck_at(Phase::Beginning), Some(20));
    assert_eq!(deck_at(Phase::FirstMain), Some(19), "CR 505.3: scheme set in motion before the archenemy gets priority in the precombat main phase");
    assert_eq!(g.scheme_deck(alice()).unwrap().len(), 19, "CR 505.3/505.1a: not again in an additional (postcombat) main phase: {seen:?}");
    g.next_turn();
    assert_eq!(g.turn.active_player, bob());
    let mut q = TriggerQueue::new();
    run_turn(&mut g, &mut q, TurnRunner::new(), |_, _| {});
    assert_eq!(g.scheme_deck(alice()).unwrap().len(), 19, "CR 505.3: only during the archenemy's own turn");
}
