//! UNVALIDATED paired animation quantities and pre-P/T grants.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, TargetsContext,
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
        "../../../fixtures/paired_pt_animation.json.fixture"
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
}
impl DecisionMaker for Choices {
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

#[test]
fn exact_woodcaller_transports_paired_values_haste_and_land_retention() {
    for definition in definitions("Woodcaller Automaton") {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        for required in ["PowerOf", "ToughnessOf", "Haste", "AddCardTypes"] {
            assert!(debug.contains(required), "{required}: {debug}");
        }
    }
}

#[test]
fn normal_and_prototype_casts_copy_distinct_current_axes_or_exact_source_lki_onto_the_land() {
    for definition in definitions("Woodcaller Automaton") {
        for prototype in [false, true] {
            for blink in [false, true] {
                let mut game = game();
                let land_definition = compile_to_runtime_definition(
                    "Forest",
                    "Type: Basic Land — Forest\n{T}: Add {G}.",
                    false,
                )
                .unwrap();
                let land =
                    game.create_object_from_definition(&land_definition, A, Zone::Battlefield);
                apply(
                    &mut game,
                    land,
                    Effect::tap(ChooseSpec::SpecificObject(land)),
                );
                counter(&mut game, land, land, 1);
                let mut dm = Choices {
                    targets: vec![Target::Object(land)],
                    ..Default::default()
                };
                let method = if prototype {
                    CastingMethod::Alternative(
                        definition
                            .alternative_casts
                            .iter()
                            .position(|method| method.name().eq_ignore_ascii_case("prototype"))
                            .unwrap(),
                    )
                } else {
                    CastingMethod::Normal
                };
                let spell = cast(&mut game, &definition, method, &mut dm);
                let stable = game.object(spell).unwrap().stable_id;
                let mut queue = TriggerQueue::new();
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                assert_eq!(game.stack.len(), 1);
                let source = game.find_object_by_stable_id(stable).unwrap();
                let base = if prototype { 3 } else { 8 };
                assert_eq!(pt(&game, source), (base, base));
                pump(&mut game, source, source, 2, 4);
                if blink {
                    let grave = game
                        .move_object_by_game_rule(source, Zone::Graveyard)
                        .unwrap();
                    let returned = game
                        .move_object_by_game_rule(grave, Zone::Battlefield)
                        .unwrap();
                    pump(&mut game, returned, returned, 40, 40);
                }
                resolve_all(&mut game, &mut dm);
                assert!(!game.is_tapped(land));
                assert_eq!(pt(&game, land), (base + 3, base + 5));
                assert!(game.object_has_card_type(land, ironsmith::CardType::Land));
                assert!(game.object_has_card_type(land, ironsmith::CardType::Creature));
                assert!(
                    game.calculated_subtypes(land)
                        .contains(&ironsmith::Subtype::Treefolk)
                );
                assert!(
                    game.calculated_subtypes(land)
                        .contains(&ironsmith::Subtype::Forest)
                );
                assert!(game.current_has_static_ability_id(
                    land,
                    ironsmith::static_abilities::StaticAbilityId::Haste
                ));
                if !blink {
                    pump(&mut game, source, source, 6, 6);
                }
                ironsmith::turn::execute_cleanup_step(&mut game);
                assert_eq!(pt(&game, land), (base + 3, base + 5));
                assert!(game.current_has_static_ability_id(
                    land,
                    ironsmith::static_abilities::StaticAbilityId::Haste
                ));
            }
        }
    }
}

#[test]
fn a_noncast_entry_cannot_untap_or_animate_the_target_land() {
    for definition in definitions("Woodcaller Automaton") {
        let mut game = game();
        let land = game.create_object_from_definition(
            &resource("Land", "Land — Forest"),
            A,
            Zone::Battlefield,
        );
        apply(
            &mut game,
            land,
            Effect::tap(ChooseSpec::SpecificObject(land)),
        );
        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices::default();
        let outcome = apply(
            &mut game,
            land,
            Effect::put_onto_battlefield(
                ChooseSpec::SpecificObject(card),
                false,
                PlayerFilter::You,
            ),
        );
        queue_outcome(&mut game, outcome, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert!(game.is_tapped(land));
        assert!(!game.object_has_card_type(land, ironsmith::CardType::Creature));
    }
}
