//! CR audit (pending-entry pass, package p11): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState, declare_attackers, declare_blockers};
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{
    AttackerDeclaration, DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker,
    compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect, DestroyEffect, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::{Phase, StackEntry, Step, Target};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{AbilityKind, CardType, ChooseSpec, Effect, ObjectId, PlayerFilter, Until, Zone};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

/// Compile a card from its full Oracle text through the strict compiler.
fn card(name: &str, text: &str) -> CardDefinition {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("fixture {name} must compile: {error:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
        "fixture {name} compiled with unimplemented content"
    );
    definition
}

/// Compile a card, returning `None` when the strict compiler rejects it or
/// leaves unimplemented content (used when compiling is itself the CR claim).
fn try_card(name: &str, text: &str) -> Option<CardDefinition> {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .ok()
        .filter(|definition| {
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition)
        })
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinitionBuilder {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(power, toughness))
}

fn vanilla(game: &mut GameState, name: &str, power: i32, toughness: i32, owner: PlayerId) -> ObjectId {
    put(game, &creature(name, power, toughness).build(), owner)
}

fn game_at(phase: Phase, step: Option<Step>) -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = ALICE;
    game.turn.priority_player = Some(ALICE);
    game.turn.phase = phase;
    game.turn.step = step;
    game
}

fn main_game() -> GameState {
    game_at(Phase::FirstMain, None)
}

fn combat_game() -> GameState {
    game_at(Phase::Combat, Some(Step::DeclareAttackers))
}

fn put(game: &mut GameState, def: &CardDefinition, controller: PlayerId) -> ObjectId {
    let id = game.create_object_from_definition(def, controller, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn life(game: &GameState, player: PlayerId) -> i32 {
    game.player(player).unwrap().life
}

fn counters(game: &GameState, id: ObjectId, kind: CounterType) -> u32 {
    game.object(id)
        .and_then(|object| object.counters.get(&kind).copied())
        .unwrap_or(0)
}

fn add_mana(game: &mut GameState, player: PlayerId, symbol: ManaSymbol, amount: u32) {
    game.player_mut(player).unwrap().mana_pool.add(symbol, amount);
}

fn sba(game: &mut GameState) {
    ironsmith::rules::state_based::apply_state_based_actions(game).unwrap();
}

fn deal(game: &mut GameState, source: ObjectId, controller: PlayerId, amount: i32, recipient: ChooseSpec) {
    DealDamageEffect::new(amount, recipient)
        .execute(game, &mut EffectContext::new_default(source, controller))
        .unwrap();
}

fn apply(game: &mut GameState, source: ObjectId, controller: PlayerId, target: ObjectId, modification: Modification) {
    ApplyContinuousEffect::new(EffectTarget::Specific(target), modification, Until::EndOfTurn)
        .execute(game, &mut EffectContext::new_default(source, controller))
        .unwrap();
}

/// A spell object of the given colored mana cost on the stack (a targeting /
/// damage source with those colors).
fn colored_spell(game: &mut GameState, cost: &str, controller: PlayerId) -> ObjectId {
    let def = card(
        &format!("Probe {cost}"),
        &format!("Mana cost: {cost}\nType: Instant\nDraw a card."),
    );
    let id = game.create_object_from_definition(&def, controller, Zone::Stack);
    game.stack.push(StackEntry::new(id, controller));
    id
}

fn can_target(game: &GameState, target: ObjectId, source: ObjectId, caster: PlayerId) -> bool {
    ironsmith::can_target_object(game, target, source, caster).is_legal()
}

fn static_count(game: &GameState, id: ObjectId, pred: impl Fn(&StaticAbility) -> bool) -> usize {
    game.current_characteristics(id)
        .unwrap()
        .abilities
        .iter()
        .filter(|ability| matches!(&ability.kind, AbilityKind::Static(s) if pred(s)))
        .count()
}

fn def_static_count(def: &CardDefinition, pred: impl Fn(&StaticAbility) -> bool) -> usize {
    def.abilities
        .iter()
        .filter(|ability| matches!(&ability.kind, AbilityKind::Static(s) if pred(s)))
        .count()
}

/// Delegates to SelectFirst, but picks optional costs whose description
/// starts with one of `optional`, answers booleans with `yes`, prefers
/// objects named `prefer`, and fixes X.
struct Dm {
    optional: Vec<&'static str>,
    yes: bool,
    prefer: Vec<String>,
    x: u32,
    seen_options: Vec<String>,
    booleans: usize,
    color: Option<ironsmith::color::Color>,
    optional_times: usize,
    first: SelectFirstDecisionMaker,
}

impl Dm {
    fn new() -> Self {
        Self {
            optional: Vec::new(),
            yes: true,
            prefer: Vec::new(),
            x: 0,
            seen_options: Vec::new(),
            booleans: 0,
            color: None,
            optional_times: 1,
            first: SelectFirstDecisionMaker,
        }
    }
    fn pick(mut self, label: &'static str) -> Self {
        self.optional.push(label);
        self
    }
    fn prefer(mut self, name: &str) -> Self {
        self.prefer.push(name.to_string());
        self
    }
    fn no(mut self) -> Self {
        self.yes = false;
        self
    }
}

impl DecisionMaker for Dm {
    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        match self.color {
            Some(color) => vec![color; ctx.count as usize],
            None => self.first.decide_colors(game, ctx),
        }
    }
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.booleans += 1;
        self.yes
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            self.x.clamp(ctx.min, ctx.max)
        } else {
            self.first.decide_number(game, ctx)
        }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        for name in &self.prefer {
            let picked: Vec<_> = ctx
                .candidates
                .iter()
                .filter(|c| c.name == *name && c.legal)
                .map(|c| c.id)
                .take(ctx.max.unwrap_or(usize::MAX).max(1))
                .collect();
            if !picked.is_empty() && picked.len() >= ctx.min {
                return picked;
            }
        }
        self.first.decide_objects(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.seen_options.extend(
            ctx.options
                .iter()
                .map(|option| format!("{} legal={}", option.description, option.legal)),
        );
        let picked: Vec<usize> = ctx
            .options
            .iter()
            .filter(|option| {
                option.legal
                    && self
                        .optional
                        .iter()
                        .any(|label| option.description.starts_with(label))
            })
            .map(|option| option.index)
            .collect();
        if !picked.is_empty() && picked.len() >= ctx.min {
            if ctx.description.starts_with("Choose optional costs") && self.optional_times > 1 {
                return picked
                    .iter()
                    .flat_map(|index| std::iter::repeat_n(*index, self.optional_times))
                    .collect();
            }
            return picked;
        }
        if ctx.description.starts_with("Choose optional costs") && ctx.min == 0 {
            return Vec::new();
        }
        self.first.decide_options(game, ctx)
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut out = Vec::new();
        for requirement in &ctx.requirements {
            let preferred = requirement.legal_targets.iter().find(|target| match target {
                Target::Object(id) => game.object(*id).is_some_and(|object| {
                    self.prefer.iter().any(|name| object.name.as_str() == name.as_str())
                }),
                Target::Player(_) => false,
            });
            match preferred {
                Some(target) => out.push(*target),
                None => {
                    if let Some(target) = requirement.legal_targets.first() {
                        if requirement.min_targets > 0 {
                            out.push(*target);
                        }
                    }
                }
            }
        }
        if out.is_empty() {
            self.first.decide_targets(game, ctx)
        } else {
            out
        }
    }
}

fn find_cast(
    game: &GameState,
    player: PlayerId,
    spell: ObjectId,
    pred: impl Fn(&CastingMethod, Zone) -> bool,
) -> Option<LegalAction> {
    compute_legal_actions(game, player)
        .expect("fixture has complete replacement state")
        .into_iter()
        .find(|action| {
            matches!(action, LegalAction::CastSpell { spell_id, from_zone, casting_method }
            if *spell_id == spell && pred(casting_method, *from_zone))
        })
}

fn any_method(_: &CastingMethod, _: Zone) -> bool {
    true
}

fn alternative(method: &CastingMethod, _: Zone) -> bool {
    matches!(method, CastingMethod::Alternative(_))
}

fn find_activation(game: &GameState, player: PlayerId, source_id: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, player)
        .expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == source_id))
}

/// Take a priority action for `player`, answering decisions with `dm` until
/// the stack grows (or the action completes); then put pending triggers on
/// the stack. Returns whether the stack grew.
fn act(game: &mut GameState, queue: &mut TriggerQueue, player: PlayerId, action: LegalAction, dm: &mut Dm) -> bool {
    game.turn.priority_player = Some(player);
    let before = game.stack.len();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..32 {
        if game.stack.len() > before || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result =
            ironsmith::game_loop::apply_decision_context_with_dm(game, queue, &mut state, &ctx, dm);
    }
    let grew = game.stack.len() > before;
    flush_triggers(game, queue, dm);
    grew
}

fn cast_with(
    game: &mut GameState,
    player: PlayerId,
    spell: ObjectId,
    pred: impl Fn(&CastingMethod, Zone) -> bool,
    dm: &mut Dm,
) {
    let action = find_cast(game, player, spell, pred).expect("spell is castable");
    let mut queue = TriggerQueue::new();
    assert!(act(game, &mut queue, player, action, dm), "the cast reached the stack");
}

fn flush_triggers(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut Dm) {
    for event in game.take_pending_trigger_events() {
        for entry in ironsmith::triggers::check_delayed_triggers(game, &event) {
            queue.add(entry);
        }
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, dm).unwrap();
}

fn resolve_top_with(game: &mut GameState, dm: &mut Dm) {
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    let mut queue = TriggerQueue::new();
    flush_triggers(game, &mut queue, dm);
}

fn resolve_all(game: &mut GameState, dm: &mut Dm) {
    for _ in 0..48 {
        if game.stack.is_empty() {
            return;
        }
        resolve_top_with(game, dm);
    }
    panic!("stack didn't empty");
}

fn zone_of(game: &GameState, stable: ironsmith::ids::StableId) -> Option<Zone> {
    game.find_object_by_stable_id(stable)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

// Combat helpers ------------------------------------------------------------

fn attack(game: &mut GameState, attacks: Vec<(ObjectId, AttackTarget)>) {
    let mut combat = CombatState::default();
    declare_attackers(game, &mut combat, attacks).unwrap();
    game.combat = Some(combat);
}

/// Declare attacks through the turn-based action, queueing attack triggers,
/// then resolve them.
fn attack_and_resolve(game: &mut GameState, attacks: &[(ObjectId, AttackTarget)], dm: &mut Dm) {
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    let declarations = attacks
        .iter()
        .map(|(creature, target)| AttackerDeclaration {
            creature: *creature,
            target: target.clone(),
        })
        .collect::<Vec<_>>();
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, &declarations)
        .unwrap();
    game.combat = Some(combat);
    resolve_queue(game, &mut queue, dm);
}

/// Declare blocks through the turn-based action, then resolve block triggers.
fn block_and_resolve(game: &mut GameState, blocks: Vec<(ObjectId, ObjectId)>, dm: &mut Dm) {
    let mut combat = game.combat.clone().unwrap_or_default();
    let mut queue = TriggerQueue::new();
    let declarations = blocks
        .iter()
        .map(|(blocker, blocking)| ironsmith::decision::BlockerDeclaration {
            blocker: *blocker,
            blocking: *blocking,
        })
        .collect::<Vec<_>>();
    ironsmith::game_loop::apply_multiplayer_blocker_declarations(
        game,
        &mut combat,
        &mut queue,
        &declarations,
    )
    .unwrap();
    game.combat = Some(combat);
    resolve_queue(game, &mut queue, dm);
}

fn resolve_queue(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut Dm) {
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, dm).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
        flush_triggers(game, queue, dm);
    }
    game.refresh_continuous_state().unwrap();
}

fn block(game: &mut GameState, blocks: Vec<(ObjectId, ObjectId)>) {
    let mut combat = game.combat.clone().unwrap_or_default();
    declare_blockers(game, &mut combat, blocks).unwrap();
    game.combat = Some(combat);
}

fn next_prompt(game: &GameState) -> Option<ironsmith::game_loop::CombatDamageAssignmentPrompt> {
    let combat = game.combat.clone().expect("combat in progress");
    ironsmith::game_loop::next_combat_damage_assignment_prompt(game, &combat, false, None)
}

fn deal_combat_damage(game: &mut GameState) {
    let combat = game.combat.clone().expect("combat in progress");
    ironsmith::game_loop::execute_combat_damage_step(game, &combat, false);
}

fn deal_combat_damage_step(game: &mut GameState, first_strike: bool) {
    let combat = game.combat.clone().expect("combat in progress");
    ironsmith::game_loop::execute_combat_damage_step(game, &combat, first_strike);
}

// ===========================================================================
// 702.1 keyword-ability general rules
// ===========================================================================

#[test]
fn cr_702_1d_with_keyword_means_with_that_ability() {
    let mut game = main_game();
    put(
        &mut game,
        &card(
            "Favorable Winds",
            "Mana cost: {1}{U}\nType: Enchantment\nCreatures you control with flying get +1/+1.",
        ),
        ALICE,
    );
    let flyer = put(&mut game, &creature("Flyer", 1, 1).with_ability(ironsmith::Ability::static_ability(StaticAbility::flying())).build(), ALICE);
    let walker = vanilla(&mut game, "Walker", 1, 1, ALICE);
    let granted = vanilla(&mut game, "Granted flyer", 1, 1, ALICE);
    apply(&mut game, walker, ALICE, granted, Modification::AddAbility(StaticAbility::flying()));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(flyer), Some(2), "CR 702.1d: a creature with a flying ability is a creature 'with flying'");
    assert_eq!(game.calculated_power(granted), Some(2), "CR 702.1d: a granted flying ability counts as 'with flying'");
    assert_eq!(game.calculated_power(walker), Some(1), "CR 702.1d: a creature without flying isn't 'with flying'");
}

// ===========================================================================
// 702.2 Deathtouch
// ===========================================================================

#[test]
fn cr_702_2a_12a_15a_11a_22a_keywords_compile_to_static_abilities() {
    for (keyword, pred) in [
        ("Deathtouch", StaticAbility::has_deathtouch as fn(&StaticAbility) -> bool),
        ("Indestructible", StaticAbility::has_indestructible),
        ("Lifelink", StaticAbility::has_lifelink),
        ("Hexproof", StaticAbility::has_hexproof),
    ] {
        let def = card(
            &format!("{keyword} holder"),
            &format!("Mana cost: {{1}}\nType: Creature — Bear\nPower/Toughness: 2/2\n{keyword}"),
        );
        assert_eq!(def_static_count(&def, pred), 1, "CR 702.2a/11a/12a/15a: {keyword} is a static ability");
    }
    let def = card("Banding holder", "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/2\nBanding");
    assert_eq!(
        def_static_count(&def, |s| s.id() == ironsmith::static_abilities::StaticAbilityId::Banding),
        1,
        "CR 702.22a: banding is a static ability"
    );
}

#[test]
fn cr_702_2b_d_f_deathtouch_destroys_from_any_zone_and_duplicates_are_redundant() {
    for zone in [Zone::Battlefield, Zone::Graveyard, Zone::Hand, Zone::Exile] {
        for instances in 0..=2usize {
            let mut game = main_game();
            let text = std::iter::repeat_n("Deathtouch", instances).collect::<Vec<_>>().join("\n");
            let def = card(
                "Deathtouch dealer",
                &format!("Mana cost: {{B}}\nType: Creature — Snake\nPower/Toughness: 1/1\n{text}"),
            );
            let source = game.create_object_from_definition(&def, ALICE, zone);
            let victim = vanilla(&mut game, "Big victim", 5, 5, BOB);
            deal(&mut game, source, ALICE, 1, ChooseSpec::SpecificObject(victim));
            sba(&mut game);
            assert_eq!(
                game.battlefield.contains(&victim),
                instances == 0,
                "CR 702.2b/d/f: deathtouch source in {zone:?} with {instances} instance(s) dealing 1 damage"
            );
            // Two instances must not do anything a single one wouldn't: the
            // damage dealt and marked is still exactly 1.
            if instances > 0 {
                assert_eq!(
                    game.player(BOB).unwrap().life,
                    20,
                    "CR 702.2f: redundant deathtouch changes nothing else"
                );
            }
        }
    }
}

#[test]
fn cr_702_2e_15c_departed_source_uses_last_known_deathtouch_and_lifelink() {
    let mut game = main_game();
    let source = vanilla(&mut game, "Temporary killer", 1, 1, ALICE);
    apply(&mut game, source, ALICE, source, Modification::AddAbility(StaticAbility::deathtouch()));
    apply(&mut game, source, ALICE, source, Modification::AddAbility(StaticAbility::lifelink()));
    let victim = vanilla(&mut game, "Big victim", 5, 5, BOB);
    game.push_to_stack(StackEntry::ability(
        source,
        ALICE,
        vec![Effect::new(DealDamageEffect::new(1, ChooseSpec::SpecificObject(victim)))],
    ));
    let moved = game.move_object_by_effect(source, Zone::Graveyard).expect("source leaves");
    assert!(!game.object_has_ability(moved, &StaticAbility::deathtouch()));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.damage_on(victim), 1);
    assert_eq!(life(&game, ALICE), 21, "CR 702.15c: last known information says the source had lifelink");
    sba(&mut game);
    assert!(
        !game.battlefield.contains(&victim),
        "CR 702.2e: last known information says the source had deathtouch"
    );
}

#[test]
fn cr_702_2c_deathtouch_trampler_one_damage_is_lethal_for_excess() {
    for deathtouch in [false, true] {
        let mut game = combat_game();
        let mut builder = creature("Trampler", 3, 3).with_ability(ironsmith::Ability::static_ability(StaticAbility::trample()));
        if deathtouch {
            builder = builder.with_ability(ironsmith::Ability::static_ability(StaticAbility::deathtouch()));
        }
        let attacker = put(&mut game, &builder.build(), ALICE);
        let blocker = vanilla(&mut game, "Wall", 0, 4, BOB);
        attack(&mut game, vec![(attacker, AttackTarget::Player(BOB))]);
        block(&mut game, vec![(blocker, attacker)]);
        let prompt = next_prompt(&game).expect("blocked trampler chooses a division");
        let split = [(Target::Object(blocker), 1), (Target::Player(BOB), 2)];
        assert_eq!(
            prompt.validate(&game, &split).is_ok(),
            deathtouch,
            "CR 702.2c: 1 deathtouch damage is lethal, so 2 may trample over (deathtouch={deathtouch})"
        );
        if deathtouch {
            prompt.record(&mut game, &split);
            deal_combat_damage(&mut game);
            assert_eq!(life(&game, BOB), 18, "CR 702.2c: excess reaches the player");
            sba(&mut game);
            assert!(!game.battlefield.contains(&blocker), "CR 702.2b: the 1 damage destroys it");
        }
    }
}

// ===========================================================================
// Redundant instances (702.4e, 702.7d, 702.8b, 702.12c, 702.19g)
// ===========================================================================

#[test]
fn cr_702_4e_7d_double_and_first_strike_duplicates_are_redundant() {
    for (keyword, instances) in [("Double strike", 1usize), ("Double strike", 2), ("First strike", 1), ("First strike", 2)] {
        let mut game = combat_game();
        let text = std::iter::repeat_n(keyword, instances).collect::<Vec<_>>().join("\n");
        let attacker = put(
            &mut game,
            &card("Striker", &format!("Mana cost: {{R}}\nType: Creature — Human\nPower/Toughness: 2/2\n{text}")),
            ALICE,
        );
        attack(&mut game, vec![(attacker, AttackTarget::Player(BOB))]);
        deal_combat_damage_step(&mut game, true);
        assert_eq!(life(&game, BOB), 18, "CR 702.4e/7d: {instances}x {keyword} deals its damage once in the first-strike step");
        deal_combat_damage_step(&mut game, false);
        let expected = if keyword == "Double strike" { 16 } else { 18 };
        assert_eq!(
            life(&game, BOB),
            expected,
            "CR 702.4e/7d: {instances}x {keyword} behaves exactly like one instance in the regular step"
        );
    }
}

#[test]
fn cr_702_8b_flash_duplicates_are_redundant() {
    for instances in [1usize, 2] {
        let mut game = game_at(Phase::Beginning, Some(Step::Upkeep));
        game.turn.active_player = BOB;
        game.turn.priority_player = Some(ALICE);
        let text = std::iter::repeat_n("Flash", instances).collect::<Vec<_>>().join("\n");
        let def = card("Flasher", &format!("Mana cost: {{G}}\nType: Creature — Elf\nPower/Toughness: 1/1\n{text}"));
        let spell = game.create_object_from_definition(&def, ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Green, 1);
        assert!(
            find_cast(&game, ALICE, spell, any_method).is_some(),
            "CR 702.8b: {instances}x flash lets it be cast on an opponent's upkeep"
        );
        let actions = compute_legal_actions(&game, ALICE).unwrap();
        assert_eq!(
            actions.iter().filter(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).count(),
            1,
            "CR 702.8b: duplicate flash doesn't create extra ways to cast"
        );
    }
}

#[test]
fn cr_702_12b_c_indestructible_ignores_destroy_and_lethal_damage() {
    for instances in 0..=2usize {
        let mut game = main_game();
        let text = std::iter::repeat_n("Indestructible", instances).collect::<Vec<_>>().join("\n");
        let def = card("Sturdy", &format!("Mana cost: {{W}}\nType: Creature — Golem\nPower/Toughness: 2/2\n{text}"));
        let first = put(&mut game, &def, ALICE);
        let second = put(&mut game, &def, ALICE);
        let source = vanilla(&mut game, "Source", 1, 1, BOB);
        DestroyEffect::with_spec(ChooseSpec::SpecificObject(first))
            .execute(&mut game, &mut EffectContext::new_default(source, BOB))
            .unwrap();
        deal(&mut game, source, BOB, 10, ChooseSpec::SpecificObject(second));
        sba(&mut game);
        let survives = instances > 0;
        assert_eq!(game.battlefield.contains(&first), survives, "CR 702.12b/c: destroy vs {instances}x indestructible");
        assert_eq!(game.battlefield.contains(&second), survives, "CR 702.12b/c: lethal damage vs {instances}x indestructible");
        if survives {
            assert_eq!(game.damage_on(second), 10, "CR 702.12b: the damage stays marked");
        }
    }
}

// ===========================================================================
// 702.11 Hexproof / 702.18 Shroud
// ===========================================================================

#[test]
fn cr_702_11c_player_hexproof_only_stops_opponents() {
    let mut game = main_game();
    put(
        &mut game,
        &card("Leyline of Sanctity", "Mana cost: {2}{W}{W}\nType: Enchantment\nIf this card is in your opening hand, you may begin the game with it on the battlefield.\nYou have hexproof. (You can't be the target of spells or abilities your opponents control.)"),
        ALICE,
    );
    game.refresh_continuous_state().unwrap();
    let bob_spell = colored_spell(&mut game, "{R}", BOB);
    let alice_spell = colored_spell(&mut game, "{R}", ALICE);
    let spec = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
    let by_bob = ironsmith::targeting::compute_legal_targets(&game, &spec, BOB, Some(bob_spell));
    let by_alice = ironsmith::targeting::compute_legal_targets(&game, &spec, ALICE, Some(alice_spell));
    assert!(!by_bob.contains(&Target::Player(ALICE)), "CR 702.11c: an opponent can't target a player with hexproof");
    assert!(by_bob.contains(&Target::Player(BOB)));
    assert!(by_alice.contains(&Target::Player(ALICE)), "CR 702.11c: the player can still target themself");
}

#[test]
fn cr_702_11d_hexproof_from_quality_restricts_only_matching_opponent_sources() {
    let mut game = main_game();
    let knight = put(
        &mut game,
        &card("Knight of Grace", "Mana cost: {1}{W}\nType: Creature — Human Knight\nPower/Toughness: 2/2\nFirst strike\nHexproof from black\nKnight of Grace gets +1/+0 as long as any player controls a black permanent."),
        ALICE,
    );
    game.refresh_continuous_state().unwrap();
    let bob_black = colored_spell(&mut game, "{B}", BOB);
    let bob_red = colored_spell(&mut game, "{R}", BOB);
    let alice_black = colored_spell(&mut game, "{B}", ALICE);
    assert!(!can_target(&game, knight, bob_black, BOB), "CR 702.11d: black spell an opponent controls can't target it");
    assert!(can_target(&game, knight, bob_red, BOB), "CR 702.11d: non-black opponent spells can target it");
    assert!(can_target(&game, knight, alice_black, ALICE), "CR 702.11d: only opponents are restricted");
    // A black-source ability an opponent controls.
    let black_permanent = put(
        &mut game,
        &card("Black pinger", "Mana cost: {B}\nType: Creature — Zombie\nPower/Toughness: 1/1"),
        BOB,
    );
    assert!(!can_target(&game, knight, black_permanent, BOB), "CR 702.11d: abilities from black sources an opponent controls can't target it");
    assert_eq!(
        static_count(&game, knight, |s| s.hexproof_from_filter().is_some()),
        1,
        "CR 702.11d: hexproof from black is a hexproof ability"
    );
}

#[test]
fn cr_702_11e_f_losing_hexproof_removes_hexproof_from_and_two_qualities_are_two_abilities() {
    let mut game = main_game();
    let veil = card(
        "Veil of Summer",
        "Mana cost: {G}\nType: Instant\nDraw a card if an opponent has cast a blue or black spell this turn. Spells you control can't be countered this turn. You and permanents you control gain hexproof from blue and from black until end of turn. (You and they can't be the targets of blue or black spells or abilities your opponents control.)",
    );
    let bear = vanilla(&mut game, "Bear", 2, 2, ALICE);
    let veil = game.create_object_from_definition(&veil, ALICE, Zone::Stack);
    game.stack.push(StackEntry::new(veil, ALICE));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    game.refresh_continuous_state().unwrap();
    let bob_black = colored_spell(&mut game, "{B}", BOB);
    let bob_blue = colored_spell(&mut game, "{U}", BOB);
    let bob_red = colored_spell(&mut game, "{R}", BOB);
    assert!(!can_target(&game, bear, bob_black, BOB), "CR 702.11f: hexproof from black");
    assert!(!can_target(&game, bear, bob_blue, BOB), "CR 702.11f: and hexproof from blue");
    assert!(can_target(&game, bear, bob_red, BOB));
    assert_eq!(
        static_count(&game, bear, |s| s.hexproof_from_filter().is_some()),
        2,
        "CR 702.11f: 'from blue and from black' behaves as two separate hexproof abilities"
    );
    // CR 702.11e: an effect that removes hexproof also removes hexproof from.
    let source = vanilla(&mut game, "Remover", 1, 1, BOB);
    apply(&mut game, source, BOB, bear, Modification::RemoveAbility(StaticAbility::hexproof()));
    game.refresh_continuous_state().unwrap();
    assert!(
        can_target(&game, bear, bob_black, BOB) && can_target(&game, bear, bob_blue, BOB),
        "CR 702.11e: losing hexproof loses every 'hexproof from' ability"
    );
}

#[test]
fn cr_702_11e_as_though_no_hexproof_covers_hexproof_from() {
    let mut game = main_game();
    let knight = put(
        &mut game,
        &card("Black-warded knight", "Mana cost: {1}{W}\nType: Creature — Human Knight\nPower/Toughness: 2/2\nHexproof from black"),
        ALICE,
    );
    put(
        &mut game,
        &card("Glaring Spotlight", "Mana cost: {1}\nType: Artifact\nCreatures your opponents control with hexproof can be the targets of spells and abilities you control as though they didn't have hexproof.\n{3}, Sacrifice Glaring Spotlight: Creatures you control gain hexproof until end of turn and can't be blocked this turn."),
        BOB,
    );
    game.refresh_continuous_state().unwrap();
    let bob_black = colored_spell(&mut game, "{B}", BOB);
    assert!(
        can_target(&game, knight, bob_black, BOB),
        "CR 702.11e: 'as though it didn't have hexproof' lets a player target a creature with hexproof from black"
    );
}

#[test]
fn cr_702_11g_hexproof_from_each_color_is_one_ability_per_color() {
    let mut game = main_game();
    let breaker = put(
        &mut game,
        &card("Breaker of Creation", "Mana cost: {6}{C}{C}\nType: Creature — Eldrazi\nPower/Toughness: 8/4\nWhen you cast this spell, you gain 1 life for each colorless permanent you control.\nHexproof from each color\nAnnihilator 2"),
        ALICE,
    );
    game.refresh_continuous_state().unwrap();
    for cost in ["{W}", "{U}", "{B}", "{R}", "{G}"] {
        let spell = colored_spell(&mut game, cost, BOB);
        assert!(!can_target(&game, breaker, spell, BOB), "CR 702.11g: hexproof from each color covers {cost}");
    }
    let colorless = colored_spell(&mut game, "{2}", BOB);
    assert!(can_target(&game, breaker, colorless, BOB), "CR 702.11g: colorless isn't a color");
}

#[test]
fn cr_702_11h_18b_duplicate_hexproof_and_shroud_are_redundant() {
    for (keyword, instances) in [("Hexproof", 1usize), ("Hexproof", 2), ("Shroud", 1), ("Shroud", 2)] {
        let mut game = main_game();
        let text = std::iter::repeat_n(keyword, instances).collect::<Vec<_>>().join("\n");
        let subject = put(&mut game, &card("Warded", &format!("Mana cost: {{U}}\nType: Creature — Bird\nPower/Toughness: 1/1\n{text}")), ALICE);
        game.refresh_continuous_state().unwrap();
        let bob_spell = colored_spell(&mut game, "{R}", BOB);
        let alice_spell = colored_spell(&mut game, "{R}", ALICE);
        assert!(!can_target(&game, subject, bob_spell, BOB), "CR 702.11h/18b: {instances}x {keyword} vs opponent");
        assert_eq!(
            can_target(&game, subject, alice_spell, ALICE),
            keyword == "Hexproof",
            "CR 702.11h/18b: {instances}x {keyword} vs controller behaves like one instance"
        );
    }
}

#[test]
fn cr_702_11e_effect_looking_for_hexproof_finds_hexproof_from() {
    let mut game = main_game();
    put(
        &mut game,
        &card("Hexproof anthem", "Mana cost: {1}{G}\nType: Enchantment\nCreatures you control with hexproof get +1/+1."),
        ALICE,
    );
    let knight = put(
        &mut game,
        &card("Black-warded knight", "Mana cost: {1}{W}\nType: Creature — Human Knight\nPower/Toughness: 2/2\nHexproof from black"),
        ALICE,
    );
    let plain = vanilla(&mut game, "Plain bear", 2, 2, ALICE);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(plain), Some(2));
    assert_eq!(
        game.calculated_power(knight),
        Some(3),
        "CR 702.11e: an effect that looks for a creature with hexproof finds one with hexproof from black"
    );
}

// ===========================================================================
// 702.15 Lifelink
// ===========================================================================

#[test]
fn cr_702_15b_d_lifelink_pays_controller_or_owner_from_any_zone() {
    let lifelinker = card("Lifelinker", "Mana cost: {W}\nType: Creature — Cleric\nPower/Toughness: 2/2\nLifelink");
    // Controlled by a non-owner on the battlefield: the controller gains.
    let mut game = main_game();
    let source = put(&mut game, &lifelinker, ALICE);
    apply(&mut game, source, BOB, source, Modification::ChangeController(BOB));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.controller_of_id(source), Some(BOB));
    let victim = vanilla(&mut game, "Victim", 5, 5, ALICE);
    deal(&mut game, source, BOB, 2, ChooseSpec::SpecificObject(victim));
    assert_eq!((life(&game, ALICE), life(&game, BOB)), (20, 22), "CR 702.15b: the source's controller gains the life");
    // From non-battlefield zones (no controller): the owner gains.
    for zone in [Zone::Graveyard, Zone::Hand, Zone::Exile, Zone::Library] {
        let mut game = main_game();
        let source = game.create_object_from_definition(&lifelinker, ALICE, zone);
        let victim = vanilla(&mut game, "Victim", 5, 5, BOB);
        deal(&mut game, source, BOB, 2, ChooseSpec::SpecificObject(victim));
        assert_eq!(game.damage_on(victim), 2);
        assert_eq!(
            (life(&game, ALICE), life(&game, BOB)),
            (22, 20),
            "CR 702.15b/d: lifelink from {zone:?} works and its owner gains the life"
        );
    }
}

// ===========================================================================
// 702.16 Protection
// ===========================================================================

#[test]
fn cr_702_16a_card_type_protection_covers_nonbattlefield_sources_of_that_type() {
    let mut game = main_game();
    let protected = put(
        &mut game,
        &card("Artifact ward", "Mana cost: {W}\nType: Creature — Soldier\nPower/Toughness: 2/2\nProtection from artifacts"),
        ALICE,
    );
    let artifact_card = card("Rod", "Mana cost: {1}\nType: Artifact\n{T}: You gain 1 life.");
    let on_battlefield = put(&mut game, &artifact_card, BOB);
    let in_graveyard = game.create_object_from_definition(&artifact_card, BOB, Zone::Graveyard);
    let nonartifact = game.create_object_from_definition(
        &card("Plain card", "Mana cost: {R}\nType: Instant\nDraw a card."),
        BOB,
        Zone::Graveyard,
    );
    deal(&mut game, on_battlefield, BOB, 1, ChooseSpec::SpecificObject(protected));
    assert_eq!(game.damage_on(protected), 0, "CR 702.16a: an artifact permanent's damage is prevented");
    deal(&mut game, in_graveyard, BOB, 1, ChooseSpec::SpecificObject(protected));
    assert_eq!(game.damage_on(protected), 0, "CR 702.16a: an artifact card in a graveyard is an artifact source");
    deal(&mut game, nonartifact, BOB, 1, ChooseSpec::SpecificObject(protected));
    assert_eq!(game.damage_on(protected), 1, "CR 702.16a: non-artifact sources aren't covered");
}

#[test]
fn cr_702_16g_m_two_quality_protection_and_redundant_instances() {
    for (text, expected_abilities) in [
        ("Protection from red and from white", 2usize),
        ("Protection from red\nProtection from red", 2),
        ("Protection from red", 1),
    ] {
        let mut game = main_game();
        let subject = put(&mut game, &card("Protected", &format!("Mana cost: {{W}}\nType: Creature — Knight\nPower/Toughness: 2/2\n{text}")), ALICE);
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            static_count(&game, subject, StaticAbility::has_protection),
            expected_abilities,
            "CR 702.16g: '{text}' behaves as separate protection abilities"
        );
        let red = colored_spell(&mut game, "{R}", BOB);
        let white = colored_spell(&mut game, "{W}", BOB);
        let black = colored_spell(&mut game, "{B}", BOB);
        deal(&mut game, red, BOB, 1, ChooseSpec::SpecificObject(subject));
        assert_eq!(game.damage_on(subject), 0, "CR 702.16g/m: red damage prevented for '{text}'");
        assert!(!can_target(&game, subject, red, BOB));
        deal(&mut game, white, BOB, 1, ChooseSpec::SpecificObject(subject));
        let white_prevented = text.contains("white");
        assert_eq!(game.damage_on(subject), if white_prevented { 0 } else { 1 }, "CR 702.16g: white for '{text}'");
        let before = game.damage_on(subject);
        deal(&mut game, black, BOB, 1, ChooseSpec::SpecificObject(subject));
        assert_eq!(game.damage_on(subject), before + 1, "CR 702.16g/m: black isn't covered by '{text}'");
    }
}

#[test]
fn cr_702_16h_protection_from_each_color() {
    let mut game = main_game();
    let angel = put(
        &mut game,
        &card("Iridescent Angel", "Mana cost: {5}{W}{U}\nType: Creature — Angel\nPower/Toughness: 4/4\nFlying, protection from each color"),
        ALICE,
    );
    game.refresh_continuous_state().unwrap();
    for cost in ["{W}", "{U}", "{B}", "{R}", "{G}"] {
        let spell = colored_spell(&mut game, cost, BOB);
        deal(&mut game, spell, BOB, 1, ChooseSpec::SpecificObject(angel));
        assert!(!can_target(&game, angel, spell, BOB), "CR 702.16h: can't be targeted by {cost}");
    }
    assert_eq!(game.damage_on(angel), 0, "CR 702.16h: damage from each color is prevented");
    let colorless = colored_spell(&mut game, "{3}", BOB);
    deal(&mut game, colorless, BOB, 1, ChooseSpec::SpecificObject(angel));
    assert_eq!(game.damage_on(angel), 1, "CR 702.16h: colorless isn't a color");
}

#[test]
fn cr_702_16i_k_protection_from_each_opponent() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.active_player = ALICE;
    game.turn.priority_player = Some(ALICE);
    game.turn.phase = Phase::FirstMain;
    let carol = PlayerId(2);
    put(
        &mut game,
        &card("Absolute Virtue", "Mana cost: {6}{W}{U}\nType: Legendary Creature — Avatar Warrior\nPower/Toughness: 8/8\nThis spell can't be countered.\nFlying\nYou have protection from each of your opponents. (You can't be dealt damage, enchanted, or targeted by anything controlled by your opponents.)"),
        ALICE,
    );
    game.refresh_continuous_state().unwrap();
    let spec = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
    for opponent in [BOB, carol] {
        let spell = colored_spell(&mut game, "{R}", opponent);
        let legal = ironsmith::targeting::compute_legal_targets(&game, &spec, opponent, Some(spell));
        assert!(!legal.contains(&Target::Player(ALICE)), "CR 702.16i/k: opponent {opponent:?} can't target Alice");
        let source = vanilla(&mut game, "Opposing source", 1, 1, opponent);
        deal(&mut game, source, opponent, 3, ChooseSpec::Player(PlayerFilter::Specific(ALICE)));
        assert_eq!(life(&game, ALICE), 20, "CR 702.16i/k: damage from {opponent:?}'s sources is prevented");
    }
    let own = vanilla(&mut game, "Own source", 1, 1, ALICE);
    deal(&mut game, own, ALICE, 1, ChooseSpec::Player(PlayerFilter::Specific(ALICE)));
    assert_eq!(life(&game, ALICE), 19, "control: Alice's own sources aren't covered");
}

#[test]
fn cr_702_16k_protection_from_chosen_player_covers_controlled_and_owned_objects() {
    let mut game = main_game();
    let tnn = card(
        "True-Name Nemesis",
        "Mana cost: {1}{U}{U}\nType: Creature — Merfolk Rogue\nPower/Toughness: 3/1\nAs this creature enters, choose a player.\nThis creature has protection from the chosen player.",
    );
    let spell = game.create_object_from_definition(&tnn, ALICE, Zone::Stack);
    game.stack.push(StackEntry::new(spell, ALICE));
    let mut dm = Dm::new().pick("Bob");
    resolve_top_with(&mut game, &mut dm);
    let nemesis = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "True-Name Nemesis").expect("entered");
    game.remove_summoning_sickness(nemesis);
    game.refresh_continuous_state().unwrap();
    let bob_spell = colored_spell(&mut game, "{2}", BOB);
    assert!(!can_target(&game, nemesis, bob_spell, BOB), "CR 702.16k: the chosen player's spells can't target it (chosen options: {:?})", dm.seen_options);
    let alice_spell = colored_spell(&mut game, "{2}", ALICE);
    assert!(can_target(&game, nemesis, alice_spell, ALICE));
    // Damage from an object Bob owns and controls, and from an object Bob
    // owns in his graveyard (not controlled by another player).
    let bob_creature = vanilla(&mut game, "Bob creature", 2, 2, BOB);
    deal(&mut game, bob_creature, BOB, 1, ChooseSpec::SpecificObject(nemesis));
    let bob_card = game.create_object_from_definition(&card("Bob card", "Mana cost: {2}\nType: Artifact\n{T}: You gain 1 life."), BOB, Zone::Graveyard);
    deal(&mut game, bob_card, ALICE, 1, ChooseSpec::SpecificObject(nemesis));
    assert_eq!(game.damage_on(nemesis), 0, "CR 702.16k: damage from objects the chosen player controls or owns is prevented");
    // It can't be blocked by creatures the chosen player controls.
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    attack(&mut game, vec![(nemesis, AttackTarget::Player(BOB))]);
    let mut combat = game.combat.clone().unwrap();
    assert!(
        declare_blockers(&game, &mut combat, vec![(bob_creature, nemesis)]).is_err(),
        "CR 702.16k: creatures the chosen player controls can't block it"
    );
}

/// An Aura attached to `host`, controlled by `controller`, with `text`.
fn attached_aura(game: &mut GameState, name: &str, text: &str, controller: PlayerId, host: ObjectId) -> ObjectId {
    let def = card(name, text);
    let aura = game.create_object_from_definition(&def, controller, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
    aura
}

#[test]
fn cr_702_16n_doesnt_remove_auras_unless_another_protection_instance_does() {
    for intrinsic_white in [false, true] {
        let mut game = main_game();
        let host_text = if intrinsic_white { "Protection from white" } else { "" };
        let host = put(&mut game, &card("Host", &format!("Mana cost: {{G}}\nType: Creature — Bear\nPower/Toughness: 2/2\n{host_text}")), ALICE);
        let spectra = attached_aura(
            &mut game,
            "Spectra Ward",
            "Mana cost: {3}{W}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +2/+2 and has protection from each color. This effect doesn't remove Auras.",
            ALICE,
            host,
        );
        let other_white = attached_aura(
            &mut game,
            "White blessing",
            "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+1.",
            BOB,
            host,
        );
        game.refresh_continuous_state().unwrap();
        let red = colored_spell(&mut game, "{R}", BOB);
        deal(&mut game, red, BOB, 1, ChooseSpec::SpecificObject(host));
        assert_eq!(game.damage_on(host), 0, "Spectra Ward grants protection from red");
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let kept = !intrinsic_white;
        assert_eq!(
            game.battlefield.contains(&spectra),
            kept,
            "CR 702.16n: Spectra Ward stays unless another protection from white instance removes it (intrinsic={intrinsic_white})"
        );
        assert_eq!(
            game.battlefield.contains(&other_white),
            kept,
            "CR 702.16n: 'doesn't remove Auras' covers every Aura, unless another instance applies (intrinsic={intrinsic_white})"
        );
    }
}

#[test]
fn cr_702_16p_benevolent_blessing_keeps_already_attached_but_blocks_new_attachments() {
    let mut game = main_game();
    let host = vanilla(&mut game, "Host", 2, 2, ALICE);
    let old_aura = attached_aura(
        &mut game,
        "Old white aura",
        "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+1.",
        ALICE,
        host,
    );
    let bob_aura = attached_aura(
        &mut game,
        "Bob white aura",
        "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets -1/-0.",
        BOB,
        host,
    );
    let blessing = card(
        "Benevolent Blessing",
        "Mana cost: {1}{W}\nType: Enchantment — Aura\nFlash\nEnchant creature\nAs this Aura enters, choose a color.\nEnchanted creature has protection from the chosen color. This effect doesn't remove Auras and Equipment you control that are already attached to it.",
    );
    let blessing = game.create_object_from_definition(&blessing, ALICE, Zone::Stack);
    game.stack.push(StackEntry::new(blessing, ALICE).with_targets(vec![Target::Object(host)]));
    let mut dm = Dm::new().pick("White").pick("white");
    dm.color = Some(ironsmith::color::Color::White);
    resolve_top_with(&mut game, &mut dm);
    let blessing = *game
        .battlefield
        .iter()
        .find(|id| game.object(**id).unwrap().name == "Benevolent Blessing")
        .expect("Benevolent Blessing entered");
    game.refresh_continuous_state().unwrap();
    let white = colored_spell(&mut game, "{W}", BOB);
    deal(&mut game, white, BOB, 1, ChooseSpec::SpecificObject(host));
    assert_eq!(game.damage_on(host), 0, "fixture: protection from white applies");
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(game.battlefield.contains(&blessing), "CR 702.16p: the Blessing itself stays attached");
    assert!(game.battlefield.contains(&old_aura), "CR 702.16p: an already-attached white Aura you control stays attached");
    assert!(
        !game.battlefield.contains(&bob_aura),
        "CR 702.16p: only Auras and Equipment you control are exempt; an opponent's white Aura falls off"
    );
    let new_aura = attached_aura(
        &mut game,
        "New white aura",
        "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+1.",
        ALICE,
        host,
    );
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        !game.battlefield.contains(&new_aura),
        "CR 702.16p: other white permanents can't become attached to the creature"
    );
}

// ===========================================================================
// 702.19 Trample
// ===========================================================================

fn walker(game: &mut GameState, controller: PlayerId, loyalty: u32) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), "Test Walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(loyalty)
        .build();
    let id = put(game, &def, controller);
    game.object_mut(id).unwrap().counters.insert(CounterType::Loyalty, loyalty);
    id
}

fn keyword_creature(game: &mut GameState, name: &str, pt: (i32, i32), text: &str, controller: PlayerId) -> ObjectId {
    put(
        game,
        &card(name, &format!("Mana cost: {{G}}\nType: Creature — Beast\nPower/Toughness: {}/{}\n{text}", pt.0, pt.1)),
        controller,
    )
}

#[test]
fn cr_702_19a_trample_does_nothing_for_blockers_or_noncombat_damage() {
    let mut game = combat_game();
    let attacker = vanilla(&mut game, "Small attacker", 1, 1, ALICE);
    let blocker = keyword_creature(&mut game, "Trampling blocker", (6, 6), "Trample", BOB);
    attack(&mut game, vec![(attacker, AttackTarget::Player(BOB))]);
    block(&mut game, vec![(blocker, attacker)]);
    deal_combat_damage(&mut game);
    assert_eq!(life(&game, ALICE), 20, "CR 702.19a: a blocking trampler's excess damage goes nowhere");
    assert_eq!(game.damage_on(attacker), 6);
    // Noncombat damage from a trampler to a creature doesn't spill over.
    let mut game = main_game();
    let trampler = keyword_creature(&mut game, "Trampler", (6, 6), "Trample", ALICE);
    let victim = vanilla(&mut game, "Victim", 1, 1, BOB);
    deal(&mut game, trampler, ALICE, 6, ChooseSpec::SpecificObject(victim));
    assert_eq!(life(&game, BOB), 20, "CR 702.19a: trample has no effect on noncombat damage");
}

#[test]
fn cr_702_19b_g_trample_lethal_check_counts_marked_damage_and_duplicates_are_redundant() {
    for text in ["Trample", "Trample\nTrample"] {
        for marked in [0u32, 1] {
            let mut game = combat_game();
            let attacker = keyword_creature(&mut game, "Trampler", (3, 3), text, ALICE);
            let blocker = vanilla(&mut game, "Blocker", 2, 2, BOB);
            if marked > 0 {
                deal(&mut game, attacker, ALICE, marked as i32, ChooseSpec::SpecificObject(blocker));
            }
            attack(&mut game, vec![(attacker, AttackTarget::Player(BOB))]);
            block(&mut game, vec![(blocker, attacker)]);
            let prompt = next_prompt(&game).expect("blocked trampler divides damage");
            let one_over = [(Target::Object(blocker), 1), (Target::Player(BOB), 2)];
            let two_over = [(Target::Object(blocker), 2), (Target::Player(BOB), 1)];
            let all_in = [(Target::Object(blocker), 3)];
            assert_eq!(
                prompt.validate(&game, &one_over).is_ok(),
                marked == 1,
                "CR 702.19b: 1 damage is lethal only with 1 already marked (text={text:?}, marked={marked})"
            );
            assert!(prompt.validate(&game, &two_over).is_ok(), "CR 702.19b: lethal then excess to the player");
            assert!(prompt.validate(&game, &all_in).is_ok(), "CR 702.19b: excess may stay on the blocker");
        }
    }
}

#[test]
fn cr_702_19b_lethal_ignores_prevention_and_unlethal_split_bars_the_player() {
    let mut game = combat_game();
    let attacker = put(
        &mut game,
        &card("Green trampler", "Mana cost: {4}{G}{G}\nType: Creature — Beast\nPower/Toughness: 6/6\nTrample"),
        ALICE,
    );
    let pro_green = keyword_creature(&mut game, "Pro-green blocker", (2, 2), "Protection from green", BOB);
    let second = vanilla(&mut game, "Second blocker", 2, 2, BOB);
    attack(&mut game, vec![(attacker, AttackTarget::Player(BOB))]);
    block(&mut game, vec![(pro_green, attacker), (second, attacker)]);
    let prompt = next_prompt(&game).expect("division");
    assert!(
        prompt
            .validate(&game, &[(Target::Object(pro_green), 2), (Target::Object(second), 2), (Target::Player(BOB), 2)])
            .is_ok(),
        "CR 702.19b: 2 damage is lethal to the protected blocker even though it will be prevented"
    );
    assert!(
        prompt
            .validate(&game, &[(Target::Object(pro_green), 0), (Target::Object(second), 2), (Target::Player(BOB), 4)])
            .is_err(),
        "CR 702.19b: prevention doesn't change the lethal requirement"
    );
    assert!(
        prompt
            .validate(&game, &[(Target::Object(pro_green), 1), (Target::Object(second), 5)])
            .is_ok(),
        "CR 702.19b: lethal need not be assigned to every blocker when nothing goes to the player"
    );
    assert!(
        prompt
            .validate(&game, &[(Target::Object(pro_green), 1), (Target::Object(second), 4), (Target::Player(BOB), 1)])
            .is_err(),
        "CR 702.19b: without lethal on every blocker, nothing may go to the player"
    );
}

#[test]
fn cr_702_19c_f_g_trample_over_planeswalkers_and_plain_attackers_on_walkers() {
    for text in ["Trample over planeswalkers", "Trample over planeswalkers\nTrample over planeswalkers", "Trample", ""] {
        let mut game = combat_game();
        let attacker = keyword_creature(&mut game, "Attacker", (7, 7), text, ALICE);
        let pw = walker(&mut game, BOB, 3);
        let blocker = vanilla(&mut game, "Blocker", 1, 1, BOB);
        attack(&mut game, vec![(attacker, AttackTarget::Planeswalker(pw))]);
        block(&mut game, vec![(blocker, attacker)]);
        let top = text.starts_with("Trample over");
        let past_loyalty = [(Target::Object(blocker), 1), (Target::Object(pw), 3), (Target::Player(BOB), 3)];
        match next_prompt(&game) {
            Some(prompt) => {
                assert_eq!(
                    prompt.validate(&game, &past_loyalty).is_ok(),
                    top,
                    "CR 702.19c/f: past-loyalty damage may reach the controller only with trample over planeswalkers ({text:?})"
                );
                if top {
                    assert!(
                        prompt.validate(&game, &[(Target::Object(blocker), 1), (Target::Object(pw), 2), (Target::Player(BOB), 4)]).is_err(),
                        "CR 702.19c: the planeswalker must first be assigned damage equal to its loyalty"
                    );
                    prompt.record(&mut game, &past_loyalty);
                } else {
                    assert!(
                        prompt.validate(&game, &[(Target::Object(blocker), 1), (Target::Object(pw), 6)]).is_ok(),
                        "CR 702.19b: trample excess goes to the attacked planeswalker"
                    );
                    prompt.record(&mut game, &[(Target::Object(blocker), 1), (Target::Object(pw), 6)]);
                }
            }
            None => assert!(text.is_empty(), "a blocked trampler gets a division ({text:?})"),
        }
        deal_combat_damage(&mut game);
        let expected_life = if top { 17 } else { 20 };
        assert_eq!(life(&game, BOB), expected_life, "CR 702.19c/f: damage to the planeswalker's controller ({text:?})");
    }
}

#[test]
fn cr_702_19d_blocked_trampler_with_no_blockers_assigns_to_what_it_attacks() {
    for (text, expected_life) in [("Trample", 16), ("Trample over planeswalkers", 16), ("", 20)] {
        let mut game = combat_game();
        let attacker = keyword_creature(&mut game, "Attacker", (4, 4), text, ALICE);
        let blocker = vanilla(&mut game, "Blocker", 1, 1, BOB);
        attack(&mut game, vec![(attacker, AttackTarget::Player(BOB))]);
        block(&mut game, vec![(blocker, attacker)]);
        ironsmith::effects::RemoveFromCombatEffect::with_spec(ChooseSpec::SpecificObject(blocker))
            .execute(&mut game, &mut EffectContext::new_default(blocker, BOB))
            .unwrap();
        assert!(ironsmith::combat_state::is_blocked(game.combat.as_ref().unwrap(), attacker));
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, BOB), expected_life, "CR 702.19d/509.1h: blocked attacker with no blockers ({text:?})");
    }
}

#[test]
fn cr_702_19e_f_removed_planeswalker_only_trample_over_planeswalkers_reaches_player() {
    for (text, blocked, expected_life) in [
        ("Trample over planeswalkers", false, 16),
        ("Trample over planeswalkers", true, 17),
        ("Trample", false, 20),
        ("", false, 20),
    ] {
        let mut game = combat_game();
        let attacker = keyword_creature(&mut game, "Attacker", (4, 4), text, ALICE);
        let pw = walker(&mut game, BOB, 3);
        let blocker = vanilla(&mut game, "Blocker", 1, 1, BOB);
        attack(&mut game, vec![(attacker, AttackTarget::Planeswalker(pw))]);
        if blocked {
            block(&mut game, vec![(blocker, attacker)]);
        }
        ironsmith::effects::RemoveFromCombatEffect::with_spec(ChooseSpec::SpecificObject(pw))
            .execute(&mut game, &mut EffectContext::new_default(pw, BOB))
            .unwrap();
        if blocked {
            let prompt = next_prompt(&game).expect("division between blocker and player");
            prompt.record(&mut game, &[(Target::Object(blocker), 1), (Target::Player(BOB), 3)]);
        }
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, BOB), expected_life, "CR 702.19e/f: attacker {text:?} (blocked={blocked}) whose planeswalker left combat");
        assert_eq!(counters(&game, pw, CounterType::Loyalty), 3, "the removed planeswalker is dealt nothing");
        assert!(
            !matches!(
                ironsmith::combat_state::get_attack_target(game.combat.as_ref().unwrap(), attacker),
                Some(AttackTarget::Player(_))
            ),
            "CR 702.19e: it doesn't become attacking the player"
        );
    }
}

// ===========================================================================
// 702.22 Banding
// ===========================================================================

fn bander(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
    keyword_creature(game, name, (1, 1), "Banding", controller)
}

#[test]
fn cr_702_22b_losing_banding_removes_bands_with_other() {
    let mut game = main_game();
    let def = creature("Leader", 1, 1)
        .with_ability(ironsmith::Ability::static_ability(StaticAbility::bands_with_other(
            ironsmith::ObjectFilter::creature(),
            "bands with other creatures",
        )))
        .build();
    let leader = put(&mut game, &def, ALICE);
    assert_eq!(static_count(&game, leader, |s| s.bands_with_other_filter().is_some()), 1);
    apply(&mut game, leader, ALICE, leader, Modification::RemoveAbility(StaticAbility::banding()));
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        static_count(&game, leader, |s| s.bands_with_other_filter().is_some()),
        0,
        "CR 702.22b: losing banding loses every 'bands with other' ability"
    );
}

#[test]
fn cr_702_22c_d_m_band_composition_and_target_rules() {
    use ironsmith::combat_state::set_attacking_band;
    let mut game = combat_game();
    let b1 = bander(&mut game, "Bander one", ALICE);
    let b2 = bander(&mut game, "Bander two", ALICE);
    let double = keyword_creature(&mut game, "Double bander", (1, 1), "Banding\nBanding", ALICE);
    let n1 = vanilla(&mut game, "Plain one", 1, 1, ALICE);
    let n2 = vanilla(&mut game, "Plain two", 1, 1, ALICE);
    let n3 = vanilla(&mut game, "Plain three", 1, 1, ALICE);
    let pw = walker(&mut game, BOB, 5);
    attack(
        &mut game,
        vec![
            (b1, AttackTarget::Player(BOB)),
            (b2, AttackTarget::Player(BOB)),
            (double, AttackTarget::Player(BOB)),
            (n1, AttackTarget::Player(BOB)),
            (n2, AttackTarget::Player(BOB)),
            (n3, AttackTarget::Planeswalker(pw)),
        ],
    );
    let base = game.combat.clone().unwrap();
    let try_band = |members: Vec<ObjectId>| {
        let mut combat = base.clone();
        set_attacking_band(&game, &mut combat, members).is_ok()
    };
    assert!(try_band(vec![b1, b2, n1]), "CR 702.22c: banders plus one non-bander form a band");
    assert!(!try_band(vec![b1, n1, n2]), "CR 702.22c: at most one creature without banding");
    assert!(!try_band(vec![double, n1, n2]), "CR 702.22m: two banding instances still allow only one non-bander");
    assert!(!try_band(vec![b1, n3]), "CR 702.22d: band members must attack the same player/planeswalker");
    let mut combat = base.clone();
    set_attacking_band(&game, &mut combat, vec![b1, n1]).unwrap();
    assert!(
        set_attacking_band(&game, &mut combat, vec![b2, n1]).is_err(),
        "CR 702.22c: each creature may be in only one band"
    );
    set_attacking_band(&game, &mut combat, vec![b2, n2]).expect("CR 702.22c: a player may declare several bands");
    assert_eq!(combat.attacking_bands.len(), 2);
}

#[test]
fn cr_702_22c_bands_with_other_quality_forms_band_with_any_number_of_that_quality() {
    use ironsmith::combat_state::set_attacking_band;
    let mut game = combat_game();
    let mut legend_filter = ironsmith::ObjectFilter::creature();
    legend_filter.supertypes.push(ironsmith::Supertype::Legendary);
    let leader_def = CardDefinitionBuilder::new(CardId::new(), "Legend leader")
        .supertypes(vec![ironsmith::Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
        .with_ability(ironsmith::Ability::static_ability(StaticAbility::bands_with_other(legend_filter, "bands with other legends")))
        .build();
    let legend_def = |name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .supertypes(vec![ironsmith::Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
            .build()
    };
    let leader = put(&mut game, &leader_def, ALICE);
    let l1 = put(&mut game, &legend_def("Legend one"), ALICE);
    let l2 = put(&mut game, &legend_def("Legend two"), ALICE);
    let plain = vanilla(&mut game, "Nonlegend", 1, 1, ALICE);
    attack(
        &mut game,
        [leader, l1, l2, plain].into_iter().map(|id| (id, AttackTarget::Player(BOB))).collect(),
    );
    let base = game.combat.clone().unwrap();
    let mut combat = base.clone();
    assert!(
        set_attacking_band(&game, &mut combat, vec![leader, l1, l2]).is_ok(),
        "CR 702.22c: a 'bands with other legends' creature bands with any number of other legends"
    );
    let mut combat = base.clone();
    assert!(
        set_attacking_band(&game, &mut combat, vec![leader, l1, plain]).is_err(),
        "CR 702.22c: the other members must have the quality"
    );
}

#[test]
fn cr_702_22e_f_g_h_band_persists_shares_blocks_but_not_abilities() {
    use ironsmith::combat_state::set_attacking_band;
    let mut game = combat_game();
    let flyer = put(
        &mut game,
        &card("Banding flyer", "Mana cost: {W}\nType: Creature — Pegasus\nPower/Toughness: 1/2\nFlying, banding"),
        ALICE,
    );
    let swampwalker = keyword_creature(&mut game, "Swampwalker", (2, 2), "Swampwalk", ALICE);
    let third = vanilla(&mut game, "Third", 1, 1, ALICE);
    put(&mut game, &card("Swamp", "Type: Basic Land — Swamp\n({T}: Add {B}.)"), BOB);
    let blocker = keyword_creature(&mut game, "Reacher", (0, 5), "Reach", BOB);
    attack(
        &mut game,
        vec![(flyer, AttackTarget::Player(BOB)), (swampwalker, AttackTarget::Player(BOB)), (third, AttackTarget::Player(BOB))],
    );
    let mut combat = game.combat.clone().unwrap();
    set_attacking_band(&game, &mut combat, vec![flyer, swampwalker]).unwrap();
    game.combat = Some(combat);
    // CR 702.22g: no ability sharing.
    assert!(!game.object_has_ability(swampwalker, &StaticAbility::flying()), "CR 702.22g: band members don't share flying");
    // CR 702.22e: removing banding later doesn't break the band.
    apply(&mut game, flyer, ALICE, flyer, Modification::RemoveAbility(StaticAbility::banding()));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.combat.as_ref().unwrap().attacking_bands, vec![vec![flyer, swampwalker]], "CR 702.22e: the band lasts for the rest of combat");
    // CR 702.22h: blocking the flyer blocks the swampwalker too.
    block(&mut game, vec![(blocker, flyer)]);
    let combat = game.combat.clone().unwrap();
    assert!(
        ironsmith::combat_state::is_blocked(&combat, swampwalker),
        "CR 702.22h: each other band member becomes blocked by the same blocker"
    );
    assert!(!ironsmith::combat_state::is_blocked(&combat, third));
    // CR 702.22f: a member removed from combat leaves the band.
    ironsmith::effects::RemoveFromCombatEffect::with_spec(ChooseSpec::SpecificObject(swampwalker))
        .execute(&mut game, &mut EffectContext::new_default(swampwalker, ALICE))
        .unwrap();
    assert!(
        game.combat.as_ref().unwrap().attacking_bands.iter().all(|band| !band.contains(&swampwalker)),
        "CR 702.22f: a creature removed from combat is removed from its band"
    );
}

#[test]
fn cr_702_22i_effect_blocking_one_member_blocks_the_whole_band() {
    use ironsmith::combat_state::set_attacking_band;
    let mut game = combat_game();
    let lead = bander(&mut game, "Bander", ALICE);
    let partner = vanilla(&mut game, "Partner", 2, 2, ALICE);
    attack(&mut game, vec![(lead, AttackTarget::Player(BOB)), (partner, AttackTarget::Player(BOB))]);
    let mut combat = game.combat.clone().unwrap();
    set_attacking_band(&game, &mut combat, vec![lead, partner]).unwrap();
    game.combat = Some(combat);
    let source = vanilla(&mut game, "Effect source", 1, 1, BOB);
    ironsmith::effects::BecomeBlockedEffect::with_spec(ChooseSpec::SpecificObject(lead))
        .execute(&mut game, &mut EffectContext::new_default(source, BOB))
        .unwrap();
    let combat = game.combat.clone().unwrap();
    assert!(ironsmith::combat_state::is_blocked(&combat, lead), "fixture: the targeted member becomes blocked");
    assert!(
        ironsmith::combat_state::is_blocked(&combat, partner),
        "CR 702.22i: if one band member becomes blocked due to an effect, the entire band becomes blocked"
    );
}

#[test]
fn cr_702_22j_k_banding_hands_damage_assignment_to_the_other_player() {
    use ironsmith::combat_state::set_attacking_band;
    // 702.22j: an attacker blocked by a creature with banding.
    let mut game = combat_game();
    let attacker = vanilla(&mut game, "Big attacker", 4, 4, ALICE);
    let banding_blocker = bander(&mut game, "Banding blocker", BOB);
    let other_blocker = vanilla(&mut game, "Other blocker", 1, 1, BOB);
    attack(&mut game, vec![(attacker, AttackTarget::Player(BOB))]);
    block(&mut game, vec![(banding_blocker, attacker), (other_blocker, attacker)]);
    let prompt = next_prompt(&game).expect("attacker division");
    assert_eq!(prompt.source, attacker);
    assert_eq!(prompt.player, BOB, "CR 702.22j: the defending player assigns the attacker's damage");
    assert!(
        prompt.validate(&game, &[(Target::Object(banding_blocker), 4), (Target::Object(other_blocker), 0)]).is_ok(),
        "CR 702.22j: the defending player may divide it as they choose among the blockers"
    );
    // 702.22k: a blocker blocking a creature with banding.
    let mut game = combat_game();
    let lead = bander(&mut game, "Banding attacker", ALICE);
    let partner = vanilla(&mut game, "Partner", 1, 1, ALICE);
    let blocker = vanilla(&mut game, "Lone blocker", 4, 4, BOB);
    attack(&mut game, vec![(lead, AttackTarget::Player(BOB)), (partner, AttackTarget::Player(BOB))]);
    let mut combat = game.combat.clone().unwrap();
    set_attacking_band(&game, &mut combat, vec![lead, partner]).unwrap();
    game.combat = Some(combat);
    block(&mut game, vec![(blocker, lead)]);
    let mut seen_blocker_prompt = false;
    while let Some(prompt) = next_prompt(&game) {
        if prompt.source == blocker {
            seen_blocker_prompt = true;
            assert_eq!(prompt.player, ALICE, "CR 702.22k: the active player assigns the blocker's damage");
            assert!(
                prompt.validate(&game, &[(Target::Object(lead), 0), (Target::Object(partner), 4)]).is_ok(),
                "CR 702.22k: the active player may divide it as they choose"
            );
        }
        let default = prompt.default_assignment(&game);
        prompt.record(&mut game, &default);
    }
    assert!(seen_blocker_prompt, "CR 702.22h/k: the blocker blocks both band members, so its damage is divided");
}

// ===========================================================================
// Upkeep-trigger keywords: 702.24 cumulative upkeep, 702.30 echo, 702.32 fading
// ===========================================================================

/// Move to `player`'s upkeep on `turn`, fire beginning-of-upkeep triggers and
/// resolve everything.
fn run_upkeep(game: &mut GameState, player: PlayerId, turn: u32, dm: &mut Dm) -> usize {
    game.turn.turn_number = turn;
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Upkeep);
    game.mark_upkeep_began(player);
    game.refresh_continuous_state().unwrap();
    let mut queue = TriggerQueue::new();
    for event in ironsmith::generate_step_trigger_events_for_active_players(game) {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    let triggered = queue.entries.len();
    resolve_queue(game, &mut queue, dm);
    triggered
}

fn named_on_battlefield(game: &GameState, name: &str) -> Option<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).is_some_and(|object| object.name == name))
}

/// Cast `def` from Alice's hand with `mana` and resolve it, returning the permanent.
fn cast_permanent(game: &mut GameState, def: &CardDefinition, mana: &[(ManaSymbol, u32)], dm: &mut Dm) -> ObjectId {
    let spell = game.create_object_from_definition(def, ALICE, Zone::Hand);
    for (symbol, amount) in mana {
        add_mana(game, ALICE, *symbol, *amount);
    }
    cast_with(game, ALICE, spell, any_method, dm);
    resolve_all(game, dm);
    named_on_battlefield(game, &def.card.name).expect("the permanent spell resolved onto the battlefield")
}

#[test]
fn cr_702_24a_cumulative_upkeep_adds_age_counter_and_requires_full_payment() {
    let remora = card(
        "Mystic Remora",
        "Mana cost: {U}\nType: Enchantment\nCumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\nWhenever an opponent casts a noncreature spell, you may draw a card unless that player pays {4}.",
    );
    let mut game = main_game();
    let id = put(&mut game, &remora, ALICE);
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 1);
    let mut dm = Dm::new();
    assert_eq!(run_upkeep(&mut game, ALICE, 5, &mut dm), 1);
    assert!(game.battlefield.contains(&id), "CR 702.24a: paying {{1}} for one age counter keeps it");
    assert_eq!(counters(&game, id, CounterType::Age), 1, "CR 702.24a: an age counter is put on it");
    assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0);
    // Second upkeep: two age counters, only one mana available: no partial payment.
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 1);
    run_upkeep(&mut game, ALICE, 7, &mut dm);
    assert!(!game.battlefield.contains(&id), "CR 702.24a: {{1}}{{1}} can't be paid, so it's sacrificed");
    assert_eq!(
        game.player(ALICE).unwrap().mana_pool.total(),
        1,
        "CR 702.24a: partial payments aren't allowed, so the lone mana isn't spent"
    );
    // Declining to pay sacrifices it even when the cost is affordable.
    let mut game = main_game();
    let id = put(&mut game, &remora, ALICE);
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 3);
    run_upkeep(&mut game, ALICE, 5, &mut Dm::new().no());
    assert!(!game.battlefield.contains(&id), "CR 702.24a: if you don't pay, sacrifice it");
}

#[test]
fn cr_702_24b_multiple_cumulative_upkeeps_each_count_all_age_counters() {
    let mut game = main_game();
    let def = card(
        "Double upkeep",
        "Mana cost: {B}\nType: Creature — Spirit\nPower/Toughness: 2/2\nCumulative upkeep—Pay 1 life.\nCumulative upkeep—Pay 1 life.",
    );
    let id = put(&mut game, &def, ALICE);
    let triggered = run_upkeep(&mut game, ALICE, 5, &mut Dm::new());
    assert_eq!(triggered, 2, "CR 702.24b: each instance triggers separately");
    assert!(game.battlefield.contains(&id));
    assert_eq!(counters(&game, id, CounterType::Age), 2);
    assert_eq!(life(&game, ALICE), 17, "CR 702.24b: the first pays 1 life, the second pays 2 life");
}

#[test]
fn cr_702_30a_echo_triggers_only_on_the_first_upkeep_after_control() {
    for pay in [true, false] {
        let mut game = main_game();
        let def = card(
            "Echo beast",
            "Mana cost: {1}{G}\nType: Creature — Beast\nPower/Toughness: 3/3\nEcho {1}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
        );
        let id = cast_permanent(&mut game, &def, &[(ManaSymbol::Green, 2)], &mut Dm::new());
        add_mana(&mut game, ALICE, ManaSymbol::Green, 2);
        let mut dm = if pay { Dm::new() } else { Dm::new().no() };
        assert_eq!(run_upkeep(&mut game, ALICE, 5, &mut dm), 1, "CR 702.30a: echo triggers at the next upkeep");
        assert_eq!(
            game.battlefield.contains(&id),
            pay,
            "CR 702.30a: sacrifice it unless you pay (pay={pay})"
        );
        if pay {
            add_mana(&mut game, ALICE, ManaSymbol::Green, 2);
            assert_eq!(
                run_upkeep(&mut game, ALICE, 7, &mut Dm::new().no()),
                0,
                "CR 702.30a: echo doesn't trigger again once it has been under your control since your last upkeep"
            );
            assert!(game.battlefield.contains(&id));
        }
    }
}

#[test]
fn cr_702_32a_fading_enters_with_counters_and_sacrifices_when_none_remain() {
    let mut game = main_game();
    let def = card(
        "Blastoderm",
        "Mana cost: {2}{G}{G}\nType: Creature — Beast\nPower/Toughness: 5/5\nShroud (This creature can't be the target of spells or abilities.)\nFading 3 (This creature enters with three fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)",
    );
    let id = cast_permanent(&mut game, &def, &[(ManaSymbol::Green, 4)], &mut Dm::new());
    assert_eq!(counters(&game, id, CounterType::Fade), 3, "CR 702.32a: enters with N fade counters");
    for (turn, left) in [(5u32, 2u32), (7, 1), (9, 0)] {
        run_upkeep(&mut game, ALICE, turn, &mut Dm::new());
        assert!(game.battlefield.contains(&id));
        assert_eq!(counters(&game, id, CounterType::Fade), left, "CR 702.32a: remove a fade counter each upkeep");
    }
    run_upkeep(&mut game, ALICE, 11, &mut Dm::new());
    assert!(!game.battlefield.contains(&id), "CR 702.32a: if you can't remove one, sacrifice it");
}

// ===========================================================================
// Combat-trigger keywords: 702.23 rampage, 702.25 flanking, 702.39 provoke
// ===========================================================================

#[test]
fn cr_702_23a_b_rampage_counts_extra_blockers_once_on_resolution() {
    let mut game = combat_game();
    let pack = put(
        &mut game,
        &card("Wolverine Pack", "Mana cost: {2}{G}{G}\nType: Creature — Wolverine\nPower/Toughness: 2/4\nRampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)"),
        ALICE,
    );
    let blockers: Vec<_> = (0..3).map(|i| vanilla(&mut game, &format!("Blocker {i}"), 1, 1, BOB)).collect();
    let mut dm = Dm::new();
    attack_and_resolve(&mut game, &[(pack, AttackTarget::Player(BOB))], &mut dm);
    block_and_resolve(&mut game, blockers.iter().map(|b| (*b, pack)).collect(), &mut dm);
    assert_eq!(
        (game.calculated_power(pack), game.calculated_toughness(pack)),
        (Some(6), Some(8)),
        "CR 702.23a: +2/+2 for each of the two blockers beyond the first"
    );
    ironsmith::effects::RemoveFromCombatEffect::with_spec(ChooseSpec::SpecificObject(blockers[0]))
        .execute(&mut game, &mut EffectContext::new_default(blockers[0], BOB))
        .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(pack), Some(6), "CR 702.23b: removing blockers later doesn't change the bonus");
}

#[test]
fn cr_702_23c_multiple_rampage_instances_trigger_separately() {
    let mut game = combat_game();
    let beast = keyword_creature(&mut game, "Double rampager", (2, 2), "Rampage 1\nRampage 2", ALICE);
    let blockers: Vec<_> = (0..2).map(|i| vanilla(&mut game, &format!("Blocker {i}"), 1, 1, BOB)).collect();
    let mut dm = Dm::new();
    attack_and_resolve(&mut game, &[(beast, AttackTarget::Player(BOB))], &mut dm);
    block_and_resolve(&mut game, blockers.iter().map(|b| (*b, beast)).collect(), &mut dm);
    assert_eq!(game.calculated_power(beast), Some(5), "CR 702.23c: rampage 1 and rampage 2 each trigger (+1 and +2)");
}

#[test]
fn cr_702_25a_b_flanking_shrinks_nonflanking_blockers_per_instance() {
    for (text, blocker_text, expected) in [
        ("Flanking", "", Some((1, 1))),
        ("Flanking", "Flanking", Some((2, 2))),
        ("Flanking\nFlanking", "", None),
    ] {
        let mut game = combat_game();
        let lancer = keyword_creature(&mut game, "Lancer", (2, 2), text, ALICE);
        let blocker = keyword_creature(&mut game, "Blocker", (2, 2), blocker_text, BOB);
        let mut dm = Dm::new();
        attack_and_resolve(&mut game, &[(lancer, AttackTarget::Player(BOB))], &mut dm);
        block_and_resolve(&mut game, vec![(blocker, lancer)], &mut dm);
        sba(&mut game);
        match expected {
            Some((p, t)) => assert_eq!(
                (game.calculated_power(blocker), game.calculated_toughness(blocker)),
                (Some(p), Some(t)),
                "CR 702.25a/b: attacker {text:?} blocked by {blocker_text:?}"
            ),
            None => assert!(
                !game.battlefield.contains(&blocker),
                "CR 702.25b: two flanking instances each give -1/-1, killing a 2/2"
            ),
        }
    }
}

#[test]
fn cr_702_39a_b_provoke_untaps_target_and_forces_block_per_instance() {
    for text in ["Provoke", "Provoke\nProvoke"] {
        let mut game = combat_game();
        let grappler = keyword_creature(&mut game, "Grappler", (1, 1), text, ALICE);
        let target = vanilla(&mut game, "Provoked", 2, 2, BOB);
        game.tap(target);
        let mut combat = CombatState::default();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[AttackerDeclaration { creature: grappler, target: AttackTarget::Player(BOB) }],
        )
        .unwrap();
        game.combat = Some(combat);
        let expected = if text.contains('\n') { 2 } else { 1 };
        assert_eq!(queue.entries.len(), expected, "CR 702.39b: each provoke instance triggers separately");
        resolve_queue(&mut game, &mut queue, &mut Dm::new());
        assert!(!game.is_tapped(target), "CR 702.39a: the provoked creature untaps");
        let mut no_block = game.combat.clone().unwrap();
        assert!(
            declare_blockers(&game, &mut no_block, vec![]).is_err(),
            "CR 702.39a: it must block the provoking creature if able"
        );
        let mut blocks = game.combat.clone().unwrap();
        assert!(declare_blockers(&game, &mut blocks, vec![(target, grappler)]).is_ok());
    }
}

// ===========================================================================
// Casting keywords
// ===========================================================================

fn library_cards(game: &mut GameState, player: PlayerId, count: usize) {
    let filler = CardDefinitionBuilder::new(CardId::new(), "Library card")
        .card_types(vec![CardType::Sorcery])
        .build();
    for _ in 0..count {
        game.create_object_from_definition(&filler, player, Zone::Library);
    }
}

fn hand_size(game: &GameState, player: PlayerId) -> usize {
    game.player(player).unwrap().hand.len()
}

#[test]
fn cr_702_27a_buyback_returns_the_spell_to_hand_only_when_paid() {
    for buyback in [false, true] {
        let mut game = main_game();
        library_cards(&mut game, ALICE, 3);
        let def = card(
            "Whispers of the Muse",
            "Mana cost: {U}\nType: Instant\nBuyback {5} (You may pay an additional {5} as you cast this spell. If you do, put this card into your hand as it resolves.)\nDraw a card.",
        );
        let spell = game.create_object_from_definition(&def, ALICE, Zone::Hand);
        let stable = game.object(spell).unwrap().stable_id;
        add_mana(&mut game, ALICE, ManaSymbol::Blue, 6);
        let mut dm = if buyback { Dm::new().pick("Buyback") } else { Dm::new() };
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        let spent = 6 - game.player(ALICE).unwrap().mana_pool.total();
        assert_eq!(spent, if buyback { 6 } else { 1 }, "CR 702.27a: buyback is an optional additional cost (buyback={buyback})");
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            zone_of(&game, stable),
            Some(if buyback { Zone::Hand } else { Zone::Graveyard }),
            "CR 702.27a: a paid buyback puts the card into its owner's hand as it resolves"
        );
        assert_eq!(hand_size(&game, ALICE), if buyback { 2 } else { 1 }, "the spell still drew a card");
    }
}

/// Activate the first ability of `card` in Alice's hand (a cycling ability).
fn activate_from_hand(game: &mut GameState, card: ObjectId, dm: &mut Dm) -> TriggerQueue {
    game.refresh_continuous_state().unwrap();
    let action = find_activation(game, ALICE, card).expect("the hand ability is available");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..32 {
        if state.pending_activation.is_none() && state.pending_cast.is_none() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm);
    }
    assert!(result.is_ok(), "hand activation failed: {result:?}");
    assert!(state.pending_activation.is_none(), "hand activation finished");
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    queue
}

#[test]
fn cr_702_29a_b_cycling_works_only_from_hand_but_exists_everywhere() {
    let def = card(
        "Cycler",
        "Mana cost: {3}{W}\nType: Creature — Bird\nPower/Toughness: 2/2\nCycling {2} ({2}, Discard this card: Draw a card.)",
    );
    let mut game = main_game();
    library_cards(&mut game, ALICE, 3);
    let in_hand = game.create_object_from_definition(&def, ALICE, Zone::Hand);
    let stable = game.object(in_hand).unwrap().stable_id;
    let on_battlefield = put(&mut game, &def, ALICE);
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 4);
    game.refresh_continuous_state().unwrap();
    assert!(find_activation(&game, ALICE, on_battlefield).is_none(), "CR 702.29a: cycling only functions in hand");
    let activated_on_battlefield = game
        .current_characteristics(on_battlefield)
        .unwrap()
        .abilities
        .iter()
        .filter(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .count();
    assert_eq!(activated_on_battlefield, 1, "CR 702.29b: the cycling ability still exists on the battlefield");
    let mut dm = Dm::new();
    let mut queue = activate_from_hand(&mut game, in_hand, &mut dm);
    assert_eq!(zone_of(&game, stable), Some(Zone::Graveyard), "CR 702.29a: discard this card is the cost");
    resolve_queue(&mut game, &mut queue, &mut dm);
    assert_eq!(hand_size(&game, ALICE), 1, "CR 702.29a: draw a card");
    assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 2, "CR 702.29a: paid {{2}}");
}

#[test]
fn cr_702_29c_d_cycle_triggers_from_new_zone_and_cycle_or_discard_once() {
    let mut game = main_game();
    library_cards(&mut game, ALICE, 3);
    put(
        &mut game,
        &card("Drake Haven", "Mana cost: {2}{U}\nType: Enchantment\nWhenever you cycle or discard a card, you may pay {1}. If you do, create a 2/2 blue Drake creature token with flying."),
        ALICE,
    );
    let faith = card(
        "Renewed Faith",
        "Mana cost: {2}{W}\nType: Instant\nYou gain 6 life.\nCycling {1}{W} ({1}{W}, Discard this card: Draw a card.)\nWhen you cycle this card, you may gain 2 life.",
    );
    let card_id = game.create_object_from_definition(&faith, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::White, 3);
    let mut dm = Dm::new();
    let mut queue = activate_from_hand(&mut game, card_id, &mut dm);
    let named = |game: &GameState, queue: &TriggerQueue, name: &str| {
        queue.entries.iter().filter(|e| e.source_name == name).count()
            + game.stack.iter().filter(|e| e.is_ability && e.source_name.as_deref() == Some(name)).count()
    };
    assert_eq!(named(&game, &queue, "Drake Haven"), 1, "CR 702.29d: 'cycle or discard' triggers only once for a cycle");
    resolve_queue(&mut game, &mut queue, &mut dm);
    assert_eq!(hand_size(&game, ALICE), 1, "fixture: the cycling ability resolved and drew a card");
    assert_eq!(life(&game, ALICE), 22, "CR 702.29c: 'when you cycle this card' triggers from the graveyard the card wound up in, and resolves");
}

#[test]
fn cr_702_29e_f_typecycling_searches_and_is_cycling() {
    let abomination = card(
        "Twisted Abomination",
        "Mana cost: {5}{B}\nType: Creature — Zombie Mutant\nPower/Toughness: 5/3\n{B}: Regenerate this creature.\nSwampcycling {2} ({2}, Discard this card: Search your library for a Swamp card, reveal it, put it into your hand, then shuffle.)",
    );
    let swamp = card("Swamp", "Type: Basic Land — Swamp\n({T}: Add {B}.)");
    // Search + 'cycle or discard' trigger.
    let mut game = main_game();
    library_cards(&mut game, ALICE, 3);
    let swamp_id = game.create_object_from_definition(&swamp, ALICE, Zone::Library);
    let swamp_stable = game.object(swamp_id).unwrap().stable_id;
    put(
        &mut game,
        &card("Drake Haven", "Mana cost: {2}{U}\nType: Enchantment\nWhenever you cycle or discard a card, you may pay {1}. If you do, create a 2/2 blue Drake creature token with flying."),
        ALICE,
    );
    let abom = game.create_object_from_definition(&abomination, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 2);
    let mut dm = Dm::new().prefer("Swamp").no();
    let mut queue = activate_from_hand(&mut game, abom, &mut dm);
    let haven = queue.entries.iter().filter(|e| e.source_name == "Drake Haven").count()
        + game.stack.iter().filter(|e| e.is_ability && e.source_name.as_deref() == Some("Drake Haven")).count();
    assert_eq!(haven, 1, "CR 702.29f: cycling triggers fire for typecycling");
    resolve_queue(&mut game, &mut queue, &mut dm);
    assert_eq!(zone_of(&game, swamp_stable), Some(Zone::Hand), "CR 702.29e: search for a Swamp card and put it into your hand");
}

// ===========================================================================
// 702.33 Kicker
// ===========================================================================

#[test]
fn cr_702_33a_d_kicker_is_optional_additional_cost_and_marks_spell_kicked() {
    for kicked in [false, true] {
        let mut game = main_game();
        let bolt = card(
            "Burst Lightning",
            "Mana cost: {R}\nType: Instant\nKicker {4} (You may pay an additional {4} as you cast this spell.)\nBurst Lightning deals 2 damage to any target. If this spell was kicked, it deals 4 damage instead.",
        );
        let spell = game.create_object_from_definition(&bolt, ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Red, 5);
        let victim = vanilla(&mut game, "Victim", 0, 10, BOB);
        let mut dm = if kicked { Dm::new().pick("Kicker").prefer("Victim") } else { Dm::new().prefer("Victim") };
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        let spent = 5 - game.player(ALICE).unwrap().mana_pool.total();
        assert_eq!(spent, if kicked { 5 } else { 1 }, "CR 702.33a: kicker is an optional additional cost (kicked={kicked})");
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.damage_on(victim), if kicked { 4 } else { 2 }, "CR 702.33d: the spell was kicked={kicked}");
    }
}

#[test]
fn cr_702_33b_e_f_two_kicker_costs_each_link_to_their_own_ability() {
    for (picks, expect_damage, expect_destroy) in [
        (vec!["Kicker {R}"], true, false),
        (vec!["Kicker {W}"], false, true),
        (vec!["Kicker {R}", "Kicker {W}"], true, true),
        (vec![], false, false),
    ] {
        let mut game = main_game();
        let mage = card(
            "Thornscape Battlemage",
            "Mana cost: {2}{G}\nType: Creature — Elf Wizard\nPower/Toughness: 2/2\nKicker {R} and/or {W} (You may pay an additional {R} and/or {W} as you cast this spell.)\nWhen this creature enters, if it was kicked with its {R} kicker, it deals 2 damage to any target.\nWhen this creature enters, if it was kicked with its {W} kicker, destroy target artifact.",
        );
        let artifact = put(&mut game, &card("Bob relic", "Mana cost: {1}\nType: Artifact\n{T}: You gain 1 life."), BOB);
        let spell = game.create_object_from_definition(&mage, ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Green, 3);
        add_mana(&mut game, ALICE, ManaSymbol::Red, 1);
        add_mana(&mut game, ALICE, ManaSymbol::White, 1);
        let mut dm = Dm::new();
        for pick in &picks {
            dm.optional.push(pick);
        }
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        let spent = 5 - game.player(ALICE).unwrap().mana_pool.total();
        assert_eq!(spent as usize, 3 + picks.len(), "CR 702.33b: 'Kicker {{R}} and/or {{W}}' is two kicker costs ({picks:?}, options seen: {:?})", dm.seen_options);
        resolve_all(&mut game, &mut dm);
        let damaged = life(&game, BOB) < 20 || life(&game, ALICE) < 20
            || game.battlefield.iter().any(|id| game.damage_on(*id) > 0);
        assert_eq!(damaged, expect_damage, "CR 702.33f: the {{R}}-kicker ability is linked to the {{R}} kicker ({picks:?})");
        assert_eq!(
            !game.battlefield.contains(&artifact),
            expect_destroy,
            "CR 702.33e/f: the {{W}}-kicker ability is linked to the {{W}} kicker ({picks:?})"
        );
    }
}

#[test]
fn cr_702_33c_d_multikicker_may_be_paid_any_number_of_times() {
    for times in [0usize, 1, 3] {
        let mut game = main_game();
        let chalice = card(
            "Everflowing Chalice",
            "Mana cost: {0}\nType: Artifact\nMultikicker {2} (You may pay an additional {2} any number of times as you cast this spell.)\nThis artifact enters with a charge counter on it for each time it was kicked.\n{T}: Add {C} for each charge counter on this artifact.",
        );
        let spell = game.create_object_from_definition(&chalice, ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Colorless, 6);
        let mut dm = Dm::new();
        if times > 0 {
            dm = dm.pick("Multikicker");
            dm.optional_times = times;
        }
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        resolve_all(&mut game, &mut dm);
        let id = named_on_battlefield(&game, "Everflowing Chalice").expect("resolved");
        assert_eq!(
            counters(&game, id, CounterType::Charge),
            times as u32,
            "CR 702.33c/d: multikicker paid {times} time(s) kicks it that many times (options seen: {:?})",
            dm.seen_options
        );
        assert_eq!(game.player(ALICE).unwrap().mana_pool.total() as usize, 6 - 2 * times);
    }
}

#[test]
fn cr_702_33g_kicked_only_targets_are_chosen_only_when_kicked() {
    let mut game = main_game();
    let barrage = card(
        "Goblin Barrage",
        "Mana cost: {3}{R}\nType: Sorcery\nKicker—Sacrifice an artifact or Goblin. (You may sacrifice an artifact or Goblin in addition to any other costs as you cast this spell.)\nGoblin Barrage deals 4 damage to target creature. If this spell was kicked, it also deals 4 damage to target player or planeswalker.",
    );
    let victim = vanilla(&mut game, "Victim", 1, 5, BOB);
    let spell = game.create_object_from_definition(&barrage, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::Red, 4);
    let mut dm = Dm::new().prefer("Victim");
    cast_with(&mut game, ALICE, spell, any_method, &mut dm);
    let entry = game.stack.last().unwrap().clone();
    assert_eq!(
        entry.targets.len(),
        1,
        "CR 702.33g: an unkicked spell is cast as if it didn't have the kicked-only target ({:?})",
        entry.targets
    );
    resolve_all(&mut game, &mut dm);
    assert_eq!(game.damage_on(victim), 4);
    assert_eq!(life(&game, BOB), 20, "CR 702.33g: the kicked-only part does nothing");
}

#[test]
fn cr_702_33h_sticker_kicker_is_kicker_plus_ticket_counter_as_a_cost() {
    let mut game = main_game();
    put(
        &mut game,
        &card(
            "Wicker Picker",
            "Mana cost: {3}\nType: Artifact Creature — Scarecrow Guest\nPower/Toughness: 2/3\nCreature spells you cast have sticker kicker {1}. (You may pay an additional {1} as you cast a creature spell. If you do, you get {TK}, then you may put a sticker on it.)",
        ),
        ALICE,
    );
    let kicked_bear = card(
        "Kicked bear",
        "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/2\nWhen this creature enters, if it was kicked, you gain 3 life.",
    );
    let spell = game.create_object_from_definition(&kicked_bear, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::Green, 2);
    let mut dm = Dm::new().pick("Sticker kicker").pick("Additional").no();
    cast_with(&mut game, ALICE, spell, any_method, &mut dm);
    assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0, "fixture: sticker kicker {{1}} was paid ({:?})", dm.seen_options);
    assert_eq!(
        game.player(ALICE).unwrap().counter_count(CounterType::Named("ticket".into())),
        1,
        "CR 702.33h: getting the ticket counter is part of the additional cost paid while casting"
    );
    resolve_all(&mut game, &mut dm);
    assert_eq!(life(&game, ALICE), 23, "CR 702.33h: sticker kicker means kicker, so the spell was kicked");
}

// ===========================================================================
// 702.34 Flashback, 702.35 Madness
// ===========================================================================

#[test]
fn cr_702_34a_flashback_casts_from_graveyard_and_exiles() {
    let mut game = main_game();
    library_cards(&mut game, ALICE, 3);
    let def = card(
        "Think Twice",
        "Mana cost: {1}{U}\nType: Instant\nDraw a card.\nFlashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
    );
    let spell = game.create_object_from_definition(&def, ALICE, Zone::Graveyard);
    let stable = game.object(spell).unwrap().stable_id;
    add_mana(&mut game, ALICE, ManaSymbol::Blue, 2);
    assert!(find_cast(&game, ALICE, spell, any_method).is_none(), "CR 702.34a: the flashback cost {{2}}{{U}} must be paid");
    add_mana(&mut game, ALICE, ManaSymbol::Blue, 1);
    let mut dm = Dm::new();
    cast_with(&mut game, ALICE, spell, |_, zone| zone == Zone::Graveyard, &mut dm);
    assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0, "CR 702.34a: pays the flashback cost rather than the mana cost");
    resolve_all(&mut game, &mut dm);
    assert_eq!(hand_size(&game, ALICE), 1);
    assert_eq!(zone_of(&game, stable), Some(Zone::Exile), "CR 702.34a: a flashed-back spell is exiled instead of going anywhere else");
}

#[test]
fn cr_702_35a_b_c_madness_exiles_then_casts_for_madness_cost_or_goes_to_graveyard() {
    for cast in [true, false] {
        let mut game = main_game();
        let temper = card(
            "Fiery Temper",
            "Mana cost: {1}{R}{R}\nType: Instant\nFiery Temper deals 3 damage to any target.\nMadness {R} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)",
        );
        let card_id = game.create_object_from_definition(&temper, ALICE, Zone::Hand);
        let stable = game.object(card_id).unwrap().stable_id;
        add_mana(&mut game, ALICE, ManaSymbol::Red, 1);
        let source = vanilla(&mut game, "Discarder", 1, 1, ALICE);
        game.push_to_stack(StackEntry::ability(source, ALICE, vec![Effect::discard(1)]));
        let mut dm = if cast { Dm::new() } else { Dm::new().no() };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(zone_of(&game, stable), Some(Zone::Exile), "CR 702.35a: discarded into exile instead of the graveyard");
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        flush_triggers(&mut game, &mut queue, &mut dm);
        assert_eq!(game.stack.len(), 1, "CR 702.35a: the madness trigger");
        resolve_all(&mut game, &mut dm);
        if cast {
            assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0, "CR 702.35b: paid the madness cost {{R}}");
            assert!(life(&game, BOB) == 17 || life(&game, ALICE) == 17 || game.battlefield.iter().any(|id| game.damage_on(*id) == 3), "CR 702.35a: the madness-cast spell resolved");
            assert_eq!(zone_of(&game, stable), Some(Zone::Graveyard));
        } else {
            assert_eq!(
                zone_of(&game, stable),
                Some(Zone::Graveyard),
                "CR 702.35a/c: if not cast, the card is put into its owner's graveyard"
            );
            assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 1);
        }
    }
}

// ===========================================================================
// 702.37 Morph
// ===========================================================================

fn face_down_method(method: &CastingMethod, _: Zone) -> bool {
    format!("{method:?}").contains("Morph") || format!("{method:?}").contains("FaceDown")
}

#[test]
fn cr_702_37a_c_d_morph_casts_a_nameless_2_2_face_down_for_three() {
    let mut game = main_game();
    let def = card(
        "Morph beast",
        "Mana cost: {4}{G}{G}\nType: Creature — Beast\nPower/Toughness: 6/6\nTrample\nMorph {2}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)",
    );
    let plain = card("Plain beast", "Mana cost: {1}\nType: Creature — Beast\nPower/Toughness: 1/1");
    let spell = game.create_object_from_definition(&def, ALICE, Zone::Hand);
    let plain_spell = game.create_object_from_definition(&plain, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 3);
    let methods: Vec<String> = compute_legal_actions(&game, ALICE)
        .unwrap()
        .into_iter()
        .filter_map(|a| match a {
            LegalAction::CastSpell { spell_id, casting_method, .. } if spell_id == plain_spell => Some(format!("{casting_method:?}")),
            _ => None,
        })
        .collect();
    assert!(
        methods.iter().all(|m| !m.contains("Morph") && !m.contains("FaceDown")),
        "CR 702.37d: a card without morph can't be cast face down ({methods:?})"
    );
    let action = compute_legal_actions(&game, ALICE)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == spell && face_down_method(casting_method, Zone::Hand)))
        .expect("CR 702.37a: morph offers a face-down cast for {3}");
    let mut dm = Dm::new();
    let mut queue = TriggerQueue::new();
    assert!(act(&mut game, &mut queue, ALICE, action, &mut dm));
    assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0, "CR 702.37c: pay {{3}} rather than its mana cost");
    let on_stack = game.stack.last().unwrap().object_id;
    let chars = game.current_characteristics(on_stack).unwrap();
    assert_eq!((chars.power, chars.toughness), (Some(2), Some(2)), "CR 702.37c: the face-down spell is 2/2");
    assert!(chars.subtypes.is_empty(), "CR 702.37c: no subtypes");
    resolve_all(&mut game, &mut dm);
    let permanent = *game.battlefield.iter().find(|id| game.is_face_down(**id)).expect("face-down permanent");
    let chars = game.current_characteristics(permanent).unwrap();
    assert_eq!((chars.power, chars.toughness), (Some(2), Some(2)), "CR 702.37c: enters with the same characteristics");
    assert!(chars.abilities.is_empty(), "CR 702.37c: no text");
    assert!(chars.mana_cost.as_ref().is_none_or(|cost| cost.mana_value() == 0), "CR 702.37c: no mana cost");
}

#[test]
fn cr_702_37f_morph_x_value_is_available_to_other_abilities() {
    let mut game = main_game();
    let def = card(
        "Warbreak Trumpeter",
        "Mana cost: {R}\nType: Creature — Goblin\nPower/Toughness: 1/1\nMorph {X}{X}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)\nWhen this creature is turned face up, create X 1/1 red Goblin creature tokens.",
    );
    let spell = game.create_object_from_definition(&def, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 3);
    let action = compute_legal_actions(&game, ALICE)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == spell && face_down_method(casting_method, Zone::Hand)))
        .expect("face-down cast");
    let mut dm = Dm::new();
    let mut queue = TriggerQueue::new();
    act(&mut game, &mut queue, ALICE, action, &mut dm);
    resolve_all(&mut game, &mut dm);
    let permanent = *game.battlefield.iter().find(|id| game.is_face_down(**id)).expect("face-down permanent");
    add_mana(&mut game, ALICE, ManaSymbol::Red, 5);
    let turn_up = compute_legal_actions(&game, ALICE)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == permanent))
        .expect("turning face up is a special action");
    dm.x = 2;
    let tokens_before = game.battlefield.len();
    act(&mut game, &mut queue, ALICE, turn_up, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0, "fixture: paid {{X}}{{X}}{{R}} with X=2");
    assert_eq!(
        game.battlefield.len() - tokens_before,
        2,
        "CR 702.37f: the face-up trigger's X equals the X chosen for the morph cost"
    );
}

// ===========================================================================
// 702.38 Amplify
// ===========================================================================

#[test]
fn cr_702_38a_b_amplify_reveals_matching_cards_per_instance() {
    for (text, revealed_beasts, expected) in [("Amplify 1", 2usize, 2u32), ("Amplify 1\nAmplify 2", 1, 3)] {
        let mut game = main_game();
        let crawler = card(
            "Amplifier",
            &format!("Mana cost: {{3}}{{G}}\nType: Creature — Beast\nPower/Toughness: 2/2\n{text}"),
        );
        let beast = card("Hand beast", "Mana cost: {G}\nType: Creature — Beast\nPower/Toughness: 1/1");
        let elf = card("Hand elf", "Mana cost: {G}\nType: Creature — Elf\nPower/Toughness: 1/1");
        for _ in 0..revealed_beasts {
            game.create_object_from_definition(&beast, ALICE, Zone::Hand);
        }
        game.create_object_from_definition(&elf, ALICE, Zone::Hand);
        let mut dm = Dm::new().prefer("Hand beast");
        let id = cast_permanent(&mut game, &crawler, &[(ManaSymbol::Green, 4)], &mut dm);
        assert_eq!(
            counters(&game, id, CounterType::PlusOnePlusOne),
            expected,
            "CR 702.38a/b: {text:?} revealing {revealed_beasts} Beast(s)"
        );
    }
}

// ===========================================================================
// 702.40 Storm, 702.41 Affinity, 702.42 Entwine
// ===========================================================================

fn filler_spell(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let def = card("Filler", "Mana cost: {R}\nType: Instant\nYou gain 1 life.");
    game.create_object_from_definition(&def, owner, Zone::Hand)
}

#[test]
fn cr_702_40a_b_storm_copies_for_each_earlier_spell_per_instance() {
    for (text, expected_copies) in [("Storm", 2usize), ("Storm\nStorm", 4)] {
        let mut game = main_game();
        add_mana(&mut game, ALICE, ManaSymbol::Red, 10);
        add_mana(&mut game, BOB, ManaSymbol::Red, 10);
        for player in [BOB, ALICE] {
            let spell = filler_spell(&mut game, player);
            cast_with(&mut game, player, spell, any_method, &mut Dm::new());
            resolve_all(&mut game, &mut Dm::new());
        }
        game.turn.priority_player = Some(ALICE);
        let shot = card("Stormer", &format!("Mana cost: {{1}}{{R}}\nType: Sorcery\nThis spell deals 1 damage to target player.\n{text}"));
        let spell = game.create_object_from_definition(&shot, ALICE, Zone::Hand);
        let mut dm = Dm::new();
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        let triggers = game.stack.iter().filter(|e| e.is_ability).count();
        assert_eq!(triggers, text.lines().count(), "CR 702.40b: each storm instance triggers separately");
        let before = life(&game, BOB) + life(&game, ALICE);
        resolve_all(&mut game, &mut dm);
        let dealt = before - (life(&game, BOB) + life(&game, ALICE));
        assert_eq!(
            dealt as usize,
            expected_copies + 1,
            "CR 702.40a: {text:?} copies once per earlier spell this turn (both players') per instance"
        );
    }
}

#[test]
fn cr_702_41a_b_affinity_reduces_per_object_and_instances_stack() {
    for (text, artifacts, mana_needed) in [
        ("Affinity for artifacts", 4usize, 0u32),
        ("Affinity for artifacts", 2, 2),
        ("Affinity for artifacts\nAffinity for artifacts", 2, 0),
    ] {
        let def = card("Affinity frog", &format!("Mana cost: {{4}}\nType: Artifact Creature — Frog\nPower/Toughness: 2/2\n{text}"));
        let relic = card("Relic", "Mana cost: {1}\nType: Artifact\n{T}: You gain 1 life.");
        let castable_with = |mana: u32| {
            let mut game = main_game();
            for _ in 0..artifacts {
                put(&mut game, &relic, ALICE);
            }
            let spell = game.create_object_from_definition(&def, ALICE, Zone::Hand);
            add_mana(&mut game, ALICE, ManaSymbol::Colorless, mana);
            game.refresh_continuous_state().unwrap();
            find_cast(&game, ALICE, spell, any_method).is_some()
        };
        assert!(castable_with(mana_needed), "CR 702.41a/b: {text:?} with {artifacts} artifacts costs {mana_needed}");
        if mana_needed > 0 {
            assert!(!castable_with(mana_needed - 1), "CR 702.41a: reduction is exactly {{1}} per artifact");
        }
    }
}

#[test]
fn cr_702_42a_b_entwine_chooses_all_modes_and_follows_printed_order() {
    for entwined in [false, true] {
        let mut game = main_game();
        library_cards(&mut game, ALICE, 10);
        let promise = card(
            "Promise of Power",
            "Mana cost: {2}{B}{B}{B}\nType: Sorcery\nChoose one —\n• You draw five cards and you lose 5 life.\n• Create an X/X black Demon creature token with flying, where X is the number of cards in your hand.\nEntwine {4} (Choose both if you pay the entwine cost.)",
        );
        let spell = game.create_object_from_definition(&promise, ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Black, 9);
        let mut dm = if entwined { Dm::new().pick("Entwine") } else { Dm::new() };
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        let spent = 9 - game.player(ALICE).unwrap().mana_pool.total();
        assert_eq!(spent, if entwined { 9 } else { 5 }, "CR 702.42a: entwine is an additional cost (entwined={entwined}, options {:?})", dm.seen_options);
        resolve_all(&mut game, &mut dm);
        if entwined {
            assert_eq!(hand_size(&game, ALICE), 5, "CR 702.42a: the draw mode happened");
            let demon = game
                .battlefield
                .iter()
                .copied()
                .find(|id| game.object(*id).unwrap().name.contains("Demon"))
                .expect("CR 702.42a: the token mode happened too");
            assert_eq!(game.calculated_power(demon), Some(5), "CR 702.42b: modes follow printed order, so X counts the five drawn cards");
        } else {
            assert_eq!(hand_size(&game, ALICE), 5, "the first mode alone");
            assert!(game.battlefield.iter().all(|id| !game.object(*id).unwrap().name.contains("Demon")));
        }
    }
}

// ===========================================================================
// 702.44 Sunburst
// ===========================================================================

#[test]
fn cr_702_44a_b_d_sunburst_counts_colors_spent_on_entering_spell() {
    let oracle = card(
        "Etched Oracle",
        "Mana cost: {4}\nType: Artifact Creature — Wizard\nPower/Toughness: 0/0\nSunburst (This creature enters with a +1/+1 counter on it for each color of mana spent to cast it.)\n{1}, Remove four +1/+1 counters from this creature: Target player draws three cards.",
    );
    let prism = card(
        "Pentad Prism",
        "Mana cost: {2}\nType: Artifact\nSunburst (This artifact enters with a charge counter on it for each color of mana spent to cast it.)\nRemove a charge counter from this artifact: Add one mana of any color.",
    );
    let double = card(
        "Double sun",
        "Mana cost: {3}\nType: Artifact Creature — Golem\nPower/Toughness: 0/0\nSunburst\nSunburst",
    );
    let mut game = main_game();
    let id = cast_permanent(&mut game, &oracle, &[(ManaSymbol::White, 1), (ManaSymbol::Blue, 1), (ManaSymbol::Black, 1), (ManaSymbol::Red, 1)], &mut Dm::new());
    assert_eq!(counters(&game, id, CounterType::PlusOnePlusOne), 4, "CR 702.44a: a creature gets a +1/+1 counter per color spent");
    let mut game = main_game();
    let id = cast_permanent(&mut game, &prism, &[(ManaSymbol::White, 1), (ManaSymbol::White, 0), (ManaSymbol::Green, 1)], &mut Dm::new());
    assert_eq!(counters(&game, id, CounterType::Charge), 2, "CR 702.44a: a noncreature gets charge counters");
    let mut game = main_game();
    let id = cast_permanent(&mut game, &prism, &[(ManaSymbol::Colorless, 2)], &mut Dm::new());
    assert_eq!(counters(&game, id, CounterType::Charge), 0, "CR 702.44b: no colored mana spent, no counters");
    let mut game = main_game();
    let id = game.create_object_from_definition(&oracle, ALICE, Zone::Hand);
    let _ = game.move_object_with_etb_processing(id, Zone::Battlefield).expect("put onto the battlefield");
    let moved = named_on_battlefield(&game, "Etched Oracle").expect("entered");
    assert_eq!(counters(&game, moved, CounterType::PlusOnePlusOne), 0, "CR 702.44b: not entering from the stack as a resolving spell");
    let mut game = main_game();
    let id = cast_permanent(&mut game, &double, &[(ManaSymbol::White, 1), (ManaSymbol::Blue, 1), (ManaSymbol::Black, 1)], &mut Dm::new());
    assert_eq!(counters(&game, id, CounterType::PlusOnePlusOne), 6, "CR 702.44d: each sunburst instance works separately");
}

#[test]
fn cr_702_44c_modular_sunburst_sets_the_counter_number() {
    let mut game = main_game();
    let wanderer = card(
        "Arcbound Wanderer",
        "Mana cost: {6}\nType: Artifact Creature — Golem\nPower/Toughness: 0/0\nModular—Sunburst (This creature enters with a +1/+1 counter on it for each color of mana spent to cast it. When it dies, you may put its +1/+1 counters on target artifact creature.)",
    );
    let id = cast_permanent(
        &mut game,
        &wanderer,
        &[(ManaSymbol::White, 1), (ManaSymbol::Blue, 1), (ManaSymbol::Black, 1), (ManaSymbol::Colorless, 3)],
        &mut Dm::new(),
    );
    assert_eq!(counters(&game, id, CounterType::PlusOnePlusOne), 3, "CR 702.44c: Modular—Sunburst enters with a counter per color spent");
}

// ===========================================================================
// 702.21 Ward
// ===========================================================================

#[test]
fn cr_702_21a_ward_counters_opponents_spell_unless_cost_paid() {
    for (bob_mana, pays, countered) in [(1u32, false, true), (3, true, false), (3, false, true)] {
        let mut game = main_game();
        let warded = keyword_creature(&mut game, "Warded", (2, 2), "Ward {2}", ALICE);
        game.turn.priority_player = Some(BOB);
        let shock = card("Shock", "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target.");
        let spell = game.create_object_from_definition(&shock, BOB, Zone::Hand);
        add_mana(&mut game, BOB, ManaSymbol::Red, bob_mana);
        let mut dm = Dm::new().prefer("Warded");
        dm.yes = pays;
        cast_with(&mut game, BOB, spell, any_method, &mut dm);
        assert!(game.stack.iter().any(|e| e.is_ability), "CR 702.21a: ward triggers when it becomes an opponent's target");
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            game.damage_on(warded) == 0,
            countered,
            "CR 702.21a: Bob mana={bob_mana}, pays={pays}"
        );
    }
    // Its controller's own spells don't trigger ward.
    let mut game = main_game();
    keyword_creature(&mut game, "Warded", (2, 2), "Ward {2}", ALICE);
    let shock = card("Shock", "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target.");
    let spell = game.create_object_from_definition(&shock, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::Red, 1);
    cast_with(&mut game, ALICE, spell, any_method, &mut Dm::new().prefer("Warded"));
    assert!(!game.stack.iter().any(|e| e.is_ability), "CR 702.21a: only opponents' spells trigger ward");
}

#[test]
fn cr_702_21b_ward_x_is_determined_on_resolution() {
    let mut game = main_game();
    let minthara = put(
        &mut game,
        &card(
            "Minthara, Merciless Soul",
            "Mana cost: {2}{W}{B}\nType: Legendary Creature — Elf Cleric\nPower/Toughness: 2/2\nMinthara has ward {X}, where X is the number of experience counters you have.\nAt the beginning of your end step, if a permanent you controlled left the battlefield this turn, you get an experience counter.\nCreatures you control get +1/+0 for each experience counter you have.",
        ),
        ALICE,
    );
    game.player_mut(ALICE).unwrap().experience_counters = 3;
    game.refresh_continuous_state().unwrap();
    game.turn.priority_player = Some(BOB);
    let shock = card("Shock", "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target.");
    let spell = game.create_object_from_definition(&shock, BOB, Zone::Hand);
    add_mana(&mut game, BOB, ManaSymbol::Red, 2);
    let mut dm = Dm::new().prefer("Minthara, Merciless Soul");
    cast_with(&mut game, BOB, spell, any_method, &mut dm);
    assert!(game.stack.iter().any(|e| e.is_ability), "ward {{X}} triggered");
    // Before the ward trigger resolves, X drops to 1: Bob can pay {1}.
    game.player_mut(ALICE).unwrap().experience_counters = 1;
    game.refresh_continuous_state().unwrap();
    resolve_all(&mut game, &mut dm);
    assert_eq!(
        game.damage_on(minthara),
        2,
        "CR 702.21b: X is determined as the ward ability resolves (X=1, Bob had {{1}} left)"
    );
}

// ===========================================================================
// 702.1a / 702.1c
// ===========================================================================

#[test]
fn cr_702_1a_scavenge_cost_equal_to_mana_cost_is_only_the_variable_part() {
    let mut game = main_game();
    put(
        &mut game,
        &card(
            "Varolz, the Scar-Striped",
            "Mana cost: {1}{B}{G}\nType: Legendary Creature — Troll Warrior\nPower/Toughness: 2/2\nEach creature card in your graveyard has scavenge. The scavenge cost is equal to its mana cost. (Exile a creature card from your graveyard and pay its mana cost: Put a number of +1/+1 counters equal to that card's power on target creature. Scavenge only as a sorcery.)\nSacrifice another creature: Regenerate Varolz.",
        ),
        ALICE,
    );
    let dead = card("Dead ogre", "Mana cost: {2}{R}\nType: Creature — Ogre\nPower/Toughness: 3/3");
    let grave = game.create_object_from_definition(&dead, ALICE, Zone::Graveyard);
    let stable = game.object(grave).unwrap().stable_id;
    let host = vanilla(&mut game, "Host", 1, 1, ALICE);
    add_mana(&mut game, ALICE, ManaSymbol::Red, 1);
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 1);
    game.refresh_continuous_state().unwrap();
    assert!(find_activation(&game, ALICE, grave).is_none(), "CR 702.1a: the scavenge cost is the card's whole mana cost ({{2}}{{R}})");
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 1);
    game.refresh_continuous_state().unwrap();
    let action = find_activation(&game, ALICE, grave).expect("CR 702.1a: scavenge is activatable for its mana cost");
    let mut dm = Dm::new().prefer("Host");
    let mut queue = TriggerQueue::new();
    act(&mut game, &mut queue, ALICE, action, &mut dm);
    assert_eq!(zone_of(&game, stable), Some(Zone::Exile), "CR 702.1a: the activation cost still includes exiling the card");
    assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0);
    resolve_all(&mut game, &mut dm);
    assert_eq!(counters(&game, host, CounterType::PlusOnePlusOne), 3);
}

#[test]
fn cr_702_1c_same_is_true_for_grants_every_variant_of_the_keyword() {
    let mut game = main_game();
    put(
        &mut game,
        &card(
            "Concerted Effort",
            "Mana cost: {2}{W}{W}\nType: Enchantment\nAt the beginning of each upkeep, creatures you control gain flying until end of turn if a creature you control has flying. The same is true for fear, first strike, double strike, landwalk, protection, trample, and vigilance.",
        ),
        ALICE,
    );
    let pro_red = keyword_creature(&mut game, "Pro red", (1, 1), "Protection from red", ALICE);
    let pro_blue = keyword_creature(&mut game, "Pro blue", (1, 1), "Protection from blue", ALICE);
    let swampwalk = keyword_creature(&mut game, "Swampwalker", (1, 1), "Swampwalk", ALICE);
    let plain = vanilla(&mut game, "Plain", 1, 1, ALICE);
    run_upkeep(&mut game, ALICE, 5, &mut Dm::new());
    let red = colored_spell(&mut game, "{R}", BOB);
    let blue = colored_spell(&mut game, "{U}", BOB);
    for id in [pro_red, pro_blue, swampwalk, plain] {
        assert!(
            !can_target(&game, id, red, BOB) && !can_target(&game, id, blue, BOB),
            "CR 702.1c: each protection variant is granted to each creature"
        );
        assert!(
            game.object_has_ability(id, &StaticAbility::landwalk(ironsmith::types::Subtype::Swamp)),
            "CR 702.1c: each landwalk variant is granted"
        );
    }
}

#[test]
fn cr_702_29f_cycling_cost_reduction_reduces_typecycling() {
    let abomination = card(
        "Twisted Abomination",
        "Mana cost: {5}{B}\nType: Creature — Zombie Mutant\nPower/Toughness: 5/3\n{B}: Regenerate this creature.\nSwampcycling {2} ({2}, Discard this card: Search your library for a Swamp card, reveal it, put it into your hand, then shuffle.)",
    );
    let fluctuator = try_card(
        "Fluctuator",
        "Mana cost: {2}\nType: Artifact\nCycling abilities you activate cost up to {2} less to activate.",
    );
    assert!(
        fluctuator.is_some(),
        "CR 702.29f: an effect that reduces cycling costs (Fluctuator) must be supported so it can reduce typecycling costs"
    );
    let mut game = main_game();
    library_cards(&mut game, ALICE, 3);
    put(&mut game, &fluctuator.unwrap(), ALICE);
    let abom = game.create_object_from_definition(&abomination, ALICE, Zone::Hand);
    game.refresh_continuous_state().unwrap();
    assert!(
        find_activation(&game, ALICE, abom).is_some(),
        "CR 702.29f: a cycling cost reduction reduces a typecycling cost ({{2}} payable with no mana)"
    );
}

#[test]
fn cr_702_29f_cant_cycle_stops_typecycling() {
    let abomination = card(
        "Twisted Abomination",
        "Mana cost: {5}{B}\nType: Creature — Zombie Mutant\nPower/Toughness: 5/3\n{B}: Regenerate this creature.\nSwampcycling {2} ({2}, Discard this card: Search your library for a Swamp card, reveal it, put it into your hand, then shuffle.)",
    );
    // An effect that stops cycling stops typecycling.
    let mut game = main_game();
    library_cards(&mut game, ALICE, 3);
    put(
        &mut game,
        &card("Stabilizer", "Mana cost: {2}\nType: Artifact\nPlayers can't cycle cards."),
        BOB,
    );
    let abom = game.create_object_from_definition(&abomination, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 2);
    game.refresh_continuous_state().unwrap();
    let plain_cycler = game.create_object_from_definition(
        &card("Plain cycler", "Mana cost: {3}{W}\nType: Creature — Bird\nPower/Toughness: 2/2\nCycling {2} ({2}, Discard this card: Draw a card.)"),
        ALICE,
        Zone::Hand,
    );
    game.refresh_continuous_state().unwrap();
    let plain_blocked = find_activation(&game, ALICE, plain_cycler).is_none();
    assert!(
        find_activation(&game, ALICE, abom).is_none(),
        "CR 702.29f: 'can't cycle' stops typecycling (ordinary cycling blocked: {plain_blocked})"
    );
}

// ===========================================================================
// 702.26 Phasing
// ===========================================================================

fn untap_for(game: &mut GameState, player: PlayerId, turn: u32) {
    game.turn.turn_number = turn;
    game.turn.active_player = player;
    game.turn.priority_player = None;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Untap);
    ironsmith::execute_untap_step(game);
    game.refresh_continuous_state().unwrap();
}

#[test]
fn cr_702_26e_resolved_effects_skip_phased_out_permanents() {
    let mut game = main_game();
    let phased = vanilla(&mut game, "Phased", 2, 2, ALICE);
    let present = vanilla(&mut game, "Present", 2, 2, ALICE);
    game.phase_out(phased);
    let pump = card("Pump all", "Mana cost: {G}\nType: Sorcery\nCreatures you control get +1/+1 until end of turn.");
    let spell = game.create_object_from_definition(&pump, ALICE, Zone::Stack);
    game.stack.push(StackEntry::new(spell, ALICE));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    apply(&mut game, present, ALICE, phased, Modification::ModifyPowerToughness { power: 5, toughness: 5 });
    apply(&mut game, present, BOB, phased, Modification::ChangeController(BOB));
    game.phase_in(phased);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(present), Some(3), "fixture: the pump applies to phased-in creatures");
    let controller = game.controller_of_id(phased);
    assert_eq!(
        game.calculated_power(phased),
        Some(2),
        "CR 702.26e: a phased-out permanent isn't included in a resolved effect's affected set, even when named specifically (controller after phasing in: {controller:?})"
    );
    assert_eq!(controller, Some(ALICE), "CR 702.26e: control-changing effects also skip it");
}

#[test]
fn cr_702_26m_p_phasing_skipped_untap_and_redundant_instances() {
    for text in ["Phasing", "Phasing\nPhasing"] {
        let mut game = main_game();
        let id = keyword_creature(&mut game, "Phaser", (2, 2), text, ALICE);
        untap_for(&mut game, ALICE, 5);
        assert!(game.is_phased_out(id), "CR 702.26p: {text:?} phases out once (not out and back in)");
        untap_for(&mut game, ALICE, 7);
        assert!(!game.is_phased_out(id), "CR 702.26p: {text:?} phases back in at the next untap step");
    }
    let mut game = main_game();
    let id = keyword_creature(&mut game, "Phaser", (2, 2), "Phasing", ALICE);
    put(&mut game, &card("Stasis", "Mana cost: {1}{U}\nType: Enchantment\nPlayers skip their untap steps.\nAt the beginning of your upkeep, sacrifice Stasis unless you pay {U}."), BOB);
    game.refresh_continuous_state().unwrap();
    untap_for(&mut game, ALICE, 5);
    assert!(!game.is_phased_out(id), "CR 702.26m: a skipped untap step has no phasing event");
}

#[test]
fn cr_702_26k_phased_out_permanents_leave_with_their_owner_without_zone_change_triggers() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.active_player = ALICE;
    game.turn.phase = Phase::FirstMain;
    let carol = PlayerId(2);
    put(
        &mut game,
        &card("Leave watcher", "Mana cost: {W}\nType: Enchantment\nWhenever a creature leaves the battlefield, you gain 1 life."),
        ALICE,
    );
    let carols = vanilla(&mut game, "Carol phased", 2, 2, carol);
    let stable = game.object(carols).unwrap().stable_id;
    game.phase_out(carols);
    let _ = game.take_pending_trigger_events();
    game.leave_game(carol).unwrap();
    assert!(
        zone_of(&game, stable).is_none() && !game.battlefield.contains(&carols),
        "CR 702.26k: the phased-out permanent leaves the game with its owner"
    );
    let mut queue = TriggerQueue::new();
    flush_triggers(&mut game, &mut queue, &mut Dm::new());
    resolve_all(&mut game, &mut Dm::new());
    assert_eq!(life(&game, ALICE), 20, "CR 702.26k: leaving the game doesn't cause zone-change abilities to trigger");
}

#[test]
fn cr_702_26n_permanent_phased_out_under_departed_controller_phases_in_after_their_turn_slot() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.turn_number = 2;
    game.turn.active_player = BOB;
    game.turn.phase = Phase::FirstMain;
    let carol = PlayerId(2);
    let carol_source = vanilla(&mut game, "Carol source", 1, 1, carol);
    let borrowed = vanilla(&mut game, "Borrowed", 2, 2, ALICE);
    ApplyContinuousEffect::new(EffectTarget::Specific(borrowed), Modification::ChangeController(carol), Until::Forever)
        .execute(&mut game, &mut EffectContext::new_default(carol_source, carol))
        .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.controller_of_id(borrowed), Some(carol));
    game.phase_out(borrowed);
    game.leave_game(carol).unwrap();
    game.refresh_continuous_state().unwrap();
    assert!(game.object(borrowed).is_some(), "fixture: Alice still owns the phased-out permanent");
    // Carol's turn (turn 3) would have begun; the next untap step is Alice's turn 4.
    untap_for(&mut game, ALICE, 4);
    assert!(
        !game.is_phased_out(borrowed),
        "CR 702.26n: it phases in during the next untap step after the departed controller's next turn would have begun"
    );
}

#[test]
fn cr_702_29d_cycle_or_discard_triggers_once_per_cycle() {
    let mut game = main_game();
    library_cards(&mut game, ALICE, 3);
    put(
        &mut game,
        &card("Drake Haven", "Mana cost: {2}{U}\nType: Enchantment\nWhenever you cycle or discard a card, you may pay {1}. If you do, create a 2/2 blue Drake creature token with flying."),
        ALICE,
    );
    let cycler = game.create_object_from_definition(
        &card("Plain cycler", "Mana cost: {3}{W}\nType: Creature — Bird\nPower/Toughness: 2/2\nCycling {2} ({2}, Discard this card: Draw a card.)"),
        ALICE,
        Zone::Hand,
    );
    add_mana(&mut game, ALICE, ManaSymbol::Colorless, 2);
    let mut dm = Dm::new().no();
    let queue = activate_from_hand(&mut game, cycler, &mut dm);
    let haven = queue.entries.iter().filter(|e| e.source_name == "Drake Haven").count()
        + game.stack.iter().filter(|e| e.is_ability && e.source_name.as_deref() == Some("Drake Haven")).count();
    assert_eq!(haven, 1, "CR 702.29d: 'cycles or discards' triggers only once when a card is cycled");
}

#[test]
fn zz_debug_gh_sticker() {
    let mut game = main_game();
    put(&mut game, &card("Wicker Picker", "Mana cost: {3}\nType: Artifact Creature — Scarecrow Guest\nPower/Toughness: 2/3\nCreature spells you cast have sticker kicker {1}."), ALICE);
    let kicked_bear = card("Kicked bear", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/2\nWhen this creature enters, if it was kicked, you gain 3 life.");
    eprintln!("DBG bear abilities {:?}", kicked_bear.abilities.iter().map(|a| format!("{:?}", a.kind).chars().take(1500).collect::<String>()).collect::<Vec<_>>());
    let spell = game.create_object_from_definition(&kicked_bear, ALICE, Zone::Hand);
    add_mana(&mut game, ALICE, ManaSymbol::Green, 2);
    let mut dm = Dm::new().pick("Sticker kicker").pick("Additional").no();
    cast_with(&mut game, ALICE, spell, any_method, &mut dm);
    for e in &game.stack { eprintln!("DBG stack paid={:?}", game.object(e.object_id).map(|o| o.optional_costs_paid.clone())); }
    resolve_all(&mut game, &mut dm);
}
