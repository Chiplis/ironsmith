//! Authored only. Exercise production opening preparation and JSON snapshots;
//! compilation/execution remain deferred under the campaign workflow.
use super::*;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn setup(public: bool) -> (WasmGame, ObjectId, ObjectId, ObjectId) {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["A".into(), "B".into()], 20, 1);
    let ability = if public {
        ironsmith::static_abilities::StaticAbility::all_players_look_at_top_cards_of_libraries()
    } else { ironsmith::static_abilities::StaticAbility::look_at_top_card_of_library() };
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Static top visibility probe")
        .card_types(vec![CardType::Enchantment]).with_ability(ironsmith::ability::Ability::static_ability(ability)).build();
    let host = wasm.game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let next = wasm.game.create_hidden_card_placeholder(A, Zone::Library, 0, "next-top-commitment".into());
    let old = wasm.game.create_hidden_card_placeholder(A, Zone::Library, 1, "original-top-commitment".into());
    wasm.game.refresh_continuous_state().unwrap();
    (wasm, host, old, next)
}
fn pending_cast(wasm: &mut WasmGame, source: ObjectId, stack: ObjectId, checkpoint: ReplayCheckpoint) {
    let mut pending = ironsmith::game_loop::PendingCast::new(
        stack, Zone::Library, A, ironsmith::provenance::ProvNodeId::default(),
        ironsmith::game_loop::CastStage::PayingMana, None, Vec::new(),
        ironsmith::alternative_cast::CastingMethod::PlayFrom {source, zone: Zone::Library, use_alternative: None},
        ironsmith::cost::OptionalCostsPaid::new(0), None, stack,
    );
    pending.display_mana_pips = vec![vec![ManaSymbol::White]];
    wasm.priority_state.pending_cast = Some(pending);
    wasm.priority_state.checkpoint = Some((*checkpoint.game).clone());
    wasm.pending_action_checkpoint = Some(checkpoint);
}
fn has_top_opening(wasm: &WasmGame, id: ObjectId) -> bool {
    wasm.last_crypto_requirements.iter().any(|requirement|
        requirement.object_id == Some(id.0) && matches!(requirement.requirement_type.as_str(), "private_open" | "public_open"))
}
fn top_flag(wasm: &mut WasmGame, player: PlayerId) -> bool {
    let snapshot: serde_json::Value = serde_json::from_str(&wasm.snapshot_json_for_host().unwrap()).unwrap();
    snapshot["players"].as_array().unwrap().iter().find(|row| row["id"] == player.0).unwrap()["can_view_library_top"].as_bool().unwrap()
}
#[test]
fn changed_top_is_withheld_in_both_opening_preparation_and_ui_until_cast_completion() {
    let _ids = crate::test_id_counter_guard();
    for public in [false, true] {
        let (mut wasm, host, old, next) = setup(public);
        let checkpoint = wasm.capture_replay_checkpoint();
        let before = wasm.capture_crypto_audit_state();
        let stack = wasm.game.move_object_by_effect(old, Zone::Stack).unwrap();
        pending_cast(&mut wasm, host, stack, checkpoint);
        wasm.update_crypto_requirements_from(before);
        assert!(!has_top_opening(&wasm, next));
        assert!(!top_flag(&mut wasm, A));
        let during_hash = wasm.static_library_top_visibility_hash();
        wasm.priority_state.pending_cast = None; wasm.priority_state.clear_checkpoint();
        wasm.pending_action_checkpoint = None;
        assert_ne!(during_hash, wasm.static_library_top_visibility_hash(), "cached UI must notice the completed action");
        let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
        assert!(has_top_opening(&wasm, next)); assert!(top_flag(&mut wasm, A));
    }
}
#[test]
fn unchanged_top_stays_available_and_an_unrelated_library_is_not_suppressed() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, host, old, _) = setup(true);
    let other = wasm.game.create_hidden_card_placeholder(B, Zone::Library, 0, "other-top".into());
    let checkpoint = wasm.capture_replay_checkpoint();
    pending_cast(&mut wasm, host, old, checkpoint);
    let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
    assert!(has_top_opening(&wasm, old)); assert!(top_flag(&mut wasm, A));
    wasm.game.move_object_by_effect(old, Zone::Hand).unwrap();
    let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
    assert!(!top_flag(&mut wasm, A)); assert!(top_flag(&mut wasm, B));
    assert!(has_top_opening(&wasm, other));
}
#[test]
fn same_top_after_shuffle_still_cannot_publish_its_new_position_mid_action() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, host, old, _) = setup(false);
    let checkpoint = wasm.capture_replay_checkpoint();
    pending_cast(&mut wasm, host, old, checkpoint);
    let order = wasm.game.player(A).unwrap().library.to_vec();
    let revision = wasm.game.library_top_revision(A);
    wasm.game.queue_transcript_library_shuffle_order(A, order.clone(), order);
    wasm.game.shuffle_player_library(A);
    assert_eq!(wasm.game.player(A).unwrap().library.last(), Some(&old));
    assert_eq!(wasm.game.library_top_revision(A), revision);
    let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
    assert!(!has_top_opening(&wasm, old)); assert!(!top_flag(&mut wasm, A));
}
#[test]
fn rollback_restores_original_top_and_missing_action_boundary_fails_closed() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, host, old, next) = setup(false);
    let checkpoint = wasm.capture_replay_checkpoint();
    let stack = wasm.game.move_object_by_effect(old, Zone::Stack).unwrap();
    pending_cast(&mut wasm, host, stack, checkpoint);
    assert!(!top_flag(&mut wasm, A));
    assert!(wasm.priority_state.rollback_action(&mut wasm.game));
    wasm.pending_action_checkpoint = None;
    let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
    assert!(has_top_opening(&wasm, old)); assert!(!has_top_opening(&wasm, next)); assert!(top_flag(&mut wasm, A));
    let checkpoint = wasm.capture_replay_checkpoint(); pending_cast(&mut wasm, host, old, checkpoint);
    wasm.pending_action_checkpoint = None; wasm.priority_state.checkpoint = None;
    assert!(!top_flag(&mut wasm, A));
    let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
    assert!(!has_top_opening(&wasm, old));
}

#[derive(Default)]
struct CaptureAnnouncementPrompt {
    snapshot: Option<GameState>,
    waiting: bool,
}
impl ironsmith::decision::DecisionMaker for CaptureAnnouncementPrompt {
    fn awaiting_choice(&self) -> bool { self.waiting }
    fn decide_targets(&mut self, game: &GameState, _: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
        self.snapshot = Some(game.clone()); self.waiting = true; Vec::new()
    }
    fn decide_options(&mut self, game: &GameState, _: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        self.snapshot = Some(game.clone()); self.waiting = true; Vec::new()
    }
}
#[test]
fn resolving_effects_local_cast_boundary_reaches_openings_without_a_host_action_checkpoint() {
    use ironsmith::effects::{EffectContext, EffectExecutor};
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, _, old, next) = setup(false);
    // The outer instruction already changed the library before it starts
    // casting. Its entry boundary must not be mistaken for the cast boundary.
    wasm.game.move_object_by_effect(old, Zone::Hand).unwrap();
    let spell = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Nested source cast")
        .card_types(vec![CardType::Sorcery]).mana_cost(ironsmith::ManaCost::new())
        .with_spell_effect(vec![ironsmith::Effect::new(ironsmith::effects::DealDamageEffect::new(
            1, ironsmith::target::ChooseSpec::target_player()))]).build();
    let candidate = wasm.game.create_object_from_definition(&spell, A, Zone::Library);
    let before = wasm.capture_crypto_audit_state();
    let mut dm = CaptureAnnouncementPrompt::default();
    let result = {
        let mut context = EffectContext::new(candidate, A, &mut dm);
        ironsmith::effects::CastSourceEffect::new().without_paying_mana_cost()
            .execute(&mut wasm.game, &mut context)
    };
    assert!(result.is_ok()); assert!(dm.waiting);
    assert!(!wasm.priority_state.has_pending_action()); assert!(wasm.pending_action_checkpoint.is_none());
    assert!(!wasm.game.has_library_top_announcement(), "live execution rolled back while the decision clone retained its exact boundary");
    let decision = dm.snapshot.take().unwrap();
    assert!(decision.has_library_top_announcement());
    assert_eq!(decision.player(A).unwrap().library.last(), Some(&next));
    assert!(!decision.static_library_top_visible_during_announcements(A));
    wasm.pending_decision_game = Some(Box::new(decision));
    wasm.update_crypto_requirements_from(before);
    assert!(!has_top_opening(&wasm, next)); assert!(!top_flag(&mut wasm, A));
    let retained = RuntimeSavepoint::capture(&wasm);
    wasm.pending_decision_game = None; retained.restore(&mut wasm);
    assert!(wasm.pending_decision_game.as_ref().unwrap().has_library_top_announcement());
    assert!(!top_flag(&mut wasm, A));
}
#[test]
fn both_land_play_owners_keep_the_boundary_through_entry_program_choices() {
    use ironsmith::effects::{ChooseColorEffect, DrawCardsEffect};
    use ironsmith::static_abilities::{CompiledStaticAbility, StaticAbility};
    let _ids = crate::test_id_counter_guard();
    for priority_route in [false, true] {
        let (mut wasm, _, _, next) = setup(false);
        wasm.game.turn.active_player = A; wasm.game.turn.priority_player = Some(A);
        wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain; wasm.game.turn.step = None;
        let program = ironsmith::resolution::ResolutionProgram::from_effects(vec![
            ironsmith::Effect::new(DrawCardsEffect::you(1)),
            ironsmith::Effect::new(ChooseColorEffect::new(ironsmith::target::PlayerFilter::You)),
        ]);
        let land = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Land with entry program")
            .card_types(vec![CardType::Land]).with_ability(ironsmith::ability::Ability::static_ability(
                StaticAbility::from_model(CompiledStaticAbility::as_enters_effect_program(
                    program, "this land", false, false, None)))).build();
        let land = wasm.game.create_object_from_definition(&land, A, Zone::Hand);
        let before = wasm.capture_crypto_audit_state(); let mut dm = CaptureAnnouncementPrompt::default();
        if priority_route {
            let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new();
            let _ = ironsmith::game_loop::apply_priority_response_with_dm(&mut wasm.game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PlayLand {land_id: land}), &mut dm);
        } else {
            let _ = ironsmith::special_actions::perform(
                ironsmith::special_actions::SpecialAction::PlayLand {card_id: land}, &mut wasm.game, A, &mut dm);
        }
        assert!(dm.waiting); assert!(!wasm.priority_state.has_pending_action());
        assert!(wasm.pending_action_checkpoint.is_none());
        let decision = dm.snapshot.take().unwrap();
        assert!(decision.has_library_top_announcement());
        assert_eq!(decision.player(A).unwrap().library.last(), Some(&next));
        assert!(!decision.static_library_top_visible_during_announcements(A));
        assert!(!wasm.game.has_library_top_announcement(), "pending land play rolls back the live state");
        wasm.pending_decision_game = Some(Box::new(decision));
        wasm.update_crypto_requirements_from(before);
        assert!(!has_top_opening(&wasm, next)); assert!(!top_flag(&mut wasm, A));
    }
}

#[test]
fn resolved_top_view_permission_opens_each_new_top_privately_survives_source_and_expires() {
    use ironsmith::effects::{EffectContext, EffectExecutor};
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, host, old, next) = setup(false);
    // Remove the static viewer: the resolving registration is the only source.
    let source = wasm.game.move_object_by_effect(host, Zone::Exile).unwrap();
    let mut spec = ironsmith::grant::GrantSpec::new(ironsmith::grant::Grantable::play_from(),
        ironsmith::target::ObjectFilter::default().owned_by(ironsmith::target::PlayerFilter::You), Zone::Library).with_top_card_only();
    spec.may_look_at_top = true;
    let effect = ironsmith::effects::GrantBySpecEffect::new(spec, ironsmith::target::PlayerFilter::You,
        ironsmith::grant::GrantDuration::UntilEndOfTurn);
    let before = wasm.capture_crypto_audit_state();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    effect.execute(&mut wasm.game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
    wasm.update_crypto_requirements_from(before);
    assert!(top_flag(&mut wasm, A));
    assert!(wasm.last_crypto_requirements.iter().any(|r| r.object_id == Some(old.0) && r.requirement_type == "private_open"));
    assert!(!wasm.last_crypto_requirements.iter().any(|r| r.requirement_type == "public_open"));
    let saved = RuntimeSavepoint::capture(&wasm);
    saved.restore(&mut wasm);
    let checkpoint = wasm.capture_replay_checkpoint();
    let stack = wasm.game.move_object_by_effect(old, Zone::Stack).unwrap();
    pending_cast(&mut wasm, source, stack, checkpoint);
    let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
    assert!(!has_top_opening(&wasm, next)); assert!(!top_flag(&mut wasm, A));
    wasm.priority_state.pending_cast = None; wasm.priority_state.clear_checkpoint(); wasm.pending_action_checkpoint = None;
    let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
    assert!(has_top_opening(&wasm, next)); assert!(top_flag(&mut wasm, A));
    wasm.game.next_turn();
    let before = wasm.capture_crypto_audit_state(); wasm.update_crypto_requirements_from(before);
    assert!(!has_top_opening(&wasm, next)); assert!(!top_flag(&mut wasm, A));
}

#[test]
fn retained_permission_reflexive_token_program_keeps_its_explicit_embedded_definition_graph() {
    use ironsmith::effects::{EffectContext, EffectExecutor};
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, host, _, _) = setup(false);
    wasm.game.turn.active_player = A; wasm.game.turn.priority_player = Some(A);
    wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain; wasm.game.turn.step = None;
    let mut spec = ironsmith::grant::GrantSpec::new(ironsmith::grant::Grantable::play_from(),
        ironsmith::target::ObjectFilter::default().owned_by(ironsmith::target::PlayerFilter::You), Zone::Library).with_top_card_only();
    spec.on_use_effects = vec![ironsmith::Effect::new(ironsmith::effects::CreateTokenEffect::one(
        ironsmith::cards::tokens::food_token_definition()))];
    let effect = ironsmith::effects::GrantBySpecEffect::new(spec, ironsmith::target::PlayerFilter::You,
        ironsmith::grant::GrantDuration::UntilEndOfTurn);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    effect.execute(&mut wasm.game, &mut EffectContext::new(host, A, &mut dm)).unwrap();
    let saved = RuntimeSavepoint::capture(&wasm);
    saved.restore(&mut wasm);
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Permission land").card_types(vec![CardType::Land]).build();
    let land = wasm.game.create_object_from_definition(&definition, A, Zone::Library);
    ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::PlayLand {card_id: land}, &mut wasm.game, A, &mut dm).unwrap();
    assert!(!wasm.game.battlefield.iter().any(|id| wasm.game.current_has_subtype(*id, ironsmith::types::Subtype::Food)));
    let mut queue = TriggerQueue::new(); ironsmith::game_loop::put_triggers_on_stack(&mut wasm.game, &mut queue).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();
    assert!(wasm.game.battlefield.iter().any(|id| wasm.game.current_has_subtype(*id, ironsmith::types::Subtype::Food)));
}
