//! UNVALIDATED full-card live static conditions; all scenarios are authored, unrun.
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
        "../../../fixtures/live_static_condition_scopes.json.fixture"
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
    land_choice: Option<&'static str>,
    land_prompts: usize,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description == "Choose a basic land type" {
            self.land_prompts += 1;
            return vec![
                ctx.options
                    .iter()
                    .find(|option| option.description == self.land_choice.unwrap_or("Island"))
                    .unwrap()
                    .index,
            ];
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
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(action, LegalAction::ActivateAbility { source: id, ability_index: index }
            | LegalAction::ActivateManaAbility { source: id, ability_index: index }
            if *id == source && *index == ability_index)
        })
        .expect("the current ability must be legally activatable");
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

fn attach(game: &mut GameState, aura: ObjectId, host: ObjectId) {
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
}
#[test]
fn four_complete_frozen_bodies_are_strict_and_round_trip() {
    let rows = fixtures()
        .into_iter()
        .filter(|r| r["proposed_complete"] == true)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 4);
    for row in rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn attached_basic_mountain_is_not_any_mountain_or_a_goblin_recipient() {
    for name in ["Goblin Caves", "Goblin Shrine"] {
        for definition in definitions(name) {
            let mut game = game();
            let basic = game.create_object_from_definition(
                &resource("Basic host", "Basic Land — Mountain"),
                B,
                Zone::Battlefield,
            );
            let nonbasic = game.create_object_from_definition(
                &resource("Nonbasic host", "Land — Mountain"),
                B,
                Zone::Battlefield,
            );
            let island = game.create_object_from_definition(
                &resource("Wrong subtype", "Basic Land — Island"),
                B,
                Zone::Battlefield,
            );
            let yours = game.create_object_from_definition(
                &vanilla("Your Goblin", "{R}", "Goblin", 2, 2),
                A,
                Zone::Battlefield,
            );
            let theirs = game.create_object_from_definition(
                &vanilla("Their Goblin", "{R}", "Goblin", 2, 2),
                B,
                Zone::Battlefield,
            );
            let elf = game.create_object_from_definition(
                &vanilla("Not Goblin", "{G}", "Elf", 2, 2),
                A,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Object(nonbasic)],
                ..Default::default()
            };
            let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve(&mut game, &mut dm);
            let aura = game
                .battlefield
                .iter()
                .copied()
                .find(|id| game.object(*id).unwrap().name == name)
                .unwrap();
            assert!(game.object(spell).is_none());
            assert_eq!(pt(&game, yours), (2, 2));
            attach(&mut game, aura, basic);
            let expected = if name == "Goblin Caves" {
                (2, 4)
            } else {
                (3, 2)
            };
            assert_eq!(pt(&game, yours), expected);
            assert_eq!(pt(&game, theirs), expected);
            assert_eq!(pt(&game, elf), (2, 2));
            game.phase_out(aura);
            assert_eq!(pt(&game, yours), (2, 2));
            game.phase_in(aura);
            assert_eq!(pt(&game, yours), expected);
            attach(&mut game, aura, island);
            assert_eq!(pt(&game, yours), (2, 2));
            attach(&mut game, aura, basic);
            assert_eq!(pt(&game, theirs), expected);
            if name == "Goblin Shrine" {
                let outcome = apply(
                    &mut game,
                    aura,
                    Effect::destroy(ChooseSpec::SpecificObject(aura)),
                );
                queue_outcome(&mut game, outcome, &mut Choices::default());
                resolve_all(&mut game, &mut Choices::default());
                assert_eq!(pt(&game, yours), (2, 2));
                assert_eq!(game.damage_on(yours), 1);
                assert_eq!(game.damage_on(theirs), 1);
                assert_eq!(game.damage_on(elf), 0);
            }
        }
    }
}
#[test]
fn modified_source_grants_a_real_second_mana_ability_and_rechecks_aura_controller() {
    for definition in definitions("Orochi Merge-Keeper") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let count = |g: &GameState| {
            g.current_abilities(source)
                .unwrap()
                .iter()
                .filter(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
                .count()
        };
        assert_eq!(count(&game), 1);
        let enemy_aura = game.create_object_from_definition(
            &resource("Foreign Aura", "Enchantment — Aura"),
            B,
            Zone::Battlefield,
        );
        attach(&mut game, enemy_aura, source);
        assert_eq!(count(&game), 1, "enemy-controlled Aura does not modify");
        let own_aura = game.create_object_from_definition(
            &resource("Own Aura", "Enchantment — Aura"),
            A,
            Zone::Battlefield,
        );
        attach(&mut game, own_aura, source);
        assert_eq!(count(&game), 2);
        game.player_mut(A).unwrap().mana_pool.empty();
        let index = current_activated_at(&game, source, 1);
        activate(&mut game, source, index, &mut Choices::default());
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
        game.phase_out(own_aura);
        assert_eq!(count(&game), 1);
        counter(&mut game, source, source, 1);
        assert_eq!(count(&game), 2, "a counter independently modifies");
    }
}
#[test]
fn poisoned_opponent_is_existential_and_excludes_teammates() {
    use ironsmith::object::CounterType;
    use ironsmith::static_abilities::StaticAbilityId as Id;
    for definition in definitions("Viridian Betrayers") {
        let d = PlayerId::from_index(3);
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Bob".into(),
                "Charlie".into(),
                "Dana".into(),
            ],
            20,
        );
        game.enable_team_vs_team(vec![vec![A, B], vec![C, d]])
            .unwrap();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(!has(&game, source, Id::Infect));
        game.add_player_counters_with_source(B, CounterType::Poison, 1, Some(source), Some(A))
            .unwrap();
        assert!(
            !has(&game, source, Id::Infect),
            "teammate is not an opponent"
        );
        game.add_player_counters_with_source(C, CounterType::Poison, 1, Some(source), Some(A))
            .unwrap();
        assert!(
            has(&game, source, Id::Infect),
            "one of two opponents suffices"
        );
        let life = game.player(d).unwrap().life;
        apply(
            &mut game,
            source,
            Effect::deal_damage(2, ChooseSpec::Player(PlayerFilter::Specific(d))),
        );
        assert_eq!(game.player(d).unwrap().poison_counters, 2);
        assert_eq!(game.player(d).unwrap().life, life);
        game.remove_player_counters_with_source(C, CounterType::Poison, 1, Some(source), Some(A));
        assert!(has(&game, source, Id::Infect));
        game.remove_player_counters_with_source(d, CounterType::Poison, 2, Some(source), Some(A));
        assert!(!has(&game, source, Id::Infect));
    }
}
