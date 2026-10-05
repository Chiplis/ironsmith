//! Source-authored only. No compilation or execution was run for this batch.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState, declare_attackers, declare_blockers};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, resolve_stack_entry,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::object::AttachmentTarget;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
fn a() -> PlayerId {
    PlayerId::from_index(0)
}
fn b() -> PlayerId {
    PlayerId::from_index(1)
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn fixture(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/scoped_damage_redirection.json.fixture"
    ))
    .unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        card["mana_cost"].as_str().unwrap_or(""),
        card["type_line"].as_str().unwrap()
    );
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(card["oracle_text"].as_str().unwrap());
    definitions(name, &text)
}
fn creature(game: &mut GameState, player: PlayerId) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Redirection recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 10))
            .build(),
        player,
        Zone::Battlefield,
    )
}
fn damage(
    game: &mut GameState,
    source: ObjectId,
    target: DamageTarget,
    combat: bool,
    unpreventable: bool,
    lki: Option<&ObjectSnapshot>,
) -> Vec<(DamageTarget, u32)> {
    game.refresh_continuous_state().unwrap();
    game.take_pending_trigger_events();
    let out = process_damage_assignments_with_event_with_source_snapshot_opts(
        game,
        source,
        target,
        3,
        combat,
        unpreventable,
        EventCause::effect(),
        lki,
    )
    .unwrap();
    assert!(
        !game
            .take_pending_trigger_events()
            .iter()
            .any(|event| event.downcast::<DamagePreventedEvent>().is_some()),
        "redirection is not prevention"
    );
    out.assignments
        .into_iter()
        .map(|assignment| (assignment.target, assignment.amount))
        .collect()
}
fn resolve(game: &mut GameState, definition: &CardDefinition, targets: Vec<Target>) {
    let source = game.create_object_from_definition(definition, a(), Zone::Stack);
    let requirements = extract_target_requirements_from_program_with_modes(
        game,
        definition.spell_effect.as_ref().unwrap(),
        a(),
        Some(source),
        None,
    );
    let assignments = if targets.is_empty() {
        assert!(requirements.is_empty());
        vec![]
    } else {
        assert_eq!(requirements.len(), 1);
        let requirement = &requirements[0];
        for target in &targets {
            assert!(requirement.legal_targets.contains(target));
        }
        vec![TargetAssignment {
            spec: requirement.spec.clone(),
            range: 0..targets.len(),
        }]
    };
    game.push_to_stack(
        StackEntry::new(source, a())
            .with_targets(targets)
            .with_target_assignments(assignments),
    );
    resolve_stack_entry(game).unwrap();
}
fn attackers(game: &mut GameState, source: ObjectId) -> CombatState {
    game.turn.active_player = b();
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.remove_summoning_sickness(source);
    let mut combat = CombatState::default();
    declare_attackers(game, &mut combat, vec![(source, AttackTarget::Player(a()))]).unwrap();
    game.combat = Some(combat.clone());
    combat
}
fn no_blocks(game: &mut GameState, combat: &mut CombatState) {
    game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    declare_blockers(game, combat, vec![]).unwrap();
    game.combat = Some(combat.clone());
}
#[test]
fn all_16_frozen_bodies_round_trip_strictly_without_reduced_oracle_examples() {
    for name in [
        "Empyrial Archangel",
        "Harsh Judgment",
        "Martyrs of Korlis",
        "Pariah",
        "Pariah's Shield",
        "Protector of the Crown",
        "Treacherous Link",
        "Veteran Bodyguard",
        "Weathered Bodyguards",
        "Ascent of the Worthy",
        "Karona's Zealot",
        "Kjeldoran Royal Guard",
        "Mirror Strike",
        "Shimian Night Stalker",
        "Sivvi's Valor",
        "Turn the Tables",
    ] {
        for definition in fixture(name) {
            assert_eq!(definition.card.name, name);
        }
    }
}
#[test]
fn self_redirects_keep_the_original_damage_and_live_host_controller() {
    for name in ["Empyrial Archangel", "Protector of the Crown"] {
        for definition in fixture(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let source = creature(&mut game, b());
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    true,
                    None
                ),
                vec![(DamageTarget::Object(host), 3)]
            );
            game.set_current_controller(host, b()).unwrap();
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(a()), 3)]
            );
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(b()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Object(host), 3)]
            );
            game.phase_out(host);
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(b()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(b()), 3)]
            );
            game.phase_in(host);
            game.move_object_by_effect(host, Zone::Graveyard).unwrap();
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(b()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(b()), 3)]
            );
        }
    }
}
#[test]
fn attachment_destination_rechecks_attachment_type_and_current_incarnation() {
    for name in ["Pariah", "Pariah's Shield"] {
        for definition in fixture(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let recipient = creature(&mut game, a());
            let next = creature(&mut game, a());
            let source = creature(&mut game, b());
            assert!(game.attach_object_to_target(host, AttachmentTarget::Object(recipient)));
            game.set_current_controller(recipient, b()).unwrap();
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    true,
                    None
                ),
                vec![(DamageTarget::Object(recipient), 3)]
            );
            assert!(game.attach_object_to_target(host, AttachmentTarget::Object(next)));
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Object(next), 3)]
            );
            game.object_mut(next).unwrap().card_types = vec![CardType::Artifact].into();
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(a()), 3)]
            );
            game.detach_object_from_current_target(host);
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(a()), 3)]
            );
        }
    }
}
#[test]
fn treacherous_link_uses_the_damaged_creatures_controller_not_the_aura_controller() {
    for definition in fixture("Treacherous Link") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let recipient = creature(&mut game, b());
        let source = creature(&mut game, a());
        game.attach_object_to_target(host, AttachmentTarget::Object(recipient));
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(recipient),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Player(b()), 3)]
        );
        game.set_current_controller(recipient, a()).unwrap();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(recipient),
                false,
                true,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        game.object_mut(recipient).unwrap().card_types = vec![CardType::Enchantment].into();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(recipient),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Object(recipient), 3)],
            "invalid original recipient cannot be redirected elsewhere"
        );
    }
}
#[test]
fn martyrs_checks_untapped_host_and_artifact_source_characteristics() {
    for definition in fixture("Martyrs of Korlis") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let source = creature(&mut game, b());
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        game.object_mut(source).unwrap().card_types =
            vec![CardType::Artifact, CardType::Creature].into();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                true,
                None
            ),
            vec![(DamageTarget::Object(host), 3)]
        );
        game.tap(host);
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        game.untap(host);
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                false,
                Some(&snapshot)
            ),
            vec![(DamageTarget::Object(host), 3)]
        );
    }
}
#[test]
fn harsh_judgment_uses_live_damage_controller_then_matching_lki() {
    for definition in fixture("Harsh Judgment") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        game.set_chosen_color(host, ironsmith::color::Color::Red);
        let spell = CardBuilder::new(CardId::new(), "Red source")
            .card_types(vec![CardType::Instant])
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
                ironsmith::mana::ManaSymbol::Red,
            ]))
            .build();
        let source = game.create_object_from_card(&spell, b(), Zone::Stack);
        let old = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        game.set_current_controller(source, a()).unwrap();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                false,
                Some(&old)
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        game.set_current_controller(source, b()).unwrap();
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                true,
                Some(&snapshot)
            ),
            vec![(DamageTarget::Player(b()), 3)]
        );
    }
}
#[test]
fn unblocked_redirects_begin_only_after_declaration_and_end_with_each_combat() {
    for name in ["Veteran Bodyguard", "Weathered Bodyguards"] {
        for definition in fixture(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let source = creature(&mut game, b());
            let mut combat = attackers(&mut game, source);
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(a()), 3)]
            );
            no_blocks(&mut game, &mut combat);
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    true,
                    true,
                    None
                ),
                vec![(DamageTarget::Object(host), 3)]
            );
            let noncombat = if name == "Veteran Bodyguard" {
                DamageTarget::Object(host)
            } else {
                DamageTarget::Player(a())
            };
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    false,
                    None
                ),
                vec![(noncombat, 3)]
            );
            ironsmith::combat_state::end_combat(&mut combat);
            game.combat = Some(combat);
            game.untap(source);
            let _fresh = attackers(&mut game, source);
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(a()), 3)]
            );
        }
    }
}
#[test]
fn mirror_strike_locks_chosen_source_and_only_redirects_its_combat_damage_to_you() {
    for definition in fixture("Mirror Strike") {
        let mut game = game();
        let source = creature(&mut game, b());
        let other = creature(&mut game, b());
        let mut combat = attackers(&mut game, source);
        no_blocks(&mut game, &mut combat);
        resolve(&mut game, &definition, vec![Target::Object(source)]);
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                true,
                false,
                None
            ),
            vec![(DamageTarget::Player(b()), 3)]
        );
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        assert_eq!(
            damage(
                &mut game,
                other,
                DamageTarget::Player(a()),
                true,
                false,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        game.effect_store
            .replacement_effects
            .clear_until_end_of_turn_effects();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                true,
                false,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
    }
}
#[test]
fn target_redirect_spells_preserve_player_vs_object_roles_and_original_recipient_legality() {
    for definition in fixture("Sivvi's Valor") {
        let mut game = game();
        let protected = creature(&mut game, a());
        let source = creature(&mut game, b());
        resolve(&mut game, &definition, vec![Target::Object(protected)]);
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(protected),
                false,
                true,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        game.object_mut(protected).unwrap().card_types = vec![CardType::Artifact].into();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(protected),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Object(protected), 3)]
        );
    }
    for definition in fixture("Turn the Tables") {
        let mut game = game();
        let source = creature(&mut game, b());
        let mut combat = attackers(&mut game, source);
        no_blocks(&mut game, &mut combat);
        resolve(&mut game, &definition, vec![Target::Object(source)]);
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                true,
                true,
                None
            ),
            vec![(DamageTarget::Object(source), 3)]
        );
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        game.phase_out(source);
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                true,
                false,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
    }
}
#[test]
fn next_turn_scope_keeps_future_creatures_and_expires_at_the_controllers_actual_next_turn() {
    for definition in definitions(
        "Unlisted next-turn redirect",
        "Type: Instant\nChoose a creature you control. Until your next turn, all damage that would be dealt to creatures you control is dealt to that creature instead.",
    ) {
        let mut game = game();
        let chosen = creature(&mut game, a());
        let source = creature(&mut game, b());
        resolve(&mut game, &definition, vec![]);
        let later = creature(&mut game, a());
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(later),
                false,
                true,
                None
            ),
            vec![(DamageTarget::Object(chosen), 3)]
        );
        game.effect_store
            .replacement_effects
            .clear_until_end_of_turn_effects();
        let turn = game.turn.turn_number;
        game.effect_store
            .replacement_effects
            .expire_at_turn_start(turn + 1, &[b()]);
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(later),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Object(chosen), 3)]
        );
        game.effect_store
            .replacement_effects
            .expire_at_turn_start(turn + 2, &[a()]);
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(later),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Object(later), 3)]
        );
    }
}

fn activate(game: &mut GameState, host: ObjectId) {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    game.turn.priority_player = Some(a());
    game.remove_summoning_sickness(host);
    game.player_mut(a())
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Black, 1);
    let action = compute_legal_actions(game, a()).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source, .. } if *source == host)).unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..16 {
        if let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress {
            progress =
                apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
                    .unwrap();
        } else {
            break;
        }
    }
    assert!(state.pending_activation.is_none());
    if game.stack_is_empty() {
        assert!(
            matches!(progress, ironsmith::GameProgress::StackResolved),
            "activation did not finish: {progress:?}"
        );
    } else {
        resolve_stack_entry(game).unwrap();
    }
}
#[test]
fn royal_guard_and_night_stalker_register_through_real_paid_activations() {
    for name in ["Kjeldoran Royal Guard", "Shimian Night Stalker"] {
        for definition in fixture(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let source = creature(&mut game, b());
            let mut combat = attackers(&mut game, source);
            no_blocks(&mut game, &mut combat);
            activate(&mut game, host);
            assert!(game.is_tapped(host));
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    true,
                    true,
                    None
                ),
                vec![(DamageTarget::Object(host), 3)]
            );
            let other = creature(&mut game, b());
            assert_eq!(
                damage(
                    &mut game,
                    other,
                    DamageTarget::Player(a()),
                    true,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(a()), 3)]
            );
            let expected = if name == "Shimian Night Stalker" {
                DamageTarget::Object(host)
            } else {
                DamageTarget::Player(a())
            };
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    false,
                    false,
                    None
                ),
                vec![(expected, 3)]
            );
            game.effect_store
                .replacement_effects
                .clear_until_end_of_turn_effects();
            assert_eq!(
                damage(
                    &mut game,
                    source,
                    DamageTarget::Player(a()),
                    true,
                    false,
                    None
                ),
                vec![(DamageTarget::Player(a()), 3)]
            );
        }
    }
}
struct PreferTarget(ObjectId);
impl ironsmith::decision::DecisionMaker for PreferTarget {
    fn decide_targets(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        assert_eq!(context.requirements.len(), 1);
        assert!(
            context.requirements[0]
                .legal_targets
                .contains(&Target::Object(self.0))
        );
        vec![Target::Object(self.0)]
    }
}
#[test]
fn karona_face_up_trigger_keeps_its_subject_distinct_from_the_chosen_destination() {
    use ironsmith::effects::{EffectContext, EffectExecutor, TurnFaceUpEffect};
    use ironsmith::game_loop::put_triggers_on_stack_with_dm;
    for definition in fixture("Karona's Zealot") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let recipient = creature(&mut game, b());
        let source = creature(&mut game, b());
        game.take_pending_trigger_events();
        assert!(game.set_face_down(host));
        TurnFaceUpEffect::new(ironsmith::target::ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new_default(host, a()))
            .unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut PreferTarget(recipient)).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(host),
                false,
                true,
                None
            ),
            vec![(DamageTarget::Object(recipient), 3)]
        );
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
        game.move_object_by_effect(recipient, Zone::Graveyard)
            .unwrap();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(host),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Object(host), 3)]
        );
    }
}
#[test]
fn ascent_real_chapters_keep_choice_scope_then_return_with_counter_and_types() {
    use ironsmith::game_loop::{
        add_saga_lore_counters, handle_saga_enters_battlefield, put_triggers_on_stack_with_dm,
    };
    for definition in fixture("Ascent of the Worthy") {
        let mut game = game();
        let chosen = creature(&mut game, a());
        let source = creature(&mut game, b());
        let grave = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Saga return fixture")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build(),
            a(),
            Zone::Graveyard,
        );
        let stable = game.object(grave).unwrap().stable_id;
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        handle_saga_enters_battlefield(&mut game, host, &mut queue, &mut dm).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let later = creature(&mut game, a());
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(later),
                false,
                true,
                None
            ),
            vec![(DamageTarget::Object(chosen), 3)]
        );
        game.turn.turn_number += 1;
        let turn = game.turn.turn_number;
        game.effect_store
            .replacement_effects
            .expire_at_turn_start(turn, &[a()]);
        add_saga_lore_counters(&mut game, &mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Object(later),
                false,
                false,
                None
            ),
            vec![(DamageTarget::Object(chosen), 3)]
        );
        game.turn.turn_number += 1;
        let turn = game.turn.turn_number;
        game.effect_store
            .replacement_effects
            .expire_at_turn_start(turn, &[a()]);
        add_saga_lore_counters(&mut game, &mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut PreferTarget(grave)).unwrap();
        resolve_stack_entry(&mut game).unwrap();
        let returned = game
            .objects_in_deterministic_order()
            .into_iter()
            .find(|object| object.stable_id == stable && object.zone == Zone::Battlefield)
            .unwrap()
            .id;
        assert!(game.current_has_subtype(returned, ironsmith::Subtype::Angel));
        assert!(game.current_has_subtype(returned, ironsmith::Subtype::Warrior));
        assert!(game.current_has_static_ability_id(
            returned,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
    }
}

#[test]
fn actual_unpreventable_damage_keeps_its_source_and_lifelink_after_redirection() {
    use ironsmith::effects::{DealDamageEffect, EffectContext, EffectExecutor};
    for definition in fixture("Empyrial Archangel") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let source = creature(&mut game, b());
        game.object_mut(source).unwrap().abilities_mut().push(
            ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::lifelink(),
            ),
        );
        DealDamageEffect::new(
            3,
            ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(a())),
        )
        .with_unpreventable(true)
        .execute(&mut game, &mut EffectContext::new_default(source, b()))
        .unwrap();
        assert_eq!(game.player(a()).unwrap().life, 20);
        assert_eq!(game.damage_on(host), 3);
        assert_eq!(
            game.player(b()).unwrap().life,
            23,
            "the original source, not the redirection host, dealt the damage"
        );
        assert!(
            !game
                .take_pending_trigger_events()
                .iter()
                .any(|event| event.downcast::<DamagePreventedEvent>().is_some())
        );
    }
}
#[test]
fn each_redirect_identity_applies_once_even_when_recipients_form_a_cycle() {
    for (pariah, link) in fixture("Pariah")
        .into_iter()
        .zip(fixture("Treacherous Link"))
    {
        let mut game = game();
        let recipient = creature(&mut game, a());
        let source = creature(&mut game, b());
        let pariah = game.create_object_from_definition(&pariah, a(), Zone::Battlefield);
        let link = game.create_object_from_definition(&link, b(), Zone::Battlefield);
        game.attach_object_to_target(pariah, AttachmentTarget::Object(recipient));
        game.attach_object_to_target(link, AttachmentTarget::Object(recipient));
        assert_eq!(
            damage(
                &mut game,
                source,
                DamageTarget::Player(a()),
                false,
                true,
                None
            ),
            vec![(DamageTarget::Player(a()), 3)]
        );
    }
}
