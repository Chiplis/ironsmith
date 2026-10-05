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
    let attackers = (0..count)
        .map(|index| {
            let attacker = creature(game, &format!("Attacker {index}"), alice, rules);
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
                rendered.contains("can block any number of creatures"),
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
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        let target = Target::Object(self.0);
        assert!(
            context
                .requirements
                .iter()
                .any(|requirement| requirement.legal_targets.contains(&target))
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
        if state.pending_cast.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            break;
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    assert!(state.pending_cast.is_none());
    assert!(!game.stack_is_empty());
    assert_eq!(
        game.player(caster).unwrap().mana_pool.total(),
        0,
        "printed W was paid"
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
