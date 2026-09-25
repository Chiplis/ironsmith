//! Emiel the Blessed: "{3}: Exile another target creature you control, then
//! return it to the battlefield under its owner's control. Whenever another
//! creature you control enters, you may pay {G/W}. If you do, put a +1/+1
//! counter on it. If it's a Unicorn, put two +1/+1 counters on it instead."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, CounterType, GameState, ObjectId, PlayerId, Subtype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Emiel the Blessed",
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

/// Accepts or declines the optional {G/W} payment.
struct Pay(bool);

impl DecisionMaker for Pay {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.0
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        ctx.options
            .iter()
            .filter(|o| o.legal)
            .take(ctx.min.max(1))
            .map(|o| o.index)
            .collect()
    }
}

fn creature(name: &str, subtype: Subtype) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

/// Emiel is on the battlefield; `entering` enters under Alice's control and
/// the trigger resolves. Returns the +1/+1 counters it ends up with.
fn enter(entering: Subtype, pay: bool) -> (u32, u32) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 1);
    let hand =
        game.create_object_from_definition(&creature("Newcomer", entering), alice, Zone::Hand);
    let mut dm = Pay(pay);
    let entered: ObjectId = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .expect("enters")
        .new_id;
    let mut queue = TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        for entry in ironsmith::triggers::check_triggers(&game, &event) {
            queue.add(entry);
        }
    }
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1, "Emiel's trigger");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    (
        game.counter_count(entered, CounterType::PlusOnePlusOne),
        game.player(alice).unwrap().mana_pool.total(),
    )
}

#[test]
fn paying_puts_one_counter_on_a_non_unicorn() {
    assert_eq!(enter(Subtype::Bear, true), (1, 0));
}

#[test]
fn paying_puts_two_counters_on_a_unicorn_instead() {
    assert_eq!(enter(Subtype::Unicorn, true), (2, 0));
}

#[test]
fn declining_the_payment_puts_no_counter() {
    assert_eq!(enter(Subtype::Unicorn, false), (0, 1));
}

#[test]
fn three_mana_blinks_another_creature_you_control() {
    use ironsmith::decision::{GameProgress, LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let emiel = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let bear = game.create_object_from_definition(
        &creature("Bear", Subtype::Bear),
        alice,
        Zone::Battlefield,
    );
    game.object_mut(bear)
        .unwrap()
        .add_counters(CounterType::PlusOnePlusOne, 1);
    let stable = game.object(bear).unwrap().stable_id;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == emiel))
        .expect("activatable");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Pay(false);
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let returned = game.find_object_by_stable_id(stable).expect("returned");
    assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
    assert_ne!(returned, bear, "a new object");
    assert_eq!(
        game.counter_count(returned, CounterType::PlusOnePlusOne),
        0,
        "counters are lost"
    );
}
