use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::effect::Effect;
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerId, Zone};

#[test]
fn plotted_designation_grants_cast_permission_without_a_printed_plot_ability() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let definition = CardDefinitionBuilder::new(CardId::new(), "Plotted fixture")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![
            ironsmith::mana::ManaSymbol::Generic(5),
        ]]))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let exiled = game.create_object_from_definition(&definition, alice, Zone::Exile);
    game.set_plotted(exiled, alice);
    let can_cast = |game: &GameState, player| {
        compute_legal_actions(game, player).iter().any(|action| {
            matches!(action, LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == exiled)
        })
    };
    assert!(
        !can_cast(&game, alice),
        "plot never permits casting on the turn it became plotted"
    );
    game.turn.turn_number += 1;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    assert!(!can_cast(&game, bob), "permission belongs to the owner");
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    assert!(
        can_cast(&game, alice),
        "being plotted must permit free casting even without a printed Plot ability"
    );
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    assert!(
        !can_cast(&game, alice),
        "plotted casting requires sorcery timing"
    );
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let action = compute_legal_actions(&game, alice).into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled)
    ).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut decisions = ironsmith::decision::SelectFirstDecisionMaker;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut decisions,
    )
    .unwrap();
    for _ in 0..12 {
        if !game.stack.is_empty() {
            break;
        }
        let ironsmith::decision::GameProgress::NeedsDecisionCtx(context) = result else {
            panic!("{result:?}");
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &context,
            &mut decisions,
        )
        .unwrap();
    }
    assert_eq!(
        game.stack.len(),
        1,
        "the free cast must actually reach the stack"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 21);
}

#[test]
fn become_plotted_marks_only_exiled_cards_and_grants_the_owner_permission() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardDefinitionBuilder::new(CardId::new(), "Designation fixture")
        .card_types(vec![CardType::Sorcery])
        .build();
    let exiled = game.create_object_from_definition(&card, bob, Zone::Exile);
    let in_hand = game.create_object_from_definition(&card, bob, Zone::Hand);
    let source = game.new_object_id();
    let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
    for target in [exiled, in_hand] {
        let effect = Effect::new(ironsmith::effects::BecomePlottedEffect::new(
            ironsmith::target::ChooseSpec::SpecificObject(target),
        ));
        ironsmith::effects::execute_effect(&mut game, &effect, &mut context).unwrap();
    }
    assert_eq!(game.plotted_by(exiled), Some(bob));
    assert_eq!(game.plotted_by(in_hand), None);
    let plotted_turn = game.plotted_turn(exiled);
    game.turn.turn_number += 1;
    ironsmith::effects::execute_effect(
        &mut game,
        &Effect::new(ironsmith::effects::BecomePlottedEffect::new(
            ironsmith::target::ChooseSpec::SpecificObject(exiled),
        )),
        &mut context,
    )
    .unwrap();
    assert_eq!(
        game.plotted_turn(exiled),
        plotted_turn,
        "an existing designation isn't renewed"
    );
    game.move_object_by_effect(exiled, Zone::Hand).unwrap();
    assert_eq!(
        game.plotted_by(exiled),
        None,
        "designation ends on leaving exile"
    );
}

fn definition() -> ironsmith::cards::CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Aven Interrupter",
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

#[test]
fn tax_applies_only_to_opponents_spells_from_graveyard_or_exile() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let mana =
        ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(3)]]);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Tax fixture")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(mana.clone())
        .build();
    for player in [alice, bob] {
        for zone in [
            Zone::Hand,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ] {
            let id = game.create_object_from_definition(&spell, player, zone);
            let cost = ironsmith::decision::calculate_effective_mana_cost(
                &game,
                player,
                game.object(id).unwrap(),
                &mana,
            );
            let taxed = player == bob && matches!(zone, Zone::Graveyard | Zone::Exile);
            assert_eq!(
                cost.mana_value(),
                if taxed { 5 } else { 3 },
                "player {player:?} casting from {zone:?}"
            );
        }
    }
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let id = game.create_object_from_definition(&spell, bob, Zone::Exile);
    assert_eq!(
        ironsmith::decision::calculate_effective_mana_cost(
            &game,
            bob,
            game.object(id).unwrap(),
            &mana
        )
        .mana_value(),
        3,
        "tax ends when its source leaves"
    );
}

#[test]
fn enters_exiles_uncounterable_spell_and_plots_it_for_its_owner() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Exiled spell fixture")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![
            ironsmith::mana::ManaSymbol::Generic(5),
        ]]))
        .with_ability(ironsmith::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::cant_be_countered_ability(),
        ))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell = game.create_object_from_definition(&spell_definition, bob, Zone::Stack);
    let stable = game.object(spell).unwrap().stable_id;
    game.set_current_controller(spell, alice);
    game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
    let aven = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let aven = game.move_object_by_effect(aven, Zone::Battlefield).unwrap();
    assert!(
        game.object_has_static_ability_id(
            aven,
            ironsmith::static_abilities::StaticAbilityId::Flash
        )
    );
    assert!(
        game.object_has_static_ability_id(
            aven,
            ironsmith::static_abilities::StaticAbilityId::Flying
        )
    );
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        2,
        "enter trigger must go above the original spell"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert!(
        game.stack.is_empty(),
        "exiling removes even an uncounterable spell from the stack"
    );
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    assert_eq!(
        game.plotted_by(exiled),
        Some(bob),
        "owner receives plotted permission, not former controller"
    );
    game.turn.turn_number += 1;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let action = |game: &GameState| {
        compute_legal_actions(game, bob).into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled))
    };
    assert!(
        action(&game).is_none(),
        "free plotted cast must still pay Aven's two-mana tax"
    );
    game.player_mut(bob)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Colorless, 2);
    let action =
        action(&game).expect("two mana pays the tax while the five-mana printed cost is waived");
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
    assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(bob).unwrap().life, 21);
}

#[test]
fn strict_snapshot_has_supported_structure_and_unrounded_score() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Aven Interrupter",
    )
    .unwrap()
    .remove(0);
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload);
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(
        snapshot
            .compiled_text
            .as_deref()
            .unwrap()
            .contains("from graveyards or from exile"),
        "the tax's origin restriction must be rendered: {snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
    println!("Unrounded similarity: {:?}", snapshot.similarity_score);
}
