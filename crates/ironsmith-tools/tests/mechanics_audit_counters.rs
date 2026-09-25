//! Regression tests for the counters / ETB & dies keywords / keyword actions /
//! permanent-kinds mechanics audit (ledger_counters.md). Each module covers one
//! finding with the CR-observable behaviour the fix established.

use ironsmith::card::PowerToughness;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_state::{Phase, Step, Target};
use ironsmith::ids::CardId;
use ironsmith::object::CounterType;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::{Subtype, Supertype};
use ironsmith::{CardType, Effect, GameState, ObjectId, PlayerId, Zone};

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}

fn bob() -> PlayerId {
    PlayerId::from_index(1)
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Every face of a real card, compiled (and registered builtin dungeons).
fn faces(name: &str) -> Vec<CardDefinition> {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap_or_else(|err| panic!("load {name}: {err}"))
    .iter()
    .map(|payload| {
        ironsmith_tools::compile_definition_from_payload(payload)
            .unwrap_or_else(|err| panic!("compile {name}: {err:?}"))
    })
    .collect()
}

fn card(name: &str) -> CardDefinition {
    faces(name).remove(0)
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn plain(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

fn new_game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn put(game: &mut GameState, def: &CardDefinition, owner: PlayerId) -> ObjectId {
    game.create_object_from_definition(def, owner, Zone::Battlefield)
}

/// Put a card onto the battlefield from hand through the full entry path
/// (replacements, as-enters programs, ETB events).
fn enter(game: &mut GameState, def: &CardDefinition, owner: PlayerId, dm: &mut dyn DecisionMaker) -> ObjectId {
    let hand = game.create_object_from_definition(def, owner, Zone::Hand);
    game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, dm)
        .expect("enters")
        .new_id
}

fn source(game: &mut GameState, owner: PlayerId) -> ObjectId {
    put(game, &plain("Effect Source", CardType::Artifact), owner)
}

/// Execute one effect as `controller` and queue the events it reports.
fn run(game: &mut GameState, effect: Effect, controller: PlayerId, dm: &mut dyn DecisionMaker) {
    let src = source(game, controller);
    run_from(game, effect, src, controller, dm);
}

fn run_from(game: &mut GameState, effect: Effect, src: ObjectId, controller: PlayerId, dm: &mut dyn DecisionMaker) {
    game.refresh_continuous_state();
    let mut ctx = ironsmith::effects::EffectContext::new(src, controller, dm);
    let outcome = ironsmith::effects::execute_effect(game, &effect, &mut ctx).expect("effect");
    for event in outcome.events {
        let provenance = event.provenance();
        game.queue_trigger_event(provenance, event);
    }
}

/// Pending trigger events -> triggered abilities (not yet on the stack).
fn pending(game: &mut GameState) -> TriggerQueue {
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    queue
}

/// Drain triggers, apply SBAs and resolve the stack until everything settles.
fn settle(game: &mut GameState, dm: &mut dyn DecisionMaker) {
    for _ in 0..40 {
        let mut queue = pending(game);
        ironsmith::game_loop::check_and_apply_sbas_with(game, &mut queue, dm).unwrap();
        ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("game did not settle");
}

/// Triggers of the current step (upkeep, end step, ...), put on the stack.
fn step_triggers(game: &mut GameState, step: Step, dm: &mut dyn DecisionMaker) {
    game.turn.phase = match step {
        Step::Untap | Step::Upkeep | Step::Draw => Phase::Beginning,
        Step::End | Step::Cleanup => Phase::Ending,
        _ => Phase::Combat,
    };
    game.turn.step = Some(step);
    let mut queue = TriggerQueue::new();
    for event in ironsmith::triggers::generate_step_trigger_events_for_active_players(game) {
        for entry in ironsmith::triggers::check_triggers(game, &event) {
            queue.add(entry);
        }
        for entry in ironsmith::triggers::check_delayed_triggers(game, &event) {
            queue.add(entry);
        }
    }
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}

fn zone_of(game: &GameState, stable: ironsmith::ids::StableId) -> Option<Zone> {
    game.find_object_by_stable_id(stable)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

fn stable(game: &GameState, id: ObjectId) -> ironsmith::ids::StableId {
    game.object(id).unwrap().stable_id
}

fn on_battlefield(game: &GameState, id: ObjectId) -> bool {
    game.battlefield.contains(&id)
}

fn named_on_battlefield(game: &GameState, name: &str) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).is_some_and(|object| object.name.as_str() == name))
        .collect()
}

/// Scriptable decision maker: `yes` answers booleans; objects and targets
/// prefer the listed names (else the minimum legal / first legal);
/// `take_max` selects as many objects as allowed; `option` picks an option.
#[derive(Default)]
struct Dm {
    yes: bool,
    prefer: Vec<&'static str>,
    take_max: bool,
    option: usize,
    target_player: Option<PlayerId>,
}

impl Dm {
    fn yes() -> Self {
        Self { yes: true, ..Self::default() }
    }
    fn prefer(mut self, names: &[&'static str]) -> Self {
        self.prefer = names.to_vec();
        self
    }
}

impl DecisionMaker for Dm {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.yes
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal = ctx.candidates.iter().filter(|c| c.legal).collect::<Vec<_>>();
        let max = ctx.max.unwrap_or(legal.len()).min(legal.len());
        let preferred = legal
            .iter()
            .filter(|c| self.prefer.iter().any(|name| c.name == *name))
            .map(|c| c.id)
            .take(max)
            .collect::<Vec<_>>();
        if !preferred.is_empty() && preferred.len() >= ctx.min {
            return preferred;
        }
        let count = if self.take_max { max } else { ctx.min.max(usize::from(max > 0 && ctx.min > 0)) };
        legal.iter().map(|c| c.id).take(count.max(ctx.min)).collect()
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.min == 0 && !self.yes {
            return Vec::new();
        }
        let legal = ctx.options.iter().filter(|o| o.legal).collect::<Vec<_>>();
        if let Some(confirm) = legal.iter().find(|o| o.description.starts_with("Confirm")) {
            return vec![confirm.index];
        }
        let pick = legal
            .iter()
            .find(|o| o.index == self.option)
            .or_else(|| legal.first())
            .map(|o| o.index);
        pick.into_iter().collect()
    }

    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut chosen = Vec::new();
        for requirement in &ctx.requirements {
            let preferred = requirement.legal_targets.iter().find(|target| match target {
                Target::Object(id) => game
                    .object(*id)
                    .is_some_and(|object| self.prefer.iter().any(|name| object.name.as_str() == *name)),
                Target::Player(player) => self.target_player == Some(*player),
            });
            let wanted = requirement.min_targets.max(1).min(requirement.legal_targets.len());
            if let Some(target) = preferred {
                chosen.push(*target);
            } else {
                chosen.extend(requirement.legal_targets.iter().take(wanted).copied());
            }
        }
        chosen
    }
}

// ---------------------------------------------------------------------------
// D1: upkeep, counters, ETB and dies keywords
// ---------------------------------------------------------------------------

mod modular {
    use super::*;

    /// D1-01 / CR 702.43a: modular counts the +1/+1 counters the creature had
    /// on the battlefield (LKI), not the counter-less graveyard card.
    #[test]
    fn dying_modular_creature_moves_its_last_known_counters() {
        let mut game = new_game();
        let worker = card("Arcbound Worker");
        let dying = put(&mut game, &worker, alice());
        let receiver = put(&mut game, &creature("Artifact Receiver", 1, 1), alice());
        game.object_mut(receiver).unwrap().card_types.push(CardType::Artifact);
        game.object_mut(dying).unwrap().add_counters(CounterType::PlusOnePlusOne, 3);
        let mut dm = Dm::yes().prefer(&["Artifact Receiver"]);
        run(&mut game, Effect::destroy(ChooseSpec::SpecificObject(dying)), bob(), &mut dm);
        settle(&mut game, &mut dm);
        assert!(!on_battlefield(&game, dying));
        assert_eq!(game.counter_count(receiver, CounterType::PlusOnePlusOne), 3);
    }
}

mod exploit {
    use super::*;

    /// D1-02 / CR 702.110b: a creature that exploits itself still gets its
    /// "when this exploits a creature" trigger (look-back).
    #[test]
    fn sidisis_faithful_exploiting_itself_still_bounces() {
        let mut game = new_game();
        let bear = put(&mut game, &creature("Bob Bear", 2, 2), bob());
        let mut dm = Dm::yes().prefer(&["Sidisi's Faithful", "Bob Bear"]);
        let faithful = enter(&mut game, &card("Sidisi's Faithful"), alice(), &mut dm);
        settle(&mut game, &mut dm);
        assert!(!on_battlefield(&game, faithful), "exploited itself");
        assert!(game.player(bob()).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name.as_str() == "Bob Bear"));
        let _ = bear;
    }
}

mod unleash {
    use super::*;

    /// D1-03 / CR 702.98a, 614.1c: unleash is an entry replacement — the
    /// creature enters with the counter and nothing goes on the stack.
    #[test]
    fn rakdos_cackler_enters_with_the_counter_without_a_trigger() {
        let mut game = new_game();
        let mut dm = Dm::yes();
        let cackler = enter(&mut game, &card("Rakdos Cackler"), alice(), &mut dm);
        assert_eq!(game.counter_count(cackler, CounterType::PlusOnePlusOne), 1);
        let queue = pending(&mut game);
        assert!(queue.entries.is_empty(), "no unleash trigger: {:?}", queue.entries.len());
    }

    #[test]
    fn declining_unleash_enters_without_a_counter() {
        let mut game = new_game();
        let mut dm = Dm::default();
        let cackler = enter(&mut game, &card("Rakdos Cackler"), alice(), &mut dm);
        assert_eq!(game.counter_count(cackler, CounterType::PlusOnePlusOne), 0);
    }
}

mod devour_amplify {
    use super::*;

    /// D1-04 / CR 702.82a: devour sacrifices as the creature enters; it
    /// enters with the counters, with no trigger to respond to.
    #[test]
    fn mycoloth_devours_as_it_enters() {
        let mut game = new_game();
        let a = put(&mut game, &creature("Fodder A", 1, 1), alice());
        let b = put(&mut game, &creature("Fodder B", 1, 1), alice());
        let mut dm = Dm { yes: true, take_max: true, ..Dm::default() };
        let mycoloth = enter(&mut game, &card("Mycoloth"), alice(), &mut dm);
        assert_eq!(game.counter_count(mycoloth, CounterType::PlusOnePlusOne), 4);
        assert!(!on_battlefield(&game, a) && !on_battlefield(&game, b));
        assert!(pending(&mut game).entries.is_empty(), "devour is not a trigger");
    }

    /// CR 702.38a: amplify reveals as the creature enters.
    #[test]
    fn glowering_rogon_amplifies_as_it_enters() {
        let mut game = new_game();
        let beast = CardDefinitionBuilder::new(CardId::new(), "Hand Beast")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Beast])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_definition(&beast, alice(), Zone::Hand);
        let mut dm = Dm { yes: true, take_max: true, ..Dm::default() };
        let rogon = enter(&mut game, &card("Glowering Rogon"), alice(), &mut dm);
        assert_eq!(game.counter_count(rogon, CounterType::PlusOnePlusOne), 1);
        assert!(pending(&mut game).entries.is_empty(), "amplify is not a trigger");
    }
}

mod fading {
    use super::*;

    fn upkeep(game: &mut GameState) {
        let mut dm = Dm::yes();
        step_triggers(game, Step::Upkeep, &mut dm);
        settle(game, &mut dm);
    }

    /// D1-05 / CR 702.32a: removing the last fade counter doesn't sacrifice
    /// it; the next upkeep's failed removal does.
    #[test]
    fn blastoderm_survives_the_upkeep_that_removes_its_last_counter() {
        let mut game = new_game();
        let blastoderm = put(&mut game, &card("Blastoderm"), alice());
        game.object_mut(blastoderm).unwrap().add_counters(CounterType::Fade, 1);
        upkeep(&mut game);
        assert!(on_battlefield(&game, blastoderm));
        assert_eq!(game.counter_count(blastoderm, CounterType::Fade), 0);
        upkeep(&mut game);
        assert!(!on_battlefield(&game, blastoderm), "can't remove a fade counter: sacrificed");
    }

    /// Removing a different counter from a fading permanent doesn't sacrifice it.
    #[test]
    fn removing_other_counters_does_not_sacrifice() {
        let mut game = new_game();
        let blastoderm = put(&mut game, &card("Blastoderm"), alice());
        game.object_mut(blastoderm).unwrap().add_counters(CounterType::PlusOnePlusOne, 1);
        let mut dm = Dm::yes();
        run(
            &mut game,
            Effect::remove_counters(CounterType::PlusOnePlusOne, 1, ChooseSpec::SpecificObject(blastoderm)),
            alice(),
            &mut dm,
        );
        settle(&mut game, &mut dm);
        assert!(on_battlefield(&game, blastoderm));
    }
}

mod boast {
    use super::*;

    fn boast_actions(game: &GameState, arni: ObjectId) -> usize {
        ironsmith::decision::compute_legal_actions(game, alice())
            .into_iter()
            .filter(|action| {
                matches!(action, ironsmith::decision::LegalAction::ActivateAbility { source, .. } if *source == arni)
            })
            .count()
    }

    /// D1-06 / CR 702.142a: boast can be activated only if the creature
    /// attacked this turn.
    #[test]
    fn axgard_braggart_can_boast_only_after_attacking() {
        let mut game = new_game();
        let arni = put(&mut game, &card("Axgard Braggart"), alice());
        game.player_mut(alice()).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::White, 3);
        game.refresh_continuous_state();
        assert_eq!(boast_actions(&game, arni), 0, "hasn't attacked");
        // Declared as an attacker this turn (what attack declaration records).
        game.turn_store.turn_history.creatures_attacked_this_turn.insert(arni);
        let _ = pending(&mut game);
        game.refresh_continuous_state();
        assert_eq!(boast_actions(&game, arni), 1, "attacked this turn");
    }
}

mod evolve {
    use super::*;

    /// D1-07 / CR 702.100a: the comparison is an intervening-if — a smaller
    /// creature entering doesn't trigger evolve at all.
    #[test]
    fn smaller_creature_does_not_trigger_evolve() {
        let mut game = new_game();
        let raptor = put(&mut game, &card("Cloudfin Raptor"), alice());
        let _ = pending(&mut game);
        let mut dm = Dm::yes();
        enter(&mut game, &creature("Tiny", 0, 1), alice(), &mut dm);
        let queue = pending(&mut game);
        assert!(queue.entries.iter().all(|entry| entry.source != raptor), "no evolve trigger");
    }

    /// The entering creature's last-known P/T is used if it's gone by
    /// resolution.
    #[test]
    fn evolve_uses_lki_of_a_creature_that_left() {
        let mut game = new_game();
        let one = put(&mut game, &card("Experiment One"), alice());
        let _ = pending(&mut game);
        let mut dm = Dm::yes();
        let big = enter(&mut game, &creature("Big", 3, 3), alice(), &mut dm);
        let mut queue = pending(&mut game);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "evolve triggered");
        game.move_object_by_effect(big, Zone::Graveyard);
        settle(&mut game, &mut dm);
        assert_eq!(game.counter_count(one, CounterType::PlusOnePlusOne), 1);
    }
}

mod echo {
    use super::*;

    fn echo_triggers(game: &mut GameState, patrol: ObjectId) -> usize {
        let mut dm = Dm::default();
        ironsmith::turn::execute_untap_step_with(game, &mut dm);
        step_triggers(game, Step::Upkeep, &mut dm);
        let count = game.stack.iter().filter(|entry| entry.object_id == patrol).count();
        game.stack.clear();
        count
    }

    /// D1-08 / CR 702.30a: echo triggers at the first upkeep after it came
    /// under your control, not later — and counters can't change that.
    #[test]
    fn echo_triggers_only_on_the_first_upkeep() {
        let mut game = new_game();
        let patrol = put(&mut game, &card("Goblin Patrol"), alice());
        game.set_summoning_sick(patrol);
        assert_eq!(echo_triggers(&mut game, patrol), 1, "came under control since last upkeep");
        assert_eq!(game.counter_count(patrol, CounterType::Echo), 0, "no echo counter");
        assert_eq!(echo_triggers(&mut game, patrol), 0, "already controlled since last upkeep");
    }

    /// Gaining control of it makes echo apply again for the new controller.
    #[test]
    fn stolen_echo_permanent_triggers_for_its_new_controller() {
        let mut game = new_game();
        let patrol = put(&mut game, &card("Goblin Patrol"), bob());
        game.set_current_controller(patrol, alice());
        game.refresh_continuous_state();
        game.set_summoning_sick(patrol);
        assert_eq!(echo_triggers(&mut game, patrol), 1);
    }
}

mod haunt {
    use super::*;

    /// D1-10 / CR 702.55a/c: the haunt card is exiled haunting a creature,
    /// and its ability triggers when that creature dies.
    #[test]
    fn blind_hunter_is_exiled_haunting_and_triggers_when_the_creature_dies() {
        let mut game = new_game();
        let hunter = put(&mut game, &card("Blind Hunter"), alice());
        let hunter_card = stable(&game, hunter);
        let bear = put(&mut game, &creature("Haunted Bear", 2, 2), bob());
        let mut dm = Dm { yes: true, target_player: Some(bob()), ..Dm::default() }.prefer(&["Haunted Bear"]);
        run(&mut game, Effect::destroy(ChooseSpec::SpecificObject(hunter)), bob(), &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(zone_of(&game, hunter_card), Some(Zone::Exile), "exiled haunting the Bear");
        run(&mut game, Effect::destroy(ChooseSpec::SpecificObject(bear)), alice(), &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(bob()).unwrap().life, 18);
        assert_eq!(game.player(alice()).unwrap().life, 22);
    }
}

mod level_up {
    use super::*;

    /// D1-11 / CR 711.2b, 613.7: a level symbol's base P/T is a 7b effect
    /// with the leveler's timestamp, so it beats an earlier 7b effect.
    #[test]
    fn student_of_warfare_level_beats_earlier_godhead_of_awe() {
        let mut game = new_game();
        let mut dm = Dm::default();
        enter(&mut game, &card("Godhead of Awe"), bob(), &mut dm);
        let student = enter(&mut game, &card("Student of Warfare"), alice(), &mut dm);
        game.object_mut(student).unwrap().add_counters(CounterType::Level, 7);
        game.refresh_continuous_state();
        assert_eq!(game.calculated_power(student), Some(4));
        assert_eq!(game.calculated_toughness(student), Some(4));
    }
}

mod champion {
    use super::*;

    /// D1-12 / CR 702.72a: the championed card returns through a
    /// leaves-the-battlefield trigger.
    #[test]
    fn championed_creature_returns_when_the_champion_leaves() {
        let mut game = new_game();
        let bear = put(&mut game, &creature("Championed Bear", 2, 2), alice());
        let bear_card = stable(&game, bear);
        let mut dm = Dm::yes().prefer(&["Championed Bear"]);
        let hero = enter(&mut game, &card("Changeling Hero"), alice(), &mut dm);
        settle(&mut game, &mut dm);
        assert!(on_battlefield(&game, hero));
        assert_eq!(zone_of(&game, bear_card), Some(Zone::Exile));
        run(&mut game, Effect::destroy(ChooseSpec::SpecificObject(hero)), bob(), &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(zone_of(&game, bear_card), Some(Zone::Battlefield));
    }

    /// If the champion left before its ETB trigger resolved, the card exiled
    /// by that trigger never returns (the LTB trigger already resolved).
    #[test]
    fn card_exiled_after_the_champion_left_stays_exiled() {
        let mut game = new_game();
        let bear = put(&mut game, &creature("Championed Bear", 2, 2), alice());
        let bear_card = stable(&game, bear);
        let mut dm = Dm::yes().prefer(&["Championed Bear"]);
        let hero = enter(&mut game, &card("Changeling Hero"), alice(), &mut dm);
        let mut queue = pending(&mut game);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        game.move_object_by_effect(hero, Zone::Graveyard);
        settle(&mut game, &mut dm);
        assert_eq!(zone_of(&game, bear_card), Some(Zone::Exile));
    }
}

mod graft_soulbond {
    use super::*;

    /// D1-13 / CR 702.58a: graft's intervening-if — no +1/+1 counter, no trigger.
    #[test]
    fn graft_without_counters_does_not_trigger() {
        let mut game = new_game();
        let initiate = put(&mut game, &card("Simic Initiate"), alice());
        let _ = pending(&mut game);
        let mut dm = Dm::yes();
        enter(&mut game, &creature("Newcomer", 1, 1), alice(), &mut dm);
        assert!(pending(&mut game).entries.iter().all(|entry| entry.source != initiate));

        game.object_mut(initiate).unwrap().add_counters(CounterType::PlusOnePlusOne, 1);
        enter(&mut game, &creature("Second Newcomer", 1, 1), alice(), &mut dm);
        assert!(pending(&mut game).entries.iter().any(|entry| entry.source == initiate));
    }

    /// CR 702.95a: a paired soulbond creature doesn't trigger again.
    #[test]
    fn paired_soulbond_creature_does_not_trigger() {
        let mut game = new_game();
        let wingcrafter = put(&mut game, &card("Wingcrafter"), alice());
        let partner = put(&mut game, &creature("Partner", 2, 2), alice());
        game.set_soulbond_pair(wingcrafter, partner);
        let _ = pending(&mut game);
        let mut dm = Dm::yes();
        enter(&mut game, &creature("Newcomer", 1, 1), alice(), &mut dm);
        assert!(pending(&mut game).entries.iter().all(|entry| entry.source != wingcrafter));
    }
}

// ---------------------------------------------------------------------------
// E1: keyword actions and counters (CR 122, 701 part 1)
// ---------------------------------------------------------------------------

mod stun {
    use super::*;

    /// E1-01 / CR 122.1d: a stun counter replaces untapping.
    #[test]
    fn stunned_creature_stays_tapped_and_loses_a_counter() {
        let mut game = new_game();
        let bear = put(&mut game, &creature("Stunned Bear", 2, 2), alice());
        game.tap(bear);
        game.object_mut(bear).unwrap().add_counters(CounterType::Stun, 2);
        let mut dm = Dm::default();
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut dm);
        assert!(game.is_tapped(bear));
        assert_eq!(game.counter_count(bear, CounterType::Stun), 1);
    }

    /// An untap effect is replaced the same way.
    #[test]
    fn untap_effect_removes_a_stun_counter_instead() {
        let mut game = new_game();
        let bear = put(&mut game, &creature("Stunned Bear", 2, 2), alice());
        game.tap(bear);
        game.object_mut(bear).unwrap().add_counters(CounterType::Stun, 1);
        let mut dm = Dm::default();
        run(&mut game, Effect::untap(ChooseSpec::SpecificObject(bear)), alice(), &mut dm);
        assert!(game.is_tapped(bear));
        assert_eq!(game.counter_count(bear, CounterType::Stun), 0);
        run(&mut game, Effect::untap(ChooseSpec::SpecificObject(bear)), alice(), &mut dm);
        assert!(!game.is_tapped(bear), "no stun counter left");
    }
}

mod shield {
    use super::*;

    /// E1-02 / CR 122.1c: a shield counter replaces destruction by an effect.
    #[test]
    fn shield_counter_stops_destruction() {
        let mut game = new_game();
        let bear = put(&mut game, &creature("Shielded Bear", 2, 2), alice());
        game.object_mut(bear).unwrap().add_counters(CounterType::Shield, 1);
        let mut dm = Dm::default();
        run(&mut game, Effect::destroy(ChooseSpec::SpecificObject(bear)), bob(), &mut dm);
        settle(&mut game, &mut dm);
        assert!(on_battlefield(&game, bear));
        assert_eq!(game.counter_count(bear, CounterType::Shield), 0);
        run(&mut game, Effect::destroy(ChooseSpec::SpecificObject(bear)), bob(), &mut dm);
        settle(&mut game, &mut dm);
        assert!(!on_battlefield(&game, bear), "no shield left");
    }

    /// It also prevents damage.
    #[test]
    fn shield_counter_prevents_damage() {
        let mut game = new_game();
        let bear = put(&mut game, &creature("Shielded Bear", 2, 2), alice());
        game.object_mut(bear).unwrap().add_counters(CounterType::Shield, 1);
        let mut dm = Dm::default();
        run(&mut game, Effect::deal_damage(3, ChooseSpec::SpecificObject(bear)), bob(), &mut dm);
        settle(&mut game, &mut dm);
        assert!(on_battlefield(&game, bear));
        assert_eq!(game.damage_on(bear), 0);
        assert_eq!(game.counter_count(bear, CounterType::Shield), 0);
    }
}

mod regenerate {
    use super::*;

    /// E1-04 / CR 701.19a: noncreature permanents can be regenerated.
    #[test]
    fn regenerated_noncreature_artifact_survives_destruction() {
        let mut game = new_game();
        let plating = put(&mut game, &plain("Artifact To Save", CardType::Artifact), alice());
        let mut dm = Dm::default();
        run(
            &mut game,
            Effect::regenerate(ChooseSpec::SpecificObject(plating), ironsmith::effect::Until::EndOfTurn),
            alice(),
            &mut dm,
        );
        run(&mut game, Effect::destroy(ChooseSpec::SpecificObject(plating)), bob(), &mut dm);
        settle(&mut game, &mut dm);
        assert!(on_battlefield(&game, plating));
        assert!(game.is_tapped(plating));
    }
}

mod discard_choice {
    use super::*;

    /// E1-05 / CR 701.9b: after Recoil bounces a permanent, its owner chooses
    /// the card to discard.
    #[test]
    fn recoil_lets_the_player_choose_the_discard() {
        let mut game = new_game();
        let memnite = put(&mut game, &plain("Bounced Artifact", CardType::Artifact), bob());
        game.create_object_from_definition(&plain("Keep Me", CardType::Sorcery), bob(), Zone::Hand);
        game.create_object_from_definition(&plain("Discard Me", CardType::Sorcery), bob(), Zone::Hand);
        let recoil_def = card("Recoil");
        let recoil = game.create_object_from_definition(&recoil_def, alice(), Zone::Stack);
        game.push_to_stack(
            ironsmith::game_state::StackEntry::new(recoil, alice()).with_targets(vec![Target::Object(memnite)]),
        );
        let mut dm = Dm::yes().prefer(&["Discard Me"]);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let graveyard = game.player(bob()).unwrap().graveyard.iter().map(|id| game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>();
        assert_eq!(graveyard, vec!["Discard Me".to_string()]);
    }
}

mod sacrifice_control {
    use super::*;

    fn breach(steal: bool) -> bool {
        let mut game = new_game();
        let def = creature("Breached Giant", 5, 5);
        game.create_object_from_definition(&def, alice(), Zone::Hand);
        let spell = game.create_object_from_definition(&card("Through the Breach"), alice(), Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice()));
        let mut dm = Dm::yes().prefer(&["Breached Giant"]);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let giant = named_on_battlefield(&game, "Breached Giant");
        assert_eq!(giant.len(), 1, "put onto the battlefield");
        let giant = giant[0];
        let _ = pending(&mut game);
        if steal {
            game.set_current_controller(giant, bob());
            game.refresh_continuous_state();
        }
        step_triggers(&mut game, Step::End, &mut dm);
        settle(&mut game, &mut dm);
        on_battlefield(&game, giant)
    }

    /// E1-07 / CR 701.21a: "sacrifice that creature" does nothing once
    /// another player controls it.
    #[test]
    fn delayed_sacrifice_skips_a_stolen_creature() {
        assert!(!breach(false), "sacrificed at the end step");
        assert!(breach(true), "Alice can't sacrifice Bob's creature");
    }
}

mod scry_surveil {
    use super::*;

    /// E1-09 / CR 701.25d: surveilling with an empty library still surveils.
    #[test]
    fn surveil_with_empty_library_triggers_dimir_spybug() {
        let mut game = new_game();
        let spybug = put(&mut game, &card("Dimir Spybug"), alice());
        let mut dm = Dm::default();
        run(&mut game, Effect::surveil(2), alice(), &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(game.counter_count(spybug, CounterType::PlusOnePlusOne), 1);
    }

    /// CR 701.22d: the same for scry.
    #[test]
    fn scry_with_empty_library_triggers_chance_met_elves() {
        let mut game = new_game();
        let elves = put(&mut game, &card("Chance-Met Elves"), alice());
        let mut dm = Dm::default();
        run(&mut game, Effect::scry(1), alice(), &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(game.counter_count(elves, CounterType::PlusOnePlusOne), 1);
    }
}

mod cant_be_regenerated {
    use super::*;

    /// E1-10 / CR 701.19c: "can't be regenerated" disables regeneration only;
    /// the destroy-without-regeneration still destroys a shielded creature.
    #[test]
    fn destroy_without_regeneration_ignores_shield() {
        let mut game = new_game();
        let bear = put(&mut game, &creature("Regen Bear", 2, 2), alice());
        let mut dm = Dm::default();
        run(
            &mut game,
            Effect::regenerate(ChooseSpec::SpecificObject(bear), ironsmith::effect::Until::EndOfTurn),
            alice(),
            &mut dm,
        );
        let wrath = card("Wrath of God");
        let spell = game.create_object_from_definition(&wrath, bob(), Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, bob()));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        settle(&mut game, &mut dm);
        assert!(!on_battlefield(&game, bear));
    }
}

// ---------------------------------------------------------------------------
// E2: keyword actions (CR 701 part 2) and designations
// ---------------------------------------------------------------------------

mod explore_connive {
    use super::*;

    /// E2-3 / CR 614.1: explore's counter goes through counter replacements.
    #[test]
    fn explore_counter_is_increased_by_hardened_scales() {
        let mut game = new_game();
        put(&mut game, &card("Hardened Scales"), alice());
        let bear = put(&mut game, &creature("Explorer", 2, 2), alice());
        game.create_object_from_definition(&plain("Nonland Top", CardType::Sorcery), alice(), Zone::Library);
        let mut dm = Dm::default();
        run(&mut game, Effect::explore(ChooseSpec::SpecificObject(bear)), alice(), &mut dm);
        assert_eq!(game.counter_count(bear, CounterType::PlusOnePlusOne), 2);
    }

    #[test]
    fn connive_counter_is_increased_by_hardened_scales() {
        let mut game = new_game();
        put(&mut game, &card("Hardened Scales"), alice());
        let bear = put(&mut game, &creature("Conniver", 2, 2), alice());
        game.create_object_from_definition(&plain("Drawn Nonland", CardType::Sorcery), alice(), Zone::Library);
        let mut dm = Dm::default();
        run(&mut game, Effect::connive(ChooseSpec::SpecificObject(bear)), alice(), &mut dm);
        assert_eq!(game.counter_count(bear, CounterType::PlusOnePlusOne), 2);
    }
}

mod ring_bearer {
    use super::*;

    fn legendary(game: &GameState, id: ObjectId) -> bool {
        game.current_characteristics(id)
            .is_some_and(|chars| chars.supertypes.contains(&Supertype::Legendary))
    }

    /// E2-4 / CR 701.54b-c: the Ring-bearer is legendary, but that isn't
    /// copiable, and it stops once another player controls it.
    #[test]
    fn ring_bearer_legendary_is_not_copied_and_ends_on_control_change() {
        let mut game = new_game();
        let bearer = put(&mut game, &creature("Bearer", 2, 2), alice());
        game.set_ring_bearer(alice(), bearer);
        game.refresh_continuous_state();
        assert!(legendary(&game, bearer));
        let mut dm = Dm::default();
        run(&mut game, Effect::create_token_copy(ChooseSpec::SpecificObject(bearer)), alice(), &mut dm);
        let copies = named_on_battlefield(&game, "Bearer");
        let copy = copies.into_iter().find(|id| *id != bearer).expect("token copy");
        game.refresh_continuous_state();
        assert!(!legendary(&game, copy), "copy isn't legendary");
        assert!(on_battlefield(&game, bearer) && on_battlefield(&game, copy), "no legend rule");
        game.set_current_controller(bearer, bob());
        game.refresh_continuous_state();
        assert!(!legendary(&game, bearer), "no longer Alice's Ring-bearer");
    }
}

mod incubate {
    use super::*;

    /// E2-5 / CR 701.53a, 614.1: incubate creates tokens, so token doublers apply.
    #[test]
    fn parallel_lives_doubles_incubators() {
        let mut game = new_game();
        put(&mut game, &card("Parallel Lives"), alice());
        let mut dm = Dm::default();
        run(&mut game, Effect::incubate(3, 1), alice(), &mut dm);
        let incubators = named_on_battlefield(&game, "Incubator");
        assert_eq!(incubators.len(), 2);
        for incubator in incubators {
            assert_eq!(game.counter_count(incubator, CounterType::PlusOnePlusOne), 3);
        }
    }
}

mod amass {
    use super::*;

    /// E2-8 / CR 701.47a: the amassed subtype is a type-changing effect, not
    /// a copiable value.
    #[test]
    fn amassed_subtype_is_not_copied() {
        let mut game = new_game();
        let army_def = CardDefinitionBuilder::new(CardId::new(), "Zombie Army")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie, Subtype::Army])
            .power_toughness(PowerToughness::fixed(0, 0))
            .build();
        let army = put(&mut game, &army_def, alice());
        game.object_mut(army).unwrap().add_counters(CounterType::PlusOnePlusOne, 1);
        let mut dm = Dm::default();
        run(&mut game, Effect::amass(Some(Subtype::Orc), 1), alice(), &mut dm);
        game.refresh_continuous_state();
        assert!(game.calculated_subtypes(army).contains(&Subtype::Orc));
        run(&mut game, Effect::create_token_copy(ChooseSpec::SpecificObject(army)), alice(), &mut dm);
        let copy = named_on_battlefield(&game, "Zombie Army").into_iter().find(|id| *id != army).expect("copy");
        game.refresh_continuous_state();
        assert!(!game.calculated_subtypes(copy).contains(&Subtype::Orc));
    }
}

mod endure {
    use super::*;

    fn kin_guard(leave: bool) -> (u32, usize) {
        let mut game = new_game();
        let mut dm = Dm::default();
        let guard = enter(&mut game, &card("Fortress Kin-Guard"), alice(), &mut dm);
        let mut queue = pending(&mut game);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        if leave {
            game.move_object_by_effect(guard, Zone::Graveyard);
        }
        settle(&mut game, &mut dm);
        (game.counter_count(guard, CounterType::PlusOnePlusOne), named_on_battlefield(&game, "Spirit").len())
    }

    /// E2-9 / CR 701.63a: if the permanent is gone, counters can't be put on
    /// it, so the Spirit is created even when the counter mode is chosen.
    #[test]
    fn endure_creates_the_spirit_when_the_permanent_left() {
        assert_eq!(kin_guard(false), (1, 0), "counter mode while it's there");
        assert_eq!(kin_guard(true).1, 1, "Spirit when it's gone");
    }
}

mod dungeons {
    use super::*;

    fn venture(game: &mut GameState, dm: &mut Dm) {
        run(game, Effect::venture_into_dungeon_player(PlayerFilter::You), alice(), dm);
    }

    /// E2-1 / CR 309.4c: moving the venture marker into a room triggers that
    /// room's ability (compiled from the dungeon's printed text).
    #[test]
    fn entering_a_room_triggers_its_room_ability() {
        let _ = card("Grizzly Bears"); // registers builtin dungeons
        let mut game = new_game();
        let mut dm = Dm::yes();
        dm.option = usize::MAX; // first legal: dungeons are listed by name
        venture(&mut game, &mut dm);
        let progress = game.active_dungeon(alice()).cloned().expect("in a dungeon");
        assert_eq!(progress.dungeon_name, "Dungeon of the Mad Mage");
        assert_eq!(progress.room_name, "Yawning Portal");
        settle(&mut game, &mut dm);
        assert_eq!(game.player(alice()).unwrap().life, 21, "Yawning Portal: gain 1 life");
    }

    /// CR 704.5t: the dungeon is completed only after the bottommost room's
    /// ability has left the stack.
    #[test]
    fn dungeon_completes_after_the_last_room_ability_resolves() {
        let _ = card("Grizzly Bears");
        let mut game = new_game();
        game.set_active_dungeon(
            alice(),
            ironsmith::dungeon::ActiveDungeonProgress::new("Tomb of Annihilation", "Sandfall Cell"),
        );
        let mut dm = Dm::yes();
        venture(&mut game, &mut dm);
        let mut queue = pending(&mut game);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "Cradle of the Death God ability");
        let mut empty = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut empty, &mut dm).unwrap();
        assert!(!game.has_completed_named_dungeon(alice(), "Tomb of Annihilation"), "room ability still on the stack");
        settle(&mut game, &mut dm);
        assert!(game.has_completed_named_dungeon(alice(), "Tomb of Annihilation"));
        assert_eq!(named_on_battlefield(&game, "The Atropal").len(), 1);
    }

    /// CR 725.2: taking the initiative ventures into Undercity; Secret
    /// Entrance searches for a basic land.
    #[test]
    fn initiative_enters_undercity_secret_entrance() {
        let _ = card("Grizzly Bears");
        let mut game = new_game();
        let forest = card("Forest");
        game.create_object_from_definition(&forest, alice(), Zone::Library);
        let mut dm = Dm::yes().prefer(&["Forest"]);
        run(&mut game, Effect::take_initiative_player(PlayerFilter::You), alice(), &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(game.active_dungeon(alice()).map(|p| p.dungeon_name.clone()).as_deref(), Some("Undercity"));
        assert!(game.player(alice()).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name.as_str() == "Forest"));
    }
}

// ---------------------------------------------------------------------------
// D2: face-down, bestow, DFCs, day/night, sagas, rooms, classes
// ---------------------------------------------------------------------------

mod face_down {
    use super::*;

    /// D2-01 / CR 708.9, 400.7: a face-down permanent that leaves is its real card.
    #[test]
    fn face_down_creature_dies_as_its_real_card() {
        let mut game = new_game();
        let bears = put(&mut game, &card("Grizzly Bears"), alice());
        let bears_card = stable(&game, bears);
        game.set_face_down(bears);
        assert_eq!(game.object(bears).unwrap().name.as_str(), "Face-down creature");
        game.move_object_by_effect(bears, Zone::Graveyard);
        let id = game.find_object_by_stable_id(bears_card).unwrap();
        let object = game.object(id).unwrap();
        assert_eq!(object.name.as_str(), "Grizzly Bears");
        assert!(object.mana_cost.is_some());
    }

    /// D2-11 / CR 201.2a: face-down permanents have no name, so they don't
    /// share a name with each other.
    #[test]
    fn echoing_truth_on_a_face_down_creature_returns_only_it() {
        let mut game = new_game();
        let a = put(&mut game, &card("Grizzly Bears"), bob());
        let b = put(&mut game, &card("Llanowar Elves"), bob());
        game.set_face_down(a);
        game.set_face_down(b);
        game.refresh_continuous_state();
        let truth = game.create_object_from_definition(&card("Echoing Truth"), alice(), Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(truth, alice()).with_targets(vec![Target::Object(a)]));
        let mut dm = Dm::default();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(!on_battlefield(&game, a));
        assert!(on_battlefield(&game, b), "the other face-down creature stays");
    }
}

mod bestow {
    use super::*;

    /// D2-02 / CR 702.103: a bestowed Aura that leaves is an enchantment
    /// creature card again.
    #[test]
    fn bestowed_aura_is_a_creature_card_in_the_graveyard() {
        let mut game = new_game();
        let eidolon = put(&mut game, &card("Boon Satyr"), alice());
        let eidolon_card = stable(&game, eidolon);
        game.object_mut(eidolon).unwrap().apply_bestow_cast_overlay();
        assert!(!game.object(eidolon).unwrap().card_types.contains(&CardType::Creature));
        game.move_object_by_effect(eidolon, Zone::Graveyard);
        let object = game.object(game.find_object_by_stable_id(eidolon_card).unwrap()).unwrap();
        assert!(object.card_types.contains(&CardType::Creature));
        assert!(!object.subtypes.contains(&Subtype::Aura));
    }
}

mod day_night {
    use super::*;

    fn cast_event(game: &mut GameState, caster: PlayerId) {
        let spell = game.create_object_from_definition(&plain("Some Spell", CardType::Instant), caster, Zone::Stack);
        let provenance = game
            .provenance_graph_mut()
            .alloc_root_event(ironsmith::events::EventKind::SpellCast);
        game.queue_trigger_event(
            provenance,
            ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
                provenance,
            ),
        );
        let _ = pending(game);
    }

    /// D2-03 / CR 730.2a: it becomes night if the previous turn's active
    /// player cast no spells, whatever other players cast.
    #[test]
    fn day_becomes_night_when_only_the_opponent_cast_spells() {
        let mut game = new_game();
        game.set_daytime(true);
        cast_event(&mut game, bob());
        game.next_turn();
        assert!(game.is_night);
    }

    #[test]
    fn day_stays_when_the_active_player_cast_a_spell() {
        let mut game = new_game();
        game.set_daytime(true);
        cast_event(&mut game, alice());
        game.next_turn();
        assert!(!game.is_night);
    }

    /// D2-04 / CR 701.27e, 702.145: transforming because day returned is a
    /// transform, so "transforms into Brutal Cathar" triggers.
    #[test]
    fn brutal_cathar_triggers_when_day_transforms_it_back() {
        let mut game = new_game();
        let defs = faces("Brutal Cathar // Moonrage Brute");
        for def in &defs {
            game.register_linked_face_definition(def);
        }
        let front = defs.iter().find(|d| d.card.name == "Brutal Cathar").unwrap();
        let cathar = put(&mut game, front, alice());
        let target = put(&mut game, &creature("Opposing Bear", 2, 2), bob());
        game.set_daytime(true);
        let _ = pending(&mut game);
        game.set_daytime(false);
        assert_eq!(game.object(cathar).unwrap().name.as_str(), "Moonrage Brute");
        let _ = pending(&mut game);
        game.set_daytime(true);
        assert_eq!(game.object(cathar).unwrap().name.as_str(), "Brutal Cathar");
        let mut dm = Dm::yes().prefer(&["Opposing Bear"]);
        settle(&mut game, &mut dm);
        assert!(!on_battlefield(&game, target), "exiled by the transform trigger");
    }
}

mod saga {
    use super::*;

    /// D2-05 / CR 714.3a: a Saga put onto the battlefield (not cast) gets its
    /// lore counter and chapter I triggers.
    #[test]
    fn reanimated_saga_gets_a_lore_counter_and_chapter_one() {
        let mut game = new_game();
        let plains = card("Plains");
        game.create_object_from_definition(&plains, alice(), Zone::Library);
        let saga = game.create_object_from_definition(&card("The Birth of Meletis"), alice(), Zone::Graveyard);
        let mut dm = Dm::yes().prefer(&["Plains"]);
        let saga = game.move_object_with_etb_processing_with_dm(saga, Zone::Battlefield, &mut dm).unwrap().new_id;
        assert_eq!(game.counter_count(saga, CounterType::Lore), 1);
        settle(&mut game, &mut dm);
        assert!(game.player(alice()).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name.as_str() == "Plains"), "chapter I");
    }
}

mod transform_leaves {
    use super::*;

    /// D2-06 / CR 712.8a: a transformed DFC that leaves has its front face.
    #[test]
    fn transformed_delver_dies_as_delver_of_secrets() {
        let mut game = new_game();
        let defs = faces("Delver of Secrets // Insectile Aberration");
        for def in &defs {
            game.register_linked_face_definition(def);
        }
        let front = defs.iter().find(|d| d.card.name == "Delver of Secrets").unwrap();
        let delver = put(&mut game, front, alice());
        let delver_card = stable(&game, delver);
        assert!(game.transform_permanent(delver));
        assert_eq!(game.object(delver).unwrap().name.as_str(), "Insectile Aberration");
        game.move_object_by_effect(delver, Zone::Graveyard);
        let object = game.object(game.find_object_by_stable_id(delver_card).unwrap()).unwrap();
        assert_eq!(object.name.as_str(), "Delver of Secrets");
    }
}

mod rooms {
    use super::*;

    fn room_defs(game: &mut GameState) -> CardDefinition {
        let defs = faces("Bottomless Pool // Locker Room");
        for def in &defs {
            game.register_linked_face_definition(def);
        }
        defs.into_iter().find(|d| d.card.name == "Bottomless Pool").unwrap()
    }

    fn unlock_actions(game: &GameState, room: ObjectId) -> Vec<ironsmith::special_actions::RoomDoor> {
        ironsmith::decision::compute_legal_actions(game, alice())
            .into_iter()
            .filter_map(|action| match action {
                ironsmith::decision::LegalAction::SpecialAction(
                    ironsmith::special_actions::SpecialAction::UnlockRoomDoor { room_id, door },
                ) if room_id == room => Some(door),
                _ => None,
            })
            .collect()
    }

    /// D2-07 / CR 709.5d-e: a Room that enters without being cast has both
    /// doors locked, and either can be unlocked.
    #[test]
    fn uncast_room_enters_with_both_doors_locked() {
        let mut game = new_game();
        let pool = room_defs(&mut game);
        let room = game.create_object_from_definition(&pool, alice(), Zone::Graveyard);
        let mut dm = Dm::default();
        let room = game.move_object_with_etb_processing_with_dm(room, Zone::Battlefield, &mut dm).unwrap().new_id;
        game.player_mut(alice()).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 6);
        game.refresh_continuous_state();
        let chars = game.current_characteristics(room).unwrap();
        assert!(chars.abilities.as_slice().is_empty(), "no door's rules text");
        let doors = unlock_actions(&game, room);
        assert_eq!(doors.len(), 2, "{doors:?}");
    }

    /// CR 709.5h: "when you unlock this door" triggers for the door that was
    /// cast.
    #[test]
    fn casting_bottomless_pool_triggers_its_unlock_ability() {
        let mut game = new_game();
        let pool = room_defs(&mut game);
        let bear = put(&mut game, &creature("Pool Target", 2, 2), bob());
        let spell = game.create_object_from_definition(&pool, alice(), Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice()));
        let mut dm = Dm::yes().prefer(&["Pool Target"]);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        settle(&mut game, &mut dm);
        assert!(!on_battlefield(&game, bear), "returned to its owner's hand");
    }
}

mod class_levels {
    use super::*;

    /// D2-08 / CR 716.2b, 716.4: class levels aren't counters — gaining a
    /// level puts no counter, and proliferate can't raise the level.
    #[test]
    fn innkeepers_talent_level_is_a_designation() {
        let mut game = new_game();
        let def = card("Innkeeper's Talent");
        let talent = put(&mut game, &def, alice());
        let level_two = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Activated(activated)
                    if activated.additional_restrictions.iter().any(|r| r.ends_with("class_level:2")) =>
                {
                    Some(activated.effects.clone())
                }
                _ => None,
            })
            .expect("Level 2 ability");
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(talent, alice(), level_two));
        let mut dm = Dm { yes: true, take_max: true, ..Dm::default() };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.class_level(talent), 2);
        assert_eq!(game.counter_count(talent, CounterType::Level), 0);
        game.object_mut(talent).unwrap().add_counters(CounterType::Level, 1);
        run(&mut game, Effect::proliferate(1), alice(), &mut dm);
        assert_eq!(game.class_level(talent), 2, "counters don't change the level");
    }
}

mod megamorph {
    use super::*;

    /// D2-10 / CR 702.37b, 614.1: megamorph's counter goes through counter
    /// replacements.
    #[test]
    fn den_protector_megamorph_counter_is_doubled_by_hardened_scales_rule() {
        let mut game = new_game();
        put(&mut game, &card("Hardened Scales"), alice());
        let protector = put(&mut game, &card("Den Protector"), alice());
        game.set_face_down(protector);
        game.refresh_continuous_state();
        {
            let pool = &mut game.player_mut(alice()).unwrap().mana_pool;
            pool.add(ironsmith::mana::ManaSymbol::Green, 1);
            pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        }
        let mut dm = Dm::yes();
        ironsmith::special_actions::perform(
            ironsmith::special_actions::SpecialAction::TurnFaceUp {
                permanent_id: protector,
                method: ironsmith::special_actions::TurnFaceUpMethod::MegamorphAbility,
            },
            &mut game,
            alice(),
            &mut dm,
        )
        .expect("turn face up");
        assert!(!game.is_face_down(protector));
        assert_eq!(game.counter_count(protector, CounterType::PlusOnePlusOne), 2);
    }
}
