//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{NumberContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{AttackEventTarget, TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/scalar_counter_quantities.json.fixture"
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
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
    x: u32,
    prefer_life: bool,
    pay_energy: bool,
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
        if self.pay_energy {
            if let Some(option) = context.options.iter().find(|option| {
                option.legal
                    && (option.description.to_ascii_lowercase().contains("energy")
                        || option.description.to_ascii_lowercase().contains("{e}"))
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
        .unwrap_or_else(|| {
            panic!(
                "cast did not publish a spell: {progress:?}; source zone: {:?}",
                game.object(id).map(|o| o.zone)
            )
        })
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
fn energy(game: &GameState, player: PlayerId) -> u32 {
    game.player(player)
        .unwrap()
        .counter_count(CounterType::Energy)
}
fn attack(game: &mut GameState, source: ObjectId, dm: &mut Choices) -> usize {
    queue_event(
        game,
        TriggerEvent::new_with_provenance(
            ironsmith::events::combat::CreatureAttackedEvent::new(
                source,
                AttackEventTarget::Player(B),
            ),
            Default::default(),
        ),
        dm,
    )
}

#[test]
fn four_full_metadata_cards_keep_typed_scalars_counters_and_artifact_round_trips() {
    let rows = fixtures();
    assert_eq!(rows.len(), 5);
    for row in rows
        .iter()
        .filter(|row| row["proposed_coverage"] == "complete")
    {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let debug = format!("{definition:?}");
            let expected = if name.starts_with("Nissa") {
                "Loyalty"
            } else if name.starts_with("Rootwire") {
                "Scaled"
            } else if name.starts_with("Vault") {
                "PlayerCounters(Any, Rad)"
            } else {
                "PlayerCounters(You, Energy)"
            };
            assert!(debug.contains(expected), "{name}: {debug}");
            if name.starts_with("Nissa") {
                assert_eq!(definition.card.loyalty, Some(7));
            }
            if name.starts_with("Vault") {
                let rendered =
                    ironsmith_text::compiled_text::unprocessed_compiled_lines(&definition)
                        .join(" ");
                assert!(
                    rendered.contains("total number of rad counters among players"),
                    "{rendered}"
                );
            }
        }
    }
    assert_eq!(
        rows.iter()
            .filter(|row| row["proposed_coverage"] == "pending")
            .count(),
        1
    );
}

#[test]
fn nissa_reads_resolution_loyalty_and_the_exact_departure_after_blink() {
    for definition in definitions("Nissa, Ascended Animist") {
        for depart in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut dm = Choices::default();
            activate(&mut game, source, activated(&definition), &mut dm);
            assert_eq!(game.object(source).unwrap().loyalty(), Some(8));
            let expected = if depart {
                apply(
                    &mut game,
                    source,
                    Effect::remove_counters(
                        CounterType::Loyalty,
                        3,
                        ChooseSpec::SpecificObject(source),
                    ),
                );
                let grave = game
                    .move_object_by_game_rule(source, Zone::Graveyard)
                    .unwrap();
                let returned = game
                    .move_object_by_game_rule(grave, Zone::Battlefield)
                    .unwrap();
                apply(
                    &mut game,
                    returned,
                    Effect::put_counters(
                        CounterType::Loyalty,
                        20,
                        ChooseSpec::SpecificObject(returned),
                    ),
                );
                game.move_object_by_game_rule(returned, Zone::Graveyard)
                    .unwrap();
                5
            } else {
                apply(
                    &mut game,
                    source,
                    Effect::put_counters(
                        CounterType::Loyalty,
                        2,
                        ChooseSpec::SpecificObject(source),
                    ),
                );
                10
            };
            resolve_all(&mut game, &mut dm);
            let horrors = tokens(&game, A, Subtype::Horror);
            assert_eq!(horrors.len(), 1);
            assert_eq!(game.current_power(horrors[0]), Some(expected));
            assert_eq!(game.current_toughness(horrors[0]), Some(expected));
            assert!(
                game.calculated_subtypes(horrors[0])
                    .contains(&Subtype::Phyrexian)
            );
        }
    }
}

#[test]
fn rootwire_scales_actual_prototype_and_normal_sacrifice_lki_and_expires_haste() {
    for definition in definitions("Rootwire Amalgam") {
        for prototype in [false, true] {
            let mut game = game();
            let mut dm = Choices::default();
            let method = if prototype {
                CastingMethod::Alternative(
                    definition
                        .alternative_casts
                        .iter()
                        .position(|method| method.name().eq_ignore_ascii_case("prototype"))
                        .unwrap(),
                )
            } else {
                CastingMethod::Normal
            };
            let spell = cast(&mut game, &definition, method, &mut dm);
            let stable = game.object(spell).unwrap().stable_id;
            resolve_all(&mut game, &mut dm);
            let source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.current_power(source),
                Some(if prototype { 2 } else { 5 })
            );
            apply(
                &mut game,
                source,
                Effect::pump(2, 2, ChooseSpec::SpecificObject(source), Until::EndOfTurn),
            );
            activate(&mut game, source, activated(&definition), &mut dm);
            let grave = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard);
            let returned = game
                .move_object_by_game_rule(grave, Zone::Battlefield)
                .unwrap();
            apply(
                &mut game,
                returned,
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
            let golems = tokens(&game, A, Subtype::Golem);
            assert_eq!(golems.len(), 1);
            let expected = if prototype { 12 } else { 21 };
            assert_eq!(game.current_power(golems[0]), Some(expected));
            assert_eq!(game.current_toughness(golems[0]), Some(expected));
            assert!(game.current_has_static_ability_id(
                golems[0],
                ironsmith::static_abilities::StaticAbilityId::Haste
            ));
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert!(!game.current_has_static_ability_id(
                golems[0],
                ironsmith::static_abilities::StaticAbilityId::Haste
            ));
        }
    }
}

#[test]
fn razorfield_pumps_the_attacker_using_energy_after_its_gain() {
    for definition in definitions("Razorfield Ripper") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        apply(&mut game, source, Effect::energy_counters(2));
        let mut dm = Choices::default();
        assert_eq!(attack(&mut game, source, &mut dm), 1);
        apply(
            &mut game,
            source,
            Effect::new(ironsmith::effects::PayEnergyEffect::new(
                1,
                ChooseSpec::SpecificPlayer(A),
            )),
        );
        resolve_all(&mut game, &mut dm);
        assert_eq!(energy(&game, A), 2);
        assert_eq!(game.current_power(source), Some(5));
        assert_eq!(game.current_toughness(source), Some(5));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.current_power(source), Some(3));
    }
}

#[test]
fn razorfield_reconfigure_alternatives_and_attached_attacker_preserve_controller_scope() {
    for definition in definitions("Razorfield Ripper") {
        for (pay_energy, steal) in [(false, false), (true, false), (true, true)] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let host = game.create_object_from_definition(
                &vanilla("Equipment host", "{1}", "Human", 1, 1),
                A,
                Zone::Battlefield,
            );
            apply(&mut game, source, Effect::energy_counters(5));
            let mut dm = Choices {
                target: Some(host),
                pay_energy,
                ..Default::default()
            };
            activate(&mut game, source, activated(&definition), &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(energy(&game, A), if pay_energy { 2 } else { 5 });
            assert!(
                !game
                    .current_card_types(source)
                    .unwrap()
                    .contains(&ironsmith::CardType::Creature)
            );
            let controller = if steal {
                game.set_current_controller(source, B).unwrap();
                apply(
                    &mut game,
                    source,
                    Effect::energy_counters_player(1, PlayerFilter::Specific(B)),
                );
                B
            } else {
                A
            };
            let before = energy(&game, controller);
            assert_eq!(attack(&mut game, host, &mut dm), 1);
            resolve_all(&mut game, &mut dm);
            assert_eq!(energy(&game, controller), before + 1);
            assert_eq!(game.current_power(host), Some(2 + before as i32));
            assert_eq!(game.current_toughness(host), Some(2 + before as i32));
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(game.current_power(host), Some(1));
        }
    }
}

#[test]
fn vault_chapters_sum_all_players_current_rad_counters_and_keep_token_types() {
    for definition in definitions("Vault 12: The Necropolis") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices::default();
        for chapter in 1..=3 {
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::add_lore_counter_and_check_chapters(
                &mut game, source, &mut queue,
            )
            .unwrap();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            if chapter == 2 {
                // Sample at resolution, not at trigger time. Permanent counters
                // and another player-counter kind are not rad counters.
                apply(
                    &mut game,
                    source,
                    Effect::player_counters(CounterType::Rad, 2, PlayerFilter::Specific(B)),
                );
                apply(
                    &mut game,
                    source,
                    Effect::energy_counters_player(10, PlayerFilter::Specific(B)),
                );
                apply(
                    &mut game,
                    source,
                    Effect::put_counters(CounterType::Rad, 20, ChooseSpec::SpecificObject(source)),
                );
            }
            resolve_all(&mut game, &mut dm);
            if chapter == 1 {
                assert_eq!(game.player(A).unwrap().counter_count(CounterType::Rad), 3);
                assert_eq!(game.player(B).unwrap().counter_count(CounterType::Rad), 3);
            }
            if chapter >= 2 {
                let zombies = tokens(&game, A, Subtype::Zombie);
                assert_eq!(zombies.len(), 8);
                for id in zombies {
                    assert!(game.calculated_subtypes(id).contains(&Subtype::Mutant));
                    assert_eq!(
                        game.current_power(id),
                        Some(if chapter == 2 { 2 } else { 4 })
                    );
                }
            }
        }
    }
}
