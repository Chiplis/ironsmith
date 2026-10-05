//! UNVALIDATED full-card complete ability-loss template scenarios. No execution under deferred workflow.
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
        "../../../fixtures/ability_loss_templates.json.fixture"
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
fn subject(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let definition=compile_to_runtime_definition("Former knight","Mana cost: {R}\nType: Snow Artifact Creature — Human Knight\nPower/Toughness: 4/4\nFlying\n{1}: This creature gets +1/+1 until end of turn.",false).unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}
fn cleanup(game: &mut GameState) {
    ironsmith::turn::execute_cleanup_step(game);
    game.refresh_continuous_state().unwrap();
}
#[test]
fn four_full_templates_round_trip_with_no_dropped_instruction() {
    let rows = fixtures()
        .into_iter()
        .filter(|row| row["proposed_complete"] == true)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 4);
    for row in rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn targeted_templates_lose_abilities_and_creature_types_but_keep_other_card_types_and_counters() {
    use ironsmith::static_abilities::StaticAbilityId as Id;
    use ironsmith::{CardType, Subtype, Supertype};
    for (name, size, subtype, color) in [
        (
            "Gift of Tusks",
            3,
            Subtype::Elephant,
            ironsmith::color::ColorSet::GREEN,
        ),
        (
            "Turn to Frog",
            1,
            Subtype::Frog,
            ironsmith::color::ColorSet::BLUE,
        ),
        (
            "Snakeform",
            1,
            Subtype::Snake,
            ironsmith::color::ColorSet::GREEN,
        ),
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let target = subject(&mut game, B);
            for _ in 0..2 {
                game.create_object_from_definition(
                    &vanilla("Draw resource", "{1}", "Human", 1, 1),
                    A,
                    Zone::Library,
                );
            }
            counter(&mut game, target, target, 2);
            let spell = cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                },
            );
            assert_eq!(
                game.stack.last().unwrap().targets,
                vec![Target::Object(target)]
            );
            assert!(game.object(spell).is_some());
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(pt(&game, target), (size + 2, size + 2));
            assert!(!has(&game, target, Id::Flying));
            assert!(
                game.calculated_characteristics(target)
                    .unwrap()
                    .abilities
                    .iter()
                    .all(|ability| !matches!(
                        ability.kind,
                        ironsmith::ability::AbilityKind::Activated(_)
                    ))
            );
            assert!(game.object_has_card_type(target, CardType::Artifact));
            assert!(game.object_has_card_type(target, CardType::Creature));
            assert!(game.current_has_supertype(target, Supertype::Snow));
            let types = game.current_subtypes(target).unwrap();
            assert!(types.contains(&subtype));
            assert!(!types.contains(&Subtype::Human));
            assert!(!types.contains(&Subtype::Knight));
            assert_eq!(game.current_colors(target), Some(color));
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                usize::from(name == "Snakeform")
            );
            cleanup(&mut game);
            assert_eq!(pt(&game, target), (6, 6));
            assert!(has(&game, target, Id::Flying));
            assert!(
                game.current_subtypes(target)
                    .unwrap()
                    .contains(&Subtype::Knight)
            );
        }
    }
}
#[test]
fn polymorphist_targets_one_player_and_locks_the_resolving_creature_set() {
    use ironsmith::static_abilities::StaticAbilityId as Id;
    for definition in definitions("Polymorphist's Jest") {
        let mut game = game();
        let affected = subject(&mut game, B);
        let other = subject(&mut game, C);
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices {
                targets: vec![Target::Player(B)],
                ..Default::default()
            },
        );
        assert_eq!(game.stack.last().unwrap().targets, vec![Target::Player(B)]);
        let before_resolution = subject(&mut game, B);
        resolve_all(&mut game, &mut Choices::default());
        for id in [affected, before_resolution] {
            assert_eq!(pt(&game, id), (1, 1));
            assert!(!has(&game, id, Id::Flying));
        }
        assert_eq!(pt(&game, other), (4, 4));
        assert!(has(&game, other, Id::Flying));
        let after_resolution = subject(&mut game, B);
        assert_eq!(pt(&game, after_resolution), (4, 4));
        assert!(has(&game, after_resolution, Id::Flying));
        game.set_current_controller(affected, C).unwrap();
        assert_eq!(pt(&game, affected), (1, 1));
        cleanup(&mut game);
        assert_eq!(pt(&game, affected), (4, 4));
        assert!(has(&game, affected, Id::Flying));
    }
}
#[test]
fn snakeform_has_no_draw_when_its_only_target_is_illegal_on_resolution() {
    for definition in definitions("Snakeform") {
        let mut game = game();
        let target = subject(&mut game, B);
        game.create_object_from_definition(
            &vanilla("Draw resource", "{1}", "Human", 1, 1),
            A,
            Zone::Library,
        );
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        game.move_object_by_game_rule(target, Zone::Graveyard)
            .unwrap();
        resolve_all(&mut game, &mut Choices::default());
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}
