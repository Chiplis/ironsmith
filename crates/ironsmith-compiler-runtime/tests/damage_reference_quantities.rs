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
        "../../../fixtures/damage_reference_quantities.json.fixture"
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

fn grant(
    game: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    ability: ironsmith::static_abilities::StaticAbility,
) {
    apply(
        game,
        source,
        Effect::grant(
            ironsmith::grant::Grantable::ability(ability),
            ChooseSpec::SpecificObject(target),
            ironsmith::grant::GrantDuration::UntilEndOfTurn,
        ),
    );
}

#[test]
fn six_full_cards_keep_exact_quantity_bindings_through_artifact_transport() {
    assert_eq!(fixtures().len(), 6);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let debug = format!("{definition:?}");
            assert!(!debug.contains("__returned_this_way_quantity__"), "{debug}");
        }
    }
}

#[test]
fn destruction_quantities_read_calculated_departure_power_only_when_the_creature_dies() {
    for name in ["Cinder Cloud", "Kaervek's Purge"] {
        for definition in definitions(name) {
            for (white, indestructible) in [(true, false), (true, true), (false, false)] {
                let mut game = game();
                let card = vanilla(
                    "Power victim",
                    if white { "{2}{W}" } else { "{2}{B}" },
                    "Human",
                    4,
                    4,
                );
                let victim = game.create_object_from_definition(&card, A, Zone::Battlefield);
                game.set_current_controller(victim, B).unwrap();
                let stable = game.object(victim).unwrap().stable_id;
                let mut dm = Choices {
                    targets: vec![Target::Object(victim)],
                    x: 3,
                    ..Default::default()
                };
                let source = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
                if name == "Kaervek's Purge" {
                    assert_eq!(game.stack.last().unwrap().x_value, Some(3));
                    assert_eq!(
                        game.stack.last().unwrap().targets,
                        vec![Target::Object(victim)]
                    );
                }
                apply(
                    &mut game,
                    source,
                    Effect::pump(3, 3, ChooseSpec::SpecificObject(victim), Until::EndOfTurn),
                );
                if indestructible {
                    grant(
                        &mut game,
                        source,
                        victim,
                        ironsmith::static_abilities::StaticAbility::indestructible(),
                    );
                }
                resolve_all(&mut game, &mut dm);
                let deals = !indestructible && (white || name == "Kaervek's Purge");
                let current = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(
                    game.object(current).unwrap().zone,
                    if indestructible {
                        Zone::Battlefield
                    } else {
                        Zone::Graveyard
                    }
                );
                assert_eq!(game.player(B).unwrap().life, if deals { 13 } else { 20 });
                assert_eq!(game.player(A).unwrap().life, 20, "controller, not owner");
            }
        }
    }
}

#[test]
fn returned_card_quantity_reads_the_current_hand_object_after_the_legal_move() {
    for definition in definitions("Morgue Burst") {
        let mut game = game();
        let creature = compile_to_runtime_definition("Graveyard-count creature", "Mana cost: {3}\nType: Creature — Lhurgoyf\nPower/Toughness: */*\nThis creature's power and toughness are each equal to the number of creature cards in all graveyards.", false).unwrap();
        let returned = game.create_object_from_definition(&creature, A, Zone::Graveyard);
        let stable = game.object(returned).unwrap().stable_id;
        for player in [A, B] {
            game.create_object_from_definition(
                &vanilla("Other graveyard card", "{1}", "Human", 1, 1),
                player,
                Zone::Graveyard,
            );
        }
        let mut dm = Choices {
            targets: vec![Target::Object(returned), Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        let hand_object = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(hand_object, returned);
        assert_eq!(game.object(hand_object).unwrap().zone, Zone::Hand);
        assert_eq!(
            game.player(B).unwrap().life,
            18,
            "power after returning is two, not the graveyard snapshot's three"
        );
    }
}

#[test]
fn heretic_uses_live_or_departure_mana_value_and_the_resolution_time_controller() {
    let prototype = compile_to_runtime_definition("Prototype artifact", "Mana cost: {5}\nType: Artifact Creature — Golem\nPower/Toughness: 5/5\nPrototype {1}{G} — 2/3", false).unwrap();
    for definition in definitions("Viashino Heretic") {
        for indestructible in [false, true] {
            let mut game = game();
            let mut dm = Choices::default();
            let method = CastingMethod::Alternative(
                prototype
                    .alternative_casts
                    .iter()
                    .position(|a| a.name().eq_ignore_ascii_case("prototype"))
                    .unwrap(),
            );
            cast(&mut game, &prototype, method, &mut dm);
            resolve_all(&mut game, &mut dm);
            let victim = *game.battlefield.iter().next().unwrap();
            let stable = game.object(victim).unwrap().stable_id;
            game.set_current_controller(victim, B).unwrap();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            if indestructible {
                grant(
                    &mut game,
                    source,
                    victim,
                    ironsmith::static_abilities::StaticAbility::indestructible(),
                );
            }
            dm.targets = vec![Target::Object(victim)];
            activate(&mut game, source, activated(&definition), &mut dm);
            game.set_current_controller(victim, C).unwrap();
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.player(C).unwrap().life, 18);
            assert_eq!(game.player(B).unwrap().life, 20);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(current).unwrap().zone,
                if indestructible {
                    Zone::Battlefield
                } else {
                    Zone::Graveyard
                }
            );
        }
    }
}

#[test]
fn unerring_sling_reads_the_paid_creature_live_then_exact_lki_without_blink_following() {
    for definition in definitions("Unerring Sling") {
        for depart in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let cost_object = game.create_object_from_definition(
                &vanilla("Tapped cost creature", "{2}", "Human", 2, 4),
                A,
                Zone::Battlefield,
            );
            let flyer = compile_to_runtime_definition(
                "Flying attacker",
                "Mana cost: {3}\nType: Creature — Bird\nPower/Toughness: 1/20\nFlying",
                false,
            )
            .unwrap();
            let victim = game.create_object_from_definition(&flyer, B, Zone::Battlefield);
            game.remove_summoning_sickness(victim);
            game.turn.active_player = B;
            game.turn.phase = Phase::Combat;
            let mut combat = ironsmith::combat_state::CombatState::default();
            ironsmith::combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(victim, ironsmith::combat_state::AttackTarget::Player(A))],
            )
            .unwrap();
            game.combat = Some(combat);
            game.turn.priority_player = Some(A);
            grant(
                &mut game,
                source,
                source,
                ironsmith::static_abilities::StaticAbility::lifelink(),
            );
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                objects: vec![cost_object],
                ..Default::default()
            };
            activate(&mut game, source, activated(&definition), &mut dm);
            assert!(game.is_tapped(cost_object));
            assert!(game.is_tapped(source));
            apply(
                &mut game,
                source,
                Effect::pump(
                    3,
                    0,
                    ChooseSpec::SpecificObject(cost_object),
                    Until::EndOfTurn,
                ),
            );
            if depart {
                let grave = game
                    .move_object_by_game_rule(cost_object, Zone::Graveyard)
                    .unwrap();
                let blink = game
                    .move_object_by_game_rule(grave, Zone::Battlefield)
                    .unwrap();
                apply(
                    &mut game,
                    source,
                    Effect::pump(50, 50, ChooseSpec::SpecificObject(blink), Until::EndOfTurn),
                );
                game.move_object_by_game_rule(blink, Zone::Graveyard)
                    .unwrap();
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.damage_on(victim), 5);
            assert_eq!(
                game.player(A).unwrap().life,
                25,
                "the artifact is the damage source and has lifelink"
            );
        }
    }
}

#[test]
fn compel_brutality_binds_power_or_loyalty_to_the_announced_damage_source() {
    for definition in definitions("Compel Brutality") {
        for mode in [0, 1] {
            let mut game = game();
            let source_def = if mode == 0 {
                vanilla("Damage creature", "{2}", "Human", 3, 3)
            } else {
                compile_to_runtime_definition(
                    "Damage planeswalker",
                    "Mana cost: {3}\nType: Planeswalker — Jace\nLoyalty: 3",
                    false,
                )
                .unwrap()
            };
            let actor = game.create_object_from_definition(&source_def, A, Zone::Battlefield);
            let victim = game.create_object_from_definition(
                &vanilla("Damage recipient", "{4}", "Human", 1, 20),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Object(actor), Target::Object(victim)],
                mode: Some(mode),
                ..Default::default()
            };
            let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            if mode == 0 {
                apply(
                    &mut game,
                    spell,
                    Effect::pump(2, 0, ChooseSpec::SpecificObject(actor), Until::EndOfTurn),
                );
            } else {
                apply(
                    &mut game,
                    spell,
                    Effect::put_counters(
                        ironsmith::object::CounterType::Loyalty,
                        4,
                        ChooseSpec::SpecificObject(actor),
                    ),
                );
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.damage_on(victim), if mode == 0 { 5 } else { 7 });
        }
    }
}
