//! Frozen whole bodies; source-authored and intentionally unrun before the
//! campaign's majority gate. Rules: CR 118.11, 121.2b/121.3, 401.4, 702.24a.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, OrderContext, SelectObjectsContext};
use ironsmith::effects::{CumulativeUpkeepEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith::target::ObjectFilter;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{CounterType, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/cumulative_action_costs.json.fixture")).unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, row["text"].as_str().unwrap(), false));
    let direct = direct.unwrap();
    assert!(!direct_loss.is_lossy(), "{name} direct: {}", direct_loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, row["text"].as_str().unwrap(), false));
    let (artifact, _) = compiled.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(artifact, decoded);
    let restored = materialize_artifact(&decoded).unwrap();
    for definition in [&direct, &restored] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, restored]
}
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str) -> ObjectId {
    let card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), name)
        .card_types(vec![ironsmith::types::CardType::Land]).build();
    game.create_object_from_card(&card, owner, zone)
}
fn payment(definition: &CardDefinition) -> CumulativeUpkeepEffect {
    fn find(effect: &Effect) -> Option<CumulativeUpkeepEffect> {
        if let Some(payment) = effect.downcast_ref::<CumulativeUpkeepEffect>() { return Some(payment.clone()); }
        let mut found = None;
        effect.visit_child_effects(&mut |child| { if found.is_none() { found = find(child); } });
        found
    }
    definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) => triggered.effects.all_effects().into_iter().find_map(find),
        _ => None,
    }).expect("typed cumulative upkeep")
}
fn notices(outcome: &ironsmith::effect::EffectOutcome) -> usize {
    outcome.events.iter().filter_map(|event| event.downcast::<ironsmith::events::other::KeywordActionEvent>())
        .filter(|event| event.action == ironsmith::events::KeywordActionKind::CumulativeUpkeepPaid).count()
}
#[derive(Default)]
struct Choices {
    accept: bool,
    pause_boolean: Option<usize>,
    boolean_calls: usize,
    pause_selection: Option<usize>,
    selection_calls: usize,
    pending: bool,
    invalid_group: bool,
    expected_unmoved: Vec<ObjectId>,
    selected: Vec<Vec<ObjectId>>,
    order_players: Vec<PlayerId>,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.boolean_calls += 1;
        self.pending = self.pause_boolean == Some(self.boolean_calls);
        self.accept
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.selection_calls += 1;
        for id in &self.expected_unmoved { assert_eq!(game.object(*id).unwrap().zone, Zone::Graveyard, "all age choices precede originals"); }
        self.pending = self.pause_selection == Some(self.selection_calls);
        let legal: Vec<_> = ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
        let selected = if self.invalid_group { vec![legal[0]] } else { legal.into_iter().take(ctx.min).collect() };
        self.selected.push(selected.clone()); selected
    }
    fn decide_order(&mut self, _: &GameState, ctx: &OrderContext) -> Vec<ObjectId> {
        self.order_players.push(ctx.player);
        ctx.items.iter().rev().map(|(id, _)| *id).collect()
    }
}
fn resolve_event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) {
    game.queue_trigger_event(Default::default(), event);
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry_with(game, dm).unwrap();
}

#[test]
fn frozen_bodies_compile_and_round_trip_and_resolve_their_complete_triggers() {
    for name in ["Braid of Fire", "Jötun Grunt", "Psychic Vortex"] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for owner in [A, B] { for n in 0..4 { card(&mut game, owner, Zone::Graveyard, &format!("Grave {owner:?} {n}")); } }
            for n in 0..4 { card(&mut game, A, Zone::Library, &format!("Private {n}")); }
            let mut dm = Choices { accept: true, ..Default::default() };
            for age in 1..=2 {
                game.turn.phase = ironsmith::Phase::Beginning;
                game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
                resolve_event(&mut game, TriggerEvent::new(ironsmith::events::phase::BeginningOfUpkeepEvent::new(A), Default::default()), &mut dm);
                assert_eq!(game.counter_count(source, CounterType::Age), age);
                assert!(game.battlefield.contains(&source));
            }
            match name {
                "Braid of Fire" => assert_eq!(game.player(A).unwrap().mana_pool.red, 3),
                "Jötun Grunt" => assert_eq!(game.player(A).unwrap().graveyard.len() + game.player(B).unwrap().graveyard.len(), 2),
                _ => {
                    assert_eq!(definition.abilities.len(), 2);
                    assert_eq!(game.player(A).unwrap().hand.len(), 3);
                    let land = card(&mut game, A, Zone::Battlefield, "End step land");
                    game.turn.phase = ironsmith::Phase::Ending;
                    game.turn.step = Some(ironsmith::game_state::Step::End);
                    resolve_event(&mut game, TriggerEvent::new(ironsmith::events::phase::BeginningOfEndStepEvent::new(A), Default::default()), &mut dm);
                    assert!(!game.battlefield.contains(&land));
                    assert!(game.player(A).unwrap().hand.is_empty());
                    assert!(game.battlefield.contains(&source));
                }
            }
        }
    }
}

#[test]
fn each_whole_body_offers_zero_payment_and_decline_or_pending_never_emits_paid() {
    for name in ["Braid of Fire", "Jötun Grunt", "Psychic Vortex"] {
        for definition in definitions(name) { for (accept, pause) in [(true, false), (false, false), (true, true)] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut dm = Choices { accept, pause_boolean: pause.then_some(1), ..Default::default() };
            let outcome = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
            assert_eq!(dm.boolean_calls, 1);
            assert_eq!(notices(&outcome), usize::from(accept && !pause));
            assert_eq!(game.battlefield.contains(&source), accept || pause);
            assert!(game.player(A).unwrap().hand.is_empty());
            assert_eq!(game.player(A).unwrap().mana_pool.red, 0);
        } }
    }
}

#[test]
fn grunt_preselects_each_pair_before_originals_and_each_owner_orders_their_library() {
    for definition in definitions("Jötun Grunt") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, 2);
        let cards: Vec<_> = [A, A, B, B].into_iter().enumerate().map(|(n, owner)| card(&mut game, owner, Zone::Graveyard, &format!("Pair {n}"))).collect();
        let stable: Vec<_> = cards.iter().map(|id| game.object(*id).unwrap().stable_id).collect();
        let mut dm = Choices { accept: true, expected_unmoved: cards.clone(), ..Default::default() };
        let outcome = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        assert_eq!(notices(&outcome), 1);
        assert_eq!(dm.selected, vec![cards[..2].to_vec(), cards[2..].to_vec()]);
        assert_eq!(dm.order_players, vec![A, B]);
        for (owner, pair) in [(A, &stable[..2]), (B, &stable[2..])] {
            let actual: Vec<_> = game.player(owner).unwrap().library.iter().map(|id| game.object(*id).unwrap().stable_id).collect();
            assert_eq!(actual, pair.iter().rev().copied().collect::<Vec<_>>());
        }
    }
}

#[test]
fn grunt_insufficient_split_groups_and_pending_or_malformed_choices_do_not_partially_pay() {
    for definition in definitions("Jötun Grunt") { for mode in 0..3 {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, 2);
        let owners = if mode == 0 { [A, A, A, B] } else { [A, A, B, B] };
        let cards: Vec<_> = owners.into_iter().map(|owner| card(&mut game, owner, Zone::Graveyard, "Cost card")).collect();
        let mut dm = Choices { accept: true, pause_selection: (mode == 1).then_some(2), invalid_group: mode == 2, ..Default::default() };
        let result = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm));
        if mode == 2 { assert!(matches!(result, Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_)))); }
        else { assert_eq!(notices(&result.unwrap()), 0); }
        for id in cards { assert_eq!(game.object(id).unwrap().zone, Zone::Graveyard); }
        assert_eq!(game.battlefield.contains(&source), mode != 0);
        if mode != 0 { assert!(game.take_pending_trigger_events().is_empty()); }
    } }
}

fn replacement(name: &str, source: ObjectId, action: ReplacementAction) -> ReplacementEffect {
    match name {
        "Braid of Fire" => ReplacementEffect::with_matcher(source, A, ironsmith::events::mana::matchers::ManaProducedBySourceMatcher::new(ObjectFilter::default()), action),
        "Jötun Grunt" => ReplacementEffect::with_matcher(source, A, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::default(), Some(Zone::Graveyard), Some(Zone::Library)), action),
        _ => ReplacementEffect::with_matcher(source, A, ironsmith::events::cards::matchers::WouldDrawCardMatcher::you(), action),
    }
}
#[test]
fn replaced_or_prevented_actions_still_pay_the_whole_cost_with_zero_original_receipts() {
    for name in ["Braid of Fire", "Jötun Grunt", "Psychic Vortex"] { for definition in definitions(name) { for prevented in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, 2);
        for _ in 0..4 { card(&mut game, A, Zone::Graveyard, "Available cost"); card(&mut game, A, Zone::Library, "Available draw"); }
        game.effect_store.replacement_effects.add_effect(replacement(name, source, if prevented { ReplacementAction::Prevent } else { ReplacementAction::Instead(vec![Effect::gain_life(1)]) }));
        let mut dm = Choices { accept: true, ..Default::default() };
        let outcome = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        assert_eq!(notices(&outcome), 1);
        assert!(game.battlefield.contains(&source));
        assert_eq!(game.player(A).unwrap().mana_pool.red, 0);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 4);
        assert_eq!(game.player(A).unwrap().life, 20 + if prevented { 0 } else if name == "Jötun Grunt" { 4 } else { 2 });
    } } }
}

#[test]
fn action_replacement_pending_and_error_restore_resources_and_never_sacrifice() {
    for name in ["Braid of Fire", "Jötun Grunt", "Psychic Vortex"] { for definition in definitions(name) { for pause in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, 2);
        for _ in 0..4 { card(&mut game, A, Zone::Graveyard, "Available cost"); card(&mut game, A, Zone::Library, "Available draw"); }
        let effects = if pause { vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(2)])] }
            else { vec![Effect::gain_life(3), Effect::lose_life(ironsmith::effect::Value::X)] };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(replacement(name, source, ReplacementAction::Additionally(effects)));
        let next_id = game.next_object_id_counter();
        let mut dm = Choices { accept: true, pause_boolean: pause.then_some(2), ..Default::default() };
        let result = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm));
        if pause { assert_eq!(notices(&result.unwrap()), 0); assert!(dm.pending); }
        else { assert!(matches!(result, Err(ironsmith::effects::ExecutionError::UnresolvableValue(_)))); }
        assert!(game.battlefield.contains(&source));
        assert_eq!(game.player(A).unwrap().mana_pool.red, 0);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 4);
        assert_eq!(game.player(A).unwrap().library.len(), 4);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), next_id);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert!(game.take_pending_trigger_events().is_empty());
    } } }
}

#[test]
fn action_cost_resource_exhaustion_remains_incomplete_and_restores_every_owner() {
    for name in ["Braid of Fire", "Jötun Grunt", "Psychic Vortex"] { for definition in definitions(name) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, 2);
        for _ in 0..4 { card(&mut game, A, Zone::Graveyard, "Available cost"); card(&mut game, A, Zone::Library, "Available draw"); }
        game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() });
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(replacement(name, source, ReplacementAction::Additionally(vec![
            Effect::gain_life(3), Effect::new(ironsmith::effects::CreateTokenEffect::you(ironsmith::cards::tokens::treasure_token_definition(), 2)),
        ])));
        let next = game.next_object_id_counter();
        let mut dm = Choices { accept: true, ..Default::default() };
        let result = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm));
        assert!(matches!(result, Err(ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. })));
        assert!(game.battlefield.contains(&source));
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(A).unwrap().mana_pool.red, 0);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 4);
        assert_eq!(game.player(A).unwrap().library.len(), 4);
        assert_eq!(game.next_object_id_counter(), next);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert!(game.take_pending_trigger_events().is_empty());
    } }
}

#[test]
fn draw_prohibitions_are_unpayable_but_empty_library_draws_and_replacements_are_payments() {
    for definition in definitions("Psychic Vortex") { for mode in 0..3 {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, if mode == 0 { 1 } else { 2 });
        if mode != 0 {
            let text = if mode == 1 { "Type: Enchantment\nPlayers can't draw cards." }
                else { "Type: Enchantment\nEach opponent can't draw more than one card each turn." };
            let prohibition = ironsmith_compiler_runtime::compile_to_runtime_definition("Draw prohibition", text, false).unwrap();
            game.create_object_from_definition(&prohibition, B, Zone::Battlefield);
        }
        let mut dm = Choices { accept: true, ..Default::default() };
        let outcome = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        assert_eq!(notices(&outcome), usize::from(mode == 0));
        assert_eq!(game.battlefield.contains(&source), mode == 0);
        assert_eq!(game.player(A).unwrap().attempted_draw_from_empty_library, mode == 0);
    } }
}

#[test]
fn all_upkeep_triggers_ignore_a_blinked_stable_identity_successor() {
    for name in ["Braid of Fire", "Jötun Grunt", "Psychic Vortex"] { for definition in definitions(name) {
        let mut game = game();
        let original = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices { accept: true, ..Default::default() };
        game.queue_trigger_event(Default::default(), TriggerEvent::new(ironsmith::events::phase::BeginningOfUpkeepEvent::new(A), Default::default()));
        put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        let cause = ironsmith::events::cause::EventCause::from_effect(original, A);
        let exile = game.move_object(original, Zone::Exile, cause.clone()).unwrap();
        let successor = game.move_object(exile, Zone::Battlefield, cause).unwrap();
        assert_ne!(original, successor);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(successor, CounterType::Age), 0);
        assert_eq!(dm.boolean_calls, 0);
        assert!(game.battlefield.contains(&successor));
    } }
}

#[test]
fn vortex_draws_capture_observers_before_replacement_additions_create_new_ones() {
    for definition in definitions("Psychic Vortex") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, 2);
        for _ in 0..3 { card(&mut game, A, Zone::Library, "Drawn land"); }
        let observer = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Draw watcher", "Type: Creature — Illusion\nPower/Toughness: 1/1\nWhenever you draw a card, you gain 1 life.", false,
        ).unwrap();
        game.effect_store.replacement_effects.add_effect(replacement("Psychic Vortex", source,
            ReplacementAction::Additionally(vec![Effect::new(ironsmith::effects::CreateTokenEffect::you(observer, 1))])));
        let mut dm = Choices { accept: true, ..Default::default() };
        let outcome = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        assert_eq!(notices(&outcome), 1);
        for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "only the first created observer sees the second original draw");
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}

#[test]
fn nonzero_decline_keeps_all_payment_resources_and_sacrifices_the_exact_source() {
    for name in ["Braid of Fire", "Jötun Grunt", "Psychic Vortex"] { for definition in definitions(name) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, 2);
        let graveyard: Vec<_> = (0..4).map(|_| card(&mut game, A, Zone::Graveyard, "Cost card")).collect();
        for _ in 0..3 { card(&mut game, A, Zone::Library, "Private card"); }
        let mut dm = Choices::default();
        let outcome = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        assert_eq!(dm.boolean_calls, 1);
        assert_eq!(notices(&outcome), 0);
        assert!(!game.battlefield.contains(&source));
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool.red, 0);
        assert_eq!(game.player(A).unwrap().library.len(), 3);
        for id in graveyard { assert_eq!(game.object(id).unwrap().zone, Zone::Graveyard); }
    } }
}

#[test]
fn paid_notice_retains_original_source_lki_when_an_action_replacement_moves_it() {
    for definition in definitions("Braid of Fire") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::Age, 1);
        game.effect_store.replacement_effects.add_one_shot_effect(replacement("Braid of Fire", source,
            ReplacementAction::Instead(vec![Effect::exile(ironsmith::target::ChooseSpec::Source)])));
        let mut dm = Choices { accept: true, ..Default::default() };
        let outcome = payment(&definition).execute(&mut game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        let paid = outcome.events.iter().filter_map(|event| event.downcast::<ironsmith::events::other::KeywordActionEvent>())
            .find(|event| event.action == ironsmith::events::KeywordActionKind::CumulativeUpkeepPaid).unwrap();
        assert_eq!(paid.source, source);
        assert_eq!(paid.snapshot.as_ref().unwrap().object_id, source);
        assert_eq!(paid.snapshot.as_ref().unwrap().zone, Zone::Battlefield);
        assert!(!game.battlefield.contains(&source));
        assert_eq!(game.player(A).unwrap().mana_pool.red, 0);
    }
}

#[test]
fn vortex_discards_its_whole_hand_even_when_the_end_step_has_no_land_to_sacrifice() {
    for definition in definitions("Psychic Vortex") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..3 { card(&mut game, A, Zone::Hand, "Discarded card"); }
        let mut dm = Choices { accept: true, ..Default::default() };
        resolve_event(&mut game, TriggerEvent::new(ironsmith::events::phase::BeginningOfEndStepEvent::new(A), Default::default()), &mut dm);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
        assert!(game.battlefield.contains(&source));
    }
}

#[test]
fn prevented_age_counter_still_requires_an_explicit_zero_payment_on_the_real_trigger() {
    for name in ["Braid of Fire", "Jötun Grunt", "Psychic Vortex"] { for definition in definitions(name) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, A, ironsmith::events::counters::matchers::WouldPutCountersMatcher::any(), ReplacementAction::Prevent,
        ));
        let mut dm = Choices { accept: true, ..Default::default() };
        resolve_event(&mut game, TriggerEvent::new(ironsmith::events::phase::BeginningOfUpkeepEvent::new(A), Default::default()), &mut dm);
        assert_eq!(dm.boolean_calls, 1);
        assert_eq!(game.counter_count(source, CounterType::Age), 0);
        assert!(game.battlefield.contains(&source));
        assert_eq!(game.player(A).unwrap().mana_pool.red, 0);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert!(!game.player(A).unwrap().attempted_draw_from_empty_library);
    } }
}
