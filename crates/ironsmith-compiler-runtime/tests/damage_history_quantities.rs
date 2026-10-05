//! UNVALIDATED exact current-turn damage receipts and full-card source regressions.
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
use ironsmith::target::ChooseSpec;
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
        "../../../fixtures/damage_history_quantities.json.fixture"
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
    distribution: Vec<(Target, u32)>,
    distribution_totals: Vec<u32>,
}
impl DecisionMaker for Choices {
    fn decide_distribute(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        self.distribution_totals.push(context.total);
        if self.distribution.is_empty() {
            SelectFirstDecisionMaker.decide_distribute(game, context)
        } else {
            self.distribution.clone()
        }
    }

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
fn hit(game: &mut GameState, source: ObjectId, target: Target, amount: i32, combat: bool) {
    let target = match target {
        Target::Object(id) => ChooseSpec::SpecificObject(id),
        Target::Player(player) => ChooseSpec::SpecificPlayer(player),
    };
    let mut damage = ironsmith::effects::DealDamageEffect::new(amount, target);
    damage.source_is_combat = combat;
    apply(game, source, Effect::new(damage));
}
fn end_step(game: &mut GameState) {
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(ironsmith::game_state::Step::End);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::BeginningOfEndStepEvent::new(A),
        Default::default(),
    );
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) {
        queue.add(entry);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, &mut Choices::default()).unwrap();
}
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}
#[test]
fn fourteen_exact_damage_history_candidates_round_trip_with_completed_occurrences() {
    let rows = fixtures();
    assert_eq!(rows.len(), 14);
    for row in &rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
        }
    }
}
#[test]
fn whipkeeper_uses_actual_received_damage_after_regeneration_and_live_response_damage() {
    for definition in definitions("Whipkeeper") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let dealer = creature(&mut game, B, "Damage dealer", 2, 40);
        let target = creature(&mut game, B, "Damage recipient", 2, 40);
        apply(
            &mut game,
            dealer,
            Effect::prevent_damage(3, ChooseSpec::SpecificObject(target), Until::EndOfTurn),
        );
        hit(&mut game, dealer, Target::Object(target), 5, false);
        apply(
            &mut game,
            dealer,
            Effect::regenerate(ChooseSpec::SpecificObject(target), Until::EndOfTurn),
        );
        apply(
            &mut game,
            dealer,
            Effect::destroy(ChooseSpec::SpecificObject(target)),
        );
        assert_eq!(game.damage_on(target), 0);
        game.remove_summoning_sickness(source);
        activate(
            &mut game,
            source,
            activated(&definition),
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        hit(&mut game, dealer, Target::Object(target), 3, false);
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(
            game.damage_on(target),
            8,
            "three response damage plus five actual damage this turn, including healed damage"
        );
    }
}
#[test]
fn zubera_death_conditions_use_actual_cumulative_damage_and_ignore_prevented_amounts() {
    for name in ["Burning-Eye Zubera", "Rushing-Tide Zubera"] {
        for definition in definitions(name) {
            for prevented in [0, 3] {
                let mut game = game();
                library(&mut game, 4);
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let dealer = creature(&mut game, B, "Damage dealer", 2, 40);
                if prevented > 0 {
                    apply(
                        &mut game,
                        dealer,
                        Effect::prevent_damage(
                            prevented,
                            ChooseSpec::SpecificObject(source),
                            Until::EndOfTurn,
                        ),
                    );
                }
                hit(&mut game, dealer, Target::Object(source), 6, false);
                let mut queue = TriggerQueue::new();
                let mut dm = Choices {
                    targets: vec![Target::Player(B)],
                    ..Default::default()
                };
                ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm)
                    .unwrap();
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                assert!(game.object(source).is_none());
                assert_eq!(game.stack.len(), usize::from(prevented == 0));
                resolve_all(&mut game, &mut dm);
                if name == "Burning-Eye Zubera" {
                    assert_eq!(
                        game.player(B).unwrap().life,
                        if prevented == 0 { 17 } else { 20 }
                    );
                } else {
                    assert_eq!(
                        game.player(A).unwrap().hand.len(),
                        if prevented == 0 { 3 } else { 0 }
                    );
                }
            }
        }
    }
}
#[test]
fn sold_out_reads_the_exiled_creatures_prior_incarnation_and_creates_only_the_real_clue() {
    for definition in definitions("Sold Out") {
        for damaged in [false, true] {
            let mut game = game();
            let dealer = witness(&mut game);
            let target = creature(&mut game, B, "Exiled damage recipient", 2, 40);
            if damaged {
                hit(&mut game, dealer, Target::Object(target), 1, false);
            }
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                },
            );
            resolve_all(&mut game, &mut Choices::default());
            assert!(game.object(target).is_none());
            assert_eq!(
                game.battlefield
                    .iter()
                    .filter(|id| game.current_has_subtype(**id, ironsmith::Subtype::Clue))
                    .count(),
                usize::from(damaged)
            );
        }
    }
}
#[test]
fn fallers_faithful_uses_destroyed_target_history_and_missing_optional_target_is_not_zero_damage() {
    for definition in definitions("Faller's Faithful") {
        for mode in 0..3 {
            let mut game = game();
            let dealer = witness(&mut game);
            for _ in 0..3 {
                game.create_object_from_definition(
                    &vanilla("Opponent library card", "{1}", "Human", 1, 1),
                    B,
                    Zone::Library,
                );
            }
            let target = creature(&mut game, B, "Destroyed recipient", 2, 40);
            if mode == 1 {
                hit(&mut game, dealer, Target::Object(target), 1, false);
            }
            let mut dm = Choices {
                targets: if mode == 2 {
                    Vec::new()
                } else {
                    vec![Target::Object(target)]
                },
                decline_targets: mode == 2,
                ..Default::default()
            };
            enter(&mut game, &definition, &mut dm);
            assert_eq!(game.object(target).is_some(), mode == 2);
            assert_eq!(
                game.player(B).unwrap().hand.len(),
                if mode == 0 { 2 } else { 0 }
            );
            assert!(game.player(A).unwrap().hand.is_empty());
        }
    }
}
#[test]
fn grisly_sigil_only_uses_noncombat_history_and_real_casualty_copies_recheck_the_same_target() {
    for definition in definitions("Grisly Sigil") {
        for mode in 0..3 {
            let mut game = game();
            let dealer = witness(&mut game);
            let target = creature(&mut game, B, "Sigil recipient", 2, 40);
            if mode < 2 {
                hit(&mut game, dealer, Target::Object(target), 1, mode == 0);
            }
            let mut dm = Choices {
                targets: vec![Target::Object(target)],
                optional: Some(mode == 2),
                kicker_payments: usize::from(mode == 2),
                objects: vec![dealer],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.damage_on(target),
                match mode {
                    0 => 2,
                    1 => 4,
                    _ => 4,
                }
            );
            assert_eq!(
                game.player(A).unwrap().life,
                match mode {
                    0 => 21,
                    1 => 23,
                    _ => 24,
                }
            );
            assert_eq!(
                game.object(dealer).is_none(),
                mode == 2,
                "the real casualty payment is not a compile-only marker"
            );
        }
    }
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
fn library_for(game: &mut GameState, player: PlayerId, count: usize) {
    for n in 0..count {
        game.create_object_from_definition(
            &vanilla(&format!("Library card {n}"), "{1}", "Human", 1, 1),
            player,
            Zone::Library,
        );
    }
}
#[test]
fn blazing_effigy_adds_three_to_other_named_source_damage_using_its_departed_identity() {
    for definition in definitions("Blazing Effigy") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let namesake = creature(&mut game, B, "Blazing Effigy", 2, 40);
        let wrong_name = creature(&mut game, B, "Wrong name", 2, 40);
        let recipient = namesake; // The announced target is still an eligible other named source.
        // A regenerated point from another name and from itself must not enter
        // the named-other-source sum; healing must not erase the eligible point.
        hit(&mut game, wrong_name, Target::Object(source), 1, false);
        hit(&mut game, source, Target::Object(source), 1, false);
        apply(
            &mut game,
            source,
            Effect::regenerate(ChooseSpec::Source, Until::EndOfTurn),
        );
        apply(
            &mut game,
            namesake,
            Effect::destroy(ChooseSpec::SpecificObject(source)),
        );
        hit(&mut game, namesake, Target::Object(source), 2, false);
        let outcome = apply(
            &mut game,
            namesake,
            Effect::destroy(ChooseSpec::SpecificObject(source)),
        );
        let mut dm = Choices {
            targets: vec![Target::Object(recipient)],
            ..Default::default()
        };
        queue_outcome(&mut game, outcome, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert!(game.object(source).is_none());
        assert_eq!(game.damage_on(recipient), 5);
    }
}
#[test]
fn hawkeye_requires_its_own_damage_to_the_exact_dying_incarnation_and_keeps_its_activation() {
    for definition in definitions("Hawkeye, Avenging Archer") {
        for blink in [false, true] {
            let mut game = game();
            library(&mut game, 3);
            let hawkeye = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert!(has(&game, hawkeye, StaticAbilityId::Reach));
            let target = creature(&mut game, B, "Hawkeye recipient", 2, 40);
            game.remove_summoning_sickness(hawkeye);
            activate(
                &mut game,
                hawkeye,
                activated(&definition),
                &mut Choices {
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                },
            );
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.damage_on(target), 1);
            let target = if blink {
                let exiled = game.move_object_by_game_rule(target, Zone::Exile).unwrap();
                game.move_object_by_game_rule(exiled, Zone::Battlefield)
                    .unwrap()
            } else {
                target
            };
            let outcome = apply(
                &mut game,
                hawkeye,
                Effect::destroy(ChooseSpec::SpecificObject(target)),
            );
            queue_outcome(&mut game, outcome, &mut Choices::default());
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(!blink));
        }
    }
}
#[test]
fn wolverine_preserves_multiplier_regeneration_and_source_antecedent_but_excludes_self_and_players()
{
    for definition in definitions("Wolverine, Best There Is") {
        for mode in 0..3 {
            let mut game = game();
            let wolverine = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let other = creature(&mut game, B, "Other recipient", 2, 40);
            let target = match mode {
                0 => Target::Object(other),
                1 => Target::Player(B),
                _ => Target::Object(wolverine),
            };
            if mode == 2 {
                apply(
                    &mut game,
                    other,
                    Effect::put_counters(
                        ironsmith::object::CounterType::PlusOnePlusOne,
                        1,
                        ChooseSpec::SpecificObject(wolverine),
                    ),
                );
            }
            hit(&mut game, wolverine, target, 1, false);
            if mode == 0 {
                assert_eq!(game.damage_on(other), 2);
            }
            if mode == 1 {
                assert_eq!(game.player(B).unwrap().life, 18);
            }
            activate(
                &mut game,
                wolverine,
                activated(&definition),
                &mut Choices::default(),
            );
            resolve_all(&mut game, &mut Choices::default());
            apply(
                &mut game,
                other,
                Effect::destroy(ChooseSpec::SpecificObject(wolverine)),
            );
            assert!(game.object(wolverine).is_some());
            assert_eq!(game.damage_on(wolverine), 0);
            end_step(&mut game);
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                game.counter_count(wolverine, ironsmith::object::CounterType::PlusOnePlusOne),
                u32::from(mode == 0 || mode == 2)
            );
            assert_eq!(
                game.counter_count(other, ironsmith::object::CounterType::PlusOnePlusOne),
                0
            );
        }
    }
}
#[test]
fn dragon_cultist_aggregates_each_historical_source_across_recipients_without_merging_sources() {
    for definition in definitions("Dragon Cultist") {
        for same_source in [false, true] {
            let mut game = game();
            let commander = creature(&mut game, A, "Commander", 2, 40);
            game.set_as_commander(commander, game.object(commander).unwrap().owner);
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let first = witness(&mut game);
            let second = witness(&mut game);
            let recipient = creature(&mut game, B, "History recipient", 2, 40);
            hit(&mut game, first, Target::Player(B), 3, false);
            hit(
                &mut game,
                if same_source { first } else { second },
                Target::Object(recipient),
                2,
                false,
            );
            game.set_current_controller(first, B).unwrap();
            end_step(&mut game);
            resolve_all(&mut game, &mut Choices::default());
            let dragons = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| game.current_has_subtype(*id, ironsmith::Subtype::Dragon))
                .collect::<Vec<_>>();
            assert_eq!(dragons.len(), usize::from(same_source));
            for dragon in dragons {
                assert_eq!(pt(&game, dragon), (4, 4));
                assert!(has(&game, dragon, StaticAbilityId::Flying));
            }
        }
    }
}
#[test]
fn case_counts_distinct_actual_damage_sources_and_its_solved_sacrifice_retains_one_play_permission()
{
    for definition in definitions("Case of the Burning Masks") {
        for distinct in [false, true] {
            let mut game = game();
            library(&mut game, 4);
            let recipient = creature(&mut game, B, "Case recipient", 2, 40);
            let case = enter(
                &mut game,
                &definition,
                &mut Choices {
                    targets: vec![Target::Object(recipient)],
                    ..Default::default()
                },
            );
            assert_eq!(game.damage_on(recipient), 3);
            let first = witness(&mut game);
            let second = witness(&mut game);
            hit(&mut game, first, Target::Object(recipient), 1, false);
            hit(
                &mut game,
                if distinct { second } else { first },
                Target::Object(recipient),
                1,
                false,
            );
            end_step(&mut game);
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.is_case_solved(case), distinct);
            if !distinct {
                continue;
            }
            // The printed activated body is usable after its real solve trigger.
            activate(
                &mut game,
                case,
                activated(&definition),
                &mut Choices::default(),
            );
            resolve_all(&mut game, &mut Choices::default());
            assert!(game.object(case).is_none());
            assert_eq!(game.player(A).unwrap().library.len(), 1);
            game.turn.phase = Phase::NextMain;
            game.turn.step = None;
            let legal = compute_legal_actions(&game, A).unwrap();
            assert_eq!(
                legal
                    .iter()
                    .filter(|a| matches!(
                        a,
                        LegalAction::CastSpell {
                            from_zone: Zone::Exile,
                            ..
                        }
                    ))
                    .count(),
                1
            );
        }
    }
}
#[test]
fn thirsting_axe_checks_the_current_host_and_exact_departed_equipment_lki() {
    for definition in definitions("Thirsting Axe") {
        for mode in 0..8 {
            let mut game = game();
            let axe = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let first = creature(&mut game, A, "First equipped creature", 2, 40);
            let second = creature(&mut game, A, "Second equipped creature", 2, 40);
            let enemy = creature(&mut game, B, "Combat recipient", 2, 40);
            activate(
                &mut game,
                axe,
                activated(&definition),
                &mut Choices {
                    targets: vec![Target::Object(first)],
                    ..Default::default()
                },
            );
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(pt(&game, first), (6, 40));
            if matches!(mode, 1 | 4) {
                hit(&mut game, first, Target::Object(enemy), 1, true);
            }
            if mode == 2 {
                hit(&mut game, first, Target::Player(B), 1, true);
            }
            if mode == 3 {
                hit(&mut game, first, Target::Object(enemy), 1, false);
            }
            if mode == 4 {
                activate(
                    &mut game,
                    axe,
                    activated(&definition),
                    &mut Choices {
                        targets: vec![Target::Object(second)],
                        ..Default::default()
                    },
                );
                resolve_all(&mut game, &mut Choices::default());
            }
            end_step(&mut game);
            if mode == 5 {
                // The pending independent trigger retains the departed Axe's
                // host even though ordinary attachment cleanup now detaches it.
                apply(
                    &mut game,
                    enemy,
                    Effect::destroy(ChooseSpec::SpecificObject(axe)),
                );
            }
            if mode == 6 {
                apply(
                    &mut game,
                    enemy,
                    Effect::unattach_objects(ChooseSpec::SpecificObject(axe)),
                );
            }
            let returned = if mode == 7 {
                let exiled = game.move_object_by_game_rule(first, Zone::Exile).unwrap();
                Some(
                    game.move_object_by_game_rule(exiled, Zone::Battlefield)
                        .unwrap(),
                )
            } else {
                None
            };
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.object(first).is_some(), matches!(mode, 1 | 4 | 6));
            if let Some(returned) = returned {
                assert!(game.object(returned).is_some());
            }
            assert_eq!(game.object(second).is_some(), mode != 4);
        }
    }
}
#[test]
fn grothama_grants_each_attacker_a_fight_with_the_correct_grantor_and_draws_by_damage_time_controller()
 {
    for definition in definitions("Grothama, All-Devouring") {
        let mut game = game();
        for player in [A, B, C] {
            library_for(&mut game, player, 8);
        }
        let first = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let second = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let attacker = creature(&mut game, A, "Grothama attacker", 1, 40);
        let mut dm = Choices {
            optional: Some(true),
            ..Default::default()
        };
        attack(&mut game, attacker, &mut dm);
        assert_eq!(
            game.stack.len(),
            2,
            "one independent grant from each Grothama"
        );
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.damage_on(attacker), 20);
        assert_eq!(game.damage_on(first), 1);
        assert_eq!(game.damage_on(second), 1);
        let dealer = creature(&mut game, A, "Changing damage source", 2, 40);
        hit(&mut game, dealer, Target::Object(first), 1, false);
        game.set_current_controller(dealer, B).unwrap();
        hit(&mut game, dealer, Target::Object(first), 3, false);
        let outcome = apply(
            &mut game,
            dealer,
            Effect::destroy(ChooseSpec::SpecificObject(first)),
        );
        queue_outcome(&mut game, outcome, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(B).unwrap().hand.len(), 3);
        assert!(game.player(C).unwrap().hand.is_empty());
        assert!(game.object(second).is_some());
    }
}

#[test]
fn impact_uses_one_source_recipient_occurrence_and_keeps_announced_shares_after_responses() {
    for definition in definitions("Impact Resonance") {
        for illegal_first in [false, true] {
            let mut game = game();
            let dealer = creature(&mut game, A, "History dealer", 2, 50);
            let first = creature(&mut game, B, "First division target", 1, 50);
            let second = creature(&mut game, C, "Second division target", 1, 50);
            apply(
                &mut game,
                dealer,
                Effect::new(ironsmith::effects::DealDamageToRecipientsEffect {
                    amount: ironsmith_core::Value::Fixed(5),
                    recipients: vec![
                        ChooseSpec::SpecificObject(first),
                        ChooseSpec::SpecificObject(second),
                        ChooseSpec::SpecificPlayer(B),
                        ChooseSpec::SpecificPlayer(C),
                    ],
                }),
            );
            let mut choices = Choices {
                targets: vec![Target::Object(first), Target::Object(second)],
                distribution: vec![(Target::Object(first), 3), (Target::Object(second), 2)],
                ..Default::default()
            };
            let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut choices);
            assert_eq!(
                choices.distribution_totals,
                vec![5],
                "one occurrence's per-recipient maximum is 5, not source total20"
            );
            let mut returned = None;
            if illegal_first {
                let exile = game.move_object_by_effect(first, Zone::Exile).unwrap();
                returned = game.move_object_by_effect(exile, Zone::Battlefield);
            }
            apply(
                &mut game,
                dealer,
                Effect::deal_damage(8, ChooseSpec::SpecificPlayer(C)),
            );
            resolve_all(&mut game, &mut choices);
            assert_eq!(
                choices.distribution_totals,
                vec![5],
                "no new allocation decision at resolution"
            );
            assert_eq!(
                game.damage_on(second),
                7,
                "a removed target's share cannot move to the survivor"
            );
            if let Some(returned) = returned {
                assert_eq!(game.damage_on(returned), 0);
            } else {
                assert_eq!(game.damage_on(first), 8);
            }
            let receipts = game
                .turn_store
                .turn_history
                .event_records
                .iter()
                .chain(game.turn_store.turn_history.staged_event_records.iter())
                .filter_map(|record| record.event.downcast::<ironsmith::events::DamageEvent>())
                .filter(|event| event.source == spell)
                .collect::<Vec<_>>();
            assert_eq!(receipts.len(), if illegal_first { 1 } else { 2 });
            assert_eq!(
                receipts.iter().map(|event| event.amount).sum::<u32>(),
                if illegal_first { 2 } else { 5 }
            );
        }
    }
}

#[test]
fn impact_with_no_prior_damage_can_be_cast_with_no_targets() {
    for definition in definitions("Impact Resonance") {
        let mut game = game();
        let mut choices = Choices {
            decline_targets: true,
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut choices);
        resolve_all(&mut game, &mut choices);
        assert!(choices.distribution_totals.is_empty());
        assert!(
            !game
                .turn_store
                .turn_history
                .event_records
                .iter()
                .chain(game.turn_store.turn_history.staged_event_records.iter())
                .any(|record| record
                    .event
                    .downcast::<ironsmith::events::DamageEvent>()
                    .is_some())
        );
    }
}
