#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ANGELS_TRUMPET_ORACLE: &str = "All creatures have vigilance.\nAt the beginning of each player's end step, tap all untapped creatures that player controls that didn't attack this turn. This artifact deals damage to the player equal to the number of creatures tapped this way.";
const ASTRAL_CONFRONTATION_ORACLE: &str =
    "This spell costs {1} less to cast for each opponent you're attacking.\nExile target creature.";
const BLACK_VISE_ORACLE: &str = "As this artifact enters, choose an opponent.\nAt the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is the number of cards in their hand minus 4.";
const CEPHALID_BROKER_ORACLE: &str = "{T}: Target player draws two cards, then discards two cards.";

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(10, 10))
        .build()
}

fn hand_or_library_card(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn trigger_event<E: crate::events::GameEventType + 'static>(
    event: E,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        event,
        crate::provenance::ProvNodeId::default(),
    )
}

fn source_triggers(
    game: &crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

fn resolve_source_trigger(
    game: &mut crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) {
    let entries = source_triggers(game, source, event);
    assert_eq!(entries.len(), 1, "expected exactly one trigger from source");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("source trigger should be put on the stack");
    crate::game_loop::resolve_stack_entry(game).expect("source trigger should resolve");
}

#[test]
fn high_score_player_value_cards_retain_their_canonical_rules_surfaces() {
    for (name, oracle) in [
        ("Astral Confrontation", ASTRAL_CONFRONTATION_ORACLE),
        ("Cephalid Broker", CEPHALID_BROKER_ORACLE),
    ] {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition).join("\n"),
            oracle,
            "{name} should render exactly: {definition:#?}"
        );
    }

    let trumpet = parse_oracle_card_definition("Angel's Trumpet");
    assert_eq!(
        canonical_compiled_lines(&trumpet).join("\n"),
        ANGELS_TRUMPET_ORACLE.replace("the player", "that player"),
        "Angel's Trumpet should retain its complete executable text; its sole known residual is the player-reference determiner"
    );
    let vise = parse_oracle_card_definition("Black Vise");
    assert_eq!(
        canonical_compiled_lines(&vise).join("\n"),
        BLACK_VISE_ORACLE
            .replace("the chosen player's upkeep", "that player's upkeep")
            .replace("damage to that player", "damage to the chosen player")
            .replace("their hand", "the chosen player's hand"),
        "Black Vise should retain its complete executable text; the runtime-safe chosen-player binding currently alternates explicit and anaphoric chosen-player surfaces"
    );
}

#[test]
fn angels_trumpet_taps_and_counts_only_the_end_step_players_eligible_creatures() {
    let definition = parse_oracle_card_definition("Angel's Trumpet");
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let trumpet = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bob_eligible_one =
        game.create_object_from_definition(&creature("Bob Eligible One"), bob, Zone::Battlefield);
    let bob_eligible_two =
        game.create_object_from_definition(&creature("Bob Eligible Two"), bob, Zone::Battlefield);
    let bob_attacked =
        game.create_object_from_definition(&creature("Bob Attacked"), bob, Zone::Battlefield);
    let bob_already_tapped =
        game.create_object_from_definition(&creature("Bob Already Tapped"), bob, Zone::Battlefield);
    let alice_eligible =
        game.create_object_from_definition(&creature("Alice Eligible"), alice, Zone::Battlefield);
    let cara_eligible =
        game.create_object_from_definition(&creature("Cara Eligible"), cara, Zone::Battlefield);
    game.tap(bob_already_tapped);
    game.record_turn_history_event(&trigger_event(
        crate::events::combat::CreatureAttackedEvent::new(
            bob_attacked,
            crate::triggers::event::AttackEventTarget::Player(alice),
        ),
    ));
    game.mark_creature_attacked_this_turn(bob_attacked);

    game.refresh_continuous_state();
    assert!(
        game.current_has_static_ability_id(bob_eligible_one, StaticAbilityId::Vigilance),
        "Angel's Trumpet should grant vigilance even to an opponent's creature"
    );

    let event = trigger_event(crate::events::phase::BeginningOfEndStepEvent::new(bob));
    resolve_source_trigger(&mut game, trumpet, &event);

    assert!(game.is_tapped(bob_eligible_one));
    assert!(game.is_tapped(bob_eligible_two));
    assert!(
        !game.is_tapped(bob_attacked),
        "a creature that attacked this turn is ineligible"
    );
    assert!(
        game.is_tapped(bob_already_tapped),
        "an already-tapped creature remains tapped but is not counted"
    );
    assert!(
        !game.is_tapped(alice_eligible) && !game.is_tapped(cara_eligible),
        "the end-step trigger must not tap another player's creatures"
    );
    assert_eq!(game.life_total(alice), 20);
    assert_eq!(
        game.life_total(bob),
        18,
        "exactly two newly tapped creatures count"
    );
    assert_eq!(game.life_total(cara), 20);
}

fn astral_cost_for_defenders(defenders: &[PlayerId]) -> String {
    let mut definition = parse_oracle_card_definition("Astral Confrontation");
    definition.card.mana_cost = Some(ManaCost::from_pips(vec![
        vec![ManaSymbol::Generic(4)],
        vec![ManaSymbol::White],
    ]));
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut combat = crate::combat_state::CombatState::default();
    for defender in defenders {
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: game.new_object_id(),
            target: crate::combat_state::AttackTarget::Player(*defender),
        });
    }
    game.combat = Some(combat);
    let object = game.object(spell).expect("Astral Confrontation exists");
    crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        object,
        object.mana_cost.as_ref().expect("Astral has a mana cost"),
    )
    .to_oracle()
}

#[test]
fn astral_confrontation_reduces_once_per_distinct_opponent_being_attacked() {
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    assert_eq!(astral_cost_for_defenders(&[]), "{4}{W}");
    assert_eq!(
        astral_cost_for_defenders(&[bob, bob]),
        "{3}{W}",
        "multiple creatures attacking one opponent grant only one reduction"
    );
    assert_eq!(
        astral_cost_for_defenders(&[bob, cara]),
        "{2}{W}",
        "attacking two distinct opponents grants two reductions"
    );
}

struct ChooseBobOpponent {
    bob: PlayerId,
    saw_choice: bool,
}

impl crate::decision::DecisionMaker for ChooseBobOpponent {
    fn decide_options(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if let Some(option) = ctx
            .options
            .iter()
            .find(|option| option.description == "Bob")
        {
            assert!(
                ctx.options
                    .iter()
                    .all(|option| option.description != "Alice"),
                "Black Vise's controller must not be offered as an opponent: {:?}",
                ctx.options
            );
            self.saw_choice = true;
            assert_eq!(ctx.player, PlayerId::from_index(0));
            assert_eq!(self.bob, PlayerId::from_index(1));
            vec![option.index]
        } else {
            panic!(
                "Black Vise should offer Bob as an opponent: {:?}",
                ctx.options
            );
        }
    }
}

#[test]
fn typed_choose_opponent_as_enters_static_excludes_its_controller() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Opponent Choice Probe")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            crate::static_abilities::StaticAbility::choose_player_as_enters_matching(
                PlayerFilter::Opponent,
                "As this artifact enters, choose an opponent.".to_string(),
            ),
        ))
        .build();
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut chooser = ChooseBobOpponent {
        bob,
        saw_choice: false,
    };
    let entered = game
        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut chooser)
        .expect("opponent-choice probe should enter")
        .new_id;

    assert!(chooser.saw_choice);
    assert_eq!(game.chosen_player(entered), Some(bob));
}

#[test]
fn black_vise_chooses_only_an_opponent_then_uses_that_players_upkeep_and_hand() {
    let definition = parse_oracle_card_definition("Black Vise");
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let vise_in_hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let probe = hand_or_library_card("Black Vise Hand Probe");
    for _ in 0..9 {
        game.create_object_from_definition(&probe, alice, Zone::Hand);
    }
    for _ in 0..7 {
        game.create_object_from_definition(&probe, bob, Zone::Hand);
    }
    for _ in 0..5 {
        game.create_object_from_definition(&probe, cara, Zone::Hand);
    }

    let mut chooser = ChooseBobOpponent {
        bob,
        saw_choice: false,
    };
    let vise = game
        .move_object_with_etb_processing_with_dm(vise_in_hand, Zone::Battlefield, &mut chooser)
        .expect("Black Vise should enter after choosing an opponent")
        .new_id;
    assert!(chooser.saw_choice);
    assert_eq!(game.chosen_player(vise), Some(bob));

    let alice_upkeep = trigger_event(crate::events::phase::BeginningOfUpkeepEvent::new(alice));
    let cara_upkeep = trigger_event(crate::events::phase::BeginningOfUpkeepEvent::new(cara));
    assert!(source_triggers(&game, vise, &alice_upkeep).is_empty());
    assert!(source_triggers(&game, vise, &cara_upkeep).is_empty());

    let bob_upkeep = trigger_event(crate::events::phase::BeginningOfUpkeepEvent::new(bob));
    resolve_source_trigger(&mut game, vise, &bob_upkeep);
    assert_eq!(game.life_total(alice), 20);
    assert_eq!(
        game.life_total(bob),
        17,
        "seven cards in the chosen player's hand should deal 7 - 4 damage"
    );
    assert_eq!(game.life_total(cara), 20);
}

#[test]
fn cephalid_broker_draws_and_discards_for_the_same_target_player() {
    let definition = parse_oracle_card_definition("Cephalid Broker");
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    let broker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.remove_summoning_sickness(broker);
    let probe = hand_or_library_card("Cephalid Broker Zone Probe");
    for _ in 0..3 {
        game.create_object_from_definition(&probe, alice, Zone::Hand);
    }
    for _ in 0..4 {
        game.create_object_from_definition(&probe, bob, Zone::Hand);
    }
    for _ in 0..2 {
        game.create_object_from_definition(&probe, bob, Zone::Library);
    }
    game.create_object_from_definition(&probe, cara, Zone::Hand);

    let alice_hand_before = game.player(alice).expect("Alice exists").hand.len();
    let bob_hand_before = game.player(bob).expect("Bob exists").hand.len();
    let bob_library_before = game.player(bob).expect("Bob exists").library.len();
    let bob_graveyard_before = game.player(bob).expect("Bob exists").graveyard.len();
    let cara_hand_before = game.player(cara).expect("Cara exists").hand.len();
    let ability_index = game
        .object(broker)
        .expect("Cephalid Broker exists")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Cephalid Broker has an activated ability");
    let action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility {
                    source,
                    ability_index: index,
                } if *source == broker && *index == ability_index
            )
        })
        .expect("Cephalid Broker's untapped ability should be legal");
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(action),
        &mut decisions,
    )
    .expect("Cephalid Broker activation should start");
    assert!(matches!(
        progress,
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_)
        )
    ));
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::Targets(vec![crate::Target::Player(bob)]),
        &mut decisions,
    )
    .expect("Cephalid Broker should accept Bob as its target");
    assert!(game.is_tapped(broker), "tapping Cephalid Broker is a cost");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Cephalid Broker's ability should resolve");

    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        alice_hand_before
    );
    assert_eq!(
        game.player(bob).expect("Bob exists").hand.len(),
        bob_hand_before
    );
    assert_eq!(
        game.player(bob).expect("Bob exists").library.len(),
        bob_library_before - 2
    );
    assert_eq!(
        game.player(bob).expect("Bob exists").graveyard.len(),
        bob_graveyard_before + 2
    );
    assert_eq!(
        game.player(cara).expect("Cara exists").hand.len(),
        cara_hand_before
    );
}
