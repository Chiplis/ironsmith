use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::get_blockers;
use ironsmith::effects::{EffectContext, ExtraTurnEffect, execute_effect};
use ironsmith::events::phase::BeginningOfUpkeepEvent;
use ironsmith::events::spells::SpellCastEvent;
use ironsmith::game_loop::execute_combat_damage_step;
use ironsmith::game_loop::{apply_attacker_declarations, apply_blocker_declarations};
use ironsmith::game_state::{PlayerControlDuration, PlayerControlStart};
use ironsmith::special_actions::{SpecialAction, can_perform};
use ironsmith::turn::{PriorityResult, PriorityTracker, execute_draw_step, pass_priority};
use ironsmith::{
    AttackTarget, AttackerDeclaration, BlockerDeclaration, CardId, CardType, CombatState, Effect,
    GameState, LegalAction, Phase, PlayerFilter, PlayerFilterExt, PlayerId, PowerToughness,
    PriorityLoopState, PriorityResponse, TriggerEvent, TriggerQueue, Zone, apply_priority_response,
    compute_legal_actions, compute_legal_attackers,
    generate_step_trigger_events_for_active_players,
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

fn enable_shared(game: &mut GameState, [alice, bob, charlie, diana]: [PlayerId; 4]) {
    game.set_teams(vec![vec![alice, bob], vec![charlie, diana]])
        .expect("valid adjacent teams");
    game.enable_shared_team_turns()
        .expect("adjacent teams can share turns");
}

fn creature(game: &mut GameState, controller: PlayerId, name: &str) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

fn simple_card(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    card_type: CardType,
    zone: Zone,
) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build();
    game.create_object_from_definition(&definition, owner, zone)
}

#[test]
fn u070_option_validates_adjacency_and_derives_primary_players() {
    let (mut invalid, [alice, bob, charlie, diana]) = four_player_game();
    invalid
        .set_teams(vec![vec![alice, charlie], vec![bob, diana]])
        .expect("identity itself is complete");
    assert!(invalid.enable_shared_team_turns().is_err());
    assert!(!invalid.shared_team_turns_enabled());

    let (mut game, players @ [alice, bob, charlie, diana]) = four_player_game();
    enable_shared(&mut game, players);
    assert_eq!(game.primary_player_for(alice), Some(bob));
    assert_eq!(game.primary_player_for(charlie), Some(diana));
    assert_eq!(game.turn.active_player, bob);
    assert_eq!(game.active_players(), vec![alice, bob]);
    assert!(game.is_active_player(alice));
    assert!(game.is_active_player(bob));
    assert!(!game.is_active_player(charlie));
    assert_eq!(
        game.team_apnap_player_order(),
        vec![alice, bob, charlie, diana]
    );

    assert!(
        game.set_shared_team_member_order(0, vec![alice, alice])
            .is_err(),
        "the selection must be a permutation of the team's members"
    );
    game.set_shared_team_member_order(0, vec![bob, alice])
        .expect("a team may choose its internal action order");
    assert_eq!(game.active_players(), vec![bob, alice]);
    assert_eq!(
        game.team_apnap_player_order(),
        vec![bob, alice, charlie, diana]
    );
}

#[test]
fn u070_each_active_teammate_draws_and_has_an_independent_land_play() {
    let (mut game, players @ [alice, bob, charlie, _diana]) = four_player_game();
    enable_shared(&mut game, players);
    simple_card(
        &mut game,
        alice,
        "Alice Draw",
        CardType::Artifact,
        Zone::Library,
    );
    simple_card(
        &mut game,
        bob,
        "Bob Draw",
        CardType::Artifact,
        Zone::Library,
    );
    simple_card(
        &mut game,
        charlie,
        "Charlie Draw",
        CardType::Artifact,
        Zone::Library,
    );
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::Step::Draw);

    let events = execute_draw_step(&mut game);
    assert_eq!(events.len(), 2);
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(game.player(bob).unwrap().hand.len(), 1);
    assert!(game.player(charlie).unwrap().hand.is_empty());

    let alice_land = simple_card(&mut game, alice, "Alice Land", CardType::Land, Zone::Hand);
    let bob_land = simple_card(&mut game, bob, "Bob Land", CardType::Land, Zone::Hand);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(bob);
    let mut dm = ironsmith::decision::AutoPassDecisionMaker;
    assert!(
        can_perform(
            &SpecialAction::PlayLand {
                card_id: alice_land
            },
            &game,
            alice,
            &mut dm
        )
        .is_ok()
    );
    assert!(
        can_perform(
            &SpecialAction::PlayLand { card_id: bob_land },
            &game,
            bob,
            &mut dm
        )
        .is_ok()
    );

    let mut queue = TriggerQueue::new();
    let mut priority = PriorityLoopState::new(game.teams_in_game());
    for (player, land) in [(alice, alice_land), (bob, bob_land)] {
        let action = compute_legal_actions(&game, player)
            .into_iter()
            .find(|action| matches!(action, LegalAction::PlayLand { land_id } if *land_id == land))
            .expect("each teammate has their own legal land play");
        apply_priority_response(
            &mut game,
            &mut queue,
            &mut priority,
            &PriorityResponse::PriorityAction(action),
        )
        .expect("team member plays their land");
    }
    assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 1);
    assert_eq!(game.player(bob).unwrap().lands_played_this_turn, 1);
}

#[test]
fn u070_priority_passes_by_team_but_any_member_can_act() {
    let (mut game, players @ [alice, bob, _charlie, diana]) = four_player_game();
    enable_shared(&mut game, players);
    game.turn.priority_player = Some(bob);
    assert!(game.team_has_priority(alice));
    assert!(game.team_has_priority(bob));

    let mut tracker = PriorityTracker::new(game.players_in_game());
    assert_eq!(
        pass_priority(&mut game, &mut tracker),
        PriorityResult::Continue
    );
    assert_eq!(game.turn.priority_player, Some(diana));
    assert_eq!(tracker.players_in_game, 2);
    assert_eq!(
        pass_priority(&mut game, &mut tracker),
        PriorityResult::PhaseEnds
    );
}

#[test]
fn u070_added_skipped_turns_and_player_control_apply_to_the_team() {
    let (mut game, players @ [alice, bob, charlie, diana]) = four_player_game();
    enable_shared(&mut game, players);

    game.turn_store.extra_turns.push(alice);
    game.next_turn();
    assert_eq!(
        game.turn.active_player, bob,
        "Alice's team takes the extra turn"
    );

    game.turn_store.skip_next_turn.insert(charlie);
    game.next_turn();
    assert_eq!(
        game.turn.active_player, bob,
        "one skip naming Charlie skips Charlie and Diana's shared turn"
    );

    game.add_player_control(
        alice,
        charlie,
        PlayerControlStart::Immediate,
        PlayerControlDuration::UntilEndOfTurn,
        None,
    );
    assert_eq!(game.controlling_player_for(charlie), alice);
    assert_eq!(game.controlling_player_for(diana), alice);
}

#[test]
fn u070_turn_relative_cost_reduction_expires_for_nonprimary_active_teammate() {
    let (mut game, players @ [alice, _bob, charlie, _diana]) = four_player_game();
    enable_shared(&mut game, players);
    let source = game.new_object_id();
    game.add_temporary_spell_cost_reduction_until(
        charlie,
        source,
        alice,
        ironsmith::ObjectFilter::default(),
        ironsmith::ManaCost::new(),
        1,
        ironsmith::Until::YourNextTurn,
    );
    game.turn.turn_number += 1;

    let reduction = game
        .effect_store
        .temporary_spell_cost_reductions
        .first()
        .expect("temporary reduction");
    assert!(
        reduction.is_expired(&game),
        "Alice's next turn has begun even though Bob is the primary player"
    );
}

#[test]
fn u070_attackers_and_blockers_are_declared_as_combined_team_sets() {
    let (mut game, players @ [alice, bob, charlie, diana]) = four_player_game();
    enable_shared(&mut game, players);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let alice_attacker = creature(&mut game, alice, "Alice Attacker");
    let bob_attacker = creature(&mut game, bob, "Bob Attacker");
    let charlie_blocker = creature(&mut game, charlie, "Charlie Blocker");
    let diana_blocker = creature(&mut game, diana, "Diana Blocker");
    game.remove_summoning_sickness(alice_attacker);
    game.remove_summoning_sickness(bob_attacker);

    let options = compute_legal_attackers(&game, &CombatState::default());
    assert!(
        options
            .iter()
            .any(|option| option.creature == alice_attacker)
    );
    assert!(options.iter().any(|option| option.creature == bob_attacker));

    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &[
            AttackerDeclaration {
                creature: alice_attacker,
                target: AttackTarget::Player(diana),
            },
            AttackerDeclaration {
                creature: bob_attacker,
                target: AttackTarget::Player(charlie),
            },
        ],
    )
    .expect("both teammates declare one legal combined attack");

    apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &[
            BlockerDeclaration {
                blocker: charlie_blocker,
                blocking: alice_attacker,
            },
            BlockerDeclaration {
                blocker: diana_blocker,
                blocking: bob_attacker,
            },
        ],
        diana,
    )
    .expect("either defender can block an attacker aimed at their teammate");
    assert_eq!(get_blockers(&combat, alice_attacker), &[charlie_blocker]);
    assert_eq!(get_blockers(&combat, bob_attacker), &[diana_blocker]);

    let damage = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(
        damage.len(),
        4,
        "both teams assign one combined damage batch"
    );
    assert!(damage.iter().any(|event| event.source == alice_attacker));
    assert!(damage.iter().any(|event| event.source == bob_attacker));
    assert!(damage.iter().any(|event| event.source == charlie_blocker));
    assert!(damage.iter().any(|event| event.source == diana_blocker));
}

#[test]
fn u070_restart_and_subgame_preserve_shared_turn_identity() {
    let (mut game, players @ [alice, bob, _charlie, _diana]) = four_player_game();
    enable_shared(&mut game, players);
    game.set_shared_team_member_order(0, vec![bob, alice])
        .expect("team order selected");
    game.restart_game(alice, &[]);
    assert!(game.shared_team_turns_enabled());
    assert_eq!(game.turn.active_player, bob);
    assert_eq!(game.active_players(), vec![bob, alice]);

    assert!(game.leave_game(bob));
    game.restart_game(alice, &[]);
    assert_eq!(game.turn.active_player, alice);
    assert_eq!(game.active_players(), vec![alice]);

    game.begin_subgame(None, alice, Vec::new())
        .expect("subgame preserves the shared-turn option");
    let shared = game.shared_team_turns().expect("shared turns in child");
    assert_eq!(shared.seats(), players);
    assert_eq!(shared.member_orders()[0], vec![bob, alice]);
}

#[test]
fn u070_step_events_and_singular_active_player_references_preserve_player_identity() {
    let (mut game, players @ [alice, bob, charlie, _diana]) = four_player_game();
    enable_shared(&mut game, players);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::Step::Upkeep);

    let events = generate_step_trigger_events_for_active_players(&game);
    let upkeep_players = events
        .iter()
        .filter_map(|event| event.downcast::<BeginningOfUpkeepEvent>())
        .map(|event| event.player)
        .collect::<Vec<_>>();
    assert_eq!(upkeep_players, vec![bob, alice]);

    let mut ctx = game.filter_context_for(charlie, None);
    ctx.active_player = game.singular_active_player(Some(alice));
    assert!(PlayerFilter::Active.matches_player(alice, &ctx));
    assert!(!PlayerFilter::Active.matches_player(bob, &ctx));

    ctx.active_player = game.singular_active_player(None);
    assert!(!PlayerFilter::Active.matches_player(alice, &ctx));
    assert!(PlayerFilter::Active.matches_player(bob, &ctx));
}

#[test]
fn u070_departing_primary_player_keeps_the_turn_and_priority_with_the_team() {
    let (mut game, players @ [alice, bob, _charlie, _diana]) = four_player_game();
    enable_shared(&mut game, players);
    game.turn.priority_player = Some(bob);

    assert!(game.leave_game(bob));
    assert_eq!(game.turn.active_player, alice);
    assert_eq!(game.turn.priority_player, Some(alice));
    assert_eq!(game.active_players(), vec![alice]);
}

#[test]
fn u070_one_effect_naming_two_teammates_adds_one_team_turn() {
    let (mut game, players @ [alice, bob, _charlie, _diana]) = four_player_game();
    enable_shared(&mut game, players);
    let source = game.new_object_id();
    let effect = Effect::new(ExtraTurnEffect::new(PlayerFilter::IteratedPlayer));
    let mut ctx = EffectContext::new_default(source, alice);

    for player in [alice, bob] {
        ctx.iteration.iterated_player = Some(player);
        execute_effect(&mut game, &effect, &mut ctx).expect("extra-turn effect resolves");
    }
    assert_eq!(game.turn_store.extra_turns, vec![bob]);

    let distinct_effect = Effect::new(ExtraTurnEffect::new(PlayerFilter::Specific(alice)));
    execute_effect(&mut game, &distinct_effect, &mut ctx).expect("distinct effect resolves");
    assert_eq!(game.turn_store.extra_turns, vec![bob, bob]);
}

#[test]
fn u070_day_night_counts_each_previous_active_teammate_separately() {
    let (mut game, players @ [alice, bob, charlie, _diana]) = four_player_game();
    enable_shared(&mut game, players);
    game.set_daytime(false);
    for caster in [alice, bob] {
        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(game.new_object_id(), caster, Zone::Hand),
            ironsmith::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&event, None, None);
    }
    game.next_turn();
    assert!(
        game.is_night,
        "one spell by each teammate is not two by one player"
    );

    for _ in 0..2 {
        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(game.new_object_id(), charlie, Zone::Hand),
            ironsmith::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&event, None, None);
    }
    game.next_turn();
    assert!(
        !game.is_night,
        "two spells by one active teammate make it day"
    );
}
