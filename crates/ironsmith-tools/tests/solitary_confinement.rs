//! Solitary Confinement: "At the beginning of your upkeep, sacrifice this
//! enchantment unless you discard a card. Skip your draw step. You have
//! shroud. Prevent all damage that would be dealt to you."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::game_state::{Phase, Step, Target};
use ironsmith::ids::CardId;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Solitary Confinement",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

fn card(name: &str, card_type: CardType) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

fn setup() -> (GameState, ObjectId) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    let confinement = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    (game, confinement)
}

#[test]
fn damage_to_you_is_prevented_but_not_to_your_creatures() {
    let (mut game, _) = setup();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bolt_source = game.create_object_from_definition(&card("Shock Source", CardType::Artifact), bob, Zone::Battlefield);
    let bear = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 5))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(bolt_source, bob, &mut dm);
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::Specific(alice))),
        &mut ctx,
    )
    .unwrap();
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::Effect::deal_damage(3, ChooseSpec::SpecificObject(bear)),
        &mut ctx,
    )
    .unwrap();
    assert_eq!(game.player(alice).unwrap().life, 20, "damage to Alice is prevented");
    assert_eq!(game.damage_on(bear), 3, "only damage to Alice is prevented");
}

#[test]
fn you_have_shroud() {
    let (game, _) = setup();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bob_card = CardDefinitionBuilder::new(CardId::new(), "Bob's Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    let mut game = game;
    let source = game.create_object_from_definition(&bob_card, bob, Zone::Hand);
    let legal = ironsmith::game_loop::compute_legal_targets(
        &game,
        &ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)),
        bob,
        Some(source),
    );
    assert!(!legal.contains(&Target::Player(alice)), "Alice can't be targeted");
    assert!(legal.contains(&Target::Player(bob)));
}

struct Discard(bool);

impl DecisionMaker for Discard {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.0
    }
}

/// Runs Alice's upkeep trigger with `hand` cards in hand; returns whether the
/// enchantment survived and the hand size afterwards.
fn upkeep(hand: usize, discard: bool) -> (bool, usize) {
    let (mut game, confinement) = setup();
    let alice = PlayerId::from_index(0);
    for i in 0..hand {
        game.create_object_from_definition(&card(&format!("Card {i}"), CardType::Sorcery), alice, Zone::Hand);
    }
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Upkeep);
    let mut queue = TriggerQueue::new();
    for event in ironsmith::triggers::generate_step_trigger_events_for_active_players(&game) {
        for entry in ironsmith::triggers::check_triggers(&game, &event) {
            queue.add(entry);
        }
    }
    let mut dm = Discard(discard);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1, "upkeep trigger");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let alive = game.object(confinement).is_some_and(|o| o.zone == Zone::Battlefield);
    (alive, game.player(alice).unwrap().hand.len())
}

#[test]
fn upkeep_discard_keeps_it_otherwise_it_is_sacrificed() {
    assert_eq!(upkeep(2, true), (true, 1), "discarding a card keeps it");
    assert_eq!(upkeep(2, false), (false, 2), "declining sacrifices it");
    assert_eq!(upkeep(0, true), (false, 0), "no card to discard");
}

#[test]
fn draw_step_is_skipped() {
    let (mut game, _) = setup();
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&card("Library Card", CardType::Sorcery), alice, Zone::Library);
    assert!(game.player_skips_draw_step(alice), "Skip your draw step");
    ironsmith::turn::execute_draw_step(&mut game);
    assert!(game.player(alice).unwrap().hand.is_empty(), "no draw");
}
