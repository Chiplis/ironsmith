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
        "../../../fixtures/target_characteristic_comparisons.json.fixture"
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
        .rev()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}
fn creature(game: &mut GameState, player: PlayerId, power: i32) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("Power witness", "{1}", "Human", power, 20),
        player,
        Zone::Battlefield,
    )
}
fn pump(game: &mut GameState, id: ObjectId, amount: i32) {
    apply(
        game,
        id,
        Effect::pump(amount, 0, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
#[test]
fn fell_declares_one_real_creature_target_inside_the_comparison_operand() {
    for definition in definitions("Fell the Mighty") {
        fn visit(effect: &Effect, targets: &mut Vec<ChooseSpec>, saw: &mut bool) {
            if let Some(effect) = effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>() {
                targets.push(effect.target.clone());
            }
            if let Some(effect) = effect.downcast_ref::<ironsmith::effects::DestroyEffect>() {
                let ChooseSpec::All(filter) = effect.spec.base() else {
                    panic!("{:?}", effect.spec);
                };
                let Some(ironsmith::filter::Comparison::GreaterThanExpr(rhs)) = &filter.power
                else {
                    panic!("{filter:?}");
                };
                let ironsmith::effect::Value::PowerOf(spec) = rhs.unhinted() else {
                    panic!("{rhs:?}");
                };
                assert!(spec.is_target());
                assert!(matches!(spec.base(), ChooseSpec::Object(_)));
                *saw = true;
            }
            effect.visit_child_effects(&mut |child| visit(child, targets, saw));
        }
        let mut targets = Vec::new();
        let mut saw = false;
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects {
                visit(effect, &mut targets, &mut saw);
            }
        }
        assert!(saw);
        assert_eq!(targets.len(), 1);
        assert!(targets[0].is_target());
    }
}
#[test]
fn fell_uses_resolution_power_without_targeting_the_destroyed_set() {
    for definition in definitions("Fell the Mighty") {
        for response in [0, 2, -2] {
            let mut game = game();
            let reference = creature(&mut game, A, 2);
            let small = creature(&mut game, B, 1);
            let middle = creature(&mut game, B, 3);
            let large = creature(&mut game, C, 5);
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Object(reference)],
                    ..Default::default()
                },
            );
            pump(&mut game, reference, response);
            resolve(&mut game, &mut Choices::default());
            assert!(
                game.object(reference).is_some(),
                "reference never exceeds its own current power"
            );
            assert_eq!(game.object(small).is_none(), 1 > 2 + response);
            assert_eq!(game.object(middle).is_none(), 3 > 2 + response);
            assert!(game.object(large).is_none());
        }
    }
}
#[test]
fn fell_loses_its_only_target_on_blink_or_shroud_and_does_not_destroy_anything() {
    for definition in definitions("Fell the Mighty") {
        for blink in [false, true] {
            let mut game = game();
            let reference = creature(&mut game, A, 2);
            let large = creature(&mut game, B, 9);
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Object(reference)],
                    ..Default::default()
                },
            );
            if blink {
                let exile = game
                    .move_object_by_game_rule(reference, Zone::Exile)
                    .unwrap();
                let returned = game
                    .move_object_by_game_rule(exile, Zone::Battlefield)
                    .unwrap();
                assert_ne!(reference, returned);
                pump(&mut game, returned, -2);
            } else {
                let protection = compile_to_runtime_definition(
                    "Target protection",
                    "Mana cost: {0}\nType: Instant\nTarget creature gains shroud until end of turn.",
                    false,
                )
                .unwrap();
                let mut dm = Choices {
                    targets: vec![Target::Object(reference)],
                    ..Default::default()
                };
                cast(&mut game, &protection, CastingMethod::Normal, &mut dm);
                resolve(&mut game, &mut Choices::default());
            }
            resolve(&mut game, &mut Choices::default());
            assert!(game.object(large).is_some());
        }
    }
}
#[test]
fn tagged_comparison_reads_live_power_and_exact_departure_lki_after_a_blink() {
    use ironsmith::effect::Value;
    use ironsmith::filter::{Comparison, ObjectFilter};
    use ironsmith::game_state::StackEntry;
    use ironsmith::snapshot::ObjectSnapshot;
    for departed in [false, true] {
        let mut game = game();
        let source = creature(&mut game, A, 1);
        let reference = creature(&mut game, A, 2);
        let middle = creature(&mut game, B, 4);
        let large = creature(&mut game, B, 7);
        let filter = ObjectFilter::creature().with_power(Comparison::GreaterThanExpr(Box::new(
            Value::PowerOf(Box::new(ChooseSpec::Tagged("reference".into()))),
        )));
        let mut entry = StackEntry::ability(source, A, vec![Effect::destroy_all(filter)]);
        entry.tagged_objects.insert(
            "reference".into(),
            vec![ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(reference).unwrap(),
                &game,
            )],
        );
        game.push_to_stack(entry);
        pump(&mut game, reference, 3);
        if departed {
            let grave = game
                .move_object_by_game_rule(reference, Zone::Graveyard)
                .unwrap();
            let returned = game
                .move_object_by_game_rule(grave, Zone::Battlefield)
                .unwrap();
            pump(&mut game, returned, 40);
            assert_eq!(
                game.stack[0].tagged_objects["reference"][0].object_id,
                reference
            );
            assert_eq!(game.stack[0].tagged_objects["reference"][0].power, Some(5));
        }
        resolve(&mut game, &mut Choices::default());
        assert!(
            game.object(middle).is_some(),
            "live or departure power is five, never initial two"
        );
        assert!(
            game.object(large).is_none(),
            "the later incarnation's power must not replace the five-power LKI"
        );
    }
}

#[test]
fn active_resolution_tag_prefers_the_exact_departure_receipt_over_its_earlier_selection() {
    use ironsmith::effect::Value;
    use ironsmith::filter::{Comparison, FilterContext, ObjectFilter};
    use ironsmith::snapshot::ObjectSnapshot;
    let mut game = game();
    let source = creature(&mut game, A, 1);
    let reference = creature(&mut game, A, 2);
    let candidate = creature(&mut game, B, 4);
    // This retained context is deliberately not a pending stack entry: the
    // movement transaction cannot mutate its earlier selection snapshot.
    let tags = std::collections::HashMap::from([(
        "selected".into(),
        vec![ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(reference).unwrap(),
            &game,
        )],
    )]);
    let filter = ObjectFilter::creature().with_power(Comparison::LessThanExpr(Box::new(
        Value::PowerOf(Box::new(ChooseSpec::Tagged("selected".into()))),
    )));
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, A);
    ctx.tagged_objects = tags.clone();
    assert!(
        !ironsmith::effects::helpers::resolve_objects_from_spec(
            &game,
            &ChooseSpec::All(filter.clone()),
            &ctx
        )
        .unwrap()
        .contains(&candidate)
    );
    pump(&mut game, reference, 3);
    let exiled = game
        .move_object_by_game_rule(reference, Zone::Exile)
        .unwrap();
    let returned = game
        .move_object_by_game_rule(exiled, Zone::Battlefield)
        .unwrap();
    pump(&mut game, returned, -2);
    game.move_object_by_game_rule(returned, Zone::Graveyard)
        .unwrap();
    assert_eq!(tags["selected"][0].power, Some(2));
    assert!(
        ironsmith::effects::helpers::resolve_objects_from_spec(
            &game,
            &ChooseSpec::All(filter.clone()),
            &ctx
        )
        .unwrap()
        .contains(&candidate),
        "four is less than exact departure power five; initial selection and later incarnation both differ"
    );
}
