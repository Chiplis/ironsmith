//! UNVALIDATED conditional base-size replacement of a complete animation.
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
        "../../../fixtures/conditional_animation_pt.json.fixture"
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
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
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

#[test]
fn exact_card_keeps_one_target_and_a_complete_animation_in_both_self_replacement_branches() {
    for definition in definitions("Behind the Mask") {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let program = definition.spell_effect.as_ref().unwrap();
        assert_eq!(program.segments.len(), 1);
        assert_eq!(program.segments[0].self_replacements.len(), 1);
        let debug = format!("{program:?}");
        assert!(debug.contains("Evidence"), "{debug}");
        assert_eq!(debug.matches("AddCardTypes").count(), 2, "{debug}");
    }
}

#[test]
fn actual_optional_evidence_payment_changes_only_base_size_and_keeps_existing_types_and_modifiers()
{
    for definition in definitions("Behind the Mask") {
        for paid in [false, true] {
            for kind in ["Artifact", "Creature — Elf", "Artifact Land"] {
                let mut game = game();
                let target_def = resource("Animation recipient", kind);
                let target = game.create_object_from_definition(&target_def, B, Zone::Battlefield);
                let printed = game
                    .current_power(target)
                    .zip(game.current_toughness(target));
                counter(&mut game, target, target, 1);
                // A creature's existing pump remains in the later layer even
                // though the animation is created after it. Noncreatures can
                // receive that pump only after becoming creatures.
                if kind.contains("Creature") {
                    pump(&mut game, target, target, 3, 2);
                }
                let evidence_def = compile_to_runtime_definition(
                    "Evidence payment",
                    "Mana cost: {8}\nType: Artifact",
                    false,
                )
                .unwrap();
                let evidence =
                    game.create_object_from_definition(&evidence_def, A, Zone::Graveyard);
                let stable = game.object(evidence).unwrap().stable_id;
                let mut dm = Choices {
                    decline: !paid,
                    objects: vec![evidence],
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                };
                cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
                let payment = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(
                    game.object(payment).unwrap().zone,
                    if paid { Zone::Exile } else { Zone::Graveyard }
                );
                resolve_all(&mut game, &mut dm);
                if !kind.contains("Creature") {
                    pump(&mut game, target, target, 3, 2);
                }
                assert!(game.object_has_card_type(target, ironsmith::CardType::Artifact));
                assert!(game.object_has_card_type(target, ironsmith::CardType::Creature));
                assert_eq!(
                    game.object_has_card_type(target, ironsmith::CardType::Land),
                    kind == "Artifact Land"
                );
                assert_eq!(pt(&game, target), if paid { (5, 4) } else { (8, 6) });
                if kind.contains("Elf") {
                    assert!(
                        game.calculated_subtypes(target)
                            .contains(&ironsmith::Subtype::Elf)
                    );
                }
                ironsmith::turn::execute_cleanup_step(&mut game);
                assert_eq!(
                    game.object_has_card_type(target, ironsmith::CardType::Creature),
                    kind.contains("Creature")
                );
                if let Some((p, t)) = printed {
                    assert_eq!(pt(&game, target), (p + 1, t + 1));
                }
            }
        }
    }
}

#[test]
fn unrelated_evidence_collected_this_turn_does_not_mark_this_spell_paid() {
    for definition in definitions("Behind the Mask") {
        let mut game = game();
        let target = game.create_object_from_definition(
            &resource("Artifact target", "Artifact"),
            B,
            Zone::Battlefield,
        );
        let evidence_def = compile_to_runtime_definition(
            "Earlier evidence",
            "Mana cost: {6}\nType: Artifact",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&evidence_def, A, Zone::Graveyard);
        let mut earlier = SelectFirstDecisionMaker;
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::CollectEvidenceEffect::new(6)),
            &mut EffectContext::new(target, A, &mut earlier),
        )
        .unwrap();
        let mut dm = Choices {
            decline: true,
            targets: vec![Target::Object(target)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, target), (4, 3));
    }
}
