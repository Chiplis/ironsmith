//! CR audit (in-review completion pass, package q06): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{GameProgress, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_state::Target;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardId, CardType, ColorSet, CounterType, DecisionMaker, EffectExecutor, LegalAction,
    ManaSymbol, ObjectId, Phase, PowerToughness, PriorityLoopState, PriorityResponse, Zone,
};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture compile failed for {name}: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture {name} has unimplemented content"
    );
    def
}

fn new_game(active: PlayerId) -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn creature(game: &mut GameState, owner: PlayerId, name: &str, p: i32, t: i32) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(p, t))
        .build();
    game.create_object_from_definition(&def, owner, Zone::Battlefield)
}

fn life(game: &GameState, p: PlayerId) -> i32 {
    game.player(p).unwrap().life
}

fn pt(game: &GameState, id: ObjectId) -> (Option<i32>, Option<i32>) {
    let c = game.current_characteristics(id).unwrap();
    (c.power, c.toughness)
}

#[derive(Default)]
struct Script {
    prefer: Vec<Target>,
    option_pick: Option<usize>,
    option_log: Vec<(PlayerId, String, Vec<String>)>,
}

impl DecisionMaker for Script {
    fn decide_boolean(&mut self, g: &GameState, c: &BooleanContext) -> bool {
        SelectFirstDecisionMaker.decide_boolean(g, c)
    }
    fn decide_number(&mut self, _g: &GameState, c: &NumberContext) -> u32 {
        c.max
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(g, c)
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        self.option_log.push((
            c.player,
            c.description.clone(),
            c.options.iter().map(|o| o.description.clone()).collect(),
        ));
        if let Some(i) = self.option_pick {
            let legal: Vec<usize> = c.options.iter().filter(|o| o.legal).map(|o| o.index).collect();
            if c.min <= 1 && c.max >= 1 && !legal.is_empty() {
                return vec![legal[i.min(legal.len() - 1)]];
            }
        }
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, _g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let mut out = Vec::new();
        for req in &c.requirements {
            if let Some(t) = self.prefer.iter().find(|t| req.legal_targets.contains(t)) {
                out.push(*t);
            } else if let Some(t) = req.legal_targets.first() {
                out.push(*t);
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
        if state.pending_method_selection.is_some() {
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(0),
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
    Ok(())
}

fn legal(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    ironsmith::compute_legal_actions(game, player).unwrap()
}

fn cast<D: DecisionMaker>(game: &mut GameState, player: PlayerId, spell: ObjectId, dm: &mut D) {
    let action = legal(game, player)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .unwrap_or_else(|| panic!("fixture spell must be castable"));
    let mut queue = TriggerQueue::new();
    complete_action(game, player, action, dm, &mut queue).expect("fixture cast");
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).expect("stack triggers");
}

fn activation_indices(game: &GameState, player: PlayerId, source: ObjectId) -> Vec<usize> {
    legal(game, player)
        .into_iter()
        .filter_map(|a| match a {
            LegalAction::ActivateAbility { source: s, ability_index } if s == source => {
                Some(ability_index)
            }
            _ => None,
        })
        .collect()
}

fn activate<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    source: ObjectId,
    ability_index: usize,
    dm: &mut D,
) {
    let mut queue = TriggerQueue::new();
    complete_action(
        game,
        player,
        LegalAction::ActivateAbility { source, ability_index },
        dm,
        &mut queue,
    )
    .expect("fixture activation");
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).expect("stack triggers");
}

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

// ---------------------------------------------------------------------------
// 613.3 / 613.4 / 613.4d: ordering within layers and sublayers
// ---------------------------------------------------------------------------

#[test]
fn cr_613_4d_switch_examples_from_the_rule_text() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::Until;
    // Example 1: 1/3, +0/+1, switch -> 4/1; then +5/+0 -> 4/6.
    let mut game = new_game(A);
    let id = creature(&mut game, A, "Switch subject", 1, 3);
    let mut ctx = EffectContext::new_default(id, A);
    for m in [
        Modification::ModifyPowerToughness { power: 0, toughness: 1 },
        Modification::SwitchPowerToughness,
    ] {
        ApplyContinuousEffect::new(EffectTarget::Specific(id), m, Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .unwrap();
    }
    assert_eq!(pt(&game, id), (Some(4), Some(1)), "CR 613.4d example 1: 1/3 +0/+1 switched is 4/1");
    ApplyContinuousEffect::new(
        EffectTarget::Specific(id),
        Modification::ModifyPowerToughness { power: 5, toughness: 0 },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(pt(&game, id), (Some(4), Some(6)), "CR 613.4d example 1: later +5/+0 applies before the switch, giving 4/6");

    // Example 2: the +0/+1 effect ends before the switch ends -> 3/1.
    let mut game = new_game(A);
    let id = creature(&mut game, A, "Switch subject", 1, 3);
    let pump_source = creature(&mut game, A, "Pump source", 1, 1);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(id),
        Modification::ModifyPowerToughness { power: 0, toughness: 1 },
        Until::ThisLeavesTheBattlefield,
    )
    .execute(&mut game, &mut EffectContext::new_default(pump_source, A))
    .unwrap();
    ApplyContinuousEffect::new(
        EffectTarget::Specific(id),
        Modification::SwitchPowerToughness,
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(id, A))
    .unwrap();
    assert_eq!(pt(&game, id), (Some(4), Some(1)), "CR 613.4d example 2 setup");
    game.move_object_by_effect(pump_source, Zone::Graveyard).unwrap();
    assert_eq!(pt(&game, id), (Some(3), Some(1)), "CR 613.4d example 2: once +0/+1 ends the switched creature is 3/1");

    // Example 3: two switches cancel -> 1/4.
    let mut game = new_game(A);
    let id = creature(&mut game, A, "Switch subject", 1, 3);
    let mut ctx = EffectContext::new_default(id, A);
    for m in [
        Modification::ModifyPowerToughness { power: 0, toughness: 1 },
        Modification::SwitchPowerToughness,
        Modification::SwitchPowerToughness,
    ] {
        ApplyContinuousEffect::new(EffectTarget::Specific(id), m, Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .unwrap();
    }
    assert_eq!(pt(&game, id), (Some(1), Some(4)), "CR 613.4d example 3: two switches cancel, giving 1/4");
}

#[test]
fn cr_613_3_613_4_same_layer_and_sublayer_effects_apply_in_timestamp_order() {
    use ironsmith::continuous::{EffectTarget, Modification, PtSublayer};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::Until;
    for reverse in [false, true] {
        let mut game = new_game(A);
        let id = creature(&mut game, A, "Timestamp subject", 1, 1);
        let mut ctx = EffectContext::new_default(id, A);
        let mut mods = vec![
            (
                Modification::SetColors(ColorSet::RED),
                Modification::SetPowerToughness { power: 2.into(), toughness: 2.into(), sublayer: PtSublayer::Setting },
            ),
            (
                Modification::SetColors(ColorSet::BLUE),
                Modification::SetPowerToughness { power: 5.into(), toughness: 7.into(), sublayer: PtSublayer::Setting },
            ),
        ];
        if reverse {
            mods.reverse();
        }
        for (color, set_pt) in mods {
            for m in [color, set_pt] {
                ApplyContinuousEffect::new(EffectTarget::Specific(id), m, Until::EndOfTurn)
                    .execute(&mut game, &mut ctx)
                    .unwrap();
            }
        }
        let chars = game.current_characteristics(id).unwrap();
        let (want_color, want_pt) = if reverse {
            (ColorSet::RED, (Some(2), Some(2)))
        } else {
            (ColorSet::BLUE, (Some(5), Some(7)))
        };
        assert_eq!(chars.colors, want_color, "CR 613.3: within layer 5 the later color-setting effect wins (reverse={reverse})");
        assert_eq!((chars.power, chars.toughness), want_pt, "CR 613.4: within layer 7b the later P/T-setting effect wins (reverse={reverse})");
    }
}

// ---------------------------------------------------------------------------
// 614.1c: "enters with" is a replacement effect
// ---------------------------------------------------------------------------

#[test]
fn cr_614_1c_enters_with_counters_modifies_the_entry_event_itself() {
    let mut game = new_game(A);
    let watcher = compile(
        "Power watcher",
        "Mana cost: {2}\nType: Enchantment\nWhenever a creature with power 3 or greater enters, you gain 1 life.",
    );
    game.create_object_from_definition(&watcher, A, Zone::Battlefield);
    let elf = compile(
        "Counter elf",
        "Mana cost: {1}\nType: Creature — Elf\nPower/Toughness: 1/1\nThis creature enters with two +1/+1 counters on it.",
    );
    let hand = game.create_object_from_definition(&elf, A, Zone::Hand);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
    let mut dm = Script::default();
    cast(&mut game, A, hand, &mut dm);
    resolve_all(&mut game, &mut dm);
    let on_bf: Vec<ObjectId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).unwrap().name == "Counter elf")
        .collect();
    assert_eq!(on_bf.len(), 1, "fixture: elf entered");
    let counters = game
        .object(on_bf[0])
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);
    assert_eq!(counters, 2, "CR 614.1c: the creature enters with two +1/+1 counters");
    assert_eq!(
        life(&game, A),
        21,
        "CR 614.1c: 'enters with' modifies the entry event, so the creature already has power 3 as it enters and the entry trigger sees it"
    );
}

// ---------------------------------------------------------------------------
// 614.5: a replacement effect applies only once to an event
// ---------------------------------------------------------------------------

#[test]
fn cr_614_5_two_damage_doublers_quadruple_rather_than_loop() {
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::ChooseSpec;
    let furnace = compile(
        "Furnace of Rath",
        "Mana cost: {1}{R}{R}{R}\nType: Enchantment\nIf a source would deal damage to a permanent or player, it deals double that damage to that permanent or player instead.",
    );
    let mut game = new_game(A);
    game.create_object_from_definition(&furnace, A, Zone::Battlefield);
    game.create_object_from_definition(&furnace, B, Zone::Battlefield);
    let source = creature(&mut game, A, "Damage source", 2, 2);
    let victim = creature(&mut game, B, "Big victim", 1, 20);
    let mut ctx = EffectContext::new_default(source, A);
    DealDamageEffect::new(2, ChooseSpec::Player(ironsmith::PlayerFilter::Specific(B)))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(life(&game, B), 12, "CR 614.5 example: two doublers turn 2 damage into 8, not 4 and not unbounded");
    DealDamageEffect::new(1, ChooseSpec::SpecificObject(victim))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(game.damage_on(victim), 4, "CR 614.5: two doublers turn 1 damage to a permanent into 4");
}

// ---------------------------------------------------------------------------
// 615.12: unpreventable damage does not consume prevention shields
// ---------------------------------------------------------------------------

#[test]
fn cr_615_12_unpreventable_damage_does_not_reduce_an_existing_shield() {
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::ChooseSpec;
    let mut game = new_game(A);
    let shield = compile(
        "Shield spell",
        "Mana cost: {W}\nType: Instant\nPrevent the next 3 damage that would be dealt to target player this turn.",
    );
    let shield = game.create_object_from_definition(&shield, A, Zone::Hand);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White, 1);
    let mut dm = Script { prefer: vec![Target::Player(B)], ..Script::default() };
    cast(&mut game, A, shield, &mut dm);
    resolve_all(&mut game, &mut dm);
    let unprevent = compile(
        "No prevention",
        "Mana cost: {2}\nType: Enchantment\nDamage can't be prevented.",
    );
    let unprevent = game.create_object_from_definition(&unprevent, A, Zone::Battlefield);
    let source = creature(&mut game, A, "Damage source", 2, 2);
    let mut ctx = EffectContext::new_default(source, A);
    DealDamageEffect::new(2, ChooseSpec::Player(ironsmith::PlayerFilter::Specific(B)))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(life(&game, B), 18, "CR 615.12: unpreventable damage is dealt in full");
    game.move_object_by_effect(unprevent, Zone::Graveyard).unwrap();
    DealDamageEffect::new(3, ChooseSpec::Player(ironsmith::PlayerFilter::Specific(B)))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        life(&game, B),
        18,
        "CR 615.12: the earlier unpreventable damage did not reduce the 3-damage shield, so it still prevents all 3"
    );
}

// ---------------------------------------------------------------------------
// 616.1 / 616.1f: the affected object's controller chooses; the other does nothing
// ---------------------------------------------------------------------------

#[test]
fn cr_616_1_616_1f_controller_chooses_between_exile_and_shuffle_replacements() {
    use ironsmith::effects::DestroyEffect;
    use ironsmith::ChooseSpec;
    let rip = compile(
        "Rest in Peace",
        "Mana cost: {1}{W}\nType: Enchantment\nIf a card or token would be put into a graveyard from anywhere, exile it instead.",
    );
    let golem = compile(
        "Shuffle Golem",
        "Mana cost: {3}\nType: Artifact Creature — Golem\nPower/Toughness: 3/3\nIf this creature would die, shuffle it into its owner's library instead.",
    );
    let mut outcomes = Vec::new();
    let mut errors = Vec::new();
    for pick in [0usize, 1] {
        let mut game = new_game(A);
        game.create_object_from_definition(&rip, B, Zone::Battlefield);
        let g = game.create_object_from_definition(&golem, A, Zone::Battlefield);
        let stable = game.object(g).unwrap().stable_id;
        let mut dm = Script { option_pick: Some(pick), ..Script::default() };
        let source = creature(&mut game, B, "Destroyer", 1, 1);
        {
            let mut ctx = EffectContext::new_default(source, B).with_decision_maker(&mut dm);
            DestroyEffect::with_spec(ChooseSpec::SpecificObject(g))
                .execute(&mut game, &mut ctx)
                .unwrap();
        }
        let now = game.find_object_by_stable_id(stable).map(|id| game.object(id).unwrap().zone);
        for (player, desc, opts) in &dm.option_log {
            if *player != A {
                errors.push(format!("pick={pick}: replacement choice {desc:?} {opts:?} asked of {player:?}, required the destroyed creature's controller A"));
            }
        }
        if now == Some(Zone::Graveyard) {
            errors.push(format!("pick={pick}: creature reached the graveyard; one replacement must apply"));
        }
        outcomes.push(now);
    }
    assert!(errors.is_empty(), "CR 616.1: {}", errors.join("\n"));
    assert!(
        outcomes.contains(&Some(Zone::Exile)) && outcomes.contains(&Some(Zone::Library)),
        "CR 616.1/616.1f: the controller chooses which replacement applies first and the other then does nothing, so both exile and library outcomes must be reachable; got {outcomes:?}"
    );
}

// ---------------------------------------------------------------------------
// 701.19a / 701.19c: regeneration
// ---------------------------------------------------------------------------

#[test]
fn cr_701_19a_regeneration_removes_attacker_and_blocker_from_combat() {
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::effects::{DestroyEffect, RegenerateEffect};
    use ironsmith::{ChooseSpec, Step, Until};
    for regenerate_blocker in [false, true] {
        let mut game = new_game(A);
        let attacker = compile(
            "Hasty attacker",
            "Mana cost: {1}\nType: Creature — Warrior\nPower/Toughness: 2/2\nHaste",
        );
        let attacker = game.create_object_from_definition(&attacker, A, Zone::Battlefield);
        let blocker = creature(&mut game, B, "Blocker", 2, 2);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(&mut game, &mut combat, vec![(attacker, AttackTarget::Player(B))])
            .expect("fixture attack");
        combat.blockers.insert(attacker, vec![blocker]);
        combat.blocked_attackers.insert(attacker);
        game.combat = Some(combat);
        game.turn.step = Some(Step::DeclareBlockers);
        let subject = if regenerate_blocker { blocker } else { attacker };
        let mut ctx = EffectContext::new_default(subject, game.controller_of_id(subject).unwrap());
        RegenerateEffect::new(ChooseSpec::SpecificObject(subject), Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .unwrap();
        DestroyEffect::with_spec(ChooseSpec::SpecificObject(subject))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(game.battlefield.contains(&subject), "CR 701.19a: shield replaces destruction");
        assert!(game.is_tapped(subject), "CR 701.19a: regenerated permanent is tapped");
        let combat = game.combat.as_ref().unwrap();
        let still_in = if regenerate_blocker {
            combat.blockers.values().any(|v| v.contains(&subject))
        } else {
            combat_state::is_attacking(combat, subject)
        };
        assert!(
            !still_in,
            "CR 701.19a: a regenerated {} creature is removed from combat",
            if regenerate_blocker { "blocking" } else { "attacking" }
        );
    }
}

#[test]
fn cr_701_19c_cant_be_regenerated_allows_shield_creation_but_shield_does_not_apply() {
    let mut game = new_game(A);
    let troll = compile(
        "Regen troll",
        "Mana cost: {1}{B}\nType: Creature — Troll\nPower/Toughness: 2/2\n{B}: Regenerate this creature.",
    );
    let troll = game.create_object_from_definition(&troll, A, Zone::Battlefield);
    let terminate = compile(
        "Terminate",
        "Mana cost: {B}{R}\nType: Instant\nDestroy target creature. It can't be regenerated.",
    );
    let terminate = game.create_object_from_definition(&terminate, A, Zone::Hand);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 2);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red, 1);
    let mut dm = Script { prefer: vec![Target::Object(troll)], ..Script::default() };
    let idx = activation_indices(&game, A, troll);
    assert!(!idx.is_empty(), "CR 701.19c: activating a regeneration-shield ability is legal");
    activate(&mut game, A, troll, idx[0], &mut dm);
    resolve_all(&mut game, &mut dm);
    assert!(game.battlefield.contains(&troll), "fixture: troll still on battlefield");
    cast(&mut game, A, terminate, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert!(
        !game.battlefield.contains(&troll),
        "CR 701.19c: 'can't be regenerated' causes an existing regeneration shield not to apply"
    );
}

// ---------------------------------------------------------------------------
// 612.7: Spy Kit names come from the Oracle card reference, not the game
// ---------------------------------------------------------------------------

#[test]
fn cr_612_7_spy_kit_names_cover_nonlegendary_creature_cards_only() {
    use ironsmith::effects::AttachToEffect;
    use ironsmith::game_state::StackEntry;
    use ironsmith::{ChooseSpec, Effect, ObjectFilter, Value};
    let kit = compile(
        "Spy Kit",
        "Mana cost: {2}\nType: Artifact — Equipment\nEquipped creature gets +1/+1 and has all names of nonlegendary creature cards in addition to its name.\nEquip {2}",
    );
    let mut game = new_game(A);
    // The engine resolves names through the card catalog (filter.rs
    // is_nonlegendary_creature_card_name); the default test build has no
    // generated registry, so the reference cards are supplied as hidden
    // library cards, which the catalog fallback consults.
    for (name, text) in [
        ("Grizzly Bears", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2"),
        ("Llanowar Elves", "Mana cost: {G}\nType: Creature — Elf Druid\nPower/Toughness: 1/1\n{T}: Add {G}."),
        ("Progenitus", "Mana cost: {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}\nType: Legendary Creature — Hydra Avatar\nPower/Toughness: 10/10"),
        ("Lightning Bolt", "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target."),
    ] {
        let def = compile(name, text);
        game.create_object_from_definition(&def, B, Zone::Library);
    }
    let kit = game.create_object_from_definition(&kit, A, Zone::Battlefield);
    let bearer = creature(&mut game, A, "Plain bearer", 2, 2);
    AttachToEffect::new(ChooseSpec::SpecificObject(bearer))
        .execute(&mut game, &mut EffectContext::new_default(kit, A))
        .unwrap();
    assert_eq!(game.current_power(bearer), Some(3), "fixture: Spy Kit attached");
    let mut errors = Vec::new();
    for (name, expected) in [
        ("Plain bearer", 1),
        ("Grizzly Bears", 1),
        ("Llanowar Elves", 1),
        ("Progenitus", 0),
        ("Lightning Bolt", 0),
    ] {
        let mut filter = ObjectFilter::creature();
        filter.specific = Some(bearer);
        filter.name = Some(name.into());
        let before = life(&game, A);
        game.stack.push(StackEntry::ability(bearer, A, vec![Effect::gain_life(Value::Count(filter))]));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let got = life(&game, A) - before;
        if got != expected {
            errors.push(format!("name {name:?}: matched {got}, required {expected}"));
        }
    }
    assert!(
        errors.is_empty(),
        "CR 612.7: the equipped creature has the name of each nonlegendary creature card (legendary and noncreature card names excluded): {}",
        errors.join("; ")
    );
}
