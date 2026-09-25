//! Regression tests for the casting/costs mechanics audit (ledger_casting.md):
//! additional and alternative costs (C1) and casting from other zones (C2).
#![allow(clippy::too_many_arguments)]

use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::PowerToughness;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::{Phase, StackEntry, Target};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::effects::EffectExecutor as _;
use ironsmith::{CardType, Effect, GameState, ObjectId, PlayerId, Zone};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

fn defs(name: &str) -> Vec<CardDefinition> {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap_or_else(|error| panic!("{name}: {error}"))
    .iter()
    .map(|payload| ironsmith_tools::compile_definition_from_payload(payload).unwrap())
    .collect()
}

fn def(name: &str) -> CardDefinition {
    defs(name).remove(0)
}

fn game(players: usize) -> GameState {
    let names = ["Alice", "Bob", "Carol", "Dave"];
    let mut game = GameState::new(
        names[..players].iter().map(|name| name.to_string()).collect(),
        20,
    );
    game.turn.turn_number = 3;
    game.turn.active_player = ALICE;
    game.turn.priority_player = Some(ALICE);
    game.turn.phase = Phase::FirstMain;
    game
}

fn add_mana(game: &mut GameState, player: PlayerId, symbol: ManaSymbol, amount: u32) {
    game.player_mut(player).unwrap().mana_pool.add(symbol, amount);
}

fn pool(game: &GameState, player: PlayerId) -> u32 {
    game.player(player).unwrap().mana_pool.total()
}

fn fixture(game: &mut GameState, name: &str, types: Vec<CardType>, owner: PlayerId, zone: Zone) -> ObjectId {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(types.clone());
    if types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    game.create_object_from_definition(&builder.build(), owner, zone)
}

/// A `{R}` instant with no effect, for filling the cast history.
fn filler(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .build();
    game.create_object_from_definition(&def, owner, Zone::Hand)
}

/// Delegates to SelectFirst, but picks optional costs whose description
/// starts with one of `optional`, answers booleans with `yes`, prefers
/// objects named `prefer`, and fixes X.
struct Dm {
    optional: Vec<&'static str>,
    yes: bool,
    prefer: Vec<&'static str>,
    x: u32,
    seen_options: Vec<String>,
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
            first: SelectFirstDecisionMaker,
        }
    }
    fn pick(mut self, label: &'static str) -> Self {
        self.optional.push(label);
        self
    }
    fn prefer(mut self, name: &'static str) -> Self {
        self.prefer.push(name);
        self
    }
    fn no(mut self) -> Self {
        self.yes = false;
        self
    }
}

impl DecisionMaker for Dm {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.yes
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value { self.x.clamp(ctx.min, ctx.max) } else { self.first.decide_number(game, ctx) }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        for name in &self.prefer {
            if let Some(candidate) = ctx.candidates.iter().find(|c| c.name == *name && c.legal) {
                return vec![candidate.id];
            }
        }
        self.first.decide_objects(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.seen_options
            .extend(ctx.options.iter().map(|option| format!("{} legal={}", option.description, option.legal)));
        let picked: Vec<usize> = ctx
            .options
            .iter()
            .filter(|option| option.legal && self.optional.iter().any(|label| option.description.starts_with(label)))
            .map(|option| option.index)
            .collect();
        if picked.len() >= ctx.min && ctx.description.starts_with("Choose optional costs") {
            return picked;
        }
        if !picked.is_empty() && picked.len() >= ctx.min {
            return picked;
        }
        self.first.decide_options(game, ctx)
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut out = Vec::new();
        for requirement in &ctx.requirements {
            let preferred = requirement.legal_targets.iter().find(|target| match target {
                Target::Object(id) => game
                    .object(*id)
                    .is_some_and(|object| self.prefer.iter().any(|name| object.name.as_str() == *name)),
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
        if out.is_empty() { self.first.decide_targets(game, ctx) } else { out }
    }
}

fn find_cast(game: &GameState, player: PlayerId, spell: ObjectId, pred: impl Fn(&CastingMethod, Zone) -> bool) -> Option<LegalAction> {
    compute_legal_actions(game, player).into_iter().find(|action| {
        matches!(action, LegalAction::CastSpell { spell_id, from_zone, casting_method }
            if *spell_id == spell && pred(casting_method, *from_zone))
    })
}

fn can_cast(game: &GameState, player: PlayerId, spell: ObjectId, pred: impl Fn(&CastingMethod, Zone) -> bool) -> bool {
    find_cast(game, player, spell, pred).is_some()
}

fn any_method(_: &CastingMethod, _: Zone) -> bool {
    true
}

fn alternative(method: &CastingMethod, _: Zone) -> bool {
    matches!(method, CastingMethod::Alternative(_))
}

/// Takes a priority action for `player` and answers decisions with `dm`
/// until the stack grows; then puts pending triggers on the stack.
fn act(game: &mut GameState, queue: &mut TriggerQueue, player: PlayerId, action: LegalAction, dm: &mut Dm) {
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
    for _ in 0..24 {
        if game.stack.len() > before || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(game, queue, &mut state, &ctx, dm);
    }
    assert!(game.stack.len() > before, "the action didn't reach the stack: {result:?}");
    flush_triggers(game, queue, dm);
}

fn cast_with(game: &mut GameState, player: PlayerId, spell: ObjectId, pred: impl Fn(&CastingMethod, Zone) -> bool, dm: &mut Dm) {
    let action = find_cast(game, player, spell, pred).expect("spell is castable");
    let mut queue = TriggerQueue::new();
    act(game, &mut queue, player, action, dm);
}

fn cast(game: &mut GameState, player: PlayerId, spell: ObjectId) {
    cast_with(game, player, spell, any_method, &mut Dm::new());
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

fn resolve_top(game: &mut GameState) {
    resolve_top_with(game, &mut Dm::new());
}

fn resolve_all(game: &mut GameState, dm: &mut Dm) {
    for _ in 0..32 {
        if game.stack.is_empty() {
            return;
        }
        resolve_top_with(game, dm);
    }
    panic!("stack didn't empty");
}

fn zone_of(game: &GameState, stable: ironsmith::ids::StableId) -> Zone {
    game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone
}

/// Resolves an ability that runs `effects` for `controller`.
fn run_ability(game: &mut GameState, controller: PlayerId, effects: Vec<Effect>, dm: &mut Dm) {
    let source = fixture(game, "Effect source", vec![CardType::Enchantment], controller, Zone::Battlefield);
    game.push_to_stack(StackEntry::ability(source, controller, effects));
    resolve_top_with(game, dm);
}

// ---------------------------------------------------------------- C2 B1/B2
mod storm {
    use super::*;

    /// Bob casts `bob`, Alice `alice` spells, then Alice casts Grapeshot and
    /// responds to storm with `responses` spells. Returns the copies made.
    fn copies(bob: usize, alice: usize, responses: usize) -> usize {
        let mut game = game(2);
        for player in [ALICE, BOB] {
            add_mana(&mut game, player, ManaSymbol::Red, 10);
        }
        for (player, count) in [(BOB, bob), (ALICE, alice)] {
            for _ in 0..count {
                let spell = filler(&mut game, player);
                cast(&mut game, player, spell);
                resolve_top(&mut game);
            }
        }
        let grapeshot = game.create_object_from_definition(&def("Grapeshot"), ALICE, Zone::Hand);
        cast(&mut game, ALICE, grapeshot);
        assert_eq!(game.stack.len(), 2, "Grapeshot and its storm trigger");
        for _ in 0..responses {
            let spell = filler(&mut game, ALICE);
            cast(&mut game, ALICE, spell);
            resolve_top(&mut game);
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Dm::new()).unwrap();
        game.stack.len() - 1
    }

    #[test]
    fn storm_counts_every_players_spells() {
        assert_eq!(copies(2, 0, 0), 2);
        assert_eq!(copies(1, 1, 0), 2);
    }

    #[test]
    fn storm_ignores_spells_cast_after_the_storm_spell() {
        assert_eq!(copies(0, 1, 2), 1);
        assert_eq!(copies(1, 0, 1), 1);
    }
}

// ---------------------------------------------------------------- C1 A3
mod copies_after_the_original_leaves {
    use super::*;

    #[test]
    fn storm_still_copies_a_countered_spell() {
        let mut game = game(2);
        add_mana(&mut game, ALICE, ManaSymbol::Red, 10);
        let first = filler(&mut game, ALICE);
        cast(&mut game, ALICE, first);
        resolve_top(&mut game);
        let grapeshot = game.create_object_from_definition(&def("Grapeshot"), ALICE, Zone::Hand);
        cast(&mut game, ALICE, grapeshot);
        assert_eq!(game.stack.len(), 2);
        let spell = game.stack[0].object_id;
        run_ability(
            &mut game,
            BOB,
            vec![Effect::counter(ironsmith::target::ChooseSpec::SpecificObject(spell))],
            &mut Dm::new(),
        );
        assert_eq!(game.stack.len(), 1, "only the storm trigger remains");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Dm::new()).unwrap();
        assert_eq!(game.stack.len(), 1, "the storm copy is still created from last known information");
        let copy = game.stack[0].object_id;
        assert_eq!(game.object(copy).unwrap().name.as_str(), "Grapeshot");
    }
}

// ---------------------------------------------------------------- C2 D1
mod cost_ordering {
    use super::*;

    fn effective_cost(artifacts: usize, tax: Option<&str>, spell: &str) -> String {
        let mut game = game(2);
        for _ in 0..artifacts {
            fixture(&mut game, "Trinket", vec![CardType::Artifact], ALICE, Zone::Battlefield);
        }
        if let Some(tax) = tax {
            game.create_object_from_definition(&def(tax), BOB, Zone::Battlefield);
        }
        let spell = game.create_object_from_definition(&def(spell), ALICE, Zone::Hand);
        game.refresh_continuous_state();
        let object = game.object(spell).unwrap().clone();
        ironsmith::decision::calculate_effective_mana_cost(&game, ALICE, &object, object.mana_cost.as_ref().unwrap())
            .to_oracle()
    }

    #[test]
    fn surplus_affinity_absorbs_a_battlefield_tax() {
        assert_eq!(effective_cost(6, Some("Thalia, Guardian of Thraben"), "Thoughtcast"), "{U}");
        assert_eq!(effective_cost(4, Some("Thalia, Guardian of Thraben"), "Thoughtcast"), "{1}{U}");
        let frogmite = effective_cost(5, Some("Sphere of Resistance"), "Frogmite");
        assert!(matches!(frogmite.as_str(), "{0}" | ""), "{frogmite}");
    }
}

// ---------------------------------------------------------------- C1 G1
mod effect_reduction_before_minimum {
    use super::*;

    #[test]
    fn an_effects_cost_reduction_applies_before_trinisphere() {
        let mut game = game(2);
        game.create_object_from_definition(&def("Trinisphere"), BOB, Zone::Battlefield);
        let spell = filler(&mut game, ALICE);
        let spell = game.move_object_by_effect(spell, Zone::Exile).unwrap();
        add_mana(&mut game, ALICE, ManaSymbol::Red, 5);
        let source = fixture(&mut game, "Cast source", vec![CardType::Artifact], ALICE, Zone::Battlefield);
        let mut entry = StackEntry::ability(
            source,
            ALICE,
            vec![Effect::new(
                ironsmith::effects::CastTaggedEffect::new("cast_me", ironsmith::target::PlayerFilter::You)
                    .cost_reduction(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]])),
            )],
        );
        entry.tagged_objects.insert(
            "cast_me".into(),
            vec![ironsmith::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game)],
        );
        game.push_to_stack(entry);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Dm::new()).unwrap();
        assert_eq!(game.stack.len(), 1, "the spell was cast");
        assert_eq!(pool(&game, ALICE), 2, "{{R}} - {{1}} = {{0}}, raised to Trinisphere's three");
    }
}

// ---------------------------------------------------------------- C2 A1/A2
mod madness {
    use super::*;

    fn setup() -> (GameState, ObjectId) {
        let mut game = game(2);
        let card = game.create_object_from_definition(&def("Fiery Temper"), ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Red, 4);
        add_mana(&mut game, BOB, ManaSymbol::Red, 4);
        (game, card)
    }

    #[test]
    fn madness_is_a_trigger_and_the_spell_is_cast_onto_the_stack() {
        let (mut game, card) = setup();
        let stable = game.object(card).unwrap().stable_id;
        let source = fixture(&mut game, "Discarder", vec![CardType::Artifact], ALICE, Zone::Battlefield);
        game.push_to_stack(StackEntry::ability(source, ALICE, vec![Effect::discard(1)]));
        let mut dm = Dm::new().prefer("Bob");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(zone_of(&game, stable), Zone::Exile, "discarded into exile");
        assert_eq!(game.player(BOB).unwrap().life, 20, "nothing resolved inside the discard");
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        flush_triggers(&mut game, &mut queue, &mut dm);
        assert_eq!(game.stack.len(), 1, "the madness trigger");
        assert!(game.stack[0].is_ability);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "Fiery Temper waits on the stack");
        assert_eq!(zone_of(&game, stable), Zone::Stack);
        assert!(!game.stack[0].is_ability);
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(ALICE), 1, "a real cast");
        assert!(game.stack[0].optional_costs_paid.was_paid_label("Madness")
            || game.object(game.stack[0].object_id).unwrap().optional_costs_paid.was_paid_label("Madness")
            || matches!(game.stack[0].casting_method, CastingMethod::Alternative(_)));
        assert_eq!(pool(&game, ALICE), 3, "paid {{R}}");
    }

    #[test]
    fn a_madness_card_exiled_another_way_is_not_castable() {
        let (mut game, card) = setup();
        let exiled = game.move_object_by_effect(card, Zone::Exile).unwrap();
        assert!(!can_cast(&game, ALICE, exiled, any_method));
        game.turn.priority_player = Some(BOB);
        assert!(!can_cast(&game, BOB, exiled, any_method));
    }
}

// ---------------------------------------------------------------- C2 A3 / A7
mod cycling {
    use super::*;

    fn cycle(game: &mut GameState, card: ObjectId) -> TriggerQueue {
        let filler = CardDefinitionBuilder::new(CardId::new(), "Library Card")
            .card_types(vec![CardType::Sorcery])
            .build();
        for _ in 0..3 {
            game.create_object_from_definition(&filler, ALICE, Zone::Library);
        }
        add_mana(game, ALICE, ManaSymbol::White, 3);
        add_mana(game, ALICE, ManaSymbol::Green, 3);
        game.refresh_continuous_state();
        let action = compute_legal_actions(game, ALICE)
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == card))
            .expect("cycling is available");
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = Dm::new();
        let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
            game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        );
        for _ in 0..16 {
            if !game.stack.is_empty() || result.is_err() {
                break;
            }
            let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
                break;
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm);
        }
        ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
        queue
    }

    fn triggers_from(game: &GameState, queue: &TriggerQueue, name: &str) -> usize {
        let from_queue = queue.entries.iter().filter(|entry| entry.source_name == name).count();
        let on_stack = game
            .stack
            .iter()
            .filter(|entry| entry.is_ability && entry.source_name.as_deref() == Some(name))
            .count();
        from_queue + on_stack
    }

    #[test]
    fn cycle_or_discard_triggers_once_per_cycle() {
        let mut game = game(2);
        game.create_object_from_definition(&def("Drake Haven"), ALICE, Zone::Battlefield);
        let card = game.create_object_from_definition(&def("Renewed Faith"), ALICE, Zone::Hand);
        let queue = cycle(&mut game, card);
        assert_eq!(triggers_from(&game, &queue, "Drake Haven"), 1);
    }

    #[test]
    fn a_cycle_trigger_fires_when_the_card_ends_up_in_exile() {
        let mut game = game(2);
        game.create_object_from_definition(&def("Rest in Peace"), BOB, Zone::Battlefield);
        let card = game.create_object_from_definition(&def("Renewed Faith"), ALICE, Zone::Hand);
        let stable = game.object(card).unwrap().stable_id;
        let queue = cycle(&mut game, card);
        assert_eq!(zone_of(&game, stable), Zone::Exile);
        assert_eq!(triggers_from(&game, &queue, "Renewed Faith"), 1, "when you cycle this card");
    }
}

// ---------------------------------------------------------------- C2 C2C-1..3
mod foretell {
    use super::*;
    use ironsmith::special_actions::SpecialAction;

    fn setup() -> (GameState, ObjectId, ObjectId) {
        let mut game = game(2);
        let def = def("Saw It Coming");
        let first = game.create_object_from_definition(&def, ALICE, Zone::Hand);
        let second = game.create_object_from_definition(&def, ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Blue, 10);
        add_mana(&mut game, BOB, ManaSymbol::Blue, 10);
        (game, first, second)
    }

    fn foretell(game: &mut GameState, card_id: ObjectId) -> Result<ObjectId, String> {
        ironsmith::special_actions::perform(SpecialAction::Foretell { card_id }, game, ALICE, &mut SelectFirstDecisionMaker)
            .map_err(|error| format!("{error:?}"))?;
        Ok(*game.exile.last().unwrap())
    }

    fn target_spell(game: &mut GameState) {
        let id = fixture(game, "Target Spell", vec![CardType::Sorcery], BOB, Zone::Stack);
        game.push_to_stack(StackEntry::new(id, BOB));
    }

    fn from_exile(method: &CastingMethod, zone: Zone) -> bool {
        zone == Zone::Exile && matches!(method, CastingMethod::Alternative(_))
    }

    #[test]
    fn a_foretold_card_is_castable_only_after_this_turn() {
        let (mut game, first, _) = setup();
        let foretold = foretell(&mut game, first).unwrap();
        target_spell(&mut game);
        assert!(!can_cast(&game, ALICE, foretold, from_exile));
        game.turn.turn_number += 1;
        assert!(can_cast(&game, ALICE, foretold, from_exile));
    }

    #[test]
    fn a_player_may_foretell_several_cards_in_a_turn() {
        let (mut game, first, second) = setup();
        foretell(&mut game, first).unwrap();
        foretell(&mut game, second).expect("a second foretell the same turn");
        assert_eq!(game.exile.len(), 2);
    }

    #[test]
    fn only_the_owner_may_cast_a_foretold_card() {
        let (mut game, first, _) = setup();
        let foretold = foretell(&mut game, first).unwrap();
        target_spell(&mut game);
        game.turn.turn_number += 1;
        game.turn.active_player = BOB;
        game.turn.priority_player = Some(BOB);
        assert!(!can_cast(&game, BOB, foretold, from_exile));
    }
}

// ---------------------------------------------------------------- C2 A4/A5
mod aftermath {
    use super::*;

    fn setup(zone: Zone) -> (GameState, ObjectId) {
        let mut game = game(2);
        let defs = defs("Cut // Ribbons");
        for def in &defs {
            game.register_linked_face_definition(def);
        }
        let cut = defs.iter().find(|def| def.card.name == "Cut").unwrap();
        let card = game.create_object_from_definition(cut, ALICE, zone);
        fixture(&mut game, "Bear", vec![CardType::Creature], BOB, Zone::Battlefield);
        add_mana(&mut game, ALICE, ManaSymbol::Black, 4);
        add_mana(&mut game, ALICE, ManaSymbol::Red, 2);
        game.refresh_continuous_state();
        (game, card)
    }

    fn other_half(method: &CastingMethod, _: Zone) -> bool {
        matches!(method, CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. })
    }

    #[test]
    fn ribbons_cannot_be_cast_from_hand() {
        let (game, card) = setup(Zone::Hand);
        assert!(can_cast(&game, ALICE, card, any_method), "Cut is castable");
        assert!(!can_cast(&game, ALICE, card, other_half));
    }

    #[test]
    fn only_ribbons_is_castable_from_the_graveyard() {
        let (game, card) = setup(Zone::Graveyard);
        assert!(can_cast(&game, ALICE, card, other_half));
        assert!(!can_cast(&game, ALICE, card, |method, zone| !other_half(method, zone)));
    }

    #[test]
    fn a_countered_ribbons_is_exiled() {
        let (mut game, card) = setup(Zone::Graveyard);
        let stable = game.object(card).unwrap().stable_id;
        let mut dm = Dm::new();
        dm.x = 1;
        cast_with(&mut game, ALICE, card, other_half, &mut dm);
        let spell = game.stack.last().unwrap().object_id;
        run_ability(
            &mut game,
            BOB,
            vec![Effect::counter(ironsmith::target::ChooseSpec::SpecificObject(spell))],
            &mut Dm::new(),
        );
        assert_eq!(zone_of(&game, stable), Zone::Exile);
    }
}

// ---------------------------------------------------------------- C2 A6 / B3
mod linked_face_mana_value {
    use super::*;

    fn mana_value(game: &GameState, id: ObjectId) -> u32 {
        ironsmith::ObjectSnapshot::from_object(game.object(id).unwrap(), game).mana_value()
    }

    #[test]
    fn a_transformed_permanent_has_its_front_faces_mana_value() {
        let mut game = game(2);
        let defs = defs("Delver of Secrets // Insectile Aberration");
        for def in &defs {
            game.register_linked_face_definition(def);
        }
        let front = defs.iter().find(|def| def.card.name == "Delver of Secrets").unwrap();
        let delver = game.create_object_from_definition(front, ALICE, Zone::Battlefield);
        assert!(game.transform_permanent(delver));
        assert_eq!(game.object(delver).unwrap().name.as_str(), "Insectile Aberration");
        assert_eq!(mana_value(&game, delver), 1);
    }

    #[test]
    fn a_split_card_outside_the_stack_has_both_halves_mana_value() {
        let mut game = game(2);
        let defs = defs("Fire // Ice");
        for def in &defs {
            game.register_linked_face_definition(def);
        }
        let fire = defs.iter().find(|def| def.card.name == "Fire").unwrap();
        let card = game.create_object_from_definition(fire, ALICE, Zone::Library);
        assert_eq!(mana_value(&game, card), 4);
    }
}

// ---------------------------------------------------------------- C2 B4
mod discover {
    use super::*;

    #[test]
    fn a_discovered_card_whose_cast_fails_goes_to_hand() {
        let mut game = game(2);
        fixture(&mut game, "Filler Land", vec![CardType::Land], ALICE, Zone::Library);
        // Shatter: "Destroy target artifact." There are no artifacts.
        let hit = game.create_object_from_definition(&def("Shatter"), ALICE, Zone::Library);
        let stable = game.object(hit).unwrap().stable_id;
        run_ability(&mut game, ALICE, vec![Effect::discover(3)], &mut Dm::new());
        assert_eq!(zone_of(&game, stable), Zone::Hand);
    }
}

// ---------------------------------------------------------------- C2 C2C-4
mod granted_suspend {
    use super::*;

    #[test]
    fn the_synthetic_suspend_permission_does_not_follow_the_card() {
        let mut game = game(2);
        let card = fixture(&mut game, "Big Creature", vec![CardType::Creature], ALICE, Zone::Exile);
        game.object_mut(card).unwrap().alternative_casts.push(
            ironsmith::AlternativeCastingMethod::Suspend { cost: ManaCost::new(), time: 0 },
        );
        let on_stack = game.move_object_by_effect(card, Zone::Stack).unwrap();
        let in_graveyard = game.move_object_by_effect(on_stack, Zone::Graveyard).unwrap();
        let back_in_hand = game.move_object_by_effect(in_graveyard, Zone::Hand).unwrap();
        let object = game.object(back_in_hand).unwrap();
        assert!(object.alternative_casts.iter().all(|method| method.suspend_spec().is_none()));
    }
}

// ---------------------------------------------------------------- C2 B5
mod granted_rebound {
    use super::*;

    #[test]
    fn a_spell_granted_rebound_is_exiled_as_it_resolves() {
        let mut game = game(2);
        game.create_object_from_definition(&def("Cast Through Time"), ALICE, Zone::Battlefield);
        let salve = CardDefinitionBuilder::new(CardId::new(), "Healing Salve")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .with_spell_effect(vec![Effect::gain_life(3)])
            .build();
        let spell = game.create_object_from_definition(&salve, ALICE, Zone::Hand);
        let stable = game.object(spell).unwrap().stable_id;
        add_mana(&mut game, ALICE, ManaSymbol::White, 1);
        game.refresh_continuous_state();
        cast(&mut game, ALICE, spell);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Dm::new()).unwrap();
        assert_eq!(game.player(ALICE).unwrap().life, 23);
        assert_eq!(zone_of(&game, stable), Zone::Exile);
    }
}

// ---------------------------------------------------------------- C2 D2
mod twobrid_with_improvise {
    use super::*;

    /// A {2/W}{2/W}{2/W} sorcery with improvise (Spectral Procession's cost).
    fn castable_with_artifacts(artifacts: usize) -> bool {
        let mut game = game(2);
        for _ in 0..artifacts {
            fixture(&mut game, "Trinket", vec![CardType::Artifact], ALICE, Zone::Battlefield);
        }
        let twobrid = vec![ManaSymbol::Generic(2), ManaSymbol::White];
        let def = CardDefinitionBuilder::new(CardId::new(), "Twobrid Procession")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![twobrid.clone(), twobrid.clone(), twobrid]))
            .with_spell_effect(vec![Effect::gain_life(1)])
            .improvise()
            .build();
        let spell = game.create_object_from_definition(&def, ALICE, Zone::Hand);
        game.refresh_continuous_state();
        can_cast(&game, ALICE, spell, any_method)
    }

    #[test]
    fn one_artifact_cannot_pay_a_whole_two_generic_half() {
        assert!(!castable_with_artifacts(3), "three artifacts pay only {{3}} of {{6}}");
    }

    #[test]
    fn two_artifacts_jointly_pay_each_two_generic_half() {
        assert!(castable_with_artifacts(6), "six artifacts pay {{2}}{{2}}{{2}}");
    }
}

// ---------------------------------------------------------------- C2 B6
mod cipher {
    use super::*;

    #[test]
    fn the_encoded_trigger_is_granted_not_printed() {
        let mut game = game(2);
        let creature = fixture(&mut game, "Carrier", vec![CardType::Creature], ALICE, Zone::Battlefield);
        fixture(&mut game, "Library Card", vec![CardType::Sorcery], ALICE, Zone::Library);
        let spell = game.create_object_from_definition(&def("Last Thoughts"), ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Blue, 4);
        game.refresh_continuous_state();
        let printed = game.object(creature).unwrap().abilities.len();
        let mut dm = Dm::new().prefer("Carrier");
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        resolve_top_with(&mut game, &mut dm);
        game.refresh_continuous_state();
        assert_eq!(
            game.object(creature).unwrap().abilities.len(),
            printed,
            "not part of the creature's copiable values"
        );
        assert!(
            game.current_abilities(creature).unwrap().len() > printed,
            "the creature has the encoded trigger"
        );
    }
}

// ---------------------------------------------------------------- C2 D3
mod ninjutsu {
    use super::*;
    use ironsmith::combat_state::{AttackTarget, CombatState};

    #[test]
    fn a_ninja_does_not_attack_a_player_who_left_the_game() {
        let carol = PlayerId(2);
        let mut game = game(3);
        game.combat = Some(CombatState::default());
        let ninja = game.create_object_from_definition(&def("Ninja of the Deep Hours"), ALICE, Zone::Hand);
        game.record_ninjutsu_attack_target(ninja, AttackTarget::Player(carol));
        game.mark_player_lost(carol);
        game.push_to_stack(StackEntry::ability(
            ninja,
            ALICE,
            vec![Effect::new(ironsmith::effects::NinjutsuEffect::new())],
        ));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Dm::new()).unwrap();
        let ninja = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().name.as_str() == "Ninja of the Deep Hours")
            .expect("the ninja still enters");
        assert!(game.combat.as_ref().unwrap().attackers.iter().all(|attacker| attacker.creature != ninja));
    }
}

// ---------------------------------------------------------------- C1 C-1
mod awaken {
    use super::*;
    use ironsmith::types::Subtype;

    #[test]
    fn an_awakened_land_is_an_elemental_and_stays_dead() {
        let mut game = game(2);
        let land = fixture(&mut game, "Test Land", vec![CardType::Land], ALICE, Zone::Battlefield);
        let land_stable = game.object(land).unwrap().stable_id;
        fixture(&mut game, "Bear", vec![CardType::Creature], BOB, Zone::Battlefield);
        let spell = game.create_object_from_definition(&def("Ruinous Path"), ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Black, 7);
        game.refresh_continuous_state();
        let mut dm = Dm::new();
        cast_with(&mut game, ALICE, spell, alternative, &mut dm);
        resolve_top_with(&mut game, &mut dm);
        game.refresh_continuous_state();
        assert!(game.object_has_card_type(land, CardType::Creature));
        assert!(game.current_subtypes(land).unwrap_or_default().contains(&Subtype::Elemental));
        assert_eq!(game.calculated_power(land), Some(4));
        run_ability(
            &mut game,
            BOB,
            vec![Effect::destroy(ironsmith::target::ChooseSpec::SpecificObject(land))],
            &mut Dm::new(),
        );
        resolve_all(&mut game, &mut Dm::new());
        assert_eq!(zone_of(&game, land_stable), Zone::Graveyard, "awaken has no return clause");
    }
}

// ---------------------------------------------------------------- C1 A1
mod casualty_x {
    use super::*;

    fn setup() -> (GameState, ObjectId, ObjectId) {
        let mut game = game(2);
        let ogre = CardDefinitionBuilder::new(CardId::new(), "Ogre")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let ogre = game.create_object_from_definition(&ogre, ALICE, Zone::Battlefield);
        let ob = game.create_object_from_definition(&def("Ob Nixilis, the Adversary"), ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Black, 2);
        add_mana(&mut game, ALICE, ManaSymbol::Red, 2);
        game.refresh_continuous_state();
        (game, ob, ogre)
    }

    #[test]
    fn declining_casualty_keeps_the_creature_and_makes_no_copy() {
        let (mut game, ob, ogre) = setup();
        cast_with(&mut game, ALICE, ob, any_method, &mut Dm::new());
        assert!(game.battlefield.contains(&ogre));
        assert_eq!(game.stack.len(), 1, "no copy trigger");
    }

    #[test]
    fn paying_casualty_sacrifices_while_casting_and_copies_with_loyalty_x() {
        let (mut game, ob, ogre) = setup();
        let mut dm = Dm::new().pick("Casualty").prefer("Ogre");
        cast_with(&mut game, ALICE, ob, any_method, &mut dm);
        assert!(!game.battlefield.contains(&ogre), "sacrificed as a cost");
        assert_eq!(game.stack.len(), 2, "the spell and its copy trigger");
        resolve_top_with(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 2, "the copy is on the stack");
        let copy = game.object(game.stack[1].object_id).unwrap();
        assert_eq!(copy.base_loyalty, Some(3));
    }
}

// ---------------------------------------------------------------- C1 A2
mod granted_casualty {
    use super::*;

    #[test]
    fn granted_casualty_is_paid_while_casting() {
        let mut game = game(2);
        game.create_object_from_definition(&def("Anhelo, the Painter"), ALICE, Zone::Battlefield);
        let ogre = CardDefinitionBuilder::new(CardId::new(), "Ogre")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let ogre = game.create_object_from_definition(&ogre, ALICE, Zone::Battlefield);
        let salve = CardDefinitionBuilder::new(CardId::new(), "Healing Salve")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .with_spell_effect(vec![Effect::gain_life(3)])
            .build();
        let spell = game.create_object_from_definition(&salve, ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::White, 1);
        game.refresh_continuous_state();
        let mut dm = Dm::new().pick("Granted Casualty").prefer("Ogre");
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        assert!(!game.battlefield.contains(&ogre), "sacrificed while casting: {:?}", dm.seen_options);
        assert_eq!(game.stack.len(), 2, "spell plus copy trigger");
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(ALICE).unwrap().life, 26, "original and copy both resolved");
    }
}

// ---------------------------------------------------------------- C1 B1
mod prowl {
    use super::*;
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::types::Subtype;

    #[test]
    fn a_non_rogue_goblin_enables_a_goblin_rogues_prowl() {
        let mut game = game(2);
        let goblin = CardDefinitionBuilder::new(CardId::new(), "Goblin Raider")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Goblin])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let goblin = game.create_object_from_definition(&goblin, ALICE, Zone::Battlefield);
        let bandit = game.create_object_from_definition(&def("Stinkdrinker Bandit"), ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Black, 2);
        game.refresh_continuous_state();
        assert!(!can_cast(&game, ALICE, bandit, any_method), "no prowl yet, and {{3}}{{B}} is unaffordable");
        // Combat damage from the Goblin to Bob.
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo { creature: goblin, target: AttackTarget::Player(BOB) });
        game.combat = Some(combat);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::CombatDamage);
        let combat = game.combat.clone().unwrap();
        let events = ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::queue_combat_damage_triggers(&mut game, &events, &mut queue);
        assert_eq!(game.player(BOB).unwrap().life, 18);
        game.combat = None;
        game.turn.phase = Phase::NextMain;
        game.turn.step = None;
        assert!(can_cast(&game, ALICE, bandit, alternative));
    }
}

// ---------------------------------------------------------------- C1 B3
mod prototype {
    use super::*;

    #[test]
    fn a_free_cast_from_omniscience_can_be_prototyped() {
        let mut game = game(2);
        game.create_object_from_definition(&def("Omniscience"), ALICE, Zone::Battlefield);
        // Phyrexian Fleshgorger's shape (its Prototype line doesn't parse yet).
        let def = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Fleshgorger")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .power_toughness(PowerToughness::fixed(7, 5))
            .alternative_cast(ironsmith::AlternativeCastingMethod::prototype(
                ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(1)],
                    vec![ManaSymbol::Black],
                    vec![ManaSymbol::Black],
                ]),
                PowerToughness::fixed(3, 3),
            ))
            .build();
        let fleshgorger = game.create_object_from_definition(&def, ALICE, Zone::Hand);
        game.refresh_continuous_state();
        let free = |method: &CastingMethod, _: Zone| !matches!(method, CastingMethod::Alternative(_) | CastingMethod::Normal);
        let mut dm = Dm::new().pick("Prototype");
        cast_with(&mut game, ALICE, fleshgorger, free, &mut dm);
        let spell = game.stack.last().unwrap().object_id;
        assert_eq!(pool(&game, ALICE), 0);
        resolve_top_with(&mut game, &mut dm);
        game.refresh_continuous_state();
        let permanent = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().name.as_str() == "Phyrexian Fleshgorger")
            .unwrap_or(spell);
        assert_eq!(game.calculated_power(permanent), Some(3), "prototyped: {:?}", dm.seen_options);
    }
}

// ---------------------------------------------------------------- C1 A4
mod entwine {
    use super::*;

    #[test]
    fn entwine_is_not_offered_when_a_mode_has_no_legal_target() {
        let mut game = game(2);
        let spell = game.create_object_from_definition(&def("Barbed Lightning"), ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Red, 5);
        game.refresh_continuous_state();
        let mut dm = Dm::new().pick("Entwine");
        cast_with(&mut game, ALICE, spell, any_method, &mut dm);
        let entwine = dm.seen_options.iter().find(|option| option.starts_with("Entwine"));
        assert!(entwine.is_none_or(|option| option.ends_with("legal=false")), "{:?}", dm.seen_options);
        assert!(!game.stack[0].optional_costs_paid.was_entwined());
    }
}

// ---------------------------------------------------------------- C1 B4
mod emerge {
    use super::*;

    #[test]
    fn emerge_reduces_by_the_sacrificed_creatures_current_mana_value() {
        let mut game = game(2);
        let big = CardDefinitionBuilder::new(CardId::new(), "Big Model")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let big = game.create_object_from_definition(&big, BOB, Zone::Battlefield);
        let small = CardDefinitionBuilder::new(CardId::new(), "Small Copier")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let small = game.create_object_from_definition(&small, ALICE, Zone::Battlefield);
        // A layer-1 copy effect (Cytoshape-style) makes the 1-drop a copy of
        // the four-mana creature.
        let copiable = ironsmith::snapshot::CopiableValues::from_object(game.object(big).unwrap());
        let mut dm = SelectFirstDecisionMaker;
        ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(small),
            ironsmith::continuous::Modification::CopyOf {
                target_id: big,
                copiable_values: Box::new(copiable),
                preserve_source_abilities: false,
                name_override: None,
                name_override_surface: None,
                add_supertypes: Vec::new(),
            },
            ironsmith::effect::Until::EndOfTurn,
        )
        .execute(&mut game, &mut ironsmith::effects::EffectContext::new(small, ALICE, &mut dm))
        .unwrap();
        let fiend = game.create_object_from_definition(&def("Elder Deep-Fiend"), ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Blue, 2);
        add_mana(&mut game, ALICE, ManaSymbol::Colorless, 5);
        game.refresh_continuous_state();
        cast_with(&mut game, ALICE, fiend, alternative, &mut Dm::new());
        assert!(!game.battlefield.contains(&small), "sacrificed for emerge");
        assert_eq!(pool(&game, ALICE), 4, "{{5}}{{U}}{{U}} reduced by mana value 4 costs {{1}}{{U}}{{U}}");
    }
}

// ---------------------------------------------------------------- C1 B2
mod surge {
    use super::*;

    #[test]
    fn a_teammates_spell_enables_surge() {
        let mut game = game(4);
        game.set_teams(vec![vec![ALICE, PlayerId(2)], vec![BOB, PlayerId(3)]]).unwrap();
        let crush = game.create_object_from_definition(&def("Crush of Tentacles"), ALICE, Zone::Hand);
        add_mana(&mut game, ALICE, ManaSymbol::Blue, 5);
        game.refresh_continuous_state();
        assert!(!can_cast(&game, ALICE, crush, any_method), "{{4}}{{U}}{{U}} is unaffordable");
        add_mana(&mut game, PlayerId(2), ManaSymbol::Red, 1);
        let spell = filler(&mut game, PlayerId(2));
        cast(&mut game, PlayerId(2), spell);
        resolve_top(&mut game);
        game.turn.priority_player = Some(ALICE);
        assert!(can_cast(&game, ALICE, crush, alternative));
    }
}


