//! Complete frozen Niko contracts. Source-authored only; execution is deferred.
use ironsmith::ability::{Ability, AbilityKind, ManaPaymentPredicate, ManaPaymentPurpose, ManaUsageRestriction};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{ManaPaymentContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm, drain_pending_trigger_events,
    put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::object::CounterType;
use ironsmith::special_actions::SpecialAction;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
#[derive(Clone, Copy, Debug)]
enum Route { Direct, Artifact }
const ROUTES: [Route; 2] = [Route::Direct, Route::Artifact];

fn definition(name: &str, route: Route) -> CardDefinition {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/foretell_state_and_grants.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let builder = || ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name);
    match route {
        Route::Direct => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_runtime_definition(builder(), text, false));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            result.unwrap()
        }
        Route::Artifact => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_artifact(builder(), text, false));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            let (artifact, _) = result.unwrap();
            artifact.validate().unwrap();
            let decoded = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
            ironsmith::artifact_materializer::materialize_artifact(&decoded).unwrap()
        }
    }
}

#[derive(Default)]
struct Choices {
    target: Option<Target>,
    target_prompts: Vec<TargetsContext>,
    cancel_mana: bool,
    mana_prompts: usize,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { false }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.target_prompts.push(ctx.clone());
        if let Some(target) = self.target {
            assert!(ctx.requirements.iter().all(|r| r.legal_targets.contains(&target)), "{ctx:?}");
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, ctx) }
    }
    fn decide_mana_payment(&mut self, _: &GameState, ctx: &ManaPaymentContext) -> ManaPaymentResponse {
        self.mana_prompts += 1;
        if self.cancel_mana { ManaPaymentResponse::Cancel }
        else { ManaPaymentResponse::Confirm { plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash } }
    }
}
fn main(game: &mut GameState, active: PlayerId) {
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(20261006);
    game.turn.turn_number = 7;
    main(&mut game, ALICE);
    for player in [ALICE, BOB] {
        for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
            ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 20);
        }
    }
    game
}
fn legal_cast(game: &GameState, card: ObjectId, method: &CastingMethod) -> Option<LegalAction> {
    compute_legal_actions(game, ALICE).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { spell_id, casting_method, .. }
            if *spell_id == card && casting_method == method))
}
fn announce(game: &mut GameState, queue: &mut TriggerQueue, action: LegalAction, dm: &mut Choices) {
    let old_stack = game.stack.len();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(game, queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none()
            && game.stack.len() > old_stack { return; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("native announcement did not complete");
}
fn cast(game: &mut GameState, queue: &mut TriggerQueue, card: ObjectId,
    method: CastingMethod, cost: u32, dm: &mut Choices) -> ObjectId {
    let stable = game.object(card).unwrap().stable_id;
    let before = game.player(ALICE).unwrap().mana_pool.total();
    let action = legal_cast(game, card, &method).expect("native legal cast");
    announce(game, queue, action, dm);
    assert_eq!(before - game.player(ALICE).unwrap().mana_pool.total(), cost);
    let spell = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack);
    spell
}
fn finish(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut Choices) {
    for _ in 0..30 {
        drain_pending_trigger_events(game, queue);
        put_triggers_on_stack_with_dm(game, queue, dm).unwrap();
        advance_priority_with_dm(game, queue, dm).unwrap();
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("native stack did not finish");
}
fn enter_niko(game: &mut GameState, queue: &mut TriggerQueue, route: Route, dm: &mut Choices) -> ObjectId {
    let card = game.create_object_from_definition(&definition("Niko Defies Destiny", route), ALICE, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    cast(game, queue, card, CastingMethod::Normal, 3, dm);
    finish(game, queue, dm);
    let saga = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(saga).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.counter_count(saga, CounterType::Lore), 1);
    saga
}
fn next_precombat_main(game: &mut GameState, queue: &mut TriggerQueue, active: PlayerId) {
    game.empty_mana_pools().unwrap();
    game.next_turn();
    assert_eq!(game.turn.active_player, active);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMain);
    assert!(matches!(runner.advance(game, queue).unwrap(), TurnAction::RunPriority));
}
fn chapter_two(game: &mut GameState, queue: &mut TriggerQueue, saga: ObjectId, dm: &mut Choices) {
    next_precombat_main(game, queue, BOB);
    finish(game, queue, dm);
    assert_eq!(game.counter_count(saga, CounterType::Lore), 1, "opponent's main adds no lore");
    next_precombat_main(game, queue, ALICE);
    assert_eq!(game.counter_count(saga, CounterType::Lore), 2);
    finish(game, queue, dm);
    let player = game.player(ALICE).unwrap();
    assert_eq!((player.mana_pool.white, player.mana_pool.blue, player.mana_pool.total()), (1, 1, 2));
    assert_eq!(player.restricted_mana.len(), 2);
    assert!(player.restricted_mana.iter().all(|unit| unit.source == saga));
}
fn foretell(game: &mut GameState, card: ObjectId, player: PlayerId) -> ObjectId {
    let stable = game.object(card).unwrap().stable_id;
    let before = game.player(player).unwrap().mana_pool.total();
    let action = SpecialAction::Foretell { card_id: card };
    assert!(compute_legal_actions(game, player).unwrap().contains(&LegalAction::SpecialAction(action.clone())));
    ironsmith::special_actions::perform(action, game, player, &mut Choices::default()).unwrap();
    assert_eq!(before - game.player(player).unwrap().mana_pool.total(), 2);
    let exile = game.find_object_by_stable_id(stable).unwrap();
    assert_ne!(exile, card);
    assert_eq!(game.object(exile).unwrap().zone, Zone::Exile);
    assert!(game.is_foretold(exile) && game.is_face_down(exile));
    exile
}
fn fixture_card(game: &mut GameState, route: Route, owner: PlayerId, zone: Zone) -> ObjectId {
    game.create_object_from_definition(&definition("Cosmos Charger", route), owner, zone)
}
fn ordinary_card(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ordinary control")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1), ManaSymbol::Blue]))
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
    game.create_object_from_definition(&def, owner, zone)
}
fn assert_foretell_union(restriction: &ManaUsageRestriction) {
    let ManaUsageRestriction::PaymentTransaction { restriction: Some(ManaPaymentPredicate::AnyOf(arms)), on_spend } = restriction
        else { panic!("expected the typed chapter-II union: {restriction:?}"); };
    assert!(on_spend.is_empty());
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0], ManaPaymentPredicate::Purpose(ManaPaymentPurpose::Foretell));
    let ManaPaymentPredicate::All(cast) = &arms[1] else { panic!("{arms:?}"); };
    assert_eq!(cast.len(), 2);
    assert_eq!(cast[0], ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell));
    let ManaPaymentPredicate::SourceMatches(filter) = &cast[1] else { panic!("{cast:?}"); };
    assert_eq!(filter.alternative_cast, Some(ironsmith::filter::AlternativeCastKind::Foretell));
    assert!(!filter.foretold);
    assert_eq!(filter.zone, None);
}
#[test]
fn full_niko_lowers_exactly_three_chapters_with_distinct_designation_and_capability() {
    use ironsmith::effect::Value;
    for route in ROUTES {
        let def = definition("Niko Defies Destiny", route);
        assert!(def.card.subtypes.contains(&ironsmith::Subtype::Saga));
        let chapters = def.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(trigger) => Some(trigger),
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(chapters.len(), 3);
        for (index, chapter) in chapters.iter().enumerate() {
            assert_eq!(chapter.trigger.saga_chapters(), Some(&[index as u32 + 1][..]));
            assert!(chapter.intervening_if.is_none());
        }
        let one = chapters[0].effects.all_effects();
        assert_eq!(one.len(), 1);
        let gain = one[0].downcast_ref::<ironsmith::effects::GainLifeEffect>().unwrap();
        assert_eq!(gain.player.base(), &ChooseSpec::Player(PlayerFilter::You));
        let filter = match gain.amount.unhinted() {
            Value::CountScaled(filter, 2) => filter,
            Value::Scaled(inner, 2) => match inner.unhinted() { Value::Count(filter) => filter, other => panic!("{other:?}") },
            other => panic!("chapter I must gain twice the qualifying count: {other:?}"),
        };
        assert!(filter.foretold);
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Exile));
        assert_eq!(filter.alternative_cast, None);
        let two = chapters[1].effects.all_effects();
        assert_eq!(two.len(), 1);
        let restricted = two[0].downcast_ref::<ironsmith::effects::ManaRestrictedEffect>().unwrap();
        assert_eq!(restricted.restrictions.len(), 1);
        assert_foretell_union(&restricted.restrictions[0]);
        assert_eq!(restricted.effects.len(), 1);
        let mana = restricted.effects[0].downcast_ref::<ironsmith::effects::AddManaEffect>().unwrap();
        assert_eq!(mana.mana, vec![ManaSymbol::White, ManaSymbol::Blue]);
        assert_eq!(mana.player, PlayerFilter::You);
        assert_eq!(chapters[2].choices.len(), 1);
        assert!(chapters[2].choices[0].is_target());
        let ChooseSpec::Object(filter) = chapters[2].choices[0].base() else { panic!("{:?}", chapters[2].choices); };
        assert_eq!(filter.alternative_cast, Some(ironsmith::filter::AlternativeCastKind::Foretell));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert!(!filter.foretold, "chapter III needs a keyword, not a retired exile designation");
        let three = chapters[2].effects.all_effects();
        assert_eq!(three.len(), 1);
        assert!(three[0].downcast_ref::<ironsmith::effects::ReturnFromGraveyardToHandEffect>().is_some());
    }
}

#[test]
fn chapter_one_counts_zero_or_multiple_owned_designated_exile_cards_only() {
    for route in ROUTES { for count in [0, 3] {
        let mut game = setup();
        for _ in 0..count {
            let card = fixture_card(&mut game, route, ALICE, Zone::Hand);
            foretell(&mut game, card, ALICE);
        }
        fixture_card(&mut game, route, ALICE, Zone::Exile); // Capability without designation.
        ordinary_card(&mut game, ALICE, Zone::Exile);
        for zone in [Zone::Hand, Zone::Graveyard] {
            let card = fixture_card(&mut game, route, ALICE, Zone::Hand);
            let exile = foretell(&mut game, card, ALICE);
            let moved = game.move_object_by_effect(exile, zone).unwrap();
            assert!(!game.is_foretold(moved));
        }
        main(&mut game, BOB);
        let card = fixture_card(&mut game, route, BOB, Zone::Hand);
        foretell(&mut game, card, BOB);
        main(&mut game, ALICE);
        let mut queue = TriggerQueue::new();
        enter_niko(&mut game, &mut queue, route, &mut Choices::default());
        assert_eq!(game.player(ALICE).unwrap().life, 20 + 2 * count);
        assert_eq!(game.player(BOB).unwrap().life, 20);
    }}
}

#[test]
fn chapter_two_pays_actual_foretell_and_cancellation_restores_units_before_retry() {
    for route in ROUTES { for source_leaves in [false, true] {
        let mut game = setup();
        let mut queue = TriggerQueue::new();
        let mut dm = Choices::default();
        let saga = enter_niko(&mut game, &mut queue, route, &mut dm);
        chapter_two(&mut game, &mut queue, saga, &mut dm);
        if source_leaves { game.move_object_by_effect(saga, Zone::Graveyard).unwrap(); }
        let card = fixture_card(&mut game, route, ALICE, Zone::Hand);
        let pool = game.player(ALICE).unwrap().mana_pool.clone();
        let units = game.player(ALICE).unwrap().restricted_mana.clone();
        let action = SpecialAction::Foretell { card_id: card };
        let mut cancel = Choices { cancel_mana: true, ..Default::default() };
        assert_eq!(ironsmith::special_actions::perform(action, &mut game, ALICE, &mut cancel),
            Err(ironsmith::special_actions::ActionError::Cancelled));
        assert!(cancel.mana_prompts > 0);
        assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
        assert!(game.exile.is_empty() && !game.has_foretold_this_turn(ALICE));
        assert_eq!(game.player(ALICE).unwrap().mana_pool, pool);
        assert_eq!(game.player(ALICE).unwrap().restricted_mana, units);
        let exile = foretell(&mut game, card, ALICE);
        assert!(game.has_foretold_this_turn(ALICE));
        assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert!(game.player(ALICE).unwrap().restricted_mana.is_empty());
        // A funded same-turn alternative cast is still forbidden.
        game.player_mut(ALICE).unwrap().mana_pool.add(ManaSymbol::Blue, 3);
        assert!(legal_cast(&game, exile, &CastingMethod::Alternative(0)).is_none());
    }}
}

#[test]
fn chapter_two_pays_full_card_ordinary_or_later_foretell_cast_after_producer_departure() {
    for route in ROUTES { for alternative in [false, true] {
        let mut game = setup();
        let card = fixture_card(&mut game, route, ALICE, Zone::Hand);
        let stable = game.object(card).unwrap().stable_id;
        let card = if alternative { foretell(&mut game, card, ALICE) } else { card };
        let mut queue = TriggerQueue::new();
        let mut dm = Choices::default();
        let saga = enter_niko(&mut game, &mut queue, route, &mut dm);
        chapter_two(&mut game, &mut queue, saga, &mut dm);
        game.move_object_by_effect(saga, Zone::Graveyard).unwrap();
        game.player_mut(ALICE).unwrap().mana_pool.add(ManaSymbol::Colorless, if alternative { 1 } else { 2 });
        let spell = cast(&mut game, &mut queue, card,
            if alternative { CastingMethod::Alternative(0) } else { CastingMethod::Normal },
            if alternative { 3 } else { 4 }, &mut dm);
        assert!(!game.is_face_down(spell));
        assert_eq!(game.object(spell).unwrap().optional_costs_paid.cast_was_foretold, Some(alternative));
        assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert!(game.player(ALICE).unwrap().restricted_mana.is_empty());
        finish(&mut game, &mut queue, &mut dm);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Battlefield);
    }}
}

#[test]
fn chapter_two_rejects_unrelated_spells_and_even_activations_of_a_foretell_card() {
    for route in ROUTES {
        let mut game = setup();
        let mut queue = TriggerQueue::new();
        let mut dm = Choices::default();
        let saga = enter_niko(&mut game, &mut queue, route, &mut dm);
        chapter_two(&mut game, &mut queue, saga, &mut dm);
        let unrelated = ordinary_card(&mut game, ALICE, Zone::Hand);
        let source = fixture_card(&mut game, route, ALICE, Zone::Battlefield);
        game.object_mut(source).unwrap().abilities_mut().push(Ability::activated(
            ironsmith::cost::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1), ManaSymbol::Blue])),
            vec![ironsmith::effect::Effect::gain_life(1)]));
        let actions = compute_legal_actions(&game, ALICE).unwrap();
        assert!(legal_cast(&game, unrelated, &CastingMethod::Normal).is_none());
        assert!(!actions.iter().any(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source)));
        let units = game.player(ALICE).unwrap().restricted_mana.clone();
        game.player_mut(ALICE).unwrap().mana_pool.add(ManaSymbol::Blue, 2);
        let action = compute_legal_actions(&game, ALICE).unwrap().into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
        announce(&mut game, &mut queue, action, &mut dm);
        finish(&mut game, &mut queue, &mut dm);
        assert_eq!(game.player(ALICE).unwrap().life, 21);
        assert_eq!(game.player(ALICE).unwrap().restricted_mana, units);
        game.player_mut(ALICE).unwrap().mana_pool.add(ManaSymbol::Blue, 2);
        cast(&mut game, &mut queue, unrelated, CastingMethod::Normal, 2, &mut dm);
        finish(&mut game, &mut queue, &mut dm);
        assert_eq!(game.player(ALICE).unwrap().restricted_mana, units);
        assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 2);
    }
}

fn morph_foreteller() -> CardDefinition {
    let price = ManaCost::from_symbols(vec![ManaSymbol::Generic(1), ManaSymbol::Blue]);
    CardDefinitionBuilder::new(CardId::new(), "Face-down Foretell capability control")
        .card_types(vec![CardType::Creature]).mana_cost(price.clone())
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 3))
        .foretell(price.clone())
        .with_ability(Ability::static_ability(ironsmith::static_abilities::StaticAbility::morph(
            ironsmith::cost::TotalCost::mana(price)))).build()
}
#[test]
fn concealed_physical_foretell_keyword_does_not_fund_morph_but_face_up_routes_remain_legal() {
    for route in ROUTES { for method in [CastingMethod::FaceDown, CastingMethod::Normal, CastingMethod::Alternative(0)] {
        let mut game = setup();
        let card = game.create_object_from_definition(&morph_foreteller(), ALICE, Zone::Hand);
        let card = if matches!(method, CastingMethod::Alternative(_)) { foretell(&mut game, card, ALICE) } else { card };
        let mut queue = TriggerQueue::new();
        let mut dm = Choices::default();
        let saga = enter_niko(&mut game, &mut queue, route, &mut dm);
        chapter_two(&mut game, &mut queue, saga, &mut dm);
        game.move_object_by_effect(saga, Zone::Graveyard).unwrap();
        if matches!(method, CastingMethod::FaceDown) {
            game.player_mut(ALICE).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
            assert!(legal_cast(&game, card, &method).is_none(), "WU cannot fund a face-down spell with no Foretell ability");
            let units = game.player(ALICE).unwrap().restricted_mana.clone();
            game.player_mut(ALICE).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
            let spell = cast(&mut game, &mut queue, card, method.clone(), 3, &mut dm);
            assert!(game.is_face_down(spell));
            assert_eq!(game.player(ALICE).unwrap().restricted_mana, units);
            assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 2);
        } else {
            let spell = cast(&mut game, &mut queue, card, method.clone(), 2, &mut dm);
            assert!(!game.is_face_down(spell));
            assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0);
            assert!(game.player(ALICE).unwrap().restricted_mana.is_empty());
        }
        finish(&mut game, &mut queue, &mut dm);
    }}
}

#[test]
fn chapter_three_targets_only_own_foretell_graveyard_and_sacrifices_after_resolution_or_invalidation() {
    for route in ROUTES { for invalidate in [false, true] {
        let mut game = setup();
        let first = fixture_card(&mut game, route, ALICE, Zone::Graveyard);
        let first_stable = game.object(first).unwrap().stable_id;
        let second = fixture_card(&mut game, route, ALICE, Zone::Graveyard);
        let ordinary = ordinary_card(&mut game, ALICE, Zone::Graveyard);
        let foreign = fixture_card(&mut game, route, BOB, Zone::Graveyard);
        let hand = fixture_card(&mut game, route, ALICE, Zone::Hand);
        let exile = fixture_card(&mut game, route, ALICE, Zone::Exile);
        let mut queue = TriggerQueue::new();
        let mut dm = Choices::default();
        let saga = enter_niko(&mut game, &mut queue, route, &mut dm);
        let saga_stable = game.object(saga).unwrap().stable_id;
        chapter_two(&mut game, &mut queue, saga, &mut dm);
        next_precombat_main(&mut game, &mut queue, BOB);
        finish(&mut game, &mut queue, &mut dm);
        next_precombat_main(&mut game, &mut queue, ALICE);
        assert_eq!(game.counter_count(saga, CounterType::Lore), 3);
        dm.target = Some(Target::Object(first));
        drain_pending_trigger_events(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        let prompt = dm.target_prompts.last().expect("chapter III must target");
        assert_eq!(prompt.requirements.len(), 1);
        let legal = &prompt.requirements[0].legal_targets;
        assert_eq!(legal.len(), 2);
        assert!(legal.contains(&Target::Object(first)) && legal.contains(&Target::Object(second)));
        for id in [ordinary, foreign, hand, exile] { assert!(!legal.contains(&Target::Object(id))); }
        advance_priority_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.object(saga).unwrap().zone, Zone::Battlefield, "final ability is still pending");
        if invalidate {
            let moved = game.move_object_by_effect(first, Zone::Exile).unwrap();
            let replacement = game.move_object_by_effect(moved, Zone::Graveyard).unwrap();
            assert_ne!(replacement, first, "same stable card is a new target incarnation");
        }
        finish(&mut game, &mut queue, &mut dm);
        let returned = game.find_object_by_stable_id(first_stable).unwrap();
        assert_eq!(game.object(returned).unwrap().zone, if invalidate { Zone::Graveyard } else { Zone::Hand });
        assert_eq!(game.object(second).unwrap().zone, Zone::Graveyard, "no fallback retargeting");
        assert_eq!(game.object(ordinary).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
        let sacrificed = game.find_object_by_stable_id(saga_stable).unwrap();
        assert_ne!(sacrificed, saga);
        assert_eq!(game.object(sacrificed).unwrap().zone, Zone::Graveyard);
    }}
}
