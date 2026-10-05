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
        "../../../fixtures/copied_base_power_draw.json.fixture"
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

fn fill_library(game: &mut GameState) {
    let card = vanilla("Draw library card", "{1}", "Human", 1, 1);
    for _ in 0..20 {
        game.create_object_from_definition(&card, A, Zone::Library);
    }
}
fn damage(game: &mut GameState, source: ObjectId, combat: bool) {
    let effect = Effect::new(
        ironsmith::effects::DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(B))
            .with_combat(combat),
    );
    let outcome = apply(game, source, effect);
    queue_outcome(game, outcome, &mut Choices::default());
}
fn copied_resource() -> CardDefinition {
    compile_to_runtime_definition(
        "Copiable robot",
        "Mana cost: {5}\nType: Artifact Creature — Robot\nPower/Toughness: 5/6",
        false,
    )
    .unwrap()
}
#[test]
fn original_damage_trigger_draws_current_base_power_and_ignores_damage_and_pumps() {
    for definition in definitions("Curie, Emergent Intelligence") {
        let mut game = game();
        fill_library(&mut game);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        apply(
            &mut game,
            source,
            Effect::set_base_power_toughness(
                4,
                7,
                ChooseSpec::SpecificObject(source),
                Until::EndOfTurn,
            ),
        );
        apply(
            &mut game,
            source,
            Effect::pump(9, 0, ChooseSpec::SpecificObject(source), Until::EndOfTurn),
        );
        damage(&mut game, source, false);
        assert!(game.stack_is_empty());
        damage(&mut game, source, true);
        assert_eq!(game.stack.len(), 1);
        let before = game.player(A).unwrap().hand.len();
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), before + 4);
    }
}
#[test]
fn real_exile_cost_copies_exact_exiled_incarnation_and_keeps_one_draw_trigger() {
    for definition in definitions("Curie, Emergent Intelligence") {
        for departed in [false, true] {
            let mut game = game();
            fill_library(&mut game);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let paid = game.create_object_from_definition(&copied_resource(), A, Zone::Battlefield);
            let stable = game.object(paid).unwrap().stable_id;
            apply(
                &mut game,
                paid,
                Effect::pump(20, 20, ChooseSpec::SpecificObject(paid), Until::EndOfTurn),
            );
            activate(
                &mut game,
                source,
                activated(&definition),
                &mut Choices {
                    objects: vec![paid],
                    ..Default::default()
                },
            );
            let exiled = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
            assert_ne!(exiled, paid);
            if departed {
                let grave = game
                    .move_object_by_game_rule(exiled, Zone::Graveyard)
                    .unwrap();
                let returned = game
                    .move_object_by_game_rule(grave, Zone::Battlefield)
                    .unwrap();
                apply(
                    &mut game,
                    returned,
                    Effect::set_base_power_toughness(
                        50,
                        50,
                        ChooseSpec::SpecificObject(returned),
                        Until::EndOfTurn,
                    ),
                );
            }
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                game.calculated_characteristics(source).unwrap().base_power,
                Some(5)
            );
            let abilities = game.current_abilities(source).unwrap();
            assert_eq!(
                abilities
                    .iter()
                    .filter(|ability| matches!(
                        ability.kind,
                        ironsmith::ability::AbilityKind::Triggered(_)
                    ))
                    .count(),
                1
            );
            assert!(
                !abilities.iter().any(|ability| matches!(
                    ability.kind,
                    ironsmith::ability::AbilityKind::Activated(_)
                )),
                "the exception grants only the damage trigger, not Curie's copy activation"
            );
            apply(
                &mut game,
                source,
                Effect::pump(10, 0, ChooseSpec::SpecificObject(source), Until::EndOfTurn),
            );
            damage(&mut game, source, true);
            assert_eq!(game.stack.len(), 1);
            game.set_current_controller(source, C).unwrap();
            let before = game.player(A).unwrap().hand.len();
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), before + 5);
            assert_eq!(game.player(C).unwrap().hand.len(), 0);
        }
    }
}
#[test]
fn a_departed_curie_does_not_copy_onto_its_new_incarnation() {
    for definition in definitions("Curie, Emergent Intelligence") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let paid = game.create_object_from_definition(&copied_resource(), A, Zone::Battlefield);
        activate(
            &mut game,
            source,
            activated(&definition),
            &mut Choices {
                objects: vec![paid],
                ..Default::default()
            },
        );
        let exile = game.move_object_by_game_rule(source, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_game_rule(exile, Zone::Battlefield)
            .unwrap();
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(
            game.calculated_characteristics(returned)
                .unwrap()
                .base_power,
            Some(1)
        );
    }
}

#[test]
fn copy_exception_ability_is_copiable_even_when_the_first_copy_loses_abilities_in_layer_six() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, RuntimeModification};
    for definition in definitions("Curie, Emergent Intelligence") {
        let mut game = game();
        fill_library(&mut game);
        for _ in 0..10 {
            game.create_object_from_definition(
                &vanilla("Second controller library", "{1}", "Human", 1, 1),
                C,
                Zone::Library,
            );
        }
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let paid = game.create_object_from_definition(&copied_resource(), A, Zone::Battlefield);
        activate(
            &mut game,
            source,
            activated(&definition),
            &mut Choices {
                objects: vec![paid],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        apply(
            &mut game,
            source,
            Effect::new(ApplyContinuousEffect::new(
                EffectTarget::Specific(source),
                Modification::RemoveAllAbilities,
                Until::EndOfTurn,
            )),
        );
        assert!(game.current_abilities(source).unwrap().is_empty());
        let second = game.create_object_from_definition(
            &vanilla("Second copy", "{1}", "Human", 1, 1),
            C,
            Zone::Battlefield,
        );
        apply(
            &mut game,
            second,
            Effect::new(ApplyContinuousEffect::new_runtime(
                EffectTarget::Specific(second),
                RuntimeModification::CopyOf {
                    source: ChooseSpec::SpecificObject(source),
                    preserve_source_abilities: false,
                    name_override: None,
                    name_override_surface: None,
                    add_supertypes: vec![],
                    copy_exception_surface: None,
                },
                Until::EndOfTurn,
            )),
        );
        assert_eq!(
            game.calculated_characteristics(second).unwrap().base_power,
            Some(5)
        );
        assert_eq!(
            game.current_abilities(second)
                .unwrap()
                .iter()
                .filter(|ability| matches!(
                    ability.kind,
                    ironsmith::ability::AbilityKind::Triggered(_)
                ))
                .count(),
            1
        );
        let effect = Effect::new(
            ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(B))
                .with_combat(true),
        );
        let outcome = apply(&mut game, second, effect);
        queue_outcome(&mut game, outcome, &mut Choices::default());
        assert_eq!(game.stack.len(), 1);
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(C).unwrap().hand.len(), 5);
        assert_eq!(game.player(A).unwrap().hand.len(), 0);
        // An ordinary resolving grant remains layer 6 and is not copied.
        apply(
            &mut game,
            second,
            Effect::new(ApplyContinuousEffect::new(
                EffectTarget::Specific(second),
                Modification::AddAbility(ironsmith::static_abilities::StaticAbility::flying()),
                Until::EndOfTurn,
            )),
        );
        assert!(game.current_has_static_ability_id(
            second,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
        let third = game.create_object_from_definition(
            &vanilla("Third copy", "{1}", "Human", 1, 1),
            A,
            Zone::Battlefield,
        );
        apply(
            &mut game,
            third,
            Effect::new(ApplyContinuousEffect::new_runtime(
                EffectTarget::Specific(third),
                RuntimeModification::CopyOf {
                    source: ChooseSpec::SpecificObject(second),
                    preserve_source_abilities: false,
                    name_override: None,
                    name_override_surface: None,
                    add_supertypes: vec![],
                    copy_exception_surface: None,
                },
                Until::EndOfTurn,
            )),
        );
        assert!(!game.current_has_static_ability_id(
            third,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
        assert_eq!(
            game.current_abilities(third)
                .unwrap()
                .iter()
                .filter(|ability| matches!(
                    ability.kind,
                    ironsmith::ability::AbilityKind::Triggered(_)
                ))
                .count(),
            1
        );
    }
}

#[test]
fn copy_exception_is_an_owned_ability_payload_and_not_an_ordinary_grant() {
    for definition in definitions("Curie, Emergent Intelligence") {
        fn walk(effect: &Effect, count: &mut usize) {
            if let Some(apply) = effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>()
            {
                for modification in &apply.runtime_modifications {
                    if let ironsmith::effects::RuntimeModification::CopyOfWithAbilities {
                        abilities,
                        ..
                    } = modification
                    {
                        assert_eq!(abilities.len(), 1);
                        *count += 1;
                        assert!(!apply.additional_modifications.iter().any(
                            |modification| matches!(
                                modification,
                                ironsmith::continuous::Modification::AddAbility(_)
                                    | ironsmith::continuous::Modification::AddAbilityGeneric(_)
                            )
                        ));
                    }
                }
            }
            effect.visit_child_effects(&mut |child| walk(child, count));
        }
        let ironsmith::ability::AbilityKind::Activated(ability) =
            &definition.abilities[activated(&definition)].kind
        else {
            panic!()
        };
        let mut count = 0;
        for segment in &ability.effects.segments {
            for effect in &segment.default_effects {
                walk(effect, &mut count);
            }
        }
        assert_eq!(count, 1);
    }
}
