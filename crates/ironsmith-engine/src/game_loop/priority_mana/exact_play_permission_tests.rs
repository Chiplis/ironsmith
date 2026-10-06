//! Authored source scenarios. Intentionally unrun during the frozen coverage pass.
use super::*;
use crate::alternative_cast::{AlternativeCastingMethod, GrantSelection};
use crate::cards::CardDefinitionBuilder;
use crate::decision::SelectFirstDecisionMaker;
use crate::grant_registry::{GrantPermissionIdentity, GrantSource, PlayFromConstraints};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;
use ironsmith_core::value_model::ManaSpendMode;

fn fixture(cost: ManaSymbol, modes: &[ManaSpendMode]) -> (GameState, ObjectId, ObjectId, Vec<CastingMethod>) {
    let player = PlayerId::from_index(0);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    let source = CardDefinitionBuilder::new(CardId::new(), "Permission source").card_types(vec![CardType::Creature]).build();
    let source = game.create_object_from_definition(&source, player, Zone::Battlefield);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Permission spell")
        .card_types(vec![CardType::Sorcery]).mana_cost(ManaCost::from_symbols(vec![cost]))
        .alternative_cast(AlternativeCastingMethod::alternative_cost("Printed price", Some(ManaCost::from_symbols(vec![cost])), vec![])).build();
    let card = game.create_object_from_definition(&spell, player, Zone::Exile);
    game.player_mut(player).unwrap().mana_pool.red = 1;
    for mode in modes {
        game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, player,
            PlayFromConstraints { cast_mana_spend_mode: *mode, ..Default::default() },
            GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
    }
    let methods = modes.iter().enumerate().map(|(index, _)| CastingMethod::ExactPermission {
        origin: Box::new(CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: None }),
        permission: GrantSelection { source, index, identity: game.effect_store.grant_registry.grants[index].permission_identity.clone().unwrap() },
    }).collect();
    (game, source, card, methods)
}

fn legacy_mana(game: &mut GameState, source: ObjectId, card: ObjectId, identity: Option<GrantPermissionIdentity>) {
    let player = PlayerId::from_index(0);
    let permission = crate::effect::ManaSpendPermission::any_type_for_casting_stable_ids(
        crate::target::PlayerFilter::You, vec![game.object(card).unwrap().stable_id]);
    game.effect_store.mana_spend_effects.permissions.push(crate::game_state::ActiveManaSpendPermission {
        permission, controller: player, play_permission_identities: identity.map(|identity| vec![identity]),
        source: crate::game_state::ManaSpendPermissionSource::Effect { source_id: source, expires_end_of_turn: u32::MAX },
    });
}

#[test]
fn exact_menu_and_payment_distinguish_color_from_type_for_same_host() {
    let player = PlayerId::from_index(0);
    let (game, _, card, methods) = fixture(ManaSymbol::Colorless, &[ManaSpendMode::AnyColor, ManaSpendMode::AnyType]);
    let actions = crate::decision::compute_legal_actions(&game, player).unwrap();
    for (method, expected) in methods.iter().zip([false, true]) {
        assert_eq!(actions.iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. }
            if *spell_id == card && casting_method == method)), expected);
        assert_eq!(crate::decision::can_cast_spell(&game, player, game.object(card).unwrap(), method), expected);
        let mut proposal = game.clone();
        let stack = propose_spell_cast(&mut proposal, card, Zone::Exile, player, method).unwrap();
        let policy = proposal.mana_spend_policy_for_cast(player, Some(stack));
        assert_eq!(proposal.try_pay_mana_cost_with_policy(player, Some(stack), &ManaCost::from_symbols(vec![ManaSymbol::Colorless]),
            0, crate::costs::PaymentReason::CastSpell, &policy).unwrap(), expected);
        assert_eq!(proposal.player(player).unwrap().mana_pool.red, u32::from(!expected));
    }
}

#[test]
fn source_only_forgery_cannot_announce_marked_reader_even_with_a_printed_price() {
    let player = PlayerId::from_index(0);
    let (game, source, card, _) = fixture(ManaSymbol::Blue, &[ManaSpendMode::AnyColor, ManaSpendMode::AnyType]);
    for alternative in [None, Some(0)] {
        let mut rejected = game.clone();
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let response = PriorityResponse::PriorityAction(LegalAction::CastSpell { spell_id: card, from_zone: Zone::Exile,
            casting_method: CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: alternative } });
        assert!(super::super::priority_apply::apply_priority_response_with_dm(&mut rejected, &mut queue, &mut state,
            &response, &mut SelectFirstDecisionMaker).is_err());
        assert_eq!(rejected.object(card).unwrap().zone, Zone::Exile);
        assert_eq!(rejected.player(player).unwrap().mana_pool.red, 1);
        assert!(rejected.stack.is_empty());
        assert!(state.pending_cast.is_none());
        assert_eq!(rejected.effect_store.grant_registry.grants.len(), 2);
    }
}

#[test]
fn legacy_reader_retains_ordinary_authority_but_cannot_take_the_new_readers_mana() {
    let player = PlayerId::from_index(0);
    let (mut game, source, card, _) = fixture(ManaSymbol::Blue, &[ManaSpendMode::AnyColor, ManaSpendMode::Normal]);
    let raw = CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: None };
    let expected = game.effect_store.grant_registry.grants[1].permission_identity.clone();
    let stack = propose_spell_cast(&mut game, card, Zone::Exile, player, &raw).unwrap();
    assert_eq!(game.object(stack).unwrap().cast_grant_usage_identity.as_deref(), expected.as_ref());
    assert!(game.object(stack).unwrap().cast_play_permission.is_none());
    let policy = game.mana_spend_policy_for_cast(player, Some(stack));
    assert!(!policy.can_pay_symbol(ManaSymbol::Red, ManaSymbol::Blue));
    assert!(!game.try_pay_mana_cost_with_policy(player, Some(stack), &ManaCost::from_symbols(vec![ManaSymbol::Blue]),
        0, crate::costs::PaymentReason::CastSpell, &policy).unwrap());
}

#[test]
fn exact_selection_excludes_unrelated_legacy_tagged_rider_but_keeps_independent_rule() {
    let player = PlayerId::from_index(0);
    let (mut game, source, card, methods) = fixture(ManaSymbol::Colorless, &[ManaSpendMode::AnyColor, ManaSpendMode::Normal]);
    let other = game.effect_store.grant_registry.grants[1].permission_identity.clone().unwrap();
    legacy_mana(&mut game, source, card, Some(other));
    assert!(game.mana_spend_policy(player, Some(card)).can_pay_symbol(ManaSymbol::Red, ManaSymbol::Colorless));
    assert!(!crate::decision::can_cast_spell(&game, player, game.object(card).unwrap(), &methods[0]));
    let mut exact = game.clone();
    let stack = propose_spell_cast(&mut exact, card, Zone::Exile, player, &methods[0]).unwrap();
    assert!(!exact.mana_spend_policy_for_cast(player, Some(stack)).can_pay_symbol(ManaSymbol::Red, ManaSymbol::Colorless));
    legacy_mana(&mut game, source, card, None);
    assert!(crate::decision::can_cast_spell(&game, player, game.object(card).unwrap(), &methods[0]));
    let stack = propose_spell_cast(&mut game, card, Zone::Exile, player, &methods[0]).unwrap();
    assert!(game.mana_spend_policy_for_cast(player, Some(stack)).can_pay_symbol(ManaSymbol::Red, ManaSymbol::Colorless));
}

#[test]
fn pending_receipt_survives_provider_cost_removal_and_clone_but_missing_authority_is_explicit() {
    let player = PlayerId::from_index(0);
    let (mut game, source, card, methods) = fixture(ManaSymbol::Blue, &[ManaSpendMode::AnyColor]);
    let stack = propose_spell_cast(&mut game, card, Zone::Exile, player, &methods[0]).unwrap();
    let expected = game.object(stack).unwrap().cast_play_permission.clone();
    game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
    game.effect_store.grant_registry.grants.clear();
    let mut saved = game.clone();
    assert_eq!(saved.object(stack).unwrap().cast_play_permission, expected);
    let mut pending = PendingCast::new(stack, Zone::Exile, player, crate::provenance::ProvNodeId::default(),
        CastStage::PayingMana, None, vec![], methods[0].clone(), Default::default(), None, stack);
    pending.mana_cost_to_pay = Some(ManaCost::from_symbols(vec![ManaSymbol::Blue]));
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let progress = prompt_spell_mana_ability_window(&mut saved, &mut queue, &mut state, pending, &mut SelectFirstDecisionMaker).unwrap();
    let GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::ManaPayment(context)) = progress else {
        panic!("selected receipt must produce the ordinary mana payment decision");
    };
    let response = crate::mana_payment::ManaPaymentResponse::Confirm { plan_id: context.plan.id, request_hash: context.plan.request_hash };
    apply_mana_payment_plan_response(&mut saved, &mut queue, &mut state, &response, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(saved.player(player).unwrap().mana_pool.red, 0);
    assert!(saved.stack.iter().any(|entry| entry.object_id == stack && entry.casting_method == methods[0]));
    assert!(!game.mana_spend_policy_for_cast(PlayerId::from_index(1), Some(stack)).can_pay_symbol(ManaSymbol::Red, ManaSymbol::Blue));
    game.object_mut(stack).unwrap().cast_play_permission = None;
    assert!(matches!(game.try_mana_spend_policy_for_cast(player, Some(stack)), Err(crate::effects::ExecutionError::IncompleteEvidence(_))));
}

#[test]
fn stale_identity_wrong_face_and_zone_reentry_never_adopt_a_new_reader() {
    let player = PlayerId::from_index(0);
    let (game, source, card, methods) = fixture(ManaSymbol::Blue, &[ManaSpendMode::AnyColor]);
    let mut stale = game.clone();
    stale.effect_store.grant_registry.grants.clear();
    stale.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, player,
        PlayFromConstraints { cast_mana_spend_mode: ManaSpendMode::AnyType, ..Default::default() },
        GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
    assert!(propose_spell_cast(&mut stale, card, Zone::Exile, player, &methods[0]).is_err());
    let mut wrong_face = methods[0].clone();
    if let CastingMethod::ExactPermission { origin, .. } = &mut wrong_face {
        *origin = Box::new(CastingMethod::SplitOtherHalfPlayFrom { source, zone: Zone::Exile, use_alternative: None });
    }
    assert!(propose_spell_cast(&mut game.clone(), card, Zone::Exile, player, &wrong_face).is_err());
    let mut departed = game.clone();
    let hand = departed.move_object_by_game_rule(card, Zone::Hand).unwrap();
    let exile = departed.move_object_by_game_rule(hand, Zone::Exile).unwrap();
    assert!(propose_spell_cast(&mut departed, exile, Zone::Exile, player, &methods[0]).is_err());
    assert!(departed.object(exile).unwrap().cast_play_permission.is_none());
}

#[test]
fn direct_public_payment_requires_complete_receipt_even_when_ordinary_mana_is_sufficient() {
    let player = PlayerId::from_index(0);
    let (mut game, _, card, methods) = fixture(ManaSymbol::Blue, &[ManaSpendMode::AnyColor]);
    let stack = propose_spell_cast(&mut game, card, Zone::Exile, player, &methods[0]).unwrap();
    let receipt = game.object(stack).unwrap().cast_play_permission.clone();
    game.player_mut(player).unwrap().mana_pool.blue = 1;
    game.object_mut(stack).unwrap().cast_play_permission = None;
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Blue]);
    for route in 0..3 {
        let mut rejected = game.clone();
        let result = match route {
            0 => rejected.try_pay_mana_cost_with_reason(player, Some(stack), &cost, 0, crate::costs::PaymentReason::CastSpell),
            1 => rejected.try_pay_mana_cost_with_reason_and_dm(player, Some(stack), &cost, 0,
                crate::costs::PaymentReason::CastSpell, &mut SelectFirstDecisionMaker),
            _ => rejected.try_pay_mana_cost_with_policy(player, Some(stack), &cost, 0,
                crate::costs::PaymentReason::CastSpell, &crate::player::ManaSpendPolicy::default()),
        };
        assert!(matches!(result, Err(crate::effects::ExecutionError::IncompleteEvidence(_))));
        assert_eq!(rejected.player(player).unwrap().mana_pool.blue, 1);
        assert_eq!(rejected.player(player).unwrap().mana_pool.red, 1);
        assert_eq!(rejected.object(stack).unwrap().zone, Zone::Stack);
    }
    assert!(!game.can_pay_mana_cost_with_reason(player, Some(stack), &cost, 0, crate::costs::PaymentReason::CastSpell));
    game.object_mut(stack).unwrap().cast_play_permission = receipt;
    assert!(game.try_pay_mana_cost_with_reason(player, Some(stack), &cost, 0, crate::costs::PaymentReason::CastSpell).unwrap());
    assert_eq!(game.player(player).unwrap().mana_pool.total(), 1);
}

fn marked_methods_for(game: &GameState, card: ObjectId) -> Vec<CastingMethod> {
    crate::decision::compute_legal_actions(game, PlayerId::from_index(0)).unwrap().into_iter().filter_map(|action| match action {
        LegalAction::CastSpell { spell_id, casting_method: method @ CastingMethod::ExactPermission { .. }, .. } if spell_id == card => Some(method),
        _ => None,
    }).collect()
}

#[test]
fn bestow_permission_queries_the_aura_face_and_an_excluded_face_does_not_poison_the_menu() {
    let player = PlayerId::from_index(0);
    for aura_only in [true, false] {
        let (mut game, source, _, _) = fixture(ManaSymbol::Blue, &[ManaSpendMode::AnyColor]);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Bestow face probe")
            .card_types(vec![CardType::Enchantment, CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .alternative_cast(AlternativeCastingMethod::Bestow { total_cost: crate::cost::TotalCost::mana(
                ManaCost::from_symbols(vec![ManaSymbol::Blue])) }).build();
        let card = game.create_object_from_definition(&definition, player, Zone::Exile);
        let grant = &mut game.effect_store.grant_registry.grants[0]; grant.target_id = Some(card);
        let mut filter = if aura_only { crate::target::ObjectFilter::enchantment() } else { crate::target::ObjectFilter::creature() };
        if aura_only { filter.excluded_card_types.push(CardType::Creature); }
        grant.filter = Some(filter);
        let methods = marked_methods_for(&game, card);
        assert_eq!(methods.len(), 1);
        assert!(matches!(methods[0].origin_method(), CastingMethod::PlayFrom { use_alternative, .. }
            if *use_alternative == aura_only.then_some(0)));
        let mut invalid = methods[0].clone();
        if let CastingMethod::ExactPermission { origin, .. } = &mut invalid {
            *origin = Box::new(CastingMethod::PlayFrom { source, zone: Zone::Exile,
                use_alternative: (!aura_only).then_some(0) });
        }
        assert!(propose_spell_cast(&mut game.clone(), card, Zone::Exile, player, &invalid).is_err());
        let stack = propose_spell_cast(&mut game, card, Zone::Exile, player, &methods[0]).unwrap();
        assert_eq!(game.object(stack).unwrap().bestow_cast_state.is_some(), aura_only);
        assert_eq!(game.object(stack).unwrap().has_card_type(CardType::Creature), !aura_only);
        assert!(game.try_pay_mana_cost_with_reason(player, Some(stack), &ManaCost::from_symbols(vec![ManaSymbol::Blue]),
            0, crate::costs::PaymentReason::CastSpell).unwrap());
    }
}

#[test]
fn prototype_on_a_linked_face_is_selected_before_permission_filter_and_price() {
    let player = PlayerId::from_index(0);
    for linked in [false, true] {
        let (mut game, _, _, _) = fixture(ManaSymbol::Blue, &[ManaSpendMode::AnyColor]);
        let back = CardDefinitionBuilder::new(CardId::new(), "Prototype chosen face")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(7, 7))
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(6), ManaSymbol::Blue]))
            .alternative_cast(AlternativeCastingMethod::prototype(ManaCost::from_symbols(vec![ManaSymbol::Blue]),
                crate::card::PowerToughness::fixed(2, 2))).build();
        let mut front = CardDefinitionBuilder::new(CardId::new(), "Prototype front face")
            .card_types(vec![CardType::Land]).build();
        front.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
        front.card.other_face = Some(back.card.id); front.card.other_face_name = Some(back.card.name.to_string());
        game.register_linked_face_definition(&back);
        let card = game.create_object_from_definition(if linked { &front } else { &back }, player, Zone::Exile);
        let grant = &mut game.effect_store.grant_registry.grants[0]; grant.target_id = Some(card);
        grant.filter = Some(crate::target::ObjectFilter::creature().with_mana_value(crate::filter::Comparison::Equal(1)));
        let methods = marked_methods_for(&game, card);
        assert_eq!(methods.len(), 1);
        assert!(match methods[0].origin_method() {
            CastingMethod::PlayFrom { use_alternative: Some(0), .. } => !linked,
            CastingMethod::SplitOtherHalfPlayFrom { use_alternative: Some(0), .. } => linked,
            _ => false,
        });
        let stack = propose_spell_cast(&mut game, card, Zone::Exile, player, &methods[0]).unwrap();
        let spell = game.object(stack).unwrap();
        assert!(spell.prototype_cast_state.is_some());
        assert_eq!(spell.mana_cost.as_ref().unwrap().mana_value(), 1);
        let cost = crate::decision::spell_mana_cost_for_cast(&game, player, spell, &methods[0], Zone::Exile).unwrap();
        assert_eq!(cost, ManaCost::from_symbols(vec![ManaSymbol::Blue]));
        assert!(game.try_pay_mana_cost_with_reason(player, Some(stack), &cost, 0, crate::costs::PaymentReason::CastSpell).unwrap());
    }
}
