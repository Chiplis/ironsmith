//! Runtime checks for the filter word-claims families: each test compiles the
//! card's real Oracle text, plays it through the engine (legal-action
//! discovery, announcement, combat declarations, saga chapters, stack
//! resolution and SBAs) and asserts the rules outcome, not the compiled text.
//! These cards compiled to plausible-looking text while meaning something
//! else, so only gameplay pins the fix.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, BlockerDeclaration, DecisionMaker, GameProgress, LegalAction,
    SelectFirstDecisionMaker, compute_legal_actions, compute_legal_attackers,
    compute_legal_blockers,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
    TextInputContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, add_lore_counter_and_check_chapters,
    apply_attacker_declarations_with_dm, apply_blocker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm, check_and_apply_sbas_with,
    drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::AttachmentTarget;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, Effect, GameState, ObjectId, Phase, PlayerId,
    PowerToughness, Step, Subtype, Supertype, Zone,
};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
const CARA: PlayerId = PlayerId(2);

fn compile(name: &str, text: &str) -> CardDefinition {
    ironsmith_tools::parse_card_definition_with_runtime_builder(name, text, false)
        .unwrap_or_else(|error| panic!("{name} failed to compile: {error}"))
}

fn game(players: usize) -> GameState {
    let mut g = GameState::new(
        ["Alice", "Bob", "Cara"][..players]
            .iter()
            .map(|name| name.to_string())
            .collect(),
        20,
    );
    g.turn.active_player = ALICE;
    g.turn.priority_player = Some(ALICE);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.turn_number = 3;
    g
}

fn add_mana(g: &mut GameState, player: PlayerId, symbol: ManaSymbol, amount: u32) {
    g.player_mut(player).unwrap().mana_pool.add(symbol, amount);
}

/// A vanilla fixture body; P/T only for creatures.
fn body(
    name: &str,
    types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    power: i32,
    toughness: i32,
    mana_value: u8,
) -> CardDefinition {
    let creature = types.contains(&CardType::Creature);
    let builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types)
        .subtypes(subtypes)
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value,
        )]));
    if creature {
        builder
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    } else {
        builder.build()
    }
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    body(name, vec![CardType::Creature], vec![], power, toughness, 2)
}

fn basic_plains(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .build()
}

fn put(g: &mut GameState, def: &CardDefinition, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.create_object_from_definition(def, owner, zone);
    if zone == Zone::Battlefield {
        g.remove_summoning_sickness(id);
    }
    id
}

fn stock_library(g: &mut GameState, player: PlayerId, count: usize) {
    let filler = body("Library filler", vec![CardType::Artifact], vec![], 0, 0, 1);
    for _ in 0..count {
        g.create_object_from_definition(&filler, player, Zone::Library);
    }
}

fn find_named(g: &GameState, name: &str, zone: Zone) -> Vec<ObjectId> {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|object| object.name == name && object.zone == zone)
        .map(|object| object.id)
        .collect()
}

fn names_in(g: &GameState, ids: &[ObjectId]) -> Vec<String> {
    ids.iter()
        .filter_map(|id| g.object(*id).map(|object| object.name.to_string()))
        .collect()
}

fn plus_counters(g: &GameState, id: ObjectId) -> u32 {
    g.object(id)
        .and_then(|object| object.counters.counts().get(&CounterType::PlusOnePlusOne).copied())
        .unwrap_or(0)
}

/// Index of the first non-mana activated ability matching `zone`.
fn activated_index(def: &CardDefinition, zone: Zone) -> usize {
    def.abilities
        .iter()
        .position(|ability| {
            matches!(ability.kind, AbilityKind::Activated(_))
                && !ability.is_mana_ability()
                && ability.functions_in(&zone)
        })
        .unwrap_or_else(|| panic!("{} has no non-mana activated ability in {zone:?}", def.name()))
}

/// Scripted chooser that records what the engine offered.
#[derive(Default)]
struct Script {
    /// Consumed by the next target announcement; empty = first legal targets.
    targets: Vec<Target>,
    /// First legal candidate with one of these names, in order of preference.
    object_preference: Vec<&'static str>,
    /// First legal option whose description contains this text.
    option_preference: Option<&'static str>,
    accept: bool,
    /// Legal targets offered per requirement, in announcement order.
    offered_targets: Vec<Vec<Target>>,
    /// (description, [(candidate name, legal)]) for each object choice.
    offered_objects: Vec<(String, Vec<(String, bool)>)>,
    /// Every object selection returned, in order.
    chosen_objects: Vec<ObjectId>,
    /// Answer to free-text prompts ("choose a card name").
    text: Option<&'static str>,
}

impl DecisionMaker for Script {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_text(&mut self, _: &GameState, c: &TextInputContext) -> String {
        self.text
            .map(str::to_string)
            .or_else(|| c.initial_value.clone())
            .unwrap_or_default()
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.accept && c.can_accept
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        self.offered_objects.push((
            c.description.clone(),
            c.candidates
                .iter()
                .map(|candidate| (candidate.name.clone(), candidate.legal))
                .collect(),
        ));
        let preferred = self.object_preference.iter().find_map(|name| {
            c.candidates
                .iter()
                .find(|candidate| candidate.legal && candidate.name == *name)
        });
        let chosen = match preferred {
            Some(found) => vec![found.id],
            None => SelectFirstDecisionMaker.decide_objects(g, c),
        };
        self.chosen_objects.extend(chosen.iter().copied());
        chosen
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        if let Some(text) = self.option_preference
            && let Some(option) = c
                .options
                .iter()
                .find(|option| option.legal && option.description.contains(text))
        {
            return vec![option.index];
        }
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        for requirement in &c.requirements {
            self.offered_targets.push(requirement.legal_targets.clone());
        }
        if self.targets.is_empty() {
            SelectFirstDecisionMaker.decide_targets(g, c)
        } else {
            std::mem::take(&mut self.targets)
        }
    }
}

/// Drive SBAs, triggers and resolution until the stack is empty.
fn settle(g: &mut GameState, queue: &mut TriggerQueue, dm: &mut Script) {
    for _ in 0..64 {
        check_and_apply_sbas_with(g, queue, dm).unwrap_or_else(|e| panic!("SBA: {e}"));
        drain_pending_trigger_events(g, queue);
        put_triggers_on_stack_with_dm(g, queue, dm).unwrap_or_else(|e| panic!("triggers: {e}"));
        if g.stack.is_empty() {
            return;
        }
        resolve_stack_entry_with(g, dm).unwrap_or_else(|e| panic!("resolution: {e}"));
    }
    panic!("stack did not settle");
}

/// Take a real legal cast (`ability == None`) or activation through priority
/// until it is on the stack; return the trigger queue for `settle`.
fn perform(
    g: &mut GameState,
    source: ObjectId,
    actor: PlayerId,
    ability: Option<usize>,
    dm: &mut Script,
) -> Result<TriggerQueue, String> {
    let initial_stack_len = g.stack.len();
    g.turn.priority_player = Some(actor);
    let action = compute_legal_actions(g, actor)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|action| match action {
            LegalAction::CastSpell { spell_id, .. } => ability.is_none() && *spell_id == source,
            LegalAction::ActivateAbility {
                source: id,
                ability_index,
            } => *id == source && Some(*ability_index) == ability,
            _ => false,
        })
        .ok_or("intended action is not legal")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if g.stack.len() > initial_stack_len {
            return Ok(queue);
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref c))
                if c.description.starts_with("Choose optional costs") =>
            {
                apply_priority_response_with_dm(
                    g,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(vec![]),
                    dm,
                )
                .map_err(|e| e.to_string())?
            }
            GameProgress::NeedsDecisionCtx(ref c) if !matches!(c, DecisionContext::Priority(_)) => {
                apply_decision_context_with_dm(g, &mut queue, &mut state, c, dm)
                    .map_err(|e| e.to_string())?
            }
            other => return Err(format!("action failed to reach the stack: {other:?}")),
        };
    }
    Err("action decision bound exceeded".into())
}

fn declare_attackers(
    g: &mut GameState,
    attacker_player: PlayerId,
    attacks: &[(ObjectId, AttackTarget)],
    dm: &mut Script,
) -> CombatState {
    g.turn.active_player = attacker_player;
    g.turn.priority_player = Some(attacker_player);
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    let declarations: Vec<_> = attacks
        .iter()
        .map(|(creature, target)| AttackerDeclaration {
            creature: *creature,
            target: target.clone(),
        })
        .collect();
    apply_attacker_declarations_with_dm(g, &mut combat, &mut queue, &declarations, dm)
        .unwrap_or_else(|e| panic!("attack declaration: {e}"));
    g.combat = Some(combat.clone());
    settle(g, &mut queue, dm);
    g.combat.clone().unwrap_or(combat)
}

fn declare_blockers(
    g: &mut GameState,
    combat: &mut CombatState,
    blocks: &[(ObjectId, ObjectId)],
    defender: PlayerId,
    dm: &mut Script,
) {
    g.turn.step = Some(Step::DeclareBlockers);
    let mut queue = TriggerQueue::new();
    let declarations: Vec<_> = blocks
        .iter()
        .map(|(blocker, blocking)| BlockerDeclaration {
            blocker: *blocker,
            blocking: *blocking,
        })
        .collect();
    apply_blocker_declarations(g, combat, &mut queue, &declarations, defender)
        .unwrap_or_else(|e| panic!("block declaration: {e}"));
    g.combat = Some(combat.clone());
    settle(g, &mut queue, dm);
    if let Some(updated) = g.combat.clone() {
        *combat = updated;
    }
}

/// Resolve a neutral ability (no card text) from `source`.
fn resolve_neutral(g: &mut GameState, source: ObjectId, controller: PlayerId, effects: Vec<Effect>) {
    let mut dm = Script::default();
    g.push_to_stack(StackEntry::ability(source, controller, effects));
    let mut queue = TriggerQueue::new();
    settle(g, &mut queue, &mut dm);
}

fn destroy(g: &mut GameState, source: ObjectId, victim: ObjectId) {
    let filter = ObjectFilter {
        specific: Some(victim),
        ..Default::default()
    };
    resolve_neutral(g, source, ALICE, vec![Effect::destroy_all(filter)]);
}

const GENESIS_OF_THE_DALEKS: &str = "Mana cost: {4}{B}{B}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after IV.)\nI, II, III — Create a 3/3 black Dalek artifact creature token with menace for each lore counter on Genesis of the Daleks.\nIV — Target opponent faces a villainous choice — Destroy all Dalek creatures and each of your opponents loses life equal to the total power of Daleks that died this turn, or destroy all non-Dalek creatures.";

/// Chapter IV counts every Dalek that died this turn, by last-known power:
/// one destroyed before the chapter plus the ones its own destruction kills.
#[test]
fn genesis_of_the_daleks_chapter_four_counts_daleks_that_died_this_turn() {
    let mut g = game(3);
    let saga_def = compile("Genesis of the Daleks", GENESIS_OF_THE_DALEKS);
    let saga = put(&mut g, &saga_def, ALICE, Zone::Battlefield);
    g.object_mut(saga)
        .unwrap()
        .counters
        .insert(CounterType::Lore, 3);
    let dalek = |name: &str, power: i32| {
        body(name, vec![CardType::Artifact, CardType::Creature], vec![Subtype::Dalek], power, power, 3)
    };
    let early = put(&mut g, &dalek("Early Dalek", 4), ALICE, Zone::Battlefield);
    put(&mut g, &dalek("Alice Dalek", 3), ALICE, Zone::Battlefield);
    let pumped = put(&mut g, &dalek("Pumped Dalek", 2), ALICE, Zone::Battlefield);
    g.object_mut(pumped)
        .unwrap()
        .counters
        .insert(CounterType::PlusOnePlusOne, 1);
    put(&mut g, &dalek("Bob Dalek", 1), BOB, Zone::Battlefield);
    let bear = put(&mut g, &creature("Bob Bear", 2, 2), BOB, Zone::Battlefield);

    destroy(&mut g, saga, early);
    assert!(find_named(&g, "Early Dalek", Zone::Battlefield).is_empty());

    let mut dm = Script {
        targets: vec![Target::Player(BOB)],
        option_preference: Some("Dalek creatures and"),
        ..Script::default()
    };
    let mut queue = TriggerQueue::new();
    add_lore_counter_and_check_chapters(&mut g, saga, &mut queue).unwrap();
    settle(&mut g, &mut queue, &mut dm);

    for name in ["Alice Dalek", "Pumped Dalek", "Bob Dalek"] {
        assert!(find_named(&g, name, Zone::Battlefield).is_empty(), "{name} should be destroyed");
    }
    assert!(g.object(bear).is_some_and(|o| o.zone == Zone::Battlefield), "non-Dalek survives");
    // 4 (died earlier this turn) + 3 + 3 (2/2 with a +1/+1 counter, LKI) + 1.
    let life: Vec<i32> = [ALICE, BOB, CARA].iter().map(|p| g.player(*p).unwrap().life).collect();
    assert_eq!(life, vec![20, 9, 9], "the Saga controller's opponents lose 11");
}

const XANTCHA: &str = "Mana cost: {1}{B}{R}\nType: Legendary Creature — Phyrexian Minion\nPower/Toughness: 5/5\nXantcha enters under the control of an opponent of your choice.\nXantcha attacks each combat if able and can't attack its owner or planeswalkers its owner controls.\n{3}: Xantcha's controller loses 2 life and you draw a card. Any player may activate this ability.";

/// Under Bob's control, Alice's Xantcha may attack Cara and Cara's
/// planeswalker, but never Alice or Alice's planeswalker.
#[test]
fn xantcha_cannot_attack_its_owner_or_owners_planeswalkers() {
    let mut g = game(3);
    let def = compile("Xantcha, Sleeper Agent", XANTCHA);
    let xantcha = put(&mut g, &def, ALICE, Zone::Battlefield);
    g.set_current_controller(xantcha, BOB).unwrap();
    g.remove_summoning_sickness(xantcha);
    let walker = body("Fixture walker", vec![CardType::Planeswalker], vec![], 0, 0, 3);
    let alice_walker = put(&mut g, &walker, ALICE, Zone::Battlefield);
    let cara_walker = put(&mut g, &walker, CARA, Zone::Battlefield);
    for id in [alice_walker, cara_walker] {
        g.object_mut(id)
            .unwrap()
            .counters
            .insert(CounterType::Loyalty, 5);
    }
    g.turn.active_player = BOB;
    g.turn.priority_player = Some(BOB);
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::DeclareAttackers);
    g.refresh_continuous_state().unwrap();
    assert_eq!(g.controller_of_id(xantcha), Some(BOB));

    let option = compute_legal_attackers(&g, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == xantcha)
        .expect("Xantcha can attack under Bob's control");
    let targets = &option.valid_targets;
    assert!(option.must_attack, "attacks each combat if able");
    assert!(targets.contains(&AttackTarget::Player(CARA)), "{targets:?}");
    assert!(targets.contains(&AttackTarget::Planeswalker(cara_walker)), "{targets:?}");
    assert!(!targets.contains(&AttackTarget::Player(ALICE)), "owner: {targets:?}");
    assert!(
        !targets.contains(&AttackTarget::Planeswalker(alice_walker)),
        "owner's planeswalker: {targets:?}"
    );
}

/// Bob attacks with two creatures; `wall` blocks the first and a plain
/// blocker the second. Only the attacker `wall` blocks may be targeted, and
/// the ability then damages it.
fn blocked_attacker_is_only_target(name: &str, text: &str, mana: &[ManaSymbol], damage: u32) {
    let mut g = game(2);
    let def = compile(name, text);
    let wall = put(&mut g, &def, ALICE, Zone::Battlefield);
    let other_blocker = put(&mut g, &creature("Alice Blocker", 1, 6), ALICE, Zone::Battlefield);
    let blocked = put(&mut g, &creature("Blocked Attacker", 2, 6), BOB, Zone::Battlefield);
    let elsewhere = put(&mut g, &creature("Other Attacker", 2, 6), BOB, Zone::Battlefield);
    for symbol in mana {
        add_mana(&mut g, ALICE, *symbol, 1);
    }
    let mut dm = Script::default();
    let mut combat = declare_attackers(
        &mut g,
        BOB,
        &[
            (blocked, AttackTarget::Player(ALICE)),
            (elsewhere, AttackTarget::Player(ALICE)),
        ],
        &mut dm,
    );
    declare_blockers(
        &mut g,
        &mut combat,
        &[(wall, blocked), (other_blocker, elsewhere)],
        ALICE,
        &mut dm,
    );

    let index = activated_index(&def, Zone::Battlefield);
    let mut queue = perform(&mut g, wall, ALICE, Some(index), &mut dm)
        .unwrap_or_else(|e| panic!("{name} activation: {e}"));
    assert_eq!(
        dm.offered_targets,
        vec![vec![Target::Object(blocked)]],
        "{name}: only the creature it's blocking is a legal target"
    );
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(g.damage_on(blocked), damage, "{name} damages the blocked attacker");
    assert_eq!(g.damage_on(elsewhere), 0);
    assert_eq!(g.damage_on(other_blocker), 0);
}

#[test]
fn tinder_wall_targets_only_the_creature_it_is_blocking() {
    blocked_attacker_is_only_target(
        "Tinder Wall",
        "Mana cost: {G}\nType: Creature — Plant Wall\nPower/Toughness: 0/3\nDefender (This creature can't attack.)\nSacrifice this creature: Add {R}{R}.\n{R}, Sacrifice this creature: It deals 2 damage to target creature it's blocking.",
        &[ManaSymbol::Red],
        2,
    );
}

#[test]
fn goblin_snowman_targets_only_the_creature_it_is_blocking() {
    blocked_attacker_is_only_target(
        "Goblin Snowman",
        "Mana cost: {3}{R}\nType: Creature — Goblin\nPower/Toughness: 1/1\nWhenever this creature blocks, prevent all combat damage that would be dealt to and dealt by it this turn.\n{T}: This creature deals 1 damage to target creature it's blocking.",
        &[],
        1,
    );
}

const CAMEL: &str = "Mana cost: {W}\nType: Creature — Camel\nPower/Toughness: 0/1\nBanding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)\nAs long as this creature is attacking, prevent all damage Deserts would deal to this creature and to creatures banded with this creature.";

/// Desert damage is prevented to the attacking Camel and its band-mate only;
/// an unbanded attacker and non-Desert damage are unaffected.
#[test]
fn camel_prevents_desert_damage_to_itself_and_its_band() {
    let mut g = game(2);
    let camel = put(&mut g, &compile("Camel", CAMEL), ALICE, Zone::Battlefield);
    let mate = put(&mut g, &creature("Band Mate", 2, 6), ALICE, Zone::Battlefield);
    let loner = put(&mut g, &creature("Unbanded Attacker", 2, 6), ALICE, Zone::Battlefield);
    let desert_def = CardDefinitionBuilder::new(CardId::new(), "Fixture Desert")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Desert])
        .build();
    let desert = put(&mut g, &desert_def, BOB, Zone::Battlefield);
    let shooter = put(&mut g, &creature("Non-Desert Shooter", 1, 1), BOB, Zone::Battlefield);
    let mut dm = Script::default();
    let mut combat = declare_attackers(
        &mut g,
        ALICE,
        &[
            (camel, AttackTarget::Player(BOB)),
            (mate, AttackTarget::Player(BOB)),
            (loner, AttackTarget::Player(BOB)),
        ],
        &mut dm,
    );
    ironsmith::combat_state::set_attacking_band(&g, &mut combat, vec![camel, mate])
        .expect("Camel's banding forms a band");
    g.combat = Some(combat);

    for target in [camel, mate, loner] {
        resolve_neutral(
            &mut g,
            desert,
            BOB,
            vec![Effect::deal_damage(1i32, ChooseSpec::SpecificObject(target))],
        );
    }
    assert_eq!(g.damage_on(camel), 0, "Desert damage to Camel is prevented");
    assert_eq!(g.damage_on(mate), 0, "Desert damage to a creature banded with Camel is prevented");
    assert_eq!(g.damage_on(loner), 1, "an unbanded attacker is not protected");

    resolve_neutral(
        &mut g,
        shooter,
        BOB,
        vec![Effect::deal_damage(1i32, ChooseSpec::SpecificObject(mate))],
    );
    assert_eq!(g.damage_on(mate), 1, "non-Desert damage is not prevented");
}

/// Aura Graft may only move the Aura onto another permanent it can enchant.
#[test]
fn aura_graft_attaches_only_to_a_permanent_the_aura_can_enchant() {
    let mut g = game(2);
    let graft = compile(
        "Aura Graft",
        "Mana cost: {1}{U}\nType: Instant\nGain control of target Aura that's attached to a permanent. Attach it to another permanent it can enchant.",
    );
    let holy_strength = compile(
        "Holy Strength",
        "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+2.",
    );
    let bob_bear = put(&mut g, &creature("Bob Bear", 2, 2), BOB, Zone::Battlefield);
    let aura = put(&mut g, &holy_strength, BOB, Zone::Battlefield);
    assert!(g.attach_object_to_target(aura, AttachmentTarget::Object(bob_bear)));
    let alice_bear = put(&mut g, &creature("Alice Bear", 2, 2), ALICE, Zone::Battlefield);
    let land = CardDefinitionBuilder::new(CardId::new(), "Alice Land")
        .card_types(vec![CardType::Land])
        .build();
    put(&mut g, &land, ALICE, Zone::Battlefield);
    put(&mut g, &body("Alice Relic", vec![CardType::Artifact], vec![], 0, 0, 1), ALICE, Zone::Battlefield);
    add_mana(&mut g, ALICE, ManaSymbol::Blue, 2);
    let spell = g.create_object_from_definition(&graft, ALICE, Zone::Hand);

    let mut dm = Script {
        targets: vec![Target::Object(aura)],
        // Illegal hosts first: a correct engine never offers them as legal.
        object_preference: vec!["Alice Land", "Alice Relic", "Bob Bear", "Alice Bear"],
        ..Script::default()
    };
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Aura Graft");
    settle(&mut g, &mut queue, &mut dm);

    let aura_object = g.object(aura).expect("the Aura stays on the battlefield");
    assert_eq!(
        aura_object.attached_to,
        Some(AttachmentTarget::Object(alice_bear)),
        "offered: {:?}",
        dm.offered_objects
    );
    assert_eq!(g.controller_of_id(aura), Some(ALICE));
    for (_, candidates) in &dm.offered_objects {
        for (name, legal) in candidates {
            if ["Alice Land", "Alice Relic", "Bob Bear"].contains(&name.as_str()) {
                assert!(!legal, "{name} must not be a legal Aura Graft destination");
            }
        }
    }
}

const BEST_OF_BOTH_WORLDS: &str = "Mana cost: {4}{W}{U}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — Assimilate target nonartifact creature an opponent controls. (Gain control of it and put a +1/+1 counter on it. It's a Borg artifact creature and loses all other creature types.)\nII — Artifact creatures you control get +2/+2 until end of turn.\nIII — Exile the Borg assimilated with this Saga, then return it to the battlefield under its owner's control.";

/// Chapter III flickers only the creature this Saga assimilated; another Borg
/// stays put, and the assimilated creature returns to its owner.
#[test]
fn best_of_both_worlds_chapter_three_returns_only_its_assimilated_borg() {
    let mut g = game(2);
    let saga = put(
        &mut g,
        &compile("The Best of Both Worlds", BEST_OF_BOTH_WORLDS),
        ALICE,
        Zone::Battlefield,
    );
    let assimilated = put(&mut g, &creature("Assimilation Target", 2, 2), BOB, Zone::Battlefield);
    let other_borg = put(
        &mut g,
        &body("Other Borg", vec![CardType::Artifact, CardType::Creature], vec![Subtype::Borg], 2, 2, 2),
        ALICE,
        Zone::Battlefield,
    );
    let mut dm = Script {
        targets: vec![Target::Object(assimilated)],
        ..Script::default()
    };
    let mut queue = TriggerQueue::new();
    add_lore_counter_and_check_chapters(&mut g, saga, &mut queue).unwrap();
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(g.controller_of_id(assimilated), Some(ALICE), "chapter I gains control");
    assert_eq!(plus_counters(&g, assimilated), 1);
    assert!(
        g.current_subtypes(assimilated)
            .is_some_and(|subtypes| subtypes.contains(&Subtype::Borg))
    );

    for _ in 0..2 {
        add_lore_counter_and_check_chapters(&mut g, saga, &mut queue).unwrap();
        settle(&mut g, &mut queue, &mut dm);
    }

    assert!(
        g.object(other_borg).is_some_and(|o| o.zone == Zone::Battlefield),
        "the unrelated Borg is not exiled"
    );
    let returned = find_named(&g, "Assimilation Target", Zone::Battlefield);
    assert_eq!(returned.len(), 1, "the assimilated creature returns");
    assert_ne!(returned[0], assimilated, "it was exiled and returned as a new object");
    assert_eq!(g.controller_of_id(returned[0]), Some(BOB), "under its owner's control");
    assert_eq!(plus_counters(&g, returned[0]), 0);
}

const DANIELS: &str = "Mana cost: {2}{B/G}{B/G}\nType: Legendary Creature — Human Advisor\nPower/Toughness: 3/1\nWhen Daniels enters, return target creature card from your graveyard to your hand.\n{2}, Put this card from your graveyard on the bottom of your library: You gain 2 life.";

/// The {2} ability works from the graveyard and moves Daniels to the bottom
/// of the library as a cost; it is not an ability of the permanent.
#[test]
fn daniels_activates_from_graveyard_and_goes_to_library_bottom_as_cost() {
    let def = compile("Daniels, Temporal Agent", DANIELS);
    let index = activated_index(&def, Zone::Graveyard);

    let mut g = game(2);
    stock_library(&mut g, ALICE, 3);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 2);
    let daniels = put(&mut g, &def, ALICE, Zone::Graveyard);
    let mut dm = Script::default();
    let mut queue = perform(&mut g, daniels, ALICE, Some(index), &mut dm)
        .expect("graveyard activation is legal");
    let library = g.player(ALICE).unwrap().library.to_vec();
    assert_eq!(library.len(), 4);
    assert_eq!(
        names_in(&g, &library[..1]),
        vec!["Daniels, Temporal Agent".to_string()],
        "paid as a cost: on the bottom before the ability resolves"
    );
    assert!(g.player(ALICE).unwrap().graveyard.is_empty());
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(g.player(ALICE).unwrap().life, 22);

    let mut g = game(2);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 2);
    let permanent = put(&mut g, &def, ALICE, Zone::Battlefield);
    let offered = compute_legal_actions(&g, ALICE).unwrap().into_iter().any(|action| {
        matches!(action, LegalAction::ActivateAbility { source, ability_index }
            if source == permanent && ability_index == index)
    });
    assert!(!offered, "not activatable while Daniels is on the battlefield");
}

const NATHAN_DRAKE: &str = "Mana cost: {U}{B}{R}\nType: Legendary Creature — Human Rogue\nPower/Toughness: 3/2\nFirst strike\nYou may spend mana as though it were mana of any color to cast spells you don't own or to activate abilities of permanents you control but don't own.\nWhenever Nathan Drake attacks, exile the top card of each player's library. You may cast a spell from among those cards.";

const GREEN_PUMPER: &str = "Mana cost: {1}{G}\nType: Creature — Elf\nPower/Toughness: 1/1\n{G}: This creature gets +1/+1 until end of turn.";

/// With only blue mana, Alice can pay {G} for an ability of a permanent she
/// controls but Bob owns; her own permanent's {G} ability stays unpayable.
#[test]
fn nathan_drake_spends_any_color_on_permanents_you_control_but_dont_own() {
    let mut g = game(2);
    put(&mut g, &compile("Nathan Drake, Treasure Hunter", NATHAN_DRAKE), ALICE, Zone::Battlefield);
    let pumper = compile("Green Pumper", GREEN_PUMPER);
    let stolen = put(&mut g, &pumper, BOB, Zone::Battlefield);
    g.set_current_controller(stolen, ALICE).unwrap();
    g.remove_summoning_sickness(stolen);
    let own = put(&mut g, &pumper, ALICE, Zone::Battlefield);
    add_mana(&mut g, ALICE, ManaSymbol::Blue, 3);
    g.refresh_continuous_state().unwrap();
    assert_eq!(g.controller_of_id(stolen), Some(ALICE));

    let actions = compute_legal_actions(&g, ALICE).unwrap();
    let offers = |source: ObjectId| {
        actions.iter().any(|action| {
            matches!(action, LegalAction::ActivateAbility { source: id, ability_index: 0 } if *id == source)
        })
    };
    assert!(!offers(own), "Alice owns this one: blue can't pay {{G}}");
    assert!(offers(stolen), "controlled but not owned: blue pays {{G}}");

    let mut dm = Script::default();
    let mut queue = perform(&mut g, stolen, ALICE, Some(0), &mut dm).expect("activation");
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(g.calculated_power(stolen), Some(2));
    assert_eq!(g.player(ALICE).unwrap().mana_pool.total(), 2);
}

const STARFLEET_COMMUNICATOR: &str = "Mana cost: {3}\nType: Artifact — Communicator Equipment\nWhen you cast this spell, copy it. (The copy becomes a token.)\nEquipped creature has \"Whenever this creature attacks, draw X cards, then you may put up to X land cards from your hand onto the battlefield tapped, where X is the number of other creatures you control with a Communicator attached to them.\"\nEquip {1}";

/// X counts other creatures with a Communicator attached: not the attacker,
/// not a creature wearing some other Equipment, not a bare creature.
#[test]
fn starfleet_communicator_counts_other_creatures_with_a_communicator() {
    let mut g = game(2);
    stock_library(&mut g, ALICE, 6);
    let communicator = compile("Starfleet Communicator", STARFLEET_COMMUNICATOR);
    let attacker = put(&mut g, &creature("Away Team Lead", 2, 2), ALICE, Zone::Battlefield);
    let linked = put(&mut g, &creature("Linked Crew", 2, 2), ALICE, Zone::Battlefield);
    let geared = put(&mut g, &creature("Geared Crew", 2, 2), ALICE, Zone::Battlefield);
    put(&mut g, &creature("Bare Crew", 2, 2), ALICE, Zone::Battlefield);
    for host in [attacker, linked] {
        let equipment = put(&mut g, &communicator, ALICE, Zone::Battlefield);
        assert!(g.attach_object_to_target(equipment, AttachmentTarget::Object(host)));
    }
    let gear = put(
        &mut g,
        &body("Plain Gear", vec![CardType::Artifact], vec![Subtype::Equipment], 0, 0, 1),
        ALICE,
        Zone::Battlefield,
    );
    assert!(g.attach_object_to_target(gear, AttachmentTarget::Object(geared)));
    let hand_before = g.player(ALICE).unwrap().hand.len();

    let mut dm = Script::default();
    declare_attackers(&mut g, ALICE, &[(attacker, AttackTarget::Player(BOB))], &mut dm);
    assert_eq!(
        g.player(ALICE).unwrap().hand.len(),
        hand_before + 1,
        "X = 1 (only Linked Crew)"
    );
}

const SHELTERING_PRAYERS: &str = "Mana cost: {W}\nType: Enchantment\nBasic lands each player controls have shroud as long as that player controls three or fewer lands. (They can't be the targets of spells or abilities.)";

/// Bob's basics have shroud at three lands and lose it at four; Cara's four
/// basics never do. Observed through Stone Rain's legal targets.
#[test]
fn sheltering_prayers_shrouds_basics_per_player_land_count() {
    let stone_rain = compile(
        "Stone Rain",
        "Mana cost: {2}{R}\nType: Sorcery\nDestroy target land.",
    );
    let prayers = compile("Sheltering Prayers", SHELTERING_PRAYERS);
    for bob_basics in [2usize, 3] {
        let mut g = game(3);
        put(&mut g, &prayers, ALICE, Zone::Battlefield);
        let bob: Vec<_> = (0..bob_basics)
            .map(|i| put(&mut g, &basic_plains(&format!("Bob Plains {i}")), BOB, Zone::Battlefield))
            .collect();
        let nonbasic = CardDefinitionBuilder::new(CardId::new(), "Bob Nonbasic")
            .card_types(vec![CardType::Land])
            .build();
        let bob_nonbasic = put(&mut g, &nonbasic, BOB, Zone::Battlefield);
        let cara: Vec<_> = (0..4)
            .map(|i| put(&mut g, &basic_plains(&format!("Cara Plains {i}")), CARA, Zone::Battlefield))
            .collect();
        add_mana(&mut g, ALICE, ManaSymbol::Red, 1);
        add_mana(&mut g, ALICE, ManaSymbol::Colorless, 2);
        let spell = g.create_object_from_definition(&stone_rain, ALICE, Zone::Hand);
        let mut dm = Script::default();
        perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Stone Rain");
        let offered = dm.offered_targets.concat();
        let bob_lands = bob_basics + 1;
        for id in &bob {
            assert_eq!(
                offered.contains(&Target::Object(*id)),
                bob_lands > 3,
                "Bob controls {bob_lands} lands"
            );
        }
        assert!(offered.contains(&Target::Object(bob_nonbasic)), "nonbasic never has shroud");
        for id in &cara {
            assert!(offered.contains(&Target::Object(*id)), "Cara controls four lands");
        }
    }
}

const LIEUTENANT_SULU: &str = "Mana cost: {2}{G}\nType: Legendary Creature — Human Pilot\nPower/Toughness: 3/3\nWhenever Lieutenant Sulu becomes tapped, he explores. (Reveal the top card of your library. Put that card into your hand if it's a land. Otherwise, put a +1/+1 counter on this creature, then put the card back or put it into your graveyard.)\n{2}{U}: Target creature can't be blocked this turn.";

/// "Lieutenant Sulu" is this card: tapping it (by attacking) explores once;
/// tapping another creature does nothing.
#[test]
fn lieutenant_sulu_explores_when_it_becomes_tapped() {
    let mut g = game(2);
    stock_library(&mut g, ALICE, 3);
    let sulu = put(&mut g, &compile("Lieutenant Hikaru Sulu", LIEUTENANT_SULU), ALICE, Zone::Battlefield);
    let other = put(&mut g, &creature("Ensign", 2, 2), ALICE, Zone::Battlefield);
    let mut dm = Script::default();
    declare_attackers(
        &mut g,
        ALICE,
        &[(sulu, AttackTarget::Player(BOB)), (other, AttackTarget::Player(BOB))],
        &mut dm,
    );
    assert_eq!(plus_counters(&g, sulu), 1, "revealed a nonland: Sulu explores once");
    assert_eq!(plus_counters(&g, other), 0);
}

const DEATHS_OASIS: &str = "Mana cost: {W}{B}{G}\nType: Enchantment\nWhenever a nontoken creature you control dies, mill two cards. Then return a creature card with lesser mana value than the creature that died from your graveyard to your hand.\n{1}, Sacrifice this enchantment: You gain life equal to the greatest mana value among creatures you control.";

/// "the creature that died" is the trigger's creature: only a card with
/// lesser mana value than it may return (not itself, not a bigger card).
#[test]
fn deaths_oasis_returns_only_a_card_with_lesser_mana_value_than_the_dead_creature() {
    let mut g = game(2);
    let oasis = put(&mut g, &compile("Death's Oasis", DEATHS_OASIS), ALICE, Zone::Battlefield);
    stock_library(&mut g, ALICE, 2);
    for (name, mana_value) in [("Milled Five", 5), ("Milled Three", 3)] {
        let def = body(name, vec![CardType::Creature], vec![], 2, 2, mana_value);
        g.create_object_from_definition(&def, ALICE, Zone::Library);
    }
    let dying = put(
        &mut g,
        &body("Dying Four", vec![CardType::Creature], vec![], 2, 2, 4),
        ALICE,
        Zone::Battlefield,
    );
    let filter = ObjectFilter {
        specific: Some(dying),
        ..Default::default()
    };
    let mut dm = Script {
        // Illegal picks first: a correct engine never offers them as legal.
        object_preference: vec!["Dying Four", "Milled Five", "Milled Three"],
        ..Script::default()
    };
    g.push_to_stack(StackEntry::ability(oasis, BOB, vec![Effect::destroy_all(filter)]));
    let mut queue = TriggerQueue::new();
    settle(&mut g, &mut queue, &mut dm);

    let hand = g.player(ALICE).unwrap().hand.to_vec();
    assert_eq!(
        names_in(&g, &hand),
        vec!["Milled Three".to_string()],
        "offered: {:?}",
        dm.offered_objects
    );
    let graveyard = names_in(&g, &g.player(ALICE).unwrap().graveyard.to_vec());
    assert!(graveyard.contains(&"Milled Five".to_string()), "{graveyard:?}");
    assert!(graveyard.contains(&"Dying Four".to_string()), "{graveyard:?}");
}

const DISCIPLE_OF_CAELUS_NIN: &str = "Mana cost: {4}{W}\nType: Creature — Human Wizard\nPower/Toughness: 3/4\nWhen this creature enters, starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.\nPermanents can't phase in.";

/// Only the permanents nobody chose phase out; chosen ones and the Disciple
/// stay.
#[test]
fn disciple_of_caelus_nin_phases_out_only_unchosen_permanents() {
    let mut g = game(2);
    let relic = body("Relic", vec![CardType::Artifact], vec![], 0, 0, 1);
    let alice: Vec<_> = (0..7).map(|_| put(&mut g, &relic, ALICE, Zone::Battlefield)).collect();
    let bob: Vec<_> = (0..2).map(|_| put(&mut g, &relic, BOB, Zone::Battlefield)).collect();
    let disciple = g.create_object_from_definition(
        &compile("Disciple of Caelus Nin", DISCIPLE_OF_CAELUS_NIN),
        ALICE,
        Zone::Stack,
    );
    let disciple = g
        .move_object_by_effect(disciple, Zone::Battlefield)
        .expect("Disciple enters");
    // Script falls back to selecting the first legal candidates up to five.
    let mut dm = Script::default();
    let mut queue = TriggerQueue::new();
    settle(&mut g, &mut queue, &mut dm);

    assert!(!g.is_phased_out(disciple));
    let chosen = |ids: &[ObjectId]| ids.iter().filter(|id| dm.chosen_objects.contains(id)).count();
    assert_eq!(chosen(&alice), 5, "Alice chooses up to five; offered {:?}", dm.offered_objects);
    assert_eq!(chosen(&bob), 2, "Bob chooses both of his");
    for id in alice.iter().chain(&bob) {
        assert_eq!(
            g.is_phased_out(*id),
            !dm.chosen_objects.contains(id),
            "exactly the permanents that weren't chosen phase out"
        );
    }
}

/// Enter `phase`/`step` and put that step's ordinary and delayed triggers on
/// the stack, then resolve them.
fn fire_step(g: &mut GameState, phase: Phase, step: Option<Step>, dm: &mut Script) {
    g.turn.phase = phase;
    g.turn.step = step;
    let mut queue = TriggerQueue::new();
    for event in ironsmith::triggers::check::generate_step_trigger_events_for_active_players(g) {
        for trigger in ironsmith::triggers::check_triggers(g, &event) {
            queue.add(trigger);
        }
        for trigger in ironsmith::triggers::check::check_delayed_triggers(g, &event) {
            queue.add(trigger);
        }
    }
    settle(g, &mut queue, dm);
}

fn legal_blockers(
    g: &mut GameState,
    combat: &CombatState,
    attacker: ObjectId,
    defender: PlayerId,
) -> Vec<ObjectId> {
    g.turn.step = Some(Step::DeclareBlockers);
    g.refresh_continuous_state().unwrap();
    compute_legal_blockers(g, combat, defender)
        .into_iter()
        .find(|option| option.attacker == attacker)
        .map(|option| option.valid_blockers)
        .unwrap_or_default()
}

fn activation_indices(g: &mut GameState, player: PlayerId, source: ObjectId) -> Vec<usize> {
    g.turn.priority_player = Some(player);
    compute_legal_actions(g, player)
        .unwrap()
        .into_iter()
        .filter_map(|action| match action {
            LegalAction::ActivateAbility {
                source: id,
                ability_index,
            } if id == source => Some(ability_index),
            _ => None,
        })
        .collect()
}

const HALLS_OF_MIST: &str = "Type: Land\nCumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\nCreatures that attacked during their controller's last turn can't attack.";

/// A creature that attacked on Alice's previous turn can't attack on her
/// next one; one that stayed home can.
#[test]
fn halls_of_mist_stops_creatures_that_attacked_during_their_controllers_last_turn() {
    let mut g = game(2);
    put(&mut g, &compile("Halls of Mist", HALLS_OF_MIST), BOB, Zone::Battlefield);
    let veteran = put(&mut g, &creature("Veteran", 2, 2), ALICE, Zone::Battlefield);
    let fresh = put(&mut g, &creature("Fresh Recruit", 2, 2), ALICE, Zone::Battlefield);
    let mut dm = Script::default();
    declare_attackers(&mut g, ALICE, &[(veteran, AttackTarget::Player(BOB))], &mut dm);
    g.combat = None;

    g.next_turn();
    assert_eq!(g.turn.active_player, BOB);
    g.next_turn();
    assert_eq!(g.turn.active_player, ALICE);
    ironsmith::turn::execute_untap_step(&mut g);
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::DeclareAttackers);
    g.turn.priority_player = Some(ALICE);
    g.refresh_continuous_state().unwrap();

    let attackers: Vec<_> = compute_legal_attackers(&g, &CombatState::default())
        .into_iter()
        .map(|option| option.creature)
        .collect();
    assert!(!attackers.contains(&veteran), "attacked last turn: {attackers:?}");
    assert!(attackers.contains(&fresh), "didn't attack last turn: {attackers:?}");
}

const AZURE_FLEET_ADMIRAL: &str = "Mana cost: {3}{U}\nType: Creature — Human Pirate\nPower/Toughness: 3/3\nWhen this creature enters, you become the monarch.\nThis creature can't be blocked by creatures the monarch controls.";

/// The defending player's creature can block the Admiral only while that
/// player isn't the monarch.
#[test]
fn azure_fleet_admiral_cant_be_blocked_by_the_monarchs_creatures() {
    let admiral_def = compile("Azure Fleet Admiral", AZURE_FLEET_ADMIRAL);
    for (monarch, can_block) in [(BOB, false), (ALICE, true)] {
        let mut g = game(2);
        g.monarch = Some(monarch);
        let admiral = put(&mut g, &admiral_def, ALICE, Zone::Battlefield);
        let guard = put(&mut g, &creature("Bob Guard", 2, 2), BOB, Zone::Battlefield);
        let mut dm = Script::default();
        let combat =
            declare_attackers(&mut g, ALICE, &[(admiral, AttackTarget::Player(BOB))], &mut dm);
        assert_eq!(g.monarch, Some(monarch));
        let blockers = legal_blockers(&mut g, &combat, admiral, BOB);
        assert_eq!(
            blockers.contains(&guard),
            can_block,
            "monarch {monarch:?}: blockers {blockers:?}"
        );
    }
}

const POMPOUS_GADABOUT: &str = "Mana cost: {2}{G}\nType: Creature — Human Citizen\nPower/Toughness: 4/2\nDuring your turn, this creature has hexproof.\nThis creature can't be blocked by creatures that don't have a name.";

/// A face-down creature has no name (CR 708.2a) and can't block the
/// Gadabout; a named creature can.
#[test]
fn pompous_gadabout_cant_be_blocked_by_nameless_creatures() {
    let mut g = game(2);
    let gadabout = put(&mut g, &compile("Pompous Gadabout", POMPOUS_GADABOUT), ALICE, Zone::Battlefield);
    let named = put(&mut g, &creature("Named Guard", 2, 2), BOB, Zone::Battlefield);
    let hidden = put(&mut g, &creature("Hidden Guard", 2, 2), BOB, Zone::Battlefield);
    assert!(g.set_face_down(hidden), "turned face down");
    let mut dm = Script::default();
    let combat =
        declare_attackers(&mut g, ALICE, &[(gadabout, AttackTarget::Player(BOB))], &mut dm);
    let blockers = legal_blockers(&mut g, &combat, gadabout, BOB);
    assert!(blockers.contains(&named), "{blockers:?}");
    assert!(!blockers.contains(&hidden), "a face-down creature has no name: {blockers:?}");
}

const SCHEMING_FENCE: &str = "Mana cost: {W}{U}\nType: Creature — Human Citizen\nPower/Toughness: 2/3\nAs this creature enters, you may choose a nonland permanent.\nActivated abilities of the chosen permanent can't be activated.\nThis creature has all activated abilities of the chosen permanent except for loyalty abilities. You may spend mana as though it were mana of any color to activate those abilities.";

const FENCE_TARGET_WALKER: &str = "Mana cost: {2}{U}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 3\n+1: Draw a card.\n{2}: You gain 2 life.";

/// Choosing Bob's planeswalker: the Fence gains its {2} ability but not its
/// +1 loyalty ability, and Bob can no longer activate the walker's ability.
#[test]
fn scheming_fence_gains_non_loyalty_activated_abilities_only() {
    let mut g = game(2);
    stock_library(&mut g, ALICE, 3);
    let walker = put(&mut g, &compile("Bob Walker", FENCE_TARGET_WALKER), BOB, Zone::Battlefield);
    g.object_mut(walker)
        .unwrap()
        .counters
        .insert(CounterType::Loyalty, 3);
    add_mana(&mut g, ALICE, ManaSymbol::White, 1);
    add_mana(&mut g, ALICE, ManaSymbol::Blue, 1);
    let spell = g.create_object_from_definition(
        &compile("Scheming Fence", SCHEMING_FENCE),
        ALICE,
        Zone::Hand,
    );
    let mut dm = Script {
        accept: true,
        object_preference: vec!["Bob Walker"],
        ..Script::default()
    };
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Scheming Fence");
    settle(&mut g, &mut queue, &mut dm);
    let fence = *find_named(&g, "Scheming Fence", Zone::Battlefield)
        .first()
        .expect("Fence entered");
    g.remove_summoning_sickness(fence);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 2);
    add_mana(&mut g, BOB, ManaSymbol::Colorless, 4);
    g.refresh_continuous_state().unwrap();

    let fence_abilities = activation_indices(&mut g, ALICE, fence);
    assert_eq!(
        fence_abilities.len(),
        1,
        "only the {{2}} ability, never the +1 loyalty ability: {fence_abilities:?}"
    );
    assert!(
        activation_indices(&mut g, BOB, walker).is_empty(),
        "the chosen permanent's activated abilities can't be activated"
    );

    let hand_before = g.player(ALICE).unwrap().hand.len();
    let mut queue = perform(&mut g, fence, ALICE, Some(fence_abilities[0]), &mut dm)
        .expect("activate the gained ability");
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(g.player(ALICE).unwrap().life, 22, "the gained ability is the {{2}} one");
    assert_eq!(g.player(ALICE).unwrap().hand.len(), hand_before, "no loyalty draw");
}

const RESCUE_FROM_THE_UNDERWORLD: &str = "Mana cost: {4}{B}\nType: Instant\nAs an additional cost to cast this spell, sacrifice a creature.\nChoose target creature card in your graveyard. Return that card and the sacrificed card to the battlefield under your control at the beginning of your next upkeep. Exile Rescue from the Underworld.";

/// Both the targeted card and the sacrificed creature stay in the graveyard
/// until Alice's next upkeep, then both return under her control.
#[test]
fn rescue_from_the_underworld_returns_both_cards_at_your_next_upkeep() {
    let mut g = game(2);
    let fodder = put(&mut g, &creature("Sacrifice Fodder", 2, 2), ALICE, Zone::Battlefield);
    let buried = put(&mut g, &creature("Buried Friend", 3, 3), ALICE, Zone::Graveyard);
    put(&mut g, &creature("Other Creature", 1, 1), ALICE, Zone::Battlefield);
    add_mana(&mut g, ALICE, ManaSymbol::Black, 1);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 4);
    let spell = g.create_object_from_definition(
        &compile("Rescue from the Underworld", RESCUE_FROM_THE_UNDERWORLD),
        ALICE,
        Zone::Hand,
    );
    let mut dm = Script {
        targets: vec![Target::Object(buried)],
        object_preference: vec!["Sacrifice Fodder"],
        ..Script::default()
    };
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Rescue");
    assert!(g.object(fodder).is_none_or(|o| o.zone != Zone::Battlefield), "sacrificed as a cost");
    settle(&mut g, &mut queue, &mut dm);
    for name in ["Sacrifice Fodder", "Buried Friend"] {
        assert_eq!(find_named(&g, name, Zone::Graveyard).len(), 1, "{name} waits for the upkeep");
    }
    assert_eq!(find_named(&g, "Rescue from the Underworld", Zone::Exile).len(), 1);

    g.next_turn();
    assert_eq!(g.turn.active_player, BOB);
    fire_step(&mut g, Phase::Beginning, Some(Step::Upkeep), &mut dm);
    for name in ["Sacrifice Fodder", "Buried Friend"] {
        assert!(find_named(&g, name, Zone::Battlefield).is_empty(), "not on Bob's upkeep");
    }
    g.next_turn();
    assert_eq!(g.turn.active_player, ALICE);
    fire_step(&mut g, Phase::Beginning, Some(Step::Upkeep), &mut dm);
    for name in ["Sacrifice Fodder", "Buried Friend"] {
        let returned = find_named(&g, name, Zone::Battlefield);
        assert_eq!(returned.len(), 1, "{name} returns at Alice's next upkeep");
        assert_eq!(g.controller_of_id(returned[0]), Some(ALICE));
    }
}

const STEADFAST_ARMASAUR: &str = "Mana cost: {3}{W}\nType: Creature — Dinosaur\nPower/Toughness: 2/3\nVigilance\n{1}{W}, {T}: This creature deals damage equal to its toughness to target creature blocking or blocked by it.";

/// Attacking Armasaur may target only the creature blocking it, not another
/// attacker's blocker or an idle creature; damage equals its toughness.
#[test]
fn steadfast_armasaur_targets_only_a_creature_in_combat_with_it() {
    let mut g = game(2);
    let armasaur = put(&mut g, &compile("Steadfast Armasaur", STEADFAST_ARMASAUR), ALICE, Zone::Battlefield);
    let other_attacker = put(&mut g, &creature("Other Attacker", 2, 6), ALICE, Zone::Battlefield);
    let its_blocker = put(&mut g, &creature("Armasaur Blocker", 1, 6), BOB, Zone::Battlefield);
    let other_blocker = put(&mut g, &creature("Other Blocker", 1, 6), BOB, Zone::Battlefield);
    let idle = put(&mut g, &creature("Idle Creature", 1, 6), BOB, Zone::Battlefield);
    let mut dm = Script::default();
    let mut combat = declare_attackers(
        &mut g,
        ALICE,
        &[
            (armasaur, AttackTarget::Player(BOB)),
            (other_attacker, AttackTarget::Player(BOB)),
        ],
        &mut dm,
    );
    declare_blockers(
        &mut g,
        &mut combat,
        &[(its_blocker, armasaur), (other_blocker, other_attacker)],
        BOB,
        &mut dm,
    );
    add_mana(&mut g, ALICE, ManaSymbol::White, 1);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 1);
    let index = activated_index(&compile("Steadfast Armasaur", STEADFAST_ARMASAUR), Zone::Battlefield);
    let mut queue = perform(&mut g, armasaur, ALICE, Some(index), &mut dm)
        .expect("vigilant Armasaur is untapped and can activate");
    assert_eq!(dm.offered_targets, vec![vec![Target::Object(its_blocker)]]);
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(g.damage_on(its_blocker), 3, "damage equal to its toughness");
    assert_eq!(g.damage_on(other_blocker), 0);
    assert_eq!(g.damage_on(idle), 0);
}

const GLYPH_OF_DOOM: &str = "Mana cost: {B}\nType: Instant\nChoose target Wall creature. At this turn's next end of combat, destroy all creatures that were blocked by that creature this turn.";

/// At end of combat, the creature the Wall blocked is destroyed even though
/// the block is over by then; the creature another blocker blocked survives.
#[test]
fn glyph_of_doom_destroys_creatures_the_wall_blocked_this_turn() {
    let mut g = game(2);
    let wall_def = body("Fixture Wall", vec![CardType::Creature], vec![Subtype::Wall], 0, 6, 2);
    let wall = put(&mut g, &wall_def, ALICE, Zone::Battlefield);
    let other_blocker = put(&mut g, &creature("Alice Blocker", 0, 6), ALICE, Zone::Battlefield);
    let blocked = put(&mut g, &creature("Walled Attacker", 2, 6), BOB, Zone::Battlefield);
    let elsewhere = put(&mut g, &creature("Other Attacker", 2, 6), BOB, Zone::Battlefield);
    let mut dm = Script::default();
    let mut combat = declare_attackers(
        &mut g,
        BOB,
        &[
            (blocked, AttackTarget::Player(ALICE)),
            (elsewhere, AttackTarget::Player(ALICE)),
        ],
        &mut dm,
    );
    declare_blockers(
        &mut g,
        &mut combat,
        &[(wall, blocked), (other_blocker, elsewhere)],
        ALICE,
        &mut dm,
    );
    add_mana(&mut g, ALICE, ManaSymbol::Black, 1);
    let spell = g.create_object_from_definition(&compile("Glyph of Doom", GLYPH_OF_DOOM), ALICE, Zone::Hand);
    dm.targets = vec![Target::Object(wall)];
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Glyph of Doom");
    settle(&mut g, &mut queue, &mut dm);
    assert!(g.object(blocked).is_some_and(|o| o.zone == Zone::Battlefield), "not before end of combat");

    // The block is history now; "were blocked ... this turn" still reads it.
    g.combat = None;
    fire_step(&mut g, Phase::Combat, Some(Step::EndCombat), &mut dm);
    assert!(find_named(&g, "Walled Attacker", Zone::Battlefield).is_empty(), "blocked by the Wall");
    assert!(g.object(elsewhere).is_some_and(|o| o.zone == Zone::Battlefield));
    assert!(g.object(wall).is_some_and(|o| o.zone == Zone::Battlefield));
}

const UNBURY: &str = "Mana cost: {1}{B}\nType: Instant\nChoose one —\n• Return target creature card from your graveyard to your hand.\n• Return two target creature cards that share a creature type from your graveyard to your hand.";

/// The two-card mode needs targets that share a creature type: two Elves
/// return; an Elf and a Goblin can't be chosen together.
#[test]
fn unbury_two_targets_must_share_a_creature_type() {
    let unbury = compile("Unbury", UNBURY);
    let card = |name: &str, subtype: Subtype| {
        body(name, vec![CardType::Creature], vec![subtype], 1, 1, 1)
    };
    for (pair, legal) in [(["Elf A", "Elf B"], true), (["Elf A", "Goblin C"], false)] {
        let mut g = game(2);
        let elf_a = put(&mut g, &card("Elf A", Subtype::Elf), ALICE, Zone::Graveyard);
        let elf_b = put(&mut g, &card("Elf B", Subtype::Elf), ALICE, Zone::Graveyard);
        let goblin = put(&mut g, &card("Goblin C", Subtype::Goblin), ALICE, Zone::Graveyard);
        let id = |name: &str| match name {
            "Elf A" => elf_a,
            "Elf B" => elf_b,
            _ => goblin,
        };
        add_mana(&mut g, ALICE, ManaSymbol::Black, 1);
        add_mana(&mut g, ALICE, ManaSymbol::Colorless, 1);
        let spell = g.create_object_from_definition(&unbury, ALICE, Zone::Hand);
        let mut dm = Script {
            targets: pair.iter().map(|name| Target::Object(id(name))).collect(),
            option_preference: Some("two target"),
            ..Script::default()
        };
        match perform(&mut g, spell, ALICE, None, &mut dm) {
            Ok(mut queue) => settle(&mut g, &mut queue, &mut dm),
            Err(error) => assert!(!legal, "the legal pair must be castable: {error}"),
        }
        let hand = names_in(&g, &g.player(ALICE).unwrap().hand.to_vec());
        if legal {
            assert_eq!(hand.len(), 2, "both Elves return: {hand:?}");
            assert!(hand.contains(&"Elf A".to_string()) && hand.contains(&"Elf B".to_string()));
        } else {
            assert!(
                !(hand.contains(&"Elf A".to_string()) && hand.contains(&"Goblin C".to_string())),
                "an Elf and a Goblin don't share a creature type: {hand:?}"
            );
        }
    }
}

const CELESTIAL_JUDGMENT: &str = "Mana cost: {4}{W}{W}\nType: Sorcery\nFor each different power among creatures on the battlefield, choose a creature with that power. Destroy each creature not chosen this way.";

/// Powers 2, 2, 3, 1: one creature per distinct power is kept (so only one
/// of the two 2-power creatures), everything else is destroyed. The default
/// chooser takes the first legal candidates, which would keep both twins if
/// the distinct-power rule weren't enforced.
#[test]
fn celestial_judgment_keeps_one_creature_per_distinct_power() {
    let mut g = game(3);
    let twin_a = put(&mut g, &creature("Twin A", 2, 2), BOB, Zone::Battlefield);
    let twin_b = put(&mut g, &creature("Twin B", 2, 2), BOB, Zone::Battlefield);
    let big = put(&mut g, &creature("Big C", 3, 3), ALICE, Zone::Battlefield);
    let small = put(&mut g, &creature("Small D", 1, 1), CARA, Zone::Battlefield);
    add_mana(&mut g, ALICE, ManaSymbol::White, 2);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 4);
    let spell = g.create_object_from_definition(
        &compile("Celestial Judgment", CELESTIAL_JUDGMENT),
        ALICE,
        Zone::Hand,
    );
    let mut dm = Script::default();
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Celestial Judgment");
    settle(&mut g, &mut queue, &mut dm);

    let alive = |id: ObjectId| g.object(id).is_some_and(|o| o.zone == Zone::Battlefield);
    let survivors: Vec<_> = [twin_a, twin_b, big, small].into_iter().filter(|id| alive(*id)).collect();
    let mut powers: Vec<_> = survivors.iter().map(|id| g.calculated_power(*id)).collect();
    powers.sort();
    assert_eq!(
        powers,
        vec![Some(1), Some(2), Some(3)],
        "one survivor per distinct power; offered {:?}",
        dm.offered_objects
    );
    assert!(alive(twin_a) != alive(twin_b), "exactly one 2-power creature is kept");
    assert!(alive(big) && alive(small));
}

const RUNED_HALO: &str = "Mana cost: {W}{W}\nType: Enchantment\nAs this enchantment enters, choose a card name.\nYou have protection from the chosen card name. (You can't be targeted, dealt damage, or enchanted by anything with that name.)";

fn runed_halo_game(name: &'static str) -> GameState {
    let mut g = game(2);
    add_mana(&mut g, ALICE, ManaSymbol::White, 2);
    let spell = g.create_object_from_definition(&compile("Runed Halo", RUNED_HALO), ALICE, Zone::Hand);
    let mut dm = Script {
        text: Some(name),
        ..Script::default()
    };
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Runed Halo");
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(find_named(&g, "Runed Halo", Zone::Battlefield).len(), 1);
    g.refresh_continuous_state().unwrap();
    g
}

/// Naming Lightning Bolt: Bolt can't target Alice and damage from anything
/// named Lightning Bolt is prevented; Shock is unaffected.
#[test]
fn runed_halo_gives_protection_from_the_chosen_name_only() {
    let bolt = compile(
        "Lightning Bolt",
        "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.",
    );
    let shock = compile(
        "Shock",
        "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target.",
    );
    for (def, protected) in [(&bolt, true), (&shock, false)] {
        let mut g = runed_halo_game("Lightning Bolt");
        add_mana(&mut g, BOB, ManaSymbol::Red, 1);
        let spell = g.create_object_from_definition(def, BOB, Zone::Hand);
        let mut dm = Script::default();
        perform(&mut g, spell, BOB, None, &mut dm).expect("cast the burn spell");
        let offered = dm.offered_targets.concat();
        assert_eq!(
            !offered.contains(&Target::Player(ALICE)),
            protected,
            "{}: offered {offered:?}",
            def.name()
        );
        assert!(offered.contains(&Target::Player(BOB)));
    }

    let mut g = runed_halo_game("Lightning Bolt");
    let named = put(&mut g, &creature("Lightning Bolt", 1, 1), BOB, Zone::Battlefield);
    let other = put(&mut g, &creature("Shock", 1, 1), BOB, Zone::Battlefield);
    resolve_neutral(
        &mut g,
        named,
        BOB,
        vec![Effect::deal_damage(3i32, ChooseSpec::SpecificPlayer(ALICE))],
    );
    assert_eq!(g.player(ALICE).unwrap().life, 20, "damage from the chosen name is prevented");
    resolve_neutral(
        &mut g,
        other,
        BOB,
        vec![Effect::deal_damage(2i32, ChooseSpec::SpecificPlayer(ALICE))],
    );
    assert_eq!(g.player(ALICE).unwrap().life, 18, "another name's damage is dealt");
}

const NEFARIOUS_LICH: &str = "Mana cost: {B}{B}{B}{B}\nType: Enchantment\nIf damage would be dealt to you, exile that many cards from your graveyard instead. If you can't, you lose the game.\nIf you would gain life, draw that many cards instead.\nWhen this enchantment leaves the battlefield, you lose the game.";

/// 3 damage exiles exactly 3 of Alice's 5 graveyard cards instead of costing
/// life; 3 more damage with only 2 left makes her lose.
#[test]
fn nefarious_lich_exiles_that_many_graveyard_cards_instead_of_damage() {
    let mut g = game(3);
    put(&mut g, &compile("Nefarious Lich", NEFARIOUS_LICH), ALICE, Zone::Battlefield);
    let filler = body("Graveyard filler", vec![CardType::Artifact], vec![], 0, 0, 1);
    for _ in 0..5 {
        g.create_object_from_definition(&filler, ALICE, Zone::Graveyard);
    }
    let pinger = put(&mut g, &creature("Pinger", 1, 1), BOB, Zone::Battlefield);
    let alice_exiled = |g: &GameState| {
        g.exile
            .iter()
            .filter(|id| g.object(**id).is_some_and(|o| o.owner == ALICE))
            .count()
    };

    resolve_neutral(
        &mut g,
        pinger,
        BOB,
        vec![Effect::deal_damage(3i32, ChooseSpec::SpecificPlayer(ALICE))],
    );
    assert_eq!(g.player(ALICE).unwrap().life, 20, "the damage is replaced");
    assert_eq!(g.player(ALICE).unwrap().graveyard.len(), 2);
    assert_eq!(alice_exiled(&g), 3, "exactly that many cards are exiled");

    resolve_neutral(
        &mut g,
        pinger,
        BOB,
        vec![Effect::deal_damage(3i32, ChooseSpec::SpecificPlayer(ALICE))],
    );
    let alice = g.player(ALICE).unwrap();
    assert!(alice.has_lost || !alice.is_in_game(), "can't exile 3 of 2: Alice loses");
}

const IDRIS: &str = "Mana cost: {1}{U}{R}\nType: Legendary Creature — Human Incarnation\nPower/Toughness: 3/3\nVanishing 3\nImprint — When Idris enters, exile another artifact you control until Idris leaves the battlefield.\nIdris has all activated and triggered abilities of the exiled card and gets +X/+X, where X is the exiled card's mana value.";

/// Imprinting a mana value 4 artifact makes Idris 7/7 with that artifact's
/// activated ability, and only that one: an unrelated card already in exile
/// lends nothing.
#[test]
fn idris_gets_the_imprinted_cards_abilities_and_plus_x_for_its_mana_value() {
    let mut g = game(2);
    let relic = put(
        &mut g,
        &compile("Imprint Relic", "Mana cost: {4}\nType: Artifact\n{T}: You gain 1 life."),
        ALICE,
        Zone::Battlefield,
    );
    put(
        &mut g,
        &compile("Exiled Stranger", "Mana cost: {1}\nType: Artifact\n{T}: You gain 3 life."),
        ALICE,
        Zone::Exile,
    );
    add_mana(&mut g, ALICE, ManaSymbol::Blue, 1);
    add_mana(&mut g, ALICE, ManaSymbol::Red, 1);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 1);
    let spell = g.create_object_from_definition(&compile("Idris, Soul of the TARDIS", IDRIS), ALICE, Zone::Hand);
    let mut dm = Script {
        object_preference: vec!["Imprint Relic"],
        ..Script::default()
    };
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Idris");
    settle(&mut g, &mut queue, &mut dm);
    let idris = *find_named(&g, "Idris, Soul of the TARDIS", Zone::Battlefield)
        .first()
        .expect("Idris entered");
    assert!(g.object(relic).is_none_or(|o| o.zone != Zone::Battlefield), "relic imprinted");
    assert_eq!(find_named(&g, "Imprint Relic", Zone::Exile).len(), 1);
    g.remove_summoning_sickness(idris);
    g.refresh_continuous_state().unwrap();

    assert_eq!(g.calculated_power(idris), Some(7), "3 + the exiled card's mana value 4");
    assert_eq!(g.calculated_toughness(idris), Some(7));
    let abilities = activation_indices(&mut g, ALICE, idris);
    assert_eq!(abilities.len(), 1, "only the imprinted card's ability: {abilities:?}");
    let mut queue = perform(&mut g, idris, ALICE, Some(abilities[0]), &mut dm).expect("activate");
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(g.player(ALICE).unwrap().life, 21, "the Relic's ability, not the Stranger's");
}

const ANJE_FALKENRATH: &str = "Mana cost: {1}{B}{R}\nType: Legendary Creature — Vampire\nPower/Toughness: 1/3\nHaste\n{T}, Discard a card: Draw a card.\nWhenever you discard a card, if it has madness, untap Anje Falkenrath.";

const FALKENRATH_GORGER: &str = "Mana cost: {R}\nType: Creature — Vampire Berserker\nPower/Toughness: 2/1\nEach Vampire creature card you own that isn't on the battlefield has madness. The madness cost is equal to its mana cost. (If you discard a card with madness, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)";

/// Gorger grants madness to Vampire cards in hand: discarding one to Anje's
/// ability untaps Anje; discarding a non-Vampire (no madness) doesn't.
#[test]
fn anje_untaps_only_when_the_discarded_card_has_granted_madness() {
    let anje_def = compile("Anje Falkenrath", ANJE_FALKENRATH);
    let gorger_def = compile("Falkenrath Gorger", FALKENRATH_GORGER);
    let index = activated_index(&anje_def, Zone::Battlefield);
    for (discard, untaps) in [("Vampire Card", true), ("Plain Card", false)] {
        let mut g = game(2);
        stock_library(&mut g, ALICE, 3);
        let anje = put(&mut g, &anje_def, ALICE, Zone::Battlefield);
        put(&mut g, &gorger_def, ALICE, Zone::Battlefield);
        let vampire = body("Vampire Card", vec![CardType::Creature], vec![Subtype::Vampire], 2, 2, 2);
        g.create_object_from_definition(&vampire, ALICE, Zone::Hand);
        let plain = body("Plain Card", vec![CardType::Artifact], vec![], 0, 0, 2);
        g.create_object_from_definition(&plain, ALICE, Zone::Hand);
        g.refresh_continuous_state().unwrap();
        let mut dm = Script {
            object_preference: vec![discard],
            // Decline the madness cast; the card goes to the graveyard.
            accept: false,
            ..Script::default()
        };
        let mut queue = perform(&mut g, anje, ALICE, Some(index), &mut dm).expect("activate Anje");
        assert!(g.is_tapped(anje), "tapped as a cost");
        settle(&mut g, &mut queue, &mut dm);
        assert!(
            find_named(&g, discard, Zone::Hand).is_empty(),
            "{discard} was discarded; offered {:?}",
            dm.offered_objects
        );
        assert_eq!(!g.is_tapped(anje), untaps, "discarded {discard}");
    }
}

const FLASHBACK_WATCHER: &str = "Mana cost: {1}{U}\nType: Creature — Human Wizard\nPower/Toughness: 1/1\nWhenever you discard a card, if it has flashback, draw a card.";

const DISCARD_OUTLET: &str = "Mana cost: {1}\nType: Artifact\n{1}, Discard a card: You gain 1 life.";

/// Discarding Think Twice (flashback) draws a card; discarding a card without
/// flashback doesn't. Snapcaster Mage can't set this up: its flashback is
/// granted to a card already in the graveyard, and a discarded card comes
/// from hand, so the printed keyword stands in for "has flashback".
#[test]
fn discard_trigger_draws_only_for_a_card_with_flashback() {
    let watcher = compile("Flashback Watcher", FLASHBACK_WATCHER);
    let outlet_def = compile("Discard Outlet", DISCARD_OUTLET);
    let think_twice = compile(
        "Think Twice",
        "Mana cost: {1}{U}\nType: Instant\nDraw a card.\nFlashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
    );
    let index = activated_index(&outlet_def, Zone::Battlefield);
    for (discard, draws) in [("Think Twice", true), ("Plain Card", false)] {
        let mut g = game(2);
        stock_library(&mut g, ALICE, 3);
        put(&mut g, &watcher, ALICE, Zone::Battlefield);
        let outlet = put(&mut g, &outlet_def, ALICE, Zone::Battlefield);
        g.create_object_from_definition(&think_twice, ALICE, Zone::Hand);
        let plain = body("Plain Card", vec![CardType::Artifact], vec![], 0, 0, 2);
        g.create_object_from_definition(&plain, ALICE, Zone::Hand);
        add_mana(&mut g, ALICE, ManaSymbol::Colorless, 1);
        let mut dm = Script {
            object_preference: vec![discard],
            ..Script::default()
        };
        let library_before = g.player(ALICE).unwrap().library.len();
        let mut queue = perform(&mut g, outlet, ALICE, Some(index), &mut dm).expect("activate outlet");
        settle(&mut g, &mut queue, &mut dm);
        assert_eq!(find_named(&g, discard, Zone::Graveyard).len(), 1, "{discard} was discarded");
        assert_eq!(
            library_before - g.player(ALICE).unwrap().library.len(),
            usize::from(draws),
            "discarded {discard}"
        );
    }
}

const PLANESWALKER_HUNTER: &str = "Mana cost: {2}\nType: Creature — Soldier\nPower/Toughness: 2/2\nWhenever this creature attacks a planeswalker, draw a card.";

/// The trigger draws when the creature attacks a planeswalker and not when
/// it attacks a player.
#[test]
fn attacks_a_planeswalker_trigger_ignores_attacks_on_players() {
    let hunter = compile("Planeswalker Hunter", PLANESWALKER_HUNTER);
    let walker = body("Fixture walker", vec![CardType::Planeswalker], vec![], 0, 0, 3);
    for at_walker in [true, false] {
        let mut g = game(2);
        stock_library(&mut g, ALICE, 3);
        let attacker = put(&mut g, &hunter, ALICE, Zone::Battlefield);
        let bob_walker = put(&mut g, &walker, BOB, Zone::Battlefield);
        g.object_mut(bob_walker)
            .unwrap()
            .counters
            .insert(CounterType::Loyalty, 5);
        let target = if at_walker {
            AttackTarget::Planeswalker(bob_walker)
        } else {
            AttackTarget::Player(BOB)
        };
        let library_before = g.player(ALICE).unwrap().library.len();
        let mut dm = Script::default();
        declare_attackers(&mut g, ALICE, &[(attacker, target)], &mut dm);
        assert_eq!(
            library_before - g.player(ALICE).unwrap().library.len(),
            usize::from(at_walker),
            "attacking a planeswalker: {at_walker}"
        );
    }
}

const GRAVE_WARD: &str = "Mana cost: {1}{B}\nType: Instant\nIf a creature card would be put into your graveyard from anywhere this turn, exile it instead.";

/// After the spell resolves, Alice's dying creature is exiled instead, while
/// her destroyed noncreature permanent still goes to the graveyard.
#[test]
fn creature_cards_go_to_exile_instead_of_the_graveyard_this_turn() {
    let mut g = game(2);
    let bear = put(&mut g, &creature("Doomed Bear", 2, 2), ALICE, Zone::Battlefield);
    let relic = put(
        &mut g,
        &body("Doomed Relic", vec![CardType::Artifact], vec![], 0, 0, 1),
        ALICE,
        Zone::Battlefield,
    );
    add_mana(&mut g, ALICE, ManaSymbol::Black, 1);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 1);
    let spell = g.create_object_from_definition(&compile("Grave Ward", GRAVE_WARD), ALICE, Zone::Hand);
    let mut dm = Script::default();
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Grave Ward");
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(
        find_named(&g, "Grave Ward", Zone::Graveyard).len(),
        1,
        "the instant itself is not a creature card"
    );

    destroy(&mut g, relic, bear);
    assert_eq!(find_named(&g, "Doomed Bear", Zone::Exile).len(), 1, "creature card exiled instead");
    assert!(find_named(&g, "Doomed Bear", Zone::Graveyard).is_empty());
    destroy(&mut g, bear, relic);
    assert_eq!(find_named(&g, "Doomed Relic", Zone::Graveyard).len(), 1, "noncreature card is unaffected");
}

const SPIDER_LEGENDS: &str = "Mana cost: {1}{G}\nType: Enchantment\nThe \"legend rule\" doesn't apply to Spiders.";

/// With the static on the battlefield, two same-named legendary Spiders
/// under one controller both survive SBAs; two same-named legendary Bears
/// still lose one to the legend rule.
#[test]
fn legend_rule_exemption_applies_only_to_spiders() {
    let mut g = game(2);
    put(&mut g, &compile("Spider Legends", SPIDER_LEGENDS), ALICE, Zone::Battlefield);
    let legend = |name: &str, subtype: Subtype| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    };
    let spider = legend("Twin Spider", Subtype::Spider);
    let bear = legend("Twin Bear", Subtype::Bear);
    for _ in 0..2 {
        put(&mut g, &spider, ALICE, Zone::Battlefield);
        put(&mut g, &bear, ALICE, Zone::Battlefield);
    }
    let mut dm = Script::default();
    let mut queue = TriggerQueue::new();
    settle(&mut g, &mut queue, &mut dm);
    assert_eq!(find_named(&g, "Twin Spider", Zone::Battlefield).len(), 2, "Spiders are exempt");
    assert_eq!(find_named(&g, "Twin Bear", Zone::Battlefield).len(), 1, "the legend rule still applies");
}

const UNTAP_PAIR: &str = "Mana cost: {1}{G}\nType: Instant\nUntap up to one target creature and up to one target land.";

/// One creature target and one land target, each from its own pool; both
/// untap and nothing else does.
#[test]
fn untap_up_to_one_creature_and_up_to_one_land() {
    let mut g = game(2);
    let bear = put(&mut g, &creature("Tapped Bear", 2, 2), ALICE, Zone::Battlefield);
    let other = put(&mut g, &creature("Other Tapped Bear", 2, 2), ALICE, Zone::Battlefield);
    let land = put(&mut g, &basic_plains("Tapped Plains"), ALICE, Zone::Battlefield);
    for id in [bear, other, land] {
        g.tap(id);
    }
    add_mana(&mut g, ALICE, ManaSymbol::Green, 1);
    add_mana(&mut g, ALICE, ManaSymbol::Colorless, 1);
    let spell = g.create_object_from_definition(&compile("Untap Pair", UNTAP_PAIR), ALICE, Zone::Hand);
    let mut dm = Script {
        targets: vec![Target::Object(bear), Target::Object(land)],
        ..Script::default()
    };
    let mut queue = perform(&mut g, spell, ALICE, None, &mut dm).expect("cast Untap Pair");
    assert_eq!(dm.offered_targets.len(), 2, "two target requirements: {:?}", dm.offered_targets);
    assert!(!dm.offered_targets[0].contains(&Target::Object(land)), "first target is a creature");
    assert!(!dm.offered_targets[1].contains(&Target::Object(bear)), "second target is a land");
    settle(&mut g, &mut queue, &mut dm);
    assert!(!g.is_tapped(bear));
    assert!(!g.is_tapped(land));
    assert!(g.is_tapped(other), "only the targets untap");
}
