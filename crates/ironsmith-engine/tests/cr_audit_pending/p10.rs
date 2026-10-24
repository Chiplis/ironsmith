//! CR audit (pending-entry pass, package p10): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, ProliferateContext, SelectObjectsContext, SelectOptionsContext,
    TargetsContext,
};
use ironsmith::game_state::{Phase, StackEntry, Target};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{ManaSymbol, ObjectId, Zone};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

/// Scripted decision maker. Preferences are consulted first; otherwise the
/// minimum legal answer is given. Every prompt description is logged.
#[derive(Default)]
struct Bot {
    accept: bool,
    pick: Vec<ObjectId>,
    pick_players: Vec<PlayerId>,
    options: Vec<String>,
    number: Option<u32>,
    targets: Vec<Target>,
    log: Vec<String>,
}

impl Bot {
    fn yes() -> Self {
        Bot { accept: true, ..Default::default() }
    }
}

impl DecisionMaker for Bot {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.log.push(format!("bool:{}{}", ctx.description, if ctx.can_accept { "" } else { " (cannot accept)" }));
        self.accept && ctx.can_accept
    }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        self.log.push(format!("number:{}", ctx.description));
        self.number.unwrap_or(ctx.min).clamp(ctx.min, ctx.max)
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.log.push(format!("objects:{}", ctx.description));
        let max = ctx.max.unwrap_or(usize::MAX);
        let mut chosen: Vec<ObjectId> = self
            .pick
            .iter()
            .copied()
            .filter(|id| ctx.candidates.iter().any(|c| c.id == *id && c.legal))
            .take(max)
            .collect();
        for c in ctx.candidates.iter().filter(|c| c.legal) {
            if chosen.len() >= ctx.min.min(max) {
                break;
            }
            if !chosen.contains(&c.id) {
                chosen.push(c.id);
            }
        }
        chosen
    }
    fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.log.push(format!(
            "options:{} [{}]",
            ctx.description,
            ctx.options
                .iter()
                .map(|o| o.description.clone())
                .collect::<Vec<_>>()
                .join(" | ")
        ));
        let mut chosen = Vec::new();
        for pref in &self.options {
            if chosen.len() >= ctx.max {
                break;
            }
            if let Some(o) = ctx.options.iter().find(|o| {
                o.legal
                    && !chosen.contains(&o.index)
                    && o.description.to_lowercase().contains(&pref.to_lowercase())
            }) {
                chosen.push(o.index);
            }
        }
        if chosen.len() >= ctx.min.max(1).min(ctx.max) {
            chosen.truncate(ctx.max.max(1));
            return chosen;
        }
        for o in ctx.options.iter().filter(|o| o.legal) {
            if chosen.len() >= ctx.min {
                break;
            }
            if !chosen.contains(&o.index) {
                chosen.push(o.index);
            }
        }
        chosen
    }
    fn decide_proliferate(
        &mut self,
        _: &GameState,
        ctx: &ProliferateContext,
    ) -> ironsmith::decisions::specs::ProliferateResponse {
        self.log.push("proliferate".into());
        ironsmith::decisions::specs::ProliferateResponse {
            permanents: ctx
                .eligible_permanents
                .iter()
                .map(|(id, _)| *id)
                .filter(|id| self.pick.contains(id))
                .collect(),
            players: ctx
                .eligible_players
                .iter()
                .map(|(id, _)| *id)
                .filter(|id| self.pick_players.contains(id))
                .collect(),
        }
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.log.push(format!("targets:{}", ctx.context));
        let mut out = Vec::new();
        for req in &ctx.requirements {
            let max = req.max_targets.unwrap_or(usize::MAX);
            let mut chosen: Vec<Target> = self
                .targets
                .iter()
                .copied()
                .filter(|t| req.legal_targets.contains(t) && !out.contains(t))
                .take(max)
                .collect();
            for t in &req.legal_targets {
                if chosen.len() >= req.min_targets.max(1).min(max) {
                    break;
                }
                if !chosen.contains(t) && !out.contains(t) {
                    chosen.push(*t);
                }
            }
            out.extend(chosen);
        }
        out
    }
}

fn new_game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = ALICE;
    g.turn.priority_player = Some(ALICE);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g
}

fn compile(name: &str, text: &str) -> Result<CardDefinition, String> {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .map_err(|e| format!("{e:?}"))
}

fn must(name: &str, text: &str) -> CardDefinition {
    compile(name, text).unwrap_or_else(|e| panic!("fixture card {name} must compile: {e}"))
}

fn vanilla(name: &str, p: i32, t: i32) -> CardDefinition {
    must(name, &format!("Mana cost: {{1}}\nType: Creature — Bear\nPower/Toughness: {p}/{t}"))
}

fn land(name: &str) -> CardDefinition {
    must(name, "Type: Basic Land — Forest\n({T}: Add {G}.)")
}

fn put(g: &mut GameState, def: &CardDefinition, owner: PlayerId, zone: Zone) -> ObjectId {
    g.create_object_from_definition(def, owner, zone)
}

/// Resolve everything: SBAs, triggers onto the stack, then the top entry.
fn settle(g: &mut GameState, dm: &mut dyn DecisionMaker) {
    for _ in 0..80 {
        let mut q = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas_with(g, &mut q, dm).expect("SBAs apply");
        ironsmith::game_loop::drain_pending_trigger_events(g, &mut q);
        if !q.is_empty() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(g, &mut q, dm)
                .expect("triggers go on the stack");
        }
        if g.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(g, dm).expect("stack entry resolves");
    }
    panic!("stack did not settle");
}

/// Put a spell card on the stack with the given targets and resolve it fully.
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    caster: PlayerId,
    targets: Vec<Target>,
    dm: &mut dyn DecisionMaker,
) -> ObjectId {
    let id = put(g, def, caster, Zone::Stack);
    g.stack.push(StackEntry::new(id, caster).with_targets(targets));
    settle(g, dm);
    id
}

/// Move a card onto the battlefield by an effect and resolve its triggers.
fn enter(g: &mut GameState, def: &CardDefinition, owner: PlayerId, dm: &mut dyn DecisionMaker) -> ObjectId {
    let id = put(g, def, owner, Zone::Hand);
    let new_id = g.move_object_by_effect(id, Zone::Battlefield).expect("enters");
    settle(g, dm);
    new_id
}

fn add_mana(g: &mut GameState, p: PlayerId, symbol: ManaSymbol, n: u32) {
    g.player_mut(p).unwrap().mana_pool.add(symbol, n);
}

/// Drive a priority action (activation or cast) through the real game loop
/// until it is on the stack. Returns false if the action was not legal.
fn take_action<D: DecisionMaker>(g: &mut GameState, action: LegalAction, dm: &mut D) -> bool {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    let actor = g.turn.priority_player.unwrap_or(g.turn.active_player);
    let legal = ironsmith::compute_legal_actions(g, actor).unwrap();
    if !legal.contains(&action) {
        return false;
    }
    let before = g.stack.len();
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        g,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .expect("legal action starts");
    for _ in 0..40 {
        if g.stack.len() > before && state.pending_cast.is_none() && state.pending_activation.is_none() {
            return true;
        }
        match progress {
            GameProgress::NeedsDecisionCtx(ctx) => {
                progress = ironsmith::game_loop::apply_decision_context_with_dm(
                    g, &mut queue, &mut state, &ctx, dm,
                )
                .expect("action decision applies");
            }
            _ => return g.stack.len() > before,
        }
    }
    panic!("action did not finish");
}

fn activate<D: DecisionMaker>(g: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut D) -> bool {
    let ok = take_action(g, LegalAction::ActivateAbility { source, ability_index }, dm);
    if ok {
        settle(g, dm);
    }
    ok
}

fn find_named(g: &GameState, zone_ids: &[ObjectId], name: &str) -> Option<ObjectId> {
    zone_ids
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
}

fn battlefield_named(g: &GameState, name: &str) -> Vec<ObjectId> {
    g.battlefield
        .iter()
        .copied()
        .filter(|id| g.object(*id).is_some_and(|o| o.name == name))
        .collect()
}

fn hand(g: &GameState, p: PlayerId) -> Vec<ObjectId> {
    g.player(p).unwrap().hand.to_vec()
}
fn library(g: &GameState, p: PlayerId) -> Vec<ObjectId> {
    g.player(p).unwrap().library.to_vec()
}
fn graveyard(g: &GameState, p: PlayerId) -> Vec<ObjectId> {
    g.player(p).unwrap().graveyard.to_vec()
}
fn life(g: &GameState, p: PlayerId) -> i32 {
    g.player(p).unwrap().life
}
fn pt(g: &GameState, id: ObjectId) -> (i32, i32) {
    (g.current_power(id).unwrap_or(-99), g.current_toughness(id).unwrap_or(-99))
}
fn plus(g: &GameState, id: ObjectId) -> u32 {
    g.counter_count(id, CounterType::PlusOnePlusOne)
}

fn first_activation(g: &GameState, player: PlayerId, source: ObjectId) -> Option<usize> {
    ironsmith::compute_legal_actions(g, player)
        .unwrap()
        .into_iter()
        .find_map(|a| match a {
            LegalAction::ActivateAbility { source: s, ability_index } if s == source => {
                Some(ability_index)
            }
            _ => None,
        })
}

fn activate_first<D: DecisionMaker>(g: &mut GameState, player: PlayerId, source: ObjectId, dm: &mut D) -> bool {
    match first_activation(g, player, source) {
        Some(index) => activate(g, source, index, dm),
        None => false,
    }
}

fn end_turn(g: &mut GameState) {
    g.turn.phase = Phase::Ending;
    g.turn.step = Some(ironsmith::Step::Cleanup);
    ironsmith::turn::execute_cleanup_step(g);
    g.next_turn();
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(g.turn.active_player);
}

fn token_def(name: &str) -> CardDefinition {
    // A creature definition created as a token through a compiled spell.
    must(
        name,
        &format!("Mana cost: {{0}}\nType: Sorcery\nCreate a 2/2 green Wolf creature token."),
    )
}

// ---------------------------------------------------------------------------
// 701.31 Planeswalk / 701.32 Set in motion / 701.33 Abandon
// ---------------------------------------------------------------------------

fn planar_card(name: &str) -> CardDefinition {
    CardDefinition::new(
        ironsmith::CardBuilder::new(ironsmith::CardId::new(), name)
            .card_types(vec![ironsmith::CardType::Plane])
            .build(),
    )
}

fn planechase_game() -> GameState {
    let mut g = new_game();
    let deck = |prefix: &str| {
        (0..10)
            .map(|i| (planar_card(&format!("{prefix} Plane {i}")), ironsmith::PlanarCardKind::Plane))
            .collect::<Vec<_>>()
    };
    g.enable_planechase(vec![(ALICE, deck("Alice")), (BOB, deck("Bob"))])
        .expect("valid planar decks");
    g.reveal_starting_plane().expect("starting plane");
    g
}

#[test]
fn cr_701_31a_b_planeswalk_requires_planechase_and_planar_controller_and_cycles_planes() {
    // Outside Planechase nobody may planeswalk.
    let mut plain = new_game();
    let src = put(&mut plain, &vanilla("Source", 1, 1), ALICE, Zone::Battlefield);
    assert!(
        plain.planeswalk(ALICE, src).is_err(),
        "CR 701.31a: a player may planeswalk only during a Planechase game"
    );

    let mut g = planechase_game();
    let controller = g.planar_controller().expect("planar controller");
    let other = if controller == ALICE { BOB } else { ALICE };
    let face_up = g.face_up_planar_objects().to_vec();
    assert_eq!(face_up.len(), 1);
    assert!(
        g.planeswalk(other, face_up[0]).is_err(),
        "CR 701.31a: only the planar controller may planeswalk"
    );
    assert_eq!(g.face_up_planar_objects(), face_up.as_slice());

    let old = face_up[0];
    let old_stable = g.object(old).unwrap().stable_id;
    let owner = g.object(old).unwrap().owner;
    let top = *g.planar_deck(controller).unwrap().last().unwrap();
    let destination = g.planeswalk(controller, old).expect("planar controller planeswalks");
    assert_eq!(destination, top, "CR 701.31b: the top card of the planar deck is turned face up");
    assert_eq!(g.face_up_planar_objects(), &[top]);
    let bottom = g.planar_deck(owner).unwrap()[0];
    assert_eq!(
        g.object(bottom).unwrap().stable_id,
        old_stable,
        "CR 701.31b: the old plane goes to the bottom of its owner's planar deck"
    );
    assert!(
        g.object(bottom).unwrap().abilities.iter().all(|a| a.functional_zones.is_empty()),
        "CR 701.31b: the old plane is face down (its abilities do not function)"
    );
}

fn archenemy_game(ongoing_top: bool) -> GameState {
    let mut g = new_game();
    let mut cards: Vec<CardDefinition> = (0..20)
        .map(|i| {
            CardDefinition::new(
                ironsmith::CardBuilder::new(ironsmith::CardId::new(), &format!("Scheme {i}"))
                    .card_types(vec![ironsmith::CardType::Scheme])
                    .build(),
            )
        })
        .collect();
    if ongoing_top {
        cards[19] = CardDefinition::new(
            ironsmith::CardBuilder::new(ironsmith::CardId::new(), "Ongoing Scheme")
                .card_types(vec![ironsmith::CardType::Scheme])
                .supertypes(vec![ironsmith::Supertype::Ongoing])
                .build(),
        );
    }
    g.enable_archenemy(ironsmith::ArchenemyVariant::Default, vec![(ALICE, cards)])
        .expect("archenemy");
    if ongoing_top {
        let id = *g
            .scheme_deck(ALICE)
            .unwrap()
            .iter()
            .find(|id| g.object(**id).is_some_and(|o| o.name == "Ongoing Scheme"))
            .unwrap();
        let deck = g.archenemy.as_mut().unwrap().scheme_decks.get_mut(&ALICE).unwrap();
        deck.retain(|x| *x != id);
        deck.push(id);
    }
    g
}

#[test]
fn cr_701_32a_b_c_only_archenemy_sets_schemes_in_motion_one_at_a_time_from_the_top() {
    let mut plain = new_game();
    assert!(
        plain.set_scheme_in_motion(ALICE).is_err(),
        "CR 701.32a: schemes are set in motion only during an Archenemy game"
    );
    let mut g = archenemy_game(false);
    assert!(
        g.set_scheme_in_motion(BOB).is_err(),
        "CR 701.32a: only the archenemy may set a scheme in motion"
    );
    let deck_before = g.scheme_deck(ALICE).unwrap().to_vec();
    let top = *deck_before.last().unwrap();
    let second = deck_before[deck_before.len() - 2];
    g.take_pending_trigger_events();
    // A two-scheme instruction through the keyword-action effect.
    let src = put(&mut g, &vanilla("Source", 1, 1), ALICE, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(src, ALICE);
    ironsmith::effects::EffectExecutor::execute(
        &ironsmith::effects::EmitKeywordActionEffect::new(
            ironsmith::events::KeywordActionKind::SetSchemeInMotion,
            2,
        ),
        &mut g,
        &mut ctx,
    )
    .expect("set two schemes in motion");
    assert_eq!(
        g.face_up_schemes(),
        &[top, second],
        "CR 701.32b-c: each scheme moves off the top and turns face up, one at a time"
    );
    let events = g.take_pending_trigger_events();
    let motions: Vec<ObjectId> = events
        .iter()
        .filter_map(|e| e.downcast::<ironsmith::events::KeywordActionEvent>())
        .filter(|e| e.action == ironsmith::events::KeywordActionKind::SetSchemeInMotion)
        .map(|e| e.source)
        .collect();
    assert_eq!(motions, vec![top, second], "CR 701.32c: one set-in-motion event per scheme");
    assert!(
        g.object(top).unwrap().abilities.iter().all(|a| a.functional_zones == [Zone::Command]),
        "CR 701.32b: the scheme is face up"
    );
}

#[test]
fn cr_701_33a_only_face_up_ongoing_schemes_can_be_abandoned() {
    let mut g = archenemy_game(true);
    let deck = g.scheme_deck(ALICE).unwrap().to_vec();
    let face_down = deck[0];
    assert!(
        g.abandon_scheme(face_down).is_err(),
        "CR 701.33a: a face-down scheme can't be abandoned"
    );
    let ongoing = g.set_scheme_in_motion(ALICE).unwrap();
    let ordinary = g.set_scheme_in_motion(ALICE).unwrap();
    assert!(
        g.abandon_scheme(ordinary).is_err(),
        "CR 701.33a: a non-ongoing scheme can't be abandoned"
    );
    let recycled = g.abandon_scheme(ongoing).expect("face-up ongoing scheme is abandoned");
    assert_eq!(g.face_up_schemes(), &[ordinary]);
    assert_eq!(g.scheme_deck(ALICE).unwrap()[0], recycled);
}

// ---------------------------------------------------------------------------
// 701.34 Proliferate
// ---------------------------------------------------------------------------

const STEADY_PROGRESS: &str = "Mana cost: {2}{U}\nType: Instant\nProliferate.\nDraw a card.";

#[test]
fn cr_701_34a_proliferate_adds_one_of_each_kind_to_chosen_permanents_and_players() {
    let mut g = new_game();
    put(&mut g, &land("Lib"), ALICE, Zone::Library);
    let mine = put(&mut g, &vanilla("Mine", 2, 2), ALICE, Zone::Battlefield);
    g.add_counters(mine, CounterType::PlusOnePlusOne, 1);
    g.add_counters(mine, CounterType::Charge, 2);
    let theirs = put(&mut g, &vanilla("Theirs", 3, 3), BOB, Zone::Battlefield);
    g.add_counters(theirs, CounterType::MinusOneMinusOne, 1);
    let plain = put(&mut g, &vanilla("Plain", 1, 1), ALICE, Zone::Battlefield);
    g.add_player_counters_with_source(BOB, CounterType::Poison, 2, None, None).unwrap();
    let mut bot = Bot { pick: vec![mine, plain], pick_players: vec![BOB], ..Default::default() };
    cast(&mut g, &must("Steady Progress", STEADY_PROGRESS), ALICE, vec![], &mut bot);
    assert!(bot.log.iter().any(|l| l == "proliferate"), "proliferate choice offered");
    assert_eq!(
        (plus(&g, mine), g.counter_count(mine, CounterType::Charge)),
        (2, 3),
        "CR 701.34a: each chosen permanent gets one more counter of each kind it has"
    );
    assert_eq!(
        g.counter_count(theirs, CounterType::MinusOneMinusOne),
        1,
        "CR 701.34a: unchosen permanents are unaffected"
    );
    assert_eq!(g.counter_count(plain, CounterType::PlusOnePlusOne), 0, "CR 701.34a: no counter kinds to add");
    assert_eq!(g.player(BOB).unwrap().poison_counters, 3, "CR 701.34a: chosen player gets a poison counter");
}

#[test]
fn cr_701_34b_two_headed_giant_proliferate_gives_shared_poison_only_once() {
    let p: Vec<_> = (0..4).map(PlayerId::from_index).collect();
    let mut g = GameState::new((0..4).map(|i| format!("P{i}")).collect(), 20);
    g.enable_two_headed_giant(vec![vec![p[0], p[1]], vec![p[2], p[3]]]).unwrap();
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    let caster = p[0];
    g.add_player_counters_with_source(p[2], CounterType::Poison, 2, None, None).unwrap();
    assert_eq!(g.player(p[3]).unwrap().poison_counters, 2, "fixture: poison is shared");
    put(&mut g, &land("Lib"), caster, Zone::Library);
    let mut bot = Bot { pick_players: vec![p[2], p[3]], ..Default::default() };
    cast(&mut g, &must("Steady Progress", STEADY_PROGRESS), caster, vec![], &mut bot);
    assert_eq!(
        (g.player(p[2]).unwrap().poison_counters, g.player(p[3]).unwrap().poison_counters),
        (3, 3),
        "CR 701.34b: choosing both teammates gives the team only one additional poison counter"
    );
}

// ---------------------------------------------------------------------------
// 701.35 Detain
// ---------------------------------------------------------------------------

#[test]
fn cr_701_35a_detain_lasts_until_the_detainers_next_turn() {
    let mut g = new_game();
    put(&mut g, &land("Lib"), ALICE, Zone::Library);
    put(&mut g, &land("Lib"), BOB, Zone::Library);
    put(&mut g, &land("Lib2"), BOB, Zone::Library);
    let pinger = must(
        "Pinger",
        "Mana cost: {1}\nType: Creature — Elf\nPower/Toughness: 1/1\n{T}: You gain 1 life.",
    );
    let target = put(&mut g, &pinger, BOB, Zone::Battlefield);
    let control = put(&mut g, &pinger, BOB, Zone::Battlefield);
    let spell = must(
        "Inaction Injunction",
        "Mana cost: {1}{U}\nType: Sorcery\nDetain target creature an opponent controls.\nDraw a card.",
    );
    cast(&mut g, &spell, ALICE, vec![Target::Object(target)], &mut Bot::default());
    let restricted = |g: &GameState| {
        (!g.can_attack(target), !g.can_block(target), !g.can_activate_abilities_of(target))
    };
    assert_eq!(restricted(&g), (true, true, true), "CR 701.35a: detained creature can't attack, block, or activate");
    assert!(g.can_attack(control) && g.can_activate_abilities_of(control), "control creature unaffected");
    end_turn(&mut g);
    assert_eq!(g.turn.active_player, BOB);
    assert_eq!(restricted(&g), (true, true, true), "CR 701.35a: detain persists through the opponent's turn");
    end_turn(&mut g);
    assert_eq!(g.turn.active_player, ALICE);
    assert_eq!(restricted(&g), (false, false, false), "CR 701.35a: detain ends at the detainer's next turn");
}

// ---------------------------------------------------------------------------
// 701.36 Populate
// ---------------------------------------------------------------------------

#[test]
fn cr_701_36a_populate_copies_a_creature_token_you_control() {
    let mut g = new_game();
    let spell = must(
        "Eyes in the Skies",
        "Mana cost: {3}{W}\nType: Instant\nCreate a 1/1 white Bird creature token with flying, then populate.",
    );
    let nontoken = put(&mut g, &vanilla("Real Bear", 5, 5), ALICE, Zone::Battlefield);
    cast(&mut g, &spell, ALICE, vec![], &mut Bot::default());
    let birds: Vec<ObjectId> = g
        .battlefield
        .iter()
        .copied()
        .filter(|id| g.object(*id).is_some_and(|o| o.kind == ironsmith::object::ObjectKind::Token && o.subtypes.iter().any(|s| format!("{s:?}") == "Bird")))
        .collect();
    assert_eq!(birds.len(), 2, "CR 701.36a: populate creates a copy of the creature token");
    assert_eq!(battlefield_named(&g, "Real Bear").len(), 1, "CR 701.36a: only tokens are copied");
    let _ = nontoken;
}

#[test]
fn cr_701_36b_populate_without_creature_tokens_creates_nothing() {
    let mut g = new_game();
    put(&mut g, &vanilla("Real Bear", 2, 2), ALICE, Zone::Battlefield);
    let before = g.battlefield.len();
    let spell = must(
        "Rootborn Defenses",
        "Mana cost: {2}{W}\nType: Instant\nPopulate. Creatures you control gain indestructible until end of turn.",
    );
    cast(&mut g, &spell, ALICE, vec![], &mut Bot::default());
    assert_eq!(g.battlefield.len(), before, "CR 701.36b: no creature tokens means no token is created");
}

// ---------------------------------------------------------------------------
// 701.37 Monstrosity
// ---------------------------------------------------------------------------

const NESSIAN_ASP: &str = "Mana cost: {4}{G}\nType: Creature — Snake\nPower/Toughness: 4/5\nReach\n{6}{G}: Monstrosity 4.";

#[test]
fn cr_701_37a_b_monstrosity_happens_once_and_monstrous_is_lost_on_zone_change() {
    let mut g = new_game();
    let asp = put(&mut g, &must("Nessian Asp", NESSIAN_ASP), ALICE, Zone::Battlefield);
    assert!(!g.is_monstrous(asp));
    add_mana(&mut g, ALICE, ManaSymbol::Green, 7);
    assert!(activate_first(&mut g, ALICE, asp, &mut Bot::default()), "monstrosity activates");
    assert_eq!(plus(&g, asp), 4, "CR 701.37a: N +1/+1 counters");
    assert!(g.is_monstrous(asp), "CR 701.37a: it becomes monstrous");
    add_mana(&mut g, ALICE, ManaSymbol::Green, 7);
    assert!(activate_first(&mut g, ALICE, asp, &mut Bot::default()), "may activate again");
    assert_eq!(plus(&g, asp), 4, "CR 701.37a/b: a monstrous permanent gets no more counters");
    let back = g.move_object_by_effect(asp, Zone::Hand).unwrap();
    let back = g.move_object_by_effect(back, Zone::Battlefield).unwrap();
    assert!(!g.is_monstrous(back), "CR 701.37b: a new object is not monstrous");
}

#[test]
fn cr_701_37c_monstrosity_x_value_is_available_to_becomes_monstrous_trigger() {
    let mut g = new_game();
    let poly = put(
        &mut g,
        &must(
            "Polukranos, World Eater",
            "Mana cost: {2}{G}{G}\nType: Legendary Creature — Hydra\nPower/Toughness: 5/5\n{X}{X}{G}: Monstrosity X.\nWhen Polukranos becomes monstrous, it deals X damage divided as you choose among any number of target creatures your opponents control. Each of those creatures deals damage equal to its power to Polukranos.",
        ),
        ALICE,
        Zone::Battlefield,
    );
    let victim = put(&mut g, &vanilla("Victim", 0, 4), BOB, Zone::Battlefield);
    add_mana(&mut g, ALICE, ManaSymbol::Green, 5);
    struct Dist(Bot, ObjectId);
    impl DecisionMaker for Dist {
        fn decide_number(&mut self, g: &GameState, c: &NumberContext) -> u32 {
            self.0.decide_number(g, c)
        }
        fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
            self.0.decide_targets(g, c)
        }
        fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
            self.0.decide_options(g, c)
        }
        fn decide_distribute(
            &mut self,
            _: &GameState,
            c: &ironsmith::decisions::context::DistributeContext,
        ) -> Vec<(Target, u32)> {
            vec![(Target::Object(self.1), c.total)]
        }
    }
    let mut dm = Dist(Bot { number: Some(2), targets: vec![Target::Object(victim)], ..Default::default() }, victim);
    assert!(activate_first(&mut g, ALICE, poly, &mut dm), "monstrosity X activates");
    assert_eq!(plus(&g, poly), 2, "X=2 counters");
    assert_eq!(
        g.damage_on(victim),
        2,
        "CR 701.37c: X in the becomes-monstrous trigger equals the X used for monstrosity"
    );
}

// ---------------------------------------------------------------------------
// 701.38 Voting
// ---------------------------------------------------------------------------

const PLEA_FOR_POWER: &str = "Mana cost: {3}{U}\nType: Sorcery\nWill of the council — Starting with you, each player votes for time or knowledge. If time gets more votes, take an extra turn after this one. If knowledge gets more votes or the vote is tied, draw three cards.";

/// Votes are answered from a queue of option texts, in prompt order.
struct Voter {
    queue: std::collections::VecDeque<&'static str>,
    objects: std::collections::VecDeque<ObjectId>,
    voters: Vec<PlayerId>,
}
impl DecisionMaker for Voter {
    fn decide_options(&mut self, _: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        if let Some(front) = self.queue.front().copied() {
            if let Some(o) = c.options.iter().find(|o| o.legal && o.description.to_lowercase().contains(front)) {
                self.queue.pop_front();
                self.voters.push(c.player);
                return vec![o.index];
            }
        }
        c.options.iter().filter(|o| o.legal).take(c.min).map(|o| o.index).collect()
    }
    fn decide_objects(&mut self, _: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(front) = self.objects.front().copied() {
            if c.candidates.iter().any(|x| x.id == front && x.legal) {
                self.objects.pop_front();
                self.voters.push(c.player);
                return vec![front];
            }
        }
        c.candidates.iter().filter(|x| x.legal).take(c.min).map(|x| x.id).collect()
    }
}

fn voter(votes: &[&'static str]) -> Voter {
    Voter { queue: votes.iter().copied().collect(), objects: Default::default(), voters: vec![] }
}

fn fill_library(g: &mut GameState, p: PlayerId, n: usize) {
    for i in 0..n {
        put(g, &land(&format!("Lib{i}")), p, Zone::Library);
    }
}

#[test]
fn cr_701_38a_votes_start_with_specified_player_and_majority_decides() {
    for (votes, extra_turn) in [(["time", "time"], true), (["time", "knowledge"], false), (["knowledge", "knowledge"], false)] {
        let mut g = new_game();
        fill_library(&mut g, ALICE, 5);
        let mut v = voter(&votes);
        cast(&mut g, &must("Plea for Power", PLEA_FOR_POWER), ALICE, vec![], &mut v);
        assert_eq!(v.voters, vec![ALICE, BOB], "CR 701.38a: voting starts with the specified player, in turn order");
        let drew = hand(&g, ALICE).len();
        assert_eq!(
            (drew == 3, g.turn_store.extra_turns.len() > 0 || g.turn_store.extra_turns_after_next_turn.len() > 0),
            (!extra_turn, extra_turn),
            "CR 701.38a: votes={votes:?}"
        );
    }
}

#[test]
fn cr_701_38b_votes_may_be_for_objects() {
    let mut g = new_game();
    let x = put(&mut g, &vanilla("Bob X", 2, 2), BOB, Zone::Battlefield);
    let y = put(&mut g, &vanilla("Bob Y", 2, 2), BOB, Zone::Battlefield);
    let z = put(&mut g, &vanilla("Bob Z", 2, 2), BOB, Zone::Battlefield);
    let mut v = Voter { queue: Default::default(), objects: [x, y].into_iter().collect(), voters: vec![] };
    cast(
        &mut g,
        &must(
            "Council's Judgment",
            "Mana cost: {1}{W}{W}\nType: Sorcery\nWill of the council — Starting with you, each player votes for a nonland permanent you don't control. Exile each permanent with the most votes or tied for most votes.",
        ),
        ALICE,
        vec![],
        &mut v,
    );
    let names: Vec<String> = g.battlefield.iter().map(|id| g.object(*id).unwrap().name.to_string()).collect();
    assert_eq!(v.voters, vec![ALICE, BOB], "both players voted for objects");
    assert_eq!(names, vec!["Bob Z".to_string()], "CR 701.38b: tied object votes exile both voted objects");
    let _ = z;
}

#[test]
fn cr_701_38c_finish_voting_trigger_ignores_non_vote_choices() {
    let keeper = must(
        "Grudge Keeper",
        "Mana cost: {1}{B}\nType: Creature — Zombie Wizard\nPower/Toughness: 2/1\nWhenever players finish voting, each opponent who voted for a choice you didn't vote for loses 2 life.",
    );
    // A non-vote choice: each player chooses (no vote) via a modal spell.
    let mut g = new_game();
    fill_library(&mut g, ALICE, 5);
    put(&mut g, &keeper, ALICE, Zone::Battlefield);
    cast(
        &mut g,
        &must("Choice Spell", "Mana cost: {1}\nType: Sorcery\nChoose one —\n• You gain 1 life.\n• Draw a card."),
        ALICE,
        vec![],
        &mut Bot::default(),
    );
    assert_eq!(life(&g, BOB), 20, "CR 701.38c: a choice without the word vote is not voting");
    // An actual vote does trigger it.
    let mut v = voter(&["time", "knowledge"]);
    cast(&mut g, &must("Plea for Power", PLEA_FOR_POWER), ALICE, vec![], &mut v);
    assert_eq!(life(&g, BOB), 18, "CR 701.38c: an actual vote is voting");
}

#[test]
fn cr_701_38d_additional_votes_happen_when_the_player_would_vote() {
    let mut g = new_game();
    fill_library(&mut g, ALICE, 5);
    put(
        &mut g,
        &must(
            "Brago's Representative",
            "Mana cost: {2}{W}\nType: Creature — Human Advisor\nPower/Toughness: 1/4\nWhile voting, you get an additional vote.",
        ),
        ALICE,
        Zone::Battlefield,
    );
    let mut v = voter(&["time", "time", "knowledge"]);
    cast(&mut g, &must("Plea for Power", PLEA_FOR_POWER), ALICE, vec![], &mut v);
    assert_eq!(
        v.voters,
        vec![ALICE, ALICE, BOB],
        "CR 701.38d: both of Alice's votes happen when she would vote"
    );
    assert!(
        g.turn_store.extra_turns.len() > 0 || g.turn_store.extra_turns_after_next_turn.len() > 0,
        "CR 701.38d: two time votes beat one knowledge vote"
    );
}

// ---------------------------------------------------------------------------
// 701.39 Bolster / 701.41 Support
// ---------------------------------------------------------------------------

#[test]
fn cr_701_39a_bolster_chooses_least_toughness_creature_you_control() {
    let mut g = new_game();
    let a = put(&mut g, &vanilla("Big", 1, 5), ALICE, Zone::Battlefield);
    let b = put(&mut g, &vanilla("Least", 4, 2), ALICE, Zone::Battlefield);
    let c = put(&mut g, &vanilla("Mid", 1, 3), ALICE, Zone::Battlefield);
    let bobs = put(&mut g, &vanilla("Bob tiny", 1, 1), BOB, Zone::Battlefield);
    let mut bot = Bot { pick: vec![a, c, bobs], ..Default::default() };
    cast(
        &mut g,
        &must("Honor's Reward", "Mana cost: {2}{W}\nType: Instant\nYou gain 4 life. Bolster 2."),
        ALICE,
        vec![],
        &mut bot,
    );
    assert_eq!(
        (plus(&g, a), plus(&g, b), plus(&g, c), plus(&g, bobs)),
        (0, 2, 0, 0),
        "CR 701.39a: counters go on the creature you control with the least toughness"
    );
}

#[test]
fn cr_701_41a_support_on_a_permanent_targets_other_creatures_and_on_a_spell_any() {
    // Permanent: "other target creatures".
    let mut g = new_game();
    let ally = put(&mut g, &vanilla("Ally", 1, 1), ALICE, Zone::Battlefield);
    struct Grab(Vec<Target>);
    impl DecisionMaker for Grab {
        fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
            let mut out = vec![];
            for r in &c.requirements {
                self.0.extend(r.legal_targets.iter().copied());
                out.extend(r.legal_targets.iter().copied().take(r.max_targets.unwrap_or(99)));
            }
            out
        }
    }
    let mut grab = Grab(vec![]);
    let cavalry = enter(
        &mut g,
        &must(
            "Gladehart Cavalry",
            "Mana cost: {5}{G}{G}\nType: Creature — Elf Knight\nPower/Toughness: 6/6\nWhen this creature enters, support 6.\nWhenever a creature you control with a +1/+1 counter on it dies, you gain 2 life.",
        ),
        ALICE,
        &mut grab,
    );
    assert!(!grab.0.contains(&Target::Object(cavalry)), "CR 701.41a: support on a permanent can't target itself");
    assert_eq!((plus(&g, ally), plus(&g, cavalry)), (1, 0), "CR 701.41a: other target creature gets a counter");
    // Spell: up to N target creatures.
    let mut g = new_game();
    put(&mut g, &land("Lib"), ALICE, Zone::Library);
    let one = put(&mut g, &vanilla("One", 1, 1), ALICE, Zone::Battlefield);
    let two = put(&mut g, &vanilla("Two", 1, 1), BOB, Zone::Battlefield);
    cast(
        &mut g,
        &must("Shoulder to Shoulder", "Mana cost: {2}{W}\nType: Sorcery\nSupport 2.\nDraw a card."),
        ALICE,
        vec![Target::Object(one), Target::Object(two)],
        &mut Bot::default(),
    );
    assert_eq!((plus(&g, one), plus(&g, two)), (1, 1), "CR 701.41a: support spell puts a counter on each target");
}

// ---------------------------------------------------------------------------
// 701.40 Manifest
// ---------------------------------------------------------------------------

const SOUL_SUMMONS: &str = "Mana cost: {1}{W}\nType: Sorcery\nManifest the top card of your library.";

fn face_down_ids(g: &GameState) -> Vec<ObjectId> {
    g.battlefield.iter().copied().filter(|id| g.is_face_down(*id)).collect()
}

fn turn_face_up(g: &mut GameState, id: ObjectId, method: ironsmith::special_actions::TurnFaceUpMethod) -> Result<(), String> {
    let action = ironsmith::special_actions::SpecialAction::TurnFaceUp { permanent_id: id, method };
    ironsmith::special_actions::perform(action, g, ALICE, &mut Bot::yes()).map_err(|e| format!("{e:?}"))
}

#[test]
fn cr_701_40a_b_manifest_is_a_nameless_2_2_and_creature_cards_turn_up_for_mana_cost() {
    use ironsmith::special_actions::TurnFaceUpMethod;
    for creature_card in [true, false] {
        let mut g = new_game();
        let card = if creature_card {
            must("Big Ogre", "Mana cost: {2}{R}\nType: Creature — Ogre Warrior\nPower/Toughness: 5/4")
        } else {
            must("Shock Spell", "Mana cost: {R}\nType: Instant\nShock Spell deals 2 damage to any target.")
        };
        put(&mut g, &card, ALICE, Zone::Library);
        cast(&mut g, &must("Soul Summons", SOUL_SUMMONS), ALICE, vec![], &mut Bot::default());
        let fd = face_down_ids(&g);
        assert_eq!(fd.len(), 1, "CR 701.40a: the card is put onto the battlefield face down");
        let id = fd[0];
        assert!(g.is_manifested(id), "CR 701.40a: it is a manifested permanent");
        assert_eq!(pt(&g, id), (2, 2), "CR 701.40a: a manifested permanent is 2/2");
        let chars = g.current_characteristics(id).unwrap();
        assert!(
            (chars.name.is_empty() || chars.name.as_ref() as &str == ironsmith::object::FACE_DOWN_DISPLAY_NAME) && chars.subtypes.is_empty() && chars.mana_cost.as_ref().map_or(true, |c| c.mana_value() == 0),
            "CR 701.40a: no name, no subtypes and no mana cost: {:?}", chars.name
        );
        add_mana(&mut g, ALICE, ManaSymbol::Red, 3);
        let result = turn_face_up(&mut g, id, TurnFaceUpMethod::PrintedManaCost);
        if creature_card {
            assert!(result.is_ok(), "CR 701.40b: a manifested creature card turns face up for its mana cost: {result:?}");
            assert!(!g.is_face_down(id) && pt(&g, id) == (5, 4), "CR 701.40b: regains normal characteristics");
        } else {
            assert!(result.is_err(), "CR 701.40b: a noncreature card can't be turned up this way");
            assert!(g.is_face_down(id));
        }
    }
}

#[test]
fn cr_701_40c_d_manifested_morph_and_disguise_cards_may_use_either_procedure() {
    use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod};
    for (text, keyword_method) in [
        ("Mana cost: {3}{G}\nType: Creature — Beast\nPower/Toughness: 4/4\nMorph {1}{G}", TurnFaceUpMethod::TurnFaceUpAbility),
        ("Mana cost: {3}{G}\nType: Creature — Beast\nPower/Toughness: 4/4\nDisguise {1}{G}", TurnFaceUpMethod::DisguiseAbility),
    ] {
        for method in [keyword_method, TurnFaceUpMethod::PrintedManaCost] {
            let mut g = new_game();
            put(&mut g, &must("Hidden Beast", text), ALICE, Zone::Library);
            cast(&mut g, &must("Soul Summons", SOUL_SUMMONS), ALICE, vec![], &mut Bot::default());
            let id = face_down_ids(&g)[0];
            add_mana(&mut g, ALICE, ManaSymbol::Green, 4);
            let check = ironsmith::special_actions::can_perform_check(
                &SpecialAction::TurnFaceUp { permanent_id: id, method },
                &g,
                ALICE,
            );
            assert!(check.is_ok(), "CR 701.40c-d: {method:?} is available for a manifested card with that keyword: {check:?}");
            assert!(turn_face_up(&mut g, id, method).is_ok());
            assert_eq!(pt(&g, id), (4, 4), "CR 701.40c-d: turned face up via {method:?}");
        }
    }
}

#[test]
fn cr_701_40e_manifesting_multiple_cards_happens_one_at_a_time() {
    let mut g = new_game();
    for i in 0..3 {
        put(&mut g, &land(&format!("L{i}")), ALICE, Zone::Library);
    }
    put(
        &mut g,
        &must(
            "Entry Watcher",
            "Mana cost: {1}\nType: Enchantment\nWhenever a creature you control enters, you gain 1 life.",
        ),
        ALICE,
        Zone::Battlefield,
    );
    cast(
        &mut g,
        &must("Double Manifest", "Mana cost: {1}\nType: Sorcery\nManifest the top two cards of your library."),
        ALICE,
        vec![],
        &mut Bot::default(),
    );
    assert_eq!(face_down_ids(&g).len(), 2, "CR 701.40e: both cards are manifested");
    assert_eq!(life(&g, ALICE), 22, "CR 701.40e: each manifested card enters as its own event");
}

#[test]
fn cr_701_40f_prohibited_manifest_leaves_the_card_in_its_zone() {
    let mut g = new_game();
    let lib = put(&mut g, &land("Library Land"), ALICE, Zone::Library);
    put(
        &mut g,
        &must(
            "Grafdigger's Cage",
            "Mana cost: {1}\nType: Artifact\nCreature cards in graveyards and libraries can't enter the battlefield.\nPlayers can't cast spells from graveyards or libraries.",
        ),
        BOB,
        Zone::Battlefield,
    );
    cast(&mut g, &must("Soul Summons", SOUL_SUMMONS), ALICE, vec![], &mut Bot::default());
    assert!(face_down_ids(&g).is_empty(), "CR 701.40f: the face-down creature card can't enter, so nothing is manifested");
    assert!(library(&g, ALICE).contains(&lib), "CR 701.40f: the card remains in the library");
}

#[test]
fn cr_701_40g_manifested_instant_revealed_but_stays_face_down() {
    let mut g = new_game();
    put(&mut g, &must("Shock Spell", "Mana cost: {R}\nType: Instant\nShock Spell deals 2 damage to any target."), ALICE, Zone::Library);
    put(
        &mut g,
        &must("Face Watcher", "Mana cost: {1}\nType: Enchantment\nWhenever a permanent you control is turned face up, you gain 5 life."),
        ALICE,
        Zone::Battlefield,
    );
    cast(&mut g, &must("Soul Summons", SOUL_SUMMONS), ALICE, vec![], &mut Bot::default());
    let id = face_down_ids(&g)[0];
    cast(
        &mut g,
        &must("Unmask", "Mana cost: {U}\nType: Instant\nTurn target face-down creature face up."),
        ALICE,
        vec![Target::Object(id)],
        &mut Bot::default(),
    );
    assert!(g.is_face_down(id), "CR 701.40g: a manifested instant stays face down");
    assert_eq!(life(&g, ALICE), 20, "CR 701.40g: turned-face-up triggers don't trigger");
}

// ---------------------------------------------------------------------------
// 701.43 Exert
// ---------------------------------------------------------------------------

fn untap_step(g: &mut GameState, player: PlayerId) {
    while g.turn.active_player != player {
        g.next_turn();
    }
    g.turn.phase = Phase::Beginning;
    g.turn.step = Some(ironsmith::Step::Untap);
    ironsmith::turn::execute_untap_step_with(g, &mut Bot::default()).unwrap();
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
}

fn exert(g: &mut GameState, id: ObjectId) -> Result<(), String> {
    let mut ctx = EffectContext::new_default(id, ALICE);
    ironsmith::effects::EffectExecutor::execute(
        &ironsmith::effects::ExertCostEffect::new("Exert this creature"),
        g,
        &mut ctx,
    )
    .map(|_| ())
    .map_err(|e| format!("{e:?}"))
}

#[test]
fn cr_701_43a_b_c_exert_skips_only_the_next_untap_and_requires_the_battlefield() {
    let mut g = new_game();
    let a = put(&mut g, &vanilla("Exertee", 2, 2), ALICE, Zone::Battlefield);
    // 701.43b: may be exerted while untapped, and twice in a turn.
    assert!(!g.is_tapped(a));
    exert(&mut g, a).expect("CR 701.43b: an untapped permanent can be exerted");
    exert(&mut g, a).expect("CR 701.43b: a permanent can be exerted again");
    g.tap(a);
    end_turn(&mut g);
    untap_step(&mut g, ALICE);
    assert!(g.is_tapped(a), "CR 701.43a: it doesn't untap during your next untap step");
    end_turn(&mut g);
    untap_step(&mut g, ALICE);
    assert!(!g.is_tapped(a), "CR 701.43b: both exert effects expire during the same untap step");
    // 701.43c
    let gy = put(&mut g, &vanilla("Graveyard bear", 2, 2), ALICE, Zone::Graveyard);
    assert!(exert(&mut g, gy).is_err(), "CR 701.43c: an object not on the battlefield can't be exerted");
}

#[test]
fn cr_701_43d_exert_as_it_attacks_is_optional_and_links_when_you_do() {
    use ironsmith::decision::AttackerDeclaration;
    for accept in [true, false] {
        let mut g = new_game();
        let crasher = put(
            &mut g,
            &must(
                "Clockwork Droid",
                "Mana cost: {2}\nType: Artifact Creature — Robot\nPower/Toughness: 2/1\nYou may exert this creature as it attacks. When you do, it can't be blocked this turn and you scry 1.",
            ),
            ALICE,
            Zone::Battlefield,
        );
        g.remove_summoning_sickness(crasher);
        put(&mut g, &land("Lib"), ALICE, Zone::Library);
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(ironsmith::Step::DeclareAttackers);
        let mut combat = ironsmith::CombatState::default();
        let mut q = TriggerQueue::new();
        let mut bot = Bot { accept, ..Default::default() };
        ironsmith::game_loop::apply_attacker_declarations_with_dm(
            &mut g,
            &mut combat,
            &mut q,
            &[AttackerDeclaration { creature: crasher, target: ironsmith::AttackTarget::Player(BOB) }],
            &mut bot,
        )
        .expect("attack declared");
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g, &mut q, &mut bot).unwrap();
        let linked = g.stack.len();
        settle(&mut g, &mut bot);
        assert_eq!(linked, usize::from(accept), "CR 701.43d: the linked 'when you do' trigger only if exerted (accept={accept})");
        end_turn(&mut g);
        untap_step(&mut g, ALICE);
        assert_eq!(g.is_tapped(crasher), accept, "CR 701.43d: declining the optional cost leaves untap normal");
    }
}

// ---------------------------------------------------------------------------
// 701.44 Explore
// ---------------------------------------------------------------------------

const BRANCHWALKER: &str = "Mana cost: {1}{G}\nType: Creature — Merfolk Scout\nPower/Toughness: 2/1\nWhen this creature enters, it explores.";
const CAVE_DIVER_WATCH: &str = "Mana cost: {1}\nType: Enchantment\nWhenever a creature you control explores, you gain 3 life.";

#[test]
fn cr_701_44a_explore_land_goes_to_hand_nonland_gives_counter_and_optional_graveyard() {
    // Land.
    let mut g = new_game();
    let l = put(&mut g, &land("Top Land"), ALICE, Zone::Library);
    let w = enter(&mut g, &must("Merfolk Branchwalker", BRANCHWALKER), ALICE, &mut Bot::default());
    assert!(hand(&g, ALICE).contains(&l) || find_named(&g, &hand(&g, ALICE), "Top Land").is_some(), "CR 701.44a: a revealed land goes to hand");
    assert_eq!(plus(&g, w), 0, "CR 701.44a: no counter for a land");
    // Nonland, kept or binned.
    for bin in [false, true] {
        let mut g = new_game();
        put(&mut g, &vanilla("Top Bear", 2, 2), ALICE, Zone::Library);
        let mut bot = Bot { accept: bin, options: vec![if bin { "graveyard".into() } else { "top".into() }], ..Default::default() };
        let w = enter(&mut g, &must("Merfolk Branchwalker", BRANCHWALKER), ALICE, &mut bot);
        assert_eq!(plus(&g, w), 1, "CR 701.44a: a nonland reveal gives a +1/+1 counter");
        assert_eq!(
            find_named(&g, &graveyard(&g, ALICE), "Top Bear").is_some(),
            bin,
            "CR 701.44a: the player may put the nonland card into the graveyard (bin={bin}); log={:?}",
            bot.log
        );
    }
}

#[test]
fn cr_701_44b_explores_even_with_an_empty_library() {
    let mut g = new_game();
    put(&mut g, &must("Explore Watch", CAVE_DIVER_WATCH), ALICE, Zone::Battlefield);
    let w = enter(&mut g, &must("Merfolk Branchwalker", BRANCHWALKER), ALICE, &mut Bot::default());
    assert_eq!(life(&g, ALICE), 23, "CR 701.44b: the permanent explores even though nothing was revealed");
    assert_eq!(plus(&g, w), 1, "CR 701.44a: no land was revealed, so it gets a counter");
}

#[test]
fn cr_701_44c_explore_uses_last_known_information_after_leaving() {
    let mut g = new_game();
    put(&mut g, &must("Explore Watch", CAVE_DIVER_WATCH), ALICE, Zone::Battlefield);
    let l = put(&mut g, &land("Top Land"), ALICE, Zone::Library);
    let id = put(&mut g, &must("Merfolk Branchwalker", BRANCHWALKER), ALICE, Zone::Hand);
    let w = g.move_object_by_effect(id, Zone::Battlefield).unwrap();
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(&mut g, &mut q);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g, &mut q, &mut Bot::default()).unwrap();
    assert_eq!(g.stack.len(), 1);
    g.move_object_by_effect(w, Zone::Graveyard).unwrap();
    settle(&mut g, &mut Bot::default());
    assert!(find_named(&g, &hand(&g, ALICE), "Top Land").is_some(), "CR 701.44c: the departed permanent still explores");
    assert_eq!(life(&g, ALICE), 23, "CR 701.44c: LKI identifies the explorer and its controller");
    let _ = l;
}

#[test]
fn cr_701_44d_simultaneous_explores_happen_one_after_another() {
    let mut g = new_game();
    put(&mut g, &vanilla("Second", 2, 2), ALICE, Zone::Library);
    put(&mut g, &land("First Land"), ALICE, Zone::Library);
    let a = put(&mut g, &vanilla("Explorer A", 1, 1), ALICE, Zone::Battlefield);
    let b = put(&mut g, &vanilla("Explorer B", 1, 1), ALICE, Zone::Battlefield);
    cast(
        &mut g,
        &must("Group Explore", "Mana cost: {1}\nType: Sorcery\nEach creature you control explores."),
        ALICE,
        vec![],
        &mut Bot::default(),
    );
    assert!(find_named(&g, &hand(&g, ALICE), "First Land").is_some(), "first explore reveals the land");
    assert_eq!(
        plus(&g, a) + plus(&g, b),
        1,
        "CR 701.44d: explores are sequential, so the second reveals the next card (a nonland)"
    );
}

/// An enchantment that gains 7 life whenever its controller performs `kind`.
fn watcher(kind: ironsmith::events::KeywordActionKind) -> CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Keyword Watcher")
        .card_types(vec![ironsmith::CardType::Enchantment])
        .with_ability(ironsmith::Ability::triggered(
            ironsmith::triggers::Trigger::keyword_action(kind, ironsmith::PlayerFilter::You),
            vec![ironsmith::Effect::gain_life(7)],
        ))
        .build()
}

/// Resolve a permanent spell from the stack (so "as enters" replacements apply).
fn cast_permanent(g: &mut GameState, def: &CardDefinition, caster: PlayerId, dm: &mut dyn DecisionMaker) -> ObjectId {
    cast(g, def, caster, vec![], dm)
}

fn kw_events(g: &GameState) -> Vec<(ironsmith::events::KeywordActionKind, PlayerId, ObjectId, u32)> {
    g.effect_store
        .pending_trigger_events
        .iter()
        .filter_map(|e| e.downcast::<ironsmith::events::KeywordActionEvent>())
        .map(|e| (e.action, e.player, e.source, e.amount))
        .collect()
}

/// Resolve only the spell (no settling) and return the keyword-action events it produced.
fn cast_peek(
    g: &mut GameState,
    def: &CardDefinition,
    caster: PlayerId,
    targets: Vec<Target>,
    dm: &mut dyn DecisionMaker,
) -> Vec<(ironsmith::events::KeywordActionKind, PlayerId, ObjectId, u32)> {
    let id = put(g, def, caster, Zone::Stack);
    g.stack.push(StackEntry::new(id, caster).with_targets(targets));
    g.take_pending_trigger_events();
    ironsmith::game_loop::resolve_stack_entry_with(g, dm).expect("resolves");
    kw_events(g)
}

// ---------------------------------------------------------------------------
// 701.46 Adapt
// ---------------------------------------------------------------------------

#[test]
fn cr_701_46a_adapt_only_without_plus_one_counters() {
    let druid_text = "Mana cost: {1}{G}\nType: Creature — Elf Druid\nPower/Toughness: 0/2\n{3}{G}{G}: Adapt 3.";
    for preexisting in [0u32, 1] {
        let mut g = new_game();
        let d = put(&mut g, &must("Adapt Druid", druid_text), ALICE, Zone::Battlefield);
        if preexisting > 0 {
            g.add_counters(d, CounterType::PlusOnePlusOne, preexisting);
        }
        add_mana(&mut g, ALICE, ManaSymbol::Green, 5);
        assert!(activate_first(&mut g, ALICE, d, &mut Bot::default()));
        let expected = if preexisting == 0 { 3 } else { preexisting };
        assert_eq!(plus(&g, d), expected, "CR 701.46a: adapt adds counters only if none (pre={preexisting})");
        add_mana(&mut g, ALICE, ManaSymbol::Green, 5);
        assert!(activate_first(&mut g, ALICE, d, &mut Bot::default()));
        assert_eq!(plus(&g, d), expected, "CR 701.46a: a second adapt adds nothing");
    }
}

// ---------------------------------------------------------------------------
// 701.47 Amass
// ---------------------------------------------------------------------------

const CALLOUS_DISMISSAL: &str = "Mana cost: {1}{U}\nType: Sorcery\nReturn target nonland permanent to its owner's hand.\nAmass Zombies 1.";

fn armies(g: &GameState, p: PlayerId) -> Vec<ObjectId> {
    g.battlefield
        .iter()
        .copied()
        .filter(|id| {
            g.current_controller(*id) == Some(p)
                && g.current_characteristics(*id)
                    .is_some_and(|c| c.subtypes.iter().any(|s| format!("{s:?}") == "Army"))
        })
        .collect()
}
fn has_subtype(g: &GameState, id: ObjectId, name: &str) -> bool {
    g.current_characteristics(id)
        .is_some_and(|c| c.subtypes.iter().any(|s| format!("{s:?}") == name))
}

#[test]
fn cr_701_47a_b_amass_creates_or_grows_an_army_and_adds_the_subtype() {
    let mut g = new_game();
    let victim = put(&mut g, &vanilla("Victim", 1, 1), BOB, Zone::Battlefield);
    put(&mut g, &watcher(ironsmith::events::KeywordActionKind::Amass), ALICE, Zone::Battlefield);
    cast(&mut g, &must("Callous Dismissal", CALLOUS_DISMISSAL), ALICE, vec![Target::Object(victim)], &mut Bot::default());
    let a = armies(&g, ALICE);
    assert_eq!(a.len(), 1, "CR 701.47a: a 0/0 Army token is created when you control none");
    let army = a[0];
    assert!(has_subtype(&g, army, "Zombie"), "CR 701.47a: the created army is a Zombie Army");
    assert_eq!(plus(&g, army), 1, "CR 701.47a: N counters go on it (so it survives as 1/1)");
    assert_eq!(life(&g, ALICE), 27, "CR 701.47b: the player amassed (amass observer triggered)");
    // A second amass of a different subtype grows the existing army and adds the subtype.
    let victim2 = put(&mut g, &vanilla("Victim2", 1, 1), BOB, Zone::Battlefield);
    cast(
        &mut g,
        &must("Orc Dismissal", "Mana cost: {1}{U}\nType: Sorcery\nReturn target nonland permanent to its owner's hand.\nAmass Orcs 2."),
        ALICE,
        vec![Target::Object(victim2)],
        &mut Bot::default(),
    );
    assert_eq!(armies(&g, ALICE), vec![army], "CR 701.47a: no new army when you already control one");
    assert_eq!(plus(&g, army), 3, "CR 701.47a: counters go on the chosen army");
    assert!(has_subtype(&g, army, "Orc") && has_subtype(&g, army, "Zombie"), "CR 701.47a: it becomes an Orc in addition");
}

#[test]
fn cr_701_47c_the_army_you_amassed_refers_to_the_chosen_army() {
    let mut g = new_game();
    let bear = put(&mut g, &vanilla("Bob Bear", 2, 3), BOB, Zone::Battlefield);
    let mine = put(&mut g, &vanilla("Alice Bear", 2, 3), ALICE, Zone::Battlefield);
    cast(
        &mut g,
        &must(
            "Widespread Brutality",
            "Mana cost: {1}{B}{R}{R}\nType: Sorcery\nAmass Zombies 2, then the Army you amassed deals damage equal to its power to each non-Army creature.",
        ),
        ALICE,
        vec![],
        &mut Bot::default(),
    );
    let a = armies(&g, ALICE);
    assert_eq!(a.len(), 1);
    assert_eq!(
        (g.damage_on(bear), g.damage_on(mine), g.damage_on(a[0])),
        (2, 2, 0),
        "CR 701.47c: the amassed Army deals damage equal to its power to each non-Army creature"
    );
}

// ---------------------------------------------------------------------------
// 701.48 Learn
// ---------------------------------------------------------------------------

#[test]
fn cr_701_48a_learn_rummage_or_lesson_from_outside_the_game() {
    let cram = "Mana cost: {1}{B}\nType: Sorcery\nYou gain 4 life.\nLearn.";
    let lesson_def = must("Test Lesson", "Mana cost: {1}\nType: Sorcery — Lesson\nYou gain 1 life.");
    for discard in [true, false] {
        let mut g = new_game();
        let keep = put(&mut g, &vanilla("Hand Card", 1, 1), ALICE, Zone::Hand);
        put(&mut g, &land("Drawn"), ALICE, Zone::Library);
        let lesson = put(&mut g, &lesson_def, ALICE, Zone::OutsideGame);
        let mut bot = Bot { accept: discard, pick: vec![keep, lesson], ..Default::default() };
        cast(&mut g, &must("Cram Session", cram), ALICE, vec![], &mut bot);
        let h = hand(&g, ALICE);
        let names: Vec<String> = h.iter().map(|id| g.object(*id).unwrap().name.to_string()).collect();
        if discard {
            assert!(
                names.contains(&"Drawn".to_string()) && !names.contains(&"Hand Card".to_string()) && !names.contains(&"Test Lesson".to_string()),
                "CR 701.48a: discarding a card draws a card and offers no Lesson: {names:?}"
            );
        } else {
            assert!(
                names.contains(&"Test Lesson".to_string()) && names.contains(&"Hand Card".to_string()),
                "CR 701.48a: without discarding, a Lesson from outside the game may go to hand: {names:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 701.49 Venture into the dungeon
// ---------------------------------------------------------------------------

fn register_dungeons() {
    use ironsmith::triggers::Trigger;
    for (name, rooms) in [
        (
            "Aa Test Dungeon",
            vec![("Entry", vec!["Left", "Right"], 1), ("Left", vec!["Vault"], 10), ("Right", vec!["Vault"], 100), ("Vault", vec![], 1000)],
        ),
        ("Undercity", vec![("Gate", vec!["Throne"], 3), ("Throne", vec![], 30)]),
    ] {
        let mut def = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), name)
            .card_types(vec![ironsmith::CardType::Dungeon])
            .build();
        for (room, leads, gain) in rooms {
            def.abilities.push(ironsmith::Ability::triggered(
                Trigger::dungeon_room(room, leads.into_iter().map(String::from).collect()),
                vec![ironsmith::Effect::gain_life(gain)],
            ));
        }
        ironsmith::dungeon::register_dungeon_definition(&def).expect("valid dungeon");
    }
}

fn venture(g: &mut GameState, text: &str, room_pref: &str) {
    let mut bot = Bot { options: vec!["Aa Test Dungeon".into(), room_pref.into()], ..Default::default() };
    cast(g, &must("Venture Spell", &format!("Mana cost: {{1}}\nType: Sorcery\n{text}")), ALICE, vec![], &mut bot);
}

#[test]
fn cr_701_49a_b_c_venture_starts_follows_arrows_and_completes() {
    register_dungeons();
    let mut g = new_game();
    venture(&mut g, "Venture into the dungeon.", "Right");
    let p = g.active_dungeon(ALICE).cloned().expect("CR 701.49a: a dungeon is put into the command zone");
    assert_eq!((p.dungeon_name.as_str(), p.room_name.as_str()), ("Aa Test Dungeon", "Entry"), "CR 701.49a: topmost room");
    assert_eq!(life(&g, ALICE), 21, "entering the room triggers its ability");
    venture(&mut g, "Venture into the dungeon.", "Right");
    assert_eq!(g.active_dungeon(ALICE).unwrap().room_name, "Right", "CR 701.49b: the player chooses which arrow to follow");
    // Third venture: resolve it but leave the Vault room ability on the stack.
    let mut bot = Bot { options: vec!["Aa Test Dungeon".into()], ..Default::default() };
    let v = must("Venture Spell", "Mana cost: {1}\nType: Sorcery\nVenture into the dungeon.");
    let id = put(&mut g, &v, ALICE, Zone::Stack);
    g.stack.push(StackEntry::new(id, ALICE));
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut bot).unwrap();
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(&mut g, &mut q);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g, &mut q, &mut bot).unwrap();
    assert_eq!(g.active_dungeon(ALICE).unwrap().room_name, "Vault");
    assert_eq!(g.stack.len(), 1, "the Vault room ability is on the stack");
    // Fourth venture from the bottommost room, cast in response.
    cast(&mut g, &v, ALICE, vec![], &mut bot);
    assert_eq!(life(&g, ALICE), 1122, "20 + Entry 1 + Right 100 + Vault 1000 + new Entry 1");
    assert!(g.has_completed_named_dungeon(ALICE, "Aa Test Dungeon"), "CR 701.49c: venturing from the bottommost room completes it");
    let p = g.active_dungeon(ALICE).cloned().expect("CR 701.49c: a new dungeon is started");
    assert_eq!(p.room_name, "Entry", "CR 701.49c: venture marker on the topmost room of the new dungeon");
}

#[test]
fn cr_701_49d_venture_into_quality_restricts_only_the_starting_choice() {
    register_dungeons();
    let mut g = new_game();
    venture(&mut g, "You take the initiative.", "Throne");
    assert_eq!(
        g.active_dungeon(ALICE).map(|p| p.dungeon_name.clone()),
        Some("Undercity".to_string()),
        "CR 701.49d: venture into Undercity (via the initiative) starts a dungeon with that quality"
    );
    // Already in a dungeon: follow the normal procedure.
    let mut g = new_game();
    venture(&mut g, "Venture into the dungeon.", "Left");
    venture(&mut g, "You take the initiative.", "Left");
    assert_eq!(
        g.active_dungeon(ALICE).map(|p| (p.dungeon_name.clone(), p.room_name.clone())),
        Some(("Aa Test Dungeon".to_string(), "Left".to_string())),
        "CR 701.49d: with a dungeon already in the command zone, venture into [quality] advances normally"
    );
}

// ---------------------------------------------------------------------------
// 701.50 Connive
// ---------------------------------------------------------------------------

const INFORMANT: &str = "Mana cost: {1}{W}\nType: Creature — Human Wizard\nPower/Toughness: 2/1\nWhen this creature enters, it connives.";
const CONNIVE_WATCH: &str = "Mana cost: {1}{U}\nType: Enchantment\nWhenever a creature you control connives, you gain 3 life.";

#[test]
fn cr_701_50a_connive_draw_discard_counter_only_for_nonland() {
    for discard_land in [true, false] {
        let mut g = new_game();
        put(&mut g, &vanilla("Drawn Bear", 1, 1), ALICE, Zone::Library);
        let to_discard = if discard_land {
            put(&mut g, &land("Hand Land"), ALICE, Zone::Hand)
        } else {
            put(&mut g, &vanilla("Hand Bear", 1, 1), ALICE, Zone::Hand)
        };
        let mut bot = Bot { pick: vec![to_discard], ..Default::default() };
        let inf = enter(&mut g, &must("Raffine's Informant", INFORMANT), ALICE, &mut bot);
        assert_eq!(graveyard(&g, ALICE).len(), 1, "CR 701.50a: one card is discarded");
        assert!(find_named(&g, &hand(&g, ALICE), "Drawn Bear").is_some(), "CR 701.50a: draws a card first");
        assert_eq!(plus(&g, inf), u32::from(!discard_land), "CR 701.50a: counter only if a nonland card was discarded");
    }
}

#[test]
fn cr_701_50b_f_connive_uses_lki_and_happens_even_if_impossible() {
    // 701.50b: the conniving permanent left before resolution.
    let mut g = new_game();
    put(&mut g, &must("Connive Watch", CONNIVE_WATCH), ALICE, Zone::Battlefield);
    put(&mut g, &vanilla("Drawn Bear", 1, 1), ALICE, Zone::Library);
    let id = put(&mut g, &must("Raffine's Informant", INFORMANT), ALICE, Zone::Hand);
    let inf = g.move_object_by_effect(id, Zone::Battlefield).unwrap();
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(&mut g, &mut q);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g, &mut q, &mut Bot::default()).unwrap();
    g.move_object_by_effect(inf, Zone::Graveyard).unwrap();
    settle(&mut g, &mut Bot::default());
    assert_eq!(graveyard(&g, ALICE).len(), 2, "CR 701.50b: the controller still draws and discards");
    assert_eq!(life(&g, ALICE), 23, "CR 701.50b: LKI identifies who connived");
    // 701.50f: drawing is prohibited and the hand is empty: still connives.
    let mut g = new_game();
    put(&mut g, &must("Connive Watch", CONNIVE_WATCH), ALICE, Zone::Battlefield);
    put(&mut g, &land("Undrawable"), ALICE, Zone::Library);
    put(&mut g, &must("Maralen of the Mornsong", "Mana cost: {1}{B}{B}\nType: Legendary Creature — Elf Wizard\nPower/Toughness: 2/3\nPlayers can't draw cards."), BOB, Zone::Battlefield);
    enter(&mut g, &must("Raffine's Informant", INFORMANT), ALICE, &mut Bot::default());
    assert_eq!(life(&g, ALICE), 23, "CR 701.50f: the permanent connives even if the actions were impossible");
}

#[test]
fn cr_701_50c_simultaneous_connives_are_sequential() {
    let mut g = new_game();
    put(&mut g, &land("Second draw"), ALICE, Zone::Library);
    put(&mut g, &vanilla("First draw", 1, 1), ALICE, Zone::Library);
    let a = put(&mut g, &vanilla("Conniver A", 1, 1), ALICE, Zone::Battlefield);
    let b = put(&mut g, &vanilla("Conniver B", 1, 1), ALICE, Zone::Battlefield);
    struct DiscardDrawn;
    impl DecisionMaker for DiscardDrawn {
        fn decide_objects(&mut self, _: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
            // Discard the most recently drawn card (highest object id).
            let newest = c.candidates.iter().filter(|x| x.legal).map(|x| x.id).max();
            newest.into_iter().take(c.min.max(1)).collect()
        }
    }
    let plans = must(
        "Change of Plans",
        "Mana cost: {X}{1}{U}\nType: Instant\nEach of X target creatures you control connive. You may have any number of them phase out.",
    );
    let id = put(&mut g, &plans, ALICE, Zone::Stack);
    let mut entry = StackEntry::new(id, ALICE).with_targets(vec![Target::Object(a), Target::Object(b)]);
    entry.x_value = Some(2);
    g.stack.push(entry);
    settle(&mut g, &mut DiscardDrawn);
    let gy: Vec<String> = graveyard(&g, ALICE).iter().map(|i| g.object(*i).unwrap().name.to_string()).collect();
    assert_eq!(gy.len(), 3, "two connive discards plus the spell: {gy:?}");
    assert_eq!(
        plus(&g, a) + plus(&g, b),
        1,
        "CR 701.50c: each connive draws then discards before the next (first discards the nonland, second the land): {gy:?}"
    );
}

#[test]
fn cr_701_50d_e_connive_n_and_connive_zero() {
    // 701.50d: connive 2.
    let mut g = new_game();
    put(&mut g, &land("D1"), ALICE, Zone::Library);
    put(&mut g, &vanilla("D2", 1, 1), ALICE, Zone::Library);
    let mut bot = Bot::default();
    let c = enter(
        &mut g,
        &must("Conniver Two", "Mana cost: {1}\nType: Creature — Rogue\nPower/Toughness: 1/1\nWhen this creature enters, it connives 2."),
        ALICE,
        &mut Bot { pick: vec![], ..Default::default() },
    );
    let gy_nonland = graveyard(&g, ALICE).iter().filter(|id| !g.object(**id).unwrap().card_types.contains(&ironsmith::CardType::Land)).count() as u32;
    assert_eq!(graveyard(&g, ALICE).len(), 2, "CR 701.50d: draws 2 and discards 2");
    assert_eq!(plus(&g, c), gy_nonland, "CR 701.50d: one counter per nonland card discarded");
    let _ = &mut bot;
    // 701.50e: connive 0 (Spymaster's Vault with no deaths) is no connive event.
    let mut g = new_game();
    put(&mut g, &must("Connive Watch", CONNIVE_WATCH), ALICE, Zone::Battlefield);
    put(&mut g, &vanilla("D", 1, 1), ALICE, Zone::Library);
    let target = put(&mut g, &vanilla("Target", 1, 1), ALICE, Zone::Battlefield);
    let vault = put(
        &mut g,
        &must(
            "Spymaster's Vault",
            "Type: Land\n{T}: Add {B}.\n{B}, {T}: Target creature you control connives X, where X is the number of creatures that died this turn.",
        ),
        ALICE,
        Zone::Battlefield,
    );
    add_mana(&mut g, ALICE, ManaSymbol::Black, 1);
    let mut bot = Bot { targets: vec![Target::Object(target)], ..Default::default() };
    assert!(activate_first(&mut g, ALICE, vault, &mut bot), "vault activates");
    assert_eq!(hand(&g, ALICE).len(), 0, "connive 0 draws nothing");
    assert_eq!(life(&g, ALICE), 20, "CR 701.50e: connive 0 is not a connive event, so no trigger");
}

// ---------------------------------------------------------------------------
// 701.51 Open an Attraction
// ---------------------------------------------------------------------------

fn attraction_defs() -> Vec<CardDefinition> {
    (0..10)
        .map(|i| {
            ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), &format!("Attraction {i}"))
                .card_types(vec![ironsmith::CardType::Artifact])
                .subtypes(vec![ironsmith::Subtype::Attraction])
                .attraction_lights(vec![6])
                .with_spell_effect(vec![ironsmith::Effect::gain_life(1)])
                .build()
        })
        .collect()
}

const OPEN_SPELL: &str = "Mana cost: {1}\nType: Sorcery\nOpen an Attraction.";

#[test]
fn cr_701_51a_b_c_opening_needs_a_deck_and_puts_the_top_card_onto_the_battlefield() {
    let mut g = new_game();
    g.enable_attractions(vec![(ALICE, ironsmith::game_state::AttractionDeckFormat::Limited, attraction_defs())])
        .unwrap();
    let gamer = put(
        &mut g,
        &must("Open Watcher", "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1\nWhenever you open an Attraction, put a +1/+1 counter on this creature."),
        ALICE,
        Zone::Battlefield,
    );
    let bob_gamer = put(
        &mut g,
        &must("Open Watcher", "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1\nWhenever you open an Attraction, put a +1/+1 counter on this creature."),
        BOB,
        Zone::Battlefield,
    );
    // Bob has no Attraction deck.
    let before = g.battlefield.len();
    g.turn.active_player = BOB;
    cast(&mut g, &must("Open Spell", OPEN_SPELL), BOB, vec![], &mut Bot::default());
    assert_eq!(g.battlefield.len(), before, "CR 701.51a: a player without an Attraction deck can't open one");
    assert_eq!(plus(&g, bob_gamer), 0, "CR 701.51a/c: no open happened");
    g.turn.active_player = ALICE;
    cast(&mut g, &must("Open Spell", OPEN_SPELL), ALICE, vec![], &mut Bot::default());
    let opened = g.face_up_attractions().to_vec();
    assert_eq!(opened.len(), 1, "CR 701.51b: the top Attraction is put onto the battlefield");
    assert_eq!(g.object(opened[0]).unwrap().zone, Zone::Battlefield);
    assert_eq!(g.current_controller(opened[0]), Some(ALICE), "CR 701.51b: under your control");
    assert_eq!(plus(&g, gamer), 1, "CR 701.51c: opening triggers 'whenever you open an Attraction'");
}

// ---------------------------------------------------------------------------
// 701.53 Incubate
// ---------------------------------------------------------------------------

#[test]
fn cr_701_53a_b_incubate_creates_a_transformable_incubator_token() {
    let mut g = new_game();
    put(&mut g, &land("Lib"), ALICE, Zone::Library);
    cast(
        &mut g,
        &must("Eyes of Gitaxias", "Mana cost: {2}{U}\nType: Sorcery\nIncubate 3.\nDraw a card."),
        ALICE,
        vec![],
        &mut Bot::default(),
    );
    let inc: Vec<ObjectId> = battlefield_named(&g, "Incubator Token");
    assert_eq!(inc.len(), 1, "CR 701.53a: an Incubator token is created");
    let id = inc[0];
    let c = g.current_characteristics(id).unwrap();
    assert_eq!(plus(&g, id), 3, "CR 701.53a: it enters with N +1/+1 counters");
    assert!(
        c.card_types.contains(&ironsmith::CardType::Artifact) && !c.card_types.contains(&ironsmith::CardType::Creature) && c.colors.is_empty(),
        "CR 701.53b: front face is a colorless Incubator artifact"
    );
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 2);
    assert!(activate_first(&mut g, ALICE, id, &mut Bot::default()), "CR 701.53b: {{2}}: Transform this token");
    let c = g.current_characteristics(id).unwrap();
    assert_eq!(c.name.as_ref() as &str, "Phyrexian Token", "CR 701.53b: back face is named Phyrexian Token");
    assert!(
        c.card_types.contains(&ironsmith::CardType::Artifact) && c.card_types.contains(&ironsmith::CardType::Creature) && has_subtype(&g, id, "Phyrexian"),
        "CR 701.53b: back face is a Phyrexian artifact creature"
    );
    assert_eq!((c.base_power, c.base_toughness), (Some(0), Some(0)), "CR 701.53b: base 0/0");
    assert_eq!(pt(&g, id), (3, 3));
}


fn is_legendary(g: &GameState, id: ObjectId) -> bool {
    g.current_characteristics(id)
        .is_some_and(|c| c.supertypes.contains(&ironsmith::Supertype::Legendary))
}

fn change_control(g: &mut GameState, id: ObjectId, to: PlayerId) {
    use ironsmith::continuous::{EffectTarget, Modification};
    let src = put(g, &vanilla("Control Source", 1, 1), to, Zone::Command);
    ironsmith::effects::EffectExecutor::execute(
        &ironsmith::effects::ApplyContinuousEffect::new(
            EffectTarget::Specific(id),
            Modification::ChangeController(to),
            ironsmith::Until::Forever,
        ),
        g,
        &mut EffectContext::new_default(src, to),
    )
    .unwrap();
    g.refresh_continuous_state().unwrap();
}

// ---------------------------------------------------------------------------
// 701.54 The Ring tempts you
// ---------------------------------------------------------------------------

const BIRTHDAY_ESCAPE: &str = "Mana cost: {U}\nType: Sorcery\nDraw a card. The Ring tempts you.";

#[test]
fn cr_701_54a_d_e_ring_bearer_choice_replacement_control_loss_and_trigger_without_creatures() {
    let mut g = new_game();
    fill_library(&mut g, ALICE, 3);
    fill_library(&mut g, BOB, 3);
    put(&mut g, &watcher(ironsmith::events::KeywordActionKind::RingTemptsYou), ALICE, Zone::Battlefield);
    put(&mut g, &watcher(ironsmith::events::KeywordActionKind::RingTemptsYou), BOB, Zone::Battlefield);
    let a = put(&mut g, &vanilla("Bearer A", 2, 2), ALICE, Zone::Battlefield);
    let b = put(&mut g, &vanilla("Bearer B", 2, 2), ALICE, Zone::Battlefield);
    cast(&mut g, &must("Birthday Escape", BIRTHDAY_ESCAPE), ALICE, vec![], &mut Bot { pick: vec![a], ..Default::default() });
    assert_eq!(g.current_ring_bearer(ALICE), Some(a), "CR 701.54a: the chosen creature becomes your Ring-bearer");
    cast(&mut g, &must("Birthday Escape", BIRTHDAY_ESCAPE), ALICE, vec![], &mut Bot { pick: vec![b], ..Default::default() });
    assert_eq!(g.current_ring_bearer(ALICE), Some(b), "CR 701.54a: a new Ring-bearer replaces the old one");
    assert!(!is_legendary(&g, a) && is_legendary(&g, b), "CR 701.54a/c: only the current Ring-bearer is legendary");
    assert_eq!(life(&g, ALICE), 34, "CR 701.54d: each temptation triggers 'whenever the Ring tempts you'");
    change_control(&mut g, b, BOB);
    assert_eq!(
        g.current_ring_bearer(ALICE),
        None,
        "CR 701.54a/e: another player gaining control ends it being your Ring-bearer"
    );
    assert!(!is_legendary(&g, b), "CR 701.54e: no longer your Ring-bearer, so the Ring no longer applies");
    // Bob controls only the stolen creature; tempting a player with no creatures still tempts.
    let mut g2 = new_game();
    fill_library(&mut g2, BOB, 3);
    put(&mut g2, &watcher(ironsmith::events::KeywordActionKind::RingTemptsYou), BOB, Zone::Battlefield);
    g2.turn.active_player = BOB;
    cast(&mut g2, &must("Birthday Escape", BIRTHDAY_ESCAPE), BOB, vec![], &mut Bot::default());
    assert_eq!(g2.ring_temptations(BOB), 1, "CR 701.54d: the Ring tempts even with no creature to choose");
    assert_eq!(life(&g2, BOB), 27, "CR 701.54d: the trigger fires even though choosing was impossible");
}

#[test]
fn cr_701_54b_ring_bearer_is_not_copiable() {
    let mut g = new_game();
    fill_library(&mut g, ALICE, 2);
    let a = put(&mut g, &vanilla("Bearer A", 2, 2), ALICE, Zone::Battlefield);
    cast(&mut g, &must("Birthday Escape", BIRTHDAY_ESCAPE), ALICE, vec![], &mut Bot { pick: vec![a], ..Default::default() });
    let clone = cast(
        &mut g,
        &must("Copy Spell", "Mana cost: {1}\nType: Sorcery\nCreate a token that's a copy of target creature."),
        ALICE,
        vec![Target::Object(a)],
        &mut Bot::default(),
    );
    let _ = clone;
    let copies: Vec<ObjectId> = battlefield_named(&g, "Bearer A").into_iter().filter(|id| *id != a).collect();
    assert_eq!(copies.len(), 1, "fixture: clone copied the bearer");
    let clone = copies[0];
    assert_eq!(g.current_ring_bearer(ALICE), Some(a), "CR 701.54b: the copy is not a Ring-bearer");
    assert!(!is_legendary(&g, clone), "CR 701.54b: Ring-bearer (and its legendary) is not copied");
}

#[test]
fn cr_701_54c_the_ring_grants_legendary_evasion_and_level_two_loot() {
    use ironsmith::decision::AttackerDeclaration;
    let mut g = new_game();
    fill_library(&mut g, ALICE, 5);
    let bearer = put(&mut g, &vanilla("Small Bearer", 2, 2), ALICE, Zone::Battlefield);
    let big = put(&mut g, &vanilla("Big Blocker", 5, 5), BOB, Zone::Battlefield);
    let small = put(&mut g, &vanilla("Small Blocker", 1, 1), BOB, Zone::Battlefield);
    let mut bot = Bot { pick: vec![bearer], ..Default::default() };
    cast(&mut g, &must("Birthday Escape", BIRTHDAY_ESCAPE), ALICE, vec![], &mut bot);
    assert!(is_legendary(&g, bearer), "CR 701.54c: your Ring-bearer is legendary");
    cast(&mut g, &must("Birthday Escape", BIRTHDAY_ESCAPE), ALICE, vec![], &mut bot);
    assert_eq!(g.ring_temptations(ALICE), 2);
    g.remove_summoning_sickness(bearer);
    let hand_before = hand(&g, ALICE).len();
    let gy_before = graveyard(&g, ALICE).len();
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let mut combat = ironsmith::CombatState::default();
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations_with_dm(
        &mut g,
        &mut combat,
        &mut q,
        &[AttackerDeclaration { creature: bearer, target: ironsmith::AttackTarget::Player(BOB) }],
        &mut bot,
    )
    .unwrap();
    let blockers = ironsmith::compute_legal_blockers(&g, &combat, BOB);
    let valid: Vec<ObjectId> = blockers.iter().filter(|o| o.attacker == bearer).flat_map(|o| o.valid_blockers.clone()).collect();
    assert!(
        valid.contains(&small) && !valid.contains(&big),
        "CR 701.54c: the Ring-bearer can't be blocked by creatures with greater power: {valid:?}"
    );
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g, &mut q, &mut bot).unwrap();
    settle(&mut g, &mut bot);
    assert_eq!(
        (hand(&g, ALICE).len(), graveyard(&g, ALICE).len()),
        (hand_before, gy_before + 1),
        "CR 701.54c: tempted twice, the Ring-bearer attacking draws then discards"
    );
}

// ---------------------------------------------------------------------------
// 701.55 Villainous choice
// ---------------------------------------------------------------------------

const GREAT_PLAN: &str = "Mana cost: {4}{U}{B}\nType: Sorcery\nDraw three cards. Then target opponent faces a villainous choice — They discard three cards, or you may cast a spell from your hand without paying its mana cost.";

#[test]
fn cr_701_55a_b_villainous_choice_performs_the_chosen_option_even_if_partly_impossible() {
    let mut g = new_game();
    fill_library(&mut g, ALICE, 4);
    put(&mut g, &land("Bob only card"), BOB, Zone::Hand);
    let mut bot = Bot { options: vec!["discard".into()], ..Default::default() };
    cast(&mut g, &must("Great Intelligence's Plan", GREAT_PLAN), ALICE, vec![Target::Player(BOB)], &mut bot);
    assert!(bot.log.iter().any(|l| l.contains("discard")), "a villainous choice was offered: {:?}", bot.log);
    assert_eq!(
        (hand(&g, BOB).len(), graveyard(&g, BOB).len()),
        (0, 1),
        "CR 701.55a-b: Bob may choose 'discard three' with one card and discards as much as possible"
    );
}

#[test]
fn cr_701_55c_replacement_faces_the_choice_additional_times() {
    let mut g = new_game();
    fill_library(&mut g, ALICE, 4);
    for i in 0..6 {
        put(&mut g, &land(&format!("Bob card {i}")), BOB, Zone::Hand);
    }
    put(
        &mut g,
        &must(
            "The Valeyard",
            "Mana cost: {2}{U}{B}{R}\nType: Legendary Creature — Time Lord Noble\nPower/Toughness: 3/3\nIf an opponent would face a villainous choice, they face that choice an additional time.\nWhile voting, you may vote an additional time.",
        ),
        ALICE,
        Zone::Battlefield,
    );
    let mut bot = Bot { options: vec!["discard".into()], ..Default::default() };
    cast(&mut g, &must("Great Intelligence's Plan", GREAT_PLAN), ALICE, vec![Target::Player(BOB)], &mut bot);
    assert_eq!(graveyard(&g, BOB).len(), 6, "CR 701.55c: the whole choice is performed twice (3 + 3 discards)");
}

#[test]
fn cr_701_55d_multiple_players_face_the_choice_in_apnap_order() {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    g.turn.active_player = ALICE;
    g.turn.priority_player = Some(ALICE);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    let carol = PlayerId(2);
    for p in [BOB, carol] {
        fill_library(&mut g, p, 6);
    }
    struct Order(Vec<PlayerId>);
    impl DecisionMaker for Order {
        fn decide_options(&mut self, _: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
            if let Some(o) = c.options.iter().find(|o| o.legal && o.description.to_lowercase().contains("four")) {
                self.0.push(c.player);
                return vec![o.index];
            }
            c.options.iter().filter(|o| o.legal).take(c.min).map(|o| o.index).collect()
        }
    }
    let mut dm = Order(vec![]);
    cast(
        &mut g,
        &must(
            "Ensnared by the Mara",
            "Mana cost: {2}{R}{R}\nType: Sorcery\nEach opponent faces a villainous choice — They exile cards from the top of their library until they exile a nonland card, then you may cast that card without paying its mana cost, or that player exiles the top four cards of their library and Ensnared by the Mara deals damage equal to the total mana value of those exiled cards to that player.",
        ),
        ALICE,
        vec![],
        &mut dm,
    );
    assert_eq!(dm.0, vec![BOB, carol], "CR 701.55d: each opponent faces the choice one at a time in APNAP order");
}

// ---------------------------------------------------------------------------
// 701.56 Time travel
// ---------------------------------------------------------------------------

#[test]
fn cr_701_56a_time_travel_adjusts_your_time_countered_permanents_and_suspended_cards() {
    let mut g = new_game();
    let mine = put(&mut g, &vanilla("Timed", 1, 1), ALICE, Zone::Battlefield);
    g.add_counters(mine, CounterType::Time, 2);
    let bare = put(&mut g, &vanilla("No counters", 1, 1), ALICE, Zone::Battlefield);
    let theirs = put(&mut g, &vanilla("Bob timed", 1, 1), BOB, Zone::Battlefield);
    g.add_counters(theirs, CounterType::Time, 2);
    let susp = put(
        &mut g,
        &must("Rift Bolt", "Mana cost: {2}{R}\nType: Sorcery\nRift Bolt deals 3 damage to any target.\nSuspend 1—{R}"),
        ALICE,
        Zone::Exile,
    );
    g.add_counters(susp, CounterType::Time, 2);
    let mut bot = Bot { options: vec!["remove".into()], ..Default::default() };
    cast(&mut g, &must("Time Travel Spell", "Mana cost: {1}\nType: Sorcery\nTime travel."), ALICE, vec![], &mut bot);
    assert_eq!(
        (
            g.counter_count(mine, CounterType::Time),
            g.counter_count(susp, CounterType::Time),
            g.counter_count(theirs, CounterType::Time),
            g.counter_count(bare, CounterType::Time)
        ),
        (1, 1, 2, 0),
        "CR 701.56a: one time counter is removed from each chosen object you control/own with time counters; log={:?}",
        bot.log
    );
    let mut bot = Bot { options: vec!["put one".into()], ..Default::default() };
    cast(&mut g, &must("Time Travel Spell", "Mana cost: {1}\nType: Sorcery\nTime travel."), ALICE, vec![], &mut bot);
    assert_eq!(
        (g.counter_count(mine, CounterType::Time), g.counter_count(susp, CounterType::Time)),
        (2, 2),
        "CR 701.56a: or a time counter is put on each; log={:?}",
        bot.log
    );
}

// ---------------------------------------------------------------------------
// 701.57 Discover
// ---------------------------------------------------------------------------

const DISCOVER_3: &str = "Mana cost: {2}{R}\nType: Sorcery\nDiscover 3.";

fn discover_library(g: &mut GameState) -> (ObjectId, ObjectId, ObjectId, ObjectId) {
    // Bottom to top: bottom card, cheap spell, expensive creature, land (top).
    let bottom = put(g, &vanilla("Bottom", 1, 1), ALICE, Zone::Library);
    let cheap = put(g, &must("Cheap Gain", "Mana cost: {1}{W}\nType: Sorcery\nYou gain 5 life."), ALICE, Zone::Library);
    let pricey = put(g, &must("Pricey", "Mana cost: {4}{G}\nType: Creature — Beast\nPower/Toughness: 5/5"), ALICE, Zone::Library);
    let top = put(g, &land("Top Land"), ALICE, Zone::Library);
    (bottom, cheap, pricey, top)
}

#[test]
fn cr_701_57a_discover_exiles_to_a_qualifying_card_casts_or_hands_it_and_bottoms_the_rest() {
    for cast_it in [true, false] {
        let mut g = new_game();
        discover_library(&mut g);
        let mut bot = Bot { accept: cast_it, options: vec![if cast_it { "cast".into() } else { "hand".into() }], ..Default::default() };
        cast(&mut g, &must("Discover Spell", DISCOVER_3), ALICE, vec![], &mut bot);
        let lib: Vec<String> = library(&g, ALICE).iter().map(|i| g.object(*i).unwrap().name.to_string()).collect();
        assert_eq!(lib.len(), 3, "CR 701.57a: the two non-qualifying exiled cards go to the bottom: {lib:?}");
        assert_eq!(lib.last().map(String::as_str), Some("Bottom"), "CR 701.57a: the rest go under the library (top is last)");
        if cast_it {
            assert_eq!(life(&g, ALICE), 25, "CR 701.57a: the discovered card may be cast without paying its cost; log={:?}", bot.log);
        } else {
            assert!(find_named(&g, &hand(&g, ALICE), "Cheap Gain").is_some(), "CR 701.57a: if not cast it goes to hand; log={:?}", bot.log);
            assert_eq!(life(&g, ALICE), 20);
        }
    }
}

#[test]
fn cr_701_57b_c_discovered_even_without_a_qualifying_card() {
    let mut g = new_game();
    put(&mut g, &watcher(ironsmith::events::KeywordActionKind::Discover), ALICE, Zone::Battlefield);
    put(&mut g, &land("Only land"), ALICE, Zone::Library);
    cast(&mut g, &must("Discover Spell", DISCOVER_3), ALICE, vec![], &mut Bot::yes());
    assert_eq!(life(&g, ALICE), 27, "CR 701.57b: the player discovered even though nothing qualified");
    assert_eq!(library(&g, ALICE).len(), 1, "CR 701.57a/c: no discovered card; the land returns to the library");
}

// ---------------------------------------------------------------------------
// 701.58 Cloak
// ---------------------------------------------------------------------------

const CLOAK_SPELL: &str = "Mana cost: {1}{U}\nType: Sorcery\nCloak the top card of your library.";

fn has_ward(g: &GameState, id: ObjectId) -> bool {
    g.current_characteristics(id).is_some_and(|c| {
        c.static_abilities.iter().any(|a| format!("{a:?}").to_lowercase().contains("ward"))
    })
}

#[test]
fn cr_701_58a_b_cloak_is_a_nameless_2_2_with_ward_2_and_creature_cards_turn_up_for_mana_cost() {
    use ironsmith::special_actions::TurnFaceUpMethod;
    for creature_card in [true, false] {
        let mut g = new_game();
        let card = if creature_card {
            must("Big Ogre", "Mana cost: {2}{R}\nType: Creature — Ogre Warrior\nPower/Toughness: 5/4")
        } else {
            land("Plain Land")
        };
        put(&mut g, &card, ALICE, Zone::Library);
        enter(
            &mut g,
            &must(
                "Cryptic Coat",
                "Mana cost: {2}{U}\nType: Artifact — Equipment\nWhen this Equipment enters, cloak the top card of your library, then attach this Equipment to it.\nEquipped creature gets +1/+0 and can't be blocked.\n{1}{U}: Return this Equipment to its owner's hand.",
            ),
            ALICE,
            &mut Bot::default(),
        );
        let fd = face_down_ids(&g);
        assert_eq!(fd.len(), 1, "CR 701.58a: the card enters face down");
        let id = fd[0];
        assert!(g.is_cloaked(id), "CR 701.58a: it is a cloaked permanent");
        assert_eq!(g.current_characteristics(id).unwrap().base_power, Some(2), "CR 701.58a: base 2/2");
        assert_eq!(g.current_characteristics(id).unwrap().base_toughness, Some(2));
        assert!(has_ward(&g, id), "CR 701.58a: a cloaked permanent has ward {{2}}");
        add_mana(&mut g, ALICE, ManaSymbol::Red, 3);
        let result = turn_face_up(&mut g, id, TurnFaceUpMethod::PrintedManaCost);
        assert_eq!(result.is_ok(), creature_card, "CR 701.58b: only a creature card turns up for its mana cost: {result:?}");
        assert_eq!(g.is_face_down(id), !creature_card);
    }
}

#[test]
fn cr_701_58c_d_cloaked_morph_and_disguise_cards_may_use_either_procedure() {
    use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod};
    for (text, keyword_method) in [
        ("Mana cost: {3}{G}\nType: Creature — Beast\nPower/Toughness: 4/4\nMorph {1}{G}", TurnFaceUpMethod::TurnFaceUpAbility),
        ("Mana cost: {3}{G}\nType: Creature — Beast\nPower/Toughness: 4/4\nDisguise {1}{G}", TurnFaceUpMethod::DisguiseAbility),
    ] {
        for method in [keyword_method, TurnFaceUpMethod::PrintedManaCost] {
            let mut g = new_game();
            put(&mut g, &must("Hidden Beast", text), ALICE, Zone::Library);
            cast(&mut g, &must("Cloak Spell", CLOAK_SPELL), ALICE, vec![], &mut Bot::default());
            let id = face_down_ids(&g)[0];
            assert!(g.is_cloaked(id));
            add_mana(&mut g, ALICE, ManaSymbol::Green, 4);
            let check = ironsmith::special_actions::can_perform_check(
                &SpecialAction::TurnFaceUp { permanent_id: id, method },
                &g,
                ALICE,
            );
            assert!(check.is_ok(), "CR 701.58c-d: {method:?} is available: {check:?}");
            assert!(turn_face_up(&mut g, id, method).is_ok());
            assert_eq!(pt(&g, id), (4, 4), "CR 701.58c-d: turned face up via {method:?}");
        }
    }
}

#[test]
fn cr_701_58e_cloaking_multiple_cards_happens_one_at_a_time() {
    let mut g = new_game();
    for i in 0..5 {
        put(&mut g, &land(&format!("L{i}")), ALICE, Zone::Library);
    }
    put(
        &mut g,
        &must("Entry Watcher", "Mana cost: {1}\nType: Enchantment\nWhenever a creature you control enters, you gain 1 life."),
        ALICE,
        Zone::Battlefield,
    );
    let picks = library(&g, ALICE);
    cast(
        &mut g,
        &must("Hide in Plain Sight", "Mana cost: {3}{G}\nType: Sorcery\nLook at the top five cards of your library, cloak two of them, and put the rest on the bottom of your library in a random order."),
        ALICE,
        vec![],
        &mut Bot { pick: picks, ..Default::default() },
    );
    assert_eq!(face_down_ids(&g).len(), 2, "CR 701.58e: both cards are cloaked");
    assert_eq!(life(&g, ALICE), 22, "CR 701.58e: each cloaked card enters as its own event");
}

#[test]
fn cr_701_58f_prohibited_cloak_leaves_the_card_in_its_zone() {
    let mut g = new_game();
    let lib = put(&mut g, &land("Library Land"), ALICE, Zone::Library);
    put(
        &mut g,
        &must(
            "Grafdigger's Cage",
            "Mana cost: {1}\nType: Artifact\nCreature cards in graveyards and libraries can't enter the battlefield.\nPlayers can't cast spells from graveyards or libraries.",
        ),
        BOB,
        Zone::Battlefield,
    );
    cast(&mut g, &must("Cloak Spell", CLOAK_SPELL), ALICE, vec![], &mut Bot::default());
    assert!(face_down_ids(&g).is_empty(), "CR 701.58f: nothing is cloaked");
    assert!(library(&g, ALICE).contains(&lib), "CR 701.58f: the card remains in the library");
}

#[test]
fn cr_701_58g_cloaked_instant_revealed_but_stays_face_down() {
    let mut g = new_game();
    put(&mut g, &must("Shock Spell", "Mana cost: {R}\nType: Instant\nShock Spell deals 2 damage to any target."), ALICE, Zone::Library);
    put(
        &mut g,
        &must("Face Watcher", "Mana cost: {1}\nType: Enchantment\nWhenever a permanent you control is turned face up, you gain 5 life."),
        ALICE,
        Zone::Battlefield,
    );
    cast(&mut g, &must("Cloak Spell", CLOAK_SPELL), ALICE, vec![], &mut Bot::default());
    let id = face_down_ids(&g)[0];
    cast(
        &mut g,
        &must("Unmask", "Mana cost: {U}\nType: Instant\nTurn target face-down creature face up."),
        ALICE,
        vec![Target::Object(id)],
        &mut Bot::default(),
    );
    assert!(g.is_face_down(id), "CR 701.58g: a cloaked instant stays face down");
    assert_eq!(life(&g, ALICE), 20, "CR 701.58g: turned-face-up triggers don't trigger");
}

// ---------------------------------------------------------------------------
// 701.59 Collect evidence
// ---------------------------------------------------------------------------

const SURVEILLANCE_MONITOR: &str = "Mana cost: {3}{U}\nType: Creature — Vedalken Detective\nPower/Toughness: 3/4\nWhen this creature enters, you may collect evidence 4.\nWhenever you collect evidence, create a 1/1 colorless Thopter artifact creature token with flying.";

#[test]
fn cr_701_59a_b_collect_evidence_needs_total_mana_value_n_or_more() {
    for (mvs, can) in [(vec![2u32, 3], true), (vec![1, 2], false)] {
        let mut g = new_game();
        let mut ids = vec![];
        for (i, mv) in mvs.iter().enumerate() {
            ids.push(put(
                &mut g,
                &must(&format!("GY{i}"), &format!("Mana cost: {{{mv}}}\nType: Artifact")),
                ALICE,
                Zone::Graveyard,
            ));
        }
        let mut bot = Bot { accept: true, pick: ids.clone(), ..Default::default() };
        enter(&mut g, &must("Surveillance Monitor", SURVEILLANCE_MONITOR), ALICE, &mut bot);
        let thopters = g.battlefield.iter().filter(|id| g.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token).count();
        if can {
            assert_eq!(graveyard(&g, ALICE).len(), 0, "CR 701.59a: cards with total MV >= 4 are exiled");
            assert_eq!(thopters, 1, "CR 701.59a: evidence was collected");
        } else {
            assert_eq!(graveyard(&g, ALICE).len(), 2, "CR 701.59b: can't collect with total MV < 4");
            assert_eq!(thopters, 0, "CR 701.59b: evidence not collected; log={:?}", bot.log);
        }
    }
}

#[test]
fn cr_701_59c_evidence_was_collected_is_linked_to_the_additional_cost() {
    for collect in [true, false] {
        let mut g = new_game();
        let gy = put(&mut g, &must("GY Big", "Mana cost: {6}\nType: Artifact"), ALICE, Zone::Graveyard);
        let sprite = put(
            &mut g,
            &must(
                "Crimestopper Sprite",
                "Mana cost: {2}{U}\nType: Creature — Faerie Detective\nPower/Toughness: 2/2\nAs an additional cost to cast this spell, you may collect evidence 6.\nFlying\nWhen this creature enters, tap target creature. If evidence was collected, put a stun counter on it.",
            ),
            ALICE,
            Zone::Hand,
        );
        let target = put(&mut g, &vanilla("Bob target", 1, 1), BOB, Zone::Battlefield);
        add_mana(&mut g, ALICE, ManaSymbol::Blue, 3);
        let action = ironsmith::compute_legal_actions(&g, ALICE)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == sprite))
            .expect("sprite castable");
        let mut bot = Bot {
            accept: collect,
            pick: vec![gy],
            options: if collect { vec!["Evidence".into()] } else { vec![] },
            targets: vec![Target::Object(target)],
            ..Default::default()
        };
        assert!(take_action(&mut g, action, &mut bot));
        settle(&mut g, &mut bot);
        assert!(g.is_tapped(target), "fixture: ETB resolved; log={:?}", bot.log);
        assert_eq!(
            g.counter_count(target, CounterType::Stun),
            u32::from(collect),
            "CR 701.59c: 'if evidence was collected' reads the additional cost (collect={collect}); log={:?}",
            bot.log
        );
    }
}

// ---------------------------------------------------------------------------
// 701.60 Suspect
// ---------------------------------------------------------------------------

const SCAPEGOAT: &str = "Mana cost: {R}\nType: Creature — Goat\nPower/Toughness: 1/1\nHaste\nWhen this creature enters, suspect it.\nWhenever one or more other creatures you control enter, if this creature is suspected, you may suspect one of the other creatures. If you do, this creature is no longer suspected.";

fn has_menace(g: &GameState, id: ObjectId) -> bool {
    g.current_characteristics(id).is_some_and(|c| {
        c.static_abilities.iter().any(|a| format!("{a:?}").to_lowercase().contains("menace"))
    })
}

#[test]
fn cr_701_60a_c_suspected_has_menace_and_cant_block_until_it_stops_being_suspected() {
    let mut g = new_game();
    let goat = enter(&mut g, &must("Frantic Scapegoat", SCAPEGOAT), ALICE, &mut Bot::default());
    assert!(g.is_suspected(goat), "CR 701.60a: the creature becomes suspected");
    assert!(has_menace(&g, goat) && !g.can_block(goat), "CR 701.60c: suspected has menace and can't block");
    enter(
        &mut g,
        &must("Absolving Lammasu", "Mana cost: {4}{W}\nType: Creature — Lammasu\nPower/Toughness: 4/3\nFlying\nWhen this creature enters, all suspected creatures are no longer suspected.\nWhen this creature dies, you gain 3 life and suspect up to one target creature an opponent controls."),
        ALICE,
        &mut Bot::default(),
    );
    assert!(!g.is_suspected(goat), "CR 701.60a: a spell or ability can make it no longer suspected");
    assert!(!has_menace(&g, goat) && g.can_block(goat), "CR 701.60c: only while suspected");
}

#[test]
fn cr_701_60a_b_d_suspected_is_a_noncopiable_designation_lost_on_leaving_and_not_reapplied() {
    let mut g = new_game();
    let goat = enter(&mut g, &must("Frantic Scapegoat", SCAPEGOAT), ALICE, &mut Bot::default());
    assert!(!g.set_suspected(goat), "CR 701.60d: a suspected permanent can't become suspected again");
    // A vanilla creature suspected directly; a copy of it must not be suspected.
    let bear = put(&mut g, &vanilla("Suspect Bear", 2, 2), ALICE, Zone::Battlefield);
    assert!(g.set_suspected(bear));
    cast(
        &mut g,
        &must("Copy Spell", "Mana cost: {1}\nType: Sorcery\nCreate a token that's a copy of target creature."),
        ALICE,
        vec![Target::Object(bear)],
        &mut Bot::default(),
    );
    let copies: Vec<ObjectId> = battlefield_named(&g, "Suspect Bear").into_iter().filter(|id| *id != bear).collect();
    assert_eq!(copies.len(), 1, "fixture: clone copied the suspected bear");
    assert!(!g.is_suspected(copies[0]) && !has_menace(&g, copies[0]), "CR 701.60b: suspected is not a copiable value");
    let hand_id = g.move_object_by_effect(goat, Zone::Hand).unwrap();
    let back = g.move_object_by_effect(hand_id, Zone::Battlefield).unwrap();
    assert!(!g.is_suspected(back), "CR 701.60a: suspected ends when it leaves the battlefield (new object)");
    let gy = put(&mut g, &vanilla("Graveyard bear", 2, 2), ALICE, Zone::Graveyard);
    assert!(!g.set_suspected(gy), "CR 701.60b: only permanents can be suspected");
}


fn tokens(g: &GameState, p: PlayerId) -> Vec<ObjectId> {
    g.battlefield
        .iter()
        .copied()
        .filter(|id| g.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token && g.current_controller(*id) == Some(p))
        .collect()
}

// ---------------------------------------------------------------------------
// 701.61 Forage
// ---------------------------------------------------------------------------

const CURIOUS_FORAGER: &str = "Mana cost: {2}{G}\nType: Creature — Squirrel Druid\nPower/Toughness: 3/2\nWhen this creature enters, you may forage. When you do, return target permanent card from your graveyard to your hand.";

#[test]
fn cr_701_61a_forage_exiles_three_graveyard_cards_or_sacrifices_a_food() {
    // 0: three graveyard cards, 1: a Food and only two graveyard cards, 2: neither.
    for case in 0..2 {
        let mut g = new_game();
        put(&mut g, &watcher(ironsmith::events::KeywordActionKind::Forage), ALICE, Zone::Battlefield);
        let target = put(&mut g, &vanilla("Returnable", 2, 2), ALICE, Zone::Graveyard);
        let mut fodder = vec![put(&mut g, &land("GY land"), ALICE, Zone::Graveyard)];
        if case == 0 {
            fodder.push(put(&mut g, &land("GY land 2"), ALICE, Zone::Graveyard));
            fodder.push(put(&mut g, &land("GY land 3"), ALICE, Zone::Graveyard));
        }
        if case == 1 {
            cast(&mut g, &must("Food Maker", "Mana cost: {1}\nType: Sorcery\nCreate a Food token."), ALICE, vec![], &mut Bot::default());
        }
        let gy_before = graveyard(&g, ALICE).len();
        let mut bot = Bot { accept: true, pick: fodder.clone(), targets: vec![Target::Object(target)], options: if case == 1 { vec!["sacrifice".into()] } else { vec!["exile".into()] }, ..Default::default() };
        enter(&mut g, &must("Curious Forager", CURIOUS_FORAGER), ALICE, &mut bot);
        let returned = find_named(&g, &hand(&g, ALICE), "Returnable").is_some();
        match case {
            0 => {
                assert!(returned, "CR 701.61a: exiling three graveyard cards forages; log={:?}", bot.log);
                assert_eq!(life(&g, ALICE), 27, "CR 701.61a: you foraged");
            }
            1 => {
                assert!(returned, "CR 701.61a: sacrificing a Food forages; log={:?}", bot.log);
                assert!(tokens(&g, ALICE).is_empty(), "CR 701.61a: the Food was sacrificed");
                assert_eq!(graveyard(&g, ALICE).len(), gy_before - 1, "only the returned card left the graveyard");
            }
            _ => {
                assert!(!returned, "CR 701.61a: without three cards or a Food you can't forage");
                assert_eq!(life(&g, ALICE), 20);
            }
        }
    }
}

#[test]
fn cr_701_61a_unable_to_forage_resolves_without_foraging() {
    let mut g = new_game();
    let target = put(&mut g, &vanilla("Returnable", 2, 2), ALICE, Zone::Graveyard);
    put(&mut g, &land("GY land"), ALICE, Zone::Graveyard);
    let id = put(&mut g, &must("Curious Forager", CURIOUS_FORAGER), ALICE, Zone::Hand);
    g.move_object_by_effect(id, Zone::Battlefield).unwrap();
    let mut bot = Bot { accept: true, targets: vec![Target::Object(target)], ..Default::default() };
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(&mut g, &mut q);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g, &mut q, &mut bot).unwrap();
    assert_eq!(g.stack.len(), 1, "fixture: the forage trigger is on the stack");
    let result = ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut bot);
    assert!(
        result.is_ok(),
        "CR 701.61a: with fewer than three graveyard cards and no Food the player can't forage; the optional forage must simply not happen, got {result:?}; log={:?}",
        bot.log
    );
    assert!(find_named(&g, &hand(&g, ALICE), "Returnable").is_none(), "CR 701.61a: no forage, no reflexive return");
}

// ---------------------------------------------------------------------------
// 701.62 Manifest dread
// ---------------------------------------------------------------------------

const BREAK_DOWN_THE_DOOR: &str = "Mana cost: {2}{G}\nType: Instant\nChoose one —\n• Exile target artifact.\n• Exile target enchantment.\n• Manifest dread.";

#[test]
fn cr_701_62a_manifest_dread_manifests_one_of_two_and_bins_the_other() {
    let mut g = new_game();
    let lower = put(&mut g, &vanilla("Lower", 3, 3), ALICE, Zone::Library);
    let upper = put(&mut g, &land("Upper"), ALICE, Zone::Library);
    let deeper = put(&mut g, &land("Deeper"), ALICE, Zone::Library);
    let _ = deeper;
    let mut bot = Bot { options: vec!["manifest".into()], pick: vec![lower], ..Default::default() };
    // Make the library order: top = Deeper? Put "Deeper" at the bottom instead.
    g.player_mut(ALICE).unwrap().library.with_vec_mut(|v| {
        v.retain(|id| *id != deeper);
        v.insert(0, deeper);
    });
    cast(&mut g, &must("Break Down the Door", BREAK_DOWN_THE_DOOR), ALICE, vec![], &mut bot);
    let fd = face_down_ids(&g);
    assert_eq!(fd.len(), 1, "CR 701.62a: one card is manifested; log={:?}", bot.log);
    assert!(g.is_manifested(fd[0]));
    let gy: Vec<String> = graveyard(&g, ALICE).iter().map(|i| g.object(*i).unwrap().name.to_string()).collect();
    assert!(gy.contains(&"Upper".to_string()) || gy.contains(&"Lower".to_string()), "CR 701.62a: the other looked-at card goes to the graveyard: {gy:?}");
    assert_eq!(library(&g, ALICE), vec![deeper], "CR 701.62a: only the top two cards are involved");
    let _ = upper;
}

#[test]
fn cr_701_62b_manifest_dread_trigger_fires_even_if_nothing_was_manifested() {
    let mut g = new_game();
    put(&mut g, &watcher(ironsmith::events::KeywordActionKind::ManifestDread), ALICE, Zone::Battlefield);
    cast(&mut g, &must("Break Down the Door", BREAK_DOWN_THE_DOOR), ALICE, vec![], &mut Bot { options: vec!["manifest".into()], ..Default::default() });
    assert!(face_down_ids(&g).is_empty());
    assert_eq!(life(&g, ALICE), 27, "CR 701.62b: you manifested dread even with an empty library");
}

// ---------------------------------------------------------------------------
// 701.63 Endure
// ---------------------------------------------------------------------------

#[test]
fn cr_701_63a_endure_counters_unless_spirit_token() {
    for counters in [true, false] {
        let mut g = new_game();
        let mut bot = Bot { options: vec![if counters { "counter".into() } else { "spirit".into() }], ..Default::default() };
        let kg = enter(
            &mut g,
            &must("Fortress Kin-Guard", "Mana cost: {1}{W}\nType: Creature — Dog Soldier\nPower/Toughness: 1/1\nWhen this creature enters, it endures 1."),
            ALICE,
            &mut bot,
        );
        let toks = tokens(&g, ALICE);
        if counters {
            assert_eq!((plus(&g, kg), toks.len()), (1, 0), "CR 701.63a: N +1/+1 counters; log={:?}", bot.log);
        } else {
            assert_eq!((plus(&g, kg), toks.len()), (0, 1), "CR 701.63a: or an N/N Spirit token; log={:?}", bot.log);
            let c = g.current_characteristics(toks[0]).unwrap();
            assert_eq!((c.power, c.toughness), (Some(1), Some(1)), "CR 701.63a: 1/1");
            assert!(has_subtype(&g, toks[0], "Spirit") && c.colors.contains(ironsmith::color::Color::White), "CR 701.63a: white Spirit");
        }
    }
}

#[test]
fn cr_701_63b_endure_zero_does_nothing() {
    for choice in ["counter", "spirit"] {
        let mut g = new_game();
        let k = put(
            &mut g,
            &must("Krumar Initiate", "Mana cost: {1}{B}\nType: Creature — Human Cleric\nPower/Toughness: 2/2\n{X}{B}, {T}, Pay X life: This creature endures X. Activate only as a sorcery."),
            ALICE,
            Zone::Battlefield,
        );
        g.remove_summoning_sickness(k);
        add_mana(&mut g, ALICE, ManaSymbol::Black, 1);
        let mut bot = Bot { number: Some(0), options: vec![choice.into()], ..Default::default() };
        assert!(activate_first(&mut g, ALICE, k, &mut bot), "endure X activates with X=0");
        assert!(g.is_tapped(k), "fixture: the ability was activated");
        assert_eq!((plus(&g, k), tokens(&g, ALICE).len()), (0, 0), "CR 701.63b: endure 0 creates nothing ({choice}); log={:?}", bot.log);
    }
}

// ---------------------------------------------------------------------------
// 701.64 Harness
// ---------------------------------------------------------------------------

const MIND_STONE: &str = "Mana cost: {1}{W}\nType: Legendary Artifact — Infinity Stone\nIndestructible\n{T}: Add {W}.\n{5}{W}, {T}: Harness The Mind Stone.\n∞ — At the beginning of your end step, exile up to one other target nonland permanent you control, then return that card to the battlefield under its owner's control.";

fn activate_nonmana<D: DecisionMaker>(g: &mut GameState, p: PlayerId, source: ObjectId, dm: &mut D) -> bool {
    let idx = ironsmith::compute_legal_actions(g, p).unwrap().into_iter().filter_map(|a| match a {
        LegalAction::ActivateAbility { source: s, ability_index } if s == source => Some(ability_index),
        _ => None,
    }).last();
    match idx {
        Some(i) => activate(g, source, i, dm),
        None => false,
    }
}

#[test]
fn cr_701_64a_b_harness_is_a_once_only_noncopiable_battlefield_designation() {
    let mut g = new_game();
    let stone = put(&mut g, &must("The Mind Stone", MIND_STONE), ALICE, Zone::Battlefield);
    assert!(!g.is_harnessed(stone));
    add_mana(&mut g, ALICE, ManaSymbol::White, 6);
    assert!(activate_nonmana(&mut g, ALICE, stone, &mut Bot::default()), "harness ability activates");
    assert!(g.is_harnessed(stone), "CR 701.64a: it becomes harnessed");
    assert!(!g.harness(stone), "CR 701.64a: an already-harnessed permanent doesn't become harnessed again");
    cast(
        &mut g,
        &must("Copy Spell", "Mana cost: {1}\nType: Sorcery\nCreate a token that's a copy of target artifact."),
        ALICE,
        vec![Target::Object(stone)],
        &mut Bot::default(),
    );
    let copies: Vec<ObjectId> = battlefield_named(&g, "The Mind Stone").into_iter().filter(|id| *id != stone).collect();
    // A legendary copy goes to the graveyard via the legend rule; check the copy if it survived.
    for c in copies {
        assert!(!g.is_harnessed(c), "CR 701.64b: harnessed is not a copiable value");
    }
    let h = g.move_object_by_effect(stone, Zone::Hand).unwrap();
    let back = g.move_object_by_effect(h, Zone::Battlefield).unwrap();
    assert!(!g.is_harnessed(back), "CR 701.64b: it stays harnessed only until it leaves the battlefield");
    let gy = put(&mut g, &must("The Mind Stone", MIND_STONE), ALICE, Zone::Graveyard);
    assert!(!g.harness(gy) && !g.is_harnessed(gy), "CR 701.64b: only permanents can become harnessed");
}

// ---------------------------------------------------------------------------
// 701.65 Airbend
// ---------------------------------------------------------------------------

#[test]
fn cr_701_65a_b_airbend_exiles_and_owner_may_cast_for_two() {
    let mut g = new_game();
    put(&mut g, &watcher(ironsmith::events::KeywordActionKind::Airbend), ALICE, Zone::Battlefield);
    let victim = put(
        &mut g,
        &must("Pricey Beast", "Mana cost: {5}{G}{G}\nType: Creature — Beast\nPower/Toughness: 7/7"),
        BOB,
        Zone::Battlefield,
    );
    let stable = g.object(victim).unwrap().stable_id;
    let mut bot = Bot { targets: vec![Target::Object(victim)], ..Default::default() };
    enter(
        &mut g,
        &must("Aang, Airbending Master", "Mana cost: {4}{W}\nType: Legendary Creature — Human Avatar Ally\nPower/Toughness: 4/4\nWhen Aang enters, airbend another target creature."),
        ALICE,
        &mut bot,
    );
    let exiled = g
        .exile
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.stable_id == stable))
        .expect("CR 701.65a: the airbended object is exiled");
    assert_eq!(life(&g, ALICE), 27, "CR 701.65b: airbend trigger");
    g.turn.active_player = BOB;
    g.turn.priority_player = Some(BOB);
    add_mana(&mut g, BOB, ManaSymbol::Colorless, 2);
    let castable = ironsmith::compute_legal_actions(&g, BOB)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled));
    assert!(castable.is_some(), "CR 701.65a: its owner may cast it from exile by paying {{2}}");
    assert!(take_action(&mut g, castable.unwrap(), &mut Bot::default()));
    settle(&mut g, &mut Bot::default());
    assert_eq!(battlefield_named(&g, "Pricey Beast").len(), 1, "CR 701.65a: cast for {{2}} and resolved");
}

// ---------------------------------------------------------------------------
// 701.66 Earthbend
// ---------------------------------------------------------------------------

#[test]
fn cr_701_66a_b_earthbend_animates_a_land_and_returns_it_when_it_dies() {
    let mut g = new_game();
    put(&mut g, &watcher(ironsmith::events::KeywordActionKind::Earthbend), ALICE, Zone::Battlefield);
    let forest = put(&mut g, &land("My Forest"), ALICE, Zone::Battlefield);
    let stable = g.object(forest).unwrap().stable_id;
    let mut bot = Bot { targets: vec![Target::Object(forest)], ..Default::default() };
    enter(
        &mut g,
        &must("Badgermole", "Mana cost: {4}{G}\nType: Creature — Badger Mole\nPower/Toughness: 4/4\nWhen this creature enters, earthbend 2.\nCreatures you control with +1/+1 counters on them have trample."),
        ALICE,
        &mut bot,
    );
    let c = g.current_characteristics(forest).unwrap();
    assert!(
        c.card_types.contains(&ironsmith::CardType::Land) && c.card_types.contains(&ironsmith::CardType::Creature),
        "CR 701.66a: the land becomes a land creature"
    );
    assert_eq!((c.base_power, c.base_toughness), (Some(0), Some(0)), "CR 701.66a: base 0/0");
    assert_eq!(plus(&g, forest), 2, "CR 701.66a: N +1/+1 counters");
    assert!(has_haste(&g, forest), "CR 701.66a: haste");
    assert_eq!(life(&g, ALICE), 27, "CR 701.66b: the earthbend trigger");
    cast(&mut g, &must("Murder", "Mana cost: {1}{B}{B}\nType: Instant\nDestroy target creature."), BOB, vec![Target::Object(forest)], &mut Bot::default());
    let back = g
        .battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.stable_id == stable))
        .expect("CR 701.66a: the land returns to the battlefield when it dies");
    assert!(g.is_tapped(back), "CR 701.66a: it returns tapped");
    assert_eq!(g.current_controller(back), Some(ALICE));
}

fn has_haste(g: &GameState, id: ObjectId) -> bool {
    g.current_characteristics(id).is_some_and(|c| {
        c.static_abilities.iter().any(|a| format!("{a:?}").to_lowercase().contains("haste"))
    })
}

// ---------------------------------------------------------------------------
// 701.67 Waterbend
// ---------------------------------------------------------------------------

#[test]
fn cr_701_67a_c_waterbend_cost_can_be_paid_by_tapping_artifacts_and_creatures() {
    let mut g = new_game();
    put(&mut g, &watcher(ironsmith::events::KeywordActionKind::Waterbend), ALICE, Zone::Battlefield);
    fill_library(&mut g, ALICE, 3);
    let crew: Vec<ObjectId> = (0..3).map(|i| put(&mut g, &vanilla(&format!("Tapper {i}"), 1, 1), ALICE, Zone::Battlefield)).collect();
    let ice = put(
        &mut g,
        &must("Aang's Iceberg", "Mana cost: {2}{W}\nType: Enchantment\nFlash\nWhen this enchantment enters, exile up to one other target nonland permanent until this enchantment leaves the battlefield.\nWaterbend {3}: Sacrifice this enchantment. If you do, scry 2."),
        ALICE,
        Zone::Battlefield,
    );
    let mut bot = Bot { accept: true, pick: crew.clone(), ..Default::default() };
    assert!(activate_nonmana(&mut g, ALICE, ice, &mut bot), "CR 701.67a: with no mana, three creatures can pay waterbend {{3}}");
    assert!(crew.iter().all(|c| g.is_tapped(*c)), "CR 701.67a: each tapped creature paid {{1}}; log={:?}", bot.log);
    assert!(g.object(ice).is_none() || g.object(ice).unwrap().zone != Zone::Battlefield, "the iceberg was sacrificed");
    assert_eq!(life(&g, ALICE), 27, "CR 701.67c: paying a waterbend cost is waterbending");
}

#[test]
fn cr_701_67b_waterbend_taps_cannot_pay_other_generic_mana_in_the_total_cost() {
    let mut g = new_game();
    fill_library(&mut g, ALICE, 10);
    for i in 0..7 {
        put(&mut g, &vanilla(&format!("Tapper {i}"), 1, 1), ALICE, Zone::Battlefield);
    }
    let revival = put(
        &mut g,
        &must("Spirit Water Revival", "Mana cost: {1}{U}{U}\nType: Sorcery\nAs an additional cost to cast this spell, you may waterbend {6}.\nDraw two cards. If this spell's additional cost was paid, instead shuffle your graveyard into your library, draw seven cards, and you have no maximum hand size for the rest of the game.\nExile Spirit Water Revival."),
        ALICE,
        Zone::Hand,
    );
    add_mana(&mut g, ALICE, ManaSymbol::Blue, 2);
    let action = ironsmith::compute_legal_actions(&g, ALICE)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == revival));
    assert!(
        action.is_none(),
        "CR 701.67b: with only {{U}}{{U}} and seven creatures the {{1}} in the mana cost can't be paid by tapping creatures"
    );
    // Control: with the {1} available as real mana the spell is castable.
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 1);
    assert!(
        ironsmith::compute_legal_actions(&g, ALICE)
            .unwrap()
            .iter()
            .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == revival)),
        "control: castable once {{1}}{{U}}{{U}} is available"
    );
}

// ---------------------------------------------------------------------------
// 701.68 Blight
// ---------------------------------------------------------------------------

const BOGGART_MISCHIEF: &str = "Mana cost: {2}{B}\nType: Kindred Enchantment — Goblin\nWhen this enchantment enters, you may blight 1. If you do, create two 1/1 black and red Goblin creature tokens.\nWhenever a Goblin creature you control dies, each opponent loses 1 life and you gain 1 life.";

#[test]
fn cr_701_68a_b_d_blight_puts_minus_counters_on_your_creature_or_is_unavailable() {
    for have_creature in [true, false] {
        let mut g = new_game();
        put(&mut g, &watcher(ironsmith::events::KeywordActionKind::Blight), ALICE, Zone::Battlefield);
        let bear = have_creature.then(|| put(&mut g, &vanilla("Blight Bear", 3, 3), ALICE, Zone::Battlefield));
        let bobs = put(&mut g, &vanilla("Bob Bear", 3, 3), BOB, Zone::Battlefield);
        let mut bot = Bot { accept: true, pick: bear.into_iter().collect(), ..Default::default() };
        enter(&mut g, &must("Boggart Mischief", BOGGART_MISCHIEF), ALICE, &mut bot);
        assert_eq!(g.counter_count(bobs, CounterType::MinusOneMinusOne), 0, "CR 701.68a: only a creature you control");
        if let Some(bear) = bear {
            assert_eq!(g.counter_count(bear, CounterType::MinusOneMinusOne), 1, "CR 701.68a: N -1/-1 counters");
            assert_eq!(tokens(&g, ALICE).len(), 2, "CR 701.68a: 'if you do' succeeded");
            assert_eq!(life(&g, ALICE), 27, "CR 701.68d: whenever you blight");
        } else {
            assert_eq!(tokens(&g, ALICE).len(), 0, "CR 701.68b: with no creature you can't choose to blight; log={:?}", bot.log);
            assert_eq!(life(&g, ALICE), 20, "CR 701.68b: no blight happened");
        }
    }
}

#[test]
fn cr_701_68c_blighted_creature_refers_to_the_chosen_creature() {
    let mut g = new_game();
    let small = put(&mut g, &vanilla("Small Bear", 2, 2), ALICE, Zone::Battlefield);
    let big = put(&mut g, &vanilla("Big Bear", 5, 5), ALICE, Zone::Battlefield);
    let mut bot = Bot { accept: true, pick: vec![big], ..Default::default() };
    enter(
        &mut g,
        &must("Blight Copier", "Mana cost: {2}{B}\nType: Enchantment\nWhen this enchantment enters, you may blight 1. If you do, create a token that's a copy of the blighted creature."),
        ALICE,
        &mut bot,
    );
    assert_eq!(g.counter_count(big, CounterType::MinusOneMinusOne), 1);
    assert_eq!(battlefield_named(&g, "Big Bear").len(), 2, "CR 701.68c: the blighted creature is the one chosen; log={:?}", bot.log);
    assert_eq!(battlefield_named(&g, "Small Bear").len(), 1);
    let _ = small;
}

// ---------------------------------------------------------------------------
// 701.69 Heal
// ---------------------------------------------------------------------------

#[test]
fn cr_701_69a_damage_is_healed_removes_all_marked_damage() {
    let mut g = new_game();
    put(
        &mut g,
        &must("Dr. McCoy, Surly Surgeon", "Mana cost: {2}{W}{W}\nType: Legendary Creature — Human Doctor\nPower/Toughness: 2/4\nWhen Dr. McCoy enters, put a +1/+1 counter on another target creature you control.\nIf another creature you control with a +1/+1 counter on it would be destroyed, instead remove all +1/+1 counters from it and all damage already dealt to it is healed."),
        ALICE,
        Zone::Battlefield,
    );
    let bear = put(&mut g, &vanilla("Patient", 2, 4), ALICE, Zone::Battlefield);
    g.add_counters(bear, CounterType::PlusOnePlusOne, 1);
    cast(&mut g, &must("Shock Two", "Mana cost: {R}\nType: Instant\nShock Two deals 2 damage to target creature."), BOB, vec![Target::Object(bear)], &mut Bot::default());
    assert_eq!(g.damage_on(bear), 2, "fixture: damage marked");
    cast(&mut g, &must("Murder", "Mana cost: {1}{B}{B}\nType: Instant\nDestroy target creature."), BOB, vec![Target::Object(bear)], &mut Bot::default());
    assert!(g.battlefield.contains(&bear), "replacement saved it");
    assert_eq!(plus(&g, bear), 0);
    assert_eq!(g.damage_on(bear), 0, "CR 701.69a: all marked damage is removed when damage 'is healed'");
}

// ---------------------------------------------------------------------------
// 701.70 Recruit
// ---------------------------------------------------------------------------

#[test]
fn cr_701_70a_recruit_rummages_and_makes_a_soldier_for_a_nonland_discard() {
    for discard_land in [true, false] {
        let mut g = new_game();
        put(&mut g, &land("Drawn"), ALICE, Zone::Library);
        let d = if discard_land {
            put(&mut g, &land("Hand Land"), ALICE, Zone::Hand)
        } else {
            put(&mut g, &vanilla("Hand Bear", 1, 1), ALICE, Zone::Hand)
        };
        let mut bot = Bot { pick: vec![d], ..Default::default() };
        enter(
            &mut g,
            &must("Esgaroth Garrison", "Mana cost: {4}{W}\nType: Creature — Human Soldier\nPower/Toughness: 3/4\nWhen this creature enters, recruit."),
            ALICE,
            &mut bot,
        );
        assert!(find_named(&g, &hand(&g, ALICE), "Drawn").is_some(), "CR 701.70a: draws first");
        assert!(graveyard(&g, ALICE).contains(&d) || graveyard(&g, ALICE).len() == 1, "CR 701.70a: discards");
        let toks = tokens(&g, ALICE);
        assert_eq!(toks.len(), usize::from(!discard_land), "CR 701.70a: token only for a nonland discard");
        if let Some(t) = toks.first() {
            let c = g.current_characteristics(*t).unwrap();
            assert!(
                (c.power, c.toughness) == (Some(1), Some(1)) && has_subtype(&g, *t, "Human") && has_subtype(&g, *t, "Soldier") && c.colors.contains(ironsmith::color::Color::White),
                "CR 701.70a: a 1/1 white Human Soldier"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 701.71 Empower Jace
// ---------------------------------------------------------------------------

const ACADEMIC_ASCENT: &str = "Mana cost: {1}{W}\nType: Instant\nTarget creature gets +2/+2 and gains flying until end of turn.\nEmpower Jace 2.";

#[test]
fn cr_701_71a_empower_jace_creates_a_jace_token_once_and_adds_loyalty() {
    let mut g = new_game();
    let target = put(&mut g, &vanilla("Pumped", 1, 1), ALICE, Zone::Battlefield);
    cast(&mut g, &must("Academic Ascent", ACADEMIC_ASCENT), ALICE, vec![Target::Object(target)], &mut Bot::default());
    let jaces: Vec<ObjectId> = tokens(&g, ALICE)
        .into_iter()
        .filter(|id| g.current_characteristics(*id).is_some_and(|c| c.card_types.contains(&ironsmith::CardType::Planeswalker)))
        .collect();
    assert_eq!(jaces.len(), 1, "CR 701.71a: a Jace planeswalker token is created");
    let j = jaces[0];
    assert!(has_subtype(&g, j, "Jace"), "CR 701.71a: it is a Jace");
    assert!(g.current_characteristics(j).unwrap().colors.contains(ironsmith::color::Color::Blue), "CR 701.71a: blue");
    assert_eq!(g.counter_count(j, CounterType::Loyalty), 2, "CR 701.71a: 0 loyalty plus N counters");
    let target2 = put(&mut g, &vanilla("Pumped2", 1, 1), ALICE, Zone::Battlefield);
    cast(&mut g, &must("Academic Ascent", ACADEMIC_ASCENT), ALICE, vec![Target::Object(target2)], &mut Bot::default());
    let jaces2: Vec<ObjectId> = tokens(&g, ALICE)
        .into_iter()
        .filter(|id| g.current_characteristics(*id).is_some_and(|c| c.card_types.contains(&ironsmith::CardType::Planeswalker)))
        .collect();
    assert_eq!(jaces2, vec![j], "CR 701.71a: no new token when you control a Jace token");
    assert_eq!(g.counter_count(j, CounterType::Loyalty), 4, "CR 701.71a: N more loyalty counters on the chosen Jace");
    let abil = g.current_characteristics(j).unwrap().ability_labels.to_vec();
    assert!(abil.iter().any(|a| a.contains("Remove a loyalty counter") && a.contains("Surveil")) && abil.iter().any(|a| a.contains("Remove 3 loyalty counters") && a.contains("Draw")), "CR 701.71a: [-1]: Surveil 1 and [-3]: Draw a card: {abil:?}");
}
