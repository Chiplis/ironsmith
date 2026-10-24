//! CR audit (pending-entry pass, package p12): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, EffectExecutor, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::LegalAction;
use ironsmith::decisions::context::{
    AttackersContext, BlockersContext, BooleanContext, PriorityContext, SelectObjectsContext,
    TargetsContext,
};
use ironsmith::decisions::spec::{AttackerDeclaration, BlockerDeclaration};
use ironsmith::game_state::{Step, Target};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CounterType, DecisionMaker, ObjectId, Zone};
use std::cell::RefCell;
use std::rc::Rc;

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

fn card(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture card {name} must compile strictly: {e:?}"))
}

fn card_loose(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, true)
        .unwrap_or_else(|e| panic!("fixture card {name} must compile: {e:?}"))
}

fn basic(kind: &str) -> CardDefinition {
    let sym = match kind {
        "Plains" => "W",
        "Island" => "U",
        "Swamp" => "B",
        "Mountain" => "R",
        _ => "G",
    };
    card(
        kind,
        &format!("Type: Basic Land — {kind}\nFirst printed set: Limited Edition Alpha\n({{T}}: Add {{{sym}}}.)"),
    )
}

fn vanilla(name: &str, p: i32, t: i32) -> CardDefinition {
    card(
        name,
        &format!("Mana cost: {{1}}\nType: Creature — Bear\nPower/Toughness: {p}/{t}"),
    )
}

fn new_game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let filler = vanilla("Library Filler", 1, 1);
    for p in [ALICE, BOB] {
        for _ in 0..12 {
            g.create_object_from_definition(&filler, p, Zone::Library);
        }
    }
    g
}

fn put(g: &mut GameState, def: &CardDefinition, owner: PlayerId, zone: Zone) -> ObjectId {
    g.create_object_from_definition(def, owner, zone)
}

fn lands(g: &mut GameState, kind: &str, n: usize, owner: PlayerId) {
    let def = basic(kind);
    for _ in 0..n {
        g.create_object_from_definition(&def, owner, Zone::Battlefield);
    }
}

fn zone_ids(g: &GameState) -> Vec<ObjectId> {
    let mut v: Vec<ObjectId> = g.battlefield.iter().copied().collect();
    v.extend(g.exile.iter().copied());
    v.extend(g.command_zone.iter().copied());
    v.extend(g.stack.iter().map(|e| e.object_id));
    for p in [ALICE, BOB] {
        let pl = g.player(p).unwrap();
        v.extend(pl.hand.iter().copied());
        v.extend(pl.graveyard.iter().copied());
        v.extend(pl.library.iter().copied());
    }
    v
}

/// All live objects named `name`, with their zones.
fn named(g: &GameState, name: &str) -> Vec<(ObjectId, Zone)> {
    zone_ids(g)
        .into_iter()
        .filter_map(|id| g.object(id).map(|o| (id, o.name.to_string(), o.zone)))
        .filter(|(_, n, _)| n == name)
        .map(|(id, _, z)| (id, z))
        .collect()
}

fn zone_of(g: &GameState, name: &str) -> Option<Zone> {
    named(g, name).first().map(|(_, z)| *z)
}

fn id_of(g: &GameState, name: &str) -> ObjectId {
    named(g, name)
        .first()
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("fixture object {name} not found"))
}

fn life(g: &GameState, p: PlayerId) -> i32 {
    g.player(p).unwrap().life
}

type Play = Box<dyn FnMut(&GameState, &LegalAction) -> bool>;
type Observer = Box<dyn FnMut(&GameState)>;

/// Scripted decision maker driving full turns through `execute_turn_with`.
struct Script {
    attack: Vec<ObjectId>,
    blocks: Vec<(ObjectId, ObjectId)>,
    plays: Vec<Play>,
    boolean: bool,
    prefer_targets: Vec<Target>,
    prefer_objects: Vec<ObjectId>,
    observe: Option<Observer>,
    booleans_seen: Vec<String>,
    actions_taken: Vec<String>,
    number: Option<u32>,
    numbers_seen: Vec<String>,
    attack_target: Option<AttackTarget>,
    prompts: Vec<String>,
    option_pref: Vec<&'static str>,
    option_repeat: usize,
}

impl Default for Script {
    fn default() -> Self {
        Script {
            attack: vec![],
            blocks: vec![],
            plays: vec![],
            boolean: true,
            prefer_targets: vec![],
            prefer_objects: vec![],
            observe: None,
            booleans_seen: vec![],
            actions_taken: vec![],
            number: None,
            numbers_seen: vec![],
            attack_target: None,
            prompts: vec![],
            option_pref: vec![],
            option_repeat: 1,
        }
    }
}

fn cast_named(name: &'static str) -> Play {
    Box::new(move |g, a| match a {
        LegalAction::CastSpell { spell_id, .. } => {
            g.object(*spell_id).is_some_and(|o| &*o.name == name)
        }
        _ => false,
    })
}

fn cast_named_empty_stack(name: &'static str) -> Play {
    Box::new(move |g, a| match a {
        LegalAction::CastSpell { spell_id, .. } => {
            g.stack.is_empty() && g.object(*spell_id).is_some_and(|o| &*o.name == name)
        }
        _ => false,
    })
}

/// Live objects whose name contains `needle`.
fn named_like(g: &GameState, needle: &str) -> Vec<(ObjectId, Zone)> {
    zone_ids(g)
        .into_iter()
        .filter_map(|id| g.object(id).map(|o| (id, o.name.contains(needle), o.zone)))
        .filter(|(_, m, _)| *m)
        .map(|(id, _, z)| (id, z))
        .collect()
}

fn activate_named(name: &'static str) -> Play {
    Box::new(move |g, a| match a {
        LegalAction::ActivateAbility { source, .. } => {
            g.object(*source).is_some_and(|o| &*o.name == name)
        }
        _ => false,
    })
}

impl Script {
    fn see(&mut self, g: &GameState) {
        if let Some(o) = self.observe.as_mut() {
            o(g);
        }
    }
}

impl DecisionMaker for Script {
    fn on_auto_pass(&mut self, g: &GameState, _p: PlayerId) {
        self.see(g);
    }
    fn decide_priority(&mut self, g: &GameState, ctx: &PriorityContext) -> LegalAction {
        self.see(g);
        if !self.plays.is_empty() {
            let pred = &mut self.plays[0];
            if let Some(a) = ctx.actions.iter().find(|a| pred(g, a)).cloned() {
                self.plays.remove(0);
                self.actions_taken.push(format!("{a:?}"));
                return a;
            }
        }
        ctx.actions
            .iter()
            .find(|a| matches!(a, LegalAction::PassPriority))
            .cloned()
            .unwrap_or_else(|| ctx.actions[0].clone())
    }
    fn decide_number(
        &mut self,
        _g: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        self.numbers_seen.push(format!("{:?}", ctx));
        self.number.map_or(ctx.min, |n| n.clamp(ctx.min, ctx.max))
    }
    fn decide_boolean(&mut self, _g: &GameState, ctx: &BooleanContext) -> bool {
        self.booleans_seen.push(format!("{:?}: {}", ctx.player, ctx.description));
        self.boolean
    }
    fn decide_attackers(
        &mut self,
        _g: &GameState,
        ctx: &AttackersContext,
    ) -> Vec<AttackerDeclaration> {
        ctx.attacker_options
            .iter()
            .filter(|o| self.attack.contains(&o.creature) || o.must_attack)
            .filter_map(|o| {
                let target = o
                    .valid_targets
                    .iter()
                    .find(|t| Some(*t) == self.attack_target.as_ref())
                    .or_else(|| o.valid_targets.iter().find(|t| matches!(t, AttackTarget::Player(_))))
                    .or(o.valid_targets.first())?
                    .clone();
                Some(AttackerDeclaration {
                    creature: o.creature,
                    target,
                })
            })
            .collect()
    }
    fn decide_blockers(&mut self, _g: &GameState, ctx: &BlockersContext) -> Vec<BlockerDeclaration> {
        let mut out = vec![];
        for (blocker, attacker) in &self.blocks {
            if ctx.blocker_options.iter().any(|o| {
                o.attacker == *attacker && o.valid_blockers.iter().any(|(b, _)| b == blocker)
            }) {
                out.push(BlockerDeclaration {
                    blocker: *blocker,
                    blocking: *attacker,
                });
            }
        }
        out
    }
    fn decide_targets(&mut self, _g: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut out = vec![];
        for req in &ctx.requirements {
            let max = req.max_targets.unwrap_or(req.legal_targets.len()).max(req.min_targets);
            let mut chosen: Vec<Target> = self
                .prefer_targets
                .iter()
                .filter(|t| req.legal_targets.contains(t))
                .take(max)
                .cloned()
                .collect();
            if chosen.len() < req.min_targets.max(1).min(req.legal_targets.len()) {
                for t in &req.legal_targets {
                    if chosen.len() >= req.min_targets.max(1).min(req.legal_targets.len()) {
                        break;
                    }
                    if !chosen.contains(t) {
                        chosen.push(t.clone());
                    }
                }
            }
            out.extend(chosen);
        }
        out
    }
    fn decide_options(
        &mut self,
        _g: &GameState,
        ctx: &ironsmith::SelectOptionsContext,
    ) -> Vec<usize> {
        self.prompts.push(format!(
            "options[{}..{:?}] {}: {:?}",
            ctx.min,
            ctx.max,
            ctx.description,
            ctx.options.iter().map(|o| (o.index, o.legal, o.description.clone())).collect::<Vec<_>>()
        ));
        let mut picked = vec![];
        for o in ctx.options.iter().filter(|o| o.legal) {
            if self.option_pref.iter().any(|p| o.description.contains(p)) {
                for _ in 0..self.option_repeat.max(1) {
                    if picked.len() < ctx.max.max(1) {
                        picked.push(o.index);
                    }
                }
            }
        }
        if !picked.is_empty() {
            return picked;
        }
        if ctx.min == 0 {
            return Vec::new();
        }
        let mut selected = Vec::new();
        let mut total = 0usize;
        for option in ctx.options.iter().filter(|o| o.legal) {
            let cost = option.point_cost.max(1) as usize;
            if total.saturating_add(cost) > ctx.max {
                continue;
            }
            selected.push(option.index);
            total += cost;
            if total >= ctx.min {
                break;
            }
        }
        selected
    }
    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.prompts.push(format!(
            "objects[{}..{:?}] {}: {:?}",
            ctx.min,
            ctx.max,
            ctx.description,
            ctx.candidates.iter().map(|c| (c.id, c.legal)).collect::<Vec<_>>()
        ));
        let preferred: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal && self.prefer_objects.contains(&c.id))
            .map(|c| c.id)
            .collect();
        if !preferred.is_empty() {
            let max = ctx.max.unwrap_or(usize::MAX);
            return preferred.into_iter().take(max).collect();
        }
        if let Some(selected) = ctx.default_alternative_selection() {
            return selected;
        }
        if ctx.relation_filter.is_some() {
            return ctx.legal_relation_selection(g, ctx.min).unwrap_or_default();
        }
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .take(ctx.min)
            .collect()
    }
}

fn run_turn(g: &mut GameState, dm: &mut Script) {
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::execute_turn_with(g, &mut combat, &mut queue, dm)
        .unwrap_or_else(|e| panic!("turn must run: {e:?}"));
}

/// Advances to and runs `n` further turns (the current turn must already be complete).
fn run_turns(g: &mut GameState, dm: &mut Script, n: usize) {
    for _ in 0..n {
        g.next_turn();
        run_turn(g, dm);
    }
}

fn observe_into<T: 'static>(
    cell: &Rc<RefCell<Vec<T>>>,
    f: impl Fn(&GameState) -> Option<T> + 'static,
) -> Observer {
    let cell = cell.clone();
    Box::new(move |g| {
        if let Some(v) = f(g) {
            cell.borrow_mut().push(v);
        }
    })
}

// ---------------------------------------------------------------------------
// 702.45 Bushido
// ---------------------------------------------------------------------------

#[test]
fn cr_702_45a_b_bushido_blocks_and_blocked_each_instance_triggers() {
    // Blocker side: two instances of bushido (1 and 2) on a blocking creature.
    let mut g = new_game();
    let samurai = put(
        &mut g,
        &card(
            "Twin Bushido Samurai",
            "Mana cost: {1}{W}\nType: Creature — Human Samurai\nPower/Toughness: 1/1\nBushido 1\nBushido 2",
        ),
        BOB,
        Zone::Battlefield,
    );
    let attacker = put(&mut g, &vanilla("Plain Attacker", 5, 5), ALICE, Zone::Battlefield);
    let seen = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        attack: vec![attacker],
        blocks: vec![(samurai, attacker)],
        observe: Some(observe_into(&seen, move |g| {
            (g.turn.step == Some(Step::DeclareBlockers) && g.stack.is_empty())
                .then(|| g.current_power(samurai).unwrap_or(-99))
        })),
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    let max_power = seen.borrow().iter().copied().max();
    assert_eq!(
        max_power,
        Some(4),
        "CR 702.45a/b: two bushido instances (1 and 2) on a blocker each trigger, making it 4/4; observed {:?}",
        seen.borrow()
    );

    // Attacker side: "becomes blocked".
    let mut g = new_game();
    let samurai = put(
        &mut g,
        &card(
            "Devoted Retainer",
            "Mana cost: {W}\nType: Creature — Human Samurai\nPower/Toughness: 1/1\nBushido 1 (Whenever this creature blocks or becomes blocked, it gets +1/+1 until end of turn.)",
        ),
        ALICE,
        Zone::Battlefield,
    );
    let blocker = put(&mut g, &vanilla("Wall Blocker", 0, 5), BOB, Zone::Battlefield);
    let seen = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        attack: vec![samurai],
        blocks: vec![(blocker, samurai)],
        observe: Some(observe_into(&seen, move |g| {
            (g.turn.step == Some(Step::DeclareBlockers) && g.stack.is_empty())
                .then(|| g.current_power(samurai).unwrap_or(-99))
        })),
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        seen.borrow().iter().copied().max(),
        Some(2),
        "CR 702.45a: bushido triggers when its creature becomes blocked; observed {:?}",
        seen.borrow()
    );
    assert_eq!(
        g.current_power(samurai),
        Some(1),
        "CR 702.45a: the bushido bonus lasts only until end of turn"
    );
}

// ---------------------------------------------------------------------------
// 702.108 Prowess
// ---------------------------------------------------------------------------

#[test]
fn cr_702_108a_b_prowess_triggers_on_noncreature_spells_per_instance() {
    let mut g = new_game();
    let def = card(
        "Double Prowess Monk",
        "Mana cost: {R}\nType: Creature — Human Monk\nPower/Toughness: 1/2\nProwess\nProwess",
    );
    let monk = put(&mut g, &def, ALICE, Zone::Battlefield);
    lands(&mut g, "Mountain", 3, ALICE);
    put(
        &mut g,
        &card("Shock Spell", "Mana cost: {R}\nType: Instant\nShock Spell deals 2 damage to any target."),
        ALICE,
        Zone::Hand,
    );
    put(&mut g, &vanilla("Creature Spell", 1, 1), ALICE, Zone::Hand);
    let seen = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![cast_named("Creature Spell"), cast_named("Shock Spell")],
        prefer_targets: vec![Target::Player(BOB)],
        observe: Some(observe_into(&seen, move |g| {
            (g.stack.is_empty()).then(|| g.current_power(monk).unwrap_or(-99))
        })),
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(dm.plays.len(), 0, "fixture: both spells cast; took {:?}", dm.actions_taken);
    assert_eq!(life(&g, BOB), 18, "fixture: shock resolved");
    assert_eq!(
        seen.borrow().iter().copied().max(),
        Some(3),
        "CR 702.108a/b: one noncreature spell with two prowess instances gives +2/+2, a creature spell gives nothing; observed {:?}",
        seen.borrow()
    );
    assert_eq!(g.current_power(monk), Some(1), "CR 702.108a: until end of turn");
}

// ---------------------------------------------------------------------------
// 702.46 Soulshift
// ---------------------------------------------------------------------------

#[test]
fn cr_702_46a_b_soulshift_instances_each_return_a_spirit_within_their_value() {
    let mut g = new_game();
    let shifter = put(
        &mut g,
        &card(
            "Double Soulshift Spirit",
            "Mana cost: {3}{B}{B}\nType: Creature — Spirit\nPower/Toughness: 3/3\nSoulshift 3\nSoulshift 1",
        ),
        ALICE,
        Zone::Battlefield,
    );
    let spirit = |name: &str, cost: &str| {
        card(
            name,
            &format!("Mana cost: {cost}\nType: Creature — Spirit\nPower/Toughness: 1/1"),
        )
    };
    let s3 = put(&mut g, &spirit("Spirit Three", "{2}{B}"), ALICE, Zone::Graveyard);
    let s1 = put(&mut g, &spirit("Spirit One", "{B}"), ALICE, Zone::Graveyard);
    put(&mut g, &spirit("Spirit Four", "{3}{B}"), ALICE, Zone::Graveyard);
    put(&mut g, &vanilla("Nonspirit One", 1, 1), ALICE, Zone::Graveyard);
    lands(&mut g, "Swamp", 3, ALICE);
    put(
        &mut g,
        &card("Murder", "Mana cost: {1}{B}{B}\nType: Instant\nDestroy target creature."),
        ALICE,
        Zone::Hand,
    );
    let mut dm = Script {
        plays: vec![cast_named("Murder")],
        prefer_targets: vec![Target::Object(shifter), Target::Object(s3), Target::Object(s1)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(zone_of(&g, "Double Soulshift Spirit"), Some(Zone::Graveyard), "fixture: murder resolved");
    assert_eq!(
        (zone_of(&g, "Spirit Three"), zone_of(&g, "Spirit One"), zone_of(&g, "Spirit Four"), zone_of(&g, "Nonspirit One")),
        (Some(Zone::Hand), Some(Zone::Hand), Some(Zone::Graveyard), Some(Zone::Graveyard)),
        "CR 702.46a/b: soulshift 3 and soulshift 1 each trigger and return one Spirit card with mana value <= N"
    );
}

// ---------------------------------------------------------------------------
// 702.105 Dethrone
// ---------------------------------------------------------------------------

#[test]
fn cr_702_105a_b_dethrone_triggers_only_against_most_life_per_instance() {
    for (bob_life, expected) in [(25, 2u32), (20, 2), (15, 0)] {
        let mut g = new_game();
        g.player_mut(BOB).unwrap().life = bob_life;
        let def = card(
            "Double Dethroner",
            "Mana cost: {1}{R}\nType: Creature — Human Rogue\nPower/Toughness: 1/1\nDethrone\nDethrone",
        );
        let c = put(&mut g, &def, ALICE, Zone::Battlefield);
        let mut dm = Script {
            attack: vec![c],
            ..Default::default()
        };
        run_turn(&mut g, &mut dm);
        assert_eq!(
            g.counter_count(c, CounterType::PlusOnePlusOne),
            expected,
            "CR 702.105a/b: Bob at {bob_life} vs Alice 20: each dethrone instance triggers only when attacking a player with the most (or tied) life"
        );
    }
}

// ---------------------------------------------------------------------------
// 702.110 Exploit
// ---------------------------------------------------------------------------

#[test]
fn cr_702_110a_b_exploit_optional_sacrifice_and_exploits_trigger() {
    for accept in [true, false] {
        let mut g = new_game();
        lands(&mut g, "Swamp", 5, ALICE);
        put(
            &mut g,
            &card(
                "Silumgar Butcher",
                "Mana cost: {4}{B}\nType: Creature — Zombie Djinn\nPower/Toughness: 3/3\nExploit (When this creature enters, you may sacrifice a creature.)\nWhen this creature exploits a creature, target creature gets -3/-3 until end of turn.",
            ),
            ALICE,
            Zone::Hand,
        );
        let fodder = put(&mut g, &vanilla("Exploit Fodder", 1, 1), ALICE, Zone::Battlefield);
        let victim = put(&mut g, &vanilla("Exploit Victim", 3, 3), BOB, Zone::Battlefield);
        let mut dm = Script {
            plays: vec![cast_named("Silumgar Butcher")],
            boolean: accept,
            prefer_objects: if accept { vec![fodder] } else { vec![] },
            prefer_targets: vec![Target::Object(victim)],
            ..Default::default()
        };
        run_turn(&mut g, &mut dm);
        assert_eq!(zone_of(&g, "Silumgar Butcher"), Some(Zone::Battlefield), "fixture: butcher resolved");
        let expected = if accept { Some(Zone::Graveyard) } else { Some(Zone::Battlefield) };
        assert_eq!(
            (zone_of(&g, "Exploit Fodder"), zone_of(&g, "Exploit Victim")),
            (expected, expected),
            "CR 702.110a/b: accept={accept}: exploit may sacrifice a creature, and only an actual sacrifice makes the creature 'exploit' (victim dies)"
        );
    }
}

// ---------------------------------------------------------------------------
// 702.101 Extort
// ---------------------------------------------------------------------------

#[test]
fn cr_702_101a_b_extort_each_instance_drains_on_any_spell_cast() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Double Extorter",
            "Mana cost: {1}{W}\nType: Creature — Human Cleric\nPower/Toughness: 2/2\nExtort\nExtort",
        ),
        ALICE,
        Zone::Battlefield,
    );
    lands(&mut g, "Plains", 3, ALICE);
    put(&mut g, &vanilla("Extort Trigger Spell", 1, 1), ALICE, Zone::Hand);
    let mut dm = Script {
        plays: vec![cast_named("Extort Trigger Spell")],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(zone_of(&g, "Extort Trigger Spell"), Some(Zone::Battlefield), "fixture: spell resolved");
    assert_eq!(
        (life(&g, ALICE), life(&g, BOB)),
        (22, 18),
        "CR 702.101a/b: two extort instances each trigger on a creature spell; each paid {{W/B}} drains 1"
    );
}

// ---------------------------------------------------------------------------
// 702.100 Evolve
// ---------------------------------------------------------------------------

#[test]
fn cr_702_100a_b_d_evolve_compares_and_each_instance_triggers() {
    let mut g = new_game();
    let raptor = put(
        &mut g,
        &card(
            "Cloudfin Raptor",
            "Mana cost: {U}\nType: Creature — Bird Mutant\nPower/Toughness: 0/1\nFlying\nEvolve (Whenever a creature you control enters, if that creature has greater power or toughness than this creature, put a +1/+1 counter on this creature.)",
        ),
        ALICE,
        Zone::Battlefield,
    );
    let twin = put(
        &mut g,
        &card(
            "Double Evolver",
            "Mana cost: {U}\nType: Creature — Mutant\nPower/Toughness: 0/1\nEvolve\nEvolve",
        ),
        ALICE,
        Zone::Battlefield,
    );
    lands(&mut g, "Island", 4, ALICE);
    put(&mut g, &vanilla("Evolve Bear", 2, 2), ALICE, Zone::Hand);
    put(&mut g, &vanilla("Evolve Small", 1, 1), ALICE, Zone::Hand);
    put(
        &mut g,
        &card("Evolve Rock", "Mana cost: {1}\nType: Artifact\n{T}: You gain 1 life."),
        ALICE,
        Zone::Hand,
    );
    let mut dm = Script {
        plays: vec![cast_named("Evolve Bear"), cast_named("Evolve Small"), cast_named("Evolve Rock")],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(dm.plays.len(), 0, "fixture: all spells cast {:?}", dm.actions_taken);
    // Raptor 0/1: Bear 2/2 -> evolve (1/2); Small 1/1 -> no; Rock (noncreature) -> no.
    assert_eq!(
        g.counter_count(raptor, CounterType::PlusOnePlusOne),
        1,
        "CR 702.100a/c: evolve compares P/T on entry and resolution; a 1/1 vs 1/2 and a noncreature never evolve"
    );
    // Twin 0/1 with two instances: Bear 2/2 -> both trigger, second still greater on resolution (1/2 vs 2/2) -> 2/3.
    assert_eq!(
        g.counter_count(twin, CounterType::PlusOnePlusOne),
        2,
        "CR 702.100d: each evolve instance triggers separately and rechecks on resolution"
    );
}

// ---------------------------------------------------------------------------
// 702.107 Outlast
// ---------------------------------------------------------------------------

#[test]
fn cr_702_107a_outlast_taps_adds_counter_and_is_sorcery_speed() {
    let mut g = new_game();
    let def = card(
        "Ainok Bond-Kin",
        "Mana cost: {1}{W}\nType: Creature — Dog Soldier\nPower/Toughness: 2/1\nOutlast {1}{W} ({1}{W}, {T}: Put a +1/+1 counter on this creature. Outlast only as a sorcery.)\nEach creature you control with a +1/+1 counter on it has first strike.",
    );
    let kin = put(&mut g, &def, ALICE, Zone::Battlefield);
    lands(&mut g, "Plains", 2, ALICE);
    let combat_offers = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![activate_named("Ainok Bond-Kin")],
        observe: Some(observe_into(&combat_offers, move |g| {
            (g.turn.step == Some(Step::BeginCombat) && g.turn.priority_player == Some(ALICE)).then(|| {
                ironsmith::compute_legal_actions(g, ALICE)
                    .map(|v| {
                        v.iter().any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == kin))
                    })
                    .unwrap_or(false)
            })
        })),
        ..Default::default()
    };
    // Delay the activation until main phase 2 so we can observe combat first.
    let first = dm.plays.remove(0);
    let mut first = Some(first);
    dm.plays.push(Box::new(move |g, a| {
        g.turn.phase == ironsmith::Phase::NextMain && first.as_mut().map_or(false, |f| f(g, a))
    }));
    run_turn(&mut g, &mut dm);
    assert!(
        combat_offers.borrow().iter().all(|offered| !offered),
        "CR 702.107a: outlast is not offered outside a main phase; observed {:?}",
        combat_offers.borrow()
    );
    assert_eq!(
        (g.counter_count(kin, CounterType::PlusOnePlusOne), g.is_tapped(kin)),
        (1, true),
        "CR 702.107a: outlast costs {{T}} and puts a +1/+1 counter on the creature (actions {:?})",
        dm.actions_taken
    );
}

// ---------------------------------------------------------------------------
// 702.64 Absorb
// ---------------------------------------------------------------------------

#[test]
fn cr_702_64a_b_c_absorb_prevents_per_instance_per_source_per_event() {
    // a/c: absorb 2 + absorb 1 versus one 4-damage spell -> 1 damage.
    // b: absorb 2 versus two separate 2-damage spells -> 0 damage.
    for (abilities, spells, expected) in [
        ("Absorb 2\nAbsorb 1", vec!["Four Damage"], 1u32),
        ("Absorb 2", vec!["Two Damage A", "Two Damage B"], 0u32),
        ("Absorb 2", vec!["Four Damage"], 2u32),
    ] {
        let mut g = new_game();
        let target = put(
            &mut g,
            &card(
                "Absorber",
                &format!("Mana cost: {{2}}{{W}}\nType: Creature — Spirit\nPower/Toughness: 1/9\n{abilities}"),
            ),
            BOB,
            Zone::Battlefield,
        );
        lands(&mut g, "Mountain", 2, ALICE);
        let mut plays: Vec<Play> = vec![];
        for name in spells.iter() {
            let n = if name.starts_with("Four") { 4 } else { 2 };
            put(
                &mut g,
                &card(name, &format!("Mana cost: {{R}}\nType: Instant\nThis spell deals {n} damage to target creature.")),
                ALICE,
                Zone::Hand,
            );
            let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
            plays.push(cast_named(leaked));
        }
        let mut dm = Script {
            plays,
            prefer_targets: vec![Target::Object(target)],
            observe: None,
            ..Default::default()
        };
        let seen = Rc::new(RefCell::new(vec![]));
        dm.observe = Some(observe_into(&seen, move |g| Some(g.damage_on(target))));
        run_turn(&mut g, &mut dm);
        assert_eq!(dm.plays.len(), 0, "fixture: spells cast {:?}", dm.actions_taken);
        assert_eq!(
            seen.borrow().iter().copied().max(),
            Some(expected),
            "CR 702.64a/b/c: [{abilities}] vs {spells:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 702.63 Vanishing
// ---------------------------------------------------------------------------

#[test]
fn cr_702_63a_vanishing_enters_with_counters_ticks_and_sacrifices() {
    let mut g = new_game();
    lands(&mut g, "Plains", 1, ALICE);
    put(
        &mut g,
        &card(
            "Vanishing Two",
            "Mana cost: {W}\nType: Creature — Spirit\nPower/Toughness: 2/2\nVanishing 2 (This creature enters with two time counters on it. At the beginning of your upkeep, remove a time counter from it. When the last is removed, sacrifice it.)",
        ),
        ALICE,
        Zone::Hand,
    );
    let mut dm = Script {
        plays: vec![cast_named("Vanishing Two")],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    let id = id_of(&g, "Vanishing Two");
    assert_eq!(g.counter_count(id, CounterType::Time), 2, "CR 702.63a: enters with N time counters");
    run_turns(&mut g, &mut dm, 2); // Bob's turn, then Alice's upkeep
    let id = id_of(&g, "Vanishing Two");
    assert_eq!(
        (g.counter_count(id, CounterType::Time), zone_of(&g, "Vanishing Two")),
        (1, Some(Zone::Battlefield)),
        "CR 702.63a: one counter removed at its controller's upkeep (not the opponent's)"
    );
    run_turns(&mut g, &mut dm, 2);
    assert_eq!(
        zone_of(&g, "Vanishing Two"),
        Some(Zone::Graveyard),
        "CR 702.63a: removing the last time counter sacrifices it"
    );
}

#[test]
fn cr_702_63b_c_numberless_vanishing_and_multiple_instances() {
    // b: "Vanishing" without a number only ticks existing counters.
    let mut g = new_game();
    let def = card(
        "Numberless Vanisher",
        "Mana cost: {W}\nType: Creature — Spirit\nPower/Toughness: 2/2\nVanishing",
    );
    let id = put(&mut g, &def, ALICE, Zone::Battlefield);
    g.add_counters(id, CounterType::Time, 1);
    let mut dm = Script::default();
    run_turn(&mut g, &mut dm);
    assert_eq!(
        zone_of(&g, "Numberless Vanisher"),
        Some(Zone::Graveyard),
        "CR 702.63b: numberless vanishing removes a counter at upkeep and sacrifices when the last is removed"
    );
    // c: two instances of vanishing 1 -> enters with 2 counters, each instance ticks once per upkeep.
    let mut g = new_game();
    lands(&mut g, "Plains", 1, ALICE);
    put(
        &mut g,
        &card(
            "Twin Vanisher",
            "Mana cost: {W}\nType: Creature — Spirit\nPower/Toughness: 2/2\nVanishing 1\nVanishing 1",
        ),
        ALICE,
        Zone::Hand,
    );
    let mut dm = Script {
        plays: vec![cast_named("Twin Vanisher")],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    let id = id_of(&g, "Twin Vanisher");
    assert_eq!(
        g.counter_count(id, CounterType::Time),
        2,
        "CR 702.63c: each vanishing instance's entry counters apply separately"
    );
    run_turns(&mut g, &mut dm, 2);
    assert_eq!(
        zone_of(&g, "Twin Vanisher"),
        Some(Zone::Graveyard),
        "CR 702.63c: both upkeep triggers remove a counter, and the last removal sacrifices it"
    );
}

// ---------------------------------------------------------------------------
// 702.58 Graft
// ---------------------------------------------------------------------------

#[test]
fn cr_702_58a_b_graft_enters_with_counters_and_moves_to_entering_creature() {
    let mut g = new_game();
    lands(&mut g, "Forest", 3, ALICE);
    put(
        &mut g,
        &card(
            "Twin Grafter",
            "Mana cost: {G}\nType: Creature — Human Mutant\nPower/Toughness: 0/0\nGraft 2\nGraft 1",
        ),
        ALICE,
        Zone::Hand,
    );
    put(&mut g, &vanilla("Graft Recipient", 1, 1), ALICE, Zone::Hand);
    let mut dm = Script {
        plays: vec![cast_named("Twin Grafter")],
        ..Default::default()
    };
    let entry_counters = Rc::new(RefCell::new(vec![]));
    dm.observe = Some(observe_into(&entry_counters, |g| {
        named(g, "Twin Grafter")
            .into_iter()
            .find(|(_, z)| *z == Zone::Battlefield)
            .map(|(id, _)| g.counter_count(id, CounterType::PlusOnePlusOne))
    }));
    dm.plays.push(cast_named("Graft Recipient"));
    run_turn(&mut g, &mut dm);
    assert_eq!(dm.plays.len(), 0, "fixture: both cast {:?}", dm.actions_taken);
    assert_eq!(
        entry_counters.borrow().first().copied(),
        Some(3),
        "CR 702.58a/b: graft 2 and graft 1 each make it enter with their counters"
    );
    let recipient = id_of(&g, "Graft Recipient");
    let grafter = id_of(&g, "Twin Grafter");
    assert_eq!(
        (g.counter_count(recipient, CounterType::PlusOnePlusOne), g.counter_count(grafter, CounterType::PlusOnePlusOne)),
        (2, 1),
        "CR 702.58a/b: each graft trigger may move one +1/+1 counter onto the entering creature"
    );
}

// ---------------------------------------------------------------------------
// 702.54 Bloodthirst
// ---------------------------------------------------------------------------

#[test]
fn cr_702_54a_c_bloodthirst_requires_opponent_damage_and_instances_stack() {
    for (damage_first, abilities, expected) in [
        (true, "Bloodthirst 2", 2u32),
        (false, "Bloodthirst 2", 0),
        (true, "Bloodthirst 1\nBloodthirst 2", 3),
    ] {
        let mut g = new_game();
        lands(&mut g, "Mountain", 3, ALICE);
        put(
            &mut g,
            &card(
                "Bloodthirsty Minotaur",
                &format!("Mana cost: {{1}}{{R}}\nType: Creature — Minotaur\nPower/Toughness: 3/3\n{abilities}"),
            ),
            ALICE,
            Zone::Hand,
        );
        put(
            &mut g,
            &card("Lightning Bolt", "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target."),
            ALICE,
            Zone::Hand,
        );
        let mut plays = vec![];
        if damage_first {
            plays.push(cast_named("Lightning Bolt"));
        }
        plays.push(cast_named("Bloodthirsty Minotaur"));
        let mut dm = Script {
            plays,
            prefer_targets: vec![Target::Player(BOB)],
            ..Default::default()
        };
        run_turn(&mut g, &mut dm);
        let id = id_of(&g, "Bloodthirsty Minotaur");
        assert_eq!(g.object(id).unwrap().zone, Zone::Battlefield, "fixture: resolved");
        assert_eq!(
            g.counter_count(id, CounterType::PlusOnePlusOne),
            expected,
            "CR 702.54a/c: [{abilities}] damage_first={damage_first}"
        );
    }
}

#[test]
fn cr_702_54b_bloodthirst_x_counts_total_opponent_damage() {
    let mut g = new_game();
    lands(&mut g, "Mountain", 3, ALICE);
    put(
        &mut g,
        &card(
            "Bloodthirst X Beast",
            "Mana cost: {1}{R}\nType: Creature — Beast\nPower/Toughness: 1/1\nBloodthirst X (This creature enters with X +1/+1 counters on it, where X is the damage dealt to your opponents this turn.)",
        ),
        ALICE,
        Zone::Hand,
    );
    put(
        &mut g,
        &card("Lightning Bolt", "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target."),
        ALICE,
        Zone::Hand,
    );
    let mut dm = Script {
        plays: vec![cast_named("Lightning Bolt"), cast_named("Bloodthirst X Beast")],
        prefer_targets: vec![Target::Player(BOB)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    let id = id_of(&g, "Bloodthirst X Beast");
    assert_eq!(
        g.counter_count(id, CounterType::PlusOnePlusOne),
        3,
        "CR 702.54b: bloodthirst X enters with counters equal to total damage opponents were dealt this turn"
    );
}

// ---------------------------------------------------------------------------
// 702.98 Unleash
// ---------------------------------------------------------------------------

#[test]
fn cr_702_98a_unleash_optional_counter_and_cannot_block_with_counter() {
    for accept in [true, false] {
        let mut g = new_game();
        lands(&mut g, "Swamp", 3, ALICE);
        put(
            &mut g,
            &card(
                "Rakdos Drake",
                "Mana cost: {2}{B}\nType: Creature — Drake\nPower/Toughness: 1/2\nFlying\nUnleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)",
            ),
            ALICE,
            Zone::Hand,
        );
        let mut dm = Script {
            plays: vec![cast_named("Rakdos Drake")],
            boolean: accept,
            ..Default::default()
        };
        run_turn(&mut g, &mut dm);
        let drake = id_of(&g, "Rakdos Drake");
        assert_eq!(
            g.counter_count(drake, CounterType::PlusOnePlusOne),
            u32::from(accept),
            "CR 702.98a: unleash lets the controller choose whether it enters with a +1/+1 counter"
        );
        g.next_turn();
        let attacker = put(&mut g, &vanilla("Unleash Attacker", 1, 1), BOB, Zone::Battlefield);
        let mut dm = Script {
            attack: vec![attacker],
            blocks: vec![(drake, attacker)],
            ..Default::default()
        };
        run_turn(&mut g, &mut dm);
        assert_eq!(
            life(&g, ALICE),
            if accept { 19 } else { 20 },
            "CR 702.98a: accept={accept}: a permanent with unleash can't block while it has a +1/+1 counter"
        );
    }
}

// ---------------------------------------------------------------------------
// 702.104 Tribute
// ---------------------------------------------------------------------------

#[test]
fn cr_702_104a_b_tribute_opponent_chooses_and_unpaid_trigger() {
    for pay in [true, false] {
        let mut g = new_game();
        lands(&mut g, "Forest", 3, ALICE);
        put(
            &mut g,
            &card(
                "Fanatic of Xenagos",
                "Mana cost: {1}{R}{G}\nType: Creature — Centaur Warrior\nPower/Toughness: 3/3\nTrample\nTribute 1 (As this creature enters, an opponent of your choice may put a +1/+1 counter on it.)\nWhen this creature enters, if tribute wasn't paid, it gets +1/+1 and gains haste until end of turn.",
            ),
            ALICE,
            Zone::Hand,
        );
        let seen = Rc::new(RefCell::new(vec![]));
        let mut dm = Script {
            plays: vec![cast_named("Fanatic of Xenagos")],
            boolean: pay,
            ..Default::default()
        };
        dm.observe = Some(observe_into(&seen, |g| {
            named(g, "Fanatic of Xenagos")
                .into_iter()
                .find(|(_, z)| *z == Zone::Battlefield)
                .filter(|_| g.stack.is_empty())
                .map(|(id, _)| (g.counter_count(id, CounterType::PlusOnePlusOne), g.current_power(id).unwrap_or(0)))
        }));
        // Mana: we only have Forests; give red via a Mountain.
        lands(&mut g, "Mountain", 1, ALICE);
        run_turn(&mut g, &mut dm);
        assert_eq!(zone_of(&g, "Fanatic of Xenagos"), Some(Zone::Battlefield), "fixture: resolved {:?}", dm.actions_taken);
        let first = seen.borrow().first().map(|(c, _)| *c);
        let max_power = seen.borrow().iter().map(|(_, p)| *p).max();
        let expected = if pay { (1, 4) } else { (0, 4) };
        assert_eq!(
            first.zip(max_power),
            Some(expected),
            "CR 702.104a/b: pay={pay}: tribute counters come from the opponent's choice; the 'tribute wasn't paid' trigger fires only if unpaid"
        );
        assert!(
            dm.booleans_seen.iter().any(|b| b.starts_with(&format!("{BOB:?}"))),
            "CR 702.104a: the chosen opponent (Bob) makes the tribute decision; prompts {:?}",
            dm.booleans_seen
        );
    }
}

// ---------------------------------------------------------------------------
// 702.48 Offering
// ---------------------------------------------------------------------------

fn patron_of_the_akki() -> CardDefinition {
    card(
        "Patron of the Akki",
        "Mana cost: {4}{R}{R}\nType: Legendary Creature — Spirit\nPower/Toughness: 5/5\nGoblin offering (You may cast this spell any time you could cast an instant by sacrificing a Goblin and paying the difference in mana costs between this and the sacrificed Goblin. Mana cost includes color.)\nWhenever Patron of the Akki attacks, creatures you control get +2/+0 until end of turn.",
    )
}

#[test]
fn cr_702_48a_b_offering_sacrifice_reduces_cost_and_grants_instant_timing() {
    let mut g = new_game();
    put(&mut g, &patron_of_the_akki(), ALICE, Zone::Hand);
    let goblin = put(
        &mut g,
        &card("Goblin Offering Fodder", "Mana cost: {1}{R}\nType: Creature — Goblin\nPower/Toughness: 2/1"),
        ALICE,
        Zone::Battlefield,
    );
    lands(&mut g, "Mountain", 4, ALICE);
    let mut dm = Script {
        // Cast during the beginning of combat (not sorcery timing).
        plays: vec![Box::new(|g, a| {
            g.turn.step == Some(Step::BeginCombat)
                && matches!(a, LegalAction::CastSpell { spell_id, .. } if g.object(*spell_id).is_some_and(|o| &*o.name == "Patron of the Akki"))
        })],
        prefer_objects: vec![goblin],
        boolean: true,
        option_pref: vec!["Offering"],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        (zone_of(&g, "Patron of the Akki"), zone_of(&g, "Goblin Offering Fodder")),
        (Some(Zone::Battlefield), Some(Zone::Graveyard)),
        "CR 702.48a/b: with a Goblin sacrificed while casting, Patron ({{4}}{{R}}{{R}}) is castable at instant speed for {{3}}{{R}} with four Mountains; actions {:?}",
        dm.actions_taken
    );
}

#[test]
fn cr_702_48c_offering_excess_colored_reduction_reduces_generic() {
    // Sacrificed Goblin costs {R}{R}{R}: {R}{R} reduce Patron's {R}{R}; the excess {R} reduces one generic,
    // leaving {3} to pay with exactly three lands.
    let mut g = new_game();
    put(&mut g, &patron_of_the_akki(), ALICE, Zone::Hand);
    let goblin = put(
        &mut g,
        &card("Triple Red Goblin", "Mana cost: {R}{R}{R}\nType: Creature — Goblin\nPower/Toughness: 2/1"),
        ALICE,
        Zone::Battlefield,
    );
    lands(&mut g, "Forest", 3, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Patron of the Akki")],
        prefer_objects: vec![goblin],
        option_pref: vec!["Offering"],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        (zone_of(&g, "Patron of the Akki"), zone_of(&g, "Triple Red Goblin")),
        (Some(Zone::Battlefield), Some(Zone::Graveyard)),
        "CR 702.48c: excess colored mana in the sacrificed permanent's cost reduces generic mana ({{4}}{{R}}{{R}} - {{R}}{{R}}{{R}} = {{3}}); actions {:?}",
        dm.actions_taken
    );
}

// ---------------------------------------------------------------------------
// 702.49 Ninjutsu
// ---------------------------------------------------------------------------

fn ninja_of_the_deep_hours() -> CardDefinition {
    card(
        "Ninja of the Deep Hours",
        "Mana cost: {3}{U}\nType: Creature — Human Ninja\nPower/Toughness: 2/2\nNinjutsu {1}{U} ({1}{U}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nWhenever this creature deals combat damage to a player, you may draw a card.",
    )
}

#[test]
fn cr_702_49a_c_ninjutsu_swaps_unblocked_attacker_attacking_same_planeswalker() {
    let mut g = new_game();
    put(&mut g, &ninja_of_the_deep_hours(), ALICE, Zone::Hand);
    lands(&mut g, "Island", 2, ALICE);
    let attacker = put(&mut g, &vanilla("Ninjutsu Sneak", 1, 1), ALICE, Zone::Battlefield);
    let pw = put(
        &mut g,
        &card(
            "Target Walker",
            "Type: Legendary Planeswalker — Teyo\nLoyalty: 5\n+1: You gain 1 life.",
        ),
        BOB,
        Zone::Battlefield,
    );
    g.add_counters(pw, CounterType::Loyalty, 5 - g.counter_count(pw, CounterType::Loyalty).min(5));
    let seen = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        attack: vec![attacker],
        attack_target: Some(AttackTarget::Planeswalker(pw)),
        plays: vec![Box::new(|g, a| {
            g.turn.step == Some(Step::DeclareBlockers)
                && matches!(a, LegalAction::ActivateAbility { source, .. } if g.object(*source).is_some_and(|o| &*o.name == "Ninja of the Deep Hours"))
        })],
        observe: None,
        ..Default::default()
    };
    dm.observe = Some(observe_into(&seen, |g| {
        named(g, "Ninja of the Deep Hours")
            .into_iter()
            .find(|(_, z)| *z == Zone::Battlefield)
            .map(|(id, _)| g.is_tapped(id))
    }));
    run_turn(&mut g, &mut dm);
    assert_eq!(
        (zone_of(&g, "Ninjutsu Sneak"), zone_of(&g, "Ninja of the Deep Hours")),
        (Some(Zone::Hand), Some(Zone::Battlefield)),
        "CR 702.49a: ninjutsu returns the unblocked attacker and puts the ninja onto the battlefield; actions {:?}",
        dm.actions_taken
    );
    assert_eq!(seen.borrow().first().copied(), Some(true), "CR 702.49a: the ninja enters tapped");
    assert_eq!(
        (g.counter_count(pw, CounterType::Loyalty), life(&g, BOB)),
        (3, 20),
        "CR 702.49c: the ninja attacks the same planeswalker the returned creature was attacking (2 damage to the walker)"
    );
}

#[test]
fn cr_702_49b_ninjutsu_card_is_revealed_while_ability_on_stack() {
    // Reveal observer: "Whenever an opponent reveals a card" style hook via CardRevealedEvent.
    let mut g = new_game();
    let ninja = put(&mut g, &ninja_of_the_deep_hours(), ALICE, Zone::Hand);
    lands(&mut g, "Island", 2, ALICE);
    let attacker = put(&mut g, &vanilla("Ninjutsu Sneak", 1, 1), ALICE, Zone::Battlefield);
    // Bob's reveal observer: "Whenever an opponent reveals a card, you gain 1 life."
    let watcher = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Reveal Watcher")
        .card_types(vec![ironsmith::CardType::Artifact])
        .with_ability(ironsmith::Ability::triggered(
            ironsmith::triggers::Trigger::player_reveals_card(
                ironsmith::PlayerFilter::Opponent,
                ironsmith::ObjectFilter::default(),
                false,
            ),
            vec![ironsmith::Effect::gain_life(1)],
        ))
        .build();
    put(&mut g, &watcher, BOB, Zone::Battlefield);
    let seen = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        attack: vec![attacker],
        plays: vec![Box::new(|g, a| {
            g.turn.step == Some(Step::DeclareBlockers)
                && matches!(a, LegalAction::ActivateAbility { source, .. } if g.object(*source).is_some_and(|o| &*o.name == "Ninja of the Deep Hours"))
        })],
        ..Default::default()
    };
    dm.observe = Some(observe_into(&seen, move |g| {
        (!g.stack.is_empty() && g.object(ninja).is_some_and(|o| o.zone == Zone::Hand)).then(|| {
            g.is_publicly_revealed_hidden_card(ninja)
                || g.hand_revealed_by_effect(ALICE)
                || g.is_hand_card_revealed_until_upkeep_ends(ninja)
                || g.is_hand_card_revealed_until_owners_next_turn(ninja)
        })
    }));
    run_turn(&mut g, &mut dm);
    assert_eq!(zone_of(&g, "Ninja of the Deep Hours"), Some(Zone::Battlefield), "fixture: ninjutsu resolved");
    assert!(
        life(&g, BOB) == 19 && !seen.borrow().is_empty() && seen.borrow().iter().all(|r| *r),
        "CR 702.49a/b: the ninjutsu card is revealed from hand (Bob's opponent-reveal watcher would leave him at 19 after 2 combat damage; he is at {}) and stays revealed while its ability is on the stack (revealed-state observations {:?})",
        life(&g, BOB),
        seen.borrow()
    );
}

#[test]
fn cr_702_49d_commander_ninjutsu_works_from_command_zone() {
    let mut g = new_game();
    let yuriko = put(
        &mut g,
        &card(
            "Yuriko, the Tiger's Shadow",
            "Mana cost: {1}{U}{B}\nType: Legendary Creature — Human Ninja\nPower/Toughness: 1/3\nCommander ninjutsu {U}{B} ({U}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand or the command zone tapped and attacking.)\nWhenever a Ninja you control deals combat damage to a player, reveal the top card of your library and put that card into your hand. Each opponent loses life equal to that card's mana value.",
        ),
        ALICE,
        Zone::Command,
    );
    g.set_as_commander(yuriko, ALICE);
    lands(&mut g, "Island", 1, ALICE);
    lands(&mut g, "Swamp", 1, ALICE);
    let attacker = put(&mut g, &vanilla("Ninjutsu Sneak", 1, 1), ALICE, Zone::Battlefield);
    let mut dm = Script {
        attack: vec![attacker],
        plays: vec![Box::new(|g, a| {
            g.turn.step == Some(Step::DeclareBlockers)
                && matches!(a, LegalAction::ActivateAbility { source, .. } if g.object(*source).is_some_and(|o| o.name.starts_with("Yuriko")))
        })],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        (zone_of(&g, "Ninjutsu Sneak"), zone_of(&g, "Yuriko, the Tiger's Shadow")),
        (Some(Zone::Hand), Some(Zone::Battlefield)),
        "CR 702.49d: commander ninjutsu functions from the command zone; actions {:?}",
        dm.actions_taken
    );
}

// ---------------------------------------------------------------------------
// 702.50 Epic
// ---------------------------------------------------------------------------

#[test]
fn cr_702_50a_b_epic_forbids_casting_and_copies_each_upkeep() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Endless Swarm",
            "Mana cost: {5}{G}{G}{G}\nType: Sorcery\nCreate a 1/1 green Snake creature token for each card in your hand.\nEpic (For the rest of the game, you can't cast spells. At the beginning of each of your upkeeps, copy this spell except for its epic ability.)",
        ),
        ALICE,
        Zone::Hand,
    );
    put(&mut g, &vanilla("Held Spell", 1, 1), ALICE, Zone::Hand);
    put(&mut g, &vanilla("Held Spell Two", 1, 1), ALICE, Zone::Hand);
    lands(&mut g, "Forest", 9, ALICE);
    let castable_after = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![cast_named("Endless Swarm")],
        ..Default::default()
    };
    dm.observe = Some(observe_into(&castable_after, |g| {
        (g.turn.active_player == ALICE
            && g.turn.priority_player == Some(ALICE)
            && zone_of(g, "Endless Swarm") == Some(Zone::Graveyard)
            && g.stack.is_empty()
            && matches!(g.turn.phase, ironsmith::Phase::FirstMain | ironsmith::Phase::NextMain))
        .then(|| {
            ironsmith::compute_legal_actions(g, ALICE)
                .map(|v| v.iter().any(|a| matches!(a, LegalAction::CastSpell { .. })))
                .unwrap_or(false)
        })
    }));
    run_turn(&mut g, &mut dm);
    let snakes_now = named_like(&g, "Snake").len();
    assert_eq!(snakes_now, 2, "fixture: two cards left in hand -> two snakes");
    assert!(
        !castable_after.borrow().is_empty() && castable_after.borrow().iter().all(|c| !c),
        "CR 702.50a/b: after epic resolves its controller can't cast spells; observed {:?}",
        castable_after.borrow()
    );
    run_turns(&mut g, &mut dm, 2);
    // Alice drew one card on turn 3 -> 3 cards in hand at the upkeep copy (drawn after upkeep -> 2).
    assert!(
        named_like(&g, "Snake").len() >= 4,
        "CR 702.50a/b: at the beginning of each of the controller's upkeeps the spell is copied (effects can still put copies on the stack); snakes {}",
        named_like(&g, "Snake").len()
    );
}

// ---------------------------------------------------------------------------
// 702.51 Convoke
// ---------------------------------------------------------------------------

#[test]
fn cr_702_51a_c_convoke_taps_creatures_for_colored_and_generic() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Stoke the Flames",
            "Mana cost: {2}{R}{R}\nType: Instant\nConvoke (Your creatures can help cast this spell. Each creature you tap while casting this spell pays for {1} or one mana of that creature's color.)\nStoke the Flames deals 4 damage to any target.",
        ),
        ALICE,
        Zone::Hand,
    );
    let red = put(
        &mut g,
        &card("Red Convoker", "Mana cost: {R}\nType: Creature — Goblin\nPower/Toughness: 1/1"),
        ALICE,
        Zone::Battlefield,
    );
    let green = put(
        &mut g,
        &card("Green Convoker", "Mana cost: {G}\nType: Creature — Elf\nPower/Toughness: 1/1"),
        ALICE,
        Zone::Battlefield,
    );
    let green2 = put(
        &mut g,
        &card("Green Convoker Two", "Mana cost: {G}\nType: Creature — Elf\nPower/Toughness: 1/1"),
        ALICE,
        Zone::Battlefield,
    );
    lands(&mut g, "Mountain", 1, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Stoke the Flames")],
        prefer_targets: vec![Target::Player(BOB)],
        prefer_objects: vec![red, green, green2],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        life(&g, BOB),
        16,
        "CR 702.51a: {{2}}{{R}}{{R}} paid with one Mountain plus a red creature (R) and two green creatures (generic); actions {:?}",
        dm.actions_taken
    );
}

#[test]
fn cr_702_51c_creatures_that_convoked_are_identified() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Venerated Loxodon",
            "Mana cost: {4}{W}\nType: Creature — Elephant Cleric\nPower/Toughness: 4/4\nConvoke (Your creatures can help cast this spell. Each creature you tap while casting this spell pays for {1} or one mana of that creature's color.)\nWhen this creature enters, put a +1/+1 counter on each creature that convoked it.",
        ),
        ALICE,
        Zone::Hand,
    );
    let mut helpers = vec![];
    for i in 0..3 {
        let name: &'static str = ["Convoke Helper A", "Convoke Helper B", "Convoke Helper C"][i];
        helpers.push(put(
            &mut g,
            &card(name, "Mana cost: {W}\nType: Creature — Soldier\nPower/Toughness: 1/1"),
            ALICE,
            Zone::Battlefield,
        ));
    }
    let bystander = put(&mut g, &vanilla("Convoke Bystander", 1, 1), ALICE, Zone::Battlefield);
    lands(&mut g, "Plains", 2, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Venerated Loxodon")],
        prefer_objects: helpers.clone(),
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(zone_of(&g, "Venerated Loxodon"), Some(Zone::Battlefield), "fixture: cast {:?}", dm.actions_taken);
    let creatures: Vec<ObjectId> = helpers.iter().copied().chain(std::iter::once(bystander)).collect();
    let tapped: Vec<u32> = creatures.iter().map(|id| u32::from(g.is_tapped(*id))).collect();
    assert_eq!(tapped.iter().sum::<u32>(), 3, "fixture: three creatures were tapped to convoke {{3}} of {{4}}{{W}}");
    let counters: Vec<u32> = creatures
        .iter()
        .map(|id| g.counter_count(*id, CounterType::PlusOnePlusOne))
        .collect();
    assert_eq!(
        counters, tapped,
        "CR 702.51c: exactly the creatures tapped to pay the cost 'convoked' the spell (counters vs tapped)"
    );
}

#[test]
fn cr_702_51b_d_convoke_applies_after_cost_reduction_and_is_redundant() {
    // Reduction {2} makes Siege-like {4}{G}{G} into {2}{G}{G}; four green creatures pay it all.
    let mut g = new_game();
    put(
        &mut g,
        &card("Creature Discount", "Mana cost: {2}\nType: Artifact\nCreature spells you cast cost {2} less to cast."),
        ALICE,
        Zone::Battlefield,
    );
    put(
        &mut g,
        &card(
            "Double Convoke Wurm",
            "Mana cost: {4}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 5/5\nConvoke\nConvoke",
        ),
        ALICE,
        Zone::Hand,
    );
    let mut helpers = vec![];
    for name in ["Wurm Helper A", "Wurm Helper B", "Wurm Helper C", "Wurm Helper D"] {
        helpers.push(put(
            &mut g,
            &card(name, "Mana cost: {G}\nType: Creature — Elf\nPower/Toughness: 1/1"),
            ALICE,
            Zone::Battlefield,
        ));
    }
    let mut dm = Script {
        plays: vec![cast_named("Double Convoke Wurm")],
        prefer_objects: helpers.clone(),
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        zone_of(&g, "Double Convoke Wurm"),
        Some(Zone::Battlefield),
        "CR 702.51b: convoke pays the total cost after reduction (four creatures pay {{2}}{{G}}{{G}}); actions {:?}",
        dm.actions_taken
    );
    assert_eq!(
        helpers.iter().filter(|id| g.is_tapped(**id)).count(),
        4,
        "CR 702.51d: a second convoke instance doesn't change what each tapped creature pays"
    );
}

// ---------------------------------------------------------------------------
// 702.52 Dredge
// ---------------------------------------------------------------------------

#[test]
fn cr_702_52a_b_dredge_replaces_draw_only_with_enough_library() {
    for (library, expect_dredge) in [(5usize, true), (2usize, false)] {
        let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let filler = vanilla("Library Filler", 1, 1);
        for _ in 0..library {
            g.create_object_from_definition(&filler, BOB, Zone::Library);
        }
        for _ in 0..10 {
            g.create_object_from_definition(&filler, ALICE, Zone::Library);
        }
        put(
            &mut g,
            &card(
                "Darkblast",
                "Mana cost: {B}\nType: Instant\nTarget creature gets -1/-1 until end of turn.\nDredge 3 (If you would draw a card, you may mill three cards instead. If you do, return this card from your graveyard to your hand.)",
            ),
            BOB,
            Zone::Graveyard,
        );
        g.next_turn(); // Bob's turn, with a draw step.
        let mut dm = Script::default();
        run_turn(&mut g, &mut dm);
        let bob = g.player(BOB).unwrap();
        let (hand, gy, lib) = (bob.hand.len(), bob.graveyard.len(), bob.library.len());
        if expect_dredge {
            assert_eq!(
                (zone_of(&g, "Darkblast"), hand, gy, lib),
                (Some(Zone::Hand), 1, 3, 2),
                "CR 702.52a: dredge 3 replaces the draw: mill three and return the card"
            );
        } else {
            assert_eq!(
                (zone_of(&g, "Darkblast"), hand, lib),
                (Some(Zone::Graveyard), 1, 1),
                "CR 702.52b: with fewer than N cards in library, dredge N can't be used; the draw happens normally"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 702.53 Transmute
// ---------------------------------------------------------------------------

#[test]
fn cr_702_53a_transmute_tutors_same_mana_value_at_sorcery_speed() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Dimir House Guard",
            "Mana cost: {3}{B}\nType: Creature — Skeleton\nPower/Toughness: 2/3\nFear (This creature can't be blocked except by artifact creatures and/or black creatures.)\nSacrifice a creature: Regenerate this creature.\nTransmute {1}{B}{B} ({1}{B}{B}, Discard this card: Search your library for a card with the same mana value as this card, reveal it, put it into your hand, then shuffle. Transmute only as a sorcery.)",
        ),
        ALICE,
        Zone::Hand,
    );
    let four = put(&mut g, &card("Mana Value Four", "Mana cost: {3}{U}\nType: Sorcery\nYou gain 1 life."), ALICE, Zone::Library);
    let three = put(&mut g, &card("Mana Value Three", "Mana cost: {2}{U}\nType: Sorcery\nYou gain 1 life."), ALICE, Zone::Library);
    lands(&mut g, "Swamp", 3, ALICE);
    let combat_offer = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![Box::new(|g, a| {
            g.turn.phase == ironsmith::Phase::NextMain
                && matches!(a, LegalAction::ActivateAbility { source, .. } if g.object(*source).is_some_and(|o| &*o.name == "Dimir House Guard"))
        })],
        prefer_objects: vec![three, four],
        ..Default::default()
    };
    dm.observe = Some(observe_into(&combat_offer, |g| {
        (g.turn.step == Some(Step::BeginCombat) && g.turn.priority_player == Some(ALICE)).then(|| {
            ironsmith::compute_legal_actions(g, ALICE)
                .map(|v| v.iter().any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if g.object(*source).is_some_and(|o| &*o.name == "Dimir House Guard"))))
                .unwrap_or(false)
        })
    }));
    run_turn(&mut g, &mut dm);
    assert!(
        combat_offer.borrow().iter().all(|o| !o),
        "CR 702.53a: transmute is activated only as a sorcery; observed {:?}",
        combat_offer.borrow()
    );
    assert_eq!(
        (zone_of(&g, "Dimir House Guard"), zone_of(&g, "Mana Value Four"), zone_of(&g, "Mana Value Three")),
        (Some(Zone::Graveyard), Some(Zone::Hand), Some(Zone::Library)),
        "CR 702.53a: discard to search for a card with the same mana value only; actions {:?}",
        dm.actions_taken
    );
}

// ---------------------------------------------------------------------------
// 702.55 Haunt
// ---------------------------------------------------------------------------

#[test]
fn cr_702_55a_b_c_haunt_spell_exiles_haunting_and_triggers_from_exile() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Cry of Contrition",
            "Mana cost: {B}\nType: Sorcery\nTarget player discards a card.\nHaunt (When this spell card is put into a graveyard after resolving, exile it haunting target creature.)\nWhen the creature this card haunts dies, target player discards a card.",
        ),
        ALICE,
        Zone::Hand,
    );
    put(
        &mut g,
        &card("Murder", "Mana cost: {1}{B}{B}\nType: Instant\nDestroy target creature."),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Swamp", 4, ALICE);
    let haunted = put(&mut g, &vanilla("Haunted Creature", 2, 2), BOB, Zone::Battlefield);
    for name in ["Bob Card A", "Bob Card B", "Bob Card C"] {
        put(&mut g, &card(name, "Mana cost: {1}\nType: Artifact"), BOB, Zone::Hand);
    }
    let mut dm = Script {
        plays: vec![cast_named("Cry of Contrition"), cast_named_empty_stack("Murder")],
        prefer_targets: vec![Target::Player(BOB), Target::Object(haunted)],
        ..Default::default()
    };
    let exiled_mid = Rc::new(RefCell::new(vec![]));
    dm.observe = Some(observe_into(&exiled_mid, |g| {
        (zone_of(g, "Haunted Creature") == Some(Zone::Battlefield)).then(|| zone_of(g, "Cry of Contrition"))
    }));
    run_turn(&mut g, &mut dm);
    assert!(
        exiled_mid.borrow().contains(&Some(Zone::Exile)),
        "CR 702.55a: the resolved haunt spell is exiled haunting the target creature; observed {:?}",
        exiled_mid.borrow()
    );
    assert_eq!(zone_of(&g, "Haunted Creature"), Some(Zone::Graveyard), "fixture: murder resolved");
    assert_eq!(
        g.player(BOB).unwrap().hand.len(),
        1,
        "CR 702.55b/c: the haunting card's 'creature it haunts dies' trigger works from exile (Bob discards twice in total)"
    );
}

#[test]
fn cr_702_55a_haunt_permanent_exiles_itself_haunting_on_death() {
    let mut g = new_game();
    let pontiff = put(
        &mut g,
        &card(
            "Orzhov Pontiff",
            "Mana cost: {1}{W}{B}\nType: Creature — Human Cleric\nPower/Toughness: 1/1\nHaunt (When this creature dies, exile it haunting target creature.)\nWhen this creature enters or the creature it haunts dies, choose one —\n• Creatures you control get +1/+1 until end of turn.\n• Creatures you don't control get -1/-1 until end of turn.",
        ),
        ALICE,
        Zone::Battlefield,
    );
    let host = put(&mut g, &vanilla("Haunt Host", 3, 3), BOB, Zone::Battlefield);
    put(
        &mut g,
        &card("Murder", "Mana cost: {1}{B}{B}\nType: Instant\nDestroy target creature."),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Swamp", 3, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Murder")],
        prefer_targets: vec![Target::Object(pontiff), Target::Object(host)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        zone_of(&g, "Orzhov Pontiff"),
        Some(Zone::Exile),
        "CR 702.55a: a haunt permanent put into a graveyard from the battlefield is exiled haunting target creature"
    );
}

// ---------------------------------------------------------------------------
// 702.56 Replicate
// ---------------------------------------------------------------------------

#[test]
fn cr_702_56a_replicate_paid_twice_creates_two_copies() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Train of Thought",
            "Mana cost: {1}{U}\nType: Sorcery\nReplicate {1}{U} (When you cast this spell, copy it for each time you paid its replicate cost.)\nDraw a card.",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Island", 6, ALICE);
    let hand_before = g.player(ALICE).unwrap().hand.len();
    let mut dm = Script {
        plays: vec![cast_named("Train of Thought")],
        number: Some(2),
        option_pref: vec!["Replicate"],
        option_repeat: 2,
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    // Turn 1 has no draw for the starting player in a two-player game; count only spell draws.
    let library_drawn = 12 - g.player(ALICE).unwrap().library.len();
    assert_eq!(zone_of(&g, "Train of Thought"), Some(Zone::Graveyard), "fixture: resolved {:?}", dm.actions_taken);
    let _ = hand_before;
    assert!(
        library_drawn == 3 || library_drawn == 4,
        "CR 702.56a: paying replicate twice copies the spell twice (three draws from the spell); library drew {library_drawn}; numbers {:?} booleans {:?}",
        dm.numbers_seen,
        dm.booleans_seen
    );
}

#[test]
fn cr_702_56b_multiple_replicate_instances_paid_separately() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Twin Replicate Draw",
            "Mana cost: {U}\nType: Sorcery\nReplicate {1}\nReplicate {U}\nDraw a card.",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Island", 3, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Twin Replicate Draw")],
        number: Some(1),
        option_pref: vec!["Replicate"],
        ..Default::default()
    };
    let draws_before = g.player(ALICE).unwrap().library.len();
    run_turn(&mut g, &mut dm);
    let spell_draws = draws_before - g.player(ALICE).unwrap().library.len();
    assert_eq!(
        spell_draws,
        3,
        "CR 702.56b: each replicate instance is paid separately (once each) and each triggers for its own payment -> original + 2 copies; prompts {:?}",
        dm.prompts
    );
}

// ---------------------------------------------------------------------------
// 702.57 Forecast
// ---------------------------------------------------------------------------

#[test]
fn cr_702_57a_b_forecast_only_in_owners_upkeep_once_and_revealed() {
    let mut g = new_game();
    let pride = put(
        &mut g,
        &card(
            "Pride of the Clouds",
            "Mana cost: {W}{U}\nType: Creature — Elemental Cat\nPower/Toughness: 1/1\nFlying\nThis creature gets +1/+1 for each other creature you control with flying.\nForecast — {2}{W}{U}, Reveal this card from your hand: Create a 1/1 white and blue Bird creature token with flying. (Activate only during your upkeep and only once each turn.)",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Plains", 4, ALICE);
    lands(&mut g, "Island", 4, ALICE);
    let offers = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![activate_named("Pride of the Clouds"), activate_named("Pride of the Clouds")],
        ..Default::default()
    };
    dm.observe = Some(observe_into(&offers, move |g| {
        (g.turn.priority_player == Some(ALICE) && g.stack.is_empty()).then(|| {
            let offered = ironsmith::compute_legal_actions(g, ALICE)
                .map(|v| v.iter().any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == pride)))
                .unwrap_or(false);
            (g.turn.step, offered, g.is_hand_card_revealed_until_upkeep_ends(pride))
        })
    }));
    run_turn(&mut g, &mut dm);
    let birds = named_like(&g, "Bird").len();
    let log = offers.borrow().clone();
    assert_eq!(birds, 1, "CR 702.57b: forecast resolves once per turn; log {log:?}");
    assert!(
        log.iter().filter(|(s, _, _)| *s != Some(Step::Upkeep)).all(|(_, o, _)| !o),
        "CR 702.57a/b: forecast can't be activated outside its owner's upkeep; log {log:?}"
    );
    assert!(
        log.iter().any(|(s, _, r)| *s == Some(Step::Upkeep) && *r),
        "CR 702.57b: the forecast card is revealed in hand for the rest of the upkeep; log {log:?}"
    );
    assert!(
        log.iter().filter(|(s, _, _)| *s != Some(Step::Upkeep)).all(|(_, _, r)| !r),
        "CR 702.57b: the reveal ends when a non-upkeep step begins; log {log:?}"
    );
}

// ---------------------------------------------------------------------------
// 702.59 Recover
// ---------------------------------------------------------------------------

#[test]
fn cr_702_59a_recover_pay_returns_otherwise_exiles() {
    for pay in [true, false] {
        let mut g = new_game();
        put(
            &mut g,
            &card(
                "Grim Harvest",
                "Mana cost: {1}{B}\nType: Instant\nReturn target creature card from your graveyard to your hand.\nRecover {2}{B} (When a creature is put into your graveyard from the battlefield, you may pay {2}{B}. If you do, return this card from your graveyard to your hand. Otherwise, exile this card.)",
            ),
            ALICE,
            Zone::Graveyard,
        );
        let victim = put(&mut g, &vanilla("Recover Victim", 1, 1), ALICE, Zone::Battlefield);
        put(
            &mut g,
            &card("Murder", "Mana cost: {1}{B}{B}\nType: Instant\nDestroy target creature."),
            ALICE,
            Zone::Hand,
        );
        lands(&mut g, "Swamp", 6, ALICE);
        let mut dm = Script {
            plays: vec![cast_named("Murder")],
            prefer_targets: vec![Target::Object(victim)],
            boolean: pay,
            ..Default::default()
        };
        run_turn(&mut g, &mut dm);
        assert_eq!(zone_of(&g, "Recover Victim"), Some(Zone::Graveyard), "fixture: murder resolved");
        assert_eq!(
            zone_of(&g, "Grim Harvest"),
            Some(if pay { Zone::Hand } else { Zone::Exile }),
            "CR 702.59a: pay={pay}: recover returns the card if paid, otherwise exiles it; prompts {:?}",
            dm.booleans_seen
        );
    }
}

// ---------------------------------------------------------------------------
// 702.60 Ripple
// ---------------------------------------------------------------------------

fn surging_flame() -> CardDefinition {
    card(
        "Surging Flame",
        "Mana cost: {1}{R}\nType: Instant\nRipple 4 (When you cast this spell, you may reveal the top four cards of your library. You may cast spells with the same name as this spell from among those cards without paying their mana costs. Put the rest on the bottom of your library.)\nSurging Flame deals 2 damage to any target.",
    )
}

#[test]
fn cr_702_60a_ripple_casts_same_name_cards_free_rest_to_bottom() {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let filler = vanilla("Library Filler", 1, 1);
    for _ in 0..8 {
        g.create_object_from_definition(&filler, BOB, Zone::Library);
    }
    for _ in 0..6 {
        g.create_object_from_definition(&filler, ALICE, Zone::Library);
    }
    let flame_def = surging_flame();
    // Library order: create puts on top? Put the ripple hit among the top four regardless.
    let hit = g.create_object_from_definition(&flame_def, ALICE, Zone::Library);
    let lib: Vec<ObjectId> = g.player(ALICE).unwrap().library.iter().copied().collect();
    let hit_index = lib.iter().position(|id| *id == hit).unwrap();
    let hit_on_top = hit_index == lib.len() - 1 || hit_index == 0;
    assert!(hit_on_top, "fixture: newly created library card is at an end of the library");
    put(&mut g, &flame_def, ALICE, Zone::Hand);
    lands(&mut g, "Mountain", 2, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Surging Flame")],
        prefer_targets: vec![Target::Player(BOB)],
        option_pref: vec!["Cast Surging Flame"],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        life(&g, BOB),
        16,
        "CR 702.60a: the revealed Surging Flame is cast without paying its mana cost (two hits of 2); actions {:?}; prompts {:?} {:?}",
        dm.actions_taken,
        dm.booleans_seen,
        dm.prompts
    );
    assert_eq!(g.player(ALICE).unwrap().library.len(), 6, "CR 702.60a: revealed cards not cast stay in the library (on the bottom)");
}

// ---------------------------------------------------------------------------
// 702.61 Split second
// ---------------------------------------------------------------------------

#[test]
fn cr_702_61a_b_split_second_blocks_spells_and_nonmana_abilities_only() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Sudden Death",
            "Mana cost: {1}{B}{B}\nType: Instant\nSplit second (As long as this spell is on the stack, players can't cast spells or activate abilities that aren't mana abilities.)\nTarget creature gets -4/-4 until end of turn.",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Swamp", 3, ALICE);
    let victim = put(&mut g, &vanilla("Split Victim", 2, 2), BOB, Zone::Battlefield);
    put(
        &mut g,
        &card("Bob Instant", "Mana cost: {R}\nType: Instant\nYou gain 3 life."),
        BOB,
        Zone::Hand,
    );
    put(
        &mut g,
        &card("Bob Pinger", "Mana cost: {1}\nType: Artifact\n{1}: You gain 1 life."),
        BOB,
        Zone::Battlefield,
    );
    lands(&mut g, "Mountain", 2, BOB);
    // Observer that triggers while the spell is on the stack: "Whenever a player casts" -> ensure triggers still work.
    put(
        &mut g,
        &card(
            "Cast Watcher",
            "Mana cost: {1}\nType: Artifact\nWhenever a player casts a spell, you gain 1 life.",
        ),
        BOB,
        Zone::Battlefield,
    );
    let offers = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![cast_named("Sudden Death")],
        prefer_targets: vec![Target::Object(victim)],
        ..Default::default()
    };
    dm.observe = Some(observe_into(&offers, |g| {
        let sd_on_stack = g
            .stack
            .iter()
            .any(|e| g.object(e.object_id).is_some_and(|o| &*o.name == "Sudden Death"));
        (sd_on_stack && g.turn.priority_player == Some(BOB)).then(|| {
            let acts = ironsmith::compute_legal_actions(g, BOB).unwrap_or_default();
            (
                acts.iter().any(|a| matches!(a, LegalAction::CastSpell { .. })),
                acts.iter().any(|a| matches!(a, LegalAction::ActivateAbility { .. })),
                acts.iter().any(|a| matches!(a, LegalAction::ActivateManaAbility { .. })),
            )
        })
    }));
    run_turn(&mut g, &mut dm);
    assert_eq!(zone_of(&g, "Split Victim"), Some(Zone::Graveyard), "fixture: resolved");
    let log = offers.borrow().clone();
    assert!(!log.is_empty(), "fixture: Bob received priority with split second on the stack");
    assert!(
        log.iter().all(|(cast, act, _)| !cast && !act),
        "CR 702.61a: no spells or non-mana abilities while split second is on the stack; log {log:?}"
    );
    assert!(
        log.iter().all(|(_, _, mana)| *mana),
        "CR 702.61b: mana abilities remain activatable; log {log:?}"
    );
    assert_eq!(life(&g, BOB), 21, "CR 702.61b: triggered abilities still trigger and resolve");
}

// ---------------------------------------------------------------------------
// 702.62 Suspend
// ---------------------------------------------------------------------------

fn rift_bolt() -> CardDefinition {
    card(
        "Rift Bolt",
        "Mana cost: {2}{R}\nType: Sorcery\nRift Bolt deals 3 damage to any target.\nSuspend 1—{R} (Rather than cast this card from your hand, you may pay {R} and exile it with a time counter on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)",
    )
}

fn is_suspend_action(g: &GameState, a: &LegalAction, name: &str) -> bool {
    match a {
        LegalAction::SpecialAction(sa) => {
            let dbg = format!("{sa:?}");
            dbg.contains("Suspend")
                && named(g, name).iter().any(|(id, _)| dbg.contains(&format!("{:?}", id)))
        }
        _ => false,
    }
}

#[test]
fn cr_702_62a_b_d_suspend_exiles_ticks_and_casts_free() {
    let mut g = new_game();
    put(&mut g, &rift_bolt(), ALICE, Zone::Hand);
    lands(&mut g, "Mountain", 1, ALICE);
    let mut dm = Script {
        plays: vec![Box::new(|g, a| is_suspend_action(g, a, "Rift Bolt"))],
        prefer_targets: vec![Target::Player(BOB)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    let bolt = id_of(&g, "Rift Bolt");
    assert_eq!(
        (g.object(bolt).unwrap().zone, g.counter_count(bolt, CounterType::Time)),
        (Zone::Exile, 1),
        "CR 702.62a/b: suspend exiles the card with N time counters (it is now 'suspended'); actions {:?}",
        dm.actions_taken
    );
    // Tap out Alice's land in advance of her next turn doesn't matter: the free cast needs no mana.
    run_turns(&mut g, &mut dm, 2);
    assert_eq!(
        (zone_of(&g, "Rift Bolt"), life(&g, BOB)),
        (Some(Zone::Graveyard), 17),
        "CR 702.62a/d: the last time counter's removal lets the owner cast it without paying its mana cost"
    );
}

#[test]
fn cr_702_62c_suspend_respects_cast_prohibitions() {
    let mut g = new_game();
    put(&mut g, &rift_bolt(), ALICE, Zone::Hand);
    lands(&mut g, "Mountain", 1, ALICE);
    put(
        &mut g,
        &card("Sorcery Lock", "Mana cost: {2}\nType: Artifact\nPlayers can't cast sorcery spells."),
        BOB,
        Zone::Battlefield,
    );
    let offered = Rc::new(RefCell::new(vec![]));
    let mut dm = Script::default();
    dm.observe = Some(observe_into(&offered, |g| {
        (g.turn.priority_player == Some(ALICE) && g.turn.phase == ironsmith::Phase::FirstMain).then(|| {
            ironsmith::compute_legal_actions(g, ALICE)
                .map(|v| v.iter().any(|a| is_suspend_action(g, a, "Rift Bolt")))
                .unwrap_or(false)
        })
    }));
    run_turn(&mut g, &mut dm);
    assert!(!offered.borrow().is_empty(), "fixture: Alice had main-phase priority");
    assert!(
        offered.borrow().iter().all(|o| !o),
        "CR 702.62c: suspend can't be used for a card that couldn't be cast (sorceries prohibited); observed {:?}",
        offered.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.66 Delve
// ---------------------------------------------------------------------------

#[test]
fn cr_702_66a_b_c_delve_exiles_graveyard_cards_for_generic_after_reduction() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Double Delve Cruise",
            "Mana cost: {7}{U}\nType: Sorcery\nDelve\nDelve\nDraw three cards.",
        ),
        ALICE,
        Zone::Hand,
    );
    put(
        &mut g,
        &card("Sorcery Discount", "Mana cost: {2}\nType: Artifact\nSorcery spells you cast cost {2} less to cast."),
        ALICE,
        Zone::Battlefield,
    );
    let mut gy = vec![];
    for i in 0..5 {
        let name: &'static str = ["Delve A", "Delve B", "Delve C", "Delve D", "Delve E"][i];
        gy.push(put(&mut g, &card(name, "Mana cost: {1}\nType: Artifact"), ALICE, Zone::Graveyard));
    }
    lands(&mut g, "Island", 1, ALICE);
    let lib_before = g.player(ALICE).unwrap().library.len();
    let mut dm = Script {
        plays: vec![cast_named("Double Delve Cruise")],
        prefer_objects: gy.clone(),
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        lib_before - g.player(ALICE).unwrap().library.len(),
        3,
        "CR 702.66a/b: {{7}}{{U}} reduced by {{2}} -> {{5}}{{U}}, paid by one Island and exiling five graveyard cards; actions {:?}",
        dm.actions_taken
    );
    assert_eq!(
        gy.iter().filter(|id| g.object(**id).is_none() || g.object(**id).unwrap().zone != Zone::Graveyard).count(),
        5,
        "CR 702.66a/c: exactly five cards exiled (one per generic mana; the second delve is redundant)"
    );
}

fn cast_alt(name: &'static str) -> Play {
    Box::new(move |g, a| match a {
        LegalAction::CastSpell { spell_id, casting_method, .. } => {
            !matches!(casting_method, ironsmith::CastingMethod::Normal)
                && g.object(*spell_id).is_some_and(|o| &*o.name == name)
        }
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// 702.53b Transmute exists in every zone
// ---------------------------------------------------------------------------

#[test]
fn cr_702_53b_transmute_counts_as_activated_ability_on_battlefield() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Transmute Only Creature",
            "Mana cost: {2}{U}\nType: Creature — Spirit\nPower/Toughness: 1/1\nTransmute {1}{U}{U}",
        ),
        BOB,
        Zone::Battlefield,
    );
    put(&mut g, &vanilla("Plain Bystander", 1, 1), BOB, Zone::Battlefield);
    put(
        &mut g,
        &card("Ability Sweeper", "Mana cost: {B}\nType: Sorcery\nDestroy all creatures with activated abilities."),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Swamp", 1, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Ability Sweeper")],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(zone_of(&g, "Ability Sweeper"), Some(Zone::Graveyard), "fixture: sweeper resolved");
    assert_eq!(
        (zone_of(&g, "Transmute Only Creature"), zone_of(&g, "Plain Bystander")),
        (Some(Zone::Graveyard), Some(Zone::Battlefield)),
        "CR 702.53b: transmute still exists on the battlefield, so the object has an activated ability"
    );
}

// ---------------------------------------------------------------------------
// 702.61c Split second redundancy
// ---------------------------------------------------------------------------

#[test]
fn cr_702_61c_double_split_second_behaves_as_one() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Double Split Bolt",
            "Mana cost: {R}\nType: Instant\nSplit second\nSplit second\nThis spell deals 2 damage to any target.",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Mountain", 1, ALICE);
    put(&mut g, &card("Bob Instant", "Mana cost: {R}\nType: Instant\nYou gain 3 life."), BOB, Zone::Hand);
    lands(&mut g, "Mountain", 1, BOB);
    let offers = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![cast_named("Double Split Bolt")],
        prefer_targets: vec![Target::Player(BOB)],
        ..Default::default()
    };
    dm.observe = Some(observe_into(&offers, |g| {
        (!g.stack.is_empty() && g.turn.priority_player == Some(BOB)).then(|| {
            let acts = ironsmith::compute_legal_actions(g, BOB).unwrap_or_default();
            (
                acts.iter().any(|a| matches!(a, LegalAction::CastSpell { .. })),
                acts.iter().any(|a| matches!(a, LegalAction::ActivateManaAbility { .. })),
            )
        })
    }));
    run_turn(&mut g, &mut dm);
    assert_eq!(life(&g, BOB), 18, "fixture: resolved");
    let log = offers.borrow().clone();
    assert!(
        !log.is_empty() && log.iter().all(|(cast, mana)| !cast && *mana),
        "CR 702.61c: two split second instances act exactly like one (spells blocked, mana abilities allowed); log {log:?}"
    );
}

// ---------------------------------------------------------------------------
// 702.96 Overload
// ---------------------------------------------------------------------------

#[test]
fn cr_702_96a_b_c_overload_replaces_target_with_each_and_ignores_hexproof() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Cyclonic Rift",
            "Mana cost: {1}{U}\nType: Instant\nReturn target nonland permanent you don't control to its owner's hand.\nOverload {6}{U} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Island", 7, ALICE);
    put(&mut g, &vanilla("Bob Plain", 2, 2), BOB, Zone::Battlefield);
    put(
        &mut g,
        &card("Bob Hexproof", "Mana cost: {1}{G}\nType: Creature — Elf\nPower/Toughness: 1/1\nHexproof"),
        BOB,
        Zone::Battlefield,
    );
    put(&mut g, &card("Bob Relic", "Mana cost: {1}\nType: Artifact"), BOB, Zone::Battlefield);
    put(&mut g, &vanilla("Alice Own", 2, 2), ALICE, Zone::Battlefield);
    lands(&mut g, "Forest", 1, BOB);
    let mut dm = Script {
        plays: vec![cast_alt("Cyclonic Rift")],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(zone_of(&g, "Cyclonic Rift"), Some(Zone::Graveyard), "fixture: overloaded rift resolved {:?}", dm.actions_taken);
    assert_eq!(
        (
            zone_of(&g, "Bob Plain"),
            zone_of(&g, "Bob Hexproof"),
            zone_of(&g, "Bob Relic"),
            zone_of(&g, "Alice Own"),
            zone_of(&g, "Forest"),
        ),
        (Some(Zone::Hand), Some(Zone::Hand), Some(Zone::Hand), Some(Zone::Battlefield), Some(Zone::Battlefield)),
        "CR 702.96a/b/c: overload changes 'target' to 'each': every nonland permanent Alice doesn't control returns, including a hexproof one"
    );
}

#[test]
fn cr_702_96b_overload_requires_no_targets() {
    // With no legal targets for the normal mode, the overloaded cast is still possible.
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Mizzium Mortars",
            "Mana cost: {1}{R}\nType: Sorcery\nMizzium Mortars deals 4 damage to target creature you don't control.\nOverload {3}{R}{R}{R} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Mountain", 6, ALICE);
    put(
        &mut g,
        &card("Shrouded Beast", "Mana cost: {1}{G}\nType: Creature — Beast\nPower/Toughness: 3/3\nShroud"),
        BOB,
        Zone::Battlefield,
    );
    let mut dm = Script {
        plays: vec![cast_alt("Mizzium Mortars")],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(
        zone_of(&g, "Shrouded Beast"),
        Some(Zone::Graveyard),
        "CR 702.96b: an overloaded spell requires no targets and affects an untargetable (shroud) creature; actions {:?}",
        dm.actions_taken
    );
}

// ---------------------------------------------------------------------------
// 702.97 Scavenge
// ---------------------------------------------------------------------------

#[test]
fn cr_702_97a_scavenge_exiles_for_power_counters_at_sorcery_speed() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Deadbridge Goliath",
            "Mana cost: {2}{G}{G}\nType: Creature — Insect\nPower/Toughness: 5/5\nScavenge {4}{G}{G} ({4}{G}{G}, Exile this card from your graveyard: Put a number of +1/+1 counters equal to this card's power on target creature. Scavenge only as a sorcery.)",
        ),
        ALICE,
        Zone::Graveyard,
    );
    let host = put(&mut g, &vanilla("Scavenge Host", 1, 1), ALICE, Zone::Battlefield);
    lands(&mut g, "Forest", 6, ALICE);
    let combat_offer = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![Box::new(|g, a| {
            g.turn.phase == ironsmith::Phase::NextMain
                && matches!(a, LegalAction::ActivateAbility { source, .. } if g.object(*source).is_some_and(|o| &*o.name == "Deadbridge Goliath"))
        })],
        prefer_targets: vec![Target::Object(host)],
        ..Default::default()
    };
    dm.observe = Some(observe_into(&combat_offer, |g| {
        (g.turn.step == Some(Step::BeginCombat) && g.turn.priority_player == Some(ALICE)).then(|| {
            ironsmith::compute_legal_actions(g, ALICE)
                .map(|v| v.iter().any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if g.object(*source).is_some_and(|o| &*o.name == "Deadbridge Goliath"))))
                .unwrap_or(false)
        })
    }));
    run_turn(&mut g, &mut dm);
    assert!(
        combat_offer.borrow().iter().all(|o| !o),
        "CR 702.97a: scavenge only as a sorcery; observed {:?}",
        combat_offer.borrow()
    );
    assert_eq!(
        (zone_of(&g, "Deadbridge Goliath"), g.counter_count(host, CounterType::PlusOnePlusOne)),
        (Some(Zone::Exile), 5),
        "CR 702.97a: exile the card from the graveyard to put counters equal to its power on target creature; actions {:?}",
        dm.actions_taken
    );
}

// ---------------------------------------------------------------------------
// 702.99 Cipher
// ---------------------------------------------------------------------------

fn last_thoughts() -> CardDefinition {
    card(
        "Last Thoughts",
        "Mana cost: {3}{U}\nType: Sorcery\nDraw a card.\nCipher (Then you may exile this spell card encoded on a creature you control. Whenever that creature deals combat damage to a player, its controller may cast a copy of the encoded card without paying its mana cost.)",
    )
}

#[test]
fn cr_702_99a_b_cipher_encodes_and_copies_on_combat_damage() {
    let mut g = new_game();
    put(&mut g, &last_thoughts(), ALICE, Zone::Hand);
    lands(&mut g, "Island", 4, ALICE);
    let carrier = put(&mut g, &vanilla("Cipher Carrier", 2, 2), ALICE, Zone::Battlefield);
    let lib0 = g.player(ALICE).unwrap().library.len();
    let mut dm = Script {
        plays: vec![cast_named("Last Thoughts")],
        attack: vec![carrier],
        prefer_objects: vec![carrier],
        prefer_targets: vec![Target::Object(carrier)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(life(&g, BOB), 18, "fixture: carrier dealt combat damage");
    assert_eq!(
        zone_of(&g, "Last Thoughts"),
        Some(Zone::Exile),
        "CR 702.99a/b: the resolved cipher card is exiled encoded on a creature; prompts {:?} {:?}",
        dm.booleans_seen,
        dm.prompts
    );
    assert_eq!(
        lib0 - g.player(ALICE).unwrap().library.len(),
        2,
        "CR 702.99a: when the encoded creature deals combat damage to a player, its controller may cast a copy (second draw); prompts {:?}",
        dm.booleans_seen
    );
}

#[test]
fn cr_702_99c_cipher_stays_encoded_through_control_change() {
    let mut g = new_game();
    put(&mut g, &last_thoughts(), ALICE, Zone::Hand);
    lands(&mut g, "Island", 4, ALICE);
    let carrier = put(&mut g, &vanilla("Cipher Carrier", 2, 2), ALICE, Zone::Battlefield);
    let mut dm = Script {
        plays: vec![cast_named("Last Thoughts")],
        prefer_objects: vec![carrier],
        prefer_targets: vec![Target::Object(carrier)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(zone_of(&g, "Last Thoughts"), Some(Zone::Exile), "fixture: encoded");
    // Bob gains control of the encoded creature.
    ironsmith::effects::ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(carrier),
        ironsmith::continuous::Modification::ChangeController(BOB),
        ironsmith::effect::Until::Forever,
    )
    .execute(&mut g, &mut EffectContext::new_default(carrier, BOB))
    .unwrap();
    g.refresh_continuous_state().unwrap();
    assert_eq!(g.controller_of_id(carrier), Some(BOB), "fixture: control changed");
    let bob_lib0 = g.player(BOB).unwrap().library.len();
    g.next_turn();
    let mut dm = Script {
        attack: vec![carrier],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(life(&g, ALICE), 18, "fixture: Bob's stolen carrier dealt combat damage");
    assert_eq!(
        (zone_of(&g, "Last Thoughts"), bob_lib0 - g.player(BOB).unwrap().library.len()),
        (Some(Zone::Exile), 2),
        "CR 702.99c: the card remains encoded after a control change; the new controller may cast the copy (draw step + copy)"
    );
}

// ---------------------------------------------------------------------------
// 702.109 Dash
// ---------------------------------------------------------------------------

#[test]
fn cr_702_109a_dash_haste_and_return_at_end_step() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Kolaghan Skirmisher",
            "Mana cost: {1}{B}\nType: Creature — Human Warrior\nPower/Toughness: 2/2\nDash {2}{B} (You may cast this spell for its dash cost. If you do, it gains haste, and it's returned from the battlefield to its owner's hand at the beginning of the next end step.)",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Swamp", 3, ALICE);
    let mut dm = Script {
        plays: vec![cast_alt("Kolaghan Skirmisher")],
        ..Default::default()
    };
    // Attack with whatever named Skirmisher is on the battlefield at declare-attackers time.
    struct AttackAll;
    let _ = AttackAll;
    dm.attack = vec![];
    let seen = Rc::new(RefCell::new(vec![]));
    dm.observe = Some(observe_into(&seen, |g| {
        named(g, "Kolaghan Skirmisher").first().map(|(_, z)| (g.turn.step, *z))
    }));
    // Pre-compute the battlefield id is impossible; instead allow attacking with any creature named Skirmisher.
    let mut dm = AttackNamed { inner: dm, name: "Kolaghan Skirmisher" };
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::execute_turn_with(&mut g, &mut combat, &mut queue, &mut dm).unwrap();
    assert_eq!(
        life(&g, BOB),
        18,
        "CR 702.109a: a dashed creature has haste and can attack the turn it's cast; actions {:?}",
        dm.inner.actions_taken
    );
    assert_eq!(
        zone_of(&g, "Kolaghan Skirmisher"),
        Some(Zone::Hand),
        "CR 702.109a: the dashed permanent returns to its owner's hand at the beginning of the next end step"
    );
}

/// Wraps a Script to attack with every creature named `name`.
struct AttackNamed {
    inner: Script,
    name: &'static str,
}

impl DecisionMaker for AttackNamed {
    fn on_auto_pass(&mut self, g: &GameState, p: PlayerId) {
        self.inner.on_auto_pass(g, p)
    }
    fn decide_priority(&mut self, g: &GameState, ctx: &PriorityContext) -> LegalAction {
        self.inner.decide_priority(g, ctx)
    }
    fn decide_boolean(&mut self, g: &GameState, ctx: &BooleanContext) -> bool {
        self.inner.decide_boolean(g, ctx)
    }
    fn decide_options(&mut self, g: &GameState, ctx: &ironsmith::SelectOptionsContext) -> Vec<usize> {
        self.inner.decide_options(g, ctx)
    }
    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.inner.decide_objects(g, ctx)
    }
    fn decide_targets(&mut self, g: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.inner.decide_targets(g, ctx)
    }
    fn decide_attackers(&mut self, g: &GameState, ctx: &AttackersContext) -> Vec<AttackerDeclaration> {
        self.inner.attack = ctx
            .attacker_options
            .iter()
            .filter(|o| g.object(o.creature).is_some_and(|ob| &*ob.name == self.name))
            .map(|o| o.creature)
            .collect();
        self.inner.decide_attackers(g, ctx)
    }
}

// ---------------------------------------------------------------------------
// 702.103 Bestow
// ---------------------------------------------------------------------------

fn hopeful_eidolon() -> CardDefinition {
    card(
        "Hopeful Eidolon",
        "Mana cost: {W}\nType: Enchantment Creature — Spirit\nPower/Toughness: 1/1\nBestow {3}{W} (If you cast this card for its bestow cost, it's an Aura spell with enchant creature. It becomes a creature again if it's not attached.)\nLifelink (Damage dealt by this creature also causes you to gain that much life.)\nEnchanted creature gets +1/+1 and has lifelink.",
    )
}

fn is_creature(g: &GameState, id: ObjectId) -> bool {
    g.object(id).is_some_and(|o| {
        g.current_card_types(id).is_some_and(|t| t.contains(&ironsmith::CardType::Creature))
            && o.zone == Zone::Battlefield
    })
}

#[test]
fn cr_702_103f_bestowed_aura_becomes_creature_when_host_leaves() {
    let mut g = new_game();
    put(&mut g, &hopeful_eidolon(), ALICE, Zone::Hand);
    let host = put(&mut g, &vanilla("Bestow Host", 2, 2), ALICE, Zone::Battlefield);
    put(
        &mut g,
        &card("Murder", "Mana cost: {1}{B}{B}\nType: Instant\nDestroy target creature."),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Plains", 4, ALICE);
    lands(&mut g, "Swamp", 3, ALICE);
    let mid = Rc::new(RefCell::new(vec![]));
    let mut dm = Script {
        plays: vec![cast_alt("Hopeful Eidolon"), cast_named_empty_stack("Murder")],
        prefer_targets: vec![Target::Object(host)],
        ..Default::default()
    };
    dm.observe = Some(observe_into(&mid, move |g| {
        (zone_of(g, "Bestow Host") == Some(Zone::Battlefield) && zone_of(g, "Hopeful Eidolon") == Some(Zone::Battlefield))
            .then(|| g.current_power(host).unwrap_or(0))
    }));
    run_turn(&mut g, &mut dm);
    assert!(mid.borrow().contains(&3), "fixture: bestowed Eidolon enchanted the host (+1/+1); observed {:?}; actions {:?}", mid.borrow(), dm.actions_taken);
    assert_eq!(zone_of(&g, "Bestow Host"), Some(Zone::Graveyard), "fixture: host destroyed");
    let eid = id_of(&g, "Hopeful Eidolon");
    assert!(
        is_creature(&g, eid),
        "CR 702.103f: an unattached bestowed Aura stops being bestowed and remains on the battlefield as a creature (zone {:?})",
        zone_of(&g, "Hopeful Eidolon")
    );
}

#[test]
fn cr_702_103e_bestow_target_illegal_on_resolution_resolves_as_creature() {
    let mut g = new_game();
    put(&mut g, &hopeful_eidolon(), ALICE, Zone::Hand);
    let host = put(&mut g, &vanilla("Bestow Host", 2, 2), ALICE, Zone::Battlefield);
    put(
        &mut g,
        &card("Murder", "Mana cost: {1}{B}{B}\nType: Instant\nDestroy target creature."),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Plains", 4, ALICE);
    lands(&mut g, "Swamp", 3, ALICE);
    let mut dm = Script {
        // Murder is cast in response, while the bestowed spell is still on the stack.
        plays: vec![cast_alt("Hopeful Eidolon"), cast_named("Murder")],
        prefer_targets: vec![Target::Object(host)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(dm.plays.len(), 0, "fixture: both cast {:?}", dm.actions_taken);
    assert_eq!(zone_of(&g, "Bestow Host"), Some(Zone::Graveyard), "fixture: host destroyed first");
    let eid = id_of(&g, "Hopeful Eidolon");
    assert!(
        is_creature(&g, eid),
        "CR 702.103e: with its target illegal, the bestowed spell ceases to be bestowed and resolves as a creature (zone {:?})",
        zone_of(&g, "Hopeful Eidolon")
    );
}

#[test]
fn cr_702_103g_bestowed_aura_phasing_in_unattached_becomes_creature() {
    let mut g = new_game();
    put(&mut g, &hopeful_eidolon(), ALICE, Zone::Hand);
    let host = put(&mut g, &vanilla("Bestow Host", 2, 2), ALICE, Zone::Battlefield);
    lands(&mut g, "Plains", 4, ALICE);
    let mut dm = Script {
        plays: vec![cast_alt("Hopeful Eidolon")],
        prefer_targets: vec![Target::Object(host)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    let eid = id_of(&g, "Hopeful Eidolon");
    assert_eq!(g.current_power(host), Some(3), "fixture: attached as bestowed Aura");
    // The Aura phases out on its own; the host then leaves; the Aura phases in unattached.
    g.phase_out(eid);
    g.move_object_by_effect(host, Zone::Graveyard).unwrap();
    g.phase_in(eid);
    g.refresh_continuous_state().unwrap();
    let mut queue = TriggerQueue::new();
    let _ = ironsmith::game_loop::check_and_apply_sbas(&mut g, &mut queue);
    let eid = named(&g, "Hopeful Eidolon").first().map(|(id, _)| *id).unwrap();
    assert!(
        is_creature(&g, eid),
        "CR 702.103g: a bestowed Aura that phases in unattached ceases to be bestowed and is a creature (zone {:?})",
        zone_of(&g, "Hopeful Eidolon")
    );
}

#[test]
fn cr_702_103c_copy_of_bestowed_aura_spell_is_bestowed() {
    let mut g = new_game();
    put(&mut g, &hopeful_eidolon(), ALICE, Zone::Hand);
    let host = put(&mut g, &vanilla("Bestow Host", 2, 2), ALICE, Zone::Battlefield);
    put(
        &mut g,
        &card("Spell Copier", "Mana cost: {U}\nType: Instant\nCopy target spell. You may choose new targets for the copy."),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Plains", 4, ALICE);
    lands(&mut g, "Island", 1, ALICE);
    let mut dm = Script {
        plays: vec![cast_alt("Hopeful Eidolon"), cast_named("Spell Copier")],
        prefer_targets: vec![Target::Object(host)],
        boolean: false,
        ..Default::default()
    };
    // Make Spell Copier target the Eidolon spell: prefer stack targets by name at decision time.
    let mut dm = TargetSpellNamed { inner: dm, spell: "Hopeful Eidolon" };
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::execute_turn_with(&mut g, &mut combat, &mut queue, &mut dm).unwrap();
    assert_eq!(dm.inner.plays.len(), 0, "fixture: both cast {:?}", dm.inner.actions_taken);
    let copies: Vec<ObjectId> = named(&g, "Hopeful Eidolon")
        .into_iter()
        .filter(|(_, z)| *z == Zone::Battlefield)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(copies.len(), 2, "fixture: original and token copy both resolved");
    assert_eq!(
        (g.current_power(host), copies.iter().filter(|id| is_creature(&g, **id)).count()),
        (Some(4), 0),
        "CR 702.103c: the copy of a bestowed Aura spell is also bestowed, so both enchant the host"
    );
}

/// Wraps a Script so spell-targeting requirements pick the named spell on the stack.
struct TargetSpellNamed {
    inner: Script,
    spell: &'static str,
}

impl DecisionMaker for TargetSpellNamed {
    fn on_auto_pass(&mut self, g: &GameState, p: PlayerId) {
        self.inner.on_auto_pass(g, p)
    }
    fn decide_priority(&mut self, g: &GameState, ctx: &PriorityContext) -> LegalAction {
        self.inner.decide_priority(g, ctx)
    }
    fn decide_boolean(&mut self, g: &GameState, ctx: &BooleanContext) -> bool {
        self.inner.decide_boolean(g, ctx)
    }
    fn decide_options(&mut self, g: &GameState, ctx: &ironsmith::SelectOptionsContext) -> Vec<usize> {
        self.inner.decide_options(g, ctx)
    }
    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.inner.decide_objects(g, ctx)
    }
    fn decide_targets(&mut self, g: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let spell_targets: Vec<Target> = g
            .stack
            .iter()
            .filter(|e| g.object(e.object_id).is_some_and(|o| &*o.name == self.spell))
            .map(|e| Target::Object(e.object_id))
            .collect();
        self.inner.prefer_targets.splice(0..0, spell_targets);
        let out = self.inner.decide_targets(g, ctx);
        out
    }
}

// ---------------------------------------------------------------------------
// 702.95d Soulbond: one pair only
// ---------------------------------------------------------------------------

#[test]
fn cr_702_95d_paired_creature_cannot_pair_with_a_second_creature() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Silverblade Paladin",
            "Mana cost: {1}{W}{W}\nType: Creature — Human Knight\nPower/Toughness: 2/2\nSoulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as this creature is paired with another creature, both creatures have double strike.",
        ),
        ALICE,
        Zone::Hand,
    );
    put(
        &mut g,
        &card(
            "Wolfir Silverheart",
            "Mana cost: {3}{G}{G}\nType: Creature — Wolf Warrior\nPower/Toughness: 4/4\nSoulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as this creature is paired with another creature, each of those creatures gets +4/+4.",
        ),
        ALICE,
        Zone::Hand,
    );
    let partner = put(&mut g, &vanilla("Soulbond Partner", 1, 1), ALICE, Zone::Battlefield);
    lands(&mut g, "Plains", 3, ALICE);
    lands(&mut g, "Forest", 5, ALICE);
    let mut dm = Script {
        plays: vec![cast_named_empty_stack("Silverblade Paladin"), cast_named_empty_stack("Wolfir Silverheart")],
        prefer_targets: vec![Target::Object(partner)],
        prefer_objects: vec![partner],
        option_pref: vec!["Soulbond Partner", "Wolfir Silverheart", "Silverblade Paladin"],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(dm.plays.len(), 0, "fixture: both cast {:?}", dm.actions_taken);
    let paladin = id_of(&g, "Silverblade Paladin");
    let wolfir = id_of(&g, "Wolfir Silverheart");
    assert!(
        g.is_soulbond_paired(paladin) && g.is_soulbond_paired(partner),
        "fixture: Paladin paired with Partner on entry; prompts {:?} {:?}",
        dm.booleans_seen,
        dm.prompts
    );
    assert!(
        !g.is_soulbond_paired(wolfir),
        "CR 702.95d: Wolfir can't pair with an already-paired creature (and no unpaired creature exists)"
    );
    assert_eq!(
        (g.current_power(partner), g.current_power(paladin), g.current_power(wolfir)),
        (Some(1), Some(2), Some(4)),
        "CR 702.95d: Paladin and Partner are paired, so neither can pair with Wolfir (no +4/+4 anywhere)"
    );
}

// ---------------------------------------------------------------------------
// 702.106 Hidden agenda
// ---------------------------------------------------------------------------

#[test]
fn cr_702_106b_d_hidden_agenda_name_is_secret_and_linked_per_object() {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let conspiracy = |name: &str, rules: &str| {
        let mut def = card(name, &format!("Type: Conspiracy\n{rules}"));
        def.card.card_types = vec![ironsmith::CardType::Conspiracy];
        def
    };
    let agenda = conspiracy(
        "Agenda Anthem",
        "Hidden agenda\nCreatures you control with the chosen name get +1/+1.",
    );
    let agenda2 = conspiracy(
        "Second Agenda Anthem",
        "Hidden agenda\nCreatures you control with the chosen name get +2/+2.",
    );
    g.enable_conspiracy(vec![(
        ALICE,
        vec![
            ironsmith::ConspiracySetupCard { definition: agenda, agenda_names: vec!["Grizzly Bears".into()] },
            ironsmith::ConspiracySetupCard { definition: agenda2, agenda_names: vec!["Runeclaw Bear".into()] },
        ],
    )])
    .unwrap();
    let ids: Vec<ObjectId> = g.conspiracy_cards().into_iter().collect();
    for id in &ids {
        assert!(
            g.agenda_names_for(BOB, *id).is_none(),
            "CR 702.106b: the secretly chosen name is hidden from opponents while face down"
        );
    }
    for id in &ids {
        g.turn.priority_player = Some(ALICE);
        ironsmith::special_actions::perform(
            ironsmith::special_actions::SpecialAction::TurnConspiracyFaceUp { conspiracy_id: *id },
            &mut g,
            ALICE,
            &mut Script::default(),
        )
        .unwrap();
    }
    let mut chosen: Vec<Vec<String>> = ids
        .iter()
        .map(|id| g.agenda_names_for(ALICE, *id).unwrap().iter().map(|n| n.to_string()).collect())
        .collect();
    chosen.sort();
    assert_eq!(
        chosen,
        vec![vec!["Grizzly Bears".to_string()], vec!["Runeclaw Bear".to_string()]],
        "fixture: each conspiracy recorded its own secret name"
    );
    let bears = put(&mut g, &vanilla("Grizzly Bears", 2, 2), ALICE, Zone::Battlefield);
    let runeclaw = put(&mut g, &vanilla("Runeclaw Bear", 2, 2), ALICE, Zone::Battlefield);
    let other = put(&mut g, &vanilla("Balduvian Bears", 2, 2), ALICE, Zone::Battlefield);
    g.refresh_continuous_state().unwrap();
    assert_eq!(
        (g.current_power(bears), g.current_power(runeclaw), g.current_power(other)),
        (Some(3), Some(4), Some(2)),
        "CR 702.106d: each 'chosen name' ability refers only to the name chosen by its own object's hidden agenda"
    );
}

#[test]
fn cr_702_100b_evolves_trigger_fires_when_evolve_adds_counters() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Renegade Krasis",
            "Mana cost: {1}{G}{G}\nType: Creature — Beast Mutant\nPower/Toughness: 3/2\nEvolve (Whenever a creature you control enters, if that creature has greater power or toughness than this creature, put a +1/+1 counter on this creature.)\nWhenever this creature evolves, put a +1/+1 counter on each other creature you control with a +1/+1 counter on it.",
        ),
        ALICE,
        Zone::Battlefield,
    );
    let marked = put(&mut g, &vanilla("Countered Ally", 1, 1), ALICE, Zone::Battlefield);
    g.add_counters(marked, CounterType::PlusOnePlusOne, 1);
    put(&mut g, &vanilla("Big Entrant", 4, 4), ALICE, Zone::Hand);
    lands(&mut g, "Forest", 1, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Big Entrant")],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    let krasis = id_of(&g, "Renegade Krasis");
    assert_eq!(g.counter_count(krasis, CounterType::PlusOnePlusOne), 1, "fixture: krasis evolved");
    assert_eq!(
        g.counter_count(marked, CounterType::PlusOnePlusOne),
        2,
        "CR 702.100b: putting counters via its evolve ability means the creature 'evolves', triggering 'whenever this creature evolves'"
    );
}

#[test]
fn cr_702_60b_each_ripple_instance_triggers_separately() {
    let mut g = new_game();
    put(
        &mut g,
        &card(
            "Double Ripple Flame",
            "Mana cost: {1}{R}\nType: Instant\nRipple 4\nRipple 4\nThis spell deals 2 damage to any target.",
        ),
        ALICE,
        Zone::Hand,
    );
    lands(&mut g, "Mountain", 2, ALICE);
    let mut dm = Script {
        plays: vec![cast_named("Double Ripple Flame")],
        prefer_targets: vec![Target::Player(BOB)],
        ..Default::default()
    };
    run_turn(&mut g, &mut dm);
    assert_eq!(life(&g, BOB), 18, "fixture: spell resolved");
    let ripple_prompts = dm.booleans_seen.iter().filter(|b| b.to_lowercase().contains("ripple")).count();
    assert_eq!(
        ripple_prompts,
        2,
        "CR 702.60b: each ripple instance triggers separately; prompts {:?}",
        dm.booleans_seen
    );
}
