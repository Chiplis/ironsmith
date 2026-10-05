use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{
    AttackTarget, CombatError, CombatState, declare_attackers, declare_blockers,
};
use ironsmith::decision::{DecisionMaker, LegalAction};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::AttachmentTarget;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use serde_json::Value;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/multiblock_permissions.json.fixture"
    ))
    .unwrap()
}
fn fixture(name: &str) -> Value {
    fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap()
}
fn source(row: &Value) -> String {
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().to_owned());
    lines.join("\n")
}
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    [direct, restored]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.active_player = PlayerId::from_index(0);
    game.turn.priority_player = Some(PlayerId::from_index(0));
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn creature(game: &mut GameState, name: &str, controller: PlayerId, rules: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(
        name,
        format!("Type: Creature — Human\nPower/Toughness: 1/5\n{rules}"),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}
fn attack_setup(
    game: &mut GameState,
    defender: PlayerId,
    count: usize,
    rules: &str,
) -> (CombatState, Vec<ObjectId>) {
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    let definition = compile_to_runtime_definition(
        "Attacker",
        format!("Type: Creature — Human\nPower/Toughness: 1/5\n{rules}"),
        false,
    )
    .unwrap();
    let attackers = (0..count)
        .map(|_| {
            let attacker =
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.remove_summoning_sickness(attacker);
            attacker
        })
        .collect::<Vec<_>>();
    let mut combat = CombatState::default();
    declare_attackers(
        game,
        &mut combat,
        attackers
            .iter()
            .map(|id| (*id, AttackTarget::Player(defender)))
            .collect(),
    )
    .unwrap();
    (combat, attackers)
}
fn blocks(
    game: &GameState,
    combat: &CombatState,
    blocker: ObjectId,
    attackers: &[ObjectId],
) -> Result<CombatState, CombatError> {
    let mut result = combat.clone();
    declare_blockers(
        game,
        &mut result,
        attackers
            .iter()
            .map(|attacker| (blocker, *attacker))
            .collect(),
    )?;
    Ok(result)
}

#[test]
fn exact_unlimited_capacity_subset_keeps_metadata_artifacts_and_rendered_meaning() {
    let rows = fixtures();
    assert_eq!(rows.len(), 18, "retain the entire frozen candidate family");
    let selected = rows
        .iter()
        .filter(|row| row["repair_group"] == "unlimited_capacity")
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 7);
    for row in selected {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, &source(row)) {
            assert_eq!(
                definition.card.mana_cost.as_ref().unwrap().to_oracle(),
                row["mana_cost"].as_str().unwrap()
            );
            if let Some(power) = row["power"].as_str() {
                let pt = definition.card.power_toughness.unwrap();
                assert_eq!(pt.power.to_string(), power);
                assert_eq!(pt.toughness.to_string(), row["toughness"].as_str().unwrap());
                assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
                    AbilityKind::Static(ability) if ability.id() == StaticAbilityId::CanBlockAnyNumber)), "{name}: executable source permission");
            }
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert!(
                rendered
                    .to_lowercase()
                    .contains("can block any number of creatures"),
                "{name}: {rendered}"
            );
            assert!(
                !rendered.contains("has can block") && !rendered.contains("gains can block"),
                "{name}: {rendered}"
            );
            if name == "Valor Made Real" {
                assert!(rendered.contains("this turn"), "{rendered}");
                let reparsed = compile_to_runtime_definition(
                    name,
                    format!("Mana cost: {{W}}\nType: Instant\n{rendered}"),
                    false,
                )
                .unwrap();
                assert!(
                    ironsmith_text::compiled_text_lines(&reparsed)
                        .join("\n")
                        .contains("any number")
                );
            }
        }
    }
}

#[test]
fn source_permission_changes_real_blocking_capacity_without_bypassing_restrictions() {
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    for name in ["Wall of Glare", "Palace Guard"] {
        for definition in definitions(name, &source(&fixture(name))) {
            let mut game = game();
            let guard = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            let ordinary = creature(&mut game, "Ordinary defender", bob, "");
            let foreign =
                game.create_object_from_definition(&definition, charlie, Zone::Battlefield);
            let (combat, attackers) = attack_setup(&mut game, bob, 8, "");
            let declared =
                blocks(&game, &combat, guard, &attackers).expect("all eight assignments are legal");
            assert!(
                attackers
                    .iter()
                    .all(|attacker| declared.blockers.get(attacker) == Some(&vec![guard]))
            );
            assert!(blocks(&game, &combat, ordinary, &attackers[..2]).is_err());
            assert!(
                blocks(&game, &combat, foreign, &attackers[..1]).is_err(),
                "another defender's creature cannot help"
            );
            assert!(
                declare_blockers(
                    &game,
                    &mut combat.clone(),
                    vec![(guard, attackers[0]), (guard, attackers[0])]
                )
                .is_err(),
                "unlimited capacity cannot duplicate one pair"
            );
            game.tap(guard);
            assert!(
                blocks(&game, &combat, guard, &attackers).is_err(),
                "tapped remains illegal"
            );
            game.untap(guard);
            let (flying_combat, flyers) = attack_setup(&mut game, bob, 2, "Flying");
            assert!(
                blocks(&game, &flying_combat, guard, &flyers).is_err(),
                "capacity does not grant reach"
            );
        }
    }
}

#[test]
fn unlimited_capacity_preserves_finite_limits_requirements_and_global_caps() {
    let bob = PlayerId::from_index(1);
    for definition in definitions(
        "Required defender",
        "Type: Creature — Human\nPower/Toughness: 1/5\nThis creature can block any number of creatures.",
    ) {
        let mut game = game();
        let guard = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
        for attacker in &attackers {
            game.effect_store
                .cant_effects
                .must_be_blocked
                .insert(*attacker);
        }
        assert!(
            blocks(&game, &combat, guard, &attackers[..1]).is_err(),
            "solver knows all three requirements can be met"
        );
        blocks(&game, &combat, guard, &attackers).unwrap();
        let mut cap =
            compile_to_runtime_definition("Global blocker cap", "Type: Enchantment", false)
                .unwrap();
        cap.abilities.push(Ability::static_ability(
            StaticAbility::max_blockers_each_combat(1),
        ));
        game.create_object_from_definition(&cap, bob, Zone::Battlefield);
        blocks(&game, &combat, guard, &attackers)
            .expect("one creature still counts as one blocker");
        let other = creature(&mut game, "Extra defender", bob, "");
        assert!(
            declare_blockers(
                &game,
                &mut combat.clone(),
                vec![
                    (guard, attackers[0]),
                    (guard, attackers[1]),
                    (other, attackers[2])
                ]
            )
            .is_err(),
            "global distinct-blocker cap still applies"
        );
    }
    for definition in definitions(
        "Finite defender",
        "Type: Creature — Human\nPower/Toughness: 1/5\nThis creature can block an additional two creatures each combat.",
    ) {
        let mut game = game();
        let guard = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let (combat, attackers) = attack_setup(&mut game, bob, 4, "");
        blocks(&game, &combat, guard, &attackers[..3]).unwrap();
        assert!(
            blocks(&game, &combat, guard, &attackers).is_err(),
            "existing finite capacity remains exact"
        );
    }
}

#[test]
fn entangler_follows_attachment_instead_of_aura_controller_and_ends_on_departure() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Entangler", &source(&fixture("Entangler"))) {
        let mut game = game();
        let host = creature(&mut game, "First host", bob, "");
        let other = creature(&mut game, "Second host", bob, "");
        let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(host)));
        let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
        blocks(&game, &combat, host, &attackers).unwrap();
        assert!(blocks(&game, &combat, other, &attackers).is_err());
        assert!(!game.current_has_static_ability_id(aura, StaticAbilityId::CanBlockAnyNumber));
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(other)));
        assert!(blocks(&game, &combat, host, &attackers).is_err());
        blocks(&game, &combat, other, &attackers).unwrap();
        game.move_object_by_game_rule(aura, Zone::Graveyard)
            .unwrap();
        assert!(blocks(&game, &combat, other, &attackers).is_err());
    }
}

struct TargetDecision(ObjectId);
impl DecisionMaker for TargetDecision {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        let target = Target::Object(self.0);
        assert!(
            context
                .requirements
                .iter()
                .any(|requirement| requirement.legal_targets.contains(&target))
        );
        assert!(
            ironsmith::targeting::validate_flat_target_assignment(&context.requirements, &[target]),
            "{}: {context:?}",
            game.object(context.source)
                .map(|object| object.name.as_str())
                .unwrap_or("unknown source")
        );
        vec![target]
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, caster: PlayerId, target: ObjectId) {
    game.turn.priority_player = Some(caster);
    game.player_mut(caster)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::White, 1);
    game.player_mut(caster).unwrap().mana_pool.add(
        ManaSymbol::Colorless,
        definition.card.mana_cost.as_ref().unwrap().mana_value() - 1,
    );
    let spell = game.create_object_from_definition(definition, caster, Zone::Hand);
    let mut state = PriorityLoopState::new(3);
    let mut queue = TriggerQueue::new();
    let mut dm = TargetDecision(target);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: spell,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        &mut dm,
    )
    .unwrap();
    for _ in 0..32 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            break;
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    assert!(!game.stack_is_empty());
    assert_eq!(
        game.player(caster).unwrap().mana_pool.total(),
        0,
        "printed cost was paid"
    );
    while !game.stack_is_empty() {
        resolve_stack_entry_with(game, &mut dm).unwrap();
        put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    }
}

#[test]
fn valor_real_cast_targets_only_selected_creature_and_expires_at_cleanup() {
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    for definition in definitions("Valor Made Real", &source(&fixture("Valor Made Real"))) {
        let mut game = game();
        let guard = creature(&mut game, "Temporary guard", bob, "");
        let other = creature(&mut game, "Other guard", bob, "");
        cast(&mut game, &definition, charlie, guard);
        let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
        blocks(&game, &combat, guard, &attackers).unwrap();
        assert!(blocks(&game, &combat, other, &attackers).is_err());
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(
            blocks(&game, &combat, guard, &attackers).is_err(),
            "temporary capacity expires"
        );
    }
}

#[test]
fn filtered_permission_rechecks_controller_and_source_presence() {
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    for definition in definitions(
        "Shared defense",
        "Type: Enchantment\nCreatures you control can block any number of creatures.",
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let guard = creature(&mut game, "Controlled defender", bob, "");
        let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
        blocks(&game, &combat, guard, &attackers).unwrap();
        game.set_current_controller(source, charlie).unwrap();
        assert!(blocks(&game, &combat, guard, &attackers).is_err());
        game.set_current_controller(source, bob).unwrap();
        blocks(&game, &combat, guard, &attackers).unwrap();
        game.move_object_by_game_rule(source, Zone::Graveyard)
            .unwrap();
        assert!(blocks(&game, &combat, guard, &attackers).is_err());
    }
}

#[test]
fn a_multiblocking_wall_receives_damage_from_each_attacker() {
    let bob = PlayerId::from_index(1);
    for definition in definitions("Wall of Glare", &source(&fixture("Wall of Glare"))) {
        let mut game = game();
        let wall = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
        let declared = blocks(&game, &combat, wall, &attackers).unwrap();
        ironsmith::game_loop::try_execute_combat_damage_step(&mut game, &declared, false).unwrap();
        assert_eq!(game.damage_on(wall), 3);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert!(
            attackers
                .iter()
                .all(|attacker| game.damage_on(*attacker) == 0)
        );
    }
}

#[test]
fn exact_finite_conditional_and_compound_subset_preserves_full_card_inputs() {
    let selected = fixtures()
        .into_iter()
        .filter(|row| row["repair_group"] == "finite_conditional_or_compound")
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 8);
    for row in selected {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, &source(&row)) {
            assert_eq!(
                definition.card.mana_cost.as_ref().unwrap().to_oracle(),
                row["mana_cost"].as_str().unwrap()
            );
            assert!(!definition.card.card_types.is_empty());
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert!(
                rendered.to_ascii_lowercase().contains("can block"),
                "{name}: {rendered}"
            );
            assert!(
                !rendered.contains("has can block") && !rendered.contains("gains can block"),
                "{name}: {rendered}"
            );
        }
    }
}

#[test]
fn attached_finite_capacity_keeps_pumps_vigilance_and_additive_counts() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (name, power, toughness, vigilance) in [
        ("Echo Circlet", 1, 5, false),
        ("Vanguard's Shield", 1, 8, false),
        ("Iona's Blessing", 3, 7, true),
    ] {
        for definition in definitions(name, &source(&fixture(name))) {
            let mut game = game();
            let host = creature(&mut game, "Finite host", bob, "");
            let other = creature(&mut game, "Other finite host", bob, "");
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            assert!(game.attach_object_to_target(source, AttachmentTarget::Object(host)));
            assert_eq!(game.current_power(host), Some(power), "{name}");
            assert_eq!(game.current_toughness(host), Some(toughness), "{name}");
            assert_eq!(
                game.current_has_static_ability_id(host, StaticAbilityId::Vigilance),
                vigilance
            );
            let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
            blocks(&game, &combat, host, &attackers[..2]).unwrap();
            assert!(blocks(&game, &combat, host, &attackers).is_err());
            assert!(blocks(&game, &combat, other, &attackers[..2]).is_err());
            let second = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            assert!(game.attach_object_to_target(second, AttachmentTarget::Object(host)));
            blocks(&game, &combat, host, &attackers)
                .expect("two independent additional allowances add");
            assert!(game.attach_object_to_target(source, AttachmentTarget::Object(other)));
            assert!(blocks(&game, &combat, host, &attackers).is_err());
            blocks(&game, &combat, other, &attackers[..2]).unwrap();
            game.move_object_by_game_rule(second, Zone::Graveyard)
                .unwrap();
            assert!(blocks(&game, &combat, host, &attackers[..2]).is_err());
        }
    }
}

#[test]
fn monarch_condition_rechecks_live_designation() {
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    for definition in definitions(
        "Entourage of Trest",
        &source(&fixture("Entourage of Trest")),
    ) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
        assert!(blocks(&game, &combat, host, &attackers[..2]).is_err());
        game.set_monarch(Some(bob))
            .expect("checked designation/departure fixture");
        blocks(&game, &combat, host, &attackers[..2]).unwrap();
        assert!(blocks(&game, &combat, host, &attackers).is_err());
        game.set_monarch(Some(charlie))
            .expect("checked designation/departure fixture");
        assert!(blocks(&game, &combat, host, &attackers[..2]).is_err());
    }
}

fn activate_first_ability(game: &mut GameState, source: ObjectId, caster: PlayerId) {
    game.turn.active_player = caster;
    game.turn.priority_player = Some(caster);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let action = ironsmith::compute_legal_actions(game, caster).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).expect("printed activation is legally available");
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..32 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            break;
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert!(!game.stack_is_empty());
    while !game.stack_is_empty() {
        resolve_stack_entry_with(game, &mut dm).unwrap();
        put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    }
}

#[test]
fn monstrous_condition_grants_reach_and_exactly_ninety_nine_additional_blocks() {
    let bob = PlayerId::from_index(1);
    for definition in definitions(
        "Hundred-Handed One",
        &source(&fixture("Hundred-Handed One")),
    ) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        assert!(!game.current_has_static_ability_id(host, StaticAbilityId::Reach));
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 3);
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        activate_first_ability(&mut game, host, bob);
        assert!(game.is_monstrous(host));
        assert_eq!(game.current_power(host), Some(6));
        assert_eq!(game.current_toughness(host), Some(8));
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Reach));
        let (combat, attackers) = attack_setup(&mut game, bob, 101, "Flying");
        blocks(&game, &combat, host, &attackers[..100]).unwrap();
        assert!(
            blocks(&game, &combat, host, &attackers).is_err(),
            "100 is finite capacity, not unlimited"
        );
    }
}

#[test]
fn temporary_compound_grants_keep_untap_pump_capacity_and_cleanup() {
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    for (name, power, toughness, unlimited) in [
        ("Act of Heroism", 3, 7, false),
        ("Give No Ground", 3, 11, true),
    ] {
        for definition in definitions(name, &source(&fixture(name))) {
            let mut game = game();
            let host = creature(&mut game, "Spell recipient", bob, "");
            let other = creature(&mut game, "Other spell recipient", bob, "");
            if name == "Act of Heroism" {
                game.tap(host);
            }
            cast(&mut game, &definition, charlie, host);
            assert!(!game.is_tapped(host));
            assert_eq!(game.current_power(host), Some(power), "{name}");
            assert_eq!(game.current_toughness(host), Some(toughness), "{name}");
            let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
            blocks(&game, &combat, host, &attackers[..2]).unwrap();
            assert_eq!(blocks(&game, &combat, host, &attackers).is_ok(), unlimited);
            assert!(blocks(&game, &combat, other, &attackers[..2]).is_err());
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(game.current_power(host), Some(1));
            assert_eq!(game.current_toughness(host), Some(5));
            assert!(blocks(&game, &combat, host, &attackers[..2]).is_err());
        }
    }
}

#[test]
fn totem_capacity_exists_only_while_its_actual_animation_is_active() {
    let bob = PlayerId::from_index(1);
    for definition in definitions("Foriysian Totem", &source(&fixture("Foriysian Totem"))) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let (combat, attackers) = attack_setup(&mut game, bob, 3, "");
        assert!(
            blocks(&game, &combat, host, &attackers[..1]).is_err(),
            "an unanimated artifact cannot block"
        );
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 4);
        activate_first_ability(&mut game, host, bob);
        assert_eq!(game.current_power(host), Some(4));
        assert_eq!(game.current_toughness(host), Some(4));
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Trample));
        blocks(&game, &combat, host, &attackers[..2]).unwrap();
        assert!(blocks(&game, &combat, host, &attackers).is_err());
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(blocks(&game, &combat, host, &attackers[..1]).is_err());
    }
}

#[test]
fn guardian_triggers_once_and_counts_only_its_current_blocked_attackers_at_resolution() {
    use ironsmith::game_state::Step;
    let bob = PlayerId::from_index(1);
    let row = fixture("Guardian of the Gateless");
    for name in ["Guardian of the Gateless", "Renamed Guardian"] {
        for definition in definitions(name, &source(&row)) {
            let mut game = game();
            let guardian = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            let other = creature(&mut game, "Other blocker", bob, "");
            let (mut combat, attackers) = attack_setup(&mut game, bob, 4, "");
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareBlockers);
            game.combat = Some(combat.clone());
            let mut queue = TriggerQueue::new();
            let declarations = attackers[..3]
                .iter()
                .map(|attacker| ironsmith::BlockerDeclaration {
                    blocker: guardian,
                    blocking: *attacker,
                })
                .chain(std::iter::once(ironsmith::BlockerDeclaration {
                    blocker: other,
                    blocking: attackers[3],
                }))
                .collect::<Vec<_>>();
            ironsmith::game_loop::apply_blocker_declarations(
                &mut game,
                &mut combat,
                &mut queue,
                &declarations,
                bob,
            )
            .unwrap();
            assert_eq!(
                queue.entries.len(),
                1,
                "blocks triggers once, not once per blocked creature"
            );
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.len(), 1);
            game.move_object_by_game_rule(attackers[0], Zone::Graveyard)
                .unwrap();
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(
                game.current_power(guardian),
                Some(5),
                "two of its blocked attackers remain; the other block does not count"
            );
            assert_eq!(game.current_toughness(guardian), Some(5));
            assert_eq!(game.current_power(other), Some(1));
            game.move_object_by_game_rule(attackers[1], Zone::Graveyard)
                .unwrap();
            assert_eq!(
                game.current_power(guardian),
                Some(5),
                "the resolved pump is a snapshot, not a continuously changing count"
            );
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(game.current_power(guardian), Some(3));
            assert_eq!(game.current_toughness(guardian), Some(3));
        }
    }
}

#[test]
fn kemba_counts_only_equipment_currently_attached_to_the_actual_blocker() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let row = fixture("Kemba's Legion");
    for definition in definitions("Kemba's Legion", &source(&row)) {
        let mut game = game();
        let kemba = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.id() == StaticAbilityId::CanBlockAdditionalForEach)));
        let other = creature(&mut game, "Other equipped creature", bob, "");
        let equipment =
            compile_to_runtime_definition("Plain Equipment", "Type: Artifact — Equipment", false)
                .unwrap();
        let aura = compile_to_runtime_definition(
            "Plain Aura",
            "Type: Enchantment — Aura\nEnchant creature",
            false,
        )
        .unwrap();
        let first = game.create_object_from_definition(&equipment, alice, Zone::Battlefield);
        let second = game.create_object_from_definition(&equipment, bob, Zone::Battlefield);
        let unrelated = game.create_object_from_definition(&equipment, bob, Zone::Battlefield);
        let aura_id = game.create_object_from_definition(&aura, bob, Zone::Battlefield);
        game.attach_object_to_target(unrelated, AttachmentTarget::Object(other));
        game.attach_object_to_target(aura_id, AttachmentTarget::Object(kemba));
        let (combat, attackers) = attack_setup(&mut game, bob, 4, "");
        blocks(&game, &combat, kemba, &attackers[..1]).unwrap();
        assert!(
            blocks(&game, &combat, kemba, &attackers[..2]).is_err(),
            "Aura and someone else's Equipment do not count"
        );
        game.attach_object_to_target(first, AttachmentTarget::Object(kemba));
        blocks(&game, &combat, kemba, &attackers[..2]).unwrap();
        assert!(blocks(&game, &combat, kemba, &attackers[..3]).is_err());
        game.attach_object_to_target(second, AttachmentTarget::Object(kemba));
        blocks(&game, &combat, kemba, &attackers[..3]).unwrap();
        assert!(blocks(&game, &combat, kemba, &attackers).is_err());
        let strip = compile_to_runtime_definition(
            "Remove blocking ability",
            "Mana cost: {W}\nType: Instant\nTarget creature loses all abilities until end of turn.",
            false,
        )
        .unwrap();
        cast(&mut game, &strip, alice, kemba);
        assert!(
            blocks(&game, &combat, kemba, &attackers[..2]).is_err(),
            "ability loss removes the dynamic allowance"
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        blocks(&game, &combat, kemba, &attackers[..3]).unwrap();
        game.attach_object_to_target(first, AttachmentTarget::Object(other));
        assert!(blocks(&game, &combat, kemba, &attackers[..3]).is_err());
        blocks(&game, &combat, kemba, &attackers[..2]).unwrap();
        game.move_object_by_game_rule(second, Zone::Graveyard)
            .unwrap();
        assert!(blocks(&game, &combat, kemba, &attackers[..2]).is_err());
    }
}

#[test]
fn blaze_of_glory_requires_each_legal_attacker_without_overriding_evasion_or_timing() {
    use ironsmith::game_state::Step;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let row = fixture("Blaze of Glory");
    for definition in definitions("Blaze of Glory", &source(&row)) {
        let mut game = game();
        let guard = creature(&mut game, "Forced defender", bob, "");
        game.mark_combat_phase_started();
        let (mut combat, attackers) = attack_setup(&mut game, bob, 3, "");
        let flyer = creature(&mut game, "Unblockable-by-guard flyer", alice, "Flying");
        game.remove_summoning_sickness(flyer);
        // All attackers belong to the same declaration, including the flyer.
        for attacker in &attackers {
            game.untap(*attacker);
        }
        combat = CombatState::default();
        declare_attackers(
            &mut game,
            &mut combat,
            attackers
                .iter()
                .copied()
                .chain(std::iter::once(flyer))
                .map(|id| (id, AttackTarget::Player(bob)))
                .collect(),
        )
        .unwrap();
        game.combat = Some(combat.clone());
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        cast(&mut game, &definition, alice, guard);
        let mut too_late = game.clone();
        too_late.turn.step = Some(Step::DeclareBlockers);
        too_late.turn.priority_player = Some(alice);
        too_late
            .player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 1);
        let late_spell = too_late.create_object_from_definition(&definition, alice, Zone::Hand);
        assert!(!ironsmith::compute_legal_actions(&too_late, alice).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == late_spell)), "the printed before-blockers restriction still applies");
        assert!(declare_blockers(&game, &mut combat.clone(), Vec::new()).is_err());
        assert!(
            blocks(&game, &combat, guard, &attackers[..2]).is_err(),
            "all three legally blockable attackers are required"
        );
        blocks(&game, &combat, guard, &attackers).unwrap();
        assert!(
            blocks(&game, &combat, guard, &[flyer]).is_err(),
            "cannot block the flyer merely because of the requirement"
        );
        game.tap(guard);
        declare_blockers(&game, &mut combat.clone(), Vec::new())
            .expect("if able never forces an illegal tapped block");
        game.untap(guard);
        ironsmith::turn::execute_cleanup_step(&mut game);
        declare_blockers(&game, &mut combat.clone(), Vec::new()).expect("the requirement expires");
        assert!(
            blocks(&game, &combat, guard, &attackers).is_err(),
            "the extra capacity also expires"
        );
    }
}
