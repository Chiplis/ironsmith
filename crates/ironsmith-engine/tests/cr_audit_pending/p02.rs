//! CR audit (pending-entry pass, package p02): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, EffectExecutor, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::decision::{GameProgress, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    AbilityKind, CardType, CastingMethod, ColorSet, CounterType, DecisionMaker, LegalAction,
    ManaSymbol, ObjectId, Phase, PriorityLoopState, PriorityResponse, Zone,
};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn compile(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture compile failed for {name}: {e:?}"))
}

fn try_compile(name: &str, text: &str) -> Result<CardDefinition, String> {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .map_err(|e| format!("{e:?}"))
}

fn new_game(active: PlayerId) -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

/// Scripted decision maker: answers like `SelectFirstDecisionMaker`, but uses a fixed X,
/// prefers listed targets, and records every target/number/option prompt it saw.
#[derive(Default)]
struct Script {
    x: Option<u32>,
    prefer: Vec<Target>,
    option_pick: Option<Box<dyn FnMut(&SelectOptionsContext) -> Vec<usize>>>,
    seen_targets: Vec<Vec<Target>>,
    seen_numbers: Vec<(u32, u32)>,
    seen_options: Vec<Vec<String>>,
    seen_object_candidates: Vec<Vec<ObjectId>>,
    accept: bool,
    take: Option<usize>,
}

impl DecisionMaker for Script {
    fn decide_boolean(&mut self, g: &GameState, c: &BooleanContext) -> bool {
        if self.accept { c.can_accept } else { SelectFirstDecisionMaker.decide_boolean(g, c) }
    }
    fn decide_number(&mut self, _g: &GameState, c: &NumberContext) -> u32 {
        self.seen_numbers.push((c.min, c.max));
        match self.x {
            Some(x) => x.clamp(c.min, c.max),
            None => c.max,
        }
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        self.seen_object_candidates
            .push(c.candidates.iter().filter(|x| x.legal).map(|x| x.id).collect());
        if let Some(n) = self.take {
            return c.candidates.iter().filter(|x| x.legal).map(|x| x.id).take(n).collect();
        }
        SelectFirstDecisionMaker.decide_objects(g, c)
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        self.seen_options
            .push(c.options.iter().map(|o| o.description.clone()).collect());
        if let Some(pick) = self.option_pick.as_mut() {
            return pick(c);
        }
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let mut out = Vec::new();
        for req in &c.requirements {
            self.seen_targets.push(req.legal_targets.clone());
            if let Some(t) = self.prefer.iter().find(|t| req.legal_targets.contains(t)) {
                out.push(*t);
            } else if let Some(t) = req.legal_targets.first() {
                if req.min_targets > 0 || req.max_targets.unwrap_or(1) > 0 {
                    out.push(*t);
                }
            }
        }
        out
    }
}

fn complete_action<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: LegalAction,
    dm: &mut D,
    queue: &mut TriggerQueue,
) -> Result<(), String> {
    let requested_method = match &action {
        LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    assert_eq!(game.turn.priority_player, Some(player), "fixture priority holder");
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
                .unwrap_or(0);
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
                other => return Err(format!("pending action has no decision: {other:?}")),
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
        return Err("action exceeded decision bound".into());
    }
    Ok(())
}

fn legal(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    ironsmith::compute_legal_actions(game, player).unwrap()
}

fn cast_action(game: &GameState, player: PlayerId, spell: ObjectId) -> Option<LegalAction> {
    legal(game, player)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
}

fn cast<D: DecisionMaker>(game: &mut GameState, player: PlayerId, spell: ObjectId, dm: &mut D) {
    let action = cast_action(game, player, spell)
        .unwrap_or_else(|| panic!("fixture spell must be castable"));
    let mut queue = TriggerQueue::new();
    complete_action(game, player, action, dm, &mut queue).expect("fixture cast");
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).expect("stack triggers");
}

fn activate<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    source: ObjectId,
    ability_index: usize,
    dm: &mut D,
) -> Result<(), String> {
    let mut queue = TriggerQueue::new();
    complete_action(
        game,
        player,
        LegalAction::ActivateAbility {
            source,
            ability_index,
        },
        dm,
        &mut queue,
    )?;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).map_err(|e| format!("{e:?}"))
}

fn can_activate(game: &GameState, player: PlayerId, source: ObjectId) -> bool {
    legal(game, player)
        .iter()
        .any(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
}

/// Pass priority until the stack is empty (each pass by the current holder).
fn resolve_all<D: DecisionMaker>(game: &mut GameState, dm: &mut D) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    for _ in 0..80 {
        if game.stack.is_empty() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).expect("stack triggers");
            if game.stack.is_empty() {
                break;
            }
        }
        match ironsmith::game_loop::apply_priority_response_with_dm(
            game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .expect("pass priority")
        {
            GameProgress::NeedsDecisionCtx(ctx) => {
                ironsmith::game_loop::apply_decision_context_with_dm(
                    game, &mut queue, &mut state, &ctx, dm,
                )
                .expect("answer decision");
            }
            GameProgress::GameOver(r) => panic!("unexpected game over {r:?}"),
            _ => {}
        }
    }
    assert!(game.stack.is_empty(), "stack did not empty");
}

/// Resolve a list of triggered/activated effects of `source` directly from a stack entry.
fn resolve_effects_of<D: DecisionMaker>(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    ability_index: usize,
    dm: &mut D,
) {
    let ability = game.object(source).unwrap().abilities[ability_index].clone();
    let effects = match ability.kind {
        AbilityKind::Triggered(t) => t.effects,
        AbilityKind::Activated(a) => a.effects,
        AbilityKind::Static(_) => panic!("static ability has no effects"),
    };
    game.stack.push(StackEntry::ability(source, controller, effects));
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).expect("resolve ability effects");
}

fn ability_index(game: &GameState, id: ObjectId, pred: impl Fn(&ironsmith::Ability) -> bool) -> usize {
    game.object(id)
        .unwrap()
        .abilities
        .iter()
        .position(|a| pred(a))
        .expect("fixture ability index")
}

fn is_triggered(a: &ironsmith::Ability) -> bool {
    matches!(a.kind, AbilityKind::Triggered(_))
}
fn is_activated(a: &ironsmith::Ability) -> bool {
    matches!(a.kind, AbilityKind::Activated(_))
}

fn tokens(game: &GameState) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).unwrap().kind == ironsmith::ObjectKind::Token)
        .collect()
}

fn life(game: &GameState, p: PlayerId) -> i32 {
    game.player(p).unwrap().life
}

// ---------------------------------------------------------------------------
// 106: mana
// ---------------------------------------------------------------------------

fn tap_for_mana(game: &mut GameState, player: PlayerId, permanent: ObjectId, index: usize) -> Result<(), String> {
    let mut dm = Script::default();
    ironsmith::special_actions::perform(
        ironsmith::special_actions::SpecialAction::ActivateManaAbility {
            permanent_id: permanent,
            ability_index: index,
        },
        game,
        player,
        &mut dm,
    )
    .map_err(|e| format!("{e:?}"))
}

#[test]
fn cr_106_8_106_9_resonance_adds_hybrid_half_and_phyrexian_color() {
    let mut game = new_game(A);
    let resonance = compile(
        "Elemental Resonance",
        "Mana cost: {2}{G}{G}\nType: Enchantment — Aura\nEnchant permanent\nAt the beginning of your first main phase, add mana equal to enchanted permanent's mana cost. (Mana cost includes color. If a mana symbol has multiple colors, choose one.)",
    );
    let host = compile("Hybrid host", "Mana cost: {R/G}{2/W}{B/P}\nType: Artifact");
    let host = game.create_object_from_definition(&host, A, Zone::Battlefield);
    let aura = game.create_object_from_definition(&resonance, A, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
    let index = ability_index(&game, aura, is_triggered);
    let mut dm = Script::default();
    resolve_effects_of(&mut game, aura, A, index, &mut dm);
    let pool = game.player(A).unwrap().mana_pool.clone();
    assert_eq!(pool.red + pool.green, 1, "CR 106.8: {{R/G}} adds exactly one mana of the chosen half: {pool:?}");
    assert!(
        (pool.white == 1 && pool.colorless == 0) || (pool.white == 0 && pool.colorless == 2),
        "CR 106.8: {{2/W}} adds one white or two colorless: {pool:?}"
    );
    assert_eq!(pool.black, 1, "CR 106.9: {{B/P}} adds one black mana: {pool:?}");
    assert_eq!(life(&game, A), 20, "CR 106.9: adding Phyrexian mana costs no life");
}

#[test]
fn cr_106_12b_tapped_for_mana_replacements_modify_production() {
    let forest = compile("Forest", "Type: Basic Land — Forest");
    let elves = compile(
        "Llanowar Elves",
        "Mana cost: {G}\nType: Creature — Elf Druid\nPower/Toughness: 1/1\n{T}: Add {G}.",
    );
    let reflection = compile(
        "Mana Reflection",
        "Mana cost: {4}{G}{G}\nType: Enchantment\nIf you tap a permanent for mana, it produces twice as much of that mana instead.",
    );
    let contamination = compile(
        "Contamination",
        "Mana cost: {2}{B}\nType: Enchantment\nAt the beginning of your upkeep, sacrifice this enchantment unless you sacrifice a creature.\nIf a land is tapped for mana, it produces {B} instead of any other type and amount.",
    );
    let mut failures = Vec::new();
    for (label, def, source_def, expect_g, expect_b) in [
        ("reflection+forest", &reflection, &forest, 2, 0),
        ("reflection+elves", &reflection, &elves, 2, 0),
        ("contamination+forest", &contamination, &forest, 0, 1),
        ("contamination+elves", &contamination, &elves, 1, 0),
    ] {
        let mut game = new_game(A);
        game.create_object_from_definition(def, A, Zone::Battlefield);
        let source = game.create_object_from_definition(source_def, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let index = game.current_abilities(source).unwrap().iter().position(is_activated).expect("mana ability");
        tap_for_mana(&mut game, A, source, index).unwrap();
        let pool = &game.player(A).unwrap().mana_pool;
        if (pool.green, pool.black) != (expect_g, expect_b) {
            failures.push(format!("{label}: got G{} B{}, expected G{expect_g} B{expect_b}", pool.green, pool.black));
        }
    }
    assert!(failures.is_empty(), "CR 106.12b: {failures:?}");
}

// ---------------------------------------------------------------------------
// 107: numbers and symbols
// ---------------------------------------------------------------------------

#[test]
fn cr_107_3e_107_3i_zaxara_hydra_uses_the_cast_spells_x() {
    let mut game = new_game(A);
    let zaxara = compile(
        "Zaxara, the Exemplary",
        "Mana cost: {1}{B}{G}{U}\nType: Legendary Creature — Nightmare Hydra\nPower/Toughness: 2/3\nDeathtouch\n{T}: Add two mana of any one color.\nWhenever you cast a spell with {X} in its mana cost, create a 0/0 green Hydra creature token, then put X +1/+1 counters on it.",
    );
    game.create_object_from_definition(&zaxara, A, Zone::Battlefield);
    let spell = compile(
        "X life sorcery",
        "Mana cost: {X}{G}\nType: Sorcery\nYou gain X life. Then you gain X life.",
    );
    let spell = game.create_object_from_definition(&spell, A, Zone::Hand);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 4);
    let mut dm = Script { x: Some(3), ..Default::default() };
    cast(&mut game, A, spell, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert_eq!(life(&game, A), 26, "CR 107.3i: every X in the spell has the announced value 3");
    let hydras: Vec<_> = tokens(&game);
    assert_eq!(hydras.len(), 1, "Zaxara trigger creates one Hydra");
    assert_eq!(
        game.counter_count(hydras[0], CounterType::PlusOnePlusOne),
        3,
        "CR 107.3e: the trigger's X is the X used by the cast spell"
    );
}

#[test]
fn cr_107_3d_suspend_x_is_chosen_immediately_before_paying() {
    let mut game = new_game(A);
    let chronicler = compile(
        "Aeon Chronicler",
        "Mana cost: {3}{U}{U}\nType: Creature — Avatar\nPower/Toughness: */*\nAeon Chronicler's power and toughness are each equal to the number of cards in your hand.\nSuspend X—{X}{3}{U}. X can't be 0.\nWhenever a time counter is removed from this card while it's exiled, draw a card.",
    );
    let card = game.create_object_from_definition(&chronicler, A, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 5);
    let mut dm = Script { x: Some(2), ..Default::default() };
    ironsmith::special_actions::perform(
        ironsmith::special_actions::SpecialAction::Suspend { card_id: card },
        &mut game,
        A,
        &mut dm,
    )
    .expect("suspend special action");
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    assert_eq!(dm.seen_numbers.first().map(|n| n.0), Some(1), "CR 107.3d: X can't be 0 bounds the choice");
    assert_eq!(game.counter_count(exiled, CounterType::Time), 2, "CR 107.3d: chosen X=2 time counters");
    let pool = &game.player(A).unwrap().mana_pool;
    assert_eq!((pool.blue, pool.colorless), (0, 0), "CR 107.3d: {{X}}{{3}}{{U}} with X=2 spends all six mana");
}

#[test]
fn cr_107_7_loyalty_symbols_put_or_remove_counters() {
    let pw = compile(
        "Loyalty symbol walker",
        "Mana cost: {0}\nType: Legendary Planeswalker — Testwalker\nLoyalty: 4\n+1: You gain 1 life.\n0: You gain 2 life.\n−2: You gain 3 life.\n−X: You gain X life.",
    );
    let mut failures = Vec::new();
    for (ordinal, expected_loyalty, expected_life) in [(0usize, 5u32, 21), (1, 4, 22), (2, 2, 23), (3, 1, 23)] {
        let mut game = new_game(A);
        let id = game.create_object_from_definition(&pw, A, Zone::Battlefield);
        let indices: Vec<usize> = game.object(id).unwrap().abilities.iter().enumerate().filter(|(_, a)| is_activated(a)).map(|(i, _)| i).collect();
        let mut dm = Script { x: Some(3), ..Default::default() };
        activate(&mut game, A, id, indices[ordinal], &mut dm).unwrap();
        let after_cost = game.counter_count(id, CounterType::Loyalty);
        resolve_all(&mut game, &mut dm);
        if after_cost != expected_loyalty || life(&game, A) != expected_life {
            failures.push(format!("ability {ordinal}: loyalty {after_cost} (want {expected_loyalty}), life {} (want {expected_life})", life(&game, A)));
        }
    }
    assert!(failures.is_empty(), "CR 107.7: {failures:?}");
}

#[test]
fn cr_107_8_student_of_warfare_level_symbols_set_pt_and_abilities() {
    use ironsmith::static_abilities::StaticAbilityId;
    let student = compile(
        "Student of Warfare",
        "Mana cost: {W}\nType: Creature — Human Knight\nPower/Toughness: 1/1\nLevel up {W} ({W}: Put a level counter on this. Level up only as a sorcery.)\nLEVEL 2-6\n3/3\nFirst strike\nLEVEL 7+\n4/4\nDouble strike",
    );
    let mut failures = Vec::new();
    for (levels, pt, fs, ds) in [(0u32, 1, false, false), (1, 1, false, false), (2, 3, true, false), (6, 3, true, false), (7, 4, false, true), (10, 4, false, true)] {
        let mut game = new_game(A);
        let id = game.create_object_from_definition(&student, A, Zone::Battlefield);
        if levels > 0 {
            game.add_counters(id, CounterType::Level, levels);
        }
        let got = (
            game.current_power(id),
            game.current_toughness(id),
            game.object_has_static_ability_id(id, StaticAbilityId::FirstStrike),
            game.object_has_static_ability_id(id, StaticAbilityId::DoubleStrike),
        );
        if got != (Some(pt), Some(pt), fs, ds) {
            failures.push(format!("levels {levels}: got {got:?}"));
        }
    }
    assert!(failures.is_empty(), "CR 107.8a/b: {failures:?}");
}

#[test]
fn cr_107_14_paying_energy_removes_one_counter_per_symbol() {
    let hub = compile(
        "Aether Hub",
        "Type: Land\nWhen this land enters, you get {E} (an energy counter).\n{T}: Add {C}.\n{T}, Pay {E}: Add one mana of any color.",
    );
    for energy in [0u32, 2] {
        let mut game = new_game(A);
        let id = game.create_object_from_definition(&hub, A, Zone::Battlefield);
        game.player_mut(A).unwrap().energy_counters = energy;
        let index = game.object(id).unwrap().abilities.iter().rposition(is_activated).unwrap();
        let result = tap_for_mana(&mut game, A, id, index);
        if energy == 0 {
            assert!(result.is_err(), "CR 107.14: {{E}} can't be paid without an energy counter");
        } else {
            result.unwrap();
            assert_eq!(game.player(A).unwrap().energy_counters, 1, "CR 107.14: paying {{E}} removes exactly one energy counter");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
        }
    }
}

// ---------------------------------------------------------------------------
// 108/109: cards, objects, controllers
// ---------------------------------------------------------------------------

#[test]
fn cr_108_2_108_2b_card_put_into_graveyard_trigger_ignores_tokens() {
    let mut game = new_game(A);
    let observer = compile(
        "Card graveyard observer",
        "Mana cost: {B}\nType: Enchantment\nWhenever another card is put into a graveyard from anywhere, you gain 1 life.",
    );
    game.create_object_from_definition(&observer, A, Zone::Battlefield);
    let bear = compile("Grizzly Bears", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let card = game.create_object_from_definition(&bear, B, Zone::Battlefield);
    let maker = compile("Token maker", "Mana cost: {0}\nType: Sorcery\nCreate a 2/2 green Bear creature token.");
    let maker = game.create_object_from_definition(&maker, A, Zone::Hand);
    let mut dm = Script::default();
    cast(&mut game, A, maker, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert_eq!(life(&game, A), 21, "sorcery card going to graveyard is a card (control)");
    let token = tokens(&game)[0];
    let wrath = compile("Wrath", "Mana cost: {0}\nType: Sorcery\nDestroy all creatures.");
    let wrath = game.create_object_from_definition(&wrath, A, Zone::Hand);
    cast(&mut game, A, wrath, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert!(game.object(card).is_none() || game.object(card).unwrap().zone != Zone::Battlefield);
    let _ = token;
    // Wrath card itself (+1) and Grizzly Bears card (+1); the dying token is not a card.
    assert_eq!(life(&game, A), 23, "CR 108.2b: a token dying is not a card put into a graveyard");
}

#[test]
fn cr_108_4a_109_4_card_outside_stack_and_battlefield_uses_owner_as_controller() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    let mut game = new_game(A);
    let bear = compile("Grizzly Bears", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let id = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    ApplyContinuousEffect::new(EffectTarget::Specific(id), Modification::ChangeController(B), ironsmith::Until::Forever)
        .execute(&mut game, &mut EffectContext::new_default(id, B))
        .unwrap();
    assert_eq!(game.current_controller(id), Some(B), "control change applies on the battlefield");
    let mut failures = Vec::new();
    let mut current = id;
    for zone in [Zone::Graveyard, Zone::Exile, Zone::Hand, Zone::Library] {
        current = game.move_object_by_effect(current, zone).unwrap();
        let reported = game.current_controller(current);
        if reported.is_some() && reported != Some(A) {
            failures.push(format!("{zone:?}: controller {reported:?}"));
        }
    }
    assert!(failures.is_empty(), "CR 108.4a/109.4: a card not on the stack or battlefield has no controller other than its owner: {failures:?}");
}

#[test]
fn cr_109_4a_mana_ability_is_controlled_by_its_activator() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    let mut game = new_game(B);
    let elves = compile("Llanowar Elves", "Mana cost: {G}\nType: Creature — Elf Druid\nPower/Toughness: 1/1\n{T}: Add {G}.");
    let id = game.create_object_from_definition(&elves, A, Zone::Battlefield);
    ApplyContinuousEffect::new(EffectTarget::Specific(id), Modification::ChangeController(B), ironsmith::Until::Forever)
        .execute(&mut game, &mut EffectContext::new_default(id, B))
        .unwrap();
    game.remove_summoning_sickness(id);
    let index = ability_index(&game, id, is_activated);
    assert!(tap_for_mana(&mut game, A, id, index).is_err(), "CR 109.4a: the owner who doesn't control it can't activate it");
    tap_for_mana(&mut game, B, id, index).unwrap();
    assert_eq!(game.player(B).unwrap().mana_pool.green, 1, "CR 109.4a: mana goes to the activating controller");
    assert_eq!(game.player(A).unwrap().mana_pool.green, 0);
}

#[test]
fn cr_109_2a_2b_card_and_spell_descriptions_select_their_zones() {
    let mut game = new_game(A);
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let on_field = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    let in_yard = game.create_object_from_definition(&bear, A, Zone::Graveyard);
    let in_hand = game.create_object_from_definition(&bear, A, Zone::Hand);
    let raise = compile("Raise Dead", "Mana cost: {0}\nType: Sorcery\nReturn target creature card from your graveyard to your hand.");
    let raise = game.create_object_from_definition(&raise, A, Zone::Hand);
    let mut dm = Script::default();
    cast(&mut game, A, raise, &mut dm);
    let seen = dm.seen_targets.concat();
    assert!(seen.contains(&Target::Object(in_yard)), "CR 109.2a: graveyard creature card is a legal target");
    assert!(
        !seen.contains(&Target::Object(on_field)) && !seen.contains(&Target::Object(in_hand)),
        "CR 109.2a: 'creature card from your graveyard' excludes other zones: {seen:?}"
    );
    resolve_all(&mut game, &mut dm);
    // Spell description: Essence Scatter only sees creature spells on the stack.
    let creature_spell = in_hand;
    cast(&mut game, A, creature_spell, &mut dm);
    let scatter = compile("Essence Scatter", "Mana cost: {0}\nType: Instant\nCounter target creature spell.");
    let scatter = game.create_object_from_definition(&scatter, A, Zone::Hand);
    let mut dm2 = Script::default();
    cast(&mut game, A, scatter, &mut dm2);
    let seen = dm2.seen_targets.concat();
    assert!(seen.contains(&Target::Object(creature_spell)) || seen.iter().any(|t| matches!(t, Target::Object(o) if game.object(*o).map(|x| x.zone) == Some(Zone::Stack))), "CR 109.2b: the creature spell is targetable: {seen:?}");
    assert!(!seen.contains(&Target::Object(on_field)), "CR 109.2b: a creature permanent is not a creature spell: {seen:?}");
}

// ---------------------------------------------------------------------------
// 110: permanents
// ---------------------------------------------------------------------------

#[test]
fn cr_110_2a_put_onto_battlefield_enters_under_instructed_players_control() {
    let mut game = new_game(A);
    let bear = compile("Grizzly Bears", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let target = game.create_object_from_definition(&bear, B, Zone::Graveyard);
    let stable = game.object(target).unwrap().stable_id;
    let spell = compile("Graveyard puller", "Mana cost: {0}\nType: Sorcery\nPut target creature card from a graveyard onto the battlefield.");
    let spell = game.create_object_from_definition(&spell, A, Zone::Hand);
    let mut dm = Script { prefer: vec![Target::Object(target)], ..Default::default() };
    cast(&mut game, A, spell, &mut dm);
    resolve_all(&mut game, &mut dm);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.object(now).unwrap().owner, B);
    assert_eq!(game.current_controller(now), Some(A), "CR 110.2a: the instructed player controls the permanent");
}

#[test]
fn cr_110_4b_permanent_spell_means_artifact_creature_enchantment_planeswalker_battle() {
    let observer = compile(
        "Permanent spell observer",
        "Mana cost: {0}\nType: Enchantment\nWhenever you cast a permanent spell, you gain 1 life.",
    );
    let mut failures = Vec::new();
    for (label, text, permanent) in [
        ("artifact", "Mana cost: {0}\nType: Artifact", true),
        ("creature", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2", true),
        ("enchantment", "Mana cost: {0}\nType: Enchantment", true),
        ("planeswalker", "Mana cost: {0}\nType: Legendary Planeswalker — Testwalker\nLoyalty: 3\n+1: You gain 1 life.", true),
        ("instant", "Mana cost: {0}\nType: Instant\nScry 1.", false),
        ("sorcery", "Mana cost: {0}\nType: Sorcery\nScry 1.", false),
        ("kindred instant", "Mana cost: {0}\nType: Kindred Instant — Elf\nScry 1.", false),
    ] {
        let mut game = new_game(A);
        game.create_object_from_definition(&observer, A, Zone::Battlefield);
        let def = compile(&format!("Permanent spell probe {label}"), text);
        let spell = game.create_object_from_definition(&def, A, Zone::Hand);
        let mut dm = Script::default();
        cast(&mut game, A, spell, &mut dm);
        resolve_all(&mut game, &mut dm);
        let gained = life(&game, A) - 20;
        if gained != i32::from(permanent) {
            failures.push(format!("{label}: gained {gained}"));
        }
    }
    assert!(failures.is_empty(), "CR 110.4b: {failures:?}");
}

#[test]
fn cr_110_3_112_3_printed_characteristics_modified_by_continuous_effects() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    let mut game = new_game(A);
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let id = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    assert_eq!((game.current_power(id), game.current_toughness(id)), (Some(2), Some(2)), "CR 110.3: printed values");
    ApplyContinuousEffect::new(EffectTarget::Specific(id), Modification::ModifyPowerToughness { power: 1, toughness: 1 }, ironsmith::Until::EndOfTurn)
        .execute(&mut game, &mut EffectContext::new_default(id, A))
        .unwrap();
    assert_eq!((game.current_power(id), game.current_toughness(id)), (Some(3), Some(3)), "CR 110.3: modified by continuous effects");
}

#[test]
fn cr_112_1_112_3_112_4_spell_color_change_applies_on_stack_and_to_the_permanent() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    let mut game = new_game(A);
    let def = compile("Black bear", "Mana cost: {B}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let card = game.create_object_from_definition(&def, A, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 1);
    let mut dm = Script::default();
    cast(&mut game, A, card, &mut dm);
    let spell = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack, "CR 112.1: casting moves the card to the stack");
    assert_eq!(game.stack.last().map(|e| e.object_id), Some(spell), "CR 112.1: on top of the stack");
    assert_eq!(game.current_colors(spell), Some(ColorSet::BLACK), "CR 112.3: printed color");
    ApplyContinuousEffect::new(EffectTarget::Specific(spell), Modification::SetColors(ColorSet::WHITE), ironsmith::Until::Forever)
        .execute(&mut game, &mut EffectContext::new_default(spell, A))
        .unwrap();
    assert_eq!(game.current_colors(spell), Some(ColorSet::WHITE), "CR 112.3: spell modified by a continuous effect");
    resolve_all(&mut game, &mut dm);
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.current_colors(permanent), Some(ColorSet::WHITE), "CR 112.4: the color change continues to apply to the permanent");
}

// ---------------------------------------------------------------------------
// Misc: pawprints, copies of cards, alternative costs, outside-the-game, sources
// ---------------------------------------------------------------------------

#[test]
fn cr_107_18_pawprint_modes_spend_a_five_point_budget() {
    let mut game = new_game(A);
    let season = compile(
        "Season of the Burrow",
        "Mana cost: {0}\nType: Sorcery\nChoose up to five {P} worth of modes. You may choose the same mode more than once.\n{P} — Create a 1/1 white Rabbit creature token.\n{P}{P} — Exile target nonland permanent. Its controller draws a card.\n{P}{P}{P} — Return target permanent card with mana value 3 or less from your graveyard to the battlefield with an indestructible counter on it.",
    );
    let id = game.create_object_from_definition(&season, A, Zone::Hand);
    let mut dm = Script::default();
    dm.option_pick = Some(Box::new(|c: &SelectOptionsContext| {
        let rabbit = c.options.iter().find(|o| o.point_cost == 1).map(|o| o.index).unwrap_or(0);
        vec![rabbit; 5]
    }));
    cast(&mut game, A, id, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert_eq!(tokens(&game).len(), 5, "CR 107.18 / 700.2i: five {{P}} buys the one-pawprint mode five times");
}

/// Spelltwine resolves for Alice: returns (game, copies seen on the stack, final life).
fn spelltwine_run() -> (Vec<(PlayerId, Option<PlayerId>, String)>, i32) {
    let mut game = new_game(A);
    let twine = compile(
        "Spelltwine",
        "Mana cost: {0}\nType: Sorcery\nExile target instant or sorcery card from your graveyard and target instant or sorcery card from an opponent's graveyard. Copy those cards. Cast the copies if able without paying their mana costs. Exile Spelltwine.",
    );
    let mine = compile("Small gain", "Mana cost: {1}\nType: Instant\nYou gain 1 life.");
    let theirs = compile("Big gain", "Mana cost: {1}\nType: Instant\nYou gain 2 life.");
    game.create_object_from_definition(&mine, A, Zone::Graveyard);
    game.create_object_from_definition(&theirs, B, Zone::Graveyard);
    let id = game.create_object_from_definition(&twine, A, Zone::Hand);
    let mut dm = Script { accept: true, ..Default::default() };
    cast(&mut game, A, id, &mut dm);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut copies: Vec<(PlayerId, Option<PlayerId>, String)> = Vec::new();
    for _ in 0..12 {
        for e in &game.stack {
            if let Some(o) = game.object(e.object_id) {
                if o.kind != ironsmith::ObjectKind::Card && !copies.iter().any(|c| c.2 == o.name.to_string()) {
                    copies.push((o.owner, game.current_controller(e.object_id), o.name.to_string()));
                }
            }
        }
        if game.stack.is_empty() { break; }
        ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut dm,
        ).unwrap();
    }
    (copies, life(&game, A))
}

#[test]
fn cr_112_2a_copy_of_a_card_cast_by_its_creator_is_owned_by_the_creator() {
    let (copies, _) = spelltwine_run();
    assert!(!copies.is_empty(), "fixture: Spelltwine cast at least one copy");
    for (owner, controller, name) in &copies {
        assert_eq!((*owner, *controller), (A, Some(A)), "CR 112.2a: copy of {name} is owned and controlled by the player who created and cast it");
    }
}

#[test]
fn cr_112_1b_each_cast_copy_of_a_card_is_a_spell() {
    let (copies, final_life) = spelltwine_run();
    assert_eq!(
        (copies.len(), final_life),
        (2, 23),
        "CR 112.1b: both copied cards are cast as spells and resolve (copies seen: {copies:?})"
    );
}

#[test]
fn cr_113_6d_alternative_cost_ability_functions_while_casting() {
    let mut game = new_game(A);
    let fow = compile(
        "Force of Will",
        "Mana cost: {3}{U}{U}\nType: Instant\nYou may pay 1 life and exile a blue card from your hand rather than pay this spell's mana cost.\nYou gain 5 life.",
    );
    let blue = compile("Blue card", "Mana cost: {U}\nType: Instant\nScry 1.");
    let id = game.create_object_from_definition(&fow, A, Zone::Hand);
    let pitched = game.create_object_from_definition(&blue, A, Zone::Hand);
    let action = legal(&game, A)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == id && *casting_method != CastingMethod::Normal))
        .expect("CR 113.6d: the alternative cost is available while casting");
    let mut dm = Script::default();
    let mut queue = TriggerQueue::new();
    complete_action(&mut game, A, action, &mut dm, &mut queue).unwrap();
    assert_eq!(life(&game, A), 19, "CR 113.6d: the alternative cost was paid");
    assert_ne!(game.object(pitched).map(|o| o.zone), Some(Zone::Hand));
    resolve_all(&mut game, &mut dm);
    assert_eq!(life(&game, A), 24);
}

#[test]
fn cr_108_5_wish_cannot_bring_a_nontraditional_card_into_the_game() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let mut game = new_game(A);
    let plane = CardDefinitionBuilder::new(ironsmith::CardId::new(), "Outside plane")
        .card_types(vec![CardType::Plane])
        .build();
    let plane = game.create_object_from_definition(&plane, A, Zone::OutsideGame);
    let stable = game.object(plane).unwrap().stable_id;
    let wish = compile("Generic wish", "Mana cost: {0}\nType: Sorcery\nYou may reveal a card you own from outside the game and put it into your hand.");
    let wish = game.create_object_from_definition(&wish, A, Zone::Hand);
    let mut dm = Script { accept: true, ..Default::default() };
    cast(&mut game, A, wish, &mut dm);
    resolve_all(&mut game, &mut dm);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::OutsideGame, "CR 108.5: the nontraditional card remains outside the game");
}

#[test]
fn cr_109_2c_source_of_your_choice_includes_stack_and_command_zone_sources() {
    let mut game = new_game(B);
    game.turn.priority_player = Some(B);
    let bolt = compile("Red bolt", "Mana cost: {0}\nType: Instant\nThis spell deals 3 damage to any target.");
    let bolt = game.create_object_from_definition(&bolt, B, Zone::Hand);
    let commander = compile("Command zone dragon", "Mana cost: {4}{R}\nType: Creature — Dragon\nPower/Toughness: 4/4");
    let commander = game.create_object_from_definition(&commander, B, Zone::Command);
    let mut dm = Script { prefer: vec![Target::Player(A)], ..Default::default() };
    cast(&mut game, B, bolt, &mut dm);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut dm,
    ).unwrap();
    assert_eq!(game.turn.priority_player, Some(A));
    let cop = compile("Prevent source", "Mana cost: {0}\nType: Instant\nPrevent all damage a source of your choice would deal this turn.");
    let cop = game.create_object_from_definition(&cop, A, Zone::Hand);
    let mut dm2 = Script::default();
    cast(&mut game, A, cop, &mut dm2);
    let mut dm3 = Script::default();
    // Resolve only the prevention spell.
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    for _ in 0..4 {
        if game.stack.len() < 2 { break; }
        ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut dm3,
        ).unwrap();
    }
    let candidates: Vec<ObjectId> = dm3.seen_object_candidates.concat();
    let bolt_now = game.stack.first().map(|e| e.object_id);
    assert!(bolt_now.is_some_and(|b| candidates.contains(&b)), "CR 109.2c: a spell on the stack is a source: {candidates:?} / options {:?}", dm3.seen_options);
    assert!(candidates.contains(&commander), "CR 109.2c: a face-up command-zone object is a source: {candidates:?}");
}

// ---------------------------------------------------------------------------
// 111: tokens
// ---------------------------------------------------------------------------

/// Resolve a zero-cost sorcery with `instruction` for Alice (with a creature to enchant or
/// target) and return the game, the created tokens and the host creature.
fn create_with(instruction: &str) -> Result<(GameState, Vec<ObjectId>, ObjectId), String> {
    let mut game = new_game(A);
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let host = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    let def = try_compile("Token instruction", &format!("Mana cost: {{0}}\nType: Sorcery\n{instruction}"))?;
    if ironsmith::cards::generated_definition_has_unimplemented_content(&def) {
        return Err(format!("unimplemented content for {instruction:?}"));
    }
    let spell = game.create_object_from_definition(&def, A, Zone::Hand);
    let mut dm = Script { prefer: vec![Target::Object(host)], ..Default::default() };
    cast(&mut game, A, spell, &mut dm);
    resolve_all(&mut game, &mut dm);
    let made = tokens(&game);
    Ok((game, made, host))
}

struct TokenSpec {
    rule: &'static str,
    instruction: &'static str,
    name: Option<&'static str>,
    types: &'static [CardType],
    subtype: &'static str,
    colors: ColorSet,
    pt: Option<(i32, i32)>,
    activated: usize,
}

fn check_token(spec: TokenSpec, extra: impl FnOnce(&mut GameState, ObjectId, ObjectId) -> Vec<String>) {
    let (mut game, made, host) = match create_with(spec.instruction) {
        Ok(x) => x,
        Err(e) => panic!("CR {}: the predefined token instruction is not supported: {e}", spec.rule),
    };
    assert_eq!(made.len(), 1, "CR {}: exactly one predefined token is created, got {}", spec.rule, made.len());
    let t = made[0];
    let mut failures = Vec::new();
    let types = game.current_card_types(t).unwrap();
    if types != spec.types {
        failures.push(format!("types {types:?}"));
    }
    let subtypes: Vec<String> = game.current_subtypes(t).unwrap().iter().map(|s| format!("{s:?}")).collect();
    if !subtypes.iter().any(|s| s == spec.subtype) {
        failures.push(format!("subtypes {subtypes:?}"));
    }
    if game.current_colors(t) != Some(spec.colors) {
        failures.push(format!("colors {:?}", game.current_colors(t)));
    }
    if let Some(name) = spec.name {
        if game.current_name(t).as_deref() != Some(name) {
            failures.push(format!("name {:?}", game.current_name(t)));
        }
    }
    let pt = game.current_power(t).zip(game.current_toughness(t));
    if spec.pt.is_some() && pt != spec.pt {
        failures.push(format!("pt {pt:?}"));
    }
    let activated = game.current_abilities(t).unwrap().iter().filter(|a| is_activated(a)).count();
    if activated != spec.activated {
        failures.push(format!("activated abilities {activated}"));
    }
    failures.extend(extra(&mut game, t, host));
    assert!(failures.is_empty(), "CR {}: {failures:?}", spec.rule);
}

fn activate_token(game: &mut GameState, t: ObjectId) -> Result<(), String> {
    let index = game.object(t).unwrap().abilities.iter().position(is_activated).ok_or("no activated ability")?;
    let mut dm = Script::default();
    if tap_for_mana(game, A, t, index).is_ok() {
        return Ok(());
    }
    activate(game, A, t, index, &mut dm)?;
    resolve_all(game, &mut dm);
    Ok(())
}

const ART: &[CardType] = &[CardType::Artifact];
const ENCH: &[CardType] = &[CardType::Enchantment];

fn library_filler(g: &mut GameState, n: usize) {
    let lib = compile("Library card", "Mana cost: {0}\nType: Artifact");
    for _ in 0..n {
        g.create_object_from_definition(&lib, A, Zone::Library);
    }
}

#[test]
fn cr_111_10a_treasure_token() {
    check_token(TokenSpec { rule: "111.10a", instruction: "Create a Treasure token.", name: None, types: ART, subtype: "Treasure", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if g.player(A).unwrap().mana_pool.total() != 1 { f.push("no mana".into()); }
        if g.battlefield.contains(&t) { f.push("not sacrificed".into()); }
        f
    });
}

#[test]
fn cr_111_10b_food_token() {
    check_token(TokenSpec { rule: "111.10b", instruction: "Create a Food token.", name: None, types: ART, subtype: "Food", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if life(g, A) != 23 { f.push(format!("life {}", life(g, A))); }
        f
    });
}

#[test]
fn cr_111_10c_gold_token() {
    check_token(TokenSpec { rule: "111.10c", instruction: "Create a Gold token.", name: None, types: ART, subtype: "Gold", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        let mut f = vec![];
        g.tap(t);
        if let Err(e) = activate_token(g, t) { f.push(format!("Gold needs no tap: {e}")); }
        if g.player(A).unwrap().mana_pool.total() != 1 { f.push("no mana".into()); }
        f
    });
}

#[test]
fn cr_111_10d_walker_token() {
    check_token(TokenSpec { rule: "111.10d", instruction: "Create a Walker token.", name: Some("Walker"), types: &[CardType::Creature], subtype: "Zombie", colors: ColorSet::BLACK, pt: Some((2, 2)), activated: 0 }, |_, _, _| vec![]);
}

#[test]
fn cr_111_10e_shard_token() {
    check_token(TokenSpec { rule: "111.10e", instruction: "Create a Shard token.", name: None, types: ENCH, subtype: "Shard", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        library_filler(g, 3);
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        let hand = g.player(A).unwrap().hand.len();
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if g.player(A).unwrap().hand.len() != hand + 1 { f.push("did not draw".into()); }
        f
    });
}

#[test]
fn cr_111_10f_clue_token() {
    check_token(TokenSpec { rule: "111.10f", instruction: "Investigate.", name: None, types: ART, subtype: "Clue", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        library_filler(g, 1);
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        let hand = g.player(A).unwrap().hand.len();
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if g.player(A).unwrap().hand.len() != hand + 1 { f.push("did not draw".into()); }
        f
    });
}

#[test]
fn cr_111_10g_blood_token() {
    check_token(TokenSpec { rule: "111.10g", instruction: "Create a Blood token.", name: None, types: ART, subtype: "Blood", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        library_filler(g, 1);
        let lib = compile("Discard fodder", "Mana cost: {0}\nType: Artifact");
        let discard = g.create_object_from_definition(&lib, A, Zone::Hand);
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if g.object(discard).map(|o| o.zone) == Some(Zone::Hand) { f.push("did not discard".into()); }
        if g.player(A).unwrap().hand.len() != 1 { f.push("did not draw".into()); }
        f
    });
}

#[test]
fn cr_111_10h_powerstone_token() {
    check_token(TokenSpec { rule: "111.10h", instruction: "Create a Powerstone token.", name: None, types: ART, subtype: "Powerstone", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if g.player(A).unwrap().mana_pool.colorless != 1 { f.push("no {C}".into()); }
        let creature = compile("One-drop creature", "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/2");
        let c = g.create_object_from_definition(&creature, A, Zone::Hand);
        let mut dm = Script::default();
        if let Some(action) = cast_action(g, A, c) {
            let mut q = TriggerQueue::new();
            if complete_action(g, A, action, &mut dm, &mut q).is_ok() && g.object(c).map(|o| o.zone) != Some(Zone::Hand) {
                f.push("powerstone mana paid for a nonartifact spell".into());
            }
        }
        f
    });
}

#[test]
fn cr_111_10i_incubator_token() {
    check_token(TokenSpec { rule: "111.10i", instruction: "Incubate 2.", name: None, types: ART, subtype: "Incubator", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        let mut f = vec![];
        if g.counter_count(t, CounterType::PlusOnePlusOne) != 2 { f.push("counters".into()); }
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if let Some(t2) = tokens(g).first().copied() {
            if g.current_name(t2).as_deref() != Some("Phyrexian Token") { f.push(format!("back name {:?}", g.current_name(t2))); }
            if g.current_card_types(t2).unwrap() != vec![CardType::Artifact, CardType::Creature] { f.push(format!("back types {:?}", g.current_card_types(t2))); }
            if g.current_power(t2) != Some(2) { f.push(format!("back power {:?}", g.current_power(t2))); }
        } else {
            f.push("token vanished".into());
        }
        f
    });
}

fn role(rule: &'static str, name: &'static str, extra: impl FnOnce(&mut GameState, ObjectId, ObjectId) -> Vec<String>) {
    let instruction: &'static str = Box::leak(format!("Create a {name} Role token attached to target creature you control.").into_boxed_str());
    check_token(TokenSpec { rule, instruction, name: Some(name), types: ENCH, subtype: "Role", colors: ColorSet::COLORLESS, pt: None, activated: 0 }, |g, t, host| {
        let mut f = vec![];
        if !g.current_subtypes(t).unwrap().iter().any(|s| format!("{s:?}") == "Aura") { f.push("not an Aura".into()); }
        if g.object(t).unwrap().attached_to != Some(ironsmith::object::AttachmentTarget::Object(host)) { f.push("not attached".into()); }
        f.extend(extra(g, t, host));
        f
    });
}

fn has_kw(g: &GameState, id: ObjectId, kw: ironsmith::static_abilities::StaticAbilityId) -> bool {
    g.object_has_static_ability_id(id, kw)
}

#[test]
fn cr_111_10j_cursed_role_token() {
    role("111.10j", "Cursed", |g, _, h| if (g.current_power(h), g.current_toughness(h)) != (Some(1), Some(1)) { vec![format!("host {:?}/{:?}", g.current_power(h), g.current_toughness(h))] } else { vec![] });
}

#[test]
fn cr_111_10k_monster_role_token() {
    use ironsmith::static_abilities::StaticAbilityId;
    role("111.10k", "Monster", |g, _, h| {
        let mut f = vec![];
        if g.current_power(h) != Some(3) { f.push("no +1/+1".into()); }
        if !has_kw(g, h, StaticAbilityId::Trample) { f.push("no trample".into()); }
        f
    });
}

#[test]
fn cr_111_10m_royal_role_token() {
    use ironsmith::static_abilities::StaticAbilityId;
    role("111.10m", "Royal", |g, _, h| {
        let mut f = vec![];
        if g.current_power(h) != Some(3) { f.push("no +1/+1".into()); }
        if !has_kw(g, h, StaticAbilityId::Ward) { f.push("no ward".into()); }
        f
    });
}

#[test]
fn cr_111_10n_sorcerer_role_token() {
    role("111.10n", "Sorcerer", |g, _, h| {
        let mut f = vec![];
        if g.current_power(h) != Some(3) { f.push("no +1/+1".into()); }
        if !g.current_abilities(h).unwrap().iter().any(is_triggered) { f.push("no attack trigger".into()); }
        f
    });
}

#[test]
fn cr_111_10p_virtuous_role_token() {
    role("111.10p", "Virtuous", |g, _, h| {
        let ench = compile("Plain enchantment", "Mana cost: {0}\nType: Enchantment");
        g.create_object_from_definition(&ench, A, Zone::Battlefield);
        // The Role itself plus the extra enchantment: +2/+2.
        if g.current_power(h) != Some(4) { vec![format!("host power {:?}", g.current_power(h))] } else { vec![] }
    });
}

#[test]
fn cr_111_10q_wicked_role_token() {
    role("111.10q", "Wicked", |g, t, h| {
        let mut f = vec![];
        if g.current_power(h) != Some(3) { f.push("no +1/+1".into()); }
        if !g.current_abilities(t).unwrap().iter().any(is_triggered) { f.push("no graveyard trigger".into()); }
        f
    });
}

#[test]
fn cr_111_10r_young_hero_role_token() {
    role("111.10r", "Young Hero", |g, _, h| {
        let mut f = vec![];
        if g.current_power(h) != Some(2) { f.push(format!("Young Hero must not pump: power {:?}", g.current_power(h))); }
        if !g.current_abilities(h).unwrap().iter().any(is_triggered) { f.push("no attack trigger".into()); }
        f
    });
}

#[test]
fn cr_111_10s_map_token() {
    check_token(TokenSpec { rule: "111.10s", instruction: "Create a Map token.", name: None, types: ART, subtype: "Map", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        library_filler(g, 1);
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if g.battlefield.contains(&t) { f.push("not sacrificed".into()); }
        f
    });
}

#[test]
fn cr_111_10t_junk_token() {
    check_token(TokenSpec { rule: "111.10t", instruction: "Create a Junk token.", name: None, types: ART, subtype: "Junk", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        let lib = compile("Library card", "Mana cost: {0}\nType: Artifact");
        let top = g.create_object_from_definition(&lib, A, Zone::Library);
        let stable = g.object(top).unwrap().stable_id;
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        let now = g.find_object_by_stable_id(stable).unwrap();
        if g.object(now).unwrap().zone != Zone::Exile { f.push("top card not exiled".into()); }
        f
    });
}

#[test]
fn cr_111_10u_lander_token() {
    check_token(TokenSpec { rule: "111.10u", instruction: "Create a Lander token.", name: None, types: ART, subtype: "Lander", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        let forest = compile("Forest", "Type: Basic Land — Forest");
        g.create_object_from_definition(&forest, A, Zone::Library);
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        let count = |g: &GameState| g.battlefield.iter().filter(|id| g.current_card_types(**id).unwrap().contains(&CardType::Land)).count();
        let before = count(g);
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if count(g) != before + 1 { f.push("no land fetched".into()); }
        f
    });
}

#[test]
fn cr_111_10v_mutagen_token() {
    check_token(TokenSpec { rule: "111.10v", instruction: "Create a Mutagen token.", name: None, types: ART, subtype: "Mutagen", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, h| {
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if g.counter_count(h, CounterType::PlusOnePlusOne) != 1 { f.push("no counter".into()); }
        f
    });
}

#[test]
fn cr_111_10w_vibranium_token() {
    use ironsmith::static_abilities::StaticAbilityId;
    check_token(TokenSpec { rule: "111.10w", instruction: "Create a Vibranium token.", name: None, types: ART, subtype: "Vibranium", colors: ColorSet::COLORLESS, pt: None, activated: 1 }, |g, t, _| {
        let mut f = vec![];
        if !has_kw(g, t, StaticAbilityId::Indestructible) { f.push("not indestructible".into()); }
        if let Err(e) = activate_token(g, t) { f.push(e); }
        if g.player(A).unwrap().mana_pool.colorless != 1 { f.push("no {C}".into()); }
        f
    });
}

#[test]
fn cr_111_10x_heartwood_token() {
    check_token(TokenSpec { rule: "111.10x", instruction: "Create a Heartwood token.", name: None, types: ART, subtype: "Heartwood", colors: ColorSet::RED.union(ColorSet::GREEN), pt: None, activated: 1 }, |g, t, _| {
        let mut f = vec![];
        if let Err(e) = activate_token(g, t) { f.push(e); }
        let p = &g.player(A).unwrap().mana_pool;
        if p.red + p.green != 1 { f.push(format!("pool {p:?}")); }
        f
    });
}

#[test]
fn cr_111_9_create_named_legendary_token() {
    let mut game = new_game(A);
    let tolsimir = compile(
        "Tolsimir, Friend to Wolves",
        "Mana cost: {2}{G}{G}{W}\nType: Legendary Creature — Elf Scout\nPower/Toughness: 3/3\nWhen Tolsimir enters, create Voja, Friend to Elves, a legendary 3/3 green and white Wolf creature token.\nWhenever a Wolf you control enters, you gain 3 life and that creature fights up to one target creature you don't control.",
    );
    let id = game.create_object_from_definition(&tolsimir, A, Zone::Battlefield);
    let index = ability_index(&game, id, is_triggered);
    let mut dm = Script::default();
    resolve_effects_of(&mut game, id, A, index, &mut dm);
    let made = tokens(&game);
    assert_eq!(made.len(), 1);
    let t = made[0];
    let got = (
        game.current_name(t),
        game.current_has_supertype(t, ironsmith::Supertype::Legendary),
        game.current_power(t),
        game.current_toughness(t),
        game.current_colors(t),
    );
    assert_eq!(
        got,
        (Some("Voja, Friend to Elves".to_string()), true, Some(3), Some(3), Some(ColorSet::GREEN.union(ColorSet::WHITE))),
        "CR 111.9: the named legendary token has the given name and characteristics"
    );
}

#[test]
fn cr_111_11_token_by_name_uses_the_oracle_card() {
    let mut game = new_game(A);
    let disa = compile(
        "Disa the Restless",
        "Mana cost: {2}{B}{R}{G}\nType: Legendary Creature — Human Scout\nPower/Toughness: 5/6\nWhenever a Lhurgoyf permanent card is put into your graveyard from anywhere other than the battlefield, put it onto the battlefield.\nWhenever one or more creatures you control deal combat damage to a player, create a Tarmogoyf token.",
    );
    let id = game.create_object_from_definition(&disa, A, Zone::Battlefield);
    let index = game.object(id).unwrap().abilities.iter().rposition(is_triggered).unwrap();
    let mut dm = Script::default();
    resolve_effects_of(&mut game, id, A, index, &mut dm);
    let made = tokens(&game);
    assert_eq!(made.len(), 1, "CR 111.11: a token is created");
    let t = made[0];
    let subtypes: Vec<String> = game.current_subtypes(t).unwrap().iter().map(|s| format!("{s:?}")).collect();
    let got = (game.current_name(t), subtypes.contains(&"Lhurgoyf".to_string()), game.current_colors(t));
    assert_eq!(
        got,
        (Some("Tarmogoyf".to_string()), true, Some(ColorSet::GREEN)),
        "CR 111.11: a token created by name copies the Oracle card's characteristics"
    );
    assert_eq!(
        game.current_toughness(t).zip(game.current_power(t)).map(|(t, p)| t - p),
        Some(1),
        "CR 111.11: Tarmogoyf's */1+* characteristic-defining ability"
    );
}

#[test]
fn cr_111_12_mimic_vat_with_nothing_exiled_creates_no_token() {
    let mut game = new_game(A);
    let vat = compile(
        "Mimic Vat",
        "Mana cost: {3}\nType: Artifact\nImprint — Whenever a nontoken creature dies, you may exile that card. If you do, return each other card exiled with this artifact to its owner's graveyard.\n{3}, {T}: Create a token that's a copy of a card exiled with this artifact. It gains haste. Exile it at the beginning of the next end step.",
    );
    let id = game.create_object_from_definition(&vat, A, Zone::Battlefield);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
    let index = ability_index(&game, id, is_activated);
    let mut dm = Script::default();
    activate(&mut game, A, id, index, &mut dm).unwrap();
    resolve_all(&mut game, &mut dm);
    assert!(tokens(&game).is_empty(), "CR 111.12: copy of a nonexistent object creates no token");
}

#[test]
fn cr_111_13_112_1a_112_2_copied_permanent_spell_becomes_token_without_being_created() {
    let mut game = new_game(A);
    let observer = compile(
        "Token creation observer",
        "Mana cost: {0}\nType: Enchantment\nWhenever you create a token, you gain 1 life.",
    );
    game.create_object_from_definition(&observer, B, Zone::Battlefield);
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let card = game.create_object_from_definition(&bear, A, Zone::Hand);
    let mut dm = Script::default();
    cast(&mut game, A, card, &mut dm);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut dm,
    ).unwrap();
    assert_eq!(game.turn.priority_player, Some(B));
    let copier = compile("Creature spell copier", "Mana cost: {0}\nType: Instant\nCopy target creature spell.");
    let copier = game.create_object_from_definition(&copier, B, Zone::Hand);
    cast(&mut game, B, copier, &mut dm);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let is_copy = |g: &GameState, id: ObjectId| g.object(id).map(|o| o.kind) == Some(ironsmith::ObjectKind::SpellCopy);
    for _ in 0..4 {
        if game.stack.iter().any(|e| is_copy(&game, e.object_id)) || game.stack.len() < 2 { break; }
        ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut dm,
        ).unwrap();
    }
    let copy = game
        .stack
        .iter()
        .map(|e| e.object_id)
        .find(|id| is_copy(&game, *id))
        .expect("CR 112.1a: the copy is a spell on the stack");
    assert_eq!(game.object(copy).unwrap().owner, B, "CR 112.2: a copy's owner is the player who put it on the stack");
    assert_eq!(game.current_controller(copy), Some(B), "CR 112.2: controller is the player who put it on the stack");
    resolve_all(&mut game, &mut dm);
    let made = tokens(&game);
    assert_eq!(made.len(), 1, "CR 111.13: the copied permanent spell becomes a token");
    assert_eq!(game.object(made[0]).unwrap().owner, B);
    assert_eq!(life(&game, B), 20, "CR 111.13: the token was not 'created'");
}

// ---------------------------------------------------------------------------
// 113: abilities
// ---------------------------------------------------------------------------

#[test]
fn cr_113_1b_player_hexproof_is_an_ability_the_player_has() {
    let mut game = new_game(B);
    let leyline = compile(
        "Leyline of Sanctity",
        "Mana cost: {2}{W}{W}\nType: Enchantment\nIf this card is in your opening hand, you may begin the game with it on the battlefield.\nYou have hexproof. (You can't be the target of spells or abilities your opponents control.)",
    );
    game.create_object_from_definition(&leyline, A, Zone::Battlefield);
    let shock = compile("Player shock", "Mana cost: {0}\nType: Instant\nThis spell deals 1 damage to target player.");
    let shock = game.create_object_from_definition(&shock, B, Zone::Hand);
    let mut dm = Script::default();
    cast(&mut game, B, shock, &mut dm);
    let seen = dm.seen_targets.concat();
    assert!(seen.contains(&Target::Player(B)), "control: Bob is targetable");
    assert!(!seen.contains(&Target::Player(A)), "CR 113.1b: Alice has hexproof from Leyline: {seen:?}");
}

#[test]
fn cr_113_3b_activated_ability_at_instant_speed_uses_the_stack() {
    let mut game = new_game(B);
    game.turn.priority_player = Some(A);
    let rod = compile("Life rod", "Mana cost: {0}\nType: Artifact\n{0}: You gain 1 life.");
    let id = game.create_object_from_definition(&rod, A, Zone::Battlefield);
    let index = ability_index(&game, id, is_activated);
    let mut dm = Script::default();
    activate(&mut game, A, id, index, &mut dm).expect("CR 113.3b: activate whenever the player has priority");
    assert_eq!(game.stack.len(), 1, "CR 113.3b: the ability is put on the stack");
    assert_eq!(life(&game, A), 20);
    resolve_all(&mut game, &mut dm);
    assert_eq!(life(&game, A), 21, "CR 113.3b: it resolves from the stack");
}

#[test]
fn cr_113_3d_static_ability_functions_only_on_the_battlefield() {
    let anthem = compile("Anthem", "Mana cost: {0}\nType: Enchantment\nCreatures you control get +1/+1.");
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    for (zone, expected) in [(Zone::Battlefield, 3), (Zone::Graveyard, 2), (Zone::Hand, 2), (Zone::Exile, 2)] {
        let mut game = new_game(A);
        game.create_object_from_definition(&anthem, A, zone);
        let id = game.create_object_from_definition(&bear, A, Zone::Battlefield);
        assert_eq!(game.current_power(id), Some(expected), "CR 113.3d: anthem in {zone:?}");
    }
}

fn cast_restriction_case(line: &str, legal_step: ironsmith::Step) -> Result<(), String> {
    let def = try_compile("Timing trick", &format!("Mana cost: {{0}}\nType: Instant\n{line}\nYou gain 1 life."))
        .map_err(|e| format!("{line:?} is not supported: {e}"))?;
    let mut game = new_game(A);
    let id = game.create_object_from_definition(&def, A, Zone::Hand);
    if cast_action(&game, A, id).is_some() {
        return Err(format!("{line:?}: castable in a main phase"));
    }
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(legal_step);
    if cast_action(&game, A, id).is_none() {
        return Err(format!("{line:?}: not castable in {legal_step:?}"));
    }
    Ok(())
}

#[test]
fn cr_113_6e_cast_only_during_combat_restricts_casting_from_hand() {
    let r = cast_restriction_case("Cast this spell only during combat.", ironsmith::Step::BeginCombat);
    assert!(r.is_ok(), "CR 113.6e: {r:?}");
}

#[test]
fn cr_113_6f_squee_may_be_cast_from_the_graveyard() {
    let squee = compile(
        "Squee, Dubious Monarch",
        "Mana cost: {2}{R}\nType: Legendary Creature — Goblin Noble\nPower/Toughness: 2/2\nHaste\nWhenever Squee attacks, create a 1/1 red Goblin creature token that's tapped and attacking.\nYou may cast this card from your graveyard by paying {3}{R} and exiling four other cards from your graveyard rather than paying its mana cost.",
    );
    let filler = compile("Filler", "Mana cost: {0}\nType: Artifact");
    let mut game = new_game(A);
    let id = game.create_object_from_definition(&squee, A, Zone::Graveyard);
    for _ in 0..4 {
        game.create_object_from_definition(&filler, A, Zone::Graveyard);
    }
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red, 4);
    let found = legal(&game, A).into_iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, .. } if spell_id == id));
    assert!(found, "CR 113.6f: the zone-permission ability functions in the graveyard");
}

#[test]
fn cr_113_6g_cant_be_countered_functions_on_the_stack() {
    let mut game = new_game(A);
    let sure = compile("Sure thing", "Mana cost: {0}\nType: Instant\nThis spell can't be countered.\nYou gain 3 life.");
    let sure = game.create_object_from_definition(&sure, A, Zone::Hand);
    let mut dm = Script::default();
    cast(&mut game, A, sure, &mut dm);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut dm,
    ).unwrap();
    let counter = compile("Counterspell", "Mana cost: {0}\nType: Instant\nCounter target spell.");
    let counter = game.create_object_from_definition(&counter, B, Zone::Hand);
    cast(&mut game, B, counter, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert_eq!(life(&game, A), 23, "CR 113.6g: the uncounterable spell still resolves");
}

#[test]
fn cr_113_6h_113_6i_entry_abilities_apply_as_the_object_enters() {
    let tapped = compile("Tapped entrant", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2\nThis creature enters tapped.\nThis creature enters with two +1/+1 counters on it.");
    let keepers = compile("Counterless entrant", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2\nThis creature enters with two +1/+1 counters on it.\nThis creature can't have counters put on it.");
    let mut results = Vec::new();
    for def in [&tapped, &keepers] {
        let mut game = new_game(A);
        let id = game.create_object_from_definition(def, A, Zone::Hand);
        let stable = game.object(id).unwrap().stable_id;
        let mut dm = Script::default();
        cast(&mut game, A, id, &mut dm);
        resolve_all(&mut game, &mut dm);
        let now = game.find_object_by_stable_id(stable).unwrap();
        results.push((game.is_tapped(now), game.counter_count(now, CounterType::PlusOnePlusOne)));
    }
    assert_eq!(results[0], (true, 2), "CR 113.6h: enters tapped and with counters");
    assert_eq!(results[1].1, 0, "CR 113.6i: 'can't have counters' functions as it enters");
}

/// Cycle a card with a self "when you cycle" trigger next to a battlefield cycle observer.
/// Returns (can activate from hand, card zone after, hand size after, life after).
fn cycle_run() -> (bool, Zone, usize, i32) {
    let mut game = new_game(A);
    let def = compile("Cycler", "Mana cost: {3}\nType: Creature — Beast\nPower/Toughness: 3/3\nCycling {0}\nWhen you cycle this card, you gain 2 life.");
    library_filler(&mut game, 1);
    let observer = compile("Cycle observer", "Mana cost: {0}\nType: Enchantment\nWhenever you cycle a card, you gain 1 life.");
    game.create_object_from_definition(&observer, A, Zone::Battlefield);
    let id = game.create_object_from_definition(&def, A, Zone::Hand);
    let stable = game.object(id).unwrap().stable_id;
    let index = ability_index(&game, id, is_activated);
    let available = can_activate(&game, A, id);
    let mut dm = Script::default();
    activate(&mut game, A, id, index, &mut dm).unwrap();
    resolve_all(&mut game, &mut dm);
    let now = game.find_object_by_stable_id(stable).unwrap();
    (available, game.object(now).unwrap().zone, game.player(A).unwrap().hand.len(), life(&game, A))
}

#[test]
fn cr_113_6j_cycling_cost_functions_from_the_hand() {
    let (available, zone, hand, life_after) = cycle_run();
    assert!(available, "CR 113.6j: cycling is activatable from the hand");
    assert_eq!((zone, hand), (Zone::Graveyard, 1), "CR 113.6j: discarded as the cost and drew a card");
    assert!(life_after >= 21, "control: battlefield cycle observer triggered");
}

#[test]
fn cr_113_6k_cycle_trigger_functions_from_the_zone_it_triggers_from() {
    let (_, _, _, life_after) = cycle_run();
    assert_eq!(life_after, 23, "CR 113.6k: the card's own 'when you cycle' trigger (+2) fires alongside the observer (+1)");
}

#[test]
fn cr_113_6m_reassembling_skeleton_only_from_the_graveyard() {
    let skeleton = compile("Reassembling Skeleton", "Mana cost: {1}{B}\nType: Creature — Skeleton Warrior\nPower/Toughness: 1/1\n{1}{B}: Return this card from your graveyard to the battlefield tapped.");
    let mut failures = Vec::new();
    for zone in [Zone::Battlefield, Zone::Hand, Zone::Exile, Zone::Graveyard] {
        let mut game = new_game(A);
        let id = game.create_object_from_definition(&skeleton, A, zone);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 2);
        let available = can_activate(&game, A, id);
        if available != (zone == Zone::Graveyard) {
            failures.push(format!("{zone:?}: available={available}"));
        }
        if zone == Zone::Graveyard && available {
            let stable = game.object(id).unwrap().stable_id;
            let mut dm = Script::default();
            let index = ability_index(&game, id, is_activated);
            activate(&mut game, A, id, index, &mut dm).unwrap();
            resolve_all(&mut game, &mut dm);
            let now = game.find_object_by_stable_id(stable).unwrap();
            if game.object(now).unwrap().zone != Zone::Battlefield || !game.is_tapped(now) {
                failures.push("did not return tapped".into());
            }
        }
    }
    assert!(failures.is_empty(), "CR 113.6m: {failures:?}");
}

#[test]
fn cr_113_6n_deck_construction_rule_is_modeled_as_a_copy_limit() {
    let rats = compile(
        "Relentless Rats",
        "Mana cost: {1}{B}{B}\nType: Creature — Rat\nPower/Toughness: 2/2\nThis creature gets +1/+1 for each other creature on the battlefield named Relentless Rats.\nA deck can have any number of cards named Relentless Rats.",
    );
    let limit = rats.abilities.iter().find_map(|a| match &a.kind {
        AbilityKind::Static(s) => s.deck_copy_limit(),
        _ => None,
    });
    assert_eq!(limit, Some(None), "CR 113.6n: the deck-construction ability lifts the copy limit");
}

#[test]
fn cr_113_9_spell_counter_cannot_target_abilities_but_ability_counter_can() {
    let mut game = new_game(A);
    let rod = compile("Life rod", "Mana cost: {0}\nType: Artifact\n{0}: You gain 1 life.");
    let id = game.create_object_from_definition(&rod, A, Zone::Battlefield);
    let mut dm = Script::default();
    let index = ability_index(&game, id, is_activated);
    activate(&mut game, A, id, index, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1);
    let counterspell = compile("Counterspell", "Mana cost: {0}\nType: Instant\nCounter target spell.");
    let stifle = compile("Stifle", "Mana cost: {0}\nType: Instant\nCounter target activated or triggered ability. (Mana abilities can't be targeted.)");
    let cs = game.create_object_from_definition(&counterspell, A, Zone::Hand);
    let st = game.create_object_from_definition(&stifle, A, Zone::Hand);
    assert!(cast_action(&game, A, cs).is_none(), "CR 113.9: an ability on the stack is not a spell");
    assert!(cast_action(&game, A, st).is_some(), "CR 113.9: abilities can be countered by ability counters");
    cast(&mut game, A, st, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert_eq!(life(&game, A), 20, "CR 113.9: the ability was countered");
}

#[test]
fn cr_113_10a_granted_activation_instructions_become_part_of_the_ability() {
    let aura = compile(
        "Sorcery speed grant",
        "Mana cost: {0}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has \"{T}: You gain 1 life. Activate only as a sorcery.\"",
    );
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let mut results = Vec::new();
    for own_main in [true, false] {
        let mut game = new_game(if own_main { A } else { B });
        game.turn.priority_player = Some(A);
        let host = game.create_object_from_definition(&bear, A, Zone::Battlefield);
        game.remove_summoning_sickness(host);
        let a = game.create_object_from_definition(&aura, A, Zone::Battlefield);
        assert!(game.attach_object_to_target(a, ironsmith::object::AttachmentTarget::Object(host)));
        results.push(can_activate(&game, A, host));
    }
    assert_eq!(results, vec![true, false], "CR 113.10a: the granted ability keeps 'activate only as a sorcery'");
}

#[test]
fn cr_113_10b_113_10c_removal_takes_all_instances_and_latest_effect_wins() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::StaticAbility;
    let def = compile("Double flier", "Mana cost: {0}\nType: Creature — Bird\nPower/Toughness: 1/1\nFlying\nFlying");
    let apply = |game: &mut GameState, id: ObjectId, m: Modification| {
        ApplyContinuousEffect::new(EffectTarget::Specific(id), m, ironsmith::Until::EndOfTurn)
            .execute(game, &mut EffectContext::new_default(id, A))
            .unwrap();
    };
    let mut game = new_game(A);
    let id = game.create_object_from_definition(&def, A, Zone::Battlefield);
    assert!(game.object_has_static_ability_id(id, StaticAbilityId::Flying));
    apply(&mut game, id, Modification::RemoveAbility(StaticAbility::flying()));
    assert!(!game.object_has_static_ability_id(id, StaticAbilityId::Flying), "CR 113.10b: both printed instances are removed");
    apply(&mut game, id, Modification::AddAbility(StaticAbility::flying()));
    assert!(game.object_has_static_ability_id(id, StaticAbilityId::Flying), "CR 113.10c: later grant prevails");
    apply(&mut game, id, Modification::RemoveAbility(StaticAbility::flying()));
    assert!(!game.object_has_static_ability_id(id, StaticAbilityId::Flying), "CR 113.10c: later removal prevails");
}

#[test]
fn cr_113_11_cant_have_flying_removes_and_blocks_later_grants() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::StaticAbility;
    let mut game = new_game(A);
    let archetype = compile(
        "Archetype of Imagination",
        "Mana cost: {4}{U}{U}\nType: Enchantment Creature — Human Wizard\nPower/Toughness: 3/2\nCreatures you control have flying.\nCreatures your opponents control lose flying and can't have or gain flying.",
    );
    let arch = game.create_object_from_definition(&archetype, A, Zone::Battlefield);
    let flier = compile("Wind Drake", "Mana cost: {0}\nType: Creature — Drake\nPower/Toughness: 2/2\nFlying");
    let theirs = game.create_object_from_definition(&flier, B, Zone::Battlefield);
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let theirs2 = game.create_object_from_definition(&bear, B, Zone::Battlefield);
    assert!(game.object_has_static_ability_id(arch, StaticAbilityId::Flying), "Archetype grants its controller's creatures flying");
    assert!(!game.object_has_static_ability_id(theirs, StaticAbilityId::Flying), "CR 113.11: an object that can't have flying loses it");
    ApplyContinuousEffect::new(EffectTarget::Specific(theirs2), Modification::AddAbility(StaticAbility::flying()), ironsmith::Until::EndOfTurn)
        .execute(&mut game, &mut EffectContext::new_default(theirs2, B))
        .unwrap();
    assert!(!game.object_has_static_ability_id(theirs2, StaticAbilityId::Flying), "CR 113.11: a later effect can't add flying");
}

#[test]
fn cr_113_12_muraganda_distinguishes_granted_abilities_from_stated_qualities() {
    let petroglyphs = compile("Muraganda Petroglyphs", "Mana cost: {3}{G}\nType: Enchantment\nCreatures with no abilities get +2/+2.");
    let bear = compile("Runeclaw Bear", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let mut failures = Vec::new();
    for (label, aura_text, expected) in [
        ("none", None, 4),
        ("has flying", Some("Enchanted creature has flying."), 2),
        ("is red", Some("Enchanted creature is red."), 4),
        ("can't be blocked", Some("Enchanted creature can't be blocked."), 4),
    ] {
        let mut game = new_game(A);
        game.create_object_from_definition(&petroglyphs, A, Zone::Battlefield);
        let host = game.create_object_from_definition(&bear, A, Zone::Battlefield);
        if let Some(text) = aura_text {
            let aura = compile(&format!("Aura {label}"), &format!("Mana cost: {{0}}\nType: Enchantment — Aura\nEnchant creature\n{text}"));
            let a = game.create_object_from_definition(&aura, A, Zone::Battlefield);
            assert!(game.attach_object_to_target(a, ironsmith::object::AttachmentTarget::Object(host)));
        }
        let p = game.current_power(host);
        if p != Some(expected) {
            failures.push(format!("{label}: power {p:?}, expected {expected}"));
        }
    }
    assert!(failures.is_empty(), "CR 113.12: {failures:?}");
}

#[test]
fn cr_113_2a_cant_block_is_an_ability_removed_by_ability_loss() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    let petroglyphs = compile("Muraganda Petroglyphs", "Mana cost: {3}{G}\nType: Enchantment\nCreatures with no abilities get +2/+2.");
    let craven = compile("Craven", "Mana cost: {0}\nType: Creature — Goblin\nPower/Toughness: 2/2\nThis creature can't block.");
    let mut game = new_game(A);
    game.create_object_from_definition(&petroglyphs, A, Zone::Battlefield);
    let id = game.create_object_from_definition(&craven, A, Zone::Battlefield);
    assert_eq!(game.current_power(id), Some(2), "CR 113.2a: 'can't block' is an ability, so the creature has an ability");
    ApplyContinuousEffect::new(EffectTarget::Specific(id), Modification::RemoveAllAbilities, ironsmith::Until::EndOfTurn)
        .execute(&mut game, &mut EffectContext::new_default(id, A))
        .unwrap();
    assert_eq!(game.current_power(id), Some(4), "CR 113.2a: after losing the ability it has none");
}

// ---------------------------------------------------------------------------
// 108.3a / 109.2d / 109.4d-g: casual variant ownership and control
// ---------------------------------------------------------------------------

fn bare_card(name: &str, types: Vec<CardType>) -> CardDefinition {
    CardDefinition::new(ironsmith::CardBuilder::new(ironsmith::CardId::new(), name).card_types(types).build())
}

#[test]
fn cr_108_3a_109_4d_planar_controller_owns_communal_deck_and_controls_planes() {
    use ironsmith::events::KeywordActionKind;
    use ironsmith::triggers::Trigger;
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let cards: Vec<_> = (0..20)
        .map(|i| {
            let mut d = bare_card(&format!("Communal Plane {i}"), vec![CardType::Plane]);
            d.abilities.push(ironsmith::Ability::triggered(
                Trigger::keyword_action_from_source(KeywordActionKind::ChaosEnsues, ironsmith::PlayerFilter::You),
                vec![ironsmith::Effect::gain_life(1)],
            ));
            (d, ironsmith::PlanarCardKind::Plane)
        })
        .collect();
    game.enable_planechase_communal(cards).expect("communal planar deck");
    game.reveal_starting_plane().unwrap();
    let kinds: Vec<ObjectId> = game.planechase.as_ref().unwrap().card_kinds.keys().copied().collect();
    let controller = game.planar_controller().expect("planar controller");
    assert!(kinds.iter().all(|id| game.object(*id).unwrap().owner == controller), "CR 108.3a: planar controller owns every planar card");
    let plane = game.face_up_planar_objects()[0];
    assert_eq!(game.controller_of_id(plane), Some(controller), "CR 109.4d: the face-up plane is controlled by the planar controller");
    game.chaos_ensues(controller, plane).unwrap();
    let mut queue = TriggerQueue::new();
    ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].controller, controller, "CR 109.4d: the plane's ability is controlled by the planar controller");
    game.next_turn();
    let next = game.planar_controller().unwrap();
    assert_ne!(next, controller, "planar controller rotates with the active player");
    assert!(kinds.iter().all(|id| game.object(*id).map(|o| o.owner == next).unwrap_or(true)), "CR 108.3a: ownership follows the planar controller");
}

#[test]
fn cr_109_2d_109_4f_this_scheme_and_scheme_control() {
    use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
    use ironsmith::triggers::Trigger;
    let mut cards: Vec<CardDefinition> = (0..20).map(|i| bare_card(&format!("Alice Scheme {i}"), vec![CardType::Scheme])).collect();
    cards[19] = bare_card("Triggered Scheme", vec![CardType::Scheme]);
    cards[19].abilities.push(ironsmith::Ability::triggered(
        Trigger::keyword_action_from_source(KeywordActionKind::SetSchemeInMotion, ironsmith::PlayerFilter::You),
        vec![ironsmith::Effect::gain_life(1)],
    ));
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.enable_archenemy(ironsmith::ArchenemyVariant::Default, vec![(A, cards)]).unwrap();
    let source = game.archenemy.as_ref().unwrap().scheme_decks.values().flatten().copied()
        .find(|o| game.object(*o).is_some_and(|c| c.name == "Triggered Scheme")).unwrap();
    {
        let deck = game.archenemy.as_mut().unwrap().scheme_decks.get_mut(&A).unwrap();
        let i = deck.iter().position(|c| *c == source).unwrap();
        deck.remove(i);
        deck.push(source);
    }
    assert_eq!(game.set_scheme_in_motion(A).unwrap(), source);
    assert_eq!(game.object(source).unwrap().zone, Zone::Command);
    assert_eq!(game.controller_of_id(source), Some(A), "CR 109.4f: a scheme is controlled by its owner");
    let events = game.take_pending_trigger_events();
    let event = events
        .iter()
        .find(|e| e.downcast::<KeywordActionEvent>().is_some_and(|k| k.action == KeywordActionKind::SetSchemeInMotion))
        .expect("set in motion event");
    let entries = ironsmith::check_triggers(&game, event);
    assert_eq!(entries.len(), 1, "CR 109.2d: 'this scheme' trigger refers to the command-zone scheme");
    assert_eq!(entries[0].source, source);
    assert_eq!(entries[0].controller, A, "CR 109.4f: its ability is controlled by the scheme's owner");
}

#[test]
fn cr_109_4g_conspiracy_is_controlled_by_its_owner() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut public = compile("Public Conspiracy", "Type: Enchantment\nCreatures you control get +1/+1.");
    public.card.card_types = vec![CardType::Conspiracy].into();
    game.enable_conspiracy(vec![(A, vec![ironsmith::ConspiracySetupCard { definition: public, agenda_names: Vec::new() }])]).unwrap();
    let id = game.conspiracy_cards()[0];
    assert_eq!(game.controller_of_id(id), Some(A));
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let mine = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    let theirs = game.create_object_from_definition(&bear, B, Zone::Battlefield);
    assert_eq!((game.current_power(mine), game.current_power(theirs)), (Some(3), Some(2)), "CR 113.6p: its static ability functions in the command zone for its owner");
    game.set_current_controller(id, B).expect("controller fixture");
    assert_eq!(game.controller_of_id(id), Some(A), "CR 109.4g: a conspiracy is controlled by its owner");
}

#[test]
fn cr_107_3f_undefined_x_is_chosen_by_the_controller() {
    let mut game = new_game(A);
    let assassin = compile(
        "Nightshade Assassin",
        "Mana cost: {2}{B}{B}\nType: Creature — Human Assassin\nPower/Toughness: 2/1\nFirst strike\nWhen this creature enters, you may reveal X black cards in your hand. If you do, target creature gets -X/-X until end of turn.\nMadness {1}{B}",
    );
    let id = game.create_object_from_definition(&assassin, A, Zone::Battlefield);
    let black = compile("Black card", "Mana cost: {B}\nType: Instant\nScry 1.");
    for _ in 0..3 {
        game.create_object_from_definition(&black, A, Zone::Hand);
    }
    let ogre = compile("Hill Giant", "Mana cost: {3}{R}\nType: Creature — Giant\nPower/Toughness: 3/3");
    let target = game.create_object_from_definition(&ogre, B, Zone::Battlefield);
    let index = ability_index(&game, id, is_triggered);
    let ability = game.object(id).unwrap().abilities[index].clone();
    let AbilityKind::Triggered(t) = ability.kind else { unreachable!() };
    let mut entry = StackEntry::ability(id, A, t.effects);
    entry.targets = vec![Target::Object(target)];
    game.stack.push(entry);
    let mut dm = Script { x: Some(2), take: Some(2), accept: true, ..Default::default() };
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).expect("resolve trigger");
    assert_eq!(
        game.current_toughness(target),
        Some(1),
        "CR 107.3f: the controller chose X=2 (numbers {:?}, objects {:?})",
        dm.seen_numbers,
        dm.seen_object_candidates
    );
}

#[test]
fn cr_107_3p_y_follows_the_same_rules_as_x() {
    let mut game = new_game(A);
    let aspect = compile(
        "Aspect of Wolf",
        "Mana cost: {1}{G}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +X/+Y, where X is half the number of Forests you control, rounded down, and Y is half the number of Forests you control, rounded up.",
    );
    let forest = compile("Forest", "Type: Basic Land — Forest");
    for _ in 0..3 {
        game.create_object_from_definition(&forest, A, Zone::Battlefield);
    }
    let bear = compile("Grizzly Bears", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let host = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    let aura = game.create_object_from_definition(&aspect, A, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
    assert_eq!(
        (game.current_power(host), game.current_toughness(host)),
        (Some(3), Some(4)),
        "CR 107.3p: X (1) and Y (2) are each defined by the text"
    );
}

#[test]
fn cr_107_4g_rage_extractor_h_means_any_phyrexian_symbol() {
    let extractor = compile(
        "Rage Extractor",
        "Mana cost: {4}{R/P}\nType: Artifact\n({R/P} can be paid with either {R} or 2 life.)\nWhenever you cast a spell with {H} in its mana cost, this artifact deals damage equal to that spell's mana value to any target.",
    );
    let mut failures = Vec::new();
    for (label, cost, symbol, expected_bob) in [
        ("black phyrexian", "{1}{B/P}", ManaSymbol::Black, 18),
        ("green-white phyrexian", "{1}{G/W/P}", ManaSymbol::Green, 18),
        ("no phyrexian", "{1}{B}", ManaSymbol::Black, 20),
    ] {
        let mut game = new_game(A);
        game.create_object_from_definition(&extractor, A, Zone::Battlefield);
        let def = compile(&format!("Spell {label}"), &format!("Mana cost: {cost}\nType: Sorcery\nScry 1."));
        let spell = game.create_object_from_definition(&def, A, Zone::Hand);
        library_filler(&mut game, 2);
        game.player_mut(A).unwrap().mana_pool.add(symbol, 1);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        let mut dm = Script { prefer: vec![Target::Player(B)], ..Default::default() };
        cast(&mut game, A, spell, &mut dm);
        resolve_all(&mut game, &mut dm);
        if life(&game, B) != expected_bob {
            failures.push(format!("{label}: Bob at {}", life(&game, B)));
        }
    }
    assert!(failures.is_empty(), "CR 107.4g: {failures:?}");
}
