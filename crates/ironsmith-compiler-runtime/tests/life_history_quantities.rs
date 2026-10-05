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
        "../../../fixtures/life_history_quantities.json.fixture"
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
}
impl DecisionMaker for Choices {
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
            && (context.description.starts_with("Choose mode for")
                || context.description == "Choose a mode")
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
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
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
    let mut state = PriorityLoopState::new(2);
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
    let mut state = PriorityLoopState::new(2);
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

fn end_step(game: &mut GameState, active: PlayerId, dm: &mut Choices) -> usize {
    game.turn.active_player = active;
    game.turn.phase = Phase::Ending;
    queue_event(
        game,
        TriggerEvent::new_with_provenance(
            ironsmith::events::phase::BeginningOfEndStepEvent::new(active),
            Default::default(),
        ),
        dm,
    )
}

fn history(game: &mut GameState, source: ObjectId) {
    // Alice: life lost 9, life gained 10, damage received 8, current life 21.
    // Bob: lost 4, gained 12, current life 28. Charlie: lost 2, current 18.
    apply(
        game,
        source,
        Effect::lose_life_player(3, PlayerFilter::Specific(A)),
    );
    apply(
        game,
        source,
        Effect::deal_damage(2, ChooseSpec::SpecificPlayer(A)),
    );
    apply(
        game,
        source,
        Effect::new(ironsmith::effects::PayLifeEffect::new(
            2,
            ChooseSpec::SpecificPlayer(A),
        )),
    );
    apply(
        game,
        source,
        Effect::gain_life_player(10, ChooseSpec::SpecificPlayer(A)),
    );
    apply(
        game,
        source,
        Effect::prevent_damage(3, ChooseSpec::SpecificPlayer(A), Until::EndOfTurn),
    );
    apply(
        game,
        source,
        Effect::deal_damage(5, ChooseSpec::SpecificPlayer(A)),
    );
    let infect = compile_to_runtime_definition(
        "Infect damage source",
        "Mana cost: {1}\nType: Creature — Horror\nPower/Toughness: 1/1\nInfect",
        false,
    )
    .unwrap();
    let infect = game.create_object_from_definition(&infect, B, Zone::Battlefield);
    apply(
        game,
        infect,
        Effect::deal_damage(4, ChooseSpec::SpecificPlayer(A)),
    );
    apply(
        game,
        source,
        Effect::lose_life_player(4, PlayerFilter::Specific(B)),
    );
    apply(
        game,
        source,
        Effect::gain_life_player(12, ChooseSpec::SpecificPlayer(B)),
    );
    apply(
        game,
        source,
        Effect::lose_life_player(2, PlayerFilter::Specific(C)),
    );
    assert_eq!(game.player(A).unwrap().life, 21);
    assert_eq!(game.player(B).unwrap().life, 28);
    assert_eq!(game.player(C).unwrap().life, 18);
}

#[test]
fn six_full_history_cards_compile_without_metadata_loss_and_round_trip() {
    assert_eq!(fixtures().len(), 6);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert!(!format!("{definition:?}").contains("PendingComparison"));
        }
    }
}

#[test]
fn children_of_korlis_counts_actual_life_lost_even_after_gains_and_resets_next_turn() {
    for definition in definitions("Children of Korlis") {
        for reset in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            history(&mut game, source);
            if reset {
                game.next_turn();
                game.turn.priority_player = Some(A);
            }
            let mut dm = Choices::default();
            activate(&mut game, source, activated(&definition), &mut dm);
            assert!(game.object(source).is_none());
            // A response adds gain, without changing the historical loss.
            apply(
                &mut game,
                source,
                Effect::gain_life_player(5, ChooseSpec::SpecificPlayer(A)),
            );
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().life, if reset { 26 } else { 35 });
            assert_eq!(game.player(B).unwrap().life, 28);
        }
    }
}

#[test]
fn simulacrum_uses_damage_received_including_infect_excluding_prevention_and_payments() {
    for definition in definitions("Simulacrum") {
        let mut game = game();
        let victim = game.create_object_from_definition(
            &vanilla("Damage sink", "{2}", "Human", 1, 30),
            A,
            Zone::Battlefield,
        );
        history(&mut game, victim);
        let mut dm = Choices {
            targets: vec![Target::Object(victim)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 29);
        assert_eq!(game.damage_on(victim), 8);
    }
}

#[test]
fn astarion_modal_quantities_bind_the_announced_player_and_resolve_after_responses() {
    for definition in definitions("Astarion, the Decadent") {
        for mode in [0, 1] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            // Use another source for the history so Astarion's lifelink is irrelevant.
            let history_source = game.create_object_from_definition(
                &vanilla("History source", "{1}", "Human", 1, 1),
                C,
                Zone::Battlefield,
            );
            history(&mut game, history_source);
            let mut dm = Choices {
                mode: Some(mode),
                targets: if mode == 0 {
                    vec![Target::Player(B)]
                } else {
                    vec![]
                },
                ..Default::default()
            };
            assert_eq!(end_step(&mut game, A, &mut dm), 1);
            if mode == 0 {
                apply(
                    &mut game,
                    source,
                    Effect::lose_life_player(1, PlayerFilter::Specific(B)),
                );
            } else {
                apply(
                    &mut game,
                    source,
                    Effect::gain_life_player(3, ChooseSpec::SpecificPlayer(A)),
                );
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.player(A).unwrap().life,
                if mode == 0 { 21 } else { 37 }
            );
            assert_eq!(
                game.player(B).unwrap().life,
                if mode == 0 { 22 } else { 28 }
            );
            assert_eq!(game.player(C).unwrap().life, 18);
        }
    }
}

#[test]
fn wound_reflection_uses_each_opponents_own_total_and_the_ability_controller() {
    for definition in definitions("Wound Reflection") {
        for controller in [A, B] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            history(&mut game, source);
            game.set_current_controller(source, controller).unwrap();
            let mut dm = Choices::default();
            assert_eq!(end_step(&mut game, C, &mut dm), 1);
            game.set_current_controller(source, C).unwrap();
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.player(A).unwrap().life,
                if controller == A { 21 } else { 12 }
            );
            assert_eq!(
                game.player(B).unwrap().life,
                if controller == B { 28 } else { 24 }
            );
            assert_eq!(game.player(C).unwrap().life, 16);
            game.next_turn();
            let before = [A, B, C].map(|p| game.player(p).unwrap().life);
            assert_eq!(end_step(&mut game, A, &mut dm), 1);
            resolve_all(&mut game, &mut dm);
            assert_eq!([A, B, C].map(|p| game.player(p).unwrap().life), before);
        }
    }
}

#[test]
fn warlock_class_reaches_level_three_through_paid_activations_before_using_turn_totals() {
    for definition in definitions("Warlock Class") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..4 {
            game.create_object_from_definition(
                &vanilla("Class look card", "{1}", "Human", 1, 1),
                A,
                Zone::Library,
            );
        }
        let mut dm = Choices::default();
        for level in [2, 3] {
            let ability = compute_legal_actions(&game, A)
                .unwrap()
                .into_iter()
                .find_map(|action| match action {
                    LegalAction::ActivateAbility {
                        source: id,
                        ability_index,
                    } if id == source => Some(ability_index),
                    _ => None,
                })
                .expect("next Class level should be payable");
            activate(&mut game, source, ability, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.class_level(source), level);
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        history(&mut game, source);
        assert_eq!(end_step(&mut game, A, &mut dm), 1);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(game.player(B).unwrap().life, 24);
        assert_eq!(game.player(C).unwrap().life, 16);
    }
}

#[test]
fn cellarspawn_difference_uses_paid_mana_and_cast_characteristics_after_the_spell_leaves() {
    for definition in definitions("Ancient Cellarspawn") {
        for variant in [0, 1, 2, 3] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let text = match variant {
                0 => "Mana cost: {4}{B}\nType: Creature — Demon\nPower/Toughness: 3/3",
                1 => "Mana cost: {X}{B}\nType: Creature — Demon\nPower/Toughness: 3/3",
                2 => {
                    "Mana cost: {5}\nType: Artifact Creature — Demon\nPower/Toughness: 5/5\nPrototype {1}{B} — 2/3"
                }
                _ => "Mana cost: {5}\nType: Creature — Human\nPower/Toughness: 3/3",
            };
            let spell_def =
                compile_to_runtime_definition("Compared cast spell", text, false).unwrap();
            let method = if variant == 2 {
                CastingMethod::Alternative(
                    spell_def
                        .alternative_casts
                        .iter()
                        .position(|a| a.name().eq_ignore_ascii_case("prototype"))
                        .unwrap(),
                )
            } else {
                CastingMethod::Normal
            };
            let mut dm = Choices {
                x: 3,
                targets: if variant == 3 {
                    vec![]
                } else {
                    vec![Target::Player(B)]
                },
                ..Default::default()
            };
            let spell = cast(&mut game, &spell_def, method, &mut dm);
            assert_eq!(game.stack.len(), if variant == 3 { 1 } else { 2 });
            apply(
                &mut game,
                source,
                Effect::counter(ChooseSpec::SpecificObject(spell)),
            );
            game.move_object_by_game_rule(source, Zone::Graveyard)
                .unwrap();
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.player(B).unwrap().life,
                if variant == 3 { 20 } else { 19 }
            );
            assert_eq!(game.player(C).unwrap().life, 20);
        }
    }
}
