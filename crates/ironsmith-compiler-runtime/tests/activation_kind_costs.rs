//! UNRUN source scenarios for exact frozen activation-kind cost modifier cards.
//! Authored during the source-only campaign; no execution or coverage is claimed.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::decision::{
    ActivationCostAbility, DecisionMaker, LegalAction, SelectFirstDecisionMaker,
    calculate_effective_activation_total_cost_for_ability, compute_legal_actions,
};
use ironsmith::decisions::context::{DecisionContext, ManaPaymentContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CounterType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ActivatedAbilityKeyword as Keyword, StaticAbilityId};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/activation_kind_costs.json.fixture")).unwrap()
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, text, false)
    });
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    restored.validate().unwrap();
    let result = [direct, materialize_artifact(&restored).unwrap()];
    for definition in &result {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    result
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    definitions_text(name, &text)
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    main_phase(&mut game, A);
    game
}
fn main_phase(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}
fn mana(game: &mut GameState, player: PlayerId, symbol: ManaSymbol, amount: u32) {
    game.player_mut(player).unwrap().mana_pool.add(symbol, amount);
}
fn printed(game: &mut GameState, name: &str, text: &str, owner: PlayerId, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn creature(game: &mut GameState, name: &str, owner: PlayerId, zone: Zone) -> ObjectId {
    printed(game, name, "Type: Creature — Human\nPower/Toughness: 2/2", owner, zone)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for n in 0..count { creature(game, &format!("Library witness {n}"), player, Zone::Library); }
}
fn index(game: &GameState, source: ObjectId, ordinal: usize) -> usize {
    game.current_abilities(source).unwrap().iter().enumerate()
        .filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_)))
        .nth(ordinal).unwrap().0
}
fn keyword_index(game: &GameState, source: ObjectId, keyword: Keyword) -> usize {
    game.current_abilities(source).unwrap().iter().position(|ability| {
        matches!(&ability.kind, AbilityKind::Activated(activated) if activated.keyword == Some(keyword))
    }).unwrap()
}
fn action(game: &GameState, player: PlayerId, source: ObjectId, ability_index: usize) -> Option<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter().find(|action| {
        matches!(action, LegalAction::ActivateAbility { source: id, ability_index: n }
            | LegalAction::ActivateManaAbility { source: id, ability_index: n }
            if *id == source && *n == ability_index)
    })
}
fn price(game: &GameState, player: PlayerId, source: ObjectId, ability_index: usize) -> ironsmith::cost::TotalCost {
    let ability = game.current_ability(source, ability_index).unwrap();
    let AbilityKind::Activated(activated) = &ability.kind else { panic!("activated ability"); };
    calculate_effective_activation_total_cost_for_ability(game, player, source,
        &activated.mana_cost, &[], ActivationCostAbility::at(game, player, source, ability_index))
}
fn assert_price(game: &GameState, player: PlayerId, source: ObjectId, ability_index: usize, symbols: Vec<ManaSymbol>) {
    let actual = price(game, player, source, ability_index);
    // Power-up retains the intrinsic entry reduction in a dynamic component until payment.
    // These base assertions use objects that did not enter this turn.
    let actual_mana = actual.mana_cost().cloned()
        .or_else(|| actual.dynamic_mana_cost().map(|dynamic| dynamic.base.clone()))
        .unwrap_or_else(|| ManaCost::from_symbols(vec![]));
    assert_eq!(actual_mana, ManaCost::from_symbols(symbols));
}
#[derive(Default)]
struct Choices {
    object: Option<ObjectId>,
    target: Option<Target>,
    cancel: bool,
    saw_cancel: bool,
    activate_mana_first: Option<(ObjectId, usize)>,
    mana_requests: Vec<(PlayerId, ManaCost)>,
    x: Option<u32>,
    x_bounds: Vec<(u32, u32)>,
    branch: Option<usize>,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            self.x_bounds.push((ctx.min, ctx.max));
            if let Some(x) = self.x { assert!(ctx.min <= x && x <= ctx.max); return x; }
        }
        SelectFirstDecisionMaker.decide_number(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(branch) = self.branch.take() {
            assert!(ctx.options.iter().any(|option| option.index == branch && option.legal));
            return vec![branch];
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.object {
            assert!(ctx.candidates.iter().any(|candidate| candidate.id == id && candidate.legal));
            vec![id]
        } else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(ctx.requirements.iter().any(|requirement| requirement.legal_targets.contains(&target)));
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, ctx) }
    }
    fn decide_mana_payment(&mut self, game: &GameState, ctx: &ManaPaymentContext) -> ManaPaymentResponse {
        self.mana_requests.push((ctx.player, ctx.request.cost.clone()));
        if let Some((source, ability_index)) = self.activate_mana_first.take() {
            return ManaPaymentResponse::Activate { source, ability_index };
        }
        if self.cancel { self.saw_cancel = true; return ManaPaymentResponse::Cancel; }
        SelectFirstDecisionMaker.decide_mana_payment(game, ctx)
    }
}
fn finish(game: &mut GameState, queue: &mut TriggerQueue, state: &mut PriorityLoopState,
    mut progress: GameProgress, choices: &mut Choices)
{
    for _ in 0..64 {
        if !state.has_pending_action() { return; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("pending activation: {progress:?}"); };
        // Clone the native pending owner as well as the game at every public suspension.
        *game = game.clone();
        *state = state.clone();
        progress = apply_decision_context_with_dm(game, queue, state, &ctx, choices).unwrap();
    }
    panic!("bounded activation did not finish");
}
fn announce(game: &mut GameState, player: PlayerId, action: LegalAction, choices: &mut Choices) {
    game.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(3);
    let mut queue = TriggerQueue::new();
    let progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    finish(game, &mut queue, &mut state, progress, choices);
    assert!(!state.has_pending_action());
}
fn activate(game: &mut GameState, player: PlayerId, source: ObjectId, ability_index: usize, choices: &mut Choices) {
    let action = action(game, player, source, ability_index).expect("selected ability must be legal");
    announce(game, player, action, choices);
}
fn resolve(game: &mut GameState, choices: &mut Choices) {
    assert_eq!(game.stack.len(), 1);
    *game = game.clone();
    resolve_stack_entry_with(game, choices).unwrap();
    assert!(game.stack.is_empty());
}
fn unblocked(game: &mut GameState, attacker: ObjectId, defender: PlayerId) {
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    game.combat = Some(CombatState { block_declaration_complete: true,
        attackers: vec![AttackerInfo { creature: attacker, target: AttackTarget::Player(defender) }],
        ..Default::default() });
    game.mark_creature_attacked_this_turn(attacker);
}

#[test]
fn eight_exact_frozen_full_bodies_compile_without_loss_and_round_trip_typed_identity() {
    assert_eq!(rows().len(), 8);
    for row in rows() {
        assert_eq!(row["baseline_category"], "parser_failure");
        assert_eq!(row["validation"], "authored_not_run");
        assert!(row["baseline_parse_error"].as_str().unwrap().len() > 20);
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            let expected = match name {
                "Silver-Fur Master" => Some(Keyword::Ninjutsu),
                "Dragonkin Berserker" => Some(Keyword::Boast),
                "Boom Scholar" => Some(Keyword::Exhaust),
                "Hulk, Gamma Goliath" => Some(Keyword::PowerUp),
                _ => None,
            };
            if let Some(expected) = expected {
                assert!(definition.abilities.iter().any(|ability| {
                    matches!(&ability.kind, AbilityKind::Activated(activated) if activated.keyword == Some(expected))
                }));
            }
            if name == "Zirda, the Dawnwaker" {
                assert!(definition.abilities.iter().any(|ability| {
                    matches!(&ability.kind, AbilityKind::Static(static_ability)
                        if static_ability.companion_deck_condition() == Some(&ironsmith_core::CompanionDeckCondition::PermanentsHaveActivatedAbility))
                }), "the frozen companion sentence is preserved as a deck constraint");
            }
        }
    }
}

#[test]
fn typed_keyword_selectors_do_not_discount_identical_ordinary_ability_twins() {
    let cases = [
        ("Fluctuator", Keyword::Cycling, "Type: Creature — Human\nPower/Toughness: 2/2\nCycling {4}{U}", Zone::Hand, 2),
        ("Silver-Fur Master", Keyword::Ninjutsu, "Type: Creature — Human Ninja\nPower/Toughness: 2/2\nNinjutsu {4}{U}", Zone::Hand, 3),
        ("Dragonkin Berserker", Keyword::Boast, "Type: Creature — Human\nPower/Toughness: 2/2\nBoast — {4}{U}: You gain 1 life.", Zone::Battlefield, 3),
        ("Boom Scholar", Keyword::Exhaust, "Type: Creature — Human\nPower/Toughness: 2/2\nExhaust — {4}{U}: You gain 1 life.", Zone::Battlefield, 2),
        ("Hulk, Gamma Goliath", Keyword::PowerUp, "Type: Creature — Human\nPower/Toughness: 2/2\nPower-up — {4}{U}: You gain 1 life.", Zone::Battlefield, 1),
    ];
    for (name, keyword, host_text, zone, remaining) in cases {
        for (modifier, mut host) in definitions(name).into_iter().zip(definitions_text("Identity host", host_text)) {
            let typed = host.abilities.iter().position(|ability| matches!(&ability.kind,
                AbilityKind::Activated(activated) if activated.keyword == Some(keyword))).unwrap();
            let mut ordinary = host.abilities[typed].clone();
            let AbilityKind::Activated(activated) = &mut ordinary.kind else { unreachable!(); };
            activated.keyword = None;
            let ordinary_index = host.abilities.len();
            host.abilities.push(ordinary);
            let mut game = game();
            game.create_object_from_definition(&modifier, A, Zone::Battlefield);
            printed(&mut game, "Dragon witness", "Type: Creature — Dragon\nPower/Toughness: 2/2", A, Zone::Battlefield);
            let source = game.create_object_from_definition(&host, A, zone);
            assert_price(&game, A, source, typed, vec![ManaSymbol::Generic(remaining), ManaSymbol::Blue]);
            assert_price(&game, A, source, ordinary_index, vec![ManaSymbol::Generic(4), ManaSymbol::Blue]);
            let selected = ActivationCostAbility::at(&game, A, source, typed).unwrap();
            assert_eq!(selected.keyword, Some(keyword));
            assert_eq!(selected.activator, Some(A));
            assert_eq!(ActivationCostAbility::at(&game, A, source, ordinary_index).unwrap().keyword, None);
        }
    }
}

#[test]
fn fluctuator_and_suppression_field_stack_tax_before_reduction_keep_blue_and_discard() {
    for (fluctuator, field) in definitions("Fluctuator").into_iter().zip(definitions("Suppression Field")) {
        for host in definitions_text("Cycling witness", "Type: Creature — Human\nPower/Toughness: 2/2\nCycling {3}{U}") {
            let mut game = game();
            game.create_object_from_definition(&fluctuator, A, Zone::Battlefield);
            game.create_object_from_definition(&field, B, Zone::Battlefield);
            let source = game.create_object_from_definition(&host, A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            let n = keyword_index(&game, source, Keyword::Cycling);
            assert_price(&game, A, source, n, vec![ManaSymbol::Generic(3), ManaSymbol::Blue]);
            library(&mut game, A, 2);
            mana(&mut game, A, ManaSymbol::Colorless, 3);
            assert!(action(&game, A, source, n).is_none(), "generic reduction never substitutes blue");
            mana(&mut game, A, ManaSymbol::Blue, 1);
            let mut choices = Choices::default();
            activate(&mut game, A, source, n, &mut choices);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            let discarded = game.find_object_by_stable_id(stable).unwrap();
            assert_ne!(discarded, source);
            assert_eq!(game.object(discarded).unwrap().zone, Zone::Graveyard);
            assert!(choices.mana_requests.iter().any(|(payer, cost)| *payer == A && cost.mana_value() == 4));
            resolve(&mut game, &mut choices);
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert_eq!(game.player(A).unwrap().library.len(), 1);
            assert!(game.player(B).unwrap().hand.is_empty());
        }
    }
}

#[test]
fn fluctuator_reduction_stops_at_zero_and_never_applies_to_an_opponent_cycling() {
    for fluctuator in definitions("Fluctuator") {
        for host in definitions_text("Cheap cycle", "Type: Creature — Human\nPower/Toughness: 2/2\nCycling {1}") {
            let mut game = game();
            game.create_object_from_definition(&fluctuator, A, Zone::Battlefield);
            let own = game.create_object_from_definition(&host, A, Zone::Hand);
            let foreign = game.create_object_from_definition(&host, B, Zone::Hand);
            library(&mut game, A, 1);
            let own_n = keyword_index(&game, own, Keyword::Cycling);
            let foreign_n = keyword_index(&game, foreign, Keyword::Cycling);
            assert!(action(&game, A, own, own_n).is_some());
            assert!(action(&game, B, foreign, foreign_n).is_none());
            assert_eq!(price(&game, B, foreign, foreign_n).mana_cost().unwrap().mana_value(), 1);
            activate(&mut game, A, own, own_n, &mut Choices::default());
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert_eq!(game.object(foreign).unwrap().zone, Zone::Hand);
        }
    }
}

#[test]
fn cycling_replaced_discard_is_paid_once_and_keeps_the_replacement_controller() {
    for modifier in definitions("Fluctuator") {
        for prevent in [false, true] {
            let mut game = game();
            game.create_object_from_definition(&modifier, A, Zone::Battlefield);
            let source = printed(&mut game, "Replaced cycle", "Type: Creature — Human\nPower/Toughness: 2/2\nCycling {3}{U}", A, Zone::Hand);
            let replacement_source = creature(&mut game, "Bob replacement", B, Zone::Battlefield);
            let replacement = if prevent { ReplacementAction::Prevent }
                else { ReplacementAction::Instead(vec![Effect::gain_life(3)]) };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                replacement_source, B, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ironsmith::target::ObjectFilter::specific(source), Some(Zone::Hand), Some(Zone::Graveyard)), replacement));
            library(&mut game, A, 1);
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            mana(&mut game, A, ManaSymbol::Blue, 1);
            let n = keyword_index(&game, source, Keyword::Cycling);
            activate(&mut game, A, source, n, &mut Choices::default());
            assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(B).unwrap().life, if prevent { 20 } else { 23 });
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), 2);
            assert_eq!(game.player(B).unwrap().life, if prevent { 20 } else { 23 });
        }
    }
}

#[test]
fn silver_fur_full_anthem_and_ninjutsu_keep_colored_cost_unblocked_return_and_defender() {
    for master in definitions("Silver-Fur Master") {
        for host in definitions_text("Costed Ninja", "Type: Creature — Human Ninja\nPower/Toughness: 2/2\nNinjutsu {2}{U}{B}") {
            let mut game = game();
            let modifier = game.create_object_from_definition(&master, A, Zone::Battlefield);
            let rogue = printed(&mut game, "Rogue witness", "Type: Creature — Human Rogue\nPower/Toughness: 2/2", A, Zone::Battlefield);
            let opponent_ninja = game.create_object_from_definition(&host, B, Zone::Battlefield);
            assert_eq!(game.calculated_power(modifier), Some(2));
            assert_eq!(game.calculated_power(rogue), Some(3));
            assert_eq!(game.calculated_power(opponent_ninja), Some(2));
            let source = game.create_object_from_definition(&host, A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            let n = keyword_index(&game, source, Keyword::Ninjutsu);
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            mana(&mut game, A, ManaSymbol::Blue, 1);
            mana(&mut game, A, ManaSymbol::Black, 1);
            assert!(action(&game, A, source, n).is_none(), "ninjutsu still requires an unblocked attacker");
            let attacker = creature(&mut game, "Borrowed attacker", B, Zone::Battlefield);
            game.object_mut(attacker).unwrap().initial_controller = A;
            let returned_stable = game.object(attacker).unwrap().stable_id;
            unblocked(&mut game, attacker, C);
            assert_price(&game, A, source, n, vec![ManaSymbol::Generic(1), ManaSymbol::Blue, ManaSymbol::Black]);
            let mut choices = Choices { object: Some(attacker), ..Default::default() };
            activate(&mut game, A, source, n, &mut choices);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            let returned = game.find_object_by_stable_id(returned_stable).unwrap();
            assert_eq!(game.object(returned).unwrap().zone, Zone::Hand);
            assert!(game.player(B).unwrap().hand.contains(&returned), "return goes to its owner");
            game.move_object_by_effect(modifier, Zone::Graveyard).unwrap();
            resolve(&mut game, &mut choices);
            let entered = game.find_object_by_stable_id(stable).unwrap();
            assert_ne!(entered, source);
            assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
            assert!(game.is_tapped(entered));
            assert_eq!(game.current_controller(entered), Some(A));
            assert!(game.combat.as_ref().unwrap().attackers.iter().any(|attack| {
                attack.creature == entered && attack.target == AttackTarget::Player(C)
            }));
        }
    }
}

#[test]
fn ninjutsu_replaced_return_stays_paid_and_does_not_transfer_the_replacement_program_to_activator() {
    for master in definitions("Silver-Fur Master") {
        for prevent in [false, true] {
            let mut game = game();
            let modifier = game.create_object_from_definition(&master, A, Zone::Battlefield);
            let source = printed(&mut game, "Replaced Ninja", "Type: Creature — Human Ninja\nPower/Toughness: 2/2\nNinjutsu {2}{U}", A, Zone::Hand);
            let attacker = creature(&mut game, "Attacker", A, Zone::Battlefield);
            let replacement_source = creature(&mut game, "Replacement controller", B, Zone::Battlefield);
            unblocked(&mut game, attacker, C);
            let replacement = if prevent { ReplacementAction::Prevent }
                else { ReplacementAction::Instead(vec![Effect::gain_life(3)]) };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                replacement_source, B, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ironsmith::target::ObjectFilter::specific(attacker), Some(Zone::Battlefield), Some(Zone::Hand)), replacement));
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            mana(&mut game, A, ManaSymbol::Blue, 1);
            let stable = game.object(source).unwrap().stable_id;
            let n = keyword_index(&game, source, Keyword::Ninjutsu);
            let mut choices = Choices { object: Some(attacker), ..Default::default() };
            activate(&mut game, A, source, n, &mut choices);
            assert_eq!(game.object(attacker).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(B).unwrap().life, if prevent { 20 } else { 23 });
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            game.move_object_by_effect(modifier, Zone::Graveyard).unwrap();
            resolve(&mut game, &mut choices);
            let entered = game.find_object_by_stable_id(stable).unwrap();
            assert!(game.is_tapped(entered));
            assert!(game.combat.as_ref().unwrap().attackers.iter().any(|attack| {
                attack.creature == entered && attack.target == AttackTarget::Player(C)
            }));
            assert_eq!(game.player(B).unwrap().life, if prevent { 20 } else { 23 });
        }
    }
}

#[test]
fn dragonkin_counts_only_current_controlled_dragons_and_respects_attack_once_and_red() {
    for berserker in definitions("Dragonkin Berserker") {
        let mut game = game();
        let source = game.create_object_from_definition(&berserker, A, Zone::Battlefield);
        let n = keyword_index(&game, source, Keyword::Boast);
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::FirstStrike));
        let one = printed(&mut game, "First Dragon", "Type: Creature — Dragon\nPower/Toughness: 2/2", A, Zone::Battlefield);
        let two = printed(&mut game, "Second Dragon", "Type: Creature — Dragon\nPower/Toughness: 2/2", A, Zone::Battlefield);
        printed(&mut game, "Foreign Dragon", "Type: Creature — Dragon\nPower/Toughness: 2/2", B, Zone::Battlefield);
        assert_price(&game, A, source, n, vec![ManaSymbol::Generic(2), ManaSymbol::Red]);
        mana(&mut game, A, ManaSymbol::Colorless, 2);
        mana(&mut game, A, ManaSymbol::Red, 1);
        assert!(action(&game, A, source, n).is_none(), "the discount does not waive attacked-this-turn");
        game.mark_creature_attacked_this_turn(source);
        let cached = action(&game, A, source, n).unwrap();
        game.move_object_by_effect(two, Zone::Graveyard).unwrap();
        assert_price(&game, A, source, n, vec![ManaSymbol::Generic(3), ManaSymbol::Red]);
        assert!(action(&game, A, source, n).is_none(), "cost changes when a Dragon leaves");
        let mut state = PriorityLoopState::new(3);
        assert!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut state,
            &PriorityResponse::PriorityAction(cached), &mut Choices::default()).is_err());
        assert!(!state.has_pending_action());
        assert!(game.stack.is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
        mana(&mut game, A, ManaSymbol::Colorless, 1);
        activate(&mut game, A, source, n, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut Choices::default());
        let dragons = game.battlefield.iter().copied().filter(|id| game.current_controller(*id) == Some(A)
            && game.current_has_subtype(*id, Subtype::Dragon)).collect::<Vec<_>>();
        assert_eq!(dragons.len(), 2);
        let token = *dragons.iter().find(|id| **id != one).unwrap();
        assert_eq!(game.calculated_power(token), Some(5));
        assert_eq!(game.calculated_toughness(token), Some(5));
        assert!(game.current_has_static_ability_id(token, StaticAbilityId::Flying));
        mana(&mut game, A, ManaSymbol::Red, 10);
        assert!(action(&game, A, source, n).is_none(), "a discount never resets the boast once-per-turn use");
    }
}

#[test]
fn boom_scholar_excludes_itself_and_opponents_while_its_full_exhaust_resolves_all_clauses() {
    for scholar in definitions("Boom Scholar") {
        let mut game = game();
        let source = game.create_object_from_definition(&scholar, A, Zone::Battlefield);
        let n = keyword_index(&game, source, Keyword::Exhaust);
        let other = printed(&mut game, "Other exhaust", "Type: Artifact\nExhaust — {4}{R}{G}: You gain 1 life.", A, Zone::Battlefield);
        let foreign = printed(&mut game, "Foreign exhaust", "Type: Artifact\nExhaust — {4}{R}{G}: You gain 1 life.", B, Zone::Battlefield);
        let vehicle = printed(&mut game, "Vehicle witness", "Type: Artifact — Vehicle\nPower/Toughness: 3/3", A, Zone::Battlefield);
        let own_creature = creature(&mut game, "Own creature", A, Zone::Battlefield);
        let foreign_creature = creature(&mut game, "Opponent creature", B, Zone::Battlefield);
        assert_price(&game, A, source, n, vec![ManaSymbol::Generic(4), ManaSymbol::Red, ManaSymbol::Green]);
        assert_price(&game, A, other, keyword_index(&game, other, Keyword::Exhaust), vec![ManaSymbol::Generic(2), ManaSymbol::Red, ManaSymbol::Green]);
        assert_price(&game, B, foreign, keyword_index(&game, foreign, Keyword::Exhaust), vec![ManaSymbol::Generic(4), ManaSymbol::Red, ManaSymbol::Green]);
        mana(&mut game, A, ManaSymbol::Colorless, 2);
        mana(&mut game, A, ManaSymbol::Red, 1);
        mana(&mut game, A, ManaSymbol::Green, 1);
        assert!(action(&game, A, source, n).is_none(), "other never discounts the modifier itself");
        mana(&mut game, A, ManaSymbol::Colorless, 2);
        activate(&mut game, A, source, n, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
        for id in [source, own_creature, vehicle] { assert!(game.current_has_static_ability_id(id, StaticAbilityId::Trample)); }
        assert!(!game.current_has_static_ability_id(foreign_creature, StaticAbilityId::Trample));
        game.next_turn(); main_phase(&mut game, A);
        mana(&mut game, A, ManaSymbol::Red, 10);
        mana(&mut game, A, ManaSymbol::Green, 10);
        assert!(action(&game, A, source, n).is_none(), "exhaust is once per object even across turns");
    }
}

#[test]
fn exhaust_other_ability_discount_cannot_leak_and_cancel_restores_mana_source_tap_and_once_limit() {
    for modifier in definitions("Boom Scholar") {
        for host in definitions_text("Two abilities", "Type: Artifact\nExhaust — {3}{R}, {T}, Remove a charge counter from this artifact: You gain 2 life.\n{3}{R}, {T}, Remove a charge counter from this artifact: You gain 2 life.") {
            let mut game = game();
            game.create_object_from_definition(&modifier, A, Zone::Battlefield);
            let source = game.create_object_from_definition(&host, A, Zone::Battlefield);
            game.add_counters(source, CounterType::Charge, 1);
            let typed = keyword_index(&game, source, Keyword::Exhaust);
            let ordinary = index(&game, source, 1);
            let mana_source = printed(&mut game, "Red source", "Type: Artifact\n{T}: Add {R}.", A, Zone::Battlefield);
            let mana_n = index(&game, mana_source, 0);
            mana(&mut game, A, ManaSymbol::Colorless, 3);
            let stale_ordinary = action(&game, A, source, ordinary).unwrap();
            game.player_mut(A).unwrap().mana_pool = Default::default();
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            assert!(action(&game, A, source, ordinary).is_none(), "the source also has exhaust; this ordinary ability still costs four");
            let mut rejected_state = PriorityLoopState::new(3);
            assert!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut rejected_state,
                &PriorityResponse::PriorityAction(stale_ordinary), &mut Choices::default()).is_err());
            assert!(!rejected_state.has_pending_action());
            assert!(!game.is_tapped(source) && !game.is_tapped(mana_source));
            assert_eq!(game.counter_count(source, CounterType::Charge), 1);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
            assert_price(&game, A, source, typed, vec![ManaSymbol::Generic(1), ManaSymbol::Red]);
            let mut choices = Choices { cancel: true, activate_mana_first: Some((mana_source, mana_n)), ..Default::default() };
            activate(&mut game, A, source, typed, &mut choices);
            assert!(choices.saw_cancel);
            assert!(game.stack.is_empty());
            assert!(!game.is_tapped(source));
            assert!(!game.is_tapped(mana_source), "the nested mana activation rolls back with its payer");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
            assert_eq!(game.counter_count(source, CounterType::Charge), 1);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert!(action(&game, A, source, typed).is_some());
            activate(&mut game, A, source, typed, &mut Choices::default());
            assert!(game.is_tapped(source));
            assert!(game.is_tapped(mana_source));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.counter_count(source, CounterType::Charge), 0);
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().life, 22);
            game.untap(source); game.untap(mana_source);
            game.add_counters(source, CounterType::Charge, 1);
            mana(&mut game, A, ManaSymbol::Colorless, 5);
            assert!(action(&game, A, source, typed).is_none());
            assert!(action(&game, A, source, ordinary).is_some());
        }
    }
}

#[test]
fn hulk_discounts_only_other_controlled_creature_power_up_and_preserves_own_full_body() {
    for hulk in definitions("Hulk, Gamma Goliath") {
        let mut game = game();
        let source = game.create_object_from_definition(&hulk, A, Zone::Battlefield);
        let creature_text = "Type: Creature — Human\nPower/Toughness: 2/2\nPower-up — {6}{R}{G}: Put five +1/+1 counters on this creature.\n{6}{R}{G}: Put five +1/+1 counters on this creature.";
        let other = printed(&mut game, "Other power-up", creature_text, A, Zone::Battlefield);
        let foreign = printed(&mut game, "Foreign power-up", creature_text, B, Zone::Battlefield);
        let artifact = printed(&mut game, "Noncreature power-up", "Type: Artifact\nPower-up — {6}{R}{G}: You gain 1 life.", A, Zone::Battlefield);
        game.next_turn(); main_phase(&mut game, A);
        let n = keyword_index(&game, source, Keyword::PowerUp);
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Reach));
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Trample));
        assert_price(&game, A, source, n, vec![ManaSymbol::Generic(6), ManaSymbol::Red, ManaSymbol::Green]);
        assert_price(&game, A, other, keyword_index(&game, other, Keyword::PowerUp), vec![ManaSymbol::Generic(3), ManaSymbol::Red, ManaSymbol::Green]);
        assert_price(&game, A, other, index(&game, other, 1), vec![ManaSymbol::Generic(6), ManaSymbol::Red, ManaSymbol::Green]);
        assert_price(&game, B, foreign, keyword_index(&game, foreign, Keyword::PowerUp), vec![ManaSymbol::Generic(6), ManaSymbol::Red, ManaSymbol::Green]);
        assert_price(&game, A, artifact, keyword_index(&game, artifact, Keyword::PowerUp), vec![ManaSymbol::Generic(6), ManaSymbol::Red, ManaSymbol::Green]);
        mana(&mut game, A, ManaSymbol::Colorless, 3);
        mana(&mut game, A, ManaSymbol::Red, 1);
        mana(&mut game, A, ManaSymbol::Green, 1);
        assert!(action(&game, A, source, n).is_none());
        let other_n = keyword_index(&game, other, Keyword::PowerUp);
        activate(&mut game, A, other, other_n, &mut Choices::default());
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.counter_count(other, CounterType::PlusOnePlusOne), 5);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        mana(&mut game, A, ManaSymbol::Colorless, 6);
        mana(&mut game, A, ManaSymbol::Red, 1);
        mana(&mut game, A, ManaSymbol::Green, 1);
        activate(&mut game, A, source, n, &mut Choices::default());
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 5);
        assert_eq!(game.calculated_power(source), Some(11));
        assert_eq!(game.calculated_toughness(source), Some(10));
    }
}

#[test]
fn hulk_entry_turn_intrinsic_discount_combines_with_another_hulks_other_creature_discount() {
    for definition in definitions("Hulk, Gamma Goliath") {
        let mut game = game();
        // The other reducer is a different permanent. Changing its name avoids the legend rule
        // while preserving its complete typed static program, including the source exclusion.
        let mut reducer = definition.clone();
        reducer.card.name = "Other Hulk witness".into();
        game.create_object_from_definition(&reducer, A, Zone::Battlefield);
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let receipt = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut Choices::default()).unwrap();
        assert!(!receipt.pending && receipt.programs.is_empty());
        let source = receipt.original.into_result().unwrap().new_id;
        let n = keyword_index(&game, source, Keyword::PowerUp);
        let total = price(&game, A, source, n);
        assert_eq!(total.dynamic_mana_cost().unwrap().base,
            ManaCost::from_symbols(vec![ManaSymbol::Generic(3), ManaSymbol::Red, ManaSymbol::Green]));
        assert!(action(&game, A, source, n).is_some(),
            "the other Hulk removes {{3}}; the intrinsic entry reduction removes the remaining {{3}}{{R}}{{G}}");
        activate(&mut game, A, source, n, &mut Choices::default());
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 5);
    }
}

#[test]
fn eidolon_taxes_only_opponents_loyalty_abilities_and_keeps_counter_payment_and_frequency() {
    for eidolon in definitions("Eidolon of Obstruction") {
        for walker in definitions_text("Two walker abilities", "Type: Planeswalker — Jace\nLoyalty: 3\n-1: You gain 2 life.\n{0}: You gain 1 life.") {
            let mut game = game();
            let modifier = game.create_object_from_definition(&eidolon, A, Zone::Battlefield);
            assert!(game.current_has_static_ability_id(modifier, StaticAbilityId::FirstStrike));
            let own = game.create_object_from_definition(&walker, A, Zone::Battlefield);
            let foreign = game.create_object_from_definition(&walker, B, Zone::Battlefield);
            let own_n = index(&game, own, 0);
            let foreign_n = index(&game, foreign, 0);
            assert!(ActivationCostAbility::at(&game, B, foreign, foreign_n).unwrap().loyalty_ability);
            assert_eq!(price(&game, A, own, own_n).mana_cost().map(ManaCost::mana_value).unwrap_or(0), 0);
            assert_price(&game, B, foreign, foreign_n, vec![ManaSymbol::Generic(1)]);
            main_phase(&mut game, B);
            assert!(action(&game, B, foreign, foreign_n).is_none());
            assert!(action(&game, B, foreign, index(&game, foreign, 1)).is_some(), "an ordinary ability of the same planeswalker is not a loyalty ability");
            mana(&mut game, B, ManaSymbol::Colorless, 1);
            activate(&mut game, B, foreign, foreign_n, &mut Choices::default());
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
            assert_eq!(game.counter_count(foreign, CounterType::Loyalty), 2);
            assert_eq!(game.counter_count(own, CounterType::Loyalty), 3);
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(B).unwrap().life, 22);
            assert_eq!(game.player(A).unwrap().life, 20);
            mana(&mut game, B, ManaSymbol::Colorless, 1);
            assert!(action(&game, B, foreign, foreign_n).is_none());
        }
    }
}

#[test]
fn suppression_field_leaves_mana_abilities_untaxed_but_taxes_same_source_draw_and_loyalty_mana() {
    for field in definitions("Suppression Field") {
        for host in definitions_text("Mana and ordinary", "Type: Artifact\n{2}, {T}: Add {G}{G}{G}.\n{2}, {T}: Draw a card.") {
            let mut game = game();
            game.create_object_from_definition(&field, C, Zone::Battlefield);
            let source = game.create_object_from_definition(&host, A, Zone::Battlefield);
            let mana_n = index(&game, source, 0);
            let draw_n = index(&game, source, 1);
            assert!(ActivationCostAbility::at(&game, A, source, mana_n).unwrap().mana_ability);
            assert!(!ActivationCostAbility::at(&game, A, source, draw_n).unwrap().mana_ability);
            assert_price(&game, A, source, mana_n, vec![ManaSymbol::Generic(2)]);
            assert_price(&game, A, source, draw_n, vec![ManaSymbol::Generic(4)]);
            mana(&mut game, A, ManaSymbol::Colorless, 2);
            assert!(action(&game, A, source, draw_n).is_none());
            activate(&mut game, A, source, mana_n, &mut Choices::default());
            assert!(game.stack.is_empty());
            assert!(game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().mana_pool.green, 3);
            game.untap(source); library(&mut game, A, 1);
            assert!(action(&game, A, source, draw_n).is_none());
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            activate(&mut game, A, source, draw_n, &mut Choices::default());
            assert!(game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            let walker = printed(&mut game, "Loyalty adds mana", "Type: Planeswalker — Jace\nLoyalty: 3\n+1: Add {G}.", B, Zone::Battlefield);
            main_phase(&mut game, B);
            let n = index(&game, walker, 0);
            let facts = ActivationCostAbility::at(&game, B, walker, n).unwrap();
            assert!(facts.loyalty_ability && !facts.mana_ability);
            assert_price(&game, B, walker, n, vec![ManaSymbol::Generic(2)]);
            assert!(action(&game, B, walker, n).is_none());
            mana(&mut game, B, ManaSymbol::Colorless, 2);
            activate(&mut game, B, walker, n, &mut Choices::default());
            assert_eq!(game.counter_count(walker, CounterType::Loyalty), 4);
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(B).unwrap().mana_pool.green, 1);
        }
    }
}

#[test]
fn zirda_uses_the_activator_not_source_controller_preserves_colored_mana_and_minimum() {
    for zirda in definitions("Zirda, the Dawnwaker") {
        for host in definitions_text("Shared activation", "Type: Artifact\n{3}: You gain 2 life. Any player may activate this ability but only during their turn before the end step.") {
            let mut game = game();
            game.create_object_from_definition(&zirda, A, Zone::Battlefield);
            let source = game.create_object_from_definition(&host, B, Zone::Battlefield);
            let n = index(&game, source, 0);
            assert_price(&game, A, source, n, vec![ManaSymbol::Generic(1)]);
            assert_price(&game, B, source, n, vec![ManaSymbol::Generic(3)]);
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            activate(&mut game, A, source, n, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().life, 22);
            assert_eq!(game.player(B).unwrap().life, 20);
            main_phase(&mut game, B);
            mana(&mut game, B, ManaSymbol::Colorless, 1);
            assert!(action(&game, B, source, n).is_none());
            mana(&mut game, B, ManaSymbol::Colorless, 2);
            activate(&mut game, B, source, n, &mut Choices::default());
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(B).unwrap().life, 22);
        }
        for (cost, generic, blue) in [("{3}", 1, 0), ("{1}{U}", 0, 1), ("{1}", 1, 0)] {
            let mut game = game();
            game.create_object_from_definition(&zirda, A, Zone::Battlefield);
            let text = format!("Type: Artifact\n{cost}, {{T}}, Pay 2 life: Draw a card.");
            let source = printed(&mut game, "Minimum witness", &text, A, Zone::Battlefield);
            let n = index(&game, source, 0);
            library(&mut game, A, 1);
            assert!(action(&game, A, source, n).is_none());
            mana(&mut game, A, ManaSymbol::Colorless, generic);
            mana(&mut game, A, ManaSymbol::Blue, blue);
            activate(&mut game, A, source, n, &mut Choices::default());
            assert!(game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().life, 18);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
        }
    }
}

#[test]
fn zirda_does_not_raise_mana_free_cost_or_discount_mana_abilities_and_own_tap_ability_resolves() {
    for zirda in definitions("Zirda, the Dawnwaker") {
        let mut game = game();
        let modifier = game.create_object_from_definition(&zirda, A, Zone::Battlefield);
        game.remove_summoning_sickness(modifier);
        let source = printed(&mut game, "Three kinds", "Type: Artifact\n{T}: You gain 1 life.\n{3}, {T}: Add {G}{G}{G}{G}.\n{3}, {T}: You gain 2 life.", A, Zone::Battlefield);
        let free_n = index(&game, source, 0);
        let mana_n = index(&game, source, 1);
        let ordinary_n = index(&game, source, 2);
        assert_eq!(price(&game, A, source, free_n).mana_cost().map(ManaCost::mana_value).unwrap_or(0), 0);
        assert_price(&game, A, source, mana_n, vec![ManaSymbol::Generic(3)]);
        assert_price(&game, A, source, ordinary_n, vec![ManaSymbol::Generic(1)]);
        activate(&mut game, A, source, free_n, &mut Choices::default());
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, 21);
        game.untap(source);
        mana(&mut game, A, ManaSymbol::Colorless, 1);
        assert!(action(&game, A, source, mana_n).is_none());
        let victim = creature(&mut game, "Blocking victim", B, Zone::Battlefield);
        let attacker = creature(&mut game, "Attack witness", A, Zone::Battlefield);
        let n = index(&game, modifier, 0);
        assert_price(&game, A, modifier, n, vec![ManaSymbol::Generic(1)]);
        activate(&mut game, A, modifier, n, &mut Choices { target: Some(Target::Object(victim)), ..Default::default() });
        assert!(game.is_tapped(modifier));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut Choices::default());
        assert!(!ironsmith::rules::combat::can_block(
            game.object(attacker).unwrap(), game.object(victim).unwrap(), &game));
    }
}

#[test]
fn pending_target_activation_keeps_original_kind_cost_and_program_when_current_slot_changes() {
    for modifier in definitions("Boom Scholar") {
        for host in definitions_text("Announced exhaust", "Type: Artifact\nExhaust — {3}{R}, {T}, Pay 2 life: Put two +1/+1 counters on target creature.") {
            let mut game = game();
            game.create_object_from_definition(&modifier, A, Zone::Battlefield);
            let source = game.create_object_from_definition(&host, A, Zone::Battlefield);
            let target = creature(&mut game, "Target", A, Zone::Battlefield);
            let n = keyword_index(&game, source, Keyword::Exhaust);
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            mana(&mut game, A, ManaSymbol::Red, 1);
            let next = action(&game, A, source, n).unwrap();
            let mut queue = TriggerQueue::new();
            let mut state = PriorityLoopState::new(3);
            let mut choices = Choices { target: Some(Target::Object(target)), ..Default::default() };
            let progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(next), &mut choices).unwrap();
            assert!(matches!(progress, GameProgress::NeedsDecisionCtx(DecisionContext::Targets(_))));
            let announced = state.pending_activation.as_ref().unwrap().announced_cost.as_ref().unwrap();
            assert_eq!(announced.ability.keyword, Some(Keyword::Exhaust));
            assert_eq!(announced.facts.keyword, Some(Keyword::Exhaust));
            assert_eq!(announced.facts.activator, Some(A));
            let replacement = compile_to_runtime_definition("New current slot",
                "Type: Artifact\n{9}{U}, Sacrifice this artifact: Target creature gets -5/-5 until end of turn.", false).unwrap();
            game.object_mut(source).unwrap().abilities = replacement.abilities.into();
            assert_eq!(ActivationCostAbility::at(&game, A, source, n).unwrap().keyword, None);
            assert_price(&game, A, source, n, vec![ManaSymbol::Generic(9), ManaSymbol::Blue]);
            // Both the game owner and pending lane recover with the original announcement.
            game = game.clone(); state = state.clone(); queue = queue.clone();
            finish(&mut game, &mut queue, &mut state, progress, &mut choices);
            assert!(game.is_tapped(source));
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.player(A).unwrap().life, 18);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(choices.mana_requests.iter().all(|(payer, cost)| *payer == A && cost.mana_value() == 2));
            resolve(&mut game, &mut choices);
            assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 2);
            assert_eq!(game.calculated_power(target), Some(4));
        }
    }
}

#[test]
fn missing_announced_cost_owner_returns_typed_incomplete_evidence_and_root_rollback_restores_resources() {
    for modifier in definitions("Boom Scholar") {
        for host in definitions_text("Missing owner exhaust", "Type: Artifact\nExhaust — {3}{R}, {T}, Pay 2 life: Put two +1/+1 counters on target creature.") {
            let mut game = game();
            game.create_object_from_definition(&modifier, A, Zone::Battlefield);
            let source = game.create_object_from_definition(&host, A, Zone::Battlefield);
            let target = creature(&mut game, "Target", A, Zone::Battlefield);
            let n = keyword_index(&game, source, Keyword::Exhaust);
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            mana(&mut game, A, ManaSymbol::Red, 1);
            let next = action(&game, A, source, n).unwrap();
            let mut queue = TriggerQueue::new();
            let mut state = PriorityLoopState::new(3);
            let progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(next), &mut Choices::default()).unwrap();
            assert!(matches!(progress, GameProgress::NeedsDecisionCtx(DecisionContext::Targets(_))));
            state.pending_activation.as_mut().unwrap().announced_cost = None;
            game = game.clone(); state = state.clone();
            let result = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::Targets(vec![Target::Object(target)]), &mut Choices::default());
            assert!(matches!(result, Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(
                ironsmith::effects::ExecutionError::IncompleteEvidence(_)))));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert!(!game.is_tapped(source));
            assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 0);
            assert!(game.stack.is_empty());
            assert!(state.pending_activation.as_ref().unwrap().announced_cost.is_none(),
                "lane restore preserves missing evidence; it must not invent it from the current slot");
            assert!(state.rollback_action(&mut game));
            assert!(!state.has_pending_action());
            assert!(state.checkpoint.is_none());
            assert!(action(&game, A, source, n).is_some(), "root rollback refunds exhaust's once limit");
            activate(&mut game, A, source, n,
                &mut Choices { target: Some(Target::Object(target)), ..Default::default() });
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 2);
        }
    }
}

#[test]
fn silver_furs_own_full_ninjutsu_keeps_both_colors_and_activates_the_new_anthem_after_entry() {
    for master in definitions("Silver-Fur Master") {
        let mut game = game();
        let first = game.create_object_from_definition(&master, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&master, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        let attacker = creature(&mut game, "Return witness", A, Zone::Battlefield);
        unblocked(&mut game, attacker, B);
        let n = keyword_index(&game, source, Keyword::Ninjutsu);
        assert_price(&game, A, source, n, vec![ManaSymbol::Blue, ManaSymbol::Black]);
        mana(&mut game, A, ManaSymbol::Blue, 1);
        assert!(action(&game, A, source, n).is_none(), "the generic reduction cannot pay black");
        mana(&mut game, A, ManaSymbol::Black, 1);
        let mut choices = Choices { object: Some(attacker), ..Default::default() };
        activate(&mut game, A, source, n, &mut choices);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut choices);
        let entered = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.is_tapped(entered));
        assert_eq!(game.calculated_power(first), Some(3));
        assert_eq!(game.calculated_toughness(first), Some(3));
        assert_eq!(game.calculated_power(entered), Some(3));
        assert_eq!(game.calculated_toughness(entered), Some(3));
        assert!(game.combat.as_ref().unwrap().attackers.iter().any(|attack| {
            attack.creature == entered && attack.target == AttackTarget::Player(B)
        }));
    }
}

#[test]
fn zirda_minimum_is_applied_after_suppression_tax_even_for_an_originally_mana_free_ability() {
    for (zirda, field) in definitions("Zirda, the Dawnwaker").into_iter().zip(definitions("Suppression Field")) {
        let mut game = game();
        game.create_object_from_definition(&zirda, A, Zone::Battlefield);
        game.create_object_from_definition(&field, B, Zone::Battlefield);
        let source = printed(&mut game, "Taxed free activation", "Type: Artifact\n{T}: You gain 2 life.", A, Zone::Battlefield);
        let n = index(&game, source, 0);
        assert_price(&game, A, source, n, vec![ManaSymbol::Generic(1)]);
        assert!(action(&game, A, source, n).is_none());
        mana(&mut game, A, ManaSymbol::Colorless, 1);
        activate(&mut game, A, source, n, &mut Choices::default());
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, 22);
    }
}

#[test]
fn announced_x_uses_the_original_cost_and_one_mana_floor_after_a_slot_change() {
    for zirda in definitions("Zirda, the Dawnwaker") {
        for host in definitions_text("Announced X witness", "Type: Artifact\nExhaust — {X}: You gain X life.") {
            let mut game = game();
            game.create_object_from_definition(&zirda, A, Zone::Battlefield);
            let source = game.create_object_from_definition(&host, A, Zone::Battlefield);
            let n = keyword_index(&game, source, Keyword::Exhaust);
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            let next = action(&game, A, source, n).unwrap();
            let mut queue = TriggerQueue::new();
            let mut state = PriorityLoopState::new(3);
            let mut choices = Choices::default();
            let progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(next), &mut choices).unwrap();
            let GameProgress::NeedsDecisionCtx(DecisionContext::Number(context)) = progress else {
                panic!("X must be announced before the discounted total is paid");
            };
            assert_eq!(context.max, 3);
            let replacement = compile_to_runtime_definition("Changed X slot",
                "Type: Artifact\n{0}: You gain 20 life.", false).unwrap();
            game.object_mut(source).unwrap().abilities = replacement.abilities.into();
            let current = game.current_ability(source, n).unwrap();
            let AbilityKind::Activated(current) = &current.kind else { panic!("current activation"); };
            assert!(!current.mana_cost.mana_cost().unwrap().has_x());
            game = game.clone(); state = state.clone();
            let progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::XValue(3), &mut choices).unwrap();
            finish(&mut game, &mut queue, &mut state, progress, &mut choices);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(choices.mana_requests.iter().all(|(_, cost)| cost.mana_value() == 1));
            resolve(&mut game, &mut choices);
            assert_eq!(game.player(A).unwrap().life, 23);
        }
    }
}

#[test]
fn generic_floor_order_is_chosen_before_unbounded_reduction_in_either_insertion_order() {
    for (fluctuator, zirda) in definitions("Fluctuator").into_iter().zip(definitions("Zirda, the Dawnwaker")) {
        for reverse in [false, true] {
            let mut game = game();
            let order = if reverse { [&zirda, &fluctuator] } else { [&fluctuator, &zirda] };
            for definition in order { game.create_object_from_definition(definition, A, Zone::Battlefield); }
            let source = printed(&mut game, "Ordered floor cycle", "Type: Creature — Human\nPower/Toughness: 1/1\nCycling {3}", A, Zone::Hand);
            library(&mut game, A, 1);
            let n = keyword_index(&game, source, Keyword::Cycling);
            assert_eq!(price(&game, A, source, n).mana_cost().unwrap().mana_value(), 0);
            assert!(action(&game, A, source, n).is_some());
            activate(&mut game, A, source, n, &mut Choices::default());
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        }
    }
}

#[test]
fn exhaust_mana_x_is_announced_once_and_reduced_after_locking_in_both_payment_owners() {
    for scholar in definitions("Boom Scholar") {
        for direct in [false, true] {
            let mut game = game();
            game.create_object_from_definition(&scholar, A, Zone::Battlefield);
            let source = printed(&mut game, "Exhaust mana X", "Type: Artifact\nExhaust — {X}, {T}: Add {G}.", A, Zone::Battlefield);
            let n = keyword_index(&game, source, Keyword::Exhaust);
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            let mut choices = Choices { x: Some(3), ..Default::default() };
            if direct {
                ironsmith::special_actions::perform_activate_mana_ability(&mut game, A, source, n, &mut choices).unwrap();
            } else { activate(&mut game, A, source, n, &mut choices); }
            assert_eq!(choices.x_bounds, vec![(0, 3)], "source's reserved tap cannot finance itself");
            assert_eq!(game.player(A).unwrap().mana_pool.colorless, 0);
            assert_eq!(game.player(A).unwrap().mana_pool.green, 1);
            assert!(game.is_tapped(source));
            assert!(game.stack.is_empty());
            game.untap(source);
            assert!(action(&game, A, source, n).is_none(), "exhaust is paid/recorded only once");
        }
    }
}

#[test]
fn hulk_power_up_dynamic_x_preserves_intrinsic_entry_reduction_and_announced_counter_amount() {
    for hulk in definitions("Hulk, Gamma Goliath") {
        for entered_this_turn in [false, true] {
            let mut game = game();
            game.create_object_from_definition(&hulk, A, Zone::Battlefield);
            let source = printed(&mut game, "Power-up X witness",
                "Mana cost: {2}\nType: Creature — Human\nPower/Toughness: 1/1\nPower-up — {X}: Put X +1/+1 counters on this creature.", A, Zone::Hand);
            let receipt = game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut Choices::default()).unwrap();
            let source = receipt.original.into_result().unwrap().new_id;
            if !entered_this_turn { game.next_turn(); main_phase(&mut game, A); }
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            let chosen = if entered_this_turn { 6 } else { 4 };
            let n = keyword_index(&game, source, Keyword::PowerUp);
            let mut choices = Choices { x: Some(chosen), ..Default::default() };
            activate(&mut game, A, source, n, &mut choices);
            assert_eq!(choices.x_bounds, vec![(0, chosen)]);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            resolve(&mut game, &mut choices);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), chosen);
        }
    }
}

#[test]
fn exhaust_mana_alternatives_select_one_original_branch_before_flattening_and_payment() {
    for scholar in definitions("Boom Scholar") {
        for direct in [false, true] {
            for chosen in [0, 1] {
                let mut game = game();
                game.create_object_from_definition(&scholar, A, Zone::Battlefield);
                let mut host = compile_to_runtime_definition("Alternative mana witness",
                    "Type: Artifact\nExhaust — {4}, {T}: Add {G}.", false).unwrap();
                let second = compile_to_runtime_definition("Second price",
                    "Type: Artifact\n{6}, {T}: Add {G}.", false).unwrap();
                let AbilityKind::Activated(second) = &second.abilities[0].kind else { panic!("second activation"); };
                let AbilityKind::Activated(first) = &mut host.abilities[0].kind else { panic!("first activation"); };
                first.mana_cost = ironsmith::cost::TotalCost::one_of(vec![first.mana_cost.clone(), second.mana_cost.clone()]);
                let source = game.create_object_from_definition(&host, A, Zone::Battlefield);
                let n = keyword_index(&game, source, Keyword::Exhaust);
                mana(&mut game, A, ManaSymbol::Colorless, 4);
                let mut choices = Choices { branch: Some(chosen), ..Default::default() };
                if direct {
                    ironsmith::special_actions::perform_activate_mana_ability(&mut game, A, source, n, &mut choices).unwrap();
                } else { activate(&mut game, A, source, n, &mut choices); }
                assert!(choices.branch.is_none(), "both legal branches are offered exactly once");
                assert_eq!(game.player(A).unwrap().mana_pool.colorless, if chosen == 0 { 2 } else { 0 });
                assert_eq!(game.player(A).unwrap().mana_pool.green, 1);
                assert!(game.is_tapped(source));
                assert!(game.stack.is_empty());
            }
        }
    }
}
