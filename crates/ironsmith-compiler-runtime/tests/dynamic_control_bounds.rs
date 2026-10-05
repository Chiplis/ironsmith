//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext, ViewCardsContext,
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
        "../../../fixtures/dynamic_control_bounds.json.fixture"
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
    mode: Option<usize>,
    x: u32,
    prefer_life: bool,
    viewed: Vec<Vec<ObjectId>>,
    untap: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        if ctx.description.starts_with("untap ") {
            self.untap
        } else {
            true
        }
    }
    fn view_cards(
        &mut self,
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        _context: &ViewCardsContext,
    ) {
        if viewer == A {
            self.viewed.push(cards.to_vec());
        }
    }

    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(mode) = self.mode
            && context.description.starts_with("Choose mode for")
        {
            assert!(
                context
                    .options
                    .iter()
                    .any(|option| option.index == mode && option.legal)
            );
            return vec![mode];
        }
        if self.prefer_life && context.description.starts_with("Choose how to pay pip") {
            if let Some(option) = context.options.iter().find(|option| {
                option.legal && option.description.to_ascii_lowercase().contains("life")
            }) {
                return vec![option.index];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_number(&mut self, game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value {
            assert!(self.x <= context.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, context)
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
        if !self.objects.is_empty() {
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
fn queue_event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) -> usize {
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) {
        queue.add(entry);
    }
    let count = queue.entries.len();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    count
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
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
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
    for _ in 0..50 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
}
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}

fn control_spec(def: &CardDefinition) -> ChooseSpec {
    fn walk(effect: &Effect, found: &mut Vec<ChooseSpec>) {
        if let Some(control) = effect.downcast_ref::<ironsmith::effects::GainControlEffect>() {
            found.push(control.target.clone());
        }
        if let Some(control) = effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>()
            && control.runtime_modifications.iter().any(|modification| matches!(modification,
                ironsmith::effects::continuous::RuntimeModification::ChangeControllerToEffectController
                | ironsmith::effects::continuous::RuntimeModification::ChangeControllerToPlayer(_)))
        {
            found.push(control.target_spec.clone().expect("control instruction has a target specification"));
        }
        effect.visit_child_effects(&mut |child| walk(child, found));
    }
    let mut specs = Vec::new();
    let ironsmith::ability::AbilityKind::Activated(ability) = &def.abilities[activated(def)].kind
    else {
        panic!();
    };
    for segment in &ability.effects.segments {
        for effect in &segment.default_effects {
            walk(effect, &mut specs);
        }
    }
    assert_eq!(specs.len(), 1);
    specs.remove(0)
}
fn allowed(game: &GameState, source: ObjectId, spec: &ChooseSpec, target: ObjectId) -> bool {
    let mut dm = SelectFirstDecisionMaker;
    let ctx = EffectContext::new(source, A, &mut dm);
    ironsmith::targeting::compute_legal_targets_with_execution_context(game, spec, &ctx)
        .contains(&Target::Object(target))
}
fn creature(game: &mut GameState, player: PlayerId, power: i32) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("Bound witness", "{1}", "Human", power, 20),
        player,
        Zone::Battlefield,
    )
}
fn island(game: &mut GameState, player: PlayerId) -> ObjectId {
    let def = compile_to_runtime_definition("Island witness", "Type: Basic Land — Island", false)
        .unwrap();
    game.create_object_from_definition(&def, player, Zone::Battlefield)
}
#[test]
fn exact_cards_preserve_a_dynamic_target_comparison_separate_from_control_duration() {
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            let spec = control_spec(&definition);
            let ChooseSpec::Object(filter) = spec.base() else {
                panic!("{spec:?}");
            };
            let Some(ironsmith_core::filter_model::Comparison::LessThanOrEqualExpr(value)) =
                &filter.power
            else {
                panic!("missing typed bound");
            };
            let ironsmith_core::Value::Count(counted) = value.unhinted() else {
                panic!("{value:?}");
            };
            assert_eq!(counted.controller, Some(PlayerFilter::You));
            assert_eq!(
                filter.controller, None,
                "opponents' creatures must remain targetable"
            );
            assert!(spec.is_target());
        }
    }
}
#[test]
fn beguiler_checks_live_creature_count_at_announcement_and_resolution_but_not_after_control() {
    for definition in definitions("Beguiler of Wills") {
        for lose_creature_in_response in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let friend = creature(&mut game, A, 1);
            let other_friend = creature(&mut game, A, 1);
            let target = creature(&mut game, B, 3);
            let too_large = creature(&mut game, B, 4);
            let negative = creature(&mut game, B, -1);
            let spec = control_spec(&definition);
            assert!(allowed(&game, source, &spec, target));
            assert!(!allowed(&game, source, &spec, too_large));
            assert!(allowed(&game, source, &spec, negative));
            activate(
                &mut game,
                source,
                activated(&definition),
                &mut Choices {
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                },
            );
            assert!(game.is_tapped(source));
            if lose_creature_in_response {
                apply(&mut game, friend, Effect::sacrifice_source());
            }
            resolve(&mut game, &mut Choices::default());
            assert_eq!(
                game.current_controller(target),
                Some(if lose_creature_in_response { B } else { A })
            );
            if !lose_creature_in_response {
                apply(&mut game, friend, Effect::sacrifice_source());
                apply(&mut game, other_friend, Effect::sacrifice_source());
                game.move_object_by_game_rule(source, Zone::Exile).unwrap();
                assert_eq!(
                    game.current_controller(target),
                    Some(A),
                    "the resolved control is permanent, not an ongoing power/count gate"
                );
            }
        }
    }
}
#[test]
fn beguiler_ability_keeps_its_controller_when_the_source_changes_controller_in_response() {
    for definition in definitions("Beguiler of Wills") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        creature(&mut game, A, 1);
        creature(&mut game, A, 1);
        let target = creature(&mut game, B, 2);
        activate(
            &mut game,
            source,
            activated(&definition),
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        game.set_current_controller(source, C).unwrap();
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.current_controller(target), Some(A));
    }
}
#[test]
fn shackles_uses_islands_only_for_target_legality_and_tapped_state_only_for_duration() {
    for definition in definitions("Vedalken Shackles") {
        for response in [0, 1, 2] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let first = island(&mut game, A);
            island(&mut game, A);
            island(&mut game, A);
            for _ in 0..5 {
                island(&mut game, B);
            }
            let target = creature(&mut game, B, 3);
            let too_large = creature(&mut game, B, 4);
            let spec = control_spec(&definition);
            assert!(allowed(&game, source, &spec, target));
            assert!(!allowed(&game, source, &spec, too_large));
            activate(
                &mut game,
                source,
                activated(&definition),
                &mut Choices {
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                },
            );
            if response == 1 {
                game.move_object_by_game_rule(first, Zone::Graveyard)
                    .unwrap();
            }
            if response == 2 {
                apply(
                    &mut game,
                    source,
                    Effect::untap(ChooseSpec::SpecificObject(source)),
                );
            }
            resolve(&mut game, &mut Choices::default());
            assert_eq!(
                game.current_controller(target),
                Some(if response == 0 { A } else { B })
            );
            if response == 0 {
                game.move_object_by_game_rule(first, Zone::Graveyard)
                    .unwrap();
                game.set_current_controller(source, C).unwrap();
                assert_eq!(
                    game.current_controller(target),
                    Some(A),
                    "neither island count nor source controller is the duration"
                );
                apply(
                    &mut game,
                    source,
                    Effect::untap(ChooseSpec::SpecificObject(source)),
                );
                assert_eq!(game.current_controller(target), Some(B));
                apply(
                    &mut game,
                    source,
                    Effect::tap(ChooseSpec::SpecificObject(source)),
                );
                assert_eq!(
                    game.current_controller(target),
                    Some(B),
                    "expired control never restarts on retap"
                );
            } else if response == 2 {
                apply(
                    &mut game,
                    source,
                    Effect::tap(ChooseSpec::SpecificObject(source)),
                );
                assert_eq!(
                    game.current_controller(target),
                    Some(B),
                    "false initial duration never creates control"
                );
            }
        }
    }
}
#[test]
fn shackles_printed_optional_untap_choice_actually_keeps_then_releases_control() {
    for definition in definitions("Vedalken Shackles") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        island(&mut game, A);
        let target = creature(&mut game, B, 1);
        activate(
            &mut game,
            source,
            activated(&definition),
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.current_controller(target), Some(A));
        ironsmith::turn::execute_untap_step_with(
            &mut game,
            &mut Choices {
                untap: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(game.is_tapped(source));
        assert_eq!(game.current_controller(target), Some(A));
        ironsmith::turn::execute_untap_step_with(
            &mut game,
            &mut Choices {
                untap: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!game.is_tapped(source));
        assert_eq!(game.current_controller(target), Some(B));
    }
}
