//! Declared attacking/defending players, exact event roles, and live curse
//! reward participants. All direct/artifact scenarios are authored, unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, LegalAction, SelectFirstDecisionMaker};
use ironsmith::effects::{CreateTokenEffect, EffectContext, EffectExecutor};
use ironsmith::events::EventCause;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::object::{AttachmentTarget, ObjectKind};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const D: PlayerId = PlayerId(3);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/player_attack_declarations.json.fixture"
    ))
    .unwrap()
}
fn definitions_from(name: &str, text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    definitions_from(name, row["text"].as_str().unwrap())
}
fn new_game(active: PlayerId, shared: bool) -> GameState {
    let mut game = GameState::new(
        vec![
            "Alice".into(),
            "Bob".into(),
            "Charlie".into(),
            "Dana".into(),
        ],
        20,
    );
    if shared {
        game.set_teams(vec![vec![A, B], vec![C, D]]).unwrap();
        game.enable_shared_team_turns().unwrap();
    }
    game.turn.active_player = if shared { B } else { active };
    game.turn.priority_player = Some(game.turn.active_player);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game
}
fn card(game: &mut GameState, player: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Declaration resource", text, false).unwrap();
    let id = game.create_object_from_definition(&definition, player, zone);
    game.remove_summoning_sickness(id);
    id
}
fn creature(game: &mut GameState, player: PlayerId) -> ObjectId {
    card(
        game,
        player,
        Zone::Battlefield,
        "Type: Creature\nPower/Toughness: 2/2",
    )
}
fn libraries(game: &mut GameState) {
    for player in [A, B, C, D] {
        for _ in 0..4 {
            card(game, player, Zone::Library, "Type: Artifact");
        }
    }
}
fn curse(
    game: &mut GameState,
    definition: &CardDefinition,
    owner: PlayerId,
    player: PlayerId,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, owner, Zone::Battlefield);
    assert!(game.attach_object_to_target(id, AttachmentTarget::Player(player)));
    game.take_pending_trigger_events();
    id
}
fn declare(game: &mut GameState, attacks: &[(ObjectId, AttackTarget)]) -> TriggerQueue {
    let declarations: Vec<_> = attacks
        .iter()
        .map(|(creature, target)| AttackerDeclaration {
            creature: *creature,
            target: target.clone(),
        })
        .collect();
    let mut queue = TriggerQueue::new();
    let mut combat = CombatState::default();
    apply_attacker_declarations(game, &mut combat, &mut queue, &declarations).unwrap();
    queue
}
fn settle(game: &mut GameState, mut queue: TriggerQueue) -> usize {
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    let count = game.stack.len();
    for _ in 0..30 {
        if game.stack_is_empty() {
            return count;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
        put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("attack declarations did not settle")
}
fn token_ids(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Token && game.controller_of(object) == player
            })
        })
        .collect()
}
fn execute(game: &mut GameState, source: ObjectId, player: PlayerId, effect: &dyn EffectExecutor) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, player, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}

#[test]
fn five_exact_curses_round_trip_complete_programs_and_jolene_retains_its_full_dependency() {
    assert_eq!(rows().len(), 6);
    for row in rows() {
        // Jolene's full fixture intentionally remains present and unignored:
        // integration also requires the separate token-replacement source batch.
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
                if triggered.trigger.compiled_model().is_some_and(|model| matches!(&model.kind, TriggerKind::PlayerAttackDeclaration { .. })))));
        }
    }
}

#[test]
fn every_curse_rewards_its_controller_and_the_opponent_attacking_its_exact_player_once() {
    for name in [
        "Curse of Bounty",
        "Curse of Disturbance",
        "Curse of Opulence",
        "Curse of Verbosity",
        "Curse of Vitality",
    ] {
        for definition in definitions(name) {
            let mut game = new_game(B, false);
            libraries(&mut game);
            curse(&mut game, &definition, A, C);
            let mut nonlands = Vec::new();
            let mut lands = Vec::new();
            for player in [A, B, C, D] {
                let nonland = card(&mut game, player, Zone::Battlefield, "Type: Artifact");
                game.tap(nonland);
                nonlands.push(nonland);
                let land = card(&mut game, player, Zone::Battlefield, "Type: Land");
                game.tap(land);
                lands.push(land);
            }
            let one = creature(&mut game, B);
            let two = creature(&mut game, B);
            let unrelated = creature(&mut game, B);
            let queue = declare(
                &mut game,
                &[
                    (one, AttackTarget::Player(C)),
                    (two, AttackTarget::Player(C)),
                    (unrelated, AttackTarget::Player(D)),
                ],
            );
            assert_eq!(
                queue.entries.len(),
                1,
                "{name}: one attacked-player event despite multiple creatures"
            );
            assert_eq!(settle(&mut game, queue), 1);
            for (index, player) in [A, B, C, D].into_iter().enumerate() {
                let rewarded = player == A || player == B;
                match name {
                    "Curse of Bounty" => {
                        assert_eq!(
                            !game.is_tapped(nonlands[index]),
                            rewarded,
                            "untap only each recipient's own nonlands"
                        );
                        assert!(game.is_tapped(lands[index]));
                    }
                    "Curse of Verbosity" => assert_eq!(
                        game.player(player).unwrap().hand.len(),
                        usize::from(rewarded)
                    ),
                    "Curse of Vitality" => assert_eq!(
                        game.player(player).unwrap().life,
                        if rewarded { 22 } else { 20 }
                    ),
                    _ => {
                        let ids = token_ids(&game, player);
                        assert_eq!(ids.len(), usize::from(rewarded));
                        if rewarded {
                            let object = game.object(ids[0]).unwrap();
                            if name == "Curse of Disturbance" {
                                assert!(
                                    object.subtypes.contains(&ironsmith::types::Subtype::Zombie)
                                );
                                assert_eq!(game.calculated_power(ids[0]), Some(2));
                                assert_eq!(game.calculated_toughness(ids[0]), Some(2));
                            } else {
                                assert!(object.subtypes.contains(&ironsmith::types::Subtype::Gold));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn shared_attack_declarations_group_by_defender_and_do_not_reward_the_controllers_teammate() {
    for definition in definitions("Curse of Verbosity") {
        // Both active teammates are opponents of the curse's controller.
        let mut game = new_game(A, true);
        libraries(&mut game);
        curse(&mut game, &definition, C, D);
        let a = creature(&mut game, A);
        let b = creature(&mut game, B);
        let queue = declare(
            &mut game,
            &[(a, AttackTarget::Player(D)), (b, AttackTarget::Player(D))],
        );
        assert_eq!(
            queue.entries.len(),
            1,
            "one defender, not one trigger per attacking teammate"
        );
        settle(&mut game, queue);
        for (player, count) in [(A, 1), (B, 1), (C, 1), (D, 0)] {
            assert_eq!(game.player(player).unwrap().hand.len(), count);
        }
        // The controller's own teammate is not their opponent.
        let mut game = new_game(A, true);
        libraries(&mut game);
        curse(&mut game, &definition, A, C);
        let a = creature(&mut game, A);
        let b = creature(&mut game, B);
        let queue = declare(
            &mut game,
            &[(a, AttackTarget::Player(C)), (b, AttackTarget::Player(C))],
        );
        settle(&mut game, queue);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(
            game.player(B).unwrap().hand.len(),
            0,
            "Attacking minus You would incorrectly reward this teammate"
        );
    }
    for definition in definitions_from(
        "Each attacked player",
        "Type: Enchantment\nWhenever a player is attacked, you gain 1 life.",
    ) {
        let mut game = new_game(A, false);
        game.create_object_from_definition(&definition, D, Zone::Battlefield);
        let one = creature(&mut game, A);
        let two = creature(&mut game, A);
        let three = creature(&mut game, A);
        let queue = declare(
            &mut game,
            &[
                (one, AttackTarget::Player(B)),
                (two, AttackTarget::Player(B)),
                (three, AttackTarget::Player(C)),
            ],
        );
        assert_eq!(
            queue.entries.len(),
            2,
            "distinct defenders keep distinct groups"
        );
        settle(&mut game, queue);
        assert_eq!(game.player(D).unwrap().life, 22);
    }
}

#[test]
fn rewards_read_current_attackers_but_keep_the_original_defender_when_the_aura_moves() {
    for definition in definitions("Curse of Verbosity") {
        for add_later_attacker in [false, true] {
            let mut game = new_game(B, false);
            libraries(&mut game);
            let source = curse(&mut game, &definition, A, C);
            let original = creature(&mut game, B);
            let elsewhere = creature(&mut game, B);
            let queue = declare(
                &mut game,
                &[
                    (original, AttackTarget::Player(C)),
                    (elsewhere, AttackTarget::Player(D)),
                ],
            );
            assert_eq!(queue.entries.len(), 1);
            game.move_object(original, Zone::Graveyard, EventCause::from_effect(source, A))
                .unwrap();
            assert!(game.attach_object_to_target(source, AttachmentTarget::Player(D)));
            if add_later_attacker {
                let token = compile_to_runtime_definition(
                    "Late attacker",
                    "Type: Creature\nPower/Toughness: 1/1",
                    false,
                )
                .unwrap();
                execute(
                    &mut game,
                    source,
                    B,
                    &CreateTokenEffect::new(token, 1, PlayerFilter::Specific(B))
                        .tapped()
                        .attacking_player(PlayerFilter::Specific(C)),
                );
            }
            assert_eq!(
                settle(&mut game, queue),
                1,
                "entering attacking creates no new player declaration"
            );
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert_eq!(
                game.player(B).unwrap().hand.len(),
                usize::from(add_later_attacker),
                "attacking the aura's new player or merely having declared an old attacker does not qualify"
            );
        }
    }
}

#[test]
fn a_new_attacking_teammate_can_qualify_for_the_reward_without_creating_another_trigger() {
    for definition in definitions("Curse of Vitality") {
        let mut game = new_game(A, true);
        let source = curse(&mut game, &definition, C, D);
        let attacker = creature(&mut game, A);
        let queue = declare(&mut game, &[(attacker, AttackTarget::Player(D))]);
        let token = compile_to_runtime_definition(
            "New attacking participant",
            "Type: Creature\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        execute(
            &mut game,
            source,
            B,
            &CreateTokenEffect::new(token, 1, PlayerFilter::Specific(B))
                .tapped()
                .attacking_player(PlayerFilter::Specific(D)),
        );
        assert_eq!(settle(&mut game, queue), 1);
        for (player, life) in [(A, 22), (B, 22), (C, 22), (D, 20)] {
            assert_eq!(game.player(player).unwrap().life, life);
        }
    }
}

#[test]
fn planeswalker_and_battle_attacks_do_not_mean_the_player_was_attacked() {
    for definition in definitions("Curse of Vitality") {
        let mut game = new_game(B, false);
        curse(&mut game, &definition, A, C);
        let planeswalker = card(
            &mut game,
            C,
            Zone::Battlefield,
            "Type: Planeswalker\nLoyalty: 5",
        );
        let battle = card(
            &mut game,
            B,
            Zone::Battlefield,
            "Type: Battle — Siege\nDefense: 5",
        );
        assert!(game.set_battle_protector(battle, C));
        let one = creature(&mut game, B);
        let two = creature(&mut game, B);
        let queue = declare(
            &mut game,
            &[
                (one, AttackTarget::Planeswalker(planeswalker)),
                (two, AttackTarget::Battle(battle)),
            ],
        );
        assert!(queue.entries.is_empty());
        settle(&mut game, queue);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}

#[test]
fn grouped_attack_actor_is_frozen_and_multiple_qualifying_defenders_are_one_event() {
    let text = "Type: Enchantment\nWhenever a player attacks one or more of your opponents, that attacking player creates a Treasure token.";
    for definition in definitions_from("Attacking player reward", text) {
        for only_controller in [false, true] {
            let mut game = new_game(B, false);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let one = creature(&mut game, B);
            let two = creature(&mut game, B);
            let queue = if only_controller {
                declare(&mut game, &[(one, AttackTarget::Player(A))])
            } else {
                declare(
                    &mut game,
                    &[
                        (one, AttackTarget::Player(C)),
                        (two, AttackTarget::Player(D)),
                    ],
                )
            };
            assert_eq!(queue.entries.len(), usize::from(!only_controller));
            game.move_object(one, Zone::Graveyard, EventCause::from_effect(source, A))
                .unwrap();
            if !only_controller {
                game.move_object(two, Zone::Graveyard, EventCause::from_effect(source, A))
                    .unwrap();
            }
            settle(&mut game, queue);
            assert_eq!(token_ids(&game, A).len(), 0);
            assert_eq!(
                token_ids(&game, B).len(),
                usize::from(!only_controller),
                "the original attacking player still creates its token after all its creatures leave"
            );
        }
    }
}

#[test]
fn opulence_gold_token_has_the_real_sacrifice_mana_ability() {
    for definition in definitions("Curse of Opulence") {
        let mut game = new_game(B, false);
        curse(&mut game, &definition, A, C);
        let attacker = creature(&mut game, B);
        let queue = declare(&mut game, &[(attacker, AttackTarget::Player(C))]);
        settle(&mut game, queue);
        let gold = token_ids(&game, A)[0];
        game.turn.priority_player = Some(A);
        let action = ironsmith::decision::compute_legal_actions(&game,A).unwrap().into_iter()
            .find(|action| matches!(action, LegalAction::ActivateManaAbility { source, .. } if *source == gold)).unwrap();
        let mut state = PriorityLoopState::new(game.players.len());
        let mut queue = TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..15 {
            if game.player(A).unwrap().mana_pool.total() == 1 {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("mana activation pending without choice");
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
        assert!(!game.battlefield.contains(&gold));
    }
}

#[test]
fn typed_live_attack_relation_fails_closed_without_its_event_defender() {
    let mut game = new_game(A, false);
    let source = card(&mut game, A, Zone::Battlefield, "Type: Artifact");
    let effect = ironsmith::effects::ForPlayersEffect::new(
        PlayerFilter::opponents_attacking_event_defender(),
        vec![ironsmith::Effect::gain_life(1)],
    );
    assert!(
        effect
            .execute(&mut game, &mut EffectContext::new_default(source, A))
            .is_err()
    );
}

#[test]
fn active_singular_subject_anaphor_means_attacker_and_shared_actors_remain_distinct() {
    for definition in definitions_from(
        "Declared actor reference",
        "Type: Enchantment\nWhenever an opponent attacks you, that player gains 2 life.",
    ) {
        let mut game = new_game(B, false);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let attacker = creature(&mut game, B);
        let queue = declare(&mut game, &[(attacker, AttackTarget::Player(A))]);
        assert_eq!(queue.entries.len(), 1);
        settle(&mut game, queue);
        assert_eq!(game.player(B).unwrap().life, 22);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
    for definition in definitions_from(
        "Shared attacking participants",
        "Type: Enchantment\nWhenever a player attacks one or more of your opponents, that attacking player creates a Treasure token.",
    ) {
        let mut game = new_game(A, true);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let a = creature(&mut game, A);
        let b = creature(&mut game, B);
        let queue = declare(
            &mut game,
            &[(a, AttackTarget::Player(C)), (b, AttackTarget::Player(C))],
        );
        assert_eq!(
            queue.entries.len(),
            2,
            "each separately authored attacking player triggers"
        );
        settle(&mut game, queue);
        assert_eq!(token_ids(&game, A).len(), 1);
        assert_eq!(token_ids(&game, B).len(), 1);
        assert!(token_ids(&game, C).is_empty());
    }
}

#[test]
fn malformed_player_attack_surfaces_do_not_compile_as_a_broader_event() {
    for clause in [
        "an unknown player is attacked",
        "enchanted creature is attacked",
        "a player attacks you during an unknown phase",
    ] {
        let text = format!("Type: Enchantment\nWhenever {clause}, you gain 1 life.");
        assert!(
            compile_to_runtime_definition("Rejected attack boundary", &text, false).is_err(),
            "{clause}"
        );
    }
}
