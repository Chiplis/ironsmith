//! CR 732.2 loop shortcuts on real, compiled cards.
//!
//! Each combo is set up from oracle text, performed by hand until the engine
//! offers the shortcut, and then checked three ways: the per-lap delta the
//! offer reports, the number of laps the shortcut executed, and that the
//! resulting game equals the same number of laps performed by hand in a
//! second game that never had shortcuts enabled.
//!
//! Combos are the well-known ones listed at https://edhrec.com/combos.
//! Set `LOOP_TRACE=1` to print every decision and the board after each step.
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction};
use ironsmith::decisions::context::{
    BooleanContext, ColorsContext, DecisionContext, ManaPaymentContext, NumberContext,
    PriorityContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    LoopShortcutStop, PriorityLoopState, advance_priority_with_dm, apply_decision_context_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, Color, CounterType, GameState, ObjectId, ObjectKind, Phase, PlayerId, Zone,
};
use ironsmith_tools::{
    compile_definition_from_payload, default_cards_path, load_card_payloads_by_names,
    normalize_lookup_name,
};
use std::collections::{BTreeMap, VecDeque};
use std::sync::OnceLock;

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

const CARDS: &[&str] = &[
    "Pili-Pala",
    "Grand Architect",
    "Basalt Monolith",
    "Rings of Brighthearth",
    "Isochron Scepter",
    "Dramatic Reversal",
    "Sol Ring",
    "Mind Stone",
    "Kinnan, Bonder Prodigy",
    "Forsaken Monument",
    "Forensic Gadgeteer",
    "Grim Monolith",
    "Power Artifact",
    "Devoted Druid",
    "Vizier of Remedies",
    "Ashnod's Altar",
    "Nim Deathmantle",
    "Grave Titan",
    "Felidar Guardian",
    "Saheeli Rai",
    "Deadeye Navigator",
    "Peregrine Drake",
    "Heliod, Sun-Crowned",
    "Walking Ballista",
    "Gravecrawler",
    "Phyrexian Altar",
    "Blood Artist",
    "Diregraf Ghoul",
    "Treasonous Ogre",
    "Greed",
    "Yawgmoth, Thran Physician",
    "Young Wolf",
    "Butcher Ghoul",
    "Bridge from Below",
    "Island",
    "Forest",
];

fn definitions() -> &'static BTreeMap<String, CardDefinition> {
    static DEFS: OnceLock<BTreeMap<String, CardDefinition>> = OnceLock::new();
    DEFS.get_or_init(|| {
        let names: Vec<String> = CARDS.iter().map(|name| name.to_string()).collect();
        let payloads =
            load_card_payloads_by_names(default_cards_path().to_str().unwrap(), &names).unwrap();
        let mut defs = BTreeMap::new();
        for name in CARDS {
            let faces = payloads
                .get(&normalize_lookup_name(name))
                .unwrap_or_else(|| panic!("{name} is missing from the card data"));
            let def = compile_definition_from_payload(&faces[0])
                .unwrap_or_else(|error| panic!("{name} failed to compile: {error}"));
            defs.insert(name.to_string(), def);
        }
        defs
    })
}

fn trace_enabled() -> bool {
    std::env::var_os("LOOP_TRACE").is_some()
}

// ---------------------------------------------------------------------------
// The player at the table
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Intent {
    Pass,
    Shortcut,
    Mana(&'static str, Option<usize>),
    Ability(&'static str, Option<usize>),
    Cast(&'static str),
}

#[derive(Default)]
struct Pilot {
    intent: Option<Intent>,
    /// Target wanted for a spell or ability whose prompt context or source
    /// name contains the key.
    target_rules: Vec<(&'static str, &'static str)>,
    /// One-shot object choices, consumed before `object_names`.
    next_objects: VecDeque<Vec<&'static str>>,
    /// Names chosen (as many as allowed) whenever objects are selected.
    object_names: Vec<&'static str>,
    /// Option whose description contains one of these is chosen.
    option_words: Vec<&'static str>,
    color: Option<Color>,
    decline_optional: bool,
    shortcut_laps: u32,
    /// `Some(k)`: Bob responds after k repetitions instead of accepting.
    bob_interrupts_after: Option<u32>,
    log: Vec<String>,
}

impl Pilot {
    fn note(&mut self, line: String) {
        if trace_enabled() {
            eprintln!("    . {line}");
        }
        self.log.push(line);
    }

    fn name(game: &GameState, id: ObjectId) -> String {
        game.object(id)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| format!("#{}", id.0))
    }
}

impl DecisionMaker for Pilot {
    fn decide_priority(&mut self, game: &GameState, ctx: &PriorityContext) -> LegalAction {
        if ctx.player != ALICE {
            return LegalAction::PassPriority;
        }
        let intent = self.intent.take().expect("Alice was asked to act without an intent");
        let named = |id: &ObjectId, name: &str| game.object(*id).is_some_and(|o| o.name == name);
        let found = ctx.actions.iter().find(|action| match (&intent, action) {
            (Intent::Pass, LegalAction::PassPriority) => true,
            (Intent::Shortcut, LegalAction::ShortcutLoop) => true,
            (
                Intent::Mana(name, index),
                LegalAction::ActivateManaAbility {
                    source,
                    ability_index,
                },
            )
            | (
                Intent::Ability(name, index),
                LegalAction::ActivateAbility {
                    source,
                    ability_index,
                },
            ) => named(source, name) && index.is_none_or(|index| index == *ability_index),
            (Intent::Cast(name), LegalAction::CastSpell { spell_id, .. }) => named(spell_id, name),
            _ => false,
        });
        match found {
            Some(action) => {
                self.note(format!("priority: {intent:?} -> {action:?}"));
                action.clone()
            }
            None => {
                let menu: Vec<String> = ctx
                    .actions
                    .iter_with_labels()
                    .map(|(action, label)| format!("{label} = {action:?}"))
                    .collect();
                panic!(
                    "{intent:?} is not available.\nmenu:\n  {}\nlog:\n  {}",
                    menu.join("\n  "),
                    self.log.join("\n  ")
                );
            }
        }
    }

    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        let answer = if ctx.player == BOB {
            self.bob_interrupts_after.is_some() && ctx.description.contains("proposes repeating")
        } else {
            !self.decline_optional
        };
        self.note(format!("boolean [{}]: {} -> {answer}", ctx.player.0, ctx.description));
        answer
    }

    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        let answer = if ctx.description.starts_with("Repeat this loop") {
            self.shortcut_laps
        } else if ctx.description.starts_with("Let the loop repeat") {
            self.bob_interrupts_after.unwrap_or(ctx.max)
        } else {
            ctx.min
        };
        let answer = answer.clamp(ctx.min, ctx.max);
        self.note(format!("number [{}]: {} -> {answer}", ctx.player.0, ctx.description));
        answer
    }

    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let source = Self::name(game, ctx.source);
        let wanted = self
            .target_rules
            .iter()
            .find(|(key, _)| ctx.context.contains(key) || source.contains(key))
            .map(|(_, name)| *name);
        let mut chosen = Vec::new();
        for requirement in &ctx.requirements {
            let pick = requirement
                .legal_targets
                .iter()
                .find(|target| match (wanted, target) {
                    (Some("@bob"), Target::Player(player)) => *player == BOB,
                    (Some("@alice"), Target::Player(player)) => *player == ALICE,
                    (Some(name), Target::Object(id)) => Self::name(game, *id) == name,
                    _ => false,
                })
                .or_else(|| {
                    requirement
                        .legal_targets
                        .iter()
                        .find(|target| matches!(target, Target::Player(player) if *player == BOB))
                })
                .or_else(|| requirement.legal_targets.first());
            if requirement.min_targets > 0 || wanted.is_some() {
                chosen.extend(pick.copied());
            }
        }
        self.note(format!("targets for {source} ({}): {chosen:?}", ctx.context));
        chosen
    }

    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        // A one-shot list names one object per entry; the standing list
        // takes every object with a listed name, up to the limit.
        let one_shot = self.next_objects.pop_front();
        let take_all = one_shot.is_none();
        let names = one_shot.unwrap_or_else(|| self.object_names.clone());
        let limit = ctx.max.unwrap_or(usize::MAX);
        let mut chosen: Vec<ObjectId> = Vec::new();
        for name in &names {
            for candidate in &ctx.candidates {
                if chosen.len() < limit
                    && candidate.legal
                    && !chosen.contains(&candidate.id)
                    && Self::name(game, candidate.id) == *name
                {
                    chosen.push(candidate.id);
                    if !take_all {
                        break;
                    }
                }
            }
        }
        for candidate in &ctx.candidates {
            if chosen.len() >= ctx.min {
                break;
            }
            if candidate.legal && !chosen.contains(&candidate.id) {
                chosen.push(candidate.id);
            }
        }
        let shown: Vec<String> = chosen.iter().map(|id| Self::name(game, *id)).collect();
        self.note(format!("objects: {} -> {shown:?}", ctx.description));
        chosen
    }

    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let preferred = ctx.options.iter().find(|option| {
            option.legal
                && self
                    .option_words
                    .iter()
                    .any(|word| option.description.to_lowercase().contains(word))
        });
        let answer = match preferred {
            Some(option) => vec![option.index],
            None => ironsmith::decision::SelectFirstDecisionMaker.decide_options(game, ctx),
        };
        let shown: Vec<&str> = ctx.options.iter().map(|o| o.description.as_str()).collect();
        self.note(format!("options: {} {shown:?} -> {answer:?}", ctx.description));
        answer
    }

    fn decide_colors(&mut self, _game: &GameState, ctx: &ColorsContext) -> Vec<Color> {
        let color = self.color.unwrap_or(Color::Blue);
        self.note(format!("colors: {} -> {color:?}", ctx.description));
        vec![color; ctx.count as usize]
    }

    fn decide_mana_payment(
        &mut self,
        _game: &GameState,
        ctx: &ManaPaymentContext,
    ) -> ManaPaymentResponse {
        assert!(
            ctx.plan.payable,
            "payment for {} is not payable.\nlog:\n  {}",
            ctx.subject,
            self.log.join("\n  ")
        );
        self.note(format!("pay for {}", ctx.subject));
        ManaPaymentResponse::Confirm {
            plan_id: ctx.plan.id,
            request_hash: ctx.plan.request_hash,
        }
    }
}

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

struct Table {
    game: GameState,
    queue: TriggerQueue,
    state: PriorityLoopState,
    progress: Option<GameProgress>,
    pilot: Pilot,
}

impl Table {
    fn new(shortcuts: bool) -> Self {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.turn_number = 3;
        game.turn.active_player = ALICE;
        game.turn.priority_player = Some(ALICE);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.set_loop_shortcuts_enabled(shortcuts);
        let state = PriorityLoopState::new(game.players_in_game());
        Self {
            game,
            queue: TriggerQueue::new(),
            state,
            progress: None,
            pilot: Pilot::default(),
        }
    }

    fn put(&mut self, name: &str, owner: PlayerId, zone: Zone) -> ObjectId {
        let def = definitions()
            .get(name)
            .unwrap_or_else(|| panic!("{name} is not in the CARDS list"));
        let id = self.game.create_object_from_definition(def, owner, zone);
        if zone == Zone::Battlefield {
            self.game.remove_summoning_sickness(id);
        }
        id
    }

    fn battlefield(&mut self, names: &[&str]) {
        for name in names {
            self.put(name, ALICE, Zone::Battlefield);
        }
    }

    fn mana(&mut self, symbol: ManaSymbol, amount: u32) {
        self.game
            .player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(symbol, amount);
    }

    fn find(&self, name: &str, zone: Zone) -> ObjectId {
        self.game
            .objects_in_deterministic_order()
            .into_iter()
            .find(|object| object.name == name && object.zone == zone)
            .map(|object| object.id)
            .unwrap_or_else(|| panic!("{name} is not in {zone:?}"))
    }

    fn pool(&self) -> u64 {
        self.game.player(ALICE).unwrap().mana_pool.total() as u64
    }

    fn fail(&self, message: &str) -> ! {
        eprintln!("engine note: {:?}", self.game.loop_shortcut_note());
        panic!(
            "{message}\nboard: {}\nlog:\n  {}",
            self.describe(),
            self.pilot.log.join("\n  ")
        );
    }

    fn describe(&self) -> String {
        let stack: Vec<String> = self
            .game
            .stack
            .iter()
            .map(|entry| {
                entry
                    .source_name
                    .clone()
                    .unwrap_or_else(|| Pilot::name(&self.game, entry.object_id))
            })
            .collect();
        let board: Vec<String> = self
            .game
            .battlefield
            .iter()
            .map(|id| {
                format!(
                    "{}{}",
                    Pilot::name(&self.game, *id),
                    if self.game.is_tapped(*id) { "(T)" } else { "" }
                )
            })
            .collect();
        format!(
            "life {}/{} pool {:?} stack {stack:?} battlefield {board:?}",
            self.game.player(ALICE).unwrap().life,
            self.game.player(BOB).unwrap().life,
            self.game.player(ALICE).unwrap().mana_pool,
        )
    }

    /// One engine call, answered by the pilot.
    fn step(&mut self) {
        let progress = match self.progress.take() {
            Some(GameProgress::NeedsDecisionCtx(ctx)) => apply_decision_context_with_dm(
                &mut self.game,
                &mut self.queue,
                &mut self.state,
                &ctx,
                &mut self.pilot,
            ),
            Some(GameProgress::GameOver(result)) => {
                self.progress = Some(GameProgress::GameOver(result));
                return;
            }
            _ => advance_priority_with_dm(&mut self.game, &mut self.queue, &mut self.pilot),
        };
        match progress {
            Ok(progress) => self.progress = Some(progress),
            Err(error) => self.fail(&format!("engine error: {error}")),
        }
    }

    fn game_over(&self) -> bool {
        matches!(self.progress, Some(GameProgress::GameOver(_)))
    }

    /// Answer everything until Alice is asked for a priority action.
    fn settle(&mut self) {
        for _ in 0..200 {
            match &self.progress {
                Some(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(ctx)))
                    if ctx.player == ALICE =>
                {
                    return;
                }
                Some(GameProgress::GameOver(_)) => return,
                _ => self.step(),
            }
        }
        self.fail("the game did not return priority to Alice");
    }

    fn act(&mut self, intent: Intent) {
        self.settle();
        if self.game_over() {
            return;
        }
        self.pilot.intent = Some(intent.clone());
        self.step();
        self.settle();
        if !self.pilot.next_objects.is_empty() {
            self.fail("a queued object choice was never asked for");
        }
        if trace_enabled() {
            eprintln!("  {intent:?} => {}", self.describe());
        }
    }

    fn tap_for_mana(&mut self, name: &'static str) {
        self.act(Intent::Mana(name, None));
    }

    fn activate(&mut self, name: &'static str) {
        self.act(Intent::Ability(name, None));
    }

    fn activate_index(&mut self, name: &'static str, index: usize) {
        self.act(Intent::Ability(name, Some(index)));
    }

    fn cast(&mut self, name: &'static str) {
        self.act(Intent::Cast(name));
    }

    fn pass(&mut self) {
        self.act(Intent::Pass);
    }

    /// Pass until the stack is empty.
    fn resolve_all(&mut self) {
        for _ in 0..50 {
            self.settle();
            if self.game.stack.is_empty() || self.game_over() {
                return;
            }
            self.pass();
        }
        self.fail("the stack did not empty");
    }

    fn offered(&mut self) -> bool {
        self.settle();
        match &self.progress {
            Some(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(ctx))) => {
                ctx.actions.contains(&LegalAction::ShortcutLoop)
            }
            _ => false,
        }
    }

    fn per_lap(&self) -> Vec<(String, Option<u8>, i64)> {
        let offer = self.game.loop_shortcut_offer().expect("an offer");
        let mut entries: Vec<_> = offer
            .per_lap
            .iter()
            .map(|entry| {
                (
                    entry.resource.clone(),
                    entry.player.map(|player| player.0),
                    entry.amount,
                )
            })
            .collect();
        entries.sort();
        entries
    }

    fn shortcut(&mut self, laps: u32) {
        self.pilot.shortcut_laps = laps;
        self.act(Intent::Shortcut);
    }

    /// Everything a player could see, without object identifiers.
    fn digest(&self) -> BTreeMap<String, i64> {
        let game = &self.game;
        let mut out = BTreeMap::new();
        for player in game.players.iter() {
            let who = player.id.0;
            out.insert(format!("life {who}"), i64::from(player.life));
            out.insert(format!("poison {who}"), i64::from(player.poison_counters));
            out.insert(format!("pool {who} {:?}", player.mana_pool), 1);
            out.insert(format!("restricted {who}"), player.restricted_mana.len() as i64);
            out.insert(format!("library {who}"), player.library.len() as i64);
        }
        out.insert("stack".to_string(), game.stack.len() as i64);
        for object in game.objects_in_deterministic_order() {
            if object.zone == Zone::Library {
                continue;
            }
            let on_battlefield = object.zone == Zone::Battlefield;
            let host = object
                .attached_to
                .and_then(|target| target.object_id())
                .map(|id| Pilot::name(game, id));
            let key = format!(
                "{:?} {} owner {} controller {} tapped {} sick {} counters {:?} on {host:?} token {}",
                object.zone,
                object.name,
                object.owner.0,
                game.controller_of(object).0,
                on_battlefield && game.is_tapped(object.id),
                on_battlefield && game.is_summoning_sick(object.id),
                object.counters.counts(),
                object.kind == ObjectKind::Token,
            );
            *out.entry(key).or_insert(0) += 1;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// The shared check
// ---------------------------------------------------------------------------

struct Combo {
    setup: fn(&mut Table),
    lap: fn(&mut Table),
    /// Laps performed by hand before the offer appears. One, unless the first
    /// lap leaves the board different from how it found it (an Equipment gets
    /// attached, a permanent that started untapped-and-ready re-enters).
    manual_laps: u32,
    per_lap: &'static [(&'static str, Option<u8>, i64)],
}

fn assert_same(shortcut: &Table, manual: &Table, what: &str) {
    let (left, right) = (shortcut.digest(), manual.digest());
    if left != right {
        let only_left: Vec<_> = left.iter().filter(|(k, v)| right.get(*k) != Some(v)).collect();
        let only_right: Vec<_> = right.iter().filter(|(k, v)| left.get(*k) != Some(v)).collect();
        panic!("{what}: shortcut and manual games differ\nshortcut only: {only_left:#?}\nmanual only: {only_right:#?}");
    }
}

fn check(combo: &Combo, laps: u32) {
    // The game that takes the shortcut.
    let mut table = Table::new(true);
    (combo.setup)(&mut table);
    table.resolve_all();
    assert!(!table.offered(), "nothing has been repeated yet");
    let mut performed = 0;
    while performed < combo.manual_laps {
        performed += 1;
        if trace_enabled() {
            eprintln!("manual lap {performed}");
        }
        (combo.lap)(&mut table);
        if table.offered() {
            break;
        }
    }
    if !table.offered() {
        table.fail("the loop was not offered after it was performed");
    }
    let mut expected: Vec<(String, Option<u8>, i64)> = combo
        .per_lap
        .iter()
        .map(|(resource, player, amount)| (resource.to_string(), *player, *amount))
        .collect();
    expected.sort();
    assert_eq!(table.per_lap(), expected, "per-lap delta");
    table.shortcut(laps);
    let report = table.game.last_loop_shortcut_report().expect("a report").clone();
    assert_eq!(
        (report.requested, report.accepted, report.executed, &report.stop),
        (laps, laps, laps, &LoopShortcutStop::Completed),
        "shortcut report"
    );
    assert!(table.offered(), "the loop stays on offer after a full run");

    // The same laps by hand, in a game that never recorded anything.
    let mut manual = Table::new(false);
    (combo.setup)(&mut manual);
    manual.resolve_all();
    for _ in 0..performed + laps {
        (combo.lap)(&mut manual);
    }
    assert!(!manual.offered(), "shortcuts are off unless enabled");
    assert_same(&table, &manual, "after the shortcut");

    // And the loop keeps working by hand afterwards.
    (combo.lap)(&mut table);
    (combo.lap)(&mut manual);
    assert_same(&table, &manual, "one more manual lap");
    assert_eq!(performed, combo.manual_laps, "laps performed by hand before the offer");
}

// ---------------------------------------------------------------------------
// Mana loops made only of mana abilities
// ---------------------------------------------------------------------------

/// Pili-Pala + Grand Architect: tap Pili-Pala (blue) for {C}{C}, spend it on
/// Pili-Pala's untap ability for one mana of any color.
const PILI_PALA: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Grand Architect", "Pili-Pala"]);
        t.mana(ManaSymbol::Blue, 1);
        t.pilot.target_rules = vec![("Grand Architect", "Pili-Pala")];
        t.pilot.object_names = vec!["Pili-Pala"];
        t.pilot.color = Some(Color::Green);
        // Pili-Pala becomes blue for the turn.
        t.activate("Grand Architect");
        t.resolve_all();
    },
    lap: |t| {
        t.tap_for_mana("Grand Architect");
        t.tap_for_mana("Pili-Pala");
    },
    manual_laps: 1,
    per_lap: &[("green mana", Some(0), 1)],
};

#[test]
fn pili_pala_grand_architect_pure_mana_ability_loop() {
    check(&PILI_PALA, 5);
}

// ---------------------------------------------------------------------------
// Mana loops through the stack
// ---------------------------------------------------------------------------

/// Basalt Monolith + Rings of Brighthearth: copy the untap ability, tap in
/// between the copy and the original. Nets {C} per lap.
const BASALT_RINGS: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Basalt Monolith", "Rings of Brighthearth"]);
        t.mana(ManaSymbol::Colorless, 5);
        let monolith = t.find("Basalt Monolith", Zone::Battlefield);
        t.game.tap(monolith);
    },
    lap: |t| {
        t.activate("Basalt Monolith"); // {3}: untap; Rings triggers
        t.pass(); // Rings: pay {2}, copy
        t.pass(); // copy: untap
        t.tap_for_mana("Basalt Monolith");
        t.pass(); // original: untap
        t.tap_for_mana("Basalt Monolith");
    },
    manual_laps: 1,
    per_lap: &[("colorless mana", Some(0), 1)],
};

#[test]
fn basalt_monolith_rings_of_brighthearth() {
    check(&BASALT_RINGS, 4);
}

/// Isochron Scepter + Dramatic Reversal with Sol Ring and Mind Stone.
const SCEPTER_REVERSAL: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Sol Ring", "Mind Stone"]);
        t.put("Isochron Scepter", ALICE, Zone::Hand);
        t.put("Dramatic Reversal", ALICE, Zone::Hand);
        t.mana(ManaSymbol::Colorless, 2);
        t.pilot.object_names = vec!["Dramatic Reversal"];
        t.cast("Isochron Scepter");
        t.resolve_all(); // imprint Dramatic Reversal
    },
    lap: |t| {
        t.tap_for_mana("Sol Ring");
        t.tap_for_mana("Mind Stone");
        t.activate("Isochron Scepter");
        t.resolve_all();
    },
    manual_laps: 1,
    per_lap: &[("colorless mana", Some(0), 1)],
};

#[test]
fn isochron_scepter_dramatic_reversal() {
    check(&SCEPTER_REVERSAL, 4);
}

/// Kinnan, Bonder Prodigy + Basalt Monolith: the Monolith taps for four.
const KINNAN_BASALT: Combo = Combo {
    setup: |t| t.battlefield(&["Kinnan, Bonder Prodigy", "Basalt Monolith"]),
    lap: |t| {
        t.tap_for_mana("Basalt Monolith");
        t.activate("Basalt Monolith");
        t.resolve_all();
    },
    manual_laps: 1,
    per_lap: &[("colorless mana", Some(0), 1)],
};

#[test]
fn kinnan_basalt_monolith() {
    check(&KINNAN_BASALT, 6);
}

#[test]
fn kinnan_does_not_ask_which_type_when_only_one_was_produced() {
    let mut table = Table::new(false);
    table.battlefield(&["Kinnan, Bonder Prodigy", "Basalt Monolith"]);
    table.tap_for_mana("Basalt Monolith");
    assert_eq!(table.pool(), 4, "three from the Monolith, one from Kinnan");
    let asked: Vec<&String> = table.pilot.log.iter().filter(|line| line.starts_with("options:")).collect();
    assert!(asked.is_empty(), "a one-type menu is not a choice: {asked:?}");
}

/// Forsaken Monument + Basalt Monolith.
const MONUMENT_BASALT: Combo = Combo {
    setup: |t| t.battlefield(&["Forsaken Monument", "Basalt Monolith"]),
    lap: |t| {
        t.tap_for_mana("Basalt Monolith");
        t.activate("Basalt Monolith");
        t.resolve_all();
    },
    manual_laps: 1,
    per_lap: &[("colorless mana", Some(0), 1)],
};

#[test]
fn forsaken_monument_basalt_monolith() {
    check(&MONUMENT_BASALT, 6);
}

/// Forensic Gadgeteer + Basalt Monolith: the untap costs {2}.
const GADGETEER_BASALT: Combo = Combo {
    setup: |t| t.battlefield(&["Forensic Gadgeteer", "Basalt Monolith"]),
    lap: |t| {
        t.tap_for_mana("Basalt Monolith");
        t.activate("Basalt Monolith");
        t.resolve_all();
    },
    manual_laps: 1,
    per_lap: &[("colorless mana", Some(0), 1)],
};

#[test]
fn forensic_gadgeteer_basalt_monolith() {
    check(&GADGETEER_BASALT, 6);
}

/// Grim Monolith + Power Artifact: the untap costs {2}.
const GRIM_POWER_ARTIFACT: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Grim Monolith"]);
        t.put("Power Artifact", ALICE, Zone::Hand);
        t.mana(ManaSymbol::Blue, 2);
        t.pilot.target_rules = vec![("Power Artifact", "Grim Monolith")];
        t.cast("Power Artifact");
        t.resolve_all();
    },
    lap: |t| {
        t.tap_for_mana("Grim Monolith");
        t.activate("Grim Monolith");
        t.resolve_all();
    },
    manual_laps: 1,
    per_lap: &[("colorless mana", Some(0), 1)],
};

#[test]
fn grim_monolith_power_artifact() {
    check(&GRIM_POWER_ARTIFACT, 6);
}

/// Devoted Druid + Vizier of Remedies: the untap puts no counter.
const DRUID_VIZIER: Combo = Combo {
    setup: |t| t.battlefield(&["Vizier of Remedies", "Devoted Druid"]),
    lap: |t| {
        t.tap_for_mana("Devoted Druid");
        t.activate("Devoted Druid");
        t.resolve_all();
    },
    manual_laps: 1,
    per_lap: &[("green mana", Some(0), 1)],
};

#[test]
fn devoted_druid_vizier_of_remedies() {
    check(&DRUID_VIZIER, 6);
}

// ---------------------------------------------------------------------------
// Loops that are not about mana
// ---------------------------------------------------------------------------

/// Ashnod's Altar + Nim Deathmantle + Grave Titan: sacrifice the Titan and a
/// Zombie for {4}, return the Titan, make two Zombies. Nets one Zombie.
const ALTAR_DEATHMANTLE: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Ashnod's Altar", "Nim Deathmantle", "Grave Titan"]);
        t.mana(ManaSymbol::Colorless, 2);
    },
    lap: |t| {
        // The only creature is sacrificed without a prompt; otherwise the
        // standing choice names it.
        t.pilot.object_names = vec!["Grave Titan"];
        t.tap_for_mana("Ashnod's Altar"); // Deathmantle triggers
        if t.pool() < 4 {
            t.pilot.object_names = vec!["Zombie Token"];
            t.tap_for_mana("Ashnod's Altar");
        }
        t.resolve_all(); // pay {4}, Titan returns, two Zombies
    },
    // The first lap attaches the Deathmantle and spends the seed mana.
    manual_laps: 2,
    per_lap: &[("Zombie Token (token)", None, 1)],
};

#[test]
fn ashnods_altar_nim_deathmantle_grave_titan() {
    check(&ALTAR_DEATHMANTLE, 4);
}

/// Felidar Guardian + Saheeli Rai: copy the Guardian, the copy blinks Saheeli.
const SAHEELI_GUARDIAN: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Felidar Guardian"]);
        let saheeli = t.put("Saheeli Rai", ALICE, Zone::Battlefield);
        let loyalty = t.game.object(saheeli).unwrap().counters.counts().get(&CounterType::Loyalty).copied();
        if loyalty.unwrap_or(0) == 0 {
            t.game.add_counters(saheeli, CounterType::Loyalty, 3);
        }
        t.pilot.target_rules = vec![
            ("Saheeli Rai", "Felidar Guardian"),
            ("Felidar Guardian", "Saheeli Rai"),
        ];
    },
    lap: |t| {
        t.activate_index("Saheeli Rai", 1); // -2: token copy of the Guardian
        t.resolve_all(); // the copy's trigger blinks Saheeli
    },
    // Saheeli starts ready; from the first blink on she has just entered.
    manual_laps: 2,
    per_lap: &[("Felidar Guardian (token)", None, 1)],
};

#[test]
fn felidar_guardian_saheeli_rai_blink_loop() {
    check(&SAHEELI_GUARDIAN, 4);
}

/// Deadeye Navigator + Peregrine Drake with five Islands.
const DEADEYE_DRAKE: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Island", "Island", "Island", "Island", "Island", "Peregrine Drake"]);
        t.put("Deadeye Navigator", ALICE, Zone::Hand);
        t.mana(ManaSymbol::Blue, 6);
        t.pilot.object_names = vec!["Island", "Peregrine Drake", "Deadeye Navigator"];
        t.pilot.target_rules = vec![("Deadeye Navigator", "Peregrine Drake")];
        t.cast("Deadeye Navigator");
        t.resolve_all(); // soulbond with the Drake
    },
    lap: |t| {
        for _ in 0..5 {
            t.tap_for_mana("Island");
        }
        t.activate("Peregrine Drake"); // {1}{U}: blink
        t.resolve_all(); // Drake untaps five lands and pairs again
    },
    // The Drake starts ready; from the first blink on it has just entered.
    manual_laps: 2,
    per_lap: &[("blue mana", Some(0), 3)],
};

#[test]
fn deadeye_navigator_peregrine_drake_blink_loop() {
    check(&DEADEYE_DRAKE, 3);
}

/// Heliod, Sun-Crowned + Walking Ballista: ping, gain a life, get the counter back.
const HELIOD_BALLISTA: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Heliod, Sun-Crowned"]);
        let ballista = t.put("Walking Ballista", ALICE, Zone::Battlefield);
        t.game.add_counters(ballista, CounterType::PlusOnePlusOne, 2);
        t.mana(ManaSymbol::White, 2);
        t.pilot.target_rules = vec![("Heliod", "Walking Ballista"), ("Walking Ballista", "@bob")];
        t.activate("Heliod, Sun-Crowned"); // Ballista gains lifelink
        t.resolve_all();
    },
    lap: |t| {
        t.activate("Walking Ballista");
        t.resolve_all();
    },
    manual_laps: 1,
    per_lap: &[("life", Some(0), 1), ("life", Some(1), -1)],
};

#[test]
fn heliod_walking_ballista_damage_loop() {
    check(&HELIOD_BALLISTA, 5);
}

/// Gravecrawler + Phyrexian Altar + Blood Artist with another Zombie.
const GRAVECRAWLER_ALTAR: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Phyrexian Altar", "Blood Artist", "Diregraf Ghoul", "Gravecrawler"]);
        t.pilot.color = Some(Color::Black);
        t.pilot.target_rules = vec![("Blood Artist", "@bob")];
    },
    lap: |t| {
        t.pilot.next_objects.push_back(vec!["Gravecrawler"]);
        t.tap_for_mana("Phyrexian Altar");
        t.resolve_all(); // Blood Artist drains
        t.cast("Gravecrawler");
        t.resolve_all();
    },
    // The Gravecrawler that was already in play is not summoning sick.
    manual_laps: 2,
    per_lap: &[("life", Some(0), 1), ("life", Some(1), -1)],
};

#[test]
fn gravecrawler_phyrexian_altar_drain_loop() {
    check(&GRAVECRAWLER_ALTAR, 5);
}

// ---------------------------------------------------------------------------
// Guard rails
// ---------------------------------------------------------------------------

#[test]
fn a_shortcut_stops_when_the_resource_runs_out() {
    // Treasonous Ogre: "Pay 3 life: Add {R}." Six activations are payable.
    let build = |shortcuts| {
        let mut t = Table::new(shortcuts);
        t.battlefield(&["Treasonous Ogre"]);
        t
    };
    let mut table = build(true);
    table.tap_for_mana("Treasonous Ogre");
    assert!(table.offered(), "a repeatable action is a one-step loop");
    assert_eq!(
        table.per_lap(),
        vec![("life".to_string(), Some(0), -3), ("red mana".to_string(), Some(0), 1)]
    );
    table.shortcut(50);
    let report = table.game.last_loop_shortcut_report().unwrap().clone();
    assert_eq!((report.requested, report.accepted), (50, 50));
    assert!(matches!(report.stop, LoopShortcutStop::Diverged { .. }), "{report:?}");
    assert!(!table.offered(), "a diverged loop is no longer offered");

    let mut manual = build(false);
    for _ in 0..1 + report.executed {
        manual.tap_for_mana("Treasonous Ogre");
    }
    assert_same(&table, &manual, "after the life ran out");
    assert_eq!(table.game.player(ALICE).unwrap().life, 20 - 3 * (1 + report.executed as i32));
    assert!(table.game.player(ALICE).unwrap().life < 3 + 3, "it stopped only when it had to");
    assert!(table.game.player(ALICE).unwrap().life >= 0);
}

// ---------------------------------------------------------------------------
// Draw loops: offered, with the draw named up front
// ---------------------------------------------------------------------------

/// Yawgmoth, Thran Physician with two undying creatures: sacrifice the one
/// without a counter, putting the -1/-1 counter on the other so that its
/// +1/+1 counter is cancelled. Two activations return the board to where it
/// was, two cards and two life later.
const YAWGMOTH_UNDYING: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Yawgmoth, Thran Physician", "Young Wolf", "Butcher Ghoul"]);
        for _ in 0..30 {
            t.put("Island", ALICE, Zone::Library);
        }
        // Give the Wolf its +1/+1 counter: sacrifice it with no target.
        t.pilot.target_rules = vec![];
        t.pilot.next_objects.push_back(vec!["Young Wolf"]);
        t.activate("Yawgmoth, Thran Physician");
        t.resolve_all();
    },
    lap: |t| {
        t.pilot.target_rules = vec![("Yawgmoth", "Young Wolf")];
        t.pilot.next_objects.push_back(vec!["Butcher Ghoul"]);
        t.activate("Yawgmoth, Thran Physician");
        t.resolve_all();
        t.pilot.target_rules = vec![("Yawgmoth", "Butcher Ghoul")];
        t.pilot.next_objects.push_back(vec!["Young Wolf"]);
        t.activate("Yawgmoth, Thran Physician");
        t.resolve_all();
    },
    // The Ghoul that started in play is not summoning sick until it returns.
    manual_laps: 2,
    per_lap: &[
        ("cards in hand", Some(0), 2),
        ("cards in library", Some(0), -2),
        ("life", Some(0), -2),
    ],
};

/// Yawgmoth with one undying creature and Bridge from Below in the graveyard:
/// sacrifice the Ghoul with no target (it returns with a counter and the
/// Bridge makes a Zombie), then sacrifice the Zombie to take the counter off.
const YAWGMOTH_BRIDGE: Combo = Combo {
    setup: |t| {
        t.battlefield(&["Yawgmoth, Thran Physician", "Butcher Ghoul"]);
        t.put("Bridge from Below", ALICE, Zone::Graveyard);
        for _ in 0..30 {
            t.put("Island", ALICE, Zone::Library);
        }
    },
    lap: |t| {
        t.pilot.target_rules = vec![];
        t.activate("Yawgmoth, Thran Physician");
        t.resolve_all();
        t.pilot.target_rules = vec![("Yawgmoth", "Butcher Ghoul")];
        t.pilot.next_objects.push_back(vec!["Zombie Token"]);
        t.activate("Yawgmoth, Thran Physician");
        t.resolve_all();
    },
    // The Ghoul that started in play is not summoning sick until it returns.
    manual_laps: 2,
    per_lap: &[
        ("cards in hand", Some(0), 2),
        ("cards in library", Some(0), -2),
        ("life", Some(0), -2),
    ],
};

#[test]
fn yawgmoth_butcher_ghoul_bridge_from_below_draw_loop() {
    check(&YAWGMOTH_BRIDGE, 3);
}

#[test]
fn yawgmoth_undying_draw_loop() {
    check(&YAWGMOTH_UNDYING, 3);
}

#[test]
fn a_draw_loop_is_offered_and_says_that_it_draws() {
    // Greed: "{B}, Pay 2 life: Draw a card."
    let build = |shortcuts| {
        let mut t = Table::new(shortcuts);
        t.battlefield(&["Greed"]);
        for _ in 0..10 {
            t.put("Island", ALICE, Zone::Library);
        }
        t.mana(ManaSymbol::Black, 5);
        t
    };
    let mut table = build(true);
    table.activate("Greed");
    table.resolve_all();
    assert!(table.offered(), "a draw loop is offered");
    let offer = table.game.loop_shortcut_offer().unwrap().clone();
    assert!(offer.draws_cards, "the offer says it draws");
    assert_eq!(offer.max_laps, Some(9), "nine cards are left in the library");
    assert_eq!(
        table.per_lap(),
        vec![
            ("black mana".to_string(), Some(0), -1),
            ("cards in hand".to_string(), Some(0), 1),
            ("cards in library".to_string(), Some(0), -1),
            ("life".to_string(), Some(0), -2),
        ]
    );
    table.shortcut(3);
    let report = table.game.last_loop_shortcut_report().unwrap().clone();
    assert_eq!((report.executed, &report.stop), (3, &LoopShortcutStop::Completed));
    let mut manual = build(false);
    for _ in 0..4 {
        manual.activate("Greed");
        manual.resolve_all();
    }
    assert_same(&table, &manual, "after three shortcut draws");
    assert_eq!(table.game.player(ALICE).unwrap().hand.len(), 4);
}

#[test]
fn a_draw_loop_never_empties_the_library() {
    // Plenty of mana and life, but only three cards after the first draw.
    let mut table = Table::new(true);
    table.battlefield(&["Greed"]);
    for _ in 0..4 {
        table.put("Island", ALICE, Zone::Library);
    }
    table.mana(ManaSymbol::Black, 9);
    table.activate("Greed");
    table.resolve_all();
    assert!(table.offered());
    assert_eq!(table.game.loop_shortcut_offer().unwrap().max_laps, Some(3));
    table.shortcut(50);
    let report = table.game.last_loop_shortcut_report().unwrap().clone();
    assert_eq!((report.requested, report.executed), (3, 3), "{report:?}");
    assert_eq!(table.game.player(ALICE).unwrap().library.len(), 0);
    assert!(!table.game_over(), "the last card is drawn, never one more");
    assert!(!table.offered(), "no repetition is left to offer");
}

#[test]
fn an_ordinary_action_is_not_a_loop() {
    let mut table = Table::new(true);
    table.battlefield(&["Forest", "Forest", "Sol Ring"]);
    table.tap_for_mana("Forest");
    assert!(!table.offered());
    table.tap_for_mana("Forest");
    assert!(!table.offered());
    table.tap_for_mana("Sol Ring");
    assert!(!table.offered());
}

#[test]
fn declining_changes_nothing() {
    let mut table = Table::new(true);
    (BASALT_RINGS.setup)(&mut table);
    (BASALT_RINGS.lap)(&mut table);
    assert!(table.offered());
    let before = table.digest();
    table.shortcut(0);
    let report = table.game.last_loop_shortcut_report().unwrap().clone();
    assert_eq!((report.executed, &report.stop), (0, &LoopShortcutStop::Declined));
    assert_eq!(table.digest(), before);
}

#[test]
fn another_player_can_shorten_a_loop_that_gives_them_priority() {
    // CR 732.2b: Bob receives priority during each Basalt Monolith lap.
    let mut table = Table::new(true);
    (BASALT_RINGS.setup)(&mut table);
    (BASALT_RINGS.lap)(&mut table);
    assert!(table.offered());
    assert_eq!(table.game.loop_shortcut_offer().unwrap().responders, vec![BOB]);
    table.pilot.bob_interrupts_after = Some(2);
    table.shortcut(10);
    let report = table.game.last_loop_shortcut_report().unwrap().clone();
    assert_eq!((report.requested, report.accepted, report.executed), (10, 2, 2));
    assert_eq!(report.stop, LoopShortcutStop::Shortened { by: BOB });
    assert!(!table.offered(), "the next lap is played out so Bob can respond");

    let mut manual = Table::new(false);
    (BASALT_RINGS.setup)(&mut manual);
    for _ in 0..3 {
        (BASALT_RINGS.lap)(&mut manual);
    }
    assert_same(&table, &manual, "after Bob shortened the loop");
}

#[test]
fn a_pure_mana_ability_loop_asks_no_other_player() {
    let mut table = Table::new(true);
    (PILI_PALA.setup)(&mut table);
    table.resolve_all();
    (PILI_PALA.lap)(&mut table);
    assert!(table.offered());
    assert!(table.game.loop_shortcut_offer().unwrap().responders.is_empty());
}

#[test]
fn a_drain_loop_stops_when_the_game_ends() {
    let mut table = Table::new(true);
    (HELIOD_BALLISTA.setup)(&mut table);
    table.resolve_all();
    (HELIOD_BALLISTA.lap)(&mut table);
    assert!(table.offered());
    table.shortcut(100);
    let report = table.game.last_loop_shortcut_report().unwrap().clone();
    assert_eq!(report.stop, LoopShortcutStop::GameOver, "{report:?}");
    assert_eq!(report.executed, 19, "Bob had 19 life left");
    assert!(table.game_over());
    assert!(table.game.player(BOB).unwrap().life <= 0);
}
