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
        "../../../fixtures/prior_damage_quantities.json.fixture"
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
        for player in [A, B, C] {
            game.player_mut(player).unwrap().mana_pool.add(color, 20);
        }
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
    kicker_payments: usize,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        true
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
        if context.description.starts_with("Choose optional costs for") {
            let legal = context
                .options
                .iter()
                .filter(|o| o.legal)
                .collect::<Vec<_>>();
            if legal.len() == 1 {
                return vec![legal[0].index; self.kicker_payments];
            }
            return legal
                .iter()
                .take(self.kicker_payments)
                .map(|o| o.index)
                .collect();
        }
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
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm)
        .unwrap();
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
    cast_for(game, definition, A, method, dm)
}
fn cast_for(
    game: &mut GameState,
    definition: &CardDefinition,
    caster: PlayerId,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, caster, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(
        compute_legal_actions(game, caster)
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
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}

fn body(name: &str, text: &str) -> CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap()
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for _ in 0..count {
        game.create_object_from_definition(
            &vanilla("Library card", "{1}", "Human", 1, 1),
            player,
            Zone::Library,
        );
    }
}
fn replace_bobs_draws(game: &mut GameState, source: ObjectId) {
    apply(
        game,
        source,
        Effect::new(ironsmith::effects::RegisterDrawReplacementEffect::new(
            PlayerFilter::Specific(B),
            vec![Effect::gain_life_player(
                1,
                ChooseSpec::Player(PlayerFilter::Specific(B)),
            )],
            ironsmith::effects::ReplacementApplyMode::UntilEndOfTurn,
        )),
    );
}
#[test]
fn four_exact_metadata_cards_and_referenced_kicks_survive_artifact_round_trip() {
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
            if definition.card.name == "Rumbling Aftershocks" {
                let debug = format!("{:?}", definition.abilities);
                assert!(debug.contains("KicksPaidOf"), "{debug}");
                assert!(
                    !debug.contains("KickCount"),
                    "the quantity must belong to the spell"
                );
            }
        }
    }
}
#[test]
fn cerebral_vortex_counts_its_participants_completed_draws_this_turn() {
    for definition in definitions("Cerebral Vortex") {
        for replaced in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(
                &vanilla("Witness", "{1}", "Human", 1, 10),
                A,
                Zone::Battlefield,
            );
            library(&mut game, A, 10);
            library(&mut game, B, 10);
            apply(&mut game, source, Effect::draw(5));
            apply(
                &mut game,
                source,
                Effect::target_draws(1, PlayerFilter::Specific(B)),
            );
            if replaced {
                replace_bobs_draws(&mut game, source);
            }
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Player(B)],
                    ..Default::default()
                },
            );
            resolve(&mut game, &mut Choices::default());
            assert_eq!(
                game.player(B).unwrap().hand.len(),
                if replaced { 1 } else { 3 }
            );
            assert_eq!(game.player(B).unwrap().life, if replaced { 21 } else { 17 });
            assert_eq!(game.player(A).unwrap().life, 20);
            game.next_turn();
            let mut dm = SelectFirstDecisionMaker;
            let value = ironsmith_core::Value::TurnHistoryCount(
                ironsmith_core::TurnHistoryCount::CardsDrawn(PlayerFilter::Specific(B)),
            );
            assert_eq!(
                ironsmith::effects::helpers::resolve_value(
                    &game,
                    &value,
                    &EffectContext::new(source, A, &mut dm)
                )
                .unwrap(),
                0
            );
        }
    }
}
#[test]
fn essence_backlash_uses_the_exact_spell_power_and_controller_even_if_countering_fails() {
    for definition in definitions("Essence Backlash") {
        for cannot_counter in [false, true] {
            let mut game = game();
            game.turn.active_player = B;
            game.turn.priority_player = Some(B);
            let text = format!(
                "Mana cost: {{2}}\nType: Creature — Human\nPower/Toughness: 6/7\n{}",
                if cannot_counter {
                    "This spell can't be countered."
                } else {
                    ""
                }
            );
            let creature = body("Powerful spell", &text);
            let spell = cast_for(
                &mut game,
                &creature,
                B,
                CastingMethod::Normal,
                &mut Choices::default(),
            );
            game.turn.priority_player = Some(A);
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Object(spell)],
                    ..Default::default()
                },
            );
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.player(B).unwrap().life, 14);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(
                game.stack.iter().any(|entry| entry.object_id == spell),
                cannot_counter
            );
        }
    }
}
#[test]
fn countered_spell_power_is_its_calculated_stack_lki_not_the_destination_graveyard_value() {
    for definition in definitions("Essence Backlash") {
        let mut game = game();
        game.turn.active_player = B;
        game.turn.priority_player = Some(B);
        for _ in 0..2 {
            game.create_object_from_definition(
                &vanilla("Dead creature", "{1}", "Human", 1, 1),
                B,
                Zone::Graveyard,
            );
        }
        let creature = body(
            "Graveyard-size spell",
            "Mana cost: {2}\nType: Creature — Avatar\nPower/Toughness: */*\nThis creature's power and toughness are each equal to the number of creature cards in your graveyard.",
        );
        let spell = cast_for(
            &mut game,
            &creature,
            B,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        assert_eq!(game.current_power(spell), Some(2));
        game.turn.priority_player = Some(A);
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices {
                targets: vec![Target::Object(spell)],
                ..Default::default()
            },
        );
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.player(B).unwrap().graveyard.len(), 3);
        assert_eq!(
            game.player(B).unwrap().life,
            18,
            "damage reads the last stack incarnation before it joined the graveyard"
        );
    }
}
#[test]
fn malignant_growth_counts_only_cards_actually_drawn_by_its_opponents_trigger() {
    for definition in definitions("Malignant Growth") {
        for replaced in [false, true] {
            let mut game = game();
            let mut dm = Choices::default();
            library(&mut game, B, 10);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            // Both printed upkeep abilities execute, including real upkeep payment.
            queue_event(
                &mut game,
                TriggerEvent::new_with_provenance(
                    ironsmith::events::phase::BeginningOfUpkeepEvent::new(A),
                    Default::default(),
                ),
                &mut dm,
            );
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&ironsmith::CounterType::Growth)
                    .copied(),
                Some(1)
            );
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&ironsmith::CounterType::Age)
                    .copied(),
                Some(1)
            );
            apply(
                &mut game,
                source,
                Effect::put_counters(
                    ironsmith::CounterType::Growth,
                    1,
                    ChooseSpec::SpecificObject(source),
                ),
            );
            // One unrelated draw this turn must not be added to the prior action metric.
            apply(
                &mut game,
                source,
                Effect::target_draws(1, PlayerFilter::Specific(B)),
            );
            game.turn.active_player = B;
            queue_event(
                &mut game,
                TriggerEvent::new_with_provenance(
                    ironsmith::events::phase::BeginningOfDrawStepEvent::new(B),
                    Default::default(),
                ),
                &mut dm,
            );
            if replaced {
                replace_bobs_draws(&mut game, source);
            }
            game.set_current_controller(source, C).unwrap();
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.player(B).unwrap().hand.len(),
                if replaced { 1 } else { 3 }
            );
            assert_eq!(game.player(B).unwrap().life, if replaced { 22 } else { 18 });
            assert_eq!(game.player(C).unwrap().life, 20);
        }
    }
}
#[test]
fn rumbling_aftershocks_counts_actual_kicker_and_multikicker_payments_on_the_cast_spell() {
    for definition in definitions("Rumbling Aftershocks") {
        for (keyword, payments) in [
            ("Kicker {1} and/or {2}", 0),
            ("Kicker {1} and/or {2}", 1),
            ("Kicker {1} and/or {2}", 2),
            ("Multikicker {1}", 3),
        ] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let kicked = body(
                "Kicked witness",
                &format!(
                    "Mana cost: {{1}}\nType: Creature — Human\nPower/Toughness: 1/1\n{keyword}"
                ),
            );
            let mut dm = Choices {
                targets: vec![Target::Player(B)],
                kicker_payments: payments,
                ..Default::default()
            };
            let spell = cast(&mut game, &kicked, CastingMethod::Normal, &mut dm);
            assert_eq!(
                game.object(spell).unwrap().optional_costs_paid.kick_count(),
                payments as u32
            );
            assert_eq!(game.stack.len(), if payments == 0 { 1 } else { 2 });
            if payments > 0 {
                // Neither controller changes nor departure of the enchantment
                // may replace the cast event's exact spell/payment reference.
                game.set_current_controller(source, C).unwrap();
                game.move_object_by_game_rule(source, Zone::Graveyard)
                    .unwrap();
                resolve(&mut game, &mut dm);
                assert_eq!(game.player(B).unwrap().life, 20 - payments as i32);
            }
        }
    }
}
#[test]
fn referenced_kick_metadata_uses_departure_lki_and_never_follows_a_new_incarnation() {
    let mut game = game();
    let creature = body(
        "Recast witness",
        "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1\nMultikicker {1}",
    );
    let spell = cast(
        &mut game,
        &creature,
        CastingMethod::Normal,
        &mut Choices {
            kicker_payments: 2,
            ..Default::default()
        },
    );
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(spell).unwrap(),
        &game,
    );
    let old_stable = snapshot.stable_id;
    let fresh = game.move_object_by_game_rule(spell, Zone::Hand).unwrap();
    assert_eq!(game.object(fresh).unwrap().stable_id, old_stable);
    // The new incarnation's costs are empty, while the original stack object
    // retains its paid 2 through a typed tag snapshot.
    assert_eq!(
        game.object(fresh).unwrap().optional_costs_paid.kick_count(),
        0
    );
    let tag: ironsmith::tag::TagKey = "original_spell".into();
    let value = ironsmith_core::Value::KicksPaidOf(Box::new(ChooseSpec::Tagged(tag.clone())));
    let mut dm = SelectFirstDecisionMaker;
    let ctx = EffectContext::new(fresh, A, &mut dm)
        .with_tagged_objects([(tag, vec![snapshot])].into_iter().collect());
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(),
        2
    );
}
