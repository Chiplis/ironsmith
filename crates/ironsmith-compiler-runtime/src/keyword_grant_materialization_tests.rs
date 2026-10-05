use super::*;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
use ironsmith::game_loop::{
    apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::object::AttachmentTarget;
use ironsmith::triggers::{
    AttackEventTarget, TriggerEvent, TriggerQueue, TriggeredAbilityEntry, check_triggers,
};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) = compile_to_artifact(name, text, false).unwrap();
    artifact.validate().unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&artifact).unwrap();
    [direct, registry.get(name).unwrap().clone()]
}

fn creature(game: &mut GameState, owner: PlayerId, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(
        "Grant recipient",
        format!("Type: Creature — Human\nPower/Toughness: 2/2\n{text}"),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn attack_triggers(
    game: &GameState,
    attacker: ObjectId,
    defender: PlayerId,
) -> Vec<TriggeredAbilityEntry> {
    check_triggers(
        game,
        &TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(attacker, AttackEventTarget::Player(defender)),
            Default::default(),
        ),
    )
}

fn stack_triggers(game: &mut GameState, entries: &[TriggeredAbilityEntry]) {
    assert!(game.stack.is_empty());
    let mut queue = TriggerQueue::new();
    for entry in entries {
        queue.add(entry.clone());
    }
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    assert!(queue.entries.is_empty());
    assert_eq!(
        game.stack.len(),
        entries.len(),
        "each trigger gets a separate stack entry"
    );
}

fn resolve_top(game: &mut GameState) {
    let before = game.stack.len();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    resolve_stack_entry_with(game, &mut dm).unwrap();
    assert_eq!(game.stack.len(), before - 1);
}

#[test]
fn keyword_grant_materialization_melee_stacks_and_respects_recipient_scope() {
    for definition in definitions(
        "Adriana, Captain of the Guard",
        "Type: Legendary Creature — Human Knight\nPower/Toughness: 4/4\nMelee\nOther creatures you control have melee.",
    ) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let recipient = creature(&mut game, alice, "Melee");
        let plain = creature(&mut game, alice, "");
        let opponent = creature(&mut game, bob, "");
        game.turn.active_player = alice;
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        game.remove_summoning_sickness(recipient);
        game.remove_summoning_sickness(plain);
        let mut combat = CombatState::default();
        let mut attack_queue = TriggerQueue::new();
        // The compiled TurnHistoryCount reads recorded attack events and their
        // controller snapshots, not the legacy attacked-player summary map.
        apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut attack_queue,
            &[
                AttackerDeclaration {
                    creature: recipient,
                    target: AttackTarget::Player(bob),
                },
                AttackerDeclaration {
                    creature: plain,
                    target: AttackTarget::Player(cara),
                },
            ],
        )
        .unwrap();
        assert_eq!(combat.attackers.len(), 2);
        assert_eq!(
            attack_queue.entries.len(),
            3,
            "recipient has two melee instances, support has one"
        );
        assert_eq!(
            attack_triggers(&game, source, bob).len(),
            1,
            "other excludes the grant source"
        );
        assert_eq!(attack_triggers(&game, plain, bob).len(), 1);
        assert!(
            attack_triggers(&game, opponent, alice).is_empty(),
            "opponent's creatures are excluded"
        );
        // Resolve the recipient's two occurrences from the actual declaration
        // queue; the support creature's independent trigger is outside this check.
        let triggers: Vec<_> = attack_queue
            .take_all()
            .into_iter()
            .filter(|trigger| trigger.source == recipient)
            .collect();
        assert_eq!(triggers.len(), 2, "printed and granted melee each trigger");
        for trigger in &triggers {
            assert_eq!(
                trigger.source, recipient,
                "the recipient owns the granted trigger"
            );
        }
        // TriggerIdentity describes the ability structurally, not a distinct
        // printed/granted occurrence. Prove both occurrences actually resolve.
        stack_triggers(&mut game, &triggers);
        resolve_top(&mut game);
        assert_eq!(game.current_power(recipient), Some(4));
        resolve_top(&mut game);
        assert_eq!(game.current_power(recipient), Some(6));
        assert_eq!(game.current_power(source), Some(4));
        assert_eq!(game.current_power(plain), Some(2));
        game.remove_object(source);
        assert!(
            attack_triggers(&game, plain, bob).is_empty(),
            "grant ends when source leaves"
        );
        assert_eq!(
            attack_triggers(&game, recipient, bob).len(),
            1,
            "printed melee remains"
        );
    }
}

#[test]
fn keyword_grant_materialization_myriad_copies_the_recipient_for_other_opponents_only() {
    for definition in definitions(
        "Legion Loyalty",
        "Type: Enchantment\nCreatures you control have myriad.",
    ) {
        for player_count in [2, 3] {
            let mut game = GameState::new(
                (0..player_count).map(|i| format!("Player {i}")).collect(),
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let recipient = creature(&mut game, alice, "");
            let opponent = creature(&mut game, bob, "");
            game.turn.active_player = alice;
            game.turn.phase = ironsmith::game_state::Phase::Combat;
            game.combat = Some(CombatState {
                attackers: vec![AttackerInfo {
                    creature: recipient,
                    target: AttackTarget::Player(bob),
                }],
                ..Default::default()
            });
            assert!(attack_triggers(&game, opponent, alice).is_empty());
            let triggers = attack_triggers(&game, recipient, bob);
            assert_eq!(triggers.len(), 1);
            assert_eq!(triggers[0].source, recipient);
            stack_triggers(&mut game, &triggers);
            resolve_top(&mut game);
            let tokens: Vec<_> = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
                .collect();
            assert_eq!(
                tokens.len(),
                player_count - 2,
                "defending player and controller get no copy"
            );
            for token in tokens {
                assert_eq!(game.object(token).unwrap().name, "Grant recipient");
                assert!(game.is_tapped(token));
                assert_eq!(game.current_controller(token), Some(alice));
                assert!(
                    game.combat
                        .as_ref()
                        .unwrap()
                        .attackers
                        .iter()
                        .any(|attacker| attacker.creature == token
                            && attacker.target == AttackTarget::Player(PlayerId::from_index(2)))
                );
            }
        }
    }
}

#[test]
fn keyword_grant_materialization_equipment_afflict_follows_attachment_and_defending_player() {
    for definition in definitions(
        "Dagger of the Worthy",
        "Type: Artifact — Equipment\nEquipped creature gets +2/+0 and has afflict 1.\nEquip {2}",
    ) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let first = creature(&mut game, alice, "");
        let second = creature(&mut game, alice, "");
        assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(first)));
        let blocked = |game: &GameState, attacker| {
            check_triggers(
                game,
                &TriggerEvent::new_with_provenance(
                    CreatureBecameBlockedEvent::with_target(
                        attacker,
                        2,
                        AttackEventTarget::Player(bob),
                    ),
                    Default::default(),
                ),
            )
        };
        assert!(
            attack_triggers(&game, first, bob).is_empty(),
            "afflict triggers on becoming blocked, not attacking"
        );
        assert!(blocked(&game, second).is_empty());
        let triggers = blocked(&game, first);
        assert_eq!(
            triggers.len(),
            1,
            "two blockers still cause only one afflict trigger"
        );
        assert_eq!(triggers[0].source, first);
        stack_triggers(&mut game, &triggers);
        resolve_top(&mut game);
        assert_eq!(game.player(bob).unwrap().life, 19);
        assert_eq!(game.player(cara).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(second)));
        assert!(blocked(&game, first).is_empty());
        assert_eq!(blocked(&game, second).len(), 1);
    }
}

#[test]
fn keyword_grant_materialization_compiles_baseline_failed_real_cards() {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/keyword_grant_materialization.json.fixture"
    ))
    .unwrap();
    let fixtures = fixtures.as_array().unwrap();
    assert_eq!(fixtures.len(), 10);
    for fixture in fixtures {
        let name = fixture["name"].as_str().unwrap();
        let text = fixture["text"].as_str().unwrap();
        let (artifact, definition) = compile_to_artifact(name, text, false)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        artifact.validate().unwrap();
        let debug = format!("{definition:?}");
        assert!(!debug.contains("KeywordFallbackText"), "{name}: {debug}");
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&artifact).unwrap();
        assert!(registry.get(name).is_some());
    }
}
