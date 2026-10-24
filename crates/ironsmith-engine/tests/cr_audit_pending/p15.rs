//! CR audit (pending-entry pass, package p15): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{
    BooleanContext, ManaPaymentContext, SelectObjectsContext, SelectOptionsContext,
};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{
    Ability, CardBuilder, CardDefinition, CardId, CardType, Effect, ObjectId, PlayerFilter,
    StackEntry, Zone,
};

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}
fn new_game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = ironsmith::Phase::FirstMain;
    game
}

fn compile(name: &str, text: &str) -> CardDefinition {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} must compile: {error:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
        "{name} compiled with unimplemented content"
    );
    definition
}

fn plain(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .build(),
    )
}

fn bear(name: &str) -> CardDefinition {
    compile(
        name,
        "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
    )
}

fn fill_library(game: &mut GameState, player: PlayerId, count: usize) {
    for index in 0..count {
        game.create_object_from_definition(
            &plain(&format!("Library card {index}"), CardType::Artifact),
            player,
            Zone::Library,
        );
    }
}

/// Answers yes to optional choices, confirms payable mana plans, prefers
/// options whose text contains one of `prefer`, and records every prompt.
#[derive(Default)]
struct Scripted {
    prefer: Vec<String>,
    decline: bool,
    option_prompts: Vec<(String, Vec<String>)>,
    boolean_prompts: Vec<String>,
    object_pick: Vec<ObjectId>,
    object_queue: std::collections::VecDeque<ObjectId>,
    bool_queue: std::collections::VecDeque<bool>,
    text: Option<String>,
    number: Option<u32>,
    target_pick: Vec<ironsmith::Target>,
}

impl Scripted {
    fn yes() -> Self {
        Self::default()
    }
    fn preferring(prefer: &[&str]) -> Self {
        Self {
            prefer: prefer.iter().map(|text| text.to_string()).collect(),
            ..Self::default()
        }
    }
}

impl DecisionMaker for Scripted {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.boolean_prompts.push(ctx.description.clone());
        if let Some(answer) = self.bool_queue.pop_front() {
            return answer && ctx.can_accept;
        }
        !self.decline && ctx.can_accept
    }

    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(front) = self.object_queue.front().copied() {
            let name = game.current_name(front).map(|name| name.to_string());
            let found = ctx.options.iter().find(|option| {
                option.legal
                    && (option.object_id == Some(front)
                        || name.as_ref().is_some_and(|name| {
                            option.description.starts_with("Enter as a copy of")
                                && option.description.ends_with(name.as_str())
                        }))
            });
            if let Some(option) = found {
                self.object_queue.pop_front();
                self.option_prompts.push((ctx.description.clone(), vec![option.description.clone()]));
                return vec![option.index];
            }
        }
        self.option_prompts.push((
            ctx.description.clone(),
            ctx.options.iter().map(|option| option.description.clone()).collect(),
        ));
        for preferred in &self.prefer {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|option| {
                    option.legal
                        && option
                            .description
                            .to_lowercase()
                            .contains(preferred.to_lowercase().as_str())
                })
            {
                return vec![option.index];
            }
        }
        if ctx.min == 0 {
            return Vec::new();
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .take(ctx.min)
            .map(|option| option.index)
            .collect()
    }

    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(front) = self.object_queue.front().copied()
            && ctx.candidates.iter().any(|candidate| candidate.legal && candidate.id == front)
        {
            self.object_queue.pop_front();
            return vec![front];
        }
        let picked: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal && self.object_pick.contains(&candidate.id))
            .map(|candidate| candidate.id)
            .collect();
        if !picked.is_empty() {
            return picked;
        }
        if let Some(selected) = ctx.default_alternative_selection() {
            return selected;
        }
        if ctx.relation_filter.is_some() {
            return ctx
                .legal_relation_selection(game, ctx.min)
                .unwrap_or_default();
        }
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.min)
            .collect()
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        self.number.unwrap_or(ctx.min).clamp(ctx.min, ctx.max)
    }

    fn decide_text(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::TextInputContext,
    ) -> String {
        self.text
            .clone()
            .or_else(|| ctx.initial_value.clone())
            .unwrap_or_default()
    }

    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::Target> {
        let mut chosen = Vec::new();
        for requirement in &ctx.requirements {
            let mut picked: Vec<ironsmith::Target> = self
                .target_pick
                .iter()
                .filter(|target| requirement.legal_targets.contains(target) && !chosen.contains(*target))
                .cloned()
                .collect();
            let max = requirement.max_targets.unwrap_or(usize::MAX);
            picked.truncate(max);
            if picked.len() < requirement.min_targets {
                for target in &requirement.legal_targets {
                    if picked.len() >= requirement.min_targets {
                        break;
                    }
                    if !picked.contains(target) && !chosen.contains(target) {
                        picked.push(target.clone());
                    }
                }
            }
            chosen.extend(picked);
        }
        chosen
    }

    fn decide_mana_payment(
        &mut self,
        _game: &GameState,
        ctx: &ManaPaymentContext,
    ) -> ManaPaymentResponse {
        if ctx.plan.payable {
            ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        } else {
            ManaPaymentResponse::Cancel
        }
    }
}

fn resolve_top(game: &mut GameState, dm: &mut dyn DecisionMaker) {
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).expect("stack entry resolves");
}

/// Put a compiled spell card on the stack under `caster` and resolve it.
fn resolve_spell(
    game: &mut GameState,
    definition: &CardDefinition,
    caster: PlayerId,
    targets: Vec<ironsmith::Target>,
    dm: &mut dyn DecisionMaker,
) -> ObjectId {
    let spell = game.create_object_from_definition(definition, caster, Zone::Stack);
    game.stack
        .push(StackEntry::new(spell, caster).with_targets(targets));
    resolve_top(game, dm);
    spell
}

fn triggered_effects(definition: &CardDefinition, index: usize) -> ironsmith::resolution::ResolutionProgram {
    let triggered: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
            _ => None,
        })
        .collect();
    triggered
        .get(index)
        .cloned()
        .unwrap_or_else(|| panic!("triggered ability {index} of {}", definition.name()))
}

fn activated_effects(definition: &CardDefinition, index: usize) -> ironsmith::resolution::ResolutionProgram {
    let activated: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated.effects.clone()),
            _ => None,
        })
        .collect();
    activated
        .get(index)
        .cloned()
        .unwrap_or_else(|| panic!("activated ability {index} of {}", definition.name()))
}

/// Put an ability of `source` (with the given program) on the stack and resolve it.
fn resolve_ability(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    program: ironsmith::resolution::ResolutionProgram,
    targets: Vec<ironsmith::Target>,
    dm: &mut dyn DecisionMaker,
) {
    game.stack
        .push(StackEntry::ability(source, controller, program).with_targets(targets));
    resolve_top(game, dm);
}

fn hand_len(game: &GameState, player: PlayerId) -> usize {
    game.player(player).unwrap().hand.len()
}

fn die_rolls(game: &GameState, player: PlayerId) -> u32 {
    game.turn_store.turn_history.completed_die_roll_count(player)
}

// ---------------------------------------------------------------------------
// CR 706: rolling dice
// ---------------------------------------------------------------------------

#[test]
fn cr_706_1_706_1a_dn_rolls_produce_each_face_from_1_to_n() {
    let mut game = new_game();
    let source = game.create_object_from_definition(
        &plain("Roller", CardType::Artifact),
        alice(),
        Zone::Battlefield,
    );
    for sides in [6u32, 20] {
        let mut seen = std::collections::BTreeMap::<i64, u32>::new();
        let rolls = 40 * sides;
        for _ in 0..rolls {
            let mut ctx = EffectContext::new_default(source, alice());
            let outcome = ironsmith::effects::execute_effect(
                &mut game,
                &Effect::roll_die(sides, PlayerFilter::You),
                &mut ctx,
            )
            .expect("die roll resolves");
            *seen.entry(outcome.as_count().expect("die result")).or_default() += 1;
        }
        let faces: Vec<i64> = seen.keys().copied().collect();
        assert_eq!(
            faces,
            (1..=i64::from(sides)).collect::<Vec<_>>(),
            "CR 706.1a: a d{sides} has exactly the outcomes 1..={sides}"
        );
        for (face, count) in &seen {
            assert!(
                *count >= 10,
                "CR 706.1a: d{sides} outcomes are equally likely; face {face} appeared only {count} times in {rolls}"
            );
        }
    }
    let before = die_rolls(&game, alice());
    let celebr = compile(
        "Celebr-8000",
        "Mana cost: {5}\nType: Artifact Creature — Clown Robot\nPower/Toughness: 3/3\nAt the beginning of combat on your turn, roll two six-sided dice. For each result of 1, this creature gets +1/+1 until end of turn. For each other result, it gains the indicated ability until end of turn. If you rolled doubles, it also gains double strike until end of turn.\n• 2 — menace\n• 3 — vigilance\n• 4 — lifelink\n• 5 — flying\n• 6 — indestructible",
    );
    let robot = game.create_object_from_definition(&celebr, alice(), Zone::Battlefield);
    let mut dm = Scripted::yes();
    resolve_ability(&mut game, robot, alice(), triggered_effects(&celebr, 0), vec![], &mut dm);
    assert_eq!(
        die_rolls(&game, alice()) - before,
        2,
        "CR 706.1: \"roll two six-sided dice\" rolls exactly two dice"
    );
}

fn celebr_with(first: u32, second: u32) -> (GameState, ObjectId) {
    let mut game = new_game();
    let celebr = compile(
        "Celebr-8000",
        "Mana cost: {5}\nType: Artifact Creature — Clown Robot\nPower/Toughness: 3/3\nAt the beginning of combat on your turn, roll two six-sided dice. For each result of 1, this creature gets +1/+1 until end of turn. For each other result, it gains the indicated ability until end of turn. If you rolled doubles, it also gains double strike until end of turn.\n• 2 — menace\n• 3 — vigilance\n• 4 — lifelink\n• 5 — flying\n• 6 — indestructible",
    );
    let robot = game.create_object_from_definition(&celebr, alice(), Zone::Battlefield);
    game.force_next_die_roll(first);
    game.force_next_die_roll(second);
    let mut dm = Scripted::yes();
    resolve_ability(&mut game, robot, alice(), triggered_effects(&celebr, 0), vec![], &mut dm);
    (game, robot)
}

#[test]
fn cr_706_5_rolling_doubles_requires_both_results_equal() {
    let (game, robot) = celebr_with(3, 3);
    assert!(
        game.object_has_static_ability_id(robot, StaticAbilityId::Vigilance),
        "CR 706.5 fixture: a result of 3 grants vigilance"
    );
    assert!(
        game.object_has_static_ability_id(robot, StaticAbilityId::DoubleStrike),
        "CR 706.5: two equal results are doubles and grant double strike"
    );
    let (game, robot) = celebr_with(3, 4);
    assert!(game.object_has_static_ability_id(robot, StaticAbilityId::Lifelink));
    assert!(
        !game.object_has_static_ability_id(robot, StaticAbilityId::DoubleStrike),
        "CR 706.5: unequal results are not doubles"
    );
    let (game, robot) = celebr_with(1, 1);
    assert_eq!(
        (game.calculated_power(robot), game.calculated_toughness(robot)),
        (Some(5), Some(5)),
        "Celebr-8000: each result of 1 gives +1/+1"
    );
    assert!(
        game.object_has_static_ability_id(robot, StaticAbilityId::DoubleStrike),
        "CR 706.5: double ones are doubles too"
    );
}

const CONTACT_OTHER_PLANE: &str = "Mana cost: {3}{U}\nType: Instant\nRoll a d20.\n1—9 | Draw two cards.\n10—19 | Scry 2, then draw two cards.\n20 | Scry 3, then draw three cards.";

#[test]
fn cr_706_3_706_3a_results_table_rows_select_the_matching_single_number_or_range() {
    let contact = compile("Contact Other Plane", CONTACT_OTHER_PLANE);
    for (forced, expected_draws) in [(1, 2usize), (9, 2), (10, 2), (19, 2), (20, 3)] {
        let mut game = new_game();
        fill_library(&mut game, alice(), 10);
        game.force_next_die_roll(forced);
        let mut dm = Scripted::yes();
        resolve_spell(&mut game, &contact, alice(), vec![], &mut dm);
        assert_eq!(
            hand_len(&game, alice()),
            expected_draws,
            "CR 706.3a: a roll of {forced} uses only the row whose N1—N2 range or single number contains it"
        );
        assert_eq!(die_rolls(&game, alice()), 1);
    }
}

const REVIVIFY: &str = "Mana cost: {2}{W}\nType: Instant\nRoll a d20 and add the number of creature cards in your graveyard that were put there from the battlefield this turn.\n1—14 | Return all creature cards in your graveyard that were put there from the battlefield this turn to your hand.\n15+ | Return those cards from your graveyard to the battlefield.";

#[test]
fn cr_706_3a_open_ended_n_plus_row_uses_the_modified_result() {
    let revivify = compile("Revivify", REVIVIFY);
    for (forced, expect_battlefield) in [(13u32, false), (14, true), (20, true)] {
        let mut game = new_game();
        let creature = game.create_object_from_definition(&bear("Fallen Bear"), alice(), Zone::Battlefield);
        let stable = game.object(creature).unwrap().stable_id;
        let source = game.create_object_from_definition(&plain("Killer", CardType::Artifact), alice(), Zone::Battlefield);
        resolve_ability(
            &mut game,
            source,
            alice(),
            vec![Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                ironsmith::ChooseSpec::SpecificObject(creature),
                Zone::Graveyard,
                false,
            ))]
            .into(),
            vec![],
            &mut Scripted::yes(),
        );
        game.force_next_die_roll(forced);
        let mut dm = Scripted::yes();
        resolve_spell(&mut game, &revivify, alice(), vec![], &mut dm);
        let now = game.find_object_by_stable_id(stable).expect("bear card");
        let zone = game.object(now).unwrap().zone;
        let expected = if expect_battlefield { Zone::Battlefield } else { Zone::Hand };
        assert_eq!(
            zone, expected,
            "CR 706.3a: natural {forced} + 1 must select the {} row",
            if expect_battlefield { "15+" } else { "1—14" }
        );
    }
}

#[test]
fn cr_706_3b_roll_modifier_result_rows_and_table_form_one_ability() {
    let investigator = compile(
        "Arcane Investigator",
        "Mana cost: {1}{U}\nType: Creature — Elf Wizard\nPower/Toughness: 2/1\nSearch the Room — {5}{U}: Roll a d20.\n1—9 | Draw a card.\n10—20 | Look at the top three cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order.",
    );
    assert_eq!(
        investigator.abilities.len(),
        1,
        "CR 706.3b: the roll instruction and its results table are one ability"
    );
    assert!(matches!(
        investigator.abilities[0].kind,
        ironsmith::ability::AbilityKind::Activated(_)
    ));
    let revivify = compile("Revivify", REVIVIFY);
    assert!(
        revivify.abilities.is_empty() && revivify.spell_effect.is_some(),
        "CR 706.3b: the roll, its printed modifier and the table are one spell ability"
    );
    let mut game = new_game();
    fill_library(&mut game, alice(), 5);
    let source = game.create_object_from_definition(&investigator, alice(), Zone::Battlefield);
    game.force_next_die_roll(4);
    let mut dm = Scripted::yes();
    resolve_ability(&mut game, source, alice(), activated_effects(&investigator, 0), vec![], &mut dm);
    assert_eq!(hand_len(&game, alice()), 1, "CR 706.3b: the 1—9 row of the same ability draws one card");
}

const DELINA: &str = "Mana cost: {3}{R}\nType: Legendary Creature — Elf Shaman\nPower/Toughness: 3/2\nWhenever Delina attacks, choose target creature you control, then roll a d20.\n1—14 | Create a tapped and attacking token that's a copy of that creature, except it's not legendary and it has \"At end of combat, exile this token.\"\n15—20 | Create one of those tokens. You may roll again.";

#[test]
fn cr_706_3c_roll_again_uses_the_same_kind_of_die() {
    let delina = compile("Delina, Wild Mage", DELINA);
    let mut game = new_game();
    let wizard = game.create_object_from_definition(&delina, alice(), Zone::Battlefield);
    let bear_id = game.create_object_from_definition(&bear("Copied Bear"), alice(), Zone::Battlefield);
    // 16 -> token, roll again; 20 is only reachable on a d20 -> token, roll again; 3 -> token, stop.
    game.force_next_die_roll(16);
    game.force_next_die_roll(20);
    game.force_next_die_roll(3);
    let mut dm = Scripted::yes();
    let event = ironsmith::TriggerEvent::new_with_provenance(
        ironsmith::events::combat::CreatureAttackedEvent::new(
            wizard,
            ironsmith::AttackEventTarget::Player(bob()),
        ),
        Default::default(),
    );
    game.stack.push(
        StackEntry::ability(wizard, alice(), triggered_effects(&delina, 0))
            .with_targets(vec![ironsmith::Target::Object(bear_id)])
            .with_triggering_event(event),
    );
    resolve_top(&mut game, &mut dm);
    let copies = game
        .battlefield
        .iter()
        .filter(|id| **id != bear_id && game.object(**id).is_some_and(|object| object.name == "Copied Bear"))
        .count();
    assert_eq!(
        (die_rolls(&game, alice()), copies),
        (3, 3),
        "CR 706.3c: each \"roll again\" rolls another d20 (a forced 20 stays 20 only on a d20)"
    );
}

#[test]
fn cr_706_6_ignored_lowest_roll_never_happened_and_ties_are_chosen() {
    let pixie = compile(
        "Pixie Guide",
        "Mana cost: {1}{U}\nType: Creature — Faerie\nPower/Toughness: 1/3\nFlying\nGrant an Advantage — If you would roll one or more dice, instead roll that many dice plus one and ignore the lowest roll.",
    );
    let mut game = new_game();
    let guide = game.create_object_from_definition(&pixie, alice(), Zone::Battlefield);
    game.force_next_die_roll(5);
    game.force_next_die_roll(12);
    let mut dm = Scripted::yes();
    let mut ctx = EffectContext::new_default(guide, alice()).with_decision_maker(&mut dm);
    let outcome = ironsmith::effects::execute_effect(&mut game, &Effect::roll_die(20, PlayerFilter::You), &mut ctx)
        .expect("roll resolves");
    assert_eq!(outcome.as_count(), Some(12), "CR 706.6: the lower roll is ignored");
    assert_eq!(die_rolls(&game, alice()), 1, "CR 706.6: the ignored roll is considered never to have happened");
    assert!(
        !game.turn_store.turn_history.player_rolled_result_this_turn(alice(), 5),
        "CR 706.6: no record of the ignored 5 exists"
    );

    game.force_next_die_roll(7);
    game.force_next_die_roll(7);
    let mut dm = Scripted::yes();
    let mut ctx = EffectContext::new_default(guide, alice()).with_decision_maker(&mut dm);
    let outcome = ironsmith::effects::execute_effect(&mut game, &Effect::roll_die(20, PlayerFilter::You), &mut ctx)
        .expect("roll resolves");
    assert_eq!(outcome.as_count(), Some(7));
    assert!(
        dm.option_prompts.iter().any(|(_, options)| options.len() == 2
            && options.iter().all(|option| option.contains("Ignore die"))),
        "CR 706.6: with tied lowest rolls the player chooses which to ignore: {:?}",
        dm.option_prompts
    );
}

const CENTAUR: &str = "Mana cost: {3}{G}{G}\nType: Creature — Centaur Performer\nPower/Toughness: 3/3\nWhen this creature enters, roll five six-sided dice and store those results on it.\nAt the beginning of combat on your turn, you may reroll any number of this creature's stored results.\nThis creature gets +X/+X, where X is the greatest number of stored results on it of the same value.";

#[test]
fn cr_706_8_706_8a_b_stored_results_note_die_kind_value_and_rerolls_replace_them() {
    let centaur = compile("Centaur of Attention", CENTAUR);
    let mut game = new_game();
    let performer = game.create_object_from_definition(&centaur, alice(), Zone::Battlefield);
    for result in [1, 1, 2, 3, 3] {
        game.force_next_die_roll(result);
    }
    resolve_ability(&mut game, performer, alice(), triggered_effects(&centaur, 0), vec![], &mut Scripted::yes());
    assert_eq!(
        game.stored_die_results(performer).map(|(sides, results)| (sides, results.to_vec())),
        Some((6, vec![1, 1, 2, 3, 3])),
        "CR 706.8a: storing notes the kind of die and each result"
    );
    assert_eq!(game.calculated_power(performer), Some(5), "CR 706.8: X counts stored values (two 1s / two 3s)");
    game.force_next_die_roll(1);
    let mut dm = Scripted::preferring(&["Result 3: 2"]);
    resolve_ability(&mut game, performer, alice(), triggered_effects(&centaur, 1), vec![], &mut dm);
    assert_eq!(
        game.stored_die_results(performer).map(|(sides, results)| (sides, results.to_vec())),
        Some((6, vec![1, 1, 1, 3, 3])),
        "CR 706.8b: the rerolled result stops being stored and the new d6 result is stored"
    );
    assert_eq!(
        (game.calculated_power(performer), game.calculated_toughness(performer)),
        (Some(6), Some(6)),
        "CR 706.8b: three equal stored values give +3/+3"
    );
}

#[test]
fn cr_706_8c_stored_results_are_linked_to_the_storing_permanent() {
    let centaur = compile("Centaur of Attention", CENTAUR);
    let mut game = new_game();
    let performer = game.create_object_from_definition(&centaur, alice(), Zone::Battlefield);
    for result in [4, 4, 4, 4, 2] {
        game.force_next_die_roll(result);
    }
    resolve_ability(&mut game, performer, alice(), triggered_effects(&centaur, 0), vec![], &mut Scripted::yes());
    assert_eq!(game.calculated_power(performer), Some(7));
    // A second permanent with the same abilities has its own, separate storage.
    let other = game.create_object_from_definition(&centaur, alice(), Zone::Battlefield);
    assert_eq!(
        (game.calculated_power(other), game.calculated_toughness(other)),
        (Some(3), Some(3)),
        "CR 706.8c: the static ability refers only to results stored by its own linked ability"
    );
    for result in [6, 5, 4, 3, 2] {
        game.force_next_die_roll(result);
    }
    resolve_ability(&mut game, other, alice(), triggered_effects(&centaur, 0), vec![], &mut Scripted::yes());
    assert_eq!(game.calculated_power(other), Some(4));
    assert_eq!(game.calculated_power(performer), Some(7), "CR 706.8c: the first Centaur keeps its own results");
}

#[test]
fn cr_706_2a_paid_reroll_modifier_lets_the_roller_activate_mana_abilities() {
    let mut game = new_game();
    let mut definition = plain("Reroll Source", CardType::Enchantment);
    definition.abilities.push(Ability::static_ability(
        ironsmith::StaticAbility::die_roll_reroll(
            PlayerFilter::You,
            ironsmith::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(1)]),
            true,
            "Once each turn, you may pay {1} to reroll one or more dice you rolled.",
        ),
    ));
    let source = game.create_object_from_definition(&definition, alice(), Zone::Battlefield);
    let land = game.create_object_from_definition(
        &compile("Wastes", "Type: Basic Land\n{T}: Add {C}."),
        alice(),
        Zone::Battlefield,
    );
    assert_eq!(game.player(alice()).unwrap().mana_pool.total(), 0);
    game.force_next_die_roll(2);
    game.force_next_die_roll(5);
    let mut dm = Scripted::yes();
    let mut ctx = EffectContext::new_default(source, alice()).with_decision_maker(&mut dm);
    let outcome = ironsmith::effects::execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
        .expect("roll resolves");
    assert_eq!(
        outcome.as_count(),
        Some(5),
        "CR 706.2a: the roller may activate a mana ability to pay the optional modifier's mana cost"
    );
    assert!(game.is_tapped(land), "CR 706.2a: the land's mana ability paid {{1}}");
}


// ---------------------------------------------------------------------------
// Shared helpers for casting/priority-driven fixtures
// ---------------------------------------------------------------------------

/// Apply one priority action and answer every non-priority decision with `dm`.
fn perform(
    game: &mut GameState,
    action: ironsmith::LegalAction,
    dm: &mut Scripted,
) -> (ironsmith::TriggerQueue, ironsmith::decision::GameProgress) {
    use ironsmith::decision::GameProgress;
    use ironsmith::decisions::context::DecisionContext;
    let mut state = ironsmith::PriorityLoopState::new(game.players_in_game());
    let mut queue = ironsmith::TriggerQueue::new();
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &ironsmith::PriorityResponse::PriorityAction(action),
        dm,
    )
    .expect("priority action starts");
    for _ in 0..64 {
        match &progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::Priority(_)) => break,
            GameProgress::NeedsDecisionCtx(context) => {
                let context = context.clone();
                progress = ironsmith::game_loop::apply_decision_context_with_dm(
                    game, &mut queue, &mut state, &context, dm,
                )
                .expect("decision applies");
            }
            _ => break,
        }
    }
    (queue, progress)
}

/// Cast `card` (already in its zone) with the first legal CastSpell action
/// whose casting method satisfies `method`, then answer decisions until the
/// spell is on the stack. Returns the trigger queue for later stacking.
fn cast_with(
    game: &mut GameState,
    player: PlayerId,
    card: ObjectId,
    method: impl Fn(&ironsmith::alternative_cast::CastingMethod) -> bool,
    dm: &mut Scripted,
) -> ironsmith::TriggerQueue {
    let action = legal_actions(game, player)
        .into_iter()
        .find(|action| matches!(action, ironsmith::LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card && method(casting_method)))
        .unwrap_or_else(|| panic!("no matching cast action among {:?}", legal_actions(game, player)));
    perform(game, action, dm).0
}

fn legal_actions(game: &GameState, player: PlayerId) -> Vec<ironsmith::LegalAction> {
    ironsmith::compute_legal_actions(game, player).expect("legal actions")
}

fn add_mana(game: &mut GameState, player: PlayerId, symbols: &[ironsmith::ManaSymbol]) {
    for symbol in symbols {
        game.player_mut(player).unwrap().mana_pool.add(*symbol, 1);
    }
}

fn link_faces(game: &mut GameState, front: &mut CardDefinition, back: &mut CardDefinition, transforming: bool) {
    use ironsmith::card::LinkedFaceLayout;
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.clone());
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some(front.card.name.clone());
    front.card.linked_face_layout = LinkedFaceLayout::TransformLike;
    back.card.linked_face_layout = LinkedFaceLayout::TransformLike;
    front.card.transforming_dfc = transforming;
    back.card.transforming_dfc = transforming;
    game.register_linked_face_definition(front);
    game.register_linked_face_definition(back);
}

fn named_on_battlefield(game: &GameState, name: &str) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| game.current_name(*id).as_deref() == Some(name))
        .collect()
}

fn stack_triggers(game: &mut GameState, dm: &mut Scripted) {
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).expect("triggers stack");
}

// ---------------------------------------------------------------------------
// CR 707: copying objects
// ---------------------------------------------------------------------------

const METAMORPHIC: &str = "Mana cost: {1}{U}\nType: Enchantment — Aura\nEnchant creature\nAs this Aura enters, choose a creature.\nEnchanted creature is a copy of the chosen creature.";

#[test]
fn cr_707_2c_static_copy_effect_locks_copiable_values_when_it_first_applies() {
    let alteration = compile("Metamorphic Alteration", METAMORPHIC);
    let mut game = new_game();
    let recipient = game.create_object_from_definition(&bear("Recipient Bear"), alice(), Zone::Battlefield);
    let wurm = game.create_object_from_definition(
        &compile("Craw Wurm", "Mana cost: {4}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 6/4"),
        bob(),
        Zone::Battlefield,
    );
    let angel = game.create_object_from_definition(
        &compile("Serra Angel", "Mana cost: {3}{W}{W}\nType: Creature — Angel\nPower/Toughness: 4/4\nFlying\nVigilance"),
        bob(),
        Zone::Battlefield,
    );
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(wurm);
    resolve_spell(&mut game, &alteration, alice(), vec![ironsmith::Target::Object(recipient)], &mut dm);
    assert_eq!(
        game.current_name(recipient).as_deref(),
        Some("Craw Wurm"),
        "CR 707.2c fixture: the enchanted creature is a copy of the chosen Craw Wurm"
    );
    // Now the chosen creature itself becomes a copy of Serra Angel.
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(angel);
    resolve_spell(&mut game, &alteration, alice(), vec![ironsmith::Target::Object(wurm)], &mut dm);
    assert_eq!(game.current_name(wurm).as_deref(), Some("Serra Angel"));
    assert_eq!(
        (
            game.current_name(recipient).as_deref().map(str::to_string),
            game.calculated_power(recipient),
            game.calculated_toughness(recipient)
        ),
        (Some("Craw Wurm".to_string()), Some(6), Some(4)),
        "CR 707.2c: the static copy effect's copiable values were fixed when it first applied"
    );
}

const GOYF: &str = "Mana cost: {1}{G}\nType: Creature — Lhurgoyf\nPower/Toughness: */1+*\nTarmogoyf's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1.";

#[test]
fn cr_707_9c_9d_except_7_7_copy_drops_the_power_toughness_cda() {
    let gargantuan = compile(
        "Quicksilver Gargantuan",
        "Mana cost: {5}{U}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 7/7\nYou may have this creature enter as a copy of any creature on the battlefield, except it's 7/7.",
    );
    let mut game = new_game();
    let goyf = game.create_object_from_definition(&compile("Tarmogoyf", GOYF), bob(), Zone::Battlefield);
    for card_type in [CardType::Artifact, CardType::Sorcery, CardType::Land] {
        game.create_object_from_definition(&plain("Graveyard card", card_type), bob(), Zone::Graveyard);
    }
    assert_eq!(
        (game.calculated_power(goyf), game.calculated_toughness(goyf)),
        (Some(3), Some(4)),
        "fixture: Tarmogoyf's CDA counts three graveyard card types"
    );
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(goyf);
    let spell = resolve_spell(&mut game, &gargantuan, alice(), vec![], &mut dm);
    let stable = game.object(spell).map(|object| object.stable_id);
    let copy = named_on_battlefield(&game, "Tarmogoyf")
        .into_iter()
        .find(|id| *id != goyf)
        .unwrap_or_else(|| panic!("Gargantuan should enter as a Tarmogoyf copy (spell stable {stable:?})"));
    assert_eq!(
        (game.calculated_power(copy), game.calculated_toughness(copy)),
        (Some(7), Some(7)),
        "CR 707.9c/707.9d: the copy provides 7/7 and does not copy Tarmogoyf's P/T-defining ability"
    );
}

#[test]
fn cr_707_9d_in_addition_exception_still_copies_type_defining_changeling() {
    let image = compile(
        "Phantasmal Image",
        "Mana cost: {1}{U}\nType: Creature — Illusion\nPower/Toughness: 0/0\nYou may have this creature enter as a copy of any creature on the battlefield, except it's an Illusion in addition to its other types and it has \"When this creature becomes the target of a spell or ability, sacrifice it.\"",
    );
    let mut game = new_game();
    let changeling = game.create_object_from_definition(
        &compile("Avian Changeling", "Mana cost: {2}{W}\nType: Creature — Shapeshifter\nPower/Toughness: 2/2\nChangeling\nFlying"),
        bob(),
        Zone::Battlefield,
    );
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(changeling);
    resolve_spell(&mut game, &image, alice(), vec![], &mut dm);
    let copy = named_on_battlefield(&game, "Avian Changeling")
        .into_iter()
        .find(|id| *id != changeling)
        .expect("Phantasmal Image copy");
    let subtypes = game.current_characteristics(copy).unwrap().subtypes.to_vec();
    assert!(
        subtypes.contains(&ironsmith::Subtype::Elf) && subtypes.contains(&ironsmith::Subtype::Illusion),
        "CR 707.9d: an \"in addition to its other types\" exception still copies the type-defining changeling ability: {subtypes:?}"
    );
}

const CLONE: &str = "Mana cost: {3}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 0/0\nYou may have this creature enter as a copy of any creature on the battlefield.";

#[test]
fn cr_707_9e_additional_effect_exception_is_lost_when_another_copy_effect_applies() {
    let ego = compile(
        "Altered Ego",
        "Mana cost: {X}{2}{G}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 0/0\nThis spell can't be countered.\nYou may have this creature enter as a copy of any creature on the battlefield, except it enters with X additional +1/+1 counters on it.",
    );
    // Control: copying an ordinary creature gets the X counters.
    let mut game = new_game();
    let bear_id = game.create_object_from_definition(&bear("Target Bear"), bob(), Zone::Battlefield);
    let mut dm = Scripted::yes();
    dm.number = Some(2);
    let card = game.create_object_from_definition(&ego, alice(), Zone::Hand);
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Green, ironsmith::ManaSymbol::Blue, ironsmith::ManaSymbol::Colorless, ironsmith::ManaSymbol::Colorless, ironsmith::ManaSymbol::Colorless, ironsmith::ManaSymbol::Colorless]);
    cast_with(&mut game, alice(), card, |_| true, &mut dm);
    dm.object_queue.push_back(bear_id);
    resolve_top(&mut game, &mut dm);
    let copy = named_on_battlefield(&game, "Target Bear").into_iter().find(|id| *id != bear_id).expect("ego copy");
    assert_eq!(game.counter_count(copy, ironsmith::CounterType::PlusOnePlusOne), 2, "fixture: X=2 counters");

    // CR 707.9e: copy Clone, then use Clone's replacement to copy the bear.
    let mut game = new_game();
    let bear_id = game.create_object_from_definition(&bear("Target Bear"), bob(), Zone::Battlefield);
    // Bob's anthem keeps his uncopied 0/0 Clone alive.
    game.create_object_from_definition(
        &compile("Glorious Anthem", "Mana cost: {1}{W}{W}\nType: Enchantment\nCreatures you control get +1/+1."),
        bob(),
        Zone::Battlefield,
    );
    let clone = game.create_object_from_definition(&compile("Clone", CLONE), bob(), Zone::Battlefield);
    let mut dm = Scripted::yes();
    dm.number = Some(2);
    let card = game.create_object_from_definition(&ego, alice(), Zone::Hand);
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Green, ironsmith::ManaSymbol::Blue, ironsmith::ManaSymbol::Colorless, ironsmith::ManaSymbol::Colorless, ironsmith::ManaSymbol::Colorless, ironsmith::ManaSymbol::Colorless]);
    cast_with(&mut game, alice(), card, |_| true, &mut dm);
    dm.object_queue.push_back(clone);
    dm.object_queue.push_back(bear_id);
    resolve_top(&mut game, &mut dm);
    let copy = named_on_battlefield(&game, "Target Bear").into_iter().find(|id| *id != bear_id);
    let copy = copy.unwrap_or_else(|| {
        panic!(
            "Altered Ego, as Clone, should have copied the bear: battlefield {:?}, prompts {:?}",
            game.battlefield.iter().map(|id| game.current_name(*id)).collect::<Vec<_>>(),
            dm.option_prompts
        )
    });
    assert_eq!(
        game.counter_count(copy, ironsmith::CounterType::PlusOnePlusOne),
        0,
        "CR 707.9e: Altered Ego's additional-counters exception does not happen once another copy effect applies"
    );
}

const MORITTE: &str = "Mana cost: {2}{G}{U}{U}\nType: Legendary Snow Creature — Shapeshifter\nPower/Toughness: 0/0\nChangeling\nYou may have Moritte enter as a copy of a permanent you control, except it's legendary and snow in addition to its other types and, if it's a creature, it enters with two additional +1/+1 counters on it and has changeling.";

#[test]
fn cr_707_9f_conditional_exception_checks_the_copy_without_that_exception() {
    let moritte = compile("Moritte of the Frost", MORITTE);
    let mutavault = compile(
        "Mutavault",
        "Type: Land\n{T}: Add {C}.\n{1}: This land becomes a 2/2 creature with all creature types until end of turn. It's still a land.",
    );
    // Creature original: counters and changeling apply.
    let mut game = new_game();
    let bear_id = game.create_object_from_definition(&bear("Copied Bear"), alice(), Zone::Battlefield);
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(bear_id);
    resolve_spell(&mut game, &moritte, alice(), vec![], &mut dm);
    let copy = named_on_battlefield(&game, "Copied Bear").into_iter().find(|id| *id != bear_id).expect("Moritte copy");
    assert_eq!(game.counter_count(copy, ironsmith::CounterType::PlusOnePlusOne), 2, "fixture: creature copy gets two counters");
    assert!(game.object_has_static_ability_id(copy, StaticAbilityId::Changeling));

    // Animated land: the copy would be a noncreature land, so no counters / changeling.
    let mut game = new_game();
    let vault = game.create_object_from_definition(&mutavault, alice(), Zone::Battlefield);
    resolve_ability(&mut game, vault, alice(), activated_effects(&mutavault, 1), vec![], &mut Scripted::yes());
    assert!(
        game.current_characteristics(vault).unwrap().card_types.contains(&CardType::Creature),
        "fixture: Mutavault is an animated creature land"
    );
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(vault);
    resolve_spell(&mut game, &moritte, alice(), vec![], &mut dm);
    let copy = named_on_battlefield(&game, "Mutavault").into_iter().find(|id| *id != vault).expect("Moritte land copy");
    let chars = game.current_characteristics(copy).unwrap();
    assert!(!chars.card_types.contains(&CardType::Creature));
    assert_eq!(
        (
            game.counter_count(copy, ironsmith::CounterType::PlusOnePlusOne),
            game.object_has_static_ability_id(copy, StaticAbilityId::Changeling)
        ),
        (0, false),
        "CR 707.9f: copying an animated land yields a noncreature, so the creature-only exception does not apply"
    );
}

const OLIVIA: &str = "Mana cost: {2}{B}{R}\nType: Legendary Creature — Vampire\nPower/Toughness: 3/3\nFlying\n{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.\n{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.";

#[test]
fn cr_707_11_name_references_track_the_permanent_after_it_becomes_a_copy() {
    let olivia_def = compile("Olivia Voldaren", OLIVIA);
    let cytoshape = compile(
        "Cytoshape",
        "Mana cost: {1}{G}{U}\nType: Instant\nChoose a nonlegendary creature on the battlefield. Target creature becomes a copy of that creature until end of turn.",
    );
    let mut game = new_game();
    let olivia = game.create_object_from_definition(&olivia_def, alice(), Zone::Battlefield);
    let victim = game.create_object_from_definition(&bear("Victim Bear"), bob(), Zone::Battlefield);
    let model = game.create_object_from_definition(&bear("Model Bear"), bob(), Zone::Battlefield);
    game.stack.push(
        StackEntry::ability(olivia, alice(), activated_effects(&olivia_def, 0))
            .with_targets(vec![ironsmith::Target::Object(victim)]),
    );
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(model);
    resolve_spell(&mut game, &cytoshape, alice(), vec![ironsmith::Target::Object(olivia)], &mut dm);
    assert_eq!(game.current_name(olivia).as_deref(), Some("Model Bear"), "fixture: Olivia is now a copy");
    resolve_top(&mut game, &mut Scripted::yes());
    assert_eq!(
        game.counter_count(olivia, ironsmith::CounterType::PlusOnePlusOne),
        1,
        "CR 707.11: \"Put a +1/+1 counter on Olivia Voldaren\" still means that permanent after it changed names"
    );
}

const O_RING: &str = "Mana cost: {2}{W}\nType: Enchantment\nWhen this enchantment enters, exile another target nonland permanent.\nWhen this enchantment leaves the battlefield, return the exiled card to the battlefield under its owner's control.";

#[test]
fn cr_707_7_copied_linked_abilities_are_linked_only_to_each_other() {
    let ring = compile("Oblivion Ring", O_RING);
    let mirrormade = compile(
        "Mirrormade",
        "Mana cost: {1}{U}{U}\nType: Enchantment\nYou may have this enchantment enter as a copy of any artifact or enchantment on the battlefield.",
    );
    let mut game = new_game();
    let first = game.create_object_from_definition(&bear("First Exiled"), bob(), Zone::Battlefield);
    let second = game.create_object_from_definition(&bear("Second Exiled"), bob(), Zone::Battlefield);
    let first_stable = game.object(first).unwrap().stable_id;
    let second_stable = game.object(second).unwrap().stable_id;
    let mut dm = Scripted::yes();
    resolve_spell(&mut game, &ring, alice(), vec![], &mut dm);
    let original = named_on_battlefield(&game, "Oblivion Ring")[0];
    let mut dm = Scripted::yes();
    dm.target_pick = vec![ironsmith::Target::Object(first)];
    stack_triggers(&mut game, &mut dm);
    resolve_top(&mut game, &mut dm);
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(original);
    resolve_spell(&mut game, &mirrormade, alice(), vec![], &mut dm);
    let copy = named_on_battlefield(&game, "Oblivion Ring")
        .into_iter()
        .find(|id| *id != original)
        .expect("Mirrormade copy of Oblivion Ring");
    let mut dm = Scripted::yes();
    dm.target_pick = vec![ironsmith::Target::Object(second)];
    stack_triggers(&mut game, &mut dm);
    resolve_top(&mut game, &mut dm);
    let zone_of = |game: &GameState, stable| game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone;
    assert_eq!((zone_of(&game, first_stable), zone_of(&game, second_stable)), (Zone::Exile, Zone::Exile));
    let killer = game.create_object_from_definition(&plain("Remover", CardType::Artifact), alice(), Zone::Battlefield);
    resolve_ability(
        &mut game,
        killer,
        alice(),
        vec![Effect::new(ironsmith::effects::MoveToZoneEffect::new(
            ironsmith::ChooseSpec::SpecificObject(copy),
            Zone::Graveyard,
            false,
        ))]
        .into(),
        vec![],
        &mut Scripted::yes(),
    );
    let mut dm = Scripted::yes();
    stack_triggers(&mut game, &mut dm);
    resolve_top(&mut game, &mut dm);
    assert_eq!(
        (zone_of(&game, first_stable), zone_of(&game, second_stable)),
        (Zone::Exile, Zone::Battlefield),
        "CR 707.7: the copy's leaves ability returns only the card its own linked enters ability exiled"
    );
}

const DELVER_FRONT: &str = "Mana cost: {U}\nType: Creature — Human Wizard\nPower/Toughness: 1/1\nAt the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.";
const DELVER_BACK: &str = "Type: Creature — Human Insect\nColor indicator: Blue\nPower/Toughness: 3/2\nFlying";

#[test]
fn cr_707_10g_copy_of_a_double_faced_permanent_spell_becomes_a_double_faced_token() {
    let mut front = compile("Delver of Secrets", DELVER_FRONT);
    let mut back = compile("Insectile Aberration", DELVER_BACK);
    let engine = compile(
        "Lithoform Engine",
        "Mana cost: {4}\nType: Legendary Artifact\n{2}, {T}: Copy target activated or triggered ability you control. You may choose new targets for the copy.\n{3}, {T}: Copy target instant or sorcery spell you control. You may choose new targets for the copy.\n{4}, {T}: Copy target permanent spell you control. (The copy becomes a token.)",
    );
    let mut game = new_game();
    link_faces(&mut game, &mut front, &mut back, true);
    let engine_id = game.create_object_from_definition(&engine, alice(), Zone::Battlefield);
    let spell = game.create_object_from_definition(&front, alice(), Zone::Stack);
    game.stack.push(StackEntry::new(spell, alice()));
    resolve_ability(
        &mut game,
        engine_id,
        alice(),
        activated_effects(&engine, 2),
        vec![ironsmith::Target::Object(spell)],
        &mut Scripted::yes(),
    );
    assert_eq!(game.stack.len(), 2, "fixture: the permanent spell copy is on the stack");
    resolve_top(&mut game, &mut Scripted::yes());
    let tokens: Vec<ObjectId> = named_on_battlefield(&game, "Delver of Secrets");
    assert_eq!(tokens.len(), 1, "the copy resolved into a token");
    let token = tokens[0];
    assert!(
        game.can_transform(token),
        "CR 707.10g: the token created from a copy of a double-faced permanent spell is double-faced"
    );
    let source = game.create_object_from_definition(&plain("Transformer", CardType::Artifact), alice(), Zone::Battlefield);
    resolve_ability(
        &mut game,
        source,
        alice(),
        vec![Effect::new(ironsmith::effects::TransformEffect::new(ironsmith::ChooseSpec::SpecificObject(token)))].into(),
        vec![],
        &mut Scripted::yes(),
    );
    assert_eq!(
        game.current_name(token).as_deref(),
        Some("Insectile Aberration"),
        "CR 707.10g: the token's back face uses the copied spell's back-face values"
    );
}

const MIZZIX: &str = "Mana cost: {3}{R}\nType: Sorcery\nExile target card that's an instant or sorcery from your graveyard. For each card exiled this way, copy it, and you may cast the copy without paying its mana cost. Exile Mizzix's Mastery.\nOverload {5}{R}{R}{R} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")";
const DIVINATION: &str = "Mana cost: {2}{U}\nType: Sorcery\nDraw two cards.";

#[test]
fn cr_707_12_cast_copy_is_created_in_the_cards_zone_cast_and_resolves_as_a_spell() {
    let mizzix = compile("Mizzix's Mastery", MIZZIX);
    let mut game = new_game();
    fill_library(&mut game, alice(), 5);
    let divination = game.create_object_from_definition(&compile("Divination", DIVINATION), alice(), Zone::Graveyard);
    let cast_before = game.turn_store.turn_history.spells_cast_by_player(alice());
    let mut dm = Scripted::yes();
    resolve_spell(&mut game, &mizzix, alice(), vec![ironsmith::Target::Object(divination)], &mut dm);
    assert_eq!(game.stack.len(), 1, "CR 707.12: the cast copy is a spell on the stack");
    let copy = game.stack[0].object_id;
    assert_eq!(game.current_name(copy).as_deref(), Some("Divination"));
    assert_eq!(
        game.turn_store.turn_history.spells_cast_by_player(alice()) - cast_before,
        1,
        "CR 707.12: casting the copy counts as casting a spell"
    );
    resolve_top(&mut game, &mut Scripted::yes());
    assert_eq!(hand_len(&game, alice()), 2, "CR 707.12: the copy resolves like any spell");
    assert!(game.object(copy).is_none(), "the resolved copy ceases to exist");
}

#[test]
fn cr_707_12a_each_created_copy_is_individually_cast_or_not() {
    let mizzix = compile("Mizzix's Mastery", MIZZIX);
    let mut game = new_game();
    fill_library(&mut game, alice(), 5);
    let first = game.create_object_from_definition(&compile("Divination", DIVINATION), alice(), Zone::Graveyard);
    let second = game.create_object_from_definition(&compile("Divination", DIVINATION), alice(), Zone::Graveyard);
    let _ = (first, second);
    let card = game.create_object_from_definition(&mizzix, alice(), Zone::Hand);
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Red; 8]);
    let mut dm = Scripted::yes();
    cast_with(&mut game, alice(), card, |method| matches!(method, ironsmith::alternative_cast::CastingMethod::Alternative(_)), &mut dm);
    assert_eq!(game.stack.len(), 1, "fixture: overloaded Mizzix's Mastery is on the stack");
    let mut dm = Scripted::yes();
    dm.bool_queue = [true, false].into_iter().collect();
    resolve_top(&mut game, &mut dm);
    assert_eq!(
        game.stack.len(),
        1,
        "CR 707.12a: the player chose to cast one copy and not the other ({:?})",
        dm.boolean_prompts
    );
}

#[test]
fn cr_707_13_garth_creates_the_named_card_from_the_oracle_reference_and_casts_it() {
    let garth = compile(
        "Garth One-Eye",
        "Mana cost: {W}{U}{B}{R}{G}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 5/5\n{T}: Choose a card name that hasn't been chosen from among Disenchant, Braingeyser, Terror, Shivan Dragon, Regrowth, and Black Lotus. Create a copy of the card with the chosen name. You may cast the copy. (You still pay its costs.)",
    );
    let regrowth = compile("Regrowth", "Mana cost: {1}{G}\nType: Sorcery\nReturn target card from your graveyard to your hand.");
    let mut game = new_game();
    game.register_linked_face_definition(&regrowth);
    // The engine notes the chosen name in lowercase; in production the card
    // registry resolves it loosely. Register the same Oracle definition under
    // that key so the fixture models the Oracle card reference.
    let mut alias = regrowth.clone();
    alias.card.name = "regrowth".into();
    game.register_linked_face_definition(&alias);
    let wizard = game.create_object_from_definition(&garth, alice(), Zone::Battlefield);
    let buried = game.create_object_from_definition(&bear("Buried Bear"), alice(), Zone::Graveyard);
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Green, ironsmith::ManaSymbol::Green]);
    let mut dm = Scripted::preferring(&["Regrowth"]);
    dm.text = Some("Regrowth".into());
    dm.target_pick = vec![ironsmith::Target::Object(buried)];
    resolve_ability(&mut game, wizard, alice(), activated_effects(&garth, 0), vec![], &mut dm);
    let top = game.stack.last().map(|entry| entry.object_id);
    let top = top.unwrap_or_else(|| panic!("CR 707.13: the Regrowth copy should be cast (prompts {:?} / {:?})", dm.option_prompts, dm.boolean_prompts));
    let chars = game.current_characteristics(top).unwrap();
    assert_eq!(chars.name.to_lowercase(), "regrowth", "CR 707.13: the copy uses the Oracle characteristics of the named card");
    assert!(chars.card_types.contains(&CardType::Sorcery));
    assert_eq!(game.player(alice()).unwrap().mana_pool.total(), 0, "Garth: you still pay the copy's costs");
}


// ---------------------------------------------------------------------------
// CR 708: face-down spells and permanents
// ---------------------------------------------------------------------------

const AINOK: &str = "Mana cost: {5}{R}\nType: Creature — Dog Scout\nPower/Toughness: 3/3\nFirst strike\nMorph {4}{R}";
const HYDRA: &str = "Mana cost: {X}{G}{G}\nType: Creature — Snake Hydra\nPower/Toughness: 0/0\nThis creature enters with X +1/+1 counters on it.\nWhen this creature dies, create a 1/1 green Snake creature token for each +1/+1 counter on it.\nMorph {3}{G}{G}\nAs this creature is turned face up, put five +1/+1 counters on it.";

fn is_face_down_cast(method: &ironsmith::alternative_cast::CastingMethod) -> bool {
    use ironsmith::alternative_cast::CastingMethod;
    matches!(method, CastingMethod::FaceDown | CastingMethod::FaceDownPlayFrom { .. })
}

/// Cast a morph card face down for {3} and return the stack object.
fn cast_face_down(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let card = game.create_object_from_definition(definition, alice(), Zone::Hand);
    add_mana(game, alice(), &[ironsmith::ManaSymbol::Colorless; 3]);
    cast_with(game, alice(), card, is_face_down_cast, &mut Scripted::yes());
    game.stack.last().expect("face-down spell on the stack").object_id
}

#[test]
fn cr_708_4_face_down_spell_has_only_face_down_characteristics_and_becomes_a_face_down_permanent() {
    let mut game = new_game();
    let spell = cast_face_down(&mut game, &compile("Ainok Tracker", AINOK));
    assert!(game.is_face_down(spell), "CR 708.4: the spell is turned face down before it is put onto the stack");
    let chars = game.current_characteristics(spell).unwrap();
    assert_eq!(
        (
            chars.name.is_empty() || chars.name.as_ref() == "Face-down creature",
            chars.colors,
            chars.card_types.to_vec(),
            chars.power,
            chars.toughness,
            chars.mana_cost.is_none()
        ),
        (true, ironsmith::ColorSet::COLORLESS, vec![CardType::Creature], Some(2), Some(2), true),
        "CR 708.4: effects see only the face-down spell's characteristics (name {:?})",
        chars.name
    );
    let stable = game.object(spell).unwrap().stable_id;
    resolve_top(&mut game, &mut Scripted::yes());
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
    assert!(game.is_face_down(permanent), "CR 708.4: the permanent the spell becomes is face down");
}

#[test]
fn cr_708_5_only_the_controller_may_look_at_face_down_spells_and_permanents() {
    let mut game = new_game();
    let spell = cast_face_down(&mut game, &compile("Ainok Tracker", AINOK));
    assert!(
        game.can_player_look_at_face_down_permanent(spell, alice()) && !game.can_player_look_at_face_down_permanent(spell, bob()),
        "CR 708.5: the controller may look at its face-down spell; the opponent may not"
    );
    let stable = game.object(spell).unwrap().stable_id;
    resolve_top(&mut game, &mut Scripted::yes());
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert!(
        game.can_player_look_at_face_down_permanent(permanent, alice())
            && !game.can_player_look_at_face_down_permanent(permanent, bob()),
        "CR 708.5: only the controller may look at its face-down permanent"
    );
}

#[test]
fn cr_708_7_morph_permits_turning_the_permanent_face_up_but_not_the_spell() {
    let mut game = new_game();
    let spell = cast_face_down(&mut game, &compile("Ainok Tracker", AINOK));
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Red; 5]);
    assert!(
        !legal_actions(&game, alice())
            .iter()
            .any(|action| matches!(action, ironsmith::LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == spell)),
        "CR 708.7: a face-down spell normally can't be turned face up"
    );
    let stable = game.object(spell).unwrap().stable_id;
    resolve_top(&mut game, &mut Scripted::yes());
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    let action = legal_actions(&game, alice())
        .into_iter()
        .find(|action| matches!(action, ironsmith::LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == permanent))
        .expect("CR 708.7: morph lets the controller turn the permanent face up");
    perform(&mut game, action, &mut Scripted::yes());
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert!(!game.is_face_down(permanent));
    assert_eq!(game.current_name(permanent).as_deref(), Some("Ainok Tracker"));
}

#[test]
fn cr_708_11_as_turned_face_up_ability_applies_while_turning_face_up() {
    let mut game = new_game();
    let spell = cast_face_down(&mut game, &compile("Hooded Hydra", HYDRA));
    let stable = game.object(spell).unwrap().stable_id;
    resolve_top(&mut game, &mut Scripted::yes());
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Green; 5]);
    let action = legal_actions(&game, alice())
        .into_iter()
        .find(|action| matches!(action, ironsmith::LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == permanent))
        .expect("Hooded Hydra can be turned face up for its morph cost");
    perform(&mut game, action, &mut Scripted::yes());
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        (
            game.object(permanent).unwrap().zone,
            game.counter_count(permanent, ironsmith::CounterType::PlusOnePlusOne),
            game.calculated_power(permanent)
        ),
        (Zone::Battlefield, 5, Some(5)),
        "CR 708.11: the counters are placed while it turns face up, so the 0/0 never sees state-based actions"
    );
}

// ---------------------------------------------------------------------------
// CR 712: double-faced cards
// ---------------------------------------------------------------------------

#[test]
fn cr_712_8b_meld_card_on_the_stack_has_only_front_face_characteristics() {
    let bruna = compile(
        "Bruna, the Fading Light",
        "Mana cost: {5}{W}{W}\nType: Legendary Creature — Angel Horror\nPower/Toughness: 5/7\nWhen you cast this spell, you may return target Angel or Human creature card from your graveyard to the battlefield.\nFlying, vigilance\n(Melds with Gisela, the Broken Blade.)",
    );
    let mut game = new_game();
    let spell = game.create_object_from_definition(&bruna, alice(), Zone::Stack);
    game.stack.push(StackEntry::new(spell, alice()));
    let chars = game.current_characteristics(spell).unwrap();
    assert_eq!(
        (chars.name.to_string(), chars.power, chars.toughness, chars.mana_cost.as_ref().map(|cost| cost.mana_value())),
        ("Bruna, the Fading Light".to_string(), Some(5), Some(7), Some(7)),
        "CR 712.8b: a meld card on the stack has only its front face's characteristics"
    );
}

const BEGGAR: &str = "Mana cost: {1}{W}\nType: Creature — Human Peasant\nPower/Toughness: 0/4\nDisturb {4}{W}{W}";
const SOUL: &str = "Type: Creature — Spirit\nColor indicator: White\nPower/Toughness: 4/4\nFlying, vigilance\nIf Generous Soul would be put into a graveyard from anywhere, exile it instead.";

#[test]
fn cr_712_11a_11d_casting_transformed_puts_the_back_face_up_using_a_front_face_permission() {
    let mut front = compile("Beloved Beggar", BEGGAR);
    let mut back = compile("Generous Soul", SOUL);
    let mut game = new_game();
    link_faces(&mut game, &mut front, &mut back, true);
    let card = game.create_object_from_definition(&front, alice(), Zone::Graveyard);
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::White; 6]);
    assert!(
        legal_actions(&game, alice()).iter().any(|action| matches!(action, ironsmith::LegalAction::CastSpell { spell_id, .. } if *spell_id == card)),
        "CR 712.11d: the front face's disturb ability is considered when deciding whether it can be cast transformed"
    );
    cast_with(&mut game, alice(), card, |method| !matches!(method, ironsmith::alternative_cast::CastingMethod::Normal), &mut Scripted::yes());
    let spell = game.stack.last().expect("disturbed spell").object_id;
    let chars = game.current_characteristics(spell).unwrap();
    assert_eq!(
        (chars.name.to_string(), chars.power, chars.toughness),
        ("Generous Soul".to_string(), Some(4), Some(4)),
        "CR 712.11a: a card cast transformed is put on the stack with its back face up"
    );
}

const VALKI: &str = "Mana cost: {1}{B}\nType: Legendary Creature — God\nPower/Toughness: 2/1\nWhen Valki enters, each opponent reveals their hand. For each opponent, exile a creature card they revealed this way until Valki leaves the battlefield.\n{X}: Choose a creature card exiled with Valki with mana value X. Valki becomes a copy of that card.";
const TIBALT: &str = "Mana cost: {5}{B}{R}\nType: Legendary Planeswalker — Tibalt\nLoyalty: 5\nAs Tibalt enters, you get an emblem with \"You may play cards exiled with Tibalt, Cosmic Impostor, and you may spend mana as though it were mana of any color to cast those spells.\"\n+2: Exile the top card of each player's library.\n−3: Exile target artifact or creature.\n−8: Exile all graveyards. Add {R}{R}{R}.";

#[test]
fn cr_712_11b_11c_modal_dfc_face_is_chosen_before_casting_and_only_that_face_is_evaluated() {
    let mut front = compile("Valki, God of Lies", VALKI);
    let mut back = compile("Tibalt, Cosmic Impostor", TIBALT);
    let mut game = new_game();
    link_faces(&mut game, &mut front, &mut back, false);
    let card = game.create_object_from_definition(&front, alice(), Zone::Hand);
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Black, ironsmith::ManaSymbol::Red]);
    let casts = |game: &GameState| -> Vec<ironsmith::alternative_cast::CastingMethod> {
        legal_actions(game, alice())
            .into_iter()
            .filter_map(|action| match action {
                ironsmith::LegalAction::CastSpell { spell_id, casting_method, .. } if spell_id == card => Some(casting_method),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        casts(&game),
        vec![ironsmith::alternative_cast::CastingMethod::Normal],
        "CR 712.11c: with two mana only the two-mana face is castable; the seven-mana face is evaluated alone"
    );
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Colorless; 5]);
    let back_method = casts(&game)
        .into_iter()
        .find(|method| *method != ironsmith::alternative_cast::CastingMethod::Normal)
        .expect("CR 712.11b: with seven mana the back face can be chosen");
    cast_with(&mut game, alice(), card, |method| *method == back_method, &mut Scripted::yes());
    let spell = game.stack.last().expect("Tibalt on the stack").object_id;
    let chars = game.current_characteristics(spell).unwrap();
    assert!(
        chars.name.as_ref() == "Tibalt, Cosmic Impostor" && chars.card_types.contains(&CardType::Planeswalker),
        "CR 712.11b: the chosen face is the face put onto the stack: {:?}",
        chars.name
    );
}

#[test]
fn cr_712_19_a_card_name_choice_may_name_either_face_or_a_melded_back_face() {
    let needle = compile(
        "Pithing Needle",
        "Mana cost: {1}\nType: Artifact\nAs this artifact enters, choose a card name.\nActivated abilities of sources with the chosen name can't be activated unless they're mana abilities.",
    );
    for name in ["Insectile Aberration", "Delver of Secrets", "Brisela, Voice of Nightmares"] {
        let mut front = compile("Delver of Secrets", DELVER_FRONT);
        let mut back = compile("Insectile Aberration", DELVER_BACK);
        let mut game = new_game();
        link_faces(&mut game, &mut front, &mut back, true);
        let mut dm = Scripted::preferring(&[name]);
        dm.text = Some(name.to_string());
        let spell = resolve_spell(&mut game, &needle, alice(), vec![], &mut dm);
        let _ = spell;
        let permanent = named_on_battlefield(&game, "Pithing Needle")[0];
        assert_eq!(
            game.chosen_named_option(permanent).map(str::to_lowercase),
            Some(name.to_lowercase()),
            "CR 712.19: {name} is a legal card-name choice"
        );
    }
}

const CURSE: &str = "Mana cost: {2}{B}\nType: Enchantment — Aura Curse\nEnchant player\nAs this permanent transforms into Curse of Leeches, attach it to a player.\nAt the beginning of enchanted player's upkeep, they lose 1 life and you gain 1 life.\nDaybound";
const LURKER: &str = "Type: Creature — Leech Horror\nColor indicator: Black\nPower/Toughness: 4/4\nLifelink\nNightbound";

#[test]
fn cr_712_20_as_this_transforms_ability_applies_during_the_transformation() {
    let mut front = compile("Curse of Leeches", CURSE);
    let mut back = compile("Leeching Lurker", LURKER);
    let mut game = new_game();
    link_faces(&mut game, &mut front, &mut back, true);
    let curse = game.create_object_from_definition(&front, alice(), Zone::Battlefield);
    let stable = game.object(curse).unwrap().stable_id;
    // Daybound permanents transform only through day/night (CR 702.145).
    game.has_day_night = true;
    game.is_night = true;
    game.apply_day_nightbound_transformations();
    let current = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.current_name(current).as_deref(), Some("Leeching Lurker"), "fixture: back face up at night");
    game.is_night = false;
    game.apply_day_nightbound_transformations();
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    let current = game.find_object_by_stable_id(stable).unwrap();
    let object = game.object(current).unwrap();
    assert!(
        object.zone == Zone::Battlefield
            && matches!(object.attached_to, Some(ironsmith::object::AttachmentTarget::Player(_))),
        "CR 712.20: Curse of Leeches is attached to a player while transforming, so it is not an unattached Aura: zone {:?}, attached {:?}",
        object.zone,
        object.attached_to
    );
}

// ---------------------------------------------------------------------------
// CR 716: Class
// ---------------------------------------------------------------------------

const WIZARD_CLASS: &str = "Mana cost: {U}\nType: Enchantment — Class\n(Gain the next level as a sorcery to add its ability.)\nYou have no maximum hand size.\n{2}{U}: Level 2\nWhen this Class becomes level 2, draw two cards.\n{4}{U}: Level 3\nWhenever you draw a card, put a +1/+1 counter on target creature you control.";

#[test]
fn cr_716_3_716_2c_unbarred_ability_works_at_level_one_and_gaining_a_level_is_activating_the_bar() {
    let class = compile("Wizard Class", WIZARD_CLASS);
    let mut game = new_game();
    fill_library(&mut game, alice(), 5);
    let permanent = game.create_object_from_definition(&class, alice(), Zone::Battlefield);
    assert_eq!(game.class_level(permanent), 1);
    assert!(
        game.object_has_ability(permanent, &ironsmith::StaticAbility::no_maximum_hand_size()),
        "CR 716.3: the ability printed above the first level bar functions at all times"
    );
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Blue; 3]);
    let action = legal_actions(&game, alice())
        .into_iter()
        .find(|action| matches!(action, ironsmith::LegalAction::ActivateAbility { source, .. } if *source == permanent))
        .expect("CR 716.2c: gaining a level is activating the level-bar ability");
    perform(&mut game, action, &mut Scripted::yes());
    resolve_top(&mut game, &mut Scripted::yes());
    assert_eq!(game.class_level(permanent), 2, "CR 716.2c: the activated level bar makes the Class level 2");
    let mut dm = Scripted::yes();
    stack_triggers(&mut game, &mut dm);
    resolve_top(&mut game, &mut dm);
    assert_eq!(hand_len(&game, alice()), 2, "the level-2 trigger drew two cards");
    assert!(game.object_has_ability(permanent, &ironsmith::StaticAbility::no_maximum_hand_size()));
}

// ---------------------------------------------------------------------------
// CR 717: Attractions
// ---------------------------------------------------------------------------

fn attraction(name: &str, lights: Vec<u8>) -> CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![ironsmith::Subtype::Attraction])
        .attraction_lights(lights)
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build()
}

#[test]
fn cr_717_2a_2b_attraction_deck_construction_minimums_and_name_rules() {
    use ironsmith::game_state::AttractionDeckFormat;
    let distinct = |count: usize| (0..count).map(|i| attraction(&format!("Attraction {i}"), vec![1, 2])).collect::<Vec<_>>();
    let try_deck = |format, cards| new_game().enable_attractions(vec![(alice(), format, cards)]);
    assert!(try_deck(AttractionDeckFormat::Constructed, distinct(10)).is_ok(), "CR 717.2a: ten differently named Attractions are legal");
    assert!(try_deck(AttractionDeckFormat::Constructed, distinct(9)).is_err(), "CR 717.2a: fewer than ten is illegal in constructed");
    let mut duplicate = distinct(9);
    duplicate.push(attraction("Attraction 0", vec![3]));
    assert!(try_deck(AttractionDeckFormat::Constructed, duplicate).is_err(), "CR 717.2a: constructed names must differ");
    assert!(
        try_deck(AttractionDeckFormat::Limited, vec![attraction("Same", vec![1]), attraction("Same", vec![2]), attraction("Same", vec![3])]).is_ok(),
        "CR 717.2b: limited allows three Attractions sharing a name"
    );
    assert!(try_deck(AttractionDeckFormat::Limited, distinct(2)).is_err(), "CR 717.2b: limited needs at least three");
}

#[test]
fn cr_717_2_717_3_717_6_attraction_deck_is_in_command_opens_to_battlefield_and_returns_to_the_junkyard() {
    use ironsmith::game_state::AttractionDeckFormat;
    let mut game = new_game();
    fill_library(&mut game, alice(), 5);
    let library_before = game.player(alice()).unwrap().library.len();
    game.enable_attractions(vec![(alice(), AttractionDeckFormat::Limited, (0..3).map(|i| attraction(&format!("Ride {i}"), vec![i as u8 + 1])).collect())])
        .unwrap();
    let deck = game.attraction_deck(alice()).unwrap().to_vec();
    assert_eq!(deck.len(), 3);
    assert!(
        deck.iter().all(|id| game.object(*id).unwrap().zone == Zone::Command)
            && game.player(alice()).unwrap().library.len() == library_before,
        "CR 717.2: the Attraction deck is a supplementary deck in the command zone, not part of the library"
    );
    let source = game.create_object_from_definition(&plain("Opener", CardType::Artifact), alice(), Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice());
    ironsmith::effects::execute_effect(&mut game, &Effect::open_attraction(), &mut ctx).unwrap();
    let opened = game.face_up_attractions().to_vec();
    assert_eq!(opened.len(), 1);
    assert_eq!(game.object(opened[0]).unwrap().zone, Zone::Battlefield, "CR 717.3: opening moves the top Attraction to the battlefield");
    let stable = game.object(opened[0]).unwrap().stable_id;
    for destination in [Zone::Graveyard, Zone::Hand, Zone::Library] {
        let current = game.find_object_by_stable_id(stable).unwrap();
        if game.object(current).unwrap().zone != Zone::Battlefield {
            break;
        }
        resolve_ability(
            &mut game,
            source,
            alice(),
            vec![Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(current), destination, false))].into(),
            vec![],
            &mut Scripted::yes(),
        );
        let after = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(after).unwrap().zone,
            Zone::Command,
            "CR 717.6: an Attraction that would go to {destination:?} goes to its owner's command zone instead"
        );
        assert!(
            !game.attraction_deck(alice()).unwrap().contains(&after),
            "CR 717.6a: the junkyard is separate from the Attraction deck"
        );
    }
}

// ---------------------------------------------------------------------------
// CR 727 restarting / CR 730 merging / CR 732 loops
// ---------------------------------------------------------------------------

const KARN: &str = "Mana cost: {7}\nType: Legendary Planeswalker — Karn\nLoyalty: 6\n+4: Target player exiles a card from their hand.\n−3: Exile target permanent.\n−14: Restart the game, leaving in exile all non-Aura permanent cards exiled with Karn. Then put those cards onto the battlefield under your control.";

#[test]
fn cr_727_4_restart_finishes_before_the_first_untap_and_new_triggers_wait_for_priority() {
    let karn = compile("Karn Liberated", KARN);
    let mut game = new_game();
    for player in [alice(), bob()] {
        fill_library(&mut game, player, 12);
    }
    let walker = game.create_object_from_definition(&karn, alice(), Zone::Battlefield);
    let entrant = game.create_object_from_definition(
        &compile("Entry Healer", "Mana cost: {1}{W}\nType: Creature — Cleric\nPower/Toughness: 2/2\nWhen this creature enters, you gain 3 life."),
        bob(),
        Zone::Battlefield,
    );
    let stable = game.object(entrant).unwrap().stable_id;
    game.stack.push(StackEntry::ability(walker, alice(), activated_effects(&karn, 1)).with_ability_index(1).with_targets(vec![ironsmith::Target::Object(entrant)]));
    resolve_top(&mut game, &mut Scripted::yes());
    assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Exile);
    game.stack.push(StackEntry::ability(walker, alice(), activated_effects(&karn, 2)).with_ability_index(2));
    resolve_top(&mut game, &mut Scripted::yes());
    let waiting = game.find_object_by_stable_id(stable).unwrap();
    assert!(
        game.object(waiting).unwrap().zone == Zone::Exile
            && !game.pending_restart_battlefield_entries().is_empty()
            && game.restart_starting_procedure_pending(),
        "CR 727.4: the additional instruction waits until the restart effect finishes resolving just before the first untap step"
    );
    // The turn runner applies the deferred instruction once the starting
    // procedure is over, immediately before the first untap step.
    game.complete_restart_starting_procedure();
    game.apply_pending_restart_battlefield_entries_with(&mut Scripted::yes());
    let returned = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        (game.object(returned).unwrap().zone, game.current_controller(returned)),
        (Zone::Battlefield, Some(alice())),
        "CR 727.4: the restart effect's additional instruction is followed as it finishes resolving"
    );
    assert!(game.stack.is_empty(), "CR 727.4: no trigger is put on the stack before a player would receive priority");
    assert_eq!(game.turn.active_player, alice());
    let mut dm = Scripted::yes();
    stack_triggers(&mut game, &mut dm);
    assert_eq!(
        game.stack.len(),
        1,
        "CR 727.4: the entering creature's trigger is waiting to be put on the stack the next time a player receives priority"
    );
}

#[test]
fn cr_727_7_restart_involves_every_player_regardless_of_range_of_influence() {
    let karn = compile("Karn Liberated", KARN);
    let names: Vec<String> = ["Alice", "Bob", "Charlie", "Diana", "Eve"].iter().map(|n| n.to_string()).collect();
    let mut game = GameState::new(names, 20);
    let seats: Vec<PlayerId> = (0..5).map(PlayerId::from_index).collect();
    game.enable_limited_range_of_influence(seats.clone(), vec![1; 5]).expect("range of influence");
    for player in &seats {
        fill_library(&mut game, *player, 10);
        game.player_mut(*player).unwrap().life = 7;
    }
    let walker = game.create_object_from_definition(&karn, alice(), Zone::Battlefield);
    resolve_ability(&mut game, walker, alice(), activated_effects(&karn, 2), vec![], &mut Scripted::yes());
    for player in &seats {
        let state = game.player(*player).unwrap();
        assert_eq!(
            (state.life, state.hand.len()),
            (20, 7),
            "CR 727.7: player {player:?} (in or out of range) starts the restarted game"
        );
    }
}

const GEMRAZER: &str = "Mana cost: {3}{G}\nType: Creature — Beast\nPower/Toughness: 4/4\nMutate {1}{G}{G}\nReach, trample\nWhenever this creature mutates, destroy target artifact or enchantment an opponent controls.";

#[test]
fn cr_730_2b_merging_is_not_entering_the_battlefield() {
    let mut game = new_game();
    let warden = game.create_object_from_definition(
        &compile("Soul Warden", "Mana cost: {W}\nType: Creature — Human Cleric\nPower/Toughness: 1/1\nWhenever another creature enters, you gain 1 life."),
        alice(),
        Zone::Battlefield,
    );
    let _ = warden;
    let host = game.create_object_from_definition(&bear("Mutate Host"), alice(), Zone::Battlefield);
    let card = game.create_object_from_definition(&compile("Gemrazer", GEMRAZER), alice(), Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Green; 3]);
    let mut dm = Scripted::yes();
    dm.target_pick = vec![ironsmith::Target::Object(host)];
    cast_with(&mut game, alice(), card, |method| !matches!(method, ironsmith::alternative_cast::CastingMethod::Normal), &mut dm);
    resolve_top(&mut game, &mut dm);
    let mut dm = Scripted::yes();
    stack_triggers(&mut game, &mut dm);
    while !game.stack.is_empty() {
        resolve_top(&mut game, &mut dm);
    }
    assert!(game.object(host).is_some(), "fixture: the merged permanent is the same object as the host");
    assert_eq!(game.object(host).unwrap().zone, Zone::Battlefield);
    assert!(
        game.find_object_by_stable_id(stable).map_or(true, |id| game.object(id).unwrap().zone != Zone::Stack),
        "fixture: the mutating spell left the stack"
    );
    assert_eq!(game.current_name(host).as_deref(), Some("Gemrazer"), "fixture: Gemrazer was put on top");
    assert_eq!(
        game.player(alice()).unwrap().life,
        20,
        "CR 730.2b: the merged permanent isn't considered to have just entered, so \"another creature enters\" doesn't trigger"
    );
}

#[test]
fn cr_732_3_fragmented_loop_forces_the_active_player_to_make_a_different_choice() {
    let mut game = new_game();
    let flyer_def = compile(
        "Flight Seeker",
        "Mana cost: {1}{U}\nType: Creature — Bird\nPower/Toughness: 1/1\n{0}: This creature gains flying until end of turn.",
    );
    let grounder_def = compile(
        "Grounding Totem",
        "Mana cost: {1}\nType: Artifact\n{0}: Target creature loses flying until end of turn.",
    );
    let flyer = game.create_object_from_definition(&flyer_def, alice(), Zone::Battlefield);
    let grounder = game.create_object_from_definition(&grounder_def, bob(), Zone::Battlefield);
    let flyer_action = |game: &GameState| {
        legal_actions(game, alice())
            .into_iter()
            .find(|action| matches!(action, ironsmith::LegalAction::ActivateAbility { source, .. } if *source == flyer))
    };
    // Active player activates, it resolves; the opponent answers, it resolves.
    let action = flyer_action(&game).expect("fixture: the flying ability is activatable");
    perform(&mut game, action, &mut Scripted::yes());
    resolve_top(&mut game, &mut Scripted::yes());
    assert!(game.object_has_static_ability_id(flyer, StaticAbilityId::Flying));
    game.turn.priority_player = Some(bob());
    let action = legal_actions(&game, bob())
        .into_iter()
        .find(|action| matches!(action, ironsmith::LegalAction::ActivateAbility { source, .. } if *source == grounder))
        .expect("fixture: the opponent can answer");
    let mut dm = Scripted::yes();
    dm.target_pick = vec![ironsmith::Target::Object(flyer)];
    perform(&mut game, action, &mut dm);
    resolve_top(&mut game, &mut Scripted::yes());
    assert!(!game.object_has_static_ability_id(flyer, StaticAbilityId::Flying));
    game.turn.priority_player = Some(alice());
    assert!(
        flyer_action(&game).is_none(),
        "CR 732.3: the fragmented loop returned the game to an earlier state, so the active player must make a different game choice"
    );
}

// ---------------------------------------------------------------------------
// Additional copy / face / subgame scenarios
// ---------------------------------------------------------------------------

const SPIDER: &str = "Mana cost: {2}{U}{B}\nType: Legendary Creature — Spider Human Hero\nPower/Toughness: 4/4\nMind Swap — You may have Superior Spider-Man enter as a copy of any creature card in a graveyard, except his name is Superior Spider-Man and he's a 4/4 Spider Human Hero in addition to his other types. When you do, exile that card.";

#[test]
fn cr_707_9g_linked_when_you_do_trigger_is_lost_if_another_copy_effect_applies() {
    let spider = compile("Superior Spider-Man", SPIDER);
    // Control: copying an ordinary creature card exiles that card.
    let mut game = new_game();
    let card = game.create_object_from_definition(&bear("Graveyard Bear"), bob(), Zone::Graveyard);
    let card_stable = game.object(card).unwrap().stable_id;
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(card);
    resolve_spell(&mut game, &spider, alice(), vec![], &mut dm);
    let mut dm = Scripted::yes();
    stack_triggers(&mut game, &mut dm);
    while !game.stack.is_empty() {
        resolve_top(&mut game, &mut dm);
    }
    let zone = game.object(game.find_object_by_stable_id(card_stable).unwrap()).unwrap().zone;
    assert_eq!(zone, Zone::Exile, "fixture: the linked \"When you do\" trigger exiles the copied card");

    // CR 707.9g: copy a Clone card, then apply Clone's copy effect to a battlefield creature.
    let mut game = new_game();
    let clone_card = game.create_object_from_definition(&compile("Clone", CLONE), bob(), Zone::Graveyard);
    let clone_stable = game.object(clone_card).unwrap().stable_id;
    let model = game.create_object_from_definition(&bear("Battlefield Bear"), bob(), Zone::Battlefield);
    let mut dm = Scripted::yes();
    dm.object_queue.push_back(clone_card);
    dm.object_queue.push_back(model);
    resolve_spell(&mut game, &spider, alice(), vec![], &mut dm);
    let entry_prompts = dm.option_prompts.clone();
    let mut dm = Scripted::yes();
    dm.option_prompts = entry_prompts;
    stack_triggers(&mut game, &mut dm);
    while !game.stack.is_empty() {
        resolve_top(&mut game, &mut dm);
    }
    let copies = named_on_battlefield(&game, "Battlefield Bear");
    let zone = game.object(game.find_object_by_stable_id(clone_stable).unwrap()).unwrap().zone;
    assert_eq!(
        (copies.len(), zone),
        (2, Zone::Graveyard),
        "CR 707.9g: Spider-Man copies the Clone card, the copied Clone replacement then copies the bear, and the linked \"When you do, exile that card\" doesn't trigger (battlefield {:?}, entry prompts {:?})",
        game.battlefield.iter().map(|id| game.current_name(*id)).collect::<Vec<_>>(),
        dm.option_prompts
    );
}

#[test]
fn cr_707_10e_copy_with_a_specified_new_target_chooses_one_of_several_doubled_tokens() {
    let frontline = compile(
        "Frontline Heroism",
        "Mana cost: {2}{R}\nType: Enchantment\nWhen this enchantment enters, create a 1/1 red Soldier creature token with haste.\nWhenever you cast a spell that targets only a single creature you control, create a 1/1 red Soldier creature token with haste, then copy that spell. The copy targets that token.",
    );
    let procession = compile(
        "Anointed Procession",
        "Mana cost: {3}{W}\nType: Enchantment\nIf an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.",
    );
    let growth = compile("Giant Growth", "Mana cost: {G}\nType: Instant\nTarget creature gets +3/+3 until end of turn.");
    let mut game = new_game();
    game.create_object_from_definition(&frontline, alice(), Zone::Battlefield);
    game.create_object_from_definition(&procession, alice(), Zone::Battlefield);
    let host = game.create_object_from_definition(&bear("Heroic Bear"), alice(), Zone::Battlefield);
    let card = game.create_object_from_definition(&growth, alice(), Zone::Hand);
    add_mana(&mut game, alice(), &[ironsmith::ManaSymbol::Green]);
    let mut dm = Scripted::yes();
    dm.target_pick = vec![ironsmith::Target::Object(host)];
    let mut queue = cast_with(&mut game, alice(), card, |_| true, &mut dm);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    stack_triggers(&mut game, &mut dm);
    assert_eq!(game.stack.len(), 2, "fixture: Frontline Heroism triggered on the spell");
    resolve_top(&mut game, &mut dm);
    let tokens: Vec<ObjectId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| matches!(game.object(*id).unwrap().kind, ironsmith::ObjectKind::Token))
        .collect();
    assert_eq!(tokens.len(), 2, "fixture: Anointed Procession doubled the Soldier token");
    let copies: Vec<&StackEntry> = game.stack.iter().filter(|entry| entry.object_id != card && game.object(entry.object_id).is_some_and(|object| object.name == "Giant Growth")).collect();
    let copy_targets: Vec<Vec<ironsmith::Target>> = game.stack.iter().map(|entry| entry.targets.clone()).collect();
    assert!(
        game.stack.len() == 2
            && game.stack.last().unwrap().targets.len() == 1
            && matches!(game.stack.last().unwrap().targets[0], ironsmith::Target::Object(id) if tokens.contains(&id)),
        "CR 707.10e: exactly one copy is created and it targets one of the two tokens: stack targets {copy_targets:?}, copies {}",
        copies.len()
    );
}

const MAGAR: &str = "Mana cost: {1}{B}{R}\nType: Legendary Creature — Minotaur Performer\nPower/Toughness: 3/3\n{1}{B}{R}: Note the name of target instant or sorcery card in your graveyard and put it onto the battlefield face down. It's a 3/3 creature with \"Whenever this creature deals combat damage to a player, you may create a copy of the card with the noted name. You may cast the copy without paying its mana cost\" and \"If this creature would leave the battlefield, exile it instead of putting it anywhere else.\"";
const SHOCK: &str = "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target.";

#[test]
fn cr_707_14_magar_copies_the_noted_card_using_its_graveyard_characteristics() {
    let magar = compile("Magar of the Magic Strings", MAGAR);
    let shock = compile("Shock", SHOCK);
    let mut game = new_game();
    game.register_linked_face_definition(&shock);
    let performer = game.create_object_from_definition(&magar, alice(), Zone::Battlefield);
    let card = game.create_object_from_definition(&shock, alice(), Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    resolve_ability(&mut game, performer, alice(), activated_effects(&magar, 0), vec![ironsmith::Target::Object(card)], &mut Scripted::yes());
    let face_down = game.find_object_by_stable_id(stable).unwrap();
    assert!(game.is_face_down(face_down) && game.object(face_down).unwrap().zone == Zone::Battlefield);
    assert_eq!((game.calculated_power(face_down), game.calculated_toughness(face_down)), (Some(3), Some(3)));
    let program = game
        .current_characteristics(face_down)
        .unwrap()
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
            _ => None,
        })
        .expect("granted combat-damage trigger");
    let event = ironsmith::TriggerEvent::new_with_provenance(
        ironsmith::events::DamageEvent::with_cause(face_down, ironsmith::events::DamageTarget::Player(bob()), 3, true, ironsmith::events::EventCause::effect()),
        Default::default(),
    );
    let mut dm = Scripted::yes();
    dm.target_pick = vec![ironsmith::Target::Player(bob())];
    game.stack.push(StackEntry::ability(face_down, alice(), program).with_triggering_event(event));
    resolve_top(&mut game, &mut dm);
    let top = game.stack.last().map(|entry| entry.object_id).expect("CR 707.14: the copy of the noted card is cast");
    let chars = game.current_characteristics(top).unwrap();
    assert!(
        chars.name.eq_ignore_ascii_case("Shock") && chars.card_types.contains(&CardType::Instant),
        "CR 707.14: the copy has the noted card's characteristics: {:?}",
        chars.name
    );
    resolve_top(&mut game, &mut Scripted::yes());
    assert_eq!(game.player(bob()).unwrap().life, 18, "the Shock copy resolves");
}

#[test]
fn cr_709_3c_cast_copy_of_a_split_card_keeps_both_halves_to_choose_from() {
    use ironsmith::card::LinkedFaceLayout;
    let mizzix = compile("Mizzix's Mastery", MIZZIX);
    let mut fire = compile("Fire", "Mana cost: {1}{R}\nType: Instant\nFire deals 2 damage divided as you choose among one or two targets.");
    let mut ice = compile("Ice", "Mana cost: {1}{U}\nType: Instant\nTap target permanent.\nDraw a card.");
    let mut game = new_game();
    fill_library(&mut game, alice(), 5);
    link_faces(&mut game, &mut fire, &mut ice, false);
    fire.card.linked_face_layout = LinkedFaceLayout::Split;
    ice.card.linked_face_layout = LinkedFaceLayout::Split;
    game.register_linked_face_definition(&fire);
    game.register_linked_face_definition(&ice);
    let card = game.create_object_from_definition(&fire, alice(), Zone::Graveyard);
    let tapped = game.create_object_from_definition(&bear("Tap Target"), bob(), Zone::Battlefield);
    let mut dm = Scripted::preferring(&["Ice"]);
    dm.target_pick = vec![ironsmith::Target::Object(tapped)];
    resolve_spell(&mut game, &mizzix, alice(), vec![ironsmith::Target::Object(card)], &mut dm);
    let top = game.stack.last().map(|entry| entry.object_id);
    let name = top.and_then(|id| game.current_name(id));
    assert_eq!(
        name.as_deref(),
        Some("Ice"),
        "CR 709.3c: the cast copy keeps both halves, so the player may cast Ice (prompts {:?})",
        dm.option_prompts
    );
}

#[test]
fn cr_712_4b_meld_back_face_cannot_be_used_outside_a_melded_permanent() {
    let garrison = compile(
        "Hanweir Garrison",
        "Mana cost: {2}{R}\nType: Creature — Human Soldier\nPower/Toughness: 2/3\nWhenever this creature attacks, create two 1/1 red Human creature tokens that are tapped and attacking.\n(Melds with Hanweir Battlements.)",
    );
    let mut game = new_game();
    let card = game.create_object_from_definition(&garrison, alice(), Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    let source = game.create_object_from_definition(&plain("Returner", CardType::Artifact), alice(), Zone::Battlefield);
    resolve_ability(
        &mut game,
        source,
        alice(),
        vec![Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(card), Zone::Battlefield, false).transformed())].into(),
        vec![],
        &mut Scripted::yes(),
    );
    let current = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        game.object(current).unwrap().zone,
        Zone::Graveyard,
        "CR 712.4b/712.14a: a meld card's back face can't determine characteristics outside a melded permanent, so it isn't put onto the battlefield transformed"
    );
}

#[test]
fn cr_712_17_double_faced_card_exiled_face_down_stays_hidden() {
    let mut front = compile("Delver of Secrets", DELVER_FRONT);
    let mut back = compile("Insectile Aberration", DELVER_BACK);
    let mut game = new_game();
    link_faces(&mut game, &mut front, &mut back, true);
    let delver = game.create_object_from_definition(&front, alice(), Zone::Battlefield);
    let stable = game.object(delver).unwrap().stable_id;
    let source = game.create_object_from_definition(&plain("Exiler", CardType::Artifact), bob(), Zone::Battlefield);
    resolve_ability(
        &mut game,
        source,
        bob(),
        vec![Effect::new(ironsmith::effects::ExileEffect::with_spec(ironsmith::ChooseSpec::SpecificObject(delver)).with_face_down(true))].into(),
        vec![],
        &mut Scripted::yes(),
    );
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert!(
        game.object(exiled).unwrap().zone == Zone::Exile
            && game.is_face_down(exiled)
            && !game.can_player_look_at_face_down_exiled_card(exiled, bob()),
        "CR 712.17: a double-faced card exiled face down remains hidden"
    );
}

#[test]
fn cr_729_2a_attraction_supplementary_deck_moves_into_and_out_of_a_subgame() {
    use ironsmith::game_state::AttractionDeckFormat;
    let mut game = new_game();
    for player in [alice(), bob()] {
        fill_library(&mut game, player, 10);
    }
    game.enable_attractions(vec![(alice(), AttractionDeckFormat::Limited, (0..3).map(|i| attraction(&format!("Ride {i}"), vec![i as u8 + 1])).collect())])
        .unwrap();
    game.begin_subgame(None, alice(), Vec::new()).expect("subgame starts");
    assert_eq!(
        game.attraction_deck(alice()).map(<[_]>::len),
        Some(3),
        "CR 729.2a: the Attraction deck moves to the subgame command zone"
    );
    let source = game.create_object_from_definition(&plain("Opener", CardType::Artifact), alice(), Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice());
    ironsmith::effects::execute_effect(&mut game, &Effect::open_attraction(), &mut ctx).unwrap();
    assert_eq!(game.attraction_deck(alice()).map(<[_]>::len), Some(2));
    game.finish_subgame_with(ironsmith::decision::GameResult::Winner(alice()), &mut Scripted::yes())
        .expect("subgame ends");
    assert_eq!(
        game.attraction_deck(alice()).map(<[_]>::len),
        Some(3),
        "CR 729.5a: the Attraction opened in the subgame returns to the bottom of its deck, which returns to the main game"
    );
}

#[test]
fn cr_708_12_revealing_a_face_down_permanent_uses_its_card_characteristics() {
    let shrieker = compile(
        "Hauntwoods Shrieker",
        "Mana cost: {1}{G}{G}\nType: Creature — Beast Mutant\nPower/Toughness: 3/3\nWhenever this creature attacks, manifest dread.\n{1}{G}: Reveal target face-down permanent. If it's a creature card, you may turn it face up.",
    );
    let mut game = new_game();
    let beast = game.create_object_from_definition(&shrieker, alice(), Zone::Battlefield);
    let artifact_card = game.create_object_from_definition(&plain("Hidden Artifact", CardType::Artifact), alice(), Zone::Library);
    let creature_card = game.create_object_from_definition(&bear("Hidden Bear"), alice(), Zone::Library);
    let mut stables = Vec::new();
    for card in [artifact_card, creature_card] {
        stables.push(game.object(card).unwrap().stable_id);
        resolve_ability(
            &mut game,
            beast,
            alice(),
            vec![Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(card), Zone::Battlefield, false).face_down())].into(),
            vec![],
            &mut Scripted::yes(),
        );
    }
    let mut outcome = Vec::new();
    for stable in &stables {
        let permanent = game.find_object_by_stable_id(*stable).unwrap();
        assert!(game.is_face_down(permanent));
        assert!(
            game.current_characteristics(permanent).unwrap().card_types.contains(&CardType::Creature),
            "fixture: every face-down permanent is a 2/2 creature"
        );
        resolve_ability(&mut game, beast, alice(), activated_effects(&shrieker, 0), vec![ironsmith::Target::Object(permanent)], &mut Scripted::yes());
        let permanent = game.find_object_by_stable_id(*stable).unwrap();
        outcome.push(game.is_face_down(permanent));
    }
    assert_eq!(
        outcome,
        vec![true, false],
        "CR 708.12: \"if it's a creature card\" reads the revealed card ignoring the face-down creature overlay"
    );
}

fn vanguard_card(name: &str, hand: i32, life: i32) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Vanguard])
            .vanguard_modifiers(hand, life)
            .build(),
    )
}

const SHAHRAZAD: &str = "Mana cost: {W}{W}\nType: Sorcery\nPlayers play a Magic subgame, using their libraries as their decks. Each player who doesn't win the subgame loses half their life, rounded up.";

#[test]
fn cr_729_1_shahrazad_plays_a_subgame_and_resumes_the_main_game() {
    let shahrazad = compile("Shahrazad", SHAHRAZAD);
    let mut game = new_game();
    for player in [alice(), bob()] {
        fill_library(&mut game, player, 12);
    }
    resolve_spell(&mut game, &shahrazad, alice(), vec![], &mut Scripted::yes());
    assert_eq!(game.subgame_depth(), 1, "CR 729.1: Shahrazad starts a Magic subgame");
    assert_eq!(game.player(alice()).unwrap().hand.len(), 7, "the subgame starts with its own opening hands");
    game.finish_subgame_with(ironsmith::decision::GameResult::Winner(alice()), &mut Scripted::yes())
        .expect("subgame ends");
    assert_eq!(game.subgame_depth(), 0);
    assert_eq!(
        (game.player(alice()).unwrap().life, game.player(bob()).unwrap().life),
        (20, 10),
        "CR 729.1: the main game resumes and the nonwinner loses half their life"
    );
}

#[test]
fn cr_729_2b_729_5b_vanguards_move_into_the_subgame_command_zone_and_back() {
    let mut game = new_game();
    for player in [alice(), bob()] {
        fill_library(&mut game, player, 12);
    }
    game.enable_vanguard(vec![(alice(), vanguard_card("Alice Avatar", 1, 5)), (bob(), vanguard_card("Bob Avatar", 0, -2))])
        .unwrap();
    game.begin_subgame(None, alice(), Vec::new()).unwrap();
    for player in [alice(), bob()] {
        let avatar = game.vanguard_card(player).expect("CR 729.2b: the vanguard is in the subgame");
        assert_eq!(game.object(avatar).unwrap().zone, Zone::Command, "CR 729.2b: subgame command zone");
    }
    assert_eq!(game.player(alice()).unwrap().life, 25, "CR 729.2b: the vanguard's modifiers apply to the subgame");
    game.finish_subgame_with(ironsmith::decision::GameResult::Winner(alice()), &mut Scripted::yes())
        .unwrap();
    for player in [alice(), bob()] {
        let avatar = game.vanguard_card(player).expect("CR 729.5b: the vanguard returns to the main game");
        assert_eq!(game.object(avatar).unwrap().zone, Zone::Command);
    }
}

#[test]
fn cr_729_2a_729_5a_planar_decks_move_in_and_revealed_planes_return_to_the_deck() {
    let planes = |prefix: &str| {
        (0..10)
            .map(|index| (plain(&format!("{prefix} Plane {index}"), CardType::Plane), ironsmith::PlanarCardKind::Plane))
            .collect::<Vec<_>>()
    };
    let mut game = new_game();
    for player in [alice(), bob()] {
        fill_library(&mut game, player, 10);
    }
    game.enable_planechase(vec![(alice(), planes("Alice")), (bob(), planes("Bob"))]).unwrap();
    game.begin_subgame(None, alice(), Vec::new()).unwrap();
    let child = game.planechase.as_ref().unwrap();
    assert_eq!(
        child.decks.values().map(Vec::len).sum::<usize>() + child.face_up.len(),
        20,
        "CR 729.2a: both planar decks are moved to the subgame"
    );
    assert_eq!(child.face_up.len(), 1, "the subgame reveals its own starting plane");
    game.finish_subgame_with(ironsmith::decision::GameResult::Winner(alice()), &mut Scripted::yes())
        .unwrap();
    let parent = game.planechase.as_ref().unwrap();
    assert!(
        parent.face_up.is_empty() && parent.decks.values().map(Vec::len).sum::<usize>() == 20,
        "CR 729.5a: the plane revealed in the subgame returns to its deck, and the decks return to the main game"
    );
}
