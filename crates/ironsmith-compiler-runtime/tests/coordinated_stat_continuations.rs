use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::object::{AttachmentTarget, CounterType};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/coordinated_stat_continuations.json.fixture"
    ))
    .unwrap();
    let card = cards.into_iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        card["mana_cost"].as_str().unwrap(),
        card["type_line"].as_str().unwrap()
    );
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(card["oracle_text"].as_str().unwrap());
    let (artifact, direct) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}

fn target(game: &mut GameState, owner: PlayerId) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Large fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(9, 9))
            .build(),
        owner,
        Zone::Battlefield,
    )
}

#[test]
fn replacement_anthem_uses_its_controller_condition_and_replaces_rather_than_stacks() {
    for definition in definitions("Precipitous Drop") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let enchanted = target(&mut game, bob);
        let other = target(&mut game, alice);
        let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(enchanted)));
        assert_eq!(
            (
                game.current_power(enchanted),
                game.current_toughness(enchanted)
            ),
            (Some(7), Some(7))
        );
        game.record_completed_dungeon(bob, "Opponent dungeon");
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.current_power(enchanted),
            Some(7),
            "recipient controller's achievement must not qualify the Aura controller"
        );
        assert!(
            game.continuous_state_is_clean_public(),
            "prime a clean static-effect cache"
        );
        game.record_completed_dungeon(alice, "Controller dungeon");
        assert!(
            !game.continuous_state_is_clean_public(),
            "recording completion must invalidate conditional continuous effects"
        );
        assert_eq!(
            game.current_power(enchanted),
            Some(4),
            "read-only characteristic queries must see the completion immediately"
        );
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            (
                game.current_power(enchanted),
                game.current_toughness(enchanted)
            ),
            (Some(4), Some(4)),
            "-5/-5 replaces -2/-2; it must not become -7/-7"
        );
        assert_eq!(game.current_power(other), Some(9));
        game.set_current_controller(aura, charlie).unwrap();
        assert_eq!(
            game.current_power(enchanted),
            Some(7),
            "an unqualified new Aura controller restores the base modifier"
        );
        game.set_current_controller(aura, alice).unwrap();
        assert_eq!(game.current_power(enchanted), Some(4));
        game.next_turn();
        assert!(
            game.has_completed_dungeon(alice),
            "completion is game-scoped, not turn-scoped"
        );
        assert_eq!(
            game.current_power(enchanted),
            Some(4),
            "the modifier stays active on another player's turn"
        );
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(other)));
        assert_eq!(game.current_power(enchanted), Some(9));
        assert_eq!(game.current_power(other), Some(4));
        game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        assert_eq!(game.current_power(other), Some(9));
    }
}

fn perform_action_and_resolve(
    game: &mut ironsmith::GameState,
    action: ironsmith::decision::LegalAction,
) {
    perform_action_and_resolve_with(
        game,
        action,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    );
}

fn perform_action_and_resolve_with(
    game: &mut ironsmith::GameState,
    action: ironsmith::decision::LegalAction,
    dm: &mut impl ironsmith::decision::DecisionMaker,
) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
    };
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_activation.is_none() && state.pending_cast.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            break;
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_activation.is_none() && state.pending_cast.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    assert!(
        !game.stack_is_empty(),
        "an activation or cast must really reach the stack"
    );
    while !game.stack_is_empty() {
        resolve_stack_entry_with(game, dm).unwrap();
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    }
}

#[test]
fn coordinated_base_stats_grant_and_subtype_removal_share_subject_and_duration() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_state::Phase;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::static_abilities::StaticAbilityId;
    for definition in definitions("Werewolf Pack Leader") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 4);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .add_counters(CounterType::PlusOnePlusOne, 1);
        let action = compute_legal_actions(&game, alice)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
            .expect("four-mana activation should be payable");
        perform_action_and_resolve(&mut game, action);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(
            (game.current_power(source), game.current_toughness(source)),
            (Some(6), Some(4))
        );
        let characteristics = game.calculated_characteristics(source).unwrap();
        assert!(characteristics.subtypes.contains(&Subtype::Werewolf));
        assert!(!characteristics.subtypes.contains(&Subtype::Human));
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Trample));
        assert_eq!(game.current_power(other), Some(3));
        assert!(
            game.calculated_characteristics(other)
                .unwrap()
                .subtypes
                .contains(&Subtype::Human)
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(
            (game.current_power(source), game.current_toughness(source)),
            (Some(4), Some(4))
        );
        assert!(
            game.calculated_characteristics(source)
                .unwrap()
                .subtypes
                .contains(&Subtype::Human)
        );
        assert!(!game.current_has_static_ability_id(source, StaticAbilityId::Trample));
    }
}
