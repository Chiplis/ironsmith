//! Frozen full-card Waterbend contracts. Authored only: execution is deferred.
//! Board resources are produced by native casts. Direct and artifact routes
//! independently compile the complete, unchanged Oracle body in the fixture.
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, DecisionContext, DistributeContext, ManaPaymentContext,
    NumberContext, PartitionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, generate_and_queue_step_triggers,
    put_triggers_on_stack_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::{ManaPaymentExecution, ManaPaymentRequest, ManaPaymentResponse,
    ManaPaymentSourceKind, PlannedPipPayment, RequiredAlternativePayment,
    execute_mana_payment_plan, plan_mana_payment};
use ironsmith::costs::{Cost, CostContext, PaymentReason};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Zone};
use serde_json::Value;

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
const KATARA: &str = "Katara, Water Tribe's Hope";

#[derive(Clone, Copy, Debug)]
enum Route { Direct, Artifact }
const ROUTES: [Route; 2] = [Route::Direct, Route::Artifact];

fn compile(name: &str, input: String, route: Route) -> CardDefinition {
    match route {
        Route::Direct => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_runtime_definition(
                    ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name), input, false));
            assert!(!loss.is_lossy(), "{name} direct loss: {}", loss.reasons_text());
            result.unwrap_or_else(|error| panic!("{name} direct: {error:?}"))
        }
        Route::Artifact => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_artifact(
                    ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name), input, false));
            assert!(!loss.is_lossy(), "{name} artifact loss: {}", loss.reasons_text());
            // Discard the helper's runtime result. Materialize the actual wire
            // roundtrip, independently of the direct compilation above.
            let (artifact, _) = result.unwrap_or_else(|error| panic!("{name} artifact: {error:?}"));
            artifact.validate().unwrap();
            let bytes = serde_json::to_vec(&artifact).unwrap();
            let decoded = serde_json::from_slice(&bytes).unwrap();
            ironsmith::artifact_materializer::materialize_artifact(&decoded).unwrap()
        }
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/card-failure-campaign/waterbend-payment-bodies.json")).unwrap()
}

fn definition(name: &str, route: Route) -> CardDefinition {
    let fixture = fixture();
    let row = fixture["cards"].as_array().unwrap().iter().find(|r| r["name"] == name).unwrap();
    let mut input = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        input.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    input.push_str(row["oracle_text"].as_str().unwrap());
    compile(name, input, route)
}

fn support(name: &str, route: Route) -> CardDefinition {
    let input = match name {
        "Grizzly Bears" => "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
        "Memnite" => "Mana cost: {0}\nType: Artifact Creature — Construct\nPower/Toughness: 1/1",
        "Ornithopter" => "Mana cost: {0}\nType: Artifact Creature — Thopter\nPower/Toughness: 0/2\nFlying",
        "Shuko" => "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature gets +1/+0.\nEquip {0} ({0}: Attach to target creature you control. Equip only as a sorcery.)",
        "Sol Ring" => "Mana cost: {1}\nType: Artifact\n{T}: Add {C}{C}.",
        "Shock" => "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target.",
        "Twiddle" => "Mana cost: {U}\nType: Instant\nYou may tap or untap target artifact, creature, or land.",
        "Pull from Eternity" => "Mana cost: {W}\nType: Instant\nPut target face-up exiled card into its owner's graveyard.",
        "Cremate" => "Mana cost: {B}\nType: Instant\nExile target card from a graveyard.\nDraw a card.",
        "Act of Treason" => "Mana cost: {2}{R}\nType: Sorcery\nGain control of target creature until end of turn. Untap that creature. It gains haste until end of turn. (It can attack and {T} this turn.)",
        "Mana Reflection" => "Mana cost: {4}{G}{G}\nType: Enchantment\nIf you tap a permanent for mana, it produces twice as much of that mana instead.",
        "Thalia, Guardian of Thraben" => "Mana cost: {1}{W}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 2/1\nFirst strike\nNoncreature spells cost {1} more to cast.",
        _ => panic!("unknown support card {name}"),
    };
    compile(name, input.into(), route)
}

#[derive(Default)]
struct Choices {
    targets: Option<Vec<Target>>,
    x: u32,
    taps: Vec<ObjectId>,
    decline: bool,
    distribution: Vec<(Target, u32)>,
    distributions_seen: Vec<Vec<Target>>,
    accepted: Vec<ManaPaymentContext>,
    scry_cards: usize,
    x_ranges: Vec<(u32, u32)>,
    assist_amount: Option<usize>,
}

impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            self.x_ranges.push((ctx.min, ctx.max));
            assert!(self.x >= ctx.min && self.x <= ctx.max, "chosen X outside legal range: {ctx:?}");
            self.x
        } else { ctx.min }
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { !self.decline }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let Some(targets) = &self.targets else { return SelectFirstDecisionMaker.decide_targets(game, ctx); };
        let mut selected = Vec::new();
        for requirement in &ctx.requirements {
            let candidates = targets.iter().copied().filter(|target| requirement.legal_targets.contains(target)
                && !selected.contains(target)).take(requirement.max_targets.unwrap_or(targets.len())).collect::<Vec<_>>();
            assert!(candidates.len() >= requirement.min_targets, "requested targets not legal: {ctx:?}");
            selected.extend(candidates);
        }
        selected
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(amount) = self.assist_amount {
            let chosen = if ctx.description.starts_with("Choose another player to assist") {
                Some(1)
            } else if ctx.description.starts_with("Choose how much generic mana") {
                Some(amount)
            } else { None };
            if let Some(index) = chosen {
                assert!(ctx.options.iter().any(|option| option.index == index && option.legal),
                    "the requested Assist contribution must have a complete caster continuation: {ctx:?}");
                return vec![index];
            }
        }
        if let Some(option) = ctx.options.iter().find(|option| option.legal && option.description == "Tap") {
            return vec![option.index];
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
    fn decide_partition(&mut self, _: &GameState, ctx: &PartitionContext) -> Vec<ObjectId> {
        self.scry_cards += ctx.cards.len();
        Vec::new()
    }
    fn decide_distribute(&mut self, _: &GameState, ctx: &DistributeContext) -> Vec<(Target, u32)> {
        self.distributions_seen.push(ctx.targets.iter().map(|target| target.target).collect());
        assert_eq!(self.distribution.iter().map(|(_, amount)| amount).sum::<u32>(), ctx.total);
        for (target, _) in &self.distribution {
            assert!(ctx.targets.iter().any(|candidate| candidate.target == *target), "illegal distribution: {ctx:?}");
        }
        self.distribution.clone()
    }
    fn decide_mana_payment(&mut self, _: &GameState, ctx: &ManaPaymentContext) -> ManaPaymentResponse {
        if ctx.request.cost.has_waterbend_obligation() {
            if self.decline { return ManaPaymentResponse::Cancel; }
            let mut preferences = ctx.request.preferences.clone();
            let wanted = self.taps.iter().map(|source| RequiredAlternativePayment {
                source: *source, kind: ManaPaymentSourceKind::Waterbend }).collect::<Vec<_>>();
            if wanted.iter().any(|choice| !preferences.required_alternatives.contains(choice)) {
                preferences.required_alternatives.extend(wanted);
                return ManaPaymentResponse::Replan { preferences };
            }
        }
        assert!(ctx.plan.payable, "successful path must have a complete price: {ctx:?}");
        self.accepted.push(ctx.clone());
        ManaPaymentResponse::Confirm { plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash }
    }
}

struct Board { game: GameState, queue: TriggerQueue, choices: Choices, route: Route }
impl Board {
    fn new(route: Route) -> Self {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.set_random_seed(20261005);
        game.turn.turn_number = 7;
        let mut board = Self { game, queue: TriggerQueue::new(), choices: Choices::default(), route };
        board.main(ALICE);
        for player in [ALICE, BOB] {
            for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green] {
                board.game.player_mut(player).unwrap().mana_pool.add(symbol, 30);
            }
            let filler = support("Memnite", route);
            for _ in 0..12 { board.game.create_object_from_definition(&filler, player, Zone::Library); }
        }
        board
    }
    fn main(&mut self, player: PlayerId) {
        self.game.turn.active_player = player;
        self.game.turn.priority_player = Some(player);
        self.game.turn.phase = ironsmith::Phase::FirstMain;
        self.game.turn.step = None;
    }
    fn pool(&mut self, player: PlayerId, symbols: &[(ManaSymbol, u32)]) {
        self.game.player_mut(player).unwrap().mana_pool = Default::default();
        for (symbol, amount) in symbols { self.game.player_mut(player).unwrap().mana_pool.add(*symbol, *amount); }
    }
    fn hand(&mut self, definition: &CardDefinition, player: PlayerId) -> ObjectId {
        self.game.create_object_from_definition(definition, player, Zone::Hand)
    }
    fn action(&self, card: ObjectId, player: PlayerId) -> Option<LegalAction> {
        compute_legal_actions(&self.game, player).unwrap().into_iter().find(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
    }
    fn announce(&mut self, action: LegalAction) -> ObjectId {
        let before = self.game.stack.len();
        let mut state = PriorityLoopState::new(self.game.players_in_game());
        let mut progress = apply_priority_response_with_dm(&mut self.game, &mut self.queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut self.choices).unwrap();
        for _ in 0..64 {
            if state.pending_cast.is_none() && state.pending_activation.is_none() && self.game.stack.len() > before {
                return self.game.stack.last().unwrap().object_id;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("native announcement stalled: {progress:?}"); };
            progress = apply_decision_context_with_dm(&mut self.game, &mut self.queue, &mut state, &ctx, &mut self.choices).unwrap();
        }
        panic!("native announcement did not finish");
    }
    fn cast(&mut self, definition: &CardDefinition, player: PlayerId, paid: u32) -> ObjectId {
        self.game.turn.priority_player = Some(player);
        let card = self.hand(definition, player);
        let stable = self.game.object(card).unwrap().stable_id;
        let before = self.game.player(player).unwrap().mana_pool.total();
        let action = self.action(card, player).expect("normal native cast must be legal");
        self.announce(action);
        let spell = self.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(before - self.game.player(player).unwrap().mana_pool.total(), paid, "{} cast mana", definition.name());
        assert_eq!(self.game.object(spell).unwrap().zone, Zone::Stack);
        spell
    }
    fn finish(&mut self) {
        for _ in 0..48 {
            drain_pending_trigger_events(&mut self.game, &mut self.queue);
            put_triggers_on_stack_with_dm(&mut self.game, &mut self.queue, &mut self.choices).unwrap();
            if self.game.stack.is_empty() { return; }
            let mut state = PriorityLoopState::new(self.game.players_in_game());
            state.reset_for_new_priority_window(&mut self.game);
            for _ in 0..self.game.players_in_game() {
                let progress = apply_priority_response_with_dm(&mut self.game, &mut self.queue, &mut state,
                    &PriorityResponse::PriorityAction(LegalAction::PassPriority), &mut self.choices).unwrap();
                if let GameProgress::NeedsDecisionCtx(ctx) = progress {
                    assert!(matches!(ctx, DecisionContext::Priority(_)), "unhandled resolving choice: {ctx:?}");
                }
            }
        }
        panic!("native stack did not empty");
    }
    fn permanent(&mut self, name: &str, player: PlayerId) -> ObjectId {
        self.main(player);
        self.choices.targets = None;
        let definition = support(name, self.route);
        let paid = match name { "Memnite" | "Ornithopter" => 0, "Shuko" | "Sol Ring" => 1, _ => 2 };
        let spell = self.cast(&definition, player, paid);
        let stable = self.game.object(spell).unwrap().stable_id;
        self.finish();
        let permanent = self.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(self.game.object(permanent).unwrap().zone, Zone::Battlefield);
        permanent
    }
    fn resources(&mut self, count: usize) -> Vec<ObjectId> {
        (0..count).map(|i| self.permanent(if i % 2 == 0 { "Shuko" } else { "Grizzly Bears" }, ALICE)).collect()
    }
    fn target_spell(&mut self, name: &str, player: PlayerId, target: ObjectId, paid: u32) {
        self.choices.targets = Some(vec![Target::Object(target)]);
        let definition = support(name, self.route);
        self.cast(&definition, player, paid);
        self.finish();
        self.choices.targets = None;
    }
    fn end_step(&mut self, player: PlayerId) {
        self.game.turn.active_player = player;
        self.game.turn.phase = ironsmith::Phase::NextMain;
        self.game.turn.step = None;
        ironsmith::turn::advance_phase(&mut self.game).unwrap();
        assert_eq!(self.game.turn.step, Some(ironsmith::Step::End));
        generate_and_queue_step_triggers(&mut self.game, &mut self.queue);
        self.finish();
    }
}

fn assert_waterbend_payment(choices: &Choices, amount: u32, taps: &[ObjectId]) {
    let accepted = choices.accepted.iter().rev().find(|ctx| ctx.request.cost.has_waterbend_obligation())
        .expect("waterbend must reach the authoritative payment interaction");
    assert_eq!(accepted.request.cost.waterbend_capacity(accepted.request.x_value), amount);
    let mut actual = accepted.plan.allocations.iter().filter_map(|allocation| match allocation.payment {
        PlannedPipPayment::Waterbend(source) => Some(source), _ => None }).collect::<Vec<_>>();
    let mut expected = taps.to_vec();
    actual.sort(); expected.sort();
    assert_eq!(actual, expected, "each selected permanent pays exactly one scoped generic pip");
}

fn waterbend_receipts(game: &GameState) -> Vec<(PlayerId, ObjectId, u32)> {
    use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
    let history = &game.turn_store.turn_history;
    history.event_records.iter().chain(history.staged_event_records.iter())
        .filter_map(|record| record.event.downcast::<KeywordActionEvent>())
        .filter(|event| event.action == KeywordActionKind::Waterbend)
        .map(|event| (event.player, event.source, event.amount)).collect()
}

#[test]
fn frozen_full_bodies_have_six_recoveries_and_an_independent_katara_regression() {
    let fixture = fixture();
    let rows = fixture["cards"].as_array().unwrap();
    assert_eq!(rows.len(), 7);
    assert_eq!(rows.iter().filter(|row| row["counts_as_recovery"] == true).count(), 6);
    let mut ids = std::collections::HashSet::new();
    for row in rows {
        assert!(ids.insert(row["oracle_id"].as_str().unwrap()));
        let name = row["name"].as_str().unwrap();
        assert_eq!(row["baseline_category"], if name == KATARA { "strict_compiled" } else { "parser_failure" });
        for route in ROUTES { assert_eq!(definition(name, route).name(), name); }
    }
}

#[test]
fn spirit_pays_one_combined_price_and_keeps_flying_ward_and_enters_scry() {
    for route in ROUTES {
        for tapped in [0, 2, 5] {
            let mut board = Board::new(route);
            let resources = board.resources(5);
            // These creatures entered this turn: waterbend has no {T} sickness restriction.
            board.pool(ALICE, &[(ManaSymbol::Blue, 2), (ManaSymbol::Colorless, 5 - tapped)]);
            board.choices.taps = resources[..tapped as usize].to_vec();
            let definition = definition("Benevolent River Spirit", route);
            let spell = board.cast(&definition, ALICE, 7 - tapped);
            let stable = board.game.object(spell).unwrap().stable_id;
            assert_waterbend_payment(&board.choices, 5, &board.choices.taps);
            assert_eq!(board.choices.accepted.last().unwrap().plan.allocations.len(), 7);
            board.finish();
            let spirit = board.game.find_object_by_stable_id(stable).unwrap();
            assert_eq!((board.game.calculated_power(spirit), board.game.calculated_toughness(spirit)), (Some(4), Some(5)));
            assert!(board.game.object_has_static_ability_id(spirit, ironsmith::static_abilities::StaticAbilityId::Flying));
            assert_eq!(board.choices.scry_cards, 2);
            for (index, resource) in resources.iter().enumerate() {
                assert_eq!(board.game.is_tapped(*resource), index < tapped as usize);
            }
            // Opposing Shock pays its own printed cost, then pays ward {2}.
            board.choices.taps.clear();
            board.pool(BOB, &[(ManaSymbol::Red, 1), (ManaSymbol::Colorless, 2)]);
            board.target_spell("Shock", BOB, spirit, 1);
            assert_eq!(board.game.player(BOB).unwrap().mana_pool.total(), 0);
            assert_eq!(board.game.damage_on(spirit), 2);
        }
    }
}

#[test]
fn mandatory_waterbend_never_replaces_colored_pips_or_ordinary_cost_increases() {
    for route in ROUTES {
        let mut board = Board::new(route);
        board.permanent("Thalia, Guardian of Thraben", BOB);
        board.resources(6);
        board.main(ALICE);
        let definition = definition("Water Whip", route);
        let card = board.hand(&definition, ALICE);
        board.pool(ALICE, &[(ManaSymbol::Blue, 2)]);
        assert!(board.action(card, ALICE).is_none(), "six taps cannot pay Thalia's ordinary generic increase");
        board.pool(ALICE, &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 1)]);
        assert!(board.action(card, ALICE).is_none(), "tap substitution cannot pay the second blue pip");
        board.pool(ALICE, &[(ManaSymbol::Blue, 2), (ManaSymbol::Colorless, 1)]);
        board.choices.targets = Some(vec![]);
        let action = board.action(card, ALICE).expect("one real generic mana plus five taps funds the full taxed price");
        board.announce(action);
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert_eq!(board.choices.accepted.last().unwrap().request.cost.waterbend_capacity(0), 5);
        board.finish();
        assert_eq!(board.game.player(ALICE).unwrap().hand.len(), 2);
    }
}

#[test]
fn water_whip_keeps_zero_one_and_two_target_bounce_and_draw_tail() {
    for route in ROUTES {
        for count in 0..=2 {
            let mut board = Board::new(route);
            let own = board.permanent("Memnite", ALICE);
            let opposing = board.permanent("Grizzly Bears", BOB);
            let own_stable = board.game.object(own).unwrap().stable_id;
            let opposing_stable = board.game.object(opposing).unwrap().stable_id;
            let resources = board.resources(3);
            board.pool(ALICE, &[(ManaSymbol::Blue, 2), (ManaSymbol::Colorless, 2)]);
            board.choices.taps = resources;
            board.choices.targets = Some([Target::Object(own), Target::Object(opposing)][..count].to_vec());
            let definition = definition("Water Whip", route);
            let spell = board.cast(&definition, ALICE, 4);
            assert_eq!(board.game.stack.last().unwrap().targets, board.choices.targets.clone().unwrap());
            assert_eq!(board.game.object(spell).unwrap().zone, Zone::Stack);
            board.finish();
            let own = board.game.find_object_by_stable_id(own_stable).unwrap();
            let opposing = board.game.find_object_by_stable_id(opposing_stable).unwrap();
            assert_eq!(board.game.object(own).unwrap().zone, if count >= 1 { Zone::Hand } else { Zone::Battlefield });
            assert_eq!(board.game.object(opposing).unwrap().zone, if count == 2 { Zone::Hand } else { Zone::Battlefield });
            assert_eq!(board.game.player(ALICE).unwrap().hand.len(), 2 + usize::from(count >= 1));
            assert_eq!(board.game.player(BOB).unwrap().hand.len(), usize::from(count == 2));
            assert_waterbend_payment(&board.choices, 5, &board.choices.taps);
        }
    }
}

#[test]
fn crashing_wave_distributes_after_tapping_and_zero_x_still_runs_its_tail() {
    for route in ROUTES {
        for x in [0, 2] {
            let mut board = Board::new(route);
            let old_tapped = board.permanent("Grizzly Bears", BOB);
            let new_tapped = board.permanent("Grizzly Bears", BOB);
            let untouched = board.permanent("Grizzly Bears", BOB);
            board.target_spell("Twiddle", BOB, old_tapped, 1);
            let own = board.permanent("Grizzly Bears", ALICE);
            let resources = board.resources(2);
            board.pool(ALICE, &[(ManaSymbol::Blue, 2), (ManaSymbol::Colorless, u32::from(x != 0))]);
            board.choices.x = x;
            board.choices.taps = if x == 0 { vec![] } else { vec![resources[0]] };
            board.choices.targets = Some(if x == 0 { vec![] } else { vec![Target::Object(new_tapped), Target::Object(own)] });
            board.choices.distribution = if x == 0 { vec![(Target::Object(old_tapped), 3)] }
                else { vec![(Target::Object(old_tapped), 1), (Target::Object(new_tapped), 2)] };
            let definition = definition("Crashing Wave", route);
            board.cast(&definition, ALICE, 2 + u32::from(x != 0));
            assert_waterbend_payment(&board.choices, x, &board.choices.taps);
            board.finish();
            assert_eq!(board.game.is_tapped(new_tapped), x != 0);
            assert_eq!(board.game.is_tapped(own), x != 0);
            assert!(!board.game.is_tapped(untouched));
            assert_eq!(board.game.counter_count(old_tapped, CounterType::Stun), if x == 0 { 3 } else { 1 });
            assert_eq!(board.game.counter_count(new_tapped, CounterType::Stun), if x == 0 { 0 } else { 2 });
            assert_eq!(board.game.counter_count(own, CounterType::Stun), 0);
            let offered = board.choices.distributions_seen.last().expect("distribution instruction must execute");
            assert!(offered.contains(&Target::Object(old_tapped)));
            assert_eq!(offered.contains(&Target::Object(new_tapped)), x != 0);
            assert!(!offered.contains(&Target::Object(own)) && !offered.contains(&Target::Object(untouched)));
        }
    }
}

#[test]
fn foggy_swamp_visions_copies_only_actually_exiled_cards_and_sacrifices_at_our_end_step() {
    for route in ROUTES {
        for remove_one in [false, true] {
            let mut board = Board::new(route);
            let first = board.permanent("Memnite", ALICE);
            let second = board.permanent("Grizzly Bears", BOB);
            let first_stable = board.game.object(first).unwrap().stable_id;
            let second_stable = board.game.object(second).unwrap().stable_id;
            board.target_spell("Shock", BOB, first, 1);
            board.target_spell("Shock", BOB, second, 1);
            let first = board.game.find_object_by_stable_id(first_stable).unwrap();
            let second = board.game.find_object_by_stable_id(second_stable).unwrap();
            assert_eq!(board.game.object(first).unwrap().zone, Zone::Graveyard);
            assert_eq!(board.game.object(second).unwrap().zone, Zone::Graveyard);
            let resources = board.resources(2);
            board.pool(ALICE, &[(ManaSymbol::Black, 2), (ManaSymbol::Colorless, 2)]);
            board.choices.x = 2;
            board.choices.taps = vec![resources[0]];
            board.choices.targets = Some(vec![Target::Object(first), Target::Object(second)]);
            let definition = definition("Foggy Swamp Visions", route);
            board.cast(&definition, ALICE, 4);
            assert_waterbend_payment(&board.choices, 2, &board.choices.taps);
            if remove_one {
                board.choices.taps.clear();
                board.target_spell("Cremate", BOB, first, 1);
            } else { board.finish(); }
            for stable in [first_stable, second_stable] {
                let current = board.game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(board.game.object(current).unwrap().zone, Zone::Exile);
            }
            let tokens = board.game.battlefield.iter().copied().filter(|id|
                board.game.object(*id).is_some_and(|object| matches!(object.kind, ironsmith::object::ObjectKind::Token)))
                .collect::<Vec<_>>();
            assert_eq!(tokens.len(), if remove_one { 1 } else { 2 });
            assert!(tokens.iter().all(|id| board.game.current_controller(*id) == Some(ALICE)));
            assert_eq!(tokens.iter().filter(|id| board.game.object(**id).unwrap().name == "Grizzly Bears").count(), 1);
            assert_eq!(tokens.iter().filter(|id| board.game.object(**id).unwrap().name == "Memnite").count(), usize::from(!remove_one));
            for token in &tokens {
                let expected = if board.game.object(*token).unwrap().name == "Memnite" { 1 } else { 2 };
                assert_eq!((board.game.calculated_power(*token), board.game.calculated_toughness(*token)), (Some(expected), Some(expected)));
            }
            board.end_step(BOB);
            assert!(tokens.iter().all(|id| board.game.object(*id).is_some_and(|object| object.zone == Zone::Battlefield)),
                "your next end step must not mean an opponent's end step");
            board.end_step(ALICE);
            assert!(tokens.iter().all(|id| board.game.object(*id).is_none_or(|object| object.zone != Zone::Battlefield)));
            assert!(resources.iter().all(|id| board.game.object(*id).unwrap().zone == Zone::Battlefield),
                "deferred sacrifice names only the tokens created by this resolution");
            assert!(board.game.effect_store.delayed_triggers.is_empty());
        }
    }
}

#[test]
fn restoration_returns_exact_exiled_incarnations_to_owners_at_the_next_end_step() {
    for route in ROUTES {
        for move_away in [false, true] {
            let mut board = Board::new(route);
            let own = board.permanent("Memnite", ALICE);
            let borrowed = board.permanent("Grizzly Bears", BOB);
            let own_stable = board.game.object(own).unwrap().stable_id;
            let borrowed_stable = board.game.object(borrowed).unwrap().stable_id;
            board.main(ALICE);
            board.target_spell("Act of Treason", ALICE, borrowed, 3);
            assert_eq!(board.game.current_controller(borrowed), Some(ALICE));
            board.pool(ALICE, &[(ManaSymbol::Blue, 2)]);
            board.choices.x = 2;
            board.choices.taps = vec![own, borrowed];
            board.choices.targets = Some(vec![Target::Object(own), Target::Object(borrowed)]);
            let definition = definition("Waterbender's Restoration", route);
            board.cast(&definition, ALICE, 2);
            assert_waterbend_payment(&board.choices, 2, &[own, borrowed]);
            board.finish();
            let own_exiled = board.game.find_object_by_stable_id(own_stable).unwrap();
            let borrowed_exiled = board.game.find_object_by_stable_id(borrowed_stable).unwrap();
            assert_eq!(board.game.object(own_exiled).unwrap().zone, Zone::Exile);
            assert_eq!(board.game.object(borrowed_exiled).unwrap().zone, Zone::Exile);
            assert_ne!(own_exiled, own);
            assert_ne!(borrowed_exiled, borrowed);
            if move_away {
                board.choices.taps.clear();
                board.target_spell("Pull from Eternity", BOB, own_exiled, 1);
            }
            board.end_step(BOB);
            let own_current = board.game.find_object_by_stable_id(own_stable).unwrap();
            let borrowed_current = board.game.find_object_by_stable_id(borrowed_stable).unwrap();
            assert_eq!(board.game.object(own_current).unwrap().zone, if move_away { Zone::Graveyard } else { Zone::Battlefield });
            assert_eq!(board.game.object(borrowed_current).unwrap().zone, Zone::Battlefield);
            assert_eq!(board.game.current_controller(borrowed_current), Some(BOB));
            assert!(!board.game.is_tapped(borrowed_current));
            assert_ne!(borrowed_current, borrowed_exiled);
            assert!(board.game.effect_store.delayed_triggers.is_empty());
            board.end_step(ALICE);
            assert_eq!(board.game.find_object_by_stable_id(borrowed_stable), Some(borrowed_current), "one-shot return must not repeat");
        }
    }
}

#[test]
fn zero_x_is_an_explicit_payment_without_targets_copies_or_delayed_objects() {
    for route in ROUTES {
        for name in ["Foggy Swamp Visions", "Waterbender's Restoration"] {
            let mut board = Board::new(route);
            let spare = board.permanent("Memnite", ALICE);
            board.pool(ALICE, if name == "Foggy Swamp Visions" {
                &[(ManaSymbol::Black, 2), (ManaSymbol::Colorless, 1)]
            } else { &[(ManaSymbol::Blue, 2)] });
            board.choices.targets = Some(vec![]);
            let definition = definition(name, route);
            board.cast(&definition, ALICE, if name == "Foggy Swamp Visions" { 3 } else { 2 });
            assert_waterbend_payment(&board.choices, 0, &[]);
            assert!(board.choices.accepted.last().unwrap().request.cost.has_waterbend_obligation());
            board.finish();
            assert!(board.game.exile.is_empty());
            assert_eq!(board.game.battlefield, vec![spare]);
            assert!(!board.game.is_tapped(spare));
            board.end_step(ALICE);
            assert_eq!(board.game.battlefield, vec![spare]);
        }
    }
}

#[test]
fn lesson_draws_three_then_keeps_or_discards_exactly_one() {
    for route in ROUTES {
        for (taps, extra_mana, decline, expected_hand) in [(0, 2, false, 3), (1, 1, false, 3),
            (2, 0, false, 3), (2, 0, true, 2), (1, 0, false, 2)] {
            let mut board = Board::new(route);
            let resources = board.resources(taps);
            board.pool(ALICE, &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 3 + extra_mana)]);
            board.choices.taps = resources.clone();
            let definition = definition("Waterbending Lesson", route);
            board.cast(&definition, ALICE, 4);
            assert_eq!(board.game.player(ALICE).unwrap().hand.len(), 0);
            board.choices.decline = decline;
            board.finish();
            assert_eq!(board.game.player(ALICE).unwrap().hand.len(), expected_hand);
            assert_eq!(board.game.player(ALICE).unwrap().library.len(), 9);
            assert_eq!(board.game.player(ALICE).unwrap().graveyard.len(), if expected_hand == 3 { 1 } else { 2 });
            if expected_hand == 3 {
                assert_waterbend_payment(&board.choices, 2, &resources);
                assert!(resources.iter().all(|id| board.game.is_tapped(*id)));
                assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
            } else {
                assert!(resources.iter().all(|id| !board.game.is_tapped(*id)));
                assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), extra_mana);
            }
        }
    }
}

#[test]
fn katara_migration_keeps_nonzero_own_turn_activation_and_entire_temporary_body() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let opposing = board.permanent("Grizzly Bears", BOB);
        let own = board.permanent("Grizzly Bears", ALICE);
        let definition = definition(KATARA, route);
        let spell = board.cast(&definition, ALICE, 5);
        let stable = board.game.object(spell).unwrap().stable_id;
        board.finish();
        let katara = board.game.find_object_by_stable_id(stable).unwrap();
        let ally = *board.game.battlefield.iter().find(|id| board.game.object(**id).is_some_and(|object| object.name == "Ally")).unwrap();
        assert_eq!((board.game.calculated_power(ally), board.game.calculated_toughness(ally)), (Some(1), Some(1)));
        assert!(board.game.object_has_static_ability_id(katara, ironsmith::static_abilities::StaticAbilityId::Vigilance));
        board.pool(ALICE, &[]);
        board.main(BOB);
        assert!(!compute_legal_actions(&board.game, ALICE).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateAbility { source, .. } if *source == katara)));
        board.main(ALICE);
        board.choices.x = 3;
        board.choices.taps = vec![katara, ally, own];
        let action = compute_legal_actions(&board.game, ALICE).unwrap().into_iter().find(|action|
            matches!(action, LegalAction::ActivateAbility { source, .. } if *source == katara)).unwrap();
        board.announce(action);
        assert!(board.choices.x_ranges.iter().any(|(min, _)| *min == 1), "X can't be zero must reach legal X bounds");
        assert_waterbend_payment(&board.choices, 3, &[katara, ally, own]);
        board.finish();
        for id in [katara, ally, own] {
            assert_eq!((board.game.calculated_power(id), board.game.calculated_toughness(id)), (Some(3), Some(3)));
            assert!(board.game.is_tapped(id));
        }
        assert_eq!(board.game.calculated_power(opposing), Some(2));
        board.game.turn.phase = ironsmith::Phase::Ending;
        board.game.turn.step = Some(ironsmith::Step::Cleanup);
        ironsmith::turn::execute_cleanup_step(&mut board.game);
        for (id, expected) in [(katara, 3), (ally, 1), (own, 2)] {
            assert_eq!((board.game.calculated_power(id), board.game.calculated_toughness(id)), (Some(expected), Some(expected)));
        }
    }
}

#[test]
fn combined_obligations_have_one_resource_budget_and_leave_ordinary_generic_unsubstituted() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let resources = board.resources(4);
        let first = ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]).with_waterbend();
        let second = ManaCost::from_symbols(vec![ManaSymbol::X]).with_waterbend();
        let cost = first.combined_with(&second).add_generic(2);
        assert_eq!(cost.waterbend_payment_scope().unwrap().obligations.len(), 2);
        assert_eq!(cost.waterbend_capacity(2), 3);
        board.pool(ALICE, &[(ManaSymbol::Colorless, 1)]);
        let mut request = ManaPaymentRequest::new(ALICE, resources[0], PaymentReason::Effect, cost).with_x(2);
        assert!(plan_mana_payment(&board.game, &request).is_err(), "a fourth artifact cannot pay the ordinary generic shortfall");
        board.pool(ALICE, &[(ManaSymbol::Colorless, 2)]);
        request.preferences.required_alternatives = resources[..3].iter().map(|source| RequiredAlternativePayment {
            source: *source, kind: ManaPaymentSourceKind::Waterbend }).collect();
        let plan = plan_mana_payment(&board.game, &request).unwrap().remove(0);
        assert_eq!(plan.allocations.iter().filter(|allocation| matches!(allocation.payment, PlannedPipPayment::Waterbend(_))).count(), 3);
        assert_eq!(execute_mana_payment_plan(&mut board.game, &request, &plan, &mut board.choices).unwrap(), ManaPaymentExecution::Paid);
        assert!(resources[..3].iter().all(|id| board.game.is_tapped(*id)));
        assert!(!board.game.is_tapped(resources[3]));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn eligibility_and_artifact_creature_overlap_are_counted_exactly_once() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let both = board.permanent("Memnite", ALICE);
        let opposing = board.permanent("Grizzly Bears", BOB);
        let tapped = board.permanent("Shuko", ALICE);
        board.target_spell("Twiddle", ALICE, tapped, 1);
        let phased = board.permanent("Grizzly Bears", ALICE);
        board.game.phase_out(phased);
        let plain_land = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Noneligible land")
            .card_types(vec![CardType::Land]).build();
        let land = board.game.create_object_from_definition(&plain_land, ALICE, Zone::Battlefield);
        board.pool(ALICE, &[]);
        let mut request = ManaPaymentRequest::new(ALICE, both, PaymentReason::Effect,
            ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]).with_waterbend());
        assert!(plan_mana_payment(&board.game, &request).is_err(), "artifact creature is one permanent; no other resource is eligible");
        request.cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]).with_waterbend();
        for bad in [opposing, tapped, phased, land] {
            request.preferences.required_alternatives = vec![RequiredAlternativePayment { source: bad, kind: ManaPaymentSourceKind::Waterbend }];
            assert!(plan_mana_payment(&board.game, &request).is_err(), "ineligible selected permanent {bad:?}");
        }
        request.preferences.required_alternatives = vec![RequiredAlternativePayment { source: both, kind: ManaPaymentSourceKind::Waterbend }];
        let plan = plan_mana_payment(&board.game, &request).unwrap().remove(0);
        assert_eq!(execute_mana_payment_plan(&mut board.game, &request, &plan, &mut board.choices).unwrap(), ManaPaymentExecution::Paid);
        assert!(board.game.is_tapped(both), "newly cast creature can pay waterbend immediately");
        assert!(!board.game.is_tapped(opposing) && !board.game.is_tapped(land));
    }
}

fn pending_payment(board: &mut Board, action: LegalAction) -> (PriorityLoopState, ManaPaymentContext) {
    let mut state = PriorityLoopState::new(board.game.players_in_game());
    let mut progress = apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut board.choices).unwrap();
    for _ in 0..32 {
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("expected a pending payment: {progress:?}"); };
        if let DecisionContext::ManaPayment(context) = ctx { return (state, context); }
        progress = apply_decision_context_with_dm(&mut board.game, &mut board.queue, &mut state, &ctx, &mut board.choices).unwrap();
    }
    panic!("did not reach a payment decision");
}

#[test]
fn native_cast_payment_pending_rejected_confirmation_and_cancel_restore_the_whole_announcement() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let ring = board.permanent("Sol Ring", ALICE);
        let resources = board.resources(3);
        board.pool(ALICE, &[(ManaSymbol::Blue, 2)]);
        board.choices.targets = Some(vec![]);
        let definition = definition("Water Whip", route);
        let card = board.hand(&definition, ALICE);
        let stable = board.game.object(card).unwrap().stable_id;
        let action = board.action(card, ALICE).unwrap();
        let (mut state, context) = pending_payment(&mut board, action);
        assert!(state.pending_cast.is_some());
        assert_eq!(context.request.cost.waterbend_capacity(context.request.x_value), 5);
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 2);
        assert!(!board.game.is_tapped(ring));
        assert!(resources.iter().all(|id| !board.game.is_tapped(*id)));
        assert_eq!(board.game.player(ALICE).unwrap().library.len(), 12, "pending price cannot run the draw body");
        assert!(waterbend_receipts(&board.game).is_empty(), "a displayed payable plan is not a completed Waterbend");
        let rejected = apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
            &PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Confirm {
                plan_id: context.plan.id.wrapping_add(1), request_hash: context.plan.request_hash }), &mut board.choices);
        assert!(rejected.is_err());
        assert!(state.pending_cast.is_some(), "bad response preserves the pending interaction");
        assert!(waterbend_receipts(&board.game).is_empty(), "rejected confirmation cannot create a receipt");
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 2);
        let (source, ability_index) = ironsmith::mana_payment::manual_mana_abilities(&board.game, &context.request)
            .into_iter().find(|(source, _)| *source == ring).unwrap();
        let progress = apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
            &PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Activate { source, ability_index }), &mut board.choices).unwrap();
        assert!(matches!(progress, GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(_))));
        assert!(board.game.is_tapped(ring));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 4);
        assert!(waterbend_receipts(&board.game).is_empty(), "manual mana production is not Waterbend completion");
        apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
            &PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Cancel), &mut board.choices).unwrap();
        assert!(state.pending_cast.is_none());
        assert!(board.game.stack.is_empty());
        let restored = board.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(board.game.object(restored).unwrap().zone, Zone::Hand);
        assert!(!board.game.is_tapped(ring));
        assert!(resources.iter().all(|id| !board.game.is_tapped(*id)));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 2);
        assert_eq!(board.game.player(ALICE).unwrap().library.len(), 12);
        assert!(board.game.player(ALICE).unwrap().graveyard.is_empty());
        assert!(waterbend_receipts(&board.game).is_empty(), "cancelled cast has no Waterbend receipt");
    }
}

#[derive(Clone, Copy, Debug)]
enum StopPayment { Cancel, Pending, InvalidConfirmation, Complete }
struct ManualChoices { ring: ObjectId, mode: StopPayment, prompts: usize, waiting: bool }
impl DecisionMaker for ManualChoices {
    fn awaiting_choice(&self) -> bool { self.waiting }
    fn decide_mana_payment(&mut self, game: &GameState, ctx: &ManaPaymentContext) -> ManaPaymentResponse {
        self.prompts += 1;
        if self.prompts == 1 {
            let (source, ability_index) = ironsmith::mana_payment::manual_mana_abilities(game, &ctx.request)
                .into_iter().find(|(source, _)| *source == self.ring).unwrap();
            return ManaPaymentResponse::Activate { source, ability_index };
        }
        assert_eq!(self.prompts, 2, "manual activation should return once to the same payment");
        match self.mode {
            StopPayment::Cancel => ManaPaymentResponse::Cancel,
            StopPayment::Pending => { self.waiting = true; ManaPaymentResponse::Cancel }
            StopPayment::InvalidConfirmation => ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id.wrapping_add(1), request_hash: ctx.plan.request_hash },
            StopPayment::Complete => ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash },
        }
    }
}

#[test]
fn direct_cost_mana_rolls_back_manual_activations_on_cancel_pending_and_invalid_confirmation() {
    for route in ROUTES {
        for mode in [StopPayment::Cancel, StopPayment::Pending, StopPayment::InvalidConfirmation, StopPayment::Complete] {
            let mut board = Board::new(route);
            let ring = board.permanent("Sol Ring", ALICE);
            board.pool(ALICE, &[]);
            let mut choices = ManualChoices { ring, mode, prompts: 0, waiting: false };
            let cost = Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]).with_waterbend());
            let result = cost.pay(&mut board.game, &mut CostContext::new(ring, ALICE, &mut choices).with_reason(PaymentReason::Effect));
            assert_eq!(choices.prompts, 2);
            match mode {
                StopPayment::Complete => {
                    assert_eq!(result.unwrap(), ironsmith::costs::CostPaymentResult::Paid);
                    assert!(board.game.is_tapped(ring));
                }
                StopPayment::Cancel => {
                    assert_eq!(result, Err(ironsmith::cost::CostPaymentError::Cancelled));
                    assert!(!board.game.is_tapped(ring));
                }
                StopPayment::Pending | StopPayment::InvalidConfirmation => {
                    assert_eq!(result, Err(ironsmith::cost::CostPaymentError::InsufficientMana));
                    assert_eq!(choices.awaiting_choice(), matches!(mode, StopPayment::Pending));
                    assert!(!board.game.is_tapped(ring));
                }
            }
            assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
            assert!(board.game.stack.is_empty());
            assert_eq!(board.game.player(ALICE).unwrap().life, 20);
            assert_eq!(waterbend_receipts(&board.game), if matches!(mode, StopPayment::Complete) {
                vec![(ALICE, ring, 2)]
            } else { vec![] }, "manual cancel, pending and error must not leak completion receipts");
        }
    }
}

#[test]
fn stale_waterbend_plan_does_not_commit_any_remaining_resource() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let resources = board.resources(2);
        board.pool(ALICE, &[]);
        let mut request = ManaPaymentRequest::new(ALICE, resources[0], PaymentReason::Effect,
            ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]).with_waterbend());
        request.preferences.required_alternatives = resources.iter().map(|source| RequiredAlternativePayment {
            source: *source, kind: ManaPaymentSourceKind::Waterbend }).collect();
        let plan = plan_mana_payment(&board.game, &request).unwrap().remove(0);
        board.target_spell("Twiddle", BOB, resources[0], 1);
        assert!(board.game.is_tapped(resources[0]));
        let life = board.game.player(ALICE).unwrap().life;
        assert!(execute_mana_payment_plan(&mut board.game, &request, &plan, &mut board.choices).is_err());
        assert!(!board.game.is_tapped(resources[1]));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert_eq!(board.game.player(ALICE).unwrap().life, life);
    }
}

#[test]
fn waterbend_uses_replaced_mana_production_and_cannot_tap_the_same_source_twice() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let ring = board.permanent("Sol Ring", ALICE);
        let resource = board.permanent("Shuko", ALICE);
        let reflection = support("Mana Reflection", route);
        board.cast(&reflection, ALICE, 6);
        board.finish();
        board.pool(ALICE, &[]);
        let mut request = ManaPaymentRequest::new(ALICE, resource, PaymentReason::Effect,
            ManaCost::from_symbols(vec![ManaSymbol::Generic(6)]).with_waterbend());
        assert!(plan_mana_payment(&board.game, &request).is_err(),
            "Sol Ring produces four mana or pays one tap, never both; Shuko adds only one");
        request.cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(5)]).with_waterbend();
        request.preferences.required_alternatives = vec![RequiredAlternativePayment {
            source: resource, kind: ManaPaymentSourceKind::Waterbend }];
        let plan = plan_mana_payment(&board.game, &request).unwrap().remove(0);
        assert_eq!(plan.mana_ability_steps.len(), 1);
        assert_eq!(plan.mana_ability_steps[0].source, ring);
        assert_eq!(plan.mana_ability_steps[0].expected_mana.total(), 4);
        assert_eq!(execute_mana_payment_plan(&mut board.game, &request, &plan, &mut board.choices).unwrap(), ManaPaymentExecution::Paid);
        assert!(board.game.is_tapped(ring) && board.game.is_tapped(resource));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert_eq!(waterbend_receipts(&board.game), vec![(ALICE, resource, 5)]);
    }
}

#[test]
fn ordinary_direct_mana_cost_still_spends_floating_mana_without_waterbend_substitution() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let resource = board.permanent("Memnite", ALICE);
        board.pool(ALICE, &[(ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 2)]);
        let cost = Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Blue, ManaSymbol::Generic(2)]));
        let result = cost.pay(&mut board.game,
            &mut CostContext::new(resource, ALICE, &mut board.choices).with_reason(PaymentReason::Effect));
        assert_eq!(result.unwrap(), ironsmith::costs::CostPaymentResult::Paid);
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert!(!board.game.is_tapped(resource));
    }
}

#[test]
fn water_whip_partial_target_failure_keeps_draw_but_all_target_failure_does_not() {
    for route in ROUTES {
        for count in [1, 2] {
            let mut board = Board::new(route);
            let first = board.permanent("Grizzly Bears", BOB);
            let second = board.permanent("Grizzly Bears", BOB);
            let second_stable = board.game.object(second).unwrap().stable_id;
            let resources = board.resources(5);
            board.pool(ALICE, &[(ManaSymbol::Blue, 2)]);
            board.choices.taps = resources;
            board.choices.targets = Some([Target::Object(first), Target::Object(second)][..count].to_vec());
            let definition = definition("Water Whip", route);
            board.cast(&definition, ALICE, 2);
            board.choices.taps.clear();
            board.target_spell("Shock", BOB, first, 1);
            assert_eq!(board.game.player(ALICE).unwrap().hand.len(), if count == 1 { 0 } else { 2 });
            let second = board.game.find_object_by_stable_id(second_stable).unwrap();
            assert_eq!(board.game.object(second).unwrap().zone, if count == 1 { Zone::Battlefield } else { Zone::Hand });
            assert!(board.game.stack.is_empty());
        }
    }
}

#[test]
fn katara_rejects_zero_x_before_spending_or_changing_any_creatures() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let definition = definition(KATARA, route);
        let spell = board.cast(&definition, ALICE, 5);
        let stable = board.game.object(spell).unwrap().stable_id;
        board.finish();
        let katara = board.game.find_object_by_stable_id(stable).unwrap();
        board.pool(ALICE, &[]);
        let action = compute_legal_actions(&board.game, ALICE).unwrap().into_iter().find(|action|
            matches!(action, LegalAction::ActivateAbility { source, .. } if *source == katara)).unwrap();
        let mut state = PriorityLoopState::new(board.game.players_in_game());
        let progress = apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut board.choices).unwrap();
        let GameProgress::NeedsDecisionCtx(DecisionContext::Number(context)) = progress else {
            panic!("Katara must announce X before paying: {progress:?}");
        };
        assert_eq!(context.min, 1);
        assert!(apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
            &PriorityResponse::XValue(0), &mut board.choices).is_err());
        assert_eq!(board.game.calculated_power(katara), Some(3));
        assert!(board.game.battlefield.iter().all(|id| !board.game.is_tapped(*id)));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn restoration_target_validation_does_not_accept_an_opponents_creature_or_spend_its_price() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let opposing = board.permanent("Grizzly Bears", BOB);
        let own = board.permanent("Memnite", ALICE);
        board.pool(ALICE, &[(ManaSymbol::Blue, 2)]);
        let definition = definition("Waterbender's Restoration", route);
        let card = board.hand(&definition, ALICE);
        let action = board.action(card, ALICE).unwrap();
        let mut state = PriorityLoopState::new(board.game.players_in_game());
        let progress = apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut board.choices).unwrap();
        assert!(matches!(progress, GameProgress::NeedsDecisionCtx(DecisionContext::Number(_))));
        let progress = apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
            &PriorityResponse::XValue(1), &mut board.choices).unwrap();
        let GameProgress::NeedsDecisionCtx(DecisionContext::Targets(context)) = progress else {
            panic!("expected X-bound targets before payment: {progress:?}");
        };
        assert!(context.requirements.iter().any(|requirement| requirement.legal_targets.contains(&Target::Object(own))));
        assert!(context.requirements.iter().all(|requirement| !requirement.legal_targets.contains(&Target::Object(opposing))));
        assert!(apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
            &PriorityResponse::Targets(vec![Target::Object(opposing)]), &mut board.choices).is_err());
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 2);
        assert!(!board.game.is_tapped(own) && !board.game.is_tapped(opposing));
        assert!(board.game.exile.is_empty());
    }
}

#[test]
fn reduced_waterbend_share_does_not_grow_back_when_ordinary_generic_is_added() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let resources = board.resources(3);
        let reduced = ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]).with_waterbend()
            .reduce_generic(2).add_generic(2);
        assert!(reduced.has_waterbend_obligation());
        assert_eq!(reduced.waterbend_capacity(0), 1);
        board.pool(ALICE, &[]);
        let mut request = ManaPaymentRequest::new(ALICE, resources[0], PaymentReason::Effect, reduced);
        assert!(plan_mana_payment(&board.game, &request).is_err(), "new ordinary generic is outside the reduced tap share");
        board.pool(ALICE, &[(ManaSymbol::Colorless, 2)]);
        request.preferences.required_alternatives = vec![RequiredAlternativePayment {
            source: resources[0], kind: ManaPaymentSourceKind::Waterbend }];
        let plan = plan_mana_payment(&board.game, &request).unwrap().remove(0);
        assert_eq!(execute_mana_payment_plan(&mut board.game, &request, &plan, &mut board.choices).unwrap(), ManaPaymentExecution::Paid);
        assert!(board.game.is_tapped(resources[0]));
        assert!(resources[1..].iter().all(|id| !board.game.is_tapped(*id)));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert_eq!(waterbend_receipts(&board.game), vec![(ALICE, resources[0], 3)],
            "a reduction changes the price, not the amount of the original Waterbend action");
    }
}

#[test]
fn accepted_zero_obligation_is_distinct_from_an_absent_obligation_and_can_be_declined() {
    for route in ROUTES {
        for decline in [false, true] {
            let mut board = Board::new(route);
            let source = board.permanent("Memnite", ALICE);
            board.pool(ALICE, &[]);
            board.choices.accepted.clear();
            board.choices.decline = decline;
            let zero = ManaCost::from_symbols(vec![ManaSymbol::X]).with_waterbend()
                .bind_x_payment_if_unbound(0);
            assert!(zero.has_waterbend_obligation());
            assert_eq!(zero.waterbend_capacity(0), 0);
            let result = Cost::mana(zero).pay(&mut board.game,
                &mut CostContext::new(source, ALICE, &mut board.choices).with_x(0).with_reason(PaymentReason::Effect));
            if decline {
                assert_eq!(result, Err(ironsmith::cost::CostPaymentError::Cancelled));
                assert!(board.choices.accepted.is_empty());
                assert!(waterbend_receipts(&board.game).is_empty());
            } else {
                assert_eq!(result.unwrap(), ironsmith::costs::CostPaymentResult::Paid);
                assert_eq!(board.choices.accepted.len(), 1, "zero needs a real accepted payment");
                assert_waterbend_payment(&board.choices, 0, &[]);
                assert_eq!(waterbend_receipts(&board.game), vec![(ALICE, source, 0)]);
            }
            board.choices.accepted.clear();
            assert_eq!(Cost::mana(ManaCost::new()).pay(&mut board.game,
                &mut CostContext::new(source, ALICE, &mut board.choices).with_reason(PaymentReason::Effect)).unwrap(),
                ironsmith::costs::CostPaymentResult::Paid);
            assert!(board.choices.accepted.is_empty(), "an absent obligation has no payment acceptance");
            assert_eq!(waterbend_receipts(&board.game), if decline { vec![] } else { vec![(ALICE, source, 0)] },
                "ordinary empty payment does not add a Waterbend receipt");
            assert!(!board.game.is_tapped(source));
            assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        }
    }
}

#[test]
fn native_assist_pays_ordinary_generic_then_the_caster_completes_waterbend() {
    for route in ROUTES {
      for contribution in [2, 4, 5] {
        let mut board = Board::new(route);
        let assistant_resource = board.permanent("Memnite", BOB);
        let resources = board.resources(3);
        board.pool(ALICE, &[(ManaSymbol::Blue, 1)]);
        board.pool(BOB, &[(ManaSymbol::Colorless, contribution)]);
        board.choices.accepted.clear();
        board.choices.assist_amount = Some(contribution as usize);
        let taps = resources[..(5 - contribution) as usize].to_vec();
        board.choices.taps = taps.clone();
        // Composition fixture, distinct from the unchanged frozen six bodies.
        // The ordinary printed generic is paid by Bob; the original additional
        // Waterbend action remains Alice's scoped obligation and receipt.
        let definition = compile("Assist Waterbend composition", "Mana cost: {2}{U}\nType: Sorcery\nAssist\nAs an additional cost to cast this spell, waterbend {3}.\nDraw a card.".into(), route);
        let spell = board.cast(&definition, ALICE, 1);
        assert_eq!(board.choices.accepted.iter().map(|context| context.player).collect::<Vec<_>>(), vec![BOB, ALICE]);
        let assistant = &board.choices.accepted[0];
        assert!(!assistant.request.cost.has_waterbend_obligation());
        assert_eq!(assistant.plan.allocations.len(), contribution as usize);
        assert!(assistant.request.assist_completion.is_some(), "helper payment proves the actual scoped caster continuation");
        assert!(assistant.plan.allocations.iter().all(|allocation| matches!(allocation.payment, PlannedPipPayment::Mana(_))));
        assert_waterbend_payment(&board.choices, 5 - contribution, &taps);
        assert_eq!(board.game.player(BOB).unwrap().mana_pool.total(), 0);
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert!(!board.game.is_tapped(assistant_resource));
        assert!(taps.iter().all(|source| board.game.is_tapped(*source)));
        assert!(resources[taps.len()..].iter().all(|source| !board.game.is_tapped(*source)));
        assert_eq!(waterbend_receipts(&board.game), vec![(ALICE, spell, 3)], "Assist is not an additional Waterbend action");
        let spell_object = board.game.object(spell).unwrap();
        assert_eq!(spell_object.mana_spent_to_cast.blue, 1);
        assert_eq!(spell_object.mana_spent_to_cast.colorless, contribution);
        assert_eq!(spell_object.caster_mana_spent_to_cast, Some(1));
        board.finish();
        assert_eq!(board.game.player(ALICE).unwrap().hand.len(), 1);
        assert_eq!(waterbend_receipts(&board.game), vec![(ALICE, spell, 3)], "resolution and trigger draining cannot duplicate payment receipts");
      }
    }
}

#[test]
fn combined_waterbend_payment_emits_one_receipt_per_original_obligation_including_zero() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let resources = board.resources(2);
        board.pool(ALICE, &[(ManaSymbol::Colorless, 2)]);
        board.choices.accepted.clear();
        board.choices.taps = resources.clone();
        let two = ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]).with_waterbend();
        let zero = ManaCost::from_symbols(vec![ManaSymbol::X]).with_waterbend().bind_x_payment_if_unbound(0);
        let one = ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]).with_waterbend();
        let combined = two.combined_with(&zero).combined_with(&one).add_generic(1);
        assert_eq!(combined.waterbend_payment_scope().unwrap().obligations.len(), 3);
        let request = ManaPaymentRequest::new(ALICE, resources[0], PaymentReason::Effect, combined.clone());
        let _ = plan_mana_payment(&board.game, &request).unwrap();
        assert!(waterbend_receipts(&board.game).is_empty(), "planner branches never emit committed action receipts");
        assert_eq!(Cost::mana(combined).pay(&mut board.game,
            &mut CostContext::new(resources[0], ALICE, &mut board.choices).with_reason(PaymentReason::Effect)).unwrap(),
            ironsmith::costs::CostPaymentResult::Paid);
        assert_eq!(board.choices.accepted.len(), 1, "three original actions are paid by one combined transaction");
        let expected = vec![(ALICE, resources[0], 2), (ALICE, resources[0], 0), (ALICE, resources[0], 1)];
        assert_eq!(waterbend_receipts(&board.game), expected);
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert!(resources.iter().all(|source| board.game.is_tapped(*source)));
        board.finish();
        assert_eq!(waterbend_receipts(&board.game), expected, "draining pending events preserves exactly the same receipts");
    }
}

fn announce_mana_ability(board: &mut Board, source: ObjectId) {
    let action = compute_legal_actions(&board.game, ALICE).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateManaAbility { source: candidate, .. } if *candidate == source))
        .expect("native mana activation must be legal");
    let mut state = PriorityLoopState::new(board.game.players_in_game());
    let mut progress = apply_priority_response_with_dm(&mut board.game, &mut board.queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut board.choices).unwrap();
    for _ in 0..32 {
        if state.pending_mana_ability.is_none() && board.game.player(ALICE).unwrap().mana_pool.blue == 1 {
            assert!(board.game.stack.is_empty(), "mana abilities resolve without using the stack");
            return;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("mana activation stalled: {progress:?}"); };
        progress = apply_decision_context_with_dm(&mut board.game, &mut board.queue, &mut state,
            &context, &mut board.choices).unwrap();
    }
    panic!("mana activation did not finish");
}

#[test]
fn native_waterbend_mana_ability_can_tap_its_source_without_recursively_activating_it() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let definition = compile("Self Waterbend mana source", "Mana cost: {0}\nType: Artifact\nWaterbend {1}: Add {U}.".into(), route);
        let spell = board.cast(&definition, ALICE, 0);
        let stable = board.game.object(spell).unwrap().stable_id;
        board.finish();
        let source = board.game.find_object_by_stable_id(stable).unwrap();
        board.pool(ALICE, &[]);
        board.choices.accepted.clear();
        board.choices.taps = vec![source];
        announce_mana_ability(&mut board, source);
        assert_waterbend_payment(&board.choices, 1, &[source]);
        assert!(board.game.is_tapped(source));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.blue, 1);
        assert_eq!(waterbend_receipts(&board.game), vec![(ALICE, source, 1)]);
    }
}

#[test]
fn native_mana_ability_with_an_independent_tap_cost_needs_a_second_waterbend_resource() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let definition = compile("Tap and Waterbend mana source", "Mana cost: {0}\nType: Artifact\n{T}, Waterbend {1}: Add {U}.".into(), route);
        let spell = board.cast(&definition, ALICE, 0);
        let stable = board.game.object(spell).unwrap().stable_id;
        board.finish();
        let source = board.game.find_object_by_stable_id(stable).unwrap();
        board.pool(ALICE, &[]);
        assert!(!compute_legal_actions(&board.game, ALICE).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateManaAbility { source: candidate, .. } if *candidate == source)),
            "the source reserved for {{T}} cannot also pay the Waterbend pip");
        assert!(!board.game.is_tapped(source));
        assert!(waterbend_receipts(&board.game).is_empty());
        let helper = board.permanent("Memnite", ALICE);
        board.choices.accepted.clear();
        board.choices.taps = vec![helper];
        announce_mana_ability(&mut board, source);
        assert_waterbend_payment(&board.choices, 1, &[helper]);
        assert!(board.game.is_tapped(source) && board.game.is_tapped(helper));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.blue, 1);
        assert_eq!(waterbend_receipts(&board.game), vec![(ALICE, source, 1)]);
    }
}

#[test]
fn activation_recursion_exclusion_does_not_reserve_an_alternative_payment_tap() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let ring = board.permanent("Sol Ring", ALICE);
        board.pool(ALICE, &[]);
        let mut request = ManaPaymentRequest::new(ALICE, ring, PaymentReason::ActivateManaAbility,
            ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]).with_waterbend());
        request.activation_excluded_sources = vec![ring];
        request.preferences.required_alternatives = vec![RequiredAlternativePayment {
            source: ring, kind: ManaPaymentSourceKind::Waterbend }];
        let plan = plan_mana_payment(&board.game, &request).unwrap().remove(0);
        assert!(plan.mana_ability_steps.is_empty(), "recursion exclusion blocks activating Sol Ring");
        assert_eq!(plan.allocations.iter().filter(|allocation| matches!(allocation.payment, PlannedPipPayment::Waterbend(id) if id == ring)).count(), 1);
        assert!(waterbend_receipts(&board.game).is_empty());
        let mut reserved = request.clone();
        reserved.reserved_tap_sources = vec![ring];
        assert!(plan_mana_payment(&board.game, &reserved).is_err(), "an independent tap reservation blocks using the same source twice");
        assert_eq!(execute_mana_payment_plan(&mut board.game, &request, &plan, &mut board.choices).unwrap(), ManaPaymentExecution::Paid);
        assert!(board.game.is_tapped(ring));
        assert_eq!(board.game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert_eq!(waterbend_receipts(&board.game), vec![(ALICE, ring, 1)]);
    }
}
