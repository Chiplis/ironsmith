//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::LinkedFaceLayout;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{NumberContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::events::phase::BeginningOfUpkeepEvent;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::{AttackEventTarget, TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/event_object_quantities.json.fixture"
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
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
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
fn snapshot(game: &GameState, id: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object_with_calculated_characteristics(game.object(id).unwrap(), game)
}
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
    x: u32,
    prefer_life: bool,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
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
        if let Some(id) = self.target {
            assert!(
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&Target::Object(id)))
            );
            vec![Target::Object(id)]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
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
fn tokens(game: &GameState, controller: PlayerId, subtype: Subtype) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|o| {
                o.kind == ObjectKind::Token
                    && game.current_controller(*id) == Some(controller)
                    && game.calculated_subtypes(*id).contains(&subtype)
            })
        })
        .collect()
}
fn counter_count(game: &GameState, id: ObjectId) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}

#[test]
fn all_eight_exact_cards_compile_without_fallback_and_keep_typed_quantities() {
    assert_eq!(fixtures().len(), 8);
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert_eq!(definition.card.name, name);
            let debug = format!("{definition:?}");
            let expected = match name {
                "Bioplasm" | "Death's Presence" => "PowerOf",
                "Felisa, Fang of Silverquill" => "CountersOn",
                "Infernal Genesis" => "FirstManaValue",
                "Prossh, Skyraider of Kher" => "ManaSpentToCast",
                _ => "ManaValueOf",
            };
            assert!(debug.contains(expected), "{name}: {debug}");
            assert!(
                !debug.contains("PendingPriorEffectMetric"),
                "{name}: {debug}"
            );
        }
    }
}

#[test]
fn death_presence_reads_calculated_simultaneous_departures_not_new_incarnations() {
    for definition in definitions("Death's Presence") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let human = vanilla("Dying Human", "{2}", "Human", 2, 2);
        let first = game.create_object_from_definition(&human, A, Zone::Battlefield);
        let second = game.create_object_from_definition(&human, A, Zone::Battlefield);
        let first_stable = game.object(first).unwrap().stable_id;
        let recipient = game.create_object_from_definition(
            &vanilla("Survivor", "{1}", "Elf", 1, 1),
            A,
            Zone::Battlefield,
        );
        apply(
            &mut game,
            source,
            Effect::pump(3, 3, ChooseSpec::SpecificObject(first), Until::EndOfTurn),
        );
        apply(
            &mut game,
            source,
            Effect::plus_one_counters(5, ChooseSpec::SpecificObject(second)),
        );
        let outcome = apply(
            &mut game,
            source,
            Effect::destroy_all(ObjectFilter::creature().with_subtype(Subtype::Human)),
        );
        let mut dm = Choices {
            target: Some(recipient),
            ..Default::default()
        };
        queue_outcome(&mut game, outcome, &mut dm);
        assert_eq!(game.stack.len(), 2);
        let grave = game.find_object_by_stable_id(first_stable).unwrap();
        let returned = game
            .move_object_by_game_rule(grave, Zone::Battlefield)
            .unwrap();
        apply(
            &mut game,
            source,
            Effect::pump(
                50,
                50,
                ChooseSpec::SpecificObject(returned),
                Until::EndOfTurn,
            ),
        );
        game.move_object_by_game_rule(returned, Zone::Graveyard)
            .unwrap();
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            counter_count(&game, recipient),
            12,
            "5 + 7 at the original simultaneous deaths"
        );
    }
}

#[test]
fn felisa_uses_all_predeath_counter_kinds_and_looks_back_when_she_dies_too() {
    for definition in definitions("Felisa, Fang of Silverquill") {
        let mut game = game();
        let felisa = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = game.create_object_from_definition(
            &vanilla("Counted counters", "{2}", "Human", 2, 2),
            A,
            Zone::Battlefield,
        );
        apply(
            &mut game,
            felisa,
            Effect::plus_one_counters(2, ChooseSpec::SpecificObject(victim)),
        );
        apply(
            &mut game,
            felisa,
            Effect::put_counters(CounterType::Charge, 3, ChooseSpec::SpecificObject(victim)),
        );
        let outcome = apply(
            &mut game,
            felisa,
            Effect::destroy_all(ObjectFilter::creature()),
        );
        let mut dm = Choices::default();
        queue_outcome(&mut game, outcome, &mut dm);
        resolve_all(&mut game, &mut dm);
        let inklings = tokens(&game, A, Subtype::Inkling);
        assert_eq!(inklings.len(), 5);
        for id in inklings {
            assert!(game.is_tapped(id));
            assert_eq!(game.current_power(id), Some(2));
            assert_eq!(game.current_toughness(id), Some(1));
            assert!(game.current_has_static_ability_id(
                id,
                ironsmith::static_abilities::StaticAbilityId::Flying
            ));
        }
    }
}

#[test]
fn infernal_genesis_uses_the_milled_card_and_that_upkeep_players_controller() {
    for definition in definitions("Infernal Genesis") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let milled = vanilla("Milled five", "{5}", "Human", 1, 1);
        game.create_object_from_definition(&milled, B, Zone::Library);
        let event =
            TriggerEvent::new_with_provenance(BeginningOfUpkeepEvent::new(B), Default::default());
        let mut dm = Choices::default();
        assert_eq!(queue_event(&mut game, event, &mut dm), 1);
        game.move_object_by_game_rule(source, Zone::Graveyard)
            .unwrap();
        resolve_all(&mut game, &mut dm);
        assert_eq!(tokens(&game, B, Subtype::Minion).len(), 5);
        assert!(tokens(&game, A, Subtype::Minion).is_empty());
    }
}

#[test]
fn bioplasm_uses_two_independent_exiled_characteristics_and_pump_expires() {
    for definition in definitions("Bioplasm") {
        for creature_card in [true, false] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let card = if creature_card {
                vanilla("Different stats", "{3}", "Human", 2, 7)
            } else {
                compile_to_runtime_definition(
                    "Noncreature",
                    "Mana cost: {3}\nType: Artifact",
                    false,
                )
                .unwrap()
            };
            game.create_object_from_definition(&card, A, Zone::Library);
            let event = TriggerEvent::new_with_provenance(
                ironsmith::events::combat::CreatureAttackedEvent::new(
                    source,
                    AttackEventTarget::Player(B),
                ),
                Default::default(),
            );
            let mut dm = Choices::default();
            assert_eq!(queue_event(&mut game, event, &mut dm), 1);
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                (game.current_power(source), game.current_toughness(source)),
                if creature_card {
                    (Some(6), Some(11))
                } else {
                    (Some(4), Some(4))
                }
            );
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(
                (game.current_power(source), game.current_toughness(source)),
                (Some(4), Some(4))
            );
        }
    }
}

#[test]
fn narci_keeps_the_exact_saga_after_sacrifice_and_blink() {
    for definition in definitions("Narci, Fable Singer") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let saga = compile_to_runtime_definition(
            "Saga quantity fixture",
            "Mana cost: {5}\nType: Enchantment — Saga\nI — You gain 1 life.",
            false,
        )
        .unwrap();
        let saga = game.create_object_from_definition(&saga, A, Zone::Battlefield);
        let event = TriggerEvent::new_with_provenance(
            ironsmith::events::other::ChapterAbilityResolvedEvent::new(saga, A, true),
            Default::default(),
        )
        .with_source_snapshot(snapshot(&game, saga));
        let mut dm = Choices::default();
        assert_eq!(queue_event(&mut game, event, &mut dm), 1);
        let grave = game
            .move_object_by_game_rule(saga, Zone::Graveyard)
            .unwrap();
        let returned = game
            .move_object_by_game_rule(grave, Zone::Battlefield)
            .unwrap();
        game.move_object_by_game_rule(returned, Zone::Exile)
            .unwrap();
        game.move_object_by_game_rule(source, Zone::Graveyard)
            .unwrap();
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 25);
        assert_eq!(game.player(B).unwrap().life, 15);
    }
}

#[test]
fn cast_trigger_values_include_announced_x_after_the_spell_is_countered() {
    for (observer, kind, subtype) in [
        ("Ovika, Enigma Goliath", "Artifact", Subtype::Goblin),
        (
            "Pure Reflection",
            "Creature — Human\nPower/Toughness: 1/1",
            Subtype::Reflection,
        ),
    ] {
        for definition in definitions(observer) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            if observer == "Pure Reflection" {
                let reflection = vanilla("Old Reflection", "{9}", "Reflection", 9, 9);
                game.create_object_from_definition(&reflection, B, Zone::Battlefield);
            }
            let spell = compile_to_runtime_definition(
                "Announced X",
                format!("Mana cost: {{X}}{{2}}\nType: {kind}"),
                false,
            )
            .unwrap();
            let mut dm = Choices {
                x: 3,
                ..Default::default()
            };
            let spell = cast(&mut game, &spell, CastingMethod::Normal, &mut dm);
            assert_eq!(game.object(spell).unwrap().x_value, Some(3));
            assert_eq!(game.stack.len(), 2);
            apply(
                &mut game,
                source,
                Effect::counter(ChooseSpec::SpecificObject(spell)),
            );
            assert_eq!(game.stack.len(), 1);
            resolve_all(&mut game, &mut dm);
            let made = tokens(&game, A, subtype);
            if observer == "Ovika, Enigma Goliath" {
                assert_eq!(made.len(), 5);
                for id in &made {
                    assert!(game.current_has_static_ability_id(
                        *id,
                        ironsmith::static_abilities::StaticAbilityId::Haste
                    ));
                }
                ironsmith::turn::execute_cleanup_step(&mut game);
                for id in made {
                    assert!(!game.current_has_static_ability_id(
                        id,
                        ironsmith::static_abilities::StaticAbilityId::Haste
                    ));
                }
            } else {
                assert_eq!(made.len(), 1);
                assert_eq!(game.current_power(made[0]), Some(5));
                assert_eq!(game.current_toughness(made[0]), Some(5));
            }
        }
    }
}

#[test]
fn ovika_reads_only_the_cast_split_half_after_it_leaves_the_stack() {
    for definition in definitions("Ovika, Enigma Goliath") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut front = compile_to_runtime_definition(
            "One half",
            "Mana cost: {1}\nType: Sorcery\nYou gain 1 life.",
            false,
        )
        .unwrap();
        let mut back = compile_to_runtime_definition(
            "Four half",
            "Mana cost: {4}\nType: Sorcery\nYou gain 4 life.",
            false,
        )
        .unwrap();
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.clone());
        front.card.linked_face_layout = LinkedFaceLayout::Split;
        back.card.other_face = Some(front.card.id);
        back.card.other_face_name = Some(front.card.name.clone());
        back.card.linked_face_layout = LinkedFaceLayout::Split;
        game.register_linked_face_definition(&back);
        let mut dm = Choices::default();
        let spell = cast(&mut game, &front, CastingMethod::SplitOtherHalf, &mut dm);
        assert_eq!(game.object(spell).unwrap().name, "Four half");
        apply(
            &mut game,
            source,
            Effect::counter(ChooseSpec::SpecificObject(spell)),
        );
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            tokens(&game, A, Subtype::Goblin).len(),
            4,
            "not the front's 1 or combined 5"
        );
    }
}

#[test]
fn pure_reflection_reads_face_down_mana_value_even_after_the_spell_is_gone() {
    for definition in definitions("Pure Reflection") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let anthem = compile_to_runtime_definition(
            "Keep zero token alive",
            "Type: Enchantment\nCreatures you control get +1/+1.",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&anthem, A, Zone::Battlefield);
        let spell = compile_to_runtime_definition(
            "Hidden six",
            "Mana cost: {6}\nType: Creature — Human\nPower/Toughness: 6/6\nMorph {2}",
            false,
        )
        .unwrap();
        let mut dm = Choices::default();
        let spell = cast(&mut game, &spell, CastingMethod::FaceDown, &mut dm);
        apply(
            &mut game,
            source,
            Effect::counter(ChooseSpec::SpecificObject(spell)),
        );
        resolve_all(&mut game, &mut dm);
        let reflection = tokens(&game, A, Subtype::Reflection);
        assert_eq!(reflection.len(), 1);
        assert_eq!(
            game.current_power(reflection[0]),
            Some(1),
            "zero mana value + anthem, not printed six or three mana paid"
        );
        assert_eq!(game.current_toughness(reflection[0]), Some(1));
    }
}

#[test]
fn prossh_counts_actual_paid_mana_including_tax_not_printed_cost() {
    for definition in definitions("Prossh, Skyraider of Kher") {
        for tax in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(
                &vanilla("Counter source", "{1}", "Human", 1, 1),
                A,
                Zone::Battlefield,
            );
            if tax {
                let taxer = compile_to_runtime_definition(
                    "Payment tax",
                    "Type: Enchantment\nCreature spells you cast cost {2} more to cast.",
                    false,
                )
                .unwrap();
                game.create_object_from_definition(&taxer, A, Zone::Battlefield);
            }
            let mut dm = Choices::default();
            let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            let expected = if tax { 8 } else { 6 };
            assert_eq!(
                game.object(spell).unwrap().mana_spent_to_cast.total(),
                expected
            );
            apply(
                &mut game,
                source,
                Effect::counter(ChooseSpec::SpecificObject(spell)),
            );
            resolve_all(&mut game, &mut dm);
            let kobolds = tokens(&game, A, Subtype::Kobold);
            assert_eq!(kobolds.len(), expected as usize);
            for id in kobolds {
                assert_eq!(game.object(id).unwrap().name, "Kobolds of Kher Keep");
            }
        }
    }
}

#[test]
fn spent_mana_reference_excludes_life_and_convoke_payments() {
    // Same source-bound cast quantity as Prossh, with a payment-only probe
    // making both non-mana payment channels observable in a single cast.
    let text = "Mana cost: {2}{G/P}\nType: Creature — Human\nPower/Toughness: 2/2\nConvoke\nWhen you cast this spell, create X 1/1 red Goblin creature tokens, where X is the amount of mana spent to cast it.";
    for definition in definitions_text("Nonmana payments", text) {
        let mut game = game();
        game.player_mut(A).unwrap().mana_pool = Default::default();
        let a = game.create_object_from_definition(
            &vanilla("Convoke one", "{2}", "Human", 1, 1),
            A,
            Zone::Battlefield,
        );
        let b = game.create_object_from_definition(
            &vanilla("Convoke two", "{2}", "Human", 1, 1),
            A,
            Zone::Battlefield,
        );
        let mut dm = Choices {
            prefer_life: true,
            ..Default::default()
        };
        let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 18);
        assert!(game.is_tapped(a) && game.is_tapped(b));
        assert_eq!(game.object(spell).unwrap().mana_spent_to_cast.total(), 0);
        resolve(&mut game, &mut dm); // Cast trigger, while the spell is still on the stack.
        assert!(tokens(&game, A, Subtype::Goblin).is_empty());
    }
}

#[test]
fn a_distinct_event_source_is_not_substituted_for_a_missing_event_object() {
    let mut game = game();
    let source = game.create_object_from_definition(
        &vanilla("Event source", "{8}", "Human", 8, 8),
        A,
        Zone::Battlefield,
    );
    let named = game.create_object_from_definition(
        &vanilla("Event object", "{2}", "Human", 2, 2),
        A,
        Zone::Battlefield,
    );
    let event = TriggerEvent::new_with_provenance(
        ironsmith::events::other::ChapterAbilityResolvedEvent::new(named, A, true),
        Default::default(),
    )
    .with_source_snapshot(snapshot(&game, source));
    game.move_object_by_game_rule(named, Zone::Exile).unwrap();
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm).with_triggering_event(event);
    execute_effect(
        &mut game,
        &Effect::new(ironsmith::effects::TagTriggeringObjectEffect::new(
            "missing",
        )),
        &mut ctx,
    )
    .unwrap();
    assert!(
        ctx.get_tagged_all("missing")
            .is_none_or(|objects| objects.is_empty())
    );
}
