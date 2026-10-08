//! Authored source witnesses for exact, durable, zero-price exile permissions.
//! All execution remains UNRUN during the source-only repair.
use super::*;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::LinkedFaceLayout;
use crate::cards::CardDefinitionBuilder;
use crate::cost::TotalCost;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::{EffectExecutor, GrantPlayTaggedDuration, GrantPlayTaggedEffect};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = PlayerId::from_index(0);
    game.turn.priority_player = Some(PlayerId::from_index(0));
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}

fn grant_price(game: &mut GameState, card: ObjectId, allow_land: bool) -> ObjectId {
    let player = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::new(), "Durable price source")
        .card_types(vec![CardType::Artifact]).build();
    let source = game.create_object_from_definition(&source, player, Zone::Battlefield);
    let snapshot = ObjectSnapshot::from_object(game.object(card).unwrap(), game);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, player, &mut dm)
        .with_tagged_objects(std::collections::HashMap::from([
            (crate::tag::TagKey::from("exact_price_card"), vec![snapshot]),
        ]));
    GrantPlayTaggedEffect::new("exact_price_card", crate::target::PlayerFilter::You,
        GrantPlayTaggedDuration::ForAsLongAsExiled, allow_land, false)
        .with_alternative_cost(TotalCost::from_costs(Vec::new()))
        .execute(game, &mut ctx).unwrap();
    source
}

fn cast_actions(game: &GameState, player: PlayerId, card: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
        .collect()
}

fn x_minimum(minimum: crate::effect::Value) -> crate::ability::Ability {
    crate::ability::Ability::static_ability(
        crate::static_abilities::StaticAbility::this_spell_x_minimum(minimum, "Minimum X"),
    ).in_zones(vec![Zone::Stack])
}

fn finish_cast(game: &mut GameState, action: LegalAction, chosen_x: u32) -> ObjectId {
    use crate::decisions::context::DecisionContext;
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).expect("priced cast starts");
    for _ in 0..32 {
        if let Some(entry) = game.stack.last() {
            assert!(state.pending_cast.is_none());
            return entry.object_id;
        }
        let response = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ctx)) =>
                PriorityResponse::ManaPaymentPlan(crate::mana_payment::ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash,
                }),
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ctx)) => {
                assert!(ctx.description.to_ascii_lowercase().starts_with("choose the next cost to pay"));
                PriorityResponse::NextCostChoice(ctx.options.iter().find(|option| option.legal)
                    .expect("mandatory cost is payable").index)
            }
            GameProgress::NeedsDecisionCtx(DecisionContext::Number(ctx)) => {
                assert!(ctx.is_x_value);
                assert!(chosen_x >= ctx.min && chosen_x <= ctx.max);
                PriorityResponse::XValue(chosen_x)
            }
            other => panic!("unexpected priced-cast continuation: {other:?}"),
        };
        progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
            &response, &mut dm).expect("mandatory cost payment succeeds");
    }
    panic!("priced cast did not finish");
}

#[test]
fn free_exile_price_keeps_mandatory_mana_and_life_costs_through_actual_payment() {
    let mut game = game();
    let player = PlayerId::from_index(0);
    let owner = PlayerId::from_index(1);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Expensive stolen spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::new().add_generic(7))
        .alternative_cast(AlternativeCastingMethod::alternative_cost("Printed alternate price",
            Some(ManaCost::new()), vec![]))
        .additional_cost(TotalCost::from_costs(vec![
            crate::costs::Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Red])),
            crate::costs::Cost::life(2),
        ])).build();
    let card = game.create_object_from_definition(&definition, owner, Zone::Exile);
    let source = grant_price(&mut game, card, false);
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    game.turn.turn_number += 1;
    assert!(cast_actions(&game, player, card).is_empty(), "mandatory red mana is still required");
    let method = CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: Some(1) };
    let mut rejected = game.clone();
    assert!(apply_priority_response_with_dm(&mut rejected, &mut TriggerQueue::new(),
        &mut PriorityLoopState::new(2), &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: card, from_zone: Zone::Exile, casting_method: method.clone(),
        }), &mut SelectFirstDecisionMaker).is_err(), "unpayable cast cannot bypass its absent menu action");
    assert_eq!(rejected.object(card).unwrap().zone, Zone::Exile);
    game.player_mut(player).unwrap().mana_pool.red = 1;
    let actions = cast_actions(&game, player, card);
    assert_eq!(actions, vec![LegalAction::CastSpell { spell_id: card, from_zone: Zone::Exile,
        casting_method: method.clone() }], "only the selected free price is granted");
    assert!(cast_actions(&game, owner, card).is_empty(), "ownership does not transfer permission");
    let mut no_life = game.clone();
    no_life.player_mut(player).unwrap().life = 1;
    assert!(cast_actions(&no_life, player, card).is_empty());
    for forged_method in [CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: None },
        CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: Some(0) }] {
        let mut rejected = game.clone();
        assert!(apply_priority_response_with_dm(&mut rejected, &mut TriggerQueue::new(),
            &mut PriorityLoopState::new(2), &PriorityResponse::PriorityAction(LegalAction::CastSpell {
                spell_id: card, from_zone: Zone::Exile, casting_method: forged_method,
            }), &mut SelectFirstDecisionMaker).is_err(), "no ordinary or competing alternative route");
        assert_eq!(rejected.object(card).unwrap().zone, Zone::Exile);
    }
    let stack = finish_cast(&mut game, actions[0].clone(), 0);
    assert_eq!(game.player(player).unwrap().mana_pool.red, 0);
    assert_eq!(game.player(player).unwrap().life, 18);
    assert_eq!(game.object(stack).unwrap().owner, owner);
    assert_eq!(game.object(stack).unwrap().controller, player);
    assert!(matches!(game.object(stack).unwrap().cast_alternative_method.as_deref(),
        Some(AlternativeCastingMethod::FromZone { total_cost, .. }) if total_cost.costs().is_empty()));
}

#[test]
fn free_exile_price_offers_and_casts_each_eligible_other_spell_face() {
    let player = PlayerId::from_index(0);
    let owner = PlayerId::from_index(1);
    for (layout, adventure) in [(LinkedFaceLayout::None, true), (LinkedFaceLayout::Split, false),
        (LinkedFaceLayout::TransformLike, false)] {
        let mut game = game();
        let front_id = CardId::new();
        let back_id = CardId::new();
        let front = CardDefinitionBuilder::new(front_id, "Priced front")
            .card_types(vec![CardType::Creature]).mana_cost(ManaCost::new().add_generic(7))
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .other_face(back_id).other_face_name("Priced other spell")
            .linked_face_layout(layout).build();
        let back = CardDefinitionBuilder::new(back_id, "Priced other spell")
            .card_types(vec![CardType::Instant]).mana_cost(ManaCost::new().add_generic(5))
            .subtypes(if adventure { vec![Subtype::Adventure] } else { vec![] })
            .additional_cost(TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Red])))
            .other_face(front_id).other_face_name("Priced front")
            .linked_face_layout(layout).build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let card = game.create_object_from_definition(&front, owner, Zone::Exile);
        let source = grant_price(&mut game, card, false);
        if layout == LinkedFaceLayout::Split {
            // Existing producers can pair the price with an ordinary reader.
            // The dedicated free-face owner must not duplicate or bypass it.
            game.effect_store.grant_registry.grant_to_card(card, Zone::Exile, player,
                crate::grant::Grantable::PlayFrom, crate::grant_registry::GrantSource::Effect {
                    source_id: source, expires_end_of_turn: u32::MAX,
                });
        }
        let other = |action: &&LegalAction| matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::SplitOtherHalfPlayFrom { .. }, .. });
        assert!(!cast_actions(&game, player, card).iter().any(|action| other(&action)),
            "chosen other face's mandatory mana is checked");
        game.player_mut(player).unwrap().mana_pool.red = 1;
        let actions = cast_actions(&game, player, card);
        let action = actions.iter().find(other).expect("other face has its own priced route").clone();
        assert!(actions.iter().all(|action| matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. }
                | CastingMethod::SplitOtherHalfPlayFrom { use_alternative: Some(_), .. }, .. })));
        game.turn.active_player = owner;
        game.turn.phase = crate::game_state::Phase::Combat;
        let off_turn = cast_actions(&game, player, card);
        assert_eq!(off_turn, vec![action.clone()], "only the instant face has ordinary off-turn timing");
        let stack = finish_cast(&mut game, action, 0);
        assert_eq!(game.object(stack).unwrap().name.as_str(), "Priced other spell");
        assert_eq!(game.player(player).unwrap().mana_pool.red, 0);
        assert_eq!(game.object(stack).unwrap().owner, owner);
        if adventure {
            let stable = game.object(stack).unwrap().stable_id;
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            let returned = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(returned).unwrap().zone, Zone::Exile);
            assert_ne!(returned, card);
            assert_eq!(game.adventure_exiled_player(returned), Some(player));
            game.turn.active_player = player;
            game.turn.priority_player = Some(player);
            game.turn.phase = crate::game_state::Phase::FirstMain;
            assert!(cast_actions(&game, player, returned).is_empty(),
                "new Adventure permission must not recover the departed free price");
            game.player_mut(player).unwrap().mana_pool.colorless = 7;
            let actions = cast_actions(&game, player, returned);
            assert_eq!(actions, vec![LegalAction::CastSpell { spell_id: returned,
                from_zone: Zone::Exile, casting_method: CastingMethod::Normal }],
                "the independent Adventure permission still permits the ordinary front price");
            let front_stack = finish_cast(&mut game, actions[0].clone(), 0);
            assert_eq!(game.object(front_stack).unwrap().name.as_str(), "Priced front");
            assert_eq!(game.player(player).unwrap().mana_pool.colorless, 0);
        }
    }
}

#[test]
fn free_exile_play_price_has_land_face_permission_only_when_authored() {
    let player = PlayerId::from_index(0);
    for allow_land in [false, true] {
        let mut game = game();
        let front_id = CardId::new();
        let back_id = CardId::new();
        let front = CardDefinitionBuilder::new(front_id, "Modal priced spell")
            .card_types(vec![CardType::Sorcery]).mana_cost(ManaCost::new().add_generic(5))
            .other_face(back_id).other_face_name("Modal priced land")
            .linked_face_layout(LinkedFaceLayout::TransformLike).build();
        let back = CardDefinitionBuilder::new(back_id, "Modal priced land")
            .card_types(vec![CardType::Land]).other_face(front_id)
            .other_face_name("Modal priced spell").linked_face_layout(LinkedFaceLayout::TransformLike).build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let card = game.create_object_from_definition(&front, PlayerId::from_index(1), Zone::Exile);
        grant_price(&mut game, card, allow_land);
        let land = LegalAction::PlayLand { land_id: card };
        assert_eq!(compute_legal_actions(&game, player).unwrap().contains(&land), allow_land);
        let mut off_turn = game.clone();
        off_turn.turn.active_player = PlayerId::from_index(1);
        assert!(!compute_legal_actions(&off_turn, player).unwrap().contains(&land));
        if allow_land {
            apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(),
                &mut PriorityLoopState::new(2), &PriorityResponse::PriorityAction(land),
                &mut SelectFirstDecisionMaker).unwrap();
            assert!(game.battlefield.iter().any(|id| game.object(*id).is_some_and(|object|
                object.name.as_str() == "Modal priced land" && object.controller == player)));
        }
    }
}

#[test]
fn free_exile_printed_x_is_zero_while_additional_only_x_is_chosen_and_paid() {
    let player = PlayerId::from_index(0);
    for printed_x in [true, false] {
        let mut game = game();
        let mana = if printed_x { ManaCost::from_symbols(vec![ManaSymbol::X, ManaSymbol::Blue]) }
            else { ManaCost::new().add_generic(6) };
        let definition = CardDefinitionBuilder::new(CardId::new(), "Priced X spell")
            .card_types(vec![CardType::Instant]).mana_cost(mana)
            .with_ability(x_minimum(crate::effect::Value::Fixed(if printed_x { 0 } else { 1 })))
            .additional_cost(TotalCost::from_cost(crate::costs::Cost::effect(
                crate::effects::PayLifeEffect::you(crate::effect::Value::X))))
            .build();
        let card = game.create_object_from_definition(&definition, PlayerId::from_index(1), Zone::Exile);
        grant_price(&mut game, card, false);
        let action = cast_actions(&game, player, card).into_iter().next().expect("free priced X cast");
        let stack = finish_cast(&mut game, action, if printed_x { 0 } else { 3 });
        assert_eq!(game.object(stack).unwrap().x_value, Some(if printed_x { 0 } else { 3 }));
        assert_eq!(game.player(player).unwrap().life, if printed_x { 20 } else { 17 });
    }
}

#[test]
fn free_exile_printed_x_positive_minimum_rejects_menu_and_forgery_with_full_rollback() {
    let player = PlayerId::from_index(0);
    let owner = PlayerId::from_index(1);
    for direct_proposal in [false, true] {
        let mut game = game();
        let definition = CardDefinitionBuilder::new(CardId::new(), "Minimum one X")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X, ManaSymbol::Blue]))
            .with_ability(x_minimum(crate::effect::Value::Fixed(1)))
            .additional_cost(TotalCost::from_costs(vec![
                crate::costs::Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Red])),
                crate::costs::Cost::life(2),
            ])).build();
        let card = game.create_object_from_definition(&definition, owner, Zone::Exile);
        let source = grant_price(&mut game, card, false);
        let budget = game.effect_store.grant_registry.create_shared_usage_budget(1);
        game.effect_store.grant_registry.grants.last_mut().unwrap().shared_usage_id = Some(budget);
        game.player_mut(player).unwrap().mana_pool.red = 1;
        game.player_mut(player).unwrap().mana_pool.colorless = 9;
        game.take_pending_trigger_events();
        let original = format!("{:?}", game.object(card).unwrap());
        let grants = format!("{:?}", game.effect_store.grant_registry.grants);
        let next_id = game.next_object_id_counter();
        let exile = game.exile.clone();
        let casts = game.turn_store.turn_history.total_spells_cast_this_turn();
        let method = CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: Some(0) };
        assert!(cast_actions(&game, player, card).is_empty(),
            "payable mandatory costs cannot make forced zero satisfy minimum one");
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let result = if direct_proposal {
            // Exercise the authoritative X phase even if a caller bypasses
            // all menu checks. Proposal really retires the exile identity and
            // spends the shared grant before this owner must undo it.
            state.save_checkpoint(&game);
            let stack = super::super::priority_mana::propose_spell_cast(
                &mut game, card, Zone::Exile, player, &method,
            ).unwrap();
            assert_ne!(stack, card);
            assert!(game.object(card).is_none());
            let mut consumed = game.clone();
            assert!(!consumed.effect_store.grant_registry.consume_shared_usage(budget));
            let pending = PendingCast::new(stack, Zone::Exile, player, ProvNodeId::default(),
                CastStage::ChoosingX, None, vec![], method.clone(),
                game.object(stack).unwrap().optional_costs_paid.clone(), None, stack);
            state.pending_cast = Some(pending);
            let pending = state.pending_cast.take().unwrap();
            check_x_or_continue(&mut game, &mut queue, &mut state, pending, &mut SelectFirstDecisionMaker)
        } else {
            apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(LegalAction::CastSpell {
                    spell_id: card, from_zone: Zone::Exile, casting_method: method.clone(),
                }), &mut SelectFirstDecisionMaker)
        };
        assert!(matches!(result, Err(GameLoopError::ActionCancelled(_))), "{result:?}");
        assert_eq!(format!("{:?}", game.object(card).unwrap()), original);
        assert_eq!(game.exile, exile);
        assert_eq!(game.next_object_id_counter(), next_id);
        assert_eq!(game.player(player).unwrap().mana_pool.red, 1);
        assert_eq!(game.player(player).unwrap().mana_pool.colorless, 9);
        assert_eq!(game.player(player).unwrap().life, 20);
        assert_eq!(game.player(owner).unwrap().life, 20);
        assert_eq!(format!("{:?}", game.effect_store.grant_registry.grants), grants);
        let mut restored = game.clone();
        assert!(restored.effect_store.grant_registry.consume_shared_usage(budget),
            "the rejected cast must not spend its permission");
        assert!(game.stack.is_empty());
        assert_eq!(game.turn_store.turn_history.total_spells_cast_this_turn(), casts);
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(queue.entries.is_empty());
        assert!(!state.has_pending_action());
        assert!(state.checkpoint.is_none());
    }
}

#[test]
fn free_exile_x_minimum_uses_only_the_selected_face_and_its_proposed_characteristics() {
    let player = PlayerId::from_index(0);
    let owner = PlayerId::from_index(1);
    for restrict_other in [false, true] {
        let mut game = game();
        let front_id = CardId::new();
        let other_id = CardId::new();
        let minimum = crate::effect::Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Source));
        let front = CardDefinitionBuilder::new(front_id, "Minimum front")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(if restrict_other { vec![ManaSymbol::X] }
                else { vec![ManaSymbol::X, ManaSymbol::Blue] }))
            .with_ability(x_minimum(minimum.clone()))
            .other_face(other_id).other_face_name("Minimum other")
            .linked_face_layout(LinkedFaceLayout::TransformLike).build();
        let other = CardDefinitionBuilder::new(other_id, "Minimum other")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(if restrict_other { vec![ManaSymbol::X, ManaSymbol::Blue] }
                else { vec![ManaSymbol::X] }))
            .with_ability(x_minimum(minimum))
            .other_face(front_id).other_face_name("Minimum front")
            .linked_face_layout(LinkedFaceLayout::TransformLike).build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&other);
        let card = game.create_object_from_definition(&front, owner, Zone::Exile);
        let source = grant_price(&mut game, card, false);
        let actions = cast_actions(&game, player, card);
        assert_eq!(actions.len(), 1, "only the face with zero printed mana value allows X=0");
        assert_eq!(matches!(&actions[0], LegalAction::CastSpell {
            casting_method: CastingMethod::SplitOtherHalfPlayFrom { .. }, .. }), !restrict_other);
        let forbidden = if restrict_other {
            CastingMethod::SplitOtherHalfPlayFrom { source, zone: Zone::Exile, use_alternative: Some(1) }
        } else {
            CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: Some(0) }
        };
        let original = format!("{:?}", game.object(card).unwrap());
        let next_id = game.next_object_id_counter();
        let mut rejected = game.clone();
        let mut state = PriorityLoopState::new(2);
        assert!(apply_priority_response_with_dm(&mut rejected, &mut TriggerQueue::new(),
            &mut state, &PriorityResponse::PriorityAction(LegalAction::CastSpell {
                spell_id: card, from_zone: Zone::Exile, casting_method: forbidden,
            }), &mut SelectFirstDecisionMaker).is_err());
        assert_eq!(format!("{:?}", rejected.object(card).unwrap()), original);
        assert_eq!(rejected.next_object_id_counter(), next_id);
        assert!(!state.has_pending_action());
        assert!(state.checkpoint.is_none());
        let stack = finish_cast(&mut game, actions[0].clone(), 0);
        assert_eq!(game.object(stack).unwrap().name.as_str(),
            if restrict_other { "Minimum front" } else { "Minimum other" });
        assert_eq!(game.object(stack).unwrap().x_value, Some(0));
        assert_eq!(game.object(stack).unwrap().controller, player);
    }
}

#[test]
fn free_exile_printed_x_stale_action_rechecks_the_current_minimum() {
    let mut game = game();
    let player = PlayerId::from_index(0);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Stale free X")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X]))
        .with_ability(x_minimum(crate::effect::Value::Fixed(0))).build();
    let card = game.create_object_from_definition(&definition, player, Zone::Exile);
    grant_price(&mut game, card, false);
    let action = cast_actions(&game, player, card).into_iter().next().unwrap();
    game.object_mut(card).unwrap().abilities_mut()[0] = x_minimum(crate::effect::Value::Fixed(1));
    assert!(cast_actions(&game, player, card).is_empty());
    let original = format!("{:?}", game.object(card).unwrap());
    let next_id = game.next_object_id_counter();
    let mut state = PriorityLoopState::new(2);
    assert!(matches!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(),
        &mut state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker),
        Err(GameLoopError::ActionCancelled(_))));
    assert_eq!(format!("{:?}", game.object(card).unwrap()), original);
    assert_eq!(game.next_object_id_counter(), next_id);
    assert!(game.stack.is_empty());
    assert!(!state.has_pending_action());
    assert!(state.checkpoint.is_none());
}

#[test]
fn effect_waived_printed_x_obeys_the_same_minimum_and_rolls_back() {
    let player = PlayerId::from_index(0);
    for minimum in [0, 1] {
        let mut game = game();
        let definition = CardDefinitionBuilder::new(CardId::new(), "Effect-waived X")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X, ManaSymbol::Blue]))
            .with_ability(x_minimum(crate::effect::Value::Fixed(minimum)))
            .additional_cost(TotalCost::from_cost(crate::costs::Cost::life(2))).build();
        let card = game.create_object_from_definition(&definition, player, Zone::Exile);
        let original = format!("{:?}", game.object(card).unwrap());
        let next_id = game.next_object_id_counter();
        let result = cast_spell_from_resolving_effect(&mut game, card, Zone::Exile, player,
            &CastingMethod::Normal, true, None, ProvNodeId::default(), &mut SelectFirstDecisionMaker)
            .unwrap();
        if minimum > 0 {
            assert!(result.is_none());
            assert_eq!(format!("{:?}", game.object(card).unwrap()), original);
            assert_eq!(game.next_object_id_counter(), next_id);
            assert!(game.stack.is_empty());
            assert_eq!(game.player(player).unwrap().life, 20);
        } else {
            let stack = result.expect("minimum zero permits the effect's free cast");
            assert_eq!(game.object(stack).unwrap().x_value, Some(0));
            assert_eq!(game.player(player).unwrap().life, 18);
        }
    }
}

#[test]
fn nonzero_and_x_from_zone_prices_keep_their_announced_cost_and_payment() {
    let player = PlayerId::from_index(0);
    for variable_price in [false, true] {
        let mut game = game();
        let definition = CardDefinitionBuilder::new(CardId::new(), "Paid exile control")
            .card_types(vec![CardType::Instant])
            .with_ability(x_minimum(crate::effect::Value::Fixed(1)))
            .mana_cost(if variable_price { ManaCost::from_symbols(vec![ManaSymbol::X, ManaSymbol::Blue]) }
                else { ManaCost::new().add_generic(7) }).build();
        let card = game.create_object_from_definition(&definition, player, Zone::Exile);
        grant_price(&mut game, card, false);
        let cost = if variable_price { ManaCost::from_symbols(vec![ManaSymbol::X, ManaSymbol::Blue]) }
            else { ManaCost::new().add_generic(2) };
        game.effect_store.grant_registry.grants.last_mut().unwrap().grantable =
            crate::grant::Grantable::AlternativeCast(AlternativeCastingMethod::cast_from_zone_with_total_cost(
                "Paid control", Zone::Exile, TotalCost::mana(cost), None, false));
        assert!(cast_actions(&game, player, card).is_empty());
        game.player_mut(player).unwrap().mana_pool.colorless = if variable_price { 3 } else { 2 };
        game.player_mut(player).unwrap().mana_pool.blue = u32::from(variable_price);
        let action = cast_actions(&game, player, card).into_iter().next().expect("paid control price");
        let stack = finish_cast(&mut game, action, if variable_price { 3 } else { 0 });
        assert_eq!(game.object(stack).unwrap().x_value, variable_price.then_some(3));
        assert_eq!(game.player(player).unwrap().mana_pool.colorless, 0);
        assert_eq!(game.player(player).unwrap().mana_pool.blue, 0);
    }
}

#[test]
fn absent_printed_mana_cost_is_not_ordinary_free_casting() {
    let mut game = game();
    let player = PlayerId::from_index(0);
    let definition = CardDefinitionBuilder::new(CardId::new(), "No printed mana cost control")
        .card_types(vec![CardType::Instant]).build();
    let hand = game.create_object_from_definition(&definition, player, Zone::Hand);
    assert!(cast_actions(&game, player, hand).is_empty(), "no printed mana cost is unpayable");
    let exile = game.move_object_by_effect(hand, Zone::Exile).unwrap();
    grant_price(&mut game, exile, false);
    let action = cast_actions(&game, player, exile).into_iter().next().expect("explicit free alternative");
    let stack = finish_cast(&mut game, action, 0);
    assert!(game.object(stack).unwrap().mana_cost.is_none());
    assert_eq!(game.player(player).unwrap().life, 20);
}

#[test]
fn paid_other_face_alternative_with_ordinary_reader_remains_payable() {
    let mut game = game();
    let player = PlayerId::from_index(0);
    let front_id = CardId::new();
    let back_id = CardId::new();
    let front = CardDefinitionBuilder::new(front_id, "Paid modal front")
        .card_types(vec![CardType::Sorcery]).mana_cost(ManaCost::new().add_generic(6))
        .other_face(back_id).other_face_name("Paid modal back")
        .linked_face_layout(LinkedFaceLayout::TransformLike).build();
    let back = CardDefinitionBuilder::new(back_id, "Paid modal back")
        .card_types(vec![CardType::Instant]).mana_cost(ManaCost::new().add_generic(5))
        .other_face(front_id).other_face_name("Paid modal front")
        .linked_face_layout(LinkedFaceLayout::TransformLike).build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let card = game.create_object_from_definition(&front, player, Zone::Exile);
    let source = grant_price(&mut game, card, false);
    game.effect_store.grant_registry.grants.last_mut().unwrap().grantable =
        crate::grant::Grantable::AlternativeCast(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Paid other-face control", Zone::Exile, TotalCost::mana(ManaCost::new().add_generic(2)), None, false));
    game.effect_store.grant_registry.grant_to_card(card, Zone::Exile, player,
        crate::grant::Grantable::PlayFrom, crate::grant_registry::GrantSource::Effect {
            source_id: source, expires_end_of_turn: u32::MAX,
        });
    assert!(cast_actions(&game, player, card).is_empty());
    game.player_mut(player).unwrap().mana_pool.colorless = 2;
    let actions: Vec<_> = cast_actions(&game, player, card).into_iter().filter(|action|
        matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::SplitOtherHalfPlayFrom { use_alternative: Some(_), .. }, .. }))
        .collect();
    assert_eq!(actions.len(), 1);
    let stack = finish_cast(&mut game, actions[0].clone(), 0);
    assert_eq!(game.object(stack).unwrap().name.as_str(), "Paid modal back");
    assert_eq!(game.player(player).unwrap().mana_pool.colorless, 0);
}
