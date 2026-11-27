//! Random-playout throughput benchmark: how fast can the native engine play
//! whole games when every decision is made by a seeded random agent?
//!
//!   cargo build --release -p ironsmith-tools --bin bench_playouts
//!   bench_playouts [format] [games] [seed] [max_turns]
//!
//! Decks are the premade tournament lists in web/ui/public/catalog/<format>.
//! Card compilation happens once up front and is not timed. Every game is two
//! random decks, seeded shuffle, 7-card hands, no mulligans.
//!
//! Environment:
//!   BENCH_MAX_DECISIONS  stop a game after this many priority decisions (default 600)
//!   BENCH_DIGEST=1       print a per-game digest of every decision the agent saw
//!                        (same seed + same digest = same game; the correctness gate
//!                        for engine optimizations). Adds formatting time.
//!   BENCH_PROBE=1        every 4th priority decision, also time `GameState::clone()`
//!                        and a fresh `compute_legal_actions` (excluded from engine time)
//!   BENCH_VERBOSE=1      per-turn timing lines
//!   BENCH_TRACE=1        per-decision trace lines
//!   BENCH_ONLY=<n>       play only game index n of the sequence
//!   BENCH_DECKS=a,b      play two text deck lists ("4 Name" lines) instead of the catalog
//!   BENCH_SIZES=1        print the in-memory size of the main model types and exit
//!   BENCH_EXACT_MENU=1   offer only actions with an exact payment proof (the engine
//!                        default); otherwise menus are bounded like the browser's

use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    AttackersContext, BlockersContext, BooleanContext, PriorityContext, TargetsContext,
};
use ironsmith::decisions::spec::{AttackerDeclaration, BlockerDeclaration};
use ironsmith::engine::prelude::{GameState, PlayerId, TriggerQueue, Zone, execute_turn_with};
use ironsmith::game_state::Target;
use ironsmith::targeting::normalize_targets_for_requirements;
use ironsmith::{CombatState, GameLoopError};
use ironsmith_tools::{compile_definition_from_payload, default_cards_path, normalize_lookup_name};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const PROBE_EVERY: u64 = 4;
/// Non-pass actions one player may take in one turn before the agent only passes.
/// Keeps a random agent from looping on repeatable activations forever.
const ACTIONS_PER_TURN_CAP: u32 = 30;

// ---------------------------------------------------------------------------
// RNG (SplitMix64; no extra dependency)
// ---------------------------------------------------------------------------

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next() % den < num
    }
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

// ---------------------------------------------------------------------------
// Random agent
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
struct Stats {
    priority_decisions: u64,
    actions_taken: u64,
    other_decisions: u64,
    cancelled: u64,
    clone_ns: Vec<u64>,
    legal_ns: Vec<u64>,
    legal_counts: Vec<usize>,
    /// Time spent in probes and digest formatting; subtracted from engine time.
    overhead: Duration,
}

struct Flags {
    digest: bool,
    probe: bool,
    verbose: bool,
    trace: bool,
    max_decisions: u64,
}

struct RandomAgent<'a> {
    rng: Rng,
    fallback: SelectFirstDecisionMaker,
    stats: Stats,
    turn_seen: u32,
    actions_this_turn: u32,
    flags: &'a Flags,
    digest: std::collections::hash_map::DefaultHasher,
}

impl RandomAgent<'_> {
    fn probe(&mut self, game: &GameState, player: PlayerId) {
        let t = Instant::now();
        let copy = game.clone();
        self.stats.clone_ns.push(t.elapsed().as_nanos() as u64);
        drop(copy);
        let t = Instant::now();
        let actions = compute_legal_actions(game, player);
        self.stats.legal_ns.push(t.elapsed().as_nanos() as u64);
        if let Ok(actions) = actions {
            self.stats.legal_counts.push(actions.len());
        }
    }

    fn note(&mut self, value: impl Hash) {
        if self.flags.digest {
            value.hash(&mut self.digest);
        }
    }

    fn exhausted(&self) -> bool {
        self.stats.priority_decisions >= self.flags.max_decisions
    }
}

impl DecisionMaker for RandomAgent<'_> {
    fn on_action_cancelled(&mut self, _game: &GameState, reason: &str) {
        self.stats.cancelled += 1;
        self.note(("cancel", reason));
        if self.flags.trace {
            eprintln!("    cancelled: {reason}");
        }
    }

    fn decide_priority(&mut self, game: &GameState, ctx: &PriorityContext) -> LegalAction {
        self.stats.priority_decisions += 1;
        let overhead = Instant::now();
        if self.flags.digest {
            let life: Vec<i32> = game.players.iter().map(|p| p.life).collect();
            let text = format!(
                "{} {:?} {:?} {} {:?} {} {:?}",
                game.turn.turn_number,
                game.turn.phase,
                game.turn.step,
                ctx.player.index(),
                life,
                game.battlefield.len(),
                ctx.actions
            );
            text.hash(&mut self.digest);
        }
        if std::env::var("BENCH_CLONE_PROBE").ok().and_then(|v| v.parse::<u64>().ok())
            == Some(self.stats.priority_decisions)
        {
            eprintln!("clone probe at decision {} (battlefield {}):", self.stats.priority_decisions, game.battlefield.len());
            for (name, ns) in game.clone_cost_report(200).into_iter().take(40) {
                eprintln!("  {ns:>9} ns  {name}");
            }
        }
        if self.flags.probe && self.stats.priority_decisions % PROBE_EVERY == 0 {
            self.probe(game, ctx.player);
        }
        if self.flags.trace {
            eprintln!(
                "    prio t{} {:?}/{:?} p{} actions={}",
                game.turn.turn_number,
                game.turn.phase,
                game.turn.step,
                ctx.player.index(),
                ctx.actions.len()
            );
            if std::env::var_os("BENCH_TRACE_ACTIONS").is_some() {
                for action in &ctx.actions {
                    let text = format!("{action:?}");
                    eprintln!("        {}", text.chars().take(200).collect::<String>());
                }
            }
        }
        self.stats.overhead += overhead.elapsed();
        if game.turn.turn_number != self.turn_seen {
            self.turn_seen = game.turn.turn_number;
            self.actions_this_turn = 0;
        }
        // Mana abilities are left to the payment planner, as a client would.
        let candidates: Vec<&LegalAction> = ctx
            .actions
            .iter()
            .filter(|a| {
                !matches!(
                    a,
                    LegalAction::PassPriority | LegalAction::ActivateManaAbility { .. }
                )
            })
            .collect();
        let can_pass = ctx.actions.iter().any(|a| matches!(a, LegalAction::PassPriority));
        let act = !candidates.is_empty()
            && !self.exhausted()
            && self.actions_this_turn < ACTIONS_PER_TURN_CAP
            && (!can_pass || self.rng.chance(3, 4));
        if act {
            self.actions_this_turn += 1;
            self.stats.actions_taken += 1;
            let chosen = candidates[self.rng.below(candidates.len())].clone();
            if self.flags.trace {
                let name = |id: &ironsmith::ids::ObjectId| {
                    game.object(*id).map(|o| o.name.to_string()).unwrap_or_default()
                };
                let label = match &chosen {
                    LegalAction::CastSpell { spell_id, .. } => format!("cast {}", name(spell_id)),
                    LegalAction::PlayLand { land_id } => format!("land {}", name(land_id)),
                    LegalAction::ActivateAbility { source, .. } => {
                        format!("activate {}", name(source))
                    }
                    other => format!("{other:?}").chars().take(120).collect(),
                };
                eprintln!("      -> {label}");
            }
            return chosen;
        }
        if can_pass {
            LegalAction::PassPriority
        } else {
            ctx.actions.first().cloned().unwrap_or(LegalAction::PassPriority)
        }
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        ctx: &AttackersContext,
    ) -> Vec<AttackerDeclaration> {
        self.stats.other_decisions += 1;
        let mut out = Vec::new();
        for o in &ctx.attacker_options {
            if o.valid_targets.is_empty() || !(o.must_attack || self.rng.chance(1, 2)) {
                continue;
            }
            let target = o.valid_targets[self.rng.below(o.valid_targets.len())].clone();
            out.push(AttackerDeclaration {
                creature: o.creature,
                target,
            });
        }
        self.note(("attackers", ctx.attacker_options.len(), out.len()));
        out
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        ctx: &BlockersContext,
    ) -> Vec<BlockerDeclaration> {
        self.stats.other_decisions += 1;
        let mut used = BTreeSet::new();
        let mut out = Vec::new();
        for option in &ctx.blocker_options {
            if option.min_blockers > 1 || !self.rng.chance(1, 2) {
                continue;
            }
            let free: Vec<_> = option
                .valid_blockers
                .iter()
                .filter(|(id, _)| !used.contains(id))
                .collect();
            if free.is_empty() {
                continue;
            }
            let (blocker, _) = free[self.rng.below(free.len())];
            used.insert(*blocker);
            out.push(BlockerDeclaration {
                blocker: *blocker,
                blocking: option.attacker,
            });
        }
        self.note(("blockers", ctx.blocker_options.len(), out.len()));
        out
    }

    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.stats.other_decisions += 1;
        let mut proposed = Vec::new();
        for requirement in &ctx.requirements {
            let mut pool = requirement.legal_targets.clone();
            self.rng.shuffle(&mut pool);
            let count = requirement
                .max_targets
                .unwrap_or(1)
                .max(requirement.min_targets)
                .min(pool.len());
            proposed.extend(pool.into_iter().take(count));
        }
        let legal: usize = ctx.requirements.iter().map(|r| r.legal_targets.len()).sum();
        self.note(("targets", ctx.requirements.len(), legal));
        normalize_targets_for_requirements(&ctx.requirements, proposed)
            .unwrap_or_else(|| self.fallback.decide_targets(game, ctx))
    }

    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.stats.other_decisions += 1;
        self.note(("bool", ctx.can_accept, ctx.description.as_str()));
        ctx.can_accept && self.rng.chance(1, 2)
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        self.stats.other_decisions += 1;
        self.note(("number", ctx.min, ctx.max));
        // Small random value: X announcements are unbounded (u32::MAX).
        let span = ctx.max.saturating_sub(ctx.min).min(4);
        ctx.min + self.rng.below(span as usize + 1) as u32
    }

    // Everything else: deterministic first-legal choices (counted as decisions).
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ironsmith::ids::ObjectId> {
        self.stats.other_decisions += 1;
        let legal = ctx.candidates.iter().filter(|c| c.legal).count();
        self.note(("objects", ctx.candidates.len(), legal, ctx.min));
        self.fallback.decide_objects(game, ctx)
    }
    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.stats.other_decisions += 1;
        let legal = ctx.options.iter().filter(|o| o.legal).count();
        self.note(("options", ctx.options.len(), legal, ctx.min, ctx.max));
        self.fallback.decide_options(game, ctx)
    }
    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::OrderContext,
    ) -> Vec<ironsmith::ids::ObjectId> {
        self.note(("order", ctx.items.len()));
        self.fallback.decide_order(game, ctx)
    }
    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        self.fallback.decide_distribute(game, ctx)
    }
    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        self.fallback.decide_colors(game, ctx)
    }
    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::CountersContext,
    ) -> Vec<(ironsmith::object::CounterType, u32)> {
        self.fallback.decide_counters(game, ctx)
    }
    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::PartitionContext,
    ) -> Vec<ironsmith::ids::ObjectId> {
        self.fallback.decide_partition(game, ctx)
    }
    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::ProliferateContext,
    ) -> ironsmith::decisions::specs::ProliferateResponse {
        self.fallback.decide_proliferate(game, ctx)
    }
}

// ---------------------------------------------------------------------------
// Decks
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct DeckEntry {
    name: String,
    count: usize,
}
#[derive(serde::Deserialize)]
struct DeckFile {
    name: String,
    mainboard: Vec<DeckEntry>,
}

struct Deck {
    name: String,
    cards: Vec<String>,
}

fn load_decks(format: &str) -> Vec<Deck> {
    let dir: PathBuf = default_cards_path()
        .parent()
        .unwrap()
        .join("web/ui/public/catalog")
        .join(format)
        .join("details");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|p| {
            let file: DeckFile = serde_json::from_slice(&std::fs::read(&p).ok()?).ok()?;
            let cards: Vec<String> = file
                .mainboard
                .iter()
                .flat_map(|e| std::iter::repeat_n(e.name.clone(), e.count))
                .collect();
            (cards.len() >= 40).then_some(Deck { name: file.name, cards })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// One game
// ---------------------------------------------------------------------------

struct GameResult {
    wall: Duration,
    engine: Duration,
    turns: u32,
    finished: bool,
    error: Option<String>,
    stats: Stats,
    digest: u64,
}

fn play_game(
    decks: [&Vec<CardDefinition>; 2],
    seed: u64,
    max_turns: u32,
    flags: &Flags,
) -> GameResult {
    let mut rng = Rng(seed);
    let mut game = GameState::new(vec!["P0".into(), "P1".into()], 20);
    for (p, deck) in decks.iter().enumerate() {
        let mut order: Vec<usize> = (0..deck.len()).collect();
        rng.shuffle(&mut order);
        for i in order {
            game.create_object_from_definition(&deck[i], PlayerId::from_index(p as u8), Zone::Library);
        }
    }
    let mut agent = RandomAgent {
        rng: Rng(seed ^ 0xA5A5_A5A5),
        fallback: SelectFirstDecisionMaker,
        stats: Stats::default(),
        turn_seen: 0,
        actions_this_turn: 0,
        flags,
        digest: std::collections::hash_map::DefaultHasher::new(),
    };
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();

    let start = Instant::now();
    game.draw_cards(PlayerId::from_index(0), 7);
    game.draw_cards(PlayerId::from_index(1), 7);
    let mut turns = 0;
    let mut finished = false;
    let mut error = None;
    while turns < max_turns {
        if game.players.iter().filter(|p| p.is_in_game()).count() <= 1 {
            finished = true;
            break;
        }
        turns += 1;
        let turn_start = Instant::now();
        let prio_before = agent.stats.priority_decisions;
        let res = execute_turn_with(&mut game, &mut combat, &mut queue, &mut agent);
        if flags.verbose {
            eprintln!(
                "  turn {turns:3}: {:9.1} ms, {:4} priority decisions, life {:?}, battlefield {}",
                turn_start.elapsed().as_secs_f64() * 1e3,
                agent.stats.priority_decisions - prio_before,
                game.players.iter().map(|p| p.life).collect::<Vec<_>>(),
                game.battlefield.len()
            );
        }
        match res {
            Ok(()) => {}
            Err(GameLoopError::GameOver) => {
                finished = true;
                break;
            }
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
        }
        let next = PlayerId::from_index(
            ((game.turn.active_player.index() + 1) % game.players.len()) as u8,
        );
        game.turn.active_player = next;
        game.turn.priority_player = Some(next);
        game.turn.turn_number += 1;
        if let Some(player) = game.player_mut(next) {
            player.begin_turn();
        }
    }
    let wall = start.elapsed();
    let life: Vec<i32> = game.players.iter().map(|p| p.life).collect();
    (turns, &life, game.battlefield.len(), finished).hash(&mut agent.digest);
    GameResult {
        wall,
        engine: wall.saturating_sub(agent.stats.overhead),
        turns,
        finished,
        error,
        digest: agent.digest.finish(),
        stats: agent.stats,
    }
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

fn pct(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn us(ns: u64) -> f64 {
    ns as f64 / 1e3
}

fn main() {
    // Card compilation recurses deeply; the CLI uses a large stack for the same reason.
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

fn run() {
    let args: Vec<String> = std::env::args().collect();
    let format = args.get(1).cloned().unwrap_or_else(|| "modern".into());
    let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
    let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let max_turns: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(60);
    let env_on = |name: &str| std::env::var_os(name).is_some();
    let flags = Flags {
        digest: env_on("BENCH_DIGEST"),
        probe: env_on("BENCH_PROBE"),
        verbose: env_on("BENCH_VERBOSE"),
        trace: env_on("BENCH_TRACE"),
        max_decisions: std::env::var("BENCH_MAX_DECISIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600),
    };
    // BENCH_EXACT_MENU=1 keeps the default exact payment proof for every offered
    // action; otherwise menus are bounded the way the browser's are.
    ironsmith::game_loop::set_priority_analysis_bounded(!env_on("BENCH_EXACT_MENU"));
    if env_on("BENCH_SIZES") {
        use std::mem::size_of;
        println!("size_of ObjectFilter = {}", size_of::<ironsmith::target::ObjectFilter>());
        println!("size_of Ability = {}", size_of::<ironsmith::Ability>());
        println!("size_of ContinuousEffect = {}", size_of::<ironsmith::continuous::ContinuousEffect>());
        println!("size_of CalculatedCharacteristics = {}", size_of::<ironsmith::continuous::CalculatedCharacteristics>());
        println!("size_of Object = {}", size_of::<ironsmith::object::Object>());
        println!("size_of GameState = {}", size_of::<GameState>());
        println!("size_of LegalAction = {}", size_of::<LegalAction>());
        return;
    }
    let only: Option<usize> = std::env::var("BENCH_ONLY").ok().and_then(|s| s.parse().ok());

    // BENCH_DECKS=a.txt,b.txt plays only those two text lists ("4 Lightning Bolt"
    // lines, '#' comments), seats swapping every other game, so a run can use the
    // same decks as another engine's benchmark.
    let fixed_decks: Option<Vec<Deck>> = std::env::var("BENCH_DECKS").ok().map(|list| {
        list.split(',')
            .map(|path| {
                let text = std::fs::read_to_string(path)
                    .unwrap_or_else(|e| panic!("read {path}: {e}"));
                let mut cards = Vec::new();
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let (count, name) = line.split_once(' ').expect("count name");
                    let count: usize = count.trim_end_matches('x').parse().expect("count");
                    cards.extend(std::iter::repeat_n(name.trim().to_string(), count));
                }
                let name = std::path::Path::new(path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Deck { name, cards }
            })
            .collect()
    });
    let fixed = fixed_decks.is_some();
    let decks = fixed_decks.unwrap_or_else(|| load_decks(&format));
    assert!(decks.len() >= 2, "need at least two decks for {format}");

    // Fix the schedule first so only the scheduled decks' cards are compiled.
    let mut rng = Rng(seed);
    let schedule: Vec<(usize, usize, u64)> = (0..games)
        .map(|g| {
            if fixed {
                return if g % 2 == 0 { (0, 1, rng.next()) } else { (1, 0, rng.next()) };
            }
            let a = rng.below(decks.len());
            let mut b = rng.below(decks.len());
            if b == a {
                b = (b + 1) % decks.len();
            }
            (a, b, rng.next())
        })
        .collect();
    let scheduled: Vec<(usize, usize, usize, u64)> = schedule
        .iter()
        .enumerate()
        .filter(|(g, _)| only.is_none_or(|o| o == *g))
        .map(|(g, (a, b, s))| (g, *a, *b, *s))
        .collect();

    // Compile every card the scheduled decks use, once (untimed setup).
    let t = Instant::now();
    let names: Vec<String> = scheduled
        .iter()
        .flat_map(|(_, a, b, _)| decks[*a].cards.iter().chain(&decks[*b].cards).cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let payloads =
        ironsmith_tools::load_card_payloads_by_names(default_cards_path().to_str().unwrap(), &names)
            .expect("load card payloads");
    let mut defs: BTreeMap<String, CardDefinition> = BTreeMap::new();
    let mut failed = BTreeSet::new();
    for name in &names {
        let compiled = payloads
            .get(&normalize_lookup_name(name))
            .and_then(|p| p.first())
            .ok_or_else(|| "no payload".to_string())
            .and_then(compile_definition_from_payload);
        match compiled {
            Ok(def) => {
                defs.insert(name.clone(), def);
            }
            Err(_) => {
                failed.insert(name.clone());
            }
        }
    }
    eprintln!(
        "setup: {} games scheduled, {} distinct cards, {} failed to compile, {:.1}s",
        scheduled.len(),
        names.len(),
        failed.len(),
        t.elapsed().as_secs_f64()
    );
    if !failed.is_empty() {
        eprintln!("  uncompiled: {}", failed.iter().cloned().collect::<Vec<_>>().join(", "));
    }
    let build = |deck: &Deck| -> Option<Vec<CardDefinition>> {
        deck.cards.iter().map(|c| defs.get(c).cloned()).collect()
    };

    let mut results = Vec::new();
    for (g, a, b, game_seed) in scheduled {
        let (Some(deck_a), Some(deck_b)) = (build(&decks[a]), build(&decks[b])) else {
            println!("game {g:3}: SKIPPED (uncompiled card)");
            continue;
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            play_game([&deck_a, &deck_b], game_seed, max_turns, &flags)
        }));
        match outcome {
            Ok(r) => {
                println!(
                    "game {g:3}: {:>26} vs {:<26} turns={:3} {:8} engine={:9.1}ms prio={:5} acts={:4} other={:4} cancels={:3}{}{}",
                    decks[a].name.chars().take(26).collect::<String>(),
                    decks[b].name.chars().take(26).collect::<String>(),
                    r.turns,
                    if r.finished { "finished" } else { "capped" },
                    r.engine.as_secs_f64() * 1e3,
                    r.stats.priority_decisions,
                    r.stats.actions_taken,
                    r.stats.other_decisions,
                    r.stats.cancelled,
                    if flags.digest { format!(" digest={:016x}", r.digest) } else { String::new() },
                    r.error.as_deref().map(|e| format!(" ERROR {e}")).unwrap_or_default(),
                );
                results.push(r);
            }
            Err(_) => println!("game {g:3}: PANIC ({} vs {})", decks[a].name, decks[b].name),
        }
    }

    if std::env::var_os("BENCH_WORK").is_some() {
        let decisions: u64 = results
            .iter()
            .map(|r| r.stats.priority_decisions + r.stats.other_decisions)
            .sum::<u64>()
            .max(1);
        eprintln!("work over {decisions} decisions:");
        for (name, count) in ironsmith::take_work_counts() {
            eprintln!("  {count:>10}  {:>9.2}/decision  {name}", count as f64 / decisions as f64);
        }
    }
    if std::env::var_os("IRONSMITH_PAY_TIERS").is_some() {
        eprintln!("payment tiers:");
        for (group, name, count, nanos) in ironsmith::take_tier_counts() {
            eprintln!("  {group:>12} {name:<28} {count:>9} {:>10.1} ms", nanos as f64 / 1e6);
        }
    }
    if std::env::var_os("IRONSMITH_PAY_TIERS").is_some() {
        eprintln!("payment tier notes:");
        for (group, detail, count) in ironsmith::take_tier_notes() {
            eprintln!("  {count:>7} {group}: {detail}");
        }
    }
    if std::env::var_os("IRONSMITH_COW_PROBE").is_some() {
        eprintln!("copy-on-write copies by site:");
        for (site, count) in ironsmith::cow_copy_report().into_iter().take(40) {
            eprintln!("  {count:>8}  {site}");
        }
    }

    // Aggregate.
    let n = results.len().max(1) as f64;
    let engine_total: f64 = results.iter().map(|r| r.engine.as_secs_f64()).sum();
    let wall_total: f64 = results.iter().map(|r| r.wall.as_secs_f64()).sum();
    let turns_total: u64 = results.iter().map(|r| r.turns as u64).sum();
    let prio_total: u64 = results.iter().map(|r| r.stats.priority_decisions).sum();
    let other_total: u64 = results.iter().map(|r| r.stats.other_decisions).sum();
    let acts_total: u64 = results.iter().map(|r| r.stats.actions_taken).sum();
    let finished = results.iter().filter(|r| r.finished).count();
    let mut per_game: Vec<u64> = results.iter().map(|r| r.engine.as_nanos() as u64).collect();
    per_game.sort_unstable();
    let mut clone_ns: Vec<u64> = results.iter().flat_map(|r| r.stats.clone_ns.clone()).collect();
    let mut legal_ns: Vec<u64> = results.iter().flat_map(|r| r.stats.legal_ns.clone()).collect();
    let legal_counts: Vec<usize> = results.iter().flat_map(|r| r.stats.legal_counts.clone()).collect();
    clone_ns.sort_unstable();
    legal_ns.sort_unstable();

    println!();
    println!(
        "=== {format}: {} games, seed {seed}, max {max_turns} turns, max {} priority decisions ===",
        results.len(),
        flags.max_decisions
    );
    println!(
        "finished {finished}/{} | {:.1} turns/game | {:.0} priority decisions/game | {:.0} actions/game",
        results.len(),
        turns_total as f64 / n,
        prio_total as f64 / n,
        acts_total as f64 / n
    );
    println!(
        "engine time: total {:.2} s (wall {:.2} s) | per game mean {:.1} ms, median {:.1} ms, max {:.1} ms",
        engine_total,
        wall_total,
        engine_total * 1e3 / n,
        pct(&per_game, 0.5) as f64 / 1e6,
        pct(&per_game, 1.0) as f64 / 1e6
    );
    println!(
        "playouts/sec (1 core): {:.3} | ms/turn: {:.1} | decisions/sec: {:.0} | mean engine µs per decision: {:.0}",
        n / engine_total,
        engine_total * 1e3 / turns_total.max(1) as f64,
        (prio_total + other_total) as f64 / engine_total,
        engine_total * 1e6 / (prio_total + other_total).max(1) as f64
    );
    if flags.probe {
        println!(
            "GameState::clone  ({} samples): median {:.1} µs, p90 {:.1} µs, p99 {:.1} µs, max {:.1} µs",
            clone_ns.len(),
            us(pct(&clone_ns, 0.5)),
            us(pct(&clone_ns, 0.9)),
            us(pct(&clone_ns, 0.99)),
            us(pct(&clone_ns, 1.0))
        );
        println!(
            "compute_legal_actions ({} samples, mean {:.1} actions): median {:.1} µs, p90 {:.1} µs, p99 {:.1} µs, max {:.1} µs",
            legal_ns.len(),
            legal_counts.iter().sum::<usize>() as f64 / legal_counts.len().max(1) as f64,
            us(pct(&legal_ns, 0.5)),
            us(pct(&legal_ns, 0.9)),
            us(pct(&legal_ns, 0.99)),
            us(pct(&legal_ns, 1.0))
        );
    }
}
