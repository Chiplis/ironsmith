//! Source-authored; compilation and execution are deferred.
use super::*;
use ironsmith::game_state::{FaceDownCastKind, HiddenIdentityCheck};
const A: PlayerId = PlayerId(0);
fn setup() -> (WasmGame, ObjectId, ObjectId, ObjectId) {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["A".into(), "B".into()], 20, 1);
    wasm.game.turn.active_player = A; wasm.game.turn.priority_player = Some(A);
    wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain; wasm.game.turn.step = None;
    wasm.game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 20);
    let mut filter = ironsmith::target::ObjectFilter::creature().owned_by(ironsmith::target::PlayerFilter::You);
    filter.zone = None; filter.power = Some(ironsmith::target::Comparison::LessThanOrEqual(2));
    let spec = ironsmith::grant::GrantSpec::new(ironsmith::grant::Grantable::play_from(), filter, Zone::Library)
        .with_top_card_only().with_usage_limit(ironsmith::grant::GrantUsageLimit::OnceEachTurn);
    let host = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Face-down top permission")
        .card_types(vec![CardType::Enchantment]).with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::grants(spec))).build();
    let host = wasm.game.create_object_from_definition(&host, A, Zone::Battlefield);
    let lower = wasm.game.create_hidden_card_placeholder(A, Zone::Library, 0, "ziffle:face-down:0".into());
    let top = wasm.game.create_hidden_card_placeholder(A, Zone::Library, 1, "ziffle:face-down:1".into());
    (wasm, host, top, lower)
}
fn action_ref(host: ObjectId, card: ObjectId, zone: &str, kind: &str) -> PriorityActionRef {
    PriorityActionRef::CastSpell {spell_id: card.0, from_zone: zone.into(), casting_method: CastingMethodRef::FaceDownPlayFrom {
        source: host.0, zone: zone.into(), face_down_kind: Some(kind.into()), face_down_permission_source: None,
    }}
}
fn priority(game: &GameState) -> ironsmith::decisions::context::PriorityContext {
    ironsmith::decisions::context::PriorityContext::new(game, A,
        ironsmith::decision::compute_legal_actions(game, A).unwrap()).unwrap()
}
fn opened_definition(kind: Option<FaceDownCastKind>) -> ironsmith::cards::CardDefinition {
    let builder = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Opened physical card")
        .card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(7, 7));
    let cost = ironsmith::cost::TotalCost::mana(ironsmith::ManaCost::new());
    let ability = match kind {
        Some(FaceDownCastKind::Morph) => Some(ironsmith::static_abilities::StaticAbility::morph(cost)),
        Some(FaceDownCastKind::Megamorph) => Some(ironsmith::static_abilities::StaticAbility::megamorph(cost)),
        Some(FaceDownCastKind::Disguise) => Some(ironsmith::static_abilities::StaticAbility::disguise(cost)),
        _ => None,
    };
    if let Some(ability) = ability { builder.with_ability(ironsmith::ability::Ability::static_ability(ability)).build() }
    else { builder.build() }
}
#[test]
fn public_keyword_claim_recomputes_exact_source_actions_and_retains_a_library_identity_obligation() {
    let _ids = crate::test_id_counter_guard();
    for kind in [FaceDownCastKind::Morph, FaceDownCastKind::Megamorph, FaceDownCastKind::Disguise] {
        let (mut wasm, host, top, lower) = setup();
        let stale_priority = priority(&wasm.game);
        let reference = action_ref(host, top, "library", kind.as_str());
        assert!(resolve_priority_action(&wasm.game, &stale_priority, None, Some(&reference)).unwrap().is_none());
        // Same public claim extraction/registration and stale-action recovery
        // used by production dispatch. No identity opening is supplied here.
        let (claimed, public_kind) = face_down_cast_claim_for_action_ref(&reference).unwrap();
        wasm.game.set_hidden_face_down_cast_claim(claimed, public_kind);
        let action = resolve_priority_action(&wasm.game, &stale_priority, None, Some(&reference)).unwrap().unwrap();
        assert_eq!(priority_action_ref_for_game(&wasm.game, &action), reference);
        let stable = wasm.game.object(top).unwrap().stable_id;
        let before = wasm.capture_crypto_audit_state();
        let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut wasm.game, &mut queue,
            &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..20 {
            if !state.has_pending_action() { break; }
            let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending cast has no prompt"); };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut wasm.game, &mut queue,
                &mut state, &context, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action());
        let spell = wasm.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(wasm.game.object(spell).unwrap().zone, Zone::Stack);
        assert!(wasm.game.is_face_down(spell)); assert_eq!(wasm.game.current_power(spell), Some(2));
        assert_eq!(wasm.game.cast_origin_snapshot(spell).unwrap().zone, Zone::Library);
        assert!(wasm.game.hidden_identity_obligations().iter().any(|claim|
            claim.stable_id == stable && claim.zone == Zone::Library && claim.check == HiddenIdentityCheck::CastFaceDown(kind)));
        assert!(wasm.game.hidden_identity_obligation_violation(spell, &opened_definition(Some(kind))).is_none());
        assert!(wasm.game.hidden_identity_obligation_violation(spell, &opened_definition(None)).is_some(),
            "the eventual authenticated opening must prove the announced keyword");
        wasm.update_crypto_requirements_from(before);
        assert!(!wasm.last_crypto_requirements.iter().any(|requirement|
            requirement.requirement_type == "public_open" && matches!(requirement.object_id, Some(id) if id == top.0 || id == spell.0)));
        wasm.game.set_hidden_face_down_cast_claim(lower, kind);
        assert!(resolve_priority_action(&wasm.game, &priority(&wasm.game), None,
            Some(&action_ref(host, lower, "library", kind.as_str()))).unwrap().is_none(), "the face-down route spends the shared once-turn grant");
    }
}
#[test]
fn a_keyword_claim_does_not_authorize_a_forged_source_origin_deep_card_or_inactive_host() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, host, top, lower) = setup();
    let stale_priority = priority(&wasm.game);
    for card in [top, lower] { wasm.game.set_hidden_face_down_cast_claim(card, FaceDownCastKind::Morph); }
    for reference in [action_ref(lower, top, "library", "morph"), action_ref(host, top, "graveyard", "morph"),
        action_ref(host, lower, "library", "morph")] {
        assert!(resolve_priority_action(&wasm.game, &stale_priority, None, Some(&reference)).unwrap().is_none());
    }
    wasm.game.phase_out(host);
    assert!(resolve_priority_action(&wasm.game, &stale_priority, None,
        Some(&action_ref(host, top, "library", "morph"))).unwrap().is_none());
}
