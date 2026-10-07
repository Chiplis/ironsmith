//! Frozen full-body scenarios, source-authored and UNRUN.
//! Only Nightveil Specter is proposed here. The fixture retains seven held
//! neighbors so their secondary obligations are not silently narrowed away.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
use ironsmith::decision::{AttackerDeclaration, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, ExecutionError, execute_effect};
use ironsmith::game_loop::{apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn text() -> String {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/linked_exile_static_permissions.json.fixture")).unwrap();
    assert_eq!(rows.len(), 8);
    let row = rows.iter().find(|row| row["name"] == "Nightveil Specter").unwrap();
    format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap())
}
fn definitions() -> [CardDefinition; 2] {
    let text = text();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Nightveil Specter", &text, false));
    let direct = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Nightveil Specter", &text, false));
    let (artifact, _) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn producer(definition: &CardDefinition) -> &Ability {
    definition.abilities.iter().find(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).unwrap()
}
fn reader(definition: &CardDefinition) -> &Ability {
    definition.abilities.iter().find(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.grant_spec().is_some())).unwrap()
}
fn pair(definition: &CardDefinition) -> ironsmith_core::LinkedExilePair {
    let AbilityKind::Triggered(trigger) = &producer(definition).kind else { unreachable!() };
    let pair = trigger.effects.linked_exile_pair.expect("compiler proves the producer");
    let AbilityKind::Static(reader) = &reader(definition).kind else { unreachable!() };
    assert_eq!(reader.grant_spec().unwrap().linked_exile_pair, Some(pair)); pair
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20); main(&mut game, A); game
}
fn main(game: &mut GameState, player: PlayerId) {
    game.turn.phase = Phase::FirstMain; game.turn.step = None;
    game.turn.active_player = player; game.turn.priority_player = Some(player); game.combat = None;
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, land: bool) -> ObjectId {
    let text = if land { "Type: Land" } else { "Mana cost: {2}\nType: Sorcery\nYou gain 1 life." };
    let definition = compile_to_runtime_definition("Exile member", text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn combat_trigger(game: &mut GameState, source: ObjectId) {
    game.remove_summoning_sickness(source); game.untap(source);
    game.turn.phase = Phase::Combat; game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = CombatState::default(); let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut combat, &mut queue,
        &[AttackerDeclaration { creature: source, target: AttackTarget::Player(B) }]).unwrap();
    assert!(queue.entries.is_empty()); game.combat = Some(combat.clone()); game.turn.step = Some(Step::CombatDamage);
    let events = ironsmith::game_loop::execute_combat_damage_step_with_dm(game, &combat, false, &mut SelectFirstDecisionMaker);
    ironsmith::game_loop::queue_combat_damage_triggers(game, &events, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
}
fn resolve(game: &mut GameState) { resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap(); }
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) {
    execute_effect(game, &effect, &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
}
fn may_play(game: &GameState, member: ObjectId, player: PlayerId) -> bool {
    game.effect_store.grant_registry.card_can_play_from_zone(game, member, Zone::Exile, player)
}
fn legal(game: &GameState, member: ObjectId, player: PlayerId) -> bool {
    compute_legal_actions(game, player).unwrap().iter().any(|action| match action {
        LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } => *spell_id == member,
        LegalAction::PlayLand { land_id, .. } => *land_id == member,
        _ => false,
    })
}
fn linked(game: &mut GameState, definition: &CardDefinition, land: bool) -> (ObjectId, ObjectId) {
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    let victim = card(game, B, Zone::Library, land); let stable = game.object(victim).unwrap().stable_id;
    combat_trigger(game, source); assert_eq!(game.stack.len(), 1); assert!(game.stack[0].linked_exile_owner.is_some());
    *game = game.clone(); resolve(game);
    let member = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(member).unwrap().zone, Zone::Exile); (source, member)
}

#[test]
fn frozen_complete_body_preserves_typed_pair_and_keywords_in_both_routes() {
    let definitions = definitions();
    assert_eq!(pair(&definitions[0]), pair(&definitions[1]));
    for definition in definitions {
        assert_eq!(definition.abilities.len(), 3);
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(ability) if ability.id() == ironsmith::static_abilities::StaticAbilityId::Flying)));
        let renamed = compile_to_runtime_definition("Unrelated caller label", &text(), false).unwrap();
        assert_eq!(pair(&definition), pair(&renamed), "card names/local CardIds do not choose the relationship");
    }
}

#[test]
fn actual_combat_exiles_the_damaged_players_top_card_and_keeps_normal_play_costs_and_timing() {
    for definition in definitions() { for land in [false, true] {
        let mut game = game(); let own_top = card(&mut game, A, Zone::Library, false);
        let (source, member) = linked(&mut game, &definition, land);
        assert_eq!(game.player(B).unwrap().life, 18); assert_eq!(game.object(own_top).unwrap().zone, Zone::Library);
        assert!(may_play(&game, member, A)); assert!(!may_play(&game, member, B));
        assert!(!legal(&game, member, A), "sorcery/land timing still applies during combat");
        main(&mut game, A);
        if !land { assert!(!legal(&game, member, A), "the permission supplies no free payment"); }
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        assert!(legal(&game, member, A));
        let unrelated = card(&mut game, B, Zone::Exile, land);
        game.add_exiled_with_source_link(source, unrelated);
        assert!(!may_play(&game, unrelated, A)); assert!(!legal(&game, unrelated, A));
        let mut queue = TriggerQueue::new(); let mut state = ironsmith::game_loop::PriorityLoopState::new(2);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| match action {
            LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } => *spell_id == member,
            LegalAction::PlayLand { land_id, .. } => *land_id == member, _ => false,
        }).unwrap();
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &ironsmith::game_loop::PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
        for _ in 0..32 {
            if !state.has_pending_action() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending cast has decision"); };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game, &mut queue, &mut state,
                &context, &mut SelectFirstDecisionMaker).unwrap();
        }
        assert!(!state.has_pending_action()); assert!(!game.exile.contains(&member));
        if !land { assert_eq!(game.player(A).unwrap().mana_pool.total(), 0); resolve(&mut game); }
    }}
}

#[test]
fn static_reader_tracks_current_controller_ability_and_phasing_but_not_new_incarnations() {
    for definition in definitions() { for mode in 0..5 {
        let mut game = game(); let (source, member) = linked(&mut game, &definition, false);
        match mode {
            0 => {
                let id = game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(source, A, source, B));
                game.refresh_continuous_state().unwrap();
                assert!(!may_play(&game, member, A)); assert!(may_play(&game, member, B));
                game.effect_store.continuous_effects.remove_effect(id); game.refresh_continuous_state().unwrap();
                assert!(may_play(&game, member, A));
            }
            1 => {
                game.phase_out(source); assert!(!may_play(&game, member, A));
                game.phase_in(source); assert!(may_play(&game, member, A));
            }
            2 => {
                let removal = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A,
                    EffectTarget::Specific(source), Modification::RemoveAllAbilities));
                game.refresh_continuous_state().unwrap(); assert!(!may_play(&game, member, A));
                game.effect_store.continuous_effects.remove_effect(removal); game.refresh_continuous_state().unwrap();
                assert!(may_play(&game, member, A));
            }
            3 => {
                let hand = game.move_object_by_game_rule(member, Zone::Hand).unwrap();
                let again = game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
                assert!(!may_play(&game, again, A));
            }
            _ => {
                let hand = game.move_object_by_game_rule(source, Zone::Hand).unwrap();
                assert!(!may_play(&game, member, A));
                game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap();
                assert!(!may_play(&game, member, A));
            }
        }
    }}
}

#[test]
fn copied_trigger_keeps_its_owner_and_pending_trigger_cannot_rebind_a_blinked_host() {
    for definition in definitions() { for blink in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        card(&mut game, B, Zone::Library, false); card(&mut game, B, Zone::Library, false);
        combat_trigger(&mut game, source); let owner = game.stack[0].linked_exile_owner.clone().unwrap();
        let trigger_id = game.stack[0].target_id(); apply(&mut game, source, Effect::copy_spell(ChooseSpec::SpecificObject(trigger_id)));
        assert_eq!(game.stack.len(), 2); assert_eq!(game.stack[1].linked_exile_owner, Some(owner.clone()));
        if blink {
            let hand = game.move_object_by_game_rule(source, Zone::Hand).unwrap();
            game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap();
        }
        game = game.clone(); resolve(&mut game); resolve(&mut game);
        let members = game.linked_exile_pair_members(&owner).unwrap(); assert_eq!(members.len(), 2);
        for member in members { assert_eq!(may_play(&game, *member, A), !blink); }
    }}
}

#[test]
fn separately_acquired_producer_cannot_feed_the_printed_static_permission() {
    for definition in definitions() { for borrowed in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let donor = game.create_object_from_definition(&definition, A, Zone::Exile);
        let modification = if borrowed { Modification::CopyTriggeredAbilities {
            filter: ObjectFilter::specific(donor), exclude_source_name: false, exclude_source_id: true,
        } } else { Modification::AddAbilityGeneric(producer(&definition).clone()) };
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A, EffectTarget::Specific(source), modification));
        game.refresh_continuous_state().unwrap();
        card(&mut game, B, Zone::Library, false); card(&mut game, B, Zone::Library, false);
        combat_trigger(&mut game, source); assert_eq!(game.stack.len(), 2);
        let owners = game.stack.iter().map(|entry| entry.linked_exile_owner.clone().unwrap()).collect::<Vec<_>>();
        assert_ne!(owners[0], owners[1]); resolve(&mut game); resolve(&mut game);
        for owner in owners {
            let members = game.linked_exile_pair_members(&owner).unwrap(); assert_eq!(members.len(), 1);
            assert_eq!(may_play(&game, members[0], A), matches!(owner.acquisition, ironsmith::linked_exile::LinkedExileAcquisition::Printed));
        }
        assert_eq!(game.get_exiled_with_source_links(source).len(), 2);
    }}
}

#[test]
fn native_recovery_is_complete_but_source_only_import_or_missing_admission_is_an_error() {
    for definition in definitions() {
        let mut game = game(); let (source, member) = linked(&mut game, &definition, false); main(&mut game, A);
        let saved = game.clone(); assert!(may_play(&saved, member, A));
        game.replace_exiled_with_source_links(std::collections::HashMap::from([(source, vec![member])]));
        assert!(matches!(compute_legal_actions(&game, A), Err(ExecutionError::IncompleteEvidence(_))));
        game = saved; assert!(may_play(&game, member, A));
        card(&mut game, B, Zone::Library, false); combat_trigger(&mut game, source);
        let library = game.player(B).unwrap().library.clone();
        game.stack.last_mut().unwrap().linked_exile_owner = None;
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker), Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence(_)))));
        assert_eq!(game.player(B).unwrap().library, library);
        assert_eq!(game.get_exiled_with_source_links(source), &[member]);
    }
}

#[test]
fn unproven_static_native_pair_and_multi_producer_compiler_body_fail_closed() {
    for mut definition in definitions() {
        for ability in &mut definition.abilities {
            if let AbilityKind::Static(reader) = &mut ability.kind && let Some(mut spec) = reader.grant_spec() {
                spec.linked_exile_pair = None; *reader = ironsmith::static_abilities::StaticAbility::grants(spec);
            }
        }
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let member = card(&mut game, B, Zone::Exile, false); game.add_exiled_with_source_link(source, member);
        assert!(matches!(compute_legal_actions(&game, A), Err(ExecutionError::IncompleteEvidence(_))));
    }
    let definition = compile_to_runtime_definition("Ambiguous producer scope",
        format!("{}\nWhenever this creature attacks, exile the top card of your library.", text()), false).unwrap();
    assert!(definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Triggered(trigger) => Some(trigger.effects.linked_exile_pair), _ => None,
    }).all(|pair| pair.is_none()));
}


#[test]
fn a_complete_acquired_pair_works_but_a_new_acquisition_cannot_adopt_old_members() {
    for definition in definitions() {
        let mut game = game();
        let host_definition = compile_to_runtime_definition("Acquiring host", "Type: Creature\nPower/Toughness: 2/3", false).unwrap();
        let host = game.create_object_from_definition(&host_definition, A, Zone::Battlefield);
        let first = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(host, A,
            EffectTarget::Specific(host), Modification::SetAbilities(definition.abilities.clone())));
        game.refresh_continuous_state().unwrap();
        card(&mut game, B, Zone::Library, false); combat_trigger(&mut game, host);
        let first_owner = game.stack[0].linked_exile_owner.clone().unwrap(); resolve(&mut game);
        let old_member = game.linked_exile_pair_members(&first_owner).unwrap()[0];
        assert!(may_play(&game, old_member, A));
        game.effect_store.continuous_effects.remove_effect(first); game.refresh_continuous_state().unwrap();
        assert!(!may_play(&game, old_member, A));
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(host, A,
            EffectTarget::Specific(host), Modification::SetAbilities(definition.abilities.clone())));
        game.refresh_continuous_state().unwrap();
        assert!(!may_play(&game, old_member, A));
        card(&mut game, B, Zone::Library, false); combat_trigger(&mut game, host);
        let second_owner = game.stack[0].linked_exile_owner.clone().unwrap();
        assert_ne!(first_owner, second_owner); resolve(&mut game);
        let new_member = game.linked_exile_pair_members(&second_owner).unwrap()[0];
        assert!(may_play(&game, new_member, A)); assert!(!may_play(&game, old_member, A));
    }
}

#[test]
fn failed_replacement_addition_rolls_back_pair_membership_then_native_recovery_replays_once() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = card(&mut game, B, Zone::Library, false);
        combat_trigger(&mut game, source); let owner = game.stack[0].linked_exile_owner.clone().unwrap();
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Library), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::lose_life(ironsmith::effect::Value::X)])));
        let next_id = game.next_object_id_counter();
        let saved = game.clone();
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::UnresolvableValue(_)))));
        assert_eq!(game.player(A).unwrap().life, 20); assert_eq!(game.player(B).unwrap().library, vec![victim]);
        assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty());
        assert!(game.get_exiled_with_source_links(source).is_empty()); assert!(game.exile.is_empty());
        assert_eq!(game.next_object_id_counter(), next_id);
        assert!(game.effect_store.replacement_effects.get_effect(replacement).is_some());
        assert_eq!(game.stack.len(), 1); assert_eq!(game.stack[0].linked_exile_owner, Some(owner.clone()));
        game = saved; game.effect_store.replacement_effects.remove_effect(replacement);
        resolve(&mut game); assert_eq!(game.linked_exile_pair_members(&owner).unwrap().len(), 1);
        let member = game.linked_exile_pair_members(&owner).unwrap()[0]; assert!(may_play(&game, member, A));
    }
}

#[test]
fn an_added_replacement_exile_has_its_own_scope_even_when_it_uses_the_same_source() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = card(&mut game, B, Zone::Library, false); let victim_stable = game.object(victim).unwrap().stable_id;
        let extra = card(&mut game, A, Zone::Library, false); let extra_stable = game.object(extra).unwrap().stable_id;
        combat_trigger(&mut game, source); let owner = game.stack[0].linked_exile_owner.clone().unwrap();
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Library), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::new(ironsmith::effects::ExileTopOfLibraryEffect::new(
                1, ironsmith::target::PlayerFilter::You))])));
        resolve(&mut game);
        let victim = game.find_object_by_stable_id(victim_stable).unwrap();
        let extra = game.find_object_by_stable_id(extra_stable).unwrap();
        assert_eq!(game.object(extra).unwrap().zone, Zone::Exile);
        assert_eq!(game.linked_exile_pair_members(&owner).unwrap(), &[victim]);
        assert_eq!(game.get_exiled_with_source_links(source).len(), 2);
        assert!(may_play(&game, victim, A)); assert!(!may_play(&game, extra, A));
    }
}

#[test]
fn mixed_typed_and_legacy_readers_keep_their_distinct_permission_identities() {
    use ironsmith::grant_registry::GrantPermissionIdentity;
    for definition in definitions() {
        let mut game = game(); let (source, member) = linked(&mut game, &definition, false);
        let unrelated = card(&mut game, B, Zone::Exile, false);
        game.add_exiled_with_source_link(source, unrelated);
        let typed_grants = game.effect_store.grant_registry.granted_play_from_for_card(&game, member, Zone::Exile, A);
        assert_eq!(typed_grants.len(), 1);
        let typed_identity = typed_grants[0].permission_identity.clone().unwrap();
        assert!(matches!(&typed_identity, GrantPermissionIdentity::Static {
            source: host, origin: ironsmith::continuous::AbilityOrigin::Printed(_), ..
        } if *host == source));
        assert!(!may_play(&game, unrelated, A));

        // Deliberately reproduce the unchanged legacy constructor path. This
        // scope has no new pairing guarantee and still reads the source union.
        let AbilityKind::Static(static_reader) = &reader(&definition).kind else { unreachable!() };
        let mut legacy = static_reader.grant_spec().unwrap();
        legacy.requires_linked_exile_pair = false; legacy.linked_exile_pair = None;
        let effect = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A,
            EffectTarget::Specific(source), Modification::AddAbility(ironsmith::static_abilities::StaticAbility::grants(legacy))));
        game.refresh_continuous_state().unwrap();
        let member_grants = game.effect_store.grant_registry.granted_play_from_for_card(&game, member, Zone::Exile, A);
        assert_eq!(member_grants.len(), 2);
        assert!(member_grants.iter().any(|grant| grant.permission_identity.as_ref() == Some(&typed_identity)));
        let legacy_identity = member_grants.iter().filter_map(|grant| grant.permission_identity.as_ref())
            .find(|identity| **identity != typed_identity).unwrap().clone();
        let unrelated_grants = game.effect_store.grant_registry.granted_play_from_for_card(&game, unrelated, Zone::Exile, A);
        assert_eq!(unrelated_grants.len(), 1);
        assert_eq!(unrelated_grants[0].permission_identity.as_ref(), Some(&legacy_identity));
        assert_ne!(legacy_identity, typed_identity);

        game.effect_store.continuous_effects.remove_effect(effect); game.refresh_continuous_state().unwrap();
        let restored = game.effect_store.grant_registry.granted_play_from_for_card(&game, member, Zone::Exile, A);
        assert_eq!(restored.len(), 1); assert_eq!(restored[0].permission_identity.as_ref(), Some(&typed_identity));
        assert!(!may_play(&game, unrelated, A));
    }
}

#[test]
fn legacy_static_grant_wire_shape_and_checksum_do_not_gain_default_binding_fields() {
    let (artifact, _) = compile_to_artifact("Legacy static permission",
        "Type: Enchantment\nYou may play lands from your graveyard.", false).unwrap();
    assert_eq!(artifact.format_version, ironsmith_compiled_artifact::FORMAT_VERSION);
    let bytes = artifact.to_json().unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(!text.contains("\"requires_linked_exile_pair\""));
    assert!(!text.contains("\"linked_exile_pair\""));
    let checksum = artifact.payload_checksum.clone();
    let restored = CompiledCardArtifact::from_json(&bytes).unwrap();
    restored.validate().unwrap(); assert_eq!(restored.payload_checksum, checksum);
    assert_eq!(restored.to_json().unwrap(), bytes,
        "an absent default must not alter the checksum's typed serialization input");
}
