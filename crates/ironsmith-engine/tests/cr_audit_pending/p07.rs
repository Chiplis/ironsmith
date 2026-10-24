//! CR audit (pending-entry pass, package p07): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effects::{
    ApplyContinuousEffect, DealDamageEffect, DestroyEffect, MoveToZoneEffect, TapEffect,
};
use ironsmith::game_state::{Phase, Target};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, ChooseSpec, DecisionMaker, EffectExecutor, ObjectId, Until, Zone};

fn compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture card {name} must compile: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture card {name} must be fully implemented"
    );
    def
}

fn two_player_main(active: PlayerId) -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}

/// Flexible decision maker used by the fixtures.
#[derive(Default)]
struct Dm {
    accept: bool,
    number: Option<u32>,
    option_words: Vec<String>,
    target_pref: Vec<Target>,
    object_pref: Vec<ObjectId>,
    object_players: Vec<PlayerId>,
    option_descriptions: Vec<String>,
    cost_mask: Option<usize>,
    object_log: Vec<(PlayerId, usize)>,
}

impl DecisionMaker for Dm {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.accept && ctx.can_accept
    }
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        self.number.unwrap_or(ctx.min).clamp(ctx.min, ctx.max)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.object_players.push(ctx.player);
        let creatures = game
            .battlefield
            .iter()
            .filter(|id| game.current_is_creature(**id))
            .count();
        self.object_log.push((ctx.player, creatures));
        let want = ctx.min.max(if ctx.min == 0 { 0 } else { 1 });
        let mut chosen: Vec<ObjectId> = self
            .object_pref
            .iter()
            .copied()
            .filter(|id| ctx.candidates.iter().any(|c| c.id == *id && c.legal))
            .take(ctx.max.unwrap_or(usize::MAX))
            .collect();
        for c in ctx.candidates.iter().filter(|c| c.legal) {
            if chosen.len() >= want {
                break;
            }
            if !chosen.contains(&c.id) {
                chosen.push(c.id);
            }
        }
        chosen
    }
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.option_descriptions.push(ctx.description.clone());
        if let Some(mask) = self.cost_mask
            && ctx.description.starts_with("Choose optional costs")
        {
            return ctx
                .options
                .iter()
                .filter(|o| mask & (1 << o.index) != 0)
                .map(|o| o.index)
                .collect();
        }
        let mut chosen: Vec<usize> = ctx
            .options
            .iter()
            .filter(|o| {
                o.legal
                    && self
                        .option_words
                        .iter()
                        .any(|w| o.description.to_lowercase().contains(&w.to_lowercase()))
            })
            .map(|o| o.index)
            .take(ctx.max)
            .collect();
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
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut out = Vec::new();
        for req in &ctx.requirements {
            let min = req.min_targets.max(1);
            let mut picked: Vec<Target> = self
                .target_pref
                .iter()
                .filter(|t| req.legal_targets.contains(t))
                .cloned()
                .take(min)
                .collect();
            for t in &req.legal_targets {
                if picked.len() >= min {
                    break;
                }
                if !picked.contains(t) {
                    picked.push(t.clone());
                }
            }
            out.extend(picked);
        }
        out
    }
}

fn act_with<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: LegalAction,
    dm: &mut D,
    queue: &mut TriggerQueue,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    game.turn.priority_player = Some(player);
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
                .ok_or_else(|| "requested casting method lost".to_string())?;
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                dm,
            );
        } else {
            let context = match &result {
                Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending native action has no decision: {other:?}")),
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
        return Err("native action exceeded decision bound".into());
    }
    Ok(())
}

fn cast_action(game: &GameState, player: PlayerId, spell: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
}

fn can_activate(game: &GameState, player: PlayerId, source: ObjectId) -> bool {
    compute_legal_actions(game, player)
        .unwrap()
        .iter()
        .any(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
}

fn cast_with(
    game: &mut GameState,
    player: PlayerId,
    spell: ObjectId,
    dm: &mut Dm,
    queue: &mut TriggerQueue,
) {
    let action = cast_action(game, player, spell).expect("fixture spell must be castable");
    act_with(game, player, action, dm, queue).expect("fixture cast must complete");
}

/// Drain pending triggers, apply SBAs, stack triggers and resolve until the stack is empty.
fn settle(game: &mut GameState, dm: &mut Dm, queue: &mut TriggerQueue) {
    for _ in 0..64 {
        ironsmith::game_loop::try_drain_pending_trigger_events(game, queue).unwrap();
        ironsmith::game_loop::check_and_apply_sbas_with(game, queue, dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("fixture did not settle");
}

/// Like `settle` but stops after `max` resolutions; returns the number of resolutions.
fn settle_bounded(
    game: &mut GameState,
    dm: &mut Dm,
    queue: &mut TriggerQueue,
    max: usize,
) -> usize {
    let mut n = 0;
    loop {
        ironsmith::game_loop::try_drain_pending_trigger_events(game, queue).unwrap();
        ironsmith::game_loop::check_and_apply_sbas_with(game, queue, dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, dm).unwrap();
        if game.stack.is_empty() || n >= max {
            return n;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
        n += 1;
    }
}

/// Drain pending trigger events into a fresh queue and return how many abilities triggered.
fn drain_count(game: &mut GameState, queue: &mut TriggerQueue) -> usize {
    ironsmith::game_loop::try_drain_pending_trigger_events(game, queue).unwrap();
    queue.entries.len()
}

fn enter_from(game: &mut GameState, def: &CardDefinition, owner: PlayerId, from: Zone) -> ObjectId {
    let card = game.create_object_from_definition(def, owner, from);
    let stable = game.object(card).map(|o| o.stable_id);
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false)
        .execute(game, &mut EffectContext::new_default(card, owner))
        .unwrap();
    game.battlefield
        .iter()
        .copied()
        .find(|id| Some(game.object(*id).unwrap().stable_id) == stable)
        .expect("fixture permanent must enter")
}

/// Resolve `effects` as an ability of `source` controlled by `controller` (events reach triggers).
fn resolve_effects(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    effects: Vec<ironsmith::Effect>,
) {
    game.stack.push(ironsmith::game_state::StackEntry::ability(
        source, controller, effects,
    ));
    ironsmith::game_loop::resolve_stack_entry(game).unwrap();
}

fn life(game: &GameState, p: PlayerId) -> i32 {
    game.player(p).unwrap().life
}

fn vanilla(name: &str, cost: &str, pt: &str) -> CardDefinition {
    compile(
        name,
        &format!("Mana cost: {cost}\nType: Creature — Bear\nPower/Toughness: {pt}"),
    )
}

fn land_def(name: &str) -> CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

// ---------------------------------------------------------------- 603.1 / 603.2a

#[test]
fn cr_603_1_603_2a_trigger_has_condition_and_effect_and_ignores_activation_bans() {
    use ironsmith::AbilityKind;
    let source_def = compile(
        "Tap watcher",
        "Mana cost: {0}\nType: Artifact\nWhenever this artifact becomes tapped, you gain 3 life.\n{0}: You gain 1 life.",
    );
    let trig = source_def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .expect("CR 603.1: a [When/Whenever/At] line is a triggered ability");
    assert!(
        !trig.effects.is_empty(),
        "CR 603.1: a triggered ability has an effect"
    );
    let ban = compile(
        "Activation ban",
        "Mana cost: {0}\nType: Enchantment\nActivated abilities of artifacts can't be activated.",
    );
    let mut game = two_player_main(alice());
    let source = game.create_object_from_definition(&source_def, alice(), Zone::Battlefield);
    assert!(
        can_activate(&game, alice(), source),
        "control: activation legal before the ban"
    );
    game.create_object_from_definition(&ban, bob(), Zone::Battlefield);
    assert!(
        !can_activate(&game, alice(), source),
        "the ban precludes activation"
    );
    // Trigger during the resolution of a spell-like effect (no one could cast spells now).
    game.turn.phase = Phase::Combat;
    let bob_source = game.create_object_from_definition(
        &vanilla("Bob source", "{0}", "1/1"),
        bob(),
        Zone::Battlefield,
    );
    resolve_effects(
        &mut game,
        bob_source,
        bob(),
        vec![ironsmith::Effect::new(TapEffect::with_spec(
            ChooseSpec::SpecificObject(source),
        ))],
    );
    let mut queue = TriggerQueue::new();
    assert_eq!(
        drain_count(&mut game, &mut queue),
        1,
        "CR 603.2a: an activation ban does not stop the ability from triggering"
    );
    let mut dm = Dm::default();
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(
        life(&game, alice()),
        23,
        "CR 603.1/603.2a: trigger resolves for its controller"
    );
}

#[test]
fn cr_603_1a_trigger_instruction_that_ability_cant_be_countered_is_supported() {
    let result = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Uncounterable trigger",
        "Mana cost: {0}\nType: Artifact\nWhenever this artifact becomes tapped, you gain 3 life. This ability can't be countered.",
        false,
    );
    assert!(
        result.is_ok(),
        "CR 603.1a: a triggered ability may carry an instruction after its effect stating that it can't be countered; compiler rejected it: {:?}",
        result.err()
    );
    let def = result.unwrap();
    let stifle = compile(
        "Ability counter",
        "Mana cost: {0}\nType: Instant\nCounter target activated or triggered ability.",
    );
    let mut game = two_player_main(alice());
    let source = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
    let counter = game.create_object_from_definition(&stifle, alice(), Zone::Hand);
    resolve_effects(
        &mut game,
        source,
        alice(),
        vec![ironsmith::Effect::new(TapEffect::with_spec(
            ChooseSpec::SpecificObject(source),
        ))],
    );
    let mut queue = TriggerQueue::new();
    let mut dm = Dm::default();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1);
    cast_with(&mut game, alice(), counter, &mut dm, &mut queue);
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(
        life(&game, alice()),
        23,
        "CR 603.1a: the uncounterable trigger still resolves"
    );
}

// ---------------------------------------------------------------- 603.2c / 603.2d / 603.2g / 603.2h

#[test]
fn cr_603_2c_one_destroy_all_event_triggers_once_per_land() {
    let watcher = compile(
        "Land grave watcher",
        "Mana cost: {0}\nType: Enchantment\nWhenever a land is put into a graveyard from the battlefield, you gain 1 life.",
    );
    let armageddon = compile(
        "Land wipe",
        "Mana cost: {0}\nType: Sorcery\nDestroy all lands.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&watcher, alice(), Zone::Battlefield);
    for (i, owner) in [alice(), alice(), bob()].into_iter().enumerate() {
        game.create_object_from_definition(
            &land_def(&format!("Land {i}")),
            owner,
            Zone::Battlefield,
        );
    }
    let spell = game.create_object_from_definition(&armageddon, alice(), Zone::Hand);
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(
        drain_count(&mut game, &mut queue),
        3,
        "CR 603.2c: one event with three land occurrences triggers three times"
    );
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(life(&game, alice()), 23);
}

#[test]
fn cr_603_2d_panharmonicon_makes_an_etb_trigger_trigger_an_additional_time() {
    let panharmonicon = compile(
        "Panharmonicon",
        "Mana cost: {4}\nType: Artifact\nIf an artifact or creature entering the battlefield causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
    );
    let etb = compile(
        "Entry gainer",
        "Mana cost: {0}\nType: Artifact\nWhen this artifact enters, you gain 3 life.",
    );
    for with_doubler in [false, true] {
        let mut game = two_player_main(alice());
        if with_doubler {
            game.create_object_from_definition(&panharmonicon, alice(), Zone::Battlefield);
        }
        enter_from(&mut game, &etb, alice(), Zone::Hand);
        let mut queue = TriggerQueue::new();
        let expected = if with_doubler { 2 } else { 1 };
        assert_eq!(
            drain_count(&mut game, &mut queue),
            expected,
            "CR 603.2d: doubler={with_doubler}"
        );
        let mut dm = Dm::default();
        settle(&mut game, &mut dm, &mut queue);
        assert_eq!(life(&game, alice()), 20 + 3 * expected as i32);
    }
}

#[test]
fn cr_603_2g_prevented_damage_and_replaced_death_trigger_nothing() {
    use ironsmith::effects::PreventAllDamageToTargetEffect;
    let dies = compile(
        "Death watcher",
        "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2\nWhen this creature dies, you gain 3 life.",
    );
    let rip = compile(
        "Grave replacer",
        "Mana cost: {0}\nType: Enchantment\nIf a card or token would be put into a graveyard from anywhere, exile it instead.",
    );
    for replace in [false, true] {
        let mut game = two_player_main(alice());
        if replace {
            game.create_object_from_definition(&rip, bob(), Zone::Battlefield);
        }
        let creature = game.create_object_from_definition(&dies, alice(), Zone::Battlefield);
        DestroyEffect::with_spec(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut EffectContext::new_default(creature, bob()))
            .unwrap();
        let mut queue = TriggerQueue::new();
        let mut dm = Dm::default();
        settle(&mut game, &mut dm, &mut queue);
        assert_eq!(
            life(&game, alice()),
            if replace { 20 } else { 23 },
            "CR 603.2g: replaced={replace}: a replaced dies event does not trigger dies abilities"
        );
    }
    let damaged = compile(
        "Damage watcher",
        "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 4/4\nWhenever this creature is dealt damage, you gain 3 life.",
    );
    for prevent in [false, true] {
        let mut game = two_player_main(alice());
        let creature = game.create_object_from_definition(&damaged, alice(), Zone::Battlefield);
        if prevent {
            PreventAllDamageToTargetEffect::new(
                ChooseSpec::SpecificObject(creature),
                Until::EndOfTurn,
            )
            .execute(
                &mut game,
                &mut EffectContext::new_default(creature, alice()),
            )
            .unwrap();
        }
        let src = game.create_object_from_definition(
            &vanilla("Pinger", "{R}", "1/1"),
            bob(),
            Zone::Battlefield,
        );
        resolve_effects(
            &mut game,
            src,
            bob(),
            vec![ironsmith::Effect::new(DealDamageEffect::new(
                2,
                ChooseSpec::SpecificObject(creature),
            ))],
        );
        assert_eq!(game.damage_on(creature), if prevent { 0 } else { 2 });
        let mut queue = TriggerQueue::new();
        let mut dm = Dm::default();
        // Bounded: the unprevented trigger repeats (reported under CR 603.2c).
        settle_bounded(&mut game, &mut dm, &mut queue, 1);
        assert_eq!(
            life(&game, alice()) > 20,
            !prevent,
            "CR 603.2g: prevented={prevent}: prevented damage is not dealt and triggers nothing"
        );
    }
}

#[test]
fn cr_603_2h_do_this_only_once_each_turn_triggers_only_until_the_action_is_taken() {
    let cornucopia = compile(
        "Ancient Cornucopia",
        "Mana cost: {2}{G}\nType: Artifact\nWhenever you cast a spell that's one or more colors, you may gain 1 life for each of that spell's colors. Do this only once each turn.\n{T}: Add one mana of any color.",
    );
    let green = compile(
        "Green cantrip",
        "Mana cost: {G}\nType: Instant\nYou gain 0 life.",
    );
    for accept_first in [true, false] {
        let mut game = two_player_main(alice());
        game.create_object_from_definition(&cornucopia, alice(), Zone::Battlefield);
        let mut gains = Vec::new();
        for _ in 0..2 {
            let spell = game.create_object_from_definition(&green, alice(), Zone::Hand);
            game.player_mut(alice()).unwrap().mana_pool.green += 1;
            let mut dm = Dm {
                accept: accept_first || !gains.is_empty(),
                ..Dm::default()
            };
            let before = life(&game, alice());
            let mut queue = TriggerQueue::new();
            cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
            settle(&mut game, &mut dm, &mut queue);
            gains.push(life(&game, alice()) - before);
        }
        let expected = if accept_first { vec![1, 0] } else { vec![0, 1] };
        assert_eq!(
            gains, expected,
            "CR 603.2h: accept_first={accept_first}: the ability triggers only while the action has not been taken this turn"
        );
    }
}

// ---------------------------------------------------------------- 603.6 family

#[test]
fn cr_603_6_zone_change_trigger_cannot_find_card_that_left_its_new_zone() {
    let def = compile(
        "Returning bear",
        "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2\nWhen this creature dies, return it to its owner's hand.",
    );
    for moved_away in [false, true] {
        let mut game = two_player_main(alice());
        let creature = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
        let stable = game.object(creature).unwrap().stable_id;
        DestroyEffect::with_spec(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut EffectContext::new_default(creature, bob()))
            .unwrap();
        let mut queue = TriggerQueue::new();
        let mut dm = Dm::default();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
            .unwrap();
        assert_eq!(game.stack.len(), 1);
        if moved_away {
            let card = game.find_object_by_stable_id(stable).unwrap();
            let exiled = game.move_object_by_effect(card, Zone::Exile).unwrap();
            // Even returning to the graveyard makes it a new object (CR 603.6 parenthetical).
            game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
        }
        settle(&mut game, &mut dm, &mut queue);
        let now = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(now).unwrap().zone,
            if moved_away {
                Zone::Graveyard
            } else {
                Zone::Hand
            },
            "CR 603.6: moved_away={moved_away}"
        );
    }
}

#[test]
fn cr_603_6b_continuous_effects_apply_the_moment_a_permanent_enters() {
    let animate = compile(
        "Land animator",
        "Mana cost: {0}\nType: Enchantment\nAll lands are 1/1 creatures that are still lands.",
    );
    let watcher = compile(
        "Creature entry watcher",
        "Mana cost: {0}\nType: Enchantment\nWhenever a creature enters, you gain 1 life.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&animate, alice(), Zone::Battlefield);
    game.create_object_from_definition(&watcher, alice(), Zone::Battlefield);
    enter_from(&mut game, &land_def("Entering land"), alice(), Zone::Hand);
    let mut queue = TriggerQueue::new();
    assert_eq!(
        drain_count(&mut game, &mut queue),
        1,
        "CR 603.6b: a land entering under 'all lands are creatures' is a creature entering"
    );
    let humility = compile(
        "Ability stripper",
        "Mana cost: {0}\nType: Enchantment\nAll creatures lose all abilities.",
    );
    let etb = compile(
        "Entry creature",
        "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2\nWhen this creature enters, you gain 3 life.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&humility, alice(), Zone::Battlefield);
    enter_from(&mut game, &etb, alice(), Zone::Hand);
    let mut queue = TriggerQueue::new();
    assert_eq!(
        drain_count(&mut game, &mut queue),
        0,
        "CR 603.6b: a creature that enters already without abilities has no ETB trigger"
    );
}

#[test]
fn cr_603_6d_enters_tapped_and_enters_with_counters_are_part_of_the_entry_event() {
    let tapped = compile(
        "Tapped entrant",
        "Mana cost: {0}\nType: Artifact\nThis artifact enters tapped.",
    );
    let tap_watcher = compile(
        "Tap watcher",
        "Mana cost: {0}\nType: Enchantment\nWhenever a permanent becomes tapped, you gain 1 life.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&tap_watcher, alice(), Zone::Battlefield);
    let id = enter_from(&mut game, &tapped, alice(), Zone::Hand);
    assert!(game.is_tapped(id), "CR 603.6d: enters tapped");
    let mut queue = TriggerQueue::new();
    assert_eq!(
        drain_count(&mut game, &mut queue),
        0,
        "CR 603.6d: entering tapped is a static effect of the entry event, not a later tap"
    );
    let countered = compile(
        "Counter entrant",
        "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 1/1\nThis creature enters with two +1/+1 counters on it.",
    );
    let power_watcher = compile(
        "Big entry watcher",
        "Mana cost: {0}\nType: Enchantment\nWhenever a creature with power 3 or greater enters, you gain 1 life.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&power_watcher, alice(), Zone::Battlefield);
    let id = enter_from(&mut game, &countered, alice(), Zone::Hand);
    assert_eq!(
        game.counter_count(id, ironsmith::CounterType::PlusOnePlusOne),
        2
    );
    let mut queue = TriggerQueue::new();
    assert_eq!(
        drain_count(&mut game, &mut queue),
        1,
        "CR 603.6d: the counters are on the creature as it enters, so it enters as a 3/3"
    );
}

#[test]
fn cr_603_6e_aura_trigger_finds_the_enchanted_card_in_the_graveyard() {
    let pact = compile(
        "Unhallowed Pact",
        "Mana cost: {2}{B}\nType: Enchantment — Aura\nEnchant creature\nWhen enchanted creature dies, return that card to the battlefield under your control.",
    );
    let mut game = two_player_main(alice());
    let creature = game.create_object_from_definition(
        &vanilla("Bob bear", "{1}{G}", "2/2"),
        bob(),
        Zone::Battlefield,
    );
    let stable = game.object(creature).unwrap().stable_id;
    let aura = game.create_object_from_definition(&pact, alice(), Zone::Battlefield);
    assert!(
        game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(creature))
    );
    DestroyEffect::with_spec(ChooseSpec::SpecificObject(creature))
        .execute(&mut game, &mut EffectContext::new_default(creature, bob()))
        .unwrap();
    let mut queue = TriggerQueue::new();
    let mut dm = Dm::default();
    settle(&mut game, &mut dm, &mut queue);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        game.object(now).unwrap().zone,
        Zone::Battlefield,
        "CR 603.6e: the Aura's leaves trigger finds the new graveyard object"
    );
    assert_eq!(game.controller_of_id(now), Some(alice()));
}

// ---------------------------------------------------------------- 604.5 / 604.6 / 604.7

#[test]
fn cr_604_5_cant_be_countered_functions_while_the_spell_is_on_the_stack() {
    let counterspell = compile(
        "Counter",
        "Mana cost: {0}\nType: Instant\nCounter target spell.",
    );
    for protected in [false, true] {
        let text = if protected {
            "Mana cost: {0}\nType: Instant\nThis spell can't be countered.\nYou gain 3 life."
        } else {
            "Mana cost: {0}\nType: Instant\nYou gain 3 life."
        };
        let gain = compile("Gain spell", text);
        let mut game = two_player_main(alice());
        let spell = game.create_object_from_definition(&gain, alice(), Zone::Hand);
        let counter = game.create_object_from_definition(&counterspell, alice(), Zone::Hand);
        let mut dm = Dm::default();
        let mut queue = TriggerQueue::new();
        cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
        cast_with(&mut game, alice(), counter, &mut dm, &mut queue);
        assert_eq!(game.stack.len(), 2);
        settle(&mut game, &mut dm, &mut queue);
        assert_eq!(
            life(&game, alice()),
            if protected { 23 } else { 20 },
            "CR 604.5: protected={protected}"
        );
    }
}

#[test]
fn cr_604_6_cast_permissions_and_restrictions_function_from_the_casting_zone() {
    let recursive = compile(
        "Graveyard caster",
        "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/2\nYou may cast this card from your graveyard.",
    );
    let plain = vanilla("Plain bear", "{0}", "2/2");
    let mut game = two_player_main(alice());
    let a = game.create_object_from_definition(&recursive, alice(), Zone::Graveyard);
    let b = game.create_object_from_definition(&plain, alice(), Zone::Graveyard);
    assert!(
        cast_action(&game, alice(), a).is_some(),
        "CR 604.6: 'You may cast this card' works from the graveyard"
    );
    assert!(cast_action(&game, alice(), b).is_none());
    let combat_only = compile(
        "Angelic Favor",
        "Mana cost: {3}{W}\nType: Instant\nIf you control a Plains, you may tap an untapped creature you control rather than pay this spell's mana cost.\nCast this spell only during combat.\nCreate a 4/4 white Angel creature token with flying. Exile it at the beginning of the next end step.",
    );
    let mut game = two_player_main(alice());
    let spell = game.create_object_from_definition(&combat_only, alice(), Zone::Hand);
    game.player_mut(alice()).unwrap().mana_pool.white = 1;
    game.player_mut(alice()).unwrap().mana_pool.colorless = 3;
    assert!(
        cast_action(&game, alice(), spell).is_none(),
        "CR 604.6: 'Cast this spell only during combat' restricts from hand"
    );
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    assert!(
        cast_action(&game, alice(), spell).is_some(),
        "CR 604.6: castable during combat"
    );
}

#[test]
fn cr_604_7_static_effect_stops_immediately_without_last_known_information() {
    let anthem = compile(
        "Anthem",
        "Mana cost: {0}\nType: Artifact\nCreatures you control get +1/+1.",
    );
    let mut game = two_player_main(alice());
    let lord = game.create_object_from_definition(&anthem, alice(), Zone::Battlefield);
    let bear = game.create_object_from_definition(
        &vanilla("Bear", "{0}", "1/1"),
        alice(),
        Zone::Battlefield,
    );
    DealDamageEffect::new(1, ChooseSpec::SpecificObject(bear))
        .execute(&mut game, &mut EffectContext::new_default(lord, bob()))
        .unwrap();
    assert_eq!(game.current_toughness(bear), Some(2));
    game.move_object_by_effect(lord, Zone::Exile).unwrap();
    assert_eq!(
        game.current_toughness(bear),
        Some(1),
        "CR 604.7: the static ability's effect ends with its source; no last-known information keeps it"
    );
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        game.object(bear)
            .map_or(true, |o| o.zone != Zone::Battlefield)
    );
}

// ---------------------------------------------------------------- 606 loyalty

fn walker_def() -> CardDefinition {
    compile(
        "Test walker",
        "Mana cost: {0}\nType: Legendary Planeswalker — Test\nLoyalty: 3\n+1: You gain 1 life.\n−2: You gain 2 life.\n−4: You gain 4 life.",
    )
}

#[test]
fn cr_606_2_606_4_loyalty_symbol_costs_add_or_remove_loyalty_counters() {
    use ironsmith::{AbilityKind, CounterType};
    let def = walker_def();
    let loyalty_flags: Vec<bool> = def
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Activated(x) => Some(x.is_loyalty_ability),
            _ => None,
        })
        .collect();
    assert_eq!(
        loyalty_flags,
        vec![true, true, true],
        "CR 606.2: loyalty-symbol costs make loyalty abilities"
    );
    for (index, expected) in [(0usize, 4u32), (1, 1)] {
        let mut game = two_player_main(alice());
        let walker = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
        assert_eq!(game.counter_count(walker, CounterType::Loyalty), 3);
        let mut dm = Dm::default();
        let mut queue = TriggerQueue::new();
        act_with(
            &mut game,
            alice(),
            LegalAction::ActivateAbility {
                source: walker,
                ability_index: index,
            },
            &mut dm,
            &mut queue,
        )
        .unwrap();
        assert_eq!(game.stack.len(), 1);
        assert_eq!(
            game.counter_count(walker, CounterType::Loyalty),
            expected,
            "CR 606.4: ability {index} cost changes loyalty counters as it is activated"
        );
    }
}

#[test]
fn cr_606_5_carth_additional_loyalty_cost_combines_with_printed_cost() {
    use ironsmith::CounterType;
    let carth = compile(
        "Carth the Lion",
        "Mana cost: {2}{B}{G}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 3/5\nPlaneswalkers' loyalty abilities you activate cost an additional [+1] to activate.",
    );
    let def = walker_def();
    let mut results = Vec::new();
    for index in [0usize, 2] {
        let mut game = two_player_main(alice());
        game.create_object_from_definition(&carth, alice(), Zone::Battlefield);
        let walker = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
        let action = LegalAction::ActivateAbility {
            source: walker,
            ability_index: index,
        };
        let legal = compute_legal_actions(&game, alice())
            .unwrap()
            .contains(&action);
        let mut after = None;
        if legal {
            let mut dm = Dm::default();
            let mut queue = TriggerQueue::new();
            act_with(&mut game, alice(), action, &mut dm, &mut queue).unwrap();
            after = Some(game.counter_count(walker, CounterType::Loyalty));
        }
        results.push((index, legal, after));
    }
    assert_eq!(
        results,
        vec![(0, true, Some(5)), (2, true, Some(0))],
        "CR 606.5: [+1] with Carth puts two counters; [-4] with Carth is a single [-3] payable from three counters"
    );
}

// ---------------------------------------------------------------- 608.2j / 608.2k / 609.2

#[test]
fn cr_608_2j_color_check_ignores_other_colors() {
    let mut results = Vec::new();
    for text in [
        "Destroy all black creatures.",
        "Destroy all nonblack creatures.",
    ] {
        let wipe = compile(
            "Color wipe",
            &format!("Mana cost: {{0}}\nType: Sorcery\n{text}"),
        );
        let mut game = two_player_main(alice());
        let ids: Vec<_> = [("WB", "{W}{B}"), ("B", "{B}"), ("W", "{W}")]
            .iter()
            .map(|(n, c)| {
                let id = game.create_object_from_definition(
                    &vanilla(n, c, "2/2"),
                    bob(),
                    Zone::Battlefield,
                );
                game.object(id).unwrap().stable_id
            })
            .collect();
        let spell = game.create_object_from_definition(&wipe, alice(), Zone::Hand);
        let mut dm = Dm::default();
        let mut queue = TriggerQueue::new();
        cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
        settle(&mut game, &mut dm, &mut queue);
        let alive: Vec<bool> = ids
            .iter()
            .map(|s| {
                game.find_object_by_stable_id(*s)
                    .is_some_and(|id| game.object(id).unwrap().zone == Zone::Battlefield)
            })
            .collect();
        results.push((text, alive));
    }
    assert_eq!(
        results,
        vec![
            ("Destroy all black creatures.", vec![false, false, true]),
            ("Destroy all nonblack creatures.", vec![true, true, false]),
        ],
        "CR 608.2j: [white-black, black, white] survivors; 'black' includes the white-and-black creature and 'nonblack' excludes it"
    );
}

#[test]
fn cr_608_2k_trigger_referenced_object_is_affected_after_characteristic_change() {
    let tapper = compile(
        "Entry tapper",
        "Mana cost: {0}\nType: Enchantment\nWhenever another creature enters, tap that creature.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&tapper, alice(), Zone::Battlefield);
    let creature = enter_from(
        &mut game,
        &vanilla("Entering bear", "{0}", "2/2"),
        bob(),
        Zone::Hand,
    );
    let mut queue = TriggerQueue::new();
    let mut dm = Dm::default();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(creature),
        Modification::SetCardTypes(vec![CardType::Artifact]),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(creature, bob()))
    .unwrap();
    assert!(!game.current_is_creature(creature));
    settle(&mut game, &mut dm, &mut queue);
    assert!(
        game.is_tapped(creature),
        "CR 608.2k: 'that creature' still refers to the object even though it is no longer a creature"
    );
}

#[test]
fn cr_609_2_type_changing_effect_does_not_reach_cards_in_other_zones() {
    let animate = compile(
        "Land animator",
        "Mana cost: {0}\nType: Enchantment\nAll lands are 1/1 creatures that are still lands.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&animate, alice(), Zone::Battlefield);
    let on_field =
        game.create_object_from_definition(&land_def("Field land"), alice(), Zone::Battlefield);
    let in_yard =
        game.create_object_from_definition(&land_def("Yard land"), alice(), Zone::Graveyard);
    let in_hand = game.create_object_from_definition(&land_def("Hand land"), alice(), Zone::Hand);
    assert!(game.current_is_creature(on_field));
    assert!(
        !game.current_is_creature(in_yard) && !game.current_is_creature(in_hand),
        "CR 609.2: the effect applies only to permanents"
    );
}

// ---------------------------------------------------------------- batch 2

fn tokens_controlled_by(game: &GameState, p: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token
                && game.controller_of_id(*id) == Some(p)
        })
        .collect()
}

fn activate_first(
    game: &mut GameState,
    player: PlayerId,
    source: ObjectId,
    dm: &mut Dm,
    queue: &mut TriggerQueue,
) {
    let action = compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
        .expect("fixture ability must be activatable");
    act_with(game, player, action, dm, queue).expect("fixture activation must complete");
}

#[test]
fn cr_603_2f_objects_in_hidden_zones_never_visible_do_not_trigger() {
    let watcher = compile(
        "Hidden watcher",
        "Mana cost: {0}\nType: Enchantment\nWhenever a creature enters, you gain 1 life.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&watcher, alice(), Zone::Library);
    game.create_object_from_definition(&watcher, alice(), Zone::Hand);
    enter_from(
        &mut game,
        &vanilla("Entering bear", "{0}", "2/2"),
        bob(),
        Zone::Hand,
    );
    let mut queue = TriggerQueue::new();
    assert_eq!(
        drain_count(&mut game, &mut queue),
        0,
        "CR 603.2f: objects in a library or hand are never visible to all players and their abilities do not trigger"
    );
}

#[test]
fn cr_603_7f_static_replacement_created_trigger_uses_source_and_its_controller_at_application() {
    use ironsmith::continuous::Modification;
    let head = compile(
        "Head of the Hunt",
        "Mana cost: {2}{B}{B}\nType: Creature — Wolf\nPower/Toughness: 4/3\nFlash\nIf a creature an opponent controls would die, exile it instead. When you do, create a 2/2 green Wolf creature token.",
    );
    let mut game = two_player_main(alice());
    let head_id = game.create_object_from_definition(&head, bob(), Zone::Battlefield);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(head_id),
        Modification::ChangeController(alice()),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(head_id, alice()))
    .unwrap();
    assert_eq!(game.controller_of_id(head_id), Some(alice()));
    let bear = game.create_object_from_definition(
        &vanilla("Bob bear", "{G}", "2/2"),
        bob(),
        Zone::Battlefield,
    );
    let bear_stable = game.object(bear).unwrap().stable_id;
    let src = game.create_object_from_definition(
        &vanilla("Killer", "{0}", "1/1"),
        alice(),
        Zone::Battlefield,
    );
    resolve_effects(
        &mut game,
        src,
        alice(),
        vec![ironsmith::Effect::new(DestroyEffect::with_spec(
            ChooseSpec::SpecificObject(bear),
        ))],
    );
    let bear_now = game.find_object_by_stable_id(bear_stable).unwrap();
    assert_eq!(game.object(bear_now).unwrap().zone, Zone::Exile);
    let mut queue = TriggerQueue::new();
    let mut dm = Dm::default();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "CR 603.7f: the replacement created exactly one triggered ability"
    );
    assert_eq!(
        (game.stack[0].object_id, game.stack[0].controller),
        (head_id, alice()),
        "CR 603.7f: source is the static ability's object; controller is that object's controller when the replacement applied"
    );
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(tokens_controlled_by(&game, alice()).len(), 1);
    assert!(tokens_controlled_by(&game, bob()).is_empty());
}

#[test]
fn cr_603_7g_608_3g_dash_delayed_trigger_is_created_on_entry_with_dash_card_as_source() {
    use ironsmith::{CastingMethod, TurnAction, TurnRunner, TurnRunnerState};
    let dasher = compile(
        "Dash raider",
        "Mana cost: {2}{R}\nType: Creature — Human Warrior\nPower/Toughness: 2/2\nDash {0}",
    );
    let mut game = two_player_main(alice());
    let card = game.create_object_from_definition(&dasher, alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let action = compute_legal_actions(&game, alice())
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card && *casting_method != CastingMethod::Normal))
        .expect("dash cast must be available");
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    act_with(&mut game, alice(), action, &mut dm, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    let before = game.effect_store.delayed_triggers.len();
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        before, 0,
        "CR 608.3g: the dash delayed trigger is created as the permanent enters, not while the spell is on the stack"
    );
    game.turn.phase = Phase::Ending;
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
    assert!(matches!(
        runner.advance(&mut game, &mut queue).unwrap(),
        TurnAction::RunPriority
    ));
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "CR 603.7g: one delayed return trigger at the next end step"
    );
    assert_eq!(
        (game.stack[0].object_id, game.stack[0].controller),
        (permanent, alice()),
        "CR 603.7g: the delayed trigger's source is the dash card and its controller is the player who took the action"
    );
    settle(&mut game, &mut dm, &mut queue);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Hand);
}

#[test]
fn cr_603_7h_608_2p_gimli_third_resolution_trigger_is_created_once_after_resolution() {
    let gimli = compile(
        "Gimli, Mournful Avenger",
        "Mana cost: {1}{R}{G}\nType: Legendary Creature — Dwarf Warrior\nPower/Toughness: 3/2\nGimli has indestructible as long as two or more creatures died under your control this turn.\nWhenever another creature you control dies, put a +1/+1 counter on Gimli. When this ability resolves for the third time this turn, Gimli fights up to one target creature you don't control.",
    );
    let mut game = two_player_main(alice());
    let g = game.create_object_from_definition(&gimli, alice(), Zone::Battlefield);
    let wall = game.create_object_from_definition(
        &vanilla("Big wall", "{0}", "0/30"),
        bob(),
        Zone::Battlefield,
    );
    let mut dm = Dm {
        target_pref: vec![Target::Object(wall)],
        ..Dm::default()
    };
    let mut damage = Vec::new();
    for i in 0..4 {
        let fodder = game.create_object_from_definition(
            &vanilla(&format!("Fodder {i}"), "{0}", "1/1"),
            alice(),
            Zone::Battlefield,
        );
        resolve_effects(
            &mut game,
            fodder,
            alice(),
            vec![ironsmith::Effect::new(DestroyEffect::with_spec(
                ChooseSpec::SpecificObject(fodder),
            ))],
        );
        let mut queue = TriggerQueue::new();
        settle(&mut game, &mut dm, &mut queue);
        damage.push(game.damage_on(wall));
    }
    assert_eq!(
        game.counter_count(g, ironsmith::CounterType::PlusOnePlusOne),
        4
    );
    assert_eq!(
        damage,
        vec![0, 0, 6, 6],
        "CR 603.7h/608.2p: the fight happens once, after the third resolution's counter (power 6), and not again on the fourth"
    );
}

#[test]
fn cr_603_12a_paying_a_cost_several_times_triggers_the_reflexive_ability_once() {
    let adversary = compile(
        "Intrepid Adversary",
        "Mana cost: {1}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/1\nLifelink\nWhen this creature enters, you may pay {1}{W} any number of times. When you pay this cost one or more times, put that many valor counters on this creature.\nCreatures you control get +1/+1 for each valor counter on this creature.",
    );
    let mut game = two_player_main(alice());
    let id = enter_from(&mut game, &adversary, alice(), Zone::Hand);
    game.player_mut(alice()).unwrap().mana_pool.white = 2;
    game.player_mut(alice()).unwrap().mana_pool.colorless = 2;
    let mut dm = Dm {
        accept: true,
        number: Some(2),
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    let mut reflexive_entries = 0;
    for _ in 0..16 {
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
            .unwrap();
        if game.stack.is_empty() {
            break;
        }
        reflexive_entries += 1;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    }
    let paid = 4
        - (game.player(alice()).unwrap().mana_pool.white
            + game.player(alice()).unwrap().mana_pool.colorless);
    assert_eq!(paid, 4, "fixture: the cost {{1}}{{W}} was paid twice");
    assert_eq!(
        (
            reflexive_entries - 1,
            game.counter_count(id, ironsmith::CounterType::Named("valor".into()))
        ),
        (1, 2),
        "CR 603.12a: paying the cost two times triggers the reflexive ability once, putting two counters"
    );
}

#[test]
fn cr_607_2e_noted_information_is_read_only_by_the_linked_object() {
    let bell = compile(
        "Bell Borca, Spectral Sergeant",
        "Mana cost: {2}{R}{W}\nType: Legendary Creature — Spirit Soldier\nPower/Toughness: */5\nNote the mana value of each card as it's put into exile.\nBell Borca's power is equal to the greatest number noted for it this turn.\nAt the beginning of your upkeep, exile the top card of your library. You may play that card this turn.",
    );
    let mut game = two_player_main(alice());
    let b1 = game.create_object_from_definition(&bell, alice(), Zone::Battlefield);
    for (name, cost) in [("Three drop", "{2}{R}"), ("One drop", "{R}")] {
        let card =
            game.create_object_from_definition(&vanilla(name, cost, "1/1"), bob(), Zone::Hand);
        resolve_effects(
            &mut game,
            b1,
            alice(),
            vec![ironsmith::Effect::new(MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(card),
                Zone::Exile,
                false,
            ))],
        );
    }
    assert_eq!(
        game.current_power(b1),
        Some(3),
        "CR 607.2e: the greatest noted mana value"
    );
    let b2 = game.create_object_from_definition(&bell, bob(), Zone::Battlefield);
    assert_eq!(
        game.current_power(b2),
        Some(0),
        "CR 607.2e: another object reads only information noted for it"
    );
}

#[test]
fn cr_607_2f_odd_or_even_choice_feeds_the_linked_ability() {
    use ironsmith::static_abilities::StaticAbilityId;
    let prerogative = compile(
        "Ashling's Prerogative",
        "Mana cost: {1}{R}\nType: Enchantment\nAs this enchantment enters, choose odd or even. (Zero is even.)\nEach creature with mana value of the chosen quality has haste.\nEach creature without mana value of the chosen quality enters tapped.",
    );
    for (word, odd_haste) in [("Odd", true), ("Even", false)] {
        let mut game = two_player_main(alice());
        let one = game.create_object_from_definition(
            &vanilla("One", "{R}", "1/1"),
            alice(),
            Zone::Battlefield,
        );
        let two = game.create_object_from_definition(
            &vanilla("Two", "{1}{R}", "2/2"),
            alice(),
            Zone::Battlefield,
        );
        let mut dm = Dm {
            option_words: vec![word.into()],
            ..Dm::default()
        };
        let card = game.create_object_from_definition(&prerogative, alice(), Zone::Hand);
        let mut ctx = EffectContext::new_default(card, alice()).with_decision_maker(&mut dm);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false)
            .execute(&mut game, &mut ctx)
            .unwrap();
        drop(ctx);
        assert_eq!(
            (
                game.current_has_static_ability_id(one, StaticAbilityId::Haste),
                game.current_has_static_ability_id(two, StaticAbilityId::Haste)
            ),
            (odd_haste, !odd_haste),
            "CR 607.2f: choice {word} decides which mana values get haste"
        );
    }
}

#[test]
fn cr_607_2g_life_paid_as_it_entered_is_read_by_the_linked_ability() {
    let processor = compile(
        "Phyrexian Processor",
        "Mana cost: {4}\nType: Artifact\nAs this artifact enters, pay any amount of life.\n{4}, {T}: Create an X/X black Phyrexian Minion creature token, where X is the life paid as this artifact entered.",
    );
    let mut game = two_player_main(alice());
    let mut dm = Dm {
        number: Some(5),
        accept: true,
        ..Dm::default()
    };
    let card = game.create_object_from_definition(&processor, alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let mut ctx = EffectContext::new_default(card, alice()).with_decision_maker(&mut dm);
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false)
        .execute(&mut game, &mut ctx)
        .unwrap();
    drop(ctx);
    assert_eq!(
        life(&game, alice()),
        15,
        "fixture: five life paid as it entered"
    );
    let id = game.find_object_by_stable_id(stable).unwrap();
    game.player_mut(alice()).unwrap().mana_pool.colorless = 4;
    let mut queue = TriggerQueue::new();
    activate_first(&mut game, alice(), id, &mut dm, &mut queue);
    settle(&mut game, &mut dm, &mut queue);
    let tokens = tokens_controlled_by(&game, alice());
    assert_eq!(tokens.len(), 1);
    assert_eq!(
        (
            game.current_power(tokens[0]),
            game.current_toughness(tokens[0])
        ),
        (Some(5), Some(5)),
        "CR 607.2g: X is the life paid as this artifact entered"
    );
}

#[test]
fn cr_607_2i_each_kicker_trigger_refers_only_to_its_own_kicker() {
    let battlemage = compile(
        "Stormscape Battlemage",
        "Mana cost: {2}{U}\nType: Creature — Metathran Wizard\nPower/Toughness: 2/2\nKicker {W} and/or {2}{B}\nWhen this creature enters, if it was kicked with its {W} kicker, you gain 3 life.\nWhen this creature enters, if it was kicked with its {2}{B} kicker, destroy target nonblack creature. That creature can't be regenerated.",
    );
    let mut results = Vec::new();
    for mask in [1usize, 2] {
        let mut game = two_player_main(alice());
        let victim = game.create_object_from_definition(
            &vanilla("Victim", "{G}", "2/2"),
            bob(),
            Zone::Battlefield,
        );
        let victim_stable = game.object(victim).unwrap().stable_id;
        let spell = game.create_object_from_definition(&battlemage, alice(), Zone::Hand);
        {
            let pool = &mut game.player_mut(alice()).unwrap().mana_pool;
            pool.blue = 1;
            pool.white = 1;
            pool.black = 1;
            pool.colorless = 4;
        }
        let mut dm = Dm {
            cost_mask: Some(mask),
            target_pref: vec![Target::Object(victim)],
            ..Dm::default()
        };
        let mut queue = TriggerQueue::new();
        cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
        settle(&mut game, &mut dm, &mut queue);
        let victim_alive = game
            .find_object_by_stable_id(victim_stable)
            .is_some_and(|id| game.object(id).unwrap().zone == Zone::Battlefield);
        results.push((mask, life(&game, alice()), victim_alive));
    }
    assert_eq!(
        results,
        vec![(1, 23, true), (2, 20, false)],
        "CR 607.2i: the {{W}} kicker feeds only the life trigger and the {{2}}{{B}} kicker only the destroy trigger"
    );
}

#[test]
fn cr_607_2j_variable_additional_cost_value_is_read_by_the_spell() {
    let hatred = compile(
        "Hatred",
        "Mana cost: {3}{B}{B}\nType: Instant\nAs an additional cost to cast this spell, pay X life.\nTarget creature gets +X/+0 until end of turn.",
    );
    let mut game = two_player_main(alice());
    let bear = game.create_object_from_definition(
        &vanilla("Bear", "{G}", "2/2"),
        alice(),
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(&hatred, alice(), Zone::Hand);
    game.player_mut(alice()).unwrap().mana_pool.black = 2;
    game.player_mut(alice()).unwrap().mana_pool.colorless = 3;
    let mut dm = Dm {
        number: Some(4),
        target_pref: vec![Target::Object(bear)],
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
    assert_eq!(
        life(&game, alice()),
        16,
        "fixture: X=4 life paid as it was cast"
    );
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(
        game.current_power(bear),
        Some(6),
        "CR 607.2j: X is the value chosen for the additional cost"
    );
}

#[test]
fn cr_607_2m_anchor_word_abilities_follow_the_chosen_anchor() {
    use ironsmith::{TurnAction, TurnRunner, TurnRunnerState};
    let siege = compile(
        "Citadel Siege",
        "Mana cost: {2}{W}{W}\nType: Enchantment\nAs this enchantment enters, choose Khans or Dragons.\n• Khans — At the beginning of combat on your turn, put two +1/+1 counters on target creature you control.\n• Dragons — At the beginning of combat on each opponent's turn, tap target creature that player controls.",
    );
    let mut results = Vec::new();
    for word in ["Khans", "Dragons"] {
        let mut game = two_player_main(alice());
        let bear = game.create_object_from_definition(
            &vanilla("Bear", "{G}", "2/2"),
            alice(),
            Zone::Battlefield,
        );
        let mut dm = Dm {
            option_words: vec![word.into()],
            ..Dm::default()
        };
        let card = game.create_object_from_definition(&siege, alice(), Zone::Hand);
        let mut ctx = EffectContext::new_default(card, alice()).with_decision_maker(&mut dm);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false)
            .execute(&mut game, &mut ctx)
            .unwrap();
        drop(ctx);
        let mut queue = TriggerQueue::new();
        settle(&mut game, &mut dm, &mut queue);
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::BeginCombat);
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        settle(&mut game, &mut dm, &mut queue);
        results.push((
            word,
            game.counter_count(bear, ironsmith::CounterType::PlusOnePlusOne),
        ));
    }
    assert_eq!(
        results,
        vec![("Khans", 2), ("Dragons", 0)],
        "CR 607.2m: only the ability preceded by the chosen anchor word functions"
    );
}

#[test]
fn cr_607_2n_cards_exiled_before_the_game_link_to_any_object_with_that_name() {
    use ironsmith::AbilityKind;
    let caller = compile(
        "Caller of the Untamed",
        "Mana cost: {3}{G}\nType: Creature — Elf Shaman\nPower/Toughness: 2/4\nBefore you shuffle your deck to start the game, you may reveal this card from your deck and exile a creature card you drafted that isn't in your deck.\n{X}, {T}: Create a token that's a copy of a card you exiled with cards named Caller of the Untamed. X is the mana value of that card.",
    );
    let pregame_index = caller
        .abilities
        .iter()
        .position(
            |a| matches!(&a.kind, AbilityKind::Static(s) if s.pregame_action_kind().is_some()),
        )
        .expect("pregame exile ability");
    let mut game = two_player_main(alice());
    game.enable_conspiracy(vec![(alice(), vec![])]).unwrap();
    let in_deck = game.create_object_from_definition(&caller, alice(), Zone::Library);
    let drafted = game.create_object_from_definition(
        &vanilla("Drafted beast", "{1}{G}", "3/3"),
        alice(),
        Zone::OutsideGame,
    );
    let exiled = game
        .exile_drafted_cards_before_shuffle(alice(), in_deck, pregame_index, &[drafted])
        .unwrap();
    assert_eq!(exiled.len(), 1);
    // A different object with the same name reads the pregame exile.
    let other = game.create_object_from_definition(&caller, alice(), Zone::Battlefield);
    game.player_mut(alice()).unwrap().mana_pool.colorless = 2;
    let mut dm = Dm {
        number: Some(2),
        object_pref: exiled.clone(),
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    activate_first(&mut game, alice(), other, &mut dm, &mut queue);
    settle(&mut game, &mut dm, &mut queue);
    let tokens = tokens_controlled_by(&game, alice());
    assert_eq!(
        tokens
            .iter()
            .map(|t| game.object(*t).unwrap().name.to_string())
            .collect::<Vec<_>>(),
        vec!["Drafted beast".to_string()],
        "CR 607.2n: the reader on another Caller links to the cards exiled with cards of that name before the game"
    );
}

#[test]
fn cr_607_2p_pregame_color_choice_persists_through_zone_changes() {
    use ironsmith::color::{Color, ColorSet};
    let faceless = compile(
        "Faceless One",
        "Mana cost: {5}\nType: Legendary Enchantment Creature — Background\nPower/Toughness: 3/3\nIf Faceless One is your commander, choose a color before the game begins. Faceless One is the chosen color.\nChoose a Background (You can have a Background as a second commander.)",
    );
    let mut game = two_player_main(alice());
    let id = game.create_object_from_definition(&faceless, alice(), Zone::Command);
    let stable = game.object(id).unwrap().stable_id;
    game.set_commander(id);
    // The engine exposes no pregame choice action for this ability; record the choice directly.
    game.set_chosen_color(id, Color::Red);
    assert_eq!(
        game.current_colors(id),
        Some(ColorSet::RED),
        "CR 607.2p/604.3: the linked characteristic-defining ability applies the pregame choice in the command zone"
    );
    let src = game.create_object_from_definition(
        &vanilla("Mover", "{0}", "1/1"),
        alice(),
        Zone::Battlefield,
    );
    resolve_effects(
        &mut game,
        src,
        alice(),
        vec![ironsmith::Effect::new(MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(id),
            Zone::Battlefield,
            false,
        ))],
    );
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.current_colors(now),
        Some(ColorSet::RED),
        "CR 607.2p: the CDA keeps referring to the pregame choice after the object changes zones"
    );
}

#[test]
fn cr_607_2q_exiled_with_counts_only_cards_exiled_to_pay_its_cost() {
    let murktide = compile(
        "Murktide Regent",
        "Mana cost: {5}{U}{U}\nType: Creature — Dragon\nPower/Toughness: 3/3\nDelve\nFlying\nThis creature enters with a +1/+1 counter on it for each instant and sorcery card exiled with it.\nWhenever an instant or sorcery card leaves your graveyard, put a +1/+1 counter on this creature.",
    );
    let instant = compile("Quick draw", "Mana cost: {U}\nType: Instant\nDraw a card.");
    let mut game = two_player_main(alice());
    let mut yard = Vec::new();
    yard.push(game.create_object_from_definition(&instant, alice(), Zone::Graveyard));
    yard.push(game.create_object_from_definition(&instant, alice(), Zone::Graveyard));
    yard.push(game.create_object_from_definition(
        &vanilla("Dead bear", "{G}", "2/2"),
        alice(),
        Zone::Graveyard,
    ));
    // An instant exiled earlier by something else is not "exiled with" Murktide.
    let unrelated = game.create_object_from_definition(&instant, alice(), Zone::Exile);
    let _ = unrelated;
    let spell = game.create_object_from_definition(&murktide, alice(), Zone::Hand);
    let stable = game.object(spell).unwrap().stable_id;
    game.player_mut(alice()).unwrap().mana_pool.blue = 2;
    game.player_mut(alice()).unwrap().mana_pool.colorless = 2;
    let mut dm = Dm {
        object_pref: yard.clone(),
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
    assert!(
        game.player(alice()).unwrap().graveyard.is_empty(),
        "fixture: all three cards delved"
    );
    // Resolve only the creature spell (cast-time graveyard-leave triggers are ignored here).
    let creature_entry = game.stack.iter().position(|e| !e.is_ability).unwrap();
    while game.stack.len() > creature_entry + 1 {
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    }
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.counter_count(now, ironsmith::CounterType::PlusOnePlusOne),
        2,
        "CR 607.2q: two instant cards were exiled to pay its cost"
    );
}

#[test]
fn cr_608_2e_608_2f_each_action_completes_for_all_players_in_apnap_order_before_the_next() {
    let edict = compile(
        "Double edict",
        "Mana cost: {0}\nType: Sorcery\nEach player sacrifices a creature. Then each player discards a card.",
    );
    let watcher = compile(
        "Death watcher",
        "Mana cost: {0}\nType: Enchantment\nWhenever a creature dies, you gain 1 life.",
    );
    let mut game = two_player_main(alice());
    game.create_object_from_definition(&watcher, alice(), Zone::Battlefield);
    for p in [alice(), bob()] {
        for i in 0..2 {
            game.create_object_from_definition(
                &vanilla(&format!("C{i}"), "{0}", "1/1"),
                p,
                Zone::Battlefield,
            );
            game.create_object_from_definition(
                &vanilla(&format!("H{i}"), "{0}", "1/1"),
                p,
                Zone::Hand,
            );
        }
    }
    let spell = game.create_object_from_definition(&edict, alice(), Zone::Hand);
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
    dm.object_log.clear();
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(
        dm.object_log,
        vec![(alice(), 4), (bob(), 4), (alice(), 2), (bob(), 2)],
        "CR 608.2e: sacrifice choices in APNAP order, then both sacrifices, then discard choices in APNAP order"
    );
    assert_eq!(
        drain_count(&mut game, &mut queue),
        2,
        "CR 608.2f: both sacrifices processed simultaneously"
    );
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(life(&game, alice()), 22);
}

#[test]
fn cr_608_2i_look_back_counts_attacker_that_is_no_longer_a_creature() {
    let captain = compile(
        "Search Party Captain",
        "Mana cost: {3}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/3\nThis spell costs {1} less to cast for each creature you attacked with this turn.\nWhen this creature enters, draw a card.",
    );
    let mut results = Vec::new();
    for (attacked, still_creature) in [(false, false), (true, true), (true, false)] {
        let mut game = two_player_main(alice());
        let cub = game.create_object_from_definition(
            &vanilla("Bear Cub", "{1}{G}", "2/2"),
            alice(),
            Zone::Battlefield,
        );
        if attacked {
            use ironsmith::combat_state::AttackTarget;
            use ironsmith::decision::AttackerDeclaration;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
            let mut combat = ironsmith::combat_state::new_combat();
            let mut q = TriggerQueue::new();
            ironsmith::game_loop::apply_attacker_declarations(
                &mut game,
                &mut combat,
                &mut q,
                &[AttackerDeclaration {
                    creature: cub,
                    target: AttackTarget::Player(bob()),
                }],
            )
            .expect("fixture attack must be legal");
            game.turn.step = None;
        }
        if !still_creature {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(cub),
                Modification::SetCardTypes(vec![CardType::Artifact]),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(cub, alice()))
            .unwrap();
            assert!(!game.current_is_creature(cub));
        }
        game.turn.phase = Phase::NextMain;
        let spell = game.create_object_from_definition(&captain, alice(), Zone::Hand);
        game.player_mut(alice()).unwrap().mana_pool.white = 1;
        game.player_mut(alice()).unwrap().mana_pool.colorless = 3;
        let mut dm = Dm::default();
        let mut queue = TriggerQueue::new();
        cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
        results.push((
            attacked,
            still_creature,
            game.player(alice()).unwrap().mana_pool.colorless,
        ));
    }
    assert_eq!(
        results,
        vec![(false, false, 0), (true, true, 1), (true, false, 1)],
        "CR 608.2i: the creature attacked with this turn still reduces the cost after ceasing to be a creature"
    );
}

#[test]
fn cr_608_2m_spell_that_leaves_the_stack_mid_resolution_finishes_resolving() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let def = CardDefinitionBuilder::new(ironsmith::CardId::new(), "Self-exiling spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(ironsmith::ManaCost::from_symbols(vec![]))
        .with_spell_effect(vec![
            ironsmith::Effect::new(MoveToZoneEffect::new(
                ChooseSpec::Source,
                Zone::Exile,
                false,
            )),
            ironsmith::Effect::gain_life(3),
        ])
        .build();
    let mut game = two_player_main(alice());
    let spell = game.create_object_from_definition(&def, alice(), Zone::Hand);
    let stable = game.object(spell).unwrap().stable_id;
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
    settle(&mut game, &mut dm, &mut queue);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        game.object(now).unwrap().zone,
        Zone::Exile,
        "fixture: the spell exiled itself while resolving"
    );
    assert_eq!(
        life(&game, alice()),
        23,
        "CR 608.2m: it continues to resolve fully after leaving the stack"
    );
}

#[test]
fn cr_608_3d_resolving_mutating_spell_merges_with_its_target() {
    use ironsmith::CastingMethod;
    let mutator = compile(
        "Mutating beast",
        "Mana cost: {3}{G}\nType: Creature — Beast\nPower/Toughness: 4/4\nMutate {0}\nTrample",
    );
    let mut game = two_player_main(alice());
    let host = game.create_object_from_definition(
        &vanilla("Host beast", "{G}", "2/2"),
        alice(),
        Zone::Battlefield,
    );
    let card = game.create_object_from_definition(&mutator, alice(), Zone::Hand);
    let action = compute_legal_actions(&game, alice())
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card && *casting_method != CastingMethod::Normal))
        .expect("mutate cast must be available");
    let mut dm = Dm {
        target_pref: vec![Target::Object(host)],
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    act_with(&mut game, alice(), action, &mut dm, &mut queue).unwrap();
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(
        game.battlefield.as_slice(),
        [host],
        "CR 608.3d: the spell merges with the target instead of entering separately"
    );
    assert!(game.current_has_static_ability_id(
        host,
        ironsmith::static_abilities::StaticAbilityId::Trample
    ));
}

#[test]
fn cr_609_4a_two_as_though_permissions_combine_flash_and_flashback() {
    use ironsmith::CastingMethod;
    let orrery = compile(
        "Vedalken Orrery",
        "Mana cost: {4}\nType: Artifact\nYou may cast spells as though they had flash.",
    );
    let sorcery = compile(
        "Flashback gain",
        "Mana cost: {0}\nType: Sorcery\nYou gain 3 life.\nFlashback {0}",
    );
    let mut results = Vec::new();
    for with_orrery in [false, true] {
        let mut game = two_player_main(alice());
        if with_orrery {
            game.create_object_from_definition(&orrery, alice(), Zone::Battlefield);
        }
        let card = game.create_object_from_definition(&sorcery, alice(), Zone::Graveyard);
        game.turn.active_player = bob();
        game.turn.priority_player = Some(alice());
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let castable = compute_legal_actions(&game, alice()).unwrap().iter().any(|a| {
            matches!(a, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card && *casting_method != CastingMethod::Normal)
        });
        results.push((with_orrery, castable));
    }
    assert_eq!(
        results,
        vec![(false, false), (true, true)],
        "CR 609.4a: flashback (cast from graveyard) and 'as though it had flash' both apply"
    );
}

#[test]
fn cr_609_7b_resolved_shield_rechecks_the_source_color_and_is_not_used_up_when_it_fails() {
    use ironsmith::color::ColorSet;
    let shield = compile(
        "Black ward",
        "Mana cost: {0}\nType: Instant\nThe next time a black source of your choice would deal damage to you this turn, prevent that damage.",
    );
    let mut game = two_player_main(alice());
    let knight = game.create_object_from_definition(
        &vanilla("Black knight", "{B}", "2/2"),
        bob(),
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(&shield, alice(), Zone::Hand);
    let mut dm = Dm {
        object_pref: vec![knight],
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    cast_with(&mut game, alice(), spell, &mut dm, &mut queue);
    settle(&mut game, &mut dm, &mut queue);
    let hit = |game: &mut GameState| {
        resolve_effects(
            game,
            knight,
            bob(),
            vec![ironsmith::Effect::new(DealDamageEffect::new(
                2,
                ChooseSpec::SpecificPlayer(alice()),
            ))],
        );
    };
    let mut lives = Vec::new();
    for color in [ColorSet::WHITE, ColorSet::BLACK, ColorSet::BLACK] {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(knight),
            Modification::SetColors(color),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(knight, bob()))
        .unwrap();
        hit(&mut game);
        lives.push(life(&game, alice()));
    }
    assert_eq!(
        lives,
        vec![18, 18, 16],
        "CR 609.7b: while white the shield prevents nothing and stays; once black again it prevents the next damage and is used up"
    );
}

#[test]
fn cr_609_7c_static_prevention_applies_to_matching_sources_off_the_battlefield() {
    let ward = compile(
        "Red ward",
        "Mana cost: {0}\nType: Enchantment\nPrevent all damage that red sources would deal to you.",
    );
    let bolt = compile(
        "Red bolt",
        "Mana cost: {R}\nType: Instant\nThis spell deals 3 damage to target player.",
    );
    let mut game = two_player_main(bob());
    game.create_object_from_definition(&ward, alice(), Zone::Battlefield);
    let spell = game.create_object_from_definition(&bolt, bob(), Zone::Hand);
    game.player_mut(bob()).unwrap().mana_pool.red = 1;
    let mut dm = Dm {
        target_pref: vec![Target::Player(alice())],
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    cast_with(&mut game, bob(), spell, &mut dm, &mut queue);
    settle(&mut game, &mut dm, &mut queue);
    assert_eq!(
        life(&game, alice()),
        20,
        "CR 609.7c: a red spell on the stack is a red source the static shield covers"
    );
    let green = game.create_object_from_definition(
        &vanilla("Green", "{G}", "2/2"),
        bob(),
        Zone::Battlefield,
    );
    resolve_effects(
        &mut game,
        green,
        bob(),
        vec![ironsmith::Effect::new(DealDamageEffect::new(
            2,
            ChooseSpec::SpecificPlayer(alice()),
        ))],
    );
    assert_eq!(
        life(&game, alice()),
        18,
        "control: a nonred source is not prevented"
    );
}

#[test]
fn cr_603_2c_single_damage_event_triggers_dealt_damage_ability_only_once() {
    let damaged = compile(
        "Damage watcher",
        "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 4/4\nWhenever this creature is dealt damage, you gain 3 life.",
    );
    let mut game = two_player_main(alice());
    let creature = game.create_object_from_definition(&damaged, alice(), Zone::Battlefield);
    let src = game.create_object_from_definition(
        &vanilla("Pinger", "{R}", "1/1"),
        bob(),
        Zone::Battlefield,
    );
    resolve_effects(
        &mut game,
        src,
        bob(),
        vec![ironsmith::Effect::new(DealDamageEffect::new(
            2,
            ChooseSpec::SpecificObject(creature),
        ))],
    );
    let mut queue = TriggerQueue::new();
    let mut dm = Dm::default();
    let resolutions = settle_bounded(&mut game, &mut dm, &mut queue, 6);
    assert_eq!(
        (resolutions, life(&game, alice())),
        (1, 23),
        "CR 603.2c: one damage event triggers the ability once; resolving it must not re-trigger it"
    );
}

#[test]
fn cr_603_10g_planeswalk_away_trigger_looks_back_at_the_departed_plane() {
    use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, CardBuilder, CardId, Effect, ManaCost, PlanarCardKind, PlayerFilter};
    let plane = |name: &str, trigger: bool| {
        let mut def = CardDefinition::new(
            CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Plane])
                .mana_cost(ManaCost::from_symbols(vec![]))
                .build(),
        );
        if trigger {
            def.abilities.push(Ability::triggered(
                Trigger::keyword_action(KeywordActionKind::Planeswalk, PlayerFilter::You),
                vec![Effect::gain_life(3)],
            ));
        }
        (def, PlanarCardKind::Plane)
    };
    let mut alice_deck: Vec<_> = (0..10)
        .map(|i| plane(&format!("Alice Plane {i}"), i == 9))
        .collect();
    let bob_deck: Vec<_> = (0..10)
        .map(|i| plane(&format!("Bob Plane {i}"), false))
        .collect();
    let _ = &mut alice_deck;
    let mut game = two_player_main(alice());
    game.enable_planechase(vec![(alice(), alice_deck), (bob(), bob_deck)])
        .unwrap();
    let named = |game: &GameState, name: &str| {
        game.planechase
            .as_ref()
            .unwrap()
            .card_kinds
            .keys()
            .copied()
            .find(|id| game.object(*id).is_some_and(|o| o.name == name))
            .unwrap()
    };
    let to_top = |game: &mut GameState, id: ObjectId| {
        let deck = game
            .planechase
            .as_mut()
            .unwrap()
            .decks
            .get_mut(&alice())
            .unwrap();
        let i = deck.iter().position(|c| *c == id).unwrap();
        deck.remove(i);
        deck.push(id);
    };
    let active = named(&game, "Alice Plane 9");
    let destination = named(&game, "Alice Plane 8");
    to_top(&mut game, active);
    game.reveal_starting_plane().unwrap();
    game.take_pending_trigger_events();
    to_top(&mut game, destination);
    game.planeswalk(alice(), active).unwrap();
    let events = game.take_pending_trigger_events();
    let event = events
        .iter()
        .find(|e| {
            e.downcast::<KeywordActionEvent>()
                .is_some_and(|k| k.action == KeywordActionKind::Planeswalk)
        })
        .expect("planeswalk event");
    let entries = ironsmith::check_triggers(&game, event);
    assert_eq!(
        entries.len(),
        1,
        "CR 603.10g: the plane planeswalked away from triggers, looking back in time even though it is now face down"
    );
}

#[test]
fn cr_608_2g_option_to_pay_mana_allows_activating_mana_abilities_even_with_floating_mana() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::decisions::context::ManaPaymentContext;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::mana_payment::{ManaPaymentResponse, manual_mana_abilities};
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, ObjectId, StackEntry, Zone};
    struct FloatExtraMana {
        source: ObjectId,
        initial_green: u32,
        calls: usize,
    }
    impl DecisionMaker for FloatExtraMana {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            true
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.calls += 1;
            assert_eq!(game.turn.priority_player, None);
            if self.calls == 1 {
                assert_eq!(
                    game.player(ctx.player).unwrap().mana_pool.green,
                    self.initial_green
                );
                assert!(ctx.plan.payable);
                assert!(
                    manual_mana_abilities(game, &ctx.request).contains(&(self.source, 0)),
                    "CR 608.2g / 605.3a: an effect giving the option to pay mana permits activating mana abilities even when floating mana already covers the cost"
                );
                ManaPaymentResponse::Activate {
                    source: self.source,
                    ability_index: 0,
                }
            } else {
                assert_eq!(self.calls, 2);
                assert_eq!(
                    game.player(ctx.player).unwrap().mana_pool.green,
                    self.initial_green + 1
                );
                ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                }
            }
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for initial_green in [0, 1] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = bob;
        game.turn.priority_player = None;
        game.player_mut(alice).unwrap().mana_pool.green = initial_green;
        let definition = CardDefinitionBuilder::new(CardId::new(), "Extra mana source")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]))
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.stack.push(StackEntry::ability(
            source,
            bob,
            vec![Effect::new(UnlessPaysEffect::new(
                vec![Effect::gain_life(5)],
                ironsmith::PlayerFilter::Specific(alice),
                vec![ManaSymbol::Green],
            ))],
        ));
        let mut dm = FloatExtraMana {
            source,
            initial_green,
            calls: 0,
        };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.calls, 2);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, initial_green);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}
