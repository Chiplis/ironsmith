//! Full frozen bodies and native counterparts. Authored, intentionally unrun
//! during the implementation-first campaign.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::color::{Color, ColorSet};
use ironsmith::decision::{DecisionMaker};
use ironsmith::decisions::context::{ManaPaymentContext, NumberContext, SelectObjectsContext};
use ironsmith::effect::{ChoiceCount, Effect, EffectOutcome, ExecutionFact, Value};
use ironsmith::effects::{CollectManaPaymentsEffect, CreateTokenEffect, DrawCardsEffect, EffectContext, EffectExecutor, ExecutionError, ForPlayersEffect, MillEffect, ChooseObjectsEffect, PutOntoBattlefieldEffect, SequenceEffect};
use ironsmith::mana::{ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const NAMES: [&str; 4] = ["Alliance of Arms", "Collective Voyage", "Minds Aglow", "Shared Trauma"];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/join_forces.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, artifact_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!artifact_loss.is_lossy(), "{name} artifact: {}", artifact_loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn native(name: &str) -> Effect {
    let player = PlayerFilter::IteratedPlayer;
    let body = match name {
        "Alliance of Arms" => vec![Effect::new(CreateTokenEffect::new(
            CardDefinitionBuilder::new(CardId::new(), "Soldier").token()
                .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Soldier])
                .color_indicator(ColorSet::from(Color::White))
                .power_toughness(PowerToughness::fixed(1, 1)).build(),
            Value::X, player,
        ))],
        "Minds Aglow" => vec![Effect::new(DrawCardsEffect::new(Value::X, player))],
        "Shared Trauma" => vec![Effect::new(MillEffect::new(Value::X, player))],
        "Collective Voyage" => {
            let tag = "native_collective_voyage_found";
            let choose = ChooseObjectsEffect::new(
                ObjectFilter::default().with_type(CardType::Land).with_supertype(Supertype::Basic)
                    .owned_by(player.clone()),
                ChoiceCount::up_to_dynamic_x(), player.clone(), tag,
            ).in_zone(Zone::Library).with_count_value(Value::X).as_optional_search();
            vec![Effect::new(choose), Effect::for_each_tagged(tag, vec![Effect::new(
                PutOntoBattlefieldEffect::new(ChooseSpec::Iterated, true, player.clone()),
            )]), Effect::shuffle_library_player(player)]
        }
        _ => unreachable!(),
    };
    Effect::new(CollectManaPaymentsEffect::new(vec![Effect::new(ForPlayersEffect::new(PlayerFilter::Any, body))]))
}
fn programs(name: &str) -> Vec<Effect> {
    let mut result = definitions(name).iter().map(|definition| Effect::new(SequenceEffect::new(
        definition.spell_effect.as_ref().unwrap().flattened_default_effects().to_vec(),
    ))).collect::<Vec<_>>();
    let native = native(name);
    let encoded = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(native.clone()).unwrap();
    let restored = serde_json::from_str(&serde_json::to_string(&encoded).unwrap()).unwrap();
    assert_eq!(encoded, restored);
    result.push(native);
    result.push(ironsmith_runtime_catalog::artifact_materializer::materialize_effect(restored).unwrap());
    result
}
fn game() -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = B;
    game.turn_store.turn_order = vec![A, C, B];
    let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Payment source").card_types(vec![CardType::Sorcery]).build(), A, Zone::Stack);
    for player in [A, B, C] {
        game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Colorless, 5);
        for index in 0..7 {
            let card = CardBuilder::new(CardId::new(), format!("Basic {index}"))
                .card_types(vec![CardType::Land]).supertypes(vec![Supertype::Basic]).build();
            game.create_object_from_card(&card, player, Zone::Library);
        }
        game.create_object_from_card(&CardBuilder::new(CardId::new(), "Nonbasic control").card_types(vec![CardType::Land]).build(), player, Zone::Library);
    }
    (game, source)
}
#[derive(Default)]
struct Choices {
    amounts: [u32; 3], numbers: Vec<PlayerId>, payments: Vec<PlayerId>, searches: Vec<PlayerId>,
    cancel: Option<PlayerId>, pause_number: Option<PlayerId>, pause_payment: Option<PlayerId>, pause_search: Option<PlayerId>,
    fail_to_find: Option<PlayerId>, pending: bool,
    selected: Vec<ironsmith::ids::StableId>, verify_search_batch: bool,
    pause_addition: bool, addition_calls: usize,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        assert!(ctx.is_x_value);
        self.numbers.push(ctx.player);
        if self.pause_number == Some(ctx.player) { self.pending = true; return 0; }
        self.amounts[ctx.player.0 as usize].min(ctx.max)
    }
    fn decide_mana_payment(&mut self, _: &GameState, ctx: &ManaPaymentContext) -> ManaPaymentResponse {
        self.payments.push(ctx.player);
        assert_eq!(ctx.request.reason, ironsmith::costs::PaymentReason::Effect);
        if self.pause_payment == Some(ctx.player) { self.pending = true; return ManaPaymentResponse::Cancel; }
        if self.cancel == Some(ctx.player) { return ManaPaymentResponse::Cancel; }
        ManaPaymentResponse::Confirm { plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash }
    }
    fn decide_boolean(&mut self, game: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
        if self.verify_search_batch {
            assert_eq!(self.selected.len(), 9, "every search completes before any replacement addition");
            assert!(self.selected.iter().all(|stable| game.find_object_by_stable_id(*stable)
                .is_some_and(|id| game.object(id).unwrap().zone == Zone::Battlefield)),
                "all players' original land arrivals precede replacement additions");
        }
        self.addition_calls += 1;
        if self.pause_addition { self.pending = true; return false; }
        true
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        assert!(game.battlefield.iter().filter_map(|id| game.object(*id))
            .all(|object| !object.has_supertype(Supertype::Basic)),
            "each player's search is selected before any searched land enters");
        self.searches.push(ctx.player);
        if self.pause_search == Some(ctx.player) { self.pending = true; return vec![]; }
        if self.fail_to_find == Some(ctx.player) { return vec![]; }
        let chosen: Vec<_> = ctx.candidates.iter().filter(|candidate| candidate.legal)
            .filter(|candidate| game.object(candidate.id).is_some_and(|object| object.has_supertype(Supertype::Basic)))
            .take(ctx.max.unwrap_or(ctx.candidates.len())).map(|candidate| candidate.id).collect();
        self.selected.extend(chosen.iter().map(|id| game.object(*id).unwrap().stable_id));
        chosen
    }
}
fn execute(program: &Effect, game: &mut GameState, source: ObjectId, choices: &mut Choices) -> Result<EffectOutcome, ExecutionError> {
    let mut ctx = EffectContext::new(source, A, choices);
    ctx.x_value = Some(99);
    ctx.mana.payment_reason = Some(ironsmith::costs::PaymentReason::CastSpell);
    let result = program.0.execute(game, &mut ctx);
    assert_eq!(ctx.x_value, Some(99), "the total's local X must not escape");
    assert_eq!(ctx.mana.payment_reason, Some(ironsmith::costs::PaymentReason::CastSpell));
    result
}
fn assert_body(name: &str, game: &GameState, amount: usize) {
    for player in [A, B, C] {
        let state = game.player(player).unwrap();
        match name {
            "Minds Aglow" => assert_eq!(state.hand.len(), amount),
            "Shared Trauma" => assert_eq!(state.graveyard.len(), amount),
            "Alliance of Arms" | "Collective Voyage" => {
                let objects = game.battlefield.iter().filter_map(|id| game.object(*id))
                    .filter(|object| game.controller_of(object) == player).collect::<Vec<_>>();
                assert_eq!(objects.len(), amount);
                if name == "Collective Voyage" {
                    assert!(objects.iter().all(|object| game.is_tapped(object.id)));
                    assert!(objects.iter().all(|object| object.has_supertype(Supertype::Basic)));
                } else {
                    assert!(objects.iter().all(|object| object.kind == ironsmith::object::ObjectKind::Token && object.has_subtype(Subtype::Soldier)));
                    for object in &objects {
                        assert_eq!(game.current_colors(object.id), Some(ColorSet::from(Color::White)));
                        assert_eq!(game.current_power(object.id), Some(1));
                        assert_eq!(game.current_toughness(object.id), Some(1));
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}
#[test]
fn four_complete_bodies_use_actual_total_controller_first_and_apnap_body_order() {
    for name in NAMES { for program in programs(name) {
        let (mut game, source) = game();
        let mut choices = Choices { amounts: [1, 2, 0], ..Default::default() };
        let receipt = execute(&program, &mut game, source, &mut choices).unwrap();
        assert_eq!(choices.numbers, vec![A, C, B]);
        assert_eq!(choices.payments, vec![A, C, B]);
        assert_eq!(receipt.execution_facts().iter().filter_map(|fact| match fact { ExecutionFact::ManaPaid { x_value } => Some(*x_value), _ => None }).collect::<Vec<_>>(), vec![1, 0, 2]);
        assert_body(name, &game, 3);
        for (player, remaining) in [(A, 4), (B, 3), (C, 5)] { assert_eq!(game.player(player).unwrap().mana_pool.total(), remaining); }
        if name == "Collective Voyage" {
            assert_eq!(choices.searches, vec![B, A, C]);
            assert_eq!(receipt.events.iter().filter(|event| event.downcast::<ironsmith::events::ShuffleLibraryEvent>().is_some()).count(), 3);
        }
    }}
}
#[test]
fn declined_payment_is_not_added_and_zero_total_still_executes_mandatory_search_shuffle() {
    for name in NAMES { for program in programs(name) {
        for all_zero in [false, true] {
            let (mut game, source) = game();
            let mut choices = Choices { amounts: if all_zero { [0, 0, 0] } else { [4, 2, 0] }, cancel: Some(A), ..Default::default() };
            let receipt = execute(&program, &mut game, source, &mut choices).unwrap();
            assert_body(name, &game, if all_zero { 0 } else { 2 });
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 5);
            if name == "Collective Voyage" {
                assert_eq!(receipt.events.iter().filter(|event| event.downcast::<ironsmith::events::ShuffleLibraryEvent>().is_some()).count(), 3);
            }
        }
    }}
}
#[test]
fn a_pending_later_contributor_rolls_back_every_prior_payment_and_retries_once() {
    for name in NAMES { for program in programs(name) { for payment_pause in [false, true] {
        let (mut game, source) = game();
        let next_id = game.next_object_id_counter();
        let libraries = [A, B, C].map(|player| game.player(player).unwrap().library.clone());
        let mut choices = Choices { amounts: [1, 2, 0], pause_number: (!payment_pause).then_some(B), pause_payment: payment_pause.then_some(B), ..Default::default() };
        let receipt = execute(&program, &mut game, source, &mut choices).unwrap();
        assert!(choices.pending);
        assert!(receipt.events.is_empty());
        assert!(receipt.execution_facts().is_empty());
        assert_eq!(game.next_object_id_counter(), next_id);
        for (index, player) in [A, B, C].iter().copied().enumerate() {
            assert_eq!(game.player(player).unwrap().mana_pool.total(), 5);
            assert_eq!(game.player(player).unwrap().library, libraries[index]);
        }
        assert_body(name, &game, 0);
        choices.pending = false; choices.pause_number = None; choices.pause_payment = None;
        execute(&program, &mut game, source, &mut choices).unwrap();
        assert_body(name, &game, 3);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 4);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 3);
    }}}
}
#[test]
fn search_pending_and_token_resource_failure_roll_back_payment_body_and_object_ids() {
    for name in ["Collective Voyage", "Alliance of Arms"] { for program in programs(name) {
        let (mut game, source) = game();
        let next_id = game.next_object_id_counter();
        let mut choices = Choices { amounts: [1, 2, 0], pause_search: (name == "Collective Voyage").then_some(C), ..Default::default() };
        if name == "Alliance of Arms" { game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 4, ..Default::default() }); }
        let result = execute(&program, &mut game, source, &mut choices);
        if name == "Alliance of Arms" { assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded { .. }))); } else { assert!(result.unwrap().events.is_empty()); assert!(choices.pending); }
        assert_body(name, &game, 0);
        assert_eq!(game.next_object_id_counter(), next_id);
        for player in [A, B, C] { assert_eq!(game.player(player).unwrap().mana_pool.total(), 5); assert_eq!(game.player(player).unwrap().library.len(), 8); }
        game.set_token_creation_limits(Default::default()); choices.pending = false; choices.pause_search = None;
        execute(&program, &mut game, source, &mut choices).unwrap();
        assert_body(name, &game, 3);
    }}
}
#[test]
fn voyage_may_find_fewer_basics_without_skipping_any_shuffle() {
    for program in programs("Collective Voyage") {
        let (mut game, source) = game();
        let mut choices = Choices { amounts: [1, 2, 0], fail_to_find: Some(B), ..Default::default() };
        let receipt = execute(&program, &mut game, source, &mut choices).unwrap();
        assert_eq!(game.battlefield.len(), 6);
        assert_eq!(game.player(B).unwrap().library.len(), 8);
        assert_eq!(receipt.events.iter().filter(|event| event.downcast::<ironsmith::events::ShuffleLibraryEvent>().is_some()).count(), 3);
    }
}

#[test]
fn payment_activates_mana_sources_and_excludes_players_already_out_of_the_game() {
    for program in programs("Minds Aglow") {
        let (mut game, source) = game();
        for player in [A, B, C] { game.player_mut(player).unwrap().mana_pool.empty(); }
        game.player_mut(C).unwrap().has_left_game = true;
        let land = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Two-mana land")
            .card_types(vec![CardType::Land]).build(), A, Zone::Battlefield);
        game.object_mut(land).unwrap().abilities_mut().push(ironsmith::ability::Ability::mana(
            ironsmith::TotalCost::from_cost(ironsmith::costs::Cost::tap()),
            vec![ManaSymbol::Colorless, ManaSymbol::Colorless],
        ));
        let mut choices = Choices { amounts: [2, 0, 5], ..Default::default() };
        execute(&program, &mut game, source, &mut choices).unwrap();
        assert_eq!(choices.numbers, vec![A, B]);
        assert!(game.is_tapped(land));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(B).unwrap().hand.len(), 2);
        assert_eq!(game.player(C).unwrap().hand.len(), 0);
    }
}
#[test]
fn cast_only_mana_is_not_a_resolution_contribution_even_with_an_inherited_cast_reason() {
    for program in programs("Minds Aglow") {
        let (mut game, source) = game();
        game.player_mut(A).unwrap().mana_pool.empty();
        for _ in 0..3 {
            game.player_mut(A).unwrap().add_restricted_mana(ironsmith::ability::RestrictedManaUnit {
                symbol: ManaSymbol::Colorless, source, source_chosen_creature_type: None,
                source_controller: Some(A),
                restrictions: vec![ironsmith::ability::ManaUsageRestriction::CastSpell {
                    card_types: vec![CardType::Sorcery], subtype_requirement: None,
                    restrict_to_matching_spell: true, grant_uncounterable: false,
                    enters_with_counters: vec![], granted_abilities: vec![],
                }],
            });
        }
        let mut choices = Choices { amounts: [3, 2, 0], ..Default::default() };
        execute(&program, &mut game, source, &mut choices).unwrap();
        assert_body("Minds Aglow", &game, 2);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
        assert_eq!(game.player(A).unwrap().restricted_mana.len(), 3);
    }
}
#[test]
fn unrepresentable_contribution_bound_is_a_typed_error_and_restores_earlier_payments() {
    for program in programs("Shared Trauma") {
        let (mut game, source) = game();
        let player = game.player_mut(B).unwrap();
        player.mana_pool.white = u32::MAX;
        player.mana_pool.blue = u32::MAX;
        let mut choices = Choices { amounts: [1, 0, 0], ..Default::default() };
        let error = execute(&program, &mut game, source, &mut choices).unwrap_err();
        assert!(matches!(error, ExecutionError::UnresolvableValue(_)));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 5);
        assert_eq!(game.player(B).unwrap().mana_pool.total_wide(), 2 * u64::from(u32::MAX) + 5);
        assert_body("Shared Trauma", &game, 0);
    }
}

#[test]
fn voyage_replacement_additions_see_all_original_arrivals_and_pending_rolls_back_every_player() {
    for program in programs("Collective Voyage") { for pause in [false, true] {
        let (mut game, source) = game();
        let shield = game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(source, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::default().with_type(CardType::Land), Some(Zone::Library), Some(Zone::Battlefield)),
                ironsmith::replacement::ReplacementAction::Additionally(vec![Effect::may(vec![Effect::gain_life(1)])]),
            ));
        let initial_id = game.next_object_id_counter();
        let mut choices = Choices { amounts: [1, 2, 0], verify_search_batch: true, pause_addition: pause, ..Default::default() };
        let receipt = execute(&program, &mut game, source, &mut choices).unwrap();
        if pause {
            assert!(choices.pending); assert!(receipt.events.is_empty()); assert!(receipt.execution_facts().is_empty());
            assert_body("Collective Voyage", &game, 0);
            assert_eq!(game.next_object_id_counter(), initial_id);
            for player in [A, B, C] { assert_eq!(game.player(player).unwrap().mana_pool.total(), 5); }
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            choices.pending = false; choices.pause_addition = false; choices.selected.clear(); choices.addition_calls = 0;
            execute(&program, &mut game, source, &mut choices).unwrap();
        }
        assert_body("Collective Voyage", &game, 3);
        assert_eq!(choices.addition_calls, 9, "one replacement completion per original land");
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 4);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 3);
    }}
}
