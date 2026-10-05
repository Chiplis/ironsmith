//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/aggregate_damage_quantities.json.fixture"
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
}
impl DecisionMaker for Choices {
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

fn cast_existing(
    game: &mut GameState,
    id: ObjectId,
    from_zone: Zone,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone,
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
fn resource(name: &str, types: &str) -> CardDefinition {
    let pt = if types.contains("Creature") {
        "\nPower/Toughness: 1/1"
    } else {
        ""
    };
    compile_to_runtime_definition(name, format!("Mana cost: {{1}}\nType: {types}{pt}"), false)
        .unwrap()
}
fn owned_spell_zones(game: &mut GameState) {
    for (player, zone, name, types) in [
        (A, Zone::Exile, "Owned instant", "Instant"),
        (A, Zone::Graveyard, "Owned sorcery", "Sorcery"),
        (A, Zone::Graveyard, "Two matching types", "Instant Sorcery"),
        (A, Zone::Graveyard, "Wrong type", "Creature — Human"),
        (B, Zone::Exile, "Wrong owner", "Instant"),
        (A, Zone::Library, "Wrong zone", "Sorcery"),
    ] {
        game.create_object_from_definition(&resource(name, types), player, zone);
    }
}

#[test]
fn seven_full_scalar_damage_cards_transport_typed_counts_and_maximums() {
    assert_eq!(fixtures().len(), 7);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let debug = format!("{definition:?}");
            assert!(!debug.contains("PendingPriorEffectMetric"), "{debug}");
        }
    }
}

#[test]
fn beacon_bolt_counts_owned_zone_union_once_and_includes_the_actual_jump_start_discard() {
    for definition in definitions("Beacon Bolt") {
        for jump_start in [false, true] {
            let mut game = game();
            owned_spell_zones(&mut game);
            let victim = game.create_object_from_definition(
                &vanilla("Damage target", "{3}", "Human", 1, 30),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            };
            let spell = if jump_start {
                let discard = game.create_object_from_definition(
                    &resource("Jump-start payment", "Instant"),
                    A,
                    Zone::Hand,
                );
                dm.objects = vec![discard];
                let method = CastingMethod::Alternative(
                    definition
                        .alternative_casts
                        .iter()
                        .position(|method| method.name().eq_ignore_ascii_case("jump-start"))
                        .unwrap(),
                );
                let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
                cast_existing(&mut game, source, Zone::Graveyard, method, &mut dm)
            } else {
                cast(&mut game, &definition, CastingMethod::Normal, &mut dm)
            };
            let stable = game.object(spell).unwrap().stable_id;
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.damage_on(victim), if jump_start { 4 } else { 3 });
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(current).unwrap().zone,
                if jump_start {
                    Zone::Exile
                } else {
                    Zone::Graveyard
                }
            );
        }
    }
}

#[test]
fn ral_preserves_loyalty_payment_and_reads_owned_spells_at_resolution() {
    for definition in definitions("Ral, Izzet Viceroy") {
        let mut game = game();
        owned_spell_zones(&mut game);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(game.object(source).unwrap().loyalty(), Some(5));
        let victim = game.create_object_from_definition(
            &vanilla("Ral target", "{2}", "Human", 1, 30),
            B,
            Zone::Battlefield,
        );
        let mut dm = Choices {
            targets: vec![Target::Object(victim)],
            ..Default::default()
        };
        let minus_three = definition
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, a)| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .nth(1)
            .unwrap()
            .0;
        activate(&mut game, source, minus_three, &mut dm);
        assert_eq!(game.object(source).unwrap().loyalty(), Some(2));
        let library = *game.player(A).unwrap().library.iter().next().unwrap();
        game.move_object_by_game_rule(library, Zone::Exile).unwrap();
        game.set_current_controller(source, B).unwrap();
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.damage_on(victim), 4);
    }
}

#[test]
fn runebound_wolf_counts_either_subtype_once_using_live_control_scope() {
    for definition in definitions("Runebound Wolf") {
        for depart in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let other = game.create_object_from_definition(
                &vanilla("Werewolf", "{1}", "Werewolf", 1, 1),
                A,
                Zone::Battlefield,
            );
            game.create_object_from_definition(
                &vanilla("Both subtypes", "{1}", "Wolf Werewolf", 1, 1),
                A,
                Zone::Battlefield,
            );
            game.create_object_from_definition(
                &vanilla("Opponent Wolf", "{1}", "Wolf", 1, 1),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Player(B)],
                ..Default::default()
            };
            activate(&mut game, source, activated(&definition), &mut dm);
            game.set_current_controller(other, C).unwrap();
            if depart {
                game.move_object_by_game_rule(source, Zone::Graveyard)
                    .unwrap();
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.player(B).unwrap().life, if depart { 19 } else { 18 });
        }
    }
}

#[test]
fn barret_counts_equipped_creatures_not_equipment_and_keeps_the_attack_recipient() {
    for definition in definitions("Barret Wallace") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let buddy = game.create_object_from_definition(
            &vanilla("Equipped ally", "{1}", "Human", 1, 1),
            A,
            Zone::Battlefield,
        );
        let opponent = game.create_object_from_definition(
            &vanilla("Equipped opponent", "{1}", "Human", 1, 1),
            B,
            Zone::Battlefield,
        );
        for (owner, host) in [(A, source), (A, source), (B, buddy), (A, opponent)] {
            let equipment = game.create_object_from_definition(
                &resource("Equipment", "Artifact — Equipment"),
                owner,
                Zone::Battlefield,
            );
            apply(
                &mut game,
                source,
                Effect::attach_objects(
                    ChooseSpec::SpecificObject(equipment),
                    ChooseSpec::SpecificObject(host),
                ),
            );
        }
        let mut dm = Choices::default();
        game.remove_summoning_sickness(source);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let mut combat = ironsmith::combat_state::CombatState::default();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[ironsmith::decision::AttackerDeclaration {
                creature: source,
                target: ironsmith::combat_state::AttackTarget::Player(B),
            }],
        )
        .unwrap();
        game.combat = Some(combat);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        game.set_current_controller(source, C).unwrap();
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 19);
        assert_eq!(game.player(C).unwrap().life, 20);
    }
}

#[test]
fn ramuh_preserves_comma_joined_exclusions_and_later_wizard_chapters() {
    for definition in definitions("Summon: Esper Ramuh") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let wizard = game.create_object_from_definition(
            &vanilla("Wizard ally", "{1}", "Wizard", 1, 10),
            A,
            Zone::Battlefield,
        );
        let victim = game.create_object_from_definition(
            &vanilla("Judgment target", "{1}", "Human", 1, 30),
            B,
            Zone::Battlefield,
        );
        for (player, zone, types) in [
            (A, Zone::Graveyard, "Instant"),
            (A, Zone::Graveyard, "Artifact"),
            (A, Zone::Graveyard, "Artifact Creature — Golem"),
            (A, Zone::Graveyard, "Land"),
            (B, Zone::Graveyard, "Instant"),
            (A, Zone::Exile, "Enchantment"),
        ] {
            game.create_object_from_definition(&resource("Counted candidate", types), player, zone);
        }
        let response = game.create_object_from_definition(
            &resource("Response card", "Sorcery"),
            A,
            Zone::Hand,
        );
        let mut dm = Choices {
            targets: vec![Target::Object(victim)],
            ..Default::default()
        };
        for chapter in 1..=3 {
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::add_lore_counter_and_check_chapters(
                &mut game, source, &mut queue,
            )
            .unwrap();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            if chapter == 1 {
                game.move_object_by_game_rule(response, Zone::Graveyard)
                    .unwrap();
            }
            resolve_all(&mut game, &mut dm);
            if chapter == 1 {
                assert_eq!(game.damage_on(victim), 3);
            }
        }
        assert_eq!(game.current_power(wizard), Some(3));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.current_power(wizard), Some(1));
    }
}

#[test]
fn triumphant_chomp_uses_the_maximum_of_two_and_current_matching_power() {
    for definition in definitions("Triumphant Chomp") {
        for populated in [0, 1, 2] {
            let mut game = game();
            let victim = game.create_object_from_definition(
                &vanilla("Chomp target", "{3}", "Human", 1, 30),
                B,
                Zone::Battlefield,
            );
            game.create_object_from_definition(
                &vanilla("Opposing Dinosaur", "{3}", "Dinosaur", 20, 30),
                B,
                Zone::Battlefield,
            );
            if populated > 0 {
                game.create_object_from_definition(
                    &vanilla("Negative Dinosaur", "{1}", "Dinosaur", -2, 10),
                    A,
                    Zone::Battlefield,
                );
            }
            let grown = (populated == 2).then(|| {
                game.create_object_from_definition(
                    &vanilla("Growing Dinosaur", "{3}", "Dinosaur", 3, 10),
                    A,
                    Zone::Battlefield,
                )
            });
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            };
            let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            if let Some(grown) = grown {
                apply(
                    &mut game,
                    spell,
                    Effect::pump(5, 0, ChooseSpec::SpecificObject(grown), Until::EndOfTurn),
                );
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.damage_on(victim), if populated == 2 { 8 } else { 2 });
        }
    }
}

#[test]
fn burn_at_the_stake_scales_the_paid_tap_group_even_after_untap_control_and_zone_changes() {
    for definition in definitions("Burn at the Stake") {
        for paid in [0, 2] {
            let mut game = game();
            let mut objects = Vec::new();
            for _ in 0..4 {
                objects.push(game.create_object_from_definition(
                    &vanilla("Tap payment", "{1}", "Human", 1, 1),
                    A,
                    Zone::Battlefield,
                ));
            }
            let mut dm = Choices {
                targets: vec![Target::Player(B)],
                objects: objects[..paid].to_vec(),
                objects_explicit: true,
                ..Default::default()
            };
            let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            if paid == 2 {
                assert!(game.is_tapped(objects[0]) && game.is_tapped(objects[1]));
                apply(
                    &mut game,
                    spell,
                    Effect::untap(ChooseSpec::SpecificObject(objects[0])),
                );
                game.set_current_controller(objects[0], C).unwrap();
                let grave = game
                    .move_object_by_game_rule(objects[1], Zone::Graveyard)
                    .unwrap();
                game.move_object_by_game_rule(grave, Zone::Battlefield)
                    .unwrap();
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.player(B).unwrap().life, 20 - 3 * paid as i32);
        }
    }
}
