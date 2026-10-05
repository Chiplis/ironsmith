//! UNVALIDATED full-card spell timing and blocked-status scenarios. No execution under deferred workflow.
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
        "../../../fixtures/combat_blocked_status.json.fixture"
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
            let minimum: usize = context
                .requirements
                .iter()
                .map(|requirement| requirement.min_targets)
                .sum();
            let maximum: usize = context
                .requirements
                .iter()
                .map(|requirement| requirement.max_targets.unwrap_or(self.targets.len()))
                .sum();
            assert!((minimum..=maximum).contains(&self.targets.len()));
            assert!(self.targets.iter().all(|target| {
                context
                    .requirements
                    .iter()
                    .any(|requirement| requirement.legal_targets.contains(target))
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
    for trigger in ironsmith::triggers::check_delayed_triggers(game, &event) {
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
fn attacker(game: &mut GameState, trample: bool) -> ObjectId {
    let definition = compile_to_runtime_definition(
        "Attacker",
        format!(
            "Type: Creature — Human\nPower/Toughness: 3/3{}",
            if trample { "\nTrample" } else { "" }
        ),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, B, Zone::Battlefield)
}
fn combat(game: &mut GameState, ids: &[ObjectId], complete: bool) {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    game.turn.active_player = B;
    game.turn.priority_player = Some(A);
    let mut state = CombatState::default();
    state.block_declaration_complete = complete;
    for id in ids {
        state.attackers.push(AttackerInfo {
            creature: *id,
            target: AttackTarget::Player(A),
        });
    }
    game.combat = Some(state);
}
fn blocked(game: &GameState, id: ObjectId) -> bool {
    ironsmith::combat_state::is_blocked(game.combat.as_ref().unwrap(), id)
}
fn library(game: &mut GameState, player: PlayerId) {
    for _ in 0..4 {
        game.create_object_from_definition(
            &vanilla("Draw resource", "{1}", "Human", 1, 1),
            player,
            Zone::Library,
        );
    }
}
#[test]
fn five_full_card_payloads_round_trip_without_loss() {
    let rows = fixtures()
        .into_iter()
        .filter(|row| row["proposed_complete"] == true)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 5);
    for row in rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn declared_blockers_timing_is_enforced_on_actual_costed_spells_after_artifact_restore() {
    for name in ["Choking Vines", "Dazzling Beauty", "Fog Patch"] {
        for definition in definitions(name) {
            let mut game = game();
            let id = attacker(&mut game, false);
            combat(&mut game, &[id], false);
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let action = LegalAction::CastSpell {
                spell_id: spell,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            };
            assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
            game.combat.as_mut().unwrap().block_declaration_complete = true;
            assert!(compute_legal_actions(&game, A).unwrap().contains(&action));
            game.turn.step = Some(ironsmith::game_state::Step::CombatDamage);
            assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
        }
    }
}
#[test]
fn choking_vines_pays_x_and_damages_even_the_selected_already_blocked_attacker() {
    for definition in definitions("Choking Vines") {
        let mut game = game();
        let first = attacker(&mut game, false);
        let already = attacker(&mut game, false);
        combat(&mut game, &[first, already], true);
        game.combat
            .as_mut()
            .unwrap()
            .blocked_attackers
            .insert(already);
        let before = game.player(A).unwrap().mana_pool.total();
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices {
                x: 2,
                targets: vec![Target::Object(first), Target::Object(already)],
                ..Default::default()
            },
        );
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 3);
        resolve_all(&mut game, &mut Choices::default());
        assert!(blocked(&game, first) && blocked(&game, already));
        assert_eq!(game.damage_on(first), 1);
        assert_eq!(game.damage_on(already), 1);
        assert!(game.combat.as_ref().unwrap().blockers.is_empty());
    }
}
#[test]
fn fog_patch_has_no_blockers_and_does_not_prevent_trample_damage() {
    for definition in definitions("Fog Patch") {
        let mut game = game();
        let ordinary = attacker(&mut game, false);
        let trampler = attacker(&mut game, true);
        combat(&mut game, &[ordinary, trampler], true);
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(blocked(&game, ordinary) && blocked(&game, trampler));
        assert!(game.combat.as_ref().unwrap().blockers.is_empty());
        assert!(game.effect_store.prevention_effects.shields().is_empty());
        let combat = game.combat.as_ref().unwrap().clone();
        ironsmith::execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(
            game.player(A).unwrap().life,
            17,
            "ordinary blocked attacker assigns no player damage; trample still assigns all three"
        );
    }
}
#[test]
fn curtain_draws_now_but_dazzling_draws_at_next_turns_upkeep() {
    for name in ["Curtain of Light", "Dazzling Beauty"] {
        for definition in definitions(name) {
            let mut game = game();
            library(&mut game, A);
            let id = attacker(&mut game, false);
            combat(&mut game, &[id], true);
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Object(id)],
                    ..Default::default()
                },
            );
            resolve_all(&mut game, &mut Choices::default());
            assert!(blocked(&game, id));
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                usize::from(name == "Curtain of Light")
            );
            if name == "Dazzling Beauty" {
                game.turn.turn_number += 1;
                game.turn.active_player = C;
                game.turn.phase = Phase::Beginning;
                game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
                event(
                    &mut game,
                    TriggerEvent::new_with_provenance(
                        ironsmith::events::BeginningOfUpkeepEvent::new(C),
                        Default::default(),
                    ),
                    &mut Choices::default(),
                );
                resolve_all(&mut game, &mut Choices::default());
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                assert!(game.player(C).unwrap().hand.is_empty());
            }
        }
    }
}
#[test]
fn trap_runner_pays_its_actual_tap_cost_and_old_blocked_status_does_not_follow_a_blink() {
    for definition in definitions("Trap Runner") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(host);
        let id = attacker(&mut game, false);
        combat(&mut game, &[id], true);
        activate(
            &mut game,
            host,
            activated_at(&definition, 0),
            &mut Choices {
                targets: vec![Target::Object(id)],
                ..Default::default()
            },
        );
        assert!(game.is_tapped(host));
        resolve_all(&mut game, &mut Choices::default());
        assert!(blocked(&game, id));
        let exile = game.move_object_by_game_rule(id, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_game_rule(exile, Zone::Battlefield)
            .unwrap();
        assert_ne!(id, returned);
        assert!(!blocked(&game, returned));
    }
}
#[test]
fn zero_x_choking_vines_is_a_legal_empty_instruction_not_a_target_error() {
    for definition in definitions("Choking Vines") {
        let mut game = game();
        combat(&mut game, &[], true);
        let before = game.player(A).unwrap().mana_pool.total();
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices {
                x: 0,
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 1);
        assert!(game.combat.as_ref().unwrap().blocked_attackers.is_empty());
    }
}
