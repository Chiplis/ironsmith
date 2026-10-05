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
        "../../../fixtures/source_comparison_returns.json.fixture"
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
use ironsmith::object::CounterType;
fn pump(game: &mut GameState, id: ObjectId, amount: i32) {
    apply(
        game,
        id,
        Effect::pump(amount, 0, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
fn return_effect(
    def: &CardDefinition,
) -> ironsmith::effects::ReturnFromGraveyardToBattlefieldEffect {
    fn visit(
        effect: &Effect,
        found: &mut Vec<ironsmith::effects::ReturnFromGraveyardToBattlefieldEffect>,
    ) {
        if let Some(effect) =
            effect.downcast_ref::<ironsmith::effects::ReturnFromGraveyardToBattlefieldEffect>()
        {
            found.push(effect.clone());
        }
        effect.visit_child_effects(&mut |child| visit(child, found));
    }
    let mut found = Vec::new();
    for ability in &def.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(ability) = &ability.kind {
            for segment in &ability.effects.segments {
                for effect in &segment.default_effects {
                    visit(effect, &mut found);
                }
            }
        }
    }
    assert_eq!(found.len(), 1);
    found.remove(0)
}
fn allowed(game: &GameState, source: ObjectId, spec: &ChooseSpec, candidate: ObjectId) -> bool {
    let mut dm = SelectFirstDecisionMaker;
    let ctx = EffectContext::new(source, A, &mut dm);
    ironsmith::targeting::compute_legal_targets_with_execution_context(game, spec, &ctx)
        .contains(&Target::Object(candidate))
}
fn end_step(game: &mut GameState, player: PlayerId, dm: &mut Choices) -> usize {
    game.turn.active_player = player;
    game.turn.phase = Phase::Ending;
    queue_event(
        game,
        TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfEndStepEvent::new(player),
            Default::default(),
        ),
        dm,
    )
}
fn attack(game: &mut GameState, attacker: ObjectId, dm: &mut Choices) {
    game.remove_summoning_sickness(attacker);
    game.turn.active_player = A;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[ironsmith::decision::AttackerDeclaration {
            creature: attacker,
            target: ironsmith::combat_state::AttackTarget::Player(B),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn surveil(game: &mut GameState, source: ObjectId) {
    game.create_object_from_definition(
        &vanilla("Surveilled card", "{1}", "Human", 1, 1),
        A,
        Zone::Library,
    );
    let outcome = apply(game, source, Effect::surveil(1));
    queue_outcome(game, outcome, &mut Choices::default());
    assert_eq!(game.stack.len(), 1);
    resolve_all(game, &mut Choices::default());
}
#[test]
fn source_power_bounds_and_entry_counters_survive_direct_and_restored_artifacts() {
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            let returned = return_effect(&definition);
            let ChooseSpec::Object(filter) = returned.target.base() else {
                panic!("{returned:?}")
            };
            assert_eq!(filter.zone, Some(Zone::Graveyard));
            assert_eq!(filter.owner, Some(PlayerFilter::You));
            let mirko = name.starts_with("Mirko");
            let rhs = match if mirko {
                filter.power.as_ref()
            } else {
                filter.mana_value.as_ref()
            } {
                Some(ironsmith::filter::Comparison::LessThanExpr(rhs)) if mirko => rhs,
                Some(ironsmith::filter::Comparison::LessThanOrEqualExpr(rhs)) if !mirko => rhs,
                other => panic!("{other:?}"),
            };
            let ironsmith::effect::Value::PowerOf(spec) = rhs.unhinted() else {
                panic!("{rhs:?}")
            };
            assert!(matches!(spec.base(), ChooseSpec::Source));
            let [counter] = returned.enters_with_counters.as_slice() else {
                panic!("{returned:?}")
            };
            assert_eq!(
                counter.counter_type,
                if mirko {
                    CounterType::Finality
                } else {
                    CounterType::PlusOnePlusOne
                }
            );
            if mirko {
                assert!(counter.object_filter.is_none());
            } else {
                assert_eq!(
                    counter.object_filter.as_ref().unwrap().subtypes,
                    [ironsmith::Subtype::Hero]
                );
                assert_eq!(
                    counter.surface,
                    ironsmith_core::BattlefieldEntryCounterSurface::IfObjectEntersThisWay
                );
            }
        }
    }
}
#[test]
fn mirko_surveillance_power_is_live_at_announcement_and_resolution() {
    for definition in definitions("Mirko, Obsessive Theorist") {
        for shrink in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let target = game.create_object_from_definition(
                &vanilla("Reanimation subject", "{1}", "Human", 1, 2),
                A,
                Zone::Graveyard,
            );
            let equal = game.create_object_from_definition(
                &vanilla("Equal power", "{1}", "Human", 2, 2),
                A,
                Zone::Graveyard,
            );
            let spec = return_effect(&definition).target;
            assert!(!allowed(&game, source, &spec, target));
            surveil(&mut game, source);
            assert_eq!(game.calculated_power(source), Some(2));
            assert!(allowed(&game, source, &spec, target));
            assert!(!allowed(&game, source, &spec, equal));
            assert_eq!(end_step(&mut game, B, &mut Choices::default()), 0);
            let stable = game.object(target).unwrap().stable_id;
            assert_eq!(
                end_step(
                    &mut game,
                    A,
                    &mut Choices {
                        targets: vec![Target::Object(target)],
                        ..Default::default()
                    }
                ),
                1
            );
            if shrink {
                pump(&mut game, source, -1);
            } else {
                game.set_current_controller(source, B).unwrap();
            }
            resolve_all(&mut game, &mut Choices::default());
            let returned = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(returned).unwrap().zone,
                if shrink {
                    Zone::Graveyard
                } else {
                    Zone::Battlefield
                }
            );
            if !shrink {
                assert_eq!(game.current_controller(returned), Some(A));
                assert_eq!(
                    game.object(returned)
                        .unwrap()
                        .counters
                        .get(&CounterType::Finality),
                    Some(&1)
                );
                apply(
                    &mut game,
                    returned,
                    Effect::destroy(ChooseSpec::SpecificObject(returned)),
                );
                assert_eq!(
                    game.object(game.find_object_by_stable_id(stable).unwrap())
                        .unwrap()
                        .zone,
                    Zone::Exile
                );
            }
        }
    }
}
#[test]
fn source_comparison_keeps_exact_departure_power_after_source_blinks() {
    for definition in definitions("Mirko, Obsessive Theorist") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        surveil(&mut game, source);
        let target = game.create_object_from_definition(
            &vanilla("Departure target", "{1}", "Human", 1, 2),
            A,
            Zone::Graveyard,
        );
        let stable = game.object(target).unwrap().stable_id;
        end_step(
            &mut game,
            A,
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        pump(&mut game, source, 3);
        let exiled = game.move_object_by_game_rule(source, Zone::Exile).unwrap();
        let new_source = game
            .move_object_by_game_rule(exiled, Zone::Battlefield)
            .unwrap();
        pump(&mut game, new_source, -2);
        assert_eq!(
            game.stack[0].source_snapshot.as_ref().unwrap().power,
            Some(5)
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Battlefield
        );
    }
}
#[test]
fn winter_soldier_uses_current_power_for_target_mana_value_and_hero_entry_counters() {
    for definition in definitions("Winter Soldier, Reborn Avenger") {
        for hero in [false, true] {
            for shrink in [false, true] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let witness=compile_to_runtime_definition("Entry size witness","Type: Enchantment\nWhenever a creature with power 3 or greater enters, you gain 1 life.",false).unwrap();
                game.create_object_from_definition(&witness, A, Zone::Battlefield);
                let card=compile_to_runtime_definition("Entering subject",format!("Mana cost: {{3}}\nType: Creature — {}\nPower/Toughness: 1/2\nThis creature enters with a +1/+1 counter on it.",if hero {"Hero"} else {"Human"}),false).unwrap();
                let target = game.create_object_from_definition(&card, A, Zone::Graveyard);
                let stable = game.object(target).unwrap().stable_id;
                let too_large = game.create_object_from_definition(
                    &vanilla("Too costly", "{4}", "Hero", 1, 2),
                    A,
                    Zone::Graveyard,
                );
                let spec = return_effect(&definition).target;
                assert!(allowed(&game, source, &spec, target));
                assert!(!allowed(&game, source, &spec, too_large));
                attack(
                    &mut game,
                    source,
                    &mut Choices {
                        targets: vec![Target::Object(target)],
                        ..Default::default()
                    },
                );
                assert_eq!(game.stack.len(), 1);
                if shrink {
                    pump(&mut game, source, -1);
                } else {
                    game.set_current_controller(source, B).unwrap();
                }
                resolve(&mut game, &mut Choices::default());
                put_triggers_on_stack_with_dm(
                    &mut game,
                    &mut TriggerQueue::new(),
                    &mut Choices::default(),
                )
                .unwrap();
                resolve_all(&mut game, &mut Choices::default());
                let returned = game.find_object_by_stable_id(stable).unwrap();
                let object = game.object(returned).unwrap();
                assert_eq!(
                    object.zone,
                    if shrink {
                        Zone::Graveyard
                    } else {
                        Zone::Battlefield
                    }
                );
                if !shrink {
                    assert_eq!(game.current_controller(returned), Some(A));
                    assert_eq!(
                        object.counters.get(&CounterType::PlusOnePlusOne),
                        Some(&(1 + u32::from(hero)))
                    );
                }
                assert_eq!(
                    game.player(A).unwrap().life,
                    20 + i32::from(hero && !shrink)
                );
            }
        }
    }
}

#[test]
fn mirko_may_return_can_be_declined_after_a_legal_target_was_announced() {
    struct Decline;
    impl DecisionMaker for Decline {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }
    for definition in definitions("Mirko, Obsessive Theorist") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        surveil(&mut game, source);
        let target = game.create_object_from_definition(
            &vanilla("Optional return", "{1}", "Human", 1, 2),
            A,
            Zone::Graveyard,
        );
        end_step(
            &mut game,
            A,
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        resolve_stack_entry_with(&mut game, &mut Decline).unwrap();
        assert_eq!(game.object(target).unwrap().zone, Zone::Graveyard);
        assert!(game.object(target).unwrap().counters.is_empty());
    }
}
