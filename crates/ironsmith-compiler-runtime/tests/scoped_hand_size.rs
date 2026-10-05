//! UNVALIDATED full-card scoped hand-size scenarios. No execution under deferred workflow.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/scoped_hand_size.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    definitions_text(name, &lines.join("\n"))
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn vanilla(name: &str, cost: &str, subtype: &str, p: i32, t: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {cost}\nType: Creature — {subtype}\nPower/Toughness: {p}/{t}"),
        false,
    )
    .unwrap()
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    objects_explicit: bool,
    x: u32,
    decline: bool,
    mode: usize,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose mode") {
            return vec![ctx.options[self.mode].index];
        }
        if ctx.description.starts_with("Choose optional costs") {
            if self.decline {
                return vec![];
            }
            return ctx
                .options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            assert!(self.x <= ctx.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if !self.targets.is_empty() {
            assert_eq!(context.requirements.len(), self.targets.len());
            for (requirement, target) in context.requirements.iter().zip(&self.targets) {
                assert!(requirement.legal_targets.contains(target));
            }
            self.targets.clone()
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if self.objects_explicit || !self.objects.is_empty() {
            for id in &self.objects {
                assert!(
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                );
            }
            self.objects.clone()
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let mut dm = SelectFirstDecisionMaker;
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("unexpected continuing trigger chain");
}
fn resource(name: &str, types: &str) -> CardDefinition {
    let pt = if types.contains("Creature") {
        "\nPower/Toughness: 1/1"
    } else {
        ""
    };
    compile_to_runtime_definition(name, format!("Mana cost: {{1}}\nType: {types}{pt}"), false)
        .unwrap()
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    // Checked execution already captures some triggers in the original observer frame.
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.current_power(id).unwrap(),
        game.current_toughness(id).unwrap(),
    )
}
fn pump(game: &mut GameState, source: ObjectId, id: ObjectId, p: i32, t: i32) {
    apply(
        game,
        source,
        Effect::pump(p, t, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
fn counter(game: &mut GameState, source: ObjectId, id: ObjectId, count: i32) {
    apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::SpecificObject(id),
        ),
    );
}
fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}

fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    let payer = game.current_controller(source).unwrap();
    game.turn.priority_player = Some(payer);
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(
        compute_legal_actions(game, payer)
            .unwrap()
            .contains(&action)
    );
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn activated_at(definition: &CardDefinition, position: usize) -> usize {
    definition
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}
fn current_activated_at(game: &GameState, source: ObjectId, position: usize) -> usize {
    game.calculated_characteristics(source)
        .unwrap()
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}
fn has(
    game: &GameState,
    id: ObjectId,
    ability: ironsmith::static_abilities::StaticAbilityId,
) -> bool {
    game.current_has_static_ability_id(id, ability)
}
fn event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn lore(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let outcome = apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::Lore,
            1,
            ChooseSpec::SpecificObject(source),
        ),
    );
    queue_outcome(game, outcome, dm);
    resolve_all(game, &mut Choices::default());
}
fn fill(game: &mut GameState, player: PlayerId, zone: Zone, n: usize) {
    for _ in 0..n {
        game.create_object_from_definition(
            &vanilla("Resource", "{1}", "Human", 1, 1),
            player,
            zone,
        );
    }
}
fn rules(game: &mut GameState) {
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
}
fn draw_step_trigger(game: &mut GameState, player: PlayerId) {
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Draw);
    game.turn.active_player = player;
    event(
        game,
        TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfDrawStepEvent::new(player),
            Default::default(),
        ),
        &mut Choices::default(),
    );
    resolve_all(game, &mut Choices::default());
}
fn permanent_spell(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let spell = cast(
        game,
        definition,
        CastingMethod::Normal,
        &mut Choices::default(),
    );
    let stable = game.object(spell).unwrap().stable_id;
    resolve_all(game, &mut Choices::default());
    game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn five_exact_hand_size_cards_round_trip_with_complete_bodies() {
    let rows = fixtures()
        .into_iter()
        .filter(|r| r["proposed_complete"] == true)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 5);
    for row in rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn anvil_global_limit_and_additional_draw_discard_follow_the_actual_draw_step_player() {
    for definition in definitions("Anvil of Bogardan") {
        let mut game = game();
        let host = permanent_spell(&mut game, &definition);
        for p in [A, B, C] {
            fill(&mut game, p, Zone::Library, 5);
            fill(&mut game, p, Zone::Hand, 9);
        }
        rules(&mut game);
        for p in [A, B, C] {
            assert_eq!(game.player(p).unwrap().max_hand_size, i32::MAX);
        }
        draw_step_trigger(&mut game, B);
        assert_eq!(game.player(B).unwrap().hand.len(), 9);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 1);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 0);
        assert_eq!(game.player(C).unwrap().hand.len(), 9);
        assert!(ironsmith::turn::get_cleanup_discard_spec(&game).is_none());
        game.move_object_by_game_rule(host, Zone::Graveyard)
            .unwrap();
        rules(&mut game);
        assert_eq!(game.player(B).unwrap().max_hand_size, 7);
        assert!(ironsmith::turn::get_cleanup_discard_spec(&game).is_some());
    }
}
#[test]
fn cursed_rack_remembers_its_chosen_opponent_across_control_change_but_not_absence() {
    for definition in definitions("Cursed Rack") {
        let mut game = game();
        let host = permanent_spell(&mut game, &definition);
        rules(&mut game);
        assert_eq!(game.chosen_player(host), Some(B));
        assert_eq!(game.player(B).unwrap().max_hand_size, 4);
        assert_eq!(game.player(A).unwrap().max_hand_size, 7);
        assert_eq!(game.player(C).unwrap().max_hand_size, 7);
        game.set_current_controller(host, C).unwrap();
        rules(&mut game);
        assert_eq!(game.player(B).unwrap().max_hand_size, 4);
        game.phase_out(host);
        rules(&mut game);
        assert_eq!(game.player(B).unwrap().max_hand_size, 7);
        game.phase_in(host);
        rules(&mut game);
        assert_eq!(game.player(B).unwrap().max_hand_size, 4);
    }
}
#[test]
fn folio_pays_two_x_then_mills_each_actual_opponents_distinct_hand_size() {
    for definition in definitions("Folio of Fancies") {
        let mut game = game();
        let host = permanent_spell(&mut game, &definition);
        for p in [A, B, C] {
            fill(&mut game, p, Zone::Library, 12);
        }
        fill(&mut game, B, Zone::Hand, 2);
        fill(&mut game, C, Zone::Hand, 1);
        let before = game.player(A).unwrap().mana_pool.total();
        activate(
            &mut game,
            host,
            activated_at(&definition, 0),
            &mut Choices {
                x: 3,
                ..Default::default()
            },
        );
        assert!(game.is_tapped(host));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 6);
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
        assert_eq!(game.player(B).unwrap().hand.len(), 5);
        assert_eq!(game.player(C).unwrap().hand.len(), 4);
        game.untap(host);
        activate(
            &mut game,
            host,
            activated_at(&definition, 1),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 0);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 5);
        assert_eq!(game.player(C).unwrap().graveyard.len(), 4);
        rules(&mut game);
        for p in [A, B, C] {
            assert_eq!(game.player(p).unwrap().max_hand_size, i32::MAX);
        }
    }
}
#[test]
fn midnight_oil_reads_live_hour_counters_then_discard_trigger_uses_the_same_controller() {
    for definition in definitions("Midnight Oil") {
        let mut game = game();
        fill(&mut game, A, Zone::Library, 10);
        let host = permanent_spell(&mut game, &definition);
        rules(&mut game);
        assert_eq!(
            game.counter_count(host, ironsmith::object::CounterType::Hour),
            7
        );
        assert_eq!(game.player(A).unwrap().max_hand_size, 7);
        for expected in [5, 3, 1, 0] {
            draw_step_trigger(&mut game, A);
            rules(&mut game);
            assert_eq!(
                game.counter_count(host, ironsmith::object::CounterType::Hour),
                expected
            );
            assert_eq!(game.player(A).unwrap().max_hand_size, expected as i32);
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 4);
        assert_eq!(game.player(B).unwrap().max_hand_size, 7);
        let discard = game.player(A).unwrap().hand[0];
        game.take_pending_trigger_events();
        ironsmith::turn::apply_cleanup_discard(&mut game, &[discard], &mut Choices::default())
            .unwrap();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap();
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, 19);
        game.set_current_controller(host, B).unwrap();
        rules(&mut game);
        assert_eq!(game.player(A).unwrap().max_hand_size, 7);
        assert_eq!(game.player(B).unwrap().max_hand_size, 0);
    }
}
#[test]
fn price_of_knowledge_keeps_opponent_damage_separate_from_global_hand_rule() {
    for definition in definitions("Price of Knowledge") {
        let mut game = game();
        permanent_spell(&mut game, &definition);
        fill(&mut game, B, Zone::Hand, 5);
        game.turn.active_player = B;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        event(
            &mut game,
            TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfUpkeepEvent::new(B),
                Default::default(),
            ),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(B).unwrap().life, 15);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(C).unwrap().life, 20);
        rules(&mut game);
        for p in [A, B, C] {
            assert_eq!(game.player(p).unwrap().max_hand_size, i32::MAX);
        }
    }
}
#[test]
fn static_hand_size_rules_use_timestamps_even_when_counter_limit_changes() {
    for oil in definitions("Midnight Oil") {
        for global in definitions("Anvil of Bogardan") {
            for oil_first in [false, true] {
                let mut game = game();
                let oil_id = if oil_first {
                    let oil_id = permanent_spell(&mut game, &oil);
                    permanent_spell(&mut game, &global);
                    oil_id
                } else {
                    permanent_spell(&mut game, &global);
                    permanent_spell(&mut game, &oil)
                };
                rules(&mut game);
                assert_eq!(
                    game.player(A).unwrap().max_hand_size,
                    if oil_first { i32::MAX } else { 7 }
                );
                apply(
                    &mut game,
                    oil_id,
                    Effect::put_counters(
                        ironsmith::object::CounterType::Hour,
                        2,
                        ChooseSpec::SpecificObject(oil_id),
                    ),
                );
                rules(&mut game);
                assert_eq!(
                    game.player(A).unwrap().max_hand_size,
                    if oil_first { i32::MAX } else { 9 }
                );
                assert_eq!(game.player(B).unwrap().max_hand_size, i32::MAX);
            }
        }
    }
}
