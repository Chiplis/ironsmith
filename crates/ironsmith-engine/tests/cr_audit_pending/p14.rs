//! CR audit (pending-entry pass, package p14): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::AttackTarget;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    AttackersContext, BooleanContext, NumberContext, PriorityContext, SelectObjectsContext,
    SelectOptionsContext, TargetsContext,
};
use ironsmith::decisions::spec::AttackerDeclaration;
use ironsmith::game_state::{Phase, Step, Target};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CastingMethod, CounterType, ObjectId, Subtype, Zone,
};
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Harness: a scripted decision maker driving full native turns.
// ---------------------------------------------------------------------------

type ActionPred = Box<dyn Fn(&GameState, &LegalAction) -> bool>;
type Observer = Box<dyn FnMut(&GameState, PlayerId)>;

#[derive(Default)]
struct Script {
    /// Ordered native actions; each is taken once, the first time it is offered.
    plan: Vec<(PlayerId, ActionPred)>,
    observer: Option<Observer>,
    attackers: Option<Box<dyn FnMut(&GameState, &AttackersContext) -> Vec<AttackerDeclaration>>>,
    boolean: Option<Box<dyn FnMut(&GameState, &BooleanContext) -> Option<bool>>>,
    objects: Option<Box<dyn FnMut(&GameState, &SelectObjectsContext) -> Option<Vec<ObjectId>>>>,
    options: Option<Box<dyn FnMut(&GameState, &SelectOptionsContext) -> Option<Vec<usize>>>>,
    number: Option<Box<dyn FnMut(&GameState, &NumberContext) -> Option<u32>>>,
    targets: Option<Box<dyn FnMut(&GameState, &TargetsContext) -> Option<Vec<Target>>>>,
    log: Rc<RefCell<Vec<String>>>,
    /// Every priority menu seen: (step, player, offered actions).
    seen: Rc<RefCell<Vec<(Option<Step>, PlayerId, Vec<LegalAction>)>>>,
}

impl Script {
    fn new() -> Self {
        Self::default()
    }
    fn step(mut self, player: PlayerId, pred: impl Fn(&GameState, &LegalAction) -> bool + 'static) -> Self {
        self.plan.push((player, Box::new(pred)));
        self
    }
    fn attack_all(mut self, defender: PlayerId) -> Self {
        self.attackers = Some(Box::new(move |_, ctx| {
            ctx.attacker_options
                .iter()
                .filter(|o| o.valid_targets.contains(&AttackTarget::Player(defender)))
                .map(|o| AttackerDeclaration { creature: o.creature, target: AttackTarget::Player(defender) })
                .collect()
        }));
        self
    }
    fn attack_with(mut self, ids: Vec<ObjectId>, defender: PlayerId) -> Self {
        self.attackers = Some(Box::new(move |_, ctx| {
            ctx.attacker_options
                .iter()
                .filter(|o| ids.contains(&o.creature))
                .map(|o| AttackerDeclaration { creature: o.creature, target: AttackTarget::Player(defender) })
                .collect()
        }));
        self
    }
    fn attackers_none(mut self) -> Self {
        self.attackers = None;
        self
    }
    fn booleans(mut self, f: impl FnMut(&GameState, &BooleanContext) -> Option<bool> + 'static) -> Self {
        self.boolean = Some(Box::new(f));
        self
    }
    fn objects(mut self, f: impl FnMut(&GameState, &SelectObjectsContext) -> Option<Vec<ObjectId>> + 'static) -> Self {
        self.objects = Some(Box::new(f));
        self
    }
    fn options(mut self, f: impl FnMut(&GameState, &SelectOptionsContext) -> Option<Vec<usize>> + 'static) -> Self {
        self.options = Some(Box::new(f));
        self
    }
    fn numbers(mut self, f: impl FnMut(&GameState, &NumberContext) -> Option<u32> + 'static) -> Self {
        self.number = Some(Box::new(f));
        self
    }
    fn targets(mut self, f: impl FnMut(&GameState, &TargetsContext) -> Option<Vec<Target>> + 'static) -> Self {
        self.targets = Some(Box::new(f));
        self
    }
    fn observe(mut self, f: impl FnMut(&GameState, PlayerId) + 'static) -> Self {
        self.observer = Some(Box::new(f));
        self
    }
    fn log_handle(&self) -> Rc<RefCell<Vec<String>>> {
        self.log.clone()
    }
}

impl DecisionMaker for Script {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        self.seen.borrow_mut().push((game.turn.step, player, Vec::new()));
        if let Some(observer) = self.observer.as_mut() {
            observer(game, player);
        }
    }
    fn decide_priority(&mut self, game: &GameState, ctx: &PriorityContext) -> LegalAction {
        self.seen.borrow_mut().push((game.turn.step, ctx.player, ctx.actions.to_vec()));
        if let Some(observer) = self.observer.as_mut() {
            observer(game, ctx.player);
        }
        if let Some(pos) = self.plan.iter().position(|(player, _)| *player == ctx.player) {
            let pred = &self.plan[pos].1;
            #[allow(unused_must_use)]
            if let Some(action) = ctx.actions.iter().find(|a| pred(game, a)).cloned() {
                self.log.borrow_mut().push(format!("priority {:?}: {:?}", ctx.player, action));
                self.plan.remove(pos);
                return action;
            }
        }
        LegalAction::PassPriority
    }
    fn decide_attackers(&mut self, game: &GameState, ctx: &AttackersContext) -> Vec<AttackerDeclaration> {
        match self.attackers.as_mut() {
            Some(f) => f(game, ctx),
            None => Vec::new(),
        }
    }
    fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
        self.log.borrow_mut().push(format!("boolean: {}", ctx.description));
        if let Some(f) = self.boolean.as_mut()
            && let Some(answer) = f(game, ctx)
        {
            return answer;
        }
        ctx.can_accept
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.log.borrow_mut().push(format!(
            "objects: {} min={} max={:?} candidates={:?}",
            ctx.description,
            ctx.min,
            ctx.max,
            ctx.candidates.iter().map(|c| (c.id, c.legal)).collect::<Vec<_>>()
        ));
        if let Some(f) = self.objects.as_mut()
            && let Some(answer) = f(game, ctx)
        {
            return answer;
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.log.borrow_mut().push(format!(
            "options[{:?}]: {} min={} max={} {:?}",
            ctx.player,
            ctx.description,
            ctx.min,
            ctx.max,
            ctx.options.iter().map(|o| (o.index, o.description.clone(), o.legal)).collect::<Vec<_>>()
        ));
        if let Some(f) = self.options.as_mut()
            && let Some(answer) = f(game, ctx)
        {
            return answer;
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        self.log.borrow_mut().push(format!("number: {} {}..{}", ctx.description, ctx.min, ctx.max));
        if let Some(f) = self.number.as_mut()
            && let Some(answer) = f(game, ctx)
        {
            return answer;
        }
        ctx.min
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.log.borrow_mut().push(format!(
            "targets: {} {:?}",
            ctx.context,
            ctx.requirements.iter().map(|r| r.legal_targets.clone()).collect::<Vec<_>>()
        ));
        if let Some(f) = self.targets.as_mut()
            && let Some(answer) = f(game, ctx)
        {
            return answer;
        }
        SelectFirstDecisionMaker.decide_targets(game, ctx)
    }
}

/// Answer the "Choose optional costs" menu with the given option indices (repeat an index to pay it repeatedly).
fn optional_costs(choice: Vec<usize>) -> impl FnMut(&GameState, &SelectOptionsContext) -> Option<Vec<usize>> {
    move |_, ctx| ctx.description.starts_with("Choose optional costs").then(|| choice.clone())
}

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}

fn new_game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Library Filler")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
        .build();
    for player in [alice(), bob()] {
        for _ in 0..15 {
            game.create_object_from_definition(&filler, player, Zone::Library);
        }
    }
    game.turn.active_player = alice();
    game.turn.turn_number = 2;
    game
}

fn compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture compile of {name} failed: {e:?}"));
    def
}

fn put(game: &mut GameState, def: &CardDefinition, player: PlayerId, zone: Zone) -> ObjectId {
    let id = game.create_object_from_definition(def, player, zone);
    if zone == Zone::Battlefield {
        game.remove_summoning_sickness(id);
    }
    id
}

fn lands(game: &mut GameState, player: PlayerId, subtype: Subtype, n: usize) -> Vec<ObjectId> {
    let def = CardDefinitionBuilder::new(CardId::new(), &format!("{subtype:?} Source"))
        .card_types(vec![CardType::Land])
        .subtypes(vec![subtype])
        .build();
    (0..n).map(|_| put(game, &def, player, Zone::Battlefield)).collect()
}

fn vanilla(game: &mut GameState, player: PlayerId, name: &str, p: i32, t: i32) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(p, t))
        .build();
    put(game, &def, player, Zone::Battlefield)
}

/// Run the active player's whole turn natively, then pass the turn.
fn run_turn(game: &mut GameState, dm: &mut Script) {
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    let result = ironsmith::game_loop::execute_turn_with(game, &mut combat, &mut queue, dm);
    match result {
        Ok(()) | Err(ironsmith::game_loop::GameLoopError::GameOver) => {}
        Err(e) => panic!("native turn failed: {e:?}; log={:?}", dm.log.borrow()),
    }
    game.next_turn();
}

fn is_cast_of(action: &LegalAction, card: ObjectId) -> bool {
    matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == card)
}
fn is_cast_with(action: &LegalAction, card: ObjectId, method: &CastingMethod) -> bool {
    matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card && casting_method == method)
}
fn is_activation(action: &LegalAction, source: ObjectId) -> bool {
    matches!(action, LegalAction::ActivateAbility { source: s, .. } if *s == source)
}

fn ability_index_where(game: &GameState, id: ObjectId, pred: impl Fn(&str) -> bool) -> Option<usize> {
    game.current_abilities(id)?
        .iter()
        .position(|ability| pred(&format!("{ability:?}")))
}

fn link_faces(game: &mut GameState, front: &mut CardDefinition, back: &mut CardDefinition) {
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.to_string());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some(front.card.name.to_string());
    back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    game.register_linked_face_definition(front);
    game.register_linked_face_definition(back);
}

fn is_creature_now(game: &GameState, id: ObjectId) -> bool {
    game.current_characteristics(id).is_some_and(|c| c.card_types.contains(&CardType::Creature))
}

fn by_name(game: &GameState, zone: Zone, name: &str) -> Vec<ObjectId> {
    game.objects_in_zone(zone)
        .into_iter()
        .filter(|id| game.object(*id).is_some_and(|o| &*o.name == name))
        .collect()
}
fn with_subtype(game: &GameState, zone: Zone, subtype: Subtype) -> Vec<ObjectId> {
    game.objects_in_zone(zone)
        .into_iter()
        .filter(|id| game.current_characteristics(*id).is_some_and(|c| c.subtypes.contains(&subtype)))
        .collect()
}
fn plus_counters(game: &GameState, id: ObjectId) -> u32 {
    game.counter_count(id, CounterType::PlusOnePlusOne)
}
fn hand_len(game: &GameState, player: PlayerId) -> usize {
    game.player(player).unwrap().hand.len()
}

// ---------------------------------------------------------------------------
// 702.164 Toxic
// ---------------------------------------------------------------------------

#[test]
fn cr_702_164_toxic_total_value_gives_poison_with_combat_damage() {
    let mut game = new_game();
    let stalker = compile(
        "Branchblight Stalker",
        "Mana cost: {1}{G}\nType: Creature — Phyrexian Elf Scout\nPower/Toughness: 3/1\nToxic 2",
    );
    let granter = compile(
        "Toxic Granter",
        "Mana cost: {2}\nType: Artifact\nCreatures you control have toxic 1.",
    );
    let attacker = put(&mut game, &stalker, alice(), Zone::Battlefield);
    put(&mut game, &granter, alice(), Zone::Battlefield);
    let mut dm = Script::new().attack_with(vec![attacker], bob());
    run_turn(&mut game, &mut dm);
    let bob_state = game.player(bob()).unwrap();
    assert_eq!(bob_state.life, 17, "CR 702.164c: combat damage keeps its other results");
    assert_eq!(
        bob_state.poison_counters, 3,
        "CR 702.164b/c: total toxic value 2+1 gives three poison counters"
    );
}

#[test]
fn cr_702_164c_toxic_noncombat_damage_gives_no_poison() {
    let mut game = new_game();
    let stalker = compile(
        "Branchblight Stalker",
        "Mana cost: {1}{G}\nType: Creature — Phyrexian Elf Scout\nPower/Toughness: 3/1\nToxic 2\n{0}: This creature deals 1 damage to target player.",
    );
    let source = put(&mut game, &stalker, alice(), Zone::Battlefield);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, source))
        .targets(|_, ctx| Some(vec![Target::Player(PlayerId::from_index(1))]).filter(|_| !ctx.requirements.is_empty()));
    run_turn(&mut game, &mut dm);
    let bob_state = game.player(bob()).unwrap();
    assert_eq!(bob_state.life, 19, "fixture: activated damage resolved; log={:?}", dm.log.borrow());
    assert_eq!(bob_state.poison_counters, 0, "CR 702.164c: only combat damage gives toxic poison");
}

// ---------------------------------------------------------------------------
// 702.181 Mobilize
// ---------------------------------------------------------------------------

#[test]
fn cr_702_181a_mobilize_tokens_enter_tapped_attacking_and_are_sacrificed() {
    let mut game = new_game();
    let packbeasts = compile(
        "Dalkovan Packbeasts",
        "Mana cost: {2}{W}\nType: Creature — Ox\nPower/Toughness: 0/4\nVigilance\nMobilize 3",
    );
    let beast = put(&mut game, &packbeasts, alice(), Zone::Battlefield);
    let seen: Rc<RefCell<Vec<(usize, usize)>>> = Rc::default();
    let seen2 = seen.clone();
    let mut dm = Script::new().attack_with(vec![beast], bob()).observe(move |g, _| {
        if g.turn.step == Some(Step::DeclareBlockers) || g.turn.step == Some(Step::DeclareAttackers) {
            let warriors = with_subtype(g, Zone::Battlefield, Subtype::Warrior);
            let tapped = warriors.iter().filter(|w| g.is_tapped(**w)).count();
            seen2.borrow_mut().push((warriors.len(), tapped));
        }
    });
    run_turn(&mut game, &mut dm);
    let observed = seen.borrow().clone();
    assert!(
        observed.iter().any(|(n, tapped)| *n == 3 && *tapped == 3),
        "CR 702.181a: three tapped Warrior tokens during combat; saw {observed:?} log={:?}",
        dm.log.borrow()
    );
    assert_eq!(game.player(bob()).unwrap().life, 17, "CR 702.181a: the tokens were attacking");
    assert!(
        with_subtype(&game, Zone::Battlefield, Subtype::Warrior).is_empty(),
        "CR 702.181a: tokens sacrificed at the beginning of the next end step"
    );
}

// ---------------------------------------------------------------------------
// 702.189 Firebending
// ---------------------------------------------------------------------------

#[test]
fn cr_702_189a_firebending_mana_lasts_until_end_of_combat() {
    let mut game = new_game();
    let sages = compile(
        "Fire Sages",
        "Mana cost: {1}{R}\nType: Creature — Human Cleric\nPower/Toughness: 2/2\nFirebending 1",
    );
    let attacker = put(&mut game, &sages, alice(), Zone::Battlefield);
    let seen: Rc<RefCell<Vec<(Option<Step>, u32)>>> = Rc::default();
    let seen2 = seen.clone();
    let mut dm = Script::new().attack_with(vec![attacker], bob()).observe(move |g, p| {
        if p == PlayerId::from_index(0) {
            seen2.borrow_mut().push((g.turn.step, g.player(p).unwrap().mana_pool.red));
        }
    });
    run_turn(&mut game, &mut dm);
    let observed = seen.borrow().clone();
    let red_in = |step: Step| observed.iter().filter(|(s, _)| *s == Some(step)).map(|(_, r)| *r).max();
    assert_eq!(red_in(Step::DeclareAttackers), Some(1), "CR 702.189a: firebending adds {{R}}; saw {observed:?}");
    assert_eq!(red_in(Step::CombatDamage), Some(1), "CR 702.189a: the mana survives step changes during combat; saw {observed:?}");
    assert_eq!(red_in(Step::End), Some(0), "CR 702.189a: the mana empties after combat; saw {observed:?}");
}

// ---------------------------------------------------------------------------
// 702.152 Blitz
// ---------------------------------------------------------------------------

#[test]
fn cr_702_152a_blitz_haste_draw_on_death_and_end_step_sacrifice() {
    let mut game = new_game();
    let decoy = compile(
        "Riveteers Decoy",
        "Mana cost: {1}{G}\nType: Creature — Human Warrior\nPower/Toughness: 3/1\nBlitz {3}{G}",
    );
    lands(&mut game, alice(), Subtype::Forest, 4);
    let card = put(&mut game, &decoy, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_with(a, card, &CastingMethod::Alternative(0)))
        .attack_all(bob());
    let hand_before = hand_len(&game, alice());
    run_turn(&mut game, &mut dm);
    assert_eq!(
        game.player(bob()).unwrap().life,
        17,
        "CR 702.152a: a blitzed creature has haste; log={:?}",
        dm.log.borrow()
    );
    assert_eq!(by_name(&game, Zone::Graveyard, "Riveteers Decoy").len(), 1, "CR 702.152a: sacrificed at the next end step");
    // hand_before counts the decoy; it was cast (-1), a draw-step draw (+1), and the dies draw (+1).
    assert_eq!(hand_len(&game, alice()), hand_before + 1, "CR 702.152a: \"When this creature dies, draw a card\"");
}

// ---------------------------------------------------------------------------
// 702.177 Exhaust
// ---------------------------------------------------------------------------

#[test]
fn cr_702_177a_exhaust_ability_activates_only_once_ever() {
    let mut game = new_game();
    let specialist = compile(
        "Prowcatcher Specialist",
        "Mana cost: {1}{R}\nType: Creature — Goblin Warrior\nPower/Toughness: 2/1\nHaste\nExhaust — {3}{R}: Put two +1/+1 counters on this creature.",
    );
    lands(&mut game, alice(), Subtype::Mountain, 8);
    let creature = put(&mut game, &specialist, alice(), Zone::Battlefield);
    let offered: Rc<RefCell<Vec<u32>>> = Rc::default();
    let offered2 = offered.clone();
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, creature))
        .step(alice(), move |_, a| is_activation(a, creature));
    run_turn(&mut game, &mut dm);
    assert_eq!(plus_counters(&game, creature), 2, "CR 702.177a: one exhaust activation resolved; log={:?}", dm.log.borrow());
    // Next own turn: still not activatable.
    run_turn(&mut game, &mut Script::new());
    let mut dm = Script::new().step(alice(), move |_, a| is_activation(a, creature)).observe(move |g, p| {
        let _ = (g, p);
        offered2.borrow_mut().push(0);
    });
    run_turn(&mut game, &mut dm);
    assert_eq!(plus_counters(&game, creature), 2, "CR 702.177a: \"Activate only once\" persists across turns");
    let _ = offered;
}

// ---------------------------------------------------------------------------
// 702.156 Ravenous
// ---------------------------------------------------------------------------

#[test]
fn cr_702_156a_ravenous_counters_and_draw_threshold() {
    for (x, draws) in [(4u32, false), (5u32, true)] {
        let mut game = new_game();
        let rabbit = compile(
            "Jacked Rabbit",
            "Mana cost: {X}{1}{W}\nType: Creature — Rabbit Warrior\nPower/Toughness: 1/2\nRavenous",
        );
        lands(&mut game, alice(), Subtype::Plains, 7);
        let card = put(&mut game, &rabbit, alice(), Zone::Hand);
        let mut dm = Script::new()
            .step(alice(), move |_, a| is_cast_of(a, card))
            .numbers(move |_, ctx| Some(x).filter(|_| ctx.is_x_value));
        let hand_before = hand_len(&game, alice());
        run_turn(&mut game, &mut dm);
        let permanent = by_name(&game, Zone::Battlefield, "Jacked Rabbit");
        assert_eq!(permanent.len(), 1, "fixture: rabbit resolved; log={:?}", dm.log.borrow());
        assert_eq!(plus_counters(&game, permanent[0]), x, "CR 702.156a: enters with X +1/+1 counters");
        let expected = hand_before - 1 + 1 + usize::from(draws);
        assert_eq!(hand_len(&game, alice()), expected, "CR 702.156a: draw only if X is 5 or more (X={x})");
    }
}

// ---------------------------------------------------------------------------
// 702.151 Reconfigure
// ---------------------------------------------------------------------------

#[test]
fn cr_702_151_reconfigure_attach_stops_being_creature_and_unattach_restores() {
    let mut game = new_game();
    let blades = compile(
        "Lizard Blades",
        "Mana cost: {1}{R}\nType: Artifact Creature — Equipment Lizard\nPower/Toughness: 1/1\nDouble strike\nEquipped creature has double strike.\nReconfigure {2}",
    );
    lands(&mut game, alice(), Subtype::Mountain, 4);
    let equipment = put(&mut game, &blades, alice(), Zone::Battlefield);
    let bear = vanilla(&mut game, alice(), "Bear", 2, 2);
    let attach_index = ability_index_where(&game, equipment, |d| d.contains("ReconfigureEffect") && d.contains("Target"))
        .expect("fixture: attach half of reconfigure");
    let unattach_index = ability_index_where(&game, equipment, |d| d.contains("ReconfigureEffect") && !d.contains("Target"))
        .expect("fixture: unattach half of reconfigure");
    let mut dm = Script::new().step(alice(), move |_, a| {
        matches!(a, LegalAction::ActivateAbility { source, ability_index } if *source == equipment && *ability_index == attach_index)
    });
    let seen = dm.seen.clone();
    run_turn(&mut game, &mut dm);
    let unattach_offered_unattached = seen.borrow().iter().take(1).any(|(_, _, actions)| {
        actions.iter().any(|a| matches!(a, LegalAction::ActivateAbility { source, ability_index } if *source == equipment && *ability_index == unattach_index))
    });
    assert!(!unattach_offered_unattached, "CR 702.151a: unattach requires being attached");
    assert_eq!(
        game.object(equipment).unwrap().attached_to,
        Some(ironsmith::object::AttachmentTarget::Object(bear)),
        "CR 702.151a: reconfigure attaches to target creature you control; log={:?}",
        dm.log.borrow()
    );
    assert!(!is_creature_now(&game, equipment), "CR 702.151b: attached reconfigure Equipment isn't a creature");
    let combat_offer = seen.borrow().iter().any(|(step, _, actions)| {
        step.is_some_and(|s| matches!(s, Step::BeginCombat | Step::DeclareAttackers | Step::End))
            && actions.iter().any(|a| is_activation(a, equipment))
    });
    assert!(!combat_offer, "CR 702.151a: reconfigure only as a sorcery");
    run_turn(&mut game, &mut Script::new());
    let mut dm = Script::new().step(alice(), move |_, a| {
        matches!(a, LegalAction::ActivateAbility { source, ability_index } if *source == equipment && *ability_index == unattach_index)
    });
    run_turn(&mut game, &mut dm);
    assert_eq!(game.object(equipment).unwrap().attached_to, None, "CR 702.151a: unattach; log={:?}", dm.log.borrow());
    assert!(is_creature_now(&game, equipment), "CR 702.151b: unattached, it's a creature again");
}

// ---------------------------------------------------------------------------
// 702.153 Casualty
// ---------------------------------------------------------------------------

#[test]
fn cr_702_153a_casualty_sacrifice_copies_the_spell() {
    let mut game = new_game();
    let maestros = compile(
        "Join the Maestros",
        "Mana cost: {4}{B}\nType: Sorcery\nCasualty 2\nCreate a 4/3 black Ogre Warrior creature token.",
    );
    lands(&mut game, alice(), Subtype::Swamp, 5);
    let fodder = vanilla(&mut game, alice(), "Fodder", 2, 2);
    vanilla(&mut game, alice(), "Too Small", 1, 1);
    let card = put(&mut game, &maestros, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card)).objects(move |_, ctx| {
        ctx.candidates.iter().any(|c| c.id == fodder).then(|| vec![fodder])
    });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(by_name(&game, Zone::Battlefield, "Fodder").is_empty(), "CR 702.153a: casualty sacrificed a power-2 creature; log={:?}", log.borrow());
    assert_eq!(
        with_subtype(&game, Zone::Battlefield, Subtype::Ogre).len(),
        2,
        "CR 702.153a: \"When you do, copy this spell\"; log={:?}",
        log.borrow()
    );
}

#[test]
fn cr_702_153b_each_casualty_instance_is_paid_and_copies_separately() {
    let mut game = new_game();
    let spell = compile(
        "Double Casualty",
        "Mana cost: {1}{B}\nType: Sorcery\nCasualty 1\nCasualty 1\nCreate a 4/3 black Ogre Warrior creature token.",
    );
    lands(&mut game, alice(), Subtype::Swamp, 2);
    vanilla(&mut game, alice(), "Fodder A", 2, 2);
    vanilla(&mut game, alice(), "Fodder B", 2, 2);
    let card = put(&mut game, &spell, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(
        with_subtype(&game, Zone::Battlefield, Subtype::Ogre).len(),
        3,
        "CR 702.153b: two casualty payments give two copies plus the original; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.154 Enlist
// ---------------------------------------------------------------------------

#[test]
fn cr_702_154a_c_enlist_taps_eligible_nonattacker_and_adds_power() {
    let mut game = new_game();
    let warbrute = compile(
        "Coalition Warbrute",
        "Mana cost: {3}{R}\nType: Creature — Minotaur Berserker\nPower/Toughness: 3/4\nEnlist\nTrample",
    );
    let attacker = put(&mut game, &warbrute, alice(), Zone::Battlefield);
    let helper = vanilla(&mut game, alice(), "Helper", 2, 2);
    let mut dm = Script::new().attack_with(vec![attacker], bob());
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(game.is_tapped(helper), "CR 702.154a/c: the enlisted creature is tapped; log={:?}", log.borrow());
    assert_eq!(game.player(bob()).unwrap().life, 15, "CR 702.154a: enlist adds the enlisted creature's power");
}

#[test]
fn cr_702_154a_summoning_sick_creature_without_haste_cannot_be_enlisted() {
    let mut game = new_game();
    let warbrute = compile(
        "Coalition Warbrute",
        "Mana cost: {3}{R}\nType: Creature — Minotaur Berserker\nPower/Toughness: 3/4\nEnlist\nTrample",
    );
    let attacker = put(&mut game, &warbrute, alice(), Zone::Battlefield);
    let fresh_def = compile("Fresh", "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/2");
    lands(&mut game, alice(), Subtype::Mountain, 1);
    let fresh_card = put(&mut game, &fresh_def, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, fresh_card))
        .attack_with(vec![attacker], bob());
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let fresh = by_name(&game, Zone::Battlefield, "Fresh");
    assert_eq!(fresh.len(), 1, "fixture: Fresh was cast precombat; log={:?}", log.borrow());
    let fresh = fresh[0];
    assert!(!game.is_tapped(fresh), "CR 702.154a: a creature that came under control this turn can't be enlisted; log={:?}", log.borrow());
    assert_eq!(game.player(bob()).unwrap().life, 17, "CR 702.154a: no enlist bonus");
}

#[test]
fn cr_702_154d_multiple_enlist_instances_each_tap_and_add_their_own_creature() {
    let mut game = new_game();
    let warbrute = compile(
        "Double Enlister",
        "Mana cost: {3}{R}\nType: Creature — Minotaur Berserker\nPower/Toughness: 3/4\nEnlist\nEnlist",
    );
    let attacker = put(&mut game, &warbrute, alice(), Zone::Battlefield);
    let a = vanilla(&mut game, alice(), "Helper A", 2, 2);
    let b = vanilla(&mut game, alice(), "Helper B", 1, 1);
    let mut taken: Vec<ObjectId> = Vec::new();
    let mut dm = Script::new().attack_with(vec![attacker], bob()).objects(move |_, ctx| {
        let pick = ctx.candidates.iter().find(|c| c.legal && !taken.contains(&c.id)).map(|c| c.id)?;
        taken.push(pick);
        Some(vec![pick])
    });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(game.is_tapped(a) && game.is_tapped(b), "CR 702.154d: each enlist instance taps its own creature; log={:?}", log.borrow());
    assert_eq!(game.player(bob()).unwrap().life, 14, "CR 702.154d: 3 + 2 + 1 damage");
}

// ---------------------------------------------------------------------------
// 702.157 Squad
// ---------------------------------------------------------------------------

#[test]
fn cr_702_157a_squad_paid_twice_creates_two_token_copies() {
    let mut game = new_game();
    let guard = compile(
        "Ultramarines Honour Guard",
        "Mana cost: {3}{W}\nType: Creature — Astartes Warrior\nPower/Toughness: 2/2\nSquad {2}\nOther creatures you control get +1/+1.",
    );
    lands(&mut game, alice(), Subtype::Plains, 8);
    let card = put(&mut game, &guard, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .options(optional_costs(vec![0, 0]));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(
        by_name(&game, Zone::Battlefield, "Ultramarines Honour Guard").len(),
        3,
        "CR 702.157a: two squad payments create two token copies; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.165 Backup
// ---------------------------------------------------------------------------

#[test]
fn cr_702_165a_c_backup_grants_only_printed_abilities_below_it() {
    let mut game = new_game();
    let aetherborn = compile(
        "Consuming Aetherborn",
        "Mana cost: {3}{B}\nType: Creature — Aetherborn Vampire\nPower/Toughness: 2/2\nBackup 1\nLifelink",
    );
    let flier_lord = compile("Sky Anthem", "Mana cost: {2}\nType: Enchantment\nCreatures you control have flying.");
    lands(&mut game, alice(), Subtype::Swamp, 4);
    put(&mut game, &flier_lord, alice(), Zone::Battlefield);
    let target = vanilla(&mut game, bob(), "Opposing Bear", 2, 2);
    let card = put(&mut game, &aetherborn, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .targets(move |_, ctx| ctx.requirements.iter().any(|r| r.legal_targets.contains(&Target::Object(target))).then(|| vec![Target::Object(target)]));
    let log = dm.log_handle();
    let snapshots: Rc<RefCell<Vec<(bool, bool, u32)>>> = Rc::default();
    let snaps = snapshots.clone();
    dm = dm.observe(move |g, _| {
        if g.turn.phase == Phase::FirstMain || g.turn.phase == Phase::NextMain {
            let abilities = g.current_abilities(target).unwrap_or_default();
            let text = format!("{abilities:?}");
            snaps.borrow_mut().push((text.contains("Lifelink"), text.contains("Flying"), plus_counters(g, target)));
        }
    });
    run_turn(&mut game, &mut dm);
    let seen = snapshots.borrow().clone();
    assert!(
        seen.iter().any(|(life, _, counters)| *life && *counters == 1),
        "CR 702.165a: another target creature gets the counter and the printed lifelink; saw {seen:?} log={:?}",
        log.borrow()
    );
    assert!(
        !seen.iter().any(|(_, flying, _)| *flying),
        "CR 702.165c: abilities granted to the backup creature by other effects aren't granted"
    );
}

// ---------------------------------------------------------------------------
// 702.172 Spree / 702.183 Tiered
// ---------------------------------------------------------------------------

fn untapped_lands(game: &GameState, player: PlayerId) -> usize {
    game.objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            game.object(*id).is_some_and(|o| o.owner == player && o.card_types.contains(&CardType::Land))
                && !game.is_tapped(*id)
        })
        .count()
}

#[test]
fn cr_702_172a_spree_chosen_modes_add_their_costs() {
    let mut game = new_game();
    let derailment = compile(
        "Explosive Derailment",
        "Mana cost: {R}\nType: Instant\nSpree\n+ {2} — Explosive Derailment deals 4 damage to target creature.\n+ {2} — Destroy target artifact.",
    );
    lands(&mut game, alice(), Subtype::Mountain, 6);
    let creature = vanilla(&mut game, bob(), "Opposing Ogre", 4, 4);
    let relic = put(
        &mut game,
        &CardDefinitionBuilder::new(CardId::new(), "Opposing Relic").card_types(vec![CardType::Artifact]).build(),
        bob(),
        Zone::Battlefield,
    );
    let card = put(&mut game, &derailment, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .options(|_, ctx| {
            let modes: Vec<usize> = ctx.options.iter().filter(|o| o.description.contains("damage") || o.description.contains("Destroy")).map(|o| o.index).collect();
            (modes.len() == 2).then_some(modes)
        });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(by_name(&game, Zone::Battlefield, "Opposing Ogre").is_empty(), "CR 702.172a: first mode resolved; log={:?}", log.borrow());
    assert!(by_name(&game, Zone::Battlefield, "Opposing Relic").is_empty(), "CR 702.172a: second mode resolved");
    let _ = (creature, relic);
    assert_eq!(untapped_lands(&game, alice()), 1, "CR 702.172a: paid {{R}} plus {{2}} for each chosen mode (5 of 6 lands)");
}

#[test]
fn cr_702_183a_tiered_chooses_one_mode_and_pays_its_cost() {
    let mut game = new_game();
    let thunder = compile(
        "Thunder Magic",
        "Mana cost: {R}\nType: Instant\nTiered\n• Thunder — {0} — Thunder Magic deals 2 damage to target creature.\n• Thundara — {3} — Thunder Magic deals 4 damage to target creature.\n• Thundaga — {5}{R} — Thunder Magic deals 8 damage to target creature.",
    );
    lands(&mut game, alice(), Subtype::Mountain, 6);
    vanilla(&mut game, bob(), "Opposing Ogre", 4, 4);
    let card = put(&mut game, &thunder, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .options(|_, ctx| ctx.options.iter().find(|o| o.description.contains("Thundara") || o.description.contains("4 damage")).map(|o| vec![o.index]));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(by_name(&game, Zone::Battlefield, "Opposing Ogre").is_empty(), "CR 702.183a: the chosen mode resolved; log={:?}", log.borrow());
    assert_eq!(untapped_lands(&game, alice()), 2, "CR 702.183a: paid {{R}} plus the chosen tier's {{3}}");
    let menus = log.borrow().iter().filter(|l| l.contains("Thundara") && l.contains("max=1")).count();
    assert!(menus >= 1, "CR 702.183a: choose exactly one mode; log={:?}", log.borrow());
}

// ---------------------------------------------------------------------------
// 702.175 Offspring
// ---------------------------------------------------------------------------

#[test]
fn cr_702_175a_offspring_paid_creates_a_one_one_token_copy() {
    let mut game = new_game();
    let rampager = compile(
        "Rust-Shield Rampager",
        "Mana cost: {3}{G}\nType: Creature — Raccoon Warrior\nPower/Toughness: 4/4\nOffspring {2}\nThis creature can't be blocked by creatures with power 2 or less.",
    );
    lands(&mut game, alice(), Subtype::Forest, 6);
    let card = put(&mut game, &rampager, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let copies = by_name(&game, Zone::Battlefield, "Rust-Shield Rampager");
    assert_eq!(copies.len(), 2, "CR 702.175a: paid offspring creates a token copy; log={:?}", log.borrow());
    let mut pts: Vec<(i32, i32)> = copies.iter().map(|id| (game.calculated_power(*id).unwrap(), game.calculated_toughness(*id).unwrap())).collect();
    pts.sort();
    assert_eq!(pts, vec![(1, 1), (4, 4)], "CR 702.175a: the copy is 1/1");
}

#[test]
fn cr_702_175a_unpaid_offspring_creates_nothing() {
    let mut game = new_game();
    let rampager = compile(
        "Rust-Shield Rampager",
        "Mana cost: {3}{G}\nType: Creature — Raccoon Warrior\nPower/Toughness: 4/4\nOffspring {2}\nThis creature can't be blocked by creatures with power 2 or less.",
    );
    lands(&mut game, alice(), Subtype::Forest, 6);
    let card = put(&mut game, &rampager, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card)).booleans(|_, _| Some(false)).options(|_, ctx| {
        ctx.options.iter().find(|o| o.description.to_lowercase().contains("offspring")).map(|_| Vec::new()).filter(|_| ctx.min == 0)
    });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Battlefield, "Rust-Shield Rampager").len(), 1, "CR 702.175a: no token without payment; log={:?}", log.borrow());
}

#[test]
fn cr_702_175b_multiple_offspring_instances_trigger_per_payment() {
    let mut game = new_game();
    let card_def = compile(
        "Double Offspring",
        "Mana cost: {1}{G}\nType: Creature — Raccoon Warrior\nPower/Toughness: 3/3\nOffspring {1}\nOffspring {2}",
    );
    lands(&mut game, alice(), Subtype::Forest, 5);
    let card = put(&mut game, &card_def, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(
        by_name(&game, Zone::Battlefield, "Double Offspring").len(),
        3,
        "CR 702.175b: each paid offspring instance creates its own copy; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.176 Impending
// ---------------------------------------------------------------------------

#[test]
fn cr_702_176a_impending_enters_with_time_counters_as_noncreature() {
    let mut game = new_game();
    let overlord = compile(
        "Overlord of the Mistmoors",
        "Mana cost: {5}{W}{W}\nType: Enchantment Creature — Avatar Horror\nPower/Toughness: 6/6\nImpending 4—{2}{W}{W}\nWhenever this permanent enters or attacks, create two 2/1 white Insect creature tokens with flying.",
    );
    lands(&mut game, alice(), Subtype::Plains, 4);
    let card = put(&mut game, &overlord, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_with(a, card, &CastingMethod::Alternative(0)));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let permanent = by_name(&game, Zone::Battlefield, "Overlord of the Mistmoors");
    assert_eq!(permanent.len(), 1, "fixture: impending cast resolved; log={:?}", log.borrow());
    let id = permanent[0];
    assert_eq!(
        game.counter_count(id, CounterType::Time),
        3,
        "CR 702.176a: enters with four time counters, one removed at the beginning of your end step"
    );
    assert!(!is_creature_now(&game, id), "CR 702.176a: not a creature while it has a time counter");
    assert_eq!(with_subtype(&game, Zone::Battlefield, Subtype::Insect).len(), 2, "CR 702.176a: the permanent still entered (ETB trigger)");
    for _ in 0..6 {
        run_turn(&mut game, &mut Script::new());
    }
    assert_eq!(game.counter_count(id, CounterType::Time), 0, "CR 702.176a: counters removed each end step");
    assert!(is_creature_now(&game, id), "CR 702.176a: a creature again once the last counter is removed");
}

// ---------------------------------------------------------------------------
// 702.163 For Mirrodin! / 702.182 Job select
// ---------------------------------------------------------------------------

#[test]
fn cr_702_163a_702_182a_equipment_creates_token_and_attaches() {
    for (name, text, subtype, pt, color) in [
        (
            "Hexgold Halberd",
            "Mana cost: {1}{R}\nType: Artifact — Equipment\nFor Mirrodin!\nEquipped creature gets +1/+0.\nEquip {2}{R}",
            Subtype::Rebel,
            (3, 2),
            Some(ironsmith::Color::Red),
        ),
        (
            "Monk's Fist",
            "Mana cost: {2}\nType: Artifact — Equipment\nJob select\nEquipped creature gets +1/+0.\nEquip {2}",
            Subtype::Hero,
            (2, 1),
            None,
        ),
    ] {
        let mut game = new_game();
        let def = compile(name, text);
        lands(&mut game, alice(), Subtype::Mountain, 3);
        let card = put(&mut game, &def, alice(), Zone::Hand);
        let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        let equipment = by_name(&game, Zone::Battlefield, name);
        assert_eq!(equipment.len(), 1, "fixture: {name} resolved; log={:?}", log.borrow());
        let tokens = with_subtype(&game, Zone::Battlefield, subtype);
        assert_eq!(tokens.len(), 1, "CR 702.163a/702.182a: {name} creates its token; log={:?}", log.borrow());
        let token = tokens[0];
        assert_eq!(
            game.object(equipment[0]).unwrap().attached_to,
            Some(ironsmith::object::AttachmentTarget::Object(token)),
            "CR 702.163a/702.182a: then attach this Equipment to it"
        );
        assert_eq!((game.calculated_power(token).unwrap(), game.calculated_toughness(token).unwrap()), pt, "{name}: token base P/T plus equipment");
        let colors = game.current_characteristics(token).unwrap().colors;
        match color {
            Some(c) => assert!(colors.contains(c), "CR 702.163a: red Rebel"),
            None => assert!(colors.is_empty(), "CR 702.182a: colorless Hero"),
        }
    }
}

// ---------------------------------------------------------------------------
// 702.174 Gift
// ---------------------------------------------------------------------------

fn artifact(game: &mut GameState, player: PlayerId, name: &str) -> ObjectId {
    put(
        game,
        &CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Artifact]).build(),
        player,
        Zone::Battlefield,
    )
}

fn gift_answer(promise: bool) -> impl FnMut(&GameState, &BooleanContext) -> Option<bool> {
    move |_, ctx| ctx.description.to_lowercase().contains("gift").then_some(promise)
}

#[test]
fn cr_702_174a_e_j_k_m_promised_gift_card_happens_first_and_unlocks_targets() {
    let wear_down = "Mana cost: {1}{G}\nType: Sorcery\nGift a card\nDestroy target artifact or enchantment. If the gift was promised, instead destroy two target artifacts and/or enchantments.";
    for promise in [false, true] {
        let mut game = new_game();
        let def = compile("Wear Down", wear_down);
        lands(&mut game, alice(), Subtype::Forest, 2);
        artifact(&mut game, bob(), "Relic A");
        artifact(&mut game, bob(), "Relic B");
        let card = put(&mut game, &def, alice(), Zone::Hand);
        let bob_hand = hand_len(&game, bob());
        let mut dm = Script::new()
            .step(alice(), move |_, a| is_cast_of(a, card))
            .options(optional_costs(if promise { vec![0] } else { vec![] }));
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        let relics_left = by_name(&game, Zone::Battlefield, "Relic A").len() + by_name(&game, Zone::Battlefield, "Relic B").len();
        if promise {
            assert_eq!(hand_len(&game, bob()), bob_hand + 1, "CR 702.174e: the chosen opponent draws a card; log={:?}", log.borrow());
            assert_eq!(relics_left, 0, "CR 702.174k/m: promised gift unlocks the two-target mode; log={:?}", log.borrow());
        } else {
            assert_eq!(hand_len(&game, bob()), bob_hand, "CR 702.174a: no gift effect without promise; log={:?}", log.borrow());
            assert_eq!(relics_left, 1, "CR 702.174m: only the unconditional target is chosen; log={:?}", log.borrow());
        }
    }
}

#[test]
fn cr_702_174d_f_h_gift_token_kinds_go_to_the_opponent() {
    let cases: [(&str, &str, Subtype, bool); 3] = [
        (
            "Crumb and Get It",
            "Mana cost: {W}\nType: Instant\nGift a Food\nTarget creature you control gets +2/+2 until end of turn. If the gift was promised, that creature also gains indestructible until end of turn.",
            Subtype::Food,
            false,
        ),
        (
            "Into the Flood Maw",
            "Mana cost: {U}\nType: Instant\nGift a tapped Fish\nReturn target creature an opponent controls to its owner's hand. If the gift was promised, instead return target nonland permanent an opponent controls to its owner's hand.",
            Subtype::Fish,
            true,
        ),
        (
            "Blooming Blast",
            "Mana cost: {1}{R}\nType: Instant\nGift a Treasure\nBlooming Blast deals 2 damage to target creature. If the gift was promised, Blooming Blast also deals 3 damage to that creature's controller.",
            Subtype::Treasure,
            false,
        ),
    ];
    for (name, text, token_type, tapped_expected) in cases {
        let mut game = new_game();
        let def = compile(name, text);
        for subtype in [Subtype::Plains, Subtype::Island, Subtype::Mountain] {
            lands(&mut game, alice(), subtype, 2);
        }
        vanilla(&mut game, alice(), "Own Bear", 2, 2);
        vanilla(&mut game, bob(), "Their Bear", 2, 2);
        let card = put(&mut game, &def, alice(), Zone::Hand);
        let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card)).options(optional_costs(vec![0]));
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        let tokens: Vec<ObjectId> = with_subtype(&game, Zone::Battlefield, token_type);
        assert_eq!(tokens.len(), 1, "CR 702.174d/f/h: {name} creates exactly one {token_type:?}; log={:?}", log.borrow());
        let token = tokens[0];
        assert_eq!(game.object(token).unwrap().owner, bob(), "CR 702.174d/f/h: the chosen player creates the token");
        if token_type == Subtype::Fish {
            let chars = game.current_characteristics(token).unwrap();
            assert!(chars.colors.contains(ironsmith::Color::Blue), "CR 702.174f: blue Fish");
            assert_eq!((chars.power, chars.toughness), (Some(1), Some(1)), "CR 702.174f: 1/1 Fish");
            // The Fish was created during Alice's turn; untap only happens in Bob's untap step, which
            // has not occurred (run_turn ended Alice's turn only).
            assert_eq!(game.is_tapped(token), tapped_expected, "CR 702.174f: the Fish enters tapped");
        }
    }
}

#[test]
fn cr_702_174b_i_permanent_gift_triggers_on_entry_octopus() {
    let mut game = new_game();
    let def = compile(
        "Octomancer",
        "Mana cost: {3}{G}{U}\nType: Creature — Frog Druid\nPower/Toughness: 3/3\nGift an Octopus\nAt the beginning of each end step, create a token that's a copy of target creature token that entered the battlefield this turn.",
    );
    lands(&mut game, alice(), Subtype::Forest, 4);
    lands(&mut game, alice(), Subtype::Island, 1);
    let card = put(&mut game, &def, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card)).options(optional_costs(vec![0]));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let octopi: Vec<ObjectId> = with_subtype(&game, Zone::Battlefield, Subtype::Octopus)
        .into_iter()
        .filter(|id| game.object(*id).unwrap().owner == bob())
        .collect();
    assert!(!octopi.is_empty(), "CR 702.174b/i: when it enters, the chosen player creates an Octopus; log={:?}", log.borrow());
    let chars = game.current_characteristics(octopi[0]).unwrap();
    assert_eq!((chars.power, chars.toughness), (Some(8), Some(8)), "CR 702.174i: 8/8 Octopus");
    assert!(chars.colors.contains(ironsmith::Color::Blue), "CR 702.174i: blue Octopus");
}

#[test]
fn cr_702_174g_gift_an_extra_turn_gives_the_opponent_an_extra_turn() {
    let mut game = new_game();
    let def = compile(
        "Perch Protection",
        "Mana cost: {4}{W}{W}\nType: Instant\nGift an extra turn\nCreate four 2/2 blue Bird creature tokens with flying. If the gift was promised, all permanents you control phase out, and until your next turn, your life total can't change and you gain protection from everything.\nExile Perch Protection.",
    );
    lands(&mut game, alice(), Subtype::Plains, 6);
    let card = put(&mut game, &def, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card)).options(optional_costs(vec![0]));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(game.turn.active_player, bob(), "fixture: Bob's turn follows; log={:?}", log.borrow());
    run_turn(&mut game, &mut Script::new());
    assert_eq!(game.turn.active_player, bob(), "CR 702.174g: the chosen player takes an extra turn after this one");
}

#[test]
fn cr_702_174c_giving_a_gift_triggers_on_spell_resolution() {
    let mut game = new_game();
    let gerbils = compile(
        "Jolly Gerbils",
        "Mana cost: {1}{W}\nType: Creature — Hamster Citizen\nPower/Toughness: 2/3\nWhenever you give a gift, draw a card.",
    );
    let blast = compile(
        "Blooming Blast",
        "Mana cost: {1}{R}\nType: Instant\nGift a Treasure\nBlooming Blast deals 2 damage to target creature. If the gift was promised, Blooming Blast also deals 3 damage to that creature's controller.",
    );
    put(&mut game, &gerbils, alice(), Zone::Battlefield);
    lands(&mut game, alice(), Subtype::Mountain, 2);
    let target = vanilla(&mut game, bob(), "Their Bear", 2, 2);
    let card = put(&mut game, &blast, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .options(optional_costs(vec![0]))
        .targets(move |_, ctx| ctx.requirements.iter().any(|r| r.legal_targets.contains(&Target::Object(target))).then(|| vec![Target::Object(target)]));
    let log = dm.log_handle();
    let hand_before = hand_len(&game, alice());
    run_turn(&mut game, &mut dm);
    // -1 cast, +1 draw step, +1 Jolly Gerbils.
    assert_eq!(hand_len(&game, alice()), hand_before + 1, "CR 702.174c: you gave a gift when the promised spell resolved; log={:?}", log.borrow());
    assert_eq!(game.player(bob()).unwrap().life, 17, "fixture: promised branch resolved");
}

#[test]
fn cr_702_174j_countered_gift_spell_gives_no_gift() {
    let mut game = new_game();
    let blast = compile(
        "Blooming Blast",
        "Mana cost: {1}{R}\nType: Instant\nGift a Treasure\nBlooming Blast deals 2 damage to target creature. If the gift was promised, Blooming Blast also deals 3 damage to that creature's controller.",
    );
    let counterspell = compile("Cancel", "Mana cost: {U}{U}\nType: Instant\nCounter target spell.");
    lands(&mut game, alice(), Subtype::Mountain, 2);
    lands(&mut game, bob(), Subtype::Island, 2);
    vanilla(&mut game, bob(), "Their Bear", 2, 2);
    let card = put(&mut game, &blast, alice(), Zone::Hand);
    let counter = put(&mut game, &counterspell, bob(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .step(bob(), move |g, a| is_cast_of(a, counter) && !g.stack.is_empty())
        .options(optional_costs(vec![0]));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Graveyard, "Cancel").len(), 1, "fixture: counterspell resolved; log={:?}", log.borrow());
    assert!(
        with_subtype(&game, Zone::Battlefield, Subtype::Treasure).is_empty(),
        "CR 702.174j: a countered gift spell's gift effect doesn't happen"
    );
}

// ---------------------------------------------------------------------------
// 702.166 Bargain
// ---------------------------------------------------------------------------

#[test]
fn cr_702_166a_b_c_bargained_permanent_uses_its_linked_ability() {
    for bargain in [false, true] {
        let mut game = new_game();
        let ouphe = compile(
            "Troublemaker Ouphe",
            "Mana cost: {1}{G}\nType: Creature — Ouphe\nPower/Toughness: 2/2\nBargain\nWhen this creature enters, if it was bargained, exile target artifact or enchantment an opponent controls.",
        );
        lands(&mut game, alice(), Subtype::Forest, 2);
        let fodder = artifact(&mut game, alice(), "Own Trinket");
        artifact(&mut game, bob(), "Their Relic");
        let card = put(&mut game, &ouphe, alice(), Zone::Hand);
        let mut dm = Script::new()
            .step(alice(), move |_, a| is_cast_of(a, card))
            .options(optional_costs(if bargain { vec![0] } else { vec![] }))
            .objects(move |_, ctx| {
                if !bargain {
                    return (ctx.min == 0).then(Vec::new);
                }
                ctx.candidates.iter().any(|c| c.id == fodder).then(|| vec![fodder])
            });
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        let relic_gone = by_name(&game, Zone::Battlefield, "Their Relic").is_empty();
        let trinket_gone = by_name(&game, Zone::Battlefield, "Own Trinket").is_empty();
        assert_eq!(trinket_gone, bargain, "CR 702.166a: bargain sacrifices only when chosen; log={:?}", log.borrow());
        assert_eq!(relic_gone, bargain, "CR 702.166b/c: the linked ETB ability checks whether it was bargained; log={:?}", log.borrow());
    }
}

#[test]
fn cr_702_166d_bargained_only_target_is_chosen_only_if_bargained() {
    for bargain in [false, true] {
        let mut game = new_game();
        let spell = compile(
            "Bargain Probe",
            "Mana cost: {1}{R}\nType: Sorcery\nBargain\nYou gain 2 life. If this spell was bargained, destroy target artifact an opponent controls.",
        );
        lands(&mut game, alice(), Subtype::Mountain, 2);
        let fodder = artifact(&mut game, alice(), "Own Trinket");
        artifact(&mut game, bob(), "Their Relic");
        let card = put(&mut game, &spell, alice(), Zone::Hand);
        let mut dm = Script::new()
            .step(alice(), move |_, a| is_cast_of(a, card))
            .options(optional_costs(if bargain { vec![0] } else { vec![] }))
            .objects(move |_, ctx| {
                if !bargain {
                    return (ctx.min == 0).then(Vec::new);
                }
                ctx.candidates.iter().any(|c| c.id == fodder).then(|| vec![fodder])
            });
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        assert_eq!(game.player(alice()).unwrap().life, 22, "fixture: spell resolved (bargain={bargain}); log={:?}", log.borrow());
        let target_prompted = log.borrow().iter().any(|l| l.starts_with("targets:") && l.contains("Bargain Probe"));
        assert_eq!(target_prompted, bargain, "CR 702.166d: targets for the bargained part are chosen only if bargained; log={:?}", log.borrow());
        assert_eq!(by_name(&game, Zone::Battlefield, "Their Relic").is_empty(), bargain, "CR 702.166d: bargained branch only");
    }
}

#[test]
fn cr_702_189b_firebend_trigger_fires_when_firebending_resolves() {
    let mut game = new_game();
    let aang = compile(
        "Elemental Student",
        "Mana cost: {2}{R}\nType: Creature — Human Ally\nPower/Toughness: 2/2\nFirebending 1\nWhenever you waterbend, earthbend, firebend, or airbend, draw a card.",
    );
    let attacker = put(&mut game, &aang, alice(), Zone::Battlefield);
    let mut dm = Script::new().attack_with(vec![attacker], bob());
    let log = dm.log_handle();
    let hand_before = hand_len(&game, alice());
    run_turn(&mut game, &mut dm);
    assert_eq!(hand_len(&game, alice()), hand_before + 2, "CR 702.189b: draw step plus the firebend trigger; log={:?}", log.borrow());
}

// ---------------------------------------------------------------------------
// 702.170 Plot
// ---------------------------------------------------------------------------

fn is_plot_of(action: &LegalAction, card: ObjectId) -> bool {
    matches!(action, LegalAction::SpecialAction(ironsmith::SpecialAction::Plot { card_id }) if *card_id == card)
}

#[test]
fn cr_702_170a_b_d_plot_special_action_then_free_cast_on_a_later_turn() {
    let mut game = new_game();
    let paladin = compile(
        "Spinewoods Paladin",
        "Mana cost: {4}{G}\nType: Creature — Human Knight\nPower/Toughness: 5/4\nTrample\nWhen this creature enters, you gain 3 life.\nPlot {3}{G}",
    );
    lands(&mut game, alice(), Subtype::Forest, 4);
    let card = put(&mut game, &paladin, alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let mut dm = Script::new().step(alice(), move |_, a| is_plot_of(a, card));
    let seen = dm.seen.clone();
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "CR 702.170a: plot exiles the card; log={:?}", log.borrow());
    assert_eq!(untapped_lands(&game, alice()), 0, "CR 702.170a: plot cost paid");
    let menus = seen.borrow().clone();
    let plot_at = menus.iter().position(|(_, _, acts)| acts.iter().any(|a| is_plot_of(a, card))).unwrap();
    let (step, player, _) = &menus[plot_at + 1];
    assert!(
        *player == alice() && *step == menus[plot_at].0,
        "CR 702.170b: plotting doesn't use the stack, so its player keeps priority in the same step"
    );
    assert!(
        !menus.iter().any(|(_, _, acts)| acts.iter().any(|a| is_cast_of(a, exiled))),
        "CR 702.170d: not castable the turn it became plotted"
    );
    let mut bob_dm = Script::new();
    let bob_seen = bob_dm.seen.clone();
    run_turn(&mut game, &mut bob_dm);
    assert!(
        !bob_seen.borrow().iter().any(|(_, _, acts)| acts.iter().any(|a| is_cast_of(a, exiled))),
        "CR 702.170d: only during the owner's main phase"
    );
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, exiled));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Battlefield, "Spinewoods Paladin").len(), 1, "CR 702.170d: cast from exile on a later turn; log={:?}", log.borrow());
    assert_eq!(game.player(alice()).unwrap().life, 23, "fixture: ETB resolved");
    assert_eq!(untapped_lands(&game, alice()), 4, "CR 702.170d: cast without paying its mana cost");
}

#[test]
fn cr_702_170c_effect_can_make_an_exiled_card_plotted() {
    let mut game = new_game();
    let luck = compile(
        "Make Your Own Luck",
        "Mana cost: {3}{G}{U}\nType: Sorcery\nLook at the top three cards of your library. You may exile a nonland card from among them. If you do, it becomes plotted. Put the rest into your hand.",
    );
    lands(&mut game, alice(), Subtype::Forest, 4);
    lands(&mut game, alice(), Subtype::Island, 1);
    let card = put(&mut game, &luck, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let exiled: Vec<ObjectId> = by_name(&game, Zone::Exile, "Library Filler");
    assert_eq!(exiled.len(), 1, "fixture: one card exiled; log={:?}", log.borrow());
    run_turn(&mut game, &mut Script::new());
    let target = exiled[0];
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, target));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(by_name(&game, Zone::Exile, "Library Filler").is_empty(), "CR 702.170c: the card became plotted and was cast later; log={:?}", log.borrow());
}

#[test]
fn cr_702_170e_f_plot_from_another_zone_exiles_from_that_zone() {
    let mut game = new_game();
    let fblthp = compile(
        "Fblthp, Lost on the Range",
        "Mana cost: {1}{U}{U}\nType: Legendary Creature — Homunculus\nPower/Toughness: 1/1\nWard {2}\nYou may look at the top card of your library any time.\nThe top card of your library has plot. The plot cost is equal to its mana cost.\nYou may plot nonland cards from the top of your library.",
    );
    put(&mut game, &fblthp, alice(), Zone::Battlefield);
    lands(&mut game, alice(), Subtype::Forest, 3);
    let top_def = compile("Top Bear", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    game.create_object_from_definition(&top_def, alice(), Zone::Library);
    let top = game.create_object_from_definition(&top_def, alice(), Zone::Library);
    // Library order: make sure the bear is on top.
    let library = game.player(alice()).unwrap().library.clone();
    let on_top = library.iter().last().copied() == Some(top) || library.iter().next().copied() == Some(top);
    assert!(on_top, "fixture: new card is at an end of the library");
    let mut dm = Script::new();
    dm.plan.push((alice(), Box::new(|g: &GameState, a: &LegalAction| matches!(a, LegalAction::SpecialAction(ironsmith::SpecialAction::Plot { card_id }) if g.object(*card_id).is_some_and(|o| o.zone == Zone::Library)))));
    let seen = dm.seen.clone();
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let plotted_from_library = seen.borrow().iter().any(|(_, _, acts)| {
        acts.iter().any(|a| matches!(a, LegalAction::SpecialAction(ironsmith::SpecialAction::Plot { .. })))
    });
    assert!(plotted_from_library, "CR 702.170f: plot may function from the library; log={:?}", log.borrow());
    assert_eq!(
        game.objects_in_zone(Zone::Exile).len(),
        1,
        "CR 702.170e/f: the card is exiled from the library as the plot action is taken; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.171 Saddle
// ---------------------------------------------------------------------------

#[test]
fn cr_702_171a_b_c_saddle_taps_power_and_marks_saddled_until_end_of_turn() {
    let mut game = new_game();
    let mustang = compile(
        "Brightfield Mustang",
        "Mana cost: {3}{W}\nType: Creature — Horse Mount\nPower/Toughness: 3/3\nWhenever this creature attacks while saddled, untap it and put a +1/+1 counter on it.\nSaddle 1",
    );
    let mount = put(&mut game, &mustang, alice(), Zone::Battlefield);
    let rider = vanilla(&mut game, alice(), "Rider", 1, 1);
    let zero = vanilla(&mut game, alice(), "Zero", 0, 1);
    let mut dm = Script::new()
        .step(alice(), move |g, a| is_activation(a, mount) && g.turn.phase == Phase::FirstMain)
        .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == rider).then(|| vec![rider]))
        .attack_with(vec![mount], bob());
    let seen = dm.seen.clone();
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(game.is_tapped(rider), "CR 702.171a/c: the rider was tapped to saddle; log={:?}", log.borrow());
    assert!(!game.is_tapped(zero), "fixture: only the chosen rider tapped");
    assert_eq!(plus_counters(&game, mount), 1, "CR 702.171b: attacked while saddled");
    assert!(!game.is_tapped(mount), "fixture: trigger untapped the mount");
    let combat_offer = seen.borrow().iter().any(|(step, _, acts)| {
        step.is_some_and(|s| matches!(s, Step::DeclareAttackers | Step::DeclareBlockers)) && acts.iter().any(|a| is_activation(a, mount))
    });
    assert!(!combat_offer, "CR 702.171a: saddle only as a sorcery");
    run_turn(&mut game, &mut Script::new());
    let mut dm = Script::new().attack_with(vec![mount], bob());
    run_turn(&mut game, &mut dm);
    assert_eq!(plus_counters(&game, mount), 1, "CR 702.171b: saddled ends at end of turn");
}

#[test]
fn cr_702_171a_saddle_requires_enough_total_power_of_other_creatures() {
    let mut game = new_game();
    let sunmare = compile(
        "Guardian Sunmare",
        "Mana cost: {3}{W}{W}\nType: Creature — Horse Mount\nPower/Toughness: 5/5\nWard {2}\nWhenever this creature attacks while saddled, you gain 4 life.\nSaddle 4",
    );
    let mount = put(&mut game, &sunmare, alice(), Zone::Battlefield);
    vanilla(&mut game, alice(), "Small Rider", 3, 3);
    let mut dm = Script::new();
    let seen = dm.seen.clone();
    run_turn(&mut game, &mut dm);
    assert!(
        !seen.borrow().iter().any(|(_, _, acts)| acts.iter().any(|a| is_activation(a, mount))),
        "CR 702.171a: other creatures with total power 3 can't pay saddle 4 (the mount's own 5 power doesn't count)"
    );
}

// ---------------------------------------------------------------------------
// 702.185 Warp
// ---------------------------------------------------------------------------

fn cast_alternative(card: ObjectId) -> impl Fn(&GameState, &LegalAction) -> bool {
    move |_, a| matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(_), .. } if *spell_id == card)
}

#[test]
fn cr_702_185a_b_c_warp_cast_exiles_at_end_step_and_counts_as_warped() {
    let mut game = new_game();
    let mechan = compile(
        "Red Tiger Mechan",
        "Mana cost: {3}{R}\nType: Artifact Creature — Robot Cat\nPower/Toughness: 3/3\nHaste\nWarp {1}{R}",
    );
    let bolt = compile(
        "Plasma Bolt",
        "Mana cost: {R}\nType: Sorcery\nPlasma Bolt deals 2 damage to any target.\nVoid — Plasma Bolt deals 3 damage instead if a nonland permanent left the battlefield this turn or a spell was warped this turn.",
    );
    lands(&mut game, alice(), Subtype::Mountain, 3);
    let card = put(&mut game, &mechan, alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let bolt_card = put(&mut game, &bolt, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), cast_alternative(card))
        .step(alice(), move |g, a| is_cast_of(a, bolt_card) && g.stack.is_empty())
        .targets(|_, _| Some(vec![Target::Player(PlayerId::from_index(1))]))
        .attackers_none();
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(game.player(bob()).unwrap().life, 17, "CR 702.185c: \"a spell was warped this turn\"; log={:?}", log.borrow());
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Exile, "CR 702.185a: exiled at the beginning of the next end step; log={:?}", log.borrow());
    run_turn(&mut game, &mut Script::new());
    // CR 702.185a: "then you may cast it from exile on a later turn" (paying its normal cost).
    lands(&mut game, alice(), Subtype::Mountain, 1);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, now));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Battlefield, "Red Tiger Mechan").len(), 1, "CR 702.185b: the warped card in exile can be cast later; log={:?}", log.borrow());
}

// ---------------------------------------------------------------------------
// 702.187 Mayhem
// ---------------------------------------------------------------------------

#[test]
fn cr_702_187a_b_mayhem_casts_only_a_card_discarded_this_turn() {
    let mut game = new_game();
    let islanders = compile(
        "Spider-Islanders",
        "Mana cost: {3}{R}\nType: Creature — Spider Horror Citizen\nPower/Toughness: 4/3\nMayhem {1}{R}",
    );
    let looter = compile("Discard Engine", "Mana cost: {0}\nType: Artifact\n{0}: Draw a card, then discard a card.");
    lands(&mut game, alice(), Subtype::Mountain, 4);
    let engine = put(&mut game, &looter, alice(), Zone::Battlefield);
    let milled = put(&mut game, &islanders, alice(), Zone::Graveyard);
    let in_hand = put(&mut game, &islanders, alice(), Zone::Hand);
    let in_hand_stable = game.object(in_hand).unwrap().stable_id;
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, engine))
        .step(alice(), move |g, a| {
            matches!(a, LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, .. }
                if g.object(*spell_id).is_some_and(|o| o.stable_id == in_hand_stable))
        })
        .objects(move |g, ctx| {
            ctx.candidates.iter().find(|c| g.object(c.id).is_some_and(|o| o.stable_id == in_hand_stable)).map(|c| vec![c.id])
        });
    let seen = dm.seen.clone();
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(
        !seen.borrow().iter().any(|(_, _, acts)| acts.iter().any(|a| is_cast_of(a, milled))),
        "CR 702.187b: a card not discarded this turn can't be cast with mayhem"
    );
    assert_eq!(by_name(&game, Zone::Battlefield, "Spider-Islanders").len(), 1, "CR 702.187a/b: the discarded card was cast from the graveyard; log={:?}", log.borrow());
    assert_eq!(untapped_lands(&game, alice()), 2, "CR 702.187b: paid the mayhem cost {{1}}{{R}}");
}

#[test]
fn cr_702_187c_costless_mayhem_plays_a_discarded_land() {
    let mut game = new_game();
    let oscorp = compile(
        "Oscorp Industries",
        "Type: Land\nThis land enters tapped.\nWhen this land enters from a graveyard, you lose 2 life.\n{T}: Add {U}, {B}, or {R}.\nMayhem",
    );
    let looter = compile("Discard Engine", "Mana cost: {0}\nType: Artifact\n{0}: Draw a card, then discard a card.");
    let engine = put(&mut game, &looter, alice(), Zone::Battlefield);
    let in_hand = put(&mut game, &oscorp, alice(), Zone::Hand);
    let stable = game.object(in_hand).unwrap().stable_id;
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, engine))
        .step(alice(), move |g, a| match a {
            LegalAction::PlayLand { land_id } => g.object(*land_id).is_some_and(|o| o.stable_id == stable && o.zone == Zone::Graveyard),
            _ => false,
        })
        .objects(move |g, ctx| ctx.candidates.iter().find(|c| g.object(c.id).is_some_and(|o| o.stable_id == stable)).map(|c| vec![c.id]));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Battlefield, "Oscorp Industries").len(), 1, "CR 702.187c: play the discarded land from the graveyard; log={:?}", log.borrow());
    assert_eq!(game.player(alice()).unwrap().life, 18, "fixture: entered from a graveyard");
}

// ---------------------------------------------------------------------------
// 702.188 Web-slinging
// ---------------------------------------------------------------------------

#[test]
fn cr_702_188a_web_slinging_returns_a_tapped_creature_as_alternative_cost() {
    let mut game = new_game();
    let spidey = compile(
        "Spider-Man, Web-Slinger",
        "Mana cost: {2}{W}\nType: Legendary Creature — Spider Human Hero\nPower/Toughness: 3/3\nWeb-slinging {W}",
    );
    lands(&mut game, alice(), Subtype::Plains, 3);
    let untapped = vanilla(&mut game, alice(), "Untapped Friend", 1, 1);
    let friend = compile("Tapped Friend", "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1\n{T}: You gain 1 life.");
    let tapped = put(&mut game, &friend, alice(), Zone::Battlefield);
    let card = put(&mut game, &spidey, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, tapped))
        .step(alice(), cast_alternative(card))
        .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == tapped).then(|| vec![tapped]));
    let log = dm.log_handle();
    let seen = dm.seen.clone();
    run_turn(&mut game, &mut dm);
    let offered: Vec<LegalAction> = seen.borrow().iter().flat_map(|(_, _, acts)| acts.iter().filter(|a| is_cast_of(a, card)).cloned()).collect();
    assert_eq!(
        by_name(&game, Zone::Battlefield, "Spider-Man, Web-Slinger").len(),
        1,
        "CR 702.188a: web-slinging cast is available and resolves; offered casts={offered:?} alternatives={:?} log={:?}",
        web_def_alts(&spidey),
        log.borrow()
    );
    assert_eq!(by_name(&game, Zone::Hand, "Tapped Friend").len(), 1, "CR 702.188a: the tapped creature returns to its owner's hand");
    assert!(by_name(&game, Zone::Battlefield, "Untapped Friend").len() == 1, "fixture: untapped creature stays");
    let _ = untapped;
    assert_eq!(untapped_lands(&game, alice()), 2, "CR 702.188a: paid {{W}} rather than the mana cost");
}

#[test]
fn cr_702_188a_web_slinging_needs_a_tapped_creature() {
    let mut game = new_game();
    let spidey = compile(
        "Spider-Man, Web-Slinger",
        "Mana cost: {2}{W}\nType: Legendary Creature — Spider Human Hero\nPower/Toughness: 3/3\nWeb-slinging {W}",
    );
    lands(&mut game, alice(), Subtype::Plains, 1);
    vanilla(&mut game, alice(), "Untapped Friend", 1, 1);
    let card = put(&mut game, &spidey, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), cast_alternative(card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Hand, "Spider-Man, Web-Slinger").len(), 1, "CR 702.188a: no tapped creature, no web-slinging cast; log={:?}", log.borrow());
}

fn web_def_alts(def: &CardDefinition) -> String {
    format!("{:?}", def.alternative_casts)
}

// ---------------------------------------------------------------------------
// 702.190 Sneak
// ---------------------------------------------------------------------------

#[test]
fn cr_702_190a_sneak_casts_sorcery_in_declare_blockers_returning_unblocked_attacker() {
    let mut game = new_game();
    let technique = compile(
        "Donatello's Technique",
        "Mana cost: {2}{U}\nType: Sorcery\nSneak {U}\nDraw two cards.",
    );
    lands(&mut game, alice(), Subtype::Island, 1);
    let attacker = vanilla(&mut game, alice(), "Ninja", 2, 2);
    let card = put(&mut game, &technique, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |g, a| cast_alternative(card)(g, a) && g.turn.step == Some(Step::DeclareBlockers))
        .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == attacker).then(|| vec![attacker]))
        .attack_with(vec![attacker], bob());
    let log = dm.log_handle();
    let hand_before = hand_len(&game, alice());
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Hand, "Ninja").len(), 1, "CR 702.190a: the unblocked attacker returned to hand; log={:?}", log.borrow());
    assert_eq!(game.player(bob()).unwrap().life, 20, "fixture: returned attacker dealt no damage");
    // -1 technique, +1 Ninja, +1 draw step, +2 technique
    assert_eq!(hand_len(&game, alice()), hand_before + 3, "CR 702.190a: the sorcery resolved during combat");
}

#[test]
fn cr_702_190b_sneaked_permanent_enters_tapped_and_attacking_the_same_player() {
    let mut game = new_game();
    let leonardo = compile(
        "Leonardo, Leader in Blue",
        "Mana cost: {W}\nType: Legendary Creature — Mutant Ninja Turtle\nPower/Toughness: 2/1\nSneak {3}{W}{W}\n{1}{W}: Leonardo gains first strike until end of turn.",
    );
    lands(&mut game, alice(), Subtype::Plains, 5);
    let attacker = vanilla(&mut game, alice(), "Ninja", 1, 1);
    let card = put(&mut game, &leonardo, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |g, a| cast_alternative(card)(g, a) && g.turn.step == Some(Step::DeclareBlockers))
        .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == attacker).then(|| vec![attacker]))
        .attack_with(vec![attacker], bob());
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let leo = by_name(&game, Zone::Battlefield, "Leonardo, Leader in Blue");
    assert_eq!(leo.len(), 1, "fixture: sneak cast resolved; log={:?}", log.borrow());
    assert_eq!(game.player(bob()).unwrap().life, 18, "CR 702.190b: entered attacking the same player and dealt combat damage");
}

// ---------------------------------------------------------------------------
// 702.191 Increment
// ---------------------------------------------------------------------------

#[test]
fn cr_702_191a_b_increment_compares_mana_spent_to_power_or_toughness() {
    for (spent, instances, expected) in [(3u32, 1usize, 0u32), (4, 1, 1), (6, 2, 2)] {
        let mut game = new_game();
        let increment_line = vec!["Increment"; instances].join("\n");
        let graffalon = compile(
            "Hungry Graffalon",
            &format!("Mana cost: {{3}}{{G}}\nType: Creature — Giraffe\nPower/Toughness: 3/4\nReach\n{increment_line}"),
        );
        let spell = compile("Mana Sink", &format!("Mana cost: {{{spent}}}\nType: Sorcery\nYou gain 1 life."));
        let creature = put(&mut game, &graffalon, alice(), Zone::Battlefield);
        lands(&mut game, alice(), Subtype::Forest, spent as usize);
        let card = put(&mut game, &spell, alice(), Zone::Hand);
        let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        assert_eq!(game.player(alice()).unwrap().life, 21, "fixture: spell resolved; log={:?}", log.borrow());
        assert_eq!(
            plus_counters(&game, creature),
            expected,
            "CR 702.191a/b: spent={spent} instances={instances}; ability labels={:?}; log={:?}",
            game.object(creature).unwrap().ability_labels.iter().cloned().collect::<Vec<_>>(),
            log.borrow()
        );
    }
}

// ---------------------------------------------------------------------------
// 702.193 Power-up
// ---------------------------------------------------------------------------

#[test]
fn cr_702_193a_b_power_up_reduced_by_mana_cost_on_entry_turn_and_once_only() {
    let mut game = new_game();
    let villain = compile(
        "Volcanic Villain",
        "Mana cost: {2}{R}\nType: Creature — Elemental Villain\nPower/Toughness: 3/2\nHaste\nPower-up — {5}{R}: Put two +1/+1 counters on this creature.",
    );
    // One Mountain only: after casting the villain, the reduced power-up cost must be
    // payable with three Islands, so {R} in the mana cost reduced the {R} of the cost.
    lands(&mut game, alice(), Subtype::Mountain, 1);
    lands(&mut game, alice(), Subtype::Island, 5);
    let card = put(&mut game, &villain, alice(), Zone::Hand);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .step(alice(), move |g, a| match a {
            LegalAction::ActivateAbility { source, .. } => g.object(*source).is_some_and(|o| &*o.name == "Volcanic Villain"),
            _ => false,
        });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let creature = by_name(&game, Zone::Battlefield, "Volcanic Villain");
    assert_eq!(creature.len(), 1, "fixture: villain resolved; log={:?}", log.borrow());
    let creature = creature[0];
    assert_eq!(plus_counters(&game, creature), 2, "CR 702.193a/b: power-up cost reduced by {{2}}{{R}} to {{3}}; log={:?}", log.borrow());
    run_turn(&mut game, &mut Script::new());
    lands(&mut game, alice(), Subtype::Mountain, 6);
    let mut dm = Script::new().step(alice(), move |_, a| is_activation(a, creature));
    run_turn(&mut game, &mut dm);
    assert_eq!(plus_counters(&game, creature), 2, "CR 702.193a: activate power-up only once");
}

#[test]
fn cr_702_193a_power_up_not_reduced_when_permanent_did_not_enter_this_turn() {
    let mut game = new_game();
    let villain = compile(
        "Volcanic Villain",
        "Mana cost: {2}{R}\nType: Creature — Elemental Villain\nPower/Toughness: 3/2\nHaste\nPower-up — {5}{R}: Put two +1/+1 counters on this creature.",
    );
    let creature = put(&mut game, &villain, alice(), Zone::Battlefield);
    lands(&mut game, alice(), Subtype::Mountain, 4);
    let mut dm = Script::new().step(alice(), move |_, a| is_activation(a, creature));
    run_turn(&mut game, &mut dm);
    assert_eq!(plus_counters(&game, creature), 0, "CR 702.193a: full {{5}}{{R}} cost when it didn't enter this turn");
}

// ---------------------------------------------------------------------------
// 702.194 Teamwork
// ---------------------------------------------------------------------------

#[test]
fn cr_702_194a_b_teamwork_taps_power_and_unlocks_its_rider() {
    for teamwork in [false, true] {
        let mut game = new_game();
        let blast = compile(
            "Repulsor Blast",
            "Mana cost: {3}{R}\nType: Sorcery\nTeamwork 2\nRepulsor Blast deals 5 damage to target creature. If this spell was cast using teamwork, it also deals 2 damage to that creature's controller.",
        );
        lands(&mut game, alice(), Subtype::Mountain, 4);
        let helper = vanilla(&mut game, alice(), "Teammate", 2, 2);
        let ogre = vanilla(&mut game, bob(), "Their Ogre", 5, 5);
        let card = put(&mut game, &blast, alice(), Zone::Hand);
        let mut dm = Script::new()
            .step(alice(), move |_, a| is_cast_of(a, card))
            .targets(move |_, _| Some(vec![Target::Object(ogre)]))
            .options(optional_costs(if teamwork { vec![0] } else { vec![] }))
            .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == helper).then(|| vec![helper]));
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        assert!(by_name(&game, Zone::Battlefield, "Their Ogre").is_empty(), "fixture: spell resolved; log={:?}", log.borrow());
        assert_eq!(game.is_tapped(helper), teamwork, "CR 702.194a: teamwork taps creatures with total power N; log={:?}", log.borrow());
        assert_eq!(game.player(bob()).unwrap().life, if teamwork { 18 } else { 20 }, "CR 702.194b: \"cast using teamwork\" rider");
    }
}

#[test]
fn cr_702_194c_teamwork_only_target_chosen_only_with_teamwork() {
    for teamwork in [false, true] {
        let mut game = new_game();
        let spell = compile(
            "Teamwork Probe",
            "Mana cost: {1}{R}\nType: Sorcery\nTeamwork 2\nYou gain 2 life. If this spell was cast using teamwork, destroy target artifact an opponent controls.",
        );
        lands(&mut game, alice(), Subtype::Mountain, 2);
        let helper = vanilla(&mut game, alice(), "Teammate", 2, 2);
        artifact(&mut game, bob(), "Their Relic");
        let card = put(&mut game, &spell, alice(), Zone::Hand);
        let mut dm = Script::new()
            .step(alice(), move |_, a| is_cast_of(a, card))
            .options(optional_costs(if teamwork { vec![0] } else { vec![] }))
            .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == helper).then(|| vec![helper]));
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        assert_eq!(game.player(alice()).unwrap().life, 22, "fixture: spell resolved; log={:?}", log.borrow());
        let target_prompted = log.borrow().iter().any(|l| l.starts_with("targets:") && l.contains("Teamwork Probe"));
        assert_eq!(target_prompted, teamwork, "CR 702.194c: teamwork-only targets chosen only with teamwork; log={:?}", log.borrow());
        assert_eq!(by_name(&game, Zone::Battlefield, "Their Relic").is_empty(), teamwork, "CR 702.194c: teamwork branch only");
    }
}

// ---------------------------------------------------------------------------
// 702.195 Storied
// ---------------------------------------------------------------------------

#[test]
fn cr_702_195a_b_enduring_story_is_gained_at_three_and_kept_for_the_game() {
    let mut game = new_game();
    let ori = compile(
        "Ori, Keeper of Songs",
        "Mana cost: {2}{W}\nType: Legendary Creature — Dwarf Bard\nPower/Toughness: 3/3\nStoried\nAs long as you have an enduring story, Ori gets +1/+0 and has vigilance.",
    );
    let trinket = compile("Disposable Trinket", "Mana cost: {0}\nType: Artifact\nSacrifice this artifact: You gain 1 life.");
    let ori_id = put(&mut game, &ori, alice(), Zone::Battlefield);
    let bob_ori = put(&mut game, &ori, bob(), Zone::Battlefield);
    let first = put(&mut game, &trinket, alice(), Zone::Battlefield);
    let second = put(&mut game, &trinket, alice(), Zone::Hand);
    run_turn(&mut game, &mut Script::new());
    assert_eq!(game.calculated_power(ori_id), Some(3), "CR 702.195a: two qualifying permanents aren't enough");
    run_turn(&mut game, &mut Script::new());
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, second));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(game.calculated_power(ori_id), Some(4), "CR 702.195a: three or more gives an enduring story; log={:?}", log.borrow());
    assert_eq!(game.calculated_power(bob_ori), Some(3), "CR 702.195b: the designation is per player");
    run_turn(&mut game, &mut Script::new());
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, first))
        .step(alice(), move |g, a| match a {
            LegalAction::ActivateAbility { source, .. } => g.object(*source).is_some_and(|o| &*o.name == "Disposable Trinket"),
            _ => false,
        });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert!(by_name(&game, Zone::Battlefield, "Disposable Trinket").is_empty(), "fixture: artifacts sacrificed; log={:?}", log.borrow());
    assert_eq!(game.calculated_power(ori_id), Some(4), "CR 702.195a: enduring story lasts for the rest of the game");
}

#[test]
fn cr_702_195c_effects_reapply_before_triggers_after_gaining_enduring_story() {
    let mut game = new_game();
    let bifur = compile(
        "Bifur, Melodic Rider",
        "Mana cost: {4}{R/W}{R/W}\nType: Legendary Creature — Dwarf Bard\nPower/Toughness: 4/5\nStoried\nWhenever Bifur enters or attacks, put a +1/+1 counter on target creature.\nAs long as you have an enduring story, if a triggered ability of a Dwarf you control triggers, that ability triggers an additional time.",
    );
    let tinker = compile(
        "Dwarf Tinker",
        "Mana cost: {1}\nType: Artifact Creature — Dwarf Artificer\nPower/Toughness: 1/1\nWhen this creature enters, you gain 1 life.",
    );
    put(&mut game, &bifur, alice(), Zone::Battlefield);
    artifact(&mut game, alice(), "Old Relic");
    lands(&mut game, alice(), Subtype::Mountain, 1);
    let card = put(&mut game, &tinker, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Battlefield, "Dwarf Tinker").len(), 1, "fixture: tinker resolved; log={:?}", log.borrow());
    assert_eq!(
        game.player(alice()).unwrap().life,
        22,
        "CR 702.195c: the third permanent's entry grants the story and Bifur's doubling applies to its own ETB trigger; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.180 Harmonize
// ---------------------------------------------------------------------------

#[test]
fn cr_702_180a_b_harmonize_casts_from_graveyard_tapping_a_creature_to_reduce_cost() {
    let mut game = new_game();
    let whisper = compile(
        "Unending Whisper",
        "Mana cost: {U}\nType: Sorcery\nDraw a card.\nHarmonize {5}{U}",
    );
    lands(&mut game, alice(), Subtype::Island, 2);
    let big = vanilla(&mut game, alice(), "Big Friend", 4, 4);
    let card = put(&mut game, &whisper, alice(), Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    let mut dm = Script::new()
        .step(alice(), move |_, a| matches!(a, LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, .. } if *spell_id == card))
        .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == big).then(|| vec![big]))
        .options(move |_, ctx| ctx.options.iter().find(|o| o.object_id == Some(big)).map(|o| vec![o.index]));
    let log = dm.log_handle();
    let seen = dm.seen.clone();
    let hand_before = hand_len(&game, alice());
    run_turn(&mut game, &mut dm);
    let now = game.find_object_by_stable_id(stable).unwrap();
    let offered: Vec<LegalAction> = seen.borrow().iter().flat_map(|(_, _, acts)| acts.iter().filter(|a| matches!(a, LegalAction::CastSpell { .. })).cloned()).collect();
    assert_eq!(
        game.object(now).unwrap().zone,
        Zone::Exile,
        "CR 702.180a: harmonize cast (cost reduced by tapping a 4-power creature) is offered, resolves and exiles; offered casts={offered:?} log={:?}",
        log.borrow()
    );
    assert!(game.is_tapped(big), "CR 702.180b: the chosen creature is tapped as the cost is paid");
    assert_eq!(untapped_lands(&game, alice()), 0, "CR 702.180a: cost {{5}}{{U}} reduced by X=4 to {{1}}{{U}}");
    assert_eq!(hand_len(&game, alice()), hand_before + 2, "fixture: draw step + the spell's draw");
}

// ---------------------------------------------------------------------------
// 702.178 Max speed / 702.179 Start your engines!
// ---------------------------------------------------------------------------

#[test]
fn cr_702_179a_b_d_e_702_178a_speed_starts_rises_once_per_turn_and_caps() {
    let mut game = new_game();
    let sarcophagus = compile(
        "Walking Sarcophagus",
        "Mana cost: {2}\nType: Artifact Creature — Zombie Cat\nPower/Toughness: 2/1\nStart your engines!\nMax speed — This creature gets +1/+2.",
    );
    let pinger = compile(
        "Pinger",
        "Mana cost: {0}\nType: Artifact\n{0}: Pinger deals 1 damage to target player.",
    );
    let car = put(&mut game, &sarcophagus, alice(), Zone::Battlefield);
    let ping = put(&mut game, &pinger, alice(), Zone::Battlefield);
    let attacker = vanilla(&mut game, alice(), "Racer", 1, 1);
    assert_eq!(game.player_speed(alice()), None, "CR 702.179b: no speed until set");
    let mut speeds = Vec::new();
    for _ in 0..4 {
        let mut dm = Script::new()
            .step(alice(), move |g, a| is_activation(a, ping) && g.turn.phase == Phase::FirstMain)
            .targets(|_, _| Some(vec![Target::Player(PlayerId::from_index(1))]))
            .attack_with(vec![attacker], bob());
        run_turn(&mut game, &mut dm);
        speeds.push(game.player_speed(alice()));
        run_turn(&mut game, &mut Script::new());
    }
    assert_eq!(
        speeds,
        vec![Some(2), Some(3), Some(4), Some(4)],
        "CR 702.179a/d: speed starts at 1, rises once per own turn when an opponent loses life (twice each turn here), max 4"
    );
    assert_eq!(game.player_speed(bob()), None, "CR 702.179b: Bob controls no start-your-engines permanent");
    assert!(game.has_max_speed(alice()), "CR 702.179e");
    assert_eq!(
        (game.calculated_power(car), game.calculated_toughness(car)),
        (Some(3), Some(3)),
        "CR 702.178a: max speed ability active at speed 4"
    );
}

#[test]
fn cr_702_179c_increasing_no_speed_sets_that_value() {
    let mut game = new_game();
    assert_eq!(game.player_speed(bob()), None);
    game.increase_speed(bob(), 2);
    assert_eq!(game.player_speed(bob()), Some(2), "CR 702.179c: no speed increased by 2 becomes 2");
}

#[test]
fn cr_702_179f_effects_read_no_speed_as_zero() {
    let mut game = new_game();
    let spell = compile("Speed Reader", "Mana cost: {1}\nType: Sorcery\nYou gain life equal to your speed.");
    lands(&mut game, alice(), Subtype::Plains, 1);
    let card = put(&mut game, &spell, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Graveyard, "Speed Reader").len(), 1, "fixture: resolved; log={:?}", log.borrow());
    assert_eq!(game.player(alice()).unwrap().life, 20, "CR 702.179f: no speed counts as 0");
    assert_eq!(game.player_speed(alice()), None, "CR 702.179b: reading speed doesn't give a speed");
}

#[test]
fn cr_702_178b_max_speed_granted_graveyard_cast_functions_from_graveyard() {
    for speed in [3u8, 4u8] {
        let mut game = new_game();
        let aura = compile(
            "Lightwheel Enhancements",
            "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature or Vehicle\nStart your engines!\nEnchanted permanent gets +1/+1 and has vigilance.\nMax speed — You may cast this card from your graveyard.",
        );
        lands(&mut game, alice(), Subtype::Plains, 1);
        vanilla(&mut game, alice(), "Mount", 2, 2);
        game.player_mut(alice()).unwrap().speed = Some(speed);
        let card = put(&mut game, &aura, alice(), Zone::Graveyard);
        let mut dm = Script::new().step(alice(), move |_, a| matches!(a, LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, .. } if *spell_id == card));
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        assert_eq!(
            by_name(&game, Zone::Battlefield, "Lightwheel Enhancements").len(),
            usize::from(speed == 4),
            "CR 702.178b: the graveyard permission works from the graveyard only at max speed (speed {speed}); log={:?}",
            log.borrow()
        );
    }
}

// ---------------------------------------------------------------------------
// 702.173 Freerunning
// ---------------------------------------------------------------------------

#[test]
fn cr_702_173a_freerunning_requires_assassin_or_commander_combat_damage() {
    for assassin in [false, true] {
        let mut game = new_game();
        let vision = compile("Eagle Vision", "Mana cost: {4}{U}\nType: Sorcery\nFreerunning {1}{U}\nDraw three cards.");
        let attacker_def = compile(
            "Hooded Blade",
            if assassin { "Mana cost: {1}\nType: Creature — Human Assassin\nPower/Toughness: 1/1" } else { "Mana cost: {1}\nType: Creature — Human Rogue\nPower/Toughness: 1/1" },
        );
        lands(&mut game, alice(), Subtype::Island, 2);
        let attacker = put(&mut game, &attacker_def, alice(), Zone::Battlefield);
        let card = put(&mut game, &vision, alice(), Zone::Hand);
        let mut dm = Script::new()
            .step(alice(), move |g, a| cast_alternative(card)(g, a) && g.turn.phase == Phase::NextMain)
            .attack_with(vec![attacker], bob());
        let log = dm.log_handle();
        run_turn(&mut game, &mut dm);
        assert_eq!(
            by_name(&game, Zone::Graveyard, "Eagle Vision").len(),
            usize::from(assassin),
            "CR 702.173a: freerunning available only after Assassin/commander combat damage (assassin={assassin}); log={:?}",
            log.borrow()
        );
    }
}

// ---------------------------------------------------------------------------
// 702.168 Disguise
// ---------------------------------------------------------------------------

#[test]
fn cr_702_168a_b_c_disguise_casts_face_down_with_ward_and_turns_up_for_cost() {
    let mut game = new_game();
    let basher = compile(
        "Bolrac-Clan Basher",
        "Mana cost: {4}{R}{R}\nType: Creature — Cyclops Warrior\nPower/Toughness: 3/2\nDouble strike, trample\nDisguise {3}{R}{R}",
    );
    lands(&mut game, alice(), Subtype::Mountain, 3);
    let card = put(&mut game, &basher, alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let mut dm = Script::new().step(alice(), move |_, a| {
        matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::FaceDown, .. } if *spell_id == card)
    });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield, "CR 702.168c: disguise allows casting face down; log={:?}", log.borrow());
    let chars = game.current_characteristics(permanent).unwrap();
    assert_eq!((chars.power, chars.toughness), (Some(2), Some(2)), "CR 702.168a/b: 2/2 face-down creature");
    assert!(chars.subtypes.is_empty() && chars.mana_cost.is_none(), "CR 702.168b: no subtypes and no mana cost");
    // The engine labels nameless face-down objects with a display-only constant that name
    // comparisons treat as nameless (filter.rs name_is_nameless).
    assert!(
        chars.name.is_empty() || &*chars.name == ironsmith::object::FACE_DOWN_DISPLAY_NAME,
        "CR 702.168b: no name, got {:?}",
        chars.name
    );
    let abilities = format!("{:?}", game.current_abilities(permanent).unwrap());
    assert!(abilities.contains("Ward"), "CR 702.168a: face-down disguise creature has ward {{2}}");
    assert_eq!(untapped_lands(&game, alice()), 0, "CR 702.168a: cast for {{3}}");
    run_turn(&mut game, &mut Script::new());
    lands(&mut game, alice(), Subtype::Mountain, 2);
    let mut dm = Script::new().step(alice(), move |_, a| matches!(a, LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == permanent));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(game.current_name(permanent).as_deref(), Some("Bolrac-Clan Basher"), "CR 702.168a: turned face up for its disguise cost; log={:?}", log.borrow());
}

#[test]
fn cr_702_168e_disguise_x_is_available_to_other_abilities() {
    let mut game = new_game();
    let vindicator = compile(
        "Aurelia's Vindicator",
        "Mana cost: {2}{W}{W}\nType: Creature — Angel\nPower/Toughness: 4/2\nFlying, lifelink, ward {2}\nDisguise {X}{3}{W}\nWhen this creature is turned face up, exile up to X other target creatures from the battlefield and/or creature cards from graveyards.\nWhen this creature leaves the battlefield, return the exiled cards to their owners' hands.",
    );
    lands(&mut game, alice(), Subtype::Plains, 9);
    let card = put(&mut game, &vindicator, alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    vanilla(&mut game, bob(), "Victim A", 2, 2);
    vanilla(&mut game, bob(), "Victim B", 2, 2);
    vanilla(&mut game, bob(), "Survivor", 2, 2);
    let mut dm = Script::new()
        .step(alice(), move |_, a| matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::FaceDown, .. } if *spell_id == card))
        .step(alice(), move |g, a| matches!(a, LegalAction::TurnFaceUp { creature_id, .. } if g.object(*creature_id).is_some_and(|o| o.stable_id == stable)))
        .numbers(|_, ctx| Some(2u32.clamp(ctx.min, ctx.max)))
        .targets(|g, ctx| {
            let picks: Vec<Target> = ctx.requirements.iter().flat_map(|r| r.legal_targets.iter().copied()).filter(|t| match t {
                Target::Object(id) => g.object(*id).is_some_and(|o| o.name.starts_with("Victim")),
                _ => false,
            }).collect();
            Some(picks)
        });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Battlefield, "Survivor").len(), 1, "fixture");
    assert_eq!(
        by_name(&game, Zone::Exile, "Victim A").len() + by_name(&game, Zone::Exile, "Victim B").len(),
        2,
        "CR 702.168e: X chosen for the disguise action is the X of the turned-face-up trigger; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.192 Paradigm
// ---------------------------------------------------------------------------

#[test]
fn cr_702_192a_paradigm_exiles_and_offers_a_free_copy_each_precombat_main() {
    let mut game = new_game();
    let practicum = compile(
        "Germination Practicum",
        "Mana cost: {3}{G}{G}\nType: Sorcery — Lesson\nPut two +1/+1 counters on each creature you control.\nParadigm",
    );
    lands(&mut game, alice(), Subtype::Forest, 5);
    let creature = vanilla(&mut game, alice(), "Student", 1, 1);
    let card = put(&mut game, &practicum, alice(), Zone::Hand);
    let mut dm = Script::new().step(alice(), move |_, a| is_cast_of(a, card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(plus_counters(&game, creature), 2, "fixture: resolved; log={:?}", log.borrow());
    assert_eq!(by_name(&game, Zone::Exile, "Germination Practicum").len(), 1, "CR 702.192a: then exile this spell");
    run_turn(&mut game, &mut Script::new());
    let mut dm = Script::new();
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(
        plus_counters(&game, creature),
        4,
        "CR 702.192a: at the beginning of your precombat main phase, cast a copy without paying its mana cost; log={:?}",
        log.borrow()
    );
    assert_eq!(by_name(&game, Zone::Exile, "Germination Practicum").len(), 1, "CR 702.192a: the card stays in exile");
}

#[test]
fn cr_702_180a_harmonize_cast_with_full_mana_and_no_creature() {
    let mut game = new_game();
    let whisper = compile(
        "Unending Whisper",
        "Mana cost: {U}\nType: Sorcery\nDraw a card.\nHarmonize {5}{U}",
    );
    lands(&mut game, alice(), Subtype::Island, 6);
    let card = put(&mut game, &whisper, alice(), Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    let mut dm = Script::new()
        .step(alice(), move |_, a| matches!(a, LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, .. } if *spell_id == card));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Exile, "CR 702.180a: cast from graveyard for {{5}}{{U}} then exiled; log={:?}", log.borrow());
}

// ---------------------------------------------------------------------------
// 702.158 Space sculptor
// ---------------------------------------------------------------------------

#[test]
fn cr_702_158a_b_c_d_e_space_sculptor_assigns_sectors_and_sector_actions_use_them() {
    use ironsmith::marker::SectorDesignation;
    let mut game = new_game();
    let beleren = compile(
        "Space Beleren",
        "Mana cost: {2}{W}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 3\nSpace sculptor\n+1: Creatures in each sector can be blocked this turn only by creatures in the same sector.\n−1: Put a +1/+1 counter on each creature in the sector of your choice.\n−5: Destroy all creatures in the sector of your choice.",
    );
    let walker = put(&mut game, &beleren, alice(), Zone::Battlefield);
    let a1 = vanilla(&mut game, alice(), "Alpha One", 2, 2);
    let a2 = vanilla(&mut game, alice(), "Beta One", 2, 2);
    let b1 = vanilla(&mut game, bob(), "Alpha Two", 2, 2);
    let fresh_def = compile("Late Arrival", "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/2");
    lands(&mut game, alice(), Subtype::Plains, 1);
    let fresh = put(&mut game, &fresh_def, alice(), Zone::Hand);
    let prompt_players: Rc<RefCell<Vec<PlayerId>>> = Rc::default();
    let pp = prompt_players.clone();
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, fresh))
        .step(alice(), move |g, a| {
            matches!(a, LegalAction::ActivateAbility { source, .. } if *source == walker) && g.stack.is_empty()
                && !by_name(g, Zone::Battlefield, "Late Arrival").is_empty()
                && ability_index_where(g, walker, |d| d.contains("PlusOnePlusOne")).is_some_and(|i| matches!(a, LegalAction::ActivateAbility { ability_index, .. } if *ability_index == i))
        })
        .options(move |g, ctx| {
            if ctx.description.starts_with("Choose a sector for") {
                pp.borrow_mut().push(ctx.player);
                let name = ctx.description.trim_start_matches("Choose a sector for ");
                let sector = if name.starts_with("Beta") { 1 } else { 0 };
                let _ = g;
                return Some(vec![sector]);
            }
            if ctx.options.iter().any(|o| o.description.to_lowercase().contains("alpha")) {
                return ctx.options.iter().find(|o| o.description.to_lowercase().contains("alpha")).map(|o| vec![o.index]);
            }
            None
        });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let players = prompt_players.borrow().clone();
    assert_eq!(players.first(), Some(&bob()), "CR 702.158c: players without space sculptor choose first; log={:?}", log.borrow());
    assert_eq!(game.sector_designation(a1), Some(SectorDesignation::Alpha), "CR 702.158b/c; log={:?}", log.borrow());
    assert_eq!(game.sector_designation(a2), Some(SectorDesignation::Beta), "CR 702.158b/c");
    assert_eq!(game.sector_designation(b1), Some(SectorDesignation::Alpha), "CR 702.158c");
    assert_eq!(game.sector_designation(walker), None, "CR 702.158c: only creatures are assigned");
    let late = by_name(&game, Zone::Battlefield, "Late Arrival");
    assert_eq!(late.len(), 1, "fixture: late creature resolved");
    assert!(game.sector_designation(late[0]).is_some(), "CR 702.158c: a creature entering later is assigned too; log={:?}", log.borrow());
    assert!(game.permanents_are_in_same_sector(a1, b1), "CR 702.158e");
    assert!(!game.permanents_are_in_same_sector(a1, a2), "CR 702.158e");
    assert_eq!(
        (plus_counters(&game, a1), plus_counters(&game, b1), plus_counters(&game, a2)),
        (1, 1, 0),
        "CR 702.158d: the action applies to each creature with the chosen sector designation; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.159b Claim the prize
// ---------------------------------------------------------------------------

#[test]
fn cr_702_159b_claim_the_prize_performs_the_prize_paragraph() {
    use ironsmith::EffectExecutor;
    let mut game = new_game();
    // Pick-a-Beeble's real Oracle text (simplified luck threshold removed). The rule requires
    // the Prize paragraph to be part of the visit ability and performable.
    let compiled = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Prize Booth",
        "Type: Artifact — Attraction\nVisit — Claim the prize!\nPrize — Create two Treasure tokens.",
        false,
    );
    assert!(
        compiled.is_ok(),
        "CR 702.159b: \"claim the prize\" visit text must be representable so its Prize actions can be performed; got {:?}",
        compiled.as_ref().err()
    );
    let mut beeble = compiled.unwrap();
    beeble.card.attraction_lights = vec![1, 2, 3, 4, 5, 6];
    game.enable_attractions(vec![(
        alice(),
        ironsmith::game_state::AttractionDeckFormat::Limited,
        vec![beeble.clone(), beeble.clone(), beeble],
    )])
    .expect("fixture: attraction deck");
    let source = vanilla(&mut game, alice(), "Guest", 1, 1);
    ironsmith::effects::OpenAttractionEffect::new()
        .execute(&mut game, &mut EffectContext::new_default(source, alice()))
        .expect("fixture: open an Attraction");
    assert_eq!(game.face_up_attractions().len(), 1, "fixture: one open Attraction");
    let mut dm = Script::new();
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(
        with_subtype(&game, Zone::Battlefield, Subtype::Treasure).len(),
        2,
        "CR 702.159b: claiming the prize performs the actions after the Prize dash; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.161 Living metal / 702.162 More Than Meets the Eye
// ---------------------------------------------------------------------------

#[test]
fn cr_702_161a_living_metal_is_a_creature_only_during_your_turn() {
    let mut game = new_game();
    let coupe = compile(
        "Acrobatic Coupe",
        "Type: Legendary Artifact — Vehicle\nPower/Toughness: 2/2\nLiving metal\nCrew 1",
    );
    let vehicle = put(&mut game, &coupe, alice(), Zone::Battlefield);
    let seen: Rc<RefCell<Vec<(PlayerId, bool)>>> = Rc::default();
    let s1 = seen.clone();
    let observer = move |g: &GameState, _: PlayerId| s1.borrow_mut().push((g.turn.active_player, is_creature_now(g, vehicle)));
    let mut dm = Script::new().observe(observer.clone());
    run_turn(&mut game, &mut dm);
    let mut dm = Script::new().observe(observer);
    run_turn(&mut game, &mut dm);
    let obs = seen.borrow().clone();
    assert!(obs.iter().filter(|(p, _)| *p == alice()).all(|(_, c)| *c) && obs.iter().any(|(p, _)| *p == alice()), "CR 702.161a: a creature during your turn; {obs:?}");
    assert!(obs.iter().filter(|(p, _)| *p == bob()).all(|(_, c)| !*c) && obs.iter().any(|(p, _)| *p == bob()), "CR 702.161a: not a creature during an opponent's turn; {obs:?}");
}

#[test]
fn cr_702_162a_more_than_meets_the_eye_casts_converted() {
    let mut game = new_game();
    let mut front = compile(
        "Arcee, Sharpshooter",
        "Mana cost: {1}{R}{W}\nType: Legendary Artifact Creature — Robot\nPower/Toughness: 2/2\nMore Than Meets the Eye {R}{W}\nFirst strike",
    );
    let mut back = compile(
        "Arcee, Acrobatic Coupe",
        "Type: Legendary Artifact — Vehicle\nPower/Toughness: 2/2\nLiving metal\nCrew 1",
    );
    link_faces(&mut game, &mut front, &mut back);
    lands(&mut game, alice(), Subtype::Mountain, 1);
    lands(&mut game, alice(), Subtype::Plains, 1);
    let card = put(&mut game, &front, alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let mut dm = Script::new().step(alice(), cast_alternative(card));
    let log = dm.log_handle();
    let seen = dm.seen.clone();
    run_turn(&mut game, &mut dm);
    let offered: Vec<LegalAction> = seen.borrow().iter().flat_map(|(_, _, acts)| acts.iter().filter(|a| is_cast_of(a, card)).cloned()).collect();
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Battlefield, "CR 702.162a: cast converted for {{R}}{{W}}; offered={offered:?} alts={:?} log={:?}", web_def_alts(&front), log.borrow());
    assert_eq!(game.current_name(now).as_deref(), Some("Arcee, Acrobatic Coupe"), "CR 702.162a: enters converted (back face up)");
}

// ---------------------------------------------------------------------------
// 702.167 Craft
// ---------------------------------------------------------------------------

#[test]
fn cr_702_167a_b_craft_exiles_graveyard_creature_card_and_returns_transformed() {
    let mut game = new_game();
    let mut front = compile(
        "Tithing Blade",
        "Mana cost: {1}{B}\nType: Artifact\nWhen this artifact enters, each opponent sacrifices a creature of their choice.\nCraft with creature {4}{B}",
    );
    let mut back = compile(
        "Consuming Sepulcher",
        "Type: Artifact\nAt the beginning of your upkeep, each opponent loses 1 life and you gain 1 life.",
    );
    link_faces(&mut game, &mut front, &mut back);
    lands(&mut game, alice(), Subtype::Swamp, 5);
    let blade = put(&mut game, &front, alice(), Zone::Battlefield);
    let stable = game.object(blade).unwrap().stable_id;
    let dead_def = compile("Dead Bear", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let dead = put(&mut game, &dead_def, alice(), Zone::Graveyard);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, blade))
        .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == dead).then(|| vec![dead]));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Exile, "Dead Bear").len(), 1, "CR 702.167b: a creature card in a graveyard is a legal material; log={:?}", log.borrow());
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Battlefield, "CR 702.167a: returned to the battlefield; log={:?}", log.borrow());
    assert_eq!(game.current_name(now).as_deref(), Some("Consuming Sepulcher"), "CR 702.167a: returned transformed");
}

#[test]
fn cr_702_167c_crafted_permanent_refers_to_its_exiled_materials() {
    let mut game = new_game();
    let mut front = compile(
        "Saheeli's Lattice",
        "Mana cost: {1}{R}\nType: Artifact\nCraft with one or more Dinosaurs {4}{R}",
    );
    let mut back = compile(
        "Mastercraft Raptor",
        "Type: Artifact Creature — Dinosaur\nPower/Toughness: */4\nMastercraft Raptor's power is equal to the total power of the exiled cards used to craft it.",
    );
    link_faces(&mut game, &mut front, &mut back);
    lands(&mut game, alice(), Subtype::Mountain, 5);
    let lattice = put(&mut game, &front, alice(), Zone::Battlefield);
    let stable = game.object(lattice).unwrap().stable_id;
    let dino = compile("Field Dino", "Mana cost: {2}{G}\nType: Creature — Dinosaur\nPower/Toughness: 3/3");
    let grave_dino = compile("Grave Dino", "Mana cost: {3}{G}\nType: Creature — Dinosaur\nPower/Toughness: 4/4");
    let d1 = put(&mut game, &dino, alice(), Zone::Battlefield);
    let d2 = put(&mut game, &grave_dino, alice(), Zone::Graveyard);
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, lattice))
        .objects(move |_, ctx| {
            let ids: Vec<ObjectId> = ctx.candidates.iter().filter(|c| c.id == d1 || c.id == d2).map(|c| c.id).collect();
            (!ids.is_empty()).then_some(ids)
        });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.current_name(now).as_deref(), Some("Mastercraft Raptor"), "fixture: crafted; log={:?}", log.borrow());
    assert_eq!(game.calculated_power(now), Some(7), "CR 702.167c: total power of the exiled cards used to craft it (3+4)");
}

// ---------------------------------------------------------------------------
// 702.165b/d Backup copies and lock-in
// ---------------------------------------------------------------------------

#[test]
fn cr_702_165b_token_copy_keeps_backup_ability_order() {
    let mut game = new_game();
    let aetherborn = compile(
        "Consuming Aetherborn",
        "Mana cost: {3}{B}\nType: Creature — Aetherborn Vampire\nPower/Toughness: 2/2\nBackup 1\nLifelink",
    );
    let copier = compile("Copy Spell", "Mana cost: {1}\nType: Sorcery\nCreate a token that's a copy of target creature you control.");
    put(&mut game, &aetherborn, alice(), Zone::Battlefield);
    lands(&mut game, alice(), Subtype::Swamp, 1);
    let target = vanilla(&mut game, bob(), "Opposing Bear", 2, 2);
    let card = put(&mut game, &copier, alice(), Zone::Hand);
    let had_lifelink: Rc<RefCell<bool>> = Rc::default();
    let h = had_lifelink.clone();
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .targets(move |g, ctx| {
            let wanted = if ctx.context.contains("Copy Spell") {
                ctx.requirements.iter().flat_map(|r| r.legal_targets.iter()).find(|t| matches!(t, Target::Object(id) if g.object(*id).is_some_and(|o| &*o.name == "Consuming Aetherborn"))).copied()
            } else {
                Some(Target::Object(target))
            };
            wanted.map(|t| vec![t])
        })
        .observe(move |g, _| {
            if format!("{:?}", g.current_abilities(target).unwrap_or_default()).contains("Lifelink") {
                *h.borrow_mut() = true;
            }
        });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Battlefield, "Consuming Aetherborn").len(), 2, "fixture: token copy created; log={:?}", log.borrow());
    assert_eq!(plus_counters(&game, target), 1, "CR 702.165b: the copy's backup trigger resolved; log={:?}", log.borrow());
    assert!(*had_lifelink.borrow(), "CR 702.165b: the copy grants the abilities printed below backup");
}

#[test]
fn cr_702_165d_granted_abilities_locked_when_backup_goes_on_the_stack() {
    let mut game = new_game();
    let aetherborn = compile(
        "Consuming Aetherborn",
        "Mana cost: {3}{B}\nType: Creature — Aetherborn Vampire\nPower/Toughness: 2/2\nBackup 1\nLifelink",
    );
    let blank = compile("Blank Out", "Mana cost: {B}\nType: Instant\nTarget creature loses all abilities until end of turn.");
    lands(&mut game, alice(), Subtype::Swamp, 5);
    let target = vanilla(&mut game, bob(), "Opposing Bear", 2, 2);
    let card = put(&mut game, &aetherborn, alice(), Zone::Hand);
    let instant = put(&mut game, &blank, alice(), Zone::Hand);
    let had_lifelink: Rc<RefCell<bool>> = Rc::default();
    let h = had_lifelink.clone();
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_cast_of(a, card))
        .step(alice(), move |g, a| is_cast_of(a, instant) && !g.stack.is_empty())
        .targets(move |g, ctx| {
            let wanted = if ctx.context.contains("Blank Out") {
                ctx.requirements.iter().flat_map(|r| r.legal_targets.iter()).find(|t| matches!(t, Target::Object(id) if g.object(*id).is_some_and(|o| &*o.name == "Consuming Aetherborn"))).copied()
            } else {
                Some(Target::Object(target))
            };
            wanted.map(|t| vec![t])
        })
        .observe(move |g, _| {
            if format!("{:?}", g.current_abilities(target).unwrap_or_default()).contains("Lifelink") {
                *h.borrow_mut() = true;
            }
        });
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Graveyard, "Blank Out").len(), 1, "fixture: Blank Out resolved in response; log={:?}", log.borrow());
    assert_eq!(plus_counters(&game, target), 1, "fixture: backup resolved; log={:?}", log.borrow());
    assert!(*had_lifelink.borrow(), "CR 702.165d: granted abilities were fixed as the trigger went on the stack; log={:?}", log.borrow());
}

// ---------------------------------------------------------------------------
// 702.177b Exhaust permission effects
// ---------------------------------------------------------------------------

#[test]
fn cr_702_177b_refueler_permission_only_before_any_exhaust_activation_this_turn() {
    let mut game = new_game();
    let specialist = compile(
        "Prowcatcher Specialist",
        "Mana cost: {1}{R}\nType: Creature — Goblin Warrior\nPower/Toughness: 2/1\nHaste\nExhaust — {3}{R}: Put two +1/+1 counters on this creature.",
    );
    let refueler = compile(
        "Elvish Refueler",
        "Mana cost: {2}{G}\nType: Creature — Elf Druid\nPower/Toughness: 2/3\nDuring your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though they haven't been activated.\nExhaust — {1}{G}: Put a +1/+1 counter on this creature.",
    );
    lands(&mut game, alice(), Subtype::Mountain, 12);
    let creature = put(&mut game, &specialist, alice(), Zone::Battlefield);
    let mut dm = Script::new().step(alice(), move |_, a| is_activation(a, creature));
    run_turn(&mut game, &mut dm);
    assert_eq!(plus_counters(&game, creature), 2, "fixture: first exhaust activation");
    put(&mut game, &refueler, alice(), Zone::Battlefield);
    run_turn(&mut game, &mut Script::new());
    let mut dm = Script::new()
        .step(alice(), move |_, a| is_activation(a, creature))
        .step(alice(), move |_, a| is_activation(a, creature));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(
        plus_counters(&game, creature),
        4,
        "CR 702.177b: Refueler re-enables the used exhaust ability once, but not after an exhaust activation this turn; log={:?}",
        log.borrow()
    );
}

// ---------------------------------------------------------------------------
// 702.186 Infinity / harness
// ---------------------------------------------------------------------------

#[test]
fn cr_702_186a_b_infinity_ability_works_only_while_harnessed() {
    let mut game = new_game();
    let stone = compile(
        "The Soul Stone",
        "Mana cost: {1}{B}\nType: Legendary Artifact — Infinity Stone\nIndestructible\n{T}: Add {B}.\n{6}{B}, {T}, Exile a creature you control: Harness The Soul Stone.\n∞ — At the beginning of your upkeep, return target creature card from your graveyard to the battlefield.",
    );
    let stone_id = put(&mut game, &stone, alice(), Zone::Battlefield);
    lands(&mut game, alice(), Subtype::Swamp, 7);
    let fodder = vanilla(&mut game, alice(), "Fodder", 1, 1);
    let dead_def = compile("Dead Bear", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    put(&mut game, &dead_def, alice(), Zone::Graveyard);
    let mut dm = Script::new()
        .step(alice(), move |_, a| {
            matches!(a, LegalAction::ActivateAbility { source, .. } if *source == stone_id)
        })
        .objects(move |_, ctx| ctx.candidates.iter().any(|c| c.id == fodder).then(|| vec![fodder]));
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Graveyard, "Dead Bear").len(), 1, "CR 702.186b: unharnessed, the ∞ upkeep ability doesn't function; log={:?}", log.borrow());
    assert!(game.is_harnessed(stone_id), "fixture: harnessed; log={:?}", log.borrow());
    run_turn(&mut game, &mut Script::new());
    let mut dm = Script::new();
    let log = dm.log_handle();
    run_turn(&mut game, &mut dm);
    assert_eq!(by_name(&game, Zone::Battlefield, "Dead Bear").len(), 1, "CR 702.186a/b: harnessed, it has the ∞ ability; log={:?}", log.borrow());
}
