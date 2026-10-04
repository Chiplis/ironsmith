//! UNVALIDATED extrema values, tie scopes and resolution-time choice regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/extrema_quantities.json.fixture"
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
fn game_with_starting_life(starting: i32) -> GameState {
    let mut game = GameState::new(
        vec!["Alice".into(), "Bob".into(), "Charlie".into()],
        starting,
    );
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
    optional: Option<bool>,
    kicker_payments: usize,
    decline_targets: bool,
    bounds: Vec<(usize, Option<usize>)>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        if let Some(choice) = self.optional {
            return choice;
        }
        let _ = ctx;
        true
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose optional costs for") {
            return context
                .options
                .iter()
                .filter(|option| option.legal)
                .take(self.kicker_payments)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.bounds.extend(
            context
                .requirements
                .iter()
                .map(|r| (r.min_targets, r.max_targets)),
        );
        if self.decline_targets {
            return Vec::new();
        }
        if !self.targets.is_empty() {
            assert!(self.targets.iter().all(|target| {
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            }));
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
            let selected = self
                .objects
                .iter()
                .copied()
                .filter(|id| {
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                })
                .take(context.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>();
            assert!(
                selected.len() >= context.min,
                "requested cost objects must satisfy the current payment decision"
            );
            selected
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
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
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
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
        if state.pending_cast.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none());
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
use ironsmith::static_abilities::StaticAbilityId;
fn game() -> GameState {
    game_with_starting_life(20)
}
fn witness(game: &mut GameState) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("History witness", "{1}", "Human", 1, 1),
        A,
        Zone::Battlefield,
    )
}
fn library(game: &mut GameState, count: usize) {
    for _ in 0..count {
        game.create_object_from_definition(
            &vanilla("Library witness", "{1}", "Human", 1, 1),
            A,
            Zone::Library,
        );
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = cast(game, definition, CastingMethod::Normal, dm);
    let stable = game.object(spell).unwrap().stable_id;
    resolve_all(game, dm);
    game.find_object_by_stable_id(stable).unwrap()
}
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.calculated_power(id).unwrap(),
        game.calculated_toughness(id).unwrap(),
    )
}
fn has(game: &GameState, id: ObjectId, ability: StaticAbilityId) -> bool {
    game.current_has_static_ability_id(id, ability)
}

fn creature(game: &mut GameState, player: PlayerId, name: &str, p: i32, t: i32) -> ObjectId {
    game.create_object_from_definition(
        &vanilla(name, "{1}", "Human", p, t),
        player,
        Zone::Battlefield,
    )
}
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}

fn attack(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    game.remove_summoning_sickness(source);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[ironsmith::decision::AttackerDeclaration {
            creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(B),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
#[test]
fn first_three_quantity_candidates_keep_full_metadata_and_artifact_semantics() {
    for name in [
        "Freelance Muscle",
        "Investigator's Journal",
        "Repay in Kind",
    ] {
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
        }
    }
}
#[test]
fn freelance_muscle_uses_the_greatest_current_axis_of_other_controlled_creatures_at_resolution() {
    for definition in definitions("Freelance Muscle") {
        for other in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let base = pt(&game, source);
            let yours = creature(&mut game, A, "Other axes", 2, 7);
            let theirs = creature(&mut game, B, "Enemy axes", 15, 16);
            if !other {
                game.set_current_controller(yours, B).unwrap();
            }
            attack(&mut game, source, &mut Choices::default());
            if other {
                // A current-characteristic change after triggering is read
                // on resolution; a greater opposing axis must not enter scope.
                apply(
                    &mut game,
                    theirs,
                    Effect::pump(7, 0, ChooseSpec::SpecificObject(yours), Until::EndOfTurn),
                );
            }
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                pt(&game, source),
                (
                    base.0 + if other { 9 } else { 0 },
                    base.1 + if other { 9 } else { 0 }
                )
            );
            assert_eq!(pt(&game, theirs), (15, 16));
        }
    }
}
#[test]
fn investigators_journal_counts_the_largest_single_players_population_and_both_draw_costs_execute()
{
    for definition in definitions("Investigator's Journal") {
        let mut game = game();
        library(&mut game, 4);
        creature(&mut game, A, "Alice creature", 1, 1);
        for _ in 0..3 {
            creature(&mut game, B, "Bob creature", 1, 1);
        }
        for _ in 0..2 {
            creature(&mut game, C, "Charlie creature", 1, 1);
        }
        let journal = enter(&mut game, &definition, &mut Choices::default());
        let suspect = ironsmith::object::CounterType::Named("suspect".into());
        assert_eq!(
            game.counter_count(journal, suspect.clone()),
            3,
            "greatest single-player count, not six creatures globally"
        );
        activate(
            &mut game,
            journal,
            activated(&definition),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.counter_count(journal, suspect), 2);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        let sacrifice_index = definition
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, ability)| {
                matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_))
            })
            .nth(1)
            .unwrap()
            .0;
        activate(&mut game, journal, sacrifice_index, &mut Choices::default());
        resolve_all(&mut game, &mut Choices::default());
        assert!(game.object(journal).is_none());
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
    }
}
#[test]
fn repay_in_kind_freezes_the_original_minimum_before_any_life_change_replacement() {
    for definition in definitions("Repay in Kind") {
        let mut game = game();
        let source = witness(&mut game);
        apply(
            &mut game,
            source,
            Effect::set_life_total_player(8, PlayerFilter::Specific(B)),
        );
        apply(
            &mut game,
            source,
            Effect::set_life_total_player(12, PlayerFilter::Specific(C)),
        );
        // A real replacement transforms Alice's loss to 24, leaving a new
        // minimum -4. Bob and Charlie must still use the original minimum 8.
        game.effect_store.replacement_effects.add_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                A,
                ironsmith::events::life::matchers::WouldLoseLifeMatcher::you(),
                ironsmith::replacement::ReplacementAction::Double,
            ),
        );
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, -4);
        assert_eq!(game.player(B).unwrap().life, 8);
        assert_eq!(game.player(C).unwrap().life, 8);
    }
}
