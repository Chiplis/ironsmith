//! CR 406.3a's cast-face-down exception. The initial action and declaration
//! choices are public and uniform; authenticated identity claims use the
//! existing face-down cast/payment owner without opening or private inspection.
use super::*;
use crate::alternative_cast::{GrantSelection, blind_play};

pub(super) fn begin(
    game: &GameState, queue: &TriggerQueue, state: &mut PriorityLoopState, card_id: ObjectId,
    incarnation: Option<u64>, permission: &GrantSelection,
) -> Result<GameProgress, GameLoopError> {
    let player = game.turn.priority_player.ok_or_else(|| GameLoopError::InvalidState("No priority player".into()))?;
    if state.has_pending_action() || !blind_play::requires_opening(game, card_id, player) {
        return Err(GameLoopError::InvalidState("Card is not awaiting a blind face-down declaration".into()));
    }
    blind_play::validate_incarnation(game, card_id, incarnation)?;
    blind_play::resolve(game, card_id, player, permission)?;
    let kinds = game.blind_face_down_cast_kinds(player);
    let kind_source_public_ids = kinds.iter().filter_map(|kind| kind.permission_source()).map(|source| {
        game.face_down_permission_source_public_id(source).map(|public_id| (source, public_id))
            .ok_or_else(|| crate::effects::ExecutionError::IncompleteEvidence(
                "face-down permission lost its exact source public identity".into()))
    }).collect::<Result<_, _>>()?;
    state.exile_face_down_root_queue = Some(Box::new(queue.clone()));
    state.pending_exile_face_down = Some(PendingExileFaceDownCast {
        card_id, incarnation, player, permission: permission.clone(), kinds, kind_source_public_ids, declared_kind: None,
    });
    resume(game, state)
}

pub(super) fn resume(game: &GameState, state: &PriorityLoopState) -> Result<GameProgress, GameLoopError> {
    let pending = state.pending_exile_face_down.as_ref().ok_or_else(|| GameLoopError::InvalidState("No face-down declaration".into()))?;
    if game.turn.priority_player != Some(pending.player) { return Err(GameLoopError::InvalidState("Face-down declaration belongs to another player".into())); }
    blind_play::validate_incarnation(game, pending.card_id, pending.incarnation)?;
    blind_play::resolve(game, pending.card_id, pending.player, &pending.permission)?;
    let mut options = pending.kinds.iter().enumerate().map(|(index, kind)|
        crate::decisions::context::SelectableOption::new(index, match kind {
            crate::game_state::FaceDownCastKind::Permission { source } => format!("Declare effect permission from {}",
                game.object(*source).map_or_else(|| "another source".to_string(), |object| object.name.to_string())),
            _ => format!("Declare {}", kind.as_str()),
        })).collect::<Vec<_>>();
    options.push(crate::decisions::context::SelectableOption::new(options.len(), "Cancel face-down cast"));
    let mut context = crate::decisions::context::SelectOptionsContext::new(pending.player, Some(pending.card_id),
        "Declare how to cast the exiled card face down", options, 1, 1);
    context.exile_face_down_choice = true;
    Ok(GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::SelectOptions(context)))
}

pub(super) fn apply_choice(
    game: &mut GameState, queue: &mut TriggerQueue, state: &mut PriorityLoopState,
    choice: usize, dm: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_exile_face_down.clone().ok_or_else(|| GameLoopError::InvalidState("No face-down declaration".into()))?;
    if game.turn.priority_player != Some(pending.player) { return Err(GameLoopError::InvalidState("Face-down declaration belongs to another player".into())); }
    blind_play::validate_incarnation(game, pending.card_id, pending.incarnation)?;
    blind_play::resolve(game, pending.card_id, pending.player, &pending.permission)?;
    if choice == pending.kinds.len() {
        let original_queue = state.exile_face_down_root_queue.take().ok_or_else(|| crate::effects::ExecutionError::IncompleteEvidence(
            "face-down cancellation lost its original trigger queue".into()))?;
        *queue = *original_queue;
        game.clear_hidden_face_down_cast_claim(pending.card_id);
        state.pending_exile_face_down = None; state.declared_exile_face_down = None;
        state.checkpoint = None;
        return advance_priority_with_dm(game, queue, dm);
    }
    let kind = *pending.kinds.get(choice).ok_or_else(|| ResponseError::IllegalChoice("Choose one offered declaration".into()))?;
    if pending.declared_kind.is_some_and(|declared| declared != kind) {
        return Err(ResponseError::IllegalChoice("Resume the original face-down declaration or cancel it".into()).into());
    }
    // An error does not accept a new public declaration. Its retry state must
    // agree with replay of the accepted prefix, including future kind choices.
    let before = (game.clone(), queue.clone(), state.clone());
    pending.declared_kind = Some(kind);
    state.pending_exile_face_down = Some(pending.clone());
    let result = (|| {
        if !game.declare_blind_face_down_cast(pending.card_id, pending.player, kind, pending.incarnation, &pending.permission) {
            return Err(ResponseError::IllegalChoice("The declared face-down cast is unavailable".into()).into());
        }
        let action = crate::decision::declared_blind_face_down_action(game, pending.player, pending.card_id, &pending.permission)?
            .ok_or_else(|| ResponseError::IllegalChoice("The declared face-down cast is unavailable".into()))?;
        state.pending_exile_face_down = None;
        state.declared_exile_face_down = Some(pending);
        super::priority_apply::apply_admitted_priority_action(game, queue, state, &action, dm)
    })();
    if result.is_err() || dm.awaiting_choice() {
        game.restore_execution_checkpoint(before.0, result.is_ok() && dm.awaiting_choice());
        *queue = before.1; *state = before.2;
        if result.is_ok() && dm.awaiting_choice() {
            // A successful suspended response carries this declaration in its
            // captured continuation. Keep its kind until that response resumes.
            if let Some(pending) = state.pending_exile_face_down.as_mut() { pending.declared_kind = Some(kind); }
            return Ok(GameProgress::Continue);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use crate::game_state::{FaceDownCastKind, HiddenIdentityCheck};
    use crate::grant_registry::{GrantSource, PlayFromConstraints};
    use crate::{CardId, CardType};
    const A: PlayerId = PlayerId::from_index(0);
    const B: PlayerId = PlayerId::from_index(1);
    fn setup(tracked: bool, kind: Option<FaceDownCastKind>, mana: u32) -> (GameState, ObjectId) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = B; game.turn.priority_player = Some(B); game.turn.phase = crate::game_state::Phase::FirstMain; game.turn.step = None;
        game.player_mut(B).unwrap().mana_pool.red = mana;
        let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Public origin").card_types(vec![CardType::Enchantment]).build(), A, Zone::Battlefield);
        let card = if tracked { game.create_hidden_card_placeholder(A, Zone::Exile, 0, "blind-morph-commitment".into()) }
        else { game.create_object_from_definition(&definition(kind), A, Zone::Exile) };
        game.set_face_down(card);
        game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, B,
            PlayFromConstraints { cast_mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::AnyColor, ..Default::default() },
            GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
        (game, card)
    }
    fn definition(kind: Option<FaceDownCastKind>) -> crate::cards::CardDefinition {
        let builder = crate::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Private printed face")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(7, 7));
        let cost = crate::cost::TotalCost::mana(crate::ManaCost::new());
        let ability = match kind {
            Some(FaceDownCastKind::Morph) => Some(crate::static_abilities::StaticAbility::morph(cost)),
            Some(FaceDownCastKind::Megamorph) => Some(crate::static_abilities::StaticAbility::megamorph(cost)),
            Some(FaceDownCastKind::Disguise) => Some(crate::static_abilities::StaticAbility::disguise(cost)),
            _ => None,
        };
        if let Some(ability) = ability { builder.with_ability(crate::ability::Ability::static_ability(ability)).build() } else { builder.build() }
    }
    fn intent(game: &GameState, card: ObjectId) -> LegalAction {
        compute_legal_actions(game, B).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::CastExiledCardFaceDown { card_id, .. } if *card_id == card)).unwrap()
    }
    fn start(game: &mut GameState, card: ObjectId, queue: &mut TriggerQueue, state: &mut PriorityLoopState) {
        let action = intent(game, card);
        let progress = apply_priority_response_with_dm(game, queue, state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
        assert!(matches!(progress, GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::SelectOptions(context)) if context.exile_face_down_choice && !context.exile_play_choice && context.options.len() == 4));
        assert!(game.is_face_down(card)); assert!(!game.can_player_look_at_face_down_exiled_card(card, B));
    }
    fn finish(game: &mut GameState, queue: &mut TriggerQueue, state: &mut PriorityLoopState, choice: usize) {
        let mut progress = apply_priority_response_with_dm(game, queue, state, &PriorityResponse::ExileFaceDownChoice(choice), &mut SelectFirstDecisionMaker).unwrap();
        for _ in 0..32 {
            if !state.has_pending_action() { return; }
            let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("face-down cast has an ordinary resumable payment decision"); };
            progress = apply_decision_context_with_dm(game, queue, state, &context, &mut SelectFirstDecisionMaker).unwrap();
        }
        panic!("face-down cast did not finish");
    }
    #[test]
    fn intent_and_public_declarations_are_uniform_before_private_rule_or_price_checks() {
        for kind in [None, Some(FaceDownCastKind::Morph), Some(FaceDownCastKind::Disguise)] {
            for mana in [0, 100] {
                let (mut game, card) = setup(false, kind, mana); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
                let actions = compute_legal_actions(&game, B).unwrap().into_iter().filter(|action| crate::decision::legal_action_source(action) == Some(card)).collect::<Vec<_>>();
                assert_eq!(actions.len(), 2); assert_eq!(crate::decision::format_action_short(&game, &intent(&game, card), None), "Cast exiled card face down");
                start(&mut game, card, &mut queue, &mut state);
                assert_eq!(state.pending_exile_face_down.as_ref().unwrap().kinds, vec![FaceDownCastKind::Morph, FaceDownCastKind::Megamorph, FaceDownCastKind::Disguise]);
                apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::ExileFaceDownChoice(3), &mut SelectFirstDecisionMaker).unwrap();
                assert!(!state.has_pending_action()); assert!(game.is_face_down(card)); assert!(game.hidden_face_down_cast_claim(card).is_none());
                assert_eq!(game.player(B).unwrap().mana_pool.red, mana); assert!(game.stack.is_empty());
            }
        }
    }
    #[test]
    fn tracked_claims_pay_without_opening_and_keep_the_authenticated_keyword_obligation() {
        for (choice, kind) in [FaceDownCastKind::Morph, FaceDownCastKind::Megamorph, FaceDownCastKind::Disguise].into_iter().enumerate() {
            let (mut game, card) = setup(true, None, 3); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
            let original = blind_play::incarnation(&game, card).unwrap(); start(&mut game, card, &mut queue, &mut state);
            assert_eq!(state.pending_exile_face_down.as_ref().unwrap().incarnation, original);
            finish(&mut game, &mut queue, &mut state, choice);
            let spell = game.stack[0].object_id; assert!(game.is_face_down(spell)); assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
            assert!(game.hidden_identity_obligations().iter().any(|obligation| obligation.check == HiddenIdentityCheck::CastFaceDown(kind)));
            assert!(game.hidden_identity_obligation_violation(spell, &definition(Some(kind))).is_none());
            assert!(game.hidden_identity_obligation_violation(spell, &definition(None)).is_some());
            let receipt = game.object(spell).unwrap().cast_play_permission.as_deref().unwrap(); assert_eq!(receipt.origin, card); assert_eq!(receipt.player, B);
            assert!(!game.can_player_look_at_face_down_exiled_card(card, B)); assert!(game.blind_face_down_declaration(card).is_none());
        }
    }
    #[test]
    fn failed_unaccepted_declaration_and_clean_prefix_admit_the_same_future_kind() {
        let (mut game, card) = setup(true, None, 0); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
        start(&mut game, card, &mut queue, &mut state);
        let clean_prefix = (game.clone(), queue.clone(), state.clone());
        let events = game.clone().take_pending_trigger_events().len(); let next = game.next_object_id_counter();
        assert!(apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::ExileFaceDownChoice(2), &mut SelectFirstDecisionMaker).is_err());
        assert_eq!(state.pending_exile_face_down.as_ref().unwrap().declared_kind, None);
        assert!(game.is_face_down(card)); assert!(!game.can_player_look_at_face_down_exiled_card(card, B)); assert!(game.hidden_face_down_cast_claim(card).is_none());
        assert!(game.stack.is_empty()); assert!(queue.entries.is_empty()); assert_eq!(game.clone().take_pending_trigger_events().len(), events); assert_eq!(game.next_object_id_counter(), next);
        let failed_savepoint = (game.clone(), queue.clone(), state.clone());
        let mut outcomes = Vec::new();
        for (mut resumed, mut queue, mut state) in [failed_savepoint, clean_prefix] {
            resumed.player_mut(B).unwrap().mana_pool.red = 3;
            // The unaccepted Disguise attempt must not prevent an otherwise
            // legal later Morph command on only one recovery route.
            finish(&mut resumed, &mut queue, &mut state, 0);
            let spell = resumed.stack[0].object_id;
            let disguise = resumed.object(spell).unwrap().face_down_cast_state.as_ref().unwrap().disguise_ward;
            assert!(resumed.hidden_identity_obligations().iter().any(|obligation| obligation.check == HiddenIdentityCheck::CastFaceDown(FaceDownCastKind::Morph)));
            assert!(!resumed.hidden_identity_obligations().iter().any(|obligation| obligation.check == HiddenIdentityCheck::CastFaceDown(FaceDownCastKind::Disguise)));
            outcomes.push((resumed.is_face_down(spell), disguise, resumed.player(B).unwrap().mana_pool.total(), state.has_pending_action()));
        }
        assert_eq!(outcomes, vec![(true, false, 0, false); 2]);
    }
    #[test]
    fn forged_kind_claim_or_incarnation_cannot_bypass_the_opaque_origin() {
        let (mut game, card) = setup(true, None, 3); let mut forged = intent(&game, card);
        if let LegalAction::CastExiledCardFaceDown { incarnation, .. } = &mut forged { *incarnation = Some(99); }
        assert!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut PriorityLoopState::new(2), &PriorityResponse::PriorityAction(forged), &mut SelectFirstDecisionMaker).is_err());
        game.set_hidden_face_down_cast_claim(card, FaceDownCastKind::Morph);
        assert!(!compute_legal_actions(&game, B).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == card)));
        let permission = blind_play::selections(&game, card, B).unwrap().remove(0);
        let method = crate::alternative_cast::CastingMethod::ExactPermission { origin: Box::new(crate::alternative_cast::CastingMethod::FaceDownPlayFrom {source: permission.source, zone: Zone::Exile}), permission };
        assert!(super::super::priority_mana::propose_spell_cast(&mut game, card, Zone::Exile, B, &method).is_err());
        assert!(game.is_face_down(card)); assert!(!game.can_player_look_at_face_down_exiled_card(card, B)); assert!(game.stack.is_empty());
    }
    #[test]
    fn local_native_cards_validate_only_the_declared_rule_and_do_not_gain_inspection() {
        for (choice, kind) in [FaceDownCastKind::Morph, FaceDownCastKind::Megamorph, FaceDownCastKind::Disguise].into_iter().enumerate() {
            let (mut game, card) = setup(false, Some(kind), 3); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
            start(&mut game, card, &mut queue, &mut state); finish(&mut game, &mut queue, &mut state, choice);
            assert!(game.is_face_down(game.stack[0].object_id)); assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        }
        let (mut game, card) = setup(false, None, 3); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
        start(&mut game, card, &mut queue, &mut state);
        assert!(apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::ExileFaceDownChoice(0), &mut SelectFirstDecisionMaker).is_err());
        assert!(game.is_face_down(card)); assert!(!game.can_player_look_at_face_down_exiled_card(card, B)); assert_eq!(game.player(B).unwrap().mana_pool.total(), 3); assert!(game.stack.is_empty());
    }
    #[test]
    fn effect_kind_keeps_both_the_actual_caster_permission_and_original_zone_grant() {
        let (mut game, card) = setup(true, None, 3); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
        let effect = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Public face-down rule").card_types(vec![CardType::Enchantment]).build(), B, Zone::Battlefield);
        game.grant_face_down_cast_permission(crate::game_state::FaceDownCastPermission {
            source: effect, player: B, zone: Zone::Exile, filter: crate::target::ObjectFilter::creature(),
            description: "Public creature face-down permission".into(), requires_source_on_battlefield: true,
            expires_after_turn: None, single_use: true,
        });
        let action = intent(&game, card);
        apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
        let receipt = state.pending_exile_face_down.as_ref().unwrap(); assert_eq!(receipt.kinds[3], FaceDownCastKind::Permission { source: effect });
        let original_grant = receipt.permission.identity.clone(); finish(&mut game, &mut queue, &mut state, 3);
        let spell = game.stack[0].object_id; assert!(game.is_face_down(spell));
        assert!(game.hidden_identity_obligations().iter().any(|obligation| obligation.check == HiddenIdentityCheck::Matches));
        assert!(game.active_face_down_cast_permission(effect, B, Zone::Exile).is_none());
        assert_eq!(game.object(spell).unwrap().cast_play_permission.as_deref().unwrap().identity, original_grant);
    }

    #[test]
    fn known_and_placeholder_claims_ignore_private_costs_and_targets_until_authenticated_opening() {
        let mut paid = Vec::new();
        for materialized in [false, true] {
            let (mut game, card) = setup(true, None, 3);
            let private = crate::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Private costly targeted spell")
                .card_types(vec![CardType::Sorcery])
                .additional_cost(crate::cost::TotalCost::mana(crate::ManaCost::new().add_generic(100)))
                .optional_cost(crate::cost::OptionalCost::custom("Private optional cost", crate::cost::TotalCost::mana(crate::ManaCost::new().add_generic(5))))
                .with_spell_effect(vec![Effect::destroy(crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(crate::target::ObjectFilter::creature())))])
                .build();
            if materialized { assert!(game.reveal_hidden_card_with_definition(card, &private).is_some()); }
            assert!(!game.can_player_look_at_face_down_exiled_card(card, B));
            let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
            start(&mut game, card, &mut queue, &mut state); finish(&mut game, &mut queue, &mut state, 0);
            let spell = game.stack[0].object_id; let object = game.object(spell).unwrap();
            assert!(object.optional_costs.is_empty()); assert!(object.additional_cost.is_free()); assert!(object.spell_effect.is_none());
            assert!(game.hidden_identity_obligation_violation(spell, &private).is_some(), "wrong public claim is still rejected on authenticated opening");
            paid.push(game.player(B).unwrap().mana_pool.total());
        }
        assert_eq!(paid, vec![0, 0]);
    }

    #[test]
    fn undeclared_exact_origins_cannot_read_hidden_faces_costs_or_payment_policy() {
        use crate::alternative_cast::{CastingMethod, play_permission};
        for materialized in [false, true] { for face_down in [false, true] {
            let (mut game, card) = setup(true, None, 100);
            if materialized { assert!(game.reveal_hidden_card_with_definition(card, &definition(Some(FaceDownCastKind::Morph))).is_some()); }
            game.set_hidden_face_down_cast_claim(card, FaceDownCastKind::Morph);
            let permission = blind_play::selections(&game, card, B).unwrap().remove(0);
            let origin = if face_down { CastingMethod::FaceDownPlayFrom { source: permission.source, zone: Zone::Exile } }
                else { CastingMethod::PlayFrom { source: permission.source, zone: Zone::Exile, use_alternative: None } };
            let method = CastingMethod::ExactPermission { origin: Box::new(origin), permission };
            let object = game.object(card).unwrap(); let pool = game.player(B).unwrap().mana_pool.clone();
            assert!(matches!(play_permission::receipt_for_method(&game, B, object, &method), Err(crate::effects::ExecutionError::IncompleteEvidence(_))));
            assert!(matches!(play_permission::selected_face(&game, B, object, &method), Err(crate::effects::ExecutionError::IncompleteEvidence(_))));
            assert!(!crate::decision::can_cast_spell(&game, B, object, &method));
            assert!(crate::decision::spell_mana_cost_for_cast(&game, B, object, &method, Zone::Exile).is_none());
            assert_eq!(play_permission::casting_spend_policy(&game, B, object, &method).mode, ironsmith_core::value_model::ManaSpendMode::Normal);
            assert_eq!(game.player(B).unwrap().mana_pool, pool); assert!(game.is_face_down(card)); assert!(game.stack.is_empty());
            assert!(!game.can_player_look_at_face_down_exiled_card(card, B));
        } }
    }

    #[test]
    fn admitted_declaration_and_captured_stack_receipt_keep_their_valid_cost_and_mana_policy() {
        use crate::alternative_cast::play_permission;
        let (mut game, card) = setup(true, None, 3); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
        start(&mut game, card, &mut queue, &mut state);
        let pending = state.pending_exile_face_down.clone().unwrap();
        assert!(game.declare_blind_face_down_cast(card, B, FaceDownCastKind::Morph, pending.incarnation, &pending.permission));
        let LegalAction::CastSpell { casting_method: method, .. } = crate::decision::declared_blind_face_down_action(&game, B, card, &pending.permission).unwrap().unwrap() else { panic!("declared face-down method"); };
        let object = game.object(card).unwrap();
        assert!(crate::decision::can_cast_spell(&game, B, object, &method));
        assert!(play_permission::receipt_for_method(&game, B, object, &method).unwrap().is_some());
        assert_eq!(play_permission::casting_spend_policy(&game, B, object, &method).mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
        assert_eq!(crate::decision::spell_mana_cost_for_cast(&game, B, object, &method, Zone::Exile).unwrap().mana_value(), 3);
        let stack = super::super::priority_mana::propose_spell_cast(&mut game, card, Zone::Exile, B, &method).unwrap();
        assert!(game.blind_face_down_declaration(card).is_none());
        let object = game.object(stack).unwrap();
        assert!(play_permission::receipt_for_method(&game, B, object, &method).unwrap().is_some());
        assert_eq!(play_permission::casting_spend_policy(&game, B, object, &method).mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
    }

    #[test]
    fn cancelling_after_an_accepted_manual_mana_activation_restores_the_exact_root_queue() {
        use crate::mana_payment::ManaPaymentResponse;
        use crate::mana::ManaSymbol;
        let (mut game, card) = setup(true, None, 2); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
        let land = crate::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Manual mana land")
            .card_types(vec![CardType::Land]).with_ability(crate::ability::Ability::mana(crate::cost::TotalCost::free(), vec![ManaSymbol::Red])).build();
        let mana_source = game.create_object_from_definition(&land, B, Zone::Battlefield);
        let preceding_source = game.create_object_from_definition(&land, B, Zone::Battlefield); game.tap(preceding_source);
        let observer = crate::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Tap-for-mana observer")
            .card_types(vec![CardType::Enchantment]).with_ability(crate::ability::Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(crate::target::PlayerFilter::You, crate::target::ObjectFilter::land()),
                vec![Effect::lose_life(1)])).build();
        let observer = game.create_object_from_definition(&observer, B, Zone::Battlefield);
        // Preserve a genuine queued trigger predating this attempt. The new
        // trigger below must be produced by the accepted native activation.
        let event = crate::events::ManaAddedEvent::new(preceding_source, B, B, vec![ManaSymbol::Red])
            .with_snapshot(Some(crate::snapshot::ObjectSnapshot::from_object(game.object(preceding_source).unwrap(), &game)))
            .with_production_provenance(crate::events::mana::ManaProductionProvenance::TappedSourceForMana).into_trigger_event();
        super::super::queue_triggers_from_event(&mut game, &mut queue, event, false);
        assert_eq!(queue.entries.len(), 1); let original_identity = queue.entries[0].trigger_identity;
        start(&mut game, card, &mut queue, &mut state);
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::ExileFaceDownChoice(0), &mut SelectFirstDecisionMaker).unwrap();
        for _ in 0..16 {
            if state.pending_cast.as_ref().is_some_and(|pending| pending.pending_mana_payment.is_some()) { break; }
            let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("ordinary mana payment must be pending"); };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
        }
        assert!(state.pending_cast.as_ref().is_some_and(|pending| pending.pending_mana_payment.is_some()));
        apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Activate { source: mana_source, ability_index: 0 }), &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.is_tapped(mana_source)); assert_eq!(game.player(B).unwrap().mana_pool.red, 3);
        assert_eq!(queue.entries.len(), 2); assert!(queue.entries.iter().any(|entry| entry.triggering_event.object_id() == Some(mana_source)));
        let saved = (game.clone(), queue.clone(), state.clone()); game = saved.0; queue = saved.1; state = saved.2;
        let progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Cancel), &mut SelectFirstDecisionMaker).unwrap();
        assert!(matches!(progress, GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::SelectOptions(context)) if context.exile_face_down_choice));
        assert!(!game.is_tapped(mana_source)); assert!(game.is_tapped(preceding_source)); assert_eq!(game.player(B).unwrap().mana_pool.red, 2);
        assert!(game.stack.is_empty()); assert_eq!(queue.entries.len(), 1); assert_eq!(queue.entries[0].source, observer);
        assert_eq!(queue.entries[0].trigger_identity, original_identity); assert_eq!(queue.entries[0].triggering_event.object_id(), Some(preceding_source));
        assert_eq!(state.pending_exile_face_down.as_ref().unwrap().declared_kind, Some(FaceDownCastKind::Morph));
        apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::ExileFaceDownChoice(3), &mut SelectFirstDecisionMaker).unwrap();
        assert!(!state.has_pending_action()); assert!(!state.has_opened_exile_play_receipt());
        assert_eq!(game.stack.len(), 1, "only the pre-existing trigger may advance"); assert!(game.is_face_down(card));
        super::super::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(B).unwrap().life, 19); assert!(game.stack.is_empty()); assert_eq!(game.player(B).unwrap().mana_pool.red, 2);
    }

}
