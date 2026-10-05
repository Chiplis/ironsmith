//! UNVALIDATED duration/source prevention; scenarios are authored, unrun.
#![allow(dead_code)]
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
        "../../../fixtures/duration_source_prevention.json.fixture"
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
    modes: Vec<usize>,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose mode") && !self.modes.is_empty() {
            return self.modes.clone();
        }
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
    cast_for(game, A, definition, method, dm)
}
fn cast_for(
    game: &mut GameState,
    player: PlayerId,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, player, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(
        compute_legal_actions(game, player)
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

fn flush(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("control trigger did not settle");
}
fn named(game: &GameState, name: &str) -> ObjectId {
    *game
        .battlefield
        .iter()
        .find(|id| game.object(**id).unwrap().name == name)
        .unwrap()
}
fn enter(
    game: &mut GameState,
    definition: &CardDefinition,
    owner: PlayerId,
    dm: &mut Choices,
) -> ObjectId {
    let old = game.create_object_from_definition(definition, owner, Zone::Hand);
    let receipt = game
        .move_object_with_etb_processing_with_dm(old, Zone::Battlefield, dm)
        .unwrap();
    assert!(!receipt.pending);
    assert!(
        receipt.programs.is_empty(),
        "do not discard entry additions"
    );
    receipt.original.into_result().unwrap().new_id
}

fn creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    game.create_object_from_definition(
        &vanilla(name, "{2}", "Soldier", 3, 50),
        owner,
        Zone::Battlefield,
    )
}
fn damage(
    game: &mut GameState,
    source: ObjectId,
    target: Target,
    combat: bool,
    unpreventable: bool,
    snapshot: Option<ironsmith::snapshot::ObjectSnapshot>,
) -> EffectOutcome {
    let controller = game
        .current_controller(source)
        .or_else(|| snapshot.as_ref().map(|s| s.controller))
        .unwrap();
    let target = match target {
        Target::Object(id) => ChooseSpec::SpecificObject(id),
        Target::Player(id) => ChooseSpec::Player(PlayerFilter::Specific(id)),
    };
    let effect = Effect::new(
        ironsmith::effects::DealDamageEffect::new(3, target)
            .with_combat(combat)
            .with_unpreventable(unpreventable),
    );
    let mut dm = SelectFirstDecisionMaker;
    let mut context = EffectContext::new(source, controller, &mut dm);
    if let Some(snapshot) = snapshot {
        context = context.with_source_snapshot(snapshot);
    }
    execute_effect(game, &effect, &mut context).unwrap()
}
fn dealt(outcome: &EffectOutcome) -> u32 {
    outcome
        .events
        .iter()
        .filter_map(|e| {
            e.downcast::<ironsmith::events::DamageEvent>()
                .map(|e| e.amount)
        })
        .sum()
}
fn next_turn_of(game: &mut GameState, player: PlayerId) {
    let original = game.turn.turn_number;
    for _ in 0..150 {
        ironsmith::turn::advance_step(game).unwrap();
        if game.turn.turn_number > original && game.turn.active_player == player {
            return;
        }
    }
    panic!("next turn did not arrive");
}
#[test]
fn seven_targeted_duration_candidates_keep_full_metadata_and_artifacts() {
    let rows = fixtures();
    assert_eq!(rows.len(), 12);
    let complete: Vec<_> = rows
        .iter()
        .filter(|row| row["proposed_complete"] == true)
        .collect();
    assert_eq!(complete.len(), 7);
    for row in complete {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
            let text = format!("{definition:?}");
            assert!(text.contains("PreventAllDamageEffect"), "{text}");
        }
    }
}
#[test]
fn kiora_and_dovin_share_one_target_then_shield_both_directions_until_the_original_controllers_next_turn()
 {
    for name in ["Kiora, the Crashing Wave", "Dovin, Hand of Control"] {
        for definition in definitions(name) {
            let mut game = game();
            let walker = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let chosen = creature(&mut game, B, "Chosen damager");
            let other = creature(&mut game, C, "Other damager");
            let mut choices = Choices {
                targets: vec![Target::Object(chosen)],
                ..Default::default()
            };
            activate(
                &mut game,
                walker,
                activated_at(&definition, 0),
                &mut choices,
            );
            assert_eq!(
                game.stack.last().unwrap().targets,
                vec![Target::Object(chosen)]
            );
            resolve_all(&mut game, &mut choices);
            assert_eq!(game.effect_store.prevention_effects.shields().len(), 2);
            for combat in [false, true] {
                assert_eq!(
                    dealt(&damage(
                        &mut game,
                        chosen,
                        Target::Player(A),
                        combat,
                        false,
                        None
                    )),
                    0
                );
                assert_eq!(
                    dealt(&damage(
                        &mut game,
                        other,
                        Target::Object(chosen),
                        combat,
                        false,
                        None
                    )),
                    0
                );
            }
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    chosen,
                    Target::Player(A),
                    false,
                    true,
                    None
                )),
                3
            );
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    other,
                    Target::Player(A),
                    false,
                    false,
                    None
                )),
                3
            );
            // The target restriction was checked at resolution; changing control
            // does not turn either exact-incarnation shield off.
            game.set_current_controller(chosen, A).unwrap();
            game.move_object_by_effect(walker, Zone::Exile).unwrap();
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    chosen,
                    Target::Player(B),
                    false,
                    false,
                    None
                )),
                0
            );
            next_turn_of(&mut game, B);
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    other,
                    Target::Object(chosen),
                    false,
                    false,
                    None
                )),
                0
            );
            next_turn_of(&mut game, A);
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    chosen,
                    Target::Player(B),
                    false,
                    false,
                    None
                )),
                3
            );
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    other,
                    Target::Object(chosen),
                    false,
                    false,
                    None
                )),
                3
            );
        }
    }
}
#[test]
fn exact_source_lki_remains_shielded_but_a_returned_card_is_a_new_incarnation() {
    for definition in definitions_text(
        "Unlisted dual shield",
        "Mana cost: {W}\nType: Instant\nUntil your next turn, prevent all damage that would be dealt to and dealt by target permanent.",
    ) {
        let mut game = game();
        let target = creature(&mut game, B, "Original source");
        let other = creature(&mut game, C, "Other source");
        let mut choices = Choices {
            targets: vec![Target::Object(target)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut choices);
        resolve_all(&mut game, &mut choices);
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(target).unwrap(),
                &game,
            );
        let exile = game.move_object_by_effect(target, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_effect(exile, Zone::Battlefield)
            .unwrap();
        assert_ne!(returned, target);
        assert_eq!(
            dealt(&damage(
                &mut game,
                target,
                Target::Player(A),
                false,
                false,
                Some(snapshot)
            )),
            0
        );
        assert_eq!(
            dealt(&damage(
                &mut game,
                returned,
                Target::Player(A),
                false,
                false,
                None
            )),
            3
        );
        assert_eq!(
            dealt(&damage(
                &mut game,
                other,
                Target::Object(returned),
                false,
                false,
                None
            )),
            3
        );
    }
}
#[test]
fn dromokas_four_modes_keep_the_targeted_spell_shield_sacrifice_counter_and_real_fight() {
    for definition in definitions("Dromoka's Command") {
        for first_pair in [true, false] {
            let mut game = game();
            let own = creature(&mut game, A, "Own fighter");
            let enemy = creature(&mut game, B, "Enemy fighter");
            let enchantment = game.create_object_from_definition(
                &resource("Sacrifice me", "Enchantment"),
                B,
                Zone::Battlefield,
            );
            let spell = game.create_object_from_definition(
                &resource("Opposing spell", "Instant"),
                B,
                Zone::Stack,
            );
            game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, B));
            let mut choices = Choices {
                modes: if first_pair { vec![0, 2] } else { vec![1, 3] },
                targets: if first_pair {
                    vec![Target::Object(spell), Target::Object(own)]
                } else {
                    vec![
                        Target::Player(B),
                        Target::Object(own),
                        Target::Object(enemy),
                    ]
                },
                objects: vec![enchantment],
                objects_explicit: true,
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut choices);
            resolve(&mut game, &mut choices);
            if first_pair {
                assert_eq!(pt(&game, own), (4, 51));
                assert_eq!(
                    dealt(&damage(
                        &mut game,
                        spell,
                        Target::Player(A),
                        false,
                        false,
                        None
                    )),
                    0
                );
                assert_eq!(
                    dealt(&damage(
                        &mut game,
                        enemy,
                        Target::Player(A),
                        false,
                        false,
                        None
                    )),
                    3
                );
            } else {
                assert!(game.object(enchantment).is_none());
                assert_eq!(game.damage_on(own), 3);
                assert_eq!(game.damage_on(enemy), 3);
            }
        }
    }
}
#[test]
fn inquisitor_preserves_the_color_test_and_shields_departure_source_damage() {
    for definition in definitions("Inquisitor's Snare") {
        for black in [true, false] {
            let mut game = game();
            let target = game.create_object_from_definition(
                &vanilla(
                    "Attacker",
                    if black { "{B}" } else { "{G}" },
                    "Soldier",
                    3,
                    50,
                ),
                B,
                Zone::Battlefield,
            );
            game.turn.active_player = B;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
            game.remove_summoning_sickness(target);
            let mut combat = ironsmith::combat_state::CombatState::default();
            ironsmith::combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(target, ironsmith::combat_state::AttackTarget::Player(A))],
            )
            .unwrap();
            game.combat = Some(combat);
            let snapshot =
                ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    game.object(target).unwrap(),
                    &game,
                );
            let mut choices = Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut choices);
            resolve_all(&mut game, &mut choices);
            assert_eq!(game.object(target).is_none(), black);
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    target,
                    Target::Player(A),
                    false,
                    false,
                    Some(snapshot)
                )),
                0
            );
        }
    }
}
#[test]
fn old_fat_spider_source_lifetime_starts_only_while_present_and_cannot_restart_after_phasing() {
    for definition in definitions("Old Fat Spider Can't See Me") {
        for departure in [false, true] {
            let mut game = game();
            let friend = creature(&mut game, A, "Protected friend");
            let enemy = creature(&mut game, B, "Chosen enemy");
            let mut choices = Choices {
                targets: vec![Target::Object(friend)],
                ..Default::default()
            };
            let saga = enter(&mut game, &definition, A, &mut choices);
            flush(&mut game, &mut choices);
            assert!(has(
                &game,
                friend,
                ironsmith::static_abilities::StaticAbilityId::Hexproof
            ));
            lore(
                &mut game,
                saga,
                &mut Choices {
                    targets: vec![Target::Object(enemy)],
                    ..Default::default()
                },
            );
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    enemy,
                    Target::Player(A),
                    false,
                    false,
                    None
                )),
                0
            );
            if departure {
                game.move_object_by_effect(saga, Zone::Exile).unwrap();
            } else {
                game.phase_out(saga);
                game.phase_in(saga);
            }
            assert_eq!(
                dealt(&damage(
                    &mut game,
                    enemy,
                    Target::Player(A),
                    false,
                    false,
                    None
                )),
                3
            );
            assert!(!has(
                &game,
                friend,
                ironsmith::static_abilities::StaticAbilityId::Hexproof
            ));
        }
    }
}
#[test]
fn kiora_other_loyalty_modes_draw_allow_a_land_and_create_the_end_step_kraken_emblem() {
    for definition in definitions("Kiora, the Crashing Wave") {
        let mut game = game();
        let walker = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.create_object_from_definition(&resource("Drawn card", "Artifact"), A, Zone::Library);
        activate(
            &mut game,
            walker,
            activated_at(&definition, 1),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        game.update_cant_effects();
        assert_eq!(game.player(A).unwrap().land_plays_per_turn, 2);
        next_turn_of(&mut game, A);
        apply(
            &mut game,
            walker,
            Effect::put_counters(
                ironsmith::object::CounterType::Loyalty,
                5,
                ChooseSpec::SpecificObject(walker),
            ),
        );
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        activate(
            &mut game,
            walker,
            activated_at(&definition, 2),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        event(
            &mut game,
            TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfEndStepEvent::new(A),
                Default::default(),
            ),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(
            game.battlefield
                .iter()
                .any(|id| game.object(*id).is_some_and(|o| o.name == "Kraken")
                    && pt(&game, *id) == (9, 9))
        );
    }
}
#[test]
fn old_fat_spider_late_chapters_draw_and_an_already_departed_saga_cannot_start_a_shield() {
    for definition in definitions("Old Fat Spider Can't See Me") {
        for leave_before_resolution in [false, true] {
            let mut game = game();
            let friend = creature(&mut game, A, "Friend");
            let enemy = creature(&mut game, B, "Enemy");
            for i in 0..3 {
                game.create_object_from_definition(
                    &resource(&format!("Library {i}"), "Artifact"),
                    A,
                    Zone::Library,
                );
            }
            let mut dm = Choices {
                targets: vec![Target::Object(friend)],
                ..Default::default()
            };
            let saga = enter(&mut game, &definition, A, &mut dm);
            flush(&mut game, &mut dm);
            let outcome = apply(
                &mut game,
                saga,
                Effect::put_counters(
                    ironsmith::object::CounterType::Lore,
                    1,
                    ChooseSpec::SpecificObject(saga),
                ),
            );
            queue_outcome(
                &mut game,
                outcome,
                &mut Choices {
                    targets: vec![Target::Object(enemy)],
                    ..Default::default()
                },
            );
            if leave_before_resolution {
                game.move_object_by_effect(saga, Zone::Exile).unwrap();
            }
            resolve_all(&mut game, &mut Choices::default());
            if leave_before_resolution {
                assert_eq!(
                    dealt(&damage(
                        &mut game,
                        enemy,
                        Target::Player(A),
                        false,
                        false,
                        None
                    )),
                    3
                );
            } else {
                lore(&mut game, saga, &mut Choices::default());
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                lore(&mut game, saga, &mut Choices::default());
                assert_eq!(game.player(A).unwrap().hand.len(), 2);
            }
        }
    }
}
#[test]
fn dovin_tax_retains_opponent_scope_and_the_three_spell_type_alternatives() {
    for definition in definitions("Dovin, Hand of Control") {
        for (types, player, expected) in [
            ("Artifact", B, 2),
            ("Instant", B, 2),
            ("Sorcery", B, 2),
            ("Enchantment", B, 1),
            ("Artifact Creature", B, 2),
            ("Artifact", A, 1),
        ] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.turn.active_player = player;
            game.turn.priority_player = Some(player);
            if player == B {
                game.player_mut(B)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 10);
            }
            let before = game.player(player).unwrap().mana_pool.total();
            cast_for(
                &mut game,
                player,
                &resource("Taxed spell", types),
                CastingMethod::Normal,
                &mut Choices::default(),
            );
            assert_eq!(
                before - game.player(player).unwrap().mana_pool.total(),
                expected,
                "{types} {player:?}"
            );
        }
    }
}

#[test]
fn hallow_gains_only_when_damage_is_actually_prevented_and_follows_only_the_resolved_permanent() {
    for definition in definitions("Hallow") {
        let mut game = game();
        game.turn.active_player = B;
        game.turn.priority_player = Some(B);
        game.player_mut(B)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 10);
        let creature_spell = vanilla("Resolved Hallow source", "{2}", "Soldier", 3, 50);
        let spell = cast_for(
            &mut game,
            B,
            &creature_spell,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        game.turn.priority_player = Some(A);
        let mut dm = Choices {
            targets: vec![Target::Object(spell)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(
            game.player(A).unwrap().life,
            20,
            "no life is gained when the shield is created"
        );
        resolve_all(&mut game, &mut Choices::default());
        let permanent = named(&game, "Resolved Hallow source");
        assert_ne!(spell, permanent);
        let recipient = creature(&mut game, C, "Different recipient");
        assert_eq!(
            dealt(&damage(
                &mut game,
                permanent,
                Target::Player(C),
                false,
                false,
                None
            )),
            0
        );
        assert_eq!(game.player(A).unwrap().life, 23);
        assert_eq!(game.player(C).unwrap().life, 20);
        assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(
            dealt(&damage(
                &mut game,
                permanent,
                Target::Object(recipient),
                true,
                false,
                None
            )),
            0
        );
        assert_eq!(game.player(A).unwrap().life, 26);
        assert_eq!(
            dealt(&damage(
                &mut game,
                permanent,
                Target::Player(C),
                false,
                true,
                None
            )),
            3
        );
        assert_eq!(
            game.player(A).unwrap().life,
            26,
            "unpreventable damage creates no gain"
        );
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(permanent).unwrap(),
                &game,
            );
        let exile = game.move_object_by_effect(permanent, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_effect(exile, Zone::Battlefield)
            .unwrap();
        assert_eq!(
            dealt(&damage(
                &mut game,
                returned,
                Target::Player(C),
                false,
                false,
                None
            )),
            3
        );
        assert_eq!(
            game.player(A).unwrap().life,
            26,
            "a later blink cannot refresh the exception"
        );
        assert_eq!(
            dealt(&damage(
                &mut game,
                permanent,
                Target::Player(C),
                false,
                false,
                Some(snapshot.clone())
            )),
            0
        );
        assert_eq!(
            game.player(A).unwrap().life,
            29,
            "damage by that exact departed permanent still uses its shield"
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(
            dealt(&damage(
                &mut game,
                permanent,
                Target::Player(C),
                false,
                false,
                Some(snapshot)
            )),
            3
        );
        assert_eq!(game.player(A).unwrap().life, 29);
    }
}

#[test]
fn hallow_follows_actual_prevention_and_defers_gain_until_all_original_recipients_are_committed() {
    for definition in definitions("Hallow") {
        let mut game = game();
        let spell = game.create_object_from_definition(
            &resource("Damage spell", "Creature"),
            B,
            Zone::Stack,
        );
        game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, B));
        let mut dm = Choices {
            targets: vec![Target::Object(spell)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve(&mut game, &mut dm);
        resolve_all(&mut game, &mut Choices::default());
        let permanent = named(&game, "Damage spell");
        let observer_definition = compile_to_runtime_definition(
            "Hallow observer",
            "Type: Creature
Power/Toughness: 1/50
Whenever you gain life, if you have 20 or less life, put a +1/+1 counter on this creature.",
            false,
        )
        .unwrap();
        let observer =
            game.create_object_from_definition(&observer_definition, A, Zone::Battlefield);
        // Both original damage proposals finish before the additional gain.
        // The gain observer distinguishes final life 20 from an early gain at 23.
        let other = creature(&mut game, B, "Unshielded source");
        let mut first = SelectFirstDecisionMaker;
        let effect = Effect::new(ironsmith::effects::DealDamageBySourcesEffect::new(
            vec![
                ChooseSpec::SpecificObject(permanent),
                ChooseSpec::SpecificObject(other),
            ],
            ironsmith::effect::Value::Fixed(3),
            ChooseSpec::Player(PlayerFilter::Specific(A)),
        ));
        let result = execute_effect(
            &mut game,
            &effect,
            &mut EffectContext::new(permanent, B, &mut first),
        )
        .unwrap();
        assert_eq!(dealt(&result), 3);
        assert_eq!(game.player(A).unwrap().life, 20);
        let gain = result
            .events
            .iter()
            .filter_map(|e| e.downcast::<ironsmith::events::LifeGainEvent>())
            .map(|e| e.amount)
            .sum::<u32>();
        assert_eq!(gain, 3);
        flush(&mut game, &mut Choices::default());
        assert_eq!(pt(&game, observer), (2, 51));
    }
}
#[test]
fn hallow_invalid_target_and_failed_deferred_gain_do_not_commit_a_partial_result() {
    for definition in definitions("Hallow") {
        let mut game = game();
        let spell = game.create_object_from_definition(
            &resource("Failed damage spell", "Instant"),
            B,
            Zone::Stack,
        );
        game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, B));
        let mut dm = Choices {
            targets: vec![Target::Object(spell)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        game.move_object_by_effect(spell, Zone::Exile).unwrap();
        resolve(&mut game, &mut dm);
        assert!(game.effect_store.prevention_effects.shields().is_empty());
        assert_eq!(game.player(A).unwrap().life, 20);
    }
    let mut game = game();
    let source = creature(&mut game, B, "Native shield source");
    let producer = creature(&mut game, A, "Native prevention creator");
    let shield = ironsmith::effects::PreventAllDamageEffect::all(Until::EndOfTurn)
        .with_target_source(ChooseSpec::SpecificObject(source))
        .with_follow_up_effects(vec![Effect::gain_life(
            ironsmith::effect::Value::EventValue(ironsmith::effect::EventValueSpec::Amount),
        )]);
    let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(
        Effect::new(shield),
    )
    .unwrap();
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();
    assert_eq!(
        restored
            .downcast_ref::<ironsmith::effects::PreventAllDamageEffect>()
            .unwrap()
            .follow_up_effects
            .len(),
        1
    );
    apply(&mut game, producer, restored);
    apply(&mut game, producer, Effect::gain_life(i32::MAX - 21));
    assert_eq!(game.player(A).unwrap().life, i32::MAX - 1);
    let history_before = format!("{:?}", game.turn_store.turn_history);
    let mut first = SelectFirstDecisionMaker;
    let result = execute_effect(
        &mut game,
        &Effect::new(ironsmith::effects::DealDamageEffect::new(
            3,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
        )),
        &mut EffectContext::new(source, B, &mut first),
    );
    assert!(
        matches!(
            result,
            Err(ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. })
        ),
        "{result:?}"
    );
    assert_eq!(game.player(A).unwrap().life, i32::MAX - 1);
    assert_eq!(
        format!("{:?}", game.turn_store.turn_history),
        history_before
    );
    assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
}

#[test]
fn gideon_emblem_checks_current_control_and_subtype_in_the_command_zone() {
    for definition in definitions("Gideon of the Trials") {
        let mut game = game();
        let gideon = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other_definition = compile_to_runtime_definition(
            "Other qualifying planeswalker",
            "Type: Planeswalker — Gideon\nLoyalty: 3",
            false,
        )
        .unwrap();
        let opponent_gideon =
            game.create_object_from_definition(&other_definition, B, Zone::Battlefield);
        activate(
            &mut game,
            gideon,
            activated_at(&definition, 2),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        game.update_cant_effects();
        assert!(game.effect_store.cant_effects.cant_lose_game.contains(&A));
        assert!(game.effect_store.cant_effects.cant_win_game.contains(&B));
        assert!(game.effect_store.cant_effects.cant_win_game.contains(&C));
        apply(&mut game, gideon, Effect::lose_the_game());
        assert!(game.player(A).unwrap().is_in_game());
        apply(
            &mut game,
            gideon,
            Effect::win_the_game_player(PlayerFilter::Specific(B)),
        );
        assert!(game.player(A).unwrap().is_in_game());
        assert!(!game.player(B).unwrap().has_won);
        apply(
            &mut game,
            gideon,
            Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
                ironsmith::continuous::EffectTarget::Specific(gideon),
                ironsmith::continuous::Modification::RemoveAllAbilities,
                Until::Forever,
            )),
        );
        game.update_cant_effects();
        assert!(
            game.effect_store.cant_effects.cant_lose_game.contains(&A),
            "Gideon's abilities are irrelevant to its subtype and the emblem's rule"
        );
        // A Gideon controlled by an opponent does not maintain our emblem.
        game.move_object_by_effect(gideon, Zone::Exile).unwrap();
        game.update_cant_effects();
        assert!(!game.effect_store.cant_effects.cant_lose_game.contains(&A));
        assert!(!game.effect_store.cant_effects.cant_win_game.contains(&B));
        game.set_current_controller(opponent_gideon, A).unwrap();
        game.update_cant_effects();
        assert!(game.effect_store.cant_effects.cant_lose_game.contains(&A));
        game.phase_out(opponent_gideon);
        game.update_cant_effects();
        assert!(!game.effect_store.cant_effects.cant_lose_game.contains(&A));
        game.phase_in(opponent_gideon);
        game.update_cant_effects();
        assert!(
            game.effect_store.cant_effects.cant_lose_game.contains(&A),
            "this is a live condition, not an ended duration"
        );
        game.set_current_controller(opponent_gideon, B).unwrap();
        game.update_cant_effects();
        assert!(!game.effect_store.cant_effects.cant_lose_game.contains(&A));
        let plain = creature(&mut game, A, "A creature named Gideon");
        apply(&mut game, plain, Effect::lose_the_game());
        assert!(
            !game.player(A).unwrap().is_in_game(),
            "name alone cannot satisfy a planeswalker-subtype condition"
        );
    }
}
#[test]
fn gideon_animation_keeps_planeswalker_type_and_real_incoming_prevention_until_cleanup() {
    for definition in definitions("Gideon of the Trials") {
        let mut game = game();
        let gideon = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let enemy = creature(&mut game, B, "Damage source");
        activate(
            &mut game,
            gideon,
            activated_at(&definition, 1),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(pt(&game, gideon), (4, 4));
        assert!(game.current_is_creature(gideon));
        assert!(
            game.calculated_characteristics(gideon)
                .unwrap()
                .card_types
                .contains(&ironsmith::CardType::Planeswalker)
        );
        assert!(has(
            &game,
            gideon,
            ironsmith::static_abilities::StaticAbilityId::Indestructible
        ));
        assert_eq!(
            dealt(&damage(
                &mut game,
                enemy,
                Target::Object(gideon),
                false,
                false,
                None
            )),
            0
        );
        assert_eq!(
            dealt(&damage(
                &mut game,
                enemy,
                Target::Object(gideon),
                true,
                false,
                None
            )),
            0
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(!game.current_is_creature(gideon));
        assert_eq!(
            dealt(&damage(
                &mut game,
                enemy,
                Target::Object(gideon),
                false,
                false,
                None
            )),
            3
        );
    }
}
#[test]
fn gideon_first_loyalty_prevents_outgoing_damage_without_protecting_the_target_from_other_sources()
{
    for definition in definitions("Gideon of the Trials") {
        let mut game = game();
        let gideon = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let chosen = creature(&mut game, B, "Chosen source");
        let other = creature(&mut game, C, "Other source");
        let mut dm = Choices {
            targets: vec![Target::Object(chosen)],
            ..Default::default()
        };
        activate(&mut game, gideon, activated_at(&definition, 0), &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            dealt(&damage(
                &mut game,
                chosen,
                Target::Player(A),
                false,
                false,
                None
            )),
            0
        );
        assert_eq!(
            dealt(&damage(
                &mut game,
                other,
                Target::Object(chosen),
                false,
                false,
                None
            )),
            3
        );
        next_turn_of(&mut game, A);
        assert_eq!(
            dealt(&damage(
                &mut game,
                chosen,
                Target::Player(A),
                false,
                false,
                None
            )),
            3
        );
    }
}

#[test]
fn hallow_rejects_a_fully_prevented_damage_amount_above_the_scalar_range_transactionally() {
    for definition in definitions("Hallow") {
        for amount in [i32::MAX as u32 + 1, u32::MAX] {
            let mut game = game();
            let source = game.create_object_from_definition(
                &resource("Expanded damage spell", "Instant"),
                B,
                Zone::Stack,
            );
            game.push_to_stack(ironsmith::game_state::StackEntry::new(source, B));
            let mut dm = Choices {
                targets: vec![Target::Object(source)],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve(&mut game, &mut dm);
            // A self-replacement is applied before the ordinary prevention
            // choice. Its valid u32 amount is then prevented in full, leaving
            // no surviving assignment for the later damage-count guard.
            let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(
                    source,
                    B,
                    ironsmith::events::damage::matchers::DamageFromSourceMatcher::new(
                        ironsmith::target::ObjectFilter::specific(source),
                    ),
                    ironsmith::replacement::ReplacementAction::Modify(
                        ironsmith::replacement::EventModification::SetTo(amount),
                    ),
                )
                .with_priority_override(ironsmith::events::ReplacementPriority::SelfReplacement),
            );
            let history_before = format!("{:?}", game.turn_store.turn_history);
            let mut first = SelectFirstDecisionMaker;
            let result = execute_effect(
                &mut game,
                &Effect::new(ironsmith::effects::DealDamageEffect::new(
                    1,
                    ChooseSpec::Player(PlayerFilter::Specific(A)),
                )),
                &mut EffectContext::new(source, B, &mut first),
            );
            let error = result.unwrap_err();
            assert!(error.is_incomplete_execution());
            assert!(
                matches!(error,ironsmith::effects::ExecutionError::ResourceLimitExceeded{requested,maximum,..} if requested==u128::from(amount)&&maximum==i32::MAX as u128)
            );
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(
                format!("{:?}", game.turn_store.turn_history),
                history_before
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_some(),
                "rollback restores the consumed replacement"
            );
            assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
        }
    }
}
