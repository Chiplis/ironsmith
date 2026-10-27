//! CR audit (pending-entry pass, package p18: leftover rows): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{
    BooleanContext, DistributeContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardId, CardType, DecisionMaker, ObjectId, Phase, PriorityLoopState, PriorityResponse, Step,
    Target, Zone,
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

/// Drive a priority action through every casting/activation decision until
/// the object reaches the stack. Errors are returned, not unwrapped.
fn act_with<D: DecisionMaker>(
    g: &mut GameState,
    q: &mut TriggerQueue,
    action: LegalAction,
    dm: &mut D,
) -> Result<(), String> {
    let before = g.stack.len();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| format!("{e:?}"))?;
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && g.stack.len() > before {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("stopped before reaching the stack: {progress:?}"));
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| format!("{e:?}"))?;
    }
    Err("did not finish".into())
}

fn cast_action(g: &GameState, id: ObjectId) -> Option<LegalAction> {
    let player = g.turn.priority_player.expect("someone has priority");
    compute_legal_actions(g, player)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
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

/// Records every decision (kind, player, description, choices) and answers
/// with simple preferences.
#[derive(Default)]
struct Recorder {
    log: Vec<(String, PlayerId, String, Vec<String>)>,
    boolean: bool,
    prefer_objects: Vec<String>,
    later_prefer_objects: Vec<String>,
    object_choices: usize,
    prefer_targets: Vec<Target>,
    distribute_each: u32,
    prefer_option: Option<String>,
}

impl DecisionMaker for Recorder {
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.log.push(("boolean".into(), c.player, c.description.clone(), vec![]));
        self.boolean && c.can_accept
    }
    fn decide_objects(&mut self, _g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        self.log.push((
            "objects".into(),
            c.player,
            c.description.clone(),
            c.candidates.iter().map(|x| format!("{}{}", x.name, if x.legal { "" } else { "(illegal)" })).collect(),
        ));
        let preferences = if self.object_choices > 0 && !self.later_prefer_objects.is_empty() {
            &self.later_prefer_objects
        } else {
            &self.prefer_objects
        };
        self.object_choices += 1;
        for name in preferences {
            let picks: Vec<_> = c
                .candidates
                .iter()
                .filter(|x| x.legal && &x.name == name)
                .map(|x| x.id)
                .take(c.max.unwrap_or(usize::MAX).max(c.min).max(1))
                .collect();
            if !picks.is_empty() && picks.len() >= c.min {
                return picks.into_iter().take(c.min.max(1)).collect();
            }
        }
        c.candidates.iter().filter(|x| x.legal).map(|x| x.id).take(c.min).collect()
    }
    fn decide_options(&mut self, _: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        self.log.push((
            "options".into(),
            c.player,
            c.description.clone(),
            c.options.iter().map(|o| o.description.clone()).collect(),
        ));
        if let Some(want) = &self.prefer_option
            && let Some(o) = c.options.iter().find(|o| o.legal && o.description.contains(want.as_str()))
        {
            return vec![o.index];
        }
        c.options.iter().filter(|o| o.legal).take(c.min.max(if c.max > 0 { 1 } else { 0 }).min(c.max)).map(|o| o.index).collect()
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.log.push(("targets".into(), c.player, c.context.clone(), vec![]));
        let mut out = vec![];
        for r in &c.requirements {
            let preferred: Vec<Target> = self
                .prefer_targets
                .iter()
                .filter(|t| r.legal_targets.contains(t) && !out.contains(*t))
                .cloned()
                .collect();
            if preferred.is_empty() {
                out.extend(r.legal_targets.iter().take(r.min_targets).cloned());
            } else {
                out.extend(preferred.into_iter().take(r.max_targets.unwrap_or(usize::MAX)));
            }
        }
        out
    }
    fn decide_distribute(&mut self, _: &GameState, c: &DistributeContext) -> Vec<(Target, u32)> {
        self.log.push((
            "distribute".into(),
            c.player,
            format!("total={} {}", c.total, c.description),
            c.targets.iter().map(|t| t.name.clone()).collect(),
        ));
        c.targets.iter().map(|t| (t.target.clone(), self.distribute_each)).collect()
    }
}

// ---------------------------------------------------------------------------
// 101.4d: APNAP restart
// ---------------------------------------------------------------------------

/// Bob (nonactive) returns two Clones simultaneously. His choice to have the
/// first Clone enter as a copy of Archangel of Strife makes every player,
/// including the active player Alice, choose war or peace. APNAP order is then
/// restarted for all outstanding choices, so Alice's war-or-peace choice comes
/// before Bob's still-outstanding copy choice for his second Clone.
#[test]
fn cr_101_4d_nonactive_clone_copy_choice_restarts_apnap_for_outstanding_choices() {
    let archangel = card(
        "Archangel of Strife",
        "Mana cost: {5}{W}{W}\nType: Creature — Angel\nPower/Toughness: 6/6\nFlying\nAs Archangel of Strife enters, each player chooses war or peace.\nCreatures controlled by players who chose war get +3/+0.\nCreatures controlled by players who chose peace get +0/+3.",
    );
    let clone = card(
        "Clone",
        "Mana cost: {3}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 0/0\nYou may have this creature enter as a copy of any creature on the battlefield.",
    );
    let mass = card(
        "Audit Mass Return",
        "Mana cost: {0}\nType: Sorcery\nReturn all creature cards from all graveyards to the battlefield under their owners' control.",
    );
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    g.create_object_from_definition(&archangel, alice(), Zone::Battlefield);
    g.create_object_from_definition(&clone, bob(), Zone::Graveyard);
    g.create_object_from_definition(&clone, bob(), Zone::Graveyard);
    let spell = g.create_object_from_definition(&mass, alice(), Zone::Hand);
    let mut dm = Recorder {
        boolean: true,
        prefer_objects: vec!["Archangel of Strife".into()],
        prefer_option: Some("copy of Archangel".into()),
        ..Default::default()
    };
    let action = cast_action(&g, spell).expect("fixture: mass return castable");
    act_with(&mut g, &mut q, action, &mut dm).expect("fixture: cast");
    let cast_len = dm.log.len();
    settle_with(&mut g, &mut q, &mut dm);
    let log: Vec<_> = dm.log[cast_len..].to_vec();
    eprintln!("p18-101.4d-log={log:#?}");
    let on_bf = g
        .battlefield
        .iter()
        .filter(|id| g.object(**id).is_some_and(|o| o.owner == bob()))
        .count();
    assert_eq!(on_bf, 2, "fixture: both Clones returned; log {log:#?}");
    let is_war_peace = |e: &(String, PlayerId, String, Vec<String>)| {
        let all = format!("{} {}", e.2, e.3.join(" ")).to_lowercase();
        all.contains("war") || all.contains("peace")
    };
    let copy_choices: Vec<usize> = log
        .iter()
        .enumerate()
        .filter(|(_, e)| e.1 == bob() && !is_war_peace(e))
        .map(|(i, _)| i)
        .collect();
    let alice_wp = log.iter().position(|e| e.1 == alice() && is_war_peace(e));
    assert!(
        alice_wp.is_some() && copy_choices.len() >= 2,
        "fixture: Alice is asked war-or-peace and Bob makes copy choices for both Clones; log {log:#?}"
    );
    // Bob's second Clone's copy decision(s): the last Bob non-war/peace decision.
    let second_clone_choice = *copy_choices.last().unwrap();
    assert!(
        alice_wp.unwrap() < second_clone_choice,
        "CR 101.4d: after the nonactive player's copy choice forces the active player to choose war or peace, APNAP restarts, so Alice chooses before Bob's outstanding second-Clone choice; log {log:#?}"
    );
}

// ---------------------------------------------------------------------------
// 601.5: choices may consider later cost-payment choices
// ---------------------------------------------------------------------------

/// The damage being divided is the power of the creature sacrificed as an
/// additional cost, which is chosen only at 601.2h. Dividing 3 damage among
/// three targets is available only if a 3-power creature will be sacrificed,
/// and CR 601.5 lets the caster consider that later choice.
#[test]
fn cr_601_5_division_may_consider_the_creature_sacrificed_later_as_a_cost() {
    let spell_def = card(
        "Audit Divided Fling",
        "Mana cost: {0}\nType: Instant\nAs an additional cost to cast this spell, sacrifice a creature.\nThis spell deals damage equal to the sacrificed creature's power divided as you choose among any number of targets.",
    );
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let big = g.create_object_from_definition(&creature("Big Fodder", 3, 3), alice(), Zone::Battlefield);
    let big_stable = g.object(big).unwrap().stable_id;
    let small = g.create_object_from_definition(&creature("Small Fodder", 1, 1), alice(), Zone::Battlefield);
    let foes: Vec<ObjectId> = (0..3)
        .map(|i| g.create_object_from_definition(&creature(&format!("Foe {i}"), 1, 1), bob(), Zone::Battlefield))
        .collect();
    let foe_stables: Vec<_> = foes.iter().map(|id| g.object(*id).unwrap().stable_id).collect();
    let spell = g.create_object_from_definition(&spell_def, alice(), Zone::Hand);
    let mut dm = Recorder {
        prefer_objects: vec!["Big Fodder".into()],
        later_prefer_objects: vec!["Small Fodder".into()],
        prefer_targets: foes.iter().map(|id| Target::Object(*id)).collect(),
        distribute_each: 1,
        ..Default::default()
    };
    // Control: the same fixture with a fixed amount divides normally, so the
    // only variable is whether the later sacrifice choice is considered.
    {
        let fixed = card(
            "Audit Fixed Divided",
            "Mana cost: {0}\nType: Instant\nAs an additional cost to cast this spell, sacrifice a creature.\nThis spell deals 3 damage divided as you choose among any number of targets.",
        );
        let mut g2 = main_game();
        let mut q2 = TriggerQueue::new();
        g2.create_object_from_definition(&creature("Big Fodder", 3, 3), alice(), Zone::Battlefield);
        let foes2: Vec<ObjectId> = (0..3)
            .map(|i| g2.create_object_from_definition(&creature(&format!("Foe {i}"), 1, 1), bob(), Zone::Battlefield))
            .collect();
        let s2 = g2.create_object_from_definition(&fixed, alice(), Zone::Hand);
        let mut d2 = Recorder {
            prefer_objects: vec!["Big Fodder".into()],
            prefer_targets: foes2.iter().map(|id| Target::Object(*id)).collect(),
            distribute_each: 1,
            ..Default::default()
        };
        let a2 = cast_action(&g2, s2).expect("control: castable");
        act_with(&mut g2, &mut q2, a2, &mut d2).expect("control: fixed-amount division casts");
        settle_with(&mut g2, &mut q2, &mut d2);
        assert_eq!(g2.player(bob()).unwrap().graveyard.len(), 3, "control: fixed division kills all three");
    }
    let action = cast_action(&g, spell);
    assert!(action.is_some(), "CR 601.5: the spell is castable (sacrificing the 3/3 funds the division)");
    let result = act_with(&mut g, &mut q, action.unwrap(), &mut dm);
    assert!(
        result.is_ok(),
        "CR 601.5: dividing the sacrificed creature's power (3) among three targets must be allowed by considering the later sacrifice choice; got {result:?}; log {:#?}",
        dm.log
    );
    settle_with(&mut g, &mut q, &mut dm);
    let gy = g.player(bob()).unwrap().graveyard.clone();
    assert!(
        foe_stables.iter().all(|s| find_stable(&g, &gy, *s).is_some()),
        "CR 601.5: each of the three targets is dealt 1 of the 3 divided damage; log {:#?}",
        dm.log
    );
    assert!(find_stable(&g, &g.player(alice()).unwrap().graveyard, big_stable).is_some(),
        "the creature used to announce the division must actually pay the sacrifice cost");
    assert_eq!(g.object(small).unwrap().zone, Zone::Battlefield,
        "a later cost prompt cannot switch payment to the smaller creature");
}

// ---------------------------------------------------------------------------
// 701.20a: reveal durations
// ---------------------------------------------------------------------------

/// A card revealed as a cost remains revealed from announcement until the spell
/// leaves the stack. The engine exposes stack-duration reveals to the view
/// layer through the stack entry's public-revealed tag (and the hidden-zone
/// source snapshot for abilities activated from a hidden zone).
#[test]
fn cr_701_20a_card_revealed_as_cost_stays_revealed_while_spell_is_on_the_stack() {
    let spell_def = card(
        "Audit Dragon Reveal",
        "Mana cost: {0}\nType: Instant\nAs an additional cost to cast this spell, reveal a Dragon card from your hand.\nYou gain 1 life.",
    );
    let dragon_def = CardDefinitionBuilder::new(CardId::new(), "Hand Dragon")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![ironsmith::Subtype::Dragon])
        .power_toughness(ironsmith::PowerToughness::fixed(4, 4))
        .build();
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    let dragon = g.create_object_from_definition(&dragon_def, alice(), Zone::Hand);
    let spell = g.create_object_from_definition(&spell_def, alice(), Zone::Hand);
    let mut dm = Recorder { prefer_objects: vec!["Hand Dragon".into()], ..Default::default() };
    let action = cast_action(&g, spell).expect("fixture: castable with a Dragon in hand");
    act_with(&mut g, &mut q, action, &mut dm).expect("fixture: cast");
    assert_eq!(g.stack.len(), 1, "fixture: spell on the stack");
    let entry = g.stack.last().unwrap();
    let tagged: Vec<ObjectId> = entry
        .tagged_objects
        .get(&ironsmith::TagKey::from(ironsmith::effects::PUBLIC_REVEALED_TAG))
        .map(|v| v.iter().map(|s| s.object_id).collect())
        .unwrap_or_default();
    assert_eq!(g.object(dragon).map(|o| o.zone), Some(Zone::Hand), "fixture: revealed card stays in hand");
    assert!(
        tagged.contains(&dragon),
        "CR 701.20a: the card revealed as a cost remains revealed while the spell is on the stack (stack entry public-revealed tag = {tagged:?}, tags = {:?})",
        entry.tagged_objects.keys().collect::<Vec<_>>()
    );
}

#[test]
fn cr_701_20a_card_activated_from_hand_stays_revealed_until_ability_leaves_the_stack() {
    let pride = card(
        "Pride of the Clouds",
        "Mana cost: {W}{U}\nType: Creature — Elemental Cat\nPower/Toughness: 1/1\nFlying\nThis creature gets +1/+1 for each other creature you control with flying.\nForecast — {2}{W}{U}, Reveal this card from your hand: Create a 1/1 white and blue Bird creature token with flying. (Activate only during your upkeep and only once each turn.)",
    );
    let mut g = main_game();
    g.turn.phase = Phase::Beginning;
    g.turn.step = Some(Step::Upkeep);
    let mut q = TriggerQueue::new();
    let source = g.create_object_from_definition(&pride, alice(), Zone::Hand);
    {
        let pool = &mut g.player_mut(alice()).unwrap().mana_pool;
        pool.white = 1;
        pool.blue = 1;
        pool.colorless = 2;
    }
    let action = compute_legal_actions(&g, alice())
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
        .expect("fixture: forecast activatable in upkeep");
    let mut dm = Recorder::default();
    act_with(&mut g, &mut q, action, &mut dm).expect("fixture: activate");
    assert_eq!(g.stack.len(), 1, "fixture: forecast ability on the stack");
    let entry = g.stack.last().unwrap();
    let snap = entry.source_snapshot.as_ref().map(|s| (s.object_id, s.zone));
    assert_eq!(
        snap,
        Some((source, Zone::Hand)),
        "CR 701.20a/602.2a: an ability activated from a hidden zone keeps its source identified as revealed (hidden-zone source snapshot) while it is on the stack"
    );
    settle_with(&mut g, &mut q, &mut dm);
    assert!(g.stack.is_empty());
}

/// Same restart scenario, but the active player also has a Clone entering
/// simultaneously (created after Bob's, so object order alone would ask Bob
/// first). APNAP: Alice's copy choice first; after Bob's copy choice creates a
/// war-or-peace choice for Alice, APNAP restarts and Alice chooses before Bob's
/// outstanding second-Clone choice.
#[test]
fn cr_101_4d_restart_with_active_player_clone_in_the_same_simultaneous_entry() {
    let archangel = card(
        "Archangel of Strife",
        "Mana cost: {5}{W}{W}\nType: Creature — Angel\nPower/Toughness: 6/6\nFlying\nAs Archangel of Strife enters, each player chooses war or peace.\nCreatures controlled by players who chose war get +3/+0.\nCreatures controlled by players who chose peace get +0/+3.",
    );
    let clone = card(
        "Clone",
        "Mana cost: {3}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 0/0\nYou may have this creature enter as a copy of any creature on the battlefield.",
    );
    let mass = card(
        "Audit Mass Return",
        "Mana cost: {0}\nType: Sorcery\nReturn all creature cards from all graveyards to the battlefield under their owners' control.",
    );
    let mut g = main_game();
    let mut q = TriggerQueue::new();
    g.create_object_from_definition(&archangel, alice(), Zone::Battlefield);
    g.create_object_from_definition(&clone, bob(), Zone::Graveyard);
    g.create_object_from_definition(&clone, bob(), Zone::Graveyard);
    g.create_object_from_definition(&clone, alice(), Zone::Graveyard);
    let spell = g.create_object_from_definition(&mass, alice(), Zone::Hand);
    let mut dm = Recorder {
        boolean: true,
        prefer_objects: vec!["Archangel of Strife".into()],
        prefer_option: Some("copy of Archangel".into()),
        ..Default::default()
    };
    let action = cast_action(&g, spell).expect("fixture: mass return castable");
    act_with(&mut g, &mut q, action, &mut dm).expect("fixture: cast");
    let cast_len = dm.log.len();
    settle_with(&mut g, &mut q, &mut dm);
    let log: Vec<(PlayerId, String)> = dm.log[cast_len..]
        .iter()
        .map(|e| {
            let wp = e.3.iter().any(|o| o == "war" || o == "peace");
            (e.1, if wp { "war/peace".to_string() } else { "copy".to_string() })
        })
        .collect();
    let returned = g.battlefield.len();
    assert_eq!(returned, 4, "fixture: Archangel plus three Clones on the battlefield; log {log:?}");
    let first_copy = |p: PlayerId| log.iter().position(|e| e.0 == p && e.1 == "copy");
    let alice_copy = first_copy(alice()).expect("fixture: Alice's Clone copy choice");
    let bob_copies: Vec<usize> = log.iter().enumerate().filter(|(_, e)| e.0 == bob() && e.1 == "copy").map(|(i, _)| i).collect();
    assert_eq!(bob_copies.len(), 2, "fixture: Bob makes two copy choices; log {log:?}");
    let alice_wp_after_bob = log
        .iter()
        .enumerate()
        .position(|(i, e)| i > bob_copies[0] && e.0 == alice() && e.1 == "war/peace");
    assert!(
        alice_copy < bob_copies[0],
        "CR 101.4/101.4d: the active player makes her simultaneous as-enters choice first; log {log:?}"
    );
    assert!(
        alice_wp_after_bob.is_some_and(|i| i < bob_copies[1]),
        "CR 101.4d: the war-or-peace choice Bob's copy choice creates for Alice restarts APNAP ahead of Bob's outstanding second-Clone choice; log {log:?}"
    );
}
