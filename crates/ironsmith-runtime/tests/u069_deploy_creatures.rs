use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::continuous::EffectTarget;
use ironsmith::decisions::context::DecisionContext;
use ironsmith::{
    AbilityKind, AttackTarget, CardId, CardType, CombatState, ContinuousEffect, GameProgress,
    GameState, LegalAction, Modification, Phase, PlayerFilter, PlayerFilterExt, PlayerId,
    PowerToughness, PriorityLoopState, PriorityResponse, Target, TriggerQueue, Until, Zone,
    apply_priority_response, compute_legal_actions, compute_legal_attackers, resolve_stack_entry,
};

fn four_player_game() -> (GameState, [PlayerId; 4]) {
    let game = GameState::new(
        vec![
            "Alice".into(),
            "Bob".into(),
            "Charlie".into(),
            "Diana".into(),
        ],
        20,
    );
    (
        game,
        [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
            PlayerId::from_index(3),
        ],
    )
}

fn enable_two_teams(game: &mut GameState, players: [PlayerId; 4]) {
    game.set_teams(vec![
        vec![players[0], players[1]],
        vec![players[2], players[3]],
    ])
    .expect("valid two-team assignment");
    game.set_deploy_creatures(true);
}

fn creature(game: &mut GameState, controller: PlayerId, name: &str) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

fn artifact(game: &mut GameState, controller: PlayerId, name: &str) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

fn deploy_ability_index(game: &GameState, source: ironsmith::ObjectId) -> usize {
    game.current_abilities(source)
        .expect("source has current abilities")
        .iter()
        .position(|ability| {
            matches!(&ability.kind, AbilityKind::Activated(activated)
                if activated.timing == ironsmith::ability::ActivationTiming::SorcerySpeed
                    && activated.mana_cost.costs().iter().any(|cost| cost.requires_tap()))
        })
        .expect("creature has the deploy ability")
}

fn set_main_phase_priority(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}

fn legal_deploy_action(
    game: &GameState,
    player: PlayerId,
    source: ironsmith::ObjectId,
) -> Option<LegalAction> {
    compute_legal_actions(game, player).into_iter().find(
        |action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source),
    )
}

fn activate_deploy(game: &mut GameState, source: ironsmith::ObjectId, target: PlayerId) {
    let player = game.current_controller(source).expect("source controller");
    let action = legal_deploy_action(game, player, source).expect("deploy action is legal");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let progress = apply_priority_response(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
    )
    .expect("deploy activation starts");
    let GameProgress::NeedsDecisionCtx(DecisionContext::Targets(context)) = progress else {
        panic!("expected teammate target decision, got {progress:?}");
    };
    assert_eq!(context.requirements.len(), 1);
    assert_eq!(
        context.requirements[0].legal_targets,
        vec![Target::Player(target)]
    );

    apply_priority_response(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(target)]),
    )
    .expect("legal teammate target completes activation");
}

#[test]
fn u069_team_identity_is_transactional_and_drives_player_filters() {
    let (mut game, [alice, bob, charlie, diana]) = four_player_game();
    assert!(
        game.set_teams(vec![vec![alice, bob], vec![bob, charlie, diana]])
            .is_err()
    );
    assert!(
        game.team_state().is_none(),
        "invalid setup is transactional"
    );

    enable_two_teams(&mut game, [alice, bob, charlie, diana]);
    assert!(game.are_teammates(alice, bob));
    assert!(!game.are_teammates(alice, alice));
    assert!(game.are_opponents(alice, charlie));
    assert!(!game.are_opponents(alice, bob));

    let context = game.filter_context_for(alice, None);
    assert!(PlayerFilter::Teammate.matches_player(bob, &context));
    assert!(!PlayerFilter::Opponent.matches_player(bob, &context));
    assert!(PlayerFilter::Opponent.matches_player(charlie, &context));

    let attacker = creature(&mut game, alice, "Team Attacker");
    game.remove_summoning_sickness(attacker);
    let option = compute_legal_attackers(&game, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == attacker)
        .expect("opposing team remains attackable");
    assert!(!option.valid_targets.contains(&AttackTarget::Player(bob)));
    assert!(
        option
            .valid_targets
            .contains(&AttackTarget::Player(charlie))
    );
}

#[test]
fn u069_deploy_is_granted_at_the_layer_six_boundary() {
    let (mut game, players @ [alice, _bob, _charlie, _diana]) = four_player_game();
    let printed_creature = creature(&mut game, alice, "Printed Creature");
    let animated = artifact(&mut game, alice, "Animated Artifact");
    let abilityless = creature(&mut game, alice, "Abilityless Creature");
    let rule_source = artifact(&mut game, alice, "Rule Source");

    assert!(game.current_abilities(printed_creature).unwrap().is_empty());
    enable_two_teams(&mut game, players);
    assert_eq!(game.current_abilities(printed_creature).unwrap().len(), 1);

    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::new(
            rule_source,
            alice,
            EffectTarget::Specific(animated),
            Modification::AddCardTypes(vec![CardType::Creature]),
        )
        .until(Until::Forever),
    );
    assert!(game.current_is_creature(animated));
    assert_eq!(game.current_abilities(animated).unwrap().len(), 1);

    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::new(
            rule_source,
            alice,
            EffectTarget::Specific(abilityless),
            Modification::SetAbilities(Vec::new()),
        )
        .until(Until::Forever),
    );
    assert!(game.current_abilities(abilityless).unwrap().is_empty());

    game.set_deploy_creatures(false);
    assert!(game.current_abilities(printed_creature).unwrap().is_empty());
}

#[test]
fn u069_tap_sorcery_timing_and_target_legality_use_the_normal_activation_path() {
    let (mut game, players @ [alice, bob, _charlie, _diana]) = four_player_game();
    enable_two_teams(&mut game, players);
    set_main_phase_priority(&mut game, alice);
    let source = creature(&mut game, alice, "Deployable Creature");
    game.set_summoning_sick(source);
    let ability_index = deploy_ability_index(&game, source);
    assert_eq!(ability_index, 0);

    assert!(
        legal_deploy_action(&game, alice, source).is_none(),
        "summoning sickness forbids the tap cost"
    );
    game.remove_summoning_sickness(source);
    assert!(legal_deploy_action(&game, alice, source).is_some());

    game.turn.phase = Phase::Combat;
    assert!(
        legal_deploy_action(&game, alice, source).is_none(),
        "deploy is sorcery speed"
    );
    game.turn.phase = Phase::FirstMain;

    activate_deploy(&mut game, source, bob);
    assert!(
        game.is_tapped(source),
        "the tap cost is paid during activation"
    );
    assert_eq!(
        game.current_controller(source),
        Some(alice),
        "control changes only on resolution"
    );
    resolve_stack_entry(&mut game).expect("deploy ability resolves");
    assert_eq!(game.current_controller(source), Some(bob));
    assert!(game.is_tapped(source));
    assert!(
        game.is_summoning_sick(source),
        "control change applies normal summoning sickness"
    );
    game.remove_summoning_sickness(source);
    set_main_phase_priority(&mut game, bob);
    assert!(
        legal_deploy_action(&game, bob, source).is_none(),
        "an already tapped creature cannot pay the deploy cost"
    );
    game.untap(source);
    assert!(legal_deploy_action(&game, bob, source).is_some());
}

#[test]
fn u069_range_and_resolution_revalidation_can_make_the_teammate_target_illegal() {
    let (mut ranged, players @ [alice, _bob, _charlie, _diana]) = four_player_game();
    enable_two_teams(&mut ranged, players);
    ranged
        .enable_limited_range_of_influence(players.to_vec(), vec![0, 1, 1, 1])
        .expect("valid limited range");
    set_main_phase_priority(&mut ranged, alice);
    let out_of_range = creature(&mut ranged, alice, "Out-of-range Creature");
    ranged.remove_summoning_sickness(out_of_range);
    assert!(legal_deploy_action(&ranged, alice, out_of_range).is_none());

    let (mut game, players @ [alice, bob, _charlie, _diana]) = four_player_game();
    enable_two_teams(&mut game, players);
    set_main_phase_priority(&mut game, alice);
    let source = creature(&mut game, alice, "Fizzling Deploy Creature");
    game.remove_summoning_sickness(source);
    activate_deploy(&mut game, source, bob);
    assert!(game.leave_game(bob));
    resolve_stack_entry(&mut game).expect("illegal target makes deploy resolve without effect");
    assert_eq!(game.current_controller(source), Some(alice));
    assert!(game.is_tapped(source), "paid costs are not refunded");

    let (mut departed_source_game, players @ [alice, bob, _charlie, _diana]) = four_player_game();
    enable_two_teams(&mut departed_source_game, players);
    set_main_phase_priority(&mut departed_source_game, alice);
    let source = creature(&mut departed_source_game, alice, "Departed Deploy Creature");
    departed_source_game.remove_summoning_sickness(source);
    activate_deploy(&mut departed_source_game, source, bob);
    let graveyard_object = departed_source_game
        .move_object_by_effect(source, Zone::Graveyard)
        .expect("source leaves as a new object");
    resolve_stack_entry(&mut departed_source_game)
        .expect("a departed source leaves nothing to transfer");
    assert_eq!(
        departed_source_game.current_controller(graveyard_object),
        Some(alice)
    );
}

#[test]
fn u069_restart_and_subgame_preserve_teams_and_the_option() {
    let (mut game, players @ [alice, bob, charlie, diana]) = four_player_game();
    enable_two_teams(&mut game, players);
    game.restart_game(alice, &[]);
    assert!(game.deploy_creatures_enabled());
    assert!(game.are_teammates(alice, bob));
    assert!(game.are_opponents(alice, charlie));

    game.begin_subgame(None, alice, Vec::new())
        .expect("subgame begins");
    assert!(game.deploy_creatures_enabled());
    assert!(game.are_teammates(alice, bob));
    assert!(game.are_teammates(charlie, diana));
}
