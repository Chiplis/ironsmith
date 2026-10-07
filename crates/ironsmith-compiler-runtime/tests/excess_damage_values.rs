//! UNVALIDATED exact-source, artifact, and live-resolution excess-damage regressions.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, TargetsContext,
};
use ironsmith::effect::{Effect, EffectMetric, EffectMetricSource, EffectOutcome, Until, Value};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/excess_damage_values.json.fixture"
    ))
    .unwrap()
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    [direct, restored]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().to_owned());
    definitions_text(name, &lines.join("\n"))
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = PlayerId::from_index(0);
    game.turn.priority_player = Some(PlayerId::from_index(0));
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ] {
        game.player_mut(PlayerId::from_index(0))
            .unwrap()
            .mana_pool
            .add(color, 20);
    }
    game
}
fn creature(game: &mut GameState, controller: usize, p: i32, t: i32, keywords: &str) -> ObjectId {
    let card = compile_to_runtime_definition(
        "Damage fixture",
        format!("Mana cost: {{2}}\nType: Creature — Human\nPower/Toughness: {p}/{t}\n{keywords}"),
        false,
    )
    .unwrap();
    game.create_object_from_definition(
        &card,
        PlayerId::from_index(controller.try_into().unwrap()),
        Zone::Battlefield,
    )
}
fn tokens(game: &GameState, subtype: Subtype) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Token
                    && game.current_controller(object.id) == Some(PlayerId::from_index(0))
                    && game.calculated_subtypes(*id).contains(&subtype)
            })
        })
        .collect()
}
fn counters(game: &GameState, id: ObjectId, kind: CounterType) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&kind)
        .copied()
        .unwrap_or(0)
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    sacrifice: Option<ObjectId>,
    x: u32,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            assert!(self.x <= ctx.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if self.targets.is_empty() {
            return SelectFirstDecisionMaker.decide_targets(game, ctx);
        }
        let mut selected = Vec::new();
        for requirement in &ctx.requirements {
            let target = self
                .targets
                .iter()
                .copied()
                .find(|target| {
                    requirement.legal_targets.contains(target) && !selected.contains(target)
                })
                .unwrap_or_else(|| {
                    panic!("missing distinct legal requested target: {requirement:?}")
                });
            selected.push(target);
        }
        selected
    }
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        // Discover chooses the hand; Bolg accepts its sacrifice.
        !ctx.description.starts_with("Cast ") && ctx.can_accept
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.sacrifice {
            if ctx
                .candidates
                .iter()
                .any(|candidate| candidate.id == id && candidate.legal)
            {
                return vec![id];
            }
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(definition, alice, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: source,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    };
    assert!(
        compute_legal_actions(game, alice)
            .unwrap()
            .contains(&action)
    );
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
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    assert_eq!(game.stack.len(), 1);
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let controller = game.current_controller(source).unwrap();
    let mut dm = SelectFirstDecisionMaker;
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) -> usize {
    let mut queue = TriggerQueue::new();
    // Checked execution already captures some triggers in the original observer frame.
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    let count = queue.entries.len();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    count
}

#[test]
fn seven_full_cards_are_lossless_typed_and_survive_artifacts() {
    assert_eq!(fixtures().len(), 7);
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let debug = format!("{:?}", definition);
            assert!(!debug.contains("PendingEffectMetric"), "{name}: {debug}");
            assert!(
                !debug.contains("PendingPriorEffectMetric"),
                "{name}: {debug}"
            );
            let mut values = Vec::new();
            fn collect(effect: &Effect, values: &mut Vec<Value>) {
                if let Some(effect) = effect.downcast_ref::<ironsmith::effects::AmassEffect>() {
                    values.push(effect.amount.clone());
                }
                if let Some(effect) = effect.downcast_ref::<ironsmith::effects::EmpowerJaceEffect>()
                {
                    values.push(effect.amount.clone());
                }
                if let Some(effect) = effect.downcast_ref::<ironsmith::effects::DiscoverEffect>() {
                    values.push(effect.count.clone());
                }
                if let Some(effect) = effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>()
                {
                    values.push(effect.count.clone());
                }
                effect.visit_child_effects(&mut |effect| collect(effect, values));
            }
            for program in
                definition
                    .spell_effect
                    .iter()
                    .chain(definition.abilities.iter().filter_map(|a| match &a.kind {
                        AbilityKind::Activated(a) => Some(&a.effects),
                        AbilityKind::Triggered(t) => Some(&t.effects),
                        _ => None,
                    }))
            {
                for effect in program.all_effects() {
                    collect(effect, &mut values);
                }
            }
            if name == "Fall of Cair Andros" {
                let trigger = definition
                    .abilities
                    .iter()
                    .find_map(|ability| match &ability.kind {
                        AbilityKind::Triggered(t) => t.trigger.compiled_model(),
                        _ => None,
                    })
                    .unwrap();
                assert!(matches!(
                    trigger.kind,
                    ironsmith_core::trigger_model::TriggerKind::IsDealtDamage {
                        noncombat_only: true,
                        excess_only: true,
                        ..
                    }
                ));
                assert!(values.iter().any(|value| matches!(
                    value.unhinted(),
                    Value::EventValue(ironsmith_core::EventValueSpec::Amount)
                )));
            } else {
                assert!(
                    values.iter().any(|value| matches!(
                        value.unhinted(),
                        Value::EffectMetric {
                            source: EffectMetricSource::Outcome,
                            metric: EffectMetric::ExcessDamage,
                            ..
                        }
                    )),
                    "{name}: {values:?}"
                );
            }
        }
    }
}

#[test]
fn paid_x_damage_creates_actual_excess_tokens_after_marked_damage_and_prevention() {
    for (name, subtype, tapped) in [
        ("Goblin Negotiation", Subtype::Goblin, false),
        ("Hell to Pay", Subtype::Treasure, true),
    ] {
        for definition in definitions(name) {
            for (marked, prevented, x, expected) in [
                (0, 0, 5, 3),
                (1, 0, 5, 4),
                (0, 2, 5, 1),
                (0, 5, 5, 0),
                (0, 0, 2, 0),
                (0, 0, 0, 0),
            ] {
                let mut game = game();
                let source = creature(&mut game, 0, 1, 1, "");
                let victim = creature(&mut game, 1, 2, 2, "Indestructible");
                if marked > 0 {
                    apply(
                        &mut game,
                        source,
                        Effect::deal_damage(marked, ChooseSpec::SpecificObject(victim)),
                    );
                }
                if prevented > 0 {
                    apply(
                        &mut game,
                        source,
                        Effect::prevent_damage(
                            prevented,
                            ChooseSpec::SpecificObject(victim),
                            Until::EndOfTurn,
                        ),
                    );
                }
                let mut dm = Choices {
                    targets: vec![Target::Object(victim)],
                    x,
                    ..Default::default()
                };
                cast(&mut game, &definition, &mut dm);
                resolve(&mut game, &mut dm);
                let made = tokens(&game, subtype);
                assert_eq!(
                    made.len(),
                    expected,
                    "{name}: marked={marked}, prevented={prevented}, X={x}"
                );
                for id in made {
                    assert_eq!(game.is_tapped(id), tapped);
                }
            }
        }
    }
}

#[test]
fn windswift_uses_the_selected_creature_and_deathtouch_lethal_threshold() {
    for definition in definitions("Windswift Slice") {
        for (keyword, expected) in [("", 3), ("Deathtouch", 7)] {
            let mut game = game();
            let source = creature(&mut game, 0, 8, 8, keyword);
            let victim = creature(&mut game, 1, 5, 5, "Indestructible");
            let mut dm = Choices {
                targets: vec![Target::Object(source), Target::Object(victim)],
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(tokens(&game, Subtype::Elf).len(), expected);
            for id in tokens(&game, Subtype::Elf) {
                assert!(game.calculated_subtypes(id).contains(&Subtype::Warrior));
                assert_eq!(game.current_power(id), Some(1));
                assert_eq!(game.current_toughness(id), Some(1));
            }
        }
    }
}

#[test]
fn violent_echoes_empowers_by_actual_excess_and_skips_zero_or_prevented_damage() {
    for definition in definitions("Violent Echoes") {
        for (toughness, prevention, expected) in [(2, 0, 4), (2, 2, 2), (6, 0, 0), (2, 6, 0)] {
            let mut game = game();
            let source = creature(&mut game, 0, 1, 1, "");
            let victim = creature(&mut game, 1, 2, toughness, "Indestructible");
            if prevention > 0 {
                apply(
                    &mut game,
                    source,
                    Effect::prevent_damage(
                        prevention,
                        ChooseSpec::SpecificObject(victim),
                        Until::EndOfTurn,
                    ),
                );
            }
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            let jaces = tokens(&game, Subtype::Jace);
            if expected == 0 {
                assert!(jaces.is_empty(), "zero excess must not run Empower 0");
            } else {
                assert_eq!(jaces.len(), 1);
                assert_eq!(counters(&game, jaces[0], CounterType::Loyalty), expected);
            }
        }
    }
}

#[test]
fn contest_discovers_excess_not_full_damage_or_target_toughness() {
    for definition in definitions("Contest of Claws") {
        let mut game = game();
        let source = creature(&mut game, 0, 7, 7, "");
        let victim = creature(&mut game, 1, 3, 3, "Indestructible");
        // Top-to-bottom: mana value 5, then 4. Discover 4 must skip the 5.
        for (name, cost) in [("Exact excess", "{4}"), ("Too expensive", "{5}")] {
            let card = compile_to_runtime_definition(
                name,
                format!("Mana cost: {cost}\nType: Creature — Human\nPower/Toughness: 1/1"),
                false,
            )
            .unwrap();
            game.create_object_from_definition(&card, PlayerId::from_index(0), Zone::Library);
        }
        let mut dm = Choices {
            targets: vec![Target::Object(source), Target::Object(victim)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        let names = game
            .player(PlayerId::from_index(0))
            .unwrap()
            .hand
            .iter()
            .map(|id| game.object(*id).unwrap().name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["Exact excess"]);
    }
}

#[test]
fn fall_exports_excess_noncombat_amount_and_keeps_controller_scope_after_source_departure() {
    for definition in definitions("Fall of Cair Andros") {
        for (target_controller, combat, damage, expected) in [
            (1, false, 7, 5),
            (0, false, 7, 0),
            (1, true, 7, 0),
            (1, false, 2, 0),
        ] {
            let mut game = game();
            let fall = game.create_object_from_definition(
                &definition,
                PlayerId::from_index(0),
                Zone::Battlefield,
            );
            let source = creature(&mut game, 0, 1, 1, "");
            let victim = creature(&mut game, target_controller, 2, 2, "Indestructible");
            let outcome = apply(
                &mut game,
                source,
                Effect::new(
                    ironsmith::effects::DealDamageEffect::new(
                        damage,
                        ChooseSpec::SpecificObject(victim),
                    )
                    .with_combat(combat),
                ),
            );
            let mut dm = Choices::default();
            let triggered = queue_outcome(&mut game, outcome, &mut dm);
            assert_eq!(triggered, usize::from(expected > 0));
            game.move_object_by_game_rule(fall, Zone::Graveyard)
                .unwrap();
            if expected > 0 {
                resolve(&mut game, &mut dm);
            }
            let armies = tokens(&game, Subtype::Army);
            if expected == 0 {
                assert!(armies.is_empty());
            } else {
                assert_eq!(armies.len(), 1);
                assert_eq!(
                    counters(&game, armies[0], CounterType::PlusOnePlusOne),
                    expected
                );
                assert!(game.calculated_subtypes(armies[0]).contains(&Subtype::Orc));
                assert_eq!(game.object(armies[0]).unwrap().name, "Orc Army Token");
            }
        }
    }
}

#[test]
fn bolg_reflexive_damage_uses_sacrificed_power_then_amasses_only_the_excess() {
    for definition in definitions("Bolg of the North") {
        let mut game = game();
        let sacrifice = creature(&mut game, 0, 7, 7, "");
        let victim = creature(&mut game, 1, 3, 3, "Indestructible");
        let mut dm = Choices {
            targets: vec![Target::Object(victim)],
            sacrifice: Some(sacrifice),
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        // Process the real ETB, then its sacrifice-triggered reflexive ability.
        for _ in 0..3 {
            let mut queue = TriggerQueue::new();
            drain_pending_trigger_events(&mut game, &mut queue);
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            if game.stack_is_empty() {
                break;
            }
            resolve(&mut game, &mut dm);
        }
        let armies = tokens(&game, Subtype::Army);
        assert_eq!(armies.len(), 1);
        assert_eq!(counters(&game, armies[0], CounterType::PlusOnePlusOne), 4);
        assert_eq!(game.object(armies[0]).unwrap().name, "Goblin Army Token");
        assert!(
            game.calculated_subtypes(armies[0])
                .contains(&Subtype::Goblin)
        );
    }
}

#[test]
fn excess_event_binding_survives_body_actions_but_a_local_result_branch_wins() {
    for (body, expected) in [
        (
            "You gain 1 life. Amass Orcs X, where X is that excess damage.",
            5,
        ),
        (
            "This enchantment deals 4 damage to target creature you control. If excess damage was dealt this way, amass Orcs X, where X is that excess damage.",
            1,
        ),
        (
            "This enchantment deals 4 damage to target creature you control. Amass Orcs X, where X is the excess damage dealt this way.",
            1,
        ),
    ] {
        let text = format!(
            "Type: Enchantment\nWhenever a creature an opponent controls is dealt excess noncombat damage, {body}"
        );
        for definition in definitions_text("Local damage context", &text) {
            let mut game = game();
            game.create_object_from_definition(
                &definition,
                PlayerId::from_index(0),
                Zone::Battlefield,
            );
            let source = creature(&mut game, 0, 3, 3, "Indestructible");
            let victim = creature(&mut game, 1, 2, 2, "Indestructible");
            let outcome = apply(
                &mut game,
                source,
                Effect::deal_damage(7, ChooseSpec::SpecificObject(victim)),
            );
            let mut dm = Choices {
                targets: vec![Target::Object(source)],
                ..Default::default()
            };
            assert_eq!(queue_outcome(&mut game, outcome, &mut dm), 1);
            resolve(&mut game, &mut dm);
            let armies = tokens(&game, Subtype::Army);
            assert_eq!(armies.len(), 1);
            assert_eq!(
                counters(&game, armies[0], CounterType::PlusOnePlusOne),
                expected,
                "{body}"
            );
        }
    }
}

#[test]
fn violent_echoes_counts_damage_beyond_planeswalker_loyalty() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType};
    for definition in definitions("Violent Echoes") {
        let mut game = game();
        let walker = CardDefinitionBuilder::new(CardId::new(), "Loyalty fixture")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(2)
            .build();
        let target =
            game.create_object_from_definition(&walker, PlayerId::from_index(1), Zone::Battlefield);
        let mut dm = Choices {
            targets: vec![Target::Object(target)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        let jaces = tokens(&game, Subtype::Jace);
        assert_eq!(jaces.len(), 1);
        assert_eq!(counters(&game, jaces[0], CounterType::Loyalty), 4);
    }
}

#[test]
fn ordinary_damage_life_and_unrelated_prior_actions_do_not_supply_excess() {
    for text in [
        "Type: Enchantment\nWhenever you gain life, amass Orcs X, where X is that excess damage.",
        "Type: Creature — Human\nPower/Toughness: 2/2\nWhenever this creature is dealt damage, amass Orcs X, where X is that excess damage.",
        "Type: Sorcery\nDraw a card. Amass Orcs X, where X is the excess damage dealt this way.",
    ] {
        assert!(
            compile_to_runtime_definition("Wrong excess producer", text, false).is_err(),
            "{text}"
        );
    }
}
