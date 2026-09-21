use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::ids::CardId;
use ironsmith::object::AttachmentTarget;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn setup() -> (GameState, ObjectId, ObjectId, TriggerQueue) {
    setup_with_protection(false)
}

fn setup_with_protection(protected: bool) -> (GameState, ObjectId, ObjectId, TriggerQueue) {
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Animate Dead",
    )
    .unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    assert_eq!(
        definition.abilities.len(),
        2,
        "ETB trigger and attached power effect; sacrifice must be delayed"
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut creature = CardDefinitionBuilder::new(CardId::new(), "Reanimated fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 3));
    if protected {
        creature = creature.with_ability(ironsmith::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::protection(
                ironsmith::ability::ProtectionFrom::Color(ironsmith::color::ColorSet::BLACK),
            ),
        ));
    }
    let creature = creature.build();
    let buried = game.create_object_from_definition(&creature, bob, Zone::Graveyard);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Black, 2);
    let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let aura_stable = game.object(hand).unwrap().stable_id;
    let action = ironsmith::decision::compute_legal_actions(&game, alice).into_iter().find(|action|
        matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)
    ).expect("Aura can target opponent's creature card in graveyard");
    let mut queue = TriggerQueue::new();
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..12 {
        if !game.stack.is_empty() {
            break;
        }
        let ironsmith::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let aura = game.find_object_by_stable_id(aura_stable).unwrap();
    assert_eq!(game.object(aura).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.object(aura).unwrap().attached_to,
        Some(AttachmentTarget::Object(buried))
    );
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "one enter trigger");
    (game, aura, buried, queue)
}

#[test]
fn removing_aura_before_enter_trigger_resolves_does_not_reanimate() {
    let (mut game, aura, buried, mut queue) = setup();
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.object(buried).unwrap().zone, Zone::Graveyard);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert!(
        game.stack.is_empty(),
        "unresolved ETB did not register a sacrifice trigger"
    );
}

#[test]
fn returns_attaches_and_sacrifices_under_changed_controller() {
    let (mut game, aura, buried, mut queue) = setup();
    let stable = game.object(buried).unwrap().stable_id;
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let returned = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.controller_of_id(returned),
        Some(PlayerId::from_index(0))
    );
    assert_eq!(
        game.object(aura).unwrap().attached_to,
        Some(AttachmentTarget::Object(returned))
    );
    assert_eq!(game.calculated_power(returned), Some(2));
    assert!(game.was_put_onto_battlefield_with_source(aura, returned));
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert_eq!(game.object(aura).unwrap().zone, Zone::Battlefield);
    game.set_current_controller(returned, PlayerId::from_index(1));
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "exactly one delayed sacrifice trigger");
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.object(game.find_object_by_stable_id(stable).unwrap())
            .unwrap()
            .zone,
        Zone::Graveyard
    );
}

#[test]
fn delayed_sacrifice_does_not_follow_creature_through_another_zone_change() {
    let (mut game, aura, buried, mut queue) = setup();
    let stable = game.object(buried).unwrap().stable_id;
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let returned = game.find_object_by_stable_id(stable).unwrap();
    let exiled = game.move_object_by_effect(returned, Zone::Exile).unwrap();
    let new_creature = game
        .move_object_by_effect(exiled, Zone::Battlefield)
        .unwrap();
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.object(new_creature).unwrap().zone, Zone::Battlefield);
}

#[test]
fn moving_enchanted_card_before_enter_trigger_resolves_does_not_return_its_new_object() {
    let (mut game, aura, buried, mut queue) = setup();
    let exiled = game.move_object_by_effect(buried, Zone::Exile).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    let stable = game.object(aura).unwrap().stable_id;
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.object(game.find_object_by_stable_id(stable).unwrap())
            .unwrap()
            .zone,
        Zone::Graveyard
    );
}

#[test]
fn protection_prevents_reattachment_but_still_registers_delayed_sacrifice() {
    let (mut game, aura, buried, mut queue) = setup_with_protection(true);
    let creature_stable = game.object(buried).unwrap().stable_id;
    let aura_stable = game.object(aura).unwrap().stable_id;
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let returned = game.find_object_by_stable_id(creature_stable).unwrap();
    assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
    assert!(game.object(returned).unwrap().attachments.is_empty());
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.object(game.find_object_by_stable_id(aura_stable).unwrap())
            .unwrap()
            .zone,
        Zone::Graveyard
    );
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.object(game.find_object_by_stable_id(creature_stable).unwrap())
            .unwrap()
            .zone,
        Zone::Graveyard
    );
}

#[test]
fn delayed_sacrifice_still_refers_to_a_creature_that_loses_its_creature_type() {
    use ironsmith::effects::EffectExecutor;
    let (mut game, aura, buried, mut queue) = setup();
    let stable = game.object(buried).unwrap().stable_id;
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let returned = game.find_object_by_stable_id(stable).unwrap();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::effects::ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(returned),
        ironsmith::continuous::Modification::SetCardTypes(vec![CardType::Artifact]),
        ironsmith::effect::Until::EndOfTurn,
    )
    .execute(
        &mut game,
        &mut ironsmith::effects::EffectContext::new(aura, PlayerId::from_index(0), &mut dm),
    )
    .unwrap();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.object(game.find_object_by_stable_id(stable).unwrap())
            .unwrap()
            .zone,
        Zone::Graveyard
    );
}

#[test]
fn strict_snapshot_passes_similarity_and_supported_behavior_gate() {
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Animate Dead",
    )
    .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.parse_error.is_none(), "{snapshot:#?}");
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
    println!("Unrounded similarity: {:?}", snapshot.similarity_score);
}
